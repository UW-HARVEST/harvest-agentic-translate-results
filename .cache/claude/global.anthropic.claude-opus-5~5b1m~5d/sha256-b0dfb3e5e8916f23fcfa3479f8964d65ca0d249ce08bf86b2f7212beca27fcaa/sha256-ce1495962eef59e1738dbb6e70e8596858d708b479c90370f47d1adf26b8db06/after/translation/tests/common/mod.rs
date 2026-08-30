//! Shared differential-testing harness for the StaticLoop verification suite.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported C symbols — the Rust implementation is
//! never called directly as a Rust function, so the `#[no_mangle]`/`extern "C"`
//! export wrappers are part of what is under test.
//!
//! ## Why every loaded library is a private on-disk copy
//!
//! `static_sum` keeps its accumulator in a function-scope `static int`, i.e.
//! per-loaded-object mutable state. glibc's loader deduplicates `dlopen` by
//! `(st_dev, st_ino)`, so opening the same path twice hands back the *same*
//! object with the *same* already-mutated accumulator. To get a genuinely
//! fresh `sum == 0` instance for each test, the harness copies each `.so` to a
//! uniquely named temporary file (a real copy, so a distinct inode) and
//! `dlopen`s that. Copies are removed when the `Pair` is dropped.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Exported ABI under test
// ---------------------------------------------------------------------------

/// `int static_sum(int update);`
pub type SumFn = unsafe extern "C" fn(c_int) -> c_int;
/// `void driver(int stride);`
pub type DriverFn = unsafe extern "C" fn(c_int);
/// Deliberately *mis*-declared width, used to probe how each side truncates an
/// out-of-`int`-range argument arriving across the FFI boundary.
pub type SumFnWide = unsafe extern "C" fn(i64) -> c_int;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// `c_src/build/libStaticLoop.so`, built by CMake.
///
/// Overridable via `STATICLOOP_C_SO` so the same suite can be re-run against a
/// C library built at a different optimisation level. `sum += update` and
/// `i * stride` are signed-overflow UB in C, so agreement with the default
/// (`-O0`) CMake build alone would not prove the Rust matches an optimised
/// build; `check_optlevels.sh` uses this hook to check `-O0/-O1/-O2/-O3/-Os`.
pub fn c_so_path() -> PathBuf {
    if let Some(over) = std::env::var_os("STATICLOOP_C_SO") {
        let p = PathBuf::from(over);
        assert!(
            p.is_file(),
            "STATICLOOP_C_SO points at a nonexistent file: {}",
            p.display()
        );
        return p;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .parent()
        .expect("translation/ must have a parent directory");
    let p = repo_root.join("c_src/build/libStaticLoop.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nBuild it first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `target/{debug,release}/libStaticLoop.so`, i.e. the cdylib for whichever
/// profile the current test binary was built with. Derived from
/// `current_exe()` (`.../target/<profile>/deps/<test>-<hash>`) so that
/// `cargo test` and `cargo test --release` each pick up their own artifact.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent() // .../deps
        .and_then(Path::parent) // .../<profile>
        .expect("test binary should live in target/<profile>/deps/");
    let p = profile_dir.join("libStaticLoop.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {} — run `cargo build` for this profile first",
        p.display()
    );
    p
}

// ---------------------------------------------------------------------------
// Unique temp paths
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn unique_path(stem: &str, ext: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&dir).ok();
    dir.join(format!(
        "staticloop_diff_{}_{}_{}_{}{}",
        stem,
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0),
        ext
    ))
}

// ---------------------------------------------------------------------------
// A single loaded implementation
// ---------------------------------------------------------------------------

pub struct Impl {
    /// Kept alive for as long as the resolved function pointers are used.
    _lib: Library,
    copy_path: PathBuf,
    pub name: &'static str,
    pub static_sum: SumFn,
    pub driver: DriverFn,
    pub static_sum_wide: SumFnWide,
}

impl Impl {
    /// Copy `src` to a private temp path and `dlopen` the copy, yielding an
    /// instance whose `static int sum` is guaranteed to start at 0.
    fn load_fresh_copy(src: &Path, name: &'static str) -> Impl {
        let copy_path = unique_path(name, ".so");
        std::fs::copy(src, &copy_path).unwrap_or_else(|e| {
            panic!("copy {} -> {}: {e}", src.display(), copy_path.display())
        });

        // SAFETY: loading a verbatim copy of a library we just built.
        let lib = unsafe { Library::new(&copy_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", copy_path.display()));

        unsafe {
            let sum: Symbol<SumFn> = lib
                .get(b"static_sum\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym static_sum: {e}"));
            let drv: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym driver: {e}"));
            let wide: Symbol<SumFnWide> = lib
                .get(b"static_sum\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym static_sum: {e}"));

            Impl {
                static_sum: *sum,
                driver: *drv,
                static_sum_wide: *wide,
                name,
                copy_path,
                _lib: lib,
            }
        }
    }

    pub fn sum(&self, update: c_int) -> c_int {
        unsafe { (self.static_sum)(update) }
    }

    pub fn sum_wide(&self, update: i64) -> c_int {
        unsafe { (self.static_sum_wide)(update) }
    }

    pub fn run_driver(&self, stride: c_int) {
        unsafe { (self.driver)(stride) }
    }
}

impl Drop for Impl {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.copy_path);
    }
}

// ---------------------------------------------------------------------------
// A matched C/Rust pair, both starting from a pristine accumulator
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
    /// Human-readable description of the CONFIGS.md row being exercised.
    pub row: String,
}

impl Pair {
    /// Two brand-new instances, each with `sum == 0`.
    pub fn fresh(row: impl Into<String>) -> Pair {
        Pair {
            c: Impl::load_fresh_copy(&c_so_path(), "c"),
            rs: Impl::load_fresh_copy(&rust_so_path(), "rs"),
            row: row.into(),
        }
    }

    /// Call `static_sum(update)` on both and assert the returned running totals
    /// are bit-identical.
    #[track_caller]
    pub fn assert_sum(&self, update: c_int) -> c_int {
        let cv = self.c.sum(update);
        let rv = self.rs.sum(update);
        assert_eq!(
            cv, rv,
            "[{}] static_sum({update}) diverged: C returned {cv} (0x{cv:08x}), \
             Rust returned {rv} (0x{rv:08x})",
            self.row
        );
        cv
    }

    /// Same, but through the deliberately-widened declaration.
    #[track_caller]
    pub fn assert_sum_wide(&self, update: i64) -> c_int {
        let cv = self.c.sum_wide(update);
        let rv = self.rs.sum_wide(update);
        assert_eq!(
            cv, rv,
            "[{}] static_sum(<i64>{update:#018x}) diverged: C returned {cv}, Rust returned {rv}",
            self.row
        );
        cv
    }

    /// Run `driver(stride)` on both, capturing each side's stdout, and assert
    /// the emitted bytes are identical.
    #[track_caller]
    pub fn assert_driver(&self, stride: c_int) -> Vec<u8> {
        let c_out = capture_stdout(|| self.c.run_driver(stride));
        let rs_out = capture_stdout(|| self.rs.run_driver(stride));
        assert_eq!(
            c_out,
            rs_out,
            "[{}] driver({stride}) stdout diverged:\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            self.row,
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            rs_out.len(),
            String::from_utf8_lossy(&rs_out),
        );
        c_out
    }

    /// Drive the accumulator to exactly `target` on both sides, from fresh.
    /// Only valid on a pair whose `sum` is still 0.
    #[track_caller]
    pub fn seed_to(&self, target: c_int) {
        let got = self.assert_sum(target);
        assert_eq!(
            got, target,
            "[{}] seed_to({target}) expected the accumulator to land on {target}, got {got}",
            self.row
        );
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// `driver` writes through C `printf`, so the observable output is on file
/// descriptor 1 — not on Rust's `std::io::stdout`. Capture it by temporarily
/// redirecting fd 1 to a scratch file. Serialised process-wide, because fd 1
/// is global state.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Redirecting fd 1 is process-global, so it is only sound if no other test
/// thread can write to stdout while the redirect is installed. libtest's own
/// progress output (`test foo ... ok`) goes to fd 1 from the main thread, so
/// the suite MUST run single-threaded; `translation/.cargo/config.toml` sets
/// `RUST_TEST_THREADS=1` to guarantee that for a bare `cargo test`.
///
/// This is enforced rather than hoped for: a contaminated capture would either
/// produce a spurious failure or, worse, silently hide a real divergence.
fn require_serial_execution() {
    let single_thread_env = std::env::var("RUST_TEST_THREADS").ok().as_deref() == Some("1");
    let single_thread_arg = {
        let args: Vec<String> = std::env::args().collect();
        args.iter().any(|a| a == "--test-threads=1")
            || args
                .windows(2)
                .any(|w| w[0] == "--test-threads" && w[1] == "1")
    };
    assert!(
        single_thread_env || single_thread_arg,
        "stdout-capturing tests require single-threaded execution (fd 1 is \
         process-global and libtest also writes to it).\n\
         Run:  cargo test -- --test-threads=1\n\
         or:   RUST_TEST_THREADS=1 cargo test\n\
         (translation/.cargo/config.toml normally sets this automatically)"
    );
}

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    require_serial_execution();
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = unique_path("stdout", ".txt");
    let cpath = CString::new(path.to_str().expect("utf-8 temp path")).unwrap();

    // Drain Rust's own buffered stdout (libtest's `LineWriter` holds partial
    // lines such as "test row32 ... " with no trailing newline) so it cannot be
    // flushed into our scratch file once fd 1 is redirected.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    unsafe {
        // Flush anything already buffered in C stdio so it lands on the real
        // stdout rather than in the capture.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2(fd, 1) failed");

        f();

        // Flush the library's buffered `printf` output into the scratch file
        // before restoring fd 1.
        libc::fflush(std::ptr::null_mut());

        assert!(libc::dup2(saved, 1) >= 0, "restoring fd 1 failed");
        libc::close(saved);
        libc::close(fd);
    }

    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);
    data
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const BASE_SEED: u64 = 0x5EED_1234_ABCD_0001;

    /// Derive a stream from the fixed base seed plus a per-row salt, so each
    /// CONFIGS.md row gets its own reproducible sequence.
    pub fn for_row(row: u64) -> Rng {
        Rng(Self::BASE_SEED ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform over the whole `i32` range, including `INT_MIN`/`INT_MAX`.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `lo..=hi`.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}
