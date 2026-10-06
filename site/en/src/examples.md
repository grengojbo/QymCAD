# Examples

Each example is a ready text for Claude. Change the sizes to yours.

## 1. A corner bracket

> Make a corner bracket of two flanges 40×30×4 at a right angle, with a triangular rib 20×20 in the middle. In each flange two holes for M4, 20 mm apart. Round the inner corner with a radius of 3. Show it from iso and from the side.

What is interesting here: the two flanges are two blocks (`box`, `move`) that merge into one body. The rib is a sketched triangle extruded 4 mm.

## 2. A tray or a small box

> Make a small box 80×50×30 with a wall 2 mm thick, open at the top. Round the inside of the bottom with a radius of 2, the outer vertical edges with a radius of 4. Make the sizes parameters `w`, `d`, `h`.

What is interesting here: `shell` hollows it out with the top face open. The parameters let you say later "make the box 100 long".

## 3. A spacer or a bushing — several sizes at once

> Make a bushing: outer diameter 12, inner 6.4, height 10. Make the inner diameter and the height parameters. Save a 3MF. Then make the height 15 and save another 3MF under another name.

What is interesting here: changing a parameter rebuilds the model, and the second file appears without modelling from scratch.

## 4. A ready STL: scale and format

> Open `~/Downloads/gear.stl`. It is in inches — convert it to millimetres. Show it and save it as a 3MF.

What is interesting here: a mesh (STL) can be moved, measured and saved in another format. But it cannot be rounded or drilled as an exact model: a mesh has no exact faces. Claude says so if you ask for the impossible.

## 5. A ready STEP: add a hole

> Open `~/Downloads/housing.step`. Find the largest flat face on top and drill a through hole of 8 mm diameter in its centre. Save a STEP and a 3MF.

What is interesting here: a STEP is an exact model, so everything that works on your own works on it. Claude finds the face (`list_faces`, `resolve`) and lays the hole on it.

## 6. A ready template: "a part for FDM printing"

The server has ready **prompts** — conversation templates with the rules of printing (clearances for screws, wall thickness, chamfers instead of roundings at the bottom):

- **Claude Code:** type `/mcp__qymcad__design_for_fdm` — it asks which part to make and what the nozzle diameter is.
- **Claude Desktop:** the "+" button in the message box → qymcad → `design_for_fdm`.

There are also `edit_stl` and `edit_step` for changing ready files.
