use mime_guess::{Mime, mime};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::utils::templates::render_error;

/// Stores the status line, headers, content metadata, and body of an HTTP response.
pub struct Response {
    pub status: String,
    pub content_type: Mime,
    pub content_length: u64,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

pub enum Body {
    Bytes(Vec<u8>),
    File(File),
}

impl Response {
    /// Serializes the HTTP response headers and writes them to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - The async writer that receives the serialized header block.
    /// * `config` - The application configuration containing global response headers.
    pub async fn write_headers<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        config: &Config,
        connection: &str
    ) -> std::io::Result<()> {
        let mut header_string = format!(
            "HTTP/1.1 {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: {}\r\n\
         Server: Ferrox\r\n",
            self.status, self.content_type, self.content_length, connection
        );

        if connection.eq_ignore_ascii_case("keep-alive") {
            // For now, we hardcode timeout to 10 as it matches CONNECTION_TIMEOUT constant in server.rs, but this will be replaced when we introduce timeout in configuration.
            header_string.push_str("Keep-Alive: timeout=10\r\n");
        }

        for (key, value) in &config.headers {
            header_string.push_str(&format!("{}: {}\r\n", key, value));
        }

        for (key, value) in &self.headers {
            header_string.push_str(&format!("{}: {}\r\n", key, value));
        }

        header_string.push_str("\r\n");

        writer.write_all(header_string.as_bytes()).await?;

        Ok(())
    }

    /// Creates an HTML response backed by an in-memory byte buffer.
    ///
    /// # Arguments
    ///
    /// * `code` - The HTTP status line, such as `200 OK`.
    /// * `body` - The HTML payload that will be sent to the client.
    pub fn new_html(code: &str, body: Vec<u8>) -> Response {
        Response {
            status: code.to_string(),
            content_type: mime::TEXT_HTML,
            content_length: body.len() as u64,
            headers: vec![],
            body: Body::Bytes(body),
        }
    }

    /// Creates a redirect response with a `Location` header.
    ///
    /// # Arguments
    ///
    /// * `code` - The HTTP redirect status line, such as `301 Moved Permanently`.
    /// * `to` - The target URL or path placed into the `Location` header.
    pub fn redirect(code: &str, to: &str) -> Response {
        Response {
            status: code.to_string(),
            content_type: mime::TEXT_HTML,
            content_length: 0,
            headers: vec![("Location".into(), format!("{to}").into())],
            body: Body::Bytes(vec![]),
        }
    }

    /// Renders a standard HTML error response for the supplied status and message.
    ///
    /// # Arguments
    ///
    /// * `code` - The numeric HTTP status code.
    /// * `message` - The human-readable status message displayed in the template.
    pub fn error(code: &str, message: &str) -> Response {
        let body: Vec<u8> = render_error(code, message);

        Response {
            status: format!("{code} {message}"),
            content_type: mime::TEXT_HTML,
            content_length: body.len() as u64,
            headers: vec![],
            body: Body::Bytes(body),
        }
    }
}
