//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `rgb_to_hsv`
//! only through the dynamic symbol. The Rust implementation is never called
//! directly, so the `#[no_mangle] extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use std::path::PathBuf;

pub type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

/// One loaded library plus its resolved `rgb_to_hsv` symbol.
pub struct Impl {
    pub name: &'static str,
    // Keep the library alive for the whole process; the fn pointer borrows it.
    _lib: &'static libloading::Library,
    pub f: RgbToHsvFn,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        dir.display(),
        candidates
    );
    candidates.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up to the profile dir.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let direct = dir.join("librgb_to_hsv_lib.so");
    if direct.exists() {
        return direct;
    }
    // Fall back to either profile dir under target/.
    for profile in ["release", "debug"] {
        let p = workspace_root()
            .join("translation/target")
            .join(profile)
            .join("librgb_to_hsv_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "librgb_to_hsv_lib.so not found near {} — run `cargo build` (and `cargo build --release`) first",
        dir.display()
    );
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { libloading::Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    let lib: &'static libloading::Library = Box::leak(Box::new(lib));
    let sym: libloading::Symbol<RgbToHsvFn> = unsafe { lib.get(b"rgb_to_hsv\0") }
        .unwrap_or_else(|e| panic!("dlsym rgb_to_hsv in {} failed: {e}", path.display()));
    let f = *sym;
    Impl { name, _lib: lib, f }
}

/// The C reference implementation, loaded from `c_src/build/*.so`.
pub fn c_impl() -> Impl {
    load("C", find_c_so())
}

/// The Rust translation, loaded from `librgb_to_hsv_lib.so` (never called directly).
pub fn rust_impl() -> Impl {
    load("Rust", find_rust_so())
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> Pair {
    Pair {
        c: c_impl(),
        rust: rust_impl(),
    }
}

/// Sentinel written into `dest` before each call so we can tell "written" from
/// "untouched", and detect any write past index 2.
pub const CANARY: u32 = 0xDEAD_BEEF;

impl Pair {
    /// Run one triple through both libraries and return `(c_out, rust_out)` as raw bits.
    pub fn run(&self, src: [f32; 3]) -> ([u32; 3], [u32; 3]) {
        let mut dc = [f32::from_bits(CANARY); 3];
        let mut dr = [f32::from_bits(CANARY); 3];
        let sc = src;
        let sr = src;
        unsafe {
            (self.c.f)(dc.as_mut_ptr(), sc.as_ptr());
            (self.rust.f)(dr.as_mut_ptr(), sr.as_ptr());
        }
        (
            [dc[0].to_bits(), dc[1].to_bits(), dc[2].to_bits()],
            [dr[0].to_bits(), dr[1].to_bits(), dr[2].to_bits()],
        )
    }

    /// Assert byte-identical output for one triple.
    pub fn assert_same(&self, row: &str, src: [f32; 3]) {
        let (c, r) = self.run(src);
        if c != r {
            panic!(
                "[{row}] DIVERGENCE\n  input  r={} g={} b={}  (bits {:#010x} {:#010x} {:#010x})\n  \
                 C    -> {:#010x} {:#010x} {:#010x}  ({} {} {})\n  \
                 Rust -> {:#010x} {:#010x} {:#010x}  ({} {} {})",
                src[0],
                src[1],
                src[2],
                src[0].to_bits(),
                src[1].to_bits(),
                src[2].to_bits(),
                c[0],
                c[1],
                c[2],
                f32::from_bits(c[0]),
                f32::from_bits(c[1]),
                f32::from_bits(c[2]),
                r[0],
                r[1],
                r[2],
                f32::from_bits(r[0]),
                f32::from_bits(r[1]),
                f32::from_bits(r[2]),
            );
        }
    }

    /// Assert byte-identical output over a whole batch, reporting the row name.
    pub fn assert_batch(&self, row: &str, inputs: impl IntoIterator<Item = [f32; 3]>) {
        let mut n = 0usize;
        for src in inputs {
            self.assert_same(row, src);
            n += 1;
        }
        assert!(n > 0, "[{row}] generated no inputs");
        eprintln!("[{row}] {n} inputs matched bit-for-bit");
    }
}

/// Deterministic SplitMix64 PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234;

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
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A raw random f32 from a random bit pattern (may be NaN / inf / denormal).
    pub fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random NaN with random sign and random non-zero payload.
    pub fn nan(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let payload = self.next_u32() & 0x007F_FFFF;
        let payload = if payload == 0 { 1 } else { payload };
        f32::from_bits(sign | 0x7F80_0000 | payload)
    }
    /// A random subnormal (possibly signed zero when the payload lands on 0).
    pub fn subnormal(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let payload = self.next_u32() & 0x007F_FFFF;
        f32::from_bits(sign | payload)
    }
}

/// How many randomized inputs each property-style row uses.
pub const N: usize = 4000;
