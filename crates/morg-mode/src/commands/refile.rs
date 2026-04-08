use std::path::PathBuf;

use morg_parser::ast::Block;

/// Parse a source/target spec: "file:line" or "file::heading-text"
fn parse_spec(spec: &str) -> Result<(PathBuf, LocationSpec), String> {
    // Try file:line (digits after last colon)
    if let Some((file_part, line_part)) = spec.rsplit_once(':') {
        // Disambiguate: file::heading vs file:123
        if !line_part.is_empty() && line_part.chars().all(|c| c.is_ascii_digit()) {
            let line: u32 = line_part.parse().map_err(|_| format!("invalid line number: {line_part}"))?;
            return Ok((PathBuf::from(file_part), LocationSpec::Line(line)));
        }
    }

    // Try file::heading
    if let Some((file_part, heading_part)) = spec.split_once("::") {
        if heading_part.is_empty() {
            return Ok((PathBuf::from(file_part), LocationSpec::End));
        }
        return Ok((PathBuf::from(file_part), LocationSpec::Heading(heading_part.to_string())));
    }

    // Just a file — target end of file
    Ok((PathBuf::from(spec), LocationSpec::End))
}

enum LocationSpec {
    Line(u32),
    Heading(String),
    End,
}

pub fn run(source_spec: &str, target_spec: &str, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (source_path, source_loc) = parse_spec(source_spec).map_err(|e| format!("invalid source: {e}"))?;
    let (target_path, target_loc) = parse_spec(target_spec).map_err(|e| format!("invalid target: {e}"))?;

    if !source_path.exists() {
        return Err(format!("source file not found: {}", source_path.display()).into());
    }

    let source_text = std::fs::read_to_string(&source_path)?;
    let source_lines: Vec<&str> = source_text.split('\n').collect();
    let source_result = morg_parser::parser::parse_document(&source_text);

    // Find the heading in the source
    let heading_line_0idx = find_heading(&source_result.document.children, &source_loc, &source_lines)?;
    let heading_level = detect_heading_level(source_lines[heading_line_0idx]);
    let subtree_end = find_subtree_end(heading_level, heading_line_0idx, &source_result.document.children, source_lines.len());

    let heading_text = source_lines[heading_line_0idx].trim();

    if dry_run {
        println!("[dry run] Would move:");
        println!("  From: {}:{} ({} lines)", source_path.display(), heading_line_0idx + 1, subtree_end - heading_line_0idx);
        println!("  Heading: {heading_text}");
        println!("  To: {target_spec}");
        return Ok(());
    }

    // Build the target content
    let same_file = source_path == target_path;

    let target_text = if same_file {
        source_text.clone()
    } else if target_path.exists() {
        std::fs::read_to_string(&target_path)?
    } else {
        String::new()
    };

    let target_lines: Vec<&str> = target_text.split('\n').collect();

    let insert_line = match &target_loc {
        LocationSpec::End => target_lines.len(),
        LocationSpec::Line(n) => (*n as usize).saturating_sub(1).min(target_lines.len()),
        LocationSpec::Heading(h) => {
            let target_result = morg_parser::parser::parse_document(&target_text);
            find_heading_insert_point(&target_result.document.children, h, target_lines.len())?
        }
    };

    if same_file {
        // Same-file refile: remove subtree, insert at new position
        let mut new_lines: Vec<String> = Vec::new();
        let mut i = 0;
        let mut inserted = false;

        // Adjust insert line if it's after the removed section
        let adjusted_insert = if insert_line > heading_line_0idx {
            insert_line - (subtree_end - heading_line_0idx)
        } else {
            insert_line
        };

        while i < source_lines.len() {
            if i == heading_line_0idx {
                i = subtree_end;
                continue;
            }

            if !inserted && new_lines.len() == adjusted_insert {
                new_lines.push(String::new());
                for sub_line in &source_lines[heading_line_0idx..subtree_end] {
                    new_lines.push(sub_line.to_string());
                }
                inserted = true;
            }

            new_lines.push(source_lines[i].to_string());
            i += 1;
        }

        if !inserted {
            new_lines.push(String::new());
            for sub_line in &source_lines[heading_line_0idx..subtree_end] {
                new_lines.push(sub_line.to_string());
            }
        }

        let content = new_lines.join("\n");
        std::fs::write(&source_path, &content)?;
    } else {
        // Cross-file refile: remove from source, insert into target
        // Remove from source
        let mut remaining: Vec<String> = Vec::new();
        let mut i = 0;
        while i < source_lines.len() {
            if i >= heading_line_0idx && i < subtree_end {
                i = subtree_end;
                continue;
            }
            remaining.push(source_lines[i].to_string());
            i += 1;
        }
        let source_content = remaining.join("\n");

        // Insert into target
        let mut target_new: Vec<String> = Vec::new();
        for (i, line) in target_lines.iter().enumerate() {
            if i == insert_line {
                target_new.push(String::new());
                for sub_line in &source_lines[heading_line_0idx..subtree_end] {
                    target_new.push(sub_line.to_string());
                }
            }
            target_new.push(line.to_string());
        }
        if insert_line >= target_lines.len() {
            target_new.push(String::new());
            for sub_line in &source_lines[heading_line_0idx..subtree_end] {
                target_new.push(sub_line.to_string());
            }
        }
        let target_content = target_new.join("\n");

        std::fs::write(&source_path, &source_content)?;
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target_path, &target_content)?;
    }

    println!(
        "Refiled \"{}\" ({} lines) from {} to {}",
        heading_text,
        subtree_end - heading_line_0idx,
        source_path.display(),
        target_spec,
    );

    Ok(())
}

fn find_heading(
    blocks: &[Block],
    loc: &LocationSpec,
    lines: &[&str],
) -> Result<usize, Box<dyn std::error::Error>> {
    match loc {
        LocationSpec::Line(n) => {
            let idx = (*n as usize).saturating_sub(1);
            if idx >= lines.len() {
                return Err(format!("line {n} is past end of file").into());
            }
            let trimmed = lines[idx].trim_start();
            if !trimmed.starts_with('#') || !trimmed[1..].starts_with(|c: char| c == ' ' || c == '#') {
                return Err(format!("line {n} is not a heading: {}", lines[idx].trim()).into());
            }
            Ok(idx)
        }
        LocationSpec::Heading(text) => {
            let text_lower = text.to_lowercase();
            for block in blocks {
                if let Block::Heading(h) = block {
                    if h.content.plain_text().to_lowercase().contains(&text_lower) {
                        return Ok((h.span.line as usize).saturating_sub(1));
                    }
                }
            }
            Err(format!("heading matching \"{text}\" not found").into())
        }
        LocationSpec::End => Err("source must specify a heading, not end of file".into()),
    }
}

fn find_heading_insert_point(
    blocks: &[Block],
    heading_text: &str,
    total_lines: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let text_lower = heading_text.to_lowercase();
    for block in blocks {
        if let Block::Heading(h) = block {
            if h.content.plain_text().to_lowercase().contains(&text_lower) {
                // Insert at the end of this heading's subtree
                let end = find_subtree_end(h.level, (h.span.line as usize) - 1, blocks, total_lines);
                return Ok(end);
            }
        }
    }
    Err(format!("target heading matching \"{heading_text}\" not found").into())
}

fn detect_heading_level(line: &str) -> u8 {
    let trimmed = line.trim_start();
    trimmed.bytes().take_while(|&b| b == b'#').count() as u8
}

fn find_subtree_end(level: u8, heading_line_0idx: usize, blocks: &[Block], total_lines: usize) -> usize {
    let mut found_self = false;
    for block in blocks {
        if let Block::Heading(h) = block {
            let line_0idx = (h.span.line as usize).saturating_sub(1);
            if line_0idx == heading_line_0idx {
                found_self = true;
                continue;
            }
            if found_self && h.level <= level {
                return line_0idx;
            }
        }
    }
    total_lines
}
