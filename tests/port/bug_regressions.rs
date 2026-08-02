use crate::util::int_to_rune;
use crate::{DIFF_DELETE, DIFF_EQUAL, Diff, DiffMatchPatch};
use std::fmt::Write as _;

#[test]
fn invalid_utf8_patch_make_preserves_upstream_panic() {
    let dmp = DiffMatchPatch::new();
    let panic = std::panic::catch_unwind(|| dmp.patch_make(b"\xe0", b""));
    assert!(
        panic.is_err(),
        "the v1.4.0 compatibility profile preserves this panic"
    );
}

#[test]
fn semantic_lossless_preserves_upstream_crlf_boundary_scoring() {
    let dmp = DiffMatchPatch::new();
    let input = vec![
        Diff::new(DIFF_EQUAL, "\r\n"),
        Diff::new(DIFF_DELETE, "\n"),
        Diff::new(DIFF_EQUAL, "\r\n%"),
    ];

    // go-diff v1.4.0 declares a blank-line-start expression but uses its
    // blank-line-end expression on both sides. That makes the original
    // arrangement win the tie and is observable in PatchToText output.
    assert_eq!(dmp.diff_cleanup_semantic_lossless(input.clone()), input);
}

#[test]
fn large_line_identifiers_roundtrip_through_both_public_apis() {
    const LINE_COUNT: u32 = 0xF800;

    let mut source = String::new();
    for line in 1..=LINE_COUNT {
        writeln!(source, "{line}").expect("write unique line fixture");
    }

    let dmp = DiffMatchPatch::new();
    let expected_last_rune = int_to_rune(LINE_COUNT);
    assert_eq!(expected_last_rune, 0x10003);

    let (chars, empty_chars, char_lines) = dmp.diff_lines_to_chars(&source, "");
    assert_eq!(
        chars.as_str().unwrap().chars().last().map(u32::from),
        Some(expected_last_rune)
    );
    let char_diffs = dmp.diff_main(&chars, &empty_chars, false);
    let char_hydrated = dmp.diff_chars_to_lines(&char_diffs, &char_lines);
    assert_eq!(dmp.diff_text1(&char_hydrated), source.as_str());
    assert!(dmp.diff_text2(&char_hydrated).is_empty());

    let (runes, empty_runes, rune_lines) = dmp.diff_lines_to_runes(&source, "");
    assert_eq!(runes.last(), Some(&expected_last_rune));
    let rune_diffs = dmp.diff_main_runes(&runes, &empty_runes, false);
    let rune_hydrated = dmp.diff_chars_to_lines(&rune_diffs, &rune_lines);
    assert_eq!(dmp.diff_text1(&rune_hydrated), source.as_str());
    assert!(dmp.diff_text2(&rune_hydrated).is_empty());
}
