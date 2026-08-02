use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("go-diff-rs-cli-{}-{nonce}", std::process::id()));
        fs::create_dir(&directory).expect("create CLI fixture directory");
        Self { directory }
    }

    fn write(&self, name: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.directory.join(name);
        fs::write(&path, bytes).expect("write CLI fixture");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_go-diff-rs")
}

#[test]
fn diff_match_and_invalid_command() {
    let fixture = Fixture::new();
    let first = fixture.write("first.txt", b"abc");
    let second = fixture.write("second.txt", b"axc");
    let pattern = fixture.write("pattern.txt", b"x");

    let diff = Command::new(binary())
        .args(["diff", first.to_str().unwrap(), second.to_str().unwrap()])
        .args(["--format", "delta"])
        .output()
        .expect("run diff command");
    assert!(diff.status.success());
    assert_eq!(diff.stdout, b"=1\t-1\t+x\t=1");

    let matched = Command::new(binary())
        .args([
            "match",
            second.to_str().unwrap(),
            pattern.to_str().unwrap(),
            "1",
        ])
        .output()
        .expect("run match command");
    assert!(matched.status.success());
    assert_eq!(matched.stdout, b"1\n");

    let invalid = Command::new(binary())
        .arg("not-a-command")
        .output()
        .expect("run invalid command");
    assert_eq!(invalid.status.code(), Some(2));
}

#[test]
fn patch_make_and_apply_roundtrip() {
    let fixture = Fixture::new();
    let first = fixture.write("first.txt", b"alpha beta\n");
    let second = fixture.write("second.txt", b"alpha delta\n");
    let made = Command::new(binary())
        .args([
            "patch-make",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .output()
        .expect("run patch-make command");
    assert!(made.status.success());
    let patch = fixture.write("change.patch", made.stdout);
    let applied = Command::new(binary())
        .args([
            "patch-apply",
            patch.to_str().unwrap(),
            first.to_str().unwrap(),
        ])
        .output()
        .expect("run patch-apply command");
    assert!(applied.status.success());
    assert_eq!(applied.stdout, b"alpha delta\n");
    assert_eq!(applied.stderr, b"applied=1\n");
}
