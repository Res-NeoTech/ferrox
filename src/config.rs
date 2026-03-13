use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct FerroxConfig {
    pub host: String,
    pub port: u16,
    pub root_dir: String,
    pub log_dir: String,
    pub max_workers: usize,
}

impl Default for FerroxConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            root_dir: "www".into(),
            log_dir: "logs".into(),
            max_workers: 4,
        }
    }
}

impl FerroxConfig {
    pub fn load() -> Self {
        fs::read_to_string("ferrox.toml")
            .and_then(|content| Ok(toml::from_str(&content).unwrap_or_default()))
            .unwrap_or_default()
    }
}
