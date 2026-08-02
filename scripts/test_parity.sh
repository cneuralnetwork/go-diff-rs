#!/bin/sh
set -eu

sha256sum --check tests/original/SHA256SUMS
python3 scripts/test_parity.py
