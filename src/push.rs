//! `nibcraft push [PATH/TO/APP]`: hand the drawings to the app. Rewrites the generated part of the
//! app's glyph file (VectorCraft: `crates/ui-egui/src/cursors/glyphs.rs`) from the project's
//! drawings and nothing else in the app, then commits the project folder. The app's developers see
//! the change like any other edit in their checkout; rebuilding the app shows the cursors.
//!
//! A drawing that gives exactly the app's glyph keeps the app's code as it is (its author
//! included); a new or redrawn one is written with the pusher's name (`git config user.name`).
//! A cursor the app ships but the project no longer draws is taken out of the app ("unwired"): it
//! falls back to the app's own drawing of it.

use crate::app_glyphs::{self, GLYPHS_RS, GlyphsFile, Variant};
use crate::emit::{self, const_name};
use crate::manifest::Manifest;
use crate::parse;
use crate::project::Project;
use std::path::{Path, PathBuf};

/// The cursors a push changed in the app, by kind.
#[derive(Debug, Default, PartialEq)]
pub struct Changes {
    pub new: Vec<String>,
    pub redrawn: Vec<String>,
    pub unwired: Vec<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.redrawn.is_empty() && self.unwired.is_empty()
    }

    /// One `Kind: A, B` line per kind that has cursors.
    pub fn lines(&self) -> Vec<String> {
        [("New", &self.new), ("Redrawn", &self.redrawn), ("Unwired", &self.unwired)]
            .iter()
            .filter(|(_, ids)| !ids.is_empty())
            .map(|(kind, ids)| format!("{kind}: {}", ids.join(", ")))
            .collect()
    }
}

/// What writing the glyphs did.
pub struct Written {
    /// The glyph file written (or that would have been, had it changed).
    pub path: PathBuf,
    pub changes: Changes,
    /// The glyphs the app now has from the project.
    pub drawn: usize,
    /// The file as it is now, to record as shipped.
    pub glyphs: GlyphsFile,
}

/// Write `p`'s drawings of `m`'s cursors into the app checkout at `app_dir`, in the order of its
/// `Cursor` values. Cursors the designer doesn't draw keep whatever the app has. `author` is
/// asked for only when a glyph is new or redrawn.
pub fn write_glyphs(p: &Project, app_dir: &Path, m: &Manifest, author: impl Fn() -> Result<String, String>) -> Result<Written, String> {
    let path = app_dir.join(GLYPHS_RS);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file = GlyphsFile::parse(&text)?;
    let variants = crate::pull::read_variants(app_dir)?;
    let mut rows: Vec<(&Variant, String, String)> = vec![];
    let mut changes = Changes::default();
    let mut drawn = 0;
    for v in &variants {
        let shipped = file.get(&v.name);
        if !m.wants(&v.name) {
            if let Some(g) = shipped {
                rows.push((v, g.konst.clone(), g.code.clone()));
            }
            continue;
        }
        let svg = p.svg_dir().join(format!("{}.svg", v.name));
        let bytes = match std::fs::read(&svg) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if shipped.is_some() {
                    changes.unwired.push(v.name.clone());
                }
                continue;
            }
            Err(e) => return Err(format!("{}: {e}", svg.display())),
        };
        drawn += 1;
        match shipped {
            Some(g) if app_glyphs::same(&bytes, g) => rows.push((v, g.konst.clone(), g.code.clone())),
            _ => {
                let glyph = parse::read(&bytes).map_err(|e| format!("{}: {e}", svg.display()))?;
                let konst = const_name(&v.name);
                rows.push((v, konst.clone(), emit::glyph(&konst, &v.name, &author()?, &glyph)));
                if shipped.is_some() { &mut changes.redrawn } else { &mut changes.new }.push(v.name.clone());
            }
        }
    }
    let unknown: Vec<&str> = m.designer_cursors().map(|c| c.id.as_str()).filter(|id| !variants.iter().any(|v| v.name == *id)).collect();
    if !unknown.is_empty() {
        return Err(format!(
            "the cursor list names cursors the app's `Cursor` has no value for: {}. Its developers need to add them first",
            unknown.join(", ")
        ));
    }
    let out = file.rewrite(&rows);
    // Read back what's about to be written: a mistake here must never reach the app.
    let glyphs = GlyphsFile::parse(&out).map_err(|e| format!("nibcraft wrote a glyph file it can't read back, so nothing was written: {e}"))?;
    if out != text {
        std::fs::write(&path, &out).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(Written { path, changes, drawn, glyphs })
}

/// What to tell the designer after a push: what changed in the app, the notes for its developers,
/// and what to do next.
pub fn summary(app: &str, w: &Written, notes: &str) -> String {
    let mut out = if w.changes.is_empty() {
        format!("{app} already has these {} drawings: nothing to push.\n", w.drawn)
    } else {
        format!("Pushed into {}:\n  {}\n", w.path.display(), w.changes.lines().join("\n  "))
    };
    if !w.changes.unwired.is_empty() {
        out.push_str(&format!("(Unwired: no drawing here any more, so {app} draws them its own way again.)\n"));
    }
    if !notes.is_empty() {
        out.push_str(&format!("\nYour notes for {app}'s developers (DESIGNER_NOTES.md), to pass on with the change:\n\n{notes}\n"));
    }
    if !w.changes.is_empty() {
        out.push_str(&format!("\nNext: rebuild {app} to see them, then commit {GLYPHS_RS} there.\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_glyphs::ENUM_RS;
    use crate::app_glyphs::tests::{ENUM, file, nib};
    use crate::project::tests::root;

    const LIST: &str = "[[cursor]]\nid = \"Pen\"\n\n[[cursor]]\nid = \"RotateToward\"\n\n[[cursor]]\nid = \"Arrow\"\ndesigner = false\n";
    const NIB: &[u8] = include_bytes!("../testdata/round-trip-checker.svg");

    /// A project and an app checkout under `r`, the app shipping `rows` and `consts`.
    fn setup(name: &str, rows: &str, consts: &[String]) -> (PathBuf, Project) {
        let r = root(name);
        let a = r.join("app");
        std::fs::create_dir_all(a.join(GLYPHS_RS).parent().unwrap()).unwrap();
        std::fs::create_dir_all(a.join(ENUM_RS).parent().unwrap()).unwrap();
        std::fs::write(a.join(GLYPHS_RS), file(rows, consts)).unwrap();
        std::fs::write(a.join(ENUM_RS), ENUM).unwrap();
        let p = Project::at(&r, "testcraft");
        std::fs::create_dir_all(p.svg_dir()).unwrap();
        (a, p)
    }

    fn author() -> Result<String, String> {
        Ok("A. Designer".into())
    }

    #[test]
    fn a_new_drawing_goes_in_with_its_author_in_cursor_order() {
        let arrow = nib("Arrow").replace("PEN", "ARROW");
        let (a, p) = setup("push-new", "    (Cursor::Arrow, &ARROW),", &[arrow]);
        std::fs::write(p.svg_dir().join("Pen.svg"), NIB).unwrap();
        let m = Manifest::parse(LIST).unwrap();
        let w = write_glyphs(&p, &a, &m, || Ok("Someone Else".into())).unwrap();
        assert_eq!(w.changes, Changes { new: vec!["Pen".into()], ..Changes::default() });
        let text = std::fs::read_to_string(a.join(GLYPHS_RS)).unwrap();
        assert!(text.contains("    (Cursor::Arrow, &ARROW),\n    (Cursor::Pen, &PEN),\n];"), "the app's own glyph stays, in order: {text}");
        assert!(text.contains("/// The `Pen` cursor, drawn by Someone Else."), "{text}");
        assert_eq!(w.glyphs.glyphs.len(), 2);
    }

    #[test]
    fn an_unchanged_drawing_keeps_the_apps_code_and_writes_nothing() {
        let (a, p) = setup("push-same", "    (Cursor::Pen, &PEN),", &[nib("Pen")]);
        std::fs::write(p.svg_dir().join("Pen.svg"), NIB).unwrap();
        let before = std::fs::read_to_string(a.join(GLYPHS_RS)).unwrap();
        let w = write_glyphs(&p, &a, &Manifest::parse(LIST).unwrap(), || Err("no author needed".into())).unwrap();
        assert!(w.changes.is_empty() && w.drawn == 1);
        assert_eq!(std::fs::read_to_string(a.join(GLYPHS_RS)).unwrap(), before);
        assert!(summary("TestCraft", &w, "").starts_with("TestCraft already has these 1 drawings"));
    }

    #[test]
    fn a_redrawn_cursor_is_replaced_and_a_deleted_drawing_unwired() {
        let rotate = nib("RotateToward").replace("const PEN", "const ROTATE_TOWARD");
        let (a, p) = setup(
            "push-redrawn",
            "    (Cursor::RotateToward(super::ROTATE_TOWARD_DRAWN), &ROTATE_TOWARD),\n    (Cursor::Pen, &PEN),",
            &[rotate, nib("Pen")],
        );
        let more = String::from_utf8(NIB.to_vec()).unwrap().replacen("</svg>", "<path d=\"M20 20 L22 20 L22 22 Z\" fill=\"#000000\"/></svg>", 1);
        std::fs::write(p.svg_dir().join("Pen.svg"), more).unwrap();
        let w = write_glyphs(&p, &a, &Manifest::parse(LIST).unwrap(), author).unwrap();
        assert_eq!(w.changes, Changes { redrawn: vec!["Pen".into()], unwired: vec!["RotateToward".into()], ..Changes::default() });
        let text = std::fs::read_to_string(a.join(GLYPHS_RS)).unwrap();
        assert!(text.contains(&format!("{}    (Cursor::Pen, &PEN),\n];", "pub(super) const GENERATED: &[(Cursor, &Glyph)] = &[\n")), "{text}");
        assert!(!text.contains("ROTATE_TOWARD") && text.contains("drawn by A. Designer"), "{text}");
        let s = summary("TestCraft", &w, "Bolder?");
        assert!(s.contains("Redrawn: Pen") && s.contains("Unwired: RotateToward") && s.contains("Bolder?") && s.contains("rebuild TestCraft"), "{s}");
    }

    #[test]
    fn a_listed_cursor_the_app_has_no_value_for_is_refused() {
        let (a, p) = setup("push-unknown", "", &[]);
        let m = Manifest::parse("[[cursor]]\nid = \"Lasso\"\n").unwrap();
        let before = std::fs::read_to_string(a.join(GLYPHS_RS)).unwrap();
        assert!(write_glyphs(&p, &a, &m, author).err().unwrap().contains("Lasso"));
        assert_eq!(std::fs::read_to_string(a.join(GLYPHS_RS)).unwrap(), before);
    }
}
