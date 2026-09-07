// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
// ONLY through their exported symbols, exactly as an external C consumer would.
// Nothing in the Rust crate is called directly, so the `#[no_mangle]` export
// wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_double, c_int, c_void};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture / fork-based crash comparison.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn __errno_location() -> *mut c_int;
}

/// `house_t` from `c_src/src/driver.c`:
/// `struct { int floors; int bedrooms; double bathrooms; }`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HouseT {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

pub type DriverFn = unsafe extern "C" fn(*const c_char);
pub type RunFn = unsafe extern "C" fn(*mut HouseT, c_int);

pub struct Impl {
    pub name: &'static str,
    pub lib: Library,
}

impl Impl {
    pub fn driver(&self) -> Symbol<'_, DriverFn> {
        unsafe {
            self.lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{}: missing symbol `driver`: {e}", self.name))
        }
    }
    pub fn run(&self) -> Symbol<'_, RunFn> {
        unsafe {
            self.lib
                .get(b"run\0")
                .unwrap_or_else(|e| panic!("{}: missing symbol `run`: {e}", self.name))
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // Use the cdylib built for the SAME profile as this test binary:
    // target/<profile>/deps/<test> -> target/<profile>/libdriver.so
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let cand = profile_dir.join("libdriver.so");
            if cand.exists() {
                return cand;
            }
        }
    }
    let base = manifest_dir().join("target");
    for p in ["release/libdriver.so", "debug/libdriver.so"] {
        let cand = base.join(p);
        if cand.exists() {
            return cand;
        }
    }
    base.join("release/libdriver.so")
}

/// Both libraries, loaded once for the whole test binary.
pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library not found at {cp:?}. Build it with:\n  cd c_src && mkdir -p build \
             && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rp.exists(),
            "Rust shared library not found at {rp:?}. Build it with: cargo build --release"
        );
        let c = unsafe { Library::new(&cp) }.expect("dlopen C lib");
        let rust = unsafe { Library::new(&rp) }.expect("dlopen Rust lib");
        Pair {
            c: Impl { name: "C", lib: c },
            rust: Impl {
                name: "Rust",
                lib: rust,
            },
        }
    })
}

/// Flush every buffer that could later spill into a redirected fd 1:
/// Rust's `std` stdout/stderr and all of libc's `FILE*` streams.
pub fn flush_all() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(std::ptr::null_mut()) };
}

/// fd 1 is a process-wide resource; serialize all capture.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f` with fd 1 redirected to a temp file and return the exact bytes it
/// wrote. `fflush(NULL)` is used so that libc's stdio buffer for the shared
/// `stdout` FILE* (used by BOTH libraries) is drained before and after.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is redirected process-wide, so the libtest harness must not be
    // writing its own progress output concurrently from another thread.
    assert_eq!(
        std::env::var("RUST_TEST_THREADS").as_deref(),
        Ok("1"),
        "these differential tests redirect fd 1 process-wide and must run \
         single-threaded; use `cargo test -- --test-threads=1` or \
         RUST_TEST_THREADS=1 (see run_tests.sh)"
    );
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    // Drain the libtest harness's own (Rust-side) buffered stdout *and* libc's
    // stdio buffers before stealing fd 1, so none of it lands in our capture.
    flush_all();
    let bytes = unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        {
            let file = File::create(&path).expect("create temp capture file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        }
        f();
        fflush(std::ptr::null_mut()); // drain the library's printf buffer
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
        std::fs::read(&path).expect("read temp capture file")
    };
    let _ = std::fs::remove_file(&path);
    drop(guard);
    bytes
}

pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

pub fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

/// Byte-for-byte stdout comparison of `driver(input)` between both libraries.
pub fn diff_driver(input: &[u8], ctx: &str) {
    let l = libs();
    let cs = CString::new(input).expect("input contains an interior NUL");
    let c_out = {
        let f = l.c.driver();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    };
    let r_out = {
        let f = l.rust.driver();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    };
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "driver() stdout mismatch [{ctx}] for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(c_out, r_out, "driver() raw byte mismatch [{ctx}]");
}

/// Byte-for-byte stdout comparison AND final-struct-state comparison of
/// `run(&house, extra)` between both libraries.
pub fn diff_run(house: HouseT, extra: c_int, ctx: &str) {
    let l = libs();
    let mut hc = house;
    let c_out = {
        let f = l.c.run();
        capture_stdout(|| unsafe { f(&mut hc, extra) })
    };
    let mut hr = house;
    let r_out = {
        let f = l.rust.run();
        capture_stdout(|| unsafe { f(&mut hr, extra) })
    };
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "run() stdout mismatch [{ctx}] house={house:?} (bathrooms bits {:#018x}) extra={extra}",
        house.bathrooms.to_bits()
    );
    assert_eq!(c_out, r_out, "run() raw byte mismatch [{ctx}]");
    assert_eq!(
        hc.floors, hr.floors,
        "run() out-param floors mismatch [{ctx}] house={house:?} extra={extra}"
    );
    assert_eq!(
        hc.bedrooms, hr.bedrooms,
        "run() out-param bedrooms mismatch [{ctx}] house={house:?} extra={extra}"
    );
    assert_eq!(
        hc.bathrooms.to_bits(),
        hr.bathrooms.to_bits(),
        "run() out-param bathrooms bit mismatch [{ctx}] house={house:?} extra={extra}"
    );
}

/// Two consecutive `run` calls on the SAME struct (mirrors `driver`).
pub fn diff_run_twice(house: HouseT, extra: c_int, ctx: &str) {
    let l = libs();
    let mut hc = house;
    let c_out = {
        let f = l.c.run();
        capture_stdout(|| unsafe {
            f(&mut hc, extra);
            f(&mut hc, extra);
        })
    };
    let mut hr = house;
    let r_out = {
        let f = l.rust.run();
        capture_stdout(|| unsafe {
            f(&mut hr, extra);
            f(&mut hr, extra);
        })
    };
    assert_eq!(
        show(&c_out),
        show(&r_out),
        "run()x2 stdout mismatch [{ctx}] house={house:?} extra={extra}"
    );
    assert_eq!(c_out, r_out, "run()x2 raw byte mismatch [{ctx}]");
    assert_eq!(hc.floors, hr.floors, "run()x2 floors mismatch [{ctx}]");
    assert_eq!(hc.bedrooms, hr.bedrooms, "run()x2 bedrooms mismatch [{ctx}]");
    assert_eq!(
        hc.bathrooms.to_bits(),
        hr.bathrooms.to_bits(),
        "run()x2 bathrooms mismatch [{ctx}]"
    );
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Outcome of running something in a forked child.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Run `f` in a forked child and report how the child terminated. Used to
/// compare undefined-behaviour paths (NULL pointers) without killing the
/// test process.
pub fn fork_outcome<F: FnOnce()>(f: F) -> Outcome {
    flush_all();
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence stdout, run, exit 0 if we survive.
            if let Ok(devnull) = File::create("/dev/null") {
                dup2(devnull.as_raw_fd(), 1);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if status & 0x7f == 0 {
            Outcome::Exited((status >> 8) & 0xff)
        } else {
            Outcome::Signaled(status & 0x7f)
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep failures reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Uniform in `[lo, hi]`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// A random *finite* f64 (rejects NaN/Inf so callers can pick shapes).
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }
}
