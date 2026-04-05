use html_escape::encode_safe;

const ERROR_TEMPLATE: &str = include_str!("../../templates/error.html");
const INDEXING_TEMPLATE: &str = include_str!("../../templates/indexing.html");

/// Renders an error page by replacing placeholders in the error template.
///
/// # Arguments
///
/// * `code` - The HTTP error status code.
/// * `message` - The error message to display.
pub fn render_error(code: &str, message: &str) -> Vec<u8> {
    ERROR_TEMPLATE
        .replace("{{CODE}}", code)
        .replace("{{MESSAGE}}", message)
        .into_bytes()
}

/// Renders a directory indexing page by replacing placeholders in the indexing template.
///
/// # Arguments
///
/// * `title` - The title of the indexing page, typically the directory name.
/// * `list` - The HTML list of files and directories within the directory.
pub fn render_indexing(title: &str, list: &str) -> Vec<u8> {
    let safe_title = encode_safe(title);
    INDEXING_TEMPLATE
        .replace("{{TITLE}}", &safe_title)
        .replace("{{LISTING}}", list)
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{render_error, render_indexing};

    #[test]
    fn render_error_inserts_status_and_message() {
        let rendered = String::from_utf8(render_error("404", "Not Found")).expect("valid utf-8");

        assert!(rendered.contains("404"));
        assert!(rendered.contains("Not Found"));
    }

    #[test]
    fn render_indexing_escapes_title_but_keeps_listing_markup() {
        let rendered = String::from_utf8(render_indexing(
            r#"<script>alert("x")</script>"#,
            "<li><a href=\"file.txt\">file.txt</a></li>",
        ))
        .expect("valid utf-8");

        assert!(
            rendered.contains("&lt;script&gt;"),
            "rendered output: {rendered}"
        );
        assert!(rendered.contains("alert"), "rendered output: {rendered}");
        assert!(rendered.contains("script"), "rendered output: {rendered}");
        assert!(rendered.contains("<li><a href=\"file.txt\">file.txt</a></li>"));
    }
}
