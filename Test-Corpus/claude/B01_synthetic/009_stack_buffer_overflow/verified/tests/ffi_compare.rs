// Integration tests that load both the C and Rust shared libraries via
// libloading and compare their byte-for-byte stdout for each exported
// function.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize all FD-redirecting tests; cargo runs tests in parallel by default
// and dup2 on stdout/stdin would race across threads.
static FD_LOCK: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    // Cargo places the cdylib for integration tests in target/<profile>/deps
    // or target/<profile>/. Try both.
    let root = workspace_root();
    let candidates = [
        root.join("target/debug/libdriver.so"),
        root.join("target/release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust libdriver.so not found in {:?}", candidates);
}

/// Capture what `f` writes to stdout, optionally feeding `stdin_input`
/// to stdin for the duration of the call.
fn capture_with_stdin<F: FnOnce()>(stdin_input: Option<&[u8]>, f: F) -> Vec<u8> {
    use std::os::unix::io::AsRawFd;

    let _g = FD_LOCK.lock().unwrap();

    // Flush our process's Rust stdout so prior writes don't bleed into the file.
    std::io::stdout().flush().ok();

    let stdout_fd = 1;
    let stdin_fd = 0;

    // Save originals
    let saved_stdout = unsafe { libc::dup(stdout_fd) };
    assert!(saved_stdout >= 0, "dup stdout failed");

    let saved_stdin = if stdin_input.is_some() {
        let f = unsafe { libc::dup(stdin_fd) };
        assert!(f >= 0, "dup stdin failed");
        Some(f)
    } else {
        None
    };

    // Create temp file for stdout
    let mut out_file = tempfile_rw();
    unsafe {
        let r = libc::dup2(out_file.as_raw_fd(), stdout_fd);
        assert!(r >= 0, "dup2 stdout failed");
    }

    // Create temp file for stdin if provided
    let mut in_file_opt = stdin_input.map(|input| {
        let mut f = tempfile_rw();
        f.write_all(input).expect("write stdin");
        f.flush().ok();
        f.seek(SeekFrom::Start(0)).expect("seek 0");
        unsafe {
            let r = libc::dup2(f.as_raw_fd(), stdin_fd);
            assert!(r >= 0, "dup2 stdin failed");
        }
        f
    });

    // Run the function
    f();

    // Flush libc stdout (the C library uses libc FILE*) — fflush(NULL)
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    std::io::stdout().flush().ok();

    // Restore originals
    unsafe {
        libc::dup2(saved_stdout, stdout_fd);
        libc::close(saved_stdout);
        if let Some(s) = saved_stdin {
            libc::dup2(s, stdin_fd);
            libc::close(s);
        }
    }

    // Read captured output
    out_file.seek(SeekFrom::Start(0)).expect("seek out");
    let mut buf = Vec::new();
    out_file.read_to_end(&mut buf).expect("read out");

    // drop temp files
    drop(in_file_opt.take());
    drop(out_file);

    buf
}

fn tempfile_rw() -> fs::File {
    // Use a unique path under target/.tmp/.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = workspace_root().join("target/.tmp");
    fs::create_dir_all(&dir).ok();
    let path = dir.join(format!("ffi-{}-{}-{}.tmp", pid, n, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open tmp")
}

fn load_lib(path: &PathBuf) -> Library {
    unsafe { Library::new(path).unwrap_or_else(|e| panic!("load {:?}: {}", path, e)) }
}

#[test]
fn print_line_matches() {
    let c_lib = load_lib(&c_lib_path());
    let r_lib = load_lib(&rust_lib_path());

    let test_inputs: Vec<&[u8]> = vec![
        b"hello",
        b"",
        b"line with spaces and\ttabs",
        b"unicode-ish: \xc3\xa9\xc3\xa1",
    ];

    for input in test_inputs {
        let cs = CString::new(input).unwrap();

        let c_out = capture_with_stdin(None, || unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                c_lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });
        let r_out = capture_with_stdin(None, || unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                r_lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });

        assert_eq!(c_out, r_out, "printLine mismatch for input {:?}", input);
    }

    // Null pointer: should print nothing.
    let c_out = capture_with_stdin(None, || unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_with_stdin(None, || unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn print_int_line_matches() {
    let c_lib = load_lib(&c_lib_path());
    let r_lib = load_lib(&rust_lib_path());

    let inputs: [c_int; 7] = [0, 1, -1, 42, -42, c_int::MAX, c_int::MIN];

    for &v in &inputs {
        let c_out = capture_with_stdin(None, || unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"printIntLine").unwrap();
            f(v);
        });
        let r_out = capture_with_stdin(None, || unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"printIntLine").unwrap();
            f(v);
        });
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}", v);
    }
}

#[test]
fn good_matches() {
    let c_lib = load_lib(&c_lib_path());
    let r_lib = load_lib(&rust_lib_path());

    // good() calls goodG2B (no stdin) then goodB2G (reads stdin).
    let stdin_inputs: Vec<&[u8]> = vec![
        b"5\n",
        b"0\n",
        b"9\n",
        b"abc\n",     // atoi -> 0 -> in range
        b"-3\n",      // atoi -> -3 -> ERROR negative branch
        b"100\n",     // atoi -> 100 -> out of bounds branch
        b"",          // EOF -> fgets fails -> ERROR
        b"   42\n",   // leading whitespace
        b"+7\n",      // explicit plus -> 7
    ];

    for input in stdin_inputs {
        let c_out = capture_with_stdin(Some(input), || unsafe {
            let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
            f();
        });
        let r_out = capture_with_stdin(Some(input), || unsafe {
            let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
            f();
        });
        assert_eq!(
            c_out, r_out,
            "good() mismatch for stdin {:?}\nC:   {:?}\nRust:{:?}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn bad_matches_safe_inputs() {
    // The C `bad()` reads an int from stdin and writes buffer[data] = 1
    // without bounds check. To compare safely we only feed inputs that
    // are negative (ERROR branch) or 0..=9 (in-bounds branch).
    let c_lib = load_lib(&c_lib_path());
    let r_lib = load_lib(&rust_lib_path());

    let stdin_inputs: Vec<&[u8]> = vec![
        b"0\n",
        b"5\n",
        b"9\n",
        b"-1\n",
        b"-100\n",
        b"abc\n",   // atoi -> 0 -> in range
        b"",        // EOF -> data stays -1 -> ERROR negative
    ];

    for input in stdin_inputs {
        let c_out = capture_with_stdin(Some(input), || unsafe {
            let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
            f();
        });
        let r_out = capture_with_stdin(Some(input), || unsafe {
            let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad").unwrap();
            f();
        });
        assert_eq!(
            c_out, r_out,
            "bad() mismatch for stdin {:?}\nC:   {:?}\nRust:{:?}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn main_matches() {
    let c_lib = load_lib(&c_lib_path());
    let r_lib = load_lib(&rust_lib_path());

    // main() runs good() (which reads stdin once for goodB2G) and then
    // bad() (which reads stdin once). So we provide two lines.
    let stdin_inputs: Vec<&[u8]> = vec![
        b"5\n3\n",
        b"0\n0\n",
        b"-1\n-1\n",
        b"abc\n9\n",
        b"100\n5\n", // good prints out-of-bounds error, bad prints array
    ];

    for input in stdin_inputs {
        let c_out = capture_with_stdin(Some(input), || unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int, *const *const c_char) -> c_int> =
                c_lib.get(b"main").unwrap();
            let _ = f(0, std::ptr::null());
        });
        let r_out = capture_with_stdin(Some(input), || unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int, *const *const c_char) -> c_int> =
                r_lib.get(b"main").unwrap();
            let _ = f(0, std::ptr::null());
        });
        assert_eq!(
            c_out, r_out,
            "main() mismatch for stdin {:?}\nC:   {:?}\nRust:{:?}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
