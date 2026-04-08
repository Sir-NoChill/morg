use std::path::PathBuf;

use morg_parser::ast::{Block, Heading};
use morg_parser::tags::TagKind;

use crate::collect;
use crate::report;

pub fn run(paths: &[PathBuf], columns_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);
    let columns: Vec<&str> = columns_str.split(',').map(str::trim).collect();

    let mut rows: Vec<Vec<String>> = Vec::new();

    for pf in &parsed {
        collect_heading_rows(&pf.document.children, &pf.path, &columns, &mut rows);
    }

    if rows.is_empty() {
        println!("No headings found.");
        return Ok(());
    }

    // Compute column widths
    let col_widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let header_len = col.len();
            let max_data = rows.iter().map(|r| r.get(i).map(|s| s.len()).unwrap_or(0)).max().unwrap_or(0);
            header_len.max(max_data).max(4)
        })
        .collect();

    // Print header
    let header: String = columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{:width$}", col.to_uppercase(), width = col_widths[i]))
        .collect::<Vec<_>>()
        .join(" | ");
    println!("| {header} |");

    let separator: String = col_widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("-|-");
    println!("|-{separator}-|");

    // Print rows
    for row in &rows {
        let line: String = columns
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
                format!("{:width$}", val, width = col_widths[i])
            })
            .collect::<Vec<_>>()
            .join(" | ");
        println!("| {line} |");
    }

    println!("\n{} entries.", rows.len());
    Ok(())
}

fn collect_heading_rows(
    blocks: &[Block],
    file: &std::path::Path,
    columns: &[&str],
    rows: &mut Vec<Vec<String>>,
) {
    for block in blocks {
        if let Block::Heading(h) = block {
            // Skip archived headings
            if h.content.tags().iter().any(|t| matches!(t.kind, TagKind::Archive)) {
                continue;
            }

            let row: Vec<String> = columns.iter().map(|col| extract_column(h, file, col)).collect();

            // Only add if there's something interesting (not just an item name)
            let has_data = row.iter().skip(1).any(|v| !v.is_empty());
            if has_data {
                rows.push(row);
            }
        }
    }
}

fn extract_column(h: &Heading, file: &std::path::Path, col: &str) -> String {
    match col {
        "item" => h.content.plain_text(),
        "todo" => {
            let tags = h.content.tags();
            for tag in &tags {
                match &tag.kind {
                    TagKind::Todo { .. } => return "TODO".to_string(),
                    TagKind::Done { .. } => return "DONE".to_string(),
                    _ => {}
                }
            }
            String::new()
        }
        "priority" => {
            for tag in h.content.tags() {
                if let TagKind::Priority { level } = &tag.kind {
                    return level.to_string();
                }
            }
            // Check property drawer
            if let Some(ref props) = h.properties {
                if let Some(p) = props.entries.get("priority") {
                    return p.clone();
                }
            }
            String::new()
        }
        "effort" => {
            for tag in h.content.tags() {
                if let TagKind::Effort { minutes } = &tag.kind {
                    return report::format_duration_minutes(*minutes);
                }
            }
            if let Some(ref props) = h.properties {
                if let Some(e) = props.entries.get("effort") {
                    return e.clone();
                }
            }
            String::new()
        }
        "deadline" => {
            for tag in h.content.tags() {
                if let TagKind::Deadline { date, .. } = &tag.kind {
                    return date.to_string();
                }
            }
            String::new()
        }
        "scheduled" => {
            for tag in h.content.tags() {
                if let TagKind::Scheduled { date, .. } = &tag.kind {
                    return date.to_string();
                }
            }
            String::new()
        }
        "location" => report::format_location(file, &h.span),
        other => {
            // Try property drawer
            if let Some(ref props) = h.properties {
                if let Some(v) = props.entries.get(other) {
                    return v.clone();
                }
            }
            String::new()
        }
    }
}
