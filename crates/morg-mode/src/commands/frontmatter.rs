use std::path::PathBuf;

use serde_yaml::Value;

use crate::collect;

pub fn run(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = collect::parse_files(paths);

    let mut merged = Value::Mapping(serde_yaml::Mapping::new());

    let mut count = 0;
    for pf in &parsed {
        if let Some(ref fm) = pf.document.frontmatter {
            deep_merge(&mut merged, &fm.data);
            count += 1;
        }
    }

    if count == 0 {
        println!("No frontmatter found.");
        return Ok(());
    }

    let yaml = serde_yaml::to_string(&merged)?;
    print!("{yaml}");
    eprintln!("---\nMerged frontmatter from {count} file(s).");

    Ok(())
}

fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Mapping(base_map), Value::Mapping(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                if let Some(base_val) = base_map.get_mut(key) {
                    deep_merge(base_val, overlay_val);
                } else {
                    base_map.insert(key.clone(), overlay_val.clone());
                }
            }
        }
        (Value::Sequence(base_seq), Value::Sequence(overlay_seq)) => {
            base_seq.extend(overlay_seq.iter().cloned());
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_merge_maps() {
        let mut base: Value = serde_yaml::from_str("a: 1\nb: 2").unwrap();
        let overlay: Value = serde_yaml::from_str("b: 3\nc: 4").unwrap();
        deep_merge(&mut base, &overlay);

        assert_eq!(base["a"], Value::Number(1.into()));
        assert_eq!(base["b"], Value::Number(3.into()));
        assert_eq!(base["c"], Value::Number(4.into()));
    }

    #[test]
    fn test_deep_merge_sequences() {
        let mut base: Value = serde_yaml::from_str("tags:\n  - a\n  - b").unwrap();
        let overlay: Value = serde_yaml::from_str("tags:\n  - c").unwrap();
        deep_merge(&mut base, &overlay);

        let tags = base["tags"].as_sequence().unwrap();
        assert_eq!(tags.len(), 3);
    }

    #[test]
    fn test_deep_merge_nested() {
        let mut base: Value = serde_yaml::from_str("meta:\n  author: Alice\n  version: 1").unwrap();
        let overlay: Value = serde_yaml::from_str("meta:\n  version: 2\n  license: MIT").unwrap();
        deep_merge(&mut base, &overlay);

        assert_eq!(base["meta"]["author"], Value::String("Alice".into()));
        assert_eq!(base["meta"]["version"], Value::Number(2.into()));
        assert_eq!(base["meta"]["license"], Value::String("MIT".into()));
    }
}
