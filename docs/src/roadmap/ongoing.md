# Ongoing Work

## All Phases Complete (1-4, items 1-36)

| Phase | Items | Description |
|-------|-------|-------------|
| 1 | 1-7 | Core: timestamps, links, markup, noweb, properties, lists |
| 2 | 8-25 | Workflow parity: priorities, inheritance, effort, footnotes, comments, archiving |
| 3 | 26 | Architecture: token macros, lexer, token-consuming parser |
| 4 | 27-36 | CommonMark compliance + polish: autolinks, images, hard breaks, code spans, link refs, HR fix, setext headings, indented code blocks, UTF-8 fix, code fence fix, event timestamp ranges |

### Stats
- **18 CLI commands** with `--format json` support
- **Rust**: 356 tests (52 CLI + 83 parser unit + 221 parser exhaustive), zero warnings
- **Lua**: 30 tests (busted via nlua)
- **Total**: 386 tests

All roadmap items are complete. No remaining features are planned.
