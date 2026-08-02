# Upstream benchmark parity

`benches/upstream.rs` contains native translations of all nine top-level Go
benchmarks from the kickoff snapshot:

1. `BenchmarkDiffCommonPrefix`
2. `BenchmarkDiffCommonSuffix`
3. `BenchmarkCommonLength` (all prefix/suffix and empty/short/long subcases)
4. `BenchmarkDiffHalfMatch`
5. `BenchmarkDiffCleanupSemantic`
6. `BenchmarkDiffMain`
7. `BenchmarkDiffMainLarge`
8. `BenchmarkDiffMainRunesLargeLines`
9. `BenchmarkDiffMainRunesLargeDiffLines`

Run them with `cargo bench --bench upstream`. Fixed iteration counts replace
Go's adaptive `b.N` because the dependency-free Rust harness uses `Instant`
instead of an external benchmark framework. Workloads, fixtures, 1-second
timeout, line conversion, and cleanup operations are preserved. Rust's owned
cleanup API clones its input for each semantic-cleanup iteration; this ownership
cost is included rather than hidden.
