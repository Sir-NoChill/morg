# Description Lists

## Summary

Support definition/description lists using markdown syntax.

## Syntax

```
- **Term**: Description of the term
- **Another term**: Its description
```

Or a more concise syntax:
```
- Term :: Description
- Another :: Its description
```

## Implementation

- **`ast.rs`**: Add `description: Option<InlineContent>` to `ListItem`. When a list item starts with `term :: ` or `**term**: `, split into term and description.
- **`parser.rs`**: Detect `::` separator in `parse_list_item_content`.

### Files to modify
- `crates/morg-parser/src/ast.rs`
- `crates/morg-parser/src/parser.rs`
