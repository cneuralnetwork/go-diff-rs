# Submission checklist

- [x] Public-repository target documented: `cneuralnetwork/go-diff-rs`
- [x] One-step native build: `make build`
- [x] One-step runnable container: `docker build --tag go-diff-rs .`
- [x] Untouched source snapshot pinned and fully hashed
- [x] 42/42 upstream test functions translated and passing
- [x] Nine/nine upstream benchmarks translated
- [x] Native CLI artifact covering diff, match, patch make/apply
- [x] Continuous native Go/Rust compatibility harness
- [x] Benchmark report with p99, startup, throughput, RSS, raw samples, method
- [x] Coverage profiles and non-conflated comparison
- [x] `DECISIONS.md` with 21 substantive decisions
- [x] Zero unsafe, zero Any, zero FFI, zero runtime dependencies evidence
- [x] Two minimized upstream bug reports and duplicate-search record for the
      primary panic
- [x] Five-minute live demo script/runbook
- [ ] Public push (performed by repository owner outside this workspace)
- [ ] Upstream issue filed by an account with repository issue permission (the connected integration returned HTTP 403); paste either ready report under `bug-cases/`
- [ ] Five-minute video URL inserted after the owner records/uploads it

Before submission, run `make check`, `make fuzz`, `make bench`, `make coverage`,
`make metrics`, `make parity-report`, `make docker-build`, and `make demo`. Commit
the regenerated evidence and replace the three remaining owner/external-action
checkboxes with links.
