#!/usr/bin/env python3
"""Combine Go statement and Rust source-coverage evidence without conflating them."""

from __future__ import annotations

import datetime as dt
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
COVERAGE = ROOT / "coverage"
LIBRARY_FILES = {"diff.rs", "match_impl.rs", "patch.rs", "text.rs", "types.rs", "util.rs"}
ALGORITHM_FILES = {"diff.rs", "match_impl.rs", "patch.rs"}


def ratio(covered: int, count: int) -> float:
    return round(100 * covered / count, 4) if count else 0.0


def aggregate(files: list[dict], selected: set[str]) -> dict[str, dict[str, float | int]]:
    output = {}
    for metric in ["functions", "lines", "regions"]:
        covered = sum(
            item["summary"][metric]["covered"]
            for item in files
            if Path(item["filename"]).name in selected
        )
        count = sum(
            item["summary"][metric]["count"]
            for item in files
            if Path(item["filename"]).name in selected
        )
        output[metric] = {"covered": covered, "count": count, "percent": ratio(covered, count)}
    return output


def go_statements() -> tuple[dict[str, dict[str, float | int]], dict[str, float | int]]:
    per_file: dict[str, list[int]] = {}
    for line in (COVERAGE / "go.out").read_text(encoding="utf-8").splitlines()[1:]:
        location, statements, executions = line.rsplit(" ", 2)
        filename = location.split(":", 1)[0]
        bucket = per_file.setdefault(filename, [0, 0])
        count = int(statements)
        bucket[1] += count
        if int(executions) > 0:
            bucket[0] += count
    report = {
        filename: {
            "covered": values[0],
            "count": values[1],
            "percent": ratio(values[0], values[1]),
        }
        for filename, values in sorted(per_file.items())
    }
    covered = sum(value["covered"] for value in report.values())
    count = sum(value["count"] for value in report.values())
    return report, {"covered": covered, "count": count, "percent": ratio(covered, count)}


def main() -> None:
    rust = json.loads((COVERAGE / "rust.json").read_text(encoding="utf-8"))
    rust_data = rust["data"][0]
    files = rust_data["files"]
    go_files, go_total = go_statements()
    library = aggregate(files, LIBRARY_FILES)
    algorithms = aggregate(files, ALGORITHM_FILES)
    report = {
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "upstream_commit": "57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586",
        "warning": (
            "Go statement coverage and Rust LLVM line/region coverage use different models; "
            "the percentage-point delta is disclosed but is not an equivalence claim."
        ),
        "go_original": {
            "model": "go cover statement blocks",
            "total": go_total,
            "files": go_files,
            "raw_profile": "coverage/go.out",
            "function_report": "coverage/go-functions.txt",
        },
        "rust_port": {
            "model": "LLVM source-based coverage",
            "library": library,
            "algorithm_modules": algorithms,
            "whole_instrumented_targets": rust_data["totals"],
            "files": {
                Path(item["filename"]).name: item["summary"]
                for item in files
                if "/src/" in item["filename"]
            },
            "raw_profile": "coverage/rust.json",
            "cargo_llvm_cov_version": rust["cargo_llvm_cov"]["version"],
        },
        "non_comparable_percentage_point_delta": round(
            library["lines"]["percent"] - go_total["percent"], 4
        ),
    }
    destination = COVERAGE / "report.json"
    destination.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(destination)
    print(
        f"Go statements: {go_total['percent']}%; "
        f"Rust library lines: {library['lines']['percent']}%; "
        f"Rust algorithm lines: {algorithms['lines']['percent']}%"
    )


if __name__ == "__main__":
    main()
