//! Level 1: `create_block` -- no allocation, pure struct construction.

mod common;

use common::{DataBlock, name_str, pair, raw_bytes};
use std::ffi::{CString, c_char, c_int, c_uchar};

fn call_both(id: c_int, name: &str, flags: c_uchar) -> (DataBlock, DataBlock) {
    let p = pair();
    let c_fn = p.c.create_block();
    let rs_fn = p.rs.create_block();
    let cname = CString::new(name).expect("no interior NUL");
    let arg = cname.as_ptr() as *const c_char;
    let a = unsafe { c_fn(id, arg, flags) };
    let b = unsafe { rs_fn(id, arg, flags) };
    (a, b)
}

/// `id`, `flags` and the NUL-terminated string must agree.
///
/// Bytes past the terminator come from an uninitialised stack slot in the C
/// (`DataBlock block;` is never zeroed), so they are indeterminate by
/// construction and are excluded from the comparison.
#[test]
fn create_block_matches_c() {
    let names = [
        "",
        "a",
        "Block_Alpha",
        "Block_Beta",
        "Block_Gamma",
        "Special",
        "Modified",
        "0123456789",
        // 31 chars: the longest string that still fits with its terminator.
        "0123456789012345678901234567890",
    ];
    let ids = [
        0,
        1,
        -1,
        99,
        i32::MAX,
        i32::MIN,
        0x0102_0304,
    ];
    let flags = [0u8, 1, 0x0f, 0xf0, 0xaa, 0x55, 0xcc, 0xff];

    for name in names {
        for &id in &ids {
            for &f in &flags {
                let (c, r) = call_both(id, name, f);
                assert_eq!(c.id, r.id, "id mismatch for ({id}, {name:?}, {f:#x})");
                assert_eq!(c.flags, r.flags, "flags mismatch for ({id}, {name:?}, {f:#x})");
                assert_eq!(
                    name_str(&c),
                    name_str(&r),
                    "name mismatch for ({id}, {name:?}, {f:#x})"
                );
                assert_eq!(
                    name_str(&c),
                    name.as_bytes(),
                    "C did not round-trip the name for ({id}, {name:?}, {f:#x})"
                );
            }
        }
    }
}

/// With a 31-character name every byte of `name` is written by `strcpy`
/// (31 chars + NUL == 32), so the whole 40-byte struct image is defined and can
/// be compared byte for byte.
#[test]
fn create_block_full_struct_is_byte_identical() {
    let name = "abcdefghijklmnopqrstuvwxyzABCDE"; // 31 chars
    assert_eq!(name.len(), 31);
    for &id in &[0, 7, -12345, i32::MIN, i32::MAX] {
        for &f in &[0u8, 0x5a, 0xff] {
            let (c, r) = call_both(id, name, f);
            let cb = raw_bytes(&c);
            let rb = raw_bytes(&r);
            // Padding bytes after `flags` are not written by either side, so
            // compare the defined prefix (offset 0..37) explicitly.
            assert_eq!(&cb[..37], &rb[..37], "struct image mismatch ({id}, {f:#x})");
        }
    }
}
