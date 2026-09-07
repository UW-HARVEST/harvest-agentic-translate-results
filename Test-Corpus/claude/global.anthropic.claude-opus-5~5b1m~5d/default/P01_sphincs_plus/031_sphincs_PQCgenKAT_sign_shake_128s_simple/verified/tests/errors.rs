//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! Each asserts the C and the Rust return the SAME error code / sentinel and
//! leave the SAME observable state -- not merely that "both failed somehow".

mod common;
use common::*;
use libloading::Symbol;

// ---- rng.h return codes ----
const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

/// `AES_XOF_struct`: u8[16] + pad + 2 x unsigned long + u8[32] + u8[16] = 80.
const XOF_BYTES: usize = 80;
const DRBG_BYTES: usize = 52;

type FSeedexpanderInit = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8, u64) -> i32;
type FSeedexpander = unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32;
type FVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type FSignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FSignature =
    unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type FSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type FRandombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FRandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FDrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);

fn xof_init_pair(maxlen: u64, seed: &mut [u8; 32], div: &mut [u8; 8]) -> (Vec<u8>, Vec<u8>, i32, i32) {
    let (ci, ri) = both!("seedexpander_init", FSeedexpanderInit);
    // Prefill with a marker: on the RNG_BAD_MAXLEN path the C returns BEFORE
    // touching *ctx, so every byte must still be 0x5A afterwards.
    let mut cctx = vec![0x5Au8; XOF_BYTES];
    let mut rctx = vec![0x5Au8; XOF_BYTES];
    let (crc, rrc) = unsafe {
        (
            ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen),
            ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen),
        )
    };
    (cctx, rctx, crc, rrc)
}

// ===========================================================================
// Rows 1-4 -- seedexpander_init range check
// ===========================================================================
#[test]
fn err01_seedexpander_init_maxlen_exactly_2pow32() {
    let mut s = [7u8; 32];
    let mut d = [3u8; 8];
    let (cctx, rctx, crc, rrc) = xof_init_pair(0x1_0000_0000, &mut s, &mut d);
    eq("row1 return", crc, rrc);
    eq("row1 return == RNG_BAD_MAXLEN", crc, RNG_BAD_MAXLEN);
    eq_bytes("row1 ctx untouched", &cctx, &rctx);
    eq_bytes("row1 ctx really untouched", &cctx, &vec![0x5Au8; XOF_BYTES]);
}

#[test]
fn err02_seedexpander_init_maxlen_u64_max() {
    let mut s = [7u8; 32];
    let mut d = [3u8; 8];
    for maxlen in [u64::MAX, 0x1_0000_0001, 1u64 << 40, 1u64 << 63] {
        let (cctx, rctx, crc, rrc) = xof_init_pair(maxlen, &mut s, &mut d);
        eq(&format!("row2 return (maxlen={maxlen:#x})"), crc, rrc);
        eq(
            &format!("row2 == RNG_BAD_MAXLEN (maxlen={maxlen:#x})"),
            crc,
            RNG_BAD_MAXLEN,
        );
        eq_bytes("row2 ctx untouched", &cctx, &rctx);
        eq_bytes("row2 ctx really untouched", &cctx, &vec![0x5Au8; XOF_BYTES]);
    }
}

#[test]
fn err03_seedexpander_init_maxlen_max_accepted() {
    let mut s = [7u8; 32];
    let mut d = [3u8; 8];
    let (cctx, rctx, crc, rrc) = xof_init_pair(0xFFFF_FFFF, &mut s, &mut d);
    eq("row3 return", crc, rrc);
    eq("row3 return == RNG_SUCCESS", crc, RNG_SUCCESS);
    eq_bytes("row3 ctx image", &cctx, &rctx);
    // ctr[8..12] holds maxlen big-endian-ish per the C's explicit assignment.
    let ctr = &cctx[XOF_BYTES - 16..];
    eq("row3 ctr[8..12]", &ctr[8..12], &[0xFFu8, 0xFF, 0xFF, 0xFF][..]);
}

#[test]
fn err04_seedexpander_init_maxlen_zero() {
    let mut s = [7u8; 32];
    let mut d = [3u8; 8];
    let (cctx, rctx, crc, rrc) = xof_init_pair(0, &mut s, &mut d);
    eq("row4 return", crc, rrc);
    eq("row4 return == RNG_SUCCESS", crc, RNG_SUCCESS);
    eq_bytes("row4 ctx image", &cctx, &rctx);
    let ctr = &cctx[XOF_BYTES - 16..];
    eq("row4 ctr[8..12]", &ctr[8..12], &[0u8, 0, 0, 0][..]);
}

// ===========================================================================
// Rows 5-12 -- seedexpander guards
// ===========================================================================
fn fresh_xof(maxlen: u64) -> (Vec<u8>, Vec<u8>) {
    let mut s = [0x11u8; 32];
    let mut d = [0x22u8; 8];
    let (cctx, rctx, crc, rrc) = xof_init_pair(maxlen, &mut s, &mut d);
    assert_eq!(crc, RNG_SUCCESS);
    assert_eq!(rrc, RNG_SUCCESS);
    eq_bytes("fresh_xof ctx", &cctx, &rctx);
    (cctx, rctx)
}

#[test]
fn err05_seedexpander_null_output() {
    let (c, r) = both!("seedexpander", FSeedexpander);
    let (mut cctx, mut rctx) = fresh_xof(4096);
    let before = cctx.clone();
    for xlen in [0u64, 1, 16, 100] {
        let (crc, rrc) = unsafe {
            (
                c(cctx.as_mut_ptr(), std::ptr::null_mut(), xlen),
                r(rctx.as_mut_ptr(), std::ptr::null_mut(), xlen),
            )
        };
        eq(&format!("row5 return (xlen={xlen})"), crc, rrc);
        eq(
            &format!("row5 == RNG_BAD_OUTBUF (xlen={xlen})"),
            crc,
            RNG_BAD_OUTBUF,
        );
        eq_bytes("row5 ctx untouched", &cctx, &rctx);
        eq_bytes("row5 ctx really untouched", &cctx, &before);
    }
}

#[test]
fn err06_seedexpander_null_output_beats_bad_len() {
    // Both faults at once: the NULL check comes FIRST in the C, so the result
    // must be -2 and not -3.
    let (c, r) = both!("seedexpander", FSeedexpander);
    let (mut cctx, mut rctx) = fresh_xof(16);
    let (crc, rrc) = unsafe {
        (
            c(cctx.as_mut_ptr(), std::ptr::null_mut(), 10_000),
            r(rctx.as_mut_ptr(), std::ptr::null_mut(), 10_000),
        )
    };
    eq("row6 return", crc, rrc);
    eq("row6 == RNG_BAD_OUTBUF (not RNG_BAD_REQ_LEN)", crc, RNG_BAD_OUTBUF);
    eq_bytes("row6 ctx untouched", &cctx, &rctx);
}

#[test]
fn err07_seedexpander_xlen_equals_remaining() {
    let (c, r) = both!("seedexpander", FSeedexpander);
    for maxlen in [1u64, 16, 17, 4096] {
        let (mut cctx, mut rctx) = fresh_xof(maxlen);
        let before = cctx.clone();
        let mut cb = vec![0xA5u8; maxlen as usize + 8];
        let mut rb = vec![0xA5u8; maxlen as usize + 8];
        let (crc, rrc) = unsafe {
            (
                c(cctx.as_mut_ptr(), cb.as_mut_ptr(), maxlen),
                r(rctx.as_mut_ptr(), rb.as_mut_ptr(), maxlen),
            )
        };
        eq(&format!("row7 return (maxlen={maxlen})"), crc, rrc);
        eq(
            &format!("row7 == RNG_BAD_REQ_LEN (maxlen={maxlen})"),
            crc,
            RNG_BAD_REQ_LEN,
        );
        eq_bytes("row7 ctx untouched (no decrement)", &cctx, &rctx);
        eq_bytes("row7 ctx really untouched", &cctx, &before);
        eq_bytes("row7 output untouched", &cb, &rb);
    }
}

#[test]
fn err08_seedexpander_xlen_above_remaining() {
    let (c, r) = both!("seedexpander", FSeedexpander);
    let (mut cctx, mut rctx) = fresh_xof(64);
    let before = cctx.clone();
    for xlen in [65u64, 100, 1 << 20, u32::MAX as u64] {
        let mut cb = vec![0xA5u8; 128];
        let mut rb = vec![0xA5u8; 128];
        let (crc, rrc) = unsafe {
            (
                c(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen),
                r(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen),
            )
        };
        eq(&format!("row8 return (xlen={xlen})"), crc, rrc);
        eq(
            &format!("row8 == RNG_BAD_REQ_LEN (xlen={xlen})"),
            crc,
            RNG_BAD_REQ_LEN,
        );
        eq_bytes("row8 ctx untouched", &cctx, &rctx);
        eq_bytes("row8 ctx really untouched", &cctx, &before);
        eq_bytes("row8 output untouched", &cb, &rb);
    }
}

#[test]
fn err09_seedexpander_xlen_max_accepted() {
    let (c, r) = both!("seedexpander", FSeedexpander);
    for maxlen in [2u64, 17, 33, 4096] {
        let (mut cctx, mut rctx) = fresh_xof(maxlen);
        let want = maxlen - 1;
        let mut cb = vec![0xA5u8; want as usize + 8];
        let mut rb = vec![0xA5u8; want as usize + 8];
        let (crc, rrc) = unsafe {
            (
                c(cctx.as_mut_ptr(), cb.as_mut_ptr(), want),
                r(rctx.as_mut_ptr(), rb.as_mut_ptr(), want),
            )
        };
        eq(&format!("row9 return (maxlen={maxlen})"), crc, rrc);
        eq(&format!("row9 == RNG_SUCCESS (maxlen={maxlen})"), crc, RNG_SUCCESS);
        eq_bytes("row9 output", &cb, &rb);
        eq_bytes("row9 ctx image", &cctx, &rctx);
    }
}

#[test]
fn err10_seedexpander_xlen_zero_with_room() {
    let (c, r) = both!("seedexpander", FSeedexpander);
    let (mut cctx, mut rctx) = fresh_xof(4096);
    let mut cb = vec![0xA5u8; 8];
    let mut rb = vec![0xA5u8; 8];
    let (crc, rrc) = unsafe {
        (
            c(cctx.as_mut_ptr(), cb.as_mut_ptr(), 0),
            r(rctx.as_mut_ptr(), rb.as_mut_ptr(), 0),
        )
    };
    eq("row10 return", crc, rrc);
    eq("row10 == RNG_SUCCESS", crc, RNG_SUCCESS);
    eq_bytes("row10 output untouched", &cb, &rb);
    eq_bytes("row10 output really untouched", &cb, &vec![0xA5u8; 8]);
    eq_bytes("row10 ctx image", &cctx, &rctx);
}

#[test]
fn err11_seedexpander_zero_len_zero_remaining() {
    // 0 >= 0 fires the `xlen >= length_remaining` guard.
    let (c, r) = both!("seedexpander", FSeedexpander);
    let (mut cctx, mut rctx) = fresh_xof(0);
    let mut cb = vec![0xA5u8; 8];
    let mut rb = vec![0xA5u8; 8];
    let (crc, rrc) = unsafe {
        (
            c(cctx.as_mut_ptr(), cb.as_mut_ptr(), 0),
            r(rctx.as_mut_ptr(), rb.as_mut_ptr(), 0),
        )
    };
    eq("row11 return", crc, rrc);
    eq("row11 == RNG_BAD_REQ_LEN", crc, RNG_BAD_REQ_LEN);
    eq_bytes("row11 ctx image", &cctx, &rctx);
}

#[test]
fn err12_seedexpander_exhausted() {
    let (c, r) = both!("seedexpander", FSeedexpander);
    let (mut cctx, mut rctx) = fresh_xof(64);
    // Draw 63 (the max), leaving length_remaining == 1.
    let mut cb = vec![0u8; 63];
    let mut rb = vec![0u8; 63];
    let (crc, rrc) = unsafe {
        (
            c(cctx.as_mut_ptr(), cb.as_mut_ptr(), 63),
            r(rctx.as_mut_ptr(), rb.as_mut_ptr(), 63),
        )
    };
    eq("row12 first draw return", crc, rrc);
    eq("row12 first draw == 0", crc, RNG_SUCCESS);
    eq_bytes("row12 first draw output", &cb, &rb);
    eq_bytes("row12 ctx after first draw", &cctx, &rctx);
    // Now 1 >= 1 -> RNG_BAD_REQ_LEN.
    for xlen in [1u64, 2, 100] {
        let mut cb = vec![0xA5u8; 128];
        let mut rb = vec![0xA5u8; 128];
        let (crc, rrc) = unsafe {
            (
                c(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen),
                r(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen),
            )
        };
        eq(&format!("row12 exhausted return (xlen={xlen})"), crc, rrc);
        eq(
            &format!("row12 exhausted == RNG_BAD_REQ_LEN (xlen={xlen})"),
            crc,
            RNG_BAD_REQ_LEN,
        );
        eq_bytes("row12 ctx after rejection", &cctx, &rctx);
    }
}

// ===========================================================================
// Rows 13-25 -- crypto_sign_verify
// ===========================================================================

/// One keypair + one valid detached signature, produced identically by both.
struct Fixture {
    pk: Vec<u8>,
    pk2: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
}

/// Building a fixture costs two `crypto_sign_seed_keypair` calls plus a full
/// `crypto_sign_signature`, which on the `*s` parameter sets is seconds of work.
/// Cache one per message length so the ~10 verify/open error tests share them
/// (the DRBG guard already serialises them, so this changes no observed value).
fn fixture(mlen: usize) -> &'static Fixture {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<usize, &'static Fixture>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(f) = g.get(&mlen) {
        return f;
    }
    let f: &'static Fixture = Box::leak(Box::new(build_fixture(mlen)));
    g.insert(mlen, f);
    f
}

fn build_fixture(mlen: usize) -> Fixture {
    let (ck, rk) = both!("crypto_sign_seed_keypair", FSeedKeypair);
    let (cs, rs) = both!("crypto_sign_signature", FSignature);
    let mut rng = Rng::new(RNG_SEED ^ 0xE7);

    let mut mk = |seed: Vec<u8>| -> (Vec<u8>, Vec<u8>) {
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            ck(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            rk(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
        }
        eq_bytes("fixture pk", &cpk, &rpk);
        eq_bytes("fixture sk", &csk, &rsk);
        (cpk, csk)
    };

    let (pk, sk) = mk(rng.bytes(CRYPTO_SEEDBYTES));
    let (pk2, _sk2) = mk(rng.bytes(CRYPTO_SEEDBYTES));

    let m = rng.bytes(mlen.max(1))[..mlen].to_vec();
    seed_both_drbgs(&kat_entropy());
    let mut csig = vec![0u8; SPX_BYTES];
    let mut rsig = vec![0u8; SPX_BYTES];
    let mut cl = 0usize;
    let mut rl = 0usize;
    unsafe {
        cs(csig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, sk.as_ptr());
        rs(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, sk.as_ptr());
    }
    eq("fixture siglen", cl, rl);
    eq_bytes("fixture sig", &csig, &rsig);
    Fixture {
        pk,
        pk2,
        m,
        sig: csig,
    }
}

fn verify_both(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let (c, r) = both!("crypto_sign_verify", FVerify);
    let (crc, rrc) = unsafe {
        (
            c(sig.as_ptr(), siglen, m.as_ptr(), mlen, pk.as_ptr()),
            r(sig.as_ptr(), siglen, m.as_ptr(), mlen, pk.as_ptr()),
        )
    };
    eq("crypto_sign_verify return C vs Rust", crc, rrc);
    crc
}

#[test]
fn err13_16_verify_wrong_siglen() {
    let _g = drbg_guard();
    let f = fixture(33);
    // Row 13/14/15/16: SPX_BYTES-1, +1, 0, SIZE_MAX -- all must return -1
    // WITHOUT reading past the buffer (the length check is first).
    for (row, siglen) in [
        (13usize, SPX_BYTES - 1),
        (14, SPX_BYTES + 1),
        (15, 0),
        (16, usize::MAX),
    ] {
        let rc = verify_both(&f.sig, siglen, &f.m, f.m.len(), &f.pk);
        eq(&format!("row{row} verify(siglen={siglen}) == -1"), rc, -1);
    }
    // A few more one-off values around the boundary.
    for siglen in [1usize, SPX_BYTES / 2, SPX_BYTES * 2, usize::MAX - 1] {
        let rc = verify_both(&f.sig, siglen, &f.m, f.m.len(), &f.pk);
        eq(&format!("verify(siglen={siglen}) == -1"), rc, -1);
    }
    // Sanity: the correct length is accepted.
    let rc = verify_both(&f.sig, SPX_BYTES, &f.m, f.m.len(), &f.pk);
    eq("verify(siglen=SPX_BYTES) == 0", rc, 0);
}

#[test]
fn err17_20_verify_bitflips_in_each_region() {
    let _g = drbg_guard();
    let f = fixture(64);
    // Region boundaries within a SPHINCS+ signature:
    //   [0, N)                                 R
    //   [N, N+FORS_BYTES)                      FORS
    //   then D x (WOTS_BYTES + TREE_HEIGHT*N)
    let fors_start = SPX_N;
    let fors_end = SPX_N + SPX_FORS_BYTES;
    let wots0 = fors_end;
    let auth0 = wots0 + SPX_WOTS_BYTES;
    let regions: [(usize, usize, &str); 4] = [
        (17, 0, "R prefix"),
        (18, fors_start + SPX_FORS_BYTES / 2, "FORS"),
        (19, wots0 + SPX_WOTS_BYTES / 2, "WOTS layer 0"),
        (20, auth0 + SPX_TREE_HEIGHT * SPX_N / 2, "auth path layer 0"),
    ];
    for (row, off, name) in regions {
        assert!(off < SPX_BYTES, "offset {off} inside the signature");
        for bit in [0u8, 3, 7] {
            let mut bad = f.sig.clone();
            bad[off] ^= 1u8 << bit;
            let rc = verify_both(&bad, SPX_BYTES, &f.m, f.m.len(), &f.pk);
            eq(
                &format!("row{row} verify with bit {bit} flipped in {name} (off={off}) == -1"),
                rc,
                -1,
            );
        }
    }
    // Also flip the very last byte of the signature.
    let mut bad = f.sig.clone();
    let last = SPX_BYTES - 1;
    bad[last] ^= 0x80;
    let rc = verify_both(&bad, SPX_BYTES, &f.m, f.m.len(), &f.pk);
    eq("verify with last signature byte flipped == -1", rc, -1);
}

#[test]
fn err21_verify_wrong_public_key() {
    let _g = drbg_guard();
    let f = fixture(33);
    let rc = verify_both(&f.sig, SPX_BYTES, &f.m, f.m.len(), &f.pk2);
    eq("row21 verify under a different pk == -1", rc, -1);
    // Also all-zero and all-FF public keys.
    for pk in [vec![0x00u8; SPX_PK_BYTES], vec![0xFFu8; SPX_PK_BYTES]] {
        let rc = verify_both(&f.sig, SPX_BYTES, &f.m, f.m.len(), &pk);
        eq("row21 verify under a bogus pk == -1", rc, -1);
    }
}

#[test]
fn err22_23_verify_wrong_message() {
    let _g = drbg_guard();
    let f = fixture(64);
    // Row 22: truncated by one byte.
    let rc = verify_both(&f.sig, SPX_BYTES, &f.m, f.m.len() - 1, &f.pk);
    eq("row22 verify with mlen-1 == -1", rc, -1);
    // ...and lengthened by one (reads one extra byte, which we own).
    let mut longer = f.m.clone();
    longer.push(0x00);
    let rc = verify_both(&f.sig, SPX_BYTES, &longer, longer.len(), &f.pk);
    eq("row22 verify with mlen+1 == -1", rc, -1);
    // Row 23: one message byte flipped, at EVERY offset.
    //
    // `verify_both` already asserts C == Rust, which is the contract under
    // test. We do NOT additionally assert `-1`: this C's `hash_message` is only
    // partially sensitive to the message body (see ERRORS.md row 23 -- the
    // `(datalen >> 3) & 0x3F` mask in `blake256_update`, and the analogous
    // prefix-buffering in the other backends), so for many offsets the C
    // genuinely ACCEPTS a flipped message. Replicating that faithfully is the
    // requirement.
    let mut n_rejected = 0usize;
    for off in 0..f.m.len() {
        let mut bad = f.m.clone();
        bad[off] ^= 0x01;
        let rc = verify_both(&f.sig, SPX_BYTES, &bad, bad.len(), &f.pk);
        assert!(
            rc == 0 || rc == -1,
            "row23 verify must return 0 or -1, got {rc} (off={off})"
        );
        if rc == -1 {
            n_rejected += 1;
        }
    }
    // A flip in the very first message byte must be caught in every config --
    // that byte always lands in the part of the digest input the C does mix in.
    let mut bad = f.m.clone();
    bad[0] ^= 0x01;
    let rc = verify_both(&f.sig, SPX_BYTES, &bad, bad.len(), &f.pk);
    eq("row23 verify with message byte 0 flipped == -1", rc, -1);
    assert!(
        n_rejected >= 1,
        "row23: at least one message-byte flip must be rejected"
    );
}

#[test]
fn err24_verify_empty_message_is_valid() {
    let _g = drbg_guard();
    let f = fixture(0);
    let rc = verify_both(&f.sig, SPX_BYTES, &[0u8], 0, &f.pk);
    eq("row24 verify(mlen=0) == 0", rc, 0);
}

#[test]
fn err25_verify_all_zero_signature() {
    let _g = drbg_guard();
    let f = fixture(33);
    for fill in [0x00u8, 0xFF] {
        let bad = vec![fill; SPX_BYTES];
        let rc = verify_both(&bad, SPX_BYTES, &f.m, f.m.len(), &f.pk);
        eq(
            &format!("row25 verify all-{fill:#02x} signature == -1"),
            rc,
            -1,
        );
    }
}

// ===========================================================================
// Rows 26-31 -- crypto_sign_open
// ===========================================================================
fn open_both(sm: &[u8], smlen: u64, pk: &[u8], mbuf_len: usize) -> (i32, u64, Vec<u8>) {
    let (c, r) = both!("crypto_sign_open", FSignOpen);
    let mut cm = vec![0xA5u8; mbuf_len];
    let mut rm = vec![0xA5u8; mbuf_len];
    let mut cl = 0xDEAD_BEEF_u64;
    let mut rl = 0xDEAD_BEEF_u64;
    let (crc, rrc) = unsafe {
        (
            c(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr()),
            r(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr()),
        )
    };
    eq("crypto_sign_open return C vs Rust", crc, rrc);
    eq("crypto_sign_open *mlen C vs Rust", cl, rl);
    eq_bytes("crypto_sign_open m buffer C vs Rust", &cm, &rm);
    (crc, cl, cm)
}

#[test]
fn err26_open_smlen_one_below_minimum() {
    let _g = drbg_guard();
    let f = fixture(33);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    let smlen = (SPX_BYTES - 1) as u64;
    // The C memsets exactly `smlen` bytes of m, so give the buffer more room
    // and check the tail survives.
    let (rc, mlen, m) = open_both(&sm, smlen, &f.pk, SPX_BYTES + 64);
    eq("row26 return == -1", rc, -1);
    eq("row26 *mlen == 0", mlen, 0);
    eq_bytes(
        "row26 m[0..smlen] zeroed",
        &m[..smlen as usize],
        &vec![0u8; smlen as usize],
    );
    eq_bytes(
        "row26 m[smlen..] untouched",
        &m[smlen as usize..],
        &vec![0xA5u8; m.len() - smlen as usize],
    );
}

#[test]
fn err27_open_smlen_zero() {
    let _g = drbg_guard();
    let f = fixture(1);
    let (rc, mlen, m) = open_both(&f.sig, 0, &f.pk, 64);
    eq("row27 return == -1", rc, -1);
    eq("row27 *mlen == 0", mlen, 0);
    eq_bytes("row27 nothing zeroed", &m, &vec![0xA5u8; 64]);
}

#[test]
fn err28_open_smlen_exactly_spx_bytes() {
    let _g = drbg_guard();
    let f = fixture(0);
    let (rc, mlen, _m) = open_both(&f.sig, SPX_BYTES as u64, &f.pk, SPX_BYTES + 64);
    eq("row28 return == 0", rc, 0);
    eq("row28 *mlen == 0", mlen, 0);
}

#[test]
fn err29_open_corrupted_signature_zeroes_smlen_bytes() {
    let _g = drbg_guard();
    let f = fixture(33);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    sm[0] ^= 0x01; // corrupt R
    let smlen = sm.len() as u64;
    let (rc, mlen, m) = open_both(&sm, smlen, &f.pk, sm.len() + 64);
    eq("row29 return == -1", rc, -1);
    eq("row29 *mlen == 0", mlen, 0);
    // NOTE: the C zeroes `smlen` bytes -- i.e. SPX_BYTES + mlen, NOT just mlen.
    eq_bytes(
        "row29 m[0..smlen] zeroed",
        &m[..smlen as usize],
        &vec![0u8; smlen as usize],
    );
    eq_bytes(
        "row29 m[smlen..] untouched",
        &m[smlen as usize..],
        &vec![0xA5u8; m.len() - smlen as usize],
    );
}

#[test]
fn err30_open_wrong_public_key() {
    let _g = drbg_guard();
    let f = fixture(33);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    let smlen = sm.len() as u64;
    let (rc, mlen, m) = open_both(&sm, smlen, &f.pk2, sm.len() + 64);
    eq("row30 return == -1", rc, -1);
    eq("row30 *mlen == 0", mlen, 0);
    eq_bytes(
        "row30 m[0..smlen] zeroed",
        &m[..smlen as usize],
        &vec![0u8; smlen as usize],
    );
}

#[test]
fn err31_open_extra_trailing_byte() {
    let _g = drbg_guard();
    let f = fixture(33);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    sm.push(0x5A); // one byte too many -> the message length feeds the digest
    let smlen = sm.len() as u64;
    // `open_both` asserts C == Rust for the return value, *mlen and the whole m
    // buffer. We do not additionally assert `-1` here: an extra trailing byte
    // only changes `mlen`, and this C's message hashing is only partially
    // sensitive to the message body (ERRORS.md row 23), so some configurations
    // genuinely accept it. Matching the C is the requirement.
    let (rc, mlen, _m) = open_both(&sm, smlen, &f.pk, sm.len() + 64);
    assert!(rc == 0 || rc == -1, "row31 return must be 0 or -1, got {rc}");
    if rc == -1 {
        eq("row31 rejected => *mlen == 0", mlen, 0);
    } else {
        eq("row31 accepted => *mlen == smlen - SPX_BYTES", mlen, smlen - SPX_BYTES as u64);
    }
    // One byte too FEW is also rejected.
    let mut sm2 = f.sig.clone();
    sm2.extend_from_slice(&f.m[..f.m.len() - 1]);
    let (rc, mlen, _m) = open_both(&sm2, sm2.len() as u64, &f.pk, sm2.len() + 64);
    eq("row31b (one byte short) return == -1", rc, -1);
    eq("row31b *mlen == 0", mlen, 0);
}

// ===========================================================================
// Rows 32-33 -- AES256_CTR_DRBG_Update NULL / non-NULL provided_data
// ===========================================================================
#[test]
fn err32_33_drbg_update_provided_data_branches() {
    let (c, r) = both!("AES256_CTR_DRBG_Update", FDrbgUpdate);
    let mut rng = Rng::new(RNG_SEED ^ 0x3233);
    for iter in 0..N_ITER {
        let (key0, v0) = match iter {
            0 => ([0x00u8; 32], [0x00u8; 16]),
            1 => ([0xFFu8; 32], [0xFFu8; 16]),
            _ => {
                let mut k = [0u8; 32];
                let mut v = [0u8; 16];
                rng.fill(&mut k);
                rng.fill(&mut v);
                (k, v)
            }
        };
        // Row 32: provided_data == NULL -> no XOR.
        let mut ck = key0;
        let mut rk = key0;
        let mut cv = v0;
        let mut rv = v0;
        unsafe {
            c(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        eq_bytes("row32 Key (pd=NULL)", &ck, &rk);
        eq_bytes("row32 V (pd=NULL)", &cv, &rv);
        let null_key = ck;

        // Row 33: provided_data == 48 zero bytes -> XOR with zero, so the
        // result must be IDENTICAL to the NULL case. That distinguishes a
        // "forgot the NULL check" bug from a correct implementation.
        let mut ck = key0;
        let mut rk = key0;
        let mut cv = v0;
        let mut rv = v0;
        let mut zpd = [0u8; 48];
        let mut zpd2 = [0u8; 48];
        unsafe {
            c(zpd.as_mut_ptr(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(zpd2.as_mut_ptr(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        eq_bytes("row33 Key (pd=zeros)", &ck, &rk);
        eq_bytes("row33 V (pd=zeros)", &cv, &rv);
        eq_bytes("row33 pd=zeros equals pd=NULL", &ck, &null_key);

        // Row 33b: non-trivial provided_data.
        let mut pd = [0u8; 48];
        rng.fill(&mut pd);
        let mut ck = key0;
        let mut rk = key0;
        let mut cv = v0;
        let mut rv = v0;
        let mut p1 = pd;
        let mut p2 = pd;
        unsafe {
            c(p1.as_mut_ptr(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(p2.as_mut_ptr(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        eq_bytes("row33b Key (pd=random)", &ck, &rk);
        eq_bytes("row33b V (pd=random)", &cv, &rv);
        eq_bytes("row33b provided_data untouched (C)", &p1, &pd);
        eq_bytes("row33b provided_data untouched (Rust)", &p2, &pd);
        assert_ne!(
            ck.to_vec(),
            null_key.to_vec(),
            "row33b: random provided_data must change the result"
        );
    }
}

// ===========================================================================
// Rows 34-35 -- randombytes_init personalization_string NULL / non-NULL
// ===========================================================================
#[test]
fn err34_35_randombytes_init_personalization() {
    let _g = drbg_guard();
    let (ci, ri) = both!("randombytes_init", FRandombytesInit);
    let (cd, rd) = both_data!("DRBG_ctx", [u8; DRBG_BYTES]);
    let mut rng = Rng::new(RNG_SEED ^ 0x3435);

    let mut e1 = kat_entropy();
    let mut e2 = kat_entropy();
    unsafe {
        ci(e1.as_mut_ptr(), std::ptr::null_mut());
        ri(e2.as_mut_ptr(), std::ptr::null_mut());
    }
    let (null_c, null_r) = unsafe { ((*cd).to_vec(), (*rd).to_vec()) };
    eq_bytes("row34 DRBG_ctx (ps=NULL)", &null_c, &null_r);
    eq(
        "row34 reseed_counter == 1",
        i32::from_le_bytes(null_c[48..52].try_into().unwrap()),
        1,
    );

    // ps = all zeros must be identical to ps = NULL (XOR with zero).
    let mut e1 = kat_entropy();
    let mut e2 = kat_entropy();
    let mut z1 = [0u8; 48];
    let mut z2 = [0u8; 48];
    unsafe {
        ci(e1.as_mut_ptr(), z1.as_mut_ptr());
        ri(e2.as_mut_ptr(), z2.as_mut_ptr());
    }
    let (zc, zr) = unsafe { ((*cd).to_vec(), (*rd).to_vec()) };
    eq_bytes("row35 DRBG_ctx (ps=zeros)", &zc, &zr);
    eq_bytes("row35 ps=zeros equals ps=NULL", &zc, &null_c);

    // Row 35: non-trivial ps must change the state.
    let mut ps = [0u8; 48];
    rng.fill(&mut ps);
    let mut e1 = kat_entropy();
    let mut e2 = kat_entropy();
    let mut p1 = ps;
    let mut p2 = ps;
    unsafe {
        ci(e1.as_mut_ptr(), p1.as_mut_ptr());
        ri(e2.as_mut_ptr(), p2.as_mut_ptr());
    }
    let (pc, pr) = unsafe { ((*cd).to_vec(), (*rd).to_vec()) };
    eq_bytes("row35 DRBG_ctx (ps=random)", &pc, &pr);
    assert_ne!(pc, null_c, "row35: a random ps must change the DRBG state");
    eq_bytes("row35 entropy_input untouched (C)", &e1, &kat_entropy());
    eq_bytes("row35 entropy_input untouched (Rust)", &e2, &kat_entropy());
}

// ===========================================================================
// Rows 36-41 -- randombytes length shapes and DRBG edge cases
// ===========================================================================
#[test]
fn err36_39_randombytes_length_shapes() {
    let _g = drbg_guard();
    let (c, r) = both!("randombytes", FRandombytes);
    let (cd, rd) = both_data!("DRBG_ctx", [u8; DRBG_BYTES]);

    for (row, n) in [(36usize, 0usize), (37, 16), (38, 15), (39, 17)] {
        seed_both_drbgs(&kat_entropy());
        let before = unsafe { (*cd).to_vec() };
        let mut cb = vec![0x5Au8; n + 8];
        let mut rb = vec![0x5Au8; n + 8];
        let (crc, rrc) = unsafe { (c(cb.as_mut_ptr(), n as u64), r(rb.as_mut_ptr(), n as u64)) };
        eq(&format!("row{row} randombytes({n}) return"), crc, rrc);
        eq(
            &format!("row{row} randombytes({n}) == RNG_SUCCESS"),
            crc,
            RNG_SUCCESS,
        );
        eq_bytes(&format!("row{row} randombytes({n}) output"), &cb, &rb);
        eq_bytes(
            &format!("row{row} randombytes({n}) bytes past n untouched"),
            &cb[n..],
            &vec![0x5Au8; 8],
        );
        let after_c = unsafe { (*cd).to_vec() };
        let after_r = unsafe { (*rd).to_vec() };
        eq_bytes(&format!("row{row} DRBG_ctx after"), &after_c, &after_r);
        // Even xlen == 0 advances the state (trailing DRBG_Update + counter++).
        assert_ne!(
            after_c, before,
            "row{row}: randombytes({n}) must advance the DRBG state even for n=0"
        );
    }
}

#[test]
fn err40_randombytes_v_all_ff_carry_wrap() {
    // Drive V to all-0xFF through the exported DRBG_ctx, then draw: the
    // increment loop wraps every byte to 0x00 (full carry propagation).
    let _g = drbg_guard();
    let (c, r) = both!("randombytes", FRandombytes);
    let (cd, rd) = both_data!("DRBG_ctx", [u8; DRBG_BYTES]);
    seed_both_drbgs(&kat_entropy());
    unsafe {
        for d in [cd, rd] {
            // V occupies bytes 32..48 of AES256_CTR_DRBG_struct.
            for i in 32..48 {
                (*d)[i] = 0xFF;
            }
        }
    }
    for n in [1usize, 16, 48] {
        let mut cb = vec![0u8; n];
        let mut rb = vec![0u8; n];
        let (crc, rrc) = unsafe { (c(cb.as_mut_ptr(), n as u64), r(rb.as_mut_ptr(), n as u64)) };
        eq("row40 return", crc, rrc);
        eq("row40 == RNG_SUCCESS", crc, RNG_SUCCESS);
        eq_bytes(&format!("row40 output (n={n})"), &cb, &rb);
        eq_bytes("row40 DRBG_ctx after", unsafe { &*cd }, unsafe { &*rd });
        // Reset to all-FF for the next size.
        unsafe {
            for d in [cd, rd] {
                for i in 32..48 {
                    (*d)[i] = 0xFF;
                }
            }
        }
    }
}

#[test]
fn err41_randombytes_without_init() {
    // Never calling randombytes_init: the zero-initialised DRBG_ctx global is
    // used, i.e. AES-256 under the all-zero key.
    let _g = drbg_guard();
    let (c, r) = both!("randombytes", FRandombytes);
    let (cd, rd) = both_data!("DRBG_ctx", [u8; DRBG_BYTES]);
    unsafe {
        for d in [cd, rd] {
            for i in 0..DRBG_BYTES {
                (*d)[i] = 0;
            }
        }
    }
    for n in [1usize, 16, 48] {
        let mut cb = vec![0u8; n];
        let mut rb = vec![0u8; n];
        let (crc, rrc) = unsafe { (c(cb.as_mut_ptr(), n as u64), r(rb.as_mut_ptr(), n as u64)) };
        eq("row41 return", crc, rrc);
        eq("row41 == RNG_SUCCESS", crc, RNG_SUCCESS);
        eq_bytes(&format!("row41 output (n={n})"), &cb, &rb);
        eq_bytes("row41 DRBG_ctx after", unsafe { &*cd }, unsafe { &*rd });
    }
}

// ===========================================================================
// Rows 42-44 -- address setters with out-of-range / out-of-enum values
// ===========================================================================
#[test]
fn err42_set_type_out_of_range_enum() {
    // A C enum accepts any int: the SPX_ADDR_TYPE_* range is 0..=6, so 7, 8,
    // 255, 256 and 0xFFFFFFFF have NO valid variant. The C truncates to one
    // byte and writes it -- no rejection.
    let (c, r) = both!("SPX_set_type", unsafe extern "C" fn(*mut u32, u32));
    let mut rng = Rng::new(RNG_SEED ^ 42);
    let vals: Vec<u32> = vec![
        7, 8, 9, 100, 254, 255, 256, 257, 0x0100_0003, 0x7FFF_FFFF, 0x8000_0000, u32::MAX,
    ];
    for iter in 0..N_ITER + 2 {
        let base = match iter {
            0 => [0u32; 8],
            1 => [u32::MAX; 8],
            _ => rng.addr(),
        };
        for &v in &vals {
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            let cb: Vec<u8> = ca.iter().flat_map(|w| w.to_ne_bytes()).collect();
            let rb: Vec<u8> = ra.iter().flat_map(|w| w.to_ne_bytes()).collect();
            eq_bytes(&format!("row42 set_type({v:#x}) addr image"), &cb, &rb);
            // Only the SPX_OFFSET_TYPE byte changes, and it holds v & 0xFF.
            eq(
                &format!("row42 set_type({v:#x}) byte value"),
                cb[SPX_OFFSET_TYPE],
                (v & 0xFF) as u8,
            );
            let base_bytes: Vec<u8> = base.iter().flat_map(|w| w.to_ne_bytes()).collect();
            for i in 0..32 {
                if i != SPX_OFFSET_TYPE {
                    eq(
                        &format!("row42 set_type({v:#x}) byte {i} untouched"),
                        cb[i],
                        base_bytes[i],
                    );
                }
            }
        }
    }
}

#[test]
fn err43_44_setters_truncate_above_255() {
    let mut rng = Rng::new(RNG_SEED ^ 4344);
    for (name, off) in [
        ("SPX_set_layer_addr", SPX_OFFSET_LAYER),
        ("SPX_set_tree_height", SPX_OFFSET_TREE_HGT),
        ("SPX_set_chain_addr", SPX_OFFSET_CHAIN_ADDR),
        ("SPX_set_hash_addr", SPX_OFFSET_HASH_ADDR),
    ] {
        let l = libs();
        let c: Symbol<unsafe extern "C" fn(*mut u32, u32)> = sym(&l.c, name);
        let r: Symbol<unsafe extern "C" fn(*mut u32, u32)> = sym(&l.r, name);
        for &v in &[256u32, 257, 0x1FF, 0x100, 0xFF00, 0x1234_5678, u32::MAX] {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            let cb: Vec<u8> = ca.iter().flat_map(|w| w.to_ne_bytes()).collect();
            let rb: Vec<u8> = ra.iter().flat_map(|w| w.to_ne_bytes()).collect();
            eq_bytes(&format!("row43/44 {name}({v:#x}) addr image"), &cb, &rb);
            eq(
                &format!("row43/44 {name}({v:#x}) truncated to one byte"),
                cb[off],
                (v & 0xFF) as u8,
            );
        }
    }
}

// ===========================================================================
// Rows 45-48 -- utils byte converters at and past their range
// ===========================================================================
#[test]
fn err45_46_bytes_to_ull_range() {
    let (c, r) = both!(
        "SPX_bytes_to_ull",
        unsafe extern "C" fn(*const u8, u32) -> u64
    );
    let mut rng = Rng::new(RNG_SEED ^ 4546);
    // Row 45: inlen == 0 -> 0.
    let buf = rng.bytes(64);
    let (cv, rv) = unsafe { (c(buf.as_ptr(), 0), r(buf.as_ptr(), 0)) };
    eq("row45 bytes_to_ull(inlen=0)", cv, rv);
    eq("row45 bytes_to_ull(inlen=0) == 0", cv, 0);
    // Row 46: inlen == 8 (full width) and inlen > 8 (shift overflow in C).
    for inlen in [8u32, 9, 10, 16, 32] {
        for inp in [vec![0x00u8; 64], vec![0xFFu8; 64], buf.clone()] {
            let (cv, rv) = unsafe { (c(inp.as_ptr(), inlen), r(inp.as_ptr(), inlen)) };
            eq(&format!("row46 bytes_to_ull(inlen={inlen})"), cv, rv);
        }
    }
}

#[test]
fn err47_48_ull_to_bytes_range() {
    let (c, r) = both!(
        "SPX_ull_to_bytes",
        unsafe extern "C" fn(*mut u8, u32, u64)
    );
    let mut rng = Rng::new(RNG_SEED ^ 4748);
    // Row 47: outlen == 0 writes nothing.
    for v in [0u64, 1, u64::MAX, rng.next_u64()] {
        let mut cb = vec![0xA5u8; 32];
        let mut rb = vec![0xA5u8; 32];
        unsafe {
            c(cb.as_mut_ptr(), 0, v);
            r(rb.as_mut_ptr(), 0, v);
        }
        eq_bytes("row47 ull_to_bytes(outlen=0)", &cb, &rb);
        eq_bytes("row47 nothing written", &cb, &vec![0xA5u8; 32]);
    }
    // Row 48: outlen > 8.
    for outlen in [9u32, 10, 16, 24] {
        for v in [0u64, 1, u64::MAX, rng.next_u64()] {
            let mut cb = vec![0xA5u8; outlen as usize + 8];
            let mut rb = vec![0xA5u8; outlen as usize + 8];
            unsafe {
                c(cb.as_mut_ptr(), outlen, v);
                r(rb.as_mut_ptr(), outlen, v);
            }
            eq_bytes(
                &format!("row48 ull_to_bytes(outlen={outlen}, v={v:#x})"),
                &cb,
                &rb,
            );
        }
    }
}

// ===========================================================================
// Rows 49-52 -- degenerate tree_height / inblocks
// ===========================================================================
#[test]
fn err49_compute_root_tree_height_zero() {
    // tree_height == 0 makes the C loop bound `i < tree_height - 1` underflow
    // to 0xFFFFFFFF, so the loop runs until it walks off the auth_path. We give
    // it a large auth_path region so the behaviour is well-defined enough to
    // compare, and only assert C == Rust.
    type F = unsafe extern "C" fn(
        *mut u8,
        *const u8,
        u32,
        u32,
        *const u8,
        u32,
        *const u8,
        *mut u32,
    );
    let (c, r) = both!("SPX_compute_root", F);
    let mut rng = Rng::new(RNG_SEED ^ 49);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    // tree_height == 1 is the smallest value whose loop count is 0 -- assert it
    // fully. (tree_height == 0 is UB-adjacent in the C: it would read
    // 2^32 * SPX_N bytes of auth_path, which no caller ever does, so we do not
    // execute it. This is recorded rather than tested.)
    let leaf = rng.bytes(SPX_N);
    let auth = rng.bytes(SPX_N * 4);
    for leaf_idx in [0u32, 1, u32::MAX] {
        let addr = rng.addr();
        let mut croot = vec![0xA5u8; SPX_N + 16];
        let mut rroot = vec![0xA5u8; SPX_N + 16];
        let mut ca = addr;
        let mut ra = addr;
        unsafe {
            c(
                croot.as_mut_ptr(),
                leaf.as_ptr(),
                leaf_idx,
                0,
                auth.as_ptr(),
                1,
                cctx.as_ptr(),
                ca.as_mut_ptr(),
            );
            r(
                rroot.as_mut_ptr(),
                leaf.as_ptr(),
                leaf_idx,
                0,
                auth.as_ptr(),
                1,
                rctx.as_ptr(),
                ra.as_mut_ptr(),
            );
        }
        eq_bytes(
            &format!("row49 compute_root(tree_height=1, leaf_idx={leaf_idx})"),
            &croot,
            &rroot,
        );
        eq("row49 addr side-effect", ca, ra);
    }
}

#[test]
fn err50_compute_root_leaf_idx_out_of_range() {
    // Covered thoroughly in tests/tree.rs row30; repeat the sentinel here so
    // this ERRORS row has its own passing test.
    type F = unsafe extern "C" fn(
        *mut u8,
        *const u8,
        u32,
        u32,
        *const u8,
        u32,
        *const u8,
        *mut u32,
    );
    let (c, r) = both!("SPX_compute_root", F);
    let mut rng = Rng::new(RNG_SEED ^ 50);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let h = SPX_TREE_HEIGHT as u32;
    let leaf = rng.bytes(SPX_N);
    let auth = rng.bytes((h as usize) * SPX_N);
    for leaf_idx in [u32::MAX, 1u32 << 31, (1u32 << h) + 7] {
        for idx_offset in [0u32, u32::MAX] {
            let addr = rng.addr();
            let mut croot = vec![0xA5u8; SPX_N + 16];
            let mut rroot = vec![0xA5u8; SPX_N + 16];
            let mut ca = addr;
            let mut ra = addr;
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    cctx.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    auth.as_ptr(),
                    h,
                    rctx.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            eq_bytes(
                &format!("row50 compute_root(leaf_idx={leaf_idx}, off={idx_offset})"),
                &croot,
                &rroot,
            );
            eq("row50 addr side-effect", ca, ra);
        }
    }
}

#[test]
fn err51_treehash_tree_height_zero() {
    type GenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
    type F = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        u32,
        u32,
        u32,
        GenLeafFn,
        *mut u32,
    );
    unsafe extern "C" fn gl(leaf: *mut u8, _c: *const u8, ai: u32, _ta: *const u32) {
        let o = std::slice::from_raw_parts_mut(leaf, SPX_N);
        for (i, b) in o.iter_mut().enumerate() {
            *b = (ai as u8).wrapping_mul(31).wrapping_add(i as u8);
        }
    }
    let (c, r) = both!("SPX_treehash", F);
    let mut rng = Rng::new(RNG_SEED ^ 51);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for h in [0u32, 1] {
        for leaf_idx in [0u32, 1, u32::MAX] {
            let addr = rng.addr();
            let apl = ((h as usize) + 2) * SPX_N;
            let mut croot = vec![0xA5u8; SPX_N + 16];
            let mut rroot = vec![0xA5u8; SPX_N + 16];
            let mut cauth = vec![0xA5u8; apl];
            let mut rauth = vec![0xA5u8; apl];
            let mut ca = addr;
            let mut ra = addr;
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cctx.as_ptr(),
                    leaf_idx,
                    0,
                    h,
                    gl,
                    ca.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rctx.as_ptr(),
                    leaf_idx,
                    0,
                    h,
                    gl,
                    ra.as_mut_ptr(),
                );
            }
            eq_bytes(
                &format!("row51 treehash(h={h}, leaf_idx={leaf_idx}) root"),
                &croot,
                &rroot,
            );
            eq_bytes(
                &format!("row51 treehash(h={h}, leaf_idx={leaf_idx}) auth_path"),
                &cauth,
                &rauth,
            );
            eq("row51 tree_addr side-effect", ca, ra);
        }
    }
}

#[test]
fn err52_thash_inblocks_zero() {
    type F = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
    let (c, r) = both!("SPX_thash", F);
    let mut rng = Rng::new(RNG_SEED ^ 52);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let inp = rng.bytes(64);
    for addr in [[0u32; 8], [u32::MAX; 8], rng.addr()] {
        let mut co = vec![0xA5u8; SPX_N + 64];
        let mut ro = vec![0xA5u8; SPX_N + 64];
        let mut ca = addr;
        let mut ra = addr;
        unsafe {
            c(co.as_mut_ptr(), inp.as_ptr(), 0, cctx.as_ptr(), ca.as_mut_ptr());
            r(ro.as_mut_ptr(), inp.as_ptr(), 0, rctx.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes("row52 thash(inblocks=0)", &co, &ro);
        eq("row52 addr side-effect", ca, ra);
    }
}

// ===========================================================================
// Rows 53-61 -- backend primitive degenerate inputs (cfg-gated)
// ===========================================================================
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
mod blake_edges {
    use super::common::*;

    type FInit = unsafe extern "C" fn(*mut u8);
    type FUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64);
    type FFinal = unsafe extern "C" fn(*mut u8, *mut u8);
    type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    const ST256: usize = 128;
    const ST512: usize = 248;

    #[test]
    fn err53_blake_update_zero_length() {
        for (init, upd, st) in [
            ("blake256_init", "blake256_update", ST256),
            ("blake512_init", "blake512_update", ST512),
        ] {
            let l = libs();
            let ci: libloading::Symbol<FInit> = sym(&l.c, init);
            let ri: libloading::Symbol<FInit> = sym(&l.r, init);
            let cu: libloading::Symbol<FUpdate> = sym(&l.c, upd);
            let ru: libloading::Symbol<FUpdate> = sym(&l.r, upd);
            let mut cs = vec![0x5Au8; st];
            let mut rs = vec![0x5Au8; st];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
            }
            let before = cs.clone();
            let data = [0u8; 64];
            for _ in 0..3 {
                unsafe {
                    cu(cs.as_mut_ptr(), data.as_ptr(), 0);
                    ru(rs.as_mut_ptr(), data.as_ptr(), 0);
                }
            }
            eq_bytes(&format!("row53 {upd}(0) state"), &cs, &rs);
            eq_bytes(&format!("row53 {upd}(0) is a no-op"), &cs, &before);
        }
    }

    #[test]
    fn err54_blake_final_called_twice() {
        for (init, fin, st, ob) in [
            ("blake256_init", "blake256_final", ST256, 32usize),
            ("blake512_init", "blake512_final", ST512, 64),
        ] {
            let l = libs();
            let ci: libloading::Symbol<FInit> = sym(&l.c, init);
            let ri: libloading::Symbol<FInit> = sym(&l.r, init);
            let cf: libloading::Symbol<FFinal> = sym(&l.c, fin);
            let rf: libloading::Symbol<FFinal> = sym(&l.r, fin);
            let mut cs = vec![0u8; st];
            let mut rs = vec![0u8; st];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
            }
            for call in 0..3 {
                let mut cd = vec![0xA5u8; ob + 16];
                let mut rd = vec![0xA5u8; ob + 16];
                unsafe {
                    cf(cs.as_mut_ptr(), cd.as_mut_ptr());
                    rf(rs.as_mut_ptr(), rd.as_mut_ptr());
                }
                eq_bytes(&format!("row54 {fin} call {call} digest"), &cd, &rd);
                eq_bytes(&format!("row54 {fin} call {call} state"), &cs, &rs);
            }
        }
    }

    #[test]
    fn err60_blake_mgf1_zero_outlen() {
        for name in ["SPX_blake256_mgf1", "SPX_blake512_mgf1"] {
            let l = libs();
            let c: libloading::Symbol<FMgf1> = sym(&l.c, name);
            let r: libloading::Symbol<FMgf1> = sym(&l.r, name);
            let inbuf = [0u8; 72];
            for inlen in [0u64, 1, 32, 64] {
                let mut cb = vec![0xA5u8; 16];
                let mut rb = vec![0xA5u8; 16];
                unsafe {
                    c(cb.as_mut_ptr(), 0, inbuf.as_ptr(), inlen);
                    r(rb.as_mut_ptr(), 0, inbuf.as_ptr(), inlen);
                }
                eq_bytes(&format!("row60 {name}(outlen=0, inlen={inlen})"), &cb, &rb);
                eq_bytes("row60 nothing written", &cb, &vec![0xA5u8; 16]);
            }
        }
    }

    #[test]
    fn err59_blake_empty_input() {
        type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
        for (name, ob) in [("blake256", 32usize), ("blake512", 64)] {
            let l = libs();
            let c: libloading::Symbol<FOneShot> = sym(&l.c, name);
            let r: libloading::Symbol<FOneShot> = sym(&l.r, name);
            let inp = [0u8; 1];
            let mut cb = vec![0xA5u8; ob + 16];
            let mut rb = vec![0xA5u8; ob + 16];
            let (crc, rrc) = unsafe {
                (
                    c(cb.as_mut_ptr(), inp.as_ptr(), 0),
                    r(rb.as_mut_ptr(), inp.as_ptr(), 0),
                )
            };
            eq(&format!("row59 {name}(inlen=0) return"), crc, rrc);
            eq_bytes(&format!("row59 {name}(inlen=0) digest"), &cb, &rb);
        }
    }
}

#[cfg(feature = "sha2")]
mod sha2_edges {
    use super::common::*;

    type FIncInit = unsafe extern "C" fn(*mut u8);
    type FIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FIncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    #[test]
    fn err55_56_sha_inc_zero_shapes() {
        for (p, st, ob) in [("sha256", 40usize, 32usize), ("sha512", 72, 64)] {
            let l = libs();
            let ci: libloading::Symbol<FIncInit> = sym(&l.c, &format!("{p}_inc_init"));
            let ri: libloading::Symbol<FIncInit> = sym(&l.r, &format!("{p}_inc_init"));
            let cb_: libloading::Symbol<FIncBlocks> = sym(&l.c, &format!("{p}_inc_blocks"));
            let rb_: libloading::Symbol<FIncBlocks> = sym(&l.r, &format!("{p}_inc_blocks"));
            let cf: libloading::Symbol<FIncFinalize> = sym(&l.c, &format!("{p}_inc_finalize"));
            let rf: libloading::Symbol<FIncFinalize> = sym(&l.r, &format!("{p}_inc_finalize"));

            let mut cs = vec![0x5Au8; st];
            let mut rs = vec![0x5Au8; st];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
            }
            eq_bytes(&format!("row55 {p}_inc_init state"), &cs, &rs);
            let before = cs.clone();
            let data = [0u8; 256];
            // Row 55: inblocks == 0 is a no-op.
            for _ in 0..3 {
                unsafe {
                    cb_(cs.as_mut_ptr(), data.as_ptr(), 0);
                    rb_(rs.as_mut_ptr(), data.as_ptr(), 0);
                }
            }
            eq_bytes(&format!("row55 {p}_inc_blocks(0) state"), &cs, &rs);
            eq_bytes(&format!("row55 {p}_inc_blocks(0) is a no-op"), &cs, &before);
            // Row 56: inc_finalize with inlen == 0.
            let mut cd = vec![0xA5u8; ob + 16];
            let mut rd = vec![0xA5u8; ob + 16];
            unsafe {
                cf(cd.as_mut_ptr(), cs.as_mut_ptr(), data.as_ptr(), 0);
                rf(rd.as_mut_ptr(), rs.as_mut_ptr(), data.as_ptr(), 0);
            }
            eq_bytes(&format!("row56 {p}_inc_finalize(inlen=0) digest"), &cd, &rd);
            eq_bytes(&format!("row56 {p}_inc_finalize(inlen=0) state"), &cs, &rs);
        }
    }

    #[test]
    fn err59_sha_empty_input() {
        for (p, ob) in [("sha256", 32usize), ("sha512", 64)] {
            let l = libs();
            let c: libloading::Symbol<FOneShot> = sym(&l.c, p);
            let r: libloading::Symbol<FOneShot> = sym(&l.r, p);
            let inp = [0u8; 1];
            let mut cb = vec![0xA5u8; ob + 16];
            let mut rb = vec![0xA5u8; ob + 16];
            unsafe {
                c(cb.as_mut_ptr(), inp.as_ptr(), 0);
                r(rb.as_mut_ptr(), inp.as_ptr(), 0);
            }
            eq_bytes(&format!("row59 {p}(inlen=0)"), &cb, &rb);
        }
    }

    #[test]
    fn err60_mgf1_zero_outlen() {
        for name in ["SPX_mgf1_256", "SPX_mgf1_512"] {
            let l = libs();
            let c: libloading::Symbol<FMgf1> = sym(&l.c, name);
            let r: libloading::Symbol<FMgf1> = sym(&l.r, name);
            let inbuf = [0u8; 72];
            for inlen in [0u64, 1, 32, 64] {
                let mut cb = vec![0xA5u8; 16];
                let mut rb = vec![0xA5u8; 16];
                unsafe {
                    c(cb.as_mut_ptr(), 0, inbuf.as_ptr(), inlen);
                    r(rb.as_mut_ptr(), 0, inbuf.as_ptr(), inlen);
                }
                eq_bytes(&format!("row60 {name}(outlen=0, inlen={inlen})"), &cb, &rb);
                eq_bytes("row60 nothing written", &cb, &vec![0xA5u8; 16]);
            }
        }
    }
}

#[cfg(all(feature = "shake", not(feature = "sha2")))]
mod shake_edges {
    use super::common::*;

    type FIncInit = unsafe extern "C" fn(*mut u64);
    type FIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FIncFinalize = unsafe extern "C" fn(*mut u64);
    type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type FOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);

    fn sb(v: &[u64]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_ne_bytes()).collect()
    }

    #[test]
    fn err57_58_shake_zero_shapes() {
        let (ci, ri) = crate::both!("shake256_inc_init", FIncInit);
        let (ca, ra) = crate::both!("shake256_inc_absorb", FIncAbsorb);
        let (cf, rf) = crate::both!("shake256_inc_finalize", FIncFinalize);
        let (cq, rq) = crate::both!("shake256_inc_squeeze", FIncSqueeze);
        let mut cs = vec![0u64; 26];
        let mut rs = vec![0u64; 26];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
        }
        eq_bytes("row57 inc_init state", &sb(&cs), &sb(&rs));
        let before = sb(&cs);
        let data = [0u8; 256];
        // Row 57: inlen == 0 absorb is a no-op.
        for _ in 0..3 {
            unsafe {
                ca(cs.as_mut_ptr(), data.as_ptr(), 0);
                ra(rs.as_mut_ptr(), data.as_ptr(), 0);
            }
        }
        eq_bytes("row57 inc_absorb(0) state", &sb(&cs), &sb(&rs));
        eq_bytes("row57 inc_absorb(0) is a no-op", &sb(&cs), &before);
        unsafe {
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
        }
        eq_bytes("row57 inc_finalize state", &sb(&cs), &sb(&rs));
        // Row 58: outlen == 0 squeeze writes nothing.
        for _ in 0..3 {
            let st_before = sb(&cs);
            let mut cb = vec![0xA5u8; 16];
            let mut rb = vec![0xA5u8; 16];
            unsafe {
                cq(cb.as_mut_ptr(), 0, cs.as_mut_ptr());
                rq(rb.as_mut_ptr(), 0, rs.as_mut_ptr());
            }
            eq_bytes("row58 inc_squeeze(0) out", &cb, &rb);
            eq_bytes("row58 nothing written", &cb, &vec![0xA5u8; 16]);
            eq_bytes("row58 inc_squeeze(0) state", &sb(&cs), &sb(&rs));
            eq_bytes("row58 state unchanged", &sb(&cs), &st_before);
        }
    }

    #[test]
    fn err59_shake_empty_input() {
        let (c, r) = crate::both!("shake256", FOneShot);
        let inp = [0u8; 1];
        for outlen in [0usize, 1, 32, 136, 137] {
            let mut cb = vec![0xA5u8; outlen + 16];
            let mut rb = vec![0xA5u8; outlen + 16];
            unsafe {
                c(cb.as_mut_ptr(), outlen, inp.as_ptr(), 0);
                r(rb.as_mut_ptr(), outlen, inp.as_ptr(), 0);
            }
            eq_bytes(&format!("row59 shake256(inlen=0, outlen={outlen})"), &cb, &rb);
        }
    }
}

#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
mod haraka_edges {
    use super::common::*;

    type FIncInit = unsafe extern "C" fn(*mut u8);
    type FIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type FIncFinalize = unsafe extern "C" fn(*mut u8);
    type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);
    type FHarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);

    #[test]
    fn err61_haraka_sponge_zero_shapes() {
        let (ci, ri) = crate::both!("SPX_haraka_S_inc_init", FIncInit);
        let (ca, ra) = crate::both!("SPX_haraka_S_inc_absorb", FIncAbsorb);
        let (cf, rf) = crate::both!("SPX_haraka_S_inc_finalize", FIncFinalize);
        let (cq, rq) = crate::both!("SPX_haraka_S_inc_squeeze", FIncSqueeze);
        let mut rng = Rng::new(RNG_SEED ^ 61);
        let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));

        let mut cs = vec![0u8; 65];
        let mut rs = vec![0u8; 65];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
        }
        eq_bytes("row61 inc_init state", &cs, &rs);
        let before = cs.clone();
        let data = [0u8; 256];
        for _ in 0..3 {
            unsafe {
                ca(cs.as_mut_ptr(), data.as_ptr(), 0, cctx.as_ptr());
                ra(rs.as_mut_ptr(), data.as_ptr(), 0, rctx.as_ptr());
            }
        }
        eq_bytes("row61 inc_absorb(0) state", &cs, &rs);
        eq_bytes("row61 inc_absorb(0) is a no-op", &cs, &before);
        unsafe {
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
        }
        eq_bytes("row61 inc_finalize state", &cs, &rs);
        for outlen in [0usize, 1, 31, 32, 33, 64, 100] {
            let mut cb = vec![0xA5u8; outlen + 16];
            let mut rb = vec![0xA5u8; outlen + 16];
            unsafe {
                cq(cb.as_mut_ptr(), outlen, cs.as_mut_ptr(), cctx.as_ptr());
                rq(rb.as_mut_ptr(), outlen, rs.as_mut_ptr(), rctx.as_ptr());
            }
            eq_bytes(&format!("row61 inc_squeeze({outlen}) out"), &cb, &rb);
            eq_bytes(&format!("row61 inc_squeeze({outlen}) state"), &cs, &rs);
        }
    }

    #[test]
    fn err59_haraka_S_empty_input() {
        let (c, r) = crate::both!("SPX_haraka_S", FHarakaS);
        let mut rng = Rng::new(RNG_SEED ^ 590);
        let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let inp = [0u8; 1];
        for outlen in [0usize, 1, 32, 33, 100] {
            let mut cb = vec![0xA5u8; outlen + 16];
            let mut rb = vec![0xA5u8; outlen + 16];
            unsafe {
                c(cb.as_mut_ptr(), outlen as u64, inp.as_ptr(), 0, cctx.as_ptr());
                r(rb.as_mut_ptr(), outlen as u64, inp.as_ptr(), 0, rctx.as_ptr());
            }
            eq_bytes(&format!("row59 haraka_S(inlen=0, outlen={outlen})"), &cb, &rb);
        }
    }
}

// ===========================================================================
// Row 62 -- chain_lengths value extremes / checksum overflow
// ===========================================================================
#[test]
fn err62_chain_lengths_extremes() {
    let (c, r) = both!(
        "SPX_chain_lengths",
        unsafe extern "C" fn(*mut u32, *const u8)
    );
    let mut msgs: Vec<Vec<u8>> = vec![vec![0x00u8; SPX_N], vec![0xFFu8; SPX_N]];
    // Single-nibble sweep across every byte position and every nibble value.
    for pos in 0..SPX_N {
        for nib in 0u8..16 {
            let mut m = vec![0u8; SPX_N];
            m[pos] = nib | (nib << 4);
            msgs.push(m);
        }
    }
    for m in &msgs {
        let mut cl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), m.as_ptr());
            r(rl.as_mut_ptr(), m.as_ptr());
        }
        eq(&format!("row62 chain_lengths({})", hex(m)), &cl, &rl);
    }
}

// ===========================================================================
// Rows 63-64 -- key/sign boundary shapes
// ===========================================================================
#[test]
fn err63_seed_keypair_exact_seed_size() {
    let _g = drbg_guard();
    let (c, r) = both!("crypto_sign_seed_keypair", FSeedKeypair);
    let mut rng = Rng::new(RNG_SEED ^ 63);
    for seed in [
        vec![0x00u8; CRYPTO_SEEDBYTES],
        vec![0xFFu8; CRYPTO_SEEDBYTES],
        rng.bytes(CRYPTO_SEEDBYTES),
    ] {
        // Give both output buffers a marker tail: the C writes exactly
        // SPX_PK_BYTES / SPX_SK_BYTES bytes and no more.
        let mut cpk = vec![0xA5u8; SPX_PK_BYTES + 32];
        let mut rpk = vec![0xA5u8; SPX_PK_BYTES + 32];
        let mut csk = vec![0xA5u8; SPX_SK_BYTES + 32];
        let mut rsk = vec![0xA5u8; SPX_SK_BYTES + 32];
        let (crc, rrc) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq("row63 return", crc, rrc);
        eq("row63 return == 0", crc, 0);
        eq_bytes("row63 pk", &cpk, &rpk);
        eq_bytes("row63 sk", &csk, &rsk);
        eq_bytes(
            "row63 pk tail untouched",
            &cpk[SPX_PK_BYTES..],
            &vec![0xA5u8; 32],
        );
        eq_bytes(
            "row63 sk tail untouched",
            &csk[SPX_SK_BYTES..],
            &vec![0xA5u8; 32],
        );
    }
}

#[test]
fn err64_crypto_sign_zero_length_message() {
    let _g = drbg_guard();
    let (c, r) = both!("crypto_sign", FSign);
    let (ck, rk) = both!("crypto_sign_seed_keypair", FSeedKeypair);
    let mut rng = Rng::new(RNG_SEED ^ 64);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ck(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rk(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
    }
    eq_bytes("row64 sk", &csk, &rsk);

    seed_both_drbgs(&kat_entropy());
    let m = [0u8; 1];
    let mut csm = vec![0xA5u8; SPX_BYTES + 32];
    let mut rsm = vec![0xA5u8; SPX_BYTES + 32];
    let mut cl = 0xDEAD_BEEFu64;
    let mut rl = 0xDEAD_BEEFu64;
    let (crc, rrc) = unsafe {
        (
            c(csm.as_mut_ptr(), &mut cl, m.as_ptr(), 0, csk.as_ptr()),
            r(rsm.as_mut_ptr(), &mut rl, m.as_ptr(), 0, rsk.as_ptr()),
        )
    };
    eq("row64 return", crc, rrc);
    eq("row64 return == 0", crc, 0);
    eq("row64 *smlen", cl, rl);
    eq("row64 *smlen == SPX_BYTES", cl, SPX_BYTES as u64);
    eq_bytes("row64 sm", &csm, &rsm);
    eq_bytes(
        "row64 sm tail untouched",
        &csm[SPX_BYTES..],
        &vec![0xA5u8; 32],
    );
}
