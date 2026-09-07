//! Phase B + C — differential tests for `app/src/rng.c` (the NIST
//! AES-256-CTR-DRBG and the seed expander), including every error return.

mod common;
use common::*;

type Aes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type DrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type Randombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type SeedexpanderInit = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8, u64) -> i32;
type Seedexpander = unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32;

/// sizeof(AES256_CTR_DRBG_struct) = 32 + 16 + 4
const DRBG_SIZE: usize = 52;
/// sizeof(AES_XOF_struct) = 16 + 8 + 8 + 32 + 16
const XOF_SIZE: usize = 80;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

#[test]
fn aes256_ecb_matches() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<Aes256Ecb>("AES256_ECB");
    let mut rng = Rng::new(0xAE50);
    for _ in 0..2000 {
        let mut key = rng.bytes(32);
        let mut ctr = rng.bytes(16);
        let mut cb = vec![0xEEu8; 16];
        let mut rb = vec![0xEEu8; 16];
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), cb.as_mut_ptr());
            r(key.as_mut_ptr(), ctr.as_mut_ptr(), rb.as_mut_ptr());
        }
        assert_bytes_eq("AES256_ECB", &cb, &rb);
    }
    // FIPS-197 style extremes
    for (kp, cp) in [(0x00u8, 0x00u8), (0xff, 0xff), (0x00, 0xff), (0xff, 0x00)] {
        let mut key = vec![kp; 32];
        let mut ctr = vec![cp; 16];
        let mut cb = vec![0u8; 16];
        let mut rb = vec![0u8; 16];
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), cb.as_mut_ptr());
            r(key.as_mut_ptr(), ctr.as_mut_ptr(), rb.as_mut_ptr());
        }
        assert_bytes_eq("AES256_ECB extreme", &cb, &rb);
    }
}

#[test]
fn drbg_update_matches_with_and_without_provided_data() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<DrbgUpdate>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(0xD8B6);
    for i in 0..500 {
        let mut ck = rng.bytes(32);
        let mut cv = rng.bytes(16);
        let mut rk = ck.clone();
        let mut rv = cv.clone();
        // provided_data == NULL is a distinct branch in the C
        let mut pd = rng.bytes(48);
        let (cp, rp) = if i % 2 == 0 {
            (pd.as_mut_ptr(), pd.as_mut_ptr())
        } else {
            (std::ptr::null_mut(), std::ptr::null_mut())
        };
        unsafe {
            c(cp, ck.as_mut_ptr(), cv.as_mut_ptr());
            r(rp, rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        assert_bytes_eq("AES256_CTR_DRBG_Update Key", &ck, &rk);
        assert_bytes_eq("AES256_CTR_DRBG_Update V", &cv, &rv);
    }
    // V immediately before a carry, so the increment loop ripples
    for pat in [
        vec![0xffu8; 16],
        {
            let mut v = vec![0u8; 16];
            v[15] = 0xff;
            v
        },
        {
            let mut v = vec![0xffu8; 16];
            v[0] = 0x00;
            v
        },
    ] {
        let mut ck = vec![0x11u8; 32];
        let mut rk = ck.clone();
        let mut cv = pat.clone();
        let mut rv = pat.clone();
        unsafe {
            c(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        assert_bytes_eq("DRBG_Update carry Key", &ck, &rk);
        assert_bytes_eq("DRBG_Update carry V", &cv, &rv);
    }
}

/// `randombytes_init` + a stream of `randombytes` calls, comparing both the
/// produced bytes and the exported `DRBG_ctx` global after every step.
#[test]
fn randombytes_stream_and_drbg_ctx_state() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<RandombytesInit>("randombytes_init");
    let (cr, rr) = libs.pair::<Randombytes>("randombytes");
    let cctx = libs.c_data("DRBG_ctx");
    let rctx = libs.r_data("DRBG_ctx");
    let mut rng = Rng::new(0xDBB6);

    let read_state = |p: *mut u8| unsafe { std::slice::from_raw_parts(p, DRBG_SIZE).to_vec() };

    for trial in 0..20 {
        let mut ent = rng.bytes(48);
        let mut pers = rng.bytes(48);
        // personalization_string == NULL is a distinct branch
        let use_pers = trial % 2 == 1;

        unsafe {
            if use_pers {
                ci(ent.as_mut_ptr(), pers.as_mut_ptr());
            } else {
                ci(ent.as_mut_ptr(), std::ptr::null_mut());
            }
        }
        let s_c = read_state(cctx);
        unsafe {
            if use_pers {
                ri(ent.as_mut_ptr(), pers.as_mut_ptr());
            } else {
                ri(ent.as_mut_ptr(), std::ptr::null_mut());
            }
        }
        let s_r = read_state(rctx);
        assert_bytes_eq("DRBG_ctx after randombytes_init", &s_c, &s_r);

        // Reseed both with the same entropy, then draw the same lengths from
        // each and compare.
        for &xlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 48, 63, 64, 100, 1000] {
            unsafe {
                if use_pers {
                    ci(ent.as_mut_ptr(), pers.as_mut_ptr());
                } else {
                    ci(ent.as_mut_ptr(), std::ptr::null_mut());
                }
            }
            let mut cb = vec![0xEEu8; xlen + 8];
            let crc = unsafe { cr(cb.as_mut_ptr(), xlen as u64) };
            let cstate = read_state(cctx);

            unsafe {
                if use_pers {
                    ri(ent.as_mut_ptr(), pers.as_mut_ptr());
                } else {
                    ri(ent.as_mut_ptr(), std::ptr::null_mut());
                }
            }
            let mut rb = vec![0xEEu8; xlen + 8];
            let rrc = unsafe { rr(rb.as_mut_ptr(), xlen as u64) };
            let rstate = read_state(rctx);

            assert_eq!(crc, rrc, "randombytes return (xlen={})", xlen);
            assert_eq!(crc, RNG_SUCCESS);
            assert_bytes_eq(&format!("randombytes out (xlen={})", xlen), &cb, &rb);
            assert_bytes_eq(&format!("DRBG_ctx after randombytes({})", xlen), &cstate, &rstate);
        }

        // A long chain of successive draws (state must evolve identically).
        unsafe {
            ci(ent.as_mut_ptr(), std::ptr::null_mut());
            ri(ent.as_mut_ptr(), std::ptr::null_mut());
        }
        for step in 0..25usize {
            let xlen = (step * 7) % 40;
            let mut cb = vec![0u8; xlen];
            let mut rb = vec![0u8; xlen];
            unsafe {
                cr(cb.as_mut_ptr(), xlen as u64);
                rr(rb.as_mut_ptr(), xlen as u64);
            }
            assert_bytes_eq(&format!("chained randombytes step {}", step), &cb, &rb);
            assert_bytes_eq(
                &format!("chained DRBG_ctx step {}", step),
                &read_state(cctx),
                &read_state(rctx),
            );
        }
    }
}

#[test]
fn seedexpander_valid_paths() {
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let (cs, rs) = libs.pair::<Seedexpander>("seedexpander");
    let mut rng = Rng::new(0x5EE0);

    for _ in 0..100 {
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        for &maxlen in &[
            1u64,
            16,
            17,
            256,
            0x1_0000,
            0xffff_ffff,
            0xffff_fffe,
            100_000,
        ] {
            let mut cx = vec![0u8; XOF_SIZE];
            let mut rx = vec![0u8; XOF_SIZE];
            let crc = unsafe { ci(cx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
            let rrc = unsafe { ri(rx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
            assert_eq!(crc, rrc, "seedexpander_init rc (maxlen={})", maxlen);
            assert_eq!(crc, RNG_SUCCESS);
            assert_bytes_eq(
                &format!("AES_XOF_struct after init (maxlen={})", maxlen),
                &cx,
                &rx,
            );

            if maxlen < 4 {
                continue;
            }
            // Chained squeezes across the internal 16-byte buffer boundary.
            for step in 0..8usize {
                let xlen = std::cmp::min((step * 5 + 1) as u64, maxlen.saturating_sub(1));
                if xlen == 0 {
                    continue;
                }
                let mut cb = vec![0xEEu8; xlen as usize + 4];
                let mut rb = vec![0xEEu8; xlen as usize + 4];
                let c1 = unsafe { cs(cx.as_mut_ptr(), cb.as_mut_ptr(), xlen) };
                let r1 = unsafe { rs(rx.as_mut_ptr(), rb.as_mut_ptr(), xlen) };
                assert_eq!(c1, r1, "seedexpander rc (maxlen={}, xlen={})", maxlen, xlen);
                assert_bytes_eq(
                    &format!("seedexpander out (maxlen={}, xlen={})", maxlen, xlen),
                    &cb,
                    &rb,
                );
                assert_bytes_eq(
                    &format!("AES_XOF_struct after squeeze (maxlen={}, xlen={})", maxlen, xlen),
                    &cx,
                    &rx,
                );
                if c1 != RNG_SUCCESS {
                    break;
                }
            }
        }
    }
}

// ==================================================================
// Phase C — error paths of rng.c
// ==================================================================

/// ERRORS.md row: `seedexpander_init`, `maxlen >= 0x100000000` -> RNG_BAD_MAXLEN
#[test]
fn err_seedexpander_init_bad_maxlen() {
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let mut seed = vec![0x11u8; 32];
    let mut div = vec![0x22u8; 8];
    for &maxlen in &[
        0x1_0000_0000u64,
        0x1_0000_0001,
        0x2_0000_0000,
        u64::MAX,
        0xffff_ffff_ffff_0000,
    ] {
        let mut cx = vec![0x5Au8; XOF_SIZE];
        let mut rx = vec![0x5Au8; XOF_SIZE];
        let crc = unsafe { ci(cx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
        let rrc = unsafe { ri(rx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
        assert_eq!(crc, RNG_BAD_MAXLEN, "C must reject maxlen={:#x}", maxlen);
        assert_eq!(rrc, crc, "seedexpander_init(maxlen={:#x})", maxlen);
        // the context must be left untouched by both
        assert_bytes_eq("ctx untouched on RNG_BAD_MAXLEN", &cx, &rx);
    }
    // one step below the limit is accepted
    let mut cx = vec![0u8; XOF_SIZE];
    let mut rx = vec![0u8; XOF_SIZE];
    let crc = unsafe { ci(cx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 0xffff_ffff) };
    let rrc = unsafe { ri(rx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 0xffff_ffff) };
    assert_eq!(crc, RNG_SUCCESS);
    assert_eq!(rrc, RNG_SUCCESS);
    assert_bytes_eq("ctx at maxlen=0xffffffff", &cx, &rx);
}

/// ERRORS.md row: `seedexpander`, `x == NULL` -> RNG_BAD_OUTBUF
#[test]
fn err_seedexpander_null_outbuf() {
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let (cs, rs) = libs.pair::<Seedexpander>("seedexpander");
    let mut seed = vec![0x33u8; 32];
    let mut div = vec![0x44u8; 8];
    let mut cx = vec![0u8; XOF_SIZE];
    let mut rx = vec![0u8; XOF_SIZE];
    unsafe {
        ci(cx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 1024);
        ri(rx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 1024);
    }
    // The NULL check comes first, so it wins even for an otherwise invalid xlen.
    for &xlen in &[0u64, 1, 16, 1024, 100_000] {
        let crc = unsafe { cs(cx.as_mut_ptr(), std::ptr::null_mut(), xlen) };
        let rrc = unsafe { rs(rx.as_mut_ptr(), std::ptr::null_mut(), xlen) };
        assert_eq!(crc, RNG_BAD_OUTBUF, "C must return RNG_BAD_OUTBUF");
        assert_eq!(rrc, crc, "seedexpander(x=NULL, xlen={})", xlen);
        assert_bytes_eq("ctx untouched on RNG_BAD_OUTBUF", &cx, &rx);
    }
}

/// ERRORS.md row: `seedexpander`, `xlen >= ctx->length_remaining`
/// -> RNG_BAD_REQ_LEN
#[test]
fn err_seedexpander_bad_req_len() {
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let (cs, rs) = libs.pair::<Seedexpander>("seedexpander");
    let mut seed = vec![0x55u8; 32];
    let mut div = vec![0x66u8; 8];

    for &maxlen in &[0u64, 1, 2, 16, 100] {
        let mut cx = vec![0u8; XOF_SIZE];
        let mut rx = vec![0u8; XOF_SIZE];
        unsafe {
            ci(cx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            ri(rx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
        }
        // exactly at the limit, and past it
        for &xlen in &[maxlen, maxlen + 1, maxlen + 1000, u64::MAX] {
            let mut cb = vec![0xEEu8; 8];
            let mut rb = vec![0xEEu8; 8];
            let crc = unsafe { cs(cx.as_mut_ptr(), cb.as_mut_ptr(), xlen) };
            let rrc = unsafe { rs(rx.as_mut_ptr(), rb.as_mut_ptr(), xlen) };
            assert_eq!(
                crc, RNG_BAD_REQ_LEN,
                "C must reject xlen={} with maxlen={}",
                xlen, maxlen
            );
            assert_eq!(rrc, crc, "seedexpander(maxlen={}, xlen={})", maxlen, xlen);
            assert_bytes_eq("out untouched on RNG_BAD_REQ_LEN", &cb, &rb);
            assert_bytes_eq("ctx untouched on RNG_BAD_REQ_LEN", &cx, &rx);
        }
        // maxlen - 1 is the largest accepted request
        if maxlen >= 1 {
            let xlen = maxlen - 1;
            let mut cb = vec![0xEEu8; (xlen as usize) + 4];
            let mut rb = vec![0xEEu8; (xlen as usize) + 4];
            let crc = unsafe { cs(cx.as_mut_ptr(), cb.as_mut_ptr(), xlen) };
            let rrc = unsafe { rs(rx.as_mut_ptr(), rb.as_mut_ptr(), xlen) };
            assert_eq!(crc, rrc);
            assert_bytes_eq("seedexpander at the accepted limit", &cb, &rb);
            assert_bytes_eq("ctx at the accepted limit", &cx, &rx);
        }
    }
}

/// `randombytes(x, 0)` still runs the trailing DRBG update and returns
/// RNG_SUCCESS in the C — verify Rust does the same.
#[test]
fn randombytes_zero_length() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<RandombytesInit>("randombytes_init");
    let (cr, rr) = libs.pair::<Randombytes>("randombytes");
    let cctx = libs.c_data("DRBG_ctx");
    let rctx = libs.r_data("DRBG_ctx");
    let mut ent: Vec<u8> = (0..48u8).collect();

    unsafe { ci(ent.as_mut_ptr(), std::ptr::null_mut()) };
    let crc = unsafe { cr(std::ptr::null_mut(), 0) };
    let cstate = unsafe { std::slice::from_raw_parts(cctx, DRBG_SIZE).to_vec() };

    unsafe { ri(ent.as_mut_ptr(), std::ptr::null_mut()) };
    let rrc = unsafe { rr(std::ptr::null_mut(), 0) };
    let rstate = unsafe { std::slice::from_raw_parts(rctx, DRBG_SIZE).to_vec() };

    assert_eq!(crc, rrc);
    assert_eq!(crc, RNG_SUCCESS);
    assert_bytes_eq("DRBG_ctx after randombytes(NULL, 0)", &cstate, &rstate);
}
