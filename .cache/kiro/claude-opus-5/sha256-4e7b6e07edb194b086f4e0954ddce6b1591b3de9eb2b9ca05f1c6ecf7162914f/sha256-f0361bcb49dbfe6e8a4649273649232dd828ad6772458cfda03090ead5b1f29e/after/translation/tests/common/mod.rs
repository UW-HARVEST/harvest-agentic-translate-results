//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls the exported
//! `gaussian_kernel` symbol on each. The Rust implementation is never called
//! directly — always through the `.so`, so the `#[no_mangle]` export wrapper
//! is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type GaussianKernelFn = unsafe extern "C" fn(*mut f32, c_int, f32);

/// Sentinel poison written into every buffer slot before a call, so that
/// "not written" is distinguishable from "written with 0.0".
pub const POISON_BITS: u32 = 0xDEAD_BEEF;

pub fn poison() -> f32 {
    f32::from_bits(POISON_BITS)
}

/// Number of guard elements kept past the region the C is allowed to write,
/// so we can also assert the C and Rust write *the same amount*.
pub const GUARD: usize = 8;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Allow pointing the harness at a differently-compiled C build (e.g. a
    // different -O level) without touching c_src/.
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_C_SO does not exist: {}", p.display());
        return p;
    }
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib sits one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    let mut candidates = vec![
        profile_dir.join("libgaussian_kernel_lib.so"),
        workspace_root()
            .join("translation/target/release/libgaussian_kernel_lib.so"),
        workspace_root()
            .join("translation/target/debug/libgaussian_kernel_lib.so"),
    ];
    candidates.retain(|p| p.exists());
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "libgaussian_kernel_lib.so not found near {}; run `cargo build` first",
            profile_dir.display()
        )
    })
}

struct Libs {
    c: Library,
    rs: Library,
    c_path: PathBuf,
    rs_path: PathBuf,
}

// SAFETY: the loaded libraries are leaked for the whole process lifetime and
// the only symbol used is a pure, thread-safe leaf function.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        // SAFETY: loading a plain C shared library with no initializers of
        // interest.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rs = unsafe { Library::new(&rs_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));
        Libs { c, rs, c_path, rs_path }
    })
}

pub fn c_so_path() -> &'static PathBuf {
    &libs().c_path
}

pub fn rust_so_path() -> &'static PathBuf {
    &libs().rs_path
}

pub fn c_gaussian_kernel() -> GaussianKernelFn {
    let l = libs();
    // SAFETY: signature matches c_src/include/lib.h exactly.
    let sym: Symbol<GaussianKernelFn> = unsafe { l.c.get(b"gaussian_kernel\0") }
        .expect("C .so does not export gaussian_kernel");
    *sym
}

pub fn rust_gaussian_kernel() -> GaussianKernelFn {
    let l = libs();
    // SAFETY: signature matches the #[no_mangle] extern "C" wrapper.
    let sym: Symbol<GaussianKernelFn> = unsafe { l.rs.get(b"gaussian_kernel\0") }
        .expect("Rust .so does not export gaussian_kernel");
    *sym
}

/// How many floats the C write loop touches: `2 * (size / 2) + 1` when
/// `size / 2 >= 0`, otherwise none.
pub fn written_len(size: c_int) -> usize {
    let hsize = size / 2;
    if hsize < 0 {
        0
    } else {
        (2 * hsize as i64 + 1) as usize
    }
}

/// Total buffer we allocate for a given `size`: everything the C may write,
/// everything the normalize loop may touch, plus guard slots.
pub fn buffer_len(size: c_int) -> usize {
    let touched = written_len(size).max(if size > 0 { size as usize } else { 0 });
    touched + GUARD
}

/// Run one differential call. Returns `(c_buffer, rust_buffer)` as raw bit
/// patterns so that `-0.0` vs `+0.0` and NaN payloads are compared exactly.
pub fn run_both(size: c_int, radius: f32, fill: &[f32]) -> (Vec<u32>, Vec<u32>) {
    let mut cbuf: Vec<f32> = fill.to_vec();
    let mut rbuf: Vec<f32> = fill.to_vec();

    let cf = c_gaussian_kernel();
    let rf = rust_gaussian_kernel();

    // SAFETY: the buffer is sized by `buffer_len`, which already accounts for
    // the C's one-past-the-end write on even `size`, plus GUARD slack.
    unsafe {
        cf(cbuf.as_mut_ptr(), size, radius);
        rf(rbuf.as_mut_ptr(), size, radius);
    }

    (
        cbuf.iter().map(|f| f.to_bits()).collect(),
        rbuf.iter().map(|f| f.to_bits()).collect(),
    )
}

/// Differential call with a poison-filled buffer of the standard length.
pub fn run_both_poisoned(size: c_int, radius: f32) -> (Vec<u32>, Vec<u32>) {
    let fill = vec![poison(); buffer_len(size)];
    run_both(size, radius, &fill)
}

pub fn assert_same(size: c_int, radius: f32, c: &[u32], r: &[u32], ctx: &str) {
    if c == r {
        return;
    }
    let mut msg = format!(
        "DIVERGENCE [{ctx}]  size={size} ({:#010x})  radius={radius:e} (bits {:#010x})\n\
         written_len={}  buffer_len={}\n",
        size as u32,
        radius.to_bits(),
        written_len(size),
        c.len()
    );
    for (i, (cv, rv)) in c.iter().zip(r.iter()).enumerate() {
        if cv != rv {
            msg.push_str(&format!(
                "  [{i}] C = {:#010x} ({:e})   RUST = {:#010x} ({:e})\n",
                cv,
                f32::from_bits(*cv),
                rv,
                f32::from_bits(*rv)
            ));
        }
    }
    panic!("{msg}");
}

pub fn check(size: c_int, radius: f32, ctx: &str) {
    let (c, r) = run_both_poisoned(size, radius);
    assert_same(size, radius, &c, &r, ctx);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in `[lo, hi]` inclusive.
    pub fn int_in(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    /// Log-uniform `f32` in `[lo, hi]`, both positive.
    pub fn logunif_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let l = (lo as f64).ln();
        let h = (hi as f64).ln();
        ((l + (h - l) * self.unit()).exp()) as f32
    }

    /// An arbitrary finite (non-NaN, non-inf) `f32` from a random bit pattern.
    pub fn finite_f32(&mut self) -> f32 {
        loop {
            let v = f32::from_bits(self.next_u32());
            if v.is_finite() {
                return v;
            }
        }
    }
}
