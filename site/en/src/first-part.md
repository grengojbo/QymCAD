# A first part in five minutes

Write to Claude:

> Make a plate 60×40×4 mm. Four countersunk holes for an M3 screw, 8 mm in from the edges. Round the vertical edges with a radius of 5. Show what came out, and save it as `plate.3mf` and `plate.qcad` in the Downloads folder.

What Claude does (it shows in the conversation as tool calls):

1. `box` — a block 60×40×4.
2. `create_sketch` on the top face, `sketch_add` — four points for the holes.
3. `hole` — countersunk holes of 3.4 mm at those points.
4. `fillet` — the four vertical edges rounded.
5. `render` — a picture; `inspect` — the body is sound and in one piece.
6. `export_mesh` → `plate.3mf`, `save_project` → `plate.qcad`.

What you see: a short report with the volume (about 9 300 mm³) and a picture of the part. In Claude Desktop the picture is small at first and opens in full when clicked.

Then:
- open `plate.3mf` in a slicer and print;
- open `plate.qcad` in the QymCAD window: the whole history of the build is there, and any step can be changed.
