# Connect to Claude

**Help -> Connect to Claude**.

Claude can model in QymCAD: you describe a part in words, and Claude builds it, checks it and saves the files. It
works through a server that comes inside the QymCAD package, beside the program. This window gives that server to
Claude Desktop and to Claude Code, so there is no path to look for.

## Claude Desktop

Press **Add to Claude Desktop**. QymCAD is added to the settings of Claude Desktop under the name `qymcad`; nothing
else there is changed, and the settings as they were are kept beside them in a file ending in `.qymcad-backup`.

Then quit Claude Desktop completely and open it again: closing its window is not enough. In the message box the tools
button now shows **qymcad**.

## Claude Code

The window shows a command. Press **Copy**, paste it into a terminal and run it once. `claude mcp list` then shows
`qymcad` as connected.

## Claude in this window

Claude can also work in the document open in this window instead of a document of its own. Turn it on:
**Settings -> General -> Claude in this window -> On**.

Then every change Claude makes appears in the window at once and is a step of its own: **Ctrl+Z** takes it back, as
it takes back your own steps. The status line says what Claude did last.

While you drag a part or hold a tool, Claude waits. If that lasts more than a few seconds, Claude is told the window
is busy and asks again later; the change it waited with does not happen behind your back.

Starting, opening and saving a document, and undoing, stay yours: Claude asks you to do them from the window's menu.

## If it did not work

- **The window says there is no server** — this build was not installed from a release. Install QymCAD from the
  Releases page: the server comes inside the package.
- **The settings could not be read** — the file of Claude Desktop holds something that is not settings, and it is
  left as it is. Open it, mend it or empty it, and press the button again.
- **Claude does not show qymcad** — quit Claude Desktop completely, not just its window, and open it again.
- **QymCAD was moved or updated** — press **Add to Claude Desktop** again, or run the command again: the path to the
  server may have changed.
- **Claude works in a document of its own, not in this window** — the setting was off, or QymCAD was closed, when
  Claude made its first change. Turn the setting on, then start Claude again: close and open Claude Desktop, or in
  Claude Code type `/mcp` and reconnect `qymcad`.
- **Claude says the window is busy** — finish what you are doing in the window: release the mouse, close the tool.
  Then ask Claude again.
- **Claude says the window has closed** — open QymCAD and start Claude again, as above. Claude does not move on to
  another window by itself: it could hold another document.
- **The status line says Claude cannot connect to this window** — turn the setting off and on again. If the same
  words come back, send them through **Help -> Report a problem**.
