use yansi::{Color, Paint};

/// Format a warning message with optional styles.
pub fn format_warning(enable_styles: bool, message: &str) -> String {
    format_msg(enable_styles, message, "Warning: ", Color::Yellow)
}

/// Format an anyhow error message with optional styles.
pub fn format_error(enable_styles: bool, error: &anyhow::Error) -> String {
    format_msg(enable_styles, &format!("{error:?}"), "Error: ", Color::Red)
}

fn format_msg(enable_styles: bool, message: &str, prefix: &'static str, color: Color) -> String {
    if enable_styles {
        format!("{}{}", prefix.paint(color), message.paint(color))
    } else {
        message.to_string()
    }
}
