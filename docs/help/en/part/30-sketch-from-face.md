# Sketch from face

The outline of a flat face becomes a sketch: lines, arcs and circles, with their constraints and sizes already on.
Replace also redraws the boss or pocket on that face by the sketch, and from then on the sizes of the sketch drive the
body. Made for a body from a STEP file, which has no sketch behind its bosses when one has to be made wider.

## How

1. Press **Sketch from face** in the Part bar (beside Size on body). A face selected already is taken at once.
2. Click a flat face: the top of a boss or the floor of a pocket. What will be replaced lights up, and the status line
   says what was found: **A boss 8.00 high** or **A pocket 4.00 deep**.
3. In the bar choose **Sketch only** or **Replace**. Replace is offered when a boss or pocket with straight walls stands
   on the face.
4. With Replace a **Height** field stands at the geometry: a new height can be typed at once, as a number or a formula.
5. **Enter** applies, **Esc** cancels. The tool stays in hand: click the next face.

## What comes out

- **Sketch N** on the face. Its lines are not tied to the body: drag them and change their sizes. Constraints
  (horizontal, vertical, parallel, tangent, equal) and sizes are laid so the sketch is **fully defined**. It is placed
  from the edges of the face the boss stands on (thin construction lines), or from the origin of the sketch when there
  are no straight edges.
- Sizes are rounded to 0.1; the status line says which, for example **rounded 29.97 -> 30.0**.
- With Replace, also a row **Boss by sketch h=8.0** (or **Pocket by sketch**). A double click opens the height, a double
  click on the sketch opens the sketch. Change a size of the sketch from 30 to 40 and the boss gets wider.
- Rounded corners of the boss (seen from above) become arcs of the sketch, tangent to their sides.
- A rounding or chamfer along the top or the foot of the boss goes with it and is laid again its size; the status line
  names it: **rounded R2.00 at its face**. Its size is a number in the node and can be changed.

## If it did not work

- **A sketch from a face is taken off a flat face**: click a flat face, not a cylinder or a rounding.
- **Nothing stands on this face as a boss or a pocket**: Sketch only still copies the outline.
- **A wall leans off square**: only bosses with straight walls are redrawn; Sketch only works.
- **The rim is rounded or cut along part of it only**: only a rim rounded or cut all round is laid again; take the odd
  rounding off ([Recognise feature](part/28-recognise-feature) -> Take off) or use Sketch only.
- **The rim is rounded or cut to different sizes**: the same - only a rim of one size is laid again.
- **The walls end at different heights**: such a boss cannot be drawn by one sketch; Sketch only works.
- **A hole goes deeper than the boss**: take the hole off first (Recognise feature -> Take off), or use Sketch only.
- The sketch is not fully defined (the status line says how many freedoms are left): add a size by hand.
