//! Phase C — error-path differential tests, one per row of ERRORS.md.
//!
//! Each test constructs the exact invalid input, calls both implementations and
//! asserts they return the *same* sentinel and leave the same side effects
//! behind, rather than merely both failing.

mod common;

use common::params::*;
use common::*;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

type SeedexpanderInit = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32;
type Seedexpander = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, u64) -> i32;
type Randombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type DrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Signature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type SignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type SetU32 = unsafe extern "C" fn(*mut u32, u32);
type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;

const DRBG_DETERMINISTIC: bool = !cfg!(feature = "urandom");

fn init_xof(libs: &Libs, maxlen: u64) -> (AesXofStruct, AesXofStruct) {
    let (ic, ir) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8;
    }
    let mut div = [0xAAu8; 8];
    let mut a = AesXofStruct::zeroed();
    let mut b = AesXofStruct::zeroed();
    unsafe {
        assert_eq!(ic(&mut a, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen), 0);
        assert_eq!(ir(&mut b, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen), 0);
    }
    eq("seedexpander_init state", a.as_bytes(), b.as_bytes());
    (a, b)
}

// ---------------------------------------------------------------------------
// Rows 1, 9, 10, 11 — seedexpander_init's maxlen bound
// ---------------------------------------------------------------------------

#[test]
fn row01_10_seedexpander_init_maxlen_bound() {
    let libs = load();
    let (ic, ir) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let mut seed = [7u8; 32];
    let mut div = [3u8; 8];

    // Row 1 / row 10: the smallest rejected value and beyond.  The C returns
    // before touching ctx, seed or diversifier, so pass null pointers for the
    // latter two: a Rust wrapper that dereferenced first would be caught here.
    for maxlen in [
        0x1_0000_0000u64,
        0x1_0000_0001,
        0x8000_0000_0000_0000,
        u64::MAX,
    ] {
        let mut a = AesXofStruct::zeroed();
        let mut b = AesXofStruct::zeroed();
        let before = a.as_bytes().to_vec();
        let (ra, rb) = unsafe {
            (
                ic(
                    &mut a,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    maxlen,
                ),
                ir(
                    &mut b,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    maxlen,
                ),
            )
        };
        assert_eq!(ra, RNG_BAD_MAXLEN, "C seedexpander_init({maxlen:#x})");
        assert_eq!(rb, RNG_BAD_MAXLEN, "Rust seedexpander_init({maxlen:#x})");
        eq("ctx untouched (C)", &before, a.as_bytes());
        eq("ctx untouched (Rust)", &before, b.as_bytes());
    }

    // Row 9: the largest accepted value.
    for maxlen in [0xFFFF_FFFFu64, 0xFFFF_FFFE] {
        let mut a = AesXofStruct::zeroed();
        let mut b = AesXofStruct::zeroed();
        let (ra, rb) = unsafe {
            (
                ic(&mut a, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen),
                ir(&mut b, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen),
            )
        };
        assert_eq!(ra, RNG_SUCCESS);
        assert_eq!(rb, RNG_SUCCESS);
        eq(&format!("state after init({maxlen:#x})"), a.as_bytes(), b.as_bytes());
        assert_eq!(a.length_remaining, maxlen);
        assert_eq!(a.buffer_pos, 16);
    }
}

#[test]
fn row11_seedexpander_init_maxlen_zero() {
    let libs = load();
    let (a, b) = init_xof(&libs, 0);
    assert_eq!(a.length_remaining, 0);
    eq("state after init(0)", a.as_bytes(), b.as_bytes());
    // and every subsequent request must fail (row 12)
    let (ec, er) = libs.pair::<Seedexpander>("seedexpander");
    let mut ca = a;
    let mut cb = b;
    for xlen in [0u64, 1, 16, 1000] {
        let mut x = [0u8; 1024];
        let mut y = [0u8; 1024];
        let (ra, rb) = unsafe { (ec(&mut ca, x.as_mut_ptr(), xlen), er(&mut cb, y.as_mut_ptr(), xlen)) };
        assert_eq!(ra, RNG_BAD_REQ_LEN, "C seedexpander(xlen={xlen}) on empty ctx");
        assert_eq!(rb, RNG_BAD_REQ_LEN, "Rust seedexpander(xlen={xlen}) on empty ctx");
        eq("state untouched", ca.as_bytes(), cb.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 2 — seedexpander with a null output buffer
// ---------------------------------------------------------------------------

#[test]
fn row02_seedexpander_null_output() {
    let libs = load();
    let (ec, er) = libs.pair::<Seedexpander>("seedexpander");
    let (mut ca, mut cb) = init_xof(&libs, 4096);
    // The NULL test precedes the length test in rng.c, so it must win even for
    // an xlen that would also be rejected as too long.
    for xlen in [0u64, 1, 16, 4095, 4096, 0xFFFF_FFFF] {
        let before = ca.as_bytes().to_vec();
        let (ra, rb) = unsafe {
            (
                ec(&mut ca, core::ptr::null_mut(), xlen),
                er(&mut cb, core::ptr::null_mut(), xlen),
            )
        };
        assert_eq!(ra, RNG_BAD_OUTBUF, "C seedexpander(NULL, {xlen})");
        assert_eq!(rb, RNG_BAD_OUTBUF, "Rust seedexpander(NULL, {xlen})");
        eq("ctx untouched (C)", &before, ca.as_bytes());
        eq("ctx untouched (Rust)", &before, cb.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Rows 3, 13, 14, 15, 16 — seedexpander's remaining-length bound
// ---------------------------------------------------------------------------

#[test]
fn row03_14_15_16_seedexpander_length_bound() {
    let libs = load();
    let (ec, er) = libs.pair::<Seedexpander>("seedexpander");

    for maxlen in [1u64, 2, 17, 100, 4096] {
        // Row 15: xlen == length_remaining exactly (smallest rejected).
        // Row 3/16: well past it.
        for xlen in [maxlen, maxlen + 1, maxlen * 2, 0xFFFF_FFFF] {
            let (mut ca, mut cb) = init_xof(&libs, maxlen);
            let before = ca.as_bytes().to_vec();
            // deliberately under-sized output buffers: neither side may write
            let mut x = [0xEEu8; 8];
            let mut y = [0xEEu8; 8];
            let (ra, rb) = unsafe {
                (ec(&mut ca, x.as_mut_ptr(), xlen), er(&mut cb, y.as_mut_ptr(), xlen))
            };
            assert_eq!(
                ra, RNG_BAD_REQ_LEN,
                "C seedexpander(xlen={xlen}, remaining={maxlen})"
            );
            assert_eq!(
                rb, RNG_BAD_REQ_LEN,
                "Rust seedexpander(xlen={xlen}, remaining={maxlen})"
            );
            eq("ctx untouched (C)", &before, ca.as_bytes());
            eq("ctx untouched (Rust)", &before, cb.as_bytes());
            eq("output untouched", &x, &y);
            assert_eq!(x, [0xEEu8; 8], "C wrote to the rejected output buffer");
            assert_eq!(y, [0xEEu8; 8], "Rust wrote to the rejected output buffer");
        }

        // Row 14: xlen == length_remaining - 1, the largest accepted request.
        if maxlen >= 1 {
            let xlen = maxlen - 1;
            let (mut ca, mut cb) = init_xof(&libs, maxlen);
            let mut x = vec![0xEEu8; xlen as usize + 8];
            let mut y = vec![0xEEu8; xlen as usize + 8];
            let (ra, rb) = unsafe {
                (ec(&mut ca, x.as_mut_ptr(), xlen), er(&mut cb, y.as_mut_ptr(), xlen))
            };
            assert_eq!(ra, rb);
            if xlen < maxlen {
                assert_eq!(ra, RNG_SUCCESS, "seedexpander(xlen={xlen}) should succeed");
                assert_eq!(ca.length_remaining, 1);
            }
            eq(&format!("seedexpander(xlen={xlen}) output"), &x, &y);
            eq("state", ca.as_bytes(), cb.as_bytes());
        }
    }

    // Row 13: xlen == 0 with a non-empty budget is a success and a no-op.
    let (mut ca, mut cb) = init_xof(&libs, 100);
    let before = ca.as_bytes().to_vec();
    let mut x = [0xEEu8; 8];
    let mut y = [0xEEu8; 8];
    let (ra, rb) = unsafe { (ec(&mut ca, x.as_mut_ptr(), 0), er(&mut cb, y.as_mut_ptr(), 0)) };
    assert_eq!(ra, RNG_SUCCESS);
    assert_eq!(rb, RNG_SUCCESS);
    eq("ctx unchanged by xlen=0 (C)", &before, ca.as_bytes());
    eq("ctx unchanged by xlen=0 (Rust)", &before, cb.as_bytes());
    assert_eq!(x, [0xEEu8; 8]);
    assert_eq!(y, [0xEEu8; 8]);
}

// ---------------------------------------------------------------------------
// Row 4 — AES256_ECB's abort path
// ---------------------------------------------------------------------------

/// `rng.c`'s `handleErrors()` is only reachable if one of `EVP_CIPHER_CTX_new`,
/// `EVP_EncryptInit_ex` or `EVP_EncryptUpdate` fails for a plain AES-256-ECB
/// single-block encryption.  There is no input to `AES256_ECB` that provokes it,
/// so the row is discharged by showing the function always succeeds for the
/// whole input domain shape it accepts, on both sides, and never aborts.
#[test]
fn row04_aes256_ecb_never_fails() {
    let libs = load();
    let f = libs.pair::<unsafe extern "C" fn(*mut u8, *mut u8, *mut u8)>("AES256_ECB");
    let mut rng = Rng::new(4);
    for _ in 0..512 {
        let mut k = [0u8; 32];
        let mut c = [0u8; 16];
        rng.fill(&mut k);
        rng.fill(&mut c);
        let mut ka = k;
        let mut kb = k;
        let mut ca = c;
        let mut cb = c;
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        unsafe {
            f.0(ka.as_mut_ptr(), ca.as_mut_ptr(), a.as_mut_ptr());
            f.1(kb.as_mut_ptr(), cb.as_mut_ptr(), b.as_mut_ptr());
        }
        eq("AES256_ECB", &a, &b);
    }
}

// ---------------------------------------------------------------------------
// Rows 5, 20, 21 — crypto_sign_verify's siglen check
// ---------------------------------------------------------------------------

fn keypair(libs: &Libs, rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let (fc, fr) = libs.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pka = vec![0u8; SPX_PK_BYTES];
    let mut ska = vec![0u8; SPX_SK_BYTES];
    let mut pkb = vec![0u8; SPX_PK_BYTES];
    let mut skb = vec![0u8; SPX_SK_BYTES];
    unsafe {
        fc(pka.as_mut_ptr(), ska.as_mut_ptr(), seed.as_ptr());
        fr(pkb.as_mut_ptr(), skb.as_mut_ptr(), seed.as_ptr());
    }
    eq("keypair pk", &pka, &pkb);
    eq("keypair sk", &ska, &skb);
    (pka, ska)
}

fn signed(libs: &Libs, rng: &mut Rng, mlen: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (pk, sk) = keypair(libs, rng);
    let (sc, _sr) = libs.pair::<Signature>("crypto_sign_signature");
    let m = rng.bytes(mlen);
    let mut sig = vec![0u8; SPX_BYTES];
    let mut l = 0usize;
    unsafe {
        assert_eq!(sc(sig.as_mut_ptr(), &mut l, m.as_ptr(), mlen, sk.as_ptr()), 0);
    }
    assert_eq!(l, SPX_BYTES);
    (pk, sig, m)
}

#[test]
fn row05_20_21_verify_wrong_siglen() {
    let libs = load();
    let (vc, vr) = libs.pair::<Verify>("crypto_sign_verify");
    let mut rng = Rng::new(5);
    let (pk, sig, m) = signed(&libs, &mut rng, 64);

    // Every wrong length must be rejected before anything is read; use a short
    // buffer for the small ones so an implementation that hashed first would
    // read out of bounds rather than return.
    let tiny = [0u8; 8];
    for siglen in [0usize, 1, 8] {
        let (ra, rb) = unsafe {
            (
                vc(tiny.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()),
                vr(tiny.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()),
            )
        };
        assert_eq!(ra, -1, "C crypto_sign_verify(siglen={siglen})");
        assert_eq!(rb, -1, "Rust crypto_sign_verify(siglen={siglen})");
    }
    // one step either side of the only accepted length
    let mut big = sig.clone();
    big.push(0);
    for (buf, siglen) in [
        (&sig, SPX_BYTES - 1),
        (&big, SPX_BYTES + 1),
        (&sig, SPX_BYTES / 2),
        (&big, SPX_BYTES * 2),
    ] {
        let (ra, rb) = unsafe {
            (
                vc(buf.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()),
                vr(buf.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()),
            )
        };
        assert_eq!(ra, -1, "C crypto_sign_verify(siglen={siglen})");
        assert_eq!(rb, -1, "Rust crypto_sign_verify(siglen={siglen})");
    }
    // sanity: the correct length still verifies on both sides
    unsafe {
        assert_eq!(vc(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()), 0);
        assert_eq!(vr(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()), 0);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — crypto_sign_verify's root comparison
// ---------------------------------------------------------------------------

#[test]
fn row06_verify_root_mismatch() {
    let libs = load();
    let (vc, vr) = libs.pair::<Verify>("crypto_sign_verify");
    let mut rng = Rng::new(6);
    let (pk, sig, m) = signed(&libs, &mut rng, 32);

    // Flip one bit in the signature, in the message, and in the public key; each
    // must make both sides return -1.
    let flips: Vec<usize> = vec![
        0,
        SPX_N - 1,
        SPX_N,
        SPX_N + SPX_FORS_BYTES / 2,
        SPX_N + SPX_FORS_BYTES,
        SPX_BYTES / 2,
        SPX_BYTES - 1,
    ];
    for off in flips {
        let mut bad = sig.clone();
        bad[off] ^= 0x01;
        let (ra, rb) = unsafe {
            (
                vc(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()),
                vr(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()),
            )
        };
        assert_eq!(ra, -1, "C accepted a signature corrupted at byte {off}");
        assert_eq!(rb, -1, "Rust accepted a signature corrupted at byte {off}");
    }
    // Message corruption.  `hash_blake.c` passes a byte count to
    // `blakeX_update()`, which interprets it as a *bit* count, so under the
    // blake backend only the first `mlen/8` bytes of the message reach the
    // digest.  Restrict the flips to that prefix so the case is a genuine
    // rejection in every backend, and require C and Rust to agree regardless.
    let hashed_prefix = if cfg!(backend_blake) { m.len() / 8 } else { m.len() };
    for off in 0..hashed_prefix.min(4) {
        let mut badm = m.clone();
        badm[off] ^= 0x80;
        let (ra, rb) = unsafe {
            (
                vc(sig.as_ptr(), SPX_BYTES, badm.as_ptr(), badm.len(), pk.as_ptr()),
                vr(sig.as_ptr(), SPX_BYTES, badm.as_ptr(), badm.len(), pk.as_ptr()),
            )
        };
        assert_eq!(ra, rb, "verify disagrees on message flip at {off}");
        assert_eq!(ra, -1, "C accepted a message corrupted at byte {off}");
    }
    for off in [0usize, SPX_N - 1, SPX_N, SPX_PK_BYTES - 1] {
        let mut badpk = pk.clone();
        badpk[off] ^= 0x01;
        let (ra, rb) = unsafe {
            (
                vc(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), badpk.as_ptr()),
                vr(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), badpk.as_ptr()),
            )
        };
        assert_eq!(ra, -1, "C accepted with pk corrupted at {off}");
        assert_eq!(rb, -1, "Rust accepted with pk corrupted at {off}");
    }
    // wrong message length
    let (ra, rb) = unsafe {
        (
            vc(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len() - 1, pk.as_ptr()),
            vr(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len() - 1, pk.as_ptr()),
        )
    };
    assert_eq!(ra, -1);
    assert_eq!(rb, -1);
}

// ---------------------------------------------------------------------------
// Rows 7, 22, 23, 24, 25, 26 — crypto_sign_open
// ---------------------------------------------------------------------------

#[test]
fn row07_22_24_open_short_smlen() {
    let libs = load();
    let (oc, or) = libs.pair::<SignOpen>("crypto_sign_open");
    let mut rng = Rng::new(7);
    let (pk, sig, _m) = signed(&libs, &mut rng, 16);

    for smlen in [0u64, 1, 16, (SPX_BYTES / 2) as u64, (SPX_BYTES - 1) as u64] {
        // m must be at least smlen bytes: the C memsets that many.
        let mut ma = vec![0xEEu8; smlen as usize + 8];
        let mut mb = vec![0xEEu8; smlen as usize + 8];
        let mut la = 0xDEADu64;
        let mut lb = 0xDEADu64;
        let (ra, rb) = unsafe {
            (
                oc(ma.as_mut_ptr(), &mut la, sig.as_ptr(), smlen, pk.as_ptr()),
                or(mb.as_mut_ptr(), &mut lb, sig.as_ptr(), smlen, pk.as_ptr()),
            )
        };
        assert_eq!(ra, -1, "C crypto_sign_open(smlen={smlen})");
        assert_eq!(rb, -1, "Rust crypto_sign_open(smlen={smlen})");
        assert_eq!(la, 0, "C did not zero *mlen (smlen={smlen})");
        assert_eq!(lb, 0, "Rust did not zero *mlen (smlen={smlen})");
        eq(&format!("crypto_sign_open m (smlen={smlen})"), &ma, &mb);
        // rows 7/22/24: exactly smlen bytes zeroed, the sentinel tail intact
        assert!(ma[..smlen as usize].iter().all(|&x| x == 0));
        assert!(ma[smlen as usize..].iter().all(|&x| x == 0xEE));
    }
}

#[test]
fn row23_open_smlen_exactly_spx_bytes() {
    let libs = load();
    let (gc, gr) = libs.pair::<Sign>("crypto_sign");
    let (oc, or) = libs.pair::<SignOpen>("crypto_sign_open");
    let mut rng = Rng::new(23);
    let (pk, sk) = keypair(&libs, &mut rng);
    // zero-length message: smlen == SPX_BYTES, the smallest accepted value
    let mut sm = vec![0u8; SPX_BYTES];
    let mut slen = 0u64;
    unsafe {
        assert_eq!(
            gc(sm.as_mut_ptr(), &mut slen, sm.as_ptr(), 0, sk.as_ptr()),
            0
        );
    }
    assert_eq!(slen, SPX_BYTES as u64);
    let mut ma = vec![0xEEu8; 8];
    let mut mb = vec![0xEEu8; 8];
    let mut la = 0xDEADu64;
    let mut lb = 0xDEADu64;
    let (ra, rb) = unsafe {
        (
            oc(ma.as_mut_ptr(), &mut la, sm.as_ptr(), slen, pk.as_ptr()),
            or(mb.as_mut_ptr(), &mut lb, sm.as_ptr(), slen, pk.as_ptr()),
        )
    };
    assert_eq!(ra, 0, "C rejected a valid zero-length message");
    assert_eq!(rb, 0, "Rust rejected a valid zero-length message");
    assert_eq!(la, 0);
    assert_eq!(lb, 0);
    eq("m untouched for an empty message", &ma, &mb);
    let _ = gr;
}

#[test]
fn row08_25_26_open_invalid_signature() {
    let libs = load();
    let (gc, _gr) = libs.pair::<Sign>("crypto_sign");
    let (oc, or) = libs.pair::<SignOpen>("crypto_sign_open");
    let mut rng = Rng::new(8);
    let (pk, sk) = keypair(&libs, &mut rng);
    let mlen = 40usize;
    let m = rng.bytes(mlen);
    let mut sm = vec![0u8; SPX_BYTES + mlen];
    let mut slen = 0u64;
    unsafe {
        assert_eq!(
            gc(sm.as_mut_ptr(), &mut slen, m.as_ptr(), mlen as u64, sk.as_ptr()),
            0
        );
    }
    assert_eq!(slen, (SPX_BYTES + mlen) as u64);

    // Row 25: corrupt the signature part.  Row 26: corrupt the public key.
    // Row 8: in both cases the whole smlen-byte output must be zeroed and
    // *mlen set to 0, not just the mlen bytes the message would occupy.
    //
    // Note on message corruption: `hash_blake.c` passes *byte* counts to
    // `blakeX_update()`, which takes **bits**, so under the blake backend only
    // the first `mlen/8` bytes of the message reach the digest and flipping a
    // later byte legitimately still verifies.  That C behaviour is the ground
    // truth, so message-corruption cases only require the two implementations
    // to agree; the signature and public key cases must reject outright.
    let mut cases: Vec<(Vec<u8>, Vec<u8>, String, bool)> = Vec::new();
    for off in [0usize, SPX_N, SPX_BYTES / 2, SPX_BYTES - 1] {
        let mut bad = sm.clone();
        bad[off] ^= 0x01;
        cases.push((bad, pk.clone(), format!("corrupt sig at {off}"), true));
    }
    for off in [0usize, 1, mlen / 2, mlen - 1] {
        let mut bad = sm.clone();
        bad[SPX_BYTES + off] ^= 0x40;
        cases.push((
            bad,
            pk.clone(),
            format!("corrupt message byte {off}"),
            // only the first mlen/8 bytes are hashed under the blake backend
            !cfg!(backend_blake) || off < mlen / 8,
        ));
    }
    for off in [0usize, SPX_N, SPX_PK_BYTES - 1] {
        let mut badpk = pk.clone();
        badpk[off] ^= 0x01;
        cases.push((sm.clone(), badpk, format!("corrupt pk at {off}"), true));
    }

    for (ci, (bad_sm, use_pk, what, must_reject)) in cases.into_iter().enumerate() {
        let mut ma = vec![0xEEu8; slen as usize + 8];
        let mut mb = vec![0xEEu8; slen as usize + 8];
        let mut la = 0xDEADu64;
        let mut lb = 0xDEADu64;
        let (ra, rb) = unsafe {
            (
                oc(ma.as_mut_ptr(), &mut la, bad_sm.as_ptr(), slen, use_pk.as_ptr()),
                or(mb.as_mut_ptr(), &mut lb, bad_sm.as_ptr(), slen, use_pk.as_ptr()),
            )
        };
        assert_eq!(ra, rb, "C and Rust disagree on {what} (case {ci})");
        assert_eq!(la, lb, "*mlen differs after {what}");
        eq(&format!("crypto_sign_open m after {what}"), &ma, &mb);
        if must_reject {
            assert_eq!(ra, -1, "C accepted: {what} (case {ci})");
            assert_eq!(la, 0, "*mlen after {what}");
            // row 8: the failure path memsets smlen bytes, not *mlen bytes
            assert!(
                ma[..slen as usize].iter().all(|&x| x == 0),
                "C did not zero all smlen bytes after {what}"
            );
            assert!(
                ma[slen as usize..].iter().all(|&x| x == 0xEE),
                "C wrote past smlen after {what}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17, 18, 19 — the DRBG's zero-length and null-pointer branches
// ---------------------------------------------------------------------------

#[test]
fn row17_randombytes_zero_length() {
    let libs = load();
    let (ic, ir) = libs.pair::<RandombytesInit>("randombytes_init");
    let (rc, rr) = libs.pair::<Randombytes>("randombytes");
    let dc = libs.c::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let dr = libs.r::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let mut ent = [0x11u8; 48];
    unsafe {
        ic(ent.as_mut_ptr(), core::ptr::null_mut());
        ir(ent.as_mut_ptr(), core::ptr::null_mut());
        let before_c = (**dc).as_bytes().to_vec();
        // A zero-length draw is not a no-op: the C still runs the reseed update
        // and increments reseed_counter.
        let ra = rc(core::ptr::null_mut(), 0);
        assert_eq!(ra, RNG_SUCCESS);
        let after_c = (**dc).as_bytes().to_vec();
        assert_ne!(before_c, after_c, "C randombytes(_, 0) left the DRBG alone");
        if DRBG_DETERMINISTIC {
            let rb = rr(core::ptr::null_mut(), 0);
            assert_eq!(rb, RNG_SUCCESS);
            eq(
                "DRBG_ctx after randombytes(_, 0)",
                (**dc).as_bytes(),
                (**dr).as_bytes(),
            );
        }
    }
}

#[test]
fn row18_randombytes_init_null_personalization() {
    let libs = load();
    let (ic, ir) = libs.pair::<RandombytesInit>("randombytes_init");
    let dc = libs.c::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let dr = libs.r::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let mut rng = Rng::new(18);
    for _ in 0..32 {
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        let mut e2 = e;
        unsafe {
            ic(e.as_mut_ptr(), core::ptr::null_mut());
            ir(e2.as_mut_ptr(), core::ptr::null_mut());
            eq(
                "DRBG_ctx after randombytes_init(_, NULL)",
                (**dc).as_bytes(),
                (**dr).as_bytes(),
            );
        }
        // ... and a zero personalization string must give the same result as
        // NULL, since XOR with zero is the identity.
        let mut zero = [0u8; 48];
        let mut e3 = e;
        let mut e4 = e;
        unsafe {
            ic(e3.as_mut_ptr(), zero.as_mut_ptr());
            let with_zero = (**dc).as_bytes().to_vec();
            ir(e4.as_mut_ptr(), zero.as_mut_ptr());
            eq("DRBG_ctx with zero personalization", &with_zero, (**dr).as_bytes());
        }
    }
}

#[test]
fn row19_drbg_update_null_provided_data() {
    let libs = load();
    let (fc, fr) = libs.pair::<DrbgUpdate>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(19);
    for _ in 0..64 {
        let mut key = [0u8; 32];
        rng.fill(&mut key);
        let mut v = [0u8; 16];
        rng.fill(&mut v);
        let mut ka = key;
        let mut kb = key;
        let mut va = v;
        let mut vb = v;
        unsafe {
            fc(core::ptr::null_mut(), ka.as_mut_ptr(), va.as_mut_ptr());
            fr(core::ptr::null_mut(), kb.as_mut_ptr(), vb.as_mut_ptr());
        }
        eq("AES256_CTR_DRBG_Update(NULL) Key", &ka, &kb);
        eq("AES256_CTR_DRBG_Update(NULL) V", &va, &vb);

        // NULL must differ from an all-zero provided_data only in that the XOR
        // is skipped, which for zeros is the same thing.
        let mut zero = [0u8; 48];
        let mut kc = key;
        let mut kd = key;
        let mut vc2 = v;
        let mut vd = v;
        unsafe {
            fc(zero.as_mut_ptr(), kc.as_mut_ptr(), vc2.as_mut_ptr());
            fr(zero.as_mut_ptr(), kd.as_mut_ptr(), vd.as_mut_ptr());
        }
        eq("AES256_CTR_DRBG_Update(zeros) Key", &kc, &kd);
        assert_eq!(ka, kc, "NULL and all-zero provided_data must agree");
    }
}

// ---------------------------------------------------------------------------
// Rows 27, 28 — out-of-range values through the address setters
// ---------------------------------------------------------------------------

#[test]
fn row27_28_out_of_range_address_values() {
    let libs = load();
    let mut rng = Rng::new(27);
    // C enums accept any int; there is no SPX_ADDR_TYPE_* variant above 6, and
    // set_type simply truncates to a byte.  Same for the other single-byte
    // setters and for the four-byte ones at their extremes.
    let mut vals: Vec<u32> = (0u32..=300).collect();
    vals.extend_from_slice(&[
        0x0FFF, 0x1000, 0xFFFF, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FF07, 0xFFFF_FFFF,
    ]);
    for name in [
        "SPX_set_type",
        "SPX_set_layer_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
        "SPX_set_keypair_addr",
        "SPX_set_tree_index",
    ] {
        let (fc, fr) = libs.pair::<SetU32>(name);
        for &v in &vals {
            let start = rng.addr();
            let mut a = start;
            let mut b = start;
            unsafe {
                fc(a.as_mut_ptr(), v);
                fr(b.as_mut_ptr(), v);
            }
            eq(
                &format!("{name}({v:#x}) out-of-range"),
                &u32s_as_bytes(&a),
                &u32s_as_bytes(&b),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 29 — thash's unchecked inblocks
// ---------------------------------------------------------------------------

#[test]
fn row29_thash_unchecked_inblocks() {
    let libs = load();
    let (fc, fr) = libs.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(29);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for inblocks in [
        0u32,
        1,
        THASH_MAX_INTERNAL as u32,
        THASH_MAX_INTERNAL as u32 + 1,
        THASH_MAX_INTERNAL as u32 + 2,
        128,
        257,
    ] {
        let inp = rng.bytes(inblocks as usize * SPX_N);
        let mut aa = rng.addr();
        let mut ab = aa;
        let mut a = vec![0xA5u8; SPX_N + 8];
        let mut b = vec![0xA5u8; SPX_N + 8];
        unsafe {
            fc(a.as_mut_ptr(), inp.as_ptr(), inblocks, cc.ptr(), aa.as_mut_ptr());
            fr(b.as_mut_ptr(), inp.as_ptr(), inblocks, cr.ptr(), ab.as_mut_ptr());
        }
        eq(&format!("SPX_thash(inblocks={inblocks})"), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// Row 30 — zero-length loops in the conversion and MGF1 helpers
// ---------------------------------------------------------------------------

#[test]
fn row30_zero_length_helpers() {
    let libs = load();
    let (uc, ur) = libs.pair::<UllToBytes>("SPX_ull_to_bytes");
    let (bc, br) = libs.pair::<BytesToUll>("SPX_bytes_to_ull");

    // outlen == 0 must write nothing at all
    let mut a = [0xEEu8; 8];
    let mut b = [0xEEu8; 8];
    unsafe {
        uc(a.as_mut_ptr(), 0, u64::MAX);
        ur(b.as_mut_ptr(), 0, u64::MAX);
    }
    assert_eq!(a, [0xEEu8; 8], "C ull_to_bytes(outlen=0) wrote");
    assert_eq!(b, [0xEEu8; 8], "Rust ull_to_bytes(outlen=0) wrote");

    // inlen == 0 must read nothing and return 0
    let (va, vb) = unsafe { (bc(core::ptr::null(), 0), br(core::ptr::null(), 0)) };
    assert_eq!(va, 0);
    assert_eq!(vb, 0);

    // outlen == 0 through the backend MGF1 / XOF entry points
    #[cfg(backend_blake)]
    {
        type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let (mc, mr) = libs.pair::<Mgf1>(name);
            let inp = [1u8; 32];
            let mut a = [0xEEu8; 8];
            let mut b = [0xEEu8; 8];
            unsafe {
                mc(a.as_mut_ptr(), 0, inp.as_ptr(), 32);
                mr(b.as_mut_ptr(), 0, inp.as_ptr(), 32);
            }
            assert_eq!(a, [0xEEu8; 8], "C {name}(outlen=0) wrote");
            assert_eq!(b, [0xEEu8; 8], "Rust {name}(outlen=0) wrote");
        }
    }
    #[cfg(backend_sha2)]
    {
        type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
        for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let (mc, mr) = libs.pair::<Mgf1>(name);
            let inp = [1u8; 32];
            let mut a = [0xEEu8; 8];
            let mut b = [0xEEu8; 8];
            unsafe {
                mc(a.as_mut_ptr(), 0, inp.as_ptr(), 32);
                mr(b.as_mut_ptr(), 0, inp.as_ptr(), 32);
            }
            assert_eq!(a, [0xEEu8; 8], "C {name}(outlen=0) wrote");
            assert_eq!(b, [0xEEu8; 8], "Rust {name}(outlen=0) wrote");
        }
    }
    #[cfg(backend_shake)]
    {
        type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
        let (mc, mr) = libs.pair::<Shake>("shake256");
        let inp = [1u8; 32];
        let mut a = [0xEEu8; 8];
        let mut b = [0xEEu8; 8];
        unsafe {
            mc(a.as_mut_ptr(), 0, inp.as_ptr(), 32);
            mr(b.as_mut_ptr(), 0, inp.as_ptr(), 32);
        }
        assert_eq!(a, [0xEEu8; 8], "C shake256(outlen=0) wrote");
        assert_eq!(b, [0xEEu8; 8], "Rust shake256(outlen=0) wrote");
        // and inlen == 0
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        unsafe {
            mc(a.as_mut_ptr(), 32, core::ptr::null(), 0);
            mr(b.as_mut_ptr(), 32, core::ptr::null(), 0);
        }
        eq("shake256(inlen=0)", &a, &b);
    }
    #[cfg(backend_haraka)]
    {
        type HarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
        let mut rng = Rng::new(30);
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let (mc, mr) = libs.pair::<HarakaS>("SPX_haraka_S");
        let inp = [1u8; 32];
        let mut a = [0xEEu8; 8];
        let mut b = [0xEEu8; 8];
        unsafe {
            mc(a.as_mut_ptr(), 0, inp.as_ptr(), 32, cc.ptr());
            mr(b.as_mut_ptr(), 0, inp.as_ptr(), 32, cr.ptr());
        }
        assert_eq!(a, [0xEEu8; 8], "C SPX_haraka_S(outlen=0) wrote");
        assert_eq!(b, [0xEEu8; 8], "Rust SPX_haraka_S(outlen=0) wrote");
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        unsafe {
            mc(a.as_mut_ptr(), 32, core::ptr::null(), 0, cc.ptr());
            mr(b.as_mut_ptr(), 32, core::ptr::null(), 0, cr.ptr());
        }
        eq("SPX_haraka_S(inlen=0)", &a, &b);
    }
}
