# go-diff-rs

A native, safe Rust port of [`sergi/go-diff`](https://github.com/sergi/go-diff),
pinned to v1.4.0 commit `57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`.
It includes the complete diff, fuzzy-match, and patch API; all 42 upstream test
functions; all nine upstream benchmarks; a runnable CLI; and reproducible
parity evidence.

This is not a transpiler, wrapper, proxy, FFI bridge, or call-out to Go. The
shipped library and CLI contain only native Rust code, have zero third-party
runtime dependencies, and forbid unsafe code.

## One-command build

```sh
make build
```

The command verifies the pinned source/test hashes and produces the runnable
artifact at `target/release/go-diff-rs`. Rust 1.85.0 is selected automatically
by `rust-toolchain.toml`.

Container alternative:

```sh
docker build --tag go-diff-rs .
```

## Library usage

```rust
use diff_match_patch::DiffMatchPatch;

let dmp = DiffMatchPatch::new();
let diffs = dmp.diff_main("Lorem ipsum dolor.", "Lorem dolor sit amet.", false);
assert_eq!(dmp.diff_text1(&diffs), "Lorem ipsum dolor.");
assert_eq!(dmp.diff_text2(&diffs), "Lorem dolor sit amet.");
```

Go strings can contain arbitrary bytes; Rust `str` cannot. Public inputs accept
strings or bytes, and the `Text` return type preserves exact bytes with optional
UTF-8 accessors. The full symbol map is in [API_PARITY.md](API_PARITY.md).

## CLI

```text
go-diff-rs diff OLD NEW [--check-lines] [--format text|html|delta|patch]
go-diff-rs match TEXT PATTERN BYTE_LOCATION
go-diff-rs patch-make OLD NEW
go-diff-rs patch-apply PATCH TARGET
```

Example:

```sh
target/release/go-diff-rs diff demo/old.txt demo/new.txt --format patch
```

## Verified status

| Evidence | Current result |
|---|---:|
| Untouched kickoff test functions translated | 42 / 42 |
| Translated upstream tests passing | 42 / 42 (100%) |
| Additional Rust/CLI tests | 11 |
| Upstream benchmarks translated | 9 / 9 |
| Unsafe blocks/declarations | 0 |
| `dyn Any` / FFI declarations | 0 / 0 |
| Third-party runtime dependencies | 0 |
| Rust implementation lines | 2,944 (under 8,000) |
| Go original statement coverage | 99.06% |
| Rust library line coverage | 97.64% |
| Rust core-algorithm line coverage | 98.46% |

The 42/42 per-file result is generated in
`tests/port/PARITY_REPORT.json`; exact adaptations are disclosed in
[TEST_ADAPTATIONS.md](TEST_ADAPTATIONS.md). Original source files were not
edited. `tests/original/SHA256SUMS` hashes the suite and fixtures, while
`SNAPSHOT_SHA256SUMS` hashes the complete archived repository.

## Validation commands

```sh
make test             # Rust port, translated suite, additions, and CLI
make test-original    # untouched Go source suite
make parity-report    # per-file 42/42 report and transcripts
make lint             # rustfmt + clippy::all as errors
make fuzz             # 60-second deterministic compatibility session
make bench            # Go-vs-Rust p99/startup/RSS/throughput report
make coverage         # raw and summarized coverage evidence
make metrics          # unsafe/Any/FFI/dependency/LOC counts
make check            # hashes + both suites + lint
```

## Behavioral equivalence

The persistent comparison harness sends identical hex-encoded requests to
native Go and Rust processes. It compares exact diff operations, delta encoding
and round-trips, all three cleanup modes, HTML/text rendering, edit distance,
fuzzy matching, patch serialization/parsing, output bytes, and application
flags. The committed 60.002-second run completed 124,772 requests with zero
divergences. See `fuzz/log.txt` and [fuzz/README.md](fuzz/README.md).

The shared performance workload produces the same 2,210 diff records and exact
reconstructed lengths in both languages. On the recorded local run:

| Metric | Go v1.4.0 | Rust port |
|---|---:|---:|
| Algorithm p99 | 148.34 ms | 110.69 ms |
| Fresh-process p99 | 150.40 ms | 127.17 ms |
| Throughput | 8.541 ops/s | 11.596 ops/s |
| Peak RSS (10-op batch) | 8,724 KiB | 3,456 KiB |

These are honest local numbers, not universal claims. Raw samples, host/toolchain
metadata, fixture hashes, output shape, and confounders are in
[bench/methodology.md](bench/methodology.md) and `bench/results.json`.

## Compatibility bug found

The comparison campaign found that upstream `PatchMake` panics when invalid
UTF-8 is replaced by longer U+FFFD sequences and byte lengths are then applied
to the original string. It also exposed an unused blank-line-start expression
that changes semantic-lossless boundaries around CRLF. Minimized, pinned
reproducers and root-cause analyses are in [BUG_REPORT.md](BUG_REPORT.md) and
`bug-cases/`. The v1.4.0 compatibility profile preserves both behaviors
explicitly rather than silently claiming behavioral improvements. The primary
finding is filed upstream as
[`sergi/go-diff#157`](https://github.com/sergi/go-diff/issues/157), with a
focused fix proposed in
[`sergi/go-diff#158`](https://github.com/sergi/go-diff/pull/158).

## Repository map

```text
go-diff-rs/
├── src/                    native Rust library and runnable binaries
├── tests/original/         untouched, kickoff-hashed Go snapshot
├── tests/port/             42 translated tests plus Rust additions
├── fuzz/                   persistent compatibility harness and log
├── bench/                  methodology, raw samples, and results.json
├── coverage/               raw profiles and honest coverage comparison
├── bug-cases/              minimized upstream regression reproducer
├── DECISIONS.md            architectural divergences and rationale
├── Dockerfile              one-command runnable image
└── .port-mortem.toml       track/source/kickoff metadata
```

## Provenance and licensing

The upstream Go implementation is MIT-licensed; the underlying Google Diff,
Match and Patch work is Apache-2.0-licensed. Both complete license texts and the
upstream author/contributor lists are preserved. See
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). No pre-existing Rust port was
consulted or incorporated.
