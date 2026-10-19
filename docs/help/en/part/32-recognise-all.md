# Recognise all

Every hole, rounding and chamfer of a body becomes a feature you can edit, all at once. The program finds them by
itself and puts them in groups of one size; you leave out what should stay and press **Enter**. It works the same on
a STEP, on a recognised body and on a body made of a mesh.

## How

1. Turn a mesh into a body first with [Recognise](part/26-recognise), either way. A STEP needs nothing of the kind.
2. Press **Recognise all** in the "Part" bar and click the body. With a face of the body selected beforehand, the body
   is taken at once.
3. What was found is lit, and a list of groups appears beside the body: **Holes Ø5 through: 3**, **Chamfers 1: 4**,
   **Roundings R2: 8**. Each group has a size field, a number or a formula.
4. Untick a group that should be left alone. A click on a feature in the view leaves out only that feature; a second
   click takes it back. Point at a row to light that group alone.
5. **Enter** makes what is taken exact; **Esc** cancels.

## What comes out

- One row in the tree: **Recognised features (15)**. One **Ctrl+Z** takes it all back.
- The faces of all the features taken are removed together, and then each feature is made again by its own tool: the
  holes first, then the chamfers, then the roundings, the larger first. A chain of roundings running round each other
  at the corners is taken off and laid again whole.
- A double click on the row opens the same list with the sizes: a new number in a group changes every feature of it.
- The sizes are brought to what was drawn as in [Recognise feature](part/28-recognise-feature).

## If it did not work

- **No hole, rounding or chamfer was found on this body** - the body has no round walls and no slanted faces between
  two faces. Recognise a mesh body first.
- **... does not lay: ...** beside a group - the program could not lay that group and has unticked it; the rest is made.
  Such a group can be made on its own afterwards: [Recognise feature](part/28-recognise-feature) on one of its
  features, or [Fillet](part/05-fillet) or [Chamfer](part/06-chamfer).
- **Curved faces left as they are** - some curved faces were not recognised as a hole, a rounding or a chamfer: they
  stay as they are.
- **Nothing is taken** - tick at least one group or click a feature.
