use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

type SieveFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES_PER_ROW: usize = 64;
const RANDOM_SEED: u64 = 0x5eed_cafe_d15c_a11e;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_sieve: SieveFn,
    rust_sieve: SieveFn,
}

impl Implementations {
    unsafe fn load() -> Self {
        let c_path = crate_root().join("c_src/build/libSieve.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "missing C shared library {}; build c_src first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library {}",
            rust_path.display()
        );

        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        let c_sieve = {
            let symbol: Symbol<'_, SieveFn> =
                unsafe { c_library.get(b"sieve\0") }.expect("C library does not export sieve");
            *symbol
        };
        let rust_sieve = {
            let symbol: Symbol<'_, SieveFn> = unsafe { rust_library.get(b"sieve\0") }
                .expect("Rust library does not export sieve");
            *symbol
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_sieve,
            rust_sieve,
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target_dir().join(profile).join("libSieve.so")
}

fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("target"))
}

fn capture_stdout(function: SieveFn, start: c_int) -> Vec<u8> {
    static STDOUT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = STDOUT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("stdout capture mutex poisoned");

    let mut pipe_fds = [-1; 2];
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush failed");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "dup2 capture failed"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close pipe writer failed");

        function(start);

        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "dup2 restore failed"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");
    }

    let mut output = Vec::new();
    unsafe { File::from_raw_fd(pipe_fds[0]) }
        .read_to_end(&mut output)
        .expect("failed to read captured stdout");
    output
}

fn assert_same_output(implementations: &Implementations, start: c_int) {
    let c_output = capture_stdout(implementations.c_sieve, start);
    let rust_output = capture_stdout(implementations.rust_sieve, start);
    assert_eq!(c_output, rust_output, "stdout differs for sieve({start})");
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

#[test]
fn all_configuration_rows_match_byte_for_byte() {
    let implementations = unsafe { Implementations::load() };
    let mut random_state = RANDOM_SEED;

    for residue in 0..=9 {
        // Exercise the low boundary and the highest non-overflowing decade.
        assert_same_output(&implementations, residue);
        assert_same_output(&implementations, 2_147_483_630 + residue);

        for _ in 0..RANDOM_CASES_PER_ROW {
            let decade = next_random(&mut random_state) % 100_000_000;
            let start = (decade * 10 + residue as u32) as c_int;
            assert_same_output(&implementations, start);
        }
    }

    // Negative signed remainders cannot equal positive 9. Include explicit
    // textually-ending-in-9 cases and a randomized bounded corpus.
    for start in [-1, -9, -10, -19, -99, -128, -255] {
        assert_same_output(&implementations, start);
    }
    for _ in 0..RANDOM_CASES_PER_ROW {
        let start = -((next_random(&mut random_state) % 512) as c_int + 1);
        assert_same_output(&implementations, start);
    }
}

#[test]
fn shared_library_paths_are_external_artifacts() {
    let root = crate_root();
    let c_path = root.join("c_src/build/libSieve.so");
    let rust_path = rust_library_path();

    assert!(c_path.is_file(), "{}", c_path.display());
    assert!(rust_path.is_file(), "{}", rust_path.display());
    assert_ne!(
        canonical_parent(&c_path),
        canonical_parent(&rust_path),
        "C and Rust libraries must be distinct artifacts"
    );
}

fn canonical_parent(path: &Path) -> PathBuf {
    path.parent()
        .expect("shared library has no parent")
        .canonicalize()
        .expect("failed to canonicalize shared library parent")
}
