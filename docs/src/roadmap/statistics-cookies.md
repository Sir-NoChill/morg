# Statistics Cookies

## Summary

A `#progress` tag on a heading automatically computes completion stats from child checkboxes and todo tags.

## Syntax

```
## Sprint Tasks #progress
- [x] Task A
- [ ] Task B
- [x] Task C
```

`morg todos` would show: `Sprint Tasks [2/3] (67%)`

## Implementation

- **`tags.rs`**: Add `Progress` variant to `TagKind` (no arguments — computed at query time).
- **`todos` command**: When a heading has `#progress`, count child open/closed items and display stats.
- **`collect.rs`**: Add helper to count child todo/done/checkbox states under a heading.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/todos.rs`
- `crates/morg-mode/src/collect.rs`
