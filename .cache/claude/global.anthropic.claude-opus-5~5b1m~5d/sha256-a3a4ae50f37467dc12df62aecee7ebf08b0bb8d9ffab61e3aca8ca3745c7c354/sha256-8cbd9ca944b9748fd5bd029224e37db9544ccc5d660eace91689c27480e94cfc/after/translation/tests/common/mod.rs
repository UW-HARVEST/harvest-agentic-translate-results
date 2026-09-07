//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `colourblind`
//! purely through its exported C symbol in each — never by linking the Rust
//! crate directly — so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type ColourblindFn = unsafe extern "C" fn(u32, *mut f32, *mut f32, *mut f32);

pub const CB_PROTANOPIA: u32 = 0;
pub const CB_DEUTERANOPIA: u32 = 1;
pub const CB_TRITANOPIA: u32 = 2;
pub const VALID: [u32; 3] = [CB_PROTANOPIA, CB_DEUTERANOPIA, CB_TRITANOPIA];

fn workspace_root() -> PathBuf {
    // .../<root>/translation  ->  .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\n\
                 Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
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

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libcolourblind_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libcolourblind_lib.so not found under {}. Run `cargo build --release` first.",
        target.display()
    );
}

struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    c: ColourblindFn,
    rust: ColourblindFn,
    c_path: PathBuf,
    rust_path: PathBuf,
}

// SAFETY: the loaded libraries are leaked for the lifetime of the process and
// `colourblind` is a pure, reentrant, stateless function in both.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c: Symbol<ColourblindFn> = c_lib
                .get(b"colourblind\0")
                .expect("C .so does not export `colourblind`");
            let rust: Symbol<ColourblindFn> = rust_lib
                .get(b"colourblind\0")
                .expect("Rust .so does not export `colourblind`");
            let c = *c;
            let rust = *rust;
            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    })
}

pub fn c_fn() -> ColourblindFn {
    libs().c
}
pub fn rust_fn() -> ColourblindFn {
    libs().rust
}
pub fn c_so_path() -> &'static Path {
    &libs().c_path
}
pub fn rust_so_path() -> &'static Path {
    &libs().rust_path
}

/// Which of the three slots a pointer argument should point at.
/// Enables the pointer-aliasing configurations (A1..A5 in CONFIGS.md).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    pub r: usize,
    pub g: usize,
    pub b: usize,
    pub name: &'static str,
}

pub const A1_DISTINCT: Alias = Alias { r: 0, g: 1, b: 2, name: "A1 distinct" };
pub const A2_R_EQ_G: Alias = Alias { r: 0, g: 0, b: 2, name: "A2 R==G" };
pub const A3_R_EQ_B: Alias = Alias { r: 0, g: 1, b: 0, name: "A3 R==B" };
pub const A4_G_EQ_B: Alias = Alias { r: 0, g: 1, b: 1, name: "A4 G==B" };
pub const A5_ALL_SAME: Alias = Alias { r: 1, g: 1, b: 1, name: "A5 R==G==B" };
pub const ALL_ALIASES: [Alias; 5] = [A1_DISTINCT, A2_R_EQ_G, A3_R_EQ_B, A4_G_EQ_B, A5_ALL_SAME];

/// Runs `f` on a fresh copy of `slots` with the given aliasing and returns the
/// resulting three slots, plus guard elements to catch out-of-bounds writes.
fn run_one(f: ColourblindFn, imp: u32, slots: [f32; 3], alias: Alias) -> [f32; 3] {
    // 3 payload slots surrounded by guards.
    let mut buf: [f32; 5] = [
        f32::from_bits(0xDEAD_BEEF),
        slots[0],
        slots[1],
        slots[2],
        f32::from_bits(0xFEED_FACE),
    ];
    let base = buf.as_mut_ptr();
    unsafe {
        let pr = base.add(1 + alias.r);
        let pg = base.add(1 + alias.g);
        let pb = base.add(1 + alias.b);
        f(imp, pr, pg, pb);
    }
    assert_eq!(
        buf[0].to_bits(),
        0xDEAD_BEEF,
        "leading guard clobbered (imp={imp}, {})",
        alias.name
    );
    assert_eq!(
        buf[4].to_bits(),
        0xFEED_FACE,
        "trailing guard clobbered (imp={imp}, {})",
        alias.name
    );
    [buf[1], buf[2], buf[3]]
}

fn bits(v: [f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

fn fmt(v: [f32; 3]) -> String {
    format!(
        "[{:?} (0x{:08X}), {:?} (0x{:08X}), {:?} (0x{:08X})]",
        v[0],
        v[0].to_bits(),
        v[1],
        v[1].to_bits(),
        v[2],
        v[2].to_bits()
    )
}

/// Core differential assertion: identical input -> bit-identical output.
#[track_caller]
pub fn assert_same_aliased(ctx: &str, imp: u32, input: [f32; 3], alias: Alias) {
    let got_c = run_one(c_fn(), imp, input, alias);
    let got_rust = run_one(rust_fn(), imp, input, alias);
    assert_eq!(
        bits(got_c),
        bits(got_rust),
        "\nDIVERGENCE [{ctx}]\n  impairment: {imp}\n  aliasing:   {}\n  input:      {}\n  C   ->      {}\n  Rust ->     {}\n",
        alias.name,
        fmt(input),
        fmt(got_c),
        fmt(got_rust),
    );
}

#[track_caller]
pub fn assert_same(ctx: &str, imp: u32, input: [f32; 3]) {
    assert_same_aliased(ctx, imp, input, A1_DISTINCT);
}

/// Asserts the three slots come back bit-for-bit unmodified (the no-op path).
#[track_caller]
pub fn assert_noop_both(ctx: &str, imp: u32, input: [f32; 3]) {
    let got_c = run_one(c_fn(), imp, input, A1_DISTINCT);
    let got_rust = run_one(rust_fn(), imp, input, A1_DISTINCT);
    assert_eq!(
        bits(got_c),
        bits(input),
        "[{ctx}] C modified the slots for out-of-range impairment {imp}: {} -> {}",
        fmt(input),
        fmt(got_c)
    );
    assert_eq!(
        bits(got_rust),
        bits(input),
        "[{ctx}] Rust modified the slots for out-of-range impairment {imp}: {} -> {}",
        fmt(input),
        fmt(got_rust)
    );
    assert_eq!(bits(got_c), bits(got_rust), "[{ctx}] C/Rust disagree for imp={imp}");
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible.
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
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo, hi).
pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// Any float, including NaNs/Infs/subnormals, from a raw bit pattern.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random subnormal (possibly negative, possibly zero).
    pub fn subnormal(&mut self) -> f32 {
        let mantissa = self.next_u32() & 0x007F_FFFF;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | mantissa)
    }
    /// A random NaN with an arbitrary sign and non-zero payload.
    pub fn nan(&mut self) -> f32 {
        let payload = (self.next_u32() & 0x007F_FFFF) | 1;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | 0x7F80_0000 | payload)
    }
    pub fn triple(&mut self, mut gen: impl FnMut(&mut Self) -> f32) -> [f32; 3] {
        [gen(self), gen(self), gen(self)]
    }
}

/// Number of randomized samples per configuration row.
pub const N: usize = 2000;
