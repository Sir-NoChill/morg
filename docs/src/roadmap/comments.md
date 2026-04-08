# Comments

## Summary

Support comment lines that are preserved in the AST but excluded from output.

## Syntax

Since `# ` is a heading, we use `//` for comments (C-style, distinct from all markdown constructs):

```
// This is a comment — ignored by all commands
// TODO: this is also a comment, not a tag

## Heading
// Internal note about this section
```

For block comments:
```
/*
Multi-line comment.
Excluded from all processing.
*/
```

## Implementation

- **`ast.rs`**: Add `Comment { text: String, span: Span }` variant to `Block`.
- **`scanner.rs`**: Classify lines starting with `//` as `Comment`. Detect `/*` and `*/` for block comments.
- **`parser.rs`**: Emit `Comment` blocks. All commands skip them during walking.

### Files to modify
- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/scanner.rs`
- `crates/morg-parser/src/parser.rs`
