//! Phase B — valid-path differential tests, CONFIGS.md group 3
//! (scalarmult, core ed25519/ristretto255, sign, kx, kem)

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

const H2C_SHA256: c_int = 1;
const H2C_SHA512: c_int = 2;

fn getter(name: &str) -> usize {
    let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(name);
    unsafe {
        assert_eq!(c(), r(), "{name}");
        c()
    }
}

// ------------------------------------------------------- rows 116-124: scalarmult

/// Produce a valid main-subgroup ed25519 point via the C implementation.
fn ed25519_point(scalar: &[u8]) -> Vec<u8> {
    let (c, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_scalarmult_ed25519_base",
    );
    let mut p = vec![0u8; 32];
    unsafe {
        assert_eq!(c(p.as_mut_ptr(), scalar.as_ptr()), 0, "ed25519_base failed");
    }
    p
}

fn ristretto_point(scalar: &[u8]) -> Vec<u8> {
    let (c, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_scalarmult_ristretto255_base",
    );
    let mut p = vec![0u8; 32];
    unsafe {
        assert_eq!(c(p.as_mut_ptr(), scalar.as_ptr()), 0, "ristretto base failed");
    }
    p
}

fn nonzero_scalar(rng: &mut Rng) -> Vec<u8> {
    let mut s = rng.bytes(32);
    s[31] &= 0x0f;
    s[0] |= 1;
    s
}

#[test]
fn rows116_118_scalarmult_curve25519() {
    let bytes = getter("crypto_scalarmult_curve25519_bytes");
    let sb = getter("crypto_scalarmult_curve25519_scalarbytes");
    assert_eq!((bytes, sb), (32, 32));
    let _ = getter("crypto_scalarmult_bytes");
    let _ = getter("crypto_scalarmult_scalarbytes");

    let mut rng = Rng::seeded();
    for name in ["crypto_scalarmult_curve25519_base", "crypto_scalarmult_base"] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(name);
        for _ in 0..200 {
            let n = rng.bytes(32);
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                same_ret(name, c(a.as_mut_ptr(), n.as_ptr()), r(b.as_mut_ptr(), n.as_ptr()));
            }
            same_bytes(name, &a, &b);
        }
    }
    for name in ["crypto_scalarmult_curve25519", "crypto_scalarmult"] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        let (bc, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
            "crypto_scalarmult_curve25519_base",
        );
        for _ in 0..200 {
            let sk = rng.bytes(32);
            let n = rng.bytes(32);
            let mut p = vec![0u8; 32];
            unsafe {
                bc(p.as_mut_ptr(), sk.as_ptr());
            }
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                same_ret(
                    name,
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                );
            }
            same_bytes(name, &a, &b);
        }
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_scalarmult_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_scalarmult_primitive", &a, &b);
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>(
        "crypto_scalarmult_curve25519_ref10_implementation",
    );
    let _ = (c, r); // data symbol, not callable; presence is what matters
}

#[test]
fn rows119_122_scalarmult_ed25519() {
    let _ = getter("crypto_scalarmult_ed25519_bytes");
    let _ = getter("crypto_scalarmult_ed25519_scalarbytes");
    let mut rng = Rng::seeded();

    for name in [
        "crypto_scalarmult_ed25519_base",
        "crypto_scalarmult_ed25519_base_noclamp",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(name);
        for _ in 0..200 {
            let n = nonzero_scalar(&mut rng);
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                same_ret(name, c(a.as_mut_ptr(), n.as_ptr()), r(b.as_mut_ptr(), n.as_ptr()));
            }
            same_bytes(name, &a, &b);
        }
    }
    for name in ["crypto_scalarmult_ed25519", "crypto_scalarmult_ed25519_noclamp"] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        for _ in 0..200 {
            let ps = nonzero_scalar(&mut rng);
            let p = ed25519_point(&ps);
            let n = nonzero_scalar(&mut rng);
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                same_ret(
                    name,
                    c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                );
            }
            same_bytes(name, &a, &b);
        }
    }
}

#[test]
fn rows123_124_scalarmult_ristretto255() {
    let _ = getter("crypto_scalarmult_ristretto255_bytes");
    let _ = getter("crypto_scalarmult_ristretto255_scalarbytes");
    let mut rng = Rng::seeded();
    let (bc, brr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_scalarmult_ristretto255_base",
    );
    for _ in 0..200 {
        let n = nonzero_scalar(&mut rng);
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret(
                "ristretto255_base",
                bc(a.as_mut_ptr(), n.as_ptr()),
                brr(b.as_mut_ptr(), n.as_ptr()),
            );
        }
        same_bytes("ristretto255_base", &a, &b);
    }
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(
        "crypto_scalarmult_ristretto255",
    );
    for _ in 0..200 {
        let ps = nonzero_scalar(&mut rng);
        let p = ristretto_point(&ps);
        let n = nonzero_scalar(&mut rng);
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret(
                "ristretto255",
                c(a.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                r(b.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
            );
        }
        same_bytes("ristretto255", &a, &b);
    }
}

// ------------------------------------------------- rows 125-139: core ed25519

#[test]
fn rows125_134_core_ed25519() {
    for g in [
        "crypto_core_ed25519_bytes",
        "crypto_core_ed25519_uniformbytes",
        "crypto_core_ed25519_hashbytes",
        "crypto_core_ed25519_scalarbytes",
        "crypto_core_ed25519_nonreducedscalarbytes",
    ] {
        let _ = getter(g);
    }
    let mut rng = Rng::seeded();

    // is_valid_point over valid main-subgroup points AND arbitrary 32-byte blobs
    let (vc, vr) = pair::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ed25519_is_valid_point",
    );
    for _ in 0..400 {
        let s = nonzero_scalar(&mut rng);
        let p = ed25519_point(&s);
        unsafe { same_ret("is_valid_point valid", vc(p.as_ptr()), vr(p.as_ptr())) };
        let junk = rng.bytes(32);
        unsafe { same_ret("is_valid_point junk", vc(junk.as_ptr()), vr(junk.as_ptr())) };
    }

    // add / sub over valid on-curve points (and, for coverage, junk pairs)
    for name in ["crypto_core_ed25519_add", "crypto_core_ed25519_sub"] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        for _ in 0..300 {
            let p = ed25519_point(&nonzero_scalar(&mut rng));
            let q = ed25519_point(&nonzero_scalar(&mut rng));
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                same_ret(
                    name,
                    c(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                    r(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                );
            }
            same_bytes(name, &a, &b);
        }
    }

    // random / scalar_random: nondeterministic, so check the outputs are
    // accepted identically by both is_valid_point / is_canonical.
    let (rc, rr) = pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_ed25519_random");
    let (cc, cr) = pair::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ed25519_scalar_is_canonical",
    );
    let (src, srr) = pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_ed25519_scalar_random");
    for _ in 0..100 {
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            rc(a.as_mut_ptr());
            rr(b.as_mut_ptr());
            // each side's own point must validate in BOTH implementations
            for p in [&a, &b] {
                same_ret("random point validity", vc(p.as_ptr()), vr(p.as_ptr()));
                assert_eq!(vc(p.as_ptr()), 1, "core_ed25519_random produced invalid point");
            }
            let mut sa = buf(32);
            let mut sb = buf(32);
            src(sa.as_mut_ptr());
            srr(sb.as_mut_ptr());
            for s in [&sa, &sb] {
                same_ret("scalar_random canonical", cc(s.as_ptr()), cr(s.as_ptr()));
                assert_eq!(cc(s.as_ptr()), 1, "scalar_random not canonical");
            }
        }
    }

    // scalar_is_canonical over crafted boundary values
    // L = 2^252 + 27742317777372353535851937790883648493
    let l: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    let mut cands: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xff; 32], l.to_vec()];
    let mut lm1 = l;
    lm1[0] -= 1;
    cands.push(lm1.to_vec());
    let mut lp1 = l;
    lp1[0] += 1;
    cands.push(lp1.to_vec());
    for _ in 0..300 {
        cands.push(rng.bytes(32));
    }
    for s in &cands {
        unsafe { same_ret("scalar_is_canonical", cc(s.as_ptr()), cr(s.as_ptr())) };
    }

    // scalar_invert (nonzero), and the void scalar ops
    let (ivc, ivr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_core_ed25519_scalar_invert",
    );
    for s in &cands {
        if s.iter().all(|&x| x == 0) {
            continue;
        }
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret("scalar_invert", ivc(a.as_mut_ptr(), s.as_ptr()), ivr(b.as_mut_ptr(), s.as_ptr()));
        }
        same_bytes("scalar_invert", &a, &b);
    }
    for name in [
        "crypto_core_ed25519_scalar_negate",
        "crypto_core_ed25519_scalar_complement",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8)>(name);
        for s in &cands {
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                c(a.as_mut_ptr(), s.as_ptr());
                r(b.as_mut_ptr(), s.as_ptr());
            }
            same_bytes(name, &a, &b);
        }
    }
    for name in [
        "crypto_core_ed25519_scalar_add",
        "crypto_core_ed25519_scalar_sub",
        "crypto_core_ed25519_scalar_mul",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>(name);
        for i in 0..cands.len() {
            let x = &cands[i];
            let y = &cands[(i * 7 + 3) % cands.len()];
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                c(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                r(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
            }
            same_bytes(name, &a, &b);
        }
    }
    // scalar_reduce: 64-byte input
    let (rdc, rdr) = pair::<unsafe extern "C" fn(*mut u8, *const u8)>(
        "crypto_core_ed25519_scalar_reduce",
    );
    let mut wide: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xff; 64]];
    for _ in 0..300 {
        wide.push(rng.bytes(64));
    }
    for s in &wide {
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            rdc(a.as_mut_ptr(), s.as_ptr());
            rdr(b.as_mut_ptr(), s.as_ptr());
        }
        same_bytes("scalar_reduce", &a, &b);
    }
    // from_string / from_string_nu / scalar_from_string with each valid hash_alg
    for name in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, Sz, *const u8, Sz, c_int) -> c_int,
        >(name);
        for &alg in &[H2C_SHA256, H2C_SHA512] {
            for &cl in &[0usize, 1, 8, 32] {
                for &ml in &[0usize, 1, 16, 64, 200] {
                    let ctx = rng.bytes(cl.max(1));
                    let msg = rng.bytes(ml.max(1));
                    let cp = if cl == 0 { std::ptr::null() } else { ctx.as_ptr() };
                    let mp = if ml == 0 { std::ptr::null() } else { msg.as_ptr() };
                    let mut a = buf(32);
                    let mut b = buf(32);
                    unsafe {
                        same_ret(
                            name,
                            c(a.as_mut_ptr(), cp, cl, mp, ml, alg),
                            r(b.as_mut_ptr(), cp, cl, mp, ml, alg),
                        );
                    }
                    same_bytes(&format!("{name} alg={alg} cl={cl} ml={ml}"), &a, &b);
                }
            }
        }
    }
}

#[test]
fn rows135_139_core_ristretto255() {
    for g in [
        "crypto_core_ristretto255_bytes",
        "crypto_core_ristretto255_hashbytes",
        "crypto_core_ristretto255_scalarbytes",
        "crypto_core_ristretto255_nonreducedscalarbytes",
    ] {
        let _ = getter(g);
    }
    let mut rng = Rng::seeded();
    let (vc, vr) = pair::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ristretto255_is_valid_point",
    );
    for _ in 0..400 {
        let p = ristretto_point(&nonzero_scalar(&mut rng));
        unsafe { same_ret("ristretto is_valid_point", vc(p.as_ptr()), vr(p.as_ptr())) };
        let junk = rng.bytes(32);
        unsafe { same_ret("ristretto is_valid junk", vc(junk.as_ptr()), vr(junk.as_ptr())) };
    }
    for name in ["crypto_core_ristretto255_add", "crypto_core_ristretto255_sub"] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(name);
        for _ in 0..300 {
            let p = ristretto_point(&nonzero_scalar(&mut rng));
            let q = ristretto_point(&nonzero_scalar(&mut rng));
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                same_ret(
                    name,
                    c(a.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                    r(b.as_mut_ptr(), p.as_ptr(), q.as_ptr()),
                );
            }
            same_bytes(name, &a, &b);
        }
    }
    // from_hash: 64-byte input, fully deterministic
    let (fc, fr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_core_ristretto255_from_hash",
    );
    for _ in 0..400 {
        let h = rng.bytes(64);
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret("ristretto from_hash", fc(a.as_mut_ptr(), h.as_ptr()), fr(b.as_mut_ptr(), h.as_ptr()));
        }
        same_bytes("ristretto from_hash", &a, &b);
    }
    for h in [vec![0u8; 64], vec![0xff; 64]] {
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret("from_hash extreme", fc(a.as_mut_ptr(), h.as_ptr()), fr(b.as_mut_ptr(), h.as_ptr()));
        }
        same_bytes("from_hash extreme", &a, &b);
    }
    // random: nondeterministic; both outputs must validate in both impls
    let (rc, rr) = pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_ristretto255_random");
    for _ in 0..100 {
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            rc(a.as_mut_ptr());
            rr(b.as_mut_ptr());
            for p in [&a, &b] {
                same_ret("ristretto random validity", vc(p.as_ptr()), vr(p.as_ptr()));
                assert_eq!(vc(p.as_ptr()), 1);
            }
        }
    }
    // scalar ops
    let mut cands: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xff; 32]];
    for _ in 0..300 {
        cands.push(rng.bytes(32));
    }
    let (cc, cr) = pair::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ristretto255_scalar_is_canonical",
    );
    for s in &cands {
        unsafe { same_ret("ristretto scalar_is_canonical", cc(s.as_ptr()), cr(s.as_ptr())) };
    }
    let (ivc, ivr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_core_ristretto255_scalar_invert",
    );
    for s in &cands {
        if s.iter().all(|&x| x == 0) {
            continue;
        }
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret("ristretto scalar_invert", ivc(a.as_mut_ptr(), s.as_ptr()), ivr(b.as_mut_ptr(), s.as_ptr()));
        }
        same_bytes("ristretto scalar_invert", &a, &b);
    }
    for name in [
        "crypto_core_ristretto255_scalar_negate",
        "crypto_core_ristretto255_scalar_complement",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8)>(name);
        for s in &cands {
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                c(a.as_mut_ptr(), s.as_ptr());
                r(b.as_mut_ptr(), s.as_ptr());
            }
            same_bytes(name, &a, &b);
        }
    }
    for name in [
        "crypto_core_ristretto255_scalar_add",
        "crypto_core_ristretto255_scalar_sub",
        "crypto_core_ristretto255_scalar_mul",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>(name);
        for i in 0..cands.len() {
            let x = &cands[i];
            let y = &cands[(i * 5 + 1) % cands.len()];
            let mut a = buf(32);
            let mut b = buf(32);
            unsafe {
                c(a.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                r(b.as_mut_ptr(), x.as_ptr(), y.as_ptr());
            }
            same_bytes(name, &a, &b);
        }
    }
    let (rdc, rdr) = pair::<unsafe extern "C" fn(*mut u8, *const u8)>(
        "crypto_core_ristretto255_scalar_reduce",
    );
    for _ in 0..300 {
        let s = rng.bytes(64);
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            rdc(a.as_mut_ptr(), s.as_ptr());
            rdr(b.as_mut_ptr(), s.as_ptr());
        }
        same_bytes("ristretto scalar_reduce", &a, &b);
    }
    let (src, srr) =
        pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_ristretto255_scalar_random");
    for _ in 0..50 {
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            src(a.as_mut_ptr());
            srr(b.as_mut_ptr());
            for s in [&a, &b] {
                same_ret("ristretto scalar_random", cc(s.as_ptr()), cr(s.as_ptr()));
            }
        }
    }
    // from_string / scalar_from_string
    for name in [
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, Sz, *const u8, Sz, c_int) -> c_int,
        >(name);
        for &alg in &[H2C_SHA256, H2C_SHA512] {
            for &cl in &[0usize, 8, 32] {
                for &ml in &[0usize, 1, 64, 200] {
                    let ctx = rng.bytes(cl.max(1));
                    let msg = rng.bytes(ml.max(1));
                    let cp = if cl == 0 { std::ptr::null() } else { ctx.as_ptr() };
                    let mp = if ml == 0 { std::ptr::null() } else { msg.as_ptr() };
                    let mut a = buf(32);
                    let mut b = buf(32);
                    unsafe {
                        same_ret(
                            name,
                            c(a.as_mut_ptr(), cp, cl, mp, ml, alg),
                            r(b.as_mut_ptr(), cp, cl, mp, ml, alg),
                        );
                    }
                    same_bytes(&format!("{name} alg={alg}"), &a, &b);
                }
            }
        }
    }
}

// -------------------------------------------------------- rows 140-152: sign

#[test]
fn rows140_152_sign() {
    for g in [
        "crypto_sign_bytes",
        "crypto_sign_seedbytes",
        "crypto_sign_publickeybytes",
        "crypto_sign_secretkeybytes",
        "crypto_sign_messagebytes_max",
        "crypto_sign_statebytes",
        "crypto_sign_ed25519_bytes",
        "crypto_sign_ed25519_seedbytes",
        "crypto_sign_ed25519_publickeybytes",
        "crypto_sign_ed25519_secretkeybytes",
        "crypto_sign_ed25519_messagebytes_max",
        "crypto_sign_ed25519ph_statebytes",
    ] {
        let _ = getter(g);
    }
    let statebytes = getter("crypto_sign_statebytes");
    let mut rng = Rng::seeded();

    let mlens = [0usize, 1, 2, 31, 32, 63, 64, 65, 127, 128, 1000, 4096];

    for prefix in ["crypto_sign", "crypto_sign_ed25519"] {
        let (skc, skr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
            &format!("{prefix}_seed_keypair"),
        );
        let (kpc, kpr) =
            pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{prefix}_keypair"));
        let (sc, sr) = pair::<
            unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int,
        >(prefix);
        let (dc, dr) = pair::<
            unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_detached"));
        let (oc, or) = pair::<
            unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_open"));
        let (vc, vr) = pair::<
            unsafe extern "C" fn(*const u8, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_verify_detached"));

        // keypair: nondeterministic, just make sure both succeed
        let mut pk = buf(32);
        let mut sk = buf(64);
        unsafe {
            same_ret(
                &format!("{prefix}_keypair"),
                kpc(pk.as_mut_ptr(), sk.as_mut_ptr()),
                kpr(pk.as_mut_ptr(), sk.as_mut_ptr()),
            );
        }

        for _ in 0..12 {
            let seed = rng.bytes(32);
            let mut pka = buf(32);
            let mut ska = buf(64);
            let mut pkb = buf(32);
            let mut skb = buf(64);
            unsafe {
                same_ret(
                    &format!("{prefix}_seed_keypair"),
                    skc(pka.as_mut_ptr(), ska.as_mut_ptr(), seed.as_ptr()),
                    skr(pkb.as_mut_ptr(), skb.as_mut_ptr(), seed.as_ptr()),
                );
            }
            same_bytes(&format!("{prefix}_seed_keypair pk"), &pka, &pkb);
            same_bytes(&format!("{prefix}_seed_keypair sk"), &ska, &skb);

            for &ml in &mlens {
                let m = rng.bytes(ml);
                // combined sign, smlen_p set / NULL
                for use_len in [true, false] {
                    let mut a = buf(ml + 64 + 8);
                    let mut b = buf(ml + 64 + 8);
                    let mut lc: c_ulonglong = 0xDEAD;
                    let mut lr: c_ulonglong = 0xDEAD;
                    unsafe {
                        same_ret(
                            prefix,
                            sc(
                                a.as_mut_ptr(),
                                if use_len { &mut lc } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                ska.as_ptr(),
                            ),
                            sr(
                                b.as_mut_ptr(),
                                if use_len { &mut lr } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                skb.as_ptr(),
                            ),
                        );
                    }
                    same_bytes(&format!("{prefix} sign ml={ml}"), &a, &b);
                    assert_eq!(lc, lr, "{prefix} smlen");
                }
                // canonical signed message
                let mut sm = buf(ml + 64);
                let mut smlen: c_ulonglong = 0;
                unsafe {
                    sc(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), ml as c_ulonglong, ska.as_ptr());
                }
                // open, m set / NULL, mlen_p set / NULL
                for use_m in [true, false] {
                    for use_len in [true, false] {
                        let mut a = buf(ml + 8);
                        let mut b = buf(ml + 8);
                        let mut lc: c_ulonglong = 0xDEAD;
                        let mut lr: c_ulonglong = 0xDEAD;
                        unsafe {
                            same_ret(
                                &format!("{prefix}_open"),
                                oc(
                                    if use_m { a.as_mut_ptr() } else { std::ptr::null_mut() },
                                    if use_len { &mut lc } else { std::ptr::null_mut() },
                                    sm.as_ptr(),
                                    smlen,
                                    pka.as_ptr(),
                                ),
                                or(
                                    if use_m { b.as_mut_ptr() } else { std::ptr::null_mut() },
                                    if use_len { &mut lr } else { std::ptr::null_mut() },
                                    sm.as_ptr(),
                                    smlen,
                                    pkb.as_ptr(),
                                ),
                            );
                        }
                        same_bytes(&format!("{prefix}_open ml={ml}"), &a, &b);
                        assert_eq!(lc, lr, "{prefix}_open mlen");
                        if use_m {
                            same_bytes(&format!("{prefix} sign round-trip"), &m, &a[..ml]);
                        }
                    }
                }
                // detached, siglen_p set / NULL
                for use_len in [true, false] {
                    let mut a = buf(64);
                    let mut b = buf(64);
                    let mut lc: c_ulonglong = 0xDEAD;
                    let mut lr: c_ulonglong = 0xDEAD;
                    unsafe {
                        same_ret(
                            &format!("{prefix}_detached"),
                            dc(
                                a.as_mut_ptr(),
                                if use_len { &mut lc } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                ska.as_ptr(),
                            ),
                            dr(
                                b.as_mut_ptr(),
                                if use_len { &mut lr } else { std::ptr::null_mut() },
                                m.as_ptr(),
                                ml as c_ulonglong,
                                skb.as_ptr(),
                            ),
                        );
                    }
                    same_bytes(&format!("{prefix}_detached ml={ml}"), &a, &b);
                    assert_eq!(lc, lr, "{prefix} siglen");
                    unsafe {
                        same_ret(
                            &format!("{prefix}_verify_detached ok"),
                            vc(a.as_ptr(), m.as_ptr(), ml as c_ulonglong, pka.as_ptr()),
                            vr(b.as_ptr(), m.as_ptr(), ml as c_ulonglong, pkb.as_ptr()),
                        );
                    }
                }
            }
        }
    }

    // rows 146-148: prehashed streaming API (both generic and ed25519ph names)
    for (init, upd, fcre, fver) in [
        (
            "crypto_sign_init",
            "crypto_sign_update",
            "crypto_sign_final_create",
            "crypto_sign_final_verify",
        ),
        (
            "crypto_sign_ed25519ph_init",
            "crypto_sign_ed25519ph_update",
            "crypto_sign_ed25519ph_final_create",
            "crypto_sign_ed25519ph_final_verify",
        ),
    ] {
        let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(init);
        let (uc, ur) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(upd);
        let (fc, fr) = pair::<
            unsafe extern "C" fn(*mut u8, *mut u8, *mut c_ulonglong, *const u8) -> c_int,
        >(fcre);
        let (vfc, vfr) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(fver);

        let seed = rng.bytes(32);
        let (skc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
            "crypto_sign_seed_keypair",
        );
        let mut pk = vec![0u8; 32];
        let mut sk = vec![0u8; 64];
        unsafe {
            skc(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }

        for &ml in &mlens {
            let m = rng.bytes(ml);
            for nch in [0usize, 1, 2, 3, 7] {
                let mut splits: Vec<usize> = Vec::new();
                for _ in 0..nch.saturating_sub(1) {
                    splits.push(rng.below(ml + 1));
                }
                splits.push(ml);
                splits.sort_unstable();

                let mut sca = buf(statebytes);
                let mut scb = buf(statebytes);
                unsafe {
                    same_ret(init, ic(sca.as_mut_ptr()), ir(scb.as_mut_ptr()));
                    let mut off = 0usize;
                    for &s in &splits {
                        let n = s - off;
                        same_ret(
                            upd,
                            uc(sca.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                            ur(scb.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                        );
                        off = s;
                    }
                }
                same_bytes(&format!("{upd} state ml={ml} n={nch}"), &sca, &scb);

                for use_len in [true, false] {
                    let mut s1 = sca.clone();
                    let mut s2 = scb.clone();
                    let mut a = buf(64);
                    let mut b = buf(64);
                    let mut lc: c_ulonglong = 0xDEAD;
                    let mut lr: c_ulonglong = 0xDEAD;
                    unsafe {
                        same_ret(
                            fcre,
                            fc(
                                s1.as_mut_ptr(),
                                a.as_mut_ptr(),
                                if use_len { &mut lc } else { std::ptr::null_mut() },
                                sk.as_ptr(),
                            ),
                            fr(
                                s2.as_mut_ptr(),
                                b.as_mut_ptr(),
                                if use_len { &mut lr } else { std::ptr::null_mut() },
                                sk.as_ptr(),
                            ),
                        );
                    }
                    same_bytes(&format!("{fcre} ml={ml} n={nch}"), &a, &b);
                    assert_eq!(lc, lr, "{fcre} siglen");

                    let mut s1 = sca.clone();
                    let mut s2 = scb.clone();
                    unsafe {
                        same_ret(
                            fver,
                            vfc(s1.as_mut_ptr(), a.as_ptr(), pk.as_ptr()),
                            vfr(s2.as_mut_ptr(), b.as_ptr(), pk.as_ptr()),
                        );
                    }
                }
            }
        }
    }

    // rows 149-151: key conversions
    let (skc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
        "crypto_sign_ed25519_seed_keypair",
    );
    let (s2sc, s2sr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_sign_ed25519_sk_to_seed",
    );
    let (s2pc, s2pr) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>("crypto_sign_ed25519_sk_to_pk");
    let (p2cc, p2cr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_sign_ed25519_pk_to_curve25519",
    );
    let (s2cc, s2cr) = pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(
        "crypto_sign_ed25519_sk_to_curve25519",
    );
    for _ in 0..100 {
        let seed = rng.bytes(32);
        let mut pk = vec![0u8; 32];
        let mut sk = vec![0u8; 64];
        unsafe {
            skc(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }
        for (name, c, r, ol, inp) in [
            ("sk_to_seed", s2sc.clone(), s2sr.clone(), 32usize, sk.clone()),
            ("sk_to_pk", s2pc.clone(), s2pr.clone(), 32, sk.clone()),
            ("pk_to_curve25519", p2cc.clone(), p2cr.clone(), 32, pk.clone()),
            ("sk_to_curve25519", s2cc.clone(), s2cr.clone(), 32, sk.clone()),
        ] {
            let mut a = buf(ol);
            let mut b = buf(ol);
            unsafe {
                same_ret(name, c(a.as_mut_ptr(), inp.as_ptr()), r(b.as_mut_ptr(), inp.as_ptr()));
            }
            same_bytes(name, &a, &b);
        }
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_sign_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_sign_primitive", &a, &b);
    }
}

// ---------------------------------------------------------- rows 153-159: kx

#[test]
fn rows153_159_kx() {
    for g in [
        "crypto_kx_publickeybytes",
        "crypto_kx_secretkeybytes",
        "crypto_kx_seedbytes",
        "crypto_kx_sessionkeybytes",
    ] {
        let _ = getter(g);
    }
    let (kpc, kpr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>("crypto_kx_keypair");
    let (skc, skr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(
        "crypto_kx_seed_keypair",
    );
    let (cc, cr) = pair::<
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int,
    >("crypto_kx_client_session_keys");
    let (sc, sr) = pair::<
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int,
    >("crypto_kx_server_session_keys");

    let mut pk = buf(32);
    let mut sk = buf(32);
    unsafe {
        same_ret("crypto_kx_keypair", kpc(pk.as_mut_ptr(), sk.as_mut_ptr()), kpr(pk.as_mut_ptr(), sk.as_mut_ptr()));
    }

    let mut rng = Rng::seeded();
    for _ in 0..60 {
        let cseed = rng.bytes(32);
        let sseed = rng.bytes(32);
        let mut cpk_a = buf(32);
        let mut csk_a = buf(32);
        let mut cpk_b = buf(32);
        let mut csk_b = buf(32);
        let mut spk_a = buf(32);
        let mut ssk_a = buf(32);
        let mut spk_b = buf(32);
        let mut ssk_b = buf(32);
        unsafe {
            same_ret(
                "kx_seed_keypair",
                skc(cpk_a.as_mut_ptr(), csk_a.as_mut_ptr(), cseed.as_ptr()),
                skr(cpk_b.as_mut_ptr(), csk_b.as_mut_ptr(), cseed.as_ptr()),
            );
            skc(spk_a.as_mut_ptr(), ssk_a.as_mut_ptr(), sseed.as_ptr());
            skr(spk_b.as_mut_ptr(), ssk_b.as_mut_ptr(), sseed.as_ptr());
        }
        same_bytes("kx_seed_keypair pk", &cpk_a, &cpk_b);
        same_bytes("kx_seed_keypair sk", &csk_a, &csk_b);
        same_bytes("kx_seed_keypair server pk", &spk_a, &spk_b);

        // rx and tx both set; rx NULL; tx NULL
        for (use_rx, use_tx) in [(true, true), (false, true), (true, false)] {
            let mut rxa = buf(32);
            let mut txa = buf(32);
            let mut rxb = buf(32);
            let mut txb = buf(32);
            unsafe {
                same_ret(
                    "kx_client_session_keys",
                    cc(
                        if use_rx { rxa.as_mut_ptr() } else { std::ptr::null_mut() },
                        if use_tx { txa.as_mut_ptr() } else { std::ptr::null_mut() },
                        cpk_a.as_ptr(),
                        csk_a.as_ptr(),
                        spk_a.as_ptr(),
                    ),
                    cr(
                        if use_rx { rxb.as_mut_ptr() } else { std::ptr::null_mut() },
                        if use_tx { txb.as_mut_ptr() } else { std::ptr::null_mut() },
                        cpk_b.as_ptr(),
                        csk_b.as_ptr(),
                        spk_b.as_ptr(),
                    ),
                );
            }
            same_bytes("kx client rx", &rxa, &rxb);
            same_bytes("kx client tx", &txa, &txb);

            let mut srxa = buf(32);
            let mut stxa = buf(32);
            let mut srxb = buf(32);
            let mut stxb = buf(32);
            unsafe {
                same_ret(
                    "kx_server_session_keys",
                    sc(
                        if use_rx { srxa.as_mut_ptr() } else { std::ptr::null_mut() },
                        if use_tx { stxa.as_mut_ptr() } else { std::ptr::null_mut() },
                        spk_a.as_ptr(),
                        ssk_a.as_ptr(),
                        cpk_a.as_ptr(),
                    ),
                    sr(
                        if use_rx { srxb.as_mut_ptr() } else { std::ptr::null_mut() },
                        if use_tx { stxb.as_mut_ptr() } else { std::ptr::null_mut() },
                        spk_b.as_ptr(),
                        ssk_b.as_ptr(),
                        cpk_b.as_ptr(),
                    ),
                );
            }
            same_bytes("kx server rx", &srxa, &srxb);
            same_bytes("kx server tx", &stxa, &stxb);

            if use_rx && use_tx {
                // row 159: full handshake agreement
                same_bytes("kx client rx == server tx", &rxa, &stxa);
                same_bytes("kx client tx == server rx", &txa, &srxa);
            }
        }
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_kx_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_kx_primitive", &a, &b);
    }
}

// -------------------------------------------------------- rows 160-171: kem

fn kem_family(prefix: &str, pkb: usize, skb: usize, ctb: usize, ssb: usize, seedb: usize, encseed: usize) {
    let (sc, sr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{prefix}_seed_keypair"
    ));
    let (kc, kr) =
        pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{prefix}_keypair"));
    let (ec, er) =
        pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!("{prefix}_enc"));
    // The generic `crypto_kem_*` façade does not expose enc_deterministic.
    let det = if libs().has(&format!("{prefix}_enc_deterministic")) {
        Some(pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int>(
            &format!("{prefix}_enc_deterministic"),
        ))
    } else {
        None
    };
    let (dc, dr) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(&format!("{prefix}_dec"));

    let mut rng = Rng::new(SEED ^ pkb as u64);
    // keypair: nondeterministic
    let mut pk = buf(pkb);
    let mut sk = buf(skb);
    unsafe {
        same_ret(
            &format!("{prefix}_keypair"),
            kc(pk.as_mut_ptr(), sk.as_mut_ptr()),
            kr(pk.as_mut_ptr(), sk.as_mut_ptr()),
        );
    }

    for _ in 0..12 {
        let seed = rng.bytes(seedb);
        let mut pka = buf(pkb);
        let mut ska = buf(skb);
        let mut pkb_ = buf(pkb);
        let mut skb_ = buf(skb);
        unsafe {
            same_ret(
                &format!("{prefix}_seed_keypair"),
                sc(pka.as_mut_ptr(), ska.as_mut_ptr(), seed.as_ptr()),
                sr(pkb_.as_mut_ptr(), skb_.as_mut_ptr(), seed.as_ptr()),
            );
        }
        same_bytes(&format!("{prefix}_seed_keypair pk"), &pka, &pkb_);
        same_bytes(&format!("{prefix}_seed_keypair sk"), &ska, &skb_);

        for _ in 0..6 {
            let Some((edc, edr)) = det.as_ref() else { break };
            let eseed = rng.bytes(encseed);
            let mut cta = buf(ctb);
            let mut ssa = buf(ssb);
            let mut ctb_ = buf(ctb);
            let mut ssb_ = buf(ssb);
            unsafe {
                same_ret(
                    &format!("{prefix}_enc_deterministic"),
                    edc(cta.as_mut_ptr(), ssa.as_mut_ptr(), pka.as_ptr(), eseed.as_ptr()),
                    edr(ctb_.as_mut_ptr(), ssb_.as_mut_ptr(), pkb_.as_ptr(), eseed.as_ptr()),
                );
            }
            same_bytes(&format!("{prefix}_enc_deterministic ct"), &cta, &ctb_);
            same_bytes(&format!("{prefix}_enc_deterministic ss"), &ssa, &ssb_);

            // dec on the valid ciphertext
            let mut da = buf(ssb);
            let mut db = buf(ssb);
            unsafe {
                same_ret(
                    &format!("{prefix}_dec"),
                    dc(da.as_mut_ptr(), cta.as_ptr(), ska.as_ptr()),
                    dr(db.as_mut_ptr(), ctb_.as_ptr(), skb_.as_ptr()),
                );
            }
            same_bytes(&format!("{prefix}_dec ss"), &da, &db);
            same_bytes(&format!("{prefix} round-trip ss"), &ssa, &da);

            // row 165/170: corrupted ciphertext -> implicit rejection; both
            // implementations must derive the SAME (pseudo-random) secret.
            for flip in [0usize, ctb / 2, ctb - 1] {
                let mut bad = cta.clone();
                bad[flip] ^= 0x5a;
                let mut da = buf(ssb);
                let mut db = buf(ssb);
                unsafe {
                    same_ret(
                        &format!("{prefix}_dec corrupt"),
                        dc(da.as_mut_ptr(), bad.as_ptr(), ska.as_ptr()),
                        dr(db.as_mut_ptr(), bad.as_ptr(), skb_.as_ptr()),
                    );
                }
                same_bytes(&format!("{prefix}_dec corrupt ss flip={flip}"), &da, &db);
            }
        }

        // random enc: nondeterministic, so cross-decapsulate
        let mut cta = buf(ctb);
        let mut ssa = buf(ssb);
        let mut ctb_ = buf(ctb);
        let mut ssb_ = buf(ssb);
        unsafe {
            same_ret(
                &format!("{prefix}_enc"),
                ec(cta.as_mut_ptr(), ssa.as_mut_ptr(), pka.as_ptr()),
                er(ctb_.as_mut_ptr(), ssb_.as_mut_ptr(), pkb_.as_ptr()),
            );
            for (label, ct, ss) in [("C-ct", &cta, &ssa), ("Rust-ct", &ctb_, &ssb_)] {
                let mut da = buf(ssb);
                let mut db = buf(ssb);
                same_ret(
                    &format!("{prefix}_dec {label}"),
                    dc(da.as_mut_ptr(), ct.as_ptr(), ska.as_ptr()),
                    dr(db.as_mut_ptr(), ct.as_ptr(), skb_.as_ptr()),
                );
                same_bytes(&format!("{prefix}_dec {label} ss"), &da, &db);
                same_bytes(&format!("{prefix} enc/dec agree {label}"), ss, &da);
            }
        }
    }
    for s in [
        "publickeybytes",
        "secretkeybytes",
        "ciphertextbytes",
        "sharedsecretbytes",
        "seedbytes",
    ] {
        let g = format!("{prefix}_{s}");
        if libs().has(&g) {
            let _ = getter(&g);
        }
    }
}

#[test]
fn rows160_165_kem_mlkem768() {
    kem_family("crypto_kem_mlkem768", 1184, 2400, 1088, 32, 64, 32);
}

#[test]
fn rows166_171_kem_xwing() {
    kem_family("crypto_kem_xwing", 1216, 32, 1120, 32, 32, 64);
    kem_family("crypto_kem", 1216, 32, 1120, 32, 32, 64);
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_kem_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_kem_primitive", &a, &b);
    }
}
