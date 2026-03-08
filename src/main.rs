mod server;
mod http;
mod handlers;

fn main() {
    server::serve("127.0.0.1:8080");
}