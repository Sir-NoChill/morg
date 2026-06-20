# morg-parser

A hand-written lexer and recursive-descent parser for morg-mode documents. Produces a typed AST from markdown extended with `#tag` syntax for metadata, time tracking, task management, and literate programming.

## CommonMark Compliance

morg-parser supports the most commonly used subset of CommonMark. The sections below document every known deviation from the [CommonMark 0.31.2 specification](https://spec.commonmark.org/0.31.2/), grouped by category.

### Intentional deviations

These constructs are handled differently from CommonMark by design, to support morg-mode features or avoid ambiguity with the `#tag` syntax.

#### `*` list marker requires indentation

CommonMark treats `* foo` as an unordered list item at any indentation level. morg-parser requires `indent > 0` for the `*` marker to avoid ambiguity with emphasis (`*italic*`) and horizontal rules (`***`). At zero indentation, use `-` or `+` instead:

```markdown
- item one      (works at any indent)
+ item two      (works at any indent)
  * sub-item    (* requires indent > 0)
```

#### Limited backslash escapes

CommonMark defines backslash escapes for all ASCII punctuation characters. morg-parser only supports escaping five characters: `#`, `[`, `*`, `~`, and `` ` ``. Backslashes before other characters (e.g. `\)`, `\]`, `\!`) are preserved literally.

#### Plain blockquotes are callouts only

CommonMark `> text` is a blockquote. morg-parser only recognises the `> [!type]` callout syntax. A plain `> text` line without a callout marker is parsed as a `BlockquoteContinuation` token and may be absorbed into a preceding callout or treated as paragraph text.

#### `#text` is a tag, not a heading attempt

CommonMark treats `#text` (no space after `#`) as a paragraph. morg-parser treats `#word` as a tag if the character after `#` is alphanumeric or `_`. To produce a literal `#`, escape it: `\#`.

### Conformant

The following CommonMark constructs are supported and behave as specified:

- ATX headings (`#` through `######`, with required space or EOL after hashes)
- Setext headings (`===` underline for h1, `---` underline for h2)
- Fenced code blocks (`` ``` `` and `~~~`, with info strings, fence length matching)
- Indented code blocks (4+ spaces or tab prefix, cannot interrupt paragraphs)
- Thematic breaks / horizontal rules (`***`, `___`, `---`, `----`, with optional spaces)
- Unordered lists (`-`, `+` markers; `*` with indent)
- Ordered lists (`1.`, `2.`, etc.)
- Tables (pipe syntax with alignment via `:---:`)
- Inline emphasis (`*italic*`, `**bold**`, `~~strikethrough~~`)
- Inline code spans (single and multi-backtick, with CommonMark space stripping)
- Inline links (`[text](url)`, with optional `"title"`)
- Link reference definitions (`[label]: url "title"`) with shortcut (`[label]`), collapsed (`[label][]`), and full (`[text][label]`) reference forms
- Images (`![alt](url)`, with optional `"title"`)
- Autolinks (`<https://example.com>`, `<user@example.com>`)
- Hard line breaks (two trailing spaces or trailing `\`)
- Backslash escapes for `#`, `[`, `*`, `~`, `` ` ``
- Blank lines as block separators
- YAML frontmatter (`---` must be the very first line of the file)
