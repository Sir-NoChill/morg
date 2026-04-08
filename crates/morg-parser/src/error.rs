use std::fmt;

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnclosedCodeFence,
    UnclosedFrontmatter,
    InvalidTagArgument,
    InvalidYaml,
    InvalidTable,
    UnclosedHtmlBlock,
    UnexpectedToken,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}: {}",
            self.span.line, self.span.col, self.kind, self.message
        )
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnclosedCodeFence => write!(f, "unclosed code fence"),
            Self::UnclosedFrontmatter => write!(f, "unclosed frontmatter"),
            Self::InvalidTagArgument => write!(f, "invalid tag argument"),
            Self::InvalidYaml => write!(f, "invalid YAML"),
            Self::InvalidTable => write!(f, "invalid table"),
            Self::UnclosedHtmlBlock => write!(f, "unclosed HTML block"),
            Self::UnexpectedToken => write!(f, "unexpected token"),
        }
    }
}

impl std::error::Error for ParseError {}
