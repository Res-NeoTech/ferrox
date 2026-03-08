use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use mime_guess::mime;

use crate::handlers::static_files::serve_file;
use crate::http::error::render_error;
use crate::http::request::Request;
use crate::http::response::Response;

pub fn serve(addr: &str) {
    let listener = TcpListener::bind(addr).unwrap();

    println!("Ferrox running on http://{}", addr);

    for stream in listener.incoming() {
        let stream: TcpStream = stream.unwrap();
        thread::spawn(|| {
            handle(stream);
        });
    }
}

fn handle(mut stream: TcpStream) {
    let mut buffer: [u8; 1024] = [0; 1024];
    let size = stream.read(&mut buffer).unwrap();

    let request: Request = Request::parse(&buffer[..size]);
    let response: Response = match serve_file(&request.path) {
        Ok(r) => r,
        Err(_) => Response { status: "500 Internal Server Error", content_type: mime::TEXT_HTML, body: render_error("500", "Internal Server Error") }
    };

    println!("{} {} {}", &request.method, &request.path, &request.version);

    let bytes = response.to_bytes();
    stream.write_all(&bytes).unwrap();
}
