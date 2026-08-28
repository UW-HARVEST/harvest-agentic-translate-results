//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes `synth_pair` through the FFI boundary only.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `void synth_pair(int16_t *pcm, int nch, const float *z)`
pub type SynthPairFn = unsafe extern "C" fn(*mut i16, std::ffi::c_int, *const f32);

/// Number of floats in the `z` scratch buffer.
///
/// The C code touches `z[0 .. 14*64]` and then, after `z += 2`,
/// `z[0 .. 14*64]` again, so the highest index read is `2 + 14*64 == 898`.
pub const Z_LEN: usize = 1024;

/// Number of `int16_t` slots in the `pcm` buffer. `pcm[16 * nch]` is written,
/// so this must exceed `16 * max_nch`.
pub const PCM_LEN: usize = 256;

/// Sentinel used to fill `pcm` so we can prove untouched slots stay untouched
/// and that both implementations write the exact same slots.
pub const PCM_FILL: i16 = -0x5A5A;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C shared library is named after the *parent* directory of `c_src`
/// (see `c_src/CMakeLists.txt`), so discover it by globbing the build dir.
/// `SYNTH_C_SO` overrides the location, which lets the same suite be replayed
/// against C builds made with different compiler flags.
fn c_library_path() -> PathBuf {
    if let Some(p) = std::env::var_os("SYNTH_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
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
        0 => panic!("no lib*.so found in {}", build_dir.display()),
        _ => found.remove(0),
    }
}

/// The Rust `cdylib` under test.
///
/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` lib target
/// (nothing in the test graph links against it), so any pre-existing
/// `target/<profile>/libsynth_pair_lib.so` may be stale — which would silently
/// make every comparison vacuous. To guarantee the `.so` matches `src/lib.rs`
/// we compile it here, once per test process, straight from source. The crate
/// has no dependencies, so a bare `rustc` invocation reproduces exactly what
/// cargo would emit for the `cdylib` target.
fn rust_library_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = manifest.join("src/lib.rs");
        let out_dir = manifest.join("target/so-under-test");
        std::fs::create_dir_all(&out_dir).expect("create target/so-under-test");
        // Unique per process so concurrently-running test binaries never clash.
        let out = out_dir.join(format!("libsynth_pair_lib-{}.so", std::process::id()));

        let mut cmd = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
        cmd.arg("--edition=2024")
            .arg("--crate-type=cdylib")
            .arg("--crate-name=synth_pair_lib")
            // Mirror `[profile.release]` from Cargo.toml.
            .arg("-C")
            .arg("opt-level=3")
            .arg("-C")
            .arg("panic=abort")
            .arg(&src)
            .arg("-o")
            .arg(&out);
        let status = cmd.output().expect("failed to run rustc");
        assert!(
            status.status.success(),
            "rustc failed to build the cdylib:\n{}",
            String::from_utf8_lossy(&status.stderr)
        );
        assert!(out.exists(), "rustc produced no output at {}", out.display());
        out
    })
    .as_path()
}

/// Path to the `.so` that `cargo build --release` produced, if present. Used by
/// the ABI test so the *shipped* artifact's symbol table is checked too.
pub fn cargo_release_so() -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libsynth_pair_lib.so");
    p.exists().then_some(p)
}

/// Path to the freshly built Rust `.so` under test.
pub fn rust_so() -> &'static Path {
    rust_library_path()
}

/// Path to the C `.so`.
pub fn c_so() -> PathBuf {
    c_library_path()
}

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: SynthPairFn,
    pub rust: SynthPairFn,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            let c_path = c_library_path();
            let r_path = rust_library_path();
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));

            let c_sym: Symbol<SynthPairFn> = c_lib
                .get(b"synth_pair\0")
                .expect("C .so must export `synth_pair`");
            let r_sym: Symbol<SynthPairFn> = rust_lib
                .get(b"synth_pair\0")
                .expect("Rust .so must export `synth_pair`");

            let c = *c_sym;
            let rust = *r_sym;
            Pair { _c_lib: c_lib, _rust_lib: rust_lib, c, rust }
        }
    }

    /// Run both implementations on identical inputs and return
    /// `(c_pcm, rust_pcm)` full output buffers.
    pub fn run(&self, z: &[f32], nch: i32) -> (Vec<i16>, Vec<i16>) {
        assert!(z.len() >= 899, "z buffer too small");
        let mut c_pcm = vec![PCM_FILL; PCM_LEN];
        let mut r_pcm = vec![PCM_FILL; PCM_LEN];
        // Offset the writes into the middle of the buffer is unnecessary; nch>=0
        // in every test, so index 0 and 16*nch are both in range.
        unsafe {
            (self.c)(c_pcm.as_mut_ptr(), nch, z.as_ptr());
            (self.rust)(r_pcm.as_mut_ptr(), nch, z.as_ptr());
        }
        (c_pcm, r_pcm)
    }

    /// Assert byte-for-byte equality of the whole `pcm` buffer.
    pub fn check(&self, z: &[f32], nch: i32, label: &str) {
        let (c_pcm, r_pcm) = self.run(z, nch);
        if c_pcm != r_pcm {
            let idx = c_pcm
                .iter()
                .zip(&r_pcm)
                .position(|(a, b)| a != b)
                .unwrap();
            panic!(
                "mismatch [{label}] nch={nch} at pcm[{idx}]: C={} ({:#06x}) Rust={} ({:#06x})\n\
                 z[0]={:e} z[64]={:e} z[448]={:e} z[896]={:e}",
                c_pcm[idx], c_pcm[idx] as u16, r_pcm[idx], r_pcm[idx] as u16,
                z[0], z[64], z[448], z[896],
            );
        }
        // Also compare raw bytes explicitly (byte-identical requirement).
        let cb: &[u8] = unsafe {
            std::slice::from_raw_parts(c_pcm.as_ptr() as *const u8, c_pcm.len() * 2)
        };
        let rb: &[u8] = unsafe {
            std::slice::from_raw_parts(r_pcm.as_ptr() as *const u8, r_pcm.len() * 2)
        };
        assert_eq!(cb, rb, "byte-level mismatch [{label}] nch={nch}");
    }
}

/// Small deterministic xorshift PRNG so runs are reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [-1, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// The 15 `z` taps read by the first half of `synth_pair` (`z[k*64]`, k=0..=14).
pub const TAPS: [usize; 15] = [
    0, 64, 128, 192, 256, 320, 384, 448, 512, 576, 640, 704, 768, 832, 896,
];

/// The 8 `z` taps read by the second half (after `z += 2`): `z[2 + k*64]`.
pub const TAPS2: [usize; 8] = [2, 130, 258, 386, 514, 642, 770, 898];
