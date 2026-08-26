use libloading::{Library, Symbol};
use std::path::PathBuf;

type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

fn lib_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest.join("c_src/build/libtranslated_rust.so");
    let rust_so = manifest.join("target/debug/libcrc16_lib.so");
    assert!(c_so.exists(), "C .so not found at {}", c_so.display());
    assert!(rust_so.exists(), "Rust .so not found at {}", rust_so.display());
    (c_so, rust_so)
}

fn call_crc16(lib: &Library, data: &[u8], init: u16) -> u16 {
    unsafe {
        let func: Symbol<Crc16Fn> = lib.get(b"crc16").unwrap();
        func(data.as_ptr(), data.len() as u32, init)
    }
}

fn compare(data: &[u8], init: u16, c_lib: &Library, rs_lib: &Library) {
    let c_result = call_crc16(c_lib, data, init);
    let rs_result = call_crc16(rs_lib, data, init);
    assert_eq!(
        c_result, rs_result,
        "Mismatch for len={} init=0x{:04x}: C=0x{:04x} Rust=0x{:04x}",
        data.len(), init, c_result, rs_result
    );
}

#[test]
fn test_crc16_empty() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        compare(&[], 0, &c_lib, &rs_lib);
        compare(&[], 0xFFFF, &c_lib, &rs_lib);
    }
}

#[test]
fn test_crc16_single_bytes() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for b in [0u8, 1, 0x7F, 0x80, 0xFF] {
            compare(&[b], 0, &c_lib, &rs_lib);
            compare(&[b], 0xFFFF, &c_lib, &rs_lib);
        }
    }
}

#[test]
fn test_crc16_short_sequences() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        // Lengths 1-7 exercise only the byte-at-a-time loop
        for len in 1..8u8 {
            let data: Vec<u8> = (0..len).collect();
            compare(&data, 0, &c_lib, &rs_lib);
        }
    }
}

#[test]
fn test_crc16_exact_8_bytes() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        // Exactly 8 bytes: one iteration of the unrolled loop, no remainder
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        compare(&data, 0, &c_lib, &rs_lib);
        compare(&data, 0x1234, &c_lib, &rs_lib);
    }
}

#[test]
fn test_crc16_longer_data() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        // 16 bytes: two unrolled iterations
        let data: Vec<u8> = (0..16).collect();
        compare(&data, 0, &c_lib, &rs_lib);
        // 13 bytes: one unrolled + 5 byte-at-a-time
        let data: Vec<u8> = (0..13).collect();
        compare(&data, 0, &c_lib, &rs_lib);
        // 100 bytes
        let data: Vec<u8> = (0..100).map(|i| (i * 37 + 13) as u8).collect();
        compare(&data, 0, &c_lib, &rs_lib);
        compare(&data, 0xBEEF, &c_lib, &rs_lib);
    }
}

#[test]
fn test_crc16_all_zeros_and_ones() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        let zeros = vec![0u8; 64];
        let ones = vec![0xFFu8; 64];
        compare(&zeros, 0, &c_lib, &rs_lib);
        compare(&ones, 0, &c_lib, &rs_lib);
        compare(&zeros, 0xFFFF, &c_lib, &rs_lib);
        compare(&ones, 0xFFFF, &c_lib, &rs_lib);
    }
}

#[test]
fn test_crc16_incremental() {
    // Feed data byte-by-byte using the previous CRC as init, compare with single-shot
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        let data: Vec<u8> = (0..32).map(|i| (i * 7) as u8).collect();
        // Single-shot
        let c_full = call_crc16(&c_lib, &data, 0);
        let rs_full = call_crc16(&rs_lib, &data, 0);
        assert_eq!(c_full, rs_full);
        // Incremental via C
        let mut c_inc: u16 = 0;
        for b in &data {
            c_inc = call_crc16(&c_lib, std::slice::from_ref(b), c_inc);
        }
        assert_eq!(c_full, c_inc, "C incremental != C full");
        // Incremental via Rust
        let mut rs_inc: u16 = 0;
        for b in &data {
            rs_inc = call_crc16(&rs_lib, std::slice::from_ref(b), rs_inc);
        }
        assert_eq!(rs_full, rs_inc, "Rust incremental != Rust full");
    }
}

#[test]
fn test_crc16_large_data() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        // 4096 bytes of pseudo-random data
        let data: Vec<u8> = (0..4096u32).map(|i| ((i.wrapping_mul(2654435761)) >> 16) as u8).collect();
        compare(&data, 0, &c_lib, &rs_lib);
        compare(&data, 0xFFFF, &c_lib, &rs_lib);
    }
}
