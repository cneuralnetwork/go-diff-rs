use diff_match_patch::DiffMatchPatch;
use std::env;
use std::fs;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.len() != 4 || arguments[0] != "batch" {
        eprintln!("usage: bench-driver batch INPUT1 INPUT2 ITERATIONS");
        return ExitCode::from(2);
    }
    let first = fs::read(&arguments[1]).expect("read first workload");
    let second = fs::read(&arguments[2]).expect("read second workload");
    let iterations: usize = arguments[3].parse().expect("parse iterations");
    let dmp = DiffMatchPatch::new();
    let mut samples = Vec::with_capacity(iterations);
    let mut last = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        last = black_box(dmp.diff_main(black_box(&first), black_box(&second), true));
        samples.push(start.elapsed().as_nanos());
    }
    println!("diffs={}", last.len());
    println!("text1_bytes={}", dmp.diff_text1(&last).len());
    println!("text2_bytes={}", dmp.diff_text2(&last).len());
    println!(
        "samples_ns={}",
        samples
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    ExitCode::SUCCESS
}
