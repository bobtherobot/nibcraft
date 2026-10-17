# Drawing guide

How to draw cursor SVGs that render the way they look in your editor. VectorCraft doesn't show
your SVG as-is: nibcraft converts it to shapes, and VectorCraft draws those with a few rules of
its own. Draw with those rules in mind and what you see is what you get.

## How a cursor is drawn

1. **White halo first.** Every black part gets a white halo underneath, so the cursor shows on
   any background. All halos are drawn before any black, so a halo never cuts into other ink.
2. **Then every part, in the order of your SVG**, top of the file first.
3. **Black fills get a 1-point black outline**, centred on their edge. A fill grows by half a
   point on every side.
4. **Strokes are drawn at exactly their width**, with flat ends cut off at the end points.
5. **White parts get no halo and no outline.** They're drawn at exactly the size you drew them.

## Setup

- **Canvas:** `viewBox="0 0 32 32"`. Units are points; 1 point is 1 pixel on a normal screen and
  2 on a Retina one. Existing cursors span about 20 points.
- **Hotspot:** one small shape with `id="hotspot"` whose centre is the click point. Any colour;
  it isn't drawn. Put its centre on a **whole point**, e.g. (4, 4).
- **File name:** `svg/<id>.svg`, with the cursor's id exactly as `CURSORS.md` lists it, e.g.
  `svg/PenAdd.svg`. Any other name is ignored by the app (handy for variants); names starting with
  `_` are skipped entirely, so use them for scratch drawings.
- **What to draw:** the cursor's description in `CURSORS.md`: when it shows, where its hotspot
  goes, and what it must match (a badge, a sibling cursor). Draw from that, never from another
  app's cursors.
- **Colours:** black (ink) and white only. Near-black and near-white are accepted.

## Quick reference

| You draw | It renders |
| --- | --- |
| Black fill | half a point bigger on every side; white halo reaching 1.5 points past the drawn edge |
| Black stroke, width *w* | exactly *w* wide, white halo 0.9 points each side, **no halo at the ends** |
| Black stroke thinner than 1 point | 1 pixel wide but grey (faded), not thinner |
| White fill or stroke | exactly as drawn, on top of what's above it in the file |
| Two black fills less than 1 point apart | merged: the gap closes |

## Fills

- **Do** draw fills half a point smaller on every side than you want them to look (1 point
  narrower and 1 point shorter overall).
- **Do** use fills for solid bodies (nibs, arrows, hands) and for small parts that need white
  around their ends, such as a "+" badge. A "+" drawn with 0.5-point arms renders with 1.5-point
  arms.
- **Do** put straight fill edges on **half points** (e.g. x = 10.5). The outline then lands on a
  whole pixel and stays sharp.
- **Don't** expect a fill to look thinner than just over 1 point: the outline alone adds 1 point.
- **Don't** leave less than 1 point between two black fills if you want white between them.

## Strokes

- **Do** use strokes for exact thin lines, 1 point or wider.
- **Do** centre 1-point strokes on **half points** (e.g. y = 10.5), and 2-point strokes on whole
  points, so they cover whole pixels.
- **Do** end open strokes inside or against another black part, where the bare ends don't show.
- **Don't** use strokes under 1 point. egui fades a line thinner than a pixel instead of thinning
  it, so a 0.5-point line shows as a 1-pixel grey line.
- **Don't** rely on stroke ends or corners: `stroke-linecap` and `stroke-linejoin` are ignored.
  Draw the part as a fill if its ends need a white halo.
- **Don't** use dashed strokes; they're rejected. For a dotted line, draw each dot as a fill
  1 point smaller than it should look, centred where it should be.

## White parts

- **Do** use white shapes on top of black to cut holes, eyes and gaps (like the pen's eye). They
  must come **after** the black in the SVG.
- **Do** draw halftones and checkers as one black square with white cells on top: white parts
  get no outline, so the cells keep their size.
- **Do** draw hollow shapes (like a hollow arrow) as a white fill with a black stroke: white fills
  get no outline, so the stroke is their ink.
- **Don't** draw your own halos: every black part already has one.

## Matching the render while you draw

- **Do** give black fills a 1-point black stroke, **centred**, in your editor. It renders the same
  as the outline VectorCraft adds anyway, so your drawing shows the real weight.
- **Don't** align that stroke inside or outside the shape: only a centred stroke matches.
- **Don't** add that stroke to white fills: on a white fill a black stroke is real ink, not a
  preview of the outline.

## Shapes and files

- **Do** use `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`, groups and
  transforms. Curves are turned into short straight lines (within 0.1 point).
- **Don't** let a shape cross itself, and don't use holes (a shape cut out of another): draw the
  hole as a white shape on top.
- **Don't** use gradients, patterns, text (convert it to outlines), images, or partial opacity.
  `check` names the element that breaks a rule.
- **Don't** trace, redraw or copy any Adobe cursor, icon or screenshot, and don't use Adobe
  software to make or check cursors. Every cursor must be original work (see the README's
  Clean-room section).

## Checking

- **Do** run `cargo run -q -- check` often. It opens the preview, which draws every cursor the way
  VectorCraft does, enlarged and at actual size. Switch the background to grey or dark to see the
  halos.
- **Don't** rely on the preview's actual-size swatch for pixel sharpness: it isn't on the
  cursor's pixel grid. Follow the half-point rules above instead.
