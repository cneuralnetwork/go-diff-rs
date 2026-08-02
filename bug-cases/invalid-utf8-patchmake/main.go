package main

import (
	"fmt"
	"unicode/utf8"

	"github.com/sergi/go-diff/diffmatchpatch"
)

func main() {
	input := string([]byte{0xe0})
	fmt.Printf("input=%q bytes=%x valid_utf8=%v\n", input, []byte(input), utf8.ValidString(input))
	diffmatchpatch.New().PatchMake(input, "")
}
