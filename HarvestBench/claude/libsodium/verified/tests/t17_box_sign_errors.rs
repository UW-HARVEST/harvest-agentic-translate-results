//! Phase C — G5 error surface (`ERRORS.md` section `## G5`).
//!
//! Three kinds of row:
//!
//! * **`return -1` rows** — driven directly on both `.so`s; the return value,
//!   every out-parameter and the exact contents of the output buffers (was it
//!   left untouched? zeroed? partially written?) are compared byte-for-byte
//!   using `canary()`-filled buffers.
//! * **`sodium_misuse()` rows** — run in a child process with the observing
//!   handler installed, so both the abort and the side effects written before
//!   it are compared (`MISUSE_EXIT`).
//! * **raw NULL-dereference rows** — `crypto_auth_hmacsha{256,512}_init` with
//!   `key == NULL` and `keylen > blocksize` takes the key-hashing branch
//!   *before* the NULL guard, so it segfaults instead of aborting cleanly.
//!   Also run out of process; C and Rust must die with the same signal.

mod common;
use common::*;
use std::ptr;

// ---------------------------------------------------------------------------
// C signatures (same shapes as in t16)
// ---------------------------------------------------------------------------

type SizeFn = unsafe extern "C" fn() -> usize;
type Two = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
type Three = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Sym5 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
type Asym6 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32;
type Det6 = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> i32;
type Det7 = unsafe extern "C" fn(
    *mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8,
) -> i32;
type ODet6 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> i32;
type ODet7 = unsafe extern "C" fn(
    *mut u8, *const u8, *const u8, u64, *const u8, *const u8, *const u8,
) -> i32;
type Seal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type SealOpen = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;

type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type SsPush =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> i32;
type SsPull = unsafe extern "C" fn(
    *mut u8, *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64,
) -> i32;
type SsRekey = unsafe extern "C" fn(*mut u8);

type KxSession = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32;

type Sign5 = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type Verify4 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type PhInit = unsafe extern "C" fn(*mut u8) -> i32;
type PhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type PhCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> i32;
type PhVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

type Auth4 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type AuthV4 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type AuthInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type AuthUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type AuthFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

// ---------------------------------------------------------------------------
// helpers / fixed rejection inputs
// ---------------------------------------------------------------------------

static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn hx(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn rep(first: &[u8], fill: u8, len: usize, last: Option<u8>) -> Vec<u8> {
    let mut v = first.to_vec();
    while v.len() < len {
        v.push(fill);
    }
    v.truncate(len);
    if let Some(l) = last {
        v[len - 1] = l;
    }
    v
}

/// The seven X25519 `has_small_order()` blocklist entries plus two entries
/// with bit 7 of byte 31 set (the comparison masks that bit off, so they are
/// rejected too) — `ERRORS.md` G5-001 / G5-002.
fn small_order_pks() -> Vec<(String, Vec<u8>)> {
    vec![
        ("00*32".into(), vec![0u8; 32]),
        ("01 00*31".into(), rep(&[1], 0, 32, None)),
        (
            "e0eb7a..b800".into(),
            hx("e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800"),
        ),
        (
            "5f9c95..1157".into(),
            hx("5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157"),
        ),
        ("p-1 (ecff..7f)".into(), rep(&[0xec], 0xff, 32, Some(0x7f))),
        ("p (edff..7f)".into(), rep(&[0xed], 0xff, 32, Some(0x7f))),
        ("p+1 (eeff..7f)".into(), rep(&[0xee], 0xff, 32, Some(0x7f))),
        // bit-7-set variants of two of the above
        ("00*31 80".into(), rep(&[0], 0, 32, Some(0x80))),
        ("ecff..ffff".into(), rep(&[0xec], 0xff, 32, None)),
    ]
}

/// ed25519 public keys that are not canonical (`ge25519_is_canonical == 0`) —
/// `ERRORS.md` G5-073.
fn noncanonical_ed_pks() -> Vec<(String, Vec<u8>)> {
    vec![
        ("y=p   (edff..7f)".into(), rep(&[0xed], 0xff, 32, Some(0x7f))),
        ("y=p+1 (eeff..7f)".into(), rep(&[0xee], 0xff, 32, Some(0x7f))),
        ("ff*32".into(), vec![0xffu8; 32]),
    ]
}

/// ed25519 encodings that are canonical but fail point decompression —
/// `ERRORS.md` G5-074 / G5-076 / G5-089.
fn undecodable_ed_pks() -> Vec<(String, Vec<u8>)> {
    vec![
        ("02 00*31".into(), rep(&[2], 0, 32, None)),
        ("07 00*31".into(), rep(&[7], 0, 32, None)),
        ("08 00*31".into(), rep(&[8], 0, 32, None)),
    ]
}

/// ed25519 small-order points — `ERRORS.md` G5-075 / G5-077 / G5-090.
fn small_order_ed_pks() -> Vec<(String, Vec<u8>)> {
    vec![
        ("00*32".into(), vec![0u8; 32]),
        ("01 00*31".into(), rep(&[1], 0, 32, None)),
        ("ecff..7f".into(), rep(&[0xec], 0xff, 32, Some(0x7f))),
        ("00*31 80".into(), rep(&[0], 0, 32, Some(0x80))),
        (
            "26e8958f..fc05".into(),
            hx("26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05"),
        ),
        (
            "c7176a70..037a".into(),
            hx("c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a"),
        ),
        (
            "26e8958f..fc85".into(),
            hx("26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85"),
        ),
        (
            "c7176a70..03fa".into(),
            hx("c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa"),
        ),
    ]
}

/// Valid, non-small-order ed25519 points that lie outside the prime-order
/// subgroup — `ERRORS.md` G5-091.
fn off_subgroup_ed_pks() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    for b in [3u8, 4, 5, 6, 9, 0x0a] {
        v.push((format!("{b:02x} 00*31"), rep(&[b], 0, 32, None)));
    }
    v.push(("ff*32".into(), vec![0xffu8; 32]));
    v
}

/// `L` (the ed25519 group order) little-endian, and the neighbours the table
/// calls out — `ERRORS.md` G5-072.
const L_HEX: &str = "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";

fn noncanonical_scalars() -> Vec<(String, Vec<u8>)> {
    let l = hx(L_HEX);
    let mut lp1 = l.clone();
    lp1[0] = lp1[0].wrapping_add(1); // 0xed -> 0xee, no carry
    vec![
        ("S = L".into(), l),
        ("S = L+1".into(), lp1),
        ("S = ff*32".into(), vec![0xffu8; 32]),
    ]
}

fn box_kp(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let seed = rng.bytes(32);
    let f = sym::<SeedKeypair>(c_lib(), "crypto_box_seed_keypair");
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 32];
    unsafe { assert_eq!(f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
    (pk, sk)
}

fn sign_kp(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let seed = rng.bytes(32);
    let f = sym::<SeedKeypair>(c_lib(), "crypto_sign_ed25519_seed_keypair");
    let mut pk = vec![0u8; 32];
    let mut sk = vec![0u8; 64];
    unsafe { assert_eq!(f(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
    (pk, sk)
}

// ===========================================================================
// crypto_box — small-order peer keys
// ===========================================================================

/// G5-001, G5-002, G5-004, G5-005 — `_beforenm` (generic, xsalsa20 and
/// xchacha20) with every small-order / blocklisted `pk`: returns -1 and leaves
/// the caller's `k` buffer completely untouched.
#[test]
fn beforenm_rejects_small_order_pk() {
    setup();
    let mut rng = Rng::new(0x20000);
    for name in [
        "crypto_box_beforenm",
        "crypto_box_curve25519xsalsa20poly1305_beforenm",
        "crypto_box_curve25519xchacha20poly1305_beforenm",
    ] {
        let (c, r) = pair::<Three>(name);
        for (what, pk) in small_order_pks() {
            for _ in 0..4 {
                let sk = rng.bytes(32);
                let mut k1 = canary(32 + 8);
                let mut k2 = canary(32 + 8);
                let (x, y) = unsafe {
                    (
                        c(k1.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()),
                        r(k2.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()),
                    )
                };
                eq_i32(&format!("{name}(pk={what}) rc"), x, y);
                assert_eq!(x, -1, "{name}(pk={what}) must reject");
                eq_bytes(&format!("{name}(pk={what}) k"), &k1, &k2);
                assert_eq!(k1, canary(32 + 8), "{name}(pk={what}) must not write k");
            }
        }
        // an all-zero sk with a *valid* pk is accepted (it is clamped inside
        // X25519), so the rejection above really is about `pk`
        let (pk, _) = box_kp(&mut rng);
        let mut k1 = canary(32);
        let mut k2 = canary(32);
        let zero = [0u8; 32];
        let (x, y) = unsafe {
            (
                c(k1.as_mut_ptr(), pk.as_ptr(), zero.as_ptr()),
                r(k2.as_mut_ptr(), pk.as_ptr(), zero.as_ptr()),
            )
        };
        eq_i32(&format!("{name}(sk=0) rc"), x, y);
        eq_bytes(&format!("{name}(sk=0) k"), &k1, &k2);
    }
}

/// G5-007, G5-008, G5-009, G5-010, G5-011, G5-012, G5-013, G5-014, G5-015,
/// G5-016 — every `crypto_box` entry point that takes a key pair propagates
/// the `_beforenm` failure: -1 with the output buffers untouched (in
/// particular the `_open_*` forms do **not** zero `m`).
#[test]
fn box_layers_propagate_beforenm_failure() {
    setup();
    let mut rng = Rng::new(0x20100);
    let mlen = 100usize;
    let nacl_mlen = 132usize; // 32 ZEROBYTES + 100

    let nacl = pair::<Asym6>("crypto_box");
    let nacl_open = pair::<Asym6>("crypto_box_open");
    let xnacl = pair::<Asym6>("crypto_box_curve25519xsalsa20poly1305");
    let xnacl_open = pair::<Asym6>("crypto_box_curve25519xsalsa20poly1305_open");
    let easy = pair::<Asym6>("crypto_box_easy");
    let oeasy = pair::<Asym6>("crypto_box_open_easy");
    let det = pair::<Det7>("crypto_box_detached");
    let odet = pair::<ODet7>("crypto_box_open_detached");
    let xeasy = pair::<Asym6>("crypto_box_curve25519xchacha20poly1305_easy");
    let xoeasy = pair::<Asym6>("crypto_box_curve25519xchacha20poly1305_open_easy");
    let xdet = pair::<Det7>("crypto_box_curve25519xchacha20poly1305_detached");
    let xodet = pair::<ODet7>("crypto_box_curve25519xchacha20poly1305_open_detached");

    for (what, pk) in small_order_pks() {
        let sk = rng.bytes(32);
        let n = rng.bytes(24);
        let m = rng.bytes(nacl_mlen);

        // --- combined (NaCl) forms: c / m untouched
        for (fname, f, len) in [
            ("crypto_box", nacl, nacl_mlen),
            ("crypto_box_open", nacl_open, nacl_mlen),
            ("crypto_box_curve25519xsalsa20poly1305", xnacl, nacl_mlen),
            ("crypto_box_curve25519xsalsa20poly1305_open", xnacl_open, nacl_mlen),
            ("crypto_box_easy", easy, mlen + 16),
            ("crypto_box_open_easy", oeasy, mlen),
            ("crypto_box_curve25519xchacha20poly1305_easy", xeasy, mlen + 16),
            ("crypto_box_curve25519xchacha20poly1305_open_easy", xoeasy, mlen),
        ] {
            let inlen = if fname.contains("open") { mlen + 16 } else { len };
            let mut o1 = canary(len + 8);
            let mut o2 = canary(len + 8);
            let (x, y) = unsafe {
                (
                    f.0(o1.as_mut_ptr(), m.as_ptr(), inlen as u64, n.as_ptr(),
                        pk.as_ptr(), sk.as_ptr()),
                    f.1(o2.as_mut_ptr(), m.as_ptr(), inlen as u64, n.as_ptr(),
                        pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{fname}(pk={what}) rc"), x, y);
            assert_eq!(x, -1, "{fname}(pk={what}) must reject");
            eq_bytes(&format!("{fname}(pk={what}) out"), &o1, &o2);
            assert_eq!(o1, canary(len + 8), "{fname}(pk={what}) must not write out");
        }

        // --- detached forms: c and mac untouched
        for (fname, f) in [
            ("crypto_box_detached", det),
            ("crypto_box_curve25519xchacha20poly1305_detached", xdet),
        ] {
            let mut c1 = canary(mlen + 8);
            let mut c2 = canary(mlen + 8);
            let mut t1 = canary(16 + 8);
            let mut t2 = canary(16 + 8);
            let (x, y) = unsafe {
                (
                    f.0(c1.as_mut_ptr(), t1.as_mut_ptr(), m.as_ptr(), mlen as u64,
                        n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                    f.1(c2.as_mut_ptr(), t2.as_mut_ptr(), m.as_ptr(), mlen as u64,
                        n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{fname}(pk={what}) rc"), x, y);
            assert_eq!(x, -1);
            eq_bytes(&format!("{fname}(pk={what}) c"), &c1, &c2);
            eq_bytes(&format!("{fname}(pk={what}) mac"), &t1, &t2);
            assert_eq!(c1, canary(mlen + 8), "{fname} must not write c");
            assert_eq!(t1, canary(16 + 8), "{fname} must not write mac");
        }
        for (fname, f) in [
            ("crypto_box_open_detached", odet),
            ("crypto_box_curve25519xchacha20poly1305_open_detached", xodet),
        ] {
            let mac = rng.bytes(16);
            let mut m1 = canary(mlen + 8);
            let mut m2 = canary(mlen + 8);
            let (x, y) = unsafe {
                (
                    f.0(m1.as_mut_ptr(), m.as_ptr(), mac.as_ptr(), mlen as u64,
                        n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                    f.1(m2.as_mut_ptr(), m.as_ptr(), mac.as_ptr(), mlen as u64,
                        n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{fname}(pk={what}) rc"), x, y);
            assert_eq!(x, -1);
            eq_bytes(&format!("{fname}(pk={what}) m"), &m1, &m2);
            assert_eq!(
                m1,
                canary(mlen + 8),
                "{fname}: m must be untouched and NOT zeroed"
            );
        }
    }
}

/// G5-021, G5-022, G5-023, G5-024, G5-041, G5-044 — the `clen < MACBYTES`
/// gate of every `_open_easy` form: -1 immediately, `m` untouched, keys never
/// used (an invalid small-order `pk` still returns -1 from *this* branch).
#[test]
fn open_easy_short_clen() {
    setup();
    let mut rng = Rng::new(0x20200);
    let asym: &[&str] = &[
        "crypto_box_open_easy",
        "crypto_box_curve25519xchacha20poly1305_open_easy",
    ];
    let sym5: &[&str] = &[
        "crypto_box_open_easy_afternm",
        "crypto_box_curve25519xchacha20poly1305_open_easy_afternm",
        "crypto_secretbox_open_easy",
        "crypto_secretbox_xchacha20poly1305_open_easy",
    ];
    let c = rng.bytes(16);
    for clen in 0..16u64 {
        for &name in asym {
            let (f, g) = pair::<Asym6>(name);
            let (pk, sk) = box_kp(&mut rng);
            let n = rng.bytes(24);
            let mut m1 = canary(64);
            let mut m2 = canary(64);
            let (x, y) = unsafe {
                (
                    f(m1.as_mut_ptr(), c.as_ptr(), clen, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                    g(m2.as_mut_ptr(), c.as_ptr(), clen, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(clen={clen}) rc"), x, y);
            assert_eq!(x, -1, "{name}(clen={clen}) must reject");
            eq_bytes(&format!("{name}(clen={clen}) m"), &m1, &m2);
            assert_eq!(m1, canary(64), "{name}: m must be untouched");
            // the gate precedes the key handling: a small-order pk gives the
            // same -1 with the same (absent) side effects
            let bad = small_order_pks()[0].1.clone();
            let mut m3 = canary(64);
            let mut m4 = canary(64);
            let (x, y) = unsafe {
                (
                    f(m3.as_mut_ptr(), c.as_ptr(), clen, n.as_ptr(), bad.as_ptr(), sk.as_ptr()),
                    g(m4.as_mut_ptr(), c.as_ptr(), clen, n.as_ptr(), bad.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(clen={clen},bad pk) rc"), x, y);
            assert_eq!(x, -1);
            eq_bytes(&format!("{name}(clen={clen},bad pk) m"), &m3, &m4);
            assert_eq!(m3, canary(64));
        }
        for &name in sym5 {
            let (f, g) = pair::<Sym5>(name);
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut m1 = canary(64);
            let mut m2 = canary(64);
            let (x, y) = unsafe {
                (
                    f(m1.as_mut_ptr(), c.as_ptr(), clen, n.as_ptr(), k.as_ptr()),
                    g(m2.as_mut_ptr(), c.as_ptr(), clen, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(clen={clen}) rc"), x, y);
            assert_eq!(x, -1, "{name}(clen={clen}) must reject");
            eq_bytes(&format!("{name}(clen={clen}) m"), &m1, &m2);
            assert_eq!(m1, canary(64), "{name}: m must be untouched");
        }
    }
}

/// G5-025, G5-026, G5-042, G5-045, G5-110 — poly1305 verification failure in
/// every `_open_detached` form: -1 with `m` neither written nor zeroed, and
/// the same -1 when `m == NULL`.
#[test]
fn open_detached_mac_failure() {
    setup();
    let mut rng = Rng::new(0x20300);
    for &(name, is_asym) in &[
        ("crypto_box_open_detached", true),
        ("crypto_box_open_detached_afternm", false),
        ("crypto_box_curve25519xchacha20poly1305_open_detached", true),
        ("crypto_box_curve25519xchacha20poly1305_open_detached_afternm", false),
        ("crypto_secretbox_open_detached", false),
        ("crypto_secretbox_xchacha20poly1305_open_detached", false),
    ] {
        // pick the matching encrypt side to build a *valid* box first
        let enc_name = name
            .replace("_open_detached_afternm", "_detached_afternm")
            .replace("_open_detached", "_detached");
        for &clen in &[0usize, 1, 16, 32, 33, 64, 65, 100] {
            let (pk, sk) = box_kp(&mut rng);
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(clen);
            let mut good_c = canary(clen.max(1));
            let mut good_t = canary(16);
            unsafe {
                if is_asym {
                    let e = sym::<Det7>(c_lib(), &enc_name);
                    assert_eq!(
                        e(good_c.as_mut_ptr(), good_t.as_mut_ptr(), m.as_ptr(), clen as u64,
                          n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                        0
                    );
                } else {
                    let e = sym::<Det6>(c_lib(), &enc_name);
                    assert_eq!(
                        e(good_c.as_mut_ptr(), good_t.as_mut_ptr(), m.as_ptr(), clen as u64,
                          n.as_ptr(), k.as_ptr()),
                        0
                    );
                }
            }
            // every documented way of breaking the MAC check
            let mut cases: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
            let mut t = good_t.clone();
            t[0] ^= 1;
            cases.push(("mac[0] bit0".into(), good_c.clone(), t, n.clone(), k.clone()));
            let mut t = good_t.clone();
            t[15] ^= 0x80;
            cases.push(("mac[15] bit7".into(), good_c.clone(), t, n.clone(), k.clone()));
            cases.push(("mac=0".into(), good_c.clone(), vec![0u8; 16], n.clone(), k.clone()));
            cases.push(("mac=ff".into(), good_c.clone(), vec![0xffu8; 16], n.clone(), k.clone()));
            if clen > 0 {
                let mut cc = good_c.clone();
                cc[0] ^= 1;
                cases.push(("c[0] bit0".into(), cc, good_t.clone(), n.clone(), k.clone()));
                let mut cc = good_c.clone();
                cc[clen - 1] ^= 0x80;
                cases.push(("c[last]".into(), cc, good_t.clone(), n.clone(), k.clone()));
            }
            let mut nn = n.clone();
            nn[0] ^= 1;
            cases.push(("n[0] bit0".into(), good_c.clone(), good_t.clone(), nn, k.clone()));
            let mut nn = n.clone();
            nn[23] ^= 1;
            cases.push(("n[23] bit0".into(), good_c.clone(), good_t.clone(), nn, k.clone()));
            let mut kk = k.clone();
            kk[0] ^= 1;
            cases.push(("k[0] bit0".into(), good_c.clone(), good_t.clone(), n.clone(), kk));

            for (label, cbuf, tag, nonce, key) in cases {
                for m_null in [false, true] {
                    let mut m1 = canary(clen.max(1) + 8);
                    let mut m2 = canary(clen.max(1) + 8);
                    let (mp1, mp2) = if m_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (m1.as_mut_ptr(), m2.as_mut_ptr())
                    };
                    let (x, y) = unsafe {
                        if is_asym {
                            let (f, g) = pair::<ODet7>(name);
                            (
                                f(mp1, cbuf.as_ptr(), tag.as_ptr(), clen as u64, nonce.as_ptr(),
                                  pk.as_ptr(), sk.as_ptr()),
                                g(mp2, cbuf.as_ptr(), tag.as_ptr(), clen as u64, nonce.as_ptr(),
                                  pk.as_ptr(), sk.as_ptr()),
                            )
                        } else {
                            let (f, g) = pair::<ODet6>(name);
                            (
                                f(mp1, cbuf.as_ptr(), tag.as_ptr(), clen as u64, nonce.as_ptr(),
                                  key.as_ptr()),
                                g(mp2, cbuf.as_ptr(), tag.as_ptr(), clen as u64, nonce.as_ptr(),
                                  key.as_ptr()),
                            )
                        }
                    };
                    let tagname = format!("{name}({label},clen={clen},m_null={m_null})");
                    eq_i32(&format!("{tagname} rc"), x, y);
                    // for `is_asym` the key comes from the pk/sk pair, so a
                    // tampered `k` is irrelevant there; only skip the assert
                    // when the tampering could not have changed anything.
                    if !(is_asym && label.starts_with("k[")) {
                        assert_eq!(x, -1, "{tagname} must reject");
                    }
                    eq_bytes(&tagname, &m1, &m2);
                    if x == -1 {
                        assert_eq!(
                            m1,
                            canary(clen.max(1) + 8),
                            "{tagname}: m must be neither written nor zeroed"
                        );
                    }
                }
            }

            // G5-110: `m == NULL` with a VALID mac is not a rejection
            let (x, y) = unsafe {
                if is_asym {
                    let (f, g) = pair::<ODet7>(name);
                    (
                        f(ptr::null_mut(), good_c.as_ptr(), good_t.as_ptr(), clen as u64,
                          n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                        g(ptr::null_mut(), good_c.as_ptr(), good_t.as_ptr(), clen as u64,
                          n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                    )
                } else {
                    let (f, g) = pair::<ODet6>(name);
                    (
                        f(ptr::null_mut(), good_c.as_ptr(), good_t.as_ptr(), clen as u64,
                          n.as_ptr(), k.as_ptr()),
                        g(ptr::null_mut(), good_c.as_ptr(), good_t.as_ptr(), clen as u64,
                          n.as_ptr(), k.as_ptr()),
                    )
                }
            };
            eq_i32(&format!("{name} m=NULL valid mac rc"), x, y);
            assert_eq!(x, 0, "{name}: m == NULL with a valid mac must return 0");
        }
    }
}

/// G5-027, G5-028, G5-029, G5-046, G5-047, G5-048 — the NaCl padded API's
/// `mlen < ZEROBYTES` / `clen < ZEROBYTES` gates (32, not BOXZEROBYTES) and
/// its poly1305 mismatch path.
#[test]
fn nacl_padded_api_rejections() {
    setup();
    let mut rng = Rng::new(0x20400);
    let seal_asym: &[&str] = &["crypto_box", "crypto_box_curve25519xsalsa20poly1305"];
    let open_asym: &[&str] = &["crypto_box_open", "crypto_box_curve25519xsalsa20poly1305_open"];
    let seal_sym: &[&str] = &[
        "crypto_box_afternm",
        "crypto_box_curve25519xsalsa20poly1305_afternm",
        "crypto_secretbox",
        "crypto_secretbox_xsalsa20poly1305",
    ];
    let open_sym: &[&str] = &[
        "crypto_box_open_afternm",
        "crypto_box_curve25519xsalsa20poly1305_open_afternm",
        "crypto_secretbox_open",
        "crypto_secretbox_xsalsa20poly1305_open",
    ];

    // --- G5-027 / G5-046: mlen in {0..31}
    for len in 0..32u64 {
        let (pk, sk) = box_kp(&mut rng);
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = rng.bytes(32);
        for &name in seal_asym {
            let (f, g) = pair::<Asym6>(name);
            let mut c1 = canary(64);
            let mut c2 = canary(64);
            let (x, y) = unsafe {
                (
                    f(c1.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                    g(c2.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(mlen={len}) rc"), x, y);
            assert_eq!(x, -1, "{name}(mlen={len}) must reject");
            eq_bytes(&format!("{name}(mlen={len}) c"), &c1, &c2);
            assert_eq!(c1, canary(64), "{name}: c must be untouched");
        }
        for &name in seal_sym {
            let (f, g) = pair::<Sym5>(name);
            let mut c1 = canary(64);
            let mut c2 = canary(64);
            let (x, y) = unsafe {
                (
                    f(c1.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), k.as_ptr()),
                    g(c2.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(mlen={len}) rc"), x, y);
            assert_eq!(x, -1, "{name}(mlen={len}) must reject");
            eq_bytes(&format!("{name}(mlen={len}) c"), &c1, &c2);
            assert_eq!(c1, canary(64), "{name}: c must be untouched");
        }
        // --- G5-028 / G5-047: clen in {0..31} (the gate is 32, NOT 16)
        for &name in open_asym {
            let (f, g) = pair::<Asym6>(name);
            let mut m1 = canary(64);
            let mut m2 = canary(64);
            let (x, y) = unsafe {
                (
                    f(m1.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                    g(m2.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(clen={len}) rc"), x, y);
            assert_eq!(x, -1, "{name}(clen={len}) must reject");
            eq_bytes(&format!("{name}(clen={len}) m"), &m1, &m2);
            assert_eq!(m1, canary(64), "{name}: m must be untouched");
        }
        for &name in open_sym {
            let (f, g) = pair::<Sym5>(name);
            let mut m1 = canary(64);
            let mut m2 = canary(64);
            let (x, y) = unsafe {
                (
                    f(m1.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), k.as_ptr()),
                    g(m2.as_mut_ptr(), m.as_ptr(), len, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32(&format!("{name}(clen={len}) rc"), x, y);
            assert_eq!(x, -1, "{name}(clen={len}) must reject");
            eq_bytes(&format!("{name}(clen={len}) m"), &m1, &m2);
            assert_eq!(m1, canary(64), "{name}: m must be untouched");
        }
    }

    // --- G5-029 / G5-048: tag mismatch, and a box whose m[0..32] was not zero
    for &clen in &[32usize, 33, 48, 64, 132] {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut m = vec![0u8; 32];
        m.extend_from_slice(&rng.bytes(clen - 32));
        let seal = sym::<Sym5>(c_lib(), "crypto_secretbox");
        let mut good = canary(clen);
        unsafe {
            assert_eq!(seal(good.as_mut_ptr(), m.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()), 0)
        };
        let mut cases: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
        let mut v = good.clone();
        v[16] ^= 1;
        cases.push(("tag[0]".into(), v, n.clone(), k.clone()));
        let mut v = good.clone();
        v[31] ^= 0x80;
        cases.push(("tag[15]".into(), v, n.clone(), k.clone()));
        if clen > 32 {
            let mut v = good.clone();
            v[32] ^= 1;
            cases.push(("c[32]".into(), v, n.clone(), k.clone()));
            let mut v = good.clone();
            v[clen - 1] ^= 1;
            cases.push(("c[last]".into(), v, n.clone(), k.clone()));
        }
        let mut nn = n.clone();
        nn[0] ^= 1;
        cases.push(("wrong n".into(), good.clone(), nn, k.clone()));
        let mut kk = k.clone();
        kk[0] ^= 1;
        cases.push(("wrong k".into(), good.clone(), n.clone(), kk));
        // a "box" built from a plaintext whose 32-byte prefix was NOT zero, so
        // c[0..32] is not the raw keystream
        {
            let mut m2 = rng.bytes(clen);
            m2[0] = 1;
            let mut v = canary(clen);
            unsafe {
                assert_eq!(
                    seal(v.as_mut_ptr(), m2.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()),
                    0
                )
            };
            // crypto_secretbox zeroes c[0..16] itself, so re-inject the
            // non-zero prefix that a hand-rolled caller would produce
            v[0] = 0xff;
            cases.push(("non-zero c[0..16]".into(), v, n.clone(), k.clone()));
        }
        for (label, cbuf, nonce, key) in cases {
            for &name in open_sym {
                let (f, g) = pair::<Sym5>(name);
                let mut m1 = canary(clen + 8);
                let mut m2 = canary(clen + 8);
                let (x, y) = unsafe {
                    (
                        f(m1.as_mut_ptr(), cbuf.as_ptr(), clen as u64, nonce.as_ptr(), key.as_ptr()),
                        g(m2.as_mut_ptr(), cbuf.as_ptr(), clen as u64, nonce.as_ptr(), key.as_ptr()),
                    )
                };
                let tag = format!("{name}({label},clen={clen})");
                eq_i32(&format!("{tag} rc"), x, y);
                eq_bytes(&tag, &m1, &m2);
                if label != "non-zero c[0..16]" {
                    assert_eq!(x, -1, "{tag} must reject");
                    assert_eq!(
                        m1,
                        canary(clen + 8),
                        "{tag}: m must be untouched and NOT zeroed"
                    );
                }
            }
        }
    }
}

/// G5-032, G5-033, G5-034, G5-035, G5-038, G5-039 — sealed-box rejections for
/// both primitives, including the fact that `crypto_box_seal` overwrites
/// `c[0..32]` with the ephemeral pk *even when it returns -1*.
#[test]
fn seal_rejections() {
    setup();
    let mut rng = Rng::new(0x20500);
    for prim in [
        "crypto_box",
        "crypto_box_curve25519xchacha20poly1305",
    ] {
        let seal = pair::<Seal>(&format!("{prim}_seal"));
        let open = pair::<SealOpen>(&format!("{prim}_seal_open"));

        // --- G5-032: small-order recipient pk -> -1, but c[0..32] == epk
        for (what, pk) in small_order_pks() {
            for &mlen in &[0usize, 1, 100] {
                let m = rng.bytes(mlen);
                let s = 0x2055 + mlen as u64;
                let mut c1 = canary(mlen + 48 + 8);
                let mut c2 = canary(mlen + 48 + 8);
                let (x, y) = {
                    let _g = RNG_LOCK.lock().unwrap();
                    reset_rngs(s);
                    let x = unsafe {
                        seal.0(c1.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr())
                    };
                    reset_rngs(s);
                    let y = unsafe {
                        seal.1(c2.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr())
                    };
                    (x, y)
                };
                eq_i32(&format!("{prim}_seal(pk={what}) rc"), x, y);
                assert_eq!(x, -1, "{prim}_seal(pk={what}) must reject");
                eq_bytes(&format!("{prim}_seal(pk={what}) c"), &c1, &c2);
                // the memcpy of the ephemeral pk runs unconditionally
                assert_ne!(
                    &c1[..32],
                    &[0xA5u8; 32][..],
                    "{prim}_seal must still write the ephemeral pk into c[0..32]"
                );
                assert_eq!(
                    &c1[32..],
                    &vec![0xA5u8; mlen + 16 + 8][..],
                    "{prim}_seal must not write past c[0..32] on failure"
                );
            }
        }

        // --- G5-033 / G5-038: clen < SEALBYTES
        let (pk, sk) = box_kp(&mut rng);
        let cbuf = rng.bytes(48);
        for clen in 0..48u64 {
            let mut m1 = canary(64);
            let mut m2 = canary(64);
            let (x, y) = unsafe {
                (
                    open.0(m1.as_mut_ptr(), cbuf.as_ptr(), clen, pk.as_ptr(), sk.as_ptr()),
                    open.1(m2.as_mut_ptr(), cbuf.as_ptr(), clen, pk.as_ptr(), sk.as_ptr()),
                )
            };
            eq_i32(&format!("{prim}_seal_open(clen={clen}) rc"), x, y);
            assert_eq!(x, -1, "{prim}_seal_open(clen={clen}) must reject");
            eq_bytes(&format!("{prim}_seal_open(clen={clen}) m"), &m1, &m2);
            assert_eq!(m1, canary(64), "{prim}_seal_open: m must be untouched");
        }

        // --- G5-034 / G5-039: small-order embedded epk, and tampering
        for &mlen in &[0usize, 1, 100] {
            let m = rng.bytes(mlen);
            let s = 0x2056 + mlen as u64;
            let mut good = canary(mlen + 48);
            {
                let _g = RNG_LOCK.lock().unwrap();
                reset_rngs(s);
                unsafe {
                    assert_eq!(seal.0(good.as_mut_ptr(), m.as_ptr(), mlen as u64, pk.as_ptr()), 0)
                };
            }
            let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
            for (what, sopk) in small_order_pks() {
                let mut v = good.clone();
                v[..32].copy_from_slice(&sopk);
                cases.push((format!("epk={what}"), v));
            }
            let mut v = good.clone();
            v[0] ^= 1;
            cases.push(("epk[0] bit0".into(), v));
            let mut v = good.clone();
            v[32] ^= 1;
            cases.push(("mac[0] bit0".into(), v));
            let mut v = good.clone();
            v[47] ^= 0x80;
            cases.push(("mac[15] bit7".into(), v));
            if mlen > 0 {
                let mut v = good.clone();
                v[48] ^= 1;
                cases.push(("body[0]".into(), v));
                let mut v = good.clone();
                v[mlen + 47] ^= 1;
                cases.push(("body[last]".into(), v));
            }
            for (label, bad) in cases {
                let mut m1 = canary(mlen.max(1) + 8);
                let mut m2 = canary(mlen.max(1) + 8);
                let (x, y) = unsafe {
                    (
                        open.0(m1.as_mut_ptr(), bad.as_ptr(), (mlen + 48) as u64,
                               pk.as_ptr(), sk.as_ptr()),
                        open.1(m2.as_mut_ptr(), bad.as_ptr(), (mlen + 48) as u64,
                               pk.as_ptr(), sk.as_ptr()),
                    )
                };
                let tag = format!("{prim}_seal_open({label},mlen={mlen})");
                eq_i32(&format!("{tag} rc"), x, y);
                assert_eq!(x, -1, "{tag} must reject");
                eq_bytes(&tag, &m1, &m2);
                assert_eq!(
                    m1,
                    canary(mlen.max(1) + 8),
                    "{tag}: m must be untouched and NOT zeroed"
                );
            }
            // wrong recipient key pair
            let (pk2, sk2) = box_kp(&mut rng);
            for (label, p, s2) in [
                ("wrong pk", pk2.clone(), sk.clone()),
                ("wrong sk", pk.clone(), sk2.clone()),
                ("wrong pair", pk2.clone(), sk2.clone()),
            ] {
                let mut m1 = canary(mlen.max(1) + 8);
                let mut m2 = canary(mlen.max(1) + 8);
                let (x, y) = unsafe {
                    (
                        open.0(m1.as_mut_ptr(), good.as_ptr(), (mlen + 48) as u64,
                               p.as_ptr(), s2.as_ptr()),
                        open.1(m2.as_mut_ptr(), good.as_ptr(), (mlen + 48) as u64,
                               p.as_ptr(), s2.as_ptr()),
                    )
                };
                let tag = format!("{prim}_seal_open({label},mlen={mlen})");
                eq_i32(&format!("{tag} rc"), x, y);
                assert_eq!(x, -1, "{tag} must reject");
                eq_bytes(&tag, &m1, &m2);
                assert_eq!(m1, canary(mlen.max(1) + 8));
            }
        }
    }
}

// ===========================================================================
// crypto_secretstream
// ===========================================================================

const SS: &str = "crypto_secretstream_xchacha20poly1305";

fn ss_state(hdr: &[u8], k: &[u8], which: usize) -> State {
    let f = if which == 0 {
        sym::<SsInitPull>(c_lib(), &format!("{SS}_init_pull"))
    } else {
        sym::<SsInitPull>(r_lib(), &format!("{SS}_init_pull"))
    };
    let mut st = State::for_sym(&format!("{SS}_statebytes"));
    unsafe { assert_eq!(f(st.as_mut_ptr(), hdr.as_ptr(), k.as_ptr()), 0) };
    st
}

/// G5-050 — `_pull` with `inlen < ABYTES`: -1, but `*mlen_p = 0` and
/// `*tag_p = 0xff` have already been written, `m` untouched, state unchanged.
#[test]
fn secretstream_pull_short_inlen() {
    setup();
    let mut rng = Rng::new(0x21000);
    let pull = pair::<SsPull>(&format!("{SS}_pull"));
    let inbuf = rng.bytes(17);
    for inlen in 0..17u64 {
        for &(mlen_null, tag_null) in &[(false, false), (false, true), (true, false), (true, true)] {
            let k = rng.bytes(32);
            let hdr = rng.bytes(24);
            let ad = rng.bytes(16);
            let mut outs: Vec<(i32, u64, u8, Vec<u8>, Vec<u8>)> = Vec::new();
            for which in 0..2usize {
                let mut st = ss_state(&hdr, &k, which);
                let before = st.bytes().to_vec();
                let f = if which == 0 { pull.0 } else { pull.1 };
                let mut m = canary(64);
                let mut o = 0xDEAD_BEEFu64;
                let mut t = 0x5Au8;
                let op = if mlen_null { ptr::null_mut() } else { &raw mut o };
                let tp = if tag_null { ptr::null_mut() } else { &raw mut t };
                let rc = unsafe {
                    f(st.as_mut_ptr(), m.as_mut_ptr(), op, tp, inbuf.as_ptr(), inlen,
                      ad.as_ptr(), 16)
                };
                assert_eq!(
                    st.bytes(),
                    &before[..],
                    "pull(inlen={inlen}) must not advance the state"
                );
                outs.push((rc, o, t, m, st.bytes().to_vec()));
            }
            let tag = format!("pull(inlen={inlen},mlen_null={mlen_null},tag_null={tag_null})");
            eq_i32(&format!("{tag} rc"), outs[0].0, outs[1].0);
            assert_eq!(outs[0].0, -1, "{tag} must reject");
            eq_usize(&format!("{tag} *mlen_p"), outs[0].1 as usize, outs[1].1 as usize);
            assert_eq!(outs[0].2, outs[1].2, "{tag} *tag_p");
            if mlen_null {
                assert_eq!(outs[0].1, 0xDEAD_BEEF, "{tag}: mlen_p == NULL must not write");
            } else {
                assert_eq!(outs[0].1, 0, "{tag}: *mlen_p must be set to 0 before the gate");
            }
            if tag_null {
                assert_eq!(outs[0].2, 0x5A, "{tag}: tag_p == NULL must not write");
            } else {
                assert_eq!(outs[0].2, 0xff, "{tag}: *tag_p must be set to 0xff before the gate");
            }
            eq_bytes(&format!("{tag} m"), &outs[0].3, &outs[1].3);
            assert_eq!(outs[0].3, canary(64), "{tag}: m must be untouched");
            eq_bytes(&format!("{tag} state"), &outs[0].4, &outs[1].4);
        }
    }
}

/// G5-052, G5-053, G5-054, G5-055 — every way of failing the stored-MAC check:
/// tampered tag byte / ciphertext / MAC, wrong or wrong-length AD, replay and
/// out-of-order pulls, a mismatched header and a mismatched key. In every case
/// -1 with `*mlen_p == 0`, `*tag_p == 0xff`, `m` untouched and the state
/// neither advanced nor rekeyed.
#[test]
fn secretstream_pull_mac_failures() {
    setup();
    let mut rng = Rng::new(0x21100);
    let push = pair::<SsPush>(&format!("{SS}_push"));
    let pull = pair::<SsPull>(&format!("{SS}_pull"));

    for &mlen in &[0usize, 1, 16, 64, 100] {
        for &adlen in &[0usize, 16] {
            let k = rng.bytes(32);
            let hdr = rng.bytes(24);
            let ad = rng.bytes(adlen.max(1));
            let m0 = rng.bytes(mlen);
            let m1 = rng.bytes(mlen);
            // build a two-message stream with the C library
            let mut st = ss_state(&hdr, &k, 0);
            let mut c0 = canary(mlen + 17);
            let mut c1 = canary(mlen + 17);
            unsafe {
                assert_eq!(
                    push.0(st.as_mut_ptr(), c0.as_mut_ptr(), ptr::null_mut(), m0.as_ptr(),
                           mlen as u64, ad.as_ptr(), adlen as u64, 0),
                    0
                );
                assert_eq!(
                    push.0(st.as_mut_ptr(), c1.as_mut_ptr(), ptr::null_mut(), m1.as_ptr(),
                           mlen as u64, ad.as_ptr(), adlen as u64, 3),
                    0
                );
            }

            // (label, ciphertext, ad, adlen, key, header, pull-msg1-first)
            let mut cases: Vec<(String, Vec<u8>, Vec<u8>, usize, Vec<u8>, Vec<u8>, bool)> =
                Vec::new();
            let mut v = c0.clone();
            v[0] ^= 1;
            cases.push(("in[0] tag byte".into(), v, ad.clone(), adlen, k.clone(), hdr.clone(), false));
            let mut v = c0.clone();
            let last = v.len() - 1;
            v[last] ^= 0x80;
            cases.push(("mac[last]".into(), v, ad.clone(), adlen, k.clone(), hdr.clone(), false));
            let mut v = c0.clone();
            v[mlen + 1] ^= 1;
            cases.push(("mac[0]".into(), v, ad.clone(), adlen, k.clone(), hdr.clone(), false));
            if mlen > 0 {
                let mut v = c0.clone();
                v[1] ^= 1;
                cases.push(("c[0]".into(), v, ad.clone(), adlen, k.clone(), hdr.clone(), false));
                let mut v = c0.clone();
                v[mlen] ^= 1;
                cases.push(("c[last]".into(), v, ad.clone(), adlen, k.clone(), hdr.clone(), false));
            }
            // G5-053: wrong AD content and wrong AD length
            if adlen > 0 {
                let mut a2 = ad.clone();
                a2[0] ^= 1;
                cases.push(("wrong ad content".into(), c0.clone(), a2, adlen, k.clone(), hdr.clone(), false));
                cases.push(("adlen 16 -> 0".into(), c0.clone(), ad.clone(), 0, k.clone(), hdr.clone(), false));
                cases.push(("adlen 16 -> 15".into(), c0.clone(), ad.clone(), 15, k.clone(), hdr.clone(), false));
            } else {
                cases.push(("adlen 0 -> 16".into(), c0.clone(), rng.bytes(16), 16, k.clone(), hdr.clone(), false));
            }
            // G5-055: wrong key (one bit)
            let mut k2 = k.clone();
            k2[0] ^= 1;
            cases.push(("wrong k".into(), c0.clone(), ad.clone(), adlen, k2, hdr.clone(), false));
            // G5-054: wrong header
            let mut h2 = hdr.clone();
            h2[0] ^= 1;
            cases.push(("wrong header[0]".into(), c0.clone(), ad.clone(), adlen, k.clone(), h2, false));
            let mut h2 = hdr.clone();
            h2[23] ^= 1;
            cases.push(("wrong header[23]".into(), c0.clone(), ad.clone(), adlen, k.clone(), h2, false));
            cases.push(("all-zero header".into(), c0.clone(), ad.clone(), adlen, k.clone(), vec![0u8; 24], false));
            // G5-054: message #2 pulled first (state desynchronisation)
            cases.push(("msg2 before msg1".into(), c1.clone(), ad.clone(), adlen, k.clone(), hdr.clone(), false));
            // G5-054: replay of message #1 after it was already consumed
            cases.push(("replay msg1".into(), c0.clone(), ad.clone(), adlen, k.clone(), hdr.clone(), true));

            for (label, cbuf, adbuf, al, key, header, replay) in cases {
                let mut outs: Vec<(i32, u64, u8, Vec<u8>, Vec<u8>)> = Vec::new();
                for which in 0..2usize {
                    let mut st = ss_state(&header, &key, which);
                    let f = if which == 0 { pull.0 } else { pull.1 };
                    if replay {
                        // consume message 1 first so the state has advanced
                        let mut tmp = canary(mlen.max(1));
                        let rc = unsafe {
                            f(st.as_mut_ptr(), tmp.as_mut_ptr(), ptr::null_mut(),
                              ptr::null_mut(), c0.as_ptr(), c0.len() as u64,
                              adbuf.as_ptr(), al as u64)
                        };
                        assert_eq!(rc, 0, "replay setup must first succeed");
                    }
                    let before = st.bytes().to_vec();
                    let mut m = canary(mlen.max(1) + 8);
                    let mut o = 0xDEADu64;
                    let mut t = 0x5Au8;
                    let rc = unsafe {
                        f(st.as_mut_ptr(), m.as_mut_ptr(), &mut o, &mut t, cbuf.as_ptr(),
                          cbuf.len() as u64, adbuf.as_ptr(), al as u64)
                    };
                    if rc == -1 {
                        assert_eq!(
                            st.bytes(),
                            &before[..],
                            "{label}: a failed pull must not advance or rekey the state"
                        );
                    }
                    outs.push((rc, o, t, m, st.bytes().to_vec()));
                }
                let tag = format!("pull({label},mlen={mlen},adlen={adlen})");
                eq_i32(&format!("{tag} rc"), outs[0].0, outs[1].0);
                assert_eq!(outs[0].0, -1, "{tag} must reject");
                eq_usize(&format!("{tag} *mlen_p"), outs[0].1 as usize, outs[1].1 as usize);
                assert_eq!(outs[0].1, 0, "{tag}: *mlen_p must be 0");
                assert_eq!(outs[0].2, outs[1].2, "{tag} *tag_p");
                assert_eq!(outs[0].2, 0xff, "{tag}: *tag_p must be 0xff");
                eq_bytes(&format!("{tag} m"), &outs[0].3, &outs[1].3);
                assert_eq!(
                    outs[0].3,
                    canary(mlen.max(1) + 8),
                    "{tag}: m must be untouched and NOT zeroed"
                );
                eq_bytes(&format!("{tag} state"), &outs[0].4, &outs[1].4);
            }
        }
    }
}

/// G5-056, G5-057, G5-058, G5-059, G5-060, G5-061, G5-062 — the documented
/// *non*-rejections of the secretstream API: any header is accepted, arbitrary
/// tag bytes are encrypted verbatim and returned verbatim, pushing after
/// `TAG_FINAL` still works, the 32-bit counter wrap triggers an implicit rekey,
/// and a one-sided `_rekey` desynchronises the stream permanently.
#[test]
fn secretstream_non_rejections() {
    setup();
    let mut rng = Rng::new(0x21200);
    let push = pair::<SsPush>(&format!("{SS}_push"));
    let pull = pair::<SsPull>(&format!("{SS}_pull"));
    let rekey = pair::<SsRekey>(&format!("{SS}_rekey"));
    let init_pull = pair::<SsInitPull>(&format!("{SS}_init_pull"));
    let init_push = pair::<SsInitPush>(&format!("{SS}_init_push"));

    // ---- G5-056: `_init_pull` accepts ANY header, including all-zero
    for hdr in [vec![0u8; 24], vec![0xffu8; 24], (0u8..24).collect::<Vec<u8>>(), rng.bytes(24)] {
        for k in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
            let mut a = State::for_sym(&format!("{SS}_statebytes"));
            let mut b = State::for_sym(&format!("{SS}_statebytes"));
            let (x, y) = unsafe {
                (
                    init_pull.0(a.as_mut_ptr(), hdr.as_ptr(), k.as_ptr()),
                    init_pull.1(b.as_mut_ptr(), hdr.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32("init_pull rc", x, y);
            assert_eq!(x, 0, "_init_pull must accept any header");
            eq_bytes("init_pull state", a.bytes(), b.bytes());
        }
    }
    // ---- G5-057: `_init_push` cannot fail
    for i in 0..6u64 {
        let k = rng.bytes(32);
        let mut h1 = canary(24);
        let mut h2 = canary(24);
        let mut a = State::for_sym(&format!("{SS}_statebytes"));
        let mut b = State::for_sym(&format!("{SS}_statebytes"));
        let (x, y) = {
            let _g = RNG_LOCK.lock().unwrap();
            reset_rngs(0x2120 + i);
            let x = unsafe { init_push.0(a.as_mut_ptr(), h1.as_mut_ptr(), k.as_ptr()) };
            reset_rngs(0x2120 + i);
            let y = unsafe { init_push.1(b.as_mut_ptr(), h2.as_mut_ptr(), k.as_ptr()) };
            (x, y)
        };
        eq_i32("init_push rc", x, y);
        assert_eq!(x, 0);
        eq_bytes("init_push header", &h1, &h2);
        eq_bytes("init_push state", a.bytes(), b.bytes());
    }

    // ---- G5-058 / G5-059 / G5-060: arbitrary tag bytes, and pushes after
    // TAG_FINAL. Every tag with bit 1 set rekeys; the tag byte is neither
    // validated on push nor on pull.
    for &tag in &[0u8, 1, 2, 3, 4, 6, 0x7f, 0x80, 0xfe, 0xff] {
        for &mlen in &[0usize, 1, 64] {
            let k = rng.bytes(32);
            let hdr = rng.bytes(24);
            let m = rng.bytes(mlen);
            let mut sent: Vec<Vec<u8>> = Vec::new();
            let mut pst: Vec<State> = (0..2).map(|w| ss_state(&hdr, &k, w)).collect();
            // three consecutive pushes with the same (possibly out-of-range) tag
            for round in 0..3usize {
                let mut outs: Vec<Vec<u8>> = Vec::new();
                for which in 0..2usize {
                    let f = if which == 0 { push.0 } else { push.1 };
                    let mut c = canary(mlen + 17);
                    let mut l = 0u64;
                    let rc = unsafe {
                        f(pst[which].as_mut_ptr(), c.as_mut_ptr(), &mut l, m.as_ptr(),
                          mlen as u64, ptr::null(), 0, tag)
                    };
                    assert_eq!(rc, 0, "push(tag={tag:#04x}) must succeed even after TAG_FINAL");
                    assert_eq!(l as usize, mlen + 17);
                    outs.push(c);
                }
                eq_bytes(&format!("push(tag={tag:#04x},round={round})"), &outs[0], &outs[1]);
                eq_bytes(
                    &format!("push(tag={tag:#04x},round={round}) state"),
                    pst[0].bytes(),
                    pst[1].bytes(),
                );
                sent.push(outs[0].clone());
            }
            // pull them back: the raw tag byte comes out unvalidated
            let mut qst: Vec<State> = (0..2).map(|w| ss_state(&hdr, &k, w)).collect();
            for (round, c) in sent.iter().enumerate() {
                let mut outs: Vec<(i32, u8, Vec<u8>)> = Vec::new();
                for which in 0..2usize {
                    let f = if which == 0 { pull.0 } else { pull.1 };
                    let mut mm = canary(mlen.max(1));
                    let mut t = 0x5Au8;
                    let rc = unsafe {
                        f(qst[which].as_mut_ptr(), mm.as_mut_ptr(), ptr::null_mut(), &mut t,
                          c.as_ptr(), c.len() as u64, ptr::null(), 0)
                    };
                    outs.push((rc, t, mm));
                }
                let what = format!("pull(tag={tag:#04x},round={round})");
                eq_i32(&format!("{what} rc"), outs[0].0, outs[1].0);
                assert_eq!(outs[0].0, 0, "{what} must succeed (tags are never validated)");
                assert_eq!(outs[0].1, outs[1].1, "{what} *tag_p");
                assert_eq!(outs[0].1, tag, "{what}: the raw decrypted tag byte is returned");
                eq_bytes(&what, &outs[0].2, &outs[1].2);
                assert_eq!(&outs[0].2[..mlen], &m[..]);
                eq_bytes(&format!("{what} state"), qst[0].bytes(), qst[1].bytes());
            }
        }
    }

    // ---- G5-061: the 32-bit counter wrap. The state layout is public
    // (`k[32] ‖ nonce[12] ‖ _pad[8]`, `nonce[0..4]` is the counter), so the
    // wrap can be reached directly by pre-setting the counter to 0xffffffff.
    for &mlen in &[0usize, 64] {
        let k = rng.bytes(32);
        let hdr = rng.bytes(24);
        let m = rng.bytes(mlen);
        let mut pst: Vec<State> = (0..2).map(|w| ss_state(&hdr, &k, w)).collect();
        let mut qst: Vec<State> = (0..2).map(|w| ss_state(&hdr, &k, w)).collect();
        for st in pst.iter_mut().chain(qst.iter_mut()) {
            unsafe { ptr::copy_nonoverlapping([0xffu8; 4].as_ptr(), st.as_mut_ptr().add(32), 4) };
        }
        let before = pst[0].bytes().to_vec();
        let mut outs: Vec<Vec<u8>> = Vec::new();
        for which in 0..2usize {
            let f = if which == 0 { push.0 } else { push.1 };
            let mut c = canary(mlen + 17);
            let rc = unsafe {
                f(pst[which].as_mut_ptr(), c.as_mut_ptr(), ptr::null_mut(), m.as_ptr(),
                  mlen as u64, ptr::null(), 0, 0)
            };
            assert_eq!(rc, 0, "the counter wrap is not an error");
            outs.push(c);
        }
        eq_bytes(&format!("push at counter wrap(mlen={mlen})"), &outs[0], &outs[1]);
        eq_bytes("push state at counter wrap", pst[0].bytes(), pst[1].bytes());
        // the wrap forced an implicit rekey: new key, counter back to 1
        assert_eq!(&pst[0].bytes()[32..36], &[1u8, 0, 0, 0], "wrap must reset the counter");
        assert_ne!(&pst[0].bytes()[..32], &before[..32], "wrap must rekey");
        // the pull side wraps in lock step, so the message still decrypts
        for which in 0..2usize {
            let f = if which == 0 { pull.0 } else { pull.1 };
            let mut p = canary(mlen.max(1));
            let rc = unsafe {
                f(qst[which].as_mut_ptr(), p.as_mut_ptr(), ptr::null_mut(), ptr::null_mut(),
                  outs[0].as_ptr(), (mlen + 17) as u64, ptr::null(), 0)
            };
            assert_eq!(rc, 0, "the pull side must wrap identically");
            assert_eq!(&p[..mlen], &m[..]);
        }
        eq_bytes("pull state at counter wrap", qst[0].bytes(), qst[1].bytes());
        eq_bytes("push == pull state after wrap", pst[0].bytes(), qst[0].bytes());
    }

    // ---- G5-062: a one-sided `_rekey` permanently desynchronises the stream
    for &mlen in &[0usize, 64] {
        let k = rng.bytes(32);
        let hdr = rng.bytes(24);
        let m = rng.bytes(mlen);
        let mut pst: Vec<State> = (0..2).map(|w| ss_state(&hdr, &k, w)).collect();
        let mut qst: Vec<State> = (0..2).map(|w| ss_state(&hdr, &k, w)).collect();
        // rekey only the push side
        for which in 0..2usize {
            let f = if which == 0 { rekey.0 } else { rekey.1 };
            unsafe { f(pst[which].as_mut_ptr()) };
        }
        eq_bytes("one-sided _rekey state", pst[0].bytes(), pst[1].bytes());
        let mut sent: Vec<Vec<u8>> = Vec::new();
        for which in 0..2usize {
            let f = if which == 0 { push.0 } else { push.1 };
            let mut c = canary(mlen + 17);
            assert_eq!(
                unsafe {
                    f(pst[which].as_mut_ptr(), c.as_mut_ptr(), ptr::null_mut(), m.as_ptr(),
                      mlen as u64, ptr::null(), 0, 0)
                },
                0
            );
            sent.push(c);
        }
        eq_bytes("post-rekey push", &sent[0], &sent[1]);
        // three pulls in a row on the un-rekeyed state must all fail
        for round in 0..3usize {
            let mut outs: Vec<(i32, u64, u8, Vec<u8>)> = Vec::new();
            for which in 0..2usize {
                let f = if which == 0 { pull.0 } else { pull.1 };
                let mut p = canary(mlen.max(1) + 8);
                let mut o = 0xDEADu64;
                let mut t = 0x5Au8;
                let rc = unsafe {
                    f(qst[which].as_mut_ptr(), p.as_mut_ptr(), &mut o, &mut t,
                      sent[0].as_ptr(), (mlen + 17) as u64, ptr::null(), 0)
                };
                outs.push((rc, o, t, p));
            }
            eq_i32(&format!("desynced pull rc(round={round})"), outs[0].0, outs[1].0);
            assert_eq!(outs[0].0, -1, "a one-sided rekey must desynchronise the stream");
            eq_usize("desynced pull *mlen_p", outs[0].1 as usize, outs[1].1 as usize);
            assert_eq!(outs[0].1, 0);
            assert_eq!(outs[0].2, 0xff);
            eq_bytes("desynced pull m", &outs[0].3, &outs[1].3);
            assert_eq!(outs[0].3, canary(mlen.max(1) + 8));
            eq_bytes("desynced pull state", qst[0].bytes(), qst[1].bytes());
        }
    }
}

// ===========================================================================
// crypto_kx
// ===========================================================================

/// G5-064, G5-067, G5-069 — the session-key functions reject small-order peer
/// keys with -1 and leave both `rx` and `tx` unwritten; `_keypair` and
/// `_seed_keypair` have no failure path at all.
#[test]
fn kx_rejections() {
    setup();
    let mut rng = Rng::new(0x22000);
    let cli = pair::<KxSession>("crypto_kx_client_session_keys");
    let srv = pair::<KxSession>("crypto_kx_server_session_keys");
    let seed_kp = pair::<SeedKeypair>("crypto_kx_seed_keypair");
    let kp = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> i32>("crypto_kx_keypair");

    for (what, peer) in small_order_pks() {
        for _ in 0..3 {
            let seed = rng.bytes(32);
            let mut pk = [0u8; 32];
            let mut sk = [0u8; 32];
            unsafe { assert_eq!(seed_kp.0(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0) };
            for (name, f) in [
                ("crypto_kx_client_session_keys", cli),
                ("crypto_kx_server_session_keys", srv),
            ] {
                let mut rx1 = canary(32 + 8);
                let mut rx2 = canary(32 + 8);
                let mut tx1 = canary(32 + 8);
                let mut tx2 = canary(32 + 8);
                let (x, y) = unsafe {
                    (
                        f.0(rx1.as_mut_ptr(), tx1.as_mut_ptr(), pk.as_ptr(), sk.as_ptr(),
                            peer.as_ptr()),
                        f.1(rx2.as_mut_ptr(), tx2.as_mut_ptr(), pk.as_ptr(), sk.as_ptr(),
                            peer.as_ptr()),
                    )
                };
                eq_i32(&format!("{name}(peer={what}) rc"), x, y);
                assert_eq!(x, -1, "{name}(peer={what}) must reject");
                eq_bytes(&format!("{name}(peer={what}) rx"), &rx1, &rx2);
                eq_bytes(&format!("{name}(peer={what}) tx"), &tx1, &tx2);
                assert_eq!(rx1, canary(32 + 8), "{name}: rx must be unwritten");
                assert_eq!(tx1, canary(32 + 8), "{name}: tx must be unwritten");

                // rx == NULL (retargeted to tx) still rejects without writing
                let mut only = canary(32 + 8);
                let rc = unsafe {
                    f.0(ptr::null_mut(), only.as_mut_ptr(), pk.as_ptr(), sk.as_ptr(),
                        peer.as_ptr())
                };
                assert_eq!(rc, -1);
                let mut only2 = canary(32 + 8);
                let rc = unsafe {
                    f.1(ptr::null_mut(), only2.as_mut_ptr(), pk.as_ptr(), sk.as_ptr(),
                        peer.as_ptr())
                };
                assert_eq!(rc, -1);
                eq_bytes(&format!("{name}(peer={what}) rx=NULL out"), &only, &only2);
                assert_eq!(only, canary(32 + 8));
            }
        }
    }

    // ---- G5-069: no failure path in either key-generation function
    for seed in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
        let mut pk1 = canary(32);
        let mut sk1 = canary(32);
        let mut pk2 = canary(32);
        let mut sk2 = canary(32);
        let (x, y) = unsafe {
            (
                seed_kp.0(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr()),
                seed_kp.1(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq_i32("crypto_kx_seed_keypair rc", x, y);
        assert_eq!(x, 0, "crypto_kx_seed_keypair must always succeed");
        eq_bytes("crypto_kx_seed_keypair pk", &pk1, &pk2);
        eq_bytes("crypto_kx_seed_keypair sk", &sk1, &sk2);
    }
    for i in 0..6u64 {
        let mut pk1 = canary(32);
        let mut sk1 = canary(32);
        let mut pk2 = canary(32);
        let mut sk2 = canary(32);
        let (x, y) = {
            let _g = RNG_LOCK.lock().unwrap();
            reset_rngs(0x2200 + i);
            let x = unsafe { kp.0(pk1.as_mut_ptr(), sk1.as_mut_ptr()) };
            reset_rngs(0x2200 + i);
            let y = unsafe { kp.1(pk2.as_mut_ptr(), sk2.as_mut_ptr()) };
            (x, y)
        };
        eq_i32("crypto_kx_keypair rc", x, y);
        assert_eq!(x, 0, "crypto_kx_keypair must always succeed");
        eq_bytes("crypto_kx_keypair pk", &pk1, &pk2);
        eq_bytes("crypto_kx_keypair sk", &sk1, &sk2);
    }
}

// ===========================================================================
// crypto_sign — ed25519 verification rejections
// ===========================================================================

/// Every `_verify_detached` flavour, in both libraries.
fn verify_all(
    sig: &[u8],
    m: &[u8],
    pk: &[u8],
    label: &str,
    expect: i32,
) {
    for name in [
        "crypto_sign_verify_detached",
        "crypto_sign_ed25519_verify_detached",
        "_crypto_sign_ed25519_verify_detached",
    ] {
        if name.starts_with('_') {
            // The internal entry point takes an extra `prehashed` flag. With
            // `prehashed = 1` the dom2 prefix is hashed in, so a signature made
            // by the plain API never verifies — only the C/Rust agreement is
            // asserted there, not the expected value.
            type V5 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8, i32) -> i32;
            let (f, g) = pair::<V5>(name);
            for ph in [0i32, 1] {
                let (x, y) = unsafe {
                    (
                        f(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr(), ph),
                        g(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr(), ph),
                    )
                };
                eq_i32(&format!("{name}({label},prehashed={ph}) rc"), x, y);
                if ph == 0 {
                    assert_eq!(x, expect, "{name}({label},prehashed=0)");
                } else {
                    assert_eq!(x, -1, "{name}({label},prehashed=1) cannot verify");
                }
            }
        } else {
            let (f, g) = pair::<Verify4>(name);
            let (x, y) = unsafe {
                (
                    f(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()),
                    g(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()),
                )
            };
            eq_i32(&format!("{name}({label}) rc"), x, y);
            assert_eq!(x, expect, "{name}({label})");
        }
    }
}

/// G5-070, G5-071, G5-072, G5-073, G5-074, G5-075, G5-076, G5-077, G5-078 —
/// all seven `_verify_detached` rejection branches, plus the two build-state
/// notes (`ED25519_COMPAT` / `ED25519_NONDETERMINISTIC` are undefined).
#[test]
fn sign_verify_detached_rejections() {
    setup();
    let mut rng = Rng::new(0x23000);
    let det = sym::<Sign5>(c_lib(), "crypto_sign_detached");

    for &mlen in &[0usize, 1, 32, 64, 100] {
        let (pk, sk) = sign_kp(&mut rng);
        let m = rng.bytes(mlen);
        let mut sig = [0u8; 64];
        unsafe {
            assert_eq!(det(sig.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr()), 0)
        };
        verify_all(&sig, &m, &pk, "valid", 0);

        // ---- G5-070: `ED25519_COMPAT` is NOT defined, so there is no
        // `sig[63] & 224` test. Every *canonical* S has S < L and L's top byte
        // is 0x10, so a genuine signature can never have bits 5..7 of sig[63]
        // set — the legacy mask would be unobservable even if compiled in. What
        // IS observable is the `#else` pair of tests, exercised below.
        assert!(
            sig[63] <= 0x10,
            "a canonical S must have sig[63] <= 0x10 (got {:#04x})",
            sig[63]
        );

        // ---- G5-071: signing is deterministic (no `randombytes`)
        let mut again = [0u8; 64];
        unsafe {
            det(again.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr())
        };
        assert_eq!(sig, again, "signing must be deterministic");

        // ---- G5-072: non-canonical S with a non-zero top nibble
        for (label, s) in noncanonical_scalars() {
            let mut bad = sig;
            bad[32..].copy_from_slice(&s);
            assert_ne!(bad[63] & 240, 0, "{label}: the guard needs sig[63] & 240 != 0");
            verify_all(&bad, &m, &pk, &format!("S non-canonical: {label}"), -1);
        }
        // S = L-1 IS canonical, so it passes *that* test and is rejected later
        {
            let mut lm1 = hx(L_HEX);
            lm1[0] -= 1; // 0xed -> 0xec
            let mut bad = sig;
            bad[32..].copy_from_slice(&lm1);
            verify_all(&bad, &m, &pk, "S = L-1 (canonical)", -1);
        }

        // ---- G5-073: non-canonical pk
        for (label, bpk) in noncanonical_ed_pks() {
            verify_all(&sig, &m, &bpk, &format!("pk non-canonical: {label}"), -1);
        }
        // ---- G5-074: pk decompression failure
        for (label, bpk) in undecodable_ed_pks() {
            verify_all(&sig, &m, &bpk, &format!("pk undecodable: {label}"), -1);
        }
        // ---- G5-075: small-order pk
        for (label, bpk) in small_order_ed_pks() {
            verify_all(&sig, &m, &bpk, &format!("pk small order: {label}"), -1);
        }
        // ---- G5-076: R decompression failure (R skips the canonicality test)
        for (label, r) in undecodable_ed_pks() {
            let mut bad = sig;
            bad[..32].copy_from_slice(&r);
            verify_all(&bad, &m, &pk, &format!("R undecodable: {label}"), -1);
        }
        // ---- G5-077: small-order R, including the non-canonical y>=p forms
        // which DO reach this branch (unlike for `pk`)
        let mut small_r = small_order_ed_pks();
        small_r.push(("edff..7f".into(), rep(&[0xed], 0xff, 32, Some(0x7f))));
        small_r.push(("eeff..7f".into(), rep(&[0xee], 0xff, 32, Some(0x7f))));
        for (label, r) in small_r {
            let mut bad = sig;
            bad[..32].copy_from_slice(&r);
            verify_all(&bad, &m, &pk, &format!("R small order: {label}"), -1);
        }

        // ---- G5-078: the cofactored equation fails
        // (a) a bit flipped in the message
        if mlen > 0 {
            for idx in [0usize, mlen / 2, mlen - 1] {
                let mut m2 = m.clone();
                m2[idx] ^= 1;
                verify_all(&sig, &m2, &pk, &format!("m[{idx}] flipped"), -1);
            }
        }
        // (b) a bit flipped in S (bytes 32..63, so S stays canonical)
        for idx in 32..63usize {
            if idx % 7 != 0 {
                continue;
            }
            let mut bad = sig;
            bad[idx] ^= 1;
            verify_all(&bad, &m, &pk, &format!("S[{}] flipped", idx - 32), -1);
        }
        // (c) mlen off by one, in both directions
        {
            let mut longer = m.clone();
            longer.push(0);
            verify_all(&sig, &longer, &pk, "message extended", -1);
            if mlen > 0 {
                verify_all(&sig, &m[..mlen - 1], &pk, "message truncated", -1);
            }
        }
        // (d) a different (valid) public key
        {
            let (pk2, _) = sign_kp(&mut rng);
            verify_all(&sig, &m, &pk2, "wrong pk", -1);
        }
        // (e) a signature transplanted from a different message
        {
            let m2 = rng.bytes(mlen + 3);
            let mut sig2 = [0u8; 64];
            unsafe {
                det(sig2.as_mut_ptr(), ptr::null_mut(), m2.as_ptr(), m2.len() as u64, sk.as_ptr())
            };
            verify_all(&sig2, &m, &pk, "transplanted signature", -1);
        }
        // (f) a bit flipped in R (still a valid point, wrong value)
        {
            let (pk2, sk2) = sign_kp(&mut rng);
            let _ = pk2;
            let mut sig2 = [0u8; 64];
            unsafe {
                det(sig2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk2.as_ptr())
            };
            let mut bad = sig;
            bad[..32].copy_from_slice(&sig2[..32]);
            verify_all(&bad, &m, &pk, "R from another key", -1);
        }
    }
}

/// G5-079, G5-081 — `crypto_sign_open`'s two failure paths, which differ in
/// exactly one respect: the `smlen < 64` path does NOT touch `m`, while the
/// verification-failure path zeroes `smlen - 64` bytes of it.
#[test]
fn sign_open_failure_side_effects() {
    setup();
    let mut rng = Rng::new(0x23100);
    let det = sym::<Sign5>(c_lib(), "crypto_sign_detached");
    for name in ["crypto_sign_open", "crypto_sign_ed25519_open"] {
        let (f, g) = pair::<Sign5>(name);

        // ---- G5-079: smlen in {0..63}
        let sm = rng.bytes(64);
        for smlen in 0..64u64 {
            for &(m_null, len_null) in
                &[(false, false), (false, true), (true, false), (true, true)]
            {
                let (pk, _) = sign_kp(&mut rng);
                let mut m1 = canary(128);
                let mut m2 = canary(128);
                let mut o1 = 0xDEAD_BEEFu64;
                let mut o2 = 0xDEAD_BEEFu64;
                let (mp1, mp2) = if m_null {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (m1.as_mut_ptr(), m2.as_mut_ptr())
                };
                let (lp1, lp2) = if len_null {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (&raw mut o1, &raw mut o2)
                };
                let (x, y) = unsafe {
                    (
                        f(mp1, lp1, sm.as_ptr(), smlen, pk.as_ptr()),
                        g(mp2, lp2, sm.as_ptr(), smlen, pk.as_ptr()),
                    )
                };
                let tag = format!("{name}(smlen={smlen},m_null={m_null},len_null={len_null})");
                eq_i32(&format!("{tag} rc"), x, y);
                assert_eq!(x, -1, "{tag} must reject");
                eq_usize(&format!("{tag} *mlen_p"), o1 as usize, o2 as usize);
                if len_null {
                    assert_eq!(o1, 0xDEAD_BEEF, "{tag}: mlen_p == NULL must not write");
                } else {
                    assert_eq!(o1, 0, "{tag}: *mlen_p must be 0");
                }
                eq_bytes(&format!("{tag} m"), &m1, &m2);
                assert_eq!(
                    m1,
                    canary(128),
                    "{tag}: the smlen < 64 path must NOT zero m"
                );
            }
        }

        // ---- G5-081: smlen >= 64 but the signature does not verify
        for &mlen in &[0usize, 1, 32, 64, 100] {
            let (pk, sk) = sign_kp(&mut rng);
            let (pk2, _) = sign_kp(&mut rng);
            let m = rng.bytes(mlen);
            let mut good = canary(mlen + 64);
            unsafe {
                assert_eq!(
                    det(good.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, sk.as_ptr()),
                    0
                )
            };
            good[64..].copy_from_slice(&m);
            let mut cases: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
            let mut v = good.clone();
            v[0] ^= 1;
            cases.push(("R flipped".into(), v, pk.clone()));
            let mut v = good.clone();
            v[40] ^= 1;
            cases.push(("S flipped".into(), v, pk.clone()));
            cases.push(("wrong pk".into(), good.clone(), pk2.clone()));
            cases.push((
                "non-canonical S = L".into(),
                {
                    let mut v = good.clone();
                    v[32..64].copy_from_slice(&hx(L_HEX));
                    v
                },
                pk.clone(),
            ));
            cases.push((
                "small-order pk".into(),
                good.clone(),
                small_order_ed_pks()[0].1.clone(),
            ));
            if mlen > 0 {
                let mut v = good.clone();
                v[64] ^= 1;
                cases.push(("message flipped".into(), v, pk.clone()));
            }
            for (label, smbuf, vpk) in cases {
                for &(m_null, len_null) in
                    &[(false, false), (false, true), (true, false), (true, true)]
                {
                    let total = mlen + 64;
                    let mut m1 = canary(mlen + 16);
                    let mut m2 = canary(mlen + 16);
                    let mut o1 = 0xDEAD_BEEFu64;
                    let mut o2 = 0xDEAD_BEEFu64;
                    let (mp1, mp2) = if m_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (m1.as_mut_ptr(), m2.as_mut_ptr())
                    };
                    let (lp1, lp2) = if len_null {
                        (ptr::null_mut(), ptr::null_mut())
                    } else {
                        (&raw mut o1, &raw mut o2)
                    };
                    let (x, y) = unsafe {
                        (
                            f(mp1, lp1, smbuf.as_ptr(), total as u64, vpk.as_ptr()),
                            g(mp2, lp2, smbuf.as_ptr(), total as u64, vpk.as_ptr()),
                        )
                    };
                    let tag = format!(
                        "{name}({label},mlen={mlen},m_null={m_null},len_null={len_null})"
                    );
                    eq_i32(&format!("{tag} rc"), x, y);
                    assert_eq!(x, -1, "{tag} must reject");
                    eq_usize(&format!("{tag} *mlen_p"), o1 as usize, o2 as usize);
                    if len_null {
                        assert_eq!(o1, 0xDEAD_BEEF);
                    } else {
                        assert_eq!(o1, 0, "{tag}: *mlen_p must be 0");
                    }
                    eq_bytes(&format!("{tag} m"), &m1, &m2);
                    if m_null {
                        assert_eq!(m1, canary(mlen + 16), "{tag}: m == NULL must not write");
                    } else {
                        let mut want = canary(mlen + 16);
                        for b in want[..mlen].iter_mut() {
                            *b = 0;
                        }
                        assert_eq!(
                            m1, want,
                            "{tag}: exactly smlen-64 bytes of m must be zeroed"
                        );
                    }
                }
            }
        }
    }
}

/// G5-083, G5-084, G5-085, G5-086, G5-087, G5-088 — the `crypto_sign`
/// entry points with no failure path at all, including `_sk_to_pk` on a
/// deliberately inconsistent `sk`.
#[test]
fn sign_infallible_entry_points() {
    setup();
    let mut rng = Rng::new(0x23200);
    let det = pair::<Sign5>("crypto_sign_detached");
    let edet = pair::<Sign5>("crypto_sign_ed25519_detached");
    let seed_kp = pair::<SeedKeypair>("crypto_sign_seed_keypair");
    let eseed_kp = pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let kp = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> i32>("crypto_sign_keypair");
    let to_seed = pair::<Two>("crypto_sign_ed25519_sk_to_seed");
    let to_pk = pair::<Two>("crypto_sign_ed25519_sk_to_pk");
    let sk_to_c = pair::<Two>("crypto_sign_ed25519_sk_to_curve25519");

    // ---- G5-083: `_detached` never fails, for any key or length
    for &mlen in &[0usize, 1, 64, 1000] {
        for sk in [vec![0u8; 64], vec![0xffu8; 64], rng.bytes(64)] {
            let m = rng.bytes(mlen);
            for (name, f) in [
                ("crypto_sign_detached", det),
                ("crypto_sign_ed25519_detached", edet),
            ] {
                let mut s1 = canary(64 + 4);
                let mut s2 = canary(64 + 4);
                let mut l1 = 0u64;
                let mut l2 = 0u64;
                let (x, y) = unsafe {
                    (
                        f.0(s1.as_mut_ptr(), &mut l1, m.as_ptr(), mlen as u64, sk.as_ptr()),
                        f.1(s2.as_mut_ptr(), &mut l2, m.as_ptr(), mlen as u64, sk.as_ptr()),
                    )
                };
                eq_i32(&format!("{name} rc"), x, y);
                assert_eq!(x, 0, "{name} must always return 0");
                assert_eq!((l1, l2), (64, 64));
                eq_bytes(&format!("{name}(sk={})", hex(&sk)), &s1, &s2);
            }
        }
    }

    // ---- G5-084 / G5-085: key generation never fails
    for seed in [vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)] {
        for (name, f) in [
            ("crypto_sign_seed_keypair", seed_kp),
            ("crypto_sign_ed25519_seed_keypair", eseed_kp),
        ] {
            let mut pk1 = canary(32);
            let mut sk1 = canary(64);
            let mut pk2 = canary(32);
            let mut sk2 = canary(64);
            let (x, y) = unsafe {
                (
                    f.0(pk1.as_mut_ptr(), sk1.as_mut_ptr(), seed.as_ptr()),
                    f.1(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()),
                )
            };
            eq_i32(&format!("{name} rc"), x, y);
            assert_eq!(x, 0);
            eq_bytes(&format!("{name} pk"), &pk1, &pk2);
            eq_bytes(&format!("{name} sk"), &sk1, &sk2);
            assert_eq!(&sk1[..32], &seed[..], "sk[0..32] must be the raw seed");
            assert_eq!(&sk1[32..], &pk1[..], "sk[32..64] must be pk");
        }
    }
    for i in 0..4u64 {
        let mut pk1 = canary(32);
        let mut sk1 = canary(64);
        let mut pk2 = canary(32);
        let mut sk2 = canary(64);
        let (x, y) = {
            let _g = RNG_LOCK.lock().unwrap();
            reset_rngs(0x2320 + i);
            let x = unsafe { kp.0(pk1.as_mut_ptr(), sk1.as_mut_ptr()) };
            reset_rngs(0x2320 + i);
            let y = unsafe { kp.1(pk2.as_mut_ptr(), sk2.as_mut_ptr()) };
            (x, y)
        };
        eq_i32("crypto_sign_keypair rc", x, y);
        assert_eq!(x, 0);
        eq_bytes("crypto_sign_keypair pk", &pk1, &pk2);
        eq_bytes("crypto_sign_keypair sk", &sk1, &sk2);
    }

    // ---- G5-086 / G5-087 / G5-088: pure copies / hashes, never validated
    for sk in [
        vec![0u8; 64],
        vec![0xffu8; 64],
        rng.bytes(64),
        // an *inconsistent* sk: the embedded pk does not match the seed
        {
            let mut v = rng.bytes(64);
            v[32..].copy_from_slice(&[7u8; 32]);
            v
        },
    ] {
        for (name, f) in [
            ("crypto_sign_ed25519_sk_to_seed", to_seed),
            ("crypto_sign_ed25519_sk_to_pk", to_pk),
            ("crypto_sign_ed25519_sk_to_curve25519", sk_to_c),
        ] {
            let mut a = canary(32 + 4);
            let mut b = canary(32 + 4);
            let (x, y) = unsafe {
                (f.0(a.as_mut_ptr(), sk.as_ptr()), f.1(b.as_mut_ptr(), sk.as_ptr()))
            };
            eq_i32(&format!("{name} rc"), x, y);
            assert_eq!(x, 0, "{name} must always return 0");
            eq_bytes(&format!("{name}(sk={})", hex(&sk)), &a, &b);
            assert_eq!(&a[32..], &[0xA5u8; 4], "{name} wrote past 32 bytes");
        }
    }
}

/// G5-089, G5-090, G5-091 — the three distinct `return -1` branches of
/// `crypto_sign_ed25519_pk_to_curve25519`, each with its own concrete input
/// class, and the fact that `curve25519_pk` is never written on failure.
#[test]
fn pk_to_curve25519_rejections() {
    setup();
    let (f, g) = pair::<Two>("crypto_sign_ed25519_pk_to_curve25519");
    for (branch, cases) in [
        ("frombytes_negate_vartime != 0", undecodable_ed_pks()),
        ("has_small_order != 0", small_order_ed_pks()),
        ("is_on_main_subgroup == 0", off_subgroup_ed_pks()),
    ] {
        for (label, pk) in cases {
            let mut a = canary(32 + 8);
            let mut b = canary(32 + 8);
            let (x, y) = unsafe {
                (f(a.as_mut_ptr(), pk.as_ptr()), g(b.as_mut_ptr(), pk.as_ptr()))
            };
            eq_i32(&format!("pk_to_curve25519({branch}: {label}) rc"), x, y);
            assert_eq!(x, -1, "pk_to_curve25519({branch}: {label}) must reject");
            eq_bytes(&format!("pk_to_curve25519({branch}: {label})"), &a, &b);
            assert_eq!(
                a,
                canary(32 + 8),
                "pk_to_curve25519({branch}: {label}) must not write the output"
            );
        }
    }
    // the non-canonical y >= p encodings reach the *small-order* branch here,
    // because this function performs no `ge25519_is_canonical` test
    for (label, pk) in noncanonical_ed_pks() {
        let mut a = canary(32 + 8);
        let mut b = canary(32 + 8);
        let (x, y) = unsafe {
            (f(a.as_mut_ptr(), pk.as_ptr()), g(b.as_mut_ptr(), pk.as_ptr()))
        };
        eq_i32(&format!("pk_to_curve25519(non-canonical {label}) rc"), x, y);
        assert_eq!(x, -1);
        eq_bytes(&format!("pk_to_curve25519(non-canonical {label})"), &a, &b);
        assert_eq!(a, canary(32 + 8));
    }
}

/// G5-092, G5-093, G5-094, G5-095, G5-096 — the multipart API: `_init` /
/// `_update` / `_final_create` never fail (including `_update` after a
/// `_final_*`, which silently operates on the zeroed state), and
/// `_final_verify` reproduces all seven verification rejections with
/// `prehashed = 1`.
#[test]
fn sign_ph_error_surface() {
    setup();
    let mut rng = Rng::new(0x23300);
    for (name, init, upd, create, verify) in [
        (
            "crypto_sign",
            pair::<PhInit>("crypto_sign_init"),
            pair::<PhUpdate>("crypto_sign_update"),
            pair::<PhCreate>("crypto_sign_final_create"),
            pair::<PhVerify>("crypto_sign_final_verify"),
        ),
        (
            "crypto_sign_ed25519ph",
            pair::<PhInit>("crypto_sign_ed25519ph_init"),
            pair::<PhUpdate>("crypto_sign_ed25519ph_update"),
            pair::<PhCreate>("crypto_sign_ed25519ph_final_create"),
            pair::<PhVerify>("crypto_sign_ed25519ph_final_verify"),
        ),
    ] {
        for &mlen in &[0usize, 1, 64, 1000] {
            let (pk, sk) = sign_kp(&mut rng);
            let m = rng.bytes(mlen);

            // ---- G5-092 / G5-093 / G5-094: nothing here can fail
            let mut sigs: Vec<Vec<u8>> = Vec::new();
            let mut post_states: Vec<Vec<u8>> = Vec::new();
            // drawn once, so both libraries see the same post-`_final` input
            let extra = rng.bytes(37);
            for which in 0..2usize {
                let (i, u, c) = (
                    if which == 0 { init.0 } else { init.1 },
                    if which == 0 { upd.0 } else { upd.1 },
                    if which == 0 { create.0 } else { create.1 },
                );
                let mut st = State::for_sym("crypto_sign_ed25519ph_statebytes");
                let mut sig = canary(64 + 4);
                let mut sl = 0xDEADu64;
                unsafe {
                    assert_eq!(i(st.as_mut_ptr()), 0, "{name}_init must return 0");
                    // `m == NULL, mlen == 0` is a legal no-op
                    assert_eq!(u(st.as_mut_ptr(), ptr::null(), 0), 0, "{name}_update(NULL,0)");
                    assert_eq!(u(st.as_mut_ptr(), m.as_ptr(), mlen as u64), 0);
                    // a huge-looking mlen is not range checked; use the real one
                    assert_eq!(
                        c(st.as_mut_ptr(), sig.as_mut_ptr(), &mut sl, sk.as_ptr()),
                        0,
                        "{name}_final_create must return 0"
                    );
                }
                assert_eq!(sl, 64);
                // ---- G5-095: `_update` after `_final_*` still returns 0, and
                // the follow-on digest is SHA-512 over an all-zero state.
                unsafe {
                    assert_eq!(
                        u(st.as_mut_ptr(), extra.as_ptr(), 37),
                        0,
                        "{name}_update after _final must still return 0"
                    );
                }
                let mut sig2 = canary(64);
                unsafe {
                    assert_eq!(
                        c(st.as_mut_ptr(), sig2.as_mut_ptr(), ptr::null_mut(), sk.as_ptr()),
                        0
                    )
                };
                sigs.push(sig[..64].to_vec());
                post_states.push(sig2.clone());
            }
            eq_bytes(&format!("{name}_final_create(mlen={mlen})"), &sigs[0], &sigs[1]);
            eq_bytes(
                &format!("{name}: signature after a post-_final _update (mlen={mlen})"),
                &post_states[0],
                &post_states[1],
            );
            let sig = sigs[0].clone();

            // ---- G5-096: `_final_verify` with each rejection class
            let mut bad_sigs: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
            for (label, s) in noncanonical_scalars() {
                let mut v = sig.clone();
                v[32..].copy_from_slice(&s);
                bad_sigs.push((format!("S non-canonical {label}"), v, pk.clone()));
            }
            for (label, bpk) in noncanonical_ed_pks() {
                bad_sigs.push((format!("pk non-canonical {label}"), sig.clone(), bpk));
            }
            for (label, bpk) in undecodable_ed_pks() {
                bad_sigs.push((format!("pk undecodable {label}"), sig.clone(), bpk));
            }
            for (label, bpk) in small_order_ed_pks() {
                bad_sigs.push((format!("pk small order {label}"), sig.clone(), bpk));
            }
            for (label, r) in undecodable_ed_pks() {
                let mut v = sig.clone();
                v[..32].copy_from_slice(&r);
                bad_sigs.push((format!("R undecodable {label}"), v, pk.clone()));
            }
            for (label, r) in small_order_ed_pks() {
                let mut v = sig.clone();
                v[..32].copy_from_slice(&r);
                bad_sigs.push((format!("R small order {label}"), v, pk.clone()));
            }
            {
                let mut v = sig.clone();
                v[40] ^= 1;
                bad_sigs.push(("S bit flipped".into(), v, pk.clone()));
                let (pk2, _) = sign_kp(&mut rng);
                bad_sigs.push(("wrong pk".into(), sig.clone(), pk2));
            }
            for (label, bs, bpk) in &bad_sigs {
                for which in 0..2usize {
                    let (i, u, v) = (
                        if which == 0 { init.0 } else { init.1 },
                        if which == 0 { upd.0 } else { upd.1 },
                        if which == 0 { verify.0 } else { verify.1 },
                    );
                    let mut st = State::for_sym("crypto_sign_ed25519ph_statebytes");
                    let rc = unsafe {
                        assert_eq!(i(st.as_mut_ptr()), 0);
                        assert_eq!(u(st.as_mut_ptr(), m.as_ptr(), mlen as u64), 0);
                        v(st.as_mut_ptr(), bs.as_ptr(), bpk.as_ptr())
                    };
                    PH_VERIFY.with(|c| {
                        let mut b = c.borrow_mut();
                        if which == 0 {
                            *b = rc;
                        } else {
                            eq_i32(&format!("{name}_final_verify({label}) rc"), *b, rc);
                        }
                    });
                    assert_eq!(rc, -1, "{name}_final_verify({label}) must reject");
                }
            }
            // and a wrong *message* under the same signature
            if mlen > 0 {
                let mut m2 = m.clone();
                m2[0] ^= 1;
                for which in 0..2usize {
                    let (i, u, v) = (
                        if which == 0 { init.0 } else { init.1 },
                        if which == 0 { upd.0 } else { upd.1 },
                        if which == 0 { verify.0 } else { verify.1 },
                    );
                    let mut st = State::for_sym("crypto_sign_ed25519ph_statebytes");
                    let rc = unsafe {
                        i(st.as_mut_ptr());
                        u(st.as_mut_ptr(), m2.as_ptr(), mlen as u64);
                        v(st.as_mut_ptr(), sig.as_ptr(), pk.as_ptr())
                    };
                    assert_eq!(rc, -1, "{name}_final_verify(wrong message) (lib={which})");
                }
            }
        }
    }
}

thread_local! {
    static PH_VERIFY: std::cell::RefCell<i32> = const { std::cell::RefCell::new(0) };
}

// ===========================================================================
// crypto_auth
// ===========================================================================

struct AuthApi {
    name: &'static str,
    bytes: usize,
    block: usize,
    one: (Auth4, Auth4),
    ver: (AuthV4, AuthV4),
    init: (AuthInit, AuthInit),
    upd: (AuthUpdate, AuthUpdate),
    fin: (AuthFinal, AuthFinal),
    statebytes: String,
}

fn auth_apis() -> Vec<AuthApi> {
    [
        ("crypto_auth_hmacsha256", 32usize, 64usize),
        ("crypto_auth_hmacsha512", 64, 128),
        ("crypto_auth_hmacsha512256", 32, 128),
    ]
    .into_iter()
    .map(|(name, bytes, block)| AuthApi {
        name,
        bytes,
        block,
        one: pair::<Auth4>(name),
        ver: pair::<AuthV4>(&format!("{name}_verify")),
        init: pair::<AuthInit>(&format!("{name}_init")),
        upd: pair::<AuthUpdate>(&format!("{name}_update")),
        fin: pair::<AuthFinal>(&format!("{name}_final")),
        statebytes: format!("{name}_statebytes"),
    })
    .collect()
}

/// G5-103, G5-104, G5-105, G5-106 — every `_verify` returns -1 for a tag that
/// differs in at least one bit, for every `inlen` including 0, and the generic
/// `crypto_auth_verify` delegates identically.
#[test]
fn auth_verify_rejections() {
    setup();
    let mut rng = Rng::new(0x24000);
    let generic = pair::<AuthV4>("crypto_auth_verify");
    let gauth = sym::<Auth4>(c_lib(), "crypto_auth");
    for api in &auth_apis() {
        for &inlen in &[0usize, 1, 63, 64, 65, 127, 128, 129, 1000] {
            let k = rng.bytes(32);
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut good = canary(api.bytes);
            unsafe { assert_eq!(api.one.0(good.as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0) };
            // every single-bit difference in a handful of positions, plus the
            // degenerate all-zero / all-ff tags
            let mut cases: Vec<(String, Vec<u8>)> = vec![
                ("zero".into(), vec![0u8; api.bytes]),
                ("ff".into(), vec![0xffu8; api.bytes]),
                ("random".into(), rng.bytes(api.bytes)),
            ];
            for byte in [0usize, 1, api.bytes / 2, api.bytes - 1] {
                for bit in [0u32, 3, 7] {
                    let mut v = good.clone();
                    v[byte] ^= 1 << bit;
                    cases.push((format!("byte{byte}bit{bit}"), v));
                }
            }
            for (label, h) in &cases {
                if h == &good {
                    continue;
                }
                let (x, y) = unsafe {
                    (
                        api.ver.0(h.as_ptr(), ip, inlen as u64, k.as_ptr()),
                        api.ver.1(h.as_ptr(), ip, inlen as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{}_verify({label},inlen={inlen}) rc", api.name), x, y);
                assert_eq!(x, -1, "{}_verify({label}) must return -1", api.name);
            }
        }
    }
    // G5-106: the generic wrapper
    for &inlen in &[0usize, 1, 64, 1000] {
        let k = rng.bytes(32);
        let input = rng.bytes(inlen);
        let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
        let mut good = canary(32);
        unsafe { assert_eq!(gauth(good.as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0) };
        for byte in 0..32usize {
            let mut v = good.clone();
            v[byte] ^= 0x80;
            let (x, y) = unsafe {
                (
                    generic.0(v.as_ptr(), ip, inlen as u64, k.as_ptr()),
                    generic.1(v.as_ptr(), ip, inlen as u64, k.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_auth_verify(byte{byte},inlen={inlen}) rc"), x, y);
            assert_eq!(x, -1);
        }
        // correct tag still accepted
        let (x, y) = unsafe {
            (
                generic.0(good.as_ptr(), ip, inlen as u64, k.as_ptr()),
                generic.1(good.as_ptr(), ip, inlen as u64, k.as_ptr()),
            )
        };
        eq_i32("crypto_auth_verify correct rc", x, y);
        assert_eq!(x, 0);
    }
}

/// G5-099, G5-102, G5-107, G5-108 — the accepting cases: `key == NULL` with
/// `keylen == 0` is legal (the pads stay plain 0x36/0x5c), the one-shot MACs
/// can never fail, and `_update` / `_final` always return 0 (including
/// `inlen == 0` and `in == NULL`).
#[test]
fn auth_accepting_edge_cases() {
    setup();
    let mut rng = Rng::new(0x24100);
    for api in &auth_apis() {
        for &inlen in &[0usize, 1, 64, 128, 1000] {
            let input = rng.bytes(inlen);
            // ---- G5-099 / G5-102: key == NULL, keylen == 0
            let mut outs: Vec<Vec<u8>> = Vec::new();
            let mut states: Vec<Vec<u8>> = Vec::new();
            for which in 0..2usize {
                let (i, u, f) = (
                    if which == 0 { api.init.0 } else { api.init.1 },
                    if which == 0 { api.upd.0 } else { api.upd.1 },
                    if which == 0 { api.fin.0 } else { api.fin.1 },
                );
                let mut st = State::for_sym(&api.statebytes);
                let mut out = canary(api.bytes + 4);
                unsafe {
                    assert_eq!(
                        i(st.as_mut_ptr(), ptr::null(), 0),
                        0,
                        "{}_init(NULL, 0) must return 0",
                        api.name
                    );
                    // G5-108: `_update` with inlen == 0 (and a NULL pointer) is
                    // a no-op that returns 0
                    assert_eq!(u(st.as_mut_ptr(), ptr::null(), 0), 0);
                    assert_eq!(u(st.as_mut_ptr(), input.as_ptr(), inlen as u64), 0);
                    assert_eq!(u(st.as_mut_ptr(), ptr::null(), 0), 0);
                    assert_eq!(f(st.as_mut_ptr(), out.as_mut_ptr()), 0);
                }
                outs.push(out);
                states.push(st.bytes().to_vec());
            }
            eq_bytes(
                &format!("{}_init(NULL,0)(inlen={inlen})", api.name),
                &outs[0],
                &outs[1],
            );
            eq_bytes(
                &format!("{}_init(NULL,0) state(inlen={inlen})", api.name),
                &states[0],
                &states[1],
            );
            assert_eq!(&outs[0][api.bytes..], &[0xA5u8; 4]);

            // an equivalent all-zero-length key given as a non-NULL pointer
            let dummy = [0u8; 4];
            let mut alt = canary(api.bytes);
            unsafe {
                let (i, u, f) = (api.init.0, api.upd.0, api.fin.0);
                let mut st = State::for_sym(&api.statebytes);
                assert_eq!(i(st.as_mut_ptr(), dummy.as_ptr(), 0), 0);
                assert_eq!(u(st.as_mut_ptr(), input.as_ptr(), inlen as u64), 0);
                assert_eq!(f(st.as_mut_ptr(), alt.as_mut_ptr()), 0);
            }
            assert_eq!(
                &alt[..], &outs[0][..api.bytes],
                "{}: key == NULL and a non-NULL zero-length key must agree",
                api.name
            );

            // The `keylen > blocksize` boundary that separates the clean
            // `sodium_misuse()` rows (G5-097 / G5-100) from the NULL-deref rows
            // (G5-098 / G5-101): a *non-NULL* key of exactly `blocksize` bytes
            // is used verbatim and accepted.
            {
                let bk = rng.bytes(api.block);
                for which in 0..2usize {
                    let (i, u, f) = (
                        if which == 0 { api.init.0 } else { api.init.1 },
                        if which == 0 { api.upd.0 } else { api.upd.1 },
                        if which == 0 { api.fin.0 } else { api.fin.1 },
                    );
                    let mut st = State::for_sym(&api.statebytes);
                    let mut out = canary(api.bytes);
                    unsafe {
                        assert_eq!(
                            i(st.as_mut_ptr(), bk.as_ptr(), api.block),
                            0,
                            "{}_init(keylen == blocksize) must return 0",
                            api.name
                        );
                        assert_eq!(u(st.as_mut_ptr(), input.as_ptr(), inlen as u64), 0);
                        assert_eq!(f(st.as_mut_ptr(), out.as_mut_ptr()), 0);
                    }
                    BLOCK_KEY.with(|c| {
                        let mut b = c.borrow_mut();
                        if which == 0 {
                            *b = out.clone();
                        } else {
                            eq_bytes(
                                &format!("{}_init(keylen={})", api.name, api.block),
                                &b,
                                &out,
                            );
                        }
                    });
                }
            }

            // ---- G5-107: the one-shot MAC can never fail
            let k = rng.bytes(32);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut a = canary(api.bytes);
            let mut b = canary(api.bytes);
            let (x, y) = unsafe {
                (
                    api.one.0(a.as_mut_ptr(), ip, inlen as u64, k.as_ptr()),
                    api.one.1(b.as_mut_ptr(), ip, inlen as u64, k.as_ptr()),
                )
            };
            eq_i32(&format!("{} rc(inlen={inlen})", api.name), x, y);
            assert_eq!(x, 0, "{} must always return 0", api.name);
            eq_bytes(&format!("{}(inlen={inlen})", api.name), &a, &b);
        }
    }
}

// ===========================================================================
// documented-unreachable rows
// ===========================================================================

/// G5-003, G5-006, G5-031, G5-037, G5-065, G5-068, G5-080, G5-082, G5-109 —
/// rows that cannot be constructed from the public API. Each is documented
/// here with the reason, and the checkable part of the claim is asserted.
#[test]
fn documented_unreachable_rows() {
    setup();
    let mut rng = Rng::new(0x25000);

    // ---- G5-003 / G5-006 / G5-065 / G5-068: the "X25519 output q is all
    // zero" post-check in crypto_scalarmult_curve25519() is dead behind the
    // blocklist: `q == 0` happens exactly when `pk` has small order, and every
    // small-order encoding is already rejected by `has_small_order()` (which
    // masks bit 255, so both sign variants are covered). Demonstrate that the
    // whole small subgroup is in the blocklist by showing that every
    // small-order encoding is rejected *before* the ladder runs — i.e. `q` is
    // never written at all.
    {
        let (c, r) = pair::<Three>("crypto_scalarmult_curve25519");
        for (what, pk) in small_order_pks() {
            let sk = rng.bytes(32);
            let mut q1 = canary(32);
            let mut q2 = canary(32);
            let (x, y) = unsafe {
                (
                    c(q1.as_mut_ptr(), sk.as_ptr(), pk.as_ptr()),
                    r(q2.as_mut_ptr(), sk.as_ptr(), pk.as_ptr()),
                )
            };
            eq_i32(&format!("crypto_scalarmult({what}) rc"), x, y);
            assert_eq!(x, -1);
            eq_bytes(&format!("crypto_scalarmult({what}) q"), &q1, &q2);
            assert_eq!(
                q1,
                canary(32),
                "{what}: the blocklist rejects before the ladder, so the \
                 all-zero-q post-check is unreachable"
            );
        }
    }

    // ---- G5-031 / G5-037: `crypto_box_seal` checks `crypto_box_keypair() != 0`,
    // but `crypto_scalarmult_curve25519_base()` has no failure path, so the
    // branch is dead (marked LCOV_EXCL_LINE upstream). Assert the claim: the
    // base scalarmult returns 0 for every scalar, including the degenerate
    // ones.
    {
        let (c, r) = pair::<Two>("crypto_scalarmult_curve25519_base");
        let mut scalars: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], vec![1u8; 32]];
        for _ in 0..20 {
            scalars.push(rng.bytes(32));
        }
        for n in &scalars {
            let mut a = canary(32);
            let mut b = canary(32);
            let (x, y) = unsafe {
                (c(a.as_mut_ptr(), n.as_ptr()), r(b.as_mut_ptr(), n.as_ptr()))
            };
            eq_i32("crypto_scalarmult_curve25519_base rc", x, y);
            assert_eq!(x, 0, "the base scalarmult must never fail (sk={})", hex(n));
            eq_bytes("crypto_scalarmult_curve25519_base", &a, &b);
        }
    }

    // ---- G5-080: `crypto_sign_ed25519_open`'s
    // `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX` test is dead on
    // x86-64: MESSAGEBYTES_MAX == SIZE_MAX - 64, so the largest possible
    // `smlen` (2^64-1) gives exactly MESSAGEBYTES_MAX and the strict `>` never
    // holds. Assert the constant, which is the whole of the claim.
    {
        let (c, r) = pair::<SizeFn>("crypto_sign_ed25519_messagebytes_max");
        let (x, y) = unsafe { (c(), r()) };
        eq_usize("crypto_sign_ed25519_messagebytes_max", x, y);
        assert_eq!(x, usize::MAX - 64, "MESSAGEBYTES_MAX must make the guard dead");
        assert_eq!(u64::MAX - 64, x as u64);
    }

    // ---- G5-082: `crypto_sign_ed25519`'s error path is dead
    // (`_crypto_sign_ed25519_detached` unconditionally returns 0 and sets
    // siglen = 64). What IS observable is that `memmove(sm+64, m, mlen)` runs
    // before any check, so `sm[64..]` always holds the message.
    {
        let (c, r) = pair::<Sign5>("crypto_sign_ed25519");
        let (dc, dr) = pair::<Sign5>("crypto_sign_ed25519_detached");
        for &mlen in &[0usize, 1, 64, 1000] {
            for sk in [vec![0u8; 64], vec![0xffu8; 64], rng.bytes(64)] {
                let m = rng.bytes(mlen);
                let mut s1 = canary(mlen + 64);
                let mut s2 = canary(mlen + 64);
                let mut l1 = 0u64;
                let mut l2 = 0u64;
                let (x, y) = unsafe {
                    (
                        c(s1.as_mut_ptr(), &mut l1, m.as_ptr(), mlen as u64, sk.as_ptr()),
                        r(s2.as_mut_ptr(), &mut l2, m.as_ptr(), mlen as u64, sk.as_ptr()),
                    )
                };
                eq_i32("crypto_sign_ed25519 rc", x, y);
                assert_eq!(x, 0, "the error path is unreachable");
                assert_eq!((l1, l2), ((mlen + 64) as u64, (mlen + 64) as u64));
                eq_bytes("crypto_sign_ed25519", &s1, &s2);
                assert_eq!(&s1[64..], &m[..], "sm[64..] is written before any check");
                // and the inner detached routine really does always succeed
                let mut d1 = canary(64);
                let mut d2 = canary(64);
                let mut dl1 = 0u64;
                let mut dl2 = 0u64;
                let (x, y) = unsafe {
                    (
                        dc(d1.as_mut_ptr(), &mut dl1, m.as_ptr(), mlen as u64, sk.as_ptr()),
                        dr(d2.as_mut_ptr(), &mut dl2, m.as_ptr(), mlen as u64, sk.as_ptr()),
                    )
                };
                eq_i32("crypto_sign_ed25519_detached rc", x, y);
                assert_eq!((x, dl1, dl2), (0, 64, 64));
                eq_bytes("crypto_sign_ed25519_detached", &d1, &d2);
            }
        }
    }

    // ---- G5-109: there is no runtime `assert()` and no `abort()` anywhere in
    // the G5 sources; the only abort path is `sodium_misuse()`. Every abort
    // observed by `misuse_paths_match` below exits with MISUSE_EXIT (i.e. the
    // installed handler ran) rather than raising SIGABRT directly, which is
    // exactly what that claim predicts. The two SIGSEGV rows (G5-098 / G5-101)
    // are NULL dereferences, not assertions. Assert the size constants the row
    // derives from.
    {
        for (name, want) in [
            ("crypto_box_messagebytes_max", u64::MAX - 16),
            ("crypto_box_curve25519xsalsa20poly1305_messagebytes_max", u64::MAX - 16),
            ("crypto_box_curve25519xchacha20poly1305_messagebytes_max", u64::MAX - 16),
            ("crypto_secretbox_messagebytes_max", u64::MAX - 16),
            ("crypto_secretbox_xchacha20poly1305_messagebytes_max", u64::MAX - 16),
            ("crypto_secretstream_xchacha20poly1305_messagebytes_max", 274877906816),
        ] {
            let (c, r) = pair::<SizeFn>(name);
            let (x, y) = unsafe { (c(), r()) };
            eq_usize(name, x, y);
            assert_eq!(x as u64, want, "{name}");
        }
    }
}

// ===========================================================================
// `sodium_misuse()` and raw-SIGSEGV rows — run out of process
// ===========================================================================

/// One byte range the misuse handler prints: a `u64` out-parameter followed by
/// a `u8` out-parameter, laid out so that a single observation covers both.
#[repr(C)]
struct Obs {
    len: u64,
    tag: u8,
    pad: [u8; 7],
}

const MISUSE_CASES: &[&str] = &[
    // crypto_box easy layer: mlen > crypto_box_MESSAGEBYTES_MAX
    "box_easy/max+1",
    "box_easy/u64max",
    "box_easy_afternm/max+1",
    "box_easy_afternm/u64max",
    "xbox_easy/max+1",
    "xbox_easy_afternm/max+1",
    // sealed boxes
    "box_seal/max+1",
    "box_seal/u64max",
    "xbox_seal/max+1",
    // secretbox easy layer
    "secretbox_easy/max+1",
    "secretbox_easy/u64max",
    "xsecretbox_easy/max+1",
    // secretstream: *outlen_p / *mlen_p / *tag_p are written before the abort
    "ss_push/max+1",
    "ss_push/u64max",
    "ss_push/max+1/nullout",
    "ss_pull/max+18",
    "ss_pull/u64max",
    "ss_pull/max+18/nullout",
    // crypto_kx with both output pointers NULL
    "kx_client/both-null",
    "kx_server/both-null",
    // crypto_auth_*_init with key == NULL and 0 < keylen <= blocksize
    "hmacsha256_init/1",
    "hmacsha256_init/32",
    "hmacsha256_init/64",
    "hmacsha512_init/1",
    "hmacsha512_init/64",
    "hmacsha512_init/128",
    "hmacsha512256_init/1",
    "hmacsha512256_init/32",
    "hmacsha512256_init/128",
];

/// `key == NULL` with `keylen > blocksize`: the hashing branch is taken first,
/// so `crypto_hash_sha*_update(..., NULL, keylen)` dereferences NULL.
const SEGV_CASES: &[&str] = &[
    "hmacsha256_init/segv/65",
    "hmacsha256_init/segv/200",
    "hmacsha512_init/segv/129",
    "hmacsha512_init/segv/200",
    "hmacsha512256_init/segv/129",
];

/// G5-017, G5-018, G5-019, G5-020, G5-030, G5-036, G5-040, G5-043, G5-049,
/// G5-051, G5-063, G5-066, G5-097, G5-098, G5-100, G5-101 — the child half.
#[test]
fn misuse_child() {
    let Some((tag, lib)) = child_case() else {
        return;
    };
    let mut obs = Obs {
        len: 0xA5A5_A5A5_A5A5_A5A5,
        tag: 0xA5,
        pad: [0xA5; 7],
    };
    set_observation((&raw const obs).cast(), 16);

    let big_box = u64::MAX - 16; // crypto_box / crypto_secretbox MESSAGEBYTES_MAX
    let ss_max = 274_877_906_816u64;
    let mbuf = [0u8; 64];
    let mut cbuf = canary(256);
    let key = [3u8; 32];
    let nonce = [4u8; 24];
    let pk = [5u8; 32];
    let sk = [6u8; 32];

    let parts: Vec<&str> = tag.split('/').collect();
    match parts[0] {
        "box_easy" | "xbox_easy" | "secretbox_easy" | "xsecretbox_easy" => {
            let mlen = if parts[1] == "u64max" { u64::MAX } else { big_box + 1 };
            let name = match parts[0] {
                "box_easy" => "crypto_box_easy",
                "xbox_easy" => "crypto_box_curve25519xchacha20poly1305_easy",
                "secretbox_easy" => "crypto_secretbox_easy",
                _ => "crypto_secretbox_xchacha20poly1305_easy",
            };
            if parts[0].contains("secretbox") {
                let f = sym::<Sym5>(lib, name);
                let rc = unsafe {
                    f(cbuf.as_mut_ptr(), mbuf.as_ptr(), mlen, nonce.as_ptr(), key.as_ptr())
                };
                println!("OBS rc={rc}");
            } else {
                let f = sym::<Asym6>(lib, name);
                let rc = unsafe {
                    f(cbuf.as_mut_ptr(), mbuf.as_ptr(), mlen, nonce.as_ptr(), pk.as_ptr(),
                      sk.as_ptr())
                };
                println!("OBS rc={rc}");
            }
        }
        "box_easy_afternm" | "xbox_easy_afternm" => {
            let mlen = if parts[1] == "u64max" { u64::MAX } else { big_box + 1 };
            let name = if parts[0] == "box_easy_afternm" {
                "crypto_box_easy_afternm"
            } else {
                "crypto_box_curve25519xchacha20poly1305_easy_afternm"
            };
            let f = sym::<Sym5>(lib, name);
            let rc = unsafe {
                f(cbuf.as_mut_ptr(), mbuf.as_ptr(), mlen, nonce.as_ptr(), key.as_ptr())
            };
            println!("OBS rc={rc}");
        }
        "box_seal" | "xbox_seal" => {
            let mlen = if parts[1] == "u64max" { u64::MAX } else { big_box + 1 };
            let name = if parts[0] == "box_seal" {
                "crypto_box_seal"
            } else {
                "crypto_box_curve25519xchacha20poly1305_seal"
            };
            let f = sym::<Seal>(lib, name);
            let rc = unsafe { f(cbuf.as_mut_ptr(), mbuf.as_ptr(), mlen, pk.as_ptr()) };
            println!("OBS rc={rc} c0={}", hex(&cbuf[..32]));
        }
        "ss_push" => {
            let mlen = if parts[1] == "u64max" { u64::MAX } else { ss_max + 1 };
            let null_out = parts.len() > 2 && parts[2] == "nullout";
            let ip = sym::<SsInitPull>(lib, &format!("{SS}_init_pull"));
            let mut st = State::for_sym(&format!("{SS}_statebytes"));
            let hdr = [7u8; 24];
            unsafe { assert_eq!(ip(st.as_mut_ptr(), hdr.as_ptr(), key.as_ptr()), 0) };
            let f = sym::<SsPush>(lib, &format!("{SS}_push"));
            let lp = if null_out { ptr::null_mut() } else { &raw mut obs.len };
            let rc = unsafe {
                f(st.as_mut_ptr(), cbuf.as_mut_ptr(), lp, mbuf.as_ptr(), mlen,
                  ptr::null(), 0, 0)
            };
            println!("OBS rc={rc} len={}", obs.len);
        }
        "ss_pull" => {
            let inlen = if parts[1] == "u64max" { u64::MAX } else { ss_max + 18 };
            let null_out = parts.len() > 2 && parts[2] == "nullout";
            let ip = sym::<SsInitPull>(lib, &format!("{SS}_init_pull"));
            let mut st = State::for_sym(&format!("{SS}_statebytes"));
            let hdr = [7u8; 24];
            unsafe { assert_eq!(ip(st.as_mut_ptr(), hdr.as_ptr(), key.as_ptr()), 0) };
            let f = sym::<SsPull>(lib, &format!("{SS}_pull"));
            let (lp, tp) = if null_out {
                (ptr::null_mut(), ptr::null_mut())
            } else {
                (&raw mut obs.len, &raw mut obs.tag)
            };
            let rc = unsafe {
                f(st.as_mut_ptr(), cbuf.as_mut_ptr(), lp, tp, mbuf.as_ptr(), inlen,
                  ptr::null(), 0)
            };
            println!("OBS rc={rc} len={} tag={:#04x}", obs.len, obs.tag);
        }
        "kx_client" | "kx_server" => {
            let name = if parts[0] == "kx_client" {
                "crypto_kx_client_session_keys"
            } else {
                "crypto_kx_server_session_keys"
            };
            let f = sym::<KxSession>(lib, name);
            let rc = unsafe {
                f(ptr::null_mut(), ptr::null_mut(), pk.as_ptr(), sk.as_ptr(), pk.as_ptr())
            };
            println!("OBS rc={rc}");
        }
        "hmacsha256_init" | "hmacsha512_init" | "hmacsha512256_init" => {
            let name = format!("crypto_auth_{}", parts[0]);
            let keylen: usize = parts[parts.len() - 1].parse().unwrap();
            let f = sym::<AuthInit>(lib, &name);
            let mut st = State::for_sym(&format!(
                "crypto_auth_{}_statebytes",
                parts[0].trim_end_matches("_init")
            ));
            let rc = unsafe { f(st.as_mut_ptr(), ptr::null(), keylen) };
            println!("OBS rc={rc} state0={}", hex(&st.bytes()[..16]));
        }
        other => panic!("unknown tag {other}"),
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// The parent half: each row is run once against the C `.so` and once against
/// the Rust `.so`; the exit status, the termination signal and the observed
/// pre-abort side effects must all match, and the C must genuinely have gone
/// through `sodium_misuse()`.
#[test]
fn misuse_paths_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    for &tag in MISUSE_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        assert_eq!(
            c.status.code(),
            Some(MISUSE_EXIT),
            "{tag}: C did not reach sodium_misuse (stdout: {}, stderr: {})",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&c.stderr)
        );
    }
}

/// G5-098, G5-101 — `key == NULL` with `keylen > blocksize` is NOT a clean
/// `sodium_misuse()`: the key-hashing branch runs first and dereferences NULL.
/// C and Rust must die from the same signal.
#[test]
fn null_key_over_blocksize_segfaults_identically() {
    if child_tag().is_some() {
        return;
    }
    setup();
    for &tag in SEGV_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c.status.signal(),
            Some(11),
            "{tag}: the C reference must die with SIGSEGV (code={:?}, stdout: {}, stderr: {})",
            c.status.code(),
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&c.stderr)
        );
        assert_eq!(c.status.code(), None, "{tag}: no clean exit expected");
    }
}

thread_local! {
    static BLOCK_KEY: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}
