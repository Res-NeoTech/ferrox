mod config;
mod handlers;
mod http;
mod server;
mod utils;

use std::sync::Arc;

use config::Config;

/// Loads the application configuration file and starts the Ferrox HTTP server.
#[tokio::main]
async fn main() {
    let config: Config = config::Config::load("ferrox.yml").expect("Failed to load ferrox.yml");
    let shared_config: Arc<Config> = Arc::new(config);
    if shared_config.tls.enabled {
        println!("[INFO] Starting Ferrox with TLS enabled.");
        tokio::join!(
            server::serve_https(Arc::clone(&shared_config)),
            server::serve_http_redirect(Arc::clone(&shared_config))
        );
    } else {
        println!("[INFO] Starting Ferrox in plain HTTP mode (TLS disabled).");
        server::serve_http(Arc::clone(&shared_config)).await;
    }
}
