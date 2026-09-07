//! Phase B rows 36-43: OTS/tree internals -- `wots.c`, `wotsx1.c`, `fors.c`
//! (`fors_gen_leafx1`), `utils.c` (`compute_root`, `treehash`) and
//! `utilsx1.c` (`wots_treehashx1`, `fors_treehashx1`).
mod common;
use common::*;

type FnInitHash = unsafe extern "C" fn(*mut SpxCtxFfi);
type FnWotsPkFromSig =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const SpxCtxFfi, *mut u32);
type FnWotsGenLeafX1 =
    unsafe extern "C" fn(*mut u8, *const SpxCtxFfi, u32, *mut LeafInfoX1Ffi);
type FnForsGenLeafX1 =
    unsafe extern "C" fn(*mut u8, *const SpxCtxFfi, u32, *mut ForsGenLeafInfoFfi);
type FnComputeRoot = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    u32,
    u32,
    *const u8,
    u32,
    *const SpxCtxFfi,
    *mut u32,
);
/// `void (*gen_leaf)(unsigned char*, const spx_ctx*, uint32_t, const uint32_t[8])`
type GenLeafFn = unsafe extern "C" fn(*mut u8, *const SpxCtxFfi, u32, *const u32);
type FnTreehash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const SpxCtxFfi,
    u32,
    u32,
    u32,
    Option<GenLeafFn>,
    *mut u32,
);
type FnForsTreehashX1 = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const SpxCtxFfi,
    u32,
    u32,
    u32,
    *mut u32,
    *mut ForsGenLeafInfoFfi,
);
type FnTreehashX1 = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const SpxCtxFfi,
    u32,
    u32,
    u32,
    *mut u32,
    *mut LeafInfoX1Ffi,
);

fn make_ctx(rng: &mut Rng) -> (SpxCtxFfi, SpxCtxFfi) {
    let l = libs();
    let (c, r) = l.pair::<FnInitHash>("SPX_initialize_hash_function");
    let mut cc = SpxCtxFfi::zeroed();
    rng.fill(&mut cc.pub_seed);
    rng.fill(&mut cc.sk_seed);
    let mut rc = cc.clone();
    unsafe {
        c(&mut cc);
        r(&mut rc);
    }
    assert_bytes_eq("spx_ctx after init", cc.as_bytes(), rc.as_bytes());
    (cc, rc)
}

/// Build a `leaf_info_x1` with ALL bytes (including the 4 padding bytes the C
/// struct has at offset 20 on x86-64) deterministically zeroed first, so that
/// both libraries observe byte-identical input.
fn new_leaf_info(
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *const u32,
    addr: [u32; 8],
) -> LeafInfoX1Ffi {
    let mut info: LeafInfoX1Ffi = unsafe { core::mem::zeroed() };
    info.wots_sig = wots_sig;
    info.wots_sign_leaf = wots_sign_leaf;
    info.wots_steps = wots_steps;
    info.leaf_addr = addr;
    info.pk_addr = addr;
    info
}

// --- row 36 -------------------------------------------------------------

#[test]
fn row36_wots_pk_from_sig() {
    let l = libs();
    let (c, r) = l.pair::<FnWotsPkFromSig>("SPX_wots_pk_from_sig");
    let mut rng = Rng::new(0x3601);
    for i in 0..96 {
        let (cc, rc) = make_ctx(&mut rng);
        let sig = rng.bytes(SPX_WOTS_BYTES);
        // Cover the checksum/chain-step extremes as well as random messages.
        let msg = match i {
            0 => vec![0x00u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            2 => vec![0x0Fu8; SPX_N],
            3 => vec![0xF0u8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut cpk = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let mut rpk = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        unsafe {
            c(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), &cc, ca.as_mut_ptr());
            r(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), &rc, ra.as_mut_ptr());
        }
        assert_bytes_eq(&format!("SPX_wots_pk_from_sig #{i} pk"), &cpk, &rpk);
        assert_bytes_eq(
            &format!("SPX_wots_pk_from_sig #{i} addr after"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }
}

// --- rows 37 / 38 -------------------------------------------------------

fn wots_gen_leafx1_case(sign_mode: bool, iters: usize, seed: u64) {
    let l = libs();
    let (c, r) = l.pair::<FnWotsGenLeafX1>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(seed);
    for it in 0..iters {
        let (cc, rc) = make_ctx(&mut rng);
        let leaf_idx = if sign_mode {
            match it {
                0 => 0u32,
                1 => 1,
                2 => (1u32 << SPX_TREE_HEIGHT) - 1,
                _ => rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1),
            }
        } else {
            rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1)
        };
        // `wots_steps` must hold SPX_WOTS_LEN entries with values < w
        // (that is what chain_lengths produces).
        let steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| (rng.next_u32() % SPX_WOTS_W as u32))
            .collect();
        let base_addr = rng.addr();

        let mut c_sig = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let mut r_sig = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let sign_leaf = if sign_mode { leaf_idx } else { u32::MAX };
        let mut c_info = new_leaf_info(c_sig.as_mut_ptr(), sign_leaf, steps.as_ptr(), base_addr);
        let mut r_info = new_leaf_info(r_sig.as_mut_ptr(), sign_leaf, steps.as_ptr(), base_addr);

        let mut c_dest = vec![0xA5u8; SPX_N + 8];
        let mut r_dest = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(c_dest.as_mut_ptr(), &cc, leaf_idx, &mut c_info);
            r(r_dest.as_mut_ptr(), &rc, leaf_idx, &mut r_info);
        }
        let m = if sign_mode { "sign" } else { "pk-only" };
        assert_bytes_eq(&format!("SPX_wots_gen_leafx1 [{m}] dest"), &c_dest, &r_dest);
        assert_bytes_eq(&format!("SPX_wots_gen_leafx1 [{m}] wots_sig"), &c_sig, &r_sig);
        assert_bytes_eq(
            &format!("SPX_wots_gen_leafx1 [{m}] leaf_addr after"),
            &addr_bytes(&c_info.leaf_addr),
            &addr_bytes(&r_info.leaf_addr),
        );
        assert_bytes_eq(
            &format!("SPX_wots_gen_leafx1 [{m}] pk_addr after"),
            &addr_bytes(&c_info.pk_addr),
            &addr_bytes(&r_info.pk_addr),
        );
        assert_eq_dbg(
            &format!("SPX_wots_gen_leafx1 [{m}] wots_sign_leaf after"),
            c_info.wots_sign_leaf,
            r_info.wots_sign_leaf,
        );
        if !sign_mode {
            // pk-only mode must leave wots_sig completely untouched.
            assert!(
                c_sig.iter().all(|&b| b == 0xA5),
                "{} C wrote wots_sig in pk-only mode",
                tag()
            );
            assert!(
                r_sig.iter().all(|&b| b == 0xA5),
                "{} Rust wrote wots_sig in pk-only mode",
                tag()
            );
        }
    }
}

#[test]
fn row37_wots_gen_leafx1_sign_mode() {
    wots_gen_leafx1_case(true, 64, 0x3701);
}

#[test]
fn row38_wots_gen_leafx1_pk_only_mode() {
    wots_gen_leafx1_case(false, 64, 0x3801);
}

// --- row 39 -------------------------------------------------------------

#[test]
fn row39_fors_gen_leafx1() {
    let l = libs();
    let (c, r) = l.pair::<FnForsGenLeafX1>("SPX_fors_gen_leafx1");
    let mut rng = Rng::new(0x3901);
    for it in 0..128 {
        let (cc, rc) = make_ctx(&mut rng);
        let addr_idx = match it {
            0 => 0u32,
            1 => 1,
            2 => (1u32 << SPX_FORS_HEIGHT) - 1,
            3 => u32::MAX,
            _ => rng.next_u32(),
        };
        let base = rng.addr();
        let mut ci = ForsGenLeafInfoFfi { leaf_addrx: base };
        let mut ri = ForsGenLeafInfoFfi { leaf_addrx: base };
        let mut cl = vec![0xA5u8; SPX_N + 8];
        let mut rl = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(cl.as_mut_ptr(), &cc, addr_idx, &mut ci);
            r(rl.as_mut_ptr(), &rc, addr_idx, &mut ri);
        }
        assert_bytes_eq(&format!("SPX_fors_gen_leafx1(addr_idx={addr_idx}) leaf"), &cl, &rl);
        assert_bytes_eq(
            &format!("SPX_fors_gen_leafx1(addr_idx={addr_idx}) info after"),
            &addr_bytes(&ci.leaf_addrx),
            &addr_bytes(&ri.leaf_addrx),
        );
    }
}

// --- row 40 -------------------------------------------------------------

#[test]
fn row40_compute_root() {
    let l = libs();
    let (c, r) = l.pair::<FnComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(0x4001);
    // tree_height == 0 is excluded: the C's `for (i = 0; i < tree_height-1; i++)`
    // underflows a uint32_t and reads far out of bounds (see ERRORS.md row 27).
    let mut heights = vec![1u32, 2, SPX_FORS_HEIGHT as u32, SPX_TREE_HEIGHT as u32];
    heights.sort_unstable();
    heights.dedup();
    for &h in &heights {
        for it in 0..48 {
            let (cc, rc) = make_ctx(&mut rng);
            let leaf = rng.bytes(SPX_N);
            let auth = rng.bytes(h as usize * SPX_N);
            let max = if h >= 32 { u32::MAX } else { (1u32 << h) - 1 };
            let leaf_idx = match it {
                0 => 0u32,
                1 => 1,
                2 => max,
                3 => max.wrapping_sub(1),
                4 => 2,
                _ => rng.next_u32() & max,
            };
            let idx_offset = match it {
                0 | 1 | 2 => 0u32,
                _ => rng.next_u32(),
            };
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut cr = vec![0xA5u8; SPX_N + 8];
            let mut rr = vec![0xA5u8; SPX_N + 8];
            unsafe {
                c(
                    cr.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    &cc,
                    ca.as_mut_ptr(),
                );
                r(
                    rr.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    &rc,
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(
                &format!("SPX_compute_root(h={h}, leaf_idx={leaf_idx}, off={idx_offset}) root"),
                &cr,
                &rr,
            );
            assert_bytes_eq(
                &format!("SPX_compute_root(h={h}) addr after"),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

// --- row 41 -------------------------------------------------------------

#[test]
fn row41_treehash_with_fn_pointer() {
    // `SPX_treehash` takes a `gen_leaf` FUNCTION POINTER. Each side is given
    // the gen_leaf from ITS OWN library, so the whole composed pipeline (the
    // callback + the thash chain inside treehash) is exercised.
    let l = libs();
    let (ct, rt) = l.pair::<FnTreehash>("SPX_treehash");
    let c_leaf = l.sym::<GenLeafFn>(Which::C, "SPX_fors_gen_leafx1");
    let r_leaf = l.sym::<GenLeafFn>(Which::R, "SPX_fors_gen_leafx1");
    let mut rng = Rng::new(0x4101);
    for h in [0u32, 1, 2, 3, 4] {
        for it in 0..24 {
            let (cc, rc) = make_ctx(&mut rng);
            let max = (1u32 << h) - 1;
            let leaf_idx = match it {
                0 => 0u32,
                1 => 1.min(max),
                2 => max,
                _ => rng.next_u32() & max,
            };
            let idx_offset = match it {
                0 | 1 | 2 => 0u32,
                _ => rng.next_u32() & 0xFFFF,
            };
            let addr = rng.addr();
            let mut ca = addr;
            let mut ra = addr;
            let mut croot = vec![0xA5u8; SPX_N + 8];
            let mut rroot = vec![0xA5u8; SPX_N + 8];
            let mut cauth = vec![0xA5u8; (h as usize + 1) * SPX_N + 8];
            let mut rauth = vec![0xA5u8; (h as usize + 1) * SPX_N + 8];
            unsafe {
                ct(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    &cc,
                    leaf_idx,
                    idx_offset,
                    h,
                    Some(*c_leaf),
                    ca.as_mut_ptr(),
                );
                rt(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    &rc,
                    leaf_idx,
                    idx_offset,
                    h,
                    Some(*r_leaf),
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(
                &format!("SPX_treehash(h={h}, leaf_idx={leaf_idx}, off={idx_offset}) root"),
                &croot,
                &rroot,
            );
            assert_bytes_eq(
                &format!("SPX_treehash(h={h}, leaf_idx={leaf_idx}, off={idx_offset}) auth_path"),
                &cauth,
                &rauth,
            );
            assert_bytes_eq(
                &format!("SPX_treehash(h={h}) addr after"),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

// --- row 42 -------------------------------------------------------------

#[test]
fn row42_wots_treehashx1() {
    let l = libs();
    let (c, r) = l.pair::<FnTreehashX1>("SPX_wots_treehashx1");
    let mut rng = Rng::new(0x4201);
    let h = SPX_TREE_HEIGHT as u32;
    let max = (1u32 << h) - 1;
    for it in 0..24 {
        let (cc, rc) = make_ctx(&mut rng);
        let idx_leaf = match it {
            0 => 0u32,
            1 => 1,
            2 => max,
            3 => u32::MAX, // the merkle_gen_root sentinel ("no auth path")
            _ => rng.next_u32() & max,
        };
        let steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| (rng.next_u32() % SPX_WOTS_W as u32))
            .collect();
        let base = rng.addr();
        let tree_addr = rng.addr();

        let mut c_sig = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let mut r_sig = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let mut c_info = new_leaf_info(c_sig.as_mut_ptr(), idx_leaf, steps.as_ptr(), base);
        let mut r_info = new_leaf_info(r_sig.as_mut_ptr(), idx_leaf, steps.as_ptr(), base);
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        let mut croot = vec![0xA5u8; SPX_N + 8];
        let mut rroot = vec![0xA5u8; SPX_N + 8];
        let mut cauth = vec![0xA5u8; h as usize * SPX_N + 8];
        let mut rauth = vec![0xA5u8; h as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                &cc,
                idx_leaf,
                0,
                h,
                cta.as_mut_ptr(),
                &mut c_info,
            );
            r(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                &rc,
                idx_leaf,
                0,
                h,
                rta.as_mut_ptr(),
                &mut r_info,
            );
        }
        assert_bytes_eq(&format!("SPX_wots_treehashx1(idx_leaf={idx_leaf}) root"), &croot, &rroot);
        assert_bytes_eq(
            &format!("SPX_wots_treehashx1(idx_leaf={idx_leaf}) auth_path"),
            &cauth,
            &rauth,
        );
        assert_bytes_eq(
            &format!("SPX_wots_treehashx1(idx_leaf={idx_leaf}) wots_sig"),
            &c_sig,
            &r_sig,
        );
        assert_bytes_eq(
            &format!("SPX_wots_treehashx1(idx_leaf={idx_leaf}) tree_addr after"),
            &addr_bytes(&cta),
            &addr_bytes(&rta),
        );
        assert_bytes_eq(
            &format!("SPX_wots_treehashx1(idx_leaf={idx_leaf}) leaf_addr after"),
            &addr_bytes(&c_info.leaf_addr),
            &addr_bytes(&r_info.leaf_addr),
        );
        assert_bytes_eq(
            &format!("SPX_wots_treehashx1(idx_leaf={idx_leaf}) pk_addr after"),
            &addr_bytes(&c_info.pk_addr),
            &addr_bytes(&r_info.pk_addr),
        );
    }
}

// --- row 43 -------------------------------------------------------------

#[test]
fn row43_fors_treehashx1() {
    // `fors_treehashx1` has the same signature as `wots_treehashx1` but calls
    // `fors_gen_leafx1`, which only reads `info->leaf_addr` (aliasing
    // `fors_gen_leaf_info.leaf_addrx`).
    let l = libs();
    let (c, r) = l.pair::<FnForsTreehashX1>("SPX_fors_treehashx1");
    let mut rng = Rng::new(0x4301);
    let h = SPX_FORS_HEIGHT as u32;
    let max = (1u32 << h) - 1;
    for it in 0..24 {
        let (cc, rc) = make_ctx(&mut rng);
        let leaf_idx = match it {
            0 => 0u32,
            1 => 1,
            2 => max,
            _ => rng.next_u32() & max,
        };
        let idx_offset = match it {
            0 | 1 | 2 => 0u32,
            3 => (it as u32) * (1u32 << h),
            _ => (rng.next_u32() % SPX_FORS_TREES as u32) * (1u32 << h),
        };
        let base = rng.addr();
        let tree_addr = rng.addr();

        // NOTE: `fors_treehashx1`'s prototype says `leaf_info_x1 *info`, but it
        // forwards that pointer to `fors_gen_leafx1`, which declares
        // `fors_gen_leaf_info *` -- the C compiler even warns about it. So the
        // C reads the FIRST 32 bytes of whatever is passed, and every real
        // caller (`fors.c: fors_sign`) passes a `fors_gen_leaf_info`. Pass that
        // type here, matching the real pipeline (and avoiding the
        // indeterminate padding bytes of `leaf_info_x1`).
        let mut c_info = ForsGenLeafInfoFfi { leaf_addrx: base };
        let mut r_info = ForsGenLeafInfoFfi { leaf_addrx: base };
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        let mut croot = vec![0xA5u8; SPX_N + 8];
        let mut rroot = vec![0xA5u8; SPX_N + 8];
        let mut cauth = vec![0xA5u8; h as usize * SPX_N + 8];
        let mut rauth = vec![0xA5u8; h as usize * SPX_N + 8];
        unsafe {
            c(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                &cc,
                leaf_idx,
                idx_offset,
                h,
                cta.as_mut_ptr(),
                &mut c_info,
            );
            r(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                &rc,
                leaf_idx,
                idx_offset,
                h,
                rta.as_mut_ptr(),
                &mut r_info,
            );
        }
        assert_bytes_eq(
            &format!("SPX_fors_treehashx1(leaf_idx={leaf_idx}, off={idx_offset}) root"),
            &croot,
            &rroot,
        );
        assert_bytes_eq(
            &format!("SPX_fors_treehashx1(leaf_idx={leaf_idx}, off={idx_offset}) auth_path"),
            &cauth,
            &rauth,
        );
        assert_bytes_eq(
            &format!("SPX_fors_treehashx1 tree_addr after"),
            &addr_bytes(&cta),
            &addr_bytes(&rta),
        );
        assert_bytes_eq(
            &format!("SPX_fors_treehashx1 leaf_addr after"),
            &addr_bytes(&c_info.leaf_addrx),
            &addr_bytes(&r_info.leaf_addrx),
        );
    }
}
