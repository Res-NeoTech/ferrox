use std::net::IpAddr;
use std::{fs::OpenOptions, net::TcpStream};
use std::io::Write;

use time::UtcDateTime;

use crate::http::{request::{Request}, response::Response};

fn append_log(append_file: &str, log: String, log_dir: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}/{}", log_dir, append_file))?;

    writeln!(file, "{log}")?;

    Ok(())
}

pub fn access(request: &Request, response: &Response, stream: &TcpStream, log_dir: &str) -> std::io::Result<()> {
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

    append_log("access.log", log, log_dir)?;

    Ok(())
}