use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::warn;

use agent_proto::events::NetworkConnection;

const DNS_CACHE_TTL: Duration = Duration::from_secs(300);
const DNS_CACHE_MAX_ENTRIES: usize = 10_000;
const DNS_RESOLVER_TIMEOUT: Duration = Duration::from_secs(3);

pub struct DnsCacheEntry {
    hostname: String,
    resolved_at: Instant,
}

pub struct NetworkCollector {
    dns_cache: Arc<RwLock<HashMap<IpAddr, DnsCacheEntry>>>,
    hosts_cache: HashMap<IpAddr, String>,
}

impl NetworkCollector {
    pub fn new() -> Self {
        let hosts_cache = Self::load_etc_hosts();
        Self {
            dns_cache: Arc::new(RwLock::new(HashMap::new())),
            hosts_cache,
        }
    }

    fn load_etc_hosts() -> HashMap<IpAddr, String> {
        #[cfg(target_os = "linux")]
        {
            let mut map = HashMap::new();
            if let Ok(contents) = std::fs::read_to_string("/etc/hosts") {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(ip) = parts[0].parse::<IpAddr>() {
                            let hostname = parts[1].to_string();
                            map.insert(ip, hostname);
                        }
                    }
                }
            }
            map
        }
        #[cfg(not(target_os = "linux"))]
        {
            HashMap::new()
        }
    }

    pub async fn resolve_ips(&self, connections: &mut [NetworkConnection]) {
        let ips_to_resolve: Vec<IpAddr> = {
            let cache = self.dns_cache.read().await;
            connections
                .iter()
                .filter_map(|c| {
                    let ip: Option<IpAddr> = c.remote_ip.parse().ok();
                    ip.filter(|ip| !ip.is_loopback() && !cache.contains_key(ip) && !self.hosts_cache.contains_key(ip))
                })
                .collect()
        };

        if !ips_to_resolve.is_empty() {
            let new_entries = Self::batch_reverse_resolve(ips_to_resolve).await;
            let mut cache = self.dns_cache.write().await;
            for (ip, hostname) in new_entries {
                cache.insert(
                    ip,
                    DnsCacheEntry {
                        hostname,
                        resolved_at: Instant::now(),
                    },
                );
            }
        }

        self.evict_expired().await;

        let cache = self.dns_cache.read().await;
        for conn in connections.iter_mut() {
            if let Ok(ip) = conn.remote_ip.parse::<IpAddr>() {
                if ip.is_loopback() {
                    continue;
                }
                if let Some(hostname) = self.hosts_cache.get(&ip) {
                    conn.remote_hostname = Some(hostname.clone());
                    conn.reconstructed_url = Some(reconstruct_url(hostname, conn.remote_port));
                } else if let Some(entry) = cache.get(&ip) {
                    conn.remote_hostname = Some(entry.hostname.clone());
                    conn.reconstructed_url = Some(reconstruct_url(&entry.hostname, conn.remote_port));
                }
            }
        }
    }

    async fn batch_reverse_resolve(ips: Vec<IpAddr>) -> Vec<(IpAddr, String)> {
        let mut results = Vec::new();

        #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
        {
            use hickory_resolver::TokioResolver;
            use hickory_resolver::config::ResolverOpts;

            let mut opts = ResolverOpts::default();
            opts.timeout = DNS_RESOLVER_TIMEOUT;
            opts.attempts = 1;

            let resolver = match TokioResolver::builder_tokio() {
                Ok(builder) => builder.with_options(opts).build(),
                Err(e) => {
                    warn!("Failed to create DNS resolver: {}", e);
                    return results;
                }
            };

            let mut resolve_futures = Vec::new();
            for ip in ips {
                let resolver = resolver.clone();
                resolve_futures.push(async move {
                    match resolver.reverse_lookup(ip).await {
                        Ok(lookup) => {
                            let hostname = lookup.iter().next().map(|n| n.to_string().trim_end_matches('.').to_string());
                            hostname.map(|h| (ip, h))
                        }
                        Err(_) => None,
                    }
                });
            }

            let resolved = futures_util::future::join_all(resolve_futures).await;
            for r in resolved {
                if let Some(pair) = r {
                    results.push(pair);
                }
            }
        }

        results
    }

    async fn evict_expired(&self) {
        let mut cache = self.dns_cache.write().await;
        let now = Instant::now();
        cache.retain(|_, entry| now.duration_since(entry.resolved_at) < DNS_CACHE_TTL);
        if cache.len() > DNS_CACHE_MAX_ENTRIES {
            let to_remove = cache.len() - DNS_CACHE_MAX_ENTRIES;
            let oldest_keys: Vec<IpAddr> = cache
                .iter()
                .filter(|(_, entry)| now.duration_since(entry.resolved_at) > Duration::from_secs(60))
                .map(|(ip, _)| *ip)
                .take(to_remove)
                .collect();
            for key in oldest_keys {
                cache.remove(&key);
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub fn load_windows_dns_cache(&self) -> HashMap<String, Vec<IpAddr>> {
        HashMap::new()
    }

    #[cfg(target_os = "windows")]
    pub fn load_windows_dns_cache(&self) -> HashMap<String, Vec<IpAddr>> {
        let mut cache: HashMap<String, Vec<IpAddr>> = HashMap::new();
        let output = match std::process::Command::new("ipconfig")
            .args(["/displaydns"])
            .output()
        {
            Ok(o) => o,
            Err(_) => return cache,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_name: Option<String> = None;

        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Record Name") {
                if let Some(dot_pos) = line.find(':') {
                    let name = line[dot_pos + 1..].trim().to_string();
                    if !name.is_empty() {
                        current_name = Some(name);
                    }
                }
            } else if line.starts_with("A (Host) Record") || line.starts_with("AAAA Record") {
                if let (Some(name), Some(dot_pos)) = (&current_name, line.find(':')) {
                    let ip_str = line[dot_pos + 1..].trim();
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        cache.entry(name.clone()).or_default().push(ip);
                    }
                }
            }
        }
        cache
    }
}

pub fn reconstruct_url(hostname: &str, port: u16) -> String {
    let scheme = match port {
        443 => "https",
        80 => "http",
        _ => "https",
    };
    if (scheme == "https" && port == 443) || (scheme == "http" && port == 80) {
        format!("{}://{}", scheme, hostname)
    } else {
        format!("{}://{}:{}", scheme, hostname, port)
    }
}

pub fn should_skip_interface(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("lo")
        || lower.starts_with("veth")
        || lower.starts_with("br-")
        || lower.starts_with("docker")
        || lower.starts_with("virbr")
        || lower.starts_with("vnet")
        || lower.starts_with("macvtap")
        || lower.starts_with("tun")
        || lower.starts_with("tap")
        || lower.contains("vpn")
        || lower.contains("tunnel")
        || lower.contains("virtual")
        || lower.contains("hyper-v")
}

pub fn should_skip_ip(ip: &str) -> bool {
    ip.is_empty()
        || ip.starts_with("127.")
        || ip.starts_with("0.")
        || ip == "::1"
        || ip.starts_with("fe80:")
        || ip.starts_with("169.254.")
}