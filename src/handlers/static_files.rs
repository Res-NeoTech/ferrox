use crate::http::response::{Body, Response};
use crate::utils::templates::{render_indexing};
use html_escape::encode_safe;
use mime_guess::{self};
use std::path::Path;
use std::{path::PathBuf};
use tokio::fs::File;

/// Resolves a requested path inside the configured serving directory and returns a response.
///
/// # Arguments
///
/// * `file_path` - The request path extracted from the HTTP request line.
/// * `serving_dir` - The root directory from which static files are served.
pub async fn serve_file(file_path: &String, serving_dir: &String) -> Result<Response, std::io::Error> {
    let base = Path::new(&serving_dir).canonicalize()?;
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
            return serve_actual_file(index_html).await;
        }

        return match index_files(canonical, file_path) {
            Ok(body) => Ok(Response::new_html("200 OK", body)),
            Err(_) => Ok(Response::error("403", "Forbidden")),
        };
    }

    serve_actual_file(canonical).await
}

/// Opens a file on disk and wraps it in a streaming `200 OK` response.
///
/// # Arguments
///
/// * `path` - The canonical filesystem path of the file to serve.
async fn serve_actual_file(path: PathBuf) -> Result<Response, std::io::Error> {
    let file = File::open(&path).await?;
    let metadata = file.metadata().await?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    Ok(Response {
        status: "200 OK".to_string(),
        content_type: mime,
        content_length: metadata.len(),
        headers: vec![],
        body: Body::File(file),
    })
}

/// Builds an HTML directory listing for a filesystem path.
///
/// # Arguments
///
/// * `path` - The canonical directory path whose contents should be listed.
/// * `display_path` - The request path displayed as the page title.
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

#[cfg(test)]
mod tests {
    use super::serve_file;
    use crate::http::response::Body;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);

        std::env::temp_dir().join(format!(
            "ferrox-static-files-test-{}-{}-{}",
            std::process::id(),
            nanos,
            counter
        ))
    }

    fn create_serving_layout() -> (PathBuf, String) {
        let root = unique_test_dir();
        let serve_dir = root.join("serve");
        fs::create_dir_all(&serve_dir).expect("serve dir should be created");
        (root, serve_dir.to_string_lossy().to_string())
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn redirects_directory_without_trailing_slash() {
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");

        let response = serve_file(&"/docs".to_string(), &serve_dir)
            .await
            .expect("directory request should succeed");

        assert_eq!(response.status, "301 Moved Permanently");
        assert_eq!(response.headers, vec![("Location".to_string(), "/docs/".to_string())]);

        cleanup(&root);
    }

    #[tokio::test]
    async fn blocks_path_traversal_outside_serving_root() {
        let (root, serve_dir) = create_serving_layout();
        fs::write(root.join("secret.txt"), "secret").expect("outside file should be created");

        let response = serve_file(&"/../secret.txt".to_string(), &serve_dir)
            .await
            .expect("traversal attempt should return a response");

        assert_eq!(response.status, "403 Forbidden");

        cleanup(&root);
    }

    #[tokio::test]
    async fn serves_index_html_from_directory() {
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");
        fs::write(
            Path::new(&serve_dir).join("docs").join("index.html"),
            "<h1>Docs</h1>",
        )
        .expect("index file should be written");

        let response = serve_file(&"/docs/".to_string(), &serve_dir)
            .await
            .expect("directory request should succeed");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type.essence_str(), "text/html");
        match response.body {
            Body::File(_) => {}
            Body::Bytes(_) => panic!("expected file-backed response"),
        }

        cleanup(&root);
    }

    #[tokio::test]
    async fn directory_listing_hides_dotfiles_and_marks_subdirectories() {
        let (root, serve_dir) = create_serving_layout();
        let docs = Path::new(&serve_dir).join("docs");
        fs::create_dir_all(docs.join("nested")).expect("nested dir should be created");
        fs::write(docs.join("file.txt"), "visible").expect("visible file should be written");
        fs::write(docs.join(".hidden"), "hidden").expect("hidden file should be written");

        let response = serve_file(&"/docs/".to_string(), &serve_dir)
            .await
            .expect("directory request should succeed");

        let body = match response.body {
            Body::Bytes(bytes) => String::from_utf8(bytes).expect("listing should be utf-8"),
            Body::File(_) => panic!("expected in-memory directory listing"),
        };

        assert!(body.contains("file.txt"), "body was: {body}");
        assert!(body.contains("nested&#x2F;"), "body was: {body}");
        assert!(!body.contains(".hidden"), "body was: {body}");

        cleanup(&root);
    }
}
