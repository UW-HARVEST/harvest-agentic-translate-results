use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type Cleanup = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type PrintResult = unsafe extern "C" fn(*const c_char, c_int);
type CleanupResources = unsafe extern "C" fn(*mut c_char);
type ArmFault = unsafe extern "C" fn();

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES_PER_ROW: usize = 64;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    cleanup: Cleanup,
    print_result: PrintResult,
    cleanup_resources: CleanupResources,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: The paths identify the two shared libraries built for this test.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        // SAFETY: Phase A established the exact exported C signatures.
        let cleanup = unsafe { *library.get::<Cleanup>(b"cleanup\0").unwrap() };
        // SAFETY: Phase A established the exact exported C signatures.
        let print_result = unsafe { *library.get::<PrintResult>(b"print_result\0").unwrap() };
        // SAFETY: Phase A established the exact exported C signatures.
        let cleanup_resources = unsafe {
            *library
                .get::<CleanupResources>(b"cleanup_resources\0")
                .unwrap()
        };
        Self {
            _library: library,
            cleanup,
            print_result,
            cleanup_resources,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    unsafe fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C library is missing; build it first: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust library is missing; run `cargo build --release` first: {}",
            rust_path.display()
        );
        Self {
            // SAFETY: Paths and signatures are validated above and in Phase A.
            c: unsafe { Api::load(&c_path) },
            // SAFETY: Paths and signatures are validated above and in Phase A.
            rust: unsafe { Api::load(&rust_path) },
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn default_value(&mut self) -> c_int {
        loop {
            let value = (self.next_u64() % 2_000_001) as c_int - 1_000_000;
            if !matches!(value, 10 | 20 | 30 | 40) {
                return value;
            }
        }
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = (self.next_u64() as usize) % (index + 1);
            values.swap(index, other);
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libcleanup_lib.so")
}

fn interposer_source_path() -> PathBuf {
    manifest_dir().join("tests/ffi_faults.c")
}

fn interposer_library_path() -> PathBuf {
    manifest_dir().join("target/test-support/libffi_faults.so")
}

fn build_interposer() -> PathBuf {
    let output = interposer_library_path();
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(interposer_source_path())
        .status()
        .expect("failed to invoke cc for the fault interposer");
    assert!(status.success(), "fault interposer compilation failed");
    output
}

unsafe fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let mut pipe_fds = [-1; 2];
    // SAFETY: pipe_fds points to storage for two descriptors.
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    // SAFETY: A null stream flushes every open output stream.
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    // SAFETY: STDOUT_FILENO is an open descriptor in the test process.
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    // SAFETY: Both descriptors are valid.
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    // SAFETY: The duplicated write descriptor is no longer needed.
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let result = call();

    // SAFETY: Flush captured C stdio before restoring stdout.
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    // SAFETY: Restore the saved stdout descriptor.
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    // SAFETY: The saved duplicate is no longer needed.
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    // SAFETY: Ownership of the pipe's read descriptor moves into File.
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    (result, bytes)
}

unsafe fn call_cleanup(function: Cleanup, values: [c_int; 4]) -> (c_int, Vec<u8>) {
    // SAFETY: The loaded symbol has the declared ABI and accepts four integers.
    unsafe { capture_stdout(|| function(values[0], values[1], values[2], values[3])) }
}

unsafe fn compare_cleanup(libraries: &Libraries, values: [c_int; 4], context: &str) {
    // SAFETY: Both symbols were loaded with the same C ABI signature.
    let (c_result, c_stdout) = unsafe { call_cleanup(libraries.c.cleanup, values) };
    // SAFETY: Both symbols were loaded with the same C ABI signature.
    let (rust_result, rust_stdout) = unsafe { call_cleanup(libraries.rust.cleanup, values) };
    assert_eq!(
        rust_result, c_result,
        "{context}: return mismatch for {values:?}"
    );
    assert_eq!(
        rust_stdout, c_stdout,
        "{context}: stdout mismatch for {values:?}"
    );
}

fn cleanup_class_rows() -> Vec<[usize; 5]> {
    let mut rows = Vec::new();
    for default_count in (0..=4).rev() {
        for ten_count in (0..=4 - default_count).rev() {
            for twenty_count in (0..=4 - default_count - ten_count).rev() {
                for thirty_count in (0..=4 - default_count - ten_count - twenty_count).rev() {
                    let forty_count = 4 - default_count - ten_count - twenty_count - thirty_count;
                    rows.push([
                        default_count,
                        ten_count,
                        twenty_count,
                        thirty_count,
                        forty_count,
                    ]);
                }
            }
        }
    }
    assert_eq!(rows.len(), 70);
    rows
}

#[test]
fn phase_b_cleanup_all_70_configuration_rows() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    // SAFETY: Library paths and Phase A signatures are checked by Libraries::load.
    let libraries = unsafe { Libraries::load() };
    let mut rng = Rng(0x5eed_c0de_d15c_a11e);

    for (row_index, counts) in cleanup_class_rows().into_iter().enumerate() {
        for case_index in 0..RANDOM_CASES_PER_ROW {
            let mut values = Vec::with_capacity(4);
            for _ in 0..counts[0] {
                values.push(rng.default_value());
            }
            for (count, value) in counts[1..].iter().zip([10, 20, 30, 40]) {
                values.extend(std::iter::repeat_n(value, *count));
            }
            rng.shuffle(&mut values);
            let values: [c_int; 4] = values.try_into().unwrap();
            // SAFETY: Both loaded functions accept all c_int bit patterns.
            unsafe {
                compare_cleanup(
                    &libraries,
                    values,
                    &format!("CONFIGS.md row {}, case {}", row_index + 1, case_index),
                );
            }
        }
    }
}

#[test]
fn phase_b_print_result_configuration_row_71() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    // SAFETY: Library paths and Phase A signatures are checked by Libraries::load.
    let libraries = unsafe { Libraries::load() };
    let labels: &[&[u8]] = &[
        b"",
        b"label",
        b"100% literal %s %d",
        &[0xff, 0x80, b'A', b'%', b'Z'],
    ];
    let results = [0, 1, -1, c_int::MIN, c_int::MAX];

    for label in labels {
        let label = CString::new(*label).unwrap();
        for result in results {
            // SAFETY: The label is NUL-terminated and both symbols have the declared ABI.
            let (_, c_stdout) =
                unsafe { capture_stdout(|| (libraries.c.print_result)(label.as_ptr(), result)) };
            // SAFETY: The label is NUL-terminated and both symbols have the declared ABI.
            let (_, rust_stdout) =
                unsafe { capture_stdout(|| (libraries.rust.print_result)(label.as_ptr(), result)) };
            assert_eq!(rust_stdout, c_stdout, "label={label:?}, result={result}");
        }
    }
}

#[test]
fn phase_b_cleanup_resources_rows_72_and_73() {
    // SAFETY: Library paths and Phase A signatures are checked by Libraries::load.
    let libraries = unsafe { Libraries::load() };

    // SAFETY: Null is explicitly accepted by the C implementation.
    unsafe {
        (libraries.c.cleanup_resources)(ptr::null_mut());
        (libraries.rust.cleanup_resources)(ptr::null_mut());
    }

    // SAFETY: Each allocation is passed once to the corresponding free-compatible export.
    unsafe {
        let c_allocation = malloc(64).cast::<c_char>();
        let rust_allocation = malloc(64).cast::<c_char>();
        assert!(!c_allocation.is_null());
        assert!(!rust_allocation.is_null());
        (libraries.c.cleanup_resources)(c_allocation);
        (libraries.rust.cleanup_resources)(rust_allocation);
    }
}

#[test]
fn phase_c_generic_boundaries() {
    let _lock = STDOUT_LOCK.lock().unwrap();
    // SAFETY: Library paths and Phase A signatures are checked by Libraries::load.
    let libraries = unsafe { Libraries::load() };

    for values in [
        [0, 0, 0, 0],
        [c_int::MIN, 0, 0, 0],
        [c_int::MAX, 0, 0, 0],
        [-1, 1, -2, 2],
    ] {
        // SAFETY: Both loaded functions accept all c_int bit patterns.
        unsafe { compare_cleanup(&libraries, values, "generic integer boundary") };
    }

    // SAFETY: This intentionally exercises the platform C behavior for a null %s.
    let (_, c_stdout) = unsafe { capture_stdout(|| (libraries.c.print_result)(ptr::null(), -7)) };
    // SAFETY: This intentionally exercises the same platform C behavior through Rust.
    let (_, rust_stdout) =
        unsafe { capture_stdout(|| (libraries.rust.print_result)(ptr::null(), -7)) };
    assert_eq!(rust_stdout, c_stdout);
    assert_eq!(c_stdout, b"(null): -7\n");
}

#[test]
fn phase_c_fault_injected_error_rows_1_and_2() {
    if std::env::var_os("DIFFERENTIAL_FAULT_CHILD").is_none() {
        let interposer = build_interposer();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "phase_c_fault_injected_error_rows_1_and_2",
                "--nocapture",
            ])
            .env("DIFFERENTIAL_FAULT_CHILD", "1")
            .env("LD_PRELOAD", &interposer)
            .output()
            .expect("failed to launch fault-injected test child");
        assert!(
            output.status.success(),
            "fault child failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let _lock = STDOUT_LOCK.lock().unwrap();
    // SAFETY: Library paths and Phase A signatures are checked by Libraries::load.
    let libraries = unsafe { Libraries::load() };
    let interposer_path = interposer_library_path();
    // SAFETY: The same library is already loaded through LD_PRELOAD in this child.
    let interposer = unsafe { Library::new(&interposer_path) }.unwrap();
    // SAFETY: ffi_faults.c exports both functions with these exact signatures.
    let arm_mismatch = unsafe {
        *interposer
            .get::<ArmFault>(b"arm_mismatch_next_strncmp\0")
            .unwrap()
    };
    // SAFETY: ffi_faults.c exports both functions with these exact signatures.
    let arm_malloc_failure = unsafe {
        *interposer
            .get::<ArmFault>(b"arm_fail_next_50_byte_malloc\0")
            .unwrap()
    };

    // ERRORS.md row 1: force the otherwise internal-only validation mismatch.
    let (c_result, c_stdout) = unsafe {
        capture_stdout(|| {
            arm_mismatch();
            (libraries.c.cleanup)(1, 2, 3, 4)
        })
    };
    let (rust_result, rust_stdout) = unsafe {
        capture_stdout(|| {
            arm_mismatch();
            (libraries.rust.cleanup)(1, 2, 3, 4)
        })
    };
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_stdout, c_stdout);
    assert_eq!(c_result, 0);
    assert_eq!(c_stdout, b"Input string validation failed.\n");

    // ERRORS.md row 2: fail exactly the operation's 50-byte allocation.
    let (c_result, c_stdout) = unsafe {
        capture_stdout(|| {
            arm_malloc_failure();
            (libraries.c.cleanup)(10, 20, 30, 40)
        })
    };
    let (rust_result, rust_stdout) = unsafe {
        capture_stdout(|| {
            arm_malloc_failure();
            (libraries.rust.cleanup)(10, 20, 30, 40)
        })
    };
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_stdout, c_stdout);
    assert_eq!(c_result, 160);
    assert_eq!(c_stdout, b"Memory allocation failed.\n");
}
