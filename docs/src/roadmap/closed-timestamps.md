# CLOSED Timestamps

## Summary

A `#closed` tag records when an item was completed, for logging and reporting.

## Syntax

```
#done shipped v0.1 #closed 2026-04-03T15:30
```

Or as a block-level tag:
```
#closed 2026-04-03T15:30
```

## Implementation

- **`tags.rs`**: Add `Closed { datetime: NaiveDateTime }` variant.
- **`todos` command**: Display completion date for done items that have `#closed`.
- **`time` command**: Could use closed timestamps for throughput reporting.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/todos.rs`
