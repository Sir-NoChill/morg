# Effort Estimates

## Summary

An `#effort` tag on a heading or task specifies the estimated time to complete.

## Syntax

```
## Refactor Parser
#effort 4h
#clock 2h30m
```

Or inline: `#todo fix bug #effort 30m`

## Implementation

- **`tags.rs`**: Add `Effort { minutes: u64 }` variant. Reuse the existing `parse_duration` function.
- **`time` command**: Show effort vs actual in the report (estimated / clocked / remaining).
- **`todos` command**: Display effort alongside todo items.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/time.rs`
- `crates/morg-mode/src/commands/todos.rs`
