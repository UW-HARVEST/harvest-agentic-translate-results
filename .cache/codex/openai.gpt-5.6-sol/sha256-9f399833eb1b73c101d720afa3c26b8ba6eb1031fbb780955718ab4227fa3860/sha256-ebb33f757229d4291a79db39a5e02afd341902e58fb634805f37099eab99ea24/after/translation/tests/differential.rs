use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

type SearchAndReplace =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;
type FailAllocArm = unsafe extern "C" fn(i32, u32);

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Loaded {
    _library: Library,
    search_and_replace: SearchAndReplace,
}

impl Loaded {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let search_and_replace = unsafe {
            *library
                .get::<SearchAndReplace>(b"searchAndReplace\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load searchAndReplace from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            search_and_replace,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation has a parent directory")
        .join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

unsafe fn call(
    function: SearchAndReplace,
    orig: &CString,
    search: &CString,
    value: &CString,
) -> Option<Vec<u8>> {
    let result = unsafe { function(orig.as_ptr(), search.as_ptr(), value.as_ptr()) };
    if result.is_null() {
        return None;
    }

    let bytes = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
    unsafe { free(result.cast()) };
    Some(bytes)
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, min: usize, max_inclusive: usize) -> usize {
        min + (self.next_u64() as usize % (max_inclusive - min + 1))
    }
}

fn safe_piece(rng: &mut Rng, min: usize, max: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnpqrstuvwyz0123456789";
    let length = rng.range(min, max);
    (0..length)
        .map(|_| ALPHABET[rng.range(0, ALPHABET.len() - 1)] as char)
        .collect()
}

fn replacement(rng: &mut Rng, empty: bool) -> String {
    if empty {
        String::new()
    } else {
        let mut value = String::from("R");
        value.push_str(&safe_piece(rng, 0, 12));
        value
    }
}

fn adjacent(search: &str, count: usize) -> String {
    search.repeat(count)
}

fn gapped(rng: &mut Rng, search: &str, count: usize) -> String {
    let mut result = String::new();
    for index in 0..count {
        if index > 0 {
            result.push_str(&safe_piece(rng, 1, 9));
        }
        result.push_str(search);
    }
    result
}

fn edge_piece(rng: &mut Rng) -> String {
    safe_piece(rng, 1, 16)
}

fn random_adjacent(rng: &mut Rng, search: &str) -> String {
    let count = rng.range(2, 8);
    adjacent(search, count)
}

fn random_gapped(rng: &mut Rng, search: &str) -> String {
    let count = rng.range(2, 8);
    gapped(rng, search, count)
}

fn case_for_row(row: usize, rng: &mut Rng) -> (String, String, String) {
    let one = "x";
    let many = "XY";

    match row {
        1 => (String::new(), one.into(), String::new()),
        2 => (String::new(), many.into(), replacement(rng, false)),
        3 => (safe_piece(rng, 1, 48), one.into(), String::new()),
        4 => (safe_piece(rng, 1, 48), many.into(), replacement(rng, false)),
        5 => (one.into(), one.into(), String::new()),
        6 => (one.into(), one.into(), replacement(rng, false)),
        7 => (
            format!("{many}{}", edge_piece(rng)),
            many.into(),
            String::new(),
        ),
        8 => (
            format!("{many}{}", edge_piece(rng)),
            many.into(),
            replacement(rng, false),
        ),
        9 => (
            format!("{}{one}", edge_piece(rng)),
            one.into(),
            String::new(),
        ),
        10 => (
            format!("{}{one}", edge_piece(rng)),
            one.into(),
            replacement(rng, false),
        ),
        11 => (
            format!("{}{many}{}", edge_piece(rng), edge_piece(rng)),
            many.into(),
            String::new(),
        ),
        12 => (
            format!("{}{many}{}", edge_piece(rng), edge_piece(rng)),
            many.into(),
            replacement(rng, false),
        ),
        13 => (random_adjacent(rng, one), one.into(), String::new()),
        14 => (
            random_adjacent(rng, one),
            one.into(),
            replacement(rng, false),
        ),
        15 => (
            format!(
                "{}{}{}",
                edge_piece(rng),
                random_adjacent(rng, many),
                edge_piece(rng)
            ),
            many.into(),
            String::new(),
        ),
        16 => (
            format!(
                "{}{}{}",
                edge_piece(rng),
                random_adjacent(rng, many),
                edge_piece(rng)
            ),
            many.into(),
            replacement(rng, false),
        ),
        17 => (random_gapped(rng, one), one.into(), String::new()),
        18 => (random_gapped(rng, one), one.into(), replacement(rng, false)),
        19 => (
            format!("{}{}", random_gapped(rng, many), edge_piece(rng)),
            many.into(),
            String::new(),
        ),
        20 => (
            format!("{}{}", random_gapped(rng, many), edge_piece(rng)),
            many.into(),
            replacement(rng, false),
        ),
        21 => (
            format!("{}{}", edge_piece(rng), random_gapped(rng, one)),
            one.into(),
            String::new(),
        ),
        22 => (
            format!("{}{}", edge_piece(rng), random_gapped(rng, one)),
            one.into(),
            replacement(rng, false),
        ),
        23 => (
            format!(
                "{}{}{}",
                edge_piece(rng),
                random_gapped(rng, many),
                edge_piece(rng)
            ),
            many.into(),
            String::new(),
        ),
        24 => (
            format!(
                "{}{}{}",
                edge_piece(rng),
                random_gapped(rng, many),
                edge_piece(rng)
            ),
            many.into(),
            replacement(rng, false),
        ),
        _ => panic!("unknown CONFIGS.md row {row}"),
    }
}

fn verify_config_row(row: usize) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    let c = unsafe { Loaded::open(&c_path) };
    let rust = unsafe { Loaded::open(&rust_path) };
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173 ^ row as u64);

    for iteration in 0..128 {
        let (orig, search, value) = case_for_row(row, &mut rng);
        let orig_c = CString::new(orig.as_bytes()).unwrap();
        let search_c = CString::new(search.as_bytes()).unwrap();
        let value_c = CString::new(value.as_bytes()).unwrap();

        let c_result = unsafe { call(c.search_and_replace, &orig_c, &search_c, &value_c) };
        let rust_result = unsafe { call(rust.search_and_replace, &orig_c, &search_c, &value_c) };
        assert_eq!(
            c_result, rust_result,
            "CONFIGS.md row {row}, iteration {iteration}: orig={orig:?}, search={search:?}, value={value:?}"
        );
    }
}

macro_rules! config_tests {
    ($(($name:ident, $row:literal)),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                verify_config_row($row);
            }
        )+
    };
}

config_tests!(
    (config_row_01, 1),
    (config_row_02, 2),
    (config_row_03, 3),
    (config_row_04, 4),
    (config_row_05, 5),
    (config_row_06, 6),
    (config_row_07, 7),
    (config_row_08, 8),
    (config_row_09, 9),
    (config_row_10, 10),
    (config_row_11, 11),
    (config_row_12, 12),
    (config_row_13, 13),
    (config_row_14, 14),
    (config_row_15, 15),
    (config_row_16, 16),
    (config_row_17, 17),
    (config_row_18, 18),
    (config_row_19, 19),
    (config_row_20, 20),
    (config_row_21, 21),
    (config_row_22, 22),
    (config_row_23, 23),
    (config_row_24, 24),
);

fn preload_path() -> PathBuf {
    manifest_dir().join("target/fail_alloc.so")
}

fn compile_preload() {
    let output = Command::new("cc")
        .current_dir(manifest_dir())
        .args([
            "-shared",
            "-fPIC",
            "tests/support/fail_alloc.c",
            "-o",
            "target/fail_alloc.so",
        ])
        .output()
        .expect("failed to execute cc for allocation-failure shim");
    assert!(
        output.status.success(),
        "allocation-failure shim compilation failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(preload_path().is_file());
}

#[test]
fn error_allocation_child() {
    let Ok(row_text) = std::env::var("DIFF_ALLOC_ROW") else {
        return;
    };
    let row: usize = row_text.parse().unwrap();
    let (kind, nth, orig, search, value) = match row {
        1 => (3, 1, "abcdef", "X", "R"),
        2 => (1, 1, "prefixX", "X", "R"),
        3 => (2, 1, "X", "X", "R"),
        4 => (2, 2, "prefixXgapX", "X", "R"),
        5 => (2, 2, "Xsuffix", "X", "R"),
        _ => panic!("unknown allocation error row {row}"),
    };

    let orig = CString::new(orig).unwrap();
    let search = CString::new(search).unwrap();
    let value = CString::new(value).unwrap();
    let process = libloading::os::unix::Library::this();
    let arm = unsafe {
        *process
            .get::<FailAllocArm>(b"fail_alloc_arm\0")
            .expect("LD_PRELOAD shim did not export fail_alloc_arm")
    };

    for path in [c_library_path(), rust_library_path()] {
        let library = unsafe { Loaded::open(&path) };
        unsafe { arm(kind, nth) };
        let result =
            unsafe { (library.search_and_replace)(orig.as_ptr(), search.as_ptr(), value.as_ptr()) };
        assert!(
            result.is_null(),
            "ERRORS.md row {row}: {} did not return NULL",
            path.display()
        );
    }
}

#[test]
fn error_rows_01_through_05_allocation_failures() {
    compile_preload();
    let executable = std::env::current_exe().unwrap();
    for row in 1..=5 {
        let output = Command::new(&executable)
            .arg("--exact")
            .arg("error_allocation_child")
            .arg("--nocapture")
            .env("DIFF_ALLOC_ROW", row.to_string())
            .env("LD_PRELOAD", preload_path())
            .env("LD_BIND_NOW", "1")
            .output()
            .unwrap_or_else(|error| panic!("failed to spawn allocation row {row}: {error}"));
        assert!(
            output.status.success(),
            "ERRORS.md allocation row {row} failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn boundary_child() {
    let Ok(case) = std::env::var("DIFF_BOUNDARY_CASE") else {
        return;
    };
    let library_kind = std::env::var("DIFF_LIBRARY").unwrap();
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown library kind {library_kind}"),
    };
    let library = unsafe { Loaded::open(&path) };
    let text = CString::new("abc").unwrap();
    let search = CString::new("a").unwrap();
    let empty = CString::new("").unwrap();
    let value = CString::new("R").unwrap();

    unsafe {
        match case.as_str() {
            "null_orig" => {
                (library.search_and_replace)(std::ptr::null(), search.as_ptr(), value.as_ptr());
            }
            "null_search" => {
                (library.search_and_replace)(text.as_ptr(), std::ptr::null(), value.as_ptr());
            }
            "null_value" => {
                (library.search_and_replace)(text.as_ptr(), search.as_ptr(), std::ptr::null());
            }
            "empty_search" => {
                (library.search_and_replace)(text.as_ptr(), empty.as_ptr(), empty.as_ptr());
            }
            _ => panic!("unknown boundary case {case}"),
        }
    }
}

fn spawn_boundary(library: &str, case: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("boundary_child")
        .arg("--nocapture")
        .env("DIFF_LIBRARY", library)
        .env("DIFF_BOUNDARY_CASE", case)
        .spawn()
        .unwrap()
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[test]
fn error_rows_06_through_08_null_pointers() {
    for (row, case) in [(6, "null_orig"), (7, "null_search"), (8, "null_value")] {
        let mut statuses = Vec::new();
        for library in ["c", "rust"] {
            let mut child = spawn_boundary(library, case);
            let status =
                wait_with_timeout(&mut child, Duration::from_secs(5)).unwrap_or_else(|| {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("ERRORS.md row {row}: {library} did not terminate")
                });
            statuses.push(status);
        }
        assert_eq!(signal(statuses[0]), Some(11), "C row {row}");
        assert_eq!(
            signal(statuses[1]),
            signal(statuses[0]),
            "ERRORS.md row {row}: Rust and C terminated differently"
        );
    }
}

#[test]
fn error_row_09_empty_search_does_not_return() {
    for library in ["c", "rust"] {
        let mut child = spawn_boundary(library, "empty_search");
        let status = wait_with_timeout(&mut child, Duration::from_millis(250));
        assert!(
            status.is_none(),
            "ERRORS.md row 9: {library} unexpectedly returned with {status:?}"
        );
        child.kill().unwrap();
        child.wait().unwrap();
    }
}

#[test]
fn dynamic_symbol_is_loadable_from_both_libraries() {
    unsafe {
        let _c = Loaded::open(&c_library_path());
        let _rust = Loaded::open(&rust_library_path());
    }
}
