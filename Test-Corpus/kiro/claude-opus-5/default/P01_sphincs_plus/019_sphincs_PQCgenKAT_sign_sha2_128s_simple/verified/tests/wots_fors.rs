//! Phase B, levels 2 and 3 — WOTS / FORS / Merkle (`CONFIGS.md` rows C32–C44).

mod common;
use common::*;

type ChainLengths = unsafe extern "C" fn(*mut u32, *const u8);
type WotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
type WotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);
type ForsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);
type WotsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1);
type ForsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut ForsGenLeafInfo);
type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type ForsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
type MerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
type MerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);
type SetU32 = unsafe extern "C" fn(*mut u32, u32);

fn abytes(a: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, 32) }
}

/// The three interesting `msg` shapes for `chain_lengths`: both extremes plus
/// random (drives `gen_chain` to `start=0,steps=W-1` and `start=W-1,steps=0`).
fn msg_cases(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v = vec![vec![0u8; n], vec![0xffu8; n]];
    for _ in 0..6 {
        v.push(rng.bytes(n));
    }
    v
}

/* ---- C32 --------------------------------------------------------- */

#[test]
fn c32_chain_lengths() {
    let l = libs();
    let (c, r) = l.pair::<ChainLengths>("SPX_chain_lengths");
    let mut rng = Rng::new(SEED + 32);
    for msg in msg_cases(&mut rng, SPX_N) {
        let mut cl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), msg.as_ptr());
            r(rl.as_mut_ptr(), msg.as_ptr());
        }
        assert_eq!(cl, rl, "chain_lengths({})", hex(&msg));
        for (i, &x) in cl[..SPX_WOTS_LEN].iter().enumerate() {
            assert!(x < SPX_WOTS_W, "chain_lengths[{i}] = {x} >= w");
        }
        assert_eq!(&cl[SPX_WOTS_LEN..], &[0xDEAD_BEEFu32; 4], "wrote past SPX_WOTS_LEN");
    }
}

/* ---- C33 --------------------------------------------------------- */

#[test]
fn c33_wots_pk_from_sig() {
    let l = libs();
    let (c, r) = l.pair::<WotsPkFromSig>("SPX_wots_pk_from_sig");
    let (ckp, rkp) = l.pair::<SetU32>("SPX_set_keypair_addr");
    let (cty, rty) = l.pair::<SetU32>("SPX_set_type");
    let mut rng = Rng::new(SEED + 33);
    for msg in msg_cases(&mut rng, SPX_N) {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        let kp = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);
        unsafe {
            cty(ca.as_mut_ptr(), 0);
            rty(ra.as_mut_ptr(), 0);
            ckp(ca.as_mut_ptr(), kp);
            rkp(ra.as_mut_ptr(), kp);
        }
        let mut cpk = vec![0xAAu8; SPX_WOTS_BYTES + 16];
        let mut rpk = vec![0xAAu8; SPX_WOTS_BYTES + 16];
        unsafe {
            c(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
            r(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes("wots_pk_from_sig pk", &cpk, &rpk);
        assert_eq!(&cpk[SPX_WOTS_BYTES..], &[0xAAu8; 16], "wrote past SPX_WOTS_BYTES");
        eq_bytes("wots_pk_from_sig mutated addr", abytes(&ca), abytes(&ra));
    }
}

/* ---- C34 --------------------------------------------------------- */

#[test]
fn c34_wots_gen_leafx1_signing() {
    let l = libs();
    let (c, r) = l.pair::<WotsGenLeafX1>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(SEED + 34);
    for _ in 0..NUM_ITERS {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let leaf_idx = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);
        // wots_steps values are chain lengths, i.e. in 0..w
        let steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| rng.below(SPX_WOTS_W as u64) as u32)
            .collect();
        let base_leaf = rng.addr();
        let base_pk = rng.addr();

        let mut csteps = steps.clone();
        let mut rsteps = steps.clone();
        let mut csig = vec![0xAAu8; SPX_WOTS_BYTES + 16];
        let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES + 16];
        let mut cinfo = LeafInfoX1 {
            wots_sig: csig.as_mut_ptr(),
            wots_sign_leaf: leaf_idx, // == leaf_idx => signing mode
            wots_steps: csteps.as_mut_ptr(),
            leaf_addr: base_leaf,
            pk_addr: base_pk,
        };
        let mut rinfo = LeafInfoX1 {
            wots_sig: rsig.as_mut_ptr(),
            wots_sign_leaf: leaf_idx,
            wots_steps: rsteps.as_mut_ptr(),
            leaf_addr: base_leaf,
            pk_addr: base_pk,
        };
        let mut cdest = vec![0xBBu8; SPX_N + 16];
        let mut rdest = vec![0xBBu8; SPX_N + 16];
        unsafe {
            c(cdest.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut cinfo);
            r(rdest.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut rinfo);
        }
        eq_bytes("wots_gen_leafx1 leaf", &cdest, &rdest);
        assert_eq!(&cdest[SPX_N..], &[0xBBu8; 16], "leaf wrote past SPX_N");
        eq_bytes("wots_gen_leafx1 wots_sig", &csig, &rsig);
        assert_eq!(&csig[SPX_WOTS_BYTES..], &[0xAAu8; 16], "sig wrote past SPX_WOTS_BYTES");
        eq_bytes("wots_gen_leafx1 leaf_addr", abytes(&cinfo.leaf_addr), abytes(&rinfo.leaf_addr));
        eq_bytes("wots_gen_leafx1 pk_addr", abytes(&cinfo.pk_addr), abytes(&rinfo.pk_addr));
        assert_eq!(csteps, rsteps, "wots_steps must not be modified");
    }
}

/* ---- C35 --------------------------------------------------------- */

#[test]
fn c35_wots_gen_leafx1_pkonly() {
    let l = libs();
    let (c, r) = l.pair::<WotsGenLeafX1>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(SEED + 35);
    for i in 0..NUM_ITERS {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let leaf_idx = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);
        let steps: Vec<u32> = if i % 2 == 0 {
            vec![0u32; SPX_WOTS_LEN]
        } else {
            (0..SPX_WOTS_LEN).map(|_| rng.next_u32()).collect()
        };
        let base_leaf = rng.addr();
        let base_pk = rng.addr();
        let mut csteps = steps.clone();
        let mut rsteps = steps.clone();
        // wots_sign_leaf = ~0u  =>  wots_k_mask = ~0  =>  the signature branch
        // can never fire, so wots_sig is never dereferenced (merkle_gen_root
        // relies on exactly this).
        let mut cinfo = LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: u32::MAX,
            wots_steps: csteps.as_mut_ptr(),
            leaf_addr: base_leaf,
            pk_addr: base_pk,
        };
        let mut rinfo = LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: u32::MAX,
            wots_steps: rsteps.as_mut_ptr(),
            leaf_addr: base_leaf,
            pk_addr: base_pk,
        };
        let mut cdest = vec![0u8; SPX_N];
        let mut rdest = vec![0u8; SPX_N];
        unsafe {
            c(cdest.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut cinfo);
            r(rdest.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut rinfo);
        }
        eq_bytes("wots_gen_leafx1 (pk-only) leaf", &cdest, &rdest);
        eq_bytes("(pk-only) leaf_addr", abytes(&cinfo.leaf_addr), abytes(&rinfo.leaf_addr));
        eq_bytes("(pk-only) pk_addr", abytes(&cinfo.pk_addr), abytes(&rinfo.pk_addr));
    }
}

/* ---- C36 --------------------------------------------------------- */

#[test]
fn c36_fors_gen_leafx1() {
    let l = libs();
    let (c, r) = l.pair::<ForsGenLeafX1>("SPX_fors_gen_leafx1");
    let mut rng = Rng::new(SEED + 36);
    let max = 1u32 << SPX_FORS_HEIGHT;
    let mut idxs = vec![0u32, 1, max - 1, max];
    for _ in 0..6 {
        idxs.push(rng.next_u32());
    }
    for addr_idx in idxs {
        for _ in 0..4 {
            let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
            let base = ForsGenLeafInfo { leaf_addrx: rng.addr() };
            let mut cinfo = base;
            let mut rinfo = base;
            let mut cleaf = vec![0xAAu8; SPX_N + 16];
            let mut rleaf = vec![0xAAu8; SPX_N + 16];
            unsafe {
                c(cleaf.as_mut_ptr(), cc.as_ptr(), addr_idx, &mut cinfo);
                r(rleaf.as_mut_ptr(), rc.as_ptr(), addr_idx, &mut rinfo);
            }
            eq_bytes(&format!("fors_gen_leafx1(addr_idx={addr_idx})"), &cleaf, &rleaf);
            assert_eq!(&cleaf[SPX_N..], &[0xAAu8; 16], "wrote past SPX_N");
            eq_bytes("fors_gen_leafx1 info", abytes(&cinfo.leaf_addrx), abytes(&rinfo.leaf_addrx));
        }
    }
}

/* ---- C37 --------------------------------------------------------- */

#[test]
fn c37_wots_treehashx1() {
    let l = libs();
    let (c, r) = l.pair::<WotsTreehashX1>("SPX_wots_treehashx1");
    let (cty, rty) = l.pair::<SetU32>("SPX_set_type");
    let mut rng = Rng::new(SEED + 37);
    let h = SPX_TREE_HEIGHT;
    let mut leaves = vec![0u32, 1, (1u32 << h) - 1, u32::MAX];
    if h >= 2 {
        leaves.push((1u32 << h) - 2);
    }
    for _ in 0..3 {
        leaves.push(rng.next_u32() & ((1u32 << h) - 1));
    }
    for &leaf_idx in &leaves {
        for iter in 0..4 {
            let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
            let idx_offset = if iter % 2 == 0 { 0 } else { (rng.next_u32() >> 8) << h };
            let steps: Vec<u32> = (0..SPX_WOTS_LEN)
                .map(|_| rng.below(SPX_WOTS_W as u64) as u32)
                .collect();
            let mut ta = rng.addr();
            unsafe {
                (ta.as_mut_ptr() as *mut u8).add(off::TYPE).write(2);
            }
            let leaf_base = rng.addr();
            let pk_base = rng.addr();

            let mut csteps = steps.clone();
            let mut rsteps = steps.clone();
            let mut csig = vec![0xAAu8; SPX_WOTS_BYTES];
            let mut rsig = vec![0xAAu8; SPX_WOTS_BYTES];
            let mut cta = ta;
            let mut rta = ta;
            let mut cinfo = LeafInfoX1 {
                wots_sig: csig.as_mut_ptr(),
                wots_sign_leaf: leaf_idx,
                wots_steps: csteps.as_mut_ptr(),
                leaf_addr: leaf_base,
                pk_addr: pk_base,
            };
            let mut rinfo = LeafInfoX1 {
                wots_sig: rsig.as_mut_ptr(),
                wots_sign_leaf: leaf_idx,
                wots_steps: rsteps.as_mut_ptr(),
                leaf_addr: leaf_base,
                pk_addr: pk_base,
            };
            unsafe {
                cty(cinfo.pk_addr.as_mut_ptr(), 1);
                rty(rinfo.pk_addr.as_mut_ptr(), 1);
            }
            let mut cauth = vec![0xCCu8; h as usize * SPX_N];
            let mut rauth = vec![0xCCu8; h as usize * SPX_N];
            let mut croot = vec![0u8; SPX_N];
            let mut rroot = vec![0u8; SPX_N];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    cta.as_mut_ptr(),
                    &mut cinfo,
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    rta.as_mut_ptr(),
                    &mut rinfo,
                );
            }
            let tag = format!("wots_treehashx1(leaf_idx={leaf_idx},off={idx_offset})");
            eq_bytes(&format!("{tag} root"), &croot, &rroot);
            eq_bytes(&format!("{tag} auth_path"), &cauth, &rauth);
            eq_bytes(&format!("{tag} wots_sig"), &csig, &rsig);
            eq_bytes(&format!("{tag} tree_addr"), abytes(&cta), abytes(&rta));
            eq_bytes(&format!("{tag} leaf_addr"), abytes(&cinfo.leaf_addr), abytes(&rinfo.leaf_addr));
            eq_bytes(&format!("{tag} pk_addr"), abytes(&cinfo.pk_addr), abytes(&rinfo.pk_addr));
        }
    }
}

/* ---- C38 --------------------------------------------------------- */

#[test]
fn c38_fors_treehashx1() {
    let l = libs();
    let (c, r) = l.pair::<ForsTreehashX1>("SPX_fors_treehashx1");
    let mut rng = Rng::new(SEED + 38);
    let mut heights = vec![1u32, 2, SPX_FORS_HEIGHT];
    heights.sort_unstable();
    heights.dedup();
    for &h in &heights {
        let mut leaves = vec![0u32, 1, (1u32 << h) - 1];
        if h >= 2 {
            leaves.push((1u32 << h) - 2);
        }
        for _ in 0..3 {
            leaves.push(rng.next_u32() & ((1u32 << h) - 1));
        }
        for &leaf_idx in &leaves {
            for iter in 0..4 {
                let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
                let idx_offset = match iter % 3 {
                    0 => 0,
                    1 => (rng.below(SPX_FORS_TREES as u64) as u32) * (1u32 << h),
                    _ => (rng.next_u32() >> 8) << h,
                };
                let mut ta = rng.addr();
                unsafe {
                    (ta.as_mut_ptr() as *mut u8).add(off::TYPE).write(3); // FORSTREE
                }
                let mut cta = ta;
                let mut rta = ta;
                let mut cinfo = ForsGenLeafInfo { leaf_addrx: ta };
                let mut rinfo = ForsGenLeafInfo { leaf_addrx: ta };
                let mut cauth = vec![0xCCu8; h as usize * SPX_N];
                let mut rauth = vec![0xCCu8; h as usize * SPX_N];
                let mut croot = vec![0u8; SPX_N];
                let mut rroot = vec![0u8; SPX_N];
                unsafe {
                    c(
                        croot.as_mut_ptr(),
                        cauth.as_mut_ptr(),
                        cc.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        cta.as_mut_ptr(),
                        &mut cinfo,
                    );
                    r(
                        rroot.as_mut_ptr(),
                        rauth.as_mut_ptr(),
                        rc.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        rta.as_mut_ptr(),
                        &mut rinfo,
                    );
                }
                let tag = format!("fors_treehashx1(h={h},leaf_idx={leaf_idx},off={idx_offset})");
                eq_bytes(&format!("{tag} root"), &croot, &rroot);
                eq_bytes(&format!("{tag} auth_path"), &cauth, &rauth);
                eq_bytes(&format!("{tag} tree_addr"), abytes(&cta), abytes(&rta));
                eq_bytes(&format!("{tag} info"), abytes(&cinfo.leaf_addrx), abytes(&rinfo.leaf_addrx));
            }
        }
    }
}

/* ---- C39 --------------------------------------------------------- */

#[test]
fn c39_fors_sign() {
    let l = libs();
    let (c, r) = l.pair::<ForsSign>("SPX_fors_sign");
    let (ckp, rkp) = l.pair::<SetU32>("SPX_set_keypair_addr");
    let mut rng = Rng::new(SEED + 39);
    for m in msg_cases(&mut rng, SPX_FORS_MSG_BYTES) {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let mut ca = rng.addr();
        let mut ra = ca;
        let kp = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);
        unsafe {
            ckp(ca.as_mut_ptr(), kp);
            rkp(ra.as_mut_ptr(), kp);
        }
        let mut csig = vec![0xAAu8; SPX_FORS_BYTES + 16];
        let mut rsig = vec![0xAAu8; SPX_FORS_BYTES + 16];
        let mut cpk = vec![0xBBu8; SPX_N + 16];
        let mut rpk = vec![0xBBu8; SPX_N + 16];
        unsafe {
            c(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), ca.as_ptr());
            r(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), rc.as_ptr(), ra.as_ptr());
        }
        eq_bytes("fors_sign sig", &csig, &rsig);
        eq_bytes("fors_sign pk", &cpk, &rpk);
        assert_eq!(&csig[SPX_FORS_BYTES..], &[0xAAu8; 16], "sig wrote past SPX_FORS_BYTES");
        assert_eq!(&cpk[SPX_N..], &[0xBBu8; 16], "pk wrote past SPX_N");
        eq_bytes("fors_sign must not mutate fors_addr", abytes(&ca), abytes(&ra));
    }
}

/* ---- C40 / C41 --------------------------------------------------- */

#[test]
fn c40_fors_pk_from_sig() {
    let l = libs();
    let (c, r) = l.pair::<ForsPkFromSig>("SPX_fors_pk_from_sig");
    let mut rng = Rng::new(SEED + 40);
    for m in msg_cases(&mut rng, SPX_FORS_MSG_BYTES) {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let addr = rng.addr();
        // A completely random (i.e. invalid) signature is still a valid input:
        // fors_pk_from_sig derives *some* pk from it and must not diverge.
        let sig = rng.bytes(SPX_FORS_BYTES);
        let mut cpk = vec![0xAAu8; SPX_N + 16];
        let mut rpk = vec![0xAAu8; SPX_N + 16];
        unsafe {
            c(cpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), cc.as_ptr(), addr.as_ptr());
            r(rpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), rc.as_ptr(), addr.as_ptr());
        }
        eq_bytes("fors_pk_from_sig pk (random sig)", &cpk, &rpk);
        assert_eq!(&cpk[SPX_N..], &[0xAAu8; 16], "wrote past SPX_N");
    }
}

#[test]
fn c41_fors_cross() {
    let l = libs();
    let (csign, rsign) = l.pair::<ForsSign>("SPX_fors_sign");
    let (cver, rver) = l.pair::<ForsPkFromSig>("SPX_fors_pk_from_sig");
    let mut rng = Rng::new(SEED + 41);
    for m in msg_cases(&mut rng, SPX_FORS_MSG_BYTES) {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let addr = rng.addr();
        let mut csig = vec![0u8; SPX_FORS_BYTES];
        let mut rsig = vec![0u8; SPX_FORS_BYTES];
        let mut cpk = vec![0u8; SPX_N];
        let mut rpk = vec![0u8; SPX_N];
        unsafe {
            csign(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), addr.as_ptr());
            rsign(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), rc.as_ptr(), addr.as_ptr());
        }
        eq_bytes("fors_sign cross sig", &csig, &rsig);
        // C-signed -> Rust-recovered, and Rust-signed -> C-recovered.
        let mut c_from_r = vec![0u8; SPX_N];
        let mut r_from_c = vec![0u8; SPX_N];
        unsafe {
            cver(c_from_r.as_mut_ptr(), rsig.as_ptr(), m.as_ptr(), cc.as_ptr(), addr.as_ptr());
            rver(r_from_c.as_mut_ptr(), csig.as_ptr(), m.as_ptr(), rc.as_ptr(), addr.as_ptr());
        }
        eq_bytes("C pk_from_sig(Rust sig) == signer pk", &c_from_r, &cpk);
        eq_bytes("Rust pk_from_sig(C sig) == signer pk", &r_from_c, &cpk);
    }
}

/* ---- C42 --------------------------------------------------------- */

#[test]
fn c42_merkle_sign() {
    let l = libs();
    let (c, r) = l.pair::<MerkleSign>("SPX_merkle_sign");
    let (clayer, rlayer) = l.pair::<SetU32>("SPX_set_layer_addr");
    let mut rng = Rng::new(SEED + 42);
    let h = SPX_TREE_HEIGHT;
    let siglen = SPX_WOTS_BYTES + h as usize * SPX_N;
    let mut leaves = vec![0u32, 1, (1u32 << h) - 1];
    for _ in 0..3 {
        leaves.push(rng.next_u32() & ((1u32 << h) - 1));
    }
    for &idx_leaf in &leaves {
        for &layer in &[0u32, SPX_D - 1] {
            for _ in 0..3 {
                let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
                let wbase = rng.addr();
                let tbase = rng.addr();
                let mut cw = wbase;
                let mut rw = wbase;
                let mut ct = tbase;
                let mut rt = tbase;
                unsafe {
                    clayer(cw.as_mut_ptr(), layer);
                    rlayer(rw.as_mut_ptr(), layer);
                    clayer(ct.as_mut_ptr(), layer);
                    rlayer(rt.as_mut_ptr(), layer);
                }
                // `root` is both an input (the message the WOTS key signs) and
                // an output (the subtree root).
                let root_in = rng.bytes(SPX_N);
                let mut croot = root_in.clone();
                let mut rroot = root_in.clone();
                let mut csig = vec![0xAAu8; siglen + 16];
                let mut rsig = vec![0xAAu8; siglen + 16];
                unsafe {
                    c(
                        csig.as_mut_ptr(),
                        croot.as_mut_ptr(),
                        cc.as_ptr(),
                        cw.as_mut_ptr(),
                        ct.as_mut_ptr(),
                        idx_leaf,
                    );
                    r(
                        rsig.as_mut_ptr(),
                        rroot.as_mut_ptr(),
                        rc.as_ptr(),
                        rw.as_mut_ptr(),
                        rt.as_mut_ptr(),
                        idx_leaf,
                    );
                }
                let tag = format!("merkle_sign(idx_leaf={idx_leaf},layer={layer})");
                eq_bytes(&format!("{tag} sig"), &csig, &rsig);
                eq_bytes(&format!("{tag} root"), &croot, &rroot);
                assert_eq!(&csig[siglen..], &[0xAAu8; 16], "sig wrote past bounds");
                eq_bytes(&format!("{tag} wots_addr"), abytes(&cw), abytes(&rw));
                eq_bytes(&format!("{tag} tree_addr"), abytes(&ct), abytes(&rt));
            }
        }
    }
}

/* ---- C43 --------------------------------------------------------- */

#[test]
fn c43_merkle_sign_no_authpath() {
    let l = libs();
    let (c, r) = l.pair::<MerkleSign>("SPX_merkle_sign");
    let mut rng = Rng::new(SEED + 43);
    let h = SPX_TREE_HEIGHT;
    let siglen = SPX_WOTS_BYTES + h as usize * SPX_N;
    for _ in 0..NUM_ITERS / 4 {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let wbase = rng.addr();
        let tbase = rng.addr();
        let mut cw = wbase;
        let mut rw = wbase;
        let mut ct = tbase;
        let mut rt = tbase;
        let root_in = rng.bytes(SPX_N);
        let mut croot = root_in.clone();
        let mut rroot = root_in.clone();
        let mut csig = vec![0u8; siglen];
        let mut rsig = vec![0u8; siglen];
        unsafe {
            // idx_leaf == ~0u: the mode merkle_gen_root uses (no WOTS signature
            // is emitted because no leaf index can ever match).
            c(
                csig.as_mut_ptr(),
                croot.as_mut_ptr(),
                cc.as_ptr(),
                cw.as_mut_ptr(),
                ct.as_mut_ptr(),
                u32::MAX,
            );
            r(
                rsig.as_mut_ptr(),
                rroot.as_mut_ptr(),
                rc.as_ptr(),
                rw.as_mut_ptr(),
                rt.as_mut_ptr(),
                u32::MAX,
            );
        }
        eq_bytes("merkle_sign(~0) sig", &csig, &rsig);
        eq_bytes("merkle_sign(~0) root", &croot, &rroot);
        eq_bytes("merkle_sign(~0) wots_addr", abytes(&cw), abytes(&rw));
        eq_bytes("merkle_sign(~0) tree_addr", abytes(&ct), abytes(&rt));
    }
}

/* ---- C44 --------------------------------------------------------- */

#[test]
fn c44_merkle_gen_root() {
    let l = libs();
    let (c, r) = l.pair::<MerkleGenRoot>("SPX_merkle_gen_root");
    let mut rng = Rng::new(SEED + 44);
    for i in 0..6 {
        let (pub_seed, sk_seed) = match i {
            0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
            1 => (vec![0xffu8; SPX_N], vec![0xffu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cc, rc) = init_ctx_pair(&pub_seed, &sk_seed);
        let mut croot = vec![0xAAu8; SPX_N + 16];
        let mut rroot = vec![0xAAu8; SPX_N + 16];
        unsafe {
            c(croot.as_mut_ptr(), cc.as_ptr());
            r(rroot.as_mut_ptr(), rc.as_ptr());
        }
        eq_bytes("merkle_gen_root", &croot, &rroot);
        assert_eq!(&croot[SPX_N..], &[0xAAu8; 16], "wrote past SPX_N");
    }
}
