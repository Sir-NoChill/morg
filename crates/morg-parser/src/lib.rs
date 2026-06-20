//! Parser for the morg-mode document format.
//!
//! morg-mode extends standard Markdown with a `#tag` system for metadata,
//! time tracking, task management, literate programming, and personal knowledge
//! management.
//!
//! # Quick start
//!
//! ```rust
//! use morg_parser::{parse_document, Block, TagKind};
//!
//! let src = "# My note\n\nFix the login flow. #todo priority A\n";
//! let result = morg_parser::parser::parse_document(src);
//!
//! for block in &result.document.children {
//!     if let Block::Heading(h) = block {
//!         for tag in h.content.tags() {
//!             println!("tag: {:?}", tag.kind);
//!         }
//!     }
//! }
//! ```
//!
//! # Entry point
//!
//! [`parse_document`] is the only function you need. It accepts a `&str` and
//! returns a [`parser::ParseResult`] containing the [`ast::Document`] and any
//! non-fatal [`error::ParseError`]s encountered.
//!
//! Errors are recoverable — the parser always produces a complete tree and
//! collects diagnostics separately, so callers can render partial output even
//! when the source contains mistakes.
//!
//! # Tag syntax
//!
//! Tags appear as `#name` (bare) or `#name value` (with an argument) anywhere
//! inline or at the start of a block line. The full set of built-in tag names
//! is defined in [`tokens::Keyword`]. Unrecognised names become
//! [`tags::TagKind::Unknown`] and are preserved in the AST.

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod tags;
pub mod tokens;

pub use ast::*;
pub use error::{ParseError, ParseErrorKind};
pub use parser::parse_document;
pub use span::Span;
pub use tags::*;
