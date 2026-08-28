//! Shared differential-test harness.
//!
//! Both implementations are loaded as **shared objects** through `libloading`
//! and are only ever reached through their exported `hsv_to_rgb` symbol, so the
//! `#[no_mangle] extern "C"` wrapper of the Rust crate is part of what is being
//! tested. Nothing in this harness calls the Rust crate directly (the crate is
//! `crate-type = ["cdylib"]`, so it cannot even be linked as an rlib).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type HsvToRgb = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // The `Library` must outlive the function pointer; it is leaked on purpose
    // (`OnceLock<Impl>` lives for the whole process).
    _lib: Library,
    pub hsv_to_rgb: HsvToRgb,
}

impl Impl {
    /// Call the exported `hsv_to_rgb` symbol.
    ///
    /// # Safety
    /// `dest` must be writable for 3 `f32` and `src` readable for 3 `f32`
    /// (unless the test is deliberately probing an out-of-contract case).
    pub unsafe fn call(&self, dest: *mut f32, src: *const f32) {
        (self.hsv_to_rgb)(dest, src)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the CMake target name is derived from
/// the name of the directory that *contains* `c_src`, so glob instead of
/// hard-coding it.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HSV_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().parent().unwrap().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C shared object found in {}\n\
             build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
        _ => found.remove(0),
    }
}

/// The Rust cdylib. Prefers the profile directory the running test binary lives
/// in (`target/debug` for `cargo test`, `target/release` for
/// `cargo test --release`) and falls back to the other one.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HSV_RUST_SO") {
        return PathBuf::from(p);
    }
    const SO: &str = "libhsv_to_rgb_lib.so";
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<testbin> -> target/<profile>
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join(SO));
        }
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("debug").join(SO));
    candidates.push(target.join("release").join(SO));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "no Rust shared object found (looked at {:?})\nbuild it with:\n  \
         cd translation && cargo build --offline",
        candidates
    );
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    unsafe {
        let lib = Library::new(&path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let sym: Symbol<HsvToRgb> = lib
            .get(b"hsv_to_rgb\0")
            .unwrap_or_else(|e| panic!("{} exports no `hsv_to_rgb`: {e}", path.display()));
        let f = *sym;
        Impl {
            name,
            path,
            _lib: lib,
            hsv_to_rgb: f,
        }
    }
}

pub fn c() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| load("C", c_so_path()))
}

pub fn rust() -> &'static Impl {
    static R: OnceLock<Impl> = OnceLock::new();
    R.get_or_init(|| load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// bit helpers — every comparison is done on raw bit patterns, never on `==`,
// so that NaN payloads and the sign of zero are part of the assertion.
// ---------------------------------------------------------------------------

pub fn bits(x: f32) -> u32 {
    x.to_bits()
}

pub fn bits3(x: &[f32]) -> [u32; 3] {
    [x[0].to_bits(), x[1].to_bits(), x[2].to_bits()]
}

pub fn f(b: u32) -> f32 {
    f32::from_bits(b)
}

pub fn show(x: f32) -> String {
    format!("{:?}(0x{:08x})", x, x.to_bits())
}

pub fn show3(x: &[u32; 3]) -> String {
    format!(
        "[{}, {}, {}]",
        show(f(x[0])),
        show(f(x[1])),
        show(f(x[2]))
    )
}

/// The canary word written around every output buffer; any surviving change
/// outside `dest[0..3]` is an out-of-bounds write.
pub const CANARY: u32 = 0xDEAD_BEEF;

/// Run one `(h, s, v)` triple through both `.so`s using freshly canaried,
/// disjoint buffers and return `(c_out, rust_out)`.
pub fn run_pair(h: f32, s: f32, v: f32) -> ([u32; 3], [u32; 3]) {
    let src = [h, s, v];
    let mut out = [[0u32; 3]; 2];
    for (idx, imp) in [c(), rust()].into_iter().enumerate() {
        // 3 canaries in front, 3 behind.
        let mut buf = [f(CANARY); 9];
        let src_copy = src;
        unsafe {
            imp.call(buf.as_mut_ptr().add(3), src_copy.as_ptr());
        }
        for (i, w) in buf.iter().enumerate() {
            if i < 3 || i >= 6 {
                assert_eq!(
                    w.to_bits(),
                    CANARY,
                    "{} wrote out of bounds at slot {i} for src={:?}",
                    imp.name,
                    src
                );
            }
        }
        assert_eq!(
            bits3(&src_copy),
            bits3(&src),
            "{} modified its `const float *src` input for src={:?}",
            imp.name,
            src
        );
        out[idx] = bits3(&buf[3..6]);
    }
    (out[0], out[1])
}

/// Assert C and Rust agree bit-for-bit for one triple.
#[track_caller]
pub fn assert_same(label: &str, h: f32, s: f32, v: f32) {
    let (cc, rr) = run_pair(h, s, v);
    assert_eq!(
        cc,
        rr,
        "{label}: divergence for h={} s={} v={}\n  C    = {}\n  Rust = {}",
        show(h),
        show(s),
        show(v),
        show3(&cc),
        show3(&rr)
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64* — fixed seed per test for reproducibility)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // avoid the zero state
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// A completely arbitrary `f32` bit pattern (NaNs, infinities, subnormals
    /// and negative zeros all occur naturally).
    pub fn any_f32(&mut self) -> f32 {
        f(self.next_u32())
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

// ---------------------------------------------------------------------------
// interesting value pools
// ---------------------------------------------------------------------------

/// Every documented / undocumented special `f32` class.
pub const SPECIAL: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,          // smallest normal
    -f32::MIN_POSITIVE,
    1e-45,                      // smallest subnormal
    -1e-45,
    1.0e-40,                    // subnormal
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    16_777_216.0,               // 2^24
    2_147_483_648.0,            // 2^31
    -2_147_483_648.0,
    2_147_483_520.0,            // largest f32 < 2^31
    1e30,
    -1e30,
    f32::INFINITY,
    f32::NEG_INFINITY,
];

/// NaNs, including several payloads and both signalling encodings.
pub const NANS: &[u32] = &[
    0x7FC0_0000, // canonical quiet NaN
    0xFFC0_0000, // negative quiet NaN ("real indefinite")
    0x7FC0_1234, // quiet, payload
    0xFFF0_0F0F, // negative quiet, payload
    0x7F80_0001, // signalling NaN
    0xFF80_0001, // negative signalling NaN
    0x7FBF_FFFF, // largest signalling NaN
    0x7FFF_FFFF, // all-ones quiet NaN
];

pub fn nans() -> impl Iterator<Item = f32> {
    NANS.iter().copied().map(f)
}

/// One representative hue per `switch` arm, plus the arms that can only be
/// reached through the `default:` label.
pub const ARM_HUES: &[(i32, f32)] = &[
    (0, 30.0),
    (1, 90.0),
    (2, 150.0),
    (3, 210.0),
    (4, 270.0),
    (5, 330.0),   // default
    (6, 380.0),   // default
    (-1, -30.0),  // default (unsigned bound check)
];

/// Hue range `[lo, hi)` that produces `i == arm` (arm 5.. and negatives all end
/// up in `default:`).
pub fn hue_range_for_arm(arm: i32) -> (f32, f32) {
    (arm as f32 * 60.0, (arm as f32 + 1.0) * 60.0)
}

// ---------------------------------------------------------------------------
// buffer-shape harness: covers aliasing, overlap, misalignment and the exact
// written extent in one mechanism by comparing the *whole* backing byte buffer
// after the call.
// ---------------------------------------------------------------------------

pub const BUF_LEN: usize = 64;

#[repr(align(16))]
pub struct ByteBuf(pub [u8; BUF_LEN]);

impl ByteBuf {
    fn patterned() -> Self {
        let mut b = ByteBuf([0u8; BUF_LEN]);
        for (j, x) in b.0.iter_mut().enumerate() {
            *x = (j as u8).wrapping_mul(37).wrapping_add(11);
        }
        b
    }
}

/// Place `src` at byte offset `src_off` and `dest` at byte offset `dst_off`
/// inside one 64-byte buffer, call the export, and return the resulting buffer
/// for each implementation. `src_off == dst_off` is the in-place case,
/// `|src_off - dst_off| < 12` is partial overlap, non-multiples of 4 are
/// misaligned.
pub fn run_shaped(
    src_vals: [f32; 3],
    src_off: usize,
    dst_off: usize,
) -> ([u8; BUF_LEN], [u8; BUF_LEN]) {
    assert!(src_off + 12 <= BUF_LEN && dst_off + 12 <= BUF_LEN);
    let mut outs = [[0u8; BUF_LEN]; 2];
    for (i, imp) in [c(), rust()].into_iter().enumerate() {
        let mut buf = ByteBuf::patterned();
        unsafe {
            let base = buf.0.as_mut_ptr();
            for k in 0..3 {
                let bytes = src_vals[k].to_bits().to_ne_bytes();
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(src_off + 4 * k), 4);
            }
            imp.call(base.add(dst_off) as *mut f32, base.add(src_off) as *const f32);
        }
        outs[i] = buf.0;
    }
    (outs[0], outs[1])
}

#[track_caller]
pub fn assert_same_shaped(label: &str, src_vals: [f32; 3], src_off: usize, dst_off: usize) {
    let (cc, rr) = run_shaped(src_vals, src_off, dst_off);
    if cc != rr {
        let diff: Vec<usize> = (0..BUF_LEN).filter(|&i| cc[i] != rr[i]).collect();
        panic!(
            "{label}: divergence for src=[{}, {}, {}] src_off={src_off} dst_off={dst_off}\n  \
             differing byte offsets: {diff:?}\n  C    = {:02x?}\n  Rust = {:02x?}",
            show(src_vals[0]),
            show(src_vals[1]),
            show(src_vals[2]),
            cc,
            rr
        );
    }
}
