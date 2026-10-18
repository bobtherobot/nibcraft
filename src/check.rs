//! The designer's precheck of a project: which of the app's cursors are drawn, which are still
//! needed, and which drawings don't convert. Drawings the app doesn't ask for are shown in the
//! preview but otherwise ignored, so variants and experiments can live in `svg/`.

use crate::emit::fingerprint;
use crate::manifest::Manifest;
use crate::parse;
use crate::preview::Card;
use crate::project::Project;
use std::path::Path;

/// What a check found.
pub struct Report {
    /// The app's cursors that are drawn, in the cursor list's order.
    pub cards: Vec<Card>,
    /// Drawings in `svg/` the app doesn't ask for, by name.
    pub extras: Vec<Card>,
    /// What stops a handoff, in plain words.
    pub problems: Vec<String>,
    /// What's left for the designer.
    pub todos: Vec<Todo>,
}

/// Something left for the designer.
#[derive(Debug, PartialEq)]
pub struct Todo {
    pub id: String,
    pub kind: TodoKind,
    pub description: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TodoKind {
    /// The app needs it and there's no drawing yet.
    Missing,
    /// The drawing doesn't convert.
    Fix,
    /// Drawn, but its description changed in a pull after the drawing was made.
    Changed,
}

/// SVGs whose names start with `_` are scratch files and are skipped entirely.
pub fn is_drawing(file: &str) -> bool {
    file.ends_with(".svg") && !file.starts_with('_')
}

/// Check drawings given as (file name, SVG bytes) against the app's cursor list.
pub fn assess(manifest: Result<&Manifest, String>, svgs: Vec<(String, Vec<u8>)>) -> Report {
    let mut problems = vec![];
    let mut todos = vec![];
    let mut cards = vec![];
    let manifest = match manifest {
        Ok(m) => Some(m),
        Err(e) => {
            problems.push(e);
            None
        }
    };
    let card = |file: &str, bytes: &[u8], description: Option<String>| Card {
        file: file.to_string(),
        glyph: parse::read(bytes),
        description,
        fingerprint: fingerprint(bytes),
        seed_note: parse::has_seed_note(bytes),
    };
    for c in manifest.iter().flat_map(|m| m.designer_cursors()) {
        let file = format!("{}.svg", c.id);
        let Some((_, bytes)) = svgs.iter().find(|(f, _)| *f == file) else {
            todos.push(Todo { id: c.id.clone(), kind: TodoKind::Missing, description: c.description() });
            continue;
        };
        let drawn = card(&file, bytes, Some(c.description()));
        let fault = match &drawn.glyph {
            Err(e) => Some(e.clone()),
            // Copied from a seed: the note would say the drawing is ignored.
            Ok(_) if drawn.seed_note => Some(format!("still has the seed's note (#{}): delete it", parse::SEED_NOTE_ID)),
            Ok(_) => None,
        };
        if let Some(e) = fault {
            problems.push(format!("{file}: {e}"));
            todos.push(Todo { id: c.id.clone(), kind: TodoKind::Fix, description: e });
        }
        cards.push(drawn);
    }
    let extras = svgs
        .iter()
        .filter(|(f, _)| !manifest.is_some_and(|m| f.strip_suffix(".svg").is_some_and(|id| m.wants(id))))
        .map(|(f, bytes)| card(f, bytes, None))
        .collect();
    // Missing first, then fixes; each kind in the cursor list's order.
    todos.sort_by_key(|t| t.kind as u8);
    Report { cards, extras, problems, todos }
}

/// Mark the drawn cursors of `m` whose description changed in a pull (`record`: id and the
/// drawing's fingerprint then) and whose drawing hasn't changed since. Doesn't stop a handoff.
pub fn mark_changed(r: &mut Report, m: &Manifest, record: &[(String, String)], svgs: &[(String, Vec<u8>)]) {
    for c in m.designer_cursors() {
        let file = format!("{}.svg", c.id);
        let unchanged = |fp: &str| svgs.iter().any(|(f, bytes)| *f == file && fingerprint(bytes) == fp);
        if record.iter().any(|(id, fp)| *id == c.id && unchanged(fp)) && !r.todos.iter().any(|t| t.id == c.id) {
            r.todos.push(Todo { id: c.id.clone(), kind: TodoKind::Changed, description: c.description() });
        }
    }
}

/// The project's cursor list as last pulled, and the stamp line saying where it came from.
pub struct Pulled {
    pub manifest: Result<Manifest, String>,
    pub stamp: Option<String>,
}

/// The stamp `pull` writes as the first line of `.nibcraft/cursors.toml`.
const STAMP_PREFIX: &str = "# Pulled from ";
const STAMP_SUFFIX: &str = ". Don't edit: `nibcraft pull` rewrites this file.";

/// The first line of `.nibcraft/cursors.toml`, saying where the list came from (`VectorCraft 0.10.0 (…)`).
pub fn stamp_line(stamp: &str) -> String {
    format!("{STAMP_PREFIX}{stamp}{STAMP_SUFFIX}\n")
}

/// `.nibcraft/cursors.toml`'s text without its stamp line: the app's list as it was pulled.
pub fn without_stamp(text: &str) -> &str {
    if text.starts_with(STAMP_PREFIX) { text.split_once('\n').map_or("", |(_, rest)| rest) } else { text }
}

/// Read the project's `.nibcraft/cursors.toml`.
pub fn pulled(p: &Project) -> Pulled {
    match std::fs::read_to_string(p.manifest()) {
        Ok(text) => {
            let stamp = text.lines().next().and_then(|l| l.strip_prefix(STAMP_PREFIX)).map(|s| s.strip_suffix(STAMP_SUFFIX).unwrap_or(s).to_string());
            Pulled { manifest: Manifest::parse(&text), stamp }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Pulled { manifest: Err(format!("no cursor list yet: run `nibcraft pull PATH/TO/{}` once", p.name)), stamp: None }
        }
        Err(e) => Pulled { manifest: Err(format!("{}: {e}", p.manifest().display())), stamp: None },
    }
}

/// The drawings in `dir` as (file name, bytes), by name. A missing folder has none.
pub fn drawings(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let shown = dir.display();
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(format!("{shown}: {e}")),
    };
    let mut names: Vec<String> = entries.filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| is_drawing(n)).collect();
    names.sort();
    names.into_iter().map(|n| std::fs::read(dir.join(&n)).map(|b| (n.clone(), b)).map_err(|e| format!("{shown}/{n}: {e}"))).collect()
}

#[cfg(test)]
mod changed_tests {
    use super::*;

    #[test]
    fn a_changed_cursor_stays_listed_until_its_drawing_changes() {
        let m = Manifest::parse("[[cursor]]\nid = \"Pen\"\nshown_when = [\"Pen.\"]\nhotspot = \"Tip.\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso.\"]\nhotspot = \"Tip.\"\n").unwrap();
        let nib = include_bytes!("../testdata/round-trip-checker.svg").to_vec();
        let svgs = vec![("Pen.svg".to_string(), nib.clone()), ("Lasso.svg".to_string(), nib.clone())];
        let record = [("Pen".to_string(), fingerprint(&nib)), ("Lasso".to_string(), fingerprint(b"before")), ("Gone".to_string(), "-".to_string())];
        let mut r = assess(Ok(&m), svgs.clone());
        mark_changed(&mut r, &m, &record, &svgs);
        let changed: Vec<&str> = r.todos.iter().filter(|t| t.kind == TodoKind::Changed).map(|t| t.id.as_str()).collect();
        assert_eq!(changed, ["Pen"]);
        assert!(r.problems.is_empty(), "a changed description doesn't stop a handoff");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NIB: &[u8] = include_bytes!("../testdata/round-trip-checker.svg");
    const LIST: &str = "[[cursor]]\nid = \"Pen\"\nshown_when = [\"Pen.\"]\nhotspot = \"Tip.\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso.\"]\nhotspot = \"Tip.\"\n\n[[cursor]]\nid = \"Busy\"\nshown_when = [\"Busy.\"]\nhotspot = \"Centre.\"\n\n[[cursor]]\nid = \"NotAllowed\"\nshown_when = [\"No.\"]\nhotspot = \"Centre.\"\ndesigner = false\n";

    fn svg(name: &str) -> (String, Vec<u8>) {
        (name.into(), NIB.to_vec())
    }

    fn list() -> Manifest {
        Manifest::parse(LIST).unwrap()
    }

    #[test]
    fn a_drawing_copied_from_a_seed_must_lose_the_seed_note() {
        let noted = String::from_utf8(NIB.to_vec()).unwrap().replacen("</svg>", "<g id=\"seed-note\"><text>Seed</text></g></svg>", 1);
        let r = assess(Ok(&list()), vec![("Pen.svg".into(), noted.clone().into_bytes()), ("Lasso-app-rendered.svg".into(), noted.into_bytes())]);
        assert_eq!(r.problems, ["Pen.svg: still has the seed's note (#seed-note): delete it"]);
        assert!(r.todos.iter().any(|t| t.id == "Pen" && t.kind == TodoKind::Fix));
        assert!(r.extras[0].seed_note && r.extras[0].glyph.is_ok(), "a seed itself is fine");
    }

    #[test]
    fn drawn_cursors_get_cards_and_undrawn_ones_are_todos() {
        let m = list();
        let r = assess(Ok(&m), vec![svg("Pen.svg")]);
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        assert_eq!(r.cards.len(), 1);
        assert!(r.cards[0].description.as_deref().unwrap().contains("- Pen."));
        let missing: Vec<&str> = r.todos.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(missing, ["Lasso", "Busy"], "NotAllowed is the system's, not the designer's");
        assert!(r.todos.iter().all(|t| t.kind == TodoKind::Missing));
    }

    #[test]
    fn drawings_the_app_doesnt_ask_for_are_extras_never_problems() {
        let m = list();
        let r = assess(Ok(&m), vec![svg("Pen.svg"), svg("Pen-bolder.svg"), ("Broken.svg".into(), b"<svg/>".to_vec()), svg("NotAllowed.svg"), svg("pen.svg")]);
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        let extras: Vec<&str> = r.extras.iter().map(|c| c.file.as_str()).collect();
        assert_eq!(extras, ["Pen-bolder.svg", "Broken.svg", "NotAllowed.svg", "pen.svg"]);
    }

    #[test]
    fn a_wanted_drawing_that_doesnt_convert_is_a_problem_and_a_fix() {
        let m = list();
        let r = assess(Ok(&m), vec![("Lasso.svg".into(), b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"4\" height=\"4\"/></svg>".to_vec())]);
        assert_eq!(r.problems.len(), 1);
        assert!(r.problems[0].starts_with("Lasso.svg: no element with id=\"hotspot\""), "{:?}", r.problems);
        let kinds: Vec<(&str, TodoKind)> = r.todos.iter().map(|t| (t.id.as_str(), t.kind)).collect();
        assert_eq!(kinds, [("Pen", TodoKind::Missing), ("Busy", TodoKind::Missing), ("Lasso", TodoKind::Fix)]);
    }

    #[test]
    fn without_a_cursor_list_every_drawing_is_an_extra() {
        let r = assess(Err("no cursor list yet".into()), vec![svg("Pen.svg")]);
        assert_eq!(r.problems, ["no cursor list yet"]);
        assert!(r.cards.is_empty() && r.todos.is_empty());
        assert_eq!(r.extras.len(), 1);
    }

    #[test]
    fn underscore_files_are_scratch() {
        assert!(is_drawing("Pen.svg"));
        assert!(!is_drawing("_round-trip-checker.svg"));
        assert!(!is_drawing("CURSORS.md"));
    }

    #[test]
    fn the_pulled_list_and_its_stamp_are_read_from_the_project() {
        let r = crate::project::tests::root("pulled");
        let p = Project::at(&r, "v");
        std::fs::create_dir_all(p.state_dir()).unwrap();
        assert!(pulled(&p).manifest.err().unwrap().contains("nibcraft pull"));
        std::fs::write(p.manifest(), format!("{}{LIST}", stamp_line("VectorCraft 0.10.0 (abc1234, 2026-10-13)"))).unwrap();
        let got = pulled(&p);
        assert_eq!(got.manifest.unwrap().cursor.len(), 4);
        assert_eq!(got.stamp.as_deref(), Some("VectorCraft 0.10.0 (abc1234, 2026-10-13)"));
        assert_eq!(without_stamp(&std::fs::read_to_string(p.manifest()).unwrap()), LIST);
        assert_eq!(drawings(&p.svg_dir()).unwrap(), vec![], "no svg folder yet");
    }
}
