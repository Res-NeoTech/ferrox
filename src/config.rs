use std::{collections::HashMap, fs};

use serde::{Deserialize, Deserializer};

/// Stores the network address settings used when binding the HTTP server.
#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_https_port")]
    pub https_port: u16,
    pub addr: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub router: RouterPreset,
    #[serde(default)]
    pub index: bool
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouterPreset {
    Static,
    Spa,
}

impl Default for RouterPreset {
    fn default() -> Self {
        RouterPreset::Static
    }
}

impl<'de> Deserialize<'de> for RouterPreset {
    /// Case-insensitive RouterPreset deserializer.
    /// 
    /// # Arguments
    /// 
    /// * `deserializer` - Serde deserializer.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "static" => Ok(RouterPreset::Static),
            "spa" => Ok(RouterPreset::Spa),
            _ => {
                eprintln!("[WARNING] Unknown router preset, falling back to static file serving.");
                Ok(RouterPreset::Static)
            },
        }
    }
}

/// Defines the filesystem paths used for static files and log output.
#[derive(Deserialize, Debug, Clone)]
pub struct PathsConfig {
    pub serve_dir: String,
    pub log_dir: String,
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
    #[serde(default = "default_tls_config")]
    pub tls: TlsConfig,
}

// Defaults

/// Default value for timeout attribute.
fn default_timeout() -> u64 {
    10 
}

fn default_http_port() -> u16 {
    80
}

fn default_https_port() -> u16 {
    443
}

/// Default value for tls_config attribute.
fn default_tls_config() -> TlsConfig {
    TlsConfig { 
        enabled: false, 
        cert_path: String::new(), 
        key_path: String::new()
    }
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
