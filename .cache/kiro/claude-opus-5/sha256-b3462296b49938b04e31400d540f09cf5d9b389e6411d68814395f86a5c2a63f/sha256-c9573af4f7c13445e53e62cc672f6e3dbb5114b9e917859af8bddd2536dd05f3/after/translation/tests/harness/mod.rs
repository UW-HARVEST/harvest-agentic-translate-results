//! Shared differential-test harness.
//!
//! Loads BOTH shared objects — the C `libdriver.so` built by CMake and the Rust
//! `libdriver.so` built by cargo — through `libloading`, and calls them only via
//! their exported `extern "C"` symbols. The Rust implementation is never called
//! directly from the test binary, so the `#[no_mangle]` export wrappers are
//! themselves under test.
//!
//! The library communicates its results exclusively through `printf` to
//! `stdout` (both public functions return `void`), so "comparing outputs" means
//! capturing the bytes each `.so` writes to file descriptor 1 and comparing them
//! byte-for-byte.
//!
//! ## Why a global lock and lock-step calling
//!
//! `stdout` redirection via `dup2` is process-global, and each `.so` owns its own
//! copy of the mutable `static house_t the_house`, whose state persists across
//! calls. Both facts are handled the same way: every comparison step takes the
//! global `HARNESS` mutex and, inside it, invokes the C function and then the
//! Rust function with the identical argument. Because *every* step always drives
//! both libraries with the same argument in the same order, the two copies of
//! the global state stay in lock-step no matter which order the `#[test]`
//! functions themselves happen to run in, and no matter how many test threads
//! libtest uses.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// glibc: `fflush(NULL)` flushes *all* open output streams, which avoids
    /// needing to resolve the `stdout` `FILE*` global.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// `void (*)(int)` — the signature of both exported entry points.
pub type IntFn = unsafe extern "C" fn(c_int);

/// Which exported entry point to drive.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `void run(int extra_bedrooms)` — the lowest-level exported entry point.
    Run,
    /// `void driver(int x)` — the convenience wrapper (`run(x); run(x);`).
    Driver,
}

impl Entry {
    pub fn name(self) -> &'static str {
        match self {
            Entry::Run => "run",
            Entry::Driver => "driver",
        }
    }
}

struct Api {
    run: IntFn,
    driver: IntFn,
    /// Kept alive for the process lifetime so the resolved function pointers
    /// stay valid.
    _lib: &'static libloading::Library,
}

impl Api {
    fn load(path: &Path) -> Api {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        // Leak so the mapping outlives every borrow; symbols are stored as raw
        // fn pointers below.
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));
        let run: IntFn = unsafe {
            *lib.get::<IntFn>(b"run\0")
                .unwrap_or_else(|e| panic!("{} does not export `run`: {e}", path.display()))
        };
        let driver: IntFn = unsafe {
            *lib.get::<IntFn>(b"driver\0")
                .unwrap_or_else(|e| panic!("{} does not export `driver`: {e}", path.display()))
        };
        Api {
            run,
            driver,
            _lib: lib,
        }
    }

    fn func(&self, entry: Entry) -> IntFn {
        match entry {
            Entry::Run => self.run,
            Entry::Driver => self.driver,
        }
    }
}

pub struct Harness {
    c: Api,
    rust: Api,
    /// Output of `run(0)` from the *pristine* (never-yet-called) global state,
    /// captured during harness construction so it is observable regardless of
    /// which `#[test]` runs first. `(c_bytes, rust_bytes)`.
    pristine_run0: (Vec<u8>, Vec<u8>),
    /// Scratch file used as the `stdout` redirection target.
    scratch: File,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Prefer the release cdylib (the shipping artifact); fall back to debug.
    let release = manifest.join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    let debug = manifest.join("target/debug/libdriver.so");
    assert!(
        debug.exists(),
        "Rust cdylib not found at {} or {}.\nBuild it first:\n  cd translation && cargo build --release",
        release.display(),
        debug.display()
    );
    debug
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// The `stdout` capture in `Harness::capture` redirects file descriptor 1 for
/// the whole process. libtest's own progress output ("test foo ... ", "ok")
/// also goes to file descriptor 1, and it is emitted by the harness thread
/// *concurrently* with other tests' bodies when more than one test thread is
/// used. That noise would land inside a capture and be diffed as if the library
/// had produced it.
///
/// Rather than let that turn into a flaky failure, require single-threaded
/// execution and say so precisely. `translation/.cargo/config.toml` sets
/// `RUST_TEST_THREADS=1` so a plain `cargo test` already satisfies this; an
/// explicit `--test-threads=1` on the command line works too.
fn assert_single_threaded() {
    let args: Vec<String> = std::env::args().collect();
    let mut cli: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--test-threads" {
            cli = args.get(i + 1).cloned();
        } else if let Some(v) = args[i].strip_prefix("--test-threads=") {
            cli = Some(v.to_string());
        }
        i += 1;
    }

    let effective = cli
        .or_else(|| std::env::var("RUST_TEST_THREADS").ok())
        .unwrap_or_default();

    assert_eq!(
        effective.trim(),
        "1",
        "\n\nThese differential tests capture file descriptor 1 to compare what each \
         .so writes to stdout, which requires single-threaded execution so that \
         libtest's own progress output cannot land inside a capture.\n\
         Run them as:\n\
         \n    cd translation && cargo test -- --test-threads=1\n\
         \nor rely on translation/.cargo/config.toml, which sets RUST_TEST_THREADS=1.\n\
         (effective --test-threads was {effective:?})\n"
    );
}

/// Acquire exclusive access to the loaded libraries and the `stdout` redirection.
pub fn harness() -> MutexGuard<'static, Harness> {
    assert_single_threaded();
    HARNESS
        .get_or_init(|| {
            let c = Api::load(&c_so_path());
            let rust = Api::load(&rust_so_path());

            let scratch_path = std::env::temp_dir().join(format!(
                "driver_diff_capture_{}_{:?}.txt",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let scratch = File::options()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&scratch_path)
                .expect("failed to create stdout capture scratch file");
            // Unlink immediately: the fd keeps it alive, nothing is left behind.
            let _ = std::fs::remove_file(&scratch_path);

            let mut h = Harness {
                c,
                rust,
                pristine_run0: (Vec::new(), Vec::new()),
                scratch,
            };

            // The very first call each library ever receives, from pristine
            // state, with a no-op delta.
            let cf = h.c.run;
            let rf = h.rust.run;
            let c_out = h.capture(|| unsafe { cf(0) });
            let r_out = h.capture(|| unsafe { rf(0) });
            h.pristine_run0 = (c_out, r_out);
            Mutex::new(h)
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

impl Harness {
    /// Run `f` with fd 1 redirected to the scratch file and return everything
    /// written to it (by any library, through any `FILE*` buffering).
    fn capture<F: FnOnce()>(&mut self, f: F) -> Vec<u8> {
        unsafe {
            // Flush our own buffered Rust stdout and every libc stream so that
            // nothing pre-existing lands in the capture.
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());

            self.scratch
                .set_len(0)
                .expect("failed to truncate capture file");
            self.scratch
                .seek(SeekFrom::Start(0))
                .expect("failed to rewind capture file");

            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(self.scratch.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

            f();

            // Redirected stdout is fully buffered; force it out before restoring.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "failed to restore fd 1");
            close(saved);

            self.scratch
                .seek(SeekFrom::Start(0))
                .expect("failed to rewind capture file for reading");
            let mut buf = Vec::new();
            self.scratch
                .read_to_end(&mut buf)
                .expect("failed to read capture file");
            buf
        }
    }

    /// The pristine-state `run(0)` output, `(c, rust)`.
    pub fn pristine_run0(&self) -> (&[u8], &[u8]) {
        (&self.pristine_run0.0, &self.pristine_run0.1)
    }

    /// Drive one entry point in BOTH libraries with the same argument and return
    /// `(c_stdout, rust_stdout)`. Always calls C first, then Rust, so the two
    /// copies of the persistent global state advance identically.
    pub fn call_both(&mut self, entry: Entry, arg: i32) -> (Vec<u8>, Vec<u8>) {
        let cf = self.c.func(entry);
        let rf = self.rust.func(entry);
        let c_out = self.capture(|| unsafe { cf(arg as c_int) });
        let r_out = self.capture(|| unsafe { rf(arg as c_int) });
        (c_out, r_out)
    }

    /// Drive ONE library only. Used exclusively by the harness-integrity test
    /// (`tests/independence.rs`) to prove the two `.so` files are distinct
    /// implementations with independent state, i.e. that `call_both` is not
    /// accidentally comparing one library against itself. Using this desyncs
    /// the two copies of the global state, so it must not be used from the
    /// lock-step differential tests.
    pub fn call_one(&mut self, which: Which, entry: Entry, arg: i32) -> Vec<u8> {
        let f = match which {
            Which::C => self.c.func(entry),
            Which::Rust => self.rust.func(entry),
        };
        self.capture(|| unsafe { f(arg as c_int) })
    }

    /// `call_both` plus the byte-for-byte assertion.
    pub fn assert_same(&mut self, entry: Entry, arg: i32, ctx: &str) {
        let (c_out, r_out) = self.call_both(entry, arg);
        assert_eq!(
            c_out,
            r_out,
            "\n[{ctx}] divergence calling {}({arg}) [arg as u32 = 0x{:08x}]\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}\n",
            entry.name(),
            arg as u32,
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            r_out.len(),
            String::from_utf8_lossy(&r_out),
        );
        // Sanity: the C library must actually have produced the expected number
        // of lines, otherwise an empty==empty comparison would pass vacuously.
        let expected_lines = match entry {
            Entry::Run => 4,
            Entry::Driver => 8,
        };
        assert_eq!(
            c_out.iter().filter(|&&b| b == b'\n').count(),
            expected_lines,
            "[{ctx}] C {}({arg}) produced unexpected output, capture is not working: {:?}",
            entry.name(),
            String::from_utf8_lossy(&c_out)
        );
    }

    /// Parse the LAST line of a capture into `(floors, bedrooms, bathrooms)` so
    /// tests can steer the persistent global state to an exact value.
    pub fn parse_last_state(out: &[u8]) -> (i64, i64, String) {
        let text = String::from_utf8_lossy(out);
        let last = text
            .lines()
            .rfind(|l| !l.is_empty())
            .unwrap_or_else(|| panic!("no output lines to parse: {text:?}"));
        // "The house has %d floors, %d bedrooms, and %.1f bathrooms"
        let nums: Vec<&str> = last
            .split_whitespace()
            .filter(|t| {
                t.chars().next().map(|c| c.is_ascii_digit() || c == '-') == Some(true)
            })
            .collect();
        assert_eq!(nums.len(), 3, "unexpected line shape: {last:?}");
        let floors: i64 = nums[0].parse().expect("floors");
        let bedrooms: i64 = nums[1].trim_end_matches(',').parse().expect("bedrooms");
        (floors, bedrooms, nums[2].to_string())
    }

    /// Current `bedrooms` value of the (identical) global state, obtained by
    /// performing a zero-delta `run` on both libraries and reading it back.
    /// Costs one `run` on each side, which keeps them in lock-step.
    pub fn probe_bedrooms(&mut self) -> i64 {
        let (c_out, r_out) = self.call_both(Entry::Run, 0);
        assert_eq!(
            c_out,
            r_out,
            "divergence during state probe\n  C:    {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        Harness::parse_last_state(&c_out).1
    }

    /// Move `bedrooms` to exactly `target` using a single `run` with the
    /// wrapping delta that gets there, asserting agreement on the way.
    pub fn set_bedrooms(&mut self, target: i32, ctx: &str) {
        let current = self.probe_bedrooms();
        let delta = target.wrapping_sub(current as i32);
        self.assert_same(Entry::Run, delta, &format!("{ctx}/steer-to-{target}"));
        let now = self.probe_bedrooms();
        assert_eq!(
            now, target as i64,
            "[{ctx}] failed to steer bedrooms to {target}, landed on {now}"
        );
    }
}

/// Selects a single library for `Harness::call_one`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

/// Deterministic SplitMix64 — fixed seed per test row for reproducibility.
pub struct Rng(u64);

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

    /// Uniform over the whole `i32` domain (including both extremes).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The exhaustive boundary set for the single `int` parameter.
pub const BOUNDARY_ARGS: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

/// Assert on `c_char` size assumptions the harness relies on.
pub fn _static_asserts() {
    let _: c_char = 0;
}
