//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every test drives BOTH the C `.so` and the
//! Rust `.so` through `dlsym`'d entry points with the same randomized inputs
//! (fixed seed) and asserts byte-identical outputs, including every buffer the
//! callee is allowed to mutate in place (addresses, contexts, info structs).

mod common;

use common::*;
use libloading::os::unix::Symbol;

// ===========================================================================
// Row 1-3 — utils.c integer/byte conversions
// ===========================================================================

#[test]
fn cfg01_ull_to_bytes() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, core::ffi::c_uint, u64);
    let fc: Symbol<F> = p.c.sym("SPX_ull_to_bytes");
    let fr: Symbol<F> = p.r.sym("SPX_ull_to_bytes");

    let mut rng = Rng::new(1);
    let mut values: Vec<u64> = vec![0, 1, 2, 0xff, 0x100, u64::MAX, u64::MAX - 1, 1 << 63];
    for _ in 0..64 {
        values.push(rng.next_u64());
    }

    for &outlen in &[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 32] {
        for &v in &values {
            // Pre-fill with a non-zero pattern so "wrote nothing" is observable.
            let mut cb = vec![0xAAu8; 64];
            let mut rb = vec![0xAAu8; 64];
            unsafe { fc(cb.as_mut_ptr(), outlen, v) };
            unsafe { fr(rb.as_mut_ptr(), outlen, v) };
            eq_bytes(&format!("ull_to_bytes(outlen={outlen}, in={v:#x})"), &cb, &rb);
        }
    }
}

#[test]
fn cfg02_u32_to_bytes() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, u32);
    let fc: Symbol<F> = p.c.sym("SPX_u32_to_bytes");
    let fr: Symbol<F> = p.r.sym("SPX_u32_to_bytes");

    let mut rng = Rng::new(2);
    let mut values: Vec<u32> = vec![0, 1, 0x7fffffff, 0x80000000, 0xffffffff];
    for _ in 0..256 {
        values.push(rng.next_u32());
    }
    for &v in &values {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe { fc(cb.as_mut_ptr(), v) };
        unsafe { fr(rb.as_mut_ptr(), v) };
        eq_bytes(&format!("u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

#[test]
fn cfg03_bytes_to_ull() {
    let p = load();
    type F = unsafe extern "C" fn(*const u8, core::ffi::c_uint) -> u64;
    let fc: Symbol<F> = p.c.sym("SPX_bytes_to_ull");
    let fr: Symbol<F> = p.r.sym("SPX_bytes_to_ull");

    let mut rng = Rng::new(3);
    let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16]];
    for _ in 0..64 {
        inputs.push(rng.bytes(16));
    }
    for &inlen in &[0u32, 1, 2, 3, 4, 5, 6, 7, 8] {
        for inp in &inputs {
            let c = unsafe { fc(inp.as_ptr(), inlen) };
            let r = unsafe { fr(inp.as_ptr(), inlen) };
            eq(&format!("bytes_to_ull(inlen={inlen})"), c, r);
        }
    }
}

// ===========================================================================
// Row 4 — address.c: all ten setters/copiers, from a RANDOM starting address
// ===========================================================================

#[test]
fn cfg04_address_setters() {
    let p = load();
    type F32 = unsafe extern "C" fn(*mut u32, u32);
    type F64 = unsafe extern "C" fn(*mut u32, u64);
    type FCP = unsafe extern "C" fn(*mut u32, *const u32);

    let one_arg_u32 = [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_keypair_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
        "SPX_set_tree_index",
    ];

    let mut rng = Rng::new(4);
    for _ in 0..256 {
        let base = rng.addr();
        let v = rng.next_u32();
        for name in one_arg_u32 {
            let fc: Symbol<F32> = p.c.sym(name);
            let fr: Symbol<F32> = p.r.sym(name);
            let mut ca = base;
            let mut ra = base;
            unsafe { fc(ca.as_mut_ptr(), v) };
            unsafe { fr(ra.as_mut_ptr(), v) };
            eq_u32s(&format!("{name}({v:#x})"), &ca, &ra);
        }

        // set_tree_addr takes a uint64_t
        let t = rng.next_u64();
        let fc: Symbol<F64> = p.c.sym("SPX_set_tree_addr");
        let fr: Symbol<F64> = p.r.sym("SPX_set_tree_addr");
        let mut ca = base;
        let mut ra = base;
        unsafe { fc(ca.as_mut_ptr(), t) };
        unsafe { fr(ra.as_mut_ptr(), t) };
        eq_u32s(&format!("SPX_set_tree_addr({t:#x})"), &ca, &ra);

        // the two copiers: `out` is also random, so untouched bytes matter
        let src = rng.addr();
        for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
            let fc: Symbol<FCP> = p.c.sym(name);
            let fr: Symbol<FCP> = p.r.sym(name);
            let mut ca = base;
            let mut ra = base;
            unsafe { fc(ca.as_mut_ptr(), src.as_ptr()) };
            unsafe { fr(ra.as_mut_ptr(), src.as_ptr()) };
            eq_u32s(name, &ca, &ra);
        }
    }
}

// ===========================================================================
// Row 5-6 — hash_<backend>.c: initialize_hash_function / prf_addr
// ===========================================================================

#[test]
fn cfg05_initialize_hash_function() {
    let p = load();
    let mut rng = Rng::new(5);
    // Extreme seeds first, then random ones.
    let fixed: [(Vec<u8>, Vec<u8>); 3] = [
        (vec![0u8; SPX_N], vec![0u8; SPX_N]),
        (vec![0xffu8; SPX_N], vec![0xffu8; SPX_N]),
        (vec![0x5au8; SPX_N], vec![0xa5u8; SPX_N]),
    ];
    for (ps, ss) in &fixed {
        init_ctx_pair_from(&p, ps, ss);
    }
    for _ in 0..32 {
        init_ctx_pair(&p, &mut rng);
    }
    assert_eq!(
        CTX_BYTES,
        core::mem::size_of::<sphincs_core_det::context::SpxCtx>(),
        "Rust SpxCtx size does not match the C spx_ctx size for {}",
        combo()
    );
}

#[test]
fn cfg06_prf_addr() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
    let fc: Symbol<F> = p.c.sym("SPX_prf_addr");
    let fr: Symbol<F> = p.r.sym("SPX_prf_addr");

    let mut rng = Rng::new(6);
    for i in 0..64 {
        let (cc, rc) = init_ctx_pair(&p, &mut rng);
        let addr = match i {
            0 => [0u32; 8],
            1 => [0xffffffffu32; 8],
            _ => rng.addr(),
        };
        let mut co = vec![0xAAu8; SPX_N + 16];
        let mut ro = vec![0xAAu8; SPX_N + 16];
        unsafe { fc(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr()) };
        unsafe { fr(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr()) };
        eq_bytes("prf_addr out", &co[..SPX_N], &ro[..SPX_N]);
        eq_bytes("prf_addr must not write past SPX_N", &co, &ro);
        eq_bytes("prf_addr must not modify ctx", cc.bytes(), rc.bytes());
    }
}

// ===========================================================================
// Row 7 — thash: the inblocks axis (incl. the 256-vs-512 primitive switch)
// ===========================================================================

#[test]
fn cfg07_thash() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_uint, *const u8, *mut u32);
    let fc: Symbol<F> = p.c.sym("SPX_thash");
    let fr: Symbol<F> = p.r.sym("SPX_thash");

    let mut rng = Rng::new(7);
    // The in-tree call sites only ever use 1, 2, SPX_WOTS_LEN and
    // SPX_FORS_TREES, but `thash` is a public entry point and the C sizes its
    // scratch with `SPX_VLA` (a VLA), so it accepts any `inblocks`.  Values
    // past `max(SPX_WOTS_LEN, SPX_FORS_TREES)` are included so that a Rust
    // fixed-size stack buffer without a heap fallback would be caught.
    let max_inblocks = SPX_WOTS_LEN.max(SPX_FORS_TREES) as u32;
    let mut blocks: Vec<u32> = vec![
        0,
        1,
        2,
        3,
        SPX_WOTS_LEN as u32,
        SPX_FORS_TREES as u32,
        max_inblocks + 1,
        max_inblocks + 8,
        2 * max_inblocks,
        200,
    ];
    blocks.sort_unstable();
    blocks.dedup();

    for &inblocks in &blocks {
        for it in 0..16 {
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let n = (inblocks as usize) * SPX_N;
            let input = match it {
                0 => vec![0u8; n],
                1 => vec![0xffu8; n],
                _ => rng.bytes(n),
            };
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut co = vec![0xAAu8; SPX_N + 16];
            let mut ro = vec![0xAAu8; SPX_N + 16];
            unsafe {
                fc(
                    co.as_mut_ptr(),
                    input.as_ptr(),
                    inblocks,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                )
            };
            unsafe {
                fr(
                    ro.as_mut_ptr(),
                    input.as_ptr(),
                    inblocks,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                )
            };
            eq_bytes(&format!("thash(inblocks={inblocks}) out"), &co, &ro);
            eq_u32s(&format!("thash(inblocks={inblocks}) addr"), &ca, &ra);
            eq_bytes("thash must not modify ctx", cc.bytes(), rc.bytes());
        }
    }
}

// ===========================================================================
// Row 8-9 — gen_message_random / hash_message: the mlen axis
// ===========================================================================

fn mlen_axis() -> Vec<usize> {
    let mut v = vec![
        0usize, 1, 2, 31, 32, 33, 55, 56, 57, 63, 64, 65, 71, 72, 111, 112, 113, 127, 128, 129,
        135, 136, 137, 239, 240, 255, 256, 257, 271, 272, 1000,
    ];
    v.push(SPX_DGST_BYTES.saturating_sub(1));
    v.push(SPX_DGST_BYTES);
    v.push(SPX_DGST_BYTES + 1);
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn cfg08_gen_message_random() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
    let fc: Symbol<F> = p.c.sym("SPX_gen_message_random");
    let fr: Symbol<F> = p.r.sym("SPX_gen_message_random");

    let mut rng = Rng::new(8);
    let (cc, rc) = init_ctx_pair(&p, &mut rng);
    for mlen in mlen_axis() {
        for _ in 0..3 {
            let sk_prf = rng.bytes(SPX_N);
            let optrand = rng.bytes(SPX_N);
            let m = rng.bytes(mlen);
            // NOTE: for the blake backend with SPX_N >= 24, `hash_blake.c`'s
            // `gen_message_random` ends with `blake512_final(&S, R)`, which
            // writes the FULL 64-byte digest through the caller's `R` pointer
            // even though `R` is only SPX_N bytes in every in-tree caller.  The
            // output buffer therefore has to be at least 64 bytes here, and the
            // whole thing is compared so that the over-write is verified too.
            let mut co = vec![0xAAu8; SPX_N + 128];
            let mut ro = vec![0xAAu8; SPX_N + 128];
            unsafe {
                fc(
                    co.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.as_ptr(),
                )
            };
            unsafe {
                fr(
                    ro.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    rc.as_ptr(),
                )
            };
            eq_bytes(&format!("gen_message_random(mlen={mlen})"), &co, &ro);
        }
    }
}

#[test]
fn cfg09_hash_message() {
    let p = load();
    type F = unsafe extern "C" fn(
        *mut u8,
        *mut u64,
        *mut u32,
        *const u8,
        *const u8,
        *const u8,
        u64,
        *const u8,
    );
    let fc: Symbol<F> = p.c.sym("SPX_hash_message");
    let fr: Symbol<F> = p.r.sym("SPX_hash_message");

    let mut rng = Rng::new(9);
    let (cc, rc) = init_ctx_pair(&p, &mut rng);
    for mlen in mlen_axis() {
        for _ in 0..3 {
            let r_rand = rng.bytes(SPX_N);
            let pk = rng.bytes(SPX_PK_BYTES);
            let m = rng.bytes(mlen);

            let mut cd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 16];
            let mut rd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 16];
            let mut ct: u64 = 0xDEADBEEFDEADBEEF;
            let mut rt: u64 = 0xDEADBEEFDEADBEEF;
            let mut cl: u32 = 0xDEADBEEF;
            let mut rl: u32 = 0xDEADBEEF;
            unsafe {
                fc(
                    cd.as_mut_ptr(),
                    &mut ct,
                    &mut cl,
                    r_rand.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.as_ptr(),
                )
            };
            unsafe {
                fr(
                    rd.as_mut_ptr(),
                    &mut rt,
                    &mut rl,
                    r_rand.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    rc.as_ptr(),
                )
            };
            eq_bytes(&format!("hash_message(mlen={mlen}) digest"), &cd, &rd);
            eq(&format!("hash_message(mlen={mlen}) tree"), ct, rt);
            eq(&format!("hash_message(mlen={mlen}) leaf_idx"), cl, rl);
        }
    }
}

// ===========================================================================
// Row 10-11 — wots.c
// ===========================================================================

#[test]
fn cfg10_chain_lengths() {
    let p = load();
    type F = unsafe extern "C" fn(*mut core::ffi::c_uint, *const u8);
    let fc: Symbol<F> = p.c.sym("SPX_chain_lengths");
    let fr: Symbol<F> = p.r.sym("SPX_chain_lengths");

    let mut rng = Rng::new(10);
    let mut msgs: Vec<Vec<u8>> = vec![vec![0u8; SPX_N], vec![0xffu8; SPX_N], vec![0x0fu8; SPX_N]];
    for _ in 0..64 {
        msgs.push(rng.bytes(SPX_N));
    }
    for m in &msgs {
        let mut cl = vec![0xEEEEEEEEu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xEEEEEEEEu32; SPX_WOTS_LEN + 4];
        unsafe { fc(cl.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr()) };
        unsafe { fr(rl.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr()) };
        eq_u32s("chain_lengths", &cl, &rl);
        // every chain length must be a valid base-w digit
        for (i, v) in cl[..SPX_WOTS_LEN].iter().enumerate() {
            assert!(
                (*v as usize) < SPX_WOTS_W,
                "chain_lengths[{i}] = {v} >= SPX_WOTS_W"
            );
        }
    }
}

#[test]
fn cfg11_wots_pk_from_sig() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
    let fc: Symbol<F> = p.c.sym("SPX_wots_pk_from_sig");
    let fr: Symbol<F> = p.r.sym("SPX_wots_pk_from_sig");

    let mut rng = Rng::new(11);
    for it in 0..8 {
        let (cc, rc) = init_ctx_pair(&p, &mut rng);
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let msg = match it {
            0 => vec![0u8; SPX_N],
            1 => vec![0xffu8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut cpk = vec![0xAAu8; SPX_WOTS_BYTES + 16];
        let mut rpk = vec![0xAAu8; SPX_WOTS_BYTES + 16];
        unsafe {
            fc(
                cpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cc.as_ptr(),
                ca.as_mut_ptr(),
            )
        };
        unsafe {
            fr(
                rpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                rc.as_ptr(),
                ra.as_mut_ptr(),
            )
        };
        eq_bytes("wots_pk_from_sig pk", &cpk, &rpk);
        eq_u32s("wots_pk_from_sig addr", &ca, &ra);
    }
}

// ===========================================================================
// Row 12 — wotsx1.c: both branches of `leaf_idx == info->wots_sign_leaf`
// ===========================================================================

#[test]
fn cfg12_wots_gen_leafx1() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);
    let fc: Symbol<F> = p.c.sym("SPX_wots_gen_leafx1");
    let fr: Symbol<F> = p.r.sym("SPX_wots_gen_leafx1");

    let mut rng = Rng::new(12);
    // step patterns, including out-of-range values (> SPX_WOTS_W-1)
    let step_patterns: Vec<Box<dyn Fn(&mut Rng, usize) -> Vec<u32>>> = vec![
        Box::new(|_r, n| vec![0u32; n]),
        Box::new(|_r, n| vec![(SPX_WOTS_W - 1) as u32; n]),
        Box::new(|_r, n| vec![SPX_WOTS_W as u32; n]),
        Box::new(|_r, n| vec![0xffffffffu32; n]),
        Box::new(|r, n| (0..n).map(|_| r.below(SPX_WOTS_W as u32)).collect()),
    ];

    for signing in [true, false] {
        for pat in &step_patterns {
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let leaf_idx = rng.next_u32() & 0xffff;
            let base = rng.addr();
            let mut steps_c = pat(&mut rng, SPX_WOTS_LEN);
            let mut steps_r = steps_c.clone();

            let mut sig_c = vec![0xAAu8; SPX_WOTS_BYTES];
            let mut sig_r = vec![0xAAu8; SPX_WOTS_BYTES];

            let mut info_c = LeafInfoX1::zeroed();
            info_c.leaf_addr = base;
            info_c.pk_addr = base;
            info_c.wots_steps = steps_c.as_mut_ptr();
            let mut info_r = info_c;
            info_r.wots_steps = steps_r.as_mut_ptr();

            if signing {
                info_c.wots_sign_leaf = leaf_idx;
                info_r.wots_sign_leaf = leaf_idx;
                info_c.wots_sig = sig_c.as_mut_ptr();
                info_r.wots_sig = sig_r.as_mut_ptr();
            } else {
                // exactly what merkle_gen_root does: never matches any leaf
                info_c.wots_sign_leaf = u32::MAX;
                info_r.wots_sign_leaf = u32::MAX;
                info_c.wots_sig = core::ptr::null_mut();
                info_r.wots_sig = core::ptr::null_mut();
            }

            let mut cd = vec![0xAAu8; SPX_N + 16];
            let mut rd = vec![0xAAu8; SPX_N + 16];
            unsafe { fc(cd.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut info_c) };
            unsafe { fr(rd.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut info_r) };

            let tag = format!("wots_gen_leafx1(signing={signing})");
            eq_bytes(&format!("{tag} dest"), &cd, &rd);
            eq_u32s(&format!("{tag} leaf_addr"), &info_c.leaf_addr, &info_r.leaf_addr);
            eq_u32s(&format!("{tag} pk_addr"), &info_c.pk_addr, &info_r.pk_addr);
            eq_u32s(&format!("{tag} wots_steps"), &steps_c, &steps_r);
            if signing {
                eq_bytes(&format!("{tag} wots_sig"), &sig_c, &sig_r);
            }
            eq(&format!("{tag} wots_sign_leaf"), info_c.wots_sign_leaf, info_r.wots_sign_leaf);
        }
    }
}

// ===========================================================================
// Row 13 — fors.c: fors_gen_leafx1
// ===========================================================================

#[test]
fn cfg13_fors_gen_leafx1() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);
    let fc: Symbol<F> = p.c.sym("SPX_fors_gen_leafx1");
    let fr: Symbol<F> = p.r.sym("SPX_fors_gen_leafx1");

    let mut rng = Rng::new(13);
    let max_leaf = 1u32 << SPX_FORS_HEIGHT;
    let idxs = [0u32, 1, max_leaf - 1, max_leaf, u32::MAX];
    for &addr_idx in &idxs {
        for _ in 0..4 {
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let base = rng.addr();
            let mut ic = ForsGenLeafInfo { leaf_addrx: base };
            let mut ir = ForsGenLeafInfo { leaf_addrx: base };
            let mut cl = vec![0xAAu8; SPX_N + 16];
            let mut rl = vec![0xAAu8; SPX_N + 16];
            unsafe { fc(cl.as_mut_ptr(), cc.as_ptr(), addr_idx, &mut ic) };
            unsafe { fr(rl.as_mut_ptr(), rc.as_ptr(), addr_idx, &mut ir) };
            eq_bytes(&format!("fors_gen_leafx1({addr_idx}) leaf"), &cl, &rl);
            eq_u32s("fors_gen_leafx1 leaf_addrx", &ic.leaf_addrx, &ir.leaf_addrx);
        }
    }
}

// ===========================================================================
// Row 14 — utils.c compute_root
// ===========================================================================

#[test]
fn cfg14_compute_root() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
    let fc: Symbol<F> = p.c.sym("SPX_compute_root");
    let fr: Symbol<F> = p.r.sym("SPX_compute_root");

    let mut rng = Rng::new(14);
    let mut heights: Vec<u32> = vec![1, 2, 3, SPX_FORS_HEIGHT as u32, SPX_TREE_HEIGHT as u32];
    heights.sort_unstable();
    heights.dedup();

    for &h in &heights {
        let span = if h >= 32 { u32::MAX } else { (1u32 << h) - 1 };
        let mut leaf_idxs: Vec<u32> = vec![0, 1, 2, 3, span];
        for _ in 0..4 {
            leaf_idxs.push(rng.next_u32() & span);
        }
        for &leaf_idx in &leaf_idxs {
            for &idx_offset in &[0u32, 1, rng.next_u32()] {
                let (cc, rc) = init_ctx_pair(&p, &mut rng);
                let leaf = rng.bytes(SPX_N);
                let auth = rng.bytes(h as usize * SPX_N);
                let addr = rng.addr();
                let mut ca = addr;
                let mut ra = addr;
                let mut cr = vec![0xAAu8; SPX_N + 16];
                let mut rr = vec![0xAAu8; SPX_N + 16];
                unsafe {
                    fc(
                        cr.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        auth.as_ptr(),
                        h,
                        cc.as_ptr(),
                        ca.as_mut_ptr(),
                    )
                };
                unsafe {
                    fr(
                        rr.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        auth.as_ptr(),
                        h,
                        rc.as_ptr(),
                        ra.as_mut_ptr(),
                    )
                };
                let tag = format!("compute_root(h={h}, leaf_idx={leaf_idx}, off={idx_offset})");
                eq_bytes(&format!("{tag} root"), &cr, &rr);
                eq_u32s(&format!("{tag} addr"), &ca, &ra);
            }
        }
    }
}

// ===========================================================================
// Row 15 — utils.c treehash, driven with a callback supplied by the TEST.
// Both libraries must invoke it with identical arguments in identical order.
// ===========================================================================

static mut CALLS: Vec<(u32, [u32; 8])> = Vec::new();

/// Deterministic pseudo-leaf so both libraries see the same data, and record
/// the (addr_idx, tree_addr) pair for call-sequence comparison.
unsafe extern "C" fn gen_leaf_cb(leaf: *mut u8, _ctx: *const u8, addr_idx: u32, tree_addr: *const u32) {
    let ta = *(tree_addr as *const [u32; 8]);
    (*core::ptr::addr_of_mut!(CALLS)).push((addr_idx, ta));
    let out = core::slice::from_raw_parts_mut(leaf, SPX_N);
    // simple deterministic mixing of addr_idx and the address words
    let mut acc = addr_idx as u64 ^ 0xA5A5_5A5A_1234_5678;
    for w in ta.iter() {
        acc = acc.rotate_left(7) ^ (*w as u64).wrapping_mul(0x9E3779B97F4A7C15);
    }
    for (i, b) in out.iter_mut().enumerate() {
        acc = acc
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *b = (acc >> (56 - (i % 8) * 8)) as u8;
    }
}

#[test]
fn cfg15_treehash() {
    let p = load();
    type GenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeaf, *mut u32);
    let fc: Symbol<F> = p.c.sym("SPX_treehash");
    let fr: Symbol<F> = p.r.sym("SPX_treehash");

    let mut rng = Rng::new(15);
    let mut heights: Vec<u32> = vec![0, 1, 2, 3, (SPX_FORS_HEIGHT as u32).min(4)];
    heights.sort_unstable();
    heights.dedup();

    for &h in &heights {
        let n_leaves = 1u32 << h;
        let mut leaf_idxs: Vec<u32> = vec![0, 1, n_leaves - 1, n_leaves, u32::MAX];
        leaf_idxs.push(rng.next_u32() % n_leaves);
        for &leaf_idx in &leaf_idxs {
            for &idx_offset in &[0u32, 1, 64, rng.next_u32() & 0xffff] {
                let (cc, rc) = init_ctx_pair(&p, &mut rng);
                let addr = rng.addr();
                let mut ca = addr;
                let mut ra = addr;
                // `treehash` does no bounds checking: for a `leaf_idx` outside
                // `0..2^h` the C writes one node *past* `h*SPX_N` (see the
                // comment on the SPX_treehash wrapper), so give it head-room and
                // compare the whole buffer, including the over-run node.
                let ap_len = (h as usize + 2) * SPX_N;
                let mut c_root = vec![0xAAu8; SPX_N];
                let mut r_root = vec![0xAAu8; SPX_N];
                let mut c_auth = vec![0xAAu8; ap_len];
                let mut r_auth = vec![0xAAu8; ap_len];

                unsafe { (*core::ptr::addr_of_mut!(CALLS)).clear() };
                unsafe {
                    fc(
                        c_root.as_mut_ptr(),
                        c_auth.as_mut_ptr(),
                        cc.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        gen_leaf_cb,
                        ca.as_mut_ptr(),
                    )
                };
                let c_calls: Vec<(u32, [u32; 8])> =
                    unsafe { (*core::ptr::addr_of_mut!(CALLS)).clone() };

                unsafe { (*core::ptr::addr_of_mut!(CALLS)).clear() };
                unsafe {
                    fr(
                        r_root.as_mut_ptr(),
                        r_auth.as_mut_ptr(),
                        rc.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        gen_leaf_cb,
                        ra.as_mut_ptr(),
                    )
                };
                let r_calls: Vec<(u32, [u32; 8])> =
                    unsafe { (*core::ptr::addr_of_mut!(CALLS)).clone() };

                let tag = format!("treehash(h={h}, leaf_idx={leaf_idx}, off={idx_offset})");
                eq(&format!("{tag} gen_leaf call count"), c_calls.len(), r_calls.len());
                for (i, (a, b)) in c_calls.iter().zip(r_calls.iter()).enumerate() {
                    eq(&format!("{tag} gen_leaf call #{i} addr_idx"), a.0, b.0);
                    eq_u32s(&format!("{tag} gen_leaf call #{i} tree_addr"), &a.1, &b.1);
                }
                eq_bytes(&format!("{tag} root"), &c_root, &r_root);
                eq_bytes(&format!("{tag} auth_path"), &c_auth, &r_auth);
                eq_u32s(&format!("{tag} tree_addr"), &ca, &ra);
            }
        }
    }
}

// ===========================================================================
// Row 16-17 — utilsx1.c
// ===========================================================================

#[test]
fn cfg16_wots_treehashx1() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1);
    let fc: Symbol<F> = p.c.sym("SPX_wots_treehashx1");
    let fr: Symbol<F> = p.r.sym("SPX_wots_treehashx1");
    type CL = unsafe extern "C" fn(*mut core::ffi::c_uint, *const u8);
    let cl: Symbol<CL> = p.c.sym("SPX_chain_lengths");

    let mut rng = Rng::new(16);
    let h = SPX_TREE_HEIGHT as u32;
    let n_leaves = 1u32 << h;
    let mut leaf_idxs: Vec<u32> = vec![0, 1, n_leaves - 1, u32::MAX];
    leaf_idxs.push(rng.next_u32() % n_leaves);

    for &leaf_idx in &leaf_idxs {
        for &idx_offset in &[0u32, rng.next_u32() & 0xffff] {
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let root_in = rng.bytes(SPX_N);
            let mut steps_c = vec![0u32; SPX_WOTS_LEN];
            unsafe { cl(steps_c.as_mut_ptr() as *mut core::ffi::c_uint, root_in.as_ptr()) };
            let mut steps_r = steps_c.clone();

            let base = rng.addr();
            let mut tree_addr_c = base;
            let mut tree_addr_r = base;

            // merkle_sign lays the WOTS signature out immediately before the
            // authentication path, so replicate that single buffer exactly.
            let total = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
            let mut buf_c = vec![0xAAu8; total];
            let mut buf_r = vec![0xAAu8; total];

            let mut info_c = LeafInfoX1::zeroed();
            info_c.leaf_addr = base;
            info_c.pk_addr = base;
            info_c.wots_sign_leaf = leaf_idx;
            let mut info_r = info_c;
            info_c.wots_sig = buf_c.as_mut_ptr();
            info_r.wots_sig = buf_r.as_mut_ptr();
            info_c.wots_steps = steps_c.as_mut_ptr();
            info_r.wots_steps = steps_r.as_mut_ptr();

            let mut c_root = vec![0xAAu8; SPX_N];
            let mut r_root = vec![0xAAu8; SPX_N];
            unsafe {
                let ap = buf_c.as_mut_ptr().add(SPX_WOTS_BYTES);
                fc(
                    c_root.as_mut_ptr(),
                    ap,
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    tree_addr_c.as_mut_ptr(),
                    &mut info_c,
                )
            };
            unsafe {
                let ap = buf_r.as_mut_ptr().add(SPX_WOTS_BYTES);
                fr(
                    r_root.as_mut_ptr(),
                    ap,
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    tree_addr_r.as_mut_ptr(),
                    &mut info_r,
                )
            };
            let tag = format!("wots_treehashx1(leaf_idx={leaf_idx}, off={idx_offset})");
            eq_bytes(&format!("{tag} root"), &c_root, &r_root);
            eq_bytes(&format!("{tag} wots_sig || auth_path"), &buf_c, &buf_r);
            eq_u32s(&format!("{tag} tree_addr"), &tree_addr_c, &tree_addr_r);
            eq_u32s(&format!("{tag} info.leaf_addr"), &info_c.leaf_addr, &info_r.leaf_addr);
            eq_u32s(&format!("{tag} info.pk_addr"), &info_c.pk_addr, &info_r.pk_addr);
            eq_u32s(&format!("{tag} info.wots_steps"), &steps_c, &steps_r);
        }
    }
}

#[test]
fn cfg17_fors_treehashx1() {
    let p = load();
    type F =
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut ForsGenLeafInfo);
    let fc: Symbol<F> = p.c.sym("SPX_fors_treehashx1");
    let fr: Symbol<F> = p.r.sym("SPX_fors_treehashx1");

    let mut rng = Rng::new(17);
    let h = SPX_FORS_HEIGHT as u32;
    let n_leaves = 1u32 << h;
    let mut leaf_idxs: Vec<u32> = vec![0, 1, n_leaves - 1];
    leaf_idxs.push(rng.next_u32() % n_leaves);

    for &leaf_idx in &leaf_idxs {
        // idx_offset values exactly as fors_sign uses them: i * 2^FORS_HEIGHT
        for &i in &[0u32, 1, (SPX_FORS_TREES as u32) - 1] {
            let idx_offset = i * n_leaves;
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let base = rng.addr();
            let mut tac = base;
            let mut tar = base;
            let mut ic = ForsGenLeafInfo { leaf_addrx: base };
            let mut ir = ForsGenLeafInfo { leaf_addrx: base };
            let mut c_root = vec![0xAAu8; SPX_N];
            let mut r_root = vec![0xAAu8; SPX_N];
            let mut c_auth = vec![0xAAu8; h as usize * SPX_N];
            let mut r_auth = vec![0xAAu8; h as usize * SPX_N];
            unsafe {
                fc(
                    c_root.as_mut_ptr(),
                    c_auth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    tac.as_mut_ptr(),
                    &mut ic,
                )
            };
            unsafe {
                fr(
                    r_root.as_mut_ptr(),
                    r_auth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    tar.as_mut_ptr(),
                    &mut ir,
                )
            };
            let tag = format!("fors_treehashx1(leaf_idx={leaf_idx}, off={idx_offset})");
            eq_bytes(&format!("{tag} root"), &c_root, &r_root);
            eq_bytes(&format!("{tag} auth_path"), &c_auth, &r_auth);
            eq_u32s(&format!("{tag} tree_addr"), &tac, &tar);
            eq_u32s(&format!("{tag} leaf_addrx"), &ic.leaf_addrx, &ir.leaf_addrx);
        }
    }
}

// ===========================================================================
// Row 18-19 — fors.c
// ===========================================================================

#[test]
fn cfg18_fors_sign() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
    let fc: Symbol<F> = p.c.sym("SPX_fors_sign");
    let fr: Symbol<F> = p.r.sym("SPX_fors_sign");

    let mut rng = Rng::new(18);
    for it in 0..4 {
        let (cc, rc) = init_ctx_pair(&p, &mut rng);
        let m = match it {
            0 => vec![0u8; SPX_FORS_MSG_BYTES],
            1 => vec![0xffu8; SPX_FORS_MSG_BYTES],
            _ => rng.bytes(SPX_FORS_MSG_BYTES),
        };
        let addr = rng.addr();
        let mut cs = vec![0xAAu8; SPX_FORS_BYTES];
        let mut rs = vec![0xAAu8; SPX_FORS_BYTES];
        let mut cpk = vec![0xAAu8; SPX_N];
        let mut rpk = vec![0xAAu8; SPX_N];
        unsafe {
            fc(
                cs.as_mut_ptr(),
                cpk.as_mut_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                addr.as_ptr(),
            )
        };
        unsafe {
            fr(
                rs.as_mut_ptr(),
                rpk.as_mut_ptr(),
                m.as_ptr(),
                rc.as_ptr(),
                addr.as_ptr(),
            )
        };
        eq_bytes("fors_sign sig", &cs, &rs);
        eq_bytes("fors_sign pk", &cpk, &rpk);
    }
}

#[test]
fn cfg19_fors_pk_from_sig() {
    let p = load();
    type FS = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
    type FV = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
    let fsign: Symbol<FS> = p.c.sym("SPX_fors_sign");
    let fc: Symbol<FV> = p.c.sym("SPX_fors_pk_from_sig");
    let fr: Symbol<FV> = p.r.sym("SPX_fors_pk_from_sig");

    let mut rng = Rng::new(19);
    for valid in [true, false] {
        for _ in 0..4 {
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let m = rng.bytes(SPX_FORS_MSG_BYTES);
            let addr = rng.addr();
            let mut sig = vec![0u8; SPX_FORS_BYTES];
            if valid {
                let mut pk = vec![0u8; SPX_N];
                unsafe {
                    fsign(
                        sig.as_mut_ptr(),
                        pk.as_mut_ptr(),
                        m.as_ptr(),
                        cc.as_ptr(),
                        addr.as_ptr(),
                    )
                };
            } else {
                rng.fill(&mut sig);
            }
            let mut cpk = vec![0xAAu8; SPX_N];
            let mut rpk = vec![0xAAu8; SPX_N];
            unsafe { fc(cpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), cc.as_ptr(), addr.as_ptr()) };
            unsafe { fr(rpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), rc.as_ptr(), addr.as_ptr()) };
            eq_bytes(&format!("fors_pk_from_sig(valid={valid})"), &cpk, &rpk);
        }
    }
}

// ===========================================================================
// Row 20-21 — merkle.c
// ===========================================================================

#[test]
fn cfg20_merkle_sign() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
    let fc: Symbol<F> = p.c.sym("SPX_merkle_sign");
    let fr: Symbol<F> = p.r.sym("SPX_merkle_sign");
    type SetU32 = unsafe extern "C" fn(*mut u32, u32);
    type SetU64 = unsafe extern "C" fn(*mut u32, u64);
    let set_type: Symbol<SetU32> = p.c.sym("SPX_set_type");
    let set_layer: Symbol<SetU32> = p.c.sym("SPX_set_layer_addr");
    let set_tree: Symbol<SetU64> = p.c.sym("SPX_set_tree_addr");
    let set_kp: Symbol<SetU32> = p.c.sym("SPX_set_keypair_addr");
    let copy_sub: Symbol<unsafe extern "C" fn(*mut u32, *const u32)> =
        p.c.sym("SPX_copy_subtree_addr");

    let mut rng = Rng::new(20);
    let n_leaves = 1u32 << SPX_TREE_HEIGHT;
    let mut idx_leaves: Vec<u32> = vec![0, 1, n_leaves - 1, u32::MAX];
    idx_leaves.push(rng.next_u32() % n_leaves);

    for &idx_leaf in &idx_leaves {
        for &layer in &[0u32, 1, (SPX_D as u32) - 1] {
            let (cc, rc) = init_ctx_pair(&p, &mut rng);
            let tree = rng.next_u64();
            // set up wots_addr / tree_addr exactly as crypto_sign_signature does
            let mut wots_addr = [0u32; 8];
            let mut tree_addr = [0u32; 8];
            unsafe {
                set_type(wots_addr.as_mut_ptr(), 0 /* SPX_ADDR_TYPE_WOTS */);
                set_type(tree_addr.as_mut_ptr(), 2 /* SPX_ADDR_TYPE_HASHTREE */);
                set_layer(tree_addr.as_mut_ptr(), layer);
                set_tree(tree_addr.as_mut_ptr(), tree);
                copy_sub(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
                set_kp(wots_addr.as_mut_ptr(), idx_leaf);
            }
            let mut wc = wots_addr;
            let mut wr = wots_addr;
            let mut tc = tree_addr;
            let mut tr = tree_addr;

            let root_in = rng.bytes(SPX_N);
            let mut root_c = root_in.clone();
            let mut root_r = root_in.clone();
            let total = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
            let mut sc = vec![0xAAu8; total];
            let mut sr = vec![0xAAu8; total];
            unsafe {
                fc(
                    sc.as_mut_ptr(),
                    root_c.as_mut_ptr(),
                    cc.as_ptr(),
                    wc.as_mut_ptr(),
                    tc.as_mut_ptr(),
                    idx_leaf,
                )
            };
            unsafe {
                fr(
                    sr.as_mut_ptr(),
                    root_r.as_mut_ptr(),
                    rc.as_ptr(),
                    wr.as_mut_ptr(),
                    tr.as_mut_ptr(),
                    idx_leaf,
                )
            };
            let tag = format!("merkle_sign(idx_leaf={idx_leaf}, layer={layer})");
            eq_bytes(&format!("{tag} sig"), &sc, &sr);
            eq_bytes(&format!("{tag} root"), &root_c, &root_r);
            eq_u32s(&format!("{tag} wots_addr"), &wc, &wr);
            eq_u32s(&format!("{tag} tree_addr"), &tc, &tr);
        }
    }
}

#[test]
fn cfg21_merkle_gen_root() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8);
    let fc: Symbol<F> = p.c.sym("SPX_merkle_gen_root");
    let fr: Symbol<F> = p.r.sym("SPX_merkle_gen_root");

    let mut rng = Rng::new(21);
    for it in 0..3 {
        let (cc, rc) = if it == 0 {
            init_ctx_pair_from(&p, &vec![0u8; SPX_N], &vec![0u8; SPX_N])
        } else {
            init_ctx_pair(&p, &mut rng)
        };
        let mut cr = vec![0xAAu8; SPX_N];
        let mut rr = vec![0xAAu8; SPX_N];
        unsafe { fc(cr.as_mut_ptr(), cc.as_ptr()) };
        unsafe { fr(rr.as_mut_ptr(), rc.as_ptr()) };
        eq_bytes("merkle_gen_root", &cr, &rr);
    }
}

// ===========================================================================
// Row 22-29 — the public api.h surface
// ===========================================================================

#[test]
fn cfg22_size_constants() {
    // `load()` already asserts all four constants agree between C and Rust and
    // with the table in CONFIGS.md; make it an explicit test as well.
    let p = load();
    for name in [
        "crypto_sign_secretkeybytes",
        "crypto_sign_publickeybytes",
        "crypto_sign_bytes",
        "crypto_sign_seedbytes",
    ] {
        type F = unsafe extern "C" fn() -> u64;
        let fc: Symbol<F> = p.c.sym(name);
        let fr: Symbol<F> = p.r.sym(name);
        eq(name, unsafe { fc() }, unsafe { fr() });
    }
}

#[test]
fn cfg23_seed_keypair() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
    let fc: Symbol<F> = p.c.sym("crypto_sign_seed_keypair");
    let fr: Symbol<F> = p.r.sym("crypto_sign_seed_keypair");

    let mut rng = Rng::new(23);
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; CRYPTO_SEEDBYTES],
        vec![0xffu8; CRYPTO_SEEDBYTES],
    ];
    for _ in 0..3 {
        seeds.push(rng.bytes(CRYPTO_SEEDBYTES));
    }
    for seed in &seeds {
        let mut cpk = vec![0xAAu8; SPX_PK_BYTES];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES];
        let mut csk = vec![0xAAu8; SPX_SK_BYTES];
        let mut rsk = vec![0xAAu8; SPX_SK_BYTES];
        let cret = unsafe { fc(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
        let rret = unsafe { fr(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
        eq("crypto_sign_seed_keypair ret", cret, rret);
        eq_bytes("crypto_sign_seed_keypair pk", &cpk, &rpk);
        eq_bytes("crypto_sign_seed_keypair sk", &csk, &rsk);
    }
}

#[test]
fn cfg24_keypair_via_drbg() {
    let _g = drbg_guard();
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
    let fc: Symbol<F> = p.c.sym("crypto_sign_keypair");
    let fr: Symbol<F> = p.r.sym("crypto_sign_keypair");

    let mut rng = Rng::new(24);
    for pers in [false, true] {
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        let mut ps = [0u8; 48];
        rng.fill(&mut ps);
        seed_drbg(&p, &e, if pers { Some(&ps) } else { None });
        eq_drbg(&p, "DRBG_ctx after randombytes_init");

        let mut cpk = vec![0xAAu8; SPX_PK_BYTES];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES];
        let mut csk = vec![0xAAu8; SPX_SK_BYTES];
        let mut rsk = vec![0xAAu8; SPX_SK_BYTES];
        let cret = unsafe { fc(cpk.as_mut_ptr(), csk.as_mut_ptr()) };
        let rret = unsafe { fr(rpk.as_mut_ptr(), rsk.as_mut_ptr()) };
        eq("crypto_sign_keypair ret", cret, rret);
        eq_bytes("crypto_sign_keypair pk", &cpk, &rpk);
        eq_bytes("crypto_sign_keypair sk", &csk, &rsk);
        eq_drbg(&p, "DRBG_ctx after crypto_sign_keypair");
    }
}

/// Shared setup: identical key pair in both libraries (from a fixed seed).
fn keypair(p: &Pair, rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
    let fc: Symbol<F> = p.c.sym("crypto_sign_seed_keypair");
    let fr: Symbol<F> = p.r.sym("crypto_sign_seed_keypair");
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    unsafe { fc(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
    unsafe { fr(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
    eq_bytes("keypair pk", &cpk, &rpk);
    eq_bytes("keypair sk", &csk, &rsk);
    (cpk, csk)
}

fn sign_mlens() -> Vec<usize> {
    vec![0usize, 1, 32, 33, 64, 1000]
}

#[test]
fn cfg25_sign_signature() {
    let _g = drbg_guard();
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
    let fc: Symbol<F> = p.c.sym("crypto_sign_signature");
    let fr: Symbol<F> = p.r.sym("crypto_sign_signature");

    let mut rng = Rng::new(25);
    let (_pk, sk) = keypair(&p, &mut rng);
    for mlen in sign_mlens() {
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        seed_drbg(&p, &e, None);

        let mut cs = vec![0xAAu8; SPX_BYTES];
        let mut rs = vec![0xAAu8; SPX_BYTES];
        let mut cl: usize = 0xdead;
        let mut rl: usize = 0xdead;
        let cret = unsafe { fc(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, sk.as_ptr()) };
        let rret = unsafe { fr(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, sk.as_ptr()) };
        eq(&format!("crypto_sign_signature(mlen={mlen}) ret"), cret, rret);
        eq(&format!("crypto_sign_signature(mlen={mlen}) siglen"), cl, rl);
        eq_bytes(&format!("crypto_sign_signature(mlen={mlen}) sig"), &cs, &rs);
        eq_drbg(&p, "DRBG_ctx after crypto_sign_signature");
    }
}

#[test]
fn cfg26_sign_verify() {
    let _g = drbg_guard();
    let p = load();
    type FS = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
    type FV = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
    let fsign: Symbol<FS> = p.c.sym("crypto_sign_signature");
    let fc: Symbol<FV> = p.c.sym("crypto_sign_verify");
    let fr: Symbol<FV> = p.r.sym("crypto_sign_verify");

    let mut rng = Rng::new(26);
    let (pk, sk) = keypair(&p, &mut rng);
    for mlen in sign_mlens() {
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        seed_drbg(&p, &e, None);
        let mut sig = vec![0u8; SPX_BYTES];
        let mut sl: usize = 0;
        unsafe { fsign(sig.as_mut_ptr(), &mut sl, m.as_ptr(), mlen, sk.as_ptr()) };

        // valid
        let cret = unsafe { fc(sig.as_ptr(), sl, m.as_ptr(), mlen, pk.as_ptr()) };
        let rret = unsafe { fr(sig.as_ptr(), sl, m.as_ptr(), mlen, pk.as_ptr()) };
        eq(&format!("verify(valid, mlen={mlen})"), cret, rret);
        assert_eq!(cret, 0, "C failed to verify its own signature");

        // corrupt one bit in each region of the signature
        let probes = [
            0usize,
            SPX_N - 1,
            SPX_N,
            SPX_N + SPX_FORS_BYTES - 1,
            SPX_N + SPX_FORS_BYTES,
            SPX_BYTES - 1,
        ];
        for &pos in &probes {
            let mut bad = sig.clone();
            bad[pos] ^= 0x01;
            let cret = unsafe { fc(bad.as_ptr(), sl, m.as_ptr(), mlen, pk.as_ptr()) };
            let rret = unsafe { fr(bad.as_ptr(), sl, m.as_ptr(), mlen, pk.as_ptr()) };
            eq(&format!("verify(sig bit {pos} flipped)"), cret, rret);
        }
        // corrupt the public key
        for pos in [0usize, SPX_N, SPX_PK_BYTES - 1] {
            let mut bad = pk.clone();
            bad[pos] ^= 0x80;
            let cret = unsafe { fc(sig.as_ptr(), sl, m.as_ptr(), mlen, bad.as_ptr()) };
            let rret = unsafe { fr(sig.as_ptr(), sl, m.as_ptr(), mlen, bad.as_ptr()) };
            eq(&format!("verify(pk byte {pos} corrupted)"), cret, rret);
        }
        // corrupt the message
        if mlen > 0 {
            let mut bad = m.clone();
            bad[mlen - 1] ^= 0x01;
            let cret = unsafe { fc(sig.as_ptr(), sl, bad.as_ptr(), mlen, pk.as_ptr()) };
            let rret = unsafe { fr(sig.as_ptr(), sl, bad.as_ptr(), mlen, pk.as_ptr()) };
            eq("verify(message corrupted)", cret, rret);
        }
    }
}

#[test]
fn cfg27_crypto_sign() {
    let _g = drbg_guard();
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    let fc: Symbol<F> = p.c.sym("crypto_sign");
    let fr: Symbol<F> = p.r.sym("crypto_sign");

    let mut rng = Rng::new(27);
    let (_pk, sk) = keypair(&p, &mut rng);
    for mlen in [0usize, 1, 32, 1000] {
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        seed_drbg(&p, &e, None);
        let mut cs = vec![0xAAu8; SPX_BYTES + mlen];
        let mut rs = vec![0xAAu8; SPX_BYTES + mlen];
        let mut cl: u64 = 0;
        let mut rl: u64 = 0;
        let cret = unsafe { fc(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen as u64, sk.as_ptr()) };
        let rret = unsafe { fr(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen as u64, sk.as_ptr()) };
        eq(&format!("crypto_sign(mlen={mlen}) ret"), cret, rret);
        eq(&format!("crypto_sign(mlen={mlen}) smlen"), cl, rl);
        eq_bytes(&format!("crypto_sign(mlen={mlen}) sm"), &cs, &rs);
        eq(
            &format!("crypto_sign(mlen={mlen}) smlen value"),
            cl as usize,
            SPX_BYTES + mlen,
        );
    }
}

#[test]
fn cfg28_crypto_sign_open() {
    let _g = drbg_guard();
    let p = load();
    type FS = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    type FO = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    let fsign: Symbol<FS> = p.c.sym("crypto_sign");
    let fc: Symbol<FO> = p.c.sym("crypto_sign_open");
    let fr: Symbol<FO> = p.r.sym("crypto_sign_open");

    let mut rng = Rng::new(28);
    let (pk, sk) = keypair(&p, &mut rng);
    for mlen in [0usize, 1, 32, 1000] {
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        seed_drbg(&p, &e, None);
        let mut sm = vec![0u8; SPX_BYTES + mlen];
        let mut smlen: u64 = 0;
        unsafe { fsign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen as u64, sk.as_ptr()) };

        // The C memsets `smlen` bytes of `m` on failure, so the output buffer
        // must be smlen bytes long even though only mlen bytes are produced.
        let mut cm = vec![0xAAu8; smlen as usize];
        let mut rm = vec![0xAAu8; smlen as usize];
        let mut cl: u64 = 0xdead;
        let mut rl: u64 = 0xdead;
        let cret = unsafe { fc(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr()) };
        let rret = unsafe { fr(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr()) };
        eq(&format!("crypto_sign_open(mlen={mlen}) ret"), cret, rret);
        eq(&format!("crypto_sign_open(mlen={mlen}) mlen"), cl, rl);
        eq_bytes(&format!("crypto_sign_open(mlen={mlen}) m"), &cm, &rm);
        assert_eq!(cret, 0, "C failed to open its own signed message");
        eq_bytes("recovered message", &cm[..mlen], &m);
    }
}

#[test]
fn cfg29_cross_verify() {
    let _g = drbg_guard();
    let p = load();
    type FS = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
    type FV = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
    let csign: Symbol<FS> = p.c.sym("crypto_sign_signature");
    let rsign: Symbol<FS> = p.r.sym("crypto_sign_signature");
    let cver: Symbol<FV> = p.c.sym("crypto_sign_verify");
    let rver: Symbol<FV> = p.r.sym("crypto_sign_verify");

    let mut rng = Rng::new(29);
    for _ in 0..3 {
        let (pk, sk) = keypair(&p, &mut rng);
        let mlen = rng.below(200) as usize;
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);

        seed_drbg(&p, &e, None);
        let mut c_sig = vec![0u8; SPX_BYTES];
        let mut c_l: usize = 0;
        unsafe { csign(c_sig.as_mut_ptr(), &mut c_l, m.as_ptr(), mlen, sk.as_ptr()) };

        seed_drbg(&p, &e, None);
        let mut r_sig = vec![0u8; SPX_BYTES];
        let mut r_l: usize = 0;
        unsafe { rsign(r_sig.as_mut_ptr(), &mut r_l, m.as_ptr(), mlen, sk.as_ptr()) };

        eq_bytes("cross: signatures", &c_sig, &r_sig);

        // C signature -> Rust verifier, and vice versa
        assert_eq!(
            unsafe { rver(c_sig.as_ptr(), c_l, m.as_ptr(), mlen, pk.as_ptr()) },
            0,
            "Rust rejected a C-produced signature"
        );
        assert_eq!(
            unsafe { cver(r_sig.as_ptr(), r_l, m.as_ptr(), mlen, pk.as_ptr()) },
            0,
            "C rejected a Rust-produced signature"
        );
    }
}

// ===========================================================================
// Row 30-33 — rng.c (the deterministic NIST DRBG and the seed expander)
// ===========================================================================

#[test]
fn cfg30_aes256_ecb() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let fc: Symbol<F> = p.c.sym("AES256_ECB");
    let fr: Symbol<F> = p.r.sym("AES256_ECB");

    let mut rng = Rng::new(30);
    let mut cases: Vec<([u8; 32], [u8; 16])> = vec![([0u8; 32], [0u8; 16]), ([0xff; 32], [0xff; 16])];
    for _ in 0..64 {
        let mut k = [0u8; 32];
        rng.fill(&mut k);
        let mut c = [0u8; 16];
        rng.fill(&mut c);
        cases.push((k, c));
    }
    for (k, ctr) in &cases {
        let mut kc = *k;
        let mut kr = *k;
        let mut cc = *ctr;
        let mut cr = *ctr;
        let mut ob = [0xAAu8; 16];
        let mut or = [0xAAu8; 16];
        unsafe { fc(kc.as_mut_ptr(), cc.as_mut_ptr(), ob.as_mut_ptr()) };
        unsafe { fr(kr.as_mut_ptr(), cr.as_mut_ptr(), or.as_mut_ptr()) };
        eq_bytes("AES256_ECB out", &ob, &or);
        eq_bytes("AES256_ECB must not modify key", &kc, &kr);
        eq_bytes("AES256_ECB must not modify ctr", &cc, &cr);
    }
}

#[test]
fn cfg31_drbg_update() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let fc: Symbol<F> = p.c.sym("AES256_CTR_DRBG_Update");
    let fr: Symbol<F> = p.r.sym("AES256_CTR_DRBG_Update");

    let mut rng = Rng::new(31);
    let v_cases: Vec<[u8; 16]> = vec![[0u8; 16], [0xffu8; 16], {
        let mut v = [0u8; 16];
        v[15] = 0xff;
        v
    }];
    for with_pd in [false, true] {
        for v0 in &v_cases {
            for _ in 0..8 {
                let mut key = [0u8; 32];
                rng.fill(&mut key);
                let mut pd = [0u8; 48];
                rng.fill(&mut pd);

                let mut kc = key;
                let mut kr = key;
                let mut vc = *v0;
                let mut vr = *v0;
                let mut pdc = pd;
                let mut pdr = pd;
                let pc = if with_pd {
                    pdc.as_mut_ptr()
                } else {
                    core::ptr::null_mut()
                };
                let pr = if with_pd {
                    pdr.as_mut_ptr()
                } else {
                    core::ptr::null_mut()
                };
                unsafe { fc(pc, kc.as_mut_ptr(), vc.as_mut_ptr()) };
                unsafe { fr(pr, kr.as_mut_ptr(), vr.as_mut_ptr()) };
                eq_bytes(&format!("DRBG_Update(pd={with_pd}) Key"), &kc, &kr);
                eq_bytes(&format!("DRBG_Update(pd={with_pd}) V"), &vc, &vr);
                eq_bytes("DRBG_Update must not modify provided_data", &pdc, &pdr);
            }
        }
    }
}

#[test]
fn cfg32_drbg_stream() {
    let _g = drbg_guard();
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, u64) -> i32;
    let fc: Symbol<F> = p.c.sym("randombytes");
    let fr: Symbol<F> = p.r.sym("randombytes");

    let mut rng = Rng::new(32);
    for pers in [false, true] {
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        let mut ps = [0u8; 48];
        rng.fill(&mut ps);
        seed_drbg(&p, &e, if pers { Some(&ps) } else { None });
        eq_drbg(&p, "DRBG_ctx after randombytes_init");

        for &xlen in &[0u64, 1, 15, 16, 17, 31, 32, 47, 48, 49, 1000] {
            let mut cb = vec![0xAAu8; xlen as usize + 16];
            let mut rb = vec![0xAAu8; xlen as usize + 16];
            let cret = unsafe { fc(cb.as_mut_ptr(), xlen) };
            let rret = unsafe { fr(rb.as_mut_ptr(), xlen) };
            eq(&format!("randombytes({xlen}) ret"), cret, rret);
            eq_bytes(&format!("randombytes({xlen}) out"), &cb, &rb);
            eq_drbg(&p, &format!("DRBG_ctx after randombytes({xlen})"));
        }
    }
}

#[test]
fn cfg33_seedexpander_stream() {
    let p = load();
    type FI = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32;
    type FS = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, u64) -> i32;
    let ic: Symbol<FI> = p.c.sym("seedexpander_init");
    let ir: Symbol<FI> = p.r.sym("seedexpander_init");
    let sc: Symbol<FS> = p.c.sym("seedexpander");
    let sr: Symbol<FS> = p.r.sym("seedexpander");

    let mut rng = Rng::new(33);
    for &maxlen in &[16u64, 17, 256, 4096, 0xFFFF_FFFF] {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let mut div = [0u8; 8];
        rng.fill(&mut div);
        // force the ctr carry path on one of the runs
        if maxlen == 256 {
            div = [0xff; 8];
        }

        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let mut s1 = seed;
        let mut d1 = div;
        let cret = unsafe { ic(&mut cctx, s1.as_mut_ptr(), d1.as_mut_ptr(), maxlen) };
        let rret = unsafe { ir(&mut rctx, s1.as_mut_ptr(), d1.as_mut_ptr(), maxlen) };
        eq(&format!("seedexpander_init(maxlen={maxlen}) ret"), cret, rret);
        eq(&format!("seedexpander_init(maxlen={maxlen}) ctx"), cctx, rctx);
        let _ = &mut d1;

        for &xlen in &[1u64, 2, 15, 16, 17, 31, 32, 33, 100] {
            if xlen >= cctx.length_remaining {
                continue;
            }
            let mut cb = vec![0xAAu8; xlen as usize + 8];
            let mut rb = vec![0xAAu8; xlen as usize + 8];
            let cret = unsafe { sc(&mut cctx, cb.as_mut_ptr(), xlen) };
            let rret = unsafe { sr(&mut rctx, rb.as_mut_ptr(), xlen) };
            eq(&format!("seedexpander({xlen}) ret"), cret, rret);
            eq_bytes(&format!("seedexpander({xlen}) out"), &cb, &rb);
            eq(&format!("seedexpander({xlen}) ctx"), cctx, rctx);
        }
    }
}

// ===========================================================================
// Row 34-36 — BLAKE backend primitives
// ===========================================================================

#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
mod blake_rows {
    use super::*;

    fn lens256() -> Vec<usize> {
        vec![0, 1, 2, 54, 55, 56, 57, 62, 63, 64, 65, 118, 119, 120, 127, 128, 129, 1000]
    }
    fn lens512() -> Vec<usize> {
        vec![
            0, 1, 2, 110, 111, 112, 113, 126, 127, 128, 129, 238, 239, 240, 255, 256, 257, 1000,
        ]
    }

    #[test]
    fn cfg34_blake256() {
        let p = load();
        type FH = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
        type FInit = unsafe extern "C" fn(*mut BlakeState256);
        type FUpd = unsafe extern "C" fn(*mut BlakeState256, *const u8, u64);
        type FFin = unsafe extern "C" fn(*mut BlakeState256, *mut u8);
        type FCmp = unsafe extern "C" fn(*mut BlakeState256, *const u8);

        let hc: Symbol<FH> = p.c.sym("blake256");
        let hr: Symbol<FH> = p.r.sym("blake256");
        let ic: Symbol<FInit> = p.c.sym("blake256_init");
        let ir: Symbol<FInit> = p.r.sym("blake256_init");
        let uc: Symbol<FUpd> = p.c.sym("blake256_update");
        let ur: Symbol<FUpd> = p.r.sym("blake256_update");
        let fc: Symbol<FFin> = p.c.sym("blake256_final");
        let fr: Symbol<FFin> = p.r.sym("blake256_final");
        let cc: Symbol<FCmp> = p.c.sym("blake256_compress");
        let cr: Symbol<FCmp> = p.r.sym("blake256_compress");

        let mut rng = Rng::new(34);

        // one-shot
        for len in lens256() {
            for it in 0..2 {
                let data = if it == 0 { vec![0u8; len] } else { rng.bytes(len) };
                let mut ob = [0xAAu8; 32];
                let mut or = [0xAAu8; 32];
                let a = unsafe { hc(ob.as_mut_ptr(), data.as_ptr(), len as u64) };
                let b = unsafe { hr(or.as_mut_ptr(), data.as_ptr(), len as u64) };
                eq(&format!("blake256({len}) ret"), a, b);
                eq_bytes(&format!("blake256({len})"), &ob, &or);
            }
        }

        // incremental: init/update*/final with random bit-length chunks
        for len in lens256() {
            let data = rng.bytes(len);
            let mut sc = BlakeState256::zeroed();
            let mut sr = BlakeState256::zeroed();
            unsafe { ic(&mut sc) };
            unsafe { ir(&mut sr) };
            eq("blake256_init state", sc, sr);
            let mut off = 0usize;
            while off < len {
                // chunk sizes in BYTES; blake256_update takes a BIT length
                let chunk = (rng.below(70) as usize + 1).min(len - off);
                unsafe { uc(&mut sc, data[off..].as_ptr(), (chunk * 8) as u64) };
                unsafe { ur(&mut sr, data[off..].as_ptr(), (chunk * 8) as u64) };
                eq(&format!("blake256_update state (off={off})"), sc, sr);
                off += chunk;
            }
            let mut ob = [0xAAu8; 32];
            let mut or = [0xAAu8; 32];
            unsafe { fc(&mut sc, ob.as_mut_ptr()) };
            unsafe { fr(&mut sr, or.as_mut_ptr()) };
            eq_bytes(&format!("blake256 incremental digest (len={len})"), &ob, &or);
            eq("blake256_final state", sc, sr);
        }

        // raw compress on a random state and a random block
        for _ in 0..32 {
            let mut sc = BlakeState256::zeroed();
            for x in sc.h.iter_mut() {
                *x = rng.next_u32();
            }
            for x in sc.s.iter_mut() {
                *x = rng.next_u32();
            }
            for x in sc.t.iter_mut() {
                *x = rng.next_u32();
            }
            sc.nullt = (rng.next_u32() & 1) as i32;
            sc.buflen = 0;
            rng.fill(&mut sc.buf);
            let sr = sc;
            let mut sc = sc;
            let mut sr = sr;
            let block = rng.bytes(64);
            unsafe { cc(&mut sc, block.as_ptr()) };
            unsafe { cr(&mut sr, block.as_ptr()) };
            eq("blake256_compress state", sc, sr);
        }
    }

    #[test]
    fn cfg35_blake512() {
        let p = load();
        type FH = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
        type FInit = unsafe extern "C" fn(*mut BlakeState512);
        type FUpd = unsafe extern "C" fn(*mut BlakeState512, *const u8, u64);
        type FFin = unsafe extern "C" fn(*mut BlakeState512, *mut u8);
        type FCmp = unsafe extern "C" fn(*mut BlakeState512, *const u8);

        let hc: Symbol<FH> = p.c.sym("blake512");
        let hr: Symbol<FH> = p.r.sym("blake512");
        let ic: Symbol<FInit> = p.c.sym("blake512_init");
        let ir: Symbol<FInit> = p.r.sym("blake512_init");
        let uc: Symbol<FUpd> = p.c.sym("blake512_update");
        let ur: Symbol<FUpd> = p.r.sym("blake512_update");
        let fc: Symbol<FFin> = p.c.sym("blake512_final");
        let fr: Symbol<FFin> = p.r.sym("blake512_final");
        let cc: Symbol<FCmp> = p.c.sym("blake512_compress");
        let cr: Symbol<FCmp> = p.r.sym("blake512_compress");

        let mut rng = Rng::new(35);

        for len in lens512() {
            for it in 0..2 {
                let data = if it == 0 { vec![0u8; len] } else { rng.bytes(len) };
                let mut ob = [0xAAu8; 64];
                let mut or = [0xAAu8; 64];
                let a = unsafe { hc(ob.as_mut_ptr(), data.as_ptr(), len as u64) };
                let b = unsafe { hr(or.as_mut_ptr(), data.as_ptr(), len as u64) };
                eq(&format!("blake512({len}) ret"), a, b);
                eq_bytes(&format!("blake512({len})"), &ob, &or);
            }
        }

        for len in lens512() {
            let data = rng.bytes(len);
            let mut sc = BlakeState512::zeroed();
            let mut sr = BlakeState512::zeroed();
            unsafe { ic(&mut sc) };
            unsafe { ir(&mut sr) };
            eq("blake512_init state", sc, sr);
            let mut off = 0usize;
            while off < len {
                let chunk = (rng.below(140) as usize + 1).min(len - off);
                unsafe { uc(&mut sc, data[off..].as_ptr(), (chunk * 8) as u64) };
                unsafe { ur(&mut sr, data[off..].as_ptr(), (chunk * 8) as u64) };
                eq(&format!("blake512_update state (off={off})"), sc, sr);
                off += chunk;
            }
            let mut ob = [0xAAu8; 64];
            let mut or = [0xAAu8; 64];
            unsafe { fc(&mut sc, ob.as_mut_ptr()) };
            unsafe { fr(&mut sr, or.as_mut_ptr()) };
            eq_bytes(&format!("blake512 incremental digest (len={len})"), &ob, &or);
            eq("blake512_final state", sc, sr);
        }

        for _ in 0..32 {
            let mut sc = BlakeState512::zeroed();
            for x in sc.h.iter_mut() {
                *x = rng.next_u64();
            }
            for x in sc.s.iter_mut() {
                *x = rng.next_u64();
            }
            for x in sc.t.iter_mut() {
                *x = rng.next_u64();
            }
            sc.nullt = (rng.next_u32() & 1) as i32;
            sc.buflen = 0;
            rng.fill(&mut sc.buf);
            let mut sr = sc;
            let mut sc = sc;
            let block = rng.bytes(128);
            unsafe { cc(&mut sc, block.as_ptr()) };
            unsafe { cr(&mut sr, block.as_ptr()) };
            eq("blake512_compress state", sc, sr);
        }
    }

    #[test]
    fn cfg36_blake_mgf1() {
        let p = load();
        type F = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
        let mut rng = Rng::new(36);

        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let fc: Symbol<F> = p.c.sym(name);
            let fr: Symbol<F> = p.r.sym(name);
            for &outlen in &[0u64, 1, 31, 32, 33, 63, 64, 65, 100, 200, 512, 1000] {
                for &inlen in &[0u64, 1, 16, 32, 64, 192, 256, 257, 1000] {
                    let input = rng.bytes(inlen as usize);
                    let mut cb = vec![0xAAu8; outlen as usize + 16];
                    let mut rb = vec![0xAAu8; outlen as usize + 16];
                    unsafe { fc(cb.as_mut_ptr(), outlen, input.as_ptr(), inlen) };
                    unsafe { fr(rb.as_mut_ptr(), outlen, input.as_ptr(), inlen) };
                    eq_bytes(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cb, &rb);
                }
            }
        }

        // the exported `cst` data symbol (const u64 cst[16] in blake512.c)
        let cc = p.c.data::<[u64; 16]>("cst");
        let cr = p.r.data::<[u64; 16]>("cst");
        let a = unsafe { *cc };
        let b = unsafe { *cr };
        eq("cst[16] data symbol", a, b);
    }
}

// ===========================================================================
// Row 37-39 — SHA2 backend primitives
// ===========================================================================

#[cfg(feature = "sha2")]
mod sha2_rows {
    use super::*;

    #[test]
    fn cfg37_sha256() {
        let p = load();
        type FH = unsafe extern "C" fn(*mut u8, *const u8, usize);
        type FInit = unsafe extern "C" fn(*mut u8);
        type FBlk = unsafe extern "C" fn(*mut u8, *const u8, usize);
        type FFin = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
        let hc: Symbol<FH> = p.c.sym("sha256");
        let hr: Symbol<FH> = p.r.sym("sha256");
        let ic: Symbol<FInit> = p.c.sym("sha256_inc_init");
        let ir: Symbol<FInit> = p.r.sym("sha256_inc_init");
        let bc: Symbol<FBlk> = p.c.sym("sha256_inc_blocks");
        let br: Symbol<FBlk> = p.r.sym("sha256_inc_blocks");
        let fc: Symbol<FFin> = p.c.sym("sha256_inc_finalize");
        let fr: Symbol<FFin> = p.r.sym("sha256_inc_finalize");

        let mut rng = Rng::new(37);
        let lens = [0usize, 1, 54, 55, 56, 57, 63, 64, 65, 119, 120, 128, 1000];
        for &len in &lens {
            let data = rng.bytes(len);
            let mut ob = [0xAAu8; 32];
            let mut or = [0xAAu8; 32];
            unsafe { hc(ob.as_mut_ptr(), data.as_ptr(), len) };
            unsafe { hr(or.as_mut_ptr(), data.as_ptr(), len) };
            eq_bytes(&format!("sha256({len})"), &ob, &or);

            // incremental: feed whole 64-byte blocks then finalize the tail
            let mut cs = [0u8; 40];
            let mut rs = [0u8; 40];
            unsafe { ic(cs.as_mut_ptr()) };
            unsafe { ir(rs.as_mut_ptr()) };
            eq_bytes("sha256_inc_init state", &cs, &rs);
            let nblocks = len / 64;
            if nblocks > 0 {
                unsafe { bc(cs.as_mut_ptr(), data.as_ptr(), nblocks) };
                unsafe { br(rs.as_mut_ptr(), data.as_ptr(), nblocks) };
                eq_bytes("sha256_inc_blocks state", &cs, &rs);
            }
            let tail = &data[nblocks * 64..];
            let mut ob2 = [0xAAu8; 32];
            let mut or2 = [0xAAu8; 32];
            unsafe { fc(ob2.as_mut_ptr(), cs.as_mut_ptr(), tail.as_ptr(), tail.len()) };
            unsafe { fr(or2.as_mut_ptr(), rs.as_mut_ptr(), tail.as_ptr(), tail.len()) };
            eq_bytes(&format!("sha256 incremental({len})"), &ob2, &or2);
            eq_bytes("sha256_inc_finalize state", &cs, &rs);
            eq_bytes("sha256 one-shot vs incremental", &ob, &ob2);
        }
    }

    #[test]
    fn cfg38_sha512() {
        let p = load();
        type FH = unsafe extern "C" fn(*mut u8, *const u8, usize);
        type FInit = unsafe extern "C" fn(*mut u8);
        type FBlk = unsafe extern "C" fn(*mut u8, *const u8, usize);
        type FFin = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
        let hc: Symbol<FH> = p.c.sym("sha512");
        let hr: Symbol<FH> = p.r.sym("sha512");
        let ic: Symbol<FInit> = p.c.sym("sha512_inc_init");
        let ir: Symbol<FInit> = p.r.sym("sha512_inc_init");
        let bc: Symbol<FBlk> = p.c.sym("sha512_inc_blocks");
        let br: Symbol<FBlk> = p.r.sym("sha512_inc_blocks");
        let fc: Symbol<FFin> = p.c.sym("sha512_inc_finalize");
        let fr: Symbol<FFin> = p.r.sym("sha512_inc_finalize");

        let mut rng = Rng::new(38);
        let lens = [0usize, 1, 110, 111, 112, 113, 127, 128, 129, 255, 256, 1000];
        for &len in &lens {
            let data = rng.bytes(len);
            let mut ob = [0xAAu8; 64];
            let mut or = [0xAAu8; 64];
            unsafe { hc(ob.as_mut_ptr(), data.as_ptr(), len) };
            unsafe { hr(or.as_mut_ptr(), data.as_ptr(), len) };
            eq_bytes(&format!("sha512({len})"), &ob, &or);

            let mut cs = [0u8; 72];
            let mut rs = [0u8; 72];
            unsafe { ic(cs.as_mut_ptr()) };
            unsafe { ir(rs.as_mut_ptr()) };
            eq_bytes("sha512_inc_init state", &cs, &rs);
            let nblocks = len / 128;
            if nblocks > 0 {
                unsafe { bc(cs.as_mut_ptr(), data.as_ptr(), nblocks) };
                unsafe { br(rs.as_mut_ptr(), data.as_ptr(), nblocks) };
                eq_bytes("sha512_inc_blocks state", &cs, &rs);
            }
            let tail = &data[nblocks * 128..];
            let mut ob2 = [0xAAu8; 64];
            let mut or2 = [0xAAu8; 64];
            unsafe { fc(ob2.as_mut_ptr(), cs.as_mut_ptr(), tail.as_ptr(), tail.len()) };
            unsafe { fr(or2.as_mut_ptr(), rs.as_mut_ptr(), tail.as_ptr(), tail.len()) };
            eq_bytes(&format!("sha512 incremental({len})"), &ob2, &or2);
            eq_bytes("sha512_inc_finalize state", &cs, &rs);
            eq_bytes("sha512 one-shot vs incremental", &ob, &ob2);
        }
    }

    #[test]
    fn cfg39_sha2_mgf1_seed_state() {
        let p = load();
        type F = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
        let mut rng = Rng::new(39);
        for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let fc: Symbol<F> = p.c.sym(name);
            let fr: Symbol<F> = p.r.sym(name);
            for &outlen in &[0u64, 1, 31, 32, 33, 63, 64, 65, 100, 200, 512, 1000] {
                for &inlen in &[0u64, 1, 16, 32, 64, 192, 256, 257, 1000] {
                    let input = rng.bytes(inlen as usize);
                    let mut cb = vec![0xAAu8; outlen as usize + 16];
                    let mut rb = vec![0xAAu8; outlen as usize + 16];
                    unsafe { fc(cb.as_mut_ptr(), outlen, input.as_ptr(), inlen) };
                    unsafe { fr(rb.as_mut_ptr(), outlen, input.as_ptr(), inlen) };
                    eq_bytes(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cb, &rb);
                }
            }
        }

        // SPX_seed_state fills the ctx's precomputed hash states
        type FS = unsafe extern "C" fn(*mut u8);
        let sc: Symbol<FS> = p.c.sym("SPX_seed_state");
        let sr: Symbol<FS> = p.r.sym("SPX_seed_state");
        for _ in 0..16 {
            let ps = rng.bytes(SPX_N);
            let ss = rng.bytes(SPX_N);
            let mut cb = new_ctx_buf();
            let mut rb = new_ctx_buf();
            cb.set_seeds(&ps, &ss);
            rb.set_seeds(&ps, &ss);
            unsafe { sc(cb.as_mut_ptr()) };
            unsafe { sr(rb.as_mut_ptr()) };
            eq_bytes("SPX_seed_state ctx image", cb.bytes(), rb.bytes());
        }
    }
}

// ===========================================================================
// Row 40 — SHAKE backend primitives
// ===========================================================================

#[cfg(all(feature = "shake", not(feature = "sha2")))]
mod shake_rows {
    use super::*;

    #[test]
    fn cfg40_shake256() {
        let p = load();
        type FH = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
        type FAbs = unsafe extern "C" fn(*mut u64, *const u8, usize);
        type FSqz = unsafe extern "C" fn(*mut u8, usize, *mut u64);
        type FIncInit = unsafe extern "C" fn(*mut u64);
        type FIncAbs = unsafe extern "C" fn(*mut u64, *const u8, usize);
        type FIncFin = unsafe extern "C" fn(*mut u64);
        type FIncSqz = unsafe extern "C" fn(*mut u8, usize, *mut u64);

        let hc: Symbol<FH> = p.c.sym("shake256");
        let hr: Symbol<FH> = p.r.sym("shake256");
        let ac: Symbol<FAbs> = p.c.sym("shake256_absorb");
        let ar: Symbol<FAbs> = p.r.sym("shake256_absorb");
        let qc: Symbol<FSqz> = p.c.sym("shake256_squeezeblocks");
        let qr: Symbol<FSqz> = p.r.sym("shake256_squeezeblocks");
        let iic: Symbol<FIncInit> = p.c.sym("shake256_inc_init");
        let iir: Symbol<FIncInit> = p.r.sym("shake256_inc_init");
        let iac: Symbol<FIncAbs> = p.c.sym("shake256_inc_absorb");
        let iar: Symbol<FIncAbs> = p.r.sym("shake256_inc_absorb");
        let ifc: Symbol<FIncFin> = p.c.sym("shake256_inc_finalize");
        let ifr: Symbol<FIncFin> = p.r.sym("shake256_inc_finalize");
        let isc: Symbol<FIncSqz> = p.c.sym("shake256_inc_squeeze");
        let isr: Symbol<FIncSqz> = p.r.sym("shake256_inc_squeeze");

        let mut rng = Rng::new(40);
        let inlens = [0usize, 1, 2, 135, 136, 137, 271, 272, 273, 1000];
        let outlens = [0usize, 1, 32, 135, 136, 137, 272, 300];

        for &inlen in &inlens {
            let data = rng.bytes(inlen);
            for &outlen in &outlens {
                let mut cb = vec![0xAAu8; outlen + 16];
                let mut rb = vec![0xAAu8; outlen + 16];
                unsafe { hc(cb.as_mut_ptr(), outlen, data.as_ptr(), inlen) };
                unsafe { hr(rb.as_mut_ptr(), outlen, data.as_ptr(), inlen) };
                eq_bytes(&format!("shake256(in={inlen}, out={outlen})"), &cb, &rb);
            }

            // absorb + squeezeblocks (the non-incremental, rate-aligned API)
            let mut cs = [0u64; 25];
            let mut rs = [0u64; 25];
            unsafe { ac(cs.as_mut_ptr(), data.as_ptr(), inlen) };
            unsafe { ar(rs.as_mut_ptr(), data.as_ptr(), inlen) };
            eq("shake256_absorb state", cs, rs);
            for &nblocks in &[0usize, 1, 2, 3] {
                let mut csx = cs;
                let mut rsx = rs;
                let mut cb = vec![0xAAu8; nblocks * 136 + 16];
                let mut rb = vec![0xAAu8; nblocks * 136 + 16];
                unsafe { qc(cb.as_mut_ptr(), nblocks, csx.as_mut_ptr()) };
                unsafe { qr(rb.as_mut_ptr(), nblocks, rsx.as_mut_ptr()) };
                eq_bytes(&format!("shake256_squeezeblocks({nblocks})"), &cb, &rb);
                eq("shake256_squeezeblocks state", csx, rsx);
            }

            // incremental absorb in random chunks, then squeeze in random chunks
            let mut cs = [0u64; 26];
            let mut rs = [0u64; 26];
            unsafe { iic(cs.as_mut_ptr()) };
            unsafe { iir(rs.as_mut_ptr()) };
            eq("shake256_inc_init state", cs, rs);
            let mut off = 0usize;
            while off < inlen {
                let chunk = (rng.below(150) as usize + 1).min(inlen - off);
                unsafe { iac(cs.as_mut_ptr(), data[off..].as_ptr(), chunk) };
                unsafe { iar(rs.as_mut_ptr(), data[off..].as_ptr(), chunk) };
                eq("shake256_inc_absorb state", cs, rs);
                off += chunk;
            }
            unsafe { ifc(cs.as_mut_ptr()) };
            unsafe { ifr(rs.as_mut_ptr()) };
            eq("shake256_inc_finalize state", cs, rs);
            for _ in 0..4 {
                let outlen = rng.below(200) as usize;
                let mut cb = vec![0xAAu8; outlen + 16];
                let mut rb = vec![0xAAu8; outlen + 16];
                unsafe { isc(cb.as_mut_ptr(), outlen, cs.as_mut_ptr()) };
                unsafe { isr(rb.as_mut_ptr(), outlen, rs.as_mut_ptr()) };
                eq_bytes(&format!("shake256_inc_squeeze({outlen})"), &cb, &rb);
                eq("shake256_inc_squeeze state", cs, rs);
            }
        }
    }
}

// ===========================================================================
// Row 41 — HARAKA backend primitives
// ===========================================================================

#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
mod haraka_rows {
    use super::*;

    #[test]
    fn cfg41_haraka() {
        let p = load();
        type FTweak = unsafe extern "C" fn(*mut u8);
        type FBlk = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
        type FS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
        type FIncInit = unsafe extern "C" fn(*mut u8);
        type FIncAbs = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
        type FIncFin = unsafe extern "C" fn(*mut u8);
        type FIncSqz = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);

        let tc: Symbol<FTweak> = p.c.sym("SPX_tweak_constants");
        let tr: Symbol<FTweak> = p.r.sym("SPX_tweak_constants");

        let mut rng = Rng::new(41);
        for _ in 0..8 {
            // tweak_constants is what initialize_hash_function calls; compare the
            // entire resulting ctx (1000+ bytes of round constants).
            let ps = rng.bytes(SPX_N);
            let ss = rng.bytes(SPX_N);
            let mut cb = new_ctx_buf();
            let mut rb = new_ctx_buf();
            cb.set_seeds(&ps, &ss);
            rb.set_seeds(&ps, &ss);
            unsafe { tc(cb.as_mut_ptr()) };
            unsafe { tr(rb.as_mut_ptr()) };
            eq_bytes("SPX_tweak_constants ctx image", cb.bytes(), rb.bytes());

            // haraka256 / haraka512 / haraka512_perm
            for (name, inlen, outlen) in [
                ("SPX_haraka256", 32usize, 32usize),
                ("SPX_haraka512", 64, 32),
                ("SPX_haraka512_perm", 64, 64),
            ] {
                let fc: Symbol<FBlk> = p.c.sym(name);
                let fr: Symbol<FBlk> = p.r.sym(name);
                for it in 0..8 {
                    let input = match it {
                        0 => vec![0u8; inlen],
                        1 => vec![0xffu8; inlen],
                        _ => rng.bytes(inlen),
                    };
                    let mut co = vec![0xAAu8; outlen + 16];
                    let mut ro = vec![0xAAu8; outlen + 16];
                    unsafe { fc(co.as_mut_ptr(), input.as_ptr(), cb.as_ptr()) };
                    unsafe { fr(ro.as_mut_ptr(), input.as_ptr(), rb.as_ptr()) };
                    eq_bytes(name, &co, &ro);
                }
            }

            // haraka_S (the sponge) across the rate boundary
            let fc: Symbol<FS> = p.c.sym("SPX_haraka_S");
            let fr: Symbol<FS> = p.r.sym("SPX_haraka_S");
            for &inlen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 1000] {
                let data = rng.bytes(inlen);
                for &outlen in &[0usize, 1, 32, 33, 64, 100, 640, 1000] {
                    let mut co = vec![0xAAu8; outlen + 16];
                    let mut ro = vec![0xAAu8; outlen + 16];
                    unsafe {
                        fc(
                            co.as_mut_ptr(),
                            outlen as u64,
                            data.as_ptr(),
                            inlen as u64,
                            cb.as_ptr(),
                        )
                    };
                    unsafe {
                        fr(
                            ro.as_mut_ptr(),
                            outlen as u64,
                            data.as_ptr(),
                            inlen as u64,
                            rb.as_ptr(),
                        )
                    };
                    eq_bytes(&format!("haraka_S(in={inlen}, out={outlen})"), &co, &ro);
                }
            }

            // the incremental sponge API: s_inc is uint8_t[65]
            let iic: Symbol<FIncInit> = p.c.sym("SPX_haraka_S_inc_init");
            let iir: Symbol<FIncInit> = p.r.sym("SPX_haraka_S_inc_init");
            let iac: Symbol<FIncAbs> = p.c.sym("SPX_haraka_S_inc_absorb");
            let iar: Symbol<FIncAbs> = p.r.sym("SPX_haraka_S_inc_absorb");
            let ifc: Symbol<FIncFin> = p.c.sym("SPX_haraka_S_inc_finalize");
            let ifr: Symbol<FIncFin> = p.r.sym("SPX_haraka_S_inc_finalize");
            let isc: Symbol<FIncSqz> = p.c.sym("SPX_haraka_S_inc_squeeze");
            let isr: Symbol<FIncSqz> = p.r.sym("SPX_haraka_S_inc_squeeze");

            for &inlen in &[0usize, 1, 31, 32, 33, 100] {
                let data = rng.bytes(inlen);
                let mut cs = [0xAAu8; 65];
                let mut rs = [0xAAu8; 65];
                unsafe { iic(cs.as_mut_ptr()) };
                unsafe { iir(rs.as_mut_ptr()) };
                eq_bytes("haraka_S_inc_init state", &cs, &rs);
                let mut off = 0usize;
                while off < inlen {
                    let chunk = (rng.below(40) as usize + 1).min(inlen - off);
                    unsafe { iac(cs.as_mut_ptr(), data[off..].as_ptr(), chunk, cb.as_ptr()) };
                    unsafe { iar(rs.as_mut_ptr(), data[off..].as_ptr(), chunk, rb.as_ptr()) };
                    eq_bytes("haraka_S_inc_absorb state", &cs, &rs);
                    off += chunk;
                }
                unsafe { ifc(cs.as_mut_ptr()) };
                unsafe { ifr(rs.as_mut_ptr()) };
                eq_bytes("haraka_S_inc_finalize state", &cs, &rs);
                for &outlen in &[0usize, 1, 32, 33, 70] {
                    let mut co = vec![0xAAu8; outlen + 16];
                    let mut ro = vec![0xAAu8; outlen + 16];
                    unsafe { isc(co.as_mut_ptr(), outlen, cs.as_mut_ptr(), cb.as_ptr()) };
                    unsafe { isr(ro.as_mut_ptr(), outlen, rs.as_mut_ptr(), rb.as_ptr()) };
                    eq_bytes(&format!("haraka_S_inc_squeeze({outlen})"), &co, &ro);
                    eq_bytes("haraka_S_inc_squeeze state", &cs, &rs);
                }
            }
        }
    }
}
