use diff_match_patch::{Diff, DiffMatchPatch};
use std::fmt::Write as _;
use std::io::{self, BufRead, BufWriter, Write};

fn main() {
    let stdin = io::stdin();
    let mut output = BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) => handle(&line).unwrap_or_else(|error| format!("ERR\t{error}")),
            Err(error) => format!("ERR\t{error}"),
        };
        writeln!(output, "{response}").expect("write oracle response");
        output.flush().expect("flush oracle response");
    }
}

fn handle(line: &str) -> Result<String, String> {
    let fields: Vec<&str> = line.split('\t').collect();
    let dmp = DiffMatchPatch::new();
    match fields.as_slice() {
        ["D", check_lines, first, second] => {
            let first = decode_hex(first)?;
            let second = decode_hex(second)?;
            let diffs = dmp.diff_main(&first, &second, *check_lines == "1");
            let delta = dmp.diff_to_delta(&diffs);
            let roundtrip = dmp
                .diff_from_delta(dmp.diff_text1(&diffs), &delta)
                .map_err(|error| error.to_string())?;
            let lossless = dmp.diff_cleanup_semantic_lossless(diffs.clone());
            let semantic = dmp.diff_cleanup_semantic(diffs.clone());
            let efficient = dmp.diff_cleanup_efficiency(semantic.clone());
            Ok(format!(
                "D\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                canonical_diffs(&diffs),
                encode_hex(delta.as_bytes()),
                encode_hex(dmp.diff_pretty_html(&diffs).as_bytes()),
                encode_hex(dmp.diff_pretty_text(&diffs).as_bytes()),
                dmp.diff_levenshtein(&diffs),
                encode_hex(dmp.diff_text1(&diffs).as_bytes()),
                encode_hex(dmp.diff_text2(&diffs).as_bytes()),
                canonical_diffs(&roundtrip),
                canonical_diffs(&lossless),
                canonical_diffs(&semantic),
                canonical_diffs(&efficient),
            ))
        }
        ["M", text, pattern, location] => {
            let text = decode_hex(text)?;
            let pattern = decode_hex(pattern)?;
            let location = location
                .parse::<isize>()
                .map_err(|error| error.to_string())?;
            Ok(format!("M\t{}", dmp.match_main(text, pattern, location)))
        }
        ["P", first, second, target] => {
            let first = decode_hex(first)?;
            let second = decode_hex(second)?;
            let target = decode_hex(target)?;
            let patch_text = dmp.patch_to_text(&dmp.patch_make(&first, &second));
            let parsed = dmp
                .patch_from_text(&patch_text)
                .map_err(|error| error.to_string())?;
            let (result, applied) = dmp.patch_apply(&parsed, target);
            let flags: String = applied
                .into_iter()
                .map(|flag| if flag { '1' } else { '0' })
                .collect();
            Ok(format!(
                "P\t{}\t{}\t{flags}",
                encode_hex(patch_text.as_bytes()),
                encode_hex(result.as_bytes()),
            ))
        }
        _ => Err("invalid request".to_owned()),
    }
}

fn canonical_diffs(diffs: &[Diff]) -> String {
    let mut output = String::new();
    for (index, diff) in diffs.iter().enumerate() {
        if index != 0 {
            output.push('|');
        }
        write!(
            output,
            "{}:{}",
            diff.operation.value(),
            encode_hex(diff.text.as_bytes())
        )
        .expect("write canonical diff");
    }
    output
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("write hex");
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("odd-length hex input".to_owned());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
            u8::from_str_radix(text, 16).map_err(|error| error.to_string())
        })
        .collect()
}
