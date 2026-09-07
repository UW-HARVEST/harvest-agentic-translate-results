//! CONFIGS.md group C — the backend hooks declared in `app/include/hash.h`
//! and `app/include/thash.h`, i.e. the level `sign.c` calls into.

mod common;

use common::*;
use std::os::raw::c_uint;

type InitFn = unsafe extern "C" fn(*mut u8);
type PrfFn = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type ThashFn = unsafe extern "C" fn(*mut u8, *const u8, c_uint, *const u8, *mut u32);
type GenMsgRandFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type HashMsgFn =
    unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8);

#[test]
fn c1_initialize_hash_function() {
    let libs = Libs::load();
    let (c, r) = pair_backend!(libs, "SPX_initialize_hash_function", InitFn);
    let mut rng = Rng::new(501);
    for i in 0..300 {
        let (ps, sk): (Vec<u8>, Vec<u8>) = match i {
            0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
            1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
            2 => (vec![0u8; SPX_N], vec![0xFFu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let mut cc = Ctx::new();
        let mut rc = Ctx::new();
        for ctx in [&mut cc, &mut rc] {
            ctx.set_pub_seed(&ps);
            ctx.set_sk_seed(&sk);
        }
        unsafe {
            c(cc.as_mut_ptr());
            r(rc.as_mut_ptr());
        }
        eq_bytes("initialize_hash_function ctx", cc.bytes(), rc.bytes());
    }
}

#[test]
fn c2_c3_prf_addr() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_prf_addr", PrfFn);
    let mut rng = Rng::new(502);
    let ps = rng.bytes(SPX_N);
    let sk = rng.bytes(SPX_N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

    // C2: every SPX_ADDR_TYPE_*.  C3: all-zero and all-0xFF addresses.
    let (set_c, set_r) = pair!(libs, "SPX_set_type", unsafe extern "C" fn(*mut u32, u32));
    for ty in 0u32..=6 {
        for i in 0..60 {
            let mut addr = match i {
                0 => [0u32; 8],
                1 => [0xFFFF_FFFFu32; 8],
                _ => rng.addr(),
            };
            let mut ca = addr;
            let mut ra = addr;
            unsafe {
                set_c(ca.as_mut_ptr(), ty);
                set_r(ra.as_mut_ptr(), ty);
            }
            eq_bytes("set_type parity", &u32s_to_bytes(&ca), &u32s_to_bytes(&ra));
            addr = ca;

            let mut co = vec![0xAAu8; SPX_N + 8];
            let mut ro = vec![0xAAu8; SPX_N + 8];
            unsafe {
                c(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
                r(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr());
            }
            eq_bytes(&format!("prf_addr(type={ty})"), &co, &ro);
        }
    }

    // Also with fresh contexts each round, so pub_seed/sk_seed vary.
    for _ in 0..100 {
        let ps = rng.bytes(SPX_N);
        let sk = rng.bytes(SPX_N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);
        let addr = rng.addr();
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
            r(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr());
        }
        eq_bytes("prf_addr(random ctx)", &co, &ro);
    }
}

#[test]
fn c4_c8_thash() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "SPX_thash", ThashFn);
    let mut rng = Rng::new(503);
    let ps = rng.bytes(SPX_N);
    let sk = rng.bytes(SPX_N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

    // C4: 1 (F path).  C5: 2 (H path -> 512-bit primitive when N >= 24).
    // C6: SPX_WOTS_LEN.  C7: SPX_FORS_TREES.  C8: intermediate widths.
    let mut widths: Vec<usize> = vec![1, 2, 3, 4, 8, 16, SPX_WOTS_LEN, SPX_FORS_TREES];
    widths.sort_unstable();
    widths.dedup();

    for &nb in &widths {
        for i in 0..60 {
            let inp: Vec<u8> = match i {
                0 => vec![0u8; nb * SPX_N],
                1 => vec![0xFFu8; nb * SPX_N],
                _ => rng.bytes(nb * SPX_N),
            };
            let addr0 = if i == 0 { [0u32; 8] } else { rng.addr() };
            let mut ca = addr0;
            let mut ra = addr0;
            let mut co = vec![0xAAu8; SPX_N + 8];
            let mut ro = vec![0xAAu8; SPX_N + 8];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), nb as c_uint, cc.as_ptr(), ca.as_mut_ptr());
                r(ro.as_mut_ptr(), inp.as_ptr(), nb as c_uint, rc.as_ptr(), ra.as_mut_ptr());
            }
            eq_bytes(&format!("thash(inblocks={nb}) out"), &co, &ro);
            eq_bytes(
                &format!("thash(inblocks={nb}) addr"),
                &u32s_to_bytes(&ca),
                &u32s_to_bytes(&ra),
            );
        }
    }
}

/// Message lengths that straddle every branch/block boundary any backend has.
fn msg_lens() -> Vec<usize> {
    let mut v: Vec<usize> = vec![
        0, 1, 2, 15, 16, 31, 32, 33, 63, 64, 65, 111, 112, 127, 128, 129, 135, 136, 137, 271, 272,
        1000, 5000,
    ];
    // gen_message_random split in hash_sha2.c: SPX_N + mlen < SHAX_BLOCK_BYTES
    let split = SHAX_BLOCK_BYTES.saturating_sub(SPX_N);
    v.push(split.saturating_sub(1));
    v.push(split);
    v.push(split + 1);
    // hash_message split: SPX_N + SPX_PK_BYTES + mlen < INBLOCKS*BLOCK
    let hsplit = (SHA2_INBLOCKS * SHAX_BLOCK_BYTES).saturating_sub(SPX_N + SPX_PK_BYTES);
    v.push(hsplit.saturating_sub(1));
    v.push(hsplit);
    v.push(hsplit + 1);
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn c9_c12_gen_message_random() {
    let libs = Libs::load();
    let (c, r) = pair_backend!(libs, "SPX_gen_message_random", GenMsgRandFn);
    let mut rng = Rng::new(504);
    let ps = rng.bytes(SPX_N);
    let sk = rng.bytes(SPX_N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

    for mlen in msg_lens() {
        for i in 0..20 {
            let sk_prf = rng.bytes(SPX_N);
            let optrand = rng.bytes(SPX_N);
            let m: Vec<u8> = match i {
                0 => vec![0u8; mlen.max(1)],
                1 => vec![0xFFu8; mlen.max(1)],
                _ => rng.bytes(mlen.max(1)),
            };
            // `hash_blake.c`'s gen_message_random does
            // `blakeX_final(&S, R)`, which writes the FULL 32- (blake256) or
            // 64-byte (blake512) digest to `R`, not just SPX_N bytes.  In
            // `sign.c` R is `sig`, so there is room.  A generous buffer both
            // avoids clobbering the test's stack and lets the extra bytes be
            // compared too.
            let mut co = vec![0xAAu8; 128];
            let mut ro = vec![0xAAu8; 128];
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
            eq_bytes(&format!("gen_message_random(mlen={mlen})"), &co, &ro);
        }
    }
}

#[test]
fn c13_c16_hash_message() {
    let libs = Libs::load();
    let (c, r) = pair_backend!(libs, "SPX_hash_message", HashMsgFn);
    let mut rng = Rng::new(505);
    let ps = rng.bytes(SPX_N);
    let sk = rng.bytes(SPX_N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

    let tree_mask: u64 = u64::MAX >> (64 - SPX_TREE_BITS);
    let leaf_mask: u32 = u32::MAX >> (32 - SPX_TREE_HEIGHT);

    for mlen in msg_lens() {
        for i in 0..20 {
            let rr = rng.bytes(SPX_N);
            let pk = rng.bytes(SPX_PK_BYTES);
            let m: Vec<u8> = match i {
                0 => vec![0u8; mlen.max(1)],
                1 => vec![0xFFu8; mlen.max(1)],
                _ => rng.bytes(mlen.max(1)),
            };
            let mut cd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
            let mut rd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
            let mut ct: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            let mut rt: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            let mut cl: u32 = 0xDEAD_BEEF;
            let mut rl: u32 = 0xDEAD_BEEF;
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
            eq_bytes(&format!("hash_message(mlen={mlen}) digest"), &cd, &rd);
            eq(&format!("hash_message(mlen={mlen}) tree"), ct, rt);
            eq(&format!("hash_message(mlen={mlen}) leaf_idx"), cl, rl);
            // The masks documented in hash_<backend>.c must hold.
            assert_eq!(ct & !tree_mask, 0, "tree not masked to {SPX_TREE_BITS} bits");
            assert_eq!(cl & !leaf_mask, 0, "leaf_idx not masked to {SPX_TREE_HEIGHT} bits");
        }
    }
}
