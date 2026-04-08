use std::path::{Path, PathBuf};

use icalendar::{Calendar, Component, TodoStatus};

use morg_parser::tags::{Repeater, TagKind, Timestamp};

use crate::collect::{self, TagContext};

pub fn run(
    paths: &[PathBuf],
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);

    let mut calendar = Calendar::new();
    calendar.name("morg-mode");
    let mut count = 0;

    for pf in &parsed {
        collect::walk_tags(&pf.path, &pf.document, |ctx: TagContext<'_>| {
            let heading = ctx.parent_heading.map(heading_text);
            let location = format!("{}:{}", ctx.file.display(), ctx.tag.span.line);

            match &ctx.tag.kind {
                TagKind::Todo { text } => {
                    let summary = build_summary(text.as_deref(), heading.as_deref());
                    let mut todo = icalendar::Todo::new();
                    todo.summary(&summary);
                    todo.description(&format!("Source: {location}"));
                    todo.status(TodoStatus::NeedsAction);
                    todo.add_property("X-MORG-SOURCE", &location);
                    calendar.push(todo.done());
                    count += 1;
                }
                TagKind::Done { text } => {
                    let summary = build_summary(text.as_deref(), heading.as_deref());
                    let mut todo = icalendar::Todo::new();
                    todo.summary(&summary);
                    todo.description(&format!("Source: {location}"));
                    todo.status(TodoStatus::Completed);
                    todo.add_property("PERCENT-COMPLETE", "100");
                    todo.add_property("X-MORG-SOURCE", &location);
                    calendar.push(todo.done());
                    count += 1;
                }
                TagKind::Deadline { date, repeater, .. } => {
                    let summary = format!(
                        "DEADLINE{}",
                        heading
                            .as_deref()
                            .map(|h| format!(": {h}"))
                            .unwrap_or_default()
                    );
                    let event = build_event(&summary, *date, &location, repeater.as_ref());
                    calendar.push(event);
                    count += 1;
                }
                TagKind::Scheduled { date, repeater, .. } => {
                    let summary = format!(
                        "SCHEDULED{}",
                        heading
                            .as_deref()
                            .map(|h| format!(": {h}"))
                            .unwrap_or_default()
                    );
                    let event = build_event(&summary, *date, &location, repeater.as_ref());
                    calendar.push(event);
                    count += 1;
                }
                TagKind::Date { date, repeater } => {
                    let summary = format!(
                        "Date{}",
                        heading
                            .as_deref()
                            .map(|h| format!(": {h}"))
                            .unwrap_or_default()
                    );
                    let event = build_event(&summary, *date, &location, repeater.as_ref());
                    calendar.push(event);
                    count += 1;
                }
                TagKind::Event { date, repeater, description } => {
                    let summary = description
                        .clone()
                        .or(heading.clone())
                        .unwrap_or_else(|| "Event".to_string());
                    let event = build_event(&summary, *date, &location, repeater.as_ref());
                    calendar.push(event);
                    count += 1;
                }
                _ => {}
            }
        });
    }

    if count == 0 {
        println!("No exportable entries found.");
        return Ok(());
    }

    let ics = calendar.done().to_string();

    match output {
        Some(path) => {
            std::fs::write(path, &ics)?;
            eprintln!("Wrote {count} entries to {}", path.display());
        }
        None => {
            print!("{ics}");
            eprintln!("---\n{count} entries exported.");
        }
    }

    Ok(())
}

fn build_event(
    summary: &str,
    ts: Timestamp,
    location: &str,
    repeater: Option<&Repeater>,
) -> icalendar::Event {
    let mut event = icalendar::Event::new();
    event.summary(summary);
    event.description(&format!("Source: {location}"));
    match ts {
        Timestamp::Date(d) => {
            event.add_property("DTSTART;VALUE=DATE", &d.format("%Y%m%d").to_string());
        }
        Timestamp::DateTime(dt) => {
            event.add_property("DTSTART", &dt.format("%Y%m%dT%H%M%S").to_string());
        }
    }
    event.add_property("X-MORG-SOURCE", location);
    if let Some(r) = repeater {
        event.add_property("RRULE", &r.as_rrule());
    }
    event.done()
}

fn build_summary(text: Option<&str>, heading: Option<&str>) -> String {
    match (text.filter(|t| !t.is_empty()), heading) {
        (Some(t), Some(h)) => format!("{t} ({h})"),
        (Some(t), None) => t.to_string(),
        (None, Some(h)) => h.to_string(),
        (None, None) => "Untitled".to_string(),
    }
}

fn heading_text(h: &morg_parser::ast::Heading) -> String {
    h.content.plain_text()
}
