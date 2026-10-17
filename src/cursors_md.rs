//! `CURSORS.md`: the app's cursor list written out for the designer, with what's drawn and what's
//! left. Rewritten by every `pull` and `check`; nothing reads it back.

use crate::check::{Report, TodoKind};
use crate::manifest::Manifest;

/// The project's `CURSORS.md`. `app` names the app; `stamp` says which version of its list this is.
pub fn render(app: &str, stamp: Option<&str>, m: &Manifest, r: &Report) -> String {
    let status = |id: &str| match r.todos.iter().find(|t| t.id == id).map(|t| t.kind) {
        Some(TodoKind::Missing) => "to draw",
        Some(TodoKind::Fix) => "to fix",
        None => "drawn",
    };
    // The seed `pull` wrote for a cursor, if any: one of the drawings the app doesn't use.
    let seed = |id: &str| {
        let file = format!("{id}-{}-rendered.svg", app.to_lowercase());
        r.extras.iter().any(|c| c.file == file).then_some(file)
    };
    let count = |kind| r.todos.iter().filter(|t| t.kind == kind).count();
    let total = m.designer_cursors().count();
    let (missing, fix) = (count(TodoKind::Missing), count(TodoKind::Fix));

    let mut out = format!("# {app} cursors\n\n");
    let from = stamp.map(|s| format!(" (pulled from {s})")).unwrap_or_default();
    out.push_str(&format!("The cursors {app} needs, from its cursor list{from}. Don't edit this file: nibcraft rewrites it on every `pull` and `check`. Notes for {app}'s developers go in DESIGNER_NOTES.md.\n\n"));
    out.push_str("Draw each cursor as `svg/<id>.svg` from its description below, never from another app's cursors. Read DRAWING-GUIDE.md (in nibcraft's top folder) first.\n\n");
    out.push_str(&format!("{total} cursors: {} drawn, {missing} to draw, {fix} to fix.\n", total - missing - fix));

    for (kind, title) in [(TodoKind::Missing, "To draw"), (TodoKind::Fix, "To fix")] {
        let items: Vec<String> = r
            .todos
            .iter()
            .filter(|t| t.kind == kind)
            .map(|t| match kind {
                TodoKind::Fix => format!("- {}: {}", t.id, t.description),
                TodoKind::Missing => match seed(&t.id) {
                    Some(file) => format!("- {} (start from svg/{file}, how {app} draws it today)", t.id),
                    None => format!("- {}", t.id),
                },
            })
            .collect();
        if !items.is_empty() {
            out.push_str(&format!("\n## {title}\n\n{}\n", items.join("\n")));
        }
    }

    for group in m.groups() {
        out.push_str(&format!("\n## {}\n", if group.is_empty() { "Other" } else { group }));
        for c in m.designer_cursors().filter(|c| c.group == group) {
            out.push_str(&format!("\n### {} · {}\n\n{}\n", c.id, status(&c.id), c.description()));
        }
    }

    if !m.not_needed.is_empty() {
        out.push_str("\n## Not needed\n\nThese tools have no cursor of their own on purpose: don't draw one.\n\n");
        for n in &m.not_needed {
            out.push_str(&format!("- **{}**: {}\n", n.what, n.why));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::assess;

    const NIB: &[u8] = include_bytes!("../testdata/round-trip-checker.svg");

    #[test]
    fn lists_what_to_draw_then_every_cursor_by_group_with_its_status() {
        let m = Manifest::parse(
            "app = \"VectorCraft\"\n[[cursor]]\nid = \"Pen\"\ngroup = \"Pen\"\nshown_when = [\"The Pen tool.\"]\nhotspot = \"Tip.\"\nlook = \"A nib.\"\n\n[[cursor]]\nid = \"Lasso\"\ngroup = \"Tools\"\nshown_when = [\"The Lasso tool.\"]\nhotspot = \"Loop start.\"\n\n[[cursor]]\nid = \"Knife\"\ngroup = \"Tools\"\nshown_when = [\"The Knife.\"]\nhotspot = \"Point.\"\n\n[[not_needed]]\nwhat = \"Liquify tools\"\nwhy = \"The app draws the brush.\"\n",
        )
        .unwrap();
        let r = assess(Ok(&m), vec![("Pen.svg".into(), NIB.to_vec()), ("Knife.svg".into(), b"<svg/>".to_vec()), ("Lasso-vectorcraft-rendered.svg".into(), NIB.to_vec())]);
        let md = render("VectorCraft", Some("VectorCraft 0.10.0 (abc1234, 2026-10-13)"), &m, &r);
        assert!(md.starts_with("# VectorCraft cursors\n\nThe cursors VectorCraft needs, from its cursor list (pulled from VectorCraft 0.10.0 (abc1234, 2026-10-13))."), "{md}");
        assert!(md.contains("3 cursors: 1 drawn, 1 to draw, 1 to fix."), "{md}");
        assert!(md.contains("## To draw\n\n- Lasso (start from svg/Lasso-vectorcraft-rendered.svg, how VectorCraft draws it today)\n"), "{md}");
        assert!(md.contains("## To fix\n\n- Knife: no element with id=\"hotspot\""), "{md}");
        assert!(md.contains("## Pen\n\n### Pen · drawn\n\nShown when:\n- The Pen tool.\n\nHotspot: Tip.\n\nLook: A nib.\n"), "{md}");
        assert!(md.contains("### Lasso · to draw") && md.contains("### Knife · to fix"), "{md}");
        assert!(md.contains("## Not needed") && md.contains("- **Liquify tools**: The app draws the brush."), "{md}");
    }
}
