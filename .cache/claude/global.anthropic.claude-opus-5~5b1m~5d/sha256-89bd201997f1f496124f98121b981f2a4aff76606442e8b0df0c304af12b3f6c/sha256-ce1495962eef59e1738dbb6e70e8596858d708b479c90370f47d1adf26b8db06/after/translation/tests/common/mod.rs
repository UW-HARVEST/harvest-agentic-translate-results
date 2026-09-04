// Shared differential-test harness: loads BOTH the C `.so` and the Rust `.so`
// through `libloading` and exposes them behind one identical interface, so every
// test can run the exact same call sequence against both implementations.
//
// The Rust side is ALWAYS reached through its `.so` exports (never by calling the
// crate directly), so the `#[no_mangle]` / `extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_double, c_int};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI types (mirrors of the C typedefs in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const RESULT_SIZE: usize = 24;
pub const RESULT_ARRAY_SIZE: usize = 248;
pub const COUNT_OFFSET: usize = 240;
pub const CAP: usize = 10;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CResult {
    pub value: c_int,
    pub scaled: c_double,
    pub rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CResultArray {
    pub data: [CResult; CAP],
    pub count: c_int,
}

pub type OperationFunc =
    Option<unsafe extern "C" fn(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int>;

// ---------------------------------------------------------------------------
// A byte-exact, over-allocated `ResultArray` buffer.
//
// Using raw bytes (rather than a typed struct) lets the tests
//   * poison padding and the unused tail and check it survives identically,
//   * compare the two implementations' output with a plain `memcmp`,
//   * hand the library a `count` larger than 10 while keeping every access the C
//     performs inside memory we actually own.
// ---------------------------------------------------------------------------

/// 248 bytes of `ResultArray` + a generous slack region the C is allowed to walk
/// into when a test deliberately sets `count > 10`.
pub const SLACK_ELEMS: usize = 64;
pub const BUF_SIZE: usize = RESULT_ARRAY_SIZE + SLACK_ELEMS * RESULT_SIZE;

#[repr(C, align(8))]
pub struct ArrBuf {
    pub bytes: [u8; BUF_SIZE],
}

impl Clone for ArrBuf {
    fn clone(&self) -> Self {
        ArrBuf { bytes: self.bytes }
    }
}

impl ArrBuf {
    pub fn zeroed() -> Self {
        ArrBuf {
            bytes: [0u8; BUF_SIZE],
        }
    }

    /// Fill every byte with a deterministic non-zero pattern so that *any*
    /// accidental extra write (or missing write) shows up in the byte compare.
    pub fn poisoned(rng: &mut Rng) -> Self {
        let mut b = [0u8; BUF_SIZE];
        for x in b.iter_mut() {
            *x = rng.next_u8();
        }
        ArrBuf { bytes: b }
    }

    pub fn as_ptr(&mut self) -> *mut CResultArray {
        self.bytes.as_mut_ptr() as *mut CResultArray
    }

    pub fn set_count(&mut self, count: c_int) {
        self.bytes[COUNT_OFFSET..COUNT_OFFSET + 4].copy_from_slice(&count.to_ne_bytes());
    }

    pub fn get_count(&self) -> c_int {
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.bytes[COUNT_OFFSET..COUNT_OFFSET + 4]);
        c_int::from_ne_bytes(b)
    }

    fn field(&self, idx: usize, off: usize, len: usize) -> &[u8] {
        let base = idx * RESULT_SIZE + off;
        &self.bytes[base..base + len]
    }

    fn field_mut(&mut self, idx: usize, off: usize, len: usize) -> &mut [u8] {
        let base = idx * RESULT_SIZE + off;
        &mut self.bytes[base..base + len]
    }

    pub fn value(&self, idx: usize) -> c_int {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.field(idx, 0, 4));
        c_int::from_ne_bytes(b)
    }

    pub fn scaled_bits(&self, idx: usize) -> u64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.field(idx, 8, 8));
        u64::from_ne_bytes(b)
    }

    pub fn rank(&self, idx: usize) -> c_int {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.field(idx, 16, 4));
        c_int::from_ne_bytes(b)
    }

    pub fn set_value(&mut self, idx: usize, v: c_int) {
        self.field_mut(idx, 0, 4).copy_from_slice(&v.to_ne_bytes());
    }

    pub fn set_scaled(&mut self, idx: usize, v: f64) {
        self.field_mut(idx, 8, 8)
            .copy_from_slice(&v.to_bits().to_ne_bytes());
    }

    pub fn set_rank(&mut self, idx: usize, v: c_int) {
        self.field_mut(idx, 16, 4).copy_from_slice(&v.to_ne_bytes());
    }
}

/// Byte-for-byte comparison with a readable diff on mismatch.
pub fn assert_bufs_eq(ctx: &str, c: &ArrBuf, r: &ArrBuf) {
    if c.bytes == r.bytes {
        return;
    }
    let mut diffs = Vec::new();
    for i in 0..BUF_SIZE {
        if c.bytes[i] != r.bytes[i] {
            let (elem, off) = (i / RESULT_SIZE, i % RESULT_SIZE);
            let where_ = if i >= COUNT_OFFSET && i < COUNT_OFFSET + 4 {
                "count".to_string()
            } else if i >= RESULT_ARRAY_SIZE {
                format!("slack[{}] off {}", (i - RESULT_ARRAY_SIZE) / RESULT_SIZE, off)
            } else {
                let f = match off {
                    0..=3 => "value",
                    4..=7 => "PADDING(4..8)",
                    8..=15 => "scaled",
                    16..=19 => "rank",
                    _ => "PADDING(20..24)",
                };
                format!("data[{}].{}", elem, f)
            };
            diffs.push(format!(
                "  byte {:>4} ({}): C=0x{:02x} Rust=0x{:02x}",
                i, where_, c.bytes[i], r.bytes[i]
            ));
            if diffs.len() >= 24 {
                diffs.push("  ...".to_string());
                break;
            }
        }
    }
    panic!(
        "ResultArray memory diverged [{}]:\n{}\n  C count={} Rust count={}",
        ctx,
        diffs.join("\n"),
        c.get_count(),
        r.get_count()
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible property testing.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2026_0903_C0FF_EE01;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn seeded() -> Self {
        Rng(SEED)
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// An `i32` biased towards interesting magnitudes (small, boundary, random).
    pub fn spicy_i32(&mut self) -> c_int {
        const EDGE: [c_int; 14] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            i32::MAX / 2,
            i32::MIN / 2,
            0x4000_0000,
        ];
        match self.below(4) {
            0 => EDGE[self.below(EDGE.len())],
            1 => (self.next_u32() % 65) as c_int - 32,
            2 => self.i32() >> (self.below(31) as u32),
            _ => self.i32(),
        }
    }
    /// An `f64` biased towards the shapes `safe_double_to_int` branches on.
    pub fn spicy_f64(&mut self) -> f64 {
        const EDGE: [f64; 22] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            0.999_999_999,
            -0.999_999_999,
            1.5,
            -1.5,
            2.5,
            -2.5,
            2147483647.0,
            2147483646.0,
            2147483646.5,
            2147483648.0,
            -2147483648.0,
            -2147483647.0,
            -2147483649.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        match self.below(5) {
            0 => EDGE[self.below(EDGE.len())],
            1 => f64::from_bits(self.next_u64()), // any bit pattern, incl. NaN payloads
            2 => (self.i32() as f64) + (self.next_u32() as f64) / 4294967296.0,
            3 => (self.i32() as f64) * (self.i32() as f64),
            _ => {
                // tight band around the +-2^31 decision boundaries
                let d = (self.next_u32() % 9) as f64 - 4.0;
                let s = if self.below(2) == 0 { 1.0 } else { -1.0 };
                s * 2147483648.0 + d
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    // Prefer the profile the test itself was built with, then fall back.
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for prof in order {
        let p = md.join("target").join(prof).join("libarrayfunc_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust cdylib found; run `cargo build --release` first");
}

/// One loaded implementation (either the C one or the Rust one).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,

    pub add_operation: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub multiply_operation: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub subtract_operation: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub modulo_operation: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub safe_double_to_int: unsafe extern "C" fn(c_double) -> c_int,
    pub compute_scaled_value: unsafe extern "C" fn(c_int, c_double) -> c_int,
    pub compare_results_in_array: unsafe extern "C" fn(*mut CResultArray, c_int, c_int) -> c_int,
    pub init_result_array: unsafe extern "C" fn(*mut CResultArray, *mut c_int, c_int),
    pub process_with_foreach: unsafe extern "C" fn(*mut CResultArray, OperationFunc) -> c_int,
    pub compute_weighted_sum: unsafe extern "C" fn(*mut CResultArray) -> c_int,
    pub arrayfunc: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {{
        let s: Symbol<$t> = unsafe { $lib.get(concat!($n, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol `{}`: {}", $n, e));
        unsafe { *s.into_raw() }
    }};
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {}", path.display(), e));
        type Op4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let me = Impl {
            name,
            add_operation: sym!(lib, Op4, "add_operation"),
            multiply_operation: sym!(lib, Op4, "multiply_operation"),
            subtract_operation: sym!(lib, Op4, "subtract_operation"),
            modulo_operation: sym!(lib, Op4, "modulo_operation"),
            safe_double_to_int: sym!(lib, unsafe extern "C" fn(c_double) -> c_int, "safe_double_to_int"),
            compute_scaled_value: sym!(lib, unsafe extern "C" fn(c_int, c_double) -> c_int, "compute_scaled_value"),
            compare_results_in_array: sym!(lib, unsafe extern "C" fn(*mut CResultArray, c_int, c_int) -> c_int, "compare_results_in_array"),
            init_result_array: sym!(lib, unsafe extern "C" fn(*mut CResultArray, *mut c_int, c_int), "init_result_array"),
            process_with_foreach: sym!(lib, unsafe extern "C" fn(*mut CResultArray, OperationFunc) -> c_int, "process_with_foreach"),
            compute_weighted_sum: sym!(lib, unsafe extern "C" fn(*mut CResultArray) -> c_int, "compute_weighted_sum"),
            arrayfunc: sym!(lib, Op4, "arrayfunc"),
            _lib: lib,
        };
        me
    }

    /// The four operations this implementation exports, in `arrayfunc` order.
    pub fn ops(&self) -> [unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int; 4] {
        [
            self.add_operation,
            self.multiply_operation,
            self.subtract_operation,
            self.modulo_operation,
        ]
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

/// Loads both `.so`s. Kept in a process-wide `OnceLock` so the libraries are
/// `dlopen`ed exactly once per test binary.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Impl::load("C", &c_so_path()),
        rs: Impl::load("Rust", &rust_so_path()),
    })
}

pub const OP_NAMES: [&str; 4] = ["add", "multiply", "subtract", "modulo"];

/// `INT32_MIN % -1` overflows the x86-64 `idiv` instruction: the C
/// `modulo_operation` raises **SIGFPE** and the process dies (verified
/// experimentally — see ERRORS.md row 2). There is no return value to compare, so
/// this single input pair is filtered out of every generator. Nothing else in the
/// library can trap: the only other division is `param4 / 2` in `arrayfunc`, whose
/// divisor is the constant 2.
pub fn is_idiv_trap(a: c_int, b: c_int) -> bool {
    a == c_int::MIN && b == -1
}
