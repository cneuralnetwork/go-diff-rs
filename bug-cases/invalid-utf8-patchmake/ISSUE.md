# PatchMake panics for a one-byte invalid UTF-8 string

## Version

Reproduced with `github.com/sergi/go-diff` v1.4.0
(`57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`) on Go 1.24.10.

## Minimal reproducer

```go
package main

import "github.com/sergi/go-diff/diffmatchpatch"

func main() {
    input := string([]byte{0xe0})
    diffmatchpatch.New().PatchMake(input, "")
}
```

## Observed result

```text
panic: runtime error: slice bounds out of range [3:1]
```

The top relevant frame is `diffmatchpatch.(*DiffMatchPatch).patchMake2` at
`patch.go:171`.

## Root cause

`DiffMain` converts the input string to `[]rune`, so the invalid byte becomes
U+FFFD. The resulting delete diff contains the three-byte UTF-8 encoding of
U+FFFD. `patchMake2` keeps the original one-byte `postpatchText`, then slices it
using the three-byte diff length at line 171.

Issue #21 established the library's replacement behavior for invalid UTF-8 in
diffs, but searches found no existing issue for this `PatchMake` panic.

## Expected behavior

`PatchMake` should not panic for a value accepted by its public `string` API.
Normalizing the source consistently with the generated diffs (or otherwise
returning/documenting a controlled result) would avoid the out-of-bounds slice.
A regression test with `string([]byte{0xe0})` covers the minimum case.
