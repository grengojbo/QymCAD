#!/usr/bin/env bash
# THE DOCUMENTATION SITE: one mdBook a language - the guide to modelling with Claude, then the help of the program,
# the same articles the help window shows (docs/help) - under one landing page that offers the languages. Built into
# target/site; .github/workflows/pages.yml publishes that folder to GitHub Pages.
#
# The book of a language is put together in target/site-src: its own pages from site/<lang>, and the help laid in
# beside them by the help-book tool of qymcad-help, which rewrites the links of the articles for the pages and
# refuses a link that names nothing.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=target/site
STAGE=target/site-src
rm -rf "$OUT" "$STAGE"
mkdir -p "$OUT" "$STAGE"
cargo build -q -p qymcad-help --bin help-book
for lang in uk en; do
    cp -R "site/$lang" "$STAGE/$lang"
    target/debug/help-book "$lang" "$PWD/$STAGE/$lang/src"
    mdbook build "$STAGE/$lang" --dest-dir "$PWD/$OUT/$lang"
done
cp site/index.html "$OUT/index.html"
# GitHub Pages runs Jekyll over what it is given unless told not to; the books are finished HTML.
touch "$OUT/.nojekyll"
echo ">>> the site is in $OUT"
