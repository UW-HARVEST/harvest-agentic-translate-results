//! Shared harness: loads the C and Rust shared libraries side by side and
//! captures everything they write to stdout (fd 1) so the two can be compared
//! byte-for-byte.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void, CString};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Mirror of the C `house_t` layout: `int`, `int`, `double`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

impl std::fmt::Debug for house_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "house_t {{ floors: {}, bedrooms: {}, bathrooms: {:?} (bits {:#018x}) }}",
            self.floors,
            self.bedrooms,
            self.bathrooms,
            self.bathrooms.to_bits()
        )
    }
}

impl house_t {
    /// Raw bytes of the struct, including any padding, for exact comparison.
    pub fn raw(&self) -> [u8; std::mem::size_of::<house_t>()] {
        unsafe { std::mem::transmute_copy(self) }
    }
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

/// Which implementation to invoke.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Impl {
    C,
    Rust,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// `target/<profile>/` — derived from the running test executable, which lives
/// in `target/<profile>/deps/`.
fn rust_artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = rust_artifact_dir();
    let candidate = dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to the sibling profile directory if the current one has no
    // cdylib (e.g. when tests are run in a different profile than the build).
    for profile in ["debug", "release"] {
        let alt = workspace_root()
            .join("translation/target")
            .join(profile)
            .join("libdriver.so");
        if alt.exists() {
            return alt;
        }
    }
    candidate
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_lib_path();
        let rust_path = rust_lib_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {c_path:?}; build it with cmake first"
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {rust_path:?}; run `cargo build` first"
        );
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("load C libdriver.so"),
                rust: Library::new(&rust_path).expect("load Rust libdriver.so"),
            }
        }
    })
}

/// fd 1 redirection is process-global, so captured calls must be serialised.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with fd 1 redirected into a temporary file and returns everything
/// that was written there. Both implementations print through the process-wide
/// libc `stdout`, so all streams are flushed before and after.
///
/// Note: fd 1 is process-global and libtest also writes its progress there, so
/// each test binary must contain exactly one `#[test]` (see the test files) and
/// Rust's own buffered stdout is flushed before the redirection is installed.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::io::AsRawFd;

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "driver_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create capture file");

    let mut buf = Vec::new();
    // Push out anything libtest (or we) buffered in Rust's stdout so it lands
    // on the real fd 1 rather than in the capture file.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut buf).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&tmp_path);
    buf
}

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(*mut house_t, c_int);

/// Calls the exported `driver` symbol of the chosen library. `input` is passed
/// as a NUL-terminated byte string exactly as given.
pub fn call_driver(which: Impl, input: &[u8]) -> Vec<u8> {
    let l = libs();
    let lib = match which {
        Impl::C => &l.c,
        Impl::Rust => &l.rust,
    };
    let sym: Symbol<DriverFn> = unsafe { lib.get(b"driver\0").expect("driver symbol") };
    let cstr = CString::new(input).expect("input contains an interior NUL");
    capture_stdout(|| unsafe { sym(cstr.as_ptr()) })
}

/// Calls the exported `run` symbol of the chosen library, returning the printed
/// output alongside the (mutated) struct.
pub fn call_run(which: Impl, house: house_t, extra_bedrooms: c_int) -> (Vec<u8>, house_t) {
    let l = libs();
    let lib = match which {
        Impl::C => &l.c,
        Impl::Rust => &l.rust,
    };
    let sym: Symbol<RunFn> = unsafe { lib.get(b"run\0").expect("run symbol") };
    let mut h = house;
    let out = capture_stdout(|| unsafe { sym(&mut h, extra_bedrooms) });
    (out, h)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

extern "C" {
    fn __errno_location() -> *mut c_int;
}

/// Calls `driver` and reports the printed output together with the value of
/// `errno` observed immediately afterwards. `errno` is seeded with `seed` first
/// so it is visible whether the callee resets it.
///
/// `errno` is read inside the redirection window, before any of the harness's
/// own syscalls can clobber it.
pub fn call_driver_errno(which: Impl, input: &[u8], seed: c_int) -> (Vec<u8>, c_int) {
    let l = libs();
    let lib = match which {
        Impl::C => &l.c,
        Impl::Rust => &l.rust,
    };
    let sym: Symbol<DriverFn> = unsafe { lib.get(b"driver\0").expect("driver symbol") };
    let cstr = CString::new(input).expect("input contains an interior NUL");
    let mut observed: c_int = 0;
    let out = capture_stdout(|| unsafe {
        *__errno_location() = seed;
        sym(cstr.as_ptr());
        observed = *__errno_location();
    });
    (out, observed)
}

/// Asserts that C and Rust `driver` agree for `input`.
pub fn assert_driver_matches(input: &[u8]) {
    let c_out = call_driver(Impl::C, input);
    let rust_out = call_driver(Impl::Rust, input);
    if c_out != rust_out {
        panic!(
            "driver({:?}) mismatch\n--- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}",
            show(input),
            c_out.len(),
            show(&c_out),
            rust_out.len(),
            show(&rust_out)
        );
    }
}

/// Asserts that C and Rust `run` agree for the given struct and argument, both
/// in printed output and in the resulting struct bytes.
pub fn assert_run_matches(house: house_t, extra_bedrooms: c_int) {
    let (c_out, c_house) = call_run(Impl::C, house, extra_bedrooms);
    let (rust_out, rust_house) = call_run(Impl::Rust, house, extra_bedrooms);
    if c_out != rust_out {
        panic!(
            "run({house:?}, {extra_bedrooms}) output mismatch\n--- C ---\n{}\n--- Rust ---\n{}",
            show(&c_out),
            show(&rust_out)
        );
    }
    if c_house.raw() != rust_house.raw() {
        panic!(
            "run({house:?}, {extra_bedrooms}) struct mismatch: C {c_house:?} vs Rust {rust_house:?}"
        );
    }
}
