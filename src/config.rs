use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub server: Vec<ServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub dir: String,
    pub cmd: String,
    /// Optional: environment variables for this server
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Config {
                server: Vec::new(),
            });
        }
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config at {}: {}", path.display(), e))?;
        let config: Config =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))?;
        Ok(config)
    }

    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("server-start");
        fs::create_dir_all(&path).ok();
        path.push("config.toml");
        path
    }
}
