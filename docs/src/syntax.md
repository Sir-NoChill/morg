# Syntax Reference

## Tags

All morg-mode metadata uses `#tag` syntax. A `#` followed immediately by an alphanumeric character (no space) is a tag. `# ` with a space is a heading.

### Block-level tags

A tag on its own line is block-level. The argument extends to end of line.

```
#todo refactor the parser
#deadline 2026-04-10
#clock-in 2026-04-03T09:00
#clock-out 2026-04-03T10:30
#clock 1h30m
#event 2026-04-10 Team meeting
```

### Inline tags

Tags can appear within text. The argument extends to the next `#` or end of line.

```
Some text #todo fix this before #deadline 2026-04-15
```

### Escaping

Use `\#` for a literal hash: `Price is \#100`.

## Code Blocks

Standard markdown fences with tags and attributes on the info string:

````
```rust #tangle file=src/main.rs
fn main() {}
```
````

## Callouts

GitHub/Obsidian-style callouts with optional metadata:

```
> [!note][#tangle file=output.txt]
> Content here.
```

## Frontmatter

YAML between `---` delimiters at the start of a file:

```
---
title: My Document
tags: [rust, morg]
---
```

## Tables

Standard markdown pipe tables with alignment:

```
| Left | Center | Right |
|:-----|:------:|------:|
| a    |   b    |     c |
```
