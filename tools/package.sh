#!/usr/bin/env bash
# Package both artifacts:
#  - the .hmod via the pin's own cargo-horizon-mod (tools/build_hmod.sh)
#  - the source-only .hnmod (tools/package.py, deterministic bytes, in the port
#    root) with the built .hmod embedded at mixins/<id>/data/ (the AE2 pattern)
# Run inside a build slot (it compiles the packaging tool):
#   ports/tools/build-slot.sh run --job if-pack --port ports/industrialforegoing \
#       --step package -- env CARGO_INCREMENTAL=0 tools/package.sh
set -euo pipefail

PORT=$(cd "$(dirname "$0")/.." && pwd)
LOGS=$PORT/target/ladder
mkdir -p "$LOGS"
cd "$PORT"

tools/build_hmod.sh

python3 tools/package.py | tee "$LOGS/package-hnmod.log"
cp target/industrialforegoing-0.1.0.hnmod industrialforegoing.hnmod

echo "== artifacts =="
for f in content/industrialforegoing.hmod industrialforegoing.hnmod; do
    [ -f "$f" ] || { echo "missing $f"; exit 1; }
    size=$(stat -c %s "$f")
    sha=$(sha256sum "$f" | cut -d' ' -f1)
    entries=$(python3 -c "import zipfile,sys; print(len(zipfile.ZipFile(sys.argv[1]).namelist()))" "$f")
    echo "$f size=$size sha256=$sha entries=$entries"
done
