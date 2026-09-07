//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `normalize` only
//! through the exported C ABI symbol — never by calling Rust code directly.
//! That way the `#[unsafe(no_mangle)] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::PathBuf;

use libloading::{Library, Symbol};

pub type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

/// Path to the C `.so`. Overridable with `C_SO`; otherwise the first `*.so`
/// found in `../c_src/build` (the file name is derived from the parent
/// directory name by `CMakeLists.txt`, so it must not be hard-coded).
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = PathBuf::from("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found
        .into_iter()
        .next()
        .expect("no *.so in ../c_src/build — build the C library first")
}

/// Path to the Rust `.so`. Overridable with `RUST_SO`.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    for cand in ["target/release/libnormalize_lib.so", "target/debug/libnormalize_lib.so"] {
        let p = PathBuf::from(cand);
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust cdylib found — run `cargo build --release`");
}

pub struct Libs {
    _c_lib: Library,
    _r_lib: Library,
    pub c: NormalizeFn,
    pub r: NormalizeFn,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

impl Libs {
    pub fn load() -> Libs {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let r_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
            let cs: Symbol<NormalizeFn> = c_lib
                .get(b"normalize\0")
                .expect("C .so does not export `normalize`");
            let rs: Symbol<NormalizeFn> = r_lib
                .get(b"normalize\0")
                .expect("Rust .so does not export `normalize`");
            let c = *cs;
            let r = *rs;
            Libs { _c_lib: c_lib, _r_lib: r_lib, c, r, c_path, r_path }
        }
    }
}

pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(Libs::load)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

impl Rng {
    pub fn new(stream: u64) -> Rng {
        Rng(SEED ^ stream.wrapping_mul(0xD1B5_4A32_D192_ED03))
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Uniform in `[-1, 1)`.
    pub fn unit(&mut self) -> f32 {
        let x = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        x * 2.0 - 1.0
    }
    /// Arbitrary bit pattern (may be NaN / inf / subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Random finite value with exponent chosen from `lo..=hi` (base-2).
    pub fn scaled(&mut self, lo: i32, hi: i32) -> f32 {
        let e = lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i32;
        let m = self.unit();
        let v = m * (2.0f32).powi(e.clamp(-149, 127));
        if v.is_finite() { v } else { m }
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub fn fmt_bits(v: &[f32]) -> String {
    let mut s = String::new();
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:08x}({:e})", x.to_bits(), x));
        if i >= 15 {
            s.push_str(" …");
            break;
        }
    }
    s
}

/// Byte pattern used to fill destination buffers and canaries so that "not
/// written" is distinguishable from "written as zero".
pub const CANARY: u8 = 0xA5;
/// Number of canary floats placed before and after every destination window.
pub const PAD: usize = 4;

/// A padded buffer: `PAD` canary floats, `n` payload floats, `PAD` canary
/// floats. `offset` shifts the payload start to control alignment.
pub struct Padded {
    pub raw: Vec<f32>,
    pub off: usize,
    pub n: usize,
}

impl Padded {
    /// `misalign` extra floats of shift (0..=3) applied on top of `PAD`.
    pub fn new(n: usize, misalign: usize) -> Padded {
        let off = PAD + misalign;
        let raw = vec![f32::from_bits(u32::from_ne_bytes([CANARY; 4])); off + n + PAD];
        Padded { raw, off, n }
    }
    pub fn filled(src: &[f32], misalign: usize) -> Padded {
        let mut p = Padded::new(src.len(), misalign);
        p.payload_mut().copy_from_slice(src);
        p
    }
    pub fn ptr(&mut self) -> *mut f32 {
        unsafe { self.raw.as_mut_ptr().add(self.off) }
    }
    pub fn cptr(&self) -> *const f32 {
        unsafe { self.raw.as_ptr().add(self.off) }
    }
    pub fn payload(&self) -> &[f32] {
        &self.raw[self.off..self.off + self.n]
    }
    pub fn payload_mut(&mut self) -> &mut [f32] {
        let (o, n) = (self.off, self.n);
        &mut self.raw[o..o + n]
    }
    /// Everything outside the payload window, as raw bits.
    pub fn canaries(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.raw[..self.off].iter().map(|x| x.to_bits()).collect();
        v.extend(self.raw[self.off + self.n..].iter().map(|x| x.to_bits()));
        v
    }
}

/// Core differential call: distinct (non-overlapping) `dest` and `src`.
///
/// Returns the C payload so callers can inspect it if they want.
pub fn diff_disjoint(
    label: &str,
    src: &[f32],
    size: c_int,
    dest_misalign: usize,
    src_misalign: usize,
) -> Vec<f32> {
    let l = libs();

    let src_c = Padded::filled(src, src_misalign);
    let src_r = Padded::filled(src, src_misalign);
    let mut dst_c = Padded::new(src.len(), dest_misalign);
    let mut dst_r = Padded::new(src.len(), dest_misalign);

    unsafe { (l.c)(dst_c.ptr(), src_c.cptr(), size) };
    unsafe { (l.r)(dst_r.ptr(), src_r.cptr(), size) };

    assert_eq!(
        bits(dst_c.payload()),
        bits(dst_r.payload()),
        "{label}: dest payload mismatch (size={size}, dest_misalign={dest_misalign}, \
         src_misalign={src_misalign})\n  in : {}\n  C  : {}\n  Rust: {}",
        fmt_bits(src),
        fmt_bits(dst_c.payload()),
        fmt_bits(dst_r.payload()),
    );
    assert_eq!(
        dst_c.canaries(),
        dst_r.canaries(),
        "{label}: dest canary mismatch — one side wrote out of bounds (size={size})"
    );
    assert_eq!(
        bits(src_c.payload()),
        bits(src_r.payload()),
        "{label}: src buffer mismatch — one side mutated its input (size={size})"
    );
    assert_eq!(
        src_c.canaries(),
        src_r.canaries(),
        "{label}: src canary mismatch (size={size})"
    );
    dst_c.payload().to_vec()
}

/// Core differential call: fully aliased, `dest == src` (in place).
pub fn diff_inplace(label: &str, src: &[f32], size: c_int, misalign: usize) -> Vec<f32> {
    let l = libs();

    let mut b_c = Padded::filled(src, misalign);
    let mut b_r = Padded::filled(src, misalign);

    unsafe {
        let p = b_c.ptr();
        (l.c)(p, p as *const f32, size)
    };
    unsafe {
        let p = b_r.ptr();
        (l.r)(p, p as *const f32, size)
    };

    assert_eq!(
        bits(b_c.payload()),
        bits(b_r.payload()),
        "{label}: in-place payload mismatch (size={size}, misalign={misalign})\n  \
         in : {}\n  C  : {}\n  Rust: {}",
        fmt_bits(src),
        fmt_bits(b_c.payload()),
        fmt_bits(b_r.payload()),
    );
    assert_eq!(
        b_c.canaries(),
        b_r.canaries(),
        "{label}: in-place canary mismatch (size={size})"
    );
    b_c.payload().to_vec()
}

/// Overlapping-window differential call.
///
/// One buffer of `n + shift` floats; `dest` starts at `dest_off`, `src` starts
/// at `src_off`, both windows are `n` long, so they overlap. C has no
/// `restrict`, so the sequential read-after-write cascade is observable
/// behaviour that both sides must reproduce identically.
pub fn diff_overlap(label: &str, buf: &[f32], n: usize, dest_off: usize, src_off: usize) {
    let l = libs();
    let need = dest_off.max(src_off) + n;
    assert!(buf.len() >= need, "{label}: buffer too small");

    let mut b_c = Padded::filled(buf, 0);
    let mut b_r = Padded::filled(buf, 0);

    unsafe {
        let base = b_c.ptr();
        (l.c)(base.add(dest_off), base.add(src_off) as *const f32, n as c_int)
    };
    unsafe {
        let base = b_r.ptr();
        (l.r)(base.add(dest_off), base.add(src_off) as *const f32, n as c_int)
    };

    assert_eq!(
        bits(b_c.payload()),
        bits(b_r.payload()),
        "{label}: overlap mismatch (n={n}, dest_off={dest_off}, src_off={src_off})\n  \
         in : {}\n  C  : {}\n  Rust: {}",
        fmt_bits(buf),
        fmt_bits(b_c.payload()),
        fmt_bits(b_r.payload()),
    );
    assert_eq!(b_c.canaries(), b_r.canaries(), "{label}: overlap canary mismatch");
}
