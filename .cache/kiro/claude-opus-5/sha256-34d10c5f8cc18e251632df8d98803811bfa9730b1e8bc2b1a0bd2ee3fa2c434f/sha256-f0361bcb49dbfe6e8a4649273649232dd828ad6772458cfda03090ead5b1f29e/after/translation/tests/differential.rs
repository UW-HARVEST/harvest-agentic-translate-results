// Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
// `libloading` and compares their behaviour across the FFI boundary. Rust
// functions are never called directly — only through the cdylib's exported
// symbols, so the `#[no_mangle]` wrappers are under test too.

#![allow(clippy::too_many_arguments)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types mirroring the C ABI
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Result_ {
    pub value: i32,
    pub scaled: f64,
    pub rank: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResultArray {
    pub data: [Result_; 10],
    pub count: i32,
}

impl ResultArray {
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

    /// Field-by-field snapshot (padding bytes are deliberately excluded: C
    /// struct assignment may or may not write them, which is not observable
    /// behaviour). `scaled` is captured by raw bits so `-0.0` and NaN payloads
    /// are compared exactly.
    pub fn snapshot(&self) -> Vec<(i32, u64, i32)> {
        let mut v = Vec::with_capacity(11);
        v.push((self.count, 0, 0));
        for r in self.data.iter() {
            v.push((r.value, r.scaled.to_bits(), r.rank));
        }
        v
    }
}

pub type BinOp = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
type FnSafeDti = unsafe extern "C" fn(f64) -> i32;
type FnCsv = unsafe extern "C" fn(i32, f64) -> i32;
type FnCmp = unsafe extern "C" fn(*mut ResultArray, i32, i32) -> i32;
type FnInit = unsafe extern "C" fn(*mut ResultArray, *mut i32, i32);
type FnForeach = unsafe extern "C" fn(*mut ResultArray, BinOp) -> i32;
type FnWsum = unsafe extern "C" fn(*mut ResultArray) -> i32;
type FnArrayfunc = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

pub const OP_NAMES: [&str; 4] = [
    "add_operation",
    "multiply_operation",
    "subtract_operation",
    "modulo_operation",
];

pub const ALL_SYMBOLS: [&str; 11] = [
    "add_operation",
    "multiply_operation",
    "subtract_operation",
    "modulo_operation",
    "safe_double_to_int",
    "compute_scaled_value",
    "compare_results_in_array",
    "init_result_array",
    "process_with_foreach",
    "compute_weighted_sum",
    "arrayfunc",
];

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libarrayfunc_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libarrayfunc_lib.so not found under {}; run `cargo build --release`",
        root.display()
    );
}

/// Both libraries, leaked so that resolved `Symbol`s are `'static`.
pub fn libs() -> (&'static Library, &'static Library) {
    static LIBS: OnceLock<(&'static Library, &'static Library)> = OnceLock::new();
    *LIBS.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", cp.display()));
        let r = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rp.display()));
        (
            Box::leak(Box::new(c)) as &'static Library,
            Box::leak(Box::new(r)) as &'static Library,
        )
    })
}

fn get<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe { lib.get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not exported: {e}"))
}

/// Resolves `name` in both libraries, returning `(c_fn, rust_fn)`.
fn pair<T: Copy + 'static>(name: &str) -> (T, T) {
    let (c, r) = libs();
    (*get::<T>(c, name), *get::<T>(r, name))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Biased toward interesting magnitudes: tiny, medium, and huge values.
    pub fn next_i32_mixed(&mut self) -> i32 {
        let r = self.next_u64();
        let v = (r >> 32) as u32 as i32;
        match r & 7 {
            0 => (v % 11) - 5,
            1 => (v % 201) - 100,
            2 => v >> 16,
            3 => i32::MAX.wrapping_add(v % 5).wrapping_sub(2),
            4 => i32::MIN.wrapping_add(v % 5).wrapping_sub(2),
            _ => v,
        }
    }
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

pub const BOUNDARY_I32: [i32; 11] = [
    0,
    1,
    -1,
    2,
    -2,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    1 << 30,
    -(1 << 30),
];

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
fn eq_i32(ctx: &str, c: i32, r: i32) {
    assert_eq!(c, r, "divergence [{ctx}]: C returned {c}, Rust returned {r}");
}

#[track_caller]
fn eq_state(ctx: &str, c: &ResultArray, r: &ResultArray) {
    let cs = c.snapshot();
    let rs = r.snapshot();
    if cs != rs {
        let mut msg = format!("array-state divergence [{ctx}]\n  slot: C -> Rust\n");
        msg += &format!("  count: {} -> {}\n", cs[0].0, rs[0].0);
        for i in 0..10 {
            if cs[i + 1] != rs[i + 1] {
                msg += &format!(
                    "  data[{i}]: value {} -> {}, scaled {:?}({:#018x}) -> {:?}({:#018x}), rank {} -> {}\n",
                    cs[i + 1].0,
                    rs[i + 1].0,
                    f64::from_bits(cs[i + 1].1),
                    cs[i + 1].1,
                    f64::from_bits(rs[i + 1].1),
                    rs[i + 1].1,
                    cs[i + 1].2,
                    rs[i + 1].2
                );
            }
        }
        panic!("{msg}");
    }
}

/// Runs `init_result_array` on both sides with the same values, returning the
/// two independently-initialised arrays (already asserted equal).
fn init_both(values: &[i32], count: i32) -> (ResultArray, ResultArray) {
    let (c_init, r_init) = pair::<FnInit>("init_result_array");
    let mut ca = ResultArray::zeroed();
    let mut ra = ResultArray::zeroed();
    let mut cv = values.to_vec();
    let mut rv = values.to_vec();
    unsafe {
        c_init(&mut ca, cv.as_mut_ptr(), count);
        r_init(&mut ra, rv.as_mut_ptr(), count);
    }
    assert_eq!(cv, rv, "init_result_array must not modify its input array");
    eq_state(&format!("init_both(count={count})"), &ca, &ra);
    (ca, ra)
}

// ===========================================================================
// Symbol parity (Phase A / D, checked from inside the harness as well)
// ===========================================================================

#[test]
fn symbols_all_exported_by_both() {
    let (c, r) = libs();
    for name in ALL_SYMBOLS {
        assert!(
            unsafe { c.get::<*const ()>(name.as_bytes()) }.is_ok(),
            "C .so is missing `{name}`"
        );
        assert!(
            unsafe { r.get::<*const ()>(name.as_bytes()) }.is_ok(),
            "Rust .so is missing `{name}`"
        );
    }
}

// ===========================================================================
// CONFIGS rows 1-10 — the four operation_func leaves
// ===========================================================================

fn bin_random(name: &str, seed: u64, iters: usize, nonzero_b: bool) {
    let (cf, rf) = pair::<BinOp>(name);
    let mut rng = Rng::new(seed);
    for i in 0..iters {
        let a = rng.next_i32_mixed();
        let mut b = rng.next_i32_mixed();
        if nonzero_b {
            if b == 0 {
                b = 1;
            }
            // ERRORS #2: INT_MIN % -1 traps; covered by its own subprocess test.
            if a == i32::MIN && b == -1 {
                b = -3;
            }
        }
        let u1 = rng.next_i32();
        let u2 = rng.next_i32();
        let c = unsafe { cf(a, b, u1, u2) };
        let rv = unsafe { rf(a, b, u1, u2) };
        eq_i32(&format!("{name} iter={i} a={a} b={b} u1={u1} u2={u2}"), c, rv);
    }
}

fn bin_boundaries(name: &str, skip_trap: bool) {
    let (cf, rf) = pair::<BinOp>(name);
    for &a in BOUNDARY_I32.iter() {
        for &b in BOUNDARY_I32.iter() {
            if skip_trap && a == i32::MIN && b == -1 {
                continue;
            }
            let c = unsafe { cf(a, b, 0, 0) };
            let rv = unsafe { rf(a, b, 0, 0) };
            eq_i32(&format!("{name} a={a} b={b}"), c, rv);
        }
    }
}

#[test]
fn cfg_add_random() {
    bin_random("add_operation", SEED ^ 1, 4096, false);
}

#[test]
fn cfg_add_boundaries() {
    bin_boundaries("add_operation", false);
}

#[test]
fn cfg_mul_random() {
    bin_random("multiply_operation", SEED ^ 2, 4096, false);
}

#[test]
fn cfg_mul_boundaries() {
    bin_boundaries("multiply_operation", false);
}

#[test]
fn cfg_sub_random() {
    bin_random("subtract_operation", SEED ^ 3, 4096, false);
}

#[test]
fn cfg_sub_boundaries() {
    bin_boundaries("subtract_operation", false);
}

#[test]
fn cfg_mod_random() {
    bin_random("modulo_operation", SEED ^ 4, 4096, true);
}

#[test]
fn cfg_mod_rank_divisors() {
    // `arrayfunc` only ever passes `rank` (0..=9) as the divisor.
    let (cf, rf) = pair::<BinOp>("modulo_operation");
    let mut rng = Rng::new(SEED ^ 5);
    for b in 0..=9i32 {
        for _ in 0..512 {
            let a = rng.next_i32_mixed();
            let c = unsafe { cf(a, b, 0, 0) };
            let rv = unsafe { rf(a, b, 0, 0) };
            eq_i32(&format!("modulo_operation rank divisor a={a} b={b}"), c, rv);
        }
    }
}

#[test]
fn cfg_mod_boundaries() {
    bin_boundaries("modulo_operation", true);
    let (cf, rf) = pair::<BinOp>("modulo_operation");
    for &a in BOUNDARY_I32.iter() {
        for b in [1i32, -1, 2, -2, 3, -3, i32::MAX, i32::MIN] {
            if a == i32::MIN && b == -1 {
                continue;
            }
            let c = unsafe { cf(a, b, 0, 0) };
            let rv = unsafe { rf(a, b, 0, 0) };
            eq_i32(&format!("modulo_operation a={a} b={b}"), c, rv);
        }
    }
}

#[test]
fn cfg_ops_unused_args_ignored() {
    let mut rng = Rng::new(SEED ^ 6);
    for name in OP_NAMES {
        let (cf, rf) = pair::<BinOp>(name);
        for _ in 0..512 {
            let a = rng.next_i32_mixed();
            let mut b = rng.next_i32_mixed();
            if name == "modulo_operation" && a == i32::MIN && b == -1 {
                b = 7;
            }
            let u1 = rng.next_i32();
            let u2 = rng.next_i32();
            let c0 = unsafe { cf(a, b, 0, 0) };
            let cg = unsafe { cf(a, b, u1, u2) };
            let r0 = unsafe { rf(a, b, 0, 0) };
            let rg = unsafe { rf(a, b, u1, u2) };
            eq_i32(&format!("{name} unused=0 a={a} b={b}"), c0, r0);
            eq_i32(&format!("{name} unused=garbage a={a} b={b}"), cg, rg);
            eq_i32(&format!("{name} C must ignore unused a={a} b={b}"), c0, cg);
        }
    }
}

// ===========================================================================
// CONFIGS rows 11-19 — numeric helpers
// ===========================================================================

fn sdti_check(ctx: &str, d: f64) {
    let (cf, rf) = pair::<FnSafeDti>("safe_double_to_int");
    let c = unsafe { cf(d) };
    let r = unsafe { rf(d) };
    eq_i32(
        &format!("safe_double_to_int {ctx} d={d:?} bits={:#018x}", d.to_bits()),
        c,
        r,
    );
}

#[test]
fn cfg_sdti_fractional() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..4096 {
        // uniform in (-2^31, 2^31) with a fractional part
        let m = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sdti_check("fractional", sign * m * 2147483646.0);
    }
    for d in [
        0.5, -0.5, 0.9999999, -0.9999999, 1.5, -1.5, 2.5, -2.5, -0.0000001, 1e-300, -1e-300,
        123456.789, -123456.789, 2147483646.5, -2147483647.5,
    ] {
        sdti_check("fractional literal", d);
    }
}

#[test]
fn cfg_sdti_exact_integers() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..4096 {
        let v = rng.next_i32();
        sdti_check("exact integer", v as f64);
    }
    for &v in BOUNDARY_I32.iter() {
        sdti_check("exact boundary integer", v as f64);
    }
}

/// Minimal `nextafter` over the raw bit pattern (no libm dependency).
fn next_after(x: f64, toward: f64) -> f64 {
    if x.is_nan() || toward.is_nan() || x == toward {
        return x;
    }
    if x == 0.0 {
        return if toward > 0.0 {
            f64::from_bits(1)
        } else {
            -f64::from_bits(1)
        };
    }
    let bits = x.to_bits();
    let up = (x < toward) == (x > 0.0);
    f64::from_bits(if up { bits + 1 } else { bits - 1 })
}

#[test]
fn cfg_sdti_boundary_neighbourhood() {
    let mut cases: Vec<f64> = Vec::new();
    for base in [i32::MAX as f64, i32::MIN as f64, 0.0f64] {
        for delta in [-3.0, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0] {
            cases.push(base + delta);
        }
        let mut up = base;
        let mut down = base;
        for _ in 0..4 {
            up = next_after(up, f64::INFINITY);
            down = next_after(down, f64::NEG_INFINITY);
            cases.push(up);
            cases.push(down);
        }
    }
    for d in cases {
        sdti_check("boundary neighbourhood", d);
    }
}

#[test]
fn cfg_sdti_specials() {
    let specials: [f64; 20] = [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1), // smallest subnormal
        -f64::from_bits(1),
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0xFFF8_DEAD_BEEF_1234), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        1e300,
        -1e300,
        2147483647.0,
        -2147483648.0,
    ];
    for d in specials {
        sdti_check("special", d);
    }
}

#[test]
fn cfg_sdti_random_bitpatterns() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..8192 {
        sdti_check("random bits", rng.next_f64_bits());
    }
}

fn csv_check(ctx: &str, base: i32, scale: f64) {
    let (cf, rf) = pair::<FnCsv>("compute_scaled_value");
    let c = unsafe { cf(base, scale) };
    let r = unsafe { rf(base, scale) };
    eq_i32(
        &format!(
            "compute_scaled_value {ctx} base={base} scale={scale:?} bits={:#018x}",
            scale.to_bits()
        ),
        c,
        r,
    );
}

#[test]
fn cfg_csv_fractional_scale() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..4096 {
        let base = rng.next_i32_mixed();
        let m = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        csv_check("fractional scale", base, sign * m);
    }
}

#[test]
fn cfg_csv_library_constants() {
    let mut rng = Rng::new(SEED ^ 17);
    let consts = [1.5f64, -1.5, 0.75, -0.75, 0.8, -0.8, 0.333, -0.333];
    for &s in consts.iter() {
        for _ in 0..512 {
            csv_check("library constant", rng.next_i32_mixed(), s);
        }
        for &b in BOUNDARY_I32.iter() {
            csv_check("library constant boundary", b, s);
        }
    }
}

#[test]
fn cfg_csv_random_scale() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..8192 {
        let base = rng.next_i32_mixed();
        csv_check("random scale bits", base, rng.next_f64_bits());
    }
}

#[test]
fn cfg_csv_boundaries() {
    let scales = [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MAX,
        f64::MIN_POSITIVE,
    ];
    for &b in BOUNDARY_I32.iter() {
        for &s in scales.iter() {
            csv_check("boundary", b, s);
        }
    }
}

// ===========================================================================
// CONFIGS row 20 — compare_results_in_array, valid indices
// ===========================================================================

#[test]
fn cfg_compare_all_inrange_pairs() {
    let (cf, rf) = pair::<FnCmp>("compare_results_in_array");
    let mut rng = Rng::new(SEED ^ 20);
    for count in 1..=10i32 {
        let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let (mut ca, mut ra) = init_both(&values, count);
        for i1 in 0..count {
            for i2 in 0..count {
                let c = unsafe { cf(&mut ca, i1, i2) };
                let r = unsafe { rf(&mut ra, i1, i2) };
                eq_i32(&format!("compare count={count} idx1={i1} idx2={i2}"), c, r);
            }
        }
        eq_state(&format!("compare must not mutate (count={count})"), &ca, &ra);
    }
}

// ===========================================================================
// CONFIGS rows 21-24 — init_result_array
// ===========================================================================

#[test]
fn cfg_init_counts_random() {
    let mut rng = Rng::new(SEED ^ 21);
    for count in [0, 1, 2, 5, 8, 9, 10i32] {
        for _ in 0..256 {
            let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            let (ca, ra) = init_both(&values, count);
            assert_eq!(ca.count, count.min(10), "C clamp for count={count}");
            eq_state(&format!("init count={count}"), &ca, &ra);
        }
    }
}

#[test]
fn cfg_init_extreme_values() {
    let extremes = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0,
        -1,
        1,
        1 << 30,
        -(1 << 30),
        1431655765,
    ];
    let (ca, ra) = init_both(&extremes, 8);
    eq_state("init extreme count=8", &ca, &ra);
    let (ca, ra) = init_both(&extremes, 10);
    eq_state("init extreme count=10", &ca, &ra);
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..512 {
        let values: Vec<i32> = (0..10)
            .map(|_| extremes[(rng.next_u64() % extremes.len() as u64) as usize])
            .collect();
        let (ca, ra) = init_both(&values, 10);
        eq_state("init extreme permutation", &ca, &ra);
    }
}

#[test]
fn cfg_init_clamped() {
    let mut rng = Rng::new(SEED ^ 23);
    for count in [11i32, 12, 100, 1000, i32::MAX] {
        let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let (ca, ra) = init_both(&values, count);
        assert_eq!(ca.count, 10, "C must clamp count={count} to 10");
        assert_eq!(ra.count, 10, "Rust must clamp count={count} to 10");
        eq_state(&format!("init clamped count={count}"), &ca, &ra);
    }
}

#[test]
fn cfg_init_reinit_stale() {
    let (c_init, r_init) = pair::<FnInit>("init_result_array");
    let mut rng = Rng::new(SEED ^ 24);
    for (first, second) in [(10i32, 3i32), (8, 0), (10, 1), (5, 9), (12, 2), (7, 7)] {
        let mut ca = ResultArray::zeroed();
        let mut ra = ResultArray::zeroed();
        let mut v1: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let mut v2: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        unsafe {
            c_init(&mut ca, v1.as_mut_ptr(), first);
            r_init(&mut ra, v1.as_mut_ptr(), first);
            c_init(&mut ca, v2.as_mut_ptr(), second);
            r_init(&mut ra, v2.as_mut_ptr(), second);
        }
        eq_state(&format!("init re-init {first} then {second}"), &ca, &ra);
    }
}

// ===========================================================================
// CONFIGS rows 25-32 — process_with_foreach
// ===========================================================================

fn ops_for(lib: &'static Library) -> [BinOp; 4] {
    [
        *get::<BinOp>(lib, OP_NAMES[0]),
        *get::<BinOp>(lib, OP_NAMES[1]),
        *get::<BinOp>(lib, OP_NAMES[2]),
        *get::<BinOp>(lib, OP_NAMES[3]),
    ]
}

/// Runs `process_with_foreach` once per side with each side's own `op` symbol
/// and asserts both the return value and the mutated array match.
fn foreach_one(op_idx: usize, values: &[i32], count: i32, ctx: &str) {
    let (c, r) = libs();
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);
    let (mut ca, mut ra) = init_both(values, count);
    let cv = unsafe { c_fe(&mut ca, c_ops[op_idx]) };
    let rv = unsafe { r_fe(&mut ra, r_ops[op_idx]) };
    eq_i32(&format!("foreach {} {ctx}", OP_NAMES[op_idx]), cv, rv);
    eq_state(&format!("foreach {} {ctx}", OP_NAMES[op_idx]), &ca, &ra);
}

fn foreach_row(op_idx: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    for count in 0..=10i32 {
        for iter in 0..256 {
            let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            foreach_one(
                op_idx,
                &values,
                count,
                &format!("count={count} iter={iter} values={values:?}"),
            );
        }
    }
}

#[test]
fn cfg_foreach_add() {
    foreach_row(0, SEED ^ 25);
}

#[test]
fn cfg_foreach_multiply() {
    foreach_row(1, SEED ^ 26);
}

#[test]
fn cfg_foreach_subtract() {
    foreach_row(2, SEED ^ 27);
}

#[test]
fn cfg_foreach_modulo() {
    foreach_row(3, SEED ^ 28);
}

#[test]
fn cfg_foreach_saturating() {
    let extremes = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 30,
        -(1 << 30),
        0,
        -1,
    ];
    let mut rng = Rng::new(SEED ^ 29);
    for op_idx in 0..4 {
        for iter in 0..256 {
            let values: Vec<i32> = (0..10)
                .map(|_| extremes[(rng.next_u64() % extremes.len() as u64) as usize])
                .collect();
            for count in [1i32, 2, 8, 10] {
                foreach_one(
                    op_idx,
                    &values,
                    count,
                    &format!("saturating iter={iter} count={count} values={values:?}"),
                );
            }
        }
    }
}

#[test]
fn cfg_foreach_repeated() {
    let (c, r) = libs();
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);
    let mut rng = Rng::new(SEED ^ 30);
    for op_idx in 0..4 {
        for count in 0..=10i32 {
            for iter in 0..64 {
                let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
                let (mut ca, mut ra) = init_both(&values, count);
                for pass in 0..3 {
                    let cv = unsafe { c_fe(&mut ca, c_ops[op_idx]) };
                    let rv = unsafe { r_fe(&mut ra, r_ops[op_idx]) };
                    let ctx = format!(
                        "repeated {} count={count} iter={iter} pass={pass}",
                        OP_NAMES[op_idx]
                    );
                    eq_i32(&ctx, cv, rv);
                    eq_state(&ctx, &ca, &ra);
                }
            }
        }
    }
}

#[test]
fn cfg_foreach_all_ops_sequence() {
    let (c, r) = libs();
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);
    let mut rng = Rng::new(SEED ^ 31);
    for count in 0..=10i32 {
        for iter in 0..128 {
            let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            let (mut ca, mut ra) = init_both(&values, count);
            for k in 0..4 {
                let cv = unsafe { c_fe(&mut ca, c_ops[k]) };
                let rv = unsafe { r_fe(&mut ra, r_ops[k]) };
                let ctx = format!("sequence count={count} iter={iter} step={k}");
                eq_i32(&ctx, cv, rv);
                eq_state(&ctx, &ca, &ra);
            }
        }
    }
}

#[test]
fn cfg_foreach_cross_abi() {
    // Feed the C library's function pointers into the Rust `process_with_foreach`
    // and vice versa: proves the `extern "C"` callback ABI is identical.
    let (c, r) = libs();
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);
    let mut rng = Rng::new(SEED ^ 32);
    for op_idx in 0..4 {
        for count in 0..=10i32 {
            for iter in 0..64 {
                let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();

                // Rust driver + C callback vs C driver + C callback.
                let (mut ca, mut ra) = init_both(&values, count);
                let cv = unsafe { c_fe(&mut ca, c_ops[op_idx]) };
                let rv = unsafe { r_fe(&mut ra, c_ops[op_idx]) };
                let ctx = format!(
                    "cross-abi rust-driver/c-callback {} count={count} iter={iter}",
                    OP_NAMES[op_idx]
                );
                eq_i32(&ctx, cv, rv);
                eq_state(&ctx, &ca, &ra);

                // C driver + Rust callback vs Rust driver + Rust callback.
                let (mut ca, mut ra) = init_both(&values, count);
                let cv = unsafe { c_fe(&mut ca, r_ops[op_idx]) };
                let rv = unsafe { r_fe(&mut ra, r_ops[op_idx]) };
                let ctx = format!(
                    "cross-abi c-driver/rust-callback {} count={count} iter={iter}",
                    OP_NAMES[op_idx]
                );
                eq_i32(&ctx, cv, rv);
                eq_state(&ctx, &ca, &ra);
            }
        }
    }
}

// ===========================================================================
// CONFIGS rows 33-35 — compute_weighted_sum
// ===========================================================================

#[test]
fn cfg_weighted_counts_random() {
    let (c_ws, r_ws) = pair::<FnWsum>("compute_weighted_sum");
    let mut rng = Rng::new(SEED ^ 33);
    for count in 0..=10i32 {
        for iter in 0..512 {
            let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            let (mut ca, mut ra) = init_both(&values, count);
            let cv = unsafe { c_ws(&mut ca) };
            let rv = unsafe { r_ws(&mut ra) };
            let ctx = format!("weighted_sum count={count} iter={iter} values={values:?}");
            eq_i32(&ctx, cv, rv);
            eq_state(&ctx, &ca, &ra);
        }
    }
}

#[test]
fn cfg_weighted_saturating() {
    let (c_ws, r_ws) = pair::<FnWsum>("compute_weighted_sum");
    let extremes = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 30,
        -(1 << 30),
        300000000,
        -300000000,
    ];
    let mut rng = Rng::new(SEED ^ 34);
    for iter in 0..1024 {
        let values: Vec<i32> = (0..10)
            .map(|_| extremes[(rng.next_u64() % extremes.len() as u64) as usize])
            .collect();
        for count in [1i32, 2, 3, 9, 10] {
            let (mut ca, mut ra) = init_both(&values, count);
            let cv = unsafe { c_ws(&mut ca) };
            let rv = unsafe { r_ws(&mut ra) };
            eq_i32(
                &format!("weighted_sum saturating iter={iter} count={count} values={values:?}"),
                cv,
                rv,
            );
        }
    }
}

#[test]
fn cfg_weighted_single() {
    // count == 1: `current > base` is false, so the ternary yields weight 1.
    let (c_ws, r_ws) = pair::<FnWsum>("compute_weighted_sum");
    let mut rng = Rng::new(SEED ^ 35);
    let mut cases: Vec<i32> = BOUNDARY_I32.to_vec();
    for _ in 0..2048 {
        cases.push(rng.next_i32_mixed());
    }
    for v in cases {
        let values = [v; 10];
        let (mut ca, mut ra) = init_both(&values, 1);
        let cv = unsafe { c_ws(&mut ca) };
        let rv = unsafe { r_ws(&mut ra) };
        eq_i32(&format!("weighted_sum count=1 value={v}"), cv, rv);
    }
}

// ===========================================================================
// CONFIGS rows 36-38 — the composed low-level pipeline
// ===========================================================================

/// Rebuilds `arrayfunc`'s body out of the low-level exports, comparing every
/// intermediate value and the whole array state after every step.
fn pipeline(values: &[i32], count: i32, op_order: [usize; 4], ctx: &str) -> (i32, i32) {
    let (c, r) = libs();
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let (c_ws, r_ws) = pair::<FnWsum>("compute_weighted_sum");
    let (c_cmp, r_cmp) = pair::<FnCmp>("compare_results_in_array");
    let (c_sdti, r_sdti) = pair::<FnSafeDti>("safe_double_to_int");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);

    let (mut ca, mut ra) = init_both(values, count);

    let mut c_res: i32 = 0;
    let mut r_res: i32 = 0;
    for (step, &k) in op_order.iter().enumerate() {
        let cv = unsafe { c_fe(&mut ca, c_ops[k]) };
        let rv = unsafe { r_fe(&mut ra, r_ops[k]) };
        let sctx = format!("{ctx} step={step} op={}", OP_NAMES[k]);
        eq_i32(&format!("pipeline foreach {sctx}"), cv, rv);
        eq_state(&format!("pipeline foreach {sctx}"), &ca, &ra);
        c_res = c_res.wrapping_add(cv);
        r_res = r_res.wrapping_add(rv);
        eq_i32(&format!("pipeline running total {sctx}"), c_res, r_res);
    }

    let cw = unsafe { c_ws(&mut ca) };
    let rw = unsafe { r_ws(&mut ra) };
    eq_i32(&format!("pipeline weighted_sum {ctx}"), cw, rw);
    c_res = c_res.wrapping_add(cw);
    r_res = r_res.wrapping_add(rw);

    let mut i: i32 = 0;
    while i < ca.count - 1 {
        let cc = unsafe { c_cmp(&mut ca, i, i + 1) };
        let rc = unsafe { r_cmp(&mut ra, i, i + 1) };
        eq_i32(&format!("pipeline compare {ctx} i={i}"), cc, rc);
        c_res = c_res.wrapping_add(cc);
        r_res = r_res.wrapping_add(rc);
        i += 1;
    }
    eq_i32(&format!("pipeline pre-scale total {ctx}"), c_res, r_res);

    let c_final = unsafe { c_sdti(c_res as f64 * 0.333) };
    let r_final = unsafe { r_sdti(r_res as f64 * 0.333) };
    eq_i32(&format!("pipeline final {ctx}"), c_final, r_final);
    (c_final, r_final)
}

#[test]
fn cfg_pipeline_arrayfunc_replica() {
    // Same shape `arrayfunc` uses: count = 8, ops in declaration order. The
    // composed result must also equal what `arrayfunc` itself returns.
    let (c_af, r_af) = pair::<FnArrayfunc>("arrayfunc");
    let mut rng = Rng::new(SEED ^ 36);
    for iter in 0..1024 {
        let p1 = rng.next_i32_mixed();
        let p2 = rng.next_i32_mixed();
        let p3 = rng.next_i32_mixed();
        let p4 = rng.next_i32_mixed();
        let values = [
            p1,
            p2,
            p3,
            p4,
            p1.wrapping_add(p2),
            p2.wrapping_sub(p3),
            p3.wrapping_mul(2),
            (p4 / 2).wrapping_add(1),
            0,
            0,
        ];
        let ctx = format!("replica iter={iter} p=({p1},{p2},{p3},{p4})");
        let (cf, rf) = pipeline(&values, 8, [0, 1, 2, 3], &ctx);
        let ca = unsafe { c_af(p1, p2, p3, p4) };
        let rav = unsafe { r_af(p1, p2, p3, p4) };
        eq_i32(&format!("arrayfunc {ctx}"), ca, rav);
        assert_eq!(cf, ca, "C pipeline replica must equal C arrayfunc [{ctx}]");
        assert_eq!(rf, rav, "Rust pipeline replica must equal Rust arrayfunc [{ctx}]");
    }
}

#[test]
fn cfg_pipeline_all_counts() {
    let mut rng = Rng::new(SEED ^ 37);
    for count in 0..=10i32 {
        for iter in 0..128 {
            let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            pipeline(
                &values,
                count,
                [0, 1, 2, 3],
                &format!("all-counts count={count} iter={iter} values={values:?}"),
            );
        }
    }
    // clamped counts too
    for count in [11i32, 25, i32::MAX] {
        for iter in 0..32 {
            let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            pipeline(
                &values,
                count,
                [0, 1, 2, 3],
                &format!("all-counts clamped count={count} iter={iter}"),
            );
        }
    }
}

fn permutations4() -> Vec<[usize; 4]> {
    let mut out = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let p = [a, b, c, d];
                    let mut seen = [false; 4];
                    for &x in p.iter() {
                        seen[x] = true;
                    }
                    if seen.iter().all(|&s| s) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

#[test]
fn cfg_pipeline_op_permutations() {
    let perms = permutations4();
    assert_eq!(perms.len(), 24);
    let mut rng = Rng::new(SEED ^ 38);
    for p in perms {
        for count in [0i32, 1, 2, 8, 10] {
            for iter in 0..16 {
                let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
                pipeline(
                    &values,
                    count,
                    p,
                    &format!("perm={p:?} count={count} iter={iter} values={values:?}"),
                );
            }
        }
    }
}

// ===========================================================================
// CONFIGS rows 39-42 — arrayfunc (the header-declared entry point)
// ===========================================================================

fn af_check(ctx: &str, p1: i32, p2: i32, p3: i32, p4: i32) {
    let (cf, rf) = pair::<FnArrayfunc>("arrayfunc");
    let c = unsafe { cf(p1, p2, p3, p4) };
    let r = unsafe { rf(p1, p2, p3, p4) };
    eq_i32(&format!("arrayfunc {ctx} ({p1},{p2},{p3},{p4})"), c, r);
}

#[test]
fn cfg_arrayfunc_random() {
    let mut rng = Rng::new(SEED ^ 39);
    for i in 0..8192 {
        af_check(
            &format!("random iter={i}"),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
    }
    for i in 0..8192 {
        af_check(
            &format!("uniform iter={i}"),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

#[test]
fn cfg_arrayfunc_small_grid() {
    for p1 in -6..=6i32 {
        for p2 in -6..=6i32 {
            for p3 in -6..=6i32 {
                for p4 in -6..=6i32 {
                    af_check("small grid", p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_arrayfunc_boundaries() {
    let vals = [
        0i32,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 30,
        -(1 << 30),
    ];
    for &p1 in vals.iter() {
        for &p2 in vals.iter() {
            for &p3 in vals.iter() {
                for &p4 in vals.iter() {
                    af_check("boundaries", p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn cfg_arrayfunc_one_extreme() {
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0, -1, 1];
    let mut rng = Rng::new(SEED ^ 42);
    for slot in 0..4 {
        for &e in extremes.iter() {
            for i in 0..256 {
                let mut p = [
                    rng.next_i32_mixed(),
                    rng.next_i32_mixed(),
                    rng.next_i32_mixed(),
                    rng.next_i32_mixed(),
                ];
                p[slot] = e;
                af_check(
                    &format!("one-extreme slot={slot} e={e} iter={i}"),
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                );
            }
        }
    }
}

// ===========================================================================
// CONFIGS row 43 — struct layout / cross-library state round-trip
// ===========================================================================

#[test]
fn cfg_layout_roundtrip() {
    assert_eq!(std::mem::size_of::<Result_>(), 24);
    assert_eq!(std::mem::align_of::<Result_>(), 8);
    assert_eq!(std::mem::size_of::<ResultArray>(), 248);
    let probe = ResultArray::zeroed();
    let basep = &probe as *const ResultArray as usize;
    assert_eq!(&probe.data as *const _ as usize - basep, 0, "data offset");
    assert_eq!(&probe.count as *const _ as usize - basep, 240, "count offset");
    let r0 = &probe.data[0] as *const Result_ as usize;
    assert_eq!(&probe.data[0].value as *const _ as usize - r0, 0, "value offset");
    assert_eq!(&probe.data[0].scaled as *const _ as usize - r0, 8, "scaled offset");
    assert_eq!(&probe.data[0].rank as *const _ as usize - r0, 16, "rank offset");
    assert_eq!(&probe.data[1] as *const _ as usize - r0, 24, "Result stride");

    let (c, r) = libs();
    let (c_init, r_init) = pair::<FnInit>("init_result_array");
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let (c_ws, r_ws) = pair::<FnWsum>("compute_weighted_sum");
    let (c_cmp, r_cmp) = pair::<FnCmp>("compare_results_in_array");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);

    let mut rng = Rng::new(SEED ^ 43);
    for count in 0..=10i32 {
        for iter in 0..64 {
            let mut values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
            let ctx = format!("layout count={count} iter={iter}");

            // C initialises the buffer, then the *Rust* functions consume it;
            // and the mirror image. Both chains must agree.
            let mut a1 = ResultArray::zeroed();
            let mut a2 = ResultArray::zeroed();
            unsafe {
                c_init(&mut a1, values.as_mut_ptr(), count);
                r_init(&mut a2, values.as_mut_ptr(), count);
            }
            eq_state(&format!("{ctx} post-init"), &a1, &a2);

            // C-initialised buffer driven by Rust; Rust-initialised by C.
            let v1 = unsafe { r_fe(&mut a1, r_ops[1]) };
            let v2 = unsafe { c_fe(&mut a2, c_ops[1]) };
            eq_i32(&format!("{ctx} swapped foreach"), v2, v1);
            eq_state(&format!("{ctx} swapped foreach"), &a2, &a1);

            let w1 = unsafe { r_ws(&mut a1) };
            let w2 = unsafe { c_ws(&mut a2) };
            eq_i32(&format!("{ctx} swapped weighted_sum"), w2, w1);

            if count >= 2 {
                let x1 = unsafe { r_cmp(&mut a1, 0, count - 1) };
                let x2 = unsafe { c_cmp(&mut a2, 0, count - 1) };
                eq_i32(&format!("{ctx} swapped compare"), x2, x1);
            }
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

// --- ERRORS #1: modulo_operation with a zero divisor -----------------------

#[test]
fn err_modulo_zero_divisor() {
    let (cf, rf) = pair::<BinOp>("modulo_operation");
    let mut rng = Rng::new(SEED ^ 101);
    let mut cases: Vec<i32> = BOUNDARY_I32.to_vec();
    for _ in 0..4096 {
        cases.push(rng.next_i32_mixed());
    }
    for a in cases {
        let c = unsafe { cf(a, 0, rng.next_i32(), rng.next_i32()) };
        let r = unsafe { rf(a, 0, 0, 0) };
        eq_i32(&format!("modulo_operation zero divisor a={a}"), c, r);
        assert_eq!(c, 0, "C sentinel for b==0 must be 0 (a={a})");
    }
}

// --- ERRORS #3/#4/#5: safe_double_to_int sentinels -------------------------

#[test]
fn err_sdti_upper_clamp() {
    let (cf, rf) = pair::<FnSafeDti>("safe_double_to_int");
    let mut cases = vec![
        i32::MAX as f64,
        2147483648.0,
        2147483649.0,
        4e9,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ];
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..2048 {
        // uniformly sample values at or above the clamp
        let k = (rng.next_u64() >> 11) as f64;
        cases.push(i32::MAX as f64 + k);
    }
    for d in cases {
        let c = unsafe { cf(d) };
        let r = unsafe { rf(d) };
        eq_i32(&format!("sdti upper clamp d={d:?}"), c, r);
        assert_eq!(c, i32::MAX, "C must clamp to INT32_MAX for d={d:?}");
    }
}

#[test]
fn err_sdti_lower_clamp() {
    let (cf, rf) = pair::<FnSafeDti>("safe_double_to_int");
    let mut cases = vec![
        i32::MIN as f64,
        -2147483649.0,
        -2147483650.0,
        -4e9,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ];
    let mut rng = Rng::new(SEED ^ 104);
    for _ in 0..2048 {
        let k = (rng.next_u64() >> 11) as f64;
        cases.push(i32::MIN as f64 - k);
    }
    for d in cases {
        let c = unsafe { cf(d) };
        let r = unsafe { rf(d) };
        eq_i32(&format!("sdti lower clamp d={d:?}"), c, r);
        assert_eq!(c, i32::MIN, "C must clamp to INT32_MIN for d={d:?}");
    }
}

#[test]
fn err_sdti_nan() {
    let (cf, rf) = pair::<FnSafeDti>("safe_double_to_int");
    let mut cases = vec![f64::NAN, -f64::NAN, 0.0 / 0.0, f64::INFINITY - f64::INFINITY];
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..2048 {
        // arbitrary NaN payloads, both signs, quiet and signalling
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let payload = if payload == 0 { 1 } else { payload };
        let sign = (rng.next_u64() & 1) << 63;
        cases.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload));
    }
    for d in cases {
        assert!(d.is_nan(), "test case must be NaN: {:#018x}", d.to_bits());
        let c = unsafe { cf(d) };
        let r = unsafe { rf(d) };
        eq_i32(&format!("sdti NaN bits={:#018x}", d.to_bits()), c, r);
        assert_eq!(c, 0, "C sentinel for NaN must be 0");
    }
}

// --- ERRORS #6/#7: compute_scaled_value saturation / NaN -------------------

#[test]
fn err_compute_scaled_value_saturation() {
    let (cf, rf) = pair::<FnCsv>("compute_scaled_value");
    let cases: [(i32, f64); 12] = [
        (i32::MAX, 2.0),
        (i32::MAX, 1.0000001),
        (i32::MIN, 2.0),
        (i32::MIN, 1.0000001),
        (i32::MAX, -2.0),
        (i32::MIN, -2.0),
        (1, f64::MAX),
        (-1, f64::MAX),
        (1, f64::INFINITY),
        (-1, f64::INFINITY),
        (1, f64::NEG_INFINITY),
        (-1, f64::NEG_INFINITY),
    ];
    for (b, s) in cases {
        let c = unsafe { cf(b, s) };
        let r = unsafe { rf(b, s) };
        eq_i32(&format!("csv saturation base={b} scale={s:?}"), c, r);
        assert!(
            c == i32::MAX || c == i32::MIN,
            "expected a clamp for base={b} scale={s:?}, got {c}"
        );
    }
}

#[test]
fn err_compute_scaled_value_nan() {
    let (cf, rf) = pair::<FnCsv>("compute_scaled_value");
    let cases: [(i32, f64); 6] = [
        (0, f64::NAN),
        (1, f64::NAN),
        (-1, f64::NAN),
        (i32::MAX, f64::NAN),
        (0, f64::INFINITY),      // 0 * inf == NaN
        (0, f64::NEG_INFINITY),  // 0 * -inf == NaN
    ];
    for (b, s) in cases {
        let c = unsafe { cf(b, s) };
        let r = unsafe { rf(b, s) };
        eq_i32(&format!("csv NaN base={b} scale={s:?}"), c, r);
        assert_eq!(c, 0, "C sentinel for NaN product must be 0 (base={b}, scale={s:?})");
    }
}

// --- ERRORS #8/#9/#10/#11: compare_results_in_array rejections ------------

#[test]
fn err_compare_idx1_too_large() {
    let (cf, rf) = pair::<FnCmp>("compare_results_in_array");
    let mut rng = Rng::new(SEED ^ 108);
    for count in 0..=10i32 {
        let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let (mut ca, mut ra) = init_both(&values, count);
        for idx1 in [count, count + 1, count + 5, 10, 11, 1000, i32::MAX] {
            let idx2 = if count > 0 { 0 } else { -1 };
            let c = unsafe { cf(&mut ca, idx1, idx2) };
            let r = unsafe { rf(&mut ra, idx1, idx2) };
            eq_i32(&format!("compare idx1={idx1} too large count={count}"), c, r);
            assert_eq!(c, 0, "C sentinel must be 0 (idx1={idx1}, count={count})");
        }
    }
}

#[test]
fn err_compare_idx2_too_large() {
    let (cf, rf) = pair::<FnCmp>("compare_results_in_array");
    let mut rng = Rng::new(SEED ^ 109);
    for count in 0..=10i32 {
        let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let (mut ca, mut ra) = init_both(&values, count);
        for idx2 in [count, count + 1, count + 5, 10, 11, 1000, i32::MAX] {
            let idx1 = if count > 0 { 0 } else { -1 };
            let c = unsafe { cf(&mut ca, idx1, idx2) };
            let r = unsafe { rf(&mut ra, idx1, idx2) };
            eq_i32(&format!("compare idx2={idx2} too large count={count}"), c, r);
            assert_eq!(c, 0, "C sentinel must be 0 (idx2={idx2}, count={count})");
        }
    }
}

#[test]
fn err_compare_empty_array() {
    // count == 0 (and the clamped-negative counts): every index fails `>= count`.
    let (cf, rf) = pair::<FnCmp>("compare_results_in_array");
    let (c_init, r_init) = pair::<FnInit>("init_result_array");
    let mut values = [7i32; 10];
    for count in [0i32, -1, -5, i32::MIN] {
        let mut ca = ResultArray::zeroed();
        let mut ra = ResultArray::zeroed();
        unsafe {
            c_init(&mut ca, values.as_mut_ptr(), count);
            r_init(&mut ra, values.as_mut_ptr(), count);
        }
        eq_state(&format!("empty-array init count={count}"), &ca, &ra);
        for idx1 in [-2i32, -1, 0, 1, 2, 9, 10] {
            for idx2 in [-2i32, -1, 0, 1, 2, 9, 10] {
                let c = unsafe { cf(&mut ca, idx1, idx2) };
                let r = unsafe { rf(&mut ra, idx1, idx2) };
                eq_i32(
                    &format!("compare empty count={count} idx=({idx1},{idx2})"),
                    c,
                    r,
                );
            }
        }
    }
}

#[test]
fn err_compare_negative_indices() {
    // There is NO lower-bound check in the C, so negative indices pass the
    // guard and out-of-bounds addresses are formed (never dereferenced).
    let (cf, rf) = pair::<FnCmp>("compare_results_in_array");
    let mut rng = Rng::new(SEED ^ 111);
    for count in 1..=10i32 {
        let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let (mut ca, mut ra) = init_both(&values, count);
        for idx1 in -8..=10i32 {
            for idx2 in -8..=10i32 {
                let c = unsafe { cf(&mut ca, idx1, idx2) };
                let r = unsafe { rf(&mut ra, idx1, idx2) };
                eq_i32(
                    &format!("compare negative count={count} idx=({idx1},{idx2})"),
                    c,
                    r,
                );
            }
        }
    }
}

// --- ERRORS #12/#13/#14: init_result_array count handling ------------------

#[test]
fn err_init_count_oversized() {
    let mut rng = Rng::new(SEED ^ 112);
    for count in [11i32, 12, 13, 50, 1000, 65536, i32::MAX, i32::MAX - 1] {
        let values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let (ca, ra) = init_both(&values, count);
        assert_eq!(ca.count, 10, "C clamps oversized count={count}");
        assert_eq!(ra.count, 10, "Rust clamps oversized count={count}");
        eq_state(&format!("init oversized count={count}"), &ca, &ra);
    }
}

#[test]
fn err_init_count_negative() {
    // The ternary keeps the negative value; the loop body never executes, so
    // `data` must stay exactly as it was.
    let (c_init, r_init) = pair::<FnInit>("init_result_array");
    let mut rng = Rng::new(SEED ^ 113);
    for count in [-1i32, -2, -10, -1000, i32::MIN, i32::MIN + 1] {
        let mut values: Vec<i32> = (0..10).map(|_| rng.next_i32_mixed()).collect();
        let mut ca = ResultArray::zeroed();
        let mut ra = ResultArray::zeroed();
        unsafe {
            c_init(&mut ca, values.as_mut_ptr(), count);
            r_init(&mut ra, values.as_mut_ptr(), count);
        }
        assert_eq!(ca.count, count, "C stores the negative count verbatim");
        eq_state(&format!("init negative count={count}"), &ca, &ra);
        for slot in ca.data.iter() {
            assert_eq!(slot.value, 0, "data must be untouched for count={count}");
            assert_eq!(slot.scaled.to_bits(), 0f64.to_bits());
            assert_eq!(slot.rank, 0);
        }
    }
}

#[test]
fn err_init_count_zero() {
    // count == 0: `values` is never read, so even a NULL pointer is fine.
    let (c_init, r_init) = pair::<FnInit>("init_result_array");
    let mut ca = ResultArray::zeroed();
    let mut ra = ResultArray::zeroed();
    unsafe {
        c_init(&mut ca, std::ptr::null_mut(), 0);
        r_init(&mut ra, std::ptr::null_mut(), 0);
    }
    assert_eq!(ca.count, 0);
    eq_state("init count=0 with NULL values", &ca, &ra);
}

// --- ERRORS #15: process_with_foreach on an empty array --------------------

#[test]
fn err_foreach_empty() {
    let (c, r) = libs();
    let (c_fe, r_fe) = pair::<FnForeach>("process_with_foreach");
    let c_ops = ops_for(c);
    let r_ops = ops_for(r);
    let mut values = [1234i32; 10];
    for op_idx in 0..4 {
        let (mut ca, mut ra) = init_both(&values, 0);
        let cv = unsafe { c_fe(&mut ca, c_ops[op_idx]) };
        let rv = unsafe { r_fe(&mut ra, r_ops[op_idx]) };
        eq_i32(&format!("foreach empty {}", OP_NAMES[op_idx]), cv, rv);
        assert_eq!(cv, 0, "C returns 0 for an empty array");
        eq_state("foreach empty must not mutate", &ca, &ra);
        for slot in ca.data.iter() {
            assert_eq!(slot.value, 0);
        }
    }
    let _ = &mut values;
}

// --- ERRORS #16/#17: arrayfunc at the signed boundaries -------------------

#[test]
fn err_arrayfunc_extreme_params() {
    // param4 / 2 at INT32_MIN, and signed overflow in p1+p2, p2-p3, p3*2.
    let interesting = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -1,
        0,
        1,
        i32::MIN / 2,
        i32::MAX / 2,
    ];
    for &p4 in interesting.iter() {
        af_check("param4 division boundary", 0, 0, 0, p4);
        af_check("param4 division boundary", 1, -1, 1, p4);
    }
    for &p1 in interesting.iter() {
        for &p2 in interesting.iter() {
            af_check("p1+p2 overflow", p1, p2, 0, 0);
            af_check("p2-p3 overflow", 0, p1, p2, 0);
        }
    }
    for &p3 in interesting.iter() {
        af_check("p3*2 overflow", 0, 0, p3, 0);
        af_check("p3*2 overflow", i32::MAX, i32::MIN, p3, i32::MIN);
    }
    af_check("all INT32_MIN", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    af_check("all INT32_MAX", i32::MAX, i32::MAX, i32::MAX, i32::MAX);
}

// --- ERRORS #24: compute_weighted_sum saturating terms --------------------

#[test]
fn err_weighted_sum_saturation() {
    let (c_ws, r_ws) = pair::<FnWsum>("compute_weighted_sum");
    for v in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        for count in 1..=10i32 {
            let values = [v; 10];
            let (mut ca, mut ra) = init_both(&values, count);
            let cv = unsafe { c_ws(&mut ca) };
            let rv = unsafe { r_ws(&mut ra) };
            eq_i32(
                &format!("weighted_sum saturation v={v} count={count}"),
                cv,
                rv,
            );
        }
    }
}

// --- ERRORS #25: signed overflow in the arithmetic leaves ------------------

#[test]
fn err_binops_overflow() {
    let cases: [(&str, i32, i32); 12] = [
        ("add_operation", i32::MAX, 1),
        ("add_operation", i32::MIN, -1),
        ("add_operation", i32::MAX, i32::MAX),
        ("add_operation", i32::MIN, i32::MIN),
        ("subtract_operation", i32::MIN, 1),
        ("subtract_operation", i32::MAX, -1),
        ("subtract_operation", i32::MIN, i32::MAX),
        ("subtract_operation", i32::MAX, i32::MIN),
        ("multiply_operation", i32::MIN, -1),
        ("multiply_operation", i32::MAX, 2),
        ("multiply_operation", i32::MIN, 2),
        ("multiply_operation", 65536, 65536),
    ];
    for (name, a, b) in cases {
        let (cf, rf) = pair::<BinOp>(name);
        let c = unsafe { cf(a, b, 0, 0) };
        let r = unsafe { rf(a, b, 0, 0) };
        eq_i32(&format!("{name} overflow a={a} b={b}"), c, r);
    }
}

// ===========================================================================
// Phase C — fatal-signal rows (ERRORS #2, #18-#23) via subprocess
//
// These conditions kill the process, so each is executed in a forked copy of
// this test binary, once against the C `.so` and once against the Rust `.so`.
// The two exit statuses (code + terminating signal) must be identical.
// ===========================================================================

const CRASH_CASE_ENV: &str = "HARVEST_CRASH_CASE";
const CRASH_SIDE_ENV: &str = "HARVEST_CRASH_SIDE";
/// Exit code used when the call unexpectedly returned instead of crashing.
const NO_CRASH_EXIT: i32 = 77;

/// Cases that are independent of the Rust build profile.
/// - `mod_intmin_neg1`: `idiv` overflow — the translation emits the instruction
///   directly, so the trap is identical in every profile.
/// - `foreach_null_op`: an indirect call through a null function pointer. Not a
///   *dereference*, so `ub_checks` does not intercept it.
const CRASH_CASES_PROFILE_INDEPENDENT: [&str; 2] = ["mod_intmin_neg1", "foreach_null_op"];

/// Null-pointer *dereferences*. Identical to the C (SIGSEGV) in the release
/// cdylib, which is the artifact this crate ships (`[profile.release]
/// panic = "abort"`). A `debug-assertions` build inserts Rust's `ub_checks`
/// instrumentation, which converts the same fatal fault into a non-unwinding
/// panic (SIGABRT) before the hardware fault happens. That instrumentation is
/// a compiler diagnostic with no C counterpart and cannot be switched off on
/// stable (the `profile.*.ub-checks` key is unstable), so `run_all.sh` skips
/// exactly this test — and nothing else — for its debug-`.so` pass.
const CRASH_CASES_NULL_DEREF: [&str; 5] = [
    "cmp_null_arr",
    "init_null_arr",
    "init_null_values",
    "foreach_null_arr",
    "wsum_null_arr",
];

/// Executes one fatal case inside `lib`. Returns only if nothing crashed.
unsafe fn run_crash_case(lib: &'static Library, case: &str) {
    let mut arr = ResultArray::zeroed();
    let mut values = [11i32, 22, 33, 44, 55, 66, 77, 88, 99, 110];
    match case {
        // ERRORS #2 — idiv overflow: INT_MIN % -1
        "mod_intmin_neg1" => {
            let f = *get::<BinOp>(lib, "modulo_operation");
            let out = f(i32::MIN, -1, 0, 0);
            println!("modulo_operation(INT_MIN, -1) returned {out}");
        }
        // ERRORS #18
        "cmp_null_arr" => {
            let f = *get::<FnCmp>(lib, "compare_results_in_array");
            let out = f(std::ptr::null_mut(), 0, 0);
            println!("compare_results_in_array(NULL) returned {out}");
        }
        // ERRORS #19
        "init_null_arr" => {
            let f = *get::<FnInit>(lib, "init_result_array");
            f(std::ptr::null_mut(), values.as_mut_ptr(), 4);
            println!("init_result_array(NULL, values, 4) returned");
        }
        // ERRORS #20
        "init_null_values" => {
            let f = *get::<FnInit>(lib, "init_result_array");
            f(&mut arr, std::ptr::null_mut(), 4);
            println!("init_result_array(arr, NULL, 4) returned");
        }
        // ERRORS #21
        "foreach_null_arr" => {
            let f = *get::<FnForeach>(lib, "process_with_foreach");
            let op = *get::<BinOp>(lib, "add_operation");
            let out = f(std::ptr::null_mut(), op);
            println!("process_with_foreach(NULL, add) returned {out}");
        }
        // ERRORS #22 — indirect call through a NULL function pointer
        "foreach_null_op" => {
            let init = *get::<FnInit>(lib, "init_result_array");
            init(&mut arr, values.as_mut_ptr(), 4);
            let f = *get::<FnForeach>(lib, "process_with_foreach");
            // Deliberately invalid: the C accepts any int-sized value here.
            #[allow(invalid_value)]
            let null_op: BinOp = std::mem::transmute::<usize, BinOp>(0);
            let out = f(&mut arr, null_op);
            println!("process_with_foreach(arr, NULL) returned {out}");
        }
        // ERRORS #23
        "wsum_null_arr" => {
            let f = *get::<FnWsum>(lib, "compute_weighted_sum");
            let out = f(std::ptr::null_mut());
            println!("compute_weighted_sum(NULL) returned {out}");
        }
        other => panic!("unknown crash case `{other}`"),
    }
}

/// The worker: a no-op unless the driver set the environment variables.
#[test]
fn crash_worker() {
    let Ok(case) = std::env::var(CRASH_CASE_ENV) else {
        return;
    };
    let side = std::env::var(CRASH_SIDE_ENV).expect("side not set");
    let (c, r) = libs();
    let lib = match side.as_str() {
        "c" => c,
        "rust" => r,
        other => panic!("unknown side `{other}`"),
    };
    unsafe { run_crash_case(lib, &case) };
    // Reached only if the operation did NOT terminate the process.
    std::process::exit(NO_CRASH_EXIT);
}

/// `(exit code, terminating signal)` of the worker for `case` against `side`.
fn spawn_crash_case(case: &str, side: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["crash_worker", "--exact", "--nocapture", "--test-threads=1"])
        .env(CRASH_CASE_ENV, case)
        .env(CRASH_SIDE_ENV, side)
        .env("RUST_BACKTRACE", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn crash worker");
    (status.code(), status.signal())
}

/// Drives every case in `cases`, asserting C/Rust exit-status parity.
fn assert_fatal_parity(cases: &[&str], label: &str) {
    // Skip when this process *is* a worker (the driver already isolated it).
    if std::env::var(CRASH_CASE_ENV).is_ok() {
        return;
    }
    let mut report = String::new();
    for &case in cases {
        let c = spawn_crash_case(case, "c");
        let r = spawn_crash_case(case, "rust");
        report += &format!("  {case:<18} C={c:?} Rust={r:?}\n");
        assert_eq!(
            c, r,
            "fatal-path divergence [{case}]: C exited with {c:?}, Rust with {r:?}\n\
             (first element is the exit code, second the terminating signal)"
        );
        assert_ne!(
            c.0,
            Some(NO_CRASH_EXIT),
            "case `{case}` was expected to terminate the process but returned normally"
        );
        assert!(
            c.1.is_some(),
            "case `{case}` was expected to die from a signal, got {c:?}"
        );
    }
    println!("fatal-path parity ({label}):\n{report}");
}

/// ERRORS #22 plus the `idiv` trap: identical in every Rust build profile.
#[test]
fn err_fatal_traps_and_indirect_call() {
    assert_fatal_parity(&CRASH_CASES_PROFILE_INDEPENDENT, "profile-independent");
}

/// ERRORS #18, #19, #20, #21, #23 — null-pointer dereferences.
#[test]
fn err_null_pointer_dereferences() {
    assert_fatal_parity(&CRASH_CASES_NULL_DEREF, "null dereferences");
}

#[test]
fn err_modulo_intmin_neg1_sigfpe() {
    // ERRORS #2 in isolation, with the expected signal spelled out.
    if std::env::var(CRASH_CASE_ENV).is_ok() {
        return;
    }
    let c = spawn_crash_case("mod_intmin_neg1", "c");
    let r = spawn_crash_case("mod_intmin_neg1", "rust");
    assert_eq!(c, r, "INT_MIN % -1: C={c:?} Rust={r:?}");
    assert_eq!(
        c.1,
        Some(8),
        "expected the C to die on SIGFPE (8) for INT_MIN % -1, got {c:?}"
    );
}
