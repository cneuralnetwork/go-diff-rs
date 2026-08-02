# Differential fuzzing

`harness.py` drives persistent native Go and Rust oracles with the same
hex-encoded requests. It compares exact diff sequences, delta encoding and
round-trips, lossless/semantic/efficiency cleanup sequences, HTML/text
renderings, Levenshtein distance, fuzzy match locations, patch
serialization/parsing, patch output, and application flags.

The fixed corpus includes empty strings, NULs, CRLF, Unicode, HTML metacharacters,
percent escapes, and invalid UTF-8. A seeded generator then produces valid and
arbitrary byte strings plus long line-mode cases. A mismatch is greedily shrunk
and written into `log.txt` as a directly replayable protocol request.

Run the scored session with:

```sh
make fuzz
```

This runs for at least 60 continuous seconds. `make fuzz-smoke` is a shorter
preflight. Go is strictly a reference oracle here; it is never linked to or
invoked by the shipped Rust library or CLI.

The committed survivor log records 124,772 requests over 60.002 continuous
seconds with zero divergences. Invalid-UTF-8 `PatchMake` is the one disclosed
reference crash excluded from that persistent stream; its minimized reproducer
is still covered as an explicit compatibility regression.
