# Column View

## Summary

Generate a tabular view of headings with selected properties, similar to org-column-view.

## Syntax

Define columns in frontmatter:
```yaml
---
columns: [item, todo, priority, effort, deadline]
---
```

Or with a `#columns` tag on a heading:
```
## Sprint Board #columns item,todo,priority,effort
```

## Command

```
morg columns <files>
```

Output:
```
| Item              | Status | Priority | Effort | Deadline   |
|-------------------|--------|----------|--------|------------|
| Fix parser bug    | TODO   | A        | 2h     | 2026-04-10 |
| Write tests       | TODO   | B        | 4h     | 2026-04-15 |
| Ship v0.1         | DONE   | A        | 8h     |            |
```

## Implementation

- **New command**: `morg columns` — walks headings, extracts specified properties/tags, renders as table.
- **Property extraction**: For each heading, look up properties from drawer, tags from inline content, and derive values (effort from `#effort`, priority from `#priority`, etc.).
- **Output**: Plain text table or TSV for piping.

### Files to modify
- `crates/morg-mode/src/cli.rs`
- `crates/morg-mode/src/commands/columns.rs` (new)
- `crates/morg-mode/src/collect.rs`
