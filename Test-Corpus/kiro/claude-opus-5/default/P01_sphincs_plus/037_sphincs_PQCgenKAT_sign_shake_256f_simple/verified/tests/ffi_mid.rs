//! Differential tests for the middle layer: `app/src/wots.c`,
//! `app/src/wotsx1.c`, `app/src/utilsx1.c`, `app/src/fors.c` and
//! `app/src/merkle.c`, plus `compute_root`/`treehash` from `app/src/utils.c`.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::os::raw::c_uint;

type FnInitHash = unsafe extern "C" fn(*mut u8);

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
    assert_bytes_eq("ctx after initialize_hash_function", &cc.bytes, &rc.bytes);
    (cc, rc)
}

fn rand_addr(rng: &mut Rng) -> [u32; 8] {
    let mut a = [0u32; 8];
    for w in a.iter_mut() {
        *w = rng.next_u32();
    }
    a
}

// ---------------------------------------------------------------------------
// app/src/wots.c
// ---------------------------------------------------------------------------

type FnChainLengths = unsafe extern "C" fn(*mut c_uint, *const u8);

#[test]
fn chain_lengths() {
    let l = libs();
    let c = unsafe { l.c::<FnChainLengths>("SPX_chain_lengths") };
    let r = unsafe { l.r::<FnChainLengths>("SPX_chain_lengths") };
    let mut rng = Rng::new(0x1111);
    for round in 0..32 {
        let msg = match round {
            0 => vec![0x00u8; SPX_N],
            1 => vec![0xffu8; SPX_N],
            _ => rng.vec(SPX_N),
        };
        let mut cl = vec![0xdead_beefu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xdead_beefu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), msg.as_ptr());
            r(rl.as_mut_ptr(), msg.as_ptr());
        }
        assert_eq!(cl, rl, "chain_lengths({msg:02x?})");
    }
}

type FnWotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);

#[test]
fn wots_pk_from_sig() {
    let l = libs();
    let c = unsafe { l.c::<FnWotsPkFromSig>("SPX_wots_pk_from_sig") };
    let r = unsafe { l.r::<FnWotsPkFromSig>("SPX_wots_pk_from_sig") };
    let (cc, rc) = ctx_pair(0x21);
    let mut rng = Rng::new(0x2222);
    for _ in 0..3 {
        let sig = rng.vec(SPX_WOTS_BYTES);
        let msg = rng.vec(SPX_N);
        let addr = rand_addr(&mut rng);
        let mut ca = addr;
        let mut ra = addr;
        let mut cpk = vec![0xAAu8; SPX_WOTS_PK_BYTES + 8];
        let mut rpk = vec![0xAAu8; SPX_WOTS_PK_BYTES + 8];
        unsafe {
            c(
                cpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cc.as_ptr(),
                ca.as_mut_ptr(),
            );
            r(
                rpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                rc.as_ptr(),
                ra.as_mut_ptr(),
            );
        }
        assert_bytes_eq("wots_pk_from_sig", &cpk, &rpk);
        assert_eq!(ca, ra, "wots_pk_from_sig addr side effect");
    }
}

// ---------------------------------------------------------------------------
// app/src/wotsx1.c
// ---------------------------------------------------------------------------

type FnWotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);

/// Runs `wots_gen_leafx1` against both libraries with identical `leaf_info_x1`
/// state and compares the leaf, the WOTS signature buffer and the mutated
/// address fields.
fn run_wots_gen_leafx1(sign_leaf: u32, leaf_idx: u32, seed: u64) {
    let l = libs();
    let c = unsafe { l.c::<FnWotsGenLeafX1>("SPX_wots_gen_leafx1") };
    let r = unsafe { l.r::<FnWotsGenLeafX1>("SPX_wots_gen_leafx1") };
    let (cc, rc) = ctx_pair(0x22);
    let mut rng = Rng::new(seed);

    let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
        .map(|_| rng.next_u32() % SPX_WOTS_W as u32)
        .collect();
    let addr = rand_addr(&mut rng);

    let mut csig = vec![0xAAu8; SPX_WOTS_BYTES];
    let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES];
    let mut cinfo = LeafInfoX1 {
        wots_sig: csig.as_mut_ptr(),
        wots_sign_leaf: sign_leaf,
        wots_steps: steps.as_mut_ptr(),
        leaf_addr: addr,
        pk_addr: addr,
    };
    let mut rinfo = LeafInfoX1 {
        wots_sig: rsig.as_mut_ptr(),
        wots_sign_leaf: sign_leaf,
        wots_steps: steps.as_mut_ptr(),
        leaf_addr: addr,
        pk_addr: addr,
    };

    let mut cd = vec![0xAAu8; SPX_N + 8];
    let mut rd = vec![0xAAu8; SPX_N + 8];
    unsafe {
        c(cd.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut cinfo);
        r(rd.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut rinfo);
    }
    assert_bytes_eq(
        &format!("wots_gen_leafx1 leaf (sign_leaf={sign_leaf}, leaf_idx={leaf_idx})"),
        &cd,
        &rd,
    );
    assert_bytes_eq(
        &format!("wots_gen_leafx1 wots_sig (sign_leaf={sign_leaf}, leaf_idx={leaf_idx})"),
        &csig,
        &rsig,
    );
    assert_eq!(cinfo.leaf_addr, rinfo.leaf_addr, "leaf_addr side effect");
    assert_eq!(cinfo.pk_addr, rinfo.pk_addr, "pk_addr side effect");
    assert_eq!(
        cinfo.wots_sign_leaf, rinfo.wots_sign_leaf,
        "wots_sign_leaf must not change"
    );
}

#[test]
fn wots_gen_leafx1_signing() {
    // leaf_idx == wots_sign_leaf: the WOTS signature is produced.
    run_wots_gen_leafx1(7, 7, 0x3333);
    run_wots_gen_leafx1(0, 0, 0x3334);
}

#[test]
fn wots_gen_leafx1_not_signing() {
    // leaf_idx != wots_sign_leaf: only the public key is produced.
    run_wots_gen_leafx1(!0u32, 5, 0x3335);
    run_wots_gen_leafx1(3, 9, 0x3336);
}

// ---------------------------------------------------------------------------
// app/src/utils.c: compute_root and treehash
// ---------------------------------------------------------------------------

type FnComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);

#[test]
fn compute_root() {
    let l = libs();
    let c = unsafe { l.c::<FnComputeRoot>("SPX_compute_root") };
    let r = unsafe { l.r::<FnComputeRoot>("SPX_compute_root") };
    let (cc, rc) = ctx_pair(0x23);
    let mut rng = Rng::new(0x4444);

    let mut heights: Vec<u32> = vec![1, 2, 3, SPX_TREE_HEIGHT as u32, SPX_FORS_HEIGHT as u32];
    heights.sort_unstable();
    heights.dedup();
    for th in heights {
        for _ in 0..3 {
            let leaf = rng.vec(SPX_N);
            let auth = rng.vec(th as usize * SPX_N);
            let leaf_idx = rng.next_u32();
            let idx_offset = rng.next_u32();
            let addr = rand_addr(&mut rng);
            let mut ca = addr;
            let mut ra = addr;
            let mut co = vec![0xAAu8; SPX_N + 8];
            let mut ro = vec![0xAAu8; SPX_N + 8];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    th,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    ro.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    th,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(
                &format!("compute_root(th={th}, leaf_idx={leaf_idx:#x}, off={idx_offset:#x})"),
                &co,
                &ro,
            );
            assert_eq!(ca, ra, "compute_root addr side effect (th={th})");
        }
    }
}

/// `gen_leaf` callback handed to `treehash`.  Deterministic, and deliberately
/// depends on the current contents of `tree_addr` so that the order in which
/// the caller updates the address is observable.
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
fn treehash() {
    let l = libs();
    let c = unsafe { l.c::<FnTreehash>("SPX_treehash") };
    let r = unsafe { l.r::<FnTreehash>("SPX_treehash") };
    let (cc, rc) = ctx_pair(0x24);
    let mut rng = Rng::new(0x5555);

    let mut heights: Vec<u32> = vec![1, 2, 3, 4, SPX_TREE_HEIGHT as u32, SPX_FORS_HEIGHT as u32];
    heights.retain(|&h| h <= 10);
    heights.sort_unstable();
    heights.dedup();
    for th in heights {
        for leaf_idx in [0u32, 1, (1u32 << th) - 1] {
            let idx_offset = rng.next_u32() & 0xffff;
            let addr = rand_addr(&mut rng);
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
            let what = format!("treehash(th={th}, leaf_idx={leaf_idx}, off={idx_offset})");
            assert_bytes_eq(&format!("{what} root"), &croot, &rroot);
            assert_bytes_eq(&format!("{what} auth_path"), &cauth, &rauth);
            assert_eq!(ca, ra, "{what} addr side effect");
        }
    }
}

// ---------------------------------------------------------------------------
// app/src/utilsx1.c
// ---------------------------------------------------------------------------

type FnWotsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1);

#[test]
fn wots_treehashx1() {
    let l = libs();
    let c = unsafe { l.c::<FnWotsTreehashX1>("SPX_wots_treehashx1") };
    let r = unsafe { l.r::<FnWotsTreehashX1>("SPX_wots_treehashx1") };
    let (cc, rc) = ctx_pair(0x25);
    let mut rng = Rng::new(0x6666);

    // Keep the height small: every leaf runs SPX_WOTS_LEN hash chains.
    let heights: Vec<u32> = vec![1, 2, 3];
    for th in heights {
        for &sign_leaf in &[0u32, 1, !0u32] {
            let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
                .map(|_| rng.next_u32() % SPX_WOTS_W as u32)
                .collect();
            let addr = rand_addr(&mut rng);
            let tree_addr = rand_addr(&mut rng);
            let idx_offset = 0u32;
            let leaf_idx = sign_leaf & ((1u32 << th) - 1);

            let mut csig = vec![0xAAu8; SPX_WOTS_BYTES];
            let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES];
            let mut cinfo = LeafInfoX1 {
                wots_sig: csig.as_mut_ptr(),
                wots_sign_leaf: sign_leaf,
                wots_steps: steps.as_mut_ptr(),
                leaf_addr: addr,
                pk_addr: addr,
            };
            let mut rinfo = LeafInfoX1 {
                wots_sig: rsig.as_mut_ptr(),
                wots_sign_leaf: sign_leaf,
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
                    leaf_idx,
                    idx_offset,
                    th,
                    cta.as_mut_ptr(),
                    &mut cinfo,
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    th,
                    rta.as_mut_ptr(),
                    &mut rinfo,
                );
            }
            let what = format!("wots_treehashx1(th={th}, sign_leaf={sign_leaf:#x})");
            assert_bytes_eq(&format!("{what} root"), &croot, &rroot);
            assert_bytes_eq(&format!("{what} auth_path"), &cauth, &rauth);
            assert_bytes_eq(&format!("{what} wots_sig"), &csig, &rsig);
            assert_eq!(cta, rta, "{what} tree_addr side effect");
            assert_eq!(cinfo.leaf_addr, rinfo.leaf_addr, "{what} leaf_addr");
            assert_eq!(cinfo.pk_addr, rinfo.pk_addr, "{what} pk_addr");
        }
    }
}

type FnForsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut ForsGenLeafInfo);

#[test]
fn fors_treehashx1() {
    let l = libs();
    let c = unsafe { l.c::<FnForsTreehashX1>("SPX_fors_treehashx1") };
    let r = unsafe { l.r::<FnForsTreehashX1>("SPX_fors_treehashx1") };
    let (cc, rc) = ctx_pair(0x26);
    let mut rng = Rng::new(0x7777);

    let mut heights: Vec<u32> = vec![1, 2, 3, 4, SPX_FORS_HEIGHT as u32];
    heights.retain(|&h| h <= 8);
    heights.sort_unstable();
    heights.dedup();
    for th in heights {
        for leaf_idx in [0u32, 1, (1u32 << th) - 1] {
            let idx_offset = (rng.next_u32() & 0xff) << th;
            let leaf_addrx = rand_addr(&mut rng);
            let tree_addr = rand_addr(&mut rng);
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
                    leaf_idx,
                    idx_offset,
                    th,
                    cta.as_mut_ptr(),
                    &mut cinfo,
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    th,
                    rta.as_mut_ptr(),
                    &mut rinfo,
                );
            }
            let what = format!("fors_treehashx1(th={th}, leaf_idx={leaf_idx})");
            assert_bytes_eq(&format!("{what} root"), &croot, &rroot);
            assert_bytes_eq(&format!("{what} auth_path"), &cauth, &rauth);
            assert_eq!(cta, rta, "{what} tree_addr side effect");
            assert_eq!(cinfo.leaf_addrx, rinfo.leaf_addrx, "{what} leaf_addrx");
        }
    }
}

// ---------------------------------------------------------------------------
// app/src/fors.c
// ---------------------------------------------------------------------------

type FnForsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);

#[test]
fn fors_gen_leafx1() {
    let l = libs();
    let c = unsafe { l.c::<FnForsGenLeafX1>("SPX_fors_gen_leafx1") };
    let r = unsafe { l.r::<FnForsGenLeafX1>("SPX_fors_gen_leafx1") };
    let (cc, rc) = ctx_pair(0x27);
    let mut rng = Rng::new(0x8888);
    for _ in 0..16 {
        let leaf_addrx = rand_addr(&mut rng);
        let addr_idx = rng.next_u32();
        let mut cinfo = ForsGenLeafInfo { leaf_addrx };
        let mut rinfo = ForsGenLeafInfo { leaf_addrx };
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), cc.as_ptr(), addr_idx, &mut cinfo);
            r(ro.as_mut_ptr(), rc.as_ptr(), addr_idx, &mut rinfo);
        }
        assert_bytes_eq(&format!("fors_gen_leafx1(addr_idx={addr_idx:#x})"), &co, &ro);
        assert_eq!(cinfo.leaf_addrx, rinfo.leaf_addrx, "leaf_addrx side effect");
    }
}

type FnForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);

#[test]
fn fors_sign() {
    let l = libs();
    let c = unsafe { l.c::<FnForsSign>("SPX_fors_sign") };
    let r = unsafe { l.r::<FnForsSign>("SPX_fors_sign") };
    let (cc, rc) = ctx_pair(0x28);
    let mut rng = Rng::new(0x9999);
    for round in 0..2 {
        let m = if round == 0 {
            vec![0u8; SPX_FORS_MSG_BYTES]
        } else {
            rng.vec(SPX_FORS_MSG_BYTES)
        };
        let fors_addr = rand_addr(&mut rng);
        let mut csig = vec![0xAAu8; SPX_FORS_BYTES + 8];
        let mut rsig = vec![0xAAu8; SPX_FORS_BYTES + 8];
        let mut cpk = vec![0xAAu8; SPX_N + 8];
        let mut rpk = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(
                csig.as_mut_ptr(),
                cpk.as_mut_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            r(
                rsig.as_mut_ptr(),
                rpk.as_mut_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        assert_bytes_eq(&format!("fors_sign sig (round={round})"), &csig, &rsig);
        assert_bytes_eq(&format!("fors_sign pk (round={round})"), &cpk, &rpk);

        // And the matching pk_from_sig, fed with the signature just produced.
        let cpks = unsafe { l.c::<FnForsSign>("SPX_fors_pk_from_sig") };
        let rpks = unsafe { l.r::<FnForsSign>("SPX_fors_pk_from_sig") };
        let mut cpk2 = vec![0xAAu8; SPX_N + 8];
        let mut rpk2 = vec![0xAAu8; SPX_N + 8];
        unsafe {
            cpks(
                cpk2.as_mut_ptr(),
                csig.as_ptr() as *mut u8,
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            rpks(
                rpk2.as_mut_ptr(),
                rsig.as_ptr() as *mut u8,
                m.as_ptr(),
                rc.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        assert_bytes_eq(
            &format!("fors_pk_from_sig on fresh signature (round={round})"),
            &cpk2,
            &rpk2,
        );
        assert_bytes_eq(
            &format!("fors_pk_from_sig round trip (round={round})"),
            &cpk,
            &cpk2,
        );
    }
}

#[test]
fn fors_pk_from_sig_random() {
    let l = libs();
    // `fors_pk_from_sig(pk, sig, m, ctx, fors_addr)` has the same shape as
    // `fors_sign` minus the second output.
    type FnForsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
    let c = unsafe { l.c::<FnForsPkFromSig>("SPX_fors_pk_from_sig") };
    let r = unsafe { l.r::<FnForsPkFromSig>("SPX_fors_pk_from_sig") };
    let (cc, rc) = ctx_pair(0x29);
    let mut rng = Rng::new(0xaaaa);
    for _ in 0..3 {
        let sig = rng.vec(SPX_FORS_BYTES);
        let m = rng.vec(SPX_FORS_MSG_BYTES);
        let fors_addr = rand_addr(&mut rng);
        let mut cpk = vec![0xAAu8; SPX_N + 8];
        let mut rpk = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(
                cpk.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            r(
                rpk.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        assert_bytes_eq("fors_pk_from_sig(random sig)", &cpk, &rpk);
    }
}

// ---------------------------------------------------------------------------
// app/src/merkle.c
// ---------------------------------------------------------------------------

type FnMerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);

#[test]
fn merkle_sign() {
    let l = libs();
    let c = unsafe { l.c::<FnMerkleSign>("SPX_merkle_sign") };
    let r = unsafe { l.r::<FnMerkleSign>("SPX_merkle_sign") };
    let (cc, rc) = ctx_pair(0x2a);
    let mut rng = Rng::new(0xbbbb);

    let siglen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
    for &idx_leaf in &[0u32, 1, (1u32 << SPX_TREE_HEIGHT) - 1, !0u32] {
        let root = rng.vec(SPX_N);
        let wots_addr = rand_addr(&mut rng);
        let tree_addr = rand_addr(&mut rng);

        let mut csig = vec![0xAAu8; siglen + 8];
        let mut rsig = vec![0xAAu8; siglen + 8];
        let mut croot = root.clone();
        let mut rroot = root.clone();
        let mut cwa = wots_addr;
        let mut rwa = wots_addr;
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        unsafe {
            c(
                csig.as_mut_ptr(),
                croot.as_mut_ptr(),
                cc.as_ptr(),
                cwa.as_mut_ptr(),
                cta.as_mut_ptr(),
                idx_leaf,
            );
            r(
                rsig.as_mut_ptr(),
                rroot.as_mut_ptr(),
                rc.as_ptr(),
                rwa.as_mut_ptr(),
                rta.as_mut_ptr(),
                idx_leaf,
            );
        }
        let what = format!("merkle_sign(idx_leaf={idx_leaf:#x})");
        assert_bytes_eq(&format!("{what} sig"), &csig, &rsig);
        assert_bytes_eq(&format!("{what} root"), &croot, &rroot);
        assert_eq!(cwa, rwa, "{what} wots_addr side effect");
        assert_eq!(cta, rta, "{what} tree_addr side effect");
    }
}

type FnMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);

#[test]
fn merkle_gen_root() {
    let l = libs();
    let c = unsafe { l.c::<FnMerkleGenRoot>("SPX_merkle_gen_root") };
    let r = unsafe { l.r::<FnMerkleGenRoot>("SPX_merkle_gen_root") };
    for tag in [0x01u8, 0x7f] {
        let (cc, rc) = ctx_pair(tag);
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), cc.as_ptr());
            r(ro.as_mut_ptr(), rc.as_ptr());
        }
        assert_bytes_eq(&format!("merkle_gen_root(tag={tag})"), &co, &ro);
    }
}
