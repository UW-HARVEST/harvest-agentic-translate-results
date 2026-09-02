//! Phase B + Phase C for `sodium/utils.c` and `sodium/codecs.c`.
//!
//! CONFIGS rows PA1–PA73, ERRORS rows A9–A46.
//! Both libraries are driven through their `.so` exports only.

mod harness;
use harness::*;

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

const SEED: u64 = 0x5EED_0001;

// ---------------------------------------------------------------------------
// utils.c — constant-time comparators (PA47–PA55, PA74–PA77)
// ---------------------------------------------------------------------------

#[test]
fn sodium_memcmp_all_lengths() {
    type F = unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int;
    let (c, r) = sym::<F>("sodium_memcmp");
    let mut rng = Rng::new(SEED);
    for len in [0usize, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 255] {
        for _ in 0..40 {
            let a = rng.bytes(len);
            // equal, and differing in exactly one randomly chosen position
            let mut b = a.clone();
            unsafe {
                assert_eq!(
                    c(a.as_ptr() as _, b.as_ptr() as _, len),
                    r(a.as_ptr() as _, b.as_ptr() as _, len),
                    "sodium_memcmp equal len={len}"
                );
            }
            if len > 0 {
                let i = rng.below(len);
                b[i] ^= 1 << rng.below(8);
                unsafe {
                    assert_eq!(
                        c(a.as_ptr() as _, b.as_ptr() as _, len),
                        r(a.as_ptr() as _, b.as_ptr() as _, len),
                        "sodium_memcmp differing len={len} i={i}"
                    );
                }
            }
        }
    }
}

#[test]
fn sodium_compare_lexicographic() {
    type F = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
    let (c, r) = sym::<F>("sodium_compare");
    let mut rng = Rng::new(SEED ^ 1);
    for len in [0usize, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 64, 65] {
        for _ in 0..60 {
            let a = rng.bytes(len);
            let mut b = if rng.below(2) == 0 { a.clone() } else { rng.bytes(len) };
            if len > 0 && rng.below(3) == 0 {
                // force a difference only in the most-significant (last) byte,
                // which is where sodium_compare's little-endian loop starts.
                b = a.clone();
                b[len - 1] = b[len - 1].wrapping_add(1);
            }
            unsafe {
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr(), len),
                    r(a.as_ptr(), b.as_ptr(), len),
                    "sodium_compare len={len} a={} b={}",
                    hex(&a),
                    hex(&b)
                );
            }
        }
    }
    // Exhaustive over 2 bytes: every ordering relation.
    for x in 0..=u16::MAX {
        let a = x.to_le_bytes();
        for y in [0u16, 1, 0xff, 0x100, 0xffff, x, x.wrapping_add(1)] {
            let b = y.to_le_bytes();
            unsafe {
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr(), 2),
                    r(a.as_ptr(), b.as_ptr(), 2),
                    "sodium_compare 2-byte {x:#06x} vs {y:#06x}"
                );
            }
        }
    }
}

#[test]
fn sodium_is_zero_all_lengths() {
    type F = unsafe extern "C" fn(*const u8, usize) -> c_int;
    let (c, r) = sym::<F>("sodium_is_zero");
    let mut rng = Rng::new(SEED ^ 2);
    for len in [0usize, 1, 2, 8, 15, 16, 17, 32, 33, 64, 129] {
        let z = vec![0u8; len];
        unsafe {
            assert_eq!(c(z.as_ptr(), len), r(z.as_ptr(), len), "is_zero zero len={len}");
        }
        for _ in 0..30 {
            let mut v = vec![0u8; len];
            if len > 0 {
                v[rng.below(len)] = 1 + rng.byte() % 255;
            }
            unsafe {
                assert_eq!(c(v.as_ptr(), len), r(v.as_ptr(), len), "is_zero len={len}");
            }
            let v = rng.bytes(len);
            unsafe {
                assert_eq!(c(v.as_ptr(), len), r(v.as_ptr(), len), "is_zero rnd len={len}");
            }
        }
    }
}

#[test]
fn crypto_verify_16_32_64() {
    type F = unsafe extern "C" fn(*const u8, *const u8) -> c_int;
    let mut rng = Rng::new(SEED ^ 3);
    for (name, n) in [("crypto_verify_16", 16usize), ("crypto_verify_32", 32), ("crypto_verify_64", 64)] {
        let (c, r) = sym::<F>(name);
        for _ in 0..200 {
            let a = rng.bytes(n);
            let mut b = a.clone();
            unsafe {
                assert_eq!(c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr()), "{name} equal");
            }
            let i = rng.below(n);
            b[i] ^= 1 << rng.below(8);
            unsafe {
                assert_eq!(c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr()), "{name} diff at {i}");
            }
            let b = rng.bytes(n);
            unsafe {
                assert_eq!(c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr()), "{name} random");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// utils.c — big-integer helpers (PA56–PA61)
// ---------------------------------------------------------------------------

#[test]
fn sodium_increment_add_sub() {
    type F1 = unsafe extern "C" fn(*mut u8, usize);
    type F2 = unsafe extern "C" fn(*mut u8, *const u8, usize);
    let (ci, ri) = sym::<F1>("sodium_increment");
    let (ca, ra) = sym::<F2>("sodium_add");
    let (cs, rs) = sym::<F2>("sodium_sub");
    let mut rng = Rng::new(SEED ^ 4);

    for len in [0usize, 1, 2, 4, 7, 8, 9, 12, 16, 24, 31, 32, 33, 64] {
        // increment: random, all-0xff (full carry), 0xff-prefix (partial carry)
        for seedv in 0..3u32 {
            let mut base = match seedv {
                0 => rng.bytes(len),
                1 => vec![0xffu8; len],
                _ => {
                    let mut v = vec![0xffu8; len];
                    if len > 1 {
                        v[len - 1] = 0x00;
                    }
                    v
                }
            };
            // exercise repeated increments so carries propagate
            let mut bc = base.clone();
            let mut br = base.clone();
            for _ in 0..5 {
                unsafe {
                    ci(bc.as_mut_ptr(), len);
                    ri(br.as_mut_ptr(), len);
                }
                eqb(&format!("sodium_increment len={len} kind={seedv}"), &bc, &br);
            }
            base.clear();
        }

        for _ in 0..40 {
            let a = rng.bytes(len);
            let b = rng.bytes(len);
            let mut ac = a.clone();
            let mut ar = a.clone();
            unsafe {
                ca(ac.as_mut_ptr(), b.as_ptr(), len);
                ra(ar.as_mut_ptr(), b.as_ptr(), len);
            }
            eqb(&format!("sodium_add len={len}"), &ac, &ar);

            let mut ac = a.clone();
            let mut ar = a.clone();
            unsafe {
                cs(ac.as_mut_ptr(), b.as_ptr(), len);
                rs(ar.as_mut_ptr(), b.as_ptr(), len);
            }
            eqb(&format!("sodium_sub len={len}"), &ac, &ar);
        }
        // maximal carry / borrow
        let ff = vec![0xffu8; len];
        let one = {
            let mut v = vec![0u8; len];
            if len > 0 {
                v[0] = 1;
            }
            v
        };
        for (nm, f_c, f_r) in [("add", ca, ra), ("sub", cs, rs)] {
            let mut ac = ff.clone();
            let mut ar = ff.clone();
            unsafe {
                f_c(ac.as_mut_ptr(), one.as_ptr(), len);
                f_r(ar.as_mut_ptr(), one.as_ptr(), len);
            }
            eqb(&format!("sodium_{nm} 0xff..+1 len={len}"), &ac, &ar);
            let mut ac = vec![0u8; len];
            let mut ar = vec![0u8; len];
            unsafe {
                f_c(ac.as_mut_ptr(), one.as_ptr(), len);
                f_r(ar.as_mut_ptr(), one.as_ptr(), len);
            }
            eqb(&format!("sodium_{nm} 0-1 len={len}"), &ac, &ar);
        }
    }
}

#[test]
fn sodium_memzero_and_stackzero() {
    let (c, r) = sym::<unsafe extern "C" fn(*mut c_void, usize)>("sodium_memzero");
    let mut rng = Rng::new(SEED ^ 5);
    for len in [0usize, 1, 15, 16, 17, 64, 1024] {
        let mut a = rng.bytes(len + CANARY);
        let mut b = a.clone();
        unsafe {
            c(a.as_mut_ptr() as _, len);
            r(b.as_mut_ptr() as _, len);
        }
        eqb(&format!("sodium_memzero len={len}"), &a, &b);
    }
    // stackzero has no observable output; assert it does not crash in either.
    let (c, r) = sym::<unsafe extern "C" fn(usize)>("sodium_stackzero");
    unsafe {
        c(0);
        r(0);
        c(256);
        r(256);
    }
}

// ---------------------------------------------------------------------------
// utils.c — sodium_pad / sodium_unpad (PA62–PA65, A45–A46)
// ---------------------------------------------------------------------------

#[test]
fn sodium_pad_unpad_roundtrip_and_errors() {
    type PAD = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
    type UNPAD = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
    let (cp, rp) = sym::<PAD>("sodium_pad");
    let (cu, ru) = sym::<UNPAD>("sodium_unpad");
    let mut rng = Rng::new(SEED ^ 6);

    for blocksize in [1usize, 2, 3, 8, 16, 17, 64, 256] {
        for unpadded in 0usize..=(blocksize * 3 + 2) {
            let max = unpadded + blocksize + 1;
            let src = rng.bytes(unpadded);

            let mut bc = vec![0u8; max + CANARY];
            let mut br = bc.clone();
            bc[..unpadded].copy_from_slice(&src);
            br[..unpadded].copy_from_slice(&src);
            let mut lc = usize::MAX;
            let mut lr = usize::MAX;
            let (rc, rr) = unsafe {
                (
                    cp(&mut lc, bc.as_mut_ptr(), unpadded, blocksize, max),
                    rp(&mut lr, br.as_mut_ptr(), unpadded, blocksize, max),
                )
            };
            assert_eq!(rc, rr, "sodium_pad rc bs={blocksize} n={unpadded}");
            assert_eq!(lc, lr, "sodium_pad len bs={blocksize} n={unpadded}");
            eqb(&format!("sodium_pad buf bs={blocksize} n={unpadded}"), &bc, &br);

            if rc == 0 {
                let mut uc = usize::MAX;
                let mut ur = usize::MAX;
                let (xc, xr) = unsafe {
                    (
                        cu(&mut uc, bc.as_ptr(), lc, blocksize),
                        ru(&mut ur, br.as_ptr(), lr, blocksize),
                    )
                };
                assert_eq!(xc, xr, "sodium_unpad rc bs={blocksize} n={unpadded}");
                assert_eq!(uc, ur, "sodium_unpad len bs={blocksize} n={unpadded}");
            }

            // A45: max_buflen too small for the padded result.
            let mut lc = 0usize;
            let mut lr = 0usize;
            let (rc, rr) = unsafe {
                (
                    cp(&mut lc, bc.as_mut_ptr(), unpadded, blocksize, unpadded),
                    rp(&mut lr, br.as_mut_ptr(), unpadded, blocksize, unpadded),
                )
            };
            assert_eq!(rc, rr, "sodium_pad tight-max bs={blocksize} n={unpadded}");
        }
    }

    // A46 / error paths for unpad: blocksize 0, padded_buflen 0,
    // padded_buflen < blocksize, and corrupted padding.
    let buf = vec![0x80u8; 64];
    for (pl, bs) in [
        (0usize, 16usize),
        (16, 0),
        (0, 0),
        (8, 16),
        (16, 16),
        (17, 16),
        (64, 16),
        (64, 65),
    ] {
        let mut uc = 12345usize;
        let mut ur = 12345usize;
        let (xc, xr) = unsafe { (cu(&mut uc, buf.as_ptr(), pl, bs), ru(&mut ur, buf.as_ptr(), pl, bs)) };
        assert_eq!(xc, xr, "sodium_unpad rc pl={pl} bs={bs}");
        if xc == 0 {
            assert_eq!(uc, ur, "sodium_unpad len pl={pl} bs={bs}");
        }
    }
    // all-zero buffer => no 0x80 terminator anywhere => must be rejected
    let zeros = vec![0u8; 64];
    for bs in [1usize, 16, 32, 64] {
        let mut uc = 0usize;
        let mut ur = 0usize;
        let (xc, xr) =
            unsafe { (cu(&mut uc, zeros.as_ptr(), 64, bs), ru(&mut ur, zeros.as_ptr(), 64, bs)) };
        assert_eq!(xc, xr, "sodium_unpad zeros bs={bs}");
    }
    // sodium_pad with blocksize == 0 (A45 sibling)
    let mut b = vec![0u8; 64];
    let mut lc = 0usize;
    let mut lr = 0usize;
    let (rc, rr) = unsafe { (cp(&mut lc, b.as_mut_ptr(), 8, 0, 64), rp(&mut lr, b.as_mut_ptr(), 8, 0, 64)) };
    assert_eq!(rc, rr, "sodium_pad blocksize=0");
}

// ---------------------------------------------------------------------------
// utils.c — mlock/munlock/mprotect stubs, malloc/allocarray/free
// (PA66–PA73, A32–A44)
// ---------------------------------------------------------------------------

#[test]
fn sodium_mlock_munlock_mprotect_stubs() {
    let mut buf = vec![0u8; 4096];
    for name in ["sodium_mlock", "sodium_munlock"] {
        let (c, r) = sym::<unsafe extern "C" fn(*mut c_void, usize) -> c_int>(name);
        unsafe {
            set_errno(0);
            let rc = c(buf.as_mut_ptr() as _, buf.len());
            let ec = errno();
            set_errno(0);
            let rr = r(buf.as_mut_ptr() as _, buf.len());
            let er = errno();
            assert_eq!(rc, rr, "{name} rc");
            assert_eq!(ec, er, "{name} errno");
        }
    }
    for name in [
        "sodium_mprotect_noaccess",
        "sodium_mprotect_readonly",
        "sodium_mprotect_readwrite",
    ] {
        let (c, r) = sym::<unsafe extern "C" fn(*mut c_void) -> c_int>(name);
        unsafe {
            set_errno(0);
            let rc = c(buf.as_mut_ptr() as _);
            let ec = errno();
            set_errno(0);
            let rr = r(buf.as_mut_ptr() as _);
            let er = errno();
            assert_eq!(rc, rr, "{name} rc");
            assert_eq!(ec, er, "{name} errno");
        }
    }
}

unsafe fn errno() -> c_int {
    *libc::__errno_location()
}
unsafe fn set_errno(v: c_int) {
    *libc::__errno_location() = v;
}

#[test]
fn sodium_malloc_free_allocarray() {
    let (cm, rm) = sym::<unsafe extern "C" fn(usize) -> *mut c_void>("sodium_malloc");
    let (cf, rf) = sym::<unsafe extern "C" fn(*mut c_void)>("sodium_free");
    let (ca, ra) = sym::<unsafe extern "C" fn(usize, usize) -> *mut c_void>("sodium_allocarray");

    // Each library must allocate a usable region for the same sizes, and each
    // must free its OWN pointer (canary layout is per-implementation).
    for size in [0usize, 1, 15, 16, 4095, 4096, 4097, 1 << 16] {
        unsafe {
            let pc = cm(size);
            let pr = rm(size);
            assert_eq!(pc.is_null(), pr.is_null(), "sodium_malloc({size}) nullness");
            if !pc.is_null() {
                // writable for `size` bytes in both
                ptr::write_bytes(pc as *mut u8, 0x5a, size);
                ptr::write_bytes(pr as *mut u8, 0x5a, size);
                cf(pc);
                rf(pr);
            }
        }
    }
    // sodium_free(NULL) must be a no-op in both.
    unsafe {
        cf(ptr::null_mut());
        rf(ptr::null_mut());
    }
    // allocarray, including the A37 overflow rejection.
    for (n, s) in [
        (0usize, 0usize),
        (1, 0),
        (0, 1),
        (1, 1),
        (16, 16),
        (1024, 8),
        (usize::MAX, 2),
        (2, usize::MAX),
        (usize::MAX, usize::MAX),
        (1 << 40, 1 << 40),
    ] {
        unsafe {
            set_errno(0);
            let pc = ca(n, s);
            let ec = errno();
            set_errno(0);
            let pr = ra(n, s);
            let er = errno();
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "sodium_allocarray({n},{s}) nullness (C null={}, Rust null={})",
                pc.is_null(),
                pr.is_null()
            );
            if pc.is_null() {
                assert_eq!(ec, er, "sodium_allocarray({n},{s}) errno");
            } else {
                cf(pc);
                rf(pr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// codecs.c — hex (PA1–PA10, A9–A13)
// ---------------------------------------------------------------------------

#[test]
fn sodium_bin2hex_all_shapes() {
    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
    let (c, r) = sym::<F>("sodium_bin2hex");
    let mut rng = Rng::new(SEED ^ 7);
    for bin_len in 0usize..=64 {
        for _ in 0..8 {
            let bin = rng.bytes(bin_len);
            let maxlen = bin_len * 2 + 1;
            let mut hc = out_buf(maxlen);
            let mut hr = out_buf(maxlen);
            unsafe {
                let pc = c(hc.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len);
                let pr = r(hr.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len);
                assert_eq!(pc as usize - hc.as_ptr() as usize, 0);
                assert_eq!(pr as usize - hr.as_ptr() as usize, 0);
            }
            eqb(&format!("sodium_bin2hex bin_len={bin_len}"), &hc, &hr);
            // oversized output buffer must produce identical bytes too
            let big = maxlen + 7;
            let mut hc = out_buf(big);
            let mut hr = out_buf(big);
            unsafe {
                c(hc.as_mut_ptr() as _, big, bin.as_ptr(), bin_len);
                r(hr.as_mut_ptr() as _, big, bin.as_ptr(), bin_len);
            }
            eqb(&format!("sodium_bin2hex oversized bin_len={bin_len}"), &hc, &hr);
        }
    }
}

#[test]
fn sodium_bin2hex_buffer_too_small_aborts_identically() {
    // A10: hex_maxlen <= bin_len*2 -> sodium_misuse() -> SIGABRT
    for (bin_len, maxlen) in [(1usize, 0usize), (1, 1), (1, 2), (4, 8), (32, 64), (32, 1)] {
        same_outcome(
            &format!("sodium_bin2hex too-small bin_len={bin_len} maxlen={maxlen}"),
            || {
                let (c, _) =
                    sym::<unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char>(
                        "sodium_bin2hex",
                    );
                let bin = vec![0xabu8; bin_len];
                let mut o = vec![0u8; maxlen.max(1) + 64];
                unsafe { c(o.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len) };
                0
            },
            || {
                let (_, r) =
                    sym::<unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char>(
                        "sodium_bin2hex",
                    );
                let bin = vec![0xabu8; bin_len];
                let mut o = vec![0u8; maxlen.max(1) + 64];
                unsafe { r(o.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len) };
                0
            },
        );
    }
}

type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> c_int;

#[allow(clippy::too_many_arguments)]
fn hex2bin_case(
    label: &str,
    bin_maxlen: usize,
    hex_s: &[u8],
    ignore: Option<&[u8]>,
    want_end: bool,
) {
    let (c, r) = sym::<Hex2Bin>("sodium_hex2bin");
    let ig = ignore.map(|s| {
        let mut v = s.to_vec();
        v.push(0);
        v
    });
    let igp = ig.as_ref().map_or(ptr::null(), |v| v.as_ptr() as *const c_char);

    let mut bc = out_buf(bin_maxlen);
    let mut br = out_buf(bin_maxlen);
    let mut lc = usize::MAX;
    let mut lr = usize::MAX;
    let mut ec: *const c_char = ptr::null();
    let mut er: *const c_char = ptr::null();
    unsafe {
        set_errno(0);
        let rc = c(
            bc.as_mut_ptr(),
            bin_maxlen,
            hex_s.as_ptr() as _,
            hex_s.len(),
            igp,
            &mut lc,
            if want_end { &mut ec } else { ptr::null_mut() },
        );
        let en_c = errno();
        set_errno(0);
        let rr = r(
            br.as_mut_ptr(),
            bin_maxlen,
            hex_s.as_ptr() as _,
            hex_s.len(),
            igp,
            &mut lr,
            if want_end { &mut er } else { ptr::null_mut() },
        );
        let en_r = errno();
        assert_eq!(rc, rr, "{label}: rc");
        assert_eq!(en_c, en_r, "{label}: errno");
        assert_eq!(lc, lr, "{label}: bin_len");
        if want_end {
            let oc = if ec.is_null() { usize::MAX } else { ec as usize - hex_s.as_ptr() as usize };
            let or = if er.is_null() { usize::MAX } else { er as usize - hex_s.as_ptr() as usize };
            assert_eq!(oc, or, "{label}: hex_end offset");
        }
        eqb(&format!("{label}: bin"), &bc, &br);
    }
}

#[test]
fn sodium_hex2bin_valid_and_invalid() {
    let mut rng = Rng::new(SEED ^ 8);
    // PA3–PA6: valid hex of every length, upper/lower/mixed case.
    for n in 0usize..=48 {
        for _ in 0..6 {
            let bin = rng.bytes(n);
            let lower: String = bin.iter().map(|b| format!("{b:02x}")).collect();
            let upper: String = bin.iter().map(|b| format!("{b:02X}")).collect();
            let mixed: String = bin
                .iter()
                .enumerate()
                .map(|(i, b)| if i % 2 == 0 { format!("{b:02x}") } else { format!("{b:02X}") })
                .collect();
            for (tag, s) in [("lower", &lower), ("upper", &upper), ("mixed", &mixed)] {
                for maxlen in [n, n + 1, n.saturating_sub(1)] {
                    hex2bin_case(
                        &format!("hex2bin {tag} n={n} maxlen={maxlen}"),
                        maxlen,
                        s.as_bytes(),
                        None,
                        true,
                    );
                    hex2bin_case(
                        &format!("hex2bin {tag} n={n} maxlen={maxlen} no-end"),
                        maxlen,
                        s.as_bytes(),
                        None,
                        false,
                    );
                }
            }
        }
    }

    // PA7–PA10 + A11–A13: ignore sets, odd nibbles, invalid chars, overflow.
    let cases: &[(&[u8], Option<&[u8]>)] = &[
        (b"", None),
        (b"0", None),                 // A12 odd
        (b"abc", None),               // A12 odd
        (b"zz", None),                // A13 invalid char
        (b"00zz", None),              // stops early
        (b"de:ad:be:ef", Some(b":")), // PA7 ignore
        (b"de:ad:be:ef", None),       // A13 without ignore
        (b"de ad\tbe\nef", Some(b" \t\n")),
        (b" deadbeef", Some(b" ")),
        (b"deadbeef ", Some(b" ")),
        (b"dead beef", Some(b"")),
        (b"::::", Some(b":")),
        (b"0:0", Some(b":")),
        (b"0:", Some(b":")),
        (b":0", Some(b":")),
        (b"00", Some(b"0")), // ignore char that is also a hex digit
        (b"0x00", Some(b"x")),
        (b"ff", None),
        (b"FF", None),
        (b"fF", None),
        (b"\0ff", None),
        (b"f\0f", None),
    ];
    for (s, ig) in cases {
        for maxlen in [0usize, 1, 2, 3, 8, 64] {
            hex2bin_case(
                &format!("hex2bin {:?} ignore={:?} maxlen={maxlen}", String::from_utf8_lossy(s), ig.map(String::from_utf8_lossy)),
                maxlen,
                s,
                *ig,
                true,
            );
            hex2bin_case(
                &format!("hex2bin-noend {:?} ignore={:?} maxlen={maxlen}", String::from_utf8_lossy(s), ig.map(String::from_utf8_lossy)),
                maxlen,
                s,
                *ig,
                false,
            );
        }
    }

    // Fuzz: random byte strings over a hex-ish alphabet, random ignore set.
    let alphabet = b"0123456789abcdefABCDEF: \t\n=xz\0";
    for _ in 0..4000 {
        let n = rng.below(20);
        let s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let ig: &[u8] = match rng.below(4) {
            0 => b":",
            1 => b" \t\n",
            2 => b"",
            _ => b":= ",
        };
        let use_ig = rng.below(2) == 0;
        hex2bin_case(
            &format!("hex2bin fuzz {:?}", s),
            rng.below(12),
            &s,
            if use_ig { Some(ig) } else { None },
            rng.below(2) == 0,
        );
    }
}

// ---------------------------------------------------------------------------
// codecs.c — base64, all four variants (PA11–PA33, A14–A22)
// ---------------------------------------------------------------------------

const B64_VARIANTS: [(c_int, &str); 4] = [
    (1, "ORIGINAL"),
    (3, "ORIGINAL_NO_PADDING"),
    (5, "URLSAFE"),
    (7, "URLSAFE_NO_PADDING"),
];

#[test]
fn sodium_base64_encoded_len_all_variants() {
    let (c, r) = sym::<unsafe extern "C" fn(usize, c_int) -> usize>("sodium_base64_encoded_len");
    for (v, name) in B64_VARIANTS {
        for n in 0usize..=200 {
            unsafe {
                assert_eq!(c(n, v), r(n, v), "encoded_len({n}, {name})");
            }
        }
        for n in [1000usize, 4096, 65535, 1 << 20] {
            unsafe {
                assert_eq!(c(n, v), r(n, v), "encoded_len({n}, {name})");
            }
        }
    }
}

#[test]
fn sodium_bin2base64_all_variants() {
    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
    let (c, r) = sym::<F>("sodium_bin2base64");
    let (cl, _) = sym::<unsafe extern "C" fn(usize, c_int) -> usize>("sodium_base64_encoded_len");
    let mut rng = Rng::new(SEED ^ 9);
    for (v, name) in B64_VARIANTS {
        for bin_len in 0usize..=90 {
            for _ in 0..5 {
                let bin = rng.bytes(bin_len);
                let maxlen = unsafe { cl(bin_len, v) };
                let mut bc = out_buf(maxlen);
                let mut br = out_buf(maxlen);
                unsafe {
                    c(bc.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len, v);
                    r(br.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len, v);
                }
                eqb(&format!("bin2base64 {name} bin_len={bin_len}"), &bc, &br);
                // slack in the output buffer
                let mut bc = out_buf(maxlen + 5);
                let mut br = out_buf(maxlen + 5);
                unsafe {
                    c(bc.as_mut_ptr() as _, maxlen + 5, bin.as_ptr(), bin_len, v);
                    r(br.as_mut_ptr() as _, maxlen + 5, bin.as_ptr(), bin_len, v);
                }
                eqb(&format!("bin2base64 {name} slack bin_len={bin_len}"), &bc, &br);
            }
        }
    }
}

type B642Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    c_int,
) -> c_int;

fn b642bin_case(label: &str, bin_maxlen: usize, s: &[u8], ignore: Option<&[u8]>, variant: c_int, want_end: bool) {
    let (c, r) = sym::<B642Bin>("sodium_base642bin");
    let ig = ignore.map(|x| {
        let mut v = x.to_vec();
        v.push(0);
        v
    });
    let igp = ig.as_ref().map_or(ptr::null(), |v| v.as_ptr() as *const c_char);
    let mut bc = out_buf(bin_maxlen);
    let mut br = out_buf(bin_maxlen);
    let mut lc = usize::MAX;
    let mut lr = usize::MAX;
    let mut ec: *const c_char = ptr::null();
    let mut er: *const c_char = ptr::null();
    unsafe {
        set_errno(0);
        let rc = c(
            bc.as_mut_ptr(),
            bin_maxlen,
            s.as_ptr() as _,
            s.len(),
            igp,
            &mut lc,
            if want_end { &mut ec } else { ptr::null_mut() },
            variant,
        );
        let en_c = errno();
        set_errno(0);
        let rr = r(
            br.as_mut_ptr(),
            bin_maxlen,
            s.as_ptr() as _,
            s.len(),
            igp,
            &mut lr,
            if want_end { &mut er } else { ptr::null_mut() },
            variant,
        );
        let en_r = errno();
        assert_eq!(rc, rr, "{label}: rc");
        assert_eq!(en_c, en_r, "{label}: errno");
        assert_eq!(lc, lr, "{label}: bin_len");
        if want_end {
            let oc = if ec.is_null() { usize::MAX } else { ec as usize - s.as_ptr() as usize };
            let or = if er.is_null() { usize::MAX } else { er as usize - s.as_ptr() as usize };
            assert_eq!(oc, or, "{label}: b64_end offset");
        }
        eqb(&format!("{label}: bin"), &bc, &br);
    }
}

#[test]
fn sodium_base642bin_roundtrip_all_variants() {
    type ENC = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
    let (cenc, _) = sym::<ENC>("sodium_bin2base64");
    let (cl, _) = sym::<unsafe extern "C" fn(usize, c_int) -> usize>("sodium_base64_encoded_len");
    let mut rng = Rng::new(SEED ^ 10);
    for (v, name) in B64_VARIANTS {
        for bin_len in 0usize..=70 {
            for _ in 0..4 {
                let bin = rng.bytes(bin_len);
                let maxlen = unsafe { cl(bin_len, v) };
                let mut enc = vec![0u8; maxlen];
                unsafe { cenc(enc.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len, v) };
                let s = CStr::from_bytes_until_nul(&enc).unwrap().to_bytes().to_vec();
                for maxbin in [bin_len, bin_len + 1, bin_len.saturating_sub(1), 0] {
                    b642bin_case(
                        &format!("base642bin {name} n={bin_len} maxbin={maxbin}"),
                        maxbin,
                        &s,
                        None,
                        v,
                        true,
                    );
                    b642bin_case(
                        &format!("base642bin-noend {name} n={bin_len} maxbin={maxbin}"),
                        maxbin,
                        &s,
                        None,
                        v,
                        false,
                    );
                }
                // decode under EVERY variant, not just the encoding one:
                // cross-variant decoding is a real error path.
                for (v2, n2) in B64_VARIANTS {
                    b642bin_case(
                        &format!("base642bin cross {name}->{n2} n={bin_len}"),
                        bin_len + 2,
                        &s,
                        None,
                        v2,
                        true,
                    );
                }
            }
        }
    }
}

#[test]
fn sodium_base642bin_malformed() {
    let cases: &[&[u8]] = &[
        b"",
        b"=",
        b"==",
        b"===",
        b"====",
        b"A",
        b"A=",
        b"A==",
        b"A===",
        b"AA",
        b"AA=",
        b"AA==",
        b"AA===",
        b"AAA",
        b"AAA=",
        b"AAAA",
        b"AAAAA",
        b"AAAA=",
        b"AAAA==",
        b"AB==",
        b"AC==",
        b"AQ==",
        b"AH==",  // trailing non-zero bits -> A18
        b"AB=",
        b"/w==",
        b"_w==",
        b"-w==",
        b"+w==",
        b"a b",
        b"a\nb",
        b"****",
        b"AAAA****",
        b"AA=A",
        b"=AAA",
        b"\0AAA",
        b"AA\0AA",
        b"AAAAAAAA",
        b"AAAAAAA=",
        b"AAAAAA==",
    ];
    for (v, name) in B64_VARIANTS {
        for s in cases {
            for ig in [None, Some(&b" \n"[..]), Some(&b""[..])] {
                for maxbin in [0usize, 1, 2, 3, 6, 32] {
                    b642bin_case(
                        &format!("base642bin bad {name} {:?} ig={:?} maxbin={maxbin}", String::from_utf8_lossy(s), ig),
                        maxbin,
                        s,
                        ig,
                        v,
                        true,
                    );
                }
            }
        }
    }
    // Fuzz over the union alphabet of both variants plus junk.
    let alphabet = b"ABCXYZabcxyz0189+/-_= \n\t*\0";
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..6000 {
        let n = rng.below(14);
        let s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let (v, _) = B64_VARIANTS[rng.below(4)];
        let ig: Option<&[u8]> = match rng.below(3) {
            0 => None,
            1 => Some(b" \n\t"),
            _ => Some(b""),
        };
        b642bin_case(&format!("base642bin fuzz {:?}", s), rng.below(12), &s, ig, v, rng.below(2) == 0);
    }
}

#[test]
fn sodium_base64_invalid_variant_aborts_identically() {
    // A14: invalid variant -> sodium_base64_check_variant -> sodium_misuse()
    for v in [0i32, 2, 4, 6, 8, 9, -1, 100, i32::MIN, i32::MAX] {
        same_outcome(
            &format!("sodium_base64_encoded_len bad variant {v}"),
            move || {
                let (c, _) = sym::<unsafe extern "C" fn(usize, c_int) -> usize>("sodium_base64_encoded_len");
                unsafe { c(10, v) as i32 & 0x7f }
            },
            move || {
                let (_, r) = sym::<unsafe extern "C" fn(usize, c_int) -> usize>("sodium_base64_encoded_len");
                unsafe { r(10, v) as i32 & 0x7f }
            },
        );
        same_outcome(
            &format!("sodium_bin2base64 bad variant {v}"),
            move || {
                type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
                let (c, _) = sym::<F>("sodium_bin2base64");
                let bin = [1u8, 2, 3];
                let mut o = [0u8; 64];
                unsafe { c(o.as_mut_ptr() as _, 64, bin.as_ptr(), 3, v) };
                0
            },
            move || {
                type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
                let (_, r) = sym::<F>("sodium_bin2base64");
                let bin = [1u8, 2, 3];
                let mut o = [0u8; 64];
                unsafe { r(o.as_mut_ptr() as _, 64, bin.as_ptr(), 3, v) };
                0
            },
        );
        same_outcome(
            &format!("sodium_base642bin bad variant {v}"),
            move || {
                let (c, _) = sym::<B642Bin>("sodium_base642bin");
                let s = b"AAAA\0";
                let mut o = [0u8; 64];
                let mut l = 0usize;
                unsafe { c(o.as_mut_ptr(), 64, s.as_ptr() as _, 4, ptr::null(), &mut l, ptr::null_mut(), v) }
            },
            move || {
                let (_, r) = sym::<B642Bin>("sodium_base642bin");
                let s = b"AAAA\0";
                let mut o = [0u8; 64];
                let mut l = 0usize;
                unsafe { r(o.as_mut_ptr(), 64, s.as_ptr() as _, 4, ptr::null(), &mut l, ptr::null_mut(), v) }
            },
        );
    }
}

#[test]
fn sodium_bin2base64_buffer_too_small_aborts_identically() {
    // A17
    for (v, name) in B64_VARIANTS {
        for (bin_len, maxlen) in [(1usize, 0usize), (1, 1), (1, 2), (3, 4), (3, 3), (32, 8)] {
            same_outcome(
                &format!("bin2base64 too-small {name} bin_len={bin_len} maxlen={maxlen}"),
                move || {
                    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
                    let (c, _) = sym::<F>("sodium_bin2base64");
                    let bin = vec![0xa5u8; bin_len];
                    let mut o = vec![0u8; 256];
                    unsafe { c(o.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len, v) };
                    0
                },
                move || {
                    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
                    let (_, r) = sym::<F>("sodium_bin2base64");
                    let bin = vec![0xa5u8; bin_len];
                    let mut o = vec![0u8; 256];
                    unsafe { r(o.as_mut_ptr() as _, maxlen, bin.as_ptr(), bin_len, v) };
                    0
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// codecs.c — IP conversion (PA34–PA46, A23–A30)
// ---------------------------------------------------------------------------

#[test]
fn sodium_ip2bin_all_shapes() {
    type F = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
    let (c, r) = sym::<F>("sodium_ip2bin");
    let inputs: &[&str] = &[
        "0.0.0.0",
        "127.0.0.1",
        "255.255.255.255",
        "1.2.3.4",
        "01.02.03.04",
        "001.002.003.004",
        "0001.2.3.4",
        "256.1.1.1",
        "1.1.1",
        "1.1.1.1.1",
        "1.1.1.",
        ".1.1.1",
        "1..1.1",
        "1.2.3.4:80",
        "1.2.3.4:0",
        "1.2.3.4:65535",
        "1.2.3.4:65536",
        "1.2.3.4:",
        "",
        " ",
        "::",
        "::1",
        "1::",
        "::ffff:1.2.3.4",
        "::ffff:0102:0304",
        "2001:db8::1",
        "2001:0db8:0000:0000:0000:0000:0000:0001",
        "fe80::1%eth0",
        "fe80::1%",
        "fe80::1%!!",
        "fe80::1%0",
        "fe80::1%eth0.5",
        "fe80::1%eth-0_1",
        "1.2.3.4%eth0",
        "[::1]",
        "[::1]:80",
        "[2001:db8::1]:443",
        "1:2:3:4:5:6:7:8",
        "1:2:3:4:5:6:7:8:9",
        "1:2:3:4:5:6:7",
        "1:::2",
        "::::",
        "gggg::1",
        "12345::1",
        "-1.2.3.4",
        "+1.2.3.4",
        "1.2.3.4 ",
        " 1.2.3.4",
        "\t1.2.3.4",
        "1.2.3.4\0extra",
        "0:0:0:0:0:ffff:7f00:1",
    ];
    for ip in inputs {
        for &len in &[ip.len(), ip.len() + 1, ip.len().saturating_sub(1), 0] {
            let mut s = ip.as_bytes().to_vec();
            s.push(0);
            let mut bc = out_buf(16);
            let mut br = out_buf(16);
            unsafe {
                let rc = c(bc.as_mut_ptr(), s.as_ptr() as _, len);
                let rr = r(br.as_mut_ptr(), s.as_ptr() as _, len);
                assert_eq!(rc, rr, "sodium_ip2bin({ip:?}, len={len}) rc");
            }
            eqb(&format!("sodium_ip2bin({ip:?}, len={len}) bin"), &bc, &br);
        }
    }
}

#[test]
fn sodium_bin2ip_all_shapes() {
    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
    let (c, r) = sym::<F>("sodium_bin2ip");
    let mut rng = Rng::new(SEED ^ 12);
    let mut cases: Vec<[u8; 16]> = vec![
        [0u8; 16],
        [0xffu8; 16],
        {
            let mut v = [0u8; 16];
            v[15] = 1;
            v
        },
        {
            // IPv4-mapped 1.2.3.4
            let mut v = [0u8; 16];
            v[10] = 0xff;
            v[11] = 0xff;
            v[12] = 1;
            v[13] = 2;
            v[14] = 3;
            v[15] = 4;
            v
        },
        {
            // IPv4-mapped 0.0.0.0
            let mut v = [0u8; 16];
            v[10] = 0xff;
            v[11] = 0xff;
            v
        },
        {
            // IPv4-compatible (no ffff)
            let mut v = [0u8; 16];
            v[12] = 127;
            v[15] = 1;
            v
        },
    ];
    for _ in 0..400 {
        let mut v = [0u8; 16];
        rng.fill(&mut v);
        cases.push(v);
        // long zero runs, to hit "::" compression selection
        let mut v = [0u8; 16];
        let start = rng.below(14);
        let n = 1 + rng.below(16 - start);
        for i in start..start + n {
            v[i] = rng.byte();
        }
        cases.push(v);
    }
    for bin in &cases {
        for maxlen in [0usize, 1, 2, 3, 4, 8, 10, 15, 16, 17, 39, 40, 46, 64] {
            let mut sc = out_buf(maxlen);
            let mut sr = out_buf(maxlen);
            unsafe {
                let pc = c(sc.as_mut_ptr() as _, maxlen, bin.as_ptr());
                let pr = r(sr.as_mut_ptr() as _, maxlen, bin.as_ptr());
                assert_eq!(
                    pc.is_null(),
                    pr.is_null(),
                    "sodium_bin2ip({}, maxlen={maxlen}) nullness",
                    hex(bin)
                );
            }
            eqb(&format!("sodium_bin2ip({}, maxlen={maxlen})", hex(bin)), &sc, &sr);
        }
    }
}

#[test]
fn ip_roundtrip_bin2ip_then_ip2bin() {
    type ENC = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
    type DEC = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
    let (cenc, renc) = sym::<ENC>("sodium_bin2ip");
    let (cdec, rdec) = sym::<DEC>("sodium_ip2bin");
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..2000 {
        let mut bin = [0u8; 16];
        rng.fill(&mut bin);
        if rng.below(3) == 0 {
            // make it IPv4-mapped sometimes
            bin[..10].fill(0);
            bin[10] = 0xff;
            bin[11] = 0xff;
        }
        if rng.below(4) == 0 {
            let start = rng.below(12);
            for i in start..(start + 4).min(16) {
                bin[i] = 0;
            }
        }
        let mut sc = vec![0u8; 64];
        let mut sr = vec![0u8; 64];
        unsafe {
            cenc(sc.as_mut_ptr() as _, 64, bin.as_ptr());
            renc(sr.as_mut_ptr() as _, 64, bin.as_ptr());
        }
        eqb("bin2ip text", &sc, &sr);
        let text = CStr::from_bytes_until_nul(&sc).unwrap();
        let n = text.to_bytes().len();
        let mut b2c = out_buf(16);
        let mut b2r = out_buf(16);
        unsafe {
            let rc = cdec(b2c.as_mut_ptr(), text.as_ptr(), n);
            let rr = rdec(b2r.as_mut_ptr(), text.as_ptr(), n);
            assert_eq!(rc, rr, "ip2bin({:?}) rc", text);
        }
        eqb(&format!("ip2bin({:?}) bin", text), &b2c, &b2r);
    }
}
