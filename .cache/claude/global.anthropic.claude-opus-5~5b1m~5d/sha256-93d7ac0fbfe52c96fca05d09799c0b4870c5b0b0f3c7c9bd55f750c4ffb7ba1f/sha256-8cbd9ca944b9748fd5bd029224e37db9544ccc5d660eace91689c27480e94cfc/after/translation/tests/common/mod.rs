//! Shared differential-test harness.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and called
//! only through their exported C symbols — the Rust crate is never linked or
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

pub type HsvToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HsvToRgbFn,
    pub rust: HsvToRgbFn,
}

// Safety: both symbols are plain reentrant leaf functions with no global state.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// Build the C shared object with CMake (idempotent) and return its path.
///
/// `$HARNESS_C_SO` overrides the path, which lets the same suite be replayed
/// against C libraries built at other optimization levels (-O0/-O2/-O3/-Ofast)
/// to prove the Rust matches the C's *semantics*, not one particular codegen.
fn build_and_find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARNESS_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARNESS_C_SO does not exist: {p:?}");
        return p;
    }
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");

    if !build.join("CMakeCache.txt").exists() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let st = Command::new("cmake")
            .current_dir(&build)
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .status()
            .expect("run cmake configure");
        assert!(st.success(), "cmake configure failed");
    }
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake build");
    assert!(st.success(), "cmake build failed");

    let mut found = Vec::new();
    for entry in std::fs::read_dir(&build).expect("read c_src/build") {
        let p = entry.expect("dir entry").path();
        if p.extension().and_then(|e| e.to_str()) == Some("so") {
            found.push(p);
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one C .so in {build:?}, found {found:?}"
    );
    found.pop().unwrap()
}

/// Build the Rust cdylib FRESH and return its path.
///
/// IMPORTANT: `cargo test` does **not** rebuild a `crate-type = ["cdylib"]`
/// lib target, because integration tests never link against it. Loading
/// `target/<profile>/libhsv_to_rgb_lib.so` opportunistically would silently
/// test a STALE artifact (verified: an injected bug went undetected). So the
/// harness always shells out to `cargo build --lib` itself.
///
/// The build uses a dedicated `CARGO_TARGET_DIR` so it can never contend with
/// the outer `cargo test` invocation's lock on `target/`.
///
/// Feature flags are forwarded from `$HARNESS_CARGO_ARGS` so the same suite can
/// be run under every feature combination.
fn build_and_find_rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = "libhsv_to_rgb_lib.so";

    let extra: Vec<String> = std::env::var("HARNESS_CARGO_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect();

    // Isolate per feature-combo so combos cannot read each other's artifacts.
    let tag: String = {
        let key = extra.join("_");
        if key.is_empty() {
            "default".to_string()
        } else {
            key.chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect()
        }
    };
    let target_dir = manifest.join("target/ffi-harness").join(&tag);

    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir(&manifest)
        .env("CARGO_TARGET_DIR", &target_dir)
        // Do not inherit the outer test run's flags/profile selection.
        .env_remove("RUSTFLAGS")
        .args(["build", "--release", "--lib", "--offline"])
        .args(&extra);

    let out = cmd.output().expect("run cargo build --release --lib");
    assert!(
        out.status.success(),
        "cargo build of the cdylib failed (args {extra:?}):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let p = target_dir.join("release").join(name);
    assert!(
        p.exists(),
        "rust cdylib missing at {p:?} after a successful build"
    );
    p
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = build_and_find_c_so();
        let rust_path = build_and_find_rust_so();

        unsafe {
            let c_lib = Library::new(&c_path).unwrap_or_else(|e| panic!("load {c_path:?}: {e}"));
            let rust_lib =
                Library::new(&rust_path).unwrap_or_else(|e| panic!("load {rust_path:?}: {e}"));

            let c_sym: Symbol<HsvToRgbFn> = c_lib
                .get(b"hsv_to_rgb\0")
                .expect("C .so exports hsv_to_rgb");
            let rust_sym: Symbol<HsvToRgbFn> = rust_lib
                .get(b"hsv_to_rgb\0")
                .expect("Rust .so exports hsv_to_rgb");

            let c = *c_sym;
            let rust = *rust_sym;
            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub const CANARY: u32 = 0xDEAD_BEEF;

/// Show a float unambiguously: value + raw bit pattern.
pub fn fmt(v: f32) -> String {
    format!("{v:?}/0x{:08x}", v.to_bits())
}

pub fn fmt3(v: &[f32; 3]) -> String {
    format!("[{}, {}, {}]", fmt(v[0]), fmt(v[1]), fmt(v[2]))
}

/// Call both libraries on `src` with disjoint destination buffers guarded by
/// canaries, and assert the 3 written floats are bit-identical.
#[track_caller]
pub fn check(row: &str, src: [f32; 3]) -> [f32; 3] {
    let l = libs();

    // 5-wide buffers: [canary, out0, out1, out2, canary]
    let mut c_buf = [f32::from_bits(CANARY); 5];
    let mut r_buf = [f32::from_bits(CANARY); 5];

    let c_src_buf = src;
    let r_src_buf = src;

    unsafe {
        (l.c)(c_buf.as_mut_ptr().add(1), c_src_buf.as_ptr());
        (l.rust)(r_buf.as_mut_ptr().add(1), r_src_buf.as_ptr());
    }

    let cb: Vec<u32> = c_buf.iter().map(|f| f.to_bits()).collect();
    let rb: Vec<u32> = r_buf.iter().map(|f| f.to_bits()).collect();

    assert_eq!(
        cb, rb,
        "[{row}] output mismatch for src = {}\n  C    = {}\n  Rust = {}",
        fmt3(&src),
        fmt3(&[c_buf[1], c_buf[2], c_buf[3]]),
        fmt3(&[r_buf[1], r_buf[2], r_buf[3]]),
    );

    // src must not be modified by either library (`const float *`).
    assert_eq!(
        c_src_buf.map(f32::to_bits),
        src.map(f32::to_bits),
        "[{row}] C modified src"
    );
    assert_eq!(
        r_src_buf.map(f32::to_bits),
        src.map(f32::to_bits),
        "[{row}] Rust modified src"
    );

    [c_buf[1], c_buf[2], c_buf[3]]
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed => reproducible property tests)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5eed_1234_9e37_79b9;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// Any f32 bit pattern (includes NaNs with arbitrary payloads, Inf,
    /// subnormals, negative zero).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

/// Boundary / special float pool used for exhaustive cross-products.
pub const SPECIAL: &[f32] = &[
    0.0,
    -0.0,
    f32::NAN,
    -f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MIN_POSITIVE,          // smallest normal
    -f32::MIN_POSITIVE,
    1e-45,                      // smallest subnormal
    -1e-45,
    f32::MAX,
    f32::MIN,
    1.0,
    -1.0,
    0.5,
    -0.5,
    60.0,
    -60.0,
    59.999996,
    360.0,
    2147483520.0,               // largest f32 < 2^31
    2147483648.0,               // 2^31
    -2147483648.0,              // -2^31
    -2147483904.0,              // one step below -2^31
    1e30,
    -1e30,
    128849018880.0,             // 2^31 * 60 exactly
    3.4028235e38,
    1.1754942e-38,              // largest subnormal
    255.0,
];
