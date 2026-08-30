//! Phase B — differential tests for the backend-agnostic core:
//! `hash.h` (prf_addr / gen_message_random / hash_message), `thash.h`,
//! `utils.c` (compute_root / treehash), `wots.c`, `wotsx1.c`, `fors.c`,
//! `utilsx1.c` and `merkle.c`.
//!
//! Every function is driven through its own exported symbol, including the
//! lowest-level ones (`SPX_wots_gen_leafx1`, `SPX_fors_treehashx1`, ...) that
//! the convenience API only reaches indirectly.

mod common;
use common::*;
use std::ffi::c_void;

type PrfAddr = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
type GenMsgRandom =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const c_void);
type HashMessage = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u32,
    *const u8,
    *const u8,
    *const u8,
    u64,
    *const c_void,
);
type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32);
type ComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const c_void, *mut u32);
type GenLeaf = unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32);
type Treehash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const c_void,
    u32,
    u32,
    u32,
    GenLeaf,
    *mut u32,
);
type ChainLengths = unsafe extern "C" fn(*mut u32, *const u8);
type WotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *mut u32);
type WotsGenLeafx1 = unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut LeafInfoX1);
type ForsGenLeafx1 = unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut u32);
type Treehashx1 = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const c_void,
    u32,
    u32,
    u32,
    *mut u32,
    *mut c_void,
);
type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const c_void, *const u32);
type ForsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *const u32);
type MerkleSign =
    unsafe extern "C" fn(*mut u8, *mut u8, *const c_void, *mut u32, *mut u32, u32);
type MerkleGenRoot = unsafe extern "C" fn(*mut u8, *const c_void);

/// `struct leaf_info_x1` from `app/include/wotsx1.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *const u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    fn zeroed() -> LeafInfoX1 {
        LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: std::ptr::null(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
}

fn msg_lens() -> Vec<usize> {
    vec![
        0, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 47, 48, 49, 55, 56, 63, 64, 65, 71, 72, 79, 80, 87,
        88, 95, 96, 103, 104, 111, 112, 119, 120, 127, 128, 129, 135, 136, 137, 200, 255, 256, 257,
        500, 1000,
    ]
}

// ==================================================================
// hash.h
// ==================================================================

#[test]
fn prf_addr_random() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0001);
    let (c, r) = libs.pair::<PrfAddr>("SPX_prf_addr");
    for _ in 0..20 {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
        for _ in 0..50 {
            let addr = rng.addr();
            let mut co = vec![0xEEu8; N + 16];
            let mut ro = vec![0xEEu8; N + 16];
            unsafe {
                c(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
                r(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr());
            }
            assert_bytes_eq("SPX_prf_addr", &co, &ro);
        }
        // every address type, with an otherwise-zero address
        for ty in 0u32..=8 {
            let mut addr = [0u32; 8];
            let b = unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) };
            b[OFFSET_TYPE] = ty as u8;
            let mut co = vec![0xEEu8; N + 16];
            let mut ro = vec![0xEEu8; N + 16];
            unsafe {
                c(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
                r(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr());
            }
            assert_bytes_eq(&format!("SPX_prf_addr(type={})", ty), &co, &ro);
        }
    }
}

#[test]
fn gen_message_random_all_lengths() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0002);
    let (c, r) = libs.pair::<GenMsgRandom>("SPX_gen_message_random");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
    for mlen in msg_lens() {
        for _ in 0..3 {
            let sk_prf = rng.bytes(N);
            let optrand = rng.bytes(N);
            let m = rng.bytes(mlen);
            // The BLAKE backend finalises the whole 32/64-byte digest into R,
            // so the output buffer must be large enough for that.
            let mut co = vec![0xEEu8; 96];
            let mut ro = vec![0xEEu8; 96];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.as_ptr(),
                );
                r(
                    ro.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    rc.as_ptr(),
                );
            }
            assert_bytes_eq(&format!("SPX_gen_message_random(mlen={})", mlen), &co, &ro);
        }
    }
}

#[test]
fn hash_message_all_lengths() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0003);
    let (c, r) = libs.pair::<HashMessage>("SPX_hash_message");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
    for mlen in msg_lens() {
        for _ in 0..3 {
            let rr = rng.bytes(N);
            let pk = rng.bytes(PK_BYTES);
            let m = rng.bytes(mlen);
            let mut cd = vec![0xEEu8; FORS_MSG_BYTES + 16];
            let mut rd = vec![0xEEu8; FORS_MSG_BYTES + 16];
            let mut ct = 0xDEAD_BEEF_DEAD_BEEFu64;
            let mut rt = 0xDEAD_BEEF_DEAD_BEEFu64;
            let mut cl = 0xDEAD_BEEFu32;
            let mut rl = 0xDEAD_BEEFu32;
            unsafe {
                c(
                    cd.as_mut_ptr(),
                    &mut ct,
                    &mut cl,
                    rr.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.as_ptr(),
                );
                r(
                    rd.as_mut_ptr(),
                    &mut rt,
                    &mut rl,
                    rr.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    rc.as_ptr(),
                );
            }
            assert_bytes_eq(&format!("SPX_hash_message digest(mlen={})", mlen), &cd, &rd);
            assert_eq!(ct, rt, "SPX_hash_message tree (mlen={})", mlen);
            assert_eq!(cl, rl, "SPX_hash_message leaf_idx (mlen={})", mlen);
        }
    }
}

// ==================================================================
// thash.h
// ==================================================================

#[test]
fn thash_all_inblocks() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0004);
    let (c, r) = libs.pair::<Thash>("SPX_thash");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let mut blocks: Vec<u32> = vec![1, 2, 3, 4, 5, 8, 16];
    blocks.push(WOTS_LEN as u32);
    blocks.push(FORS_TREES as u32);
    blocks.sort_unstable();
    blocks.dedup();

    for &nb in &blocks {
        for _ in 0..40 {
            let inp = rng.bytes(nb as usize * N);
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut co = vec![0xEEu8; N + 16];
            let mut ro = vec![0xEEu8; N + 16];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    inp.as_ptr(),
                    nb,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    ro.as_mut_ptr(),
                    inp.as_ptr(),
                    nb,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(&format!("SPX_thash(inblocks={})", nb), &co, &ro);
            assert_eq!(ca, ra, "SPX_thash must not alter addr (inblocks={})", nb);
        }
    }
}

// ==================================================================
// utils.c: compute_root / treehash
// ==================================================================

#[test]
fn compute_root_all_heights() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0005);
    let (c, r) = libs.pair::<ComputeRoot>("SPX_compute_root");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let maxh = std::cmp::max(TREE_HEIGHT, FORS_HEIGHT) as u32;
    for h in 1..=maxh {
        for _ in 0..25 {
            let leaf = rng.bytes(N);
            let auth = rng.bytes(h as usize * N);
            // exercise both parities and offsets, incl. the maximum index
            let leaf_idx = match rng.below(4) {
                0 => 0,
                1 => (1u32 << h) - 1,
                2 => rng.next_u32() & ((1u32 << h) - 1),
                _ => rng.next_u32(),
            };
            let idx_offset = match rng.below(3) {
                0 => 0,
                1 => rng.next_u32(),
                _ => (1u32 << h) * rng.below(64),
            };
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut co = vec![0xEEu8; N + 16];
            let mut ro = vec![0xEEu8; N + 16];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    ro.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(
                &format!(
                    "SPX_compute_root(h={}, leaf_idx={}, off={})",
                    h, leaf_idx, idx_offset
                ),
                &co,
                &ro,
            );
            assert_eq!(ca, ra, "compute_root addr side effect (h={})", h);
        }
    }
}

/// Deterministic, library-independent `gen_leaf` so that `treehash` itself is
/// what is being compared (both libraries see identical leaves).
unsafe extern "C" fn test_gen_leaf(leaf: *mut u8, _ctx: *const c_void, addr_idx: u32, tree_addr: *const u32) {
    let out = std::slice::from_raw_parts_mut(leaf, N);
    let a = std::slice::from_raw_parts(tree_addr, 8);
    let mut x = 0x9E37_79B9_7F4A_7C15u64 ^ (addr_idx as u64);
    for w in a {
        x = x.wrapping_mul(0x100_0000_01B3).wrapping_add(*w as u64);
    }
    for (i, b) in out.iter_mut().enumerate() {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *b = (x >> (32 + (i % 8))) as u8;
    }
}

#[test]
fn treehash_with_shared_gen_leaf() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0006);
    let (c, r) = libs.pair::<Treehash>("SPX_treehash");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let maxh = std::cmp::min(std::cmp::max(TREE_HEIGHT, FORS_HEIGHT), 10) as u32;
    for h in 0..=maxh {
        for _ in 0..10 {
            let leaf_idx = if h == 0 { 0 } else { rng.next_u32() & ((1u32 << h) - 1) };
            let idx_offset = match rng.below(3) {
                0 => 0,
                1 => (1u32 << h) * rng.below(32),
                _ => rng.next_u32() & 0xffff,
            };
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut croot = vec![0xEEu8; N + 16];
            let mut rroot = vec![0xEEu8; N + 16];
            let mut cauth = vec![0xEEu8; (h as usize + 1) * N + 16];
            let mut rauth = vec![0xEEu8; (h as usize + 1) * N + 16];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    test_gen_leaf,
                    ca.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    test_gen_leaf,
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(&format!("SPX_treehash root(h={})", h), &croot, &rroot);
            assert_bytes_eq(&format!("SPX_treehash auth(h={})", h), &cauth, &rauth);
            assert_eq!(ca, ra, "treehash addr side effect (h={})", h);
        }
    }
}

/// `treehash` composed with each library's own `fors_gen_leafx1` — the
/// `fors_gen_leaf_info` struct is a bare `uint32_t[8]`, so it doubles as the
/// `tree_addr` argument of the generic `gen_leaf` signature.
#[test]
fn treehash_with_native_fors_gen_leaf() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0007);
    let (ct, rt) = libs.pair::<Treehash>("SPX_treehash");
    let cgl = libs.c::<GenLeaf>("SPX_fors_gen_leafx1");
    let rgl = libs.r::<GenLeaf>("SPX_fors_gen_leafx1");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let maxh = std::cmp::min(FORS_HEIGHT, 8) as u32;
    for h in 0..=maxh {
        for _ in 0..5 {
            let leaf_idx = if h == 0 { 0 } else { rng.next_u32() & ((1u32 << h) - 1) };
            let idx_offset = (1u32 << h) * rng.below(16);
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut croot = vec![0xEEu8; N + 16];
            let mut rroot = vec![0xEEu8; N + 16];
            let mut cauth = vec![0xEEu8; (h as usize + 1) * N + 16];
            let mut rauth = vec![0xEEu8; (h as usize + 1) * N + 16];
            unsafe {
                ct(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    *cgl,
                    ca.as_mut_ptr(),
                );
                rt(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    *rgl,
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(&format!("treehash+fors_gen_leafx1 root(h={})", h), &croot, &rroot);
            assert_bytes_eq(&format!("treehash+fors_gen_leafx1 auth(h={})", h), &cauth, &rauth);
            assert_eq!(ca, ra);
        }
    }
}

// ==================================================================
// wots.c / wotsx1.c
// ==================================================================

#[test]
fn chain_lengths_random() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0008);
    let (c, r) = libs.pair::<ChainLengths>("SPX_chain_lengths");
    for _ in 0..2000 {
        let msg = rng.bytes(N);
        let mut cl = vec![0xDEAD_BEEFu32; WOTS_LEN + 4];
        let mut rl = vec![0xDEAD_BEEFu32; WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), msg.as_ptr());
            r(rl.as_mut_ptr(), msg.as_ptr());
        }
        assert_eq!(cl, rl, "SPX_chain_lengths({})", hex(&msg));
    }
    for pat in [0x00u8, 0xff, 0x0f, 0xf0, 0x01, 0x80] {
        let msg = vec![pat; N];
        let mut cl = vec![0u32; WOTS_LEN + 4];
        let mut rl = vec![0u32; WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), msg.as_ptr());
            r(rl.as_mut_ptr(), msg.as_ptr());
        }
        assert_eq!(cl, rl, "SPX_chain_lengths(pat={:#x})", pat);
    }
}

#[test]
fn wots_pk_from_sig_random() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0009);
    let (c, r) = libs.pair::<WotsPkFromSig>("SPX_wots_pk_from_sig");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
    for _ in 0..20 {
        let sig = rng.bytes(WOTS_BYTES);
        let msg = rng.bytes(N);
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut cpk = vec![0xEEu8; WOTS_BYTES + 16];
        let mut rpk = vec![0xEEu8; WOTS_BYTES + 16];
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
        assert_bytes_eq("SPX_wots_pk_from_sig", &cpk, &rpk);
        assert_eq!(ca, ra, "wots_pk_from_sig addr side effect");
    }
    // all-zero and all-ff messages hit the extreme chain lengths
    for pat in [0x00u8, 0xff] {
        let sig = rng.bytes(WOTS_BYTES);
        let msg = vec![pat; N];
        let mut ca = [0u32; 8];
        let mut ra = [0u32; 8];
        let mut cpk = vec![0xEEu8; WOTS_BYTES + 16];
        let mut rpk = vec![0xEEu8; WOTS_BYTES + 16];
        unsafe {
            c(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
            r(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
        }
        assert_bytes_eq(&format!("SPX_wots_pk_from_sig(msg={:#x}..)", pat), &cpk, &rpk);
        assert_eq!(ca, ra);
    }
}

/// `wots_gen_leafx1` has two distinct code paths: `leaf_idx ==
/// info->wots_sign_leaf` (also emit the WOTS signature) and `!=` (public key
/// only).  Both are exercised.
#[test]
fn wots_gen_leafx1_both_branches() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000A);
    let (c, r) = libs.pair::<WotsGenLeafx1>("SPX_wots_gen_leafx1");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    for iter in 0..40 {
        let msg = rng.bytes(N);
        // derive plausible steps via chain_lengths, and also try raw randoms
        let mut steps = vec![0u32; WOTS_LEN];
        if iter % 2 == 0 {
            let cl = libs.c::<ChainLengths>("SPX_chain_lengths");
            unsafe { cl(steps.as_mut_ptr(), msg.as_ptr()) };
        } else {
            for s in steps.iter_mut() {
                *s = rng.below(WOTS_W as u32);
            }
        }
        let leaf_idx = rng.next_u32();
        let signing = iter % 3 != 0;
        let base_addr = rng.addr();

        let mut csig = vec![0xEEu8; WOTS_BYTES + 16];
        let mut rsig = vec![0xEEu8; WOTS_BYTES + 16];
        let mut cinfo = LeafInfoX1::zeroed();
        let mut rinfo = LeafInfoX1::zeroed();
        cinfo.wots_sig = csig.as_mut_ptr();
        rinfo.wots_sig = rsig.as_mut_ptr();
        cinfo.wots_steps = steps.as_ptr();
        rinfo.wots_steps = steps.as_ptr();
        cinfo.wots_sign_leaf = if signing { leaf_idx } else { !0u32 };
        rinfo.wots_sign_leaf = cinfo.wots_sign_leaf;
        cinfo.leaf_addr = base_addr;
        rinfo.leaf_addr = base_addr;
        cinfo.pk_addr = base_addr;
        rinfo.pk_addr = base_addr;

        let mut cdest = vec![0xEEu8; N + 16];
        let mut rdest = vec![0xEEu8; N + 16];
        unsafe {
            c(cdest.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut cinfo);
            r(rdest.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut rinfo);
        }
        assert_bytes_eq(
            &format!("SPX_wots_gen_leafx1 dest(signing={})", signing),
            &cdest,
            &rdest,
        );
        if signing {
            assert_bytes_eq("SPX_wots_gen_leafx1 wots_sig", &csig, &rsig);
        }
        assert_eq!(
            cinfo.leaf_addr, rinfo.leaf_addr,
            "leaf_addr after wots_gen_leafx1"
        );
        assert_eq!(cinfo.pk_addr, rinfo.pk_addr, "pk_addr after wots_gen_leafx1");
        assert_eq!(cinfo.wots_sign_leaf, rinfo.wots_sign_leaf);
    }
}

#[test]
fn fors_gen_leafx1_random() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000B);
    let (c, r) = libs.pair::<ForsGenLeafx1>("SPX_fors_gen_leafx1");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
    for _ in 0..300 {
        let mut ca = rng.addr();
        let mut ra = ca;
        let idx = rng.next_u32();
        let mut co = vec![0xEEu8; N + 16];
        let mut ro = vec![0xEEu8; N + 16];
        unsafe {
            c(co.as_mut_ptr(), cc.as_ptr(), idx, ca.as_mut_ptr());
            r(ro.as_mut_ptr(), rc.as_ptr(), idx, ra.as_mut_ptr());
        }
        assert_bytes_eq("SPX_fors_gen_leafx1", &co, &ro);
        assert_eq!(ca, ra, "fors_gen_leafx1 info/addr side effect");
    }
}

#[test]
fn fors_treehashx1_all_heights() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000C);
    let (c, r) = libs.pair::<Treehashx1>("SPX_fors_treehashx1");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let maxh = std::cmp::min(FORS_HEIGHT, 10) as u32;
    for h in 1..=maxh {
        for _ in 0..5 {
            let leaf_idx = rng.next_u32() & ((1u32 << h) - 1);
            let idx_offset = (1u32 << h) * rng.below(16);
            let taddr = rng.addr();
            let mut ct = taddr;
            let mut rt = taddr;
            let mut cinfo = rng.addr();
            let mut rinfo = cinfo;
            let mut croot = vec![0xEEu8; N + 16];
            let mut rroot = vec![0xEEu8; N + 16];
            let mut cauth = vec![0xEEu8; h as usize * N + 16];
            let mut rauth = vec![0xEEu8; h as usize * N + 16];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    ct.as_mut_ptr(),
                    cinfo.as_mut_ptr() as *mut c_void,
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    rt.as_mut_ptr(),
                    rinfo.as_mut_ptr() as *mut c_void,
                );
            }
            assert_bytes_eq(&format!("SPX_fors_treehashx1 root(h={})", h), &croot, &rroot);
            assert_bytes_eq(&format!("SPX_fors_treehashx1 auth(h={})", h), &cauth, &rauth);
            assert_eq!(ct, rt, "fors_treehashx1 tree_addr (h={})", h);
            assert_eq!(cinfo, rinfo, "fors_treehashx1 info (h={})", h);
        }
    }
}

#[test]
fn wots_treehashx1_all_heights() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000D);
    let (c, r) = libs.pair::<Treehashx1>("SPX_wots_treehashx1");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let maxh = std::cmp::min(TREE_HEIGHT, 6) as u32;
    for h in 1..=maxh {
        // both the "signing leaf inside the tree" and the merkle_gen_root
        // case (`wots_sign_leaf == ~0`, no auth path wanted)
        for mode in 0..2 {
            let msg = rng.bytes(N);
            let mut steps = vec![0u32; WOTS_LEN];
            {
                let cl = libs.c::<ChainLengths>("SPX_chain_lengths");
                unsafe { cl(steps.as_mut_ptr(), msg.as_ptr()) };
            }
            let leaf_idx = rng.next_u32() & ((1u32 << h) - 1);
            let idx_offset = 0;
            let taddr = rng.addr();
            let base_addr = rng.addr();

            let mut ct = taddr;
            let mut rt = taddr;
            let mut csig = vec![0xEEu8; WOTS_BYTES + 16];
            let mut rsig = vec![0xEEu8; WOTS_BYTES + 16];
            let mut cinfo = LeafInfoX1::zeroed();
            let mut rinfo = LeafInfoX1::zeroed();
            cinfo.wots_sig = csig.as_mut_ptr();
            rinfo.wots_sig = rsig.as_mut_ptr();
            cinfo.wots_steps = steps.as_ptr();
            rinfo.wots_steps = steps.as_ptr();
            cinfo.wots_sign_leaf = if mode == 0 { leaf_idx } else { !0u32 };
            rinfo.wots_sign_leaf = cinfo.wots_sign_leaf;
            cinfo.leaf_addr = base_addr;
            rinfo.leaf_addr = base_addr;
            cinfo.pk_addr = base_addr;
            rinfo.pk_addr = base_addr;

            let mut croot = vec![0xEEu8; N + 16];
            let mut rroot = vec![0xEEu8; N + 16];
            let mut cauth = vec![0xEEu8; h as usize * N + 16];
            let mut rauth = vec![0xEEu8; h as usize * N + 16];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    ct.as_mut_ptr(),
                    &mut cinfo as *mut LeafInfoX1 as *mut c_void,
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    rt.as_mut_ptr(),
                    &mut rinfo as *mut LeafInfoX1 as *mut c_void,
                );
            }
            assert_bytes_eq(
                &format!("SPX_wots_treehashx1 root(h={}, mode={})", h, mode),
                &croot,
                &rroot,
            );
            assert_bytes_eq(
                &format!("SPX_wots_treehashx1 auth(h={}, mode={})", h, mode),
                &cauth,
                &rauth,
            );
            if mode == 0 {
                assert_bytes_eq("SPX_wots_treehashx1 wots_sig", &csig, &rsig);
            }
            assert_eq!(ct, rt, "wots_treehashx1 tree_addr");
            assert_eq!(cinfo.leaf_addr, rinfo.leaf_addr);
            assert_eq!(cinfo.pk_addr, rinfo.pk_addr);
        }
    }
}

// ==================================================================
// fors.c
// ==================================================================

#[test]
fn fors_sign_and_pk_from_sig() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000E);
    let (cs, rs) = libs.pair::<ForsSign>("SPX_fors_sign");
    let (cv, rv) = libs.pair::<ForsPkFromSig>("SPX_fors_pk_from_sig");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    for i in 0..10 {
        let m = match i {
            0 => vec![0u8; FORS_MSG_BYTES],
            1 => vec![0xffu8; FORS_MSG_BYTES],
            _ => rng.bytes(FORS_MSG_BYTES),
        };
        let fors_addr = rng.addr();

        let mut csig = vec![0xEEu8; FORS_BYTES + 16];
        let mut rsig = vec![0xEEu8; FORS_BYTES + 16];
        let mut cpk = vec![0xEEu8; N + 16];
        let mut rpk = vec![0xEEu8; N + 16];
        unsafe {
            cs(
                csig.as_mut_ptr(),
                cpk.as_mut_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            rs(
                rsig.as_mut_ptr(),
                rpk.as_mut_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        assert_bytes_eq("SPX_fors_sign sig", &csig, &rsig);
        assert_bytes_eq("SPX_fors_sign pk", &cpk, &rpk);

        // pk_from_sig on the signature just produced, and on a random one
        for sig in [csig.clone(), rng.bytes(FORS_BYTES)] {
            let mut cpk2 = vec![0xEEu8; N + 16];
            let mut rpk2 = vec![0xEEu8; N + 16];
            unsafe {
                cv(
                    cpk2.as_mut_ptr(),
                    sig.as_ptr(),
                    m.as_ptr(),
                    cc.as_ptr(),
                    fors_addr.as_ptr(),
                );
                rv(
                    rpk2.as_mut_ptr(),
                    sig.as_ptr(),
                    m.as_ptr(),
                    rc.as_ptr(),
                    fors_addr.as_ptr(),
                );
            }
            assert_bytes_eq("SPX_fors_pk_from_sig", &cpk2, &rpk2);
        }
        // round-trip: the derived pk must equal the signing pk
        assert_bytes_eq("fors round-trip", &cpk[..N], {
            let mut t = vec![0xEEu8; N + 16];
            unsafe {
                cv(t.as_mut_ptr(), csig.as_ptr(), m.as_ptr(), cc.as_ptr(), fors_addr.as_ptr());
            }
            &t[..N].to_vec()
        });
    }
}

// ==================================================================
// merkle.c
// ==================================================================

#[test]
fn merkle_sign_and_gen_root() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000F);
    let (cm, rm) = libs.pair::<MerkleSign>("SPX_merkle_sign");
    let (cg, rg) = libs.pair::<MerkleGenRoot>("SPX_merkle_gen_root");
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let siglen = WOTS_BYTES + TREE_HEIGHT * N;
    for i in 0..4 {
        let root_in = rng.bytes(N);
        let wots_addr = rng.addr();
        let tree_addr = rng.addr();
        let idx_leaf = if i == 0 {
            !0u32
        } else {
            rng.next_u32() & ((1u32 << TREE_HEIGHT) - 1)
        };

        let mut cwa = wots_addr;
        let mut rwa = wots_addr;
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        let mut csig = vec![0xEEu8; siglen + 16];
        let mut rsig = vec![0xEEu8; siglen + 16];
        let mut croot = root_in.clone();
        let mut rroot = root_in.clone();
        unsafe {
            cm(
                csig.as_mut_ptr(),
                croot.as_mut_ptr(),
                cc.as_ptr(),
                cwa.as_mut_ptr(),
                cta.as_mut_ptr(),
                idx_leaf,
            );
            rm(
                rsig.as_mut_ptr(),
                rroot.as_mut_ptr(),
                rc.as_ptr(),
                rwa.as_mut_ptr(),
                rta.as_mut_ptr(),
                idx_leaf,
            );
        }
        assert_bytes_eq(&format!("SPX_merkle_sign sig(idx={})", idx_leaf), &csig, &rsig);
        assert_bytes_eq(&format!("SPX_merkle_sign root(idx={})", idx_leaf), &croot, &rroot);
        assert_eq!(cwa, rwa, "merkle_sign wots_addr");
        assert_eq!(cta, rta, "merkle_sign tree_addr");
    }

    for _ in 0..3 {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
        let mut croot = vec![0xEEu8; N + 16];
        let mut rroot = vec![0xEEu8; N + 16];
        unsafe {
            cg(croot.as_mut_ptr(), cc.as_ptr());
            rg(rroot.as_mut_ptr(), rc.as_ptr());
        }
        assert_bytes_eq("SPX_merkle_gen_root", &croot, &rroot);
    }
}
