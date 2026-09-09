//! Phase C — error-path differential tests, ERRORS.md groups 1 & 4
//! (sodium/, randombytes, verify, shorthash, onetimeauth, ipcrypt; pwhash,
//!  generichash, auth, kdf, hash, xof)
//!
//! Every row asserts the SAME error code / sentinel AND the SAME errno.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

const EPERM: i32 = 1;
const ENOENT: i32 = 2;
const ENOMEM: i32 = 12;
const EINVAL: i32 = 22;
const EFBIG: i32 = 27;
const ENOSYS: i32 = 38;
const ERANGE: i32 = 34;

/// Compare (ret, errno) from a C call and a Rust call.
///
/// A macro rather than a function so both calls are sequenced statements and
/// may therefore both take `&mut` of the same output buffer.
macro_rules! cmp2 {
    ($ctx:expr, $cexpr:expr, $rexpr:expr) => {{
        let ctx: &str = &$ctx;
        set_errno(0);
        let a: i32 = unsafe { $cexpr };
        let ea = errno();
        set_errno(0);
        let b: i32 = unsafe { $rexpr };
        let eb = errno();
        assert_eq!(a, b, "{}: return differs (C={} Rust={})", ctx, a, b);
        assert_eq!(ea, eb, "{}: errno differs (C={} Rust={})", ctx, ea, eb);
        a
    }};
}

// ================================================ rows 1-5: ENOSYS mlock/mprotect

#[test]
fn rows1_5_mlock_mprotect_enosys() {
    let mut b = vec![0u8; 4096];
    for name in ["sodium_mlock", "sodium_munlock"] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, Sz) -> c_int>(name);
        let ret = cmp2!(name, c(b.as_mut_ptr(), 64), r(b.as_mut_ptr(), 64));
        assert_eq!(ret, -1, "{name} must fail in this build");
        set_errno(0);
        let _ = unsafe { c(b.as_mut_ptr(), 64) };
        assert_eq!(errno(), ENOSYS, "{name} errno must be ENOSYS");
    }
    // mprotect_* need a sodium_malloc'ed pointer, but the fallback ignores it
    let (mc, mr) = pair::<unsafe extern "C" fn(Sz) -> *mut u8>("sodium_malloc");
    let (fc, fr) = pair::<unsafe extern "C" fn(*mut u8)>("sodium_free");
    unsafe {
        let pc = mc(64);
        let pr = mr(64);
        for name in [
            "sodium_mprotect_noaccess",
            "sodium_mprotect_readonly",
            "sodium_mprotect_readwrite",
        ] {
            let (c, r) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(name);
            let ret = cmp2!(name, c(pc), r(pr));
            assert_eq!(ret, -1, "{name} must fail in this build");
            set_errno(0);
            let _ = c(pc);
            assert_eq!(errno(), ENOSYS, "{name} errno");
        }
        fc(pc);
        fr(pr);
    }
}

// ============================================== row 6: allocarray overflow

#[test]
fn row6_allocarray_overflow() {
    let (c, r) = pair::<unsafe extern "C" fn(Sz, Sz) -> *mut u8>("sodium_allocarray");
    for (count, size) in [
        (2usize, usize::MAX / 2),
        (usize::MAX, 2),
        (usize::MAX, usize::MAX),
        (1 << 40, 1 << 40),
        (3, usize::MAX / 3),
    ] {
        set_errno(0);
        let pc = unsafe { c(count, size) };
        let ec = errno();
        set_errno(0);
        let pr = unsafe { r(count, size) };
        let er = errno();
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "sodium_allocarray({count},{size}): NULL-ness differs"
        );
        assert!(pc.is_null(), "expected NULL for overflowing allocarray");
        assert_eq!(ec, er, "sodium_allocarray errno differs (C={ec} Rust={er})");
        assert_eq!(ec, ENOMEM, "sodium_allocarray errno must be ENOMEM");
    }
    // count == 0 is NOT an overflow: must succeed identically
    let (fc, fr) = pair::<unsafe extern "C" fn(*mut u8)>("sodium_free");
    unsafe {
        let pc = c(0, usize::MAX);
        let pr = r(0, usize::MAX);
        assert_eq!(pc.is_null(), pr.is_null(), "allocarray(0, MAX)");
        fc(pc);
        fr(pr);
    }
}

// ======================================== rows 10-15: sodium_pad / sodium_unpad

#[test]
fn rows10_11_13_15_pad_unpad_errors() {
    let (pc, pr) = pair::<unsafe extern "C" fn(*mut Sz, *mut u8, Sz, Sz, Sz) -> c_int>("sodium_pad");
    let (uc, ur) =
        pair::<unsafe extern "C" fn(*mut Sz, *const u8, Sz, Sz) -> c_int>("sodium_unpad");
    let mut b = vec![0u8; 512];
    let mut lc: Sz = 0xDEAD;
    let mut lr: Sz = 0xDEAD;

    // row 10: blocksize == 0
    for ul in [0usize, 1, 16] {
        let ret = cmp2!(
            "sodium_pad blocksize=0",
            pc(&mut lc, b.as_mut_ptr(), ul, 0, 512),
            pr(&mut lr, b.as_mut_ptr(), ul, 0, 512)
        );
        assert_eq!(ret, -1);
    }
    // row 11: xpadded_len >= max_buflen (max_buflen too small, incl. 0)
    for (ul, bs, maxl) in [
        (0usize, 16usize, 0usize),
        (0, 16, 1),
        (0, 16, 15),
        (16, 16, 16),
        (16, 16, 31),
        (1, 1, 1),
        (100, 7, 100),
    ] {
        let ret = cmp2!(&format!("sodium_pad too small ul={ul} bs={bs} maxl={maxl}"), pc(&mut lc, b.as_mut_ptr(), ul, bs, maxl), pr(&mut lr, b.as_mut_ptr(), ul, bs, maxl));
        assert_eq!(ret, -1, "expected -1 for ul={ul} bs={bs} maxl={maxl}");
    }
    // row 13/14: unpad blocksize == 0 or padded_buflen < blocksize
    for (plen, bs) in [
        (0usize, 0usize),
        (16, 0),
        (0, 16),
        (1, 16),
        (15, 16),
        (16, 17),
        (5, 100),
    ] {
        let ret = cmp2!(&format!("sodium_unpad plen={plen} bs={bs}"), uc(&mut lc, b.as_ptr(), plen, bs), ur(&mut lr, b.as_ptr(), plen, bs));
        assert_eq!(ret, -1, "expected -1 for plen={plen} bs={bs}");
    }
    // row 15: no 0x80 barrier in the final block
    let mut rng = Rng::seeded();
    for bs in [1usize, 2, 8, 16, 17, 64] {
        for plen in [bs, bs * 2, bs * 3] {
            // all-zero padding: no barrier
            let z = vec![0u8; plen + 8];
            let ret = cmp2!(&format!("sodium_unpad no barrier bs={bs} plen={plen}"), uc(&mut lc, z.as_ptr(), plen, bs), ur(&mut lr, z.as_ptr(), plen, bs));
            assert_eq!(ret, -1);
            assert_eq!(lc, lr, "unpad out param on failure");
            // random content: whatever the C decides, Rust must match
            for _ in 0..50 {
                let v = rng.bytes(plen + 8);
                let mut xc: Sz = 0xDEAD;
                let mut xr: Sz = 0xDEAD;
                cmp2!(&format!("sodium_unpad random bs={bs} plen={plen}"), uc(&mut xc, v.as_ptr(), plen, bs), ur(&mut xr, v.as_ptr(), plen, bs));
                assert_eq!(xc, xr, "unpad out param");
            }
        }
    }
}

// ================================= rows 18-20: sodium_hex2bin rejections

#[test]
fn rows18_20_hex2bin_errors() {
    type F = unsafe extern "C" fn(
        *mut u8,
        Sz,
        *const c_char,
        Sz,
        *const c_char,
        *mut Sz,
        *mut *const c_char,
    ) -> c_int;
    let (c, r) = pair::<F>("sodium_hex2bin");
    let cases: &[(&str, usize)] = &[
        // (hex, bin_maxlen) — ERANGE cases
        ("00", 0),
        ("0011", 1),
        ("001122", 2),
        ("00112233445566778899", 4),
        // odd nibble count -> EINVAL
        ("0", 8),
        ("000", 8),
        ("abcde", 8),
        // trailing junk with hex_end == NULL -> EINVAL
        ("00zz", 8),
        ("00 ", 8),
        ("gg", 8),
        ("!!", 8),
        ("00:11", 8),
        // empty is fine
        ("", 8),
    ];
    for (hex_s, maxl) in cases {
        let hb = hex_s.as_bytes();
        for use_ignore in [false, true] {
            let ig: &[u8] = b"\0";
            let igp = if use_ignore {
                ig.as_ptr() as *const c_char
            } else {
                std::ptr::null()
            };
            let mut ob = buf((*maxl).max(1) + 8);
            let mut or = buf((*maxl).max(1) + 8);
            let mut bc: Sz = 0xDEAD;
            let mut br: Sz = 0xDEAD;
            let ctx = format!("hex2bin {hex_s:?} maxl={maxl} ig={use_ignore}");
            cmp2!(
                &ctx,
                c(ob.as_mut_ptr(), *maxl, hb.as_ptr() as *const c_char, hb.len(), igp, &mut bc, std::ptr::null_mut()),
                r(or.as_mut_ptr(), *maxl, hb.as_ptr() as *const c_char, hb.len(), igp, &mut br, std::ptr::null_mut())
            );
            same_bytes(&ctx, &ob, &or);
            assert_eq!(bc, br, "{ctx}: bin_len differs");
        }
    }
}

// ======================== rows 17, 21, 24: sodium_misuse paths (fork + SIGABRT)

/// Run one library's function in a forked child and report the outcome.
fn forked_c(f: impl FnOnce()) -> Outcome {
    run_forked(f)
}

#[test]
fn row17_bin2hex_output_too_small_aborts() {
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz) -> *mut c_char,
    >("sodium_bin2hex");
    let bin = vec![0xABu8; 8];
    // hex_maxlen <= bin_len * 2 -> sodium_misuse() -> abort()
    for (maxl, blen) in [(0usize, 1usize), (1, 1), (2, 1), (16, 8), (15, 8)] {
        let oc = forked_c(|| {
            let mut o = vec![0u8; 64];
            unsafe { c(o.as_mut_ptr() as *mut c_char, maxl, bin.as_ptr(), blen) };
        });
        let or = forked_c(|| {
            let mut o = vec![0u8; 64];
            unsafe { r(o.as_mut_ptr() as *mut c_char, maxl, bin.as_ptr(), blen) };
        });
        assert_eq!(oc, or, "bin2hex maxl={maxl} blen={blen}: outcome differs");
        assert_eq!(
            oc,
            Outcome::Signaled(libc::SIGABRT),
            "bin2hex maxl={maxl} blen={blen} must abort"
        );
    }
}

#[test]
fn row21_base64_bad_variant_aborts() {
    // Any variant with (v & ~0x6) != 0x1 must sodium_misuse() -> abort().
    let bad: &[c_int] = &[0, 2, 4, 6, 8, 9, -1, i32::MIN, i32::MAX, 100, 0x10];
    let (elc, elr) = pair::<unsafe extern "C" fn(Sz, c_int) -> Sz>("sodium_base64_encoded_len");
    let (bc, br) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz, c_int) -> *mut c_char,
    >("sodium_bin2base64");
    let (dc, dr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            Sz,
            *const c_char,
            Sz,
            *const c_char,
            *mut Sz,
            *mut *const c_char,
            c_int,
        ) -> c_int,
    >("sodium_base642bin");
    for &v in bad {
        let oc = forked_c(|| {
            unsafe { elc(16, v) };
        });
        let or = forked_c(|| {
            unsafe { elr(16, v) };
        });
        assert_eq!(oc, or, "encoded_len variant={v}");
        assert_eq!(oc, Outcome::Signaled(libc::SIGABRT), "encoded_len variant={v}");

        let oc = forked_c(|| {
            let bin = [1u8, 2, 3];
            let mut o = vec![0u8; 64];
            unsafe { bc(o.as_mut_ptr() as *mut c_char, 64, bin.as_ptr(), 3, v) };
        });
        let or = forked_c(|| {
            let bin = [1u8, 2, 3];
            let mut o = vec![0u8; 64];
            unsafe { br(o.as_mut_ptr() as *mut c_char, 64, bin.as_ptr(), 3, v) };
        });
        assert_eq!(oc, or, "bin2base64 variant={v}");
        assert_eq!(oc, Outcome::Signaled(libc::SIGABRT), "bin2base64 variant={v}");

        let oc = forked_c(|| {
            let b64 = b"AQID";
            let mut o = vec![0u8; 64];
            unsafe {
                dc(o.as_mut_ptr(), 64, b64.as_ptr() as *const c_char, 4, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), v)
            };
        });
        let or = forked_c(|| {
            let b64 = b"AQID";
            let mut o = vec![0u8; 64];
            unsafe {
                dr(o.as_mut_ptr(), 64, b64.as_ptr() as *const c_char, 4, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), v)
            };
        });
        assert_eq!(oc, or, "base642bin variant={v}");
        assert_eq!(oc, Outcome::Signaled(libc::SIGABRT), "base642bin variant={v}");
    }
    // and the four valid variants must NOT abort
    for &v in &[1, 3, 5, 7] {
        let oc = forked_c(|| {
            unsafe { elc(16, v) };
        });
        assert_eq!(oc, Outcome::Exited(0), "valid variant {v} must not abort (C)");
        let or = forked_c(|| {
            unsafe { elr(16, v) };
        });
        assert_eq!(or, Outcome::Exited(0), "valid variant {v} must not abort (Rust)");
    }
}

#[test]
fn row24_bin2base64_output_too_small_aborts() {
    let (bc, br) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz, c_int) -> *mut c_char,
    >("sodium_bin2base64");
    let (elc, _) = pair::<unsafe extern "C" fn(Sz, c_int) -> Sz>("sodium_base64_encoded_len");
    for &v in &[1, 3, 5, 7] {
        for blen in [1usize, 2, 3, 4, 16] {
            let need = unsafe { elc(blen, v) };
            for maxl in [0usize, 1, need - 1, need.saturating_sub(2)] {
                if maxl >= need {
                    continue;
                }
                let oc = forked_c(|| {
                    let bin = vec![7u8; blen];
                    let mut o = vec![0u8; 128];
                    unsafe { bc(o.as_mut_ptr() as *mut c_char, maxl, bin.as_ptr(), blen, v) };
                });
                let or = forked_c(|| {
                    let bin = vec![7u8; blen];
                    let mut o = vec![0u8; 128];
                    unsafe { br(o.as_mut_ptr() as *mut c_char, maxl, bin.as_ptr(), blen, v) };
                });
                assert_eq!(oc, or, "bin2base64 v={v} blen={blen} maxl={maxl}");
                assert_eq!(oc, Outcome::Signaled(libc::SIGABRT));
            }
        }
    }
}

// ============================== rows 25-29: sodium_base642bin rejections

#[test]
fn rows25_29_base642bin_errors() {
    type F = unsafe extern "C" fn(
        *mut u8,
        Sz,
        *const c_char,
        Sz,
        *const c_char,
        *mut Sz,
        *mut *const c_char,
        c_int,
    ) -> c_int;
    let (c, r) = pair::<F>("sodium_base642bin");
    // (b64, bin_maxlen)
    let cases: &[(&str, usize)] = &[
        // ERANGE: output too small
        ("AQID", 0),
        ("AQID", 1),
        ("AQID", 2),
        ("AQIDBAUGBwgJ", 3),
        // dangling bits / bad trailing group
        ("A", 8),
        ("AB", 8),
        ("ABC", 8),
        ("AQIDA", 8),
        ("/w==", 8),
        ("/x==", 8),
        ("QQ==", 8),
        ("QR==", 8),
        // padding errors
        ("AQ=", 8),
        ("AQ", 8),
        ("AQI", 8),
        ("AQ=A", 8),
        ("AQ==A", 8),
        ("====", 8),
        ("=", 8),
        // invalid characters / trailing junk
        ("AQ!D", 8),
        ("AQID!", 8),
        ("AQID ", 8),
        ("-_-_", 8),
        ("+/+/", 8),
        ("", 8),
    ];
    let ignores: [Option<&[u8]>; 2] = [None, Some(b" \0")];
    for &(b64, maxl) in cases {
        let bb = b64.as_bytes();
        for &v in &[1i32, 3, 5, 7] {
            for ig in ignores.iter() {
                for use_end in [false, true] {
                    let igp = match ig {
                        None => std::ptr::null(),
                        Some(s) => s.as_ptr() as *const c_char,
                    };
                    let mut ob = buf(maxl + 8);
                    let mut or = buf(maxl + 8);
                    let mut bc: Sz = 0xDEAD;
                    let mut brr: Sz = 0xDEAD;
                    let mut ec: *const c_char = std::ptr::null();
                    let mut er: *const c_char = std::ptr::null();
                    let ctx = format!("base642bin {b64:?} v={v} maxl={maxl} ig={} end={use_end}", ig.is_some());
                    cmp2!(
                        &ctx,
                        c(ob.as_mut_ptr(), maxl, bb.as_ptr() as *const c_char, bb.len(), igp, &mut bc, if use_end { &mut ec } else { std::ptr::null_mut() }, v),
                        r(or.as_mut_ptr(), maxl, bb.as_ptr() as *const c_char, bb.len(), igp, &mut brr, if use_end { &mut er } else { std::ptr::null_mut() }, v)
                    );
                    same_bytes(&ctx, &ob, &or);
                    assert_eq!(bc, brr, "{ctx}: bin_len differs");
                    if use_end {
                        assert_eq!(
                            ec as usize - bb.as_ptr() as usize,
                            er as usize - bb.as_ptr() as usize,
                            "{ctx}: b64_end differs"
                        );
                    }
                }
            }
        }
    }
}

// ============================ rows 30-37: ip2bin / bin2ip rejections

#[test]
fn rows30_37_ip_errors() {
    let (ic, ir) =
        pair::<unsafe extern "C" fn(*mut u8, *const c_char, Sz) -> c_int>("sodium_ip2bin");
    let (bc, br) =
        pair::<unsafe extern "C" fn(*mut c_char, Sz, *const u8) -> *mut c_char>("sodium_bin2ip");

    let bad: &[&str] = &[
        // zone errors
        "fe80::1%",
        "fe80::1%e th0",
        "fe80::1%eth0!",
        "fe80::1%eth/0",
        "fe80::1%%",
        "1.2.3.4%eth0",
        "1.2.3.4%",
        // ipv6 parse failures
        ":",
        ":::",
        "1:::2",
        "1:2:3:4:5:6:7:8:9",
        "gggg::1",
        "12345::1",
        "::1::2",
        "1:2:3:4:5:6:7",
        ":1:2:3:4:5:6:7:8",
        "1:2:3:4:5:6:7:8:",
        // ipv4 parse failures
        "",
        "1",
        "1.2",
        "1.2.3",
        "1.2.3.4.5",
        "256.1.1.1",
        "1.2.3.256",
        "-1.2.3.4",
        "1.2.3.",
        ".1.2.3",
        "1..2.3",
        "01.2.3.4",
        "1.2.3.04",
        "a.b.c.d",
        "1.2.3.4 ",
        " 1.2.3.4",
    ];
    for ip in bad {
        let ipb = ip.as_bytes();
        let mut ob = buf(16 + 8);
        let mut or = buf(16 + 8);
        let ctx = format!("sodium_ip2bin {ip:?}");
        cmp2!(&ctx, ic(ob.as_mut_ptr(), ipb.as_ptr() as *const c_char, ipb.len()), ir(or.as_mut_ptr(), ipb.as_ptr() as *const c_char, ipb.len()));
        same_bytes(&ctx, &ob, &or);
    }
    // rows 35-37: bin2ip with an output buffer that is too small
    let mut rng = Rng::seeded();
    let mut bins: Vec<[u8; 16]> = vec![[0u8; 16], [0xff; 16]];
    {
        let mut v4 = [0u8; 16];
        v4[10] = 0xff;
        v4[11] = 0xff;
        v4[12] = 255;
        v4[13] = 255;
        v4[14] = 255;
        v4[15] = 255;
        bins.push(v4);
    }
    for _ in 0..200 {
        let mut b = [0u8; 16];
        rng.fill(&mut b);
        bins.push(b);
    }
    for b in &bins {
        for maxl in 0usize..=48 {
            let mut ob = buf(maxl + 8);
            let mut or = buf(maxl + 8);
            unsafe {
                let pc = bc(ob.as_mut_ptr() as *mut c_char, maxl, b.as_ptr());
                let pr = br(or.as_mut_ptr() as *mut c_char, maxl, b.as_ptr());
                let ctx = format!("bin2ip {} maxl={maxl}", hex(b));
                assert_eq!(pc.is_null(), pr.is_null(), "{ctx}: NULL-ness differs");
                same_bytes(&ctx, &ob, &or);
            }
        }
    }
}

// ================================ rows 44-49: verify / memcmp / onetimeauth

#[test]
fn rows45_49_verify_mismatches() {
    let mut rng = Rng::seeded();
    for (name, n) in [("crypto_verify_16", 16usize), ("crypto_verify_32", 32), ("crypto_verify_64", 64)] {
        let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8) -> c_int>(name);
        for i in 0..n {
            for bit in 0..8 {
                let a = rng.bytes(n);
                let mut b = a.clone();
                b[i] ^= 1 << bit;
                let ret = unsafe {
                    let x = c(a.as_ptr(), b.as_ptr());
                    let y = r(a.as_ptr(), b.as_ptr());
                    same_ret(&format!("{name} diff@{i}.{bit}"), x, y);
                    x
                };
                assert_eq!(ret, -1, "{name} must reject a differing input");
            }
        }
    }
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8, Sz) -> c_int>("sodium_memcmp");
    for len in [1usize, 7, 16, 33, 64, 129] {
        for i in 0..len {
            let a = rng.bytes(len);
            let mut b = a.clone();
            b[i] ^= 0xff;
            unsafe {
                let x = c(a.as_ptr(), b.as_ptr(), len);
                let y = r(a.as_ptr(), b.as_ptr(), len);
                same_ret("sodium_memcmp mismatch", x, y);
                assert_eq!(x, -1);
            }
        }
    }
    // row 45: onetimeauth verify rejection
    for prefix in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let (osc, _) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int>(prefix);
        let (vc, vr) = pair::<
            unsafe extern "C" fn(*const u8, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{prefix}_verify"));
        for &l in &[0usize, 1, 16, 17, 64, 1000] {
            let m = rng.bytes(l);
            let k = rng.bytes(32);
            let mut tag = vec![0u8; 16];
            unsafe {
                osc(tag.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr());
            }
            for i in 0..16 {
                let mut bad = tag.clone();
                bad[i] ^= 0x01;
                unsafe {
                    let x = vc(bad.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr());
                    let y = vr(bad.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr());
                    same_ret(&format!("{prefix}_verify bad tag"), x, y);
                    assert_eq!(x, -1);
                }
            }
            // wrong key, wrong message
            let k2 = rng.bytes(32);
            unsafe {
                let x = vc(tag.as_ptr(), m.as_ptr(), l as c_ulonglong, k2.as_ptr());
                let y = vr(tag.as_ptr(), m.as_ptr(), l as c_ulonglong, k2.as_ptr());
                same_ret(&format!("{prefix}_verify wrong key"), x, y);
            }
            if l > 0 {
                let mut m2 = m.clone();
                m2[0] ^= 1;
                unsafe {
                    let x = vc(tag.as_ptr(), m2.as_ptr(), l as c_ulonglong, k.as_ptr());
                    let y = vr(tag.as_ptr(), m2.as_ptr(), l as c_ulonglong, k.as_ptr());
                    same_ret(&format!("{prefix}_verify wrong msg"), x, y);
                    assert_eq!(x, -1);
                }
            }
        }
    }
}

// ======================================== row 44: randombytes_sysrandom_close

#[test]
fn row44_randombytes_close() {
    for name in ["randombytes_close", "randombytes_sysrandom_close"] {
        if !libs().has(name) {
            continue;
        }
        let (c, r) = pair::<unsafe extern "C" fn() -> c_int>(name);
        // Repeated closes: whatever the C returns, Rust must return the same.
        for _ in 0..3 {
            cmp2!(name, c(), r());
        }
    }
}

// =============================== rows 167-191: pwhash argon2 rejections

#[test]
fn rows167_179_pwhash_argon2_errors() {
    type PwF = unsafe extern "C" fn(
        *mut u8,
        c_ulonglong,
        *const c_char,
        c_ulonglong,
        *const u8,
        c_ulonglong,
        Sz,
        c_int,
    ) -> c_int;
    let (c, r) = pair::<PwF>("crypto_pwhash");
    let pw = b"password\0";
    let salt = vec![0x42u8; 16];
    let mut out = buf(128);

        // row 167: out-of-range alg values crossing the FFI boundary
    for alg in [
        0i32,
        3,
        4,
        -1,
        -2,
        99,
        1000,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        0x1_0001u32 as i32,
    ] {
        let ret = cmp2!(
            &format!("crypto_pwhash alg={alg}"),
            c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 3, 8192, alg),
            r(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 3, 8192, alg)
        );
        assert_eq!(ret, -1, "alg={alg} must be rejected");
        set_errno(0);
        let _ = unsafe { c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 3, 8192, alg) };
        assert_eq!(errno(), EINVAL, "alg={alg} errno");
    }

        // rows 172-179 through the generic and both primitive entry points
    for (name, alg, ops_min) in [
        ("crypto_pwhash", 1i32, 3u64),
        ("crypto_pwhash", 2, 1),
        ("crypto_pwhash_argon2i", 1, 3),
        ("crypto_pwhash_argon2id", 2, 1),
    ] {
        let (c, r) = pair::<PwF>(name);
        // outlen below BYTES_MIN (16)
        for ol in [0u64, 1, 2, 8, 15] {
            let ret = cmp2!(&format!("{name} outlen={ol}"), c(out.as_mut_ptr(), ol, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops_min, 8192, alg), r(out.as_mut_ptr(), ol, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops_min, 8192, alg));
            assert_eq!(ret, -1, "{name} outlen={ol}");
        }
        // opslimit below MIN
        for ops in 0..ops_min {
            let ret = cmp2!(&format!("{name} ops={ops}"), c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops, 8192, alg), r(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops, 8192, alg));
            assert_eq!(ret, -1, "{name} ops={ops}");
        }
        // memlimit below MIN (8192)
        for mem in [0usize, 1, 1023, 1024, 4096, 8191] {
            let ret = cmp2!(&format!("{name} mem={mem}"), c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops_min, mem, alg), r(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops_min, mem, alg));
            assert_eq!(ret, -1, "{name} mem={mem}");
        }
        // opslimit / memlimit above MAX
        for (ops, mem) in [
            (u64::MAX, 8192usize),
            (u64::MAX, usize::MAX),
            (ops_min, usize::MAX),
            (0x1_0000_0000u64, 8192),
        ] {
            cmp2!(&format!("{name} ops={ops} mem={mem}"), c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops, mem, alg), r(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops, mem, alg));
        }
        // outlen above BYTES_MAX.
        // NOTE: the C does `memset(out, 0, outlen)` BEFORE validating outlen,
        // so a huge outlen writes out of bounds and crashes the process. That
        // is the C's genuine behaviour, so the differential assertion is that
        // BOTH sides terminate identically; run each in a forked child.
        for ol in [4_294_967_295u64, 4_294_967_296, u64::MAX] {
            let oc = run_forked(|| {
                let mut o = vec![0u8; 128];
                let pwl = b"password\0";
                let sl = vec![0x42u8; 16];
                let rc = unsafe {
                    c(o.as_mut_ptr(), ol, pwl.as_ptr() as *const c_char, 8, sl.as_ptr(), ops_min, 8192, alg)
                };
                unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
            });
            let or_ = run_forked(|| {
                let mut o = vec![0u8; 128];
                let pwl = b"password\0";
                let sl = vec![0x42u8; 32];
                let rc = unsafe {
                    r(o.as_mut_ptr(), ol, pwl.as_ptr() as *const c_char, 8, sl.as_ptr(), ops_min, 8192, alg)
                };
                unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
            });
            assert_eq!(oc, or_, "{name} outlen={ol}: outcome differs");
        }
        // passwdlen above PASSWD_MAX
        cmp2!(&format!("{name} passwdlen huge"), c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, u64::MAX, salt.as_ptr(), ops_min, 8192, alg), r(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, u64::MAX, salt.as_ptr(), ops_min, 8192, alg));
        // row 177: out == passwd aliasing
        let mut alias = vec![0x41u8; 64];
        let ret = cmp2!(&format!("{name} aliasing"), c(alias.as_mut_ptr(), 32, alias.as_ptr() as *const c_char, 8, salt.as_ptr(), ops_min, 8192, alg), r(alias.as_mut_ptr(), 32, alias.as_ptr() as *const c_char, 8, salt.as_ptr(), ops_min, 8192, alg));
        assert_eq!(ret, -1, "{name} aliasing must be rejected");
    }
        // rows 178-179: primitive entry point with the WRONG alg id
    for (name, wrong) in [
        ("crypto_pwhash_argon2i", 2i32),
        ("crypto_pwhash_argon2i", 0),
        ("crypto_pwhash_argon2i", 3),
        ("crypto_pwhash_argon2id", 1),
        ("crypto_pwhash_argon2id", 0),
        ("crypto_pwhash_argon2id", 3),
    ] {
        let (c, r) = pair::<PwF>(name);
        let ret = cmp2!(&format!("{name} wrong alg={wrong}"), c(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 3, 8192, wrong), r(out.as_mut_ptr(), 32, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 3, 8192, wrong));
        assert_eq!(ret, -1, "{name} alg={wrong}");
    }
}

#[test]
fn rows169_170_181_187_pwhash_str_errors() {
    type VF = unsafe extern "C" fn(*const c_char, *const c_char, c_ulonglong) -> c_int;
    type NF = unsafe extern "C" fn(*const c_char, c_ulonglong, Sz) -> c_int;
    let pw = b"password\0";

    let malformed: &[&str] = &[
        "",
        "$",
        "$$",
        "x",
        "$argon2",
        "$argon2i",
        "$argon2i$",
        "$argon2id$",
        "$argon2x$v=19$m=8,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=18$m=8,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=20$m=8,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=19$m=8,t=3,p=1$",
        "$argon2i$v=19$m=8,t=3,p=1$$",
        "$argon2i$v=19$m=0,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=19$m=8,t=0,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=19$m=8,t=3,p=0$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=19$m=08,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2i$v=19$m=8,t=3,p=1$c2FsdA$aGFzaA",
        "$argon2i$v=19$m=8,t=3,p=1$!!!!$!!!!",
        "$argon2i$v=19$m=99999999999999999999,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE",
        "$argon2id$v=19$m=8,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGE trailing",
        "$7$C6..../....c2FsdHNhbHQ$",
        "$2y$10$abcdefghijklmnopqrstuv",
        "not even close to a hash string",
    ];
    for (vname, nname) in [
        ("crypto_pwhash_str_verify", "crypto_pwhash_str_needs_rehash"),
        ("crypto_pwhash_argon2i_str_verify", "crypto_pwhash_argon2i_str_needs_rehash"),
        ("crypto_pwhash_argon2id_str_verify", "crypto_pwhash_argon2id_str_needs_rehash"),
    ] {
        let (vc, vr) = pair::<VF>(vname);
        let (nc, nr) = pair::<NF>(nname);
        for s in malformed {
            let mut z = s.as_bytes().to_vec();
            z.push(0);
            let ret = cmp2!(&format!("{vname} {s:?}"), vc(z.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8), vr(z.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8));
            assert_eq!(ret, -1, "{vname} {s:?} must be rejected");
            cmp2!(&format!("{nname} {s:?}"), nc(z.as_ptr() as *const c_char, 3, 8192), nr(z.as_ptr() as *const c_char, 3, 8192));
        }
        // row 185: strings at/over STRBYTES
        for n in [127usize, 128, 129, 200, 512] {
            let mut z = vec![b'$'; n];
            z.push(0);
            cmp2!(&format!("{nname} len={n}"), nc(z.as_ptr() as *const c_char, 3, 8192), nr(z.as_ptr() as *const c_char, 3, 8192));
        }
        // needs_rehash with out-of-range opslimit/memlimit
        let good = b"$argon2id$v=19$m=8,t=1,p=1$c2FsdHNhbHRzYWx0c2FsdA$RdescudvJCsgt3ub+b+dWRWJTmaaJObG\0";
        for (ops, mem) in [
            (u64::MAX, 8192usize),
            (0x1_0000_0000u64, 8192),
            (1, usize::MAX),
            (1, 8192),
            (99, 8192),
        ] {
            cmp2!(&format!("{nname} ops={ops} mem={mem}"), nc(good.as_ptr() as *const c_char, ops, mem), nr(good.as_ptr() as *const c_char, ops, mem));
        }
    }
    // row 181: wrong password against a well-formed string
    let (vc, vr) = pair::<VF>("crypto_pwhash_str_verify");
    let strbytes = {
        let (c, _) = pair::<unsafe extern "C" fn() -> Sz>("crypto_pwhash_strbytes");
        unsafe { c() }
    };
    let (sc, _) = pair::<
        unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz) -> c_int,
    >("crypto_pwhash_str");
    let mut s = vec![0u8; strbytes];
    unsafe {
        sc(s.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 8, 1, 8192);
    }
    for bad in [
        &b"passwore\0"[..],
        &b"Password\0"[..],
        &b"\0"[..],
        &b"passwordd\0"[..],
        &b"passwor\0"[..],
    ] {
        let n = bad.len() - 1;
        let ret = cmp2!("crypto_pwhash_str_verify wrong pw", vc(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, n as c_ulonglong), vr(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, n as c_ulonglong));
        assert_eq!(ret, -1, "wrong password must be rejected");
    }
    // passwdlen above PASSWD_MAX
    cmp2!("crypto_pwhash_str_verify passwdlen huge", vc(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, u64::MAX), vr(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, u64::MAX));
}

#[test]
fn row168_pwhash_str_alg_bad_alg_aborts() {
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz, c_int) -> c_int,
    >("crypto_pwhash_str_alg");
    for alg in [0i32, 3, -1, 999, i32::MIN, i32::MAX] {
        let oc = run_forked(|| {
            let mut s = vec![0u8; 256];
            let pw = b"pw\0";
            unsafe { c(s.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 2, 1, 8192, alg) };
        });
        let or = run_forked(|| {
            let mut s = vec![0u8; 256];
            let pw = b"pw\0";
            unsafe { r(s.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 2, 1, 8192, alg) };
        });
        assert_eq!(oc, or, "crypto_pwhash_str_alg alg={alg}: outcome differs");
        assert_eq!(
            oc,
            Outcome::Signaled(libc::SIGABRT),
            "crypto_pwhash_str_alg alg={alg} must abort"
        );
    }
}

// ================================ rows 192-206: scrypt rejections

#[test]
fn rows192_206_scrypt_errors() {
    let pw = b"password\0";
    let salt = vec![0x42u8; 32];
    let mut out = buf(128);

    let (c, r) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            c_ulonglong,
            *const c_char,
            c_ulonglong,
            *const u8,
            c_ulonglong,
            Sz,
        ) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256");
    // row 192: outlen < 16
    for ol in [0u64, 1, 8, 15] {
        let ret = cmp2!(&format!("scrypt outlen={ol}"), c(out.as_mut_ptr(), ol, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 32768, 16777216), r(out.as_mut_ptr(), ol, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), 32768, 16777216));
        assert_eq!(ret, -1, "scrypt outlen={ol}");
    }
    // row 193: huge outlen. Like argon2, the C memsets `out` for `outlen`
    // bytes BEFORE validating it, so compare termination in a forked child.
    for ol in [0x1f_ffff_ffe0u64, 0x1f_ffff_ffe1, u64::MAX] {
        let oc = run_forked(|| {
            let mut o = vec![0u8; 128];
            let pwl = b"password\0";
            let sl = vec![0x42u8; 32];
            let rc = unsafe {
                c(o.as_mut_ptr(), ol, pwl.as_ptr() as *const c_char, 8, sl.as_ptr(), 32768, 16777216)
            };
            unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
        });
        let or_ = run_forked(|| {
            let mut o = vec![0u8; 128];
            let pwl = b"password\0";
            let sl = vec![0x42u8; 32];
            let rc = unsafe {
                r(o.as_mut_ptr(), ol, pwl.as_ptr() as *const c_char, 8, sl.as_ptr(), 32768, 16777216)
            };
            unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
        });
        assert_eq!(oc, or_, "scrypt outlen={ol}: outcome differs");
    }
    // row 194: unusable params
    for (ol, ops, mem) in [
        (32, 0, 16777216),
        (32, 1, 0),
        (32, 0, 0),
        (32, u64::MAX, usize::MAX),
        (32, 1, 1),
        (32, 16, 1024),
    ] {
        cmp2!(&format!("scrypt ol={ol} ops={ops} mem={mem}"), c(out.as_mut_ptr(), ol, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops, mem), r(out.as_mut_ptr(), ol, pw.as_ptr() as *const c_char, 8, salt.as_ptr(), ops, mem));
    }
    // row 195: out == passwd
    let mut alias = vec![0x41u8; 128];
    let ret = cmp2!("scrypt aliasing", c(alias.as_mut_ptr(), 32, alias.as_ptr() as *const c_char, 8, salt.as_ptr(), 32768, 16777216), r(alias.as_mut_ptr(), 32, alias.as_ptr() as *const c_char, 8, salt.as_ptr(), 32768, 16777216));
    assert_eq!(ret, -1, "scrypt aliasing must be rejected");
    // passwdlen at PASSWD_MAX.
    // NOTE: crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX == SODIUM_SIZE_MAX,
    // so u64::MAX is NOT rejected and the C reads `passwd` out of bounds. That
    // is the C's genuine behaviour; compare termination in a forked child.
    for pl in [u64::MAX, u64::MAX - 1, 1u64 << 40] {
        let oc = run_forked(|| {
            let mut o = vec![0u8; 128];
            let pwl = b"password\0";
            let sl = vec![0x42u8; 32];
            let rc = unsafe {
                c(o.as_mut_ptr(), 32, pwl.as_ptr() as *const c_char, pl, sl.as_ptr(), 32768, 16777216)
            };
            unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
        });
        let or_ = run_forked(|| {
            let mut o = vec![0u8; 128];
            let pwl = b"password\0";
            let sl = vec![0x42u8; 32];
            let rc = unsafe {
                r(o.as_mut_ptr(), 32, pwl.as_ptr() as *const c_char, pl, sl.as_ptr(), 32768, 16777216)
            };
            unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
        });
        assert_eq!(oc, or_, "scrypt passwdlen={pl}: outcome differs");
    }

    // rows 201-206: _ll parameter rejections
    let (lc, lr) = pair::<
        unsafe extern "C" fn(*const u8, Sz, *const u8, Sz, u64, u32, u32, *mut u8, Sz) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256_ll");
    let cases: &[(u64, u32, u32, usize)] = &[
        // N not a power of two / < 2
        (0, 8, 1, 32),
        (1, 8, 1, 32),
        (3, 8, 1, 32),
        (5, 8, 1, 32),
        (6, 8, 1, 32),
        (7, 8, 1, 32),
        (1000, 8, 1, 32),
        (u64::MAX, 8, 1, 32),
        (1u64 << 32, 8, 1, 32),
        ((1u64 << 32) + 1, 8, 1, 32),
        // r == 0 / p == 0
        (16, 0, 1, 32),
        (16, 8, 0, 32),
        (16, 0, 0, 32),
        // r * p >= 2^30
        (16, 1 << 15, 1 << 15, 32),
        (16, 1 << 29, 2, 32),
        (16, u32::MAX, u32::MAX, 32),
        (16, 1 << 30, 1, 32),
        // allocation overflow
        (1u64 << 31, 1 << 20, 1, 32),
        (2, u32::MAX, 1, 32),
        // buflen 0 (valid) and huge
        (16, 8, 1, 0),
        (16, 8, 1, usize::MAX),
    ];
    for &(n, rr_, p, bl) in cases {
        let mut ob = buf(64);
        let mut or = buf(64);
        cmp2!(&format!("scrypt_ll N={n} r={rr_} p={p} bl={bl}"), lc(pw.as_ptr(), 8, salt.as_ptr(), 32, n, rr_, p, ob.as_mut_ptr(), bl.min(64)), lr(pw.as_ptr(), 8, salt.as_ptr(), 32, n, rr_, p, or.as_mut_ptr(), bl.min(64)));
        same_bytes(&format!("scrypt_ll N={n} r={rr_} p={p}"), &ob, &or);
    }

    // rows 196-200: _str_verify / _str_needs_rehash on malformed strings
    let (vc, vr) = pair::<unsafe extern "C" fn(*const c_char, *const c_char, c_ulonglong) -> c_int>(
        "crypto_pwhash_scryptsalsa208sha256_str_verify",
    );
    let (nc, nr) = pair::<unsafe extern "C" fn(*const c_char, c_ulonglong, Sz) -> c_int>(
        "crypto_pwhash_scryptsalsa208sha256_str_needs_rehash",
    );
    let malformed: &[&str] = &[
        "",
        "$",
        "$7",
        "$7$",
        "$6$C6..../....c2FsdHNhbHQ$",
        "$argon2id$v=19$m=8,t=1,p=1$c2FsdA$aGFzaA",
        "x",
        "$7$C6..../....",
        // wrong length (must be exactly 101 chars)
        &"$7$C6..../....SodiumChloride$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"[..],
    ];
    for s in malformed {
        let mut z = s.as_bytes().to_vec();
        z.push(0);
        let ret = cmp2!(&format!("scrypt_str_verify {s:?}"), vc(z.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8), vr(z.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8));
        assert_ne!(ret, 0, "scrypt_str_verify {s:?} must be rejected");
        cmp2!(&format!("scrypt_needs_rehash {s:?}"), nc(z.as_ptr() as *const c_char, 32768, 16777216), nr(z.as_ptr() as *const c_char, 32768, 16777216));
    }
    // exactly-101-char strings with garbage payloads
    for filler in [b'a', b'.', b'/', b'0', b'z'] {
        let mut z = b"$7$".to_vec();
        z.resize(101, filler);
        z.push(0);
        cmp2!("scrypt_str_verify 101-char garbage", vc(z.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8), vr(z.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8));
        cmp2!("scrypt_needs_rehash 101-char garbage", nc(z.as_ptr() as *const c_char, 32768, 16777216), nr(z.as_ptr() as *const c_char, 32768, 16777216));
    }
    // needs_rehash with unusable params
    let (sc, _) = pair::<
        unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256_str");
    let sb = {
        let (c, _) =
            pair::<unsafe extern "C" fn() -> Sz>("crypto_pwhash_scryptsalsa208sha256_strbytes");
        unsafe { c() }
    };
    let mut s = vec![0u8; sb];
    unsafe {
        sc(s.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 8, 32768, 16777216);
    }
    for (ops, mem) in [(0u64, 16777216usize), (1, 0), (u64::MAX, usize::MAX), (32768, 16777216)] {
        cmp2!(&format!("scrypt_needs_rehash ops={ops} mem={mem}"), nc(s.as_ptr() as *const c_char, ops, mem), nr(s.as_ptr() as *const c_char, ops, mem));
    }
    // row 197: wrong password
    for bad in [&b"passwore\0"[..], &b"\0"[..]] {
        let n = bad.len() - 1;
        let ret = cmp2!("scrypt_str_verify wrong pw", vc(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, n as c_ulonglong), vr(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, n as c_ulonglong));
        assert_ne!(ret, 0);
    }
    // _str with unusable params
    let (sc2, sr2) = pair::<
        unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256_str");
    for (ops, mem) in [(0u64, 16777216usize), (1, 0), (u64::MAX, usize::MAX), (0, 0)] {
        let mut a = vec![0u8; sb];
        let mut b = vec![0u8; sb];
        cmp2!(&format!("scrypt_str ops={ops} mem={mem}"), sc2(a.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 8, ops, mem), sr2(b.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 8, ops, mem));
    }
    // passwdlen at PASSWD_MAX for _str: same out-of-bounds read as above.
    for pl in [u64::MAX, 1u64 << 40] {
        let oc = run_forked(|| {
            let mut a = vec![0u8; 256];
            let pwl = b"password\0";
            let rc = unsafe {
                sc2(a.as_mut_ptr() as *mut c_char, pwl.as_ptr() as *const c_char, pl, 32768, 16777216)
            };
            unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
        });
        let or_ = run_forked(|| {
            let mut b = vec![0u8; 256];
            let pwl = b"password\0";
            let rc = unsafe {
                sr2(b.as_mut_ptr() as *mut c_char, pwl.as_ptr() as *const c_char, pl, 32768, 16777216)
            };
            unsafe { libc::_exit(if rc == -1 { 66 } else { 0 }) };
        });
        assert_eq!(oc, or_, "scrypt_str passwdlen={pl}: outcome differs");
    }
}

// ============================ rows 207-212: generichash bound rejections

#[test]
fn rows207_212_generichash_errors() {
    let statebytes = {
        let (c, _) = pair::<unsafe extern "C" fn() -> Sz>("crypto_generichash_statebytes");
        unsafe { c() }
    };
    let mut rng = Rng::seeded();
    let m = rng.bytes(64);
    let key = rng.bytes(128);

    let bad_out = [0usize, 65, 66, 127, 128, 1000, usize::MAX];
    let bad_key = [65usize, 66, 127, 128, 1000, usize::MAX];

    for name in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, Sz, *const u8, c_ulonglong, *const u8, Sz) -> c_int,
        >(name);
        let mut out = buf(256);
        for &ol in &bad_out {
            let ret = cmp2!(&format!("{name} outlen={ol}"), c(out.as_mut_ptr(), ol, m.as_ptr(), 64, std::ptr::null(), 0), r(out.as_mut_ptr(), ol, m.as_ptr(), 64, std::ptr::null(), 0));
            assert_eq!(ret, -1, "{name} outlen={ol}");
        }
        for &kl in &bad_key {
            let ret = cmp2!(&format!("{name} keylen={kl}"), c(out.as_mut_ptr(), 32, m.as_ptr(), 64, key.as_ptr(), kl), r(out.as_mut_ptr(), 32, m.as_ptr(), 64, key.as_ptr(), kl));
            assert_eq!(ret, -1, "{name} keylen={kl}");
        }
    }
    // salt_personal
    let (spc, spr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            Sz,
            *const u8,
            c_ulonglong,
            *const u8,
            Sz,
            *const u8,
            *const u8,
        ) -> c_int,
    >("crypto_generichash_blake2b_salt_personal");
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);
    let mut out = buf(256);
    for &ol in &bad_out {
        let ret = cmp2!(&format!("blake2b_salt_personal outlen={ol}"), spc(out.as_mut_ptr(), ol, m.as_ptr(), 64, std::ptr::null(), 0, salt.as_ptr(), pers.as_ptr()), spr(out.as_mut_ptr(), ol, m.as_ptr(), 64, std::ptr::null(), 0, salt.as_ptr(), pers.as_ptr()));
        assert_eq!(ret, -1);
    }
    for &kl in &bad_key {
        let ret = cmp2!(&format!("blake2b_salt_personal keylen={kl}"), spc(out.as_mut_ptr(), 32, m.as_ptr(), 64, key.as_ptr(), kl, salt.as_ptr(), pers.as_ptr()), spr(out.as_mut_ptr(), 32, m.as_ptr(), 64, key.as_ptr(), kl, salt.as_ptr(), pers.as_ptr()));
        assert_eq!(ret, -1);
    }
    // init variants
    for name in ["crypto_generichash_init", "crypto_generichash_blake2b_init"] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz, Sz) -> c_int>(name);
        let mut sa = buf(statebytes);
        let mut sb = buf(statebytes);
        for &ol in &bad_out {
            let ret = cmp2!(&format!("{name} outlen={ol}"), c(sa.as_mut_ptr(), std::ptr::null(), 0, ol), r(sb.as_mut_ptr(), std::ptr::null(), 0, ol));
            assert_eq!(ret, -1, "{name} outlen={ol}");
        }
        for &kl in &bad_key {
            let ret = cmp2!(&format!("{name} keylen={kl}"), c(sa.as_mut_ptr(), key.as_ptr(), kl, 32), r(sb.as_mut_ptr(), key.as_ptr(), kl, 32));
            assert_eq!(ret, -1, "{name} keylen={kl}");
        }
    }
    let (ic, ir) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, Sz, Sz, *const u8, *const u8) -> c_int,
    >("crypto_generichash_blake2b_init_salt_personal");
    let mut sa = buf(statebytes);
    let mut sb = buf(statebytes);
    for &ol in &bad_out {
        let ret = cmp2!(&format!("init_salt_personal outlen={ol}"), ic(sa.as_mut_ptr(), std::ptr::null(), 0, ol, salt.as_ptr(), pers.as_ptr()), ir(sb.as_mut_ptr(), std::ptr::null(), 0, ol, salt.as_ptr(), pers.as_ptr()));
        assert_eq!(ret, -1);
    }
    for &kl in &bad_key {
        let ret = cmp2!(&format!("init_salt_personal keylen={kl}"), ic(sa.as_mut_ptr(), key.as_ptr(), kl, 32, salt.as_ptr(), pers.as_ptr()), ir(sb.as_mut_ptr(), key.as_ptr(), kl, 32, salt.as_ptr(), pers.as_ptr()));
        assert_eq!(ret, -1);
    }
}

// ============================ rows 213-217: hmac init misuse + verify

#[test]
fn rows213_214_hmac_init_null_key_aborts() {
    for name in [
        "crypto_auth_hmacsha256_init",
        "crypto_auth_hmacsha512_init",
        "crypto_auth_hmacsha512256_init",
    ] {
        if !libs().has(name) {
            continue;
        }
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz) -> c_int>(name);
        for kl in [1usize, 16, 32, 64, 1000] {
            let oc = run_forked(|| {
                let mut s = vec![0u8; 512];
                unsafe { c(s.as_mut_ptr(), std::ptr::null(), kl) };
            });
            let or = run_forked(|| {
                let mut s = vec![0u8; 512];
                unsafe { r(s.as_mut_ptr(), std::ptr::null(), kl) };
            });
            assert_eq!(oc, or, "{name} NULL key keylen={kl}: outcome differs");
        }
        // key == NULL with keylen == 0 must NOT abort
        let oc = run_forked(|| {
            let mut s = vec![0u8; 512];
            unsafe { c(s.as_mut_ptr(), std::ptr::null(), 0) };
        });
        let or = run_forked(|| {
            let mut s = vec![0u8; 512];
            unsafe { r(s.as_mut_ptr(), std::ptr::null(), 0) };
        });
        assert_eq!(oc, or, "{name} NULL key keylen=0");
        assert_eq!(oc, Outcome::Exited(0), "{name} NULL key keylen=0 must succeed");
    }
}

#[test]
fn rows215_217_hmac_verify_mismatch() {
    let mut rng = Rng::seeded();
    for (name, outlen) in [
        ("crypto_auth_hmacsha256", 32usize),
        ("crypto_auth_hmacsha512", 64),
        ("crypto_auth_hmacsha512256", 32),
        ("crypto_auth", 32),
    ] {
        let (oc, _) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int,
        >(name);
        let (vc, vr) = pair::<
            unsafe extern "C" fn(*const u8, *const u8, c_ulonglong, *const u8) -> c_int,
        >(&format!("{name}_verify"));
        for &l in &[0usize, 1, 64, 128, 1000] {
            let m = rng.bytes(l);
            let k = rng.bytes(32);
            let mut tag = vec![0u8; outlen];
            unsafe {
                oc(tag.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr());
            }
            for i in 0..outlen {
                let mut bad = tag.clone();
                bad[i] ^= 0x80;
                let ret = cmp2!(&format!("{name}_verify bad tag @{i}"), vc(bad.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()), vr(bad.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()));
                assert_ne!(ret, 0, "{name}_verify must reject a bad tag");
            }
            // wrong key
            let k2 = rng.bytes(32);
            cmp2!(&format!("{name}_verify wrong key"), vc(tag.as_ptr(), m.as_ptr(), l as c_ulonglong, k2.as_ptr()), vr(tag.as_ptr(), m.as_ptr(), l as c_ulonglong, k2.as_ptr()));
            // all-zero and all-ff tags
            for fill in [0u8, 0xff] {
                let bad = vec![fill; outlen];
                cmp2!(&format!("{name}_verify tag={fill:02x}"), vc(bad.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()), vr(bad.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()));
            }
        }
    }
}

// ============================== rows 218-221: kdf bound rejections

#[test]
fn rows218_221_kdf_errors() {
    let mut rng = Rng::seeded();
    let key = rng.bytes(32);
    let ctx = b"context\0";
    let mut out = buf(256);

    for name in [
        "crypto_kdf_derive_from_key",
        "crypto_kdf_blake2b_derive_from_key",
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, Sz, u64, *const c_char, *const u8) -> c_int,
        >(name);
        for sl in [0usize, 1, 8, 15, 65, 66, 128, 1000, usize::MAX] {
            let ret = cmp2!(&format!("{name} subkey_len={sl}"), c(out.as_mut_ptr(), sl.min(256), 0, ctx.as_ptr() as *const c_char, key.as_ptr()), r(out.as_mut_ptr(), sl.min(256), 0, ctx.as_ptr() as *const c_char, key.as_ptr()));
            if sl < 16 || sl > 64 {
                assert_eq!(ret, -1, "{name} subkey_len={sl} must be rejected");
                set_errno(0);
        let _ = unsafe { c(out.as_mut_ptr(), sl.min(256), 0, ctx.as_ptr() as *const c_char, key.as_ptr()) };
        let e = errno();
                assert_eq!(e, EINVAL, "{name} subkey_len={sl} errno");
            }
        }
    }
    for (name, maxv) in [
        ("crypto_kdf_hkdf_sha256_expand", 0xffusize * 32),
        ("crypto_kdf_hkdf_sha512_expand", 0xff * 64),
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, Sz, *const c_char, Sz, *const u8) -> c_int,
        >(name);
        let prk = rng.bytes(64);
        let mut big = buf(maxv + 64);
        for ol in [maxv + 1, maxv + 2, maxv * 2, usize::MAX] {
            let ret = cmp2!(&format!("{name} out_len={ol}"), c(big.as_mut_ptr(), ol, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()), r(big.as_mut_ptr(), ol, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()));
            assert_eq!(ret, -1, "{name} out_len={ol} must be rejected");
            set_errno(0);
        let _ = unsafe { c(big.as_mut_ptr(), ol, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()) };
        let e = errno();
            assert_eq!(e, EINVAL, "{name} errno");
        }
        // exactly at the max must succeed
        let ret = cmp2!(&format!("{name} out_len=max"), c(big.as_mut_ptr(), maxv, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()), r(big.as_mut_ptr(), maxv, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()));
        assert_eq!(ret, 0, "{name} out_len=max must succeed");
    }
}

// ================== rows 222-225: sha3 / xof phase-violation rejections

#[test]
fn rows222_223_sha3_phase_errors() {
    let mut rng = Rng::seeded();
    let m = rng.bytes(200);
    for (prefix, outlen) in [("crypto_hash_sha3256", 32usize), ("crypto_hash_sha3512", 64)] {
        let sb = {
            let (c, _) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
            unsafe { c() }
        };
        let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(&format!("{prefix}_init"));
        let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(
            &format!("{prefix}_update"),
        );
        let (fc, fr) =
            pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{prefix}_final"));

        let mut sa = buf(sb);
        let mut sbb = buf(sb);
        let mut oa = buf(outlen);
        let mut ob = buf(outlen);
        unsafe {
            ic(sa.as_mut_ptr());
            ir(sbb.as_mut_ptr());
            uc(sa.as_mut_ptr(), m.as_ptr(), 200);
            ur(sbb.as_mut_ptr(), m.as_ptr(), 200);
            same_ret(
                &format!("{prefix}_final first"),
                fc(sa.as_mut_ptr(), oa.as_mut_ptr()),
                fr(sbb.as_mut_ptr(), ob.as_mut_ptr()),
            );
        }
        same_bytes(&format!("{prefix} first final"), &oa, &ob);
        // row 223: double final (no intervening update) must fail
        let mut oa2 = buf(outlen);
        let mut ob2 = buf(outlen);
        let ret = cmp2!(
            &format!("{prefix}_final twice"),
            fc(sa.as_mut_ptr(), oa2.as_mut_ptr()),
            fr(sbb.as_mut_ptr(), ob2.as_mut_ptr())
        );
        assert_eq!(ret, -1, "{prefix}_final twice must fail");
        same_bytes(&format!("{prefix} out after double final"), &oa2, &ob2);
        // row 222: update after final must fail. NOTE: the C's failing update
        // ALSO resets phase back to ABSORBING, so the next final succeeds
        // again — we assert only that C and Rust agree on all of it.
        let ret = cmp2!(
            &format!("{prefix}_update after final"),
            uc(sa.as_mut_ptr(), m.as_ptr(), 200),
            ur(sbb.as_mut_ptr(), m.as_ptr(), 200)
        );
        assert_eq!(ret, -1, "{prefix}_update after final must fail");
        same_bytes(&format!("{prefix} state after bad update"), &sa, &sbb);
        // repeated bad calls must stay in lockstep
        for _ in 0..3 {
            cmp2!(
                &format!("{prefix}_update repeated"),
                uc(sa.as_mut_ptr(), m.as_ptr(), 10),
                ur(sbb.as_mut_ptr(), m.as_ptr(), 10)
            );
            cmp2!(
                &format!("{prefix}_final repeated"),
                fc(sa.as_mut_ptr(), oa2.as_mut_ptr()),
                fr(sbb.as_mut_ptr(), ob2.as_mut_ptr())
            );
            same_bytes(&format!("{prefix} state lockstep"), &sa, &sbb);
            same_bytes(&format!("{prefix} out lockstep"), &oa2, &ob2);
        }
    }
}

#[test]
fn rows224_225_xof_phase_errors() {
    let mut rng = Rng::seeded();
    let m = rng.bytes(200);
    for prefix in [
        "crypto_xof_shake128",
        "crypto_xof_shake256",
        "crypto_xof_turboshake128",
        "crypto_xof_turboshake256",
    ] {
        let sb = {
            let (c, _) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
            unsafe { c() }
        };
        let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(&format!("{prefix}_init"));
        let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(
            &format!("{prefix}_update"),
        );
        let (qc, qr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, Sz) -> c_int>(&format!(
            "{prefix}_squeeze"
        ));

        let mut sa = buf(sb);
        let mut sbb = buf(sb);
        let mut oa = buf(64);
        let mut ob = buf(64);
        unsafe {
            ic(sa.as_mut_ptr());
            ir(sbb.as_mut_ptr());
            uc(sa.as_mut_ptr(), m.as_ptr(), 200);
            ur(sbb.as_mut_ptr(), m.as_ptr(), 200);
            same_ret(
                &format!("{prefix}_squeeze first"),
                qc(sa.as_mut_ptr(), oa.as_mut_ptr(), 64),
                qr(sbb.as_mut_ptr(), ob.as_mut_ptr(), 64),
            );
        }
        same_bytes(&format!("{prefix} first squeeze"), &oa, &ob);
        // row 224/225: update after squeeze began
        let ret = cmp2!(&format!("{prefix}_update after squeeze"), uc(sa.as_mut_ptr(), m.as_ptr(), 200), ur(sbb.as_mut_ptr(), m.as_ptr(), 200));
        assert_eq!(ret, -1, "{prefix}_update after squeeze must fail");
        same_bytes(&format!("{prefix} state after bad update"), &sa, &sbb);
        // further squeezes / updates must stay in lockstep
        for _ in 0..4 {
            let mut a = buf(37);
            let mut b = buf(37);
            cmp2!(&format!("{prefix}_squeeze after bad update"), qc(sa.as_mut_ptr(), a.as_mut_ptr(), 37), qr(sbb.as_mut_ptr(), b.as_mut_ptr(), 37));
            same_bytes(&format!("{prefix} squeeze lockstep"), &a, &b);
            cmp2!(&format!("{prefix}_update repeated"), uc(sa.as_mut_ptr(), m.as_ptr(), 5), ur(sbb.as_mut_ptr(), m.as_ptr(), 5));
            same_bytes(&format!("{prefix} state lockstep"), &sa, &sbb);
        }
        // zero-length squeeze on a fresh state
        let mut sa = buf(sb);
        let mut sbb = buf(sb);
        unsafe {
            ic(sa.as_mut_ptr());
            ir(sbb.as_mut_ptr());
        }
        cmp2!(&format!("{prefix}_squeeze len=0"), qc(sa.as_mut_ptr(), oa.as_mut_ptr(), 0), qr(sbb.as_mut_ptr(), ob.as_mut_ptr(), 0));
    }
}

// -------------------------------------------------- unused constant silencer
#[allow(dead_code)]
fn _unused() {
    let _ = (EPERM, ENOENT, EFBIG, ERANGE);
}
