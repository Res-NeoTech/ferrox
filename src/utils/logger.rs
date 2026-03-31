use std::net::IpAddr;
use tokio::fs::OpenOptions; 
use tokio::io::AsyncWriteExt;

use crate::config::Config; 
use crate::http::{request::Request, response::Response};
use time::UtcDateTime; 

/// Appends a log line to a specified log file.
///
/// # Arguments
///
/// * `config` - Reference to the server configuration.
/// * `append_file` - The name of the log file to append to.
/// * `log` - The log entry as a string.
async fn append_log(config: &Config, append_file: &str, log: String) -> std::io::Result<()> {
    let file_path = format!("{}/{}", config.paths.log_dir, append_file);

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(&file_path)
        .await?;

    let log_line = format!("{}\n", log);
    file.write_all(log_line.as_bytes()).await?;

    Ok(())
}

/// Logs an HTTP access event in a standard log format.
///
/// # Arguments
///
/// * `config` - Reference to the server configuration.
/// * `request` - The incoming HTTP request.
/// * `response` - The outgoing HTTP response.
/// * `stream` - The TCP stream of the connection.
pub async fn access(config: &Config, request: &Request<'_>, response: &Response, connecting_ip: IpAddr, requested_ip: IpAddr) {
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
        request.header("user-agent").unwrap_or(&"-".to_string()),
        requested_ip.to_string()
    );

    match append_log(config, "access.log", log).await {
        Ok(()) => { },
        Err(_) => eprintln!("Failed to persist log. Make sure directory {} exists.", config.paths.log_dir)
    };
}

/// Logs an error message related to a specific concern.
///
/// # Arguments
///
/// * `config` - Reference to the server configuration.
/// * `concern` - The component or area where the error occurred.
/// * `error` - The error message or description.
pub async fn error_log(config: &Config, concern: &str, error: String) {
    let date: UtcDateTime = UtcDateTime::now();
    let log: String = format!("{} [{}]: {}", date.to_string(), concern, error);

    match append_log(config, "error.log", log).await {
        Ok(()) => { },
        Err(_) => eprintln!("Failed to persist log. Make sure directory {} exists.", config.paths.log_dir)
    };
}