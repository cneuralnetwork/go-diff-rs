#!/bin/sh
set -eu

mkdir -p bench/bin target/go-cache
GOCACHE="$PWD/target/go-cache" go -C bench/go-driver build -trimpath -o ../bin/go-bench-driver .
cargo build --release --locked --bin bench-driver
python3 bench/run.py
