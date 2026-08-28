use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

type SieveFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static NEXT_CAPTURE_ID: AtomicU64 = AtomicU64::new(0);
const STDOUT_FD: c_int = 1;

struct StdoutRedirect<'a> {
    saved_fd: c_int,
    _lock: MutexGuard<'a, ()>,
}

impl Drop for StdoutRedirect<'_> {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.saved_fd, STDOUT_FD), STDOUT_FD);
            assert_eq!(close(self.saved_fd), 0);
        }
    }
}

fn shared_library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest.join("../c_src/build/libSieve.so");
    let rust_library = manifest.join("target/release/libSieve.so");

    assert!(
        c_library.is_file(),
        "missing C library: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "missing Rust library: {}",
        rust_library.display()
    );
    (c_library, rust_library)
}

fn temporary_capture_file() -> (PathBuf, File) {
    let id = NEXT_CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "sieve-differential-{}-{id}.out",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture file");
    (path, file)
}

fn capture_calls(function: &SieveFn, inputs: &[c_int]) -> Vec<u8> {
    let lock = STDOUT_LOCK.lock().expect("lock stdout redirection");
    let (path, mut output) = temporary_capture_file();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_fd = unsafe { dup(STDOUT_FD) };
    assert!(saved_fd >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(output.as_raw_fd(), STDOUT_FD) }, STDOUT_FD);

    {
        let redirect = StdoutRedirect {
            saved_fd,
            _lock: lock,
        };
        for &input in inputs {
            unsafe { function(input) };
        }
        drop(redirect);
    }

    output.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read capture");
    drop(output);
    remove_file(path).expect("remove capture file");
    bytes
}

fn assert_row_matches(c_sieve: &SieveFn, rust_sieve: &SieveFn, row: u8, inputs: &[c_int]) {
    let c_output = capture_calls(c_sieve, inputs);
    let rust_output = capture_calls(rust_sieve, inputs);
    assert_eq!(
        rust_output, c_output,
        "CONFIGS.md row {row} diverged for inputs {inputs:?}"
    );
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

#[test]
fn every_configuration_matches_through_shared_library_ffi() {
    let (c_path, rust_path) = shared_library_paths();
    let c_library = unsafe { Library::new(c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(rust_path) }.expect("load Rust shared library");
    let c_sieve: Symbol<SieveFn> = unsafe { c_library.get(b"sieve\0") }.expect("load C sieve");
    let rust_sieve: Symbol<SieveFn> =
        unsafe { rust_library.get(b"sieve\0") }.expect("load Rust sieve");

    let mut state = 0x5eed_c0de_d15c_a11u64;

    let mut immediate = Vec::with_capacity(258);
    immediate.extend([9, 2_147_483_639]);
    for _ in 0..256 {
        let decade = (next_random(&mut state) % 100_000_000) as c_int;
        immediate.push(decade * 10 + 9);
    }
    assert_row_matches(&c_sieve, &rust_sieve, 1, &immediate);

    let mut nonnegative = Vec::with_capacity(259);
    nonnegative.extend([0, 8, 10, 2_147_483_638]);
    for _ in 0..255 {
        let decade = (next_random(&mut state) % 100_000_000) as c_int;
        let remainder = (next_random(&mut state) % 9) as c_int;
        nonnegative.push(decade * 10 + remainder);
    }
    assert_row_matches(&c_sieve, &rust_sieve, 2, &nonnegative);

    let mut negative = Vec::with_capacity(258);
    negative.extend([-1, -9]);
    for _ in 0..256 {
        negative.push(-((next_random(&mut state) % 512) as c_int + 1));
    }
    assert_row_matches(&c_sieve, &rust_sieve, 3, &negative);
}
