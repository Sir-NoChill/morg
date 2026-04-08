# Inline Markup

## Summary

Parse standard markdown inline formatting: bold, italic, strikethrough, and inline code.

## Syntax

```
**bold text**
*italic text*
~~strikethrough~~
`inline code`
```

## Implementation

### Parser changes

- **`ast.rs`**: Add variants to `InlineSegment`:
  ```
  Bold(InlineContent),
  Italic(InlineContent),
  Strikethrough(InlineContent),
  Code(String),
  ```
  Note: `Bold` and `Italic` contain `InlineContent` to allow nesting (e.g., `**bold with #tag**`). `Code` is a plain string (no tag parsing inside backticks).

- **`parser.rs` (inline parser)**: Detect delimiter characters and parse matched pairs:
  - `**...**` -- bold
  - `*...*` -- italic (when not `**`)
  - `~~...~~` -- strikethrough
  - `` `...` `` -- code (no inner parsing)

### Files to modify

- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/parser.rs`
- `crates/morg-mode/src/collect.rs` (walk new segment types for tags)
