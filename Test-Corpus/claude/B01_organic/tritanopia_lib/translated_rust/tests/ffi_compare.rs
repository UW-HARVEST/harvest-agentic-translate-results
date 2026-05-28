use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CbRgb255 {
    r: u8,
    g: u8,
    b: u8,
}

type TritanopiaFn = unsafe extern "C" fn(CbRgb255) -> CbRgb255;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    project_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    // Use the cdylib produced by cargo for this crate.
    let mut target = project_root().join("target/release/libtritanopia_lib.so");
    if !target.exists() {
        target = project_root().join("target/debug/libtritanopia_lib.so");
    }
    target
}

#[test]
fn tritanopia_matches_c_for_all_inputs() {
    let c_path = c_so_path();
    let rust_path = rust_so_path();
    assert!(
        c_path.exists(),
        "C shared library not found at {:?}. Build with cmake first.",
        c_path
    );
    assert!(
        rust_path.exists(),
        "Rust shared library not found at {:?}. Build with `cargo build` first.",
        rust_path
    );

    unsafe {
        let c_lib = Library::new(&c_path).expect("load C lib");
        let rust_lib = Library::new(&rust_path).expect("load Rust lib");

        let c_fn: Symbol<TritanopiaFn> = c_lib.get(b"tritanopia\0").expect("C tritanopia symbol");
        let rust_fn: Symbol<TritanopiaFn> =
            rust_lib.get(b"tritanopia\0").expect("Rust tritanopia symbol");

        let mut mismatches = 0u64;
        let mut first_mismatch: Option<(CbRgb255, CbRgb255, CbRgb255)> = None;

        // Exhaustive sweep across the entire 24-bit RGB cube.
        // 16,777,216 calls -> still cheap (a few seconds).
        for r in 0u8..=255 {
            for g in 0u8..=255 {
                for b in 0u8..=255 {
                    let input = CbRgb255 { r, g, b };
                    let c_out = c_fn(input);
                    let rust_out = rust_fn(input);
                    if c_out != rust_out {
                        if first_mismatch.is_none() {
                            first_mismatch = Some((input, c_out, rust_out));
                        }
                        mismatches += 1;
                    }
                }
            }
        }

        if let Some((inp, c_out, rust_out)) = first_mismatch {
            panic!(
                "Found {} mismatches. First: input=({},{},{}) c=({},{},{}) rust=({},{},{})",
                mismatches, inp.r, inp.g, inp.b, c_out.r, c_out.g, c_out.b, rust_out.r, rust_out.g, rust_out.b
            );
        }
    }
}
