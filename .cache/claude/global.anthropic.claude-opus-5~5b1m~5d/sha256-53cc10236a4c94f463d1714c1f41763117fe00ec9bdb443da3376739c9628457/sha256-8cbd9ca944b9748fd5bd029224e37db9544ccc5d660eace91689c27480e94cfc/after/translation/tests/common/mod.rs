//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and every call
//! goes through the exported C-ABI symbol, so the `#[no_mangle]` wrappers are
//! part of what is under test. No Rust function is ever called directly.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_SO") {
        return PathBuf::from(p);
    }
    let base = workspace_root().join("translation/target");
    // Prefer the profile the test binary itself was built with, then the other.
    let mut order = vec!["debug", "release"];
    if cfg!(not(debug_assertions)) {
        order.reverse();
    }
    for prof in order {
        let p = base.join(prof).join("libdoubleneg_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {}; build it with `cargo build` / `cargo build --release`",
        base.display()
    );
}

/// The two loaded shared objects.
pub struct Pair {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        // SAFETY: both paths point at ordinary shared objects with no
        // constructors that could violate Rust's invariants.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let r = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));
        Pair {
            c,
            r,
            c_path,
            r_path,
        }
    }

    /// Fetch the same symbol from both libraries.
    ///
    /// # Safety
    /// `T` must match the C signature of `name`.
    pub unsafe fn both<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let n = String::from_utf8_lossy(name).to_string();
        let cs = unsafe { self.c.get::<T>(name) }
            .unwrap_or_else(|e| panic!("C .so is missing symbol {n}: {e}"));
        let rs = unsafe { self.r.get::<T>(name) }
            .unwrap_or_else(|e| panic!("Rust .so is missing symbol {n}: {e}"));
        (cs, rs)
    }
}

// ---------------------------------------------------------------------------
// Function types (mirroring c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

pub type FnConvertDoubleToInt = unsafe extern "C" fn(f64) -> i32;
pub type FnFindValueInBuffer = unsafe extern "C" fn(*const i8, usize, i32) -> i32;
pub type FnProcessNegation = unsafe extern "C" fn(i32) -> i32;
pub type FnCreateNumericBuffer = unsafe extern "C" fn(*mut i8, i32, i32);
pub type FnCalculateWithDoubles = unsafe extern "C" fn(i32, i32, i32) -> f64;
pub type FnDoubleneg = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

pub const SYM_CONVERT: &[u8] = b"convert_double_to_int\0";
pub const SYM_FIND: &[u8] = b"find_value_in_buffer\0";
pub const SYM_NEG: &[u8] = b"process_negation\0";
pub const SYM_CREATE: &[u8] = b"create_numeric_buffer\0";
pub const SYM_CALC: &[u8] = b"calculate_with_doubles\0";
pub const SYM_DOUBLENEG: &[u8] = b"doubleneg\0";

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, property-style testing)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Rng {
        // Avoid the xorshift fixed point at 0.
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// A raw bit pattern reinterpreted as `f64` — covers every class
    /// (normal, subnormal, zero, infinity, quiet/signalling NaN).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// A "reasonable" finite double spread over many magnitudes.
    pub fn next_f64_scaled(&mut self) -> f64 {
        let mantissa = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let exp = self.below(41) as i32 - 8; // 1e-8 .. 1e32
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * mantissa * 10f64.powi(exp)
    }
}

// ---------------------------------------------------------------------------
// Bitwise double comparison (so -0.0 vs 0.0 and NaN payloads are caught)
// ---------------------------------------------------------------------------

pub fn same_f64_bits(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn show_f64(v: f64) -> String {
    format!("{v:?} (bits 0x{:016x})", v.to_bits())
}

// ---------------------------------------------------------------------------
// stdout capture — `doubleneg` prints through libc `printf`
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes every open C stream.
    fn fflush(stream: *mut c_void) -> i32;
}

/// Run `f` with fd 1 redirected into a temporary file and return both the
/// closure's value and the captured bytes.
///
/// The C and the Rust library share the process's libc, so `fflush(NULL)`
/// drains whichever `stdout` buffer either of them wrote into.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "doubleneg_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let out = capture_stdout_to(&path, f);
    let _ = std::fs::remove_file(&path);
    out
}

fn capture_stdout_to<R>(path: &Path, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::Write;
    std::io::stdout().flush().ok();
    let file = std::fs::File::create(path).expect("create capture file");
    // SAFETY: plain POSIX fd juggling; the saved descriptor is restored below.
    let (saved, value) = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        let value = f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        (saved, value)
    };
    let _ = saved;
    drop(file);
    let bytes = std::fs::read(path).expect("read capture file");
    (value, bytes)
}

/// Pretty-print the first difference between two byte streams.
pub fn diff_report(label: &str, a: &[u8], b: &[u8]) -> String {
    let pos = a.iter().zip(b.iter()).position(|(x, y)| x != y);
    let mut s = format!(
        "{label}: stdout differs (C {} bytes, Rust {} bytes)\n",
        a.len(),
        b.len()
    );
    match pos {
        Some(i) => {
            let lo = i.saturating_sub(80);
            s.push_str(&format!("first difference at byte {i}\n"));
            s.push_str(&format!("  C   : {:?}\n", String::from_utf8_lossy(&a[lo..(i + 80).min(a.len())])));
            s.push_str(&format!("  Rust: {:?}\n", String::from_utf8_lossy(&b[lo..(i + 80).min(b.len())])));
        }
        None => {
            let n = a.len().min(b.len());
            s.push_str(&format!("common prefix of {n} bytes is equal; lengths differ\n"));
            if a.len() > n {
                s.push_str(&format!("  C   extra: {:?}\n", String::from_utf8_lossy(&a[n..])));
            }
            if b.len() > n {
                s.push_str(&format!("  Rust extra: {:?}\n", String::from_utf8_lossy(&b[n..])));
            }
        }
    }
    s
}
