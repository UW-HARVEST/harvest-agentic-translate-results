//! Phase B, CONFIGS.md rows 8-11: the hash-function hooks of
//! `lib/<backend>/src/hash_<backend>.c`.

mod common;

use common::params::*;
use common::*;

type PrfAddr = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type GenMsgRand = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type HashMessage = unsafe extern "C" fn(
    *mut u8,   // digest
    *mut u64,  // tree
    *mut u32,  // leaf_idx
    *const u8, // R
    *const u8, // pk
    *const u8, // m
    u64,       // mlen
    *const u8, // ctx
);

#[test]
fn row08_initialize_hash_function() {
    let libs = load();
    let mut rng = Rng::new(8);
    // make_ctx_pair already asserts the two spx_ctx byte images agree, which is
    // the whole content of this row for sha2 (state_seeded[_512]) and haraka
    // (tweaked512_rc64 / tweaked256_rc32).
    for _ in 0..64 {
        let ps = rng.bytes(SPX_N);
        let ss = rng.bytes(SPX_N);
        let _ = make_ctx_pair(&libs, &ps, &ss);
    }
    for (ps, ss) in [
        (vec![0u8; SPX_N], vec![0u8; SPX_N]),
        (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
        (vec![0u8; SPX_N], vec![0xFFu8; SPX_N]),
    ] {
        let _ = make_ctx_pair(&libs, &ps, &ss);
    }
    eprintln!("[{}] spx_ctx is {} bytes", tag(), CTX_SIZE);
}

#[test]
fn row09_prf_addr() {
    let libs = load();
    let (fc, fr) = libs.pair::<PrfAddr>("SPX_prf_addr");
    let mut rng = Rng::new(9);
    for round in 0..128 {
        let ps = rng.bytes(SPX_N);
        let ss = rng.bytes(SPX_N);
        let (cc, cr) = make_ctx_pair(&libs, &ps, &ss);
        for _ in 0..8 {
            let addr = if round == 0 { [0u32; 8] } else { rng.addr() };
            let mut a = vec![0xA5u8; SPX_N + 8];
            let mut b = vec![0xA5u8; SPX_N + 8];
            unsafe {
                fc(a.as_mut_ptr(), cc.ptr(), addr.as_ptr());
                fr(b.as_mut_ptr(), cr.ptr(), addr.as_ptr());
            }
            eq("SPX_prf_addr", &a, &b);
        }
    }
    // address type sweep 0..=6 (all SPX_ADDR_TYPE_* variants)
    let ps = rng.bytes(SPX_N);
    let ss = rng.bytes(SPX_N);
    let (cc, cr) = make_ctx_pair(&libs, &ps, &ss);
    let set_type = libs.pair::<unsafe extern "C" fn(*mut u32, u32)>("SPX_set_type");
    for ty in 0..=6u32 {
        let mut addr = rng.addr();
        unsafe { set_type.0(addr.as_mut_ptr(), ty) };
        let mut a = vec![0u8; SPX_N];
        let mut b = vec![0u8; SPX_N];
        unsafe {
            fc(a.as_mut_ptr(), cc.ptr(), addr.as_ptr());
            fr(b.as_mut_ptr(), cr.ptr(), addr.as_ptr());
        }
        eq(&format!("SPX_prf_addr type={ty}"), &a, &b);
    }
}

#[test]
fn row10_gen_message_random() {
    let libs = load();
    let (fc, fr) = libs.pair::<GenMsgRand>("SPX_gen_message_random");
    let mut rng = Rng::new(10);
    let ps = rng.bytes(SPX_N);
    let ss = rng.bytes(SPX_N);
    let (cc, cr) = make_ctx_pair(&libs, &ps, &ss);

    // `hash_blake.c`'s gen_message_random ends in `blakeX_final(&S, R)`, which
    // writes SPX_BLAKE{256,512}_OUTPUT_BYTES — *more* than SPX_N.  `sign.c` gets
    // away with it because R is the head of an SPX_BYTES signature buffer.  Use
    // a generous sentinel-filled buffer and compare all of it, so the number of
    // bytes each side writes is part of the comparison.
    for &mlen in MLEN_SWEEP {
        for rep in 0..4 {
            let sk_prf = rng.bytes(SPX_N);
            let optrand = rng.bytes(SPX_N);
            let m = match rep {
                0 => vec![0u8; mlen],
                1 => vec![0xFFu8; mlen],
                _ => rng.bytes(mlen),
            };
            let mut a = vec![0xA5u8; 256];
            let mut b = vec![0xA5u8; 256];
            unsafe {
                fc(
                    a.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.ptr(),
                );
                fr(
                    b.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cr.ptr(),
                );
            }
            eq(
                &format!("SPX_gen_message_random(mlen={mlen}, rep={rep})"),
                &a,
                &b,
            );
        }
    }
}

#[test]
fn row11_hash_message() {
    let libs = load();
    let (fc, fr) = libs.pair::<HashMessage>("SPX_hash_message");
    let mut rng = Rng::new(11);
    let ps = rng.bytes(SPX_N);
    let ss = rng.bytes(SPX_N);
    let (cc, cr) = make_ctx_pair(&libs, &ps, &ss);

    for &mlen in MLEN_SWEEP {
        for rep in 0..4 {
            let r = rng.bytes(SPX_N);
            let pk = rng.bytes(SPX_PK_BYTES);
            let m = match rep {
                0 => vec![0u8; mlen],
                1 => vec![0xFFu8; mlen],
                _ => rng.bytes(mlen),
            };
            let mut da = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
            let mut db = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
            let mut ta = 0xDEAD_BEEF_DEAD_BEEFu64;
            let mut tb = 0xDEAD_BEEF_DEAD_BEEFu64;
            let mut la = 0xDEAD_BEEFu32;
            let mut lb = 0xDEAD_BEEFu32;
            unsafe {
                fc(
                    da.as_mut_ptr(),
                    &mut ta,
                    &mut la,
                    r.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cc.ptr(),
                );
                fr(
                    db.as_mut_ptr(),
                    &mut tb,
                    &mut lb,
                    r.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cr.ptr(),
                );
            }
            eq(&format!("SPX_hash_message digest(mlen={mlen})"), &da, &db);
            assert_eq!(ta, tb, "SPX_hash_message tree (mlen={mlen})");
            assert_eq!(la, lb, "SPX_hash_message leaf_idx (mlen={mlen})");
            // The C masks both outputs; check the invariants that mask implies.
            let tree_bits = SPX_TREE_HEIGHT * (SPX_D - 1);
            assert!(
                tree_bits >= 64 || ta < (1u64 << tree_bits),
                "tree {ta:#x} exceeds {tree_bits} bits"
            );
            assert!(
                la < (1u32 << SPX_TREE_HEIGHT),
                "leaf_idx {la} exceeds {SPX_TREE_HEIGHT} bits"
            );
        }
    }
}
