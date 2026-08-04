use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_char;
use std::os::unix::io::AsRawFd;

type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn close(fd: i32) -> i32;
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // Try release then debug
    let release = format!("{}/target/release/libdriver.so", manifest);
    let debug = format!("{}/target/debug/libdriver.so", manifest);
    if std::path::Path::new(&release).exists() {
        release
    } else {
        debug
    }
}

/// Capture everything printed to stdout (including printf in dynamic
/// libraries) while running `f`.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush both Rust and C buffers before redirecting
        std::io::Write::flush(&mut std::io::stdout()).ok();
        fflush(std::ptr::null_mut());

        let stdout_fd = 1i32;
        let saved = dup(stdout_fd);
        assert!(saved >= 0, "dup failed");

        // Make a temp file to capture stdout into
        let path = std::env::temp_dir().join(format!(
            "driver-stdout-{}-{}.tmp",
            std::process::id(),
            rand_suffix()
        ));
        let file = File::create(&path).expect("create tmp file");
        let fd = file.as_raw_fd();
        let r = dup2(fd, stdout_fd);
        assert!(r >= 0, "dup2 failed");

        f();

        // Flush after the call
        std::io::Write::flush(&mut std::io::stdout()).ok();
        fflush(std::ptr::null_mut());

        // Restore stdout
        let r = dup2(saved, stdout_fd);
        assert!(r >= 0, "dup2 restore failed");
        close(saved);

        // Read captured contents
        drop(file);
        let mut out = Vec::new();
        let mut f = File::open(&path).expect("open tmp file");
        f.seek(SeekFrom::Start(0)).ok();
        f.read_to_end(&mut out).expect("read tmp file");
        std::fs::remove_file(&path).ok();
        out
    }
}

fn rand_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64).wrapping_mul(2654435761)
}

fn run_driver(lib: &Library, s1: &str, s2: &str) -> Vec<u8> {
    let cs1 = CString::new(s1).unwrap();
    let cs2 = CString::new(s2).unwrap();
    let driver: Symbol<DriverFn> = unsafe { lib.get(b"driver\0").unwrap() };
    capture_stdout(|| unsafe { driver(cs1.as_ptr(), cs2.as_ptr()) })
}

fn cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("", ""),
        ("hello", ""),
        ("", "abc"),
        ("hello world", " "),
        ("hello world", "world"),
        ("abcdef", "fedcba"),
        ("abcdef", "xyz"),
        ("aaaaa", "a"),
        ("aaaaa", "b"),
        ("the quick brown fox", "qf"),
        ("12345", "54321"),
        ("12345", "6789"),
        ("a", "a"),
        ("a", "b"),
        ("longer string with many characters", "xyz"),
        ("longer string with many characters", "z"),
        ("longer string with many characters", "g"),
        ("\t\n ", " \n"),
        ("\x01\x02\x03", "\x03"),
        ("mixed CASE Letters", "L"),
    ]
}

#[test]
fn driver_matches_c() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    for (s1, s2) in cases() {
        let c_out = run_driver(&c_lib, s1, s2);
        let r_out = run_driver(&r_lib, s1, s2);
        assert_eq!(
            c_out, r_out,
            "mismatch for s1={:?}, s2={:?}: C={:?}, Rust={:?}",
            s1, s2, c_out, r_out
        );
    }
}
