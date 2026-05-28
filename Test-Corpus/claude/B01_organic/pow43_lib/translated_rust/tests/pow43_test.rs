use libloading::{Library, Symbol};
use std::path::PathBuf;

type Pow43Fn = unsafe extern "C" fn(i32) -> f32;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Determine debug or release - default cargo test uses target/debug
    let candidates = [
        workspace_root().join("target/debug/libpow43_lib.so"),
        workspace_root().join("target/release/libpow43_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust .so not found, tried: {:?}", candidates);
}

unsafe fn load_pow43(path: &std::path::Path) -> (Library, Symbol<'static, Pow43Fn>) {
    let lib = Library::new(path).unwrap_or_else(|e| {
        panic!("Failed to load {}: {}", path.display(), e);
    });
    // Extend symbol lifetime by transmuting; we'll keep the library alive in tuple
    let sym: Symbol<Pow43Fn> = lib.get(b"pow43\0").expect("pow43 symbol missing");
    let sym_static: Symbol<'static, Pow43Fn> = std::mem::transmute(sym);
    (lib, sym_static)
}

fn compare_for(x: i32) {
    unsafe {
        let (_clib, c_sym) = load_pow43(&c_lib_path());
        let (_rlib, r_sym) = load_pow43(&rust_lib_path());
        let c_val = c_sym(x);
        let r_val = r_sym(x);
        assert_eq!(
            c_val.to_bits(),
            r_val.to_bits(),
            "Mismatch at x={}: C={} ({:#x}) Rust={} ({:#x})",
            x,
            c_val,
            c_val.to_bits(),
            r_val,
            r_val.to_bits()
        );
    }
}

fn compare_range(values: impl IntoIterator<Item = i32>) {
    unsafe {
        let (_clib, c_sym) = load_pow43(&c_lib_path());
        let (_rlib, r_sym) = load_pow43(&rust_lib_path());
        for x in values {
            let c_val = c_sym(x);
            let r_val = r_sym(x);
            assert_eq!(
                c_val.to_bits(),
                r_val.to_bits(),
                "Mismatch at x={}: C={} ({:#x}) Rust={} ({:#x})",
                x,
                c_val,
                c_val.to_bits(),
                r_val,
                r_val.to_bits()
            );
        }
    }
}

#[test]
fn test_pow43_low_table_indices() {
    // 0..129 hits the table directly via g_pow43[16 + x]
    compare_range(0..129);
}

#[test]
fn test_pow43_negative_table_indices() {
    // x in -16..0 still in table (16 + x is in 0..16)
    // Note: C code: if (x < 129) return g_pow43[16 + x]
    // For x in -16..0, this still loads from table (negative entries)
    compare_range(-16..0);
}

#[test]
fn test_pow43_mid_range_below_1024() {
    // 129..1024 takes mult=16, x <<= 3 path
    compare_range(129..1024);
}

#[test]
fn test_pow43_high_range() {
    // x >= 1024 takes mult=256 path.
    // The g_pow43 table only has 145 elements; the algorithm requires
    // (x+sign) <= 8255 for the index 16 + ((x+sign)>>6) to be in-bounds.
    // Beyond that is undefined behavior in C (reading past static array),
    // so we limit the test to the algorithm's valid input domain.
    compare_range(1024..8192);
}

#[test]
fn test_pow43_specific_boundaries() {
    let boundaries = [
        0, 1, 2, 127, 128, 129, 130, 1023, 1024, 1025, 4096, 8000, 8191,
    ];
    for &x in &boundaries {
        compare_for(x);
    }
}
