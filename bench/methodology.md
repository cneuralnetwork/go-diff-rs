# Benchmark methodology

## Shared workload

Both native drivers import/read the untouched upstream `speedtest1.txt` and
`speedtest2.txt` fixtures and call `DiffMain(first, second, true)` with default
configuration. Fixture SHA-256 values, byte lengths, output diff count, and
reconstructed byte lengths are recorded in `results.json`.

## Metrics

- Algorithm latency: 50 sequential calls inside one process, measured with each
  language's monotonic clock. The report contains min, mean, p50, p95, p99, and
  max; percentile selection uses nearest rank.
- Throughput: reciprocal of mean per-operation algorithm latency.
- Startup: 30 fresh process invocations, each including process startup, fixture
  reads, one operation, and normal exit. Python's monotonic nanosecond clock
  measures the whole invocation.
- RSS: GNU `/usr/bin/time -f %M` peak resident set size for a 10-operation batch,
  reported in KiB.

Raw latency/startup samples and the RSS tool outputs are kept under `bench/raw/`.
The source and port run sequentially on the same otherwise-uncontrolled host.
No CPU affinity, turbo-frequency control, cache flushing, or statistical
outlier removal is used; these are explicit environmental confounders. Numbers
are local comparative evidence, not universal performance claims.

## Reproduction

```sh
make bench
cargo bench --bench upstream
```

`make bench` regenerates the comparative report. `cargo bench --bench upstream`
runs the native translations of all nine top-level upstream Go benchmarks.
Override sample counts with `GO_DIFF_BENCH_SAMPLES` and
`GO_DIFF_STARTUP_SAMPLES`.
