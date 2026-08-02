#!/usr/bin/env python3
"""Verify one-to-one source/Rust test-function mapping and emit pass rates."""

from __future__ import annotations

import datetime as dt
import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = ROOT / "tests" / "original" / "go-diff" / "diffmatchpatch"
PORT = ROOT / "tests" / "port"
FILES = ["diff", "index", "match", "patch", "stringutil"]


def snake_case(name: str) -> str:
    first = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", first).lower()


def names(path: Path, pattern: str) -> list[str]:
    return re.findall(pattern, path.read_text(encoding="utf-8"), re.MULTILINE)


def run_module(module: str) -> tuple[int, str]:
    completed = subprocess.run(
        [
            "cargo", "test", "--lib", f"port_tests::{module}_test::", "--locked",
            "--", "--test-threads=1",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    transcript = completed.stdout + completed.stderr
    match = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed", transcript)
    if match is None:
        raise RuntimeError(f"could not parse {module} test output\n{transcript}")
    passed, failed = map(int, match.groups())
    if completed.returncode != 0 or failed != 0:
        raise RuntimeError(f"{module} parity tests failed\n{transcript}")
    return passed, transcript


def main() -> None:
    reports = []
    total_original = 0
    total_passed = 0
    for stem in FILES:
        go_path = ORIGINAL / f"{stem}_test.go"
        rust_path = PORT / f"{stem}_test.rs"
        go_names = [snake_case(value) for value in names(go_path, r"^func Test(\w+)\(")]
        rust_names = names(rust_path, r"^fn test_(\w+)\(")
        missing = sorted(set(go_names) - set(rust_names))
        extra = sorted(set(rust_names) - set(go_names))
        passed, transcript = run_module(stem)
        expected = len(go_names)
        if missing or extra or passed != expected:
            raise RuntimeError(
                f"{stem}: expected={expected}, passed={passed}, missing={missing}, extra={extra}"
            )
        transcript_path = PORT / f"{stem}_test.transcript.txt"
        transcript_path.write_text(transcript, encoding="utf-8")
        reports.append({
            "original_file": f"diffmatchpatch/{stem}_test.go",
            "rust_file": f"tests/port/{stem}_test.rs",
            "original_test_functions": expected,
            "translated_test_functions": len(rust_names),
            "passed": passed,
            "failed": 0,
            "pass_rate_percent": 100.0,
            "missing_functions": missing,
            "extra_functions": extra,
            "transcript": f"tests/port/{stem}_test.transcript.txt",
        })
        total_original += expected
        total_passed += passed

    report = {
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "upstream_commit": "57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586",
        "original_manifest": "tests/original/SHA256SUMS",
        "original_files_modified": 0,
        "overall": {
            "original_test_functions": total_original,
            "translated_test_functions": total_original,
            "passed": total_passed,
            "failed": 0,
            "pass_rate_percent": 100.0,
        },
        "files": reports,
        "helper_only_original_files": ["diffmatchpatch/benchutil_test.go"],
    }
    destination = PORT / "PARITY_REPORT.json"
    destination.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(destination)
    print(f"translated parity: {total_passed}/{total_original} (100.0%)")


if __name__ == "__main__":
    main()
