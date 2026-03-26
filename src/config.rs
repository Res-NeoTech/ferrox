use serde::{Deserialize};
use std::{collections::HashMap, fs};

/// Stores the network address settings used when binding the HTTP server.
#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub http_port: u16,
    pub https_port: u16,
    pub addr: String,
    pub router: RouterPreset
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RouterPreset {
    Static,
    Spa
}

/// Defines the filesystem paths used for static files and log output.
#[derive(Deserialize, Debug, Clone)]
pub struct PathsConfig {
    pub serve_dir: String,
    pub log_dir: String
}

// Defines basic tls config.
#[derive(Deserialize, Debug, Clone)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
}

/// Represents the full Ferrox configuration loaded from `ferrox-compose.yml`.
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub paths: PathsConfig,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub tls: TlsConfig
}

impl Config {
    /// Loads and deserializes the YAML configuration file from disk.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the YAML configuration file.
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str: String = fs::read_to_string(path)?;
        
        let config: Config = serde_yaml::from_str(&config_str)?; 
        
        Ok(config)
    }
}
