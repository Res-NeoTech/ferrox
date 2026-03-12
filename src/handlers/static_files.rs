use crate::http::response::{Body, Response};
use crate::utils::templates::{render_indexing};
use html_escape::encode_safe;
use mime_guess::{self};
use std::path::Path;
use std::{fs::File, path::PathBuf};

const SERVING_DIR: &str = "www";

pub fn serve_file(file_path: &String) -> Result<Response, std::io::Error> {
    let base = Path::new(SERVING_DIR).canonicalize()?;
    let requested_path = base.join(file_path.trim_start_matches('/'));

    let canonical = match requested_path.canonicalize() {
        Ok(p) if p.starts_with(&base) => p,
        Ok(_) => return Ok(Response::error("403", "Forbidden")),
        Err(_) => return Ok(Response::error("404", "Not Found")),
    };

    if canonical.is_dir() {
        if !file_path.ends_with('/') {
            return Ok(Response::redirect("301 Moved Permanently", &format!("{}/", file_path)));
        }

        let index_html = canonical.join("index.html");
        
        if index_html.exists() {
            return serve_actual_file(index_html);
        }

        return match index_files(canonical, file_path) {
            Ok(body) => Ok(Response::new_html("200 OK", body)),
            Err(_) => Ok(Response::error("403", "Forbidden")),
        };
    }

    serve_actual_file(canonical)
}

fn serve_actual_file(path: PathBuf) -> Result<Response, std::io::Error> {
    let file = File::open(&path)?;
    let metadata = file.metadata()?;
    let mime = mime_guess::from_path(&path).first_or_text_plain();

    Ok(Response {
        status: "200 OK".to_string(),
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