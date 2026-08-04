// Compares the C and Rust shared libraries by loading both with libloading
// and comparing stdout output for the same inputs.
//
// We capture stdout by redirecting fd 1 to a temporary file with dup2, then
// reading the file back. We must call fflush(stdout) before swapping fds so
// libc's stdio buffer is committed.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize stdout-capturing tests so they don't race on fd 1.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Use the locally-built cdylib. Cargo builds the cdylib for the package
    // before running tests, so target/<profile>/libdriver.so exists.
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join("libdriver.so")
}

/// Run `f` while capturing everything written to stdout (fd 1) by the current
/// process and any C libraries linked into it.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let _guard = STDOUT_LOCK.lock().unwrap();

    // Make sure all pending Rust + C stdout is flushed before we swap.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    let tmp = tempfile();
    let saved_fd = unsafe { dup(1) };
    assert!(saved_fd >= 0, "dup failed");
    let new_fd = tmp.as_raw_fd();
    let r = unsafe { dup2(new_fd, 1) };
    assert!(r >= 0, "dup2 failed");

    f();

    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    // Restore stdout
    unsafe {
        dup2(saved_fd, 1);
        close(saved_fd);
    }

    // Read back what was captured
    let mut out = String::new();
    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).unwrap();
    tmp.read_to_string(&mut out).unwrap();
    out
}

fn tempfile() -> File {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("driver_test_{}_{}.tmp", pid, nanos));
    let f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    // Unlink immediately so the file goes away after close.
    let _ = std::fs::remove_file(&path);
    f
}

type DriverFn = unsafe extern "C" fn(c_uint, c_uint, bool, c_int);

fn load_driver(lib: &Library) -> Symbol<'_, DriverFn> {
    unsafe { lib.get(b"driver\0").expect("driver symbol not found") }
}

fn run_driver(lib: &Library, x: c_uint, y: c_uint, b: bool, z: c_int) -> String {
    let driver = load_driver(lib);
    capture_stdout(|| unsafe { driver(x, y, b, z) })
}

#[test]
fn driver_matches_c_for_assorted_inputs() {
    let c = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    let cases: Vec<(c_uint, c_uint, bool, c_int)> = vec![
        (0, 0, false, 0),
        (1, 2, true, 42),
        (3, 7, true, -1),
        (2, 5, false, 100),
        (0, 7, true, i32::MAX),
        (3, 0, false, i32::MIN),
        // Out-of-range values: C truncates via bit-field width.
        (4, 8, true, 0),
        (7, 15, false, -100),
        (u32::MAX, u32::MAX, true, -1),
        (5, 10, false, 12345),
    ];

    for (x, y, b, z) in cases {
        let c_out = run_driver(&c, x, y, b, z);
        let r_out = run_driver(&r, x, y, b, z);
        assert_eq!(
            c_out, r_out,
            "driver({}, {}, {}, {}) C={:?} Rust={:?}",
            x, y, b, z, c_out, r_out
        );
    }
}

// print_foo takes a pointer to a struct whose binary layout matches the C
// bit-field struct. We construct the struct by-bytes so we don't depend on
// the Rust crate's internal type. The layout (verified empirically) is:
//   byte 0: bit0..1 = x, bit2..4 = y, bit5 = b
//   bytes 1..3: padding
//   bytes 4..7: z (little-endian int)
type PrintFooFn = unsafe extern "C" fn(*const u8);

fn build_foo_bytes(x: c_uint, y: c_uint, b: bool, z: c_int) -> [u8; 8] {
    let xv = (x & 0x3) as u8;
    let yv = (y & 0x7) as u8;
    let bv = if b { 1u8 } else { 0u8 };
    let packed = xv | (yv << 2) | (bv << 5);
    let mut out = [0u8; 8];
    out[0] = packed;
    let zb = z.to_le_bytes();
    out[4..8].copy_from_slice(&zb);
    out
}

fn run_print_foo(lib: &Library, bytes: &[u8; 8]) -> String {
    let f: Symbol<PrintFooFn> = unsafe { lib.get(b"print_foo\0").expect("print_foo symbol") };
    capture_stdout(|| unsafe { f(bytes.as_ptr()) })
}

#[test]
fn print_foo_matches_c_for_assorted_inputs() {
    let c = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    let cases: Vec<(c_uint, c_uint, bool, c_int)> = vec![
        (0, 0, false, 0),
        (1, 2, true, 42),
        (3, 7, true, -1),
        (2, 5, false, 100),
        (0, 7, true, i32::MAX),
        (3, 0, false, i32::MIN),
    ];

    for (x, y, b, z) in cases {
        let bytes = build_foo_bytes(x, y, b, z);
        let c_out = run_print_foo(&c, &bytes);
        let r_out = run_print_foo(&r, &bytes);
        assert_eq!(
            c_out, r_out,
            "print_foo({}, {}, {}, {}) C={:?} Rust={:?}",
            x, y, b, z, c_out, r_out
        );
    }
}

#[test]
fn rust_so_exports_all_c_symbols() {
    // Sanity: ensure both libs export the same public functions we test.
    let c = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    for sym in &[b"driver\0".as_ref(), b"print_foo\0".as_ref()] {
        unsafe {
            let _: Symbol<unsafe extern "C" fn()> = c.get(sym).expect("C missing symbol");
            let _: Symbol<unsafe extern "C" fn()> = r.get(sym).expect("Rust missing symbol");
        }
    }
}
