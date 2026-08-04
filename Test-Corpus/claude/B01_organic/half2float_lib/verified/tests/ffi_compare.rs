// Integration test: compare C and Rust shared libraries through FFI
// Both libraries are loaded with libloading and `half2float` is invoked from
// each. Their bit patterns must match for every u16 input.

use libloading::{Library, Symbol};
use std::path::PathBuf;

type Half2FloatFn = unsafe extern "C" fn(u16) -> f32;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    // Cargo places integration test artifacts in target/<profile>/deps; the
    // cdylib is in target/<profile>/. Determine which profile we're in.
    let mut candidates = Vec::new();
    let root = workspace_root();
    candidates.push(root.join("target/release/libhalf2float_lib.so"));
    candidates.push(root.join("target/debug/libhalf2float_lib.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Could not find libhalf2float_lib.so in target/release or target/debug. \
         Run `cargo build` first."
    );
}

unsafe fn load_half2float(lib: &Library) -> Symbol<'_, Half2FloatFn> {
    unsafe { lib.get(b"half2float\0").expect("half2float symbol not found") }
}

#[test]
fn half2float_all_values_match() {
    let c_lib_path = c_library_path();
    assert!(
        c_lib_path.exists(),
        "C shared library not found at {:?}. Build with cmake first.",
        c_lib_path
    );
    let r_lib_path = rust_library_path();

    unsafe {
        let c_lib = Library::new(&c_lib_path).expect("failed to load C lib");
        let r_lib = Library::new(&r_lib_path).expect("failed to load Rust lib");
        let c_fn = load_half2float(&c_lib);
        let r_fn = load_half2float(&r_lib);

        // Exhaustively compare every possible 16-bit input.
        for h in 0u32..=0xFFFFu32 {
            let h = h as u16;
            let c_out = c_fn(h);
            let r_out = r_fn(h);
            let c_bits = c_out.to_bits();
            let r_bits = r_out.to_bits();
            assert_eq!(
                c_bits, r_bits,
                "mismatch for input 0x{:04x}: C=0x{:08x} Rust=0x{:08x}",
                h, c_bits, r_bits
            );
        }
    }
}
