use mime_guess::Mime;
use std::{fs::File, io::Write};

pub struct Response {
    pub status: &'static str,
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
         Server: Ferrox\r\n",
            self.status, self.content_type, self.content_length,
        )?;

        for (key, value) in &self.headers {
            write!(writer, "{}: {}\r\n", key, value)?;
        }

        writer.write_all(b"\r\n")?;

        Ok(())
    }
}
