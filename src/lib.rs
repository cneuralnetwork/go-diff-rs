// Copyright (c) 2012-2016 The go-diff authors. All rights reserved.
// Rust port copyright (c) 2026 the go-diff-rs authors.
// See LICENSE and APACHE-LICENSE-2.0 for license details.

#![forbid(unsafe_code)]

//! Diff, fuzzy-match, and patch algorithms.
//!
//! This is a native Rust source port of `github.com/sergi/go-diff` v1.4.0.
//! It intentionally preserves Go byte-string behavior, including invalid UTF-8.

mod diff;
mod match_impl;
mod patch;
mod text;
mod types;
mod util;

pub use text::Text;
pub use types::{
    DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, Deadline, Diff, DiffMatchPatch, FOUR_BYTE_BITS,
    ONE_BYTE_BITS, Operation, Patch, PatchError, THREE_BYTE_BITS, TWO_BYTE_BITS,
    UNICODE_INVALID_RANGE_DELTA, UNICODE_INVALID_RANGE_END, UNICODE_INVALID_RANGE_START,
    UNICODE_RANGE_MAX,
};

#[cfg(test)]
#[path = "../tests/port/mod.rs"]
mod port_tests;
