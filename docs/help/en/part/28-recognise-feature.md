# Recognise feature

A hole, a rounding or a chamfer you can see on a body becomes a feature you can edit: **Hole Ø5**, **Rounding R2**,
**Chamfer 1**. Click it, check the size, **Enter**. It works on a body made of a mesh, on a recognised body and on a
STEP alike.

## How

1. Bring the mesh in and turn it into a body with [Recognise](part/26-recognise) - either way: exact surfaces or a
   polyhedron.
2. Press **Recognise feature** in the Part bar.
3. Click the feature: the wall of a hole, a strip of a rounding, the slanted face of a chamfer. The faces it is made of
   light up, and the status line says what was found and the size it is brought to: **Hole through: Ø4.98 -> Ø5**.
4. The size is in a field at the geometry - a number or a formula. A blind hole has its depth as well; a hole
   through goes through and has none.
5. **Enter** makes the feature exact; **Esc** cancels. The tool stays in hand: click the next feature.

## What you get

- One row in the timeline: **Recognised hole Ø5 through**, **Recognised rounding R2**, **Recognised chamfer 1**. The
  faces of the old feature are taken off - the faces around close over them - and the feature is made again by its
  own tool, round and exact.
- A double click on the row opens the same tool with the size in the field: type another diameter and the hole
  follows.
- Sizes are brought to what was most likely drawn: a tenth of a millimetre, and for a hole the nearest common bore
  for a metric screw - 3.2, 3.4, 4.5, 5.5, 6.6 and the like - when it is within 0.15 mm.

## If it did not work

- **No surface found here** - the mesh is rougher than the tolerance. Raise **Tolerance** in the bar (10, then 100)
  and click again.
- **No face to drill it from** - the round wall is a shaft or a boss, not a hole.
- **Not met by two faces along its length** - click another strip of the rounding, or round the edge with
  [Fillet](part/05-fillet).
- **The legs of this chamfer differ** - an uneven chamfer is made with [Chamfer](part/06-chamfer) and two distances.
- **No hole, rounding or chamfer here** - click the wall of a hole, a rounding, or a slanted face between two faces.
- A hole drawn with fewer than 8 flat strips is too rough to be told from flat faces: recognise the mesh again with a
  larger tolerance, or draw the hole with [Hole](part/08-hole).
