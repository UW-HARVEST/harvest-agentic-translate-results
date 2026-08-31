//! Smoke test: both libraries load and trivial exports agree.
mod common;

use std::ffi::CStr;
use std::os::raw::c_char;

#[test]
fn version_number() {
    let (cf, rf) = pair!("LZ4_versionNumber", fn() -> i32);
    unsafe {
        assert_eq!(cf(), rf());
        assert_eq!(cf(), 11000);
    }
}

#[test]
fn version_string() {
    let (cf, rf) = pair!("LZ4_versionString", fn() -> *const c_char);
    unsafe {
        let a = CStr::from_ptr(cf());
        let b = CStr::from_ptr(rf());
        assert_eq!(a, b);
        assert_eq!(a.to_str().unwrap(), "1.10.0");
    }
}

#[test]
fn xxh_version_number() {
    let (cf, rf) = pair!("LZ4_XXH_versionNumber", fn() -> u32);
    unsafe {
        assert_eq!(cf(), rf());
    }
}

#[test]
fn compress_bound() {
    let (cf, rf) = pair!("LZ4_compressBound", fn(i32) -> i32);
    let mut inputs: Vec<i32> = vec![
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        254,
        255,
        256,
        65535,
        65536,
        0x7E000000 - 1,
        0x7E000000,
        0x7E000000 + 1,
        i32::MAX,
    ];
    for i in 0..64 {
        inputs.push(i * 977);
    }
    unsafe {
        for &n in &inputs {
            assert_eq!(cf(n), rf(n), "LZ4_compressBound({})", n);
        }
    }
}

#[test]
fn sizeof_helpers() {
    unsafe {
        {
            let (cf, rf) = pair!("LZ4_sizeofState", fn() -> i32);
            assert_eq!(cf(), rf());
        }
        {
            let (cf, rf) = pair!("LZ4_sizeofStateHC", fn() -> i32);
            assert_eq!(cf(), rf());
        }
        {
            let (cf, rf) = pair!("LZ4_sizeofStreamState", fn() -> i32);
            assert_eq!(cf(), rf());
        }
        {
            let (cf, rf) = pair!("LZ4_sizeofStreamStateHC", fn() -> i32);
            assert_eq!(cf(), rf());
        }
        {
            let (cf, rf) = pair!("LZ4F_compressionLevel_max", fn() -> i32);
            assert_eq!(cf(), rf());
        }
        {
            let (cf, rf) = pair!("LZ4F_getVersion", fn() -> u32);
            assert_eq!(cf(), rf());
        }
    }
}

#[test]
fn decoder_ring_buffer_size() {
    let (cf, rf) = pair!("LZ4_decoderRingBufferSize", fn(i32) -> i32);
    unsafe {
        for n in [-1i32, 0, 1, 2, 100, 65535, 65536, 1 << 20, 0x7E000000, i32::MAX] {
            assert_eq!(cf(n), rf(n), "LZ4_decoderRingBufferSize({})", n);
        }
    }
}
