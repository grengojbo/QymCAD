# Corner fillet and chamfer

![A sharp corner and the same one rounded: the radius becomes a dimension, the tangencies become constraints.](img/sketch-corner/)

## How to do it

- **Fillet** (key **F**): click the corner of two lines or of a line and an arc, type the radius into the field at
  the corner, **Enter**.
- **Fillet by its chord or arc length**: on the bar above press **Chord** (the straight distance between the two
  ends of the arc) or **Arc length** instead of **Radius**, then click the corner, type the value, **Enter**.
- **Chamfer**: click the corner of two lines, type the size into the field at the corner, **Enter**.
- **Chamfer by two values**: on the bar above press **Two distances** (**Leg 1** and **Leg 2**) or **Leg and angle**
  (**Length** and the **Angle** between that line and the cut); **Symmetric** gives one size along both lines. The
  first value runs along the line you click nearer to: click the corner a little to the side of that line. Type the first
  value, **Tab** to the second, **Enter**.
- **Fillet every corner of the contour**: select the contour, press the button, type the **R of every corner**,
  **Enter**.

One step of undo per operation: **Ctrl+Z** brings the sharp corner back.

## What you get

A fillet inserts an arc and adds **two tangencies** and a **radius dimension** - or, given by its chord or arc length,
a dimension of the chord or of the arc length; a chamfer adds a segment and
its dimensions, measured from the sharp corner: the legs, or a leg and the angle. The sharp corner stays as a point
the dimensions stand on. All of these are constraints: move a side — the corner rebuilds itself; change the dimension — the fillet
changes.

Fillets usually go **last**, once the contour is defined: before that they get in the way of picking corners.

## If it did not work

- The size is not taken — it is more than the corner holds (longer than a side); the reason is written at the field.
- A chord or an arc length is not taken — the arc it makes reaches past the end of a side. Type a smaller value.
- The angle of a chamfer is not taken — with that angle the cut does not meet the other line. Make the angle smaller.
- The legs of a chamfer went the other way round — click the corner nearer to the line the first value should run
  along.
- The click did not take the corner — not exactly two lines meet at that point. Click right on the vertex of the
  corner.
