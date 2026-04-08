# Custom TODO Sequences

## Summary

Allow users to define custom TODO workflow states beyond the built-in `#todo`/`#done`. Defined in YAML frontmatter, applied per-file or globally.

## Syntax

Frontmatter definition:
```yaml
---
todo_sequences:
  - [TODO, NEXT, WAIT, "|", DONE, CANCELLED]
  - [IDEA, DRAFT, REVIEW, "|", PUBLISHED]
---
```

Usage in text:
```
#NEXT write the parser tests
#WAIT blocked on upstream PR
#CANCELLED no longer needed
```

States before `|` are "open" (incomplete), states after are "closed" (complete).

## Implementation

- **`tags.rs`**: Add `CustomState { name: String, sequence_index: usize, is_done: bool }` variant to `TagKind`. During parsing, if a tag name matches a defined sequence state, produce this variant instead of `Unknown`.
- **`parser.rs`**: Accept a `ParseConfig` with todo sequences. Pass from CLI → parser.
- **`todos` command**: Show custom states with their status (open/closed). Group by sequence.
- **`ical` command**: Map open states to `NEEDS-ACTION`, closed to `COMPLETED`.

### Files to modify
- `crates/morg-parser/src/tags.rs`
- `crates/morg-parser/src/parser.rs`
- `crates/morg-mode/src/commands/todos.rs`
- `crates/morg-mode/src/commands/ical.rs`
