use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::FutureExt;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use rust_socketio::asynchronous::{Client, ClientBuilder};
use rust_socketio::{Payload, TransportType};

#[derive(Debug, Clone)]
pub enum SocketCommand {
    ScreenshotRequest { command_id: String, payload: Value },
    PolicyUpdate { command_id: String, payload: Value },
    NLQuery { query_id: String, query: String, payload: Value },
}

#[derive(Clone)]
pub struct SocketClient {
    client: Client,
    connected: Arc<std::sync::atomic::AtomicBool>,
}

/// Connect to Socket.IO and return (SocketClient, command receiver).
///
/// URL: `{server}/socketio/?token={install_token}&type=agent&device_id={device_id}`
pub async fn connect_socket(
    server: &str,
    device_id: &str,
    install_token: &str,
) -> Result<(SocketClient, mpsc::Receiver<SocketCommand>)> {
    let (command_tx, command_rx) = mpsc::channel::<SocketCommand>(256);

    let socket_url = format!(
        "{}/socketio/?token={}&type=agent&device_id={}",
        server, install_token, device_id
    );

    let connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let connected_connect = connected.clone();
    let connected_disconnect = connected.clone();
    let tx_screenshot = command_tx.clone();
    let tx_policy = command_tx.clone();
    let tx_nl_query = command_tx.clone();

    info!(url = %socket_url, "Connecting to Socket.IO server");

    let client = ClientBuilder::new(socket_url)
        .transport_type(TransportType::Websocket)
        .auth(serde_json::json!({
            "token": install_token,
            "type": "agent",
            "device_id": device_id
        }))
        .reconnect(true)
        .reconnect_on_disconnect(true)
        .reconnect_delay(1000, 5000)
        .on("connect", move |_: Payload, _| {
            let connected = connected_connect.clone();
            async move {
                info!("Socket.IO connected");
                connected.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            .boxed()
        })
        .on("disconnect", move |payload: Payload, _| {
            let connected = connected_disconnect.clone();
            async move {
                warn!("Socket.IO disconnected: {:?}", payload);
                connected.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            .boxed()
        })
        .on("screenshot_request", move |payload: Payload, _| {
            let tx = tx_screenshot.clone();
            async move {
                info!("Socket.IO screenshot_request callback fired");
                parse_screenshot_request(payload, &tx);
            }
            .boxed()
        })
        .on("policy_update", move |payload: Payload, _| {
            let tx = tx_policy.clone();
            async move {
                info!("Socket.IO policy_update callback fired");
                parse_generic_command("policy_update", payload, &tx, |id, data| SocketCommand::PolicyUpdate {
                    command_id: id,
                    payload: data,
                });
            }
            .boxed()
        })
        .on("nl_query", move |payload: Payload, _| {
            let tx = tx_nl_query.clone();
            async move {
                info!("Socket.IO nl_query callback fired");
                parse_nl_query(payload, &tx);
            }
            .boxed()
        })
        .on("error", |err: Payload, _| {
            async move {
                error!("Socket.IO error: {:?}", err);
            }
            .boxed()
        })
        .connect()
        .await
        .context("Failed to connect to Socket.IO server")?;

    info!("Socket.IO client initialized and connected");

    Ok((
        SocketClient {
            client,
            connected,
        },
        command_rx,
    ))
}

fn parse_generic_command<F>(event_name: &str, payload: Payload, tx: &mpsc::Sender<SocketCommand>, make_cmd: F)
where
    F: FnOnce(String, Value) -> SocketCommand,
{
    #[allow(deprecated)]
    let data: Option<Value> = match payload {
        Payload::Text(mut values) => values.pop(),
        Payload::String(s) => serde_json::from_str::<Value>(&s).ok(),
        Payload::Binary(_) => {
            warn!("Received binary data for {}, ignoring", event_name);
            None
        }
    };
    if let Some(data) = data {
        let command_id = data
            .get("command_id")
            .or_else(|| data.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        info!(command_id = %command_id, "Received {} via Socket.IO", event_name);
        if let Err(e) = tx.try_send(make_cmd(command_id, data)) {
            error!("Failed to send {} to channel: {}", event_name, e);
        }
    }
}

fn parse_nl_query(payload: Payload, tx: &mpsc::Sender<SocketCommand>) {
    #[allow(deprecated)]
    let data: Option<Value> = match payload {
        Payload::Text(mut values) => values.pop(),
        Payload::String(s) => serde_json::from_str::<Value>(&s).ok(),
        Payload::Binary(_) => {
            warn!("Received binary data for nl_query, ignoring");
            None
        }
    };
    if let Some(data) = data {
        let query_id = data
            .get("query_id")
            .or_else(|| data.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let query = data
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        info!(query_id = %query_id, "Received nl_query via Socket.IO");
        if let Err(e) = tx.try_send(SocketCommand::NLQuery { query_id, query, payload: data }) {
            error!("Failed to send nl_query to channel: {}", e);
        }
    }
}

fn parse_screenshot_request(payload: Payload, tx: &mpsc::Sender<SocketCommand>) {
    parse_generic_command("screenshot_request", payload, tx, |_command_id, payload| {
        let req_id = payload
            .get("request_id")
            .or_else(|| payload.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        SocketCommand::ScreenshotRequest {
            command_id: req_id,
            payload,
        }
    });
}

impl SocketClient {
    pub async fn emit(&self, event: &str, data: serde_json::Value) -> Result<()> {
        self.client
            .emit(event, data)
            .await
            .context("Failed to emit Socket.IO event")
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting Socket.IO client");
        self.client
            .disconnect()
            .await
            .context("Failed to disconnect Socket.IO client")
    }
}
