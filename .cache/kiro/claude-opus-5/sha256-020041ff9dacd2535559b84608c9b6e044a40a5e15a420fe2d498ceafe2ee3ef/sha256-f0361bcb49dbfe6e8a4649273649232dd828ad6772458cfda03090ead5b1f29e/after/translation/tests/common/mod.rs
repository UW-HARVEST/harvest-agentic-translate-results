//! Shared differential-test harness.
//!
//! Loads the C shared object AND the Rust `cdylib` through `libloading` and
//! exposes them behind identical raw function pointers, so every call in the
//! test suite crosses a real FFI boundary in both directions. The Rust side is
//! NEVER called directly as a Rust function — only via its `#[no_mangle]`
//! export, exactly as an external consumer would.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// ABI of the single exported symbol: `void hsv_to_rgb(float *, const float *)`.
pub type HsvFn = unsafe extern "C" fn(*mut f32, *const f32);

pub const SYMBOL: &[u8] = b"hsv_to_rgb\0";

/// One loaded implementation under test.
#[derive(Clone, Copy)]
pub struct Impl {
    pub name: &'static str,
    pub f: HsvFn,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C `.so` produced by `c_src/build`. The CMake project name is
/// derived from the parent directory name, so the file name is not fixed;
/// discover it instead of hard-coding it.
fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

/// Locate every Rust `cdylib` build that exists (release and/or debug). Both are
/// compared against C when present, because differing optimisation levels can
/// legalise different floating-point code generation.
fn find_rust_sos() -> Vec<(&'static str, PathBuf)> {
    let target = repo_root().join("translation").join("target");
    let mut out = Vec::new();
    for (label, profile) in [("rust(release)", "release"), ("rust(debug)", "debug")] {
        let p = target.join(profile).join("libhsv_to_rgb_lib.so");
        if p.is_file() {
            out.push((label, p));
        }
    }
    assert!(
        !out.is_empty(),
        "no Rust cdylib found under {}; run `cargo build --release` first",
        target.display()
    );
    out
}

fn load(path: &Path) -> HsvFn {
    // Leaked on purpose: the function pointer must outlive the `Library`
    // borrow for the whole test process.
    let lib = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
    }));
    let sym: Symbol<HsvFn> = unsafe {
        lib.get(SYMBOL)
            .unwrap_or_else(|e| panic!("dlsym hsv_to_rgb in {}: {e}", path.display()))
    };
    *sym
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static RUST_IMPLS: OnceLock<Vec<Impl>> = OnceLock::new();

pub fn c_impl() -> Impl {
    *C_IMPL.get_or_init(|| Impl { name: "c", f: load(&find_c_so()) })
}

pub fn rust_impls() -> &'static [Impl] {
    RUST_IMPLS.get_or_init(|| {
        find_rust_sos()
            .into_iter()
            .map(|(name, p)| Impl { name, f: load(&p) })
            .collect()
    })
}

// ---------------------------------------------------------------------------
// Calling convention used by the comparisons
// ---------------------------------------------------------------------------

/// Destination buffer length. Only the first 3 elements are part of the
/// contract; the trailing elements are canaries that must remain untouched, so
/// a Rust implementation writing outside `dest[0..3]` is caught.
pub const DEST_LEN: usize = 8;
const CANARY: u32 = 0x7F80_1D0Du32; // a fixed, recognisable NaN bit pattern

/// Invoke `imp` with a fresh canary-filled destination and return all
/// `DEST_LEN` result words as raw bits (bit-exact, so `-0.0` vs `+0.0` and
/// distinct NaN payloads are distinguished).
pub fn call(imp: Impl, src: &[f32; 3]) -> [u32; DEST_LEN] {
    let mut buf = [0f32; DEST_LEN];
    for slot in buf.iter_mut() {
        *slot = f32::from_bits(CANARY);
    }
    // `src` is copied so an implementation that mutates its input is detected
    // by the caller comparing `src_after` too.
    let src_copy = *src;
    unsafe { (imp.f)(buf.as_mut_ptr(), src_copy.as_ptr()) };
    let mut out = [0u32; DEST_LEN];
    for (o, b) in out.iter_mut().zip(buf.iter()) {
        *o = b.to_bits();
    }
    out
}

/// Invoke `imp` with `dest` and `src` pointing into the SAME buffer at the
/// given element offsets, reproducing caller aliasing. Returns the whole
/// buffer's bits afterwards.
pub fn call_aliased(
    imp: Impl,
    buf_init: &[f32],
    dest_off: usize,
    src_off: usize,
) -> Vec<u32> {
    let mut buf = buf_init.to_vec();
    assert!(dest_off + 3 <= buf.len() && src_off + 3 <= buf.len());
    unsafe {
        let base = buf.as_mut_ptr();
        (imp.f)(base.add(dest_off), base.add(src_off) as *const f32);
    }
    buf.iter().map(|f| f.to_bits()).collect()
}

fn fmt_bits(bits: &[u32]) -> String {
    bits.iter()
        .map(|b| format!("{:08x}({})", b, f32::from_bits(*b)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_src(src: &[f32; 3]) -> String {
    format!(
        "h={:08x}({}) s={:08x}({}) v={:08x}({})",
        src[0].to_bits(),
        src[0],
        src[1].to_bits(),
        src[1],
        src[2].to_bits(),
        src[2]
    )
}

/// The core assertion: C and every Rust build must produce bit-identical
/// destination buffers (including the canaries) for `src`.
#[track_caller]
pub fn assert_same(row: &str, src: &[f32; 3]) {
    let c = call(c_impl(), src);
    for r in rust_impls() {
        let got = call(*r, src);
        if got != c {
            panic!(
                "[{row}] divergence for {}\n  c    : {}\n  {:<12}: {}",
                fmt_src(src),
                fmt_bits(&c),
                r.name,
                fmt_bits(&got)
            );
        }
        // Canaries: also verify the C reference itself only wrote 3 slots, so a
        // silent agreement on over-writing cannot hide.
        for (i, w) in c.iter().enumerate().skip(3) {
            assert_eq!(
                *w, CANARY,
                "[{row}] C wrote outside dest[0..3] at index {i} for {}",
                fmt_src(src)
            );
        }
    }
}

#[track_caller]
pub fn assert_same_aliased(row: &str, buf: &[f32], dest_off: usize, src_off: usize) {
    let c = call_aliased(c_impl(), buf, dest_off, src_off);
    for r in rust_impls() {
        let got = call_aliased(*r, buf, dest_off, src_off);
        if got != c {
            panic!(
                "[{row}] aliased divergence (dest_off={dest_off} src_off={src_off}) buf={:?}\n  c    : {}\n  {:<12}: {}",
                buf,
                fmt_bits(&c),
                r.name,
                fmt_bits(&got)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
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
    /// Uniform in `[0,1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo,hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// A fully arbitrary 32-bit pattern reinterpreted as `f32` (covers normals,
    /// subnormals, zeros, infinities and both NaN kinds).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Iteration count for the property-style rows.
pub const ITERS: usize = 200_000;

/// Interesting saturation values reused by several rows.
pub const SPECIAL_S: &[f32] = &[
    0.0,
    -0.0,
    1e-45,
    1e-40,
    f32::EPSILON,
    0.25,
    0.5,
    1.0,
    1.5,
    2.0,
    1e30,
    f32::MAX,
    f32::INFINITY,
    -0.5,
    -1.0,
    -1e30,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// Interesting value-channel values reused by several rows.
pub const SPECIAL_V: &[f32] = &[
    0.0,
    -0.0,
    1e-45,
    1e-38,
    0.5,
    1.0,
    2.0,
    1e30,
    f32::MAX,
    f32::INFINITY,
    -1.0,
    -1e30,
    f32::NEG_INFINITY,
    f32::NAN,
];

/// One representative hue inside each `switch` arm, including both ways of
/// reaching `default:`.
pub const ARM_HUES: &[f32] = &[
    30.0,   // i = 0
    90.0,   // i = 1
    150.0,  // i = 2
    210.0,  // i = 3
    270.0,  // i = 4
    330.0,  // i = 5   -> default
    450.0,  // i = 7   -> default
    -30.0,  // i = -1  -> default
    -390.0, // i = -7  -> default
];
