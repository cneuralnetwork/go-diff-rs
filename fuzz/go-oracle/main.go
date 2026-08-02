package main

import (
	"bufio"
	"encoding/hex"
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/sergi/go-diff/diffmatchpatch"
)

func main() {
	scanner := bufio.NewScanner(os.Stdin)
	scanner.Buffer(make([]byte, 64*1024), 16*1024*1024)
	writer := bufio.NewWriter(os.Stdout)
	defer writer.Flush()
	for scanner.Scan() {
		response, err := handle(scanner.Text())
		if err != nil {
			response = "ERR\t" + err.Error()
		}
		fmt.Fprintln(writer, response)
		writer.Flush()
	}
	if err := scanner.Err(); err != nil {
		fmt.Fprintln(writer, "ERR\t"+err.Error())
	}
}

func handle(line string) (string, error) {
	fields := strings.Split(line, "\t")
	dmp := diffmatchpatch.New()
	switch {
	case len(fields) == 4 && fields[0] == "D":
		first, err := hex.DecodeString(fields[2])
		if err != nil {
			return "", err
		}
		second, err := hex.DecodeString(fields[3])
		if err != nil {
			return "", err
		}
		diffs := dmp.DiffMain(string(first), string(second), fields[1] == "1")
		delta := dmp.DiffToDelta(diffs)
		roundtrip, err := dmp.DiffFromDelta(dmp.DiffText1(diffs), delta)
		if err != nil {
			return "", err
		}
		semantic := dmp.DiffCleanupSemantic(append([]diffmatchpatch.Diff(nil), diffs...))
		lossless := dmp.DiffCleanupSemanticLossless(append([]diffmatchpatch.Diff(nil), diffs...))
		efficient := dmp.DiffCleanupEfficiency(append([]diffmatchpatch.Diff(nil), semantic...))
		return fmt.Sprintf("D\t%s\t%x\t%x\t%x\t%d\t%x\t%x\t%s\t%s\t%s\t%s",
			canonicalDiffs(diffs), []byte(delta), []byte(dmp.DiffPrettyHtml(diffs)),
			[]byte(dmp.DiffPrettyText(diffs)), dmp.DiffLevenshtein(diffs),
			[]byte(dmp.DiffText1(diffs)), []byte(dmp.DiffText2(diffs)),
			canonicalDiffs(roundtrip), canonicalDiffs(lossless), canonicalDiffs(semantic), canonicalDiffs(efficient)), nil
	case len(fields) == 4 && fields[0] == "M":
		text, err := hex.DecodeString(fields[1])
		if err != nil {
			return "", err
		}
		pattern, err := hex.DecodeString(fields[2])
		if err != nil {
			return "", err
		}
		location, err := strconv.Atoi(fields[3])
		if err != nil {
			return "", err
		}
		return fmt.Sprintf("M\t%d", dmp.MatchMain(string(text), string(pattern), location)), nil
	case len(fields) == 4 && fields[0] == "P":
		first, err := hex.DecodeString(fields[1])
		if err != nil {
			return "", err
		}
		second, err := hex.DecodeString(fields[2])
		if err != nil {
			return "", err
		}
		target, err := hex.DecodeString(fields[3])
		if err != nil {
			return "", err
		}
		patchText := dmp.PatchToText(dmp.PatchMake(string(first), string(second)))
		parsed, err := dmp.PatchFromText(patchText)
		if err != nil {
			return "", err
		}
		result, applied := dmp.PatchApply(parsed, string(target))
		flags := strings.Builder{}
		for _, flag := range applied {
			if flag {
				flags.WriteByte('1')
			} else {
				flags.WriteByte('0')
			}
		}
		return fmt.Sprintf("P\t%x\t%x\t%s", []byte(patchText), []byte(result), flags.String()), nil
	default:
		return "", fmt.Errorf("invalid request")
	}
}

func canonicalDiffs(diffs []diffmatchpatch.Diff) string {
	parts := make([]string, 0, len(diffs))
	for _, diff := range diffs {
		parts = append(parts, fmt.Sprintf("%d:%x", int8(diff.Type), []byte(diff.Text)))
	}
	return strings.Join(parts, "|")
}
