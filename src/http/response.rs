use mime_guess::Mime;

pub struct Response {
    pub status: &'static str,
    pub content_type: Mime,
    pub body: Vec<u8>,
}

impl Response {
    pub fn to_bytes(&self) -> Vec<u8> {
        let headers = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             Server: Ferrox\r\n\
             \r\n",
            self.status,
            self.content_type.to_string(),
            self.body.len()
        );

        let mut response = headers.into_bytes();
        response.extend_from_slice(&self.body);

        response
        // Divide headers and body sending to improve efficiency.
    }
}