// Integration test: loads both the C and Rust shared libraries and compares
// their `driver(c)` outputs byte-for-byte by capturing stdout via dup2.

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::io::{Read, Seek, SeekFrom};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    static stdout: *mut std::ffi::c_void;
    fn fileno(stream: *mut std::ffi::c_void) -> c_int;
}

type DriverFn = unsafe extern "C" fn(c_char);

/// Calls the loaded driver function with `c`, capturing whatever it writes
/// to stdout, and returning the captured bytes.
fn run_driver_capture(driver: &Symbol<DriverFn>, c: c_char) -> Vec<u8> {
    unsafe {
        // Flush whatever might be buffered in libc stdout before redirecting.
        fflush(stdout);

        // Save the current stdout fd.
        let stdout_fd = fileno(stdout);
        let saved = dup(stdout_fd);
        assert!(saved >= 0, "dup failed");

        // Create a temp file to redirect stdout to.
        let mut tmp = tempfile_simple();
        let tmp_fd = file_fd(&tmp);

        // Redirect stdout to the temp file.
        let r = dup2(tmp_fd, stdout_fd);
        assert!(r >= 0, "dup2 redirect failed");

        // Call driver via the loaded symbol.
        driver(c);

        // Flush stdout (the libc FILE* used by printf).
        fflush(stdout);

        // Restore original stdout.
        let r = dup2(saved, stdout_fd);
        assert!(r >= 0, "dup2 restore failed");
        close(saved);

        // Read back the captured content.
        tmp.seek(SeekFrom::Start(0)).expect("seek failed");
        let mut buf = Vec::new();
        tmp.read_to_end(&mut buf).expect("read failed");
        buf
    }
}

fn tempfile_simple() -> std::fs::File {
    // Use a unique name based on PID + a counter to avoid collisions across
    // calls in the same process.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("driver-cmp-{}-{}.tmp", pid, n));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open temp file");
    // Unlink so it is cleaned up when closed.
    let _ = std::fs::remove_file(&path);
    f
}

fn file_fd(f: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    f.as_raw_fd()
}

fn project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> std::path::PathBuf {
    project_root().join("c_src").join("build").join("libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // Tests are launched from CARGO_MANIFEST_DIR. The cdylib build output
    // sits under target/{debug,release}. We prefer release if it exists,
    // otherwise debug.
    let release = project_root().join("target").join("release").join("libdriver.so");
    if release.exists() {
        return release;
    }
    project_root().join("target").join("debug").join("libdriver.so")
}

#[test]
fn driver_outputs_match_for_all_inputs() {
    // Build the rust library if needed by depending on it being already built.
    // (Tests are run after a build by cargo test, but we use libloading to
    // load the cdylib explicitly.)
    let c_path = c_lib_path();
    let r_path = rust_lib_path();

    assert!(c_path.exists(), "C .so not built at {:?}", c_path);
    assert!(r_path.exists(), "Rust .so not built at {:?}", r_path);

    unsafe {
        let c_lib = Library::new(&c_path).expect("load C .so");
        let r_lib = Library::new(&r_path).expect("load Rust .so");

        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver\0").expect("C driver symbol");
        let r_driver: Symbol<DriverFn> = r_lib.get(b"driver\0").expect("Rust driver symbol");

        // Iterate over the full range of `char` values. On x86_64 Linux,
        // `char` is signed (i8), so we use i8::MIN..=i8::MAX. To be thorough
        // and exercise the same code paths the C driver does on negative
        // values (EOF-adjacent), we test the full signed-byte range.
        for v in i8::MIN..=i8::MAX {
            let c = v as c_char;

            let c_out = run_driver_capture(&c_driver, c);
            let r_out = run_driver_capture(&r_driver, c);

            assert_eq!(
                c_out, r_out,
                "mismatch for input {} (0x{:02x}):\nC:    {:?}\nRust: {:?}",
                v,
                v as u8,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out),
            );
        }
    }
}
