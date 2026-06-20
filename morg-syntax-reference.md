# Morg Syntax Reference

Morg is a Markdown-derived format with org-mode-style **tag semantics** layered
on top. Files use `.md`, `.morg`, or `.markdown` extensions. The parser is
CommonMark-compatible for most block/inline constructs, with the following
extensions.

---

## 1. Document Structure

```
[frontmatter]
[blocks...]
```

Parsing is two-phase: a block tokenizer classifies each line, then an inline
tokenizer is called on demand per block. A def-ref resolution pass runs after
parsing to resolve link references.

---

## 2. Frontmatter

YAML block at the very top of the file, delimited by `---` on its own line.
Must start on line 1.

```
---
title: My Document
author: Alice
tags: [work, project]
---
```

- Parsed as `serde_yaml::Value` and stored in `Document.frontmatter`.
- A `---` line that appears *after* frontmatter is closed becomes a horizontal
  rule instead.
- Custom TODO workflow states can be declared here (consumed by the
  `CustomState` tag kind).

---

## 3. Block Elements

### 3.1 Headings

**ATX style** — `#` through `######` followed by a space:

```
# H1
## H2
### H3
#### H4
##### H5
###### H6
```

Up to 3 leading spaces are allowed. 4+ spaces make it an indented code block
instead.

**Setext style** — underline the text with `=` (H1) or `-` (H2):

```
My Heading
==========

Subheading
----------
```

Rules: 0–3 leading spaces on the underline; all underline chars must be the
same; not recognized inside frontmatter.

**Property drawer** — a heading may be immediately followed (after optional
blank lines) by a `#properties`…`#end` block:

```
## My Task

#properties
id = abc-123
effort = 2h30m
owner = alice
#end

Content here.
```

Properties are `key = value` pairs, stored as `HashMap<String, String>`.

---

### 3.2 Paragraphs

Consecutive non-blank text lines form a paragraph. Inline content is parsed
within each paragraph.

---

### 3.3 Code Blocks

**Fenced** — opening fence of 3+ backticks or tildes, optional info string:

```
```rust #tangle file=main.rs
fn main() {}
```
```

The closing fence must use the same character and be at least as long as the
opening fence.

**Info string fields** (space-separated, on the opening fence line):

| Field | Description |
|---|---|
| `lang` | First token not starting with `#` and not containing `=`; the language identifier. |
| `#tagname` | Zero or more tags (see Section 5). |
| `key=value` | Zero or more key-value attributes. |

Common attribute: `file=path/to/output` (used with `#tangle`).

**Indented code** — 4+ leading spaces or a leading tab; no info string, no
tags, no attributes.

```
    fn hello() {
        println!("hi");
    }
```

---

### 3.4 Block Tags

A line starting with `#name` (where `name` begins with an alphanumeric or
`_` character) is a **block-level tag**:

```
#deadline 2026-04-10
#todo Fix the parser
#clock 1h30m
```

Syntax: `#<name> [arg]`

- `name` — alphanumeric, `-`, `_` characters.
- `arg` — the rest of the line after a single space (optional).

Known keywords are parsed into structured `TagKind` values (see Section 5).
Unknown names produce `TagKind::Unknown { name, value }`.

`#properties` and `#end` are structural tokens used by property drawers and
are not valid standalone block tags.

---

### 3.5 Callouts

Obsidian/GitHub-flavored callout syntax:

```
> [!note]
> This is a note.

> [!warning][#priority A key=val]
> A warning with metadata.
```

Syntax: `> [!type]` optionally followed by `[metadata]` on the same line.

- `type` — lowercase string (`note`, `warning`, `tip`, `info`, etc.).
- `metadata` — space-separated `#tags` and `key=value` attributes.
- Continuation lines start with `> `.
- The body is recursively parsed as a full morg document.

---

### 3.6 Tables

GFM-style pipe tables:

```
| Name  | Age | Score |
|-------|:---:|------:|
| Alice | 30  |  99.5 |
| Bob   | 25  |  87.0 |
```

- Header row, then an optional separator row for alignment.
- Separator cells: `---` (none), `:---` (left), `---:` (right), `:---:` (center).
- Leading/trailing `|` are optional but recommended.
- Cell content is inline-parsed.

---

### 3.7 Lists

**Unordered** — markers `- `, `+ ` (at any indent), or `* ` (indented only;
bare `* ` at column 0 is a horizontal rule):

```
- Item one
- Item two
  - Nested item (2-space indent)
    - Deeper nesting
+ Also unordered
```

**Ordered** — `N. ` where N is one or more digits:

```
1. First
2. Second
   1. Nested ordered
```

**Checkboxes** — prefix the item text with `[ ] ` (unchecked) or `[x] ` /
`[X] ` (checked):

```
- [ ] Unchecked task
- [x] Done task
- [X] Also done
- Regular item (no checkbox)
```

**Description lists** — use ` :: ` as a term/description separator:

```
- Term :: The definition of that term
- Another term :: Its description
```

Nesting is determined by indentation: items indented more than their predecessor
become children of that item.

---

### 3.8 Horizontal Rules

Three or more `-`, `*`, or `_` characters (spaces allowed between):

```
---
***
___
- - -
* * *
```

---

### 3.9 HTML Blocks

A line starting with `<tagname` opens an HTML block; it continues until the
matching `</tagname>` or a blank line:

```html
<div class="box">
  <p>Raw HTML content</p>
</div>
```

Void elements (`br`, `hr`, `img`, `input`, etc.) are self-closing. The raw
text is stored verbatim.

---

### 3.10 Comments

**Line comment:**

```
// This entire line is a comment
```

**Block comment:**

```
/*
Multi-line
comment block
*/
```

Comments are parsed into `Block::Comment` and can be filtered out by consumers.

---

### 3.11 Footnote Definitions

```
[^1]: The content of footnote one.
[^note]: Another footnote with a named label.
```

Syntax: `[^label]: content` where `label` contains no spaces.

---

### 3.12 Link Reference Definitions

```
[foo]: /url/path
[bar]: https://example.com "Optional title"
[baz]: <https://example.com> "Title in angle-bracket URL"
```

- Not rendered; consumed by the def-ref pass to build a symbol table.
- First definition wins (CommonMark rule).
- Labels are matched case-insensitively with whitespace collapsed.
- Title may be in double quotes, single quotes, or parentheses.

---

## 4. Inline Elements

### 4.1 Text and Escapes

Plain text. Backslash-escape these special characters to treat them literally:

```
\#  \[  \*  \~  \`
```

---

### 4.2 Emphasis

```
**bold text**
*italic text*
~~strikethrough~~
```

These nest within each other and within other inline constructs.

---

### 4.3 Inline Code

Single or multiple backticks; the fence length must match:

```
`code here`
``code with a `backtick` inside``
``` `` ```
```

A single leading/trailing space is stripped if both are present and the content
is not all spaces.

---

### 4.4 Links

**Inline link:**

```
[display text](url)
[display text](url "Title string")
[display text](url "Title" [#tag key=val])
```

- Title is optional, in double quotes.
- After the title, an optional `[metadata]` block holds space-separated
  `#tags` and `key=value` attributes.

**Autolink:**

```
<https://example.com>
<user@example.com>
```

URLs must contain `://`; emails must contain `@` (not at start/end). Rendered
as a link with text equal to the raw content.

**Reference link:**

```
[display text][label]   ← full reference
[label][]               ← collapsed reference (label = display text)
[label]                 ← shortcut reference
```

Resolved against link reference definitions in a post-parse pass. Unresolved
references are left as-is.

---

### 4.5 Images

```
![alt text](image.png)
![photo](pic.jpg "My Photo")
```

Same syntax as inline links, prefixed with `!`. No metadata block.

---

### 4.6 Footnote References

```
See the note[^1] for details.
Also see[^note] here.
```

---

### 4.7 Inline Tags

`#tagname` (and optional argument text) can appear inline within paragraphs,
headings, list items, table cells, and link text:

```
Fix the #todo bug in the parser.
Meeting at 9am #event 2026-06-15T09:00 Weekly standup
```

Inline tag argument: the text immediately following a space after `#name`,
consumed until the next `#tag` or end of inline context. Use `\#` to emit a
literal `#` that won't start a tag.

---

### 4.8 Hard Line Breaks

Within a paragraph, a hard break is inserted when:

- A line ends with **2 or more spaces** followed by a newline.
- A line ends with a **backslash** (`\`) before the newline.

Soft line breaks (single newline with no trailing spaces) are rendered as
a space by most consumers.

---

## 5. Tag System

Tags (`#name arg`) are the core semantic extension. They appear both as
block-level statements and inline within content. The same keyword table
drives both contexts.

### 5.1 TODO Keywords

```
#todo [description]
#done [description]
```

- `text` — optional arbitrary string attached to the state.
- Custom workflow states defined in frontmatter produce `TagKind::CustomState`.

---

### 5.2 Timestamps

**Deadline:**

```
#deadline 2026-04-10
#deadline 2026-04-10T14:00
#deadline 2026-04-10 +1w
#deadline 2026-04-10T14:00 +1w -3d
```

**Scheduled:**

```
#scheduled 2026-04-05
#scheduled 2026-04-05 +2m
```

**Generic date:**

```
#date 2026-01-01
#date 2026-01-01 +1y
```

**Timestamp formats:**

| Format | Example |
|---|---|
| Date only | `2026-04-10` |
| Date + time | `2026-04-10T14:00` |
| Date + time + seconds | `2026-04-10T14:00:00` |

**Repeater** (optional, after timestamp): `+N<unit>`

| Unit char | Frequency |
|---|---|
| `d` / `D` | Daily |
| `w` / `W` | Weekly |
| `m` / `M` | Monthly |
| `y` / `Y` | Yearly |

Example: `+1w` = every week, `+2m` = every 2 months.

**Warning period** (optional, after repeater): `-Nd` (days before deadline to
start warning).

---

### 5.3 Events

```
#event 2026-04-10 Team meeting
#event 2026-04-10T09:00/2026-04-10T17:00 Workshop
#event 2026-04-10/2026-04-12 Conference
#event 2026-01-01 +1y New Year
#event 2026-04-10/2026-04-12 +1y Annual Conference
```

Full syntax: `#event START[/END] [+repeater] [description]`

- `START` and optional `END` — date or datetime.
- `/` separator between start and end dates.
- Repeater and description come after the date range.
- `description` — the remainder of the argument after the dates and repeater.

---

### 5.4 Time Tracking

```
#clock-in 2026-04-03T09:00
#clock-out 2026-04-03T10:30
#clock 2026-04-03T09:00/2026-04-03T10:30
#clock 1h30m
```

- `#clock-in` / `#clock-out` — pair of datetime stamps (ISO 8601, `T` separator).
- `#clock DATETIME/DATETIME` — completed clock range.
- `#clock DURATION` — recorded duration.

**Duration format:** `NhNm` where `N` is digits, `h` = hours, `m` = minutes.

| Example | Minutes |
|---|---|
| `2h` | 120 |
| `45m` | 45 |
| `1h30m` | 90 |

---

### 5.5 Metadata Tags

```
#priority A
#priority B
#priority C
#priority X    ← custom single-char priority

#effort 2h30m

#closed 2026-04-10T15:30

#archive
#progress
```

- `#priority` — built-in levels A, B, C; any single alphanumeric character is
  accepted as a custom level.
- `#effort` — estimated effort in duration format.
- `#closed` — timestamp when the item was closed (datetime required).
- `#archive` — marks the containing heading as archived; archived items are
  skipped by agenda/export commands.
- `#progress` — marks an item as in-progress.

---

### 5.6 Tangling

```
```python #tangle file=scripts/hello.py
print("hello")
```
```

`#tangle` on a code block's info string marks it for extraction. The `file`
attribute specifies the output path. The tangle command collects all such
blocks and writes their bodies to the respective files.

---

### 5.7 Unknown / Custom Tags

Any `#name` not in the keyword list is accepted and stored as:

```
TagKind::Unknown { name: String, value: Option<String> }
```

This allows user-defined tags that downstream tools can handle. Example:

```
#project alpha
#context office
```

---

## 6. Property Drawers

Attached to the heading immediately above (blank lines between heading and
drawer are allowed):

```
## Task Title

#properties
id = task-001
owner = alice
estimate = 3h
custom-field = anything
#end
```

- Delimited by `#properties` and `#end` on their own lines.
- Each line is `key = value` (whitespace around `=` is trimmed).
- Keys and values are stored as plain strings.
- An unclosed drawer is a parse error; the parser recovers and continues.

---

## 7. Parse Errors and Recovery

The parser is recoverable: errors are collected in `ParseResult.errors` but
parsing continues. Key error cases:

| Situation | Recovery |
|---|---|
| Unclosed frontmatter | `---` required; rest of file treated as body |
| Unclosed code fence | Body collected until EOF |
| Unclosed HTML block | Collected until blank line or EOF |
| Unclosed `#properties` | Error recorded; drawer ends at EOF |
| Invalid YAML frontmatter | Error recorded; no frontmatter in AST |
| Bad timestamp argument | Tag stored as `TagKind::Unknown` |
| Unresolved link reference | `InlineSegment::LinkRef` left in AST |

---

## 8. Precedence and Edge Cases

- `*` at column 0 with a space is a horizontal rule candidate, not a list marker.
- `---` at the very top of a file opens frontmatter; anywhere else it is a
  horizontal rule (after frontmatter is closed).
- Inline `#` followed by a space or end-of-input is plain text, not a tag.
- 4+ leading spaces override heading/list detection — the line becomes an
  indented code block.
- `[^...` is always a footnote (def or ref), never a link reference.
- Link resolution: inline `[text](url)` takes precedence over `[text][label]`
  when `(` follows `]`.
- Setext headings are not recognized inside frontmatter.
