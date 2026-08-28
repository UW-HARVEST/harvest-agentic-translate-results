//! Differential test: loads BOTH the C shared library and the Rust cdylib via
//! `libloading` and compares the observable effect of `premultiply` byte-for-byte.
//!
//! No Rust function is ever called directly -- everything goes through the
//! `.so` export, so the `#[no_mangle]` wrapper is exercised as well.

use std::ffi::{c_int, OsStr};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// Mirrors `cp_pixel_t` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

/// Mirrors `cp_image_t` from `c_src/include/lib.h`.
#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

type PremultiplyFn = unsafe extern "C" fn(*mut CpImage);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared library produced by the CMake build.
fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("so")) {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locate the Rust cdylib.
///
/// `cargo test` only emits an `rmeta` for a `crate-type = ["cdylib"]` library,
/// so the `.so` is not guaranteed to exist. If it is missing we build it
/// ourselves into a dedicated `CARGO_TARGET_DIR` (which avoids contending for
/// the lock on the main target directory) so that `cargo test` works from a
/// clean checkout with no extra manual steps.
fn rust_library_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(Path::parent)
            .expect("target/<profile>/deps/<test>");

        if let Some(found) = find_cdylib(profile_dir) {
            return found;
        }

        // Not built by this cargo invocation -- build it out-of-band.
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let nested_target = manifest_dir.join("target/ffi-cdylib");
        let release = profile_dir.file_name() == Some(OsStr::new("release"));

        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build")
            .arg("--lib")
            .arg("--manifest-path")
            .arg(manifest_dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &nested_target)
            // Do not inherit the test harness's RUSTFLAGS/profile overrides.
            .env_remove("CARGO_PRIMARY_PACKAGE")
            .env_remove("RUSTC_WRAPPER");
        if release {
            cmd.arg("--release");
        }

        let status = cmd.status().expect("failed to spawn cargo to build cdylib");
        assert!(status.success(), "cargo build --lib failed: {status}");

        let nested_profile = nested_target.join(if release { "release" } else { "debug" });
        find_cdylib(&nested_profile).unwrap_or_else(|| {
            panic!(
                "Rust cdylib still not found in {} after building",
                nested_profile.display()
            )
        })
    })
    .clone()
}

fn find_cdylib(dir: &Path) -> Option<PathBuf> {
    for name in ["libpremultiply_lib.so", "premultiply_lib.so"] {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c: PremultiplyFn,
    rust: PremultiplyFn,
}

impl Impls {
    fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_library_path()).expect("load C .so");
            let rust_lib = Library::new(rust_library_path()).expect("load Rust .so");

            let c_sym: Symbol<PremultiplyFn> =
                c_lib.get(b"premultiply\0").expect("C premultiply symbol");
            let rust_sym: Symbol<PremultiplyFn> = rust_lib
                .get(b"premultiply\0")
                .expect("Rust premultiply symbol (missing #[no_mangle] export?)");

            let c = *c_sym;
            let rust = *rust_sym;

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Run both implementations on identical copies of `pixels` and assert the
    /// resulting buffers are byte-identical.
    fn assert_parity(&self, label: &str, w: c_int, h: c_int, pixels: &[CpPixel]) {
        let mut c_buf = pixels.to_vec();
        let mut rust_buf = pixels.to_vec();

        let mut c_img = CpImage {
            w,
            h,
            pix: c_buf.as_mut_ptr(),
        };
        let mut rust_img = CpImage {
            w,
            h,
            pix: rust_buf.as_mut_ptr(),
        };

        unsafe {
            (self.c)(&mut c_img);
            (self.rust)(&mut rust_img);
        }

        // Compare the raw bytes, not just the struct fields.
        let c_bytes = as_bytes(&c_buf);
        let rust_bytes = as_bytes(&rust_buf);

        if c_bytes != rust_bytes {
            let mismatch = c_bytes
                .iter()
                .zip(rust_bytes.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            let pixel = mismatch / 4;
            panic!(
                "{label}: mismatch at byte {mismatch} (pixel {pixel}, w={w}, h={h})\n  \
                 input  = {:?}\n  C      = {:?}\n  Rust   = {:?}",
                pixels[pixel], c_buf[pixel], rust_buf[pixel]
            );
        }

        // The struct itself must be left untouched by both.
        assert_eq!(c_img.w, rust_img.w, "{label}: img.w diverged");
        assert_eq!(c_img.h, rust_img.h, "{label}: img.h diverged");
    }
}

fn as_bytes(pixels: &[CpPixel]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4) }
}

/// Small deterministic xorshift PRNG so the test is reproducible.
struct Rng(u64);

impl Rng {
    fn next_u8(&mut self) -> u8 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 33) as u8
    }
}

#[test]
fn symbol_is_exported_by_both() {
    // Loading alone proves the export names line up.
    let _ = Impls::load();
}

/// Exhaustive: every (channel value, alpha value) pair, 256*256 = 65536 pixels.
/// This is the complete input domain of the per-channel arithmetic.
#[test]
fn exhaustive_channel_alpha_pairs() {
    let impls = Impls::load();

    let mut pixels = Vec::with_capacity(256 * 256);
    for a in 0u16..256 {
        for v in 0u16..256 {
            pixels.push(CpPixel {
                r: v as u8,
                g: v as u8,
                b: v as u8,
                a: a as u8,
            });
        }
    }
    assert_eq!(pixels.len(), 65536);

    // w * h must equal pixels.len() so the C loop bound stays inside the buffer.
    impls.assert_parity("exhaustive 256x256", 256, 256, &pixels);
}

/// Exhaustive over distinct r/g/b values combined with a sweep of alphas, to
/// prove the three channels are not accidentally swapped or aliased.
#[test]
fn exhaustive_distinct_channels() {
    let impls = Impls::load();

    let mut pixels = Vec::new();
    for a in 0u16..256 {
        for v in 0u16..256 {
            pixels.push(CpPixel {
                r: v as u8,
                g: (255 - v) as u8,
                b: v.wrapping_mul(7) as u8,
                a: a as u8,
            });
        }
    }
    impls.assert_parity("distinct channels 256x256", 256, 256, &pixels);
}

#[test]
fn random_images_various_dimensions() {
    let impls = Impls::load();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);

    let dims: &[(c_int, c_int)] = &[
        (1, 1),
        (1, 2),
        (2, 1),
        (3, 5),
        (5, 3),
        (7, 7),
        (16, 16),
        (17, 13),
        (32, 1),
        (1, 32),
        (64, 64),
        (100, 37),
        (255, 3),
    ];

    for &(w, h) in dims {
        let n = (w as usize) * (h as usize);
        for round in 0..8 {
            let pixels: Vec<CpPixel> = (0..n)
                .map(|_| CpPixel {
                    r: rng.next_u8(),
                    g: rng.next_u8(),
                    b: rng.next_u8(),
                    a: rng.next_u8(),
                })
                .collect();
            impls.assert_parity(&format!("random {w}x{h} round {round}"), w, h, &pixels);
        }
    }
}

/// Extreme / boundary pixel values.
#[test]
fn boundary_pixel_values() {
    let impls = Impls::load();

    let interesting: [u8; 12] = [0, 1, 2, 3, 127, 128, 129, 200, 253, 254, 255, 64];
    let mut pixels = Vec::new();
    for &a in &interesting {
        for &r in &interesting {
            for &g in &interesting {
                for &b in &interesting {
                    pixels.push(CpPixel { r, g, b, a });
                }
            }
        }
    }
    // 12^4 = 20736 pixels; use w=144, h=144.
    assert_eq!(pixels.len(), 144 * 144);
    impls.assert_parity("boundary values 144x144", 144, 144, &pixels);
}

/// Degenerate dimensions: the C loop bound is `(int)stride * h`, so anything
/// that makes the bound <= 0 must leave the buffer completely untouched.
#[test]
fn degenerate_dimensions_leave_buffer_untouched() {
    let impls = Impls::load();
    let mut rng = Rng(0xDEAD_BEEF_CAFE_1234);

    let pixels: Vec<CpPixel> = (0..64)
        .map(|_| CpPixel {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
            a: rng.next_u8(),
        })
        .collect();

    let dims: &[(c_int, c_int)] = &[
        (0, 0),
        (0, 8),
        (8, 0),
        (0, -1),
        (-1, 0),
        (-1, 8),
        (8, -1),
        (-4, -4),
        (-2, -2),
        (-8, -2),
        (-1, -1),
        (-64, 1),
        (1, -64),
    ];

    for &(w, h) in dims {
        impls.assert_parity(&format!("degenerate {w}x{h}"), w, h, &pixels);

        // Mirror the C loop bound: `int stride = w * sizeof(cp_pixel_t);`
        // followed by `i < (int)stride * h`. Note that two negative dimensions
        // produce a *positive* bound, so the loop really does run in that case.
        let limit = w.wrapping_mul(4).wrapping_mul(h);
        if limit <= 0 {
            let mut buf = pixels.clone();
            let mut img = CpImage {
                w,
                h,
                pix: buf.as_mut_ptr(),
            };
            unsafe { (impls.rust)(&mut img) };
            assert_eq!(
                as_bytes(&buf),
                as_bytes(&pixels),
                "degenerate {w}x{h}: Rust wrote to the buffer but the C loop bound is <= 0"
            );
        }
    }
}

/// Calling twice in a row must converge the same way in both implementations
/// (premultiply is not idempotent, so this catches accumulated drift).
#[test]
fn repeated_application() {
    let impls = Impls::load();
    let mut rng = Rng(0x0123_4567_89AB_CDEF);

    let w: c_int = 40;
    let h: c_int = 40;
    let n = (w as usize) * (h as usize);

    let mut c_buf: Vec<CpPixel> = (0..n)
        .map(|_| CpPixel {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
            a: rng.next_u8(),
        })
        .collect();
    let mut rust_buf = c_buf.clone();

    let mut c_img = CpImage {
        w,
        h,
        pix: c_buf.as_mut_ptr(),
    };
    let mut rust_img = CpImage {
        w,
        h,
        pix: rust_buf.as_mut_ptr(),
    };

    for pass in 0..10 {
        unsafe {
            (impls.c)(&mut c_img);
            (impls.rust)(&mut rust_img);
        }
        assert_eq!(
            as_bytes(&c_buf),
            as_bytes(&rust_buf),
            "divergence after pass {pass}"
        );
    }
}

/// The buffer may be larger than `w * h`; the trailing pixels must not be
/// touched by either implementation.
#[test]
fn does_not_write_past_the_loop_bound() {
    let impls = Impls::load();
    let mut rng = Rng(0xFEED_FACE_5678_9ABC);

    let w: c_int = 10;
    let h: c_int = 10;
    let used = (w as usize) * (h as usize);
    let total = used + 37;

    let pixels: Vec<CpPixel> = (0..total)
        .map(|_| CpPixel {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
            a: rng.next_u8(),
        })
        .collect();

    let mut c_buf = pixels.clone();
    let mut rust_buf = pixels.clone();

    let mut c_img = CpImage {
        w,
        h,
        pix: c_buf.as_mut_ptr(),
    };
    let mut rust_img = CpImage {
        w,
        h,
        pix: rust_buf.as_mut_ptr(),
    };

    unsafe {
        (impls.c)(&mut c_img);
        (impls.rust)(&mut rust_img);
    }

    assert_eq!(as_bytes(&c_buf), as_bytes(&rust_buf), "full buffer mismatch");
    assert_eq!(
        as_bytes(&c_buf[used..]),
        as_bytes(&pixels[used..]),
        "C wrote past the loop bound (unexpected)"
    );
    assert_eq!(
        as_bytes(&rust_buf[used..]),
        as_bytes(&pixels[used..]),
        "Rust wrote past the loop bound"
    );
}

/// Struct layout sanity: the FFI ABI must be what the C header describes.
#[test]
fn struct_layout_matches_c_abi() {
    assert_eq!(std::mem::size_of::<CpPixel>(), 4);
    assert_eq!(std::mem::align_of::<CpPixel>(), 1);
    assert_eq!(
        std::mem::size_of::<CpImage>(),
        2 * std::mem::size_of::<c_int>() + std::mem::size_of::<*mut CpPixel>()
    );
}
