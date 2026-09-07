//! Phase B, level 1 — `hash.h` / `thash.h` (`CONFIGS.md` rows C26–C31).

mod common;
use common::*;

type InitHash = unsafe extern "C" fn(*mut u8);
type PrfAddr = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type GenMsgRandom = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type HashMessage =
    unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8);
type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);

/* ---- C26 --------------------------------------------------------- */

#[test]
fn c26_initialize_hash_function() {
    let l = libs();
    let (c, r) = l.pair::<InitHash>("SPX_initialize_hash_function");
    let mut rng = Rng::new(SEED + 26);
    for i in 0..NUM_ITERS {
        let (pub_seed, sk_seed) = match i {
            0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
            1 => (vec![0xffu8; SPX_N], vec![0xffu8; SPX_N]),
            2 => (vec![0u8; SPX_N], vec![0xffu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let mut cc = Ctx::new();
        let mut rc = Ctx::new();
        cc.set_seeds(&pub_seed, &sk_seed);
        rc.set_seeds(&pub_seed, &sk_seed);
        unsafe {
            c(cc.as_mut_ptr());
            r(rc.as_mut_ptr());
        }
        eq_bytes("initialize_hash_function spx_ctx image", cc.bytes(), rc.bytes());
    }
}

/* ---- C27 --------------------------------------------------------- */

#[test]
fn c27_prf_addr() {
    let l = libs();
    let (c, r) = l.pair::<PrfAddr>("SPX_prf_addr");
    let (cset, rset) = l.pair::<unsafe extern "C" fn(*mut u32, u32)>("SPX_set_type");
    let mut rng = Rng::new(SEED + 27);
    for i in 0..NUM_ITERS {
        let (pub_seed, sk_seed) = match i {
            0 => (vec![0u8; SPX_N], vec![0u8; SPX_N]),
            1 => (vec![0xffu8; SPX_N], vec![0xffu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cc, rc) = init_ctx_pair(&pub_seed, &sk_seed);
        for ty in ADDR_TYPES {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                cset(ca.as_mut_ptr(), ty);
                rset(ra.as_mut_ptr(), ty);
            }
            // guard bytes: prf_addr must write exactly SPX_N
            let mut co = vec![0xAAu8; SPX_N + 16];
            let mut ro = vec![0xAAu8; SPX_N + 16];
            unsafe {
                c(co.as_mut_ptr(), cc.as_ptr(), ca.as_ptr());
                r(ro.as_mut_ptr(), rc.as_ptr(), ra.as_ptr());
            }
            eq_bytes(&format!("prf_addr(type={ty})"), &co, &ro);
            assert_eq!(&co[SPX_N..], &[0xAAu8; 16], "prf_addr wrote past SPX_N");
        }
    }
}

/* ---- C28 --------------------------------------------------------- */

#[test]
fn c28_gen_message_random() {
    let l = libs();
    let (c, r) = l.pair::<GenMsgRandom>("SPX_gen_message_random");
    let mut rng = Rng::new(SEED + 28);
    let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for mlen in mlen_cases() {
        for i in 0..4 {
            let sk_prf = match i {
                0 => vec![0u8; SPX_N],
                1 => vec![0xffu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let optrand = rng.bytes(SPX_N);
            let m = rng.bytes(mlen.max(1)); // never a dangling pointer
            // NOTE: hash_blake.c's gen_message_random ends in
            // `blakeX_final(&S, R)`, which writes the FULL 32/64-byte digest
            // into R, not just SPX_N bytes.  That is the C's behaviour and the
            // Rust must reproduce it, so the whole 64-byte tail is compared
            // rather than asserted untouched.
            let mut co = vec![0xAAu8; SPX_N + 64];
            let mut ro = vec![0xAAu8; SPX_N + 64];
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

/* ---- C29 --------------------------------------------------------- */

#[test]
fn c29_hash_message() {
    let l = libs();
    let (c, r) = l.pair::<HashMessage>("SPX_hash_message");
    let mut rng = Rng::new(SEED + 29);
    let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for mlen in mlen_cases() {
        for i in 0..4 {
            let rr = rng.bytes(SPX_N);
            let pk = match i {
                0 => vec![0u8; SPX_PK_BYTES],
                1 => vec![0xffu8; SPX_PK_BYTES],
                _ => rng.bytes(SPX_PK_BYTES),
            };
            let m = rng.bytes(mlen.max(1));
            let mut cd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 16];
            let mut rd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 16];
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
            let tag = format!("hash_message(mlen={mlen})");
            eq_bytes(&format!("{tag} digest"), &cd, &rd);
            assert_eq!(ct, rt, "{tag} tree");
            assert_eq!(cl, rl, "{tag} leaf_idx");
            assert_eq!(
                &cd[SPX_FORS_MSG_BYTES..],
                &[0xAAu8; 16],
                "hash_message wrote past SPX_FORS_MSG_BYTES"
            );
            // The C masks tree/leaf_idx to the parameter set's bit widths.
            let tree_bits = SPX_TREE_HEIGHT * (SPX_D - 1);
            assert!(
                tree_bits >= 64 || ct >> tree_bits == 0,
                "{tag} tree not masked to {tree_bits} bits"
            );
            assert!(cl >> SPX_TREE_HEIGHT == 0, "{tag} leaf_idx not masked");
        }
    }
}

/* ---- C30 --------------------------------------------------------- */

#[test]
fn c30_thash() {
    let l = libs();
    let (c, r) = l.pair::<Thash>("SPX_thash");
    let (cset, rset) = l.pair::<unsafe extern "C" fn(*mut u32, u32)>("SPX_set_type");
    let mut rng = Rng::new(SEED + 30);

    let mut blocks = vec![1u32, 2, 3];
    blocks.push(SPX_FORS_TREES);
    blocks.push(SPX_WOTS_LEN as u32);
    blocks.sort_unstable();
    blocks.dedup();

    for &inblocks in &blocks {
        for ty in ADDR_TYPES {
            for _ in 0..8 {
                let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
                let inp = rng.bytes(inblocks as usize * SPX_N);
                let base = rng.addr();
                let mut ca = base;
                let mut ra = base;
                unsafe {
                    cset(ca.as_mut_ptr(), ty);
                    rset(ra.as_mut_ptr(), ty);
                }
                let mut co = vec![0xAAu8; SPX_N + 16];
                let mut ro = vec![0xAAu8; SPX_N + 16];
                unsafe {
                    c(co.as_mut_ptr(), inp.as_ptr(), inblocks, cc.as_ptr(), ca.as_mut_ptr());
                    r(ro.as_mut_ptr(), inp.as_ptr(), inblocks, rc.as_ptr(), ra.as_mut_ptr());
                }
                eq_bytes(&format!("thash(inblocks={inblocks},type={ty})"), &co, &ro);
                assert_eq!(&co[SPX_N..], &[0xAAu8; 16], "thash wrote past SPX_N");
                eq_bytes(
                    "thash must not mutate addr",
                    unsafe { std::slice::from_raw_parts(ca.as_ptr() as *const u8, 32) },
                    unsafe { std::slice::from_raw_parts(ra.as_ptr() as *const u8, 32) },
                );
            }
        }
    }
}

/* ---- C31 --------------------------------------------------------- */

#[test]
fn c31_thash_edge_seeds() {
    let l = libs();
    let (c, r) = l.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(SEED + 31);
    for seed_pat in [Some(0x00u8), Some(0xffu8), None] {
        let pub_seed = match seed_pat {
            Some(p) => vec![p; SPX_N],
            None => rng.bytes(SPX_N),
        };
        let sk_seed = match seed_pat {
            Some(p) => vec![p; SPX_N],
            None => rng.bytes(SPX_N),
        };
        let (cc, rc) = init_ctx_pair(&pub_seed, &sk_seed);
        for ty in ADDR_TYPES {
            for inp_pat in [Some(0x00u8), Some(0xffu8), None] {
                let inp = match inp_pat {
                    Some(p) => vec![p; SPX_N],
                    None => rng.bytes(SPX_N),
                };
                let mut addr = [0u32; 8];
                unsafe {
                    (addr.as_mut_ptr() as *mut u8).add(off::TYPE).write(ty as u8);
                }
                let mut ca = addr;
                let mut ra = addr;
                let mut co = vec![0u8; SPX_N];
                let mut ro = vec![0u8; SPX_N];
                unsafe {
                    c(co.as_mut_ptr(), inp.as_ptr(), 1, cc.as_ptr(), ca.as_mut_ptr());
                    r(ro.as_mut_ptr(), inp.as_ptr(), 1, rc.as_ptr(), ra.as_mut_ptr());
                }
                eq_bytes(&format!("thash edge (type={ty}, seed={seed_pat:?})"), &co, &ro);
            }
        }
    }
}
