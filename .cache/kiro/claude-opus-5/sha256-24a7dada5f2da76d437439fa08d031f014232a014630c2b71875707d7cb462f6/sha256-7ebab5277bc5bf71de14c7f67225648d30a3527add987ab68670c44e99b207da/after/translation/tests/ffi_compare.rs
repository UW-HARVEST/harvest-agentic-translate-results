//! Differential test: load BOTH the C shared library and the Rust cdylib via
//! `libloading` and compare their observable effects byte-for-byte.
//!
//! Neither implementation is ever called directly as a Rust function; both go
//! through `dlopen`/`dlsym` so the `#[no_mangle]` export wrappers are exercised
//! exactly as an external C caller would exercise them.

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Types mirroring c_src/include/lib.h
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

type FlipHorizontalFn = unsafe extern "C" fn(*mut CpImage);

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locate the C shared library produced by the CMake build.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src/build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("so")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("lib"))
        {
            return path;
        }
    }
    panic!("no lib*.so found in {}", build_dir.display());
}

/// Locate the Rust cdylib. `cargo test` builds it for the active profile, but
/// fall back to the other profile so the test also works after a bare
/// `cargo build --release`.
fn rust_library_path() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let name = format!("{}flip_horizontal_lib{}", DLL_PREFIX, DLL_SUFFIX);

    let mut candidates: Vec<PathBuf> = Vec::new();
    // Prefer the profile the test binary itself was built into.
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().and_then(|p| p.parent()).map(Path::to_path_buf))
    {
        candidates.push(dir.join(&name));
    }
    candidates.push(target.join("debug").join(&name));
    candidates.push(target.join("release").join(&name));

    for candidate in &candidates {
        if candidate.is_file() {
            return candidate.clone();
        }
    }
    panic!(
        "Rust cdylib {name} not found. Tried: {candidates:?}. \
         Run `cargo build` / `cargo build --release` first."
    );
}

const DLL_PREFIX: &str = "lib";
#[cfg(target_os = "macos")]
const DLL_SUFFIX: &str = ".dylib";
#[cfg(not(target_os = "macos"))]
const DLL_SUFFIX: &str = ".so";

/// The two loaded implementations plus their resolved `flip_horizontal` symbol.
struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c_flip: FlipHorizontalFn,
    rust_flip: FlipHorizontalFn,
}

fn load_impls() -> Impls {
    unsafe {
        let c_lib = Library::new(c_library_path()).expect("failed to dlopen the C library");
        let rust_lib = Library::new(rust_library_path()).expect("failed to dlopen the Rust cdylib");

        let c_sym: Symbol<FlipHorizontalFn> = c_lib
            .get(b"flip_horizontal\0")
            .expect("C library does not export flip_horizontal");
        let rust_sym: Symbol<FlipHorizontalFn> = rust_lib
            .get(b"flip_horizontal\0")
            .expect("Rust cdylib does not export flip_horizontal");

        let c_flip = *c_sym;
        let rust_flip = *rust_sym;

        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_flip,
            rust_flip,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deterministic pixel data so both sides start from identical bytes.
fn make_pixels(count: usize, seed: u64) -> Vec<CpPixel> {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u8
    };
    (0..count)
        .map(|_| CpPixel {
            r: next(),
            g: next(),
            b: next(),
            a: next(),
        })
        .collect()
}

fn as_bytes(pixels: &[CpPixel]) -> &[u8] {
    // CpPixel is #[repr(C)] of four u8s: 4 bytes, no padding.
    unsafe { std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 4) }
}

/// Run one case through both implementations and require identical results.
///
/// `buf_len` is the number of pixels actually allocated, which lets us model
/// the exact buffer the C function is allowed to touch.
fn check_case(impls: &Impls, w: c_int, h: c_int, buf_len: usize, seed: u64) {
    let mut c_pixels = make_pixels(buf_len, seed);
    let mut rust_pixels = c_pixels.clone();

    assert_eq!(
        as_bytes(&c_pixels),
        as_bytes(&rust_pixels),
        "test harness bug: inputs differ before the call (w={w}, h={h})"
    );

    let mut c_img = CpImage {
        w,
        h,
        pix: c_pixels.as_mut_ptr(),
    };
    let mut rust_img = CpImage {
        w,
        h,
        pix: rust_pixels.as_mut_ptr(),
    };

    unsafe {
        (impls.c_flip)(&mut c_img);
        (impls.rust_flip)(&mut rust_img);
    }

    assert_eq!(
        as_bytes(&c_pixels),
        as_bytes(&rust_pixels),
        "pixel buffers differ for w={w}, h={h}, buf_len={buf_len}"
    );

    // The header fields must be left untouched by both implementations.
    assert_eq!(c_img.w, rust_img.w, "w field differs for w={w}, h={h}");
    assert_eq!(c_img.h, rust_img.h, "h field differs for w={w}, h={h}");
    assert_eq!(c_img.w, w, "C mutated w for w={w}, h={h}");
    assert_eq!(c_img.h, h, "C mutated h for w={w}, h={h}");
    assert_eq!(rust_img.w, w, "Rust mutated w for w={w}, h={h}");
    assert_eq!(rust_img.h, h, "Rust mutated h for w={w}, h={h}");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn both_libraries_export_flip_horizontal() {
    // Loading already asserts the symbol resolves in both .so files.
    let _ = load_impls();
}

/// Degenerate shapes: the loop bound `h / 2` is zero, so nothing is touched.
#[test]
fn no_op_shapes() {
    let impls = load_impls();
    for (w, h) in [(0, 0), (1, 0), (0, 1), (1, 1), (5, 1), (0, 7), (13, 0)] {
        let buf = (w as usize) * (h as usize);
        check_case(&impls, w, h, buf.max(1), 0x1234 ^ (w as u64) << 8 ^ h as u64);
    }
}

/// Even heights: every row participates in a swap.
#[test]
fn even_heights() {
    let impls = load_impls();
    for w in [1, 2, 3, 4, 5, 8, 16, 17, 33, 64] {
        for h in [2, 4, 6, 8, 16, 32] {
            let buf = (w as usize) * (h as usize);
            check_case(&impls, w, h, buf, 0xABCD ^ (w as u64) << 16 ^ h as u64);
        }
    }
}

/// Odd heights: the middle row must be left in place.
#[test]
fn odd_heights() {
    let impls = load_impls();
    for w in [1, 2, 3, 7, 9, 15, 32, 63] {
        for h in [3, 5, 7, 9, 15, 31, 33] {
            let buf = (w as usize) * (h as usize);
            check_case(&impls, w, h, buf, 0x5A5A ^ (w as u64) << 16 ^ h as u64);
        }
    }
}

/// `h` negative makes `h / 2` non-positive in C (truncation toward zero), so
/// the function is a no-op regardless of `w`.
#[test]
fn negative_height_is_no_op() {
    let impls = load_impls();
    for h in [-1, -2, -3, -8, -33, c_int::MIN + 1] {
        for w in [0, 1, 4, 9] {
            check_case(&impls, w, h, 64, 0x777 ^ (h as u64) << 8 ^ w as u64);
        }
    }
}

/// Negative `w` combined with a height too small to enter the loop: still a
/// no-op, and no pointer arithmetic is performed by either side.
#[test]
fn negative_width_with_no_flips() {
    let impls = load_impls();
    for w in [-1, -5, -100] {
        for h in [0, 1, -1, -4] {
            check_case(&impls, w, h, 32, 0x999 ^ (w as u64) << 8 ^ h as u64);
        }
    }
}

/// A buffer larger than `w * h`: bytes past the logical image must be
/// untouched by both implementations.
#[test]
fn oversized_buffer_tail_untouched() {
    let impls = load_impls();
    for (w, h) in [(3, 4), (5, 5), (8, 2), (1, 9), (7, 6)] {
        let needed = (w as usize) * (h as usize);
        check_case(&impls, w, h, needed + 37, 0x2468 ^ (w as u64) << 16 ^ h as u64);
    }
}

/// Wide-and-short and tall-and-narrow extremes.
#[test]
fn extreme_aspect_ratios() {
    let impls = load_impls();
    for (w, h) in [(1, 512), (512, 2), (1024, 3), (2, 1000), (255, 4), (4, 255)] {
        let buf = (w as usize) * (h as usize);
        check_case(&impls, w, h, buf, 0xFEED ^ (w as u64) << 16 ^ h as u64);
    }
}

/// Applying the operation twice must return to the original bytes, and the C
/// and Rust results must agree at every intermediate step.
#[test]
fn double_application_is_identity_and_matches() {
    let impls = load_impls();
    for (w, h) in [(4, 6), (5, 7), (3, 2), (9, 8), (16, 16)] {
        let buf_len = (w as usize) * (h as usize);
        let original = make_pixels(buf_len, 0xC0FFEE ^ (w as u64) << 8 ^ h as u64);
        let mut c_pixels = original.clone();
        let mut rust_pixels = original.clone();

        let mut c_img = CpImage {
            w,
            h,
            pix: c_pixels.as_mut_ptr(),
        };
        let mut rust_img = CpImage {
            w,
            h,
            pix: rust_pixels.as_mut_ptr(),
        };

        for pass in 1..=2 {
            unsafe {
                (impls.c_flip)(&mut c_img);
                (impls.rust_flip)(&mut rust_img);
            }
            assert_eq!(
                as_bytes(&c_pixels),
                as_bytes(&rust_pixels),
                "mismatch after pass {pass} for w={w}, h={h}"
            );
        }

        assert_eq!(
            as_bytes(&original),
            as_bytes(&c_pixels),
            "C is not an involution for w={w}, h={h}"
        );
        assert_eq!(
            as_bytes(&original),
            as_bytes(&rust_pixels),
            "Rust is not an involution for w={w}, h={h}"
        );
    }
}

/// Randomised sweep over many shapes and seeds.
#[test]
fn randomised_sweep() {
    let impls = load_impls();
    let mut state: u64 = 0xDEAD_BEEF_CAFE_F00D;
    let mut next = |bound: u64| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) % bound) as c_int
    };

    for iteration in 0..500u64 {
        let w = next(40);
        let h = next(40);
        let needed = (w as usize) * (h as usize);
        // Occasionally over-allocate to also cover the tail-untouched property.
        let slack = (iteration % 5) as usize * 3;
        check_case(&impls, w, h, needed + slack + 1, iteration);
    }
}
