use crate::{http::error::render_error, http::response::Response};
use mime_guess::{self, mime};
use std::path::PathBuf;

const SERVING_DIR: &str = "www";

pub fn serve_file(file_path: &String) -> Result<Response, std::io::Error> {
    let path = PathBuf::from(SERVING_DIR).join(file_path.trim_start_matches('/'));
    let base = PathBuf::from(SERVING_DIR)
        .canonicalize()
        .expect("Serving dir must exist");

    let mut canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let body = render_error("404", "Not Found");
            return Ok(Response {
                status: "404 Not Found",
                body,
                content_type: mime::TEXT_HTML,
            });
        }
    };

    if !canonical.starts_with(&base) {
        let body = render_error("403", "Forbidden");

        return Ok(Response {
            status: "403 Forbidden",
            body,
            content_type: mime::TEXT_HTML,
        });
    }

    if canonical.is_dir() {
        canonical = canonical.join("index.html");
    }

    let body = std::fs::read(&canonical)?;

    let mime = mime_guess::from_path(&canonical).first_or_text_plain();

    Ok(Response {
        status: "200 OK",
        body,
        content_type: mime,
    })
}
