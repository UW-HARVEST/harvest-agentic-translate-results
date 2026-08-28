//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported `premultiply` symbol. The Rust crate is
//! never linked directly, so these tests exercise the `#[no_mangle] extern "C"`
//! export wrapper exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `cp_image_t` from `c_src/include/lib.h`.
///
/// `pix` is typed `*mut u8` rather than `*mut cp_pixel_t` because the C code
/// immediately casts it to `uint8_t *` (line 7) and only ever does byte access.
#[repr(C)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut u8,
}

/// `cp_pixel_t` from `c_src/include/lib.h` — used only to assert the layout the
/// C code assumes (`sizeof == 4`, `alignof == 1`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub type PremultiplyFn = unsafe extern "C" fn(*mut CpImage);

pub struct Libs {
    pub c: PremultiplyFn,
    pub rust: PremultiplyFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    // Keep the libraries alive for the process lifetime.
    _c_lib: Library,
    _rust_lib: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — CMakeLists.txt derives the target
/// name from the *parent* directory, so the file name is not fixed. Glob it.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}.\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// `target/<profile>/libpremultiply_lib.so`, derived from the running test
/// binary's own location (`target/<profile>/deps/<test>-<hash>`).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    const SO: &str = "libpremultiply_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    // deps/ -> <profile>/
    if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
        let cand = profile_dir.join(SO);
        if cand.is_file() {
            return cand;
        }
    }
    // Fallback: scan target/*/
    let target = manifest_dir().join("target");
    if let Ok(rd) = std::fs::read_dir(&target) {
        for e in rd.flatten() {
            let cand = e.path().join(SO);
            if cand.is_file() {
                return cand;
            }
        }
    }
    panic!(
        "could not locate {SO}; build it with `cargo build --release --offline` \
         (searched next to {} and under {})",
        exe.display(),
        target.display()
    );
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // Layout sanity: the C code hard-codes sizeof(cp_pixel_t) == 4.
        assert_eq!(std::mem::size_of::<CpPixel>(), 4);
        assert_eq!(std::mem::align_of::<CpPixel>(), 1);
        assert_eq!(std::mem::size_of::<c_int>(), 4);

        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c_sym: Symbol<PremultiplyFn> = c_lib
                .get(b"premultiply\0")
                .unwrap_or_else(|e| panic!("C premultiply: {e}"));
            let rust_sym: Symbol<PremultiplyFn> = rust_lib
                .get(b"premultiply\0")
                .unwrap_or_else(|e| panic!("Rust premultiply: {e}"));
            let c = *c_sym;
            let rust = *rust_sym;
            Libs { c, rust, c_path, rust_path, _c_lib: c_lib, _rust_lib: rust_lib }
        }
    })
}

// ---------------------------------------------------------------------------
// Loop-bound model, transcribed from c_src/src/lib.c lines 6 and 8.
// Used ONLY to decide whether a (w, h) pair is safe to execute against a
// buffer of a given size; correctness itself is always decided by comparing
// the two libraries' observable output.
// ---------------------------------------------------------------------------

/// Returns `(stride, limit, pixel_iterations)`.
pub fn semantics(w: i32, h: i32) -> (i32, i32, usize) {
    let stride = w.wrapping_mul(4); // int stride = w * sizeof(cp_pixel_t);
    let limit = stride.wrapping_mul(h); // (int)stride * h
    let iters = if limit > 0 { (limit / 4) as usize } else { 0 };
    (stride, limit, iters)
}

pub fn predicted_bytes(w: i32, h: i32) -> usize {
    semantics(w, h).2 * 4
}

// ---------------------------------------------------------------------------
// Guarded buffer
// ---------------------------------------------------------------------------

pub const CANARY: usize = 64;
pub const CANARY_BYTE: u8 = 0xA5;

/// A pixel payload surrounded by canary bytes, so that any write outside the
/// intended extent is detected.
pub struct Guarded {
    mem: Vec<u8>,
    off: usize,
    len: usize,
}

impl Guarded {
    /// `misalign` is the desired `pix as usize % 4`.
    pub fn new(payload: &[u8], misalign: usize) -> Guarded {
        let len = payload.len();
        let total = CANARY * 2 + len + 8;
        let mut mem = vec![CANARY_BYTE; total];
        let base = mem.as_ptr() as usize;
        let mut off = CANARY;
        while (base + off) % 4 != misalign % 4 {
            off += 1;
        }
        assert!(off + len + CANARY <= total);
        mem[off..off + len].copy_from_slice(payload);
        Guarded { mem, off, len }
    }

    pub fn ptr(&mut self) -> *mut u8 {
        unsafe { self.mem.as_mut_ptr().add(self.off) }
    }

    pub fn misalign(&self) -> usize {
        (self.mem.as_ptr() as usize + self.off) % 4
    }

    pub fn payload(&self) -> &[u8] {
        &self.mem[self.off..self.off + self.len]
    }

    /// Every canary byte before and after the payload must be pristine.
    pub fn assert_canaries(&self, who: &str, ctx: &str) {
        for (i, &b) in self.mem[..self.off].iter().enumerate() {
            assert_eq!(
                b, CANARY_BYTE,
                "{who} wrote {b:#04x} BEFORE the payload at -{} ({ctx})",
                self.off - i
            );
        }
        for (i, &b) in self.mem[self.off + self.len..].iter().enumerate() {
            assert_eq!(
                b, CANARY_BYTE,
                "{who} wrote {b:#04x} AFTER the payload at +{i} ({ctx})"
            );
        }
    }
}

/// Outcome of running one implementation over one input.
pub struct Outcome {
    pub payload: Vec<u8>,
}

/// Run `f` (one implementation) on a freshly seeded guarded buffer.
fn run_one(
    f: PremultiplyFn,
    w: i32,
    h: i32,
    payload: &[u8],
    misalign: usize,
    calls: usize,
    who: &str,
    ctx: &str,
) -> Outcome {
    let mut g = Guarded::new(payload, misalign);
    assert_eq!(g.misalign(), misalign % 4, "failed to achieve misalignment");
    let mut img = CpImage { w, h, pix: g.ptr() };
    for _ in 0..calls {
        unsafe { f(&mut img as *mut CpImage) };
    }
    g.assert_canaries(who, ctx);
    Outcome { payload: g.payload().to_vec() }
}

/// The core differential assertion: run the C `.so` and the Rust `.so` over
/// identical inputs and require byte-identical output plus intact canaries.
///
/// Returns the (shared) resulting payload so callers can make extra assertions.
pub fn assert_same(w: i32, h: i32, payload: &[u8], misalign: usize, calls: usize) -> Vec<u8> {
    let need = predicted_bytes(w, h);
    assert!(
        need <= payload.len(),
        "test bug: (w={w}, h={h}) would touch {need} bytes but the payload is \
         only {} bytes",
        payload.len()
    );

    let l = libs();
    let ctx = format!(
        "w={w} h={h} len={} misalign={misalign} calls={calls} \
         (stride={}, limit={}, iters={})",
        payload.len(),
        semantics(w, h).0,
        semantics(w, h).1,
        semantics(w, h).2
    );

    let c = run_one(l.c, w, h, payload, misalign, calls, "C", &ctx);
    let r = run_one(l.rust, w, h, payload, misalign, calls, "Rust", &ctx);

    if c.payload != r.payload {
        let mut diffs = Vec::new();
        for (i, (&a, &b)) in c.payload.iter().zip(r.payload.iter()).enumerate() {
            if a != b {
                let px = i / 4;
                let ch = ["r", "g", "b", "a"][i % 4];
                diffs.push(format!(
                    "  byte {i} (pixel {px}, channel {ch}): input={:#04x} C={a:#04x} Rust={b:#04x}",
                    payload[i]
                ));
                if diffs.len() >= 12 {
                    diffs.push("  ...".into());
                    break;
                }
            }
        }
        panic!(
            "C/Rust divergence for {ctx}\n{}\n(total differing bytes: {})",
            diffs.join("\n"),
            c.payload
                .iter()
                .zip(r.payload.iter())
                .filter(|(a, b)| a != b)
                .count()
        );
    }
    c.payload
}

/// Convenience: build a payload of `n` pixels and diff it, aligned, one call.
pub fn assert_same_simple(w: i32, h: i32, payload: &[u8]) -> Vec<u8> {
    assert_same(w, h, payload, 0, 1)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as i32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u64() >> 32) as u32) % n
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u32) as i32
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}
