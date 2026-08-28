use libloading::{Library, Symbol};
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

type SearchAndReplace =
    unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;
type FailAllocArm = unsafe extern "C" fn(usize);

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct APIs {
    _c: Library,
    _rust: Library,
    c: SearchAndReplace,
    rust: SearchAndReplace,
}

impl APIs {
    unsafe fn load() -> Self {
        let c = unsafe { Library::new(c_library_path()) }.expect("load C shared library");
        let rust = unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library");
        let c_fn: Symbol<SearchAndReplace> =
            unsafe { c.get(b"searchAndReplace\0") }.expect("load C symbol");
        let rust_fn: Symbol<SearchAndReplace> =
            unsafe { rust.get(b"searchAndReplace\0") }.expect("load Rust symbol");
        let c_fn = *c_fn;
        let rust_fn = *rust_fn;
        Self {
            _c: c,
            _rust: rust,
            c: c_fn,
            rust: rust_fn,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C library must be built before tests")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
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

    let bytes = unsafe { CStr::from_ptr(result) }
        .to_bytes_with_nul()
        .to_vec();
    unsafe { free(result.cast()) };
    Some(bytes)
}

fn compare_case(apis: &APIs, row: usize, orig: Vec<u8>, search: Vec<u8>, value: Vec<u8>) {
    let orig = CString::new(orig).expect("generated orig contains no NUL");
    let search = CString::new(search).expect("generated search contains no NUL");
    let value = CString::new(value).expect("generated value contains no NUL");

    let c = unsafe { call(apis.c, &orig, &search, &value) };
    let rust = unsafe { call(apis.rust, &orig, &search, &value) };
    assert_eq!(
        c, rust,
        "CONFIGS.md row {row} diverged for orig={orig:?}, search={search:?}, value={value:?}"
    );
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn bytes(&mut self, alphabet: &[u8], min: usize, max: usize) -> Vec<u8> {
        let len = min + self.next() as usize % (max - min + 1);
        (0..len)
            .map(|_| alphabet[self.next() as usize % alphabet.len()])
            .collect()
    }
}

#[test]
fn valid_configuration_matrix_matches() {
    let apis = unsafe { APIs::load() };
    let mut rng = Rng(0x6a09_e667_f3bc_c909);

    for iteration in 0..128 {
        let value = if iteration % 2 == 0 {
            Vec::new()
        } else {
            rng.bytes(b"0123456789", 1, 12)
        };
        compare_case(&apis, 1, Vec::new(), b"Q".to_vec(), value);

        let orig = rng.bytes(b"abcdef", 1, 64);
        let search = rng.bytes(b"QWERTY", 1, 8);
        let value = if iteration % 2 == 0 {
            Vec::new()
        } else {
            rng.bytes(b"0123456789", 1, 12)
        };
        compare_case(&apis, 2, orig, search, value);
    }

    let mut row = 3;
    for prefix in [false, true] {
        for later_match in 0..3 {
            for suffix in [false, true] {
                for nonempty_value in [false, true] {
                    for _ in 0..128 {
                        let search = rng.bytes(b"QWERTY", 1, 8);
                        let mut orig = Vec::new();
                        if prefix {
                            orig.extend(rng.bytes(b"abcdef", 1, 20));
                        }
                        orig.extend(&search);
                        match later_match {
                            0 => {}
                            1 => orig.extend(&search),
                            2 => {
                                orig.extend(rng.bytes(b"abcdef", 1, 20));
                                orig.extend(&search);
                            }
                            _ => unreachable!(),
                        }
                        if suffix {
                            orig.extend(rng.bytes(b"abcdef", 1, 20));
                        }
                        let value = if nonempty_value {
                            rng.bytes(b"0123456789", 1, 16)
                        } else {
                            Vec::new()
                        };
                        compare_case(&apis, row, orig, search.clone(), value);
                    }
                    row += 1;
                }
            }
        }
    }
    assert_eq!(row, 27);
}

#[test]
fn long_input_boundary_matches() {
    let apis = unsafe { APIs::load() };
    let mut orig = vec![b'a'; 128 * 1024];
    orig.extend(b"SEARCH");
    orig.extend(vec![b'b'; 128 * 1024]);
    compare_case(&apis, 26, orig, b"SEARCH".to_vec(), vec![b'v'; 64 * 1024]);
}

fn spawn_probe(kind: &str, probe: &str, argument: Option<&str>) -> std::process::Child {
    let mut command = Command::new(std::env::current_exe().expect("resolve test executable"));
    command
        .arg("--exact")
        .arg("ffi_probe")
        .arg("--nocapture")
        .env("FFI_PROBE", probe)
        .env("FFI_LIBRARY", kind)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(argument) = argument {
        command.env("FFI_ARGUMENT", argument);
    }
    command.spawn().expect("spawn FFI probe")
}

fn run_probe(kind: &str, probe: &str, argument: Option<&str>) -> ExitStatus {
    spawn_probe(kind, probe, argument)
        .wait()
        .expect("wait for FFI probe")
}

#[test]
fn empty_search_nontermination_matches() {
    for kind in ["c", "rust"] {
        let mut child = spawn_probe(kind, "empty_search", None);
        thread::sleep(Duration::from_millis(200));
        assert!(
            child.try_wait().expect("poll empty-search probe").is_none(),
            "{kind} returned for an empty search string"
        );
        child.kill().expect("kill nonterminating probe");
        child.wait().expect("reap nonterminating probe");
    }
}

#[test]
fn null_pointer_boundaries_match() {
    for argument in ["orig", "search", "value"] {
        let c = run_probe("c", "null_pointer", Some(argument));
        let rust = run_probe("rust", "null_pointer", Some(argument));
        assert_eq!(
            c.signal(),
            rust.signal(),
            "null {argument} produced different termination signals: C={c:?}, Rust={rust:?}"
        );
        assert!(
            c.signal().is_some() && rust.signal().is_some(),
            "null {argument} did not terminate both probes by signal: C={c:?}, Rust={rust:?}"
        );
    }
}

fn fault_injector_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let target =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-support/libfail_alloc.so");
        fs::create_dir_all(target.parent().expect("injector target has parent"))
            .expect("create injector output directory");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-O2", "-o"])
            .arg(&target)
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fail_alloc.c"))
            .status()
            .expect("compile allocation fault injector");
        assert!(
            status.success(),
            "allocation fault injector failed to build"
        );
        target
    })
}

fn allocation_failure_case(row: usize) {
    let status = Command::new(std::env::current_exe().expect("resolve test executable"))
        .arg("--exact")
        .arg("ffi_probe")
        .arg("--nocapture")
        .env("FFI_PROBE", "allocation_failure")
        .env("FFI_ARGUMENT", row.to_string())
        .env("LD_PRELOAD", fault_injector_path())
        .status()
        .expect("run allocation failure probe");
    assert!(
        status.success(),
        "ERRORS.md row {row} probe failed: {status}"
    );
}

#[test]
fn initial_malloc_failure_matches() {
    allocation_failure_case(1);
}

#[test]
fn replacement_realloc_failure_matches() {
    allocation_failure_case(2);
}

#[test]
fn gap_realloc_failure_matches() {
    allocation_failure_case(3);
}

#[test]
fn suffix_realloc_failure_matches() {
    allocation_failure_case(4);
}

#[test]
fn no_match_strdup_failure_matches() {
    allocation_failure_case(5);
}

#[test]
fn ffi_probe() {
    let Ok(probe) = std::env::var("FFI_PROBE") else {
        return;
    };

    let apis = unsafe { APIs::load() };
    if probe == "allocation_failure" {
        let row: usize = std::env::var("FFI_ARGUMENT")
            .expect("allocation row")
            .parse()
            .expect("numeric allocation row");
        let injector_path = std::env::var_os("LD_PRELOAD").expect("preloaded injector path");
        let injector = unsafe { Library::new(injector_path) }.expect("load injector");
        let arm: Symbol<FailAllocArm> =
            unsafe { injector.get(b"fail_alloc_arm\0") }.expect("load injector arm function");
        let (orig, search, value, fail_at) = match row {
            1 => (b"prefixX".as_slice(), b"X".as_slice(), b"v".as_slice(), 1),
            2 => (b"X".as_slice(), b"X".as_slice(), b"v".as_slice(), 1),
            3 => (b"XgapX".as_slice(), b"X".as_slice(), b"v".as_slice(), 2),
            4 => (b"Xsuffix".as_slice(), b"X".as_slice(), b"v".as_slice(), 2),
            5 => (b"abc".as_slice(), b"X".as_slice(), b"v".as_slice(), 1),
            _ => panic!("unknown ERRORS.md row {row}"),
        };
        let orig = CString::new(orig).unwrap();
        let search = CString::new(search).unwrap();
        let value = CString::new(value).unwrap();

        unsafe { arm(fail_at) };
        let c = unsafe { (apis.c)(orig.as_ptr(), search.as_ptr(), value.as_ptr()) };
        unsafe { arm(fail_at) };
        let rust = unsafe { (apis.rust)(orig.as_ptr(), search.as_ptr(), value.as_ptr()) };
        assert!(c.is_null(), "C did not return NULL for ERRORS.md row {row}");
        assert!(
            rust.is_null(),
            "Rust did not return NULL for ERRORS.md row {row}"
        );
        return;
    }

    let function = match std::env::var("FFI_LIBRARY").as_deref() {
        Ok("c") => apis.c,
        Ok("rust") => apis.rust,
        other => panic!("unknown FFI library {other:?}"),
    };
    if probe == "empty_search" {
        let orig = c"abc";
        let empty = c"";
        unsafe {
            function(orig.as_ptr(), empty.as_ptr(), empty.as_ptr());
        }
        panic!("empty search unexpectedly returned");
    }
    if probe == "null_pointer" {
        let text = c"abc";
        let null = std::ptr::null();
        let argument = std::env::var("FFI_ARGUMENT").expect("null argument name");
        unsafe {
            match argument.as_str() {
                "orig" => function(null, text.as_ptr(), text.as_ptr()),
                "search" => function(text.as_ptr(), null, text.as_ptr()),
                "value" => function(text.as_ptr(), text.as_ptr(), null),
                _ => panic!("unknown null argument {argument}"),
            };
        }
        panic!("null pointer unexpectedly returned");
    }
    panic!("unknown FFI probe {probe}");
}
