# Property Drawers

## Summary

Add per-heading key-value metadata blocks, analogous to org-mode's `:PROPERTIES:...:END:` drawers but using morg tag syntax.

## Syntax

A `#properties` block immediately after a heading, terminated by `#end`:

```
## My Task

#properties
id = 550e8400-e29b-41d4-a716-446655440000
effort = 2h30m
category = backend
#end
```

## Implementation

### Parser changes

- **`ast.rs`**: Add `PropertyDrawer` struct:
  ```
  PropertyDrawer {
      entries: HashMap<String, String>,
      span: Span,
  }
  ```
  Add `properties: Option<PropertyDrawer>` field to `Heading`.

- **`scanner.rs`**: Recognize `#properties` and `#end` as special line kinds.
- **`parser.rs`**: After parsing a heading, check if the next non-blank block is a `#properties` tag. If so, consume lines until `#end`, parsing each as `key = value`.

### CLI changes

- **`collect.rs`**: Expose heading properties in `TagContext`.
- **`todos`/`agenda`**: Display relevant properties (effort, category) when present.

### Files to modify

- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/scanner.rs`
- `crates/morg-parser/src/parser.rs`
- `crates/morg-mode/src/collect.rs`
