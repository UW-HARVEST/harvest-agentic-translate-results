//! Phase B rows 19-31: WOTS / FORS leaf generation, compute_root, the generic
//! function-pointer `treehash`, and the specialised `*_treehashx1` builders.
mod common;
use common::*;

type FChainLengths = unsafe extern "C" fn(*mut u32, *const u8);
type FWotsPk = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
type FWotsLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfo);
type FForsLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsInfo);
type FComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
type GenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
type FTreehash =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeaf, *mut u32);
type FTreehashX1W =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfo);
type FTreehashX1F =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut ForsInfo);

/// `struct leaf_info_x1` from `app/include/wotsx1.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LeafInfo {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *const u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// `struct fors_gen_leaf_info` from `app/include/fors.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForsInfo {
    pub leaf_addrx: [u32; 8],
}

fn leaf_info_image(i: &LeafInfo) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&i.wots_sign_leaf.to_ne_bytes());
    v.extend_from_slice(&addr_to_bytes(&i.leaf_addr));
    v.extend_from_slice(&addr_to_bytes(&i.pk_addr));
    v
}

// ---------- row 19 ----------
#[test]
fn b19_chain_lengths() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 19);
    unsafe {
        let cf = sym!(p.c, b"SPX_chain_lengths\0", FChainLengths);
        let rf = sym!(p.r, b"SPX_chain_lengths\0", FChainLengths);
        let mut msgs: Vec<Vec<u8>> = vec![vec![0u8; N], vec![0xffu8; N], vec![0x0fu8; N], vec![0xf0u8; N]];
        for _ in 0..200 {
            msgs.push(rng.bytes(N));
        }
        for m in msgs {
            let mut cl = vec![0xEEEE_EEEEu32; WOTS_LEN + 8];
            let mut rl = vec![0xEEEE_EEEEu32; WOTS_LEN + 8];
            cf(cl.as_mut_ptr(), m.as_ptr());
            rf(rl.as_mut_ptr(), m.as_ptr());
            eqv("chain_lengths", &cl, &rl);
        }
    }
}

// ---------- row 20 ----------
#[test]
fn b20_wots_pk_from_sig() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 20);
    unsafe {
        let cf = sym!(p.c, b"SPX_wots_pk_from_sig\0", FWotsPk);
        let rf = sym!(p.r, b"SPX_wots_pk_from_sig\0", FWotsPk);
        for it in 0..12 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let sig = rng.bytes(WOTS_BYTES);
            let msg = match it {
                0 => vec![0u8; N],
                1 => vec![0xffu8; N],
                _ => rng.bytes(N),
            };
            let a = rng.addr();
            let mut ca = a;
            let mut ra = a;
            let mut cpk = obuf(WOTS_BYTES);
            let mut rpk = obuf(WOTS_BYTES);
            cf(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
            rf(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
            eqb("wots_pk_from_sig pk", &cpk, &rpk);
            eqb("wots_pk_from_sig addr", &addr_to_bytes(&ca), &addr_to_bytes(&ra));
        }
    }
}

// ---------- rows 21 & 22 ----------
unsafe fn wots_leaf_case(p: &Pair, rng: &mut Rng, signing: bool, iters: usize) {
    let cf = sym!(p.c, b"SPX_wots_gen_leafx1\0", FWotsLeaf);
    let rf = sym!(p.r, b"SPX_wots_gen_leafx1\0", FWotsLeaf);
    for it in 0..iters {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);

        // steps: mostly in range 0..WOTS_W-1, sometimes out of range
        let mut steps = vec![0u32; WOTS_LEN];
        for s in steps.iter_mut() {
            *s = match it % 4 {
                0 => rng.below(WOTS_W as u32),
                1 => 0,
                2 => WOTS_W as u32 - 1,
                _ => rng.next_u32(), // deliberately out of range
            };
        }

        let leaf_idx = rng.next_u32();
        let a = rng.addr();

        let mut csig = obuf(WOTS_BYTES);
        let mut rsig = obuf(WOTS_BYTES);
        let mut ci = LeafInfo {
            wots_sig: csig.as_mut_ptr(),
            wots_sign_leaf: if signing { leaf_idx } else { leaf_idx.wrapping_add(1) },
            wots_steps: steps.as_ptr(),
            leaf_addr: a,
            pk_addr: a,
        };
        let mut ri = LeafInfo { wots_sig: rsig.as_mut_ptr(), ..ci };

        let mut cd = obuf(N);
        let mut rd = obuf(N);
        cf(cd.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut ci);
        rf(rd.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut ri);

        eqb("wots_gen_leafx1 dest", &cd, &rd);
        eqb("wots_gen_leafx1 wots_sig", &csig, &rsig);
        eqb("wots_gen_leafx1 info", &leaf_info_image(&ci), &leaf_info_image(&ri));
    }
}

#[test]
fn b21_wots_gen_leafx1_signing() {
    let p = pair();
    unsafe { wots_leaf_case(p, &mut Rng::new(SEED ^ 21), true, 16) }
}

#[test]
fn b22_wots_gen_leafx1_pkonly() {
    let p = pair();
    unsafe { wots_leaf_case(p, &mut Rng::new(SEED ^ 22), false, 16) }
}

// ---------- row 23 ----------
#[test]
fn b23_fors_gen_leafx1() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 23);
    unsafe {
        let cf = sym!(p.c, b"SPX_fors_gen_leafx1\0", FForsLeaf);
        let rf = sym!(p.r, b"SPX_fors_gen_leafx1\0", FForsLeaf);
        for it in 0..64 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let a = rng.addr();
            let idx = match it {
                0 => 0u32,
                1 => 0xFFFF_FFFF,
                2 => 0xFF,
                3 => 0x100,
                _ => rng.next_u32(),
            };
            let mut ci = ForsInfo { leaf_addrx: a };
            let mut ri = ForsInfo { leaf_addrx: a };
            let mut cl = obuf(N);
            let mut rl = obuf(N);
            cf(cl.as_mut_ptr(), cc.as_ptr(), idx, &mut ci);
            rf(rl.as_mut_ptr(), rc.as_ptr(), idx, &mut ri);
            eqb("fors_gen_leafx1 leaf", &cl, &rl);
            eqb(
                "fors_gen_leafx1 info",
                &addr_to_bytes(&ci.leaf_addrx),
                &addr_to_bytes(&ri.leaf_addrx),
            );
        }
    }
}

// ---------- rows 24-26 ----------
unsafe fn compute_root_case(p: &Pair, rng: &mut Rng, tree_height: u32, iters: usize) {
    let cf = sym!(p.c, b"SPX_compute_root\0", FComputeRoot);
    let rf = sym!(p.r, b"SPX_compute_root\0", FComputeRoot);
    let max_leaf = if tree_height >= 32 { u32::MAX } else { (1u32 << tree_height) - 1 };
    for it in 0..iters {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);
        let leaf = rng.bytes(N);
        let ap = rng.bytes(tree_height as usize * N);
        let leaf_idx = match it % 5 {
            0 => 0,
            1 => max_leaf,
            2 => rng.next_u32() & max_leaf | 1,       // odd
            3 => rng.next_u32() & max_leaf & !1,      // even
            _ => rng.next_u32(),                      // unconstrained
        };
        let idx_offset = match it % 3 {
            0 => 0,
            1 => rng.next_u32(),
            _ => (rng.below(64)) << tree_height,
        };
        let a = rng.addr();
        let mut ca = a;
        let mut ra = a;
        let mut cr = obuf(N);
        let mut rr = obuf(N);
        cf(cr.as_mut_ptr(), leaf.as_ptr(), leaf_idx, idx_offset, ap.as_ptr(), tree_height, cc.as_ptr(), ca.as_mut_ptr());
        rf(rr.as_mut_ptr(), leaf.as_ptr(), leaf_idx, idx_offset, ap.as_ptr(), tree_height, rc.as_ptr(), ra.as_mut_ptr());
        eqb(&format!("compute_root h={tree_height} leaf_idx={leaf_idx} off={idx_offset}"), &cr, &rr);
        eqb(&format!("compute_root addr h={tree_height}"), &addr_to_bytes(&ca), &addr_to_bytes(&ra));
    }
}

#[test]
fn b24_compute_root_fors_height() {
    let p = pair();
    unsafe { compute_root_case(p, &mut Rng::new(SEED ^ 24), FORS_HEIGHT as u32, 40) }
}

#[test]
fn b25_compute_root_tree_height() {
    let p = pair();
    unsafe { compute_root_case(p, &mut Rng::new(SEED ^ 25), TREE_HEIGHT as u32, 40) }
}

#[test]
fn b26_compute_root_small() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    for h in [1u32, 2, 3] {
        unsafe { compute_root_case(p, &mut rng, h, 20) }
    }
}

// ---------- rows 27 & 28 ----------
/// Deterministic leaf generator with a C ABI, shared by both libraries.
/// Depends on `addr_idx` AND on the bytes of `tree_addr`, so any address
/// divergence shows up in the root/auth path.
unsafe extern "C" fn test_gen_leaf(leaf: *mut u8, _ctx: *const u8, addr_idx: u32, tree_addr: *const u32) {
    let ta = core::slice::from_raw_parts(tree_addr as *const u8, 32);
    let mut acc = (addr_idx as u64) ^ 0x9E37_79B9_7F4A_7C15;
    for i in 0..N {
        acc = acc
            .wrapping_mul(6364136223846793005)
            .wrapping_add((ta[i % 32] as u64) ^ (i as u64) ^ 1);
        acc ^= acc >> 29;
        *leaf.add(i) = (acc >> 33) as u8;
    }
}

unsafe fn treehash_case(p: &Pair, rng: &mut Rng, tree_height: u32, leaf_idx_kind: u8, iters: usize) {
    let cf = sym!(p.c, b"SPX_treehash\0", FTreehash);
    let rf = sym!(p.r, b"SPX_treehash\0", FTreehash);
    let max_leaf = (1u32 << tree_height) - 1;
    for it in 0..iters {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);
        let leaf_idx = match leaf_idx_kind {
            0 => match it % 4 {
                0 => 0,
                1 => max_leaf,
                2 => rng.next_u32() & max_leaf | 1,
                _ => rng.next_u32() & max_leaf & !1,
            },
            _ => u32::MAX,
        };
        let idx_offset = if it % 2 == 0 { 0 } else { rng.next_u32() };
        let a = rng.addr();
        let mut ca = a;
        let mut ra = a;
        let mut croot = obuf(N);
        let mut rroot = obuf(N);
        let mut cap = obuf(tree_height as usize * N);
        let mut rap = obuf(tree_height as usize * N);
        cf(croot.as_mut_ptr(), cap.as_mut_ptr(), cc.as_ptr(), leaf_idx, idx_offset, tree_height, test_gen_leaf, ca.as_mut_ptr());
        rf(rroot.as_mut_ptr(), rap.as_mut_ptr(), rc.as_ptr(), leaf_idx, idx_offset, tree_height, test_gen_leaf, ra.as_mut_ptr());
        eqb(&format!("treehash root h={tree_height} leaf={leaf_idx}"), &croot, &rroot);
        eqb(&format!("treehash auth h={tree_height} leaf={leaf_idx}"), &cap, &rap);
        eqb("treehash addr", &addr_to_bytes(&ca), &addr_to_bytes(&ra));
    }
}

#[test]
fn b27_treehash_callback() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 27);
    for h in [1u32, 2, 3, 4] {
        unsafe { treehash_case(p, &mut rng, h, 0, 12) }
    }
}

#[test]
fn b28_treehash_no_authpath() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 28);
    for h in [1u32, 2, 3, 4] {
        unsafe { treehash_case(p, &mut rng, h, 1, 6) }
    }
}

// ---------- row 29 ----------
#[test]
fn b29_fors_treehashx1() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 29);
    let th = FORS_HEIGHT as u32;
    let max_leaf = (1u32 << th) - 1;
    unsafe {
        let cf = sym!(p.c, b"SPX_fors_treehashx1\0", FTreehashX1F);
        let rf = sym!(p.r, b"SPX_fors_treehashx1\0", FTreehashX1F);
        for it in 0..16 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let leaf_idx = match it % 4 {
                0 => 0,
                1 => max_leaf,
                2 => rng.next_u32() & max_leaf,
                _ => u32::MAX,
            };
            let idx_offset = (rng.below(FORS_TREES as u32)) << th;
            let a = rng.addr();
            let mut ca = a;
            let mut ra = a;
            let mut ci = ForsInfo { leaf_addrx: rng.addr() };
            let mut ri = ForsInfo { leaf_addrx: ci.leaf_addrx };
            let mut croot = obuf(N);
            let mut rroot = obuf(N);
            let mut cap = obuf(th as usize * N);
            let mut rap = obuf(th as usize * N);
            cf(croot.as_mut_ptr(), cap.as_mut_ptr(), cc.as_ptr(), leaf_idx, idx_offset, th, ca.as_mut_ptr(), &mut ci);
            rf(rroot.as_mut_ptr(), rap.as_mut_ptr(), rc.as_ptr(), leaf_idx, idx_offset, th, ra.as_mut_ptr(), &mut ri);
            eqb("fors_treehashx1 root", &croot, &rroot);
            eqb("fors_treehashx1 auth", &cap, &rap);
            eqb("fors_treehashx1 addr", &addr_to_bytes(&ca), &addr_to_bytes(&ra));
            eqb("fors_treehashx1 info", &addr_to_bytes(&ci.leaf_addrx), &addr_to_bytes(&ri.leaf_addrx));
        }
    }
}

// ---------- rows 30 & 31 ----------
unsafe fn wots_treehashx1_case(p: &Pair, rng: &mut Rng, root_only: bool, iters: usize) {
    let cf = sym!(p.c, b"SPX_wots_treehashx1\0", FTreehashX1W);
    let rf = sym!(p.r, b"SPX_wots_treehashx1\0", FTreehashX1W);
    let th = TREE_HEIGHT as u32;
    let max_leaf = (1u32 << th) - 1;
    for it in 0..iters {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);
        let leaf_idx = if root_only {
            u32::MAX
        } else {
            match it % 3 {
                0 => 0,
                1 => max_leaf,
                _ => rng.next_u32() & max_leaf,
            }
        };
        let mut steps = vec![0u32; WOTS_LEN];
        for s in steps.iter_mut() {
            *s = rng.below(WOTS_W as u32);
        }
        let a = rng.addr();
        let mut ca = a;
        let mut ra = a;
        let mut csig = obuf(WOTS_BYTES);
        let mut rsig = obuf(WOTS_BYTES);
        let ia = rng.addr();
        let pa = rng.addr();
        let mut ci = LeafInfo {
            wots_sig: csig.as_mut_ptr(),
            wots_sign_leaf: leaf_idx,
            wots_steps: steps.as_ptr(),
            leaf_addr: ia,
            pk_addr: pa,
        };
        let mut ri = LeafInfo { wots_sig: rsig.as_mut_ptr(), ..ci };
        let mut croot = obuf(N);
        let mut rroot = obuf(N);
        let mut cap = obuf(th as usize * N);
        let mut rap = obuf(th as usize * N);
        cf(croot.as_mut_ptr(), cap.as_mut_ptr(), cc.as_ptr(), leaf_idx, 0, th, ca.as_mut_ptr(), &mut ci);
        rf(rroot.as_mut_ptr(), rap.as_mut_ptr(), rc.as_ptr(), leaf_idx, 0, th, ra.as_mut_ptr(), &mut ri);
        eqb("wots_treehashx1 root", &croot, &rroot);
        eqb("wots_treehashx1 auth", &cap, &rap);
        eqb("wots_treehashx1 sig", &csig, &rsig);
        eqb("wots_treehashx1 addr", &addr_to_bytes(&ca), &addr_to_bytes(&ra));
        eqb("wots_treehashx1 info", &leaf_info_image(&ci), &leaf_info_image(&ri));
    }
}

#[test]
fn b30_wots_treehashx1_signing() {
    let p = pair();
    unsafe { wots_treehashx1_case(p, &mut Rng::new(SEED ^ 30), false, 6) }
}

#[test]
fn b31_wots_treehashx1_root_only() {
    let p = pair();
    unsafe { wots_treehashx1_case(p, &mut Rng::new(SEED ^ 31), true, 4) }
}
