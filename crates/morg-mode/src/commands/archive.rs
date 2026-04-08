use std::path::{Path, PathBuf};

use morg_parser::ast::{Block, Heading};
use morg_parser::tags::TagKind;

use crate::collect;

pub fn run(
    paths: &[PathBuf],
    suffix: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect::resolve_paths(paths);
    let mut total_archived = 0;

    for file_path in &files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warning: cannot read {}: {e}", file_path.display());
                continue;
            }
        };

        let result = morg_parser::parser::parse_document(&source);
        let lines: Vec<&str> = source.split('\n').collect();

        // Find archived heading ranges: (start_line, end_line) 0-indexed
        let archived_ranges = find_archived_ranges(&result.document.children, &lines);

        if archived_ranges.is_empty() {
            continue;
        }

        // Build archive file path
        let archive_path = archive_file_path(file_path, suffix);

        // Extract archived content
        let mut archived_content = String::new();
        for (start, end) in &archived_ranges {
            for line_idx in *start..*end {
                if line_idx < lines.len() {
                    archived_content.push_str(lines[line_idx]);
                    archived_content.push('\n');
                }
            }
            archived_content.push('\n');
        }

        // Build remaining content (source without archived ranges)
        let mut remaining = String::new();
        let mut skip_until: Option<usize> = None;
        for (line_idx, line) in lines.iter().enumerate() {
            if let Some(end) = skip_until {
                if line_idx < end {
                    continue;
                }
                skip_until = None;
            }

            let in_archive = archived_ranges.iter().any(|(s, e)| line_idx >= *s && line_idx < *e);
            if in_archive {
                // Find the end of this range and skip to it
                if let Some((_, end)) = archived_ranges.iter().find(|(s, _)| *s == line_idx) {
                    skip_until = Some(*end);
                }
                continue;
            }

            remaining.push_str(line);
            remaining.push('\n');
        }

        // Remove trailing extra newlines but keep one final newline
        while remaining.ends_with("\n\n\n") {
            remaining.pop();
        }

        let count = archived_ranges.len();
        total_archived += count;

        if dry_run {
            println!(
                "[dry run] {} — would archive {} subtree(s) to {}",
                file_path.display(),
                count,
                archive_path.display()
            );
            for (start, end) in &archived_ranges {
                let heading_line = lines.get(*start).unwrap_or(&"");
                println!("  line {}: {} ({} lines)", start + 1, heading_line.trim(), end - start);
            }
        } else {
            // Append to archive file
            let mut existing_archive = if archive_path.exists() {
                std::fs::read_to_string(&archive_path)?
            } else {
                String::new()
            };
            if !existing_archive.is_empty() && !existing_archive.ends_with('\n') {
                existing_archive.push('\n');
            }
            existing_archive.push_str(&archived_content);

            if let Some(parent) = archive_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&archive_path, &existing_archive)?;

            // Rewrite source file
            std::fs::write(file_path, &remaining)?;

            println!(
                "{} — archived {} subtree(s) to {}",
                file_path.display(),
                count,
                archive_path.display()
            );
        }
    }

    if total_archived == 0 {
        println!("No archived subtrees found.");
    } else if !dry_run {
        println!("\nArchived {total_archived} subtree(s) total.");
    }

    Ok(())
}

/// Find line ranges for headings that have #archive.
/// Returns Vec<(start_line_0indexed, end_line_0indexed)>.
fn find_archived_ranges(blocks: &[Block], lines: &[&str]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();

    for (i, block) in blocks.iter().enumerate() {
        if let Block::Heading(h) = block
            && is_archived_heading(h, blocks, i) {
                let start = (h.span.line as usize).saturating_sub(1);
                let end = find_subtree_end(h.level, blocks, i, lines.len());
                ranges.push((start, end));
            }
    }

    ranges
}

/// Check if a heading is marked as archived — either via inline #archive tag
/// or a block-level #archive tag in the immediately following blocks.
fn is_archived_heading(heading: &Heading, blocks: &[Block], heading_idx: usize) -> bool {
    // Check inline tags on the heading itself
    if heading.content.tags().iter().any(|t| matches!(t.kind, TagKind::Archive)) {
        return true;
    }

    // Check immediately following block tags (before next heading or non-tag block)
    for block in &blocks[heading_idx + 1..] {
        match block {
            Block::BlockTag(t) if matches!(t.kind, TagKind::Archive) => return true,
            Block::BlankLine(_) => continue,
            _ => break,
        }
    }

    false
}

/// Find the end line (exclusive) of a heading's subtree.
/// The subtree extends until the next heading at the same or higher level, or EOF.
fn find_subtree_end(level: u8, blocks: &[Block], heading_idx: usize, total_lines: usize) -> usize {
    for block in &blocks[heading_idx + 1..] {
        if let Block::Heading(h) = block
            && h.level <= level {
                // This heading is at same or higher level — subtree ends before it
                return (h.span.line as usize).saturating_sub(1);
            }
    }
    // No subsequent heading found — subtree extends to EOF
    total_lines
}

fn archive_file_path(source: &Path, suffix: &str) -> PathBuf {
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let ext = source.extension().unwrap_or_default().to_string_lossy();
    let archive_name = if ext.is_empty() {
        format!("{stem}{suffix}")
    } else {
        format!("{stem}{suffix}.{ext}")
    };
    source.with_file_name(archive_name)
}
