//! Shared differential-test harness.
//!
//! Both the C reference library and the Rust translation are loaded as *shared
//! objects* through `libloading` and driven only through their exported
//! `extern "C"` symbols. No Rust function is ever called directly, so the
//! `#[unsafe(no_mangle)]` export wrappers and the C ABI (argument truncation,
//! `xmm0` return) are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `float half2float(uint16_t)` — the declared prototype.
pub type Half2Float = unsafe extern "C" fn(u16) -> f32;

/// The same symbol seen through a *widened* prototype. C accepts any `int` at a
/// `uint16_t` parameter, so this is how an out-of-range value actually crosses
/// the FFI boundary (see ERRORS.md rows E7/E8).
pub type Half2FloatWide = unsafe extern "C" fn(u32) -> f32;

pub struct Libs {
    pub c: Library,
    /// The Rust cdylib built with the `dev` profile: debug assertions,
    /// integer-overflow checks and slice bounds checks are ON, so a divergence
    /// that would trap in Rust shows up as a panic here.
    pub rust: Library,
    /// The Rust cdylib built with the `release` profile (`panic = "abort"`),
    /// i.e. the artifact an external consumer actually links.
    pub rust_release: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    pub rust_release_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build `c_src` with CMake if the shared object is not there yet.
fn c_so_path() -> PathBuf {
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    let candidates = |dir: &Path| -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        let mut found = None;
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("lib"))
                    .unwrap_or(false)
            {
                found = Some(p);
            }
        }
        found
    };

    if let Some(p) = candidates(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let ok = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake configure")
        .success();
    assert!(ok, "cmake configure of c_src failed");
    let ok = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake build")
        .success();
    assert!(ok, "cmake build of c_src failed");

    candidates(&build).expect("no .so produced by the C build")
}

const CDYLIB: &str = "libhalf2float_lib.so";

/// The feature set the differential run is for. `cargo test` does not tell a
/// test binary which features the *cdylib* should be built with, so it is
/// passed through the environment; empty means "no features", which is the only
/// valid combination for this crate (`Cargo.toml` has no `[features]`).
fn feature_args() -> Vec<String> {
    let mut args = vec!["--no-default-features".to_string()];
    match std::env::var("HALF2FLOAT_FEATURES") {
        Ok(f) if !f.trim().is_empty() => {
            args.push("--features".to_string());
            args.push(f);
        }
        _ => {}
    }
    args
}

/// `cargo test` compiles the lib target only as an rlib/rmeta for the test
/// harness -- it does not emit the `cdylib`. Build it explicitly, into a
/// *separate* target directory so it cannot contend with the lock cargo holds
/// on `target/` while the tests run.
fn build_rust_so(profile: &str) -> PathBuf {
    if let Ok(p) = std::env::var(match profile {
        "release" => "HALF2FLOAT_RUST_SO_RELEASE",
        _ => "HALF2FLOAT_RUST_SO",
    }) {
        let p = PathBuf::from(p);
        assert!(p.exists(), "{} from env does not exist", p.display());
        return p;
    }

    let target_dir = manifest_dir().join("target/cdylib-under-test");
    let out = target_dir.join(profile).join(CDYLIB);

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(manifest_dir())
        .arg("build")
        .args(feature_args())
        .arg("--target-dir")
        .arg(&target_dir);
    if profile == "release" {
        cmd.arg("--release");
    }
    // Do not inherit the outer cargo's build state.
    cmd.env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("RUSTC_WRAPPER");

    let status = cmd.status().expect("run cargo build for the cdylib");
    assert!(
        status.success(),
        "cargo build ({profile}) of the cdylib failed"
    );
    assert!(
        out.exists(),
        "cargo build ({profile}) did not produce {}",
        out.display()
    );
    out
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = build_rust_so("debug");
        let rust_release_path = build_rust_so("release");
        let open = |p: &Path| {
            unsafe { Library::new(p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
        };
        Libs {
            c: open(&c_path),
            rust: open(&rust_path),
            rust_release: open(&rust_release_path),
            c_path,
            rust_path,
            rust_release_path,
        }
    })
}

impl Libs {
    pub fn c_fn(&self) -> Symbol<'_, Half2Float> {
        unsafe { self.c.get(b"half2float\0") }.expect("C .so does not export half2float")
    }
    /// The `dev`-profile Rust cdylib.
    pub fn rust_fn(&self) -> Symbol<'_, Half2Float> {
        unsafe { self.rust.get(b"half2float\0") }
            .expect("Rust .so (debug) does not export half2float")
    }
    /// The `release`-profile Rust cdylib.
    pub fn rust_release_fn(&self) -> Symbol<'_, Half2Float> {
        unsafe { self.rust_release.get(b"half2float\0") }
            .expect("Rust .so (release) does not export half2float")
    }
    /// Both Rust build profiles, labelled -- every differential assertion runs
    /// against both, because a debug build traps where a release build wraps.
    pub fn rust_variants(&self) -> Vec<(&'static str, Symbol<'_, Half2Float>)> {
        vec![
            ("rust/debug", self.rust_fn()),
            ("rust/release", self.rust_release_fn()),
        ]
    }
    pub fn c_fn_wide(&self) -> Symbol<'_, Half2FloatWide> {
        unsafe { self.c.get(b"half2float\0") }.expect("C .so does not export half2float")
    }
    pub fn rust_variants_wide(&self) -> Vec<(&'static str, Symbol<'_, Half2FloatWide>)> {
        vec![
            (
                "rust/debug",
                unsafe { self.rust.get(b"half2float\0") }.expect("Rust .so (debug) half2float"),
            ),
            (
                "rust/release",
                unsafe { self.rust_release.get(b"half2float\0") }
                    .expect("Rust .so (release) half2float"),
            ),
        ]
    }
}

/// Call the C library and *both* Rust build profiles with `h` and assert the
/// returned `float` is **bit-identical**. Floats are compared through
/// `to_bits`, never with `==`, so NaN payloads and signed zeros are checked too.
#[track_caller]
pub fn assert_same(label: &str, h: u16) -> u32 {
    let l = libs();
    let c = l.c_fn();
    let cv = unsafe { c(h) };
    let cb = cv.to_bits();
    for (name, r) in l.rust_variants() {
        let rv = unsafe { r(h) };
        let rb = rv.to_bits();
        assert_eq!(
            cb, rb,
            "[{label}] half2float(0x{h:04X}): C = 0x{cb:08X} ({cv:e}) but {name} = 0x{rb:08X} ({rv:e})"
        );
    }
    cb
}

/// Same, but through the widened (`u32`) prototype.
#[track_caller]
pub fn assert_same_wide(label: &str, v: u32) -> u32 {
    let l = libs();
    let c = l.c_fn_wide();
    let cb = unsafe { c(v) }.to_bits();
    for (name, r) in l.rust_variants_wide() {
        let rb = unsafe { r(v) }.to_bits();
        assert_eq!(
            cb, rb,
            "[{label}] half2float(wide 0x{v:08X}): C = 0x{cb:08X} but {name} = 0x{rb:08X}"
        );
    }
    cb
}

// ---------------------------------------------------------------------------
// Fixed-seed PRNG (SplitMix64) so every randomized row is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x1234_5678_9ABC_DEF0;

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
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
}

/// Compose a half-precision bit pattern from its table-selecting parts:
/// `n` = `h >> 10` (axis A), `mant` = `h & 0x3ff` (axis B).
pub fn h_from(n: u16, mant: u16) -> u16 {
    assert!(n < 64 && mant < 1024);
    (n << 10) | mant
}

// ---------------------------------------------------------------------------
// Independent oracle: the three lookup tables parsed straight out of the C
// source text, plus the C expression re-evaluated. Used to prove the Rust
// tables are element-wise identical to the C ones (they are `static`, hence
// invisible to `nm`).
// ---------------------------------------------------------------------------

pub struct CTables {
    pub mantissa: Vec<u32>,
    pub offset: Vec<u32>,
    pub exponent: Vec<u32>,
}

fn hex_literals(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && (bytes[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
            out.push(u32::from_str_radix(&s[start..j], 16).expect("hex literal"));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

pub fn c_tables() -> &'static CTables {
    static T: OnceLock<CTables> = OnceLock::new();
    T.get_or_init(|| {
        let src = std::fs::read_to_string(manifest_dir().join("c_src/src/lib.c"))
            .expect("read c_src/src/lib.c");
        let grab = |marker: &str| -> Vec<u32> {
            let i = src.find(marker).unwrap_or_else(|| panic!("marker {marker} not in C source"));
            let j = src[i..].find("};").expect("end of initializer") + i;
            hex_literals(&src[i..j])
        };
        let t = CTables {
            mantissa: grab("m__mantissa[2048]"),
            offset: grab("m__offset[64]"),
            exponent: grab("m__exponent[64]"),
        };
        assert_eq!(t.mantissa.len(), 2048, "C m__mantissa element count");
        assert_eq!(t.offset.len(), 64, "C m__offset element count");
        assert_eq!(t.exponent.len(), 64, "C m__exponent element count");
        t
    })
}

/// Re-evaluation of the exact C expression, straight from the parsed C tables.
pub fn oracle_bits(h: u16) -> u32 {
    let t = c_tables();
    let n = (h >> 10) as usize;
    let idx = (h & 0x3ff) as usize + t.offset[n] as usize;
    t.mantissa[idx].wrapping_add(t.exponent[n])
}
