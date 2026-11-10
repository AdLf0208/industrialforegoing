#!/usr/bin/env bash
# Build the content half and put it where both the staged check and the bundle
# need it:
#   content/industrialforegoing.hmod                       the delivered content mod
#   mixins/industrialforegoing/data/industrialforegoing-content.hmod
#                                                          the same bytes, which the
#                                                          mixins planner stages as
#                                                          `native-data/<id>/` for
#                                                          `native_data!`
# Run inside a build slot:
#   ports/tools/build-slot.sh run --job if-hmod --port ports/industrialforegoing \
#       --step package -- env CARGO_INCREMENTAL=0 tools/build_hmod.sh
set -euo pipefail

PORT=$(cd "$(dirname "$0")/.." && pwd)
HORIZON=${HORIZON:-${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}/ports/_shared/pins/horizon-1d63e0a}
LOGS=$PORT/target/ladder
mkdir -p "$LOGS" "$PORT/mixins/industrialforegoing/data"
cd "$PORT"

CARGO_TARGET_DIR=$PORT/target/carry-tools
export CARGO_TARGET_DIR
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod build) \
    2>&1 | tee "$LOGS/package-hmod.log"
cp content/target/horizon-mod/industrialforegoing.hmod content/industrialforegoing.hmod
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod check industrialforegoing.hmod) \
    2>&1 | tee -a "$LOGS/package-hmod.log"
cp content/industrialforegoing.hmod mixins/industrialforegoing/data/industrialforegoing-content.hmod

size=$(stat -c %s content/industrialforegoing.hmod)
sha=$(sha256sum content/industrialforegoing.hmod | cut -d' ' -f1)
entries=$(python3 -c "import zipfile,sys; print(len(zipfile.ZipFile(sys.argv[1]).namelist()))" content/industrialforegoing.hmod)
echo "content/industrialforegoing.hmod size=$size sha256=$sha entries=$entries"
