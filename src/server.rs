use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::{thread};
use time::UtcDateTime;

use crate::handlers::static_files::serve_file;
use crate::http::request::Request;
use crate::http::response::{Body, Response};

pub fn serve(addr: &str) {
    let listener: TcpListener = TcpListener::bind(addr).unwrap();

    println!("Ferrox running on http://{}", addr);

    for stream in listener.incoming() {
        let stream: TcpStream = stream.unwrap();
        thread::spawn(|| {
            if let Err(e) = handle(stream) {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer: [u8; 1024] = [0; 1024];
    let size: usize = stream.read(&mut buffer).unwrap();

    let connecting_ip: IpAddr = stream.peer_addr()?.ip();
    let requested_ip: IpAddr = stream.local_addr()?.ip();
    let date: UtcDateTime = UtcDateTime::now();

    let request: Request = Request::parse(&buffer[..size]);

    let mut response: Response = match serve_file(&request.path) {
        Ok(r) => r,
        Err(_) => Response::error("500", "Internal Server Error")
    };

    println!(
        "{} - [{}] \"{} {} {}\" {} {} - \"{}\" \"{}\"",
        &connecting_ip.to_string(),
        &date.to_string(),
        &request.method,
        &request.path,
        &request.version,
        &response.status,
        &response.content_length,
        &request.headers.get("User-Agent").unwrap(),
        &requested_ip.to_string()
    );

    response.write_headers(&mut stream)?;

    match &mut response.body {
        Body::Bytes(bytes) => {
            stream.write_all(bytes)?;
        }
        Body::File(file) => {
            std::io::copy(file, &mut stream)?;
        }
    }

    Ok(())
}
