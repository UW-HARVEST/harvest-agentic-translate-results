//! Shared harness: loads BOTH the C `.so` and the Rust `cdylib` via `libloading`
//! and calls every function through the FFI boundary only.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub const STRUCT_SIZE: usize = 28;

/// Raw 28-byte image of `struct tflac`, so that padding bytes are observable.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(4))]
pub struct Raw(pub [u8; STRUCT_SIZE]);

impl Raw {
    pub fn zeroed() -> Self {
        Raw([0u8; STRUCT_SIZE])
    }

    fn u32_at(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.0[off..off + 4].try_into().unwrap())
    }
    fn set_u32(&mut self, off: usize, v: u32) {
        self.0[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }

    pub fn blocksize(&self) -> u32 {
        self.u32_at(0)
    }
    pub fn samplerate(&self) -> u32 {
        self.u32_at(4)
    }
    pub fn channels(&self) -> u32 {
        self.u32_at(8)
    }
    pub fn bitdepth(&self) -> u32 {
        self.u32_at(12)
    }
    pub fn channel_mode(&self) -> u8 {
        self.0[16]
    }
    pub fn max_rice_value(&self) -> u8 {
        self.0[17]
    }
    pub fn min_partition_order(&self) -> u8 {
        self.0[18]
    }
    pub fn max_partition_order(&self) -> u8 {
        self.0[19]
    }
    pub fn partition_order(&self) -> u8 {
        self.0[20]
    }
    pub fn cur_blocksize(&self) -> u32 {
        self.u32_at(24)
    }

    pub fn set_blocksize(&mut self, v: u32) -> &mut Self {
        self.set_u32(0, v);
        self
    }
    pub fn set_samplerate(&mut self, v: u32) -> &mut Self {
        self.set_u32(4, v);
        self
    }
    pub fn set_channels(&mut self, v: u32) -> &mut Self {
        self.set_u32(8, v);
        self
    }
    pub fn set_bitdepth(&mut self, v: u32) -> &mut Self {
        self.set_u32(12, v);
        self
    }
    pub fn set_channel_mode(&mut self, v: u8) -> &mut Self {
        self.0[16] = v;
        self
    }
    pub fn set_max_rice_value(&mut self, v: u8) -> &mut Self {
        self.0[17] = v;
        self
    }
    pub fn set_min_partition_order(&mut self, v: u8) -> &mut Self {
        self.0[18] = v;
        self
    }
    pub fn set_max_partition_order(&mut self, v: u8) -> &mut Self {
        self.0[19] = v;
        self
    }
    pub fn set_partition_order(&mut self, v: u8) -> &mut Self {
        self.0[20] = v;
        self
    }
    pub fn set_cur_blocksize(&mut self, v: u32) -> &mut Self {
        self.set_u32(24, v);
        self
    }

    /// A struct that passes every validation check.
    pub fn valid() -> Self {
        let mut r = Raw::zeroed();
        r.set_blocksize(4096)
            .set_samplerate(44100)
            .set_channels(2)
            .set_bitdepth(16);
        r
    }
}

impl std::fmt::Debug for Raw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("tflac")
            .field("blocksize", &self.blocksize())
            .field("samplerate", &self.samplerate())
            .field("channels", &self.channels())
            .field("bitdepth", &self.bitdepth())
            .field("channel_mode", &self.channel_mode())
            .field("max_rice_value", &self.max_rice_value())
            .field("min_partition_order", &self.min_partition_order())
            .field("max_partition_order", &self.max_partition_order())
            .field("partition_order", &self.partition_order())
            .field("cur_blocksize", &self.cur_blocksize())
            .field("pad[21..24]", &&self.0[21..24])
            .field("bytes", &self.0)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn sh(cmd: &str, cwd: &Path) -> String {
    let out = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `{cmd}`: {e}"));
    if !out.status.success() {
        panic!(
            "command `{cmd}` failed ({:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
}

/// Build (if needed) and return the path of the C shared object.
pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let root = workspace_root();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        std::fs::create_dir_all(&build).unwrap();
        // Always (re)configure + build; cmake makes this a cheap no-op when the
        // C sources are unchanged. c_src/ itself is never modified.
        sh(
            "cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            &build,
        );
        find_so(&build).expect("no C .so produced in c_src/build")
    })
    .as_path()
}

/// Build and return the path of the Rust `cdylib`.
///
/// The tests deliberately never call the Rust functions directly; they are
/// always resolved out of this shared object, exactly like an external C caller.
pub fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Build the cdylib with the SAME profile this test binary was compiled
        // with, so that e.g. `panic = "abort"` / optimisation settings of the
        // release profile are actually exercised. `cargo test` does not build
        // the cdylib target itself, so this is required, not a fallback.
        let release = !cfg!(debug_assertions);
        let (cmd, profile_dir) = if release {
            ("cargo build --offline --release --lib", "release")
        } else {
            ("cargo build --offline --lib", "debug")
        };
        let dir = manifest.join("target").join(profile_dir);
        // ALWAYS rebuild: skipping when an artifact already exists would silently
        // test a stale `.so` against fresh `src/lib.rs` (verified by mutation
        // testing — a deliberate bug went undetected with a conditional build).
        sh(cmd, &manifest);
        find_so(&dir).unwrap_or_else(|| {
            panic!("no Rust .so found in {}", dir.display());
        })
    })
    .as_path()
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C .so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust .so");
        Libs { c, rust }
    })
}

type FnSizeMemory = unsafe extern "C" fn(u32) -> u32;
type FnValidate = unsafe extern "C" fn(*mut Raw) -> std::ffi::c_int;

fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe { lib.get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` missing: {e}"))
}

pub struct Api {
    pub size_memory: Symbol<'static, FnSizeMemory>,
    pub validate: Symbol<'static, FnValidate>,
}

pub fn c_api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| {
        let l = &libs().c;
        Api {
            size_memory: sym(l, "tflac_size_memory"),
            validate: sym(l, "flac_validate"),
        }
    })
}

pub fn rust_api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| {
        let l = &libs().rust;
        Api {
            size_memory: sym(l, "tflac_size_memory"),
            validate: sym(l, "flac_validate"),
        }
    })
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `tflac_size_memory` in both libraries and assert byte-identical results.
pub fn diff_size_memory(blocksize: u32) -> u32 {
    let c = unsafe { (c_api().size_memory)(blocksize) };
    let r = unsafe { (rust_api().size_memory)(blocksize) };
    assert_eq!(
        c, r,
        "tflac_size_memory({blocksize}) diverged: C={c:#010x} Rust={r:#010x}"
    );
    c
}

/// Call `flac_validate` in both libraries on identical struct images and assert
/// the return code AND all 28 bytes (including padding) match.
pub fn diff_validate(input: Raw) -> (std::ffi::c_int, Raw) {
    let mut cs = input;
    let mut rs = input;
    let cr = unsafe { (c_api().validate)(&mut cs as *mut Raw) };
    let rr = unsafe { (rust_api().validate)(&mut rs as *mut Raw) };
    assert_eq!(
        cr, rr,
        "flac_validate return diverged for {input:?}: C={cr} Rust={rr}"
    );
    assert_eq!(
        cs, rs,
        "flac_validate struct diverged for {input:?}\n C out: {cs:?}\n R out: {rs:?}"
    );
    (cr, cs)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `lo..=hi`.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}
