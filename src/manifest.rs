//! An app's cursor manifest: the cursors it needs, what each is for and where its hotspot is. The
//! app owns it (VectorCraft: `crates/tools/src/cursors.toml`); `pull` copies it into the project as
//! `.nibcraft/cursors.toml`, and designers draw what it lists. Nothing here decides what a cursor is for.

use serde::Deserialize;

/// Where an app checkout keeps its manifest.
pub const APP_MANIFEST: &str = "crates/tools/src/cursors.toml";

#[derive(Debug, Deserialize)]
pub struct Manifest {
    /// The app's name as it's written (`VectorCraft`).
    pub app: Option<String>,
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
    /// How sure the app is of this description, 0 to 10 (from the research behind it), so the
    /// designer knows how closely to follow it. Kept as written: a number out of range is ignored
    /// rather than making the whole list unreadable.
    pub confidence: Option<toml::Value>,
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
        if let Some(c) = self.confidence() {
            out.push_str(&format!("\n\nConfidence: {c}/10 ({})", follow(c)));
        }
        out
    }

    /// The cursor's confidence, when it's a whole number from 0 to 10.
    pub fn confidence(&self) -> Option<u8> {
        self.confidence.as_ref()?.as_integer().filter(|c| (0..=10).contains(c)).map(|c| c as u8)
    }
}

/// How closely to follow a description rated `c` out of 10.
fn follow(c: u8) -> &'static str {
    match c {
        9.. => "follow the description closely",
        7..=8 => "follow its meaning and shape; the details are yours",
        4..=6 => "keep the meaning; the shape is yours",
        1..=3 => "free to design, keeping the meaning, the hotspot and the family it belongs to",
        0 => "a placeholder",
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
confidence = 7

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
        assert_eq!(
            m.cursor[0].description(),
            "Shown when:\n- The Pen tool.\n- Curvature away from points.\n\nHotspot: The nib's tip.\n\nLook: A nib.\n\nConfidence: 7/10 (follow its meaning and shape; the details are yours)"
        );
        assert!(!m.cursor[2].description().contains("Look:"));
        assert!(!m.cursor[2].description().contains("Confidence:"));
    }

    #[test]
    fn confidence_says_how_closely_to_follow_and_ignores_numbers_out_of_range() {
        let with = |c: &str| Manifest::parse(&format!("[[cursor]]\nid = \"Pen\"\nconfidence = {c}\n")).unwrap().cursor[0].description();
        assert!(with("10").contains("Confidence: 10/10 (follow the description closely)"), "{}", with("10"));
        assert!(with("5").contains("Confidence: 5/10 (keep the meaning; the shape is yours)"));
        assert!(with("2").contains("Confidence: 2/10 (free to design, keeping the meaning, the hotspot and the family it belongs to)"));
        assert!(with("0").contains("Confidence: 0/10 (a placeholder)"));
        for bad in ["11", "-1", "7.5", "\"high\""] {
            assert!(!with(bad).contains("Confidence:"), "{bad}");
        }
    }

    #[test]
    fn a_broken_list_is_an_error() {
        assert!(Manifest::parse("[[cursor]]\nshown_when = 3\n").unwrap_err().contains("can't read the cursor list"));
    }
}
