// Integration test: low-level utility functions in utils.c / utils.rs
//
// We compare:
//   SPX_ull_to_bytes
//   SPX_u32_to_bytes
//   SPX_bytes_to_ull
// between the C and Rust shared libraries.

mod common;

use common::*;

type FnUllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type FnU32ToBytes = unsafe extern "C" fn(*mut u8, u32);
type FnBytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;

#[test]
fn test_ull_to_bytes() {
    let libs = open_libs();
    unsafe {
        let c_fn: libloading::Symbol<FnUllToBytes> = sym(&libs.c, b"SPX_ull_to_bytes");
        let r_fn: libloading::Symbol<FnUllToBytes> = sym(&libs.r, b"SPX_ull_to_bytes");

        let inputs: &[(u32, u64)] = &[
            (0, 0),
            (1, 0xab),
            (4, 0x12345678),
            (8, 0xdead_beef_cafe_babe),
            (3, 0x010203),
            (8, 0),
            (8, u64::MAX),
        ];

        for &(outlen, inp) in inputs {
            let mut c_buf = vec![0xCDu8; 16];
            let mut r_buf = vec![0xCDu8; 16];
            c_fn(c_buf.as_mut_ptr(), outlen, inp);
            r_fn(r_buf.as_mut_ptr(), outlen, inp);
            assert_eq!(c_buf, r_buf, "mismatch for ull_to_bytes(outlen={}, inp={:#x})", outlen, inp);
        }
    }
}

#[test]
fn test_u32_to_bytes() {
    let libs = open_libs();
    unsafe {
        let c_fn: libloading::Symbol<FnU32ToBytes> = sym(&libs.c, b"SPX_u32_to_bytes");
        let r_fn: libloading::Symbol<FnU32ToBytes> = sym(&libs.r, b"SPX_u32_to_bytes");

        let inputs: &[u32] = &[0, 1, 0xdead_beef, u32::MAX, 0x01020304, 0xff00ff00];
        for &inp in inputs {
            let mut c_buf = [0xCDu8; 8];
            let mut r_buf = [0xCDu8; 8];
            c_fn(c_buf.as_mut_ptr(), inp);
            r_fn(r_buf.as_mut_ptr(), inp);
            assert_eq!(c_buf, r_buf, "mismatch for u32_to_bytes({:#x})", inp);
        }
    }
}

#[test]
fn test_bytes_to_ull() {
    let libs = open_libs();
    unsafe {
        let c_fn: libloading::Symbol<FnBytesToUll> = sym(&libs.c, b"SPX_bytes_to_ull");
        let r_fn: libloading::Symbol<FnBytesToUll> = sym(&libs.r, b"SPX_bytes_to_ull");

        let inputs: &[(&[u8], u32)] = &[
            (&[][..], 0),
            (&[0xab][..], 1),
            (&[0x12, 0x34, 0x56, 0x78][..], 4),
            (&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe][..], 8),
        ];
        for (buf, len) in inputs {
            let c = c_fn(buf.as_ptr(), *len);
            let r = r_fn(buf.as_ptr(), *len);
            assert_eq!(c, r, "mismatch for bytes_to_ull({:?})", buf);
        }
    }
}
