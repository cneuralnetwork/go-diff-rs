mod bug_regressions;
mod concurrency_test;
mod diff_test;
mod index_test;
mod match_test;
mod patch_test;
mod rust_api_test;
mod stringutil_test;

use crate::{Diff, Operation, Text};

fn d(operation: Operation, text: impl Into<Text>) -> Diff {
    Diff::new(operation, text)
}
