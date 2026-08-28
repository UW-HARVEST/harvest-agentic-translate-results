// Shared differential-test harness.
//
// Loads BOTH the C `.so` and the Rust `.so` through `libloading` and exposes their
// exported symbols behind an identical `Api` facade. Rust functions are NEVER called
// directly — every call goes through `dlsym` on the cdylib, so the `#[no_mangle]`
// export wrappers are under test too.
//
// The library under test keeps THREE mutable `static`s (`accumulator`, `multiplier`,
// `operation_count`) that persist across calls and that `findrep` branches on. Because
// `dlopen` de-duplicates by (dev, ino), loading the same path twice would share that
// state between tests. So every `Api` is loaded from a *fresh private copy* of the
// `.so`, giving each test pristine statics (`0`, `1`, `0`) on both sides and keeping
// the two libraries in lockstep.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub type Fn1 = unsafe extern "C" fn(c_int) -> c_int;
pub type Fn2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Fn4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnStr = unsafe extern "C" fn(*mut c_char, c_int);

/// The 8 exported symbols of the library, resolved from one `.so` instance.
pub struct Api {
    /// Kept alive so the resolved function pointers stay valid.
    _lib: Library,
    tmp_path: PathBuf,
    pub which: &'static str,
    pub add_to_accumulator: Fn2,
    pub multiply_with_multiplier: Fn2,
    pub subtract_from_accumulator: Fn2,
    pub divide_multiplier: Fn2,
    pub process_octal_string: FnStr,
    pub find_and_replace_char: FnStr,
    pub validate_and_normalize: Fn1,
    pub findrep: Fn4,
}

impl Drop for Api {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.tmp_path);
    }
}

impl Api {
    /// Dispatch by index, mirroring the C `operations[4]` table order.
    pub unsafe fn op(&self, idx: usize, a: c_int, b: c_int) -> c_int {
        unsafe {
            match idx {
                0 => (self.add_to_accumulator)(a, b),
                1 => (self.multiply_with_multiplier)(a, b),
                2 => (self.subtract_from_accumulator)(a, b),
                3 => (self.divide_multiplier)(a, b),
                _ => unreachable!(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<projectname>.so` — the CMake project name is derived from the
/// parent directory name, so scan for the single `.so` in the build tree.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}. Build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// `target/{release,debug}/libfindrep_lib.so`
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().join("target");
    let rel = root.join("release/libfindrep_lib.so");
    let dbg = root.join("debug/libfindrep_lib.so");
    if rel.exists() {
        rel
    } else {
        assert!(
            dbg.exists(),
            "no libfindrep_lib.so under {}; run `cargo build --release`",
            root.display()
        );
        dbg
    }
}

// ---------------------------------------------------------------------------
// Fresh-state loading
// ---------------------------------------------------------------------------

fn tmp_dir() -> PathBuf {
    std::env::var("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// Copy `src` to a brand-new file so `dlopen` treats it as a distinct object and
/// gives us a private copy of the mutable statics.
fn fresh_copy(src: &Path, tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = tmp_dir().join("difftest_so");
    std::fs::create_dir_all(&dir).expect("create temp dir for .so copies");
    let dst = dir.join(format!("{}_{}_{}.so", tag, std::process::id(), n));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    dst
}

fn load(src: &Path, which: &'static str) -> Api {
    let tmp_path = fresh_copy(src, which);
    unsafe {
        let lib = Library::new(&tmp_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", tmp_path.display()));

        macro_rules! sym {
            ($t:ty, $name:literal) => {{
                let s = lib.get::<$t>(concat!($name, "\0").as_bytes()).unwrap_or_else(|e| {
                    panic!("{} .so is missing symbol `{}`: {e}", which, $name)
                });
                *s
            }};
        }

        let api = Api {
            which,
            add_to_accumulator: sym!(Fn2, "add_to_accumulator"),
            multiply_with_multiplier: sym!(Fn2, "multiply_with_multiplier"),
            subtract_from_accumulator: sym!(Fn2, "subtract_from_accumulator"),
            divide_multiplier: sym!(Fn2, "divide_multiplier"),
            process_octal_string: sym!(FnStr, "process_octal_string"),
            find_and_replace_char: sym!(FnStr, "find_and_replace_char"),
            validate_and_normalize: sym!(Fn1, "validate_and_normalize"),
            findrep: sym!(Fn4, "findrep"),
            _lib: lib,
            tmp_path,
        };
        api
    }
}

/// A matched pair of libraries, both with pristine statics.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

/// Load a fresh C + Rust pair. Call this at the start of every test so the hidden
/// static state starts at (`accumulator=0`, `multiplier=1`, `operation_count=0`).
pub fn fresh_pair() -> Pair {
    Pair {
        c: load(&c_so_path(), "c"),
        r: load(&rust_so_path(), "rust"),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds keep failures reproducible.
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
    /// Uniform over the whole `i32` range (all bit patterns).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    /// A value biased toward the interesting classes of
    /// `validate_and_normalize`: negatives, 0, the 1..63 clamp-up band, the
    /// 64..511 identity band, the >511 clamp-down band, and extremes.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(10) {
            0 => 0,
            1 => self.range_i32(1, 63),
            2 => self.range_i32(64, 511),
            3 => self.range_i32(512, i32::MAX),
            4 => self.range_i32(i32::MIN, -1),
            5 => *self.pick(&BOUNDARIES),
            6 => self.range_i32(-64, 64),
            7 => self.range_i32(500, 520),
            _ => self.next_i32(),
        }
    }
}

/// Boundary values around every constant the C compares against:
/// `0100`=64, `0777`=511, `0150`=104, `010`=8, plus type extremes.
pub const BOUNDARIES: [i32; 27] = [
    i32::MIN,
    i32::MIN + 1,
    -512,
    -511,
    -105,
    -104,
    -65,
    -64,
    -63,
    -8,
    -2,
    -1,
    0,
    1,
    2,
    7,
    8,
    9,
    63,
    64,
    65,
    103,
    104,
    105,
    510,
    511,
    512,
];

// ---------------------------------------------------------------------------
// Buffer helpers
// ---------------------------------------------------------------------------

/// Matches the `char message[100]` / `char search_buffer[100]` size in the C.
pub const BUF: usize = 100;

/// Fill sentinel — anything the library does not write must stay `0xAA`, which
/// detects a terminator written at the wrong offset or bytes clobbered past it.
pub const SENTINEL: u8 = 0xAA;

pub fn sentinel_buf() -> Vec<u8> {
    vec![SENTINEL; BUF]
}

/// A NUL-terminated buffer holding `s`, padded to `BUF` with the sentinel.
pub fn cstr_buf(s: &[u8]) -> Vec<u8> {
    assert!(s.len() + 1 <= BUF);
    let mut v = vec![SENTINEL; BUF];
    v[..s.len()].copy_from_slice(s);
    v[s.len()] = 0;
    v
}

pub fn show(b: &[u8]) -> String {
    let mut out = String::new();
    for &x in b {
        match x {
            0 => out.push_str("\\0"),
            0x20..=0x7e => out.push(x as char),
            _ => out.push_str(&format!("\\x{x:02x}")),
        }
    }
    out
}

/// Assert two raw buffers are byte-identical, printing an escaped diff on failure.
#[track_caller]
pub fn assert_bytes_eq(cb: &[u8], rb: &[u8], ctx: &str) {
    if cb != rb {
        let at = cb
            .iter()
            .zip(rb.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(cb.len().min(rb.len()));
        panic!(
            "buffer mismatch ({ctx})\n  first differing byte index: {at}\n  C   : {}\n  Rust: {}",
            show(cb),
            show(rb)
        );
    }
}
