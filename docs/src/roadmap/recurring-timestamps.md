# Recurring Timestamps

## Summary

Add repeater suffixes to date-bearing tags so that deadlines, events, and scheduled items can express recurrence.

## Syntax

Append a repeater after the date: `+Nd`, `+Nw`, `+Nm`, `+Ny` (days, weeks, months, years).

```
#deadline 2026-04-03 +1w
#event 2026-01-01 +1y New Year
#date 2026-04-01 +1m
```

## Implementation

### Parser changes

- **`tags.rs`**: Add `Repeater` struct with `interval: u32` and `unit: RepeaterUnit` enum (`Day`, `Week`, `Month`, `Year`). Add `repeater: Option<Repeater>` to `Deadline`, `Date`, `Event`, and `Scheduled` tag variants.
- **Tag argument parsing**: After parsing the date, check for `+Nunit` suffix before consuming remaining text.

### CLI changes

- **`agenda`**: When listing entries, if a repeater is present, generate occurrences up to a configurable horizon (default: 90 days from today).
- **`ical`**: Emit `RRULE` property on recurring events (e.g., `RRULE:FREQ=WEEKLY;INTERVAL=1`).

### Files to modify

- `crates/morg-parser/src/tags.rs` -- Repeater type + parsing
- `crates/morg-mode/src/commands/agenda.rs` -- occurrence expansion
- `crates/morg-mode/src/commands/ical.rs` -- RRULE generation
