# Installing

Three steps: **download → install → connect**.

## 1. Download and install QymCAD

The builds are on the [Releases](https://github.com/grengojbo/QymCAD/releases) page. The server for Claude (`qymcad-mcp`) is already inside: there is nothing else to download.

| System | File | How to install |
|---|---|---|
| Windows 10/11 | `qymcad-…-x64.msi` | Run it and go through the installer. Or `…-win64.zip`: unpack it anywhere. |
| macOS (Apple Silicon) | `qymcad-…-macos-arm64.zip` | Unpack it and move `QymCAD.app` to Applications. The build has no Apple signature: clear the quarantine once with the command from `README.txt` in the archive. |
| Linux | `qymcad-…-x86_64.AppImage` | Put the file where you like (for example `~/Applications`) and let it run: `chmod +x qymcad-*.AppImage`. |

## 2. Connect Claude — one button

1. Open QymCAD.
2. **Help → Connect to Claude.**
3. Choose what you work with:
   - **Claude Desktop** — the program adds QymCAD to Claude Desktop's settings by itself. It changes no other setting and keeps the old version of the file beside it. Then **quit Claude Desktop completely and open it again**.
   - **Claude Code** — the program shows a ready command. Press "Copy", paste it into a terminal and run it.

## 3. Check

- **Claude Desktop:** the message box has a tools button (a sliders icon or "+"), and **qymcad** is in it. Ask Claude: "Which qymcad tools do you have?"
- **Claude Code:** `claude mcp list` shows `qymcad … ✓ Connected`; in a session, `/mcp` shows the server and its tools.

Claude Code starts the server in the current folder: relative file paths (`plate.3mf`) are counted from there. It is handy to open Claude Code in the folder where the files should land.

## When the button does not fit — by hand

What the button does can be done by hand.

**Where the server is:**

| System | Path |
|---|---|
| macOS | `/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp` |
| Windows | beside `qymcad.exe`: `qymcad-mcp.exe` (installed by the msi: `C:\Program Files\QymCAD\qymcad-mcp.exe`) |
| Linux | the AppImage itself, with the argument `mcp` |

**Claude Desktop** — the settings file:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

Add to it (macOS):

```json
{
  "mcpServers": {
    "qymcad": {
      "command": "/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp"
    }
  }
}
```

On Linux: `"command": "/home/YOU/Applications/qymcad-….AppImage", "args": ["mcp"]`.

**Claude Code:**

```bash
claude mcp add qymcad -- /Applications/QymCAD.app/Contents/MacOS/qymcad-mcp
```
