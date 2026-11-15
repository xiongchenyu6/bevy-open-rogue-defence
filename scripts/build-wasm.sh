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

# Recreate only the distribution; unchanged compiled inputs can reuse the exact
# published engine bytes, avoiding compiler/toolchain drift on content-only edits.
rm -rf dist
mkdir -p dist
engine_inputs_sha256="$(python3 tools/web_engine_inputs.py)"

build_engine() {
  local required_bindgen_version installed_bindgen_version
  required_bindgen_version="$(
    awk '/^name = "wasm-bindgen"$/ { getline; print; exit }' Cargo.lock |
      sed -E 's/.*"([0-9.]+)".*/\1/'
  )"
  if ! command -v wasm-bindgen >/dev/null 2>&1; then
    echo "wasm-bindgen ${required_bindgen_version} is required." >&2
    return 1
  fi
  installed_bindgen_version="$(wasm-bindgen --version | awk '{print $2}')"
  if [ "$installed_bindgen_version" != "$required_bindgen_version" ]; then
    echo "wasm-bindgen ${required_bindgen_version} is required; found ${installed_bindgen_version}." >&2
    return 1
  fi
  cargo build \
    --locked \
    --profile wasm-release \
    --target wasm32-unknown-unknown \
    --bin rogue_commander
  wasm-bindgen \
    --target web \
    --no-typescript \
    --out-dir dist \
    --out-name rogue_commander \
    target/wasm32-unknown-unknown/wasm-release/rogue_commander.wasm
  # Exclude timestamps/filenames so identical engine bytes compress identically.
  gzip -n -9 -kf dist/rogue_commander_bg.wasm
}

if [ -n "${WASM_REUSE_BASE_URL:-}" ] && python3 tools/reuse_web_engine.py \
  --base-url "$WASM_REUSE_BASE_URL" --output dist --fingerprint "$engine_inputs_sha256"; then
  echo "Compiled inputs unchanged; using the verified published engine."
else
  build_engine
fi

cp -R web/. dist/
cp -R assets dist/assets
cp -R THIRD_PARTY_NOTICES dist/THIRD_PARTY_NOTICES

# A deployment receipt lets release verification distinguish the new build from
# a cached successful page. It is intentionally separate from the player UI.
python3 - "$engine_inputs_sha256" <<'PY'
import hashlib
import json
import pathlib
import subprocess
import sys

sys.path.insert(0, "tools")
from web_engine_inputs import fingerprint

engine_inputs = sys.argv[1]
if fingerprint(pathlib.Path.cwd())[0] != engine_inputs:
    raise SystemExit("Compiled inputs changed during the build; rebuild before publishing.")
root = pathlib.Path("dist")
revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
wasm = root / "rogue_commander_bg.wasm"
(root / "build.json").write_text(json.dumps({
    "revision": revision,
    "engine_inputs_sha256": engine_inputs,
    "wasm_sha256": hashlib.sha256(wasm.read_bytes()).hexdigest(),
    "wasm_bytes": wasm.stat().st_size,
}, indent=2) + "\n")
PY

for required_asset in "${required_web_assets[@]}"; do
  copied_asset="dist/${required_asset}"
  if [ ! -s "$copied_asset" ] || ! cmp -s "$required_asset" "$copied_asset"; then
    echo "Required web asset was not copied intact: ${required_asset}" >&2
    exit 1
  fi
done

touch dist/.nojekyll

python3 tools/generate_web_manifest.py dist \
  --revision "$(git rev-parse HEAD)" --template web/sw.js

echo "WASM site built at ${project_root}/dist"
