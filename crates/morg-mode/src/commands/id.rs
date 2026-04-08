use std::path::PathBuf;

use morg_parser::ast::Block;

use crate::collect;

pub fn run(paths: &[PathBuf], dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect::resolve_paths(paths);
    let mut total_assigned = 0;

    for file_path in &files {
        let source = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warning: cannot read {}: {e}", file_path.display());
                continue;
            }
        };

        let result = morg_parser::parser::parse_document(&source);
        let lines: Vec<&str> = source.split('\n').collect();

        // Find headings that lack an id property
        let mut insertions: Vec<(usize, String, String)> = Vec::new(); // (after_line_0idx, heading_text, uuid)

        for block in &result.document.children {
            if let Block::Heading(h) = block {
                let has_id = h.properties.as_ref()
                    .is_some_and(|p| p.entries.contains_key("id"));

                if !has_id {
                    let line_0idx = (h.span.line as usize).saturating_sub(1);
                    let heading_text = h.content.plain_text();
                    let uuid = generate_uuid();

                    // Find insertion point: right after the heading line
                    // If there's already a #properties block, add id to it
                    // Otherwise, insert a new #properties block
                    let has_existing_drawer = h.properties.is_some();

                    if has_existing_drawer {
                        // Add id line inside the existing drawer
                        // Find the #properties line
                        let props_line = (h.properties.as_ref().unwrap().span.line as usize).saturating_sub(1);
                        insertions.push((props_line, heading_text, uuid));
                    } else {
                        insertions.push((line_0idx, heading_text, uuid));
                    }
                }
            }
        }

        if insertions.is_empty() {
            continue;
        }

        if dry_run {
            println!("[dry run] {} — would assign {} ID(s):", file_path.display(), insertions.len());
            for (_, heading, uuid) in &insertions {
                println!("  {heading} → {uuid}");
            }
            total_assigned += insertions.len();
            continue;
        }

        // Build new file content with inserted IDs
        let mut new_lines: Vec<String> = Vec::new();
        let mut insertion_map: std::collections::HashMap<usize, Vec<(String, bool)>> = std::collections::HashMap::new();

        for (line_idx, _heading_text, uuid) in &insertions {
            // Check if this heading already has a properties drawer
            let has_drawer = result.document.children.iter().any(|b| {
                if let Block::Heading(h) = b {
                    let h_line = (h.span.line as usize).saturating_sub(1);
                    h_line == *line_idx && h.properties.is_some()
                } else {
                    false
                }
            });

            // For headings with existing drawers, insertion_map key is the #properties line
            // For headings without drawers, key is the heading line
            insertion_map
                .entry(*line_idx)
                .or_default()
                .push((uuid.clone(), has_drawer));
        }

        for (i, line) in lines.iter().enumerate() {
            new_lines.push(line.to_string());

            if let Some(ids) = insertion_map.get(&i) {
                for (uuid, has_drawer) in ids {
                    if *has_drawer {
                        // Insert "id = uuid" right after #properties line
                        new_lines.push(format!("id = {uuid}"));
                    } else {
                        // Insert a full #properties block after the heading
                        new_lines.push(String::new());
                        new_lines.push("#properties".to_string());
                        new_lines.push(format!("id = {uuid}"));
                        new_lines.push("#end".to_string());
                    }
                }
            }
        }

        let content = new_lines.join("\n");
        std::fs::write(file_path, &content)?;

        println!("{} — assigned {} ID(s)", file_path.display(), insertions.len());
        for (_, heading, uuid) in &insertions {
            println!("  {heading} → {uuid}");
        }
        total_assigned += insertions.len();
    }

    if total_assigned == 0 {
        println!("All headings already have IDs.");
    } else if !dry_run {
        println!("\nAssigned {total_assigned} ID(s) total.");
    }

    Ok(())
}

fn generate_uuid() -> String {
    // Simple UUID v4 generation without external crate
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seed = now.as_nanos();

    // xorshift-based PRNG seeded from timestamp + incrementing counter
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut state = seed.wrapping_add(counter as u128).wrapping_mul(6364136223846793005) as u64;

    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let val = state.wrapping_mul(0x2545F4914F6CDD1D);
        for (i, b) in val.to_le_bytes().iter().enumerate() {
            if i < chunk.len() {
                chunk[i] = *b;
            }
        }
    }

    // Set version 4 and variant bits
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}
