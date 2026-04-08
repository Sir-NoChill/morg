use std::path::Path;

use morg_parser::span::Span;

pub fn format_location(file: &Path, span: &Span) -> String {
    format!("{}:{}", file.display(), span.line)
}

pub fn format_duration_minutes(minutes: u64) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours > 0 && mins > 0 {
        format!("{hours}h{mins}m")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{mins}m")
    }
}
