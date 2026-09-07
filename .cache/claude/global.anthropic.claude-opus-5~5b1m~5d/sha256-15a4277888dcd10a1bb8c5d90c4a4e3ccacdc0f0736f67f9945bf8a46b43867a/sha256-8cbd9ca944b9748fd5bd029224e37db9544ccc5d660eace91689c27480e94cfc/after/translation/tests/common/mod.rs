//! Shared differential-test harness: loads BOTH the C `.so` and the Rust
//! `.so` with `libloading` and calls `premultiply` through the dynamic symbol
//! in each, exactly as an external C consumer would.
//!
//! Nothing in the crate under test is ever called directly — the Rust side is
//! always reached through `libloading`, so the `#[no_mangle] extern "C"`
//! wrapper and the C ABI struct layout are part of what is verified.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirror of c_src/include/lib.h
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub type PremultiplyFn = unsafe extern "C" fn(*mut CpImage);

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so_in(dir: &Path, must_contain: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        let name = p.file_name()?.to_string_lossy().to_string();
        // `alt_O*.so` are verify.sh's supplementary higher-optimization C builds;
        // they are selected explicitly via PREMULTIPLY_C_SO, never by default.
        if name.starts_with("alt_O") {
            continue;
        }
        if let Some(frag) = must_contain {
            if !name.contains(frag) {
                continue;
            }
        }
        found.push(p);
    }
    found.sort();
    found.into_iter().next()
}

/// Path to the C shared library, building it on demand if it is not there yet.
fn c_so_path() -> PathBuf {
    // Allows Phase D to re-run the whole suite against a differently-compiled
    // C artifact (e.g. an -O2 build) without touching c_src/.
    if let Ok(p) = std::env::var("PREMULTIPLY_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "PREMULTIPLY_C_SO={:?} is not a file", p);
        return p;
    }
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so_in(&build, None) {
        return p;
    }

    // Build it: cmake configure + build, exactly as documented.
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = std::process::Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("run cmake configure");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output()
        .expect("run cmake build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );

    find_so_in(&build, None).expect("C .so present after cmake --build")
}

/// Path to the Rust `cdylib`. Prefer the profile the test itself was built
/// with (the directory containing the test executable), then fall back to the
/// usual `target/{debug,release}` locations, building if necessary.
fn rust_so_path() -> PathBuf {
    // Allows Phase D to re-run the whole suite against the release-profile
    // cdylib (opt-level=3, panic=abort) as well as the debug one.
    if let Ok(p) = std::env::var("PREMULTIPLY_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "PREMULTIPLY_RUST_SO={:?} is not a file", p);
        return p;
    }
    let frag = "premultiply_lib";

    // target/<profile>/deps/<test-exe>  ->  target/<profile>/
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1).take(3) {
            if let Some(p) = find_so_in(dir, Some(frag)) {
                return p;
            }
        }
    }

    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["debug", "release"] {
        if let Some(p) = find_so_in(&target.join(profile), Some(frag)) {
            return p;
        }
    }

    let out = std::process::Command::new(env!("CARGO"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("build")
        .output()
        .expect("cargo build for the cdylib");
    assert!(
        out.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    find_so_in(&target.join("debug"), Some(frag)).expect("Rust .so present after cargo build")
}

pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c_premultiply: PremultiplyFn,
    pub rust_premultiply: PremultiplyFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// Safety: the function pointers are only ever used while `Libs` (and hence both
// `Library` handles) is alive; `Libs` lives in a `OnceLock` for the whole
// process lifetime.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", c_path));
            let rust = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", rust_path));

            let c_sym: libloading::Symbol<PremultiplyFn> = c
                .get(b"premultiply\0")
                .expect("C .so must export `premultiply`");
            let r_sym: libloading::Symbol<PremultiplyFn> = rust
                .get(b"premultiply\0")
                .expect("Rust .so must export `premultiply`");
            let c_premultiply = *c_sym;
            let rust_premultiply = *r_sym;
            drop(c_sym);
            drop(r_sym);

            Libs {
                _c: c,
                _rust: rust,
                c_premultiply,
                rust_premultiply,
                c_path,
                rust_path,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible property-style testing)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform-ish in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn pixels(&mut self, n: usize) -> Vec<CpPixel> {
        (0..n)
            .map(|_| CpPixel {
                r: self.next_u8(),
                g: self.next_u8(),
                b: self.next_u8(),
                a: self.next_u8(),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The differential primitive
// ---------------------------------------------------------------------------

/// Byte view of a pixel slice, for exact comparison.
pub fn bytes(p: &[CpPixel]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p.as_ptr() as *const u8, p.len() * 4) }
}

/// Run `premultiply` on an identical copy of `input` in the C lib and in the
/// Rust lib and assert the resulting buffers, and the struct fields, are
/// byte-identical.
///
/// `logical_w` / `logical_h` are written into `cp_image_t` verbatim (they may
/// legitimately disagree with `input.len()`, which is the point of several
/// rows in CONFIGS.md / ERRORS.md).
#[track_caller]
pub fn diff(input: &[CpPixel], logical_w: c_int, logical_h: c_int, label: &str) -> Vec<CpPixel> {
    let l = libs();

    let mut c_buf = input.to_vec();
    let mut r_buf = input.to_vec();

    let mut c_img = CpImage {
        w: logical_w,
        h: logical_h,
        pix: c_buf.as_mut_ptr(),
    };
    let mut r_img = CpImage {
        w: logical_w,
        h: logical_h,
        pix: r_buf.as_mut_ptr(),
    };

    unsafe {
        (l.c_premultiply)(&mut c_img);
        (l.rust_premultiply)(&mut r_img);
    }

    // The C never writes through `img`, so both must leave the struct alone.
    assert_eq!(
        (c_img.w, c_img.h),
        (logical_w, logical_h),
        "[{label}] C mutated cp_image_t dimensions"
    );
    assert_eq!(
        (r_img.w, r_img.h),
        (logical_w, logical_h),
        "[{label}] Rust mutated cp_image_t dimensions"
    );
    assert!(
        std::ptr::eq(c_img.pix, c_buf.as_mut_ptr()),
        "[{label}] C mutated cp_image_t::pix"
    );
    assert!(
        std::ptr::eq(r_img.pix, r_buf.as_mut_ptr()),
        "[{label}] Rust mutated cp_image_t::pix"
    );

    if bytes(&c_buf) != bytes(&r_buf) {
        let idx = bytes(&c_buf)
            .iter()
            .zip(bytes(&r_buf))
            .position(|(a, b)| a != b)
            .unwrap();
        let px = idx / 4;
        panic!(
            "[{label}] divergence at byte {idx} (pixel {px}, channel {}):\n  \
             w={logical_w} h={logical_h} len={}\n  \
             input  = {:?}\n  C      = {:?}\n  Rust   = {:?}",
            idx % 4,
            input.len(),
            input.get(px),
            c_buf.get(px),
            r_buf.get(px),
        );
    }

    c_buf
}

/// Same as [`diff`] but `w`/`h` are taken from the buffer dimensions.
#[track_caller]
pub fn diff_wh(input: &[CpPixel], w: usize, h: usize, label: &str) -> Vec<CpPixel> {
    assert_eq!(input.len(), w * h, "test bug: buffer/shape mismatch");
    diff(input, w as c_int, h as c_int, label)
}
