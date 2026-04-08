use std::path::PathBuf;

use morg_parser::ast::{Block, Checkbox, Heading};
use morg_parser::tags::{PriorityLevel, TagKind};

use crate::collect::{self, TagContext};
use crate::report;

pub fn run(paths: &[PathBuf], json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);

    let mut todos: Vec<TodoEntry> = Vec::new();

    for pf in &parsed {
        let sequences = collect::todo_sequences(&pf.document);

        // Collect tag-based todos
        let mut current_priority: Option<PriorityLevel> = None;
        let mut current_effort: Option<u64> = None;

        collect::walk_tags(&pf.path, &pf.document, |ctx: TagContext<'_>| {
            if ctx.is_archived {
                return;
            }

            match &ctx.tag.kind {
                TagKind::Priority { level } => {
                    current_priority = Some(*level);
                }
                TagKind::Effort { minutes } => {
                    current_effort = Some(*minutes);
                }
                TagKind::Todo { text } => {
                    todos.push(TodoEntry {
                        status: "TODO",
                        text: text.clone().unwrap_or_default(),
                        location: report::format_location(ctx.file, &ctx.tag.span),
                        heading: ctx.parent_heading.map(heading_text),
                        priority: current_priority,
                        effort: current_effort,
                        inherited_tags: ctx.inherited_tags.clone(),
                    });
                    current_priority = None;
                    current_effort = None;
                }
                TagKind::Done { text } => {
                    todos.push(TodoEntry {
                        status: "DONE",
                        text: text.clone().unwrap_or_default(),
                        location: report::format_location(ctx.file, &ctx.tag.span),
                        heading: ctx.parent_heading.map(heading_text),
                        priority: current_priority,
                        effort: current_effort,
                        inherited_tags: ctx.inherited_tags.clone(),
                    });
                    current_priority = None;
                    current_effort = None;
                }
                TagKind::Unknown { name, value } => {
                    // Check if this matches a custom TODO sequence state
                    if let Some((state_name, is_done)) = collect::match_custom_state(name, &sequences) {
                        let status = if is_done { "DONE" } else { "TODO" };
                        todos.push(TodoEntry {
                            status,
                            text: value.clone().unwrap_or_default(),
                            location: report::format_location(ctx.file, &ctx.tag.span),
                            heading: ctx.parent_heading.map(heading_text),
                            priority: current_priority,
                            effort: current_effort,
                            inherited_tags: ctx.inherited_tags.clone(),
                        });
                        // Override the status display with the actual state name
                        if let Some(last) = todos.last_mut() {
                            last.status = if is_done { "DONE" } else { "TODO" };
                            // Prepend state name to text for display
                            let prefix = format!("[{state_name}] ");
                            last.text = format!("{prefix}{}", last.text);
                        }
                        current_priority = None;
                        current_effort = None;
                    }
                }
                _ => {}
            }
        });

        // Collect checkbox-based todos from lists
        let mut current_heading: Option<&Heading> = None;
        collect_checkboxes(&pf.document.children, &pf.path, &mut current_heading, &mut todos);
    }

    // Sort: open items first, then by priority (A before B before C), then by location
    todos.sort_by(|a, b| {
        let status_ord = |s: &str| if s == "TODO" { 0 } else { 1 };
        status_ord(a.status)
            .cmp(&status_ord(b.status))
            .then_with(|| a.priority.cmp(&b.priority))
    });

    if json {
        let items: Vec<serde_json::Value> = todos.iter().map(|e| {
            let (file, lnum) = parse_location(&e.location);
            serde_json::json!({
                "status": e.status,
                "text": e.text,
                "file": file,
                "line": lnum,
                "heading": e.heading,
                "priority": e.priority.map(|p| p.to_string()),
                "effort": e.effort.map(|m| report::format_duration_minutes(m)),
                "tags": e.inherited_tags,
            })
        }).collect();
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if todos.is_empty() {
        println!("No TODOs found.");
        return Ok(());
    }

    for entry in &todos {
        let heading_ctx = entry
            .heading
            .as_deref()
            .map(|h| format!(" ({h})"))
            .unwrap_or_default();
        let pri = entry
            .priority
            .map(|p| format!(" [{p}]"))
            .unwrap_or_default();
        let eff = entry
            .effort
            .map(|m| format!(" ~{}", report::format_duration_minutes(m)))
            .unwrap_or_default();
        let tags = if entry.inherited_tags.is_empty() {
            String::new()
        } else {
            format!(" :{}", entry.inherited_tags.join(":"))
        };
        println!(
            "[{status}]{pri}{eff} {text}{heading_ctx}{tags}  -- {loc}",
            status = entry.status,
            text = entry.text,
            loc = entry.location,
        );
    }

    let open = todos.iter().filter(|t| t.status == "TODO").count();
    let done = todos.iter().filter(|t| t.status == "DONE").count();
    println!("\n{open} open, {done} done, {} total", todos.len());

    Ok(())
}

struct TodoEntry {
    status: &'static str,
    text: String,
    location: String,
    heading: Option<String>,
    priority: Option<PriorityLevel>,
    effort: Option<u64>,
    inherited_tags: Vec<String>,
}

fn collect_checkboxes<'a>(
    blocks: &'a [Block],
    file: &std::path::Path,
    current_heading: &mut Option<&'a Heading>,
    todos: &mut Vec<TodoEntry>,
) {
    for block in blocks {
        match block {
            Block::Heading(h) => {
                // Skip archived headings
                if h.content.tags().iter().any(|t| matches!(t.kind, TagKind::Archive)) {
                    continue;
                }
                *current_heading = Some(h);
            }
            Block::List(list) => {
                for item in &list.items {
                    if let Some(checkbox) = &item.checkbox {
                        let status = match checkbox {
                            Checkbox::Unchecked => "TODO",
                            Checkbox::Checked => "DONE",
                        };
                        todos.push(TodoEntry {
                            status,
                            text: item.content.plain_text(),
                            location: report::format_location(file, &item.span),
                            heading: current_heading.map(heading_text),
                            priority: None,
                            effort: None,
                            inherited_tags: Vec::new(),
                        });
                    }
                    collect_checkboxes(&item.children, file, current_heading, todos);
                }
            }
            Block::Callout(c) => {
                collect_checkboxes(&c.content, file, current_heading, todos);
            }
            _ => {}
        }
    }
}

fn heading_text(h: &Heading) -> String {
    h.content.plain_text()
}

fn parse_location(loc: &str) -> (&str, u32) {
    if let Some((file, line)) = loc.rsplit_once(':') {
        (file, line.parse().unwrap_or(0))
    } else {
        (loc, 0)
    }
}
