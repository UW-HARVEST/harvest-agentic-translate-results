//! Shared harness: loads BOTH shared objects through `libloading` and calls
//! every function across the FFI boundary. The Rust implementation is *never*
//! called directly as a Rust function — only through the exported `.so` symbols,
//! exactly as an external C consumer would.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int, f64) -> c_int;
pub type ScFn = unsafe extern "C" fn(*mut f32, *mut f32, c_int) -> f64;

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}; build the C library first", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|f| f.to_str())
                    .map(|f| f.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libunderhanded_c_nuke_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` first");
}

pub fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let r = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
        Libs { c, r, c_path, r_path }
    })
}

pub fn sym_match(lib: &Library) -> Symbol<'_, MatchFn> {
    unsafe { lib.get(b"match\0") }.expect("symbol `match` missing")
}

pub fn sym_sc(lib: &Library) -> Symbol<'_, ScFn> {
    unsafe { lib.get(b"spectral_contrast\0") }.expect("symbol `spectral_contrast` missing")
}

// ---------------------------------------------------------------------------
// Call wrappers. Each returns the raw result together with the post-call
// contents of every caller-visible buffer, so both are compared bit-for-bit.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchOut {
    pub ret: c_int,
    /// bit patterns of the `test` buffer after the call
    pub test_after: Vec<u64>,
    /// bit patterns of the `reference` buffer after the call
    pub reference_after: Vec<u64>,
}

/// `int match(double*, double*, int, double)`
pub fn call_match(
    lib: &Library,
    test: &[f64],
    reference: &[f64],
    bins: c_int,
    threshold: f64,
) -> MatchOut {
    let f = sym_match(lib);
    let mut t = test.to_vec();
    let mut r = reference.to_vec();
    let ret = unsafe { f(t.as_mut_ptr(), r.as_mut_ptr(), bins, threshold) };
    MatchOut {
        ret,
        test_after: t.iter().map(|x| x.to_bits()).collect(),
        reference_after: r.iter().map(|x| x.to_bits()).collect(),
    }
}

/// `match` with `test` and `reference` pointing at the *same* buffer.
pub fn call_match_aliased(lib: &Library, data: &[f64], bins: c_int, threshold: f64) -> MatchOut {
    let f = sym_match(lib);
    let mut d = data.to_vec();
    let p = d.as_mut_ptr();
    let ret = unsafe { f(p, p, bins, threshold) };
    let bits: Vec<u64> = d.iter().map(|x| x.to_bits()).collect();
    MatchOut { ret, test_after: bits.clone(), reference_after: bits }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScOut {
    /// bit pattern of the returned `double`
    pub ret: u64,
    pub a_after: Vec<u32>,
    pub b_after: Vec<u32>,
}

/// `double spectral_contrast(float*, float*, int)`
pub fn call_sc(lib: &Library, a: &[f32], b: &[f32], length: c_int) -> ScOut {
    let f = sym_sc(lib);
    let mut av = a.to_vec();
    let mut bv = b.to_vec();
    let ret = unsafe { f(av.as_mut_ptr(), bv.as_mut_ptr(), length) };
    ScOut {
        ret: ret.to_bits(),
        a_after: av.iter().map(|x| x.to_bits()).collect(),
        b_after: bv.iter().map(|x| x.to_bits()).collect(),
    }
}

/// `spectral_contrast` with `a == b`.
pub fn call_sc_aliased(lib: &Library, data: &[f32], length: c_int) -> ScOut {
    let f = sym_sc(lib);
    let mut d = data.to_vec();
    let p = d.as_mut_ptr();
    let ret = unsafe { f(p, p, length) };
    let bits: Vec<u32> = d.iter().map(|x| x.to_bits()).collect();
    ScOut { ret: ret.to_bits(), a_after: bits.clone(), b_after: bits }
}

/// `spectral_contrast(NULL, NULL, length)` — legal in C for `length <= 0`.
pub fn call_sc_null(lib: &Library, length: c_int) -> u64 {
    let f = sym_sc(lib);
    unsafe { f(std::ptr::null_mut(), std::ptr::null_mut(), length) }.to_bits()
}

/// `match(NULL, NULL, bins, threshold)`.
pub fn call_match_null(lib: &Library, bins: c_int, threshold: f64) -> c_int {
    let f = sym_match(lib);
    unsafe { f(std::ptr::null_mut(), std::ptr::null_mut(), bins, threshold) }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every row reproducible.
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
    /// uniform in [0, 1)
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    pub fn range(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Input shape generators (axis C of CONFIGS.md)
// ---------------------------------------------------------------------------

/// A handful of NaN bit patterns with *distinct payloads*, including signalling
/// NaNs, so that `ADDSD`/`MULSS` destination-operand propagation is observable.
pub const F64_NANS: [u64; 8] = [
    0x7FF8_0000_0000_0000, // canonical qNaN
    0xFFF8_0000_0000_0000, // negative qNaN (x86 "indefinite")
    0x7FF8_0000_DEAD_BEEF,
    0x7FFF_FFFF_FFFF_FFFF,
    0x7FF0_0000_0000_0001, // sNaN, payload 1
    0xFFF4_1234_5678_9ABC, // negative sNaN
    0x7FF7_FFFF_FFFF_FFFF, // sNaN, max payload
    0x7FF8_1111_2222_3333,
];

pub const F32_NANS: [u32; 6] = [
    0x7FC0_0000,
    0xFFC0_0000,
    0x7FC0_BEEF,
    0x7F80_0001, // sNaN
    0xFFBF_FFFF, // negative sNaN
    0x7FFF_FFFF,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape64 {
    /// C1: uniform [0,1)
    Unit,
    /// C2: uniform [-1,1)
    Signed,
    /// C3: exact small integers — low 32 mantissa bits are zero
    SmallInts,
    /// C3': powers of two
    PowersOfTwo,
    /// C4: constant vector
    Constant,
    /// C5: mixture of +0.0 / -0.0 / small values
    Zeros,
    /// C6: denormals and 1e±300 magnitudes
    Extremes,
    /// C7: infinities mixed in
    Infinities,
    /// C8: NaNs with distinct payloads mixed in
    Nans,
    /// C9: monotone ramp
    Ramp,
    /// C10: single impulse
    Impulse,
    /// fully bit-random doubles (any class)
    BitRandom,
}

pub const ALL_SHAPE64: [Shape64; 12] = [
    Shape64::Unit,
    Shape64::Signed,
    Shape64::SmallInts,
    Shape64::PowersOfTwo,
    Shape64::Constant,
    Shape64::Zeros,
    Shape64::Extremes,
    Shape64::Infinities,
    Shape64::Nans,
    Shape64::Ramp,
    Shape64::Impulse,
    Shape64::BitRandom,
];

pub fn gen64(shape: Shape64, n: usize, rng: &mut Rng) -> Vec<f64> {
    let mut v = Vec::with_capacity(n);
    match shape {
        Shape64::Unit => {
            for _ in 0..n {
                v.push(rng.unit());
            }
        }
        Shape64::Signed => {
            for _ in 0..n {
                v.push(rng.unit() * 2.0 - 1.0);
            }
        }
        Shape64::SmallInts => {
            for _ in 0..n {
                v.push(rng.range(2000) as f64 - 1000.0);
            }
        }
        Shape64::PowersOfTwo => {
            for _ in 0..n {
                let e = rng.range(60) as i32 - 30;
                let s = if rng.bool() { 1.0 } else { -1.0 };
                v.push(s * (2.0f64).powi(e));
            }
        }
        Shape64::Constant => {
            let c = rng.unit() * 10.0;
            for _ in 0..n {
                v.push(c);
            }
        }
        Shape64::Zeros => {
            for _ in 0..n {
                v.push(match rng.range(4) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => 0.0,
                    _ => rng.unit() * 1e-8,
                });
            }
        }
        Shape64::Extremes => {
            for _ in 0..n {
                v.push(match rng.range(5) {
                    0 => 5e-324 * (1 + rng.range(4)) as f64,
                    1 => 1e-320 * rng.unit(),
                    2 => 1e300 * rng.unit(),
                    3 => -1e300 * rng.unit(),
                    _ => f64::MAX * rng.unit(),
                });
            }
        }
        Shape64::Infinities => {
            for _ in 0..n {
                v.push(match rng.range(4) {
                    0 => f64::INFINITY,
                    1 => f64::NEG_INFINITY,
                    _ => rng.unit() * 100.0 - 50.0,
                });
            }
        }
        Shape64::Nans => {
            for _ in 0..n {
                v.push(match rng.range(3) {
                    0 => f64::from_bits(F64_NANS[rng.range(F64_NANS.len())]),
                    1 => f64::from_bits(F64_NANS[rng.range(F64_NANS.len())]),
                    _ => rng.unit() * 10.0,
                });
            }
        }
        Shape64::Ramp => {
            let base = rng.unit();
            let step = rng.unit() * 0.5;
            for i in 0..n {
                v.push(base + step * i as f64);
            }
        }
        Shape64::Impulse => {
            for _ in 0..n {
                v.push(0.0);
            }
            if n > 0 {
                let k = rng.range(n);
                v[k] = 1.0 + rng.unit() * 1000.0;
            }
        }
        Shape64::BitRandom => {
            for _ in 0..n {
                v.push(f64::from_bits(rng.next_u64()));
            }
        }
    }
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape32 {
    Unit,
    Signed,
    Zeros,
    Denormals,
    Huge,
    Infinities,
    Nans,
    Ramp,
    Impulse,
    BitRandom,
}

pub const ALL_SHAPE32: [Shape32; 10] = [
    Shape32::Unit,
    Shape32::Signed,
    Shape32::Zeros,
    Shape32::Denormals,
    Shape32::Huge,
    Shape32::Infinities,
    Shape32::Nans,
    Shape32::Ramp,
    Shape32::Impulse,
    Shape32::BitRandom,
];

pub fn gen32(shape: Shape32, n: usize, rng: &mut Rng) -> Vec<f32> {
    let mut v = Vec::with_capacity(n);
    match shape {
        Shape32::Unit => {
            for _ in 0..n {
                v.push(rng.unit() as f32);
            }
        }
        Shape32::Signed => {
            for _ in 0..n {
                v.push((rng.unit() * 2.0 - 1.0) as f32);
            }
        }
        Shape32::Zeros => {
            for _ in 0..n {
                v.push(if rng.bool() { 0.0 } else { -0.0 });
            }
        }
        Shape32::Denormals => {
            for _ in 0..n {
                v.push(f32::from_bits(rng.next_u32() & 0x807F_FFFF));
            }
        }
        Shape32::Huge => {
            for _ in 0..n {
                let s = if rng.bool() { 1.0f32 } else { -1.0 };
                v.push(s * f32::MAX * (0.5 + 0.5 * rng.unit() as f32));
            }
        }
        Shape32::Infinities => {
            for _ in 0..n {
                v.push(match rng.range(4) {
                    0 => f32::INFINITY,
                    1 => f32::NEG_INFINITY,
                    _ => (rng.unit() * 100.0 - 50.0) as f32,
                });
            }
        }
        Shape32::Nans => {
            for _ in 0..n {
                v.push(match rng.range(3) {
                    0 => f32::from_bits(F32_NANS[rng.range(F32_NANS.len())]),
                    1 => f32::from_bits(F32_NANS[rng.range(F32_NANS.len())]),
                    _ => (rng.unit() * 10.0) as f32,
                });
            }
        }
        Shape32::Ramp => {
            let base = rng.unit() as f32;
            let step = (rng.unit() * 0.5) as f32;
            for i in 0..n {
                v.push(base + step * i as f32);
            }
        }
        Shape32::Impulse => {
            for _ in 0..n {
                v.push(0.0);
            }
            if n > 0 {
                let k = rng.range(n);
                v[k] = 1.0 + (rng.unit() * 1000.0) as f32;
            }
        }
        Shape32::BitRandom => {
            for _ in 0..n {
                v.push(f32::from_bits(rng.next_u32()));
            }
        }
    }
    v
}

/// The `threshold` axis (D) of `CONFIGS.md`.
pub const THRESHOLDS: [f64; 12] = [
    -1.5,
    -0.0,
    0.0,
    1e-12,
    0.25,
    0.5,
    0.9,
    1.0,
    1.5,
    1e12,
    f64::INFINITY,
    f64::NEG_INFINITY,
];

pub fn nan_threshold() -> f64 {
    f64::from_bits(0x7FF8_0000_1234_5678)
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

pub fn assert_match_eq(ctx: &str, c: &MatchOut, r: &MatchOut) {
    if c != r {
        panic!(
            "match divergence [{ctx}]\n  C   ret={} \n  Rust ret={}\n  C   test_after={:?}\n  Rust test_after={:?}\n  C   ref_after={:?}\n  Rust ref_after={:?}",
            c.ret, r.ret, c.test_after, r.test_after, c.reference_after, r.reference_after
        );
    }
}

pub fn assert_sc_eq(ctx: &str, c: &ScOut, r: &ScOut) {
    if c != r {
        let mut diff = String::new();
        if c.ret != r.ret {
            diff.push_str(&format!(
                "  ret: C=0x{:016X} ({}) Rust=0x{:016X} ({})\n",
                c.ret,
                f64::from_bits(c.ret),
                r.ret,
                f64::from_bits(r.ret)
            ));
        }
        for (i, (x, y)) in c.a_after.iter().zip(r.a_after.iter()).enumerate() {
            if x != y {
                diff.push_str(&format!("  a[{i}]: C=0x{x:08X} Rust=0x{y:08X}\n"));
            }
        }
        for (i, (x, y)) in c.b_after.iter().zip(r.b_after.iter()).enumerate() {
            if x != y {
                diff.push_str(&format!("  b[{i}]: C=0x{x:08X} Rust=0x{y:08X}\n"));
            }
        }
        panic!("spectral_contrast divergence [{ctx}]\n{diff}");
    }
}

// ---------------------------------------------------------------------------
// Out-of-process probes.
//
// A few inputs are undefined behaviour in the C and make the C `.so` crash
// (negative or zero VLA element counts, `NULL` with a positive length). Those
// cannot be exercised in-process without taking the test runner down with them,
// so each is run in a `fork()`ed child and only the *outcome* (clean exit with a
// value, or death by signal) is compared.
// ---------------------------------------------------------------------------

pub mod probe {
    use std::ffi::c_int;

    use libloading::Library;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Outcome {
        Exited(i32),
        Signaled(i32),
    }

    pub const SIGSEGV: i32 = 11;

    impl Outcome {
        pub fn crashed(self) -> bool {
            matches!(self, Outcome::Signaled(_))
        }
    }

    /// Run `f` in a forked child. Returns the child's outcome plus the `u64` it
    /// reported over a pipe (`None` if it died before reporting).
    pub fn run_isolated<F: FnOnce() -> u64>(f: F) -> (Outcome, Option<u64>) {
        let mut fds = [0 as c_int; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            unsafe { libc::close(fds[0]) };
            let v = f();
            let b = v.to_ne_bytes();
            unsafe { libc::write(fds[1], b.as_ptr() as *const libc::c_void, 8) };
            unsafe { libc::close(fds[1]) };
            unsafe { libc::_exit(0) };
        }
        unsafe { libc::close(fds[1]) };
        let mut buf = [0u8; 8];
        let mut got = 0usize;
        loop {
            let n = unsafe {
                libc::read(fds[0], buf[got..].as_mut_ptr() as *mut libc::c_void, 8 - got)
            };
            if n <= 0 {
                break;
            }
            got += n as usize;
            if got == 8 {
                break;
            }
        }
        unsafe { libc::close(fds[0]) };
        let mut status: c_int = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };
        let outcome = if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        };
        (outcome, if got == 8 { Some(u64::from_ne_bytes(buf)) } else { None })
    }

    /// `match(test, reference, bins, threshold)` in a child process.
    pub fn isolated_match(
        lib: &Library,
        test: &[f64],
        reference: &[f64],
        bins: c_int,
        threshold: f64,
    ) -> (Outcome, Option<u64>) {
        let f = super::sym_match(lib);
        let mut t = test.to_vec();
        let mut r = reference.to_vec();
        run_isolated(move || {
            let ret = unsafe { f(t.as_mut_ptr(), r.as_mut_ptr(), bins, threshold) };
            ret as u32 as u64
        })
    }

    /// `match(NULL, NULL, bins, threshold)` in a child process.
    pub fn isolated_match_null(
        lib: &Library,
        bins: c_int,
        threshold: f64,
    ) -> (Outcome, Option<u64>) {
        let f = super::sym_match(lib);
        run_isolated(move || {
            let ret =
                unsafe { f(std::ptr::null_mut(), std::ptr::null_mut(), bins, threshold) };
            ret as u32 as u64
        })
    }

    /// `spectral_contrast(NULL, NULL, length)` in a child process.
    pub fn isolated_sc_null(lib: &Library, length: c_int) -> (Outcome, Option<u64>) {
        let f = super::sym_sc(lib);
        run_isolated(move || {
            unsafe { f(std::ptr::null_mut(), std::ptr::null_mut(), length) }.to_bits()
        })
    }

    /// `spectral_contrast(a, b, length)` in a child process, with `length`
    /// deliberately allowed to exceed the buffers.
    pub fn isolated_sc(
        lib: &Library,
        a: &[f32],
        b: &[f32],
        length: c_int,
    ) -> (Outcome, Option<u64>) {
        let f = super::sym_sc(lib);
        let mut av = a.to_vec();
        let mut bv = b.to_vec();
        run_isolated(move || {
            unsafe { f(av.as_mut_ptr(), bv.as_mut_ptr(), length) }.to_bits()
        })
    }

    /// Named probe used by `differential.rs::row14_match_bins_zero`; returns the
    /// terminating signal of the C and of the Rust `.so` (None ⇒ clean exit).
    pub fn spawn_probe(kind: &str) -> (Option<i32>, Option<i32>) {
        let l = super::libs();
        let (t, r) = (vec![1.25f64; 8], vec![2.5f64; 8]);
        let call = |lib: &Library| -> Outcome {
            match kind {
                "bins_zero" => isolated_match(lib, &t, &r, 0, 0.25).0,
                "bins_negative" => isolated_match(lib, &t, &r, -1, 0.25).0,
                "null_positive" => isolated_match_null(lib, 4, 0.25).0,
                other => panic!("unknown probe kind {other}"),
            }
        };
        let sig = |o: Outcome| match o {
            Outcome::Signaled(s) => Some(s),
            Outcome::Exited(_) => None,
        };
        (sig(call(&l.c)), sig(call(&l.r)))
    }
}
