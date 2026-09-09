//! Phase C — error-path differential tests for group G4 (ERRORS.md rows 405-508)
//! plus generic boundaries (zero and oversized lengths, one step past every
//! documented range, out-of-range tag/flag ints across FFI).
//!
//! Rows whose rejection is a *return value* / `errno` side effect are compared
//! directly. Rows whose rejection is a `sodium_misuse()` abort are compared by
//! re-executing this binary in two child processes (one per library) and
//! comparing how the two children died — see `zz_abort_child` below.
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use std::os::raw::c_int;
use std::ptr;

// ===========================================================================
// C signatures (same shapes as t05)
// ===========================================================================
type SzFn = unsafe extern "C" fn() -> usize;
type IntFn = unsafe extern "C" fn() -> c_int;
type U8Fn = unsafe extern "C" fn() -> u8;
type KeygenFn = unsafe extern "C" fn(*mut u8);
type EncFn = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type DecFn = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;
type EncDetFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type DecDetFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;
type BeforenmFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
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
type SsRekey = unsafe extern "C" fn(*mut u8);
type StreamFn = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type XorFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XorIc64Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int;
type XorIc32Fn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
type CoreFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

const G: usize = 5;

unsafe fn fpair<F: Copy + 'static>(name: &str) -> (F, F) {
    let (a, b) = pair::<F>(name);
    (*a, *b)
}

#[track_caller]
fn sz(name: &str) -> usize {
    unsafe {
        let (c, r) = fpair::<SzFn>(name);
        let (a, b) = (c(), r());
        assert_eq!(a, b, "{name}: C={a} Rust={b}");
        a
    }
}

// ===========================================================================
// child-process driver for the `sodium_misuse()` abort rows
// ===========================================================================
#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return; // normal test run: nothing to do
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    macro_rules! sym {
        ($t:ty, $n:expr) => {{
            let mut n = ($n).as_bytes().to_vec();
            n.push(0);
            let s: libloading::Symbol<$t> = unsafe { h.get(&n) }.unwrap();
            *s
        }};
    }
    /// `<pfx>_messagebytes_max() + 1` — one step past the documented range.
    macro_rules! past_max {
        ($pfx:expr) => {{
            let f = sym!(SzFn, format!("{}_messagebytes_max", $pfx));
            (unsafe { f() } as u64).wrapping_add(1)
        }};
    }
    let mut rng = Rng::new(0xE6_60_06);
    unsafe {
        match case.as_str() {
            // ---- rows 405-407 / 415-417: aegis encrypt length misuse --------
            c if c.starts_with("aegis_") => {
                let (pfx, which) = if c.contains("128l") {
                    ("crypto_aead_aegis128l", &c[c.len() - 3..])
                } else {
                    ("crypto_aead_aegis256", &c[c.len() - 3..])
                };
                let kb = sz_child(h, &format!("{pfx}_keybytes"));
                let npb = sz_child(h, &format!("{pfx}_npubbytes"));
                let k = rng.bytes(kb);
                let npub = rng.bytes(npb);
                let m = rng.bytes(64);
                let mut out = vec![0u8; 128];
                let mut mac = vec![0u8; 64];
                let big = past_max!(pfx);
                match which {
                    // row 405 / 415: crypto_aead_*_encrypt, mlen > MAX
                    "enc" => {
                        let f = sym!(EncFn, format!("{pfx}_encrypt"));
                        let mut cl = 0u64;
                        f(
                            out.as_mut_ptr(),
                            &mut cl,
                            m.as_ptr(),
                            big,
                            ptr::null(),
                            0,
                            ptr::null(),
                            npub.as_ptr(),
                            k.as_ptr(),
                        );
                    }
                    // row 406 / 416: encrypt_detached, mlen > MAX (checked
                    // AFTER *maclen_p was set to ABYTES)
                    "edm" => {
                        let f = sym!(EncDetFn, format!("{pfx}_encrypt_detached"));
                        let mut ml = 0u64;
                        f(
                            out.as_mut_ptr(),
                            mac.as_mut_ptr(),
                            &mut ml,
                            m.as_ptr(),
                            big,
                            ptr::null(),
                            0,
                            ptr::null(),
                            npub.as_ptr(),
                            k.as_ptr(),
                        );
                    }
                    // row 407 / 417: encrypt_detached, adlen > MAX
                    "eda" => {
                        let f = sym!(EncDetFn, format!("{pfx}_encrypt_detached"));
                        let mut ml = 0u64;
                        f(
                            out.as_mut_ptr(),
                            mac.as_mut_ptr(),
                            &mut ml,
                            m.as_ptr(),
                            64,
                            m.as_ptr(),
                            big,
                            ptr::null(),
                            npub.as_ptr(),
                            k.as_ptr(),
                        );
                    }
                    other => panic!("bad aegis case {other}"),
                }
            }
            // ---- rows 442 / 448 / 454: chacha-family encrypt mlen misuse ----
            c if c.starts_with("aeadmax_") => {
                let pfx = &c["aeadmax_".len()..];
                let kb = sz_child(h, &format!("{pfx}_keybytes"));
                let npb = sz_child(h, &format!("{pfx}_npubbytes"));
                let k = rng.bytes(kb);
                let npub = rng.bytes(npb);
                let m = rng.bytes(64);
                let mut out = vec![0u8; 128];
                let big = past_max!(pfx);
                let f = sym!(EncFn, format!("{pfx}_encrypt"));
                let mut cl = 0u64;
                f(
                    out.as_mut_ptr(),
                    &mut cl,
                    m.as_ptr(),
                    big,
                    ptr::null(),
                    0,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                );
            }
            // ---- rows 460 / 466: secretbox_easy mlen misuse -----------------
            c if c.starts_with("sbmax_") => {
                let (pfx, easy) = if c.ends_with("xchacha") {
                    (
                        "crypto_secretbox_xchacha20poly1305",
                        "crypto_secretbox_xchacha20poly1305_easy",
                    )
                } else {
                    ("crypto_secretbox", "crypto_secretbox_easy")
                };
                let k = rng.bytes(32);
                let n = rng.bytes(24);
                let m = rng.bytes(64);
                let mut out = vec![0u8; 128];
                let big = past_max!(pfx);
                let f = sym!(SbEasy, easy);
                f(out.as_mut_ptr(), m.as_ptr(), big, n.as_ptr(), k.as_ptr());
            }
            // ---- row 476: secretstream push mlen misuse ---------------------
            "ss_push_mlen" => {
                let pfx = "crypto_secretstream_xchacha20poly1305";
                let stb = sz_child(h, &format!("{pfx}_statebytes"));
                let hb = sz_child(h, &format!("{pfx}_headerbytes"));
                let k = rng.bytes(32);
                let header = rng.bytes(hb);
                let mut st = vec![0u64; stb / 8 + 2];
                let ip = sym!(SsInitPull, format!("{pfx}_init_pull"));
                ip(st.as_mut_ptr() as *mut u8, header.as_ptr(), k.as_ptr());
                let m = rng.bytes(64);
                let mut out = vec![0u8; 128];
                let mut ol = 0xdead_beefu64;
                let big = past_max!(pfx);
                let f = sym!(SsPush, format!("{pfx}_push"));
                f(
                    st.as_mut_ptr() as *mut u8,
                    out.as_mut_ptr(),
                    &mut ol,
                    m.as_ptr(),
                    big,
                    ptr::null(),
                    0,
                    0,
                );
            }
            // ---- row 480: secretstream pull inlen - 17 > MAX ----------------
            "ss_pull_inlen" => {
                let pfx = "crypto_secretstream_xchacha20poly1305";
                let stb = sz_child(h, &format!("{pfx}_statebytes"));
                let hb = sz_child(h, &format!("{pfx}_headerbytes"));
                let ab = sz_child(h, &format!("{pfx}_abytes"));
                let k = rng.bytes(32);
                let header = rng.bytes(hb);
                let mut st = vec![0u64; stb / 8 + 2];
                let ip = sym!(SsInitPull, format!("{pfx}_init_pull"));
                ip(st.as_mut_ptr() as *mut u8, header.as_ptr(), k.as_ptr());
                let inbuf = rng.bytes(64);
                let mut out = vec![0u8; 128];
                let mut ml = 0u64;
                let mut tag = 0u8;
                let big = past_max!(pfx).wrapping_add(ab as u64);
                let f = sym!(SsPull, format!("{pfx}_pull"));
                f(
                    st.as_mut_ptr() as *mut u8,
                    out.as_mut_ptr(),
                    &mut ml,
                    &mut tag,
                    inbuf.as_ptr(),
                    big,
                    ptr::null(),
                    0,
                );
            }
            // ---- row 487: crypto_stream_chacha20_ietf clen > MAX ------------
            "chacha20_ietf_clen" => {
                let big = past_max!("crypto_stream_chacha20_ietf");
                let k = rng.bytes(32);
                let n = rng.bytes(12);
                let mut out = vec![0u8; 128];
                let f = sym!(StreamFn, "crypto_stream_chacha20_ietf");
                f(out.as_mut_ptr(), big, n.as_ptr(), k.as_ptr());
            }
            // ---- row 488: crypto_stream_chacha20_ietf_xor mlen > MAX --------
            "chacha20_ietf_xor_mlen" => {
                let big = past_max!("crypto_stream_chacha20_ietf");
                let k = rng.bytes(32);
                let n = rng.bytes(12);
                let m = rng.bytes(64);
                let mut out = vec![0u8; 128];
                let f = sym!(XorFn, "crypto_stream_chacha20_ietf_xor");
                f(out.as_mut_ptr(), m.as_ptr(), big, n.as_ptr(), k.as_ptr());
            }
            // ---- row 489: the only ic guard in the module -------------------
            // case name: "ietf_ic_<ic>_<mlen>"
            c if c.starts_with("ietf_ic_") => {
                let mut it = c["ietf_ic_".len()..].split('_');
                let ic: u32 = it.next().unwrap().parse().unwrap();
                let ml: usize = it.next().unwrap().parse().unwrap();
                let k = rng.bytes(32);
                let n = rng.bytes(12);
                let m = rng.bytes(ml);
                let mut out = vec![0u8; ml + 8];
                let f = sym!(XorIc32Fn, "crypto_stream_chacha20_ietf_xor_ic");
                let r = f(
                    out.as_mut_ptr(),
                    m.as_ptr(),
                    ml as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                );
                // did not abort: exit with a code derived from the result so
                // that a silent behaviour difference still shows up
                std::process::exit(if r == 0 { 0 } else { 3 });
            }
            other => panic!("unknown abort case {other}"),
        }
    }
    // Reached only when the call did NOT abort.
    std::process::exit(0);
}

/// `sz()` against a single library (child-process helper).
fn sz_child(h: &'static libloading::Library, name: &str) -> usize {
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    let f: libloading::Symbol<SzFn> = unsafe { h.get(&n) }.unwrap();
    unsafe { f() }
}

// ===========================================================================
// abort rows
// ===========================================================================
#[test]
fn e405_407_aegis128l_encrypt_length_misuse() {
    // rows 405, 406, 407 — mlen / adlen one step past
    // crypto_aead_aegis128l_MESSAGEBYTES_MAX (= 2^61-1) kills the process.
    for case in ["aegis_128l_enc", "aegis_128l_edm", "aegis_128l_eda"] {
        let t = diff_abort_case(case);
        assert!(t.signal.is_some(), "{case}: expected a fatal signal, got {t:?}");
    }
}

#[test]
fn e415_417_aegis256_encrypt_length_misuse() {
    // rows 415, 416, 417
    for case in ["aegis_256_enc", "aegis_256_edm", "aegis_256_eda"] {
        let t = diff_abort_case(case);
        assert!(t.signal.is_some(), "{case}: expected a fatal signal, got {t:?}");
    }
}

#[test]
fn e442_448_454_aead_encrypt_length_misuse() {
    // rows 442 (chacha20poly1305, MAX = SIZE_MAX-16), 448 (ietf, MAX =
    // min(SIZE_MAX-16, 64*(2^32-1))), 454 (xchacha20poly1305_ietf).
    for pfx in [
        "crypto_aead_chacha20poly1305",
        "crypto_aead_chacha20poly1305_ietf",
        "crypto_aead_xchacha20poly1305_ietf",
    ] {
        let case = format!("aeadmax_{pfx}");
        let t = diff_abort_case(&case);
        assert!(t.signal.is_some(), "{case}: expected a fatal signal, got {t:?}");
    }
}

#[test]
fn e460_466_secretbox_easy_length_misuse() {
    // rows 460, 466 — mlen > MESSAGEBYTES_MAX (= SIZE_MAX-16)
    for case in ["sbmax_xsalsa", "sbmax_xchacha"] {
        let t = diff_abort_case(case);
        assert!(t.signal.is_some(), "{case}: expected a fatal signal, got {t:?}");
    }
}

#[test]
fn e476_480_secretstream_length_misuse() {
    // rows 476 (push mlen > MAX, checked after *outlen_p = 0) and
    // 480 (pull inlen - ABYTES > MAX)
    for case in ["ss_push_mlen", "ss_pull_inlen"] {
        let t = diff_abort_case(case);
        assert!(t.signal.is_some(), "{case}: expected a fatal signal, got {t:?}");
    }
}

#[test]
fn e487_488_chacha20_ietf_length_misuse() {
    // rows 487, 488 — the ietf cap (64 * 2^32) IS reachable with a 64-bit
    // length argument, unlike the non-ietf SODIUM_SIZE_MAX cap (see e484_492).
    for case in ["chacha20_ietf_clen", "chacha20_ietf_xor_mlen"] {
        let t = diff_abort_case(case);
        assert!(t.signal.is_some(), "{case}: expected a fatal signal, got {t:?}");
    }
}

#[test]
fn e489_chacha20_ietf_xor_ic_overflow_guard() {
    // row 489 — the ONLY ic check in the module:
    //   ic > 2^32 - ceil(mlen/64)  ->  sodium_misuse()
    // Threshold: mlen == 0 -> 2^32; 1..64 -> 2^32-1; 65..128 -> 2^32-2; ...
    // Both the accepted and the rejected side of the boundary are compared.
    let max32 = 0xffff_ffffu32;
    // one step past the range -> must abort in both libraries
    for (ic, ml) in [
        (max32, 65usize),
        (max32, 128),
        (max32, 129),
        (max32 - 1, 129),
        (max32 - 1, 1000),
        (max32 - 2, 193),
    ] {
        let case = format!("ietf_ic_{ic}_{ml}");
        let t = diff_abort_case(&case);
        assert!(
            t.signal.is_some(),
            "ic={ic} mlen={ml}: expected abort in both, got {t:?}"
        );
    }
    // exactly at the range limit -> must NOT abort, and both must agree
    for (ic, ml) in [
        (max32, 0usize),
        (max32, 1),
        (max32, 63),
        (max32, 64),
        (max32 - 1, 65),
        (max32 - 1, 128),
        (0, 1000),
    ] {
        let case = format!("ietf_ic_{ic}_{ml}");
        let t = diff_abort_case(&case);
        assert_eq!(
            t,
            Term {
                code: Some(0),
                signal: None
            },
            "ic={ic} mlen={ml}: must be accepted by both"
        );
    }
}

#[test]
fn e484_492_500_501_chacha20_salsa20_length_guards_unreachable() {
    // rows 484, 485, 486, 490, 491, 492, 500, 501 — these guards compare a
    // `unsigned long long` length against SODIUM_SIZE_MAX = min(UINT64_MAX,
    // SIZE_MAX) = UINT64_MAX on this target, so NO argument value can trigger
    // them: the misuse path is unreachable through the public API. What IS
    // differentially observable is the cap itself, so we compare that (in both
    // libraries) and assert it equals usize::MAX.
    for name in [
        "crypto_stream_chacha20_messagebytes_max",
        "crypto_stream_salsa20_messagebytes_max",
        "crypto_stream_salsa2012_messagebytes_max",
        "crypto_stream_salsa208_messagebytes_max",
        "crypto_stream_xsalsa20_messagebytes_max",
        "crypto_stream_xchacha20_messagebytes_max",
        "crypto_stream_messagebytes_max",
    ] {
        assert_eq!(sz(name), usize::MAX, "{name}: cap must be SODIUM_SIZE_MAX");
    }
    // The ietf cap, by contrast, is smaller than u64::MAX and therefore
    // reachable — exercised by e487_488 / e489.
    assert!(sz("crypto_stream_chacha20_ietf_messagebytes_max") < usize::MAX);
}

// ===========================================================================
// aegis return-value rows
// ===========================================================================
struct Aead {
    pfx: String,
    kb: usize,
    npb: usize,
    ab: usize,
    enc_c: EncFn,
    enc_r: EncFn,
    dec_c: DecFn,
    dec_r: DecFn,
    ed_c: EncDetFn,
    ed_r: EncDetFn,
    dd_c: DecDetFn,
    dd_r: DecDetFn,
}

impl Aead {
    fn new(pfx: &str) -> Aead {
        unsafe {
            let (enc_c, enc_r) = fpair::<EncFn>(&format!("{pfx}_encrypt"));
            let (dec_c, dec_r) = fpair::<DecFn>(&format!("{pfx}_decrypt"));
            let (ed_c, ed_r) = fpair::<EncDetFn>(&format!("{pfx}_encrypt_detached"));
            let (dd_c, dd_r) = fpair::<DecDetFn>(&format!("{pfx}_decrypt_detached"));
            Aead {
                pfx: pfx.to_string(),
                kb: sz(&format!("{pfx}_keybytes")),
                npb: sz(&format!("{pfx}_npubbytes")),
                ab: sz(&format!("{pfx}_abytes")),
                enc_c,
                enc_r,
                dec_c,
                dec_r,
                ed_c,
                ed_r,
                dd_c,
                dd_r,
            }
        }
    }

    fn seal(&self, m: &[u8], ad: &[u8], npub: &[u8], k: &[u8]) -> Vec<u8> {
        let clen = m.len() + self.ab;
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0u8; clen];
        let mut b = vec![0u8; clen];
        unsafe {
            let ra = (self.enc_c)(
                a.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                m.len() as u64,
                adp,
                ad.len() as u64,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            );
            let rb = (self.enc_r)(
                b.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                m.len() as u64,
                adp,
                ad.len() as u64,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            );
            eq_i32("seal rc", ra, rb);
        }
        eq_bytes("seal out", &a, &b);
        a
    }

    /// Combined decrypt with an explicit `clen` (may be shorter than ABYTES).
    fn dec_raw(
        &self,
        label: &str,
        ct: &[u8],
        clen: u64,
        ad: &[u8],
        npub: &[u8],
        k: &[u8],
        mlen_p_null: bool,
    ) -> c_int {
        let mbuf = (clen as usize).saturating_sub(self.ab).min(1 << 16);
        let adp = if ad.is_empty() {
            ptr::null()
        } else {
            ad.as_ptr()
        };
        let mut a = vec![0x33u8; mbuf + G];
        let mut b = vec![0x33u8; mbuf + G];
        let mut la = 0xdead_beefu64;
        let mut lb = 0xdead_beefu64;
        let (pa, pb) = if mlen_p_null {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (&mut la as *mut u64, &mut lb as *mut u64)
        };
        let (ra, rb) = unsafe {
            (
                (self.dec_c)(
                    a.as_mut_ptr(),
                    pa,
                    ptr::null_mut(),
                    ct.as_ptr(),
                    clen,
                    adp,
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
                (self.dec_r)(
                    b.as_mut_ptr(),
                    pb,
                    ptr::null_mut(),
                    ct.as_ptr(),
                    clen,
                    adp,
                    ad.len() as u64,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32(&format!("{label}: rc"), ra, rb);
        eq_bytes(&format!("{label}: m"), &a, &b);
        assert_eq!(la, lb, "{label}: mlen_p");
        if !mlen_p_null && ra != 0 {
            assert_eq!(la, 0, "{label}: *mlen_p must be 0 on failure");
        }
        ra
    }

    fn dec_det_raw(
        &self,
        label: &str,
        c: *const u8,
        clen: u64,
        mac: &[u8],
        ad: *const u8,
        adlen: u64,
        npub: &[u8],
        k: &[u8],
        m_null: bool,
    ) -> c_int {
        let mbuf = (clen as usize).min(1 << 16);
        let mut a = vec![0x77u8; mbuf + G];
        let mut b = vec![0x77u8; mbuf + G];
        let (pa, pb) = if m_null {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (a.as_mut_ptr(), b.as_mut_ptr())
        };
        let (ra, rb) = unsafe {
            (
                (self.dd_c)(
                    pa,
                    ptr::null_mut(),
                    c,
                    clen,
                    mac.as_ptr(),
                    ad,
                    adlen,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
                (self.dd_r)(
                    pb,
                    ptr::null_mut(),
                    c,
                    clen,
                    mac.as_ptr(),
                    ad,
                    adlen,
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32(&format!("{label}: rc"), ra, rb);
        eq_bytes(&format!("{label}: m"), &a, &b);
        if m_null {
            assert!(
                a.iter().all(|&x| x == 0x77),
                "{label}: verify-only mode must not write"
            );
        }
        ra
    }

    /// `*_decrypt` with `clen < ABYTES` (the short-ciphertext rejection) plus
    /// the tag-mismatch rejection, with `mlen_p` NULL and non-NULL.
    fn short_and_forged(&self, seed: u64) {
        let mut rng = Rng::new(seed);
        let k = rng.bytes(self.kb);
        let npub = rng.bytes(self.npb);
        // every clen in 0..ABYTES must be rejected
        let junk = rng.bytes(self.ab + 64);
        for clen in 0..self.ab {
            for np in [false, true] {
                let r = self.dec_raw(
                    &format!("{} short clen={clen} mlen_p_null={np}", self.pfx),
                    &junk,
                    clen as u64,
                    &[],
                    &npub,
                    &k,
                    np,
                );
                assert_eq!(r, -1, "{}: clen={clen} must be rejected", self.pfx);
            }
        }
        // clen == ABYTES (empty message) with a forged mac
        for mlen in [0usize, 1, 16, 33, 64, 1000] {
            let m = rng.bytes(mlen);
            let ad = rng.bytes(17);
            let ct = self.seal(&m, &ad, &npub, &k);
            for np in [false, true] {
                assert_eq!(
                    self.dec_raw(
                        &format!("{} valid mlen={mlen}", self.pfx),
                        &ct,
                        ct.len() as u64,
                        &ad,
                        &npub,
                        &k,
                        np
                    ),
                    0
                );
            }
            // forge every byte position of the mac
            for i in 0..self.ab {
                let mut bad = ct.clone();
                bad[mlen + i] ^= 0x01;
                for np in [false, true] {
                    assert_eq!(
                        self.dec_raw(
                            &format!("{} forged mac[{i}] mlen={mlen}", self.pfx),
                            &bad,
                            bad.len() as u64,
                            &ad,
                            &npub,
                            &k,
                            np
                        ),
                        -1
                    );
                }
                // detached, m non-NULL (buffer must be wiped identically) and
                // m == NULL (verify-only)
                for m_null in [false, true] {
                    assert_eq!(
                        self.dec_det_raw(
                            &format!("{} det forged m_null={m_null}", self.pfx),
                            bad.as_ptr(),
                            mlen as u64,
                            &bad[mlen..],
                            ad.as_ptr(),
                            ad.len() as u64,
                            &npub,
                            &k,
                            m_null
                        ),
                        -1
                    );
                }
            }
        }
    }
}

#[test]
fn e408_414_aegis128l_decrypt_rejections() {
    // rows 408 (clen < ABYTES), 409 (tag mismatch through the combined API),
    // 412 (tag mismatch, m != NULL -> m wiped), 413 (tag mismatch, m == NULL).
    let a = Aead::new("crypto_aead_aegis128l");
    a.short_and_forged(0x0408);
    // row 414 is unreachable from the public API: `maclen` is hard-wired to
    // ABYTES, so the `maclen != 16 && maclen != 32` leg of aegis128l_mac()
    // cannot be selected. We assert the invariant that makes it unreachable.
    assert_eq!(a.ab, 32, "row 414: ABYTES must be 32");
}

#[test]
fn e410_411_aegis128l_decrypt_detached_oversized() {
    // rows 410, 411 — `clen`/`adlen` one step past MESSAGEBYTES_MAX returns -1
    // (NO abort), the documented asymmetry against encrypt_detached which
    // aborts for the very same overflow.
    let a = Aead::new("crypto_aead_aegis128l");
    let max = sz("crypto_aead_aegis128l_messagebytes_max") as u64;
    let mut rng = Rng::new(0x0410);
    let k = rng.bytes(a.kb);
    let npub = rng.bytes(a.npb);
    let mac = rng.bytes(a.ab);
    let c = rng.bytes(64);
    for clen in [max + 1, max + 2, u64::MAX / 2, u64::MAX] {
        for m_null in [false, true] {
            let r = a.dec_det_raw(
                &format!("aegis128l dd clen={clen}"),
                c.as_ptr(),
                clen,
                &mac,
                ptr::null(),
                0,
                &npub,
                &k,
                m_null,
            );
            assert_eq!(r, -1, "row 410: clen={clen} must return -1");
        }
    }
    for adlen in [max + 1, max + 2, u64::MAX] {
        let r = a.dec_det_raw(
            &format!("aegis128l dd adlen={adlen}"),
            c.as_ptr(),
            0,
            &mac,
            c.as_ptr(),
            adlen,
            &npub,
            &k,
            false,
        );
        assert_eq!(r, -1, "row 411: adlen={adlen} must return -1");
    }
}

#[test]
fn e418_424_aegis256_decrypt_rejections() {
    // rows 418, 419, 422, 423 (and 424, unreachable: maclen is always ABYTES)
    let a = Aead::new("crypto_aead_aegis256");
    a.short_and_forged(0x0418);
    assert_eq!(a.ab, 32, "row 424: ABYTES must be 32");
}

#[test]
fn e420_421_aegis256_decrypt_detached_oversized() {
    // rows 420, 421
    let a = Aead::new("crypto_aead_aegis256");
    let max = sz("crypto_aead_aegis256_messagebytes_max") as u64;
    let mut rng = Rng::new(0x0420);
    let k = rng.bytes(a.kb);
    let npub = rng.bytes(a.npb);
    let mac = rng.bytes(a.ab);
    let c = rng.bytes(64);
    for clen in [max + 1, max + 2, u64::MAX] {
        for m_null in [false, true] {
            assert_eq!(
                a.dec_det_raw(
                    &format!("aegis256 dd clen={clen}"),
                    c.as_ptr(),
                    clen,
                    &mac,
                    ptr::null(),
                    0,
                    &npub,
                    &k,
                    m_null
                ),
                -1
            );
        }
    }
    for adlen in [max + 1, u64::MAX] {
        assert_eq!(
            a.dec_det_raw(
                &format!("aegis256 dd adlen={adlen}"),
                c.as_ptr(),
                0,
                &mac,
                c.as_ptr(),
                adlen,
                &npub,
                &k,
                false
            ),
            -1
        );
    }
}

// ===========================================================================
// aes256gcm rows 425-441
// ===========================================================================
#[test]
fn e425_434_aes256gcm_enosys() {
    // rows 425-434 — `is_available()` is 0 and every operation sets
    // errno = ENOSYS and returns -1 without aborting.
    unsafe {
        let (av_c, av_r) = fpair::<IntFn>("crypto_aead_aes256gcm_is_available");
        eq_i32("row425 is_available", av_c(), av_r());
        assert_eq!(av_c(), 0);

        let stb = sz("crypto_aead_aes256gcm_statebytes");
        let mut rng = Rng::new(0x0425);
        let k = rng.bytes(32);
        let npub = rng.bytes(12);
        let m = rng.bytes(64);
        let ad = rng.bytes(17);
        let mac = rng.bytes(16);
        let mut st_c = vec![0u128; (stb + 15) / 16];
        let mut st_r = vec![0u128; (stb + 15) / 16];
        let sp_c = st_c.as_mut_ptr() as *const u8;
        let sp_r = st_r.as_mut_ptr() as *const u8;
        let mut out = vec![0u8; 128];
        let mut out2 = vec![0u8; 128];
        let mut mo = vec![0u8; 32];
        let mut mo2 = vec![0u8; 32];

        macro_rules! two {
            ($name:literal, $call:expr) => {{
                set_errno(0);
                let ra = $call(0);
                let ea = errno();
                set_errno(0);
                let rb = $call(1);
                let eb = errno();
                eq_i32(concat!($name, " rc"), ra, rb);
                assert_eq!(ra, -1, concat!($name, ": must return -1"));
                assert_eq!(
                    (ea, eb),
                    (ENOSYS, ENOSYS),
                    concat!($name, ": errno must be ENOSYS in both")
                );
            }};
        }

        // row 430: beforenm
        let (bn_c, bn_r) = fpair::<BeforenmFn>("crypto_aead_aes256gcm_beforenm");
        two!("row430 beforenm", |i: usize| if i == 0 {
            bn_c(st_c.as_mut_ptr() as *mut u8, k.as_ptr())
        } else {
            bn_r(st_r.as_mut_ptr() as *mut u8, k.as_ptr())
        });

        // rows 426/431: combined encrypt + encrypt_afternm
        for (name, (f, g), last) in [
            (
                "row426 encrypt",
                fpair::<EncFn>("crypto_aead_aes256gcm_encrypt"),
                (k.as_ptr(), k.as_ptr()),
            ),
            (
                "row431 encrypt_afternm",
                fpair::<EncFn>("crypto_aead_aes256gcm_encrypt_afternm"),
                (sp_c, sp_r),
            ),
        ] {
            let mut la = 0xdead_beefu64;
            let mut lb = 0xdead_beefu64;
            set_errno(0);
            let ra = f(
                out.as_mut_ptr(),
                &mut la,
                m.as_ptr(),
                64,
                ad.as_ptr(),
                17,
                ptr::null(),
                npub.as_ptr(),
                last.0,
            );
            let ea = errno();
            set_errno(0);
            let rb = g(
                out2.as_mut_ptr(),
                &mut lb,
                m.as_ptr(),
                64,
                ad.as_ptr(),
                17,
                ptr::null(),
                npub.as_ptr(),
                last.1,
            );
            let eb = errno();
            eq_i32(&format!("{name} rc"), ra, rb);
            assert_eq!(ra, -1);
            assert_eq!((ea, eb), (ENOSYS, ENOSYS), "{name} errno");
            assert_eq!((la, lb), (0xdead_beef, 0xdead_beef), "{name}: *clen_p");
        }

        // rows 427/432: detached encrypt + afternm
        for (name, (f, g), last) in [
            (
                "row427 encrypt_detached",
                fpair::<EncDetFn>("crypto_aead_aes256gcm_encrypt_detached"),
                (k.as_ptr(), k.as_ptr()),
            ),
            (
                "row432 encrypt_detached_afternm",
                fpair::<EncDetFn>("crypto_aead_aes256gcm_encrypt_detached_afternm"),
                (sp_c, sp_r),
            ),
        ] {
            let mut la = 0xdead_beefu64;
            let mut lb = 0xdead_beefu64;
            set_errno(0);
            let ra = f(
                out.as_mut_ptr(),
                mo.as_mut_ptr(),
                &mut la,
                m.as_ptr(),
                64,
                ad.as_ptr(),
                17,
                ptr::null(),
                npub.as_ptr(),
                last.0,
            );
            let ea = errno();
            set_errno(0);
            let rb = g(
                out2.as_mut_ptr(),
                mo2.as_mut_ptr(),
                &mut lb,
                m.as_ptr(),
                64,
                ad.as_ptr(),
                17,
                ptr::null(),
                npub.as_ptr(),
                last.1,
            );
            let eb = errno();
            eq_i32(&format!("{name} rc"), ra, rb);
            assert_eq!(ra, -1);
            assert_eq!((ea, eb), (ENOSYS, ENOSYS), "{name} errno");
            assert_eq!((la, lb), (0xdead_beef, 0xdead_beef), "{name}: *maclen_p");
        }

        // rows 428/433: combined decrypt + afternm, incl. clen < ABYTES
        for (name, (f, g), last) in [
            (
                "row428 decrypt",
                fpair::<DecFn>("crypto_aead_aes256gcm_decrypt"),
                (k.as_ptr(), k.as_ptr()),
            ),
            (
                "row433 decrypt_afternm",
                fpair::<DecFn>("crypto_aead_aes256gcm_decrypt_afternm"),
                (sp_c, sp_r),
            ),
        ] {
            for clen in [0u64, 1, 15, 16, 17, 80] {
                let mut la = 0xdead_beefu64;
                let mut lb = 0xdead_beefu64;
                set_errno(0);
                let ra = f(
                    out.as_mut_ptr(),
                    &mut la,
                    ptr::null_mut(),
                    m.as_ptr(),
                    clen,
                    ad.as_ptr(),
                    17,
                    npub.as_ptr(),
                    last.0,
                );
                let ea = errno();
                set_errno(0);
                let rb = g(
                    out2.as_mut_ptr(),
                    &mut lb,
                    ptr::null_mut(),
                    m.as_ptr(),
                    clen,
                    ad.as_ptr(),
                    17,
                    npub.as_ptr(),
                    last.1,
                );
                let eb = errno();
                eq_i32(&format!("{name} clen={clen} rc"), ra, rb);
                assert_eq!(ra, -1);
                assert_eq!((ea, eb), (ENOSYS, ENOSYS), "{name} errno");
                // row 441 (the aesni `clen < ABYTES -> *mlen_p = 0` path) is
                // compiled out here: the stub never writes *mlen_p at all.
                assert_eq!((la, lb), (0xdead_beef, 0xdead_beef), "{name}: *mlen_p");
            }
        }

        // rows 429/434: detached decrypt + afternm, m non-NULL and m == NULL
        for (name, (f, g), last) in [
            (
                "row429 decrypt_detached",
                fpair::<DecDetFn>("crypto_aead_aes256gcm_decrypt_detached"),
                (k.as_ptr(), k.as_ptr()),
            ),
            (
                "row434 decrypt_detached_afternm",
                fpair::<DecDetFn>("crypto_aead_aes256gcm_decrypt_detached_afternm"),
                (sp_c, sp_r),
            ),
        ] {
            for m_null in [false, true] {
                let (pa, pb) = if m_null {
                    (ptr::null_mut(), ptr::null_mut())
                } else {
                    (out.as_mut_ptr(), out2.as_mut_ptr())
                };
                set_errno(0);
                let ra = f(
                    pa,
                    ptr::null_mut(),
                    m.as_ptr(),
                    64,
                    mac.as_ptr(),
                    ad.as_ptr(),
                    17,
                    npub.as_ptr(),
                    last.0,
                );
                let ea = errno();
                set_errno(0);
                let rb = g(
                    pb,
                    ptr::null_mut(),
                    m.as_ptr(),
                    64,
                    mac.as_ptr(),
                    ad.as_ptr(),
                    17,
                    npub.as_ptr(),
                    last.1,
                );
                let eb = errno();
                eq_i32(&format!("{name} m_null={m_null} rc"), ra, rb);
                assert_eq!(ra, -1);
                assert_eq!((ea, eb), (ENOSYS, ENOSYS), "{name} errno");
            }
        }
        // the state buffers were never written by anything above
        let sc = std::slice::from_raw_parts(sp_c, stb);
        let sr = std::slice::from_raw_parts(sp_r, stb);
        eq_bytes("aes256gcm state untouched", sc, sr);
        assert!(sc.iter().all(|&x| x == 0));
    }
}

#[test]
fn e435_441_aes256gcm_hw_paths_compiled_out() {
    // rows 435-441 describe the aesni/armcrypto implementation, which is NOT
    // compiled into this build (no HAVE_TMMINTRIN_H/HAVE_WMMINTRIN_H, no
    // HAVE_ARMCRYPTO). The observable consequence is that `is_available()`
    // reports 0 and every entry point short-circuits to -1/ENOSYS before any
    // `required_blocks()` / `crypto_verify_16` logic can run. Verified here as
    // C/Rust agreement on the availability flag; the -1/ENOSYS behaviour for
    // the very inputs those rows describe (oversized ad_len/m_len, forged mac)
    // is asserted below.
    unsafe {
        let (av_c, av_r) = fpair::<IntFn>("crypto_aead_aes256gcm_is_available");
        eq_i32("is_available", av_c(), av_r());
        assert_eq!(av_c(), 0);
        let (ed_c, ed_r) = fpair::<EncDetFn>("crypto_aead_aes256gcm_encrypt_detached");
        let (dd_c, dd_r) = fpair::<DecDetFn>("crypto_aead_aes256gcm_decrypt_detached");
        let mut rng = Rng::new(0x0435);
        let k = rng.bytes(32);
        let npub = rng.bytes(12);
        let m = rng.bytes(64);
        let mac = rng.bytes(16);
        let mut out = vec![0u8; 128];
        let mut mo = vec![0u8; 32];
        // oversized ad_len / m_len (rows 435-438) and a forged mac (rows
        // 439/440) all take the same stub path here
        for (ml, al) in [
            (u64::MAX, 0u64),
            (0, u64::MAX),
            (64, u64::MAX),
            (u64::MAX / 2, 17),
            (16 * ((1u64 << 32) - 1), 17),
        ] {
            let mut la = 0xdead_beefu64;
            let mut lb = 0xdead_beefu64;
            set_errno(0);
            let ra = ed_c(
                out.as_mut_ptr(),
                mo.as_mut_ptr(),
                &mut la,
                m.as_ptr(),
                ml,
                m.as_ptr(),
                al,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            );
            let ea = errno();
            set_errno(0);
            let rb = ed_r(
                out.as_mut_ptr(),
                mo.as_mut_ptr(),
                &mut lb,
                m.as_ptr(),
                ml,
                m.as_ptr(),
                al,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            );
            let eb = errno();
            eq_i32(&format!("gcm ed ml={ml} al={al} rc"), ra, rb);
            assert_eq!(ra, -1);
            assert_eq!((ea, eb), (ENOSYS, ENOSYS));
            assert_eq!((la, lb), (0xdead_beef, 0xdead_beef));
            set_errno(0);
            let ra = dd_c(
                out.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                ml,
                mac.as_ptr(),
                m.as_ptr(),
                al,
                npub.as_ptr(),
                k.as_ptr(),
            );
            let ea = errno();
            set_errno(0);
            let rb = dd_r(
                out.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                ml,
                mac.as_ptr(),
                m.as_ptr(),
                al,
                npub.as_ptr(),
                k.as_ptr(),
            );
            let eb = errno();
            eq_i32(&format!("gcm dd ml={ml} al={al} rc"), ra, rb);
            assert_eq!(ra, -1);
            assert_eq!((ea, eb), (ENOSYS, ENOSYS));
        }
    }
}

// ===========================================================================
// chacha20poly1305 / ietf / xchacha rows 443-459
// ===========================================================================
#[test]
fn e443_447_chacha20poly1305_rejections() {
    // rows 444 (clen < 16), 445 (tag mismatch, combined), 446 (tag mismatch,
    // m != NULL -> m zeroed), 447 (tag mismatch, m == NULL -> raw
    // crypto_verify_16 result).
    let a = Aead::new("crypto_aead_chacha20poly1305");
    a.short_and_forged(0x0444);
    // row 443: `_encrypt_detached` has NO rejection path at all — it always
    // returns 0 and sets *maclen_p = 16.
    let mut rng = Rng::new(0x0443);
    let k = rng.bytes(a.kb);
    let npub = rng.bytes(a.npb);
    for &(ml, al) in &[
        (0usize, 0usize),
        (0, 1),
        (1, 0),
        (1, 1),
        (64, 4096),
        (1000, 0),
    ] {
        let m = rng.bytes(ml);
        let ad = rng.bytes(al);
        let adp = if al == 0 { ptr::null() } else { ad.as_ptr() };
        let mut ca = vec![0u8; ml + G];
        let mut cb = ca.clone();
        let mut ma = vec![0u8; a.ab + G];
        let mut mb = ma.clone();
        let mut la = 0xdead_beefu64;
        let mut lb = 0xdead_beefu64;
        let (ra, rb) = unsafe {
            (
                (a.ed_c)(
                    ca.as_mut_ptr(),
                    ma.as_mut_ptr(),
                    &mut la,
                    m.as_ptr(),
                    ml as u64,
                    adp,
                    al as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
                (a.ed_r)(
                    cb.as_mut_ptr(),
                    mb.as_mut_ptr(),
                    &mut lb,
                    m.as_ptr(),
                    ml as u64,
                    adp,
                    al as u64,
                    ptr::null(),
                    npub.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32("row443 encrypt_detached rc", ra, rb);
        assert_eq!(ra, 0, "row 443: no rejection path exists");
        eq_bytes("row443 c", &ca, &cb);
        eq_bytes("row443 mac", &ma, &mb);
        assert_eq!((la, lb), (16, 16), "row 443: *maclen_p = 16");
    }
}

#[test]
fn e448_453_chacha20poly1305_ietf_rejections() {
    // rows 450 (clen < 16), 451, 452 (m != NULL -> zeroed), 453 (m == NULL);
    // row 449 (`_encrypt_detached` has no checks) is asserted at the end.
    let a = Aead::new("crypto_aead_chacha20poly1305_ietf");
    a.short_and_forged(0x0450);
    let mut rng = Rng::new(0x0449);
    let k = rng.bytes(a.kb);
    let npub = rng.bytes(a.npb);
    let mut la = 0xdead_beefu64;
    let mut lb = 0xdead_beefu64;
    let mut ca = vec![0u8; G];
    let mut cb = vec![0u8; G];
    let mut ma = vec![0u8; a.ab + G];
    let mut mb = vec![0u8; a.ab + G];
    let (ra, rb) = unsafe {
        (
            (a.ed_c)(
                ca.as_mut_ptr(),
                ma.as_mut_ptr(),
                &mut la,
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            ),
            (a.ed_r)(
                cb.as_mut_ptr(),
                mb.as_mut_ptr(),
                &mut lb,
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            ),
        )
    };
    eq_i32("row449 rc", ra, rb);
    assert_eq!(ra, 0, "row 449: no rejection path");
    eq_bytes("row449 mac", &ma, &mb);
    assert_eq!((la, lb), (16, 16));
}

#[test]
fn e454_459_xchacha20poly1305_ietf_rejections() {
    // rows 456 (clen < 16), 457, 458 (m != NULL -> zeroed), 459 (m == NULL);
    // row 455 (`_encrypt_detached` has no checks).
    let a = Aead::new("crypto_aead_xchacha20poly1305_ietf");
    a.short_and_forged(0x0456);
    let mut rng = Rng::new(0x0455);
    let k = rng.bytes(a.kb);
    let npub = rng.bytes(a.npb);
    let mut la = 0xdead_beefu64;
    let mut lb = 0xdead_beefu64;
    let mut ma = vec![0u8; a.ab + G];
    let mut mb = vec![0u8; a.ab + G];
    let mut ca = vec![0u8; G];
    let mut cb = vec![0u8; G];
    let (ra, rb) = unsafe {
        (
            (a.ed_c)(
                ca.as_mut_ptr(),
                ma.as_mut_ptr(),
                &mut la,
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            ),
            (a.ed_r)(
                cb.as_mut_ptr(),
                mb.as_mut_ptr(),
                &mut lb,
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                npub.as_ptr(),
                k.as_ptr(),
            ),
        )
    };
    eq_i32("row455 rc", ra, rb);
    assert_eq!(ra, 0, "row 455: no rejection path");
    eq_bytes("row455 mac", &ma, &mb);
    assert_eq!((la, lb), (16, 16));
}

// ===========================================================================
// secretbox rows 461-473
// ===========================================================================
struct Sb {
    pfx: String,
    mb: usize,
    easy_c: SbEasy,
    easy_r: SbEasy,
    open_c: SbEasy,
    open_r: SbEasy,
    det_c: SbDet,
    det_r: SbDet,
    od_c: SbOpenDet,
    od_r: SbOpenDet,
}

impl Sb {
    fn new(pfx: &str) -> Sb {
        let (easy, open_easy, det, open_det) = if pfx == "crypto_secretbox" {
            (
                "crypto_secretbox_easy".to_string(),
                "crypto_secretbox_open_easy".to_string(),
                "crypto_secretbox_detached".to_string(),
                "crypto_secretbox_open_detached".to_string(),
            )
        } else {
            (
                format!("{pfx}_easy"),
                format!("{pfx}_open_easy"),
                format!("{pfx}_detached"),
                format!("{pfx}_open_detached"),
            )
        };
        unsafe {
            let (easy_c, easy_r) = fpair::<SbEasy>(&easy);
            let (open_c, open_r) = fpair::<SbEasy>(&open_easy);
            let (det_c, det_r) = fpair::<SbDet>(&det);
            let (od_c, od_r) = fpair::<SbOpenDet>(&open_det);
            Sb {
                pfx: pfx.to_string(),
                mb: sz(&format!("{pfx}_macbytes")),
                easy_c,
                easy_r,
                open_c,
                open_r,
                det_c,
                det_r,
                od_c,
                od_r,
            }
        }
    }

    fn open_raw(&self, label: &str, ct: &[u8], clen: u64, n: &[u8], k: &[u8]) -> c_int {
        let mbuf = (clen as usize).saturating_sub(self.mb);
        let mut a = vec![0x33u8; mbuf + G];
        let mut b = vec![0x33u8; mbuf + G];
        let (ra, rb) = unsafe {
            (
                (self.open_c)(a.as_mut_ptr(), ct.as_ptr(), clen, n.as_ptr(), k.as_ptr()),
                (self.open_r)(b.as_mut_ptr(), ct.as_ptr(), clen, n.as_ptr(), k.as_ptr()),
            )
        };
        eq_i32(&format!("{label}: rc"), ra, rb);
        eq_bytes(&format!("{label}: m"), &a, &b);
        ra
    }

    fn od_raw(
        &self,
        label: &str,
        c: &[u8],
        mac: &[u8],
        n: &[u8],
        k: &[u8],
        m_null: bool,
    ) -> c_int {
        let mut a = vec![0x77u8; c.len() + G];
        let mut b = vec![0x77u8; c.len() + G];
        let (pa, pb) = if m_null {
            (ptr::null_mut(), ptr::null_mut())
        } else {
            (a.as_mut_ptr(), b.as_mut_ptr())
        };
        let (ra, rb) = unsafe {
            (
                (self.od_c)(
                    pa,
                    c.as_ptr(),
                    mac.as_ptr(),
                    c.len() as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                ),
                (self.od_r)(
                    pb,
                    c.as_ptr(),
                    mac.as_ptr(),
                    c.len() as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                ),
            )
        };
        eq_i32(&format!("{label}: rc"), ra, rb);
        eq_bytes(&format!("{label}: m"), &a, &b);
        ra
    }

    fn rejections(&self, seed: u64) {
        let mut rng = Rng::new(seed);
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        // clen < MACBYTES must be rejected before m is touched
        let junk = rng.bytes(64);
        for clen in 0..self.mb {
            let r = self.open_raw(
                &format!("{} open_easy clen={clen}", self.pfx),
                &junk,
                clen as u64,
                &n,
                &k,
            );
            assert_eq!(r, -1, "{}: clen={clen} must be rejected", self.pfx);
        }
        for mlen in [0usize, 1, 16, 31, 32, 33, 64, 1000] {
            let m = rng.bytes(mlen);
            let mut ct = vec![0u8; mlen + self.mb];
            let mut ct2 = ct.clone();
            unsafe {
                let ra = (self.easy_c)(
                    ct.as_mut_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (self.easy_r)(
                    ct2.as_mut_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32("easy rc", ra, rb);
            }
            eq_bytes("easy out", &ct, &ct2);
            assert_eq!(
                self.open_raw(
                    &format!("{} valid mlen={mlen}", self.pfx),
                    &ct,
                    ct.len() as u64,
                    &n,
                    &k
                ),
                0
            );
            for i in 0..self.mb {
                let mut bad = ct.clone();
                bad[i] ^= 0x01;
                assert_eq!(
                    self.open_raw(
                        &format!("{} bad mac[{i}] mlen={mlen}", self.pfx),
                        &bad,
                        bad.len() as u64,
                        &n,
                        &k
                    ),
                    -1
                );
                // open_detached: the C does NOT zero m (unlike the AEADs), so
                // the untouched 0x77 fill must survive in both libraries
                for m_null in [false, true] {
                    let r = self.od_raw(
                        &format!("{} od bad m_null={m_null}", self.pfx),
                        &bad[self.mb..],
                        &bad[..self.mb],
                        &n,
                        &k,
                        m_null,
                    );
                    assert_eq!(r, -1);
                }
            }
            // m == NULL with a valid mac -> early return 0, nothing written
            assert_eq!(
                self.od_raw(
                    &format!("{} od valid m=NULL", self.pfx),
                    &ct[self.mb..],
                    &ct[..self.mb],
                    &n,
                    &k,
                    true
                ),
                0
            );
            // `_detached` itself has no rejection path: always 0
            let mut ca = vec![0u8; mlen + G];
            let mut cb = ca.clone();
            let mut ma = vec![0u8; self.mb + G];
            let mut mbf = ma.clone();
            let (ra, rb) = unsafe {
                (
                    (self.det_c)(
                        ca.as_mut_ptr(),
                        ma.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    ),
                    (self.det_r)(
                        cb.as_mut_ptr(),
                        mbf.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as u64,
                        n.as_ptr(),
                        k.as_ptr(),
                    ),
                )
            };
            eq_i32("detached rc", ra, rb);
            assert_eq!(ra, 0, "{}: _detached never fails", self.pfx);
            eq_bytes("detached c", &ca, &cb);
            eq_bytes("detached mac", &ma, &mbf);
        }
    }
}

#[test]
fn e461_465_secretbox_rejections() {
    // rows 461 (`_detached` never fails), 462 (clen < MACBYTES), 463 (tag
    // mismatch), 464 (bad mac -> -1, m NOT zeroed), 465 (m == NULL -> 0)
    Sb::new("crypto_secretbox").rejections(0x0462);
}

#[test]
fn e466_470_secretbox_xchacha_rejections() {
    // rows 467, 468, 469, 470
    Sb::new("crypto_secretbox_xchacha20poly1305").rejections(0x0468);
}

#[test]
fn e471_473_secretbox_nacl_form_rejections() {
    // rows 471 (`crypto_secretbox`: mlen < ZEROBYTES=32 -> -1, c untouched),
    // 472 (`crypto_secretbox_open`: clen < 32 -> -1, m untouched),
    // 473 (bad mac at c[16..32) -> -1, m untouched; c[0..16) is NOT checked).
    let zb = sz("crypto_secretbox_zerobytes");
    let bzb = sz("crypto_secretbox_boxzerobytes");
    assert_eq!((zb, bzb), (32, 16));
    let mut rng = Rng::new(0x0471);
    unsafe {
        for (seal, open) in [
            ("crypto_secretbox", "crypto_secretbox_open"),
            (
                "crypto_secretbox_xsalsa20poly1305",
                "crypto_secretbox_xsalsa20poly1305_open",
            ),
        ] {
            let (sc, sr) = fpair::<SbEasy>(seal);
            let (oc, or) = fpair::<SbEasy>(open);
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let m = vec![0u8; 128];
            // rows 471/472: every length below 32 is rejected, buffers untouched
            for len in 0..zb {
                let mut a = vec![0xA5u8; 128];
                let mut b = vec![0xA5u8; 128];
                let ra = sc(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let rb = sr(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{seal} mlen={len} rc"), ra, rb);
                assert_eq!(ra, -1, "row 471: mlen={len} must be rejected");
                eq_bytes(&format!("{seal} mlen={len} c"), &a, &b);
                assert!(a.iter().all(|&x| x == 0xA5), "row 471: c was written");

                let mut a = vec![0x33u8; 128];
                let mut b = vec![0x33u8; 128];
                let ra = oc(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let rb = or(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{open} clen={len} rc"), ra, rb);
                assert_eq!(ra, -1, "row 472: clen={len} must be rejected");
                eq_bytes(&format!("{open} clen={len} m"), &a, &b);
                assert!(a.iter().all(|&x| x == 0x33), "row 472: m was written");
            }
            // row 473: forged mac
            for payload in [0usize, 1, 32, 33, 100] {
                let mlen = zb + payload;
                let mut plain = vec![0u8; mlen];
                let body = rng.bytes(payload);
                plain[zb..].copy_from_slice(&body);
                let mut ct = vec![0u8; mlen];
                let r = sc(
                    ct.as_mut_ptr(),
                    plain.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                assert_eq!(r, 0);
                for i in 0..16 {
                    let mut bad = ct.clone();
                    bad[bzb + i] ^= 0x01;
                    let mut a = vec![0x33u8; mlen + G];
                    let mut b = vec![0x33u8; mlen + G];
                    let ra =
                        oc(a.as_mut_ptr(), bad.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    let rb =
                        or(b.as_mut_ptr(), bad.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr());
                    eq_i32(&format!("{open} forged[{i}] rc"), ra, rb);
                    assert_eq!(ra, -1, "row 473: forged mac must be rejected");
                    eq_bytes(&format!("{open} forged[{i}] m"), &a, &b);
                    assert!(a.iter().all(|&x| x == 0x33), "row 473: m must be untouched");
                }
                // c[0..BOXZEROBYTES) is NOT validated: scribbling on it must
                // still verify (it is not part of the authenticated data).
                let mut scribbled = ct.clone();
                for i in 0..bzb {
                    scribbled[i] = 0xff;
                }
                let mut a = vec![0x33u8; mlen + G];
                let mut b = vec![0x33u8; mlen + G];
                let ra = oc(
                    a.as_mut_ptr(),
                    scribbled.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                let rb = or(
                    b.as_mut_ptr(),
                    scribbled.as_ptr(),
                    mlen as u64,
                    n.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{open} scribbled rc"), ra, rb);
                assert_eq!(ra, 0, "row 473: c[0..16) is not checked");
                eq_bytes(&format!("{open} scribbled m"), &a, &b);
            }
        }
    }
}

// ===========================================================================
// secretstream rows 474-483
// ===========================================================================
struct Ss {
    stb: usize,
    hb: usize,
    ab: usize,
    ipush_c: SsInitPush,
    ipush_r: SsInitPush,
    ipull_c: SsInitPull,
    ipull_r: SsInitPull,
    push_c: SsPush,
    push_r: SsPush,
    pull_c: SsPull,
    pull_r: SsPull,
    rekey_c: SsRekey,
    rekey_r: SsRekey,
}

struct St {
    buf: Vec<u64>,
    n: usize,
}
impl St {
    fn new(n: usize) -> St {
        St {
            buf: vec![0u64; n / 8 + 2],
            n,
        }
    }
    fn ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.n) }
    }
}

impl Ss {
    fn new() -> Ss {
        let p = "crypto_secretstream_xchacha20poly1305";
        unsafe {
            let (ipush_c, ipush_r) = fpair::<SsInitPush>(&format!("{p}_init_push"));
            let (ipull_c, ipull_r) = fpair::<SsInitPull>(&format!("{p}_init_pull"));
            let (push_c, push_r) = fpair::<SsPush>(&format!("{p}_push"));
            let (pull_c, pull_r) = fpair::<SsPull>(&format!("{p}_pull"));
            let (rekey_c, rekey_r) = fpair::<SsRekey>(&format!("{p}_rekey"));
            Ss {
                stb: sz(&format!("{p}_statebytes")),
                hb: sz(&format!("{p}_headerbytes")),
                ab: sz(&format!("{p}_abytes")),
                ipush_c,
                ipush_r,
                ipull_c,
                ipull_r,
                push_c,
                push_r,
                pull_c,
                pull_r,
                rekey_c,
                rekey_r,
            }
        }
    }

    fn init_pull(&self, header: &[u8], k: &[u8]) -> (St, St) {
        let mut a = St::new(self.stb);
        let mut b = St::new(self.stb);
        let ra = unsafe { (self.ipull_c)(a.ptr(), header.as_ptr(), k.as_ptr()) };
        let rb = unsafe { (self.ipull_r)(b.ptr(), header.as_ptr(), k.as_ptr()) };
        eq_i32("init_pull rc", ra, rb);
        assert_eq!(ra, 0, "row 475: init_pull can never fail");
        eq_bytes("init_pull state", a.bytes(), b.bytes());
        (a, b)
    }
}

#[test]
fn e474_475_483_secretstream_infallible_entry_points() {
    // rows 474 (init_push always 0), 475 (init_pull always 0 for ANY header),
    // 483 (rekey is `void` and cannot fail).
    install_det_random();
    let ss = Ss::new();
    let mut rng = Rng::new(0x0474);
    for i in 0..8u64 {
        let k = rng.bytes(32);
        let mut a = St::new(ss.stb);
        let mut b = St::new(ss.stb);
        let mut ha = vec![0u8; ss.hb + G];
        let mut hbf = vec![0u8; ss.hb + G];
        det_reseed(0x900 + i);
        let ra = unsafe { (ss.ipush_c)(a.ptr(), ha.as_mut_ptr(), k.as_ptr()) };
        det_reseed(0x900 + i);
        let rb = unsafe { (ss.ipush_r)(b.ptr(), hbf.as_mut_ptr(), k.as_ptr()) };
        eq_i32("row474 init_push rc", ra, rb);
        assert_eq!(ra, 0, "row 474: init_push always returns 0");
        eq_bytes("row474 header", &ha, &hbf);
        eq_bytes("row474 state", a.bytes(), b.bytes());
        // row 483: rekey any number of times, still in lockstep
        for _ in 0..3 {
            unsafe {
                (ss.rekey_c)(a.ptr());
                (ss.rekey_r)(b.ptr());
            }
            eq_bytes("row483 rekey state", a.bytes(), b.bytes());
        }
    }
    // row 475: arbitrary headers, including all-zero and all-0xff
    let k = rng.bytes(32);
    for h in [
        vec![0u8; ss.hb],
        vec![0xffu8; ss.hb],
        rng.bytes(ss.hb),
        (0..ss.hb as u8).collect::<Vec<u8>>(),
    ] {
        let _ = ss.init_pull(&h, &k);
    }
}

#[test]
fn e477_secretstream_arbitrary_tag_accepted() {
    // row 477 — NO tag validation: every one of the 256 byte values is accepted
    // verbatim, and only bit 0x02 (TAG_REKEY) has a side effect.
    let ss = Ss::new();
    let mut rng = Rng::new(0x0477);
    let k = rng.bytes(32);
    let header = rng.bytes(ss.hb);
    for tag in 0u8..=255 {
        let (mut sc, mut sr) = ss.init_pull(&header, &k);
        let m = rng.bytes(21);
        let olen = m.len() + ss.ab;
        let mut a = vec![0xA5u8; olen + G];
        let mut b = vec![0xA5u8; olen + G];
        let mut la = 0xdead_beefu64;
        let mut lb = 0xdead_beefu64;
        let (ra, rb) = unsafe {
            (
                (ss.push_c)(
                    sc.ptr(),
                    a.as_mut_ptr(),
                    &mut la,
                    m.as_ptr(),
                    m.len() as u64,
                    ptr::null(),
                    0,
                    tag,
                ),
                (ss.push_r)(
                    sr.ptr(),
                    b.as_mut_ptr(),
                    &mut lb,
                    m.as_ptr(),
                    m.len() as u64,
                    ptr::null(),
                    0,
                    tag,
                ),
            )
        };
        eq_i32(&format!("row477 push tag={tag} rc"), ra, rb);
        assert_eq!(ra, 0, "row 477: tag {tag} must be accepted");
        eq_bytes(&format!("row477 push tag={tag}"), &a, &b);
        eq_bytes(&format!("row477 state tag={tag}"), sc.bytes(), sr.bytes());
        assert_eq!((la, lb), (olen as u64, olen as u64));

        // the tag round-trips verbatim through pull
        let (mut pc, mut pr) = ss.init_pull(&header, &k);
        let mut ma = vec![0x33u8; m.len() + G];
        let mut mbf = vec![0x33u8; m.len() + G];
        let mut ta = 0u8;
        let mut tb = 0u8;
        let mut xa = 0u64;
        let mut xb = 0u64;
        let (ra, rb) = unsafe {
            (
                (ss.pull_c)(
                    pc.ptr(),
                    ma.as_mut_ptr(),
                    &mut xa,
                    &mut ta,
                    a.as_ptr(),
                    olen as u64,
                    ptr::null(),
                    0,
                ),
                (ss.pull_r)(
                    pr.ptr(),
                    mbf.as_mut_ptr(),
                    &mut xb,
                    &mut tb,
                    b.as_ptr(),
                    olen as u64,
                    ptr::null(),
                    0,
                ),
            )
        };
        eq_i32(&format!("row477 pull tag={tag} rc"), ra, rb);
        assert_eq!(ra, 0);
        eq_bytes(&format!("row477 pull tag={tag} m"), &ma, &mbf);
        assert_eq!((ta, tb), (tag, tag), "row 477: tag must round-trip");
        assert_eq!((xa, xb), (m.len() as u64, m.len() as u64));
    }
}

#[test]
fn e479_481_secretstream_pull_rejections() {
    // rows 479 (inlen < ABYTES=17 -> -1, *mlen_p = 0, *tag_p = 0xff) and
    // 481 (mac mismatch -> -1, m NOT written, state NOT advanced).
    let ss = Ss::new();
    let mut rng = Rng::new(0x0479);
    let k = rng.bytes(32);
    let header = rng.bytes(ss.hb);
    let junk = rng.bytes(64);

    // row 479: every inlen below ABYTES
    for inlen in 0..ss.ab {
        for nulls in [(false, false), (true, false), (false, true), (true, true)] {
            let (mut pc, mut pr) = ss.init_pull(&header, &k);
            let mut ma = vec![0x33u8; 64];
            let mut mbf = vec![0x33u8; 64];
            let mut la = 0xdead_beefu64;
            let mut lb = 0xdead_beefu64;
            let mut ta = 0x5Au8;
            let mut tb = 0x5Au8;
            let (pla, plb) = if nulls.0 {
                (ptr::null_mut(), ptr::null_mut())
            } else {
                (&mut la as *mut u64, &mut lb as *mut u64)
            };
            let (pta, ptb) = if nulls.1 {
                (ptr::null_mut(), ptr::null_mut())
            } else {
                (&mut ta as *mut u8, &mut tb as *mut u8)
            };
            let (ra, rb) = unsafe {
                (
                    (ss.pull_c)(
                        pc.ptr(),
                        ma.as_mut_ptr(),
                        pla,
                        pta,
                        junk.as_ptr(),
                        inlen as u64,
                        ptr::null(),
                        0,
                    ),
                    (ss.pull_r)(
                        pr.ptr(),
                        mbf.as_mut_ptr(),
                        plb,
                        ptb,
                        junk.as_ptr(),
                        inlen as u64,
                        ptr::null(),
                        0,
                    ),
                )
            };
            eq_i32(&format!("row479 inlen={inlen} rc"), ra, rb);
            assert_eq!(ra, -1, "row 479: inlen={inlen} must be rejected");
            eq_bytes(&format!("row479 inlen={inlen} m"), &ma, &mbf);
            assert!(ma.iter().all(|&x| x == 0x33), "row 479: m was written");
            assert_eq!((la, lb), if nulls.0 { (0xdead_beef, 0xdead_beef) } else { (0, 0) });
            assert_eq!((ta, tb), if nulls.1 { (0x5A, 0x5A) } else { (0xff, 0xff) });
            eq_bytes(&format!("row479 state inlen={inlen}"), pc.bytes(), pr.bytes());
        }
    }

    // row 481: forged ciphertext / wrong ad / replay
    for mlen in [0usize, 1, 17, 64, 65, 1000] {
        let (mut sc, mut sr) = ss.init_pull(&header, &k);
        let m = rng.bytes(mlen);
        let ad = rng.bytes(17);
        let olen = mlen + ss.ab;
        let mut out = vec![0u8; olen];
        let mut out2 = vec![0u8; olen];
        unsafe {
            let ra = (ss.push_c)(
                sc.ptr(),
                out.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                17,
                0,
            );
            let rb = (ss.push_r)(
                sr.ptr(),
                out2.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                17,
                0,
            );
            eq_i32("push rc", ra, rb);
        }
        eq_bytes("push out", &out, &out2);

        // every single-bit forgery in the output, plus a wrong ad
        let mut forgeries: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
        for i in 0..olen {
            let mut bad = out.clone();
            bad[i] ^= 0x01;
            forgeries.push((format!("bit{i}"), bad, ad.clone()));
        }
        let mut badad = ad.clone();
        badad[3] ^= 0x80;
        forgeries.push(("badad".into(), out.clone(), badad));
        forgeries.push(("noad".into(), out.clone(), vec![]));

        for (name, inbuf, adx) in forgeries {
            let (mut pc, mut pr) = ss.init_pull(&header, &k);
            let before = pc.bytes().to_vec();
            let mut ma = vec![0x33u8; mlen + G];
            let mut mbf = vec![0x33u8; mlen + G];
            let mut la = 0xdead_beefu64;
            let mut lb = 0xdead_beefu64;
            let mut ta = 0x5Au8;
            let mut tb = 0x5Au8;
            let adp = if adx.is_empty() {
                ptr::null()
            } else {
                adx.as_ptr()
            };
            let (ra, rb) = unsafe {
                (
                    (ss.pull_c)(
                        pc.ptr(),
                        ma.as_mut_ptr(),
                        &mut la,
                        &mut ta,
                        inbuf.as_ptr(),
                        olen as u64,
                        adp,
                        adx.len() as u64,
                    ),
                    (ss.pull_r)(
                        pr.ptr(),
                        mbf.as_mut_ptr(),
                        &mut lb,
                        &mut tb,
                        inbuf.as_ptr(),
                        olen as u64,
                        adp,
                        adx.len() as u64,
                    ),
                )
            };
            eq_i32(&format!("row481 {name} mlen={mlen} rc"), ra, rb);
            assert_eq!(ra, -1, "row 481: forgery {name} must be rejected");
            eq_bytes(&format!("row481 {name} m"), &ma, &mbf);
            assert!(
                ma.iter().all(|&x| x == 0x33),
                "row 481: m must not be written on failure"
            );
            assert_eq!((la, lb), (0, 0), "row 481: *mlen_p stays 0");
            assert_eq!((ta, tb), (0xff, 0xff), "row 481: *tag_p stays 0xff");
            eq_bytes(&format!("row481 {name} state"), pc.bytes(), pr.bytes());
            eq_bytes(&format!("row481 {name} state un-advanced"), &before, pc.bytes());
            // and the un-advanced state still accepts the genuine message
            let mut ma = vec![0u8; mlen + G];
            let mut mbf = vec![0u8; mlen + G];
            let (ra, rb) = unsafe {
                (
                    (ss.pull_c)(
                        pc.ptr(),
                        ma.as_mut_ptr(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                        out.as_ptr(),
                        olen as u64,
                        ad.as_ptr(),
                        17,
                    ),
                    (ss.pull_r)(
                        pr.ptr(),
                        mbf.as_mut_ptr(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                        out.as_ptr(),
                        olen as u64,
                        ad.as_ptr(),
                        17,
                    ),
                )
            };
            eq_i32(&format!("row481 {name} recovery rc"), ra, rb);
            assert_eq!(ra, 0, "row 481: state must be usable after a failure");
            eq_bytes(&format!("row481 {name} recovery m"), &ma[..mlen], &m);
        }
        // replay: pulling the same message twice must fail the second time
        let (mut pc, mut pr) = ss.init_pull(&header, &k);
        let mut ma = vec![0u8; mlen + G];
        let mut mbf = vec![0u8; mlen + G];
        for round in 0..2 {
            let (ra, rb) = unsafe {
                (
                    (ss.pull_c)(
                        pc.ptr(),
                        ma.as_mut_ptr(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                        out.as_ptr(),
                        olen as u64,
                        ad.as_ptr(),
                        17,
                    ),
                    (ss.pull_r)(
                        pr.ptr(),
                        mbf.as_mut_ptr(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                        out.as_ptr(),
                        olen as u64,
                        ad.as_ptr(),
                        17,
                    ),
                )
            };
            eq_i32(&format!("row481 replay round={round} rc"), ra, rb);
            assert_eq!(ra, if round == 0 { 0 } else { -1 }, "row 481: replay");
            eq_bytes(&format!("row481 replay m round={round}"), &ma, &mbf);
        }
    }
}

#[test]
fn e478_482_secretstream_unreachable_and_ub_rows() {
    // row 478 — the 32-bit message counter wrap (2^32-1 pushes) triggers an
    // implicit rekey and is NOT an error. Reaching it requires ~4.3e9 pushes,
    // which is not feasible in a test, so it is NOT covered by an exhaustive
    // run. What IS checked here (and in t05 row 655) is the *mechanism*: the
    // counter is a 4-byte little-endian field that is incremented per message
    // and reset to 1 by a rekey, and both libraries keep it identical.
    //
    // row 482 — `pull` with `m == NULL` is undefined behaviour (the code
    // unconditionally calls crypto_stream_chacha20_ietf_xor_ic(m, ...)); there
    // is deliberately no verify-only mode, unlike the AEAD decrypt_detached
    // family, so this must NOT be exercised. We assert the *contrast* instead:
    // the AEAD family accepts m == NULL where secretstream does not.
    let ss = Ss::new();
    let mut rng = Rng::new(0x0478);
    let k = rng.bytes(32);
    let header = rng.bytes(ss.hb);
    let (mut sc, mut sr) = ss.init_pull(&header, &k);
    assert_eq!(&sc.bytes()[32..36], &[1u8, 0, 0, 0], "counter starts at 1");
    let m = rng.bytes(9);
    for i in 1..=64u32 {
        let mut a = vec![0u8; 9 + ss.ab];
        let mut b = a.clone();
        unsafe {
            let ra = (ss.push_c)(
                sc.ptr(),
                a.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                9,
                ptr::null(),
                0,
                0,
            );
            let rb = (ss.push_r)(
                sr.ptr(),
                b.as_mut_ptr(),
                ptr::null_mut(),
                m.as_ptr(),
                9,
                ptr::null(),
                0,
                0,
            );
            eq_i32("row478 push rc", ra, rb);
        }
        eq_bytes("row478 push out", &a, &b);
        eq_bytes("row478 state", sc.bytes(), sr.bytes());
        assert_eq!(
            &sc.bytes()[32..36],
            &(i + 1).to_le_bytes(),
            "row478: counter must be little-endian and increment by 1"
        );
    }
    // a REKEY-tag push resets the counter to 1 in both libraries
    let mut a = vec![0u8; 9 + ss.ab];
    let mut b = a.clone();
    unsafe {
        (ss.push_c)(
            sc.ptr(),
            a.as_mut_ptr(),
            ptr::null_mut(),
            m.as_ptr(),
            9,
            ptr::null(),
            0,
            0x02,
        );
        (ss.push_r)(
            sr.ptr(),
            b.as_mut_ptr(),
            ptr::null_mut(),
            m.as_ptr(),
            9,
            ptr::null(),
            0,
            0x02,
        );
    }
    eq_bytes("row478 rekey push out", &a, &b);
    eq_bytes("row478 rekey state", sc.bytes(), sr.bytes());
    assert_eq!(&sc.bytes()[32..36], &[1u8, 0, 0, 0], "row478: counter reset");
}

// ===========================================================================
// crypto_stream rows 484-502
// ===========================================================================
#[test]
fn e493_498_stream_zero_length_shortcircuits() {
    // rows 493 (chacha20 ref: stream / ietf_ext / xor_ic / ietf_ext_xor_ic with
    // clen/mlen == 0), 496 (salsa20 ref), 497 (salsa2012), 498 (salsa208) —
    // early `return 0` with the output buffer untouched.
    let mut rng = Rng::new(0x0493);
    unsafe {
        for (name, nb) in [
            ("crypto_stream_chacha20", 8usize),
            ("crypto_stream_chacha20_ietf", 12),
            ("crypto_stream_chacha20_ietf_ext", 12),
            ("crypto_stream_salsa20", 8),
            ("crypto_stream_salsa2012", 8),
            ("crypto_stream_salsa208", 8),
            ("crypto_stream_xsalsa20", 24),
            ("crypto_stream_xchacha20", 24),
            ("crypto_stream", 24),
        ] {
            let k = rng.bytes(32);
            let n = rng.bytes(nb);
            let (f_c, f_r) = fpair::<StreamFn>(name);
            let mut a = vec![0xA5u8; 64];
            let mut b = vec![0xA5u8; 64];
            let ra = f_c(a.as_mut_ptr(), 0, n.as_ptr(), k.as_ptr());
            let rb = f_r(b.as_mut_ptr(), 0, n.as_ptr(), k.as_ptr());
            eq_i32(&format!("{name} clen=0 rc"), ra, rb);
            assert_eq!(ra, 0, "{name}: clen==0 must return 0");
            eq_bytes(&format!("{name} clen=0 out"), &a, &b);
            assert!(a.iter().all(|&x| x == 0xA5), "{name}: clen==0 wrote output");

            if name != "crypto_stream_chacha20_ietf_ext" {
                let (x_c, x_r) = fpair::<XorFn>(&format!("{name}_xor"));
                let mut a = vec![0xA5u8; 64];
                let mut b = vec![0xA5u8; 64];
                let (pa, pb) = (a.as_mut_ptr(), b.as_mut_ptr());
                let ra = x_c(pa, pa, 0, n.as_ptr(), k.as_ptr());
                let rb = x_r(pb, pb, 0, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{name}_xor mlen=0 rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{name}_xor mlen=0 out"), &a, &b);
                assert!(a.iter().all(|&x| x == 0xA5), "{name}_xor: mlen==0 wrote");
            }
        }
        // the `_xor_ic` forms, 64-bit and 32-bit
        for (name, nb) in [
            ("crypto_stream_chacha20", 8usize),
            ("crypto_stream_salsa20", 8),
            ("crypto_stream_xsalsa20", 24),
            ("crypto_stream_xchacha20", 24),
        ] {
            let k = rng.bytes(32);
            let n = rng.bytes(nb);
            let (f_c, f_r) = fpair::<XorIc64Fn>(&format!("{name}_xor_ic"));
            for ic in [0u64, 1, u64::MAX] {
                let mut a = vec![0xA5u8; 64];
                let mut b = vec![0xA5u8; 64];
                let (pa, pb) = (a.as_mut_ptr(), b.as_mut_ptr());
                let ra = f_c(pa, pa, 0, n.as_ptr(), ic, k.as_ptr());
                let rb = f_r(pb, pb, 0, n.as_ptr(), ic, k.as_ptr());
                eq_i32(&format!("{name}_xor_ic ic={ic} mlen=0 rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{name}_xor_ic ic={ic} out"), &a, &b);
                assert!(a.iter().all(|&x| x == 0xA5));
            }
        }
        for name in [
            "crypto_stream_chacha20_ietf",
            "crypto_stream_chacha20_ietf_ext",
        ] {
            let k = rng.bytes(32);
            let n = rng.bytes(12);
            let (f_c, f_r) = fpair::<XorIc32Fn>(&format!("{name}_xor_ic"));
            for ic in [0u32, 1, 0xffff_ffff] {
                let mut a = vec![0xA5u8; 64];
                let mut b = vec![0xA5u8; 64];
                let (pa, pb) = (a.as_mut_ptr(), b.as_mut_ptr());
                let ra = f_c(pa, pa, 0, n.as_ptr(), ic, k.as_ptr());
                let rb = f_r(pb, pb, 0, n.as_ptr(), ic, k.as_ptr());
                eq_i32(&format!("{name}_xor_ic ic={ic} mlen=0 rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{name}_xor_ic ic={ic} out"), &a, &b);
                assert!(a.iter().all(|&x| x == 0xA5));
            }
        }
    }
}

#[test]
fn e494_502_stream_no_checks_always_zero() {
    // rows 494, 495 (salsa20: no length and no 64-bit ic check — the counter is
    // written straight into in[8..16) and allowed to wrap), 497, 498 (salsa2012
    // / salsa208), 499 (xsalsa20 family), 502 (crypto_stream / _xor default).
    let mut rng = Rng::new(0x0494);
    unsafe {
        for (name, nb, has_ic) in [
            ("crypto_stream_salsa20", 8usize, true),
            ("crypto_stream_salsa2012", 8, false),
            ("crypto_stream_salsa208", 8, false),
            ("crypto_stream_xsalsa20", 24, true),
            ("crypto_stream_xchacha20", 24, true),
            ("crypto_stream", 24, false),
        ] {
            let k = rng.bytes(32);
            let n = rng.bytes(nb);
            let (s_c, s_r) = fpair::<StreamFn>(name);
            let (x_c, x_r) = fpair::<XorFn>(&format!("{name}_xor"));
            for len in [0usize, 1, 64, 65, 1000] {
                let m = rng.bytes(len);
                let mut a = vec![0u8; len + G];
                let mut b = vec![0u8; len + G];
                let ra = s_c(a.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let rb = s_r(b.as_mut_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{name} len={len} rc"), ra, rb);
                assert_eq!(ra, 0, "{name}: always returns 0");
                eq_bytes(&format!("{name} len={len}"), &a, &b);
                let mut a = vec![0u8; len + G];
                let mut b = vec![0u8; len + G];
                let ra = x_c(a.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                let rb = x_r(b.as_mut_ptr(), m.as_ptr(), len as u64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{name}_xor len={len} rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{name}_xor len={len}"), &a, &b);
            }
            if has_ic {
                let (i_c, i_r) = fpair::<XorIc64Fn>(&format!("{name}_xor_ic"));
                // no ic range check exists: even u64::MAX (which wraps the
                // counter mid-message) must succeed identically.
                for ic in [
                    0u64,
                    1,
                    0xffff_ffff,
                    1u64 << 32,
                    (1u64 << 32) + 1,
                    1u64 << 63,
                    u64::MAX,
                    u64::MAX - 1,
                ] {
                    for len in [0usize, 1, 64, 65, 128, 129] {
                        let m = rng.bytes(len);
                        let mut a = vec![0u8; len + G];
                        let mut b = vec![0u8; len + G];
                        let ra = i_c(
                            a.as_mut_ptr(),
                            m.as_ptr(),
                            len as u64,
                            n.as_ptr(),
                            ic,
                            k.as_ptr(),
                        );
                        let rb = i_r(
                            b.as_mut_ptr(),
                            m.as_ptr(),
                            len as u64,
                            n.as_ptr(),
                            ic,
                            k.as_ptr(),
                        );
                        eq_i32(&format!("{name}_xor_ic ic={ic:#x} len={len} rc"), ra, rb);
                        assert_eq!(ra, 0, "{name}_xor_ic: no ic rejection exists");
                        eq_bytes(&format!("{name}_xor_ic ic={ic:#x} len={len}"), &a, &b);
                    }
                }
            }
            assert!(
                has_ic || !has_sym(&format!("{name}_xor_ic")),
                "{name}: expected no _xor_ic entry point"
            );
        }
    }
}

#[test]
fn e490_492_chacha20_ietf_ext_no_ic_guard() {
    // rows 490, 491, 492 — the ietf_ext entry points cap length at
    // SODIUM_SIZE_MAX (unreachable, see e484_492) and deliberately have NO ic
    // overflow guard, so ic values that `crypto_stream_chacha20_ietf_xor_ic`
    // rejects with sodium_misuse() must be ACCEPTED here, identically.
    let mut rng = Rng::new(0x0490);
    unsafe {
        let (f_c, f_r) = fpair::<XorIc32Fn>("crypto_stream_chacha20_ietf_ext_xor_ic");
        for ic in [0u32, 1, 0x7fff_ffff, 0xffff_fffe, 0xffff_ffff] {
            for len in [0usize, 1, 64, 65, 128, 129, 1000] {
                let k = rng.bytes(32);
                let n = rng.bytes(12);
                let m = rng.bytes(len);
                let mut a = vec![0u8; len + G];
                let mut b = vec![0u8; len + G];
                let ra = f_c(
                    a.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                );
                let rb = f_r(
                    b.as_mut_ptr(),
                    m.as_ptr(),
                    len as u64,
                    n.as_ptr(),
                    ic,
                    k.as_ptr(),
                );
                eq_i32(&format!("ext_xor_ic ic={ic:#x} len={len} rc"), ra, rb);
                assert_eq!(ra, 0, "row 491: ext form has no ic guard");
                eq_bytes(&format!("ext_xor_ic ic={ic:#x} len={len}"), &a, &b);
            }
        }
    }
}

// ===========================================================================
// crypto_core rows 503-505
// ===========================================================================
#[test]
fn e503_505_core_no_rejection_path() {
    // rows 503 (salsa20 / salsa2012 / salsa208), 504 (hsalsa20), 505
    // (hchacha20) — no argument is validated and the return value is always 0,
    // for `c == NULL` (sigma constants) and for any 16-byte `c`.
    let mut rng = Rng::new(0x0503);
    unsafe {
        for (name, outlen) in [
            ("crypto_core_salsa20", 64usize),
            ("crypto_core_salsa2012", 64),
            ("crypto_core_salsa208", 64),
            ("crypto_core_hsalsa20", 32),
            ("crypto_core_hchacha20", 32),
        ] {
            let (f_c, f_r) = fpair::<CoreFn>(name);
            for _ in 0..6 {
                let inp = rng.bytes(16);
                let k = rng.bytes(32);
                for cst in [None, Some(vec![0u8; 16]), Some(vec![0xffu8; 16]), Some(rng.bytes(16))] {
                    let cp = match &cst {
                        None => ptr::null(),
                        Some(v) => v.as_ptr(),
                    };
                    let mut a = vec![0xA5u8; outlen + G];
                    let mut b = vec![0xA5u8; outlen + G];
                    let ra = f_c(a.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    let rb = f_r(b.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), cp);
                    eq_i32(&format!("{name} rc"), ra, rb);
                    assert_eq!(ra, 0, "{name}: always returns 0");
                    eq_bytes(&format!("{name} out"), &a, &b);
                    assert!(a[outlen..].iter().all(|&x| x == 0xA5), "{name}: overrun");
                }
            }
        }
    }
}

// ===========================================================================
// rows 506-508: getters, nullable pointers, absent weak-key checks
// ===========================================================================
#[test]
fn e506_getters_never_fail() {
    // row 506 — every G4 getter is a constant read: stable across repeated
    // calls, identical in both libraries, and no failure path exists.
    let getters = [
        "crypto_aead_aegis128l_keybytes",
        "crypto_aead_aegis128l_nsecbytes",
        "crypto_aead_aegis128l_npubbytes",
        "crypto_aead_aegis128l_abytes",
        "crypto_aead_aegis128l_messagebytes_max",
        "crypto_aead_aegis256_keybytes",
        "crypto_aead_aegis256_nsecbytes",
        "crypto_aead_aegis256_npubbytes",
        "crypto_aead_aegis256_abytes",
        "crypto_aead_aegis256_messagebytes_max",
        "crypto_aead_aes256gcm_keybytes",
        "crypto_aead_aes256gcm_nsecbytes",
        "crypto_aead_aes256gcm_npubbytes",
        "crypto_aead_aes256gcm_abytes",
        "crypto_aead_aes256gcm_statebytes",
        "crypto_aead_aes256gcm_messagebytes_max",
        "crypto_aead_chacha20poly1305_keybytes",
        "crypto_aead_chacha20poly1305_nsecbytes",
        "crypto_aead_chacha20poly1305_npubbytes",
        "crypto_aead_chacha20poly1305_abytes",
        "crypto_aead_chacha20poly1305_messagebytes_max",
        "crypto_aead_chacha20poly1305_ietf_keybytes",
        "crypto_aead_chacha20poly1305_ietf_nsecbytes",
        "crypto_aead_chacha20poly1305_ietf_npubbytes",
        "crypto_aead_chacha20poly1305_ietf_abytes",
        "crypto_aead_chacha20poly1305_ietf_messagebytes_max",
        "crypto_aead_xchacha20poly1305_ietf_keybytes",
        "crypto_aead_xchacha20poly1305_ietf_nsecbytes",
        "crypto_aead_xchacha20poly1305_ietf_npubbytes",
        "crypto_aead_xchacha20poly1305_ietf_abytes",
        "crypto_aead_xchacha20poly1305_ietf_messagebytes_max",
        "crypto_secretbox_keybytes",
        "crypto_secretbox_noncebytes",
        "crypto_secretbox_macbytes",
        "crypto_secretbox_zerobytes",
        "crypto_secretbox_boxzerobytes",
        "crypto_secretbox_messagebytes_max",
        "crypto_secretbox_xsalsa20poly1305_keybytes",
        "crypto_secretbox_xsalsa20poly1305_noncebytes",
        "crypto_secretbox_xsalsa20poly1305_macbytes",
        "crypto_secretbox_xsalsa20poly1305_zerobytes",
        "crypto_secretbox_xsalsa20poly1305_boxzerobytes",
        "crypto_secretbox_xsalsa20poly1305_messagebytes_max",
        "crypto_secretbox_xchacha20poly1305_keybytes",
        "crypto_secretbox_xchacha20poly1305_noncebytes",
        "crypto_secretbox_xchacha20poly1305_macbytes",
        "crypto_secretbox_xchacha20poly1305_messagebytes_max",
        "crypto_secretstream_xchacha20poly1305_statebytes",
        "crypto_secretstream_xchacha20poly1305_abytes",
        "crypto_secretstream_xchacha20poly1305_headerbytes",
        "crypto_secretstream_xchacha20poly1305_keybytes",
        "crypto_secretstream_xchacha20poly1305_messagebytes_max",
        "crypto_stream_chacha20_keybytes",
        "crypto_stream_chacha20_noncebytes",
        "crypto_stream_chacha20_messagebytes_max",
        "crypto_stream_chacha20_ietf_keybytes",
        "crypto_stream_chacha20_ietf_noncebytes",
        "crypto_stream_chacha20_ietf_messagebytes_max",
        "crypto_stream_salsa20_keybytes",
        "crypto_stream_salsa20_noncebytes",
        "crypto_stream_salsa20_messagebytes_max",
        "crypto_stream_salsa2012_keybytes",
        "crypto_stream_salsa2012_noncebytes",
        "crypto_stream_salsa2012_messagebytes_max",
        "crypto_stream_salsa208_keybytes",
        "crypto_stream_salsa208_noncebytes",
        "crypto_stream_salsa208_messagebytes_max",
        "crypto_stream_xsalsa20_keybytes",
        "crypto_stream_xsalsa20_noncebytes",
        "crypto_stream_xsalsa20_messagebytes_max",
        "crypto_stream_xchacha20_keybytes",
        "crypto_stream_xchacha20_noncebytes",
        "crypto_stream_xchacha20_messagebytes_max",
        "crypto_stream_keybytes",
        "crypto_stream_noncebytes",
        "crypto_stream_messagebytes_max",
        "crypto_core_salsa20_outputbytes",
        "crypto_core_salsa20_inputbytes",
        "crypto_core_salsa20_keybytes",
        "crypto_core_salsa20_constbytes",
        "crypto_core_salsa2012_outputbytes",
        "crypto_core_salsa2012_inputbytes",
        "crypto_core_salsa2012_keybytes",
        "crypto_core_salsa2012_constbytes",
        "crypto_core_salsa208_outputbytes",
        "crypto_core_salsa208_inputbytes",
        "crypto_core_salsa208_keybytes",
        "crypto_core_salsa208_constbytes",
        "crypto_core_hsalsa20_outputbytes",
        "crypto_core_hsalsa20_inputbytes",
        "crypto_core_hsalsa20_keybytes",
        "crypto_core_hsalsa20_constbytes",
        "crypto_core_hchacha20_outputbytes",
        "crypto_core_hchacha20_inputbytes",
        "crypto_core_hchacha20_keybytes",
        "crypto_core_hchacha20_constbytes",
    ];
    for name in getters {
        let first = sz(name);
        for _ in 0..3 {
            assert_eq!(sz(name), first, "{name}: getter is not stable");
        }
    }
    // the u8 tag getters
    unsafe {
        for name in [
            "crypto_secretstream_xchacha20poly1305_tag_message",
            "crypto_secretstream_xchacha20poly1305_tag_push",
            "crypto_secretstream_xchacha20poly1305_tag_rekey",
            "crypto_secretstream_xchacha20poly1305_tag_final",
        ] {
            let (c, r) = fpair::<U8Fn>(name);
            let a = c();
            assert_eq!(a, r(), "{name}");
            assert_eq!(a, c(), "{name}: not stable");
        }
    }
    // `*_keygen` is `void` and cannot fail (only randombytes could abort)
    install_det_random();
    for (name, n) in [
        ("crypto_aead_aegis128l_keygen", 16usize),
        ("crypto_aead_aegis256_keygen", 32),
        ("crypto_aead_aes256gcm_keygen", 32),
        ("crypto_aead_chacha20poly1305_keygen", 32),
        ("crypto_aead_chacha20poly1305_ietf_keygen", 32),
        ("crypto_aead_xchacha20poly1305_ietf_keygen", 32),
        ("crypto_secretbox_keygen", 32),
        ("crypto_secretbox_xsalsa20poly1305_keygen", 32),
        ("crypto_secretstream_xchacha20poly1305_keygen", 32),
        ("crypto_stream_chacha20_keygen", 32),
        ("crypto_stream_chacha20_ietf_keygen", 32),
        ("crypto_stream_salsa20_keygen", 32),
        ("crypto_stream_salsa2012_keygen", 32),
        ("crypto_stream_salsa208_keygen", 32),
        ("crypto_stream_xsalsa20_keygen", 32),
        ("crypto_stream_xchacha20_keygen", 32),
        ("crypto_stream_keygen", 32),
    ] {
        let (c, r) = unsafe { fpair::<KeygenFn>(name) };
        let mut a = vec![0u8; n + G];
        let mut b = vec![0u8; n + G];
        det_reseed(0xC0FFEE);
        unsafe { c(a.as_mut_ptr()) };
        det_reseed(0xC0FFEE);
        unsafe { r(b.as_mut_ptr()) };
        eq_bytes(name, &a, &b);
        assert!(a[n..].iter().all(|&x| x == 0), "{name}: wrote past {n}");
    }
}

#[test]
fn e507_intentionally_nullable_pointers() {
    // row 507 — there is NO runtime NULL check anywhere in G4, so the only
    // pointers that may legitimately be NULL are:
    //   * `nsec` (always ignored via `(void) nsec`, never dereferenced, and
    //     passing a NON-NULL nsec must change nothing),
    //   * `clen_p` / `mlen_p` / `maclen_p` / `outlen_p` / `tag_p`,
    //   * `ad` when `adlen == 0`,
    //   * the `c` constant of the core primitives,
    //   * `m` in the AEAD `*_decrypt_detached` / secretbox `*_open_detached`
    //     verify-only mode (secretstream `pull` has NO such mode — row 482).
    // Passing NULL anywhere else is UB and is deliberately not exercised.
    let mut rng = Rng::new(0x0507);
    for pfx in [
        "crypto_aead_aegis128l",
        "crypto_aead_aegis256",
        "crypto_aead_chacha20poly1305",
        "crypto_aead_chacha20poly1305_ietf",
        "crypto_aead_xchacha20poly1305_ietf",
    ] {
        let a = Aead::new(pfx);
        let k = rng.bytes(a.kb);
        let npub = rng.bytes(a.npb);
        let m = rng.bytes(48);
        let mut nsec_scratch = rng.bytes(64);
        // non-NULL nsec must be ignored on encrypt...
        let mut out = [vec![0u8; 48 + a.ab], vec![0u8; 48 + a.ab]];
        let nsecs: [*const u8; 2] = [ptr::null(), nsec_scratch.as_ptr()];
        for (i, ns) in nsecs.iter().enumerate() {
            let mut b = vec![0u8; 48 + a.ab];
            unsafe {
                let ra = (a.enc_c)(
                    out[i].as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    48,
                    ptr::null(),
                    0,
                    *ns,
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (a.enc_r)(
                    b.as_mut_ptr(),
                    ptr::null_mut(),
                    m.as_ptr(),
                    48,
                    ptr::null(),
                    0,
                    *ns,
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{pfx} nsec[{i}] rc"), ra, rb);
            }
            eq_bytes(&format!("{pfx} nsec[{i}]"), &out[i], &b);
        }
        let (first, rest) = out.split_at(1);
        eq_bytes(
            &format!("{pfx}: non-NULL nsec must be ignored"),
            &first[0],
            &rest[0],
        );
        let ct = first[0].clone();
        // ...and on decrypt
        let nsp = nsec_scratch.as_mut_ptr();
        for i in 0..2usize {
            let nsec: *mut u8 = if i == 0 { ptr::null_mut() } else { nsp };
            let mut x = vec![0u8; 48 + G];
            let mut y = vec![0u8; 48 + G];
            unsafe {
                let ra = (a.dec_c)(
                    x.as_mut_ptr(),
                    ptr::null_mut(),
                    nsec,
                    ct.as_ptr(),
                    ct.len() as u64,
                    ptr::null(),
                    0,
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                let rb = (a.dec_r)(
                    y.as_mut_ptr(),
                    ptr::null_mut(),
                    nsec,
                    ct.as_ptr(),
                    ct.len() as u64,
                    ptr::null(),
                    0,
                    npub.as_ptr(),
                    k.as_ptr(),
                );
                eq_i32(&format!("{pfx} dec nsec[{i}] rc"), ra, rb);
                assert_eq!(ra, 0);
            }
            eq_bytes(&format!("{pfx} dec nsec[{i}]"), &x[..48], &m);
            eq_bytes(&format!("{pfx} dec nsec[{i}] C/Rust"), &x, &y);
        }
        // ad == NULL with adlen == 0 was already exercised above; m == NULL
        // verify-only mode:
        assert_eq!(
            a.dec_det_raw(
                &format!("{pfx} verify-only"),
                ct.as_ptr(),
                48,
                &ct[48..],
                ptr::null(),
                0,
                &npub,
                &k,
                true
            ),
            0
        );
    }
    // secretbox open_detached verify-only mode
    for pfx in [
        "crypto_secretbox",
        "crypto_secretbox_xchacha20poly1305",
    ] {
        let sb = Sb::new(pfx);
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        let m = rng.bytes(48);
        let mut ct = vec![0u8; 48 + sb.mb];
        unsafe {
            let r = (sb.easy_c)(ct.as_mut_ptr(), m.as_ptr(), 48, n.as_ptr(), k.as_ptr());
            assert_eq!(r, 0);
        }
        assert_eq!(
            sb.od_raw(
                &format!("{pfx} verify-only"),
                &ct[sb.mb..],
                &ct[..sb.mb],
                &n,
                &k,
                true
            ),
            0
        );
    }
    // core `c` constant may be NULL
    unsafe {
        for (name, outlen) in [
            ("crypto_core_salsa20", 64usize),
            ("crypto_core_hsalsa20", 32),
            ("crypto_core_hchacha20", 32),
        ] {
            let (f_c, f_r) = fpair::<CoreFn>(name);
            let inp = rng.bytes(16);
            let k = rng.bytes(32);
            let mut a = vec![0u8; outlen + G];
            let mut b = vec![0u8; outlen + G];
            let ra = f_c(a.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), ptr::null());
            let rb = f_r(b.as_mut_ptr(), inp.as_ptr(), k.as_ptr(), ptr::null());
            eq_i32(&format!("{name} c=NULL rc"), ra, rb);
            assert_eq!(ra, 0);
            eq_bytes(&format!("{name} c=NULL"), &a, &b);
        }
    }
}

#[test]
fn e508_no_weak_key_rejection_in_g4() {
    // row 508 — G4 has NO `sodium_is_zero` shared-key / weak-key rejection
    // (that lives in crypto_box / crypto_kx). An all-zero key, an all-zero
    // nonce and an all-0xff key must therefore all be accepted, identically.
    let mut rng = Rng::new(0x0508);
    for pfx in [
        "crypto_aead_aegis128l",
        "crypto_aead_aegis256",
        "crypto_aead_chacha20poly1305",
        "crypto_aead_chacha20poly1305_ietf",
        "crypto_aead_xchacha20poly1305_ietf",
    ] {
        let a = Aead::new(pfx);
        for (k, npub) in [
            (vec![0u8; a.kb], vec![0u8; a.npb]),
            (vec![0xffu8; a.kb], vec![0xffu8; a.npb]),
            (vec![0u8; a.kb], rng.bytes(a.npb)),
        ] {
            for mlen in [0usize, 1, 64] {
                let m = rng.bytes(mlen);
                let ct = a.seal(&m, &[], &npub, &k);
                assert_eq!(
                    a.dec_raw(
                        &format!("{pfx} weak-key mlen={mlen}"),
                        &ct,
                        ct.len() as u64,
                        &[],
                        &npub,
                        &k,
                        false
                    ),
                    0,
                    "row 508: no weak-key rejection exists in G4"
                );
            }
        }
    }
    unsafe {
        for (name, nb) in [
            ("crypto_stream_chacha20", 8usize),
            ("crypto_stream_salsa20", 8),
            ("crypto_stream_xsalsa20", 24),
            ("crypto_stream_xchacha20", 24),
            ("crypto_stream", 24),
        ] {
            let (f_c, f_r) = fpair::<StreamFn>(name);
            for (k, n) in [
                (vec![0u8; 32], vec![0u8; nb]),
                (vec![0xffu8; 32], vec![0xffu8; nb]),
            ] {
                let mut a = vec![0u8; 64 + G];
                let mut b = vec![0u8; 64 + G];
                let ra = f_c(a.as_mut_ptr(), 64, n.as_ptr(), k.as_ptr());
                let rb = f_r(b.as_mut_ptr(), 64, n.as_ptr(), k.as_ptr());
                eq_i32(&format!("{name} weak-key rc"), ra, rb);
                assert_eq!(ra, 0);
                eq_bytes(&format!("{name} weak-key"), &a, &b);
            }
        }
    }
}
