use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type SieveFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static TEMP_COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();

struct LoadedSieve {
    _library: Library,
    function: SieveFn,
}

impl LoadedSieve {
    fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let function = unsafe {
            let symbol: Symbol<'_, SieveFn> = library
                .get(b"sieve\0")
                .unwrap_or_else(|error| panic!("missing sieve in {}: {error}", path.display()));
            *symbol
        };
        Self {
            _library: library,
            function,
        }
    }

    fn call_and_capture(&self, input: c_int) -> Vec<u8> {
        let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
        let path = next_temp_path();
        let mut output = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|error| {
                panic!("failed to create capture file {}: {error}", path.display())
            });

        unsafe {
            assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-call fflush failed");
        }
        let saved_stdout = unsafe { dup(1) };
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            unsafe { dup2(output.as_raw_fd(), 1) },
            1,
            "redirecting stdout failed"
        );

        unsafe {
            (self.function)(input);
            assert_eq!(fflush(std::ptr::null_mut()), 0, "post-call fflush failed");
        }

        assert_eq!(
            unsafe { dup2(saved_stdout, 1) },
            1,
            "restoring stdout failed"
        );
        assert_eq!(unsafe { close(saved_stdout) }, 0, "closing dup failed");

        output.seek(SeekFrom::Start(0)).expect("capture seek failed");
        let mut bytes = Vec::new();
        output
            .read_to_end(&mut bytes)
            .expect("capture read failed");
        drop(output);
        std::fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));
        bytes
    }
}

fn next_temp_path() -> PathBuf {
    let counter = TEMP_COUNTER.get_or_init(|| Mutex::new(0));
    let mut value = counter.lock().expect("temp counter poisoned");
    *value += 1;
    std::env::temp_dir().join(format!(
        "sieve-differential-{}-{}.out",
        std::process::id(),
        *value
    ))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_SIEVE_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../c_src/build/libSieve.so")
        })
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_SIEVE_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libSieve.so")
        })
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn assert_matches(c: &LoadedSieve, rust: &LoadedSieve, row: usize, input: c_int) {
    let c_output = c.call_and_capture(input);
    let rust_output = rust.call_and_capture(input);
    assert_eq!(
        rust_output, c_output,
        "CONFIGS.md row {row} diverged for input {input}"
    );
}

#[test]
fn valid_path_configuration_matrix_matches_byte_for_byte() {
    let c = LoadedSieve::open(&c_library_path());
    let rust = LoadedSieve::open(&rust_library_path());
    let mut random = 0x5eed_c0de_d1ff_e7a1_u64;

    // CONFIGS.md row 1: negative values must count through zero to positive 9.
    for input in [-1, -2, -9, -10, -99, -200] {
        assert_matches(&c, &rust, 1, input);
    }
    for _ in 0..64 {
        let input = -((next_random(&mut random) % 200) as c_int + 1);
        assert_matches(&c, &rust, 1, input);
    }

    // CONFIGS.md rows 2-11: every nonnegative decimal residue class.
    for residue in (0 as c_int)..=(9 as c_int) {
        let row = if residue == 9 {
            2
        } else {
            residue as usize + 3
        };

        assert_matches(&c, &rust, row, residue);
        assert_matches(&c, &rust, row, 2_147_483_630 + residue);

        for _ in 0..64 {
            let decade = (next_random(&mut random) % 10_000_000) as c_int;
            let input = decade * 10 + residue;
            assert_matches(&c, &rust, row, input);
        }
    }
}
