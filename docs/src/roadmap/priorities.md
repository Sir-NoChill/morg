# Priority Tags

## Summary

Add a `#priority` tag for task prioritization, supporting A/B/C levels (extensible to numeric).

## Syntax

```
#todo fix critical bug #priority A
#todo update docs #priority C
```

Or as a block-level tag associated with the nearest heading:
```
## Fix Critical Bug
#priority A
#deadline 2026-04-10
```

## Implementation

- **`tags.rs`**: Add `Priority { level: PriorityLevel }` variant. `PriorityLevel` enum: `A`, `B`, `C` (or `Custom(String)` for extensibility).
- **`todos` command**: Display priority alongside status. Sort by priority when listing.
- **`agenda` command**: Show priority on deadline/scheduled entries.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/todos.rs`
- `crates/morg-mode/src/commands/agenda.rs`
