# Markdown Links

## Summary

Parse standard markdown links in inline content, with an extended syntax for metadata.

## Syntax

```
[visible text](url)
[visible text](url "title")
[visible text](filename.ext "title" [#tangle file=out.rs])
[](#heading-anchor)
```

The optional metadata bracket `[...]` inside the link uses the same tag/attribute syntax as code fences and callout metadata.

## Implementation

### Parser changes

- **`ast.rs`**: Add `Link` variant to `InlineSegment`:
  ```
  Link {
      text: String,
      url: String,
      title: Option<String>,
      tags: Vec<Tag>,
      attributes: HashMap<String, String>,
  }
  ```
- **`parser.rs` (inline parser)**: Detect `[` and parse the full `[text](url "title" [metadata])` structure. Fall back to plain text if the pattern doesn't complete.

### CLI changes

- **`tangle`**: Links with `#tangle` metadata could specify include-style file insertion.
- **`collect.rs`**: Walk `Link` segments for tags.

### Files to modify

- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/parser.rs`
- `crates/morg-mode/src/collect.rs`
- `crates/morg-mode/src/commands/tangle.rs`
