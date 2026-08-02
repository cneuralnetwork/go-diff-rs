use super::d;
use crate::{DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, DiffMatchPatch, Patch};

#[test]
fn test_patch_string() {
    let patch = Patch::new(
        vec![
            d(DIFF_EQUAL, "jump"),
            d(DIFF_DELETE, "s"),
            d(DIFF_INSERT, "ed"),
            d(DIFF_EQUAL, " over "),
            d(DIFF_DELETE, "the"),
            d(DIFF_INSERT, "a"),
            d(DIFF_EQUAL, "\nlaz"),
        ],
        20,
        21,
        18,
        17,
    );
    assert_eq!(
        patch.to_string(),
        "@@ -21,18 +22,17 @@\n jump\n-s\n+ed\n  over \n-the\n+a\n %0Alaz\n"
    );
}

#[test]
fn test_patch_from_text() {
    let dmp = DiffMatchPatch::new();
    let cases = [
        ("", None),
        (
            "@@ -21,18 +22,17 @@\n jump\n-s\n+ed\n  over \n-the\n+a\n %0Alaz\n",
            None,
        ),
        ("@@ -1 +1 @@\n-a\n+b\n", None),
        ("@@ -1,3 +0,0 @@\n-abc\n", None),
        ("@@ -0,0 +1,3 @@\n+abc\n", None),
        (
            "@@ _0,0 +0,0 @@\n+abc\n",
            Some("Invalid patch string: @@ _0,0 +0,0 @@"),
        ),
        ("Bad\nPatch\n", Some("Invalid patch string")),
    ];
    for (patch_text, error_prefix) in cases {
        match error_prefix {
            None => {
                let patches = dmp.patch_from_text(patch_text).unwrap();
                if patch_text.is_empty() {
                    assert!(patches.is_empty());
                } else {
                    assert_eq!(patches[0].to_string(), patch_text);
                }
            }
            Some(prefix) => {
                let error = dmp.patch_from_text(patch_text).unwrap_err().to_string();
                assert!(error.starts_with(prefix), "{error:?}");
            }
        }
    }

    let patches = dmp
        .patch_from_text(
            "@@ -1,21 +1,21 @@\n-%601234567890-=%5B%5D%5C;',./\n+~!@#$%25%5E&*()_+%7B%7D%7C:%22%3C%3E?\n",
        )
        .unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(
        patches[0].diffs(),
        [
            d(DIFF_DELETE, "`1234567890-=[]\\;',./"),
            d(DIFF_INSERT, "~!@#$%^&*()_+{}|:\"<>?"),
        ]
    );
}

#[test]
fn test_patch_to_text() {
    let dmp = DiffMatchPatch::new();
    for patch_text in [
        "@@ -21,18 +22,17 @@\n jump\n-s\n+ed\n  over \n-the\n+a\n  laz\n",
        "@@ -1,9 +1,9 @@\n-f\n+F\n oo+fooba\n@@ -7,9 +7,9 @@\n obar\n-,\n+.\n  tes\n",
    ] {
        let patches = dmp.patch_from_text(patch_text).unwrap();
        assert_eq!(dmp.patch_to_text(&patches), patch_text);
    }
}

#[test]
fn test_patch_add_context() {
    let mut dmp = DiffMatchPatch::new();
    dmp.patch_margin = 4;
    let cases = [
        (
            "Simple case",
            "@@ -21,4 +21,10 @@\n-jump\n+somersault\n",
            "The quick brown fox jumps over the lazy dog.",
            "@@ -17,12 +17,18 @@\n fox \n-jump\n+somersault\n s ov\n",
        ),
        (
            "Not enough trailing context",
            "@@ -21,4 +21,10 @@\n-jump\n+somersault\n",
            "The quick brown fox jumps.",
            "@@ -17,10 +17,16 @@\n fox \n-jump\n+somersault\n s.\n",
        ),
        (
            "Not enough leading context",
            "@@ -3 +3,2 @@\n-e\n+at\n",
            "The quick brown fox jumps.",
            "@@ -1,7 +1,8 @@\n Th\n-e\n+at\n  qui\n",
        ),
        (
            "Ambiguity",
            "@@ -3 +3,2 @@\n-e\n+at\n",
            "The quick brown fox jumps.  The quick brown fox crashes.",
            "@@ -1,27 +1,28 @@\n Th\n-e\n+at\n  quick brown fox jumps. \n",
        ),
    ];
    for (name, patch_text, text, expected) in cases {
        let patch = dmp.patch_from_text(patch_text).unwrap().remove(0);
        assert_eq!(
            dmp.patch_add_context(patch, text).to_string(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn test_patch_make_and_patch_to_text() {
    let mut dmp = DiffMatchPatch::new();
    let text1 = "The quick brown fox jumps over the lazy dog.";
    let text2 = "That quick brown fox jumped over a lazy dog.";
    let expected = "@@ -1,11 +1,12 @@\n Th\n-e\n+at\n  quick b\n@@ -22,18 +22,17 @@\n jump\n-s\n+ed\n  over \n-the\n+a\n  laz\n";
    assert!(dmp.patch_make("", "").is_empty());
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make(text2, text1)),
        "@@ -1,8 +1,7 @@\n Th\n-at\n+e\n  qui\n@@ -21,17 +21,18 @@\n jump\n-ed\n+s\n  over \n-a\n+the\n  laz\n"
    );
    assert_eq!(dmp.patch_to_text(&dmp.patch_make(text1, text2)), expected);
    let diffs = dmp.diff_main(text1, text2, false);
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make_from_diffs(&diffs)),
        expected
    );
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make_from_text_and_diffs(text1, &diffs)),
        expected
    );
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make_deprecated(text1, text2, &diffs)),
        expected
    );
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make("`1234567890-=[]\\;',./", "~!@#$%^&*()_+{}|:\"<>?")),
        "@@ -1,21 +1,21 @@\n-%601234567890-=%5B%5D%5C;',./\n+~!@#$%25%5E&*()_+%7B%7D%7C:%22%3C%3E?\n"
    );
    let repeated = "abcdef".repeat(100);
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make(&repeated, format!("{repeated}123"))),
        "@@ -573,28 +573,31 @@\n cdefabcdefabcdefabcdefabcdef\n+123\n"
    );
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make(
            "2016-09-01T03:07:14.807830741Z",
            "2016-09-01T03:07:15.154800781Z"
        )),
        "@@ -15,16 +15,16 @@\n 07:1\n+5.15\n 4\n-.\n 80\n+0\n 78\n-3074\n 1Z\n"
    );

    dmp.diff_timeout = std::time::Duration::ZERO;
    let lorem1 = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vivamus ut risus et enim consectetur convallis a non ipsum. Sed nec nibh cursus, interdum libero vel.";
    let lorem2 = "Lorem a ipsum dolor sit amet, consectetur adipiscing elit. Vivamus ut risus et enim consectetur convallis a non ipsum. Sed nec nibh cursus, interdum liberovel.";
    let diffs = dmp.diff_main(lorem1, lorem2, true);
    assert_eq!(dmp.diff_text1(&diffs), lorem1);
    assert_eq!(dmp.diff_text2(&diffs), lorem2);
    assert_eq!(
        dmp.patch_to_text(&dmp.patch_make_from_text_and_diffs(lorem1, &diffs)),
        "@@ -1,14 +1,16 @@\n Lorem \n+a \n ipsum do\n@@ -148,13 +148,12 @@\n m libero\n- \n vel.\n"
    );
}

#[test]
fn test_patch_split_max() {
    let dmp = DiffMatchPatch::new();
    let cases = [
        (
            "abcdefghijklmnopqrstuvwxyz01234567890",
            "XabXcdXefXghXijXklXmnXopXqrXstXuvXwxXyzX01X23X45X67X89X0",
            "@@ -1,32 +1,46 @@\n+X\n ab\n+X\n cd\n+X\n ef\n+X\n gh\n+X\n ij\n+X\n kl\n+X\n mn\n+X\n op\n+X\n qr\n+X\n st\n+X\n uv\n+X\n wx\n+X\n yz\n+X\n 012345\n@@ -25,13 +39,18 @@\n zX01\n+X\n 23\n+X\n 45\n+X\n 67\n+X\n 89\n+X\n 0\n",
        ),
        (
            "abcdef1234567890123456789012345678901234567890123456789012345678901234567890uvwxyz",
            "abcdefuvwxyz",
            "@@ -3,78 +3,8 @@\n cdef\n-1234567890123456789012345678901234567890123456789012345678901234567890\n uvwx\n",
        ),
        (
            "1234567890123456789012345678901234567890123456789012345678901234567890",
            "abc",
            "@@ -1,32 +1,4 @@\n-1234567890123456789012345678\n 9012\n@@ -29,32 +1,4 @@\n-9012345678901234567890123456\n 7890\n@@ -57,14 +1,3 @@\n-78901234567890\n+abc\n",
        ),
        (
            "abcdefghij , h : 0 , t : 1 abcdefghij , h : 0 , t : 1 abcdefghij , h : 0 , t : 1",
            "abcdefghij , h : 1 , t : 1 abcdefghij , h : 1 , t : 1 abcdefghij , h : 0 , t : 1",
            "@@ -2,32 +2,32 @@\n bcdefghij , h : \n-0\n+1\n  , t : 1 abcdef\n@@ -29,32 +29,32 @@\n bcdefghij , h : \n-0\n+1\n  , t : 1 abcdef\n",
        ),
    ];
    for (text1, text2, expected) in cases {
        let patches = dmp.patch_split_max(dmp.patch_make(text1, text2));
        assert_eq!(dmp.patch_to_text(&patches), expected);
    }
}

#[test]
fn test_patch_add_padding() {
    let dmp = DiffMatchPatch::new();
    let cases = [
        (
            "",
            "test",
            "@@ -0,0 +1,4 @@\n+test\n",
            "@@ -1,8 +1,12 @@\n %01%02%03%04\n+test\n %01%02%03%04\n",
        ),
        (
            "XY",
            "XtestY",
            "@@ -1,2 +1,6 @@\n X\n+test\n Y\n",
            "@@ -2,8 +2,12 @@\n %02%03%04X\n+test\n Y%01%02%03\n",
        ),
        (
            "XXXXYYYY",
            "XXXXtestYYYY",
            "@@ -1,8 +1,12 @@\n XXXX\n+test\n YYYY\n",
            "@@ -5,8 +5,12 @@\n XXXX\n+test\n YYYY\n",
        ),
    ];
    for (text1, text2, expected, padded) in cases {
        let mut patches = dmp.patch_make(text1, text2);
        assert_eq!(dmp.patch_to_text(&patches), expected);
        let _ = dmp.patch_add_padding(&mut patches);
        assert_eq!(dmp.patch_to_text(&patches), padded);
    }
}

#[test]
fn test_patch_apply() {
    let mut dmp = DiffMatchPatch::new();
    dmp.match_distance = 1000;
    dmp.match_threshold = 0.5;
    dmp.patch_delete_threshold = 0.5;
    let cases: &[(&str, &str, &str, &str, &[bool])] = &[
        ("", "", "Hello world.", "Hello world.", &[]),
        (
            "The quick brown fox jumps over the lazy dog.",
            "That quick brown fox jumped over a lazy dog.",
            "The quick brown fox jumps over the lazy dog.",
            "That quick brown fox jumped over a lazy dog.",
            &[true, true],
        ),
        (
            "The quick brown fox jumps over the lazy dog.",
            "That quick brown fox jumped over a lazy dog.",
            "The quick red rabbit jumps over the tired tiger.",
            "That quick red rabbit jumped over a tired tiger.",
            &[true, true],
        ),
        (
            "The quick brown fox jumps over the lazy dog.",
            "That quick brown fox jumped over a lazy dog.",
            "I am the very model of a modern major general.",
            "I am the very model of a modern major general.",
            &[false, false],
        ),
        (
            "x1234567890123456789012345678901234567890123456789012345678901234567890y",
            "xabcy",
            "x123456789012345678901234567890-----++++++++++-----123456789012345678901234567890y",
            "xabcy",
            &[true, true],
        ),
        (
            "x1234567890123456789012345678901234567890123456789012345678901234567890y",
            "xabcy",
            "x12345678901234567890---------------++++++++++---------------12345678901234567890y",
            "xabc12345678901234567890---------------++++++++++---------------12345678901234567890y",
            &[false, true],
        ),
    ];
    for &(text1, text2, base, expected, applied) in cases {
        let actual = dmp.patch_apply(&dmp.patch_make(text1, text2), base);
        assert_eq!(actual.0, expected);
        assert_eq!(actual.1, applied);
    }

    dmp.patch_delete_threshold = 0.6;
    let actual = dmp.patch_apply(
        &dmp.patch_make(
            "x1234567890123456789012345678901234567890123456789012345678901234567890y",
            "xabcy",
        ),
        "x12345678901234567890---------------++++++++++---------------12345678901234567890y",
    );
    assert_eq!(actual.0, "xabcy");
    assert_eq!(actual.1, [true, true]);

    dmp.match_distance = 0;
    dmp.match_threshold = 0.0;
    dmp.patch_delete_threshold = 0.5;
    let actual = dmp.patch_apply(
        &dmp.patch_make(
            "abcdefghijklmnopqrstuvwxyz--------------------1234567890",
            "abcXXXXXXXXXXdefghijklmnopqrstuvwxyz--------------------1234567YYYYYYYYYY890",
        ),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ--------------------1234567890",
    );
    assert_eq!(
        actual.0,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ--------------------1234567YYYYYYYYYY890"
    );
    assert_eq!(actual.1, [false, true]);

    dmp.match_threshold = 0.5;
    dmp.match_distance = 1000;
    for (text1, text2, base, expected, applied) in [
        ("", "test", "", "test", vec![true]),
        (
            "The quick brown fox jumps over the lazy dog.",
            "Woof",
            "The quick brown fox jumps over the lazy dog.",
            "Woof",
            vec![true, true],
        ),
        ("", "test", "", "test", vec![true]),
        ("XY", "XtestY", "XY", "XtestY", vec![true]),
        ("y", "y123", "x", "x123", vec![true]),
    ] {
        let actual = dmp.patch_apply(&dmp.patch_make(text1, text2), base);
        assert_eq!(actual.0, expected);
        assert_eq!(actual.1, applied);
    }
}

#[test]
fn test_patch_make_out_of_range_panic() {
    let text1 = "\n  1111111111111 000000\n  ------------- ------\n  xxxxxxxxxxxxx ------\n  xxxxxxxxxxxxx ------\n  xxxxxxxxxxxxx xxxxxx\n  xxxxxxxxxxxxx ......\n  xxxxxxxxxxxxx 111111\n  xxxxxxxxxxxxx ??????\n  xxxxxxxxxxxxx 333333\n  xxxxxxxxxxxxx 555555\n  xxxxxxxxxx xxxxx\n  xxxxxxxxxx xxxxx\n  xxxxxxxxxx xxxxx\n  xxxxxxxxxx xxxxx\n";
    let text2 = "\n  2222222222222 000000\n  xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    assert_eq!(DiffMatchPatch::new().patch_make(text1, text2).len(), 6);
}
