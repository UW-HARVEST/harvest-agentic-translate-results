//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `normalize` symbol, so the Rust
//! `#[no_mangle] extern "C"` wrapper is exercised exactly like an external C
//! consumer would exercise it.

#![allow(dead_code)]

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_int;
use std::path::PathBuf;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// loading
// ---------------------------------------------------------------------------

pub type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

pub struct Impl {
    pub name: &'static str,
    pub normalize: NormalizeFn,
    _lib: Library,
}

/// The C shared object. `HARVEST_C_SO` overrides the default location, so the
/// same differential suite can be run against a C `.so` built with different
/// compiler flags.
pub fn c_so_path() -> PathBuf {
    match std::env::var_os("HARVEST_C_SO") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so"),
    }
}

/// `target/<profile>/libnormalize_lib.so` (the artifact an external consumer
/// links against), located relative to the running test binary so that the
/// `dev` and `release` profiles are picked up automatically.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir").to_path_buf();
    let profile = deps.parent().expect("profile dir").to_path_buf();
    for cand in [
        profile.join("libnormalize_lib.so"),
        deps.join("libnormalize_lib.so"),
    ] {
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "libnormalize_lib.so not found in {:?} or {:?} — run `cargo build` first",
        profile, deps
    );
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    assert!(
        path.is_file(),
        "shared object {:?} is missing (build it first)",
        path
    );
    unsafe {
        let lib = Library::new(&path).unwrap_or_else(|e| panic!("dlopen {:?}: {e}", path));
        let sym: Symbol<NormalizeFn> = lib
            .get(b"normalize\0")
            .unwrap_or_else(|e| panic!("dlsym normalize in {:?}: {e}", path));
        let f = *sym;
        Impl {
            name,
            normalize: f,
            _lib: lib,
        }
    }
}

/// `(c, rust)`
pub fn load_impls() -> (Impl, Impl) {
    (load("C", c_so_path()), load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// 64-byte aligned buffers (so element offsets are exactly controllable)
// ---------------------------------------------------------------------------

pub struct AlignedBuf {
    ptr: *mut u8,
    layout: Layout,
    len: usize,
}

impl AlignedBuf {
    pub fn from_slice(v: &[f32]) -> Self {
        let n = v.len().max(1);
        let layout = Layout::from_size_align(n * 4, 64).unwrap();
        let ptr = unsafe { alloc(layout) };
        assert!(!ptr.is_null(), "allocation failed");
        unsafe {
            std::ptr::write_bytes(ptr, 0, n * 4);
            std::ptr::copy_nonoverlapping(v.as_ptr() as *const u8, ptr, v.len() * 4);
        }
        AlignedBuf {
            ptr,
            layout,
            len: v.len(),
        }
    }

    pub fn as_ptr(&self) -> *const f32 {
        self.ptr as *const f32
    }
    pub fn as_mut_ptr(&self) -> *mut f32 {
        self.ptr as *mut f32
    }
    pub fn as_slice(&self) -> &[f32] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const f32, self.len) }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

// ---------------------------------------------------------------------------
// bitwise comparison
// ---------------------------------------------------------------------------

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub fn cmp_bits(what: &str, cv: &[f32], rv: &[f32]) -> Result<(), String> {
    if cv.len() != rv.len() {
        return Err(format!("{what}: length {} vs {}", cv.len(), rv.len()));
    }
    let cb = bits(cv);
    let rb = bits(rv);
    if cb == rb {
        return Ok(());
    }
    let i = cb.iter().zip(rb.iter()).position(|(a, b)| a != b).unwrap();
    let ndiff = cb.iter().zip(rb.iter()).filter(|(a, b)| a != b).count();
    Err(format!(
        "{what}: {ndiff}/{} elements differ; first at [{i}]: C=0x{:08x} ({:e}) Rust=0x{:08x} ({:e})",
        cb.len(),
        cb[i],
        f32::from_bits(cb[i]),
        rb[i],
        f32::from_bits(rb[i])
    ))
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

/// Both regions live inside ONE allocation, at the given element offsets.
/// The whole allocation (payload, guard elements, `src` region) is compared,
/// so stray or missing writes anywhere are detected.
pub fn diff_shared(
    c: &Impl,
    r: &Impl,
    base: &[f32],
    dest_off: usize,
    src_off: usize,
    size: c_int,
) -> Result<(), String> {
    if size > 0 {
        let n = size as usize;
        assert!(dest_off + n <= base.len(), "dest region out of buffer");
        assert!(src_off + n <= base.len(), "src region out of buffer");
    }
    let cb = AlignedBuf::from_slice(base);
    let rb = AlignedBuf::from_slice(base);
    unsafe {
        let d = cb.as_mut_ptr().add(dest_off);
        let s = cb.as_ptr().add(src_off);
        (c.normalize)(d, s, size);
    }
    unsafe {
        let d = rb.as_mut_ptr().add(dest_off);
        let s = rb.as_ptr().add(src_off);
        (r.normalize)(d, s, size);
    }
    cmp_bits("buffer", cb.as_slice(), rb.as_slice())
}

/// Apply a whole SEQUENCE of `normalize` calls to the same allocation (so the
/// composed pipeline is compared, not just one isolated call).
/// Each element of `calls` is `(dest_off, src_off, size)`.
pub fn diff_shared_calls(
    c: &Impl,
    r: &Impl,
    base: &[f32],
    calls: &[(usize, usize, c_int)],
) -> Result<(), String> {
    let cb = AlignedBuf::from_slice(base);
    let rb = AlignedBuf::from_slice(base);
    for &(dest_off, src_off, size) in calls {
        if size > 0 {
            let n = size as usize;
            assert!(dest_off + n <= base.len() && src_off + n <= base.len());
        }
        unsafe {
            let d = cb.as_mut_ptr().add(dest_off);
            let s = cb.as_ptr().add(src_off);
            (c.normalize)(d, s, size);
        }
        unsafe {
            let d = rb.as_mut_ptr().add(dest_off);
            let s = rb.as_ptr().add(src_off);
            (r.normalize)(d, s, size);
        }
    }
    cmp_bits("buffer", cb.as_slice(), rb.as_slice())
}

/// `dest` and `src` live in two separate allocations.
pub fn diff_separate(
    c: &Impl,
    r: &Impl,
    dest_init: &[f32],
    src_init: &[f32],
    size: c_int,
) -> Result<(), String> {
    if size > 0 {
        let n = size as usize;
        assert!(n <= dest_init.len() && n <= src_init.len());
    }
    let cd = AlignedBuf::from_slice(dest_init);
    let cs = AlignedBuf::from_slice(src_init);
    let rd = AlignedBuf::from_slice(dest_init);
    let rs = AlignedBuf::from_slice(src_init);
    unsafe { (c.normalize)(cd.as_mut_ptr(), cs.as_ptr(), size) };
    unsafe { (r.normalize)(rd.as_mut_ptr(), rs.as_ptr(), size) };
    cmp_bits("dest", cd.as_slice(), rd.as_slice())?;
    cmp_bits("src", cs.as_slice(), rs.as_slice())
}

/// Run ONE implementation on a single allocation and return the resulting
/// buffer (used to pin down the C ground-truth value, not only C == Rust).
pub fn run_one(i: &Impl, base: &[f32], dest_off: usize, src_off: usize, size: c_int) -> Vec<f32> {
    let b = AlignedBuf::from_slice(base);
    unsafe {
        let d = b.as_mut_ptr().add(dest_off);
        let s = b.as_ptr().add(src_off);
        (i.normalize)(d, s, size);
    }
    b.as_slice().to_vec()
}

/// Same, with two separate allocations; returns `(dest, src)`.
pub fn run_one_separate(
    i: &Impl,
    dest_init: &[f32],
    src_init: &[f32],
    size: c_int,
) -> (Vec<f32>, Vec<f32>) {
    let d = AlignedBuf::from_slice(dest_init);
    let s = AlignedBuf::from_slice(src_init);
    unsafe { (i.normalize)(d.as_mut_ptr(), s.as_ptr(), size) };
    (d.as_slice().to_vec(), s.as_slice().to_vec())
}

pub fn bits_of(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) + value generators
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
    /// uniform in [0, 1)
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// uniform in [-1, 1)
    pub fn pm1(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    pub fn sign(&mut self) -> f32 {
        if self.bool() {
            -1.0
        } else {
            1.0
        }
    }
    pub fn finite_bits(&mut self) -> f32 {
        loop {
            let f = f32::from_bits(self.next_u32());
            if f.is_finite() {
                return f;
            }
        }
    }
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VClass {
    UniformPm1,
    RandomFiniteBits,
    RandomAnyBits,
    Zeros,
    SignedZeros,
    Tiny,
    Huge,
    WithNan,
    WithInf,
    SingleNonZero,
    MixedMagnitudes,
    SmallInts,
    Boundary,
    NearUnitSum,
}

/// V1..V14 of CONFIGS.md, in order.
pub const ALL_CLASSES: [VClass; 14] = [
    VClass::UniformPm1,
    VClass::RandomFiniteBits,
    VClass::RandomAnyBits,
    VClass::Zeros,
    VClass::SignedZeros,
    VClass::Tiny,
    VClass::Huge,
    VClass::WithNan,
    VClass::WithInf,
    VClass::SingleNonZero,
    VClass::MixedMagnitudes,
    VClass::SmallInts,
    VClass::Boundary,
    VClass::NearUnitSum,
];

pub const BOUNDARY_VALUES: [f32; 22] = [
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    -f32::EPSILON,
    1.0e-45,  // smallest positive denormal
    -1.0e-45, // smallest negative denormal
    1.1754942e-38, // largest denormal
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    0.0,
    -0.0,
    3.0,
    4.0,
    1.0e-20,
    1.0e20,
    16777216.0, // 2^24
];

/// `V1..V14` fill. `out.len()` is the requested element count.
pub fn gen_values(class: VClass, rng: &mut Rng, out: &mut [f32]) {
    let n = out.len();
    match class {
        VClass::UniformPm1 => {
            for x in out.iter_mut() {
                *x = rng.pm1();
            }
        }
        VClass::RandomFiniteBits => {
            for x in out.iter_mut() {
                *x = rng.finite_bits();
            }
        }
        VClass::RandomAnyBits => {
            for x in out.iter_mut() {
                *x = rng.any_bits();
            }
        }
        VClass::Zeros => {
            for x in out.iter_mut() {
                *x = 0.0;
            }
        }
        VClass::SignedZeros => {
            for x in out.iter_mut() {
                *x = if rng.bool() { -0.0 } else { 0.0 };
            }
        }
        VClass::Tiny => {
            for x in out.iter_mut() {
                let e = -30.0 + 10.0 * rng.unit();
                *x = rng.sign() * 10f32.powf(e);
            }
        }
        VClass::Huge => {
            for x in out.iter_mut() {
                let e = 19.0 + 19.4 * rng.unit();
                *x = rng.sign() * 10f32.powf(e);
            }
        }
        VClass::WithNan => {
            for x in out.iter_mut() {
                *x = rng.pm1();
            }
            if n > 0 {
                let idx = rng.below(n);
                let payload = 1 + (rng.next_u32() & 0x7f_fffe);
                let sign = if rng.bool() { 0x8000_0000u32 } else { 0 };
                out[idx] = f32::from_bits(sign | 0x7f80_0000 | payload);
            }
        }
        VClass::WithInf => {
            for x in out.iter_mut() {
                *x = rng.pm1();
            }
            if n > 0 {
                let idx = rng.below(n);
                out[idx] = if rng.bool() {
                    f32::NEG_INFINITY
                } else {
                    f32::INFINITY
                };
            }
        }
        VClass::SingleNonZero => {
            for x in out.iter_mut() {
                *x = 0.0;
            }
            if n > 0 {
                let idx = rng.below(n);
                let mut v = rng.finite_bits();
                if v == 0.0 {
                    v = 1.0;
                }
                out[idx] = v;
            }
        }
        VClass::MixedMagnitudes => {
            for (i, x) in out.iter_mut().enumerate() {
                let e = if i % 2 == 0 {
                    19.0 + 2.0 * rng.unit()
                } else {
                    -22.0 + 2.0 * rng.unit()
                };
                *x = rng.sign() * 10f32.powf(e);
            }
        }
        VClass::SmallInts => {
            for x in out.iter_mut() {
                *x = (rng.below(17) as i32 - 8) as f32;
            }
        }
        VClass::Boundary => {
            for x in out.iter_mut() {
                *x = BOUNDARY_VALUES[rng.below(BOUNDARY_VALUES.len())];
            }
        }
        VClass::NearUnitSum => {
            for x in out.iter_mut() {
                *x = rng.pm1();
            }
            let s: f64 = out.iter().map(|v| (*v as f64) * (*v as f64)).sum();
            if s > 0.0 {
                let k = (1.0 / s.sqrt()) as f32;
                for x in out.iter_mut() {
                    *x *= k;
                }
            }
        }
    }
}

/// Axis-2 sizes of CONFIGS.md.
pub const SIZES: [i32; 25] = [
    0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
    1000,
];

/// Guard/garbage fill used for every element that is not part of the nominal
/// `src` region, so that stray writes show up in the comparison.
pub fn fill_garbage(rng: &mut Rng, out: &mut [f32]) {
    for x in out.iter_mut() {
        *x = rng.finite_bits();
    }
}

// ---------------------------------------------------------------------------
// aliasing layouts
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Alias {
    /// A: two separate allocations
    Sep,
    /// B: one allocation, dest region before src region
    SameDestFirst,
    /// C: one allocation, src region before dest region
    SameSrcFirst,
    /// D: dest == src
    InPlace,
    /// E: dest = src + k
    OverlapDestAfter(usize),
    /// F: dest = src - k
    OverlapDestBefore(usize),
}

/// Offsets for a single-allocation layout: `(buffer_len, dest_off, src_off)`.
/// `head` is a leading pad, `tail` a trailing pad; both are filled with
/// garbage guard values.
pub fn layout_offsets(alias: Alias, size: usize, head: usize) -> (usize, usize, usize) {
    let gap = 2usize;
    let tail = 4usize;
    let (dest_off, src_off) = match alias {
        Alias::Sep => unreachable!("Sep uses two allocations"),
        Alias::SameDestFirst => (head, head + size + gap),
        Alias::SameSrcFirst => (head + size + gap, head),
        Alias::InPlace => (head, head),
        Alias::OverlapDestAfter(k) => (head + k, head),
        Alias::OverlapDestBefore(k) => (head, head + k),
    };
    let len = dest_off.max(src_off) + size + tail;
    (len, dest_off, src_off)
}

/// Overlap distances `k` exercised for the partial-overlap modes.
pub fn overlap_ks(size: usize) -> Vec<usize> {
    let mut v = vec![1usize, 2, 3, size / 2, size.saturating_sub(1)];
    v.retain(|&k| k >= 1 && k < size);
    v.sort_unstable();
    v.dedup();
    v
}
