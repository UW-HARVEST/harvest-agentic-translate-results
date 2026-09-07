//! Phase B rows 48-53: the public `api.h` surface (`sign.c`).
//!
//! `crypto_sign_keypair` / `crypto_sign_signature` consume the process-global
//! DRBG (`randombytes`), so every test that touches it re-seeds the DRBG in
//! BOTH libraries first and holds `drbg_lock()`.
mod common;
use common::*;

type FnSizes = unsafe extern "C" fn() -> core::ffi::c_ulonglong;
type FnSeedKeypair =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> core::ffi::c_int;
type FnKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> core::ffi::c_int;
type FnSignature = unsafe extern "C" fn(
    *mut u8,
    *mut usize,
    *const u8,
    usize,
    *const u8,
) -> core::ffi::c_int;
type FnVerify = unsafe extern "C" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
) -> core::ffi::c_int;
type FnSign = unsafe extern "C" fn(
    *mut u8,
    *mut core::ffi::c_ulonglong,
    *const u8,
    core::ffi::c_ulonglong,
    *const u8,
) -> core::ffi::c_int;
type FnOpen = unsafe extern "C" fn(
    *mut u8,
    *mut core::ffi::c_ulonglong,
    *const u8,
    core::ffi::c_ulonglong,
    *const u8,
) -> core::ffi::c_int;
type FnRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FnDrbgPtr = *mut DrbgCtxFfi;

// Each entry costs two full sign+verify operations on both libraries, which is
// expensive for the `s` parameter sets (tree height 8-9). Keep the shapes that
// matter: empty, one byte, the driver's base length, a block boundary and the
// driver's maximum. (`layer1_keyed`/`errors` sweep the full length ladder
// against the much cheaper hash-level entry points.)
const MLENS: &[usize] = &[0, 1, 33, 64, 231];

/// Seed both libraries' DRBGs identically and assert their `DRBG_ctx` globals
/// agree afterwards.
fn reseed(entropy: &[u8; 48], pers: Option<&[u8; 48]>) {
    let l = libs();
    let (c, r) = l.pair::<FnRbInit>("randombytes_init");
    let mut e1 = *entropy;
    let mut e2 = *entropy;
    let mut p1 = pers.copied();
    let mut p2 = pers.copied();
    unsafe {
        c(
            e1.as_mut_ptr(),
            p1.as_mut().map_or(core::ptr::null_mut(), |p| p.as_mut_ptr()),
        );
        r(
            e2.as_mut_ptr(),
            p2.as_mut().map_or(core::ptr::null_mut(), |p| p.as_mut_ptr()),
        );
    }
    assert_drbg_eq("after randombytes_init");
}

fn assert_drbg_eq(what: &str) {
    let l = libs();
    let c = l.sym::<FnDrbgPtr>(Which::C, "DRBG_ctx");
    let r = l.sym::<FnDrbgPtr>(Which::R, "DRBG_ctx");
    let cv = unsafe { (**c).clone() };
    let rv = unsafe { (**r).clone() };
    assert_bytes_eq(
        &format!("DRBG_ctx {what} (Key||V)"),
        &[&cv.Key[..], &cv.V[..]].concat(),
        &[&rv.Key[..], &rv.V[..]].concat(),
    );
    assert_eq_dbg(
        &format!("DRBG_ctx {what} reseed_counter"),
        cv.reseed_counter,
        rv.reseed_counter,
    );
}

// --- row 48 -------------------------------------------------------------

#[test]
fn row48_size_getters() {
    let l = libs();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", CRYPTO_SECRETKEYBYTES),
        ("crypto_sign_publickeybytes", CRYPTO_PUBLICKEYBYTES),
        ("crypto_sign_bytes", CRYPTO_BYTES),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES),
    ] {
        let (c, r) = l.pair::<FnSizes>(name);
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        assert_eq_dbg(name, cv, rv);
        assert_eq_dbg(
            &format!("{name} == the Rust params table"),
            cv,
            expect as core::ffi::c_ulonglong,
        );
    }
}

// --- row 49 -------------------------------------------------------------

#[test]
fn row49_crypto_sign_seed_keypair() {
    let l = libs();
    let (c, r) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x4901);
    for it in 0..64 {
        let seed = match it {
            0 => vec![0x00u8; CRYPTO_SEEDBYTES],
            1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        let mut cpk = vec![0xA5u8; CRYPTO_PUBLICKEYBYTES + 8];
        let mut rpk = vec![0xA5u8; CRYPTO_PUBLICKEYBYTES + 8];
        let mut csk = vec![0xA5u8; CRYPTO_SECRETKEYBYTES + 8];
        let mut rsk = vec![0xA5u8; CRYPTO_SECRETKEYBYTES + 8];
        let cv = unsafe { c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
        let rv = unsafe { r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
        assert_eq_dbg(&format!("crypto_sign_seed_keypair #{it} retval"), cv, rv);
        assert_bytes_eq(&format!("crypto_sign_seed_keypair #{it} pk"), &cpk, &rpk);
        assert_bytes_eq(&format!("crypto_sign_seed_keypair #{it} sk"), &csk, &rsk);
    }
}

// --- row 50 -------------------------------------------------------------

#[test]
fn row50_crypto_sign_keypair_deterministic() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnKeypair>("crypto_sign_keypair");
    let mut rng = Rng::new(0x5001);
    for it in 0..8 {
        let mut ent = [0u8; 48];
        for (i, b) in ent.iter_mut().enumerate() {
            *b = if it == 0 { i as u8 } else { rng.byte() };
        }
        // Both libraries start from the same DRBG state ...
        reseed(&ent, None);
        let mut cpk = vec![0xA5u8; CRYPTO_PUBLICKEYBYTES + 8];
        let mut csk = vec![0xA5u8; CRYPTO_SECRETKEYBYTES + 8];
        let cv = unsafe { c(cpk.as_mut_ptr(), csk.as_mut_ptr()) };
        let c_drbg = {
            let p = l.sym::<FnDrbgPtr>(Which::C, "DRBG_ctx");
            unsafe { (**p).clone() }
        };

        reseed(&ent, None);
        let mut rpk = vec![0xA5u8; CRYPTO_PUBLICKEYBYTES + 8];
        let mut rsk = vec![0xA5u8; CRYPTO_SECRETKEYBYTES + 8];
        let rv = unsafe { r(rpk.as_mut_ptr(), rsk.as_mut_ptr()) };
        let r_drbg = {
            let p = l.sym::<FnDrbgPtr>(Which::R, "DRBG_ctx");
            unsafe { (**p).clone() }
        };

        assert_eq_dbg(&format!("crypto_sign_keypair #{it} retval"), cv, rv);
        assert_bytes_eq(&format!("crypto_sign_keypair #{it} pk"), &cpk, &rpk);
        assert_bytes_eq(&format!("crypto_sign_keypair #{it} sk"), &csk, &rsk);
        assert_eq_dbg(
            &format!("crypto_sign_keypair #{it} DRBG consumed identically"),
            c_drbg,
            r_drbg,
        );
    }
}

// --- rows 51 / 52 -------------------------------------------------------

#[test]
fn row51_crypto_sign_signature_and_verify() {
    let _g = drbg_lock();
    let l = libs();
    let (csig, rsig) = l.pair::<FnSignature>("crypto_sign_signature");
    let (cver, rver) = l.pair::<FnVerify>("crypto_sign_verify");
    let (csk_gen, _) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x5101);

    for &mlen in MLENS {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
        unsafe {
            csk_gen(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut ent = [0u8; 48];
        rng.fill(&mut ent);

        // `crypto_sign_signature` draws `optrand` from the DRBG, so re-seed
        // before each side to make the signature deterministic.
        // Re-seed BOTH, run C (which advances C's DRBG), snapshot C's DRBG;
        // then re-seed BOTH again and run Rust, snapshot Rust's DRBG. The two
        // snapshots must be equal -- i.e. both consumed the DRBG identically.
        reseed(&ent, None);
        let mut cs = vec![0xA5u8; SPX_BYTES + 8];
        let mut cl: usize = usize::MAX;
        let cv = unsafe {
            csig(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, sk.as_ptr())
        };
        let c_drbg = {
            let p = l.sym::<FnDrbgPtr>(Which::C, "DRBG_ctx");
            unsafe { (**p).clone() }
        };

        reseed(&ent, None);
        let mut rs = vec![0xA5u8; SPX_BYTES + 8];
        let mut rl: usize = usize::MAX;
        let rv = unsafe {
            rsig(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, sk.as_ptr())
        };
        let r_drbg = {
            let p = l.sym::<FnDrbgPtr>(Which::R, "DRBG_ctx");
            unsafe { (**p).clone() }
        };

        assert_eq_dbg(&format!("crypto_sign_signature(mlen={mlen}) retval"), cv, rv);
        assert_eq_dbg(&format!("crypto_sign_signature(mlen={mlen}) *siglen"), cl, rl);
        assert_eq_dbg(
            &format!("crypto_sign_signature(mlen={mlen}) *siglen == SPX_BYTES"),
            cl,
            SPX_BYTES,
        );
        assert_bytes_eq(&format!("crypto_sign_signature(mlen={mlen}) sig"), &cs, &rs);
        assert_eq_dbg(
            &format!("crypto_sign_signature(mlen={mlen}) DRBG consumed identically"),
            c_drbg,
            r_drbg,
        );

        // row 52: the freshly produced signature must verify (return 0) on both.
        let a = unsafe { cver(cs.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
        let b = unsafe { rver(cs.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
        assert_eq_dbg(&format!("crypto_sign_verify(mlen={mlen}) retval"), a, b);
        assert_eq_dbg(
            &format!("crypto_sign_verify(mlen={mlen}) accepts a valid signature"),
            a,
            0,
        );
    }
}

// --- row 53 -------------------------------------------------------------

#[test]
fn row53_crypto_sign_open_round_trip() {
    let _g = drbg_lock();
    let l = libs();
    let (csign, rsign) = l.pair::<FnSign>("crypto_sign");
    let (copen, ropen) = l.pair::<FnOpen>("crypto_sign_open");
    let (kg, _) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x5301);

    for &mlen in MLENS {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
        unsafe {
            kg(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut ent = [0u8; 48];
        rng.fill(&mut ent);

        reseed(&ent, None);
        let mut csm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut csmlen: core::ffi::c_ulonglong = u64::MAX;
        let cv = unsafe {
            csign(
                csm.as_mut_ptr(),
                &mut csmlen,
                m.as_ptr(),
                mlen as core::ffi::c_ulonglong,
                sk.as_ptr(),
            )
        };

        reseed(&ent, None);
        let mut rsm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut rsmlen: core::ffi::c_ulonglong = u64::MAX;
        let rv = unsafe {
            rsign(
                rsm.as_mut_ptr(),
                &mut rsmlen,
                m.as_ptr(),
                mlen as core::ffi::c_ulonglong,
                sk.as_ptr(),
            )
        };

        assert_eq_dbg(&format!("crypto_sign(mlen={mlen}) retval"), cv, rv);
        assert_eq_dbg(&format!("crypto_sign(mlen={mlen}) *smlen"), csmlen, rsmlen);
        assert_eq_dbg(
            &format!("crypto_sign(mlen={mlen}) *smlen == SPX_BYTES+mlen"),
            csmlen as usize,
            SPX_BYTES + mlen,
        );
        assert_bytes_eq(&format!("crypto_sign(mlen={mlen}) sm"), &csm, &rsm);

        // Open the SAME sm on both sides.
        let mut cm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut rm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut cml: core::ffi::c_ulonglong = u64::MAX;
        let mut rml: core::ffi::c_ulonglong = u64::MAX;
        let a = unsafe { copen(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), csmlen, pk.as_ptr()) };
        let b = unsafe { ropen(rm.as_mut_ptr(), &mut rml, csm.as_ptr(), csmlen, pk.as_ptr()) };
        assert_eq_dbg(&format!("crypto_sign_open(mlen={mlen}) retval"), a, b);
        assert_eq_dbg(&format!("crypto_sign_open(mlen={mlen}) accepts"), a, 0);
        assert_eq_dbg(&format!("crypto_sign_open(mlen={mlen}) *mlen"), cml, rml);
        assert_eq_dbg(
            &format!("crypto_sign_open(mlen={mlen}) *mlen == mlen"),
            cml as usize,
            mlen,
        );
        assert_bytes_eq(&format!("crypto_sign_open(mlen={mlen}) m"), &cm, &rm);
        assert_bytes_eq(
            &format!("crypto_sign_open(mlen={mlen}) recovered message"),
            &m[..mlen],
            &cm[..mlen],
        );
    }
}
