//! Phase B, CONFIGS.md rows 19-31: the Merkle machinery of `app/src/utils.c`
//! (`compute_root`, `treehash`) and `app/src/utilsx1.c`
//! (`fors_treehashx1`, `wots_treehashx1`), plus `app/src/wotsx1.c`'s
//! `wots_gen_leafx1`.

mod common;

use common::params::*;
use common::*;

type ComputeRoot = unsafe extern "C" fn(
    *mut u8,   // root
    *const u8, // leaf
    u32,       // leaf_idx
    u32,       // idx_offset
    *const u8, // auth_path
    u32,       // tree_height
    *const u8, // ctx
    *mut u32,  // addr
);

type GenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);

type Treehash = unsafe extern "C" fn(
    *mut u8,   // root
    *mut u8,   // auth_path
    *const u8, // ctx
    u32,       // leaf_idx
    u32,       // idx_offset
    u32,       // tree_height
    GenLeafFn, // gen_leaf
    *mut u32,  // tree_addr
);

type TreehashX1 = unsafe extern "C" fn(
    *mut u8,        // root
    *mut u8,        // auth_path
    *const u8,      // ctx
    u32,            // leaf_idx
    u32,            // idx_offset
    u32,            // tree_height
    *mut u32,       // tree_addr
    *mut LeafInfoX1,// info
);

type WotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);

fn compute_root_case(
    _libs: &Libs,
    cc: &Ctx,
    cr: &Ctx,
    fc: &ComputeRoot,
    fr: &ComputeRoot,
    rng: &mut Rng,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
) {
    let leaf = rng.bytes(SPX_N);
    let ap = rng.bytes(tree_height as usize * SPX_N);
    let base = rng.addr();
    let mut aa = base;
    let mut ab = base;
    let mut a = vec![0xA5u8; SPX_N + 8];
    let mut b = vec![0xA5u8; SPX_N + 8];
    unsafe {
        fc(
            a.as_mut_ptr(),
            leaf.as_ptr(),
            leaf_idx,
            idx_offset,
            ap.as_ptr(),
            tree_height,
            cc.ptr(),
            aa.as_mut_ptr(),
        );
        fr(
            b.as_mut_ptr(),
            leaf.as_ptr(),
            leaf_idx,
            idx_offset,
            ap.as_ptr(),
            tree_height,
            cr.ptr(),
            ab.as_mut_ptr(),
        );
    }
    let what = format!("SPX_compute_root(h={tree_height}, idx={leaf_idx}, off={idx_offset})");
    eq(&what, &a, &b);
    eq(&format!("{what} addr"), &u32s_as_bytes(&aa), &u32s_as_bytes(&ab));
}

#[test]
fn row19_compute_root_height1_parity() {
    let libs = load();
    let (fc, fr) = libs.pair::<ComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(19);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for leaf_idx in [0u32, 1, 2, 3, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        for _ in 0..16 {
            compute_root_case(&libs, &cc, &cr, &fc, &fr, &mut rng, leaf_idx, 0, 1);
        }
    }
}

#[test]
fn row20_compute_root_tree_height() {
    let libs = load();
    let (fc, fr) = libs.pair::<ComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(20);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let h = SPX_TREE_HEIGHT as u32;
    let n = 1u32 << h;
    for leaf_idx in [0u32, 1, n / 2, n - 2, n - 1] {
        compute_root_case(&libs, &cc, &cr, &fc, &fr, &mut rng, leaf_idx, 0, h);
    }
    for _ in 0..64 {
        let leaf_idx = rng.below(n);
        compute_root_case(&libs, &cc, &cr, &fc, &fr, &mut rng, leaf_idx, 0, h);
    }
}

#[test]
fn row21_compute_root_fors_height_with_offset() {
    let libs = load();
    let (fc, fr) = libs.pair::<ComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(21);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let h = SPX_FORS_HEIGHT as u32;
    let n = 1u32 << h;
    for i in 0..SPX_FORS_TREES as u32 {
        let idx_offset = i * n;
        for leaf_idx in [0u32, n - 1, rng.below(n)] {
            compute_root_case(&libs, &cc, &cr, &fc, &fr, &mut rng, leaf_idx, idx_offset, h);
        }
    }
}

#[test]
fn row22_compute_root_height_sweep() {
    let libs = load();
    let (fc, fr) = libs.pair::<ComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(22);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let top = SPX_TREE_HEIGHT.max(SPX_FORS_HEIGHT) as u32;
    for h in 1..=top {
        let n = if h >= 32 { u32::MAX } else { 1u32 << h };
        for leaf_idx in [0u32, 1, n - 1, rng.below(n)] {
            for idx_offset in [0u32, n, 3 * n, rng.next_u32() & !(n - 1)] {
                compute_root_case(&libs, &cc, &cr, &fc, &fr, &mut rng, leaf_idx, idx_offset, h);
            }
        }
    }
}

/// Drives the exported `SPX_treehash` through its C function-pointer argument,
/// giving each side its own `SPX_fors_gen_leafx1` as the leaf generator.
fn treehash_case(
    libs: &Libs,
    rng: &mut Rng,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
) {
    let (fc, fr) = libs.pair::<Treehash>("SPX_treehash");
    let (gc, gr) = libs.pair::<GenLeafFn>("SPX_fors_gen_leafx1");
    let (cc, cr) = make_ctx_pair(libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let base = rng.addr();
    let mut aa = base;
    let mut ab = base;
    let apl = tree_height as usize * SPX_N;
    let mut ra = vec![0xA5u8; SPX_N + 8];
    let mut rb = vec![0xA5u8; SPX_N + 8];
    let mut pa = vec![0x5Au8; apl + 8];
    let mut pb = vec![0x5Au8; apl + 8];
    unsafe {
        fc(
            ra.as_mut_ptr(),
            pa.as_mut_ptr(),
            cc.ptr(),
            leaf_idx,
            idx_offset,
            tree_height,
            *gc,
            aa.as_mut_ptr(),
        );
        fr(
            rb.as_mut_ptr(),
            pb.as_mut_ptr(),
            cr.ptr(),
            leaf_idx,
            idx_offset,
            tree_height,
            *gr,
            ab.as_mut_ptr(),
        );
    }
    let what = format!("SPX_treehash(h={tree_height}, idx={leaf_idx}, off={idx_offset})");
    eq(&format!("{what} root"), &ra, &rb);
    eq(&format!("{what} auth_path"), &pa, &pb);
    eq(&format!("{what} tree_addr"), &u32s_as_bytes(&aa), &u32s_as_bytes(&ab));
}

#[test]
fn row23_treehash_small() {
    let libs = load();
    let mut rng = Rng::new(23);
    for h in 1..=2u32 {
        for leaf_idx in 0..(1u32 << h) {
            treehash_case(&libs, &mut rng, leaf_idx, 0, h);
        }
    }
}

#[test]
fn row24_treehash_fors_height() {
    let libs = load();
    let mut rng = Rng::new(24);
    let h = SPX_FORS_HEIGHT as u32;
    let n = 1u32 << h;
    for i in [0u32, 1, (SPX_FORS_TREES as u32) - 1] {
        for leaf_idx in [0u32, n - 1, rng.below(n)] {
            treehash_case(&libs, &mut rng, leaf_idx, i * n, h);
        }
    }
    // also the Merkle subtree height, which is what merkle_sign uses
    let hm = SPX_TREE_HEIGHT as u32;
    let nm = 1u32 << hm;
    for leaf_idx in [0u32, nm - 1, rng.below(nm)] {
        treehash_case(&libs, &mut rng, leaf_idx, 0, hm);
    }
}

fn fors_treehashx1_case(libs: &Libs, rng: &mut Rng, leaf_idx: u32, idx_offset: u32, h: u32) {
    let (fc, fr) = libs.pair::<TreehashX1>("SPX_fors_treehashx1");
    let (cc, cr) = make_ctx_pair(libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let base = rng.addr();
    // fors_treehashx1's `info` is really a fors_gen_leaf_info (uint32_t[8]);
    // the C prototype in utilsx1.h says leaf_info_x1*, and the reference code
    // relies on the first member overlapping.  Use the wider struct so both
    // sides see identical memory.
    let mut ia = LeafInfoX1::zeroed();
    let mut ib = LeafInfoX1::zeroed();
    let info_addr = rng.addr();
    // leaf_addrx of fors_gen_leaf_info aliases the head of leaf_info_x1, i.e.
    // { wots_sig, wots_sign_leaf, wots_steps } — 8 words on LP64.
    let head = u32s_as_bytes(&info_addr);
    unsafe {
        core::ptr::copy_nonoverlapping(head.as_ptr(), &mut ia as *mut _ as *mut u8, 32);
        core::ptr::copy_nonoverlapping(head.as_ptr(), &mut ib as *mut _ as *mut u8, 32);
    }
    let mut aa = base;
    let mut ab = base;
    let apl = h as usize * SPX_N;
    let mut ra = vec![0xA5u8; SPX_N + 8];
    let mut rb = vec![0xA5u8; SPX_N + 8];
    let mut pa = vec![0x5Au8; apl + 8];
    let mut pb = vec![0x5Au8; apl + 8];
    unsafe {
        fc(
            ra.as_mut_ptr(),
            pa.as_mut_ptr(),
            cc.ptr(),
            leaf_idx,
            idx_offset,
            h,
            aa.as_mut_ptr(),
            &mut ia,
        );
        fr(
            rb.as_mut_ptr(),
            pb.as_mut_ptr(),
            cr.ptr(),
            leaf_idx,
            idx_offset,
            h,
            ab.as_mut_ptr(),
            &mut ib,
        );
    }
    let what = format!("SPX_fors_treehashx1(h={h}, idx={leaf_idx}, off={idx_offset})");
    eq(&format!("{what} root"), &ra, &rb);
    eq(&format!("{what} auth_path"), &pa, &pb);
    eq(&format!("{what} tree_addr"), &u32s_as_bytes(&aa), &u32s_as_bytes(&ab));
    unsafe {
        let sa = core::slice::from_raw_parts(&ia as *const _ as *const u8, 32);
        let sb = core::slice::from_raw_parts(&ib as *const _ as *const u8, 32);
        eq(&format!("{what} leaf_addrx"), sa, sb);
    }
}

#[test]
fn row25_fors_treehashx1_fors_height() {
    let libs = load();
    let mut rng = Rng::new(25);
    let h = SPX_FORS_HEIGHT as u32;
    let n = 1u32 << h;
    for i in [0u32, 1, (SPX_FORS_TREES as u32) - 1] {
        for leaf_idx in [0u32, 1, n - 1, rng.below(n)] {
            fors_treehashx1_case(&libs, &mut rng, leaf_idx, i * n, h);
        }
    }
}

#[test]
fn row26_fors_treehashx1_small_trees() {
    let libs = load();
    let mut rng = Rng::new(26);
    for h in 1..=3u32 {
        for leaf_idx in 0..(1u32 << h) {
            for off in [0u32, 1u32 << h, 5u32 << h] {
                fors_treehashx1_case(&libs, &mut rng, leaf_idx, off, h);
            }
        }
    }
}

fn wots_gen_leafx1_case(libs: &Libs, rng: &mut Rng, leaf_idx: u32, sign_leaf: u32, steps_mode: u8) {
    let (fc, fr) = libs.pair::<WotsGenLeafX1>("SPX_wots_gen_leafx1");
    let (cc, cr) = make_ctx_pair(libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));

    let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
        .map(|i| match steps_mode {
            0 => 0,
            1 => (SPX_WOTS_W - 1) as u32,
            2 => (i % SPX_WOTS_W) as u32,
            _ => rng.below(SPX_WOTS_W as u32),
        })
        .collect();

    let mut siga = vec![0xA5u8; SPX_WOTS_BYTES + 8];
    let mut sigb = vec![0xA5u8; SPX_WOTS_BYTES + 8];
    let leaf_addr = rng.addr();
    let pk_addr = rng.addr();

    let mut ia = LeafInfoX1::zeroed();
    ia.wots_sig = siga.as_mut_ptr();
    ia.wots_sign_leaf = sign_leaf;
    ia.wots_steps = steps.as_mut_ptr();
    ia.leaf_addr = leaf_addr;
    ia.pk_addr = pk_addr;
    let mut ib = ia;
    ib.wots_sig = sigb.as_mut_ptr();

    let mut da = vec![0x5Au8; SPX_N + 8];
    let mut db = vec![0x5Au8; SPX_N + 8];
    unsafe {
        fc(da.as_mut_ptr(), cc.ptr(), leaf_idx, &mut ia);
        fr(db.as_mut_ptr(), cr.ptr(), leaf_idx, &mut ib);
    }
    let what = format!("SPX_wots_gen_leafx1(leaf={leaf_idx}, sign={sign_leaf}, steps={steps_mode})");
    eq(&format!("{what} dest"), &da, &db);
    eq(&format!("{what} wots_sig"), &siga, &sigb);
    eq(
        &format!("{what} leaf_addr"),
        &u32s_as_bytes(&ia.leaf_addr),
        &u32s_as_bytes(&ib.leaf_addr),
    );
    eq(
        &format!("{what} pk_addr"),
        &u32s_as_bytes(&ia.pk_addr),
        &u32s_as_bytes(&ib.pk_addr),
    );
    assert_eq!(ia.wots_sign_leaf, ib.wots_sign_leaf);
}

#[test]
fn row27_wots_gen_leafx1_not_signing() {
    let libs = load();
    let mut rng = Rng::new(27);
    for leaf_idx in [0u32, 1, 7, 0xFFFF_FFFE] {
        for mode in 0..4u8 {
            // sign_leaf deliberately different from leaf_idx: wots_k_mask = ~0
            wots_gen_leafx1_case(&libs, &mut rng, leaf_idx, leaf_idx ^ 0x8000_0001, mode);
        }
    }
    // the merkle_gen_root sentinel
    for leaf_idx in [0u32, 1, 5] {
        wots_gen_leafx1_case(&libs, &mut rng, leaf_idx, u32::MAX, 3);
    }
}

#[test]
fn row28_wots_gen_leafx1_signing() {
    let libs = load();
    let mut rng = Rng::new(28);
    for leaf_idx in [0u32, 1, 3, 0xFFFF_FFFF] {
        for mode in 0..4u8 {
            wots_gen_leafx1_case(&libs, &mut rng, leaf_idx, leaf_idx, mode);
        }
    }
}

fn wots_treehashx1_case(libs: &Libs, rng: &mut Rng, leaf_idx: u32, sign_leaf: u32, h: u32) {
    let (fc, fr) = libs.pair::<TreehashX1>("SPX_wots_treehashx1");
    let (cc, cr) = make_ctx_pair(libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
        .map(|_| rng.below(SPX_WOTS_W as u32))
        .collect();
    let mut siga = vec![0xA5u8; SPX_WOTS_BYTES + 8];
    let mut sigb = vec![0xA5u8; SPX_WOTS_BYTES + 8];
    let leaf_addr = rng.addr();
    let pk_addr = rng.addr();

    let mut ia = LeafInfoX1::zeroed();
    ia.wots_sig = siga.as_mut_ptr();
    ia.wots_sign_leaf = sign_leaf;
    ia.wots_steps = steps.as_mut_ptr();
    ia.leaf_addr = leaf_addr;
    ia.pk_addr = pk_addr;
    let mut ib = ia;
    ib.wots_sig = sigb.as_mut_ptr();

    let base = rng.addr();
    let mut aa = base;
    let mut ab = base;
    let apl = h as usize * SPX_N;
    let mut ra = vec![0x11u8; SPX_N + 8];
    let mut rb = vec![0x11u8; SPX_N + 8];
    let mut pa = vec![0x22u8; apl + 8];
    let mut pb = vec![0x22u8; apl + 8];
    unsafe {
        fc(
            ra.as_mut_ptr(),
            pa.as_mut_ptr(),
            cc.ptr(),
            leaf_idx,
            0,
            h,
            aa.as_mut_ptr(),
            &mut ia,
        );
        fr(
            rb.as_mut_ptr(),
            pb.as_mut_ptr(),
            cr.ptr(),
            leaf_idx,
            0,
            h,
            ab.as_mut_ptr(),
            &mut ib,
        );
    }
    let what = format!("SPX_wots_treehashx1(h={h}, idx={leaf_idx}, sign={sign_leaf})");
    eq(&format!("{what} root"), &ra, &rb);
    eq(&format!("{what} auth_path"), &pa, &pb);
    eq(&format!("{what} tree_addr"), &u32s_as_bytes(&aa), &u32s_as_bytes(&ab));
    eq(&format!("{what} wots_sig"), &siga, &sigb);
    eq(
        &format!("{what} leaf_addr"),
        &u32s_as_bytes(&ia.leaf_addr),
        &u32s_as_bytes(&ib.leaf_addr),
    );
    eq(
        &format!("{what} pk_addr"),
        &u32s_as_bytes(&ia.pk_addr),
        &u32s_as_bytes(&ib.pk_addr),
    );
}

#[test]
fn row29_wots_treehashx1_signing() {
    let libs = load();
    let mut rng = Rng::new(29);
    let h = SPX_TREE_HEIGHT as u32;
    let n = 1u32 << h;
    for leaf_idx in [0u32, 1, n / 2, n - 1] {
        wots_treehashx1_case(&libs, &mut rng, leaf_idx, leaf_idx, h);
    }
}

#[test]
fn row30_wots_treehashx1_sentinel() {
    let libs = load();
    let mut rng = Rng::new(30);
    let h = SPX_TREE_HEIGHT as u32;
    // the merkle_gen_root case: no leaf index can equal ~0
    wots_treehashx1_case(&libs, &mut rng, u32::MAX, u32::MAX, h);
    wots_treehashx1_case(&libs, &mut rng, 0, u32::MAX, h);
}

#[test]
fn row31_wots_treehashx1_small() {
    let libs = load();
    let mut rng = Rng::new(31);
    for h in 1..=2u32 {
        for leaf_idx in 0..(1u32 << h) {
            wots_treehashx1_case(&libs, &mut rng, leaf_idx, leaf_idx, h);
            wots_treehashx1_case(&libs, &mut rng, leaf_idx, u32::MAX, h);
        }
    }
}
