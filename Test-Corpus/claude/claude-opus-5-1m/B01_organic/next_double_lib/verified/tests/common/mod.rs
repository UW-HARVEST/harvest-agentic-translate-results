// Each test binary uses a different subset of this harness.
#![allow(dead_code)]

//! Shared harness for the differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading`
//! and driven only through their exported `next_double` symbol. The Rust
//! implementation is never called directly as a Rust function, so the
//! `#[unsafe(no_mangle)] extern "C"` export wrapper is part of what is tested.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Byte-for-byte mirror of the C `cn_rnd_t`.
///
/// ```c
/// typedef struct cn_rnd_t { uint64_t state[2]; } cn_rnd_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CnRnd {
    pub state: [u64; 2],
}

impl CnRnd {
    pub fn new(s0: u64, s1: u64) -> Self {
        Self { state: [s0, s1] }
    }
}

pub type NextDoubleFn = unsafe extern "C" fn(*mut CnRnd) -> f64;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    // `_lib` must outlive `next_double`; keep it alive for the struct's lifetime.
    _lib: Library,
    next_double: NextDoubleFn,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        let sym: Symbol<NextDoubleFn> = unsafe { lib.get(b"next_double\0") }
            .unwrap_or_else(|e| panic!("`next_double` missing from {} .so: {e}", name));
        let next_double = *sym;
        Self {
            name,
            _lib: lib,
            next_double,
        }
    }

    /// Call the exported `next_double` and return the raw IEEE-754 bit pattern
    /// of the result together with the post-call state.
    ///
    /// The result is returned as `u64` bits (never as `f64` compared with `==`)
    /// so that `+0.0` / `-0.0` and every NaN encoding are distinguished.
    pub fn call(&self, rnd: &mut CnRnd) -> u64 {
        let v = unsafe { (self.next_double)(rnd as *mut CnRnd) };
        v.to_bits()
    }

    /// Call through a raw pointer (used by the misalignment / guard-byte tests).
    pub unsafe fn call_raw(&self, rnd: *mut CnRnd) -> u64 {
        unsafe { (self.next_double)(rnd) }.to_bits()
    }
}

/// Directory holding the crate manifest.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Env var to point the harness at a specific C `.so` (used to additionally
/// differential-test against an optimized `-O2` build of the same C source).
pub const C_SO_VAR: &str = "HARVEST_C_SO";

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var(C_SO_VAR) {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "{C_SO_VAR} points at {} which does not exist",
            p.display()
        );
        return p;
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared object not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Locate the Rust `cdylib` (`[lib] name = "next_double_lib"`).
///
/// Found relative to the test executable so that it works for any profile and
/// any `CARGO_TARGET_DIR`.
/// Env var to point the harness at a specific Rust `.so` (used to run the whole
/// suite against the release artifact as well as the debug one).
pub const RUST_SO_VAR: &str = "HARVEST_RUST_SO";

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var(RUST_SO_VAR) {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "{RUST_SO_VAR} points at {} which does not exist",
            p.display()
        );
        assert_not_stale(&p);
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>
    let deps = exe.parent().expect("deps dir");
    let mut candidates = vec![deps.to_path_buf()];
    if let Some(profile) = deps.parent() {
        candidates.push(profile.to_path_buf());
    }
    for dir in &candidates {
        let p = dir.join("libnext_double_lib.so");
        if p.exists() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "Rust cdylib `libnext_double_lib.so` not found in any of {:?}. \
         Run `cargo build` first.",
        candidates
    );
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact, so a
/// stale `.so` would silently be tested instead of the current source. Refuse to
/// run in that case rather than reporting a false pass.
fn assert_not_stale(so: &Path) {
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
    };
    let so_time = mtime(so);
    let src = manifest_dir().join("src/lib.rs");
    let src_time = mtime(&src);
    assert!(
        so_time >= src_time,
        "STALE Rust cdylib: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib-only lib target. Run:\n  \
         cargo build --no-default-features && cargo test --no-default-features",
        so.display(),
        src.display()
    );
}

/// The pair of implementations under differential test.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", &c_so_path()),
        rust: Impl::load("Rust", &rust_so_path()),
    }
}

impl Pair {
    /// Drive both implementations for `iters` sequential calls starting from
    /// `start`, asserting on every step that the returned bit pattern and the
    /// mutated state are identical.
    ///
    /// `ctx` is included in failure messages so a diverging row of
    /// `CONFIGS.md` is immediately identifiable.
    pub fn assert_stream_eq(&self, ctx: &str, start: CnRnd, iters: usize) {
        let mut cs = start;
        let mut rs = start;
        for i in 0..iters {
            let cb = self.c.call(&mut cs);
            let rb = self.rust.call(&mut rs);
            assert_eq!(
                cb, rb,
                "{ctx}: return bits differ at iteration {i}\n  start = {:#018x?}\n  \
                 C    = {cb:#018x} ({})\n  Rust = {rb:#018x} ({})",
                start.state,
                f64::from_bits(cb),
                f64::from_bits(rb)
            );
            assert_eq!(
                cs, rs,
                "{ctx}: post-call state differs at iteration {i}\n  start = {:#018x?}\n  \
                 C    = {:#018x?}\n  Rust = {:#018x?}",
                start.state, cs.state, rs.state
            );
        }
    }

    /// Single call variant of [`Pair::assert_stream_eq`].
    pub fn assert_call_eq(&self, ctx: &str, start: CnRnd) {
        self.assert_stream_eq(ctx, start, 1);
    }
}

/// Deterministic splitmix64 used to generate test inputs.
///
/// Fixed seed => reproducible failures. This is test-input generation only and
/// is deliberately *not* the algorithm under test.
pub struct SplitMix64(u64);

pub const FIXED_SEED: u64 = 0x2545_F491_4F6C_DD1D;

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A non-zero `u64`.
    pub fn next_nonzero(&mut self) -> u64 {
        loop {
            let v = self.next_u64();
            if v != 0 {
                return v;
            }
        }
    }

    pub fn next_state(&mut self) -> CnRnd {
        CnRnd::new(self.next_u64(), self.next_u64())
    }
}

pub fn rng() -> SplitMix64 {
    SplitMix64::new(FIXED_SEED)
}
