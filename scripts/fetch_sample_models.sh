#!/usr/bin/env bash
# Downloads the Khronos glTF sample models the demos and the render benchmark
# use into examples/assets/models/ (not part of the repository; each model has
# its own licence, see https://github.com/KhronosGroup/glTF-Sample-Assets).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/examples/assets/models"
BASE="https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/main/Models"
mkdir -p "$DEST"

for model in DamagedHelmet Fox CesiumMan; do
    if [ -s "$DEST/$model.glb" ]; then
        echo "have $model.glb"
        continue
    fi
    echo "fetching $model.glb"
    curl -fL --retry 3 -o "$DEST/$model.glb" "$BASE/$model/glTF-Binary/$model.glb"
done
