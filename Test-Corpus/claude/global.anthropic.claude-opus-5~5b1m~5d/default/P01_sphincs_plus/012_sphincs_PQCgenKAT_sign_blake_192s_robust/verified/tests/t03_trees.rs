//! CONFIGS.md rows 15-26: `utils.c`/`utilsx1.c`/`wots*.c`/`fors.c`/`merkle.c`.

mod common;
use common::*;

type FInit = unsafe extern "C" fn(*mut u8);
type FComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
type FGenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut u32);
type FTreehash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u32,
    u32,
    u32,
    Option<FGenLeaf>,
    *mut u32,
);
type FChainLengths = unsafe extern "C" fn(*mut u32, *const u8);
type FWotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
type FWotsGenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut CLeafInfo);
type FTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut CLeafInfo);
type FForsTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut [u32; 8]);
type FForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type FForsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
type FMerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
type FMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);

/// `struct leaf_info_x1` from `app/include/wotsx1.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CLeafInfo {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

fn init_pair(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let p = libs();
    let ci = f!(p.c, "SPX_initialize_hash_function", FInit);
    let ri = f!(p.rust, "SPX_initialize_hash_function", FInit);
    let mut a = new_ctx();
    seed_ctx(&mut a, rng);
    let mut b = a.clone();
    unsafe {
        ci(a.as_mut_ptr());
        ri(b.as_mut_ptr());
    }
    (a, b)
}

/// Leaf indices the treehash routines are contractually allowed to see: any
/// index inside the tree, plus the `(uint32_t)~0` sentinel `merkle_gen_root`
/// uses to say "do not bother producing an auth path".  (A `leaf_idx` outside
/// `[0, 2^tree_height)` makes the C write past the end of `auth_path`, which is
/// undefined behaviour -- see ERRORS.md.)
fn leaf_indices(tree_height: u32) -> Vec<u32> {
    let n = 1u32 << tree_height;
    let mut v = vec![0u32, 1 % n, n / 2, n - 1, 0xFFFF_FFFF];
    v.dedup();
    v
}

fn rand_addr(rng: &mut Rng) -> [u32; 8] {
    let mut a = [0u32; 8];
    for w in a.iter_mut() {
        *w = rng.next_u32();
    }
    a
}

/// row 15 -- `SPX_compute_root`
#[test]
fn row15_compute_root() {
    let p = libs();
    let cf = f!(p.c, "SPX_compute_root", FComputeRoot);
    let rf = f!(p.rust, "SPX_compute_root", FComputeRoot);
    let mut rng = Rng::new(SEED ^ 15);

    let mut hs = vec![1u32, 2, SPX_FORS_HEIGHT as u32, SPX_TREE_HEIGHT as u32];
    hs.sort_unstable();
    hs.dedup();

    for &th in &hs {
        for case in 0..8 {
            let (ca, ra) = init_pair(&mut rng);
            let leaf = rng.bytes(SPX_N);
            let auth = rng.bytes(th as usize * SPX_N);
            let leaf_idx = match case {
                0 => 0u32,
                1 => 1,
                2 => 0xFFFF_FFFE,
                3 => 0xFFFF_FFFF,
                _ => rng.next_u32(),
            };
            let idx_offset = if case % 2 == 0 { 0 } else { rng.next_u32() };
            let addr = rand_addr(&mut rng);
            let mut a1 = addr;
            let mut a2 = addr;
            let mut r1 = vec![0x33u8; SPX_N + 8];
            let mut r2 = vec![0x33u8; SPX_N + 8];
            unsafe {
                cf(
                    r1.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    th,
                    ca.as_ptr(),
                    a1.as_mut_ptr(),
                );
                rf(
                    r2.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    th,
                    ra.as_ptr(),
                    a2.as_mut_ptr(),
                );
            }
            eq_bytes(
                &format!("compute_root(th={th},leaf_idx={leaf_idx},off={idx_offset})"),
                &r1,
                &r2,
            );
            eq(&format!("compute_root addr(th={th})"), a1, a2);
        }
    }
}

/// row 16 -- `SPX_treehash` driven with each side's own `SPX_fors_gen_leafx1`
/// as the `gen_leaf` callback.
#[test]
fn row16_treehash() {
    let p = libs();
    let cf = f!(p.c, "SPX_treehash", FTreehash);
    let rf = f!(p.rust, "SPX_treehash", FTreehash);
    let cgl = f!(p.c, "SPX_fors_gen_leafx1", FGenLeaf);
    let rgl = f!(p.rust, "SPX_fors_gen_leafx1", FGenLeaf);
    let mut rng = Rng::new(SEED ^ 16);

    let mut hs = vec![1u32, 2, 3, SPX_FORS_HEIGHT as u32];
    hs.sort_unstable();
    hs.dedup();

    for &th in &hs {
        let idxs = leaf_indices(th);
        for (k, &leaf_idx) in idxs.iter().enumerate() {
            let (ca, ra) = init_pair(&mut rng);
            let idx_offset = if k % 2 == 0 { 0 } else { rng.next_u32() & 0xFFFF };
            let addr = rand_addr(&mut rng);
            let mut a1 = addr;
            let mut a2 = addr;
            let mut r1 = vec![0x44u8; SPX_N + 8];
            let mut r2 = vec![0x44u8; SPX_N + 8];
            let mut p1 = vec![0x55u8; th as usize * SPX_N + 8];
            let mut p2 = p1.clone();
            unsafe {
                cf(
                    r1.as_mut_ptr(),
                    p1.as_mut_ptr(),
                    ca.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    th,
                    Some(cgl),
                    a1.as_mut_ptr(),
                );
                rf(
                    r2.as_mut_ptr(),
                    p2.as_mut_ptr(),
                    ra.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    th,
                    Some(rgl),
                    a2.as_mut_ptr(),
                );
            }
            eq_bytes(&format!("treehash root(th={th},li={leaf_idx})"), &r1, &r2);
            eq_bytes(&format!("treehash auth(th={th},li={leaf_idx})"), &p1, &p2);
            eq(&format!("treehash addr(th={th},li={leaf_idx})"), a1, a2);
        }
    }
}

/// row 17 -- `SPX_chain_lengths`
#[test]
fn row17_chain_lengths() {
    let p = libs();
    let cf = f!(p.c, "SPX_chain_lengths", FChainLengths);
    let rf = f!(p.rust, "SPX_chain_lengths", FChainLengths);
    let mut rng = Rng::new(SEED ^ 17);

    for iter in 0..64 {
        let msg = match iter {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            2 => vec![0x0Fu8; SPX_N],
            3 => vec![0xF0u8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let mut l1 = vec![0x9999_9999u32; SPX_WOTS_LEN + 2];
        let mut l2 = l1.clone();
        unsafe {
            cf(l1.as_mut_ptr(), msg.as_ptr());
            rf(l2.as_mut_ptr(), msg.as_ptr());
        }
        eq(&format!("chain_lengths({})", hex(&msg)), l1, l2);
    }
}

/// row 18 -- `SPX_wots_pk_from_sig`
#[test]
fn row18_wots_pk_from_sig() {
    let p = libs();
    let cf = f!(p.c, "SPX_wots_pk_from_sig", FWotsPkFromSig);
    let rf = f!(p.rust, "SPX_wots_pk_from_sig", FWotsPkFromSig);
    let mut rng = Rng::new(SEED ^ 18);

    for iter in 0..8 {
        let (ca, ra) = init_pair(&mut rng);
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let msg = match iter {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let addr = rand_addr(&mut rng);
        let mut a1 = addr;
        let mut a2 = addr;
        let mut k1 = vec![0x66u8; SPX_WOTS_BYTES + 8];
        let mut k2 = k1.clone();
        unsafe {
            cf(
                k1.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                ca.as_ptr(),
                a1.as_mut_ptr(),
            );
            rf(
                k2.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                ra.as_ptr(),
                a2.as_mut_ptr(),
            );
        }
        eq_bytes("wots_pk_from_sig", &k1, &k2);
        eq("wots_pk_from_sig addr", a1, a2);
    }
}

/// row 19 -- `SPX_wots_gen_leafx1`: signing leaf vs pk-only leaf, NULL and
/// non-NULL `wots_sig`, random and `chain_lengths`-derived `wots_steps`.
#[test]
fn row19_wots_gen_leafx1() {
    let p = libs();
    let cf = f!(p.c, "SPX_wots_gen_leafx1", FWotsGenLeaf);
    let rf = f!(p.rust, "SPX_wots_gen_leafx1", FWotsGenLeaf);
    let ccl = f!(p.c, "SPX_chain_lengths", FChainLengths);
    let mut rng = Rng::new(SEED ^ 19);

    for case in 0..12 {
        let (ca, ra) = init_pair(&mut rng);
        let leaf_idx = rng.next_u32() & 0xFFFF;
        let signing = case % 3 != 2;
        // `wots_sig == NULL` is only legal when this is NOT the signing leaf:
        // otherwise the C `memcpy(info->wots_sig + i*SPX_N, ...)` dereferences
        // NULL.  (Not an error path -- just outside the C's contract.)
        let with_sig = signing || case % 2 == 0;

        let mut steps = vec![0u32; SPX_WOTS_LEN];
        if case % 2 == 0 {
            let msg = rng.bytes(SPX_N);
            unsafe { ccl(steps.as_mut_ptr(), msg.as_ptr()) };
        } else {
            for s in steps.iter_mut() {
                *s = rng.next_u32() % (SPX_WOTS_W as u32);
            }
        }
        let mut steps1 = steps.clone();
        let mut steps2 = steps.clone();

        let mut sig1 = vec![0x77u8; SPX_WOTS_BYTES];
        let mut sig2 = sig1.clone();
        let leaf_addr = rand_addr(&mut rng);
        let pk_addr = rand_addr(&mut rng);

        let mut i1 = CLeafInfo {
            wots_sig: if with_sig {
                sig1.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
            wots_sign_leaf: if signing { leaf_idx } else { leaf_idx ^ 1 },
            wots_steps: steps1.as_mut_ptr(),
            leaf_addr,
            pk_addr,
        };
        let mut i2 = CLeafInfo {
            wots_sig: if with_sig {
                sig2.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
            wots_sign_leaf: i1.wots_sign_leaf,
            wots_steps: steps2.as_mut_ptr(),
            leaf_addr,
            pk_addr,
        };

        let mut d1 = vec![0x88u8; SPX_N + 8];
        let mut d2 = d1.clone();
        unsafe {
            cf(d1.as_mut_ptr(), ca.as_ptr(), leaf_idx, &mut i1);
            rf(d2.as_mut_ptr(), ra.as_ptr(), leaf_idx, &mut i2);
        }
        let tag = format!("wots_gen_leafx1(signing={signing},with_sig={with_sig})");
        eq_bytes(&format!("{tag} dest"), &d1, &d2);
        if with_sig {
            eq_bytes(&format!("{tag} wots_sig"), &sig1, &sig2);
        }
        eq(&format!("{tag} steps"), steps1, steps2);
        eq(&format!("{tag} leaf_addr"), i1.leaf_addr, i2.leaf_addr);
        eq(&format!("{tag} pk_addr"), i1.pk_addr, i2.pk_addr);
        eq(
            &format!("{tag} wots_sign_leaf"),
            i1.wots_sign_leaf,
            i2.wots_sign_leaf,
        );
    }
}

/// row 20 -- `SPX_wots_treehashx1`
#[test]
fn row20_wots_treehashx1() {
    let p = libs();
    let cf = f!(p.c, "SPX_wots_treehashx1", FTreehashX1);
    let rf = f!(p.rust, "SPX_wots_treehashx1", FTreehashX1);
    let ccl = f!(p.c, "SPX_chain_lengths", FChainLengths);
    let mut rng = Rng::new(SEED ^ 20);

    let th = SPX_TREE_HEIGHT as u32;
    let idxs = leaf_indices(th);

    for (k, &leaf_idx) in idxs.iter().enumerate() {
        let (ca, ra) = init_pair(&mut rng);
        let idx_offset = if k % 2 == 0 { 0 } else { rng.next_u32() & 0xFFF };

        let msg = rng.bytes(SPX_N);
        let mut steps = vec![0u32; SPX_WOTS_LEN];
        unsafe { ccl(steps.as_mut_ptr(), msg.as_ptr()) };
        let mut steps1 = steps.clone();
        let mut steps2 = steps.clone();

        let mut sig1 = vec![0x99u8; SPX_WOTS_BYTES];
        let mut sig2 = sig1.clone();
        let leaf_addr = rand_addr(&mut rng);
        let pk_addr = rand_addr(&mut rng);
        let mut i1 = CLeafInfo {
            wots_sig: sig1.as_mut_ptr(),
            wots_sign_leaf: leaf_idx,
            wots_steps: steps1.as_mut_ptr(),
            leaf_addr,
            pk_addr,
        };
        let mut i2 = CLeafInfo {
            wots_sig: sig2.as_mut_ptr(),
            wots_sign_leaf: leaf_idx,
            wots_steps: steps2.as_mut_ptr(),
            leaf_addr,
            pk_addr,
        };

        let taddr = rand_addr(&mut rng);
        let mut t1 = taddr;
        let mut t2 = taddr;
        let mut r1 = vec![0xAAu8; SPX_N + 8];
        let mut r2 = r1.clone();
        let mut ap1 = vec![0xBBu8; th as usize * SPX_N + 8];
        let mut ap2 = ap1.clone();
        unsafe {
            cf(
                r1.as_mut_ptr(),
                ap1.as_mut_ptr(),
                ca.as_ptr(),
                leaf_idx,
                idx_offset,
                th,
                t1.as_mut_ptr(),
                &mut i1,
            );
            rf(
                r2.as_mut_ptr(),
                ap2.as_mut_ptr(),
                ra.as_ptr(),
                leaf_idx,
                idx_offset,
                th,
                t2.as_mut_ptr(),
                &mut i2,
            );
        }
        let tag = format!("wots_treehashx1(li={leaf_idx},off={idx_offset})");
        eq_bytes(&format!("{tag} root"), &r1, &r2);
        eq_bytes(&format!("{tag} auth"), &ap1, &ap2);
        eq_bytes(&format!("{tag} wots_sig"), &sig1, &sig2);
        eq(&format!("{tag} tree_addr"), t1, t2);
        eq(&format!("{tag} leaf_addr"), i1.leaf_addr, i2.leaf_addr);
        eq(&format!("{tag} pk_addr"), i1.pk_addr, i2.pk_addr);
    }
}

/// row 21 -- `SPX_fors_gen_leafx1`
#[test]
fn row21_fors_gen_leafx1() {
    let p = libs();
    let cf = f!(p.c, "SPX_fors_gen_leafx1", FGenLeaf);
    let rf = f!(p.rust, "SPX_fors_gen_leafx1", FGenLeaf);
    let mut rng = Rng::new(SEED ^ 21);

    for case in 0..16 {
        let (ca, ra) = init_pair(&mut rng);
        let addr_idx = match case {
            0 => 0u32,
            1 => 1,
            2 => 0xFFFF_FFFF,
            _ => rng.next_u32(),
        };
        let info = rand_addr(&mut rng);
        let mut i1 = info;
        let mut i2 = info;
        let mut d1 = vec![0xCCu8; SPX_N + 8];
        let mut d2 = d1.clone();
        unsafe {
            cf(d1.as_mut_ptr(), ca.as_ptr(), addr_idx, i1.as_mut_ptr());
            rf(d2.as_mut_ptr(), ra.as_ptr(), addr_idx, i2.as_mut_ptr());
        }
        eq_bytes(&format!("fors_gen_leafx1({addr_idx})"), &d1, &d2);
        eq(&format!("fors_gen_leafx1 info({addr_idx})"), i1, i2);
    }
}

/// row 22 -- `SPX_fors_treehashx1`
#[test]
fn row22_fors_treehashx1() {
    let p = libs();
    let cf = f!(p.c, "SPX_fors_treehashx1", FForsTreehashX1);
    let rf = f!(p.rust, "SPX_fors_treehashx1", FForsTreehashX1);
    let mut rng = Rng::new(SEED ^ 22);

    let th = SPX_FORS_HEIGHT as u32;
    let n_leaves = 1u32 << th;
    let idxs = leaf_indices(th);

    for (k, &leaf_idx) in idxs.iter().enumerate() {
        let (ca, ra) = init_pair(&mut rng);
        let idx_offset = (k as u32) * n_leaves;
        let info = rand_addr(&mut rng);
        let mut i1 = info;
        let mut i2 = info;
        let taddr = rand_addr(&mut rng);
        let mut t1 = taddr;
        let mut t2 = taddr;
        let mut r1 = vec![0xDDu8; SPX_N + 8];
        let mut r2 = r1.clone();
        let mut ap1 = vec![0xEEu8; th as usize * SPX_N + 8];
        let mut ap2 = ap1.clone();
        unsafe {
            cf(
                r1.as_mut_ptr(),
                ap1.as_mut_ptr(),
                ca.as_ptr(),
                leaf_idx,
                idx_offset,
                th,
                t1.as_mut_ptr(),
                &mut i1,
            );
            rf(
                r2.as_mut_ptr(),
                ap2.as_mut_ptr(),
                ra.as_ptr(),
                leaf_idx,
                idx_offset,
                th,
                t2.as_mut_ptr(),
                &mut i2,
            );
        }
        let tag = format!("fors_treehashx1(li={leaf_idx},off={idx_offset})");
        eq_bytes(&format!("{tag} root"), &r1, &r2);
        eq_bytes(&format!("{tag} auth"), &ap1, &ap2);
        eq(&format!("{tag} tree_addr"), t1, t2);
        eq(&format!("{tag} info"), i1, i2);
    }
}

/// row 23 -- `SPX_fors_sign`
#[test]
fn row23_fors_sign() {
    let p = libs();
    let cf = f!(p.c, "SPX_fors_sign", FForsSign);
    let rf = f!(p.rust, "SPX_fors_sign", FForsSign);
    let mut rng = Rng::new(SEED ^ 23);

    for iter in 0..3 {
        let (ca, ra) = init_pair(&mut rng);
        let m = match iter {
            0 => vec![0u8; SPX_FORS_MSG_BYTES],
            1 => vec![0xFFu8; SPX_FORS_MSG_BYTES],
            _ => rng.bytes(SPX_FORS_MSG_BYTES),
        };
        let addr = rand_addr(&mut rng);
        let mut s1 = vec![0x12u8; SPX_FORS_BYTES + 8];
        let mut s2 = s1.clone();
        let mut k1 = vec![0x34u8; SPX_N + 8];
        let mut k2 = k1.clone();
        unsafe {
            cf(
                s1.as_mut_ptr(),
                k1.as_mut_ptr(),
                m.as_ptr(),
                ca.as_ptr(),
                addr.as_ptr(),
            );
            rf(
                s2.as_mut_ptr(),
                k2.as_mut_ptr(),
                m.as_ptr(),
                ra.as_ptr(),
                addr.as_ptr(),
            );
        }
        eq_bytes(&format!("fors_sign sig(iter={iter})"), &s1, &s2);
        eq_bytes(&format!("fors_sign pk(iter={iter})"), &k1, &k2);
    }
}

/// row 24 -- `SPX_fors_pk_from_sig`
#[test]
fn row24_fors_pk_from_sig() {
    let p = libs();
    let cf = f!(p.c, "SPX_fors_pk_from_sig", FForsPkFromSig);
    let rf = f!(p.rust, "SPX_fors_pk_from_sig", FForsPkFromSig);
    let mut rng = Rng::new(SEED ^ 24);

    for iter in 0..4 {
        let (ca, ra) = init_pair(&mut rng);
        let sig = rng.bytes(SPX_FORS_BYTES);
        let m = match iter {
            0 => vec![0u8; SPX_FORS_MSG_BYTES],
            1 => vec![0xFFu8; SPX_FORS_MSG_BYTES],
            _ => rng.bytes(SPX_FORS_MSG_BYTES),
        };
        let addr = rand_addr(&mut rng);
        let mut k1 = vec![0x56u8; SPX_N + 8];
        let mut k2 = k1.clone();
        unsafe {
            cf(
                k1.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                ca.as_ptr(),
                addr.as_ptr(),
            );
            rf(
                k2.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                ra.as_ptr(),
                addr.as_ptr(),
            );
        }
        eq_bytes(&format!("fors_pk_from_sig(iter={iter})"), &k1, &k2);
    }
}

/// row 25 -- `SPX_merkle_sign`
#[test]
fn row25_merkle_sign() {
    let p = libs();
    let cf = f!(p.c, "SPX_merkle_sign", FMerkleSign);
    let rf = f!(p.rust, "SPX_merkle_sign", FMerkleSign);
    let mut rng = Rng::new(SEED ^ 25);

    let idxs = leaf_indices(SPX_TREE_HEIGHT as u32);
    let siglen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

    for &idx_leaf in idxs.iter() {
        let (ca, ra) = init_pair(&mut rng);
        let root = rng.bytes(SPX_N);
        let wa = rand_addr(&mut rng);
        let ta = rand_addr(&mut rng);
        let mut w1 = wa;
        let mut w2 = wa;
        let mut t1 = ta;
        let mut t2 = ta;
        let mut s1 = vec![0x78u8; siglen + 8];
        let mut s2 = s1.clone();
        let mut r1 = vec![0u8; SPX_N + 8];
        r1[..SPX_N].copy_from_slice(&root);
        for x in r1[SPX_N..].iter_mut() {
            *x = 0x9A;
        }
        let mut r2 = r1.clone();
        unsafe {
            cf(
                s1.as_mut_ptr(),
                r1.as_mut_ptr(),
                ca.as_ptr(),
                w1.as_mut_ptr(),
                t1.as_mut_ptr(),
                idx_leaf,
            );
            rf(
                s2.as_mut_ptr(),
                r2.as_mut_ptr(),
                ra.as_ptr(),
                w2.as_mut_ptr(),
                t2.as_mut_ptr(),
                idx_leaf,
            );
        }
        let tag = format!("merkle_sign(idx_leaf={idx_leaf})");
        eq_bytes(&format!("{tag} sig"), &s1, &s2);
        eq_bytes(&format!("{tag} root"), &r1, &r2);
        eq(&format!("{tag} wots_addr"), w1, w2);
        eq(&format!("{tag} tree_addr"), t1, t2);
    }
}

/// row 26 -- `SPX_merkle_gen_root`
#[test]
fn row26_merkle_gen_root() {
    let p = libs();
    let cf = f!(p.c, "SPX_merkle_gen_root", FMerkleGenRoot);
    let rf = f!(p.rust, "SPX_merkle_gen_root", FMerkleGenRoot);
    let mut rng = Rng::new(SEED ^ 26);

    for _ in 0..3 {
        let (ca, ra) = init_pair(&mut rng);
        let mut r1 = vec![0xBCu8; SPX_N + 8];
        let mut r2 = r1.clone();
        unsafe {
            cf(r1.as_mut_ptr(), ca.as_ptr());
            rf(r2.as_mut_ptr(), ra.as_ptr());
        }
        eq_bytes("merkle_gen_root", &r1, &r2);
    }
}
