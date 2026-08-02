# DiffCleanupSemanticLossless never uses blanklineStartRegex

## Version

Reproduced with `github.com/sergi/go-diff` v1.4.0
(`57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`) on Go 1.24.10.

## Minimal reproducer

```go
package main

import (
    "fmt"
    "github.com/sergi/go-diff/diffmatchpatch"
)

func main() {
    diffs := []diffmatchpatch.Diff{
        {Type: diffmatchpatch.DiffEqual, Text: "\r\n"},
        {Type: diffmatchpatch.DiffDelete, Text: "\n"},
        {Type: diffmatchpatch.DiffEqual, Text: "\r\n%"},
    }
    fmt.Printf("%#v\n", diffmatchpatch.New().DiffCleanupSemanticLossless(diffs))
}
```

## Observed result

The cleanup keeps the three boundaries unchanged. During a Go/Rust differential
campaign this also produced a different, though application-equivalent, patch
serialization around a valid Unicode/CRLF input.

## Root cause

`diff.go` declares both `blanklineEndRegex` and `blanklineStartRegex`, but
`diffCleanupSemanticScore` contains:

```go
blankLine1 := lineBreak1 && blanklineEndRegex.MatchString(one)
blankLine2 := lineBreak2 && blanklineEndRegex.MatchString(two)
```

The second line appears intended to use `blanklineStartRegex`. As written, that
start expression is never referenced anywhere in the package. In the minimized
case, using it changes the boundary score and shifts the lone LF left as the
algorithm's comments describe.

## Suggested fix

Use `blanklineStartRegex.MatchString(two)` for `blankLine2` and add the minimized
case as a regression. This is output-affecting for semantic cleanup and patch
serialization, so it may warrant a release-note compatibility warning.
