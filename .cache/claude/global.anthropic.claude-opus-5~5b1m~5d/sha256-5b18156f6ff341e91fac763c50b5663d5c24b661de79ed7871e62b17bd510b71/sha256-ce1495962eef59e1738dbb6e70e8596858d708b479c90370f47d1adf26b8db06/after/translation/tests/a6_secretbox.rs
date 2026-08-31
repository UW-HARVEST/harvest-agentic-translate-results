//! Area 6 — `crypto_secretbox/`: the generic (xsalsa20poly1305) family, the
//! NaCl-style zero-padded API and the xchacha20poly1305 primitive family.
//!
//! Covers `configs_6.md` rows 6.78–6.102 and `errors_6.md` rows 6.50–6.67.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int};
use std::ptr::{null, null_mut};

// c/m/mlen/n/k — `_easy`, `_open_easy`, `crypto_secretbox`, `crypto_secretbox_open`
type Easy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
// c/mac/m/mlen/n/k
type Det = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
// m/c/mac/clen/n/k
type OpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
type Getter = unsafe extern "C" fn() -> usize;
type Keygen = unsafe extern "C" fn(*mut u8);
type Primitive = unsafe extern "C" fn() -> *const c_char;

const MLEN: [usize; 14] = [0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129];
const BIG_SECRETBOX: [usize; 9] = [32, 33, 63, 64, 65, 4096, 131072, 131073, 262145];
const MAC: usize = 16;

fn poisoned(len: usize) -> Vec<u8> {
    let mut v = padded(len);
    for b in v[..len].iter_mut() {
        *b = 0xDD;
    }
    v
}

struct Sb {
    name: &'static str,
    easy: (Symbol<'static, Easy>, Symbol<'static, Easy>),
    open_easy: (Symbol<'static, Easy>, Symbol<'static, Easy>),
    det: (Symbol<'static, Det>, Symbol<'static, Det>),
    open_det: (Symbol<'static, OpenDet>, Symbol<'static, OpenDet>),
}

fn sb(prefix: &'static str) -> Sb {
    Sb {
        name: prefix,
        easy: both::<Easy>(&format!("{prefix}_easy")),
        open_easy: both::<Easy>(&format!("{prefix}_open_easy")),
        det: both::<Det>(&format!("{prefix}_detached")),
        open_det: both::<OpenDet>(&format!("{prefix}_open_detached")),
    }
}

fn families() -> Vec<Sb> {
    vec![sb("crypto_secretbox"), sb("crypto_secretbox_xchacha20poly1305")]
}

// ---------------------------------------------------------------- primitives

/// `_easy` on both libraries; asserts agreement and returns the ciphertext.
fn easy(s: &Sb, m: &[u8], mlen: usize, n: &[u8], k: &[u8], label: &str) -> Vec<u8> {
    let mut cc = padded(mlen + MAC);
    let mut cr = padded(mlen + MAC);
    let rc = unsafe { (s.easy.0)(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { (s.easy.1)(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
    eqi(&format!("{label}: easy ret"), rc, rr);
    assert_eq!(rc, 0, "{label}: C easy must return 0");
    eqb(&format!("{label}: easy c"), &cc, &cr);
    check_pad(&format!("{label}: easy c (C)"), &cc, mlen + MAC);
    check_pad(&format!("{label}: easy c (Rust)"), &cr, mlen + MAC);
    cc.truncate(mlen + MAC);
    cc
}

fn open_easy(s: &Sb, c: &[u8], n: &[u8], k: &[u8], expect: c_int, label: &str) -> Vec<u8> {
    let clen = c.len();
    let mcap = if clen >= MAC { clen - MAC } else { 64 };
    let mut mc = poisoned(mcap);
    let mut mr = poisoned(mcap);
    let rc =
        unsafe { (s.open_easy.0)(mc.as_mut_ptr(), c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
    let rr =
        unsafe { (s.open_easy.1)(mr.as_mut_ptr(), c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
    eqi(&format!("{label}: open_easy ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C open_easy return");
    eqb(&format!("{label}: open_easy m"), &mc, &mr);
    check_pad(&format!("{label}: open_easy m (C)"), &mc, mcap);
    check_pad(&format!("{label}: open_easy m (Rust)"), &mr, mcap);
    if rc != 0 {
        // secretbox does *not* zero `m` on failure (unlike the AEADs)
        assert!(
            mc[..mcap].iter().all(|b| *b == 0xDD),
            "{label}: m touched on failure (C)"
        );
        assert!(
            mr[..mcap].iter().all(|b| *b == 0xDD),
            "{label}: m touched on failure (Rust)"
        );
    }
    mc.truncate(mcap);
    mc
}

fn detached(s: &Sb, m: &[u8], mlen: usize, n: &[u8], k: &[u8], label: &str) -> (Vec<u8>, Vec<u8>) {
    let mut cc = padded(mlen);
    let mut cr = padded(mlen);
    let mut ac = padded(MAC);
    let mut ar = padded(MAC);
    let rc = unsafe {
        (s.det.0)(
            cc.as_mut_ptr(),
            ac.as_mut_ptr(),
            m.as_ptr(),
            mlen as u64,
            n.as_ptr(),
            k.as_ptr(),
        )
    };
    let rr = unsafe {
        (s.det.1)(
            cr.as_mut_ptr(),
            ar.as_mut_ptr(),
            m.as_ptr(),
            mlen as u64,
            n.as_ptr(),
            k.as_ptr(),
        )
    };
    eqi(&format!("{label}: detached ret"), rc, rr);
    // errors_6.md 6.51 / 6.57: `_detached` has no checks at all, always 0.
    assert_eq!(rc, 0, "{label}: C detached must return 0");
    eqb(&format!("{label}: detached c"), &cc, &cr);
    eqb(&format!("{label}: detached mac"), &ac, &ar);
    check_pad(&format!("{label}: det c (C)"), &cc, mlen);
    check_pad(&format!("{label}: det c (Rust)"), &cr, mlen);
    check_pad(&format!("{label}: det mac (C)"), &ac, MAC);
    check_pad(&format!("{label}: det mac (Rust)"), &ar, MAC);
    cc.truncate(mlen);
    ac.truncate(MAC);
    (cc, ac)
}

fn open_detached(
    s: &Sb,
    c: &[u8],
    mac: &[u8],
    n: &[u8],
    k: &[u8],
    verify_only: bool,
    expect: c_int,
    label: &str,
) -> Vec<u8> {
    let clen = c.len();
    let mut mc = poisoned(clen);
    let mut mr = poisoned(clen);
    let (pc, pr) = if verify_only {
        (null_mut(), null_mut())
    } else {
        (mc.as_mut_ptr(), mr.as_mut_ptr())
    };
    let rc = unsafe {
        (s.open_det.0)(
            pc,
            c.as_ptr(),
            mac.as_ptr(),
            clen as u64,
            n.as_ptr(),
            k.as_ptr(),
        )
    };
    let rr = unsafe {
        (s.open_det.1)(
            pr,
            c.as_ptr(),
            mac.as_ptr(),
            clen as u64,
            n.as_ptr(),
            k.as_ptr(),
        )
    };
    eqi(&format!("{label}: open_detached ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C open_detached return");
    eqb(&format!("{label}: open_detached m"), &mc, &mr);
    check_pad(&format!("{label}: od m (C)"), &mc, clen);
    check_pad(&format!("{label}: od m (Rust)"), &mr, clen);
    if verify_only || rc != 0 {
        assert!(
            mc[..clen].iter().all(|b| *b == 0xDD),
            "{label}: m written (verify_only={verify_only}, rc={rc})"
        );
    }
    mc.truncate(clen);
    mc
}

// ============================================================== 6.78 / 6.89 / 6.93

#[test]
fn constant_getters() {
    let smax_minus_16 = usize::MAX - 16;
    for (sym, want) in [
        ("crypto_secretbox_keybytes", 32usize),
        ("crypto_secretbox_noncebytes", 24),
        ("crypto_secretbox_macbytes", 16),
        ("crypto_secretbox_zerobytes", 32),
        ("crypto_secretbox_boxzerobytes", 16),
        ("crypto_secretbox_messagebytes_max", smax_minus_16),
        ("crypto_secretbox_xsalsa20poly1305_keybytes", 32),
        ("crypto_secretbox_xsalsa20poly1305_noncebytes", 24),
        ("crypto_secretbox_xsalsa20poly1305_macbytes", 16),
        ("crypto_secretbox_xsalsa20poly1305_zerobytes", 32),
        ("crypto_secretbox_xsalsa20poly1305_boxzerobytes", 16),
        ("crypto_secretbox_xsalsa20poly1305_messagebytes_max", smax_minus_16),
        ("crypto_secretbox_xchacha20poly1305_keybytes", 32),
        ("crypto_secretbox_xchacha20poly1305_noncebytes", 24),
        ("crypto_secretbox_xchacha20poly1305_macbytes", 16),
        ("crypto_secretbox_xchacha20poly1305_messagebytes_max", smax_minus_16),
    ] {
        let (c, r) = both::<Getter>(sym);
        unsafe {
            let cv = c();
            let rv = r();
            assert_eq!(cv, want, "C {sym}");
            assert_eq!(rv, cv, "Rust {sym}");
        }
    }
    // the generic aliases must agree with the xsalsa20poly1305 primitives
    for (a, b) in [
        ("crypto_secretbox_keybytes", "crypto_secretbox_xsalsa20poly1305_keybytes"),
        ("crypto_secretbox_noncebytes", "crypto_secretbox_xsalsa20poly1305_noncebytes"),
        ("crypto_secretbox_macbytes", "crypto_secretbox_xsalsa20poly1305_macbytes"),
        ("crypto_secretbox_zerobytes", "crypto_secretbox_xsalsa20poly1305_zerobytes"),
        (
            "crypto_secretbox_boxzerobytes",
            "crypto_secretbox_xsalsa20poly1305_boxzerobytes",
        ),
        (
            "crypto_secretbox_messagebytes_max",
            "crypto_secretbox_xsalsa20poly1305_messagebytes_max",
        ),
    ] {
        let (ca, _) = both::<Getter>(a);
        let (cb, _) = both::<Getter>(b);
        unsafe { assert_eq!(ca(), cb(), "{a} != {b}") };
    }
    // _primitive()
    let (pc, pr) = both::<Primitive>("crypto_secretbox_primitive");
    unsafe {
        let sc = std::ffi::CStr::from_ptr(pc()).to_bytes().to_vec();
        let sr = std::ffi::CStr::from_ptr(pr()).to_bytes().to_vec();
        eqb("crypto_secretbox_primitive", &sc, &sr);
        assert_eq!(&sc, b"xsalsa20poly1305");
    }
    // row 6.93 / errors 6.67: the xchacha20poly1305 family has no
    // _zerobytes/_boxzerobytes/_keygen/_primitive and no NaCl-style variant.
    for absent in [
        "crypto_secretbox_xchacha20poly1305_zerobytes",
        "crypto_secretbox_xchacha20poly1305_boxzerobytes",
        "crypto_secretbox_xchacha20poly1305_keygen",
        "crypto_secretbox_xchacha20poly1305_primitive",
        "crypto_secretbox_xchacha20poly1305",
        "crypto_secretbox_xchacha20poly1305_open",
    ] {
        let mut nul: Vec<u8> = absent.as_bytes().to_vec();
        nul.push(0);
        unsafe {
            let in_c = c_lib()
                .get::<*const std::ffi::c_void>(&nul)
                .is_ok();
            let in_r = rust_lib()
                .get::<*const std::ffi::c_void>(&nul)
                .is_ok();
            assert!(!in_c, "C unexpectedly exports {absent}");
            assert_eq!(in_c, in_r, "{absent}: export mismatch (C {in_c}, Rust {in_r})");
        }
    }
}

// ============================================================== 6.79 / errors 6.82

#[test]
fn keygens() {
    for sym in [
        "crypto_secretbox_keygen",
        "crypto_secretbox_xsalsa20poly1305_keygen",
    ] {
        let (c, r) = both::<Keygen>(sym);
        for seed in [3u64, 17, 0xFEED] {
            rng_reseed(seed);
            let mut a = padded(32);
            let mut b = padded(32);
            unsafe {
                c(a.as_mut_ptr());
                r(b.as_mut_ptr());
            }
            eqb(sym, &a, &b);
            check_pad(sym, &a, 32);
            check_pad(sym, &b, 32);
            assert!(a[..32].iter().any(|x| *x != 0));
            let mut a2 = padded(32);
            let mut b2 = padded(32);
            unsafe {
                c(a2.as_mut_ptr());
                r(b2.as_mut_ptr());
            }
            eqb(sym, &a2, &b2);
            assert_ne!(&a[..32], &a2[..32], "{sym}: two calls identical");
        }
    }
}

// ============================================================== 6.80 / 6.81 / 6.94 / 6.95

#[test]
fn easy_round_trip() {
    let mut rng = Rng::new(0x6080);
    for s in families() {
        let lens: Vec<usize> = MLEN
            .iter()
            .copied()
            .chain(BIG_SECRETBOX.iter().copied())
            .chain(0..=300)
            .collect();
        for mlen in lens {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen + 1);
            let label = format!("{} easy mlen={mlen}", s.name);
            let c = easy(&s, &m, mlen, &n, &k, &label);
            assert_eq!(c.len(), mlen + MAC);
            let out = open_easy(&s, &c, &n, &k, 0, &label);
            eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
            // the leading MACBYTES of `c` are the MAC produced by `_detached`
            let (cd, mac) = detached(&s, &m, mlen, &n, &k, &label);
            eqb(&format!("{label}: easy mac == detached mac"), &c[..MAC], &mac);
            eqb(&format!("{label}: easy body == detached body"), &c[MAC..], &cd);
        }
    }
}

// ============================================================== 6.82 / 6.85 / 6.96 / 6.99

#[test]
fn detached_disjoint_buffers() {
    let mut rng = Rng::new(0x6082);
    for s in families() {
        let lens: Vec<usize> = MLEN
            .iter()
            .copied()
            .chain(BIG_SECRETBOX.iter().copied())
            .chain([0usize, 1, 31, 32, 33, 64, 4096])
            .collect();
        for mlen in lens {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen + 1);
            let label = format!("{} detached mlen={mlen}", s.name);
            let (c, mac) = detached(&s, &m, mlen, &n, &k, &label);
            let out = open_detached(&s, &c, &mac, &n, &k, false, 0, &label);
            eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
            // must equal the `_easy` framing split at MACBYTES
            let e = easy(&s, &m, mlen, &n, &k, &label);
            eqb(&format!("{label}: mac"), &e[..MAC], &mac);
            eqb(&format!("{label}: body"), &e[MAC..], &c);
        }
    }
}

// ============================================================== 6.83 / 6.97 / errors 6.55 / 6.61

#[test]
fn open_detached_verify_only() {
    let mut rng = Rng::new(0x6083);
    for s in families() {
        for &clen in MLEN.iter() {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(clen + 1);
            let label = format!("{} verify-only clen={clen}", s.name);
            let (c, mac) = detached(&s, &m, clen, &n, &k, &label);
            // valid MAC, m == NULL -> 0 and nothing written
            open_detached(&s, &c, &mac, &n, &k, true, 0, &label);
            // tampered MAC -> -1
            for pos in 0..MAC {
                let mut bad = mac.clone();
                bad[pos] ^= 0x80;
                open_detached(&s, &c, &bad, &n, &k, true, -1, &label);
                open_detached(&s, &c, &bad, &n, &k, false, -1, &label);
            }
            // tampered ciphertext -> -1
            for pos in 0..clen.min(8) {
                let mut bad = c.clone();
                bad[pos] ^= 0x01;
                open_detached(&s, &bad, &mac, &n, &k, true, -1, &label);
                open_detached(&s, &bad, &mac, &n, &k, false, -1, &label);
            }
            // wrong key / wrong nonce -> -1
            let mut k2 = k.clone();
            k2[0] ^= 0xff;
            open_detached(&s, &c, &mac, &n, &k2, true, -1, &label);
            let mut n2 = n.clone();
            n2[23] ^= 0xff;
            open_detached(&s, &c, &mac, &n2, &k, false, -1, &label);
        }
    }
}

// ============================================================== 6.84 / 6.98

#[test]
fn easy_in_place_and_overlap() {
    let mut rng = Rng::new(0x6084);
    for s in families() {
        for &mlen in [0usize, 1, 31, 32, 33, 64, 4096].iter() {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(mlen + 1);
            let label = format!("{} in-place mlen={mlen}", s.name);
            let reference = easy(&s, &m, mlen, &n, &k, &label);

            // (a) c == m: `_detached` sees c' = c + 16 and m = c, i.e. the
            //     memmove overlap branch for mlen > 16.
            let mut bc = padded(mlen + MAC);
            let mut br = padded(mlen + MAC);
            bc[..mlen].copy_from_slice(&m[..mlen]);
            br[..mlen].copy_from_slice(&m[..mlen]);
            let rc = unsafe {
                (s.easy.0)(
                    bc.as_mut_ptr(),
                    bc.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                (s.easy.1)(
                    br.as_mut_ptr(),
                    br.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            eqi(&format!("{label}: easy c==m ret"), rc, rr);
            eqb(&format!("{label}: easy c==m"), &bc, &br);
            eqb(
                &format!("{label}: easy c==m matches out-of-place"),
                &reference,
                &bc[..mlen + MAC],
            );
            check_pad(&format!("{label}: easy c==m (C)"), &bc, mlen + MAC);

            // (b) m == c + MACBYTES: the documented "no-copy" layout; the
            //     memmove branch is *not* taken (pointers are equal).
            let mut bc = padded(mlen + MAC);
            let mut br = padded(mlen + MAC);
            bc[MAC..MAC + mlen].copy_from_slice(&m[..mlen]);
            br[MAC..MAC + mlen].copy_from_slice(&m[..mlen]);
            let rc = unsafe {
                (s.easy.0)(
                    bc.as_mut_ptr(),
                    bc.as_ptr().add(MAC),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                (s.easy.1)(
                    br.as_mut_ptr(),
                    br.as_ptr().add(MAC),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            eqi(&format!("{label}: easy m==c+16 ret"), rc, rr);
            eqb(&format!("{label}: easy m==c+16"), &bc, &br);
            eqb(
                &format!("{label}: easy m==c+16 matches out-of-place"),
                &reference,
                &bc[..mlen + MAC],
            );

            // (c) partially overlapping `_detached` buffers, both directions
            for shift in [1usize, 8, 15] {
                if mlen <= shift {
                    continue;
                }
                // c > m, c - m == shift
                let mut bc = padded(mlen + shift + MAC);
                let mut br = padded(mlen + shift + MAC);
                bc[..mlen].copy_from_slice(&m[..mlen]);
                br[..mlen].copy_from_slice(&m[..mlen]);
                let mut ac = padded(MAC);
                let mut ar = padded(MAC);
                let rc = unsafe {
                    (s.det.0)(
                        bc.as_mut_ptr().add(shift),
                        ac.as_mut_ptr(),
                        bc.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rr = unsafe {
                    (s.det.1)(
                        br.as_mut_ptr().add(shift),
                        ar.as_mut_ptr(),
                        br.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                eqi(&format!("{label}: overlap c>m shift={shift} ret"), rc, rr);
                eqb(&format!("{label}: overlap c>m shift={shift}"), &bc, &br);
                eqb(&format!("{label}: overlap mac shift={shift}"), &ac, &ar);
                eqb(
                    &format!("{label}: overlap c>m result shift={shift}"),
                    &reference[MAC..],
                    &bc[shift..shift + mlen],
                );
                eqb(
                    &format!("{label}: overlap c>m mac shift={shift}"),
                    &reference[..MAC],
                    &ac[..MAC],
                );

                // m > c, m - c == shift
                let mut bc = padded(mlen + shift + MAC);
                let mut br = padded(mlen + shift + MAC);
                bc[shift..shift + mlen].copy_from_slice(&m[..mlen]);
                br[shift..shift + mlen].copy_from_slice(&m[..mlen]);
                let mut ac = padded(MAC);
                let mut ar = padded(MAC);
                let rc = unsafe {
                    (s.det.0)(
                        bc.as_mut_ptr(),
                        ac.as_mut_ptr(),
                        bc.as_ptr().add(shift),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                let rr = unsafe {
                    (s.det.1)(
                        br.as_mut_ptr(),
                        ar.as_mut_ptr(),
                        br.as_ptr().add(shift),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    )
                };
                eqi(&format!("{label}: overlap m>c shift={shift} ret"), rc, rr);
                eqb(&format!("{label}: overlap m>c shift={shift}"), &bc, &br);
                eqb(&format!("{label}: overlap m>c mac shift={shift}"), &ac, &ar);
                eqb(
                    &format!("{label}: overlap m>c result shift={shift}"),
                    &reference[MAC..],
                    &bc[..mlen],
                );
            }

            // (d) open in place: m == c (hits the memmove branch in
            //     `_open_detached` for clen > 16)
            let clen = mlen + MAC;
            let mut bc = padded(clen);
            let mut br = padded(clen);
            bc[..clen].copy_from_slice(&reference);
            br[..clen].copy_from_slice(&reference);
            let rc = unsafe {
                (s.open_easy.0)(
                    bc.as_mut_ptr(),
                    bc.as_ptr(),
                    clen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                (s.open_easy.1)(
                    br.as_mut_ptr(),
                    br.as_ptr(),
                    clen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            eqi(&format!("{label}: open in-place ret"), rc, rr);
            assert_eq!(rc, 0);
            eqb(&format!("{label}: open in-place"), &bc, &br);
            eqb(
                &format!("{label}: open in-place plaintext"),
                &m[..mlen],
                &bc[..mlen],
            );
            check_pad(&format!("{label}: open in-place (C)"), &bc, clen);

            // (e) open with m == c + MACBYTES (no memmove; pointers equal)
            let mut bc = padded(clen);
            let mut br = padded(clen);
            bc[..clen].copy_from_slice(&reference);
            br[..clen].copy_from_slice(&reference);
            let rc = unsafe {
                (s.open_det.0)(
                    bc.as_mut_ptr().add(MAC),
                    bc.as_ptr().add(MAC),
                    bc.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            let rr = unsafe {
                (s.open_det.1)(
                    br.as_mut_ptr().add(MAC),
                    br.as_ptr().add(MAC),
                    br.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                )
            };
            eqi(&format!("{label}: open_detached in-place ret"), rc, rr);
            assert_eq!(rc, 0);
            eqb(&format!("{label}: open_detached in-place"), &bc, &br);
            eqb(
                &format!("{label}: open_detached in-place plaintext"),
                &m[..mlen],
                &bc[MAC..MAC + mlen],
            );
        }
    }
}

// ============================================================== 6.86 / 6.87 / 6.88

struct Nacl {
    seal: (Symbol<'static, Easy>, Symbol<'static, Easy>),
    open: (Symbol<'static, Easy>, Symbol<'static, Easy>),
}

fn nacl(prefix: &str) -> Nacl {
    if prefix.is_empty() {
        Nacl {
            seal: both::<Easy>("crypto_secretbox"),
            open: both::<Easy>("crypto_secretbox_open"),
        }
    } else {
        Nacl {
            seal: both::<Easy>("crypto_secretbox_xsalsa20poly1305"),
            open: both::<Easy>("crypto_secretbox_xsalsa20poly1305_open"),
        }
    }
}

fn nacl_seal(nc: &Nacl, m: &[u8], mlen: usize, n: &[u8], k: &[u8], expect: c_int, label: &str) -> Vec<u8> {
    let mut cc = poisoned(mlen.max(32));
    let mut cr = poisoned(mlen.max(32));
    let rc = unsafe { (nc.seal.0)(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { (nc.seal.1)(cr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
    eqi(&format!("{label}: secretbox ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C secretbox return");
    eqb(&format!("{label}: secretbox c"), &cc, &cr);
    check_pad(&format!("{label}: secretbox c (C)"), &cc, mlen.max(32));
    check_pad(&format!("{label}: secretbox c (Rust)"), &cr, mlen.max(32));
    if rc != 0 {
        assert!(
            cc[..mlen.max(32)].iter().all(|b| *b == 0xDD),
            "{label}: c touched on rejection"
        );
    } else {
        assert!(
            cc[..16].iter().all(|b| *b == 0),
            "{label}: c[0..16] must be forced to zero"
        );
    }
    cc.truncate(mlen.max(32));
    cc
}

fn nacl_open(nc: &Nacl, c: &[u8], n: &[u8], k: &[u8], expect: c_int, label: &str) -> Vec<u8> {
    let clen = c.len();
    let mut mc = poisoned(clen.max(32));
    let mut mr = poisoned(clen.max(32));
    let rc = unsafe { (nc.open.0)(mc.as_mut_ptr(), c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { (nc.open.1)(mr.as_mut_ptr(), c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
    eqi(&format!("{label}: secretbox_open ret"), rc, rr);
    assert_eq!(rc, expect, "{label}: C secretbox_open return");
    eqb(&format!("{label}: secretbox_open m"), &mc, &mr);
    check_pad(&format!("{label}: open m (C)"), &mc, clen.max(32));
    if rc != 0 {
        assert!(
            mc[..clen.max(32)].iter().all(|b| *b == 0xDD),
            "{label}: m touched on failure (not zeroed)"
        );
    } else {
        assert!(
            mc[..32].iter().all(|b| *b == 0),
            "{label}: m[0..32] must be re-zeroed"
        );
    }
    mc.truncate(clen.max(32));
    mc
}

#[test]
fn nacl_style_round_trip() {
    let mut rng = Rng::new(0x6086);
    let generic = nacl("");
    let prim = nacl("xsalsa20poly1305");
    let lens: Vec<usize> = MLEN
        .iter()
        .copied()
        .chain([0usize, 1, 32, 63, 64, 65, 4096, 131073])
        .collect();
    for plen in lens {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut m = vec![0u8; 32];
        m.extend_from_slice(&rng.bytes(plen + 1)[..plen]);
        let mlen = 32 + plen;
        let label = format!("nacl plen={plen}");
        let c = nacl_seal(&generic, &m, mlen, &n, &k, 0, &label);
        // c[0..16] == 0 (BOXZEROBYTES), MAC at c[16..32]
        assert!(c[..16].iter().all(|b| *b == 0));
        let out = nacl_open(&generic, &c, &n, &k, 0, &label);
        assert!(out[..32].iter().all(|b| *b == 0));
        eqb(&format!("{label}: plaintext"), &m[32..], &out[32..]);
        // row 6.88: the primitive entry points must be byte-identical
        let c2 = nacl_seal(&prim, &m, mlen, &n, &k, 0, &label);
        eqb(&format!("{label}: primitive == wrapper"), &c, &c2);
        let out2 = nacl_open(&prim, &c, &n, &k, 0, &label);
        eqb(&format!("{label}: primitive open == wrapper"), &out, &out2);
    }
}

// ============================================================== 6.90

#[test]
fn easy_matches_nacl_framing() {
    let mut rng = Rng::new(0x6090);
    let g = nacl("");
    let s = sb("crypto_secretbox");
    for &plen in [0usize, 1, 15, 16, 17, 31, 32, 33, 64, 65, 1024].iter() {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let pt = rng.bytes(plen + 1);
        let mut padded_m = vec![0u8; 32];
        padded_m.extend_from_slice(&pt[..plen]);
        let label = format!("easy-vs-nacl plen={plen}");
        let e = easy(&s, &pt, plen, &n, &k, &label);
        let c = nacl_seal(&g, &padded_m, 32 + plen, &n, &k, 0, &label);
        // easy output == secretbox output shifted by BOXZEROBYTES (16)
        eqb(&format!("{label}: framing"), &e, &c[16..]);
    }
}

// ============================================================== 6.91 / 6.100

#[test]
fn corner_keys_and_nonces() {
    let mut rng = Rng::new(0x6091);
    let g = nacl("");
    let keys: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32], rng.bytes(32)];
    let mut nonces: Vec<Vec<u8>> = vec![vec![0u8; 24], vec![0xffu8; 24], rng.bytes(24)];
    // non-zero high half only: n[0..16] (hsalsa/hchacha input) all zero,
    // n[16..24] (stream nonce) non-zero, and the mirror image.
    let mut hi = vec![0u8; 24];
    for b in hi[16..].iter_mut() {
        *b = 0x9C;
    }
    nonces.push(hi);
    let mut lo = vec![0u8; 24];
    for b in lo[..16].iter_mut() {
        *b = 0x3E;
    }
    nonces.push(lo);
    for s in families() {
        for k in keys.iter() {
            for n in nonces.iter() {
                for &mlen in [0usize, 1, 32, 33].iter() {
                    let m = rng.bytes(mlen + 1);
                    let label = format!("{} corner k={:#02x} n={:#02x} mlen={mlen}", s.name, k[0], n[0]);
                    let c = easy(&s, &m, mlen, n, k, &label);
                    let out = open_easy(&s, &c, n, k, 0, &label);
                    eqb(&format!("{label}: plaintext"), &m[..mlen], &out);
                    let (cd, mac) = detached(&s, &m, mlen, n, k, &label);
                    eqb(&format!("{label}: detached"), &c[MAC..], &cd);
                    eqb(&format!("{label}: mac"), &c[..MAC], &mac);
                }
                // NaCl-style with the same corner values
                for &plen in [0usize, 1, 33].iter() {
                    let mut m = vec![0u8; 32];
                    m.extend_from_slice(&rng.bytes(plen + 1)[..plen]);
                    let label = format!("nacl corner k={:#02x} n={:#02x} plen={plen}", k[0], n[0]);
                    let c = nacl_seal(&g, &m, 32 + plen, n, k, 0, &label);
                    let out = nacl_open(&g, &c, n, k, 0, &label);
                    eqb(&format!("{label}: plaintext"), &m[32..], &out[32..]);
                }
            }
        }
    }
}

// ============================================================== 6.92 / 6.101 (pinned)

/// Independent structural check of the xsalsa20poly1305 secretbox
/// construction, assembled out of the already-verified low-level primitives of
/// the *other* library.  Stronger than a single KAT: it pins the exact
/// composition (hsalsa20 subkey, 32-byte zero-prefixed block0, Poly1305 key
/// from block0[0..32], salsa20 keystream restart at ic = 1).
#[test]
fn xsalsa20poly1305_construction_pinned() {
    type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
    type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
    type XorIc = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
    type Ota = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
    let (hs, _) = both::<Core>("crypto_core_hsalsa20");
    let (sx, _) = both::<Xor>("crypto_stream_salsa20_xor");
    let (sxic, _) = both::<XorIc>("crypto_stream_salsa20_xor_ic");
    let (ota, _) = both::<Ota>("crypto_onetimeauth_poly1305");
    let s = sb("crypto_secretbox");

    for &mlen in [0usize, 1, 16, 31, 32, 33, 64, 65, 200].iter() {
        // fully deterministic inputs
        let k: Vec<u8> = (0..32).map(|i| (i as u8).wrapping_mul(11).wrapping_add(3)).collect();
        let n: Vec<u8> = (0..24).map(|i| 0x60u8 + i as u8).collect();
        let m: Vec<u8> = (0..mlen + 1).map(|i| (i * 37 + 5) as u8).collect();

        let mut subkey = [0u8; 32];
        unsafe { hs(subkey.as_mut_ptr(), n.as_ptr(), k.as_ptr(), null()) };
        let mlen0 = mlen.min(32);
        let mut block0 = [0u8; 64];
        block0[32..32 + mlen0].copy_from_slice(&m[..mlen0]);
        let mut b0 = [0u8; 64];
        unsafe {
            sx(
                b0.as_mut_ptr(),
                block0.as_ptr(),
                64,
                n.as_ptr().add(16),
                subkey.as_ptr(),
            )
        };
        let mut ct = vec![0u8; mlen];
        ct[..mlen0].copy_from_slice(&b0[32..32 + mlen0]);
        if mlen > mlen0 {
            unsafe {
                sxic(
                    ct.as_mut_ptr().add(mlen0),
                    m.as_ptr().add(mlen0),
                    (mlen - mlen0) as u64,
                    n.as_ptr().add(16),
                    1,
                    subkey.as_ptr(),
                )
            };
        }
        let mut mac = [0u8; 16];
        unsafe { ota(mac.as_mut_ptr(), ct.as_ptr(), mlen as u64, b0.as_ptr()) };

        let label = format!("xsalsa20poly1305 construction mlen={mlen}");
        let e = easy(&s, &m, mlen, &n, &k, &label);
        eqb(&format!("{label}: mac"), &mac, &e[..MAC]);
        eqb(&format!("{label}: body"), &ct, &e[MAC..]);
    }
}

#[test]
fn xchacha20poly1305_construction_pinned() {
    type Core = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
    type Xor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
    type XorIc = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
    type Ota = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
    let (hc, _) = both::<Core>("crypto_core_hchacha20");
    let (cx, _) = both::<Xor>("crypto_stream_chacha20_xor");
    let (cxic, _) = both::<XorIc>("crypto_stream_chacha20_xor_ic");
    let (ota, _) = both::<Ota>("crypto_onetimeauth_poly1305");
    let s = sb("crypto_secretbox_xchacha20poly1305");
    let sx = sb("crypto_secretbox");

    for &mlen in [0usize, 1, 16, 31, 32, 33, 64, 65, 200].iter() {
        let k: Vec<u8> = (0..32).map(|i| (i as u8).wrapping_mul(13).wrapping_add(1)).collect();
        let n: Vec<u8> = (0..24).map(|i| 0x30u8 + i as u8).collect();
        let m: Vec<u8> = (0..mlen + 1).map(|i| (i * 41 + 9) as u8).collect();

        let mut subkey = [0u8; 32];
        unsafe { hc(subkey.as_mut_ptr(), n.as_ptr(), k.as_ptr(), null()) };
        let mlen0 = mlen.min(32);
        let mut block0 = [0u8; 64];
        block0[32..32 + mlen0].copy_from_slice(&m[..mlen0]);
        // NB: `_detached` XORs only mlen0 + 32 bytes of block0 here.
        let mut b0 = block0;
        unsafe {
            cx(
                b0.as_mut_ptr(),
                block0.as_ptr(),
                (mlen0 + 32) as u64,
                n.as_ptr().add(16),
                subkey.as_ptr(),
            )
        };
        let mut ct = vec![0u8; mlen];
        ct[..mlen0].copy_from_slice(&b0[32..32 + mlen0]);
        if mlen > mlen0 {
            unsafe {
                cxic(
                    ct.as_mut_ptr().add(mlen0),
                    m.as_ptr().add(mlen0),
                    (mlen - mlen0) as u64,
                    n.as_ptr().add(16),
                    1,
                    subkey.as_ptr(),
                )
            };
        }
        let mut mac = [0u8; 16];
        unsafe { ota(mac.as_mut_ptr(), ct.as_ptr(), mlen as u64, b0.as_ptr()) };

        let label = format!("xchacha20poly1305 construction mlen={mlen}");
        let e = easy(&s, &m, mlen, &n, &k, &label);
        eqb(&format!("{label}: mac"), &mac, &e[..MAC]);
        eqb(&format!("{label}: body"), &ct, &e[MAC..]);
        // row 6.101: must differ from the xsalsa20poly1305 default
        let e2 = easy(&sx, &m, mlen, &n, &k, &label);
        assert_ne!(e, e2, "{label}: the two primitive families collapsed");
    }
}

// ============================================================== 6.102

#[test]
fn cross_family_open_fails() {
    let mut rng = Rng::new(0x6102);
    let a = sb("crypto_secretbox");
    let b = sb("crypto_secretbox_xchacha20poly1305");
    for &mlen in [0usize, 1, 32, 33].iter() {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = rng.bytes(mlen + 1);
        let label = format!("cross-family mlen={mlen}");
        let ca = easy(&a, &m, mlen, &n, &k, &label);
        let cb = easy(&b, &m, mlen, &n, &k, &label);
        open_easy(&b, &ca, &n, &k, -1, &label);
        open_easy(&a, &cb, &n, &k, -1, &label);
    }
}

// ============================================================ ERROR SURFACE

/// errors_6.md 6.52 / 6.53 / 6.58 / 6.59
#[test]
fn open_easy_short_and_bad_mac() {
    let mut rng = Rng::new(0x6E52);
    for s in families() {
        // clen < MACBYTES -> -1 before any crypto
        for clen in 0..MAC {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let c = rng.bytes(clen.max(1));
            open_easy(
                &s,
                &c[..clen],
                &n,
                &k,
                -1,
                &format!("{} open_easy clen={clen}", s.name),
            );
        }
        // clen == MACBYTES: a valid empty box opens, random data does not
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = rng.bytes(1);
        let label = format!("{} open_easy boundary", s.name);
        let c = easy(&s, &m, 0, &n, &k, &label);
        assert_eq!(c.len(), MAC);
        open_easy(&s, &c, &n, &k, 0, &label);
        let junk = rng.bytes(MAC);
        open_easy(&s, &junk, &n, &k, -1, &label);
        // MACBYTES + 1: one plaintext byte
        let m1 = rng.bytes(2);
        let c1 = easy(&s, &m1, 1, &n, &k, &label);
        assert_eq!(c1.len(), MAC + 1);
        open_easy(&s, &c1, &n, &k, 0, &label);
        // tamper in every byte position
        for &mlen in [0usize, 1, 17, 32, 33].iter() {
            let m = rng.bytes(mlen + 1);
            let c = easy(&s, &m, mlen, &n, &k, &label);
            for pos in 0..c.len() {
                let mut bad = c.clone();
                bad[pos] ^= 0x40;
                open_easy(
                    &s,
                    &bad,
                    &n,
                    &k,
                    -1,
                    &format!("{label} mlen={mlen} pos={pos}"),
                );
            }
            // wrong key / wrong nonce
            let mut k2 = k.clone();
            k2[31] ^= 0x01;
            open_easy(&s, &c, &n, &k2, -1, &label);
            let mut n2 = n.clone();
            n2[0] ^= 0x01;
            open_easy(&s, &c, &n2, &k, -1, &label);
        }
    }
}

/// errors_6.md 6.62 / 6.64 — the NaCl-style API rejects `mlen < ZEROBYTES`.
#[test]
fn nacl_short_inputs_rejected() {
    let mut rng = Rng::new(0x6E62);
    for prefix in ["", "xsalsa20poly1305"] {
        let g = nacl(prefix);
        for len in [0usize, 1, 16, 17, 31] {
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = rng.bytes(32);
            let label = format!("nacl({prefix}) len={len}");
            nacl_seal(&g, &m, len, &n, &k, -1, &label);
            let c = rng.bytes(32);
            nacl_open(&g, &c[..len], &n, &k, -1, &label);
        }
        // exactly 32 is accepted
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = vec![0u8; 32];
        let label = format!("nacl({prefix}) len=32");
        let c = nacl_seal(&g, &m, 32, &n, &k, 0, &label);
        nacl_open(&g, &c, &n, &k, 0, &label);
    }
}

/// errors_6.md 6.63 — non-zero `m[0..32]` is silently accepted but the box is
/// unopenable.
#[test]
fn nacl_nonzero_padding_is_accepted_but_unopenable() {
    let mut rng = Rng::new(0x6E63);
    let g = nacl("");
    for &plen in [0usize, 1, 32, 65].iter() {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut m = rng.bytes(32 + plen + 1);
        m.truncate(32 + plen);
        // make sure the padding really is non-zero
        m[0] |= 1;
        let label = format!("nacl nonzero-pad plen={plen}");
        let c = nacl_seal(&g, &m, 32 + plen, &n, &k, 0, &label);
        // silently accepted...
        assert!(c[..16].iter().all(|b| *b == 0));
        // ...but not openable
        nacl_open(&g, &c, &n, &k, -1, &label);
    }
}

/// errors_6.md 6.65 / 6.66 — MAC failures, and the fact that `c[0..16]` is not
/// validated (garbage there still opens fine).
#[test]
fn nacl_mac_and_boxzero_handling() {
    let mut rng = Rng::new(0x6E65);
    let g = nacl("");
    for &plen in [0usize, 1, 17, 32, 33].iter() {
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let mut m = vec![0u8; 32];
        m.extend_from_slice(&rng.bytes(plen + 1)[..plen]);
        let label = format!("nacl mac plen={plen}");
        let c = nacl_seal(&g, &m, 32 + plen, &n, &k, 0, &label);
        let good = nacl_open(&g, &c, &n, &k, 0, &label);

        // row 6.66: garbage in the BOXZEROBYTES prefix still opens, with an
        // identical result (those bytes are neither MAC-covered nor returned).
        let mut c2 = c.clone();
        for (i, b) in c2[..16].iter_mut().enumerate() {
            *b = 0xA0u8.wrapping_add(i as u8);
        }
        let out2 = nacl_open(&g, &c2, &n, &k, 0, &format!("{label} garbage-pad"));
        eqb(&format!("{label}: garbage-pad result"), &good, &out2);

        // row 6.65: MAC / ciphertext tampering, wrong key, wrong nonce
        for pos in 16..c.len() {
            let mut bad = c.clone();
            bad[pos] ^= 0x08;
            nacl_open(&g, &bad, &n, &k, -1, &format!("{label} pos={pos}"));
        }
        let mut k2 = k.clone();
        k2[5] ^= 0xff;
        nacl_open(&g, &c, &n, &k2, -1, &label);
        let mut n2 = n.clone();
        n2[20] ^= 0xff;
        nacl_open(&g, &c, &n2, &k, -1, &label);
    }
}

/// errors_6.md 6.50 / 6.56 — `_easy` with `mlen > MESSAGEBYTES_MAX` aborts via
/// `sodium_misuse()`.
#[test]
fn easy_messagebytes_max_aborts() {
    for name in [
        "crypto_secretbox_easy",
        "crypto_secretbox_xchacha20poly1305_easy",
    ] {
        let (c, r) = both::<Easy>(name);
        let huge = (usize::MAX - 16) as u64 + 1;
        eq_abort(
            &format!("{name} mlen > MESSAGEBYTES_MAX"),
            move || unsafe {
                let k = [0u8; 32];
                let n = [0u8; 24];
                let mut out = [0u8; 64];
                c(out.as_mut_ptr(), out.as_ptr(), huge, n.as_ptr(), k.as_ptr());
            },
            move || unsafe {
                let k = [0u8; 32];
                let n = [0u8; 24];
                let mut out = [0u8; 64];
                r(out.as_mut_ptr(), out.as_ptr(), huge, n.as_ptr(), k.as_ptr());
            },
        );
    }
}
