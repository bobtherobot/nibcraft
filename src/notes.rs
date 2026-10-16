//! `DESIGNER_NOTES.md`: the designer's notes for the app's developers (a drawing that needs a
//! behaviour change, a question, an idea). The app's sync prints them; they never change what a
//! cursor is for by themselves.

/// The notes file `pull` starts a project with.
pub fn template(app: &str) -> String {
    format!(
        "# Designer notes for {app}\n\nWrite here anything {app}'s developers should know about the cursors: a question, a drawing that would need the app to behave differently, an idea for a new cursor. They see these notes every time they bring your drawings in.\n\nOne paragraph or bullet per note, on one line. Describe what you mean in your own words; don't name or describe other companies' apps or their cursors.\n\n"
    )
}

/// What the designer wrote in `text`: the notes without the template's own heading and guidance.
pub fn written(text: &str, app: &str) -> String {
    text.strip_prefix(template(app).trim_end()).unwrap_or(text).trim().to_string()
}

/// Vendor names that mustn't appear in what's handed to the app (lowercase, whole words).
const DENY: &[&str] = &["adobe", "illustrator", "photoshop", "acrobat", "indesign", "creative cloud"];

/// The denied names that occur in `text` as whole words, ignoring case.
pub fn vendor_names(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    DENY.iter()
        .copied()
        .filter(|term| lower.match_indices(term).any(|(i, _)| !word(lower[..i].chars().next_back()) && !word(lower[i + term.len()..].chars().next())))
        .collect()
}

/// A warning for the designer when the notes name other companies' products, else `None`.
pub fn warning(text: &str) -> Option<String> {
    let names = vendor_names(text);
    (!names.is_empty()).then(|| {
        format!("DESIGNER_NOTES.md names {}: describe what you mean in your own words instead (the app can't take notes that point at other companies' products).", names.join(", "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_vendor_names_as_whole_words_only() {
        assert_eq!(vendor_names("Like ILLUSTRATOR's pen. Adobe-style."), ["adobe", "illustrator"]);
        assert!(vendor_names("An illustrators' guild; badobe").is_empty());
        assert!(warning("Make the nib bolder.").is_none());
        assert!(warning("As in Photoshop").unwrap().contains("photoshop"));
    }

    #[test]
    fn only_what_the_designer_wrote_is_passed_on() {
        assert_eq!(written(&template("VectorCraft"), "VectorCraft"), "");
        assert_eq!(written(&format!("{}- Pen nib bolder?\n", template("VectorCraft")), "VectorCraft"), "- Pen nib bolder?");
        assert_eq!(written("Own heading.\n", "VectorCraft"), "Own heading.");
    }

    #[test]
    fn the_template_passes_its_own_check() {
        assert!(warning(&template("VectorCraft")).is_none());
    }
}
