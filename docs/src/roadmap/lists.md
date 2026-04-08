# Lists with Checkboxes

## Summary

Parse markdown lists (unordered, ordered) with optional checkbox syntax for task tracking.

## Syntax

```
- Regular item
- [ ] Unchecked task
- [x] Completed task
  - Nested item
  - [ ] Nested task

1. Ordered item
2. [ ] Ordered task
```

## Implementation

### Parser changes

- **`ast.rs`**: Add list types:
  ```
  List {
      kind: ListKind,     // Unordered or Ordered
      items: Vec<ListItem>,
      span: Span,
  }

  ListItem {
      checkbox: Option<Checkbox>,  // None, Unchecked, Checked
      content: InlineContent,
      children: Vec<Block>,       // nested blocks (sub-lists, paragraphs)
      span: Span,
  }

  enum ListKind { Unordered, Ordered }
  enum Checkbox { Unchecked, Checked }
  ```
  Add `List` variant to `Block`.

- **`scanner.rs`**: Classify lines starting with `- `, `* ` (when not heading), `+ `, or `N. ` as `ListItem` lines. Track indentation level.
- **`parser.rs`**: Parse consecutive list item lines into a `List` block. Handle indentation-based nesting by recursively parsing indented content as child blocks.

### CLI changes

- **`todos`**: Collect checkboxes as lightweight todos (distinct from `#todo` tags but included in listing).
- **`collect.rs`**: Walk list items for inline tags and checkboxes.

### Files to modify

- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/scanner.rs`
- `crates/morg-parser/src/parser.rs`
- `crates/morg-mode/src/collect.rs`
- `crates/morg-mode/src/commands/todos.rs`
