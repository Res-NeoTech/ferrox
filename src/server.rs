use std::io::{Error, ErrorKind, Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use threadpool::ThreadPool;

use crate::handlers::static_files::serve_file;
use crate::http::request::Request;
use crate::http::response::{Body, Response};
use crate::utils::logger;

const MAX_HEADER_SIZE: u64 = 8192; // 8KB
const MAX_WORKERS: usize = 4;
const READ_TIMEOUT_SEC: u64 = 5;
const WRITE_TIMEOUT_SEC: u64 = 5;

pub fn serve(addr: &str) {
    let listener = TcpListener::bind(addr).unwrap();
    let pool = ThreadPool::new(MAX_WORKERS);

    println!("Ferrox running on http://{addr} with {MAX_WORKERS} workers");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SEC)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT_SEC)));

                pool.execute(move || {
                    if let Err(e) = handle(stream) {
                        logger::error_log("core", format!("Connection error: {}", e));
                    }
                });
            }
            Err(e) => {
                logger::error_log("core", format!("Failed to accept connection: {}", e));
            }
        }
    }
}

fn handle(mut stream: TcpStream) -> Result<()> {
    let mut full_data: Vec<u8> = Vec::new();
    let mut temp_buffer: [u8; 1024] = [0u8; 1024];

    loop {
        let bytes_read = stream.read(&mut temp_buffer)?;

        if bytes_read == 0 {
            return Ok(());
        }

        full_data.extend_from_slice(&temp_buffer[..bytes_read]);

        if full_data.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }

        if full_data.len() > MAX_HEADER_SIZE as usize {
            return Err(Error::new(
                ErrorKind::ArgumentListTooLong,
                "Max header size reached.",
            ));
        }
    }

    let request = match Request::parse(&full_data) {
        Ok(r) => r,
        Err(e) => {
            logger::error_log("parser", format!("Failed to parse http request: {}", e));

            let error_res = Response::error("400", "Bad Request");
            let _ = error_res.write_headers(&mut stream);
            if let Body::Bytes(b) = error_res.body {
                let _ = stream.write_all(&b);
            }

            return Ok(());
        }
    };

    let mut response: Response = match serve_file(&request.path) {
        Ok(r) => r,
        Err(e) => {
            logger::error_log("file", format!("Failed to server static file: {}", e));
            Response::error("500", "Internal Server Error")
        }
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

    logger::access(&request, &response, &stream)?;

    Ok(())
}
