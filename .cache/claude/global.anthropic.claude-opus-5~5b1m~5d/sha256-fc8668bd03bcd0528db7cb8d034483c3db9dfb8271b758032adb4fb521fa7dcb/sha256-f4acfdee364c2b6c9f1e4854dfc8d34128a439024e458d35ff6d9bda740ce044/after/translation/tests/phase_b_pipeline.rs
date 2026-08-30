//! Phase B rows 32-42, 51, 52: FORS, Merkle, and the full public API driven
//! exactly the way a real consumer drives it.
mod common;
use common::*;

type FForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type FForsPk = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
type FMerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
type FMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);
type FSeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type FKp = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type FSignature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type FVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type FSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FSizes = unsafe extern "C" fn() -> u64;

fn mlen_sweep() -> Vec<usize> {
    let mut v = vec![
        0usize, 1, 2, 15, 16, 17, 23, 24, 25, 31, 32, 33, 39, 40, 41, 47, 48, 49, 55, 56, 57, 63,
        64, 65, 71, 72, 73, 79, 80, 81, 95, 96, 97, 103, 104, 105, 111, 112, 113, 127, 128, 129,
        135, 136, 137, 167, 168, 169, 255, 256, 257, 500, 1000,
    ];
    let blk = if N >= 24 { 128usize } else { 64 };
    let inblocks = (N + PK_BYTES + blk - 1) / blk;
    for d in [-1i64, 0, 1] {
        for base in [blk as i64 - N as i64, (inblocks * blk) as i64 - N as i64 - PK_BYTES as i64] {
            let x = base + d;
            if x >= 0 {
                v.push(x as usize);
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Seeds the DRBG of both libraries identically.
unsafe fn seed_both(p: &Pair, entropy: &[u8], pers: Option<&[u8]>) {
    let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
    let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
    let mut e1 = entropy.to_vec();
    let mut e2 = entropy.to_vec();
    match pers {
        None => {
            ci(e1.as_mut_ptr(), core::ptr::null_mut());
            ri(e2.as_mut_ptr(), core::ptr::null_mut());
        }
        Some(x) => {
            let mut p1 = x.to_vec();
            let mut p2 = x.to_vec();
            ci(e1.as_mut_ptr(), p1.as_mut_ptr());
            ri(e2.as_mut_ptr(), p2.as_mut_ptr());
        }
    }
    eqb("DRBG_ctx after randombytes_init", &drbg_image(&p.c), &drbg_image(&p.r));
}

// ---------- rows 32-34 ----------
unsafe fn fors_case(p: &Pair, rng: &mut Rng, m: &[u8], label: &str) {
    let cfs = sym!(p.c, b"SPX_fors_sign\0", FForsSign);
    let rfs = sym!(p.r, b"SPX_fors_sign\0", FForsSign);
    let cfp = sym!(p.c, b"SPX_fors_pk_from_sig\0", FForsPk);
    let rfp = sym!(p.r, b"SPX_fors_pk_from_sig\0", FForsPk);

    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let cc = make_ctx(&p.c, &ps, &ss);
    let rc = make_ctx(&p.r, &ps, &ss);
    let a = rng.addr();

    let mut csig = obuf(FORS_BYTES);
    let mut rsig = obuf(FORS_BYTES);
    let mut cpk = obuf(N);
    let mut rpk = obuf(N);
    cfs(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), a.as_ptr());
    rfs(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), rc.as_ptr(), a.as_ptr());
    eqb(&format!("{label} fors_sign sig"), &csig, &rsig);
    eqb(&format!("{label} fors_sign pk"), &cpk, &rpk);

    // round trip + garbage signature
    for sig in [csig[..FORS_BYTES].to_vec(), rng.bytes(FORS_BYTES)] {
        let mut cpk2 = obuf(N);
        let mut rpk2 = obuf(N);
        cfp(cpk2.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), cc.as_ptr(), a.as_ptr());
        rfp(rpk2.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), rc.as_ptr(), a.as_ptr());
        eqb(&format!("{label} fors_pk_from_sig"), &cpk2, &rpk2);
    }
    // the honest signature must reproduce the signer's pk
    let mut cpk3 = obuf(N);
    cfp(cpk3.as_mut_ptr(), csig.as_ptr(), m.as_ptr(), cc.as_ptr(), a.as_ptr());
    eqb(&format!("{label} fors round trip"), &cpk3, &cpk);
}

#[test]
fn b32_fors_sign() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..6 {
        let m = rng.bytes(FORS_MSG_BYTES);
        unsafe { fors_case(p, &mut rng, &m, "random") }
    }
}

#[test]
fn b33_fors_sign_extremes() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 33);
    for pat in [0x00u8, 0xff, 0x55, 0xaa] {
        let m = vec![pat; FORS_MSG_BYTES];
        unsafe { fors_case(p, &mut rng, &m, &format!("pat{pat:#x}")) }
    }
}

#[test]
fn b34_fors_pk_from_sig() {
    // covered by b32/b33 (round trip + garbage); this row additionally checks a
    // signature whose leaf-index bytes are extreme.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 34);
    unsafe {
        let cfp = sym!(p.c, b"SPX_fors_pk_from_sig\0", FForsPk);
        let rfp = sym!(p.r, b"SPX_fors_pk_from_sig\0", FForsPk);
        for _ in 0..8 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let a = rng.addr();
            for m in [vec![0u8; FORS_MSG_BYTES], vec![0xffu8; FORS_MSG_BYTES], rng.bytes(FORS_MSG_BYTES)] {
                let sig = rng.bytes(FORS_BYTES);
                let mut cpk = obuf(N);
                let mut rpk = obuf(N);
                cfp(cpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), cc.as_ptr(), a.as_ptr());
                rfp(rpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), rc.as_ptr(), a.as_ptr());
                eqb("fors_pk_from_sig", &cpk, &rpk);
            }
        }
    }
}

// ---------- rows 35-37 ----------
unsafe fn merkle_case(p: &Pair, rng: &mut Rng, idx_leaf: u32, iters: usize) {
    let cf = sym!(p.c, b"SPX_merkle_sign\0", FMerkleSign);
    let rf = sym!(p.r, b"SPX_merkle_sign\0", FMerkleSign);
    let siglen = WOTS_BYTES + TREE_HEIGHT * N;
    for _ in 0..iters {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);
        let wa = rng.addr();
        let ta = rng.addr();
        let root0 = rng.bytes(N);

        let mut cwa = wa;
        let mut cta = ta;
        let mut rwa = wa;
        let mut rta = ta;
        let mut csig = obuf(siglen);
        let mut rsig = obuf(siglen);
        let mut croot = obuf(N);
        let mut rroot = obuf(N);
        croot[..N].copy_from_slice(&root0);
        rroot[..N].copy_from_slice(&root0);

        cf(csig.as_mut_ptr(), croot.as_mut_ptr(), cc.as_ptr(), cwa.as_mut_ptr(), cta.as_mut_ptr(), idx_leaf);
        rf(rsig.as_mut_ptr(), rroot.as_mut_ptr(), rc.as_ptr(), rwa.as_mut_ptr(), rta.as_mut_ptr(), idx_leaf);
        eqb(&format!("merkle_sign sig idx={idx_leaf}"), &csig, &rsig);
        eqb(&format!("merkle_sign root idx={idx_leaf}"), &croot, &rroot);
        eqb("merkle_sign wots_addr", &addr_to_bytes(&cwa), &addr_to_bytes(&rwa));
        eqb("merkle_sign tree_addr", &addr_to_bytes(&cta), &addr_to_bytes(&rta));
    }
}

#[test]
fn b35_merkle_sign() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 35);
    let maxl = (1u32 << TREE_HEIGHT) - 1;
    for _ in 0..4 {
        let idx = rng.next_u32() & maxl;
        unsafe { merkle_case(p, &mut rng, idx, 1) }
    }
}

#[test]
fn b36_merkle_sign_extremes() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 36);
    let maxl = (1u32 << TREE_HEIGHT) - 1;
    for idx in [0u32, maxl, u32::MAX] {
        unsafe { merkle_case(p, &mut rng, idx, 1) }
    }
}

#[test]
fn b37_merkle_gen_root() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 37);
    unsafe {
        let cf = sym!(p.c, b"SPX_merkle_gen_root\0", FMerkleGenRoot);
        let rf = sym!(p.r, b"SPX_merkle_gen_root\0", FMerkleGenRoot);
        for _ in 0..4 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let mut cr = obuf(N);
            let mut rr = obuf(N);
            cf(cr.as_mut_ptr(), cc.as_ptr());
            rf(rr.as_mut_ptr(), rc.as_ptr());
            eqb("merkle_gen_root", &cr, &rr);
        }
    }
}

// ---------- rows 38 & 39 ----------
#[test]
fn b38_seed_keypair() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 38);
    unsafe {
        let cf = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rf = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; SEED_BYTES], vec![0xffu8; SEED_BYTES]];
        for _ in 0..4 {
            seeds.push(rng.bytes(SEED_BYTES));
        }
        for seed in seeds {
            let mut cpk = obuf(PK_BYTES);
            let mut rpk = obuf(PK_BYTES);
            let mut csk = obuf(SK_BYTES);
            let mut rsk = obuf(SK_BYTES);
            let cr = cf(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let rr = rf(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eqv("seed_keypair ret", cr, rr);
            eqb("seed_keypair pk", &cpk, &rpk);
            eqb("seed_keypair sk", &csk, &rsk);
        }
    }
}

#[test]
fn b39_keypair_via_drbg() {
    let _guard = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 39);
    unsafe {
        let cf = sym!(p.c, b"crypto_sign_keypair\0", FKp);
        let rf = sym!(p.r, b"crypto_sign_keypair\0", FKp);
        for i in 0..3 {
            let ent = rng.bytes(48);
            let pers = if i % 2 == 0 { None } else { Some(rng.bytes(48)) };
            seed_both(p, &ent, pers.as_deref());
            let mut cpk = obuf(PK_BYTES);
            let mut rpk = obuf(PK_BYTES);
            let mut csk = obuf(SK_BYTES);
            let mut rsk = obuf(SK_BYTES);
            let cr = cf(cpk.as_mut_ptr(), csk.as_mut_ptr());
            let rr = rf(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            eqv("keypair ret", cr, rr);
            eqb("keypair pk", &cpk, &rpk);
            eqb("keypair sk", &csk, &rsk);
            eqb("DRBG_ctx after keypair", &drbg_image(&p.c), &drbg_image(&p.r));
        }
    }
}

// ---------- rows 40-42, 51 ----------
#[test]
fn b40_sign_verify_mlen_sweep() {
    let _guard = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 40);
    unsafe {
        let csg = sym!(p.c, b"crypto_sign_signature\0", FSignature);
        let rsg = sym!(p.r, b"crypto_sign_signature\0", FSignature);
        let cvf = sym!(p.c, b"crypto_sign_verify\0", FVerify);
        let rvf = sym!(p.r, b"crypto_sign_verify\0", FVerify);
        let ckp = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rkp = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);

        let seed = rng.bytes(SEED_BYTES);
        let mut cpk = obuf(PK_BYTES);
        let mut rpk = obuf(PK_BYTES);
        let mut csk = obuf(SK_BYTES);
        let mut rsk = obuf(SK_BYTES);
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
        eqb("keypair pk", &cpk, &rpk);
        eqb("keypair sk", &csk, &rsk);

        // limit the number of full signatures for the slow parameter sets
        let sweep = mlen_sweep();
        let step = if TREE_HEIGHT >= 8 { 7 } else { 3 };
        for (i, mlen) in sweep.iter().enumerate() {
            if i % step != 0 && *mlen > 1 {
                continue;
            }
            let mlen = *mlen;
            let m = rng.bytes(mlen.max(1));
            let ent = rng.bytes(48);
            seed_both(p, &ent, None);

            let mut csig = obuf(SPX_BYTES);
            let mut rsig = obuf(SPX_BYTES);
            let mut csl: usize = 0;
            let mut rsl: usize = 0;
            let cr = csg(csig.as_mut_ptr(), &mut csl, m.as_ptr(), mlen, csk.as_ptr());
            let rr = rsg(rsig.as_mut_ptr(), &mut rsl, m.as_ptr(), mlen, rsk.as_ptr());
            eqv(&format!("signature ret mlen={mlen}"), cr, rr);
            eqv(&format!("siglen mlen={mlen}"), csl, rsl);
            eqb(&format!("signature bytes mlen={mlen}"), &csig, &rsig);
            eqv("siglen == SPX_BYTES", csl, SPX_BYTES);

            eqb("DRBG_ctx after sign", &drbg_image(&p.c), &drbg_image(&p.r));

            // cross verification, both directions
            for (name, sig) in [("c-sig", &csig), ("r-sig", &rsig)] {
                let a = cvf(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, cpk.as_ptr());
                let b = rvf(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, rpk.as_ptr());
                eqv(&format!("verify {name} mlen={mlen}"), a, b);
                eqv(&format!("verify {name} ok mlen={mlen}"), a, 0);
            }
        }
    }
}

#[test]
fn b41_sign_drbg_lockstep() {
    let _guard = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 41);
    unsafe {
        let csg = sym!(p.c, b"crypto_sign_signature\0", FSignature);
        let rsg = sym!(p.r, b"crypto_sign_signature\0", FSignature);
        let ckp = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rkp = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);
        let seed = rng.bytes(SEED_BYTES);
        let mut cpk = obuf(PK_BYTES);
        let mut rpk = obuf(PK_BYTES);
        let mut csk = obuf(SK_BYTES);
        let mut rsk = obuf(SK_BYTES);
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());

        // Seed once, then sign repeatedly: the DRBG state must advance
        // identically across a sequence of signatures.
        let ent = rng.bytes(48);
        seed_both(p, &ent, None);
        for k in 0..3 {
            let m = rng.bytes(1 + k * 37);
            let mut csig = obuf(SPX_BYTES);
            let mut rsig = obuf(SPX_BYTES);
            let mut csl = 0usize;
            let mut rsl = 0usize;
            csg(csig.as_mut_ptr(), &mut csl, m.as_ptr(), m.len(), csk.as_ptr());
            rsg(rsig.as_mut_ptr(), &mut rsl, m.as_ptr(), m.len(), rsk.as_ptr());
            eqb(&format!("sequential sign #{k}"), &csig, &rsig);
            eqv("siglen", csl, rsl);
            eqb(&format!("DRBG_ctx after sign #{k}"), &drbg_image(&p.c), &drbg_image(&p.r));
        }
    }
}

#[test]
fn b42_sign_open_roundtrip() {
    let _guard = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 42);
    unsafe {
        let cs = sym!(p.c, b"crypto_sign\0", FSign);
        let rs = sym!(p.r, b"crypto_sign\0", FSign);
        let co = sym!(p.c, b"crypto_sign_open\0", FOpen);
        let ro = sym!(p.r, b"crypto_sign_open\0", FOpen);
        let ckp = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rkp = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);

        let seed = rng.bytes(SEED_BYTES);
        let mut cpk = obuf(PK_BYTES);
        let mut rpk = obuf(PK_BYTES);
        let mut csk = obuf(SK_BYTES);
        let mut rsk = obuf(SK_BYTES);
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());

        for mlen in [0usize, 1, 33, 64, 65, 137, 1000] {
            let m = rng.bytes(mlen.max(1));
            let ent = rng.bytes(48);
            seed_both(p, &ent, None);

            let mut csm = obuf(SPX_BYTES + mlen);
            let mut rsm = obuf(SPX_BYTES + mlen);
            let mut csl = 0u64;
            let mut rsl = 0u64;
            let a = cs(csm.as_mut_ptr(), &mut csl, m.as_ptr(), mlen as u64, csk.as_ptr());
            let b = rs(rsm.as_mut_ptr(), &mut rsl, m.as_ptr(), mlen as u64, rsk.as_ptr());
            eqv(&format!("crypto_sign ret mlen={mlen}"), a, b);
            eqv(&format!("smlen mlen={mlen}"), csl, rsl);
            eqv("smlen value", csl as usize, SPX_BYTES + mlen);
            eqb(&format!("sm mlen={mlen}"), &csm, &rsm);

            let mut cm = obuf(SPX_BYTES + mlen);
            let mut rm = obuf(SPX_BYTES + mlen);
            let mut cml = 0u64;
            let mut rml = 0u64;
            let a = co(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), csl, cpk.as_ptr());
            let b = ro(rm.as_mut_ptr(), &mut rml, rsm.as_ptr(), rsl, rpk.as_ptr());
            eqv(&format!("open ret mlen={mlen}"), a, b);
            eqv("open ret ok", a, 0);
            eqv(&format!("open mlen out mlen={mlen}"), cml, rml);
            eqv("open mlen value", cml as usize, mlen);
            eqb(&format!("open recovered mlen={mlen}"), &cm, &rm);
        }
    }
}

// ---------- row 51 ----------
#[test]
fn b51_cross_library_pipeline() {
    let _guard = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 51);
    unsafe {
        let ckp = sym!(p.c, b"crypto_sign_keypair\0", FKp);
        let rkp = sym!(p.r, b"crypto_sign_keypair\0", FKp);
        let csg = sym!(p.c, b"crypto_sign_signature\0", FSignature);
        let rsg = sym!(p.r, b"crypto_sign_signature\0", FSignature);
        let cvf = sym!(p.c, b"crypto_sign_verify\0", FVerify);
        let rvf = sym!(p.r, b"crypto_sign_verify\0", FVerify);

        // C keypair -> Rust sign -> C verify
        let ent = rng.bytes(48);
        seed_both(p, &ent, None);
        let mut pk = obuf(PK_BYTES);
        let mut sk = obuf(SK_BYTES);
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr());
        let m = rng.bytes(97);
        let mut sig = obuf(SPX_BYTES);
        let mut sl = 0usize;
        rsg(sig.as_mut_ptr(), &mut sl, m.as_ptr(), m.len(), sk.as_ptr());
        eqv("C-key/Rust-sign/C-verify", cvf(sig.as_ptr(), sl, m.as_ptr(), m.len(), pk.as_ptr()), 0);

        // Rust keypair -> C sign -> Rust verify
        let ent = rng.bytes(48);
        seed_both(p, &ent, None);
        let mut pk = obuf(PK_BYTES);
        let mut sk = obuf(SK_BYTES);
        rkp(pk.as_mut_ptr(), sk.as_mut_ptr());
        let m = rng.bytes(3);
        let mut sig = obuf(SPX_BYTES);
        let mut sl = 0usize;
        csg(sig.as_mut_ptr(), &mut sl, m.as_ptr(), m.len(), sk.as_ptr());
        eqv("Rust-key/C-sign/Rust-verify", rvf(sig.as_ptr(), sl, m.as_ptr(), m.len(), pk.as_ptr()), 0);
    }
}

// ---------- row 52 ----------
#[test]
fn b52_size_getters() {
    let p = pair();
    unsafe {
        for (name, expect) in [
            (&b"crypto_sign_secretkeybytes\0"[..], SK_BYTES),
            (&b"crypto_sign_publickeybytes\0"[..], PK_BYTES),
            (&b"crypto_sign_bytes\0"[..], SPX_BYTES),
            (&b"crypto_sign_seedbytes\0"[..], SEED_BYTES),
        ] {
            let cf = sym!(p.c, name, FSizes);
            let rf = sym!(p.r, name, FSizes);
            let cv = cf();
            let rv = rf();
            eqv(&format!("{} C vs Rust", String::from_utf8_lossy(name)), cv, rv);
            eqv(&format!("{} value", String::from_utf8_lossy(name)), cv as usize, expect);
        }
    }
}
