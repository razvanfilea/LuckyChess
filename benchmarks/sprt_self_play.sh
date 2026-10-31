#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN_DIR="${SCRIPT_DIR}/bin"
BASE_BIN="${BIN_DIR}/lucky_base"
DEV_BIN="${BIN_DIR}/lucky_dev"

# Arguments with defaults
ELO0="${1:-0.0}"
ELO1="${2:-5.0}"
TC="${3:-8+0.08}"
CORES="${CORES:-12}"

if [ ! -f "${BASE_BIN}" ]; then
    echo "Error: Baseline engine not found at ${BASE_BIN}"
    echo "Run ./fastchess/save_baseline.sh first on your base commit/branch."
    exit 1
fi

echo "=========================================================="
echo " Running SPRT Test: Dev (Current) vs Base (Snapshot)"
echo " Bounds: [${ELO0}, ${ELO1}] | TC: ${TC} | Threads: ${CORES}"
if [ -f "${BIN_DIR}/lucky_base.info" ]; then
    echo " Baseline info:"
    sed 's/^/   /' "${BIN_DIR}/lucky_base.info"
fi
echo "=========================================================="

# 1. Ensure opening book exists
"${SCRIPT_DIR}/download_book.sh"

# 2. Build current dev engine
echo "==> Compiling current (dev) version..."
cargo build --release --manifest-path "${REPO_ROOT}/Cargo.toml" --bin lucky_chess
cp "${REPO_ROOT}/target/release/lucky_chess" "${DEV_BIN}"

BOOK_PATH="${SCRIPT_DIR}/UHO_Lichess_4852_v1.epd"
PGN_DIR="${SCRIPT_DIR}/pgn"
mkdir -p "${PGN_DIR}"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
PGN_OUT="${PGN_DIR}/sprt_${TIMESTAMP}.pgn"

# 3. Run SPRT with fastchess
fastchess \
  -engine cmd="${DEV_BIN}" name=lucky_dev \
  -engine cmd="${BASE_BIN}" name=lucky_base \
  -openings file="${BOOK_PATH}" format=epd order=random \
  -each tc="${TC}" option.Hash=32 option.Threads=1 \
  -sprt elo0="${ELO0}" elo1="${ELO1}" alpha=0.05 beta=0.05 \
  -rounds 1000 \
  -repeat \
  -concurrency "${CORES}" \
  -draw movenumber=40 movecount=8 score=10 \
  -resign movecount=5 score=600 \
  -pgnout file="${PGN_OUT}" notation=san

# Auto-cleanup fastchess autosave config
rm -f "${REPO_ROOT}/config.json" "${SCRIPT_DIR}/config.json"

echo ""
echo "==> SPRT test finished. PGN saved to: ${PGN_OUT}"
echo ""
