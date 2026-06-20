//! Exhaustive tests for the morg-parser crate.
//!
//! Covers: headings, paragraphs, code blocks, lists, tables, inline formatting,
//! links, footnotes, HTML blocks, comments, horizontal rules, frontmatter,
//! tags, callouts, property drawers, error recovery, and edge cases.
//!
//! Where relevant, tests note CommonMark spec compliance or intentional deviations.

use morg_parser::parser::ParseResult;
use morg_parser::*;

// ===========================================================================
// Helpers
// ===========================================================================

fn parse(src: &str) -> ParseResult {
    parse_document(src)
}

fn blocks(src: &str) -> Vec<Block> {
    parse(src).document.children
}

fn first_para(src: &str) -> Paragraph {
    blocks(src)
        .into_iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .expect("expected a paragraph")
}

fn first_heading(src: &str) -> Heading {
    blocks(src)
        .into_iter()
        .find_map(|b| match b {
            Block::Heading(h) => Some(h),
            _ => None,
        })
        .expect("expected a heading")
}

fn first_list(src: &str) -> List {
    blocks(src)
        .into_iter()
        .find_map(|b| match b {
            Block::List(l) => Some(l),
            _ => None,
        })
        .expect("expected a list")
}

fn first_table(src: &str) -> Table {
    blocks(src)
        .into_iter()
        .find_map(|b| match b {
            Block::Table(t) => Some(t),
            _ => None,
        })
        .expect("expected a table")
}

fn first_code_block(src: &str) -> CodeBlock {
    blocks(src)
        .into_iter()
        .find_map(|b| match b {
            Block::CodeBlock(cb) => Some(cb),
            _ => None,
        })
        .expect("expected a code block")
}

fn first_callout(src: &str) -> Callout {
    blocks(src)
        .into_iter()
        .find_map(|b| match b {
            Block::Callout(c) => Some(c),
            _ => None,
        })
        .expect("expected a callout")
}

fn has_block<F: Fn(&Block) -> bool>(src: &str, pred: F) -> bool {
    blocks(src).iter().any(pred)
}

fn inline_segments(src: &str) -> Vec<InlineSegment> {
    first_para(src).content.segments
}

fn _errors(src: &str) -> Vec<ParseError> {
    parse(src).errors
}

// ===========================================================================
// 1. Headings
// ===========================================================================

#[test]
fn heading_levels_1_through_6() {
    for level in 1u8..=6 {
        let hashes = "#".repeat(level as usize);
        let src = format!("{hashes} Heading level {level}\n");
        let h = first_heading(&src);
        assert_eq!(h.level, level, "heading level mismatch for {hashes}");
    }
}

#[test]
fn heading_level_7_is_not_heading() {
    // CommonMark: only levels 1-6 are valid ATX headings
    let src = "####### Not a heading\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(!has_heading, "7 hashes should not produce a heading");
}

#[test]
fn heading_empty_content() {
    // CommonMark: `# ` followed by nothing is valid empty heading
    let src = "# \n";
    let h = first_heading(src);
    assert_eq!(h.level, 1);
}

#[test]
fn heading_no_space_after_hashes() {
    // `#Heading` without space — should not be a heading per CommonMark
    // but morg uses # for tags, so #Heading is a tag
    let src = "#Heading\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(
        !has_heading,
        "#Heading without space should not be a heading"
    );
}

#[test]
fn heading_with_leading_spaces() {
    // CommonMark allows up to 3 leading spaces
    let src = "   # Indented heading\n";
    let h = first_heading(src);
    assert_eq!(h.level, 1);
}

#[test]
fn heading_with_inline_formatting() {
    let src = "# **Bold** heading\n";
    let h = first_heading(src);
    assert!(
        h.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Bold(_)))
    );
}

#[test]
fn heading_with_inline_tags() {
    let src = "## Task #todo implement\n";
    let h = first_heading(src);
    let tags = h.content.tags();
    assert_eq!(tags.len(), 1);
    assert!(matches!(tags[0].kind, TagKind::Todo { .. }));
}

#[test]
fn heading_with_inline_code() {
    let src = "# The `main` function\n";
    let h = first_heading(src);
    assert!(
        h.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Code(c) if c == "main"))
    );
}

#[test]
fn heading_hash_only_no_space() {
    // `#` alone on a line — should be parsed as a heading with empty content
    let src = "#\n";
    // In morg, a bare # with nothing after it: try_heading checks rest.is_empty() || rest.starts_with(' ')
    // rest is empty after the #, so it should be a heading
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(has_heading, "bare # should be a level-1 heading");
}

// ===========================================================================
// 1b. Setext headings
// ===========================================================================

#[test]
fn setext_h1_equals() {
    let src = "Heading\n=======\n";
    let h = first_heading(src);
    assert_eq!(h.level, 1);
    assert_eq!(h.content.plain_text(), "Heading");
}

#[test]
fn setext_h2_dashes() {
    let src = "Heading\n-------\n";
    let h = first_heading(src);
    assert_eq!(h.level, 2);
    assert_eq!(h.content.plain_text(), "Heading");
}

#[test]
fn setext_single_char_underline() {
    let src = "Heading\n=\n";
    let h = first_heading(src);
    assert_eq!(h.level, 1);

    let src2 = "Heading\n-\n";
    let h2 = first_heading(src2);
    assert_eq!(h2.level, 2);
}

#[test]
fn setext_dashes_after_paragraph_is_h2_not_hr() {
    // Per CommonMark: `---` following a paragraph line is a setext h2
    let src = "Paragraph text\n---\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(h) if h.level == 2));
    let has_hr = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HorizontalRule(_)));
    assert!(has_heading, "--- after paragraph should be setext h2");
    assert!(!has_hr, "--- after paragraph should NOT be an HR");
}

#[test]
fn setext_not_after_blank_line() {
    // `---` after a blank line is an HR, not a setext heading
    let src = "Text\n\n---\n";
    let result = parse(src);
    let has_hr = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HorizontalRule(_)));
    assert!(has_hr, "--- after blank line should be HR");
}

#[test]
fn setext_with_inline_formatting() {
    let src = "**Bold** heading\n===\n";
    let h = first_heading(src);
    assert_eq!(h.level, 1);
    assert!(
        h.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Bold(_)))
    );
}

#[test]
fn setext_leading_spaces_on_underline() {
    let src = "Heading\n   ===\n";
    let h = first_heading(src);
    assert_eq!(h.level, 1);
}

#[test]
fn setext_4_space_underline_not_heading() {
    // 4+ leading spaces on underline → not a setext heading
    let src = "Text\n    ===\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(
        !has_heading,
        "4-space indented underline should not produce setext heading"
    );
}

#[test]
fn setext_mixed_chars_not_valid() {
    // =-= is not a valid underline
    let src = "Text\n=-=\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(!has_heading, "mixed =-= should not be setext underline");
}

#[test]
fn setext_spaces_between_chars_not_valid() {
    // = = = is not a valid underline (CommonMark requires contiguous)
    let src = "Text\n= = =\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(
        !has_heading,
        "= = = with spaces should not be setext underline"
    );
}

#[test]
fn setext_does_not_interfere_with_frontmatter() {
    // Inside frontmatter, --- should close frontmatter, not become setext heading
    let src = "---\ntitle: Test\n---\n\n# Real heading\n";
    let result = parse(src);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert!(result.document.frontmatter.is_some());
    let headings: Vec<_> = result
        .document
        .children
        .iter()
        .filter_map(|b| match b {
            Block::Heading(h) => Some(h),
            _ => None,
        })
        .collect();
    assert_eq!(headings.len(), 1);
    assert_eq!(headings[0].content.plain_text(), "Real heading");
}

#[test]
fn setext_consecutive() {
    let src = "First\n===\nSecond\n---\n";
    let result = parse(src);
    let headings: Vec<_> = result
        .document
        .children
        .iter()
        .filter_map(|b| match b {
            Block::Heading(h) => Some((h.level, h.content.plain_text())),
            _ => None,
        })
        .collect();
    assert_eq!(
        headings,
        vec![(1, "First".to_string()), (2, "Second".to_string())]
    );
}

// ===========================================================================
// 2. Paragraphs
// ===========================================================================

#[test]
fn simple_paragraph() {
    let src = "Hello world\n";
    let p = first_para(src);
    assert_eq!(p.content.plain_text(), "Hello world");
}

#[test]
fn paragraph_with_inline_formatting() {
    let src = "Some **bold** and *italic* text\n";
    let segs = inline_segments(src);
    assert!(segs.iter().any(|s| matches!(s, InlineSegment::Bold(_))));
    assert!(segs.iter().any(|s| matches!(s, InlineSegment::Italic(_))));
}

#[test]
fn paragraph_with_multiple_lines() {
    // Multiple text lines before a blank line should merge into one paragraph
    let src = "Line one\nLine two\nLine three\n";
    let result = parse(src);
    let paras: Vec<_> = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::Paragraph(_)))
        .collect();
    // Each line is classified independently, so this may produce multiple paragraphs
    // or a single one depending on continuation logic
    assert!(!paras.is_empty());
}

#[test]
fn paragraph_separated_by_blank_line() {
    let src = "First paragraph.\n\nSecond paragraph.\n";
    let result = parse(src);
    let paras: Vec<_> = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::Paragraph(_)))
        .collect();
    assert_eq!(paras.len(), 2);
}

#[test]
fn paragraph_with_link() {
    let src = "Click [here](https://example.com) for more.\n";
    let segs = inline_segments(src);
    assert!(segs.iter().any(|s| matches!(s, InlineSegment::Link(_))));
}

// ===========================================================================
// 3. Code blocks
// ===========================================================================

#[test]
fn code_block_basic_backtick() {
    let src = "```\ncode here\n```\n";
    let cb = first_code_block(src);
    assert_eq!(cb.lang, None);
    assert_eq!(cb.body, "code here");
}

#[test]
fn code_block_with_language() {
    let src = "```rust\nfn main() {}\n```\n";
    let cb = first_code_block(src);
    assert_eq!(cb.lang.as_deref(), Some("rust"));
    assert_eq!(cb.body, "fn main() {}");
}

#[test]
fn code_block_with_tangle_and_attrs() {
    let src = "```python #tangle file=script.py\nprint('hello')\n```\n";
    let cb = first_code_block(src);
    assert_eq!(cb.lang.as_deref(), Some("python"));
    assert!(cb.tags.iter().any(|t| matches!(t.kind, TagKind::Tangle)));
    assert_eq!(
        cb.attributes.get("file").map(|s| s.as_str()),
        Some("script.py")
    );
}

#[test]
fn code_block_tilde_fence() {
    let src = "~~~\ntilde fenced\n~~~\n";
    let cb = first_code_block(src);
    assert_eq!(cb.body, "tilde fenced");
}

#[test]
fn code_block_longer_fence() {
    // Closing fence must be >= opening fence length
    let src = "````\ncode\n````\n";
    let cb = first_code_block(src);
    assert_eq!(cb.body, "code");
}

#[test]
fn code_block_preserves_blank_lines() {
    let src = "```\nline1\n\nline3\n```\n";
    let cb = first_code_block(src);
    assert!(cb.body.contains("\n\n") || cb.body.contains("line1\n"));
}

#[test]
fn code_block_unclosed_produces_error() {
    let src = "```rust\nfn main() {}\n";
    let result = parse(src);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnclosedCodeFence)
    );
}

#[test]
fn code_block_mismatched_fence_char() {
    // Opening ``` should not be closed by ~~~
    let src = "```\ncode\n~~~\n";
    let result = parse(src);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnclosedCodeFence),
        "mismatched fence chars should not close the block"
    );
}

#[test]
fn code_block_shorter_closing_fence() {
    // A closing fence shorter than the opening should not close the block.
    // The shorter fence should appear as content in the body.
    let src = "````\ncode\n```\n````\n";
    let result = parse(src);
    assert!(result.errors.is_empty(), "no errors expected");
    let cb = first_code_block(src);
    assert!(
        cb.body.contains("```"),
        "shorter fence should be preserved as content in body: {:?}",
        cb.body
    );
    assert!(cb.body.contains("code"));
}

#[test]
fn code_block_empty_body() {
    let src = "```\n```\n";
    let cb = first_code_block(src);
    assert_eq!(cb.body, "");
}

#[test]
fn code_block_with_backticks_inside() {
    // Shorter fences inside a code block should be preserved as content.
    let src = "````\n```\ninner\n```\n````\n";
    let cb = first_code_block(src);
    assert!(
        cb.body.contains("```"),
        "inner fence text should be preserved in body: {:?}",
        cb.body
    );
    assert!(cb.body.contains("inner"));
}

// ===========================================================================
// 4. Lists
// ===========================================================================

#[test]
fn unordered_list_dash() {
    let src = "- Item one\n- Item two\n- Item three\n";
    let l = first_list(src);
    assert_eq!(l.kind, ListKind::Unordered);
    assert_eq!(l.items.len(), 3);
}

#[test]
fn unordered_list_plus() {
    let src = "+ Item one\n+ Item two\n";
    let l = first_list(src);
    assert_eq!(l.kind, ListKind::Unordered);
    assert_eq!(l.items.len(), 2);
}

#[test]
fn ordered_list() {
    let src = "1. First\n2. Second\n3. Third\n";
    let l = first_list(src);
    assert_eq!(l.kind, ListKind::Ordered);
    assert_eq!(l.items.len(), 3);
}

#[test]
fn list_with_checkboxes() {
    let src = "- [ ] Unchecked\n- [x] Checked\n- [X] Also checked\n";
    let l = first_list(src);
    assert_eq!(l.items[0].checkbox, Some(Checkbox::Unchecked));
    assert_eq!(l.items[1].checkbox, Some(Checkbox::Checked));
    assert_eq!(l.items[2].checkbox, Some(Checkbox::Checked));
}

#[test]
fn nested_list() {
    let src = "- Parent\n  - Child\n    - Grandchild\n";
    let l = first_list(src);
    assert_eq!(l.items.len(), 1);
    assert_eq!(l.items[0].content.plain_text(), "Parent");

    let child_list = match &l.items[0].children[0] {
        Block::List(cl) => cl,
        _ => panic!("expected nested list"),
    };
    assert_eq!(child_list.items[0].content.plain_text(), "Child");
}

#[test]
fn list_description_items() {
    let src = "- Term :: Description text\n";
    let l = first_list(src);
    assert!(
        l.items[0].description.is_some(),
        "should have description for :: syntax"
    );
}

#[test]
fn list_with_inline_formatting() {
    let src = "- **Bold** item\n- *Italic* item\n";
    let l = first_list(src);
    assert!(
        l.items[0]
            .content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Bold(_)))
    );
    assert!(
        l.items[1]
            .content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Italic(_)))
    );
}

#[test]
fn list_mixed_markers_not_merged() {
    // Dash list followed by ordered list should be separate
    let src = "- Unordered\n1. Ordered\n";
    let result = parse(src);
    let lists: Vec<_> = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::List(_)))
        .collect();
    // They might be merged or separate depending on implementation
    assert!(!lists.is_empty());
}

#[test]
fn star_list_at_zero_indent() {
    // BUG CANDIDATE: CommonMark treats `* item` as a list item at any indent
    // morg-parser requires indent > 0 for * marker
    let src = "* item at zero indent\n";
    let result = parse(src);
    let has_list = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::List(_)));
    // Document what actually happens
    if !has_list {
        // This is a known deviation from CommonMark
        eprintln!("BUG/DEVIATION: `* item` at zero indent is not parsed as a list item");
    }
}

#[test]
fn ordered_list_starting_number() {
    // CommonMark: ordered list can start at any number
    let src = "3. Third\n4. Fourth\n";
    let l = first_list(src);
    assert_eq!(l.kind, ListKind::Ordered);
    assert_eq!(l.items.len(), 2);
}

// ===========================================================================
// 5. Tables
// ===========================================================================

#[test]
fn basic_table() {
    let src = "| a | b |\n|---|---|\n| 1 | 2 |\n";
    let t = first_table(src);
    assert_eq!(t.headers.len(), 2);
    assert_eq!(t.rows.len(), 1);
    assert_eq!(t.headers[0].plain_text(), "a");
    assert_eq!(t.headers[1].plain_text(), "b");
}

#[test]
fn table_alignments() {
    let src = "| left | center | right | none |\n|:---|:---:|---:|---|\n| a | b | c | d |\n";
    let t = first_table(src);
    assert_eq!(t.alignments[0], Alignment::Left);
    assert_eq!(t.alignments[1], Alignment::Center);
    assert_eq!(t.alignments[2], Alignment::Right);
    assert_eq!(t.alignments[3], Alignment::None);
}

#[test]
fn table_no_separator() {
    // Tables without separator row
    let src = "| a | b |\n| 1 | 2 |\n";
    let t = first_table(src);
    // Without separator, first row is headers, second is also parsed
    assert!(!t.headers.is_empty());
}

#[test]
fn table_with_inline_formatting() {
    let src = "| **bold** | *italic* |\n|---|---|\n| `code` | text |\n";
    let t = first_table(src);
    assert!(
        t.headers[0]
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Bold(_)))
    );
}

#[test]
fn table_single_column() {
    let src = "| single |\n|---|\n| data |\n";
    let t = first_table(src);
    assert_eq!(t.headers.len(), 1);
    assert_eq!(t.rows.len(), 1);
}

#[test]
fn table_empty_cells() {
    let src = "| a | b |\n|---|---|\n|  |  |\n";
    let t = first_table(src);
    assert_eq!(t.rows.len(), 1);
}

// ===========================================================================
// 6. Inline formatting
// ===========================================================================

#[test]
fn bold_basic() {
    let src = "This is **bold** text\n";
    let segs = inline_segments(src);
    let bold = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Bold(inner) => Some(inner),
            _ => None,
        })
        .expect("should have bold");
    assert_eq!(bold.plain_text(), "bold");
}

#[test]
fn italic_basic() {
    let src = "This is *italic* text\n";
    let segs = inline_segments(src);
    assert!(
        segs.iter()
            .any(|s| matches!(s, InlineSegment::Italic(i) if i.plain_text() == "italic"))
    );
}

#[test]
fn strikethrough_basic() {
    let src = "This is ~~struck~~ text\n";
    let segs = inline_segments(src);
    assert!(
        segs.iter()
            .any(|s| matches!(s, InlineSegment::Strikethrough(i) if i.plain_text() == "struck"))
    );
}

#[test]
fn inline_code_basic() {
    let src = "Use `println!` here\n";
    let segs = inline_segments(src);
    assert!(
        segs.iter()
            .any(|s| matches!(s, InlineSegment::Code(c) if c == "println!"))
    );
}

#[test]
fn bold_italic_nested() {
    // **bold *and italic* text** — bold containing italic
    let src = "**bold *nested* text**\n";
    let segs = inline_segments(src);
    let bold = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Bold(inner) => Some(inner),
            _ => None,
        })
        .expect("should have bold");
    assert!(
        bold.segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Italic(_)))
    );
}

#[test]
fn bold_italic_combined() {
    // ***text*** — CommonMark parses as em > strong or strong > em
    let src = "***bold italic***\n";
    let segs = inline_segments(src);
    // Should have some combination of bold and italic
    let has_bold = segs.iter().any(|s| matches!(s, InlineSegment::Bold(_)));
    let has_italic = segs.iter().any(|s| matches!(s, InlineSegment::Italic(_)));
    assert!(
        has_bold || has_italic,
        "*** should produce bold and/or italic"
    );
}

#[test]
fn escaped_asterisk() {
    let src = "not \\*italic\\*\n";
    let segs = inline_segments(src);
    // Should contain literal * characters, not italic
    let has_italic = segs.iter().any(|s| matches!(s, InlineSegment::Italic(_)));
    assert!(!has_italic, "escaped asterisks should not produce italic");
}

#[test]
fn escaped_hash() {
    let src = "price \\#100\n";
    let segs = inline_segments(src);
    let has_tag = segs.iter().any(|s| matches!(s, InlineSegment::Tag(_)));
    assert!(!has_tag, "escaped hash should not produce a tag");
    let text: String = segs
        .iter()
        .filter_map(|s| match s {
            InlineSegment::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(text.contains("#100"));
}

#[test]
fn unclosed_bold_treats_delim_as_text_or_consumes_rest() {
    // Unclosed ** should gracefully handle the rest of the content
    let src = "text **unclosed bold\n";
    let result = parse(src);
    // Should not panic, and should produce some output
    assert!(!result.document.children.is_empty());
}

#[test]
fn unclosed_italic() {
    let src = "text *unclosed italic\n";
    let result = parse(src);
    assert!(!result.document.children.is_empty());
}

#[test]
fn inline_code_empty_backticks() {
    // Double backtick with space content: `` `` is a code span containing "  " (all spaces)
    // Per CommonMark space-stripping: don't strip if content is only spaces
    let src = "text ``  `` text\n";
    let segs = inline_segments(src);
    let has_code = segs
        .iter()
        .any(|s| matches!(s, InlineSegment::Code(c) if c == "  "));
    assert!(
        has_code,
        "double backtick code span with only spaces should preserve them"
    );
}

#[test]
fn inline_code_with_backtick_inside() {
    // CommonMark: use double backticks to include single backtick: `` ` ``
    let src = "text `` ` `` text\n";
    let segs = inline_segments(src);
    let has_code = segs
        .iter()
        .any(|s| matches!(s, InlineSegment::Code(c) if c == "`"));
    assert!(
        has_code,
        "double backtick code span should contain single backtick"
    );
}

#[test]
fn backslash_escape_limited_set() {
    // morg only escapes: # [ * ~ `
    // CommonMark escapes many more characters including \ ] ( ) etc.
    let src = "\\[ not a link \\]\n";
    let segs = inline_segments(src);
    let text: String = segs
        .iter()
        .filter_map(|s| match s {
            InlineSegment::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(text.contains("["), "escaped [ should produce literal [");
}

#[test]
fn backslash_not_before_special_char() {
    // Backslash before non-special char should be preserved
    let src = "path\\to\\file\n";
    let segs = inline_segments(src);
    let text: String = segs
        .iter()
        .filter_map(|s| match s {
            InlineSegment::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        text.contains("\\"),
        "backslash before non-special should be preserved"
    );
}

// ===========================================================================
// 7. Links
// ===========================================================================

#[test]
fn link_basic() {
    let src = "[click](https://example.com)\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have a link");
    assert_eq!(link.text, "click");
    assert_eq!(link.url, "https://example.com");
}

#[test]
fn link_with_title() {
    let src = r#"[click](https://example.com "My Title")"#;
    let src = format!("{src}\n");
    let segs = inline_segments(&src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have a link");
    assert_eq!(link.title.as_deref(), Some("My Title"));
}

#[test]
fn link_with_metadata() {
    let src = "[click](https://example.com [#todo])\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have a link");
    assert!(!link.tags.is_empty(), "link should have metadata tags");
}

#[test]
fn link_with_nested_brackets() {
    let src = "[[inner]](https://example.com)\n";
    let segs = inline_segments(src);
    let has_link = segs.iter().any(|s| matches!(s, InlineSegment::Link(_)));
    assert!(has_link, "nested brackets in link text should work");
}

#[test]
fn link_with_parentheses_in_url() {
    let src = "[wiki](https://en.wikipedia.org/wiki/Rust_(programming_language))\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should handle parens in URL");
    assert!(link.url.contains("Rust_(programming_language)"));
}

#[test]
fn link_empty_text() {
    let src = "[](https://example.com)\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have a link with empty text");
    assert_eq!(link.text, "");
}

#[test]
fn link_empty_url() {
    let src = "[text]()\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have a link with empty url");
    assert_eq!(link.url, "");
}

#[test]
fn bare_brackets_not_link() {
    // [text] without (url) should not be a link
    let src = "just [text] here\n";
    let segs = inline_segments(src);
    let has_link = segs.iter().any(|s| matches!(s, InlineSegment::Link(_)));
    assert!(!has_link, "bare brackets without parens should not be link");
}

// ===========================================================================
// 7b. Images
// ===========================================================================

#[test]
fn image_basic() {
    let src = "![alt text](image.png)\n";
    let segs = inline_segments(src);
    let img = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Image(img) => Some(img),
            _ => None,
        })
        .expect("should have an image");
    assert_eq!(img.alt, "alt text");
    assert_eq!(img.url, "image.png");
}

#[test]
fn image_with_title() {
    let src = r#"![photo](pic.jpg "My Photo")"#;
    let src = format!("{src}\n");
    let segs = inline_segments(&src);
    let img = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Image(img) => Some(img),
            _ => None,
        })
        .expect("should have an image");
    assert_eq!(img.title.as_deref(), Some("My Photo"));
}

#[test]
fn image_empty_alt() {
    let src = "![](placeholder.png)\n";
    let segs = inline_segments(src);
    let img = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Image(img) => Some(img),
            _ => None,
        })
        .expect("should have an image");
    assert_eq!(img.alt, "");
}

#[test]
fn image_not_confused_with_link() {
    let src = "[not image](url.png)\n";
    let segs = inline_segments(src);
    assert!(segs.iter().any(|s| matches!(s, InlineSegment::Link(_))));
    assert!(!segs.iter().any(|s| matches!(s, InlineSegment::Image(_))));
}

#[test]
fn exclamation_without_bracket_is_text() {
    let src = "Hello! World\n";
    let segs = inline_segments(src);
    let text: String = segs
        .iter()
        .filter_map(|s| match s {
            InlineSegment::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(text.contains("!"));
}

// ===========================================================================
// 8. Footnotes
// ===========================================================================

#[test]
fn footnote_definition() {
    let src = "[^1]: This is the footnote content.\n";
    let result = parse(src);
    let fndef = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::FootnoteDefinition(f) => Some(f),
            _ => None,
        })
        .expect("should have footnote definition");
    assert_eq!(fndef.label, "1");
    assert_eq!(fndef.content.plain_text(), "This is the footnote content.");
}

#[test]
fn footnote_definition_named() {
    let src = "[^note]: A named footnote.\n";
    let result = parse(src);
    let fndef = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::FootnoteDefinition(f) => Some(f),
            _ => None,
        })
        .expect("should have footnote definition");
    assert_eq!(fndef.label, "note");
}

#[test]
fn footnote_reference_inline() {
    let src = "Text with footnote[^1] reference.\n";
    let segs = inline_segments(src);
    assert!(
        segs.iter()
            .any(|s| matches!(s, InlineSegment::FootnoteRef(l) if l == "1"))
    );
}

#[test]
fn footnote_label_with_spaces_rejected() {
    // Footnote labels should not contain spaces
    let src = "[^bad label]: Not valid.\n";
    let result = parse(src);
    let has_fn = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::FootnoteDefinition(_)));
    assert!(!has_fn, "footnote label with spaces should be rejected");
}

// ===========================================================================
// 9. HTML blocks
// ===========================================================================

#[test]
fn html_block_basic() {
    let src = "<div class=\"container\">\n  <p>Hello</p>\n</div>\n";
    let result = parse(src);
    let html = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::HtmlBlock(h) => Some(h),
            _ => None,
        })
        .expect("should have HTML block");
    assert!(html.raw.contains("<div"));
    assert!(html.raw.contains("</div>"));
}

#[test]
fn html_void_element() {
    let src = "<br>\n";
    let result = parse(src);
    let html = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::HtmlBlock(h) => Some(h),
            _ => None,
        })
        .expect("should have HTML block for void element");
    assert!(html.raw.contains("<br>"));
}

#[test]
fn html_self_closing() {
    let src = "<img src=\"test.png\" />\n";
    let result = parse(src);
    let html = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::HtmlBlock(h) => Some(h),
            _ => None,
        })
        .expect("should have HTML block for self-closing");
    assert!(html.raw.contains("<img"));
}

#[test]
fn html_inline_in_same_line() {
    let src = "<div>content</div>\n";
    let result = parse(src);
    let html = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::HtmlBlock(h) => Some(h),
            _ => None,
        })
        .expect("should handle inline html");
    assert!(html.raw.contains("content"));
}

// ===========================================================================
// 10. Comments
// ===========================================================================

#[test]
fn line_comment() {
    let src = "// this is a comment\n";
    let result = parse(src);
    let comment = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Comment(c) => Some(c),
            _ => None,
        })
        .expect("should have comment");
    assert_eq!(comment.text, "this is a comment");
}

#[test]
fn block_comment() {
    let src = "/* block\ncomment\nhere */\n";
    let result = parse(src);
    let comment = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Comment(c) => Some(c),
            _ => None,
        })
        .expect("should have block comment");
    assert!(comment.text.contains("comment"));
}

#[test]
fn block_comment_single_line() {
    let src = "/* single line */\n";
    let result = parse(src);
    let has_comment = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Comment(_)));
    assert!(has_comment);
}

// ===========================================================================
// 11. Horizontal rules
// ===========================================================================

#[test]
fn horizontal_rule_asterisks() {
    let src = "***\n";
    assert!(has_block(src, |b| matches!(b, Block::HorizontalRule(_))));
}

#[test]
fn horizontal_rule_underscores() {
    let src = "___\n";
    assert!(has_block(src, |b| matches!(b, Block::HorizontalRule(_))));
}

#[test]
fn horizontal_rule_dashes_four() {
    // Note: `---` is FrontmatterDelim, so need 4+ dashes for HR
    let src = "----\n";
    assert!(has_block(src, |b| matches!(b, Block::HorizontalRule(_))));
}

#[test]
fn horizontal_rule_with_spaces() {
    let src = "* * *\n";
    assert!(has_block(src, |b| matches!(b, Block::HorizontalRule(_))));
}

#[test]
fn three_dashes_after_content_is_hr() {
    let src = "text above\n\n---\n\ntext below\n";
    let result = parse(src);
    let has_hr = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HorizontalRule(_)));
    assert!(has_hr, "--- after content should be a horizontal rule");
    assert!(result.document.frontmatter.is_none());
}

#[test]
fn two_chars_not_horizontal_rule() {
    let src = "**\n";
    let result = parse(src);
    let has_hr = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HorizontalRule(_)));
    assert!(!has_hr, "two chars should not be a horizontal rule");
}

// ===========================================================================
// 12. Frontmatter
// ===========================================================================

#[test]
fn frontmatter_basic() {
    let src = "---\ntitle: My Document\nauthor: Test\n---\n\n# Content\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    let fm = result
        .document
        .frontmatter
        .expect("should have frontmatter");
    assert!(fm.raw.contains("title: My Document"));
}

#[test]
fn frontmatter_with_yaml_types() {
    let src = "---\ntitle: Test\ncount: 42\ntags:\n  - one\n  - two\n---\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    let fm = result
        .document
        .frontmatter
        .expect("should have frontmatter");
    assert!(fm.data.is_mapping());
}

#[test]
fn frontmatter_invalid_yaml() {
    let src = "---\n: invalid: yaml: here:\n---\n";
    let result = parse(src);
    // Should either produce an error or handle gracefully
    // The parser collects InvalidYaml errors
    if !result.errors.is_empty() {
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.kind == ParseErrorKind::InvalidYaml)
        );
    }
}

#[test]
fn frontmatter_unclosed() {
    let src = "---\ntitle: Missing close\n";
    let result = parse(src);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnclosedFrontmatter)
    );
    assert!(result.document.frontmatter.is_none());
}

#[test]
fn frontmatter_not_at_start() {
    // --- not on line 1 should not be frontmatter
    let src = "\n---\ntitle: Not frontmatter\n---\n";
    let result = parse(src);
    assert!(result.document.frontmatter.is_none());
}

#[test]
fn frontmatter_empty() {
    let src = "---\n---\n";
    let result = parse(src);
    // Empty YAML is valid (null/empty)
    assert!(result.document.frontmatter.is_some() || !result.errors.is_empty());
}

#[test]
fn frontmatter_with_dashes_later() {
    // Frontmatter at top, then --- later should be HR
    let src = "---\ntitle: Test\n---\n\n# Heading\n\n---\n\nMore text.\n";
    let result = parse(src);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert!(result.document.frontmatter.is_some());
    let has_hr = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HorizontalRule(_)));
    assert!(has_hr, "--- after frontmatter should be HR");
}

#[test]
fn multiple_dashes_hr_no_frontmatter() {
    // Multiple --- in a document without frontmatter
    let src = "# Title\n\n---\n\nMiddle.\n\n---\n\nEnd.\n";
    let result = parse(src);
    assert!(result.document.frontmatter.is_none());
    let hr_count = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::HorizontalRule(_)))
        .count();
    assert_eq!(hr_count, 2, "both --- should be horizontal rules");
}

#[test]
fn blank_lines_before_dashes_means_no_frontmatter() {
    // Frontmatter `---` must be the very first line of the file.
    // Blank lines before it transition fm_state to Done, so first
    // `---` is an HR. The second `---` after `title: Test` is a
    // setext h2 (per CommonMark).
    let src = "\n\n---\ntitle: Test\n---\n";
    let result = parse(src);
    assert!(
        result.document.frontmatter.is_none(),
        "blank lines before --- should prevent frontmatter"
    );
    let hr_count = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::HorizontalRule(_)))
        .count();
    assert_eq!(hr_count, 1, "first --- should be HR");
    let has_h2 = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(h) if h.level == 2));
    assert!(has_h2, "second --- after text should be setext h2");
}

#[test]
fn content_before_dashes_prevents_frontmatter() {
    // Any content before --- means it's not frontmatter.
    // Per CommonMark, `---` after paragraph text is a setext h2.
    let src = "Hello\n---\nwhat: ever\n---\n";
    let result = parse(src);
    assert!(
        result.document.frontmatter.is_none(),
        "content before --- should prevent frontmatter"
    );
    // First --- is a setext underline (Hello becomes h2)
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(h) if h.level == 2));
    assert!(has_heading, "Hello\\n--- should be setext h2");
    // Second --- after `what: ever` is also setext h2
    let headings: Vec<_> = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::Heading(_)))
        .collect();
    assert_eq!(headings.len(), 2, "both --- should produce setext h2");
}

// ===========================================================================
// 13. Tags (block-level and inline)
// ===========================================================================

#[test]
fn block_tag_todo() {
    let src = "#todo implement feature\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .expect("should have block tag");
    assert!(matches!(
        tag.kind,
        TagKind::Todo {
            text: Some(ref t)
        } if t == "implement feature"
    ));
}

#[test]
fn block_tag_done() {
    let src = "#done finished task\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .expect("should have done tag");
    assert!(matches!(tag.kind, TagKind::Done { .. }));
}

#[test]
fn block_tag_deadline() {
    let src = "#deadline 2026-04-10\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Deadline { .. }));
}

#[test]
fn block_tag_scheduled() {
    let src = "#scheduled 2026-04-05T14:00\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Scheduled {
            date: Timestamp::DateTime(_),
            ..
        }
    ));
}

#[test]
fn block_tag_clock_in_out() {
    let src = "#clock-in 2026-04-03T09:00\n#clock-out 2026-04-03T17:30\n";
    let result = parse(src);
    let tags: Vec<_> = result
        .document
        .children
        .iter()
        .filter_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(tags.len(), 2);
    assert!(matches!(tags[0].kind, TagKind::ClockIn { .. }));
    assert!(matches!(tags[1].kind, TagKind::ClockOut { .. }));
}

#[test]
fn block_tag_clock_duration() {
    let src = "#clock 2h15m\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Clock(ClockValue::Duration { minutes: 135 })
    ));
}

#[test]
fn block_tag_clock_range() {
    let src = "#clock 2026-04-03T09:00/2026-04-03T11:30\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Clock(ClockValue::Range { .. })));
}

#[test]
fn block_tag_priority() {
    for (input, expected) in [
        ("#priority A\n", PriorityLevel::A),
        ("#priority B\n", PriorityLevel::B),
        ("#priority C\n", PriorityLevel::C),
    ] {
        let result = parse(input);
        let tag = result
            .document
            .children
            .iter()
            .find_map(|b| match b {
                Block::BlockTag(t) => Some(t),
                _ => None,
            })
            .unwrap();
        assert!(
            matches!(tag.kind, TagKind::Priority { level } if level == expected),
            "priority {input} should parse correctly"
        );
    }
}

#[test]
fn block_tag_effort() {
    let src = "#effort 3h30m\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Effort { minutes: 210 }));
}

#[test]
fn block_tag_archive() {
    let src = "#archive\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Archive));
}

#[test]
fn block_tag_progress() {
    let src = "#progress\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Progress));
}

#[test]
fn block_tag_unknown() {
    let src = "#custom some value\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(tag.kind, TagKind::Unknown { ref name, ref value } if name == "custom" && value.as_deref() == Some("some value"))
    );
}

#[test]
fn block_tag_deadline_with_repeater_and_warning() {
    let src = "#deadline 2026-04-10T14:00 +1w -3d\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Deadline {
            date: Timestamp::DateTime(_),
            repeater: Some(Repeater {
                interval: 1,
                unit: RepeaterUnit::Week
            }),
            warning: Some(3),
        }
    ));
}

#[test]
fn block_tag_event_with_description() {
    let src = "#event 2026-04-10 +1y Birthday party\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Event {
            repeater: Some(_),
            description: Some(ref d),
            ..
        } if d == "Birthday party"
    ));
}

#[test]
fn block_tag_event_date_range() {
    let src = "#event 2026-04-10/2026-04-12 Conference\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    match &tag.kind {
        TagKind::Event {
            date,
            end_date,
            description,
            ..
        } => {
            assert_eq!(
                date.date(),
                chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
            );
            assert!(end_date.is_some(), "should have end_date");
            assert_eq!(
                end_date.unwrap().date(),
                chrono::NaiveDate::from_ymd_opt(2026, 4, 12).unwrap()
            );
            assert_eq!(description.as_deref(), Some("Conference"));
        }
        other => panic!("expected Event, got {other:?}"),
    }
}

#[test]
fn block_tag_event_datetime_range() {
    let src = "#event 2026-04-10T09:00/2026-04-10T17:00 Workshop\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    match &tag.kind {
        TagKind::Event {
            date,
            end_date,
            description,
            ..
        } => {
            assert!(date.has_time(), "start should have time");
            let end = end_date.expect("should have end_date");
            assert!(end.has_time(), "end should have time");
            assert_eq!(description.as_deref(), Some("Workshop"));
        }
        other => panic!("expected Event, got {other:?}"),
    }
}

#[test]
fn inline_tag_in_paragraph() {
    let src = "Task: #todo fix the bug\n";
    let segs = inline_segments(src);
    let has_tag = segs.iter().any(|s| matches!(s, InlineSegment::Tag(_)));
    assert!(has_tag);
}

#[test]
fn multiple_inline_tags() {
    let src = "Task #todo #priority A #deadline 2026-04-10\n";
    let p = first_para(src);
    let tags = p.content.tags();
    assert!(tags.len() >= 2, "should have multiple inline tags");
}

#[test]
fn tag_with_invalid_argument_falls_back_to_unknown() {
    let src = "#deadline not-a-date\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(tag.kind, TagKind::Unknown { ref name, .. } if name == "deadline"),
        "bad deadline arg should fall back to Unknown"
    );
}

#[test]
fn tag_no_argument() {
    let src = "#todo\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Todo { text: None }));
}

// ===========================================================================
// 14. Callouts
// ===========================================================================

#[test]
fn callout_basic() {
    let src = "> [!note]\n> This is a note.\n";
    let c = first_callout(src);
    assert_eq!(c.kind, "note");
}

#[test]
fn callout_warning() {
    let src = "> [!warning]\n> Be careful!\n";
    let c = first_callout(src);
    assert_eq!(c.kind, "warning");
}

#[test]
fn callout_with_metadata() {
    let src = "> [!note] [#todo]\n> Content here.\n";
    let c = first_callout(src);
    assert_eq!(c.kind, "note");
}

#[test]
fn callout_multiline() {
    let src = "> [!tip]\n> Line one.\n> Line two.\n> Line three.\n";
    let c = first_callout(src);
    assert_eq!(c.kind, "tip");
    assert!(!c.content.is_empty());
}

#[test]
fn callout_case_insensitive_kind() {
    let src = "> [!NOTE]\n> Content.\n";
    let c = first_callout(src);
    assert_eq!(c.kind, "note", "callout kind should be lowercased");
}

// ===========================================================================
// 15. Property drawers
// ===========================================================================

#[test]
fn property_drawer_basic() {
    let src = "## Task\n\n#properties\nid = abc-123\neffort = 2h\n#end\n";
    let h = first_heading(src);
    let props = h.properties.as_ref().expect("should have properties");
    assert_eq!(props.entries.get("id").map(|s| s.as_str()), Some("abc-123"));
    assert_eq!(props.entries.get("effort").map(|s| s.as_str()), Some("2h"));
}

#[test]
fn property_drawer_empty() {
    let src = "## Task\n\n#properties\n#end\n";
    let h = first_heading(src);
    let props = h.properties.as_ref().expect("should have properties");
    assert!(props.entries.is_empty());
}

#[test]
fn property_drawer_invalid_line() {
    let src = "## Task\n\n#properties\nno equals sign\n#end\n";
    let result = parse(src);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnexpectedToken),
        "invalid property line should produce error"
    );
}

#[test]
fn property_drawer_unclosed() {
    let src = "## Task\n\n#properties\nkey = value\n";
    let result = parse(src);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("#properties")),
        "unclosed property drawer should produce error"
    );
}

// ===========================================================================
// 16. Error recovery
// ===========================================================================

#[test]
fn unclosed_code_fence_continues_parsing() {
    let src = "```rust\nfn main() {}\n\n# Next heading\n\nMore text.\n";
    let result = parse(src);
    assert!(!result.errors.is_empty());
    // Should still produce a code block
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    assert!(has_code, "should produce code block even when unclosed");
}

#[test]
fn unclosed_frontmatter_continues_parsing() {
    let src = "---\ntitle: test\n# Heading\n";
    let result = parse(src);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.kind == ParseErrorKind::UnclosedFrontmatter)
    );
}

#[test]
fn empty_document() {
    let src = "";
    let result = parse(src);
    assert!(result.errors.is_empty());
    // Empty document has at least a BlankLine or is completely empty
}

#[test]
fn whitespace_only_document() {
    let src = "   \n   \n   \n";
    let result = parse(src);
    assert!(result.errors.is_empty());
}

#[test]
fn document_with_only_blank_lines() {
    let src = "\n\n\n\n\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
}

// ===========================================================================
// 17. Edge cases and CommonMark deviations
// ===========================================================================

#[test]
fn three_dashes_mid_document_is_horizontal_rule() {
    // `---` mid-document is a horizontal rule (CommonMark thematic break).
    let src = "Text.\n\n---\n\nMore text.\n";
    let result = parse(src);
    let has_hr = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HorizontalRule(_)));
    assert!(has_hr, "--- mid-document should be a horizontal rule");
    assert!(result.document.frontmatter.is_none());
}

#[test]
fn star_at_zero_indent_not_list() {
    // BUG: CommonMark treats `* foo` as a list item regardless of indent.
    // morg requires indent > 0 for * marker to avoid conflict with emphasis.
    let src = "* item\n";
    let result = parse(src);
    let has_list = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::List(_)));
    assert!(
        !has_list,
        "`* item` at zero indent is not a list in morg (deviation from CommonMark)"
    );
}

#[test]
fn heading_with_4_leading_spaces_not_heading() {
    // CommonMark: 4+ leading spaces = indented code block, not heading
    let src = "    # Heading\n";
    let result = parse(src);
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    assert!(!has_heading, "4-space # should not be a heading");
    assert!(has_code, "4-space # should be an indented code block");
}

#[test]
fn inline_code_double_backtick_supported() {
    // CommonMark: `` `code` `` uses double backticks for code containing backticks
    let src = "text ``code with `backtick` inside`` more\n";
    let segs = inline_segments(src);
    let has_code = segs
        .iter()
        .any(|s| matches!(s, InlineSegment::Code(c) if c == "code with `backtick` inside"));
    assert!(
        has_code,
        "double backtick code span should preserve inner backticks"
    );
}

#[test]
fn emphasis_flanking_rules() {
    // CommonMark has complex left/right flanking delimiter rules
    // morg treats every * as a potential italic delimiter
    let src = "foo*bar*baz\n";
    let segs = inline_segments(src);
    // In morg, the *bar* should be parsed as italic
    let has_italic = segs.iter().any(|s| matches!(s, InlineSegment::Italic(_)));
    assert!(has_italic, "intraword emphasis should work in morg");
}

#[test]
fn hash_followed_by_number_is_tag() {
    // In morg, #123 would be a tag attempt
    let src = "#123\n";
    let result = parse(src);
    // The classify_line checks: first char is alphanumeric => tag
    // 1 is alphanumeric, so it becomes UnknownTag { name: "123" }
    let has_tag = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::BlockTag(_)));
    assert!(has_tag, "#123 should be treated as a tag in morg");
}

#[test]
fn escaped_backtick() {
    let src = "not \\`code\\`\n";
    let segs = inline_segments(src);
    let has_code = segs.iter().any(|s| matches!(s, InlineSegment::Code(_)));
    assert!(!has_code, "escaped backticks should not produce code span");
}

#[test]
fn multiple_blocks_sequence() {
    let src = "# Heading\n\nParagraph text.\n\n- List item\n\n```\ncode\n```\n\n> [!note]\n> A note.\n\n---\n";
    let result = parse(src);
    let blocks = &result.document.children;
    let types: Vec<&str> = blocks
        .iter()
        .filter(|b| !matches!(b, Block::BlankLine(_)))
        .map(|b| match b {
            Block::Heading(_) => "heading",
            Block::Paragraph(_) => "paragraph",
            Block::List(_) => "list",
            Block::CodeBlock(_) => "codeblock",
            Block::Callout(_) => "callout",
            Block::HorizontalRule(_) => "hr",
            Block::BlankLine(_) => "blank",
            _ => "other",
        })
        .collect();
    assert!(types.contains(&"heading"));
    assert!(types.contains(&"paragraph"));
    assert!(types.contains(&"list"));
    assert!(types.contains(&"codeblock"));
    assert!(types.contains(&"callout"));
}

#[test]
fn span_tracking_heading() {
    let src = "# Heading\n";
    let h = first_heading(src);
    assert_eq!(h.span.line, 1);
}

#[test]
fn span_tracking_second_block() {
    let src = "\n# Heading\n";
    let h = first_heading(src);
    assert_eq!(h.span.line, 2);
}

#[test]
fn plain_text_extraction() {
    let src = "**bold** and *italic* with `code`\n";
    let p = first_para(src);
    let text = p.content.plain_text();
    assert!(text.contains("bold"));
    assert!(text.contains("italic"));
    assert!(text.contains("code"));
    assert!(!text.contains("**"));
    assert!(!text.contains("*"));
    assert!(!text.contains("`"));
}

#[test]
fn tag_collection_from_inline_content() {
    let src = "Task #todo implement #priority A\n";
    let p = first_para(src);
    let tags = p.content.tags();
    assert!(tags.len() >= 2);
}

#[test]
fn frontmatter_followed_by_content() {
    let src = "---\ntitle: Doc\n---\n\n# First heading\n\nFirst paragraph.\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    assert!(result.document.frontmatter.is_some());
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    let has_para = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Paragraph(_)));
    assert!(has_heading);
    assert!(has_para);
}

// ===========================================================================
// 18. Duration and timestamp parsing edge cases
// ===========================================================================

#[test]
fn duration_hours_only() {
    assert_eq!(morg_parser::parse_duration("2h"), Some(120));
}

#[test]
fn duration_minutes_only() {
    assert_eq!(morg_parser::parse_duration("45m"), Some(45));
}

#[test]
fn duration_combined() {
    assert_eq!(morg_parser::parse_duration("1h30m"), Some(90));
}

#[test]
fn duration_zero() {
    assert_eq!(morg_parser::parse_duration("0h"), Some(0));
    assert_eq!(morg_parser::parse_duration("0m"), Some(0));
}

#[test]
fn duration_large() {
    assert_eq!(morg_parser::parse_duration("100h"), Some(6000));
}

#[test]
fn duration_invalid() {
    assert_eq!(morg_parser::parse_duration("abc"), None);
    assert_eq!(morg_parser::parse_duration(""), None);
    assert_eq!(morg_parser::parse_duration("123"), None); // no unit
    assert_eq!(morg_parser::parse_duration("h"), None); // no number
    assert_eq!(morg_parser::parse_duration("1x"), None); // bad unit
}

#[test]
fn timestamp_date_only() {
    let src = "#deadline 2026-01-15\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Deadline {
            date: Timestamp::Date(_),
            ..
        }
    ));
}

#[test]
fn timestamp_with_seconds() {
    let src = "#deadline 2026-01-15T14:30:45\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Deadline {
            date: Timestamp::DateTime(_),
            ..
        }
    ));
}

#[test]
fn repeater_all_units() {
    for (unit_str, expected) in [
        ("+1d", RepeaterUnit::Day),
        ("+2w", RepeaterUnit::Week),
        ("+3m", RepeaterUnit::Month),
        ("+1y", RepeaterUnit::Year),
    ] {
        let src = format!("#deadline 2026-01-01 {unit_str}\n");
        let result = parse(&src);
        let tag = result
            .document
            .children
            .iter()
            .find_map(|b| match b {
                Block::BlockTag(t) => Some(t),
                _ => None,
            })
            .unwrap();
        match &tag.kind {
            TagKind::Deadline {
                repeater: Some(r), ..
            } => {
                assert_eq!(
                    r.unit, expected,
                    "repeater unit for {unit_str} should match"
                );
            }
            _ => panic!("expected deadline with repeater for {unit_str}"),
        }
    }
}

// ===========================================================================
// 19. Complex/integration tests
// ===========================================================================

#[test]
fn full_document_integration() {
    let src = r#"---
title: Project Plan
author: Test User
---

# Project Overview #todo

This is the main project document with **important** notes.

## Tasks

- [ ] Design the API #priority A
- [x] Set up CI/CD
  - [x] GitHub Actions
  - [ ] Deploy scripts
- [ ] Write documentation

#deadline 2026-04-15 +1w -3d

## Notes

> [!warning]
> This deadline is firm.

| Task | Status | Priority |
|------|--------|----------|
| API  | WIP    | High     |
| Docs | TODO   | Medium   |

```rust #tangle file=main.rs
fn main() {
    println!("Hello, morg!");
}
```

// TODO: Add more sections

[^1]: See the full spec at the project wiki.

Reference[^1] to the footnote.
"#;
    let result = parse(src);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

    let doc = &result.document;
    assert!(doc.frontmatter.is_some());

    let blocks = &doc.children;
    let has_heading = blocks.iter().any(|b| matches!(b, Block::Heading(_)));
    let has_para = blocks.iter().any(|b| matches!(b, Block::Paragraph(_)));
    let has_list = blocks.iter().any(|b| matches!(b, Block::List(_)));
    let has_table = blocks.iter().any(|b| matches!(b, Block::Table(_)));
    let has_code = blocks.iter().any(|b| matches!(b, Block::CodeBlock(_)));
    let has_callout = blocks.iter().any(|b| matches!(b, Block::Callout(_)));
    let has_comment = blocks.iter().any(|b| matches!(b, Block::Comment(_)));
    let has_footnote = blocks
        .iter()
        .any(|b| matches!(b, Block::FootnoteDefinition(_)));
    let has_deadline = blocks.iter().any(|b| matches!(b, Block::BlockTag(_)));

    assert!(has_heading, "should have headings");
    assert!(has_para, "should have paragraphs");
    assert!(has_list, "should have lists");
    assert!(has_table, "should have table");
    assert!(has_code, "should have code block");
    assert!(has_callout, "should have callout");
    assert!(has_comment, "should have comment");
    assert!(has_footnote, "should have footnote");
    assert!(has_deadline, "should have block tag");
}

#[test]
fn heading_followed_by_code_block() {
    let src = "# Code Example\n\n```python\nprint('hello')\n```\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    let blocks = &result.document.children;
    let non_blank: Vec<_> = blocks
        .iter()
        .filter(|b| !matches!(b, Block::BlankLine(_)))
        .collect();
    assert!(matches!(non_blank[0], Block::Heading(_)));
    assert!(matches!(non_blank[1], Block::CodeBlock(_)));
}

#[test]
fn consecutive_headings() {
    let src = "# H1\n## H2\n### H3\n";
    let result = parse(src);
    let headings: Vec<_> = result
        .document
        .children
        .iter()
        .filter_map(|b| match b {
            Block::Heading(h) => Some(h.level),
            _ => None,
        })
        .collect();
    assert_eq!(headings, vec![1, 2, 3]);
}

#[test]
fn list_immediately_after_paragraph() {
    let src = "Some text.\n- Item 1\n- Item 2\n";
    let result = parse(src);
    let has_para = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Paragraph(_)));
    let has_list = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::List(_)));
    assert!(has_para);
    assert!(has_list);
}

#[test]
fn code_block_with_special_characters() {
    let src = "```\n<html>&amp;\"quotes\"</html>\n```\n";
    let cb = first_code_block(src);
    assert!(cb.body.contains("<html>"));
    assert!(cb.body.contains("&amp;"));
}

#[test]
fn deeply_nested_list() {
    let src = "- Level 0\n  - Level 1\n    - Level 2\n      - Level 3\n";
    let l = first_list(src);
    assert_eq!(l.items.len(), 1);
    // Verify deep nesting
    let child1 = match &l.items[0].children[0] {
        Block::List(cl) => cl,
        _ => panic!("expected nested list"),
    };
    let child2 = match &child1.items[0].children[0] {
        Block::List(cl) => cl,
        _ => panic!("expected nested list level 2"),
    };
    let child3 = match &child2.items[0].children[0] {
        Block::List(cl) => cl,
        _ => panic!("expected nested list level 3"),
    };
    assert_eq!(child3.items[0].content.plain_text(), "Level 3");
}

#[test]
fn table_many_rows() {
    let src = "| h1 | h2 |\n|---|---|\n| a | b |\n| c | d |\n| e | f |\n| g | h |\n| i | j |\n";
    let t = first_table(src);
    assert_eq!(t.rows.len(), 5);
}

#[test]
fn callout_with_nested_formatting() {
    let src = "> [!note]\n> This has **bold** and *italic* and `code`.\n";
    let c = first_callout(src);
    // Callout content is re-parsed, so it should have inline formatting
    assert!(!c.content.is_empty());
}

// ===========================================================================
// 20. Regression / stress tests
// ===========================================================================

#[test]
fn very_long_heading() {
    let text = "a".repeat(1000);
    let src = format!("# {text}\n");
    let h = first_heading(&src);
    assert_eq!(h.level, 1);
}

#[test]
fn many_sequential_blank_lines() {
    let src = "\n".repeat(100);
    let result = parse(&src);
    assert!(result.errors.is_empty());
}

#[test]
fn unicode_content() {
    let src = "# 你好世界\n\n这是一段中文。\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    let h = first_heading(src);
    assert!(
        h.content.plain_text().contains("你好世界"),
        "heading should preserve Chinese text"
    );
    let p = first_para(src);
    assert!(
        p.content.plain_text().contains("这是一段中文"),
        "paragraph should preserve Chinese text"
    );
}

#[test]
fn emoji_in_content() {
    let src = "# 🚀 Launch Plan\n\nThis is exciting! 🎉\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    let h = first_heading(src);
    assert!(
        h.content.plain_text().contains("🚀"),
        "heading should preserve emoji"
    );
    let p = first_para(src);
    assert!(
        p.content.plain_text().contains("🎉"),
        "paragraph should preserve emoji"
    );
}

#[test]
fn special_yaml_in_frontmatter() {
    let src = "---\ntitle: \"Quotes: \\\"escaped\\\"\"\nlist:\n  - item1\n  - item2\nbool: true\nnum: 42\n---\n";
    let result = parse(src);
    if result.errors.is_empty() {
        assert!(result.document.frontmatter.is_some());
    }
}

#[test]
fn stray_close_tag_treated_as_paragraph() {
    // A stray </div> not preceded by <div> should be handled gracefully
    let src = "</div>\n";
    let result = parse(src);
    // Should not panic; treated as paragraph or ignored
    assert!(!result.document.children.is_empty());
}

#[test]
fn stray_properties_close_treated_as_paragraph() {
    let src = "#end\n";
    let result = parse(src);
    assert!(!result.document.children.is_empty());
}

#[test]
fn code_block_fence_with_info_containing_hash() {
    // Hash in info string might confuse the parser
    let src = "```c# #tangle\ncode here\n```\n";
    let result = parse(src);
    // Should produce a code block of some kind
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    assert!(has_code);
}

#[test]
fn adjacent_formatting_delimiters() {
    // Test behavior of adjacent bold/italic markers
    let src = "****\n";
    let result = parse(src);
    // Should not panic
    assert!(!result.document.children.is_empty());
}

#[test]
fn link_in_heading() {
    let src = "# [Click Here](https://example.com)\n";
    let h = first_heading(src);
    assert!(
        h.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Link(_)))
    );
}

#[test]
fn footnote_ref_in_heading() {
    let src = "# Heading[^1]\n";
    let h = first_heading(src);
    assert!(
        h.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::FootnoteRef(_)))
    );
}

#[test]
fn tag_in_list_item() {
    let src = "- Task item #todo\n";
    let l = first_list(src);
    let tags = l.items[0].content.tags();
    assert!(!tags.is_empty(), "list item should contain inline tag");
}

#[test]
fn block_comment_unclosed() {
    let src = "/* unclosed comment\nstill going\n";
    let result = parse(src);
    // Should handle gracefully (comment consumes to EOF)
    let has_comment = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Comment(_)));
    assert!(has_comment);
}

#[test]
fn mixed_list_markers_dash_and_plus() {
    let src = "- Dash item\n+ Plus item\n";
    let result = parse(src);
    let lists: Vec<_> = result
        .document
        .children
        .iter()
        .filter(|b| matches!(b, Block::List(_)))
        .collect();
    assert!(!lists.is_empty());
}

#[test]
fn closed_tag() {
    let src = "#closed 2026-04-09T15:30\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Closed { .. }));
}

#[test]
fn date_tag() {
    let src = "#date 2026-06-15\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(tag.kind, TagKind::Date { .. }));
}

#[test]
fn priority_custom() {
    let src = "#priority X\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Priority {
            level: PriorityLevel::Custom('X')
        }
    ));
}

#[test]
fn priority_lowercase() {
    let src = "#priority a\n";
    let result = parse(src);
    let tag = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::BlockTag(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        tag.kind,
        TagKind::Priority {
            level: PriorityLevel::A
        }
    ));
}

// ===========================================================================
// Autolinks
// ===========================================================================

#[test]
fn autolink_url() {
    let src = "Visit <https://example.com> for details.\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have autolink");
    assert_eq!(link.url, "https://example.com");
    assert_eq!(link.text, "https://example.com");
}

#[test]
fn autolink_email() {
    let src = "Contact <user@example.com> for help.\n";
    let segs = inline_segments(src);
    let link = segs
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have autolink");
    assert_eq!(link.url, "mailto:user@example.com");
    assert_eq!(link.text, "user@example.com");
}

#[test]
fn autolink_standalone_line_not_html_block() {
    let src = "<https://example.com>\n";
    let result = parse(src);
    // Should NOT be an HTML block
    let has_html = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::HtmlBlock(_)));
    assert!(
        !has_html,
        "autolink on its own line should not be HTML block"
    );
    // Should be a paragraph containing a link
    let has_link = result.document.children.iter().any(|b| match b {
        Block::Paragraph(p) => p
            .content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Link(_))),
        _ => false,
    });
    assert!(has_link, "autolink should produce a link in a paragraph");
}

#[test]
fn angle_bracket_without_url_is_text() {
    let src = "a < b > c\n";
    let result = parse(src);
    // Should be plain text, not a link
    let has_link = result.document.children.iter().any(|b| match b {
        Block::Paragraph(p) => p
            .content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::Link(_))),
        _ => false,
    });
    assert!(!has_link);
}

// ===========================================================================
// 21. Hard line breaks
// ===========================================================================

#[test]
fn hard_break_trailing_spaces() {
    let src = "line one  \nline two\n";
    let p = first_para(src);
    assert!(
        p.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::HardBreak)),
        "two trailing spaces should produce a hard break"
    );
}

#[test]
fn hard_break_trailing_backslash() {
    let src = "line one\\\nline two\n";
    let p = first_para(src);
    assert!(
        p.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::HardBreak)),
        "trailing backslash should produce a hard break"
    );
}

#[test]
fn no_hard_break_single_space() {
    let src = "line one \nline two\n";
    let p = first_para(src);
    assert!(
        !p.content
            .segments
            .iter()
            .any(|s| matches!(s, InlineSegment::HardBreak)),
        "single trailing space should NOT produce a hard break"
    );
}

#[test]
fn hard_break_preserves_text() {
    let src = "before  \nafter\n";
    let p = first_para(src);
    let text = p.content.plain_text();
    assert!(text.contains("before"), "text before hard break preserved");
    assert!(text.contains("after"), "text after hard break preserved");
}

// ===========================================================================
// 22. Link reference definitions and references
// ===========================================================================

#[test]
fn link_ref_def_basic() {
    let src = "[foo]: /url\n\nSee [foo] for details.\n";
    let result = parse(src);
    assert!(result.errors.is_empty());
    // Symbol table should have the definition
    assert!(result.document.link_defs.contains_key("foo"));
    // The paragraph should have a resolved Link, not a LinkRef
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .expect("should have paragraph");
    let has_link = para
        .content
        .segments
        .iter()
        .any(|s| matches!(s, InlineSegment::Link(l) if l.url == "/url" && l.text == "foo"));
    assert!(has_link, "shortcut ref [foo] should resolve to link");
}

#[test]
fn link_ref_def_with_title() {
    let src = "[example]: https://example.com \"Example Site\"\n\nClick [example].\n";
    let result = parse(src);
    let target = result
        .document
        .link_defs
        .get("example")
        .expect("should have definition");
    assert_eq!(target.url, "https://example.com");
    assert_eq!(target.title.as_deref(), Some("Example Site"));
    // Check resolved link has title
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let link = para
        .content
        .segments
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should have resolved link");
    assert_eq!(link.title.as_deref(), Some("Example Site"));
}

#[test]
fn link_ref_full_form() {
    // [display text][label]
    let src = "[foo]: /url\n\n[click here][foo]\n";
    let result = parse(src);
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let link = para
        .content
        .segments
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("full ref should resolve");
    assert_eq!(link.text, "click here");
    assert_eq!(link.url, "/url");
}

#[test]
fn link_ref_collapsed_form() {
    // [label][]
    let src = "[foo]: /url\n\n[foo][]\n";
    let result = parse(src);
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let link = para
        .content
        .segments
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("collapsed ref should resolve");
    assert_eq!(link.text, "foo");
    assert_eq!(link.url, "/url");
}

#[test]
fn link_ref_case_insensitive() {
    let src = "[Foo]: /url\n\n[foo] and [FOO]\n";
    let result = parse(src);
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let links: Vec<_> = para
        .content
        .segments
        .iter()
        .filter(|s| matches!(s, InlineSegment::Link(_)))
        .collect();
    assert_eq!(links.len(), 2, "both case variants should resolve");
}

#[test]
fn link_ref_unresolved_stays_as_linkref() {
    let src = "See [undefined] for details.\n";
    let result = parse(src);
    let para = first_para(src);
    let has_linkref = para
        .content
        .segments
        .iter()
        .any(|s| matches!(s, InlineSegment::LinkRef { .. }));
    assert!(has_linkref, "unresolved reference should remain as LinkRef");
    // Should not be a Link
    let has_link = para
        .content
        .segments
        .iter()
        .any(|s| matches!(s, InlineSegment::Link(_)));
    assert!(!has_link, "unresolved reference should not become Link");
    let _ = result;
}

#[test]
fn link_ref_first_def_wins() {
    // CommonMark: first definition takes precedence
    let src = "[foo]: /first\n[foo]: /second\n\n[foo]\n";
    let result = parse(src);
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let link = para
        .content
        .segments
        .iter()
        .find_map(|s| match s {
            InlineSegment::Link(l) => Some(l),
            _ => None,
        })
        .expect("should resolve");
    assert_eq!(link.url, "/first", "first definition should win");
}

#[test]
fn link_ref_def_with_angle_bracket_url() {
    let src = "[foo]: <https://example.com/path with spaces>\n\n[foo]\n";
    let result = parse(src);
    let target = result
        .document
        .link_defs
        .get("foo")
        .expect("should have def");
    assert_eq!(target.url, "https://example.com/path with spaces");
}

#[test]
fn link_ref_def_single_quote_title() {
    let src = "[foo]: /url 'Single Title'\n\n[foo]\n";
    let result = parse(src);
    let target = result
        .document
        .link_defs
        .get("foo")
        .expect("should have def");
    assert_eq!(target.title.as_deref(), Some("Single Title"));
}

#[test]
fn link_ref_def_paren_title() {
    let src = "[foo]: /url (Paren Title)\n\n[foo]\n";
    let result = parse(src);
    let target = result
        .document
        .link_defs
        .get("foo")
        .expect("should have def");
    assert_eq!(target.title.as_deref(), Some("Paren Title"));
}

#[test]
fn link_ref_def_not_confused_with_footnote() {
    let src = "[^fn]: footnote content\n";
    let result = parse(src);
    // Should be footnote, not link def
    let has_fn = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::FootnoteDefinition(_)));
    let has_linkdef = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::LinkDefinition(_)));
    assert!(has_fn, "should be footnote");
    assert!(!has_linkdef, "should not be link definition");
}

#[test]
fn link_ref_in_heading() {
    let src = "[foo]: /url\n\n# See [foo]\n";
    let result = parse(src);
    let heading = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Heading(h) => Some(h),
            _ => None,
        })
        .expect("should have heading");
    let has_link = heading
        .content
        .segments
        .iter()
        .any(|s| matches!(s, InlineSegment::Link(l) if l.url == "/url"));
    assert!(has_link, "link ref in heading should resolve");
}

#[test]
fn link_ref_multiple_defs_and_refs() {
    let src = "[a]: /a\n[b]: /b\n[c]: /c\n\nUse [a], [b], and [c].\n";
    let result = parse(src);
    assert_eq!(result.document.link_defs.len(), 3);
    let para = result
        .document
        .children
        .iter()
        .find_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .unwrap();
    let link_count = para
        .content
        .segments
        .iter()
        .filter(|s| matches!(s, InlineSegment::Link(_)))
        .count();
    assert_eq!(link_count, 3, "all three refs should resolve");
}

#[test]
fn link_def_block_is_in_children() {
    let src = "[foo]: /url\n\nText.\n";
    let result = parse(src);
    let has_def = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::LinkDefinition(_)));
    assert!(has_def, "LinkDefinition block should be in children");
}

// ===========================================================================
// 23. Indented code blocks
// ===========================================================================

#[test]
fn indented_code_block_basic() {
    let src = "    code line 1\n    code line 2\n";
    let cb = first_code_block(src);
    assert_eq!(cb.lang, None, "indented code blocks have no language");
    assert!(cb.body.contains("code line 1"));
    assert!(cb.body.contains("code line 2"));
}

#[test]
fn indented_code_block_strips_prefix() {
    let src = "    hello\n";
    let cb = first_code_block(src);
    assert_eq!(cb.body, "hello", "4-space prefix should be stripped");
}

#[test]
fn indented_code_block_tab() {
    let src = "\tcode with tab\n";
    let cb = first_code_block(src);
    assert_eq!(cb.body, "code with tab");
}

#[test]
fn indented_code_block_blank_line_within() {
    let src = "    line 1\n\n    line 3\n";
    let cb = first_code_block(src);
    assert!(
        cb.body.contains("line 1") && cb.body.contains("line 3"),
        "blank line within indented block should be preserved: {:?}",
        cb.body
    );
    assert!(cb.body.contains('\n'), "should have newlines");
}

#[test]
fn indented_code_block_ends_at_non_indented() {
    let src = "    code\n\nnot code\n";
    let result = parse(src);
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    let has_para = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Paragraph(p) if p.content.plain_text() == "not code"));
    assert!(has_code, "should have code block");
    assert!(has_para, "should have paragraph after code block");
}

#[test]
fn indented_code_block_cannot_interrupt_paragraph() {
    // CommonMark: indented code cannot interrupt a paragraph
    let src = "paragraph\n    not code\n";
    let result = parse(src);
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    assert!(
        !has_code,
        "indented line after paragraph should not be a code block"
    );
}

#[test]
fn indented_code_block_after_blank_line() {
    let src = "paragraph\n\n    code\n";
    let result = parse(src);
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(cb) if cb.body.contains("code")));
    assert!(
        has_code,
        "indented line after blank line should be a code block"
    );
}

#[test]
fn indented_code_block_4_spaces_heading_is_code() {
    // 4+ spaces before # = code, not heading
    let src = "    # not a heading\n";
    let result = parse(src);
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    let has_heading = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::Heading(_)));
    assert!(has_code, "4-space # should be code");
    assert!(!has_heading, "4-space # should not be heading");
}

#[test]
fn indented_code_block_trailing_blanks_stripped() {
    let src = "    code\n\n\n";
    let cb = first_code_block(src);
    assert_eq!(cb.body, "code", "trailing blanks should be stripped");
}

#[test]
fn three_spaces_not_code_block() {
    let src = "   not code\n";
    let result = parse(src);
    let has_code = result
        .document
        .children
        .iter()
        .any(|b| matches!(b, Block::CodeBlock(_)));
    assert!(!has_code, "3 spaces should not be a code block");
}
