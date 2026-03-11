use crate::http::response::{Body, Response};
use crate::utils::templates::{render_error, render_indexing};
use mime_guess::{self, mime};
use std::{fs::File, path::PathBuf};
use html_escape::encode_safe;

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
                content_type: mime::TEXT_HTML,
                content_length: body.len() as u64,
                headers: vec![],
                body: Body::Bytes(body),
            });
        }
    };

    if !canonical.starts_with(&base) {
        let body = render_error("403", "Forbidden");

        return Ok(Response {
            status: "403 Forbidden",
            content_type: mime::TEXT_HTML,
            content_length: body.len() as u64,
            headers: vec![],
            body: Body::Bytes(body),
        });
    }

    if canonical.is_dir() {
        let index = canonical.join("index.html");

        if !index.exists() {
            if !file_path.ends_with("/") {
                return Ok(Response {
                    status: "301 Moved Permanently",
                    content_type: mime::TEXT_HTML,
                    content_length: 0,
                    headers: vec![("Location".into(), format!("{}/", file_path).into())],
                    body: Body::Bytes(vec![]),
                });
            }

            match index_files(canonical.clone(), file_path) {
                Ok(body) => {
                    return Ok(Response {
                        status: "200 OK",
                        content_type: mime::TEXT_HTML,
                        content_length: body.len() as u64,
                        headers: vec![],
                        body: Body::Bytes(body),
                    });
                }
                Err(_) => {
                    let body = render_error("403", "Forbidden");

                    return Ok(Response {
                        status: "403 Forbidden",
                        content_type: mime::TEXT_HTML,
                        content_length: body.len() as u64,
                        headers: vec![],
                        body: Body::Bytes(body),
                    });
                }
            }
        }

        canonical = index;
    }

    let file = File::open(&canonical)?;
    let metadata = file.metadata()?;
    let mime = mime_guess::from_path(&canonical).first_or_text_plain();

    Ok(Response {
        status: "200 OK",
        content_type: mime,
        content_length: metadata.len(),
        headers: vec![],
        body: Body::File(file),
    })
}

fn index_files(path: PathBuf, display_path: &String) -> Result<Vec<u8>, std::io::Error> {
    let dir_entries = std::fs::read_dir(&path)?;
    let mut html_list = String::new();

    if display_path != "/" {
        html_list.push_str("<li><a href=\"..\">..</a></li>");
    }

    for entry in dir_entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') { continue; }

        let href = if entry.file_type()?.is_dir() {
            format!("{}/", name)
        } else {
            name
        };

        html_list.push_str(&format!("<li><a href=\"{save_href}\">{save_href}</a></li>", save_href = encode_safe(&href)));
    }

    Ok(render_indexing(display_path, &html_list))
}