//! An app's cursor manifest: the cursors it needs, what each is for and where its hotspot is. The
//! app owns it (VectorCraft: `crates/tools/src/cursors.toml`); `pull` copies it into the project as
//! `.cursorcraft/cursors.toml`, and designers draw what it lists. Nothing here decides what a cursor is for.

use serde::Deserialize;

/// Where an app checkout keeps its manifest.
pub const APP_MANIFEST: &str = "crates/tools/src/cursors.toml";

#[derive(Debug, Deserialize)]
pub struct Manifest {
    /// The app's name as it's written (`VectorCraft`).
    pub app: Option<String>,
    /// The command, run in the app's checkout, that writes the cursors the app draws in its own
    /// code as `<id>.svg` into the folder added after it (`pull` turns them into seeds).
    pub seeds: Option<Vec<String>>,
    #[serde(default)]
    pub cursor: Vec<Cursor>,
    #[serde(default)]
    pub not_needed: Vec<NotNeeded>,
}

/// One cursor the app needs. Its `id` is the name the designer draws (`svg/<id>.svg`).
#[derive(Debug, Deserialize)]
pub struct Cursor {
    pub id: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub shown_when: Vec<String>,
    #[serde(default)]
    pub hotspot: String,
    pub look: Option<String>,
    /// False for a cursor the operating system draws on purpose: designers never see it.
    #[serde(default = "yes")]
    pub designer: bool,
}

/// A tool that deliberately has no cursor of its own, so nobody draws one.
#[derive(Debug, Deserialize)]
pub struct NotNeeded {
    pub what: String,
    #[serde(default)]
    pub why: String,
}

fn yes() -> bool {
    true
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("can't read the cursor list: {e}"))
    }

    /// The cursors designers draw, in the manifest's order.
    pub fn designer_cursors(&self) -> impl Iterator<Item = &Cursor> {
        self.cursor.iter().filter(|c| c.designer)
    }

    /// Is `id` a cursor designers draw?
    pub fn wants(&self, id: &str) -> bool {
        self.designer_cursors().any(|c| c.id == id)
    }

    /// The groups of designer cursors, in order of first appearance.
    pub fn groups(&self) -> Vec<&str> {
        let mut out: Vec<&str> = vec![];
        for c in self.designer_cursors() {
            if !out.contains(&c.group.as_str()) {
                out.push(&c.group);
            }
        }
        out
    }
}

impl Cursor {
    /// The cursor's description as plain text: when it shows, its hotspot and its look.
    pub fn description(&self) -> String {
        let mut out = String::from("Shown when:\n");
        for s in &self.shown_when {
            out.push_str(&format!("- {s}\n"));
        }
        out.push_str(&format!("\nHotspot: {}", self.hotspot));
        if let Some(look) = &self.look {
            out.push_str(&format!("\n\nLook: {look}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = r#"
[[cursor]]
id = "Pen"
group = "Pen"
shown_when = ["The Pen tool.", "Curvature away from points."]
hotspot = "The nib's tip."
look = "A nib."

[[cursor]]
id = "NotAllowed"
group = "Navigation"
shown_when = ["Nothing selected."]
hotspot = "The centre."
designer = false

[[cursor]]
id = "PenAdd"
group = "Pen"
shown_when = ["Over a segment."]
hotspot = "The nib's tip."

[[not_needed]]
what = "Liquify tools"
why = "The app draws the brush."
"#;

    #[test]
    fn reads_cursors_and_leaves_out_the_systems() {
        let m = Manifest::parse(TOML).unwrap();
        let ids: Vec<&str> = m.designer_cursors().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["Pen", "PenAdd"]);
        assert!(m.wants("Pen") && !m.wants("NotAllowed") && !m.wants("Lasso"));
        assert_eq!(m.groups(), ["Pen"]);
        assert_eq!(m.not_needed[0].what, "Liquify tools");
    }

    #[test]
    fn a_description_lists_when_it_shows_its_hotspot_and_look() {
        let m = Manifest::parse(TOML).unwrap();
        assert_eq!(m.cursor[0].description(), "Shown when:\n- The Pen tool.\n- Curvature away from points.\n\nHotspot: The nib's tip.\n\nLook: A nib.");
        assert!(!m.cursor[2].description().contains("Look:"));
    }

    #[test]
    fn a_broken_list_is_an_error() {
        assert!(Manifest::parse("[[cursor]]\nshown_when = 3\n").unwrap_err().contains("can't read the cursor list"));
    }
}
