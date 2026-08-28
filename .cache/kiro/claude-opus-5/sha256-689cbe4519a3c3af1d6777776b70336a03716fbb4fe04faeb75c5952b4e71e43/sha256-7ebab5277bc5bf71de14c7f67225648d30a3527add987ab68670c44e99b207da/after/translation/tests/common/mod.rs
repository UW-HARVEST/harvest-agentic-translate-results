//! Shared harness: loads the C reference `.so` and the Rust `.so` and calls
//! both purely through their exported C ABI symbols.

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

pub type GaussianKernelFn = unsafe extern "C" fn(*mut f32, c_int, f32);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, pred: &dyn Fn(&str) -> bool) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.ends_with(".so") && pred(&name) {
            return Some(p);
        }
    }
    None
}

/// Path to the C reference shared library (name derives from the parent
/// directory name in `c_src/CMakeLists.txt`, so it is discovered by scanning).
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    find_so(&build, &|_| true).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib`, searched in both cargo profiles.
pub fn rust_so_path() -> PathBuf {
    let target = manifest_dir().join("target");
    let pred = |n: &str| n.contains("gaussian_kernel_lib");
    // Prefer the profile this test binary was built with, so `cargo test` and
    // `cargo test --release` each exercise their own cdylib.
    let profiles: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in profiles {
        if let Some(p) = find_so(&target.join(profile), &pred) {
            return p;
        }
    }
    panic!(
        "libgaussian_kernel_lib.so not found under {}; run `cargo build` first",
        target.display()
    );
}

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: GaussianKernelFn,
    pub rust: GaussianKernelFn,
}

impl Impls {
    pub fn load() -> Impls {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");
            let c_sym: Symbol<GaussianKernelFn> =
                c_lib.get(b"gaussian_kernel\0").expect("C gaussian_kernel");
            let rust_sym: Symbol<GaussianKernelFn> = rust_lib
                .get(b"gaussian_kernel\0")
                .expect("Rust gaussian_kernel");
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
}

/// Slack elements kept past `size`: the C original writes
/// `2 * (size / 2) + 1` elements, i.e. one past `size` for even `size`.
pub const SLACK: usize = 8;

/// Sentinel pre-fill so that untouched slots are compared too.
fn prefill(buf: &mut [f32]) {
    for (i, s) in buf.iter_mut().enumerate() {
        *s = f32::from_bits(0xDEAD_0000u32 ^ (i as u32));
    }
}

fn bits(buf: &[f32]) -> Vec<u32> {
    buf.iter().map(|v| v.to_bits()).collect()
}

/// Runs both implementations on identical buffers and asserts bit-identical
/// results across the whole allocation (including the slack region).
pub fn assert_same(impls: &Impls, size: c_int, radius: f32) {
    let cap = size.max(0) as usize + SLACK;
    let mut c_buf = vec![0.0f32; cap];
    let mut rust_buf = vec![0.0f32; cap];
    prefill(&mut c_buf);
    prefill(&mut rust_buf);

    unsafe {
        (impls.c)(c_buf.as_mut_ptr(), size, radius);
        (impls.rust)(rust_buf.as_mut_ptr(), size, radius);
    }

    let (cb, rb) = (bits(&c_buf), bits(&rust_buf));
    if cb != rb {
        let idx = cb.iter().zip(&rb).position(|(a, b)| a != b).unwrap();
        panic!(
            "mismatch for size={size}, radius={radius:?} (bits 0x{:08x}) at index {idx}:\n  \
             C    = {:?} (0x{:08x})\n  Rust = {:?} (0x{:08x})",
            radius.to_bits(),
            c_buf[idx],
            cb[idx],
            rust_buf[idx],
            rb[idx],
        );
    }
}
