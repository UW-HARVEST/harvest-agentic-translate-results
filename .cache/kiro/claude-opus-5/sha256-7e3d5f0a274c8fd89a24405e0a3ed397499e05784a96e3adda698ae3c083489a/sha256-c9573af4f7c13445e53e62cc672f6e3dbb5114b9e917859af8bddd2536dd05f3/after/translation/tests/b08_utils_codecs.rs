//! Phase B — CONFIGS.md rows 62–66 (ipcrypt) and 142–159 (utils / codecs /
//! runtime / version / randombytes).
//!
//! Differential harness: the identical exported symbol is invoked in both the
//! C `.so` and the Rust `.so` (loaded via `libloading`) and the results are
//! compared byte-for-byte. Only VALID inputs are exercised here; invalid
//! inputs / misuse-abort paths live in a separate Phase C file, so we never
//! pass an out-of-range base64 `variant` (which would call `sodium_misuse()`
//! and abort the process).

mod common;
use common::*;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

// ---------------------------------------------------------------------------
// FFI signatures (taken verbatim from the public headers).
// ---------------------------------------------------------------------------

// crypto_ipcrypt
type IpEncDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type IpNdEnc = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type IpNdDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type Keygen = unsafe extern "C" fn(*mut u8);
type Sz = unsafe extern "C" fn() -> usize;

// utils
type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> c_int;
type Bin2B64 = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
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
type B64EncLen = unsafe extern "C" fn(usize, c_int) -> usize;
type Ip2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
type Bin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
type MemCmp = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
type Compare = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
type IsZero = unsafe extern "C" fn(*const u8, usize) -> c_int;
type Incr = unsafe extern "C" fn(*mut u8, usize);
type AddSub = unsafe extern "C" fn(*mut u8, *const u8, usize);
type MemZero = unsafe extern "C" fn(*mut u8, usize);
type StackZero = unsafe extern "C" fn(usize);
type Lock = unsafe extern "C" fn(*mut u8, usize) -> c_int;
type Malloc = unsafe extern "C" fn(usize) -> *mut u8;
type AllocArray = unsafe extern "C" fn(usize, usize) -> *mut u8;
type Free = unsafe extern "C" fn(*mut u8);
type MProtect = unsafe extern "C" fn(*mut u8) -> c_int;

// version
type VerStr = unsafe extern "C" fn() -> *const c_char;
type VerInt = unsafe extern "C" fn() -> c_int;

// randombytes
type RbBufDet = unsafe extern "C" fn(*mut u8, usize, *const u8);
type RbBuf = unsafe extern "C" fn(*mut u8, usize);
type RbRandom = unsafe extern "C" fn() -> u32;
type RbUniform = unsafe extern "C" fn(u32) -> u32;
type RbVoid = unsafe extern "C" fn();
type RbClose = unsafe extern "C" fn() -> c_int;
type RbName = unsafe extern "C" fn() -> *const c_char;
type RbSeedBytes = unsafe extern "C" fn() -> usize;

// ---------------------------------------------------------------------------
// Shared 16-byte IP input shapes used by all of rows 62–64.
// ---------------------------------------------------------------------------

fn ip_input_shapes() -> Vec<[u8; 16]> {
    let mut v: Vec<[u8; 16]> = Vec::new();
    // all-zero (== `::`)
    v.push([0u8; 16]);
    // all-0xff
    v.push([0xffu8; 16]);
    // IPv4-mapped 192.0.2.1  -> ::ffff:192.0.2.1
    let mut m = [0u8; 16];
    m[10] = 0xff;
    m[11] = 0xff;
    m[12] = 192;
    m[13] = 0;
    m[14] = 2;
    m[15] = 1;
    v.push(m);
    // IPv4-mapped 255.255.255.255
    let mut m2 = [0u8; 16];
    m2[10] = 0xff;
    m2[11] = 0xff;
    m2[12] = 255;
    m2[13] = 255;
    m2[14] = 255;
    m2[15] = 255;
    v.push(m2);
    // full IPv6 2001:db8::1
    let mut s = [0u8; 16];
    s[0] = 0x20;
    s[1] = 0x01;
    s[2] = 0x0d;
    s[3] = 0xb8;
    s[15] = 0x01;
    v.push(s);
    v
}

// ===========================================================================
// Rows 62 / 66 — deterministic ipcrypt encrypt/decrypt + keygen + constants
// ===========================================================================

#[test]
fn ipcrypt_deterministic_encrypt_decrypt() {
    let d = duo();
    let (cenc, renc) = d.pair::<IpEncDec>("crypto_ipcrypt_encrypt");
    let (cdec, rdec) = d.pair::<IpEncDec>("crypto_ipcrypt_decrypt");
    let mut rng = Rng::new(0x0019_9C1B_7A00);

    let shapes = ip_input_shapes();
    for iter in 0..2000usize {
        let key = rng.bytes(16);
        // Mix fixed shapes with fully random 16-byte inputs.
        let inp: Vec<u8> = if iter < shapes.len() {
            shapes[iter].to_vec()
        } else {
            rng.bytes(16)
        };
        let mut oc = [0u8; 16];
        let mut or = [0u8; 16];
        unsafe {
            cenc(oc.as_mut_ptr(), inp.as_ptr(), key.as_ptr());
            renc(or.as_mut_ptr(), inp.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_encrypt", &oc, &or);

        // Round-trip decrypt of the C ciphertext, compared across libraries.
        let mut dc = [0u8; 16];
        let mut dr = [0u8; 16];
        unsafe {
            cdec(dc.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
            rdec(dr.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_decrypt", &dc, &dr);
        eq_bytes("ipcrypt roundtrip (C)", &inp, &dc);
    }
}

#[test]
fn ipcrypt_pfx_deterministic_encrypt_decrypt() {
    let d = duo();
    if !d.has("crypto_ipcrypt_pfx_encrypt") {
        return;
    }
    let (cenc, renc) = d.pair::<IpEncDec>("crypto_ipcrypt_pfx_encrypt");
    let (cdec, rdec) = d.pair::<IpEncDec>("crypto_ipcrypt_pfx_decrypt");
    let mut rng = Rng::new(0x0050_4658_0000);
    let shapes = ip_input_shapes();
    for iter in 0..1500usize {
        let key = rng.bytes(32); // PFX_KEYBYTES == 32
        let inp: Vec<u8> = if iter < shapes.len() {
            shapes[iter].to_vec()
        } else {
            rng.bytes(16)
        };
        let mut oc = [0u8; 16];
        let mut or = [0u8; 16];
        unsafe {
            cenc(oc.as_mut_ptr(), inp.as_ptr(), key.as_ptr());
            renc(or.as_mut_ptr(), inp.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_pfx_encrypt", &oc, &or);
        let mut dc = [0u8; 16];
        let mut dr = [0u8; 16];
        unsafe {
            cdec(dc.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
            rdec(dr.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_pfx_decrypt", &dc, &dr);
        eq_bytes("ipcrypt_pfx roundtrip (C)", &inp, &dc);
    }
}

// ===========================================================================
// Row 63 — nd (8-byte tweak, 24-byte output)
// ===========================================================================

#[test]
fn ipcrypt_nd_encrypt_decrypt() {
    let d = duo();
    let (cenc, renc) = d.pair::<IpNdEnc>("crypto_ipcrypt_nd_encrypt");
    let (cdec, rdec) = d.pair::<IpNdDec>("crypto_ipcrypt_nd_decrypt");
    let mut rng = Rng::new(0x006E_6400_0000);
    let shapes = ip_input_shapes();
    for iter in 0..2000usize {
        let key = rng.bytes(16); // ND_KEYBYTES == 16
        let tweak = rng.bytes(8); // ND_TWEAKBYTES == 8
        let inp: Vec<u8> = if iter < shapes.len() {
            shapes[iter].to_vec()
        } else {
            rng.bytes(16)
        };
        let mut oc = [0u8; 24];
        let mut or = [0u8; 24];
        unsafe {
            cenc(oc.as_mut_ptr(), inp.as_ptr(), tweak.as_ptr(), key.as_ptr());
            renc(or.as_mut_ptr(), inp.as_ptr(), tweak.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_nd_encrypt", &oc, &or);

        let mut dc = [0u8; 16];
        let mut dr = [0u8; 16];
        unsafe {
            cdec(dc.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
            rdec(dr.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_nd_decrypt", &dc, &dr);
        eq_bytes("ipcrypt_nd roundtrip (C)", &inp, &dc);
    }
}

// ===========================================================================
// Row 64 — ndx (16-byte tweak, 32-byte output)
// ===========================================================================

#[test]
fn ipcrypt_ndx_encrypt_decrypt() {
    let d = duo();
    let (cenc, renc) = d.pair::<IpNdEnc>("crypto_ipcrypt_ndx_encrypt");
    let (cdec, rdec) = d.pair::<IpNdDec>("crypto_ipcrypt_ndx_decrypt");
    let mut rng = Rng::new(0x006E_4458_0000);
    let shapes = ip_input_shapes();
    for iter in 0..2000usize {
        let key = rng.bytes(32); // NDX_KEYBYTES == 32
        let tweak = rng.bytes(16); // NDX_TWEAKBYTES == 16
        let inp: Vec<u8> = if iter < shapes.len() {
            shapes[iter].to_vec()
        } else {
            rng.bytes(16)
        };
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        unsafe {
            cenc(oc.as_mut_ptr(), inp.as_ptr(), tweak.as_ptr(), key.as_ptr());
            renc(or.as_mut_ptr(), inp.as_ptr(), tweak.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_ndx_encrypt", &oc, &or);

        let mut dc = [0u8; 16];
        let mut dr = [0u8; 16];
        unsafe {
            cdec(dc.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
            rdec(dr.as_mut_ptr(), oc.as_ptr(), key.as_ptr());
        }
        eq_bytes("ipcrypt_ndx_decrypt", &dc, &dr);
        eq_bytes("ipcrypt_ndx roundtrip (C)", &inp, &dc);
    }
}

// ===========================================================================
// Row 65 — text-form ipcrypt entry points (only if exported)
// ===========================================================================

#[test]
fn ipcrypt_str_forms_if_exported() {
    let d = duo();
    // No `crypto_ipcrypt_*_str_*` symbols are exported by this build of
    // libsodium (verified via `nm -D`). Guard so the test degrades cleanly if
    // a future build adds them (and forces coverage to be written).
    for name in [
        "crypto_ipcrypt_str_encrypt",
        "crypto_ipcrypt_str_decrypt",
        "crypto_ipcrypt_nd_str_encrypt",
        "crypto_ipcrypt_nd_str_decrypt",
        "crypto_ipcrypt_ndx_str_encrypt",
        "crypto_ipcrypt_ndx_str_decrypt",
    ] {
        assert!(
            !d.has(name),
            "unexpected str-form symbol {name}; add differential coverage for it"
        );
    }
}

// ===========================================================================
// Row 66 — keygen (bounds/no-crash) + all ipcrypt *bytes / *keybytes constants
// ===========================================================================

#[test]
fn ipcrypt_keygen_matches() {
    let d = duo();
    for (name, kb_name) in [
        ("crypto_ipcrypt_keygen", "crypto_ipcrypt_keybytes"),
        ("crypto_ipcrypt_nd_keygen", "crypto_ipcrypt_nd_keybytes"),
        ("crypto_ipcrypt_ndx_keygen", "crypto_ipcrypt_ndx_keybytes"),
        ("crypto_ipcrypt_pfx_keygen", "crypto_ipcrypt_pfx_keybytes"),
    ] {
        if !d.has(name) {
            continue;
        }
        let (cf, rf) = d.pair::<Keygen>(name);
        let (kbc, _) = d.pair::<Sz>(kb_name);
        let kb = unsafe { kbc() };
        // Over-allocate with a canary tail; keygen must not write past kb.
        let mut cbuf = vec![0xAAu8; kb + 16];
        let mut rbuf = vec![0xAAu8; kb + 16];
        unsafe {
            cf(cbuf.as_mut_ptr());
            rf(rbuf.as_mut_ptr());
        }
        assert!(
            cbuf[kb..].iter().all(|&b| b == 0xAA),
            "{name}: C wrote past keybytes"
        );
        assert!(
            rbuf[kb..].iter().all(|&b| b == 0xAA),
            "{name}: Rust wrote past keybytes"
        );
    }
}

#[test]
fn ipcrypt_constants() {
    let d = duo();
    let expect: &[(&str, usize)] = &[
        ("crypto_ipcrypt_bytes", 16),
        ("crypto_ipcrypt_keybytes", 16),
        ("crypto_ipcrypt_nd_keybytes", 16),
        ("crypto_ipcrypt_nd_tweakbytes", 8),
        ("crypto_ipcrypt_nd_inputbytes", 16),
        ("crypto_ipcrypt_nd_outputbytes", 24),
        ("crypto_ipcrypt_ndx_keybytes", 32),
        ("crypto_ipcrypt_ndx_tweakbytes", 16),
        ("crypto_ipcrypt_ndx_inputbytes", 16),
        ("crypto_ipcrypt_ndx_outputbytes", 32),
        ("crypto_ipcrypt_pfx_keybytes", 32),
        ("crypto_ipcrypt_pfx_bytes", 16),
    ];
    for (name, want) in expect {
        let (cf, rf) = d.pair::<Sz>(name);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, *want, "{name}: C returned {c}, header says {want}");
        assert_eq!(c, r, "{name}: C={c} Rust={r}");
    }
}

// ===========================================================================
// Row 142 — sodium_bin2hex / sodium_hex2bin
// ===========================================================================

#[test]
fn bin2hex_hex2bin() {
    let d = duo();
    let (c_b2h, r_b2h) = d.pair::<Bin2Hex>("sodium_bin2hex");
    let (c_h2b, r_h2b) = d.pair::<Hex2Bin>("sodium_hex2bin");
    let mut rng = Rng::new(0x00B1_2E00_0000);

    let bin_lens = [0usize, 1, 2, 31, 32, 1000];
    // ignore sets: NULL, "", " :\n"
    let ignores: [Option<&CStr>; 3] = [
        None,
        Some(CStr::from_bytes_with_nul(b"\0").unwrap()),
        Some(CStr::from_bytes_with_nul(b" :\n\0").unwrap()),
    ];

    for &bl in &bin_lens {
        for _rep in 0..8 {
            let bin = rng.bytes(bl);
            let hex_max = bl * 2 + 1;
            let mut hc = vec![0i8; hex_max];
            let mut hr = vec![0i8; hex_max];
            unsafe {
                c_b2h(hc.as_mut_ptr() as *mut c_char, hex_max, bin.as_ptr(), bl);
                r_b2h(hr.as_mut_ptr() as *mut c_char, hex_max, bin.as_ptr(), bl);
            }
            eq_bytes(
                "bin2hex",
                &hc.iter().map(|&x| x as u8).collect::<Vec<_>>(),
                &hr.iter().map(|&x| x as u8).collect::<Vec<_>>(),
            );

            // Build a mixed-case hex string of the C output for hex2bin, and
            // for the " :\n" ignore case, inject those ignore chars.
            let hex_c = unsafe { CStr::from_ptr(hc.as_ptr() as *const c_char) }
                .to_bytes()
                .to_vec();
            for (ig_idx, ign) in ignores.iter().enumerate() {
                let mut input: Vec<u8> = hex_c
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| {
                        // Mixed-case: upper-case alpha nibbles on even indices.
                        if i % 2 == 0 {
                            (b as char).to_ascii_uppercase() as u8
                        } else {
                            (b as char).to_ascii_lowercase() as u8
                        }
                    })
                    .collect();
                if ig_idx == 2 {
                    // splice " :\n" tokens (all in the ignore set) between bytes
                    let mut spliced = Vec::with_capacity(input.len() * 2);
                    for (i, &b) in input.iter().enumerate() {
                        if i % 2 == 0 && i > 0 {
                            spliced.extend_from_slice(b" :\n");
                        }
                        spliced.push(b);
                    }
                    input = spliced;
                }
                input.push(0);
                let ig_ptr = ign.map_or(ptr::null(), |c| c.as_ptr());

                let mut binc = vec![0u8; bl.max(1)];
                let mut binr = vec![0u8; bl.max(1)];
                let mut lc = 0usize;
                let mut lr = 0usize;
                let mut endc: *const c_char = ptr::null();
                let mut endr: *const c_char = ptr::null();
                let (rc, ec) = with_errno(|| unsafe {
                    c_h2b(
                        binc.as_mut_ptr(),
                        binc.len(),
                        input.as_ptr() as *const c_char,
                        input.len() - 1,
                        ig_ptr,
                        &mut lc,
                        &mut endc,
                    )
                });
                let (rr, er) = with_errno(|| unsafe {
                    r_h2b(
                        binr.as_mut_ptr(),
                        binr.len(),
                        input.as_ptr() as *const c_char,
                        input.len() - 1,
                        ig_ptr,
                        &mut lr,
                        &mut endr,
                    )
                });
                eq_i32(&format!("hex2bin ret bl={bl} ig={ig_idx}"), rc, rr);
                eq_i32(&format!("hex2bin errno bl={bl} ig={ig_idx}"), ec, er);
                assert_eq!(lc, lr, "hex2bin bin_len bl={bl} ig={ig_idx}");
                // hex_end offset relative to input start must match.
                let offc = endc as usize - input.as_ptr() as usize;
                let offr = endr as usize - input.as_ptr() as usize;
                assert_eq!(offc, offr, "hex2bin hex_end bl={bl} ig={ig_idx}");
                eq_bytes(
                    &format!("hex2bin out bl={bl} ig={ig_idx}"),
                    &binc[..lc],
                    &binr[..lr],
                );
            }
        }
    }
}

// ===========================================================================
// Row 143 — sodium_bin2base64 / sodium_base642bin (variant × bin_len sweep)
// ===========================================================================

const VARIANTS: [c_int; 4] = [1, 3, 5, 7];

#[test]
fn bin2base64_base642bin() {
    let d = duo();
    let (c_enc, r_enc) = d.pair::<Bin2B64>("sodium_bin2base64");
    let (c_dec, r_dec) = d.pair::<B642Bin>("sodium_base642bin");
    let (c_len, r_len) = d.pair::<B64EncLen>("sodium_base64_encoded_len");
    let mut rng = Rng::new(0x00B6_422B_0000);

    let bin_lens = [0usize, 1, 2, 3, 4, 5, 31, 32, 33, 1000];
    for &v in &VARIANTS {
        for &bl in &bin_lens {
            for _rep in 0..6 {
                let bin = rng.bytes(bl);
                let enc_len_c = unsafe { c_len(bl, v) };
                let enc_len_r = unsafe { r_len(bl, v) };
                assert_eq!(enc_len_c, enc_len_r, "encoded_len v={v} bl={bl}");
                let cap = enc_len_c.max(1);
                let mut bc = vec![0i8; cap];
                let mut br = vec![0i8; cap];
                unsafe {
                    c_enc(bc.as_mut_ptr() as *mut c_char, cap, bin.as_ptr(), bl, v);
                    r_enc(br.as_mut_ptr() as *mut c_char, cap, bin.as_ptr(), bl, v);
                }
                eq_bytes(
                    &format!("bin2base64 v={v} bl={bl}"),
                    &bc.iter().map(|&x| x as u8).collect::<Vec<_>>(),
                    &br.iter().map(|&x| x as u8).collect::<Vec<_>>(),
                );

                // Decode the C-produced base64 back to binary in both libs.
                let b64 = unsafe { CStr::from_ptr(bc.as_ptr() as *const c_char) };
                let b64_bytes = b64.to_bytes();
                let mut oc = vec![0u8; bl.max(1)];
                let mut or = vec![0u8; bl.max(1)];
                let mut lc = 0usize;
                let mut lr = 0usize;
                let (rc, ec) = with_errno(|| unsafe {
                    c_dec(
                        oc.as_mut_ptr(),
                        oc.len(),
                        b64_bytes.as_ptr() as *const c_char,
                        b64_bytes.len(),
                        ptr::null(),
                        &mut lc,
                        ptr::null_mut(),
                        v,
                    )
                });
                let (rr, er) = with_errno(|| unsafe {
                    r_dec(
                        or.as_mut_ptr(),
                        or.len(),
                        b64_bytes.as_ptr() as *const c_char,
                        b64_bytes.len(),
                        ptr::null(),
                        &mut lr,
                        ptr::null_mut(),
                        v,
                    )
                });
                eq_i32(&format!("base642bin ret v={v} bl={bl}"), rc, rr);
                eq_i32(&format!("base642bin errno v={v} bl={bl}"), ec, er);
                assert_eq!(lc, lr, "base642bin bin_len v={v} bl={bl}");
                eq_bytes(
                    &format!("base642bin out v={v} bl={bl}"),
                    &oc[..lc],
                    &or[..lr],
                );
                // Round-trip must recover the original bytes (C side).
                eq_bytes(&format!("base64 roundtrip v={v} bl={bl}"), &bin, &oc[..lc]);
            }
        }
    }
}

// ===========================================================================
// Row 144 — sodium_base64_encoded_len (variant × bin_len 0..=64)
// ===========================================================================

#[test]
fn base64_encoded_len() {
    let d = duo();
    let (c_len, r_len) = d.pair::<B64EncLen>("sodium_base64_encoded_len");
    for &v in &VARIANTS {
        for bl in 0..=64usize {
            let c = unsafe { c_len(bl, v) };
            let r = unsafe { r_len(bl, v) };
            assert_eq!(c, r, "encoded_len v={v} bl={bl}: C={c} Rust={r}");
        }
    }
}

// ===========================================================================
// Row 145 — sodium_base642bin with ignore + whitespace-injected input
// ===========================================================================

#[test]
fn base642bin_with_ignore() {
    let d = duo();
    let (c_enc, _) = d.pair::<Bin2B64>("sodium_bin2base64");
    let (c_dec, r_dec) = d.pair::<B642Bin>("sodium_base642bin");
    let mut rng = Rng::new(0x00B6_4915_0000);
    let ignore = CStr::from_bytes_with_nul(b" \t\r\n\0").unwrap();

    let bin_lens = [0usize, 1, 2, 3, 16, 31, 32, 33, 100];
    for &v in &VARIANTS {
        for &bl in &bin_lens {
            for _rep in 0..6 {
                let bin = rng.bytes(bl);
                let cap = 4 * (bl / 3 + 2) + 8;
                let mut bc = vec![0i8; cap];
                unsafe {
                    c_enc(bc.as_mut_ptr() as *mut c_char, cap, bin.as_ptr(), bl, v);
                }
                let b64 = unsafe { CStr::from_ptr(bc.as_ptr() as *const c_char) }
                    .to_bytes()
                    .to_vec();
                // Inject whitespace (all in the ignore set) between chars.
                let ws: [&[u8]; 4] = [b" ", b"\t", b"\r", b"\n"];
                let mut input: Vec<u8> = Vec::with_capacity(b64.len() * 2);
                for (i, &ch) in b64.iter().enumerate() {
                    if i > 0 {
                        input.extend_from_slice(ws[i % ws.len()]);
                    }
                    input.push(ch);
                }
                // Trailing whitespace too (still valid; consumed by ignore).
                input.extend_from_slice(b" \n");

                let mut oc = vec![0u8; bl.max(1)];
                let mut or = vec![0u8; bl.max(1)];
                let mut lc = 0usize;
                let mut lr = 0usize;
                let (rc, ec) = with_errno(|| unsafe {
                    c_dec(
                        oc.as_mut_ptr(),
                        oc.len(),
                        input.as_ptr() as *const c_char,
                        input.len(),
                        ignore.as_ptr(),
                        &mut lc,
                        ptr::null_mut(),
                        v,
                    )
                });
                let (rr, er) = with_errno(|| unsafe {
                    r_dec(
                        or.as_mut_ptr(),
                        or.len(),
                        input.as_ptr() as *const c_char,
                        input.len(),
                        ignore.as_ptr(),
                        &mut lr,
                        ptr::null_mut(),
                        v,
                    )
                });
                eq_i32(&format!("base642bin(ignore) ret v={v} bl={bl}"), rc, rr);
                eq_i32(&format!("base642bin(ignore) errno v={v} bl={bl}"), ec, er);
                assert_eq!(lc, lr, "base642bin(ignore) bin_len v={v} bl={bl}");
                eq_bytes(
                    &format!("base642bin(ignore) out v={v} bl={bl}"),
                    &oc[..lc],
                    &or[..lr],
                );
            }
        }
    }
}

// ===========================================================================
// Row 146 — sodium_base642bin b64_end out-param: non-NULL and NULL
// ===========================================================================

#[test]
fn base642bin_b64_end() {
    let d = duo();
    let (c_enc, _) = d.pair::<Bin2B64>("sodium_bin2base64");
    let (c_dec, r_dec) = d.pair::<B642Bin>("sodium_base642bin");
    let mut rng = Rng::new(0x00B6_4E4D_0000);

    for &v in &VARIANTS {
        for &bl in &[1usize, 2, 3, 15, 16, 17, 64] {
            let bin = rng.bytes(bl);
            let cap = 4 * (bl / 3 + 2) + 8;
            let mut bc = vec![0i8; cap];
            unsafe {
                c_enc(bc.as_mut_ptr() as *mut c_char, cap, bin.as_ptr(), bl, v);
            }
            let b64 = unsafe { CStr::from_ptr(bc.as_ptr() as *const c_char) }
                .to_bytes()
                .to_vec();

            // --- non-NULL b64_end ---
            let mut oc = vec![0u8; bl];
            let mut or = vec![0u8; bl];
            let mut lc = 0usize;
            let mut lr = 0usize;
            let mut endc: *const c_char = ptr::null();
            let mut endr: *const c_char = ptr::null();
            let (rc, _) = with_errno(|| unsafe {
                c_dec(
                    oc.as_mut_ptr(),
                    oc.len(),
                    b64.as_ptr() as *const c_char,
                    b64.len(),
                    ptr::null(),
                    &mut lc,
                    &mut endc,
                    v,
                )
            });
            let (rr, _) = with_errno(|| unsafe {
                r_dec(
                    or.as_mut_ptr(),
                    or.len(),
                    b64.as_ptr() as *const c_char,
                    b64.len(),
                    ptr::null(),
                    &mut lr,
                    &mut endr,
                    v,
                )
            });
            eq_i32(&format!("b64_end(nonnull) ret v={v} bl={bl}"), rc, rr);
            let offc = endc as usize - b64.as_ptr() as usize;
            let offr = endr as usize - b64.as_ptr() as usize;
            assert_eq!(offc, offr, "b64_end offset v={v} bl={bl}");
            eq_bytes(&format!("b64_end out v={v} bl={bl}"), &oc[..lc], &or[..lr]);

            // --- NULL b64_end (must fully consume) ---
            let mut oc2 = vec![0u8; bl];
            let mut or2 = vec![0u8; bl];
            let mut lc2 = 0usize;
            let mut lr2 = 0usize;
            let (rc2, ec2) = with_errno(|| unsafe {
                c_dec(
                    oc2.as_mut_ptr(),
                    oc2.len(),
                    b64.as_ptr() as *const c_char,
                    b64.len(),
                    ptr::null(),
                    &mut lc2,
                    ptr::null_mut(),
                    v,
                )
            });
            let (rr2, er2) = with_errno(|| unsafe {
                r_dec(
                    or2.as_mut_ptr(),
                    or2.len(),
                    b64.as_ptr() as *const c_char,
                    b64.len(),
                    ptr::null(),
                    &mut lr2,
                    ptr::null_mut(),
                    v,
                )
            });
            eq_i32(&format!("b64_end(null) ret v={v} bl={bl}"), rc2, rr2);
            eq_i32(&format!("b64_end(null) errno v={v} bl={bl}"), ec2, er2);
            assert_eq!(lc2, lr2, "b64_end(null) bin_len v={v} bl={bl}");
            eq_bytes(
                &format!("b64_end(null) out v={v} bl={bl}"),
                &oc2[..lc2],
                &or2[..lr2],
            );
        }
    }
}

// ===========================================================================
// Row 147 — sodium_ip2bin / sodium_bin2ip
// ===========================================================================

#[test]
fn ip2bin_bin2ip() {
    let d = duo();
    let (c_i2b, r_i2b) = d.pair::<Ip2Bin>("sodium_ip2bin");
    let (c_b2i, r_b2i) = d.pair::<Bin2Ip>("sodium_bin2ip");

    // Valid textual forms only.
    let inputs: &[&str] = &[
        "0.0.0.0",
        "127.0.0.1",
        "192.0.2.1",
        "255.255.255.255",
        "::",
        "::1",
        "2001:db8::1",
        "2001:0db8:0000:0000:0000:0000:0000:0001",
        "fe80::1",
        "::ffff:192.0.2.1",
        "::ffff:255.255.255.255",
        "2001:db8:0:0:0:0:2:1",
        "fe80::1%eth0",
        "fe80::1%1",
        "abcd:ef01:2345:6789:abcd:ef01:2345:6789",
    ];

    for ip in inputs {
        let bytes = ip.as_bytes();
        let mut bc = [0u8; 16];
        let mut br = [0u8; 16];
        let (rc, _) = with_errno(|| unsafe {
            c_i2b(bc.as_mut_ptr(), bytes.as_ptr() as *const c_char, bytes.len())
        });
        let (rr, _) = with_errno(|| unsafe {
            r_i2b(br.as_mut_ptr(), bytes.as_ptr() as *const c_char, bytes.len())
        });
        eq_i32(&format!("ip2bin ret `{ip}`"), rc, rr);
        eq_bytes(&format!("ip2bin `{ip}`"), &bc, &br);

        // Now render the 16-byte form back to text in both libs.
        let mut sc = vec![0i8; 46];
        let mut sr = vec![0i8; 46];
        let pc = unsafe { c_b2i(sc.as_mut_ptr() as *mut c_char, 46, bc.as_ptr()) };
        let pr = unsafe { r_b2i(sr.as_mut_ptr() as *mut c_char, 46, br.as_ptr()) };
        assert_eq!(pc.is_null(), pr.is_null(), "bin2ip nullness `{ip}`");
        if !pc.is_null() {
            let strc = unsafe { CStr::from_ptr(sc.as_ptr() as *const c_char) };
            let strr = unsafe { CStr::from_ptr(sr.as_ptr() as *const c_char) };
            assert_eq!(strc, strr, "bin2ip text `{ip}`");
        }
    }

    // Also sweep random 16-byte binaries through bin2ip -> ip2bin round-trip.
    let mut rng = Rng::new(0x0019_2B1B_0000);
    for _ in 0..500 {
        let mut bin = rng.bytes(16);
        // Randomly make some IPv4-mapped.
        if rng.u8() & 1 == 0 {
            for i in 0..10 {
                bin[i] = 0;
            }
            bin[10] = 0xff;
            bin[11] = 0xff;
        }
        let mut sc = vec![0i8; 46];
        let mut sr = vec![0i8; 46];
        let pc = unsafe { c_b2i(sc.as_mut_ptr() as *mut c_char, 46, bin.as_ptr()) };
        let pr = unsafe { r_b2i(sr.as_mut_ptr() as *mut c_char, 46, bin.as_ptr()) };
        assert_eq!(pc.is_null(), pr.is_null(), "bin2ip nullness random");
        if !pc.is_null() {
            let strc = unsafe { CStr::from_ptr(sc.as_ptr() as *const c_char) };
            let strr = unsafe { CStr::from_ptr(sr.as_ptr() as *const c_char) };
            assert_eq!(strc, strr, "bin2ip text random");
            // Round-trip: parse the rendered text back to bytes and confirm.
            let txt = strc.to_bytes();
            let mut bc2 = [0u8; 16];
            let rc2 = unsafe {
                c_i2b(bc2.as_mut_ptr(), txt.as_ptr() as *const c_char, txt.len())
            };
            assert_eq!(rc2, 0, "ip2bin re-parse of rendered text failed");
            eq_bytes("bin2ip/ip2bin roundtrip", &bin, &bc2);
        }
    }
}

// ===========================================================================
// Row 148 — sodium_pad / sodium_unpad
// ===========================================================================

#[test]
fn pad_unpad() {
    let d = duo();
    let (c_pad, r_pad) = d.pair::<Pad>("sodium_pad");
    let (c_unpad, r_unpad) = d.pair::<Unpad>("sodium_unpad");
    let mut rng = Rng::new(0x009A_D900_0000);

    let blocksizes = [1usize, 2, 15, 16, 17, 64];
    let unpadded = [0usize, 1, 15, 16, 17, 63, 64, 65];
    for &bs in &blocksizes {
        for &ul in &unpadded {
            // padded length <= this; give generous max.
            let max_buflen = ul + bs + 64;
            let base = rng.bytes(ul);

            let mut cbuf = base.clone();
            cbuf.resize(max_buflen, 0);
            let mut rbuf = base.clone();
            rbuf.resize(max_buflen, 0);
            let mut plc = 0usize;
            let mut plr = 0usize;
            let (rc, ec) =
                with_errno(|| unsafe { c_pad(&mut plc, cbuf.as_mut_ptr(), ul, bs, max_buflen) });
            let (rr, er) =
                with_errno(|| unsafe { r_pad(&mut plr, rbuf.as_mut_ptr(), ul, bs, max_buflen) });
            eq_i32(&format!("pad ret bs={bs} ul={ul}"), rc, rr);
            eq_i32(&format!("pad errno bs={bs} ul={ul}"), ec, er);
            assert_eq!(plc, plr, "pad padded_len bs={bs} ul={ul}");
            if rc == 0 {
                eq_bytes(
                    &format!("pad buf bs={bs} ul={ul}"),
                    &cbuf[..plc],
                    &rbuf[..plr],
                );
                // unpad the padded buffer.
                let mut ulc = 0usize;
                let mut ulr = 0usize;
                let (urc, uec) =
                    with_errno(|| unsafe { c_unpad(&mut ulc, cbuf.as_ptr(), plc, bs) });
                let (urr, uer) =
                    with_errno(|| unsafe { r_unpad(&mut ulr, rbuf.as_ptr(), plr, bs) });
                eq_i32(&format!("unpad ret bs={bs} ul={ul}"), urc, urr);
                eq_i32(&format!("unpad errno bs={bs} ul={ul}"), uec, uer);
                assert_eq!(ulc, ulr, "unpad unpadded_len bs={bs} ul={ul}");
                assert_eq!(ulc, ul, "unpad must recover original len bs={bs} ul={ul}");
            }
        }
    }
}

// ===========================================================================
// Rows 149 / 150 — memcmp / compare / is_zero / increment / add / sub,
// including full carry/borrow chains with all-0xff operands.
// ===========================================================================

#[test]
fn compare_family() {
    let d = duo();
    let (c_mc, r_mc) = d.pair::<MemCmp>("sodium_memcmp");
    let (c_cmp, r_cmp) = d.pair::<Compare>("sodium_compare");
    let (c_iz, r_iz) = d.pair::<IsZero>("sodium_is_zero");
    let (c_inc, r_inc) = d.pair::<Incr>("sodium_increment");
    let (c_add, r_add) = d.pair::<AddSub>("sodium_add");
    let (c_sub, r_sub) = d.pair::<AddSub>("sodium_sub");
    let mut rng = Rng::new(0x00C0_15A0_0000);

    let lens = [0usize, 1, 8, 16, 32, 64];
    for &len in &lens {
        // Operand generators: all-zero, all-0xff, mixed, random, near-equal.
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        cases.push((vec![0u8; len], vec![0u8; len]));
        cases.push((vec![0xffu8; len], vec![0xffu8; len]));
        cases.push((vec![0u8; len], vec![0xffu8; len]));
        cases.push((vec![0xffu8; len], vec![0u8; len]));
        for _ in 0..40 {
            let a = rng.bytes(len);
            let b = rng.bytes(len);
            cases.push((a, b));
        }
        // near-equal: b = a with one byte bumped
        if len > 0 {
            let a = rng.bytes(len);
            let mut b = a.clone();
            let idx = rng.below(len);
            b[idx] = b[idx].wrapping_add(1);
            cases.push((a, b));
        }

        for (a, b) in &cases {
            // memcmp
            let (mc, mr) = unsafe {
                (
                    c_mc(a.as_ptr(), b.as_ptr(), len),
                    r_mc(a.as_ptr(), b.as_ptr(), len),
                )
            };
            eq_i32(&format!("memcmp len={len}"), mc, mr);
            // compare
            let (cc, cr) = unsafe {
                (
                    c_cmp(a.as_ptr(), b.as_ptr(), len),
                    r_cmp(a.as_ptr(), b.as_ptr(), len),
                )
            };
            eq_i32(&format!("compare len={len}"), cc, cr);
            // is_zero
            let (zc, zr) = unsafe { (c_iz(a.as_ptr(), len), r_iz(a.as_ptr(), len)) };
            eq_i32(&format!("is_zero len={len}"), zc, zr);

            // increment
            let mut ic = a.clone();
            let mut ir = a.clone();
            unsafe {
                c_inc(ic.as_mut_ptr(), len);
                r_inc(ir.as_mut_ptr(), len);
            }
            eq_bytes(&format!("increment len={len}"), &ic, &ir);

            // add
            let mut ac = a.clone();
            let mut ar = a.clone();
            unsafe {
                c_add(ac.as_mut_ptr(), b.as_ptr(), len);
                r_add(ar.as_mut_ptr(), b.as_ptr(), len);
            }
            eq_bytes(&format!("add len={len}"), &ac, &ar);

            // sub
            let mut sc = a.clone();
            let mut sr = a.clone();
            unsafe {
                c_sub(sc.as_mut_ptr(), b.as_ptr(), len);
                r_sub(sr.as_mut_ptr(), b.as_ptr(), len);
            }
            eq_bytes(&format!("sub len={len}"), &sc, &sr);
        }
    }
}

/// CONFIGS.md row 150 — `sodium_increment` overflow and `sodium_add`/`sodium_sub`
/// full carry/borrow chains driven with all-0xff operands at every length.
#[test]
fn increment_add_sub_full_carry() {
    let d = duo();
    let (c_inc, r_inc) = d.pair::<Incr>("sodium_increment");
    let (c_add, r_add) = d.pair::<AddSub>("sodium_add");
    let (c_sub, r_sub) = d.pair::<AddSub>("sodium_sub");
    let lens = [0usize, 1, 8, 16, 32, 64];
    for &len in &lens {
        // increment overflow: all-0xff -> wraps to zero, full carry chain.
        let mut ic = vec![0xffu8; len];
        let mut ir = vec![0xffu8; len];
        unsafe {
            c_inc(ic.as_mut_ptr(), len);
            r_inc(ir.as_mut_ptr(), len);
        }
        eq_bytes(&format!("increment overflow len={len}"), &ic, &ir);

        // add full carry: 0xff + 0xff
        let ff = vec![0xffu8; len];
        let mut ac = vec![0xffu8; len];
        let mut ar = vec![0xffu8; len];
        unsafe {
            c_add(ac.as_mut_ptr(), ff.as_ptr(), len);
            r_add(ar.as_mut_ptr(), ff.as_ptr(), len);
        }
        eq_bytes(&format!("add full-carry len={len}"), &ac, &ar);

        // sub full borrow: 0x00 - 0xff
        let mut sc = vec![0u8; len];
        let mut sr = vec![0u8; len];
        unsafe {
            c_sub(sc.as_mut_ptr(), ff.as_ptr(), len);
            r_sub(sr.as_mut_ptr(), ff.as_ptr(), len);
        }
        eq_bytes(&format!("sub full-borrow len={len}"), &sc, &sr);
    }
}

// ===========================================================================
// Row 151 — sodium_stackzero / sodium_memzero / sodium_mlock / sodium_munlock
// ===========================================================================

#[test]
fn memzero_stackzero_lock() {
    let d = duo();
    let (c_mz, r_mz) = d.pair::<MemZero>("sodium_memzero");
    let (c_sz, r_sz) = d.pair::<StackZero>("sodium_stackzero");
    let (c_ml, r_ml) = d.pair::<Lock>("sodium_mlock");
    let (c_mul, r_mul) = d.pair::<Lock>("sodium_munlock");
    let mut rng = Rng::new(0x0033_2000_0000);

    for &len in &[0usize, 1, 7, 16, 32, 64, 1000] {
        let src = rng.bytes(len);
        let mut cbuf = src.clone();
        let mut rbuf = src.clone();
        unsafe {
            c_mz(cbuf.as_mut_ptr(), len);
            r_mz(rbuf.as_mut_ptr(), len);
        }
        assert!(cbuf.iter().all(|&b| b == 0), "C memzero len={len}");
        assert!(rbuf.iter().all(|&b| b == 0), "Rust memzero len={len}");
        eq_bytes(&format!("memzero len={len}"), &cbuf, &rbuf);

        // stackzero: just must not crash and returns nothing.
        unsafe {
            c_sz(len);
            r_sz(len);
        }

        // mlock / munlock: return value shape must match; no crash.
        let mut mbuf_c = vec![0u8; len.max(1)];
        let mut mbuf_r = vec![0u8; len.max(1)];
        let (mlc, _) = with_errno(|| unsafe { c_ml(mbuf_c.as_mut_ptr(), len) });
        let (mlr, _) = with_errno(|| unsafe { r_ml(mbuf_r.as_mut_ptr(), len) });
        eq_i32(&format!("mlock ret len={len}"), mlc, mlr);
        let (muc, _) = with_errno(|| unsafe { c_mul(mbuf_c.as_mut_ptr(), len) });
        let (mur, _) = with_errno(|| unsafe { r_mul(mbuf_r.as_mut_ptr(), len) });
        eq_i32(&format!("munlock ret len={len}"), muc, mur);
    }
}

// ===========================================================================
// Row 152 — sodium_malloc / allocarray / free / mprotect_* transitions.
// Compare return-code / null-ness, NOT addresses.
// ===========================================================================

#[test]
fn secure_alloc_and_mprotect() {
    let d = duo();
    let (c_malloc, r_malloc) = d.pair::<Malloc>("sodium_malloc");
    let (c_aa, r_aa) = d.pair::<AllocArray>("sodium_allocarray");
    let (c_free, r_free) = d.pair::<Free>("sodium_free");
    let (c_no, r_no) = d.pair::<MProtect>("sodium_mprotect_noaccess");
    let (c_ro, r_ro) = d.pair::<MProtect>("sodium_mprotect_readonly");
    let (c_rw, r_rw) = d.pair::<MProtect>("sodium_mprotect_readwrite");

    for &size in &[0usize, 1, 4095, 4096, 4097] {
        let pc = unsafe { c_malloc(size) };
        let pr = unsafe { r_malloc(size) };
        assert_eq!(pc.is_null(), pr.is_null(), "malloc nullness size={size}");
        if !pc.is_null() {
            // Writable region: fill it (must not crash under either lib).
            unsafe {
                if size > 0 {
                    ptr::write_bytes(pc, 0x11, size);
                    ptr::write_bytes(pr, 0x11, size);
                }
            }
            // noaccess -> readonly -> readwrite transitions, compare ret codes.
            let (nc, _) = with_errno(|| unsafe { c_no(pc) });
            let (nr, _) = with_errno(|| unsafe { r_no(pr) });
            eq_i32(&format!("mprotect_noaccess size={size}"), nc, nr);
            let (roc, _) = with_errno(|| unsafe { c_ro(pc) });
            let (ror, _) = with_errno(|| unsafe { r_ro(pr) });
            eq_i32(&format!("mprotect_readonly size={size}"), roc, ror);
            let (rwc, _) = with_errno(|| unsafe { c_rw(pc) });
            let (rwr, _) = with_errno(|| unsafe { r_rw(pr) });
            eq_i32(&format!("mprotect_readwrite size={size}"), rwc, rwr);
            unsafe {
                c_free(pc);
                r_free(pr);
            }
        }
    }

    // allocarray: count x size shapes.
    for &(count, size) in &[(0usize, 16usize), (1, 16), (16, 32), (1000, 1), (1, 4097)] {
        let pc = unsafe { c_aa(count, size) };
        let pr = unsafe { r_aa(count, size) };
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "allocarray nullness count={count} size={size}"
        );
        if !pc.is_null() {
            unsafe {
                c_free(pc);
                r_free(pr);
            }
        }
    }

    // free(NULL) must be a no-op in both.
    unsafe {
        c_free(ptr::null_mut());
        r_free(ptr::null_mut());
    }
}

// ===========================================================================
// Row 154 — version string / library version accessors / library_minimal
// ===========================================================================

#[test]
fn version_accessors() {
    let d = duo();
    let (c_vs, r_vs) = d.pair::<VerStr>("sodium_version_string");
    let (c_maj, r_maj) = d.pair::<VerInt>("sodium_library_version_major");
    let (c_min, r_min) = d.pair::<VerInt>("sodium_library_version_minor");
    let (c_lm, r_lm) = d.pair::<VerInt>("sodium_library_minimal");

    let cs = unsafe { CStr::from_ptr(c_vs()) };
    let rs = unsafe { CStr::from_ptr(r_vs()) };
    assert_eq!(cs, rs, "sodium_version_string");

    unsafe {
        eq_i32("library_version_major", c_maj(), r_maj());
        eq_i32("library_version_minor", c_min(), r_min());
        eq_i32("library_minimal", c_lm(), r_lm());
    }
}

// ===========================================================================
// Row 157 — randombytes_buf_deterministic: FIXED seeds, byte-identical output
// ===========================================================================

#[test]
fn randombytes_buf_deterministic_identical() {
    let d = duo();
    let (c_f, r_f) = d.pair::<RbBufDet>("randombytes_buf_deterministic");
    let mut seed_rng = Rng::new(0x00DE_7E2B_0000);

    for &size in &[0usize, 1, 63, 64, 65, 1000] {
        // Several fixed seeds per size.
        for _ in 0..8 {
            let seed = seed_rng.bytes(32);
            let mut oc = vec![0u8; size.max(1)];
            let mut or = vec![0u8; size.max(1)];
            unsafe {
                c_f(oc.as_mut_ptr(), size, seed.as_ptr());
                r_f(or.as_mut_ptr(), size, seed.as_ptr());
            }
            eq_bytes(
                &format!("randombytes_buf_deterministic size={size}"),
                &oc[..size],
                &or[..size],
            );
        }
        // Also an all-zero seed and an all-0xff seed.
        for fill in [0u8, 0xff] {
            let seed = vec![fill; 32];
            let mut oc = vec![0u8; size.max(1)];
            let mut or = vec![0u8; size.max(1)];
            unsafe {
                c_f(oc.as_mut_ptr(), size, seed.as_ptr());
                r_f(or.as_mut_ptr(), size, seed.as_ptr());
            }
            eq_bytes(
                &format!("randombytes_buf_deterministic size={size} seed={fill:#x}"),
                &oc[..size],
                &or[..size],
            );
        }
    }
}

// ===========================================================================
// Row 158 — implementation_name / random / uniform / buf / stir / close /
// set_implementation.  These are non-deterministic, so only check shape /
// no-crash / distribution sanity, EXCEPT uniform(0) and uniform(1) which are
// deterministic (both must return 0).
// ===========================================================================

#[test]
fn randombytes_runtime_api() {
    let d = duo();
    let (c_name, r_name) = d.pair::<RbName>("randombytes_implementation_name");
    let (c_rand, r_rand) = d.pair::<RbRandom>("randombytes_random");
    let (c_uni, r_uni) = d.pair::<RbUniform>("randombytes_uniform");
    let (c_buf, r_buf) = d.pair::<RbBuf>("randombytes_buf");
    let (c_stir, r_stir) = d.pair::<RbVoid>("randombytes_stir");
    let (c_close, r_close) = d.pair::<RbClose>("randombytes_close");

    // Implementation name string MUST match.
    let cn = unsafe { CStr::from_ptr(c_name()) };
    let rn = unsafe { CStr::from_ptr(r_name()) };
    assert_eq!(cn, rn, "randombytes_implementation_name");

    // uniform(0) and uniform(1) are deterministic -> both return 0.
    unsafe {
        eq_i32("uniform(0)", c_uni(0) as i32, r_uni(0) as i32);
        assert_eq!(c_uni(0), 0, "C uniform(0) must be 0");
        assert_eq!(r_uni(0), 0, "Rust uniform(0) must be 0");
        eq_i32("uniform(1)", c_uni(1) as i32, r_uni(1) as i32);
        assert_eq!(c_uni(1), 0, "C uniform(1) must be 0");
        assert_eq!(r_uni(1), 0, "Rust uniform(1) must be 0");
    }

    // uniform(bound) for the other bounds: only range sanity (< bound).
    for &bound in &[2u32, 3, 255, 256, 1u32 << 31] {
        for _ in 0..200 {
            let c = unsafe { c_uni(bound) };
            let r = unsafe { r_uni(bound) };
            assert!(c < bound, "C uniform({bound}) = {c} out of range");
            assert!(r < bound, "Rust uniform({bound}) = {r} out of range");
        }
    }

    // random(): no crash; collect a few and require they aren't all identical
    // (distribution sanity, tolerant of the astronomically unlikely).
    let mut cvals = Vec::new();
    let mut rvals = Vec::new();
    for _ in 0..64 {
        cvals.push(unsafe { c_rand() });
        rvals.push(unsafe { r_rand() });
    }
    assert!(
        cvals.iter().any(|&v| v != cvals[0]),
        "C random() produced 64 identical values"
    );
    assert!(
        rvals.iter().any(|&v| v != rvals[0]),
        "Rust random() produced 64 identical values"
    );

    // buf(): no crash for a range of sizes; not comparing bytes.
    for &size in &[0usize, 1, 16, 64, 1000] {
        let mut cb = vec![0u8; size.max(1)];
        let mut rb = vec![0u8; size.max(1)];
        unsafe {
            c_buf(cb.as_mut_ptr(), size);
            r_buf(rb.as_mut_ptr(), size);
        }
    }

    // stir(): no crash, no return.
    unsafe {
        c_stir();
        r_stir();
    }

    // close(): return-value shape must match (default impl returns 0 or -1).
    let (cc, _) = with_errno(|| unsafe { c_close() });
    let (rc, _) = with_errno(|| unsafe { r_close() });
    eq_i32("randombytes_close", cc, rc);
}

// ===========================================================================
// Row 159 — randombytes_seedbytes + randombytes constants
// ===========================================================================

#[test]
fn randombytes_seedbytes_constant() {
    let d = duo();
    let (c_sb, r_sb) = d.pair::<RbSeedBytes>("randombytes_seedbytes");
    let (c, r) = unsafe { (c_sb(), r_sb()) };
    assert_eq!(c, 32, "randombytes_SEEDBYTES must be 32, got {c}");
    assert_eq!(c, r, "randombytes_seedbytes C={c} Rust={r}");
}
