//! Phase B rows 54-58: `rng.c` -- the NIST AES-256-CTR DRBG, `AES256_ECB`,
//! `AES256_CTR_DRBG_Update` and the seed expander, including the exported
//! `DRBG_ctx` global.

mod common;
use common::*;

type FRandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FRandombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FAesEcb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FDrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FSeedexpanderInit = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8, u64) -> i32;
type FSeedexpander = unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32;

/// `AES_XOF_struct` from `app/include/rng.h`:
/// `u8 buffer[16]; unsigned long buffer_pos; unsigned long length_remaining;
///  u8 key[32]; u8 ctr[16];` -- 8-byte aligned, so 16+8(pad)+8+8+32+16 = 80.
pub const XOF_BYTES: usize = 80;
/// `AES256_CTR_DRBG_struct`: `u8 Key[32]; u8 V[16]; int reseed_counter;` = 52.
pub const DRBG_BYTES: usize = 52;

fn drbg_images() -> (Vec<u8>, Vec<u8>) {
    let (cd, rd) = both_data!("DRBG_ctx", [u8; DRBG_BYTES]);
    unsafe { ((*cd).to_vec(), (*rd).to_vec()) }
}

// ---------------------------------------------------------------------------
// Rows 54-55 -- randombytes_init + randombytes, with and without a
// personalization string; the exported DRBG_ctx is compared after every call.
// ---------------------------------------------------------------------------
fn randombytes_sequence(ps: Option<&[u8; 48]>, label: &str) {
    let _g = drbg_guard();
    let (ci, ri) = both!("randombytes_init", FRandombytesInit);
    let (c, r) = both!("randombytes", FRandombytes);

    let mut e1 = kat_entropy();
    let mut e2 = kat_entropy();
    let mut p1 = ps.copied().unwrap_or([0u8; 48]);
    let mut p2 = p1;
    unsafe {
        match ps {
            None => {
                ci(e1.as_mut_ptr(), std::ptr::null_mut());
                ri(e2.as_mut_ptr(), std::ptr::null_mut());
            }
            Some(_) => {
                ci(e1.as_mut_ptr(), p1.as_mut_ptr());
                ri(e2.as_mut_ptr(), p2.as_mut_ptr());
            }
        }
    }
    let (cd, rd) = drbg_images();
    eq_bytes(&format!("{label}: DRBG_ctx after randombytes_init"), &cd, &rd);
    eq(
        &format!("{label}: reseed_counter == 1 after init"),
        i32::from_le_bytes(cd[48..52].try_into().unwrap()),
        1,
    );

    for &n in &[0usize, 1, 15, 16, 17, 31, 32, 33, 48, 137, 1000] {
        // Prefill with a marker: for n == 15 the C only writes 15 bytes even
        // though it computed a whole 16-byte AES block, so byte 15 must stay.
        let mut cb = vec![0x5Au8; n + 8];
        let mut rb = vec![0x5Au8; n + 8];
        let (crc, rrc) = unsafe { (c(cb.as_mut_ptr(), n as u64), r(rb.as_mut_ptr(), n as u64)) };
        eq(&format!("{label}: randombytes({n}) return"), crc, rrc);
        eq(&format!("{label}: randombytes({n}) return == 0"), crc, 0);
        eq_bytes(&format!("{label}: randombytes({n}) output"), &cb, &rb);
        let (cd, rd) = drbg_images();
        eq_bytes(
            &format!("{label}: DRBG_ctx after randombytes({n})"),
            &cd,
            &rd,
        );
    }
}

#[test]
fn row54_randombytes_no_personalization() {
    randombytes_sequence(None, "ps=NULL");
}

#[test]
fn row55_randombytes_with_personalization() {
    let mut rng = Rng::new(RNG_SEED ^ 55);
    let mut ps = [0u8; 48];
    rng.fill(&mut ps);
    randombytes_sequence(Some(&ps), "ps=random");
    randombytes_sequence(Some(&[0x00u8; 48]), "ps=all00");
    randombytes_sequence(Some(&[0xFFu8; 48]), "ps=allFF");
}

// ---------------------------------------------------------------------------
// Row 56 -- AES256_ECB
// ---------------------------------------------------------------------------
#[test]
fn row56_aes256_ecb() {
    let (c, r) = both!("AES256_ECB", FAesEcb);
    let mut rng = Rng::new(RNG_SEED ^ 56);
    let mut cases: Vec<([u8; 32], [u8; 16])> = vec![
        ([0x00; 32], [0x00; 16]),
        ([0xFF; 32], [0xFF; 16]),
        ([0x00; 32], [0xFF; 16]),
        ([0xFF; 32], [0x00; 16]),
    ];
    // FIPS-197 C.3 AES-256 vector: key 00..1F, plaintext 00112233..EEFF
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = i as u8;
    }
    let pt: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
        0xFF,
    ];
    cases.push((k, pt));
    for _ in 0..N_ITER * 4 {
        let mut key = [0u8; 32];
        let mut ctr = [0u8; 16];
        rng.fill(&mut key);
        rng.fill(&mut ctr);
        cases.push((key, ctr));
    }

    for (key, ctr) in &cases {
        let mut ck = *key;
        let mut rk = *key;
        let mut cc = *ctr;
        let mut rc = *ctr;
        let mut cout = [0x5Au8; 16];
        let mut rout = [0x5Au8; 16];
        unsafe {
            c(ck.as_mut_ptr(), cc.as_mut_ptr(), cout.as_mut_ptr());
            r(rk.as_mut_ptr(), rc.as_mut_ptr(), rout.as_mut_ptr());
        }
        eq_bytes(
            &format!("AES256_ECB(key={}, ctr={})", hex(key), hex(ctr)),
            &cout,
            &rout,
        );
        // The C never modifies key/ctr; the Rust must not either.
        eq_bytes("AES256_ECB key untouched", &ck, key);
        eq_bytes("AES256_ECB ctr untouched", &cc, ctr);
        eq_bytes("AES256_ECB key untouched (Rust)", &rk, key);
        eq_bytes("AES256_ECB ctr untouched (Rust)", &rc, ctr);
    }

    // FIPS-197 known answer, so we know both are really AES-256-ECB.
    let mut ck = k;
    let mut cc = pt;
    let mut cout = [0u8; 16];
    unsafe { c(ck.as_mut_ptr(), cc.as_mut_ptr(), cout.as_mut_ptr()) };
    eq(
        "AES256_ECB FIPS-197 C.3 vector",
        hex(&cout),
        "8EA2B7CA516745BFEAFC49904B496089".to_string(),
    );
}

// ---------------------------------------------------------------------------
// Row 57 -- AES256_CTR_DRBG_Update, both provided_data branches
// ---------------------------------------------------------------------------
#[test]
fn row57_drbg_update() {
    let (c, r) = both!("AES256_CTR_DRBG_Update", FDrbgUpdate);
    let mut rng = Rng::new(RNG_SEED ^ 57);
    let mut keys: Vec<[u8; 32]> = vec![[0x00; 32], [0xFF; 32]];
    let mut vs: Vec<[u8; 16]> = vec![[0x00; 16], [0xFF; 16]];
    for _ in 0..N_ITER {
        let mut k = [0u8; 32];
        let mut v = [0u8; 16];
        rng.fill(&mut k);
        rng.fill(&mut v);
        keys.push(k);
        vs.push(v);
    }
    for (key, v) in keys.iter().zip(vs.iter()) {
        for pd in [None, Some([0u8; 48]), Some([0xFFu8; 48]), Some(*&{
            let mut p = [0u8; 48];
            rng.fill(&mut p);
            p
        })] {
            let mut ck = *key;
            let mut rk = *key;
            let mut cv = *v;
            let mut rv = *v;
            let mut cp = pd;
            let mut rp = pd;
            unsafe {
                let cpp = match cp.as_mut() {
                    Some(p) => p.as_mut_ptr(),
                    None => std::ptr::null_mut(),
                };
                let rpp = match rp.as_mut() {
                    Some(p) => p.as_mut_ptr(),
                    None => std::ptr::null_mut(),
                };
                c(cpp, ck.as_mut_ptr(), cv.as_mut_ptr());
                r(rpp, rk.as_mut_ptr(), rv.as_mut_ptr());
            }
            let tag = match pd {
                None => "pd=NULL".to_string(),
                Some(p) => format!("pd={}", hex(&p[..8])),
            };
            eq_bytes(&format!("DRBG_Update {tag} Key"), &ck, &rk);
            eq_bytes(&format!("DRBG_Update {tag} V"), &cv, &rv);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 58 -- seedexpander_init + seedexpander (full ctx image compared)
// ---------------------------------------------------------------------------
#[test]
fn row58_seedexpander() {
    let (ci, ri) = both!("seedexpander_init", FSeedexpanderInit);
    let (c, r) = both!("seedexpander", FSeedexpander);
    let mut rng = Rng::new(RNG_SEED ^ 58);

    for &maxlen in &[16u64, 17, 64, 4096, 0xFFFF_FFFF] {
        for iter in 0..3 {
            let (mut seed, mut div) = match iter {
                0 => ([0x00u8; 32], [0x00u8; 8]),
                1 => ([0xFFu8; 32], [0xFFu8; 8]),
                _ => {
                    let mut s = [0u8; 32];
                    let mut d = [0u8; 8];
                    rng.fill(&mut s);
                    rng.fill(&mut d);
                    (s, d)
                }
            };
            let mut cctx = vec![0x5Au8; XOF_BYTES];
            let mut rctx = vec![0x5Au8; XOF_BYTES];
            let (crc, rrc) = unsafe {
                (
                    ci(
                        cctx.as_mut_ptr(),
                        seed.as_mut_ptr(),
                        div.as_mut_ptr(),
                        maxlen,
                    ),
                    ri(
                        rctx.as_mut_ptr(),
                        seed.as_mut_ptr(),
                        div.as_mut_ptr(),
                        maxlen,
                    ),
                )
            };
            eq(&format!("seedexpander_init(maxlen={maxlen}) return"), crc, rrc);
            eq(
                &format!("seedexpander_init(maxlen={maxlen}) return == 0"),
                crc,
                0,
            );
            eq_bytes(
                &format!("seedexpander_init(maxlen={maxlen}) ctx image"),
                &cctx,
                &rctx,
            );

            // Now draw a sequence of requests, always staying strictly below
            // length_remaining (>= would be RNG_BAD_REQ_LEN, see ERRORS rows
            // 7/8/11/12).
            let mut remaining = maxlen;
            for &want in &[1u64, 15, 16, 17, 33, 7] {
                if want >= remaining {
                    break;
                }
                let mut cb = vec![0xA5u8; want as usize + 8];
                let mut rb = vec![0xA5u8; want as usize + 8];
                let (crc, rrc) = unsafe {
                    (
                        c(cctx.as_mut_ptr(), cb.as_mut_ptr(), want),
                        r(rctx.as_mut_ptr(), rb.as_mut_ptr(), want),
                    )
                };
                eq(
                    &format!("seedexpander(maxlen={maxlen}, want={want}) return"),
                    crc,
                    rrc,
                );
                eq(
                    &format!("seedexpander(maxlen={maxlen}, want={want}) return == 0"),
                    crc,
                    0,
                );
                eq_bytes(
                    &format!("seedexpander(maxlen={maxlen}, want={want}) output"),
                    &cb,
                    &rb,
                );
                eq_bytes(
                    &format!("seedexpander(maxlen={maxlen}, want={want}) ctx image"),
                    &cctx,
                    &rctx,
                );
                remaining -= want;
            }
        }
    }
}
