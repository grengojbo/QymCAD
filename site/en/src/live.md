# Claude in the QymCAD window

Claude can work on a document of its own "behind the scenes", or right in the window you have open. Then you see every change it makes at once, and you can show it with the mouse what you mean.

## Turning it on

In QymCAD: **Settings -> General -> Claude in this window -> On**. Nothing else to change: Claude Desktop and Claude Code connected through **Help -> Connect to Claude** find the window themselves.

Turn it on any time: even when Claude Desktop was opened before QymCAD, or the switch was turned on mid-conversation, Claude finds the window at its next request. The one exception: once Claude has built something in a document of its own, it stays with it so the work is not lost.

For now this works on macOS and Linux. On Windows Claude works on a document of its own.

## What you see

Every change Claude makes appears in the window at once and is a step of its own: **Ctrl+Z** takes it back, like your own steps. The status line says what Claude did last, e.g. "Claude: Fillet".

While you drag a part or hold a tool, Claude waits. If that lasts more than a few seconds, Claude is told the window is busy and asks again later — the change does not happen behind your back.

Starting, opening and saving a document, and undoing, are yours, from the window's menu; Claude will ask.

## Show Claude what you mean

Select a face, an edge, a plane, a sketch or a feature in the window and say "this", "this face", "this radius". Claude reads what is selected and knows:

- what it is (a flat face, a cylinder of radius 2 mm, an edge 40 mm long, a work plane…);
- where it is;
- which operation made it — so "make this radius 0.2 mm smaller" changes exactly that fillet.

Examples:

> Select a face of the fillet: "make the radius 0.2 mm smaller".

> Select the top face: "draw a circle 8 mm across in its middle and extrude it 5 mm".

> Select a work plane: "make a 5 mm hole through in this plane".

If the radius follows a parameter (`r`), Claude writes the change into that fillet's own expression (`r - 0.2`) and leaves the parameter, which other operations may read. If one fillet makes several faces, Claude warns that they all change.

Nothing selected — Claude asks you to click.

## The picture — as you see it

In the window Claude looks at the model through your camera: turn and zoom, and Claude sees the same. A standard view can still be asked for: "show it from the top".

## Your language

Claude knows which language your QymCAD is in, and names menus the way you see them.

## If it did not work

- **Claude says there is no window** — turn the switch on as above and ask again; nothing needs restarting.
- **Claude works on a document of its own, not in the window** — it had already built something there before the switch was turned on. Restart Claude: Claude Desktop — quit and open again; Claude Code — `/mcp` and reconnect `qymcad`.
- **"The window is busy"** — finish what you are doing (release the mouse, close the tool) and ask again.
- **"The window has closed"** — open QymCAD again and ask again: Claude first reads what is open (it could be another document) and goes on.
