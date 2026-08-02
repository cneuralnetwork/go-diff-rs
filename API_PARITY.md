# Public API parity

Source: `sergi/go-diff/diffmatchpatch` v1.4.0 at
`57c41f4cb9849a2e83cdbd7644b31e6d7a7e2586`.

Rust uses snake_case and native ownership/error conventions. `Text` preserves
arbitrary Go string bytes while still accepting `&str`, `String`, byte slices,
and byte vectors through `AsRef<[u8]>`/`From`.

| Go API | Rust API | Status |
|---|---|---|
| `New` | `DiffMatchPatch::new` / `Default` | Complete |
| `Operation`, `DiffDelete`, `DiffEqual`, `DiffInsert` | `Operation`, `DIFF_DELETE`, `DIFF_EQUAL`, `DIFF_INSERT` | Complete |
| `Operation.String` | `Display` / `Debug` | Complete |
| `UNICODE_INVALID_RANGE_START`, `UNICODE_INVALID_RANGE_END`, `UNICODE_INVALID_RANGE_DELTA`, `UNICODE_RANGE_MAX` | same uppercase constants | Complete |
| `ONE_BYTE_BITS`, `TWO_BYTE_BITS`, `THREE_BYTE_BITS`, `FOUR_BYTE_BITS` | same uppercase constants | Complete |
| `Diff{Type, Text}` | `Diff{operation, text}` | Complete |
| `Patch{Start1, Start2, Length1, Length2}` | `Patch{start1, start2, length1, length2}` | Complete |
| private `Patch.diffs` | private storage plus `diffs()` / `diffs_mut()` | Complete, accessor added |
| `Patch.String` | `Display` and `Patch::to_text` | Complete |
| `DiffMain` | `diff_main` | Complete |
| `DiffMainRunes` | `diff_main_runes` | Complete |
| `DiffBisect` | `diff_bisect` with `Deadline` | Complete |
| `DiffLinesToChars` | `diff_lines_to_chars` | Complete |
| `DiffLinesToRunes` | `diff_lines_to_runes` | Complete |
| `DiffCharsToLines` | `diff_chars_to_lines` | Complete |
| `DiffCommonPrefix` | `diff_common_prefix` | Complete |
| `DiffCommonSuffix` | `diff_common_suffix` | Complete |
| `DiffCommonOverlap` | `diff_common_overlap` | Complete |
| `DiffHalfMatch` | `diff_half_match` | Complete |
| `DiffCleanupSemantic` | `diff_cleanup_semantic` | Complete |
| `DiffCleanupSemanticLossless` | `diff_cleanup_semantic_lossless` | Complete |
| `DiffCleanupEfficiency` | `diff_cleanup_efficiency` | Complete |
| `DiffCleanupMerge` | `diff_cleanup_merge` | Complete |
| `DiffXIndex` | `diff_x_index` | Complete |
| `DiffPrettyHtml` | `diff_pretty_html` | Complete |
| `DiffPrettyText` | `diff_pretty_text` | Complete |
| `DiffText1` | `diff_text1` | Complete |
| `DiffText2` | `diff_text2` | Complete |
| `DiffLevenshtein` | `diff_levenshtein` | Complete |
| `DiffToDelta` | `diff_to_delta` | Complete |
| `DiffFromDelta` | `diff_from_delta` returning `Result` | Complete |
| `MatchMain` | `match_main` | Complete |
| `MatchBitap` | `match_bitap` | Complete |
| `MatchAlphabet` | `match_alphabet` | Complete |
| variadic `PatchMake` | four typed `patch_make*` entry points | Complete |
| `PatchAddContext` | `patch_add_context` | Complete |
| `PatchDeepCopy` | `patch_deep_copy` | Complete |
| `PatchApply` | `patch_apply` | Complete |
| `PatchAddPadding` | `patch_add_padding` | Complete |
| `PatchSplitMax` | `patch_split_max` | Complete |
| `PatchToText` | `patch_to_text` | Complete |
| `PatchFromText` | `patch_from_text` returning `Result` | Complete |
| seven public configuration fields | seven snake_case public fields | Complete |

There are no omitted exported source symbols. Rust-only additions are `Text`,
`Deadline`, `PatchError`, constructors, byte accessors, and the command-line
artifact.
