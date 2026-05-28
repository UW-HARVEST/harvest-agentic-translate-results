// Integration test: compares C and Rust shared library implementations of crc16
// through libloading FFI. Both .so libs must produce byte-identical results.

use libloading::{Library, Symbol};
use std::path::PathBuf;

type Crc16Fn = unsafe extern "C" fn(*const u8, u32, u16) -> u16;

fn c_lib_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Cargo sets OUT_DIR; we need the cdylib output dir. Use CARGO_TARGET_DIR
    // or fallback to relative target/release.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut p = PathBuf::from(manifest_dir).join("target/release/libcrc16_lib.so");
    if !p.exists() {
        p = PathBuf::from(manifest_dir).join("target/debug/libcrc16_lib.so");
    }
    p
}

struct Libs {
    _c: Library,
    _r: Library,
    c_fn: Crc16Fn,
    r_fn: Crc16Fn,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            let c_lib =
                Library::new(c_lib_path()).expect("failed to load C shared library");
            let r_lib =
                Library::new(rust_lib_path()).expect("failed to load Rust shared library");
            let c_sym: Symbol<Crc16Fn> =
                c_lib.get(b"crc16\0").expect("C lib missing crc16");
            let r_sym: Symbol<Crc16Fn> =
                r_lib.get(b"crc16\0").expect("Rust lib missing crc16");
            let c_fn: Crc16Fn = *c_sym;
            let r_fn: Crc16Fn = *r_sym;
            Libs {
                _c: c_lib,
                _r: r_lib,
                c_fn,
                r_fn,
            }
        }
    }

    fn check(&self, data: &[u8], crc: u16) {
        let c_out =
            unsafe { (self.c_fn)(data.as_ptr(), data.len() as u32, crc) };
        let r_out =
            unsafe { (self.r_fn)(data.as_ptr(), data.len() as u32, crc) };
        assert_eq!(
            c_out, r_out,
            "mismatch len={} crc={:#x} data[..min(16)]={:?}",
            data.len(),
            crc,
            &data[..data.len().min(16)]
        );
    }
}

#[test]
fn empty_input() {
    let l = Libs::load();
    for crc in [0u16, 1, 0xFFFF, 0x1234, 0x8000] {
        l.check(&[], crc);
    }
}

#[test]
fn single_byte_inputs() {
    let l = Libs::load();
    for b in 0u16..=255 {
        let data = [b as u8];
        for crc in [0u16, 0xFFFF, 0xABCD] {
            l.check(&data, crc);
        }
    }
}

#[test]
fn small_inputs_lengths_1_to_15() {
    let l = Libs::load();
    // Make a deterministic stream
    let mut data = [0u8; 16];
    for (i, b) in data.iter_mut().enumerate() {
        *b = ((i * 37 + 7) & 0xFF) as u8;
    }
    for len in 1..=15 {
        for crc in [0u16, 0x1, 0xFFFF, 0xABCD, 0x8000, 0x1234] {
            l.check(&data[..len], crc);
        }
    }
}

#[test]
fn aligned_8_block_inputs() {
    let l = Libs::load();
    let mut data = vec![0u8; 64];
    for (i, b) in data.iter_mut().enumerate() {
        *b = ((i * 131 + 17) & 0xFF) as u8;
    }
    for len in [8, 16, 24, 32, 40, 48, 56, 64] {
        for crc in [0u16, 0xFFFF, 0xDEAD] {
            l.check(&data[..len], crc);
        }
    }
}

#[test]
fn mixed_lengths() {
    let l = Libs::load();
    let mut data = vec![0u8; 1024];
    let mut x: u32 = 0xCAFEBABE;
    for b in data.iter_mut() {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        *b = (x >> 24) as u8;
    }
    let lens = [
        0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 17, 23, 24, 25, 31, 32, 33,
        63, 64, 65, 100, 127, 128, 129, 255, 256, 257, 511, 512, 513, 1023, 1024,
    ];
    for &len in &lens {
        for crc in [0u16, 0xFFFF, 0x1234, 0xABCD, 0x8000, 0x0001] {
            l.check(&data[..len], crc);
        }
    }
}

#[test]
fn all_zeros() {
    let l = Libs::load();
    let data = vec![0u8; 200];
    for len in [0usize, 1, 7, 8, 9, 16, 100, 200] {
        for crc in [0u16, 0xFFFF, 0xAAAA] {
            l.check(&data[..len], crc);
        }
    }
}

#[test]
fn all_ones() {
    let l = Libs::load();
    let data = vec![0xFFu8; 200];
    for len in [0usize, 1, 7, 8, 9, 16, 100, 200] {
        for crc in [0u16, 0xFFFF, 0x5555] {
            l.check(&data[..len], crc);
        }
    }
}

#[test]
fn large_input() {
    let l = Libs::load();
    let mut data = vec![0u8; 65536];
    let mut x: u32 = 0xDEADBEEF;
    for b in data.iter_mut() {
        x = x.wrapping_mul(22695477).wrapping_add(1);
        *b = (x >> 16) as u8;
    }
    for crc in [0u16, 0xFFFF] {
        l.check(&data, crc);
    }
}
