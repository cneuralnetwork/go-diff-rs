#!/usr/bin/env python3
"""Deterministic, dependency-free differential fuzzer for the public API."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import os
import platform
import random
import subprocess
import time
from pathlib import Path


SEED = 0x57C41F4C
TEXT_ATOMS = [
    "a", "b", " ", "\n", "\r\n", "\t", "&", "<", "%", "é", "ﬁ", "中",
    "🙂", "\u0680", "\x00",
]


class Oracle:
    def __init__(self, command: str) -> None:
        self.command = command
        self.process = subprocess.Popen(
            [command], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, encoding="ascii",
        )

    def ask(self, request: str) -> str:
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.process.stdin.write(request + "\n")
        self.process.stdin.flush()
        response = self.process.stdout.readline()
        if response == "":
            stderr = ""
            if self.process.stderr is not None:
                stderr = self.process.stderr.read()
            raise RuntimeError(f"oracle exited for {request!r}: {stderr}")
        return response.rstrip("\n")

    def close(self) -> None:
        if self.process.stdin is not None:
            self.process.stdin.close()
        self.process.wait(timeout=5)


def hx(value: bytes) -> str:
    return value.hex()


def valid_text(rng: random.Random, maximum: int = 80) -> bytes:
    return "".join(rng.choice(TEXT_ATOMS) for _ in range(rng.randrange(maximum + 1))).encode()


def arbitrary_text(rng: random.Random, maximum: int = 80) -> bytes:
    if rng.random() < 0.75:
        return valid_text(rng, maximum)
    return rng.randbytes(rng.randrange(maximum + 1))


def random_request(rng: random.Random, case: int) -> str:
    selector = case % 7
    if selector <= 3:
        if case % 97 == 0:
            first = ((b"alpha\n" * 30) + rng.randbytes(12) + (b"omega\n" * 30))
            second = ((b"alpha\n" * 30) + rng.randbytes(12) + (b"omega\n" * 30))
            check = "1"
        else:
            first = arbitrary_text(rng)
            second = arbitrary_text(rng)
            check = str(rng.randrange(2))
        return f"D\t{check}\t{hx(first)}\t{hx(second)}"
    if selector == 4:
        text = arbitrary_text(rng, 160)
        if text and rng.random() < 0.7:
            start = rng.randrange(len(text))
            pattern = text[start : start + rng.randrange(33)]
        else:
            pattern = arbitrary_text(rng, 32)
        pattern = pattern[:32]
        location = rng.randrange(-20, len(text) + 21)
        return f"M\t{hx(text)}\t{hx(pattern)}\t{location}"
    # Invalid UTF-8 is exercised by diff and match requests. PatchMake has a
    # pinned upstream crash for such input; that is isolated under bug-cases/.
    first = valid_text(rng, 100)
    second = valid_text(rng, 100)
    # Apply the generated patch to its exact source. Fuzzy-target examples are
    # already exhaustive in the translated upstream PatchApply table.
    target = first
    return f"P\t{hx(first)}\t{hx(second)}\t{hx(target)}"


def fixed_requests() -> list[str]:
    values = [
        b"", b"abc", b"a\x00b\n", "café 🙂".encode(), b"\xe0\xe5",
        b"A&B <tag> % +", b"one\r\ntwo\r\n", "\u0680\u0681\u0682".encode(),
    ]
    requests: list[str] = []
    for first in values:
        for second in values:
            requests.append(f"D\t0\t{hx(first)}\t{hx(second)}")
            requests.append(f"D\t1\t{hx(first)}\t{hx(second)}")
            try:
                first.decode("utf-8")
                second.decode("utf-8")
            except UnicodeDecodeError:
                pass
            else:
                requests.append(f"P\t{hx(first)}\t{hx(second)}\t{hx(first)}")
    for text in values:
        for pattern in values[:4]:
            requests.append(f"M\t{hx(text)}\t{hx(pattern[:32])}\t0")
    return requests


def compare(go: Oracle, rust: Oracle, request: str) -> tuple[bool, str, str]:
    go_response = go.ask(request)
    rust_response = rust.ask(request)
    return go_response == rust_response, go_response, rust_response


def shrink(go: Oracle, rust: Oracle, request: str) -> str:
    fields = request.split("\t")
    hex_indexes = {"D": [2, 3], "M": [1, 2], "P": [1, 2]}[fields[0]]
    for index in hex_indexes:
        data = bytes.fromhex(fields[index])
        chunk = max(1, len(data) // 2)
        while data and chunk:
            changed = False
            offset = 0
            while offset < len(data):
                candidate = data[:offset] + data[offset + chunk :]
                if fields[0] == "P":
                    try:
                        candidate.decode("utf-8")
                    except UnicodeDecodeError:
                        offset += chunk
                        continue
                candidate_fields = fields.copy()
                candidate_fields[index] = candidate.hex()
                if fields[0] == "P" and index == 1:
                    candidate_fields[3] = candidate.hex()
                candidate_request = "\t".join(candidate_fields)
                same, _, _ = compare(go, rust, candidate_request)
                if not same:
                    data = candidate
                    fields = candidate_fields
                    changed = True
                else:
                    offset += chunk
            if not changed:
                chunk //= 2
        fields[index] = data.hex()
    return "\t".join(fields)


def sha256(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--duration", type=float, default=60.0)
    parser.add_argument("--go", required=True)
    parser.add_argument("--rust", required=True)
    parser.add_argument("--log", default="fuzz/log.txt")
    arguments = parser.parse_args()

    start_wall = dt.datetime.now(dt.timezone.utc)
    start = time.monotonic()
    rng = random.Random(SEED)
    go = Oracle(arguments.go)
    rust = Oracle(arguments.rust)
    cases = 0
    operation_counts = {"D": 0, "M": 0, "P": 0}
    failure: tuple[str, str, str] | None = None
    try:
        for request in fixed_requests():
            same, go_response, rust_response = compare(go, rust, request)
            cases += 1
            operation_counts[request[0]] += 1
            if not same:
                failure = (request, go_response, rust_response)
                break
        deadline = start + arguments.duration
        while failure is None and time.monotonic() < deadline:
            request = random_request(rng, cases)
            same, go_response, rust_response = compare(go, rust, request)
            cases += 1
            operation_counts[request[0]] += 1
            if not same:
                minimal = shrink(go, rust, request)
                _, go_response, rust_response = compare(go, rust, minimal)
                failure = (minimal, go_response, rust_response)
    finally:
        go.close()
        rust.close()

    elapsed = time.monotonic() - start
    end_wall = dt.datetime.now(dt.timezone.utc)
    lines = [
        f"status={'FAIL' if failure else 'PASS'}",
        "upstream_commit=57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586",
        f"seed={SEED}",
        f"requested_duration_seconds={arguments.duration:.3f}",
        f"elapsed_seconds={elapsed:.3f}",
        f"cases={cases}",
        f"diff_requests={operation_counts['D']}",
        f"match_requests={operation_counts['M']}",
        f"patch_requests={operation_counts['P']}",
        f"divergences={1 if failure else 0}",
        "known_reference_crashes_excluded=invalid-utf8 PatchMake (see BUG_REPORT.md)",
        f"started_at_utc={start_wall.isoformat()}",
        f"ended_at_utc={end_wall.isoformat()}",
        f"platform={platform.platform()}",
        f"python={platform.python_version()}",
        f"go_oracle_sha256={sha256(arguments.go)}",
        f"rust_oracle_sha256={sha256(arguments.rust)}",
    ]
    if failure is not None:
        request, go_response, rust_response = failure
        lines.extend([
            f"request={request}",
            f"go_response={go_response}",
            f"rust_response={rust_response}",
        ])
    report = "\n".join(lines) + "\n"
    Path(arguments.log).write_text(report, encoding="utf-8")
    print(report, end="")
    return 1 if failure else 0


if __name__ == "__main__":
    raise SystemExit(main())
