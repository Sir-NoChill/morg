use std::collections::HashMap;

use crate::span::Span;
use crate::tags::Tag;

// ===========================================================================
// Document — top-level AST node
// ===========================================================================

/// The root of a parsed morg-mode document.
///
/// After parsing, [`Document::link_defs`] contains a symbol table mapping
/// normalised link labels to their destinations. This is populated by a
/// **def-ref resolution pass** that runs automatically at the end of
/// [`parse_document`](crate::parser::parse_document):
///
/// 1. **Collect** — walk `children` and extract every
///    [`Block::LinkDefinition`]. Insert each into `link_defs`.
/// 2. **Resolve** — walk all inline content and convert every
///    [`InlineSegment::LinkRef`] whose label matches an entry in
///    `link_defs` into an [`InlineSegment::Link`].
///
/// This two-pass design (parse → resolve) keeps the parser itself
/// single-pass and context-free while still supporting forward references.
/// The same pattern can be reused for future def-ref features (e.g.
/// cross-file `id:` references) by adding new symbol tables and
/// resolution passes.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub frontmatter: Option<Frontmatter>,
    pub children: Vec<Block>,
    /// Symbol table: normalised link label → (url, optional title).
    /// Populated by the def-ref resolution pass after parsing.
    pub link_defs: HashMap<String, LinkTarget>,
}

/// The resolved destination of a link reference definition.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkTarget {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    pub raw: String,
    pub data: serde_yaml::Value,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(Heading),
    Paragraph(Paragraph),
    CodeBlock(CodeBlock),
    BlankLine(Span),
    BlockTag(Tag),
    Callout(Callout),
    Table(Table),
    HtmlBlock(HtmlBlock),
    List(List),
    HorizontalRule(Span),
    Comment(Comment),
    FootnoteDefinition(FootnoteDefinition),
    /// `[label]: url "title"` — link reference definition.
    /// Consumed by the def-ref resolution pass to populate
    /// [`Document::link_defs`]; not rendered in output.
    LinkDefinition(LinkDefinition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteDefinition {
    pub label: String,
    pub content: InlineContent,
    pub span: Span,
}

/// A link reference definition: `[label]: url "optional title"`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkDefinition {
    pub label: String,
    pub url: String,
    pub title: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub content: InlineContent,
    pub properties: Option<PropertyDrawer>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDrawer {
    pub entries: HashMap<String, String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    pub content: InlineContent,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    pub lang: Option<String>,
    pub tags: Vec<Tag>,
    pub attributes: HashMap<String, String>,
    pub body: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Callout {
    pub kind: String,
    pub tags: Vec<Tag>,
    pub attributes: HashMap<String, String>,
    pub content: Vec<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub headers: Vec<InlineContent>,
    pub alignments: Vec<Alignment>,
    pub rows: Vec<Vec<InlineContent>>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HtmlBlock {
    pub raw: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub kind: ListKind,
    pub items: Vec<ListItem>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub checkbox: Option<Checkbox>,
    pub content: InlineContent,
    pub description: Option<InlineContent>,
    pub children: Vec<Block>,
    pub indent: usize,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkbox {
    Unchecked,
    Checked,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineContent {
    pub segments: Vec<InlineSegment>,
}

impl InlineContent {
    pub fn plain(text: &str) -> Self {
        Self {
            segments: vec![InlineSegment::Text(text.to_string())],
        }
    }

    pub fn empty() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn tags(&self) -> Vec<&Tag> {
        let mut result = Vec::new();
        collect_tags_from_segments(&self.segments, &mut result);
        result
    }

    /// Extract plain text from inline content, stripping all markup.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        plain_text_segments(&self.segments, &mut out);
        out.trim().to_string()
    }
}

fn plain_text_segments(segments: &[InlineSegment], out: &mut String) {
    for seg in segments {
        match seg {
            InlineSegment::Text(t) => out.push_str(t),
            InlineSegment::Code(c) => out.push_str(c),
            InlineSegment::Tag(_) => {}
            InlineSegment::Bold(inner)
            | InlineSegment::Italic(inner)
            | InlineSegment::Strikethrough(inner) => {
                plain_text_segments(&inner.segments, out);
            }
            InlineSegment::Link(link) => out.push_str(&link.text),
            InlineSegment::Image(img) => out.push_str(&img.alt),
            InlineSegment::HardBreak => out.push('\n'),
            InlineSegment::LinkRef { text, .. } => out.push_str(text),
            InlineSegment::FootnoteRef(label) => {
                out.push_str("[^");
                out.push_str(label);
                out.push(']');
            }
        }
    }
}

fn collect_tags_from_segments<'a>(segments: &'a [InlineSegment], out: &mut Vec<&'a Tag>) {
    for seg in segments {
        match seg {
            InlineSegment::Tag(t) => out.push(t),
            InlineSegment::Bold(inner)
            | InlineSegment::Italic(inner)
            | InlineSegment::Strikethrough(inner) => {
                collect_tags_from_segments(&inner.segments, out);
            }
            InlineSegment::Link(link) => {
                for t in &link.tags {
                    out.push(t);
                }
            }
            InlineSegment::Text(_)
            | InlineSegment::Code(_)
            | InlineSegment::FootnoteRef(_)
            | InlineSegment::Image(_)
            | InlineSegment::HardBreak
            | InlineSegment::LinkRef { .. } => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineSegment {
    Text(String),
    Tag(Tag),
    Bold(InlineContent),
    Italic(InlineContent),
    Strikethrough(InlineContent),
    Code(String),
    Link(Link),
    Image(Image),
    HardBreak,
    FootnoteRef(String),
    /// An unresolved link reference: `[text][label]` or `[label]`.
    /// The def-ref resolution pass converts these to [`Link`] when
    /// a matching [`LinkDefinition`] exists. Any that remain
    /// unresolved after the pass are left as-is (rendered as plain
    /// text by consumers).
    LinkRef {
        /// Display text. For shortcut refs (`[label]`), same as `label`.
        text: String,
        /// The normalised label used to look up [`Document::link_defs`].
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub alt: String,
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub text: String,
    pub url: String,
    pub title: Option<String>,
    pub tags: Vec<Tag>,
    pub attributes: HashMap<String, String>,
}
