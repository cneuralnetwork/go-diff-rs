use crate::{DIFF_DELETE, DIFF_EQUAL, Diff, DiffMatchPatch};

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
