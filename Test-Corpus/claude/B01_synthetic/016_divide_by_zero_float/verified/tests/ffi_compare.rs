// Integration tests that compare the behavior of the C `.so` against the
// Rust `.so` through their exported C ABI symbols.
//
// We never call Rust functions directly; both libraries are loaded via
// libloading so the `#[no_mangle]` export wrappers are exercised exactly the
// way an external C consumer would invoke them.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::PathBuf;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Globals: only one library load per side, only one FD-redirection at a time.
// ---------------------------------------------------------------------------

static FD_MUTEX: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver_c.so")
}

fn rust_so_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    workspace_root().join(format!("target/{}/libdriver.so", profile))
}

fn load_c() -> Library {
    unsafe { Library::new(c_so_path()).expect("load C .so") }
}
fn load_rust() -> Library {
    unsafe { Library::new(rust_so_path()).expect("load Rust .so") }
}

// ---------------------------------------------------------------------------
// FD redirection helpers — drive stdin from a memory-backed file and capture
// stdout into a file.  We dup2 over fd 0 / fd 1 of the running process so
// libc's FILE* `stdin` and `stdout` (used by the C library) and Rust's
// io::stdin/stdout (used by the Rust library) both see the same backing.
// ---------------------------------------------------------------------------

struct StdRedirect {
    saved_stdin: RawFd,
    saved_stdout: RawFd,
    captured_stdout_path: PathBuf,
}

impl StdRedirect {
    fn new(stdin_bytes: &[u8], tag: &str) -> Self {
        // Make sure C stdio buffers don't carry over between runs.
        unsafe {
            libc::fflush(std::ptr::null_mut()); // flush all FILE*
        }

        let dir = std::env::temp_dir();
        let stdin_path = dir.join(format!("driver_test_stdin_{}_{}.txt", tag, std::process::id()));
        let stdout_path = dir.join(format!("driver_test_stdout_{}_{}.txt", tag, std::process::id()));

        {
            let mut f = File::create(&stdin_path).unwrap();
            f.write_all(stdin_bytes).unwrap();
            f.flush().unwrap();
        }

        let stdin_file = OpenOptions::new().read(true).open(&stdin_path).unwrap();
        let stdout_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&stdout_path)
            .unwrap();

        // Save originals
        let saved_stdin = unsafe { libc::dup(0) };
        let saved_stdout = unsafe { libc::dup(1) };
        assert!(saved_stdin >= 0 && saved_stdout >= 0, "dup of std fds");

        // Redirect
        let new_in = stdin_file.as_raw_fd();
        let new_out = stdout_file.as_raw_fd();
        assert!(unsafe { libc::dup2(new_in, 0) } >= 0);
        assert!(unsafe { libc::dup2(new_out, 1) } >= 0);

        // Re-open libc's stdin against the new fd 0 to clear its internal
        // buffer/position (otherwise consecutive runs see EOF stale state).
        let mode_r = CString::new("r").unwrap();
        let mode_w = CString::new("w").unwrap();
        unsafe {
            // Use freopen with /dev/fd/0 to rebind without changing the fd.
            let path = CString::new("/dev/stdin").unwrap();
            libc::freopen(path.as_ptr(), mode_r.as_ptr(), libc_stdin());
            let pathw = CString::new("/dev/stdout").unwrap();
            libc::freopen(pathw.as_ptr(), mode_w.as_ptr(), libc_stdout());
        }

        // Files keep the redirected fd alive after dup2'ing because dup2
        // duplicates onto fd 0/1 — the originals can be dropped.
        drop(stdin_file);
        drop(stdout_file);

        Self {
            saved_stdin,
            saved_stdout,
            captured_stdout_path: stdout_path,
        }
    }

    fn finish(self) -> Vec<u8> {
        // Flush libc and Rust stdout so all buffered output is written to the
        // redirected fd before we restore.
        unsafe {
            libc::fflush(std::ptr::null_mut());
        }
        let _ = std::io::stdout().flush();

        // Restore.
        unsafe {
            libc::dup2(self.saved_stdin, 0);
            libc::dup2(self.saved_stdout, 1);
            libc::close(self.saved_stdin);
            libc::close(self.saved_stdout);

            // Rebind libc stdin/stdout back to terminal so subsequent
            // diagnostic prints work as expected.
            let mode_r = CString::new("r").unwrap();
            let mode_w = CString::new("w").unwrap();
            let path = CString::new("/dev/stdin").unwrap();
            libc::freopen(path.as_ptr(), mode_r.as_ptr(), libc_stdin());
            let pathw = CString::new("/dev/stdout").unwrap();
            libc::freopen(pathw.as_ptr(), mode_w.as_ptr(), libc_stdout());
        }

        // Read captured stdout
        let mut f = File::open(&self.captured_stdout_path).unwrap();
        let mut out = Vec::new();
        f.read_to_end(&mut out).unwrap();
        let _ = std::fs::remove_file(&self.captured_stdout_path);
        out
    }
}

// libc::stdin / libc::stdout are not portably FFI-typed; reach them via extern.
extern "C" {
    static stdin: *mut libc::FILE;
    static stdout: *mut libc::FILE;
}
fn libc_stdin() -> *mut libc::FILE { unsafe { stdin } }
fn libc_stdout() -> *mut libc::FILE { unsafe { stdout } }

// ---------------------------------------------------------------------------
// Generic runner: run a `void()` symbol from a loaded library against a given
// stdin payload and return captured stdout bytes.
// ---------------------------------------------------------------------------

fn run_void_fn(lib: &Library, sym: &str, stdin_bytes: &[u8]) -> Vec<u8> {
    let _g = FD_MUTEX.lock().unwrap();
    let redir = StdRedirect::new(stdin_bytes, sym);
    unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(sym.as_bytes()).unwrap();
        f();
    }
    redir.finish()
}

fn run_print_line(lib: &Library, msg: &[u8]) -> Vec<u8> {
    let _g = FD_MUTEX.lock().unwrap();
    let redir = StdRedirect::new(b"", "printLine");
    let cstr = CString::new(msg).unwrap();
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").unwrap();
        f(cstr.as_ptr());
    }
    redir.finish()
}

fn run_print_line_null(lib: &Library) -> Vec<u8> {
    let _g = FD_MUTEX.lock().unwrap();
    let redir = StdRedirect::new(b"", "printLineNull");
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    }
    redir.finish()
}

fn run_print_int_line(lib: &Library, n: c_int) -> Vec<u8> {
    let _g = FD_MUTEX.lock().unwrap();
    let redir = StdRedirect::new(b"", "printIntLine");
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"printIntLine").unwrap();
        f(n);
    }
    redir.finish()
}

fn run_main(lib: &Library, stdin_bytes: &[u8]) -> Vec<u8> {
    let _g = FD_MUTEX.lock().unwrap();
    let redir = StdRedirect::new(stdin_bytes, "main");
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int, *const *const c_char) -> c_int> =
            lib.get(b"main").unwrap();
        let _ = f(0, std::ptr::null());
    }
    redir.finish()
}

// ---------------------------------------------------------------------------
// Tests — lowest level first, then the higher-level orchestrators.
// ---------------------------------------------------------------------------

#[test]
fn print_line_basic() {
    let c = load_c();
    let r = load_rust();
    for msg in [
        b"hello".as_slice(),
        b"".as_slice(),
        b"a longer line with spaces".as_slice(),
        b"unicode-ish bytes \xe2\x9c\x93".as_slice(),
    ] {
        let co = run_print_line(&c, msg);
        let ro = run_print_line(&r, msg);
        assert_eq!(co, ro, "printLine mismatch for {:?}", msg);
    }
}

#[test]
fn print_line_null() {
    let c = load_c();
    let r = load_rust();
    let co = run_print_line_null(&c);
    let ro = run_print_line_null(&r);
    assert_eq!(co, ro, "printLine(NULL) mismatch");
}

#[test]
fn print_int_line_basic() {
    let c = load_c();
    let r = load_rust();
    for &n in &[0, 1, -1, 42, -42, i32::MIN, i32::MAX, 100, 50] {
        let co = run_print_int_line(&c, n);
        let ro = run_print_int_line(&r, n);
        assert_eq!(co, ro, "printIntLine mismatch for {}", n);
    }
}

#[test]
fn bad_with_various_input() {
    let c = load_c();
    let r = load_rust();
    let inputs: &[&[u8]] = &[
        b"2\n",        // 100/2 = 50
        b"4\n",        // 100/4 = 25
        b"0.5\n",      // 100/0.5 = 200
        b"-2\n",       // -50
        b"0\n",        // divide-by-zero -> infinity -> INT_MIN cast (or large?)
        b"   1.5\n",   // leading whitespace
        b"abc\n",      // atof returns 0 -> div by zero
        b"",           // EOF -> fgets fails -> "fgets() failed."
        b"100\n",      // 100/100 = 1
        b"1e2\n",      // 100/100 = 1
    ];
    for input in inputs {
        let co = run_void_fn(&c, "bad", input);
        let ro = run_void_fn(&r, "bad", input);
        assert_eq!(
            co, ro,
            "bad() mismatch for input {:?}\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&co),
            String::from_utf8_lossy(&ro)
        );
    }
}

#[test]
fn good_with_various_input() {
    let c = load_c();
    let r = load_rust();
    // good() calls goodG2B (no input) then goodB2G (reads one line).
    let inputs: &[&[u8]] = &[
        b"2\n",
        b"4\n",
        b"0\n",
        b"0.0000001\n", // below threshold -> divide-by-zero branch
        b"abc\n",
        b"",
        b"-5\n",
        b"1e1\n",
    ];
    for input in inputs {
        let co = run_void_fn(&c, "good", input);
        let ro = run_void_fn(&r, "good", input);
        assert_eq!(
            co, ro,
            "good() mismatch for input {:?}\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&co),
            String::from_utf8_lossy(&ro)
        );
    }
}

#[test]
fn main_end_to_end() {
    let c = load_c();
    let r = load_rust();
    // main() calls good() then bad(), so it consumes TWO lines from stdin
    // (one for goodB2G, one for bad).
    let inputs: &[&[u8]] = &[
        b"2\n2\n",
        b"4\n5\n",
        b"0\n0\n",
        b"abc\nxyz\n",
        b"",
        b"3\n",
    ];
    for input in inputs {
        let co = run_main(&c, input);
        let ro = run_main(&r, input);
        assert_eq!(
            co, ro,
            "main() mismatch for input {:?}\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&co),
            String::from_utf8_lossy(&ro)
        );
    }
}

#[test]
fn nm_exports_match() {
    // Sanity check: every symbol exported by the C .so (T or B/D) must also
    // be exported by the Rust .so. We use `nm -D` and parse.
    use std::process::Command;
    let parse = |path: &PathBuf| -> Vec<String> {
        let out = Command::new("nm").arg("-D").arg(path).output().unwrap();
        assert!(out.status.success(), "nm -D failed on {:?}", path);
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut syms: Vec<String> = text
            .lines()
            .filter_map(|l| {
                // Format: "<addr> <type> <name>" or "         <type> <name>".
                let mut parts = l.split_whitespace();
                let _first = parts.next()?;
                let (ty, name) = match (parts.next(), parts.next()) {
                    (Some(ty), Some(name)) => (ty, name),
                    _ => {
                        // Undefined: "         U name"
                        return None;
                    }
                };
                // Only consider defined exported text/data we care about.
                if !"TtRrDdBb".contains(ty) {
                    return None;
                }
                // Strip GLIBC-internal symbols.
                if name.starts_with('_') {
                    return None;
                }
                Some(name.to_string())
            })
            .collect();
        syms.sort();
        syms.dedup();
        syms
    };

    let c_syms = parse(&c_so_path());
    let r_syms = parse(&rust_so_path());
    for sym in &c_syms {
        assert!(
            r_syms.contains(sym),
            "Rust .so missing exported symbol `{}`. C exports: {:?}, Rust exports: {:?}",
            sym,
            c_syms,
            r_syms
        );
    }
}

// Suppress dead-code warnings for helpers we keep for completeness.
#[allow(dead_code)]
fn _unused(_x: &Mutex<()>) {}
