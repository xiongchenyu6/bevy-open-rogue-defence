#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${1:-$project_dir/target/desktop-packages}"
mkdir -p "$project_dir/target/steamrt-build" "$project_dir/target/steamrt-cargo" "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"
if [[ -e "$output_dir/rogue-commander-linux" ]]; then
  echo "Package output already exists; choose another output directory." >&2
  exit 2
fi
image='rogue-commander-linux-sdk:rust-1.98.0-sniper'
docker build --file "$project_dir/tools/desktop/Dockerfile" --tag "$image" "$project_dir/tools/desktop"
docker run --rm --user "$(id -u):$(id -g)" \
  --mount "type=bind,src=$project_dir,dst=/project,readonly" \
  --mount "type=bind,src=$project_dir/target/steamrt-build,dst=/build" \
  --mount "type=bind,src=$project_dir/target/steamrt-cargo,dst=/cache" \
  --mount "type=bind,src=$output_dir,dst=/out" \
  --env CARGO_HOME=/cache --env CARGO_TARGET_DIR=/build --env CARGO_INCREMENTAL=0 \
  --workdir /project "$image" bash -c '
    set -euo pipefail
    rustc --version
    cargo build --locked --release --bin rogue_commander -j2
    cargo metadata --locked --filter-platform x86_64-unknown-linux-gnu --format-version 1 > /build/metadata-linux.json
    python3 tools/package_desktop.py --binary /build/release/rogue_commander \
      --metadata /build/metadata-linux.json --output /out/rogue-commander-linux \
      --profile release --target x86_64-unknown-linux-gnu
  '
