//! Shared harness: loads the C and Rust shared libraries side by side and
//! captures everything each one writes to stdout so the bytes can be compared.

use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// stdout redirection is process-global, so only one capture may be in flight.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Runs `f` with fd 1 pointed at a temporary file and returns the bytes it
/// produced. `fflush(NULL)` is issued on both sides of the swap so that data
/// buffered by libc's `stdout` lands in the right place.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();

    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));

    let bytes = {
        let file = std::fs::File::create(&path).expect("create capture file");

        unsafe {
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

            f();

            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restoring stdout failed");
            close(saved);
        }

        std::fs::read(&path).expect("read capture file")
    };

    let _ = std::fs::remove_file(&path);
    bytes
}

/// Directory holding the build artifacts for the profile the tests run under
/// (`target/debug` or `target/release`).
fn rust_artifact_dir() -> PathBuf {
    // current_exe() is <target>/<profile>/deps/<test binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile> directory")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so")
}

/// The two libraries under comparison.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    fn load() -> Libs {
        let c_path = c_library_path();
        assert!(
            c_path.exists(),
            "C shared library missing at {}; build it with cmake first",
            c_path.display()
        );

        let rust_path = rust_artifact_dir().join("libdriver.so");
        assert!(
            rust_path.exists(),
            "Rust cdylib missing at {}",
            rust_path.display()
        );

        unsafe {
            Libs {
                c: Library::new(&c_path).expect("load C libdriver.so"),
                rust: Library::new(&rust_path).expect("load Rust libdriver.so"),
            }
        }
    }
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(Libs::load)
}

/// Looks up `name` in both libraries and hands the pair to `run`, which is
/// invoked once per implementation. Returns `(c_stdout, rust_stdout)`.
pub fn compare<T, F>(name: &[u8], run: F) -> (Vec<u8>, Vec<u8>)
where
    F: Fn(&Symbol<'_, T>),
{
    let libs = libs();
    let c_sym: Symbol<'_, T> = unsafe { libs.c.get(name) }
        .unwrap_or_else(|e| panic!("C symbol {:?} not found: {e}", String::from_utf8_lossy(name)));
    let rust_sym: Symbol<'_, T> = unsafe { libs.rust.get(name) }.unwrap_or_else(|e| {
        panic!(
            "Rust symbol {:?} not found: {e}",
            String::from_utf8_lossy(name)
        )
    });

    let c_out = capture_stdout(|| run(&c_sym));
    let rust_out = capture_stdout(|| run(&rust_sym));
    (c_out, rust_out)
}

/// Asserts the two byte streams are identical, printing a readable diff.
pub fn assert_same(label: &str, c_out: &[u8], rust_out: &[u8]) {
    assert_eq!(
        c_out,
        rust_out,
        "\n{label}: stdout mismatch\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}\n  C    hex: {}\n  Rust hex: {}",
        c_out.len(),
        String::from_utf8_lossy(c_out),
        rust_out.len(),
        String::from_utf8_lossy(rust_out),
        hex(c_out),
        hex(rust_out),
    );
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub type VoidFn = unsafe extern "C" fn();
pub type CharFn = unsafe extern "C" fn(c_char);
pub type StrFn = unsafe extern "C" fn(*const c_char);
pub type IntFn = unsafe extern "C" fn(c_int);
