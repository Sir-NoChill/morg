# Parser Architecture Refactor

## Summary

Refactor `morg-parser` to follow an LLVM/rustc-style architecture where:
1. Keywords and token definitions live in a macro-generated definitions file
2. A separate lexer/tokenizer produces a token stream
3. The parser consumes tokens via rules, not raw classified lines

## Motivation

The current parser interleaves line classification (scanner.rs) with recursive descent parsing (parser.rs). As the tag vocabulary grows, adding new keywords requires changes in multiple places. An LLVM-style approach centralizes definitions and makes the system more maintainable.

## Architecture

### Token definitions via `macro_rules!`

```rust
// tokens.rs
macro_rules! define_keywords {
    ($($name:ident => $string:literal),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Keyword {
            $($name,)*
        }

        impl Keyword {
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($string => Some(Self::$name),)*
                    _ => None,
                }
            }

            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$name => $string,)*
                }
            }
        }
    };
}

define_keywords! {
    Todo => "todo",
    Done => "done",
    Deadline => "deadline",
    Scheduled => "scheduled",
    Date => "date",
    Event => "event",
    ClockIn => "clock-in",
    ClockOut => "clock-out",
    Clock => "clock",
    Tangle => "tangle",
    Priority => "priority",
    Effort => "effort",
    Closed => "closed",
    Archive => "archive",
    Progress => "progress",
    Properties => "properties",
    End => "end",
}
```

### Token types

```rust
#[derive(Debug, Clone)]
pub enum Token {
    // Structure
    Heading { level: u8 },
    FencedCodeOpen { info: String },
    FencedCodeClose,
    FrontmatterDelim,
    HorizontalRule,
    ListMarker { kind: ListKind, indent: usize },
    TablePipe,
    BlockquoteMarker,

    // Tags
    Tag { keyword: Keyword },
    UnknownTag { name: String },
    TagArg(String),

    // Inline
    Text(String),
    Bold,
    Italic,
    Strikethrough,
    Code(String),
    LinkOpen,
    LinkClose,

    // Metadata
    Attribute { key: String, value: String },
    PropertyLine { key: String, value: String },

    // Comments
    LineComment(String),
    BlockCommentOpen,
    BlockCommentClose,

    // Control
    Newline,
    Eof,
}
```

### Lexer

The lexer operates on the full source, producing a `Vec<Token>` (or lazy iterator). It handles:
- Line-level classification (headings, fences, table pipes)
- Tag recognition (using `Keyword::from_str`)
- Inline tokenization (markup delimiters, links, footnotes)

### Parser

The parser consumes tokens through a `peek()`/`advance()` interface similar to the current scanner, but operating on `Token`s. Grammar rules become cleaner:

```rust
fn parse_heading(&mut self) -> Heading {
    let Token::Heading { level } = self.expect_heading();
    let content = self.parse_inline_until(Token::Newline);
    let properties = if self.peek() == Token::Tag { keyword: Keyword::Properties } {
        Some(self.parse_property_drawer())
    } else {
        None
    };
    Heading { level, content, properties }
}
```

## Implementation Plan

1. Create `tokens.rs` with `define_keywords!` macro and `Token` enum
2. Create `lexer.rs` that replaces `scanner.rs` — produces token stream
3. Refactor `parser.rs` to consume tokens instead of classified lines
4. Remove `scanner.rs` once migration is complete
5. Update all tests

## Reference

- rustc lexer: `compiler/rustc_lexer/src/lib.rs` — character-level tokenization
- rustc parser: `compiler/rustc_parse/src/parser/` — token-consuming parser
- LLVM TableGen: keyword definitions via `.td` files, consumed by backends

### Files to modify
- `crates/morg-parser/src/tokens.rs` (new)
- `crates/morg-parser/src/lexer.rs` (new)
- `crates/morg-parser/src/parser.rs` (rewrite)
- `crates/morg-parser/src/scanner.rs` (remove)
- `crates/morg-parser/src/lib.rs`
