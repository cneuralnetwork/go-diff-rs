#!/bin/sh
set -eu

printf '\n[1/7] Pinned source and toolchains\n'
sed -n '1,12p' .port-mortem.toml
rustc --version
go version

printf '\n[2/7] One-command release build\n'
make build
target/release/go-diff-rs --help

printf '\n[3/7] Untouched source suite\n'
make test-original

printf '\n[4/7] Native Rust suite and CLI tests\n'
make test

printf '\n[5/7] Live CLI output\n'
target/release/go-diff-rs diff demo/old.txt demo/new.txt --format patch

printf '\n[6/7] Behavioral and performance evidence\n'
sed -n '1,18p' fuzz/log.txt
jq '{workload, original_go, port_rust, rust_over_go_ratio}' bench/results.json

printf '\n[7/7] Honest quality and coverage totals\n'
jq . evidence/metrics.json
jq '{go: .go_original.total, rust_library: .rust_port.library, rust_algorithms: .rust_port.algorithm_modules}' coverage/report.json
