use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_INPUT_ID: AtomicUsize = AtomicUsize::new(0);

struct InputFile(PathBuf);

impl InputFile {
    fn new(input: &[u8]) -> Self {
        let id = NEXT_INPUT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "driver-differential-input-{}-{id}",
            std::process::id()
        ));
        fs::write(&path, input).expect("failed to create subprocess input file");
        Self(path)
    }
}

impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn run(binary: &Path, input: &Path) -> Output {
    Command::new(binary)
        .stdin(Stdio::from(
            File::open(input).expect("failed to open subprocess input file"),
        ))
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()))
}

fn assert_matches_c(name: &str, input: &[u8]) {
    let input_file = InputFile::new(input);
    let c_binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver");
    let rust_binary = Path::new(env!("CARGO_BIN_EXE_driver"));

    let c = run(&c_binary, &input_file.0);
    let rust = run(rust_binary, &input_file.0);

    assert_eq!(rust.stdout, c.stdout, "{name}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{name}: stderr differs");
    assert_eq!(rust.status, c.status, "{name}: exit status differs");
}

#[test]
fn matches_c_for_every_input_class() {
    let binary_input: Vec<u8> = (0_u8..=u8::MAX).collect();
    let large_input: Vec<u8> = (0_u8..=u8::MAX).cycle().take(65_536).collect();

    let cases: [(&str, &[u8]); 4] = [
        ("empty input", b""),
        ("single item", b"x"),
        ("multiline and binary input", &binary_input),
        ("large input", &large_input),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}
