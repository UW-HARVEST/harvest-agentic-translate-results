//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; the Rust
//! side is *never* called directly, so the `#[no_mangle] extern "C"` wrapper is
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

pub type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

// ---------------------------------------------------------------------------
// Locating and building the two shared objects
// ---------------------------------------------------------------------------

/// Working-directory root that holds `c_src/` and `translation/`.
pub fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    if !build.is_dir() {
        panic!(
            "C build dir {} missing; run:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        );
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("read c_src/build")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    match found.len() {
        0 => panic!("no lib*.so found in {}", build.display()),
        _ => found.remove(0),
    }
}

/// The Rust `cdylib`. Built by `cargo build` before/alongside the test binary,
/// but a test binary can be run standalone, so build it on demand if absent.
fn find_rust_so() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the profile the tests were built with, then fall back.
    let candidates = ["release", "debug"];
    for profile in candidates {
        let p = root.join("target").join(profile).join("libtfm_lib.so");
        if p.is_file() {
            return p;
        }
    }
    // Build it.
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--quiet"])
        .current_dir(&root)
        .status()
        .expect("spawn cargo build");
    assert!(status.success(), "cargo build --release failed");
    let p = root.join("target").join("release").join("libtfm_lib.so");
    assert!(p.is_file(), "{} still missing after build", p.display());
    p
}

pub fn c_so_path() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(find_c_so)
}

pub fn rust_so_path() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(find_rust_so)
}

/// The two libraries, kept alive for the whole process.
struct Libs {
    c: Library,
    rust: Library,
}

// SAFETY: the loaded libraries are leaf numeric code with no thread-local or
// global state; sharing the handles across test threads is fine.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C .so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust .so");
        Libs { c, rust }
    })
}

/// `tfm` from the **C** shared object, resolved by symbol name.
pub fn c_tfm() -> TfmFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| unsafe {
        let s: Symbol<TfmFn> = libs().c.get(b"tfm\0").expect("C .so exports tfm");
        *s.into_raw() as usize
    });
    // SAFETY: address came from dlsym on a library that outlives the process.
    unsafe { std::mem::transmute::<usize, TfmFn>(addr) }
}

/// `tfm` from the **Rust** shared object, resolved by symbol name.
pub fn rust_tfm() -> TfmFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| unsafe {
        let s: Symbol<TfmFn> = libs().rust.get(b"tfm\0").expect("Rust .so exports tfm");
        *s.into_raw() as usize
    });
    // SAFETY: as above.
    unsafe { std::mem::transmute::<usize, TfmFn>(addr) }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

pub const POISON_BITS: u32 = 0xDEAD_BEEF;

pub fn poison(n: usize) -> Vec<f32> {
    vec![f32::from_bits(POISON_BITS); n]
}

fn fmt_bits(v: &[f32]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("0x{:08x}({})", x.to_bits(), x));
    }
    s.push(']');
    s
}

/// Run one `count`-element transform through both `.so`s on freshly poisoned
/// destination buffers and assert the raw bits agree.
///
/// `dest_len` lets a caller deliberately over-allocate.
pub fn diff_call(label: &str, src: &[f32], count: c_int) {
    let dest_len = if count > 0 { 2 * count as usize } else { 4 };
    let mut dc = poison(dest_len);
    let mut dr = poison(dest_len);

    unsafe {
        (c_tfm())(dc.as_mut_ptr(), src.as_ptr(), count);
        (rust_tfm())(dr.as_mut_ptr(), src.as_ptr(), count);
    }

    assert_bits_eq(label, src, count, &dc, &dr);
}

pub fn assert_bits_eq(label: &str, src: &[f32], count: c_int, dc: &[f32], dr: &[f32]) {
    if dc.iter().map(|x| x.to_bits()).eq(dr.iter().map(|x| x.to_bits())) {
        return;
    }
    let first = dc
        .iter()
        .zip(dr.iter())
        .position(|(a, b)| a.to_bits() != b.to_bits())
        .unwrap();
    panic!(
        "{label}: C/Rust divergence at dest[{first}]\n  \
         count = {count}\n  \
         src   = {}\n  \
         C     = {}\n  \
         Rust  = {}",
        fmt_bits(src),
        fmt_bits(dc),
        fmt_bits(dr),
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
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

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[-1, 1)`, then scaled by a random power of two in
    /// `2^-30 .. 2^30`, so magnitudes span the whole useful float range.
    pub fn normal_f32(&mut self) -> f32 {
        let m = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        let sign = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        let exp = self.below(61) as i32 - 30;
        sign * m * 2f32.powi(exp)
    }

    /// Small-magnitude value, so `sqd` stays comfortably finite.
    pub fn tame_f32(&mut self) -> f32 {
        let m = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        let sign = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        sign * m * 16.0
    }

    /// Any of the 2^32 bit patterns, NaNs and infinities included.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    pub fn subnormal_f32(&mut self) -> f32 {
        let bits = (self.next_u32() & 0x007F_FFFF) | ((self.next_u32() & 1) << 31);
        f32::from_bits(bits)
    }

    pub fn qnan_f32(&mut self) -> f32 {
        // exponent all ones, quiet bit set, random non-zero payload, random sign
        let payload = (self.next_u32() & 0x003F_FFFF) | 1;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | 0x7F80_0000 | 0x0040_0000 | payload)
    }

    pub fn snan_f32(&mut self) -> f32 {
        let payload = (self.next_u32() & 0x003F_FFFF) | 1;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | 0x7F80_0000 | payload) // quiet bit clear
    }

    pub fn signed_zero(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            0.0
        } else {
            -0.0
        }
    }

    pub fn inf(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        }
    }

    /// Huge finite magnitude near `FLT_MAX`.
    pub fn huge_f32(&mut self) -> f32 {
        let m = 1.0 + (self.next_u32() >> 9) as f32 / (1u32 << 23) as f32; // [1,2)
        let sign = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        let exp = 100 + self.below(28) as i32; // 2^100 .. 2^127
        sign * m * 2f32.powi(exp)
    }
}

/// How many randomized inputs each `CONFIGS.md` row uses.
pub const ITERS: usize = 4000;
/// Element count used by the "many" rows.
pub const MANY: usize = 64;
