//! `nibcraft pull [PATH/TO/APP] [--no-seeds]`: copy the app's cursor list into its project,
//! creating the project the first time, and write the seeds ([`crate::seeds`]). Safe to run at any
//! time: it never touches a drawing.

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
    pub first: bool,
    pub seeded: Seeded,
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
        [] => current.and_then(|p| p.app_dir()).ok_or("pull needs the path to the app's checkout the first time, e.g. `nibcraft pull ../vectorcraft`")?,
        _ => return Err(format!("pull takes one path, to the app's checkout, and optionally {NO_SEEDS}")),
    };
    let app_dir = app_dir.canonicalize().map_err(|e| format!("{}: {e}", app_dir.display()))?;
    let list_path = app_dir.join(APP_MANIFEST);
    let text = std::fs::read_to_string(&list_path).map_err(|e| format!("{}: {e} (is this an app checkout with a cursor list?)", list_path.display()))?;
    let manifest = Manifest::parse(&text)?;
    let app = manifest.app.clone().unwrap_or_else(|| app_dir.file_name().and_then(|n| n.to_str()).unwrap_or("app").to_string());
    let project = Project::at(root, &app.to_lowercase());
    let stamp = stamp(&app, &app_dir);

    let before = match std::fs::read_to_string(project.manifest()) {
        Ok(old) => Some(Manifest::parse(&old).map(|m| ids(&m)).unwrap_or_default()),
        Err(_) => None,
    };
    let now = ids(&manifest);
    let (added, dropped) = match &before {
        Some(old) => (now.iter().filter(|id| !old.contains(id)).cloned().collect(), old.iter().filter(|id| !now.contains(id)).cloned().collect()),
        None => (vec![], vec![]),
    };

    for dir in [project.svg_dir(), project.state_dir()] {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write(&project.manifest(), &format!("{}{}", stamp_line(&stamp), without_stamp(&text)))?;
    write(&project.pull_from(), &format!("{}\n", app_dir.display()))?;
    if !project.notes().exists() {
        write(&project.notes(), &notes::template(&app))?;
    }
    project::set(root, &project.name)?;
    let seeded = if with_seeds { seeds::seed(&project, &app, &app_dir, &manifest) } else { Seeded::Skipped(NO_SEEDS.to_string()) };
    Ok(Pulled { project, app, stamp, added, dropped, first: before.is_none(), seeded })
}

/// Has the app's cursor list changed since the project was last pulled? `None` when the app
/// checkout isn't known or can't be read.
pub fn stale(p: &Project) -> Option<bool> {
    let app = std::fs::read_to_string(p.app_dir()?.join(APP_MANIFEST)).ok()?;
    let ours = std::fs::read_to_string(p.manifest()).ok()?;
    Some(without_stamp(&ours) != app)
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
    let version = std::fs::read_to_string(app_dir.join("Cargo.toml"))
        .ok()
        .and_then(|t| t.parse::<toml::Table>().ok())
        .and_then(|t| {
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
    use crate::check;
    use crate::project::tests::root;

    const LIST: &str = "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Pen\"\nshown_when = [\"Pen.\"]\nhotspot = \"Tip.\"\n";

    /// An app checkout under `r` with this cursor list.
    fn app(r: &Path, list: &str) -> PathBuf {
        let a = r.join("app");
        std::fs::create_dir_all(a.join("crates/tools/src")).unwrap();
        std::fs::write(a.join(APP_MANIFEST), list).unwrap();
        std::fs::write(a.join("Cargo.toml"), "[workspace.package]\nversion = \"1.2.3\"\n").unwrap();
        a
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
        assert_eq!(stale(p), Some(false));
        assert!(matches!(&got.seeded, Seeded::Skipped(why) if why.contains("no seed command")), "{:?}", got.seeded);
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
        std::fs::write(a.join(APP_MANIFEST), "app = \"TestCraft\"\n\n[[cursor]]\nid = \"Lasso\"\nshown_when = [\"Lasso.\"]\nhotspot = \"Tip.\"\n").unwrap();
        assert_eq!(stale(&first.project), Some(true));
        let again = pull(&r, &[]).unwrap();
        assert!(!again.first);
        assert_eq!((again.added, again.dropped), (vec!["Lasso".to_string()], vec!["Pen".to_string()]));
        assert_eq!(std::fs::read_to_string(again.project.svg_dir().join("Pen.svg")).unwrap(), "mine");
        assert_eq!(std::fs::read_to_string(again.project.notes()).unwrap(), "my notes");
        assert_eq!(stale(&again.project), Some(false));
    }

    #[test]
    fn pull_needs_a_path_the_first_time_and_a_cursor_list_there() {
        let r = root("pull-bad");
        assert!(pull(&r, &[]).err().unwrap().contains("the first time"));
        std::fs::create_dir_all(r.join("empty")).unwrap();
        assert!(pull(&r, &[r.join("empty").display().to_string()]).err().unwrap().contains("cursor list"));
    }
}
