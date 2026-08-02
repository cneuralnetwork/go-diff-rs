# Test-suite adaptations

## Preserved source suite

`tests/original/go-diff/` is an unmodified archive of source tag v1.4.0 at
commit `57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`. `diff -qr` against that commit
is empty. The kickoff test/fixture hashes are in `tests/original/SHA256SUMS`;
the whole archived tree is covered by `SNAPSHOT_SHA256SUMS`.

## Rust translation

All 42 Go `Test*` functions are represented by 42 Rust `#[test]` functions in
`tests/port/`. No source test function, table row, expected value, timeout, or
fixture was intentionally removed. Additional Rust tests cover Go-compatible
invalid-UTF-8 behavior, both upstream bugs discovered by differential testing,
Rust API ergonomics, and concurrency. Two integration tests exercise the
runnable CLI.

The unavoidable mechanical adaptations are:

1. Go identifiers become Rust snake_case identifiers.
2. `string` payloads become byte-preserving `Text` values where needed.
3. Go `[]rune` becomes `Vec<u32>` so invalid Go rune values remain expressible.
4. `assert/testify` calls become standard Rust assertions.
5. Go errors become `Result<_, PatchError>` and error prefixes remain checked.
   When `PatchFromText` fails after complete patches, the error retains them and
   exposes them through `PatchError::partial_patches`.
6. Go `nil` diff slices map to an empty Rust `Vec` in the successful empty-delta case.
7. The zero-argument and variadic `PatchMake(...interface{})` cases call the
   corresponding typed Rust entry points.
8. The intentional out-of-range panic is observed with `catch_unwind`.
9. Fixture paths are rooted with `CARGO_MANIFEST_DIR` instead of depending on
   the process working directory.
10. Time uses `std::time::{Duration, Instant}`; the original 200 ms timeout,
    13 doublings, and 100× upper tolerance are preserved.

The original Go suite remains independently runnable with `make test-original`.
The translated suite that exercises the Rust port runs with `make test`.
