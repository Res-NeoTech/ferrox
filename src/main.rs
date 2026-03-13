mod server;
mod http;
mod handlers;
mod utils;
mod config;

fn main() {
    let conf = config::FerroxConfig::load();
    server::serve(&conf);
}