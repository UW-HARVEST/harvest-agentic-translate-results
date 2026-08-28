//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects through `libloading` and called
//! only through their exported `hsl_to_rgb` symbol — the Rust implementation is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper and the C ABI
//! are part of what is under test.
//!
//! The Rust side is loaded in *both* the `debug` and the `release` build, since
//! they are different codegen (and `release` additionally sets
//! `panic = "abort"`); every differential assertion is made against each of
//! them.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `void hsl_to_rgb(float *dest, const float *src)`
pub type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Lib {
    pub name: String,
    pub path: PathBuf,
    // The library must outlive the function pointer; keep it alive and never
    // drop it (the harness is a process-lifetime singleton).
    _lib: &'static libloading::os::unix::Library,
    pub f: HslToRgb,
}

/// `RTLD_NOW | RTLD_LOCAL` on Linux. `RTLD_NOW` makes the loader resolve *every*
/// undefined symbol immediately, so a successful `dlopen` is itself proof that
/// the object has no unresolved imports. `RTLD_LOCAL` keeps the two objects'
/// identically named `hsl_to_rgb` exports from shadowing each other.
const RTLD_NOW_LOCAL: i32 = 2;

impl Lib {
    /// Open an arbitrary shared object exporting `hsl_to_rgb` (used by
    /// `tests/optlevels.rs` to load the C compiled at several `-O` levels).
    pub fn open_public(name: &str, path: &Path) -> Lib {
        Lib::open(name, path)
    }

    fn open(name: &str, path: &Path) -> Lib {
        let lib = unsafe {
            libloading::os::unix::Library::open(Some(path), RTLD_NOW_LOCAL)
        }
        .unwrap_or_else(|e| panic!("dlopen({}, RTLD_NOW) failed: {e}", path.display()));
        let lib: &'static libloading::os::unix::Library = Box::leak(Box::new(lib));
        let sym: libloading::os::unix::Symbol<HslToRgb> = unsafe { lib.get(b"hsl_to_rgb\0") }
            .unwrap_or_else(|e| panic!("dlsym(hsl_to_rgb) in {} failed: {e}", path.display()));
        Lib {
            name: name.to_string(),
            path: path.to_path_buf(),
            _lib: lib,
            f: *sym,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

/// Find `c_src/build/lib<something>.so`. The CMake project name is derived from
/// the name of the directory *containing* `c_src`, so the file name is not
/// fixed and has to be globbed.
pub fn c_so_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    if let Some(p) = find_so(&build_dir) {
        return p;
    }
    // Not built yet: run the documented CMake build.
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
    assert!(ok, "failed to build the C shared library in {}", build_dir.display());
    find_so(&build_dir).unwrap_or_else(|| {
        panic!("no .so produced in {}", build_dir.display());
    })
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let p = e.ok()?.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") && p.is_file() {
            best = Some(p);
        }
    }
    best
}

/// Build the Rust `cdylib` for `profile` into a private target directory (so the
/// nested cargo invocation cannot deadlock on the outer `cargo test`'s lock on
/// `target/`) and return the resulting `.so`.
fn rust_so_path(profile: &str) -> Option<PathBuf> {
    let target_dir = manifest_dir().join("target").join("ffi-so");
    let out = target_dir.join(profile).join("libhsl_to_rgb_lib.so");

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target_dir)
        // Do not inherit the outer test run's rustflags/feature selection state.
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .arg("build")
        .arg("--offline")
        .arg("--lib");
    if profile == "release" {
        cmd.arg("--release");
    }
    // Forward the feature selection the outer test run was compiled with, so
    // that `cargo test --no-default-features --features X` really tests that
    // configuration of the shared object as well.
    if let Ok(feats) = std::env::var("HSL_TEST_FEATURES") {
        cmd.arg("--no-default-features");
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats);
        }
    }
    let status = cmd.status().ok()?;
    if !status.success() {
        return None;
    }
    if out.is_file() { Some(out) } else { None }
}

pub fn c_lib() -> &'static Lib {
    static C: OnceLock<Lib> = OnceLock::new();
    C.get_or_init(|| {
        let p = c_so_path();
        Lib::open("c", &p)
    })
}

/// Every Rust build variant to be checked against the C.
pub fn rust_libs() -> &'static [Lib] {
    static R: OnceLock<Vec<Lib>> = OnceLock::new();
    R.get_or_init(|| {
        let mut v = Vec::new();
        for profile in ["debug", "release"] {
            if let Some(p) = rust_so_path(profile) {
                v.push(Lib::open(&format!("rust-{profile}"), &p));
            }
        }
        assert!(
            !v.is_empty(),
            "could not build/locate the Rust cdylib (target/ffi-so/{{debug,release}}/libhsl_to_rgb_lib.so)"
        );
        v
    })
}

// ---------------------------------------------------------------------------
// Buffer layouts
// ---------------------------------------------------------------------------

/// A recognisable non-NaN canary so that a stray write is obvious, and so that
/// an accidental "no write at all" cannot masquerade as a match.
pub const CANARY: u32 = 0xDEAD_BEEF;

/// Where `src` and `dest` sit inside one shared allocation. This models
/// disjoint buffers, full aliasing and every partial overlap with a single
/// mechanism, and always keeps canary padding around the touched words so an
/// out-of-bounds store is detected.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub len: usize,
    pub src_off: usize,
    pub dst_off: usize,
}

impl Layout {
    pub const fn new(len: usize, src_off: usize, dst_off: usize) -> Layout {
        Layout { len, src_off, dst_off }
    }
}

/// Disjoint, with 4 canary words in front of `dest` and 4 behind it.
pub const DISJOINT: Layout = Layout::new(11, 0, 4);

/// Run one call against `lib` in the given layout and return the raw bit
/// pattern of the *whole* backing allocation afterwards.
pub fn run_layout(lib: &Lib, lay: Layout, h: f32, s: f32, l: f32) -> Vec<u32> {
    assert!(lay.src_off + 3 <= lay.len && lay.dst_off + 3 <= lay.len);
    // Over-align and pad generously: the buffer is heap allocated so that a
    // genuine out-of-bounds write is likely to be caught by the canaries rather
    // than silently clobbering an unrelated stack slot.
    let mut buf: Vec<f32> = (0..lay.len).map(|_| f32::from_bits(CANARY)).collect();
    buf[lay.src_off] = h;
    buf[lay.src_off + 1] = s;
    buf[lay.src_off + 2] = l;
    let base = buf.as_mut_ptr();
    unsafe {
        (lib.f)(base.add(lay.dst_off), base.add(lay.src_off));
    }
    buf.iter().map(|v| v.to_bits()).collect()
}

/// Straightforward disjoint call returning just the three outputs as bits.
pub fn run(lib: &Lib, h: f32, s: f32, l: f32) -> [u32; 3] {
    let out = run_layout(lib, DISJOINT, h, s, l);
    [out[4], out[5], out[6]]
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

fn fmt_f32(bits: u32) -> String {
    format!("{:#010x} ({:e})", bits, f32::from_bits(bits))
}

fn fmt_buf(b: &[u32]) -> String {
    b.iter().map(|&x| fmt_f32(x)).collect::<Vec<_>>().join(", ")
}

/// The core differential assertion: for this input and layout, every Rust
/// variant's post-call memory image must equal the C's, word for word.
pub fn assert_same_layout(ctx: &str, lay: Layout, h: f32, s: f32, l: f32) {
    let cbuf = run_layout(c_lib(), lay, h, s, l);
    for r in rust_libs() {
        let rbuf = run_layout(r, lay, h, s, l);
        if cbuf != rbuf {
            panic!(
                "DIVERGENCE [{ctx}] ({} vs c)\n  layout   : {lay:?}\n  h = {}\n  s = {}\n  l = {}\n  C   : {}\n  Rust: {}",
                r.name,
                fmt_f32(h.to_bits()),
                fmt_f32(s.to_bits()),
                fmt_f32(l.to_bits()),
                fmt_buf(&cbuf),
                fmt_buf(&rbuf),
            );
        }
    }
}

pub fn assert_same(ctx: &str, h: f32, s: f32, l: f32) {
    assert_same_layout(ctx, DISJOINT, h, s, l);
}

/// Check a whole batch, reporting at most a handful of failures at once.
pub fn assert_same_batch(ctx: &str, cases: impl IntoIterator<Item = (f32, f32, f32)>) {
    let mut n = 0usize;
    for (h, s, l) in cases {
        assert_same(ctx, h, s, l);
        n += 1;
    }
    assert!(n > 0, "[{ctx}] generated no cases");
}

// ---------------------------------------------------------------------------
// Deterministic randomness (PCG32, fixed seeds — reproducible)
// ---------------------------------------------------------------------------

pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng { state: 0, inc: (seed << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(0x853c_49e6_748f_ea9b ^ seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[0, 1)` with 24 random mantissa bits.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    /// A completely arbitrary `f32` bit pattern (so ~0.4 % NaNs, subnormals,
    /// infinities and both zeros all appear naturally).
    pub fn bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A finite-or-not float with a uniformly random *exponent*, which spreads
    /// the samples over the whole magnitude range instead of concentrating them
    /// near `1e38` the way `bits_f32` does.
    pub fn log_uniform(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let exp = self.below(255); // 0 (subnormal) .. 254 (max finite)
        let mant = self.next_u32() & 0x007f_ffff;
        f32::from_bits(sign | (exp << 23) | mant)
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
}

// ---------------------------------------------------------------------------
// Special-value pools
// ---------------------------------------------------------------------------

/// Every interesting bit-pattern class of an `f32`, including a signalling NaN
/// (bit 22 clear) and NaNs with a non-trivial payload and a set sign bit —
/// exactly the values whose propagation depends on the SSE
/// "first-source-operand-wins" rule the translation has to reproduce.
pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-45,  // FLT_TRUE_MIN (smallest subnormal)
    -1e-45,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    60.0,
    120.0,
    180.0,
    240.0,
    300.0,
    360.0,
];

/// NaN patterns kept separate so a test can ask for "definitely a NaN".
pub const NANS: &[u32] = &[
    0x7fc0_0000, // default quiet NaN
    0xffc0_0000, // negative quiet NaN
    0x7fc0_1234, // quiet NaN, payload
    0xffff_ffff, // quiet NaN, all payload bits set, negative
    0x7f80_0001, // signalling NaN, minimal payload
    0xff80_0001, // negative signalling NaN
    0x7fbf_ffff, // signalling NaN, maximal payload
];

pub fn nan_floats() -> Vec<f32> {
    NANS.iter().map(|&b| f32::from_bits(b)).collect()
}

/// `SPECIALS` plus every NaN pattern.
pub fn specials_and_nans() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIALS.to_vec();
    v.extend(nan_floats());
    v
}

/// The exact sector boundaries the `if/else if` chain in `lib.c` tests against,
/// plus signed zero.
pub const HUE_BOUNDARIES: &[f32] = &[-0.0, 0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];

/// One ULP either side of every boundary.
pub fn hue_boundary_neighbours() -> Vec<f32> {
    let mut v = Vec::new();
    for &b in HUE_BOUNDARIES {
        v.push(next_up(b));
        v.push(next_down(b));
    }
    v
}

pub fn next_up(x: f32) -> f32 {
    if x.is_nan() || x == f32::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    if (b >> 31) == 0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

pub fn next_down(x: f32) -> f32 {
    if x.is_nan() || x == f32::NEG_INFINITY {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1 | (1 << 31));
    }
    let b = x.to_bits();
    if (b >> 31) == 0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

/// A hue drawn so that all seven outcomes of the dispatch chain are hit.
pub fn random_hue_any_sector(rng: &mut Rng) -> f32 {
    match rng.below(8) {
        0 => rng.range(0.0, 60.0),
        1 => rng.range(60.0, 120.0),
        2 => rng.range(120.0, 180.0),
        3 => rng.range(180.0, 240.0),
        4 => rng.range(240.0, 300.0),
        5 => rng.range(300.0, 360.0),
        6 => rng.range(360.0, 1.0e6),
        _ => rng.range(-1.0e6, 0.0),
    }
}
