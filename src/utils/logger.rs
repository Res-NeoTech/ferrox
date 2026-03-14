use std::net::IpAddr;
use std::{fs::OpenOptions, net::TcpStream};
use std::io::Write;

use time::UtcDateTime;

use crate::http::{request::{Request}, response::Response};

const LOGGING_DIR: &str = "logs";

fn append_log(append_file: &str, log: String) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}/{}", LOGGING_DIR, append_file))?;

    writeln!(file, "{log}")?;

    Ok(())
}

pub fn access(request: &Request, response: &Response, stream: &TcpStream) -> std::io::Result<()> {
    let connecting_ip: IpAddr = stream.peer_addr()?.ip();
    let requested_ip: IpAddr = stream.local_addr()?.ip();
    let date: UtcDateTime = UtcDateTime::now();

    let log: String = format!(
        "{} - [{}] \"{} {} {}\" {} {} - \"{}\" \"{}\"",
        connecting_ip.to_string(),
        date.to_string(),
        request.method,
        request.path,
        request.version,
        response.status,
        response.content_length,
        request.headers.get("User-Agent").unwrap_or(&"-".to_string()),
        requested_ip.to_string()
    );

    match append_log("access.log", log) {
        Ok(()) => { },
        Err(_) => eprintln!("Something went wrong while persisting the log. Make sure directory {LOGGING_DIR} exists.")
    };

    Ok(())
}

pub fn error_log(concern: &str, error: String) {
    let date: UtcDateTime = UtcDateTime::now();

    let log: String = format!("{} [{}]: {}", date.to_string(), concern, error);

    match append_log("error.log", log) {
        Ok(()) => { },
        Err(_) => eprintln!("Something went wrong while persisting the log. Make sure directory {LOGGING_DIR} exists.")
    };
}