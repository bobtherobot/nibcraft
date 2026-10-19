//! nibcraft: turns cursor drawings (SVG) into Rust glyph data for an app such as VectorCraft.
//! The app decides which cursors exist (its cursor list) and is the source of truth; designers
//! draw them here. The app knows nothing of nibcraft: nibcraft reads the app's checkout and writes
//! only the generated part of its glyph file.
//!
//!     nibcraft pull [PATH/TO/APP] [--no-seeds]
//!                                      bring the project level with the app (creates the project
//!                                      the first time; later the path is remembered): its cursor
//!                                      list, its shipped cursors as drawings, and seeds (how the
//!                                      app draws cursors nobody has drawn yet; builds the app,
//!                                      --no-seeds skips that)
//!     nibcraft check [PROJECT]      preview the drawings as the app will show them, list what's
//!                                      left to draw, and refresh CURSORS.md
//!     nibcraft push [PATH/TO/APP]   check, write the drawings into the app's glyph file, and
//!                                      commit the project; then rebuild the app to see them
//!     nibcraft projects             list the projects, marking the current one
//!     nibcraft use PROJECT          make a project the current one
//!     nibcraft gen FILE.svg [more.svg…]   print a `const` per SVG
//!
//! check works on the project named, else the current one; push on the current one.
//!
//! See README.md for the workflow and DRAWING-GUIDE.md for the SVG rules.

mod app_glyphs;
mod check;
mod commit;
mod cursors_md;
mod emit;
mod geom;
mod manifest;
mod notes;
mod parse;
mod preview;
mod project;
mod pull;
mod push;
mod seeds;

use project::Project;
use seeds::Seeded;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};

fn generate(files: &[String]) -> Result<String, String> {
    if files.is_empty() {
        return Err("gen needs at least one SVG file".into());
    }
    let mut out = String::new();
    for file in files {
        let path = Path::new(file);
        let bytes = std::fs::read(path).map_err(|e| format!("{file}: {e}"))?;
        let glyph = parse::read(&bytes).map_err(|e| format!("{file}: {e}"))?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("glyph");
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&emit::glyph(&emit::const_name(stem), stem, &commit::author(Path::new(".")).unwrap_or_else(|| "(unknown)".into()), &glyph));
    }
    Ok(out)
}

/// Open `page` in the system's browser without waiting for it. Returns a note when it can't.
fn open_in_browser(page: &Path) -> Option<String> {
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    let spawned = cmd.arg(page).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    spawned.err().map(|e| format!("(Couldn't open the preview in a browser: {e}. Open it yourself.)\n"))
}

/// What checking a project found, with the preview written and a summary for the designer.
struct Checked {
    report: check::Report,
    app: String,
    preview: PathBuf,
    text: String,
}

/// Check project `p`: refresh its `CURSORS.md` and write its preview. Problems are in the report;
/// only a project that can't be read at all is an error.
fn check_project(p: &Project) -> Result<Checked, String> {
    let pulled = check::pulled(p);
    let svgs = check::drawings(&p.svg_dir())?;
    let mut report = check::assess(pulled.manifest.as_ref().map_err(Clone::clone), svgs.clone());
    if let Ok(m) = &pulled.manifest {
        check::mark_changed(&mut report, m, &pull::changes(p), &svgs);
    }
    let app = pulled.manifest.as_ref().ok().and_then(|m| m.app.clone()).unwrap_or_else(|| p.name.clone());
    let mut advice = vec![];
    if let Some(Ok(why)) = p.app_dir().map(|d| pull::behind(p, &d))
        && !why.is_empty()
    {
        advice.push(format!("{app} has changed since your last pull ({}): run `nibcraft pull` to get it.", why.join("; ")));
    }
    if let Some(w) = std::fs::read_to_string(p.notes()).ok().as_deref().and_then(notes::warning) {
        advice.push(w);
    }
    if let Ok(m) = &pulled.manifest {
        let md = cursors_md::render(&app, pulled.stamp.as_deref(), m, &report);
        std::fs::write(p.cursors_md(), md).map_err(|e| format!("{}: {e}", p.cursors_md().display()))?;
    }
    let out = p.preview();
    let html = preview::page(&p.name, &report.cards, &report.extras, &report.problems, &advice, &report.todos);
    std::fs::write(&out, html).map_err(|e| format!("{}: {e}", out.display()))?;
    let preview = out.canonicalize().unwrap_or(out);

    let count = |kind| report.todos.iter().filter(|t| t.kind == kind).count();
    let mut text = format!(
        "{}: {} drawn, {} to draw, {} to fix",
        p.name,
        report.cards.len() - count(check::TodoKind::Fix),
        count(check::TodoKind::Missing),
        count(check::TodoKind::Fix)
    );
    let changed = count(check::TodoKind::Changed);
    if changed > 0 {
        text.push_str(&format!(", {changed} changed since last pull (see CURSORS.md)"));
    }
    if !report.extras.is_empty() {
        text.push_str(&format!(" ({} other drawings, not used)", report.extras.len()));
    }
    text.push_str(&format!(".\nPreview: {}\n", preview.display()));
    let n = report.problems.len();
    if n > 0 {
        text.push_str(&format!("\n{n} problem{}:\n", if n == 1 { "" } else { "s" }));
        for problem in &report.problems {
            text.push_str(&format!("  - {problem}\n"));
        }
    }
    for a in &advice {
        text.push_str(&format!("\nNote: {a}\n"));
    }
    Ok(Checked { report, app, preview, text })
}

/// `pull [PATH/TO/APP]`: copy the app's cursor list in, then check the project.
fn pull_cmd(args: &[String]) -> Result<String, String> {
    let got = pull::pull(Path::new("."), args)?;
    let checked = check_project(&got.project)?;
    let mut out = format!("Pulled {}'s cursor list ({}) into {}.\n", got.app, got.stamp, got.project.dir.display());
    if !got.first {
        if got.added.is_empty() && got.dropped.is_empty() {
            out.push_str("No cursors added or removed since the last pull.\n");
        }
        if !got.changed.is_empty() {
            out.push_str(&format!("Descriptions changed: {} (see CURSORS.md › Changed since last pull)\n", got.changed.join(", ")));
        }
        if !got.added.is_empty() {
            out.push_str(&format!("New in the list: {}\n", got.added.join(", ")));
        }
        if !got.dropped.is_empty() {
            out.push_str(&format!("No longer in the list: {} (their drawings stay in svg/ and are now ignored)\n", got.dropped.join(", ")));
        }
    }
    let s = &got.synced;
    for (ids, what) in [
        (&s.written, format!("Drawings written from what {} ships", got.app)),
        (&s.replaced, format!("Replaced by {}'s changed glyph (your drawing is in git if you committed it)", got.app)),
        (&s.removed, format!("Removed: {} no longer ships them", got.app)),
    ] {
        if !ids.is_empty() {
            out.push_str(&format!("{what}: {}\n", ids.join(", ")));
        }
    }
    match &got.seeded {
        Seeded::Written(ids, failed) => {
            if ids.is_empty() {
                out.push_str(&format!("No seeds: every cursor {} draws itself is drawn here already.\n", got.app));
            } else {
                let files: Vec<String> = ids.iter().filter_map(|id| got.project.seed(id, &got.app).file_name().map(|n| n.to_string_lossy().into_owned())).collect();
                out.push_str(&format!(
                    "Seeds (how {} draws them today; copy one to <id>.svg to start from it, never draw in the seed: pull overwrites it): {}\n",
                    got.app,
                    files.join(", ")
                ));
            }
            for f in failed {
                out.push_str(&format!("A seed couldn't be written: {f}\n"));
            }
        }
        Seeded::Skipped(why) if why == pull::NO_SEEDS => out.push_str(&format!("Seeds skipped ({why}).\n")),
        Seeded::Skipped(why) => out.push_str(&format!("No seeds this time: {why}.\n")),
    }
    out.push_str(&format!("\n{}", checked.text));
    Ok(out)
}

/// `check [PROJECT]`: the designer's precheck. Opens the preview in the browser, problems or not.
fn precheck(args: &[String]) -> Result<String, String> {
    let p = project::resolve(Path::new("."), args)?;
    let mut checked = check_project(&p)?;
    if let Some(note) = open_in_browser(&checked.preview) {
        checked.text.push_str(&note);
    }
    if checked.report.problems.is_empty() {
        Ok(format!("{}\nNo problems. When you're done, run `nibcraft push`.\n", checked.text))
    } else {
        Err(format!("check found problems.\n{}", checked.text))
    }
}

/// `push [PATH/TO/APP]`: check, then write the drawings into the app checkout (the one last
/// pulled from, or PATH) and commit the project folder.
fn push_cmd(args: &[String]) -> Result<String, String> {
    let root = Path::new(".");
    let p = project::resolve(root, &[])?;
    let app_dir = match args {
        [path] => PathBuf::from(path.trim_end_matches('/')),
        [] => p.app_dir().ok_or("push needs the path to the app's checkout: pull from it first, or name it")?,
        _ => return Err("push takes at most one path, to the app's checkout".into()),
    };
    let app_dir = app_dir.canonicalize().map_err(|e| format!("{}: {e}", app_dir.display()))?;
    let checked = check_project(&p)?;
    if !checked.report.problems.is_empty() {
        return Err(format!("not pushed: fix these, then run `nibcraft push` again.\n{}", checked.text));
    }
    let why = pull::behind(&p, &app_dir)?;
    if !why.is_empty() {
        return Err(format!("not pushed: {} has changed since your last pull ({}). Run `nibcraft pull` first (commit your drawings before, to keep them), then push.", checked.app, why.join("; ")));
    }
    let manifest = check::pulled(&p).manifest?;
    let author = || commit::author(root).ok_or_else(|| "who's pushing? Set your name with `git config user.name \"Your Name\"`: each glyph names its author".to_string());
    let written = push::write_glyphs(&p, &app_dir, &manifest, author)?;
    pull::write_shipped(&p, &manifest, &written.glyphs)?;
    let shown = p.dir.display();
    let committed = match commit::commit(&p.dir, &commit::message(&p.name, &written.changes)) {
        Ok(commit::Outcome::Committed(hash)) => format!("Committed {shown} as {hash}.\n"),
        Ok(commit::Outcome::NothingToCommit) => {
            format!("Nothing to commit in {shown}: it's already saved.\n")
        }
        Ok(commit::Outcome::NotARepo) => {
            format!("Not committed: {shown} isn't in a git repository.\n")
        }
        Err(e) => {
            return Err(format!("pushed, but the commit failed; commit {shown} yourself.\n{e}"));
        }
    };
    let notes = notes::written(&std::fs::read_to_string(p.notes()).unwrap_or_default(), &checked.app);
    Ok(format!("{}\n{}\n{committed}", checked.text, push::summary(&checked.app, &written, &notes)))
}

/// `projects`: the projects here, the current one marked.
fn projects() -> Result<String, String> {
    let root = Path::new(".");
    let names = project::list(root);
    if names.is_empty() {
        return Err("no projects yet: `nibcraft pull PATH/TO/APP` creates one".into());
    }
    let now = project::current(root);
    let mut out = String::new();
    for n in names {
        let mark = if now.as_deref() == Some(n.as_str()) { "*" } else { " " };
        out.push_str(&format!("{mark} {n}\n"));
    }
    if now.is_none() {
        out.push_str("\nNo current project: pick one with `nibcraft use PROJECT`.\n");
    }
    Ok(out)
}

/// `use PROJECT`: make it the current project.
fn use_project(args: &[String]) -> Result<String, String> {
    let [name] = args else {
        return Err("use needs one project, e.g. `nibcraft use vectorcraft`".into());
    };
    project::set(Path::new("."), name)?;
    Ok(format!("Current project: {}. pull, check and push now use it.\n", project::current(Path::new(".")).unwrap_or_default()))
}

const USAGE: &str = "usage: nibcraft pull [PATH/TO/APP] [--no-seeds] | check [PROJECT] | push [PATH/TO/APP] | projects | use PROJECT | gen FILE.svg…";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    let result = match args.first().map(String::as_str) {
        Some("pull") => pull_cmd(rest),
        Some("check") => precheck(rest),
        Some("push") => push_cmd(rest),
        Some("projects") => projects(),
        Some("use") => use_project(rest),
        Some("gen") => generate(rest),
        _ => Err(USAGE.into()),
    };
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nibcraft: {e}");
            ExitCode::FAILURE
        }
    }
}
