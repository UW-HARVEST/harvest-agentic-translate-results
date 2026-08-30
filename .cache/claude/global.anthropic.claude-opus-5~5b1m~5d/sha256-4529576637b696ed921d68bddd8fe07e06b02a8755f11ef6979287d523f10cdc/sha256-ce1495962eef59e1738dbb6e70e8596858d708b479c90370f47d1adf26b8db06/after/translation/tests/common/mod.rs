//! Differential test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls the library only
//! through its exported C symbols. The Rust crate is a `cdylib`, so these
//! integration tests cannot even link it directly — every call below goes
//! through `dlsym`, exactly as an external C consumer would do.
//!
//! Because the library's only state is a file-scope mutable global with no
//! reset entry point, the two libraries must be driven in LOCKSTEP: every
//! step calls C and then Rust with the same argument while holding a global
//! lock, so both globals always see the identical operation sequence.
//! stdout is captured with `dup2` around each call, which is also process
//! global, so the same lock serialises that.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `int fflush(FILE *)` — passing NULL flushes every open C stream.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// The exact format string used by `print_the_house` in the C.
pub const FMT: &str = "The house has %d floors, %d bedrooms, and %.1f bathrooms\n";

// ---------------------------------------------------------------------------
// library locations
// ---------------------------------------------------------------------------

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {p:?}.\nBuild it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // current_exe is <target>/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|d| d.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {p:?}. Run `cargo build` first."
    );
    p
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a temp file, run `f`, flush all C streams, restore fd 1,
/// and return the captured bytes.
fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    // Don't let the Rust-side test output land in the capture file.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        tag,
        next_seq()
    ));
    let file = File::create(&path).expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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
    /// Uniformly random `i32` over the FULL range (every bit pattern).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

// ---------------------------------------------------------------------------
// harness
// ---------------------------------------------------------------------------

type Fn1 = unsafe extern "C" fn(c_int);

/// Model of the C library's `static house_t the_house`, used as a
/// cross-check that the two libraries stay in lockstep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

impl State {
    /// `static house_t the_house = {.floors=2, .bedrooms=5, .bathrooms=2.5};`
    pub const fn initial() -> Self {
        State {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        }
    }

    fn line(&self) -> String {
        format!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            self.floors, self.bedrooms, self.bathrooms
        )
    }

    /// Reproduce `void run(int extra_bedrooms)` and return its expected output.
    fn model_run(&mut self, extra: i32) -> String {
        let mut out = String::new();
        out.push_str(&self.line()); // print_the_house()
        self.floors = self.floors.wrapping_add(1); // add_floor_to_the_house()
        out.push_str(&self.line()); // print_the_house()
        self.bathrooms += 1.0; // the_house.bathrooms += 1.0
        out.push_str(&self.line()); // print_the_house()
        self.bedrooms = self.bedrooms.wrapping_add(extra); // add_bedrooms()
        out.push_str(&self.line()); // print_the_house()
        out
    }

    /// Reproduce `void driver(int x) { run(x); run(x); }`.
    fn model_driver(&mut self, x: i32) -> String {
        let mut out = self.model_run(x);
        out.push_str(&self.model_run(x));
        out
    }
}

pub struct Harness {
    // Keep the Library values alive for the whole process; the raw fn
    // pointers copied out of them stay valid as long as these live.
    _c_lib: libloading::Library,
    _r_lib: libloading::Library,
    c_run: Fn1,
    c_driver: Fn1,
    r_run: Fn1,
    r_driver: Fn1,
    /// Model state, advanced identically to both libraries' globals.
    model: State,
    steps: u64,
}

/// Entry point selector — mirrors the two exported symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `void run(int extra_bedrooms)` — the low-level entry point.
    Run,
    /// `void driver(int x)` — the wrapper (`run(x); run(x);`).
    Driver,
}

impl Harness {
    fn load() -> Self {
        // fd-1 redirection is process-global, so concurrent test threads (and
        // libtest's own progress output) would contaminate the capture file.
        // See .cargo/config.toml.
        assert_eq!(
            std::env::var("RUST_TEST_THREADS").as_deref(),
            Ok("1"),
            "these differential tests capture stdout by redirecting fd 1, which is \
             process-global; they must run single-threaded. Run them via `cargo test` \
             (which picks up .cargo/config.toml) or set RUST_TEST_THREADS=1 explicitly."
        );
        unsafe {
            let c_lib = libloading::Library::new(c_so_path()).expect("dlopen C libdriver.so");
            let r_lib = libloading::Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

            let c_run = *c_lib
                .get::<Fn1>(b"run\0")
                .expect("C .so must export `run`");
            let c_driver = *c_lib
                .get::<Fn1>(b"driver\0")
                .expect("C .so must export `driver`");
            let r_run = *r_lib
                .get::<Fn1>(b"run\0")
                .expect("Rust .so must export `run`");
            let r_driver = *r_lib
                .get::<Fn1>(b"driver\0")
                .expect("Rust .so must export `driver`");

            Harness {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c_run,
                c_driver,
                r_run,
                r_driver,
                model: State::initial(),
                steps: 0,
            }
        }
    }

    /// Current modelled `bedrooms` accumulator (used to aim at exact boundaries).
    pub fn bedrooms(&self) -> i32 {
        self.model.bedrooms
    }
    pub fn floors(&self) -> i32 {
        self.model.floors
    }
    pub fn bathrooms(&self) -> f64 {
        self.model.bathrooms
    }
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// One differential step: call C, then Rust, with the SAME argument, and
    /// assert the captured stdout is byte-identical. Returns the bytes.
    pub fn step(&mut self, entry: Entry, arg: i32, ctx: &str) -> Vec<u8> {
        let (cf, rf, name) = match entry {
            Entry::Run => (self.c_run, self.r_run, "run"),
            Entry::Driver => (self.c_driver, self.r_driver, "driver"),
        };

        let before = self.model;
        let expected = match entry {
            Entry::Run => self.model.model_run(arg),
            Entry::Driver => self.model.model_driver(arg),
        };

        let c_out = capture("c", || unsafe { cf(arg) });
        let r_out = capture("r", || unsafe { rf(arg) });
        self.steps += 1;

        // Guard against capture contamination: every captured line must be a
        // well-formed `print_the_house` line. If foreign bytes ever land in the
        // capture file we want a loud failure, never a silent false pass.
        for (label, buf) in [("C", &c_out), ("RUST", &r_out)] {
            let s = String::from_utf8_lossy(buf);
            assert!(
                s.ends_with('\n'),
                "[{ctx}] {label} capture not newline-terminated: {s:?}"
            );
            for line in s.lines() {
                assert!(
                    line.starts_with("The house has ") && line.ends_with(" bathrooms"),
                    "[{ctx}] {label} capture contaminated by foreign output: {line:?}"
                );
            }
        }

        if c_out != r_out {
            panic!(
                "DIVERGENCE [{ctx}] step #{} {name}({arg}) state-before={before:?}\n\
                 C   ({} bytes): {:?}\n\
                 RUST({} bytes): {:?}",
                self.steps,
                c_out.len(),
                String::from_utf8_lossy(&c_out),
                r_out.len(),
                String::from_utf8_lossy(&r_out),
            );
        }

        // Cross-check against the independent model of the C semantics. This
        // catches harness desynchronisation (e.g. a call that hit only one
        // library) which a plain C-vs-Rust compare would silently accept.
        let got = String::from_utf8_lossy(&c_out);
        assert_eq!(
            got, expected,
            "model mismatch [{ctx}] step #{} {name}({arg}) state-before={before:?}",
            self.steps
        );

        // `run` prints 4 lines, `driver` prints 8.
        let want_lines = if entry == Entry::Run { 4 } else { 8 };
        assert_eq!(
            c_out.iter().filter(|&&b| b == b'\n').count(),
            want_lines,
            "[{ctx}] {name} should print {want_lines} lines"
        );

        c_out
    }

    pub fn run(&mut self, arg: i32, ctx: &str) -> Vec<u8> {
        self.step(Entry::Run, arg, ctx)
    }
    pub fn driver(&mut self, arg: i32, ctx: &str) -> Vec<u8> {
        self.step(Entry::Driver, arg, ctx)
    }
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// Acquire exclusive access to both libraries + the fd-1 redirection.
pub fn lock() -> MutexGuard<'static, Harness> {
    HARNESS
        .get_or_init(|| Mutex::new(Harness::load()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
