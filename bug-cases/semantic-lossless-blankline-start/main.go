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
