//! Lexer for morg-mode source files.
//!
//! Two-phase design:
//! 1. **Block tokenizer** (`Lexer::new`) classifies each line into a block-level token
//!    plus a `RawLine` carrying the raw text. One token sequence per line, separated by `Newline`.
//! 2. **Inline tokenizer** (`tokenize_inline`) is called by the parser on demand to break
//!    raw text into inline tokens (bold, italic, tags, links, etc.). This is never called
//!    eagerly — the parser controls when inline parsing happens.

use crate::span::Span;
use crate::tokens::{Keyword, Spanned, Token};

// ===========================================================================
// Block-level lexer
// ===========================================================================

/// A lexer that produces block-level tokens from source text.
pub struct Lexer<'a> {
    source: &'a str,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let tokens = tokenize_blocks(source);
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn peek(&self) -> &Spanned {
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    pub fn advance(&mut self) -> &Spanned {
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            tok
        } else {
            &EOF_TOKEN
        }
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek().kind, Token::Eof)
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Skip tokens until (and including) the next Newline or Eof.
    pub fn skip_to_next_line(&mut self) {
        while self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            if matches!(tok.kind, Token::Newline | Token::Eof) {
                return;
            }
        }
    }
}

static EOF_TOKEN: Spanned = Spanned {
    kind: Token::Eof,
    span: Span {
        start: 0,
        end: 0,
        line: 0,
        col: 0,
    },
};

/// Tokenize source into block-level tokens. Each line produces:
/// - One block-classification token (Heading, FencedCodeOpen, BlankLine, etc.)
/// - A `RawLine` token carrying the full line text
/// - A `Newline` token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrontmatterState {
    BeforeContent,
    InsideFrontmatter,
    Done,
}

fn tokenize_blocks(source: &str) -> Vec<Spanned> {
    let mut tokens = Vec::new();
    let mut byte_offset: usize = 0;
    let mut frontmatter_state = FrontmatterState::BeforeContent;
    let mut fenced_code_depth: u32 = 0;

    let lines: Vec<&str> = source.split('\n').collect();
    let mut line_idx = 0;

    while line_idx < lines.len() {
        let line_text = lines[line_idx];
        let line_number = (line_idx + 1) as u32;
        let span = Span::new(byte_offset, byte_offset + line_text.len(), line_number, 1);

        classify_line(line_text, span, &mut tokens, &mut frontmatter_state);

        // Track fenced code blocks so the setext lookahead is suppressed inside them.
        if let Some(last_block) = tokens.iter().rev().find(|t| {
            matches!(
                t.kind,
                Token::FencedCodeOpen { .. } | Token::FencedCodeClose { .. }
            )
        }) {
            match last_block.kind {
                Token::FencedCodeOpen { .. } => fenced_code_depth = 1,
                Token::FencedCodeClose { .. } => fenced_code_depth = 0,
                _ => {}
            }
        }

        // Setext heading lookahead: if the current line was classified as
        // plain text (Text + RawLine) and the next line is a setext underline
        // (contiguous `=` or `-`, optionally indented 0-3 spaces), rewrite
        // the Text token as a Heading and skip the underline line.
        // Suppressed inside frontmatter and fenced code blocks.
        if line_idx + 1 < lines.len()
            && frontmatter_state == FrontmatterState::Done
            && fenced_code_depth == 0
        {
            let was_text_line = tokens.len() >= 2
                && matches!(tokens[tokens.len() - 2].kind, Token::Text(_))
                && matches!(tokens[tokens.len() - 1].kind, Token::RawLine(_));

            if was_text_line && let Some(level) = try_setext_underline(lines[line_idx + 1]) {
                // Rewrite the Text marker to a Heading token
                let marker_idx = tokens.len() - 2;
                tokens[marker_idx].kind = Token::Heading { level };

                // Emit Newline for the current (heading text) line
                tokens.push(Spanned {
                    kind: Token::Newline,
                    span: Span::new(
                        byte_offset + line_text.len(),
                        byte_offset + line_text.len() + 1,
                        line_number,
                        (line_text.len() + 1) as u32,
                    ),
                });
                byte_offset += line_text.len() + 1;

                // Skip the underline line entirely (no tokens emitted)
                let ul_text = lines[line_idx + 1];
                byte_offset += ul_text.len() + 1;
                line_idx += 2; // skip both current line (already processed) and underline
                continue;
            }
        }

        tokens.push(Spanned {
            kind: Token::Newline,
            span: Span::new(
                byte_offset + line_text.len(),
                byte_offset + line_text.len() + 1,
                line_number,
                (line_text.len() + 1) as u32,
            ),
        });

        byte_offset += line_text.len() + 1;
        line_idx += 1;
    }

    // Replace final Newline with Eof
    if let Some(last) = tokens.last_mut()
        && last.kind == Token::Newline
    {
        last.kind = Token::Eof;
    }

    tokens
}

/// Check if a line is a setext heading underline.
/// Returns `Some(1)` for `=` underlines (h1), `Some(2)` for `-` underlines (h2),
/// or `None` if the line is not a valid setext underline.
///
/// Per CommonMark: 0-3 leading spaces, then one or more `=` or `-` (all the same),
/// then optional trailing spaces. No other characters allowed.
fn try_setext_underline(line: &str) -> Option<u8> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent >= 4 {
        return None;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first = trimmed.as_bytes()[0];
    if first != b'=' && first != b'-' {
        return None;
    }
    if !trimmed.bytes().all(|b| b == first) {
        return None;
    }
    Some(if first == b'=' { 1 } else { 2 })
}

/// Classify a single line into block-level token(s) + RawLine.
fn classify_line(text: &str, span: Span, out: &mut Vec<Spanned>, fm_state: &mut FrontmatterState) {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        if *fm_state == FrontmatterState::BeforeContent {
            *fm_state = FrontmatterState::Done;
        }
        out.push(Spanned {
            kind: Token::BlankLine,
            span,
        });
        return;
    }

    // Comments
    if trimmed.starts_with("//") {
        out.push(Spanned {
            kind: Token::LineComment,
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }
    if trimmed.starts_with("/*") {
        out.push(Spanned {
            kind: Token::BlockCommentOpen,
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }
    if trimmed.ends_with("*/") || trimmed == "*/" {
        out.push(Spanned {
            kind: Token::BlockCommentClose,
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Footnote definition: [^label]: content
    if let Some(label) = try_footnote_def(trimmed) {
        out.push(Spanned {
            kind: Token::FootnoteDefStart { label },
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Link reference definition: [label]: url "title"
    if let Some(label) = try_link_def(trimmed) {
        out.push(Spanned {
            kind: Token::LinkDefStart { label },
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Frontmatter delimiter
    if trimmed == "---" {
        match *fm_state {
            FrontmatterState::BeforeContent => {
                *fm_state = FrontmatterState::InsideFrontmatter;
                out.push(Spanned {
                    kind: Token::FrontmatterDelim,
                    span,
                });
            }
            FrontmatterState::InsideFrontmatter => {
                *fm_state = FrontmatterState::Done;
                out.push(Spanned {
                    kind: Token::FrontmatterDelim,
                    span,
                });
            }
            FrontmatterState::Done => {
                out.push(Spanned {
                    kind: Token::HorizontalRule,
                    span,
                });
            }
        }
        return;
    }
    if *fm_state == FrontmatterState::BeforeContent {
        *fm_state = FrontmatterState::Done;
    }

    // Horizontal rule
    if is_horizontal_rule(trimmed) {
        out.push(Spanned {
            kind: Token::HorizontalRule,
            span,
        });
        return;
    }

    // Code fence — emit a RawLine alongside so the parser can recover the
    // raw text when an inner fence doesn't match the outer block's fence.
    if let Some(tok) = try_code_fence(trimmed) {
        out.push(Spanned { kind: tok, span });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // HTML
    if let Some(tok) = try_html_line(trimmed) {
        out.push(Spanned { kind: tok, span });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Table row
    if trimmed.starts_with('|') {
        out.push(Spanned {
            kind: Token::TableRow,
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Callout
    if let Some((kind, metadata)) = try_callout(trimmed) {
        out.push(Spanned {
            kind: Token::CalloutStart { kind, metadata },
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Blockquote continuation
    if trimmed.starts_with('>') {
        out.push(Spanned {
            kind: Token::BlockquoteContinuation,
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // List item
    if let Some((indent, ordered)) = try_list_item(text) {
        out.push(Spanned {
            kind: Token::ListMarker { ordered, indent },
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Heading
    if let Some(level) = try_heading(text) {
        out.push(Spanned {
            kind: Token::Heading { level },
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Properties markers
    if trimmed == "#properties" {
        out.push(Spanned {
            kind: Token::PropertiesOpen,
            span,
        });
        return;
    }
    if trimmed == "#end" {
        out.push(Spanned {
            kind: Token::PropertiesClose,
            span,
        });
        return;
    }

    // Block-level tag check
    if let Some(rest) = trimmed.strip_prefix('#')
        && !rest.is_empty()
        && !rest.starts_with(' ')
    {
        let first = rest.chars().next().unwrap();
        if first.is_alphanumeric() || first == '_' {
            let name_end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .unwrap_or(rest.len());
            let name = &rest[..name_end];
            let tag_tok = match Keyword::from_str(name) {
                Some(kw) => Token::Tag(kw),
                None => Token::UnknownTag {
                    name: name.to_string(),
                },
            };
            out.push(Spanned {
                kind: tag_tok,
                span,
            });
            let arg = rest[name_end..].trim();
            if !arg.is_empty() {
                out.push(Spanned {
                    kind: Token::TagArg(arg.to_string()),
                    span,
                });
            }
            return;
        }
    }

    // Indented code block: 4+ leading spaces or a leading tab
    if text.starts_with("    ") || text.starts_with('\t') {
        out.push(Spanned {
            kind: Token::IndentedCodeLine,
            span,
        });
        out.push(Spanned {
            kind: Token::RawLine(text.to_string()),
            span,
        });
        return;
    }

    // Plain text — emit as RawLine for parser to inline-tokenize
    out.push(Spanned {
        kind: Token::Text(String::new()),
        span,
    }); // marker: this is a text line
    out.push(Spanned {
        kind: Token::RawLine(text.to_string()),
        span,
    });
}

// ===========================================================================
// Inline tokenizer (called by parser on demand)
// ===========================================================================

/// Tokenize inline content from raw text. Called by the parser when it needs
/// to break a text line into inline segments (bold, italic, tags, links, etc.).
pub fn tokenize_inline(text: &str, span: Span) -> Vec<Spanned> {
    let mut out = Vec::new();
    tokenize_inline_into(text, span, &mut out);
    out
}

fn tokenize_inline_into(text: &str, span: Span, out: &mut Vec<Spanned>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut current_text = String::new();

    while i < bytes.len() {
        let ch = bytes[i];

        // Backslash escape
        if ch == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'#' || next == b'[' || next == b'*' || next == b'~' || next == b'`' {
                current_text.push(next as char);
                i += 2;
                continue;
            }
            current_text.push('\\');
            i += 1;
            continue;
        }

        // Inline code
        if ch == b'`'
            && let Some((code, end)) = scan_backtick_code(text, i)
        {
            flush_text_token(&mut current_text, span, out);
            out.push(Spanned {
                kind: Token::InlineCode(code.to_string()),
                span,
            });
            i = end;
            continue;
        }

        // Bold **
        if ch == b'*' && peek(bytes, i + 1) == Some(b'*') {
            flush_text_token(&mut current_text, span, out);
            out.push(Spanned {
                kind: Token::BoldDelim,
                span,
            });
            i += 2;
            continue;
        }

        // Strikethrough ~~
        if ch == b'~' && peek(bytes, i + 1) == Some(b'~') {
            flush_text_token(&mut current_text, span, out);
            out.push(Spanned {
                kind: Token::StrikethroughDelim,
                span,
            });
            i += 2;
            continue;
        }

        // Italic * (not **)
        if ch == b'*' && peek(bytes, i + 1) != Some(b'*') {
            flush_text_token(&mut current_text, span, out);
            out.push(Spanned {
                kind: Token::ItalicDelim,
                span,
            });
            i += 1;
            continue;
        }

        // Image ![alt](url)
        if ch == b'!'
            && peek(bytes, i + 1) == Some(b'[')
            && let Some((img_tok, end)) = try_image(text, i)
        {
            flush_text_token(&mut current_text, span, out);
            out.push(Spanned {
                kind: img_tok,
                span,
            });
            i = end;
            continue;
        }

        // Autolink <url> or <email>
        if ch == b'<' {
            if let Some((link_tok, end)) = try_autolink(text, i) {
                flush_text_token(&mut current_text, span, out);
                out.push(Spanned {
                    kind: link_tok,
                    span,
                });
                i = end;
                continue;
            }
            current_text.push('<');
            i += 1;
            continue;
        }

        // Footnote ref [^label]
        if ch == b'['
            && peek(bytes, i + 1) == Some(b'^')
            && let Some((label, end)) = try_footnote_ref(text, i)
        {
            flush_text_token(&mut current_text, span, out);
            out.push(Spanned {
                kind: Token::FootnoteRef { label },
                span,
            });
            i = end;
            continue;
        }

        // Link [text](url ...) or reference link [text][label] / [label]
        if ch == b'[' {
            // Try inline link first: [text](url)
            if let Some((link_tok, end)) = try_link(text, i) {
                flush_text_token(&mut current_text, span, out);
                out.push(Spanned {
                    kind: link_tok,
                    span,
                });
                i = end;
                continue;
            }
            // Try reference link: [text][label] or [label]
            if let Some((ref_tok, end)) = try_link_ref(text, i) {
                flush_text_token(&mut current_text, span, out);
                out.push(Spanned {
                    kind: ref_tok,
                    span,
                });
                i = end;
                continue;
            }
        }

        // Tag
        if ch == b'#' {
            if let Some(next) = peek(bytes, i + 1)
                && ((next as char).is_alphanumeric() || next == b'_')
            {
                flush_text_token(&mut current_text, span, out);
                let (tok, arg_tok, end) = tokenize_tag(text, i + 1, span);
                out.push(Spanned { kind: tok, span });
                if let Some(at) = arg_tok {
                    out.push(Spanned { kind: at, span });
                }
                i = end;
                continue;
            }
            current_text.push('#');
            i += 1;
            continue;
        }

        // Default: push the character as text.
        // For ASCII bytes this is trivial. For multi-byte UTF-8 sequences
        // we must decode the full character rather than pushing each byte
        // as a separate Latin-1 codepoint.
        if ch < 0x80 {
            current_text.push(ch as char);
            i += 1;
        } else {
            // Determine UTF-8 sequence length from the leading byte and
            // push the whole character at once.
            let char_len = utf8_char_len(ch);
            if i + char_len <= bytes.len() {
                if let Ok(s) = std::str::from_utf8(&bytes[i..i + char_len]) {
                    current_text.push_str(s);
                } else {
                    // Invalid UTF-8 — push replacement character
                    current_text.push('\u{FFFD}');
                }
            } else {
                current_text.push('\u{FFFD}');
            }
            i += char_len;
        }
    }

    flush_text_token(&mut current_text, span, out);
}

/// Return the byte length of a UTF-8 character from its leading byte.
fn utf8_char_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead < 0xE0 {
        2
    } else if lead < 0xF0 {
        3
    } else {
        4
    }
}

fn flush_text_token(buf: &mut String, span: Span, out: &mut Vec<Spanned>) {
    if !buf.is_empty() {
        out.push(Spanned {
            kind: Token::Text(std::mem::take(buf)),
            span,
        });
    }
}

// ===========================================================================
// Line-level classification helpers
// ===========================================================================

fn try_heading(text: &str) -> Option<u8> {
    // CommonMark: only 0-3 leading spaces allowed before ATX heading.
    // 4+ spaces is an indented code block, not a heading.
    let indent = text.len() - text.trim_start().len();
    if indent >= 4 {
        return None;
    }
    let trimmed = text.trim_start();
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&hashes) {
        let rest = &trimmed[hashes..];
        if rest.is_empty() || rest.starts_with(' ') {
            return Some(hashes as u8);
        }
    }
    None
}

fn try_code_fence(trimmed: &str) -> Option<Token> {
    let fence_char = trimmed.chars().next()?;
    if fence_char != '`' && fence_char != '~' {
        return None;
    }
    let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
    if fence_len < 3 {
        return None;
    }
    let rest = trimmed[fence_len..].trim();
    if rest.is_empty() {
        Some(Token::FencedCodeClose {
            fence_char,
            fence_len,
        })
    } else {
        Some(Token::FencedCodeOpen {
            info: rest.to_string(),
            fence_char,
            fence_len,
        })
    }
}

fn try_html_line(trimmed: &str) -> Option<Token> {
    if !trimmed.starts_with('<') {
        return None;
    }
    let rest = &trimmed[1..];
    let closing = rest.starts_with('/');
    let tag_start = if closing { &rest[1..] } else { rest };
    let tag_name: String = tag_start
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if tag_name.is_empty() {
        return None;
    }
    if !closing {
        let after_name = &tag_start[tag_name.len()..];
        if after_name.starts_with("://") || after_name.starts_with('@') {
            return None;
        }
    }
    if closing {
        Some(Token::HtmlClose { tag: tag_name })
    } else {
        Some(Token::HtmlOpen { tag: tag_name })
    }
}

fn is_horizontal_rule(trimmed: &str) -> bool {
    if trimmed.len() < 3 {
        return false;
    }
    let first = trimmed.chars().next().unwrap();
    if first != '-' && first != '*' && first != '_' {
        return false;
    }
    trimmed.chars().all(|c| c == first || c == ' ')
}

fn try_callout(trimmed: &str) -> Option<(String, Option<String>)> {
    let rest = trimmed.strip_prefix('>')?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("[!")?;
    let end = rest.find(']')?;
    let kind = &rest[..end];
    if kind.is_empty() {
        return None;
    }
    let after_type = &rest[end + 1..];
    let metadata = if let Some(meta_rest) = after_type.trim_start().strip_prefix('[') {
        meta_rest.find(']').and_then(|meta_end| {
            let meta = meta_rest[..meta_end].trim();
            if meta.is_empty() {
                None
            } else {
                Some(meta.to_string())
            }
        })
    } else {
        None
    };
    Some((kind.to_lowercase(), metadata))
}

fn try_list_item(text: &str) -> Option<(usize, bool)> {
    let indent = text.len() - text.trim_start().len();
    let trimmed = text.trim_start();
    if (trimmed.starts_with("- ") || trimmed.starts_with("+ ")) && trimmed.len() > 2 {
        return Some((indent, false));
    }
    if indent > 0 && trimmed.starts_with("* ") && trimmed.len() > 2 {
        return Some((indent, false));
    }
    let digits_end = trimmed.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digits_end > 0 && digits_end < trimmed.len() {
        let after = &trimmed[digits_end..];
        if after.starts_with(". ") && after.len() > 2 {
            return Some((indent, true));
        }
    }
    None
}

fn try_footnote_def(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("[^")?;
    let end = rest.find("]:")?;
    let label = &rest[..end];
    if label.is_empty() || label.contains(' ') {
        return None;
    }
    Some(label.to_string())
}

/// Detect `[label]: url "optional title"` link reference definitions.
/// Returns the label if this line is a link definition (the rest is in the RawLine).
fn try_link_def(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix('[')?;
    // Must not start with ^ (that's a footnote)
    if rest.starts_with('^') {
        return None;
    }
    let end = rest.find("]:")?;
    let label = &rest[..end];
    if label.is_empty() {
        return None;
    }
    // After ]: there must be a space and then content
    let after = &rest[end + 2..];
    if after.is_empty() || (!after.starts_with(' ') && !after.starts_with('\t')) {
        return None;
    }
    Some(label.to_string())
}

// ===========================================================================
// Inline helpers
// ===========================================================================

fn peek(bytes: &[u8], i: usize) -> Option<u8> {
    bytes.get(i).copied()
}

fn scan_backtick_code(text: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut n = 0;
    while start + n < bytes.len() && bytes[start + n] == b'`' {
        n += 1;
    }
    if n == 0 {
        return None;
    }
    let content_start = start + n;
    if content_start >= bytes.len() {
        return None;
    }
    let mut i = content_start;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run_start = i;
            while i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            if i - run_start == n {
                let code = &text[content_start..run_start];
                let stripped = if code.len() >= 2
                    && code.starts_with(' ')
                    && code.ends_with(' ')
                    && !code.trim_matches(' ').is_empty()
                {
                    &code[1..code.len() - 1]
                } else {
                    code
                };
                return Some((stripped, i));
            }
        } else {
            i += 1;
        }
    }
    None
}

fn try_footnote_ref(text: &str, start: usize) -> Option<(String, usize)> {
    let rest = &text[start..];
    if !rest.starts_with("[^") {
        return None;
    }
    let after = &rest[2..];
    let end = after.find(']')?;
    let label = &after[..end];
    if label.is_empty() || label.contains(' ') {
        return None;
    }
    Some((label.to_string(), start + 2 + end + 1))
}

/// Try to parse a reference link at `start`:
///  - Full: `[display text][label]`
///  - Collapsed: `[label][]`
///  - Shortcut: `[label]` (not followed by `(` or `[`)
fn try_link_ref(text: &str, start: usize) -> Option<(Token, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(start).copied() != Some(b'[') {
        return None;
    }
    // Don't match footnote refs [^...]
    if bytes.get(start + 1).copied() == Some(b'^') {
        return None;
    }

    // Find closing ] for the first bracket group
    let mut pos = start + 1;
    let mut depth = 1i32;
    while pos < bytes.len() && depth > 0 {
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
            continue;
        }
        if bytes[pos] == b'[' {
            depth += 1;
        } else if bytes[pos] == b']' {
            depth -= 1;
        }
        if depth > 0 {
            pos += 1;
        }
    }
    if depth != 0 {
        return None;
    }
    let first_text = &text[start + 1..pos];
    if first_text.is_empty() {
        return None;
    }
    let after_first = pos + 1;

    // Full reference: [text][label]
    if after_first < bytes.len() && bytes[after_first] == b'[' {
        let label_start = after_first + 1;
        if let Some(close) = text[label_start..].find(']') {
            let label = &text[label_start..label_start + close];
            if label.is_empty() {
                // Collapsed: [label][]
                return Some((
                    Token::LinkRef {
                        text: first_text.to_string(),
                        label: first_text.to_string(),
                    },
                    label_start + close + 1,
                ));
            }
            return Some((
                Token::LinkRef {
                    text: first_text.to_string(),
                    label: label.to_string(),
                },
                label_start + close + 1,
            ));
        }
    }

    // Shortcut reference: [label] not followed by ( or [
    if after_first >= bytes.len() || (bytes[after_first] != b'(' && bytes[after_first] != b'[') {
        return Some((
            Token::LinkRef {
                text: first_text.to_string(),
                label: first_text.to_string(),
            },
            after_first,
        ));
    }

    None
}

fn try_link(text: &str, start: usize) -> Option<(Token, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(start).copied() != Some(b'[') {
        return None;
    }

    let mut depth = 0i32;
    let mut pos = start;
    let bracket_close;
    loop {
        if pos >= bytes.len() {
            return None;
        }
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
            continue;
        }
        if bytes[pos] == b'[' {
            depth += 1;
        } else if bytes[pos] == b']' {
            depth -= 1;
            if depth == 0 {
                bracket_close = pos;
                break;
            }
        }
        pos += 1;
    }

    let link_text = &text[start + 1..bracket_close];
    pos = bracket_close + 1;
    if pos >= bytes.len() || bytes[pos] != b'(' {
        return None;
    }

    let mut paren_depth = 0i32;
    let paren_close;
    loop {
        if pos >= bytes.len() {
            return None;
        }
        if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
            pos += 2;
            continue;
        }
        if bytes[pos] == b'(' {
            paren_depth += 1;
        } else if bytes[pos] == b')' {
            paren_depth -= 1;
            if paren_depth == 0 {
                paren_close = pos;
                break;
            }
        }
        pos += 1;
    }

    let paren_inner = text[bracket_close + 2..paren_close].trim();
    let (url, title, meta) = parse_link_paren(paren_inner);

    Some((
        Token::Link {
            text: link_text.to_string(),
            url,
            title,
            meta,
        },
        paren_close + 1,
    ))
}

fn try_autolink(text: &str, start: usize) -> Option<(Token, usize)> {
    let rest = &text[start + 1..];
    let end = rest.find('>')?;
    let content = &rest[..end];
    if content.is_empty()
        || content.contains(' ')
        || content.contains('\n')
        || content.contains('<')
    {
        return None;
    }
    let is_uri = content.contains("://");
    let is_email =
        !is_uri && content.contains('@') && !content.starts_with('@') && !content.ends_with('@');
    if !is_uri && !is_email {
        return None;
    }
    let url = if is_email {
        format!("mailto:{content}")
    } else {
        content.to_string()
    };
    Some((
        Token::Link {
            text: content.to_string(),
            url,
            title: None,
            meta: None,
        },
        start + 1 + end + 1,
    ))
}

fn try_image(text: &str, start: usize) -> Option<(Token, usize)> {
    let (link_tok, end) = try_link(text, start + 1)?;
    match link_tok {
        Token::Link {
            text: alt,
            url,
            title,
            ..
        } => Some((Token::Image { alt, url, title }, end)),
        _ => None,
    }
}

fn parse_link_paren(inner: &str) -> (String, Option<String>, Option<String>) {
    let inner = inner.trim();
    let url_end = inner.find([' ', '"', '[']).unwrap_or(inner.len());
    let url = inner[..url_end].to_string();
    let rest = inner[url_end..].trim();

    if rest.is_empty() {
        return (url, None, None);
    }

    let (title, rest) = if let Some(after_quote) = rest.strip_prefix('"') {
        if let Some(end) = after_quote.find('"') {
            (
                Some(after_quote[..end].to_string()),
                after_quote[end + 1..].trim(),
            )
        } else {
            (None, rest)
        }
    } else {
        (None, rest)
    };

    let meta = if rest.starts_with('[') {
        rest.find(']').map(|end| rest[1..end].to_string())
    } else {
        None
    };

    (url, title, meta)
}

fn tokenize_tag(text: &str, name_start: usize, _span: Span) -> (Token, Option<Token>, usize) {
    let bytes = text.as_bytes();
    let mut pos = name_start;

    while pos < bytes.len() {
        let c = bytes[pos] as char;
        if c.is_alphanumeric() || c == '-' || c == '_' {
            pos += 1;
        } else {
            break;
        }
    }

    let name = &text[name_start..pos];
    let tok = match Keyword::from_str(name) {
        Some(kw) => Token::Tag(kw),
        None => Token::UnknownTag {
            name: name.to_string(),
        },
    };

    let mut arg = String::new();
    if pos < bytes.len() && bytes[pos] == b' ' {
        pos += 1;
        while pos < bytes.len() {
            let c = bytes[pos];
            if c == b'#'
                && let Some(next) = peek(bytes, pos + 1)
                && ((next as char).is_alphanumeric() || next == b'_')
            {
                break;
            }
            if c == b'\\' {
                if peek(bytes, pos + 1) == Some(b'#') {
                    arg.push('#');
                    pos += 2;
                    continue;
                }
                arg.push('\\');
                pos += 1;
                continue;
            }
            arg.push(c as char);
            pos += 1;
        }
    }

    let arg_tok = {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Token::TagArg(trimmed.to_string()))
        }
    };

    (tok, arg_tok, pos)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn block_tokens(source: &str) -> Vec<Token> {
        Lexer::new(source)
            .tokens
            .iter()
            .map(|s| s.kind.clone())
            .filter(|t| !matches!(t, Token::Newline | Token::Eof))
            .collect()
    }

    fn inline_tokens(text: &str) -> Vec<Token> {
        tokenize_inline(text, Span::empty(1, 1))
            .into_iter()
            .map(|s| s.kind)
            .collect()
    }

    // Block-level tests

    #[test]
    fn test_heading() {
        let tokens = block_tokens("# Hello world");
        assert!(matches!(&tokens[0], Token::Heading { level: 1 }));
        assert!(matches!(&tokens[1], Token::RawLine(_)));
    }

    #[test]
    fn test_code_fence() {
        let tokens = block_tokens("```rust #tangle file=main.rs");
        assert!(
            matches!(&tokens[0], Token::FencedCodeOpen { info, .. } if info == "rust #tangle file=main.rs")
        );
    }

    #[test]
    fn test_block_tag() {
        let tokens = block_tokens("#deadline 2026-04-10");
        assert!(matches!(&tokens[0], Token::Tag(Keyword::Deadline)));
        assert!(matches!(&tokens[1], Token::TagArg(a) if a == "2026-04-10"));
    }

    #[test]
    fn test_list_item() {
        let tokens = block_tokens("- [ ] Task item");
        assert!(matches!(
            &tokens[0],
            Token::ListMarker {
                ordered: false,
                indent: 0
            }
        ));
        assert!(matches!(&tokens[1], Token::RawLine(_)));
    }

    #[test]
    fn test_properties() {
        let tokens = block_tokens("#properties");
        assert!(matches!(&tokens[0], Token::PropertiesOpen));
    }

    #[test]
    fn test_comment() {
        let tokens = block_tokens("// this is a comment");
        assert!(matches!(&tokens[0], Token::LineComment));
        assert!(matches!(&tokens[1], Token::RawLine(_)));
    }

    #[test]
    fn test_horizontal_rule() {
        let tokens = block_tokens("***");
        assert!(matches!(&tokens[0], Token::HorizontalRule));
    }

    #[test]
    fn test_frontmatter() {
        let tokens = block_tokens("---");
        assert!(matches!(&tokens[0], Token::FrontmatterDelim));
    }

    #[test]
    fn test_blank_line() {
        let tokens = block_tokens("");
        assert!(matches!(&tokens[0], Token::BlankLine));
    }

    #[test]
    fn test_unknown_tag() {
        let tokens = block_tokens("#custom value");
        assert!(matches!(&tokens[0], Token::UnknownTag { name } if name == "custom"));
        assert!(matches!(&tokens[1], Token::TagArg(a) if a == "value"));
    }

    // Inline tokenizer tests

    #[test]
    fn test_inline_tag() {
        let tokens = inline_tokens("some text #todo fix this");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "some text "));
        assert!(matches!(&tokens[1], Token::Tag(Keyword::Todo)));
        assert!(matches!(&tokens[2], Token::TagArg(a) if a == "fix this"));
    }

    #[test]
    fn test_inline_bold_italic() {
        let tokens = inline_tokens("**bold** and *italic*");
        assert!(matches!(&tokens[0], Token::BoldDelim));
        assert!(matches!(&tokens[1], Token::Text(t) if t == "bold"));
        assert!(matches!(&tokens[2], Token::BoldDelim));
        assert!(matches!(&tokens[3], Token::Text(t) if t == " and "));
        assert!(matches!(&tokens[4], Token::ItalicDelim));
        assert!(matches!(&tokens[5], Token::Text(t) if t == "italic"));
        assert!(matches!(&tokens[6], Token::ItalicDelim));
    }

    #[test]
    fn test_inline_code() {
        let tokens = inline_tokens("use `println!` here");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "use "));
        assert!(matches!(&tokens[1], Token::InlineCode(c) if c == "println!"));
        assert!(matches!(&tokens[2], Token::Text(t) if t == " here"));
    }

    #[test]
    fn test_inline_link() {
        let tokens = inline_tokens("[click](https://example.com)");
        assert!(
            matches!(&tokens[0], Token::Link { text, url, .. } if text == "click" && url == "https://example.com")
        );
    }

    #[test]
    fn test_inline_footnote_ref() {
        let tokens = inline_tokens("text[^1] more");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "text"));
        assert!(matches!(&tokens[1], Token::FootnoteRef { label } if label == "1"));
        assert!(matches!(&tokens[2], Token::Text(t) if t == " more"));
    }

    #[test]
    fn test_inline_escaped_hash() {
        let tokens = inline_tokens(r"price \#100");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "price #100"));
    }

    // Double-backtick code spans
    #[test]
    fn test_inline_double_backtick_code() {
        let tokens = inline_tokens("use ``code with `backtick` inside`` here");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "use "));
        assert!(matches!(&tokens[1], Token::InlineCode(c) if c == "code with `backtick` inside"));
        assert!(matches!(&tokens[2], Token::Text(t) if t == " here"));
    }
    #[test]
    fn test_inline_triple_backtick_code() {
        let tokens = inline_tokens("``` `` ```");
        assert!(matches!(&tokens[0], Token::InlineCode(c) if c == "``"));
    }
    #[test]
    fn test_inline_backtick_space_stripping() {
        let tokens = inline_tokens("`` `foo` ``");
        assert!(matches!(&tokens[0], Token::InlineCode(c) if c == "`foo`"));
    }
    #[test]
    fn test_inline_backtick_no_strip_all_spaces() {
        let tokens = inline_tokens("``  ``");
        assert!(matches!(&tokens[0], Token::InlineCode(c) if c == "  "));
    }
    // Autolinks
    #[test]
    fn test_inline_autolink_url() {
        let tokens = inline_tokens("visit <https://example.com> now");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "visit "));
        assert!(
            matches!(&tokens[1], Token::Link { text, url, .. } if text == "https://example.com" && url == "https://example.com")
        );
        assert!(matches!(&tokens[2], Token::Text(t) if t == " now"));
    }
    #[test]
    fn test_inline_autolink_email() {
        let tokens = inline_tokens("email <user@example.com> please");
        assert!(
            matches!(&tokens[1], Token::Link { text, url, .. } if text == "user@example.com" && url == "mailto:user@example.com")
        );
    }
    #[test]
    fn test_html_tag_not_autolink() {
        let tokens = inline_tokens("some <b>bold</b> text");
        let has_link = tokens.iter().any(|t| matches!(t, Token::Link { .. }));
        assert!(!has_link);
    }
    // Images
    #[test]
    fn test_inline_image() {
        let tokens = inline_tokens("![alt text](image.png)");
        assert!(
            matches!(&tokens[0], Token::Image { alt, url, .. } if alt == "alt text" && url == "image.png")
        );
    }
    #[test]
    fn test_inline_image_with_title() {
        let tokens = inline_tokens(r#"![photo](pic.jpg "My Photo")"#);
        assert!(matches!(&tokens[0], Token::Image { alt, url, title, .. }
            if alt == "photo" && url == "pic.jpg" && title.as_deref() == Some("My Photo")));
    }

    // Link reference definitions (block-level)
    #[test]
    fn test_block_link_def() {
        let tokens = block_tokens("[foo]: /url");
        assert!(matches!(&tokens[0], Token::LinkDefStart { label } if label == "foo"));
        assert!(matches!(&tokens[1], Token::RawLine(_)));
    }

    #[test]
    fn test_block_link_def_not_footnote() {
        let tokens = block_tokens("[^note]: footnote");
        assert!(matches!(&tokens[0], Token::FootnoteDefStart { label } if label == "note"));
    }

    // Link references (inline)
    #[test]
    fn test_inline_link_ref_shortcut() {
        let tokens = inline_tokens("see [foo] here");
        assert!(matches!(&tokens[0], Token::Text(t) if t == "see "));
        assert!(
            matches!(&tokens[1], Token::LinkRef { text, label } if text == "foo" && label == "foo")
        );
        assert!(matches!(&tokens[2], Token::Text(t) if t == " here"));
    }

    #[test]
    fn test_inline_link_ref_full() {
        let tokens = inline_tokens("[click here][foo]");
        assert!(
            matches!(&tokens[0], Token::LinkRef { text, label } if text == "click here" && label == "foo")
        );
    }

    #[test]
    fn test_inline_link_ref_collapsed() {
        let tokens = inline_tokens("[foo][]");
        assert!(
            matches!(&tokens[0], Token::LinkRef { text, label } if text == "foo" && label == "foo")
        );
    }

    #[test]
    fn test_inline_link_preferred_over_ref() {
        // [text](url) should be a Link, not a LinkRef
        let tokens = inline_tokens("[foo](https://example.com)");
        assert!(matches!(&tokens[0], Token::Link { text, .. } if text == "foo"));
    }

    // Setext headings (block-level lookahead)
    #[test]
    fn test_setext_h1() {
        let tokens = block_tokens("Heading\n===");
        assert!(matches!(&tokens[0], Token::Heading { level: 1 }));
        assert!(matches!(&tokens[1], Token::RawLine(t) if t == "Heading"));
    }

    #[test]
    fn test_setext_h2() {
        let tokens = block_tokens("Heading\n---");
        assert!(matches!(&tokens[0], Token::Heading { level: 2 }));
        assert!(matches!(&tokens[1], Token::RawLine(t) if t == "Heading"));
    }

    #[test]
    fn test_setext_single_char() {
        let tokens = block_tokens("Heading\n=");
        assert!(matches!(&tokens[0], Token::Heading { level: 1 }));
    }

    #[test]
    fn test_setext_not_in_frontmatter() {
        // Inside frontmatter, --- should close frontmatter, not create a setext heading
        let tokens = block_tokens("---\ntitle: Test\n---");
        assert!(matches!(&tokens[0], Token::FrontmatterDelim));
        // title: Test should be raw content, not a heading
        let has_heading = tokens.iter().any(|t| matches!(t, Token::Heading { .. }));
        assert!(
            !has_heading,
            "should not produce heading inside frontmatter"
        );
    }

    #[test]
    fn test_setext_with_leading_spaces() {
        let tokens = block_tokens("Heading\n   ===");
        assert!(matches!(&tokens[0], Token::Heading { level: 1 }));
    }

    #[test]
    fn test_setext_4_spaces_not_valid() {
        // 4+ leading spaces on underline → not a setext heading
        let tokens = block_tokens("Heading\n    ===");
        let has_heading = tokens.iter().any(|t| matches!(t, Token::Heading { .. }));
        assert!(
            !has_heading,
            "4-space indented underline should not be setext"
        );
    }

    // Indented code blocks
    #[test]
    fn test_indented_code_line() {
        let tokens = block_tokens("    code here");
        assert!(matches!(&tokens[0], Token::IndentedCodeLine));
        assert!(matches!(&tokens[1], Token::RawLine(t) if t == "    code here"));
    }

    #[test]
    fn test_tab_indented_code_line() {
        let tokens = block_tokens("\tcode here");
        assert!(matches!(&tokens[0], Token::IndentedCodeLine));
    }

    #[test]
    fn test_3_spaces_not_code() {
        let tokens = block_tokens("   not code");
        assert!(!tokens.iter().any(|t| matches!(t, Token::IndentedCodeLine)));
    }

    #[test]
    fn test_4_spaces_before_heading_is_code_not_heading() {
        let tokens = block_tokens("    # not a heading");
        assert!(matches!(&tokens[0], Token::IndentedCodeLine));
        assert!(!tokens.iter().any(|t| matches!(t, Token::Heading { .. })));
    }
}
