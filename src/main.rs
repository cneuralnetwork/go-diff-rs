use diff_match_patch::DiffMatchPatch;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("go-diff-rs: {message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.is_empty() || arguments[0] == "--help" || arguments[0] == "-h" {
        print_help();
        return Ok(());
    }
    let dmp = DiffMatchPatch::new();
    match arguments[0].as_str() {
        "diff" if arguments.len() >= 3 => {
            let first = read(&arguments[1])?;
            let second = read(&arguments[2])?;
            let format = arguments
                .windows(2)
                .find(|pair| pair[0] == "--format")
                .map_or("text", |pair| pair[1].as_str());
            let check_lines = arguments.iter().any(|argument| argument == "--check-lines");
            let diffs = dmp.diff_main(&first, &second, check_lines);
            let output = match format {
                "text" => dmp.diff_pretty_text(&diffs),
                "html" => dmp.diff_pretty_html(&diffs),
                "delta" => dmp.diff_to_delta(&diffs),
                "patch" => dmp.patch_to_text(&dmp.patch_make_from_text_and_diffs(&first, &diffs)),
                other => return Err((2, format!("unknown diff format: {other}"))),
            };
            io::stdout()
                .write_all(output.as_bytes())
                .map_err(|error| (1, error.to_string()))?;
        }
        "match" if arguments.len() == 4 => {
            let text = read(&arguments[1])?;
            let pattern = read(&arguments[2])?;
            let location = arguments[3]
                .parse::<isize>()
                .map_err(|error| (2, format!("invalid location: {error}")))?;
            println!("{}", dmp.match_main(&text, &pattern, location));
        }
        "patch-make" if arguments.len() == 3 => {
            let first = read(&arguments[1])?;
            let second = read(&arguments[2])?;
            let output = dmp.patch_to_text(&dmp.patch_make(&first, &second));
            io::stdout()
                .write_all(output.as_bytes())
                .map_err(|error| (1, error.to_string()))?;
        }
        "patch-apply" if arguments.len() == 3 => {
            let patch_text = read(&arguments[1])?;
            let target = read(&arguments[2])?;
            let patches = dmp
                .patch_from_text(&patch_text)
                .map_err(|error| (1, error.to_string()))?;
            let (output, applied) = dmp.patch_apply(&patches, &target);
            io::stdout()
                .write_all(output.as_bytes())
                .map_err(|error| (1, error.to_string()))?;
            eprintln!(
                "applied={}",
                applied
                    .iter()
                    .map(|value| if *value { "1" } else { "0" })
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        _ => {
            print_help();
            return Err((2, "invalid command or arguments".to_owned()));
        }
    }
    Ok(())
}

fn read(path: &str) -> Result<Vec<u8>, (u8, String)> {
    fs::read(path).map_err(|error| (1, format!("cannot read {path}: {error}")))
}

fn print_help() {
    println!(
        "go-diff-rs\n\n\
Usage:\n  \
go-diff-rs diff OLD NEW [--check-lines] [--format text|html|delta|patch]\n  \
go-diff-rs match TEXT PATTERN BYTE_LOCATION\n  \
go-diff-rs patch-make OLD NEW\n  \
go-diff-rs patch-apply PATCH TARGET"
    );
}
