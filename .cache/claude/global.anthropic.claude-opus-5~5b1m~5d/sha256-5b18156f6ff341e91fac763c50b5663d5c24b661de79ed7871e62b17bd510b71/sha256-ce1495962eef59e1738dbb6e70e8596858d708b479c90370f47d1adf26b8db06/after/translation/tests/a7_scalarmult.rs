//! Area 7 — `crypto_scalarmult`: curve25519 (`_base` / point), ed25519
//! (clamped / noclamp, `_base` / point) and ristretto255 (`_base` / point).
//!
//! Covers `configs_7.md` rows 7.1–7.28 and `errors_7.md` rows 7.1–7.33,
//! 7.127 (accessors) and 7.128 (dispatch).
mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, CStr};

type Mult = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type MultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type IntFn = unsafe extern "C" fn() -> c_int;

// ------------------------------------------------------------------ helpers

fn hx(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}
fn h32(s: &str) -> [u8; 32] {
    let v = hx(s);
    let mut a = [0u8; 32];
    a.copy_from_slice(&v);
    a
}
/// Distinctive prefill so that "q untouched" is observable.
fn pat() -> Vec<u8> {
    (0..32u8).map(|i| 0x5Au8.wrapping_add(i.wrapping_mul(7))).collect()
}

/// A (C, Rust) pair of 3-argument scalarmult functions.
struct P3 {
    name: String,
    c: Symbol<'static, Mult>,
    r: Symbol<'static, Mult>,
}
impl P3 {
    fn new(name: &str) -> Self {
        let (c, r) = both::<Mult>(name);
        P3 { name: name.to_string(), c, r }
    }
    /// Differentially run with a padded, pre-filled output buffer.
    /// Returns `(rc, q[0..32])`.
    fn run(&self, n: &[u8], p: &[u8]) -> (c_int, Vec<u8>) {
        assert_eq!(n.len(), 32);
        assert_eq!(p.len(), 32);
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(&pat());
        qr[..32].copy_from_slice(&pat());
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eqi(&format!("{}({}, {}) rc", self.name, hex(n), hex(p)), rc, rr);
        eqb(&format!("{}({}, {}) q", self.name, hex(n), hex(p)), &qc, &qr);
        check_pad(&format!("{}(C)", self.name), &qc, 32);
        check_pad(&format!("{}(Rust)", self.name), &qr, 32);
        (rc, qc[..32].to_vec())
    }
    /// Fully aliased call: `q == n` (and `p` separate).
    fn run_alias_n(&self, n: &[u8], p: &[u8]) -> (c_int, Vec<u8>) {
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(n);
        qr[..32].copy_from_slice(n);
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), qc.as_ptr(), p.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), qr.as_ptr(), p.as_ptr()) };
        eqi(&format!("{} alias q==n rc", self.name), rc, rr);
        eqb(&format!("{} alias q==n q", self.name), &qc, &qr);
        check_pad(&format!("{} alias q==n(C)", self.name), &qc, 32);
        (rc, qc[..32].to_vec())
    }
    /// Fully aliased call: `q == p`.
    fn run_alias_p(&self, n: &[u8], p: &[u8]) -> (c_int, Vec<u8>) {
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(p);
        qr[..32].copy_from_slice(p);
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), n.as_ptr(), qc.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), n.as_ptr(), qr.as_ptr()) };
        eqi(&format!("{} alias q==p rc", self.name), rc, rr);
        eqb(&format!("{} alias q==p q", self.name), &qc, &qr);
        check_pad(&format!("{} alias q==p(C)", self.name), &qc, 32);
        (rc, qc[..32].to_vec())
    }
}

/// A (C, Rust) pair of 2-argument `_base` functions.
struct P2 {
    name: String,
    c: Symbol<'static, MultBase>,
    r: Symbol<'static, MultBase>,
}
impl P2 {
    fn new(name: &str) -> Self {
        let (c, r) = both::<MultBase>(name);
        P2 { name: name.to_string(), c, r }
    }
    fn run(&self, n: &[u8]) -> (c_int, Vec<u8>) {
        assert_eq!(n.len(), 32);
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(&pat());
        qr[..32].copy_from_slice(&pat());
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), n.as_ptr()) };
        eqi(&format!("{}({}) rc", self.name, hex(n)), rc, rr);
        eqb(&format!("{}({}) q", self.name, hex(n)), &qc, &qr);
        check_pad(&format!("{}(C)", self.name), &qc, 32);
        check_pad(&format!("{}(Rust)", self.name), &qr, 32);
        (rc, qc[..32].to_vec())
    }
    fn run_alias(&self, n: &[u8]) -> (c_int, Vec<u8>) {
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(n);
        qr[..32].copy_from_slice(n);
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), qc.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), qr.as_ptr()) };
        eqi(&format!("{} alias rc", self.name), rc, rr);
        eqb(&format!("{} alias q", self.name), &qc, &qr);
        check_pad(&format!("{} alias(C)", self.name), &qc, 32);
        (rc, qc[..32].to_vec())
    }
}

// ------------------------------------------------------------------ constants

/// The group order `L = 2^252 + 27742317777372353535851937790883648493`, LE.
const L_HEX: &str = "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
const X25519_BASEPOINT: &str =
    "0900000000000000000000000000000000000000000000000000000000000000";
const ED25519_BASEPOINT: &str =
    "5866666666666666666666666666666666666666666666666666666666666666";

/// The 7 curve25519 blocklist encodings (`x25519_ref10.c:19-51`).
const CURVE_SMALL_ORDER: [&str; 7] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000000",
    "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
    "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

/// The 8 small-order ed25519 point encodings.
const ED_SMALL_ORDER: [&str; 8] = [
    "0100000000000000000000000000000000000000000000000000000000000000",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
];

/// The published ristretto255 multiples of the basepoint, `n = 1 … 15`.
const RISTRETTO_MULTIPLES: [&str; 15] = [
    "e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76",
    "6a493210f7499cd17fecb510ae0cea23a110e8d5b901f8acadd3095c73a3b919",
    "94741f5d5d52755ece4f23f044ee27d5d1ea1e2bd196b462166b16152a9d0259",
    "da80862773358b466ffadfe0b3293ab3d9fd53c5ea6c955358f568322daf6a57",
    "e882b131016b52c1d3337080187cf768423efccbb517bb495ab812c4160ff44e",
    "f64746d3c92b13050ed8d80236a7f0007c3b3f962f5ba793d19a601ebb1df403",
    "44f53520926ec81fbd5a387845beb7df85a96a24ece18738bdcfa6a7822a176d",
    "903293d8f2287ebe10e2374dc1a53e0bc887e592699f02d077d5263cdd55601c",
    "02622ace8f7303a31cafc63f8fc48fdc16e1c8c8d234b2f0d6685282a9076031",
    "20706fd788b2720a1ed2a5dad4952b01f413bcf0e7564de8cdc816689e2db95f",
    "bce83f8ba5dd2fa572864c24ba1810f9522bc6004afe95877ac73241cafdab42",
    "e4549ee16b9aa03099ca208c67adafcafa4c3f3e4e5303de6026e3ca8ff84460",
    "aa52e000df2e16f55fb1032fc33bc42742dad6bd5a8fc0be0167436c5948501f",
    "46376b80f409b29dc2b5f6f0c52591990896e5716f41477cd30085ab7f10301e",
    "e0c418f7c8d9c4cdd7395b93ea124f3ad99021bb681dfc3302a9d99a2e53e64e",
];

fn clamp_curve(n: &mut [u8; 32]) {
    n[0] &= 248;
    n[31] &= 127;
    n[31] |= 64;
}

// ================================================================ 7.28 / 7.127

#[test]
fn scalarmult_accessors() {
    for f in [
        "crypto_scalarmult_bytes",
        "crypto_scalarmult_scalarbytes",
        "crypto_scalarmult_curve25519_bytes",
        "crypto_scalarmult_curve25519_scalarbytes",
        "crypto_scalarmult_ed25519_bytes",
        "crypto_scalarmult_ed25519_scalarbytes",
        "crypto_scalarmult_ristretto255_bytes",
        "crypto_scalarmult_ristretto255_scalarbytes",
    ] {
        let (c, r) = both::<SizeFn>(f);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{f}: C {vc} != Rust {vr}");
        assert_eq!(vc, 32, "{f}: expected 32, got {vc}");
    }
    let (c, r) = both::<StrFn>("crypto_scalarmult_primitive");
    unsafe {
        let sc = CStr::from_ptr(c()).to_str().unwrap();
        let sr = CStr::from_ptr(r()).to_str().unwrap();
        assert_eq!(sc, sr);
        assert_eq!(sc, "curve25519");
    }
}

// ================================================================ curve25519

// 7.1, 7.2, 7.3, 7.4 + errors 7.11 / 7.12 (`_base` can never fail).
#[test]
fn curve25519_base_vectors_and_edge_scalars() {
    let base = P2::new("crypto_scalarmult_curve25519_base");
    let generic = P2::new("crypto_scalarmult_base");

    // 7.1 — RFC 7748 §6.1 Alice and Bob.  (Note: `configs_7.md` row 7.1 pairs
    // the §5.2 *scalar* `a546e36b…` with the §6.1 *public key* `8520f009…`;
    // the reference C shows that the §6.1 scalar is `77076d0a…`, so the vector
    // used here is the corrected one and `a546e36b…` is checked separately.)
    let n = h32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let (rc, q) = base.run(&n);
    eqi("base RFC7748 rc", rc, 0);
    assert_eq!(
        hex(&q),
        "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
    );
    let (rc2, q2) = generic.run(&n);
    eqi("crypto_scalarmult_base rc", rc2, 0);
    eqb("generic base == curve25519 base", &q, &q2);

    let nb = h32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    let (rc, qb) = base.run(&nb);
    eqi("base RFC7748 Bob rc", rc, 0);
    assert_eq!(
        hex(&qb),
        "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"
    );
    // The RFC 7748 §6.1 shared secret.
    let m = P3::new("crypto_scalarmult_curve25519");
    let (rc, s) = m.run(&n, &qb);
    eqi("RFC7748 §6.1 DH rc", rc, 0);
    assert_eq!(
        hex(&s),
        "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"
    );
    let (rc, s2) = m.run(&nb, &q);
    eqi("RFC7748 §6.1 DH rc", rc, 0);
    eqb("RFC7748 §6.1 DH agreement", &s, &s2);

    // The §5.2 scalar through `_base`.
    let n52 = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let (rc, q52) = base.run(&n52);
    eqi("base §5.2 scalar rc", rc, 0);
    assert_eq!(
        hex(&q52),
        "1c9fd88f45606d932a80c71824ae151d15d73e77de38e8e000852e614fae7019"
    );

    // 7.2 / 7.3 — degenerate scalars all still succeed.
    for hexs in [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        L_HEX,
    ] {
        let n = h32(hexs);
        let (rc, q) = base.run(&n);
        eqi(&format!("base({hexs}) rc"), rc, 0);
        let (rc2, q2) = generic.run(&n);
        eqi("generic rc", rc2, 0);
        eqb("generic == curve25519", &q, &q2);
        assert_ne!(q, pat(), "q must have been written");
    }

    // 7.4 — clamping is idempotent.
    let mut rng = Rng::new(0x7401);
    for _ in 0..64 {
        let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (_, q1) = base.run(&a);
        clamp_curve(&mut a);
        let (_, q2) = base.run(&a);
        eqb("clamp idempotent", &q1, &q2);
        // Clamping again changes nothing.
        let mut b = a;
        clamp_curve(&mut b);
        assert_eq!(a, b);
    }
}

// 7.5 — in-place `q == n` on the `_base` path.
#[test]
fn curve25519_base_inplace() {
    let base = P2::new("crypto_scalarmult_curve25519_base");
    let generic = P2::new("crypto_scalarmult_base");
    let mut rng = Rng::new(0x7501);
    for i in 0..48 {
        let n: [u8; 32] = if i == 0 {
            [0u8; 32]
        } else if i == 1 {
            [0xffu8; 32]
        } else {
            rng.bytes(32).try_into().unwrap()
        };
        let (rc_a, q_a) = base.run_alias(&n);
        let (rc_b, q_b) = base.run(&n);
        eqi("inplace vs separate rc", rc_a, rc_b);
        eqb("inplace vs separate q", &q_a, &q_b);
        let (rc_c, q_c) = generic.run_alias(&n);
        eqi("generic inplace rc", rc_c, rc_b);
        eqb("generic inplace q", &q_c, &q_b);
    }
}

// 7.6 — RFC 7748 X25519 vector 1.
#[test]
fn curve25519_mult_vector() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let g = P3::new("crypto_scalarmult");
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let p = h32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
    let (rc, q) = m.run(&n, &p);
    eqi("mult RFC7748 rc", rc, 0);
    assert_eq!(
        hex(&q),
        "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"
    );
    let (rc2, q2) = g.run(&n, &p);
    eqi("generic mult rc", rc2, 0);
    eqb("generic == curve25519", &q, &q2);

    // RFC 7748 vector 2.
    let n2 = h32("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d");
    let p2 = h32("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493");
    let (rc3, q3) = m.run(&n2, &p2);
    eqi("mult RFC7748 vec2 rc", rc3, 0);
    assert_eq!(
        hex(&q3),
        "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957"
    );
}

// 7.7 — `mult(n, basepoint)` must agree with `_base(n)`.
#[test]
fn curve25519_basepoint_vs_base() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let b = P2::new("crypto_scalarmult_curve25519_base");
    let bp = h32(X25519_BASEPOINT);
    let mut rng = Rng::new(0x7701);
    for i in 0..80 {
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        if i % 2 == 0 {
            clamp_curve(&mut n);
        }
        let (rc1, q1) = m.run(&n, &bp);
        let (rc2, q2) = b.run(&n);
        eqi("ladder vs base rc", rc1, 0);
        eqi("base rc", rc2, 0);
        eqb("ladder(basepoint) == mult_base", &q1, &q2);
    }
    // Basepoint with the top bit set is masked away by fe25519_frombytes.
    let mut bp_hi = bp;
    bp_hi[31] |= 0x80;
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let (rc1, q1) = m.run(&n, &bp);
    let (rc2, q2) = m.run(&n, &bp_hi);
    eqi("bit255 basepoint rc", rc1, rc2);
    eqb("bit255 basepoint q", &q1, &q2);
}

// 7.8, 7.9 — non-canonical (but not blocklisted) points are accepted.
#[test]
fn curve25519_noncanonical_points_accepted() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");

    // p+2 … p+8 (>= 2^255-19, not on the blocklist).
    for k in 2u8..=8 {
        let mut p = h32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");
        p[0] = 0xed + k;
        let (rc, q) = m.run(&n, &p);
        eqi(&format!("p+{k} accepted"), rc, 0);
        assert_ne!(q, pat());
        // Reducing by hand: p + k ≡ k, i.e. the same as the canonical `k`.
        let mut red = [0u8; 32];
        red[0] = k;
        let (rc2, q2) = m.run(&n, &red);
        eqi("reduced form rc", rc, rc2);
        eqb("non-canonical reduces mod p", &q, &q2);
    }
    // 7.9 — bit 255 set on an otherwise valid point.
    let mut rng = Rng::new(0x7901);
    let base = P2::new("crypto_scalarmult_curve25519_base");
    for _ in 0..32 {
        let sk: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (_, pk) = base.run(&sk);
        let mut pk_hi: [u8; 32] = pk.clone().try_into().unwrap();
        pk_hi[31] |= 0x80;
        let (rc1, q1) = m.run(&n, &pk);
        let (rc2, q2) = m.run(&n, &pk_hi);
        eqi("bit255 masked rc", rc1, rc2);
        eqb("bit255 masked q", &q1, &q2);
        eqi("accepted", rc1, 0);
    }
}

// 7.10 — 100 random X25519 DH agreements.
#[test]
fn curve25519_dh_agreement() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let b = P2::new("crypto_scalarmult_curve25519_base");
    let mg = P3::new("crypto_scalarmult");
    let bg = P2::new("crypto_scalarmult_base");
    let mut rng = Rng::new(0x7a01);
    for i in 0..100 {
        let ska: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let skb: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (_, pka) = b.run(&ska);
        let (_, pkb) = b.run(&skb);
        let (rc1, s1) = m.run(&ska, &pkb);
        let (rc2, s2) = m.run(&skb, &pka);
        eqi("dh rc a", rc1, 0);
        eqi("dh rc b", rc2, 0);
        eqb(&format!("dh agreement #{i}"), &s1, &s2);
        // The generic alias must be indistinguishable.
        let (_, pka_g) = bg.run(&ska);
        eqb("generic base", &pka, &pka_g);
        let (_, s1g) = mg.run(&ska, &pkb);
        eqb("generic mult", &s1, &s1g);
    }
}

// 7.11 — `n` all zero with a valid `p` still succeeds because of clamping.
#[test]
fn curve25519_zero_scalar_with_valid_point() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let b = P2::new("crypto_scalarmult_curve25519_base");
    let zero = [0u8; 32];
    let mut rng = Rng::new(0x7b01);
    for _ in 0..16 {
        let sk: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (_, pk) = b.run(&sk);
        let (rc, q) = m.run(&zero, &pk);
        eqi("zero scalar with valid p", rc, 0);
        assert_ne!(q, pat(), "q written");
        // Effective scalar is 2^254 -> same as the explicitly clamped zero.
        let mut clamped = zero;
        clamp_curve(&mut clamped);
        let (rc2, q2) = m.run(&clamped, &pk);
        eqi("clamped zero rc", rc, rc2);
        eqb("clamped zero q", &q, &q2);
    }
    // all-0xff scalar with a valid point
    let ff = [0xffu8; 32];
    let sk: [u8; 32] = rng.bytes(32).try_into().unwrap();
    let (_, pk) = b.run(&sk);
    let (rc, _) = m.run(&ff, &pk);
    eqi("0xff scalar", rc, 0);
    // n = L with a valid point
    let (rc, _) = m.run(&h32(L_HEX), &pk);
    eqi("n = L", rc, 0);
}

// 7.12 / error 7.128 — runtime dispatch is a no-op on this build.
#[test]
fn curve25519_pick_best_implementation() {
    let (c, r) = both::<IntFn>("_crypto_scalarmult_curve25519_pick_best_implementation");
    let m = P3::new("crypto_scalarmult_curve25519");
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let p = h32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
    let (_, before) = m.run(&n, &p);
    for _ in 0..3 {
        let (rc, rr) = unsafe { (c(), r()) };
        eqi("pick_best_implementation", rc, rr);
        eqi("pick_best_implementation == 0", rc, 0);
        let (rc, after) = m.run(&n, &p);
        eqi("mult after dispatch", rc, 0);
        eqb("dispatch does not change results", &before, &after);
    }
}

// errors 7.1–7.8, 7.10 — the small-order blocklist; `q` must stay untouched.
#[test]
fn curve25519_small_order_rejected_q_untouched() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let g = P3::new("crypto_scalarmult");
    let mut rng = Rng::new(0x7c01);
    for (idx, hs) in CURVE_SMALL_ORDER.iter().enumerate() {
        for trial in 0..4 {
            let n: [u8; 32] = if trial == 0 {
                h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
            } else {
                rng.bytes(32).try_into().unwrap()
            };
            let p = h32(hs);
            let (rc, q) = m.run(&n, &p);
            eqi(&format!("blocklist[{idx}] rc"), rc, -1);
            eqb(&format!("blocklist[{idx}] q untouched"), &q, &pat());
            let (rc2, q2) = g.run(&n, &p);
            eqi("generic blocklist rc", rc2, -1);
            eqb("generic blocklist q untouched", &q2, &pat());

            // error 7.8 — bit 255 cannot bypass the guard.
            let mut p_hi = p;
            p_hi[31] |= 0x80;
            let (rc3, q3) = m.run(&n, &p_hi);
            eqi(&format!("blocklist[{idx}]|0x80 rc"), rc3, -1);
            eqb(&format!("blocklist[{idx}]|0x80 q untouched"), &q3, &pat());
        }
    }
    // Aliased output on the early-reject path: q == p means q keeps p.
    for hs in CURVE_SMALL_ORDER.iter() {
        let p = h32(hs);
        let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (rc, q) = m.run_alias_p(&n, &p);
        eqi("aliased blocklist rc", rc, -1);
        eqb("aliased blocklist q == p", &q, &p);
    }
}

// Generic FFI boundary: all-zero / all-0xff scalars and points.
#[test]
fn curve25519_boundary_shapes() {
    let m = P3::new("crypto_scalarmult_curve25519");
    let b = P2::new("crypto_scalarmult_curve25519_base");
    for nh in [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        L_HEX,
        "0100000000000000000000000000000000000000000000000000000000000000",
    ] {
        let n = h32(nh);
        let (rc, _) = b.run(&n);
        eqi("base never fails", rc, 0);
        for ph in [
            "0000000000000000000000000000000000000000000000000000000000000000",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0200000000000000000000000000000000000000000000000000000000000000",
            "0900000000000000000000000000000000000000000000000000000000000000",
            X25519_BASEPOINT,
        ] {
            let p = h32(ph);
            let (_rc, _q) = m.run(&n, &p);
        }
    }
    // `p` = all 0xff: bit 255 masked -> 2^255-1 = p+18, not blocklisted.
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let (rc, _) = m.run(&n, &[0xffu8; 32]);
    eqi("all-0xff point accepted", rc, 0);
}

// ================================================================== ed25519

// 7.13 — random scalars through `_base`; result must be a valid point.
#[test]
fn ed25519_base_random_points_valid() {
    let b = P2::new("crypto_scalarmult_ed25519_base");
    let (cv, rv) = both::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ed25519_is_valid_point",
    );
    let mut rng = Rng::new(0x8001);
    for _ in 0..80 {
        let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (rc, q) = b.run(&n);
        eqi("ed25519_base rc", rc, 0);
        let (a, bb) = unsafe { (cv(q.as_ptr()), rv(q.as_ptr())) };
        eqi("is_valid_point agreement", a, bb);
        assert_eq!(a, 1, "output must be canonical and on the main subgroup");
    }
}

// 7.14 — clamped vs noclamp.
#[test]
fn ed25519_base_clamp_vs_noclamp() {
    let bc = P2::new("crypto_scalarmult_ed25519_base");
    let bn = P2::new("crypto_scalarmult_ed25519_base_noclamp");
    let mut rng = Rng::new(0x8101);
    for _ in 0..64 {
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        // Pre-clamped: the two must agree.
        n[0] &= 248;
        n[31] &= 127;
        n[31] |= 64;
        let (rc1, q1) = bc.run(&n);
        let (rc2, q2) = bn.run(&n);
        eqi("clamped rc", rc1, 0);
        eqi("noclamp rc", rc2, 0);
        eqb("pre-clamped scalars agree", &q1, &q2);

        // Not clamped: they must differ.
        let mut m = n;
        m[0] |= 1;
        m[31] &= 0x3f;
        let (rc3, q3) = bc.run(&m);
        let (rc4, q4) = bn.run(&m);
        eqi("clamped rc", rc3, 0);
        eqi("noclamp rc", rc4, 0);
        assert_ne!(q3, q4, "clamping must change the result");
    }
}

// 7.15, 7.16, 7.17 — small `n` vectors, homomorphism, bit-255 masking.
#[test]
fn ed25519_base_noclamp_vectors() {
    let bn = P2::new("crypto_scalarmult_ed25519_base_noclamp");
    let (ca, ra) = both::<Mult>("crypto_core_ed25519_add");

    let mut one = [0u8; 32];
    one[0] = 1;
    let (rc, q) = bn.run(&one);
    eqi("n=1 rc", rc, 0);
    assert_eq!(hex(&q), ED25519_BASEPOINT, "n=1 must be the basepoint");

    // 7.16 — additive homomorphism 1 … 8.
    let mut acc = q.clone();
    let bp = q.clone();
    for k in 2u8..=8 {
        let mut n = [0u8; 32];
        n[0] = k;
        let (rc, qk) = bn.run(&n);
        eqi(&format!("n={k} rc"), rc, 0);
        let mut sc = padded(32);
        let mut sr = padded(32);
        let x = unsafe { ca(sc.as_mut_ptr(), acc.as_ptr(), bp.as_ptr()) };
        let y = unsafe { ra(sr.as_mut_ptr(), acc.as_ptr(), bp.as_ptr()) };
        eqi("core add rc", x, y);
        eqb("core add", &sc, &sr);
        eqb(&format!("n={k} == repeated add"), &qk, &sc[..32]);
        acc = sc[..32].to_vec();
    }

    // n = L - 1 must equal -B; adding B gives the identity encoding.
    let mut lm1 = h32(L_HEX);
    lm1[0] -= 1;
    let (rc, negb) = bn.run(&lm1);
    eqi("n=L-1 rc", rc, 0);
    let mut sc = padded(32);
    let mut sr = padded(32);
    let x = unsafe { ca(sc.as_mut_ptr(), negb.as_ptr(), bp.as_ptr()) };
    let y = unsafe { ra(sr.as_mut_ptr(), negb.as_ptr(), bp.as_ptr()) };
    eqi("add rc", x, y);
    eqb("add", &sc, &sr);
    assert_eq!(
        hex(&sc[..32]),
        "0100000000000000000000000000000000000000000000000000000000000000",
        "(L-1)B + B must be the identity"
    );

    // 7.17 — bit 255 of `n` is always cleared.
    let mut rng = Rng::new(0x8201);
    for _ in 0..48 {
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;
        let mut n_hi = n;
        n_hi[31] |= 0x80;
        for f in [
            "crypto_scalarmult_ed25519_base",
            "crypto_scalarmult_ed25519_base_noclamp",
        ] {
            let b = P2::new(f);
            let (rc1, q1) = b.run(&n);
            let (rc2, q2) = b.run(&n_hi);
            eqi(&format!("{f} bit255 rc"), rc1, rc2);
            eqb(&format!("{f} bit255 masked"), &q1, &q2);
        }
    }
}

// 7.18, 7.19 — the point variants.
#[test]
fn ed25519_mult_commutativity() {
    let bc = P2::new("crypto_scalarmult_ed25519_base");
    let bn = P2::new("crypto_scalarmult_ed25519_base_noclamp");
    let mc = P3::new("crypto_scalarmult_ed25519");
    let mn = P3::new("crypto_scalarmult_ed25519_noclamp");
    let (csm, rsm) = both::<Mult>("crypto_core_ed25519_scalar_mul");
    let mut rng = Rng::new(0x8301);

    for _ in 0..60 {
        let n1: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let n2: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (_, p2) = bc.run(&n2);
        let (_, p1) = bc.run(&n1);
        let (rc1, q1) = mc.run(&n1, &p2);
        let (rc2, q2) = mc.run(&n2, &p1);
        eqi("mult rc", rc1, 0);
        eqi("mult rc", rc2, 0);
        eqb("clamped DH commutes", &q1, &q2);

        // == base_noclamp(clamp(n1) * clamp(n2) mod L)
        let mut c1 = n1;
        let mut c2 = n2;
        for c in [&mut c1, &mut c2] {
            c[0] &= 248;
            c[31] &= 127;
            c[31] |= 64;
        }
        let mut pc = padded(32);
        let mut pr = padded(32);
        unsafe {
            csm(pc.as_mut_ptr(), c1.as_ptr(), c2.as_ptr());
            rsm(pr.as_mut_ptr(), c1.as_ptr(), c2.as_ptr());
        }
        eqb("scalar_mul", &pc, &pr);
        let (rc3, q3) = bn.run(&pc[..32]);
        eqi("base_noclamp(product) rc", rc3, 0);
        eqb("mult == base_noclamp(product)", &q1, &q3);
    }

    // 7.19 — noclamp against the basepoint.
    let bp = h32(ED25519_BASEPOINT);
    for _ in 0..60 {
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;
        let (rc1, q1) = mn.run(&n, &bp);
        let (rc2, q2) = bn.run(&n);
        eqi("noclamp mult rc", rc1, rc2);
        eqb("noclamp(basepoint) == base_noclamp", &q1, &q2);
    }
}

// 7.20, 7.21 — aliasing.
#[test]
fn ed25519_inplace_aliasing() {
    let mut rng = Rng::new(0x8401);
    let bc = P2::new("crypto_scalarmult_ed25519_base");
    for (mname, bname) in [
        ("crypto_scalarmult_ed25519", "crypto_scalarmult_ed25519_base"),
        (
            "crypto_scalarmult_ed25519_noclamp",
            "crypto_scalarmult_ed25519_base_noclamp",
        ),
    ] {
        let m = P3::new(mname);
        let b = P2::new(bname);
        for _ in 0..40 {
            let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let seed: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let (_, p) = bc.run(&seed);
            let p: [u8; 32] = p.try_into().unwrap();

            let (rc0, q0) = m.run(&n, &p);
            let (rc1, q1) = m.run_alias_n(&n, &p);
            eqi(&format!("{mname} alias q==n rc"), rc0, rc1);
            eqb(&format!("{mname} alias q==n"), &q0, &q1);
            let (rc2, q2) = m.run_alias_p(&n, &p);
            eqi(&format!("{mname} alias q==p rc"), rc0, rc2);
            eqb(&format!("{mname} alias q==p"), &q0, &q2);

            // 7.21 — `_base` aliasing.
            let (rc3, q3) = b.run(&n);
            let (rc4, q4) = b.run_alias(&n);
            eqi(&format!("{bname} alias rc"), rc3, rc4);
            eqb(&format!("{bname} alias"), &q3, &q4);
        }
    }
}

// errors 7.13–7.25.
#[test]
fn ed25519_error_paths_and_side_effects() {
    let bc = P2::new("crypto_scalarmult_ed25519_base");
    let mut rng = Rng::new(0x8501);
    let (_, valid_p) = bc.run(&rng.bytes(32));
    let valid_p: [u8; 32] = valid_p.try_into().unwrap();

    for mname in [
        "crypto_scalarmult_ed25519",
        "crypto_scalarmult_ed25519_noclamp",
    ] {
        let m = P3::new(mname);
        let n: [u8; 32] = rng.bytes(32).try_into().unwrap();

        // 7.13 — non-canonical `y` encodings: p[0] >= 0xed, p[1..30]=0xff,
        // p[31] & 0x7f == 0x7f.
        for lo in [0xedu8, 0xee, 0xef, 0xf0, 0xfe, 0xff] {
            let mut p = [0xffu8; 32];
            p[0] = lo;
            p[31] = 0x7f;
            let (rc, q) = m.run(&n, &p);
            eqi(&format!("{mname} non-canonical p rc"), rc, -1);
            eqb(&format!("{mname} non-canonical p: q untouched"), &q, &pat());
            let mut p2 = p;
            p2[31] |= 0x80;
            let (rc2, q2) = m.run(&n, &p2);
            eqi(&format!("{mname} non-canonical p|0x80 rc"), rc2, -1);
            eqb("q untouched", &q2, &pat());
        }

        // 7.14 — `y` that does not decode.
        let mut nondecode = 0usize;
        for k in 2u8..64 {
            let mut p = [0u8; 32];
            p[0] = k;
            let (rc, q) = m.run(&n, &p);
            if rc == -1 {
                eqb(&format!("{mname} p={k}: q untouched"), &q, &pat());
                nondecode += 1;
            }
        }
        assert!(nondecode > 0, "expected some non-decodable y values");

        // 7.15, 7.16 — the 8 torsion encodings.
        for hs in ED_SMALL_ORDER.iter() {
            let p = h32(hs);
            let (rc, q) = m.run(&n, &p);
            eqi(&format!("{mname} small order {hs} rc"), rc, -1);
            eqb(&format!("{mname} small order {hs}: q untouched"), &q, &pat());
        }

        // 7.17 — a valid point off the main subgroup (order 2L / 8L).
        let (ca, ra) = both::<Mult>("crypto_core_ed25519_add");
        for tors in [ED_SMALL_ORDER[1], ED_SMALL_ORDER[4], ED_SMALL_ORDER[5]] {
            let t = h32(tors);
            let mut oc = padded(32);
            let mut or = padded(32);
            let x = unsafe { ca(oc.as_mut_ptr(), valid_p.as_ptr(), t.as_ptr()) };
            let y = unsafe { ra(or.as_mut_ptr(), valid_p.as_ptr(), t.as_ptr()) };
            eqi("core add rc", x, y);
            eqb("core add", &oc, &or);
            let off: [u8; 32] = oc[..32].try_into().unwrap();
            let (rc, q) = m.run(&n, &off);
            eqi(&format!("{mname} off-subgroup rc"), rc, -1);
            eqb(&format!("{mname} off-subgroup: q untouched"), &q, &pat());
        }
    }

    // 7.18 — clamped, n = 0, valid p: late reject, q holds a real point.
    let mc = P3::new("crypto_scalarmult_ed25519");
    let (rc, q) = mc.run(&[0u8; 32], &valid_p);
    eqi("clamped n=0 rc", rc, -1);
    assert_ne!(q, pat(), "late reject must leave q written");
    assert_ne!(
        hex(&q),
        "0100000000000000000000000000000000000000000000000000000000000000",
        "clamped n=0 is scalar 2^254, not the identity"
    );
    // The written value must be the clamped result.
    let mut clamped = [0u8; 32];
    clamped[0] &= 248;
    clamped[31] |= 64;
    let mn = P3::new("crypto_scalarmult_ed25519_noclamp");
    let (rc2, q2) = mn.run(&clamped, &valid_p);
    eqi("noclamp(clamp(0)) rc", rc2, 0);
    eqb("late reject q == clamped result", &q, &q2);

    // 7.19 — noclamp n = 0: q is the identity encoding.
    let (rc, q) = mn.run(&[0u8; 32], &valid_p);
    eqi("noclamp n=0 rc", rc, -1);
    assert_eq!(
        hex(&q),
        "0100000000000000000000000000000000000000000000000000000000000000"
    );

    // 7.20 — n = L (and multiples), bit 255 clear.
    let l = h32(L_HEX);
    let (rc, q) = mn.run(&l, &valid_p);
    eqi("noclamp n=L rc", rc, -1);
    assert_eq!(
        hex(&q),
        "0100000000000000000000000000000000000000000000000000000000000000"
    );
    // 7.21 — L + 2^255 reduces to the same thing.
    let mut l_hi = l;
    l_hi[31] |= 0x80;
    let (rc2, q2) = mn.run(&l_hi, &valid_p);
    eqi("noclamp n=L+2^255 rc", rc2, -1);
    eqb("bit255 ignored", &q, &q2);

    // 7.22 — base, clamped, n = 0.
    let bcp = P2::new("crypto_scalarmult_ed25519_base");
    let bnp = P2::new("crypto_scalarmult_ed25519_base_noclamp");
    let (rc, q) = bcp.run(&[0u8; 32]);
    eqi("base clamped n=0 rc", rc, -1);
    let (rc2, q2) = bnp.run(&clamped);
    eqi("base_noclamp(clamp(0)) rc", rc2, 0);
    eqb("base late reject q", &q, &q2);

    // 7.23 — base_noclamp, n = 0.
    let (rc, q) = bnp.run(&[0u8; 32]);
    eqi("base_noclamp n=0 rc", rc, -1);
    assert_eq!(
        hex(&q),
        "0100000000000000000000000000000000000000000000000000000000000000"
    );

    // 7.24 — base_noclamp, n = L (and L + 2^255).
    let (rc, q) = bnp.run(&l);
    eqi("base_noclamp n=L rc", rc, -1);
    assert_eq!(
        hex(&q),
        "0100000000000000000000000000000000000000000000000000000000000000"
    );
    let (rc2, q2) = bnp.run(&l_hi);
    eqi("base_noclamp n=L+2^255 rc", rc2, -1);
    eqb("bit255 ignored", &q, &q2);

    // 2L (mod 2^255) is also ≡ 0 mod L with bit 255 clear.
    let mut two_l = [0u8; 32];
    let mut carry = 0u16;
    for i in 0..32 {
        let v = (l[i] as u16) * 2 + carry;
        two_l[i] = v as u8;
        carry = v >> 8;
    }
    two_l[31] &= 0x7f;
    let (rc, q) = bnp.run(&two_l);
    eqi("base_noclamp n=2L rc", rc, -1);
    assert_eq!(
        hex(&q),
        "0100000000000000000000000000000000000000000000000000000000000000"
    );
}

// ============================================================== ristretto255

// 7.22 — the published multiples of the ristretto255 basepoint.
#[test]
fn ristretto_base_vectors() {
    let b = P2::new("crypto_scalarmult_ristretto255_base");
    for (i, want) in RISTRETTO_MULTIPLES.iter().enumerate() {
        let mut n = [0u8; 32];
        n[0] = (i + 1) as u8;
        let (rc, q) = b.run(&n);
        eqi(&format!("ristretto base n={} rc", i + 1), rc, 0);
        assert_eq!(hex(&q), *want, "ristretto multiple {}", i + 1);
    }
    // Self-consistency against repeated addition.
    let (ca, ra) = both::<Mult>("crypto_core_ristretto255_add");
    let bp = h32(RISTRETTO_MULTIPLES[0]);
    let mut acc = bp;
    for i in 1..15 {
        let mut oc = padded(32);
        let mut or = padded(32);
        let x = unsafe { ca(oc.as_mut_ptr(), acc.as_ptr(), bp.as_ptr()) };
        let y = unsafe { ra(or.as_mut_ptr(), acc.as_ptr(), bp.as_ptr()) };
        eqi("ristretto add rc", x, y);
        eqb("ristretto add", &oc, &or);
        assert_eq!(hex(&oc[..32]), RISTRETTO_MULTIPLES[i]);
        acc = oc[..32].try_into().unwrap();
    }
}

// 7.23 — DH commutativity on the ristretto group.
#[test]
fn ristretto_dh_commutes() {
    let b = P2::new("crypto_scalarmult_ristretto255_base");
    let m = P3::new("crypto_scalarmult_ristretto255");
    let (csr, rsr) = both::<unsafe extern "C" fn(*mut u8, *const u8)>(
        "crypto_core_ristretto255_scalar_reduce",
    );
    let mut rng = Rng::new(0x9001);
    for i in 0..80 {
        // Reduce 64 random bytes into a canonical non-zero scalar.
        let mk = |rng: &mut Rng| -> [u8; 32] {
            loop {
                let raw = rng.bytes(64);
                let mut oc = padded(32);
                let mut or = padded(32);
                unsafe {
                    csr(oc.as_mut_ptr(), raw.as_ptr());
                    rsr(or.as_mut_ptr(), raw.as_ptr());
                }
                eqb("scalar_reduce", &oc, &or);
                if oc[..32].iter().any(|&x| x != 0) {
                    return oc[..32].try_into().unwrap();
                }
            }
        };
        let a = mk(&mut rng);
        let c = mk(&mut rng);
        let (rc1, pa) = b.run(&a);
        let (rc2, pc) = b.run(&c);
        eqi("ristretto base rc", rc1, 0);
        eqi("ristretto base rc", rc2, 0);
        let (rc3, s1) = m.run(&a, &pc);
        let (rc4, s2) = m.run(&c, &pa);
        eqi("ristretto mult rc", rc3, 0);
        eqi("ristretto mult rc", rc4, 0);
        eqb(&format!("ristretto DH #{i}"), &s1, &s2);
    }
}

// 7.24, 7.25 — bit-255 masking and the `n = 1` identity map.
#[test]
fn ristretto_bit255_and_unit_scalar() {
    let b = P2::new("crypto_scalarmult_ristretto255_base");
    let m = P3::new("crypto_scalarmult_ristretto255");
    let mut rng = Rng::new(0x9101);
    let mut one = [0u8; 32];
    one[0] = 1;
    for i in 0..40 {
        let mut s = [0u8; 32];
        s[0] = (i + 1) as u8;
        let (_, p) = b.run(&s);
        let p: [u8; 32] = p.try_into().unwrap();

        // 7.25 — n = 1 is the identity map (and re-encodes canonically).
        let (rc, q) = m.run(&one, &p);
        eqi("n=1 rc", rc, 0);
        eqb("n=1 is the identity map", &q, &p);

        // 7.24 — bit 255 of `n` is masked off.
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;
        let mut n_hi = n;
        n_hi[31] |= 0x80;
        let (rc1, q1) = m.run(&n, &p);
        let (rc2, q2) = m.run(&n_hi, &p);
        eqi("bit255 rc", rc1, rc2);
        eqb("bit255 masked", &q1, &q2);
        let (rc3, q3) = b.run(&n);
        let (rc4, q4) = b.run(&n_hi);
        eqi("base bit255 rc", rc3, rc4);
        eqb("base bit255 masked", &q3, &q4);
    }
}

// 7.26 — aliasing on the ristretto paths.
#[test]
fn ristretto_inplace_aliasing() {
    let b = P2::new("crypto_scalarmult_ristretto255_base");
    let m = P3::new("crypto_scalarmult_ristretto255");
    let mut rng = Rng::new(0x9201);
    for i in 0..40 {
        let mut s = [0u8; 32];
        s[0] = (i + 3) as u8;
        let (_, p) = b.run(&s);
        let p: [u8; 32] = p.try_into().unwrap();
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;

        let (rc0, q0) = m.run(&n, &p);
        let (rc1, q1) = m.run_alias_n(&n, &p);
        eqi("alias q==n rc", rc0, rc1);
        eqb("alias q==n", &q0, &q1);
        let (rc2, q2) = m.run_alias_p(&n, &p);
        eqi("alias q==p rc", rc0, rc2);
        eqb("alias q==p", &q0, &q2);

        let (rc3, q3) = b.run(&n);
        let (rc4, q4) = b.run_alias(&n);
        eqi("base alias rc", rc3, rc4);
        eqb("base alias", &q3, &q4);
    }
}

// 7.27 — ristretto255 and ed25519_noclamp are not interchangeable.
#[test]
fn ristretto_vs_ed25519_noclamp_differ() {
    let rb = P2::new("crypto_scalarmult_ristretto255_base");
    let rm = P3::new("crypto_scalarmult_ristretto255");
    let em = P3::new("crypto_scalarmult_ed25519_noclamp");
    let mut rng = Rng::new(0x9301);
    let mut compared = 0;
    for i in 0..64 {
        let mut s = [0u8; 32];
        s[0] = (i + 1) as u8;
        let (_, p) = rb.run(&s);
        let p: [u8; 32] = p.try_into().unwrap();
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;
        let (rc1, q1) = rm.run(&n, &p);
        let (rc2, q2) = em.run(&n, &p);
        if rc1 == 0 && rc2 == 0 {
            assert_ne!(q1, q2, "ristretto255 and ed25519 must not agree");
            compared += 1;
        }
    }
    assert!(compared > 0, "no encoding decoded under both interpretations");
}

// errors 7.26–7.33.
#[test]
fn ristretto_error_paths_and_side_effects() {
    let b = P2::new("crypto_scalarmult_ristretto255_base");
    let m = P3::new("crypto_scalarmult_ristretto255");
    let (cv, rv) = both::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ristretto255_is_valid_point",
    );
    let mut rng = Rng::new(0x9401);
    let mut nn: [u8; 32] = rng.bytes(32).try_into().unwrap();
    nn[31] &= 0x7f;

    // 7.26 — non-canonical byte strings: >= 2^255-19, or bit 255 set.
    let mut noncanon: Vec<[u8; 32]> = Vec::new();
    for k in 0u8..8 {
        let mut p = [0xffu8; 32];
        p[0] = 0xedu8.wrapping_add(k);
        p[31] = 0x7f;
        noncanon.push(p);
    }
    for _ in 0..8 {
        let mut p: [u8; 32] = rng.bytes(32).try_into().unwrap();
        p[31] |= 0x80;
        noncanon.push(p);
    }
    for p in &noncanon {
        let (rc, q) = m.run(&nn, p);
        eqi("non-canonical ristretto rc", rc, -1);
        eqb("non-canonical ristretto: q untouched", &q, &pat());
    }

    // 7.27, 7.28 — canonical bytes that are not on the ristretto image.
    let mut rejected = 0;
    for _ in 0..200 {
        let mut p: [u8; 32] = rng.bytes(32).try_into().unwrap();
        p[31] &= 0x7f;
        let (a, bb) = unsafe { (cv(p.as_ptr()), rv(p.as_ptr())) };
        eqi("ristretto is_valid_point agreement", a, bb);
        if a != 0 {
            continue;
        }
        let (rc, q) = m.run(&nn, &p);
        eqi("bad ristretto encoding rc", rc, -1);
        eqb("bad ristretto encoding: q untouched", &q, &pat());
        rejected += 1;
        if rejected >= 40 {
            break;
        }
    }
    assert!(rejected > 0, "expected invalid ristretto encodings");

    // Aliased early reject: q == p leaves p in place.
    let (rc, q) = m.run_alias_p(&nn, &noncanon[0]);
    eqi("aliased non-canonical rc", rc, -1);
    eqb("aliased non-canonical q == p", &q, &noncanon[0]);

    // 7.29 — the identity encoding is *accepted* but yields a zero output.
    let zero = [0u8; 32];
    let (a, bb) = unsafe { (cv(zero.as_ptr()), rv(zero.as_ptr())) };
    eqi("identity validity agreement", a, bb);
    for _ in 0..8 {
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;
        let (rc, q) = m.run(&n, &zero);
        eqi("ristretto identity point rc", rc, -1);
        assert_eq!(hex(&q), hex(&[0u8; 32]), "late reject: q = 32 zero bytes");
    }

    // 7.30 — n = 0 with a valid non-identity p.
    let mut s = [0u8; 32];
    s[0] = 5;
    let (_, p) = b.run(&s);
    let p: [u8; 32] = p.try_into().unwrap();
    let (rc, q) = m.run(&[0u8; 32], &p);
    eqi("ristretto n=0 rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));

    // 7.31 — n = L (and L + 2^255, and 2L mod 2^255).
    let l = h32(L_HEX);
    let (rc, q) = m.run(&l, &p);
    eqi("ristretto n=L rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));
    let mut l_hi = l;
    l_hi[31] |= 0x80;
    let (rc, q) = m.run(&l_hi, &p);
    eqi("ristretto n=L+2^255 rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));

    // 7.32 — base with n = 0.
    let (rc, q) = b.run(&[0u8; 32]);
    eqi("ristretto base n=0 rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));

    // 7.33 — base with n = L, and with bit 255 set.
    let (rc, q) = b.run(&l);
    eqi("ristretto base n=L rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));
    let (rc, q) = b.run(&l_hi);
    eqi("ristretto base n=L+2^255 rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));

    // Aliased late reject on the base path.
    let (rc, q) = b.run_alias(&[0u8; 32]);
    eqi("aliased base n=0 rc", rc, -1);
    assert_eq!(hex(&q), hex(&[0u8; 32]));
}
