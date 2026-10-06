# How to talk to Claude about a part

## Sizes

- Write in **millimetres**. Another unit is possible too, but name it: "in inches".
- Ask for sizes that may change to be **parameters**: "make the thickness a parameter `t`". Then "set t to 5" rebuilds the whole part: the holes, the roundings, everything that depends on it.

## Where things stand

- A block, a cylinder and the other starting shapes are laid **centred on the origin**, on the XY plane (Z = 0 at the bottom). A plate 60×40 runs from −30 to 30 along X and from −20 to 20 along Y.
  So "a hole at (8, 8)" is **not** 8 mm from the corner but 8 mm from the centre. "8 mm in from the edges" is the safer way to put it. If a hole still lands past the part, Claude gets a `hole-in-air` warning with the part's bounds and puts it right.
- The sides: **top** (+Z), **bottom**, **front** (−Y), **back** (+Y), **left** (−X), **right** (+X). You can say them in your own language — Claude translates.
- Edges are described in words: "the vertical edges", "the edges of the top face", "the edge between the top and the front". Claude turns that into a query. The query is kept in the feature, so after the sizes change the rounding finds "its" edges again.

## Checking — ask for it

Claude does not see your part until it looks. Good things to ask:

| Ask | What happens |
|---|---|
| "Show it from the top" / "show it from the side" | `render` from that side |
| "Measure the distance between the holes" | `measure` |
| "Check that the part is sound" | `inspect`: valid, one body, the smallest radius of a round face (a rounding or the wall of a hole) |
| "What faces does it have?" | `list_faces`: planes, cylinders, their sizes |
| "What is in the document?" | `get_document`: parts, features (red ones with their reason), bodies with volumes |

## Mistakes and going back

- **"Put it back as it was"** — `undo` takes back the last step. Several steps made as one batch (`apply_ops`) are taken back together.
- When an operation fails, its feature turns **red** and the answer names the reason: "the radius is bigger than the edge can take" and so on. Sometimes the geometry kernel's own words are added (`kernel_said`). Claude sees them and usually suggests another value by itself.
- **"Switch the rounding off, let us look without it"** — `suppress_feature`. The feature stays in the history but is not built.
