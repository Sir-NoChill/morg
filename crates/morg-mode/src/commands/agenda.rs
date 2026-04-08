use std::path::PathBuf;

use chrono::{Datelike, Duration, NaiveDate};
use morg_parser::tags::{Repeater, RepeaterUnit, TagKind, Timestamp};

use crate::collect::{self, TagContext};
use crate::report;

const DEFAULT_HORIZON_DAYS: i64 = 90;

pub fn run(paths: &[PathBuf], json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);
    let today = chrono::Local::now().date_naive();
    let horizon = today + Duration::days(DEFAULT_HORIZON_DAYS);

    let mut entries: Vec<AgendaEntry> = Vec::new();

    for pf in &parsed {
        collect::walk_tags(&pf.path, &pf.document, |ctx: TagContext<'_>| {
            if ctx.is_archived {
                return;
            }
            let entry = match &ctx.tag.kind {
                TagKind::Deadline {
                    date,
                    repeater,
                    warning,
                } => Some(AgendaEntry {
                    date: *date,
                    kind: "DEADLINE",
                    repeater: *repeater,
                    warning: *warning,
                    description: None,
                    location: report::format_location(ctx.file, &ctx.tag.span),
                }),
                TagKind::Scheduled {
                    date,
                    repeater,
                    warning,
                } => Some(AgendaEntry {
                    date: *date,
                    kind: "SCHEDULED",
                    repeater: *repeater,
                    warning: *warning,
                    description: None,
                    location: report::format_location(ctx.file, &ctx.tag.span),
                }),
                TagKind::Date { date, repeater } => Some(AgendaEntry {
                    date: *date,
                    kind: "DATE",
                    repeater: *repeater,
                    warning: None,
                    description: None,
                    location: report::format_location(ctx.file, &ctx.tag.span),
                }),
                TagKind::Event {
                    date,
                    repeater,
                    description,
                } => Some(AgendaEntry {
                    date: *date,
                    kind: "EVENT",
                    repeater: *repeater,
                    warning: None,
                    description: description.clone(),
                    location: report::format_location(ctx.file, &ctx.tag.span),
                }),
                _ => None,
            };

            if let Some(e) = entry {
                entries.push(e);
            }
        });
    }

    // Expand recurring entries and warning periods into occurrences
    let mut expanded: Vec<ExpandedEntry> = Vec::new();
    for entry in &entries {
        let base_date = entry.date.date();
        match entry.repeater {
            Some(repeater) => {
                let mut date = base_date;
                while date <= horizon {
                    if date >= today {
                        expanded.push(ExpandedEntry {
                            date,
                            timestamp: entry.date,
                            kind: entry.kind,
                            description: entry.description.clone(),
                            location: entry.location.clone(),
                            recurring: true,
                            is_warning: false,
                        });
                    }
                    date = advance_date(date, repeater);
                }
            }
            None => {
                expanded.push(ExpandedEntry {
                    date: base_date,
                    timestamp: entry.date,
                    kind: entry.kind,
                    description: entry.description.clone(),
                    location: entry.location.clone(),
                    recurring: false,
                    is_warning: false,
                });
            }
        }

        // Add warning entries for deadlines/scheduled
        if let Some(days) = entry.warning {
            let warn_date = base_date - Duration::days(days as i64);
            if warn_date >= today && warn_date <= horizon {
                expanded.push(ExpandedEntry {
                    date: warn_date,
                    timestamp: entry.date,
                    kind: entry.kind,
                    description: entry.description.clone(),
                    location: entry.location.clone(),
                    recurring: false,
                    is_warning: true,
                });
            }
        }
    }

    expanded.sort_by_key(|e| e.date);

    if json {
        let items: Vec<serde_json::Value> = expanded
            .iter()
            .map(|e| {
                let (file, lnum) = parse_location(&e.location);
                serde_json::json!({
                    "date": e.date.to_string(),
                    "kind": e.kind,
                    "description": e.description,
                    "file": file,
                    "line": lnum,
                    "recurring": e.recurring,
                    "warning": e.is_warning,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if expanded.is_empty() {
        println!("No agenda entries found.");
        return Ok(());
    }

    for entry in &expanded {
        let desc = entry
            .description
            .as_deref()
            .map(|d| format!("  {d}"))
            .unwrap_or_default();
        let time_str = if entry.timestamp.has_time() {
            format!(" {}", entry.timestamp)
        } else {
            String::new()
        };
        let repeat_marker = if entry.recurring { " (recurring)" } else { "" };
        let warn_marker = if entry.is_warning { " (upcoming)" } else { "" };
        println!(
            "{date}{time_str}  [{kind}]{desc}{repeat_marker}{warn_marker}  -- {loc}",
            date = entry.date,
            kind = entry.kind,
            loc = entry.location,
        );
    }

    println!("\n{} agenda entries.", expanded.len());

    Ok(())
}

struct AgendaEntry {
    date: Timestamp,
    kind: &'static str,
    repeater: Option<Repeater>,
    warning: Option<u32>,
    description: Option<String>,
    location: String,
}

struct ExpandedEntry {
    date: NaiveDate,
    timestamp: Timestamp,
    kind: &'static str,
    description: Option<String>,
    location: String,
    recurring: bool,
    is_warning: bool,
}

fn advance_date(date: NaiveDate, repeater: Repeater) -> NaiveDate {
    let n = repeater.interval as i64;
    match repeater.unit {
        RepeaterUnit::Day => date + Duration::days(n),
        RepeaterUnit::Week => date + Duration::weeks(n),
        RepeaterUnit::Month => {
            let month = date.month0() as i64 + n;
            let year = date.year() + (month / 12) as i32;
            let month = (month % 12) as u32 + 1;
            let day = date.day().min(days_in_month(year, month));
            NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
        }
        RepeaterUnit::Year => {
            let year = date.year() + repeater.interval as i32;
            NaiveDate::from_ymd_opt(year, date.month(), date.day())
                .or_else(|| NaiveDate::from_ymd_opt(year, date.month(), 28))
                .unwrap_or(date)
        }
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(
        if month == 12 { year + 1 } else { year },
        if month == 12 { 1 } else { month + 1 },
        1,
    )
    .map(|d| d.pred_opt().unwrap().day())
    .unwrap_or(28)
}

fn parse_location(loc: &str) -> (&str, u32) {
    if let Some((file, line)) = loc.rsplit_once(':') {
        (file, line.parse().unwrap_or(0))
    } else {
        (loc, 0)
    }
}
