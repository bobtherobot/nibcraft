//! Handing a checked project over to the app: what changed since the last handoff, and the
//! command an app developer runs to bring the drawings in.

use crate::preview::Card;
use std::collections::HashMap;
use std::path::Path;

/// The record of the last handoff (in the project's `.nibcraft/handoff`): one `id fingerprint`
/// line per drawn cursor. `ready` commits it, so "since the last handoff" is the same everywhere.
/// Older records (`File.svg svg-fingerprint description-fingerprint`) still read.
pub fn record(cards: &[Card]) -> String {
    let mut out = String::from("# Written by `nibcraft ready`; don't edit. id fingerprint\n");
    for c in cards {
        out.push_str(&format!("{} {}\n", id(&c.file), c.fingerprint));
    }
    out
}

fn id(file: &str) -> &str {
    file.strip_suffix(".svg").unwrap_or(file)
}

/// A record's lines as (id, fingerprint), in order.
fn entries(record: &str) -> Vec<(&str, &str)> {
    record
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            Some((id(words.next()?), words.next()?))
        })
        .collect()
}

/// Cursors that changed between two handoff records, by kind.
#[derive(Debug, Default, PartialEq)]
pub struct Changes {
    pub new: Vec<String>,
    pub redrawn: Vec<String>,
    pub removed: Vec<String>,
}

/// What changed from the `previous` record (none on the first handoff) to `current`.
pub fn changes(previous: Option<&str>, current: &str) -> Changes {
    let before = entries(previous.unwrap_or(""));
    let now = entries(current);
    let old: HashMap<&str, &str> = before.iter().copied().collect();
    let mut c = Changes::default();
    for &(id, fp) in &now {
        match old.get(id) {
            None => c.new.push(id.to_string()),
            Some(&old_fp) if old_fp != fp => c.redrawn.push(id.to_string()),
            Some(_) => {}
        }
    }
    c.removed = before.iter().filter(|(id, _)| !now.iter().any(|(n, _)| n == id)).map(|(id, _)| id.to_string()).collect();
    c
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.redrawn.is_empty() && self.removed.is_empty()
    }

    /// One `Kind: A, B` line per kind that has cursors.
    pub fn lines(&self) -> Vec<String> {
        [("New", &self.new), ("Redrawn", &self.redrawn), ("Removed", &self.removed)]
            .iter()
            .filter(|(_, ids)| !ids.is_empty())
            .map(|(kind, ids)| format!("{kind}: {}", ids.join(", ")))
            .collect()
    }
}

/// What to tell the designer: the changes, then the command to run in the app's checkout.
/// `nibcraft` is nibcraft's top folder; `app_dir` the app checkout, when known.
pub fn instructions(app: &str, nibcraft: &Path, app_dir: Option<&Path>, cursors: usize, changes: &Changes, first: bool) -> String {
    let summary = if first {
        format!("First handoff: all {cursors} drawn cursors are new.")
    } else if changes.is_empty() {
        "Nothing changed since the last handoff.".to_string()
    } else {
        format!("Since the last handoff:\n  {}", changes.lines().join("\n  "))
    };
    let place = app_dir.map_or_else(|| format!("in the {app} checkout"), |d| format!("in {}", d.display()));
    format!("Ready to hand over: {cursors} drawn cursors, no problems.\n\n{summary}\n\nNext, a {app} developer runs this {place}:\n\n  cargo xtask cursors {}\n", nibcraft.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(file: &str, fingerprint: &str) -> Card {
        Card { file: file.into(), glyph: Err(String::new()), description: None, fingerprint: fingerprint.into(), seed_note: false }
    }

    #[test]
    fn record_has_a_line_per_cursor_by_id() {
        let r = record(&[card("Pen.svg", "11111111"), card("Lasso.svg", "22222222")]);
        let lines: Vec<&str> = r.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines, ["Pen 11111111", "Lasso 22222222"]);
    }

    #[test]
    fn changes_are_sorted_into_kinds() {
        let before = record(&[card("Kept.svg", "1"), card("Drawn.svg", "2"), card("Gone.svg", "4")]);
        let after = record(&[card("Kept.svg", "1"), card("Drawn.svg", "9"), card("Fresh.svg", "5")]);
        let c = changes(Some(&before), &after);
        assert_eq!(c, Changes { new: vec!["Fresh".into()], redrawn: vec!["Drawn".into()], removed: vec!["Gone".into()] });
    }

    #[test]
    fn an_old_record_with_file_names_and_description_fingerprints_still_reads() {
        let old = "# Written by `nibcraft ready`; don't edit. file svg-fingerprint description-fingerprint\nPen.svg 1 aaaa\nGone.svg 2 bbbb\n";
        let c = changes(Some(old), &record(&[card("Pen.svg", "1")]));
        assert_eq!(c, Changes { removed: vec!["Gone".into()], ..Changes::default() });
    }

    #[test]
    fn first_handoff_makes_everything_new() {
        let c = changes(None, &record(&[card("A.svg", "1"), card("B.svg", "2")]));
        assert_eq!(c.new, ["A", "B"]);
    }

    #[test]
    fn instructions_give_the_command_to_run_in_the_app() {
        let c = Changes { new: vec!["Lasso".into()], redrawn: vec!["Pen".into()], ..Changes::default() };
        let text = instructions("VectorCraft", Path::new("/home/me/nibcraft"), Some(Path::new("/home/me/vectorcraft")), 45, &c, false);
        assert!(text.contains("runs this in /home/me/vectorcraft:\n\n  cargo xtask cursors /home/me/nibcraft\n"), "{text}");
        assert!(text.contains("New: Lasso") && text.contains("Redrawn: Pen"), "{text}");
        assert!(!text.contains("/update-cursors"), "{text}");
        let none = instructions("VectorCraft", Path::new("/x"), None, 3, &Changes::default(), false);
        assert!(none.contains("Nothing changed since the last handoff") && none.contains("in the VectorCraft checkout"), "{none}");
    }
}
