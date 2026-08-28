//! Shared harness: loads the C and Rust shared objects side by side and
//! exposes their exported symbols through identical function-pointer types.
//!
//! Every call into the Rust implementation goes through `dlopen`/`dlsym` on the
//! built cdylib, so the `#[no_mangle]` export wrappers are exercised exactly as
//! an external C caller would exercise them.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Mirror of the C `StringBuffer` layout so tests can inspect the fields of a
/// buffer produced by either implementation.
#[repr(C)]
#[derive(Debug)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

pub type CreateBufferFn = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
pub type AppendToBufferFn = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
pub type DestroyBufferFn = unsafe extern "C" fn(*mut StringBuffer);
pub type GetOperationNameFn = unsafe extern "C" fn(c_int) -> *const c_char;
pub type PerformOperationFn = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
pub type BuffappFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C or the Rust `.so`).
pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub create_buffer: CreateBufferFn,
    pub append_to_buffer: AppendToBufferFn,
    pub destroy_buffer: DestroyBufferFn,
    pub get_operation_name: GetOperationNameFn,
    pub perform_operation: PerformOperationFn,
    pub buffapp: BuffappFn,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        // SAFETY: the paths point at the two artifacts built from this repo.
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));

            fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
                // SAFETY: symbol types are declared to match the C header.
                unsafe {
                    let s: Symbol<T> = lib.get(name).unwrap_or_else(|e| {
                        panic!("missing symbol {}: {e}", String::from_utf8_lossy(name))
                    });
                    *s
                }
            }

            let create_buffer = sym::<CreateBufferFn>(&lib, b"create_buffer\0");
            let append_to_buffer = sym::<AppendToBufferFn>(&lib, b"append_to_buffer\0");
            let destroy_buffer = sym::<DestroyBufferFn>(&lib, b"destroy_buffer\0");
            let get_operation_name = sym::<GetOperationNameFn>(&lib, b"get_operation_name\0");
            let perform_operation = sym::<PerformOperationFn>(&lib, b"perform_operation\0");
            let buffapp = sym::<BuffappFn>(&lib, b"buffapp\0");

            Impl {
                _lib: lib,
                name,
                create_buffer,
                append_to_buffer,
                destroy_buffer,
                get_operation_name,
                perform_operation,
                buffapp,
            }
        }
    }
}

fn repo_root() -> PathBuf {
    // Cargo runs tests with CWD set to the package root (`translation/`).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nbuild the C library first: \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "so"))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build_dir.display()
    );
    found.pop().unwrap()
}

fn rust_so_path() -> PathBuf {
    // An explicit override lets the sweep script point the same tests at a
    // specific artifact (e.g. the release cdylib it just built).
    let (path, autobuild) = match std::env::var("RUST_SO_PATH") {
        Ok(p) => (PathBuf::from(p), false),
        Err(_) => {
            // Resolve the cdylib belonging to *this* test binary's profile:
            // current_exe is target/<profile>/deps/<test>, so the artifact sits
            // two levels up.
            let exe = std::env::current_exe().expect("current_exe");
            let profile_dir = exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("target/<profile>/deps/<test> layout")
                .to_path_buf();
            (profile_dir.join("libbuffapp_lib.so"), true)
        }
    };

    if autobuild {
        // `cargo test` only builds the crate types the test binaries link
        // against (the rlib); it does NOT refresh the cdylib. Without this
        // step the tests would dlopen a stale .so and pass even when
        // src/lib.rs has regressed.
        ensure_cdylib_built(&path);
    }

    assert!(
        path.exists(),
        "no cdylib at {}; run `cargo build` (or set RUST_SO_PATH)",
        path.display()
    );
    assert_not_stale(&path);
    path
}

static BUILD_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn ensure_cdylib_built(so: &Path) {
    BUILD_ONCE.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Match the profile of the currently running test binary.
        let release = so.parent().is_some_and(|p| p.ends_with("release"));

        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.current_dir(&manifest).arg("build").arg("--lib");
        if release {
            cmd.arg("--release");
        }
        // Building the lib cannot re-enter the test harness, so there is no
        // recursion risk here.
        let out = cmd.output().expect("failed to spawn cargo build --lib");
        assert!(
            out.status.success(),
            "cargo build --lib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

/// Fail loudly if the cdylib predates the source it is supposed to be built
/// from. This is the backstop that turns a silent false pass into an error.
fn assert_not_stale(so: &Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("lib.rs");
    let (Ok(so_m), Ok(src_m)) = (
        std::fs::metadata(so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) else {
        return;
    };
    assert!(
        so_m >= src_m,
        "STALE ARTIFACT: {} is older than {}. The tests would be comparing C \
         against an out-of-date Rust library. Run `cargo build` (and \
         `cargo build --release`) before testing.",
        so.display(),
        src.display()
    );
}

/// Both implementations, ready for differential comparison.
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", &c_so_path()),
        rs: Impl::load("Rust", &rust_so_path()),
    }
}

/// Path of the C shared library under test.
pub fn c_so_path_pub() -> PathBuf {
    c_so_path()
}

/// Path of the Rust cdylib under test.
pub fn rust_so_path_pub() -> PathBuf {
    rust_so_path()
}

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Read a NUL-terminated C string as raw bytes (no UTF-8 assumptions).
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null(), "unexpected NULL C string");
    unsafe {
        let mut out = Vec::new();
        let mut i = 0isize;
        loop {
            let b = *p.offset(i) as u8;
            if b == 0 {
                break;
            }
            out.push(b);
            i += 1;
        }
        out
    }
}

/// Read `len` bytes of a buffer's payload plus its NUL terminator.
pub unsafe fn buffer_bytes(buf: *const StringBuffer) -> Vec<u8> {
    unsafe {
        let len = (*buf).length;
        assert!(len >= 0, "negative buffer length {len}");
        let data = (*buf).data;
        assert!(!data.is_null());
        let mut out = std::slice::from_raw_parts(data as *const u8, len as usize).to_vec();
        // Include the terminator that strcpy wrote.
        out.push(*data.offset(len as isize) as u8);
        out
    }
}

static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// fd 1 is process-global, so only one capture may be in flight at a time.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with file descriptor 1 redirected to a temporary file and return the
/// bytes it wrote. Both `.so`s use the process-wide glibc `stdout`, so
/// `fflush(NULL)` before and after the swap makes the capture exact.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "buffapp_stdout_{}_{}.bin",
        std::process::id(),
        n
    ));

    // SAFETY: fd juggling is restored before returning.
    unsafe {
        // Two independent buffering layers sit in front of fd 1 and both must
        // be drained before the redirect, otherwise their leftover bytes get
        // written into our temp file and corrupt the capture:
        //   * libc's `stdout`, used by `printf` inside both .so files;
        //   * Rust's `std::io::Stdout` LineWriter, used by the libtest harness
        //     for its progress lines, which can hold a partial line.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        fflush(std::ptr::null_mut());

        let file = std::fs::File::create(&path).expect("create capture file");
        let fd = {
            use std::os::fd::AsRawFd;
            file.as_raw_fd()
        };
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        fflush(std::ptr::null_mut());
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
        close(saved);
        drop(file);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Interesting `int` values, avoiding only the cases where the C code has
/// hardware-trapping undefined behaviour (`INT_MIN / -1`).
pub const INTERESTING: &[c_int] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 16, 31, 32, 33, 63, 64, 100, 999, 1000, 12345, 65535, 65536,
    1_000_000, 2_000_000_000, c_int::MAX, c_int::MAX - 1, -1, -2, -3, -4, -5, -7, -8, -100, -999,
    -12345, -1_000_000, -2_000_000_000, c_int::MIN + 1, c_int::MIN,
];
