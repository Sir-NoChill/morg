# Footnotes

## Summary

Support markdown-style footnotes for references and annotations.

## Syntax

Reference in text: `[^name]`
Definition at document level: `[^name]: Footnote text here`

```
This claim needs a source[^1].

[^1]: Smith et al., 2025, "On the nature of markdown"
```

## Implementation

- **`ast.rs`**: Add `FootnoteRef { label: String }` to `InlineSegment`. Add `FootnoteDefinition { label: String, content: InlineContent, span: Span }` to `Block`.
- **`scanner.rs`**: Classify lines starting with `[^name]:` as `FootnoteDefinition`.
- **`parser.rs`**: Inline parser detects `[^name]` references. Block parser handles definitions.

### Files to modify
- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/scanner.rs`
- `crates/morg-parser/src/parser.rs`
