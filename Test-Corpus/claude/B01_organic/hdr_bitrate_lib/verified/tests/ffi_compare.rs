use libloading::{Library, Symbol};
use std::os::raw::c_uint;

type HdrBitrateFn = unsafe extern "C" fn(*const u8) -> c_uint;

fn load_libs() -> (Library, Library) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/build/libtranslated_rust.so", manifest);
    let rust_path = format!("{}/target/release/libhdr_bitrate_lib.so", manifest);
    unsafe {
        let c = Library::new(&c_path).expect("failed to load C lib");
        let r = Library::new(&rust_path).expect("failed to load Rust lib");
        (c, r)
    }
}

#[test]
fn compare_hdr_bitrate_all_valid_inputs() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<HdrBitrateFn> = unsafe { c_lib.get(b"hdr_bitrate").unwrap() };
    let r_fn: Symbol<HdrBitrateFn> = unsafe { r_lib.get(b"hdr_bitrate").unwrap() };

    // The function reads h[1] and h[2]. To avoid undefined behavior in either
    // implementation, only test inputs where the table indices are in-bounds:
    //   j = ((h[1] >> 1) & 3) - 1  must be in 0..3  => (h[1] & 0x6) != 0
    //   k = (h[2] >> 4)            must be in 0..15 => h[2] < 0xF0
    for h1 in 0u16..=255 {
        let h1 = h1 as u8;
        if (h1 & 0x6) == 0 {
            continue;
        }
        for h2 in 0u16..=255 {
            let h2 = h2 as u8;
            if h2 >= 0xF0 {
                continue;
            }
            let buf = [0u8, h1, h2, 0u8];
            let c_out = unsafe { c_fn(buf.as_ptr()) };
            let r_out = unsafe { r_fn(buf.as_ptr()) };
            assert_eq!(
                c_out, r_out,
                "mismatch for h1={:#x}, h2={:#x}: C={}, Rust={}",
                h1, h2, c_out, r_out
            );
        }
    }
}

#[test]
fn export_symbols_match() {
    let (c_lib, r_lib) = load_libs();
    let _: Symbol<HdrBitrateFn> = unsafe { c_lib.get(b"hdr_bitrate").unwrap() };
    let _: Symbol<HdrBitrateFn> = unsafe { r_lib.get(b"hdr_bitrate").unwrap() };
}
