use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};

/// Represents a parsed HTTP request line together with its header map.
pub struct Request<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub version: &'a str,
    headers: HashMap<&'a str, &'a str>,
}

impl<'a> Request<'a> {
    /// Parses a raw HTTP request buffer into a [`Request`] value.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The raw bytes read from the client connection.
    pub fn parse(buffer: &'a [u8]) -> Result<Self> {
        let header_end = Self::find_headers_end(buffer)
            .ok_or(Error::new(ErrorKind::InvalidData, "No header terminator."))?;

        let header_bytes = &buffer[..header_end];
        let mut lines = header_bytes.split(|&b| b == b'\n');

        let first_line = lines
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "Missing request line."))?;

        let first_line = Self::strip_cr(first_line);

        let (method, path, version) = Self::split_request_line(first_line)?;

        let mut headers: HashMap<&'a str, &'a str> = HashMap::new();

        for line in lines {
            let line = Self::strip_cr(line);

            if line.is_empty() {
                break;
            }

            let (key, value) = Self::parse_header(line)?;

            let key = std::str::from_utf8(key)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid header key."))?;

            let val_str = std::str::from_utf8(value)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid header value."))?;

            let val_str = val_str.trim();

            headers.insert(key, val_str);
        }

        Ok(Self {
            method: std::str::from_utf8(method)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8."))?,
            path: std::str::from_utf8(path)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8."))?,
            version: std::str::from_utf8(version)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8."))?,
            headers,
        })
    }

    /// Retrieves a header value using a case-insensitive search. Optimized for zero-copy.
    /// 
    /// # Arguments
    /// 
    /// * `search_key` - header key to retreive from request.
    pub fn header(&self, search_key: &str) -> Option<&'a str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(search_key))
            .map(|(_, v)| *v)
    }

    /// Finds the byte offset where the HTTP header block terminates.
    ///
    /// # Arguments
    ///
    /// * `buf` - The raw request buffer to scan for the `\r\n\r\n` terminator.
    fn find_headers_end(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|w| w == b"\r\n\r\n")
    }

    /// Splits the HTTP request line into method, path, and version slices.
    ///
    /// # Arguments
    ///
    /// * `line` - The first line of the HTTP request without the trailing carriage return.
    fn split_request_line(line: &[u8]) -> Result<(&[u8], &[u8], &[u8])> {
        let mut parts = line.split(|&b| b == b' ');

        let method = parts
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "No method."))?;
        let path = parts
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "No path."))?;
        let version = parts
            .next()
            .ok_or(Error::new(ErrorKind::InvalidData, "No version."))?;

        if parts.next().is_some() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Too many parts in request line.",
            ));
        }

        Ok((method, path, version))
    }

    /// Parses a single HTTP header line into trimmed key and value slices.
    ///
    /// # Arguments
    ///
    /// * `line` - A raw header line in the form `name: value`.
    fn parse_header(line: &[u8]) -> Result<(&[u8], &[u8])> {
        let pos = line
            .iter()
            .position(|&b| b == b':')
            .ok_or(Error::new(ErrorKind::InvalidData, "Malformed header."))?;

        let key = &line[..pos];
        let value = &line[pos + 1..];

        Ok((Self::trim(key), Self::trim(value)))
    }

    /// Removes leading and trailing ASCII spaces from a byte slice.
    ///
    /// # Arguments
    ///
    /// * `s` - The byte slice to trim in place by adjusting its bounds.
    fn trim(mut s: &[u8]) -> &[u8] {
        while s.first() == Some(&b' ') {
            s = &s[1..];
        }
        while s.last() == Some(&b' ') {
            s = &s[..s.len() - 1];
        }
        s
    }

    /// Removes a trailing carriage return from a line when present.
    ///
    /// # Arguments
    ///
    /// * `line` - The request or header line that may end with `\r`.
    fn strip_cr(line: &[u8]) -> &[u8] {
        if line.ends_with(b"\r") {
            &line[..line.len() - 1]
        } else {
            line
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Request;
    use std::io::ErrorKind;

    #[test]
    fn parses_request_line_and_normalizes_headers() {
        let raw = b"GET /docs HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Ferrox Test\r\nX-Value:  spaced value  \r\n\r\n";

        let request = Request::parse(raw).expect("request should parse");

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/docs");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(
            request.header("host"),
            Some("example.com")
        );
        assert_eq!(
            request.header("user-agent"),
            Some("Ferrox Test")
        );
        assert_eq!(
            request.header("x-value"),
            Some("spaced value")
        );
    }

    #[test]
    fn rejects_request_without_header_terminator() {
        let err = Request::parse(b"GET / HTTP/1.1\r\nHost: example.com")
            .err()
            .expect("request without terminator must fail");

        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_request_line_with_too_many_parts() {
        let err = Request::parse(b"GET / HTTP/1.1 EXTRA\r\nHost: example.com\r\n\r\n")
            .err()
            .expect("malformed request line must fail");

        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_malformed_header() {
        let err = Request::parse(b"GET / HTTP/1.1\r\nHost example.com\r\n\r\n")
            .err()
            .expect("malformed header must fail");

        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }
}
