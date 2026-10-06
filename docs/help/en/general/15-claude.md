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

## If it did not work

- **The window says there is no server** — this build was not installed from a release. Install QymCAD from the
  Releases page: the server comes inside the package.
- **The settings could not be read** — the file of Claude Desktop holds something that is not settings, and it is
  left as it is. Open it, mend it or empty it, and press the button again.
- **Claude does not show qymcad** — quit Claude Desktop completely, not just its window, and open it again.
- **QymCAD was moved or updated** — press **Add to Claude Desktop** again, or run the command again: the path to the
  server may have changed.
