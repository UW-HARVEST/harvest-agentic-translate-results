//! Phase B rows 29-38: the tree machinery -- `compute_root`, `treehash` (with a
//! caller-supplied C-ABI `gen_leaf` callback), `wots_gen_leafx1`,
//! `wots_pk_from_sig`, `wots_treehashx1`, `fors_gen_leafx1`, `fors_treehashx1`.
//!
//! These are the lowest-level entry points in the library and the ones the
//! convenience wrappers hide; they are driven directly here.

mod common;
use common::*;
use libloading::Symbol;

// void compute_root(u8 *root, const u8 *leaf, u32 leaf_idx, u32 idx_offset,
//                   const u8 *auth_path, u32 tree_height,
//                   const spx_ctx *ctx, u32 addr[8]);
type FComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);

// void treehash(u8 *root, u8 *auth_path, const spx_ctx *ctx,
//               u32 leaf_idx, u32 idx_offset, u32 tree_height,
//               void (*gen_leaf)(u8*, const spx_ctx*, u32, const u32[8]),
//               u32 tree_addr[8]);
type GenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
type FTreehash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u32,
    u32,
    u32,
    GenLeafFn,
    *mut u32,
);

// void wots_gen_leafx1(u8 *dest, const spx_ctx *ctx, u32 leaf_idx,
//                      leaf_info_x1 *v_info);
type FWotsGenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);

// void wots_pk_from_sig(u8 *pk, const u8 *sig, const u8 *msg,
//                       const spx_ctx *ctx, u32 addr[8]);
type FWotsPkFromSig =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);

// void wots_treehashx1 / fors_treehashx1 (root, auth_path, ctx, leaf_idx,
//                       idx_offset, tree_height, tree_addr[8], info)
type FTreehashX1 = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u32,
    u32,
    u32,
    *mut u32,
    *mut std::ffi::c_void,
);

// void fors_gen_leafx1(u8 *leaf, const spx_ctx *ctx, u32 addr_idx,
//                      fors_gen_leaf_info *info);
type FForsGenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);

/// `struct leaf_info_x1` from `app/include/wotsx1.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// `struct fors_gen_leaf_info` from `app/include/fors.h`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

// ---------------------------------------------------------------------------
// Rows 29-30 -- compute_root
// ---------------------------------------------------------------------------
#[test]
fn row29_compute_root() {
    let (c, r) = both!("SPX_compute_root", FComputeRoot);
    let mut rng = Rng::new(RNG_SEED ^ 29);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));

    let mut heights: Vec<u32> = vec![1, 2, 3];
    heights.push(SPX_FORS_HEIGHT as u32);
    heights.push(SPX_TREE_HEIGHT as u32);
    heights.sort_unstable();
    heights.dedup();

    for &h in &heights {
        let mut leaf_idxs: Vec<u32> = vec![0, 1, (1u32 << h) - 1];
        if h >= 2 {
            leaf_idxs.push((1u32 << h) - 2);
        }
        for _ in 0..N_ITER {
            leaf_idxs.push(rng.below(1u32 << h));
        }
        let offsets: Vec<u32> = vec![0, 1, 7, 8, 1u32 << h, 3 * (1u32 << h), rng.next_u32() >> 4];
        for &leaf_idx in &leaf_idxs {
            for &off in &offsets {
                let leaf = rng.bytes(SPX_N);
                let auth = rng.bytes((h as usize) * SPX_N);
                let addr = rng.addr();
                let mut croot = vec![0xA5u8; SPX_N + 32];
                let mut rroot = vec![0xA5u8; SPX_N + 32];
                let mut ca = addr;
                let mut ra = addr;
                unsafe {
                    c(
                        croot.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        off,
                        auth.as_ptr(),
                        h,
                        cctx.as_ptr(),
                        ca.as_mut_ptr(),
                    );
                    r(
                        rroot.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        off,
                        auth.as_ptr(),
                        h,
                        rctx.as_ptr(),
                        ra.as_mut_ptr(),
                    );
                }
                eq_bytes(
                    &format!("compute_root(h={h}, leaf_idx={leaf_idx}, off={off}) root"),
                    &croot,
                    &rroot,
                );
                eq(
                    &format!("compute_root(h={h}) addr side-effect"),
                    ca,
                    ra,
                );
            }
        }
    }
}

#[test]
fn row30_compute_root_out_of_range_leaf_idx() {
    // leaf_idx far beyond 2^tree_height: the C has no bounds check, it just
    // shifts the extra high bits away. Must match.
    let (c, r) = both!("SPX_compute_root", FComputeRoot);
    let mut rng = Rng::new(RNG_SEED ^ 30);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let h = SPX_TREE_HEIGHT as u32;
    for &leaf_idx in &[u32::MAX, u32::MAX - 1, 1u32 << 31, (1u32 << h) + 5] {
        for &off in &[0u32, u32::MAX, 1u32 << 31] {
            let leaf = rng.bytes(SPX_N);
            let auth = rng.bytes((h as usize) * SPX_N);
            let addr = rng.addr();
            let mut croot = vec![0xA5u8; SPX_N + 32];
            let mut rroot = vec![0xA5u8; SPX_N + 32];
            let mut ca = addr;
            let mut ra = addr;
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    off,
                    auth.as_ptr(),
                    h,
                    cctx.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    off,
                    auth.as_ptr(),
                    h,
                    rctx.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            eq_bytes(
                &format!("compute_root oob(leaf_idx={leaf_idx}, off={off})"),
                &croot,
                &rroot,
            );
            eq("compute_root oob addr", ca, ra);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 31 -- treehash with a caller-supplied gen_leaf callback
//
// The callback must be a plain `extern "C"` fn (no captures), so it derives the
// leaf deterministically from (addr_idx, tree_addr) only. Both libraries get
// the SAME callback, so the only thing under test is treehash's own control
// flow, stack discipline and auth-path bookkeeping.
// ---------------------------------------------------------------------------
unsafe extern "C" fn test_gen_leaf(
    leaf: *mut u8,
    _ctx: *const u8,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let out = std::slice::from_raw_parts_mut(leaf, SPX_N);
    let ta = std::slice::from_raw_parts(tree_addr, 8);
    // Cheap deterministic mixing of addr_idx and the whole tree_addr.
    let mut acc = 0x9E37_79B9_7F4A_7C15u64 ^ (addr_idx as u64);
    for &w in ta {
        acc = acc
            .rotate_left(7)
            .wrapping_mul(0x0100_0000_01B3)
            ^ (w as u64);
    }
    for (i, b) in out.iter_mut().enumerate() {
        acc = acc.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *b = ((acc >> 33) as u8) ^ (i as u8);
    }
}

#[test]
fn row31_treehash_with_callback() {
    let (c, r) = both!("SPX_treehash", FTreehash);
    let mut rng = Rng::new(RNG_SEED ^ 31);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for h in 0u32..=4 {
        let mut leaf_idxs: Vec<u32> = vec![0, 1u32.wrapping_sub(1)];
        if h > 0 {
            leaf_idxs.push(1);
            leaf_idxs.push((1u32 << h) - 1);
            for _ in 0..N_ITER {
                leaf_idxs.push(rng.below(1u32 << h));
            }
        }
        for &leaf_idx in &leaf_idxs {
            for &off in &[0u32, 1, 8, 3 * (1u32 << h)] {
                let addr = rng.addr();
                let apl = ((h as usize) + 1) * SPX_N;
                let mut croot = vec![0xA5u8; SPX_N + 32];
                let mut rroot = vec![0xA5u8; SPX_N + 32];
                let mut cauth = vec![0xA5u8; apl + 32];
                let mut rauth = vec![0xA5u8; apl + 32];
                let mut ca = addr;
                let mut ra = addr;
                unsafe {
                    c(
                        croot.as_mut_ptr(),
                        cauth.as_mut_ptr(),
                        cctx.as_ptr(),
                        leaf_idx,
                        off,
                        h,
                        test_gen_leaf,
                        ca.as_mut_ptr(),
                    );
                    r(
                        rroot.as_mut_ptr(),
                        rauth.as_mut_ptr(),
                        rctx.as_ptr(),
                        leaf_idx,
                        off,
                        h,
                        test_gen_leaf,
                        ra.as_mut_ptr(),
                    );
                }
                eq_bytes(
                    &format!("treehash(h={h}, leaf_idx={leaf_idx}, off={off}) root"),
                    &croot,
                    &rroot,
                );
                eq_bytes(
                    &format!("treehash(h={h}, leaf_idx={leaf_idx}, off={off}) auth_path"),
                    &cauth,
                    &rauth,
                );
                eq(&format!("treehash(h={h}) tree_addr side-effect"), ca, ra);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 32-34 -- wots_gen_leafx1, both wots_k_mask paths
// ---------------------------------------------------------------------------
fn wots_gen_leaf_case(
    signing: bool,
    steps_override: Option<u32>,
    label: &str,
    seed: u64,
) {
    let (c, r) = both!("SPX_wots_gen_leafx1", FWotsGenLeaf);
    let mut rng = Rng::new(seed);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));

    for _ in 0..N_ITER {
        let leaf_idx = rng.next_u32();
        let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| match steps_override {
                Some(v) => v,
                None => rng.below(SPX_WOTS_W as u32),
            })
            .collect();
        let addr = rng.addr();
        let pkaddr = rng.addr();

        let mut csig = vec![0xA5u8; SPX_WOTS_BYTES];
        let mut rsig = vec![0xA5u8; SPX_WOTS_BYTES];
        let mut cdest = vec![0xA5u8; SPX_N + 32];
        let mut rdest = vec![0xA5u8; SPX_N + 32];

        let mut cinfo = LeafInfoX1 {
            wots_sig: if signing {
                csig.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
            wots_sign_leaf: if signing { leaf_idx } else { !leaf_idx },
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: addr,
            pk_addr: pkaddr,
        };
        let mut rinfo = LeafInfoX1 {
            wots_sig: if signing {
                rsig.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
            wots_sign_leaf: if signing { leaf_idx } else { !leaf_idx },
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: addr,
            pk_addr: pkaddr,
        };

        unsafe {
            c(cdest.as_mut_ptr(), cctx.as_ptr(), leaf_idx, &mut cinfo);
            r(rdest.as_mut_ptr(), rctx.as_ptr(), leaf_idx, &mut rinfo);
        }
        eq_bytes(&format!("wots_gen_leafx1({label}) dest"), &cdest, &rdest);
        if signing {
            eq_bytes(&format!("wots_gen_leafx1({label}) wots_sig"), &csig, &rsig);
        }
        eq(
            &format!("wots_gen_leafx1({label}) info.leaf_addr"),
            cinfo.leaf_addr,
            rinfo.leaf_addr,
        );
        eq(
            &format!("wots_gen_leafx1({label}) info.pk_addr"),
            cinfo.pk_addr,
            rinfo.pk_addr,
        );
        eq(
            &format!("wots_gen_leafx1({label}) info.wots_sign_leaf"),
            cinfo.wots_sign_leaf,
            rinfo.wots_sign_leaf,
        );
    }
}

#[test]
fn row32_wots_gen_leafx1_signing_path() {
    wots_gen_leaf_case(true, None, "signing", RNG_SEED ^ 32);
}

#[test]
fn row33_wots_gen_leafx1_pk_only_path() {
    wots_gen_leaf_case(false, None, "pk-only", RNG_SEED ^ 33);
}

#[test]
fn row34_wots_gen_leafx1_step_extremes() {
    wots_gen_leaf_case(true, Some(0), "steps=0", RNG_SEED ^ 340);
    wots_gen_leaf_case(
        true,
        Some((SPX_WOTS_W - 1) as u32),
        "steps=w-1",
        RNG_SEED ^ 341,
    );
    wots_gen_leaf_case(false, Some(0), "pk-only steps=0", RNG_SEED ^ 342);
}

// ---------------------------------------------------------------------------
// Row 35 -- wots_pk_from_sig
// ---------------------------------------------------------------------------
#[test]
fn row35_wots_pk_from_sig() {
    let (c, r) = both!("SPX_wots_pk_from_sig", FWotsPkFromSig);
    let mut rng = Rng::new(RNG_SEED ^ 35);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let msgs: Vec<Vec<u8>> = {
        let mut v = vec![vec![0x00u8; SPX_N], vec![0xFFu8; SPX_N]];
        for _ in 0..N_ITER {
            v.push(rng.bytes(SPX_N));
        }
        v
    };
    for msg in &msgs {
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let addr = rng.addr();
        let mut cpk = vec![0xA5u8; SPX_WOTS_BYTES + 32];
        let mut rpk = vec![0xA5u8; SPX_WOTS_BYTES + 32];
        let mut ca = addr;
        let mut ra = addr;
        unsafe {
            c(
                cpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cctx.as_ptr(),
                ca.as_mut_ptr(),
            );
            r(
                rpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                rctx.as_ptr(),
                ra.as_mut_ptr(),
            );
        }
        eq_bytes(&format!("wots_pk_from_sig(msg={}) pk", hex(msg)), &cpk, &rpk);
        eq("wots_pk_from_sig addr side-effect", ca, ra);
    }
}

// ---------------------------------------------------------------------------
// Row 36 -- wots_treehashx1
// ---------------------------------------------------------------------------
#[test]
fn row36_wots_treehashx1() {
    let (c, r) = both!("SPX_wots_treehashx1", FTreehashX1);
    let mut rng = Rng::new(RNG_SEED ^ 36);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let h = SPX_TREE_HEIGHT as u32;
    let mut leaf_idxs: Vec<u32> = vec![0, 1, (1u32 << h) - 1, u32::MAX];
    for _ in 0..N_ITER_SLOW {
        leaf_idxs.push(rng.below(1u32 << h));
    }
    for &leaf_idx in &leaf_idxs {
        let mut steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| rng.below(SPX_WOTS_W as u32))
            .collect();
        let addr = rng.addr();
        let pkaddr = rng.addr();
        let taddr = rng.addr();

        let mut csig = vec![0xA5u8; SPX_WOTS_BYTES];
        let mut rsig = vec![0xA5u8; SPX_WOTS_BYTES];
        let mut cinfo = LeafInfoX1 {
            wots_sig: csig.as_mut_ptr(),
            wots_sign_leaf: leaf_idx,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: addr,
            pk_addr: pkaddr,
        };
        let mut rinfo = LeafInfoX1 {
            wots_sig: rsig.as_mut_ptr(),
            wots_sign_leaf: leaf_idx,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: addr,
            pk_addr: pkaddr,
        };
        let apl = (h as usize) * SPX_N;
        let mut croot = vec![0xA5u8; SPX_N + 32];
        let mut rroot = vec![0xA5u8; SPX_N + 32];
        let mut cauth = vec![0xA5u8; apl + 32];
        let mut rauth = vec![0xA5u8; apl + 32];
        let mut cta = taddr;
        let mut rta = taddr;
        unsafe {
            c(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                cctx.as_ptr(),
                leaf_idx,
                0,
                h,
                cta.as_mut_ptr(),
                (&mut cinfo as *mut LeafInfoX1).cast(),
            );
            r(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                rctx.as_ptr(),
                leaf_idx,
                0,
                h,
                rta.as_mut_ptr(),
                (&mut rinfo as *mut LeafInfoX1).cast(),
            );
        }
        eq_bytes(
            &format!("wots_treehashx1(leaf_idx={leaf_idx}) root"),
            &croot,
            &rroot,
        );
        eq_bytes(
            &format!("wots_treehashx1(leaf_idx={leaf_idx}) auth_path"),
            &cauth,
            &rauth,
        );
        eq_bytes(
            &format!("wots_treehashx1(leaf_idx={leaf_idx}) wots_sig"),
            &csig,
            &rsig,
        );
        eq("wots_treehashx1 tree_addr side-effect", cta, rta);
        eq(
            "wots_treehashx1 info.leaf_addr",
            cinfo.leaf_addr,
            rinfo.leaf_addr,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 37 -- fors_gen_leafx1
// ---------------------------------------------------------------------------
#[test]
fn row37_fors_gen_leafx1() {
    let (c, r) = both!("SPX_fors_gen_leafx1", FForsGenLeaf);
    let mut rng = Rng::new(RNG_SEED ^ 37);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let mut idxs: Vec<u32> = vec![0, 1, (1u32 << SPX_FORS_HEIGHT) - 1, u32::MAX];
    for _ in 0..N_ITER {
        idxs.push(rng.next_u32());
    }
    for &addr_idx in &idxs {
        for base in [[0u32; 8], [u32::MAX; 8], rng.addr()] {
            let mut cinfo = ForsGenLeafInfo { leaf_addrx: base };
            let mut rinfo = ForsGenLeafInfo { leaf_addrx: base };
            let mut cl = vec![0xA5u8; SPX_N + 32];
            let mut rl = vec![0xA5u8; SPX_N + 32];
            unsafe {
                c(cl.as_mut_ptr(), cctx.as_ptr(), addr_idx, &mut cinfo);
                r(rl.as_mut_ptr(), rctx.as_ptr(), addr_idx, &mut rinfo);
            }
            eq_bytes(
                &format!("fors_gen_leafx1(addr_idx={addr_idx}) leaf"),
                &cl,
                &rl,
            );
            eq(
                &format!("fors_gen_leafx1(addr_idx={addr_idx}) info side-effect"),
                cinfo.leaf_addrx,
                rinfo.leaf_addrx,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 38 -- fors_treehashx1
//
// NOTE: `fors_treehashx1`'s prototype says `leaf_info_x1*` but the only caller
// (`fors_sign`) passes a `fors_gen_leaf_info*` (32 bytes) and
// `fors_gen_leafx1` reads it as such. The test replicates the C exactly by
// passing a 32-byte `ForsGenLeafInfo`.
// ---------------------------------------------------------------------------
#[test]
fn row38_fors_treehashx1() {
    let (c, r) = both!("SPX_fors_treehashx1", FTreehashX1);
    let mut rng = Rng::new(RNG_SEED ^ 38);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let h = SPX_FORS_HEIGHT as u32;
    let mut leaf_idxs: Vec<u32> = vec![0, 1, (1u32 << h) - 1];
    for _ in 0..N_ITER_SLOW {
        leaf_idxs.push(rng.below(1u32 << h));
    }
    for &leaf_idx in &leaf_idxs {
        for &off in &[0u32, 1u32 << h, 3 * (1u32 << h)] {
            let taddr = rng.addr();
            let mut cinfo = ForsGenLeafInfo {
                leaf_addrx: rng.addr(),
            };
            let mut rinfo = ForsGenLeafInfo {
                leaf_addrx: cinfo.leaf_addrx,
            };
            let apl = (h as usize) * SPX_N;
            let mut croot = vec![0xA5u8; SPX_N + 32];
            let mut rroot = vec![0xA5u8; SPX_N + 32];
            let mut cauth = vec![0xA5u8; apl + 32];
            let mut rauth = vec![0xA5u8; apl + 32];
            let mut cta = taddr;
            let mut rta = taddr;
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cctx.as_ptr(),
                    leaf_idx,
                    off,
                    h,
                    cta.as_mut_ptr(),
                    (&mut cinfo as *mut ForsGenLeafInfo).cast(),
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rctx.as_ptr(),
                    leaf_idx,
                    off,
                    h,
                    rta.as_mut_ptr(),
                    (&mut rinfo as *mut ForsGenLeafInfo).cast(),
                );
            }
            eq_bytes(
                &format!("fors_treehashx1(leaf_idx={leaf_idx}, off={off}) root"),
                &croot,
                &rroot,
            );
            eq_bytes(
                &format!("fors_treehashx1(leaf_idx={leaf_idx}, off={off}) auth_path"),
                &cauth,
                &rauth,
            );
            eq("fors_treehashx1 tree_addr side-effect", cta, rta);
            eq(
                "fors_treehashx1 info side-effect",
                cinfo.leaf_addrx,
                rinfo.leaf_addrx,
            );
        }
    }
}
