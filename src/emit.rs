//! Writing a parsed glyph as Rust: a `const` of the app's `Glyph` type, as VectorCraft keeps them
//! in `crates/ui-egui/src/cursors/glyphs.rs`.

use crate::geom::Pt;
use crate::parse::{Glyph, Part};
use std::fmt::Write as _;

/// `PenAdd.svg` (or `pen-add.svg`) → `PEN_ADD`: a word starts at each capital after a lowercase
/// letter or digit.
pub fn const_name(file_stem: &str) -> String {
    let mut name = String::new();
    let mut after_word = false;
    for c in file_stem.chars() {
        if c.is_ascii_uppercase() && after_word {
            name.push('_');
        }
        after_word = c.is_ascii_lowercase() || c.is_ascii_digit();
        name.push(if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' });
    }
    if name.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

/// A short, stable fingerprint of some bytes (FNV-1a): of a drawing, or of a glyph's [`parts`].
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", h >> 32)
}

fn num(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    let r = if r == 0.0 { 0.0 } else { r }; // no "-0.0"
    let s = format!("{r:.2}");
    let s = s.trim_end_matches('0');
    if s.ends_with('.') { format!("{s}0") } else { s.to_string() }
}

fn points(pts: &[Pt]) -> String {
    let items: Vec<String> = pts.iter().map(|&(x, y)| format!("({}, {})", num(x), num(y))).collect();
    format!("&[{}]", items.join(", "))
}

/// `g`'s parts as the lines of a glyph's `parts: &[…]` list. Two drawings that give the same lines
/// draw the same cursor, so this is what a pull and a push compare with the app's glyphs.
pub fn parts(g: &Glyph) -> String {
    let mut out = String::new();
    for part in &g.parts {
        match part {
            Part::Fill { points: pts, tris, white } => {
                let t: Vec<String> = tris.iter().map(|t| format!("[{}, {}, {}]", t[0], t[1], t[2])).collect();
                let _ = writeln!(out, "        GlyphPart::Fill {{ points: {}, tris: &[{}], white: {white} }},", points(pts), t.join(", "));
            }
            Part::Stroke { points: pts, closed, width, white } => {
                let _ = writeln!(
                    out,
                    "        GlyphPart::Stroke {{ points: {}, closed: {closed}, width: {}, white: {white} }},",
                    points(pts),
                    num(*width)
                );
            }
        }
    }
    out
}

/// `g` as a `const` named `name`, drawn by `author`. The app names a cursor by its `id` (its
/// `Cursor` value), never by file, so nothing here says SVG.
pub fn glyph(name: &str, id: &str, author: &str, g: &Glyph) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "/// The `{id}` cursor, drawn by {author}.");
    let _ = writeln!(out, "/// Points relative to the hotspot, at ({}, {}) in the drawing.", num(g.hotspot.0), num(g.hotspot.1));
    let _ = writeln!(out, "#[rustfmt::skip]");
    let _ = writeln!(out, "#[allow(clippy::approx_constant, reason = \"generated coordinates, such as 6.28\")]");
    let _ = writeln!(out, "const {name}: Glyph = Glyph {{");
    let _ = writeln!(out, "    id: {id:?},");
    let _ = writeln!(out, "    parts: &[");
    out.push_str(&parts(g));
    let _ = writeln!(out, "    ],");
    let _ = writeln!(out, "}};");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_numbers() {
        assert_eq!(const_name("pen-nib"), "PEN_NIB");
        assert_eq!(const_name("2x"), "_2X");
        assert_eq!(const_name("PenAdd"), "PEN_ADD");
        assert_eq!(const_name("ResizeNeSw"), "RESIZE_NE_SW");
        assert_eq!(const_name("ResizeH"), "RESIZE_H");
        assert_eq!(const_name("RotateToward1"), "ROTATE_TOWARD1");
        assert_eq!(const_name("Pen2Add"), "PEN2_ADD");
        assert_eq!(num(1.0), "1.0");
        assert_eq!(num(-0.001), "0.0");
        assert_eq!(num(1.23456), "1.23");
        assert_eq!(num(1.2), "1.2");
    }

    #[test]
    fn glyph_names_its_id_author_and_hotspot() {
        let g = crate::parse::read(include_bytes!("../testdata/round-trip-checker.svg")).unwrap();
        let out = glyph("NIB", "Nib\"x", "A. Designer", &g);
        assert!(
            out.starts_with("/// The `Nib\"x` cursor, drawn by A. Designer.\n/// Points relative to the hotspot, at (4.0, 4.0) in the drawing.\n"),
            "{out}"
        );
        assert!(out.contains(&format!("    id: \"Nib\\\"x\",\n    parts: &[\n{}    ],\n}};\n", parts(&g))), "{out}");
        assert!(!out.to_lowercase().contains("svg") && !out.contains("fp:"), "the app names cursors by id, and keeps no fingerprints: {out}");
    }

    #[test]
    fn coordinates_that_look_like_constants_dont_trip_clippy() {
        let g = crate::parse::read(include_bytes!("../testdata/round-trip-checker.svg")).unwrap();
        let out = glyph("NIB", "Nib", "A. Designer", &g);
        assert!(out.contains("#[allow(clippy::approx_constant, reason = \"generated coordinates, such as 6.28\")]\nconst NIB"), "{out}");
    }
}
