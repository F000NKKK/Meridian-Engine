#!/usr/bin/env bash
# Runs the four render benchmark scenes (helmet, shadows, instances, crowd) at
# 1920x1080 and writes one JSON line per scene to bench_results.jsonl, with a
# readable table per scene on the terminal. Needs the sample models:
#   scripts/fetch_sample_models.sh
# Optional: FRAMES=300 WIDTHxHEIGHT via MERIDIAN_WINDOW=2560x1440.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
FRAMES="${FRAMES:-300}"
OUT="${OUT:-bench_results.jsonl}"
: > "$OUT"

cargo build --release -p meridian-examples --example render_bench
for scene in helmet shadows instances crowd; do
    echo "== $scene"
    ./target/release/examples/render_bench "$scene" --frames "$FRAMES" >> "$OUT"
done
echo
echo "Done: send me $OUT (one JSON line per scene)."
