//! Shared harness: loads BOTH shared objects via `libloading` and calls the
//! exported `colourblind` symbol through a raw function pointer.
//!
//! Nothing here ever calls a Rust function directly — the Rust side is reached
//! only through `dlopen`/`dlsym` on `libcolourblind_lib.so`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` wrapper is under
//! test as well.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

/// ABI of the single exported entry point.
pub type ColourblindFn = unsafe extern "C" fn(c_int, *mut f32, *mut f32, *mut f32);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// `cargo test` builds the integration-test binaries but **not** the `cdylib`
/// artifact, so `target/<profile>/libcolourblind_lib.so` can be arbitrarily
/// stale — which would silently make every differential test compare the C
/// library against an old Rust build. Rebuild it explicitly, then assert
/// freshness, so a stale load can never pass unnoticed.
fn ensure_rust_so_fresh(profile: &str) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.current_dir(&manifest).arg("build").arg("--lib");
    if profile == "release" {
        cmd.arg("--release");
    } else if profile != "debug" {
        cmd.arg("--profile").arg(profile);
    }
    // Keep the child from inheriting the parent's "we are inside a test" env,
    // which would otherwise confuse target-dir selection.
    cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
    let out = cmd
        .output()
        .expect("failed to spawn `cargo build --lib` to refresh the cdylib");
    assert!(
        out.status.success(),
        "`cargo build --lib` failed while refreshing the cdylib:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_fresh(so: &PathBuf) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let m = |p: &PathBuf| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
    };
    let so_t = m(so);
    let src_t = m(&src);
    assert!(
        so_t >= src_t,
        "STALE Rust .so: {} is older than {} — the differential tests would be \
         comparing the C library against an outdated Rust build. Run \
         `cargo build --release --lib` first.",
        so.display(),
        src.display()
    );
}

fn find_rust_so() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib sits two directories up from the test executable.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>/deps/<test>")
        .to_path_buf();
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();

    ensure_rust_so_fresh(&profile);

    let candidate = profile_dir.join("libcolourblind_lib.so");
    if candidate.exists() {
        assert_fresh(&candidate);
        return candidate;
    }
    for p in [
        profile_dir.join("deps/libcolourblind_lib.so"),
        workspace_root().join("translation/target/release/libcolourblind_lib.so"),
        workspace_root().join("translation/target/debug/libcolourblind_lib.so"),
    ] {
        if p.exists() {
            assert_fresh(&p);
            return p;
        }
    }
    panic!(
        "libcolourblind_lib.so not found near {} — run `cargo build --lib` first",
        profile_dir.display()
    );
}

/// Both libraries, kept alive for the process lifetime.
pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: ColourblindFn,
    pub rust: ColourblindFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// The two function pointers are plain code addresses into libraries we never
// unload; sharing them across threads is sound.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            let c_sym: Symbol<ColourblindFn> = c_lib
                .get(b"colourblind\0")
                .expect("C .so must export `colourblind`");
            let rust_sym: Symbol<ColourblindFn> = rust_lib
                .get(b"colourblind\0")
                .expect("Rust .so must export `colourblind`");

            let c = *c_sym;
            let rust = *rust_sym;

            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Render an `f32` as `bits (value)` so NaN payloads and `-0.0` are visible.
pub fn show(x: f32) -> String {
    format!("0x{:08X} ({:e})", x.to_bits(), x)
}

pub fn show3(t: (f32, f32, f32)) -> String {
    format!("[{}, {}, {}]", show(t.0), show(t.1), show(t.2))
}

/// Call one implementation on a fresh copy of `(r, g, b)` with three distinct
/// pointers and return the resulting bit patterns.
pub fn call(f: ColourblindFn, mode: c_int, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let mut rr = r;
    let mut gg = g;
    let mut bb = b;
    unsafe { f(mode, &mut rr, &mut gg, &mut bb) };
    (rr, gg, bb)
}

/// Differential assertion: identical inputs must yield bit-identical outputs.
#[track_caller]
pub fn assert_same(row: &str, mode: c_int, r: f32, g: f32, b: f32) {
    let l = libs();
    let c_out = call(l.c, mode, r, g, b);
    let rust_out = call(l.rust, mode, r, g, b);
    assert!(
        bits_eq(c_out, rust_out),
        "[{row}] divergence\n  mode  = {mode}\n  input = {}\n  C     = {}\n  Rust  = {}",
        show3((r, g, b)),
        show3(c_out),
        show3(rust_out)
    );
}

pub fn bits_eq(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
    a.0.to_bits() == b.0.to_bits()
        && a.1.to_bits() == b.1.to_bits()
        && a.2.to_bits() == b.2.to_bits()
}

pub const MODES: [c_int; 3] = [0, 1, 2];

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[-1, 1)`.
    pub fn signed_unit(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// Any 32-bit pattern reinterpreted as `f32` (may be NaN/inf/subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Any *finite* `f32`: force the exponent field away from all-ones.
    pub fn finite_f32(&mut self) -> f32 {
        let bits = self.next_u32();
        let exp = (bits >> 23) & 0xFF;
        let exp = if exp == 0xFF { 0xFE } else { exp };
        f32::from_bits((bits & 0x807F_FFFF) | (exp << 23))
    }

    /// A quiet NaN with a random non-zero payload and random sign.
    pub fn qnan(&mut self) -> f32 {
        let bits = self.next_u32();
        let payload = (bits & 0x003F_FFFF) | 0x0000_0001; // keep payload non-zero
        f32::from_bits((bits & 0x8000_0000) | 0x7F80_0000 | 0x0040_0000 | payload)
    }

    /// A signalling NaN (mantissa MSB clear, payload non-zero) with random sign.
    pub fn snan(&mut self) -> f32 {
        let bits = self.next_u32();
        let payload = (bits & 0x003F_FFFF) | 0x0000_0001; // non-zero => not inf
        f32::from_bits((bits & 0x8000_0000) | 0x7F80_0000 | payload)
    }
}
