use std::{fs::File, io::Write};
use mime_guess::Mime;

pub struct Response {
    pub status: &'static str,
    pub content_type: Mime,
    pub content_length: u64,
    pub body: Body
}

pub enum Body {
    Bytes(Vec<u8>),
    File(File),
}

impl Response {
    pub fn write_headers<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let headers = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             Server: Ferrox\r\n\
             \r\n",
            self.status,
            self.content_type.to_string(),
            self.content_length
        );

        writer.write_all(headers.as_bytes())?;

        Ok(())
    }
}