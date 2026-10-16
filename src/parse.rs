//! Reading a cursor SVG into glyph parts: filled shapes and stroked lines, in black (ink) or white
//! (halo colour), with coordinates relative to the hotspot.
//!
//! Supported: paths and the basic shapes (`rect`, `circle`, `ellipse`, `line`, `polyline`,
//! `polygon`), transforms and groups, solid black or white fills and strokes. Rejected with an
//! error: gradients, patterns, text, images, dashes, partial opacity, other colours.

use crate::geom::{self, Pt};
use usvg::tiny_skia_path::PathSegment;

/// One drawn part of a glyph, in document order.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// A filled closed shape. `tris` is empty when the shape is convex (egui fills it directly).
    Fill { points: Vec<Pt>, tris: Vec<[u16; 3]>, white: bool },
    /// A stroked line, open or closed.
    Stroke { points: Vec<Pt>, closed: bool, width: f32, white: bool },
}

/// A parsed glyph: its parts, relative to the hotspot.
#[derive(Debug)]
pub struct Glyph {
    pub parts: Vec<Part>,
    pub hotspot: Pt,
}

const HOTSPOT_ID: &str = "hotspot";

/// The id of the note `pull` writes into a seed (and any id an editor makes from it, such as
/// `seed-note_1`). Never part of the glyph: [`has_seed_note`] finds it so `check` can ask for it to go.
pub const SEED_NOTE_ID: &str = "seed-note";

/// Does this SVG still hold a seed's note?
pub fn has_seed_note(svg: &[u8]) -> bool {
    let text = String::from_utf8_lossy(svg);
    text.contains(&format!("id=\"{SEED_NOTE_ID}")) || text.contains(&format!("id='{SEED_NOTE_ID}"))
}

pub fn read(svg: &[u8]) -> Result<Glyph, String> {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).map_err(|e| format!("not a valid SVG: {e}"))?;
    let hotspot = match tree.node_by_id(HOTSPOT_ID) {
        Some(n) => {
            let b = n.abs_bounding_box();
            ((b.left() + b.right()) / 2.0, (b.top() + b.bottom()) / 2.0)
        }
        None => return Err(format!("no element with id=\"{HOTSPOT_ID}\": mark the cursor's click point with a small shape of that id")),
    };
    let mut parts = Vec::new();
    walk(tree.root(), hotspot, &mut parts)?;
    if parts.is_empty() {
        return Err("nothing to draw besides the hotspot".into());
    }
    Ok(Glyph { parts, hotspot })
}

fn walk(group: &usvg::Group, hotspot: Pt, parts: &mut Vec<Part>) -> Result<(), String> {
    for node in group.children() {
        if node.id() == HOTSPOT_ID || node.id().starts_with(SEED_NOTE_ID) {
            continue;
        }
        let name = if node.id().is_empty() { "an element".to_string() } else { format!("#{}", node.id()) };
        match node {
            usvg::Node::Group(g) => {
                if g.opacity().get() < 1.0 {
                    return Err(format!("{name}: partial opacity isn't supported"));
                }
                walk(g, hotspot, parts)?;
            }
            usvg::Node::Path(p) => path(p, &name, hotspot, parts)?,
            usvg::Node::Image(_) => return Err(format!("{name}: images aren't supported, draw it with shapes")),
            usvg::Node::Text(_) => return Err(format!("{name}: text isn't supported, convert it to outlines")),
        }
    }
    Ok(())
}

fn white(paint: &usvg::Paint, opacity: f32, name: &str) -> Result<bool, String> {
    let usvg::Paint::Color(c) = paint else {
        return Err(format!("{name}: only solid colours are supported (no gradients or patterns)"));
    };
    if opacity < 1.0 {
        return Err(format!("{name}: partial opacity isn't supported"));
    }
    match (c.red, c.green, c.blue) {
        (0..=40, 0..=40, 0..=40) => Ok(false),
        (215..=255, 215..=255, 215..=255) => Ok(true),
        (r, g, b) => Err(format!("{name}: colour #{r:02x}{g:02x}{b:02x}; cursors use black (ink) and white (halo colour) only")),
    }
}

fn path(p: &usvg::Path, name: &str, hotspot: Pt, parts: &mut Vec<Part>) -> Result<(), String> {
    if !p.is_visible() {
        return Ok(());
    }
    let data = p.data().clone().transform(p.abs_transform()).ok_or_else(|| format!("{name}: its transform collapses it"))?;
    let subpaths = flatten(&data, hotspot);
    if let Some(fill) = p.fill() {
        let white = white(fill.paint(), fill.opacity().get(), name)?;
        for (pts, _) in &subpaths {
            let points = geom::simplify(pts);
            if points.len() < 3 {
                continue;
            }
            let tris = if geom::is_convex(&points) { vec![] } else { geom::triangulate(&points).map_err(|e| format!("{name}: {e}"))? };
            parts.push(Part::Fill { points, tris, white });
        }
    }
    if let Some(stroke) = p.stroke() {
        let white = white(stroke.paint(), stroke.opacity().get(), name)?;
        if stroke.dasharray().is_some() {
            return Err(format!("{name}: dashed strokes aren't supported"));
        }
        let scale = p.abs_transform().get_scale();
        let width = stroke.width().get() * (scale.0 + scale.1) / 2.0;
        for (pts, closed) in subpaths {
            let points = if closed { geom::simplify(&pts) } else { pts };
            if points.len() >= 2 {
                parts.push(Part::Stroke { points, closed, width, white });
            }
        }
    }
    Ok(())
}

/// The path's subpaths as point lists relative to the hotspot, each with whether it's closed.
fn flatten(data: &usvg::tiny_skia_path::Path, hotspot: Pt) -> Vec<(Vec<Pt>, bool)> {
    let rel = |p: usvg::tiny_skia_path::Point| (p.x - hotspot.0, p.y - hotspot.1);
    let mut subpaths = Vec::new();
    let mut cur: Vec<Pt> = Vec::new();
    for seg in data.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                if cur.len() > 1 {
                    subpaths.push((std::mem::take(&mut cur), false));
                }
                cur = vec![rel(p)];
            }
            PathSegment::LineTo(p) => cur.push(rel(p)),
            PathSegment::QuadTo(c, p) => {
                let a = cur.last().copied().unwrap_or(rel(c));
                geom::flatten_quad(a, rel(c), rel(p), &mut cur);
            }
            PathSegment::CubicTo(c1, c2, p) => {
                let a = cur.last().copied().unwrap_or(rel(c1));
                geom::flatten_cubic(a, rel(c1), rel(c2), rel(p), &mut cur);
            }
            PathSegment::Close => {
                if cur.len() > 1 {
                    let start = cur[0];
                    subpaths.push((std::mem::take(&mut cur), true));
                    cur = vec![start];
                }
            }
        }
    }
    if cur.len() > 1 {
        subpaths.push((cur, false));
    }
    subpaths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg(body: &str) -> Vec<u8> {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">{body}</svg>"#).into_bytes()
    }

    #[test]
    fn coordinates_are_relative_to_the_hotspot() {
        let g = read(&svg(r##"<circle id="hotspot" cx="4" cy="4" r="0.5"/><polygon points="4,4 8,4 8,8" fill="#000"/>"##)).unwrap();
        assert_eq!(g.hotspot, (4.0, 4.0));
        let [Part::Fill { points, tris, white: false }] = &g.parts[..] else { panic!("{:?}", g.parts) };
        assert!(tris.is_empty(), "a triangle is convex");
        assert!(points.contains(&(0.0, 0.0)) && points.contains(&(4.0, 4.0)));
    }

    #[test]
    fn strokes_keep_width_and_colour_and_transforms_apply() {
        let g = read(&svg(r#"<circle id="hotspot" cx="0" cy="0" r="0.5"/><g transform="translate(10 0)"><line x1="0" y1="0" x2="0" y2="5" stroke="white" stroke-width="1.2"/></g>"#)).unwrap();
        let [Part::Stroke { points, closed: false, width, white: true }] = &g.parts[..] else { panic!("{:?}", g.parts) };
        assert_eq!(points, &vec![(10.0, 0.0), (10.0, 5.0)]);
        assert!((width - 1.2).abs() < 1e-6);
    }

    #[test]
    fn concave_fills_are_triangulated() {
        let g = read(&svg(r#"<circle id="hotspot" cx="0" cy="0" r="0.5"/><polygon points="0,0 0,15 3.8,11.4 6.4,17 8.6,16 6.1,10.6 11,10.6" fill="black"/>"#)).unwrap();
        let [Part::Fill { points, tris, .. }] = &g.parts[..] else { panic!("{:?}", g.parts) };
        assert_eq!(tris.len(), points.len() - 2);
    }

    #[test]
    fn a_seed_note_is_found_but_never_drawn() {
        let noted = svg(r##"<circle id="hotspot" cx="0" cy="0" r="0.5"/><rect width="4" height="4"/><g id="seed-note" fill="#f00"><text x="0" y="1">Seed</text></g>"##);
        assert!(has_seed_note(&noted));
        let g = read(&noted).unwrap();
        assert_eq!(g.parts.len(), 1, "only the rect: {:?}", g.parts);
        assert!(!has_seed_note(&svg(r#"<circle id="hotspot" r="0.5"/>"#)));
    }

    #[test]
    fn unsupported_things_are_named() {
        let no_hotspot = read(&svg(r#"<rect width="4" height="4"/>"#)).unwrap_err();
        assert!(no_hotspot.contains("hotspot"), "{no_hotspot}");
        let red = read(&svg(r##"<circle id="hotspot" cx="0" cy="0" r="0.5"/><rect id="nib" width="4" height="4" fill="#f00"/>"##)).unwrap_err();
        assert!(red.contains("#nib") && red.contains("ff0000"), "{red}");
        let dashed = read(&svg(r#"<circle id="hotspot" cx="0" cy="0" r="0.5"/><line x2="5" stroke="black" stroke-dasharray="1 1"/>"#)).unwrap_err();
        assert!(dashed.contains("dashed"), "{dashed}");
    }

    /// The round-trip check: the SVG redraws VectorCraft's hand-written `pen()` nib in
    /// `cursors.rs`, so reading it must give back that function's numbers.
    #[test]
    fn round_trip_checker_matches_vectorcraft_pen_nib() {
        let g = read(include_bytes!("../testdata/round-trip-checker.svg")).unwrap();
        assert_eq!(g.hotspot, (4.0, 4.0));
        let [Part::Fill { points: nib, tris, white: false }, Part::Fill { points: eye, white: true, .. }, Part::Stroke { points: stem, closed: false, width, white: false }] = &g.parts[..] else {
            panic!("{:?}", g.parts)
        };
        assert_eq!(nib, &vec![(0.0, 0.0), (4.0, 12.0), (8.0, 16.0), (16.0, 8.0), (12.0, 4.0)]);
        assert!(tris.is_empty(), "the nib is convex");
        assert!(eye.len() >= 6, "{eye:?}");
        for &(x, y) in eye {
            let r = ((x - 6.0).powi(2) + (y - 6.0).powi(2)).sqrt();
            assert!((r - 1.4).abs() < 0.01, "eye point ({x}, {y}) is {r} from (6, 6)");
        }
        assert_eq!(stem, &vec![(10.0, 14.0), (14.0, 18.0)]);
        assert!((width - 1.2).abs() < 1e-6);
    }
}
