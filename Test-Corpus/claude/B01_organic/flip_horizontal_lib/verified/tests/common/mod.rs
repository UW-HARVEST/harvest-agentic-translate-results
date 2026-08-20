//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects via `libloading` and
//! called through their exported `flip_horizontal` symbol. The Rust
//! implementation is never called directly, so the `#[no_mangle] extern "C"`
//! wrapper and the `#[repr(C)]` layouts are part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C ABI mirror (declared here independently of the crate under test, exactly
// as an external consumer would declare it from `c_src/include/lib.h`).
// ---------------------------------------------------------------------------

/// `typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// `typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CpImage {
    pub w: std::ffi::c_int,
    pub h: std::ffi::c_int,
    pub pix: *mut CpPixel,
}

pub const PIXEL_SIZE: usize = 4;

/// `void flip_horizontal(cp_image_t *img);`
pub type FlipHorizontalFn = unsafe extern "C" fn(*mut CpImage);

// ---------------------------------------------------------------------------
// Shared-object loading
// ---------------------------------------------------------------------------

pub struct Lib {
    /// Kept alive so the loaded symbol stays valid.
    _lib: libloading::Library,
    flip: FlipHorizontalFn,
    pub which: &'static str,
    pub path: PathBuf,
}

impl Lib {
    fn open(path: PathBuf, which: &'static str) -> Lib {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} .so at {}: {e}", which, path.display()));
        // Resolve the symbol by its exact exported name. This also asserts the
        // symbol is actually exported from the object.
        let flip: FlipHorizontalFn = unsafe {
            let sym: libloading::Symbol<FlipHorizontalFn> =
                lib.get(b"flip_horizontal\0").unwrap_or_else(|e| {
                    panic!("{} .so does not export `flip_horizontal`: {e}", which)
                });
            *sym
        };
        Lib { _lib: lib, flip, which, path }
    }

    /// The C reference implementation, built by `c_src/CMakeLists.txt`.
    pub fn c() -> Lib {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("c_src/build/libtranslated_rust.so");
        assert!(
            path.exists(),
            "C shared object not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            path.display()
        );
        Lib::open(path, "C")
    }

    /// The Rust translation's cdylib. Prefers the same cargo profile directory
    /// as the running test binary (`target/<profile>/deps/<test>` ->
    /// `target/<profile>/`), falling back to the other profiles.
    pub fn rust() -> Lib {
        const SO: &str = "libflip_horizontal_lib.so";
        let mut candidates: Vec<PathBuf> = Vec::new();

        // Explicit override, used to re-run the whole suite against the
        // release / panic=abort artifact.
        if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
            if !p.is_empty() {
                candidates.push(PathBuf::from(p));
            }
        }

        if let Ok(exe) = std::env::current_exe() {
            // target/<profile>/deps/<test-bin>  ->  target/<profile>/<SO>
            if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
                candidates.push(profile_dir.join(SO));
            }
            if let Some(deps) = exe.parent() {
                candidates.push(deps.join(SO));
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for profile in ["debug", "release"] {
            candidates.push(root.join("target").join(profile).join(SO));
        }

        let path = candidates
            .iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                panic!(
                    "Rust shared object `{SO}` not found. Looked in:\n{}\n\n\
                     NOTE: `cargo test` does NOT build the cdylib artifact, because the\n\
                     integration tests do not link against it (they dlopen it). Build it\n\
                     first:\n  cargo build && cargo build --release\n\
                     or just use ./run_tests.sh",
                    candidates
                        .iter()
                        .map(|p| format!("  {}", p.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .clone();

        assert_so_fresh(&path);
        Lib::open(path, "Rust")
    }

    /// Call `flip_horizontal` through the FFI boundary.
    pub unsafe fn flip(&self, img: *mut CpImage) {
        unsafe { (self.flip)(img) }
    }
}

/// Guards against the trap that `cargo test` does not rebuild the `cdylib`:
/// if the `.so` we are about to load is older than any Rust source file, the
/// whole test run would be validating dead code.
fn assert_so_fresh(so: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src_dir = root.join("src");
    let entries = match std::fs::read_dir(&src_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Ok(src_mtime) = entry.metadata().and_then(|m| m.modified()) {
            assert!(
                so_mtime >= src_mtime,
                "STALE ARTIFACT: {} is older than {}.\n\
                 `cargo test` does not rebuild the cdylib. Run `cargo build && \
                 cargo build --release` (or ./run_tests.sh) first, otherwise the \
                 tests validate an outdated library.",
                so.display(),
                p.display()
            );
        }
    }
}

/// Loads both libraries. Every test starts from here.
pub fn both() -> (Lib, Lib) {
    (Lib::c(), Lib::rust())
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every run reproducible.
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

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }

    pub fn i32_any(&mut self) -> i32 {
        self.next_u64() as i32
    }

    /// `n` random bytes.
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

// ---------------------------------------------------------------------------
// The differential core
// ---------------------------------------------------------------------------

/// One differential case.
pub struct Case {
    /// `img->w` handed to the library (may be negative / INT_MIN / ...).
    pub w: i32,
    /// `img->h` handed to the library.
    pub h: i32,
    /// The pixel buffer contents (length must be a multiple of 4). This is the
    /// *real* allocation; it is independent of `w`/`h` on purpose so that
    /// oversized-buffer / canary shapes can be expressed.
    pub data: Vec<u8>,
    /// Pass `img->pix == NULL` instead of a pointer into `data`.
    pub null_pix: bool,
    /// How many times to invoke `flip_horizontal` on the same image.
    pub calls: usize,
}

impl Case {
    /// A case whose buffer is exactly `w * h` pixels of random data.
    pub fn exact(rng: &mut Rng, w: i32, h: i32) -> Case {
        let px = (w.max(0) as i64 * h.max(0) as i64).max(0) as usize;
        Case { w, h, data: rng.bytes(px * PIXEL_SIZE), null_pix: false, calls: 1 }
    }

    /// A case with a buffer of `pixels` pixels of random data, regardless of
    /// `w`/`h`.
    pub fn sized(rng: &mut Rng, w: i32, h: i32, pixels: usize) -> Case {
        Case { w, h, data: rng.bytes(pixels * PIXEL_SIZE), null_pix: false, calls: 1 }
    }

    /// A case with `img->pix == NULL` (no allocation at all).
    pub fn null_pix(w: i32, h: i32) -> Case {
        Case { w, h, data: Vec::new(), null_pix: true, calls: 1 }
    }

    pub fn with_calls(mut self, calls: usize) -> Case {
        self.calls = calls;
        self
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Case {
        self.data = data;
        self
    }
}

/// Result of driving one implementation over one case.
pub struct Observed {
    pub data: Vec<u8>,
    pub w: i32,
    pub h: i32,
    /// Whether `img->pix` still holds the pointer we handed in.
    pub pix_unchanged: bool,
}

fn run_one(lib: &Lib, case: &Case) -> Observed {
    assert_eq!(case.data.len() % PIXEL_SIZE, 0, "buffer must be a whole number of pixels");
    let mut buf = case.data.clone();
    let pix: *mut CpPixel =
        if case.null_pix { std::ptr::null_mut() } else { buf.as_mut_ptr().cast::<CpPixel>() };
    let mut img = CpImage { w: case.w, h: case.h, pix };
    for _ in 0..case.calls {
        unsafe { lib.flip(&mut img) };
    }
    Observed { data: buf, w: img.w, h: img.h, pix_unchanged: img.pix == pix }
}

/// Runs `case` through BOTH shared objects and asserts byte-identical results.
///
/// Panics with a detailed report on the first divergence.
pub fn assert_same(c: &Lib, r: &Lib, case: &Case, label: &str) {
    let _ = assert_same_observed(c, r, case, label);
}

/// Same as [`assert_same`] but returns the two observations so callers can make
/// further assertions without re-running the (possibly very slow) libraries.
pub fn assert_same_observed(c: &Lib, r: &Lib, case: &Case, label: &str) -> (Observed, Observed) {
    let oc = run_one(c, case);
    let orr = run_one(r, case);

    let ctx = || {
        format!(
            "case `{label}`: w={} h={} calls={} null_pix={} buffer_pixels={}",
            case.w,
            case.h,
            case.calls,
            case.null_pix,
            case.data.len() / PIXEL_SIZE
        )
    };

    assert_eq!(oc.w, orr.w, "img->w diverged ({}): C={} Rust={}", ctx(), oc.w, orr.w);
    assert_eq!(oc.h, orr.h, "img->h diverged ({}): C={} Rust={}", ctx(), oc.h, orr.h);
    assert_eq!(
        oc.pix_unchanged,
        orr.pix_unchanged,
        "img->pix mutation diverged ({}): C_unchanged={} Rust_unchanged={}",
        ctx(),
        oc.pix_unchanged,
        orr.pix_unchanged
    );
    // The C never writes to the struct, so both must have left it pristine.
    assert_eq!(oc.w, case.w, "C modified img->w ({})", ctx());
    assert_eq!(oc.h, case.h, "C modified img->h ({})", ctx());
    assert!(oc.pix_unchanged, "C modified img->pix ({})", ctx());

    assert_eq!(
        oc.data.len(),
        orr.data.len(),
        "buffer length diverged ({}) — harness bug",
        ctx()
    );
    if oc.data != orr.data {
        let idx = oc.data.iter().zip(orr.data.iter()).position(|(a, b)| a != b).unwrap();
        let ndiff = oc.data.iter().zip(orr.data.iter()).filter(|(a, b)| a != b).count();
        panic!(
            "PIXEL BUFFER DIVERGED\n  {}\n  first differing byte index {} (pixel {}, channel {})\n\
             \n  C   : {:02x?}\n  Rust: {:02x?}\n  ({} of {} bytes differ)\n  input: {:02x?}",
            ctx(),
            idx,
            idx / PIXEL_SIZE,
            ["r", "g", "b", "a"][idx % PIXEL_SIZE],
            &oc.data[idx.saturating_sub(8)..(idx + 8).min(oc.data.len())],
            &orr.data[idx.saturating_sub(8)..(idx + 8).min(orr.data.len())],
            ndiff,
            oc.data.len(),
            &case.data[..case.data.len().min(64)],
        );
    }

    (oc, orr)
}

/// Asserts the case is a complete no-op in BOTH implementations *and* that the
/// two agree. This is the "(S) silent no-op" rejection of `ERRORS.md`.
pub fn assert_same_and_noop(c: &Lib, r: &Lib, case: &Case, label: &str) {
    let (oc, orr) = assert_same_observed(c, r, case, label);
    assert_eq!(
        oc.data, case.data,
        "expected C no-op but the buffer changed (case `{label}`, w={} h={})",
        case.w, case.h
    );
    assert_eq!(
        orr.data, case.data,
        "expected Rust no-op but the buffer changed (case `{label}`, w={} h={})",
        case.w, case.h
    );
}
