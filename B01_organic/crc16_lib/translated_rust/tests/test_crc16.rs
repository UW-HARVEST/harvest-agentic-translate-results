use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libcrc16_lib.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Find the built cdylib in target/debug or target/release
    let debug = manifest.join("target/debug/libcrc16_lib.so");
    if debug.exists() {
        return debug;
    }
    manifest.join("target/release/libcrc16_lib.so")
}

fn call_crc16(lib: &Library, data: &[u8], init: u16) -> u16 {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*const u8, u32, u16) -> u16> =
            lib.get(b"crc16").unwrap();
        func(data.as_ptr(), data.len() as u32, init)
    }
}

fn with_libs(f: impl Fn(&Library, &Library)) {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    f(&c_lib, &r_lib);
}

#[test]
fn test_empty_input() {
    with_libs(|c, r| {
        for init in [0u16, 0xFFFF, 0x1234] {
            let cv = call_crc16(c, &[], init);
            let rv = call_crc16(r, &[], init);
            assert_eq!(cv, rv, "empty input, init=0x{init:04x}: C=0x{cv:04x} Rust=0x{rv:04x}");
        }
    });
}

#[test]
fn test_single_byte() {
    with_libs(|c, r| {
        for b in 0..=255u8 {
            let cv = call_crc16(c, &[b], 0);
            let rv = call_crc16(r, &[b], 0);
            assert_eq!(cv, rv, "byte {b}: C=0x{cv:04x} Rust=0x{rv:04x}");
        }
    });
}

#[test]
fn test_short_inputs() {
    with_libs(|c, r| {
        for len in 1..8usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 13) as u8).collect();
            let cv = call_crc16(c, &data, 0);
            let rv = call_crc16(r, &data, 0);
            assert_eq!(cv, rv, "len={len}: C=0x{cv:04x} Rust=0x{rv:04x}");
        }
    });
}

#[test]
fn test_exact_8_bytes() {
    with_libs(|c, r| {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        for init in [0u16, 0xFFFF, 0xABCD] {
            let cv = call_crc16(c, &data, init);
            let rv = call_crc16(r, &data, init);
            assert_eq!(cv, rv, "8 bytes init=0x{init:04x}: C=0x{cv:04x} Rust=0x{rv:04x}");
        }
    });
}

#[test]
fn test_longer_inputs() {
    with_libs(|c, r| {
        for len in [9, 15, 16, 17, 31, 32, 64, 100, 255, 1024] {
            let data: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(7).wrapping_add(3)).collect();
            for init in [0u16, 0xFFFF] {
                let cv = call_crc16(c, &data, init);
                let rv = call_crc16(r, &data, init);
                assert_eq!(cv, rv, "len={len} init=0x{init:04x}: C=0x{cv:04x} Rust=0x{rv:04x}");
            }
        }
    });
}

#[test]
fn test_all_zeros_and_ones() {
    with_libs(|c, r| {
        for len in [8, 16, 24] {
            let zeros = vec![0u8; len];
            let ones = vec![0xFFu8; len];
            let cv = call_crc16(c, &zeros, 0);
            let rv = call_crc16(r, &zeros, 0);
            assert_eq!(cv, rv, "zeros len={len}: C=0x{cv:04x} Rust=0x{rv:04x}");
            let cv = call_crc16(c, &ones, 0);
            let rv = call_crc16(r, &ones, 0);
            assert_eq!(cv, rv, "ones len={len}: C=0x{cv:04x} Rust=0x{rv:04x}");
        }
    });
}

#[test]
fn test_nonzero_initial_crc() {
    with_libs(|c, r| {
        let data: Vec<u8> = (0..20).collect();
        for init in [0x0001u16, 0x8000, 0x7FFF, 0xFFFF, 0x1234, 0xDEAD] {
            let cv = call_crc16(c, &data, init);
            let rv = call_crc16(r, &data, init);
            assert_eq!(cv, rv, "init=0x{init:04x}: C=0x{cv:04x} Rust=0x{rv:04x}");
        }
    });
}
