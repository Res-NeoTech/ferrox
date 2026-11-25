use std::path::{Path, PathBuf};

use html_escape::encode_safe;
use mime_guess;
use tokio::fs::File;
use urlencoding::decode;

use crate::{
    config::RouterPreset,
    http::response::{Body, Response},
    utils::templates::render_indexing,
};

/// Resolves a requested path inside the configured serving directory and returns a response.
///
/// # Arguments
///
/// * `raw_path` - The request path as sent by the client (still percent-encoded, no query).
/// * `query` - The query string, if any. Preserved in directory redirects.
/// * `serving_dir` - The root directory from which static files are served.
/// * `logic` - Router preset (static files or SPA fallback).
/// * `index` - Whether directory listing is enabled.
pub async fn serve_file(
    raw_path: &str,
    query: Option<&str>,
    serving_dir: &str,
    logic: &RouterPreset,
    index: &bool,
) -> Result<Response, std::io::Error> {
    // Decoded form is only used to find the file on disk; redirects reuse `raw_path`.
    let file_path = match decode(raw_path) {
        Ok(p) => p,
        Err(_) => return Ok(Response::error("400", "Bad Request")),
    };
    let file_path: &str = &file_path;

    let base = tokio::fs::canonicalize(&serving_dir).await?;
    let requested_path = base.join(file_path.trim_start_matches('/'));

    let canonical = match tokio::fs::canonicalize(&requested_path).await {
        Ok(p) if p.starts_with(&base) => p,
        Ok(_) => return Ok(Response::error("403", "Forbidden")),
        Err(_) => {
            if logic == &RouterPreset::Spa {
                return spa_fallback(&base).await;
            } else {
                return Ok(Response::error("404", "Not Found"));
            }
        }
    };

    if canonical.is_dir() {
        if logic == &RouterPreset::Static {
            if !file_path.ends_with('/') {
                let location = match query {
                    Some(q) => format!("{raw_path}/?{q}"),
                    None => format!("{raw_path}/"),
                };
                return Ok(Response::redirect("301 Moved Permanently", &location));
            }

            let index_html = canonical.join("index.html");

            if tokio::fs::try_exists(&index_html).await.unwrap_or(false) {
                return serve_actual_file(index_html).await;
            }

            if *index {
                return match index_files(canonical, file_path).await {
                    Ok(body) => Ok(Response::new_html("200 OK", body)),
                    Err(_) => Ok(Response::error("403", "Forbidden")),
                };
            } else {
                return Ok(Response::error("403", "Forbidden"));
            }
        } else {
            return spa_fallback(&base).await;
        }
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

/// Serves the `index.html` file from the base directory as a fallback for SPA routing.
///
/// This function is called when a requested path does not directly map to a file or directory
/// and the router is configured for Single Page Application (SPA) mode.
///
/// # Arguments
/// * `base` - The base directory from which static files are served.
async fn spa_fallback(base: &Path) -> Result<Response, std::io::Error> {
    let fallback = base.join("index.html");

    if tokio::fs::try_exists(&fallback).await.unwrap_or(false) {
        serve_actual_file(fallback).await
    } else {
        Ok(Response::error("404", "Not Found"))
    }
}

/// Builds an HTML directory listing for a filesystem path.
///
/// # Arguments
///
/// * `path` - The canonical directory path whose contents should be listed.
/// * `display_path` - The request path displayed as the page title.
async fn index_files(path: PathBuf, display_path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut dir_entries = tokio::fs::read_dir(&path).await?;
    let mut html_list = String::new();

    if display_path != "/" {
        html_list.push_str("<li><a href=\"..\">..</a></li>");
    }

    while let Some(entry) = dir_entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type().await?;

        let href = if file_type.is_dir() {
            format!("{}/", name)
        } else {
            name
        };

        html_list.push_str(&format!(
            "<li><a href=\"{save_href}\">{save_href}</a></li>",
            save_href = encode_safe(&href)
        ));
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

    // Static files test

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
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");

        // ACT
        let response = serve_file(
            "/docs",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false,
        )
        .await
        .expect("directory request should succeed");

        // ASSERT
        assert_eq!(response.status, "301 Moved Permanently");
        assert_eq!(
            response.headers,
            vec![("Location".to_string(), "/docs/".to_string())]
        );

        cleanup(&root);
    }

    #[tokio::test]
    async fn directory_redirect_preserves_query() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");

        // ACT
        let response = serve_file(
            "/docs",
            Some("page=2&sort=asc"),
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false,
        )
        .await
        .expect("directory request should succeed");

        // ASSERT
        assert_eq!(response.status, "301 Moved Permanently");
        assert_eq!(
            response.headers,
            vec![("Location".to_string(), "/docs/?page=2&sort=asc".to_string())]
        );

        cleanup(&root);
    }

    #[tokio::test]
    async fn directory_redirect_keeps_path_percent_encoded() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("my docs"))
            .expect("dir with space should be created");

        // ACT
        let response = serve_file(
            "/my%20docs",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false,
        )
        .await
        .expect("directory request should succeed");

        // ASSERT
        assert_eq!(response.status, "301 Moved Permanently");
        assert_eq!(
            response.headers,
            vec![("Location".to_string(), "/my%20docs/".to_string())]
        );

        cleanup(&root);
    }

    #[tokio::test]
    async fn invalid_percent_encoding_returns_bad_request() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();

        // ACT: %FF decodes to a byte that isn't valid UTF-8.
        let response = serve_file(
            "/%FF",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false,
        )
        .await
        .expect("invalid encoding should return a response");

        // ASSERT
        assert_eq!(response.status, "400 Bad Request");

        cleanup(&root);
    }

    #[tokio::test]
    async fn blocks_path_traversal_outside_serving_root() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::write(root.join("secret.txt"), "secret").expect("outside file should be created");

        // ACT
        let response = serve_file(
            "/../secret.txt",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false,
        )
        .await
        .expect("traversal attempt should return a response");

        // ASSERT
        assert_eq!(response.status, "403 Forbidden");

        cleanup(&root);
    }

    #[tokio::test]
    async fn serves_index_html_from_directory() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");
        fs::write(
            Path::new(&serve_dir).join("docs").join("index.html"),
            "<h1>Docs</h1>",
        )
        .expect("index file should be written");

        // ACT
        let response = serve_file(
            "/docs/",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false,
        )
        .await
        .expect("directory request should succeed");

        match response.body {
            Body::File(_) => {}
            Body::Bytes(_) => panic!("expected file-backed response"),
        }

        // ASSERT
        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type.essence_str(), "text/html");

        cleanup(&root);
    }

    #[tokio::test]
    async fn directory_listing_hides_dotfiles_and_marks_subdirectories() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        let docs = Path::new(&serve_dir).join("docs");
        fs::create_dir_all(docs.join("nested")).expect("nested dir should be created");
        fs::write(docs.join("file.txt"), "visible").expect("visible file should be written");
        fs::write(docs.join(".hidden"), "hidden").expect("hidden file should be written");

        // ACT
        let response = serve_file(
            "/docs/",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &true,
        )
        .await
        .expect("directory request should succeed");

        let body = match response.body {
            Body::Bytes(bytes) => String::from_utf8(bytes).expect("listing should be utf-8"),
            Body::File(_) => panic!("expected in-memory directory listing"),
        };

        // ASSERT
        assert!(body.contains("file.txt"), "body was: {body}");
        assert!(body.contains("nested&#x2F;"), "body was: {body}");
        assert!(!body.contains(".hidden"), "body was: {body}");

        cleanup(&root);
    }

    #[tokio::test]
    async fn directory_listing_returns_forbidden_when_index_disabled() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        let docs = Path::new(&serve_dir).join("docs");
        fs::create_dir_all(docs.join("nested")).expect("nested dir should be created");
        fs::write(docs.join("file.txt"), "visible").expect("visible file should be written");

        // ACT
        let response = serve_file(
            "/docs/",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Static,
            &false, // Disable indexing
        )
        .await
        .expect("directory request should succeed");

        // ASSERT
        assert_eq!(response.status, "403 Forbidden");

        cleanup(&root);
    }

    // SPA test

    #[tokio::test]
    async fn serves_index_when_spa() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::write(
            Path::new(&serve_dir).join("index.html"),
            "<h1>Hey there!</h1>",
        )
        .expect("index file should be written");

        // ACT
        let response = serve_file(
            "/docs/getting-started",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Spa,
            &false,
        )
        .await
        .expect("directory request should succeed");

        match response.body {
            Body::File(_) => {}
            Body::Bytes(_) => panic!("expected file-backed response"),
        }

        // ASSERT
        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type.essence_str(), "text/html");

        cleanup(&root);
    }

    #[tokio::test]
    async fn not_found_when_no_index_but_spa() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();

        // ACT
        let response = serve_file(
            "/docs/getting-started",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Spa,
            &false,
        )
        .await
        .expect("directory request should succeed");

        match response.body {
            Body::Bytes(_) => {}
            Body::File(_) => panic!("expected byte-backed response"),
        }

        // ASSERT
        assert_eq!(response.status, "404 Not Found");

        cleanup(&root);
    }

    #[tokio::test]
    async fn serves_index_when_dir_exists() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::write(
            Path::new(&serve_dir).join("index.html"),
            "<h1>Hey there!</h1>",
        )
        .expect("index file should be written");
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");

        // ACT
        let response = serve_file(
            "/docs",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Spa,
            &false,
        )
        .await
        .expect("directory request should succeed");

        match response.body {
            Body::File(_) => {}
            Body::Bytes(_) => panic!("expected file-backed response"),
        }

        // ASSERT
        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type.essence_str(), "text/html");

        cleanup(&root);
    }

    #[tokio::test]
    async fn serves_actual_file_when_spa() {
        // ARRANGE
        let (root, serve_dir) = create_serving_layout();
        fs::create_dir_all(Path::new(&serve_dir).join("docs")).expect("docs dir should be created");
        fs::write(
            Path::new(&serve_dir).join("docs").join("doc.js"),
            "alert('Ferrox is faster than Leclerc!')",
        )
        .expect("text file should be written");

        // ACT
        let response = serve_file(
            "/docs/doc.js",
            None,
            &serve_dir,
            &crate::config::RouterPreset::Spa,
            &false,
        )
        .await
        .expect("directory request should succeed");

        match response.body {
            Body::File(_) => {}
            Body::Bytes(_) => panic!("expected file-backed response"),
        }

        // ASSERT
        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type.essence_str(), "text/javascript");

        cleanup(&root);
    }
}
