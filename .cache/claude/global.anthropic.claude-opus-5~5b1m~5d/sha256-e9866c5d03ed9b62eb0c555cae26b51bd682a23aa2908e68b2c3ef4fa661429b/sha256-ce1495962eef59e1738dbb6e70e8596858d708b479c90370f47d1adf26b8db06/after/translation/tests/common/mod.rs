//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported `normalize` symbol — the Rust crate is never
//! linked or called directly, so the `#[no_mangle] extern "C"` wrapper is part
//! of what gets tested.

#![allow(dead_code)]

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `void normalize(float *dest, const float *src, int size);`
pub type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

// ---------------------------------------------------------------------------
// locating and loading the two shared objects
// ---------------------------------------------------------------------------

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn repo_root() -> PathBuf {
    crate_dir().parent().expect("crate has a parent dir").to_path_buf()
}

/// The C `.so`. `c_src/CMakeLists.txt` names the project after the *parent*
/// directory of `c_src`, so the file name is environment dependent: glob it.
pub fn c_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Some(p) = std::env::var_os("NORM_C_SO").map(PathBuf::from) {
            if p.is_file() {
                return p;
            }
        }
        let build_dir = repo_root().join("c_src").join("build");
        if let Some(p) = newest_so_in(&build_dir) {
            return p;
        }
        // Not built yet: build it.
        let _ = std::fs::create_dir_all(&build_dir);
        let ok = Command::new("cmake")
            .current_dir(&build_dir)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && Command::new("cmake")
                .current_dir(&build_dir)
                .args(["--build", "."])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        assert!(ok, "failed to cmake-build the C library in {}", build_dir.display());
        newest_so_in(&build_dir).unwrap_or_else(|| {
            panic!("no *.so produced in {}", build_dir.display())
        })
    })
    .clone()
}

fn newest_so_in(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let e = e.ok()?;
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            let t = e.metadata().and_then(|m| m.modified()).ok()?;
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// The Rust `cdylib`. `cargo test` does *not* build `cdylib` artifacts, so the
/// artifact under test is selected explicitly with `NORM_RUST_SO` (this is what
/// `run_all.sh` does, so that both the `dev` and the `release` `.so` are
/// exercised). Without the variable, prefer `release` — that is the optimised
/// artifact where LLVM may auto-vectorise, i.e. the risky one — then `debug`,
/// then build one into a private target dir (a private target dir avoids
/// contending on the `target/` build lock held by the running `cargo test`).
pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Some(p) = std::env::var_os("NORM_RUST_SO").map(PathBuf::from) {
            assert!(p.is_file(), "NORM_RUST_SO={} is not a file", p.display());
            return p;
        }
        const NAME: &str = "libnormalize_lib.so";
        let target = crate_dir().join("target");
        for d in ["release", "debug"] {
            let p = target.join(d).join(NAME);
            if p.is_file() {
                return p;
            }
        }
        let priv_target = target.join("ffi-so");
        let p = priv_target.join("release").join(NAME);
        if p.is_file() {
            return p;
        }
        let _ = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .current_dir(crate_dir())
            .args(["build", "--release", "--offline", "--target-dir"])
            .arg(&priv_target)
            .status();
        assert!(
            p.is_file(),
            "could not find the Rust cdylib; run `cargo build --release` in {} first",
            crate_dir().display()
        );
        p
    })
    .clone()
}

fn load(path: &Path) -> NormalizeFn {
    unsafe {
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        let sym: Symbol<'static, NormalizeFn> = lib
            .get(b"normalize\0")
            .unwrap_or_else(|e| panic!("dlsym normalize in {}: {e}", path.display()));
        *sym
    }
}

pub fn c_normalize() -> NormalizeFn {
    static F: OnceLock<NormalizeFn> = OnceLock::new();
    *F.get_or_init(|| load(&c_so_path()))
}

pub fn rust_normalize() -> NormalizeFn {
    static F: OnceLock<NormalizeFn> = OnceLock::new();
    *F.get_or_init(|| load(&rust_so_path()))
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) + f32 generators
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// uniform in `0..n`
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// uniform f32 in `[0, 1)`
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// uniform f32 in `[-1, 1)`
    pub fn signed_unit(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    /// random f32 with the given exponent range, random sign
    pub fn scaled(&mut self, lo_exp: i32, hi_exp: i32) -> f32 {
        let e = lo_exp + self.below((hi_exp - lo_exp + 1) as usize) as i32;
        let m = 1.0f32 + self.unit();
        let s = if self.bool() { -1.0f32 } else { 1.0f32 };
        s * m * (2.0f32).powi(e)
    }
    /// random finite (non-NaN, non-inf) f32 drawn from raw bit patterns
    pub fn finite_bits(&mut self) -> f32 {
        loop {
            let b = self.next_u32();
            let f = f32::from_bits(b);
            if f.is_finite() {
                return f;
            }
        }
    }
    pub fn pow2(&mut self) -> f32 {
        let e = -12 + self.below(25) as i32;
        let s = if self.bool() { -1.0f32 } else { 1.0f32 };
        s * (2.0f32).powi(e)
    }
    pub fn subnormal(&mut self) -> f32 {
        // mantissa-only bit pattern -> subnormal
        let b = (self.next_u32() & 0x007F_FFFF) | ((self.next_u32() & 1) << 31);
        f32::from_bits(b)
    }
}

/// The value distributions of `CONFIGS.md` axis D.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dist {
    Unit,
    Wide,
    FiniteBits,
    Pow2,
    Dominant,
    AllEqual,
    Subnormal,
    OneHot,
    SignedZeros,
    SumIsOne,
    Tiny,
    SmallInts,
    OverflowEdge,
}

pub fn gen_data(dist: Dist, n: usize, rng: &mut Rng) -> Vec<f32> {
    match dist {
        Dist::Unit => (0..n).map(|_| rng.signed_unit()).collect(),
        Dist::Wide => (0..n).map(|_| rng.scaled(-40, 40)).collect(),
        Dist::FiniteBits => (0..n).map(|_| rng.finite_bits()).collect(),
        Dist::Pow2 => (0..n).map(|_| rng.pow2()).collect(),
        Dist::Dominant => {
            let big = rng.below(n.max(1));
            (0..n)
                .map(|i| {
                    if i == big {
                        rng.scaled(58, 62)
                    } else {
                        rng.scaled(-62, -58)
                    }
                })
                .collect()
        }
        Dist::AllEqual => {
            let v = rng.scaled(-20, 20);
            vec![v; n]
        }
        Dist::Subnormal => (0..n)
            .map(|_| if rng.bool() { rng.subnormal() } else { rng.signed_unit() })
            .collect(),
        Dist::OneHot => {
            let hot = rng.below(n.max(1));
            let v = loop {
                let v = rng.scaled(-30, 30);
                if v != 0.0 {
                    break v;
                }
            };
            (0..n).map(|i| if i == hot { v } else { 0.0f32 }).collect()
        }
        Dist::SignedZeros => (0..n)
            .map(|_| match rng.below(3) {
                0 => 0.0f32,
                1 => -0.0f32,
                _ => rng.signed_unit(),
            })
            .collect(),
        Dist::SumIsOne => {
            // n components each 1/sqrt(n) is not exact; instead use a power-of-two
            // split so the squares add up to exactly 1.0 in f32.
            if n == 0 {
                return Vec::new();
            }
            // 1 = 2^-k + 2^-k + ... requires n to be a power of two; for the
            // general case put 1.0 in one slot and 0.0 elsewhere (sum == 1.0f
            // exactly), sign randomised.
            let hot = rng.below(n);
            let s = if rng.bool() { -1.0f32 } else { 1.0f32 };
            (0..n).map(|i| if i == hot { s } else { 0.0f32 }).collect()
        }
        Dist::Tiny => (0..n).map(|_| rng.signed_unit() * 1e-20f32).collect(),
        Dist::SmallInts => (0..n)
            .map(|_| {
                let v = (rng.below(9) as f32) - 4.0;
                if rng.bool() { v } else { v * 2.0 }
            })
            .collect(),
        // magnitudes chosen so that `sum` is finite for small n but overflows
        // to +inf once n grows: (1e19)^2 = 1e38, and f32::MAX ~ 3.4e38.
        Dist::OverflowEdge => (0..n).map(|_| if rng.bool() { 1e19f32 } else { -1.8e19f32 }).collect(),
    }
}

// ---------------------------------------------------------------------------
// aligned scratch buffers with sentinel fill
// ---------------------------------------------------------------------------

/// Fill value for bytes the function must not touch.
pub const SENTINEL: u32 = 0xA5C3_5A3C;
/// Number of sentinel floats kept in front of / behind the live window.
pub const GUARD: usize = 8;

struct Aligned {
    ptr: *mut u8,
    layout: Layout,
    len: usize, // floats
}

impl Aligned {
    fn new(len: usize) -> Self {
        let len = len.max(1);
        let layout = Layout::from_size_align(len * 4, 64).unwrap();
        let ptr = unsafe { alloc(layout) };
        assert!(!ptr.is_null(), "allocation of {} floats failed", len);
        Aligned { ptr, layout, len }
    }
    fn as_f32(&self) -> *mut f32 {
        self.ptr as *mut f32
    }
    fn write(&self, bits: &[u32]) {
        assert!(bits.len() <= self.len);
        unsafe { std::ptr::copy_nonoverlapping(bits.as_ptr(), self.ptr as *mut u32, bits.len()) };
    }
    fn read(&self, n: usize) -> Vec<u32> {
        let mut v = vec![0u32; n];
        unsafe { std::ptr::copy_nonoverlapping(self.ptr as *const u32, v.as_mut_ptr(), n) };
        v
    }
}

impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) };
    }
}

/// Which region a pointer argument points into, or `Null`.
#[derive(Clone, Copy, Debug)]
pub enum P {
    Null,
    /// region A at the given float index
    A(usize),
    /// region B at the given float index
    B(usize),
}

/// One differential test case: the initial contents of up to two independent
/// memory regions, plus where `dest`/`src` point and the `size` argument.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub dst: P,
    pub src: P,
    pub size: c_int,
    pub label: String,
}

impl Scenario {
    /// Two disjoint regions: `src` window at `off` in A, `dest` window at `off`
    /// in B, both padded with `GUARD` sentinels front and back.
    pub fn disjoint(data: &[f32], off: usize, size: c_int) -> Scenario {
        let n = data.len();
        let total = GUARD + off + n + GUARD;
        let mut a = vec![SENTINEL; total];
        for (i, v) in data.iter().enumerate() {
            a[GUARD + off + i] = v.to_bits();
        }
        let b = vec![SENTINEL; total];
        Scenario {
            a,
            b,
            dst: P::B(GUARD + off),
            src: P::A(GUARD + off),
            size,
            label: format!("disjoint off={off} n={n} size={size}"),
        }
    }

    /// One region, `dest == src` (exact in-place).
    pub fn in_place(data: &[f32], off: usize, size: c_int) -> Scenario {
        let n = data.len();
        let total = GUARD + off + n + GUARD;
        let mut a = vec![SENTINEL; total];
        for (i, v) in data.iter().enumerate() {
            a[GUARD + off + i] = v.to_bits();
        }
        Scenario {
            a,
            b: Vec::new(),
            dst: P::A(GUARD + off),
            src: P::A(GUARD + off),
            size,
            label: format!("in_place off={off} n={n} size={size}"),
        }
    }

    /// One region, `dest = src + shift` (shift > 0 forward, < 0 backward).
    pub fn overlap(data: &[f32], shift: isize, size: c_int) -> Scenario {
        let n = data.len();
        let pad = shift.unsigned_abs() + GUARD;
        let total = pad + n + pad;
        let mut a = vec![SENTINEL; total];
        let src_i = pad;
        for (i, v) in data.iter().enumerate() {
            a[src_i + i] = v.to_bits();
        }
        let dst_i = (src_i as isize + shift) as usize;
        Scenario {
            a,
            b: Vec::new(),
            dst: P::A(dst_i),
            src: P::A(src_i),
            size,
            label: format!("overlap shift={shift} n={n} size={size}"),
        }
    }
}

/// Result of running one scenario: the final bits of both regions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub a: Vec<u32>,
    pub b: Vec<u32>,
}

pub fn exec(f: NormalizeFn, s: &Scenario) -> Outcome {
    let ra = Aligned::new(s.a.len());
    let rb = Aligned::new(s.b.len());
    ra.write(&s.a);
    rb.write(&s.b);

    let resolve = |p: P| -> *mut f32 {
        match p {
            P::Null => std::ptr::null_mut(),
            P::A(i) => unsafe { ra.as_f32().add(i) },
            P::B(i) => unsafe { rb.as_f32().add(i) },
        }
    };
    let dst = resolve(s.dst);
    let src = resolve(s.src) as *const f32;

    unsafe { f(dst, src, s.size) };

    Outcome { a: ra.read(s.a.len()), b: rb.read(s.b.len()) }
}

fn fmt_bits(v: &[u32]) -> String {
    v.iter()
        .map(|&b| format!("{:08x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_diff(x: &[u32], y: &[u32]) -> Option<usize> {
    if x.len() != y.len() {
        return Some(x.len().min(y.len()));
    }
    x.iter().zip(y).position(|(a, b)| a != b)
}

/// Run one scenario through both `.so`s and assert bit-for-bit equality of
/// every byte of every region (including the sentinel guards).
pub fn assert_same(s: &Scenario) {
    let c = exec(c_normalize(), s);
    let r = exec(rust_normalize(), s);
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE for [{}]\n", s.label);
    msg += &format!("  size = {}, dst = {:?}, src = {:?}\n", s.size, s.dst, s.src);
    for (name, ci, ri, init) in [
        ("region A", &c.a, &r.a, &s.a),
        ("region B", &c.b, &r.b, &s.b),
    ] {
        if ci != ri {
            let i = first_diff(ci, ri).unwrap();
            msg += &format!(
                "  {name}: first difference at float index {i}\n    init  = {:08x}\n    C     = {:08x} ({:e})\n    Rust  = {:08x} ({:e})\n",
                init.get(i).copied().unwrap_or(0),
                ci[i],
                f32::from_bits(ci[i]),
                ri[i],
                f32::from_bits(ri[i]),
            );
            let lo = i.saturating_sub(4);
            let hi = (i + 5).min(ci.len());
            msg += &format!("    init [{lo}..{hi}] = {}\n", fmt_bits(&init[lo..hi.min(init.len())]));
            msg += &format!("    C    [{lo}..{hi}] = {}\n", fmt_bits(&ci[lo..hi]));
            msg += &format!("    Rust [{lo}..{hi}] = {}\n", fmt_bits(&ri[lo..hi]));
        }
    }
    panic!("{msg}");
}

/// The `size` values of `CONFIGS.md` axis A.
pub const SIZES: &[i32] = &[
    1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255, 256, 1024, 4093,
];
