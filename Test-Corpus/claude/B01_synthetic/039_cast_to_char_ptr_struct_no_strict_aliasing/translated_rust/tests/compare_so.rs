//! Integration test that loads BOTH the C .so and the Rust .so via
//! `libloading`, calls each library's `driver(int)` symbol, captures the
//! bytes written to stdout, and asserts the captured byte streams are
//! byte-for-byte identical.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Tests must serialize because they redirect the global fd 1 (stdout).
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // The cdylib is in target/<profile>/libdriver.so.
    // Use CARGO_MANIFEST_DIR + "target/debug/libdriver.so" or a profile path
    // selected at runtime.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Try both debug and release; prefer the one that exists / is newer.
    let debug = p.join("debug").join("libdriver.so");
    let release = p.join("release").join("libdriver.so");
    p = if release.exists() && debug.exists() {
        // Pick the most recently modified
        let dm = std::fs::metadata(&debug).unwrap().modified().unwrap();
        let rm = std::fs::metadata(&release).unwrap().modified().unwrap();
        if rm >= dm { release } else { debug }
    } else if release.exists() {
        release
    } else {
        debug
    };
    p
}

/// Capture everything written to fd 1 (stdout) while `f` is running.
/// Returns the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure any in-flight buffered output is flushed to fd 1 first.
    unsafe {
        // Flush both libc stdio AND Rust's stdout buffer to be safe.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc::fflush(std::ptr::null_mut());
    }

    // Save the original fd 1.
    let saved_stdout: RawFd = unsafe { libc::dup(1) };
    assert!(saved_stdout >= 0, "dup(1) failed");

    // Create a pipe.
    let mut fds = [0 as libc::c_int; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe() failed");
    let read_fd: RawFd = fds[0];
    let write_fd: RawFd = fds[1];

    // Replace fd 1 with the write end of the pipe.
    let rc = unsafe { libc::dup2(write_fd, 1) };
    assert_eq!(rc, 1, "dup2 failed");
    // We no longer need the original write_fd (fd 1 is its alias now).
    unsafe { libc::close(write_fd) };

    // Run the function under capture.
    f();

    // Flush libc stdio AND Rust stdout to ensure all output went through fd 1.
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    // Restore fd 1 to the original stdout.
    let rc = unsafe { libc::dup2(saved_stdout, 1) };
    assert_eq!(rc, 1, "dup2 restore failed");
    unsafe { libc::close(saved_stdout) };

    // Read everything from the read end of the pipe.
    // Use a File so we can use std::io::Read.
    use std::os::unix::io::FromRawFd;
    let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).expect("read pipe");
    buf
}

unsafe fn call_driver(lib: &Library, floors: i32) -> Vec<u8> {
    let func: Symbol<unsafe extern "C" fn(i32)> = lib.get(b"driver\0").expect("driver symbol");
    capture_stdout(|| {
        func(floors);
    })
}

fn run_case(floors: i32) {
    let _g = STDOUT_LOCK.lock().unwrap();

    let c_lib = unsafe { Library::new(c_so_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };

    let c_out = unsafe { call_driver(&c_lib, floors) };
    let r_out = unsafe { call_driver(&r_lib, floors) };

    assert_eq!(
        c_out, r_out,
        "driver({}) outputs differ.\n  C    = {:?}\n  Rust = {:?}",
        floors,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

#[test]
fn driver_zero() {
    run_case(0);
}

#[test]
fn driver_positive_small() {
    run_case(1);
    run_case(2);
    run_case(3);
    run_case(42);
}

#[test]
fn driver_negative() {
    run_case(-1);
    run_case(-42);
    run_case(i32::MIN);
}

#[test]
fn driver_max() {
    run_case(i32::MAX);
}

#[test]
fn driver_powers_of_two() {
    for shift in 0..31 {
        run_case(1i32 << shift);
    }
}

#[test]
fn driver_random_assortment() {
    let cases: &[i32] = &[
        0, 1, -1, 7, -7, 100, 1000, 99999, -99999, 0x12345678, -0x12345678, 0x7fffffff,
        -0x80000000, 0xdeadbeefu32 as i32, 0xcafebabeu32 as i32,
    ];
    for &c in cases {
        run_case(c);
    }
}

/// Compare the dynamically exported (T/D/R) symbols of both shared
/// libraries. Every symbol the C .so exports must also be exported by
/// the Rust .so with the exact same name. Linker-internal sentinels
/// (`_init`, `_fini`, `_edata`, `_end`, `__bss_start`) are excluded
/// because they are emitted by the linker itself rather than the
/// translation unit.
#[test]
fn symbol_parity() {
    use std::process::Command;

    fn exported(path: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("nm");
        assert!(out.status.success(), "nm failed: {:?}", out);
        let s = String::from_utf8_lossy(&out.stdout);
        let mut syms: Vec<String> = Vec::new();
        for line in s.lines() {
            // Format: "<addr> <type> <name>"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let typ = parts[parts.len() - 2];
            let name = parts[parts.len() - 1];
            // Only consider externally-relevant symbol types.
            if !matches!(typ, "T" | "D" | "R" | "B") {
                continue;
            }
            // Skip linker-emitted sentinels and weak helpers we cannot
            // reasonably reproduce on the Rust side.
            if matches!(
                name,
                "_init"
                    | "_fini"
                    | "_edata"
                    | "_end"
                    | "__bss_start"
                    | "__data_start"
                    | "__dso_handle"
                    | "_IO_stdin_used"
            ) {
                continue;
            }
            syms.push(name.to_string());
        }
        syms.sort();
        syms.dedup();
        syms
    }

    let c_syms = exported(&c_so_path());
    let r_syms = exported(&rust_so_path());

    let mut missing: Vec<&String> = Vec::new();
    for s in &c_syms {
        if !r_syms.contains(s) {
            missing.push(s);
        }
    }
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {:?}\nC syms: {:?}\nRust syms: {:?}",
        missing,
        c_syms,
        r_syms
    );
}

// We need libc — pull it in via build dependency. Cargo dev-dependencies
// is fine because tests are the only user.
extern crate libc;
// suppress unused import warning if libc isn't directly named
#[allow(dead_code)]
fn _force_link_libc() {
    let _ = unsafe { libc::dup(0) };
}
// silence unused warning around CString import in some compiler versions
#[allow(dead_code)]
fn _u(_: CString) {}
