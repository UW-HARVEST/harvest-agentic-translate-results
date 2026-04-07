use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy)]
struct CbRgb255 {
    r: u8,
    g: u8,
    b: u8,
}

type ContrastRatioFn = unsafe extern "C" fn(CbRgb255, CbRgb255) -> f32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libcontrast_ratio_lib.so")
}

fn load_contrast_ratio(lib: &Library) -> Symbol<ContrastRatioFn> {
    unsafe { lib.get(b"contrast_ratio") }.expect("symbol not found")
}

fn rgb(r: u8, g: u8, b: u8) -> CbRgb255 {
    CbRgb255 { r, g, b }
}

#[test]
fn contrast_ratio_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
    let rs_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
    let c_fn = load_contrast_ratio(&c_lib);
    let rs_fn = load_contrast_ratio(&rs_lib);

    let test_pairs: Vec<(CbRgb255, CbRgb255)> = vec![
        // black vs white
        (rgb(0, 0, 0), rgb(255, 255, 255)),
        // same color
        (rgb(128, 128, 128), rgb(128, 128, 128)),
        // red vs blue
        (rgb(255, 0, 0), rgb(0, 0, 255)),
        // near-threshold values around 0.04045 * 255 ≈ 10.3
        (rgb(10, 10, 10), rgb(11, 11, 11)),
        // all zeros
        (rgb(0, 0, 0), rgb(0, 0, 0)),
        // extremes
        (rgb(0, 0, 0), rgb(1, 1, 1)),
        (rgb(254, 254, 254), rgb(255, 255, 255)),
        // asymmetric
        (rgb(100, 200, 50), rgb(30, 60, 90)),
        // single channel diffs
        (rgb(255, 0, 0), rgb(0, 255, 0)),
        (rgb(0, 255, 0), rgb(0, 0, 255)),
        // all 255 vs all 0
        (rgb(255, 255, 255), rgb(0, 0, 0)),
    ];

    for (i, (a, b)) in test_pairs.iter().enumerate() {
        let c_result = unsafe { c_fn(*a, *b) };
        let rs_result = unsafe { rs_fn(*a, *b) };
        assert!(
            c_result.to_bits() == rs_result.to_bits(),
            "Mismatch at test {i}: C={c_result} (bits={:#010x}), Rust={rs_result} (bits={:#010x}), \
             A=({},{},{}), B=({},{},{})",
            c_result.to_bits(), rs_result.to_bits(),
            a.r, a.g, a.b, b.r, b.g, b.b
        );
    }
}

/// Exhaustive test: sweep a subset of RGB space
#[test]
fn contrast_ratio_sweep() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
    let rs_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
    let c_fn = load_contrast_ratio(&c_lib);
    let rs_fn = load_contrast_ratio(&rs_lib);

    // Test every 51st value (0,51,102,153,204,255) = 6^6 = 46656 combos
    let vals: Vec<u8> = (0..=5).map(|i| i * 51).collect();
    let mut mismatches = 0;
    for &r1 in &vals {
        for &g1 in &vals {
            for &b1 in &vals {
                for &r2 in &vals {
                    for &g2 in &vals {
                        for &b2 in &vals {
                            let a = rgb(r1, g1, b1);
                            let b = rgb(r2, g2, b2);
                            let c_r = unsafe { c_fn(a, b) };
                            let rs_r = unsafe { rs_fn(a, b) };
                            if c_r.to_bits() != rs_r.to_bits() {
                                if mismatches < 5 {
                                    eprintln!(
                                        "MISMATCH: ({r1},{g1},{b1}) vs ({r2},{g2},{b2}): C={c_r} Rust={rs_r}"
                                    );
                                }
                                mismatches += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(mismatches == 0, "{mismatches} mismatches found in sweep");
}
