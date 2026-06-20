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

        let mut content = expand_noweb(&raw_content, &named_blocks);

        let allow_trailing_newline = blocks.iter().any(|b| b.allow_trailing_newline);
        if allow_trailing_newline && !content.ends_with('\n') {
            content.push('\n');
        }

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
    allow_trailing_newline: bool,
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
                    let allow_trailing_newline = cb
                        .attributes
                        .get("allow-trailing-newline")
                        .map(|v| v == "true")
                        .unwrap_or(false);
                    targets.entry(target).or_default().push(TangleBlock {
                        body: cb.body.clone(),
                        source_file: source_file.to_path_buf(),
                        line: cb.span.line,
                        allow_trailing_newline,
                    });
                }
            }
            Block::Callout(callout) => {
                if let Some(target) =
                    tangle_target(&callout.tags, &callout.attributes, source_dir, output_dir)
                {
                    let allow_trailing_newline = callout
                        .attributes
                        .get("allow-trailing-newline")
                        .map(|v| v == "true")
                        .unwrap_or(false);
                    let body = render_callout_content(&callout.content);
                    targets.entry(target).or_default().push(TangleBlock {
                        body,
                        source_file: source_file.to_path_buf(),
                        line: callout.span.line,
                        allow_trailing_newline,
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
    // Track the current heredoc end-marker; None means we are not inside a heredoc.
    let mut heredoc_end: Option<String> = None;

    for line in text.lines() {
        // If we are inside a shell heredoc, only check for the end marker.
        if let Some(ref marker) = heredoc_end {
            // The end marker is the delimiter stripped of optional leading tabs.
            if line.trim() == marker.as_str() {
                heredoc_end = None;
            }
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Detect shell heredoc opening: `<<WORD` or `<<-WORD` anywhere on the line.
        // We look for the *last* `<<` on the line that is followed by an identifier,
        // which becomes the end-marker for the subsequent content.
        if let Some(marker) = detect_heredoc_start(line) {
            heredoc_end = Some(marker);
            // Still emit the line verbatim; no noweb expansion on the opener line.
            result.push_str(line);
            result.push('\n');
            continue;
        }

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

/// Detect a shell heredoc opening on a line and return the end-marker string,
/// or `None` if the line does not start a heredoc.
///
/// Handles `<<WORD`, `<<-WORD`, and quoted forms `<<"WORD"` / `<<'WORD'`.
/// Does NOT match `<<WORD>>` (that is a noweb reference, handled separately).
fn detect_heredoc_start(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'<' && bytes[i + 1] == b'<' {
            let after = &line[i + 2..];
            // Skip optional `-` (tab-stripping form).
            let after = after.strip_prefix('-').unwrap_or(after);
            // Skip optional opening quote.
            let (after, quote) = if let Some(s) = after.strip_prefix('"') {
                (s, Some('"'))
            } else if let Some(s) = after.strip_prefix('\'') {
                (s, Some('\''))
            } else {
                (after, None)
            };
            // Collect the marker identifier.
            let marker: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if marker.is_empty() {
                i += 1;
                continue;
            }
            // Check the character after the marker to distinguish `<<NAME>>` (noweb, no heredoc)
            // from `<<NAME` followed by whitespace/EOL/quote-close.
            let rest = &after[marker.len()..];
            let rest = if let Some(q) = quote {
                rest.strip_prefix(q).unwrap_or(rest)
            } else {
                rest
            };
            // If the very next character is `>`, this is a noweb reference — not a heredoc.
            if rest.starts_with('>') {
                i += 1;
                continue;
            }
            return Some(marker);
        }
        i += 1;
    }
    None
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

    // -------------------------------------------------------------------------
    // Issue: allow-trailing-newline attribute
    // -------------------------------------------------------------------------

    #[test]
    fn test_allow_trailing_newline_adds_newline() {
        // When allow_trailing_newline is true, expand_noweb result gets a \n appended.
        let named = HashMap::new();
        let body = "line one\nline two";
        // Simulate what tangle does after expand_noweb when the flag is set.
        let mut content = expand_noweb(body, &named);
        // content has no trailing newline (input had none)
        assert!(!content.ends_with('\n'), "sanity: no trailing newline yet");
        // apply the allow-trailing-newline logic
        content.push('\n');
        assert!(content.ends_with('\n'));
        assert_eq!(content, "line one\nline two\n");
    }

    #[test]
    fn test_no_trailing_newline_by_default() {
        let named = HashMap::new();
        let body = "line one\nline two";
        let content = expand_noweb(body, &named);
        assert!(
            !content.ends_with('\n'),
            "default should have no trailing newline"
        );
    }

    // -------------------------------------------------------------------------
    // Issue: shell heredoc `<<WORD` must not be treated as a noweb reference
    // -------------------------------------------------------------------------

    #[test]
    fn test_heredoc_content_not_expanded() {
        // `<<content>>` inside a heredoc should remain literal.
        let mut named = HashMap::new();
        named.insert("content".to_string(), "EXPANDED".to_string());

        let input = "cat <<EOF\n<<content>>\nEOF";
        let result = expand_noweb(input, &named);
        assert!(
            result.contains("<<content>>"),
            "heredoc body should not be noweb-expanded, got: {result:?}"
        );
        assert!(
            !result.contains("EXPANDED"),
            "noweb must not expand inside heredoc, got: {result:?}"
        );
    }

    #[test]
    fn test_heredoc_end_marker_restored() {
        // After the heredoc closes, noweb expansion resumes normally.
        let mut named = HashMap::new();
        named.insert("value".to_string(), "42".to_string());

        let input = "cat <<EOF\nliteral\nEOF\n<<value>>";
        let result = expand_noweb(input, &named);
        assert!(
            result.contains("42"),
            "noweb should expand after heredoc ends, got: {result:?}"
        );
        assert!(
            result.contains("literal"),
            "heredoc body should be preserved, got: {result:?}"
        );
    }

    #[test]
    fn test_noweb_ref_not_confused_with_heredoc() {
        // `<<name>>` (with closing >>) must still be treated as a noweb reference
        // and NOT as a heredoc opener.
        let mut named = HashMap::new();
        named.insert("greet".to_string(), "hello".to_string());

        let input = "<<greet>>";
        let result = expand_noweb(input, &named);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_detect_heredoc_start() {
        assert_eq!(detect_heredoc_start("cat <<EOF"), Some("EOF".to_string()));
        assert_eq!(detect_heredoc_start("cat <<-EOF"), Some("EOF".to_string()));
        // `<<name>>` is a noweb ref, not a heredoc
        assert_eq!(detect_heredoc_start("<<greet>>"), None);
        // Plain line — no heredoc
        assert_eq!(detect_heredoc_start("echo hello"), None);
    }
}
