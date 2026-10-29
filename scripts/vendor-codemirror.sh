#!/usr/bin/env bash
# Empaquette CodeMirror 6 dans ui/vendor/codemirror.js et copie les polices dans ui/fonts/.
# C'est la seule étape npm du projet. Les fichiers produits sont commités :
# cargo build n'a jamais besoin de npm. Versions figées dans scripts/vendor/package-lock.json.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$root/scripts/vendor"
vendor="$root/ui/vendor"
fonts="$root/ui/fonts"

cd "$work"
npm ci --ignore-scripts --no-audit --no-fund

mkdir -p "$vendor" "$fonts"
meta="$(mktemp)"
trap 'rm -f "$meta"' EXIT
./node_modules/.bin/esbuild entry.js \
  --bundle --format=esm --minify --target=es2020 \
  --legal-comments=none --metafile="$meta" \
  --outfile="$vendor/codemirror.js"
node licenses.mjs "$meta" "$vendor/LICENSES.md"

for weight in 400 500 600 700; do
  cp "node_modules/@fontsource/inter/files/inter-latin-$weight-normal.woff2" "$fonts/"
done
for weight in 400 500; do
  cp "node_modules/@fontsource/jetbrains-mono/files/jetbrains-mono-latin-$weight-normal.woff2" "$fonts/"
done
cp node_modules/@fontsource/inter/LICENSE "$fonts/LICENSE-inter.txt"
cp node_modules/@fontsource/jetbrains-mono/LICENSE "$fonts/LICENSE-jetbrains-mono.txt"

echo "Écrit : ui/vendor/codemirror.js, ui/vendor/LICENSES.md, ui/fonts/"
