use crate::Text;
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

pub const UNICODE_INVALID_RANGE_START: u32 = 0xD800;
pub const UNICODE_INVALID_RANGE_END: u32 = 0xDFFF;
pub const UNICODE_INVALID_RANGE_DELTA: u32 =
    UNICODE_INVALID_RANGE_END - UNICODE_INVALID_RANGE_START + 1;
pub const UNICODE_RANGE_MAX: u32 = 0x10_FFFF;
pub const ONE_BYTE_BITS: u8 = 7;
pub const TWO_BYTE_BITS: u8 = 11;
pub const THREE_BYTE_BITS: u8 = 16;
pub const FOUR_BYTE_BITS: u8 = 21;

/// An open numeric diff operation, matching Go's `type Operation int8`.
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Operation(i8);

impl Operation {
    pub const DELETE: Self = Self(-1);
    pub const EQUAL: Self = Self(0);
    pub const INSERT: Self = Self(1);

    #[must_use]
    pub const fn new(value: i8) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> i8 {
        self.0
    }
}

pub const DIFF_DELETE: Operation = Operation::DELETE;
pub const DIFF_EQUAL: Operation = Operation::EQUAL;
pub const DIFF_INSERT: Operation = Operation::INSERT;

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::DELETE => f.write_str("Delete"),
            Self::EQUAL => f.write_str("Equal"),
            Self::INSERT => f.write_str("Insert"),
            _ => write!(f, "Operation({})", self.0),
        }
    }
}

impl fmt::Debug for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<i8> for Operation {
    fn from(value: i8) -> Self {
        Self(value)
    }
}

impl From<Operation> for i8 {
    fn from(value: Operation) -> Self {
        value.0
    }
}

/// One diff operation and its exact byte payload.
#[derive(Clone, Default, Eq, PartialEq, Debug)]
pub struct Diff {
    pub operation: Operation,
    pub text: Text,
}

impl Diff {
    #[must_use]
    pub fn new(operation: Operation, text: impl Into<Text>) -> Self {
        Self {
            operation,
            text: text.into(),
        }
    }
}

/// A patch operation. Coordinates and lengths use byte offsets, as in go-diff.
#[derive(Clone, Default, Eq, PartialEq, Debug)]
pub struct Patch {
    pub(crate) diffs: Vec<Diff>,
    pub start1: isize,
    pub start2: isize,
    pub length1: isize,
    pub length2: isize,
}

impl Patch {
    #[must_use]
    pub fn new(
        diffs: Vec<Diff>,
        start1: isize,
        start2: isize,
        length1: isize,
        length2: isize,
    ) -> Self {
        Self {
            diffs,
            start1,
            start2,
            length1,
            length2,
        }
    }

    #[must_use]
    pub fn diffs(&self) -> &[Diff] {
        &self.diffs
    }

    pub fn diffs_mut(&mut self) -> &mut Vec<Diff> {
        &mut self.diffs
    }
}

/// Deadline used by the public bisect API.
#[derive(Clone, Copy, Debug)]
pub enum Deadline {
    Infinite,
    At(Instant),
}

impl Deadline {
    pub(crate) fn expired(self) -> bool {
        match self {
            Self::Infinite => false,
            Self::At(at) => Instant::now() > at,
        }
    }
}

/// Diff-match-patch configuration.
#[derive(Clone, Debug)]
pub struct DiffMatchPatch {
    pub diff_timeout: Duration,
    pub diff_edit_cost: isize,
    pub match_distance: isize,
    pub patch_delete_threshold: f64,
    pub patch_margin: isize,
    pub match_max_bits: isize,
    pub match_threshold: f64,
}

impl Default for DiffMatchPatch {
    fn default() -> Self {
        Self {
            diff_timeout: Duration::from_secs(1),
            diff_edit_cost: 4,
            match_threshold: 0.5,
            match_distance: 1000,
            patch_delete_threshold: 0.5,
            patch_margin: 4,
            match_max_bits: 32,
        }
    }
}

impl DiffMatchPatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum PatchError {
    InvalidPatchString(Text),
    InvalidPatchMode {
        mode: u8,
        line: Text,
    },
    InvalidEscape(String),
    InvalidUtf8Token(Text),
    InvalidNumber(String),
    NegativeNumber(String),
    InvalidDiffOperation(u8),
    DeltaLength {
        consumed: usize,
        source_bytes: usize,
    },
    PartialPatchParse {
        patches: Vec<Patch>,
        error: Box<PatchError>,
    },
}

impl PatchError {
    /// Return patches parsed successfully before a later patch parse error.
    #[must_use]
    pub fn partial_patches(&self) -> Option<&[Patch]> {
        match self {
            Self::PartialPatchParse { patches, .. } => Some(patches),
            _ => None,
        }
    }
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPatchString(line) => write!(f, "Invalid patch string: {line}"),
            Self::InvalidPatchMode { mode, line } => {
                write!(f, "Invalid patch mode '{}' in: {}", char::from(*mode), line)
            }
            Self::InvalidEscape(message) | Self::InvalidNumber(message) => f.write_str(message),
            Self::InvalidUtf8Token(token) => {
                f.write_str("invalid UTF-8 token: \"")?;
                for &byte in token.as_bytes() {
                    for escaped in std::ascii::escape_default(byte) {
                        f.write_str(&char::from(escaped).to_string())?;
                    }
                }
                f.write_str("\"")
            }
            Self::NegativeNumber(number) => {
                write!(f, "Negative number in DiffFromDelta: {number}")
            }
            Self::InvalidDiffOperation(operation) => write!(
                f,
                "Invalid diff operation in DiffFromDelta: {}",
                char::from(*operation)
            ),
            Self::DeltaLength {
                consumed,
                source_bytes,
            } => write!(
                f,
                "Delta length ({consumed}) is different from source text length ({source_bytes})"
            ),
            Self::PartialPatchParse { error, .. } => fmt::Display::fmt(error, f),
        }
    }
}

impl Error for PatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PartialPatchParse { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}
