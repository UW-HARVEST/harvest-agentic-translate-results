//! Phase B — differential tests through the FFI boundary.
//!
//! Both shared objects (`build_c/libcdriver.so` and `target/*/libdriver.so`)
//! are loaded with `libloading` and driven through their exported C symbols
//! (`run`, `main`). The Rust implementation is never called as a Rust function,
//! so the `#[no_mangle] extern "C"` wrappers are exercised too.
//!
//! stdout is captured by temporarily `dup2`-ing a temp file onto fd 1 around
//! each call. This is process-global state, so the whole file is one single
//! `#[test]` executed sequentially.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::os::raw::c_int;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every C stdio stream, including the ones the
    /// loaded C shared object writes to (same libc instance).
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn signal(signum: c_int, handler: usize) -> usize;
}

struct Capture {
    path: std::path::PathBuf,
    saved: c_int,
}

impl Capture {
    /// Redirects fd 1 to a fresh temp file.
    fn begin(tag: &str) -> Capture {
        let path = std::env::temp_dir().join(format!(
            "driver_ffi_{}_{}_{}.out",
            std::process::id(),
            tag,
            unsafe { COUNTER }
        ));
        unsafe { COUNTER += 1 };
        let file = std::fs::File::create(&path).expect("create capture file");
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1)");
        let fd = {
            use std::os::fd::AsRawFd;
            file.as_raw_fd()
        };
        assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 -> 1");
        drop(file);
        Capture { path, saved }
    }

    /// Flushes C stdio, restores fd 1 and returns everything written.
    fn end(self) -> Vec<u8> {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
        let bytes = std::fs::read(&self.path).expect("read capture file");
        let _ = std::fs::remove_file(&self.path);
        bytes
    }
}

static mut COUNTER: u64 = 0;

type RunFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

fn call_run(lib: &Library, tag: &str, v: c_int) -> Vec<u8> {
    let f: Symbol<RunFn> = unsafe { lib.get(b"run\0").expect("dlsym run") };
    let cap = Capture::begin(tag);
    unsafe { f(v) };
    cap.end()
}

fn call_main(lib: &Library, tag: &str, stdin_path: &std::path::Path) -> (Vec<u8>, c_int) {
    let f: Symbol<MainFn> = unsafe { lib.get(b"main\0").expect("dlsym main") };
    let input = std::fs::File::open(stdin_path).expect("open stdin file");
    let saved_stdin = unsafe { dup(0) };
    assert!(saved_stdin >= 0);
    {
        use std::os::fd::AsRawFd;
        assert!(unsafe { dup2(input.as_raw_fd(), 0) } >= 0);
    }
    let cap = Capture::begin(tag);
    let rc = unsafe { f() };
    let out = cap.end();
    unsafe {
        dup2(saved_stdin, 0);
        close(saved_stdin);
        // The C `main` (and its Rust twin) restores SIGPIPE to its default
        // disposition; put the test process back the way Rust set it up.
        const SIGPIPE: c_int = 13;
        const SIG_IGN: usize = 1;
        signal(SIGPIPE, SIG_IGN);
    }
    (out, rc)
}

#[test]
fn ffi_differential() {
    ensure_c_artifacts();

    let c_lib = unsafe { Library::new(c_so()).expect("dlopen C .so") };
    let r_lib = unsafe { Library::new(rust_so()).expect("dlopen Rust .so") };

    // Both symbols must resolve in both libraries (Phase D, symbol parity).
    for name in [&b"run\0"[..], &b"main\0"[..]] {
        unsafe {
            c_lib
                .get::<*const ()>(name)
                .unwrap_or_else(|e| panic!("C .so missing {:?}: {e}", name));
            r_lib
                .get::<*const ()>(name)
                .unwrap_or_else(|e| panic!("Rust .so missing {:?}: {e}", name));
        }
    }

    // ---- CONFIGS row 25: exported `main`, fd 0 redirected to a file --------
    // Done first, while both libraries still hold pristine global state.
    let stdin_path = std::env::temp_dir().join(format!("driver_ffi_stdin_{}", std::process::id()));
    std::fs::write(&stdin_path, b"7\n").unwrap();
    let (c_out, c_rc) = call_main(&c_lib, "c_main", &stdin_path);
    let (r_out, r_rc) = call_main(&r_lib, "r_main", &stdin_path);
    let _ = std::fs::remove_file(&stdin_path);
    assert_eq!(
        c_rc, r_rc,
        "exported main return value differs: C={c_rc} Rust={r_rc}"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "exported main stdout differs"
    );
    assert!(!c_out.is_empty(), "capture harness produced no output");

    // ---- CONFIGS rows 20, 22, 24: hand-picked `run` arguments --------------
    // (state keeps accumulating in both libraries, in lockstep)
    let fixed: &[c_int] = &[
        0,
        0,
        1,
        -1,
        5,
        -5,
        i32::MAX,
        i32::MIN,
        i32::MAX,
        i32::MAX,
        i32::MIN,
        i32::MIN,
        2,
        -3,
        1_000_000_000,
        1_000_000_000,
        1_000_000_000,
        -1_000_000_000,
        i32::MAX - 1,
        i32::MIN + 1,
        0,
    ];
    for (i, &v) in fixed.iter().enumerate() {
        let c = call_run(&c_lib, "c", v);
        let r = call_run(&r_lib, "r", v);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "run({v}) differs at fixed step {i}"
        );
    }

    // ---- CONFIGS row 21: 200 randomized calls, compared step by step ------
    let mut rng = Rng::new(0x5EED_0000_1111_2222);
    for i in 0..200 {
        let v = match rng.below(6) {
            0 => 0,
            1 => rng.next_u32() as i32,
            2 => (rng.below(21) as i32) - 10,
            3 => i32::MAX - (rng.below(4) as i32),
            4 => i32::MIN + (rng.below(4) as i32),
            _ => (rng.next_u64() as i64 % 1_000_003) as i32,
        };
        let c = call_run(&c_lib, "c", v);
        let r = call_run(&r_lib, "r", v);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "run({v}) differs at random step {i}"
        );
    }

    // ---- CONFIGS row 23: long run of run(1): bathrooms grows past 100.5 ---
    for i in 0..150 {
        let c = call_run(&c_lib, "c", 1);
        let r = call_run(&r_lib, "r", 1);
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "run(1) differs at growth step {i}"
        );
    }

    // Sanity: the last capture really did contain four printed lines.
    let last_c = call_run(&c_lib, "c", 0);
    assert_eq!(
        last_c.iter().filter(|&&b| b == b'\n').count(),
        4,
        "expected 4 lines per run() call, got {:?}",
        String::from_utf8_lossy(&last_c)
    );
    let last_r = call_run(&r_lib, "r", 0);
    assert_eq!(
        String::from_utf8_lossy(&last_c),
        String::from_utf8_lossy(&last_r)
    );
}
