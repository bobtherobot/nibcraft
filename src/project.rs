//! Cursor projects: one folder per app under `projects/` (`projects/vectorcraft`), and which one is
//! current, so commands can be run without naming it.
//!
//! A project holds:
//!
//!     svg/               the designer's drawings, one `<id>.svg` per cursor (other files are ignored),
//!                        and the seeds `pull` writes: `<id>-<app>-rendered.svg`, how the app draws a
//!                        cursor nobody has drawn yet (rewritten by every pull)
//!     CURSORS.md         what the app needs, written by `pull` and `check`: never edit it
//!     DESIGNER_NOTES.md  the designer's notes for the app's developers
//!     .nibcraft/      nibcraft's own files, never edited by hand:
//!         cursors.toml   the app's cursor list as last pulled (committed, so `check` works anywhere)
//!         handoff        what the last `ready` handed over (committed)
//!         pull-from      where the app's checkout is on this machine (local, gitignored)

use std::path::{Path, PathBuf};

/// The folder holding the projects, in the working directory.
pub const PROJECTS_DIR: &str = "projects";

/// The file in the working directory naming the current project. Local, so gitignored.
pub const CURRENT_FILE: &str = ".nibcraft-project";

/// The folder in each project holding nibcraft's own files.
pub const STATE_DIR: &str = ".nibcraft";

/// A project's folder and the files in it.
pub struct Project {
    pub name: String,
    pub dir: PathBuf,
}

impl Project {
    /// The project `name` under `root`, whether or not it exists yet.
    pub fn at(root: &Path, name: &str) -> Self {
        Project { name: name.to_string(), dir: root.join(PROJECTS_DIR).join(name) }
    }

    pub fn svg_dir(&self) -> PathBuf {
        self.dir.join("svg")
    }
    pub fn state_dir(&self) -> PathBuf {
        self.dir.join(STATE_DIR)
    }
    pub fn manifest(&self) -> PathBuf {
        self.state_dir().join("cursors.toml")
    }
    pub fn cursors_md(&self) -> PathBuf {
        self.dir.join("CURSORS.md")
    }
    pub fn notes(&self) -> PathBuf {
        self.dir.join("DESIGNER_NOTES.md")
    }
    pub fn preview(&self) -> PathBuf {
        self.dir.join("PREVIEW.html")
    }
    pub fn pull_from(&self) -> PathBuf {
        self.state_dir().join("pull-from")
    }
    pub fn handoff(&self) -> PathBuf {
        self.state_dir().join("handoff")
    }
    /// The cursors whose description changed in a pull, until their drawing changes ([`crate::pull::changes`]).
    pub fn changed(&self) -> PathBuf {
        self.state_dir().join("changed")
    }
    /// The seed `pull` writes for cursor `id` of `app`: `svg/Crosshair-vectorcraft-rendered.svg`.
    pub fn seed(&self, id: &str, app: &str) -> PathBuf {
        self.svg_dir().join(format!("{id}-{}-rendered.svg", app.to_lowercase()))
    }

    /// The app checkout this project was last pulled from, if it was pulled on this machine.
    pub fn app_dir(&self) -> Option<PathBuf> {
        let text = std::fs::read_to_string(self.pull_from()).ok()?;
        let path = text.trim();
        (!path.is_empty()).then(|| PathBuf::from(path))
    }
}

/// The projects in `root`, by name.
pub fn list(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root.join(PROJECTS_DIR))
        .map(|it| it.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().into_string().ok()).filter(|n| !n.starts_with('.')).collect())
        .unwrap_or_default();
    out.sort();
    out
}

/// The current project in `root`, if one was chosen and still exists.
pub fn current(root: &Path) -> Option<String> {
    let name = std::fs::read_to_string(root.join(CURRENT_FILE)).ok()?.trim().to_string();
    (!name.is_empty() && Project::at(root, &name).dir.is_dir()).then_some(name)
}

/// Make `name` the current project in `root`.
pub fn set(root: &Path, name: &str) -> Result<(), String> {
    let name = clean(name);
    if !Project::at(root, name).dir.is_dir() {
        return Err(format!("{name}: no such project in {PROJECTS_DIR}/ (`nibcraft pull PATH/TO/APP` creates one)"));
    }
    std::fs::write(root.join(CURRENT_FILE), format!("{name}\n")).map_err(|e| format!("{CURRENT_FILE}: {e}"))
}

/// `vectorcraft`, `vectorcraft/` or `projects/vectorcraft` → `vectorcraft`.
fn clean(name: &str) -> &str {
    let name = name.trim_end_matches('/');
    name.strip_prefix(&format!("{PROJECTS_DIR}/")).unwrap_or(name)
}

/// The project a command works on: the one named in `args`, else the current one. It must exist.
pub fn resolve(root: &Path, args: &[String]) -> Result<Project, String> {
    let name = match args {
        [name] => clean(name).to_string(),
        [] => current(root).ok_or_else(|| "no project chosen: name one, or pick one with `nibcraft use NAME` (`nibcraft projects` lists them)".to_string())?,
        _ => return Err("expected at most one project".into()),
    };
    let p = Project::at(root, &name);
    if !p.dir.is_dir() {
        return Err(format!("{name}: no such project in {PROJECTS_DIR}/ (`nibcraft pull PATH/TO/APP` creates one)"));
    }
    Ok(p)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A fresh, empty directory under the system temp dir.
    pub fn root(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nibcraft-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn project(root: &Path, name: &str) {
        std::fs::create_dir_all(root.join(PROJECTS_DIR).join(name)).unwrap();
    }

    #[test]
    fn projects_are_the_folders_under_projects() {
        let r = root("list");
        project(&r, "vectorcraft");
        project(&r, "photocraft");
        std::fs::create_dir_all(r.join("svg-old")).unwrap();
        assert_eq!(list(&r), ["photocraft", "vectorcraft"]);
    }

    #[test]
    fn the_current_project_is_remembered_and_used_when_none_is_named() {
        let r = root("use");
        project(&r, "a");
        project(&r, "b");
        assert_eq!(current(&r), None);
        assert!(resolve(&r, &[]).err().unwrap().contains("nibcraft use"));
        set(&r, "projects/b/").unwrap();
        assert_eq!(current(&r).as_deref(), Some("b"));
        assert_eq!(resolve(&r, &[]).unwrap().name, "b");
        assert_eq!(resolve(&r, &["a".into()]).unwrap().name, "a", "a named project wins");
        assert!(resolve(&r, &["a".into(), "b".into()]).is_err());
        assert!(resolve(&r, &["missing".into()]).err().unwrap().contains("no such project"));
    }

    #[test]
    fn only_an_existing_project_can_be_made_current() {
        let r = root("bad");
        assert!(set(&r, "missing").unwrap_err().contains("no such project"));
        assert_eq!(current(&r), None);
    }

    #[test]
    fn the_app_path_is_read_from_pull_from() {
        let r = root("app");
        project(&r, "v");
        let p = Project::at(&r, "v");
        assert_eq!(p.app_dir(), None);
        std::fs::create_dir_all(p.state_dir()).unwrap();
        std::fs::write(p.pull_from(), "/home/me/vectorcraft\n").unwrap();
        assert_eq!(p.app_dir(), Some(PathBuf::from("/home/me/vectorcraft")));
        assert_eq!(p.svg_dir(), r.join("projects/v/svg"));
    }

    #[test]
    fn a_seed_is_named_after_its_cursor_and_app() {
        let p = Project::at(Path::new("/r"), "vectorcraft");
        assert_eq!(p.seed("Crosshair", "VectorCraft"), PathBuf::from("/r/projects/vectorcraft/svg/Crosshair-vectorcraft-rendered.svg"));
    }
}
