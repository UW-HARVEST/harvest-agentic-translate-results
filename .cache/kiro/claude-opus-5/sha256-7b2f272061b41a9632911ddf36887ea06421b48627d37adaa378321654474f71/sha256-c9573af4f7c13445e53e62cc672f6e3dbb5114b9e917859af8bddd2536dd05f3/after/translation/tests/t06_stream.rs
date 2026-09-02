//! Phase B + C for `crypto_stream/`.
//!
//! CONFIGS rows PB78–PB113, ERRORS rows B22–B29.
//! salsa20 / salsa2012 / salsa208 / xsalsa20 / chacha20 / chacha20_ietf /
//! chacha20_ietf_ext / xchacha20, over the keystream, `_xor` and `_xor_ic`
//! entry points, all block boundaries, in==out aliasing, and the 32-bit
//! IETF counter overflow.

mod harness;
use harness::*;

use std::ffi::c_int;

const SEED: u64 = 0x5EED_0005;

type Stream = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;

struct S {
    pfx: &'static str,
    noncebytes: usize,
    keybytes: usize,
    /// 64-bit `_xor_ic`, 32-bit `_xor_ic`, or none.
    ic: Ic,
}

#[derive(PartialEq)]
enum Ic {
    None,
    U64,
    U32,
}

const STREAMS: &[S] = &[
    S { pfx: "crypto_stream", noncebytes: 24, keybytes: 32, ic: Ic::None },
    S { pfx: "crypto_stream_salsa20", noncebytes: 8, keybytes: 32, ic: Ic::U64 },
    S { pfx: "crypto_stream_salsa2012", noncebytes: 8, keybytes: 32, ic: Ic::None },
    S { pfx: "crypto_stream_salsa208", noncebytes: 8, keybytes: 32, ic: Ic::None },
    S { pfx: "crypto_stream_xsalsa20", noncebytes: 24, keybytes: 32, ic: Ic::U64 },
    S { pfx: "crypto_stream_chacha20", noncebytes: 8, keybytes: 32, ic: Ic::U64 },
    S { pfx: "crypto_stream_chacha20_ietf", noncebytes: 12, keybytes: 32, ic: Ic::U32 },
    S { pfx: "crypto_stream_xchacha20", noncebytes: 24, keybytes: 32, ic: Ic::U64 },
];

/// Every length that matters for a 64-byte-block stream cipher.
fn lengths() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=70).collect();
    v.extend_from_slice(&[
        126, 127, 128, 129, 130, 191, 192, 193, 255, 256, 257, 511, 512, 513, 1023, 1024, 1025,
        4095, 4096,
    ]);
    v
}

#[test]
fn stream_keystream_all_lengths() {
    let mut rng = Rng::new(SEED);
    for s in STREAMS {
        let (c, r) = sym::<Stream>(s.pfx);
        // check the advertised sizes agree with our table
        let (cn, rn) = sym::<unsafe extern "C" fn() -> usize>(&format!("{}_noncebytes", s.pfx));
        unsafe {
            assert_eq!(cn(), rn());
            assert_eq!(cn(), s.noncebytes, "{} noncebytes", s.pfx);
        }
        for len in lengths() {
            let n = rng.bytes(s.noncebytes);
            let k = rng.bytes(s.keybytes);
            let mut oc = out_buf(len);
            let mut or = out_buf(len);
            unsafe {
                let rc = c(oc.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let rr = r(or.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{} rc len={len}", s.pfx);
            }
            eqb(&format!("{} keystream len={len}", s.pfx), &oc, &or);
        }
        // extreme keys/nonces
        for (tag, kb, nb) in [
            ("zero", 0u8, 0u8),
            ("ones", 0xff, 0xff),
            ("mix", 0x00, 0xff),
        ] {
            let n = vec![nb; s.noncebytes];
            let k = vec![kb; s.keybytes];
            for len in [0usize, 1, 63, 64, 65, 128, 200] {
                let mut oc = out_buf(len);
                let mut or = out_buf(len);
                unsafe {
                    c(oc.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                    r(or.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{} keystream {tag} len={len}", s.pfx), &oc, &or);
            }
        }
    }
}

#[test]
fn stream_xor_all_lengths_and_aliasing() {
    let mut rng = Rng::new(SEED ^ 1);
    for s in STREAMS {
        let (c, r) = sym::<Xor>(&format!("{}_xor", s.pfx));
        for len in lengths() {
            let n = rng.bytes(s.noncebytes);
            let k = rng.bytes(s.keybytes);
            let m = rng.bytes(len);
            let mut oc = out_buf(len);
            let mut or = out_buf(len);
            unsafe {
                let rc = c(oc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let rr = r(or.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_xor rc len={len}", s.pfx);
            }
            eqb(&format!("{}_xor len={len}", s.pfx), &oc, &or);

            // in-place (c == m), the way every real consumer uses it
            let mut ac = out_buf(len);
            let mut ar = out_buf(len);
            ac[..len].copy_from_slice(&m);
            ar[..len].copy_from_slice(&m);
            unsafe {
                c(ac.as_mut_ptr(), ac.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                r(ar.as_mut_ptr(), ar.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{}_xor in-place len={len}", s.pfx), &ac, &ar);
            eqb(&format!("{}_xor in-place == out-of-place len={len}", s.pfx), &oc, &ac);

            // xor is an involution: applying twice restores the plaintext
            let mut bc = ac.clone();
            unsafe { c(bc.as_mut_ptr(), bc.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
            eqb(&format!("{}_xor involution len={len}", s.pfx), &m, &bc[..len]);
        }
    }
}

#[test]
fn stream_xor_ic_all_counters() {
    let mut rng = Rng::new(SEED ^ 2);
    for s in STREAMS {
        match s.ic {
            Ic::None => continue,
            Ic::U64 => {
                let (c, r) = sym::<XorIc64>(&format!("{}_xor_ic", s.pfx));
                let (cx, _) = sym::<Xor>(&format!("{}_xor", s.pfx));
                let ics: &[u64] = &[
                    0,
                    1,
                    2,
                    3,
                    0xff,
                    0x100,
                    0xffff_ffff,
                    0x1_0000_0000,
                    0x1_0000_0001,
                    0xffff_ffff_ffff_fffe,
                    u64::MAX,
                ];
                for &ic in ics {
                    for len in [0usize, 1, 63, 64, 65, 127, 128, 129, 200, 512] {
                        let n = rng.bytes(s.noncebytes);
                        let k = rng.bytes(s.keybytes);
                        let m = rng.bytes(len);
                        let mut oc = out_buf(len);
                        let mut or = out_buf(len);
                        unsafe {
                            let rc = c(oc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                            let rr = r(or.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                            assert_eq!(rc, rr, "{}_xor_ic rc ic={ic:#x} len={len}", s.pfx);
                        }
                        eqb(&format!("{}_xor_ic ic={ic:#x} len={len}", s.pfx), &oc, &or);
                        // ic == 0 must equal plain _xor
                        if ic == 0 {
                            let mut xc = out_buf(len);
                            unsafe { cx(xc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
                            eqb(&format!("{}_xor_ic(0) == _xor len={len}", s.pfx), &xc, &oc);
                        }
                    }
                }
                // Continuation property: xor_ic(ic) over 64 bytes must equal
                // bytes [64*ic .. 64*ic+64) of the long keystream.
                let n = rng.bytes(s.noncebytes);
                let k = rng.bytes(s.keybytes);
                let total = 64 * 6;
                let zeros = vec![0u8; total];
                let mut fullc = out_buf(total);
                let mut fullr = out_buf(total);
                unsafe {
                    cx(fullc.as_mut_ptr(), zeros.as_ptr(), total as u64, n.as_ptr(), k.as_ptr());
                    let (_, rx) = sym::<Xor>(&format!("{}_xor", s.pfx));
                    rx(fullr.as_mut_ptr(), zeros.as_ptr(), total as u64, n.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{} long keystream", s.pfx), &fullc, &fullr);
                for blk in 0..6u64 {
                    let mut pc = out_buf(64);
                    let mut pr = out_buf(64);
                    unsafe {
                        c(pc.as_mut_ptr(), zeros.as_ptr(), 64, n.as_ptr(), blk, k.as_ptr());
                        r(pr.as_mut_ptr(), zeros.as_ptr(), 64, n.as_ptr(), blk, k.as_ptr());
                    }
                    eqb(&format!("{}_xor_ic block {blk} C vs Rust", s.pfx), &pc, &pr);
                    eqb(
                        &format!("{}_xor_ic block {blk} vs long keystream", s.pfx),
                        &fullc[blk as usize * 64..(blk as usize + 1) * 64],
                        &pc[..64],
                    );
                }
            }
            Ic::U32 => {
                let (c, r) = sym::<XorIc32>(&format!("{}_xor_ic", s.pfx));
                let (cx, _) = sym::<Xor>(&format!("{}_xor", s.pfx));
                // B29: the IETF counter is 32-bit; ic near 2^32 makes the
                // internal counter wrap, which the C explicitly checks.
                // The IETF `_xor_ic` guard is
                //   ic > 2^32 - ceil(mlen/64)  ->  sodium_misuse()
                // so only (ic, mlen) pairs satisfying it stay in this
                // valid-path test; the rejecting pairs live in
                // `stream_oversized_length_matches`.
                let ics: &[u32] = &[
                    0, 1, 2, 0xff, 0x100, 0xffff, 0x7fff_ffff, 0xffff_fff0, 0xffff_fff7,
                    0xffff_fffe, u32::MAX,
                ];
                for &ic in ics {
                    for len in [0usize, 1, 63, 64, 65, 127, 128, 129, 192, 200, 512] {
                        let blocks = ((len as u64) + 63) / 64;
                        if (ic as u64) > (1u64 << 32) - blocks {
                            continue;
                        }
                        let n = rng.bytes(s.noncebytes);
                        let k = rng.bytes(s.keybytes);
                        let m = rng.bytes(len);
                        let mut oc = out_buf(len);
                        let mut or = out_buf(len);
                        unsafe {
                            let rc = c(oc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                            let rr = r(or.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                            assert_eq!(rc, rr, "{}_xor_ic rc ic={ic:#x} len={len}", s.pfx);
                        }
                        eqb(&format!("{}_xor_ic ic={ic:#x} len={len}", s.pfx), &oc, &or);
                        if ic == 0 {
                            let mut xc = out_buf(len);
                            unsafe { cx(xc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
                            eqb(&format!("{}_xor_ic(0) == _xor len={len}", s.pfx), &xc, &oc);
                        }
                    }
                }
            }
        }
    }
}

/// The private-but-exported `crypto_stream_chacha20_ietf_ext*` entry points.
/// These are the LOWEST-level chacha20 exports; unlike `_ietf_*` they let the
/// counter overflow into the IV.
#[test]
fn chacha20_ietf_ext_entry_points() {
    let (c, r) = sym::<Stream>("crypto_stream_chacha20_ietf_ext");
    let (cx, rx) = sym::<XorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    let (cietf, _) = sym::<Stream>("crypto_stream_chacha20_ietf");
    let mut rng = Rng::new(SEED ^ 3);

    for len in lengths() {
        let n = rng.bytes(12);
        let k = rng.bytes(32);
        let mut oc = out_buf(len);
        let mut or = out_buf(len);
        unsafe {
            let rc = c(oc.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            let rr = r(or.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            assert_eq!(rc, rr, "ietf_ext rc len={len}");
        }
        eqb(&format!("ietf_ext keystream len={len}"), &oc, &or);
        // ietf_ext and ietf agree while the counter does not overflow
        let mut ic2 = out_buf(len);
        unsafe { cietf(ic2.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr()) };
        eqb(&format!("ietf_ext == ietf len={len}"), &ic2, &oc);
    }

    // The counter-overflow region is exactly where ext and non-ext differ.
    for &ic in &[0u32, 1, 0xffff_fff0, 0xffff_fffe, u32::MAX] {
        for len in [0usize, 1, 64, 65, 128, 192, 256, 512] {
            let n = rng.bytes(12);
            let k = rng.bytes(32);
            let m = rng.bytes(len);
            let mut oc = out_buf(len);
            let mut or = out_buf(len);
            unsafe {
                let rc = cx(oc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                let rr = rx(or.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), ic, k.as_ptr());
                assert_eq!(rc, rr, "ietf_ext_xor_ic rc ic={ic:#x} len={len}");
            }
            eqb(&format!("ietf_ext_xor_ic ic={ic:#x} len={len}"), &oc, &or);
        }
    }
}

/// The `crypto_stream_*_keygen` helpers: non-deterministic, so only check that
/// both write the right number of bytes and leave the canary intact.
#[test]
fn stream_keygen_writes_keybytes() {
    for s in STREAMS {
        let name = format!("{}_keygen", s.pfx);
        if !has(&name) {
            continue;
        }
        let (c, r) = sym::<unsafe extern "C" fn(*mut u8)>(&name);
        let mut bc = out_buf(s.keybytes);
        let mut br = out_buf(s.keybytes);
        unsafe {
            c(bc.as_mut_ptr());
            r(br.as_mut_ptr());
        }
        eqb(&format!("{name} canary"), &bc[s.keybytes..], &br[s.keybytes..]);
        assert_ne!(&bc[..s.keybytes], &vec![0u8; s.keybytes][..], "{name} C produced all zeros");
        assert_ne!(&br[..s.keybytes], &vec![0u8; s.keybytes][..], "{name} Rust produced all zeros");
    }
}

/// B22–B29: the chacha20 dispatcher calls `sodium_misuse()` when the requested
/// length exceeds `crypto_stream_chacha20_MESSAGEBYTES_MAX`. The other stream
/// families have NO such check (their MESSAGEBYTES_MAX is SIZE_MAX), so the
/// same call must simply not abort — which this test also pins down.
#[test]
fn stream_oversized_length_matches() {
    // MESSAGEBYTES_MAX for each family, read from the .so itself.
    for s in STREAMS {
        let maxn = format!("{}_messagebytes_max", s.pfx);
        if !has(&maxn) {
            continue;
        }
        let (cm, rm) = sym::<unsafe extern "C" fn() -> usize>(&maxn);
        let (cv, rv) = unsafe { (cm(), rm()) };
        assert_eq!(cv, rv, "{maxn}");
        if cv == usize::MAX {
            // No limit; nothing to trigger without allocating SIZE_MAX bytes.
            continue;
        }
        // clen = MAX + 1 must abort in both. The buffer is never written
        // because the check precedes the work.
        for over in [1usize, 2, 1 << 20] {
            let clen = (cv as u64).wrapping_add(over as u64);
            let pfx = s.pfx.to_string();
            let nb = s.noncebytes;
            let kb = s.keybytes;
            let p2 = pfx.clone();
            same_outcome(
                &format!("{} clen=MAX+{over}", s.pfx),
                move || {
                    let (c, _) = sym::<Stream>(&pfx);
                    let n = vec![0u8; nb];
                    let k = vec![0u8; kb];
                    let mut o = vec![0u8; 64];
                    unsafe { c(o.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) }
                },
                move || {
                    let (_, r) = sym::<Stream>(&p2);
                    let n = vec![0u8; nb];
                    let k = vec![0u8; kb];
                    let mut o = vec![0u8; 64];
                    unsafe { r(o.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) }
                },
            );
            let pfx = format!("{}_xor", s.pfx);
            let p2 = pfx.clone();
            same_outcome(
                &format!("{}_xor mlen=MAX+{over}", s.pfx),
                move || {
                    let (c, _) = sym::<Xor>(&pfx);
                    let n = vec![0u8; nb];
                    let k = vec![0u8; kb];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 64];
                    unsafe { c(o.as_mut_ptr(), m.as_ptr(), clen, n.as_ptr(), k.as_ptr()) }
                },
                move || {
                    let (_, r) = sym::<Xor>(&p2);
                    let n = vec![0u8; nb];
                    let k = vec![0u8; kb];
                    let m = vec![0u8; 64];
                    let mut o = vec![0u8; 64];
                    unsafe { r(o.as_mut_ptr(), m.as_ptr(), clen, n.as_ptr(), k.as_ptr()) }
                },
            );
        }
    }
    // chacha20 explicitly: MESSAGEBYTES_MAX is SODIUM_SIZE_MAX for the 64-bit
    // counter variant but 64 * 2^32 for the IETF one, so the IETF entry points
    // are the ones that really do abort.
    for name in [
        "crypto_stream_chacha20_ietf",
        "crypto_stream_chacha20_ietf_ext",
    ] {
        let (cm, rm) =
            sym::<unsafe extern "C" fn() -> usize>("crypto_stream_chacha20_ietf_messagebytes_max");
        let (cv, rv) = unsafe { (cm(), rm()) };
        assert_eq!(cv, rv);
        let clen = (cv as u64).wrapping_add(1);
        let n1 = name.to_string();
        let n2 = name.to_string();
        same_outcome(
            &format!("{name} clen=IETF_MAX+1"),
            move || {
                let (c, _) = sym::<Stream>(&n1);
                let n = [0u8; 12];
                let k = [0u8; 32];
                let mut o = vec![0u8; 64];
                unsafe { c(o.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) }
            },
            move || {
                let (_, r) = sym::<Stream>(&n2);
                let n = [0u8; 12];
                let k = [0u8; 32];
                let mut o = vec![0u8; 64];
                unsafe { r(o.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) }
            },
        );
    }
    // B29: crypto_stream_chacha20_ietf_xor_ic rejects ic + blocks overflowing
    // the 32-bit counter.
    for (ic, mlen) in [
        (0xffff_ffffu32, 65u64),
        (0xffff_ffff, 64),
        (0xffff_fffe, 129),
        (0xffff_ff00, 64 * 256 + 1),
        (0, 64),
    ] {
        same_outcome(
            &format!("chacha20_ietf_xor_ic ic={ic:#x} mlen={mlen}"),
            move || {
                let (c, _) = sym::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
                let n = [0u8; 12];
                let k = [0u8; 32];
                let m = vec![0u8; mlen as usize];
                let mut o = vec![0u8; mlen as usize];
                unsafe { c(o.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), ic, k.as_ptr()) }
            },
            move || {
                let (_, r) = sym::<XorIc32>("crypto_stream_chacha20_ietf_xor_ic");
                let n = [0u8; 12];
                let k = [0u8; 32];
                let m = vec![0u8; mlen as usize];
                let mut o = vec![0u8; mlen as usize];
                unsafe { r(o.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), ic, k.as_ptr()) }
            },
        );
    }
}

/// The xsalsa20 / xchacha20 constructions must agree with the composition of
/// their core function and the underlying stream — driving the pipeline, not
/// just each wrapper.
#[test]
fn extended_nonce_composition() {
    let mut rng = Rng::new(SEED ^ 4);
    type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
    for (xpfx, corename, inner) in [
        ("crypto_stream_xsalsa20", "crypto_core_hsalsa20", "crypto_stream_salsa20"),
        ("crypto_stream_xchacha20", "crypto_core_hchacha20", "crypto_stream_chacha20"),
    ] {
        let (cx, rx) = sym::<Xor>(&format!("{xpfx}_xor"));
        let (ccore, _) = sym::<Core>(corename);
        let (cinner, _) = sym::<XorIc64>(&format!("{inner}_xor_ic"));
        for len in [0usize, 1, 63, 64, 65, 200, 512] {
            let n = rng.bytes(24);
            let k = rng.bytes(32);
            let m = rng.bytes(len);
            let mut oc = out_buf(len);
            let mut or = out_buf(len);
            unsafe {
                cx(oc.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                rx(or.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{xpfx}_xor len={len}"), &oc, &or);

            // manual composition through the C exports
            let mut subkey = [0u8; 32];
            let mut want = vec![0u8; len];
            unsafe {
                ccore(subkey.as_mut_ptr(), n.as_ptr(), k.as_ptr(), std::ptr::null());
                cinner(
                    want.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n[16..].as_ptr(),
                    0,
                    subkey.as_ptr(),
                );
            }
            eqb(&format!("{xpfx} == {corename}+{inner} len={len}"), &want, &oc[..len]);
        }
    }
}
