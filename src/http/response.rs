use mime_guess::{Mime, mime};
use std::{fs::File, io::Write};

use crate::utils::templates::render_error;

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
    pub fn write_headers<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write!(
            writer,
            "HTTP/1.1 {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Server: Ferrox\r\n\
         X-Content-Type-Options: nosniff\r\n\
         X-Frame-Options: DENY\r\n",
            self.status, self.content_type, self.content_length,
        )?;

        for (key, value) in &self.headers {
            write!(writer, "{}: {}\r\n", key, value)?;
        }

        writer.write_all(b"\r\n")?;

        Ok(())
    }

    pub fn new_html(code: &str, body: Vec<u8>) -> Response {
        Response {
            status: code.to_string(),
            content_type: mime::TEXT_HTML,
            content_length: body.len() as u64,
            headers: vec![],
            body: Body::Bytes(body),
        }
    }

    pub fn redirect(code: &str, to: &str) -> Response {
        Response {
            status: code.to_string(),
            content_type: mime::TEXT_HTML,
            content_length: 0,
            headers: vec![("Location".into(), format!("{to}").into())],
            body: Body::Bytes(vec![]),
        }
    }

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
