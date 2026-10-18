# VectorCraft cursors

The cursors VectorCraft needs, from its cursor list (pulled from VectorCraft 0.12.0 (76116cd1, 2026-10-18)). Don't edit this file: nibcraft rewrites it on every `pull` and `check`. Notes for VectorCraft's developers go in DESIGNER_NOTES.md.

Draw each cursor as `svg/<id>.svg` from its description below, never from another app's cursors. Read DRAWING-GUIDE.md (in nibcraft's top folder) first.

Some descriptions end with a confidence from 0 to 10: how sure VectorCraft is of the description, from the research behind it. The higher it is, the more closely to follow the description; the lower, the more the drawing is yours, as long as it keeps the cursor's meaning and hotspot.

78 cursors: 67 drawn, 11 to draw, 0 to fix, 24 changed since last pull.

## To draw

- SelectionArrow
- PenDrawing
- EyedropperType
- Mesh
- BlobBrush
- Eraser
- PathEraser
- Join
- Shaper
- PuppetPin
- PrintTiling

## Changed since last pull

VectorCraft changed what these cursors must say after they were drawn. Check each drawing against its description below and redraw it where it no longer fits. A cursor leaves this list when its drawing changes; if a drawing already fits, delete its line from `.nibcraft/changed`.

- Arrow
- SelectionArrowHollow
- DirectAnchor
- DirectAnchorSelected
- ReshapeSegment
- GroupSelection
- SelectionCopy
- Move
- CornerRadius
- PathBracket
- TypeWidget
- Pen
- PenAdd
- PenDelete
- PenClose
- PenContinue
- PenJoin
- PenConvert
- AnchorPoint
- Curvature
- Crosshair
- Width
- Scissors
- ScissorsNotAllowed

## Selection

### Arrow · changed since last pull

Shown when:
- Any tool with no cursor of its own (Touch Type, Rotate View).
- The resting state of Live Paint Selection, Reshape (away from a selected path), Free Transform, Puppet Warp and Crop Image (nothing selected or outside the box), the Perspective Grid (locked or away from handles) and Perspective Selection.
- Outside the canvas (menus, panels, the toolbar) the operating system's own pointer shows instead.

Hotspot: The arrow's tip.

Look: The plain pointer: a solid arrow with a tail, like the operating system's pointer. AddStop, RemoveStop and SliceSelect build on it.

### SelectionArrow · to draw

Shown when:
- The Selection tool on the canvas: over empty canvas or over an unselected object (no Alt).
- Dragging with the Selection tool (a move), or with Direct Selection (anchors, a straight segment, a handle or a whole path), until the drag snaps.
- The Pen while dragging out handles, from a new anchor, the first anchor (closing the path) or the last one.

Hotspot: The arrowhead's tip.

Look: A solid arrowhead with no tail: the selection tools' pointer, unlike Arrow. SelectionArrowHollow is its hollow twin, and PathBracket and TypeWidget build on it. While dragging it says "a move is under way". No tail.

Confidence: 6/10 (keep the meaning; the shape is yours)

### SelectionArrowHollow · changed since last pull

Shown when:
- Direct Selection at rest where no other cursor applies: empty canvas, fills, handles, straight segments and frame edges.
- Direct Selection with Shift held.
- While a Selection or Direct Selection drag snaps to a point or a guide (SelectionArrow turns hollow).
- The Selection tool over an anchor when Snap to Point is on.
- Selection and Direct Selection over a pie widget of an ellipse, and while dragging one.

Hotspot: The arrow's tip.

Look: SelectionArrow's hollow twin, a hollow arrowhead with no tail: white with a dark outline, its tip where SelectionArrow's is. DirectAnchor, DirectAnchorSelected, GroupSelection and CornerRadius build on it. No tail.

Confidence: 6/10 (keep the meaning; the shape is yours)

### DirectAnchor · changed since last pull

Shown when:
- Direct Selection over an anchor of a path that isn't selected.

Hotspot: The arrow's tip.

Look: SelectionArrowHollow with a small hollow square badge (an unselected anchor). No tail.

Confidence: 7/10 (follow its meaning and shape; the details are yours)

### DirectAnchorSelected · changed since last pull

Shown when:
- Direct Selection over an anchor of a selected path, whether or not that anchor is selected.

Hotspot: The arrow's tip.

Look: SelectionArrowHollow with a small filled square badge (the anchor's path is selected). DirectAnchor is the same with the square hollow. No tail.

Confidence: 7/10 (follow its meaning and shape; the details are yours)

### ReshapeSegment · changed since last pull

Shown when:
- Direct Selection over a curved segment, unless its path is selected whole (then a drag moves the path).
- Direct Selection while it drags a curved segment.
- Direct Selection over a curved segment of a selected blend's spine.
- The Anchor Point tool over a segment, straight or curved, away from anchors and handles.
- The Anchor Point tool while it reshapes a segment.
- The Pen with Alt held over a segment of a selected path, and while it reshapes one.
- Never over a straight segment with Direct Selection: a drag there moves the segment instead of bending it.

Hotspot: The arrowhead's tip.

Look: Says "a drag bends this segment": a solid arrowhead with a small curve between two anchor marks. No tail.

Confidence: 7/10 (follow its meaning and shape; the details are yours)

### GroupSelection · changed since last pull

Shown when:
- The Group Selection tool: each click selects the next enclosing group.
- Direct Selection with Alt held (Alt makes it Group Selection for the next click).
- The Group Selection tool while dragging, an Alt-drag copy included.

Hotspot: The arrow's tip.

Look: SelectionArrowHollow with a plus badge. No tail.

Confidence: 6/10 (keep the meaning; the shape is yours)

### SelectionCopy · changed since last pull

Shown when:
- Alt-dragging a copy with the Selection tool (the drag leaves the original behind).
- Alt-dragging a copy with Direct Selection: an Alt-press anywhere on a path takes the whole path, and the drag copies it.
- Pressing Alt during a Selection or Direct Selection move turns it into a copy and shows this.

Hotspot: The front arrowhead's tip.

Look: Says "a copy": two arrowheads, one solid and one hollow, one in front of the other. No tail.

Confidence: 6/10 (keep the meaning; the shape is yours)

### Move · changed since last pull

Shown when:
- The Selection tool over a selected object, or over any object with Alt held (once an Alt-drag starts, SelectionCopy shows).
- Reshape over a selected path or while dragging.
- The Curvature tool over a point (a drag moves it).
- Inside the Free Transform and Crop Image boxes.
- Puppet Warp over a pin.
- The Artboard tool inside an artboard.
- The Gradient tool over the annotator's handles and during most gradient drags.
- The Perspective Grid and Perspective Selection over plane widgets and handles.

Hotspot: The centre.

Look: A four-way move cursor.

### CornerRadius · changed since last pull

Shown when:
- Selection and Direct Selection (not Group Selection) over a Live Corners widget, and while dragging one (rounding the corners).

Hotspot: The arrow's tip.

Look: SelectionArrowHollow with a rounded-corner badge. No tail.

Confidence: 5/10 (keep the meaning; the shape is yours)

### PathBracket · changed since last pull

Shown when:
- Selection and Direct Selection over a bracket of selected type on a path, or dragging one (the type slides along its path).

Hotspot: The arrow's tip.

Look: SelectionArrow with a bracket badge. No tail.

### TypeWidget · changed since last pull

Shown when:
- The Selection tool over the type widget of selected type (a double-click converts point type to area type and back).

Hotspot: The arrow's tip.

Look: SelectionArrow with a type badge (a letter T). No tail.

## Pen

### Pen · changed since last pull

Shown when:
- The Pen tool when no path is being drawn, where a click starts a new path.
- That includes over a selected path with Shift held or Auto Add/Delete off, and with Alt held away from selected paths.
- With Use Precise Cursors on, the crosshair shows instead.
- With Cmd/Ctrl held, the Pen lends a selection tool, and that tool's own cursors show instead.

Hotspot: The nib's tip.

Look: A pen nib with a small asterisk badge (a new path starts here). Every Pen-family cursor is this nib with a different badge; PenDrawing is the bare nib.

Confidence: 8/10 (follow its meaning and shape; the details are yours)

### PenDrawing · to draw

Shown when:
- The Pen tool while a path is being drawn, where a click adds the path's next point.
- The close, join and convert badges still show over the points they act on.

Hotspot: The nib's tip.

Look: The bare Pen nib: the Pen without its asterisk, so starting a path visibly changes the cursor.

Confidence: 8/10 (follow its meaning and shape; the details are yours)

### PenAdd · changed since last pull

Shown when:
- The Pen, when no path is being drawn, over a segment of a selected path with Auto Add/Delete on and Shift up.
- The Pen, when no path is being drawn, over a selected blend's spine.
- The Add Anchor Point tool everywhere (and the Delete Anchor Point tool with Alt).
- The Curvature tool over a segment of a selected path.
- Stands in for PuppetPin until it's drawn.

Hotspot: The nib's tip.

Look: The Pen nib with a plus badge.

Confidence: 9/10 (follow the description closely)

### PenDelete · changed since last pull

Shown when:
- The Pen, when no path is being drawn, over an anchor of a selected path with Auto Add/Delete on and Shift up.
- The Pen, when no path is being drawn, over a point of a selected blend's spine.
- The Delete Anchor Point tool everywhere (and the Add Anchor Point tool with Alt).
- The Mesh tool with Alt over a mesh point.

Hotspot: The nib's tip.

Look: The Pen nib with a minus badge.

Confidence: 9/10 (follow the description closely)

### PenClose · changed since last pull

Shown when:
- The Pen while drawing, over the first anchor once the path has two or more (a click closes it).
- The Curvature tool over the first point of the path being drawn, once it has three or more, without Alt.
- The Curvature tool near the other end of an open path whose end point is current.

Hotspot: The nib's tip.

Look: The Pen nib with a ring badge (closing the loop).

Confidence: 9/10 (follow the description closely)

### PenContinue · changed since last pull

Shown when:
- The Pen, when no path is being drawn, over an end of an open path (a click continues it).
- The Curvature tool over an open end of a selected path, without Alt.

Hotspot: The nib's tip.

Look: The Pen nib with a slash badge.

Confidence: 7/10 (follow its meaning and shape; the details are yours)

### PenJoin · changed since last pull

Shown when:
- The Pen while drawing, over an end of another open path (a click joins the two).

Hotspot: The nib's tip.

Look: The Pen nib with a badge showing a line running into an end point.

Confidence: 6/10 (keep the meaning; the shape is yours)

### PenConvert · changed since last pull

Shown when:
- The Pen while drawing, over the last anchor without Alt (a click retracts its outgoing handle).
- With Alt held the Pen shows AnchorPoint over that anchor instead.
- The Curvature tool with Alt held over a point (a click toggles it between corner and smooth).

Hotspot: The nib's tip.

Look: The Pen nib with a caret badge. It is the Pen's own badge, not the AnchorPoint cursor.

Confidence: 7/10 (follow its meaning and shape; the details are yours)

### AnchorPoint · changed since last pull

Shown when:
- The Anchor Point tool over anchors, handle ends and empty canvas (ReshapeSegment shows over a segment).
- The Anchor Point tool while it converts an anchor or drags a handle.
- The Pen with Alt held, over an anchor or a handle end of a selected path, the last anchor of the path being drawn included.

Hotspot: The caret's apex.

Look: A caret (an upside-down V) like the Anchor Point tool's own icon, with the hotspot clearly marked at its apex.

Confidence: 7/10 (follow its meaning and shape; the details are yours)

### Curvature · changed since last pull

Shown when:
- The Curvature tool over empty canvas or unselected art, where a click starts a new path.
- The Curvature tool while drawing a path, away from its points (a click adds the next point).
- Elsewhere it borrows: PenAdd over a segment of a selected path, Move over a point, PenContinue at an open end, PenConvert with Alt over a point, PenClose over the first point.

Hotspot: The nib's tip.

Look: The Pen nib with a curve badge.

Confidence: 3/10 (free to design, keeping the meaning, the hotspot and the family it belongs to)

## Type

### Text · drawn

Shown when:
- Every horizontal type tool: Type, Area Type and Type on a Path (the Area and Path ones show the crosshair over a suitable path).
- Stands in for TypeVertical until it's drawn.

Hotspot: The I-beam's centre.

Look: An I-beam with a short crossbar marking the baseline.

### TypeVertical · drawn

Shown when:
- The vertical type tools: Vertical Type, Vertical Area Type and Vertical Type on a Path (the Area and Path ones show the crosshair over a suitable path).

Hotspot: The I-beam's centre.

Look: The Text I-beam turned a quarter turn.

## Transform and resize

### Crosshair · changed since last pull

Shown when:
- The generic drawing cursor: all shape, line, grid, flare, cut, graph and liquify tools, and Place.
- The Rotate, Reflect, Scale and Shear tools with a selection (Reflect, Scale and Shear even mid-drag).
- The Area Type and Type on a Path tools over a suitable path.
- The Artboard tool outside every artboard; the Gradient tool away from the annotator; Free Transform corners in Distort or Perspective mode.
- With Use Precise Cursors on, it replaces the Pen-family, Anchor Point, Curvature, Scissors, Eyedropper, Slice, Blend and Shape Builder cursors (not the selection arrows).
- Stands in for every drawing tool cursor not drawn yet.

Hotspot: The centre of the open gap.

Look: Four arms around an open centre, no dot, so the art under the hotspot stays visible.

### ResizeH · drawn

Shown when:
- The left and right handles of the bounding box (or whichever handle reads as left/right once the box is rotated) with Selection, Free Transform, Artboard, Slice Selection and Crop Image.
- Over or dragging a vertical ruler guide.
- Perspective Grid extent handles and near-horizontal Perspective Selection handles.
- The freeform gradient's spread ring.

Hotspot: The centre.

Look: A horizontal double arrow. ResizeV, ResizeNwSe and ResizeNeSw are the same arrow turned.

### ResizeV · drawn

Shown when:
- The top and bottom handles of the same boxes as ResizeH.
- Over or dragging a horizontal ruler guide.
- The Perspective Grid's horizon, height and cell-size handles, and near-vertical Perspective Selection handles.

Hotspot: The centre.

Look: The ResizeH arrow, vertical.

### ResizeNwSe · drawn

Shown when:
- The top-left and bottom-right corner handles of the same boxes as ResizeH (rotation-adjusted).
- Perspective Selection handles at 112.5° to 157.5°.

Hotspot: The centre.

Look: The ResizeH arrow, diagonal from top-left to bottom-right.

### ResizeNeSw · drawn

Shown when:
- The top-right and bottom-left corner handles of the same boxes as ResizeH (rotation-adjusted).
- Perspective Selection handles at 22.5° to 67.5°.

Hotspot: The centre.

Look: The ResizeH arrow, diagonal from bottom-left to top-right.

### ShearH · drawn

Shown when:
- Free Transform with Cmd/Ctrl held over the top or bottom handle of the bounding box, and while that drag shears the object sideways.
- Without Cmd/Ctrl the same handles show ResizeV.

Hotspot: The centre, between the two arrows.

Look: Two single arrows side by side, one above the other, pointing opposite ways (left and right): one side sliding past the other.

### ShearV · drawn

Shown when:
- Free Transform with Cmd/Ctrl held over the left or right handle of the bounding box, and while that drag shears the object up or down.
- Without Cmd/Ctrl the same handles show ResizeH.

Hotspot: The centre, between the two arrows.

Look: The ShearH arrows turned upright.

### Rotate · drawn

Shown when:
- The Rotate tool while dragging.
- The Gradient tool over its rotate zone, or rotating.
- Puppet Warp with Alt near a selected pin.
- The bounding-box rotate cursor when the pointer is exactly on the box's centre.

Hotspot: Below the arc, in its empty middle.

Look: An arc with an arrowhead at each end. RotateToward is the same arc, turned.

### RotateToward · drawn

Shown when:
- Selection and Free Transform in the rotate zone just outside a bounding-box corner, and while rotating there. The Rotate, Gradient and Puppet Warp rotate cursors don't turn.

Hotspot: Below the arc, in its empty middle, as for Rotate.

Look: The Rotate arc drawn for the top-right corner (direction 1). VectorCraft turns it in 45° steps so it wraps whichever corner the pointer is at.

## Navigation

### Hand · drawn

Shown when:
- The Hand tool, or Space held with any tool (not with Cmd/Ctrl): ready to pan.

Hotspot: The middle of the hand.

Look: An open hand. HandGrab is the same hand closed.

### HandGrab · drawn

Shown when:
- While panning: a Hand tool drag, a Space drag, or a middle-button drag with any tool.

Hotspot: The middle of the hand, as for Hand.

Look: The Hand, closed (grabbing).

### ZoomIn · drawn

Shown when:
- The Zoom tool, or Cmd/Ctrl+Space with any tool: a click zooms in.

Hotspot: The lens centre.

Look: A magnifier with a plus in its lens.

### ZoomOut · drawn

Shown when:
- The Zoom tool, or Cmd/Ctrl+Space, with Alt held: a click zooms out.

Hotspot: The lens centre.

Look: The ZoomIn magnifier with a minus in its lens.

## Colour and gradient

### Eyedropper · drawn

Shown when:
- The Eyedropper tool away from type, whatever the modifier keys.
- Over type the Eyedropper shows EyedropperType.
- With Use Precise Cursors on, the crosshair shows instead.

Hotspot: The pipette's tip.

Look: A pipette.

### EyedropperType · to draw

Shown when:
- The Eyedropper tool over type, where a click samples the type's attributes.
- With Use Precise Cursors on, the crosshair shows instead.

Hotspot: The pipette's tip.

Look: The Eyedropper pipette with a small T.

### AddStop · drawn

Shown when:
- The Gradient tool over the annotator's bar (a click adds a stop).
- The freeform gradient over its line or the art.

Hotspot: The arrow's tip.

Look: Arrow with a plus badge. RemoveStop is the same with a minus.

### RemoveStop · drawn

Shown when:
- The Gradient tool while a stop is dragged off the bar (releasing deletes it).
- The freeform gradient while a point is dragged out.

Hotspot: The arrow's tip.

Look: Arrow with a minus badge.

### MeshAdd · drawn

Shown when:
- The Mesh tool over a mesh or a path, off a mesh point: a click adds a mesh point (turning a path into a mesh).
- Elsewhere the Mesh tool shows Mesh, or PenDelete with Alt over a mesh point.

Hotspot: Where the mesh point goes.

### Mesh · to draw

Shown when:
- The Mesh tool over a mesh point or handle, where a drag moves it.
- The Mesh tool away from any mesh or path, where a click adds nothing.
- With Use Precise Cursors on, the crosshair shows instead.

Hotspot: The same point as MeshAdd's.

Look: MeshAdd without its plus.

### LivePaintBucket · drawn

Shown when:
- The Live Paint Bucket: a click fills a face (or an edge) of a Live Paint group.

Hotspot: Where the paint pours out.

## Width, blend, slice and shape builder

### Width · changed since last pull

Shown when:
- The Width tool away from strokes.

Hotspot: The arrow's tip.

Look: SelectionArrowHollow with a lens badge (a stroke that swells in the middle). WidthAdd and WidthPoint build on it. No tail.

### WidthAdd · drawn

Shown when:
- The Width tool over a stroke, away from its width points (a drag adds one).

Hotspot: The arrow's tip.

Look: The Width cursor with a plus.

### WidthPoint · drawn

Shown when:
- The Width tool over a width point or a handle end, and during any Width drag.

Hotspot: The arrow's tip.

Look: The Width cursor with a mark across the swell (the width being dragged).

### Blend · drawn

Shown when:
- The Blend tool away from art. With Use Precise Cursors on, the crosshair shows instead.

Hotspot: The crosshair's centre.

Look: Crosshair with a hollow square badge. BlendObject and BlendAnchor change the badge.

### BlendObject · drawn

Shown when:
- The Blend tool over an object it can blend (away from its anchors).

Hotspot: The crosshair's centre.

Look: Crosshair with a filled square badge.

### BlendAnchor · drawn

Shown when:
- The Blend tool over an anchor point (the blend starts there).

Hotspot: The crosshair's centre.

Look: Crosshair with a target badge (a ring with a centre dot).

### Slice · drawn

Shown when:
- The Slice tool, always. With Use Precise Cursors on, the plain crosshair shows instead.

Hotspot: The crosshair's centre.

Look: Crosshair with a blade badge.

### SliceSelect · drawn

Shown when:
- The Slice Selection tool away from the selected slice's handles.

Hotspot: The arrow's tip.

Look: Arrow with a small slice badge (a rectangle cut by a line).

### ShapeBuilder · drawn

Shown when:
- The Shape Builder tool (merge mode), and during a drag that started without Alt.

Hotspot: The crosshair's centre.

Look: Crosshair with a plus badge. ShapeBuilderErase is the same with a minus.

### ShapeBuilderErase · drawn

Shown when:
- The Shape Builder with Alt held (erase mode), and during a drag that started with Alt.

Hotspot: The crosshair's centre.

Look: Crosshair with a minus badge.

## Drawing and editing tools

### Pencil · drawn

Shown when:
- The Pencil tool: draws a freeform path; starting near the end of a selected open path continues it; Alt while drawing closes the path.

Hotspot: The lead tip.

### Paintbrush · drawn

Shown when:
- The Paintbrush tool: draws a brushed freeform path (with pen pressure), continuing a selected path from its end; Alt closes it.

Hotspot: The bristle tip.

### BlobBrush · to draw

Shown when:
- The Blob Brush tool: paints filled shapes that merge with touching ones of the same colour. No brush-size outline shows while hovering.

Hotspot: The brush centre.

### Eraser · to draw

Shown when:
- The Eraser tool: a drag erases the area it covers. No size outline shows while hovering.

Hotspot: The eraser's centre.

### Smooth · drawn

Shown when:
- The Smooth tool: a drag along a selected path smooths it.

Hotspot: The tip.

### PathEraser · to draw

Shown when:
- The Path Eraser tool: a drag along a selected path erases that part of it.

Hotspot: The tip.

### Join · to draw

Shown when:
- The Join tool: scrubbing across the gap or overlap between open path ends joins them.

Hotspot: The tip.

### Knife · drawn

Shown when:
- The Knife tool: a freehand drag cuts objects along the line.

Hotspot: The blade's point.

### Scissors · changed since last pull

Shown when:
- The Scissors tool over a path segment: a click splits the path there.

Hotspot: Where the blades cross, exact enough to cut where aimed.

Look: Scissors. ScissorsNotAllowed is this drawing with a badge.

Confidence: 2/10 (free to design, keeping the meaning, the hotspot and the family it belongs to)

### ScissorsNotAllowed · changed since last pull

Shown when:
- The Scissors tool away from every path segment: a click does nothing.

Hotspot: The same point as Scissors.

Look: The Scissors with a small not-allowed badge, so the cursor keeps its size and shape and only the badge says a click won't cut.

Confidence: 2/10 (free to design, keeping the meaning, the hotspot and the family it belongs to)

### Shaper · to draw

Shown when:
- The Shaper tool: a rough drawn shape becomes a clean live shape (rectangle, ellipse, polygon, line); scribbling over art deletes it.

Hotspot: The tip.

### Lasso · drawn

Shown when:
- The Lasso tool: drag a loop round anchor points and segments to select them; Shift adds to the selection, Alt takes from it.

Hotspot: Where the loop is drawn from.

### MagicWand · drawn

Shown when:
- The Magic Wand tool: a click selects objects like the one clicked (per the Magic Wand options); Shift adds, Alt subtracts.

Hotspot: The wand's tip.

### PuppetPin · to draw

Shown when:
- Puppet Warp over the selection's mesh where a pin can go: a click adds a pin.
- Elsewhere Puppet Warp shows NotAllowed (off the mesh), Move (over a pin) and Rotate (Alt near a selected pin).

Hotspot: Where the pin goes.

### Measure · drawn

Shown when:
- The Measure tool: drag to measure distance and angle.

Hotspot: Exact, because precision matters here (an open centre suits it).

### PrintTiling · to draw

Shown when:
- The Print Tiling tool: a drag moves the printable page area over the artboard (it snaps to the artboard's edges).

Hotspot: The page corner being placed.

## Symbolism tools

### SymbolSprayer · drawn

Shown when:
- The Symbol Sprayer: a drag sprays instances of the current symbol; Alt reverses it.

Hotspot: The spray centre.

### SymbolShifter · drawn

Shown when:
- The Symbol Shifter: a drag pushes symbol instances along; Alt reverses it.

Hotspot: The brush centre.

### SymbolScruncher · drawn

Shown when:
- The Symbol Scruncher: a drag pulls instances together; Alt reverses it.

Hotspot: The brush centre.

### SymbolSizer · drawn

Shown when:
- The Symbol Sizer: a drag enlarges instances; Alt reverses it.

Hotspot: The brush centre.

### SymbolSpinner · drawn

Shown when:
- The Symbol Spinner: a drag rotates instances in its direction; Alt reverses it.

Hotspot: The brush centre.

### SymbolStainer · drawn

Shown when:
- The Symbol Stainer: a drag tints instances toward the fill colour; Alt reverses it.

Hotspot: The brush centre.

### SymbolScreener · drawn

Shown when:
- The Symbol Screener: a drag makes instances more transparent; Alt reverses it.

Hotspot: The brush centre.

### SymbolStyler · drawn

Shown when:
- The Symbol Styler: a drag applies the chosen graphic style to instances; Alt reverses it.

Hotspot: The brush centre.

## Not needed

These tools have no cursor of their own on purpose: don't draw one.

- **Liquify tools (Warp, Twirl, Pucker, Bloat, Scallop, Crystallize, Wrinkle)**: They show the Crosshair, and the app draws the brush's real size and tilt as an outline; a fixed-size cursor would disagree with it.
- **Shape, line, grid, flare, graph, cut and gradient tools**: The generic Crosshair is the intended drawing cursor for them.
