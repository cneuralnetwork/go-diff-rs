use crate::{
    DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, Deadline, Diff, DiffMatchPatch, FOUR_BYTE_BITS,
    ONE_BYTE_BITS, Operation, Patch, PatchError, THREE_BYTE_BITS, TWO_BYTE_BITS, Text,
    UNICODE_INVALID_RANGE_DELTA, UNICODE_INVALID_RANGE_END, UNICODE_INVALID_RANGE_START,
    UNICODE_RANGE_MAX,
};
use std::error::Error;
use std::time::{Duration, Instant};

#[test]
fn text_preserves_bytes_and_exposes_native_conversions() {
    let mut empty = Text::new();
    assert!(empty.is_empty());
    empty.as_mut_bytes().extend_from_slice(b"ok");
    assert_eq!(empty.len(), 2);
    assert_eq!(empty.as_ref(), b"ok");
    assert_eq!(&*empty, b"ok");
    assert_eq!(empty.as_str().unwrap(), "ok");
    assert_eq!(empty.to_string(), "ok");
    assert_eq!(format!("{empty:?}"), "b\"ok\"");

    let invalid = Text::from_bytes(vec![0xff, b'x']);
    assert!(invalid.as_str().is_err());
    assert_eq!(invalid.to_string_lossy(), "�x");
    assert_eq!(format!("{invalid:?}"), "b\"\\xffx\"");

    assert_eq!(Text::from(vec![b'a']).into_bytes(), b"a");
    assert_eq!(Text::from(&b"ab"[..]), "ab");
    assert_eq!(Text::from(b"ab"), "ab");
    assert_eq!(Text::from(String::from("ab")), "ab");
    assert_eq!(Text::from("ab"), "ab");
}

#[test]
fn public_types_keep_go_shape_and_formatting() {
    assert_eq!(DIFF_DELETE.value(), -1);
    assert_eq!(DIFF_EQUAL.value(), 0);
    assert_eq!(DIFF_INSERT.value(), 1);
    assert_eq!(Operation::from(-1).to_string(), "Delete");
    assert_eq!(format!("{:?}", Operation::from(0)), "Equal");
    let unknown = Operation::new(7);
    assert_eq!(unknown.value(), 7);
    assert_eq!(unknown.to_string(), "Operation(7)");
    assert_eq!(i8::from(unknown), 7);

    let mut patch = Patch::new(vec![Diff::new(DIFF_EQUAL, "x")], 1, 2, 3, 4);
    assert_eq!(patch.diffs().len(), 1);
    patch.diffs_mut().push(Diff::new(DIFF_INSERT, "y"));
    assert_eq!(patch.diffs().len(), 2);
    assert_eq!(
        (patch.start1, patch.start2, patch.length1, patch.length2),
        (1, 2, 3, 4)
    );

    assert!(!Deadline::Infinite.expired());
    assert!(Deadline::At(Instant::now() - Duration::from_millis(1)).expired());
    assert_eq!(
        DiffMatchPatch::new().diff_edit_cost,
        DiffMatchPatch::default().diff_edit_cost
    );

    assert_eq!(UNICODE_INVALID_RANGE_START, 0xD800);
    assert_eq!(UNICODE_INVALID_RANGE_END, 0xDFFF);
    assert_eq!(UNICODE_INVALID_RANGE_DELTA, 0x800);
    assert_eq!(UNICODE_RANGE_MAX, 0x10_FFFF);
    assert_eq!(
        (
            ONE_BYTE_BITS,
            TWO_BYTE_BITS,
            THREE_BYTE_BITS,
            FOUR_BYTE_BITS
        ),
        (7, 11, 16, 21)
    );
}

#[test]
fn patch_errors_are_typed_and_reproducible() {
    let cases = [
        (
            PatchError::InvalidPatchString(Text::from("bad")),
            "Invalid patch string: bad",
        ),
        (
            PatchError::InvalidPatchMode {
                mode: b'?',
                line: Text::from("?bad"),
            },
            "Invalid patch mode '?' in: ?bad",
        ),
        (
            PatchError::InvalidEscape("invalid escape".to_owned()),
            "invalid escape",
        ),
        (
            PatchError::InvalidNumber("invalid number".to_owned()),
            "invalid number",
        ),
        (
            PatchError::InvalidUtf8Token(Text::from_bytes(vec![0xff])),
            "invalid UTF-8 token: \"\\xff\"",
        ),
        (
            PatchError::NegativeNumber("-1".to_owned()),
            "Negative number in DiffFromDelta: -1",
        ),
        (
            PatchError::InvalidDiffOperation(b'?'),
            "Invalid diff operation in DiffFromDelta: ?",
        ),
        (
            PatchError::DeltaLength {
                consumed: 1,
                source_bytes: 2,
            },
            "Delta length (1) is different from source text length (2)",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
        assert!(error.source().is_none());
    }
}
