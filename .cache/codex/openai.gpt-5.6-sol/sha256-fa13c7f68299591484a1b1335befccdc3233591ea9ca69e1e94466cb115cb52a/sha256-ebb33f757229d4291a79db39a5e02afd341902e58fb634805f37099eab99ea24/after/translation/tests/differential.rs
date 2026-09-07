use libloading::Library;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Dataentry = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

struct APIs {
    _c_library: Library,
    _rust_library: Library,
    c: Dataentry,
    rust: Dataentry,
}

impl APIs {
    fn load() -> Self {
        let c_library = unsafe { Library::new(c_library_path()).expect("load C shared library") };
        let rust_library =
            unsafe { Library::new(rust_library_path()).expect("load Rust shared library") };
        let c = unsafe {
            *c_library
                .get::<Dataentry>(b"dataentry\0")
                .expect("load C dataentry")
        };
        let rust = unsafe {
            *rust_library
                .get::<Dataentry>(b"dataentry\0")
                .expect("load Rust dataentry")
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c,
            rust,
        }
    }

    #[track_caller]
    fn compare(&self, args: [i32; 4]) -> i32 {
        let c_result = unsafe { (self.c)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (self.rust)(args[0], args[1], args[2], args[3]) };
        assert_eq!(
            c_result, rust_result,
            "FFI results differ for dataentry({},{},{},{})",
            args[0], args[1], args[2], args[3]
        );
        c_result
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C .so in {}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdataentry_lib.so")
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
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

    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn range(&mut self, start: i32, end: i32) -> i32 {
        assert!(start < end);
        start + (self.next_u64() % (end - start) as u64) as i32
    }

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 != 0
    }
}

fn exercise(iterations: usize, seed: u64, mut make_args: impl FnMut(&mut Rng) -> [i32; 4]) {
    let apis = APIs::load();
    let mut rng = Rng::new(seed);
    for _ in 0..iterations {
        apis.compare(make_args(&mut rng));
    }
}

// CONFIGS.md rows 1-6: mode 1.

#[test]
fn config_01_mode1_fallback_hit() {
    exercise(512, 0x0101, |rng| {
        let count_selector = if rng.bool() { 0 } else { -rng.range(1, 10_000) };
        [1, count_selector, rng.range(0, 5), rng.i32()]
    });
    let apis = APIs::load();
    apis.compare([1, i32::MIN, 0, i32::MIN]);
}

#[test]
fn config_02_mode1_fallback_miss() {
    exercise(512, 0x0102, |rng| {
        let count_selector = if rng.bool() { 0 } else { -rng.range(1, 10_000) };
        let target = if rng.bool() {
            -rng.range(1, 10_000)
        } else {
            rng.range(5, 10_000)
        };
        [1, count_selector, target, rng.i32()]
    });
    let apis = APIs::load();
    assert_eq!(apis.compare([1, i32::MIN, i32::MIN, i32::MAX]), -2);
    assert_eq!(apis.compare([1, 0, i32::MAX, i32::MIN]), -2);
}

#[test]
fn config_03_mode1_one_hit() {
    exercise(512, 0x0103, |rng| [1, 1, 0, rng.i32()]);
}

#[test]
fn config_04_mode1_one_miss() {
    exercise(512, 0x0104, |rng| {
        let target = if rng.bool() {
            -rng.range(1, 10_000)
        } else {
            rng.range(1, 10_000)
        };
        [1, 1, target, rng.i32()]
    });
}

#[test]
fn config_05_mode1_many_hit() {
    exercise(512, 0x0105, |rng| {
        let count = rng.range(2, 65);
        [1, count, rng.range(0, count), rng.i32()]
    });
}

#[test]
fn config_06_mode1_many_miss() {
    exercise(512, 0x0106, |rng| {
        let count = rng.range(2, 65);
        let target = if rng.bool() {
            -rng.range(1, 10_000)
        } else {
            count + rng.range(0, 10_000)
        };
        [1, count, target, rng.i32()]
    });
}

// CONFIGS.md rows 7-12: mode 2.

#[test]
fn config_07_mode2_fallback_zero_multiplier() {
    exercise(512, 0x0207, |rng| {
        let count_selector = if rng.bool() { 0 } else { -rng.range(1, 10_000) };
        [2, count_selector, 0, rng.i32()]
    });
    let apis = APIs::load();
    assert_eq!(apis.compare([2, i32::MIN, 0, i32::MIN]), 0);
}

#[test]
fn config_08_mode2_fallback_nonzero_multiplier() {
    exercise(512, 0x0208, |rng| {
        let count_selector = if rng.bool() { 0 } else { -rng.range(1, 10_000) };
        let multiplier = if rng.bool() {
            rng.range(1, 1_001)
        } else {
            -rng.range(1, 1_001)
        };
        [2, count_selector, multiplier, rng.i32()]
    });
    let apis = APIs::load();
    apis.compare([2, 0, i32::MAX, i32::MIN]);
}

#[test]
fn config_09_mode2_one_zero_multiplier() {
    exercise(512, 0x0209, |rng| [2, 1, 0, rng.i32()]);
    let apis = APIs::load();
    assert_eq!(apis.compare([2, 1, 0, i32::MAX]), 0);
}

#[test]
fn config_10_mode2_one_nonzero_multiplier() {
    exercise(512, 0x0210, |rng| {
        let multiplier = if rng.bool() {
            rng.range(1, 1_001)
        } else {
            -rng.range(1, 1_001)
        };
        [2, 1, multiplier, rng.i32()]
    });
    let apis = APIs::load();
    apis.compare([2, 1, i32::MAX, i32::MIN]);
}

#[test]
fn config_11_mode2_many_zero_multiplier() {
    exercise(512, 0x0211, |rng| [2, rng.range(2, 65), 0, rng.i32()]);
    let apis = APIs::load();
    assert_eq!(apis.compare([2, 64, 0, i32::MIN]), 0);
}

#[test]
fn config_12_mode2_many_nonzero_multiplier() {
    exercise(512, 0x0212, |rng| {
        let multiplier = if rng.bool() {
            rng.range(1, 1_001)
        } else {
            -rng.range(1, 1_001)
        };
        [2, rng.range(2, 65), multiplier, rng.i32()]
    });
    let apis = APIs::load();
    apis.compare([2, 64, i32::MAX, i32::MIN]);
}

#[test]
fn config_13_mode2_fallback_nonzero_multiplier_zero_total() {
    exercise(512, 0x0213, |rng| {
        let count_selector = if rng.bool() { 0 } else { -rng.range(1, 10_000) };
        [2, count_selector, i32::MIN, rng.i32()]
    });
}

#[test]
fn config_14_mode2_one_nonzero_multiplier_zero_total() {
    exercise(512, 0x0214, |rng| [2, 1, i32::MIN, rng.i32()]);
}

#[test]
fn config_15_mode2_many_nonzero_multiplier_zero_total() {
    exercise(512, 0x0215, |rng| {
        [2, rng.range(2, 65), i32::MIN, rng.i32()]
    });
}

// CONFIGS.md rows 16-27: every lookup coordinate.

fn exercise_lookup(row: i32, col: i32, seed: u64) {
    exercise(512, seed, |rng| [3, row, col, rng.i32()]);
    let apis = APIs::load();
    apis.compare([3, row, col, i32::MIN]);
    apis.compare([3, row, col, i32::MAX]);
}

macro_rules! lookup_test {
    ($name:ident, $row:expr, $col:expr, $seed:expr) => {
        #[test]
        fn $name() {
            exercise_lookup($row, $col, $seed);
        }
    };
}

lookup_test!(config_16_mode3_row0_col0, 0, 0, 0x0316);
lookup_test!(config_17_mode3_row0_col1, 0, 1, 0x0317);
lookup_test!(config_18_mode3_row0_col2, 0, 2, 0x0318);
lookup_test!(config_19_mode3_row1_col0, 1, 0, 0x0319);
lookup_test!(config_20_mode3_row1_col1, 1, 1, 0x0320);
lookup_test!(config_21_mode3_row1_col2, 1, 2, 0x0321);
lookup_test!(config_22_mode3_row2_col0, 2, 0, 0x0322);
lookup_test!(config_23_mode3_row2_col1, 2, 1, 0x0323);
lookup_test!(config_24_mode3_row2_col2, 2, 2, 0x0324);
lookup_test!(config_25_mode3_row3_col0, 3, 0, 0x0325);
lookup_test!(config_26_mode3_row3_col1, 3, 1, 0x0326);
lookup_test!(config_27_mode3_row3_col2, 3, 2, 0x0327);

#[test]
fn config_28_default_mode() {
    exercise(2_048, 0x0428, |rng| {
        let mode = loop {
            let candidate = rng.i32();
            if !matches!(candidate, 1..=3) {
                break candidate;
            }
        };
        [mode, rng.i32(), rng.i32(), rng.i32()]
    });
    let apis = APIs::load();
    for mode in [i32::MIN, -1, 0, 4, i32::MAX] {
        apis.compare([mode, i32::MIN, i32::MAX, i32::MIN]);
        apis.compare([mode, i32::MAX, i32::MIN, i32::MAX]);
    }
}

// ERRORS.md rows 1-7.

fn parse_worker_result(output: &[u8]) -> i32 {
    let stdout = String::from_utf8_lossy(output);
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("DIFF_RESULT="))
        .unwrap_or_else(|| panic!("worker result missing from output:\n{stdout}"))
        .parse()
        .expect("worker result is an i32")
}

fn allocation_worker_result(which: &str) -> i32 {
    let executable = env::current_exe().expect("current integration-test executable");
    let output = Command::new("sh")
        .arg("-c")
        .arg(
            "ulimit -v 65536; exec \"$DIFF_TEST_EXE\" allocation_failure_worker --exact --nocapture",
        )
        .env("DIFF_TEST_EXE", executable)
        .env("DIFF_ALLOC_WORKER", which)
        .output()
        .expect("run allocation-failure worker");
    assert!(
        output.status.success(),
        "allocation worker {which} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_worker_result(&output.stdout)
}

fn load_one(path: &Path) -> (Library, Dataentry) {
    let library = unsafe { Library::new(path).expect("load worker shared library") };
    let function = unsafe {
        *library
            .get::<Dataentry>(b"dataentry\0")
            .expect("load worker dataentry")
    };
    (library, function)
}

#[test]
fn allocation_failure_worker() {
    let Ok(which) = env::var("DIFF_ALLOC_WORKER") else {
        return;
    };
    let (path, mode) = match which.as_str() {
        "c-mode1" => (c_library_path(), 1),
        "rust-mode1" => (rust_library_path(), 1),
        "c-mode2" => (c_library_path(), 2),
        "rust-mode2" => (rust_library_path(), 2),
        _ => panic!("unknown allocation worker: {which}"),
    };
    let (_library, function) = load_one(&path);
    let result = unsafe { function(mode, i32::MAX, 1, 1) };
    println!("DIFF_RESULT={result}");
}

#[test]
fn error_01_mode1_allocation_failure() {
    let c_result = allocation_worker_result("c-mode1");
    let rust_result = allocation_worker_result("rust-mode1");
    assert_eq!(c_result, -1);
    assert_eq!(rust_result, c_result);
}

#[test]
fn error_02_mode1_missing_target() {
    let apis = APIs::load();
    let mut rng = Rng::new(0xe002);
    for _ in 0..1_024 {
        let count_selector = if rng.bool() { 0 } else { rng.range(1, 65) };
        let count = if count_selector > 0 {
            count_selector
        } else {
            5
        };
        let target = if rng.bool() {
            -rng.range(1, 100_000)
        } else {
            count + rng.range(0, 100_000)
        };
        assert_eq!(apis.compare([1, count_selector, target, rng.i32()]), -2);
    }
    assert_eq!(apis.compare([1, 1, i32::MIN, i32::MAX]), -2);
    assert_eq!(apis.compare([1, 1, i32::MAX, i32::MIN]), -2);
}

#[test]
fn error_03_mode2_allocation_failure() {
    let c_result = allocation_worker_result("c-mode2");
    let rust_result = allocation_worker_result("rust-mode2");
    assert_eq!(c_result, -1);
    assert_eq!(rust_result, c_result);
}

#[test]
fn error_04_mode3_negative_row() {
    exercise(1_024, 0xe004, |rng| {
        [3, -rng.range(1, i32::MAX), rng.range(0, 3), rng.i32()]
    });
    let apis = APIs::load();
    for row in [i32::MIN, -1] {
        assert_eq!(apis.compare([3, row, 0, i32::MAX]), 0);
    }
}

#[test]
fn error_05_mode3_row_too_large() {
    let apis = APIs::load();
    let mut rng = Rng::new(0xe005);
    for _ in 0..1_024 {
        let row = rng.range(4, i32::MAX);
        assert_eq!(apis.compare([3, row, rng.range(0, 3), rng.i32()]), 0);
    }
    assert_eq!(apis.compare([3, i32::MAX, 0, i32::MIN]), 0);
}

#[test]
fn error_06_mode3_negative_column() {
    let apis = APIs::load();
    let mut rng = Rng::new(0xe006);
    for _ in 0..1_024 {
        let column = -rng.range(1, i32::MAX);
        assert_eq!(apis.compare([3, rng.range(0, 4), column, rng.i32()]), 0);
    }
    assert_eq!(apis.compare([3, 0, i32::MIN, i32::MAX]), 0);
}

#[test]
fn error_07_mode3_column_too_large() {
    let apis = APIs::load();
    let mut rng = Rng::new(0xe007);
    for _ in 0..1_024 {
        let column = rng.range(3, i32::MAX);
        assert_eq!(apis.compare([3, rng.range(0, 4), column, rng.i32()]), 0);
    }
    assert_eq!(apis.compare([3, 0, i32::MAX, i32::MIN]), 0);
}

#[test]
fn exported_symbol_is_available_from_both_shared_libraries() {
    let _apis = APIs::load();
}
