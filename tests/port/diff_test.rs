use super::d;
use crate::util::{common_prefix_runes, common_suffix_runes, indexes_to_string};
use crate::{DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, Deadline, Diff, DiffMatchPatch, Text};
use std::time::{Duration, Instant};

fn rebuild(diffs: &[Diff]) -> [Text; 2] {
    let dmp = DiffMatchPatch::new();
    [dmp.diff_text1(diffs), dmp.diff_text2(diffs)]
}

#[test]
fn test_diff_common_prefix() {
    let dmp = DiffMatchPatch::new();
    for (text1, text2, expected) in [
        ("abc", "xyz", 0),
        ("1234abcdef", "1234xyz", 4),
        ("1234", "1234xyz", 4),
    ] {
        assert_eq!(dmp.diff_common_prefix(text1, text2), expected);
    }
}

#[test]
fn test_common_prefix_length() {
    for (text1, text2, expected) in [
        ("abc", "xyz", 0),
        ("1234abcdef", "1234xyz", 4),
        ("1234", "1234xyz", 4),
    ] {
        let first: Vec<u32> = text1.chars().map(u32::from).collect();
        let second: Vec<u32> = text2.chars().map(u32::from).collect();
        assert_eq!(common_prefix_runes(&first, &second), expected);
    }
}

#[test]
fn test_diff_common_suffix() {
    let dmp = DiffMatchPatch::new();
    for (text1, text2, expected) in [
        ("abc", "xyz", 0),
        ("abcdef1234", "xyz1234", 4),
        ("1234", "xyz1234", 4),
    ] {
        assert_eq!(dmp.diff_common_suffix(text1, text2), expected);
    }
}

#[test]
fn test_common_suffix_length() {
    for (text1, text2, expected) in [
        ("abc", "xyz", 0),
        ("abcdef1234", "xyz1234", 4),
        ("1234", "xyz1234", 4),
        ("123", "a3", 1),
    ] {
        let first: Vec<u32> = text1.chars().map(u32::from).collect();
        let second: Vec<u32> = text2.chars().map(u32::from).collect();
        assert_eq!(common_suffix_runes(&first, &second), expected);
    }
}

#[test]
fn test_diff_common_overlap() {
    let dmp = DiffMatchPatch::new();
    for (text1, text2, expected) in [
        ("", "abcd", 0),
        ("abc", "abcd", 3),
        ("123456", "abcd", 0),
        ("123456xxx", "xxxabcd", 3),
        ("fi", "ﬁi", 0),
    ] {
        assert_eq!(dmp.diff_common_overlap(text1, text2), expected);
    }
}

#[test]
fn test_diff_half_match() {
    let mut dmp = DiffMatchPatch::new();
    dmp.diff_timeout = Duration::from_nanos(1);
    let cases: &[(&str, &str, Option<[&str; 5]>)] = &[
        ("1234567890", "abcdef", None),
        ("12345", "23", None),
        (
            "1234567890",
            "a345678z",
            Some(["12", "90", "a", "z", "345678"]),
        ),
        (
            "a345678z",
            "1234567890",
            Some(["a", "z", "12", "90", "345678"]),
        ),
        (
            "abc56789z",
            "1234567890",
            Some(["abc", "z", "1234", "0", "56789"]),
        ),
        (
            "a23456xyz",
            "1234567890",
            Some(["a", "xyz", "1", "7890", "23456"]),
        ),
        (
            "121231234123451234123121",
            "a1234123451234z",
            Some(["12123", "123121", "a", "z", "1234123451234"]),
        ),
        (
            "x-=-=-=-=-=-=-=-=-=-=-=-=",
            "xx-=-=-=-=-=-=-=",
            Some(["", "-=-=-=-=-=", "x", "", "x-=-=-=-=-=-=-="]),
        ),
        (
            "-=-=-=-=-=-=-=-=-=-=-=-=y",
            "-=-=-=-=-=-=-=yy",
            Some(["-=-=-=-=-=", "", "", "y", "-=-=-=-=-=-=-=y"]),
        ),
        (
            "qHilloHelloHew",
            "xHelloHeHulloy",
            Some(["qHillo", "w", "x", "Hulloy", "HelloHe"]),
        ),
    ];
    for &(text1, text2, expected) in cases {
        let actual = dmp.diff_half_match(text1, text2);
        let expected = expected.map(|parts| parts.map(Text::from));
        assert_eq!(actual, expected);
    }
    dmp.diff_timeout = Duration::ZERO;
    assert_eq!(
        dmp.diff_half_match("qHilloHelloHew", "xHelloHeHulloy"),
        None
    );
}

#[test]
fn test_diff_bisect_split() {
    let dmp = DiffMatchPatch::new();
    let text1: Vec<u32> = "STUV\u{5}WX\u{5}YZ\u{5}[".chars().map(u32::from).collect();
    let text2: Vec<u32> = "WĺĻļ\u{5}YZ\u{5}ĽľĿŀZ".chars().map(u32::from).collect();
    let diffs = dmp.diff_bisect_split(
        &text1,
        &text2,
        7,
        6,
        Deadline::At(Instant::now() + Duration::from_secs(3600)),
    );
    assert!(diffs.iter().all(|diff| diff.text.as_str().is_ok()));
}

#[test]
fn test_diff_lines_to_chars() {
    let dmp = DiffMatchPatch::new();
    type LineCase<'a> = (&'a str, &'a str, &'a [u8], &'a [u8], &'a [&'a str]);
    let cases: &[LineCase<'_>] = &[
        (
            "",
            "alpha\r\nbeta\r\n\r\n\r\n",
            b"",
            b"\x01\x02\x03\x03",
            &["", "alpha\r\n", "beta\r\n", "\r\n"],
        ),
        ("a", "b", b"\x01", b"\x02", &["", "a", "b"]),
        (
            "alpha\nbeta\nalpha",
            "",
            b"\x01\x02\x03",
            b"",
            &["", "alpha\n", "beta\n", "alpha"],
        ),
        (
            "abc\ndefg\n12345\n",
            "abc\ndef\n12345\n678",
            b"\x01\x02\x03",
            b"\x01\x04\x03\x05",
            &["", "abc\n", "defg\n", "12345\n", "def\n", "678"],
        ),
    ];
    for &(text1, text2, expected1, expected2, expected_lines) in cases {
        let (actual1, actual2, lines) = dmp.diff_lines_to_chars(text1, text2);
        assert_eq!(actual1.as_bytes(), expected1);
        assert_eq!(actual2.as_bytes(), expected2);
        assert_eq!(
            lines,
            expected_lines
                .iter()
                .map(|line| Text::from(*line))
                .collect::<Vec<_>>()
        );
    }
    let line_list: Vec<String> = std::iter::once(String::new())
        .chain((1..=300).map(|value| format!("{value}\n")))
        .collect();
    let joined = line_list.concat();
    let expected = indexes_to_string(&(1..=300).collect::<Vec<u32>>());
    let (actual, empty, lines) = dmp.diff_lines_to_chars(joined, "");
    assert_eq!(actual.as_bytes(), expected);
    assert!(empty.is_empty());
    assert_eq!(
        lines,
        line_list
            .iter()
            .map(|line| Text::from(line.as_str()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_diff_chars_to_lines() {
    let dmp = DiffMatchPatch::new();
    let actual = dmp.diff_chars_to_lines(
        &[
            d(DIFF_EQUAL, b"\x01\x02\x01"),
            d(DIFF_INSERT, b"\x02\x01\x02"),
        ],
        &[Text::from(""), Text::from("alpha\n"), Text::from("beta\n")],
    );
    assert_eq!(
        actual,
        [
            d(DIFF_EQUAL, "alpha\nbeta\nalpha\n"),
            d(DIFF_INSERT, "beta\nalpha\nbeta\n"),
        ]
    );
    let line_list: Vec<String> = std::iter::once(String::new())
        .chain((1..=300).map(|value| format!("{value}\n")))
        .collect();
    let lines = line_list
        .iter()
        .map(|line| Text::from(line.as_str()))
        .collect::<Vec<_>>();
    let chars = indexes_to_string(&(1..=300).collect::<Vec<u32>>());
    assert_eq!(
        dmp.diff_chars_to_lines(&[d(DIFF_DELETE, chars)], &lines),
        [d(DIFF_DELETE, line_list.concat())]
    );
}

#[test]
fn test_diff_cleanup_merge() {
    let dmp = DiffMatchPatch::new();
    let cases = vec![
        (vec![], vec![]),
        (
            vec![d(DIFF_EQUAL, "a"), d(DIFF_DELETE, "b"), d(DIFF_INSERT, "c")],
            vec![d(DIFF_EQUAL, "a"), d(DIFF_DELETE, "b"), d(DIFF_INSERT, "c")],
        ),
        (
            vec![d(DIFF_EQUAL, "a"), d(DIFF_EQUAL, "b"), d(DIFF_EQUAL, "c")],
            vec![d(DIFF_EQUAL, "abc")],
        ),
        (
            vec![
                d(DIFF_DELETE, "a"),
                d(DIFF_DELETE, "b"),
                d(DIFF_DELETE, "c"),
            ],
            vec![d(DIFF_DELETE, "abc")],
        ),
        (
            vec![
                d(DIFF_INSERT, "a"),
                d(DIFF_INSERT, "b"),
                d(DIFF_INSERT, "c"),
            ],
            vec![d(DIFF_INSERT, "abc")],
        ),
        (
            vec![
                d(DIFF_DELETE, "a"),
                d(DIFF_INSERT, "b"),
                d(DIFF_DELETE, "c"),
                d(DIFF_INSERT, "d"),
                d(DIFF_EQUAL, "e"),
                d(DIFF_EQUAL, "f"),
            ],
            vec![
                d(DIFF_DELETE, "ac"),
                d(DIFF_INSERT, "bd"),
                d(DIFF_EQUAL, "ef"),
            ],
        ),
        (
            vec![
                d(DIFF_DELETE, "a"),
                d(DIFF_INSERT, "abc"),
                d(DIFF_DELETE, "dc"),
            ],
            vec![
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "d"),
                d(DIFF_INSERT, "b"),
                d(DIFF_EQUAL, "c"),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "x"),
                d(DIFF_DELETE, "a"),
                d(DIFF_INSERT, "abc"),
                d(DIFF_DELETE, "dc"),
                d(DIFF_EQUAL, "y"),
            ],
            vec![
                d(DIFF_EQUAL, "xa"),
                d(DIFF_DELETE, "d"),
                d(DIFF_INSERT, "b"),
                d(DIFF_EQUAL, "cy"),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "x"),
                d(DIFF_DELETE, "ā"),
                d(DIFF_INSERT, "ābc"),
                d(DIFF_DELETE, "dc"),
                d(DIFF_EQUAL, "y"),
            ],
            vec![
                d(DIFF_EQUAL, "xā"),
                d(DIFF_DELETE, "d"),
                d(DIFF_INSERT, "b"),
                d(DIFF_EQUAL, "cy"),
            ],
        ),
        (
            vec![d(DIFF_EQUAL, "a"), d(DIFF_INSERT, "ba"), d(DIFF_EQUAL, "c")],
            vec![d(DIFF_INSERT, "ab"), d(DIFF_EQUAL, "ac")],
        ),
        (
            vec![d(DIFF_EQUAL, "c"), d(DIFF_INSERT, "ab"), d(DIFF_EQUAL, "a")],
            vec![d(DIFF_EQUAL, "ca"), d(DIFF_INSERT, "ba")],
        ),
        (
            vec![
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "b"),
                d(DIFF_EQUAL, "c"),
                d(DIFF_DELETE, "ac"),
                d(DIFF_EQUAL, "x"),
            ],
            vec![d(DIFF_DELETE, "abc"), d(DIFF_EQUAL, "acx")],
        ),
        (
            vec![
                d(DIFF_EQUAL, "x"),
                d(DIFF_DELETE, "ca"),
                d(DIFF_EQUAL, "c"),
                d(DIFF_DELETE, "b"),
                d(DIFF_EQUAL, "a"),
            ],
            vec![d(DIFF_EQUAL, "xca"), d(DIFF_DELETE, "cba")],
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(dmp.diff_cleanup_merge(input), expected);
    }
}

#[test]
fn test_diff_cleanup_semantic_lossless() {
    let dmp = DiffMatchPatch::new();
    let cases = vec![
        (vec![], vec![]),
        (
            vec![
                d(DIFF_EQUAL, "AAA\r\n\r\nBBB"),
                d(DIFF_INSERT, "\r\nDDD\r\n\r\nBBB"),
                d(DIFF_EQUAL, "\r\nEEE"),
            ],
            vec![
                d(DIFF_EQUAL, "AAA\r\n\r\n"),
                d(DIFF_INSERT, "BBB\r\nDDD\r\n\r\n"),
                d(DIFF_EQUAL, "BBB\r\nEEE"),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "AAA\r\nBBB"),
                d(DIFF_INSERT, " DDD\r\nBBB"),
                d(DIFF_EQUAL, " EEE"),
            ],
            vec![
                d(DIFF_EQUAL, "AAA\r\n"),
                d(DIFF_INSERT, "BBB DDD\r\n"),
                d(DIFF_EQUAL, "BBB EEE"),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "The c"),
                d(DIFF_INSERT, "ow and the c"),
                d(DIFF_EQUAL, "at."),
            ],
            vec![
                d(DIFF_EQUAL, "The "),
                d(DIFF_INSERT, "cow and the "),
                d(DIFF_EQUAL, "cat."),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "The-c"),
                d(DIFF_INSERT, "ow-and-the-c"),
                d(DIFF_EQUAL, "at."),
            ],
            vec![
                d(DIFF_EQUAL, "The-"),
                d(DIFF_INSERT, "cow-and-the-"),
                d(DIFF_EQUAL, "cat."),
            ],
        ),
        (
            vec![d(DIFF_EQUAL, "a"), d(DIFF_DELETE, "a"), d(DIFF_EQUAL, "ax")],
            vec![d(DIFF_DELETE, "a"), d(DIFF_EQUAL, "aax")],
        ),
        (
            vec![d(DIFF_EQUAL, "xa"), d(DIFF_DELETE, "a"), d(DIFF_EQUAL, "a")],
            vec![d(DIFF_EQUAL, "xaa"), d(DIFF_DELETE, "a")],
        ),
        (
            vec![
                d(DIFF_EQUAL, "The xxx. The "),
                d(DIFF_INSERT, "zzz. The "),
                d(DIFF_EQUAL, "yyy."),
            ],
            vec![
                d(DIFF_EQUAL, "The xxx."),
                d(DIFF_INSERT, " The zzz."),
                d(DIFF_EQUAL, " The yyy."),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "The ♕. The "),
                d(DIFF_INSERT, "♔. The "),
                d(DIFF_EQUAL, "♖."),
            ],
            vec![
                d(DIFF_EQUAL, "The ♕."),
                d(DIFF_INSERT, " The ♔."),
                d(DIFF_EQUAL, " The ♖."),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "♕♕"),
                d(DIFF_INSERT, "♔♔"),
                d(DIFF_EQUAL, "♖♖"),
            ],
            vec![
                d(DIFF_EQUAL, "♕♕"),
                d(DIFF_INSERT, "♔♔"),
                d(DIFF_EQUAL, "♖♖"),
            ],
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(dmp.diff_cleanup_semantic_lossless(input), expected);
    }
}

#[test]
fn test_diff_cleanup_semantic() {
    let dmp = DiffMatchPatch::new();
    let cases = vec![
        (vec![], vec![]),
        (
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_INSERT, "cd"),
                d(DIFF_EQUAL, "12"),
                d(DIFF_DELETE, "e"),
            ],
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_INSERT, "cd"),
                d(DIFF_EQUAL, "12"),
                d(DIFF_DELETE, "e"),
            ],
        ),
        (
            vec![
                d(DIFF_DELETE, "abc"),
                d(DIFF_INSERT, "ABC"),
                d(DIFF_EQUAL, "1234"),
                d(DIFF_DELETE, "wxyz"),
            ],
            vec![
                d(DIFF_DELETE, "abc"),
                d(DIFF_INSERT, "ABC"),
                d(DIFF_EQUAL, "1234"),
                d(DIFF_DELETE, "wxyz"),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "2016-09-01T03:07:1"),
                d(DIFF_INSERT, "5.15"),
                d(DIFF_EQUAL, "4"),
                d(DIFF_DELETE, "."),
                d(DIFF_EQUAL, "80"),
                d(DIFF_INSERT, "0"),
                d(DIFF_EQUAL, "78"),
                d(DIFF_DELETE, "3074"),
                d(DIFF_EQUAL, "1Z"),
            ],
            vec![
                d(DIFF_EQUAL, "2016-09-01T03:07:1"),
                d(DIFF_INSERT, "5.15"),
                d(DIFF_EQUAL, "4"),
                d(DIFF_DELETE, "."),
                d(DIFF_EQUAL, "80"),
                d(DIFF_INSERT, "0"),
                d(DIFF_EQUAL, "78"),
                d(DIFF_DELETE, "3074"),
                d(DIFF_EQUAL, "1Z"),
            ],
        ),
        (
            vec![d(DIFF_DELETE, "a"), d(DIFF_EQUAL, "b"), d(DIFF_DELETE, "c")],
            vec![d(DIFF_DELETE, "abc"), d(DIFF_INSERT, "b")],
        ),
        (
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_EQUAL, "cd"),
                d(DIFF_DELETE, "e"),
                d(DIFF_EQUAL, "f"),
                d(DIFF_INSERT, "g"),
            ],
            vec![d(DIFF_DELETE, "abcdef"), d(DIFF_INSERT, "cdfg")],
        ),
        (
            vec![
                d(DIFF_INSERT, "1"),
                d(DIFF_EQUAL, "A"),
                d(DIFF_DELETE, "B"),
                d(DIFF_INSERT, "2"),
                d(DIFF_EQUAL, "_"),
                d(DIFF_INSERT, "1"),
                d(DIFF_EQUAL, "A"),
                d(DIFF_DELETE, "B"),
                d(DIFF_INSERT, "2"),
            ],
            vec![d(DIFF_DELETE, "AB_AB"), d(DIFF_INSERT, "1A2_1A2")],
        ),
        (
            vec![
                d(DIFF_EQUAL, "The c"),
                d(DIFF_DELETE, "ow and the c"),
                d(DIFF_EQUAL, "at."),
            ],
            vec![
                d(DIFF_EQUAL, "The "),
                d(DIFF_DELETE, "cow and the "),
                d(DIFF_EQUAL, "cat."),
            ],
        ),
        (
            vec![d(DIFF_DELETE, "abcxx"), d(DIFF_INSERT, "xxdef")],
            vec![d(DIFF_DELETE, "abcxx"), d(DIFF_INSERT, "xxdef")],
        ),
        (
            vec![d(DIFF_DELETE, "abcxxx"), d(DIFF_INSERT, "xxxdef")],
            vec![
                d(DIFF_DELETE, "abc"),
                d(DIFF_EQUAL, "xxx"),
                d(DIFF_INSERT, "def"),
            ],
        ),
        (
            vec![d(DIFF_DELETE, "xxxabc"), d(DIFF_INSERT, "defxxx")],
            vec![
                d(DIFF_INSERT, "def"),
                d(DIFF_EQUAL, "xxx"),
                d(DIFF_DELETE, "abc"),
            ],
        ),
        (
            vec![
                d(DIFF_DELETE, "abcd1212"),
                d(DIFF_INSERT, "1212efghi"),
                d(DIFF_EQUAL, "----"),
                d(DIFF_DELETE, "A3"),
                d(DIFF_INSERT, "3BC"),
            ],
            vec![
                d(DIFF_DELETE, "abcd"),
                d(DIFF_EQUAL, "1212"),
                d(DIFF_INSERT, "efghi"),
                d(DIFF_EQUAL, "----"),
                d(DIFF_DELETE, "A"),
                d(DIFF_EQUAL, "3"),
                d(DIFF_INSERT, "BC"),
            ],
        ),
        (
            vec![
                d(DIFF_EQUAL, "James McCarthy "),
                d(DIFF_DELETE, "close to "),
                d(DIFF_EQUAL, "sign"),
                d(DIFF_DELETE, "ing"),
                d(DIFF_INSERT, "s"),
                d(DIFF_EQUAL, " new "),
                d(DIFF_DELETE, "E"),
                d(DIFF_INSERT, "fi"),
                d(DIFF_EQUAL, "ve"),
                d(DIFF_INSERT, "-yea"),
                d(DIFF_EQUAL, "r"),
                d(DIFF_DELETE, "ton"),
                d(DIFF_EQUAL, " deal"),
                d(DIFF_INSERT, " at Everton"),
            ],
            vec![
                d(DIFF_EQUAL, "James McCarthy "),
                d(DIFF_DELETE, "close to "),
                d(DIFF_EQUAL, "sign"),
                d(DIFF_DELETE, "ing"),
                d(DIFF_INSERT, "s"),
                d(DIFF_EQUAL, " new "),
                d(DIFF_INSERT, "five-year deal at "),
                d(DIFF_EQUAL, "Everton"),
                d(DIFF_DELETE, " deal"),
            ],
        ),
        (
            vec![
                d(DIFF_INSERT, "星球大戰：新的希望 "),
                d(DIFF_EQUAL, "star wars: "),
                d(DIFF_DELETE, "episodio iv - un"),
                d(DIFF_EQUAL, "a n"),
                d(DIFF_DELETE, "u"),
                d(DIFF_EQUAL, "e"),
                d(DIFF_DELETE, "va"),
                d(DIFF_INSERT, "w"),
                d(DIFF_EQUAL, " "),
                d(DIFF_DELETE, "es"),
                d(DIFF_INSERT, "ho"),
                d(DIFF_EQUAL, "pe"),
                d(DIFF_DELETE, "ranza"),
            ],
            vec![
                d(DIFF_INSERT, "星球大戰：新的希望 "),
                d(DIFF_EQUAL, "star wars: "),
                d(DIFF_DELETE, "episodio iv - una nueva esperanza"),
                d(DIFF_INSERT, "a new hope"),
            ],
        ),
        (
            vec![
                d(DIFF_INSERT, "킬러 인 "),
                d(DIFF_EQUAL, "리커버리"),
                d(DIFF_DELETE, " 보이즈"),
            ],
            vec![
                d(DIFF_INSERT, "킬러 인 "),
                d(DIFF_EQUAL, "리커버리"),
                d(DIFF_DELETE, " 보이즈"),
            ],
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(dmp.diff_cleanup_semantic(input), expected);
    }
}

#[test]
fn test_diff_cleanup_efficiency() {
    let mut dmp = DiffMatchPatch::new();
    dmp.diff_edit_cost = 4;
    let cases = vec![
        (vec![], vec![]),
        (
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_INSERT, "12"),
                d(DIFF_EQUAL, "wxyz"),
                d(DIFF_DELETE, "cd"),
                d(DIFF_INSERT, "34"),
            ],
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_INSERT, "12"),
                d(DIFF_EQUAL, "wxyz"),
                d(DIFF_DELETE, "cd"),
                d(DIFF_INSERT, "34"),
            ],
        ),
        (
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_INSERT, "12"),
                d(DIFF_EQUAL, "xyz"),
                d(DIFF_DELETE, "cd"),
                d(DIFF_INSERT, "34"),
            ],
            vec![d(DIFF_DELETE, "abxyzcd"), d(DIFF_INSERT, "12xyz34")],
        ),
        (
            vec![
                d(DIFF_INSERT, "12"),
                d(DIFF_EQUAL, "x"),
                d(DIFF_DELETE, "cd"),
                d(DIFF_INSERT, "34"),
            ],
            vec![d(DIFF_DELETE, "xcd"), d(DIFF_INSERT, "12x34")],
        ),
        (
            vec![
                d(DIFF_DELETE, "ab"),
                d(DIFF_INSERT, "12"),
                d(DIFF_EQUAL, "xy"),
                d(DIFF_INSERT, "34"),
                d(DIFF_EQUAL, "z"),
                d(DIFF_DELETE, "cd"),
                d(DIFF_INSERT, "56"),
            ],
            vec![d(DIFF_DELETE, "abxyzcd"), d(DIFF_INSERT, "12xy34z56")],
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(dmp.diff_cleanup_efficiency(input), expected);
    }
    dmp.diff_edit_cost = 5;
    assert_eq!(
        dmp.diff_cleanup_efficiency(vec![
            d(DIFF_DELETE, "ab"),
            d(DIFF_INSERT, "12"),
            d(DIFF_EQUAL, "wxyz"),
            d(DIFF_DELETE, "cd"),
            d(DIFF_INSERT, "34")
        ]),
        vec![d(DIFF_DELETE, "abwxyzcd"), d(DIFF_INSERT, "12wxyz34")]
    );
}

#[test]
fn test_diff_pretty_html() {
    let dmp = DiffMatchPatch::new();
    let diffs = [
        d(DIFF_EQUAL, "a\n"),
        d(DIFF_DELETE, "<B>b</B>"),
        d(DIFF_INSERT, "c&d"),
    ];
    assert_eq!(
        dmp.diff_pretty_html(&diffs),
        "<span>a&para;<br></span><del style=\"background:#ffe6e6;\">&lt;B&gt;b&lt;/B&gt;</del><ins style=\"background:#e6ffe6;\">c&amp;d</ins>"
    );
}

#[test]
fn test_diff_pretty_text() {
    let dmp = DiffMatchPatch::new();
    assert_eq!(
        dmp.diff_pretty_text(&[
            d(DIFF_EQUAL, "a\n"),
            d(DIFF_DELETE, "<B>b</B>"),
            d(DIFF_INSERT, "c&d"),
        ]),
        "a\n\x1b[31m<B>b</B>\x1b[0m\x1b[32mc&d\x1b[0m"
    );
    assert_eq!(
        dmp.diff_pretty_text(&[
            d(DIFF_EQUAL, "a\n"),
            d(DIFF_DELETE, "b\nc\n"),
            d(DIFF_EQUAL, "def"),
            d(DIFF_INSERT, "\ng\nh"),
            d(DIFF_EQUAL, "\ni"),
        ]),
        "a\n\x1b[31mb\x1b[0m\n\x1b[31mc\x1b[0m\n\x1b[31m\x1b[0mdef\x1b[32m\x1b[0m\n\x1b[32mg\x1b[0m\n\x1b[32mh\x1b[0m\ni"
    );
}

#[test]
fn test_diff_text() {
    let dmp = DiffMatchPatch::new();
    let diffs = [
        d(DIFF_EQUAL, "jump"),
        d(DIFF_DELETE, "s"),
        d(DIFF_INSERT, "ed"),
        d(DIFF_EQUAL, " over "),
        d(DIFF_DELETE, "the"),
        d(DIFF_INSERT, "a"),
        d(DIFF_EQUAL, " lazy"),
    ];
    assert_eq!(dmp.diff_text1(&diffs), "jumps over the lazy");
    assert_eq!(dmp.diff_text2(&diffs), "jumped over a lazy");
}

#[test]
fn test_diff_delta() {
    let dmp = DiffMatchPatch::new();
    let failures = [
        (
            "jumps over the lazyx",
            "=4\t-1\t+ed\t=6\t-3\t+a\t=5\t+old dog",
            "Delta length (19) is different from source text length (20)",
        ),
        (
            "umps over the lazy",
            "=4\t-1\t+ed\t=6\t-3\t+a\t=5\t+old dog",
            "Delta length (19) is different from source text length (18)",
        ),
        ("", "+%c3%xy", "invalid URL escape \"%xy\""),
        ("", "+%c3xy", "invalid UTF-8 token: \"\\xc3xy\""),
        ("", "a", "Invalid diff operation in DiffFromDelta: a"),
        ("", "-", "strconv.ParseInt: parsing \"\": invalid syntax"),
        ("", "--1", "Negative number in DiffFromDelta: -1"),
    ];
    for (text, delta, expected) in failures {
        let error = dmp.diff_from_delta(text, delta).unwrap_err().to_string();
        assert!(
            error.starts_with(expected),
            "actual={error:?}, expected={expected:?}"
        );
    }
    assert!(dmp.diff_from_delta("", "").unwrap().is_empty());

    let mut diffs = vec![
        d(DIFF_EQUAL, "jump"),
        d(DIFF_DELETE, "s"),
        d(DIFF_INSERT, "ed"),
        d(DIFF_EQUAL, " over "),
        d(DIFF_DELETE, "the"),
        d(DIFF_INSERT, "a"),
        d(DIFF_EQUAL, " lazy"),
        d(DIFF_INSERT, "old dog"),
    ];
    let text1 = dmp.diff_text1(&diffs);
    assert_eq!(text1, "jumps over the lazy");
    let delta = dmp.diff_to_delta(&diffs);
    assert_eq!(delta, "=4\t-1\t+ed\t=6\t-3\t+a\t=5\t+old dog");
    assert_eq!(dmp.diff_from_delta(&text1, &delta).unwrap(), diffs);

    diffs = vec![
        d(DIFF_EQUAL, "\u{0680} \0 \t %"),
        d(DIFF_DELETE, "\u{0681} \x01 \n ^"),
        d(DIFF_INSERT, "\u{0682} \x02 \\ |"),
    ];
    let text1 = dmp.diff_text1(&diffs);
    assert_eq!(text1, "\u{0680} \0 \t %\u{0681} \x01 \n ^");
    let delta = dmp.diff_to_delta(&diffs);
    assert_eq!(delta, "=7\t-7\t+%DA%82 %02 %5C %7C");
    assert_eq!(dmp.diff_from_delta(&text1, &delta).unwrap(), diffs);

    diffs = vec![d(
        DIFF_INSERT,
        "A-Z a-z 0-9 - _ . ! ~ * ' ( ) ; / ? : @ & = + $ , # ",
    )];
    let delta = dmp.diff_to_delta(&diffs);
    assert_eq!(
        delta,
        "+A-Z a-z 0-9 - _ . ! ~ * ' ( ) ; / ? : @ & = + $ , # "
    );
    assert_eq!(dmp.diff_from_delta("", &delta).unwrap(), diffs);
}

#[test]
fn test_diff_x_index() {
    let dmp = DiffMatchPatch::new();
    assert_eq!(
        dmp.diff_x_index(
            &[
                d(DIFF_DELETE, "a"),
                d(DIFF_INSERT, "1234"),
                d(DIFF_EQUAL, "xyz")
            ],
            2,
        ),
        5
    );
    assert_eq!(
        dmp.diff_x_index(
            &[
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "1234"),
                d(DIFF_EQUAL, "xyz")
            ],
            3,
        ),
        1
    );
}

#[test]
fn test_diff_levenshtein() {
    let dmp = DiffMatchPatch::new();
    assert_eq!(
        dmp.diff_levenshtein(&[
            d(DIFF_DELETE, "абв"),
            d(DIFF_INSERT, "1234"),
            d(DIFF_EQUAL, "эюя")
        ]),
        4
    );
    assert_eq!(
        dmp.diff_levenshtein(&[
            d(DIFF_EQUAL, "эюя"),
            d(DIFF_DELETE, "абв"),
            d(DIFF_INSERT, "1234")
        ]),
        4
    );
    assert_eq!(
        dmp.diff_levenshtein(&[
            d(DIFF_DELETE, "абв"),
            d(DIFF_EQUAL, "эюя"),
            d(DIFF_INSERT, "1234")
        ]),
        7
    );
}

#[test]
fn test_diff_bisect() {
    let dmp = DiffMatchPatch::new();
    let expected = vec![
        d(DIFF_DELETE, "c"),
        d(DIFF_INSERT, "m"),
        d(DIFF_EQUAL, "a"),
        d(DIFF_DELETE, "t"),
        d(DIFF_INSERT, "p"),
    ];
    assert_eq!(dmp.diff_bisect("cat", "map", Deadline::Infinite), expected);
    assert_eq!(
        dmp.diff_bisect("cat", "map", Deadline::At(Instant::now())),
        vec![d(DIFF_DELETE, "cat"), d(DIFF_INSERT, "map")]
    );
    assert_eq!(
        dmp.diff_bisect(b"\xe0\xe5", b"\xe0\xe5", Deadline::Infinite),
        vec![d(DIFF_EQUAL, "��")]
    );
}

#[test]
fn test_diff_main() {
    let mut dmp = DiffMatchPatch::new();
    let simple = vec![
        ("", "", vec![]),
        ("abc", "abc", vec![d(DIFF_EQUAL, "abc")]),
        (
            "abc",
            "ab123c",
            vec![
                d(DIFF_EQUAL, "ab"),
                d(DIFF_INSERT, "123"),
                d(DIFF_EQUAL, "c"),
            ],
        ),
        (
            "a123bc",
            "abc",
            vec![
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "123"),
                d(DIFF_EQUAL, "bc"),
            ],
        ),
        (
            "abc",
            "a123b456c",
            vec![
                d(DIFF_EQUAL, "a"),
                d(DIFF_INSERT, "123"),
                d(DIFF_EQUAL, "b"),
                d(DIFF_INSERT, "456"),
                d(DIFF_EQUAL, "c"),
            ],
        ),
        (
            "a123b456c",
            "abc",
            vec![
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "123"),
                d(DIFF_EQUAL, "b"),
                d(DIFF_DELETE, "456"),
                d(DIFF_EQUAL, "c"),
            ],
        ),
    ];
    for (text1, text2, expected) in simple {
        assert_eq!(dmp.diff_main(text1, text2, false), expected);
    }
    dmp.diff_timeout = Duration::ZERO;
    let real = vec![
        ("a", "b", vec![d(DIFF_DELETE, "a"), d(DIFF_INSERT, "b")]),
        (
            "Apples are a fruit.",
            "Bananas are also fruit.",
            vec![
                d(DIFF_DELETE, "Apple"),
                d(DIFF_INSERT, "Banana"),
                d(DIFF_EQUAL, "s are a"),
                d(DIFF_INSERT, "lso"),
                d(DIFF_EQUAL, " fruit."),
            ],
        ),
        (
            "ax\t",
            "\u{0680}x\0",
            vec![
                d(DIFF_DELETE, "a"),
                d(DIFF_INSERT, "\u{0680}"),
                d(DIFF_EQUAL, "x"),
                d(DIFF_DELETE, "\t"),
                d(DIFF_INSERT, "\0"),
            ],
        ),
        (
            "1ayb2",
            "abxab",
            vec![
                d(DIFF_DELETE, "1"),
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "y"),
                d(DIFF_EQUAL, "b"),
                d(DIFF_DELETE, "2"),
                d(DIFF_INSERT, "xab"),
            ],
        ),
        (
            "abcy",
            "xaxcxabc",
            vec![
                d(DIFF_INSERT, "xaxcx"),
                d(DIFF_EQUAL, "abc"),
                d(DIFF_DELETE, "y"),
            ],
        ),
        (
            "ABCDa=bcd=efghijklmnopqrsEFGHIJKLMNOefg",
            "a-bcd-efghijklmnopqrs",
            vec![
                d(DIFF_DELETE, "ABCD"),
                d(DIFF_EQUAL, "a"),
                d(DIFF_DELETE, "="),
                d(DIFF_INSERT, "-"),
                d(DIFF_EQUAL, "bcd"),
                d(DIFF_DELETE, "="),
                d(DIFF_INSERT, "-"),
                d(DIFF_EQUAL, "efghijklmnopqrs"),
                d(DIFF_DELETE, "EFGHIJKLMNOefg"),
            ],
        ),
        (
            "a [[Pennsylvania]] and [[New",
            " and [[Pennsylvania]]",
            vec![
                d(DIFF_INSERT, " "),
                d(DIFF_EQUAL, "a"),
                d(DIFF_INSERT, "nd"),
                d(DIFF_EQUAL, " [[Pennsylvania]]"),
                d(DIFF_DELETE, " and [[New"),
            ],
        ),
    ];
    for (text1, text2, expected) in real {
        assert_eq!(dmp.diff_main(text1, text2, false), expected);
    }
    assert_eq!(
        dmp.diff_main(b"\xe0\xe5", b"", false),
        vec![d(DIFF_DELETE, "��")]
    );
}

#[test]
fn test_diff_main_with_timeout() {
    let mut dmp = DiffMatchPatch::new();
    dmp.diff_timeout = Duration::from_millis(200);
    let mut first = "`Twas brillig, and the slithy toves\nDid gyre and gimble in the wabe:\nAll mimsy were the borogoves,\nAnd the mome raths outgrabe.\n".to_owned();
    let mut second = "I am the very model of a modern major general,\nI've information vegetable, animal, and mineral,\nI know the kings of England, and I quote the fights historical,\nFrom Marathon to Waterloo, in order categorical.\n".to_owned();
    for _ in 0..13 {
        first = first.repeat(2);
        second = second.repeat(2);
    }
    let start = Instant::now();
    let _ = dmp.diff_main(&first, &second, true);
    let elapsed = start.elapsed();
    assert!(elapsed >= dmp.diff_timeout, "elapsed={elapsed:?}");
    assert!(elapsed < dmp.diff_timeout * 100, "elapsed={elapsed:?}");
}

#[test]
fn test_diff_main_with_check_lines() {
    let mut dmp = DiffMatchPatch::new();
    dmp.diff_timeout = Duration::ZERO;
    let cases = [
        (
            "1234567890\n".repeat(13),
            "abcdefghij\n".repeat(13),
        ),
        (
            "1234567890".repeat(13),
            "abcdefghij".repeat(13),
        ),
        (
            "1234567890\n".repeat(13),
            "abcdefghij\n1234567890\n1234567890\n1234567890\nabcdefghij\n1234567890\n1234567890\n1234567890\nabcdefghij\n1234567890\n1234567890\n1234567890\nabcdefghij\n".to_owned(),
        ),
    ];
    for (index, (text1, text2)) in cases.into_iter().enumerate() {
        let direct = dmp.diff_main(&text1, &text2, false);
        let line_mode = dmp.diff_main(&text1, &text2, true);
        if index != 2 {
            assert_eq!(direct, line_mode);
        }
        assert_eq!(rebuild(&direct), rebuild(&line_mode));
    }
}

#[test]
fn test_massive_rune_diff_conversion() {
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/original/go-diff/testdata/fixture.go"
    ))
    .unwrap();
    let dmp = DiffMatchPatch::new();
    let (first, second, lines) = dmp.diff_lines_to_chars(b"", &fixture);
    let diffs = dmp.diff_chars_to_lines(&dmp.diff_main(&first, &second, false), &lines);
    assert!(dmp.diff_text1(&diffs).is_empty());
    assert_eq!(dmp.diff_text2(&diffs).as_bytes(), fixture);
}

#[test]
fn test_diff_partial_line_index() {
    let dmp = DiffMatchPatch::new();
    let first =
        "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10 text1";
    let second =
        "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10 text2";
    let (encoded1, encoded2, lines) = dmp.diff_lines_to_chars(first, second);
    let diffs = dmp.diff_chars_to_lines(&dmp.diff_main(&encoded1, &encoded2, false), &lines);
    assert_eq!(
        diffs,
        vec![
            d(
                DIFF_EQUAL,
                "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\n"
            ),
            d(DIFF_DELETE, "line 10 text1"),
            d(DIFF_INSERT, "line 10 text2"),
        ]
    );
}
