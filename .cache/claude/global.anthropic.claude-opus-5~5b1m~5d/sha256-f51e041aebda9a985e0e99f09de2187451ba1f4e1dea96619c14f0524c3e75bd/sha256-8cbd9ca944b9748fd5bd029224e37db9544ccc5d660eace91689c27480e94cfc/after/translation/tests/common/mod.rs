//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls `gaussian_kernel` across the FFI boundary in both.
//!
//! Nothing here calls a Rust function directly -- the Rust implementation is
//! only ever reached through the exported `#[no_mangle]` symbol in the cdylib,
//! exactly as an external C consumer would reach it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type GaussianKernelFn = unsafe extern "C" fn(*mut f32, i32, f32);

/// Canary value used to fill the slack region of the buffers so that any write
/// past the region the implementation is supposed to touch is detected.
pub const GUARD: u32 = 0xDEAD_BEEF;
/// Extra slots after `size` so the (deliberate) one-past-the-end store made for
/// even `size` is captured and compared instead of corrupting memory.
pub const SLACK: usize = 16;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let is_so = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false);
            if is_so {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}. Build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libgaussian_kernel_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {}. Build it with: cargo build --release",
        base.display()
    )
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_kernel: GaussianKernelFn,
    pub rust_kernel: GaussianKernelFn,
}

// The raw fn pointers are plain code addresses; the owning `Library` handles are
// kept alive for the whole process in a `OnceLock`, so sharing is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", c_path.display()));
        let rust = Library::new(&r_path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", r_path.display()));
        let c_sym: Symbol<GaussianKernelFn> = c
            .get(b"gaussian_kernel\0")
            .expect("C .so does not export gaussian_kernel");
        let r_sym: Symbol<GaussianKernelFn> = rust
            .get(b"gaussian_kernel\0")
            .expect("Rust .so does not export gaussian_kernel");
        let c_kernel = *c_sym;
        let rust_kernel = *r_sym;
        Libs {
            _c: c,
            _rust: rust,
            c_kernel,
            rust_kernel,
        }
    })
}

/// Deterministic PRNG (xorshift64*) so every run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    /// Uniform f32 in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform f32 in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Any f32 bit pattern (includes NaN, inf, denormals, negative zero).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// Number of `f32` slots the C is known to write for a given `size`
/// (`2 * (size / 2) + 1`, or none when that count is <= 0).
pub fn written_slots(size: i32) -> usize {
    let hsize = size / 2;
    let n = 2i64 * hsize as i64 + 1;
    if n <= 0 {
        0
    } else {
        n as usize
    }
}

/// Total buffer length used for a given `size`: enough for everything the C may
/// touch plus guard slack.
pub fn buf_len(size: i32) -> usize {
    let need = written_slots(size).max(size.max(0) as usize);
    need + SLACK
}

/// Bit-exact comparison of two buffers, reporting the first differing slot.
pub fn assert_bits_eq(c: &[f32], r: &[f32], ctx: &str) {
    assert_eq!(c.len(), r.len(), "{ctx}: buffer length mismatch");
    for i in 0..c.len() {
        let (cb, rb) = (c[i].to_bits(), r[i].to_bits());
        if cb != rb {
            panic!(
                "{ctx}: divergence at index {i}\n  C    = {:e} (bits 0x{:08X})\n  Rust = {:e} \
                 (bits 0x{:08X})\n  C buf    = {:?}\n  Rust buf = {:?}",
                c[i],
                cb,
                r[i],
                rb,
                &c[..c.len().min(40)],
                &r[..r.len().min(40)],
            );
        }
    }
}

/// Run one differential case: identical pre-filled buffers, call C then Rust
/// through their exported symbols, compare every slot bit-for-bit.
pub fn diff_case(size: i32, radius: f32, prefill: &[f32]) {
    let l = libs();
    let mut cbuf = prefill.to_vec();
    let mut rbuf = prefill.to_vec();
    unsafe {
        (l.c_kernel)(cbuf.as_mut_ptr(), size, radius);
        (l.rust_kernel)(rbuf.as_mut_ptr(), size, radius);
    }
    assert_bits_eq(
        &cbuf,
        &rbuf,
        &format!(
            "size={size} radius={radius:e} (radius bits 0x{:08X}) buflen={}",
            radius.to_bits(),
            prefill.len()
        ),
    );
}

/// `diff_case` with a fresh guard-filled buffer sized for `size`.
pub fn diff(size: i32, radius: f32) {
    let prefill = vec![f32::from_bits(GUARD); buf_len(size)];
    diff_case(size, radius, &prefill);
}

/// `diff` at an offset into a larger buffer, exercising the pointer-arithmetic
/// path with a non-zero base offset.
pub fn diff_at_offset(size: i32, radius: f32, offset: usize) {
    let l = libs();
    let total = buf_len(size) + offset;
    let mut cbuf = vec![f32::from_bits(GUARD); total];
    let mut rbuf = cbuf.clone();
    unsafe {
        (l.c_kernel)(cbuf.as_mut_ptr().add(offset), size, radius);
        (l.rust_kernel)(rbuf.as_mut_ptr().add(offset), size, radius);
    }
    assert_bits_eq(
        &cbuf,
        &rbuf,
        &format!("offset={offset} size={size} radius={radius:e}"),
    );
}
