# Capture Templates

## Summary

Quick-capture new entries into target files/headings using configurable templates. Equivalent to org-capture.

## Syntax

Templates defined in a config file (`~/.config/morg/capture.yaml`):
```yaml
templates:
  todo:
    target: ~/notes/inbox.md
    template: "- [ ] {input}"
  journal:
    target: ~/notes/journal.md
    heading: "## {date}"
    template: "{input}"
  meeting:
    target: ~/notes/meetings.md
    template: |
      ## {input}
      #scheduled {date}
      #clock-in {datetime}
```

## Commands

```
morg capture todo "Buy groceries"
morg capture journal "Interesting insight about X"
morg capture meeting "Standup"
```

## Implementation

- **Config**: Parse capture templates from YAML config file.
- **New command**: `morg capture <template-name> <input>` — renders template, appends to target file under target heading.
- **Template expansion**: Replace `{input}`, `{date}`, `{datetime}`, `{time}` placeholders.

### Files to modify
- `crates/morg-mode/src/cli.rs`
- `crates/morg-mode/src/commands/capture.rs` (new)
- `crates/morg-mode/src/config.rs` (new)
