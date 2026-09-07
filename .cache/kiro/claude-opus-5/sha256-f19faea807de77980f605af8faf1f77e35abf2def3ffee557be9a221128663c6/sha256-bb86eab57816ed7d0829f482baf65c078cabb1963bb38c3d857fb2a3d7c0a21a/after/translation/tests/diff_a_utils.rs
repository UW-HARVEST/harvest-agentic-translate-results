//! CONFIGS.md group A — lowest-level `utils.c` / `address.c` entry points.
//!
//! Every function is reached through `dlsym` on both `.so`s.

mod common;

use common::*;
use std::os::raw::c_uint;

const ITER: usize = 400;

#[test]
fn a1_ull_to_bytes() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u8, c_uint, u64);
    let (c, r) = pair!(libs, "SPX_ull_to_bytes", F);
    let mut rng = Rng::new(1);

    for &outlen in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16] {
        for i in 0..ITER {
            let v: u64 = match i {
                0 => 0,
                1 => 1,
                2 => u64::MAX,
                3 => 0xFF,
                4 => 0x100,
                5 => 0x8000_0000_0000_0000,
                _ => rng.next_u64(),
            };
            // Guard bytes on both sides of the window catch over-long writes.
            let mut cb = vec![0xAAu8; outlen + 8];
            let mut rb = vec![0xAAu8; outlen + 8];
            unsafe {
                c(cb.as_mut_ptr().add(4), outlen as c_uint, v);
                r(rb.as_mut_ptr().add(4), outlen as c_uint, v);
            }
            eq_bytes(&format!("ull_to_bytes(outlen={outlen}, in={v:#x})"), &cb, &rb);
        }
    }
}

#[test]
fn a2_u32_to_bytes() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u8, u32);
    let (c, r) = pair!(libs, "SPX_u32_to_bytes", F);
    let mut rng = Rng::new(2);

    for i in 0..ITER {
        let v: u32 = match i {
            0 => 0,
            1 => 1,
            2 => u32::MAX,
            3 => 0xFF,
            4 => 0x0100_0000,
            _ => rng.next_u32(),
        };
        let mut cb = [0xAAu8; 12];
        let mut rb = [0xAAu8; 12];
        unsafe {
            c(cb.as_mut_ptr().add(4), v);
            r(rb.as_mut_ptr().add(4), v);
        }
        eq_bytes(&format!("u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

#[test]
fn a3_a4_bytes_to_ull() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*const u8, c_uint) -> u64;
    let (c, r) = pair!(libs, "SPX_bytes_to_ull", F);
    let mut rng = Rng::new(3);

    // A3: inlen 0..=8 (the documented range).  A4: 9 and 16, past the 64-bit
    // width, where the C shift amount exceeds 63.
    for &inlen in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16] {
        for i in 0..ITER {
            let buf: Vec<u8> = match i {
                0 => vec![0u8; inlen.max(1)],
                1 => vec![0xFFu8; inlen.max(1)],
                2 => (0..inlen.max(1)).map(|k| k as u8).collect(),
                _ => rng.bytes(inlen.max(1)),
            };
            let cv = unsafe { c(buf.as_ptr(), inlen as c_uint) };
            let rv = unsafe { r(buf.as_ptr(), inlen as c_uint) };
            eq(&format!("bytes_to_ull(inlen={inlen}, in={:02x?})", &buf), cv, rv);
        }
    }
}

/// Drives one `set_*(addr, v)` style function over an address array and checks
/// the whole 32-byte address afterwards.
fn addr_setter_u32(libs: &Libs, name: &'static str, seed: u64, vals: &[u32]) {
    type F = unsafe extern "C" fn(*mut u32, u32);
    let cname = std::ffi::CString::new(name).unwrap();
    let c: libloading::os::unix::Symbol<F> = libs.c(cname.to_str().unwrap());
    let r: libloading::os::unix::Symbol<F> = libs.r(cname.to_str().unwrap());
    let mut rng = Rng::new(seed);

    for &v in vals {
        for _ in 0..ITER {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(
                &format!("{name}(v={v:#x}) addr"),
                &u32s_to_bytes(&ca),
                &u32s_to_bytes(&ra),
            );
        }
    }
}

const TRUNC_VALS: &[u32] = &[0, 1, 2, 6, 7, 8, 127, 128, 254, 255, 256, 257, 1000, 0xFFFF, 0xFFFF_FFFF];

#[test]
fn a5_set_layer_addr() {
    let libs = Libs::load();
    addr_setter_u32(&libs, "SPX_set_layer_addr", 5, TRUNC_VALS);
    // plus fully random layers
    let mut rng = Rng::new(55);
    let vals: Vec<u32> = (0..64).map(|_| rng.next_u32()).collect();
    addr_setter_u32(&libs, "SPX_set_layer_addr", 56, &vals);
}

#[test]
fn a6_set_tree_addr() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u32, u64);
    let (c, r) = pair!(libs, "SPX_set_tree_addr", F);
    let mut rng = Rng::new(6);

    let mut vals: Vec<u64> = vec![
        0,
        1,
        0xFF,
        0x100,
        0xFFFF_FFFF,
        0x1_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
        u64::MAX,
    ];
    vals.extend((0..ITER).map(|_| rng.next_u64()));

    for v in vals {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        eq_bytes(
            &format!("set_tree_addr(tree={v:#x})"),
            &u32s_to_bytes(&ca),
            &u32s_to_bytes(&ra),
        );
    }
}

#[test]
fn a7_a8_set_type() {
    let libs = Libs::load();
    // A7: all valid SPX_ADDR_TYPE_* values.  A8: out-of-range enum ints — C
    // enums accept any int, and set_type just truncates to a byte.
    let valid: &[u32] = &[0, 1, 2, 3, 4, 5, 6];
    let invalid: &[u32] = &[
        7, 8, 9, 16, 100, 255, 256, 257, 262, 1000, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000,
        0xFFFF_FFFF,
    ];
    addr_setter_u32(&libs, "SPX_set_type", 7, valid);
    addr_setter_u32(&libs, "SPX_set_type", 8, invalid);
}

#[test]
fn a9_set_keypair_addr() {
    let libs = Libs::load();
    let mut rng = Rng::new(9);
    let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0xFFFF, 0x1_0000, 0xFFFF_FFFF];
    vals.extend((0..32).map(|_| rng.next_u32()));
    addr_setter_u32(&libs, "SPX_set_keypair_addr", 9, &vals);
}

#[test]
fn a10_set_chain_addr() {
    let libs = Libs::load();
    addr_setter_u32(&libs, "SPX_set_chain_addr", 10, TRUNC_VALS);
}

#[test]
fn a11_set_hash_addr() {
    let libs = Libs::load();
    addr_setter_u32(&libs, "SPX_set_hash_addr", 11, TRUNC_VALS);
}

#[test]
fn a12_set_tree_height() {
    let libs = Libs::load();
    addr_setter_u32(&libs, "SPX_set_tree_height", 12, TRUNC_VALS);
}

#[test]
fn a13_set_tree_index() {
    let libs = Libs::load();
    let mut rng = Rng::new(13);
    let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0xFFFF, 0x1_0000, 0xFFFF_FFFF];
    vals.extend((0..32).map(|_| rng.next_u32()));
    addr_setter_u32(&libs, "SPX_set_tree_index", 13, &vals);
}

#[test]
fn a14_copy_subtree_addr() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u32, *const u32);
    let (c, r) = pair!(libs, "SPX_copy_subtree_addr", F);
    let mut rng = Rng::new(14);

    for _ in 0..ITER {
        let inp = rng.addr();
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), inp.as_ptr());
            r(ra.as_mut_ptr(), inp.as_ptr());
        }
        eq_bytes("copy_subtree_addr", &u32s_to_bytes(&ca), &u32s_to_bytes(&ra));
        // Sanity on the C semantics we are mirroring: memcpy of
        // SPX_OFFSET_TREE + 8 bytes.
        let ib = u32s_to_bytes(&inp);
        let ob = u32s_to_bytes(&ca);
        assert_eq!(&ib[..OFF_TREE + 8], &ob[..OFF_TREE + 8]);
    }
}

#[test]
fn a15_copy_keypair_addr() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u32, *const u32);
    let (c, r) = pair!(libs, "SPX_copy_keypair_addr", F);
    let mut rng = Rng::new(15);

    for _ in 0..ITER {
        let inp = rng.addr();
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), inp.as_ptr());
            r(ra.as_mut_ptr(), inp.as_ptr());
        }
        eq_bytes("copy_keypair_addr", &u32s_to_bytes(&ca), &u32s_to_bytes(&ra));
        let ib = u32s_to_bytes(&inp);
        let ob = u32s_to_bytes(&ca);
        assert_eq!(&ib[..OFF_TREE + 8], &ob[..OFF_TREE + 8]);
        assert_eq!(
            &ib[OFF_KP_ADDR..OFF_KP_ADDR + 4],
            &ob[OFF_KP_ADDR..OFF_KP_ADDR + 4]
        );
    }
}
