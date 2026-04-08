# SCHEDULED Tag

## Summary

Add a `#scheduled` tag parallel to `#deadline`, representing when work on an item is planned to begin (as opposed to when it's due).

## Syntax

```
#scheduled 2026-04-05
#scheduled 2026-04-05 +1w
```

## Implementation

### Parser changes

- **`tags.rs`**: Add `Scheduled { date: NaiveDate, repeater: Option<Repeater> }` variant to `TagKind`. Parse identically to `Deadline`.

### CLI changes

- **`agenda`**: Include scheduled items in the chronological listing, labeled `[SCHEDULED]`.
- **`ical`**: Export as `VEVENT` with summary prefix "SCHEDULED:".
- **`todos`**: Optionally show scheduled date alongside todo items.

### Files to modify

- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/commands/agenda.rs`
- `crates/morg-mode/src/commands/ical.rs`
