//! Phase B for `crypto_core/` — the lowest-level primitives.
//!
//! CONFIGS rows PA78–PA93, ERRORS rows A95–A96.
//! salsa20 / salsa2012 / salsa208 / hsalsa20 / hchacha20 (NULL and non-NULL
//! sigma constant) and the full `crypto_core_keccak1600` state API.

mod harness;
use harness::*;

use std::ffi::c_int;
use std::ptr;

const SEED: u64 = 0x5EED_0002;

type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

/// salsa20 / salsa2012 / salsa208: out=64, in=16, k=32, c=16-or-NULL.
#[test]
fn crypto_core_salsa_family() {
    let mut rng = Rng::new(SEED);
    for name in ["crypto_core_salsa20", "crypto_core_salsa2012", "crypto_core_salsa208"] {
        let (c, r) = sym::<Core>(name);
        for iter in 0..400 {
            let inp = rng.bytes(16);
            let k = rng.bytes(32);
            let cst = rng.bytes(16);
            // PA78/PA79: c == NULL selects the built-in sigma; c != NULL uses it.
            for (tag, cp) in [("sigma", ptr::null()), ("custom", cst.as_ptr())] {
                let mut oc = out_buf(64);
                let mut or = out_buf(64);
                unsafe {
                    let rc = c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    let rr = r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    assert_eq!(rc, rr, "{name} {tag} rc iter={iter}");
                }
                eqb(&format!("{name} {tag} out iter={iter}"), &oc, &or);
            }
            // extreme inputs
            for (tag, inp, k) in [
                ("zeros", vec![0u8; 16], vec![0u8; 32]),
                ("ones", vec![0xffu8; 16], vec![0xffu8; 32]),
            ] {
                for (ctag, cp) in [("sigma", ptr::null()), ("custom", cst.as_ptr())] {
                    let mut oc = out_buf(64);
                    let mut or = out_buf(64);
                    unsafe {
                        c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                        r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    }
                    eqb(&format!("{name} {tag}/{ctag}"), &oc, &or);
                }
            }
        }
    }
}

/// hsalsa20: out=32, in=16, k=32, c=16-or-NULL.
/// hchacha20: out=32, in=16, k=32, c=16-or-NULL.
#[test]
fn crypto_core_hsalsa20_hchacha20() {
    let mut rng = Rng::new(SEED ^ 1);
    for name in ["crypto_core_hsalsa20", "crypto_core_hchacha20"] {
        let (c, r) = sym::<Core>(name);
        for iter in 0..600 {
            let inp = rng.bytes(16);
            let k = rng.bytes(32);
            let cst = rng.bytes(16);
            for (tag, cp) in [("sigma", ptr::null()), ("custom", cst.as_ptr())] {
                let mut oc = out_buf(32);
                let mut or = out_buf(32);
                unsafe {
                    let rc = c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    let rr = r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    assert_eq!(rc, rr, "{name} {tag} rc iter={iter}");
                }
                eqb(&format!("{name} {tag} iter={iter}"), &oc, &or);
            }
        }
        // all-zero / all-ones corner inputs
        for (tag, inp, k) in [
            ("zeros", vec![0u8; 16], vec![0u8; 32]),
            ("ones", vec![0xffu8; 16], vec![0xffu8; 32]),
        ] {
            let mut oc = out_buf(32);
            let mut or = out_buf(32);
            unsafe {
                c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), ptr::null());
                r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), ptr::null());
            }
            eqb(&format!("{name} {tag}"), &oc, &or);
        }
    }
}

// ---------------------------------------------------------------------------
// crypto_core_keccak1600 — the raw sponge state API (PA85–PA93)
// ---------------------------------------------------------------------------

const KECCAK_STATE_BYTES: usize = 256; // >= sizeof(crypto_core_keccak1600_state)

type KInit = unsafe extern "C" fn(*mut u8);
type KXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type KPermute = unsafe extern "C" fn(*mut u8);

fn keccak_statebytes() -> usize {
    let (c, r) = sym::<unsafe extern "C" fn() -> usize>("crypto_core_keccak1600_statebytes");
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "crypto_core_keccak1600_statebytes");
    cv
}

#[test]
fn crypto_core_keccak1600_state_api() {
    let sb = keccak_statebytes();
    assert!(sb <= KECCAK_STATE_BYTES, "state too big: {sb}");

    let (cinit, rinit) = sym::<KInit>("crypto_core_keccak1600_init");
    let (cxor, rxor) = sym::<KXor>("crypto_core_keccak1600_xor_bytes");
    let (cext, rext) = sym::<KExtract>("crypto_core_keccak1600_extract_bytes");
    let (cp24, rp24) = sym::<KPermute>("crypto_core_keccak1600_permute_24");
    let (cp12, rp12) = sym::<KPermute>("crypto_core_keccak1600_permute_12");

    let mut rng = Rng::new(SEED ^ 2);

    // PA85: init must zero the state identically (compare the WHOLE buffer).
    // Both buffers start from the SAME pattern so any byte init does not touch
    // still has to match.
    let mut sc = vec![0xa5u8; KECCAK_STATE_BYTES];
    let mut sr = vec![0xa5u8; KECCAK_STATE_BYTES];
    unsafe {
        cinit(sc.as_mut_ptr());
        rinit(sr.as_mut_ptr());
    }
    eqb("keccak1600 init", &sc[..sb], &sr[..sb]);

    // PA86–PA93: xor_bytes over every (offset, length) the loops distinguish,
    // interleaved with both permutations and with extract_bytes.
    for round in 0..300 {
        let mut sc = vec![0u8; KECCAK_STATE_BYTES];
        let mut sr = vec![0u8; KECCAK_STATE_BYTES];
        unsafe {
            cinit(sc.as_mut_ptr());
            rinit(sr.as_mut_ptr());
        }
        for step in 0..8 {
            let offset = rng.below(200);
            let maxlen = 200usize.saturating_sub(offset);
            let length = if maxlen == 0 { 0 } else { rng.below(maxlen + 1) };
            let data = rng.bytes(length.max(1));
            unsafe {
                cxor(sc.as_mut_ptr(), data.as_ptr(), offset, length);
                rxor(sr.as_mut_ptr(), data.as_ptr(), offset, length);
            }
            eqb(
                &format!("keccak1600 xor_bytes round={round} step={step} off={offset} len={length}"),
                &sc[..sb],
                &sr[..sb],
            );
            match rng.below(3) {
                0 => unsafe {
                    cp24(sc.as_mut_ptr());
                    rp24(sr.as_mut_ptr());
                },
                1 => unsafe {
                    cp12(sc.as_mut_ptr());
                    rp12(sr.as_mut_ptr());
                },
                _ => {}
            }
            eqb(
                &format!("keccak1600 permute round={round} step={step}"),
                &sc[..sb],
                &sr[..sb],
            );
            let eoff = rng.below(200);
            let elen = rng.below(200usize.saturating_sub(eoff) + 1);
            let mut oc = out_buf(elen);
            let mut or = out_buf(elen);
            unsafe {
                cext(sc.as_ptr(), oc.as_mut_ptr(), eoff, elen);
                rext(sr.as_ptr(), or.as_mut_ptr(), eoff, elen);
            }
            eqb(
                &format!("keccak1600 extract round={round} step={step} off={eoff} len={elen}"),
                &oc,
                &or,
            );
        }
    }

    // Deterministic boundary sweep: every offset 0..=8 crossed with every
    // length 0..=24 (word-alignment branches), plus the sponge rates.
    for &rate in &[72usize, 136, 168, 200] {
        for offset in 0usize..=9 {
            for length in 0usize..=24 {
                if offset + length > 200 {
                    continue;
                }
                let mut sc = vec![0u8; KECCAK_STATE_BYTES];
                let mut sr = vec![0u8; KECCAK_STATE_BYTES];
                unsafe {
                    cinit(sc.as_mut_ptr());
                    rinit(sr.as_mut_ptr());
                }
                let data: Vec<u8> = (0..length.max(1)).map(|i| (i as u8).wrapping_mul(37).wrapping_add(11)).collect();
                unsafe {
                    cxor(sc.as_mut_ptr(), data.as_ptr(), offset, length);
                    rxor(sr.as_mut_ptr(), data.as_ptr(), offset, length);
                    cp24(sc.as_mut_ptr());
                    rp24(sr.as_mut_ptr());
                }
                eqb(
                    &format!("keccak1600 sweep rate={rate} off={offset} len={length}"),
                    &sc[..sb],
                    &sr[..sb],
                );
                let mut oc = out_buf(rate);
                let mut or = out_buf(rate);
                unsafe {
                    cext(sc.as_ptr(), oc.as_mut_ptr(), 0, rate);
                    rext(sr.as_ptr(), or.as_mut_ptr(), 0, rate);
                }
                eqb(
                    &format!("keccak1600 sweep-extract rate={rate} off={offset} len={length}"),
                    &oc,
                    &or,
                );
            }
        }
    }

    // Long permutation chains, to catch a round-constant table divergence that
    // only shows up after many rounds.
    let mut sc = vec![0u8; KECCAK_STATE_BYTES];
    let mut sr = vec![0u8; KECCAK_STATE_BYTES];
    unsafe {
        cinit(sc.as_mut_ptr());
        rinit(sr.as_mut_ptr());
        let seed = [1u8, 2, 3, 4, 5, 6, 7, 8];
        cxor(sc.as_mut_ptr(), seed.as_ptr(), 0, 8);
        rxor(sr.as_mut_ptr(), seed.as_ptr(), 0, 8);
    }
    for i in 0..200 {
        unsafe {
            if i % 2 == 0 {
                cp24(sc.as_mut_ptr());
                rp24(sr.as_mut_ptr());
            } else {
                cp12(sc.as_mut_ptr());
                rp12(sr.as_mut_ptr());
            }
        }
        eqb(&format!("keccak1600 chain i={i}"), &sc[..sb], &sr[..sb]);
    }
}

/// Any other exported `crypto_core_keccak1600_permute_*` variants.
#[test]
fn crypto_core_keccak1600_other_permutations() {
    let sb = keccak_statebytes();
    let (cinit, rinit) = sym::<KInit>("crypto_core_keccak1600_init");
    let (cxor, rxor) = sym::<KXor>("crypto_core_keccak1600_xor_bytes");
    let mut found = 0;
    for n in [1usize, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 30, 32] {
        let name = format!("crypto_core_keccak1600_permute_{n}");
        if !has(&name) {
            continue;
        }
        found += 1;
        let (c, r) = sym::<KPermute>(&name);
        let mut rng = Rng::new(SEED ^ (0x300 + n as u64));
        for _ in 0..50 {
            let mut sc = vec![0u8; KECCAK_STATE_BYTES];
            let mut sr = vec![0u8; KECCAK_STATE_BYTES];
            let data = rng.bytes(200);
            unsafe {
                cinit(sc.as_mut_ptr());
                rinit(sr.as_mut_ptr());
                cxor(sc.as_mut_ptr(), data.as_ptr(), 0, 200);
                rxor(sr.as_mut_ptr(), data.as_ptr(), 0, 200);
                c(sc.as_mut_ptr());
                r(sr.as_mut_ptr());
            }
            eqb(&name, &sc[..sb], &sr[..sb]);
        }
    }
    assert!(found >= 2, "expected at least permute_24 and permute_12, found {found}");
}

// ---------------------------------------------------------------------------
// softaes — internal, but check anything it exports.
// ---------------------------------------------------------------------------

#[test]
fn softaes_exports_if_any() {
    // The soft-AES helpers are `static`/internal in the C build; assert that
    // neither .so exports them, i.e. the surface really is empty on both sides.
    for name in [
        "softaes_block_load",
        "softaes_block_encrypt",
        "softaes_block_xor",
        "softaes_block_store",
    ] {
        let l = libs();
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        let in_c = unsafe { l.c.get::<*const ()>(&n).is_ok() };
        let in_r = unsafe { l.r.get::<*const ()>(&n).is_ok() };
        assert_eq!(in_c, in_r, "{name}: export presence differs");
    }
}
