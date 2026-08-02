use crate::DiffMatchPatch;
use std::collections::HashMap;

#[test]
fn test_match_alphabet() {
    let dmp = DiffMatchPatch::new();
    assert_eq!(
        dmp.match_alphabet("abc"),
        HashMap::from([(b'a', 4), (b'b', 2), (b'c', 1)])
    );
    assert_eq!(
        dmp.match_alphabet("abcaba"),
        HashMap::from([(b'a', 37), (b'b', 18), (b'c', 8)])
    );
}

#[test]
fn test_match_bitap() {
    let mut dmp = DiffMatchPatch::new();
    dmp.match_distance = 100;
    dmp.match_threshold = 0.5;
    let cases = [
        ("Exact match #1", "abcdefghijk", "fgh", 5, 5),
        ("Exact match #2", "abcdefghijk", "fgh", 0, 5),
        ("Fuzzy match #1", "abcdefghijk", "efxhi", 0, 4),
        ("Fuzzy match #2", "abcdefghijk", "cdefxyhijk", 5, 2),
        ("Fuzzy match #3", "abcdefghijk", "bxy", 1, -1),
        ("Overflow", "123456789xx0", "3456789x0", 2, 2),
        ("Before start match", "abcdef", "xxabc", 4, 0),
        ("Beyond end match", "abcdef", "defyy", 4, 3),
        ("Oversized pattern", "abcdef", "xabcdefy", 0, 0),
    ];
    for (name, text, pattern, location, expected) in cases {
        assert_eq!(dmp.match_bitap(text, pattern, location), expected, "{name}");
    }

    dmp.match_threshold = 0.4;
    assert_eq!(dmp.match_bitap("abcdefghijk", "efxyhi", 1), 4);
    dmp.match_threshold = 0.3;
    assert_eq!(dmp.match_bitap("abcdefghijk", "efxyhi", 1), -1);
    dmp.match_threshold = 0.0;
    assert_eq!(dmp.match_bitap("abcdefghijk", "bcdef", 1), 1);
    dmp.match_threshold = 0.5;
    assert_eq!(dmp.match_bitap("abcdexyzabcde", "abccde", 3), 0);
    assert_eq!(dmp.match_bitap("abcdexyzabcde", "abccde", 5), 8);
    dmp.match_distance = 10;
    assert_eq!(
        dmp.match_bitap("abcdefghijklmnopqrstuvwxyz", "abcdefg", 24),
        -1
    );
    assert_eq!(
        dmp.match_bitap("abcdefghijklmnopqrstuvwxyz", "abcdxxefg", 1),
        0
    );
    dmp.match_distance = 1000;
    assert_eq!(
        dmp.match_bitap("abcdefghijklmnopqrstuvwxyz", "abcdefg", 24),
        0
    );
}

#[test]
fn test_match_main() {
    let mut dmp = DiffMatchPatch::new();
    let cases = [
        ("Equality", "abcdef", "abcdef", 1000, 0),
        ("Null text", "", "abcdef", 1, -1),
        ("Null pattern", "abcdef", "", 3, 3),
        ("Exact match", "abcdef", "de", 3, 3),
        ("Beyond end match", "abcdef", "defy", 4, 3),
        ("Oversized pattern", "abcdef", "abcdefy", 0, 0),
    ];
    for (name, text, pattern, location, expected) in cases {
        assert_eq!(dmp.match_main(text, pattern, location), expected, "{name}");
    }
    dmp.match_threshold = 0.7;
    assert_eq!(
        dmp.match_main(
            "I am the very model of a modern major general.",
            " that berry ",
            5
        ),
        4
    );
}
