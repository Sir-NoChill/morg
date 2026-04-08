# morg-mode

A markdown-idiomatic replacement for Emacs org-mode, built in Rust.

morg-mode extends standard markdown with a `#tag` system for metadata, time tracking, task management, and literate programming. It operates purely on source files and is designed for use with any editor (primarily Neovim).

## Design Principles

- **Markdown-first**: Standard markdown is valid morg. Extensions use `#tag` syntax that degrades gracefully.
- **File-level**: All features operate on `.md`/`.morg` files. No editor runtime required.
- **Strongly typed**: The parser produces a fully typed AST. Known tags are parsed and validated; unknown tags are preserved.
- **Lenient**: Parse errors are collected, not fatal. Partial documents still produce useful ASTs.

## Architecture

The project is a Cargo workspace with two crates:

- **`morg-parser`** -- hand-written line-oriented parser with inline sub-parsing
- **`morg-mode`** -- CLI binary with subcommands for tangling, time tracking, todos, agenda, frontmatter, and iCal export

## Tag Syntax

Tags are prefixed with `#` (no space after -- a space after `#` makes it a heading). To use a literal `#`, escape with `\#`.

Tags can appear inline (`text #todo fix this`) or block-level (`#deadline 2026-04-10` on its own line).

## CLI Commands

```
morg tangle <files...> [--output-dir <dir>]
morg time <files...> [--project <name>]
morg todos <files...>
morg agenda <files...>
morg frontmatter <files...>
morg ical <files...> [-o output.ics]
```
