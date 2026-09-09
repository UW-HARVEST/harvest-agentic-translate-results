//! Phase B — valid-path differential tests for group G1:
//! `sodium/utils.c`, `sodium/codecs.c`, `sodium/core.c`, `sodium/runtime.c`,
//! `sodium/version.c`, `crypto_verify/verify.c`.
mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

const SEED: u64 = 0xC0FFEE_1234_5678;

const VARIANTS: [c_int; 4] = [1, 3, 5, 7]; // ORIGINAL, ORIGINAL_NO_PADDING, URLSAFE, URLSAFE_NO_PADDING

// ---------------------------------------------------------------- bin2hex ---
#[test]
fn g1_bin2hex_all_lengths() {
    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
    unsafe {
        let (c, r) = pair::<F>("sodium_bin2hex");
        let mut rng = Rng::new(SEED);
        for bin_len in 0..=64usize {
            for _ in 0..8 {
                let bin = rng.bytes(bin_len);
                let maxlen = bin_len * 2 + 1;
                let mut cb = vec![0xAAu8; maxlen + 8];
                let mut rb = vec![0xAAu8; maxlen + 8];
                let cp = c(cb.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len);
                let rp = r(rb.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len);
                assert_eq!(
                    cp as usize == cb.as_ptr() as usize,
                    rp as usize == rb.as_ptr() as usize,
                    "sodium_bin2hex len={bin_len}: returned-pointer identity differs"
                );
                eq_bytes(&format!("sodium_bin2hex len={bin_len}"), &cb, &rb);
            }
        }
    }
}

// ---------------------------------------------------------------- hex2bin ---
#[test]
fn g1_hex2bin_valid_and_ignore() {
    type F = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
    ) -> c_int;
    unsafe {
        let (c, r) = pair::<F>("sodium_hex2bin");
        let mut rng = Rng::new(SEED ^ 1);
        let ignores: [Option<&[u8]>; 4] = [None, Some(b": \0"), Some(b"\0"), Some(b"-:\0")];
        let hexdig = b"0123456789abcdefABCDEF";
        for nbytes in 0..=48usize {
            for (ii, ign) in ignores.iter().enumerate() {
                for _ in 0..6 {
                    // build a hex string, possibly sprinkled with ignore chars
                    let mut s: Vec<u8> = Vec::new();
                    for _ in 0..nbytes {
                        s.push(hexdig[rng.below(hexdig.len())]);
                        s.push(hexdig[rng.below(hexdig.len())]);
                        if ii == 1 && rng.below(3) == 0 {
                            s.push(b':');
                        }
                        if ii == 3 && rng.below(4) == 0 {
                            s.push(b'-');
                        }
                    }
                    s.push(0);
                    let hex_len = s.len() - 1;
                    for &maxlen in &[nbytes, nbytes + 1, nbytes.saturating_sub(1)] {
                        let mut cb = vec![0x55u8; maxlen + 8];
                        let mut rb = vec![0x55u8; maxlen + 8];
                        let mut cl = 0usize;
                        let mut rl = 0usize;
                        let mut ce: *const c_char = ptr::null();
                        let mut re: *const c_char = ptr::null();
                        let ip = ign.map_or(ptr::null(), |x| x.as_ptr() as *const c_char);
                        let cr = c(
                            cb.as_mut_ptr(),
                            maxlen,
                            s.as_ptr() as *const c_char,
                            hex_len,
                            ip,
                            &mut cl,
                            &mut ce,
                        );
                        let rr = r(
                            rb.as_mut_ptr(),
                            maxlen,
                            s.as_ptr() as *const c_char,
                            hex_len,
                            ip,
                            &mut rl,
                            &mut re,
                        );
                        let ctx = format!("sodium_hex2bin n={nbytes} ign={ii} maxlen={maxlen}");
                        eq_i32(&ctx, cr, rr);
                        assert_eq!(cl, rl, "{ctx}: bin_len differs");
                        let coff = if ce.is_null() {
                            usize::MAX
                        } else {
                            ce as usize - s.as_ptr() as usize
                        };
                        let roff = if re.is_null() {
                            usize::MAX
                        } else {
                            re as usize - s.as_ptr() as usize
                        };
                        assert_eq!(coff, roff, "{ctx}: hex_end offset differs");
                        eq_bytes(&ctx, &cb, &rb);
                    }
                }
            }
        }
    }
}

#[test]
fn g1_hex2bin_null_bin_len_and_null_hex_end() {
    type F = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
    ) -> c_int;
    unsafe {
        let (c, r) = pair::<F>("sodium_hex2bin");
        for s in [
            &b"00\0"[..],
            &b"deadbeef\0"[..],
            &b"dead:beef\0"[..],
            &b"\0"[..],
            &b"0\0"[..],
        ] {
            let hex_len = s.len() - 1;
            let mut cb = [0u8; 16];
            let mut rb = [0u8; 16];
            let cr = c(
                cb.as_mut_ptr(),
                16,
                s.as_ptr() as *const c_char,
                hex_len,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            let rr = r(
                rb.as_mut_ptr(),
                16,
                s.as_ptr() as *const c_char,
                hex_len,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            eq_i32("hex2bin null out params", cr, rr);
            eq_bytes("hex2bin null out params", &cb, &rb);
        }
    }
}

// ------------------------------------------------------- base64_encoded_len -
#[test]
fn g1_base64_encoded_len_all_variants() {
    type F = unsafe extern "C" fn(usize, c_int) -> usize;
    unsafe {
        let (c, r) = pair::<F>("sodium_base64_encoded_len");
        for v in VARIANTS {
            for n in 0..=200usize {
                assert_eq!(
                    c(n, v),
                    r(n, v),
                    "sodium_base64_encoded_len({n}, variant={v})"
                );
            }
            for n in [1000usize, 4096, 65535, 1 << 20] {
                assert_eq!(c(n, v), r(n, v), "sodium_base64_encoded_len({n}, {v})");
            }
        }
    }
}

// ------------------------------------------------------------- bin2base64 ---
#[test]
fn g1_bin2base64_all_variants_all_lengths() {
    type F = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
    type L = unsafe extern "C" fn(usize, c_int) -> usize;
    unsafe {
        let (c, r) = pair::<F>("sodium_bin2base64");
        let (cl, _rl) = pair::<L>("sodium_base64_encoded_len");
        let mut rng = Rng::new(SEED ^ 2);
        for v in VARIANTS {
            for bin_len in 0..=70usize {
                for _ in 0..6 {
                    let bin = rng.bytes(bin_len);
                    let maxlen = cl(bin_len, v);
                    let mut cb = vec![0xAAu8; maxlen + 8];
                    let mut rb = vec![0xAAu8; maxlen + 8];
                    let cp = c(
                        cb.as_mut_ptr() as *mut c_char,
                        maxlen,
                        bin.as_ptr(),
                        bin_len,
                        v,
                    );
                    let rp = r(
                        rb.as_mut_ptr() as *mut c_char,
                        maxlen,
                        bin.as_ptr(),
                        bin_len,
                        v,
                    );
                    assert_eq!(cp.is_null(), rp.is_null(), "bin2base64 nullness v={v}");
                    eq_bytes(&format!("sodium_bin2base64 v={v} len={bin_len}"), &cb, &rb);
                }
            }
        }
    }
}

// ------------------------------------------------------------- base642bin ---
#[test]
fn g1_base642bin_roundtrip_all_variants() {
    type E = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
    type L = unsafe extern "C" fn(usize, c_int) -> usize;
    type D = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
        c_int,
    ) -> c_int;
    unsafe {
        let (ce, _re) = pair::<E>("sodium_bin2base64");
        let (cl, _) = pair::<L>("sodium_base64_encoded_len");
        let (cd, rd) = pair::<D>("sodium_base642bin");
        let mut rng = Rng::new(SEED ^ 3);
        let ignores: [Option<&[u8]>; 3] = [None, Some(b" \n\0"), Some(b"\0")];
        for v in VARIANTS {
            for bin_len in 0..=70usize {
                for _ in 0..5 {
                    let bin = rng.bytes(bin_len);
                    let maxlen = cl(bin_len, v);
                    let mut b64 = vec![0u8; maxlen + 8];
                    let p = ce(
                        b64.as_mut_ptr() as *mut c_char,
                        maxlen,
                        bin.as_ptr(),
                        bin_len,
                        v,
                    );
                    assert!(!p.is_null());
                    let b64_len = b64.iter().position(|&x| x == 0).unwrap();
                    // optionally sprinkle ignore chars
                    for (ii, ign) in ignores.iter().enumerate() {
                        let mut s: Vec<u8> = Vec::new();
                        for &ch in &b64[..b64_len] {
                            if ii == 1 && rng.below(4) == 0 {
                                s.push(b' ');
                            }
                            s.push(ch);
                        }
                        s.push(0);
                        let slen = s.len() - 1;
                        for &outmax in &[bin_len, bin_len + 3] {
                            let mut cb = vec![0x55u8; outmax + 8];
                            let mut rb = vec![0x55u8; outmax + 8];
                            let mut cbl = 0usize;
                            let mut rbl = 0usize;
                            let mut cend: *const c_char = ptr::null();
                            let mut rend: *const c_char = ptr::null();
                            let ip = ign.map_or(ptr::null(), |x| x.as_ptr() as *const c_char);
                            let cr = cd(
                                cb.as_mut_ptr(),
                                outmax,
                                s.as_ptr() as *const c_char,
                                slen,
                                ip,
                                &mut cbl,
                                &mut cend,
                                v,
                            );
                            let rr = rd(
                                rb.as_mut_ptr(),
                                outmax,
                                s.as_ptr() as *const c_char,
                                slen,
                                ip,
                                &mut rbl,
                                &mut rend,
                                v,
                            );
                            let ctx = format!(
                                "sodium_base642bin v={v} bin_len={bin_len} ign={ii} outmax={outmax}"
                            );
                            eq_i32(&ctx, cr, rr);
                            assert_eq!(cbl, rbl, "{ctx}: bin_len differs");
                            let co = if cend.is_null() {
                                usize::MAX
                            } else {
                                cend as usize - s.as_ptr() as usize
                            };
                            let ro = if rend.is_null() {
                                usize::MAX
                            } else {
                                rend as usize - s.as_ptr() as usize
                            };
                            assert_eq!(co, ro, "{ctx}: b64_end offset differs");
                            eq_bytes(&ctx, &cb, &rb);
                            if cr == 0 {
                                eq_bytes(&format!("{ctx} roundtrip"), &bin, &cb[..cbl]);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn g1_base642bin_random_strings() {
    // Random (mostly invalid) base64-ish strings across all four variants:
    // exercises every rejection/acceptance branch of the decoder.
    type D = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
        c_int,
    ) -> c_int;
    unsafe {
        let (cd, rd) = pair::<D>("sodium_base642bin");
        let mut rng = Rng::new(SEED ^ 4);
        let alphabet: Vec<u8> =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/-_= \n\t.,!".to_vec();
        for v in VARIANTS {
            for _ in 0..4000 {
                let n = rng.below(24);
                let mut s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
                s.push(0);
                let outmax = rng.below(20);
                let mut cb = vec![0x55u8; outmax + 8];
                let mut rb = vec![0x55u8; outmax + 8];
                let mut cbl = usize::MAX;
                let mut rbl = usize::MAX;
                let mut cend: *const c_char = ptr::null();
                let mut rend: *const c_char = ptr::null();
                let ip: *const c_char = if rng.below(2) == 0 {
                    b" \n\0".as_ptr() as *const c_char
                } else {
                    ptr::null()
                };
                let cr = cd(
                    cb.as_mut_ptr(),
                    outmax,
                    s.as_ptr() as *const c_char,
                    n,
                    ip,
                    &mut cbl,
                    &mut cend,
                    v,
                );
                let rr = rd(
                    rb.as_mut_ptr(),
                    outmax,
                    s.as_ptr() as *const c_char,
                    n,
                    ip,
                    &mut rbl,
                    &mut rend,
                    v,
                );
                let ctx = format!(
                    "sodium_base642bin fuzz v={v} s={:?} outmax={outmax}",
                    String::from_utf8_lossy(&s[..n])
                );
                eq_i32(&ctx, cr, rr);
                assert_eq!(cbl, rbl, "{ctx}: bin_len differs");
                let co = if cend.is_null() {
                    usize::MAX
                } else {
                    cend as usize - s.as_ptr() as usize
                };
                let ro = if rend.is_null() {
                    usize::MAX
                } else {
                    rend as usize - s.as_ptr() as usize
                };
                assert_eq!(co, ro, "{ctx}: b64_end offset differs");
                eq_bytes(&ctx, &cb[..cbl.min(cb.len())], &rb[..rbl.min(rb.len())]);
            }
        }
    }
}

#[test]
fn g1_hex2bin_random_strings() {
    type F = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
    ) -> c_int;
    unsafe {
        let (c, r) = pair::<F>("sodium_hex2bin");
        let mut rng = Rng::new(SEED ^ 5);
        let alphabet: Vec<u8> = b"0123456789abcdefABCDEFghxyz:-. \n".to_vec();
        for _ in 0..8000 {
            let n = rng.below(20);
            let mut s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
            s.push(0);
            let outmax = rng.below(12);
            let mut cb = vec![0x55u8; outmax + 8];
            let mut rb = vec![0x55u8; outmax + 8];
            let mut cbl = usize::MAX;
            let mut rbl = usize::MAX;
            let mut cend: *const c_char = ptr::null();
            let mut rend: *const c_char = ptr::null();
            let ip: *const c_char = match rng.below(3) {
                0 => ptr::null(),
                1 => b":\0".as_ptr() as *const c_char,
                _ => b" \n:-\0".as_ptr() as *const c_char,
            };
            let cr = c(
                cb.as_mut_ptr(),
                outmax,
                s.as_ptr() as *const c_char,
                n,
                ip,
                &mut cbl,
                &mut cend,
            );
            let rr = r(
                rb.as_mut_ptr(),
                outmax,
                s.as_ptr() as *const c_char,
                n,
                ip,
                &mut rbl,
                &mut rend,
            );
            let ctx = format!(
                "sodium_hex2bin fuzz s={:?} outmax={outmax}",
                String::from_utf8_lossy(&s[..n])
            );
            eq_i32(&ctx, cr, rr);
            assert_eq!(cbl, rbl, "{ctx}: bin_len differs");
            let co = if cend.is_null() {
                usize::MAX
            } else {
                cend as usize - s.as_ptr() as usize
            };
            let ro = if rend.is_null() {
                usize::MAX
            } else {
                rend as usize - s.as_ptr() as usize
            };
            assert_eq!(co, ro, "{ctx}: hex_end offset differs");
            eq_bytes(&ctx, &cb, &rb);
        }
    }
}

// ------------------------------------------------------- compare / is_zero --
#[test]
fn g1_compare_is_zero_increment_add_sub() {
    unsafe {
        let (ccmp, rcmp) =
            pair::<unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int>("sodium_compare");
        let (cmc, rmc) = pair::<unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int>(
            "sodium_memcmp",
        );
        let (ciz, riz) =
            pair::<unsafe extern "C" fn(*const u8, usize) -> c_int>("sodium_is_zero");
        let (cinc, rinc) = pair::<unsafe extern "C" fn(*mut u8, usize)>("sodium_increment");
        let (cadd, radd) = pair::<unsafe extern "C" fn(*mut u8, *const u8, usize)>("sodium_add");
        let (csub, rsub) = pair::<unsafe extern "C" fn(*mut u8, *const u8, usize)>("sodium_sub");
        let mut rng = Rng::new(SEED ^ 6);
        for len in 0..=40usize {
            // hand-picked carry-boundary shapes plus random ones
            let mut shapes: Vec<(Vec<u8>, Vec<u8>)> = vec![
                (vec![0u8; len], vec![0u8; len]),
                (vec![0xffu8; len], vec![0u8; len]),
                (vec![0u8; len], vec![0xffu8; len]),
                (vec![0xffu8; len], vec![0xffu8; len]),
                (vec![0xffu8; len], vec![1u8; len]),
            ];
            if len > 0 {
                let mut a = vec![0u8; len];
                a[len - 1] = 1;
                shapes.push((a.clone(), vec![0u8; len]));
                let mut b = vec![0u8; len];
                b[0] = 1;
                shapes.push((vec![0u8; len], b.clone()));
                shapes.push((a, b));
            }
            for _ in 0..12 {
                shapes.push((rng.bytes(len), rng.bytes(len)));
            }
            // one-bit-difference-at-each-position shapes
            for i in 0..len {
                let mut a = vec![0x80u8; len];
                let mut b = a.clone();
                b[i] ^= 1;
                shapes.push((a.clone(), b.clone()));
                a[i] = 0xff;
                shapes.push((a, b));
            }
            for (a, b) in shapes {
                let ap = if len == 0 { ptr::null() } else { a.as_ptr() };
                let bp = if len == 0 { ptr::null() } else { b.as_ptr() };
                let ctx = format!("len={len} a={} b={}", hex(&a), hex(&b));
                eq_i32(
                    &format!("sodium_compare {ctx}"),
                    ccmp(ap, bp, len),
                    rcmp(ap, bp, len),
                );
                eq_i32(
                    &format!("sodium_memcmp {ctx}"),
                    cmc(ap as *const c_void, bp as *const c_void, len),
                    rmc(ap as *const c_void, bp as *const c_void, len),
                );
                eq_i32(
                    &format!("sodium_is_zero {ctx}"),
                    ciz(ap, len),
                    riz(ap, len),
                );
                let mut x = a.clone();
                let mut y = a.clone();
                cinc(if len == 0 { ptr::null_mut() } else { x.as_mut_ptr() }, len);
                rinc(if len == 0 { ptr::null_mut() } else { y.as_mut_ptr() }, len);
                eq_bytes(&format!("sodium_increment {ctx}"), &x, &y);
                let mut x = a.clone();
                let mut y = a.clone();
                if len > 0 {
                    cadd(x.as_mut_ptr(), bp, len);
                    radd(y.as_mut_ptr(), bp, len);
                }
                eq_bytes(&format!("sodium_add {ctx}"), &x, &y);
                let mut x = a.clone();
                let mut y = a.clone();
                if len > 0 {
                    csub(x.as_mut_ptr(), bp, len);
                    rsub(y.as_mut_ptr(), bp, len);
                }
                eq_bytes(&format!("sodium_sub {ctx}"), &x, &y);
            }
        }
    }
}

#[test]
fn g1_memzero_and_stackzero() {
    unsafe {
        let (c, r) = pair::<unsafe extern "C" fn(*mut c_void, usize)>("sodium_memzero");
        let mut rng = Rng::new(SEED ^ 7);
        for len in 0..=64usize {
            let mut a = rng.bytes(64);
            let mut b = a.clone();
            c(a.as_mut_ptr() as *mut c_void, len);
            r(b.as_mut_ptr() as *mut c_void, len);
            eq_bytes(&format!("sodium_memzero len={len}"), &a, &b);
        }
        // NULL with len 0 must be accepted by both
        c(ptr::null_mut(), 0);
        r(ptr::null_mut(), 0);
        let (c, r) = pair::<unsafe extern "C" fn(usize)>("sodium_stackzero");
        for len in [0usize, 1, 64, 512] {
            c(len);
            r(len);
        }
    }
}

// -------------------------------------------------------------- pad/unpad ---
#[test]
fn g1_pad_unpad_matrix() {
    type P =
        unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
    type U = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
    unsafe {
        let (cp, rp) = pair::<P>("sodium_pad");
        let (cu, ru) = pair::<U>("sodium_unpad");
        let mut rng = Rng::new(SEED ^ 8);
        for blocksize in [1usize, 2, 3, 4, 8, 15, 16, 17, 32, 64, 255, 256] {
            for unpadded in 0..=80usize {
                let data = rng.bytes(unpadded);
                for extra in [0usize, 1, blocksize, 2 * blocksize, 200] {
                    let max_buflen = unpadded + extra;
                    let mut cb = vec![0x77u8; max_buflen.max(1) + 16];
                    let mut rb = cb.clone();
                    cb[..unpadded].copy_from_slice(&data);
                    rb[..unpadded].copy_from_slice(&data);
                    let mut cl = usize::MAX;
                    let mut rl = usize::MAX;
                    let cr = cp(&mut cl, cb.as_mut_ptr(), unpadded, blocksize, max_buflen);
                    let rr = rp(&mut rl, rb.as_mut_ptr(), unpadded, blocksize, max_buflen);
                    let ctx = format!(
                        "sodium_pad unpadded={unpadded} bs={blocksize} max={max_buflen}"
                    );
                    eq_i32(&ctx, cr, rr);
                    assert_eq!(cl, rl, "{ctx}: padded_buflen differs");
                    eq_bytes(&ctx, &cb, &rb);
                    if cr == 0 {
                        let mut cul = usize::MAX;
                        let mut rul = usize::MAX;
                        let cur = cu(&mut cul, cb.as_ptr(), cl, blocksize);
                        let rur = ru(&mut rul, rb.as_ptr(), rl, blocksize);
                        eq_i32(&format!("sodium_unpad {ctx}"), cur, rur);
                        assert_eq!(cul, rul, "sodium_unpad {ctx}: unpadded_buflen differs");
                        if cur == 0 {
                            assert_eq!(cul, unpadded, "sodium_unpad {ctx}: roundtrip length");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn g1_unpad_arbitrary_buffers() {
    type U = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
    unsafe {
        let (cu, ru) = pair::<U>("sodium_unpad");
        let mut rng = Rng::new(SEED ^ 9);
        for _ in 0..20000 {
            let blocksize = rng.range(1, 20);
            let padded = rng.below(60);
            let mut buf = rng.bytes(padded);
            // bias towards buffers that look like valid padding
            if padded > 0 && rng.below(2) == 0 {
                let k = rng.below(padded);
                for x in buf[k..].iter_mut() {
                    *x = 0;
                }
                buf[k] = 0x80;
            }
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            let bp = if padded == 0 { ptr::null() } else { buf.as_ptr() };
            let cr = cu(&mut cl, bp, padded, blocksize);
            let rr = ru(&mut rl, bp, padded, blocksize);
            let ctx = format!("sodium_unpad fuzz bs={blocksize} buf={}", hex(&buf));
            eq_i32(&ctx, cr, rr);
            assert_eq!(cl, rl, "{ctx}: unpadded_buflen differs");
        }
    }
}

// --------------------------------------------------------------- ip2bin -----
#[test]
fn g1_ip2bin_bin2ip() {
    type I = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
    type B = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
    unsafe {
        let (ci, ri) = pair::<I>("sodium_ip2bin");
        let (cb2, rb2) = pair::<B>("sodium_bin2ip");
        let cases: &[&[u8]] = &[
            b"0.0.0.0\0",
            b"127.0.0.1\0",
            b"255.255.255.255\0",
            b"1.2.3.4\0",
            b"01.2.3.4\0",
            b"1.2.3\0",
            b"1.2.3.4.5\0",
            b"256.1.1.1\0",
            b"::\0",
            b"::1\0",
            b"1::\0",
            b"::ffff:127.0.0.1\0",
            b"2001:db8::1\0",
            b"2001:0db8:0000:0000:0000:0000:0000:0001\0",
            b"fe80::1%eth0\0",
            b"1:2:3:4:5:6:7:8\0",
            b"1:2:3:4:5:6:7:8:9\0",
            b"1:2:3:4:5:6:7\0",
            b"1::2::3\0",
            b"gggg::1\0",
            b"\0",
            b" 1.2.3.4\0",
            b"1.2.3.4 \0",
            b"[::1]\0",
            b"::ffff:1.2.3.4.5\0",
            b"12345::1\0",
            b"1:2:3:4:5:6:1.2.3.4\0",
            b"::1.2.3.4\0",
        ];
        for s in cases {
            let slen = s.len() - 1;
            for &pass_len in &[slen, slen + 1, 0] {
                let mut ca = [0xEEu8; 16];
                let mut ra = [0xEEu8; 16];
                let cr = ci(ca.as_mut_ptr(), s.as_ptr() as *const c_char, pass_len);
                let rr = ri(ra.as_mut_ptr(), s.as_ptr() as *const c_char, pass_len);
                let ctx = format!(
                    "sodium_ip2bin {:?} len={pass_len}",
                    String::from_utf8_lossy(&s[..slen])
                );
                eq_i32(&ctx, cr, rr);
                eq_bytes(&ctx, &ca, &ra);
            }
        }
        // bin2ip over random 16-byte values and every output buffer size
        let mut rng = Rng::new(SEED ^ 10);
        for _ in 0..4000 {
            let mut bin = rng.bytes(16);
            match rng.below(4) {
                0 => {
                    // IPv4-mapped
                    for i in 0..10 {
                        bin[i] = 0;
                    }
                    bin[10] = 0xff;
                    bin[11] = 0xff;
                }
                1 => {
                    for x in bin.iter_mut() {
                        *x = 0;
                    }
                }
                2 => {
                    for i in 0..15 {
                        bin[i] = 0;
                    }
                    bin[15] = 1;
                }
                _ => {}
            }
            for maxlen in [0usize, 1, 2, 4, 8, 15, 16, 39, 40, 46, 64] {
                let mut co = vec![0x33u8; maxlen + 8];
                let mut ro = vec![0x33u8; maxlen + 8];
                let cp = cb2(co.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let rp = rb2(ro.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let ctx = format!("sodium_bin2ip bin={} maxlen={maxlen}", hex(&bin));
                assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: nullness differs");
                eq_bytes(&ctx, &co, &ro);
            }
            // roundtrip: bin -> text -> bin
            let mut txt = vec![0u8; 64];
            let p = cb2(txt.as_mut_ptr() as *mut c_char, 64, bin.as_ptr());
            if !p.is_null() {
                let tlen = txt.iter().position(|&x| x == 0).unwrap();
                let mut ca = [0u8; 16];
                let mut ra = [0u8; 16];
                let cr = ci(ca.as_mut_ptr(), txt.as_ptr() as *const c_char, tlen);
                let rr = ri(ra.as_mut_ptr(), txt.as_ptr() as *const c_char, tlen);
                let ctx = format!("ip roundtrip {}", String::from_utf8_lossy(&txt[..tlen]));
                eq_i32(&ctx, cr, rr);
                eq_bytes(&ctx, &ca, &ra);
            }
        }
    }
}

// ------------------------------------------------------------ crypto_verify -
#[test]
fn g1_crypto_verify_16_32_64() {
    unsafe {
        for (name, n) in [
            ("crypto_verify_16", 16usize),
            ("crypto_verify_32", 32),
            ("crypto_verify_64", 64),
        ] {
            let (c, r) =
                pair::<unsafe extern "C" fn(*const u8, *const u8) -> c_int>(name);
            let (cb, rb) = pair::<unsafe extern "C" fn() -> usize>(&format!("{name}_bytes"));
            assert_eq!(cb(), rb(), "{name}_bytes");
            assert_eq!(cb(), n, "{name}_bytes value");
            let mut rng = Rng::new(SEED ^ n as u64);
            for _ in 0..200 {
                let a = rng.bytes(n);
                // identical
                eq_i32(
                    &format!("{name} equal"),
                    c(a.as_ptr(), a.as_ptr()),
                    r(a.as_ptr(), a.as_ptr()),
                );
                let b = a.clone();
                eq_i32(
                    &format!("{name} equal copy"),
                    c(a.as_ptr(), b.as_ptr()),
                    r(a.as_ptr(), b.as_ptr()),
                );
                // differ at every byte position, every bit
                for i in 0..n {
                    for bit in [0u8, 1, 7] {
                        let mut b = a.clone();
                        b[i] ^= 1 << bit;
                        eq_i32(
                            &format!("{name} diff at {i} bit {bit}"),
                            c(a.as_ptr(), b.as_ptr()),
                            r(a.as_ptr(), b.as_ptr()),
                        );
                    }
                }
                let b = rng.bytes(n);
                eq_i32(
                    &format!("{name} random"),
                    c(a.as_ptr(), b.as_ptr()),
                    r(a.as_ptr(), b.as_ptr()),
                );
            }
        }
    }
}

// ------------------------------------------------------------- runtime -----
#[test]
fn g1_runtime_has_all() {
    unsafe {
        for n in [
            "sodium_runtime_has_neon",
            "sodium_runtime_has_armcrypto",
            "sodium_runtime_has_sse2",
            "sodium_runtime_has_sse3",
            "sodium_runtime_has_ssse3",
            "sodium_runtime_has_sse41",
            "sodium_runtime_has_avx",
            "sodium_runtime_has_avx2",
            "sodium_runtime_has_avx512f",
            "sodium_runtime_has_pclmul",
            "sodium_runtime_has_aesni",
            "sodium_runtime_has_rdrand",
        ] {
            let (c, r) = pair::<unsafe extern "C" fn() -> c_int>(n);
            eq_i32(n, c(), r());
        }
    }
}

#[test]
fn g1_sodium_init_idempotent() {
    unsafe {
        let (c, r) = pair::<unsafe extern "C" fn() -> c_int>("sodium_init");
        // already initialised by the harness => must return 1 on both
        for _ in 0..3 {
            eq_i32("sodium_init (repeat)", c(), r());
        }
    }
}

// -------------------------------------------------------- secure allocator --
#[test]
fn g1_sodium_malloc_family() {
    // Differential: every return code is recorded per library and compared, so
    // build-config-dependent behaviour (this build has no HAVE_PAGE_PROTECTION,
    // hence mprotect_* returns -1/ENOSYS) must agree between C and Rust rather
    // than matching a hard-coded expectation.
    let mut results: Vec<Vec<(String, i32)>> = Vec::new();
    unsafe {
        for lib in ["c", "rust"] {
            let mut rec: Vec<(String, i32)> = Vec::new();
            let l = libs();
            let h = if lib == "c" { l.c } else { l.rs };
            let malloc: libloading::Symbol<unsafe extern "C" fn(usize) -> *mut c_void> =
                h.get(b"sodium_malloc\0").unwrap();
            let allocarray: libloading::Symbol<
                unsafe extern "C" fn(usize, usize) -> *mut c_void,
            > = h.get(b"sodium_allocarray\0").unwrap();
            let free: libloading::Symbol<unsafe extern "C" fn(*mut c_void)> =
                h.get(b"sodium_free\0").unwrap();
            let noaccess: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> =
                h.get(b"sodium_mprotect_noaccess\0").unwrap();
            let readonly: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> =
                h.get(b"sodium_mprotect_readonly\0").unwrap();
            let readwrite: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> =
                h.get(b"sodium_mprotect_readwrite\0").unwrap();
            let mlock: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize) -> c_int> =
                h.get(b"sodium_mlock\0").unwrap();
            let munlock: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize) -> c_int> =
                h.get(b"sodium_munlock\0").unwrap();

            for size in [0usize, 1, 7, 8, 16, 4095, 4096, 4097, 100_000] {
                let p = malloc(size);
                rec.push((format!("malloc({size}).is_null"), p.is_null() as i32));
                assert!(!p.is_null(), "{lib}: sodium_malloc({size}) returned NULL");
                if size > 0 {
                    std::ptr::write_bytes(p as *mut u8, 0x5A, size);
                    assert_eq!(*(p as *const u8), 0x5A);
                }
                rec.push((format!("mprotect_readonly({size})"), readonly(p)));
                rec.push((format!("mprotect_noaccess({size})"), noaccess(p)));
                rec.push((format!("mprotect_readwrite({size})"), readwrite(p)));
                if size > 0 {
                    std::ptr::write_bytes(p as *mut u8, 0xA5, size);
                }
                free(p);
            }
            for (cnt, sz) in [(0usize, 0usize), (1, 1), (10, 10), (4, 1024), (1, 65536)] {
                let p = allocarray(cnt, sz);
                rec.push((format!("allocarray({cnt},{sz}).is_null"), p.is_null() as i32));
                assert!(!p.is_null(), "{lib}: sodium_allocarray({cnt},{sz}) NULL");
                let n = cnt * sz;
                if n > 0 {
                    std::ptr::write_bytes(p as *mut u8, 0x11, n);
                }
                free(p);
            }
            // free(NULL) must be a no-op
            free(ptr::null_mut());
            // mprotect_* on a NULL pointer
            rec.push(("mprotect_readonly(NULL)".into(), readonly(ptr::null_mut())));
            // mlock/munlock on a plain buffer
            let mut buf = vec![0u8; 4096];
            let mr = mlock(buf.as_mut_ptr() as *mut c_void, buf.len());
            rec.push(("mlock(4096)".into(), mr));
            let ur = munlock(buf.as_mut_ptr() as *mut c_void, buf.len());
            rec.push(("munlock(4096)".into(), ur));
            rec.push(("mlock(0)".into(), mlock(buf.as_mut_ptr() as *mut c_void, 0)));
            rec.push((
                "munlock(0)".into(),
                munlock(buf.as_mut_ptr() as *mut c_void, 0),
            ));
            results.push(rec);
        }
    }
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0], results[1],
        "sodium_malloc/mprotect/mlock family: C and Rust return codes differ"
    );
}

// -------------------------------------------------- randombytes_deterministic
#[test]
fn g1_randombytes_buf_deterministic() {
    unsafe {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut c_void, usize, *const u8)>(
                "randombytes_buf_deterministic",
            );
        let (cs, rs) = pair::<unsafe extern "C" fn() -> usize>("randombytes_seedbytes");
        assert_eq!(cs(), rs());
        assert_eq!(cs(), 32);
        let mut rng = Rng::new(SEED ^ 11);
        for size in [
            0usize, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000, 4096, 5000,
        ] {
            for _ in 0..4 {
                let seed = rng.bytes(32);
                let mut cb = vec![0u8; size + 8];
                let mut rb = vec![0u8; size + 8];
                c(cb.as_mut_ptr() as *mut c_void, size, seed.as_ptr());
                r(rb.as_mut_ptr() as *mut c_void, size, seed.as_ptr());
                eq_bytes(
                    &format!("randombytes_buf_deterministic size={size} seed={}", hex(&seed)),
                    &cb,
                    &rb,
                );
            }
        }
        // all-zero and all-ff seeds
        for seed in [[0u8; 32], [0xffu8; 32]] {
            let mut cb = [0u8; 200];
            let mut rb = [0u8; 200];
            c(cb.as_mut_ptr() as *mut c_void, 200, seed.as_ptr());
            r(rb.as_mut_ptr() as *mut c_void, 200, seed.as_ptr());
            eq_bytes("randombytes_buf_deterministic edge seed", &cb, &rb);
        }
    }
}
