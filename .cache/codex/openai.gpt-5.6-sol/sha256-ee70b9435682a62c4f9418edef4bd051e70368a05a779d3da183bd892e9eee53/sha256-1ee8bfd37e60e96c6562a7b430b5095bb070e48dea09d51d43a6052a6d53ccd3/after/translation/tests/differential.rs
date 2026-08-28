use libloading::Library;
use std::path::{Path, PathBuf};

type Rev16 = unsafe extern "C" fn(u32) -> u32;

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-Nek5bk.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/librev16_lib.so")
}

unsafe fn load_rev16(library: &Library) -> Rev16 {
    unsafe {
        *library
            .get::<Rev16>(b"rev16\0")
            .expect("load exported rev16 symbol")
    }
}

fn assert_same_bytes(c_rev16: Rev16, rust_rev16: Rev16, input: u32) {
    let c_result = unsafe { c_rev16(input) };
    let rust_result = unsafe { rust_rev16(input) };

    assert_eq!(
        rust_result.to_ne_bytes(),
        c_result.to_ne_bytes(),
        "rev16 output differs for input 0x{input:08x}: C=0x{c_result:08x}, Rust=0x{rust_result:08x}"
    );
}

#[test]
fn config_1_rev16_matches_across_full_width_inputs() {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load the C shared library");
    let rust_library =
        unsafe { Library::new(rust_library_path()) }.expect("load the Rust shared library");
    let c_rev16 = unsafe { load_rev16(&c_library) };
    let rust_rev16 = unsafe { load_rev16(&rust_library) };

    let boundaries = [
        0,
        1,
        2,
        0x7fff,
        0x8000,
        0xffff,
        0x0001_0000,
        0x7fff_ffff,
        0x8000_0000,
        0xaaaa_5555,
        0x5555_aaaa,
        u32::MAX,
    ];
    for input in boundaries {
        assert_same_bytes(c_rev16, rust_rev16, input);
    }

    // Fixed-seed xorshift32 covers value-dependent behavior reproducibly.
    let mut input = 0x6d2b_79f5_u32;
    for _ in 0..100_000 {
        input ^= input << 13;
        input ^= input >> 17;
        input ^= input << 5;
        assert_same_bytes(c_rev16, rust_rev16, input);
    }
}
