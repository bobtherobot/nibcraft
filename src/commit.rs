//! Committing a handed-over project, so the app gets a saved version: `ready` stages and commits
//! the project folder, and only that folder, with a message saying what changed.

use crate::handoff::Changes;
use std::path::Path;
use std::process::Command;

/// The commit message for handing over the project `name`: a conventional-commit subject with the
/// counts, then one `Kind: A, B` line per kind of change.
pub fn message(name: &str, changes: &Changes) -> String {
    let counts: Vec<String> = [("new", &changes.new), ("redrawn", &changes.redrawn), ("removed", &changes.removed)]
        .iter()
        .filter(|(_, files)| !files.is_empty())
        .map(|(kind, files)| format!("{} {kind}", files.len()))
        .collect();
    if counts.is_empty() {
        return format!("chore: update {name} cursor folder\n");
    }
    format!("feat: hand over {name} cursors ({})\n\n{}\n", counts.join(", "), changes.lines().join("\n"))
}

/// Run git in `folder`, returning its stdout, or its stderr as the error.
fn git(folder: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(folder).args(args).output().map_err(|e| format!("couldn't run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// What `commit` did.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Committed, with this short hash.
    Committed(String),
    /// The folder had no changes to commit.
    NothingToCommit,
    /// The folder isn't in a git repository.
    NotARepo,
}

/// Stage everything in `folder` (new, changed and deleted files) and commit just that, leaving
/// anything else that's staged elsewhere in the repository alone.
pub fn commit(folder: &Path, message: &str) -> Result<Outcome, String> {
    if git(folder, &["rev-parse", "--is-inside-work-tree"]).is_err() {
        return Ok(Outcome::NotARepo);
    }
    git(folder, &["add", "--all", "--", "."])?;
    if git(folder, &["diff", "--cached", "--quiet", "--", "."]).is_ok() {
        return Ok(Outcome::NothingToCommit);
    }
    git(folder, &["commit", "--quiet", "--message", message, "--", "."])?;
    Ok(Outcome::Committed(git(folder, &["rev-parse", "--short", "HEAD"])?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn message_counts_and_lists_the_changes() {
        let c = Changes { new: vec!["A".into(), "B".into()], redrawn: vec!["Pen".into()], ..Changes::default() };
        assert_eq!(message("vectorcraft", &c), "feat: hand over vectorcraft cursors (2 new, 1 redrawn)\n\nNew: A, B\nRedrawn: Pen\n");
        assert_eq!(message("vectorcraft", &Changes::default()), "chore: update vectorcraft cursor folder\n");
    }

    /// A fresh git repository under the system temp dir, with a cursor folder and a file outside it.
    fn repo(name: &str) -> PathBuf {
        let r = std::env::temp_dir().join(format!("nibcraft-test-commit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(r.join("svg-x")).unwrap();
        for args in [&["init", "--quiet"][..], &["config", "user.name", "Test"], &["config", "user.email", "test@example.com"], &["config", "commit.gpgsign", "false"]] {
            git(&r, args).unwrap();
        }
        std::fs::write(r.join("svg-x/a.svg"), "a").unwrap();
        std::fs::write(r.join("outside.txt"), "o").unwrap();
        r
    }

    #[test]
    fn commits_only_the_folder_and_then_has_nothing_to_commit() {
        let r = repo("only");
        git(&r, &["add", "outside.txt"]).unwrap();
        let Outcome::Committed(hash) = commit(&r.join("svg-x"), "feat: x\n").unwrap() else { panic!("not committed") };
        assert_eq!(git(&r, &["show", "--name-only", "--format=%s", &hash]).unwrap(), "feat: x\n\nsvg-x/a.svg");
        assert_eq!(git(&r, &["diff", "--cached", "--name-only"]).unwrap(), "outside.txt", "other staged files stay staged");
        assert_eq!(commit(&r.join("svg-x"), "feat: x\n").unwrap(), Outcome::NothingToCommit);
    }

    #[test]
    fn deletions_are_committed_too() {
        let r = repo("delete");
        commit(&r.join("svg-x"), "feat: x\n").unwrap();
        std::fs::remove_file(r.join("svg-x/a.svg")).unwrap();
        let Outcome::Committed(hash) = commit(&r.join("svg-x"), "chore: y\n").unwrap() else { panic!("not committed") };
        assert_eq!(git(&r, &["show", "--name-status", "--format=", &hash]).unwrap(), "D\tsvg-x/a.svg");
    }

    #[test]
    fn outside_a_repository_nothing_happens() {
        let d = std::env::temp_dir().join(format!("nibcraft-test-commit-norepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(commit(&d, "x").unwrap(), Outcome::NotARepo);
    }
}
