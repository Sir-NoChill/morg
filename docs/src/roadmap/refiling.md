# Refiling / Cross-File References

## Summary

Support moving entries between headings/files and maintaining cross-file references via IDs.

## Syntax

References use standard markdown links with an `id:` scheme:
```
See [related task](id:550e8400-e29b-41d4-a716-446655440000)
```

IDs are assigned via property drawers:
```
## My Task
#properties
id = 550e8400-e29b-41d4-a716-446655440000
#end
```

## Commands

- `morg refile <source-heading> <target-file>::<target-heading>` — moves a subtree.
- `morg refs <files>` — lists all cross-file references and validates they resolve.
- `morg id <files>` — assigns UUIDs to headings that lack an `id` property.

## Implementation

- **New commands**: `refile`, `refs`, `id` in CLI.
- **`collect.rs`**: Build an ID → (file, heading) index across parsed files.
- **Link resolution**: Detect `id:` scheme links and resolve against the index.

### Files to modify
- `crates/morg-mode/src/cli.rs`
- `crates/morg-mode/src/commands/refile.rs` (new)
- `crates/morg-mode/src/commands/refs.rs` (new)
- `crates/morg-mode/src/commands/id.rs` (new)
- `crates/morg-mode/src/collect.rs`
