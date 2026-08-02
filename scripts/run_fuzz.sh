#!/bin/sh
set -eu

duration=${1:-60}
mkdir -p fuzz/bin
mkdir -p target/go-cache
GOCACHE="$PWD/target/go-cache" go -C fuzz/go-oracle build -trimpath -o ../bin/go-oracle .
cargo build --release --locked --bin differential-oracle
python3 fuzz/harness.py \
    --duration "$duration" \
    --go fuzz/bin/go-oracle \
    --rust target/release/differential-oracle \
    --log fuzz/log.txt
