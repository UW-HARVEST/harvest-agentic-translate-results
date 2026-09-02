//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes their exported
//! symbols as plain `extern "C"` function pointers. Nothing in this module
//! ever calls a Rust function directly — every call crosses the FFI boundary
//! exactly as an external consumer's would, so the `#[no_mangle]` wrappers are
//! under test too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

pub type FnConvert = unsafe extern "C" fn(f64) -> c_int;
pub type FnFind = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
pub type FnNegation = unsafe extern "C" fn(c_int) -> c_int;
pub type FnCreate = unsafe extern "C" fn(*mut c_char, c_int, c_int);
pub type FnCalc = unsafe extern "C" fn(c_int, c_int, c_int) -> f64;
pub type FnDoubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    // Kept alive for the whole process: the raw function pointers below point
    // into this mapping. The struct is stored in a `OnceLock` and never dropped.
    _lib: Library,
    pub convert_double_to_int: FnConvert,
    pub find_value_in_buffer: FnFind,
    pub process_negation: FnNegation,
    pub create_numeric_buffer: FnCreate,
    pub calculate_with_doubles: FnCalc,
    pub doubleneg: FnDoubleneg,
}

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        // SAFETY: the libraries are plain C ABI shared objects with no
        // initialisers that run arbitrary code beyond libc's own.
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));

            macro_rules! sym {
                ($t:ty, $n:literal) => {{
                    let s = lib
                        .get::<$t>(concat!($n, "\0").as_bytes())
                        .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, $n));
                    *s
                }};
            }

            let convert_double_to_int = sym!(FnConvert, "convert_double_to_int");
            let find_value_in_buffer = sym!(FnFind, "find_value_in_buffer");
            let process_negation = sym!(FnNegation, "process_negation");
            let create_numeric_buffer = sym!(FnCreate, "create_numeric_buffer");
            let calculate_with_doubles = sym!(FnCalc, "calculate_with_doubles");
            let doubleneg = sym!(FnDoubleneg, "doubleneg");

            Api {
                name,
                path,
                _lib: lib,
                convert_double_to_int,
                find_value_in_buffer,
                process_negation,
                create_numeric_buffer,
                calculate_with_doubles,
                doubleneg,
            }
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    let root = workspace_root().join("translation");
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libdoubleneg_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libdoubleneg_lib.so not found under {}/target/{{release,debug}}. \
         Build it first: cd translation && cargo build --release",
        root.display()
    )
}

/// Both implementations, loaded once per test binary.
pub fn apis() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Api::load("C", find_c_so()),
        rust: Api::load("Rust", find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// An `i32` biased toward interesting values (extremes, small magnitudes,
    /// byte boundaries) but still covering the full range.
    pub fn spicy_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 20] = [
            0,
            1,
            -1,
            2,
            -2,
            7,
            10,
            42,
            100,
            127,
            -128,
            255,
            256,
            -255,
            -256,
            1000,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len() as u64) as usize],
            1 => (self.next_u64() % 512) as i32 - 256,
            _ => self.next_i32(),
        }
    }

    /// An `f64` biased toward the classes `convert_double_to_int` distinguishes.
    pub fn spicy_f64(&mut self) -> f64 {
        const SPECIAL: [f64; 24] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            0.9999999999,
            -0.9999999999,
            42.7,
            -42.7,
            2147483647.0,
            2147483646.5,
            2147483647.5,
            2147483648.0,
            -2147483648.0,
            -2147483648.5,
            -2147483649.0,
            1e9,
            -1e9,
            1e300,
            -1e300,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        match self.below(5) {
            0 => SPECIAL[self.below(SPECIAL.len() as u64) as usize],
            1 => f64::from_bits(self.next_u64()),
            2 => (self.next_i32() as f64) + (self.next_u32() as f64) / (u32::MAX as f64),
            3 => self.next_i32() as f64,
            _ => {
                let m = self.next_u64() as f64 / u64::MAX as f64;
                let e = self.below(64) as i32 - 32;
                let s = if self.below(2) == 0 { 1.0 } else { -1.0 };
                s * m * 2f64.powi(e)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture — `doubleneg` is observable through 18 `printf` calls, so the
// printed bytes are part of the contract, not just the return value.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run `f`, returning its value plus everything it wrote to fd 1.
///
/// Serialised process-wide: fd 1 is a shared resource and the test harness is
/// multi-threaded.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let tmp = std::env::temp_dir().join(format!(
        "doubleneg-capture-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&tmp).expect("create capture file");

    // SAFETY: plain fd juggling; every descriptor is restored below.
    unsafe {
        use std::os::fd::AsRawFd;
        fflush(std::ptr::null_mut()); // flush anything already buffered
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let value = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        drop(file);

        let bytes = std::fs::read(&tmp).expect("read capture file");
        let _ = std::fs::remove_file(&tmp);
        (value, bytes)
    }
}

/// Pretty-print a byte blob for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Differentially run `doubleneg` in both libraries, comparing the return
/// value AND every byte it printed.
///
/// Must only be used from a test binary containing a SINGLE `#[test]`: the
/// capture redirects fd 1 process-wide and libtest writes its own progress
/// lines to fd 1 from other threads.
pub fn diff_doubleneg(p1: c_int, p2: c_int, p3: c_int, p4: c_int, label: &str) {
    const BANNER: &[u8] = b"=== Starting foo() execution ===\n";
    let p = apis();

    let (rc, out_c) = capture_stdout(|| unsafe { (p.c.doubleneg)(p1, p2, p3, p4) });
    let (rr, out_r) = capture_stdout(|| unsafe { (p.rust.doubleneg)(p1, p2, p3, p4) });

    // Guard against foreign bytes sneaking into the capture.
    for (which, out) in [("C", &out_c), ("Rust", &out_r)] {
        assert!(
            out.starts_with(BANNER),
            "{label}: {which} capture is contaminated (does not start with the banner): {:?}",
            show(&out[..out.len().min(80)])
        );
    }

    assert_eq!(
        rc, rr,
        "{label}: doubleneg({p1}, {p2}, {p3}, {p4}) return value C={rc} Rust={rr}"
    );

    if out_c != out_r {
        let c_lines: Vec<&[u8]> = out_c.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = out_r.split(|&b| b == b'\n').collect();
        let first = (0..c_lines.len().max(r_lines.len()))
            .find(|&i| c_lines.get(i) != r_lines.get(i))
            .unwrap_or(0);
        panic!(
            "{label}: doubleneg({p1}, {p2}, {p3}, {p4}) stdout differs at line {first}\n  \
             C   : {:?}\n  Rust: {:?}\n--- full C ---\n{}\n--- full Rust ---\n{}",
            c_lines.get(first).map(|l| show(l)),
            r_lines.get(first).map(|l| show(l)),
            show(&out_c),
            show(&out_r)
        );
    }
}
