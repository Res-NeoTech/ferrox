const TEMPLATE: &str = include_str!("../../templates/error.html");

pub fn render_error(code: &str, message: &str) -> Vec<u8> {
    TEMPLATE.replace("{{CODE}}", code).replace("{{MESSAGE}}", message).into_bytes()
}