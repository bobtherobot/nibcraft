//! `nibcraft pull [PATH/TO/APP] [--no-seeds]`: bring the project level with the app, creating it
//! the first time. The app is the source of truth: pull copies its cursor list in, writes each
//! cursor the app ships as a drawing (`svg/<id>.svg`, from its glyph), replaces a drawing whose
//! glyph changed in the app since the last pull or push, and writes the seeds ([`crate::seeds`]).
//!
//! A drawing that differs from the app's glyph is kept when the app's glyph hasn't changed since
//! the last pull or push (`.nibcraft/shipped`): that's the designer's work not pushed yet. Anything
//! else is replaced: commit your work along the way.

use crate::app_glyphs::{self, ENUM_RS, GLYPHS_RS, GlyphsFile};
use crate::check::{stamp_line, without_stamp};
use crate::manifest::{APP_MANIFEST, Manifest};
use crate::notes;
use crate::project::{self, Project};
use crate::seeds::{self, Seeded};
use std::path::{Path, PathBuf};
use std::process::Command;

/// What a pull did, for the designer.
pub struct Pulled {
    pub project: Project,
    pub app: String,
    pub stamp: String,
    /// Cursors the list gained and lost since the previous pull (empty on the first).
    pub added: Vec<String>,
    pub dropped: Vec<String>,
    /// Cursors whose description this pull changed (empty on the first).
    pub changed: Vec<String>,
    pub first: bool,
    pub synced: Synced,
    pub seeded: Seeded,
}

/// What a pull did to the drawings to bring them level with the app, by cursor id.
#[derive(Debug, Default, PartialEq)]
pub struct Synced {
    /// Shipped by the app and not drawn here: written from the app's glyph.
    pub written: Vec<String>,
    /// Their glyph changed in the app since the last pull or push: replaced by it.
    pub replaced: Vec<String>,
    /// The app no longer ships a glyph for them: their drawing is removed.
    pub removed: Vec<String>,
}

/// The flag that skips the seeds (and the app's build they need).
pub const NO_SEEDS: &str = "--no-seeds";

/// Pull into `root` from `app_dir`, or from where the current project was last pulled.
pub fn pull(root: &Path, args: &[String]) -> Result<Pulled, String> {
    let with_seeds = !args.iter().any(|a| a == NO_SEEDS);
    let args: Vec<&String> = args.iter().filter(|a| *a != NO_SEEDS).collect();
    let current = project::current(root).map(|n| Project::at(root, &n));
    let app_dir = match args[..] {
        [path] => PathBuf::from(path.trim_end_matches('/')),
        [] => current
            .and_then(|p| p.app_dir())
            .ok_or("pull needs the path to the app's checkout the first time, e.g. `nibcraft pull ../vectorcraft`")?,
        _ => return Err(format!("pull takes one path, to the app's checkout, and optionally {NO_SEEDS}")),
    };
    let app_dir = app_dir.canonicalize().map_err(|e| format!("{}: {e}", app_dir.display()))?;
    let list_path = app_dir.join(APP_MANIFEST);
    let text =
        std::fs::read_to_string(&list_path).map_err(|e| format!("{}: {e} (is this an app checkout with a cursor list?)", list_path.display()))?;
    let manifest = Manifest::parse(&text)?;
    let glyphs = read_glyphs(&app_dir)?;
    let app = manifest.app.clone().unwrap_or_else(|| app_dir.file_name().and_then(|n| n.to_str()).unwrap_or("app").to_string());
    let project = Project::at(root, &app.to_lowercase());
    let stamp = stamp(&app, &app_dir);

    let old_list = std::fs::read_to_string(project.manifest()).ok().map(|old| Manifest::parse(&old).ok());
    let before = old_list.as_ref().map(|m| m.as_ref().map(ids).unwrap_or_default());
    let now = ids(&manifest);
    let (added, dropped) = match &before {
        Some(old) => (now.iter().filter(|id| !old.contains(id)).cloned().collect(), old.iter().filter(|id| !now.contains(id)).cloned().collect()),
        None => (vec![], vec![]),
    };
    // Changed: a description that differs from the last pull's, or a cursor new to the list whose
    // drawing is already there (renamed), which wasn't drawn from this description either.
    let changed: Vec<String> = match old_list.flatten() {
        Some(old) => manifest
            .designer_cursors()
            .filter(|c| match old.designer_cursors().find(|o| o.id == c.id) {
                Some(o) => o.description() != c.description(),
                None => project.svg_dir().join(format!("{}.svg", c.id)).is_file(),
            })
            .map(|c| c.id.clone())
            .collect(),
        None => vec![],
    };

    for dir in [project.svg_dir(), project.state_dir()] {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write(&project.manifest(), &format!("{}{}", stamp_line(&stamp), without_stamp(&text)))?;
    write(&project.pull_from(), &format!("{}\n", app_dir.display()))?;
    record_changes(&project, &now, &changed)?;
    if !project.notes().exists() {
        write(&project.notes(), &notes::template(&app))?;
    }
    let synced = sync(&project, &manifest, &glyphs, &app)?;
    write_shipped(&project, &manifest, &glyphs)?;
    // Older nibcraft's record of what `ready` handed over: `shipped` does its job now.
    let _ = std::fs::remove_file(project.state_dir().join("handoff"));
    project::set(root, &project.name)?;

    // Seeds for the cursors nobody has drawn yet, here or in the app; the others' old seeds go.
    let to_seed: Vec<String> =
        manifest.designer_cursors().filter(|c| glyphs.get(&c.id).is_none() && !drawing(&project, &c.id).is_file()).map(|c| c.id.clone()).collect();
    for c in manifest.designer_cursors().filter(|c| !to_seed.contains(&c.id)) {
        let _ = std::fs::remove_file(project.seed(&c.id, &app)); // Usually not there.
    }
    let seeded = if with_seeds { seeds::seed(root, &project, &app, &app_dir, &to_seed) } else { Seeded::Skipped(NO_SEEDS.to_string()) };
    Ok(Pulled { project, app, stamp, added, dropped, changed, first: before.is_none(), synced, seeded })
}

/// Cursor `id`'s drawing in project `p`.
fn drawing(p: &Project, id: &str) -> PathBuf {
    p.svg_dir().join(format!("{id}.svg"))
}

/// The app's generated glyphs, from its checkout at `app_dir`.
pub fn read_glyphs(app_dir: &Path) -> Result<GlyphsFile, String> {
    let path = app_dir.join(GLYPHS_RS);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("{}: {e} (is this a VectorCraft checkout with generated cursor glyphs?)", path.display()))?;
    GlyphsFile::parse(&text)
}

/// The app's `Cursor` values, in order, from its checkout at `app_dir`.
pub fn read_variants(app_dir: &Path) -> Result<Vec<app_glyphs::Variant>, String> {
    let path = app_dir.join(ENUM_RS);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v = app_glyphs::variants(&text);
    if v.is_empty() {
        return Err(format!("{}: can't find `pub enum Cursor`; was it moved? nibcraft needs updating to match it", path.display()));
    }
    Ok(v)
}

/// Bring `p`'s drawings of `m`'s cursors level with the app's `glyphs`; see the module's doc.
fn sync(p: &Project, m: &Manifest, glyphs: &GlyphsFile, app: &str) -> Result<Synced, String> {
    let shipped = shipped(p);
    let mut out = Synced::default();
    for c in m.designer_cursors() {
        let path = drawing(p, &c.id);
        let bytes = match std::fs::read(&path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let before = shipped.iter().find(|(id, _)| *id == c.id).map(|(_, fp)| fp.as_str());
        match (glyphs.get(&c.id), bytes) {
            (Some(g), None) => {
                write(&path, &app_glyphs::svg(g, app)?)?;
                out.written.push(c.id.clone());
            }
            // The same, or the designer's work on a glyph the app hasn't changed since: keep it.
            (Some(g), Some(b)) if app_glyphs::same(&b, g) || before == Some(g.fingerprint().as_str()) => {}
            (Some(g), Some(_)) => {
                write(&path, &app_glyphs::svg(g, app)?)?;
                out.replaced.push(c.id.clone());
            }
            // Shipped at the last pull or push, and taken out of the app since.
            (None, Some(_)) if before.is_some() => {
                std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                out.removed.push(c.id.clone());
            }
            (None, _) => {}
        }
    }
    Ok(out)
}

/// The app's glyphs as of the last pull or push, from `.nibcraft/shipped`: (id, [`AppGlyph::fingerprint`]).
///
/// [`AppGlyph::fingerprint`]: app_glyphs::AppGlyph::fingerprint
pub fn shipped(p: &Project) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(p.shipped()).unwrap_or_default();
    text.lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.split_once(' ')).map(|(id, fp)| (id.to_string(), fp.trim().to_string())).collect()
}

/// The app's glyphs of `m`'s cursors as they are now: what [`shipped`] reads back.
fn shipped_now(m: &Manifest, glyphs: &GlyphsFile) -> Vec<(String, String)> {
    m.designer_cursors().filter_map(|c| glyphs.get(&c.id)).map(|g| (g.id.clone(), g.fingerprint())).collect()
}

/// Record the app's glyphs of `m`'s cursors as they are now.
pub fn write_shipped(p: &Project, m: &Manifest, glyphs: &GlyphsFile) -> Result<(), String> {
    let mut text = String::from("# The app's glyphs as of the last `nibcraft pull` or `push`; don't edit. id fingerprint\n");
    for (id, fp) in shipped_now(m, glyphs) {
        text.push_str(&format!("{id} {fp}\n"));
    }
    write(&p.shipped(), &text)
}

/// Why the app checkout at `app_dir` is ahead of the project's last pull or push, in plain words;
/// empty when it isn't.
pub fn behind(p: &Project, app_dir: &Path) -> Result<Vec<String>, String> {
    let list = std::fs::read_to_string(app_dir.join(APP_MANIFEST)).map_err(|e| format!("{}: {e}", app_dir.join(APP_MANIFEST).display()))?;
    let ours = std::fs::read_to_string(p.manifest()).map_err(|e| format!("{}: {e}", p.manifest().display()))?;
    let mut out = vec![];
    if without_stamp(&ours) != list {
        out.push("its cursor list has changed".to_string());
    }
    let m = Manifest::parse(&list)?;
    let now = shipped_now(&m, &read_glyphs(app_dir)?);
    let before = shipped(p);
    let mut ids: Vec<&str> =
        now.iter().filter(|e| !before.contains(e)).chain(before.iter().filter(|e| !now.contains(e))).map(|(id, _)| id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    if !ids.is_empty() {
        out.push(format!("its glyphs have changed ({})", ids.join(", ")));
    }
    Ok(out)
}

/// The fingerprint recorded for a changed cursor that had no drawing yet.
pub const NOT_DRAWN: &str = "-";

/// The cursors whose description changed in a pull, each with its drawing's fingerprint at the
/// time ([`NOT_DRAWN`] if it had none), from `.nibcraft/changed`. One line per cursor: `id fingerprint`.
pub fn changes(p: &Project) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(p.changed()).unwrap_or_default();
    text.lines().filter_map(|l| l.split_once(' ')).map(|(id, fp)| (id.to_string(), fp.trim().to_string())).collect()
}

/// Add the cursors this pull `changed` to the record, with their drawings' fingerprints now, and
/// drop the cursors no longer listed (`now`). An empty record removes the file.
fn record_changes(p: &Project, now: &[String], changed: &[String]) -> Result<(), String> {
    let mut record: Vec<(String, String)> = changes(p).into_iter().filter(|(id, _)| now.contains(id) && !changed.contains(id)).collect();
    for id in changed {
        let fp = std::fs::read(p.svg_dir().join(format!("{id}.svg"))).map(|b| crate::emit::fingerprint(&b)).unwrap_or_else(|_| NOT_DRAWN.to_string());
        record.push((id.clone(), fp));
    }
    if record.is_empty() {
        return match std::fs::remove_file(p.changed()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", p.changed().display())),
            _ => Ok(()),
        };
    }
    let text: String = record.iter().map(|(id, fp)| format!("{id} {fp}\n")).collect();
    write(&p.changed(), &text)
}

fn ids(m: &Manifest) -> Vec<String> {
    m.designer_cursors().map(|c| c.id.clone()).collect()
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `VectorCraft 0.10.0 (a9faa901, 2026-10-13)`: the app, its version and the commit the list
/// came from, with `-dirty` when the list has changes not committed yet.
fn stamp(app: &str, app_dir: &Path) -> String {
    let version = std::fs::read_to_string(app_dir.join("Cargo.toml")).ok().and_then(|t| t.parse::<toml::Table>().ok()).and_then(|t| {
        let ws = t.get("workspace").and_then(|w| w.get("package")).and_then(|p| p.get("version"));
        ws.or_else(|| t.get("package").and_then(|p| p.get("version"))).and_then(|v| v.as_str()).map(String::from)
    });
    let commit = git(app_dir, &["rev-parse", "--short", "HEAD"]).map(|hash| {
        let dirty = git(app_dir, &["status", "--porcelain", "--", APP_MANIFEST]).is_some_and(|s| !s.is_empty());
        let date = git(app_dir, &["log", "-1", "--format=%cs"]).unwrap_or_default();
        let hash = if dirty { format!("{hash}-dirty") } else { hash };
        if date.is_empty() { hash } else { format!("{hash}, {date}") }
    });
    let mut out = app.to_string();
    if let Some(v) = version {
        out.push_str(&format!(" {v}"));
    }
    if let Some(c) = commit {
        out.push_str(&format!(" ({c})"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_glyphs::tests::{ENUM, file, nib};
    use crate::check;
    use crate::project::tests::root;

    const LIST: &str = "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Pen\"\nshown_when = [\"Pen.\"]\nhotspot = \"Tip.\"\n";

    /// An app checkout under `r` with this cursor list.
    fn app(r: &Path, list: &str) -> PathBuf {
        let a = r.join("app");
        std::fs::create_dir_all(a.join("crates/tools/src")).unwrap();
        std::fs::write(a.join(APP_MANIFEST), list).unwrap();
        std::fs::write(a.join("Cargo.toml"), "[workspace.package]\nversion = \"1.2.3\"\n").unwrap();
        std::fs::write(a.join(ENUM_RS), ENUM).unwrap();
        ship(&a, &[]);
        a
    }

    /// The app at `a` ships these glyphs (the round-trip checker's, under each id).
    fn ship(a: &Path, ids: &[&str]) {
        let rows: Vec<String> = ids.iter().map(|id| format!("    (Cursor::{id}, &{}),", crate::emit::const_name(id))).collect();
        let consts: Vec<String> = ids.iter().map(|id| nib(id)).collect();
        std::fs::create_dir_all(a.join(GLYPHS_RS).parent().unwrap()).unwrap();
        std::fs::write(a.join(GLYPHS_RS), file(&rows.join("\n"), &consts)).unwrap();
    }

    fn up_to_date(p: &Project, a: &Path) -> bool {
        behind(p, a).unwrap().is_empty()
    }

    #[test]
    fn the_first_pull_creates_the_project_and_remembers_the_app() {
        let r = root("pull-first");
        let a = app(&r, LIST);
        let got = pull(&r, &[format!("{}/", a.display())]).unwrap();
        let p = &got.project;
        assert!(got.first && got.app == "TestCraft" && p.name == "testcraft");
        assert!(got.stamp.starts_with("TestCraft 1.2.3"), "{}", got.stamp);
        assert!(p.svg_dir().is_dir() && p.notes().is_file() && !p.dir.join("README.md").exists());
        assert_eq!(p.app_dir(), Some(a.canonicalize().unwrap()));
        assert_eq!(project::current(&r).as_deref(), Some("testcraft"));
        let pulled = check::pulled(p);
        assert_eq!(pulled.manifest.unwrap().cursor.len(), 1);
        assert_eq!(pulled.stamp.as_deref(), Some(got.stamp.as_str()));
        assert!(up_to_date(p, &a));
        assert!(matches!(&got.seeded, Seeded::Skipped(why) if why.contains("git")), "not a git checkout, so no seeds: {:?}", got.seeded);
        let quiet = pull(&r, &[NO_SEEDS.into()]).unwrap();
        assert_eq!(quiet.seeded, Seeded::Skipped(NO_SEEDS.into()));
    }

    #[test]
    fn a_later_pull_reports_added_and_dropped_cursors_and_keeps_the_designers_files() {
        let r = root("pull-again");
        let a = app(&r, LIST);
        let first = pull(&r, &[a.display().to_string()]).unwrap();
        std::fs::write(first.project.svg_dir().join("Pen.svg"), "mine").unwrap();
        std::fs::write(first.project.notes(), "my notes").unwrap();
        std::fs::write(a.join(APP_MANIFEST), "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso.\"]\nhotspot = \"Tip.\"\n")
            .unwrap();
        assert_eq!(behind(&first.project, &a).unwrap(), ["its cursor list has changed"]);
        let again = pull(&r, &[]).unwrap();
        assert!(!again.first);
        assert_eq!((again.added, again.dropped), (vec!["Lasso".to_string()], vec!["Pen".to_string()]));
        assert_eq!(std::fs::read_to_string(again.project.svg_dir().join("Pen.svg")).unwrap(), "mine");
        assert_eq!(std::fs::read_to_string(again.project.notes()).unwrap(), "my notes");
        assert!(up_to_date(&again.project, &a));
    }

    #[test]
    fn a_pull_records_changed_descriptions_until_their_drawing_changes() {
        let r = root("pull-changed");
        let both = |pen: &str| {
            format!(
                "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Pen\"\nshown_when = [\"{pen}\"]\nhotspot = \"Tip.\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso.\"]\nhotspot = \"Tip.\"\n"
            )
        };
        let a = app(&r, &both("Pen."));
        let first = pull(&r, &[a.display().to_string()]).unwrap();
        assert!(first.changed.is_empty() && changes(&first.project).is_empty());
        std::fs::write(first.project.svg_dir().join("Pen.svg"), "mine").unwrap();

        std::fs::write(a.join(APP_MANIFEST), both("Pen, with a badge.")).unwrap();
        let again = pull(&r, &[]).unwrap();
        assert_eq!(again.changed, ["Pen"]);
        assert_eq!(changes(&again.project), [("Pen".to_string(), crate::emit::fingerprint(b"mine"))]);

        // A pull with nothing new keeps the record; a cursor that leaves the list leaves it.
        let same = pull(&r, &[]).unwrap();
        assert!(same.changed.is_empty());
        assert_eq!(changes(&same.project).len(), 1);
        std::fs::write(
            a.join(APP_MANIFEST),
            "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso, changed.\"]\nhotspot = \"Tip.\"\n",
        )
        .unwrap();
        let gone = pull(&r, &[]).unwrap();
        assert_eq!(gone.changed, ["Lasso"]);
        assert_eq!(changes(&gone.project), [("Lasso".to_string(), NOT_DRAWN.to_string())]);

        // A cursor new to the list whose drawing is already there (a renamed one) wasn't drawn
        // from this description either.
        std::fs::write(gone.project.svg_dir().join("Knife.svg"), "renamed").unwrap();
        std::fs::write(a.join(APP_MANIFEST), "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso, changed.\"]\nhotspot = \"Tip.\"\n\n[[cursor]]\nid = \"Knife\"\nshown_when = [\"Knife.\"]\nhotspot = \"Tip.\"\n").unwrap();
        let renamed = pull(&r, &[]).unwrap();
        assert_eq!((renamed.added.clone(), renamed.changed.clone()), (vec!["Knife".to_string()], vec!["Knife".to_string()]));
    }

    #[test]
    fn the_app_is_the_source_of_truth_for_what_it_ships() {
        let r = root("pull-sync");
        let a = app(&r, LIST);
        ship(&a, &["Pen"]);
        let first = pull(&r, &[a.display().to_string(), NO_SEEDS.into()]).unwrap();
        let p = &first.project;
        let pen = p.svg_dir().join("Pen.svg");
        assert_eq!(first.synced, Synced { written: vec!["Pen".into()], ..Synced::default() });
        let g = read_glyphs(&a).unwrap();
        assert!(app_glyphs::same(&std::fs::read(&pen).unwrap(), g.get("Pen").unwrap()));
        assert_eq!(shipped(p), [("Pen".to_string(), g.get("Pen").unwrap().fingerprint())]);

        // The designer's work on a glyph the app hasn't changed is kept.
        let mine = String::from_utf8(include_bytes!("../testdata/round-trip-checker.svg").to_vec()).unwrap();
        let mine = mine.replacen("</svg>", "<path d=\"M20 20 L22 20 L22 22 Z\" fill=\"#000000\"/></svg>", 1);
        std::fs::write(&pen, &mine).unwrap();
        let kept = pull(&r, &[NO_SEEDS.into()]).unwrap();
        assert_eq!(kept.synced, Synced::default());
        assert_eq!(std::fs::read_to_string(&pen).unwrap(), mine);

        // The app's glyph changed elsewhere: the app wins.
        let changed = file("    (Cursor::Pen, &PEN),", &[nib("Pen").replacen("(0.0, 0.0)", "(0.5, 0.0)", 1)]);
        std::fs::write(a.join(GLYPHS_RS), changed).unwrap();
        assert_eq!(behind(p, &a).unwrap(), ["its glyphs have changed (Pen)"]);
        let replaced = pull(&r, &[NO_SEEDS.into()]).unwrap();
        assert_eq!(replaced.synced, Synced { replaced: vec!["Pen".into()], ..Synced::default() });
        assert!(app_glyphs::same(&std::fs::read(&pen).unwrap(), read_glyphs(&a).unwrap().get("Pen").unwrap()));
        assert!(up_to_date(p, &a));

        // The app stopped shipping it: the drawing goes.
        ship(&a, &[]);
        let removed = pull(&r, &[NO_SEEDS.into()]).unwrap();
        assert_eq!(removed.synced, Synced { removed: vec!["Pen".into()], ..Synced::default() });
        assert!(!pen.exists() && shipped(p).is_empty());

        // A drawing the app never shipped is the designer's.
        std::fs::write(&pen, &mine).unwrap();
        assert_eq!(pull(&r, &[NO_SEEDS.into()]).unwrap().synced, Synced::default());
        assert!(pen.exists());
    }

    #[test]
    fn pull_needs_a_path_the_first_time_and_a_cursor_list_there() {
        let r = root("pull-bad");
        assert!(pull(&r, &[]).err().unwrap().contains("the first time"));
        std::fs::create_dir_all(r.join("empty")).unwrap();
        assert!(pull(&r, &[r.join("empty").display().to_string()]).err().unwrap().contains("cursor list"));
    }
}
