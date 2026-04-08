# Time-of-Day Timestamps

## Summary

Extend date-bearing tags to optionally include a time component.

## Syntax

```
#deadline 2026-04-10T14:00
#scheduled 2026-04-05T09:30
#event 2026-04-10T14:00 Team standup
```

Uses the existing ISO 8601 `T` separator already supported by `#clock-in`/`#clock-out`.

## Implementation

- **`tags.rs`**: Change `Deadline`, `Scheduled`, `Date`, `Event` to carry either `NaiveDate` or `NaiveDateTime`. Use an enum `DateOrDateTime` to represent both.
- **`agenda` command**: Display time when present. Sort by datetime, not just date.
- **`ical` command**: Emit `DTSTART` with time component when present (not all-day).

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/agenda.rs`
- `crates/morg-mode/src/commands/ical.rs`
