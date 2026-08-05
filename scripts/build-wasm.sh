#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

required_web_assets=(
  "assets/fonts/rogue_commander_ui.ttf"
  "assets/fonts/OFL.txt"
)

for required_asset in "${required_web_assets[@]}"; do
  if [ ! -s "$required_asset" ]; then
    echo "Required web asset is missing or empty: ${required_asset}" >&2
    exit 1
  fi
done

required_bindgen_version="$(
  awk '/^name = "wasm-bindgen"$/ { getline; print; exit }' Cargo.lock |
    sed -E 's/.*"([0-9.]+)".*/\1/'
)"

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "wasm-bindgen ${required_bindgen_version} is required." >&2
  exit 1
fi

installed_bindgen_version="$(wasm-bindgen --version | awk '{print $2}')"
if [ "$installed_bindgen_version" != "$required_bindgen_version" ]; then
  echo "wasm-bindgen ${required_bindgen_version} is required; found ${installed_bindgen_version}." >&2
  exit 1
fi

cargo build \
  --locked \
  --profile wasm-release \
  --target wasm32-unknown-unknown \
  --bin rogue_commander

rm -rf dist
mkdir -p dist

wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir dist \
  --out-name rogue_commander \
  target/wasm32-unknown-unknown/wasm-release/rogue_commander.wasm

# Pages cannot be configured with a custom Content-Encoding header. Ship a
# gzip sidecar that the loading screen expands in browsers supporting the
# standard DecompressionStream API, while retaining the raw WASM as fallback.
gzip -9 -kf dist/rogue_commander_bg.wasm

cp web/index.html dist/index.html
cp -R assets dist/assets

for required_asset in "${required_web_assets[@]}"; do
  copied_asset="dist/${required_asset}"
  if [ ! -s "$copied_asset" ] || ! cmp -s "$required_asset" "$copied_asset"; then
    echo "Required web asset was not copied intact: ${required_asset}" >&2
    exit 1
  fi
done

touch dist/.nojekyll

echo "WASM site built at ${project_root}/dist"
