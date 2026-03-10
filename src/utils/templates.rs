const ERROR_TEMPLATE: &str = include_str!("../../templates/error.html");
const INDEXING_TEMPLATE: &str = include_str!("../../templates/indexing.html");

pub fn render_error(code: &str, message: &str) -> Vec<u8> {
    ERROR_TEMPLATE.replace("{{CODE}}", code).replace("{{MESSAGE}}", message).into_bytes()
}

pub fn render_indexing(title: &str, list: &str) -> Vec<u8> {
    INDEXING_TEMPLATE.replace("{{TITLE}}", title).replace("{{LISTING}}", list).into_bytes()
}