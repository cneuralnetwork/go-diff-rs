#!/bin/sh
set -eu

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov 0.8.6 is required; see coverage/README.md" >&2
    exit 2
fi

mkdir -p coverage target/go-cache target/go-mod-cache
GOCACHE="$PWD/target/go-cache" \
GOMODCACHE="$PWD/target/go-mod-cache" \
go -C tests/original/go-diff test -coverprofile=../../../coverage/go.out ./...
GOCACHE="$PWD/target/go-cache" \
GOMODCACHE="$PWD/target/go-mod-cache" \
go -C tests/original/go-diff tool cover \
    -func="$PWD/coverage/go.out" > coverage/go-functions.txt
cargo llvm-cov --lib --tests --locked --json --summary-only \
    --output-path coverage/rust.json
python3 scripts/coverage_report.py
