use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

pub struct Request {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
}

impl Request {
    pub fn parse(buffer: &[u8]) -> Result<Self> {
        let request: std::borrow::Cow<'_, str> = String::from_utf8_lossy(buffer);
        let mut lines = request.lines();

        let first_line = lines
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No first line found."))?;

        let parts: Vec<&str> = first_line.split_whitespace().collect();

        let method = parts
            .get(0)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No method found."))?;
        let path = parts
            .get(1)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No request path found."))?;
        let version = parts
            .get(2)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No protocol version found."))?;

        let mut headers: HashMap<String, String> = HashMap::new();

        for line in lines {
            if line.is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
            version: version.to_string(),
            headers,
        })
    }
}
