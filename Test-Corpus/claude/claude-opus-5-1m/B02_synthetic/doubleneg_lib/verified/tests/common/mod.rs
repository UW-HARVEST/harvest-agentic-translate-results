// Shared differential-test harness.
//
// Loads BOTH shared libraries (the C reference `.so` and the Rust `.so`) through
// `libloading` and exposes their exported symbols as plain `extern "C"` function
// pointers.  Rust functions are NEVER called directly from the test crate: every
// call goes through the dynamic-symbol table exactly as an external C consumer
// would, so the `#[no_mangle]` export wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Exported signatures (from c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub type FnConvertDoubleToInt = unsafe extern "C" fn(f64) -> c_int;
pub type FnFindValueInBuffer = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
pub type FnProcessNegation = unsafe extern "C" fn(c_int) -> c_int;
pub type FnCreateNumericBuffer = unsafe extern "C" fn(*mut c_char, c_int, c_int);
pub type FnCalculateWithDoubles = unsafe extern "C" fn(c_int, c_int, c_int) -> f64;
pub type FnDoubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// The six symbols the C `.so` exports, resolved out of one library.
pub struct Api {
    pub name: &'static str,
    pub convert_double_to_int: FnConvertDoubleToInt,
    pub find_value_in_buffer: FnFindValueInBuffer,
    pub process_negation: FnProcessNegation,
    pub create_numeric_buffer: FnCreateNumericBuffer,
    pub calculate_with_doubles: FnCalculateWithDoubles,
    pub doubleneg: FnDoubleneg,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }.unwrap_or_else(|e| {
        panic!(
            "symbol {:?} missing from shared library: {e}",
            String::from_utf8_lossy(name)
        )
    });
    *s
}

unsafe fn load_api(path: &PathBuf, name: &'static str) -> Api {
    assert!(
        path.exists(),
        "shared library not found: {}\n\
         Build the C side with:\n  \
           cd c_src && mkdir -p build && cd build && cmake .. \
           -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust side with:\n  cargo build",
        path.display()
    );
    // Leaked so the resolved function pointers stay valid for the whole process.
    let lib: &'static Library = Box::leak(Box::new(
        unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display())),
    ));
    Api {
        name,
        convert_double_to_int: unsafe { sym(lib, b"convert_double_to_int\0") },
        find_value_in_buffer: unsafe { sym(lib, b"find_value_in_buffer\0") },
        process_negation: unsafe { sym(lib, b"process_negation\0") },
        create_numeric_buffer: unsafe { sym(lib, b"create_numeric_buffer\0") },
        calculate_with_doubles: unsafe { sym(lib, b"calculate_with_doubles\0") },
        doubleneg: unsafe { sym(lib, b"doubleneg\0") },
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so` (CMake names the target after the parent
/// directory of `c_src`, which is the crate root `translated_rust`).
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("c_src").join("build");
    for cand in ["libtranslated_rust.so", "libc_src.so"] {
        let p = build.join(cand);
        if p.exists() {
            return p;
        }
    }
    // Fall back to whatever single .so CMake produced.
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                return p;
            }
        }
    }
    build.join("libtranslated_rust.so")
}

/// `target/<profile>/libdoubleneg_lib.so` — derived from the running test
/// binary's location (`target/<profile>/deps/<test>-<hash>`) so it is correct
/// for every profile and feature combination.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let name = "libdoubleneg_lib.so";
    for dir in [profile, deps] {
        let p = dir.join(name);
        if p.exists() {
            return p;
        }
    }
    profile.join(name)
}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` lib target (only
/// `cargo build` emits the `.so`), so without this guard the suite would happily
/// keep testing a stale library and every mutation would silently "pass".
/// Refuse to run if the `.so` is older than any input that produced it.
fn assert_not_stale(so: &std::path::Path, sources: &[PathBuf], what: &str) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => panic!("cannot stat {}: {e}", so.display()),
    };
    for src in sources {
        let Ok(src_mtime) = std::fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            src_mtime <= so_mtime,
            "STALE {what} LIBRARY\n  \
             {} is newer than\n  {}\n\n\
             `cargo test` does not rebuild a cdylib-only target. Rebuild first:\n  \
             cargo build --no-default-features   # Rust .so\n  \
             cmake --build c_src/build            # C .so\n\
             or just run ./run_all.sh, which does both before testing.",
            src.display(),
            so.display()
        );
    }
}

fn collect_files(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_files(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c, rust)` — the two implementations under differential test.
pub fn apis() -> &'static (Api, Api) {
    APIS.get_or_init(|| {
        let c_so = c_so_path();
        let rust_so = rust_so_path();

        let mut rust_srcs = Vec::new();
        collect_files(&manifest_dir().join("src"), &mut rust_srcs);
        rust_srcs.push(manifest_dir().join("Cargo.toml"));
        assert_not_stale(&rust_so, &rust_srcs, "RUST");

        let mut c_srcs = Vec::new();
        collect_files(&manifest_dir().join("c_src").join("src"), &mut c_srcs);
        collect_files(&manifest_dir().join("c_src").join("include"), &mut c_srcs);
        assert_not_stale(&c_so, &c_srcs, "C");

        unsafe { (load_api(&c_so, "C"), load_api(&rust_so, "Rust")) }
    })
}

pub fn c() -> &'static Api {
    &apis().0
}

pub fn rs() -> &'static Api {
    &apis().1
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every test reproducible.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform over the entire `i32` domain (includes INT_MIN / INT_MAX).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    pub fn range_i32(&mut self, lo: i32, hi_inclusive: i32) -> i32 {
        let span = (hi_inclusive as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// An `int` biased toward the values the C code branches on, while still
    /// reaching the full domain.
    pub fn interesting_i32(&mut self) -> i32 {
        const EXTREMES: [i32; 18] = [
            0,
            1,
            -1,
            2,
            -2,
            7,
            -7,
            10,
            -10,
            42,
            255,
            256,
            -256,
            1000,
            -1000,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
        ];
        match self.below(4) {
            0 => EXTREMES[self.below(EXTREMES.len() as u64) as usize],
            1 => self.range_i32(-1000, 1000),
            2 => self.range_i32(-100_000, 100_000),
            _ => self.next_i32(),
        }
    }

    /// An arbitrary `f64` bit pattern: mixes every IEEE class (zeros,
    /// subnormals, normals, infinities, NaNs) with `int`-boundary values.
    pub fn interesting_f64(&mut self) -> f64 {
        const SPECIALS: [f64; 20] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            1.9999,
            -1.9999,
            f64::MIN_POSITIVE,
            5e-324,
            -5e-324,
            2147483647.0,
            2147483647.5,
            -2147483648.0,
            -2147483648.5,
            2147483648.0,
            -2147483649.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        match self.below(4) {
            0 => SPECIALS[self.below(SPECIALS.len() as u64) as usize],
            1 => {
                // Dense around the int range so truncation is exercised.
                let v = self.next_i32() as f64;
                v + (self.next_u32() as f64 / u32::MAX as f64) - 0.5
            }
            2 => f64::from_bits(self.next_u64()),
            _ => {
                let m = self.next_u32() as f64 / 4096.0;
                if self.below(2) == 0 { m } else { -m }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Compare `f64` results by raw bits so `-0.0` vs `0.0` and differing NaN
/// payloads are treated as divergences (they are observable through `%e`).
#[track_caller]
pub fn assert_f64_bits_eq(cv: f64, rv: f64, ctx: &str) {
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "f64 divergence for {ctx}: C = {cv:?} (bits {:#018x}) vs Rust = {rv:?} (bits {:#018x})",
        cv.to_bits(),
        rv.to_bits()
    );
}

#[track_caller]
pub fn assert_i32_eq(cv: i32, rv: i32, ctx: &str) {
    assert_eq!(cv, rv, "int divergence for {ctx}: C = {cv} vs Rust = {rv}");
}

#[track_caller]
pub fn assert_bytes_eq(cv: &[u8], rv: &[u8], ctx: &str) {
    if cv == rv {
        return;
    }
    let at = cv
        .iter()
        .zip(rv.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| cv.len().min(rv.len()));
    panic!(
        "byte divergence for {ctx}: len C = {}, len Rust = {}, first differing index {at}\n\
         C    = {:?}\nRust = {:?}",
        cv.len(),
        rv.len(),
        &cv[at.saturating_sub(4)..(at + 8).min(cv.len())],
        &rv[at.saturating_sub(4)..(at + 8).min(rv.len())],
    );
}

// ---------------------------------------------------------------------------
// stdout capture — `doubleneg` writes to the process's libc stdout, which both
// libraries share.  Redirect fd 1 to a temp file around each call so the two
// implementations' output can be diffed byte-for-byte.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, which avoids having to
    /// bind glibc's `stdout` global.
    fn fflush(stream: *mut c_void) -> c_int;
}

static CAPTURE_SEQ: Mutex<u64> = Mutex::new(0);
static REDIRECT_ACTIVE: AtomicBool = AtomicBool::new(false);

use std::sync::atomic::{AtomicBool, Ordering};

/// Runs `f` with fd 1 pointed at `target_fd`, restoring fd 1 afterwards.
///
/// fd 1 is process-global, so this must never be nested and never run
/// concurrently with another test in the same binary — every test that touches
/// stdout therefore lives in a test binary that contains exactly one `#[test]`.
/// The `REDIRECT_ACTIVE` flag turns a violation of that rule into a loud panic
/// instead of silently corrupting the captured bytes.
fn with_fd1_redirected<R>(target_fd: c_int, f: impl FnOnce() -> R) -> R {
    use std::io::Write;

    /// Restores fd 1 in `Drop`, so a panic inside `f` (i.e. a detected
    /// divergence) cannot leave stdout pointing at the capture file / /dev/null.
    /// Without this, libtest's own "test result: FAILED" and the panic message
    /// would be swallowed and a real failure could look like a pass.
    struct Restore(c_int);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                fflush(std::ptr::null_mut());
                dup2(self.0, 1);
                close(self.0);
            }
            REDIRECT_ACTIVE.store(false, Ordering::SeqCst);
        }
    }

    assert!(
        !REDIRECT_ACTIVE.swap(true, Ordering::SeqCst),
        "fd 1 redirection is already active: stdout-capturing tests must not be \
         nested or run concurrently (keep them in a single-#[test] binary)"
    );

    // Flush the Rust-side buffered stdout (the libtest harness's own progress
    // text) so it cannot land inside the capture file.
    let _ = std::io::stdout().flush();

    let restore = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        // Build the guard BEFORE the dup2 can fail, so fd 1 is always restored.
        let restore = Restore(saved);
        assert!(dup2(target_fd, 1) >= 0, "dup2 onto fd 1 failed");
        restore
    };

    let value = f();
    drop(restore);
    value
}

/// Runs `f` with fd 1 redirected to a temp file; returns `(f's value, stdout bytes)`.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::os::unix::io::AsRawFd;

    let seq = {
        let mut s = CAPTURE_SEQ.lock().unwrap_or_else(|e| e.into_inner());
        *s += 1;
        *s
    };
    let path = std::env::temp_dir().join(format!("dnf-capture-{}-{seq}.bin", std::process::id()));

    let file = std::fs::File::create(&path)
        .unwrap_or_else(|e| panic!("cannot create capture file {}: {e}", path.display()));
    let value = with_fd1_redirected(file.as_raw_fd(), f);
    drop(file);

    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (value, bytes)
}

/// Runs a whole batch of calls with fd 1 pointed at `/dev/null`, for the rows
/// that only compare return values (`doubleneg` prints ~1.5 KiB per call).
pub fn silence_stdout<R>(f: impl FnOnce() -> R) -> R {
    let devnull = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("open /dev/null");
    use std::os::unix::io::AsRawFd;
    let r = with_fd1_redirected(devnull.as_raw_fd(), f);
    drop(devnull);
    r
}
