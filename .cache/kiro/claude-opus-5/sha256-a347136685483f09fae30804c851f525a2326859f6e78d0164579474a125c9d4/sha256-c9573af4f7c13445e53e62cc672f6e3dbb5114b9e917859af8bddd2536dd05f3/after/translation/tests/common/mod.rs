//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and called
//! only through their exported `to_barycentric` symbol, so the Rust
//! `#[no_mangle] extern "C"` wrapper and its ABI are part of what is tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// Mirrors `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
    pub fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

pub type ToBarycentric = unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_one_so(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    found.pop()
}

/// Path to the C shared object built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src").join("build");
    find_one_so(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let target = repo_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let dir = target.join(profile);
        if let Some(p) = find_one_so(&dir) {
            return p;
        }
    }
    panic!(
        "no Rust .so found under {}. Build it with:\n  cd translation && cargo build --release",
        target.display()
    )
}

/// Both libraries, kept alive for the lifetime of the harness.
pub struct Dual {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: ToBarycentric,
    pub rust: ToBarycentric,
}

impl Dual {
    pub fn load() -> Self {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        // SAFETY: both paths point at shared objects we just built ourselves;
        // loading them runs their (empty) initialisers.
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));
            let c_sym: Symbol<ToBarycentric> = c_lib
                .get(b"to_barycentric\0")
                .expect("C .so does not export to_barycentric");
            let r_sym: Symbol<ToBarycentric> = rust_lib
                .get(b"to_barycentric\0")
                .expect("Rust .so does not export to_barycentric");
            let c = *c_sym;
            let rust = *r_sym;
            Dual {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    pub fn call_c(&self, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
        // SAFETY: signature matches the C declaration exactly; all arguments
        // are by-value PODs and the function touches no memory.
        unsafe { (self.c)(p1, p2, p3, p) }
    }

    pub fn call_rust(&self, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
        // SAFETY: see `call_c`.
        unsafe { (self.rust)(p1, p2, p3, p) }
    }
}

/// Accumulates divergences so a whole row is reported at once instead of
/// aborting on the first mismatch.
pub struct Diff<'a> {
    dual: &'a Dual,
    row: &'static str,
    cases: u64,
    failures: Vec<String>,
}

const MAX_REPORTED: usize = 12;

impl<'a> Diff<'a> {
    pub fn new(dual: &'a Dual, row: &'static str) -> Self {
        Diff {
            dual,
            row,
            cases: 0,
            failures: Vec::new(),
        }
    }

    /// Calls both libraries and records a failure unless the two results are
    /// bit-for-bit identical (so `-0.0 != 0.0` and NaN payloads must match).
    pub fn check(&mut self, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) {
        self.cases += 1;
        let c = self.dual.call_c(p1, p2, p3, p);
        let r = self.dual.call_rust(p1, p2, p3, p);
        if c.bits() != r.bits() {
            if self.failures.len() < MAX_REPORTED {
                self.failures.push(format!(
                    "  in  p1=({:#010x},{:#010x}) p2=({:#010x},{:#010x}) \
                     p3=({:#010x},{:#010x}) p=({:#010x},{:#010x})\n\
                     \x20     [{:e},{:e}] [{:e},{:e}] [{:e},{:e}] [{:e},{:e}]\n\
                     \x20 C   = ({:#010x},{:#010x}) [{:e},{:e}]\n\
                     \x20rust = ({:#010x},{:#010x}) [{:e},{:e}]",
                    p1.x.to_bits(), p1.y.to_bits(),
                    p2.x.to_bits(), p2.y.to_bits(),
                    p3.x.to_bits(), p3.y.to_bits(),
                    p.x.to_bits(), p.y.to_bits(),
                    p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, p.x, p.y,
                    c.x.to_bits(), c.y.to_bits(), c.x, c.y,
                    r.x.to_bits(), r.y.to_bits(), r.x, r.y,
                ));
            } else {
                self.failures.push(String::new());
            }
        }
    }

    /// Panics if anything diverged. Returns the number of cases checked.
    pub fn finish(self) -> u64 {
        let n = self.failures.iter().filter(|s| !s.is_empty()).count();
        let total = self.failures.len();
        if total > 0 {
            let mut msg = format!(
                "{}: {} of {} cases diverged (showing {})\n",
                self.row, total, self.cases, n
            );
            for f in self.failures.iter().filter(|s| !s.is_empty()) {
                msg.push_str(f);
                msg.push('\n');
            }
            panic!("{msg}");
        }
        assert!(self.cases > 0, "{}: no cases were checked", self.row);
        self.cases
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[-1, 1)`.
    pub fn signed_unit(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }

    /// Small integer-valued float in `[-mag, mag]` — every intermediate of the
    /// dot products stays exact in binary32 for small magnitudes.
    pub fn small_int(&mut self, mag: i32) -> f32 {
        (self.next_u32() as i32).rem_euclid(2 * mag + 1) as f32 - mag as f32
    }

    /// Any 32-bit word reinterpreted as `f32` (every IEEE class).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Random *normal* float spanning the whole binary32 normal range.
    pub fn wide_normal(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let exp = 1 + self.below(254); // 1..=254 -> normal
        let mant = self.next_u32() & 0x007F_FFFF;
        f32::from_bits(sign | (exp << 23) | mant)
    }

    /// Random subnormal (`exp == 0`, non-zero mantissa).
    pub fn subnormal(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let mant = 1 + (self.next_u32() & 0x007F_FFFE);
        f32::from_bits(sign | mant)
    }

    /// Random quiet NaN with a random (non-zero) payload.
    pub fn quiet_nan(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let payload = self.next_u32() & 0x003F_FFFF; // below the quiet bit
        f32::from_bits(sign | 0x7FC0_0000 | payload)
    }

    /// Random signalling NaN (quiet bit clear, payload non-zero).
    pub fn signalling_nan(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let payload = 1 + (self.next_u32() & 0x003F_FFFE);
        f32::from_bits(sign | 0x7F80_0000 | payload)
    }

    pub fn signed_inf(&mut self) -> f32 {
        if self.bool() {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        }
    }

    pub fn vec2_unit(&mut self) -> Vec2 {
        Vec2::new(self.signed_unit(), self.signed_unit())
    }

    pub fn vec2_any(&mut self) -> Vec2 {
        Vec2::new(self.any_f32(), self.any_f32())
    }

    pub fn vec2_wide(&mut self) -> Vec2 {
        Vec2::new(self.wide_normal(), self.wide_normal())
    }

    pub fn vec2_small_int(&mut self, mag: i32) -> Vec2 {
        Vec2::new(self.small_int(mag), self.small_int(mag))
    }
}

/// Component-wise helpers used to build the geometric shapes.
pub fn add2(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x + b.x, a.y + b.y)
}
pub fn sub2(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}
pub fn scale2(a: Vec2, k: f32) -> Vec2 {
    Vec2::new(a.x * k, a.y * k)
}
pub fn perp2(a: Vec2) -> Vec2 {
    Vec2::new(-a.y, a.x)
}

/// The eight floats of the argument list, addressed 0..8, so tests can splice
/// special values into random positions.
pub fn from_components(c: [f32; 8]) -> (Vec2, Vec2, Vec2, Vec2) {
    (
        Vec2::new(c[0], c[1]),
        Vec2::new(c[2], c[3]),
        Vec2::new(c[4], c[5]),
        Vec2::new(c[6], c[7]),
    )
}
