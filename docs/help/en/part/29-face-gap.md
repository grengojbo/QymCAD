# Size on body

The distance between two parallel flat faces of a body becomes a size you change by a number: the thickness of a plate,
the width of a block, the height of a step. Made for a body from a STEP or a mesh, which has no sketch to change a size
in.

## How

1. Press **Size on body** in the Part bar.
2. Click the first flat face, then a second one parallel to it: facing it or turned the same way. Both light up, and
   the status line says how far apart they are now: **Between the faces now: 40.00**.
3. In the bar choose what to **move**: **The first face**, **The second face** or **Both by half** - the middle then
   stays where it is.
4. Type the size into the field at the geometry - a number or a formula. The preview shows the new faces.
5. **Enter** applies, **Esc** cancels.

## Hole position

Click the round wall of a hole, then a flat face along its axis (a side wall, not the face the hole opens in). The field
shows the distance from the axis to the face: **From the axis of the hole to the face now: 15.00**. Type another and
press **Enter** - the hole moves, its diameter and depth as they were. The tree gets **Hole position 10**.

## What you get

- One row in the tree: **Size 50**. A double click opens the same tool with the size in the field and the same face
  moving.
- The faces beside follow the face that moves, as with [Push face](part/11-push-face). Recognised chamfers and roundings
  beside keep their size.

## If it did not work

- **This face is not parallel to the first** - click a flat face parallel to the first one.
- **The size is as it stands** - type another number.
- **The size between the faces is above zero** - the size, or the parameter it is typed as, came to 0: type a larger
  number or change the parameter.
- **The axis of the hole runs into this face** - click a flat face lying along the axis, a side wall rather than the face
  the hole opens in.
- **The distance from the axis of the hole to the face is above zero** - the distance, or the parameter it is typed
  as, came to 0: type a larger number or change the parameter.
- A face beside a curved one (the top with a hole on a body of exact surfaces, say) may not move: try moving the
  second face.
