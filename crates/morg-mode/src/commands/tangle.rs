use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use morg_parser::ast::Block;
use morg_parser::tags::{Tag, TagKind};

use crate::collect;

pub fn run(paths: &[PathBuf], output_dir: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);

    // Pass 1: collect named blocks for noweb resolution
    let mut named_blocks: HashMap<String, String> = HashMap::new();
    for pf in &parsed {
        collect_named_blocks(&pf.document.children, &mut named_blocks);
    }

    // Pass 2: collect tangleable blocks by target file
    let mut targets: HashMap<PathBuf, Vec<TangleBlock>> = HashMap::new();
    for pf in &parsed {
        let source_dir = pf.path.parent().unwrap_or(Path::new("."));
        collect_tangle_blocks(
            &pf.document.children,
            source_dir,
            output_dir,
            &pf.path,
            &mut targets,
        );
    }

    if targets.is_empty() {
        println!("No tangleable blocks found.");
        return Ok(());
    }

    // Pass 3: write files, expanding noweb references
    for (target, blocks) in &targets {
        let raw_content: String = blocks
            .iter()
            .map(|b| b.body.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let content = expand_noweb(&raw_content, &named_blocks);

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, &content)?;

        println!(
            "{} <- {} block(s) from {}",
            target.display(),
            blocks.len(),
            blocks
                .iter()
                .map(|b| format!("{}:{}", b.source_file.display(), b.line))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!("\nTangled {} file(s).", targets.len());

    Ok(())
}

struct TangleBlock {
    body: String,
    source_file: PathBuf,
    line: u32,
}

/// Collect all code blocks with a `name=` attribute into a map.
fn collect_named_blocks(blocks: &[Block], named: &mut HashMap<String, String>) {
    for block in blocks {
        match block {
            Block::CodeBlock(cb) => {
                if let Some(name) = cb.attributes.get("name") {
                    // If multiple blocks share a name, concatenate them
                    let entry = named.entry(name.clone()).or_default();
                    if !entry.is_empty() {
                        entry.push('\n');
                    }
                    entry.push_str(&cb.body);
                }
            }
            Block::Callout(callout) => {
                collect_named_blocks(&callout.content, named);
            }
            _ => {}
        }
    }
}

fn collect_tangle_blocks(
    blocks: &[Block],
    source_dir: &Path,
    output_dir: Option<&Path>,
    source_file: &Path,
    targets: &mut HashMap<PathBuf, Vec<TangleBlock>>,
) {
    for block in blocks {
        match block {
            Block::CodeBlock(cb) => {
                if let Some(target) =
                    tangle_target(&cb.tags, &cb.attributes, source_dir, output_dir)
                {
                    targets.entry(target).or_default().push(TangleBlock {
                        body: cb.body.clone(),
                        source_file: source_file.to_path_buf(),
                        line: cb.span.line,
                    });
                }
            }
            Block::Callout(callout) => {
                if let Some(target) =
                    tangle_target(&callout.tags, &callout.attributes, source_dir, output_dir)
                {
                    let body = render_callout_content(&callout.content);
                    targets.entry(target).or_default().push(TangleBlock {
                        body,
                        source_file: source_file.to_path_buf(),
                        line: callout.span.line,
                    });
                }
                collect_tangle_blocks(
                    &callout.content,
                    source_dir,
                    output_dir,
                    source_file,
                    targets,
                );
            }
            _ => {}
        }
    }
}

fn tangle_target(
    tags: &[Tag],
    attributes: &HashMap<String, String>,
    source_dir: &Path,
    output_dir: Option<&Path>,
) -> Option<PathBuf> {
    let has_tangle = tags.iter().any(|t| matches!(t.kind, TagKind::Tangle));
    if !has_tangle {
        return None;
    }

    let file_attr = attributes.get("file")?;
    let base = output_dir.unwrap_or(source_dir);
    Some(base.join(file_attr))
}

/// Expand all `<<name>>` noweb references in the text, preserving indentation.
fn expand_noweb(text: &str, named: &HashMap<String, String>) -> String {
    let mut visited = HashSet::new();
    expand_noweb_recursive(text, named, &mut visited)
}

fn expand_noweb_recursive(
    text: &str,
    named: &HashMap<String, String>,
    visited: &mut HashSet<String>,
) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        // Check if the entire line (minus indent) is a noweb ref — use indent-preserving expansion
        if let Some((indent, ref_name)) = parse_noweb_ref(line) {
            if visited.contains(ref_name) {
                eprintln!("warning: circular noweb reference <<{ref_name}>>, skipping");
                result.push_str(line);
                result.push('\n');
                continue;
            }

            match named.get(ref_name) {
                Some(body) => {
                    visited.insert(ref_name.to_string());
                    let expanded = expand_noweb_recursive(body, named, visited);
                    visited.remove(ref_name);

                    for (i, exp_line) in expanded.lines().enumerate() {
                        if i > 0 {
                            result.push('\n');
                        }
                        if !exp_line.is_empty() {
                            result.push_str(indent);
                            result.push_str(exp_line);
                        }
                    }
                    result.push('\n');
                }
                None => {
                    eprintln!("warning: unresolved noweb reference <<{ref_name}>>");
                    result.push_str(line);
                    result.push('\n');
                }
            }
        } else {
            // Handle inline <<ref>> within the line
            let expanded_line = expand_inline_refs(line, named, visited);
            result.push_str(&expanded_line);
            result.push('\n');
        }
    }

    // Remove trailing newline to match input convention
    if result.ends_with('\n') && !text.ends_with('\n') {
        result.pop();
    }

    result
}

/// Expand `<<name>>` references that appear inline within a line.
fn expand_inline_refs(
    line: &str,
    named: &HashMap<String, String>,
    visited: &mut HashSet<String>,
) -> String {
    let mut result = String::new();
    let mut pos = 0;
    let bytes = line.as_bytes();

    while pos < bytes.len() {
        if pos + 2 < bytes.len()
            && bytes[pos] == b'<'
            && bytes[pos + 1] == b'<'
            && let Some(end) = line[pos + 2..].find(">>")
        {
            let ref_name = &line[pos + 2..pos + 2 + end];
            if !ref_name.is_empty() && !ref_name.contains('<') && !ref_name.contains('>') {
                if visited.contains(ref_name) {
                    eprintln!("warning: circular noweb reference <<{ref_name}>>, skipping");
                    result.push_str(&line[pos..pos + 2 + end + 2]);
                } else if let Some(body) = named.get(ref_name) {
                    visited.insert(ref_name.to_string());
                    let expanded = expand_noweb_recursive(body, named, visited);
                    visited.remove(ref_name);
                    result.push_str(&expanded);
                } else {
                    eprintln!("warning: unresolved noweb reference <<{ref_name}>>");
                    result.push_str(&line[pos..pos + 2 + end + 2]);
                }
                pos = pos + 2 + end + 2;
                continue;
            }
        }
        result.push(bytes[pos] as char);
        pos += 1;
    }

    result
}

/// If this line is a noweb reference like `    <<name>>`, return (indent, name).
fn parse_noweb_ref(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    let rest = trimmed.strip_prefix("<<")?;
    let name = rest.strip_suffix(">>")?;

    // Name must be non-empty and not contain special chars
    if name.is_empty() || name.contains('<') || name.contains('>') {
        return None;
    }

    Some((indent, name))
}

fn render_callout_content(blocks: &[Block]) -> String {
    let mut lines = Vec::new();
    for block in blocks {
        match block {
            Block::Paragraph(p) => {
                lines.push(p.content.plain_text());
            }
            Block::CodeBlock(cb) => {
                lines.push(cb.body.clone());
            }
            Block::Heading(h) => {
                lines.push(h.content.plain_text());
            }
            Block::HtmlBlock(h) => {
                lines.push(h.raw.clone());
            }
            Block::BlankLine(_) => {
                lines.push(String::new());
            }
            _ => {}
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noweb_simple_expansion() {
        let mut named = HashMap::new();
        named.insert("imports".to_string(), "use std::io;".to_string());

        let input = "<<imports>>\n\nfn main() {}";
        let result = expand_noweb(input, &named);
        assert_eq!(result, "use std::io;\n\nfn main() {}");
    }

    #[test]
    fn test_noweb_indentation() {
        let mut named = HashMap::new();
        named.insert(
            "body".to_string(),
            "println!(\"hello\");\nprintln!(\"world\");".to_string(),
        );

        let input = "fn main() {\n    <<body>>\n}";
        let result = expand_noweb(input, &named);
        assert_eq!(
            result,
            "fn main() {\n    println!(\"hello\");\n    println!(\"world\");\n}"
        );
    }

    #[test]
    fn test_noweb_recursive() {
        let mut named = HashMap::new();
        named.insert("inner".to_string(), "x + 1".to_string());
        named.insert("outer".to_string(), "let y = <<inner>>;".to_string());

        let input = "<<outer>>";
        let result = expand_noweb(input, &named);
        assert_eq!(result, "let y = x + 1;");
    }

    #[test]
    fn test_noweb_circular() {
        let mut named = HashMap::new();
        named.insert("a".to_string(), "<<b>>".to_string());
        named.insert("b".to_string(), "<<a>>".to_string());

        let input = "<<a>>";
        let result = expand_noweb(input, &named);
        // Should not infinite loop — the circular ref is left unexpanded
        assert!(result.contains("<<a>>") || result.contains("<<b>>"));
    }

    #[test]
    fn test_noweb_unresolved() {
        let named = HashMap::new();
        let input = "<<missing>>";
        let result = expand_noweb(input, &named);
        assert_eq!(result, "<<missing>>");
    }

    #[test]
    fn test_parse_noweb_ref() {
        assert_eq!(parse_noweb_ref("<<imports>>"), Some(("", "imports")));
        assert_eq!(parse_noweb_ref("    <<body>>"), Some(("    ", "body")));
        assert_eq!(parse_noweb_ref("not a ref"), None);
        assert_eq!(parse_noweb_ref("<<>>"), None);
    }
}
