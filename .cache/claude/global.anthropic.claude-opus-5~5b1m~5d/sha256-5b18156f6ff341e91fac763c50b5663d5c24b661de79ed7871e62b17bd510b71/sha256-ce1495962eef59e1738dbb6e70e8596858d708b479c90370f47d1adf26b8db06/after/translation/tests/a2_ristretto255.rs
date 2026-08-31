//! Area 2, part 4 — `crypto_core/ed25519/core_ristretto255.c` and the
//! `ristretto255_*` internals of `ref10/ed25519_ref10.c`.
//!
//! Covers `configs_2.md` rows 2.104 - 2.127 (ristretto half) and `errors_2.md`
//! rows 2.12 - 2.17 and 2.44 - 2.52.
mod common;
use common::*;
use std::ffi::c_int;

// ------------------------------------------------------------------ constants

/// `L = 2^252 + 27742317777372353535851937790883648493`, little-endian.
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

fn l_minus_one() -> [u8; 32] {
    let mut v = L;
    v[0] = 0xec;
    v
}
fn zero32() -> [u8; 32] {
    [0u8; 32]
}
fn one32() -> [u8; 32] {
    let mut v = [0u8; 32];
    v[0] = 1;
    v
}

/// The ristretto255 identity is the all-zero encoding.
const RIST_IDENTITY: [u8; 32] = [0u8; 32];

/// The ristretto255 base point encoding (RFC 9496).
const RIST_BASEPOINT: [u8; 32] = [
    0xe2, 0xf2, 0xae, 0x0a, 0x6a, 0xbc, 0x4e, 0x71, 0xa8, 0x84, 0xa9, 0x61, 0xc5, 0x00, 0x51, 0x5f,
    0x58, 0xe3, 0x0b, 0x6a, 0xa5, 0x82, 0xdd, 0x8d, 0xb6, 0xa6, 0x59, 0x45, 0xe0, 0x8d, 0x2d, 0x76,
];

/// `p - 1 = 2^255 - 20`: canonical, even, but makes `1 - s^2 == 0`, so
/// `ristretto255_frombytes` rejects it through the `iszero(h->Y)` arm
/// (`errors_2.md` row 2.16).
fn p_minus_one() -> [u8; 32] {
    let mut v = [0xffu8; 32];
    v[0] = 0xec;
    v[31] = 0x7f;
    v
}

// --------------------------------------------------------------- FFI typedefs

type Getter = unsafe extern "C" fn() -> usize;
type Fn1 = unsafe extern "C" fn(*mut u8);
type Fn2 = unsafe extern "C" fn(*mut u8, *const u8);
type Fn2i = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Fn3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type Fn3i = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type Pred = unsafe extern "C" fn(*const u8) -> c_int;
type FromString =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int;

#[repr(C)]
#[derive(Copy, Clone)]
struct P3([i32; 40]);
impl P3 {
    fn new() -> P3 {
        P3([0x5A5A5A5A; 40])
    }
    fn bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}
type RistFromBytes = unsafe extern "C" fn(*mut P3, *const u8) -> c_int;
type RistToBytes = unsafe extern "C" fn(*mut u8, *const P3);
type GePred = unsafe extern "C" fn(*const P3) -> c_int;

// ------------------------------------------------------------------- helpers

/// Every 32-byte encoding shape that matters for `ristretto255_frombytes`.
fn rist_encodings(seed: u64) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = vec![RIST_IDENTITY, RIST_BASEPOINT, p_minus_one()];
    // s[0] odd  -> rejected (errors_2 row 2.12, `s[0] & 1`)
    {
        let mut a = RIST_BASEPOINT;
        a[0] |= 1;
        v.push(a);
        v.push(one32());
    }
    // bit 255 set -> rejected (`e` term)
    {
        let mut a = RIST_BASEPOINT;
        a[31] |= 0x80;
        v.push(a);
        let mut b = RIST_IDENTITY;
        b[31] = 0x80;
        v.push(b);
    }
    // s >= 2^255-19 -> rejected (`c & d`)
    for first in [0xecu8, 0xed, 0xee, 0xf0, 0xfe] {
        let mut a = [0xffu8; 32];
        a[0] = first;
        a[31] = 0x7f;
        v.push(a);
        let mut b = a;
        b[31] = 0xff;
        v.push(b);
    }
    v.push([0xffu8; 32]);
    // small even values
    for x in 0u8..64 {
        let mut a = [0u8; 32];
        a[0] = x * 2;
        v.push(a);
    }
    let mut rng = Rng::new(seed);
    for _ in 0..400 {
        v.push(rng.bytes(32).try_into().unwrap());
    }
    // random strings forced through the canonicity gate, so field work runs
    for _ in 0..1200 {
        let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
        a[0] &= 0xfe;
        a[31] &= 0x7f;
        v.push(a);
    }
    v
}

/// A pool of genuinely valid ristretto255 encodings, produced by `from_hash`.
fn valid_points(n: usize, seed: u64) -> Vec<[u8; 32]> {
    let (fh, _) = both::<Fn2i>("crypto_core_ristretto255_from_hash");
    let mut rng = Rng::new(seed);
    let mut v: Vec<[u8; 32]> = vec![RIST_IDENTITY, RIST_BASEPOINT];
    for _ in 0..n {
        let h = rng.bytes(64);
        let mut o = [0u8; 32];
        assert_eq!(unsafe { fh(o.as_mut_ptr(), h.as_ptr()) }, 0);
        v.push(o);
    }
    v
}

fn scalar_pool(seed: u64) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = vec![zero32(), one32(), L, l_minus_one(), [0xffu8; 32]];
    let mut rng = Rng::new(seed);
    for _ in 0..150 {
        let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
        a[31] &= 0x0f;
        v.push(a);
    }
    for _ in 0..150 {
        v.push(rng.bytes(32).try_into().unwrap());
    }
    v
}

// -------------------------------------------------------------------- getters

/// Row 2.127 (ristretto half).
#[test]
fn ristretto255_getters() {
    for (name, want) in [
        ("crypto_core_ristretto255_bytes", 32usize),
        ("crypto_core_ristretto255_hashbytes", 64),
        ("crypto_core_ristretto255_scalarbytes", 32),
        ("crypto_core_ristretto255_nonreducedscalarbytes", 64),
    ] {
        assert!(has(name), "{name} must be exported by both libraries");
        let (c, r) = both::<Getter>(name);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
        assert_eq!(vc, want, "{name}: C returned {vc}, header says {want}");
    }
}

// -------------------------------------------------------- frombytes / is_valid

/// Rows 2.104 - 2.106 and error rows 2.12 - 2.17 / 2.44.
#[test]
fn ristretto255_is_valid_point() {
    let (c, r) = both::<Pred>("crypto_core_ristretto255_is_valid_point");
    assert!(has("_sodium_ristretto255_frombytes"));
    let (cfb, rfb) = both::<RistFromBytes>("_sodium_ristretto255_frombytes");
    let (ctb, rtb) = both::<RistToBytes>("_sodium_ristretto255_p3_tobytes");
    let (onc, _) = both::<GePred>("_sodium_ge25519_is_on_curve");

    // 2.104 / 2.105: the identity and the base point are valid
    assert_eq!(unsafe { c(RIST_IDENTITY.as_ptr()) }, 1, "the all-zero identity must be valid");
    assert_eq!(unsafe { r(RIST_IDENTITY.as_ptr()) }, 1);
    assert_eq!(unsafe { c(RIST_BASEPOINT.as_ptr()) }, 1, "the base point must be valid");
    assert_eq!(unsafe { r(RIST_BASEPOINT.as_ptr()) }, 1);

    // error row 2.12: the three independent canonicity rejections
    for (label, s) in [
        ("s[0] odd", {
            let mut a = RIST_BASEPOINT;
            a[0] |= 1;
            a
        }),
        ("bit 255 set", {
            let mut a = RIST_BASEPOINT;
            a[31] |= 0x80;
            a
        }),
        ("s >= p", {
            let mut a = [0xffu8; 32];
            a[0] = 0xee;
            a[31] = 0x7f;
            a
        }),
        ("all-0xff", [0xffu8; 32]),
    ] {
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eqi(&format!("is_valid_point [{label}]"), a, b);
        assert_eq!(a, 0, "is_valid_point [{label}] must reject");
    }
    // error row 2.16: p - 1 is canonical but decodes to Y == 0
    {
        let s = p_minus_one();
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eqi("is_valid_point [p-1]", a, b);
        assert_eq!(a, 0, "p-1 must be rejected (Y == 0)");
    }

    // the full sweep, cross-checked against the private decoder
    let mut n_ok = 0usize;
    let mut n_bad = 0usize;
    for s in rist_encodings(0x2_0104) {
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eqi(&format!("ristretto255_is_valid_point({})", hex(&s)), a, b);
        let mut pc = P3::new();
        let mut pr = P3::new();
        let (rc, rr2) = unsafe { (cfb(&mut pc, s.as_ptr()), rfb(&mut pr, s.as_ptr())) };
        eqi(&format!("ristretto255_frombytes({})", hex(&s)), rc, rr2);
        assert_eq!(a, if rc == 0 { 1 } else { 0 }, "wrapper disagrees with frombytes");
        assert!(rc == 0 || rc == -1, "ristretto255_frombytes must return 0 or -1");
        if rc == 0 {
            n_ok += 1;
            eqb("ristretto255_frombytes p3", &pc.bytes(), &pr.bytes());
            assert_eq!(unsafe { onc(&pc) }, 1, "decoded ristretto point is not on the curve");
            // 2.114: re-encoding must reproduce the input exactly
            let mut ec = padded(32);
            let mut er = padded(32);
            unsafe {
                ctb(ec.as_mut_ptr(), &pc);
                rtb(er.as_mut_ptr(), &pr);
            }
            eqb("ristretto255_p3_tobytes", &ec[..32], &er[..32]);
            check_pad("ristretto255_p3_tobytes(C)", &ec, 32);
            check_pad("ristretto255_p3_tobytes(Rust)", &er, 32);
            assert_eq!(&ec[..32], &s[..], "ristretto255 encode/decode is not a bijection");
        } else {
            n_bad += 1;
        }
    }
    assert!(n_ok > 50 && n_bad > 50, "coverage: {n_ok} accepted, {n_bad} rejected");

    // 2.106: `_random` and `_from_hash` output is always valid
    let (crand, rrand) = both::<Fn1>("crypto_core_ristretto255_random");
    for i in 0..200u64 {
        let seed = 0x0bad_c0de_0000_0001u64 ^ (i.wrapping_mul(0x9E37_79B9) | 1);
        rng_reseed(seed);
        let mut oc = padded(32);
        unsafe { crand(oc.as_mut_ptr()) };
        rng_reseed(seed);
        let mut or = padded(32);
        unsafe { rrand(or.as_mut_ptr()) };
        eqb("crypto_core_ristretto255_random", &oc[..32], &or[..32]);
        check_pad("ristretto255_random(C)", &oc, 32);
        check_pad("ristretto255_random(Rust)", &or, 32);
        assert_eq!(unsafe { c(oc.as_ptr()) }, 1, "random() produced an invalid point");
    }
    rng_reset();
    for p in valid_points(200, 0x2_0106) {
        assert_eq!(unsafe { c(p.as_ptr()) }, 1);
        assert_eq!(unsafe { r(p.as_ptr()) }, 1);
    }
}

// ----------------------------------------------------------------- from_hash

/// Rows 2.111 - 2.113 and 2.115.
#[test]
fn ristretto255_from_hash() {
    let (c, r) = both::<Fn2i>("crypto_core_ristretto255_from_hash");
    let (cp, rp) = both::<Fn2>("_sodium_ristretto255_from_hash");
    let (valid, _) = both::<Pred>("crypto_core_ristretto255_is_valid_point");

    let mut inputs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64]];
    // bit 255 of each half is ignored by fe25519_frombytes
    for a in [0x00u8, 0x80] {
        for b in [0x00u8, 0x80] {
            let mut v = [0x5cu8; 64];
            v[31] = a;
            v[63] = b;
            inputs.push(v);
        }
    }
    for i in 0..64 * 8 {
        let mut v = [0u8; 64];
        v[i / 8] = 1u8 << (i % 8);
        inputs.push(v);
    }
    let mut rng = Rng::new(0x2_0111);
    for _ in 0..1500 {
        inputs.push(rng.bytes(64).try_into().unwrap());
    }

    let mut seen: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    for h in &inputs {
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), h.as_ptr()) };
        let ec = errno();
        set_errno(0);
        let rr2 = unsafe { r(or.as_mut_ptr(), h.as_ptr()) };
        let er = errno();
        eqi("ristretto255_from_hash ret", rc, rr2);
        assert_eq!(rc, 0, "crypto_core_ristretto255_from_hash always returns 0");
        assert_eq!(ec, er);
        eqb(&format!("ristretto255_from_hash({})", hex(&h[..8])), &oc[..32], &or[..32]);
        check_pad("from_hash(C)", &oc, 32);
        check_pad("from_hash(Rust)", &or, 32);
        assert_eq!(unsafe { valid(oc.as_ptr()) }, 1, "from_hash output must be valid");
        seen.insert(oc[..32].to_vec());

        // the public wrapper is `ristretto255_from_hash` verbatim
        let mut pc = padded(32);
        let mut pr = padded(32);
        unsafe {
            cp(pc.as_mut_ptr(), h.as_ptr());
            rp(pr.as_mut_ptr(), h.as_ptr());
        }
        eqb("_sodium_ristretto255_from_hash", &pc[..32], &pr[..32]);
        assert_eq!(&pc[..32], &oc[..32]);

        // flipping the ignored high bit of either half must not change anything
        let mut h2 = *h;
        h2[31] ^= 0x80;
        h2[63] ^= 0x80;
        let mut qc = [0u8; 32];
        unsafe { c(qc.as_mut_ptr(), h2.as_ptr()) };
        assert_eq!(&qc[..], &oc[..32], "bit 255 of each half must be ignored");
    }
    // 2.112: the all-zero input maps to the identity (t = 0 in both halves)
    {
        let mut o = [0u8; 32];
        assert_eq!(unsafe { c(o.as_mut_ptr(), [0u8; 64].as_ptr()) }, 0);
        assert_eq!(unsafe { valid(o.as_ptr()) }, 1);
    }
    assert!(seen.len() > 500, "from_hash outputs are suspiciously repetitive");
}

// ------------------------------------------------------------------ add / sub

/// Rows 2.107 - 2.110 and error rows 2.45 - 2.48.
#[test]
fn ristretto255_add_and_sub() {
    let (ca, ra) = both::<Fn3i>("crypto_core_ristretto255_add");
    let (cs, rs) = both::<Fn3i>("crypto_core_ristretto255_sub");
    let (valid, _) = both::<Pred>("crypto_core_ristretto255_is_valid_point");

    let run = |c: &Fn3i, r: &Fn3i, p: &[u8; 32], q: &[u8; 32], label: &str| -> (c_int, Vec<u8>) {
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let ec = errno();
        set_errno(0);
        let rr2 = unsafe { r(or.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let er = errno();
        eqi(&format!("{label} ret ({}, {})", hex(p), hex(q)), rc, rr2);
        assert_eq!(ec, er, "{label} errno");
        if rc == 0 {
            eqb(&format!("{label} out"), &oc[..32], &or[..32]);
        }
        check_pad(&format!("{label}(C)"), &oc, 32);
        check_pad(&format!("{label}(Rust)"), &or, 32);
        (rc, oc[..32].to_vec())
    };

    let good = valid_points(40, 0x2_0107);
    let mut bad: Vec<[u8; 32]> = vec![
        {
            let mut a = RIST_BASEPOINT;
            a[0] |= 1;
            a
        },
        {
            let mut a = RIST_BASEPOINT;
            a[31] |= 0x80;
            a
        },
        [0xffu8; 32],
        p_minus_one(),
    ];
    let mut rng = Rng::new(0x2_0108);
    while bad.len() < 12 {
        let s: [u8; 32] = rng.bytes(32).try_into().unwrap();
        if unsafe { valid(s.as_ptr()) } == 0 {
            bad.push(s);
        }
    }

    // 2.108 / 2.110: identity is neutral
    for p in &good {
        let (rc, o) = run(&ca, &ra, p, &RIST_IDENTITY, "rist_add(P, 0)");
        assert_eq!(rc, 0);
        assert_eq!(&o[..], &p[..], "P + identity != P");
        let (rc, o) = run(&cs, &rs, p, &RIST_IDENTITY, "rist_sub(P, 0)");
        assert_eq!(rc, 0);
        assert_eq!(&o[..], &p[..], "P - identity != P");
        // 2.109: P - P == identity
        let (rc, o) = run(&cs, &rs, p, p, "rist_sub(P, P)");
        assert_eq!(rc, 0);
        assert_eq!(&o[..], &RIST_IDENTITY[..], "P - P != identity");
    }

    // 2.107 / 2.109 plus every rejection combination (errors 2.45 - 2.48)
    let mut pool: Vec<[u8; 32]> = good.clone();
    pool.extend(bad.iter().cloned());
    let mut n_ok = 0usize;
    let mut n_fail = 0usize;
    for p in &pool {
        for q in &pool {
            let pv = unsafe { valid(p.as_ptr()) } == 1;
            let qv = unsafe { valid(q.as_ptr()) } == 1;
            let want = if pv && qv { 0 } else { -1 };
            let (rca, oa) = run(&ca, &ra, p, q, "rist_add");
            let (rcs, os) = run(&cs, &rs, p, q, "rist_sub");
            assert_eq!(rca, want, "rist_add({}, {}) sentinel", hex(p), hex(q));
            assert_eq!(rcs, want, "rist_sub({}, {}) sentinel", hex(p), hex(q));
            if want == 0 {
                n_ok += 1;
                assert_eq!(unsafe { valid(oa.as_ptr()) }, 1);
                assert_eq!(unsafe { valid(os.as_ptr()) }, 1);
                // commutativity of add, and (p+q)-q == p
                let (_, ob) = run(&ca, &ra, q, p, "rist_add(swapped)");
                assert_eq!(oa, ob, "ristretto255_add is not commutative");
                let arr: [u8; 32] = oa.clone().try_into().unwrap();
                let (rc, back) = run(&cs, &rs, &arr, q, "rist_sub(roundtrip)");
                assert_eq!(rc, 0);
                assert_eq!(&back[..], &p[..], "(p+q)-q != p");
            } else {
                n_fail += 1;
            }
        }
    }
    assert!(n_ok > 100 && n_fail > 100, "coverage: {n_ok} / {n_fail}");
}

// ---------------------------------------------------------------- from_string

fn h2c_strings(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"QUUX-V01-CS02-with-ristretto255_XMD:SHA-512_R255MAP_RO_".to_vec(),
        vec![0u8; 32],
        vec![0xffu8; 63],
        vec![0x41u8; 64],
        vec![0x42u8; 65],
        vec![0x43u8; 128],
        vec![0x44u8; 129],
        vec![0x47u8; 255],
        vec![0x48u8; 256],
        vec![0x49u8; 300],
        vec![0x4au8; 1000],
    ];
    for n in [1usize, 3, 31, 33, 48, 96] {
        v.push(rng.bytes(n));
    }
    v
}

/// Rows 2.116 - 2.118 and error rows 2.49 / 2.50.
#[test]
fn ristretto255_from_string() {
    let (c, r) = both::<FromString>("crypto_core_ristretto255_from_string");
    let (valid, _) = both::<Pred>("crypto_core_ristretto255_is_valid_point");
    let mut rng = Rng::new(0x2_0116);
    let strings = h2c_strings(&mut rng);

    let call = |ctx: Option<&[u8]>, msg: &[u8], alg: c_int| -> (c_int, Vec<u8>) {
        let (cp, cl) = match ctx {
            None => (std::ptr::null(), 0usize),
            Some(s) => (s.as_ptr(), s.len()),
        };
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), cp, cl, msg.as_ptr(), msg.len(), alg) };
        let ec = errno();
        set_errno(0);
        let rr2 = unsafe { r(or.as_mut_ptr(), cp, cl, msg.as_ptr(), msg.len(), alg) };
        let er = errno();
        eqi(&format!("ristretto255_from_string ret (alg {alg}, ctx_len {cl})"), rc, rr2);
        assert_eq!(ec, er, "ristretto255_from_string errno (alg {alg})");
        if rc == 0 {
            eqb("ristretto255_from_string out", &oc[..32], &or[..32]);
        }
        check_pad("ristretto255_from_string(C)", &oc, 32);
        check_pad("ristretto255_from_string(Rust)", &or, 32);
        (rc, oc[..32].to_vec())
    };

    for alg in [1i32, 2] {
        for ctx in &strings {
            for msg in &strings {
                let (rc, out) = call(Some(ctx), msg, alg);
                assert_eq!(rc, 0);
                assert_eq!(unsafe { valid(out.as_ptr()) }, 1);
            }
        }
        for msg in &strings {
            let (rc, a) = call(None, msg, alg);
            assert_eq!(rc, 0);
            let (_, b) = call(Some(&[]), msg, alg);
            assert_eq!(a, b, "ctx=NULL must behave like ctx_len=0");
        }
    }
    for msg in &strings {
        let (_, a) = call(Some(b"ctx"), msg, 1);
        let (_, b) = call(Some(b"ctx"), msg, 2);
        assert_ne!(a, b, "H2CSHA256 and H2CSHA512 must differ");
    }

    // error rows 2.49 / 2.50
    for alg in [0i32, 3, -1, 255, 256, i32::MIN, i32::MAX] {
        let (rc, _) = call(Some(b"ctx"), b"msg", alg);
        assert_eq!(rc, -1, "from_string(alg={alg}) must fail");
        set_errno(0);
        let mut o = [0u8; 32];
        assert_eq!(
            unsafe { c(o.as_mut_ptr(), b"ctx".as_ptr(), 3, b"msg".as_ptr(), 3, alg) },
            -1
        );
        assert_eq!(errno(), EINVAL, "from_string(alg={alg}) must set EINVAL");
    }
}

// ------------------------------------------------------------ scalar wrappers

/// Rows 2.119 - 2.126 and error rows 2.51 / 2.52 — every ristretto255 scalar
/// entry point, cross-checked against its ed25519 counterpart.
#[test]
fn ristretto255_scalar_wrappers() {
    let pool = scalar_pool(0x2_0120);

    // ---- unary void wrappers: negate, complement
    for (rist, ed) in [
        ("crypto_core_ristretto255_scalar_negate", "crypto_core_ed25519_scalar_negate"),
        (
            "crypto_core_ristretto255_scalar_complement",
            "crypto_core_ed25519_scalar_complement",
        ),
    ] {
        let (c, r) = both::<Fn2>(rist);
        let (ce, _) = both::<Fn2>(ed);
        for s in &pool {
            let mut oc = padded(32);
            let mut or = padded(32);
            let mut oe = [0u8; 32];
            unsafe {
                c(oc.as_mut_ptr(), s.as_ptr());
                r(or.as_mut_ptr(), s.as_ptr());
                ce(oe.as_mut_ptr(), s.as_ptr());
            }
            eqb(&format!("{rist}({})", hex(s)), &oc[..32], &or[..32]);
            check_pad(&format!("{rist}(C)"), &oc, 32);
            check_pad(&format!("{rist}(Rust)"), &or, 32);
            assert_eq!(&oc[..32], &oe[..], "{rist} must delegate to {ed}");
        }
    }

    // ---- scalar_reduce (row 2.124), 64-byte input
    {
        let (c, r) = both::<Fn2>("crypto_core_ristretto255_scalar_reduce");
        let (ce, _) = both::<Fn2>("crypto_core_ed25519_scalar_reduce");
        let mut inputs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64]];
        {
            let mut a = [0u8; 64];
            a[..32].copy_from_slice(&L);
            inputs.push(a);
            let mut b = [0u8; 64];
            b[..32].copy_from_slice(&l_minus_one());
            inputs.push(b);
        }
        let mut rng = Rng::new(0x2_0124);
        for _ in 0..400 {
            inputs.push(rng.bytes(64).try_into().unwrap());
        }
        for t in &inputs {
            let mut oc = padded(32);
            let mut or = padded(32);
            let mut oe = [0u8; 32];
            unsafe {
                c(oc.as_mut_ptr(), t.as_ptr());
                r(or.as_mut_ptr(), t.as_ptr());
                ce(oe.as_mut_ptr(), t.as_ptr());
            }
            eqb("ristretto255_scalar_reduce", &oc[..32], &or[..32]);
            check_pad("scalar_reduce(C)", &oc, 32);
            check_pad("scalar_reduce(Rust)", &or, 32);
            assert_eq!(&oc[..32], &oe[..], "must delegate to the ed25519 version");
        }
        // == L -> 0
        let mut t = [0u8; 64];
        t[..32].copy_from_slice(&L);
        let mut o = [0u8; 32];
        unsafe { c(o.as_mut_ptr(), t.as_ptr()) };
        assert_eq!(o, [0u8; 32]);
    }

    // ---- binary void wrappers: add, sub, mul (rows 2.122 / 2.123)
    for (rist, ed) in [
        ("crypto_core_ristretto255_scalar_add", "crypto_core_ed25519_scalar_add"),
        ("crypto_core_ristretto255_scalar_sub", "crypto_core_ed25519_scalar_sub"),
        ("crypto_core_ristretto255_scalar_mul", "crypto_core_ed25519_scalar_mul"),
    ] {
        let (c, r) = both::<Fn3>(rist);
        let (ce, _) = both::<Fn3>(ed);
        let mut rng = Rng::new(0x2_0122);
        let mut pairs: Vec<([u8; 32], [u8; 32])> = vec![
            (zero32(), zero32()),
            (one32(), zero32()),
            (zero32(), one32()),
            (l_minus_one(), l_minus_one()),
            ([0xffu8; 32], [0xffu8; 32]),
            (L, L),
        ];
        for _ in 0..2000 {
            pairs.push((pool[rng.below(pool.len())], pool[rng.below(pool.len())]));
        }
        for (x, y) in &pairs {
            let mut oc = padded(32);
            let mut or = padded(32);
            let mut oe = [0u8; 32];
            unsafe {
                c(oc.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                r(or.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                ce(oe.as_mut_ptr(), x.as_ptr(), y.as_ptr());
            }
            eqb(&format!("{rist}({}, {})", hex(x), hex(y)), &oc[..32], &or[..32]);
            check_pad(&format!("{rist}(C)"), &oc, 32);
            check_pad(&format!("{rist}(Rust)"), &or, 32);
            assert_eq!(&oc[..32], &oe[..], "{rist} must agree with {ed}");
        }
    }

    // ---- scalar_invert (row 2.120, error row 2.51)
    {
        let (c, r) = both::<Fn2i>("crypto_core_ristretto255_scalar_invert");
        let (ce, _) = both::<Fn2i>("crypto_core_ed25519_scalar_invert");
        let (mul, _) = both::<Fn3>("crypto_core_ristretto255_scalar_mul");
        let (canon, _) = both::<Pred>("crypto_core_ristretto255_scalar_is_canonical");
        for s in &pool {
            let mut oc = padded(32);
            let mut or = padded(32);
            let mut oe = [0u8; 32];
            set_errno(0);
            let rc = unsafe { c(oc.as_mut_ptr(), s.as_ptr()) };
            let ec = errno();
            set_errno(0);
            let rr2 = unsafe { r(or.as_mut_ptr(), s.as_ptr()) };
            let er = errno();
            let re = unsafe { ce(oe.as_mut_ptr(), s.as_ptr()) };
            eqi(&format!("ristretto255_scalar_invert({})", hex(s)), rc, rr2);
            assert_eq!(ec, er);
            assert_eq!(rc, re, "must delegate to the ed25519 version");
            let want = if *s == [0u8; 32] { -1 } else { 0 };
            assert_eq!(rc, want, "scalar_invert({}) sentinel", hex(s));
            eqb("ristretto255_scalar_invert out", &oc[..32], &or[..32]);
            check_pad("scalar_invert(C)", &oc, 32);
            check_pad("scalar_invert(Rust)", &or, 32);
            assert_eq!(&oc[..32], &oe[..]);
            if rc == 0 && unsafe { canon(s.as_ptr()) } == 1 {
                let mut prod = [0u8; 32];
                unsafe { mul(prod.as_mut_ptr(), s.as_ptr(), oc.as_ptr()) };
                assert_eq!(prod, one32(), "s * invert(s) != 1");
            }
        }
        // error row 2.51 explicitly
        let mut o = padded(32);
        assert_eq!(unsafe { c(o.as_mut_ptr(), zero32().as_ptr()) }, -1);
        assert_eq!(&o[..32], &[0u8; 32][..]);
    }

    // ---- scalar_is_canonical (row 2.125, error row 2.52)
    {
        let (c, r) = both::<Pred>("crypto_core_ristretto255_scalar_is_canonical");
        let (cp, _) = both::<Pred>("_sodium_sc25519_is_canonical");
        for (label, s, want) in [
            ("0", zero32(), 1),
            ("1", one32(), 1),
            ("L-1", l_minus_one(), 1),
            ("L", L, 0),
            ("all-0xff", [0xffu8; 32], 0),
        ] {
            let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
            eqi(&format!("ristretto255_scalar_is_canonical({label})"), a, b);
            assert_eq!(a, want, "is_canonical({label}): C gave {a}, expected {want}");
        }
        for s in &pool {
            let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
            eqi("ristretto255_scalar_is_canonical", a, b);
            assert_eq!(a, unsafe { cp(s.as_ptr()) }, "must call sc25519_is_canonical directly");
        }
    }

    // ---- scalar_random (row 2.119)
    {
        let (c, r) = both::<Fn1>("crypto_core_ristretto255_scalar_random");
        let (canon, _) = both::<Pred>("crypto_core_ristretto255_scalar_is_canonical");
        for i in 0..200u64 {
            let seed = 0x5eed_0000_0000_0001u64 ^ (i.wrapping_mul(0x9E37_79B9) | 1);
            rng_reseed(seed);
            let mut oc = padded(32);
            unsafe { c(oc.as_mut_ptr()) };
            rng_reseed(seed);
            let mut or = padded(32);
            unsafe { r(or.as_mut_ptr()) };
            eqb("ristretto255_scalar_random", &oc[..32], &or[..32]);
            check_pad("scalar_random(C)", &oc, 32);
            check_pad("scalar_random(Rust)", &or, 32);
            assert!(oc[31] <= 0x1f, "r[31] must be <= 0x1f");
            assert_eq!(unsafe { canon(oc.as_ptr()) }, 1);
            assert_ne!(&oc[..32], &[0u8; 32][..]);
        }
        rng_reset();
    }

    // ---- scalar_from_string (row 2.126)
    {
        let (c, r) = both::<FromString>("crypto_core_ristretto255_scalar_from_string");
        let (ce, _) = both::<FromString>("crypto_core_ed25519_scalar_from_string");
        let (canon, _) = both::<Pred>("crypto_core_ristretto255_scalar_is_canonical");
        let mut rng = Rng::new(0x2_0126);
        let strings = h2c_strings(&mut rng);
        for alg in [1i32, 2] {
            for ctx in &strings {
                for msg in &strings {
                    let mut oc = padded(32);
                    let mut or = padded(32);
                    let mut oe = [0u8; 32];
                    set_errno(0);
                    let rc = unsafe {
                        c(
                            oc.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctx.len(),
                            msg.as_ptr(),
                            msg.len(),
                            alg,
                        )
                    };
                    let ec = errno();
                    set_errno(0);
                    let rr2 = unsafe {
                        r(
                            or.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctx.len(),
                            msg.as_ptr(),
                            msg.len(),
                            alg,
                        )
                    };
                    let er = errno();
                    unsafe {
                        ce(oe.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
                    };
                    eqi("ristretto255_scalar_from_string ret", rc, rr2);
                    assert_eq!(ec, er);
                    assert_eq!(rc, 0);
                    eqb("ristretto255_scalar_from_string out", &oc[..32], &or[..32]);
                    check_pad("scalar_from_string(C)", &oc, 32);
                    check_pad("scalar_from_string(Rust)", &or, 32);
                    assert_eq!(&oc[..32], &oe[..], "must delegate to the ed25519 version");
                    assert_eq!(unsafe { canon(oc.as_ptr()) }, 1);
                }
            }
            // ctx == NULL
            let mut o = [0u8; 32];
            assert_eq!(
                unsafe { c(o.as_mut_ptr(), std::ptr::null(), 0, b"m".as_ptr(), 1, alg) },
                0
            );
        }
        // bad hash_alg
        for alg in [0i32, 3, -1, i32::MIN, i32::MAX] {
            let mut oc = [0u8; 32];
            let mut or = [0u8; 32];
            set_errno(0);
            let rc = unsafe { c(oc.as_mut_ptr(), b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg) };
            let ec = errno();
            set_errno(0);
            let rr2 = unsafe { r(or.as_mut_ptr(), b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg) };
            let er = errno();
            eqi(&format!("ristretto255_scalar_from_string(alg={alg})"), rc, rr2);
            assert_eq!(rc, -1);
            assert_eq!(ec, EINVAL);
            assert_eq!(er, EINVAL);
        }
    }
}
