# What this is

You describe a part in words, the way you would describe it to a designer you know: "a plate 60 by 40, 4 thick, four holes for an M3 screw in the corners, the corners rounded". From that Claude builds a **real solid model** in QymCAD — with exact faces, sizes and a history of how it was built. Then it checks it by itself: it measures, looks at a picture, looks for mistakes. And it saves a file for printing (3MF, STL) or for another CAD program (STEP).

You do not need to know how to model. It helps only to know what you want to get and a few of its sizes.

What you need:

| What | What for |
|---|---|
| **Claude Desktop** or **Claude Code** | The conversation with Claude. Desktop is an ordinary program with a chat window; Code is the same in a terminal, for those who work there. |
| **QymCAD** | The program itself. It installs with it the `qymcad-mcp` server, through which Claude does the modelling "behind the scenes", with no window. The window is there to connect Claude, to open the result, turn it around and touch something up by hand. |
| **A slicer** (optional) | Bambu Studio, OrcaSlicer, PrusaSlicer — to print the 3MF or the STL. |
