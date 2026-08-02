use diff_match_patch::DiffMatchPatch;
use std::hint::black_box;
use std::time::{Duration, Instant};

const DATA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/original/go-diff/testdata/"
);

fn main() {
    if cfg!(debug_assertions) {
        println!("Run `cargo bench --bench upstream` for the nine translated benchmarks.");
        return;
    }
    let (speed1, speed2) = speedtest_texts();
    let dmp = DiffMatchPatch::new();
    let common_value = "ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖ";
    benchmark("BenchmarkDiffCommonPrefix", 1_000_000, || {
        black_box(dmp.diff_common_prefix(common_value, common_value));
    });
    benchmark("BenchmarkDiffCommonSuffix", 1_000_000, || {
        black_box(dmp.diff_common_suffix(common_value, common_value));
    });
    let long1 = format!("{}B{}", "A".repeat(1000), "C".repeat(1000));
    let long2 = format!("{}-{}", "A".repeat(1000), "C".repeat(1000));
    benchmark("BenchmarkCommonLength", 25_000, || {
        for (left, right) in [("", ""), ("AABCC", "AA-CC"), (&long1, &long2)] {
            black_box(dmp.diff_common_prefix(left, right));
            black_box(dmp.diff_common_suffix(left, right));
        }
    });
    benchmark("BenchmarkDiffHalfMatch", 50, || {
        black_box(dmp.diff_half_match(&speed1, &speed2));
    });

    let cleanup_dmp = DiffMatchPatch::new();
    let cleanup_input = cleanup_dmp.diff_main(&speed1, &speed2, false);
    benchmark("BenchmarkDiffCleanupSemantic", 100, || {
        black_box(cleanup_dmp.diff_cleanup_semantic(cleanup_input.clone()));
    });

    let mut first = "`Twas brillig, and the slithy toves\nDid gyre and gimble in the wabe:\nAll mimsy were the borogoves,\nAnd the mome raths outgrabe.\n".to_owned();
    let mut second = "I am the very model of a modern major general,\nI've information vegetable, animal, and mineral,\nI know the kings of England, and I quote the fights historical,\nFrom Marathon to Waterloo, in order categorical.\n".to_owned();
    for _ in 0..10 {
        first = first.repeat(2);
        second = second.repeat(2);
    }
    let mut timeout_dmp = DiffMatchPatch::new();
    timeout_dmp.diff_timeout = Duration::from_secs(1);
    benchmark("BenchmarkDiffMain", 1, || {
        black_box(timeout_dmp.diff_main(&first, &second, true));
    });
    benchmark("BenchmarkDiffMainLarge", 20, || {
        black_box(dmp.diff_main(&speed1, &speed2, true));
    });
    benchmark("BenchmarkDiffMainRunesLargeLines", 20, || {
        let (first, second, lines) = dmp.diff_lines_to_runes(&speed1, &speed2);
        let diffs = dmp.diff_main_runes(&first, &second, false);
        black_box(dmp.diff_chars_to_lines(&diffs, &lines));
    });

    let large_lines =
        std::fs::read(format!("{DATA}diff10klinestest.txt")).expect("read diff10klinestest.txt");
    benchmark("BenchmarkDiffMainRunesLargeDiffLines", 3, || {
        let (first, second, lines) = dmp.diff_lines_to_runes(&large_lines, b"");
        let diffs = dmp.diff_main_runes(&first, &second, false);
        black_box(dmp.diff_chars_to_lines(&diffs, &lines));
    });
}

fn speedtest_texts() -> (Vec<u8>, Vec<u8>) {
    (
        std::fs::read(format!("{DATA}speedtest1.txt")).expect("read speedtest1.txt"),
        std::fs::read(format!("{DATA}speedtest2.txt")).expect("read speedtest2.txt"),
    )
}

fn benchmark(name: &str, iterations: u64, mut operation: impl FnMut()) {
    operation();
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    let elapsed = start.elapsed();
    println!(
        "{name}\titerations={iterations}\tns_per_op={:.1}",
        elapsed.as_nanos() as f64 / iterations as f64
    );
}
