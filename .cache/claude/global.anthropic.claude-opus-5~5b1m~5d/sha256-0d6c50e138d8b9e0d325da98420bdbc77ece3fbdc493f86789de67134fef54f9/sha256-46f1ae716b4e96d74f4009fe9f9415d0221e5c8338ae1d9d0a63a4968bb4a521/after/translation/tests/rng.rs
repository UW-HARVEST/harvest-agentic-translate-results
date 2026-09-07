//! Phase B rows 54-59: the deterministic RNG (`app/src/rng.c`).
//!
//! Includes the exported `DRBG_ctx` **data** symbol, which the tests read
//! through `dlsym` on both `.so`s to prove the state stays in lockstep.
mod common;
use common::*;

type FnRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FnRb = unsafe extern "C" fn(*mut u8, core::ffi::c_ulonglong) -> core::ffi::c_int;
type FnAesEcb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FnDrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FnSeInit = unsafe extern "C" fn(
    *mut AesXofFfi,
    *mut u8,
    *mut u8,
    core::ffi::c_ulong,
) -> core::ffi::c_int;
type FnSe =
    unsafe extern "C" fn(*mut AesXofFfi, *mut u8, core::ffi::c_ulong) -> core::ffi::c_int;
type FnDrbgPtr = *mut DrbgCtxFfi;

fn drbg(which: Which) -> DrbgCtxFfi {
    let p = libs().sym::<FnDrbgPtr>(which, "DRBG_ctx");
    unsafe { (**p).clone() }
}

fn drbg_bytes(v: &DrbgCtxFfi) -> Vec<u8> {
    let mut o = Vec::with_capacity(52);
    o.extend_from_slice(&v.Key);
    o.extend_from_slice(&v.V);
    o.extend_from_slice(&v.reseed_counter.to_le_bytes());
    o
}

#[track_caller]
fn assert_drbg_eq(what: &str) {
    assert_bytes_eq(
        &format!("DRBG_ctx {what}"),
        &drbg_bytes(&drbg(Which::C)),
        &drbg_bytes(&drbg(Which::R)),
    );
}

// --- rows 54 / 55 -------------------------------------------------------

fn randombytes_sequence(pers: Option<[u8; 48]>, seed: u64, label: &str) {
    let _g = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<FnRbInit>("randombytes_init");
    let (cb, rb) = l.pair::<FnRb>("randombytes");
    let mut rng = Rng::new(seed);

    for round in 0..8 {
        let mut ent = [0u8; 48];
        if round == 0 {
            for (i, b) in ent.iter_mut().enumerate() {
                *b = i as u8; // the driver's entropy
            }
        } else {
            rng.fill(&mut ent);
        }

        let mut e1 = ent;
        let mut e2 = ent;
        let mut p1 = pers;
        let mut p2 = pers;
        unsafe {
            ci(
                e1.as_mut_ptr(),
                p1.as_mut().map_or(core::ptr::null_mut(), |p| p.as_mut_ptr()),
            );
            ri(
                e2.as_mut_ptr(),
                p2.as_mut().map_or(core::ptr::null_mut(), |p| p.as_mut_ptr()),
            );
        }
        assert_drbg_eq(&format!("{label} after init (round {round})"));

        for &xlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 48, 64, 100, 1000] {
            let mut co = vec![0xA5u8; xlen + 8];
            let mut ro = vec![0xA5u8; xlen + 8];
            let cv = unsafe { cb(co.as_mut_ptr(), xlen as core::ffi::c_ulonglong) };
            let rv = unsafe { rb(ro.as_mut_ptr(), xlen as core::ffi::c_ulonglong) };
            assert_eq_dbg(&format!("{label} randombytes({xlen}) retval"), cv, rv);
            assert_eq_dbg(&format!("{label} randombytes({xlen}) == RNG_SUCCESS"), cv, 0);
            assert_bytes_eq(&format!("{label} randombytes({xlen}) output"), &co, &ro);
            assert_drbg_eq(&format!("{label} after randombytes({xlen})"));
        }
    }
}

#[test]
fn row54_randombytes_null_personalization() {
    randombytes_sequence(None, 0x5401, "pers=NULL");
}

#[test]
fn row55_randombytes_with_personalization() {
    let mut rng = Rng::new(0x5501);
    let mut p = [0u8; 48];
    rng.fill(&mut p);
    randombytes_sequence(Some(p), 0x5502, "pers=random");
    // Also the two documented extremes of the XOR branch.
    randombytes_sequence(Some([0u8; 48]), 0x5503, "pers=all-zero");
    randombytes_sequence(Some([0xFFu8; 48]), 0x5504, "pers=all-ones");
}

// --- row 56 -------------------------------------------------------------

#[test]
fn row56_aes256_ecb() {
    let l = libs();
    let (c, r) = l.pair::<FnAesEcb>("AES256_ECB");
    let mut rng = Rng::new(0x5601);
    for it in 0..512 {
        let mut key = match it {
            0 => vec![0x00u8; 32],
            1 => vec![0xFFu8; 32],
            2 => (0..32u8).collect(),
            _ => rng.bytes(32),
        };
        let mut ctr = match it {
            0 => vec![0x00u8; 16],
            1 => vec![0xFFu8; 16],
            2 => (0..16u8).collect(),
            _ => rng.bytes(16),
        };
        let mut k2 = key.clone();
        let mut c2 = ctr.clone();
        let mut co = vec![0xA5u8; 24];
        let mut ro = vec![0xA5u8; 24];
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), co.as_mut_ptr());
            r(k2.as_mut_ptr(), c2.as_mut_ptr(), ro.as_mut_ptr());
        }
        assert_bytes_eq(&format!("AES256_ECB #{it} out"), &co, &ro);
        // The C must not modify key or ctr.
        assert_bytes_eq(&format!("AES256_ECB #{it} key unchanged"), &key, &k2);
        assert_bytes_eq(&format!("AES256_ECB #{it} ctr unchanged"), &ctr, &c2);
    }
}

// --- row 57 -------------------------------------------------------------

#[test]
fn row57_aes256_ctr_drbg_update() {
    let l = libs();
    let (c, r) = l.pair::<FnDrbgUpdate>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(0x5701);
    for it in 0..256 {
        let key0 = if it == 0 { vec![0u8; 32] } else { rng.bytes(32) };
        // V all-0xFF forces the 128-bit big-endian carry to propagate the whole
        // way through the increment loop.
        let v0 = match it {
            0 => vec![0u8; 16],
            1 => vec![0xFFu8; 16],
            2 => {
                let mut v = vec![0u8; 16];
                v[15] = 0xFF;
                v
            }
            3 => {
                let mut v = vec![0xFFu8; 16];
                v[0] = 0x00;
                v
            }
            _ => rng.bytes(16),
        };
        let use_pd = it % 3 != 0;
        let pd = rng.bytes(48);

        let mut ck = key0.clone();
        let mut cv = v0.clone();
        let mut cp = pd.clone();
        let mut rk = key0.clone();
        let mut rv = v0.clone();
        let mut rp = pd.clone();
        unsafe {
            c(
                if use_pd { cp.as_mut_ptr() } else { core::ptr::null_mut() },
                ck.as_mut_ptr(),
                cv.as_mut_ptr(),
            );
            r(
                if use_pd { rp.as_mut_ptr() } else { core::ptr::null_mut() },
                rk.as_mut_ptr(),
                rv.as_mut_ptr(),
            );
        }
        let m = if use_pd { "with provided_data" } else { "provided_data=NULL" };
        assert_bytes_eq(&format!("AES256_CTR_DRBG_Update #{it} [{m}] Key"), &ck, &rk);
        assert_bytes_eq(&format!("AES256_CTR_DRBG_Update #{it} [{m}] V"), &cv, &rv);
        assert_bytes_eq(
            &format!("AES256_CTR_DRBG_Update #{it} [{m}] provided_data unchanged"),
            &cp,
            &rp,
        );
    }
}

// --- rows 58 / 59 -------------------------------------------------------

#[test]
fn row58_seedexpander_sequences() {
    let l = libs();
    let (ci, ri) = l.pair::<FnSeInit>("seedexpander_init");
    let (cs, rs) = l.pair::<FnSe>("seedexpander");
    let mut rng = Rng::new(0x5801);

    for &maxlen in &[1u64, 2, 16, 17, 33, 256, 65536, 0xFFFF_FFFF] {
        for _ in 0..8 {
            let mut seed = rng.bytes(32);
            let mut div = rng.bytes(8);
            let mut seed2 = seed.clone();
            let mut div2 = div.clone();

            let mut cx = AesXofFfi::zeroed();
            let mut rx = AesXofFfi::zeroed();
            let cv = unsafe {
                ci(
                    &mut cx,
                    seed.as_mut_ptr(),
                    div.as_mut_ptr(),
                    maxlen as core::ffi::c_ulong,
                )
            };
            let rv = unsafe {
                ri(
                    &mut rx,
                    seed2.as_mut_ptr(),
                    div2.as_mut_ptr(),
                    maxlen as core::ffi::c_ulong,
                )
            };
            assert_eq_dbg(&format!("seedexpander_init(maxlen={maxlen}) retval"), cv, rv);
            assert_eq_dbg(&format!("seedexpander_init(maxlen={maxlen}) == RNG_SUCCESS"), cv, 0);
            assert_bytes_eq(
                &format!("seedexpander_init(maxlen={maxlen}) AES_XOF_struct (80 bytes)"),
                cx.as_bytes(),
                rx.as_bytes(),
            );

            for &xlen in &[0u64, 1, 7, 15, 16, 17, 31, 32, 33, 64, 100, 300] {
                let mut co = vec![0xA5u8; xlen as usize + 8];
                let mut ro = vec![0xA5u8; xlen as usize + 8];
                let a = unsafe {
                    cs(&mut cx, co.as_mut_ptr(), xlen as core::ffi::c_ulong)
                };
                let b = unsafe {
                    rs(&mut rx, ro.as_mut_ptr(), xlen as core::ffi::c_ulong)
                };
                assert_eq_dbg(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) retval"),
                    a,
                    b,
                );
                assert_bytes_eq(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) output"),
                    &co,
                    &ro,
                );
                assert_bytes_eq(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ctx after"),
                    cx.as_bytes(),
                    rx.as_bytes(),
                );
            }
        }
    }
}

#[test]
fn row59_seedexpander_counter_rollover() {
    // Pre-set ctr[12..16] to 0xFFFFFFFF so the 4-byte counter increment inside
    // `seedexpander` wraps all the way through.
    let l = libs();
    let (cs, rs) = l.pair::<FnSe>("seedexpander");
    let (ci, ri) = l.pair::<FnSeInit>("seedexpander_init");
    let mut rng = Rng::new(0x5901);

    for pattern in [
        [0xFFu8, 0xFF, 0xFF, 0xFF],
        [0x00, 0xFF, 0xFF, 0xFF],
        [0xFF, 0xFF, 0xFF, 0xFE],
        [0x12, 0x34, 0x56, 0xFF],
    ] {
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut seed2 = seed.clone();
        let mut div2 = div.clone();
        let mut cx = AesXofFfi::zeroed();
        let mut rx = AesXofFfi::zeroed();
        unsafe {
            ci(&mut cx, seed.as_mut_ptr(), div.as_mut_ptr(), 0xFFFF_FFFF);
            ri(&mut rx, seed2.as_mut_ptr(), div2.as_mut_ptr(), 0xFFFF_FFFF);
        }
        cx.ctr[12..16].copy_from_slice(&pattern);
        rx.ctr[12..16].copy_from_slice(&pattern);

        // 300 bytes forces ~19 AES blocks, i.e. many counter increments.
        for &xlen in &[300u64, 300, 300] {
            let mut co = vec![0xA5u8; xlen as usize + 8];
            let mut ro = vec![0xA5u8; xlen as usize + 8];
            let a = unsafe { cs(&mut cx, co.as_mut_ptr(), xlen as core::ffi::c_ulong) };
            let b = unsafe { rs(&mut rx, ro.as_mut_ptr(), xlen as core::ffi::c_ulong) };
            assert_eq_dbg(&format!("seedexpander rollover {pattern:?} retval"), a, b);
            assert_bytes_eq(&format!("seedexpander rollover {pattern:?} out"), &co, &ro);
            assert_bytes_eq(
                &format!("seedexpander rollover {pattern:?} ctx"),
                cx.as_bytes(),
                rx.as_bytes(),
            );
        }
    }
}
