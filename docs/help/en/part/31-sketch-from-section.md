# Sketch from section

A body of a mesh - a part brought in from STL, OBJ or 3MF, or made a polyhedron - has no sketch behind it. Cut it by a
plane, and its section becomes a sketch of lines, arcs and circles with their constraints and sizes on: extrude or
revolve it to have the part again, its sizes yours to change.

## How

1. Press **Sketch from section** in the Part bar (beside Sketch from face). A face selected already takes its body at
   once.
2. Click the body. It is cut through its middle, and the section is drawn on the plane: lines in one colour, arcs and
   circles in another, curves no line and no arc fits (splines) in a third. The bar counts them: **Lines 4, arcs 4,
   circles 1, splines 0**. The view is cut at the plane: what stands beyond it is not drawn, and the cut is filled, so
   you see where the body is cut. Esc gives the whole view back.
3. In the bar choose the plane: **XY (table)**, **XZ (front)** or **YZ (side)**. Or click a **datum plane** or a
   **flat face** - of this body or another one: the section is taken off it, and the field starts at the middle of the
   body. Click it again, or choose a plane in the bar, to go back to the bar's plane.
4. The **Offset** field moves the plane along its normal; the section follows it. The **Tolerance** field says how far a
   corner of the mesh may lie off the line or arc it is taken into - make it larger for a rough mesh.
5. **Enter** lays the sketch, **Esc** cancels. The tool stays in hand: change the offset and press Enter again for the
   next section.

## What comes out

- **Sketch N** on the plane. Where the offset is not zero, or the plane is a face, a **datum plane** is laid there
  first and the sketch stands on it: change the plane's distance - or move the face or the datum plane it is laid off -
  and the sketch moves with it.
- The curves are the sketch's own, not tied to the mesh. Constraints (horizontal, vertical, parallel, tangent, equal)
  and sizes are laid so the sketch is **fully defined**, measured from the origin of the sketch.
- Sizes are rounded to 0.1; the status line says which, for example **rounded 39.96 -> 40.0**. A hole is rounded to
  the nearest standard bore within 0.15 - a clearance or tapping hole for a metric screw: a printed hole of 5.42 is
  laid **5.5**.
- The mesh stays as it is. Make the body from the sketch with **Extrude** or **Revolve**, then hide the mesh with the
  eye in the tree.

## If it did not work

- **This body is made of exact surfaces**: its edges are exact already - take its outline with
  [Sketch from face](part/30-sketch-from-face) or a projection in a sketch.
- **The plane misses the body**: the offset puts the plane beside it. Change the offset.
- **Click a body of a mesh first**: a plane was clicked with no body held. Click the body, then the plane.
- **The body was let go** when you clicked it: the face it was taken by lets it go. Click another flat face for the
  plane.
- **A circle comes out as many short lines**: the mesh is too coarse - a circle needs at least 8 corners. Make the
  tolerance larger, or bring the mesh in finer.
- **A rounding comes out as lines**: a rounding of 2 or 3 chords cannot be told from a chamfer; draw the arc by hand.
- **Splines where you expected arcs**: the corners of the mesh are shaken more than the tolerance; make it larger.
- The sketch is not fully defined (the status line says how many freedoms are left): the points of a spline are left
  free; add sizes by hand.
