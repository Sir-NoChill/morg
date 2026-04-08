//! Token-consuming parser for morg-mode.
//!
//! Consumes the `Lexer` token stream to produce the same AST as `parser.rs`.
//! Block-level tokens drive the structure; `lexer::tokenize_inline` is called
//! on demand for text content.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::{ParseError, ParseErrorKind};
use crate::lexer::{self, Lexer};
use crate::span::Span;
use crate::tags::{self, Tag};
use crate::tokens::Token;

pub struct ParseResult {
    pub document: Document,
    pub errors: Vec<ParseError>,
}

pub fn parse_document(source: &str) -> ParseResult {
    let mut lex = Lexer::new(source);
    let mut errors = Vec::new();

    let frontmatter = parse_frontmatter(&mut lex, &mut errors);
    let mut children = Vec::new();

    while !lex.is_eof() {
        match parse_block(&mut lex, &mut errors) {
            Some(block) => children.push(block),
            None => {
                lex.skip_to_next_line();
            }
        }
    }

    ParseResult {
        document: Document { frontmatter, children },
        errors,
    }
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

fn parse_frontmatter(lex: &mut Lexer<'_>, errors: &mut Vec<ParseError>) -> Option<Frontmatter> {
    let first = lex.peek();
    if first.span.line != 1 || !matches!(first.kind, Token::FrontmatterDelim) {
        return None;
    }

    let open_span = lex.advance().span;
    skip_newline(lex);

    let mut yaml_lines: Vec<String> = Vec::new();

    loop {
        if lex.is_eof() {
            errors.push(ParseError {
                kind: ParseErrorKind::UnclosedFrontmatter,
                span: open_span,
                message: "frontmatter opened but never closed with ---".to_string(),
            });
            return None;
        }

        let tok = lex.peek();
        if matches!(tok.kind, Token::FrontmatterDelim) {
            let close_span = lex.advance().span;
            skip_newline(lex);

            let raw = yaml_lines.join("\n");
            let span = open_span.merge(close_span);

            match serde_yaml::from_str(&raw) {
                Ok(data) => return Some(Frontmatter { raw, data, span }),
                Err(e) => {
                    errors.push(ParseError {
                        kind: ParseErrorKind::InvalidYaml,
                        span,
                        message: format!("invalid YAML in frontmatter: {e}"),
                    });
                    return None;
                }
            }
        }

        // Collect raw line content
        let raw = extract_raw_line(lex);
        yaml_lines.push(raw);
    }
}

// ---------------------------------------------------------------------------
// Block dispatcher
// ---------------------------------------------------------------------------

fn parse_block(lex: &mut Lexer<'_>, errors: &mut Vec<ParseError>) -> Option<Block> {
    let tok = lex.peek();

    match &tok.kind {
        Token::BlankLine => {
            let span = lex.advance().span;
            skip_newline(lex);
            Some(Block::BlankLine(span))
        }
        Token::Heading { level } => {
            let level = *level;
            parse_heading(lex, level, errors)
        }
        Token::FencedCodeOpen { .. } | Token::FencedCodeClose { .. } => {
            Some(parse_code_block(lex, errors))
        }
        Token::CalloutStart { .. } => Some(parse_callout(lex, errors)),
        Token::ListMarker { .. } => Some(parse_list(lex)),
        Token::TableRow => Some(parse_table(lex)),
        Token::HtmlOpen { .. } => Some(parse_html_block(lex, errors)),
        Token::HorizontalRule => {
            let span = lex.advance().span;
            skip_newline(lex);
            Some(Block::HorizontalRule(span))
        }
        Token::LineComment => Some(parse_line_comment(lex)),
        Token::BlockCommentOpen => Some(parse_block_comment(lex)),
        Token::FootnoteDefStart { .. } => Some(parse_footnote_def(lex)),
        Token::FrontmatterDelim => {
            // --- not at line 1 — treat as paragraph text
            let span = lex.advance().span;
            skip_newline(lex);
            Some(Block::Paragraph(Paragraph {
                content: InlineContent::plain("---"),
                span,
            }))
        }
        Token::Tag(_) | Token::UnknownTag { .. } => {
            Some(parse_block_tag(lex))
        }
        Token::Text(_) | Token::RawLine(_) => parse_paragraph(lex),
        Token::PropertiesOpen | Token::PropertiesClose | Token::BlockCommentClose
        | Token::HtmlClose { .. } | Token::BlockquoteContinuation => {
            // Stray structural tokens — treat as text paragraph
            parse_paragraph(lex)
        }
        _ => {
            // Skip unknown tokens
            lex.skip_to_next_line();
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Heading + property drawer
// ---------------------------------------------------------------------------

fn parse_heading(lex: &mut Lexer<'_>, level: u8, errors: &mut Vec<ParseError>) -> Option<Block> {
    let head_span = lex.advance().span; // consume Heading token
    let raw = consume_raw_line(lex);
    skip_newline(lex);

    let text = raw.trim_start();
    let content_start = text.find(' ').map(|i| i + 1).unwrap_or(text.len());
    let content_text = &text[content_start..];
    let content = build_inline_content(content_text, head_span);

    // Look ahead for property drawer
    let saved = lex.position();
    skip_blank_lines(lex);

    let properties = if matches!(lex.peek().kind, Token::PropertiesOpen) {
        Some(parse_property_drawer(lex, errors))
    } else {
        lex.set_position(saved);
        None
    };

    Some(Block::Heading(Heading {
        level,
        content,
        properties,
        span: head_span,
    }))
}

fn parse_property_drawer(lex: &mut Lexer<'_>, errors: &mut Vec<ParseError>) -> PropertyDrawer {
    let open_span = lex.advance().span; // consume PropertiesOpen
    skip_newline(lex);

    let mut entries = HashMap::new();
    let mut last_span = open_span;

    loop {
        if lex.is_eof() {
            errors.push(ParseError {
                kind: ParseErrorKind::UnexpectedToken,
                span: open_span,
                message: "#properties block opened but never closed with #end".to_string(),
            });
            break;
        }

        match &lex.peek().kind {
            Token::PropertiesClose => {
                last_span = lex.advance().span;
                skip_newline(lex);
                break;
            }
            Token::BlankLine => {
                lex.advance();
                skip_newline(lex);
            }
            _ => {
                let line_span = lex.peek().span;
                let raw = extract_raw_line(lex);
                last_span = line_span;
                let trimmed = raw.trim();
                if let Some((key, value)) = trimmed.split_once('=') {
                    entries.insert(key.trim().to_string(), value.trim().to_string());
                } else {
                    errors.push(ParseError {
                        kind: ParseErrorKind::UnexpectedToken,
                        span: line_span,
                        message: format!("invalid property line: {trimmed}"),
                    });
                }
            }
        }
    }

    PropertyDrawer {
        entries,
        span: open_span.merge(last_span),
    }
}

// ---------------------------------------------------------------------------
// Code block
// ---------------------------------------------------------------------------

fn parse_code_block(lex: &mut Lexer<'_>, errors: &mut Vec<ParseError>) -> Block {
    let tok = lex.advance();
    let open_span = tok.span;

    let (fence_char, fence_len, info_string) = match &tok.kind {
        Token::FencedCodeOpen { info, fence_char, fence_len } => (*fence_char, *fence_len, info.clone()),
        Token::FencedCodeClose { fence_char, fence_len } => (*fence_char, *fence_len, String::new()),
        _ => unreachable!(),
    };
    skip_newline(lex);
    let info_str = &info_string;

    let (lang, code_tags, attributes) = parse_code_info(info_str, open_span);
    let mut body_lines: Vec<String> = Vec::new();

    loop {
        if lex.is_eof() {
            errors.push(ParseError {
                kind: ParseErrorKind::UnclosedCodeFence,
                span: open_span,
                message: "code fence opened but never closed".to_string(),
            });
            break;
        }

        let is_close = matches!(
            &lex.peek().kind,
            Token::FencedCodeClose { fence_char: fc, fence_len: fl }
                if *fc == fence_char && *fl >= fence_len
        );

        if is_close {
            let close_span = lex.advance().span;
            skip_newline(lex);
            return Block::CodeBlock(CodeBlock {
                lang,
                tags: code_tags,
                attributes,
                body: body_lines.join("\n"),
                span: open_span.merge(close_span),
            });
        }

        let raw = extract_raw_line(lex);
        body_lines.push(raw);
    }

    Block::CodeBlock(CodeBlock {
        lang,
        tags: code_tags,
        attributes,
        body: body_lines.join("\n"),
        span: open_span,
    })
}

fn parse_code_info(info: &str, span: Span) -> (Option<String>, Vec<Tag>, HashMap<String, String>) {
    let parts: Vec<&str> = info.split_whitespace().collect();
    if parts.is_empty() {
        return (None, Vec::new(), HashMap::new());
    }
    let lang = parts.first()
        .filter(|p| !p.starts_with('#') && !p.contains('='))
        .map(|p| p.to_string());
    let meta_start = if lang.is_some() { 1 } else { 0 };
    let meta_str = parts[meta_start..].join(" ");
    let (tag_list, attrs) = parse_metadata(&meta_str, span);
    (lang, tag_list, attrs)
}

fn parse_metadata(info: &str, span: Span) -> (Vec<Tag>, HashMap<String, String>) {
    let mut tag_list = Vec::new();
    let mut attrs = HashMap::new();
    for part in info.split_whitespace() {
        if part.starts_with('#') && part.len() > 1 {
            tag_list.push(tags::parse_tag(&part[1..], None, span));
        } else if let Some((key, value)) = part.split_once('=') {
            attrs.insert(key.to_string(), value.to_string());
        }
    }
    (tag_list, attrs)
}

// ---------------------------------------------------------------------------
// Callout
// ---------------------------------------------------------------------------

fn parse_callout(lex: &mut Lexer<'_>, errors: &mut Vec<ParseError>) -> Block {
    let tok = lex.advance();
    let open_span = tok.span;

    let (kind, metadata) = match &tok.kind {
        Token::CalloutStart { kind, metadata } => (kind.clone(), metadata.clone()),
        _ => unreachable!(),
    };

    let (callout_tags, attributes) = match metadata.as_deref() {
        Some(meta) => parse_metadata(meta, open_span),
        None => (Vec::new(), HashMap::new()),
    };

    // Get raw line text to extract content after [!type][metadata]
    let mut content_lines: Vec<String> = Vec::new();
    let raw = consume_raw_line(lex);
    skip_newline(lex);

    // Extract content after the [!type] (and optional [metadata]) on the first line
    let first_text = raw.trim_start();
    if let Some(rest) = first_text.strip_prefix('>') {
        let rest = rest.trim_start();
        if let Some(after_type) = rest.find(']') {
            let mut after = &rest[after_type + 1..];
            let trimmed_after = after.trim_start();
            if trimmed_after.starts_with('[') {
                if let Some(meta_end) = trimmed_after.find(']') {
                    after = &trimmed_after[meta_end + 1..];
                }
            }
            let after = after.trim();
            if !after.is_empty() {
                content_lines.push(after.to_string());
            }
        }
    }

    let mut last_span = open_span;

    // Collect continuation lines
    loop {
        if matches!(lex.peek().kind, Token::BlockquoteContinuation) {
            last_span = lex.advance().span;
            let raw = consume_raw_line(lex);
            skip_newline(lex);
            let text = raw.trim_start();
            let stripped = text.strip_prefix('>').unwrap_or(text);
            content_lines.push(stripped.trim_start().to_string());
        } else {
            break;
        }
    }

    let inner_source = content_lines.join("\n");
    let inner_result = parse_document(&inner_source);
    errors.extend(inner_result.errors);

    Block::Callout(Callout {
        kind,
        tags: callout_tags,
        attributes,
        content: inner_result.document.children,
        span: open_span.merge(last_span),
    })
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

fn parse_list(lex: &mut Lexer<'_>) -> Block {
    let first_span = lex.peek().span;
    let list_kind = match &lex.peek().kind {
        Token::ListMarker { ordered, .. } => if *ordered { ListKind::Ordered } else { ListKind::Unordered },
        _ => unreachable!(),
    };

    // Collect all list items with their indents as a flat sequence
    let mut flat_items: Vec<ListItem> = Vec::new();
    let mut last_span = first_span;

    while matches!(lex.peek().kind, Token::ListMarker { .. }) {
        let tok = lex.advance();
        let item_span = tok.span;
        let indent = match &tok.kind {
            Token::ListMarker { indent, .. } => *indent,
            _ => 0,
        };

        let raw = consume_raw_line(lex);
        skip_newline(lex);
        last_span = item_span;

        let (checkbox, content_text) = parse_list_item_content(&raw);
        let (term_text, desc) = if let Some((term, desc_text)) = content_text.split_once(" :: ") {
            (term, Some(build_inline_content(desc_text, item_span)))
        } else {
            (content_text, None)
        };

        let content = build_inline_content(term_text, item_span);

        flat_items.push(ListItem {
            checkbox,
            content,
            description: desc,
            children: Vec::new(),
            indent,
            span: item_span,
        });
    }

    // Build nested structure from flat items based on indent levels
    let items = nest_list_items(flat_items);

    Block::List(List {
        kind: list_kind,
        items,
        span: first_span.merge(last_span),
    })
}

/// Convert a flat sequence of list items (with indent levels) into a nested tree.
/// Items with greater indent become children of the preceding item with lesser indent.
fn nest_list_items(flat: Vec<ListItem>) -> Vec<ListItem> {
    if flat.is_empty() {
        return flat;
    }

    let mut result: Vec<ListItem> = Vec::new();
    let mut i = 0;

    while i < flat.len() {
        let mut item = flat[i].clone();
        i += 1;

        // Collect children: subsequent items with indent > this item's indent
        let mut child_items: Vec<ListItem> = Vec::new();

        while i < flat.len() && flat[i].indent > item.indent {
            child_items.push(flat[i].clone());
            i += 1;
        }

        if !child_items.is_empty() {
            // Determine child list kind from the first child
            let child_kind = if child_items.iter().any(|c| c.indent > 0) {
                // Mixed — use unordered as default
                ListKind::Unordered
            } else {
                ListKind::Unordered
            };

            let nested_children = nest_list_items(child_items);
            let child_span = if let Some(last) = nested_children.last() {
                item.span.merge(last.span)
            } else {
                item.span
            };

            item.children.push(Block::List(List {
                kind: child_kind,
                items: nested_children,
                span: child_span,
            }));
        }

        result.push(item);
    }

    result
}

fn parse_list_item_content(text: &str) -> (Option<Checkbox>, &str) {
    let trimmed = text.trim_start();
    let after_marker = if trimmed.starts_with("- ") || trimmed.starts_with("+ ") || trimmed.starts_with("* ") {
        &trimmed[2..]
    } else {
        let digits_end = trimmed.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
        if trimmed[digits_end..].starts_with(". ") {
            &trimmed[digits_end + 2..]
        } else {
            trimmed
        }
    };

    if after_marker.starts_with("[ ] ") {
        (Some(Checkbox::Unchecked), &after_marker[4..])
    } else if after_marker.starts_with("[x] ") || after_marker.starts_with("[X] ") {
        (Some(Checkbox::Checked), &after_marker[4..])
    } else {
        (None, after_marker)
    }
}

// ---------------------------------------------------------------------------
// Table
// ---------------------------------------------------------------------------

fn parse_table(lex: &mut Lexer<'_>) -> Block {
    let first_span = lex.peek().span;
    lex.advance(); // consume TableRow
    let first_raw = consume_raw_line(lex);
    skip_newline(lex);

    let headers = parse_table_row_content(&first_raw, first_span);
    let mut alignments = Vec::new();
    let mut rows: Vec<Vec<InlineContent>> = Vec::new();
    let mut last_span = first_span;

    // Check for separator
    if matches!(lex.peek().kind, Token::TableRow) {
        let sep_span = lex.peek().span;
        let saved = lex.position();
        lex.advance();
        let sep_raw = consume_raw_line(lex);
        if let Some(aligns) = try_parse_separator(&sep_raw) {
            alignments = aligns;
            last_span = sep_span;
            skip_newline(lex);
        } else {
            lex.set_position(saved);
        }
    }

    // Data rows
    while matches!(lex.peek().kind, Token::TableRow) {
        let row_span = lex.peek().span;
        lex.advance();
        let raw = consume_raw_line(lex);
        skip_newline(lex);
        rows.push(parse_table_row_content(&raw, row_span));
        last_span = row_span;
    }

    if alignments.is_empty() && !headers.is_empty() {
        alignments = vec![Alignment::None; headers.len()];
    }

    Block::Table(Table {
        headers,
        alignments,
        rows,
        span: first_span.merge(last_span),
    })
}

fn parse_table_row_content(line: &str, span: Span) -> Vec<InlineContent> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|')
        .map(|cell| build_inline_content(cell.trim(), span))
        .collect()
}

fn try_parse_separator(line: &str) -> Option<Vec<Alignment>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let cells: Vec<&str> = inner.split('|').collect();
    let mut alignments = Vec::new();
    for cell in cells {
        let cell = cell.trim();
        if cell.is_empty() { return None; }
        let left = cell.starts_with(':');
        let right = cell.ends_with(':');
        let middle = if left { &cell[1..] } else { cell };
        let middle = if right { &middle[..middle.len() - 1] } else { middle };
        if middle.is_empty() || !middle.chars().all(|c| c == '-') { return None; }
        alignments.push(match (left, right) {
            (true, true) => Alignment::Center,
            (true, false) => Alignment::Left,
            (false, true) => Alignment::Right,
            (false, false) => Alignment::None,
        });
    }
    Some(alignments)
}

// ---------------------------------------------------------------------------
// HTML block
// ---------------------------------------------------------------------------

fn parse_html_block(lex: &mut Lexer<'_>, errors: &mut Vec<ParseError>) -> Block {
    let tok = lex.advance();
    let open_span = tok.span;
    let open_tag = match &tok.kind {
        Token::HtmlOpen { tag } => tag.clone(),
        _ => unreachable!(),
    };

    let raw = consume_raw_line(lex);
    skip_newline(lex);

    let mut raw_lines = vec![raw.clone()];
    let mut last_span = open_span;

    let trimmed = raw.trim();
    let is_self_closing = trimmed.ends_with("/>")
        || is_void_element(&open_tag)
        || trimmed.contains(&format!("</{open_tag}"));

    if !is_self_closing {
        loop {
            if lex.is_eof() {
                errors.push(ParseError {
                    kind: ParseErrorKind::UnclosedHtmlBlock,
                    span: open_span,
                    message: format!("HTML block <{open_tag}> never closed"),
                });
                break;
            }
            if matches!(lex.peek().kind, Token::BlankLine) {
                break;
            }
            let is_close = matches!(&lex.peek().kind, Token::HtmlClose { tag } if *tag == open_tag);
            let line_span = lex.peek().span;
            let line_raw = extract_raw_line(lex);
            raw_lines.push(line_raw);
            last_span = line_span;
            if is_close {
                break;
            }
        }
    }

    Block::HtmlBlock(HtmlBlock {
        raw: raw_lines.join("\n"),
        span: open_span.merge(last_span),
    })
}

fn is_void_element(tag: &str) -> bool {
    matches!(tag,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input"
        | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

fn parse_line_comment(lex: &mut Lexer<'_>) -> Block {
    let span = lex.advance().span; // LineComment
    let raw = consume_raw_line(lex);
    skip_newline(lex);
    let text = raw.trim_start().strip_prefix("//").unwrap_or(&raw).trim().to_string();
    Block::Comment(Comment { text, span })
}

fn parse_block_comment(lex: &mut Lexer<'_>) -> Block {
    let open_span = lex.advance().span;
    let first_raw = consume_raw_line(lex);
    skip_newline(lex);

    let mut text_lines = vec![first_raw.trim_start().strip_prefix("/*").unwrap_or("").trim().to_string()];
    let mut last_span = open_span;

    loop {
        if lex.is_eof() { break; }
        if matches!(lex.peek().kind, Token::BlockCommentClose) {
            last_span = lex.advance().span;
            let raw = consume_raw_line(lex);
            skip_newline(lex);
            let trimmed = raw.trim();
            let without = trimmed.strip_suffix("*/").unwrap_or(trimmed);
            if !without.trim().is_empty() {
                text_lines.push(without.trim().to_string());
            }
            break;
        }
        last_span = lex.peek().span;
        let raw = extract_raw_line(lex);
        text_lines.push(raw);
    }

    Block::Comment(Comment {
        text: text_lines.join("\n"),
        span: open_span.merge(last_span),
    })
}

// ---------------------------------------------------------------------------
// Footnote definition
// ---------------------------------------------------------------------------

fn parse_footnote_def(lex: &mut Lexer<'_>) -> Block {
    let tok = lex.advance();
    let span = tok.span;
    let label = match &tok.kind {
        Token::FootnoteDefStart { label } => label.clone(),
        _ => unreachable!(),
    };
    let raw = consume_raw_line(lex);
    skip_newline(lex);

    let prefix = format!("[^{label}]: ");
    let content_text = raw.trim().strip_prefix(&prefix).unwrap_or("");
    let content = build_inline_content(content_text, span);

    Block::FootnoteDefinition(FootnoteDefinition { label, content, span })
}

// ---------------------------------------------------------------------------
// Block tag
// ---------------------------------------------------------------------------

fn parse_block_tag(lex: &mut Lexer<'_>) -> Block {
    let tok = lex.advance();
    let span = tok.span;

    let name = match &tok.kind {
        Token::Tag(kw) => kw.as_str().to_string(),
        Token::UnknownTag { name } => name.clone(),
        _ => unreachable!(),
    };

    // Consume optional argument
    let arg_string = if matches!(lex.peek().kind, Token::TagArg(_)) {
        let arg_tok = lex.advance();
        match &arg_tok.kind {
            Token::TagArg(a) => Some(a.clone()),
            _ => None,
        }
    } else {
        None
    };
    skip_newline(lex);

    Block::BlockTag(tags::parse_tag(&name, arg_string.as_deref(), span))
}

// ---------------------------------------------------------------------------
// Paragraph
// ---------------------------------------------------------------------------

fn parse_paragraph(lex: &mut Lexer<'_>) -> Option<Block> {
    let first_span = lex.peek().span;
    let mut text_lines: Vec<String> = Vec::new();
    let mut last_span = first_span;

    loop {
        match &lex.peek().kind {
            Token::Text(_) | Token::RawLine(_) | Token::BlockquoteContinuation
            | Token::HtmlClose { .. } | Token::PropertiesOpen | Token::PropertiesClose
            | Token::BlockCommentClose => {
                last_span = lex.peek().span;
                let raw = extract_raw_line(lex);
                text_lines.push(raw);
            }
            _ => break,
        }
    }

    if text_lines.is_empty() {
        return None;
    }

    let full_text = text_lines.join("\n");
    let span = first_span.merge(last_span);
    let content = build_inline_content(&full_text, span);

    Some(Block::Paragraph(Paragraph { content, span }))
}

// ---------------------------------------------------------------------------
// Inline content builder (uses lexer::tokenize_inline)
// ---------------------------------------------------------------------------

fn build_inline_content(text: &str, span: Span) -> InlineContent {
    let tokens = lexer::tokenize_inline(text, span);
    tokens_to_inline_content(&tokens, span)
}

fn tokens_to_inline_content(tokens: &[crate::tokens::Spanned], base_span: Span) -> InlineContent {
    let mut segments: Vec<InlineSegment> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i].kind {
            Token::Text(t) => {
                segments.push(InlineSegment::Text(t.clone()));
                i += 1;
            }
            Token::InlineCode(c) => {
                segments.push(InlineSegment::Code(c.clone()));
                i += 1;
            }
            Token::BoldDelim => {
                // Collect inner tokens until matching BoldDelim
                i += 1;
                let inner_end = find_matching_delim(&tokens[i..], Token::BoldDelim);
                let inner = tokens_to_inline_content(&tokens[i..i + inner_end], base_span);
                segments.push(InlineSegment::Bold(inner));
                i += inner_end + 1; // skip closing delim
            }
            Token::ItalicDelim => {
                i += 1;
                let inner_end = find_matching_delim(&tokens[i..], Token::ItalicDelim);
                let inner = tokens_to_inline_content(&tokens[i..i + inner_end], base_span);
                segments.push(InlineSegment::Italic(inner));
                i += inner_end + 1;
            }
            Token::StrikethroughDelim => {
                i += 1;
                let inner_end = find_matching_delim(&tokens[i..], Token::StrikethroughDelim);
                let inner = tokens_to_inline_content(&tokens[i..i + inner_end], base_span);
                segments.push(InlineSegment::Strikethrough(inner));
                i += inner_end + 1;
            }
            Token::Link { text, url, title, meta } => {
                let (link_tags, attrs) = match meta.as_deref() {
                    Some(m) => parse_metadata(m, base_span),
                    None => (Vec::new(), HashMap::new()),
                };
                segments.push(InlineSegment::Link(Link {
                    text: text.clone(),
                    url: url.clone(),
                    title: title.clone(),
                    tags: link_tags,
                    attributes: attrs,
                }));
                i += 1;
            }
            Token::FootnoteRef { label } => {
                segments.push(InlineSegment::FootnoteRef(label.clone()));
                i += 1;
            }
            Token::Tag(kw) => {
                let arg = if i + 1 < tokens.len() {
                    if let Token::TagArg(a) = &tokens[i + 1].kind {
                        i += 1;
                        Some(a.as_str())
                    } else { None }
                } else { None };
                let tag = tags::parse_tag(kw.as_str(), arg, base_span);
                segments.push(InlineSegment::Tag(tag));
                i += 1;
            }
            Token::UnknownTag { name } => {
                let arg = if i + 1 < tokens.len() {
                    if let Token::TagArg(a) = &tokens[i + 1].kind {
                        i += 1;
                        Some(a.as_str())
                    } else { None }
                } else { None };
                let tag = tags::parse_tag(name, arg, base_span);
                segments.push(InlineSegment::Tag(tag));
                i += 1;
            }
            Token::TagArg(a) => {
                // Stray tag arg without a preceding tag — treat as text
                segments.push(InlineSegment::Text(a.clone()));
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    InlineContent { segments }
}

fn find_matching_delim(tokens: &[crate::tokens::Spanned], delim: Token) -> usize {
    for (idx, tok) in tokens.iter().enumerate() {
        if tok.kind == delim {
            return idx;
        }
    }
    tokens.len() // no match found — consume everything
}

// ---------------------------------------------------------------------------
// Token stream helpers
// ---------------------------------------------------------------------------

/// Consume all tokens until the next Newline/Eof, collecting RawLine content.
/// Returns the raw text of the line.
fn extract_raw_line(lex: &mut Lexer<'_>) -> String {
    let mut raw = String::new();
    loop {
        match &lex.peek().kind {
            Token::Newline | Token::Eof => {
                if matches!(lex.peek().kind, Token::Newline) {
                    lex.advance();
                }
                break;
            }
            Token::RawLine(text) => {
                raw = text.clone();
                lex.advance();
            }
            _ => {
                lex.advance();
            }
        }
    }
    raw
}

/// Consume and return the RawLine text if present (without advancing past Newline).
fn consume_raw_line(lex: &mut Lexer<'_>) -> String {
    if let Token::RawLine(text) = &lex.peek().kind {
        let t = text.clone();
        lex.advance();
        t
    } else {
        String::new()
    }
}

fn skip_newline(lex: &mut Lexer<'_>) {
    if matches!(lex.peek().kind, Token::Newline) {
        lex.advance();
    }
}

fn skip_blank_lines(lex: &mut Lexer<'_>) {
    while matches!(lex.peek().kind, Token::BlankLine) {
        lex.advance();
        skip_newline(lex);
    }
}

// ===========================================================================
// Tests — mirror key tests from parser.rs against parser_v2
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags::TagKind;

    #[test]
    fn test_v2_full_document() {
        let src = "---\ntitle: Test\n---\n\n# Heading\n\nSome text with #todo inline tag.\n\n```rust #tangle file=main.rs\nfn main() {}\n```\n\n#deadline 2026-04-10\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let doc = &result.document;
        assert!(doc.frontmatter.is_some());

        let has_heading = doc.children.iter().any(|b| matches!(b, Block::Heading(h) if h.level == 1));
        assert!(has_heading, "should have heading");

        let code_block = doc.children.iter().find_map(|b| match b {
            Block::CodeBlock(cb) => Some(cb),
            _ => None,
        });
        assert!(code_block.is_some(), "should have code block");
        let cb = code_block.unwrap();
        assert_eq!(cb.lang.as_deref(), Some("rust"));
        assert!(cb.tags.iter().any(|t| matches!(t.kind, TagKind::Tangle)));
        assert_eq!(cb.attributes.get("file").map(|s| s.as_str()), Some("main.rs"));
        assert_eq!(cb.body, "fn main() {}");

        let has_deadline = doc.children.iter().any(|b| {
            matches!(b, Block::BlockTag(Tag { kind: TagKind::Deadline { .. }, .. }))
        });
        assert!(has_deadline, "should have deadline");
    }

    #[test]
    fn test_v2_inline_tags() {
        let src = "some text #todo fix this\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let para = result.document.children.iter().find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        }).unwrap();

        let tags: Vec<_> = para.content.tags();
        assert_eq!(tags.len(), 1);
        assert!(matches!(tags[0].kind, TagKind::Todo { .. }));
    }

    #[test]
    fn test_v2_bold_italic() {
        let src = "**bold** and *italic* text\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let para = result.document.children.iter().find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        }).unwrap();

        assert!(para.content.segments.iter().any(|s| matches!(s, InlineSegment::Bold(_))));
        assert!(para.content.segments.iter().any(|s| matches!(s, InlineSegment::Italic(_))));
    }

    #[test]
    fn test_v2_link() {
        let src = "[click](https://example.com)\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let para = result.document.children.iter().find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        }).unwrap();

        assert!(para.content.segments.iter().any(|s| matches!(s, InlineSegment::Link(_))));
    }

    #[test]
    fn test_v2_table() {
        let src = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let table = result.document.children.iter().find_map(|b| match b {
            Block::Table(t) => Some(t),
            _ => None,
        });
        assert!(table.is_some());
        let t = table.unwrap();
        assert_eq!(t.headers.len(), 2);
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn test_v2_callout() {
        let src = "> [!note]\n> This is a note.\n> With two lines.\n\nRegular text.\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let callout = result.document.children.iter().find_map(|b| match b {
            Block::Callout(c) => Some(c),
            _ => None,
        });
        assert!(callout.is_some());
        assert_eq!(callout.unwrap().kind, "note");
    }

    #[test]
    fn test_v2_list_with_checkboxes() {
        let src = "- [ ] Unchecked task\n- [x] Checked task\n- Regular item\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let list = result.document.children.iter().find_map(|b| match b {
            Block::List(l) => Some(l),
            _ => None,
        });
        let l = list.unwrap();
        assert_eq!(l.items.len(), 3);
        assert_eq!(l.items[0].checkbox, Some(Checkbox::Unchecked));
        assert_eq!(l.items[1].checkbox, Some(Checkbox::Checked));
        assert_eq!(l.items[2].checkbox, None);
    }

    #[test]
    fn test_v2_property_drawer() {
        let src = "## My Task\n\n#properties\nid = abc-123\neffort = 2h30m\n#end\n\nContent.\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let h = result.document.children.iter().find_map(|b| match b {
            Block::Heading(h) => Some(h),
            _ => None,
        }).unwrap();
        assert!(h.properties.is_some());
        let props = h.properties.as_ref().unwrap();
        assert_eq!(props.entries.get("id").map(|s| s.as_str()), Some("abc-123"));
    }

    #[test]
    fn test_v2_clock_tags() {
        let src = "#clock-in 2026-04-03T09:00\n#clock-out 2026-04-03T10:30\n#clock 1h30m\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let tags: Vec<_> = result.document.children.iter()
            .filter_map(|b| match b { Block::BlockTag(t) => Some(t), _ => None })
            .collect();
        assert_eq!(tags.len(), 3);
        assert!(matches!(tags[0].kind, TagKind::ClockIn { .. }));
        assert!(matches!(tags[1].kind, TagKind::ClockOut { .. }));
        assert!(matches!(tags[2].kind, TagKind::Clock(_)));
    }

    #[test]
    fn test_v2_comments() {
        let src = "// this is a comment\n\nText.\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let has_comment = result.document.children.iter().any(|b| matches!(b, Block::Comment(_)));
        assert!(has_comment);
    }

    #[test]
    fn test_v2_horizontal_rule() {
        let src = "Text above.\n\n***\n\nText below.\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let has_hr = result.document.children.iter().any(|b| matches!(b, Block::HorizontalRule(_)));
        assert!(has_hr);
    }

    #[test]
    fn test_v2_footnote() {
        let src = "[^1]: This is a footnote.\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let has_fn = result.document.children.iter().any(|b| matches!(b, Block::FootnoteDefinition(_)));
        assert!(has_fn);
    }

    #[test]
    fn test_v2_html_block() {
        let src = "<div class=\"container\">\n  <p>Hello</p>\n</div>\n";
        let result = parse_document(src);

        let html = result.document.children.iter().find_map(|b| match b {
            Block::HtmlBlock(h) => Some(h),
            _ => None,
        });
        assert!(html.is_some());
        assert!(html.unwrap().raw.contains("<div"));
    }

    #[test]
    fn test_v2_unclosed_fence_recovery() {
        let src = "```rust\nfn main() {}\n\n# Next heading\n";
        let result = parse_document(src);
        assert!(!result.errors.is_empty());
        assert!(!result.document.children.is_empty());
    }

    #[test]
    fn test_v2_nested_list() {
        let src = "- Parent one\n  - Child A\n  - Child B\n- Parent two\n  - Child C\n    - Grandchild\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let list = result.document.children.iter().find_map(|b| match b {
            Block::List(l) => Some(l),
            _ => None,
        }).expect("should have a list");

        // Two top-level items
        assert_eq!(list.items.len(), 2, "should have 2 top-level items");

        // First parent has 2 children
        assert_eq!(list.items[0].content.plain_text(), "Parent one");
        assert_eq!(list.items[0].children.len(), 1, "parent one should have 1 child block (nested list)");
        if let Block::List(child_list) = &list.items[0].children[0] {
            assert_eq!(child_list.items.len(), 2, "child list should have 2 items");
            assert_eq!(child_list.items[0].content.plain_text(), "Child A");
            assert_eq!(child_list.items[1].content.plain_text(), "Child B");
        } else {
            panic!("expected nested list");
        }

        // Second parent has 1 child with its own grandchild
        assert_eq!(list.items[1].content.plain_text(), "Parent two");
        if let Block::List(child_list) = &list.items[1].children[0] {
            assert_eq!(child_list.items.len(), 1);
            assert_eq!(child_list.items[0].content.plain_text(), "Child C");
            // Child C has a grandchild
            assert_eq!(child_list.items[0].children.len(), 1);
            if let Block::List(grandchild_list) = &child_list.items[0].children[0] {
                assert_eq!(grandchild_list.items.len(), 1);
                assert_eq!(grandchild_list.items[0].content.plain_text(), "Grandchild");
            } else {
                panic!("expected grandchild list");
            }
        } else {
            panic!("expected nested list for parent two");
        }
    }

    #[test]
    fn test_v2_nested_checkbox_list() {
        let src = "- [ ] Parent task\n  - [x] Subtask done\n  - [ ] Subtask pending\n";
        let result = parse_document(src);
        assert!(result.errors.is_empty());

        let list = result.document.children.iter().find_map(|b| match b {
            Block::List(l) => Some(l),
            _ => None,
        }).unwrap();

        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].checkbox, Some(Checkbox::Unchecked));

        if let Block::List(child_list) = &list.items[0].children[0] {
            assert_eq!(child_list.items.len(), 2);
            assert_eq!(child_list.items[0].checkbox, Some(Checkbox::Checked));
            assert_eq!(child_list.items[1].checkbox, Some(Checkbox::Unchecked));
        } else {
            panic!("expected nested list");
        }
    }
}
