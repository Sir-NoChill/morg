# Tag Inheritance

## Summary

Tags on a heading are inherited by all sub-headings beneath it, mirroring org-mode's tag inheritance. This allows tagging a project heading with `#project backend` and having all children automatically carry that context.

## Behavior

```markdown
## Backend #project backend
### API Routes
#todo add pagination
```

When querying todos, the "add pagination" entry inherits `#project backend` from its ancestor heading.

## Implementation

- **`collect.rs`**: Maintain a stack of heading tags during AST walking. When entering a child heading, push parent tags. `TagContext` gains an `inherited_tags: Vec<&Tag>` field.
- **`todos` command**: Display inherited project/category tags alongside todo items.
- **`time` command**: Use inherited tags for project filtering (not just heading text match).
- **`agenda` command**: Show inherited tags on agenda entries.

### Files to modify
- `crates/morg-mode/src/collect.rs`
- `crates/morg-mode/src/commands/todos.rs`
- `crates/morg-mode/src/commands/time.rs`
- `crates/morg-mode/src/commands/agenda.rs`
