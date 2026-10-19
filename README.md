# nibcraft

Turns cursor designs drawn as SVG into Rust glyph data for VectorCraft
(`crates/ui-egui/src/cursors/glyphs.rs`). VectorCraft draws its cursors from that data; the SVGs
here are their editable sources.

**VectorCraft is the source of truth: it decides which cursors exist and what each is for, and
what it ships is what `pull` gives you.** VectorCraft keeps its cursor list
(`crates/tools/src/cursors.toml`: every cursor's id, when it shows and where its hotspot is).
nibcraft copies that list in and shows what's drawn and what's left. Nothing a designer writes
changes what a cursor is for.

**VectorCraft knows nothing of nibcraft.** nibcraft is a separate tool: it reads VectorCraft's
checkout (the cursor list, the `Cursor` values in `crates/tools/src/lib.rs`, the generated glyphs)
and writes only the generated part of `glyphs.rs`. The workflow is pull → draw → push → rebuild
VectorCraft.


## Project Structure
Each app gets a folder under the "projects" folder (`projects/vectorcraft/`), and contains the following:

- **svg/ (sub-folder)** - holds your drawings, one svg file for each cursor, named after the cursor's ID (svg/PenAdd.svg). It also holds the **seeds** that `pull` writes (see below), named `<ID>-<app>-rendered.svg` (svg/Crosshair-vectorcraft-rendered.svg).

- **CURSORS.md** - lists every cursor VectorCraft needs and its status (written by nibcraft: don't edit)

- **DESIGNER_NOTES.md** - your notes for VectorCraft's developers

- **PREVIEW.html** - the preview `check` writes (local only, not committed)

- **.nibcraft/ (hidden sub-folder)** - nibcraft's own files. Don't edit or delete them (except a line of `changed`, see Step 2):
   - `cursors.toml` - VectorCraft's cursor list as of your last pull
   - `shipped` - VectorCraft's glyphs as of your last pull or push: how `pull` tells your work from VectorCraft's changes
   - `changed` - the cursors whose description changed after you drew them (see Step 2)
   - `pull-from` - where VectorCraft lives on this computer (local only, not committed)

### Seeds

Some cursors VectorCraft already draws itself, in its own code, before any designer has drawn them. For each of those that has no drawing here yet, `pull` writes a **seed**: an SVG of how VectorCraft draws that cursor today, as a starting point. `CURSORS.md` points to it in its "To draw" list.

**Never draw in a seed file.** Every `pull` overwrites every seed, so anything you change in one is lost — you've been warned! To start from a seed, copy it to the cursor's own name and draw in the copy:

```sh
cp projects/vectorcraft/svg/Crosshair-vectorcraft-rendered.svg projects/vectorcraft/svg/Crosshair.svg
```

Once a cursor is drawn, here or in VectorCraft, the next `pull` deletes its seed.

To make seeds, nibcraft builds VectorCraft's own cursor-drawing code in a scratch copy of your VectorCraft checkout (a git worktree under nibcraft's `target/seeds/`, uncommitted changes to tracked files included), with a small test of nibcraft's own (`runtime/vectorcraft_seeds.rs`) dropped in. Your VectorCraft checkout isn't changed; `git worktree list` there shows the scratch copy.

## Workflow

Run commands from the nibcraft folder (not a project folder).

### Step 1 - Pull

Use this command:

```sh
# first time
cargo run -- pull ../vectorcraft
```

(Leave out `-q` the first time: cargo then shows it building nibcraft, instead of a terminal that looks stuck.)

This sets up and synchronizes NibCraft with VectorCraft on your system:
   - Remembers where VectorCraft lives on your system.
   - Gets VectorCraft's cursor list.
   - Creates the project folder the first time.
   - Makes it the current project you are working on.
   - Writes every cursor VectorCraft ships as a drawing (`svg/<ID>.svg`), from its glyph data.
   - Writes the seeds. To do that it builds VectorCraft, so the first pull takes a few minutes: you'll see VectorCraft's build scroll by. Later pulls are quicker. Add `--no-seeds` to skip this. (Building means running VectorCraft's own code, so only pull from a VectorCraft checkout you trust.)

The "pull" command writes `CURSORS.md`, which lists every cursor and its status, and prints which cursors were added to or dropped from the list since the last pull, and whose descriptions changed. A dropped cursor's drawing stays in `svg/` and is ignored.

**VectorCraft wins.** A drawing that differs from what VectorCraft ships is kept only while VectorCraft's glyph for it hasn't changed since your last pull or push: that's your work in progress. When someone else changes that glyph in VectorCraft, `pull` replaces your drawing with VectorCraft's, and when VectorCraft stops shipping one, `pull` deletes its drawing. So **commit your project as you go** (`git commit projects/vectorcraft`): anything replaced is then still in git.

Drawings written from VectorCraft's data are straight segments only (VectorCraft keeps points, not curves). A drawing of yours that gives exactly VectorCraft's glyph, curves and all, is left as you drew it.

Whenever you update the VectorCraft repo (e.g. to get the latest version), you can just run:

```sh
# afterwards: when VectorCraft is updated
cargo run -q -- pull
```

... since the first time you did this, NibCraft stored the VectorCraft location for you.

### Step 2 - Draw

Cursor descriptions are listed in `CURSORS.md`. Use these descriptions to draw cursors.

Read [DRAWING-GUIDE.md](DRAWING-GUIDE.md) first.

**Confidence.** Some descriptions end with a confidence from 0 to 10, such as `Confidence: 7/10`. It says how sure VectorCraft is of the description, from the research behind it, and so how closely to follow it:
- 9–10: follow the description closely.
- 7–8: follow its meaning and shape; the details are yours.
- 4–6: keep the meaning; the shape is yours.
- 1–3: free to design, keeping the meaning, the hotspot and the family the cursor belongs to.
- 0: a placeholder.

Whatever the number, what a description states plainly (such as "No tail.") is not up for interpretation.

**Changed since last pull.** When VectorCraft changes what a cursor you've already drawn must say, `CURSORS.md` lists it under "Changed since last pull" and marks it "changed since last pull". Check the drawing against the new description and redraw it where it no longer fits. A cursor leaves the list once its drawing changes. If a drawing already fits, delete its line from `.nibcraft/changed` (the one place you may edit there).

**Name each file after its cursor's ID.** Every cursor in `CURSORS.md` has an ID, such as `PenAdd`. Save its drawing in `svg/` as that ID plus `.svg`: `svg/PenAdd.svg`. The name must match the ID exactly, including upper and lower case: `penadd.svg` or `Pen-Add.svg` won't be used.

Files whose names aren't a cursor ID are ignored by VectorCraft, so you can keep variants and experiments beside the real ones (`svg/PenAdd working 1.svg`).

Names starting with `_` are scratch and skipped entirely.

#### Drawing rules

See [DRAWING-GUIDE.md](DRAWING-GUIDE.md) for how VectorCraft draws a cursor and the do's and
don'ts that keep the render looking like your SVG: why fills render bolder, stroke widths and
ends, white parts, hotspot and pixel alignment, and the supported SVG features.

#### Clean-room

These cursors go into an open-source, clean-room project. Every SVG here must be **original work**, designed from the written descriptions in `CURSORS.md` only. Never trace, redraw from, or copy any Adobe cursor, icon or screenshot, and never use Adobe software to make or check them. Write `DESIGNER_NOTES.md` in your own words too: `check` warns when it names other companies' products.

### Step 3 - Check
Check your progress as often as you like:

```sh
cargo run -q -- check
```

It opens a preview (`projects/vectorcraft/PREVIEW.html`) that draws each cursor as VectorCraft will, but includes the hotspot (in red) for reference only so you can quickly scan through your work.

Switch the background to grey or dark to see the white halos.

Drawings VectorCraft doesn't use are shown at the end.

The "To do" column lists **[m] missing** cursors (no drawing yet), **[f] fix** items (the drawing doesn't convert) and **[c] changed** cursors (the description changed after you drew them).

It also refreshes `CURSORS.md`, and notes when VectorCraft's list has changed since your last pull.

A drawing of a listed cursor that doesn't convert is a problem; `check` fails until it's fixed. Nothing else is: a changed cursor doesn't stop a push.

### Step 4 - Push
When you're happy with your work, push it into VectorCraft:

```sh
cargo run -q -- push
```

The `push` command:
- Does a check, and stops if there are problems.
- Stops if VectorCraft has changed since your last pull (its cursor list or its glyphs): pull first (commit before, to keep your drawings), then push.
- Rewrites the generated part of `crates/ui-egui/src/cursors/glyphs.rs` in the VectorCraft checkout you pulled from (or `push PATH/TO/VECTORCRAFT`), and nothing else there. A drawing that gives exactly what VectorCraft ships keeps VectorCraft's code as it is, author included; a new or redrawn one is written with your name (`git config user.name`). A cursor VectorCraft ships but you no longer have a drawing of is unwired: VectorCraft draws it its own way again.
- Commits the project folder (only that folder; it doesn't `git push`).
- Lists what's new, redrawn or unwired, and your `DESIGNER_NOTES.md` to pass on to VectorCraft's developers.

Then rebuild VectorCraft to see your cursors. The change to `glyphs.rs` is an ordinary edit in the VectorCraft checkout: commit it there like any other change (a new cursor's ID needs a `Cursor` value in VectorCraft first; push says so).

### Other Commands

The `projects` command lists the projects (the current one starred)

```
cargo run -q -- projects
```

The `use` command switches projects (for now we only have VectorCraft)

```
cargo run -q -- use NAME
```

Developer tool (designers don't need it):

```sh
# Print one drawing's generated Rust glyph
cargo run -q -- gen projects/vectorcraft/svg/Pen.svg
```


## Development

```sh
cargo test
```

- `src/parse.rs`: reads an SVG with `usvg` into glyph parts.
- `src/geom.rs`: flattening, convexity and ear-clipping triangulation.
- `src/emit.rs`: writes the Rust.
- `src/manifest.rs`: reads an app's cursor list.
- `src/app_glyphs.rs`: VectorCraft's side, read and written from outside: its `Cursor` values, its generated glyphs, and a glyph as an SVG.
- `src/pull.rs`: `pull`, copying the cursor list in, bringing the drawings level with VectorCraft (`.nibcraft/shipped`), and recording the cursors whose description changed (`.nibcraft/changed`).
- `src/seeds.rs`: `pull`'s seeds: builds VectorCraft in a scratch worktree with `runtime/vectorcraft_seeds.rs` dropped in, and writes what it draws as `<id>-<app>-rendered.svg`.
- `src/push.rs`: `push`, rewriting the generated part of VectorCraft's `glyphs.rs`.
- `src/check.rs`: the `check` precheck of a project against its cursor list.
- `src/cursors_md.rs`: writes `CURSORS.md`.
- `src/notes.rs`: `DESIGNER_NOTES.md`'s template and vendor-name warning.
- `src/preview.rs`: the HTML swatch book.
- `src/commit.rs`: `push`'s commit of the project folder.
- `src/project.rs`: projects, their files and the current project (`projects`, `use`).
- `runtime/vectorcraft_seeds.rs`: the test `pull` drops into VectorCraft's `cursors` module to draw the seeds. When VectorCraft's drawing code changes shape, it follows.

`NIBCRAFT_APP=/path/to/vectorcraft cargo test -- --ignored` also checks that every glyph VectorCraft ships reads back the same from the SVG `pull` writes for it.

`testdata/round-trip-checker.svg` is the round-trip check: it redraws `cursors.rs`'s `pen()` nib. Its glyph matches the hand-written one, except that the stem's halo no longer cuts into the nib; `cargo test` checks that it still converts to `pen()`'s numbers.

Licence: MIT OR Apache-2.0, at your option, like VectorCraft (`LICENSE-MIT`, `LICENSE-APACHE`).
