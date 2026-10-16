# Fully define

The constraints and dimensions a contour lacks, laid in one go, so that nothing of it can move any more. The sizes are
measured from the origin of the sketch.

## How to do it

1. Press **Fully define** on the left panel.
2. Click a contour: the whole of it is taken - every curve joined to the one clicked end to end. Or select its curves
   first (with **Shift** for several) and then press the button.
3. The sizes that will be laid are drawn dashed with their values, and the bar above says how many constraints and
   sizes **Enter** lays.
4. Press **Enter**. The status line says what was laid and that the contour is fully defined.

The tool stays in hand: click the next contour. **Esc** puts it down. **Undo** takes the whole of it back in one step.

## What it lays

- The obvious constraints first: horizontal and vertical lines, perpendicular and parallel sides, equal sides,
  tangent arcs.
- Then the sizes: the length of each side, the radius of each arc and circle, and how far the contour stands from the
  axes of the sketch.
- Each size is rounded to a tenth of a millimetre - "29.97 -> 30.0" - and the drawing follows it. Where a rounded size
  cannot be met, the measured one is kept.

A size that follows from the others is not laid: nothing is laid twice.

## If it did not work

- The status says to click a shape - the click landed off the curves. Click right on a line or an arc.
- Some freedom is left - the status says how many. Points standing alone, or curves not joined to the contour, are not
  taken: select them too, or join them to the contour.
- The sizes are not the ones you want - change them as any dimension: a double click on the value.
