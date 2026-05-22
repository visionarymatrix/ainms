use anyhow::Result;

#[allow(dead_code)]
pub fn load_config_from_file(path: &str) -> Result<crate::ConfigFile> {
    let contents = std::fs::read_to_string(path)?;
    let cfg: crate::ConfigFile = toml::from_str(&contents)?;
    Ok(cfg)
}