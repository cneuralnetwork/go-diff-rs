#!/usr/bin/env python3
"""Reproducible source-versus-port benchmark report generator."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "bench" / "raw"
FIRST = ROOT / "tests" / "original" / "go-diff" / "testdata" / "speedtest1.txt"
SECOND = ROOT / "tests" / "original" / "go-diff" / "testdata" / "speedtest2.txt"
GO = ROOT / "bench" / "bin" / "go-bench-driver"
RUST = ROOT / "target" / "release" / "bench-driver"
SAMPLES = int(os.environ.get("GO_DIFF_BENCH_SAMPLES", "50"))
STARTUP_SAMPLES = int(os.environ.get("GO_DIFF_STARTUP_SAMPLES", "30"))


def command_output(command: list[str]) -> str:
    return subprocess.check_output(command, text=True).strip()


def parse_batch(output: str) -> tuple[dict[str, str], list[int]]:
    fields = dict(line.split("=", 1) for line in output.splitlines())
    samples = [int(value) for value in fields.pop("samples_ns").split(",")]
    return fields, samples


def percentile(values: list[int], percentage: float) -> int:
    ordered = sorted(values)
    rank = max(0, math.ceil(percentage / 100 * len(ordered)) - 1)
    return ordered[rank]


def distribution(values: list[int]) -> dict[str, float | int]:
    return {
        "samples": len(values),
        "min": min(values),
        "p50": percentile(values, 50),
        "p95": percentile(values, 95),
        "p99": percentile(values, 99),
        "max": max(values),
        "mean": round(statistics.fmean(values), 1),
    }


def startup_samples(binary: Path) -> list[int]:
    values = []
    for _ in range(STARTUP_SAMPLES):
        start = time.perf_counter_ns()
        subprocess.run(
            [str(binary), "batch", str(FIRST), str(SECOND), "1"],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        values.append(time.perf_counter_ns() - start)
    return values


def max_rss_kib(binary: Path, label: str) -> int:
    output = RAW / f"{label}-rss-kib.txt"
    subprocess.run(
        [
            "/usr/bin/time", "-f", "%M", "-o", str(output), "--",
            str(binary), "batch", str(FIRST), str(SECOND), "10",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return int(output.read_text().strip())


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def benchmark(binary: Path, label: str) -> dict[str, object]:
    output = command_output([
        str(binary), "batch", str(FIRST), str(SECOND), str(SAMPLES)
    ])
    fields, latency = parse_batch(output)
    startup = startup_samples(binary)
    RAW.joinpath(f"{label}-latency-ns.txt").write_text(
        "\n".join(map(str, latency)) + "\n"
    )
    RAW.joinpath(f"{label}-startup-ns.txt").write_text(
        "\n".join(map(str, startup)) + "\n"
    )
    return {
        "output_shape": {key: int(value) for key, value in fields.items()},
        "latency_ns": distribution(latency),
        "startup_ns": distribution(startup),
        "throughput_ops_per_second": round(1_000_000_000 / statistics.fmean(latency), 3),
        "max_rss_kib": max_rss_kib(binary, label),
        "binary_sha256": sha256(binary),
    }


def main() -> None:
    RAW.mkdir(parents=True, exist_ok=True)
    results = {
        "schema_version": 1,
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "upstream_commit": "57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586",
        "host": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "processor": platform.processor(),
            "cpu_count": os.cpu_count(),
        },
        "toolchains": {
            "go": command_output(["go", "version"]),
            "rustc": command_output(["rustc", "--version"]),
        },
        "workload": {
            "name": "upstream speedtest pair / DiffMain(check_lines=true)",
            "first_bytes": FIRST.stat().st_size,
            "second_bytes": SECOND.stat().st_size,
            "first_sha256": sha256(FIRST),
            "second_sha256": sha256(SECOND),
            "latency_samples": SAMPLES,
            "startup_samples": STARTUP_SAMPLES,
            "rss_batch_iterations": 10,
        },
        "original_go": benchmark(GO, "go"),
        "port_rust": benchmark(RUST, "rust"),
    }
    go = results["original_go"]
    rust = results["port_rust"]
    assert isinstance(go, dict) and isinstance(rust, dict)
    results["rust_over_go_ratio"] = {
        "latency_p99": round(
            rust["latency_ns"]["p99"] / go["latency_ns"]["p99"], 4
        ),
        "startup_p99": round(
            rust["startup_ns"]["p99"] / go["startup_ns"]["p99"], 4
        ),
        "max_rss": round(rust["max_rss_kib"] / go["max_rss_kib"], 4),
        "throughput": round(
            rust["throughput_ops_per_second"] / go["throughput_ops_per_second"], 4
        ),
    }
    destination = ROOT / "bench" / "results.json"
    destination.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")
    print(destination)
    print(json.dumps(results["rust_over_go_ratio"], indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
