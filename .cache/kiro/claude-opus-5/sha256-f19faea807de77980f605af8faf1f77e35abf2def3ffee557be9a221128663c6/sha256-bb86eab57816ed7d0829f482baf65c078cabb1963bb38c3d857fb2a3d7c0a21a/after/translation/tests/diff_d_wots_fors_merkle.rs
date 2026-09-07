//! CONFIGS.md group D — WOTS / FORS / Merkle, driven through the low-level
//! entry points (`wots.h`, `wotsx1.h`, `forsx1.h`, `utils.h`, `utilsx1.h`,
//! `merkle.h`) rather than only through `crypto_sign*`.

mod common;

use common::*;
use std::os::raw::c_uint;

type ChainLengthsFn = unsafe extern "C" fn(*mut c_uint, *const u8);
type WotsPkFromSigFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
type WotsGenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);
type ForsGenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut u32);
type ComputeRootFn =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
type GenLeafCb = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
type TreehashFn =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeafCb, *mut u32);
type TreehashX1Fn =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1);
type ForsSignFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type ForsPkFromSigFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
type MerkleSignFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
type MerkleGenRootFn = unsafe extern "C" fn(*mut u8, *const u8);

// ---------------------------------------------------------------------------
// D1 / D2 — chain_lengths (base_w + wots_checksum)
// ---------------------------------------------------------------------------

#[test]
fn d1_d2_chain_lengths() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_chain_lengths", ChainLengthsFn);
    let mut rng = Rng::new(601);
    for i in 0..600 {
        let msg: Vec<u8> = match i {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            2 => (0..SPX_N).map(|k| (k * 17) as u8).collect(),
            _ => rng.bytes(SPX_N),
        };
        let mut cl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), msg.as_ptr());
            r(rl.as_mut_ptr(), msg.as_ptr());
        }
        eq(&format!("chain_lengths (msg[0]={:#x})", msg[0]), cl.clone(), rl.clone());
        for k in 0..SPX_WOTS_LEN {
            assert!(cl[k] < SPX_WOTS_W as u32, "chain length {k} out of range: {}", cl[k]);
        }
    }
}

// ---------------------------------------------------------------------------
// D3 — wots_pk_from_sig
// ---------------------------------------------------------------------------

#[test]
fn d3_wots_pk_from_sig() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_wots_pk_from_sig", WotsPkFromSigFn);
    let mut rng = Rng::new(602);
    for i in 0..80 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let sig: Vec<u8> = match i {
            0 => vec![0u8; SPX_WOTS_BYTES],
            1 => vec![0xFFu8; SPX_WOTS_BYTES],
            _ => rng.bytes(SPX_WOTS_BYTES),
        };
        let msg: Vec<u8> = match i {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let addr0 = if i == 2 { [0u32; 8] } else { rng.addr() };
        let mut ca = addr0;
        let mut ra = addr0;
        let mut cpk = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        let mut rpk = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        unsafe {
            c(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
            r(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes("wots_pk_from_sig pk", &cpk, &rpk);
        eq_bytes(
            "wots_pk_from_sig addr",
            &u32s_to_bytes(&ca),
            &u32s_to_bytes(&ra),
        );
    }
}

// ---------------------------------------------------------------------------
// D4 / D5 / D6 / D7 — wots_gen_leafx1
// ---------------------------------------------------------------------------

fn wots_gen_leaf_case(libs: &Libs, seed: u64, sign_leaf: u32, leaf_idx: u32, iters: usize) {
    let (c, r) = pair!(libs, "SPX_wots_gen_leafx1", WotsGenLeafFn);
    let mut rng = Rng::new(seed);
    for i in 0..iters {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(libs, &ps, &sk);

        // wots_steps: the chain positions at which to snapshot the signature.
        // D7 covers 0, W-1 and random.
        let steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| match i % 3 {
                0 => 0,
                1 => (SPX_WOTS_W - 1) as u32,
                _ => rng.below(SPX_WOTS_W as u32),
            })
            .collect();
        let addr0 = rng.addr();

        let mut c_steps = steps.clone();
        let mut r_steps = steps.clone();
        let mut c_sig = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        let mut r_sig = vec![0xAAu8; SPX_WOTS_BYTES + 8];

        let mut c_info = LeafInfoX1::zeroed();
        let mut r_info = LeafInfoX1::zeroed();
        c_info.wots_sig = c_sig.as_mut_ptr();
        r_info.wots_sig = r_sig.as_mut_ptr();
        c_info.wots_sign_leaf = sign_leaf;
        r_info.wots_sign_leaf = sign_leaf;
        c_info.wots_steps = c_steps.as_mut_ptr();
        r_info.wots_steps = r_steps.as_mut_ptr();
        c_info.leaf_addr = addr0;
        r_info.leaf_addr = addr0;
        c_info.pk_addr = addr0;
        r_info.pk_addr = addr0;

        let mut cd = vec![0xAAu8; SPX_N + 8];
        let mut rd = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(cd.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut c_info);
            r(rd.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut r_info);
        }
        eq_bytes("wots_gen_leafx1 dest", &cd, &rd);
        eq_bytes("wots_gen_leafx1 wots_sig", &c_sig, &r_sig);
        eq_bytes(
            "wots_gen_leafx1 leaf_addr",
            &u32s_to_bytes(&c_info.leaf_addr),
            &u32s_to_bytes(&r_info.leaf_addr),
        );
        eq_bytes(
            "wots_gen_leafx1 pk_addr",
            &u32s_to_bytes(&c_info.pk_addr),
            &u32s_to_bytes(&r_info.pk_addr),
        );
        eq("wots_gen_leafx1 wots_steps", c_steps, r_steps);
    }
}


#[test]
fn d4_d7_wots_gen_leafx1_signing() {
    let libs = Libs::load();
    // D4: leaf_idx == wots_sign_leaf, so wots_k_mask = 0 and the WOTS
    // signature is emitted.  D7 varies wots_steps over 0, W-1 and random.
    for leaf in [0u32, 1, 7, 0x1234] {
        wots_gen_leaf_case(&libs, 603 + leaf as u64, leaf, leaf, 12);
    }
}

#[test]
fn d5_wots_gen_leafx1_not_signing() {
    let libs = Libs::load();
    // wots_sign_leaf != leaf_idx -> wots_k_mask = ~0, no signature written.
    wots_gen_leaf_case(&libs, 604, 0xAAAA, 0x5555, 24);
}

#[test]
fn d6_wots_gen_leafx1_all_ones_leaf() {
    let libs = Libs::load();
    // merkle_gen_root passes ~0u as wots_sign_leaf; the signing path only fires
    // when leaf_idx is also ~0u.
    wots_gen_leaf_case(&libs, 605, u32::MAX, u32::MAX, 12);
    wots_gen_leaf_case(&libs, 606, u32::MAX, 0, 12);
}

// ---------------------------------------------------------------------------
// D8 — compute_root
// ---------------------------------------------------------------------------

#[test]
fn d8_compute_root() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_compute_root", ComputeRootFn);
    let mut rng = Rng::new(607);

    let mut heights: Vec<u32> = vec![1, 2, 3, SPX_TREE_HEIGHT as u32, SPX_FORS_HEIGHT as u32];
    heights.sort_unstable();
    heights.dedup();

    for &h in &heights {
        for i in 0..40 {
            let ps = rng.bytes(SPX_N);
            let sk = rng.bytes(SPX_N);
            let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
            let leaf = rng.bytes(SPX_N);
            let auth = rng.bytes(h as usize * SPX_N);
            // Cover both parities of leaf_idx explicitly (the memcpy order
            // branch in utils.c:57) plus random values.
            let leaf_idx = match i {
                0 => 0u32,
                1 => 1,
                2 => 2,
                3 => 3,
                4 => (1u32 << h.min(31)).wrapping_sub(1),
                _ => rng.next_u32(),
            };
            let idx_offset = match i % 3 {
                0 => 0u32,
                1 => 1 << h.min(31),
                _ => rng.next_u32(),
            };
            let addr0 = rng.addr();
            let mut ca = addr0;
            let mut ra = addr0;
            let mut cr = vec![0xAAu8; SPX_N + 8];
            let mut rr = vec![0xAAu8; SPX_N + 8];
            unsafe {
                c(
                    cr.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    rr.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            eq_bytes(
                &format!("compute_root(h={h}, leaf_idx={leaf_idx}, off={idx_offset}) root"),
                &cr,
                &rr,
            );
            eq_bytes(
                &format!("compute_root(h={h}) addr"),
                &u32s_to_bytes(&ca),
                &u32s_to_bytes(&ra),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// D9 — treehash with a caller-supplied gen_leaf function pointer
// ---------------------------------------------------------------------------

/// A deterministic `gen_leaf` shared by both implementations, so the comparison
/// isolates `treehash`'s own stack/auth-path logic.
unsafe extern "C" fn test_gen_leaf(leaf: *mut u8, _ctx: *const u8, addr_idx: u32, tree_addr: *const u32) {
    let out = std::slice::from_raw_parts_mut(leaf, SPX_N);
    let addr = std::slice::from_raw_parts(tree_addr, 8);
    for i in 0..SPX_N {
        let a = addr[i % 8];
        out[i] = (addr_idx as u8)
            .wrapping_mul(31)
            .wrapping_add(i as u8)
            ^ (a >> ((i % 4) * 8)) as u8
            ^ ((addr_idx >> 8) as u8);
    }
}

#[test]
fn d9_treehash() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_treehash", TreehashFn);
    let mut rng = Rng::new(608);

    for &h in &[1u32, 2, 3, 4] {
        for i in 0..30 {
            let ps = rng.bytes(SPX_N);
            let sk = rng.bytes(SPX_N);
            let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
            let leaf_idx = match i {
                0 => 0u32,
                1 => 1,
                2 => (1u32 << h) - 1,
                _ => rng.below(1u32 << h),
            };
            let idx_offset = match i % 3 {
                0 => 0u32,
                1 => 1u32 << h,
                _ => (rng.next_u32() >> h) << h,
            };
            let addr0 = rng.addr();
            let mut ca = addr0;
            let mut ra = addr0;
            let mut croot = vec![0xAAu8; SPX_N + 8];
            let mut rroot = vec![0xAAu8; SPX_N + 8];
            let mut cap = vec![0xAAu8; h as usize * SPX_N + 8];
            let mut rap = vec![0xAAu8; h as usize * SPX_N + 8];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cap.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    test_gen_leaf,
                    ca.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    rap.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    test_gen_leaf,
                    ra.as_mut_ptr(),
                );
            }
            eq_bytes(&format!("treehash(h={h}, leaf={leaf_idx}) root"), &croot, &rroot);
            eq_bytes(&format!("treehash(h={h}, leaf={leaf_idx}) auth_path"), &cap, &rap);
            eq_bytes(
                &format!("treehash(h={h}) addr"),
                &u32s_to_bytes(&ca),
                &u32s_to_bytes(&ra),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// D10 — wots_treehashx1
// ---------------------------------------------------------------------------

#[test]
fn d10_wots_treehashx1() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_wots_treehashx1", TreehashX1Fn);
    let mut rng = Rng::new(609);
    let h = SPX_TREE_HEIGHT as u32;

    for i in 0..24 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let leaf_idx = match i {
            0 => 0u32,
            1 => 1,
            2 => (1u32 << h) - 1,
            3 => u32::MAX, // the merkle_gen_root "no auth path" value
            _ => rng.below(1u32 << h),
        };
        let idx_offset = match i % 3 {
            0 => 0u32,
            1 => 1u32 << h,
            _ => (rng.next_u32() >> h) << h,
        };
        let addr0 = rng.addr();
        let steps: Vec<u32> = (0..SPX_WOTS_LEN).map(|_| rng.below(SPX_WOTS_W as u32)).collect();

        let mut ca = addr0;
        let mut ra = addr0;
        let mut c_steps = steps.clone();
        let mut r_steps = steps.clone();
        let mut c_sig = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        let mut r_sig = vec![0xAAu8; SPX_WOTS_BYTES + 8];
        let mut c_info = LeafInfoX1::zeroed();
        let mut r_info = LeafInfoX1::zeroed();
        c_info.wots_sig = c_sig.as_mut_ptr();
        r_info.wots_sig = r_sig.as_mut_ptr();
        c_info.wots_sign_leaf = leaf_idx;
        r_info.wots_sign_leaf = leaf_idx;
        c_info.wots_steps = c_steps.as_mut_ptr();
        r_info.wots_steps = r_steps.as_mut_ptr();
        c_info.leaf_addr = addr0;
        r_info.leaf_addr = addr0;
        c_info.pk_addr = addr0;
        r_info.pk_addr = addr0;

        let mut croot = vec![0xAAu8; SPX_N + 8];
        let mut rroot = vec![0xAAu8; SPX_N + 8];
        let mut cap = vec![0xAAu8; h as usize * SPX_N + 8];
        let mut rap = vec![0xAAu8; h as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cap.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ca.as_mut_ptr(),
                &mut c_info,
            );
            r(
                rroot.as_mut_ptr(),
                rap.as_mut_ptr(),
                rc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ra.as_mut_ptr(),
                &mut r_info,
            );
        }
        eq_bytes(&format!("wots_treehashx1(leaf={leaf_idx}) root"), &croot, &rroot);
        eq_bytes(&format!("wots_treehashx1(leaf={leaf_idx}) auth_path"), &cap, &rap);
        eq_bytes(
            "wots_treehashx1 wots_sig",
            &c_sig,
            &r_sig,
        );
        eq_bytes(
            "wots_treehashx1 tree_addr",
            &u32s_to_bytes(&ca),
            &u32s_to_bytes(&ra),
        );
        eq_bytes(
            "wots_treehashx1 info.leaf_addr",
            &u32s_to_bytes(&c_info.leaf_addr),
            &u32s_to_bytes(&r_info.leaf_addr),
        );
        eq_bytes(
            "wots_treehashx1 info.pk_addr",
            &u32s_to_bytes(&c_info.pk_addr),
            &u32s_to_bytes(&r_info.pk_addr),
        );
    }
}

// ---------------------------------------------------------------------------
// D11 — fors_treehashx1
// ---------------------------------------------------------------------------

#[test]
fn d11_fors_treehashx1() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_fors_treehashx1", TreehashX1Fn);
    let mut rng = Rng::new(610);
    let h = SPX_FORS_HEIGHT as u32;

    for i in 0..40 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let leaf_idx = match i {
            0 => 0u32,
            1 => 1,
            2 => (1u32 << h) - 1,
            _ => rng.below(1u32 << h),
        };
        // fors_sign uses idx_offset = i * (1 << SPX_FORS_HEIGHT)
        let idx_offset = match i % 3 {
            0 => 0u32,
            1 => (i as u32) * (1u32 << h),
            _ => (rng.below(SPX_FORS_TREES as u32)) * (1u32 << h),
        };
        let addr0 = rng.addr();

        let mut ca = addr0;
        let mut ra = addr0;
        // fors_treehashx1 forwards `info` to fors_gen_leafx1, which reads it as
        // a `fors_gen_leaf_info` (leaf_addrx[8]); allocate the larger
        // leaf_info_x1 so that any over-read is still inside our buffer.
        let mut c_info = LeafInfoX1::zeroed();
        let mut r_info = LeafInfoX1::zeroed();
        let ib = u32s_to_bytes(&addr0);
        unsafe {
            std::ptr::copy_nonoverlapping(ib.as_ptr(), &mut c_info as *mut _ as *mut u8, 32);
            std::ptr::copy_nonoverlapping(ib.as_ptr(), &mut r_info as *mut _ as *mut u8, 32);
        }

        let mut croot = vec![0xAAu8; SPX_N + 8];
        let mut rroot = vec![0xAAu8; SPX_N + 8];
        let mut cap = vec![0xAAu8; h as usize * SPX_N + 8];
        let mut rap = vec![0xAAu8; h as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cap.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ca.as_mut_ptr(),
                &mut c_info,
            );
            r(
                rroot.as_mut_ptr(),
                rap.as_mut_ptr(),
                rc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ra.as_mut_ptr(),
                &mut r_info,
            );
        }
        eq_bytes(&format!("fors_treehashx1(leaf={leaf_idx}) root"), &croot, &rroot);
        eq_bytes(&format!("fors_treehashx1(leaf={leaf_idx}) auth_path"), &cap, &rap);
        eq_bytes(
            "fors_treehashx1 tree_addr",
            &u32s_to_bytes(&ca),
            &u32s_to_bytes(&ra),
        );
        let cib = unsafe { std::slice::from_raw_parts(&c_info as *const _ as *const u8, 32) };
        let rib = unsafe { std::slice::from_raw_parts(&r_info as *const _ as *const u8, 32) };
        eq_bytes("fors_treehashx1 info.leaf_addrx", cib, rib);
    }
}

// ---------------------------------------------------------------------------
// D12 — fors_gen_leafx1
// ---------------------------------------------------------------------------

#[test]
fn d12_fors_gen_leafx1() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_fors_gen_leafx1", ForsGenLeafFn);
    let mut rng = Rng::new(611);
    for i in 0..300 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let addr_idx = match i {
            0 => 0u32,
            1 => 1,
            2 => u32::MAX,
            3 => 0xFFFF_FFFE,
            _ => rng.next_u32(),
        };
        let addr0 = if i == 4 { [0u32; 8] } else { rng.addr() };
        let mut ca = addr0;
        let mut ra = addr0;
        let mut cl = vec![0xAAu8; SPX_N + 8];
        let mut rl = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(cl.as_mut_ptr(), cc.as_ptr(), addr_idx, ca.as_mut_ptr());
            r(rl.as_mut_ptr(), rc.as_ptr(), addr_idx, ra.as_mut_ptr());
        }
        eq_bytes(&format!("fors_gen_leafx1(addr_idx={addr_idx}) leaf"), &cl, &rl);
        eq_bytes(
            "fors_gen_leafx1 leaf_addrx",
            &u32s_to_bytes(&ca),
            &u32s_to_bytes(&ra),
        );
    }
}

// ---------------------------------------------------------------------------
// D13 / D14 / D15 / D16 — fors_sign, fors_pk_from_sig
// ---------------------------------------------------------------------------

#[test]
fn d13_d15_fors_sign_and_pk_from_sig() {
    let libs = Libs::load();
    let (cs, rs) = pair!(libs, "SPX_fors_sign", ForsSignFn);
    let (cv, rv) = pair!(libs, "SPX_fors_pk_from_sig", ForsPkFromSigFn);
    let mut rng = Rng::new(612);

    for i in 0..24 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        // D14: all-zero and all-0xFF messages give the extreme FORS indices.
        let m: Vec<u8> = match i {
            0 => vec![0u8; SPX_FORS_MSG_BYTES],
            1 => vec![0xFFu8; SPX_FORS_MSG_BYTES],
            _ => rng.bytes(SPX_FORS_MSG_BYTES),
        };
        let fors_addr = if i == 2 { [0u32; 8] } else { rng.addr() };

        let mut c_sig = vec![0xAAu8; SPX_FORS_BYTES + 8];
        let mut r_sig = vec![0xAAu8; SPX_FORS_BYTES + 8];
        let mut c_pk = vec![0xAAu8; SPX_N + 8];
        let mut r_pk = vec![0xAAu8; SPX_N + 8];
        unsafe {
            cs(
                c_sig.as_mut_ptr(),
                c_pk.as_mut_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            rs(
                r_sig.as_mut_ptr(),
                r_pk.as_mut_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq_bytes("fors_sign sig", &c_sig, &r_sig);
        eq_bytes("fors_sign pk", &c_pk, &r_pk);

        // D15: round-trip — derive the pk back from the signature.
        let mut c_pk2 = vec![0xAAu8; SPX_N + 8];
        let mut r_pk2 = vec![0xAAu8; SPX_N + 8];
        unsafe {
            cv(
                c_pk2.as_mut_ptr(),
                c_sig.as_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            rv(
                r_pk2.as_mut_ptr(),
                r_sig.as_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq_bytes("fors_pk_from_sig pk (round trip)", &c_pk2, &r_pk2);
        eq_bytes("fors round trip matches fors_sign pk", &c_pk[..SPX_N], &c_pk2[..SPX_N]);
    }
}

#[test]
fn d16_fors_pk_from_sig_random() {
    let libs = Libs::load();
    let (cv, rv) = pair!(libs, "SPX_fors_pk_from_sig", ForsPkFromSigFn);
    let mut rng = Rng::new(613);
    for i in 0..24 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let sig: Vec<u8> = match i {
            0 => vec![0u8; SPX_FORS_BYTES],
            1 => vec![0xFFu8; SPX_FORS_BYTES],
            _ => rng.bytes(SPX_FORS_BYTES),
        };
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let fors_addr = rng.addr();
        let mut c_pk = vec![0xAAu8; SPX_N + 8];
        let mut r_pk = vec![0xAAu8; SPX_N + 8];
        unsafe {
            cv(c_pk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), cc.as_ptr(), fors_addr.as_ptr());
            rv(r_pk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), rc.as_ptr(), fors_addr.as_ptr());
        }
        eq_bytes("fors_pk_from_sig(random sig) pk", &c_pk, &r_pk);
    }
}

// ---------------------------------------------------------------------------
// D17 / D18 / D19 — merkle_sign, merkle_gen_root
// ---------------------------------------------------------------------------

#[test]
fn d17_d18_merkle_sign() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_merkle_sign", MerkleSignFn);
    let mut rng = Rng::new(614);
    let siglen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
    let h = SPX_TREE_HEIGHT as u32;

    for i in 0..16 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let root0: Vec<u8> = match i {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let idx_leaf = match i {
            0 => 0u32,
            1 => 1,
            2 => (1u32 << h) - 1,
            3 => u32::MAX, // D18: merkle_gen_root's "no auth path" value
            _ => rng.below(1u32 << h),
        };
        let wots_addr0 = rng.addr();
        let tree_addr0 = rng.addr();

        let mut cwa = wots_addr0;
        let mut rwa = wots_addr0;
        let mut cta = tree_addr0;
        let mut rta = tree_addr0;
        let mut c_sig = vec![0xAAu8; siglen + 8];
        let mut r_sig = vec![0xAAu8; siglen + 8];
        let mut c_root = root0.clone();
        let mut r_root = root0.clone();
        unsafe {
            c(
                c_sig.as_mut_ptr(),
                c_root.as_mut_ptr(),
                cc.as_ptr(),
                cwa.as_mut_ptr(),
                cta.as_mut_ptr(),
                idx_leaf,
            );
            r(
                r_sig.as_mut_ptr(),
                r_root.as_mut_ptr(),
                rc.as_ptr(),
                rwa.as_mut_ptr(),
                rta.as_mut_ptr(),
                idx_leaf,
            );
        }
        eq_bytes(&format!("merkle_sign(idx_leaf={idx_leaf}) sig"), &c_sig, &r_sig);
        eq_bytes(&format!("merkle_sign(idx_leaf={idx_leaf}) root"), &c_root, &r_root);
        eq_bytes(
            "merkle_sign wots_addr",
            &u32s_to_bytes(&cwa),
            &u32s_to_bytes(&rwa),
        );
        eq_bytes(
            "merkle_sign tree_addr",
            &u32s_to_bytes(&cta),
            &u32s_to_bytes(&rta),
        );
    }
}

#[test]
fn d19_merkle_gen_root() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_merkle_gen_root", MerkleGenRootFn);
    let mut rng = Rng::new(615);
    for i in 0..8 {
        let (ps, sk): (Vec<u8>, Vec<u8>) = match i {
            0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
            1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let mut cr = vec![0xAAu8; SPX_N + 8];
        let mut rr = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(cr.as_mut_ptr(), cc.as_ptr());
            r(rr.as_mut_ptr(), rc.as_ptr());
        }
        eq_bytes("merkle_gen_root root", &cr, &rr);
    }
}
