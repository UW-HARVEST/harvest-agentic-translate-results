// Integration test: address.c / address.rs functions.

mod common;

use common::*;

type FnSetU32 = unsafe extern "C" fn(*mut u32, u32);
type FnSetU64 = unsafe extern "C" fn(*mut u32, u64);
type FnCopy = unsafe extern "C" fn(*mut u32, *const u32);

fn fresh_addr() -> [u32; 8] {
    let mut a = [0u32; 8];
    for i in 0..8 {
        a[i] = 0xa5a5_a5a5 ^ (i as u32);
    }
    a
}

unsafe fn check_u32(libs: &Libs, name: &[u8], values: &[u32]) {
    let c_fn: libloading::Symbol<FnSetU32> = sym(&libs.c, name);
    let r_fn: libloading::Symbol<FnSetU32> = sym(&libs.r, name);
    for &v in values {
        let mut c_addr = fresh_addr();
        let mut r_addr = fresh_addr();
        c_fn(c_addr.as_mut_ptr(), v);
        r_fn(r_addr.as_mut_ptr(), v);
        assert_eq!(c_addr, r_addr, "mismatch for {} v={:#x}", String::from_utf8_lossy(name), v);
    }
}

unsafe fn check_u64(libs: &Libs, name: &[u8], values: &[u64]) {
    let c_fn: libloading::Symbol<FnSetU64> = sym(&libs.c, name);
    let r_fn: libloading::Symbol<FnSetU64> = sym(&libs.r, name);
    for &v in values {
        let mut c_addr = fresh_addr();
        let mut r_addr = fresh_addr();
        c_fn(c_addr.as_mut_ptr(), v);
        r_fn(r_addr.as_mut_ptr(), v);
        assert_eq!(c_addr, r_addr, "mismatch for {} v={:#x}", String::from_utf8_lossy(name), v);
    }
}

#[test]
fn test_set_layer_addr() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_layer_addr", &[0u32, 0xab, 0xff, 0x12345678]);
    }
}

#[test]
fn test_set_tree_addr() {
    let libs = open_libs();
    unsafe {
        check_u64(&libs, b"SPX_set_tree_addr", &[0u64, 1, 0xdead_beef_cafe_babe, u64::MAX]);
    }
}

#[test]
fn test_set_type() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_type", &[0u32, 1, 2, 3, 4, 5, 6]);
    }
}

#[test]
fn test_set_keypair_addr() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_keypair_addr", &[0u32, 0xabcdef, 0xff_aaff, u32::MAX]);
    }
}

#[test]
fn test_set_chain_addr() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_chain_addr", &[0u32, 1, 7, 15, 0xff, 0x12345678]);
    }
}

#[test]
fn test_set_hash_addr() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_hash_addr", &[0u32, 1, 7, 15, 0xff, 0x12345678]);
    }
}

#[test]
fn test_set_tree_height() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_tree_height", &[0u32, 1, 7, 15, 0xff, 0x12345678]);
    }
}

#[test]
fn test_set_tree_index() {
    let libs = open_libs();
    unsafe {
        check_u32(&libs, b"SPX_set_tree_index", &[0u32, 0xabcd, 0x12345678, u32::MAX]);
    }
}

#[test]
fn test_copy_subtree_addr() {
    let libs = open_libs();
    unsafe {
        let c_fn: libloading::Symbol<FnCopy> = sym(&libs.c, b"SPX_copy_subtree_addr");
        let r_fn: libloading::Symbol<FnCopy> = sym(&libs.r, b"SPX_copy_subtree_addr");

        let mut src = [0u32; 8];
        for i in 0..8 {
            src[i] = 0xdead_0000 + i as u32;
        }
        let mut c_addr = [0xa5a5_a5a5u32; 8];
        let mut r_addr = [0xa5a5_a5a5u32; 8];
        c_fn(c_addr.as_mut_ptr(), src.as_ptr());
        r_fn(r_addr.as_mut_ptr(), src.as_ptr());
        assert_eq!(c_addr, r_addr);
    }
}

#[test]
fn test_copy_keypair_addr() {
    let libs = open_libs();
    unsafe {
        let c_fn: libloading::Symbol<FnCopy> = sym(&libs.c, b"SPX_copy_keypair_addr");
        let r_fn: libloading::Symbol<FnCopy> = sym(&libs.r, b"SPX_copy_keypair_addr");

        let mut src = [0u32; 8];
        for i in 0..8 {
            src[i] = 0xfeed_0000 + i as u32;
        }
        let mut c_addr = [0xa5a5_a5a5u32; 8];
        let mut r_addr = [0xa5a5_a5a5u32; 8];
        c_fn(c_addr.as_mut_ptr(), src.as_ptr());
        r_fn(r_addr.as_mut_ptr(), src.as_ptr());
        assert_eq!(c_addr, r_addr);
    }
}
