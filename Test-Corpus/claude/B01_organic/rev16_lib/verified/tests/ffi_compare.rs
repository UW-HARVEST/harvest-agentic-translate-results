use libloading::{Library, Symbol};

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    // The rust .so is built as librev16_lib.so as cdylib in target/debug
    // The integration test runs with CARGO_MANIFEST_DIR set to the crate root.
    // We try debug first, then release.
    if std::path::Path::new("target/debug/librev16_lib.so").exists() {
        "target/debug/librev16_lib.so"
    } else {
        "target/release/librev16_lib.so"
    }
}

type Rev16Fn = unsafe extern "C" fn(u32) -> u32;

#[test]
fn compare_rev16_basic() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rev16: Symbol<Rev16Fn> = c_lib.get(b"rev16").expect("symbol rev16 in C");
        let rust_rev16: Symbol<Rev16Fn> = rust_lib.get(b"rev16").expect("symbol rev16 in Rust");

        // Test specific known values
        let cases: &[u32] = &[
            0u32,
            1,
            0xFFFF,
            0x8000,
            0x0001,
            0xAAAA,
            0x5555,
            0xCCCC,
            0x3333,
            0xF0F0,
            0x0F0F,
            0xFF00,
            0x00FF,
            0x1234,
            0xDEAD,
            0xBEEF,
            0xCAFE,
            0xBABE,
            0xFFFFFFFF,
            0x12345678,
            0x80000000,
        ];

        for &v in cases {
            let c_out = c_rev16(v);
            let r_out = rust_rev16(v);
            assert_eq!(
                c_out, r_out,
                "mismatch for input 0x{:08X}: C=0x{:08X}, Rust=0x{:08X}",
                v, c_out, r_out
            );
        }
    }
}

#[test]
fn compare_rev16_all_16bit() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rev16: Symbol<Rev16Fn> = c_lib.get(b"rev16").expect("symbol rev16 in C");
        let rust_rev16: Symbol<Rev16Fn> = rust_lib.get(b"rev16").expect("symbol rev16 in Rust");

        for v in 0u32..=0xFFFFu32 {
            let c_out = c_rev16(v);
            let r_out = rust_rev16(v);
            assert_eq!(
                c_out, r_out,
                "mismatch for input 0x{:08X}: C=0x{:08X}, Rust=0x{:08X}",
                v, c_out, r_out
            );
        }
    }
}

#[test]
fn compare_rev16_random_high_bits() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rev16: Symbol<Rev16Fn> = c_lib.get(b"rev16").expect("symbol rev16 in C");
        let rust_rev16: Symbol<Rev16Fn> = rust_lib.get(b"rev16").expect("symbol rev16 in Rust");

        // Pseudo-random LCG for deterministic coverage of upper bits.
        let mut state: u64 = 0xDEADBEEFCAFEBABE;
        for _ in 0..100_000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = (state >> 32) as u32;
            let c_out = c_rev16(v);
            let r_out = rust_rev16(v);
            assert_eq!(
                c_out, r_out,
                "mismatch for input 0x{:08X}: C=0x{:08X}, Rust=0x{:08X}",
                v, c_out, r_out
            );
        }
    }
}
