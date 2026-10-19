//! Seeds: how the app draws, in its own code, a cursor nobody has drawn yet, as a starting point.
//! `pull` writes each as `svg/<id>-<app>-rendered.svg`. The app has no part in this: nibcraft makes
//! a scratch git worktree of the app's checkout under its own `target/seeds/` (uncommitted changes
//! included), drops its own drawing module into it ([`MODULE`]), runs it there as a test, and
//! resets the worktree on the next pull. The worktree and its build stay, so later pulls only
//! rebuild what changed.
//!
//! Seeds are rewritten by every pull, so nobody draws in them: the designer copies one to
//! `<id>.svg` and draws there. The app ignores them (only `<id>.svg` is a drawing).

use crate::parse::SEED_NOTE_ID;
use crate::project::Project;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The module nibcraft drops into the app's `cursors` module to draw its code-drawn cursors.
const MODULE: &str = include_str!("../runtime/vectorcraft_seeds.rs");
/// Where it goes, the file whose `mod` list gets it, and the line it goes after.
const MODULE_AT: &str = "crates/ui-egui/src/cursors/nibcraft_seeds.rs";
const CURSORS_RS: &str = "crates/ui-egui/src/cursors.rs";
const AFTER: &str = "mod glyphs;\n";
/// The crate and test that write the seeds.
const CRATE: &str = "vectorcraft-ui-egui";
const TEST: &str = "cursors::nibcraft_seeds::write_seeds";

/// What seeding did.
#[derive(Debug, PartialEq)]
pub enum Seeded {
    /// The seeds written (or rewritten), by cursor id, and the ones that couldn't be, with why.
    Written(Vec<String>, Vec<String>),
    /// Why there are no seeds this time; the pull goes on without them.
    Skipped(String),
}

/// Where nibcraft keeps its scratch worktree of the app, the build of it, and what it draws.
/// Absolute, since git and cargo run elsewhere: a relative path would land in the app's checkout.
fn scratch(root: &Path, p: &Project) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let dir = std::path::absolute(root.join("target").join("seeds")).map_err(|e| format!("{}: {e}", root.display()))?;
    Ok((dir.join(&p.name), dir.join("target"), dir.join(format!("{}-drawn", p.name))))
}

/// Write the seeds of `ids` (cursors with no drawing and no glyph in the app): build the app's
/// code-drawn cursors in the scratch worktree and keep the ones asked for. Progress (the build)
/// goes to the terminal as it happens.
pub fn seed(root: &Path, p: &Project, app: &str, app_dir: &Path, ids: &[String]) -> Seeded {
    if ids.is_empty() {
        return Seeded::Written(vec![], vec![]);
    }
    let (worktree, target, out) = match scratch(root, p) {
        Ok(paths) => paths,
        Err(e) => return Seeded::Skipped(e),
    };
    let _ = std::fs::remove_dir_all(&out); // A leftover from an interrupted pull, if any.
    let result = (|| {
        let worktree = prepare(app_dir, &worktree)?;
        std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        let out = out.canonicalize().map_err(|e| format!("{}: {e}", out.display()))?;
        eprintln!("Getting {app}'s own drawings of {} cursors: building {app} in {}.", ids.len(), worktree.display());
        eprintln!("The first time this takes several minutes; later pulls are quicker.");
        let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["test", "-q", "-p", CRATE, "--lib", TEST, "--", "--ignored", "--exact"])
            .env("CARGO_TARGET_DIR", &target)
            .env("NIBCRAFT_SEEDS_DIR", &out)
            .current_dir(&worktree)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| format!("couldn't run cargo: {e}"))?;
        if !status.success() {
            return Err(format!("building {app}'s drawings failed ({status}); see the output above"));
        }
        place(p, app, ids, &out)
    })();
    let _ = std::fs::remove_dir_all(&out); // Only scratch: a leftover is cleared by the next pull.
    result.map_or_else(Seeded::Skipped, |(written, failed)| Seeded::Written(written, failed))
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| format!("couldn't run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!("git {} in {}: {}", args.join(" "), dir.display(), String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Bring the scratch worktree at `worktree` to the app checkout's current state, uncommitted
/// changes to tracked files included, with nibcraft's module dropped in. Returns its absolute path.
fn prepare(app_dir: &Path, worktree: &Path) -> Result<PathBuf, String> {
    // A commit of the working tree as it is, without touching it; empty when nothing's changed.
    let snapshot = git(app_dir, &["stash", "create"])?;
    let commit = if snapshot.is_empty() { git(app_dir, &["rev-parse", "HEAD"])? } else { snapshot };
    if worktree.join(".git").exists() {
        git(worktree, &["checkout", "--quiet", "--detach", "--force", &commit])?;
    } else {
        let _ = std::fs::remove_dir_all(worktree); // Not a worktree: whatever's there is ours.
        let parent = worktree.parent().ok_or("no folder for the scratch worktree")?;
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        git(app_dir, &["worktree", "prune"])?;
        let at = worktree.to_string_lossy();
        git(app_dir, &["worktree", "add", "--quiet", "--detach", &at, &commit])?;
    }
    let worktree = worktree.canonicalize().map_err(|e| format!("{}: {e}", worktree.display()))?;
    let cursors = worktree.join(CURSORS_RS);
    let text = std::fs::read_to_string(&cursors).map_err(|e| format!("{}: {e}", cursors.display()))?;
    let Some(i) = text.find(AFTER) else {
        return Err(format!("{CURSORS_RS} has no `{}` line any more: nibcraft's seeds need updating to match it", AFTER.trim()));
    };
    let at = i + AFTER.len();
    let with_module = format!("{}#[cfg(test)]\nmod nibcraft_seeds;\n{}", &text[..at], &text[at..]);
    std::fs::write(&cursors, with_module).map_err(|e| format!("{}: {e}", cursors.display()))?;
    let module = worktree.join(MODULE_AT);
    if let Some(dir) = module.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&module, MODULE).map_err(|e| format!("{}: {e}", module.display()))?;
    Ok(worktree)
}

/// Write each of `ids` the app drew into `dir` as that cursor's seed: the ids written, and a
/// message for each seed that couldn't be (the others are still written).
fn place(p: &Project, app: &str, ids: &[String], dir: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let (mut written, mut failed) = (vec![], vec![]);
    for id in ids {
        let from = dir.join(format!("{id}.svg"));
        let Ok(svg) = std::fs::read_to_string(&from) else {
            continue;
        }; // Not drawn in code (it borrows a look).
        let to = p.seed(id, app);
        match std::fs::write(&to, with_warning(&svg, id, app)) {
            Ok(()) => written.push(id.clone()),
            Err(e) => failed.push(format!("{}: {e}", to.display())),
        }
    }
    written.sort();
    Ok((written, failed))
}

/// The seed with a comment at the top saying not to draw in it, and the same in red text at the
/// canvas's top left, which a drawing program shows (nibcraft never draws it: [`SEED_NOTE_ID`]).
fn with_warning(svg: &str, id: &str, app: &str) -> String {
    let warning = format!(
        "<!-- Seed: how {app} draws {id} today. Don't draw in this file: every `nibcraft pull` overwrites it. Copy it to {id}.svg and draw there. -->\n"
    );
    let svg = match svg.split_once('\n') {
        Some((first, rest)) if first.starts_with("<?xml") => format!("{first}\n{warning}{rest}"),
        _ => format!("{warning}{svg}"),
    };
    let note = format!(
        "  <g id=\"{SEED_NOTE_ID}\" fill=\"#ff0000\" font-family=\"sans-serif\" font-size=\"1.4\">\n    \
         <text x=\"0.5\" y=\"1.6\">Seed, ignored: every pull overwrites it.</text>\n    \
         <text x=\"0.5\" y=\"3.2\">To draw, save a copy as {id}.svg.</text>\n  </g>\n"
    );
    match svg.rfind("</svg>") {
        Some(end) => format!("{}{note}{}", &svg[..end], &svg[end..]),
        None => svg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::tests::root;

    const SVG: &str = "<?xml version=\"1.0\"?>\n<svg>\n</svg>\n";

    fn project(name: &str) -> Project {
        let p = Project::at(&root(name), "testcraft");
        std::fs::create_dir_all(p.svg_dir()).unwrap();
        std::fs::create_dir_all(p.state_dir()).unwrap();
        p
    }

    #[test]
    fn seeds_are_written_for_the_ids_asked_that_the_app_drew() {
        let p = project("seed-place");
        let dir = p.dir.join("out");
        std::fs::create_dir_all(&dir).unwrap();
        for id in ["Cross", "Pen"] {
            std::fs::write(dir.join(format!("{id}.svg")), SVG).unwrap();
        }
        let ids = ["Cross".to_string(), "Borrowed".to_string()];
        assert_eq!(place(&p, "TestCraft", &ids, &dir).unwrap(), (vec!["Cross".to_string()], vec![]));
        let seed = std::fs::read_to_string(p.svg_dir().join("Cross-testcraft-rendered.svg")).unwrap();
        assert!(seed.starts_with("<?xml version=\"1.0\"?>\n<!-- Seed: how TestCraft draws Cross today. Don't draw in this file"), "{seed}");
        assert!(seed.contains("<g id=\"seed-note\"") && seed.contains(">To draw, save a copy as Cross.svg.</text>"), "{seed}");
        assert!(!p.svg_dir().join("Pen-testcraft-rendered.svg").exists(), "only the ids asked for");
    }

    #[test]
    fn the_scratch_folders_are_absolute_whatever_the_root() {
        let (worktree, target, out) = scratch(Path::new("."), &Project::at(Path::new("."), "testcraft")).unwrap();
        assert!(worktree.is_absolute() && target.is_absolute() && out.is_absolute());
        assert!(worktree.ends_with("target/seeds/testcraft"), "{}", worktree.display());
    }

    #[test]
    fn nothing_to_seed_builds_nothing() {
        let p = project("seed-none");
        assert_eq!(seed(&p.dir, &p, "TestCraft", Path::new("/nonexistent"), &[]), Seeded::Written(vec![], vec![]));
    }

    /// A git repository standing in for the app, with a `cursors.rs` that has the `mod` line.
    fn app(r: &Path, cursors: &str) -> PathBuf {
        let a = r.join("app");
        std::fs::create_dir_all(a.join("crates/ui-egui/src/cursors")).unwrap();
        std::fs::write(a.join(CURSORS_RS), cursors).unwrap();
        for args in [
            &["init", "--quiet"][..],
            &["config", "user.name", "Test"],
            &["config", "user.email", "t@example.com"],
            &["config", "commit.gpgsign", "false"],
        ] {
            git(&a, args).unwrap();
        }
        git(&a, &["add", "."]).unwrap();
        git(&a, &["commit", "--quiet", "-m", "app"]).unwrap();
        a
    }

    #[test]
    fn the_worktree_follows_the_app_uncommitted_changes_included_and_gets_the_module() {
        let r = root("seed-worktree");
        let a = app(&r, "mod glyphs;\nfn glyph() {}\n");
        let wt = r.join("target/seeds/testcraft");
        let got = prepare(&a, &wt).unwrap();
        let cursors = std::fs::read_to_string(got.join(CURSORS_RS)).unwrap();
        assert_eq!(cursors, "mod glyphs;\n#[cfg(test)]\nmod nibcraft_seeds;\nfn glyph() {}\n");
        assert_eq!(std::fs::read_to_string(got.join(MODULE_AT)).unwrap(), MODULE);
        assert_eq!(git(&a, &["status", "--porcelain"]).unwrap(), "", "the app's checkout is untouched");

        // An uncommitted change in the app reaches the worktree; the module goes in once.
        std::fs::write(a.join(CURSORS_RS), "mod glyphs;\nfn glyph() { /* changed */ }\n").unwrap();
        let again = prepare(&a, &wt).unwrap();
        let cursors = std::fs::read_to_string(again.join(CURSORS_RS)).unwrap();
        assert_eq!(cursors, "mod glyphs;\n#[cfg(test)]\nmod nibcraft_seeds;\nfn glyph() { /* changed */ }\n");
        assert_eq!(std::fs::read_to_string(a.join(CURSORS_RS)).unwrap(), "mod glyphs;\nfn glyph() { /* changed */ }\n");
    }

    #[test]
    fn an_app_reorganised_past_the_module_line_is_named() {
        let r = root("seed-reorganised");
        let a = app(&r, "fn glyph() {}\n");
        let e = prepare(&a, &r.join("target/seeds/testcraft")).unwrap_err();
        assert!(e.contains("need updating"), "{e}");
    }
}
