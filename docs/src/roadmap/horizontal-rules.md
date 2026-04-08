# Horizontal Rules

## Summary

Parse `---` (when not at line 1), `***`, and `___` as horizontal rules (thematic breaks).

## Syntax

```
Content above

---

Content below
```

Note: `---` at line 1 is already frontmatter. At any other position, three or more `-`, `*`, or `_` on a line alone becomes a horizontal rule.

## Implementation

- **`ast.rs`**: Add `HorizontalRule(Span)` variant to `Block`.
- **`scanner.rs`**: Classify lines matching `^[-*_]{3,}\s*$` (when not frontmatter context).
- **`parser.rs`**: Emit `HorizontalRule` blocks. Currently `---` not at line 1 is treated as a paragraph — change this.

### Files to modify
- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/scanner.rs`
- `crates/morg-parser/src/parser.rs`
