use libloading::Library;
use std::ffi::{CString, c_void};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

const C_LIBRARY: &str = "c_src/build/libdriver.so";
const RUST_LIBRARY: &str = "target/release/libdriver.so";
const SYMBOLS: &[&[u8]] = &[
    b"bad\0",
    b"driver\0",
    b"good\0",
    b"printIntLine\0",
    b"printLine\0",
];

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        Self {
            c: load_library(C_LIBRARY),
            rust: load_library(RUST_LIBRARY),
        }
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_library(relative: &str) -> Library {
    let path = crate_path(relative);
    assert!(
        path.is_file(),
        "shared library is missing: {}",
        path.display()
    );
    unsafe { Library::new(path).expect("failed to load shared library") }
}

fn runner_path() -> PathBuf {
    let deps_dir = std::env::current_exe()
        .expect("failed to find current test executable")
        .parent()
        .expect("test executable has no parent")
        .to_path_buf();

    fs::read_dir(&deps_dir)
        .expect("failed to read test dependency directory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let metadata = entry.metadata().ok()?;
            if name.starts_with("ffi_runner-")
                && metadata.is_file()
                && metadata.permissions().mode() & 0o111 != 0
            {
                Some((metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), path))
            } else {
                None
            }
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
        .expect("failed to locate ffi_runner test executable")
}

fn invoke(relative_library: &str, mode: &str, inputs: &[String]) -> Vec<u8> {
    let output = Command::new(runner_path())
        .arg(crate_path(relative_library))
        .arg(mode)
        .args(inputs)
        .output()
        .expect("failed to execute ffi_runner");
    assert!(
        output.status.success(),
        "ffi_runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn encode_hex(value: &CString) -> String {
    let mut encoded = String::with_capacity(value.as_bytes().len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write;
        write!(encoded, "{byte:02x}").unwrap();
    }
    encoded
}

fn compare(relative_mode: &str, inputs: &[String]) -> Vec<u8> {
    let c_output = invoke(C_LIBRARY, relative_mode, inputs);
    let rust_output = invoke(RUST_LIBRARY, relative_mode, inputs);
    assert_eq!(rust_output, c_output);
    c_output
}

#[test]
fn symbols_all_c_exports_are_loadable_from_both_libraries() {
    let libraries = Libraries::load();
    for symbol in SYMBOLS {
        unsafe {
            libraries
                .c
                .get::<*const c_void>(symbol)
                .expect("C library is missing an expected symbol");
            libraries
                .rust
                .get::<*const c_void>(symbol)
                .expect("Rust library is missing an expected symbol");
        }
    }
}

#[test]
fn configs_row_1_print_line_non_null_randomized() {
    let mut rng = FixedRng::new(0x9d1f_2a33_e84c_761b);
    let mut strings = vec![
        CString::new("").unwrap(),
        CString::new("plain text").unwrap(),
        CString::new("100% literal").unwrap(),
        CString::new("two\nlines").unwrap(),
    ];

    for _ in 0..512 {
        let length = (rng.next_u64() % 257) as usize;
        let bytes = (0..length)
            .map(|_| ((rng.next_u64() % 255) + 1) as u8)
            .collect::<Vec<_>>();
        strings.push(CString::new(bytes).unwrap());
    }

    let inputs = strings.iter().map(encode_hex).collect::<Vec<_>>();
    compare("printLine", &inputs);
}

#[test]
fn configs_row_2_print_int_line_randomized_full_range() {
    let mut rng = FixedRng::new(0x42a9_0ec7_b551_10d3);
    let mut inputs = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    inputs.extend((0..2048).map(|_| rng.next_u64() as u32 as i32));
    let inputs = inputs
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

    compare("printIntLine", &inputs);
}

#[test]
fn configs_row_3_bad_direct_call() {
    assert_eq!(compare("bad", &[]), b"0\n0\n");
}

#[test]
fn configs_row_4_good_direct_call() {
    assert_eq!(compare("good", &[]), b"0\n2\n");
}

#[test]
fn configs_row_5_driver_composed_call() {
    assert_eq!(
        compare("driver", &[]),
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n"
    );
}

#[test]
fn errors_row_1_print_line_null_writes_nothing() {
    assert!(compare("printLineNull", &[]).is_empty());
}
