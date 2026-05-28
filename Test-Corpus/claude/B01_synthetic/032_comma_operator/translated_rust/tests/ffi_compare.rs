// Integration test that loads BOTH the C-compiled .so and the Rust-compiled .so
// via libloading and compares their outputs through the FFI boundary for the
// `driver(int)` function.
//
// We capture libc stdout by redirecting fd 1 to a temporary file via dup2,
// calling the FFI function, fflush()ing libc stdout, and reading the file.

use libloading::{Library, Symbol};
use std::ffi::{c_int, CString};
use std::fs;
use std::io::Read;
use std::os::raw::c_char;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    static stdout: *mut core::ffi::c_void;
}

// Serialize stdout-capturing tests: the process's fd 1 is a shared resource.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Capture libc stdout while running `f`. Returns the bytes written to fd 1.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Make sure any buffered Rust println!/print! is flushed first.
    use std::io::Write as _;
    let _ = std::io::stdout().flush();

    // Create temp file for capturing.
    let tmp_path = std::env::temp_dir().join(format!(
        "ffi_capture_{}_{}.txt",
        std::process::id(),
        rand_suffix()
    ));
    let tmp = fs::File::create(&tmp_path).expect("create tmp file");
    let tmp_fd = tmp.as_raw_fd();

    unsafe {
        // Flush libc's stdout buffer before redirecting.
        fflush(stdout);

        // Save original stdout.
        let saved = dup(1);
        assert!(saved >= 0, "dup failed");

        // Redirect fd 1 to the tmp file.
        let r = dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");

        // Run the function; printf will write into tmp file via fd 1.
        f();

        // Flush libc stdout so all bytes hit the file.
        fflush(stdout);

        // Restore original stdout.
        let r = dup2(saved, 1);
        assert!(r >= 0, "dup2 restore failed");
        close(saved);
    }

    // Tmp file goes out of scope; read its contents from path.
    drop(tmp);
    let mut buf = Vec::new();
    let mut f = fs::File::open(&tmp_path).expect("open tmp file");
    f.read_to_end(&mut buf).expect("read tmp file");
    let _ = fs::remove_file(&tmp_path);
    buf
}

fn rand_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src").join("build").join("libdriver.so");
    if !p.exists() {
        panic!(
            "C shared library not found at {:?}. Build it with:\n  \
             gcc -shared -fPIC -o {} {}",
            p,
            p.display(),
            workspace_root().join("c_src/src/main.c").display()
        );
    }
    p
}

fn rust_so_path() -> PathBuf {
    // tests are linked against the dev profile by default unless `--release`
    // is passed to cargo test. We try both.
    let release = workspace_root().join("target/release/libtranslated_rust.so");
    let debug = workspace_root().join("target/debug/libtranslated_rust.so");
    if release.exists() {
        release
    } else if debug.exists() {
        debug
    } else {
        panic!(
            "Rust shared library not found. Build it with `cargo build` first."
        );
    }
}

unsafe fn load_driver<'lib>(
    lib: &'lib Library,
) -> Symbol<'lib, unsafe extern "C" fn(c_int)> {
    lib.get::<unsafe extern "C" fn(c_int)>(b"driver\0")
        .expect("driver symbol")
}

fn run_driver_and_capture(lib_path: &Path, x: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("load shared library");
        let driver = load_driver(&lib);
        capture_stdout(|| {
            driver(x);
        })
    }
}

fn compare_for(x: c_int) {
    let c_out = run_driver_and_capture(&c_so_path(), x);
    let r_out = run_driver_and_capture(&rust_so_path(), x);
    assert_eq!(
        c_out, r_out,
        "Mismatch for x={}: C={:?} Rust={:?}",
        x,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

// Run all driver comparisons in a single #[test] to avoid concurrent
// stdout-capture races with the test harness. Capturing stdout requires
// dup2'ing fd 1, which is process-wide; running in parallel would corrupt
// captured output. The mutex inside `capture_stdout` only protects against
// our own functions racing — it cannot stop the test harness's own writes.
#[test]
fn driver_matches_c_for_all_inputs() {
    // Edge case: x == 0 (no iterations).
    compare_for(0);
    // Edge case: x == 1 (one iteration).
    compare_for(1);
    // Small values 0..16.
    for x in 0..16 {
        compare_for(x);
    }
    // Negative values and INT_MIN — all yield zero iterations.
    for x in [-1, -10, i32::MIN].iter() {
        compare_for(*x);
    }
    // Medium values.
    compare_for(100);
    compare_for(1000);
    // Larger value to exercise more iterations.
    compare_for(10_000);
}

#[test]
fn exports_match_c() {
    // Validate that every dynamic symbol exported by the C .so is also
    // exported by the Rust .so. We use `nm -D --defined-only`.
    let c = std::process::Command::new("nm")
        .args(["-D", "--defined-only", c_so_path().to_str().unwrap()])
        .output()
        .expect("run nm on c .so");
    let r = std::process::Command::new("nm")
        .args(["-D", "--defined-only", rust_so_path().to_str().unwrap()])
        .output()
        .expect("run nm on rust .so");

    fn parse_text_symbols(out: &[u8]) -> std::collections::BTreeSet<String> {
        String::from_utf8_lossy(out)
            .lines()
            .filter_map(|line| {
                // Format: "<addr> <type> <name>"
                let mut it = line.split_whitespace();
                let _addr = it.next()?;
                let typ = it.next()?;
                let name = it.next()?;
                // Only "T" / "t" symbols (text/code) are meaningful user code
                // exports; skip data, init, fini, weak runtime stuff.
                let skip = matches!(
                    name,
                    "_init"
                        | "_fini"
                        | "__bss_start"
                        | "_edata"
                        | "_end"
                        | "__cxa_finalize"
                        | "__gmon_start__"
                        | "_ITM_deregisterTMCloneTable"
                        | "_ITM_registerTMCloneTable"
                );
                if skip {
                    return None;
                }
                if typ == "T" || typ == "t" {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    let c_syms = parse_text_symbols(&c.stdout);
    let r_syms = parse_text_symbols(&r.stdout);

    // Every C-exported text symbol must be in Rust.
    let missing: Vec<_> = c_syms.difference(&r_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {:?}",
        missing
    );
}

// Use CString to silence dead_code warnings if the helper is added later.
#[allow(dead_code)]
fn _use_cstring(_: CString, _: *const c_char) {}
