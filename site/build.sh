#!/usr/bin/env bash
# THE DOCUMENTATION SITE: the guide to modelling with Claude, one mdBook a language, under one landing page that
# offers the languages. Built into target/site; .github/workflows/pages.yml publishes that folder to GitHub Pages.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=target/site
rm -rf "$OUT"
mkdir -p "$OUT"
for lang in uk en; do
    mdbook build "site/$lang" --dest-dir "$PWD/$OUT/$lang"
done
cp site/index.html "$OUT/index.html"
# GitHub Pages runs Jekyll over what it is given unless told not to; the books are finished HTML.
touch "$OUT/.nojekyll"
echo ">>> the site is in $OUT"
