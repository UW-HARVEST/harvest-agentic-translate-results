//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects via `libloading`; the Rust
//! implementation is NEVER called directly, always through
//! `target/<profile>/libdriver.so`, so the `#[no_mangle]` export wrappers and
//! the C ABI are part of what is under test.
//!
//! stdout comparison: `driver`/`run` communicate only through `printf`, so each
//! call is wrapped in an fd-1 redirection to a temp file. Both `.so`s import
//! `printf` from the same `libc.so.6`, hence share one `FILE *stdout`; a single
//! `fflush(NULL)` flushes whichever one wrote.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// house_t — must match `typedef struct { int; int; double; } house_t;`
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct House {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

impl House {
    /// Raw bytes of the struct, for byte-exact ABI/layout comparison.
    pub fn raw(&self) -> [u8; std::mem::size_of::<House>()] {
        unsafe { std::mem::transmute_copy(self) }
    }
}

// ---------------------------------------------------------------------------
// libc bits used by the harness
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn __errno_location() -> *mut c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}
pub fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(*const c_char);
pub type RunFn = unsafe extern "C" fn(*mut House, c_int);

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub driver: DriverFn,
    pub run: RunFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
        };
        let driver: DriverFn = unsafe {
            let s: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: symbol `driver` missing: {e}"));
            *s
        };
        let run: RunFn = unsafe {
            let s: Symbol<RunFn> = lib
                .get(b"run\0")
                .unwrap_or_else(|e| panic!("{name}: symbol `run` missing: {e}"));
            *s
        };
        Impl {
            name,
            _lib: lib,
            driver,
            run,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    if !p.exists() {
        // Build the C reference library on demand (never modifying c_src).
        let src = manifest_dir().join("../c_src");
        let build = src.join("build");
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let cfg = std::process::Command::new("cmake")
            .current_dir(&build)
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .output()
            .expect("cmake not available");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = std::process::Command::new("cmake")
            .current_dir(&build)
            .arg("--build")
            .arg(".")
            .output()
            .expect("cmake build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&bld.stderr)
        );
    }
    assert!(
        p.exists(),
        "C shared library not built: {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `target/<profile>/` for the currently running test binary.
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// True when the running test binary was built with the `release` profile.
pub fn is_release() -> bool {
    profile_dir()
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false)
}

pub fn rust_so_path() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib for the same profile sits one level up.
    let p = profile_dir().join("libdriver.so");
    // `cargo test` neither builds nor refreshes the `cdylib` artifact of the
    // package under test (the test binaries have nothing to link against), so
    // an existing `libdriver.so` may be STALE. Always run `cargo build` — it is
    // a no-op when up to date, and it guarantees the `.so` under test matches
    // `src/lib.rs`. Done once per process via `pair()`'s `OnceLock`.
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(manifest_dir().join("Cargo.toml"));
    if is_release() {
        cmd.arg("--release");
    }
    // Propagate the feature selection of this test build so the cdylib matches.
    if let Ok(feats) = std::env::var("DRIVER_TEST_FEATURES") {
        cmd.arg("--no-default-features");
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats);
        }
    }
    let out = cmd.output().expect("failed to spawn cargo build for cdylib");
    assert!(
        out.status.success(),
        "`cargo build` for the cdylib failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        p.exists(),
        "Rust cdylib still not found at {} after `cargo build`",
        p.display()
    );
    p
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &c_so_path()),
        rs: Impl::load("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f` with fd 1 redirected to a temp file and return everything written.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver-difftest-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));

    let out = {
        let file = std::fs::File::create(&path).expect("create capture file");
        unsafe {
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
            drop(file);

            f();

            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }
        std::fs::read(&path).expect("read capture file")
    };

    let _ = std::fs::remove_file(&path);
    drop(guard);
    out
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `driver(input)` in both libraries; assert identical stdout bytes.
/// `input` is the exact byte content; a NUL terminator is appended.
pub fn diff_driver(label: &str, input: &[u8]) {
    let mut cstr = input.to_vec();
    cstr.push(0);

    let p = pair();
    let c_out = capture(|| unsafe { (p.c.driver)(cstr.as_ptr() as *const c_char) });
    let r_out = capture(|| unsafe { (p.rs.driver)(cstr.as_ptr() as *const c_char) });

    assert_eq!(
        c_out,
        r_out,
        "\n[{label}] driver({:?}) diverged\n  C   : {}\n  Rust: {}\n",
        show(input),
        show(&c_out),
        show(&r_out)
    );
    // Non-vacuity: `driver` always prints either the 1-line rejection or 8
    // house lines, so an empty capture means the harness is broken.
    let lines = c_out.iter().filter(|b| **b == b'\n').count();
    assert!(
        lines == 1 || lines == 8,
        "\n[{label}] driver({:?}) produced {lines} lines (expected 1 or 8) -- capture broken?\n  {}\n",
        show(input),
        show(&c_out)
    );
}

/// Call `run` `n` times on a fresh copy of `start` in both libraries; assert
/// identical stdout bytes AND identical resulting struct bytes.
pub fn diff_run_seq(label: &str, start: House, extras: &[c_int]) {
    let p = pair();

    let mut c_house = start;
    let c_out = capture(|| {
        for &e in extras {
            unsafe { (p.c.run)(&mut c_house as *mut House, e) }
        }
    });

    let mut r_house = start;
    let r_out = capture(|| {
        for &e in extras {
            unsafe { (p.rs.run)(&mut r_house as *mut House, e) }
        }
    });

    assert_eq!(
        c_out,
        r_out,
        "\n[{label}] run(start={:?}, extras={:?}) stdout diverged\n  C   : {}\n  Rust: {}\n",
        start,
        extras,
        show(&c_out),
        show(&r_out)
    );
    // Non-vacuity: `run` prints exactly 4 lines per call, so a silently empty
    // capture (a broken harness) can never make this assertion pass.
    assert_eq!(
        c_out.iter().filter(|b| **b == b'\n').count(),
        4 * extras.len(),
        "\n[{label}] harness produced {} lines, expected {} -- capture broken?\n  {}\n",
        c_out.iter().filter(|b| **b == b'\n').count(),
        4 * extras.len(),
        show(&c_out)
    );
    assert_eq!(
        c_house.raw(),
        r_house.raw(),
        "\n[{label}] run(start={:?}, extras={:?}) resulting house_t bytes diverged\n  C   : {:?}\n  Rust: {:?}\n",
        start,
        extras,
        c_house,
        r_house
    );
}

pub fn diff_run(label: &str, start: House, extra: c_int) {
    diff_run_seq(label, start, &[extra]);
}

// ---------------------------------------------------------------------------
// Test-body isolation
// ---------------------------------------------------------------------------

/// Run a whole test body in a forked child.
///
/// `capture()` redirects **fd 1**, which is process-global state. libtest writes
/// its own progress lines ("test foo ... ok") straight to fd 1 from other
/// threads, so with the default `--test-threads=N` those lines land inside a
/// concurrent capture file and corrupt the comparison. Running each test body
/// in its own single-threaded child keeps the parent's fd 1 untouched, so the
/// suite is correct under plain `cargo test` as well as `--test-threads=1`.
///
/// The child's panic message goes to stderr (never redirected), so failures are
/// still reported normally; the parent turns a non-zero child status into a
/// panic of its own.
pub fn isolated<F: FnOnce()>(f: F) {
    // Load/build both libraries in the PARENT so the child inherits them and no
    // nested `cargo build` happens once per test.
    let _ = pair();

    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok();
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        unsafe {
            fflush(std::ptr::null_mut());
            _exit(if ok { 0 } else { 1 })
        }
    }
    let mut status: c_int = -1;
    let w = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    assert_eq!(
        status,
        0,
        "isolated test body failed in child: {} (the child's panic message is above)",
        describe_status(status)
    );
}

// ---------------------------------------------------------------------------
// Crash-equivalence (for the unchecked-NULL contracts)
// ---------------------------------------------------------------------------

/// Wait-status of a forked child that ran `f`. Returns the raw status so that
/// "exited 0" and "killed by SIGSEGV" are distinguishable and comparable.
pub fn child_status<F: FnOnce()>(f: F) -> c_int {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = -1;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        status
    }
}

pub fn describe_status(status: c_int) -> String {
    if status & 0x7f == 0x7f {
        format!("stopped(sig={})", (status >> 8) & 0xff)
    } else if status & 0x7f != 0 {
        format!("signalled(sig={})", status & 0x7f)
    } else {
        format!("exited({})", (status >> 8) & 0xff)
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    /// Uniform in `[lo, hi]` inclusive, works across the whole i64 range.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi as i128) - (lo as i128) + 1;
        let r = (self.next_u64() as u128) % (span as u128);
        ((lo as i128) + r as i128) as i64
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range_i64(lo as i64, hi as i64) as i32
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}
