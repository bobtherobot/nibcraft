//! Seeds: how the app draws a cursor nobody has drawn yet, as a starting point. `pull` runs the
//! command the app's cursor list names (`seeds = [...]`) in the app's checkout, and writes each
//! cursor it gets back as `svg/<id>-<app>-rendered.svg`, unless `svg/<id>.svg` exists already.
//!
//! Seeds are rewritten by every pull, so nobody draws in them: the designer copies one to
//! `<id>.svg` and draws there. The app ignores them (only `<id>.svg` is a drawing). Seeds of
//! cursors that have been drawn since are left as they are, for reference.

use crate::manifest::Manifest;
use crate::parse::SEED_NOTE_ID;
use crate::project::Project;
use std::path::Path;
use std::process::{Command, Stdio};

/// What seeding did.
#[derive(Debug, PartialEq)]
pub enum Seeded {
    /// The seeds written (or rewritten), by cursor id, and the ones that couldn't be, with why.
    Written(Vec<String>, Vec<String>),
    /// Why there are no seeds this time; the pull goes on without them.
    Skipped(String),
}

/// Run the app's seed command and write its drawings into the project as seeds. Progress (the
/// app's build) goes to the terminal as it happens.
pub fn seed(p: &Project, app: &str, app_dir: &Path, m: &Manifest) -> Seeded {
    let Some((program, args)) = m.seeds.as_deref().and_then(<[String]>::split_first) else {
        return Seeded::Skipped(format!("{app}'s cursor list names no seed command"));
    };
    let tmp = p.state_dir().join("seeds-tmp");
    let _ = std::fs::remove_dir_all(&tmp); // A leftover from an interrupted pull, if any.
    // Absolute: the command runs in the app's checkout, not here.
    let tmp = match std::fs::create_dir_all(&tmp).and_then(|()| tmp.canonicalize()) {
        Ok(tmp) => tmp,
        Err(e) => return Seeded::Skipped(format!("{}: {e}", tmp.display())),
    };
    eprintln!("Getting {app}'s own cursor drawings: building {app} first. The first time this takes a few minutes; later pulls are quick.");
    eprintln!("Running `{} {} {}` in {}", program, args.join(" "), tmp.display(), app_dir.display());
    let status = Command::new(program).args(args).arg(&tmp).current_dir(app_dir).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::inherit()).status();
    let result = match status {
        Ok(s) if s.success() => match std::fs::read_dir(&tmp).map(|d| d.count()) {
            Ok(0) => Seeded::Skipped(format!("{app}'s seed command wrote no drawings")),
            _ => place(p, app, m, &tmp).map_or_else(Seeded::Skipped, |(written, failed)| Seeded::Written(written, failed)),
        },
        Ok(s) => Seeded::Skipped(format!("{app}'s seed command failed ({s}); see its output above")),
        Err(e) => Seeded::Skipped(format!("couldn't run `{program}`: {e}")),
    };
    let _ = std::fs::remove_dir_all(&tmp); // Only scratch: a leftover is cleared by the next pull.
    result
}

/// Write each `<id>.svg` in `dir` that the app wants and nobody has drawn as `id`'s seed: the ids
/// written, and a message for each seed that couldn't be (the others are still written).
fn place(p: &Project, app: &str, m: &Manifest, dir: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (mut written, mut failed) = (vec![], vec![]);
    for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
        let Some(id) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".svg")) else { continue };
        if !m.wants(id) || p.svg_dir().join(format!("{id}.svg")).exists() {
            continue;
        }
        let to = p.seed(id, app);
        match std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display())).and_then(|svg| {
            std::fs::write(&to, with_warning(&svg, id, app)).map_err(|e| format!("{}: {e}", to.display()))
        }) {
            Ok(()) => written.push(id.to_string()),
            Err(e) => failed.push(e),
        }
    }
    written.sort();
    Ok((written, failed))
}

/// The seed with a comment at the top saying not to draw in it, and the same in red text at the
/// canvas's top left, which a drawing program shows (nibcraft never draws it: [`SEED_NOTE_ID`]).
fn with_warning(svg: &str, id: &str, app: &str) -> String {
    let warning = format!("<!-- Seed: how {app} draws {id} today. Don't draw in this file: every `nibcraft pull` overwrites it. Copy it to {id}.svg and draw there. -->\n");
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

    const LIST: &str = "app = \"TestCraft\"\n[[cursor]]\nid = \"Cross\"\n[[cursor]]\nid = \"Pen\"\n[[cursor]]\nid = \"Busy\"\ndesigner = false\n";
    const SVG: &str = "<?xml version=\"1.0\"?>\n<svg>\n</svg>\n";

    fn project(name: &str) -> Project {
        let p = Project::at(&root(name), "testcraft");
        std::fs::create_dir_all(p.svg_dir()).unwrap();
        std::fs::create_dir_all(p.state_dir()).unwrap();
        p
    }

    #[test]
    fn seeds_are_written_for_wanted_cursors_nobody_has_drawn() {
        let p = project("seed-place");
        let dir = p.dir.join("out");
        std::fs::create_dir_all(&dir).unwrap();
        for id in ["Cross", "Pen", "Busy", "Unlisted"] {
            std::fs::write(dir.join(format!("{id}.svg")), SVG).unwrap();
        }
        std::fs::write(p.svg_dir().join("Pen.svg"), "mine").unwrap();
        let m = Manifest::parse(LIST).unwrap();
        assert_eq!(place(&p, "TestCraft", &m, &dir).unwrap(), (vec!["Cross".to_string()], vec![]));
        let seed = std::fs::read_to_string(p.svg_dir().join("Cross-testcraft-rendered.svg")).unwrap();
        assert!(seed.starts_with("<?xml version=\"1.0\"?>\n<!-- Seed: how TestCraft draws Cross today. Don't draw in this file"), "{seed}");
        assert!(seed.contains("<g id=\"seed-note\"") && seed.contains(">To draw, save a copy as Cross.svg.</text>"), "{seed}");
        assert_eq!(std::fs::read_to_string(p.svg_dir().join("Pen.svg")).unwrap(), "mine");
        assert!(!p.svg_dir().join("Pen-testcraft-rendered.svg").exists() && !p.svg_dir().join("Busy-testcraft-rendered.svg").exists());
    }

    #[test]
    fn every_pull_overwrites_a_seed_but_keeps_the_seed_of_a_cursor_drawn_since() {
        let p = project("seed-again");
        let dir = p.dir.join("out");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cross.svg"), SVG).unwrap();
        let m = Manifest::parse(LIST).unwrap();
        let seed = p.seed("Cross", "TestCraft");
        std::fs::write(&seed, "edited by mistake").unwrap();
        place(&p, "TestCraft", &m, &dir).unwrap();
        assert!(std::fs::read_to_string(&seed).unwrap().contains("Seed:"), "overwritten");
        std::fs::write(&seed, "kept for reference").unwrap();
        std::fs::write(p.svg_dir().join("Cross.svg"), "drawn").unwrap();
        assert_eq!(place(&p, "TestCraft", &m, &dir).unwrap(), (vec![], vec![]));
        assert_eq!(std::fs::read_to_string(&seed).unwrap(), "kept for reference");
    }

    #[test]
    fn a_list_without_a_seed_command_skips_seeding() {
        let p = project("seed-none");
        let m = Manifest::parse(LIST).unwrap();
        assert!(matches!(seed(&p, "TestCraft", &p.dir, &m), Seeded::Skipped(why) if why.contains("no seed command")));
    }

    /// `dir` relative to the working directory, by enough `..` to reach the root.
    #[cfg(unix)]
    fn pathdiff(dir: &Path) -> std::path::PathBuf {
        let up = std::env::current_dir().unwrap().components().count() - 1;
        let mut rel = std::path::PathBuf::new();
        for _ in 0..up {
            rel.push("..");
        }
        rel.join(dir.strip_prefix("/").unwrap())
    }

    #[cfg(unix)]
    #[test]
    fn the_command_runs_in_the_app_checkout_with_the_folder_added() {
        let p = project("seed-run");
        // A project named by a relative path (nibcraft runs from its own folder) still gets
        // the drawings: the command is given the folder's absolute path.
        let p = Project { name: p.name, dir: pathdiff(&p.dir) };
        let list = format!("seeds = [\"sh\", \"-c\", \"cp seed.svg \\\"$0/Cross.svg\\\"\"]\n{LIST}");
        let m = Manifest::parse(&list).unwrap();
        let app = p.dir.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("seed.svg"), SVG).unwrap();
        assert_eq!(seed(&p, "TestCraft", &app, &m), Seeded::Written(vec!["Cross".into()], vec![]));
        assert!(p.seed("Cross", "TestCraft").is_file());
        assert!(!p.state_dir().join("seeds-tmp").exists(), "the scratch folder is removed");
        let failing = Manifest::parse(&format!("seeds = [\"false\"]\n{LIST}")).unwrap();
        assert!(matches!(seed(&p, "TestCraft", &app, &failing), Seeded::Skipped(why) if why.contains("failed")));
    }
}
