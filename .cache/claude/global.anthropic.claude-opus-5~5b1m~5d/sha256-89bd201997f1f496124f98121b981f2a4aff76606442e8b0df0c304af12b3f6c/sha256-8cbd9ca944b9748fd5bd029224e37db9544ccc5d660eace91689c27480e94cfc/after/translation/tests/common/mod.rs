//! Shared differential-test harness.
//!
//! BOTH the C `.so` and the Rust `.so` are loaded with `libloading`; no Rust
//! function is ever called directly, so the `#[no_mangle] extern "C"` export
//! wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C types mirrored for the tests (independent re-declaration on purpose: if the
// crate's own layout were wrong, these would catch it).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Result_ {
    pub value: i32,
    pub scaled: f64,
    pub rank: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ResultArray {
    pub data: [Result_; 10],
    pub count: i32,
}

impl ResultArray {
    /// Non-zero "poison" state so that slots the library never writes are still
    /// compared meaningfully.
    pub fn poisoned() -> Self {
        let mut a = ResultArray {
            data: [Result_ {
                value: 0,
                scaled: 0.0,
                rank: 0,
            }; 10],
            count: -12345,
        };
        for (i, s) in a.data.iter_mut().enumerate() {
            s.value = 0x5A5A_0000u32 as i32 + i as i32;
            s.scaled = -1.75 * (i as f64 + 1.0);
            s.rank = -100 - i as i32;
        }
        a
    }

    pub fn zeroed() -> Self {
        ResultArray {
            data: [Result_ {
                value: 0,
                scaled: 0.0,
                rank: 0,
            }; 10],
            count: 0,
        }
    }

    /// Field-wise, bit-exact fingerprint (avoids comparing struct *padding*,
    /// which C does not promise to copy, while still comparing every declared
    /// byte of every declared member — including the `double` bit patterns).
    pub fn fingerprint(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(4 + 10 * 16);
        v.extend_from_slice(&self.count.to_le_bytes());
        for s in self.data.iter() {
            v.extend_from_slice(&s.value.to_le_bytes());
            v.extend_from_slice(&s.scaled.to_bits().to_le_bytes());
            v.extend_from_slice(&s.rank.to_le_bytes());
        }
        v
    }
}

pub type OperationFunc =
    Option<unsafe extern "C" fn(a: i32, b: i32, unused1: i32, unused2: i32) -> i32>;

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation/Cargo.toml -> <root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}.\n\
             Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile the tests themselves were built with, then release.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libarrayfunc_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it first: cargo build --release",
        target.display()
    )
}

// ---------------------------------------------------------------------------
// Typed façade over one loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    f_add: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,
    f_mul: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,
    f_sub: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,
    f_mod: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,
    f_sdti: unsafe extern "C" fn(f64) -> i32,
    f_csv: unsafe extern "C" fn(i32, f64) -> i32,
    f_cmp: unsafe extern "C" fn(*mut ResultArray, i32, i32) -> i32,
    f_init: unsafe extern "C" fn(*mut ResultArray, *mut i32, i32),
    f_foreach: unsafe extern "C" fn(*mut ResultArray, OperationFunc) -> i32,
    f_weighted: unsafe extern "C" fn(*mut ResultArray) -> i32,
    f_arrayfunc: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Lib {
    pub fn load(path: &Path, name: &'static str) -> Lib {
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            Lib {
                name,
                f_add: sym(&lib, b"add_operation\0"),
                f_mul: sym(&lib, b"multiply_operation\0"),
                f_sub: sym(&lib, b"subtract_operation\0"),
                f_mod: sym(&lib, b"modulo_operation\0"),
                f_sdti: sym(&lib, b"safe_double_to_int\0"),
                f_csv: sym(&lib, b"compute_scaled_value\0"),
                f_cmp: sym(&lib, b"compare_results_in_array\0"),
                f_init: sym(&lib, b"init_result_array\0"),
                f_foreach: sym(&lib, b"process_with_foreach\0"),
                f_weighted: sym(&lib, b"compute_weighted_sum\0"),
                f_arrayfunc: sym(&lib, b"arrayfunc\0"),
                _lib: lib,
            }
        }
    }

    pub fn add(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.f_add)(a, b, u1, u2) }
    }
    pub fn mul(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.f_mul)(a, b, u1, u2) }
    }
    pub fn sub(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.f_sub)(a, b, u1, u2) }
    }
    pub fn modulo(&self, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        unsafe { (self.f_mod)(a, b, u1, u2) }
    }
    pub fn sdti(&self, d: f64) -> i32 {
        unsafe { (self.f_sdti)(d) }
    }
    pub fn compute_scaled_value(&self, base: i32, scale: f64) -> i32 {
        unsafe { (self.f_csv)(base, scale) }
    }
    pub fn compare(&self, arr: &mut ResultArray, i1: i32, i2: i32) -> i32 {
        unsafe { (self.f_cmp)(arr as *mut ResultArray, i1, i2) }
    }
    pub fn init(&self, arr: &mut ResultArray, values: &mut [i32], count: i32) {
        unsafe { (self.f_init)(arr as *mut ResultArray, values.as_mut_ptr(), count) }
    }
    pub fn foreach(&self, arr: &mut ResultArray, op: OperationFunc) -> i32 {
        unsafe { (self.f_foreach)(arr as *mut ResultArray, op) }
    }
    pub fn weighted(&self, arr: &mut ResultArray) -> i32 {
        unsafe { (self.f_weighted)(arr as *mut ResultArray) }
    }
    pub fn arrayfunc(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.f_arrayfunc)(a, b, c, d) }
    }

    /// The library's *own* `add/mul/sub/mod` as a raw `operation_func`, so that
    /// `process_with_foreach` in the C `.so` calls the C op and the one in the
    /// Rust `.so` calls the Rust op (mirroring how `arrayfunc` wires them up).
    pub fn own_op(&self, which: usize) -> OperationFunc {
        Some(match which {
            0 => self.f_add,
            1 => self.f_mul,
            2 => self.f_sub,
            3 => self.f_mod,
            _ => unreachable!(),
        })
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Lib::load(&find_c_so(), "C"),
        rs: Lib::load(&find_rust_so(), "Rust"),
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — reproducible property-style inputs
// ---------------------------------------------------------------------------

pub const DEFAULT_SEED: u64 = 0x5EED_1234_ABCD_EF01;

/// Fixed seed for reproducibility; overridable via `DIFF_SEED=<u64>` so the
/// whole suite can be re-run over fresh random inputs without editing code.
#[allow(non_snake_case)]
pub fn SEED_value() -> u64 {
    match std::env::var("DIFF_SEED") {
        Ok(s) => s.parse().unwrap_or(DEFAULT_SEED),
        Err(_) => DEFAULT_SEED,
    }
}

#[allow(non_upper_case_globals)]
pub struct SeedConst;
impl std::ops::BitXor<u64> for SeedConst {
    type Output = u64;
    fn bitxor(self, rhs: u64) -> u64 {
        SEED_value() ^ rhs
    }
}
#[allow(non_upper_case_globals)]
pub const SEED: SeedConst = SeedConst;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Small-magnitude int, biased toward interesting little values.
    pub fn small_i32(&mut self) -> i32 {
        (self.below(41) as i32) - 20
    }
    /// A mix of tiny, medium and extreme ints.
    pub fn mixed_i32(&mut self) -> i32 {
        const EXTREMES: [i32; 10] = [
            i32::MIN,
            i32::MIN + 1,
            -65536,
            -3,
            -1,
            0,
            1,
            3,
            i32::MAX - 1,
            i32::MAX,
        ];
        match self.below(4) {
            0 => self.small_i32(),
            1 => EXTREMES[self.below(10) as usize],
            2 => self.next_i32() >> (self.below(24) as u32),
            _ => self.next_i32(),
        }
    }
    /// Arbitrary f64 (any bit pattern: NaN / INF / subnormal all reachable).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// f64 drawn from a mix of well-behaved and pathological classes.
    pub fn mixed_f64(&mut self) -> f64 {
        match self.below(8) {
            0 => f64::NAN,
            1 => -f64::NAN,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => 0.0,
            5 => -0.0,
            6 => self.any_f64(),
            _ => {
                let m = (self.next_u64() % 4_000_000_007) as f64 / 1_000_000.0;
                if self.below(2) == 0 {
                    -m
                } else {
                    m
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(ctx: &str, c: i32, rs: i32) {
    assert_eq!(c, rs, "return value mismatch [{ctx}]: C={c} Rust={rs}");
}

#[track_caller]
pub fn eq_state(ctx: &str, c: &ResultArray, rs: &ResultArray) {
    if c.fingerprint() == rs.fingerprint() {
        return;
    }
    let mut msg = format!("ResultArray state mismatch [{ctx}]\n");
    if c.count != rs.count {
        msg += &format!("  count: C={} Rust={}\n", c.count, rs.count);
    }
    for i in 0..10 {
        let (a, b) = (&c.data[i], &rs.data[i]);
        if a.value != b.value || a.scaled.to_bits() != b.scaled.to_bits() || a.rank != b.rank {
            msg += &format!(
                "  data[{i}]: C={{value:{} scaled:{:?}/{:#018x} rank:{}}} \
                 Rust={{value:{} scaled:{:?}/{:#018x} rank:{}}}\n",
                a.value,
                a.scaled,
                a.scaled.to_bits(),
                a.rank,
                b.value,
                b.scaled,
                b.scaled.to_bits(),
                b.rank
            );
        }
    }
    panic!("{msg}");
}
