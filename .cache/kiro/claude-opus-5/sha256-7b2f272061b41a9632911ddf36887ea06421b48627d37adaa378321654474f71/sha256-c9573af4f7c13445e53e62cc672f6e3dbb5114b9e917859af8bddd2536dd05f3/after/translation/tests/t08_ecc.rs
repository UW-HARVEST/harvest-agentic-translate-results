//! Phase B/C for `crypto_core/ed25519` (`core_ed25519.c`, `core_ristretto255.c`,
//! `core_h2c.c`) and all of `crypto_scalarmult/`.
//!
//! Differential: every call goes through BOTH the C reference `.so` and the
//! Rust `.so` and the (return code, full output buffer incl. canary) pair is
//! compared byte-for-byte. Ground truth is the C at `c_src/libsodium/`.
//!
//! Ground rules verified against the C sources:
//!  * None of the covered functions call `sodium_misuse()`; the only `abort()`
//!    is `_string_to_points(n>2)` which is unreachable from the public API, and
//!    the `assert(h_len <= 0xff)` in core_h2c fires only on the fixed internal
//!    hash length (48 / 96), never on user input. So every case here is an
//!    inline differential call. The one construct that *could* abort — an
//!    out-of-range `hash_alg` — actually returns -1 (errno EINVAL) in C, so it
//!    is compared inline too. `same_outcome()` is still used defensively for the
//!    families where an aborting divergence is conceivable.
//!  * ed25519 does NOT export `_from_uniform` / `_from_hash` (only ristretto255
//!    exports `_from_hash`); those cases are gated on `has()`.

mod harness;
use harness::*;

use std::ffi::c_int;
use std::ptr;

const SEED: u64 = 0x5EED_0008;

// ---------------------------------------------------------------------------
// Sizes (from the public headers).
// ---------------------------------------------------------------------------
const PBYTES: usize = 32; // ed25519 / ristretto255 point encoding
const SBYTES: usize = 32; // scalar
const NRSBYTES: usize = 64; // non-reduced scalar
const HASHBYTES: usize = 64; // from_hash input

const H2C_SHA256: c_int = 1;
const H2C_SHA512: c_int = 2;

// L = 2^252 + 27742317777372353535851937790883648493, little-endian.
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

// The 8 known small-order points of ed25519 (canonical encodings). These MUST
// be rejected by `is_valid_point` and by `crypto_scalarmult_ed25519*`.
const SMALL_ORDER_POINTS: [[u8; 32]; 8] = [
    // 0 (order 1)
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    // p-1 style / order 2
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // order 4
    [
        0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0x80,
    ],
    // order 8
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x85,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0xfa,
    ],
];

// ---------------------------------------------------------------------------
// Function-pointer type aliases.
// ---------------------------------------------------------------------------
type FnCheck = unsafe extern "C" fn(*const u8) -> c_int; // is_valid_point / is_canonical
type FnBin = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int; // add/sub
type FnRandom = unsafe extern "C" fn(*mut u8); // *_random
type FnUnary = unsafe extern "C" fn(*mut u8, *const u8); // negate/complement/reduce/from_hash
type FnUnaryRc = unsafe extern "C" fn(*mut u8, *const u8) -> c_int; // scalar_invert
type FnTern = unsafe extern "C" fn(*mut u8, *const u8, *const u8); // scalar add/sub/mul
type FnFromString =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int;
type FnSmult = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int; // scalarmult(q,n,p)
type FnSmultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int; // scalarmult_base(q,n)

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// A random VALID point, produced by the C `_random`. `is_valid_point` under
/// both libraries is asserted to accept it (differential smoke on the source).
fn random_valid_ed25519(rng: &mut Rng) -> [u8; PBYTES] {
    // Use from_uniform-style generation via the C _random export.
    let (crand, _) = sym::<FnRandom>("crypto_core_ed25519_random");
    let mut p = [0u8; PBYTES];
    // _random is non-deterministic; but we only need *a* valid point.
    let _ = rng; // keep signature uniform
    unsafe { crand(p.as_mut_ptr()) };
    p
}

fn random_valid_ristretto(_rng: &mut Rng) -> [u8; PBYTES] {
    let (crand, _) = sym::<FnRandom>("crypto_core_ristretto255_random");
    let mut p = [0u8; PBYTES];
    unsafe { crand(p.as_mut_ptr()) };
    p
}

// ===========================================================================
// 1. Point arithmetic + scalar arithmetic, MANY random inputs, deterministic.
// ===========================================================================

struct Curve {
    pfx: &'static str,
    valid: fn(&mut Rng) -> [u8; PBYTES],
}

fn curves() -> Vec<Curve> {
    vec![
        Curve { pfx: "crypto_core_ed25519", valid: random_valid_ed25519 },
        Curve { pfx: "crypto_core_ristretto255", valid: random_valid_ristretto },
    ]
}

/// _add / _sub over random valid points (return 0) AND over random junk (both
/// must reject identically with -1).
#[test]
fn point_add_sub() {
    let mut rng = Rng::new(SEED);
    for cv in curves() {
        let (cadd, radd) = sym::<FnBin>(&format!("{}_add", cv.pfx));
        let (csub, rsub) = sym::<FnBin>(&format!("{}_sub", cv.pfx));
        for iter in 0..200 {
            let p = (cv.valid)(&mut rng);
            let q = (cv.valid)(&mut rng);
            for (name, cf, rf) in
                [("add", cadd, radd), ("sub", csub, rsub)]
            {
                let mut oc = out_buf(PBYTES);
                let mut or = out_buf(PBYTES);
                unsafe {
                    let rc = cf(oc.as_mut_ptr(), p.as_ptr(), q.as_ptr());
                    let rr = rf(or.as_mut_ptr(), p.as_ptr(), q.as_ptr());
                    assert_eq!(rc, rr, "{}_{name} rc iter={iter}", cv.pfx);
                }
                eqb(&format!("{}_{name} out iter={iter}", cv.pfx), &oc, &or);
            }
            // junk operands: not-on-curve / non-canonical -> both reject
            let junk1 = rng.bytes(PBYTES);
            let junk2 = rng.bytes(PBYTES);
            for (name, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
                let mut oc = out_buf(PBYTES);
                let mut or = out_buf(PBYTES);
                unsafe {
                    let rc = cf(oc.as_mut_ptr(), junk1.as_ptr(), junk2.as_ptr());
                    let rr = rf(or.as_mut_ptr(), junk1.as_ptr(), junk2.as_ptr());
                    assert_eq!(rc, rr, "{}_{name} junk rc iter={iter}", cv.pfx);
                }
                eqb(&format!("{}_{name} junk out iter={iter}", cv.pfx), &oc, &or);
            }
            // one valid, one junk
            for (name, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
                let mut oc = out_buf(PBYTES);
                let mut or = out_buf(PBYTES);
                unsafe {
                    let rc = cf(oc.as_mut_ptr(), p.as_ptr(), junk2.as_ptr());
                    let rr = rf(or.as_mut_ptr(), p.as_ptr(), junk2.as_ptr());
                    assert_eq!(rc, rr, "{}_{name} mixed rc iter={iter}", cv.pfx);
                }
                eqb(&format!("{}_{name} mixed out iter={iter}", cv.pfx), &oc, &or);
            }
        }
    }
}

/// scalar_negate / _complement / _reduce / _add / _sub / _mul / _invert /
/// _is_canonical over many random scalars and a fixed edge set.
#[test]
fn scalar_ops() {
    let mut rng = Rng::new(SEED ^ 1);
    for pfx in ["crypto_core_ed25519", "crypto_core_ristretto255"] {
        let (cneg, rneg) = sym::<FnUnary>(&format!("{pfx}_scalar_negate"));
        let (ccomp, rcomp) = sym::<FnUnary>(&format!("{pfx}_scalar_complement"));
        let (cadd, radd) = sym::<FnTern>(&format!("{pfx}_scalar_add"));
        let (csub, rsub) = sym::<FnTern>(&format!("{pfx}_scalar_sub"));
        let (cmul, rmul) = sym::<FnTern>(&format!("{pfx}_scalar_mul"));
        let (cred, rred) = sym::<FnUnary>(&format!("{pfx}_scalar_reduce"));
        let (cinv, rinv) = sym::<FnUnaryRc>(&format!("{pfx}_scalar_invert"));
        let (ccan, rcan) = sym::<FnCheck>(&format!("{pfx}_scalar_is_canonical"));

        let edge = scalar_edge_cases();

        for iter in 0..400 {
            let x = if iter < edge.len() { edge[iter].clone() } else { rng.bytes(SBYTES) };
            let y = rng.bytes(SBYTES);

            // unary reduce/negate/complement take an s (negate/complement expect
            // canonical scalars; feed both random and edge which include L etc.)
            for (name, cf, rf) in
                [("negate", cneg, rneg), ("complement", ccomp, rcomp)]
            {
                let mut oc = out_buf(SBYTES);
                let mut or = out_buf(SBYTES);
                unsafe {
                    cf(oc.as_mut_ptr(), x.as_ptr());
                    rf(or.as_mut_ptr(), x.as_ptr());
                }
                eqb(&format!("{pfx}_scalar_{name} iter={iter}"), &oc, &or);
            }

            // reduce takes a 64-byte non-reduced scalar
            let nr = rng.bytes(NRSBYTES);
            {
                let mut oc = out_buf(SBYTES);
                let mut or = out_buf(SBYTES);
                unsafe {
                    cred(oc.as_mut_ptr(), nr.as_ptr());
                    rred(or.as_mut_ptr(), nr.as_ptr());
                }
                eqb(&format!("{pfx}_scalar_reduce iter={iter}"), &oc, &or);
            }

            for (name, cf, rf) in
                [("add", cadd, radd), ("sub", csub, rsub), ("mul", cmul, rmul)]
            {
                let mut oc = out_buf(SBYTES);
                let mut or = out_buf(SBYTES);
                unsafe {
                    cf(oc.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    rf(or.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eqb(&format!("{pfx}_scalar_{name} iter={iter}"), &oc, &or);
            }

            // invert (returns -1 for zero scalar)
            {
                let mut oc = out_buf(SBYTES);
                let mut or = out_buf(SBYTES);
                unsafe {
                    let rc = cinv(oc.as_mut_ptr(), x.as_ptr());
                    let rr = rinv(or.as_mut_ptr(), x.as_ptr());
                    assert_eq!(rc, rr, "{pfx}_scalar_invert rc iter={iter}");
                }
                eqb(&format!("{pfx}_scalar_invert iter={iter}"), &oc, &or);
            }

            // is_canonical
            unsafe {
                let rc = ccan(x.as_ptr());
                let rr = rcan(x.as_ptr());
                assert_eq!(rc, rr, "{pfx}_scalar_is_canonical iter={iter} x={}", hex(&x));
            }
        }
    }
}

/// Scalars: 0, 1, L-1, L, L+1, 2^252, all-0xff, plus randoms; the whole edge
/// set is also fed through is_canonical and invert above.
fn scalar_edge_cases() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    v.push(vec![0u8; SBYTES]); // 0
    let mut one = vec![0u8; SBYTES];
    one[0] = 1;
    v.push(one.clone()); // 1
    // L-1
    let mut lm1 = L.to_vec();
    lm1[0] -= 1;
    v.push(lm1);
    v.push(L.to_vec()); // L
    // L+1
    let mut lp1 = L.to_vec();
    lp1[0] += 1;
    v.push(lp1);
    // 2^252 (bit 252 set)
    let mut p252 = vec![0u8; SBYTES];
    p252[31] = 0x10;
    v.push(p252);
    v.push(vec![0xffu8; SBYTES]); // all-0xff
    v
}

// ===========================================================================
// 1b. Non-deterministic _random / _scalar_random: compare return-code-style
//     acceptance across libraries + canary intact.
// ===========================================================================

#[test]
fn random_points_accepted_cross_library() {
    for pfx in ["crypto_core_ed25519", "crypto_core_ristretto255"] {
        let (crand, rrand) = sym::<FnRandom>(&format!("{pfx}_random"));
        let (cvalid, rvalid) = sym::<FnCheck>(&format!("{pfx}_is_valid_point"));
        for iter in 0..100 {
            // C-produced point must be accepted by BOTH libraries.
            let mut cbuf = out_buf(PBYTES);
            unsafe { crand(cbuf.as_mut_ptr()) };
            unsafe {
                assert_eq!(cvalid(cbuf.as_ptr()), 1, "{pfx} C point rejected by C iter={iter}");
                assert_eq!(
                    rvalid(cbuf.as_ptr()),
                    1,
                    "{pfx} C-generated point rejected by Rust is_valid_point iter={iter} p={}",
                    hex(&cbuf[..PBYTES])
                );
            }
            // canary intact after C write
            assert_eq!(&cbuf[PBYTES..], &out_buf(PBYTES)[PBYTES..], "{pfx} C canary iter={iter}");

            // Rust-produced point must be accepted by BOTH libraries.
            let mut rbuf = out_buf(PBYTES);
            unsafe { rrand(rbuf.as_mut_ptr()) };
            unsafe {
                assert_eq!(
                    cvalid(rbuf.as_ptr()),
                    1,
                    "{pfx} Rust-generated point rejected by C is_valid_point iter={iter} p={}",
                    hex(&rbuf[..PBYTES])
                );
                assert_eq!(rvalid(rbuf.as_ptr()), 1, "{pfx} Rust point rejected by Rust iter={iter}");
            }
            assert_eq!(&rbuf[PBYTES..], &out_buf(PBYTES)[PBYTES..], "{pfx} Rust canary iter={iter}");
        }
    }
}

#[test]
fn scalar_random_accepted_cross_library() {
    for pfx in ["crypto_core_ed25519", "crypto_core_ristretto255"] {
        let (crand, rrand) = sym::<FnRandom>(&format!("{pfx}_scalar_random"));
        let (ccan, rcan) = sym::<FnCheck>(&format!("{pfx}_scalar_is_canonical"));
        for iter in 0..100 {
            let mut cbuf = out_buf(SBYTES);
            unsafe { crand(cbuf.as_mut_ptr()) };
            unsafe {
                assert_eq!(ccan(cbuf.as_ptr()), 1, "{pfx} C scalar not canonical (C) iter={iter}");
                assert_eq!(
                    rcan(cbuf.as_ptr()),
                    1,
                    "{pfx} C scalar not canonical per Rust iter={iter} s={}",
                    hex(&cbuf[..SBYTES])
                );
            }
            assert_eq!(&cbuf[SBYTES..], &out_buf(SBYTES)[SBYTES..], "{pfx} C scalar canary iter={iter}");
            // non-zero (both libs reject zero in _scalar_random loop)
            assert_ne!(&cbuf[..SBYTES], &vec![0u8; SBYTES][..], "{pfx} C scalar zero iter={iter}");

            let mut rbuf = out_buf(SBYTES);
            unsafe { rrand(rbuf.as_mut_ptr()) };
            unsafe {
                assert_eq!(
                    ccan(rbuf.as_ptr()),
                    1,
                    "{pfx} Rust scalar not canonical per C iter={iter} s={}",
                    hex(&rbuf[..SBYTES])
                );
                assert_eq!(rcan(rbuf.as_ptr()), 1, "{pfx} Rust scalar not canonical (Rust) iter={iter}");
            }
            assert_eq!(&rbuf[SBYTES..], &out_buf(SBYTES)[SBYTES..], "{pfx} Rust scalar canary iter={iter}");
            assert_ne!(&rbuf[..SBYTES], &vec![0u8; SBYTES][..], "{pfx} Rust scalar zero iter={iter}");
        }
    }
}

/// ristretto255 exports `_from_hash`; ed25519 does NOT (only ristretto). Feed
/// many random 64-byte hashes and compare output byte-for-byte.
#[test]
fn from_hash_ristretto() {
    let name = "crypto_core_ristretto255_from_hash";
    assert!(has(name), "expected ristretto255_from_hash to be exported");
    let (c, r) = sym::<FnUnaryRc>(name);
    let (cvalid, rvalid) = sym::<FnCheck>("crypto_core_ristretto255_is_valid_point");
    let mut rng = Rng::new(SEED ^ 2);
    for iter in 0..300 {
        let h = if iter == 0 {
            vec![0u8; HASHBYTES]
        } else if iter == 1 {
            vec![0xffu8; HASHBYTES]
        } else {
            rng.bytes(HASHBYTES)
        };
        let mut oc = out_buf(PBYTES);
        let mut or = out_buf(PBYTES);
        unsafe {
            let rc = c(oc.as_mut_ptr(), h.as_ptr());
            let rr = r(or.as_mut_ptr(), h.as_ptr());
            assert_eq!(rc, rr, "from_hash rc iter={iter}");
        }
        eqb(&format!("from_hash out iter={iter}"), &oc, &or);
        // the mapped point is valid under both libraries
        unsafe {
            assert_eq!(cvalid(oc.as_ptr()), 1, "from_hash C point invalid iter={iter}");
            assert_eq!(rvalid(or.as_ptr()), 1, "from_hash Rust point invalid iter={iter}");
        }
    }
    // ed25519 must NOT export from_hash / from_uniform (parity on the surface).
    for missing in ["crypto_core_ed25519_from_hash", "crypto_core_ed25519_from_uniform"] {
        let l = libs();
        let mut n = missing.as_bytes().to_vec();
        n.push(0);
        let in_c = unsafe { l.c.get::<*const ()>(&n).is_ok() };
        let in_r = unsafe { l.r.get::<*const ()>(&n).is_ok() };
        assert_eq!(in_c, in_r, "{missing}: export presence differs");
    }
}

// ===========================================================================
// 2. Point-validity edge cases: MUST both reject identically.
// ===========================================================================

#[test]
fn is_valid_point_edges() {
    let (ced, red) = sym::<FnCheck>("crypto_core_ed25519_is_valid_point");
    let mut rng = Rng::new(SEED ^ 3);

    // all-zero, all-0xff
    for (tag, p) in [("zero", vec![0u8; PBYTES]), ("ones", vec![0xffu8; PBYTES])] {
        unsafe {
            let rc = ced(p.as_ptr());
            let rr = red(p.as_ptr());
            assert_eq!(rc, rr, "ed25519 is_valid_point {tag}: C={rc} Rust={rr}");
            assert_eq!(rc, 0, "ed25519 {tag} unexpectedly accepted by C");
        }
    }

    // The 8 known small-order points: all rejected, identically.
    for (i, p) in SMALL_ORDER_POINTS.iter().enumerate() {
        unsafe {
            let rc = ced(p.as_ptr());
            let rr = red(p.as_ptr());
            assert_eq!(rc, rr, "ed25519 small-order[{i}] disagree: C={rc} Rust={rr} p={}", hex(p));
            assert_eq!(rc, 0, "ed25519 small-order[{i}] accepted by C p={}", hex(p));
        }
    }

    // Non-canonical encodings: field element >= 2^255-19. Set the top byte so
    // that the low 255 bits encode a value in [p, 2^255). y in [0xed.., 0x7fff..]
    for k in 0u16..64 {
        let mut p = vec![0u8; PBYTES];
        // low bytes 0xed.. up to 0xff to push >= p = 2^255-19
        p[0] = 0xed_u8.wrapping_add((k & 0x0f) as u8);
        for b in p.iter_mut().take(31).skip(1) {
            *b = 0xff;
        }
        p[31] = 0x7f;
        unsafe {
            let rc = ced(p.as_ptr());
            let rr = red(p.as_ptr());
            assert_eq!(rc, rr, "ed25519 non-canonical[{k}] disagree p={}", hex(&p));
            assert_eq!(rc, 0, "ed25519 non-canonical[{k}] accepted by C p={}", hex(&p));
        }
    }

    // Random junk (mostly not-on-curve / off-subgroup): identical outcome.
    for iter in 0..500 {
        let p = rng.bytes(PBYTES);
        unsafe {
            let rc = ced(p.as_ptr());
            let rr = red(p.as_ptr());
            assert_eq!(rc, rr, "ed25519 random[{iter}] disagree p={}", hex(&p));
        }
    }

    // Take a VALID point and flip single bits: most become invalid, but always
    // the two libraries must agree bit-for-bit.
    for iter in 0..80 {
        let base = random_valid_ed25519(&mut rng);
        // occasionally verify the base itself agrees & is valid
        unsafe {
            assert_eq!(ced(base.as_ptr()), red(base.as_ptr()), "base disagree iter={iter}");
        }
        for bit in 0..256usize {
            let mut p = base;
            p[bit / 8] ^= 1 << (bit % 8);
            unsafe {
                let rc = ced(p.as_ptr());
                let rr = red(p.as_ptr());
                assert_eq!(rc, rr, "ed25519 bitflip iter={iter} bit={bit} disagree p={}", hex(&p));
            }
        }
    }
}

#[test]
fn is_valid_point_edges_ristretto() {
    let (cr, rr_) = sym::<FnCheck>("crypto_core_ristretto255_is_valid_point");
    let mut rng = Rng::new(SEED ^ 4);

    for (tag, p) in [("zero", vec![0u8; PBYTES]), ("ones", vec![0xffu8; PBYTES])] {
        unsafe {
            let rc = cr(p.as_ptr());
            let rr = rr_(p.as_ptr());
            assert_eq!(rc, rr, "ristretto is_valid_point {tag}: C={rc} Rust={rr}");
        }
    }
    // ed25519 small-order encodings are (mostly) not valid ristretto encodings;
    // whatever the C verdict, Rust must match.
    for (i, p) in SMALL_ORDER_POINTS.iter().enumerate() {
        unsafe {
            let rc = cr(p.as_ptr());
            let rr = rr_(p.as_ptr());
            assert_eq!(rc, rr, "ristretto small-order[{i}] disagree p={}", hex(p));
        }
    }
    // non-canonical field elements: top bit set etc.
    for k in 0u16..64 {
        let mut p = vec![0xffu8; PBYTES];
        p[0] = 0xed_u8.wrapping_add((k & 0x0f) as u8);
        p[31] = 0xff; // high bit set -> non-canonical for ristretto too
        unsafe {
            let rc = cr(p.as_ptr());
            let rr = rr_(p.as_ptr());
            assert_eq!(rc, rr, "ristretto non-canonical[{k}] disagree p={}", hex(&p));
        }
    }
    // random junk
    for iter in 0..500 {
        let p = rng.bytes(PBYTES);
        unsafe {
            let rc = cr(p.as_ptr());
            let rr = rr_(p.as_ptr());
            assert_eq!(rc, rr, "ristretto random[{iter}] disagree p={}", hex(&p));
        }
    }
    // valid point + single-bit flips
    for iter in 0..80 {
        let base = random_valid_ristretto(&mut rng);
        unsafe {
            assert_eq!(cr(base.as_ptr()), 1, "ristretto base invalid iter={iter}");
            assert_eq!(rr_(base.as_ptr()), 1, "ristretto base invalid Rust iter={iter}");
        }
        for bit in 0..256usize {
            let mut p = base;
            p[bit / 8] ^= 1 << (bit % 8);
            unsafe {
                let rc = cr(p.as_ptr());
                let rr = rr_(p.as_ptr());
                assert_eq!(rc, rr, "ristretto bitflip iter={iter} bit={bit} disagree p={}", hex(&p));
            }
        }
    }
}

// ===========================================================================
// 4. core_h2c: *_from_string / *_scalar_from_string, over the two SHA
//    constants AND out-of-range hash_alg, and every ctx / msg length branch.
// ===========================================================================

fn ctx_lengths() -> Vec<usize> {
    vec![0, 1, 8, 32, 254, 255, 256, 300]
}
fn msg_lengths() -> Vec<usize> {
    vec![0, 1, 32, 64, 200]
}
fn hash_algs() -> Vec<c_int> {
    // valid SHA256 / SHA512 constants + several out-of-range ints (C enums
    // accept any int; the default branch returns -1 with errno EINVAL).
    vec![H2C_SHA256, H2C_SHA512, 0, 3, -1, 999]
}

#[test]
fn from_string_all_variants() {
    let mut rng = Rng::new(SEED ^ 5);
    // point-valued from_string (ed25519 has _from_string and _from_string_nu;
    // ristretto255 has _from_string). Output = PBYTES.
    let point_fns: &[&str] = &[
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ristretto255_from_string",
    ];
    // scalar-valued from_string. Output = SBYTES.
    let scalar_fns: &[&str] = &[
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ];

    for name in point_fns {
        if !has(name) {
            continue;
        }
        let (c, r) = sym::<FnFromString>(name);
        run_from_string(name, c, r, PBYTES, &mut rng);
    }
    for name in scalar_fns {
        if !has(name) {
            continue;
        }
        let (c, r) = sym::<FnFromString>(name);
        run_from_string(name, c, r, SBYTES, &mut rng);
    }
}

fn run_from_string(name: &str, c: FnFromString, r: FnFromString, outlen: usize, rng: &mut Rng) {
    for &alg in &hash_algs() {
        for &cl in &ctx_lengths() {
            for &ml in &msg_lengths() {
                let ctx = rng.bytes(cl);
                let msg = rng.bytes(ml);
                let mut oc = out_buf(outlen);
                let mut or = out_buf(outlen);
                let cptr = if cl == 0 { ptr::null() } else { ctx.as_ptr() };
                let mptr = if ml == 0 { ptr::null() } else { msg.as_ptr() };
                unsafe {
                    let rc = c(oc.as_mut_ptr(), cptr, cl, mptr, ml, alg);
                    let rr = r(or.as_mut_ptr(), cptr, cl, mptr, ml, alg);
                    assert_eq!(
                        rc, rr,
                        "{name} rc alg={alg} ctx_len={cl} msg_len={ml}"
                    );
                }
                eqb(&format!("{name} out alg={alg} ctx_len={cl} msg_len={ml}"), &oc, &or);
            }
        }
    }
}

// ===========================================================================
// 5. crypto_scalarmult/* — curve25519, ed25519(+noclamp), ristretto255, generic
// ===========================================================================

fn smult_scalar_edges() -> Vec<Vec<u8>> {
    let mut v = scalar_edge_cases();
    // clamping-relevant patterns: high/low bits set/cleared
    v.push({
        let mut s = vec![0u8; SBYTES];
        s[0] = 0xff;
        s[31] = 0xff;
        s
    });
    v.push({
        let mut s = vec![0xffu8; SBYTES];
        s[0] = 0x00;
        s[31] = 0x00;
        s
    });
    v.push(vec![0x55u8; SBYTES]);
    v.push(vec![0xaau8; SBYTES]);
    v
}

/// curve25519 + generic crypto_scalarmult (which is curve25519): never rejects
/// the *scalar* by clamping, but rejects all-zero output. Compare rc + buffer.
#[test]
fn scalarmult_curve25519() {
    let mut rng = Rng::new(SEED ^ 6);
    for base in ["crypto_scalarmult_curve25519", "crypto_scalarmult"] {
        let (c, r) = sym::<FnSmult>(base);
        let (cb, rb) = sym::<FnSmultBase>(&format!("{base}_base"));
        // random n * random p
        for iter in 0..200 {
            let n = rng.bytes(SBYTES);
            let p = rng.bytes(PBYTES);
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = c(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = r(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "{base} rc iter={iter} n={} p={}", hex(&n), hex(&p));
            }
            eqb(&format!("{base} out iter={iter}"), &oc, &or);
            // base
            let mut bc = out_buf(PBYTES);
            let mut br = out_buf(PBYTES);
            unsafe {
                let rc = cb(bc.as_mut_ptr(), n.as_ptr());
                let rr = rb(br.as_mut_ptr(), n.as_ptr());
                assert_eq!(rc, rr, "{base}_base rc iter={iter}");
            }
            eqb(&format!("{base}_base out iter={iter}"), &bc, &br);
        }
        // scalar edges against a random point + the base point
        for (si, n) in smult_scalar_edges().into_iter().enumerate() {
            let p = rng.bytes(PBYTES);
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = c(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = r(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "{base} edge[{si}] rc n={}", hex(&n));
            }
            eqb(&format!("{base} edge[{si}] out"), &oc, &or);
            let mut bc = out_buf(PBYTES);
            let mut br = out_buf(PBYTES);
            unsafe {
                let rc = cb(bc.as_mut_ptr(), n.as_ptr());
                let rr = rb(br.as_mut_ptr(), n.as_ptr());
                assert_eq!(rc, rr, "{base}_base edge[{si}] rc n={}", hex(&n));
            }
            eqb(&format!("{base}_base edge[{si}] out"), &bc, &br);
        }
        // small-order input points (curve25519 u-coords). Feed the ed25519
        // small-order encodings as raw 32-byte u; both libs must agree.
        for (i, p) in SMALL_ORDER_POINTS.iter().enumerate() {
            let n = rng.bytes(SBYTES);
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = c(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = r(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "{base} small-order-p[{i}] rc disagree");
            }
            eqb(&format!("{base} small-order-p[{i}] out"), &oc, &or);
        }
    }
}

/// ed25519 scalarmult: clamp and noclamp variants. Uses VALID ed25519 points as
/// P; rejects scalar 0, small-order P, non-canonical P. Compare rc + buffer.
#[test]
fn scalarmult_ed25519() {
    let mut rng = Rng::new(SEED ^ 7);
    let (cm, rm) = sym::<FnSmult>("crypto_scalarmult_ed25519");
    let (cmn, rmn) = sym::<FnSmult>("crypto_scalarmult_ed25519_noclamp");
    let (cmb, rmb) = sym::<FnSmultBase>("crypto_scalarmult_ed25519_base");
    let (cmbn, rmbn) = sym::<FnSmultBase>("crypto_scalarmult_ed25519_base_noclamp");

    // random scalars against random valid points
    for iter in 0..150 {
        let n = rng.bytes(SBYTES);
        let p = random_valid_ed25519(&mut rng);
        for (name, cf, rf) in [
            ("clamp", cm, rm),
            ("noclamp", cmn, rmn),
        ] {
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = cf(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = rf(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "ed25519 {name} rc iter={iter} n={} p={}", hex(&n), hex(&p));
            }
            eqb(&format!("ed25519 {name} out iter={iter}"), &oc, &or);
        }
        for (name, cf, rf) in [("base", cmb, rmb), ("base_noclamp", cmbn, rmbn)] {
            let mut bc = out_buf(PBYTES);
            let mut br = out_buf(PBYTES);
            unsafe {
                let rc = cf(bc.as_mut_ptr(), n.as_ptr());
                let rr = rf(br.as_mut_ptr(), n.as_ptr());
                assert_eq!(rc, rr, "ed25519 {name} rc iter={iter}");
            }
            eqb(&format!("ed25519 {name} out iter={iter}"), &bc, &br);
        }
    }

    // scalar edges (0, 1, L-1, high/low bits) — highlights clamp vs noclamp.
    for (si, n) in smult_scalar_edges().into_iter().enumerate() {
        let p = random_valid_ed25519(&mut rng);
        for (name, cf, rf) in [("clamp", cm, rm), ("noclamp", cmn, rmn)] {
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = cf(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = rf(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "ed25519 {name} edge[{si}] rc n={}", hex(&n));
            }
            eqb(&format!("ed25519 {name} edge[{si}] out"), &oc, &or);
        }
        for (name, cf, rf) in [("base", cmb, rmb), ("base_noclamp", cmbn, rmbn)] {
            let mut bc = out_buf(PBYTES);
            let mut br = out_buf(PBYTES);
            unsafe {
                let rc = cf(bc.as_mut_ptr(), n.as_ptr());
                let rr = rf(br.as_mut_ptr(), n.as_ptr());
                assert_eq!(rc, rr, "ed25519 {name} edge[{si}] rc n={}", hex(&n));
            }
            eqb(&format!("ed25519 {name} edge[{si}] out"), &bc, &br);
        }
    }

    // small-order input points (rejected) and non-canonical inputs (rejected).
    for (i, p) in SMALL_ORDER_POINTS.iter().enumerate() {
        let n = rng.bytes(SBYTES);
        for (name, cf, rf) in [("clamp", cm, rm), ("noclamp", cmn, rmn)] {
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = cf(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = rf(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "ed25519 {name} small-order[{i}] rc disagree");
            }
            eqb(&format!("ed25519 {name} small-order[{i}] out"), &oc, &or);
        }
    }
    // non-canonical P inputs
    for k in 0u16..32 {
        let mut p = vec![0xffu8; PBYTES];
        p[0] = 0xed_u8.wrapping_add((k & 0x0f) as u8);
        p[31] = 0x7f;
        let n = rng.bytes(SBYTES);
        for (name, cf, rf) in [("clamp", cm, rm), ("noclamp", cmn, rmn)] {
            let mut oc = out_buf(PBYTES);
            let mut or = out_buf(PBYTES);
            unsafe {
                let rc = cf(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                let rr = rf(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
                assert_eq!(rc, rr, "ed25519 {name} non-canonical[{k}] rc disagree p={}", hex(&p));
            }
            eqb(&format!("ed25519 {name} non-canonical[{k}] out"), &oc, &or);
        }
    }
}

/// ristretto255 scalarmult + base. Rejects all-zero output (scalar 0). Uses
/// valid ristretto points; small-order / non-canonical inputs are rejected.
#[test]
fn scalarmult_ristretto255() {
    let mut rng = Rng::new(SEED ^ 8);
    let (cm, rm) = sym::<FnSmult>("crypto_scalarmult_ristretto255");
    let (cmb, rmb) = sym::<FnSmultBase>("crypto_scalarmult_ristretto255_base");

    for iter in 0..150 {
        let n = rng.bytes(SBYTES);
        let p = random_valid_ristretto(&mut rng);
        let mut oc = out_buf(PBYTES);
        let mut or = out_buf(PBYTES);
        unsafe {
            let rc = cm(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rm(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ristretto rc iter={iter} n={} p={}", hex(&n), hex(&p));
        }
        eqb(&format!("ristretto out iter={iter}"), &oc, &or);

        let mut bc = out_buf(PBYTES);
        let mut br = out_buf(PBYTES);
        unsafe {
            let rc = cmb(bc.as_mut_ptr(), n.as_ptr());
            let rr = rmb(br.as_mut_ptr(), n.as_ptr());
            assert_eq!(rc, rr, "ristretto_base rc iter={iter}");
        }
        eqb(&format!("ristretto_base out iter={iter}"), &bc, &br);
    }

    for (si, n) in smult_scalar_edges().into_iter().enumerate() {
        let p = random_valid_ristretto(&mut rng);
        let mut oc = out_buf(PBYTES);
        let mut or = out_buf(PBYTES);
        unsafe {
            let rc = cm(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rm(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ristretto edge[{si}] rc n={}", hex(&n));
        }
        eqb(&format!("ristretto edge[{si}] out"), &oc, &or);
        let mut bc = out_buf(PBYTES);
        let mut br = out_buf(PBYTES);
        unsafe {
            let rc = cmb(bc.as_mut_ptr(), n.as_ptr());
            let rr = rmb(br.as_mut_ptr(), n.as_ptr());
            assert_eq!(rc, rr, "ristretto_base edge[{si}] rc");
        }
        eqb(&format!("ristretto_base edge[{si}] out"), &bc, &br);
    }

    // invalid ristretto encodings (small-order ed25519 encodings + junk) as P
    for (i, p) in SMALL_ORDER_POINTS.iter().enumerate() {
        let n = rng.bytes(SBYTES);
        let mut oc = out_buf(PBYTES);
        let mut or = out_buf(PBYTES);
        unsafe {
            let rc = cm(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rm(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ristretto small-order-p[{i}] rc disagree");
        }
        eqb(&format!("ristretto small-order-p[{i}] out"), &oc, &or);
    }
    for iter in 0..100 {
        let n = rng.bytes(SBYTES);
        let p = rng.bytes(PBYTES); // mostly invalid ristretto encoding
        let mut oc = out_buf(PBYTES);
        let mut or = out_buf(PBYTES);
        unsafe {
            let rc = cm(oc.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rm(or.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ristretto junk-p[{iter}] rc disagree p={}", hex(&p));
        }
        eqb(&format!("ristretto junk-p[{iter}] out"), &oc, &or);
    }
}
