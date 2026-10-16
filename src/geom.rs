//! Geometry for glyphs: flattening curves into points, the convexity test, and ear-clipping
//! triangulation of the simple polygons egui can't fill on its own.

pub type Pt = (f32, f32);

/// How far a flattened curve may stray from the true curve, in points. Cursors are drawn at
/// 1–2 pixels per point, so a tenth of a point is invisible.
pub const TOLERANCE: f32 = 0.1;

/// Points along a quadratic Bézier from `a` (excluded) to `b` (included).
pub fn flatten_quad(a: Pt, c: Pt, b: Pt, out: &mut Vec<Pt>) {
    let n = segments(&[a, c, b]);
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        out.push((u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0, u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1));
    }
}

/// Points along a cubic Bézier from `a` (excluded) to `b` (included).
pub fn flatten_cubic(a: Pt, c1: Pt, c2: Pt, b: Pt, out: &mut Vec<Pt>) {
    let n = segments(&[a, c1, c2, b]);
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push((w0 * a.0 + w1 * c1.0 + w2 * c2.0 + w3 * b.0, w0 * a.1 + w1 * c1.1 + w2 * c2.1 + w3 * b.1));
    }
}

/// Enough straight segments that the curve stays within [`TOLERANCE`]: from the control
/// polygon's length (an upper bound of the curve's), between 2 and 32.
fn segments(ctrl: &[Pt]) -> usize {
    let len: f32 = ctrl.windows(2).map(|w| dist(w[0], w[1])).sum();
    ((len / (8.0 * TOLERANCE).sqrt() / 2.0).ceil() as usize).clamp(2, 32)
}

pub fn dist(a: Pt, b: Pt) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Twice the signed area: positive when the points turn clockwise on screen (y down).
pub fn signed_area2(pts: &[Pt]) -> f32 {
    let n = pts.len();
    (0..n).map(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        a.0 * b.1 - b.0 * a.1
    })
    .sum()
}

fn cross(o: Pt, a: Pt, b: Pt) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Drops repeated points (the closing point of a closed path included) and points on a straight
/// line between their neighbours.
pub fn simplify(pts: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for &p in pts {
        if out.last().is_none_or(|&q| dist(p, q) > 1e-3) {
            out.push(p);
        }
    }
    while out.len() > 1 && out.first().zip(out.last()).is_some_and(|(a, b)| dist(*a, *b) <= 1e-3) {
        out.pop();
    }
    let mut changed = true;
    while changed && out.len() > 3 {
        changed = false;
        let n = out.len();
        for i in 0..n {
            let (a, b, c) = (out[(i + n - 1) % n], out[i], out[(i + 1) % n]);
            if cross(a, b, c).abs() < 1e-4 {
                out.remove(i);
                changed = true;
                break;
            }
        }
    }
    out
}

/// Is the closed polygon convex (every turn the same way)?
pub fn is_convex(pts: &[Pt]) -> bool {
    let n = pts.len();
    if n < 4 {
        return true;
    }
    let mut sign = 0.0f32;
    for i in 0..n {
        let c = cross(pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        if c.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = c.signum();
        } else if c.signum() != sign {
            return false;
        }
    }
    true
}

/// Triangles covering a simple polygon (no holes, no self-intersections), by ear clipping.
pub fn triangulate(pts: &[Pt]) -> Result<Vec<[u16; 3]>, String> {
    if pts.len() > u16::MAX as usize {
        return Err(format!("{} points is too many for one shape", pts.len()));
    }
    let ccw = signed_area2(pts) < 0.0;
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    if ccw {
        idx.reverse();
    }
    let mut tris = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 {
        guard += 1;
        if guard > 10_000 {
            return Err("shape could not be triangulated: does it cross itself?".into());
        }
        let n = idx.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (pts[idx[(i + n - 1) % n]], pts[idx[i]], pts[idx[(i + 1) % n]]);
            if cross(a, b, c) <= 0.0 {
                return false; // a reflex corner can't be an ear
            }
            !idx.iter().any(|&j| {
                let p = pts[j];
                p != a && p != b && p != c && cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
            })
        });
        let Some(i) = ear else {
            return Err("shape could not be triangulated: does it cross itself?".into());
        };
        tris.push([idx[(i + n - 1) % n] as u16, idx[i] as u16, idx[(i + 1) % n] as u16]);
        idx.remove(i);
    }
    if let [a, b, c] = idx[..] {
        tris.push([a as u16, b as u16, c as u16]);
    }
    Ok(tris)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_of(pts: &[Pt], tris: &[[u16; 3]]) -> f32 {
        tris.iter().map(|t| signed_area2(&[pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]]).abs() / 2.0).sum()
    }

    #[test]
    fn convexity() {
        assert!(is_convex(&[(0.0, 0.0), (4.0, 12.0), (8.0, 16.0), (16.0, 8.0), (12.0, 4.0)]));
        // An arrow pointer: concave.
        assert!(!is_convex(&[(0.0, 0.0), (0.0, 15.0), (3.8, 11.4), (6.4, 17.0), (8.6, 16.0), (6.1, 10.6), (11.0, 10.6)]));
    }

    #[test]
    fn concave_shapes_triangulate_to_their_area_either_winding() {
        let arrow = [(0.0, 0.0), (0.0, 15.0), (3.8, 11.4), (6.4, 17.0), (8.6, 16.0), (6.1, 10.6), (11.0, 10.6)];
        let want = signed_area2(&arrow).abs() / 2.0;
        let tris = triangulate(&arrow).unwrap();
        assert_eq!(tris.len(), arrow.len() - 2);
        assert!((area_of(&arrow, &tris) - want).abs() < 1e-3);
        let rev: Vec<Pt> = arrow.iter().rev().copied().collect();
        let tris = triangulate(&rev).unwrap();
        assert!((area_of(&rev, &tris) - want).abs() < 1e-3);
    }

    #[test]
    fn simplify_drops_duplicates_closing_points_and_collinear_points() {
        let pts = [(0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (10.0, 10.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)];
        assert_eq!(simplify(&pts), vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    }

    #[test]
    fn curves_flatten_within_bounds() {
        let mut out = vec![];
        flatten_cubic((0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0), &mut out);
        assert!(out.len() >= 2 && out.len() <= 32);
        assert_eq!(*out.last().unwrap(), (10.0, 0.0));
    }
}
