use libloading::{Library, Symbol};
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path).expect("Failed to load C .so") }
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/libhalf2float_lib.so");
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

#[test]
fn test_half2float_exhaustive() {
    let c_lib = load_c_lib();
    let rust_lib = load_rust_lib();

    let c_fn: Symbol<unsafe extern "C" fn(u16) -> f32> =
        unsafe { c_lib.get(b"half2float").unwrap() };
    let rust_fn: Symbol<unsafe extern "C" fn(u16) -> f32> =
        unsafe { rust_lib.get(b"half2float").unwrap() };

    let mut mismatches = Vec::new();
    for h in 0u16..=u16::MAX {
        let c_val = unsafe { c_fn(h) };
        let rust_val = unsafe { rust_fn(h) };
        let c_bits = c_val.to_bits();
        let rust_bits = rust_val.to_bits();
        if c_bits != rust_bits {
            mismatches.push((h, c_bits, rust_bits));
            if mismatches.len() >= 20 {
                break;
            }
        }
    }
    if !mismatches.is_empty() {
        for (h, c, r) in &mismatches {
            eprintln!("MISMATCH h=0x{h:04x}: C=0x{c:08x} Rust=0x{r:08x}");
        }
        panic!("{} mismatches found (showing first 20)", mismatches.len());
    }
}
