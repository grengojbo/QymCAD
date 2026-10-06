# Встановлення

Три кроки: **завантажити → встановити → підключити**.

## 1. Завантажте й встановіть QymCAD

Збірки лежать на сторінці [Releases](https://github.com/grengojbo/QymCAD/releases). Сервер для Claude (`qymcad-mcp`) уже всередині: окремо нічого завантажувати не треба.

| Система               | Файл                       | Як встановити                                                                                                                                   |
| --------------------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows 10/11         | `qymcad-…-x64.msi`         | Запустіть і пройдіть установку. Або `…-win64.zip`: розпакуйте будь-куди.                                                                        |
| macOS (Apple Silicon) | `qymcad-…-macos-arm64.zip` | Розпакуйте, перенесіть `QymCAD.app` у «Програми». Збірка без підпису Apple: один раз зніміть карантин командою з файлу `ПРОЧИТАЙ.txt` в архіві. |
| Linux                 | `qymcad-…-x86_64.AppImage` | Покладіть файл куди зручно (наприклад, `~/Applications`) і дозвольте йому запускатися: `chmod +x qymcad-*.AppImage`.                            |

## 2. Підключіть до Claude — одна кнопка

1. Відкрийте QymCAD.
2. **Довідка → Підключити до Claude.**
3. Оберіть, з чим працюєте:
   - **Claude Desktop** — програма сама додасть QymCAD у налаштування Claude Desktop. Інших налаштувань вона не змінює, а стару версію файлу зберігає поруч. Після цього **повністю закрийте Claude Desktop і відкрийте знову**.
   - **Claude Code** — програма покаже готову команду. Натисніть «Копіювати», вставте її в термінал і запустіть.

## 3. Перевірте

- **Claude Desktop:** у полі введення є кнопка інструментів (значок повзунків або «+»), а в ній — **qymcad**. Спитайте Claude: «Які інструменти qymcad тобі доступні?»
- **Claude Code:** `claude mcp list` показує `qymcad … ✓ Connected`; у сесії команда `/mcp` показує сервер та його інструменти.

Claude Code запускає сервер у поточній папці: відносні шляхи до файлів (`plate.3mf`) рахуються від неї. Зручно відкривати Claude Code в папці, куди мають лягати файли.

## Якщо кнопка не підходить — вручну

Те саме, що робить кнопка, можна зробити руками.

**Де лежить сервер:**

| Система | Шлях                                                                                                                 |
| ------- | -------------------------------------------------------------------------------------------------------------------- |
| macOS   | `/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp`                                                                 |
| Windows | поруч із `qymcad.exe`: `qymcad-mcp.exe` (встановлений з msi: `C:\Program Files\QymCAD\qymcad-mcp.exe`) |
| Linux   | сам AppImage з аргументом `mcp`                                                                                      |

**Claude Desktop** — файл налаштувань:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

Додайте до нього (macOS):

```json
{
  "mcpServers": {
    "qymcad": {
      "command": "/Applications/QymCAD.app/Contents/MacOS/qymcad-mcp"
    }
  }
}
```

Для Linux: `"command": "/home/ВИ/Applications/qymcad-….AppImage", "args": ["mcp"]`.

**Claude Code:**

```bash
claude mcp add qymcad -- /Applications/QymCAD.app/Contents/MacOS/qymcad-mcp
```
