#!/usr/bin/env bash
# Assemble QymCAD into a macOS .app and zip it. Runs after `cargo build --release` on an Apple Silicon
# machine, with OCCT installed at $OCCT_ROOT (built from source - see .github/workflows/release.yml).
#
# WHAT MAKES A MAC BUNDLE DIFFERENT. On Linux `linuxdeploy` gathers the shared libraries and rewrites
# their paths; on Windows the DLLs simply sit beside the executable and are found there. macOS does
# neither: a dylib carries the path it was BUILT at, baked into whatever loads it, so a copied library is
# looked for where it used to live and the program dies on start. `install_name_tool` rewrites those
# paths to `@rpath`, and `@rpath` is pointed at the bundle's own Frameworks directory.
set -euo pipefail

BIN=target/release/qymcad
[ -x "$BIN" ] || { echo "!!! no $BIN - run cargo build --release first"; exit 1; }
# THE SERVER FOR CLAUDE travels inside the app, beside the program: it is built against the same kernel libraries
# and finds them in the same Frameworks, so a person connecting Claude has nothing more to download.
SERVER=target/release/qymcad-mcp
[ -x "$SERVER" ] || { echo "!!! no $SERVER - run cargo build --release --bin qymcad --bin qymcad-mcp first"; exit 1; }
OCCT_ROOT=${OCCT_ROOT:?set OCCT_ROOT to the OCCT installation}

# THE NAME. A tag names the package itself; anything else carries the commit, so two builds three days
# apart cannot share a file name and a report can always be traced to one of them.
if [ -n "${QYMCAD_VERSION:-}" ]; then
    NAME="qymcad-${QYMCAD_VERSION#v}"
else
    VER=$(grep -m1 '^version' Cargo.toml | sed 's/[^0-9.]//g')
    git config --global --add safe.directory "$PWD" 2>/dev/null || true
    SHA=$(git rev-parse --short=9 HEAD 2>/dev/null || true)
    NAME="qymcad-${VER:-0.0.0}${SHA:+-dev.$SHA}"
fi

APP=dist/QymCAD.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp "$BIN" "$APP/Contents/MacOS/qymcad"
cp "$SERVER" "$APP/Contents/MacOS/qymcad-mcp"
cp assets/icons/macos/qymcad.icns "$APP/Contents/Resources/"

# The licence and the notices travel with the binary: AGPL asks for the licence text to accompany the
# program, LGPL-2.1 (OCCT) for the notice.
cp LICENSE "$APP/Contents/Resources/LICENSE.txt"
cp THIRD-PARTY-NOTICES.md "$APP/Contents/Resources/"

VER_PLIST=$(grep -m1 '^version' Cargo.toml | sed 's/[^0-9.]//g')
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>QymCAD</string>
    <key>CFBundleDisplayName</key><string>QymCAD</string>
    <key>CFBundleIdentifier</key><string>io.github.grengojbo.qymcad</string>
    <key>CFBundleExecutable</key><string>qymcad</string>
    <key>CFBundleIconFile</key><string>qymcad</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${VER_PLIST:-0.0.0}</string>
    <key>CFBundleVersion</key><string>${VER_PLIST:-0.0.0}</string>
    <key>LSMinimumSystemVersion</key><string>12.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# --- the kernel libraries, with their paths rewritten to live inside the bundle ---
#
# THE LINKS ARE KEPT AS LINKS, and that is the difference between an 80 MB download and a third of it.
# OCCT installs every module under three names - `libTKernel.dylib` -> `libTKernel.7.9.dylib` ->
# `libTKernel.7.9.3.dylib`, one file and two links to it. A plain `cp` follows each link and writes THREE
# full copies: 144 files for 48 modules, measured on the runner. Only one of the three is ever loaded -
# the name written into the dependencies - so the other two were pure weight.
#
# `-RP` copies links as links on both toolchains (POSIX: with -R and no -H/-L, a link is copied, not
# followed), and `zip -y` stores them as links instead of expanding them again in the archive.
echo ">>> gathering the kernel libraries"
cp -RP "$OCCT_ROOT"/lib/*.dylib "$APP/Contents/Frameworks/"
n=$(find "$APP/Contents/Frameworks" -type f -name '*.dylib' | wc -l | tr -d ' ')
links=$(find "$APP/Contents/Frameworks" -type l -name '*.dylib' | wc -l | tr -d ' ')
[ "$n" -gt 0 ] || { echo "!!! no dylibs found under $OCCT_ROOT/lib"; exit 1; }
echo ">>> libraries: $n, links to them: $links"

# The executable looks for them beside itself, one directory up and into Frameworks. This one is NOT
# allowed to fail quietly: without the rpath every library below is unreachable and the program does not
# start at all, so a swallowed error here would ship as a green build.
install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/qymcad"
install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/qymcad-mcp"

# What a file depends on, one path per line. The header line - the file's own name - is dropped.
#
# NOT A PIPELINE ENDING IN `grep`. This script runs under `set -o pipefail`, and `grep` returns 1 when it
# matches nothing: a bundle whose paths are ALREADY `@rpath` (which is what OCCT built by CMake produces)
# made the whole script end right here, silently, six minutes into a build.
deps_of() {
    otool -L "$@" | awk 'NR>1 {print $1}'
}

# Each library must both ANNOUNCE itself by @rpath and LOOK FOR its neighbours the same way: the OCCT
# modules depend on one another, and a path left pointing at the build machine is a start-up failure on
# anybody else's.
rewritten=0
rewrite_deps() {
    local file=$1 dep
    while IFS= read -r dep; do
        case "$dep" in
            "$OCCT_ROOT"/*)
                install_name_tool -change "$dep" "@rpath/$(basename "$dep")" "$file" 2>/dev/null || true
                rewritten=$((rewritten + 1))
                ;;
        esac
    done <<< "$(deps_of "$file")"
}

for lib in "$APP"/Contents/Frameworks/*.dylib; do
    # A link is the same file under another name: writing through it would set the id of the real file to
    # whichever name came last, and the loop would undo its own work two times out of three.
    if [ -L "$lib" ]; then
        continue
    fi
    install_name_tool -id "@rpath/$(basename "$lib")" "$lib" 2>/dev/null || true
    rewrite_deps "$lib"
done
rewrite_deps "$APP/Contents/MacOS/qymcad"
rewrite_deps "$APP/Contents/MacOS/qymcad-mcp"
echo ">>> paths rewritten to @rpath: $rewritten"

# NOT A PATH LEFT POINTING HOME. A single dependency still naming the build machine means the program
# starts nowhere but here, and it says so only on somebody else's computer.
#
# THE CHECK IS NOT `grep -q`. Under `set -o pipefail` an early-exiting `grep -q` kills `otool` with a
# broken pipe, the pipeline reports that failure, and the `if` reads it as "nothing found" - the sentinel
# passed a bundle in which every path still named the build machine. Read it all, then look.
left=$(otool -L "$APP/Contents/MacOS/qymcad" "$APP/Contents/MacOS/qymcad-mcp" "$APP"/Contents/Frameworks/*.dylib | grep "$OCCT_ROOT" || true)
if [ -n "$left" ]; then
    echo "!!! a library still points at the build machine:"
    printf '%s\n' "$left" | head -5 || true
    exit 1
fi

# UNSIGNED, AND SAID SO IN BOTH LANGUAGES. Without an Apple signature the system marks the download as
# quarantined and refuses to open it: "the app is damaged, move it to the Bin" - which it is not.
#
# THE RIGHT-CLICK IS NOT THE WAY ANY MORE. These notes used to say "Control-click and choose Open", the
# advice that worked for years. Reported behaviour: on a current macOS it does nothing - the same refusal
# appears, and the program went to the Bin instead. Recent releases block code that carries no signature
# from an identified developer in every condition, and the Control-click exception went with them.
#
# What is left is the mark itself: `xattr -cr` clears the quarantine attribute, and the program then opens
# by an ordinary double click. It is done once per download - an extended attribute stays cleared, a
# restart does not bring it back - so the steps are written out plainly, for a person who has never opened
# a terminal. The app is moved to Applications first, as the guide on the site says, so the command names
# it by a path that is the same on every Mac and nothing has to be dragged into the terminal.
cat > dist/README.txt <<'TXT'
QymCAD - build for macOS (Apple Silicon).

FIRST RUN. The build carries no Apple developer signature, and macOS marks everything downloaded
from the internet as "quarantined": it will say the app is damaged and offer to move it to the Bin.
It is not damaged. The mark has to be cleared, once.

  1. Unpack the archive and move QymCAD.app to Applications.

  2. Open Terminal: Command+Space, type "Terminal", press Enter.

  3. Type these two lines into it, pressing Enter after each:

        cd /Applications
        xattr -cr QymCAD.app

     Nothing is printed in reply; that is how it should be.

  4. Open QymCAD.app with an ordinary double click.

The mark is gone for good on this copy: a restart does not bring it back. A build downloaded anew
has to be cleared the same way.

Requires macOS 12 or newer, an Apple Silicon machine.
TXT

cat > dist/ПРОЧИТАЙ.txt <<'TXT'
QymCAD - збірка для macOS (Apple Silicon).

ПЕРШИЙ ЗАПУСК. Збірка не має підпису розробника Apple, а macOS позначає все завантажене з інтернету
«карантином»: вона скаже, що програму пошкоджено, і запропонує перемістити її в Смітник. Програму не
пошкоджено. Позначку треба зняти, один раз.

  1. Розпакуйте архів і перенесіть QymCAD.app у «Програми».

  2. Відкрийте Термінал: Command+Пробіл, наберіть «Термінал», Enter.

  3. Наберіть у ньому ці два рядки, після кожного натискаючи Enter:

        cd /Applications
        xattr -cr QymCAD.app

     У відповідь нічого не надрукується, так і має бути.

  4. Відкрийте QymCAD.app звичайним подвійним клацанням.

Позначку знято назавжди для цієї копії: перезавантаження її не поверне. Збірку, завантажену знову,
доведеться звільнити так само.

Потрібна macOS 12 або новіша, комп'ютер на Apple Silicon.
TXT

ZIP="dist/$NAME-macos-arm64.zip"
rm -f "$ZIP"
( cd dist && zip -r -y -q "$(basename "$ZIP")" QymCAD.app README.txt ПРОЧИТАЙ.txt )
echo ">>> DONE: $ZIP ($(du -h "$ZIP" | cut -f1))"
