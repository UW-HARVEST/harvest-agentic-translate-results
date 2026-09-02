// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
// call goes through `dlsym`'d symbols, so the `#[no_mangle] extern "C"` export
// wrappers are part of what is under test. Rust functions are never called
// directly.
//
// The library's only observable output is what it `printf`s to stdout, so a
// "step" is: redirect fd 1 to a scratch file, call the C export, snapshot the
// bytes; then do the same for the Rust export; then compare.
//
// Both libraries keep their OWN copy of the file-scope `static house_t
// the_house`, and every call mutates it. To keep the two copies in lockstep,
// every step must deliver the same call to C and then to Rust while holding the
// global harness lock. Steps from different `#[test]` threads may interleave,
// but because each step is atomic both libraries observe the *same* global call
// sequence, so their states stay identical.

#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::Library;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Run `f` with fd 1 redirected to a scratch file and return the bytes it
/// wrote. `fflush(NULL)` is issued before and after so that libc's stdio buffer
/// (fully buffered when stdout is a pipe, as under `cargo test`) is drained at
/// exactly the right moments.
///
/// fd 1 is restored and the scratch file removed by a drop guard, so a panic
/// inside `f` cannot leave the rest of the process writing into a temp file.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    struct Restore {
        saved: i32,
        fd: i32,
        path: PathBuf,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                libc::fflush(std::ptr::null_mut());
                libc::dup2(self.saved, 1);
                libc::close(self.saved);
                libc::close(self.fd);
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    let n = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("driver-diff-{}-{}.out", std::process::id(), n));
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).expect("no interior NUL");

    let guard = unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");
        Restore {
            saved,
            fd,
            path: path.clone(),
        }
    };

    f();

    // Drain libc's buffer and put fd 1 back before reading the file.
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let bytes = std::fs::read(&path).expect("read capture file");
    drop(guard);
    bytes
}

// ---------------------------------------------------------------------------
// The pair of libraries
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(c_int);

pub struct Pair {
    c: Library,
    r: Library,
}

static PAIR: OnceLock<Mutex<Pair>> = OnceLock::new();

/// Acquire the global harness. Poisoning is ignored: a failing assertion in one
/// test must not make every other test report a bogus "poisoned" error.
pub fn pair() -> MutexGuard<'static, Pair> {
    let m = PAIR.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared object not found at {} — build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            cp.display()
        );
        assert!(
            rp.exists(),
            "Rust shared object not found at {} — build it with:\n  cd translation && cargo build --release",
            rp.display()
        );
        let c = unsafe { Library::new(&cp) }.expect("dlopen C .so");
        let r = unsafe { Library::new(&rp) }.expect("dlopen Rust .so");
        Mutex::new(Pair { c, r })
    });
    match m.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

impl Pair {
    fn c_driver(&self) -> libloading::Symbol<'_, DriverFn> {
        unsafe { self.c.get(b"driver\0") }.expect("C .so exports `driver`")
    }
    fn r_driver(&self) -> libloading::Symbol<'_, DriverFn> {
        unsafe { self.r.get(b"driver\0") }.expect("Rust .so exports `driver`")
    }
    fn c_run(&self) -> libloading::Symbol<'_, RunFn> {
        unsafe { self.c.get(b"run\0") }.expect("C .so exports `run`")
    }
    fn r_run(&self) -> libloading::Symbol<'_, RunFn> {
        unsafe { self.r.get(b"run\0") }.expect("Rust .so exports `run`")
    }

    /// One `run(n)` step against both libraries.
    pub fn run_step(&self, n: i32) -> (Vec<u8>, Vec<u8>) {
        let cf = self.c_run();
        let rf = self.r_run();
        let c = capture(|| unsafe { cf(n as c_int) });
        let r = capture(|| unsafe { rf(n as c_int) });
        (c, r)
    }

    /// One `driver(bytes)` step against both libraries. `bytes` is passed
    /// exactly as given plus a terminating NUL, so inputs with embedded NULs
    /// and non-UTF-8 bytes can be exercised.
    pub fn driver_step_raw(&self, bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut buf = bytes.to_vec();
        buf.push(0);
        let cf = self.c_driver();
        let rf = self.r_driver();
        let c = capture(|| unsafe { cf(buf.as_ptr() as *const c_char) });
        let r = capture(|| unsafe { rf(buf.as_ptr() as *const c_char) });
        (c, r)
    }

    /// `driver` with a stale non-zero `errno` installed before each call
    /// (ERRORS.md rows 8/9).
    pub fn driver_step_with_errno(&self, bytes: &[u8], errno_value: i32) -> (Vec<u8>, Vec<u8>) {
        let mut buf = bytes.to_vec();
        buf.push(0);
        let cf = self.c_driver();
        let rf = self.r_driver();
        let c = capture(|| unsafe {
            *libc::__errno_location() = errno_value;
            cf(buf.as_ptr() as *const c_char)
        });
        let r = capture(|| unsafe {
            *libc::__errno_location() = errno_value;
            rf(buf.as_ptr() as *const c_char)
        });
        (c, r)
    }

    /// `driver` with a stale `errno` installed before the call, also reporting
    /// the `errno` value observed immediately after the call returns — `errno`
    /// is a caller-visible side effect of `parse_val` (ERRORS.md rows 2-5, 8, 9).
    pub fn driver_step_errno(
        &self,
        bytes: &[u8],
        pre: i32,
    ) -> ((Vec<u8>, i32), (Vec<u8>, i32)) {
        let mut buf = bytes.to_vec();
        buf.push(0);
        let cf = self.c_driver();
        let rf = self.r_driver();
        let ce = std::cell::Cell::new(i32::MIN);
        let re = std::cell::Cell::new(i32::MIN);
        let c = capture(|| unsafe {
            *libc::__errno_location() = pre;
            cf(buf.as_ptr() as *const c_char);
            ce.set(*libc::__errno_location());
        });
        let r = capture(|| unsafe {
            *libc::__errno_location() = pre;
            rf(buf.as_ptr() as *const c_char);
            re.set(*libc::__errno_location());
        });
        ((c, ce.get()), (r, re.get()))
    }

    /// Raw function pointers, for the forked-child null-pointer test.
    pub fn raw_drivers(&self) -> (DriverFn, DriverFn) {
        (*self.c_driver(), *self.r_driver())
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Assert the two captures are byte-identical.
#[track_caller]
pub fn same(label: &str, c: &[u8], r: &[u8]) {
    assert_eq!(
        c,
        r,
        "\ndivergence in {label}\n  C   ({} bytes): \"{}\"\n  Rust({} bytes): \"{}\"\n",
        c.len(),
        show(c),
        r.len(),
        show(r)
    );
}

pub const ERROR_LINE: &[u8] = b"An error occurred\n";

/// The four `The house has …` lines a successful `run` prints.
pub fn is_four_house_lines(b: &[u8]) -> bool {
    let s = match std::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let lines: Vec<&str> = s.lines().collect();
    lines.len() == 4 && lines.iter().all(|l| l.starts_with("The house has "))
}

/// Parse `floors`, `bedrooms`, `bathrooms` out of the last `The house has …`
/// line of a capture. Used to build state-targeted inputs (CONFIGS.md row 20).
pub fn parse_last_state(b: &[u8]) -> Option<(i32, i32, f64)> {
    let s = std::str::from_utf8(b).ok()?;
    let line = s.lines().filter(|l| l.starts_with("The house has ")).last()?;
    // The house has %d floors, %d bedrooms, and %.1f bathrooms
    let rest = line.strip_prefix("The house has ")?;
    let (floors, rest) = rest.split_once(" floors, ")?;
    let (bedrooms, rest) = rest.split_once(" bedrooms, and ")?;
    let bathrooms = rest.strip_suffix(" bathrooms")?;
    Some((
        floors.parse().ok()?,
        bedrooms.parse().ok()?,
        bathrooms.parse().ok()?,
    ))
}

pub fn cstr_bytes(c: &CStr) -> &[u8] {
    c.to_bytes()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every run reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        self.range_i64(lo as i64, hi as i64) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range_usize(0, xs.len() - 1)]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Random ASCII whitespace run that `strtol` must skip.
pub fn ws(rng: &mut Rng, max: usize) -> Vec<u8> {
    let n = rng.range_usize(1, max);
    let set = [b' ', b'\t', b'\n', b'\x0b', b'\x0c', b'\r'];
    (0..n).map(|_| *rng.pick(&set)).collect()
}
