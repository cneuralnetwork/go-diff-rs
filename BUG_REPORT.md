# Bug Catcher findings

## Finding 1: `PatchMake` panics on invalid UTF-8

## Status

Reproduced against the pinned upstream v1.4.0 commit and filed as
[sergi/go-diff#157](https://github.com/sergi/go-diff/issues/157) on 2026-08-02.
The duplicate audit found closed issue #21 about DiffMain's replacement policy
and closed issues #31/#127 for different valid-text `PatchMake` panics; none
covers this invalid-byte length expansion. The connected GitHub integration's
write attempt returned HTTP 403, so the report was filed through the owner's
authenticated GitHub CLI session and then verified live.

A focused fix is submitted as draft
[sergi/go-diff#158](https://github.com/sergi/go-diff/pull/158), commit
`b3ae7e790c4ce6bf423549807848b2c1c2fec5a0`. It normalizes the source only when
it exactly matches `DiffMain`'s reconstructed U+FFFD form, preserves handcrafted
raw-byte diffs, and adds regression coverage for both affected `PatchMake`
entry points. `go test ./...`, the uncached race suite, and `go vet ./...` pass.

## Minimal reproducer

```go
input := string([]byte{0xe0})
diffmatchpatch.New().PatchMake(input, "")
```

Run the self-contained pinned reproducer:

```sh
mkdir -p target/go-cache
GOCACHE="$PWD/target/go-cache" go -C bug-cases/invalid-utf8-patchmake run .
```

Observed result:

```text
input="\xe0" bytes=e0 valid_utf8=false
panic: runtime error: slice bounds out of range [3:1]
```

## Root cause

`DiffMain` intentionally converts a Go string to `[]rune`; each invalid input
byte becomes U+FFFD. The delete diff for one invalid byte therefore contains a
replacement rune encoded as three UTF-8 bytes. `patchMake2` retains the
original one-byte `postpatchText`, advances/slices it using the three-byte diff
length, and panics at `patch.go:171`.

The crash violates the otherwise documented invalid-UTF-8 replacement behavior
and makes an exported API unsafe for arbitrary Go strings. The published
reproducer is the one-byte minimum obtained by shrinking the original `e0e5`
case found by the comparison campaign.

## Compatibility decision

The Rust port's default v1.4.0 profile intentionally preserves the panic; the
regression is explicit in `tests/port/bug_regressions.rs` and recorded in
`DECISIONS.md`. It is excluded only from the survivor stream because a crashing
reference process cannot complete a persistent differential session. Invalid
UTF-8 remains covered for DiffMain and MatchMain. The upstream PR does not alter
the port's pinned v1.4.0 compatibility target.

## Finding 2: semantic-lossless scoring never uses its start expression

The first full comparison attempt found a valid UTF-8/CRLF patch whose applied
bytes and flags agreed but whose serialized equality boundaries differed. The
shrinker reduced the relevant cleanup input to:

```go
[]diffmatchpatch.Diff{
    {Type: diffmatchpatch.DiffEqual, Text: "\r\n"},
    {Type: diffmatchpatch.DiffDelete, Text: "\n"},
    {Type: diffmatchpatch.DiffEqual, Text: "\r\n%"},
}
```

`diff.go` declares `blanklineStartRegex`, but
`diffCleanupSemanticScore` tests `blanklineEndRegex` for both `one` and `two`.
Consequently the start expression is unused and the cleanup keeps the input
boundaries. The Rust port now preserves that exact result. A standalone
reproducer and ready-to-paste issue are in
`bug-cases/semantic-lossless-blankline-start/`.
