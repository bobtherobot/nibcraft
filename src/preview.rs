//! An HTML swatch book of a project's drawings, drawn the way VectorCraft's `draw_glyph`
//! (`crates/ui-egui/src/cursors.rs`) draws them, so the designer sees each cursor as the app will
//! before pushing it.

use crate::check::{Todo, TodoKind};
use crate::emit::const_name;
use crate::geom::Pt;
use crate::parse::{Glyph, Part};
use std::fmt::Write as _;
use std::path::Path;

/// One SVG's entry in the book: its file name, its glyph or why it couldn't be read, its
/// description from the app's cursor list (none for a drawing the app doesn't use), and the SVG's
/// fingerprint (as in generated glyphs).
pub struct Card {
    pub file: String,
    pub glyph: Result<Glyph, String>,
    pub description: Option<String>,
    pub fingerprint: String,
    /// It still holds a seed's note ([`crate::parse::SEED_NOTE_ID`]).
    pub seed_note: bool,
}

/// `draw_glyph`'s halos: 3 points under a fill, 1.8 points wider than a stroke.
const FILL_HALO: f32 = 3.0;
const STROKE_HALO: f32 = 1.8;
/// The SVG grid the cursors are drawn on, in points.
const GRID: f32 = 32.0;
/// Pixel sizes of the enlarged and actual-size views.
const BIG: f32 = 160.0;
const ACTUAL: f32 = 32.0;

fn num(v: f32) -> String {
    let s = format!("{:.2}", (v * 100.0).round() / 100.0);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn points(pts: &[Pt]) -> String {
    pts.iter().map(|&(x, y)| format!("{},{}", num(x), num(y))).collect::<Vec<_>>().join(" ")
}

fn colour(white: bool) -> &'static str {
    if white { "#fff" } else { "#000" }
}

/// A line part as an SVG element: `polygon` when closed, `polyline` when open.
fn line(pts: &[Pt], closed: bool, stroke: &str, width: f32) -> String {
    let tag = if closed { "polygon" } else { "polyline" };
    format!(r#"<{tag} points="{}" fill="none" stroke="{stroke}" stroke-width="{}"/>"#, points(pts), num(width))
}

/// The glyph's shapes in `draw_glyph`'s order: every halo first, then every part.
fn shapes(g: &Glyph) -> String {
    let mut out = String::new();
    for part in &g.parts {
        match part {
            Part::Fill { points: pts, white: false, .. } => out.push_str(&line(pts, true, "#fff", FILL_HALO)),
            Part::Stroke { points: pts, closed, width, white: false } => out.push_str(&line(pts, *closed, "#fff", width + STROKE_HALO)),
            _ => {}
        }
    }
    for part in &g.parts {
        match part {
            // A concave fill's triangles cover the same area as its outline, so draw the outline.
            Part::Fill { points: pts, white, .. } => {
                let outline = if *white { String::new() } else { r##" stroke="#000" stroke-width="1""##.to_string() };
                let _ = write!(out, r#"<polygon points="{}" fill="{}"{outline}/>"#, points(pts), colour(*white));
            }
            Part::Stroke { points: pts, closed, width, white } => out.push_str(&line(pts, *closed, colour(*white), *width)),
        }
    }
    out
}

/// The glyph as an SVG `size` pixels square, laid on the grid by its hotspot.
pub fn glyph_svg(g: &Glyph, size: f32) -> String {
    let (x, y) = g.hotspot;
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}" viewBox="{} {} {g32} {g32}">{}</svg>"#,
        num(-x),
        num(-y),
        shapes(g),
        s = num(size),
        g32 = num(GRID)
    )
}

/// A red crosshair on the hotspot, for the enlarged view.
fn with_hotspot(svg: &str) -> String {
    let mark = r##"<g class="hotspot" stroke="#e00" stroke-width="0.25"><line x1="-2" y1="0" x2="2" y2="0"/><line x1="0" y1="-2" x2="0" y2="2"/></g>"##;
    svg.replacen("</svg>", &format!("{mark}</svg>"), 1)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The seed note as the preview shows it, in red at the top left of the enlarged view: on a seed,
/// what to do with it; on a drawing, that the note must go.
fn with_seed_note(svg: &str, g: &Glyph, file: &str, seed: bool) -> String {
    let (x, y) = g.hotspot;
    let lines: Vec<String> = if seed {
        let id = file.split('-').next().unwrap_or(file);
        vec!["Seed, ignored: every pull overwrites it.".into(), format!("To draw, save a copy as {id}.svg.")]
    } else {
        vec!["Delete the seed note before handing over.".into()]
    };
    let text: String = lines
        .iter()
        .enumerate()
        .map(|(i, l)| format!(r#"<text x="{}" y="{}">{}</text>"#, num(0.5 - x), num(1.6 * (i + 1) as f32 - y), esc(l)))
        .collect();
    let note = format!(r##"<g class="seed-note" fill="#e00" font-family="sans-serif" font-size="1.4">{text}</g>"##);
    svg.replacen("</svg>", &format!("{note}</svg>"), 1)
}

fn card(c: &Card) -> String {
    let stem = Path::new(&c.file).file_stem().and_then(|s| s.to_str()).unwrap_or("glyph");
    let file = esc(&c.file);
    let desc = match &c.description {
        Some(d) => format!(r#"<p class="desc">{}</p>"#, esc(d)),
        None => r#"<p class="desc missing">Not in the cursor list: the app doesn't use it.</p>"#.to_string(),
    };
    let class = if c.description.is_some() { "card" } else { "card unused" };
    match &c.glyph {
        Ok(g) => format!(
            r#"<figure class="{class}"><div class="swatch big">{}</div><div class="swatch actual">{}</div><figcaption><b>{file}</b><code>{} ({})</code>{desc}</figcaption></figure>"#,
            if c.seed_note { with_seed_note(&with_hotspot(&glyph_svg(g, BIG)), g, &c.file, c.description.is_none()) } else { with_hotspot(&glyph_svg(g, BIG)) },
            glyph_svg(g, ACTUAL),
            esc(&const_name(stem)),
            esc(&c.fingerprint)
        ),
        Err(e) => format!(r#"<figure class="{class} failed"><div class="error">{}</div><figcaption><b>{file}</b>{desc}</figcaption></figure>"#, esc(e)),
    }
}

/// The letter and word a to-do is marked with in the side column.
fn kind_mark(kind: TodoKind) -> (&'static str, &'static str) {
    match kind {
        TodoKind::Missing => ("m", "missing"),
        TodoKind::Fix => ("f", "fix"),
        TodoKind::Changed => ("c", "changed"),
    }
}

/// One to-do as an entry in the side column: its mark and name, and its description when opened.
fn todo_item(t: &Todo) -> String {
    let (letter, word) = kind_mark(t.kind);
    format!(
        r#"<li><details><summary><span class="kind {word}" title="{word}">[{letter}]</span> {}</summary><p class="desc">{}</p></details></li>"#,
        esc(&t.id),
        esc(&t.description)
    )
}

/// The side column listing what's left for the designer, or nothing when nothing is.
fn todo_column(todos: &[Todo]) -> String {
    if todos.is_empty() {
        return String::new();
    }
    format!(
        r#"<aside class="todos"><h2>To do ({})</h2><p class="hint">[m] missing: the app needs it, no drawing yet. [f] fix: the drawing doesn't convert. Open one for its description.</p><ul>{}</ul></aside>"#,
        todos.len(),
        todos.iter().map(todo_item).collect::<String>()
    )
}

const STYLE: &str = r#"
:root { color-scheme: light dark; --page: #fafafa; --text: #222; --card: #fff; --line: #ddd; --bg: #f2f2f2; }
@media (prefers-color-scheme: dark) { :root { --page: #1b1b1b; --text: #eee; --card: #262626; --line: #3a3a3a; } }
body { margin: 0; padding: 16px; background: var(--page); color: var(--text); font: 14px/1.4 system-ui, sans-serif; }
header { display: flex; flex-wrap: wrap; gap: 8px 24px; align-items: baseline; margin-bottom: 16px; }
h1 { font-size: 20px; margin: 0; }
.controls label { margin-right: 12px; white-space: nowrap; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(196px, 1fr)); gap: 12px; }
.card { margin: 0; padding: 12px; background: var(--card); border: 1px solid var(--line); border-radius: 8px; display: flex; flex-direction: column; align-items: center; gap: 8px; }
.swatch { background: var(--bg); border-radius: 4px; display: flex; }
.swatch svg { display: block; }
.big svg { outline: 1px dashed color-mix(in srgb, currentColor 25%, transparent); }
.actual { padding: 8px; }
figcaption { display: flex; flex-direction: column; align-items: center; text-align: center; word-break: break-all; }
figcaption code { font-size: 11px; opacity: 0.7; }
.failed { border-color: #d33; }
.error { color: #d33; font-size: 12px; min-height: 160px; display: flex; align-items: center; text-align: center; word-break: break-word; }
.bad { color: #d33; }
.desc { margin: 4px 0 0; font-size: 12px; text-align: left; white-space: pre-wrap; word-break: normal; overflow-wrap: anywhere; }
.missing { color: #c70; }
.unused { opacity: 0.75; }
.advice { margin: 0 0 16px; padding-left: 20px; color: #c70; }
.extras { grid-column: 1 / -1; font-size: 15px; margin: 12px 0 0; }
.problems { margin: 0 0 16px; padding-left: 20px; color: #d33; }
.layout { display: grid; grid-template-columns: 1fr; gap: 16px; align-items: start; }
.layout:has(.todos) { grid-template-columns: minmax(0, 1fr) 280px; }
@media (max-width: 720px) { .layout:has(.todos) { grid-template-columns: 1fr; } }
.todos { position: sticky; top: 16px; max-height: calc(100vh - 32px); overflow-y: auto; padding: 12px; background: var(--card); border: 1px dashed #38c; border-radius: 8px; }
.todos h2 { font-size: 15px; margin: 0; }
.todos .hint { margin: 4px 0 8px; font-size: 12px; opacity: 0.7; }
.todos ul { list-style: none; margin: 0; padding: 0; }
.todos li { padding: 4px 0; border-top: 1px solid var(--line); }
.todos summary { cursor: pointer; overflow-wrap: anywhere; }
.kind { font: 12px ui-monospace, monospace; }
.kind.missing { color: #38c; }
.kind.fix { color: #d33; }
body:has(#bg-mid:checked) { --bg: #8a8a8a; }
body:has(#bg-dark:checked) { --bg: #202020; }
body:has(#bg-check:checked) .swatch { background: repeating-conic-gradient(#ccc 0 25%, #fff 0 50%) 0 0 / 16px 16px; }
body:not(:has(#show-hotspot:checked)) .hotspot { display: none; }
"#;

/// The whole book for the project `folder`: the check's problems and advice, then a card per
/// drawing the app uses in the order given, then the drawings it doesn't, with what's left for the
/// designer in a column on the right.
pub fn page(folder: &str, cards: &[Card], extras: &[Card], problems: &[String], advice: &[String], todos: &[Todo]) -> String {
    let folder = esc(folder);
    let mut summary = match problems.len() {
        0 => format!("{} cursors, no problems", cards.len()),
        1 => format!(r#"{} cursors, <span class="bad">1 problem</span>"#, cards.len()),
        n => format!(r#"{} cursors, <span class="bad">{n} problems</span>"#, cards.len()),
    };
    if !todos.is_empty() {
        summary.push_str(&format!(", {} to do", todos.len()));
    }
    let problems = if problems.is_empty() {
        String::new()
    } else {
        format!(r#"<ul class="problems">{}</ul>"#, problems.iter().map(|p| format!("<li>{}</li>", esc(p))).collect::<String>())
    };
    let advice = if advice.is_empty() {
        String::new()
    } else {
        format!(r#"<ul class="advice">{}</ul>"#, advice.iter().map(|a| format!("<li>{}</li>", esc(a))).collect::<String>())
    };
    let mut body: String = cards.iter().map(card).collect();
    if !extras.is_empty() {
        body.push_str(&format!(r#"<h2 class="extras">Other drawings ({}): not in the cursor list, so the app ignores them</h2>"#, extras.len()));
        body.extend(extras.iter().map(card));
    }
    let todos = todo_column(todos);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Cursor Swatches</title>
<style>{STYLE}</style>
</head>
<body>
<header>
<h1>Cursor swatches: {folder}</h1>
<span>{summary}</span>
<span class="controls">Background:
<label><input type="radio" name="bg" id="bg-light" checked> light</label>
<label><input type="radio" name="bg" id="bg-mid"> grey</label>
<label><input type="radio" name="bg" id="bg-dark"> dark</label>
<label><input type="radio" name="bg" id="bg-check"> checker</label>
<label><input type="checkbox" id="show-hotspot" checked> hotspot</label>
</span>
</header>
{problems}
{advice}
<div class="layout">
<main class="grid">{body}</main>
{todos}
</div>
</body>
</html>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::read;

    fn nib() -> Glyph {
        read(include_bytes!("../testdata/round-trip-checker.svg")).unwrap()
    }

    #[test]
    fn halos_are_drawn_under_all_ink() {
        let svg = glyph_svg(&nib(), 32.0);
        let last_halo = svg.rfind(r##"stroke="#fff""##).unwrap();
        let first_ink = svg.find(r##"fill="#000""##).unwrap();
        assert!(last_halo < first_ink, "{svg}");
        // A black fill gets a 3-point halo and a 1-point outline; the 1.2 stem a 3-point halo.
        assert!(svg.contains(r##"stroke="#fff" stroke-width="3""##), "{svg}");
        assert!(svg.contains(r##"stroke="#000" stroke-width="1""##), "{svg}");
        assert!(svg.contains(r##"stroke="#000" stroke-width="1.2""##), "{svg}");
    }

    #[test]
    fn glyph_is_placed_by_its_hotspot() {
        let g = nib();
        assert_eq!(g.hotspot, (4.0, 4.0));
        assert!(glyph_svg(&g, 32.0).contains(r#"viewBox="-4 -4 32 32""#));
    }

    #[test]
    fn white_parts_have_no_halo_or_outline() {
        let g = Glyph { parts: vec![Part::Fill { points: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)], tris: vec![], white: true }], hotspot: (0.0, 0.0) };
        let svg = glyph_svg(&g, 32.0);
        assert!(svg.contains(r##"fill="#fff""##) && !svg.contains("stroke="), "{svg}");
    }

    #[test]
    fn page_has_a_card_per_drawing_then_the_unused_ones_and_lists_problems_and_advice() {
        let html = page(
            "vectorcraft",
            &[Card { file: "Pen.svg".into(), glyph: Ok(nib()), description: Some("Pen <tool>.".into()), fingerprint: "c3f8dae0".into(), seed_note: false }],
            &[Card { file: "bad<1>.svg".into(), glyph: Err("no element with id=\"hotspot\"".into()), description: None, fingerprint: "00000000".into(), seed_note: false }],
            &["Lasso.svg: not a valid SVG".into()],
            &["Pull again".into()],
            &[],
        );
        assert_eq!(html.matches(r#"<figure class="card"#).count(), 2);
        assert!(html.contains("PEN (c3f8dae0)") && html.contains("Pen.svg") && html.contains("vectorcraft"));
        assert!(html.contains("Pen &lt;tool&gt;."), "descriptions are escaped");
        assert!(html.contains("Other drawings (1)") && html.contains("the app doesn't use it"), "{html}");
        assert!(html.contains("bad&lt;1&gt;.svg") && html.contains("id=&quot;hotspot&quot;"), "names and errors are escaped");
        assert!(html.contains("1 problem") && html.contains("<li>Lasso.svg: not a valid SVG</li>"));
        assert!(html.contains(r#"<ul class="advice"><li>Pull again</li></ul>"#));
    }

    #[test]
    fn todos_are_listed_in_a_side_column_with_their_kind_and_a_count() {
        let todo = |id: &str, kind, description: &str| Todo { id: id.into(), kind, description: description.into() };
        let todos = [todo("Lasso<1>", TodoKind::Missing, "Lasso <tool>."), todo("Crosshair", TodoKind::Fix, "Gap.")];
        let html = page("vectorcraft", &[], &[], &[], &[], &todos);
        assert!(html.contains(r#"<aside class="todos"><h2>To do (2)</h2>"#), "{html}");
        assert!(!html.contains("<figure"), "to-dos get no cards");
        assert!(html.contains(r#"<summary><span class="kind missing" title="missing">[m]</span> Lasso&lt;1&gt;</summary>"#), "{html}");
        assert!(html.contains("Lasso &lt;tool&gt;."));
        assert!(html.contains(r#">[f]</span> Crosshair"#), "{html}");
        assert!(html.contains("2 to do"));
    }

    #[test]
    fn no_side_column_when_nothing_is_left_to_draw() {
        assert!(!page("vectorcraft", &[], &[], &[], &[], &[]).contains(r#"<aside"#));
    }
}
