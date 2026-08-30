//! Differential tests for the arguments that drive the C implementation's
//! variable length arrays (`SPX_VLA` in `app/include/utils.h`, plus the `inlen`
//! VLA inside `mgf1_*`).
//!
//! Inside the library these lengths are bounded by the parameter set, but the
//! routines are exported symbols and the C versions accept any size, so the
//! Rust translation has to as well.  These cases are exactly the ones a
//! fixed-size Rust array sized for the internal worst case would reject.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::os::raw::{c_uint, c_ulong};

type FnInitHash = unsafe extern "C" fn(*mut u8);
type FnThash = unsafe extern "C" fn(*mut u8, *const u8, c_uint, *const u8, *mut u32);
type FnMgf1 = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);

fn ctx_pair(tag: u8) -> (Box<Ctx>, Box<Ctx>) {
    let l = libs();
    let ci = unsafe { l.c::<FnInitHash>("SPX_initialize_hash_function") };
    let ri = unsafe { l.r::<FnInitHash>("SPX_initialize_hash_function") };
    let mut cc = Ctx::seeded(tag);
    let mut rc = Ctx::seeded(tag);
    unsafe {
        ci(cc.as_mut_ptr());
        ri(rc.as_mut_ptr());
    }
    (cc, rc)
}

/// The largest `inblocks` the library itself ever passes to `thash`
/// (`max(SPX_WOTS_LEN, SPX_FORS_TREES)`).
const INTERNAL_MAX_INBLOCKS: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
    SPX_WOTS_LEN
} else {
    SPX_FORS_TREES
};

/// The largest `tree_height` the library itself ever passes to a tree hash.
const INTERNAL_MAX_TREE_HEIGHT: usize = if SPX_TREE_HEIGHT > SPX_FORS_HEIGHT {
    SPX_TREE_HEIGHT
} else {
    SPX_FORS_HEIGHT
};

#[test]
fn thash_above_internal_max_inblocks() {
    let l = libs();
    let c = unsafe { l.c::<FnThash>("SPX_thash") };
    let r = unsafe { l.r::<FnThash>("SPX_thash") };
    let (cc, rc) = ctx_pair(0x51);
    let mut rng = Rng::new(0x3001);

    for extra in [1usize, 5, 64] {
        let inblocks = INTERNAL_MAX_INBLOCKS + extra;
        let inp = rng.vec(inblocks * SPX_N);
        let mut addr = [0u32; 8];
        for w in addr.iter_mut() {
            *w = rng.next_u32();
        }
        let mut ca = addr;
        let mut ra = addr;
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(
                co.as_mut_ptr(),
                inp.as_ptr(),
                inblocks as c_uint,
                cc.as_ptr(),
                ca.as_mut_ptr(),
            );
            r(
                ro.as_mut_ptr(),
                inp.as_ptr(),
                inblocks as c_uint,
                rc.as_ptr(),
                ra.as_mut_ptr(),
            );
        }
        assert_bytes_eq(&format!("thash(inblocks={inblocks})"), &co, &ro);
        assert_eq!(ca, ra, "thash(inblocks={inblocks}) addr side effect");
    }
}

#[test]
fn mgf1_above_internal_max_inlen() {
    let l = libs();
    let names: &[&str] = match BACKEND {
        "blake" => &["SPX_blake256_mgf1", "SPX_blake512_mgf1"],
        "sha2" => &["SPX_mgf1_256", "SPX_mgf1_512"],
        _ => &[],
    };
    let mut rng = Rng::new(0x3002);
    for name in names {
        let c = unsafe { l.c_backend::<FnMgf1>(name) };
        let r = unsafe { l.r::<FnMgf1>(name) };
        // Well past `2 * SPX_N + SPX_SHA512_OUTPUT_BYTES`, the largest input the
        // library itself ever passes.
        for inlen in [200usize, 300, 1000] {
            for outlen in [1usize, 32, 64, 200] {
                let inp = rng.vec(inlen);
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                unsafe {
                    c(co.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                    r(ro.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                }
                assert_bytes_eq(&format!("{name}(inlen={inlen}, outlen={outlen})"), &co, &ro);
            }
        }
    }
}

unsafe extern "C" fn gen_leaf_probe(leaf: *mut u8, _ctx: *const u8, addr_idx: u32, addr: *const u32) {
    unsafe {
        let ab = core::slice::from_raw_parts(addr as *const u8, 32);
        let out = core::slice::from_raw_parts_mut(leaf, SPX_N);
        let mut acc = addr_idx.wrapping_mul(0x9E37_79B9);
        for (i, b) in out.iter_mut().enumerate() {
            acc = acc
                .wrapping_add(ab[i % 32] as u32)
                .wrapping_mul(0x0100_1001)
                .rotate_left(7);
            *b = (acc >> 13) as u8;
        }
    }
}

type FnTreehash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u32,
    u32,
    u32,
    unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32),
    *mut u32,
);

#[test]
fn treehash_above_internal_max_height() {
    let l = libs();
    let c = unsafe { l.c::<FnTreehash>("SPX_treehash") };
    let r = unsafe { l.r::<FnTreehash>("SPX_treehash") };
    let (cc, rc) = ctx_pair(0x52);
    let mut rng = Rng::new(0x3003);

    for extra in [1u32, 2] {
        let th = INTERNAL_MAX_TREE_HEIGHT as u32 + extra;
        if th > 16 {
            continue; // 2^th leaf callbacks; keep the runtime bounded
        }
        let leaf_idx = rng.next_u32() & ((1u32 << th) - 1);
        let idx_offset = rng.next_u32() & 0xffff;
        let mut addr = [0u32; 8];
        for w in addr.iter_mut() {
            *w = rng.next_u32();
        }
        let mut ca = addr;
        let mut ra = addr;
        let mut croot = vec![0xAAu8; SPX_N + 8];
        let mut rroot = vec![0xAAu8; SPX_N + 8];
        let mut cauth = vec![0xAAu8; th as usize * SPX_N + 8];
        let mut rauth = vec![0xAAu8; th as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                th,
                gen_leaf_probe,
                ca.as_mut_ptr(),
            );
            r(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                rc.as_ptr(),
                leaf_idx,
                idx_offset,
                th,
                gen_leaf_probe,
                ra.as_mut_ptr(),
            );
        }
        assert_bytes_eq(&format!("treehash(th={th}) root"), &croot, &rroot);
        assert_bytes_eq(&format!("treehash(th={th}) auth_path"), &cauth, &rauth);
        assert_eq!(ca, ra, "treehash(th={th}) addr side effect");
    }
}

type FnForsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut ForsGenLeafInfo);

#[test]
fn fors_treehashx1_above_internal_max_height() {
    let l = libs();
    let c = unsafe { l.c::<FnForsTreehashX1>("SPX_fors_treehashx1") };
    let r = unsafe { l.r::<FnForsTreehashX1>("SPX_fors_treehashx1") };
    let (cc, rc) = ctx_pair(0x53);
    let mut rng = Rng::new(0x3004);

    let th = INTERNAL_MAX_TREE_HEIGHT as u32 + 1;
    if th > 15 {
        return; // 2^th leaves, each two hashes
    }
    let mut leaf_addrx = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    for w in leaf_addrx.iter_mut() {
        *w = rng.next_u32();
    }
    for w in tree_addr.iter_mut() {
        *w = rng.next_u32();
    }
    let mut cinfo = ForsGenLeafInfo { leaf_addrx };
    let mut rinfo = ForsGenLeafInfo { leaf_addrx };
    let mut cta = tree_addr;
    let mut rta = tree_addr;
    let mut croot = vec![0xAAu8; SPX_N + 8];
    let mut rroot = vec![0xAAu8; SPX_N + 8];
    let mut cauth = vec![0xAAu8; th as usize * SPX_N + 8];
    let mut rauth = vec![0xAAu8; th as usize * SPX_N + 8];
    unsafe {
        c(
            croot.as_mut_ptr(),
            cauth.as_mut_ptr(),
            cc.as_ptr(),
            0,
            0,
            th,
            cta.as_mut_ptr(),
            &mut cinfo,
        );
        r(
            rroot.as_mut_ptr(),
            rauth.as_mut_ptr(),
            rc.as_ptr(),
            0,
            0,
            th,
            rta.as_mut_ptr(),
            &mut rinfo,
        );
    }
    assert_bytes_eq(&format!("fors_treehashx1(th={th}) root"), &croot, &rroot);
    assert_bytes_eq(&format!("fors_treehashx1(th={th}) auth"), &cauth, &rauth);
    assert_eq!(cta, rta, "tree_addr side effect");
    assert_eq!(cinfo.leaf_addrx, rinfo.leaf_addrx, "leaf_addrx side effect");
}

type FnWotsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1);

#[test]
fn wots_treehashx1_above_internal_max_height() {
    let l = libs();
    let c = unsafe { l.c::<FnWotsTreehashX1>("SPX_wots_treehashx1") };
    let r = unsafe { l.r::<FnWotsTreehashX1>("SPX_wots_treehashx1") };
    let (cc, rc) = ctx_pair(0x54);
    let mut rng = Rng::new(0x3005);

    let th = INTERNAL_MAX_TREE_HEIGHT as u32 + 1;
    // Every leaf runs SPX_WOTS_LEN chains of SPX_WOTS_W - 1 hashes.
    if th > 8 {
        return;
    }
    let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
        .map(|_| rng.next_u32() % SPX_WOTS_W as u32)
        .collect();
    let mut addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    for w in addr.iter_mut() {
        *w = rng.next_u32();
    }
    for w in tree_addr.iter_mut() {
        *w = rng.next_u32();
    }
    let mut csig = vec![0xAAu8; SPX_WOTS_BYTES];
    let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES];
    let mut cinfo = LeafInfoX1 {
        wots_sig: csig.as_mut_ptr(),
        wots_sign_leaf: 1,
        wots_steps: steps.as_mut_ptr(),
        leaf_addr: addr,
        pk_addr: addr,
    };
    let mut rinfo = LeafInfoX1 {
        wots_sig: rsig.as_mut_ptr(),
        wots_sign_leaf: 1,
        wots_steps: steps.as_mut_ptr(),
        leaf_addr: addr,
        pk_addr: addr,
    };
    let mut cta = tree_addr;
    let mut rta = tree_addr;
    let mut croot = vec![0xAAu8; SPX_N + 8];
    let mut rroot = vec![0xAAu8; SPX_N + 8];
    let mut cauth = vec![0xAAu8; th as usize * SPX_N + 8];
    let mut rauth = vec![0xAAu8; th as usize * SPX_N + 8];
    unsafe {
        c(
            croot.as_mut_ptr(),
            cauth.as_mut_ptr(),
            cc.as_ptr(),
            1,
            0,
            th,
            cta.as_mut_ptr(),
            &mut cinfo,
        );
        r(
            rroot.as_mut_ptr(),
            rauth.as_mut_ptr(),
            rc.as_ptr(),
            1,
            0,
            th,
            rta.as_mut_ptr(),
            &mut rinfo,
        );
    }
    assert_bytes_eq(&format!("wots_treehashx1(th={th}) root"), &croot, &rroot);
    assert_bytes_eq(&format!("wots_treehashx1(th={th}) auth"), &cauth, &rauth);
    assert_bytes_eq(&format!("wots_treehashx1(th={th}) wots_sig"), &csig, &rsig);
    assert_eq!(cta, rta, "tree_addr side effect");
    assert_eq!(cinfo.leaf_addr, rinfo.leaf_addr, "leaf_addr side effect");
    assert_eq!(cinfo.pk_addr, rinfo.pk_addr, "pk_addr side effect");
}
