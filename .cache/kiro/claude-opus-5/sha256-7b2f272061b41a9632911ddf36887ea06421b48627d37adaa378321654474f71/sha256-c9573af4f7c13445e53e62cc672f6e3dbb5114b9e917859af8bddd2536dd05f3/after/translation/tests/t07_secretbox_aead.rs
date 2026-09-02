//! Phase B + C for `crypto_secretbox/`, `crypto_secretstream/` and
//! `crypto_aead/`.
//!
//! CONFIGS rows PB114–PB209, ERRORS rows B30–B79.
//!
//! Every entry point is driven, including the raw ZEROBYTES/BOXZEROBYTES
//! `crypto_secretbox_xsalsa20poly1305` API, the `_detached` variants, the
//! verify-only `m == NULL` decrypt path, and the whole `aes256gcm` ENOSYS stub.

mod harness;
use harness::*;

use std::ffi::c_int;
use std::ptr;

const SEED: u64 = 0x5EED_0007;
const STATE_MAX: usize = 256;

unsafe fn errno() -> c_int {
    *libc::__errno_location()
}
unsafe fn set_errno(v: c_int) {
    *libc::__errno_location() = v;
}

// ---------------------------------------------------------------------------
// secretbox (PB114–PB140, B30–B38)
// ---------------------------------------------------------------------------

type Box5 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type BoxDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type BoxOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;

struct SB {
    pfx: &'static str,
    noncebytes: usize,
    macbytes: usize,
}

const SECRETBOXES: &[SB] = &[
    SB { pfx: "crypto_secretbox", noncebytes: 24, macbytes: 16 },
    SB { pfx: "crypto_secretbox_xchacha20poly1305", noncebytes: 24, macbytes: 16 },
];

fn mlens() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=40).collect();
    v.extend_from_slice(&[63, 64, 65, 127, 128, 129, 255, 256, 257, 1000]);
    v
}

#[test]
fn secretbox_easy_roundtrip_and_tamper() {
    let mut rng = Rng::new(SEED);
    for s in SECRETBOXES {
        let (ce, re) = sym::<Box5>(&format!("{}_easy", s.pfx));
        let (co, ro) = sym::<Box5>(&format!("{}_open_easy", s.pfx));
        for mlen in mlens() {
            let n = rng.bytes(s.noncebytes);
            let k = rng.bytes(32);
            let m = rng.bytes(mlen);
            let clen = mlen + s.macbytes;
            let mut cc = out_buf(clen);
            let mut cr = out_buf(clen);
            unsafe {
                let rc = ce(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = re(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_easy rc mlen={mlen}", s.pfx);
            }
            eqb(&format!("{}_easy mlen={mlen}", s.pfx), &cc, &cr);

            // open with the correct ciphertext
            let mut pc = out_buf(mlen);
            let mut pr = out_buf(mlen);
            unsafe {
                let rc = co(pc.as_mut_ptr(), cc.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
                let rr = ro(pr.as_mut_ptr(), cr.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_open_easy rc mlen={mlen}", s.pfx);
                assert_eq!(rc, 0, "{}_open_easy should succeed mlen={mlen}", s.pfx);
            }
            eqb(&format!("{}_open_easy mlen={mlen}", s.pfx), &pc, &pr);
            eqb(&format!("{} roundtrip mlen={mlen}", s.pfx), &m, &pc[..mlen]);

            // B32/B36: every truncated ciphertext length, incl. clen < MACBYTES
            for shortlen in 0..=clen.min(s.macbytes + 2) {
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = co(pc.as_mut_ptr(), cc.as_ptr(), shortlen as u64, n.as_ptr(), k.as_ptr());
                    let rr = ro(pr.as_mut_ptr(), cr.as_ptr(), shortlen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_open_easy short clen={shortlen} mlen={mlen}", s.pfx);
                }
                eqb(&format!("{}_open_easy short clen={shortlen}", s.pfx), &pc, &pr);
            }
            // tamper: flip a bit in the MAC, then in the body
            for pos in [0usize, s.macbytes - 1, s.macbytes, clen - 1] {
                if pos >= clen {
                    continue;
                }
                let mut bad_c = cc[..clen].to_vec();
                let mut bad_r = cr[..clen].to_vec();
                bad_c[pos] ^= 0x01;
                bad_r[pos] ^= 0x01;
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = co(pc.as_mut_ptr(), bad_c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
                    let rr = ro(pr.as_mut_ptr(), bad_r.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_open_easy tamper@{pos} mlen={mlen}", s.pfx);
                    assert_eq!(rc, -1, "{}_open_easy tamper@{pos} must fail", s.pfx);
                }
                eqb(&format!("{}_open_easy tamper@{pos} mlen={mlen}", s.pfx), &pc, &pr);
            }
            // wrong key / wrong nonce
            let k2 = rng.bytes(32);
            let n2 = rng.bytes(s.noncebytes);
            for (tag, np, kp) in [("wrong-key", n.as_ptr(), k2.as_ptr()), ("wrong-nonce", n2.as_ptr(), k.as_ptr())] {
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = co(pc.as_mut_ptr(), cc.as_ptr(), clen as u64, np, kp);
                    let rr = ro(pr.as_mut_ptr(), cr.as_ptr(), clen as u64, np, kp);
                    assert_eq!(rc, rr, "{}_open_easy {tag} mlen={mlen}", s.pfx);
                }
                eqb(&format!("{}_open_easy {tag} mlen={mlen}", s.pfx), &pc, &pr);
            }
            // in-place easy (c == m is not allowed for _easy since c is longer,
            // but the documented in-place form c = m - MACBYTES is)
            let mut buf_c = out_buf(clen);
            let mut buf_r = out_buf(clen);
            buf_c[s.macbytes..clen].copy_from_slice(&m);
            buf_r[s.macbytes..clen].copy_from_slice(&m);
            unsafe {
                ce(buf_c.as_mut_ptr(), buf_c[s.macbytes..].as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                re(buf_r.as_mut_ptr(), buf_r[s.macbytes..].as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{}_easy in-place mlen={mlen}", s.pfx), &buf_c, &buf_r);
        }
    }
}

#[test]
fn secretbox_detached_all_shapes() {
    let mut rng = Rng::new(SEED ^ 1);
    for s in SECRETBOXES {
        let (cd, rd) = sym::<BoxDet>(&format!("{}_detached", s.pfx));
        let (co, ro) = sym::<BoxOpenDet>(&format!("{}_open_detached", s.pfx));
        for mlen in mlens() {
            let n = rng.bytes(s.noncebytes);
            let k = rng.bytes(32);
            let m = rng.bytes(mlen);
            let mut cc = out_buf(mlen);
            let mut cr = out_buf(mlen);
            let mut mc = out_buf(s.macbytes);
            let mut mr = out_buf(s.macbytes);
            unsafe {
                let rc = cd(cc.as_mut_ptr(), mc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = rd(cr.as_mut_ptr(), mr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_detached rc mlen={mlen}", s.pfx);
            }
            eqb(&format!("{}_detached c mlen={mlen}", s.pfx), &cc, &cr);
            eqb(&format!("{}_detached mac mlen={mlen}", s.pfx), &mc, &mr);

            let mut pc = out_buf(mlen);
            let mut pr = out_buf(mlen);
            unsafe {
                let rc = co(pc.as_mut_ptr(), cc.as_ptr(), mc.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = ro(pr.as_mut_ptr(), cr.as_ptr(), mr.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_open_detached rc mlen={mlen}", s.pfx);
                assert_eq!(rc, 0);
            }
            eqb(&format!("{}_open_detached mlen={mlen}", s.pfx), &pc, &pr);
            eqb(&format!("{} detached roundtrip mlen={mlen}", s.pfx), &m, &pc[..mlen]);

            // verify-only path: m == NULL
            unsafe {
                let rc = co(ptr::null_mut(), cc.as_ptr(), mc.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = ro(ptr::null_mut(), cr.as_ptr(), mr.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_open_detached m=NULL rc mlen={mlen}", s.pfx);
            }
            // bad mac
            let mut badmac = mc[..s.macbytes].to_vec();
            badmac[rng.below(s.macbytes)] ^= 0x80;
            let mut pc = out_buf(mlen.max(1));
            let mut pr = out_buf(mlen.max(1));
            unsafe {
                let rc = co(pc.as_mut_ptr(), cc.as_ptr(), badmac.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = ro(pr.as_mut_ptr(), cr.as_ptr(), badmac.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{}_open_detached bad mac mlen={mlen}", s.pfx);
                assert_eq!(rc, -1);
            }
            eqb(&format!("{}_open_detached bad mac mlen={mlen}", s.pfx), &pc, &pr);
        }
    }
}

/// PB114–PB127: the RAW `crypto_secretbox_xsalsa20poly1305` API with its
/// ZEROBYTES=32 / BOXZEROBYTES=16 padding contract, plus `crypto_secretbox`
/// and `crypto_secretbox_open` which are aliases of it.
#[test]
fn secretbox_raw_zerobytes_api() {
    let mut rng = Rng::new(SEED ^ 2);
    let zb = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>("crypto_secretbox_zerobytes");
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        a
    };
    let bzb = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>("crypto_secretbox_boxzerobytes");
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        a
    };
    assert_eq!((zb, bzb), (32, 16));

    for name in [
        "crypto_secretbox",
        "crypto_secretbox_xsalsa20poly1305",
    ] {
        let (cs, rs) = sym::<Box5>(name);
        let (cso, rso) = sym::<Box5>(&format!("{name}_open"));
        // mlen here is the PADDED length. Every value 0..=64 exercises the
        // `mlen < 32` rejection and the first-block split at 32.
        for mlen in 0usize..=80 {
            let n = rng.bytes(24);
            let k = rng.bytes(32);
            let mut m = rng.bytes(mlen);
            if mlen >= zb {
                for b in m[..zb].iter_mut() {
                    *b = 0;
                }
            }
            let mut cc = out_buf(mlen.max(1));
            let mut cr = out_buf(mlen.max(1));
            unsafe {
                let rc = cs(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = rs(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{name} rc mlen={mlen}");
            }
            eqb(&format!("{name} mlen={mlen}"), &cc, &cr);

            let mut pc = out_buf(mlen.max(1));
            let mut pr = out_buf(mlen.max(1));
            unsafe {
                let rc = cso(pc.as_mut_ptr(), cc.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                let rr = rso(pr.as_mut_ptr(), cr.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                assert_eq!(rc, rr, "{name}_open rc mlen={mlen}");
            }
            eqb(&format!("{name}_open mlen={mlen}"), &pc, &pr);
            if mlen >= zb {
                eqb(&format!("{name} raw roundtrip mlen={mlen}"), &m, &pc[..mlen]);
            }
            // open a corrupted box
            if mlen > bzb {
                let mut badc = cc[..mlen].to_vec();
                let mut badr = cr[..mlen].to_vec();
                badc[bzb] ^= 1;
                badr[bzb] ^= 1;
                let mut pc = out_buf(mlen);
                let mut pr = out_buf(mlen);
                unsafe {
                    let rc = cso(pc.as_mut_ptr(), badc.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    let rr = rso(pr.as_mut_ptr(), badr.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{name}_open corrupted mlen={mlen}");
                }
                eqb(&format!("{name}_open corrupted mlen={mlen}"), &pc, &pr);
            }
            // non-zero padding in the first ZEROBYTES: the C does not check it,
            // it just encrypts; make sure both behave the same.
            if mlen >= zb {
                m[0] = 0xff;
                let mut cc = out_buf(mlen);
                let mut cr = out_buf(mlen);
                unsafe {
                    cs(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    rs(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{name} dirty-pad mlen={mlen}"), &cc, &cr);
            }
        }
    }
}

/// B30/B33/B37: the `_easy` wrappers call `sodium_misuse()` when
/// `mlen > MESSAGEBYTES_MAX`.
#[test]
fn secretbox_oversized_aborts_identically() {
    for s in SECRETBOXES {
        let maxn = format!("{}_messagebytes_max", s.pfx);
        let (cm, rm) = sym::<unsafe extern "C" fn() -> usize>(&maxn);
        let (cv, rv) = unsafe { (cm(), rm()) };
        assert_eq!(cv, rv, "{maxn}");
        if cv == usize::MAX {
            continue;
        }
        let mlen = (cv as u64).wrapping_add(1);
        let nb = s.noncebytes;
        for suffix in ["_easy", "_detached"] {
            let a = format!("{}{suffix}", s.pfx);
            let b = a.clone();
            if suffix == "_easy" {
                same_outcome(
                    &format!("{a} mlen=MAX+1"),
                    move || {
                        let (c, _) = sym::<Box5>(&a);
                        let n = vec![0u8; nb];
                        let k = vec![0u8; 32];
                        let m = vec![0u8; 64];
                        let mut o = vec![0u8; 128];
                        unsafe { c(o.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), k.as_ptr()) }
                    },
                    move || {
                        let (_, r) = sym::<Box5>(&b);
                        let n = vec![0u8; nb];
                        let k = vec![0u8; 32];
                        let m = vec![0u8; 64];
                        let mut o = vec![0u8; 128];
                        unsafe { r(o.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), k.as_ptr()) }
                    },
                );
            } else {
                same_outcome(
                    &format!("{a} mlen=MAX+1"),
                    move || {
                        let (c, _) = sym::<BoxDet>(&a);
                        let n = vec![0u8; nb];
                        let k = vec![0u8; 32];
                        let m = vec![0u8; 64];
                        let mut o = vec![0u8; 128];
                        let mut mac = vec![0u8; 16];
                        unsafe {
                            c(o.as_mut_ptr(), mac.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), k.as_ptr())
                        }
                    },
                    move || {
                        let (_, r) = sym::<BoxDet>(&b);
                        let n = vec![0u8; nb];
                        let k = vec![0u8; 32];
                        let m = vec![0u8; 64];
                        let mut o = vec![0u8; 128];
                        let mut mac = vec![0u8; 16];
                        unsafe {
                            r(o.as_mut_ptr(), mac.as_mut_ptr(), m.as_ptr(), mlen, n.as_ptr(), k.as_ptr())
                        }
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// secretstream (PB141–PB155, B39–B42)
// ---------------------------------------------------------------------------

type SsInit = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SsPush =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> c_int;
type SsPull = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
) -> c_int;

const SS: &str = "crypto_secretstream_xchacha20poly1305";

#[test]
fn secretstream_full_lifecycle() {
    let hb = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{SS}_headerbytes"));
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        a
    };
    let ab = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{SS}_abytes"));
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        a
    };
    let sb = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{SS}_statebytes"));
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        assert!(a <= STATE_MAX);
        a
    };
    let tags: Vec<u8> = ["_tag_message", "_tag_push", "_tag_rekey", "_tag_final"]
        .iter()
        .map(|s| {
            let (c, r) = sym::<unsafe extern "C" fn() -> u8>(&format!("{SS}{s}"));
            let (a, b) = unsafe { (c(), r()) };
            assert_eq!(a, b);
            a
        })
        .collect();

    let (cip, rip) = sym::<SsInit>(&format!("{SS}_init_push"));
    let (cpu, rpu) = sym::<SsPush>(&format!("{SS}_push"));
    let (cil, ril) = sym::<SsInitPull>(&format!("{SS}_init_pull"));
    let (cpl, rpl) = sym::<SsPull>(&format!("{SS}_pull"));
    let (crk, rrk) = sym::<unsafe extern "C" fn(*mut u8)>(&format!("{SS}_rekey"));

    let mut rng = Rng::new(SEED ^ 3);

    // Every ordering of tags across a multi-message stream, plus explicit
    // rekey() calls, ad present/absent, and mlen 0.
    let scripts: Vec<Vec<(u8, usize, bool, bool)>> = vec![
        // (tag, mlen, has_ad, explicit_rekey_after)
        vec![(tags[0], 0, false, false)],
        vec![(tags[0], 1, false, false)],
        vec![(tags[0], 32, true, false)],
        vec![(tags[1], 32, true, false)],
        vec![(tags[2], 32, false, false)],
        vec![(tags[3], 32, true, false)],
        vec![
            (tags[0], 0, false, false),
            (tags[0], 1, true, false),
            (tags[1], 63, false, false),
            (tags[2], 64, true, false),
            (tags[0], 65, false, true),
            (tags[0], 127, true, false),
            (tags[3], 128, false, false),
        ],
        vec![
            (tags[0], 1000, true, true),
            (tags[2], 0, false, false),
            (tags[0], 17, true, false),
            (tags[3], 0, false, false),
        ],
        vec![(tags[0], 16, false, true), (tags[0], 16, false, true), (tags[0], 16, false, true)],
    ];

    for (si, script) in scripts.iter().enumerate() {
        let k = rng.bytes(32);
        // `init_push` draws the header from the CSPRNG, so it cannot be
        // compared byte-for-byte. It is exactly equivalent to
        // `randombytes_buf(header)` followed by `init_pull` (verified against
        // the C in `secretstream_init_push_equals_init_pull`), so the
        // deterministic part of the sending pipeline is seeded here with a
        // fixed header through `init_pull`.
        let header = rng.bytes(hb);
        let mut hc = out_buf(hb);
        let mut hr = out_buf(hb);
        hc[..hb].copy_from_slice(&header);
        hr[..hb].copy_from_slice(&header);
        let mut stc = vec![0xa5u8; STATE_MAX];
        let mut str_ = vec![0xa5u8; STATE_MAX];
        unsafe {
            let rc = cil(stc.as_mut_ptr(), hc.as_ptr(), k.as_ptr());
            let rr = ril(str_.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
            assert_eq!(rc, rr, "init_pull(send) rc script={si}");
        }
        eqb(&format!("init_pull(send) state script={si}"), &stc[..sb], &str_[..sb]);

        let mut cts: Vec<Vec<u8>> = Vec::new();
        let mut ads: Vec<Vec<u8>> = Vec::new();
        let mut msgs: Vec<Vec<u8>> = Vec::new();
        let mut used_tags: Vec<u8> = Vec::new();

        for (mi, &(tag, mlen, has_ad, rekey_after)) in script.iter().enumerate() {
            let m = rng.bytes(mlen);
            let adn = 1 + rng.below(40);
            let ad = if has_ad { rng.bytes(adn) } else { Vec::new() };
            let (adp, adlen) = if has_ad {
                (ad.as_ptr(), ad.len() as u64)
            } else {
                (ptr::null(), 0u64)
            };
            let clen = mlen + ab;
            let mut cc = out_buf(clen);
            let mut cr = out_buf(clen);
            let mut lc = u64::MAX;
            let mut lr = u64::MAX;
            unsafe {
                let rc = cpu(
                    stc.as_mut_ptr(),
                    cc.as_mut_ptr(),
                    &mut lc,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen,
                    tag,
                );
                let rr = rpu(
                    str_.as_mut_ptr(),
                    cr.as_mut_ptr(),
                    &mut lr,
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen,
                    tag,
                );
                assert_eq!(rc, rr, "push rc script={si} msg={mi}");
                assert_eq!(lc, lr, "push clen script={si} msg={mi}");
            }
            eqb(&format!("push c script={si} msg={mi} tag={tag}"), &cc, &cr);
            eqb(&format!("push state script={si} msg={mi}"), &stc[..sb], &str_[..sb]);

            // clen_p == NULL must be accepted
            let mut cc2 = out_buf(clen);
            let mut cr2 = out_buf(clen);
            let mut sc2 = stc.clone();
            let mut sr2 = str_.clone();
            unsafe {
                let rc = cpu(
                    sc2.as_mut_ptr(),
                    cc2.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen,
                    tag,
                );
                let rr = rpu(
                    sr2.as_mut_ptr(),
                    cr2.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    mlen as u64,
                    adp,
                    adlen,
                    tag,
                );
                assert_eq!(rc, rr, "push clen_p=NULL rc");
            }
            eqb(&format!("push clen_p=NULL c script={si} msg={mi}"), &cc2, &cr2);

            cts.push(cc[..clen].to_vec());
            ads.push(ad);
            msgs.push(m);
            used_tags.push(tag);

            if rekey_after {
                unsafe {
                    crk(stc.as_mut_ptr());
                    rrk(str_.as_mut_ptr());
                }
                eqb(&format!("rekey state script={si} msg={mi}"), &stc[..sb], &str_[..sb]);
            }
        }

        // Now pull the whole stream back, in both libraries, from the header
        // each produced (they are identical, checked above).
        let mut pstc = vec![0u8; STATE_MAX];
        let mut pstr = vec![0u8; STATE_MAX];
        unsafe {
            let rc = cil(pstc.as_mut_ptr(), hc.as_ptr(), k.as_ptr());
            let rr = ril(pstr.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
            assert_eq!(rc, rr, "init_pull rc script={si}");
        }
        eqb(&format!("init_pull state script={si}"), &pstc[..sb], &pstr[..sb]);

        for (mi, ct) in cts.iter().enumerate() {
            let mlen = msgs[mi].len();
            let ad = &ads[mi];
            let (adp, adlen) = if ad.is_empty() {
                (ptr::null(), 0u64)
            } else {
                (ad.as_ptr(), ad.len() as u64)
            };
            let mut pc = out_buf(mlen);
            let mut pr = out_buf(mlen);
            let mut lc = u64::MAX;
            let mut lr = u64::MAX;
            let mut tc = 0xffu8;
            let mut tr = 0xffu8;
            unsafe {
                let rc = cpl(
                    pstc.as_mut_ptr(),
                    pc.as_mut_ptr(),
                    &mut lc,
                    &mut tc,
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen,
                );
                let rr = rpl(
                    pstr.as_mut_ptr(),
                    pr.as_mut_ptr(),
                    &mut lr,
                    &mut tr,
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen,
                );
                assert_eq!(rc, rr, "pull rc script={si} msg={mi}");
                assert_eq!(lc, lr, "pull mlen script={si} msg={mi}");
                assert_eq!(tc, tr, "pull tag script={si} msg={mi}");
                assert_eq!(rc, 0, "pull should succeed script={si} msg={mi}");
                assert_eq!(tc, used_tags[mi], "pull tag value script={si} msg={mi}");
            }
            eqb(&format!("pull m script={si} msg={mi}"), &pc, &pr);
            eqb(&format!("pull roundtrip script={si} msg={mi}"), &msgs[mi], &pc[..mlen]);
            eqb(&format!("pull state script={si} msg={mi}"), &pstc[..sb], &pstr[..sb]);

            // tag_p == NULL and mlen_p == NULL, on a clone of the state
            let mut s1 = pstc.clone();
            let mut s2 = pstr.clone();
            let mut q1 = out_buf(mlen);
            let mut q2 = out_buf(mlen);
            unsafe {
                // note: this consumes the clone, not the live state
                let rc = cpl(
                    s1.as_mut_ptr(),
                    q1.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen,
                );
                let rr = rpl(
                    s2.as_mut_ptr(),
                    q2.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ct.as_ptr(),
                    ct.len() as u64,
                    adp,
                    adlen,
                );
                assert_eq!(rc, rr, "pull NULL out-params rc");
            }

            // An EXPLICIT rekey() on the sending side must be mirrored on the
            // receiving side (an implicit TAG_REKEY is handled internally by
            // both push and pull, so it needs no mirroring here).
            if script[mi].3 {
                unsafe {
                    crk(pstc.as_mut_ptr());
                    rrk(pstr.as_mut_ptr());
                }
                eqb(
                    &format!("pull-side rekey state script={si} msg={mi}"),
                    &pstc[..sb],
                    &pstr[..sb],
                );
            }
        }
    }
}

#[test]
fn secretstream_error_paths() {
    let hb = 24usize;
    let ab = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{SS}_abytes"));
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        a
    };
    let (cip, rip) = sym::<SsInit>(&format!("{SS}_init_push"));
    let (cpu, rpu) = sym::<SsPush>(&format!("{SS}_push"));
    let (cil, ril) = sym::<SsInitPull>(&format!("{SS}_init_pull"));
    let (cpl, rpl) = sym::<SsPull>(&format!("{SS}_pull"));
    let mut rng = Rng::new(SEED ^ 4);

    let k = rng.bytes(32);
    let m = rng.bytes(64);
    // Fixed header (see the note in `secretstream_full_lifecycle`).
    let header = rng.bytes(hb);
    let hc = header.clone();
    let hr = header.clone();
    let mut stc = vec![0u8; STATE_MAX];
    let mut str_ = vec![0u8; STATE_MAX];
    unsafe {
        cil(stc.as_mut_ptr(), hc.as_ptr(), k.as_ptr());
        ril(str_.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
    }
    let mut ct = vec![0u8; 64 + ab];
    unsafe {
        cpu(stc.as_mut_ptr(), ct.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 64, ptr::null(), 0, 0);
    }

    // B40: inlen < ABYTES on pull
    for clen in 0..=ab + 1 {
        let mut pstc = vec![0u8; STATE_MAX];
        let mut pstr = vec![0u8; STATE_MAX];
        unsafe {
            cil(pstc.as_mut_ptr(), hc.as_ptr(), k.as_ptr());
            ril(pstr.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
        }
        let mut pc = out_buf(80);
        let mut pr = out_buf(80);
        let mut lc = u64::MAX;
        let mut lr = u64::MAX;
        let mut tc = 0xffu8;
        let mut tr = 0xffu8;
        unsafe {
            let rc = cpl(pstc.as_mut_ptr(), pc.as_mut_ptr(), &mut lc, &mut tc, ct.as_ptr(), clen as u64, ptr::null(), 0);
            let rr = rpl(pstr.as_mut_ptr(), pr.as_mut_ptr(), &mut lr, &mut tr, ct.as_ptr(), clen as u64, ptr::null(), 0);
            assert_eq!(rc, rr, "pull short clen={clen}");
            if rc == 0 {
                assert_eq!(lc, lr);
                assert_eq!(tc, tr);
            }
        }
        eqb(&format!("pull short clen={clen}"), &pc, &pr);
    }

    // B42: MAC mismatch — every single-byte corruption of the ciphertext.
    for pos in 0..ct.len() {
        let mut bad = ct.clone();
        bad[pos] ^= 0x40;
        let mut pstc = vec![0u8; STATE_MAX];
        let mut pstr = vec![0u8; STATE_MAX];
        unsafe {
            cil(pstc.as_mut_ptr(), hc.as_ptr(), k.as_ptr());
            ril(pstr.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
        }
        let mut pc = out_buf(80);
        let mut pr = out_buf(80);
        let mut lc = u64::MAX;
        let mut lr = u64::MAX;
        let mut tc = 0xffu8;
        let mut tr = 0xffu8;
        unsafe {
            let rc = cpl(
                pstc.as_mut_ptr(), pc.as_mut_ptr(), &mut lc, &mut tc,
                bad.as_ptr(), bad.len() as u64, ptr::null(), 0,
            );
            let rr = rpl(
                pstr.as_mut_ptr(), pr.as_mut_ptr(), &mut lr, &mut tr,
                bad.as_ptr(), bad.len() as u64, ptr::null(), 0,
            );
            assert_eq!(rc, rr, "pull corrupted@{pos}");
            assert_eq!(rc, -1, "pull corrupted@{pos} must fail");
        }
        eqb(&format!("pull corrupted@{pos}"), &pc, &pr);
    }

    // Wrong header, wrong key, wrong ad.
    let k2 = rng.bytes(32);
    let h2 = rng.bytes(hb);
    for (tag, hp, kp) in [("wrong-header", h2.as_ptr(), k.as_ptr()), ("wrong-key", hc.as_ptr(), k2.as_ptr())] {
        let mut pstc = vec![0u8; STATE_MAX];
        let mut pstr = vec![0u8; STATE_MAX];
        unsafe {
            let rc = cil(pstc.as_mut_ptr(), hp, kp);
            let rr = ril(pstr.as_mut_ptr(), hp, kp);
            assert_eq!(rc, rr, "init_pull {tag}");
        }
        let mut pc = out_buf(80);
        let mut pr = out_buf(80);
        let mut lc = 0u64;
        let mut lr = 0u64;
        let mut tc = 0u8;
        let mut tr = 0u8;
        unsafe {
            let rc = cpl(pstc.as_mut_ptr(), pc.as_mut_ptr(), &mut lc, &mut tc, ct.as_ptr(), ct.len() as u64, ptr::null(), 0);
            let rr = rpl(pstr.as_mut_ptr(), pr.as_mut_ptr(), &mut lr, &mut tr, ct.as_ptr(), ct.len() as u64, ptr::null(), 0);
            assert_eq!(rc, rr, "pull {tag}");
        }
        eqb(&format!("pull {tag}"), &pc, &pr);
    }
    // ad mismatch
    let ad = rng.bytes(20);
    let mut pstc = vec![0u8; STATE_MAX];
    let mut pstr = vec![0u8; STATE_MAX];
    unsafe {
        cil(pstc.as_mut_ptr(), hc.as_ptr(), k.as_ptr());
        ril(pstr.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
    }
    let mut pc = out_buf(80);
    let mut pr = out_buf(80);
    let mut lc = 0u64;
    let mut lr = 0u64;
    let mut tc = 0u8;
    let mut tr = 0u8;
    unsafe {
        let rc = cpl(pstc.as_mut_ptr(), pc.as_mut_ptr(), &mut lc, &mut tc, ct.as_ptr(), ct.len() as u64, ad.as_ptr(), 20);
        let rr = rpl(pstr.as_mut_ptr(), pr.as_mut_ptr(), &mut lr, &mut tr, ct.as_ptr(), ct.len() as u64, ad.as_ptr(), 20);
        assert_eq!(rc, rr, "pull ad mismatch");
        assert_eq!(rc, -1);
    }
    eqb("pull ad mismatch", &pc, &pr);

    // Out-of-range tag values passed to push. The C stores `tag` verbatim into
    // the first block and only tests `tag & TAG_REKEY`, so ANY u8 is a legal
    // input across the FFI boundary — including values with no named variant.
    for tag in 0u8..=255 {
        let mut sc = vec![0u8; STATE_MAX];
        let mut sr = vec![0u8; STATE_MAX];
        let h1 = header.clone();
        let h2 = header.clone();
        unsafe {
            cil(sc.as_mut_ptr(), h1.as_ptr(), k.as_ptr());
            ril(sr.as_mut_ptr(), h2.as_ptr(), k.as_ptr());
        }
        eqb(&format!("init_pull state tag={tag}"), &sc[..STATE_MAX], &sr[..STATE_MAX]);
        let mut cc = out_buf(64 + ab);
        let mut cr = out_buf(64 + ab);
        let mut lc = 0u64;
        let mut lr = 0u64;
        unsafe {
            let rc = cpu(sc.as_mut_ptr(), cc.as_mut_ptr(), &mut lc, m.as_ptr(), 64, ptr::null(), 0, tag);
            let rr = rpu(sr.as_mut_ptr(), cr.as_mut_ptr(), &mut lr, m.as_ptr(), 64, ptr::null(), 0, tag);
            assert_eq!(rc, rr, "push tag={tag} rc");
            assert_eq!(lc, lr, "push tag={tag} clen");
        }
        eqb(&format!("push tag={tag}"), &cc, &cr);
        // and pull it back, checking the tag survives verbatim
        let mut pc2 = vec![0u8; STATE_MAX];
        let mut pr2 = vec![0u8; STATE_MAX];
        unsafe {
            cil(pc2.as_mut_ptr(), h1.as_ptr(), k.as_ptr());
            ril(pr2.as_mut_ptr(), h2.as_ptr(), k.as_ptr());
        }
        let mut oc = out_buf(64);
        let mut or = out_buf(64);
        let mut tc = 0u8;
        let mut tr = 0u8;
        unsafe {
            let rc = cpl(pc2.as_mut_ptr(), oc.as_mut_ptr(), &mut lc, &mut tc, cc.as_ptr(), (64 + ab) as u64, ptr::null(), 0);
            let rr = rpl(pr2.as_mut_ptr(), or.as_mut_ptr(), &mut lr, &mut tr, cr.as_ptr(), (64 + ab) as u64, ptr::null(), 0);
            assert_eq!(rc, rr, "pull tag={tag} rc");
            assert_eq!(tc, tr, "pull tag={tag} tag_out");
        }
        eqb(&format!("pull tag={tag} m"), &oc, &or);
    }
}

#[test]
fn secretstream_oversized_aborts_identically() {
    let (cm, rm) = sym::<unsafe extern "C" fn() -> usize>(&format!("{SS}_messagebytes_max"));
    let (cv, rv) = unsafe { (cm(), rm()) };
    assert_eq!(cv, rv);
    if cv == usize::MAX {
        return;
    }
    let mlen = (cv as u64).wrapping_add(1);
    same_outcome(
        "secretstream push mlen=MAX+1",
        move || {
            let (ci, _) = sym::<SsInit>(&format!("{SS}_init_push"));
            let (cp, _) = sym::<SsPush>(&format!("{SS}_push"));
            let k = [0u8; 32];
            let mut h = [0u8; 24];
            let mut st = vec![0u8; STATE_MAX];
            let m = [0u8; 64];
            let mut o = vec![0u8; 128];
            unsafe {
                ci(st.as_mut_ptr(), h.as_mut_ptr(), k.as_ptr());
                cp(st.as_mut_ptr(), o.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen, ptr::null(), 0, 0)
            }
        },
        move || {
            let (_, ri) = sym::<SsInit>(&format!("{SS}_init_push"));
            let (_, rp) = sym::<SsPush>(&format!("{SS}_push"));
            let k = [0u8; 32];
            let mut h = [0u8; 24];
            let mut st = vec![0u8; STATE_MAX];
            let m = [0u8; 64];
            let mut o = vec![0u8; 128];
            unsafe {
                ri(st.as_mut_ptr(), h.as_mut_ptr(), k.as_ptr());
                rp(st.as_mut_ptr(), o.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen, ptr::null(), 0, 0)
            }
        },
    );
    // pull with mlen > MAX
    same_outcome(
        "secretstream pull clen=MAX+1",
        move || {
            let (ci, _) = sym::<SsInitPull>(&format!("{SS}_init_pull"));
            let (cp, _) = sym::<SsPull>(&format!("{SS}_pull"));
            let k = [0u8; 32];
            let h = [0u8; 24];
            let mut st = vec![0u8; STATE_MAX];
            let c = vec![0u8; 128];
            let mut o = vec![0u8; 128];
            unsafe {
                ci(st.as_mut_ptr(), h.as_ptr(), k.as_ptr());
                cp(st.as_mut_ptr(), o.as_mut_ptr(), ptr::null_mut(), ptr::null_mut(), c.as_ptr(), mlen, ptr::null(), 0)
            }
        },
        move || {
            let (_, ri) = sym::<SsInitPull>(&format!("{SS}_init_pull"));
            let (_, rp) = sym::<SsPull>(&format!("{SS}_pull"));
            let k = [0u8; 32];
            let h = [0u8; 24];
            let mut st = vec![0u8; STATE_MAX];
            let c = vec![0u8; 128];
            let mut o = vec![0u8; 128];
            unsafe {
                ri(st.as_mut_ptr(), h.as_ptr(), k.as_ptr());
                rp(st.as_mut_ptr(), o.as_mut_ptr(), ptr::null_mut(), ptr::null_mut(), c.as_ptr(), mlen, ptr::null(), 0)
            }
        },
    );
}

// ---------------------------------------------------------------------------
// aead (PB156–PB209, B43–B79)
// ---------------------------------------------------------------------------

type AeadEnc = unsafe extern "C" fn(
    *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8, *const u8,
) -> c_int;
type AeadDec = unsafe extern "C" fn(
    *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64, *const u8, *const u8,
) -> c_int;
type AeadEncDet = unsafe extern "C" fn(
    *mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8, *const u8,
) -> c_int;
type AeadDecDet = unsafe extern "C" fn(
    *mut u8, *mut u8, *const u8, u64, *const u8, *const u8, u64, *const u8, *const u8,
) -> c_int;

struct Aead {
    pfx: &'static str,
    keybytes: usize,
    npubbytes: usize,
    abytes: usize,
}

const AEADS: &[Aead] = &[
    Aead { pfx: "crypto_aead_chacha20poly1305", keybytes: 32, npubbytes: 8, abytes: 16 },
    Aead { pfx: "crypto_aead_chacha20poly1305_ietf", keybytes: 32, npubbytes: 12, abytes: 16 },
    Aead { pfx: "crypto_aead_xchacha20poly1305_ietf", keybytes: 32, npubbytes: 24, abytes: 16 },
    Aead { pfx: "crypto_aead_aegis128l", keybytes: 16, npubbytes: 16, abytes: 32 },
    Aead { pfx: "crypto_aead_aegis256", keybytes: 32, npubbytes: 32, abytes: 32 },
];

fn aead_lens() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=40).collect();
    v.extend_from_slice(&[
        62, 63, 64, 65, 66, 95, 96, 97, 127, 128, 129, 191, 192, 193, 255, 256, 257, 1000,
    ]);
    v
}

fn ad_lens() -> Vec<usize> {
    vec![0, 1, 2, 15, 16, 17, 31, 32, 33, 63, 64, 65, 200]
}

#[test]
fn aead_combined_roundtrip() {
    let mut rng = Rng::new(SEED ^ 5);
    for a in AEADS {
        // verify the advertised sizes
        for (suffix, want) in [
            ("_keybytes", a.keybytes),
            ("_npubbytes", a.npubbytes),
            ("_abytes", a.abytes),
        ] {
            let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{}{suffix}", a.pfx));
            let (cv, rv) = unsafe { (c(), r()) };
            assert_eq!(cv, rv, "{}{suffix}", a.pfx);
            assert_eq!(cv, want, "{}{suffix}", a.pfx);
        }
        let (ce, re) = sym::<AeadEnc>(&format!("{}_encrypt", a.pfx));
        let (cd, rd) = sym::<AeadDec>(&format!("{}_decrypt", a.pfx));

        for mlen in aead_lens() {
            for &adlen in &[0usize, 1, 16, 17, 33] {
                let k = rng.bytes(a.keybytes);
                let npub = rng.bytes(a.npubbytes);
                let m = rng.bytes(mlen);
                let ad = rng.bytes(adlen);
                let adp = if adlen == 0 { ptr::null() } else { ad.as_ptr() };
                let clen = mlen + a.abytes;

                let mut cc = out_buf(clen);
                let mut cr = out_buf(clen);
                let mut lc = u64::MAX;
                let mut lr = u64::MAX;
                unsafe {
                    let rc = ce(cc.as_mut_ptr(), &mut lc, m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                    let rr = re(cr.as_mut_ptr(), &mut lr, m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_encrypt rc m={mlen} ad={adlen}", a.pfx);
                    assert_eq!(lc, lr, "{}_encrypt clen m={mlen} ad={adlen}", a.pfx);
                }
                eqb(&format!("{}_encrypt m={mlen} ad={adlen}", a.pfx), &cc, &cr);

                // clen_p == NULL
                let mut cc2 = out_buf(clen);
                let mut cr2 = out_buf(clen);
                unsafe {
                    ce(cc2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                    re(cr2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{}_encrypt clen_p=NULL m={mlen} ad={adlen}", a.pfx), &cc2, &cr2);

                // decrypt
                let mut pc = out_buf(mlen);
                let mut pr = out_buf(mlen);
                let mut mc = u64::MAX;
                let mut mr = u64::MAX;
                unsafe {
                    let rc = cd(pc.as_mut_ptr(), &mut mc, ptr::null_mut(), cc.as_ptr(), clen as u64, adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    let rr = rd(pr.as_mut_ptr(), &mut mr, ptr::null_mut(), cr.as_ptr(), clen as u64, adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_decrypt rc m={mlen} ad={adlen}", a.pfx);
                    assert_eq!(mc, mr, "{}_decrypt mlen m={mlen} ad={adlen}", a.pfx);
                    assert_eq!(rc, 0, "{}_decrypt should succeed", a.pfx);
                }
                eqb(&format!("{}_decrypt m={mlen} ad={adlen}", a.pfx), &pc, &pr);
                eqb(&format!("{} roundtrip m={mlen} ad={adlen}", a.pfx), &m, &pc[..mlen]);

                // B44/B46: every clen below ABYTES, and a few above
                for shortlen in 0..=a.abytes {
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    let mut mc = u64::MAX;
                    let mut mr = u64::MAX;
                    unsafe {
                        let rc = cd(pc.as_mut_ptr(), &mut mc, ptr::null_mut(), cc.as_ptr(), shortlen as u64, adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                        let rr = rd(pr.as_mut_ptr(), &mut mr, ptr::null_mut(), cr.as_ptr(), shortlen as u64, adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                        assert_eq!(rc, rr, "{}_decrypt short clen={shortlen}", a.pfx);
                        if rc == 0 {
                            assert_eq!(mc, mr);
                        }
                    }
                    eqb(&format!("{}_decrypt short clen={shortlen} m={mlen}", a.pfx), &pc, &pr);
                }
                // tamper in the tag and in the body
                for pos in [0usize, mlen.saturating_sub(1), clen - a.abytes, clen - 1] {
                    if pos >= clen {
                        continue;
                    }
                    let mut bad = cc[..clen].to_vec();
                    bad[pos] ^= 0x10;
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    let mut mc = u64::MAX;
                    let mut mr = u64::MAX;
                    unsafe {
                        let rc = cd(pc.as_mut_ptr(), &mut mc, ptr::null_mut(), bad.as_ptr(), clen as u64, adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                        let rr = rd(pr.as_mut_ptr(), &mut mr, ptr::null_mut(), bad.as_ptr(), clen as u64, adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                        assert_eq!(rc, rr, "{}_decrypt tamper@{pos}", a.pfx);
                        assert_eq!(rc, -1, "{}_decrypt tamper@{pos} must fail", a.pfx);
                    }
                    eqb(&format!("{}_decrypt tamper@{pos} m={mlen}", a.pfx), &pc, &pr);
                }
                // wrong ad
                if adlen > 0 {
                    let mut bad_ad = ad.clone();
                    bad_ad[0] ^= 1;
                    let mut pc = out_buf(mlen.max(1));
                    let mut pr = out_buf(mlen.max(1));
                    let mut mc = 0u64;
                    let mut mr = 0u64;
                    unsafe {
                        let rc = cd(pc.as_mut_ptr(), &mut mc, ptr::null_mut(), cc.as_ptr(), clen as u64, bad_ad.as_ptr(), adlen as u64, npub.as_ptr(), k.as_ptr());
                        let rr = rd(pr.as_mut_ptr(), &mut mr, ptr::null_mut(), cr.as_ptr(), clen as u64, bad_ad.as_ptr(), adlen as u64, npub.as_ptr(), k.as_ptr());
                        assert_eq!(rc, rr, "{}_decrypt wrong ad", a.pfx);
                        assert_eq!(rc, -1);
                    }
                    eqb(&format!("{}_decrypt wrong ad m={mlen}", a.pfx), &pc, &pr);
                }
            }
        }
        // ad length sweep at a fixed message length
        for adlen in ad_lens() {
            let k = rng.bytes(a.keybytes);
            let npub = rng.bytes(a.npubbytes);
            let m = rng.bytes(70);
            let ad = rng.bytes(adlen);
            let adp = if adlen == 0 { ptr::null() } else { ad.as_ptr() };
            let mut cc = out_buf(70 + a.abytes);
            let mut cr = out_buf(70 + a.abytes);
            unsafe {
                ce(cc.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 70, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                re(cr.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 70, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{}_encrypt ad sweep adlen={adlen}", a.pfx), &cc, &cr);
        }
        // extreme keys and nonces
        for (tag, fill) in [("zero", 0u8), ("ones", 0xff)] {
            let k = vec![fill; a.keybytes];
            let npub = vec![fill; a.npubbytes];
            for mlen in [0usize, 1, 31, 32, 33, 64, 65] {
                let m = vec![fill; mlen];
                let mut cc = out_buf(mlen + a.abytes);
                let mut cr = out_buf(mlen + a.abytes);
                unsafe {
                    ce(cc.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, ptr::null(), 0, ptr::null(), npub.as_ptr(), k.as_ptr());
                    re(cr.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, ptr::null(), 0, ptr::null(), npub.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{}_encrypt {tag} m={mlen}", a.pfx), &cc, &cr);
            }
        }
    }
}

#[test]
fn aead_detached_roundtrip_and_verify_only() {
    let mut rng = Rng::new(SEED ^ 6);
    for a in AEADS {
        let (ce, re) = sym::<AeadEncDet>(&format!("{}_encrypt_detached", a.pfx));
        let (cd, rd) = sym::<AeadDecDet>(&format!("{}_decrypt_detached", a.pfx));
        for mlen in aead_lens() {
            for &adlen in &[0usize, 1, 16, 31, 32, 33] {
                let k = rng.bytes(a.keybytes);
                let npub = rng.bytes(a.npubbytes);
                let m = rng.bytes(mlen);
                let ad = rng.bytes(adlen);
                let adp = if adlen == 0 { ptr::null() } else { ad.as_ptr() };

                let mut cc = out_buf(mlen);
                let mut cr = out_buf(mlen);
                let mut mac_c = out_buf(a.abytes);
                let mut mac_r = out_buf(a.abytes);
                let mut lc = u64::MAX;
                let mut lr = u64::MAX;
                unsafe {
                    let rc = ce(cc.as_mut_ptr(), mac_c.as_mut_ptr(), &mut lc, m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                    let rr = re(cr.as_mut_ptr(), mac_r.as_mut_ptr(), &mut lr, m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_encrypt_detached rc m={mlen} ad={adlen}", a.pfx);
                    assert_eq!(lc, lr, "{}_encrypt_detached maclen", a.pfx);
                }
                eqb(&format!("{}_encrypt_detached c m={mlen} ad={adlen}", a.pfx), &cc, &cr);
                eqb(&format!("{}_encrypt_detached mac m={mlen} ad={adlen}", a.pfx), &mac_c, &mac_r);

                // maclen_p == NULL
                let mut cc2 = out_buf(mlen);
                let mut cr2 = out_buf(mlen);
                let mut mac_c2 = out_buf(a.abytes);
                let mut mac_r2 = out_buf(a.abytes);
                unsafe {
                    ce(cc2.as_mut_ptr(), mac_c2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                    re(cr2.as_mut_ptr(), mac_r2.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, adp, adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr());
                }
                eqb(&format!("{}_encrypt_detached maclen_p=NULL c", a.pfx), &cc2, &cr2);
                eqb(&format!("{}_encrypt_detached maclen_p=NULL mac", a.pfx), &mac_c2, &mac_r2);

                // decrypt_detached
                let mut pc = out_buf(mlen);
                let mut pr = out_buf(mlen);
                unsafe {
                    let rc = cd(pc.as_mut_ptr(), ptr::null_mut(), cc.as_ptr(), mlen as u64, mac_c.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    let rr = rd(pr.as_mut_ptr(), ptr::null_mut(), cr.as_ptr(), mlen as u64, mac_r.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_decrypt_detached rc m={mlen} ad={adlen}", a.pfx);
                    assert_eq!(rc, 0, "{}_decrypt_detached should succeed", a.pfx);
                }
                eqb(&format!("{}_decrypt_detached m={mlen} ad={adlen}", a.pfx), &pc, &pr);
                eqb(&format!("{} detached roundtrip m={mlen}", a.pfx), &m, &pc[..mlen]);

                // B48: the verify-only path, m == NULL. On the AEGIS variants
                // this returns the RAW crypto_verify result, not -1/0.
                unsafe {
                    let rc = cd(ptr::null_mut(), ptr::null_mut(), cc.as_ptr(), mlen as u64, mac_c.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    let rr = rd(ptr::null_mut(), ptr::null_mut(), cr.as_ptr(), mlen as u64, mac_r.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_decrypt_detached m=NULL rc m={mlen}", a.pfx);
                }
                // verify-only with a BAD mac
                let mut badmac = mac_c[..a.abytes].to_vec();
                badmac[0] ^= 0x01;
                unsafe {
                    let rc = cd(ptr::null_mut(), ptr::null_mut(), cc.as_ptr(), mlen as u64, badmac.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    let rr = rd(ptr::null_mut(), ptr::null_mut(), cr.as_ptr(), mlen as u64, badmac.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_decrypt_detached m=NULL bad mac rc m={mlen}", a.pfx);
                }
                // bad mac with a real output buffer
                let mut pc = out_buf(mlen.max(1));
                let mut pr = out_buf(mlen.max(1));
                unsafe {
                    let rc = cd(pc.as_mut_ptr(), ptr::null_mut(), cc.as_ptr(), mlen as u64, badmac.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    let rr = rd(pr.as_mut_ptr(), ptr::null_mut(), cr.as_ptr(), mlen as u64, badmac.as_ptr(), adp, adlen as u64, npub.as_ptr(), k.as_ptr());
                    assert_eq!(rc, rr, "{}_decrypt_detached bad mac rc", a.pfx);
                    assert_eq!(rc, -1);
                }
                eqb(&format!("{}_decrypt_detached bad mac m={mlen}", a.pfx), &pc, &pr);
            }
        }
        // in-place detached (c == m)
        for mlen in [0usize, 1, 31, 32, 33, 64, 200] {
            let k = rng.bytes(a.keybytes);
            let npub = rng.bytes(a.npubbytes);
            let m = rng.bytes(mlen);
            let mut bc = out_buf(mlen);
            let mut br = out_buf(mlen);
            bc[..mlen].copy_from_slice(&m);
            br[..mlen].copy_from_slice(&m);
            let mut mac_c = out_buf(a.abytes);
            let mut mac_r = out_buf(a.abytes);
            unsafe {
                ce(bc.as_mut_ptr(), mac_c.as_mut_ptr(), ptr::null_mut(), bc.as_ptr(), mlen as u64, ptr::null(), 0, ptr::null(), npub.as_ptr(), k.as_ptr());
                re(br.as_mut_ptr(), mac_r.as_mut_ptr(), ptr::null_mut(), br.as_ptr(), mlen as u64, ptr::null(), 0, ptr::null(), npub.as_ptr(), k.as_ptr());
            }
            eqb(&format!("{}_encrypt_detached in-place c m={mlen}", a.pfx), &bc, &br);
            eqb(&format!("{}_encrypt_detached in-place mac m={mlen}", a.pfx), &mac_c, &mac_r);
        }
    }
}

/// B70–B79: with no `HAVE_AESNI`/`HAVE_ARMCRYPTO`, every operational
/// `crypto_aead_aes256gcm_*` entry point is the ENOSYS stub. All nine of them,
/// plus `is_available() == 0`.
#[test]
fn aes256gcm_is_the_enosys_stub() {
    let (cav, rav) = sym::<unsafe extern "C" fn() -> c_int>("crypto_aead_aes256gcm_is_available");
    unsafe {
        let (a, b) = (cav(), rav());
        assert_eq!(a, b, "aes256gcm_is_available");
        assert_eq!(a, 0, "this build must NOT have aes256gcm available");
    }

    let mut rng = Rng::new(SEED ^ 7);
    let k = rng.bytes(32);
    let npub = rng.bytes(12);
    let m = rng.bytes(64);
    let ad = rng.bytes(16);
    let mut o = vec![0u8; 256];
    let mut mac = vec![0u8; 16];
    let mut ctx_c = vec![0u8; 1024];
    let mut ctx_r = vec![0u8; 1024];
    let mut l = 0u64;

    macro_rules! both {
        ($name:literal, $ty:ty, $call:expr) => {{
            let (c, r) = sym::<$ty>($name);
            unsafe {
                set_errno(0);
                let rc = ($call)(c);
                let ec = errno();
                set_errno(0);
                let rr = ($call)(r);
                let er = errno();
                assert_eq!(rc, rr, concat!($name, ": rc"));
                assert_eq!(ec, er, concat!($name, ": errno"));
                assert_eq!(rc, -1, concat!($name, ": must be -1"));
                assert_eq!(ec, libc::ENOSYS, concat!($name, ": must be ENOSYS"));
            }
        }};
    }

    both!("crypto_aead_aes256gcm_encrypt", AeadEnc, |f: AeadEnc| unsafe {
        f(o.as_mut_ptr(), &mut l, m.as_ptr(), 64, ad.as_ptr(), 16, ptr::null(), npub.as_ptr(), k.as_ptr())
    });
    both!("crypto_aead_aes256gcm_decrypt", AeadDec, |f: AeadDec| unsafe {
        f(o.as_mut_ptr(), &mut l, ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, npub.as_ptr(), k.as_ptr())
    });
    both!("crypto_aead_aes256gcm_encrypt_detached", AeadEncDet, |f: AeadEncDet| unsafe {
        f(o.as_mut_ptr(), mac.as_mut_ptr(), &mut l, m.as_ptr(), 64, ad.as_ptr(), 16, ptr::null(), npub.as_ptr(), k.as_ptr())
    });
    both!("crypto_aead_aes256gcm_decrypt_detached", AeadDecDet, |f: AeadDecDet| unsafe {
        f(o.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 64, mac.as_ptr(), ad.as_ptr(), 16, npub.as_ptr(), k.as_ptr())
    });

    // beforenm and the four afternm variants
    {
        type BeforeNm = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
        let (c, r) = sym::<BeforeNm>("crypto_aead_aes256gcm_beforenm");
        unsafe {
            set_errno(0);
            let rc = c(ctx_c.as_mut_ptr(), k.as_ptr());
            let ec = errno();
            set_errno(0);
            let rr = r(ctx_r.as_mut_ptr(), k.as_ptr());
            let er = errno();
            assert_eq!(rc, rr, "beforenm rc");
            assert_eq!(ec, er, "beforenm errno");
            assert_eq!(rc, -1);
            assert_eq!(ec, libc::ENOSYS);
        }
        eqb("beforenm ctx untouched", &ctx_c, &ctx_r);
    }
    type EncAfter = unsafe extern "C" fn(
        *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8, *const u8,
    ) -> c_int;
    type DecAfter = unsafe extern "C" fn(
        *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64, *const u8, *const u8,
    ) -> c_int;
    type EncDetAfter = unsafe extern "C" fn(
        *mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8, *const u8,
    ) -> c_int;
    type DecDetAfter = unsafe extern "C" fn(
        *mut u8, *mut u8, *const u8, u64, *const u8, *const u8, u64, *const u8, *const u8,
    ) -> c_int;

    both!("crypto_aead_aes256gcm_encrypt_afternm", EncAfter, |f: EncAfter| {
        f(o.as_mut_ptr(), &mut l, m.as_ptr(), 64, ad.as_ptr(), 16, ptr::null(), npub.as_ptr(), ctx_c.as_ptr())
    });
    both!("crypto_aead_aes256gcm_decrypt_afternm", DecAfter, |f: DecAfter| {
        f(o.as_mut_ptr(), &mut l, ptr::null_mut(), m.as_ptr(), 64, ad.as_ptr(), 16, npub.as_ptr(), ctx_c.as_ptr())
    });
    both!("crypto_aead_aes256gcm_encrypt_detached_afternm", EncDetAfter, |f: EncDetAfter| {
        f(o.as_mut_ptr(), mac.as_mut_ptr(), &mut l, m.as_ptr(), 64, ad.as_ptr(), 16, ptr::null(), npub.as_ptr(), ctx_c.as_ptr())
    });
    both!("crypto_aead_aes256gcm_decrypt_detached_afternm", DecDetAfter, |f: DecDetAfter| {
        f(o.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), 64, mac.as_ptr(), ad.as_ptr(), 16, npub.as_ptr(), ctx_c.as_ptr())
    });

    // keygen still works (it is outside the #if)
    let (ck, rk) = sym::<unsafe extern "C" fn(*mut u8)>("crypto_aead_aes256gcm_keygen");
    let mut bc = out_buf(32);
    let mut br = out_buf(32);
    unsafe {
        ck(bc.as_mut_ptr());
        rk(br.as_mut_ptr());
    }
    eqb("aes256gcm_keygen canary", &bc[32..], &br[32..]);
}

/// B43/B45/B58/B64: `MESSAGEBYTES_MAX` overflow. On the chacha family the
/// encrypt path calls `sodium_misuse()`; on the AEGIS detached path the C
/// returns `-1` instead. Both are covered by comparing the process outcome.
#[test]
fn aead_oversized_lengths_match() {
    for a in AEADS {
        let (cm, rm) = sym::<unsafe extern "C" fn() -> usize>(&format!("{}_messagebytes_max", a.pfx));
        let (cv, rv) = unsafe { (cm(), rm()) };
        assert_eq!(cv, rv, "{}_messagebytes_max", a.pfx);
        if cv == usize::MAX {
            continue;
        }
        let over = (cv as u64).wrapping_add(1);
        let kb = a.keybytes;
        let nb = a.npubbytes;
        for (suffix, kind) in [("_encrypt", 0), ("_encrypt_detached", 1), ("_decrypt", 2), ("_decrypt_detached", 3)] {
            let nm = format!("{}{suffix}", a.pfx);
            let n1 = nm.clone();
            let n2 = nm.clone();
            let mk = move |use_rust: bool| -> i32 {
                let name = if use_rust { &n2 } else { &n1 };
                let k = vec![0u8; kb];
                let npub = vec![0u8; nb];
                let m = vec![0u8; 128];
                let mut o = vec![0u8; 256];
                let mut mac = vec![0u8; 32];
                let mut l = 0u64;
                unsafe {
                    match kind {
                        0 => {
                            let (c, r) = sym::<AeadEnc>(name);
                            let f = if use_rust { r } else { c };
                            f(o.as_mut_ptr(), &mut l, m.as_ptr(), over, ptr::null(), 0, ptr::null(), npub.as_ptr(), k.as_ptr())
                        }
                        1 => {
                            let (c, r) = sym::<AeadEncDet>(name);
                            let f = if use_rust { r } else { c };
                            f(o.as_mut_ptr(), mac.as_mut_ptr(), &mut l, m.as_ptr(), over, ptr::null(), 0, ptr::null(), npub.as_ptr(), k.as_ptr())
                        }
                        2 => {
                            let (c, r) = sym::<AeadDec>(name);
                            let f = if use_rust { r } else { c };
                            f(o.as_mut_ptr(), &mut l, ptr::null_mut(), m.as_ptr(), over, ptr::null(), 0, npub.as_ptr(), k.as_ptr())
                        }
                        _ => {
                            let (c, r) = sym::<AeadDecDet>(name);
                            let f = if use_rust { r } else { c };
                            f(o.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), over, mac.as_ptr(), ptr::null(), 0, npub.as_ptr(), k.as_ptr())
                        }
                    }
                }
            };
            let m1 = mk.clone();
            same_outcome(&format!("{nm} len=MAX+1"), move || m1(false), move || mk(true));
        }
    }
}

/// `init_push` is the only non-deterministic entry point in the secretstream
/// API: it fills the header from the CSPRNG. What IS checkable, and what the
/// rest of the secretstream tests rely on, is that
///
///   init_push(state, header, k)  ==  randombytes_buf(header) ; init_pull(state, header, k)
///
/// This test pins that down in BOTH libraries, and cross-checks that each
/// library's `init_pull` derives the same state from the other's header.
#[test]
fn secretstream_init_push_equals_init_pull() {
    let hb = 24usize;
    let sb = {
        let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{SS}_statebytes"));
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b);
        a
    };
    let (cip, rip) = sym::<SsInit>(&format!("{SS}_init_push"));
    let (cil, ril) = sym::<SsInitPull>(&format!("{SS}_init_pull"));
    let mut rng = Rng::new(SEED ^ 0x99);

    for _ in 0..200 {
        let k = rng.bytes(32);

        // C: init_push, then C+Rust init_pull from the header it produced.
        let mut hc = out_buf(hb);
        let mut sc_push = vec![0xa5u8; STATE_MAX];
        unsafe {
            assert_eq!(cip(sc_push.as_mut_ptr(), hc.as_mut_ptr(), k.as_ptr()), 0);
        }
        let mut sc_pull = vec![0xa5u8; STATE_MAX];
        let mut sr_pull = vec![0xa5u8; STATE_MAX];
        unsafe {
            assert_eq!(cil(sc_pull.as_mut_ptr(), hc.as_ptr(), k.as_ptr()), 0);
            assert_eq!(ril(sr_pull.as_mut_ptr(), hc.as_ptr(), k.as_ptr()), 0);
        }
        eqb("C init_push state == C init_pull state", &sc_push[..sb], &sc_pull[..sb]);
        eqb("C init_pull == Rust init_pull (C header)", &sc_pull[..sb], &sr_pull[..sb]);

        // Rust: init_push, then C+Rust init_pull from ITS header.
        let mut hr = out_buf(hb);
        let mut sr_push = vec![0x5au8; STATE_MAX];
        unsafe {
            assert_eq!(rip(sr_push.as_mut_ptr(), hr.as_mut_ptr(), k.as_ptr()), 0);
        }
        let mut sc2 = vec![0x5au8; STATE_MAX];
        let mut sr2 = vec![0x5au8; STATE_MAX];
        unsafe {
            cil(sc2.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
            ril(sr2.as_mut_ptr(), hr.as_ptr(), k.as_ptr());
        }
        eqb("Rust init_push state == Rust init_pull state", &sr_push[..sb], &sr2[..sb]);
        eqb("C init_pull == Rust init_pull (Rust header)", &sc2[..sb], &sr2[..sb]);

        // headers must be well-formed and not constant
        eqb("init_push canary C", &hc[hb..], &out_buf(hb)[hb..]);
        eqb("init_push canary Rust", &hr[hb..], &out_buf(hb)[hb..]);
        assert_ne!(&hc[..hb], &vec![0u8; hb][..], "C header all zeros");
        assert_ne!(&hr[..hb], &vec![0u8; hb][..], "Rust header all zeros");
    }
}
