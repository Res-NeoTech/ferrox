use std::io::{Error, ErrorKind};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use urlencoding::decode;

use crate::config::Config;
use crate::handlers::static_files::serve_file;
use crate::http::request::Request;
use crate::http::response::{Body, Response};
use crate::utils::logger;

use std::fs::File;
use std::io::BufReader;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::{ServerConfig, pki_types::CertificateDer, pki_types::PrivateKeyDer};

const MAX_HEADER_SIZE: u64 = 8192; // 8KB
const CONNECTION_TIMEOUT_SEC: u64 = 10;
const UNSPECIFIED_IP: IpAddr = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);

/// Starts the TCP server and spawns an async task for each accepted connection.
///
/// # Arguments
///
/// * `config` - Shared server configuration used for binding, serving files, and logging.
pub async fn serve_http(config: Arc<Config>) {
    let addr = format!("{}:{}", config.server.addr, config.server.http_port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect(&format!("Ferrox failed to bind on http://{addr}"));

    println!("Ferrox running on http://{addr}");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(res) => res,
            Err(e) => {
                logger::error_log(&config, "core", format!("Failed to accept: {}", e)).await;
                continue;
            }
        };

        let task_config: Arc<Config> = Arc::clone(&config);
        let peer_ip = stream.peer_addr().map(|a| a.ip()).unwrap_or(UNSPECIFIED_IP);
        let local_ip = stream
            .local_addr()
            .map(|a| a.ip())
            .unwrap_or(UNSPECIFIED_IP);

        tokio::spawn(async move {
            if let Err(e) = handle(stream, Arc::clone(&task_config), peer_ip, local_ip).await {
                logger::error_log(&task_config, "core", format!("Connection error: {}", e)).await;
            }
        });
    }
}

/// Starts the HTTP listener that redirects incoming requests to the HTTPS endpoint.
///
/// # Arguments
///
/// * `config` - Shared server configuration used for binding, redirect targets, and logging.
pub async fn serve_http_redirect(config: Arc<Config>) {
    let addr = format!("{}:{}", config.server.addr, config.server.http_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect(&format!("Ferrox failed to bind on http://{addr}"));

    println!("HTTP Redirector running on http://{}", addr);

    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(res) => res,
            Err(_) => continue,
        };

        let task_config = Arc::clone(&config);
        let peer_ip = stream.peer_addr().map(|a| a.ip()).unwrap_or(UNSPECIFIED_IP);
        let local_ip = stream
            .local_addr()
            .map(|a| a.ip())
            .unwrap_or(UNSPECIFIED_IP);

        tokio::spawn(async move {
            let timeout_duration = Duration::from_secs(CONNECTION_TIMEOUT_SEC);

            let request_head: Vec<u8> = match tokio::time::timeout(
                timeout_duration,
                read_request_head(&mut stream, MAX_HEADER_SIZE),
            )
            .await
            {
                Ok(Ok(h)) => h,
                Ok(Err(e)) => {
                    logger::error_log(&task_config, "core", format!("Connection error: {}", e))
                        .await;
                    vec![]
                }
                Err(_) => {
                    logger::error_log(
                        &task_config,
                        "core",
                        "HTTP Redirect connection timed out".to_string(),
                    )
                    .await;
                    vec![]
                }
            };

            if !request_head.is_empty() {
                if let Ok(request) = Request::parse(&request_head) {
                    let https_port_str = if task_config.server.https_port == 443 {
                        "".to_string()
                    } else {
                        format!(":{}", task_config.server.https_port)
                    };

                    let local_ip_str = local_ip.to_string();
                    let host = request
                        .headers
                        .get("Host")
                        .map(|s| s.as_str())
                        .unwrap_or(&local_ip_str);
                    let clean_host = host.split(':').next().unwrap_or(host);

                    let redirect_response = Response::redirect(
                        "301 Moved Permanently",
                        &format!("https://{}{}{}", clean_host, https_port_str, request.path),
                    );

                    match redirect_response
                        .write_headers(&mut stream, &task_config)
                        .await
                    {
                        Ok(()) => (),
                        Err(e) => {
                            logger::error_log(
                                &task_config,
                                "core",
                                format!("Failed to redirect to https: {}", e),
                            )
                            .await;
                            return;
                        }
                    };

                    logger::access(
                        &task_config,
                        &request,
                        &redirect_response,
                        peer_ip,
                        local_ip,
                    )
                    .await;
                }
            }
        });
    }
}

/// Starts the TLS-encrypted TCP server and spawns an async task for each accepted connection.
///
/// # Arguments
///
/// * `config` - Shared server configuration used for binding, serving files, and logging.
pub async fn serve_https(config: Arc<Config>) {
    let addr = format!("{}:{}", config.server.addr, config.server.https_port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect(&format!("Ferrox failed to bind on https://{addr}"));

    let tls_server_config = load_tls_config(&config).expect("Failed to load TLS configuration");
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_server_config));

    println!("Ferrox running on https://{addr}");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(res) => res,
            Err(e) => {
                logger::error_log(&config, "core", format!("Failed to accept: {}", e)).await;
                continue;
            }
        };

        let task_config: Arc<Config> = Arc::clone(&config);
        let acceptor: TlsAcceptor = tls_acceptor.clone();
        let peer_ip = stream.peer_addr().map(|a| a.ip()).unwrap_or(UNSPECIFIED_IP);
        let local_ip = stream
            .local_addr()
            .map(|a| a.ip())
            .unwrap_or(UNSPECIFIED_IP);

        tokio::spawn(async move {
            let timeout_duration = Duration::from_secs(CONNECTION_TIMEOUT_SEC);

            let tls_stream = match tokio::time::timeout(timeout_duration, acceptor.accept(stream))
                .await
            {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    logger::error_log(&task_config, "tls", format!("TLS handshake failed: {}", e))
                        .await;
                    return;
                }
                Err(_) => {
                    logger::error_log(&task_config, "tls", "TLS handshake timed out".to_string())
                        .await;
                    return;
                }
            };

            if let Err(e) = handle(tls_stream, Arc::clone(&task_config), peer_ip, local_ip).await {
                logger::error_log(&task_config, "core", format!("Connection error: {}", e)).await;
            }
        });
    }
}

/// Reads a single HTTP request from the stream and sends the generated response.
///
/// # Arguments
///
/// * `stream` - The TCP stream connected to the client.
/// * `config` - Shared server configuration used for parsing, file serving, and logging.
async fn handle<S>(
    mut stream: S,
    config: Arc<Config>,
    peer_ip: IpAddr,
    local_ip: IpAddr,
) -> std::io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let request_head: Vec<u8> = match tokio::time::timeout(
        Duration::from_secs(CONNECTION_TIMEOUT_SEC),
        read_request_head(&mut stream, MAX_HEADER_SIZE),
    )
    .await
    {
        Ok(Ok(head)) => head,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Client timed out while sending request headers.",
            ));
        }
    };

    if request_head.is_empty() {
        return Ok(());
    }

    let request = match Request::parse(&request_head) {
        Ok(r) => r,
        Err(e) => {
            logger::error_log(
                &config,
                "parser",
                format!("Failed to parse http request: {}", e),
            )
            .await;

            let error_res = Response::error("400", "Bad Request");
            let _ = error_res.write_headers(&mut stream, &config).await?;
            if let Body::Bytes(b) = error_res.body {
                let _ = stream.write_all(&b).await;
            }

            return Ok(());
        }
    };

    let decoded_path = match decode(&request.path) {
        Ok(p) => p.into_owned(),
        Err(_) => {
            let error_res = Response::error("400", "Bad Request");
            let _ = error_res.write_headers(&mut stream, &config).await?;
            if let Body::Bytes(b) = error_res.body {
                let _ = stream.write_all(&b).await;
            }

            return Ok(());
        }
    };

    let mut response: Response = match serve_file(
        &decoded_path,
        &config.paths.serve_dir,
        &config.server.router,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            logger::error_log(
                &config,
                "file",
                format!("Failed to server static file: {}", e),
            )
            .await;
            Response::error("500", "Internal Server Error")
        }
    };

    response.write_headers(&mut stream, &config).await?;

    match &mut response.body {
        Body::Bytes(bytes) => {
            stream.write_all(bytes).await?;
        }
        Body::File(file) => {
            tokio::io::copy(file, &mut stream).await?;
        }
    }

    logger::access(&config, &request, &response, peer_ip, local_ip).await;

    Ok(())
}

/// Loads the TLS certificate chain and private key into a server configuration.
///
/// # Arguments
///
/// * `config` - The application configuration containing certificate and key file paths.
fn load_tls_config(config: &Config) -> std::io::Result<ServerConfig> {
    // Reading private key
    let cert_file = File::open(&config.tls.cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut cert_reader).collect::<std::result::Result<Vec<_>, _>>()?;

    // Reading public key
    let key_file = File::open(&config.tls.key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let key: PrivateKeyDer<'static> =
        rustls_pemfile::private_key(&mut key_reader)?.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "No private key found")
        })?;

    // Assembling tls config
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(server_config)
}

/// Reads request head and returns it's vector.
///
/// # Arguments
///
/// * `stream` - The TCP stream connected to the client.
/// * `max_header_size` - Max header size authorized.
async fn read_request_head<S>(stream: &mut S, max_header_size: u64) -> std::io::Result<Vec<u8>>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let mut full_data: Vec<u8> = Vec::with_capacity(1024);
    let mut temp_buffer: [u8; 1024] = [0u8; 1024];
    let mut search_start: usize = 0;

    loop {
        let bytes_read: usize = stream.read(&mut temp_buffer).await?;
        let check_start: usize = search_start.saturating_sub(3);

        if bytes_read == 0 {
            if full_data.is_empty() {
                return Ok(vec![]);
            } else {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Client disconnected before sending full request headers.",
                ));
            }
        }

        full_data.extend_from_slice(&temp_buffer[..bytes_read]);

        if full_data[check_start..]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            break;
        }

        if full_data.len() > max_header_size as usize {
            return Err(Error::new(
                ErrorKind::ArgumentListTooLong,
                "Max header size reached.",
            ));
        }

        search_start = full_data.len();
    }

    Ok(full_data)
}
