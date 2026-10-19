//! The app's side, read and written from outside: the order of its `Cursor` values, and its
//! generated glyphs (VectorCraft: `crates/ui-egui/src/cursors/glyphs.rs`, the `GENERATED` table and
//! one `const` per drawn cursor). The app knows nothing of nibcraft: `pull` reads these files and
//! `push` rewrites the generated part of the glyph file, nothing else.

use crate::emit::{self, fingerprint};
use crate::geom::Pt;
use std::fmt::Write as _;

/// Where the app keeps its generated glyphs.
pub const GLYPHS_RS: &str = "crates/ui-egui/src/cursors/glyphs.rs";
/// Where the app declares its `Cursor` values, in the order the `GENERATED` table follows.
pub const ENUM_RS: &str = "crates/tools/src/lib.rs";

const TABLE_START: &str = "pub(super) const GENERATED: &[(Cursor, &Glyph)] = &[\n";
const TABLE_END: &str = "\n];\n";
const CONSTS_MARK: &str = "\n// Generated glyphs";
const MARK_LINE: &str = "// Generated glyphs, one per drawn cursor.";
const TESTS_MARK: &str = "\n#[cfg(test)]";

/// A `Cursor` value: its name, and whether it carries a value (`RotateToward(u8)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub has_value: bool,
}

/// The variants of `pub enum Cursor` in `src`, in order.
pub fn variants(src: &str) -> Vec<Variant> {
    src.lines()
        .skip_while(|l| !l.starts_with("pub enum Cursor"))
        .skip(1)
        .take_while(|l| !l.starts_with('}'))
        .map(str::trim)
        .filter(|l| l.starts_with(|c: char| c.is_ascii_uppercase()))
        .map(|l| {
            let name: String = l.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            Variant { has_value: l[name.len()..].starts_with('('), name }
        })
        .collect()
}

/// One generated glyph in the app: its cursor id, `const` name, code (doc comment to `};`), the
/// lines of its `parts` list and its hotspot in the drawing.
#[derive(Debug, Clone)]
pub struct AppGlyph {
    pub id: String,
    pub konst: String,
    pub code: String,
    pub parts: String,
    pub hotspot: Pt,
}

impl AppGlyph {
    /// A fingerprint of what it draws ([`shape`]): what a pull records and later compares.
    pub fn fingerprint(&self) -> String {
        fingerprint(shape(&self.parts).as_bytes())
    }
}

/// A glyph's `parts` lines without their triangles: what it draws. A fill's triangles can come out
/// differently for the same points (ear clipping on rounded coordinates), and cover the same area.
pub fn shape(parts: &str) -> String {
    parts
        .lines()
        .map(|l| match l.split_once(", tris: &[") {
            Some((head, rest)) => {
                format!(
                    "{head}{}\n",
                    rest.find("]], ").or_else(|| rest.find("], ")).map_or("", |i| &rest[i + if rest[i..].starts_with("]]") { 2 } else { 1 }..])
                )
            }
            None => format!("{l}\n"),
        })
        .collect()
}

/// The app's glyph file, split where the generated part starts and ends.
#[derive(Debug)]
pub struct GlyphsFile {
    text: String,
    table: usize,
    table_end: usize,
    mark: usize,
    tests: usize,
    pub glyphs: Vec<AppGlyph>,
}

impl GlyphsFile {
    pub fn parse(text: &str) -> Result<Self, String> {
        let missing = |what: &str| format!("{GLYPHS_RS}: can't find {what}; was the file reorganised? nibcraft needs updating to match it");
        let table = text.find(TABLE_START).ok_or_else(|| missing("the GENERATED table"))? + TABLE_START.len();
        let table_end = table + text[table..].find(TABLE_END).ok_or_else(|| missing("the end of the GENERATED table"))?;
        let mark = table_end + text[table_end..].find(CONSTS_MARK).ok_or_else(|| missing("the `// Generated glyphs` line"))? + 1;
        let tests = mark + text[mark..].find(TESTS_MARK).ok_or_else(|| missing("the tests"))? + 1;
        let glyphs = consts(&text[mark..tests])?;
        Ok(GlyphsFile { text: text.to_string(), table, table_end, mark, tests, glyphs })
    }

    pub fn get(&self, id: &str) -> Option<&AppGlyph> {
        self.glyphs.iter().find(|g| g.id == id)
    }

    /// The file with its generated part replaced by `rows`: each a cursor and its glyph's `const`
    /// name and code, in the order of the table. Everything else stays as it is.
    pub fn rewrite(&self, rows: &[(&Variant, String, String)]) -> String {
        let table: String = rows
            .iter()
            .map(|(v, k, _)| {
                if v.has_value {
                    format!("    (Cursor::{}(super::{k}_DRAWN), &{k}),\n", v.name)
                } else {
                    format!("    (Cursor::{}, &{k}),\n", v.name)
                }
            })
            .collect();
        let code: Vec<&str> = rows.iter().map(|(_, _, c)| c.trim_end_matches('\n')).collect();
        let t = &self.text;
        format!(
            "{}{}{}{MARK_LINE}\n\n{}\n{}",
            &t[..self.table],
            table.trim_end_matches('\n'),
            &t[self.table_end..self.mark],
            code.iter().map(|c| format!("{c}\n")).collect::<Vec<_>>().join("\n"),
            &t[self.tests..]
        )
    }
}

/// The generated `const`s in `region`, each from its doc comment to its closing `};`.
fn consts(region: &str) -> Result<Vec<AppGlyph>, String> {
    let mut out = vec![];
    let mut current: Option<String> = None;
    for line in region.split_inclusive('\n') {
        if current.is_none() && line.starts_with("///") {
            current = Some(String::new());
        }
        let Some(code) = current.as_mut() else {
            continue;
        };
        code.push_str(line);
        if line.trim_end() == "};" {
            out.push(glyph(code)?);
            current = None;
        }
    }
    Ok(out)
}

fn glyph(code: &str) -> Result<AppGlyph, String> {
    let bad = |what: &str| format!("{GLYPHS_RS}: a generated glyph without {what}:\n{code}");
    let konst = code.lines().find_map(|l| l.strip_prefix("const ")?.split(':').next()).ok_or_else(|| bad("a `const`"))?.to_string();
    let id = code.lines().find_map(|l| l.trim().strip_prefix("id: \"")?.split('"').next()).ok_or_else(|| bad("an `id`"))?.to_string();
    let parts = code.split_once("    parts: &[\n").and_then(|(_, rest)| rest.split_once("    ],\n")).ok_or_else(|| bad("its `parts`"))?.0.to_string();
    let hotspot = code
        .split_once("the hotspot, at (")
        .and_then(|(_, rest)| rest.split_once(')'))
        .and_then(|(xy, _)| xy.split_once(", "))
        .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)))
        .ok_or_else(|| bad("its hotspot in the drawing"))?;
    Ok(AppGlyph { id, konst, code: code.to_string(), parts, hotspot })
}

/// A number for the SVG: at most three decimals, no trailing zeros.
fn num(x: f32) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// The text after `key` in `line`, up to the first of `ends`.
fn field<'a>(line: &'a str, key: &str, ends: &[char]) -> Option<&'a str> {
    let rest = line.split_once(key)?.1;
    Some(rest.split(ends).next().unwrap_or(rest))
}

/// The `(x, y)` pairs of a part's `points: &[…]`.
fn points(line: &str) -> Option<Vec<Pt>> {
    let list = field(line, "points: &[", &[']'])?;
    list.split(')')
        .filter_map(|p| p.split_once('(').map(|(_, xy)| xy))
        .map(|xy| {
            let (x, y) = xy.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

/// The app's glyph `g` as an SVG a designer can edit: its parts as paths around its hotspot, where
/// the drawing had them. Reading it back gives the same glyph ([`emit::parts`]); the curves of the
/// original drawing are gone (the app keeps only points).
pub fn svg(g: &AppGlyph, app: &str) -> Result<String, String> {
    let at = |(x, y): Pt| (x + g.hotspot.0, y + g.hotspot.1);
    let mut body = String::new();
    for line in g.parts.lines() {
        let bad = || format!("{}: can't read this part of its glyph: {line}", g.id);
        let pts = points(line).ok_or_else(bad)?;
        let colour = if line.contains("white: true") { "#ffffff" } else { "#000000" };
        let mut d = String::new();
        for (i, p) in pts.iter().map(|&p| at(p)).enumerate() {
            let _ = write!(d, "{}{} {} ", if i == 0 { "M" } else { "L" }, num(p.0), num(p.1));
        }
        let d = d.trim_end();
        if line.trim_start().starts_with("GlyphPart::Fill") {
            let _ = writeln!(body, "  <path d=\"{d} Z\" fill=\"{colour}\"/>");
        } else {
            let width = field(line, "width: ", &[',', ' ']).ok_or_else(bad)?;
            let d = if line.contains("closed: true") { format!("{d} Z") } else { d.to_string() };
            let _ = writeln!(body, "  <path d=\"{d}\" fill=\"none\" stroke=\"{colour}\" stroke-width=\"{width}\"/>");
        }
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"32\" height=\"32\" viewBox=\"0 0 32 32\">\n  \
         <title>{}</title>\n  \
         <!-- {app}'s {} cursor as it ships, from its glyph data: straight segments only. -->\n{body}  \
         <circle id=\"hotspot\" cx=\"{}\" cy=\"{}\" r=\"0.25\" fill=\"#ff0000\"/>\n</svg>\n",
        g.id,
        g.id,
        num(g.hotspot.0),
        num(g.hotspot.1)
    ))
}

/// Does the drawing `svg` give exactly the app's glyph `g`?
pub fn same(svg: &[u8], g: &AppGlyph) -> bool {
    let hundredths = |(x, y): Pt| ((x * 100.0).round() as i32, (y * 100.0).round() as i32);
    crate::parse::read(svg).is_ok_and(|d| shape(&emit::parts(&d)) == shape(&g.parts) && hundredths(d.hotspot) == hundredths(g.hotspot))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const ENUM: &str = "pub enum Cursor {\n    /// The arrow.\n    Arrow,\n    RotateToward(u8),\n    Pen,\n}\n";

    /// A glyph file with `rows` in the table and `consts` below the mark.
    pub fn file(rows: &str, consts: &[String]) -> String {
        format!("//! Head.\n\n{TABLE_START}{rows}\n];\n\nfn generated() {{}}\n\n{MARK_LINE}\n\n{}\n#[cfg(test)]\nmod tests {{}}\n", consts.join("\n"))
    }

    /// The round-trip checker's glyph as the app keeps it, for cursor `id`.
    pub fn nib(id: &str) -> String {
        let g = crate::parse::read(include_bytes!("../testdata/round-trip-checker.svg")).unwrap();
        emit::glyph(&emit::const_name(id), id, "A. Designer", &g)
    }

    #[test]
    fn reads_variants_in_order_with_their_values() {
        let v: Vec<(String, bool)> = variants(ENUM).into_iter().map(|v| (v.name, v.has_value)).collect();
        assert_eq!(v, [("Arrow".into(), false), ("RotateToward".into(), true), ("Pen".into(), false)]);
    }

    #[test]
    fn reads_the_generated_glyphs() {
        let f = GlyphsFile::parse(&file("    (Cursor::Pen, &PEN),", &[nib("Pen")])).unwrap();
        let g = f.get("Pen").unwrap();
        assert_eq!((g.konst.as_str(), g.hotspot), ("PEN", (4.0, 4.0)));
        assert!(g.parts.starts_with("        GlyphPart::Fill { points: &[(0.0, 0.0), (4.0, 12.0)"), "{}", g.parts);
        assert!(g.code.starts_with("/// The `Pen` cursor, drawn by A. Designer.") && g.code.ends_with("};\n"));
        assert!(GlyphsFile::parse("//! Nothing here.\n").unwrap_err().contains("GENERATED table"));
    }

    #[test]
    fn rewrites_only_the_generated_part() {
        let old = file("    (Cursor::Pen, &PEN),", &[nib("Pen")]);
        let f = GlyphsFile::parse(&old).unwrap();
        let v = variants(ENUM);
        let rows = vec![(&v[1], "ROTATE_TOWARD".to_string(), nib("RotateToward")), (&v[2], "PEN".to_string(), nib("Pen"))];
        let text = f.rewrite(&rows);
        assert!(
            text.contains(&format!(
                "{TABLE_START}    (Cursor::RotateToward(super::ROTATE_TOWARD_DRAWN), &ROTATE_TOWARD),\n    (Cursor::Pen, &PEN),\n];\n"
            )),
            "{text}"
        );
        assert!(
            text.starts_with("//! Head.\n") && text.contains("\nfn generated() {}\n") && text.ends_with("};\n\n#[cfg(test)]\nmod tests {}\n"),
            "{text}"
        );
        assert_eq!(GlyphsFile::parse(&text).unwrap().glyphs.len(), 2);
        let same_rows = vec![(&v[2], "PEN".to_string(), f.get("Pen").unwrap().code.clone())];
        assert_eq!(f.rewrite(&same_rows), old, "rewriting the same glyphs changes nothing");
    }

    #[test]
    fn a_shape_ignores_the_triangles_only() {
        let fill = |tris: &str| {
            format!("        GlyphPart::Fill {{ points: &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], tris: &[{tris}], white: false }},\n")
        };
        let plain = "        GlyphPart::Fill { points: &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], white: false },\n";
        assert_eq!(shape(&fill("[0, 1, 2], [0, 2, 3]")), plain);
        assert_eq!(shape(&fill("[1, 2, 3], [1, 3, 0]")), plain);
        assert_eq!(shape(&fill("")), plain);
        let stroke = "        GlyphPart::Stroke { points: &[(0.0, 0.0), (1.0, 0.0)], closed: false, width: 1.2, white: false },\n";
        assert_eq!(shape(stroke), stroke);
    }

    #[test]
    fn an_app_glyph_becomes_an_svg_that_reads_back_the_same() {
        let f = GlyphsFile::parse(&file("    (Cursor::Pen, &PEN),", &[nib("Pen")])).unwrap();
        let g = f.get("Pen").unwrap();
        let svg = svg(g, "TestCraft").unwrap();
        assert!(same(svg.as_bytes(), g), "{svg}");
        assert!(svg.contains("<circle id=\"hotspot\" cx=\"4\" cy=\"4\""), "{svg}");
        assert!(!same(include_bytes!("../testdata/round-trip-checker.svg"), &AppGlyph { parts: String::new(), ..g.clone() }));
    }

    /// Every glyph VectorCraft ships reads back the same from its SVG. Needs a VectorCraft
    /// checkout: `NIBCRAFT_APP=/path/to/vectorcraft cargo test -- --ignored`.
    #[test]
    #[ignore = "needs a VectorCraft checkout in NIBCRAFT_APP"]
    fn every_glyph_of_the_app_reads_back_the_same() {
        let app = std::env::var("NIBCRAFT_APP").unwrap();
        let f = GlyphsFile::parse(&std::fs::read_to_string(std::path::Path::new(&app).join(GLYPHS_RS)).unwrap()).unwrap();
        assert!(!f.glyphs.is_empty());
        let mut bad = vec![];
        for g in &f.glyphs {
            let svg = svg(g, "VectorCraft").unwrap();
            if !same(svg.as_bytes(), g) {
                let back = emit::parts(&crate::parse::read(svg.as_bytes()).unwrap());
                let diff: Vec<String> = g
                    .parts
                    .lines()
                    .zip(back.lines())
                    .filter(|(a, b)| a != b)
                    .map(|(a, b)| {
                        let i = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
                        format!("  app:  …{}\n  back: …{}", &a[i.saturating_sub(40)..], &b[i.saturating_sub(40)..])
                    })
                    .collect();
                bad.push(format!("{} ({} vs {} parts):\n{}", g.id, g.parts.lines().count(), back.lines().count(), diff.join("\n")));
            }
        }
        assert!(bad.is_empty(), "{} of {} don't read back the same:\n{}", bad.len(), f.glyphs.len(), bad.join("\n"));
    }
}
