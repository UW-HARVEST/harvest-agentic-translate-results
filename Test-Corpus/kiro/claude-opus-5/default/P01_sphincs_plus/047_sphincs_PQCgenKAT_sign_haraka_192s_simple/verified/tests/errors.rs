//! Phase C — error / rejection surface (`ERRORS.md` rows E1–E50).
//!
//! Every test constructs the exact invalid input, calls BOTH libraries, and
//! asserts the SAME error code / sentinel (not merely "both failed").

mod common;
use common::*;

type SizeFn = unsafe extern "C" fn() -> u64;
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Signature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type Randombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type SeedexpanderInit = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32;
type Seedexpander = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, u64) -> i32;
type DrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type SetU32 = unsafe extern "C" fn(*mut u32, u32);
type SetU64 = unsafe extern "C" fn(*mut u32, u64);
type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
type ComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
type GenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
type TreeHash =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeafFn, *mut u32);
type WotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
type WotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

fn abytes(a: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, 32) }
}

fn kat_entropy() -> Vec<u8> {
    (0..48u8).collect()
}

fn seed_both(l: &Libs) {
    let (c, r) = l.pair::<RandombytesInit>("randombytes_init");
    let mut ce = kat_entropy();
    let mut re = kat_entropy();
    unsafe {
        c(ce.as_mut_ptr(), std::ptr::null_mut());
        r(re.as_mut_ptr(), std::ptr::null_mut());
    }
}

/// A signature/key set both libraries agree on, for the verify/open rows.
struct Fixture {
    pk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
}

fn fixture(l: &Libs, mlen: usize) -> Fixture {
    let (ckp, rkp) = l.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    let (csig, rsig) = l.pair::<Signature>("crypto_sign_signature");
    let mut rng = Rng::new(SEED + 900 + mlen as u64);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
    }
    eq_bytes("fixture pk", &cpk, &rpk);
    eq_bytes("fixture sk", &csk, &rsk);
    let m = rng.bytes(mlen.max(1));
    // crypto_sign_signature draws optrand from the DRBG; seed it first so both
    // sides produce the same signature.
    seed_both(l);
    let mut cs = vec![0u8; SPX_BYTES];
    let mut rs = vec![0u8; SPX_BYTES];
    let mut cl = 0usize;
    let mut rl = 0usize;
    unsafe {
        csig(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr());
        rsig(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr());
    }
    eq_bytes("fixture signature", &cs, &rs);
    Fixture {
        pk: cpk,
        m,
        sig: cs,
    }
}

/// Calls `crypto_sign_verify` on both libraries and asserts identical returns.
#[track_caller]
fn verify_both(l: &Libs, sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let (c, r) = l.pair::<Verify>("crypto_sign_verify");
    let (cv, rv) = unsafe {
        (
            c(sig.as_ptr(), siglen, m.as_ptr(), mlen, pk.as_ptr()),
            r(sig.as_ptr(), siglen, m.as_ptr(), mlen, pk.as_ptr()),
        )
    };
    assert_eq!(cv, rv, "crypto_sign_verify return code differs");
    cv
}

/// Calls `crypto_sign_open` on both libraries; asserts identical return code,
/// identical `*mlen` and identical output buffer.
#[track_caller]
fn open_both(l: &Libs, sm: &[u8], smlen: u64, pk: &[u8], mbuf_len: usize) -> (i32, u64) {
    let (c, r) = l.pair::<Open>("crypto_sign_open");
    let mut cm = vec![0x5Au8; mbuf_len];
    let mut rm = vec![0x5Au8; mbuf_len];
    let mut cl = 0xDEAD_BEEF_DEAD_BEEFu64;
    let mut rl = 0xDEAD_BEEF_DEAD_BEEFu64;
    let (cv, rv) = unsafe {
        (
            c(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr()),
            r(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr()),
        )
    };
    assert_eq!(cv, rv, "crypto_sign_open return code differs");
    assert_eq!(cl, rl, "crypto_sign_open *mlen differs");
    eq_bytes("crypto_sign_open m buffer", &cm, &rm);
    (cv, cl)
}

/* ============ E1–E4: crypto_sign_verify siglen ==================== */

#[test]
fn e01_verify_siglen_zero() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    assert_eq!(verify_both(l, &f.sig, 0, &f.m, 33, &f.pk), -1);
}

#[test]
fn e02_verify_siglen_minus_one() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    assert_eq!(verify_both(l, &f.sig, SPX_BYTES - 1, &f.m, 33, &f.pk), -1);
}

#[test]
fn e03_verify_siglen_plus_one() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    assert_eq!(verify_both(l, &f.sig, SPX_BYTES + 1, &f.m, 33, &f.pk), -1);
}

#[test]
fn e04_verify_siglen_huge() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    // The length check runs before `sig` is touched, so this is safe.
    for siglen in [usize::MAX, usize::MAX / 2, 1 << 40] {
        assert_eq!(verify_both(l, &f.sig, siglen, &f.m, 33, &f.pk), -1);
    }
}

/* ============ E5–E9: crypto_sign_verify rejection ================= */

#[test]
fn e05_verify_corrupt_sig_regions() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    // one representative offset in each structural region of the signature
    let fors_end = SPX_N + SPX_FORS_BYTES;
    let wots_end = fors_end + SPX_WOTS_BYTES;
    let offsets = [
        0usize,                       // R
        SPX_N,                        // first FORS sk
        SPX_N + SPX_N,                // first FORS auth node
        fors_end - 1,                 // last FORS byte
        fors_end,                     // first WOTS byte of layer 0
        wots_end - 1,                 // last WOTS byte of layer 0
        wots_end,                     // first auth-path byte of layer 0
        SPX_BYTES - 1,                // last signature byte
    ];
    for off in offsets {
        let mut bad = f.sig.clone();
        bad[off] ^= 0x01;
        assert_eq!(
            verify_both(l, &bad, SPX_BYTES, &f.m, 33, &f.pk),
            -1,
            "flipping sig[{off}] must be rejected"
        );
    }
}

/// NOTE on rejection expectations for message mutations.
///
/// `hash_blake.c` calls `blakeX_update(&S, m, mlen)` with a length in BYTES,
/// but `blake256_update`/`blake512_update` take a length in BITS (see
/// `blake256()`, which passes `inlen*8`).  For short inputs the accumulated bit
/// count never reaches a compression, so `blake*_final` returns the unchanged
/// IV and `hash_message`'s seed does not depend on the message at all.  That is
/// the C's behaviour and the Rust reproduces it byte-for-byte, so "flip any
/// message byte ⇒ rejected" is NOT a property of this library.
///
/// The differential requirement — C and Rust return the same code — is asserted
/// for every case by `verify_both` / `open_both`.  On top of that these tests
/// require that at least one mutation in the set IS rejected, so they cannot
/// pass vacuously.
#[test]
fn e06_verify_wrong_message() {
    let _drbg = drbg_lock();
    let l = libs();
    let mlen = 1000usize;
    let f = fixture(l, mlen);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for i in 0..mlen {
        let mut m = f.m.clone();
        m[i] ^= 0x80;
        // verify_both asserts C == Rust
        match verify_both(l, &f.sig, SPX_BYTES, &m, mlen, &f.pk) {
            -1 => rejected += 1,
            0 => accepted += 1,
            other => panic!("unexpected return {other}"),
        }
    }
    assert!(
        rejected > 0,
        "no single-byte message mutation was rejected — the test is vacuous"
    );
    eprintln!("e06: {rejected} rejected, {accepted} accepted (byte/bit length quirk)");
}

#[test]
fn e07_verify_wrong_pk() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    let g = fixture(l, 34);
    assert_eq!(verify_both(l, &f.sig, SPX_BYTES, &f.m, 33, &g.pk), -1);
    for i in 0..SPX_PK_BYTES {
        let mut pk = f.pk.clone();
        pk[i] ^= 0x01;
        assert_eq!(verify_both(l, &f.sig, SPX_BYTES, &f.m, 33, &pk), -1);
    }
}

#[test]
fn e08_verify_wrong_mlen() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 1000);
    let mut rejected = 0usize;
    for mlen in [0usize, 1, 999, 1001, 1016, 1024] {
        let mut m = f.m.clone();
        m.resize(2048, 0);
        if verify_both(l, &f.sig, SPX_BYTES, &m, mlen, &f.pk) == -1 {
            rejected += 1;
        }
    }
    assert!(rejected > 0, "no wrong mlen was rejected — the test is vacuous");
}

#[test]
fn e09_verify_all_zero() {
    let l = libs();
    let sig = vec![0u8; SPX_BYTES];
    let pk = vec![0u8; SPX_PK_BYTES];
    let m = vec![0u8; 33];
    assert_eq!(verify_both(l, &sig, SPX_BYTES, &m, 33, &pk), -1);
    let sig = vec![0xffu8; SPX_BYTES];
    let pk = vec![0xffu8; SPX_PK_BYTES];
    let m = vec![0xffu8; 33];
    assert_eq!(verify_both(l, &sig, SPX_BYTES, &m, 33, &pk), -1);
}

#[test]
fn e10_verify_valid() {
    let _drbg = drbg_lock();
    let l = libs();
    for mlen in [0usize, 1, 33, 231] {
        let f = fixture(l, mlen);
        assert_eq!(
            verify_both(l, &f.sig, SPX_BYTES, &f.m, mlen, &f.pk),
            0,
            "valid signature must verify (mlen={mlen})"
        );
    }
}

/* ============ E11–E15: crypto_sign_open ========================== */

#[test]
fn e11_open_smlen_zero() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    let sm = vec![0u8; SPX_BYTES + 33];
    let (ret, mlen) = open_both(l, &sm, 0, &f.pk, 64);
    assert_eq!(ret, -1);
    assert_eq!(mlen, 0);
}

#[test]
fn e12_open_smlen_minus_one() {
    let _drbg = drbg_lock();
    let l = libs();
    let f = fixture(l, 33);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    for smlen in [1u64, (SPX_BYTES - 1) as u64] {
        // m must have room for the memset(m, 0, smlen) the C does.
        let (ret, mlen) = open_both(l, &sm, smlen, &f.pk, SPX_BYTES + 64);
        assert_eq!(ret, -1, "smlen={smlen}");
        assert_eq!(mlen, 0, "smlen={smlen}");
    }
}

#[test]
fn e13_open_smlen_exactly_sigbytes() {
    let _drbg = drbg_lock();
    let l = libs();
    // Boundary: smlen == SPX_BYTES is ACCEPTED and means an empty message.
    let f = fixture(l, 0);
    let (ret, mlen) = open_both(l, &f.sig, SPX_BYTES as u64, &f.pk, SPX_BYTES + 64);
    assert_eq!(ret, 0, "smlen == SPX_BYTES with a valid empty-message sig");
    assert_eq!(mlen, 0);
}

#[test]
fn e14_open_bad_signature() {
    let _drbg = drbg_lock();
    let l = libs();
    let mlen = 1000usize;
    let f = fixture(l, mlen);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    // Offsets inside the signature are always rejected.  Offsets inside the
    // appended message may or may not be, because of the blake byte/bit length
    // quirk documented above -- but C and Rust must agree either way.
    for off in [0usize, SPX_N, SPX_N + 1, SPX_BYTES / 2, SPX_BYTES - 1] {
        let mut bad = sm.clone();
        bad[off] ^= 0x01;
        let (ret, mlen_out) =
            open_both(l, &bad, (SPX_BYTES + mlen) as u64, &f.pk, SPX_BYTES + mlen + 64);
        assert_eq!(ret, -1, "corrupting the signature at sm[{off}] must be rejected");
        assert_eq!(mlen_out, 0, "*mlen must be zeroed on failure");
    }
    let mut rejected = 0usize;
    for off in [SPX_BYTES, SPX_BYTES + 1, SPX_BYTES + 32, SPX_BYTES + mlen - 1] {
        let mut bad = sm.clone();
        bad[off] ^= 0x01;
        let (ret, mlen_out) =
            open_both(l, &bad, (SPX_BYTES + mlen) as u64, &f.pk, SPX_BYTES + mlen + 64);
        if ret == -1 {
            rejected += 1;
            assert_eq!(mlen_out, 0);
        } else {
            assert_eq!(mlen_out as usize, mlen);
        }
    }
    assert!(rejected > 0, "no message-region corruption was rejected");
}

#[test]
fn e15_open_smlen_too_long() {
    let _drbg = drbg_lock();
    let l = libs();
    let mlen = 1000usize;
    let f = fixture(l, mlen);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);
    sm.extend_from_slice(&[0xEEu8; 64]); // trailing junk
    let (ret, mlen_out) = open_both(
        l,
        &sm,
        (SPX_BYTES + mlen + 64) as u64,
        &f.pk,
        SPX_BYTES + mlen + 128,
    );
    assert_eq!(ret, -1, "a longer message than was signed must be rejected");
    assert_eq!(mlen_out, 0);
}

/* ============ E16–E18: the always-succeed paths =================== */

#[test]
fn e16_seed_keypair_always_zero() {
    let l = libs();
    let (c, r) = l.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    for seed in [
        vec![0u8; CRYPTO_SEEDBYTES],
        vec![0xffu8; CRYPTO_SEEDBYTES],
        (0..CRYPTO_SEEDBYTES).map(|i| i as u8).collect(),
    ] {
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut rpk = vec![0u8; SPX_PK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        let mut rsk = vec![0u8; SPX_SK_BYTES];
        let (cv, rv) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0, "crypto_sign_seed_keypair has no error path");
        eq_bytes("pk", &cpk, &rpk);
        eq_bytes("sk", &csk, &rsk);
    }
}

#[test]
fn e17_signature_always_zero() {
    let _drbg = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<Signature>("crypto_sign_signature");
    for sk in [vec![0u8; SPX_SK_BYTES], vec![0xffu8; SPX_SK_BYTES]] {
        seed_both(l);
        let m = [0u8; 1];
        let mut cs = vec![0u8; SPX_BYTES];
        let mut rs = vec![0u8; SPX_BYTES];
        let mut cl = usize::MAX;
        let mut rl = usize::MAX;
        let (cv, rv) = unsafe {
            (
                c(cs.as_mut_ptr(), &mut cl, m.as_ptr(), 0, sk.as_ptr()),
                r(rs.as_mut_ptr(), &mut rl, m.as_ptr(), 0, sk.as_ptr()),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0, "crypto_sign_signature has no error path");
        assert_eq!(cl, rl);
        assert_eq!(cl, SPX_BYTES);
        eq_bytes("degenerate-sk signature", &cs, &rs);
    }
}

#[test]
fn e18_sign_mlen_zero() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ckp, rkp) = l.pair::<Keypair>("crypto_sign_keypair");
    let (c, r) = l.pair::<Sign>("crypto_sign");
    seed_both(l);
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
    }
    let m = [0u8; 1];
    let mut csm = vec![0u8; SPX_BYTES];
    let mut rsm = vec![0u8; SPX_BYTES];
    let mut cl = u64::MAX;
    let mut rl = u64::MAX;
    let (cv, rv) = unsafe {
        (
            c(csm.as_mut_ptr(), &mut cl, m.as_ptr(), 0, csk.as_ptr()),
            r(rsm.as_mut_ptr(), &mut rl, m.as_ptr(), 0, rsk.as_ptr()),
        )
    };
    assert_eq!(cv, rv);
    assert_eq!(cv, 0);
    assert_eq!(cl, rl);
    assert_eq!(cl as usize, SPX_BYTES);
    eq_bytes("crypto_sign(mlen=0)", &csm, &rsm);
}

/* ============ E19–E29: seedexpander =============================== */

fn xof_init_both(l: &Libs, maxlen: u64) -> (i32, AesXofStruct, AesXofStruct) {
    let (c, r) = l.pair::<SeedexpanderInit>("seedexpander_init");
    let mut rng = Rng::new(SEED + 19);
    let seed = rng.bytes(32);
    let div = rng.bytes(8);
    let mut cs = seed.clone();
    let mut rs = seed.clone();
    let mut cd = div.clone();
    let mut rd = div.clone();
    // Pre-fill with a recognisable pattern so "ctx untouched" is verifiable.
    let mut cctx = AesXofStruct {
        buffer: [0x11; 16],
        buffer_pos: 0x2222_2222_2222_2222,
        length_remaining: 0x3333_3333_3333_3333,
        key: [0x44; 32],
        ctr: [0x55; 16],
    };
    let mut rctx = cctx;
    let (cv, rv) = unsafe {
        (
            c(&mut cctx, cs.as_mut_ptr(), cd.as_mut_ptr(), maxlen),
            r(&mut rctx, rs.as_mut_ptr(), rd.as_mut_ptr(), maxlen),
        )
    };
    assert_eq!(cv, rv, "seedexpander_init return (maxlen={maxlen})");
    assert_eq!(cctx, rctx, "AES_XOF_struct (maxlen={maxlen})");
    (cv, cctx, rctx)
}

#[test]
fn e19_seedexpander_init_maxlen_2p32() {
    let l = libs();
    let (ret, cctx, _) = xof_init_both(l, 0x1_0000_0000);
    assert_eq!(ret, RNG_BAD_MAXLEN);
    // rejected before anything is written
    assert_eq!(cctx.key, [0x44u8; 32], "ctx must be untouched on rejection");
    assert_eq!(cctx.length_remaining, 0x3333_3333_3333_3333);
}

#[test]
fn e20_seedexpander_init_maxlen_max() {
    let l = libs();
    for maxlen in [u64::MAX, 0x1_0000_0001, 0x8000_0000_0000_0000] {
        let (ret, _, _) = xof_init_both(l, maxlen);
        assert_eq!(ret, RNG_BAD_MAXLEN, "maxlen={maxlen}");
    }
}

#[test]
fn e21_seedexpander_init_maxlen_2p32_minus_1() {
    let l = libs();
    let (ret, cctx, _) = xof_init_both(l, 0xFFFF_FFFF);
    assert_eq!(ret, RNG_SUCCESS, "0xFFFFFFFF is the largest accepted maxlen");
    assert_eq!(cctx.length_remaining, 0xFFFF_FFFF);
    assert_eq!(&cctx.ctr[8..12], &[0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(&cctx.ctr[12..16], &[0, 0, 0, 0]);
    assert_eq!(cctx.buffer_pos, 16);
    assert_eq!(cctx.buffer, [0u8; 16]);
}

#[test]
fn e22_seedexpander_init_maxlen_zero() {
    let l = libs();
    let (ret, cctx, _) = xof_init_both(l, 0);
    assert_eq!(ret, RNG_SUCCESS);
    assert_eq!(cctx.length_remaining, 0);
}

#[test]
fn e23_seedexpander_null_out() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let (_, mut cctx, mut rctx) = xof_init_both(l, 1024);
    let before = cctx;
    let (cv, rv) = unsafe {
        (
            c(&mut cctx, std::ptr::null_mut(), 16),
            r(&mut rctx, std::ptr::null_mut(), 16),
        )
    };
    assert_eq!(cv, rv);
    assert_eq!(cv, RNG_BAD_OUTBUF);
    assert_eq!(cctx, before, "ctx must be untouched");
    assert_eq!(cctx, rctx);
}

#[test]
fn e24_seedexpander_null_out_precedence() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let (_, mut cctx, mut rctx) = xof_init_both(l, 16);
    // xlen is ALSO out of range; the null check must win.
    let (cv, rv) = unsafe {
        (
            c(&mut cctx, std::ptr::null_mut(), 1_000_000),
            r(&mut rctx, std::ptr::null_mut(), 1_000_000),
        )
    };
    assert_eq!(cv, rv);
    assert_eq!(cv, RNG_BAD_OUTBUF, "the NULL check precedes the length check");
    assert_eq!(cctx, rctx);
}

#[test]
fn e25_seedexpander_xlen_equals_remaining() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    for maxlen in [1u64, 16, 64, 1024] {
        let (_, mut cctx, mut rctx) = xof_init_both(l, maxlen);
        let before = cctx;
        let mut cb = vec![0u8; maxlen as usize];
        let mut rb = vec![0u8; maxlen as usize];
        // `xlen >= length_remaining` -- so EQUAL is rejected (C off-by-one).
        let (cv, rv) = unsafe {
            (
                c(&mut cctx, cb.as_mut_ptr(), maxlen),
                r(&mut rctx, rb.as_mut_ptr(), maxlen),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, RNG_BAD_REQ_LEN, "xlen == length_remaining (maxlen={maxlen})");
        assert_eq!(cctx, before, "ctx must be untouched on rejection");
        assert_eq!(cctx, rctx);
    }
}

#[test]
fn e26_seedexpander_xlen_gt_remaining() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let (_, mut cctx, mut rctx) = xof_init_both(l, 64);
    for xlen in [65u64, 1000, u64::MAX] {
        let mut cb = vec![0u8; 16];
        let mut rb = vec![0u8; 16];
        let (cv, rv) = unsafe {
            (
                c(&mut cctx, cb.as_mut_ptr(), xlen),
                r(&mut rctx, rb.as_mut_ptr(), xlen),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, RNG_BAD_REQ_LEN, "xlen={xlen}");
        assert_eq!(cctx, rctx);
    }
}

#[test]
fn e27_seedexpander_xlen_max_ok() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    for maxlen in [2u64, 17, 64, 1024] {
        let (_, mut cctx, mut rctx) = xof_init_both(l, maxlen);
        let xlen = maxlen - 1;
        let mut cb = vec![0xAAu8; xlen as usize + 16];
        let mut rb = vec![0xAAu8; xlen as usize + 16];
        let (cv, rv) = unsafe {
            (
                c(&mut cctx, cb.as_mut_ptr(), xlen),
                r(&mut rctx, rb.as_mut_ptr(), xlen),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, RNG_SUCCESS, "xlen == length_remaining - 1 must be accepted");
        eq_bytes(&format!("seedexpander(maxlen={maxlen})"), &cb, &rb);
        assert_eq!(&cb[xlen as usize..], &[0xAAu8; 16], "overran xlen");
        assert_eq!(cctx, rctx);
    }
}

#[test]
fn e28_seedexpander_xlen_zero() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let (_, mut cctx, mut rctx) = xof_init_both(l, 1024);
    let before = cctx;
    let mut cb = [0xAAu8; 16];
    let mut rb = [0xAAu8; 16];
    let (cv, rv) = unsafe {
        (
            c(&mut cctx, cb.as_mut_ptr(), 0),
            r(&mut rctx, rb.as_mut_ptr(), 0),
        )
    };
    assert_eq!(cv, rv);
    assert_eq!(cv, RNG_SUCCESS);
    assert_eq!(cb, [0xAAu8; 16], "nothing must be written for xlen == 0");
    assert_eq!(cb, rb);
    // length_remaining -= 0; the rest of the state is unchanged.
    assert_eq!(cctx, before);
    assert_eq!(cctx, rctx);
}

#[test]
fn e29_seedexpander_zero_budget() {
    let l = libs();
    let (c, r) = l.pair::<Seedexpander>("seedexpander");
    let (_, mut cctx, mut rctx) = xof_init_both(l, 0);
    // length_remaining == 0, so even xlen == 0 satisfies `xlen >= remaining`.
    for xlen in [0u64, 1, 16] {
        let mut cb = [0u8; 16];
        let mut rb = [0u8; 16];
        let (cv, rv) = unsafe {
            (
                c(&mut cctx, cb.as_mut_ptr(), xlen),
                r(&mut rctx, rb.as_mut_ptr(), xlen),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, RNG_BAD_REQ_LEN, "zero budget rejects xlen={xlen}");
        assert_eq!(cctx, rctx);
    }
}

/* ============ E30–E33: randombytes / DRBG ======================== */

#[test]
fn e30_randombytes_xlen_zero() {
    let _drbg = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<Randombytes>("randombytes");
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");
    seed_both(l);
    let before = unsafe { **cg };
    let mut cb = [0xAAu8; 16];
    let mut rb = [0xAAu8; 16];
    let (cv, rv) = unsafe { (c(cb.as_mut_ptr(), 0), r(rb.as_mut_ptr(), 0)) };
    assert_eq!(cv, rv);
    assert_eq!(cv, RNG_SUCCESS);
    assert_eq!(cb, [0xAAu8; 16], "nothing written for xlen == 0");
    assert_eq!(cb, rb);
    let after = unsafe { **cg };
    assert_eq!(after, unsafe { **rg }, "DRBG_ctx after randombytes(_, 0)");
    // The loop body is skipped, but the trailing Update + counter bump still run.
    assert_eq!(after.reseed_counter, before.reseed_counter + 1);
    assert_ne!(after.key, before.key, "the trailing DRBG Update still happens");
}

#[test]
fn e31_randombytes_always_success() {
    let _drbg = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<Randombytes>("randombytes");
    seed_both(l);
    for xlen in [0usize, 1, 15, 16, 17, 100, 4096] {
        let mut cb = vec![0u8; xlen.max(1)];
        let mut rb = vec![0u8; xlen.max(1)];
        let (cv, rv) = unsafe {
            (
                c(cb.as_mut_ptr(), xlen as u64),
                r(rb.as_mut_ptr(), xlen as u64),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, RNG_SUCCESS, "randombytes has no failure path (xlen={xlen})");
        eq_bytes(&format!("randombytes(xlen={xlen})"), &cb, &rb);
    }
}

#[test]
fn e32_randombytes_init_null_pers() {
    let _drbg = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<RandombytesInit>("randombytes_init");
    let cg = l.c::<*mut Drbg>("DRBG_ctx");
    let rg = l.rs::<*mut Drbg>("DRBG_ctx");
    let entropy = kat_entropy();

    // NULL personalization: no XOR.
    let mut ce = entropy.clone();
    let mut re = entropy.clone();
    unsafe {
        ci(ce.as_mut_ptr(), std::ptr::null_mut());
        ri(re.as_mut_ptr(), std::ptr::null_mut());
    }
    let null_state = unsafe { **cg };
    assert_eq!(null_state, unsafe { **rg });

    // An all-zero personalization string must be equivalent to NULL (XOR by 0).
    let mut zero = vec![0u8; 48];
    let mut zero2 = vec![0u8; 48];
    let mut ce = entropy.clone();
    let mut re = entropy.clone();
    unsafe {
        ci(ce.as_mut_ptr(), zero.as_mut_ptr());
        ri(re.as_mut_ptr(), zero2.as_mut_ptr());
    }
    assert_eq!(unsafe { **cg }, unsafe { **rg });
    assert_eq!(
        unsafe { **cg },
        null_state,
        "an all-zero personalization must match the NULL path"
    );

    // A non-zero one must differ from the NULL path, identically on both sides.
    let mut pers: Vec<u8> = (0..48u8).map(|i| i ^ 0xA5).collect();
    let mut pers2 = pers.clone();
    let mut ce = entropy.clone();
    let mut re = entropy.clone();
    unsafe {
        ci(ce.as_mut_ptr(), pers.as_mut_ptr());
        ri(re.as_mut_ptr(), pers2.as_mut_ptr());
    }
    assert_eq!(unsafe { **cg }, unsafe { **rg });
    assert_ne!(unsafe { **cg }, null_state);
}

#[test]
fn e33_drbg_update_null_data() {
    let l = libs();
    let (c, r) = l.pair::<DrbgUpdate>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(SEED + 33_00);
    for i in 0..8 {
        let key = rng.bytes(32);
        let v = if i == 0 { vec![0xffu8; 16] } else { rng.bytes(16) };
        // NULL provided_data -> no XOR
        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cv = v.clone();
        let mut rv = v.clone();
        unsafe {
            c(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        eq_bytes("DRBG_Update(NULL) Key", &ck, &rk);
        eq_bytes("DRBG_Update(NULL) V", &cv, &rv);
        // an all-zero provided_data must be equivalent
        let mut zeros = vec![0u8; 48];
        let mut zeros2 = vec![0u8; 48];
        let mut ck2 = key.clone();
        let mut rk2 = key.clone();
        let mut cv2 = v.clone();
        let mut rv2 = v.clone();
        unsafe {
            c(zeros.as_mut_ptr(), ck2.as_mut_ptr(), cv2.as_mut_ptr());
            r(zeros2.as_mut_ptr(), rk2.as_mut_ptr(), rv2.as_mut_ptr());
        }
        eq_bytes("DRBG_Update(zeros) Key", &ck2, &rk2);
        eq_bytes("zeros == NULL path", &ck2, &ck);
        eq_bytes("zeros == NULL path (V)", &cv2, &cv);
    }
}

/* ============ E34–E39: out-of-range ADRS setters ================== */

/// C enum parameters accept any `int`; every one of these setters narrows with
/// `(unsigned char)` or writes a fixed-width big-endian field, so no value is
/// rejected.  The Rust must truncate identically.
#[test]
fn e34_set_type_out_of_range() {
    let l = libs();
    let (c, r) = l.pair::<SetU32>("SPX_set_type");
    let mut rng = Rng::new(SEED + 34_00);
    let vals = [
        7u32,          // one past SPX_ADDR_TYPE_FORSPRF
        8,
        127,
        128,
        255,
        256,           // truncates to 0x00
        257,
        0x0000_FF00,   // truncates to 0x00
        0x1234_5678,
        u32::MAX,      // truncates to 0xFF
    ];
    for v in vals {
        for _ in 0..4 {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(&format!("set_type({v})"), abytes(&ca), abytes(&ra));
            assert_eq!(abytes(&ca)[off::TYPE], v as u8, "must store the low byte");
        }
    }
}

#[test]
fn e35_set_layer_out_of_range() {
    let l = libs();
    let (c, r) = l.pair::<SetU32>("SPX_set_layer_addr");
    let mut rng = Rng::new(SEED + 35_00);
    for v in [SPX_D, SPX_D + 1, 255, 256, 0x1_0000, u32::MAX] {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        eq_bytes(&format!("set_layer_addr({v})"), abytes(&ca), abytes(&ra));
        assert_eq!(abytes(&ca)[off::LAYER], v as u8);
    }
}

#[test]
fn e36_set_chain_hash_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(SEED + 36_00);
    for (name, offset) in [
        ("SPX_set_chain_addr", off::CHAIN_ADDR),
        ("SPX_set_hash_addr", off::HASH_ADDR),
    ] {
        let (c, r) = l.pair::<SetU32>(name);
        for v in [
            SPX_WOTS_W,
            SPX_WOTS_LEN as u32,
            SPX_WOTS_LEN as u32 + 1,
            255,
            256,
            u32::MAX,
        ] {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(&format!("{name}({v})"), abytes(&ca), abytes(&ra));
            assert_eq!(abytes(&ca)[offset], v as u8);
        }
    }
}

#[test]
fn e37_set_tree_height_out_of_range() {
    let l = libs();
    let (c, r) = l.pair::<SetU32>("SPX_set_tree_height");
    let mut rng = Rng::new(SEED + 37_00);
    for v in [
        SPX_TREE_HEIGHT + 1,
        SPX_FULL_HEIGHT,
        SPX_FULL_HEIGHT + 1,
        255,
        256,
        u32::MAX,
    ] {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        eq_bytes(&format!("set_tree_height({v})"), abytes(&ca), abytes(&ra));
        assert_eq!(abytes(&ca)[off::TREE_HGT], v as u8);
    }
}

#[test]
fn e38_set_tree_addr_out_of_range() {
    let l = libs();
    let (c, r) = l.pair::<SetU64>("SPX_set_tree_addr");
    let mut rng = Rng::new(SEED + 38_00);
    let bits = SPX_TREE_HEIGHT * (SPX_D - 1);
    let vals = [
        u64::MAX,
        1u64 << 63,
        if bits < 64 { 1u64 << bits } else { u64::MAX },
        0xDEAD_BEEF_CAFE_BABE,
    ];
    for v in vals {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        eq_bytes(&format!("set_tree_addr({v:#x})"), abytes(&ca), abytes(&ra));
        assert_eq!(
            &abytes(&ca)[off::TREE..off::TREE + 8],
            &v.to_be_bytes(),
            "must write 8 big-endian bytes, unmasked"
        );
    }
}

#[test]
fn e39_set_kp_treeindex_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(SEED + 39_00);
    for (name, offset) in [
        ("SPX_set_keypair_addr", off::KP_ADDR),
        ("SPX_set_tree_index", off::TREE_INDEX),
    ] {
        let (c, r) = l.pair::<SetU32>(name);
        for v in [
            1u32 << SPX_TREE_HEIGHT,
            (1u32 << SPX_TREE_HEIGHT) + 1,
            u32::MAX,
            0xDEAD_BEEF,
        ] {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(&format!("{name}({v})"), abytes(&ca), abytes(&ra));
            assert_eq!(&abytes(&ca)[offset..offset + 4], &v.to_be_bytes());
        }
    }
}

/* ============ E40–E43: utils.c boundaries ======================== */

#[test]
fn e40_ull_to_bytes_outlen_zero() {
    let l = libs();
    let (c, r) = l.pair::<UllToBytes>("SPX_ull_to_bytes");
    for v in [0u64, 1, u64::MAX] {
        let mut cb = [0xAAu8; 16];
        let mut rb = [0xAAu8; 16];
        unsafe {
            c(cb.as_mut_ptr(), 0, v);
            r(rb.as_mut_ptr(), 0, v);
        }
        assert_eq!(cb, [0xAAu8; 16], "outlen == 0 must write nothing");
        assert_eq!(cb, rb);
    }
}

#[test]
fn e41_ull_to_bytes_outlen_gt8() {
    let l = libs();
    let (c, r) = l.pair::<UllToBytes>("SPX_ull_to_bytes");
    let mut rng = Rng::new(SEED + 41_00);
    for outlen in [9u32, 10, 12, 16] {
        for v in [0u64, 1, u64::MAX, rng.next_u64(), rng.next_u64()] {
            let mut cb = [0xAAu8; 32];
            let mut rb = [0xAAu8; 32];
            unsafe {
                c(cb.as_mut_ptr(), outlen, v);
                r(rb.as_mut_ptr(), outlen, v);
            }
            eq_bytes(&format!("ull_to_bytes(outlen={outlen},v={v:#x})"), &cb, &rb);
            // the excess high bytes are zero, the low 8 are the big-endian value
            let n = outlen as usize;
            assert_eq!(&cb[n - 8..n], &v.to_be_bytes());
            assert!(cb[..n - 8].iter().all(|&b| b == 0));
            assert_eq!(&cb[n..], &[0xAAu8; 32][n..]);
        }
    }
}

#[test]
fn e42_bytes_to_ull_inlen_zero() {
    let l = libs();
    let (c, r) = l.pair::<BytesToUll>("SPX_bytes_to_ull");
    let b = [0xffu8; 16];
    let (cv, rv) = unsafe { (c(b.as_ptr(), 0), r(b.as_ptr(), 0)) };
    assert_eq!(cv, rv);
    assert_eq!(cv, 0, "inlen == 0 must return 0");
}

#[test]
fn e43_bytes_to_ull_inlen_gt8() {
    let l = libs();
    let (c, r) = l.pair::<BytesToUll>("SPX_bytes_to_ull");
    let mut rng = Rng::new(SEED + 43_00);
    // For inlen > 8 the shift count 8*(inlen-1-i) reaches >= 64.  That is UB in
    // C and an over-shift in Rust; whatever the compiled C does, the Rust must
    // do the same.
    for inlen in 9u32..=16 {
        for _ in 0..NUM_ITERS {
            let b = rng.bytes(16);
            let (cv, rv) = unsafe { (c(b.as_ptr(), inlen), r(b.as_ptr(), inlen)) };
            assert_eq!(
                cv, rv,
                "bytes_to_ull(inlen={inlen}, {}) diverges",
                hex(&b)
            );
        }
        for pat in [0x00u8, 0x01, 0x80, 0xff] {
            let b = vec![pat; 16];
            let (cv, rv) = unsafe { (c(b.as_ptr(), inlen), r(b.as_ptr(), inlen)) };
            assert_eq!(cv, rv, "bytes_to_ull(inlen={inlen}, all {pat:#02x})");
        }
    }
}

/* ============ E44–E46: thash inblocks ============================ */

#[test]
fn e44_thash_inblocks_zero() {
    let l = libs();
    let (c, r) = l.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(SEED + 44_00);
    for _ in 0..8 {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let inp = rng.bytes(SPX_N); // not read for inblocks == 0
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        let mut co = vec![0xAAu8; SPX_N + 16];
        let mut ro = vec![0xAAu8; SPX_N + 16];
        unsafe {
            c(co.as_mut_ptr(), inp.as_ptr(), 0, cc.as_ptr(), ca.as_mut_ptr());
            r(ro.as_mut_ptr(), inp.as_ptr(), 0, rc.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes("thash(inblocks=0)", &co, &ro);
        eq_bytes("thash(inblocks=0) addr", abytes(&ca), abytes(&ra));
    }
}

#[test]
fn e45_thash_inblocks_boundary() {
    let l = libs();
    let (c, r) = l.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(SEED + 45_00);
    // 1 vs 2 is the branch boundary: haraka F-vs-H, and the `inblocks > 1`
    // wide-hash switch for sha2/blake when SPX_*512 is set.
    for _ in 0..8 {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let inp = rng.bytes(2 * SPX_N);
        let base = rng.addr();
        let mut outs = Vec::new();
        for inblocks in [1u32, 2] {
            let mut ca = base;
            let mut ra = base;
            let mut co = vec![0u8; SPX_N];
            let mut ro = vec![0u8; SPX_N];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), inblocks, cc.as_ptr(), ca.as_mut_ptr());
                r(ro.as_mut_ptr(), inp.as_ptr(), inblocks, rc.as_ptr(), ra.as_mut_ptr());
            }
            eq_bytes(&format!("thash(inblocks={inblocks})"), &co, &ro);
            outs.push(co);
        }
        assert_ne!(outs[0], outs[1], "inblocks 1 and 2 must not collide");
    }
}

#[test]
fn e46_thash_inblocks_large() {
    let l = libs();
    let (c, r) = l.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(SEED + 46_00);
    for inblocks in [SPX_FORS_TREES, SPX_WOTS_LEN as u32] {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let inp = rng.bytes(inblocks as usize * SPX_N);
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        let mut co = vec![0xAAu8; SPX_N + 16];
        let mut ro = vec![0xAAu8; SPX_N + 16];
        unsafe {
            c(co.as_mut_ptr(), inp.as_ptr(), inblocks, cc.as_ptr(), ca.as_mut_ptr());
            r(ro.as_mut_ptr(), inp.as_ptr(), inblocks, rc.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes(&format!("thash(inblocks={inblocks})"), &co, &ro);
    }
}

/* ============ E47–E48: tree-height boundaries ==================== */

#[test]
fn e47_compute_root_height_one() {
    let l = libs();
    let (c, r) = l.pair::<ComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(SEED + 47_00);
    // tree_height == 1 is the minimum sane value: `tree_height - 1` == 0 so the
    // loop body never runs.  tree_height == 0 underflows to 0xFFFFFFFF and would
    // loop ~2^32 times reading past auth_path, so it is deliberately not called.
    for leaf_idx in [0u32, 1, 2, 3, u32::MAX] {
        for _ in 0..4 {
            let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
            let leaf = rng.bytes(SPX_N);
            let auth = rng.bytes(SPX_N);
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            let mut croot = vec![0u8; SPX_N];
            let mut rroot = vec![0u8; SPX_N];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    0,
                    auth.as_ptr(),
                    1,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    leaf.as_ptr(),
                    leaf_idx,
                    0,
                    auth.as_ptr(),
                    1,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            eq_bytes(&format!("compute_root(h=1,leaf_idx={leaf_idx})"), &croot, &rroot);
            eq_bytes("compute_root(h=1) addr", abytes(&ca), abytes(&ra));
        }
    }
}

unsafe extern "C" fn synth_leaf(
    leaf: *mut u8,
    _ctx: *const u8,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let ta = std::slice::from_raw_parts(tree_addr, 8);
    let mut h = 0xABCD_EF01_2345_6789u64 ^ (addr_idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for w in ta {
        h ^= (*w as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = h.rotate_left(23).wrapping_add(0x94D0_49BB_1331_11EB);
    }
    let out = std::slice::from_raw_parts_mut(leaf, SPX_N);
    for (i, b) in out.iter_mut().enumerate() {
        h = (h ^ (h >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9).wrapping_add(i as u64);
        *b = (h >> 32) as u8;
    }
}

#[test]
fn e48_treehash_height_zero() {
    let l = libs();
    let (c, r) = l.pair::<TreeHash>("SPX_treehash");
    let mut rng = Rng::new(SEED + 48_00);
    for leaf_idx in [0u32, 1, 2, u32::MAX] {
        for idx_offset in [0u32, 1, 0x1000] {
            let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
            let base = rng.addr();
            let mut cta = base;
            let mut rta = base;
            let mut cauth = vec![0xCCu8; SPX_N];
            let mut rauth = vec![0xCCu8; SPX_N];
            let mut croot = vec![0u8; SPX_N];
            let mut rroot = vec![0u8; SPX_N];
            unsafe {
                c(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    cc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    0,
                    synth_leaf,
                    cta.as_mut_ptr(),
                );
                r(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    rc.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    0,
                    synth_leaf,
                    rta.as_mut_ptr(),
                );
            }
            let tag = format!("treehash(h=0,leaf_idx={leaf_idx},off={idx_offset})");
            eq_bytes(&format!("{tag} root"), &croot, &rroot);
            eq_bytes(&format!("{tag} auth_path"), &cauth, &rauth);
            eq_bytes(&format!("{tag} tree_addr"), abytes(&cta), abytes(&rta));
        }
    }
}

/* ============ E49–E50 ============================================ */

#[test]
fn e49_wots_chain_clamp() {
    let l = libs();
    let (c, r) = l.pair::<WotsPkFromSig>("SPX_wots_pk_from_sig");
    let mut rng = Rng::new(SEED + 49_00);
    // all-0x00 msg  -> chain_lengths all 0  -> gen_chain(start=0, steps=w-1)
    // all-0xff msg  -> chain_lengths all w-1 -> gen_chain(start=w-1, steps=0)
    // Both hit the `i < SPX_WOTS_W` clamp in gen_chain from opposite ends.
    for msg in [vec![0u8; SPX_N], vec![0xffu8; SPX_N]] {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let sig = rng.bytes(SPX_WOTS_BYTES);
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        let mut cpk = vec![0u8; SPX_WOTS_BYTES];
        let mut rpk = vec![0u8; SPX_WOTS_BYTES];
        unsafe {
            c(cpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr(), ca.as_mut_ptr());
            r(rpk.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rc.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes(&format!("wots_pk_from_sig(msg={:#04x}..)", msg[0]), &cpk, &rpk);
        eq_bytes("wots_pk_from_sig addr", abytes(&ca), abytes(&ra));
    }
}

#[test]
fn e50_wots_gen_leafx1_null_sig() {
    let l = libs();
    let (c, r) = l.pair::<WotsGenLeafX1>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(SEED + 50_00);
    for _ in 0..8 {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let leaf_idx = rng.next_u32();
        // wots_sign_leaf != leaf_idx  =>  wots_k_mask = ~0  =>  `k == wots_k`
        // can never hold, so wots_sig (NULL here) is never dereferenced.
        let sign_leaf = leaf_idx.wrapping_add(1);
        let mut steps: Vec<u32> = (0..SPX_WOTS_LEN).map(|_| rng.next_u32()).collect();
        let mut steps2 = steps.clone();
        let base_leaf = rng.addr();
        let base_pk = rng.addr();
        let mut cinfo = LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: sign_leaf,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: base_leaf,
            pk_addr: base_pk,
        };
        let mut rinfo = LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: sign_leaf,
            wots_steps: steps2.as_mut_ptr(),
            leaf_addr: base_leaf,
            pk_addr: base_pk,
        };
        let mut cd = vec![0u8; SPX_N];
        let mut rd = vec![0u8; SPX_N];
        unsafe {
            c(cd.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut cinfo);
            r(rd.as_mut_ptr(), rc.as_ptr(), leaf_idx, &mut rinfo);
        }
        eq_bytes("wots_gen_leafx1(NULL sig) leaf", &cd, &rd);
        eq_bytes("leaf_addr", abytes(&cinfo.leaf_addr), abytes(&rinfo.leaf_addr));
        eq_bytes("pk_addr", abytes(&cinfo.pk_addr), abytes(&rinfo.pk_addr));
    }
}

/* ============ generic FFI boundaries ============================= */

/// Out-of-range enum values crossing the FFI boundary, applied through the
/// *composed* API rather than a single setter: an ADRS carrying an invalid type
/// byte is fed to prf_addr / thash / wots_pk_from_sig.
#[test]
fn e51_out_of_range_type_through_pipeline() {
    let l = libs();
    let (cset, rset) = l.pair::<SetU32>("SPX_set_type");
    let (cprf, rprf) = l.pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u32)>("SPX_prf_addr");
    let (cth, rth) = l.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(SEED + 51_00);
    for ty in [7u32, 8, 42, 255, 256, 1000, u32::MAX] {
        let (cc, rc) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            cset(ca.as_mut_ptr(), ty);
            rset(ra.as_mut_ptr(), ty);
        }
        let mut cp = vec![0u8; SPX_N];
        let mut rp = vec![0u8; SPX_N];
        unsafe {
            cprf(cp.as_mut_ptr(), cc.as_ptr(), ca.as_ptr());
            rprf(rp.as_mut_ptr(), rc.as_ptr(), ra.as_ptr());
        }
        eq_bytes(&format!("prf_addr with type={ty}"), &cp, &rp);
        let inp = rng.bytes(SPX_N);
        let mut co = vec![0u8; SPX_N];
        let mut ro = vec![0u8; SPX_N];
        unsafe {
            cth(co.as_mut_ptr(), inp.as_ptr(), 1, cc.as_ptr(), ca.as_mut_ptr());
            rth(ro.as_mut_ptr(), inp.as_ptr(), 1, rc.as_ptr(), ra.as_mut_ptr());
        }
        eq_bytes(&format!("thash with type={ty}"), &co, &ro);
    }
}

/// The size accessors take no input and can never fail; assert the exact values
/// so a wrong parameter set is caught as an error, not a silent difference.
#[test]
fn e52_size_accessors_exact() {
    let l = libs();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let (c, r) = l.pair::<SizeFn>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{name}");
        assert_eq!(cv, expect, "{name}");
    }
}
