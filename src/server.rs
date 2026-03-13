use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use threadpool::ThreadPool;

use crate::config::FerroxConfig;
use crate::handlers::static_files::serve_file;
use crate::http::request::Request;
use crate::http::response::{Body, Response};
use crate::utils::logger;

const MAX_HEADER_SIZE: u64 = 8192; // 8KB
const READ_TIMEOUT_SEC: u64 = 5;
const WRITE_TIMEOUT_SEC: u64 = 5;

pub fn serve(config: &FerroxConfig) {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).unwrap();
    let pool = ThreadPool::new(config.max_workers);

    println!("Ferrox running on http://{} with {} workers", addr, config.max_workers);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SEC)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT_SEC)));

                let conf = config.clone();
                pool.execute(move || {
                    if let Err(e) = handle(stream, conf) {
                        eprintln!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("Failed to accept connection: {}", e),
        }
    }
}

fn handle(mut stream: TcpStream, config: FerroxConfig) -> std::io::Result<()> {
    let mut buffer: [u8; 8192] = [0; MAX_HEADER_SIZE as usize];

    let bytes_read = stream.read(&mut buffer)?;

    if bytes_read == 0 {
        return Ok(());
    }

    let request: Request = Request::parse(&buffer[..bytes_read]);

    let mut response: Response = match serve_file(&request.path, &config.root_dir) {
        Ok(r) => r,
        Err(_) => Response::error("500", "Internal Server Error")
    };

    response.write_headers(&mut stream)?;

    match &mut response.body {
        Body::Bytes(bytes) => {
            stream.write_all(bytes)?;
        }
        Body::File(file) => {
            std::io::copy(file, &mut stream)?;
        }
    }

    match logger::access(&request, &response, &stream, &config.log_dir) {
        Ok(()) => { },
        Err(_) => eprintln!("Failed to save log. Make sure the correct directory exists and created.")
    }

    Ok(())
}