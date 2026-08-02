use crate::util::{index_of, index_of_runes, int_to_rune, last_index_of, rune_to_int};
use crate::{UNICODE_INVALID_RANGE_DELTA, UNICODE_RANGE_MAX};

#[test]
fn test_runes_index_of() {
    let target: Vec<u32> = "abcde".chars().map(u32::from).collect();
    let cases = [
        ("abc", 0, 0),
        ("cde", 0, 2),
        ("e", 0, 4),
        ("cdef", 0, -1),
        ("abcdef", 0, -1),
        ("abc", 2, -1),
        ("cde", 2, 2),
        ("e", 2, 4),
        ("cdef", 2, -1),
        ("abcdef", 2, -1),
        ("e", 6, -1),
    ];
    for (pattern, start, expected) in cases {
        let pattern: Vec<u32> = pattern.chars().map(u32::from).collect();
        assert_eq!(index_of_runes(&target, &pattern, start), expected);
    }
}

#[test]
fn test_index_of() {
    let cases = [
        ("hi world", "world", -1, 3),
        ("hi world", "world", 0, 3),
        ("hi world", "world", 1, 3),
        ("hi world", "world", 2, 3),
        ("hi world", "world", 3, 3),
        ("hi world", "world", 4, -1),
        ("abbc", "b", -1, 1),
        ("abbc", "b", 0, 1),
        ("abbc", "b", 1, 1),
        ("abbc", "b", 2, 2),
        ("abbc", "b", 3, -1),
        ("abbc", "b", 4, -1),
        ("aββc", "β", -1, 1),
        ("aββc", "β", 0, 1),
        ("aββc", "β", 1, 1),
        ("aββc", "β", 3, 3),
        ("aββc", "β", 5, -1),
        ("aββc", "β", 6, -1),
    ];
    for (text, pattern, start, expected) in cases {
        assert_eq!(
            index_of(text.as_bytes(), pattern.as_bytes(), start),
            expected
        );
    }
}

#[test]
fn test_last_index_of() {
    let cases = [
        ("hi world", "world", -1, -1),
        ("hi world", "world", 0, -1),
        ("hi world", "world", 1, -1),
        ("hi world", "world", 2, -1),
        ("hi world", "world", 3, -1),
        ("hi world", "world", 4, -1),
        ("hi world", "world", 5, -1),
        ("hi world", "world", 6, -1),
        ("hi world", "world", 7, 3),
        ("hi world", "world", 8, 3),
        ("abbc", "b", -1, -1),
        ("abbc", "b", 0, -1),
        ("abbc", "b", 1, 1),
        ("abbc", "b", 2, 2),
        ("abbc", "b", 3, 2),
        ("abbc", "b", 4, 2),
        ("aββc", "β", -1, -1),
        ("aββc", "β", 0, -1),
        ("aββc", "β", 1, 1),
        ("aββc", "β", 3, 3),
        ("aββc", "β", 5, 3),
        ("aββc", "β", 6, 3),
    ];
    for (text, pattern, start, expected) in cases {
        assert_eq!(
            last_index_of(text.as_bytes(), pattern.as_bytes(), start),
            expected
        );
    }
}

#[test]
fn test_rune_to_int() {
    for value in 0..=UNICODE_RANGE_MAX - UNICODE_INVALID_RANGE_DELTA - 3 {
        let rune = int_to_rune(value);
        assert_eq!(rune_to_int(rune), value, "value={value}, rune={rune}");
    }
    assert!(
        std::panic::catch_unwind(|| {
            int_to_rune(UNICODE_RANGE_MAX - UNICODE_INVALID_RANGE_DELTA - 2);
        })
        .is_err()
    );
}
