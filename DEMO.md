# Five-minute demo runbook

The presenter records the terminal while running `make demo`. Keep
`DECISIONS.md`, `fuzz/log.txt`, and `bench/results.json` ready in adjacent tabs.

## 0:00–0:35 — provenance and architecture

- Show `.port-mortem.toml`: source URL, v1.4.0 commit, track H.
- State that Go exists only as a separately built reference for tests/evidence.
- Point out the zero-dependency Cargo manifest and `forbid(unsafe_code)`.

## 0:35–1:05 — one-command artifact

- Let `make build` verify hashes and build the release binary.
- Show CLI help and the exact artifact path.
- Mention `docker build --tag go-diff-rs .` as the isolated alternative.

## 1:05–2:10 — tests live

- Show the untouched Go suite passing.
- Show all Rust tests, including the 42 translated functions, CLI round-trip,
  byte-invalid case, and multithreaded soak.
- Open `tests/port/PARITY_REPORT.json` briefly: 42/42, 100% per file, no missing
  or extra translated names.

## 2:10–2:45 — working behavior

- Show the live patch-format CLI output for `demo/old.txt` → `demo/new.txt`.
- Optionally run patch-make/apply if time remains.
- Explain that library inputs accept bytes and return byte-preserving `Text`.

## 2:45–3:30 — continuous compatibility evidence

- Show `fuzz/log.txt`: at least 60 seconds, deterministic seed, case count,
  binary hashes, zero divergences.
- Explain the compared surfaces: diff sequence, delta, rendering, matching,
  serialization/parsing, and patch application.

## 3:30–4:15 — honest performance and coverage

- Show p99, fresh-process p99, throughput, and RSS for both native binaries.
- State the fixture/output-shape equality and uncontrolled-host confounders.
- Show 99.06% Go statements versus 97.64% Rust library lines; explicitly say
  those coverage models differ.

## 4:15–5:00 — engineering discipline and bonus work

- Show zero unsafe/Any/FFI/dependencies and the under-8,000 line count.
- Scroll through several of the 21 decisions, not just their headings.
- Show both minimized upstream findings and the filed Bug Catcher issue:
  https://github.com/sergi/go-diff/issues/157.
- End on the README reproduction commands and public repository URL.
