# Timestamp Ranges

## Summary

Support date/time ranges for multi-day or multi-hour events.

## Syntax

```
#event 2026-04-10/2026-04-12 Conference
#event 2026-04-10T09:00/2026-04-10T17:00 Workshop
```

Uses `/` separator, consistent with `#clock` range syntax.

## Implementation

- **`tags.rs`**: `Event` variant gains optional end date/time. Parse `/` separator after start date.
- **`agenda` command**: Show range events on each day they span.
- **`ical` command**: Emit `DTEND` for ranged events.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/agenda.rs`
- `crates/morg-mode/src/commands/ical.rs`
