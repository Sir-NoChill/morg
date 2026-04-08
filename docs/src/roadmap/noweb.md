# Noweb References

## Summary

Support org-babel-style noweb references (`<<block-name>>`) inside code blocks, allowing code block composition by reference. Named blocks are identified by a `name=` attribute on the code fence. This also replaces the need for a separate include directive.

## Syntax

### Naming a block

````
```rust #tangle file=main.rs name=imports
use std::io;
```
````

### Referencing a block

````
```rust #tangle file=main.rs
<<imports>>

fn main() {
    <<main-body>>
}
```
````

### Include via link

For including content from external files:

```
[](./utils.rs "include" [#tangle file=main.rs])
```

## Implementation

### Parser changes

- **`ast.rs`**: The `name=` attribute already works via the existing `attributes: HashMap` on `CodeBlock`. No AST change needed for naming. For noweb references, they are resolved at tangle time, not parse time -- the parser just stores the raw body.

### CLI changes

- **`tangle.rs`**: After collecting all tangle blocks, build a name-to-body map from blocks with `name=` attributes. Then do a second pass over all block bodies, replacing `<<name>>` patterns with the corresponding named block content. Handle:
  - Indentation preservation (the `<<ref>>` indent is prepended to each line of the expansion)
  - Missing references (warn, leave the `<<ref>>` in place)
  - Circular references (detect and error)
  - Recursive expansion (references within expanded blocks)

### Files to modify

- `crates/morg-mode/src/commands/tangle.rs` -- noweb expansion logic
- `crates/morg-mode/src/commands/tangle.rs` -- include link handling
