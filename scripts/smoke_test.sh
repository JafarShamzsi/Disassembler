#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SAMPLE="tests/notepad.exe"

if [ ! -f "$SAMPLE" ]; then
    echo "Missing smoke-test sample: $SAMPLE" >&2
    exit 1
fi

echo "[smoke] fmt"
cargo fmt --check

echo "[smoke] tests"
cargo test

echo "[smoke] clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "[smoke] names"
cargo run -- "$SAMPLE" --names > /tmp/disassembler-smoke-names.txt
grep -q "\[NAMES\] Binary Names" /tmp/disassembler-smoke-names.txt
grep -q "Imports:" /tmp/disassembler-smoke-names.txt

echo "[smoke] functions"
cargo run -- "$SAMPLE" --functions > /tmp/disassembler-smoke-functions.txt
grep -q "\[FUNCTIONS\] Function Analysis" /tmp/disassembler-smoke-functions.txt
grep -q "Callers" /tmp/disassembler-smoke-functions.txt

echo "[smoke] json export"
cargo run -- "$SAMPLE" --output /tmp/disassembler-smoke.json --format json > /tmp/disassembler-smoke-export.txt
grep -q '"binary_format": "PE"' /tmp/disassembler-smoke.json
grep -q '"import_count":' /tmp/disassembler-smoke.json

echo "[smoke] ok"
