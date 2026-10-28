#!/usr/bin/env bash
# Builds the two archives Meridian Installer installs from an engine release
# (Meridian-Installer docs/release-contract.md): meridian-examples.zip
# (`meridian.examples`) and meridian-assets.zip (`meridian.assets`). The SDK
# (`meridian.sdk`) is the tag's source archive and needs no asset.
#
# Usage: scripts/package_release_assets.sh [output-dir]   (default: dist/)
#
# meridian-examples.zip holds the examples crate at its root (Cargo.toml,
# build.rs, README.md, src/, examples/, assets/ — the scenes build.rs compiles
# and the audio and textures the examples load) and LICENSE, so it runs on its
# own. It builds inside an SDK checkout, because its manifest inherits the
# workspace's. The glTF sample models are not in the repository (see
# scripts/fetch_sample_models.sh) and so not in either archive.
# meridian-assets.zip holds the same sample assets (audio/, textures/, scenes/)
# and LICENSE at its root: the contents of examples/assets/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$(mkdir -p "${1:-$ROOT/dist}" && cd "${1:-$ROOT/dist}" && pwd)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

EXAMPLES="$ROOT/examples"

mkdir -p "$STAGE/examples/assets"
cp -R "$EXAMPLES/Cargo.toml" "$EXAMPLES/build.rs" "$EXAMPLES/README.md" \
    "$EXAMPLES/src" "$EXAMPLES/examples" "$STAGE/examples/"
cp -R "$EXAMPLES/assets/." "$STAGE/examples/assets/"
cp "$ROOT/LICENSE" "$STAGE/examples/"

mkdir -p "$STAGE/assets"
cp -R "$EXAMPLES/assets/." "$STAGE/assets/"
cp "$ROOT/LICENSE" "$STAGE/assets/"

rm -f "$OUT/meridian-examples.zip" "$OUT/meridian-assets.zip"
(cd "$STAGE/examples" && zip -qr "$OUT/meridian-examples.zip" .)
(cd "$STAGE/assets" && zip -qr "$OUT/meridian-assets.zip" .)
echo "wrote $OUT/meridian-examples.zip and $OUT/meridian-assets.zip"
