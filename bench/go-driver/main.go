package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/sergi/go-diff/diffmatchpatch"
)

func main() {
	if len(os.Args) != 5 || os.Args[1] != "batch" {
		fmt.Fprintln(os.Stderr, "usage: bench-driver batch INPUT1 INPUT2 ITERATIONS")
		os.Exit(2)
	}
	first, err := os.ReadFile(os.Args[2])
	if err != nil {
		panic(err)
	}
	second, err := os.ReadFile(os.Args[3])
	if err != nil {
		panic(err)
	}
	iterations, err := strconv.Atoi(os.Args[4])
	if err != nil {
		panic(err)
	}
	dmp := diffmatchpatch.New()
	samples := make([]string, 0, iterations)
	var last []diffmatchpatch.Diff
	for i := 0; i < iterations; i++ {
		start := time.Now()
		last = dmp.DiffMain(string(first), string(second), true)
		samples = append(samples, strconv.FormatInt(time.Since(start).Nanoseconds(), 10))
	}
	fmt.Printf("diffs=%d\n", len(last))
	fmt.Printf("text1_bytes=%d\n", len(dmp.DiffText1(last)))
	fmt.Printf("text2_bytes=%d\n", len(dmp.DiffText2(last)))
	fmt.Printf("samples_ns=%s\n", strings.Join(samples, ","))
}
