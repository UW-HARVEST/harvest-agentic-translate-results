//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, bottom-up: address primitives, byte
//! utilities, `thash`, the `hash.h` entry points, the backend primitives, WOTS,
//! FORS, Merkle/treehash, and finally the `api.h` top level.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols and compares outputs byte-for-byte over `ITERS` pseudo-random
//! inputs from a fixed seed.

mod common;
use common::*;

/* ================================================================== */
/* Rows 1-11 — address primitives (app/src/address.c)                 */
/* ================================================================== */

/// `copy_subtree_addr` / `copy_keypair_addr` memcpy `SPX_OFFSET_TREE + 8`
/// bytes, which is 40 for the 32-byte address layouts — i.e. *past* the end of
/// `uint32_t addr[8]`. The C does this deliberately, so both implementations
/// get an oversized buffer and the whole buffer is compared.
const ADDR_SLOTS: usize = 16;

fn addr_buf(rng: &mut Rng) -> [u32; ADDR_SLOTS] {
    let mut a = [0u32; ADDR_SLOTS];
    for x in a.iter_mut() {
        *x = rng.next_u32();
    }
    a
}

macro_rules! set_u32_row {
    ($name:ident, $row:expr, $sym:expr) => {
        #[test]
        fn $name() {
            let p = libs();
            let c: FnSetU32 = p.c.f($sym);
            let r: FnSetU32 = p.r.f($sym);
            let mut rng = Rng::for_row($row);
            for i in 0..ITERS {
                let base = addr_buf(&mut rng);
                // Mix small in-range values with full-width u32 values.
                let v = if i % 4 == 0 { i as u32 } else { rng.next_u32() };
                let mut ac = base;
                let mut ar = base;
                unsafe {
                    c(ac.as_mut_ptr(), v);
                    r(ar.as_mut_ptr(), v);
                }
                eq_u32s($sym, i, &ac, &ar);
            }
        }
    };
}

set_u32_row!(cfg_01_set_layer_addr, 1, "SPX_set_layer_addr");
set_u32_row!(cfg_05_set_keypair_addr, 5, "SPX_set_keypair_addr");
set_u32_row!(cfg_07_set_chain_addr, 7, "SPX_set_chain_addr");
set_u32_row!(cfg_08_set_hash_addr, 8, "SPX_set_hash_addr");
set_u32_row!(cfg_09_set_tree_height, 9, "SPX_set_tree_height");
set_u32_row!(cfg_10_set_tree_index, 10, "SPX_set_tree_index");

#[test]
fn cfg_02_set_tree_addr() {
    let p = libs();
    let c: FnSetU64 = p.c.f("SPX_set_tree_addr");
    let r: FnSetU64 = p.r.f("SPX_set_tree_addr");
    let mut rng = Rng::for_row(2);
    for i in 0..ITERS {
        let base = addr_buf(&mut rng);
        let v = match i % 4 {
            0 => 0u64,
            1 => u64::MAX,
            2 => 1u64 << (SPX_TREE_BITS.min(63)),
            _ => rng.next_u64(),
        };
        let mut ac = base;
        let mut ar = base;
        unsafe {
            c(ac.as_mut_ptr(), v);
            r(ar.as_mut_ptr(), v);
        }
        eq_u32s("SPX_set_tree_addr", i, &ac, &ar);
    }
}

#[test]
fn cfg_03_set_type_all_valid_types() {
    let p = libs();
    let c: FnSetU32 = p.c.f("SPX_set_type");
    let r: FnSetU32 = p.r.f("SPX_set_type");
    let mut rng = Rng::for_row(3);
    let types = [
        SPX_ADDR_TYPE_WOTS,
        SPX_ADDR_TYPE_WOTSPK,
        SPX_ADDR_TYPE_HASHTREE,
        SPX_ADDR_TYPE_FORSTREE,
        SPX_ADDR_TYPE_FORSPK,
        SPX_ADDR_TYPE_WOTSPRF,
        SPX_ADDR_TYPE_FORSPRF,
    ];
    for i in 0..ITERS {
        let base = addr_buf(&mut rng);
        for &t in types.iter() {
            let mut ac = base;
            let mut ar = base;
            unsafe {
                c(ac.as_mut_ptr(), t);
                r(ar.as_mut_ptr(), t);
            }
            eq_u32s("SPX_set_type", i, &ac, &ar);
        }
    }
}

#[test]
fn cfg_04_copy_subtree_addr() {
    let p = libs();
    let c: FnCopyAddr = p.c.f("SPX_copy_subtree_addr");
    let r: FnCopyAddr = p.r.f("SPX_copy_subtree_addr");
    let mut rng = Rng::for_row(4);
    for i in 0..ITERS {
        let src = addr_buf(&mut rng);
        let dst = addr_buf(&mut rng);
        let mut oc = dst;
        let mut or = dst;
        unsafe {
            c(oc.as_mut_ptr(), src.as_ptr());
            r(or.as_mut_ptr(), src.as_ptr());
        }
        eq_u32s("SPX_copy_subtree_addr", i, &oc, &or);
    }
}

#[test]
fn cfg_06_copy_keypair_addr() {
    let p = libs();
    let c: FnCopyAddr = p.c.f("SPX_copy_keypair_addr");
    let r: FnCopyAddr = p.r.f("SPX_copy_keypair_addr");
    let mut rng = Rng::for_row(6);
    for i in 0..ITERS {
        let src = addr_buf(&mut rng);
        let dst = addr_buf(&mut rng);
        let mut oc = dst;
        let mut or = dst;
        unsafe {
            c(oc.as_mut_ptr(), src.as_ptr());
            r(or.as_mut_ptr(), src.as_ptr());
        }
        eq_u32s("SPX_copy_keypair_addr", i, &oc, &or);
    }
}

/// Row 11: the composed `set_*` sequence that `fors_sign` / `merkle_sign` /
/// `wots_gen_leafx1` apply, comparing after every step. This catches
/// offset-aliasing (`CHAIN_ADDR` and `TREE_HGT` share a byte; `HASH_ADDR` sits
/// inside the `TREE_INDEX` field).
#[test]
fn cfg_11_composed_address_sequence() {
    let p = libs();
    let sl_c: FnSetU32 = p.c.f("SPX_set_layer_addr");
    let sl_r: FnSetU32 = p.r.f("SPX_set_layer_addr");
    let st_c: FnSetU64 = p.c.f("SPX_set_tree_addr");
    let st_r: FnSetU64 = p.r.f("SPX_set_tree_addr");
    let ty_c: FnSetU32 = p.c.f("SPX_set_type");
    let ty_r: FnSetU32 = p.r.f("SPX_set_type");
    let kp_c: FnSetU32 = p.c.f("SPX_set_keypair_addr");
    let kp_r: FnSetU32 = p.r.f("SPX_set_keypair_addr");
    let ch_c: FnSetU32 = p.c.f("SPX_set_chain_addr");
    let ch_r: FnSetU32 = p.r.f("SPX_set_chain_addr");
    let ha_c: FnSetU32 = p.c.f("SPX_set_hash_addr");
    let ha_r: FnSetU32 = p.r.f("SPX_set_hash_addr");
    let th_c: FnSetU32 = p.c.f("SPX_set_tree_height");
    let th_r: FnSetU32 = p.r.f("SPX_set_tree_height");
    let ti_c: FnSetU32 = p.c.f("SPX_set_tree_index");
    let ti_r: FnSetU32 = p.r.f("SPX_set_tree_index");
    let cs_c: FnCopyAddr = p.c.f("SPX_copy_subtree_addr");
    let cs_r: FnCopyAddr = p.r.f("SPX_copy_subtree_addr");
    let ck_c: FnCopyAddr = p.c.f("SPX_copy_keypair_addr");
    let ck_r: FnCopyAddr = p.r.f("SPX_copy_keypair_addr");

    let mut rng = Rng::for_row(11);
    for i in 0..ITERS {
        let mut ac = addr_buf(&mut rng);
        let mut ar = ac;
        let mut bc = addr_buf(&mut rng);
        let mut br = bc;

        let layer = rng.below(SPX_D);
        let tree = rng.next_u64();
        let kp = rng.next_u32();
        let chain = rng.below(SPX_WOTS_LEN as u32);
        let hash = rng.below(SPX_WOTS_W);
        let hgt = rng.below(SPX_TREE_HEIGHT + 1);
        let idx = rng.next_u32();

        macro_rules! step {
            ($lbl:expr, $cf:expr, $rf:expr, $($a:expr),*) => {{
                unsafe { $cf(ac.as_mut_ptr(), $($a),*); $rf(ar.as_mut_ptr(), $($a),*); }
                eq_u32s(concat!("composed/", $lbl), i, &ac, &ar);
            }};
        }

        step!("set_layer", sl_c, sl_r, layer);
        step!("set_tree", st_c, st_r, tree);
        step!("set_type=WOTS", ty_c, ty_r, SPX_ADDR_TYPE_WOTS);
        step!("set_keypair", kp_c, kp_r, kp);
        step!("set_chain", ch_c, ch_r, chain);
        step!("set_hash", ha_c, ha_r, hash);
        step!("set_type=WOTSPRF", ty_c, ty_r, SPX_ADDR_TYPE_WOTSPRF);
        step!("set_tree_height", th_c, th_r, hgt);
        step!("set_tree_index", ti_c, ti_r, idx);

        unsafe {
            cs_c(bc.as_mut_ptr(), ac.as_ptr());
            cs_r(br.as_mut_ptr(), ar.as_ptr());
        }
        eq_u32s("composed/copy_subtree", i, &bc, &br);
        unsafe {
            ck_c(bc.as_mut_ptr(), ac.as_ptr());
            ck_r(br.as_mut_ptr(), ar.as_ptr());
        }
        eq_u32s("composed/copy_keypair", i, &bc, &br);
    }
}

/* ================================================================== */
/* Rows 12-15 — byte / integer utilities (app/src/utils.c)            */
/* ================================================================== */

#[test]
fn cfg_12_ull_to_bytes() {
    let p = libs();
    let c: FnUllToBytes = p.c.f("SPX_ull_to_bytes");
    let r: FnUllToBytes = p.r.f("SPX_ull_to_bytes");
    let mut rng = Rng::for_row(12);
    for i in 0..ITERS {
        for &outlen in &[1usize, 2, 3, 4, 8] {
            let v = match i % 4 {
                0 => 0u64,
                1 => u64::MAX,
                2 => rng.next_u64() & 0xFF,
                _ => rng.next_u64(),
            };
            // Pre-fill with a pattern so "wrote nothing" is detectable.
            let mut bc = vec![0xA5u8; outlen + 8];
            let mut br = bc.clone();
            unsafe {
                c(bc.as_mut_ptr(), outlen as core::ffi::c_uint, v);
                r(br.as_mut_ptr(), outlen as core::ffi::c_uint, v);
            }
            eq_bytes("SPX_ull_to_bytes", i, &bc, &br);
        }
    }
}

#[test]
fn cfg_13_u32_to_bytes() {
    let p = libs();
    let c: FnU32ToBytes = p.c.f("SPX_u32_to_bytes");
    let r: FnU32ToBytes = p.r.f("SPX_u32_to_bytes");
    let mut rng = Rng::for_row(13);
    for i in 0..ITERS {
        let v = match i % 4 {
            0 => 0u32,
            1 => u32::MAX,
            2 => 0x0000_0100,
            _ => rng.next_u32(),
        };
        let mut bc = [0xA5u8; 12];
        let mut br = bc;
        unsafe {
            c(bc.as_mut_ptr(), v);
            r(br.as_mut_ptr(), v);
        }
        eq_bytes("SPX_u32_to_bytes", i, &bc, &br);
    }
}

#[test]
fn cfg_14_bytes_to_ull() {
    let p = libs();
    let c: FnBytesToUll = p.c.f("SPX_bytes_to_ull");
    let r: FnBytesToUll = p.r.f("SPX_bytes_to_ull");
    let mut rng = Rng::for_row(14);
    for i in 0..ITERS {
        let buf = rng.bytes(16);
        for &inlen in &[1usize, 2, 3, 4, 8] {
            let (a, b) = unsafe {
                (
                    c(buf.as_ptr(), inlen as core::ffi::c_uint),
                    r(buf.as_ptr(), inlen as core::ffi::c_uint),
                )
            };
            eq("SPX_bytes_to_ull", i, a, b);
        }
    }
}

#[test]
fn cfg_15_ull_roundtrip() {
    let p = libs();
    let uc: FnUllToBytes = p.c.f("SPX_ull_to_bytes");
    let ur: FnUllToBytes = p.r.f("SPX_ull_to_bytes");
    let bc_: FnBytesToUll = p.c.f("SPX_bytes_to_ull");
    let br_: FnBytesToUll = p.r.f("SPX_bytes_to_ull");
    let mut rng = Rng::for_row(15);
    for i in 0..ITERS {
        for &n in &[1usize, 2, 4, 8] {
            let v = rng.next_u64() >> (64 - 8 * n as u32);
            let mut b1 = vec![0u8; n];
            let mut b2 = vec![0u8; n];
            unsafe {
                uc(b1.as_mut_ptr(), n as core::ffi::c_uint, v);
                ur(b2.as_mut_ptr(), n as core::ffi::c_uint, v);
                eq_bytes("roundtrip/encode", i, &b1, &b2);
                eq(
                    "roundtrip/decode",
                    i,
                    bc_(b1.as_ptr(), n as core::ffi::c_uint),
                    br_(b2.as_ptr(), n as core::ffi::c_uint),
                );
                eq(
                    "roundtrip/value",
                    i,
                    bc_(b1.as_ptr(), n as core::ffi::c_uint),
                    v,
                );
            }
        }
    }
}

/* ================================================================== */
/* Rows 16-20 — thash (backend x THASH x inblocks)                    */
/* ================================================================== */

/// Build two byte-identical contexts (one per implementation) by running each
/// library's own `initialize_hash_function` over the same seeds — the way a
/// real consumer sets up state.
fn init_ctx_pair(rng: &mut Rng) -> (Ctx, Ctx) {
    let p = libs();
    let ic: FnInitHash = p.c.f("SPX_initialize_hash_function");
    let ir: FnInitHash = p.r.f("SPX_initialize_hash_function");
    let pub_seed = rng.bytes(SPX_N);
    let sk_seed = rng.bytes(SPX_N);
    let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
    let mut cr = Ctx::with_seeds(&pub_seed, &sk_seed);
    unsafe {
        ic(cc.as_mut_ptr());
        ir(cr.as_mut_ptr());
    }
    (cc, cr)
}

fn thash_row(row: u64, blocks: usize, label: &str) {
    let p = libs();
    let c: FnThash = p.c.f("SPX_thash");
    let r: FnThash = p.r.f("SPX_thash");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        eq_bytes("thash/ctx-setup", i, &cc.0, &cr.0);
        let input = rng.bytes(blocks * SPX_N + 8);
        let addr = addr_buf(&mut rng);
        let mut oc = vec![0xA5u8; SPX_N + 8];
        let mut or = oc.clone();
        let mut ac = addr;
        let mut ar = addr;
        unsafe {
            c(
                oc.as_mut_ptr(),
                input.as_ptr(),
                blocks as core::ffi::c_uint,
                cc.as_ptr(),
                ac.as_mut_ptr(),
            );
            r(
                or.as_mut_ptr(),
                input.as_ptr(),
                blocks as core::ffi::c_uint,
                cr.as_ptr(),
                ar.as_mut_ptr(),
            );
        }
        eq_bytes(label, i, &oc, &or);
        eq_u32s("thash/addr-mutation", i, &ac, &ar);
    }
}

#[test]
fn cfg_16_thash_inblocks_1() {
    thash_row(16, 1, "SPX_thash(inblocks=1)");
}

#[test]
fn cfg_17_thash_inblocks_2() {
    thash_row(17, 2, "SPX_thash(inblocks=2)");
}

#[test]
fn cfg_18_thash_inblocks_wots_len() {
    thash_row(18, SPX_WOTS_LEN, "SPX_thash(inblocks=SPX_WOTS_LEN)");
}

#[test]
fn cfg_19_thash_inblocks_fors_trees() {
    thash_row(19, SPX_FORS_TREES as usize, "SPX_thash(inblocks=SPX_FORS_TREES)");
}

#[test]
fn cfg_20_thash_inblocks_3() {
    thash_row(20, 3, "SPX_thash(inblocks=3)");
}

/* ================================================================== */
/* Rows 21-31 — hash.h backend entry points                           */
/* ================================================================== */

#[test]
fn cfg_21_initialize_hash_function() {
    let p = libs();
    let ic: FnInitHash = p.c.f("SPX_initialize_hash_function");
    let ir: FnInitHash = p.r.f("SPX_initialize_hash_function");
    let mut rng = Rng::for_row(21);
    for i in 0..ITERS {
        let pub_seed = match i % 3 {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let sk_seed = rng.bytes(SPX_N);
        let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
        let mut cr = Ctx::with_seeds(&pub_seed, &sk_seed);
        unsafe {
            ic(cc.as_mut_ptr());
            ir(cr.as_mut_ptr());
        }
        eq_bytes("SPX_initialize_hash_function/full ctx", i, &cc.0, &cr.0);
    }
}

fn prf_addr_row(row: u64, addr_mode: u8) {
    let p = libs();
    let c: FnPrfAddr = p.c.f("SPX_prf_addr");
    let r: FnPrfAddr = p.r.f("SPX_prf_addr");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let addr: [u32; ADDR_SLOTS] = match addr_mode {
            0 => addr_buf(&mut rng),
            1 => [0u32; ADDR_SLOTS],
            _ => [u32::MAX; ADDR_SLOTS],
        };
        let mut oc = vec![0xA5u8; SPX_N + 8];
        let mut or = oc.clone();
        unsafe {
            c(oc.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
            r(or.as_mut_ptr(), cr.as_ptr(), addr.as_ptr());
        }
        eq_bytes("SPX_prf_addr", i, &oc, &or);
    }
}

#[test]
fn cfg_22_prf_addr_random() {
    prf_addr_row(22, 0);
}

#[test]
fn cfg_23_prf_addr_boundary_addresses() {
    prf_addr_row(23, 1);
    prf_addr_row(23, 2);
}

fn gen_message_random_for(row: u64, mlens: &[usize]) {
    let p = libs();
    let c: FnGenMsgRandom = p.c.f("SPX_gen_message_random");
    let r: FnGenMsgRandom = p.r.f("SPX_gen_message_random");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let sk_prf = rng.bytes(SPX_N);
        let optrand = rng.bytes(SPX_N);
        for &mlen in mlens {
            let m = rng.bytes(mlen.max(1));
            // NOTE: the BLAKE backend's `gen_message_random` finalises straight
            // into `R` with `blakeX_final`, which writes 32 (BLAKE-256) or 64
            // (BLAKE-512) bytes, not SPX_N. In `sign.c` `R` points into the
            // SPX_BYTES signature buffer so that is harmless; here the output
            // buffer must be at least as large or we corrupt the heap.
            let mut oc = vec![0xA5u8; SPX_N + 64 + 8];
            let mut or = oc.clone();
            unsafe {
                c(
                    oc.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.as_ptr(),
                );
                r(
                    or.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cr.as_ptr(),
                );
            }
            eq_bytes(
                &format!("SPX_gen_message_random(mlen={mlen})"),
                i,
                &oc,
                &or,
            );
        }
    }
}

#[test]
fn cfg_24_gen_message_random_mlen_zero() {
    gen_message_random_for(24, &[0]);
}

#[test]
fn cfg_25_26_27_gen_message_random_block_boundary() {
    // sha2 branches on SPX_N + mlen < SPX_SHAX_BLOCK_BYTES; the sponge backends
    // change block count at the same magnitudes.
    let b = SHAX_BLOCK_BYTES - SPX_N;
    gen_message_random_for(25, &[b - 1, b, b + 1]);
}

#[test]
fn cfg_28_gen_message_random_multiblock() {
    let mut rng = Rng::for_row(280);
    let mut lens: Vec<usize> = vec![1, 31, 32, 33, 135, 136, 137, 255, 256, 257];
    for _ in 0..6 {
        lens.push((rng.next_u32() % 4096) as usize + 1);
    }
    gen_message_random_for(28, &lens);
}

fn hash_message_for(row: u64, mlens: &[usize]) {
    let p = libs();
    let c: FnHashMessage = p.c.f("SPX_hash_message");
    let r: FnHashMessage = p.r.f("SPX_hash_message");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let rr = rng.bytes(SPX_N);
        let pk = rng.bytes(SPX_PK_BYTES);
        for &mlen in mlens {
            let m = rng.bytes(mlen.max(1));
            let mut dc = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
            let mut dr = dc.clone();
            let mut tc = 0u64;
            let mut tr = 0u64;
            let mut lc = 0u32;
            let mut lr = 0u32;
            unsafe {
                c(
                    dc.as_mut_ptr(),
                    &mut tc,
                    &mut lc,
                    rr.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.as_ptr(),
                );
                r(
                    dr.as_mut_ptr(),
                    &mut tr,
                    &mut lr,
                    rr.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cr.as_ptr(),
                );
            }
            let lbl = format!("SPX_hash_message(mlen={mlen})");
            eq_bytes(&lbl, i, &dc, &dr);
            eq(&format!("{lbl}/tree"), i, tc, tr);
            eq(&format!("{lbl}/leaf_idx"), i, lc, lr);
            // The C masks tree to SPX_TREE_BITS and leaf_idx to SPX_TREE_HEIGHT.
            let tree_mask = if SPX_TREE_BITS >= 64 {
                u64::MAX
            } else {
                (!0u64) >> (64 - SPX_TREE_BITS)
            };
            assert_eq!(tc & !tree_mask, 0, "{lbl}: tree not masked to SPX_TREE_BITS");
            let leaf_mask = (!0u32) >> (32 - SPX_TREE_HEIGHT);
            assert_eq!(lc & !leaf_mask, 0, "{lbl}: leaf_idx not masked");
        }
    }
}

#[test]
fn cfg_29_hash_message_mlen_zero() {
    hash_message_for(29, &[0]);
}

#[test]
fn cfg_30_hash_message_block_boundary() {
    // sha2: SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES
    let b = SHA2_INBLOCKS * SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
    hash_message_for(30, &[b.saturating_sub(1), b, b + 1]);
}

#[test]
fn cfg_31_hash_message_random_lengths() {
    let mut rng = Rng::for_row(310);
    let mut lens: Vec<usize> = vec![1, 31, 32, 33, 135, 136, 137];
    for _ in 0..6 {
        lens.push((rng.next_u32() % 4096) as usize + 1);
    }
    hash_message_for(31, &lens);
}

/* ================================================================== */
/* Rows 32-36 — BLAKE primitives (lib/blake)                          */
/* ================================================================== */

#[cfg(all(feature = "blake", not(feature = "sha2"), not(feature = "shake")))]
mod blake_rows {
    use super::*;

    /// `blakestate256` = `unsigned int h[8], s[4], t[2]; int buflen, nullt;
    ///                    unsigned char buf[64];`
    const ST256: usize = 4 * (8 + 4 + 2) + 4 + 4 + 64;
    /// `blakestate512` = `unsigned long long h[8], s[4], t[2]; int buflen,
    ///                    nullt; unsigned char buf[128];`
    const ST512: usize = 8 * (8 + 4 + 2) + 4 + 4 + 128;

    fn lens_256(rng: &mut Rng) -> Vec<usize> {
        let mut v = vec![0usize, 1, 54, 55, 56, 57, 63, 64, 65, 118, 119, 120, 128, 129];
        for _ in 0..8 {
            v.push((rng.next_u32() % 4096) as usize);
        }
        v
    }

    fn lens_512(rng: &mut Rng) -> Vec<usize> {
        let mut v = vec![0usize, 1, 110, 111, 112, 113, 127, 128, 129, 255, 256, 257];
        for _ in 0..8 {
            v.push((rng.next_u32() % 4096) as usize);
        }
        v
    }

    #[allow(clippy::too_many_arguments)]
    fn blake_row(
        row: u64,
        state_bytes: usize,
        out_bytes: usize,
        init: &str,
        update: &str,
        final_: &str,
        one_shot: &str,
        lens: fn(&mut Rng) -> Vec<usize>,
    ) {
        let p = libs();
        let ic: FnBlakeInit = p.c.f(init);
        let ir: FnBlakeInit = p.r.f(init);
        let uc: FnBlakeUpdate = p.c.f(update);
        let ur: FnBlakeUpdate = p.r.f(update);
        let fc: FnBlakeFinal = p.c.f(final_);
        let fr: FnBlakeFinal = p.r.f(final_);
        let osc: FnBlakeOneShot = p.c.f(one_shot);
        let osr: FnBlakeOneShot = p.r.f(one_shot);

        let mut rng = Rng::for_row(row);
        for (i, inlen) in lens(&mut rng).into_iter().enumerate() {
            let input = rng.bytes(inlen.max(1));

            // --- incremental ---
            let mut sc = vec![0u8; state_bytes];
            let mut sr = vec![0u8; state_bytes];
            let mut hc = vec![0xA5u8; out_bytes + 8];
            let mut hr = hc.clone();
            unsafe {
                ic(sc.as_mut_ptr());
                ir(sr.as_mut_ptr());
                eq_bytes(&format!("{init}/state"), i, &sc, &sr);
                // NOTE: blake*_update takes the length in BITS (the one-shot
                // `blake256`/`blake512` pass `inlen * 8`).
                uc(sc.as_mut_ptr(), input.as_ptr(), (inlen as u64) * 8);
                ur(sr.as_mut_ptr(), input.as_ptr(), (inlen as u64) * 8);
                eq_bytes(&format!("{update}/state(inlen={inlen})"), i, &sc, &sr);
                fc(sc.as_mut_ptr(), hc.as_mut_ptr());
                fr(sr.as_mut_ptr(), hr.as_mut_ptr());
            }
            eq_bytes(&format!("{final_}(inlen={inlen})"), i, &hc, &hr);
            eq_bytes(&format!("{final_}/state(inlen={inlen})"), i, &sc, &sr);

            // --- split updates: exercises the internal buffer path ---
            if inlen >= 3 {
                let split = 1 + (rng.next_u32() as usize % (inlen - 1));
                let mut s2c = vec![0u8; state_bytes];
                let mut s2r = vec![0u8; state_bytes];
                let mut h2c = vec![0xA5u8; out_bytes + 8];
                let mut h2r = h2c.clone();
                unsafe {
                    ic(s2c.as_mut_ptr());
                    ir(s2r.as_mut_ptr());
                    uc(s2c.as_mut_ptr(), input.as_ptr(), (split as u64) * 8);
                    ur(s2r.as_mut_ptr(), input.as_ptr(), (split as u64) * 8);
                    uc(
                        s2c.as_mut_ptr(),
                        input.as_ptr().add(split),
                        ((inlen - split) as u64) * 8,
                    );
                    ur(
                        s2r.as_mut_ptr(),
                        input.as_ptr().add(split),
                        ((inlen - split) as u64) * 8,
                    );
                    fc(s2c.as_mut_ptr(), h2c.as_mut_ptr());
                    fr(s2r.as_mut_ptr(), h2r.as_mut_ptr());
                }
                eq_bytes(&format!("{update}/split({split}/{inlen})"), i, &h2c, &h2r);
                // Deliberately NOT asserting that a split update equals a
                // single update: this reference BLAKE `update` only flushes its
                // internal buffer when `((datalen >> 3) & 0x3F) >= fill`, so
                // split calls can legitimately produce a different digest in
                // the C too. The C is ground truth, so only the C-vs-Rust
                // comparison above is a correctness requirement.
            }

            // --- one shot ---
            let mut gc = vec![0xA5u8; out_bytes + 8];
            let mut gr = gc.clone();
            let (rc, rr) = unsafe {
                (
                    osc(gc.as_mut_ptr(), input.as_ptr(), inlen as u64),
                    osr(gr.as_mut_ptr(), input.as_ptr(), inlen as u64),
                )
            };
            eq_bytes(&format!("{one_shot}(inlen={inlen})"), i, &gc, &gr);
            eq(&format!("{one_shot}/retval"), i, rc, rr);
            eq_bytes(&format!("{one_shot}/matches-incremental"), i, &gc, &hc);
        }
    }

    #[test]
    fn cfg_32_blake256() {
        blake_row(
            32,
            ST256,
            32,
            "blake256_init",
            "blake256_update",
            "blake256_final",
            "blake256",
            lens_256,
        );
    }

    #[test]
    fn cfg_33_blake512() {
        blake_row(
            33,
            ST512,
            64,
            "blake512_init",
            "blake512_update",
            "blake512_final",
            "blake512",
            lens_512,
        );
    }

    #[test]
    fn cfg_34_blake_compress() {
        let p = libs();
        let c256: FnBlakeCompress = p.c.f("blake256_compress");
        let r256: FnBlakeCompress = p.r.f("blake256_compress");
        let c512: FnBlakeCompress = p.c.f("blake512_compress");
        let r512: FnBlakeCompress = p.r.f("blake512_compress");
        let i256: FnBlakeInit = p.c.f("blake256_init");
        let i512: FnBlakeInit = p.c.f("blake512_init");

        let mut rng = Rng::for_row(34);
        for i in 0..ITERS {
            let mut sc = vec![0u8; ST256];
            unsafe { i256(sc.as_mut_ptr()) };
            let mut sr = sc.clone();
            let block = rng.bytes(64);
            unsafe {
                c256(sc.as_mut_ptr(), block.as_ptr());
                r256(sr.as_mut_ptr(), block.as_ptr());
            }
            eq_bytes("blake256_compress", i, &sc, &sr);

            let mut sc = vec![0u8; ST512];
            unsafe { i512(sc.as_mut_ptr()) };
            let mut sr = sc.clone();
            let block = rng.bytes(128);
            unsafe {
                c512(sc.as_mut_ptr(), block.as_ptr());
                r512(sr.as_mut_ptr(), block.as_ptr());
            }
            eq_bytes("blake512_compress", i, &sc, &sr);
        }
    }

    #[test]
    fn cfg_35_blake_mgf1() {
        let p = libs();
        for sym in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let c: FnMgf1 = p.c.f(sym);
            let r: FnMgf1 = p.r.f(sym);
            let mut rng = Rng::for_row(35);
            let mut cases: Vec<(usize, usize)> = Vec::new();
            for &ol in &[1usize, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
                for &il in &[0usize, 1, 16, 64] {
                    cases.push((ol, il));
                }
            }
            for _ in 0..8 {
                cases.push((
                    (rng.next_u32() % 512) as usize + 1,
                    (rng.next_u32() % 256) as usize,
                ));
            }
            for (i, (outlen, inlen)) in cases.into_iter().enumerate() {
                let input = rng.bytes(inlen.max(1));
                // mgf1 writes whole digest blocks, which can run past `outlen`;
                // the buffer is padded generously and compared in full.
                let mut oc = vec![0xA5u8; outlen + 128];
                let mut or = oc.clone();
                unsafe {
                    c(
                        oc.as_mut_ptr(),
                        outlen as core::ffi::c_ulong,
                        input.as_ptr(),
                        inlen as core::ffi::c_ulong,
                    );
                    r(
                        or.as_mut_ptr(),
                        outlen as core::ffi::c_ulong,
                        input.as_ptr(),
                        inlen as core::ffi::c_ulong,
                    );
                }
                eq_bytes(&format!("{sym}(outlen={outlen},inlen={inlen})"), i, &oc, &or);
            }
        }
    }

    #[test]
    fn cfg_36_blake_cst_data_symbol() {
        let p = libs();
        let c = unsafe { p.c.data("cst", 128) };
        let r = unsafe { p.r.data("cst", 128) };
        eq_bytes("cst", 0, c, r);
    }
}

/* ================================================================== */
/* Rows 37-40 — SHA-2 primitives (lib/sha2)                           */
/* ================================================================== */

#[cfg(feature = "sha2")]
mod sha2_rows {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn sha_row(
        row: u64,
        state_bytes: usize,
        block_bytes: usize,
        out_bytes: usize,
        init: &str,
        blocks: &str,
        finalize: &str,
        one_shot: &str,
        fin_lens: &[usize],
    ) {
        let p = libs();
        let ic: FnShaIncInit = p.c.f(init);
        let ir: FnShaIncInit = p.r.f(init);
        let bc: FnShaIncBlocks = p.c.f(blocks);
        let br: FnShaIncBlocks = p.r.f(blocks);
        let fc: FnShaIncFinalize = p.c.f(finalize);
        let fr: FnShaIncFinalize = p.r.f(finalize);
        let osc: FnSha = p.c.f(one_shot);
        let osr: FnSha = p.r.f(one_shot);

        let mut rng = Rng::for_row(row);
        for (i, &nblocks) in [0usize, 1, 2, 5].iter().enumerate() {
            for &finlen in fin_lens {
                let blk = rng.bytes((nblocks * block_bytes).max(1));
                let tail = rng.bytes(finlen.max(1));

                let mut sc = vec![0u8; state_bytes];
                let mut sr = vec![0u8; state_bytes];
                unsafe {
                    ic(sc.as_mut_ptr());
                    ir(sr.as_mut_ptr());
                }
                eq_bytes(&format!("{init}/state"), i, &sc, &sr);
                unsafe {
                    bc(sc.as_mut_ptr(), blk.as_ptr(), nblocks);
                    br(sr.as_mut_ptr(), blk.as_ptr(), nblocks);
                }
                eq_bytes(&format!("{blocks}/state(n={nblocks})"), i, &sc, &sr);

                let mut hc = vec![0xA5u8; out_bytes + 8];
                let mut hr = hc.clone();
                unsafe {
                    fc(hc.as_mut_ptr(), sc.as_mut_ptr(), tail.as_ptr(), finlen);
                    fr(hr.as_mut_ptr(), sr.as_mut_ptr(), tail.as_ptr(), finlen);
                }
                eq_bytes(
                    &format!("{finalize}(n={nblocks},finlen={finlen})"),
                    i,
                    &hc,
                    &hr,
                );
                eq_bytes(&format!("{finalize}/state"), i, &sc, &sr);

                // With no prefix blocks the incremental result must equal the
                // one-shot over the same bytes (both C and Rust).
                if nblocks == 0 {
                    let mut gc = vec![0xA5u8; out_bytes + 8];
                    let mut gr = gc.clone();
                    unsafe {
                        osc(gc.as_mut_ptr(), tail.as_ptr(), finlen);
                        osr(gr.as_mut_ptr(), tail.as_ptr(), finlen);
                    }
                    eq_bytes(&format!("{one_shot}(inlen={finlen})"), i, &gc, &gr);
                    eq_bytes(&format!("{one_shot}/matches-incremental"), i, &gc, &hc);
                }
            }
        }
    }

    #[test]
    fn cfg_37_sha256() {
        let mut rng = Rng::for_row(370);
        let mut lens: Vec<usize> = vec![0, 1, 54, 55, 56, 57, 63, 64, 65, 119, 120, 128];
        for _ in 0..6 {
            lens.push((rng.next_u32() % 1024) as usize);
        }
        sha_row(
            37,
            40,
            64,
            32,
            "sha256_inc_init",
            "sha256_inc_blocks",
            "sha256_inc_finalize",
            "sha256",
            &lens,
        );
    }

    #[test]
    fn cfg_38_sha512() {
        let mut rng = Rng::for_row(380);
        let mut lens: Vec<usize> = vec![0, 1, 110, 111, 112, 113, 127, 128, 129, 255, 256];
        for _ in 0..6 {
            lens.push((rng.next_u32() % 1024) as usize);
        }
        sha_row(
            38,
            72,
            128,
            64,
            "sha512_inc_init",
            "sha512_inc_blocks",
            "sha512_inc_finalize",
            "sha512",
            &lens,
        );
    }

    #[test]
    fn cfg_39_mgf1() {
        let p = libs();
        for sym in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let c: FnMgf1 = p.c.f(sym);
            let r: FnMgf1 = p.r.f(sym);
            let mut rng = Rng::for_row(39);
            let mut cases: Vec<(usize, usize)> = Vec::new();
            for &ol in &[1usize, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
                for &il in &[0usize, 1, 16, 64] {
                    cases.push((ol, il));
                }
            }
            for _ in 0..8 {
                cases.push((
                    (rng.next_u32() % 512) as usize + 1,
                    (rng.next_u32() % 256) as usize,
                ));
            }
            for (i, (outlen, inlen)) in cases.into_iter().enumerate() {
                let input = rng.bytes(inlen.max(1));
                let mut oc = vec![0xA5u8; outlen + 128];
                let mut or = oc.clone();
                unsafe {
                    c(
                        oc.as_mut_ptr(),
                        outlen as core::ffi::c_ulong,
                        input.as_ptr(),
                        inlen as core::ffi::c_ulong,
                    );
                    r(
                        or.as_mut_ptr(),
                        outlen as core::ffi::c_ulong,
                        input.as_ptr(),
                        inlen as core::ffi::c_ulong,
                    );
                }
                eq_bytes(&format!("{sym}(outlen={outlen},inlen={inlen})"), i, &oc, &or);
            }
        }
    }

    #[test]
    fn cfg_40_seed_state() {
        let p = libs();
        let c: FnSeedState = p.c.f("SPX_seed_state");
        let r: FnSeedState = p.r.f("SPX_seed_state");
        let mut rng = Rng::for_row(40);
        for i in 0..ITERS {
            let pub_seed = match i % 3 {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let sk_seed = rng.bytes(SPX_N);
            let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
            let mut cr = Ctx::with_seeds(&pub_seed, &sk_seed);
            unsafe {
                c(cc.as_mut_ptr());
                r(cr.as_mut_ptr());
            }
            eq_bytes("SPX_seed_state/full ctx", i, &cc.0, &cr.0);
        }
    }
}

/* ================================================================== */
/* Rows 41-43 — SHAKE-256 primitives (lib/shake)                      */
/* ================================================================== */

#[cfg(all(feature = "shake", not(feature = "sha2")))]
mod shake_rows {
    use super::*;

    const RATE: usize = 136;

    fn u64s(s: &[u64]) -> Vec<u8> {
        s.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    #[test]
    fn cfg_41_shake256_incremental() {
        let p = libs();
        let ic: FnShakeIncInit = p.c.f("shake256_inc_init");
        let ir: FnShakeIncInit = p.r.f("shake256_inc_init");
        let ac: FnShakeIncAbsorb = p.c.f("shake256_inc_absorb");
        let ar: FnShakeIncAbsorb = p.r.f("shake256_inc_absorb");
        let fc: FnShakeIncFinalize = p.c.f("shake256_inc_finalize");
        let fr: FnShakeIncFinalize = p.r.f("shake256_inc_finalize");
        let qc: FnShakeIncSqueeze = p.c.f("shake256_inc_squeeze");
        let qr: FnShakeIncSqueeze = p.r.f("shake256_inc_squeeze");

        let mut rng = Rng::for_row(41);
        for i in 0..ITERS {
            // 1..4 absorb chunks with sizes straddling the 136-byte rate.
            let nchunks = 1 + (i % 4);
            let mut chunks: Vec<Vec<u8>> = Vec::new();
            for k in 0..nchunks {
                let len = match (i + k) % 6 {
                    0 => 0usize,
                    1 => 1,
                    2 => RATE - 1,
                    3 => RATE,
                    4 => RATE + 1,
                    _ => (rng.next_u32() % 600) as usize,
                };
                let mut b = rng.bytes(len.max(1));
                b.truncate(len);
                chunks.push(b);
            }

            let mut sc = [0u64; 26];
            let mut sr = [0u64; 26];
            unsafe {
                ic(sc.as_mut_ptr());
                ir(sr.as_mut_ptr());
            }
            eq_bytes("shake256_inc_init/state", i, &u64s(&sc), &u64s(&sr));
            for ch in &chunks {
                unsafe {
                    ac(sc.as_mut_ptr(), ch.as_ptr(), ch.len());
                    ar(sr.as_mut_ptr(), ch.as_ptr(), ch.len());
                }
                eq_bytes(
                    &format!("shake256_inc_absorb/state(len={})", ch.len()),
                    i,
                    &u64s(&sc),
                    &u64s(&sr),
                );
            }
            unsafe {
                fc(sc.as_mut_ptr());
                fr(sr.as_mut_ptr());
            }
            eq_bytes("shake256_inc_finalize/state", i, &u64s(&sc), &u64s(&sr));

            // 1..3 successive squeezes (state must advance identically).
            let nsq = 1 + (i % 3);
            for k in 0..nsq {
                let outlen = match (i + k) % 5 {
                    0 => 1usize,
                    1 => RATE - 1,
                    2 => RATE,
                    3 => RATE + 1,
                    _ => (rng.next_u32() % 400) as usize + 1,
                };
                let mut oc = vec![0xA5u8; outlen + 8];
                let mut or = oc.clone();
                unsafe {
                    qc(oc.as_mut_ptr(), outlen, sc.as_mut_ptr());
                    qr(or.as_mut_ptr(), outlen, sr.as_mut_ptr());
                }
                eq_bytes(&format!("shake256_inc_squeeze(outlen={outlen})"), i, &oc, &or);
                eq_bytes("shake256_inc_squeeze/state", i, &u64s(&sc), &u64s(&sr));
            }
        }
    }

    #[test]
    fn cfg_42_shake256_absorb_squeezeblocks() {
        let p = libs();
        let ac: FnShakeAbsorb = p.c.f("shake256_absorb");
        let ar: FnShakeAbsorb = p.r.f("shake256_absorb");
        let qc: FnShakeSqueezeblocks = p.c.f("shake256_squeezeblocks");
        let qr: FnShakeSqueezeblocks = p.r.f("shake256_squeezeblocks");
        let mut rng = Rng::for_row(42);
        let mut lens: Vec<usize> = vec![0, 1, 135, 136, 137, 271, 272, 273];
        for _ in 0..6 {
            lens.push((rng.next_u32() % 1000) as usize);
        }
        for (i, inlen) in lens.into_iter().enumerate() {
            let input = rng.bytes(inlen.max(1));
            for &nblocks in &[0usize, 1, 2, 3] {
                // `shake256_absorb` operates on a bare `uint64_t s[25]`.
                let mut sc = [0u64; 25];
                let mut sr = [0u64; 25];
                unsafe {
                    ac(sc.as_mut_ptr(), input.as_ptr(), inlen);
                    ar(sr.as_mut_ptr(), input.as_ptr(), inlen);
                }
                eq_bytes(
                    &format!("shake256_absorb/state(inlen={inlen})"),
                    i,
                    &u64s(&sc),
                    &u64s(&sr),
                );
                let mut oc = vec![0xA5u8; nblocks * RATE + 8];
                let mut or = oc.clone();
                unsafe {
                    qc(oc.as_mut_ptr(), nblocks, sc.as_mut_ptr());
                    qr(or.as_mut_ptr(), nblocks, sr.as_mut_ptr());
                }
                eq_bytes(
                    &format!("shake256_squeezeblocks(n={nblocks},inlen={inlen})"),
                    i,
                    &oc,
                    &or,
                );
                eq_bytes("shake256_squeezeblocks/state", i, &u64s(&sc), &u64s(&sr));
            }
        }
    }

    #[test]
    fn cfg_43_shake256_one_shot() {
        let p = libs();
        let c: FnShake = p.c.f("shake256");
        let r: FnShake = p.r.f("shake256");
        let mut rng = Rng::for_row(43);
        let mut cases: Vec<(usize, usize)> = Vec::new();
        for &ol in &[1usize, 32, 135, 136, 137, 272] {
            for &il in &[0usize, 1, 135, 136, 137] {
                cases.push((ol, il));
            }
        }
        for _ in 0..10 {
            cases.push((
                (rng.next_u32() % 512) as usize + 1,
                (rng.next_u32() % 512) as usize,
            ));
        }
        for (i, (outlen, inlen)) in cases.into_iter().enumerate() {
            let input = rng.bytes(inlen.max(1));
            let mut oc = vec![0xA5u8; outlen + 8];
            let mut or = oc.clone();
            unsafe {
                c(oc.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                r(or.as_mut_ptr(), outlen, input.as_ptr(), inlen);
            }
            eq_bytes(
                &format!("shake256(outlen={outlen},inlen={inlen})"),
                i,
                &oc,
                &or,
            );
        }
    }
}

/* ================================================================== */
/* Rows 44-47 — Haraka primitives (lib/haraka)                        */
/* ================================================================== */

#[cfg(all(
    not(feature = "sha2"),
    not(feature = "shake"),
    not(feature = "blake")
))]
mod haraka_rows {
    use super::*;

    const RATE: usize = 32;

    #[test]
    fn cfg_44_tweak_constants() {
        let p = libs();
        let c: FnTweakConstants = p.c.f("SPX_tweak_constants");
        let r: FnTweakConstants = p.r.f("SPX_tweak_constants");
        let mut rng = Rng::for_row(44);
        for i in 0..ITERS {
            let pub_seed = match i % 3 {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let sk_seed = rng.bytes(SPX_N);
            let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
            let mut cr = Ctx::with_seeds(&pub_seed, &sk_seed);
            unsafe {
                c(cc.as_mut_ptr());
                r(cr.as_mut_ptr());
            }
            eq_bytes("SPX_tweak_constants/full ctx", i, &cc.0, &cr.0);
        }
    }

    #[test]
    fn cfg_45_haraka_permutations() {
        let p = libs();
        let h256c: FnHaraka = p.c.f("SPX_haraka256");
        let h256r: FnHaraka = p.r.f("SPX_haraka256");
        let h512c: FnHaraka = p.c.f("SPX_haraka512");
        let h512r: FnHaraka = p.r.f("SPX_haraka512");
        let permc: FnHaraka = p.c.f("SPX_haraka512_perm");
        let permr: FnHaraka = p.r.f("SPX_haraka512_perm");

        let mut rng = Rng::for_row(45);
        for i in 0..ITERS {
            let (cc, cr) = init_ctx_pair(&mut rng);
            eq_bytes("haraka/ctx-setup", i, &cc.0, &cr.0);

            let in32 = match i % 3 {
                0 => vec![0u8; 32],
                1 => vec![0xFFu8; 32],
                _ => rng.bytes(32),
            };
            let mut oc = vec![0xA5u8; 32 + 8];
            let mut or = oc.clone();
            unsafe {
                h256c(oc.as_mut_ptr(), in32.as_ptr(), cc.as_ptr());
                h256r(or.as_mut_ptr(), in32.as_ptr(), cr.as_ptr());
            }
            eq_bytes("SPX_haraka256", i, &oc, &or);

            let in64 = match i % 3 {
                0 => vec![0u8; 64],
                1 => vec![0xFFu8; 64],
                _ => rng.bytes(64),
            };
            let mut oc = vec![0xA5u8; 32 + 8];
            let mut or = oc.clone();
            unsafe {
                h512c(oc.as_mut_ptr(), in64.as_ptr(), cc.as_ptr());
                h512r(or.as_mut_ptr(), in64.as_ptr(), cr.as_ptr());
            }
            eq_bytes("SPX_haraka512", i, &oc, &or);

            let mut oc = vec![0xA5u8; 64 + 8];
            let mut or = oc.clone();
            unsafe {
                permc(oc.as_mut_ptr(), in64.as_ptr(), cc.as_ptr());
                permr(or.as_mut_ptr(), in64.as_ptr(), cr.as_ptr());
            }
            eq_bytes("SPX_haraka512_perm", i, &oc, &or);
        }
    }

    #[test]
    fn cfg_46_haraka_sponge_incremental() {
        let p = libs();
        let ic: FnHarakaSIncInit = p.c.f("SPX_haraka_S_inc_init");
        let ir: FnHarakaSIncInit = p.r.f("SPX_haraka_S_inc_init");
        let ac: FnHarakaSIncAbsorb = p.c.f("SPX_haraka_S_inc_absorb");
        let ar: FnHarakaSIncAbsorb = p.r.f("SPX_haraka_S_inc_absorb");
        let fc: FnHarakaSIncFinalize = p.c.f("SPX_haraka_S_inc_finalize");
        let fr: FnHarakaSIncFinalize = p.r.f("SPX_haraka_S_inc_finalize");
        let qc: FnHarakaSIncSqueeze = p.c.f("SPX_haraka_S_inc_squeeze");
        let qr: FnHarakaSIncSqueeze = p.r.f("SPX_haraka_S_inc_squeeze");

        let mut rng = Rng::for_row(46);
        for i in 0..ITERS {
            let (cc, cr) = init_ctx_pair(&mut rng);
            let mut sc = [0u8; 65];
            let mut sr = [0u8; 65];
            unsafe {
                ic(sc.as_mut_ptr());
                ir(sr.as_mut_ptr());
            }
            eq_bytes("haraka_S_inc_init/state", i, &sc, &sr);

            let nchunks = 1 + (i % 4);
            for k in 0..nchunks {
                let len = match (i + k) % 6 {
                    0 => 0usize,
                    1 => 1,
                    2 => RATE - 1,
                    3 => RATE,
                    4 => RATE + 1,
                    _ => (rng.next_u32() % 200) as usize,
                };
                let ch = rng.bytes(len.max(1));
                unsafe {
                    ac(sc.as_mut_ptr(), ch.as_ptr(), len, cc.as_ptr());
                    ar(sr.as_mut_ptr(), ch.as_ptr(), len, cr.as_ptr());
                }
                eq_bytes(
                    &format!("haraka_S_inc_absorb/state(len={len})"),
                    i,
                    &sc,
                    &sr,
                );
            }
            unsafe {
                fc(sc.as_mut_ptr());
                fr(sr.as_mut_ptr());
            }
            eq_bytes("haraka_S_inc_finalize/state", i, &sc, &sr);

            let nsq = 1 + (i % 3);
            for k in 0..nsq {
                let outlen = match (i + k) % 5 {
                    0 => 1usize,
                    1 => RATE - 1,
                    2 => RATE,
                    3 => RATE + 1,
                    _ => (rng.next_u32() % 200) as usize + 1,
                };
                let mut oc = vec![0xA5u8; outlen + 8];
                let mut or = oc.clone();
                unsafe {
                    qc(oc.as_mut_ptr(), outlen, sc.as_mut_ptr(), cc.as_ptr());
                    qr(or.as_mut_ptr(), outlen, sr.as_mut_ptr(), cr.as_ptr());
                }
                eq_bytes(
                    &format!("haraka_S_inc_squeeze(outlen={outlen})"),
                    i,
                    &oc,
                    &or,
                );
                eq_bytes("haraka_S_inc_squeeze/state", i, &sc, &sr);
            }
        }
    }

    #[test]
    fn cfg_47_haraka_S_one_shot() {
        let p = libs();
        let c: FnHarakaS = p.c.f("SPX_haraka_S");
        let r: FnHarakaS = p.r.f("SPX_haraka_S");
        let mut rng = Rng::for_row(47);
        let mut cases: Vec<(usize, usize)> = Vec::new();
        for &ol in &[1usize, 31, 32, 33, 64] {
            for &il in &[0usize, 1, 31, 32, 33] {
                cases.push((ol, il));
            }
        }
        for _ in 0..10 {
            cases.push((
                (rng.next_u32() % 512) as usize + 1,
                (rng.next_u32() % 512) as usize,
            ));
        }
        for (i, (outlen, inlen)) in cases.into_iter().enumerate() {
            let (cc, cr) = init_ctx_pair(&mut rng);
            let input = rng.bytes(inlen.max(1));
            let mut oc = vec![0xA5u8; outlen + 8];
            let mut or = oc.clone();
            unsafe {
                c(
                    oc.as_mut_ptr(),
                    outlen as u64,
                    input.as_ptr(),
                    inlen as u64,
                    cc.as_ptr(),
                );
                r(
                    or.as_mut_ptr(),
                    outlen as u64,
                    input.as_ptr(),
                    inlen as u64,
                    cr.as_ptr(),
                );
            }
            eq_bytes(
                &format!("SPX_haraka_S(outlen={outlen},inlen={inlen})"),
                i,
                &oc,
                &or,
            );
        }
    }
}

/* ================================================================== */
/* Rows 48-52 — WOTS+ (app/src/wots.c, wotsx1.c)                      */
/* ================================================================== */

fn chain_lengths_row(row: u64, msgs: &[Vec<u8>]) {
    let p = libs();
    let c: FnChainLengths = p.c.f("SPX_chain_lengths");
    let r: FnChainLengths = p.r.f("SPX_chain_lengths");
    for (i, m) in msgs.iter().enumerate() {
        let mut lc = vec![0u32; SPX_WOTS_LEN + 4];
        let mut lr = lc.clone();
        unsafe {
            c(lc.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
            r(lr.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
        }
        eq_u32s("SPX_chain_lengths", i, &lc, &lr);
        // Every base-w digit must be < SPX_WOTS_W (property of the C output).
        for (j, &v) in lc[..SPX_WOTS_LEN].iter().enumerate() {
            assert!(
                v < SPX_WOTS_W,
                "chain_lengths[{j}] = {v} >= SPX_WOTS_W in the C output"
            );
        }
    }
}

#[test]
fn cfg_48_chain_lengths_random() {
    let mut rng = Rng::for_row(48);
    let msgs: Vec<Vec<u8>> = (0..ITERS).map(|_| rng.bytes(SPX_N)).collect();
    chain_lengths_row(48, &msgs);
}

#[test]
fn cfg_49_chain_lengths_extremes() {
    let msgs = vec![
        vec![0x00u8; SPX_N],
        vec![0xFFu8; SPX_N],
        vec![0x0Fu8; SPX_N],
        vec![0xF0u8; SPX_N],
    ];
    chain_lengths_row(49, &msgs);
}

#[test]
fn cfg_50_wots_pk_from_sig() {
    let p = libs();
    let c: FnWotsPkFromSig = p.c.f("SPX_wots_pk_from_sig");
    let r: FnWotsPkFromSig = p.r.f("SPX_wots_pk_from_sig");
    let mut rng = Rng::for_row(50);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let msg = match i % 3 {
            0 => vec![0u8; SPX_N],
            1 => vec![0xFFu8; SPX_N],
            _ => rng.bytes(SPX_N),
        };
        let addr = addr_buf(&mut rng);
        let mut pc = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let mut pr = pc.clone();
        let mut ac = addr;
        let mut ar = addr;
        unsafe {
            c(
                pc.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cc.as_ptr(),
                ac.as_mut_ptr(),
            );
            r(
                pr.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                cr.as_ptr(),
                ar.as_mut_ptr(),
            );
        }
        eq_bytes("SPX_wots_pk_from_sig/pk", i, &pc, &pr);
        eq_u32s("SPX_wots_pk_from_sig/addr", i, &ac, &ar);
    }
}

/// Rows 51/52: `wots_gen_leafx1` in both of its modes.
fn wots_gen_leafx1_row(row: u64, signing: bool) {
    let p = libs();
    let c: FnWotsGenLeafX1 = p.c.f("SPX_wots_gen_leafx1");
    let r: FnWotsGenLeafX1 = p.r.f("SPX_wots_gen_leafx1");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let leaf_idx = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);
        let root = rng.bytes(SPX_N);

        // `wots_steps` comes from chain_lengths, as in merkle_sign.
        let cl: FnChainLengths = p.c.f("SPX_chain_lengths");
        let mut steps = vec![0u32; SPX_WOTS_LEN];
        unsafe { cl(steps.as_mut_ptr() as *mut core::ffi::c_uint, root.as_ptr()) };

        let base_addr = addr_buf(&mut rng);
        let mut sig_c = vec![0xA5u8; SPX_WOTS_BYTES];
        let mut sig_r = sig_c.clone();

        let mut ic = LeafInfoX1 {
            wots_sig: if signing {
                sig_c.as_mut_ptr()
            } else {
                core::ptr::null_mut()
            },
            wots_sign_leaf: if signing { leaf_idx } else { !0u32 },
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: base_addr[..8].try_into().unwrap(),
            pk_addr: base_addr[8..16].try_into().unwrap(),
        };
        let mut ir = LeafInfoX1 {
            wots_sig: if signing {
                sig_r.as_mut_ptr()
            } else {
                core::ptr::null_mut()
            },
            ..ic
        };

        let mut dc = vec![0xA5u8; SPX_N + 8];
        let mut dr = dc.clone();
        unsafe {
            c(dc.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut ic);
            r(dr.as_mut_ptr(), cr.as_ptr(), leaf_idx, &mut ir);
        }
        let lbl = if signing { "signing" } else { "pk-only" };
        eq_bytes(&format!("SPX_wots_gen_leafx1[{lbl}]/dest"), i, &dc, &dr);
        eq_u32s(
            &format!("SPX_wots_gen_leafx1[{lbl}]/leaf_addr"),
            i,
            &ic.leaf_addr,
            &ir.leaf_addr,
        );
        eq_u32s(
            &format!("SPX_wots_gen_leafx1[{lbl}]/pk_addr"),
            i,
            &ic.pk_addr,
            &ir.pk_addr,
        );
        if signing {
            eq_bytes(
                "SPX_wots_gen_leafx1[signing]/wots_sig",
                i,
                &sig_c,
                &sig_r,
            );
        }
    }
}

#[test]
fn cfg_51_wots_gen_leafx1_pk_only() {
    wots_gen_leafx1_row(51, false);
}

#[test]
fn cfg_52_wots_gen_leafx1_signing() {
    wots_gen_leafx1_row(52, true);
}

/* ================================================================== */
/* Rows 53-57 — FORS (app/src/fors.c)                                 */
/* ================================================================== */

#[test]
fn cfg_53_fors_gen_leafx1() {
    let p = libs();
    let c: FnForsGenLeafX1 = p.c.f("SPX_fors_gen_leafx1");
    let r: FnForsGenLeafX1 = p.r.f("SPX_fors_gen_leafx1");
    let max_idx = (SPX_FORS_TREES as u64) * (1u64 << SPX_FORS_HEIGHT) - 1;
    let mut rng = Rng::for_row(53);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let addr_idx = match i % 3 {
            0 => 0u32,
            1 => max_idx as u32,
            _ => (rng.next_u64() % (max_idx + 1)) as u32,
        };
        let base = addr_buf(&mut rng);
        let mut ic = ForsGenLeafInfo {
            leaf_addrx: base[..8].try_into().unwrap(),
        };
        let mut ir = ic;
        let mut lc = vec![0xA5u8; SPX_N + 8];
        let mut lr = lc.clone();
        unsafe {
            c(lc.as_mut_ptr(), cc.as_ptr(), addr_idx, &mut ic);
            r(lr.as_mut_ptr(), cr.as_ptr(), addr_idx, &mut ir);
        }
        eq_bytes("SPX_fors_gen_leafx1/leaf", i, &lc, &lr);
        eq_u32s(
            "SPX_fors_gen_leafx1/leaf_addrx",
            i,
            &ic.leaf_addrx,
            &ir.leaf_addrx,
        );
    }
}

/// Returns (message, C signature, C pk) so row 56 can reuse them.
fn fors_sign_row(row: u64, msgs: &[Vec<u8>], iters: usize) -> Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let p = libs();
    let c: FnForsSign = p.c.f("SPX_fors_sign");
    let r: FnForsSign = p.r.f("SPX_fors_sign");
    let mut rng = Rng::for_row(row);
    let mut out = Vec::new();
    for i in 0..iters {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let m = if msgs.is_empty() {
            rng.bytes(SPX_FORS_MSG_BYTES)
        } else {
            msgs[i % msgs.len()].clone()
        };
        let fors_addr = addr_buf(&mut rng);
        let mut sc = vec![0xA5u8; SPX_FORS_BYTES + 8];
        let mut sr = sc.clone();
        let mut pc = vec![0xA5u8; SPX_N + 8];
        let mut pr = pc.clone();
        unsafe {
            c(
                sc.as_mut_ptr(),
                pc.as_mut_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            r(
                sr.as_mut_ptr(),
                pr.as_mut_ptr(),
                m.as_ptr(),
                cr.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq_bytes("SPX_fors_sign/sig", i, &sc, &sr);
        eq_bytes("SPX_fors_sign/pk", i, &pc, &pr);

        // Round trip through fors_pk_from_sig on both sides.
        let pkc: FnForsPkFromSig = p.c.f("SPX_fors_pk_from_sig");
        let pkr: FnForsPkFromSig = p.r.f("SPX_fors_pk_from_sig");
        let mut qc = vec![0xA5u8; SPX_N + 8];
        let mut qr = qc.clone();
        unsafe {
            pkc(
                qc.as_mut_ptr(),
                sc.as_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            pkr(
                qr.as_mut_ptr(),
                sr.as_ptr(),
                m.as_ptr(),
                cr.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq_bytes("SPX_fors_pk_from_sig(valid)/pk", i, &qc, &qr);
        eq_bytes("SPX_fors_pk_from_sig/reproduces fors_sign pk", i, &qc, &pc);
        out.push((m, sc, pc));
    }
    out
}

#[test]
fn cfg_54_56_fors_sign_and_pk_from_sig() {
    fors_sign_row(54, &[], ITERS.min(8));
}

#[test]
fn cfg_55_fors_sign_index_extremes() {
    let msgs = vec![
        vec![0x00u8; SPX_FORS_MSG_BYTES],
        vec![0xFFu8; SPX_FORS_MSG_BYTES],
        vec![0xAAu8; SPX_FORS_MSG_BYTES],
        vec![0x55u8; SPX_FORS_MSG_BYTES],
    ];
    fors_sign_row(55, &msgs, 4);
}

#[test]
fn cfg_57_fors_pk_from_sig_random_signature() {
    let p = libs();
    let c: FnForsPkFromSig = p.c.f("SPX_fors_pk_from_sig");
    let r: FnForsPkFromSig = p.r.f("SPX_fors_pk_from_sig");
    let mut rng = Rng::for_row(57);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let sig = rng.bytes(SPX_FORS_BYTES);
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let fors_addr = addr_buf(&mut rng);
        let mut pc = vec![0xA5u8; SPX_N + 8];
        let mut pr = pc.clone();
        unsafe {
            c(
                pc.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cc.as_ptr(),
                fors_addr.as_ptr(),
            );
            r(
                pr.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cr.as_ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq_bytes("SPX_fors_pk_from_sig(random sig)", i, &pc, &pr);
    }
}

/* ================================================================== */
/* Rows 58-69 — Merkle / treehash                                     */
/* ================================================================== */

fn compute_root_row(row: u64, tree_height: u32, parity: Option<u32>, offset_random: bool) {
    let p = libs();
    let c: FnComputeRoot = p.c.f("SPX_compute_root");
    let r: FnComputeRoot = p.r.f("SPX_compute_root");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let leaf = rng.bytes(SPX_N);
        let auth_path = rng.bytes(tree_height as usize * SPX_N);
        let mut leaf_idx = rng.next_u32() & ((1u32 << tree_height.min(31)) - 1);
        if let Some(par) = parity {
            leaf_idx = (leaf_idx & !1) | par;
        }
        let idx_offset = if offset_random {
            (rng.next_u32() & 0xFFFF) << tree_height.min(15)
        } else {
            0
        };
        let addr = addr_buf(&mut rng);
        let mut oc = vec![0xA5u8; SPX_N + 8];
        let mut or = oc.clone();
        let mut ac = addr;
        let mut ar = addr;
        unsafe {
            c(
                oc.as_mut_ptr(),
                leaf.as_ptr(),
                leaf_idx,
                idx_offset,
                auth_path.as_ptr(),
                tree_height,
                cc.as_ptr(),
                ac.as_mut_ptr(),
            );
            r(
                or.as_mut_ptr(),
                leaf.as_ptr(),
                leaf_idx,
                idx_offset,
                auth_path.as_ptr(),
                tree_height,
                cr.as_ptr(),
                ar.as_mut_ptr(),
            );
        }
        let lbl = format!("SPX_compute_root(h={tree_height},idx={leaf_idx},off={idx_offset})");
        eq_bytes(&lbl, i, &oc, &or);
        eq_u32s(&format!("{lbl}/addr"), i, &ac, &ar);
    }
}

#[test]
fn cfg_58_compute_root_even_leaf() {
    compute_root_row(58, SPX_TREE_HEIGHT, Some(0), false);
}

#[test]
fn cfg_59_compute_root_odd_leaf() {
    compute_root_row(59, SPX_TREE_HEIGHT, Some(1), false);
}

#[test]
fn cfg_60_compute_root_fors_height_with_offset() {
    compute_root_row(60, SPX_FORS_HEIGHT, None, true);
}

#[test]
fn cfg_61_compute_root_height_one() {
    compute_root_row(61, 1, None, false);
}

#[test]
fn cfg_62_treehash_with_test_callback() {
    let p = libs();
    let c: FnTreehash = p.c.f("SPX_treehash");
    let r: FnTreehash = p.r.f("SPX_treehash");
    let mut rng = Rng::for_row(62);
    for i in 0..ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        for &h in &[1u32, 2, 3] {
            let leaf_idx = rng.next_u32() & ((1u32 << h) - 1);
            let idx_offset = if i % 2 == 0 { 0 } else { (rng.next_u32() & 0xFF) << h };
            let addr = addr_buf(&mut rng);
            let mut rc = vec![0xA5u8; SPX_N + 8];
            let mut rr = rc.clone();
            let mut pc = vec![0xA5u8; h as usize * SPX_N + 8];
            let mut pr = pc.clone();
            let mut ac = addr;
            let mut ar = addr;
            unsafe {
                c(
                    rc.as_mut_ptr(),
                    pc.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    test_gen_leaf,
                    ac.as_mut_ptr(),
                );
                r(
                    rr.as_mut_ptr(),
                    pr.as_mut_ptr(),
                    cr.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    test_gen_leaf,
                    ar.as_mut_ptr(),
                );
            }
            let lbl = format!("SPX_treehash[cb](h={h},idx={leaf_idx},off={idx_offset})");
            eq_bytes(&format!("{lbl}/root"), i, &rc, &rr);
            eq_bytes(&format!("{lbl}/auth_path"), i, &pc, &pr);
            eq_u32s(&format!("{lbl}/tree_addr"), i, &ac, &ar);
        }
    }
}

#[test]
fn cfg_63_treehash_with_fors_gen_leafx1() {
    // Real usage: `treehash`'s `gen_leaf` gets `tree_addr` as its 4th argument,
    // and `fors_gen_leafx1` reinterprets it as `fors_gen_leaf_info *`
    // (`{ uint32_t leaf_addrx[8] }`) — layout compatible by construction.
    let p = libs();
    let c: FnTreehash = p.c.f("SPX_treehash");
    let r: FnTreehash = p.r.f("SPX_treehash");
    let gc: GenLeafFn = p.c.f("SPX_fors_gen_leafx1");
    let gr: GenLeafFn = p.r.f("SPX_fors_gen_leafx1");
    let h = SPX_FORS_HEIGHT.min(4);
    let mut rng = Rng::for_row(63);
    for i in 0..ITERS.min(8) {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let leaf_idx = rng.next_u32() & ((1u32 << h) - 1);
        let idx_offset = if i % 2 == 0 { 0 } else { (rng.next_u32() & 0xFF) << h };
        let addr = addr_buf(&mut rng);
        let mut rc = vec![0xA5u8; SPX_N + 8];
        let mut rr = rc.clone();
        let mut pc = vec![0xA5u8; h as usize * SPX_N + 8];
        let mut pr = pc.clone();
        let mut ac = addr;
        let mut ar = addr;
        unsafe {
            c(
                rc.as_mut_ptr(),
                pc.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                gc,
                ac.as_mut_ptr(),
            );
            r(
                rr.as_mut_ptr(),
                pr.as_mut_ptr(),
                cr.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                gr,
                ar.as_mut_ptr(),
            );
        }
        let lbl = format!("SPX_treehash[fors_gen_leafx1](h={h},idx={leaf_idx})");
        eq_bytes(&format!("{lbl}/root"), i, &rc, &rr);
        eq_bytes(&format!("{lbl}/auth_path"), i, &pc, &pr);
        eq_u32s(&format!("{lbl}/tree_addr"), i, &ac, &ar);
    }
}

/// Rows 64/65: `wots_treehashx1`.
fn wots_treehashx1_row(row: u64, sentinel: bool) {
    let p = libs();
    let c: FnTreehashX1 = p.c.f("SPX_wots_treehashx1");
    let r: FnTreehashX1 = p.r.f("SPX_wots_treehashx1");
    let cl: FnChainLengths = p.c.f("SPX_chain_lengths");
    let h = SPX_TREE_HEIGHT;
    let mut rng = Rng::for_row(row);
    for i in 0..HEAVY_ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let root = rng.bytes(SPX_N);
        let mut steps = vec![0u32; SPX_WOTS_LEN];
        unsafe { cl(steps.as_mut_ptr() as *mut core::ffi::c_uint, root.as_ptr()) };

        let leaf_idx = if sentinel {
            !0u32
        } else {
            rng.next_u32() & ((1u32 << h) - 1)
        };
        let addr = addr_buf(&mut rng);
        let mut sig_c = vec![0xA5u8; SPX_WOTS_BYTES];
        let mut sig_r = sig_c.clone();

        let mut ic = LeafInfoX1 {
            wots_sig: if sentinel {
                core::ptr::null_mut()
            } else {
                sig_c.as_mut_ptr()
            },
            wots_sign_leaf: leaf_idx,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: addr[..8].try_into().unwrap(),
            pk_addr: addr[8..16].try_into().unwrap(),
        };
        let mut ir = LeafInfoX1 {
            wots_sig: if sentinel {
                core::ptr::null_mut()
            } else {
                sig_r.as_mut_ptr()
            },
            ..ic
        };

        let mut rootc = vec![0xA5u8; SPX_N + 8];
        let mut rootr = rootc.clone();
        let mut apc = vec![0xA5u8; h as usize * SPX_N + 8];
        let mut apr = apc.clone();
        let mut ac = addr;
        let mut ar = addr;
        unsafe {
            c(
                rootc.as_mut_ptr(),
                apc.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                0,
                h,
                ac.as_mut_ptr(),
                &mut ic,
            );
            r(
                rootr.as_mut_ptr(),
                apr.as_mut_ptr(),
                cr.as_ptr(),
                leaf_idx,
                0,
                h,
                ar.as_mut_ptr(),
                &mut ir,
            );
        }
        let lbl = if sentinel { "sentinel" } else { "signing" };
        eq_bytes(&format!("SPX_wots_treehashx1[{lbl}]/root"), i, &rootc, &rootr);
        eq_bytes(
            &format!("SPX_wots_treehashx1[{lbl}]/auth_path"),
            i,
            &apc,
            &apr,
        );
        eq_u32s(&format!("SPX_wots_treehashx1[{lbl}]/tree_addr"), i, &ac, &ar);
        eq_u32s(
            &format!("SPX_wots_treehashx1[{lbl}]/leaf_addr"),
            i,
            &ic.leaf_addr,
            &ir.leaf_addr,
        );
        if !sentinel {
            eq_bytes(
                "SPX_wots_treehashx1[signing]/wots_sig",
                i,
                &sig_c,
                &sig_r,
            );
        }
    }
}

#[test]
fn cfg_64_wots_treehashx1_signing() {
    wots_treehashx1_row(64, false);
}

#[test]
fn cfg_65_wots_treehashx1_sentinel() {
    wots_treehashx1_row(65, true);
}

#[test]
fn cfg_66_fors_treehashx1() {
    let p = libs();
    let c: FnTreehashX1 = p.c.f("SPX_fors_treehashx1");
    let r: FnTreehashX1 = p.r.f("SPX_fors_treehashx1");
    let h = SPX_FORS_HEIGHT;
    let mut rng = Rng::for_row(66);
    for i in 0..ITERS.min(8) {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let leaf_idx = rng.next_u32() & ((1u32 << h) - 1);
        let tree_no = rng.below(SPX_FORS_TREES);
        let idx_offset = tree_no * (1u32 << h);
        let addr = addr_buf(&mut rng);

        // `fors_treehashx1` passes `info` on to `fors_gen_leafx1`, which treats
        // it as `fors_gen_leaf_info *`; only the first 8 u32 matter.
        let mut ic = LeafInfoX1 {
            leaf_addr: addr[..8].try_into().unwrap(),
            pk_addr: addr[8..16].try_into().unwrap(),
            ..Default::default()
        };
        let mut ir = ic;

        let mut rootc = vec![0xA5u8; SPX_N + 8];
        let mut rootr = rootc.clone();
        let mut apc = vec![0xA5u8; h as usize * SPX_N + 8];
        let mut apr = apc.clone();
        let mut ac = addr;
        let mut ar = addr;
        unsafe {
            c(
                rootc.as_mut_ptr(),
                apc.as_mut_ptr(),
                cc.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ac.as_mut_ptr(),
                &mut ic,
            );
            r(
                rootr.as_mut_ptr(),
                apr.as_mut_ptr(),
                cr.as_ptr(),
                leaf_idx,
                idx_offset,
                h,
                ar.as_mut_ptr(),
                &mut ir,
            );
        }
        eq_bytes("SPX_fors_treehashx1/root", i, &rootc, &rootr);
        eq_bytes("SPX_fors_treehashx1/auth_path", i, &apc, &apr);
        eq_u32s("SPX_fors_treehashx1/tree_addr", i, &ac, &ar);
        eq_u32s(
            "SPX_fors_treehashx1/info.leaf_addr",
            i,
            &ic.leaf_addr,
            &ir.leaf_addr,
        );
    }
}

/// Rows 67/68: `merkle_sign`.
fn merkle_sign_row(row: u64, sentinel: bool) {
    let p = libs();
    let c: FnMerkleSign = p.c.f("SPX_merkle_sign");
    let r: FnMerkleSign = p.r.f("SPX_merkle_sign");
    let mut rng = Rng::for_row(row);
    for i in 0..HEAVY_ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let wots_addr = addr_buf(&mut rng);
        let tree_addr = addr_buf(&mut rng);
        let idx_leaf = if sentinel {
            !0u32
        } else {
            rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1)
        };
        // `root` is an in/out parameter: the WOTS message on the way in, the
        // subtree root on the way out.
        let root_in = rng.bytes(SPX_N);
        let mut rootc = root_in.clone();
        let mut rootr = root_in.clone();
        let mut sc = vec![0xA5u8; MERKLE_SIG_BYTES + 8];
        let mut sr = sc.clone();
        let mut wc = wots_addr;
        let mut wr = wots_addr;
        let mut tc = tree_addr;
        let mut tr = tree_addr;
        unsafe {
            c(
                sc.as_mut_ptr(),
                rootc.as_mut_ptr(),
                cc.as_ptr(),
                wc.as_mut_ptr(),
                tc.as_mut_ptr(),
                idx_leaf,
            );
            r(
                sr.as_mut_ptr(),
                rootr.as_mut_ptr(),
                cr.as_ptr(),
                wr.as_mut_ptr(),
                tr.as_mut_ptr(),
                idx_leaf,
            );
        }
        let lbl = if sentinel { "sentinel" } else { "normal" };
        eq_bytes(&format!("SPX_merkle_sign[{lbl}]/sig"), i, &sc, &sr);
        eq_bytes(&format!("SPX_merkle_sign[{lbl}]/root"), i, &rootc, &rootr);
        eq_u32s(&format!("SPX_merkle_sign[{lbl}]/wots_addr"), i, &wc, &wr);
        eq_u32s(&format!("SPX_merkle_sign[{lbl}]/tree_addr"), i, &tc, &tr);
    }
}

#[test]
fn cfg_67_merkle_sign_normal() {
    merkle_sign_row(67, false);
}

#[test]
fn cfg_68_merkle_sign_sentinel() {
    merkle_sign_row(68, true);
}

#[test]
fn cfg_69_merkle_gen_root() {
    let p = libs();
    let c: FnMerkleGenRoot = p.c.f("SPX_merkle_gen_root");
    let r: FnMerkleGenRoot = p.r.f("SPX_merkle_gen_root");
    let mut rng = Rng::for_row(69);
    for i in 0..HEAVY_ITERS {
        let (cc, cr) = init_ctx_pair(&mut rng);
        let mut oc = vec![0xA5u8; SPX_N + 8];
        let mut or = oc.clone();
        unsafe {
            c(oc.as_mut_ptr(), cc.as_ptr());
            r(or.as_mut_ptr(), cr.as_ptr());
        }
        eq_bytes("SPX_merkle_gen_root", i, &oc, &or);
    }
}

/* ================================================================== */
/* Rows 70-78 — top-level API (app/include/api.h)                     */
/* ================================================================== */

/// Put both DRBGs into the same state so `crypto_sign_keypair` /
/// `crypto_sign_signature` (which call `randombytes` internally for the seed
/// and for `optrand`) consume identical randomness on both sides.
fn seed_both_drbgs(entropy: &[u8]) {
    let p = libs();
    let c: FnRandombytesInit = p.c.f("randombytes_init");
    let r: FnRandombytesInit = p.r.f("randombytes_init");
    let mut ec = entropy.to_vec();
    let mut er = entropy.to_vec();
    unsafe {
        c(ec.as_mut_ptr(), core::ptr::null_mut());
        r(er.as_mut_ptr(), core::ptr::null_mut());
    }
}

struct KeyPair {
    pk_c: Vec<u8>,
    sk_c: Vec<u8>,
    pk_r: Vec<u8>,
    sk_r: Vec<u8>,
}

fn seed_keypair(seed: &[u8], iter: usize) -> KeyPair {
    let p = libs();
    let c: FnSeedKeypair = p.c.f("crypto_sign_seed_keypair");
    let r: FnSeedKeypair = p.r.f("crypto_sign_seed_keypair");
    let mut pk_c = vec![0xA5u8; SPX_PK_BYTES + 8];
    let mut sk_c = vec![0xA5u8; SPX_SK_BYTES + 8];
    let mut pk_r = pk_c.clone();
    let mut sk_r = sk_c.clone();
    let (rc, rr) = unsafe {
        (
            c(pk_c.as_mut_ptr(), sk_c.as_mut_ptr(), seed.as_ptr()),
            r(pk_r.as_mut_ptr(), sk_r.as_mut_ptr(), seed.as_ptr()),
        )
    };
    eq("crypto_sign_seed_keypair/retval", iter, rc, rr);
    eq_bytes("crypto_sign_seed_keypair/pk", iter, &pk_c, &pk_r);
    eq_bytes("crypto_sign_seed_keypair/sk", iter, &sk_c, &sk_r);
    KeyPair {
        pk_c,
        sk_c,
        pk_r,
        sk_r,
    }
}

#[test]
fn cfg_70_size_constants() {
    let p = libs();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES),
        ("crypto_sign_bytes", SPX_BYTES),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES),
    ] {
        let c: FnU64Void = p.c.f(name);
        let r: FnU64Void = p.r.f(name);
        let (a, b) = unsafe { (c(), r()) };
        eq(name, 0, a, b);
        eq(&format!("{name}/expected"), 0, a as usize, expect);
    }
}

#[test]
fn cfg_71_seed_keypair() {
    let mut rng = Rng::for_row(71);
    for i in 0..HEAVY_ITERS {
        let seed = match i {
            0 => vec![0u8; CRYPTO_SEEDBYTES],
            1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        seed_keypair(&seed, i);
    }
}

#[test]
fn cfg_72_keypair_via_drbg() {
    let p = libs();
    let c: FnKeypair = p.c.f("crypto_sign_keypair");
    let r: FnKeypair = p.r.f("crypto_sign_keypair");
    let mut rng = Rng::for_row(72);
    for i in 0..HEAVY_ITERS {
        let entropy = rng.bytes(48);
        seed_both_drbgs(&entropy);
        let mut pk_c = vec![0xA5u8; SPX_PK_BYTES + 8];
        let mut sk_c = vec![0xA5u8; SPX_SK_BYTES + 8];
        let mut pk_r = pk_c.clone();
        let mut sk_r = sk_c.clone();
        let (rc, rr) = unsafe {
            (
                c(pk_c.as_mut_ptr(), sk_c.as_mut_ptr()),
                r(pk_r.as_mut_ptr(), sk_r.as_mut_ptr()),
            )
        };
        eq("crypto_sign_keypair/retval", i, rc, rr);
        eq_bytes("crypto_sign_keypair/pk", i, &pk_c, &pk_r);
        eq_bytes("crypto_sign_keypair/sk", i, &sk_c, &sk_r);
    }
}

/// Signs `m` with both implementations and checks `sig`/`siglen` match.
/// `optrand` comes from each library's own DRBG, so both are re-seeded with the
/// same entropy immediately before the call.
fn sign_both(kp: &KeyPair, m: &[u8], entropy: &[u8], iter: usize) -> (Vec<u8>, Vec<u8>) {
    let p = libs();
    let c: FnSignature = p.c.f("crypto_sign_signature");
    let r: FnSignature = p.r.f("crypto_sign_signature");
    seed_both_drbgs(entropy);
    let mut sc = vec![0xA5u8; SPX_BYTES + 8];
    let mut sr = sc.clone();
    let mut lc = 0usize;
    let mut lr = 0usize;
    let (rc, rr) = unsafe {
        (
            c(
                sc.as_mut_ptr(),
                &mut lc,
                m.as_ptr(),
                m.len(),
                kp.sk_c.as_ptr(),
            ),
            r(
                sr.as_mut_ptr(),
                &mut lr,
                m.as_ptr(),
                m.len(),
                kp.sk_r.as_ptr(),
            ),
        )
    };
    eq("crypto_sign_signature/retval", iter, rc, rr);
    eq("crypto_sign_signature/siglen", iter, lc, lr);
    eq("crypto_sign_signature/siglen==SPX_BYTES", iter, lc, SPX_BYTES);
    eq_bytes("crypto_sign_signature/sig", iter, &sc, &sr);
    sc.truncate(SPX_BYTES);
    sr.truncate(SPX_BYTES);
    (sc, sr)
}

fn msg_lengths(rng: &mut Rng) -> Vec<usize> {
    let mut v = vec![0usize, 1, 32, 33, 47, 48, 64, 127, 128, 129];
    for _ in 0..2 {
        v.push((rng.next_u32() % 1024) as usize + 1);
    }
    v
}

#[test]
fn cfg_73_74_75_sign_and_verify() {
    let p = libs();
    let vc: FnVerify = p.c.f("crypto_sign_verify");
    let vr: FnVerify = p.r.f("crypto_sign_verify");
    let mut rng = Rng::for_row(73);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let kp = seed_keypair(&seed, 0);
    for (i, mlen) in msg_lengths(&mut rng).into_iter().enumerate() {
        let m = rng.bytes(mlen.max(1));
        let m = &m[..mlen];
        let entropy = rng.bytes(48);
        let (sc, sr) = sign_both(&kp, m, &entropy, i);

        // Row 75: each side verifies its own signature.
        let (a, b) = unsafe {
            (
                vc(sc.as_ptr(), sc.len(), m.as_ptr(), mlen, kp.pk_c.as_ptr()),
                vr(sr.as_ptr(), sr.len(), m.as_ptr(), mlen, kp.pk_r.as_ptr()),
            )
        };
        eq(&format!("crypto_sign_verify(mlen={mlen})"), i, a, b);
        eq(&format!("crypto_sign_verify(mlen={mlen})/accepts"), i, a, 0);
    }
}

#[test]
fn cfg_76_77_sign_open_roundtrip() {
    let p = libs();
    let sc_: FnSign = p.c.f("crypto_sign");
    let sr_: FnSign = p.r.f("crypto_sign");
    let oc_: FnSignOpen = p.c.f("crypto_sign_open");
    let or_: FnSignOpen = p.r.f("crypto_sign_open");

    let mut rng = Rng::for_row(76);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let kp = seed_keypair(&seed, 0);

    for (i, mlen) in [0usize, 1, 33, 257].into_iter().enumerate() {
        let mv = rng.bytes(mlen.max(1));
        let m = &mv[..mlen];
        let entropy = rng.bytes(48);
        seed_both_drbgs(&entropy);

        let mut smc = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut smr = smc.clone();
        let mut lc = 0u64;
        let mut lr = 0u64;
        let (rc, rr) = unsafe {
            (
                sc_(
                    smc.as_mut_ptr(),
                    &mut lc,
                    m.as_ptr(),
                    mlen as u64,
                    kp.sk_c.as_ptr(),
                ),
                sr_(
                    smr.as_mut_ptr(),
                    &mut lr,
                    m.as_ptr(),
                    mlen as u64,
                    kp.sk_r.as_ptr(),
                ),
            )
        };
        eq("crypto_sign/retval", i, rc, rr);
        eq("crypto_sign/smlen", i, lc, lr);
        eq(
            "crypto_sign/smlen==SPX_BYTES+mlen",
            i,
            lc as usize,
            SPX_BYTES + mlen,
        );
        eq_bytes("crypto_sign/sm", i, &smc, &smr);

        // Row 77: open.
        let mut mc = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut mr = mc.clone();
        let mut nc = 0u64;
        let mut nr = 0u64;
        let (rc, rr) = unsafe {
            (
                oc_(mc.as_mut_ptr(), &mut nc, smc.as_ptr(), lc, kp.pk_c.as_ptr()),
                or_(mr.as_mut_ptr(), &mut nr, smr.as_ptr(), lr, kp.pk_r.as_ptr()),
            )
        };
        eq("crypto_sign_open/retval", i, rc, rr);
        eq("crypto_sign_open/accepts", i, rc, 0);
        eq("crypto_sign_open/mlen", i, nc, nr);
        eq("crypto_sign_open/mlen==mlen", i, nc as usize, mlen);
        eq_bytes("crypto_sign_open/m", i, &mc, &mr);
        eq_bytes("crypto_sign_open/recovers message", i, &mc[..mlen], m);
    }
}

#[test]
fn cfg_78_cross_implementation_sign_verify() {
    let p = libs();
    let vc: FnVerify = p.c.f("crypto_sign_verify");
    let vr: FnVerify = p.r.f("crypto_sign_verify");
    let mut rng = Rng::for_row(78);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let kp = seed_keypair(&seed, 0);
    for i in 0..HEAVY_ITERS {
        let mlen = (rng.next_u32() % 200) as usize;
        let mv = rng.bytes(mlen.max(1));
        let m = &mv[..mlen];
        let entropy = rng.bytes(48);
        let (sc, sr) = sign_both(&kp, m, &entropy, i);
        // C-signed verified by Rust, and Rust-signed verified by C.
        let a = unsafe { vr(sc.as_ptr(), sc.len(), m.as_ptr(), mlen, kp.pk_r.as_ptr()) };
        let b = unsafe { vc(sr.as_ptr(), sr.len(), m.as_ptr(), mlen, kp.pk_c.as_ptr()) };
        eq("cross: Rust verifies C signature", i, a, 0);
        eq("cross: C verifies Rust signature", i, b, 0);
    }
}

/* ================================================================== */
/* Rows 79-88 — deterministic DRBG (app/src/rng.c)                    */
/* ================================================================== */

fn drbg_state() -> (Vec<u8>, Vec<u8>) {
    let p = libs();
    unsafe {
        (
            p.c.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec(),
            p.r.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec(),
        )
    }
}

fn randombytes_init_row(row: u64, with_pers: bool) {
    let p = libs();
    let c: FnRandombytesInit = p.c.f("randombytes_init");
    let r: FnRandombytesInit = p.r.f("randombytes_init");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let entropy = match i % 3 {
            0 => vec![0u8; 48],
            1 => vec![0xFFu8; 48],
            _ => rng.bytes(48),
        };
        let mut pers = rng.bytes(48);
        let mut ec = entropy.clone();
        let mut er = entropy.clone();
        unsafe {
            if with_pers {
                c(ec.as_mut_ptr(), pers.as_mut_ptr());
                r(er.as_mut_ptr(), pers.as_mut_ptr());
            } else {
                c(ec.as_mut_ptr(), core::ptr::null_mut());
                r(er.as_mut_ptr(), core::ptr::null_mut());
            }
        }
        let (sc, sr) = drbg_state();
        eq_bytes("randombytes_init/DRBG_ctx", i, &sc, &sr);
        // The C never modifies its inputs.
        eq_bytes("randombytes_init/entropy untouched", i, &ec, &entropy);
        let _ = &mut pers;
    }
}

#[test]
fn cfg_79_randombytes_init_null_personalization() {
    randombytes_init_row(79, false);
}

#[test]
fn cfg_80_randombytes_init_with_personalization() {
    randombytes_init_row(80, true);
}

#[test]
fn cfg_81_randombytes_lengths() {
    let p = libs();
    let c: FnRandombytes = p.c.f("randombytes");
    let r: FnRandombytes = p.r.f("randombytes");
    let mut rng = Rng::for_row(81);
    let mut lens: Vec<usize> = vec![1, 15, 16, 17, 31, 32, 33, 48, 100];
    for _ in 0..6 {
        lens.push((rng.next_u32() % 4096) as usize + 1);
    }
    for (i, xlen) in lens.into_iter().enumerate() {
        let entropy = rng.bytes(48);
        seed_both_drbgs(&entropy);
        let mut oc = vec![0xA5u8; xlen + 8];
        let mut or = oc.clone();
        let (rc, rr) = unsafe {
            (
                c(oc.as_mut_ptr(), xlen as u64),
                r(or.as_mut_ptr(), xlen as u64),
            )
        };
        eq(&format!("randombytes(xlen={xlen})/retval"), i, rc, rr);
        eq(&format!("randombytes(xlen={xlen})/retval==0"), i, rc, 0);
        eq_bytes(&format!("randombytes(xlen={xlen})/out"), i, &oc, &or);
        let (sc, sr) = drbg_state();
        eq_bytes(&format!("randombytes(xlen={xlen})/DRBG_ctx"), i, &sc, &sr);
    }
}

#[test]
fn cfg_82_randombytes_state_chaining() {
    let p = libs();
    let c: FnRandombytes = p.c.f("randombytes");
    let r: FnRandombytes = p.r.f("randombytes");
    let mut rng = Rng::for_row(82);
    let entropy = rng.bytes(48);
    seed_both_drbgs(&entropy);
    for i in 0..10 {
        let xlen = (rng.next_u32() % 300) as usize + 1;
        let mut oc = vec![0xA5u8; xlen + 8];
        let mut or = oc.clone();
        unsafe {
            c(oc.as_mut_ptr(), xlen as u64);
            r(or.as_mut_ptr(), xlen as u64);
        }
        eq_bytes("randombytes/chained out", i, &oc, &or);
        let (sc, sr) = drbg_state();
        eq_bytes("randombytes/chained DRBG_ctx", i, &sc, &sr);
    }
}

#[test]
fn cfg_83_aes256_ecb() {
    let p = libs();
    let c: FnAes256Ecb = p.c.f("AES256_ECB");
    let r: FnAes256Ecb = p.r.f("AES256_ECB");
    let mut rng = Rng::for_row(83);
    for i in 0..ITERS {
        let mut key = match i % 3 {
            0 => vec![0u8; 32],
            1 => vec![0xFFu8; 32],
            _ => rng.bytes(32),
        };
        let mut ctr = match i % 4 {
            0 => vec![0u8; 16],
            1 => vec![0xFFu8; 16],
            _ => rng.bytes(16),
        };
        let mut oc = vec![0xA5u8; 16 + 8];
        let mut or = oc.clone();
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), oc.as_mut_ptr());
            r(key.as_mut_ptr(), ctr.as_mut_ptr(), or.as_mut_ptr());
        }
        eq_bytes("AES256_ECB", i, &oc, &or);
    }
}

fn drbg_update_row(row: u64, mode: u8) {
    let p = libs();
    let c: FnDrbgUpdate = p.c.f("AES256_CTR_DRBG_Update");
    let r: FnDrbgUpdate = p.r.f("AES256_CTR_DRBG_Update");
    let mut rng = Rng::for_row(row);
    for i in 0..ITERS {
        let key = rng.bytes(32);
        let v = if mode == 2 {
            vec![0xFFu8; 16]
        } else {
            rng.bytes(16)
        };
        let mut pd = rng.bytes(48);
        let mut kc = key.clone();
        let mut kr = key.clone();
        let mut vc = v.clone();
        let mut vr = v.clone();
        unsafe {
            if mode == 1 {
                c(pd.as_mut_ptr(), kc.as_mut_ptr(), vc.as_mut_ptr());
                r(pd.as_mut_ptr(), kr.as_mut_ptr(), vr.as_mut_ptr());
            } else {
                c(
                    core::ptr::null_mut(),
                    kc.as_mut_ptr(),
                    vc.as_mut_ptr(),
                );
                r(
                    core::ptr::null_mut(),
                    kr.as_mut_ptr(),
                    vr.as_mut_ptr(),
                );
            }
        }
        eq_bytes("AES256_CTR_DRBG_Update/Key", i, &kc, &kr);
        eq_bytes("AES256_CTR_DRBG_Update/V", i, &vc, &vr);
    }
}

#[test]
fn cfg_84_drbg_update_null_provided_data() {
    drbg_update_row(84, 0);
}

#[test]
fn cfg_85_drbg_update_with_provided_data() {
    drbg_update_row(85, 1);
}

#[test]
fn cfg_86_drbg_update_counter_rollover() {
    drbg_update_row(86, 2);
}

#[test]
fn cfg_87_seedexpander() {
    let p = libs();
    let ic: FnSeedexpanderInit = p.c.f("seedexpander_init");
    let ir: FnSeedexpanderInit = p.r.f("seedexpander_init");
    let ec: FnSeedexpander = p.c.f("seedexpander");
    let er: FnSeedexpander = p.r.f("seedexpander");
    let mut rng = Rng::for_row(87);
    for (i, &maxlen) in [16u64, 256, 4096, 0xFFFF_FFFF].iter().enumerate() {
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut sc = AesXofStruct::default();
        let mut sr = AesXofStruct::default();
        let (rc, rr) = unsafe {
            (
                ic(
                    &mut sc,
                    seed.as_mut_ptr(),
                    div.as_mut_ptr(),
                    maxlen as core::ffi::c_ulong,
                ),
                ir(
                    &mut sr,
                    seed.as_mut_ptr(),
                    div.as_mut_ptr(),
                    maxlen as core::ffi::c_ulong,
                ),
            )
        };
        eq("seedexpander_init/retval", i, rc, rr);
        eq("seedexpander_init/retval==0", i, rc, 0);
        eq_bytes("seedexpander_init/ctx", i, &sc.bytes(), &sr.bytes());

        for &xlen in &[1usize, 15, 16, 17, 33] {
            if (xlen as u64) >= maxlen {
                continue;
            }
            let mut oc = vec![0xA5u8; xlen + 8];
            let mut or = oc.clone();
            let (rc, rr) = unsafe {
                (
                    ec(&mut sc, oc.as_mut_ptr(), xlen as core::ffi::c_ulong),
                    er(&mut sr, or.as_mut_ptr(), xlen as core::ffi::c_ulong),
                )
            };
            eq(&format!("seedexpander(xlen={xlen})/retval"), i, rc, rr);
            eq_bytes(&format!("seedexpander(xlen={xlen})/out"), i, &oc, &or);
            eq_bytes(
                &format!("seedexpander(xlen={xlen})/ctx"),
                i,
                &sc.bytes(),
                &sr.bytes(),
            );
        }
    }
}

#[test]
fn cfg_88_seedexpander_counter_rollover() {
    let p = libs();
    let ic: FnSeedexpanderInit = p.c.f("seedexpander_init");
    let ir: FnSeedexpanderInit = p.r.f("seedexpander_init");
    let ec: FnSeedexpander = p.c.f("seedexpander");
    let er: FnSeedexpander = p.r.f("seedexpander");
    let mut rng = Rng::for_row(88);
    let mut seed = rng.bytes(32);
    // seedexpander_init writes maxlen into ctr[8..12] and zeroes ctr[12..16],
    // so the block counter always starts at 0; force it to 0xFF..FF afterwards
    // to exercise the carry chain, and also just call it many times.
    let mut div = rng.bytes(8);
    let mut sc = AesXofStruct::default();
    let mut sr = AesXofStruct::default();
    unsafe {
        ic(
            &mut sc,
            seed.as_mut_ptr(),
            div.as_mut_ptr(),
            0xFFFF_FFFF as core::ffi::c_ulong,
        );
        ir(
            &mut sr,
            seed.as_mut_ptr(),
            div.as_mut_ptr(),
            0xFFFF_FFFF as core::ffi::c_ulong,
        );
    }
    // Drive the last counter byte up to 0xFF and past it.
    sc.ctr[12..16].copy_from_slice(&[0x00, 0x00, 0x00, 0xFD]);
    sr.ctr[12..16].copy_from_slice(&[0x00, 0x00, 0x00, 0xFD]);
    for i in 0..40 {
        let xlen = 33usize;
        let mut oc = vec![0xA5u8; xlen + 8];
        let mut or = oc.clone();
        let (rc, rr) = unsafe {
            (
                ec(&mut sc, oc.as_mut_ptr(), xlen as core::ffi::c_ulong),
                er(&mut sr, or.as_mut_ptr(), xlen as core::ffi::c_ulong),
            )
        };
        eq("seedexpander/rollover retval", i, rc, rr);
        eq_bytes("seedexpander/rollover out", i, &oc, &or);
        eq_bytes(
            "seedexpander/rollover ctx",
            i,
            &sc.bytes(),
            &sr.bytes(),
        );
    }
    // And explicitly the all-0xFF carry case.
    sc.ctr[12..16].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    sr.ctr[12..16].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    let mut oc = vec![0xA5u8; 40];
    let mut or = oc.clone();
    unsafe {
        ec(&mut sc, oc.as_mut_ptr(), 32);
        er(&mut sr, or.as_mut_ptr(), 32);
    }
    eq_bytes("seedexpander/full carry out", 0, &oc, &or);
    eq_bytes("seedexpander/full carry ctx", 0, &sc.bytes(), &sr.bytes());
}
