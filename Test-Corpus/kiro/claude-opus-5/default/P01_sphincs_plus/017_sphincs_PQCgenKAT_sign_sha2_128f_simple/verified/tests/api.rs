//! Phase B, level 4 — `api.h` (`CONFIGS.md` rows C45–C53).
//!
//! `crypto_sign_keypair` and `crypto_sign_signature` call `randombytes`
//! internally.  The Rust cdylib exports the `rng.c` (NIST AES-256-CTR-DRBG)
//! `randombytes`, and the flat C `.so` links the same `rng.c`, so seeding both
//! DRBGs identically with `randombytes_init` makes both fully deterministic and
//! byte-comparable — exactly what `PQCgenKAT_sign.c` relies on.

mod common;
use common::*;

type SizeFn = unsafe extern "C" fn() -> u64;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type Signature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type HashMessage = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u32,
    *const u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
);
type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type MerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
type SetU32 = unsafe extern "C" fn(*mut u32, u32);
type SetU64 = unsafe extern "C" fn(*mut u32, u64);
type InitHash = unsafe extern "C" fn(*mut u8);

/// `entropy_input[i] = i`, the seeding `PQCgenKAT_sign.c` uses.
fn kat_entropy() -> Vec<u8> {
    (0..48u8).collect()
}

/// Seeds both DRBGs identically.
fn seed_both(l: &Libs, entropy: &[u8], pers: Option<&[u8]>) {
    let (c, r) = l.pair::<RandombytesInit>("randombytes_init");
    let mut ce = entropy.to_vec();
    let mut re = entropy.to_vec();
    let mut cp = pers.map(|p| p.to_vec());
    let mut rp = pers.map(|p| p.to_vec());
    unsafe {
        c(
            ce.as_mut_ptr(),
            cp.as_mut().map_or(std::ptr::null_mut(), |v| v.as_mut_ptr()),
        );
        r(
            re.as_mut_ptr(),
            rp.as_mut().map_or(std::ptr::null_mut(), |v| v.as_mut_ptr()),
        );
    }
}

/* ---- C45 --------------------------------------------------------- */

#[test]
fn c45_size_accessors() {
    let l = libs();
    for name in [
        "crypto_sign_secretkeybytes",
        "crypto_sign_publickeybytes",
        "crypto_sign_bytes",
        "crypto_sign_seedbytes",
    ] {
        let (c, r) = l.pair::<SizeFn>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{name}()");
    }
}

/* ---- C46 --------------------------------------------------------- */

#[test]
fn c46_seed_keypair() {
    let l = libs();
    let (c, r) = l.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(SEED + 46);
    for i in 0..NUM_ITERS {
        let seed = match i {
            0 => vec![0u8; CRYPTO_SEEDBYTES],
            1 => vec![0xffu8; CRYPTO_SEEDBYTES],
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        let mut cpk = vec![0xAAu8; SPX_PK_BYTES + 16];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES + 16];
        let mut csk = vec![0xBBu8; SPX_SK_BYTES + 16];
        let mut rsk = vec![0xBBu8; SPX_SK_BYTES + 16];
        let (cv, rv) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_seed_keypair return");
        assert_eq!(cv, 0, "C always returns 0");
        eq_bytes("seed_keypair pk", &cpk, &rpk);
        eq_bytes("seed_keypair sk", &csk, &rsk);
        assert_eq!(&cpk[SPX_PK_BYTES..], &[0xAAu8; 16], "pk overrun");
        assert_eq!(&csk[SPX_SK_BYTES..], &[0xBBu8; 16], "sk overrun");
    }
}

/* ---- C47 --------------------------------------------------------- */

#[test]
fn c47_keypair_via_drbg() {
    let _drbg = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<Keypair>("crypto_sign_keypair");
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");
    let mut rng = Rng::new(SEED + 47);
    for i in 0..8 {
        let entropy = if i == 0 { kat_entropy() } else { rng.bytes(48) };
        let pers: Option<Vec<u8>> = if i % 2 == 0 { None } else { Some(rng.bytes(48)) };
        seed_both(l, &entropy, pers.as_deref());
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        let (cv, rv) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_keypair return");
        eq_bytes("keypair pk", &cpk, &rpk);
        eq_bytes("keypair sk", &csk, &rsk);
        assert_eq!(unsafe { **cg }, unsafe { **rg }, "DRBG_ctx after keypair");
    }
}

/* ---- C48 --------------------------------------------------------- */

#[test]
fn c48_signature_via_drbg() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (c, r) = l.pair::<Signature>("crypto_sign_signature");
    let mut rng = Rng::new(SEED + 48);
    let mut mlens = vec![0usize, 1, 32, 33, 64, 128, 231, 1000];
    mlens.push(GEN_MSG_RANDOM_BOUNDARY);
    mlens.push(HASH_MESSAGE_BOUNDARY);
    mlens.sort_unstable();
    mlens.dedup();

    for mlen in mlens {
        seed_both(l, &kat_entropy(), None);
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        }
        eq_bytes("sk must already agree", &csk, &rsk);
        let m = rng.bytes(mlen.max(1));
        let mut csig = vec![0xAAu8; SPX_BYTES + 16];
        let mut rsig = vec![0xAAu8; SPX_BYTES + 16];
        let mut cl = usize::MAX;
        let mut rl = usize::MAX;
        let (cv, rv) = unsafe {
            (
                c(csig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr()),
                r(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_signature return (mlen={mlen})");
        assert_eq!(cl, rl, "siglen (mlen={mlen})");
        assert_eq!(cl, SPX_BYTES, "siglen must be SPX_BYTES");
        eq_bytes(&format!("signature (mlen={mlen})"), &csig, &rsig);
        assert_eq!(&csig[SPX_BYTES..], &[0xAAu8; 16], "sig overrun");
    }
}

/* ---- C49 --------------------------------------------------------- */

/// Reconstruct `crypto_sign_signature` from the low-level exports in exactly
/// the order `sign.c` uses, and check it reproduces the one-shot output.  Run
/// against both libraries, so a divergence in the *composition* (address
/// plumbing, index shifting between layers) is caught even where every
/// individual primitive agrees.
#[test]
fn c49_signature_lowlevel_recompose() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (csign, rsign) = l.pair::<Signature>("crypto_sign_signature");
    let mut rng = Rng::new(SEED + 49);

    for mlen in [0usize, 1, 33, 231] {
        seed_both(l, &kat_entropy(), None);
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut csig = vec![0u8; SPX_BYTES];
        let mut rsig = vec![0u8; SPX_BYTES];
        let mut cl = 0usize;
        let mut rl = 0usize;
        unsafe {
            csign(csig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr());
            rsign(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr());
        }
        eq_bytes(&format!("one-shot signature (mlen={mlen})"), &csig, &rsig);

        // R is the first SPX_N bytes; recompose the rest from it.
        for side in [0usize, 1] {
            let sig_ref = if side == 0 { &csig } else { &rsig };
            let sk = if side == 0 { &csk } else { &rsk };
            let recomposed = recompose(l, side, sk, &sig_ref[..SPX_N], &m, mlen);
            eq_bytes(
                &format!(
                    "{} low-level recomposition (mlen={mlen})",
                    if side == 0 { "C" } else { "Rust" }
                ),
                &sig_ref[SPX_N..],
                &recomposed,
            );
        }
    }
}

/// Rebuilds `sig[SPX_N..]` the way `crypto_sign_signature` does, using only the
/// exported low-level entry points of one library (`side` 0 = C, 1 = Rust).
fn recompose(l: &Libs, side: usize, sk: &[u8], r_field: &[u8], m: &[u8], mlen: usize) -> Vec<u8> {
    let pick = |name: &str| -> usize {
        if side == 0 {
            l.c::<unsafe extern "C" fn()>(name).into_raw() as usize
        } else {
            l.rs::<unsafe extern "C" fn()>(name).into_raw() as usize
        }
    };
    macro_rules! sym {
        ($t:ty, $n:expr) => {
            unsafe { std::mem::transmute::<usize, $t>(pick($n)) }
        };
    }
    let init_hash: InitHash = sym!(InitHash, "SPX_initialize_hash_function");
    let hash_message: HashMessage = sym!(HashMessage, "SPX_hash_message");
    let fors_sign: ForsSign = sym!(ForsSign, "SPX_fors_sign");
    let merkle_sign: MerkleSign = sym!(MerkleSign, "SPX_merkle_sign");
    let set_type: SetU32 = sym!(SetU32, "SPX_set_type");
    let set_layer: SetU32 = sym!(SetU32, "SPX_set_layer_addr");
    let set_kp: SetU32 = sym!(SetU32, "SPX_set_keypair_addr");
    let set_tree: SetU64 = sym!(SetU64, "SPX_set_tree_addr");
    let copy_subtree: unsafe extern "C" fn(*mut u32, *const u32) =
        sym!(unsafe extern "C" fn(*mut u32, *const u32), "SPX_copy_subtree_addr");

    let pk = &sk[2 * SPX_N..];
    let mut ctx = Ctx::new();
    ctx.set_seeds(&pk[..SPX_N], &sk[..SPX_N]);
    unsafe { init_hash(ctx.as_mut_ptr()) };

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    unsafe {
        set_type(wots_addr.as_mut_ptr(), 0); // SPX_ADDR_TYPE_WOTS
        set_type(tree_addr.as_mut_ptr(), 2); // SPX_ADDR_TYPE_HASHTREE
    }

    let mut out = Vec::with_capacity(SPX_BYTES - SPX_N);
    let mut mhash = vec![0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    unsafe {
        hash_message(
            mhash.as_mut_ptr(),
            &mut tree,
            &mut idx_leaf,
            r_field.as_ptr(),
            pk.as_ptr(),
            m.as_ptr(),
            mlen as u64,
            ctx.as_ptr(),
        );
        set_tree(wots_addr.as_mut_ptr(), tree);
        set_kp(wots_addr.as_mut_ptr(), idx_leaf);
    }

    let mut root = vec![0u8; SPX_N];
    let mut fors_sig = vec![0u8; SPX_FORS_BYTES];
    unsafe {
        fors_sign(
            fors_sig.as_mut_ptr(),
            root.as_mut_ptr(),
            mhash.as_ptr(),
            ctx.as_ptr(),
            wots_addr.as_ptr(),
        );
    }
    out.extend_from_slice(&fors_sig);

    let layer_len = SPX_WOTS_BYTES + SPX_TREE_HEIGHT as usize * SPX_N;
    for i in 0..SPX_D {
        let mut layer_sig = vec![0u8; layer_len];
        unsafe {
            set_layer(tree_addr.as_mut_ptr(), i);
            set_tree(tree_addr.as_mut_ptr(), tree);
            copy_subtree(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
            set_kp(wots_addr.as_mut_ptr(), idx_leaf);
            merkle_sign(
                layer_sig.as_mut_ptr(),
                root.as_mut_ptr(),
                ctx.as_ptr(),
                wots_addr.as_mut_ptr(),
                tree_addr.as_mut_ptr(),
                idx_leaf,
            );
        }
        out.extend_from_slice(&layer_sig);
        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    out
}

/* ---- C50 --------------------------------------------------------- */

#[test]
fn c50_verify_valid_cross() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (csign, rsign) = l.pair::<Signature>("crypto_sign_signature");
    let (cver, rver) = l.pair::<Verify>("crypto_sign_verify");
    let mut rng = Rng::new(SEED + 50);
    for mlen in [0usize, 1, 32, 33, 128, 231, 1000] {
        seed_both(l, &kat_entropy(), None);
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut csig = vec![0u8; SPX_BYTES];
        let mut rsig = vec![0u8; SPX_BYTES];
        let mut cl = 0usize;
        let mut rl = 0usize;
        unsafe {
            csign(csig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr());
            rsign(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr());
        }
        eq_bytes("signature", &csig, &rsig);
        // Every cross combination of {C,Rust} signer x {C,Rust} verifier.
        for (vname, v) in [("C", &cver), ("Rust", &rver)] {
            for (sname, s, pk) in [("C", &csig, &cpk), ("Rust", &rsig, &rpk)] {
                let got =
                    unsafe { v(s.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
                assert_eq!(
                    got, 0,
                    "{vname} verify of {sname} signature failed (mlen={mlen})"
                );
            }
        }
    }
}

/* ---- C51 / C52 --------------------------------------------------- */

#[test]
fn c51_sign_attached() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (c, r) = l.pair::<Sign>("crypto_sign");
    let mut rng = Rng::new(SEED + 51);
    for mlen in [0usize, 1, 33, 231, 1000] {
        seed_both(l, &kat_entropy(), None);
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut csm = vec![0xAAu8; SPX_BYTES + mlen + 16];
        let mut rsm = vec![0xAAu8; SPX_BYTES + mlen + 16];
        let mut cl = 0u64;
        let mut rl = 0u64;
        let (cv, rv) = unsafe {
            (
                c(csm.as_mut_ptr(), &mut cl, m.as_ptr(), mlen as u64, csk.as_ptr()),
                r(rsm.as_mut_ptr(), &mut rl, m.as_ptr(), mlen as u64, rsk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign return (mlen={mlen})");
        assert_eq!(cl, rl, "smlen (mlen={mlen})");
        assert_eq!(cl as usize, SPX_BYTES + mlen);
        eq_bytes(&format!("crypto_sign sm (mlen={mlen})"), &csm, &rsm);
        assert_eq!(&csm[SPX_BYTES + mlen..], &[0xAAu8; 16], "sm overrun");
        eq_bytes("sm tail must be the message", &csm[SPX_BYTES..SPX_BYTES + mlen], &m[..mlen]);
    }
}

#[test]
fn c52_open_attached() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (csign, rsign) = l.pair::<Sign>("crypto_sign");
    let (c, r) = l.pair::<Open>("crypto_sign_open");
    let mut rng = Rng::new(SEED + 52);
    for mlen in [0usize, 1, 33, 231, 1000] {
        seed_both(l, &kat_entropy(), None);
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut csm = vec![0u8; SPX_BYTES + mlen];
        let mut rsm = vec![0u8; SPX_BYTES + mlen];
        let mut sl = 0u64;
        unsafe {
            csign(csm.as_mut_ptr(), &mut sl, m.as_ptr(), mlen as u64, csk.as_ptr());
            rsign(rsm.as_mut_ptr(), &mut sl, m.as_ptr(), mlen as u64, rsk.as_ptr());
        }
        eq_bytes("sm", &csm, &rsm);
        let mut cm = vec![0xAAu8; SPX_BYTES + mlen + 16];
        let mut rm = vec![0xAAu8; SPX_BYTES + mlen + 16];
        let mut cl = u64::MAX;
        let mut rl = u64::MAX;
        let (cv, rv) = unsafe {
            (
                c(cm.as_mut_ptr(), &mut cl, csm.as_ptr(), sl, cpk.as_ptr()),
                r(rm.as_mut_ptr(), &mut rl, rsm.as_ptr(), sl, rpk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_open return (mlen={mlen})");
        assert_eq!(cv, 0, "valid sm must open");
        assert_eq!(cl, rl, "*mlen (mlen={mlen})");
        assert_eq!(cl as usize, mlen);
        eq_bytes(&format!("crypto_sign_open m buffer (mlen={mlen})"), &cm, &rm);
        eq_bytes("recovered message", &cm[..mlen], &m[..mlen]);
    }
}

/* ---- C53 --------------------------------------------------------- */

/// The `PQCgenKAT_sign.c` loop shape: one seeding, then several
/// keypair→sign→open rounds so the DRBG state chains across iterations.
#[test]
fn c53_full_roundtrip_chained() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (csign, rsign) = l.pair::<Sign>("crypto_sign");
    let (copen, ropen) = l.pair::<Open>("crypto_sign_open");
    let (crb, rrb) = l.pair::<unsafe extern "C" fn(*mut u8, u64) -> i32>("randombytes");
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");

    seed_both(l, &kat_entropy(), None);
    const LOOP_COUNT: usize = 7;
    const BASE_MLEN: usize = 33;
    for i in 0..LOOP_COUNT {
        // seed + msg drawn from the chained DRBG on both sides
        let mut cseed = vec![0u8; 48];
        let mut rseed = vec![0u8; 48];
        unsafe {
            crb(cseed.as_mut_ptr(), 48);
            rrb(rseed.as_mut_ptr(), 48);
        }
        eq_bytes(&format!("randombytes seed round {i}"), &cseed, &rseed);
        let mlen = BASE_MLEN * (i + 1);
        let mut cmsg = vec![0u8; mlen];
        let mut rmsg = vec![0u8; mlen];
        unsafe {
            crb(cmsg.as_mut_ptr(), mlen as u64);
            rrb(rmsg.as_mut_ptr(), mlen as u64);
        }
        eq_bytes(&format!("randombytes msg round {i}"), &cmsg, &rmsg);

        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        }
        eq_bytes(&format!("pk round {i}"), &cpk, &rpk);
        eq_bytes(&format!("sk round {i}"), &csk, &rsk);

        let mut csm = vec![0u8; SPX_BYTES + mlen];
        let mut rsm = vec![0u8; SPX_BYTES + mlen];
        let mut csl = 0u64;
        let mut rsl = 0u64;
        unsafe {
            csign(csm.as_mut_ptr(), &mut csl, cmsg.as_ptr(), mlen as u64, csk.as_ptr());
            rsign(rsm.as_mut_ptr(), &mut rsl, rmsg.as_ptr(), mlen as u64, rsk.as_ptr());
        }
        assert_eq!(csl, rsl, "smlen round {i}");
        eq_bytes(&format!("sm round {i}"), &csm, &rsm);

        let mut cm1 = vec![0u8; SPX_BYTES + mlen];
        let mut rm1 = vec![0u8; SPX_BYTES + mlen];
        let mut cml = 0u64;
        let mut rml = 0u64;
        let (cv, rv) = unsafe {
            (
                copen(cm1.as_mut_ptr(), &mut cml, csm.as_ptr(), csl, cpk.as_ptr()),
                ropen(rm1.as_mut_ptr(), &mut rml, rsm.as_ptr(), rsl, rpk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "open return round {i}");
        assert_eq!(cv, 0);
        assert_eq!(cml, rml);
        assert_eq!(cml as usize, mlen);
        eq_bytes(&format!("opened m round {i}"), &cm1[..mlen], &cmsg);
        eq_bytes(&format!("opened m round {i} (rust)"), &rm1[..mlen], &rmsg);
        assert_eq!(unsafe { **cg }, unsafe { **rg }, "DRBG_ctx after round {i}");
    }
}
