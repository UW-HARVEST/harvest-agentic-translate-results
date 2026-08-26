use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

type Replace = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Loaded {
    library: Library,
}

impl Loaded {
    fn open(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    fn call(&self, orig: &[u8], search: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        assert_eq!(orig.last(), Some(&0));
        assert_eq!(search.last(), Some(&0));
        assert_eq!(value.last(), Some(&0));
        unsafe {
            let function: Symbol<'_, Replace> = self
                .library
                .get(b"searchAndReplace\0")
                .expect("missing searchAndReplace");
            let output = function(
                orig.as_ptr().cast(),
                search.as_ptr().cast(),
                value.as_ptr().cast(),
            );
            if output.is_null() {
                return None;
            }
            let bytes = CStr::from_ptr(output).to_bytes_with_nul().to_vec();
            free(output.cast());
            Some(bytes)
        }
    }
}

struct Pair {
    c: Loaded,
    rust: Loaded,
}

impl Pair {
    fn new() -> Self {
        Self {
            c: Loaded::open(&c_library_path()),
            rust: Loaded::open(&rust_library_path()),
        }
    }

    fn compare(&self, orig: Vec<u8>, search: Vec<u8>, value: Vec<u8>) {
        let c = self.c.call(&orig, &search, &value);
        let rust = self.rust.call(&orig, &search, &value);
        assert_eq!(
            rust,
            c,
            "orig={:?}, search={:?}, value={:?}",
            visible(&orig),
            visible(&search),
            visible(&value)
        );
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn range(&mut self, low: usize, high: usize) -> usize {
        low + (self.next() as usize % (high - low))
    }

    fn filler(&mut self, min: usize, max: usize) -> Vec<u8> {
        let len = self.range(min, max + 1);
        (0..len).map(|_| b'a' + self.range(0, 26) as u8).collect()
    }

    fn token(&mut self) -> Vec<u8> {
        let mut token = vec![b'#'];
        let len = self.range(1, 6);
        token.extend((0..len).map(|_| b'A' + self.range(0, 26) as u8));
        token.push(b'#');
        token
    }
}

fn z(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

fn visible(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())]
}

fn joined(parts: &[&[u8]]) -> Vec<u8> {
    let mut result = Vec::new();
    for part in parts {
        result.extend_from_slice(part);
    }
    result
}

fn randomized(seed: u64, mut case: impl FnMut(&mut Rng, usize) -> (Vec<u8>, Vec<u8>, Vec<u8>)) {
    let pair = Pair::new();
    let mut rng = Rng::new(seed);
    for iteration in 0..96 {
        let (orig, search, value) = case(&mut rng, iteration);
        pair.compare(z(orig), z(search), z(value));
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("current test executable")
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .join("libdriver.so")
}

#[test]
fn config_01_empty_orig_no_match() {
    randomized(0x0101, |rng, iteration| {
        let value = if iteration % 2 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 12)
        };
        (Vec::new(), rng.token(), value)
    });
}

#[test]
fn config_02_nonempty_orig_no_match() {
    randomized(0x0202, |rng, iteration| {
        let mut orig = rng.filler(1, 30);
        let search = rng.token();
        let value_len = match iteration % 3 {
            0 => 1,
            1 => search.len(),
            _ => search.len() + 7,
        };
        if iteration % 8 == 0 {
            orig.push(0);
            orig.extend_from_slice(&search);
        }
        (orig, search, rng.filler(value_len, value_len))
    });
}

#[test]
fn config_03_whole_match_empty_replacement() {
    randomized(0x0303, |rng, _| {
        let search = rng.token();
        (search.clone(), search, Vec::new())
    });
}

#[test]
fn config_04_whole_match_nonempty_replacement() {
    randomized(0x0404, |rng, iteration| {
        let search = rng.token();
        let value_len = match iteration % 3 {
            0 => 1,
            1 => search.len(),
            _ => search.len() + 9,
        };
        (search.clone(), search, rng.filler(value_len, value_len))
    });
}

#[test]
fn config_05_one_match_at_start_with_suffix() {
    randomized(0x0505, |rng, iteration| {
        let search = rng.token();
        let suffix = rng.filler(1, 20);
        let value = if iteration % 4 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 15)
        };
        (joined(&[&search, &suffix]), search, value)
    });
}

#[test]
fn config_06_one_match_in_middle() {
    randomized(0x0606, |rng, iteration| {
        let search = rng.token();
        let prefix = rng.filler(1, 20);
        let suffix = rng.filler(1, 20);
        let value = if iteration % 4 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 15)
        };
        (joined(&[&prefix, &search, &suffix]), search, value)
    });
}

#[test]
fn config_07_one_match_at_end() {
    randomized(0x0707, |rng, iteration| {
        let search = rng.token();
        let prefix = rng.filler(1, 20);
        let value = if iteration % 4 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 15)
        };
        (joined(&[&prefix, &search]), search, value)
    });
}

#[test]
fn config_08_adjacent_matches_at_start() {
    randomized(0x0808, |rng, iteration| {
        let search = rng.token();
        let mut orig = Vec::new();
        for _ in 0..rng.range(2, 7) {
            orig.extend_from_slice(&search);
        }
        let value = if iteration % 5 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 12)
        };
        (orig, search, value)
    });
}

#[test]
fn config_09_prefixed_adjacent_matches() {
    randomized(0x0909, |rng, iteration| {
        let search = rng.token();
        let mut orig = rng.filler(1, 15);
        for _ in 0..rng.range(2, 7) {
            orig.extend_from_slice(&search);
        }
        let value = if iteration % 5 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 12)
        };
        (orig, search, value)
    });
}

fn separated_case(
    rng: &mut Rng,
    iteration: usize,
    prefix: bool,
    suffix: bool,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let search = rng.token();
    let mut orig = if prefix {
        rng.filler(1, 15)
    } else {
        Vec::new()
    };
    let count = rng.range(2, 6);
    for index in 0..count {
        orig.extend_from_slice(&search);
        if index + 1 < count {
            orig.extend_from_slice(&rng.filler(1, 12));
        }
    }
    if suffix {
        orig.extend_from_slice(&rng.filler(1, 15));
    }
    let value = if iteration % 5 == 0 {
        Vec::new()
    } else {
        rng.filler(1, 12)
    };
    (orig, search, value)
}

#[test]
fn config_10_separated_at_start_no_suffix() {
    randomized(0x1010, |rng, iteration| {
        separated_case(rng, iteration, false, false)
    });
}

#[test]
fn config_11_prefixed_separated_no_suffix() {
    randomized(0x1111, |rng, iteration| {
        separated_case(rng, iteration, true, false)
    });
}

#[test]
fn config_12_separated_at_start_with_suffix() {
    randomized(0x1212, |rng, iteration| {
        separated_case(rng, iteration, false, true)
    });
}

#[test]
fn config_13_prefixed_separated_with_suffix() {
    randomized(0x1313, |rng, iteration| {
        separated_case(rng, iteration, true, true)
    });
}

#[test]
fn config_14_overlapping_candidates() {
    randomized(0x1414, |rng, iteration| {
        let byte = b'A' + rng.range(0, 26) as u8;
        let search_len = rng.range(2, 6);
        let search = vec![byte; search_len];
        let orig_len = search_len + rng.range(1, search_len * 4);
        let value = if iteration % 5 == 0 {
            Vec::new()
        } else {
            rng.filler(1, 10)
        };
        (vec![byte; orig_len], search, value)
    });
}

fn spawn_helper(operation: &str, library: &Path, argument: &str, preload: Option<&Path>) -> Child {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .arg("--exact")
        .arg("subprocess_entry")
        .arg("--nocapture")
        .env("DIFF_CHILD_OPERATION", operation)
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_ARGUMENT", argument)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(preload) = preload {
        command.env("LD_PRELOAD", preload);
    }
    command.spawn().expect("spawn differential helper")
}

fn wait_for_marker(child: &mut Child, marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() {
        if let Some(status) = child.try_wait().expect("query helper") {
            panic!("helper exited before call boundary: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "helper did not reach call boundary"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn marker_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{label}",
        std::process::id()
    ))
}

fn nonterminating_status(library: &Path, argument: &str, label: &str) -> &'static str {
    let marker = marker_path(label);
    let _ = fs::remove_file(&marker);
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    let mut child = command
        .arg("--exact")
        .arg("subprocess_entry")
        .arg("--nocapture")
        .env("DIFF_CHILD_OPERATION", "nonterm")
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_ARGUMENT", argument)
        .env("DIFF_CHILD_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn nontermination helper");
    wait_for_marker(&mut child, &marker);
    thread::sleep(Duration::from_millis(75));
    let state = if child
        .try_wait()
        .expect("query nontermination helper")
        .is_none()
    {
        child.kill().expect("kill nonterminating helper");
        let _ = child.wait();
        "still-running"
    } else {
        "returned"
    };
    let _ = fs::remove_file(marker);
    state
}

#[test]
fn config_15_empty_search_does_not_terminate() {
    let mut rng = Rng::new(0x1515);
    for iteration in 0..16 {
        let orig = if iteration % 2 == 0 {
            ""
        } else {
            // Leaking the short generated value is acceptable in this test process.
            Box::leak(
                String::from_utf8(rng.filler(1, 12))
                    .expect("ASCII")
                    .into_boxed_str(),
            )
        };
        let argument = format!("{orig}:");
        let c = nonterminating_status(&c_library_path(), &argument, &format!("c-{iteration}"));
        let rust = nonterminating_status(
            &rust_library_path(),
            &argument,
            &format!("rust-{iteration}"),
        );
        assert_eq!(c, "still-running");
        assert_eq!(rust, c);
    }
}

fn null_status(library: &Path, argument: &str, label: &str) -> ExitStatus {
    let marker = marker_path(label);
    let _ = fs::remove_file(&marker);
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    let mut child = command
        .arg("--exact")
        .arg("subprocess_entry")
        .arg("--nocapture")
        .env("DIFF_CHILD_OPERATION", "null")
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_ARGUMENT", argument)
        .env("DIFF_CHILD_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn null helper");
    wait_for_marker(&mut child, &marker);
    let status = child.wait().expect("wait for null helper");
    let _ = fs::remove_file(marker);
    status
}

#[test]
fn errors_05_through_07_null_arguments() {
    for (index, argument) in ["orig", "search", "value"].into_iter().enumerate() {
        let c = null_status(&c_library_path(), argument, &format!("null-c-{index}"));
        let rust = null_status(
            &rust_library_path(),
            argument,
            &format!("null-rust-{index}"),
        );
        assert_eq!(c.signal(), Some(11), "C {argument}: {c}");
        assert_eq!(rust.signal(), c.signal(), "Rust {argument}: {rust}");
    }
}

fn fail_alloc_library() -> &'static Path {
    static SHIM: OnceLock<PathBuf> = OnceLock::new();
    SHIM.get_or_init(|| {
        let output =
            std::env::temp_dir().join(format!("libdriver_fail_alloc_{}.so", std::process::id()));
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fail_alloc.c");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(source)
            .arg("-o")
            .arg(&output)
            .status()
            .expect("compile allocation interposer");
        assert!(status.success(), "allocation interposer compilation failed");
        output
    })
}

fn allocation_failure_status(library: &Path, row: usize) -> ExitStatus {
    spawn_helper(
        "alloc",
        library,
        &row.to_string(),
        Some(fail_alloc_library()),
    )
    .wait()
    .expect("wait for allocation helper")
}

#[test]
fn errors_01_through_04_allocation_failures_return_null() {
    for row in 1..=4 {
        let c = allocation_failure_status(&c_library_path(), row);
        let rust = allocation_failure_status(&rust_library_path(), row);
        assert!(c.success(), "C allocation row {row}: {c}");
        assert_eq!(rust.code(), c.code(), "Rust allocation row {row}: {rust}");
    }
}

#[test]
fn subprocess_entry() {
    let Ok(operation) = std::env::var("DIFF_CHILD_OPERATION") else {
        return;
    };
    let path = PathBuf::from(std::env::var_os("DIFF_CHILD_LIBRARY").expect("library path"));
    let argument = std::env::var("DIFF_CHILD_ARGUMENT").expect("child argument");

    match operation.as_str() {
        "nonterm" => {
            let (orig, value) = argument.split_once(':').expect("nonterm arguments");
            let loaded = Loaded::open(&path);
            let orig = z(orig.as_bytes().to_vec());
            let search = z(Vec::new());
            let value = z(value.as_bytes().to_vec());
            fs::write(
                std::env::var_os("DIFF_CHILD_MARKER").expect("marker path"),
                b"ready",
            )
            .expect("write marker");
            let _ = loaded.call(&orig, &search, &value);
        }
        "null" => {
            let loaded = Loaded::open(&path);
            let function: Symbol<'_, Replace> = unsafe {
                loaded
                    .library
                    .get(b"searchAndReplace\0")
                    .expect("missing searchAndReplace")
            };
            let string = b"x\0";
            let null = std::ptr::null();
            fs::write(
                std::env::var_os("DIFF_CHILD_MARKER").expect("marker path"),
                b"ready",
            )
            .expect("write marker");
            unsafe {
                match argument.as_str() {
                    "orig" => function(null, string.as_ptr().cast(), string.as_ptr().cast()),
                    "search" => function(string.as_ptr().cast(), null, string.as_ptr().cast()),
                    "value" => function(string.as_ptr().cast(), string.as_ptr().cast(), null),
                    _ => panic!("unknown null argument"),
                };
            }
        }
        "alloc" => {
            let row: usize = argument.parse().expect("allocation row");
            let shim_path = PathBuf::from(std::env::var_os("LD_PRELOAD").expect("preload library"));
            let shim = Loaded::open(&shim_path);
            let arm: Symbol<'_, unsafe extern "C" fn(u64)> =
                unsafe { shim.library.get(b"fail_alloc_arm\0").expect("arm symbol") };
            let disarm: Symbol<'_, unsafe extern "C" fn()> = unsafe {
                shim.library
                    .get(b"fail_alloc_disarm\0")
                    .expect("disarm symbol")
            };
            let loaded = Loaded::open(&path);
            let function: Symbol<'_, Replace> = unsafe {
                loaded
                    .library
                    .get(b"searchAndReplace\0")
                    .expect("missing searchAndReplace")
            };
            let (orig, search, value, fail_at): (&[u8], &[u8], &[u8], u64) = match row {
                1 => (b"ax\0", b"x\0", b"z\0", 1),
                2 => (b"x\0", b"x\0", b"z\0", 1),
                3 => (b"xax\0", b"x\0", b"z\0", 2),
                4 => (b"xa\0", b"x\0", b"z\0", 2),
                _ => panic!("unknown allocation row"),
            };
            let output = unsafe {
                arm(fail_at);
                let output = function(
                    orig.as_ptr().cast(),
                    search.as_ptr().cast(),
                    value.as_ptr().cast(),
                );
                disarm();
                output
            };
            std::process::exit(if output.is_null() { 0 } else { 2 });
        }
        _ => panic!("unknown child operation"),
    }
}
