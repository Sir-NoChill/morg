# Archiving

## Summary

An `#archive` tag marks a heading and its subtree as archived — hidden from default views but preserved in the file.

## Syntax

```
## Old Task #archive
```

Or as a block-level tag after a heading:
```
## Completed Sprint
#archive
```

## Behavior

- `morg todos` skips archived headings by default. `--include-archived` flag to show them.
- `morg agenda` skips archived entries by default.
- `morg archive <files>` command moves archived subtrees to a separate `_archive.md` file (or configurable location).

## Implementation

- **`tags.rs`**: Add `Archive` variant to `TagKind`.
- **`collect.rs`**: Track archive state during walking. `TagContext` gains `is_archived: bool`.
- **`commands`**: Default to skipping archived entries. Add `--include-archived` flag.
- **New command**: `morg archive` — extracts archived subtrees into archive files.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-mode/src/collect.rs`
- `crates/morg-mode/src/commands/todos.rs`
- `crates/morg-mode/src/commands/agenda.rs`
- `crates/morg-mode/src/commands/archive.rs` (new)
- `crates/morg-mode/src/cli.rs`
