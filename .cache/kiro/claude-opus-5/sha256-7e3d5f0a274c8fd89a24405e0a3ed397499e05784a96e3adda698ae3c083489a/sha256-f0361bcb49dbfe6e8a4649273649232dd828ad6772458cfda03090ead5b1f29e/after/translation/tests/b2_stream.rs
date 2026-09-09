//! Phase B — valid-path differential tests, CONFIGS.md group 2, part 1:
//! crypto_stream_* (rows 104-115)

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

pub const LENS: &[usize] = &[
    0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 1000, 4096,
];

/// keystream + xor + (optional) xor_ic for one stream primitive.
fn stream_family(prefix: &str, noncebytes: usize, ic64: bool, has_ic: bool) {
    let (kbc, kbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_keybytes"));
    let keybytes = unsafe { kbc() };
    unsafe { assert_eq!(keybytes, kbr(), "{prefix}_keybytes") };
    let (nbc, nbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_noncebytes"));
    unsafe { assert_eq!(nbc(), nbr(), "{prefix}_noncebytes") };
    assert_eq!(unsafe { nbc() }, noncebytes, "{prefix} noncebytes constant");

    let (ksc, ksr) = pair::<
        unsafe extern "C" fn(*mut u8, c_ulonglong, *const u8, *const u8) -> c_int,
    >(prefix);
    let (xc, xr) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int,
    >(&format!("{prefix}_xor"));

    let mut rng = Rng::new(SEED ^ prefix.len() as u64 ^ noncebytes as u64);
    for &l in LENS {
        for _ in 0..6 {
            let k = rng.bytes(keybytes);
            let n = rng.bytes(noncebytes);
            let m = rng.bytes(l);

            let mut a = buf(l + 8);
            let mut b = buf(l + 8);
            unsafe {
                same_ret(
                    prefix,
                    ksc(a.as_mut_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    ksr(b.as_mut_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes(&format!("{prefix} keystream len={l}"), &a, &b);

            let mut a = buf(l + 8);
            let mut b = buf(l + 8);
            unsafe {
                same_ret(
                    &format!("{prefix}_xor"),
                    xc(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    xr(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes(&format!("{prefix}_xor len={l}"), &a, &b);

            // in-place xor (c == m) — a real consumer pattern
            let mut a = m.clone();
            let mut b = m.clone();
            a.resize(l + 8, CANARY);
            b.resize(l + 8, CANARY);
            unsafe {
                xc(a.as_mut_ptr(), a.as_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr());
                xr(b.as_mut_ptr(), b.as_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr());
            }
            same_bytes(&format!("{prefix}_xor in-place len={l}"), &a, &b);

            if has_ic {
                let name = format!("{prefix}_xor_ic");
                if ic64 {
                    let (c, r) = pair::<
                        unsafe extern "C" fn(
                            *mut u8,
                            *const u8,
                            c_ulonglong,
                            *const u8,
                            u64,
                            *const u8,
                        ) -> c_int,
                    >(&name);
                    for &ic in &[
                        0u64,
                        1,
                        2,
                        0xffff_ffff,
                        0x1_0000_0000,
                        0xffff_ffff_ffff_fffe,
                        u64::MAX,
                    ] {
                        let mut a = buf(l + 8);
                        let mut b = buf(l + 8);
                        unsafe {
                            same_ret(
                                &name,
                                c(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), ic, k.as_ptr()),
                                r(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), ic, k.as_ptr()),
                            );
                        }
                        same_bytes(&format!("{name} len={l} ic={ic}"), &a, &b);
                    }
                } else {
                    let (c, r) = pair::<
                        unsafe extern "C" fn(
                            *mut u8,
                            *const u8,
                            c_ulonglong,
                            *const u8,
                            u32,
                            *const u8,
                        ) -> c_int,
                    >(&name);
                    // largest legal ic for this mlen (one more triggers misuse — Phase C)
                    let blocks = ((l as u64) + 63) / 64;
                    let max_ic = (1u64 << 32) - blocks;
                    let mut ics: Vec<u32> = vec![0, 1, 2, 1000];
                    if max_ic <= u32::MAX as u64 {
                        ics.push(max_ic as u32);
                        if max_ic > 0 {
                            ics.push((max_ic - 1) as u32);
                        }
                    }
                    for ic in ics {
                        let mut a = buf(l + 8);
                        let mut b = buf(l + 8);
                        unsafe {
                            same_ret(
                                &name,
                                c(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), ic, k.as_ptr()),
                                r(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), ic, k.as_ptr()),
                            );
                        }
                        same_bytes(&format!("{name} len={l} ic={ic}"), &a, &b);
                    }
                }
            }
        }
    }
    // keygen only exercises length; messagebytes_max / primitive getters
    let g = format!("{prefix}_keygen");
    if libs().has(&g) {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8)>(&g);
        let mut a = buf(keybytes);
        let mut b = buf(keybytes);
        unsafe {
            c(a.as_mut_ptr());
            r(b.as_mut_ptr());
        }
    }
    let g = format!("{prefix}_messagebytes_max");
    if libs().has(&g) {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
}

#[test]
fn row104_crypto_stream_generic() {
    stream_family("crypto_stream", 24, false, false);
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_stream_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_stream_primitive", &a, &b);
    }
}

#[test]
fn rows105_109_salsa() {
    stream_family("crypto_stream_salsa20", 8, true, true);
    stream_family("crypto_stream_salsa2012", 8, false, false);
    stream_family("crypto_stream_salsa208", 8, false, false);
}

#[test]
fn row110_xsalsa20() {
    stream_family("crypto_stream_xsalsa20", 24, true, true);
}

#[test]
fn rows111_112_chacha20() {
    stream_family("crypto_stream_chacha20", 8, true, true);
    stream_family("crypto_stream_chacha20_ietf", 12, false, true);
}

#[test]
fn row113_chacha20_ietf_ext() {
    // Low-level internal-but-exported entry points used by xchacha and
    // secretstream. No ic overflow check here.
    let (ksc, ksr) = pair::<
        unsafe extern "C" fn(*mut u8, c_ulonglong, *const u8, *const u8) -> c_int,
    >("crypto_stream_chacha20_ietf_ext");
    let (xc, xr) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, u32, *const u8) -> c_int,
    >("crypto_stream_chacha20_ietf_ext_xor_ic");
    let mut rng = Rng::seeded();
    for &l in LENS {
        for _ in 0..6 {
            let k = rng.bytes(32);
            let n = rng.bytes(12);
            let m = rng.bytes(l);
            let mut a = buf(l + 8);
            let mut b = buf(l + 8);
            unsafe {
                same_ret(
                    "ietf_ext",
                    ksc(a.as_mut_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr()),
                    ksr(b.as_mut_ptr(), l as c_ulonglong, n.as_ptr(), k.as_ptr()),
                );
            }
            same_bytes(&format!("ietf_ext keystream len={l}"), &a, &b);
            for &ic in &[0u32, 1, 2, 0xffff_ffff, 0xffff_fffe, 1 << 31] {
                let mut a = buf(l + 8);
                let mut b = buf(l + 8);
                unsafe {
                    same_ret(
                        "ietf_ext_xor_ic",
                        xc(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), ic, k.as_ptr()),
                        xr(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, n.as_ptr(), ic, k.as_ptr()),
                    );
                }
                same_bytes(&format!("ietf_ext_xor_ic len={l} ic={ic}"), &a, &b);
            }
        }
    }
}

#[test]
fn row114_xchacha20() {
    stream_family("crypto_stream_xchacha20", 24, true, true);
}
