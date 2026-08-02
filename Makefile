.PHONY: build test test-original verify-original lint check docker-build bench fuzz fuzz-smoke coverage metrics parity-report demo

build: verify-original
	cargo build --release --locked

test: verify-original
	cargo test --all-targets --locked

test-original: verify-original
	mkdir -p target/go-cache target/go-mod-cache
	cd tests/original/go-diff && GOCACHE="$(CURDIR)/target/go-cache" GOMODCACHE="$(CURDIR)/target/go-mod-cache" go test ./...

verify-original:
	sha256sum --check tests/original/SHA256SUMS
	sha256sum --check tests/original/SNAPSHOT_SHA256SUMS

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features --locked -- -D warnings

check: verify-original test-original test lint

docker-build:
	docker build --tag go-diff-rs:local .

bench:
	./scripts/run_bench.sh

fuzz:
	./scripts/run_fuzz.sh 60

fuzz-smoke:
	./scripts/run_fuzz.sh 5

coverage:
	./scripts/run_coverage.sh

metrics:
	./scripts/collect_metrics.sh

parity-report:
	./scripts/test_parity.sh

demo:
	./scripts/demo.sh
