//! nibcraft's seeds for VectorCraft: how VectorCraft draws, in its own code, the cursors nobody has
//! drawn yet, written as SVG starting points. This file is nibcraft's, not VectorCraft's: `pull`
//! copies it into a scratch worktree of the VectorCraft checkout as
//! `crates/ui-egui/src/cursors/nibcraft_seeds.rs` (a test-only child of the `cursors` module, so it
//! reaches its private drawing code), runs `write_seeds`, and throws the worktree's changes away.
//! When VectorCraft's drawing code changes shape, this file follows it.
//!
//! Written: every cursor in `GLYPHS` drawn in code, with no generated glyph and no stand-in (a
//! drawn cursor's SVG comes from its glyph data, on nibcraft's side). The white strokes in the
//! code-drawn glyphs are their halos, which nibcraft adds by itself, so they're left out; white
//! fills (hollow shapes) stay.

use egui::epaint::{CircleShape, ColorMode, PathShape};
use egui::{Color32, Pos2, Rect, Shape, pos2, vec2};
use vectorcraft_tools::Cursor;

use super::{GLYPHS, HALO, INK, glyph, glyphs};

/// The side of nibcraft's canvas (`viewBox="0 0 32 32"`), in points.
const CANVAS: f32 = 32.0;

/// Writes `<id>.svg` for each code-drawn cursor into the folder in `NIBCRAFT_SEEDS_DIR`.
#[test]
#[ignore = "run by nibcraft's pull"]
fn write_seeds() {
    let dir = std::path::PathBuf::from(std::env::var_os("NIBCRAFT_SEEDS_DIR").expect("NIBCRAFT_SEEDS_DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    for c in GLYPHS {
        if glyphs::generated(c).is_some() || c.stand_in().is_some() {
            continue;
        }
        let Some(elements) = glyph(c, Pos2::ZERO).and_then(|shapes| shape_elements(&shapes)) else { continue };
        let id = id(c);
        if let Some(svg) = svg(&id, &elements) {
            std::fs::write(dir.join(format!("{id}.svg")), svg).unwrap();
        }
    }
}

/// Hand-drawn shapes as elements; `None` if one can't be written.
fn shape_elements(shapes: &[Shape]) -> Option<Vec<Element>> {
    let parts = shapes.iter().map(element).collect::<Result<Vec<_>, ()>>().ok()?;
    Some(merge_outlines(parts.into_iter().flatten().collect()))
}

/// The cursor's id: its `Cursor` value without a payload (`RotateToward(1)` is `RotateToward`).
fn id(c: Cursor) -> String {
    let name = format!("{c:?}");
    name.split('(').next().unwrap_or_default().to_string()
}

/// One element to write: a filled or stroked outline, in points relative to the hotspot.
enum Element {
    Polygon { points: Vec<Pos2>, closed: bool, fill: Option<Color32>, stroke: Option<(f32, Color32)> },
    Circle { centre: Pos2, radius: f32, fill: Option<Color32>, stroke: Option<(f32, Color32)> },
}

impl Element {
    /// The area it covers, strokes included.
    fn bounds(&self) -> Rect {
        match self {
            Element::Polygon { points, stroke, .. } => Rect::from_points(points).expand(stroke.map_or(0.0, |(w, _)| w / 2.0)),
            Element::Circle { centre, radius, stroke, .. } => {
                Rect::from_center_size(*centre, vec2(2.0 * radius, 2.0 * radius)).expand(stroke.map_or(0.0, |(w, _)| w / 2.0))
            }
        }
    }
}

/// The ink of a shape: `None` for halo strokes and invisible parts. Shapes this module doesn't
/// know give `Err`, so the cursor is left out rather than drawn wrong.
fn element(s: &Shape) -> Result<Option<Element>, ()> {
    let ink = |c: Color32| c != Color32::TRANSPARENT;
    // Cursors are black and white only: another colour would be written wrongly, so refuse it.
    if shape_colours(s).unwrap_or_default().iter().any(|&c| ink(c) && c != INK && c != HALO) {
        return Err(());
    }
    // A white stroke is a halo: nibcraft draws its own.
    let stroke = |w: f32, c: Color32| (w > 0.0 && ink(c) && c != HALO).then_some((w, c));
    Ok(match s {
        Shape::Noop => None,
        Shape::LineSegment { points, stroke: st } => {
            stroke(st.width, st.color).map(|st| Element::Polygon { points: points.to_vec(), closed: false, fill: None, stroke: Some(st) })
        }
        Shape::Path(PathShape { points, .. }) if points.len() < 2 => None,
        Shape::Path(PathShape { points, closed, fill, stroke: st }) => {
            let ColorMode::Solid(colour) = st.color else { return Err(()) };
            let fill = (*closed && ink(*fill)).then_some(*fill);
            let st = stroke(st.width, colour);
            (fill.is_some() || st.is_some()).then(|| Element::Polygon { points: points.clone(), closed: *closed, fill, stroke: st })
        }
        Shape::Circle(CircleShape { center, radius, fill, stroke: st }) => {
            let fill = ink(*fill).then_some(*fill);
            let st = stroke(st.width, st.color);
            (fill.is_some() || st.is_some()).then_some(Element::Circle { centre: *center, radius: *radius, fill, stroke: st })
        }
        _ => return Err(()),
    })
}

/// The colours a shape paints with, `None` for a shape [`element`] doesn't know.
fn shape_colours(s: &Shape) -> Option<Vec<Color32>> {
    match s {
        Shape::Noop => Some(vec![]),
        Shape::LineSegment { stroke, .. } => Some(vec![stroke.color]),
        Shape::Path(PathShape { fill, stroke, .. }) => match stroke.color {
            ColorMode::Solid(c) => Some(vec![*fill, c]),
            ColorMode::UV(_) => None,
        },
        Shape::Circle(CircleShape { fill, stroke, .. }) => Some(vec![*fill, stroke.color]),
        _ => None,
    }
}

/// A fill followed by the outline of the same shape as one element, so the designer edits one path.
fn merge_outlines(elements: Vec<Element>) -> Vec<Element> {
    let mut out: Vec<Element> = Vec::with_capacity(elements.len());
    for e in elements {
        if let (
            Some(Element::Polygon { points: filled, fill: Some(_), stroke: outline @ None, .. }),
            Element::Polygon { points, closed: true, fill: None, stroke: Some(stroke) },
        ) = (out.last_mut(), &e)
            && filled == points
        {
            *outline = Some(*stroke);
            continue;
        }
        out.push(e);
    }
    out
}

/// A number for the SVG: at most three decimals, no trailing zeros.
fn num(x: f32) -> String {
    let s = format!("{:.3}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn colour(c: Color32) -> &'static str {
    if c == HALO { "#ffffff" } else { "#000000" }
}

/// Cursor `id`'s elements as an SVG on nibcraft's canvas, its hotspot marked; `None` when
/// there's nothing to draw.
fn svg(id: &str, elements: &[Element]) -> Option<String> {
    let bounds = elements.iter().map(Element::bounds).reduce(|a, b| a.union(b))?;
    // The glyph centred on the canvas, the hotspot on a half point.
    let hotspot = ((pos2(CANVAS, CANVAS) / 2.0 - bounds.center().to_vec2()) * 2.0).round() / 2.0;
    let at = |p: Pos2| p + hotspot.to_vec2();
    let paint = |fill: Option<Color32>, stroke: Option<(f32, Color32)>| {
        let fill = fill.map_or("none", colour);
        match stroke {
            Some((w, c)) => format!(r#"fill="{fill}" stroke="{}" stroke-width="{}""#, colour(c), num(w)),
            None => format!(r#"fill="{fill}""#),
        }
    };
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"32\" height=\"32\" viewBox=\"0 0 32 32\">\n  \
         <title>{id}</title>\n  \
         <!-- How VectorCraft draws its {id} cursor today. Without the white halos: nibcraft adds those. -->\n"
    );
    for e in elements {
        match e {
            Element::Polygon { points, closed, fill, stroke } => {
                let mut d = String::new();
                for (i, p) in points.iter().map(|&p| at(p)).enumerate() {
                    d.push_str(&format!("{}{} {} ", if i == 0 { "M" } else { "L" }, num(p.x), num(p.y)));
                }
                if *closed {
                    d.push('Z');
                }
                out.push_str(&format!("  <path d=\"{}\" {}/>\n", d.trim_end(), paint(*fill, *stroke)));
            }
            Element::Circle { centre, radius, fill, stroke } => {
                let c = at(*centre);
                out.push_str(&format!("  <circle cx=\"{}\" cy=\"{}\" r=\"{}\" {}/>\n", num(c.x), num(c.y), num(*radius), paint(*fill, *stroke)));
            }
        }
    }
    out.push_str(&format!("  <circle id=\"hotspot\" cx=\"{}\" cy=\"{}\" r=\"0.25\" fill=\"#ff0000\"/>\n</svg>\n", num(hotspot.x), num(hotspot.y)));
    Some(out)
}
