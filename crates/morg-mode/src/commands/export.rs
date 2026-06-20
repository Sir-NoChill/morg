use std::path::{Path, PathBuf};

use morg_parser::ast::*;
use morg_parser::tags::TagKind;

use crate::collect;

pub fn run(
    paths: &[PathBuf],
    output: Option<&Path>,
    standalone: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);
    let mut html = String::new();

    if standalone {
        let title = parsed
            .first()
            .and_then(|pf| pf.document.frontmatter.as_ref())
            .and_then(|fm| fm.data.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("morg document");

        html.push_str(&format!(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\n{CSS}\n</style>\n</head>\n<body>\n<article>\n"
        ));
    }

    // Collect footnote definitions for rendering at the end
    let mut footnotes: Vec<(String, String)> = Vec::new();

    for pf in &parsed {
        render_blocks(&pf.document.children, &mut html, &mut footnotes);
    }

    // Render footnotes section
    if !footnotes.is_empty() {
        html.push_str("<section class=\"footnotes\">\n<hr>\n<ol>\n");
        for (label, content) in &footnotes {
            html.push_str(&format!(
                "<li id=\"fn-{label}\"><p>{content} <a href=\"#fnref-{label}\">↩</a></p></li>\n"
            ));
        }
        html.push_str("</ol>\n</section>\n");
    }

    if standalone {
        html.push_str("</article>\n</body>\n</html>\n");
    }

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &html)?;
            eprintln!("Exported to {}", path.display());
        }
        None => {
            print!("{html}");
        }
    }

    Ok(())
}

fn render_blocks(blocks: &[Block], out: &mut String, footnotes: &mut Vec<(String, String)>) {
    for block in blocks {
        render_block(block, out, footnotes);
    }
}

fn render_block(block: &Block, out: &mut String, footnotes: &mut Vec<(String, String)>) {
    match block {
        Block::Heading(h) => {
            let level = h.level;
            let id = slug(&h.content.plain_text());
            out.push_str(&format!("<h{level} id=\"{id}\">"));
            render_inline(&h.content, out);
            out.push_str(&format!("</h{level}>\n"));
        }
        Block::Paragraph(p) => {
            out.push_str("<p>");
            render_inline(&p.content, out);
            out.push_str("</p>\n");
        }
        Block::CodeBlock(cb) => {
            let lang_attr = cb
                .lang
                .as_deref()
                .map(|l| format!(" class=\"language-{l}\""))
                .unwrap_or_default();
            out.push_str(&format!(
                "<pre><code{lang_attr}>{}</code></pre>\n",
                escape_html(&cb.body)
            ));
        }
        Block::BlankLine(_) => {}
        Block::BlockTag(tag) => {
            render_block_tag(tag, out);
        }
        Block::Callout(c) => {
            out.push_str(&format!(
                "<blockquote class=\"callout callout-{}\">\n",
                c.kind
            ));
            out.push_str(&format!(
                "<p class=\"callout-title\">{}</p>\n",
                capitalize(&c.kind)
            ));
            render_blocks(&c.content, out, footnotes);
            out.push_str("</blockquote>\n");
        }
        Block::Table(t) => {
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (i, header) in t.headers.iter().enumerate() {
                let align = t.alignments.get(i).copied().unwrap_or(Alignment::None);
                let style = alignment_style(align);
                out.push_str(&format!("<th{style}>"));
                render_inline(header, out);
                out.push_str("</th>\n");
            }
            out.push_str("</tr>\n</thead>\n<tbody>\n");
            for row in &t.rows {
                out.push_str("<tr>\n");
                for (i, cell) in row.iter().enumerate() {
                    let align = t.alignments.get(i).copied().unwrap_or(Alignment::None);
                    let style = alignment_style(align);
                    out.push_str(&format!("<td{style}>"));
                    render_inline(cell, out);
                    out.push_str("</td>\n");
                }
                out.push_str("</tr>\n");
            }
            out.push_str("</tbody>\n</table>\n");
        }
        Block::HtmlBlock(h) => {
            out.push_str(&h.raw);
            out.push('\n');
        }
        Block::List(list) => {
            let tag = match list.kind {
                ListKind::Unordered => "ul",
                ListKind::Ordered => "ol",
            };
            out.push_str(&format!("<{tag}>\n"));
            for item in &list.items {
                render_list_item(item, out, footnotes);
            }
            out.push_str(&format!("</{tag}>\n"));
        }
        Block::HorizontalRule(_) => {
            out.push_str("<hr>\n");
        }
        Block::Comment(_) => {
            // Comments are not rendered
        }
        Block::FootnoteDefinition(fd) => {
            let mut content = String::new();
            render_inline(&fd.content, &mut content);
            footnotes.push((fd.label.clone(), content));
        }
        Block::LinkDefinition(_) => {
            // Link definitions are consumed by the resolution pass;
            // they produce no visible output.
        }
    }
}

fn render_list_item(item: &ListItem, out: &mut String, footnotes: &mut Vec<(String, String)>) {
    out.push_str("<li>");
    if let Some(checkbox) = &item.checkbox {
        let checked = match checkbox {
            Checkbox::Checked => " checked disabled",
            Checkbox::Unchecked => " disabled",
        };
        out.push_str(&format!("<input type=\"checkbox\"{checked}> "));
    }
    render_inline(&item.content, out);
    if let Some(ref desc) = item.description {
        out.push_str("<dl><dd>");
        render_inline(desc, out);
        out.push_str("</dd></dl>");
    }
    for child in &item.children {
        render_block(child, out, footnotes);
    }
    out.push_str("</li>\n");
}

fn render_inline(content: &InlineContent, out: &mut String) {
    for seg in &content.segments {
        render_inline_segment(seg, out);
    }
}

fn render_inline_segment(seg: &InlineSegment, out: &mut String) {
    match seg {
        InlineSegment::Text(t) => out.push_str(&escape_html(t)),
        InlineSegment::Tag(tag) => render_inline_tag(tag, out),
        InlineSegment::Bold(inner) => {
            out.push_str("<strong>");
            render_inline(inner, out);
            out.push_str("</strong>");
        }
        InlineSegment::Italic(inner) => {
            out.push_str("<em>");
            render_inline(inner, out);
            out.push_str("</em>");
        }
        InlineSegment::Strikethrough(inner) => {
            out.push_str("<del>");
            render_inline(inner, out);
            out.push_str("</del>");
        }
        InlineSegment::Code(c) => {
            out.push_str(&format!("<code>{}</code>", escape_html(c)));
        }
        InlineSegment::Link(link) => {
            let title_attr = link
                .title
                .as_deref()
                .map(|t| format!(" title=\"{}\"", escape_html(t)))
                .unwrap_or_default();
            out.push_str(&format!(
                "<a href=\"{}\"{}>{}</a>",
                escape_html(&link.url),
                title_attr,
                escape_html(&link.text),
            ));
        }
        InlineSegment::Image(img) => {
            let title_attr = img
                .title
                .as_deref()
                .map(|t| format!(" title=\"{}\"", escape_html(t)))
                .unwrap_or_default();
            out.push_str(&format!(
                "<img src=\"{}\" alt=\"{}\"{}/>",
                escape_html(&img.url),
                escape_html(&img.alt),
                title_attr,
            ));
        }
        InlineSegment::FootnoteRef(label) => {
            out.push_str(&format!(
                "<sup><a id=\"fnref-{label}\" href=\"#fn-{label}\">{label}</a></sup>"
            ));
        }
        InlineSegment::HardBreak => {
            out.push_str("<br />\n");
        }
        InlineSegment::LinkRef { text, .. } => {
            // Unresolved reference — render as plain text
            out.push_str(&escape_html(text));
        }
    }
}

fn render_inline_tag(tag: &morg_parser::tags::Tag, out: &mut String) {
    match &tag.kind {
        TagKind::Todo { text } => {
            out.push_str("<span class=\"tag tag-todo\">TODO</span>");
            if let Some(t) = text {
                out.push_str(&format!(" {}", escape_html(t)));
            }
        }
        TagKind::Done { text } => {
            out.push_str("<span class=\"tag tag-done\">DONE</span>");
            if let Some(t) = text {
                out.push_str(&format!(" <del>{}</del>", escape_html(t)));
            }
        }
        TagKind::Deadline { date, .. } => {
            out.push_str(&format!(
                "<span class=\"tag tag-deadline\">DEADLINE: {date}</span>"
            ));
        }
        TagKind::Scheduled { date, .. } => {
            out.push_str(&format!(
                "<span class=\"tag tag-scheduled\">SCHEDULED: {date}</span>"
            ));
        }
        TagKind::Priority { level } => {
            out.push_str(&format!(
                "<span class=\"tag tag-priority tag-priority-{}\">#{level}</span>",
                level.to_string().to_lowercase()
            ));
        }
        TagKind::Archive => {
            out.push_str("<span class=\"tag tag-archive\">ARCHIVE</span>");
        }
        // Other tags: render as subtle spans
        _ => {}
    }
}

fn render_block_tag(tag: &morg_parser::tags::Tag, out: &mut String) {
    match &tag.kind {
        TagKind::Todo { text } => {
            out.push_str("<p><span class=\"tag tag-todo\">TODO</span>");
            if let Some(t) = text {
                out.push_str(&format!(" {}", escape_html(t)));
            }
            out.push_str("</p>\n");
        }
        TagKind::Done { text } => {
            out.push_str("<p><span class=\"tag tag-done\">DONE</span>");
            if let Some(t) = text {
                out.push_str(&format!(" <del>{}</del>", escape_html(t)));
            }
            out.push_str("</p>\n");
        }
        TagKind::Deadline { date, .. } => {
            out.push_str(&format!(
                "<p class=\"planning\"><strong>DEADLINE:</strong> {date}</p>\n"
            ));
        }
        TagKind::Scheduled { date, .. } => {
            out.push_str(&format!(
                "<p class=\"planning\"><strong>SCHEDULED:</strong> {date}</p>\n"
            ));
        }
        _ => {}
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn alignment_style(align: Alignment) -> &'static str {
    match align {
        Alignment::Left => " style=\"text-align:left\"",
        Alignment::Center => " style=\"text-align:center\"",
        Alignment::Right => " style=\"text-align:right\"",
        Alignment::None => "",
    }
}

#[cfg(test)]
pub fn render_document(doc: &morg_parser::Document) -> String {
    let mut html = String::new();
    let mut footnotes: Vec<(String, String)> = Vec::new();
    render_blocks(&doc.children, &mut html, &mut footnotes);
    if !footnotes.is_empty() {
        html.push_str("<section class=\"footnotes\">\n<hr>\n<ol>\n");
        for (label, content) in &footnotes {
            html.push_str(&format!(
                "<li id=\"fn-{label}\"><p>{content} <a href=\"#fnref-{label}\">↩</a></p></li>\n"
            ));
        }
        html.push_str("</ol>\n</section>\n");
    }
    html
}

const CSS: &str = r#"
body { max-width: 48em; margin: 2em auto; padding: 0 1em; font-family: system-ui, sans-serif; line-height: 1.6; color: #222; }
h1, h2, h3, h4, h5, h6 { margin-top: 1.5em; }
pre { background: #f5f5f5; padding: 1em; overflow-x: auto; border-radius: 4px; }
code { font-size: 0.9em; }
p code { background: #f0f0f0; padding: 0.15em 0.3em; border-radius: 3px; }
blockquote { border-left: 4px solid #ddd; margin: 1em 0; padding: 0.5em 1em; }
blockquote.callout { border-radius: 4px; }
blockquote.callout-note { border-left-color: #4a9eff; background: #f0f7ff; }
blockquote.callout-warning { border-left-color: #f5a623; background: #fff8f0; }
blockquote.callout-tip { border-left-color: #2ecc71; background: #f0fff5; }
blockquote.callout-danger { border-left-color: #e74c3c; background: #fff0f0; }
.callout-title { font-weight: bold; margin: 0 0 0.5em; }
table { border-collapse: collapse; width: 100%; margin: 1em 0; }
th, td { border: 1px solid #ddd; padding: 0.5em; }
th { background: #f5f5f5; }
.tag { display: inline-block; padding: 0.1em 0.4em; border-radius: 3px; font-size: 0.85em; font-weight: bold; }
.tag-todo { background: #fff3cd; color: #856404; }
.tag-done { background: #d4edda; color: #155724; }
.tag-deadline { background: #f8d7da; color: #721c24; }
.tag-scheduled { background: #cce5ff; color: #004085; }
.tag-priority-a { background: #f8d7da; color: #721c24; }
.tag-priority-b { background: #fff3cd; color: #856404; }
.tag-priority-c { background: #d4edda; color: #155724; }
.tag-archive { background: #e2e3e5; color: #383d41; }
.planning { color: #666; font-size: 0.9em; }
.footnotes { font-size: 0.9em; color: #555; }
li { margin: 0.25em 0; }
input[type="checkbox"] { margin-right: 0.5em; }
hr { border: none; border-top: 1px solid #ddd; margin: 2em 0; }
del { color: #888; }
"#;

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use morg_parser::parse_document;

    /// Parse markdown source and render to HTML fragment.
    fn export(src: &str) -> String {
        let result = parse_document(src);
        assert!(
            result.errors.is_empty(),
            "parse errors: {:?}",
            result.errors
        );
        render_document(&result.document)
    }

    // --- Link reference definitions: resolved links in export output ---

    #[test]
    fn export_link_ref_shortcut() {
        let html = export("[foo]: /url\n\nSee [foo] for details.\n");
        assert!(
            html.contains(r#"<a href="/url">foo</a>"#),
            "shortcut ref should render as <a>: {html}"
        );
    }

    #[test]
    fn export_link_ref_full() {
        let html = export("[foo]: /url\n\n[click here][foo]\n");
        assert!(
            html.contains(r#"<a href="/url">click here</a>"#),
            "full ref should render with display text: {html}"
        );
    }

    #[test]
    fn export_link_ref_collapsed() {
        let html = export("[foo]: /url\n\n[foo][]\n");
        assert!(
            html.contains(r#"<a href="/url">foo</a>"#),
            "collapsed ref should render as <a>: {html}"
        );
    }

    #[test]
    fn export_link_ref_with_title() {
        let html = export("[foo]: /url \"My Title\"\n\n[foo]\n");
        assert!(
            html.contains(r#"title="My Title""#),
            "title should appear in the anchor: {html}"
        );
        assert!(
            html.contains(r#"href="/url""#),
            "url should appear in the anchor: {html}"
        );
    }

    #[test]
    fn export_link_ref_unresolved_is_plain_text() {
        let html = export("See [undefined] here.\n");
        // Unresolved ref should NOT produce an <a> tag
        assert!(
            !html.contains("<a "),
            "unresolved ref should not produce a link: {html}"
        );
        assert!(
            html.contains("undefined"),
            "unresolved ref text should appear: {html}"
        );
    }

    #[test]
    fn export_link_ref_case_insensitive() {
        let html = export("[Foo]: /url\n\n[foo] and [FOO]\n");
        let link_count = html.matches(r#"<a href="/url">"#).count();
        assert_eq!(link_count, 2, "both case variants should resolve: {html}");
    }

    #[test]
    fn export_link_def_produces_no_output() {
        let html = export("[foo]: /url\n\nParagraph.\n");
        // The definition itself should not render as visible text
        assert!(
            !html.contains("[foo]:"),
            "link definition should not appear in output: {html}"
        );
        assert!(
            !html.contains("/url</"),
            "link definition URL should not appear as raw text: {html}"
        );
    }

    #[test]
    fn export_multiple_link_defs() {
        let html = export("[a]: /a\n[b]: /b\n\nSee [a] and [b].\n");
        assert!(html.contains(r#"<a href="/a">a</a>"#), "ref [a]: {html}");
        assert!(html.contains(r#"<a href="/b">b</a>"#), "ref [b]: {html}");
    }

    #[test]
    fn export_link_ref_in_heading() {
        let html = export("[foo]: /url\n\n# See [foo]\n");
        assert!(
            html.contains(r#"<a href="/url">foo</a>"#),
            "link ref in heading should resolve: {html}"
        );
        // Should be inside an <h1>
        assert!(html.contains("<h1"), "heading should render: {html}");
    }

    #[test]
    fn export_link_ref_in_list() {
        let html = export("[foo]: /url\n\n- Item with [foo]\n");
        assert!(
            html.contains(r#"<a href="/url">foo</a>"#),
            "link ref in list item should resolve: {html}"
        );
    }

    #[test]
    fn export_link_ref_in_bold() {
        let html = export("[foo]: /url\n\n**bold [foo] text**\n");
        assert!(
            html.contains(r#"<a href="/url">foo</a>"#),
            "link ref inside bold should resolve: {html}"
        );
    }

    // --- Verify other inline elements still export correctly ---

    #[test]
    fn export_inline_link() {
        let html = export("[click](https://example.com)\n");
        assert!(html.contains(r#"<a href="https://example.com">click</a>"#));
    }

    #[test]
    fn export_image() {
        let html = export("![alt](img.png)\n");
        assert!(html.contains(r#"<img src="img.png" alt="alt"/>"#));
    }

    #[test]
    fn export_autolink() {
        let html = export("<https://example.com>\n");
        assert!(html.contains(r#"<a href="https://example.com">https://example.com</a>"#));
    }

    #[test]
    fn export_hard_break() {
        let html = export("line one  \nline two\n");
        assert!(html.contains("<br />"), "hard break: {html}");
    }

    #[test]
    fn export_image_with_title() {
        let html = export("![photo](pic.jpg \"My Photo\")\n");
        assert!(html.contains(r#"title="My Photo""#));
        assert!(html.contains(r#"src="pic.jpg""#));
    }

    #[test]
    fn export_code_span() {
        let html = export("Use `println!` here\n");
        assert!(html.contains("<code>println!</code>"));
    }

    #[test]
    fn export_double_backtick_code() {
        let html = export("Use ``code with `tick` inside`` here\n");
        assert!(html.contains("<code>code with `tick` inside</code>"));
    }

    // --- Integration: full document with link refs ---

    #[test]
    fn export_full_document_with_link_refs() {
        let src = r#"# Documentation

[rust]: https://www.rust-lang.org "The Rust Language"
[morg]: /morg

This project is built with [rust]. See the [morg] docs.

## Links

- [Official site][rust]
- [Our docs][morg]

Check [rust][] for the latest release.
"#;
        let html = export(src);
        // All three ref forms should resolve to the Rust URL
        let rust_count = html.matches(r#"href="https://www.rust-lang.org""#).count();
        assert!(
            rust_count >= 3,
            "all [rust] refs should resolve ({rust_count} found): {html}"
        );
        // Title should be present
        assert!(
            html.contains(r#"title="The Rust Language""#),
            "title should propagate: {html}"
        );
        // morg refs should resolve
        assert!(
            html.matches(r#"href="/morg""#).count() >= 2,
            "all [morg] refs should resolve: {html}"
        );
        // Link definitions should not appear as visible text
        assert!(
            !html.contains("[rust]:"),
            "defs should be invisible: {html}"
        );
        assert!(
            !html.contains("[morg]:"),
            "defs should be invisible: {html}"
        );
    }

    // --- Setext headings ---

    #[test]
    fn export_setext_h1() {
        let html = export("My Title\n========\n");
        assert!(html.contains("<h1"), "setext = should produce h1: {html}");
        assert!(html.contains("My Title"), "heading text: {html}");
    }

    #[test]
    fn export_setext_h2() {
        let html = export("Subtitle\n--------\n");
        assert!(html.contains("<h2"), "setext - should produce h2: {html}");
        assert!(html.contains("Subtitle"), "heading text: {html}");
    }

    #[test]
    fn export_setext_dashes_after_paragraph_is_h2() {
        let html = export("Some text\n---\n");
        assert!(
            html.contains("<h2"),
            "--- after text should be setext h2, not <hr>: {html}"
        );
        assert!(!html.contains("<hr"), "should not produce <hr>: {html}");
    }

    // --- Indented code blocks ---

    #[test]
    fn export_indented_code_block() {
        let html = export("    code here\n");
        assert!(
            html.contains("<pre><code>"),
            "should produce code block: {html}"
        );
        assert!(html.contains("code here"), "code content: {html}");
    }

    #[test]
    fn export_indented_code_block_strips_indent() {
        let html = export("    hello world\n");
        // The 4-space indent should be stripped from the output
        assert!(
            html.contains("hello world"),
            "indent should be stripped: {html}"
        );
        // Should NOT start with 4 spaces inside <code>
        assert!(
            !html.contains("<code>    "),
            "prefix should not appear in output: {html}"
        );
    }
}
