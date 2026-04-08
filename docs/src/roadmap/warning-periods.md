# Warning Periods

## Summary

Deadlines can specify a warning period so they appear in the agenda before they're due.

## Syntax

```
#deadline 2026-04-10 -3d
#deadline 2026-04-10 +1w -5d
```

`-Nd` means "show in agenda N days before the deadline". Can combine with repeater.

## Implementation

- **`tags.rs`**: Add `warning: Option<u32>` (days) to `Deadline` and `Scheduled`. Parse `-Nd` suffix after date and optional repeater.
- **`agenda` command**: When expanding agenda entries, if a deadline has a warning period, generate an additional "upcoming" entry N days before.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/agenda.rs`
