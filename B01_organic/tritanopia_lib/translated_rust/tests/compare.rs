use libloading::{Library, Symbol};
use tritanopia_lib::{cb_rgb_255, tritanopia};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtritanopia_lib.so");

type TritanopiaFn = unsafe extern "C" fn(cb_rgb_255) -> cb_rgb_255;

fn load_c_tritanopia() -> (Library, TritanopiaFn) {
    let lib = unsafe { Library::new(C_LIB_PATH) }.expect("Failed to load C .so");
    let func: Symbol<TritanopiaFn> = unsafe { lib.get(b"tritanopia") }.expect("symbol not found");
    let func = *func;
    (lib, func)
}

#[test]
fn test_tritanopia_all_byte_combos() {
    let (_lib, c_fn) = load_c_tritanopia();
    let mut mismatches = Vec::new();

    // Test a representative grid of all R,G,B values (every 5th value + boundaries)
    let vals: Vec<u8> = (0..=255u16).filter(|v| v % 5 == 0 || *v == 255).map(|v| v as u8).collect();

    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                let input = cb_rgb_255 { R: r, G: g, B: b };
                let c_out = unsafe { c_fn(cb_rgb_255 { R: r, G: g, B: b }) };
                let rust_out = tritanopia(input);
                if c_out.R != rust_out.R || c_out.G != rust_out.G || c_out.B != rust_out.B {
                    mismatches.push((r, g, b, c_out, rust_out));
                    if mismatches.len() >= 20 {
                        break;
                    }
                }
            }
            if mismatches.len() >= 20 { break; }
        }
        if mismatches.len() >= 20 { break; }
    }

    if !mismatches.is_empty() {
        for (r, g, b, c, rs) in &mismatches {
            eprintln!(
                "MISMATCH input=({},{},{}) C=({},{},{}) Rust=({},{},{})",
                r, g, b, c.R, c.G, c.B, rs.R, rs.G, rs.B
            );
        }
        panic!("{} mismatches found (showing first 20)", mismatches.len());
    }
}

#[test]
fn test_tritanopia_specific_values() {
    let (_lib, c_fn) = load_c_tritanopia();

    let test_cases: Vec<(u8, u8, u8)> = vec![
        (0, 0, 0), (255, 255, 255), (255, 0, 0), (0, 255, 0), (0, 0, 255),
        (128, 128, 128), (1, 1, 1), (254, 254, 254), (100, 200, 50),
    ];

    for (r, g, b) in test_cases {
        let input = cb_rgb_255 { R: r, G: g, B: b };
        let c_out = unsafe { c_fn(cb_rgb_255 { R: r, G: g, B: b }) };
        let rust_out = tritanopia(input);
        assert_eq!(
            (c_out.R, c_out.G, c_out.B),
            (rust_out.R, rust_out.G, rust_out.B),
            "Mismatch for input ({},{},{}): C=({},{},{}) Rust=({},{},{})",
            r, g, b, c_out.R, c_out.G, c_out.B, rust_out.R, rust_out.G, rust_out.B
        );
    }
}
