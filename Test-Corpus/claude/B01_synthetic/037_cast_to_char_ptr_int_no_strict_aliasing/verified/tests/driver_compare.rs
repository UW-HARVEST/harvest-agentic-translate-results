//! Integration test comparing the C and Rust implementations of `driver(int)`.
//!
//! Both implementations write to stdout via libc `printf`. We capture stdout
//! by redirecting fd 1 to a temporary file around the call, then read the
//! captured bytes back. This is done for both .so files independently and
//! the resulting byte streams must match exactly.

use libloading::{Library, Symbol};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize all stdout-redirecting tests so they don't race each other.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

extern "C" {
    fn fflush(stream: *mut libc_stub::FILE) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

mod libc_stub {
    #[repr(C)]
    pub struct FILE {
        _opaque: [u8; 0],
    }
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn rust_so_path() -> PathBuf {
    // Pick the .so produced for whatever profile cargo test is using.
    // tests run under the dev profile by default, so target/debug.
    let mut candidates = Vec::new();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in &["debug", "release"] {
        let mut p = manifest.clone();
        p.push("target");
        p.push(profile);
        p.push("libdriver.so");
        candidates.push(p);
    }
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "could not find Rust libdriver.so in any of: {:?}",
        candidates
    );
}

/// Invoke `driver(x)` from the given library and return the bytes it
/// printed to stdout.
fn run_driver(lib_path: &std::path::Path, x: c_int) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Open the library and resolve the symbol.
    let lib = unsafe { Library::new(lib_path) }
        .unwrap_or_else(|e| panic!("loading {:?}: {}", lib_path, e));
    let driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("getting driver from {:?}: {}", lib_path, e));

    // Redirect stdout (fd 1) to a temp file.
    let mut tmp = tempfile();

    // Flush whatever is already buffered in libc stdout.
    unsafe {
        // fflush(NULL) flushes all open output streams.
        fflush(std::ptr::null_mut());
    }

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup(1) failed");

    unsafe {
        let r = dup2(tmp.as_raw_fd(), 1);
        assert!(r >= 0, "dup2 to tmp failed");
    }

    // Call the function under test.
    unsafe { driver(x) };

    // Flush libc stdout so all bytes hit the temp file.
    unsafe {
        fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    unsafe {
        let r = dup2(saved_stdout, 1);
        assert!(r >= 0, "dup2 restore failed");
        close(saved_stdout);
    }

    // Rewind and read all captured bytes.
    tmp.seek(SeekFrom::Start(0)).expect("seek tmp");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read tmp");
    out
}

/// Create an anonymous-like temp file that we can use as an fd target.
fn tempfile() -> std::fs::File {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("driver-cmp-{}-{}.tmp", pid, nanos));
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open tmp");
    // Unlink immediately so the file is cleaned up after the fd closes.
    let _ = fs::remove_file(&path);
    f
}

fn compare_for(x: c_int) {
    let c_out = run_driver(&c_so_path(), x);
    let r_out = run_driver(&rust_so_path(), x);
    assert_eq!(
        c_out, r_out,
        "driver({}) mismatch:\n  C   = {:?}\n  Rust= {:?}",
        x, c_out, r_out
    );
}

/// All driver comparison checks live in a single #[test] so that the test
/// harness doesn't race with our stdout redirection from another thread.
#[test]
fn driver_matches_c_for_many_inputs() {
    // Boundary values
    compare_for(0);
    compare_for(1);
    compare_for(-1);
    compare_for(i32::MAX);
    compare_for(i32::MIN);

    // Assorted positive
    for x in [
        2, 3, 7, 15, 16, 255, 256, 257, 0xdead_beefu32 as i32, 0x1234_5678,
        0x0102_0304, 100_000, 1_000_000_000,
    ] {
        compare_for(x);
    }

    // Assorted negative
    for x in [
        -2, -3, -100, -255, -256, -1_000_000, -2_000_000_000, -2_147_483_647,
    ] {
        compare_for(x);
    }

    // Each single-byte pattern, replicated across bytes — covers all
    // 256 possible byte values in any of the four positions.
    for b in 0u8..=0xffu8 {
        let v = u32::from_ne_bytes([b, b, b, b]) as i32;
        compare_for(v);
    }

    // A handful of mixed-byte patterns to spot-check ordering.
    for x in [
        0x00_00_00_01_i32, 0x00_00_01_00, 0x00_01_00_00, 0x01_00_00_00,
        0x12_34_56_78, 0x78_56_34_12, 0x7f_ff_ff_ff_u32 as i32,
        0x80_00_00_00_u32 as i32, 0xff_ff_ff_fe_u32 as i32,
    ] {
        compare_for(x);
    }
}
