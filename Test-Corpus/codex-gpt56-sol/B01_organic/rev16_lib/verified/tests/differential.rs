use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Rev16 = unsafe extern "C" fn(u32) -> u32;

fn shared_library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_root.join("c_src/build/libtranslated_rust.so");
    let target_root = option_env!("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root.join("target"));
    let rust_candidates = [
        target_root.join("debug/librev16_lib.so"),
        target_root.join("debug/deps/librev16_lib.so"),
    ];
    let rust_library = rust_candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| target_root.join("debug/librev16_lib.so"));
    (c_library, rust_library)
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "required shared library does not exist: {}",
        path.display()
    );
}

fn compare(c_rev16: Rev16, rust_rev16: Rev16, input: u32) {
    // Both calls cross a dynamically loaded C ABI boundary.
    let c_output = unsafe { c_rev16(input) };
    let rust_output = unsafe { rust_rev16(input) };
    assert_eq!(
        c_output.to_ne_bytes(),
        rust_output.to_ne_bytes(),
        "rev16 output differs for input {input:#010x}"
    );
}

#[test]
fn rev16_matches_c_for_complete_configuration_surface() {
    let (c_path, rust_path) = shared_library_paths();
    assert_library_exists(&c_path);
    assert_library_exists(&rust_path);

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_rev16: Symbol<Rev16> = unsafe { c_library.get(b"rev16\0") }.expect("load C rev16");
    let rust_rev16: Symbol<Rev16> =
        unsafe { rust_library.get(b"rev16\0") }.expect("load Rust rev16");

    for input in [
        0,
        1,
        u16::MAX as u32,
        (u16::MAX as u32) + 1,
        0x8000_0000,
        u32::MAX,
    ] {
        compare(*c_rev16, *rust_rev16, input);
    }

    // Exercise every low-16-bit payload, alternating clear and nonzero upper bits.
    for low in 0..=u16::MAX as u32 {
        let upper = if low & 1 == 0 { 0 } else { 0xA5A5_0000 };
        compare(*c_rev16, *rust_rev16, upper | low);
    }

    // Fixed-seed xorshift32 supplies reproducible, full-width property cases.
    let mut state = 0x6D2B_79F5_u32;
    for _ in 0..100_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        compare(*c_rev16, *rust_rev16, state);
    }
}
