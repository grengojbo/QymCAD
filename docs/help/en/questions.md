# Questions and answers

Short answers to what is asked most. The full story is behind the link.

## Where is the button of a tool?

The buttons stand on the left, in the strip beside the tree, and show what is open: in a sketch the buttons of the
sketch, in a part the buttons of the part. The buttons carry icons only; the name and the key come up in the tip when
the pointer rests on one. The quickest way is search: press **Space** or **Ctrl+K**, type the name, for example
"sketch from face", and press **Enter**. See [How any tool works](general/16-tools).

## How do I leave a sketch?

**Ctrl+Enter** finishes the sketch and goes back to the part. The same key goes up from a part to the assembly. See
[Keyboard shortcuts](general/10-hotkeys).

## The label of a dimension does not move

Take the number itself with the mouse: the dimension line and the extension lines do not move the label. See
[Dimensions](sketch/11-dimensions).

## The faces of a body run together into one patch

The edges of a body are drawn as a darker line, and the face under the pointer lights up while a tool that takes faces
is in hand. If there are no lines, check **Settings -> Viewport -> Body edges -> Shown**. See
[The viewport](general/09-viewport).

## How do I give a value a name?

Beside the value in the field of a tool stands **Parameter**: type a name, for example `d_hole`, and the value becomes
a parameter in **ƒx Parameters**. Change it there from then on, and everything that refers to it follows. See
[Parameters and formulas](general/05-parameters).

## How do I change a size of a STEP body that has no sketch behind it?

Click the face of the boss or pocket with [Sketch from face](part/30-sketch-from-face) and choose **Replace**: the
height and the main sizes of the outline become fields, the rest sizes of the sketch. Holes, roundings and chamfers are
made exact by [Recognise all](part/32-recognise-all).

## Where did Claude save the file?

A file named without a folder goes beside the file of the open document, and into **Downloads** while the document is
not saved. Ask Claude where the file is: the full path is in its answer. Nothing is written into the folder QymCAD is
installed in: an update replaces that folder. See [Connect to Claude](general/15-claude).

## Claude cut one part with another and nothing changed

A boolean works between the bodies of one part. To cut a pocket, ask Claude to sketch its outline on a face of the part
and extrude it as a cut.
