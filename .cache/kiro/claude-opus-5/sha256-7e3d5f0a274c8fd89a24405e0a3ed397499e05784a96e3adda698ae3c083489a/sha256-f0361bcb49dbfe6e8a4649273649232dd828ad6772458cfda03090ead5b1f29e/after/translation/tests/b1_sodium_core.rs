//! Phase B — valid-path differential tests, CONFIGS.md group 1
//! (sodium/ utils, codecs, randombytes, verify, shorthash, onetimeauth, ipcrypt)

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

type Sz = usize;

// ------------------------------------------------------------------ rows 17-22

#[test]
fn row17_sodium_memcmp() {
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8, Sz) -> c_int>("sodium_memcmp");
    let mut rng = Rng::seeded();
    for &len in &[0usize, 1, 2, 15, 16, 17, 31, 32, 63, 64, 100] {
        for _ in 0..40 {
            let a = rng.bytes(len);
            let mut b = a.clone();
            if len > 0 && rng.next_u8() & 1 == 0 {
                let i = rng.below(len);
                b[i] ^= 1 << (rng.below(8));
            }
            unsafe {
                same_ret(
                    &format!("sodium_memcmp len={len}"),
                    c(a.as_ptr(), b.as_ptr(), len),
                    r(a.as_ptr(), b.as_ptr(), len),
                );
            }
        }
    }
}

#[test]
fn row18_sodium_compare() {
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8, Sz) -> c_int>("sodium_compare");
    let mut rng = Rng::seeded();
    for &len in &[0usize, 1, 2, 3, 8, 16, 32, 64] {
        for _ in 0..60 {
            let a = rng.bytes(len);
            let mut b = if rng.next_u8() & 3 == 0 { a.clone() } else { rng.bytes(len) };
            if len > 0 && rng.next_u8() & 3 == 1 {
                // near-equal: differ only in the most significant (last) byte
                b = a.clone();
                b[len - 1] = b[len - 1].wrapping_add(1);
            }
            unsafe {
                same_ret(
                    &format!("sodium_compare len={len}"),
                    c(a.as_ptr(), b.as_ptr(), len),
                    r(a.as_ptr(), b.as_ptr(), len),
                );
            }
        }
    }
}

#[test]
fn row19_sodium_is_zero() {
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, Sz) -> c_int>("sodium_is_zero");
    let mut rng = Rng::seeded();
    for &len in &[0usize, 1, 2, 15, 16, 17, 32, 64] {
        // all zero
        let z = vec![0u8; len];
        unsafe { same_ret("is_zero all0", c(z.as_ptr(), len), r(z.as_ptr(), len)) };
        // one nonzero at each index
        for i in 0..len {
            let mut v = vec![0u8; len];
            v[i] = 1;
            unsafe { same_ret("is_zero one", c(v.as_ptr(), len), r(v.as_ptr(), len)) };
        }
        for _ in 0..20 {
            let v = rng.bytes(len);
            unsafe { same_ret("is_zero rnd", c(v.as_ptr(), len), r(v.as_ptr(), len)) };
        }
    }
}

#[test]
fn row20_sodium_increment() {
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, Sz)>("sodium_increment");
    let mut rng = Rng::seeded();
    for &len in &[0usize, 1, 2, 3, 7, 8, 9, 12, 16, 24, 32, 33, 64] {
        // exhaustive-ish special values
        let mut cases: Vec<Vec<u8>> = vec![
            vec![0u8; len],
            vec![0xff; len],
        ];
        if len > 0 {
            let mut v = vec![0xff; len];
            v[len - 1] = 0;
            cases.push(v);
            let mut v = vec![0u8; len];
            v[0] = 0xff;
            cases.push(v);
        }
        for _ in 0..40 {
            cases.push(rng.bytes(len));
        }
        for base in cases {
            let mut a = base.clone();
            let mut b = base.clone();
            unsafe {
                c(a.as_mut_ptr(), len);
                r(b.as_mut_ptr(), len);
            }
            same_bytes(&format!("sodium_increment len={len}"), &a, &b);
        }
    }
}

#[test]
fn row21_sodium_add() {
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz)>("sodium_add");
    let mut rng = Rng::seeded();
    for &len in &[0usize, 1, 2, 3, 7, 8, 9, 12, 16, 24, 32, 33, 64] {
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![0xff; len], vec![0x01; len]),
            (vec![0xff; len], vec![0xff; len]),
            (vec![0u8; len], vec![0u8; len]),
        ];
        for _ in 0..40 {
            cases.push((rng.bytes(len), rng.bytes(len)));
        }
        for (base, addend) in cases {
            let mut a = base.clone();
            let mut b = base.clone();
            unsafe {
                c(a.as_mut_ptr(), addend.as_ptr(), len);
                r(b.as_mut_ptr(), addend.as_ptr(), len);
            }
            same_bytes(&format!("sodium_add len={len}"), &a, &b);
        }
    }
}

#[test]
fn row22_sodium_sub() {
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz)>("sodium_sub");
    let mut rng = Rng::seeded();
    for &len in &[0usize, 1, 2, 3, 8, 16, 24, 32, 33, 64] {
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (vec![0u8; len], vec![0x01; len]),
            (vec![0u8; len], vec![0xff; len]),
            (vec![0xff; len], vec![0xff; len]),
        ];
        for _ in 0..40 {
            cases.push((rng.bytes(len), rng.bytes(len)));
        }
        for (base, sub) in cases {
            let mut a = base.clone();
            let mut b = base.clone();
            unsafe {
                c(a.as_mut_ptr(), sub.as_ptr(), len);
                r(b.as_mut_ptr(), sub.as_ptr(), len);
            }
            same_bytes(&format!("sodium_sub len={len}"), &a, &b);
        }
    }
}

// ------------------------------------------------------------------- rows 1-5

#[test]
fn row1_bin2hex() {
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz) -> *mut c_char,
    >("sodium_bin2hex");
    let mut rng = Rng::seeded();
    for len in 0usize..=64 {
        for _ in 0..8 {
            let bin = rng.bytes(len);
            let maxlen = len * 2 + 1;
            let mut ob = buf(maxlen + 8);
            let mut or = buf(maxlen + 8);
            unsafe {
                let pc = c(ob.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len);
                let pr = r(or.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len);
                assert_eq!(
                    pc as usize - ob.as_ptr() as usize,
                    pr as usize - or.as_ptr() as usize,
                    "bin2hex return offset"
                );
            }
            same_bytes(&format!("bin2hex len={len}"), &ob, &or);
        }
    }
}

#[test]
fn rows2_5_hex2bin() {
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
    let mut rng = Rng::seeded();

    let ignores: [Option<&[u8]>; 3] = [None, Some(b": \0"), Some(b"\r\n \0")];

    for len in 0usize..=48 {
        for _ in 0..6 {
            let bin = rng.bytes(len);
            // build hex, optionally with separators and trailing junk
            let mut hex_s = String::new();
            let sep = rng.next_u8() % 3;
            for (i, b) in bin.iter().enumerate() {
                if sep == 1 && i > 0 {
                    hex_s.push(':');
                } else if sep == 2 && i > 0 {
                    hex_s.push(' ');
                }
                hex_s.push_str(&format!("{b:02x}"));
            }
            let trailing = rng.next_u8() % 3;
            if trailing == 1 {
                hex_s.push('z');
            } else if trailing == 2 {
                hex_s.push_str("ZZ!");
            }
            let hex_b = hex_s.as_bytes();

            for ig in ignores.iter() {
                for &use_end in &[false, true] {
                    for &use_binlen in &[false, true] {
                        for &maxl in &[len, len + 4] {
                            let mut ob = buf(maxl.max(1) + 8);
                            let mut or = buf(maxl.max(1) + 8);
                            let mut bc: Sz = 0xDEAD;
                            let mut br: Sz = 0xDEAD;
                            let mut ec: *const c_char = std::ptr::null();
                            let mut er: *const c_char = std::ptr::null();
                            let igp = match ig {
                                None => std::ptr::null(),
                                Some(s) => s.as_ptr() as *const c_char,
                            };
                            unsafe {
                                let rc = c(
                                    ob.as_mut_ptr(),
                                    maxl,
                                    hex_b.as_ptr() as *const c_char,
                                    hex_b.len(),
                                    igp,
                                    if use_binlen { &mut bc } else { std::ptr::null_mut() },
                                    if use_end { &mut ec } else { std::ptr::null_mut() },
                                );
                                let rr = r(
                                    or.as_mut_ptr(),
                                    maxl,
                                    hex_b.as_ptr() as *const c_char,
                                    hex_b.len(),
                                    igp,
                                    if use_binlen { &mut br } else { std::ptr::null_mut() },
                                    if use_end { &mut er } else { std::ptr::null_mut() },
                                );
                                let ctx = format!(
                                    "hex2bin hex={hex_s:?} maxl={maxl} ig={} end={use_end} bl={use_binlen}",
                                    ig.is_some()
                                );
                                same_ret(&ctx, rc, rr);
                                same_bytes(&ctx, &ob, &or);
                                assert_eq!(bc, br, "{ctx}: bin_len differs");
                                if use_end {
                                    let oc = ec as usize - hex_b.as_ptr() as usize;
                                    let or_ = er as usize - hex_b.as_ptr() as usize;
                                    assert_eq!(oc, or_, "{ctx}: hex_end differs");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ rows 6-16

const B64_VARIANTS: [c_int; 4] = [1, 3, 5, 7];

#[test]
fn row16_base64_encoded_len() {
    let (c, r) = pair::<unsafe extern "C" fn(Sz, c_int) -> Sz>("sodium_base64_encoded_len");
    for v in B64_VARIANTS {
        for len in 0usize..=200 {
            unsafe {
                assert_eq!(c(len, v), r(len, v), "encoded_len len={len} variant={v}");
            }
        }
    }
}

#[test]
fn rows6_9_bin2base64() {
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz, c_int) -> *mut c_char,
    >("sodium_bin2base64");
    let (elc, _) = pair::<unsafe extern "C" fn(Sz, c_int) -> Sz>("sodium_base64_encoded_len");
    let mut rng = Rng::seeded();
    for v in B64_VARIANTS {
        for len in 0usize..=90 {
            for _ in 0..6 {
                let bin = rng.bytes(len);
                let maxlen = unsafe { elc(len, v) };
                let mut ob = buf(maxlen + 8);
                let mut or = buf(maxlen + 8);
                unsafe {
                    let pc = c(ob.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len, v);
                    let pr = r(or.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len, v);
                    assert_eq!(
                        pc as usize - ob.as_ptr() as usize,
                        pr as usize - or.as_ptr() as usize
                    );
                }
                same_bytes(&format!("bin2base64 v={v} len={len}"), &ob, &or);
            }
        }
    }
}

#[test]
fn rows10_15_base642bin() {
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
    let (encc, _) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz, c_int) -> *mut c_char,
    >("sodium_bin2base64");
    let (elc, _) = pair::<unsafe extern "C" fn(Sz, c_int) -> Sz>("sodium_base64_encoded_len");
    let mut rng = Rng::seeded();

    let ignores: [Option<&[u8]>; 3] = [None, Some(b" \0"), Some(b" \r\n\0")];

    for v in B64_VARIANTS {
        for len in 0usize..=48 {
            for _ in 0..5 {
                let bin = rng.bytes(len);
                let cap = unsafe { elc(len, v) };
                let mut enc = vec![0u8; cap + 1];
                unsafe {
                    encc(enc.as_mut_ptr() as *mut c_char, cap, bin.as_ptr(), len, v);
                }
                let nul = enc.iter().position(|&x| x == 0).unwrap();
                let mut b64: Vec<u8> = enc[..nul].to_vec();
                // sprinkle ignorable whitespace / trailing junk
                let mode = rng.next_u8() % 4;
                if mode == 1 && !b64.is_empty() {
                    let at = rng.below(b64.len());
                    b64.insert(at, b' ');
                } else if mode == 2 {
                    b64.push(b' ');
                    b64.push(b' ');
                } else if mode == 3 {
                    b64.push(b'*');
                }

                for ig in ignores.iter() {
                    for &use_end in &[false, true] {
                        for &use_binlen in &[false, true] {
                            for &maxl in &[len, len + 3] {
                                let mut ob = buf(maxl + 8);
                                let mut or = buf(maxl + 8);
                                let mut bc: Sz = 0xDEAD;
                                let mut br: Sz = 0xDEAD;
                                let mut ec: *const c_char = std::ptr::null();
                                let mut er: *const c_char = std::ptr::null();
                                let igp = match ig {
                                    None => std::ptr::null(),
                                    Some(s) => s.as_ptr() as *const c_char,
                                };
                                unsafe {
                                    let rc = c(
                                        ob.as_mut_ptr(),
                                        maxl,
                                        b64.as_ptr() as *const c_char,
                                        b64.len(),
                                        igp,
                                        if use_binlen { &mut bc } else { std::ptr::null_mut() },
                                        if use_end { &mut ec } else { std::ptr::null_mut() },
                                        v,
                                    );
                                    let rr = r(
                                        or.as_mut_ptr(),
                                        maxl,
                                        b64.as_ptr() as *const c_char,
                                        b64.len(),
                                        igp,
                                        if use_binlen { &mut br } else { std::ptr::null_mut() },
                                        if use_end { &mut er } else { std::ptr::null_mut() },
                                        v,
                                    );
                                    let ctx = format!(
                                        "base642bin v={v} b64={:?} maxl={maxl} ig={} end={use_end}",
                                        String::from_utf8_lossy(&b64),
                                        ig.is_some()
                                    );
                                    same_ret(&ctx, rc, rr);
                                    same_bytes(&ctx, &ob, &or);
                                    assert_eq!(bc, br, "{ctx}: bin_len differs");
                                    if use_end {
                                        assert_eq!(
                                            ec as usize - b64.as_ptr() as usize,
                                            er as usize - b64.as_ptr() as usize,
                                            "{ctx}: b64_end differs"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------- rows 23-26

#[test]
fn rows23_26_pad_unpad() {
    let (pc, pr) = pair::<
        unsafe extern "C" fn(*mut Sz, *mut u8, Sz, Sz, Sz) -> c_int,
    >("sodium_pad");
    let (uc, ur) = pair::<
        unsafe extern "C" fn(*mut Sz, *const u8, Sz, Sz) -> c_int,
    >("sodium_unpad");
    let mut rng = Rng::seeded();
    for &bs in &[1usize, 2, 3, 4, 5, 7, 8, 16, 17, 32, 64, 100] {
        for ul in 0usize..=70 {
            for _ in 0..3 {
                let base = rng.bytes(ul);
                for &use_out in &[false, true] {
                    for &slack in &[0usize, 1, bs, bs * 2] {
                        let maxl = ul + bs + slack + 1;
                        let mut a = base.clone();
                        a.resize(maxl, CANARY);
                        let mut b = a.clone();
                        let mut lc: Sz = 0xDEAD;
                        let mut lr: Sz = 0xDEAD;
                        let (rc, rr) = unsafe {
                            (
                                pc(
                                    if use_out { &mut lc } else { std::ptr::null_mut() },
                                    a.as_mut_ptr(),
                                    ul,
                                    bs,
                                    maxl,
                                ),
                                pr(
                                    if use_out { &mut lr } else { std::ptr::null_mut() },
                                    b.as_mut_ptr(),
                                    ul,
                                    bs,
                                    maxl,
                                ),
                            )
                        };
                        let ctx = format!("sodium_pad bs={bs} ul={ul} maxl={maxl}");
                        same_ret(&ctx, rc, rr);
                        same_bytes(&ctx, &a, &b);
                        assert_eq!(lc, lr, "{ctx}: padded_buflen differs");
                        if rc == 0 && use_out {
                            // now unpad
                            let plen = lc;
                            let mut xc: Sz = 0xDEAD;
                            let mut xr: Sz = 0xDEAD;
                            let (qc, qr) = unsafe {
                                (
                                    uc(&mut xc, a.as_ptr(), plen, bs),
                                    ur(&mut xr, b.as_ptr(), plen, bs),
                                )
                            };
                            let ctx2 = format!("sodium_unpad bs={bs} plen={plen}");
                            same_ret(&ctx2, qc, qr);
                            assert_eq!(xc, xr, "{ctx2}: unpadded_buflen differs");
                            if qc == 0 {
                                assert_eq!(xc, ul, "{ctx2}: round-trip length");
                            }
                            // NOTE: sodium_unpad() dereferences
                            // `unpadded_buflen_p` unconditionally in the C, so
                            // NULL is UB there and is not a testable case.
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------- rows 27-34

#[test]
fn rows27_34_ip2bin_bin2ip() {
    let (ic, ir) =
        pair::<unsafe extern "C" fn(*mut u8, *const c_char, Sz) -> c_int>("sodium_ip2bin");
    let (bc, br) =
        pair::<unsafe extern "C" fn(*mut c_char, Sz, *const u8) -> *mut c_char>("sodium_bin2ip");

    let mut ips: Vec<String> = vec![
        "0.0.0.0".into(),
        "127.0.0.1".into(),
        "255.255.255.255".into(),
        "1.2.3.4".into(),
        "10.0.0.255".into(),
        "::".into(),
        "::1".into(),
        "1::".into(),
        "::ffff:1.2.3.4".into(),
        "::ffff:255.255.255.255".into(),
        "2001:db8::1".into(),
        "2001:0db8:0000:0000:0000:0000:0000:0001".into(),
        "fe80::1%eth0".into(),
        "fe80::1%1".into(),
        "fe80::1%a-b_c.d".into(),
        "1:2:3:4:5:6:7:8".into(),
        "1:2:3:4:5:6:1.2.3.4".into(),
        "1::8".into(),
        "1:2::7:8".into(),
        "::2:3:4:5:6:7:8".into(),
        "1:2:3:4:5:6:7::".into(),
        "0:0:0:0:0:0:0:0".into(),
        "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff".into(),
        "abcd:ef01:2345:6789:abcd:ef01:2345:6789".into(),
        "0:0:0:1:0:0:0:1".into(),
        "1:0:0:0:0:0:0:1".into(),
        "::ffff:0:0".into(),
        "64:ff9b::1.2.3.4".into(),
        // invalid ones (still must agree)
        "".into(),
        "1".into(),
        "1.2.3".into(),
        "1.2.3.4.5".into(),
        "256.1.1.1".into(),
        "1.2.3.04".into(),
        ":::".into(),
        "1:::2".into(),
        "1:2:3:4:5:6:7:8:9".into(),
        "gggg::1".into(),
        "1.2.3.4%eth0".into(),
        "fe80::1%".into(),
        "fe80::1%eth 0".into(),
        "12345::1".into(),
    ];
    let mut rng = Rng::seeded();
    // random IPv4/IPv6 text
    for _ in 0..200 {
        if rng.next_u8() & 1 == 0 {
            ips.push(format!(
                "{}.{}.{}.{}",
                rng.next_u8(),
                rng.next_u8(),
                rng.next_u8(),
                rng.next_u8()
            ));
        } else {
            let g: Vec<String> = (0..8).map(|_| format!("{:x}", rng.next_u32() & 0xffff)).collect();
            ips.push(g.join(":"));
        }
    }

    for ip in &ips {
        let ipb = ip.as_bytes();
        for &ilen in &[ipb.len()] {
            let mut ob = buf(16 + 8);
            let mut or = buf(16 + 8);
            let (rc, rr) = unsafe {
                (
                    ic(ob.as_mut_ptr(), ipb.as_ptr() as *const c_char, ilen),
                    ir(or.as_mut_ptr(), ipb.as_ptr() as *const c_char, ilen),
                )
            };
            let ctx = format!("sodium_ip2bin {ip:?}");
            same_ret(&ctx, rc, rr);
            same_bytes(&ctx, &ob, &or);
        }
    }

    // bin2ip over the 16-byte binaries plus randoms and various ip_maxlen
    let mut bins: Vec<[u8; 16]> = Vec::new();
    for ip in &ips {
        let mut b = [0u8; 16];
        let ipb = ip.as_bytes();
        if unsafe { ic(b.as_mut_ptr(), ipb.as_ptr() as *const c_char, ipb.len()) } == 0 {
            bins.push(b);
        }
    }
    for _ in 0..300 {
        let mut b = [0u8; 16];
        rng.fill(&mut b);
        // sometimes make it IPv4-mapped
        match rng.next_u8() % 4 {
            0 => {
                b[..10].fill(0);
                b[10] = 0xff;
                b[11] = 0xff;
            }
            1 => {
                let z = rng.below(15);
                b[..z].fill(0);
            }
            _ => {}
        }
        bins.push(b);
    }
    for b in &bins {
        for &maxl in &[0usize, 1, 2, 3, 4, 8, 10, 16, 22, 39, 40, 46, 64] {
            let mut ob = buf(maxl + 8);
            let mut or = buf(maxl + 8);
            unsafe {
                let pc = bc(ob.as_mut_ptr() as *mut c_char, maxl, b.as_ptr());
                let pr = br(or.as_mut_ptr() as *mut c_char, maxl, b.as_ptr());
                let ctx = format!("sodium_bin2ip {} maxl={maxl}", hex(b));
                assert_eq!(pc.is_null(), pr.is_null(), "{ctx}: NULL-ness differs");
                if !pc.is_null() {
                    assert_eq!(
                        pc as usize - ob.as_ptr() as usize,
                        pr as usize - or.as_ptr() as usize,
                        "{ctx}: return ptr offset"
                    );
                }
                same_bytes(&ctx, &ob, &or);
            }
        }
    }
}

// ----------------------------------------------------------------- rows 35-37

#[test]
fn row35_crypto_verify() {
    for (name, n) in [("crypto_verify_16", 16usize), ("crypto_verify_32", 32), ("crypto_verify_64", 64)] {
        let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8) -> c_int>(name);
        let mut rng = Rng::new(SEED ^ n as u64);
        for _ in 0..200 {
            let a = rng.bytes(n);
            let mut b = a.clone();
            match rng.next_u8() % 4 {
                0 => {}
                1 => b[0] ^= 0x80,
                2 => b[n / 2] ^= 1,
                _ => b[n - 1] ^= 0xff,
            }
            unsafe { same_ret(name, c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr())) };
        }
    }
    // *_bytes getters
    for name in ["crypto_verify_16_bytes", "crypto_verify_32_bytes", "crypto_verify_64_bytes"] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(name);
        unsafe { assert_eq!(c(), r(), "{name}") };
    }
}

// ----------------------------------------------------------------- rows 36-37

#[test]
fn rows36_37_shorthash() {
    let lens = [0usize, 1, 2, 7, 8, 9, 15, 16, 17, 31, 32, 63, 64, 65, 127, 128, 1000];
    for (name, outn, keyn) in [
        ("crypto_shorthash", 8usize, 16usize),
        ("crypto_shorthash_siphash24", 8, 16),
        ("crypto_shorthash_siphashx24", 16, 16),
    ] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int>(name);
        let mut rng = Rng::new(SEED ^ outn as u64);
        for &l in &lens {
            for _ in 0..20 {
                let m = rng.bytes(l);
                let k = rng.bytes(keyn);
                let mut ob = buf(outn);
                let mut or = buf(outn);
                unsafe {
                    same_ret(
                        name,
                        c(ob.as_mut_ptr(), m.as_ptr(), l as u64, k.as_ptr()),
                        r(or.as_mut_ptr(), m.as_ptr(), l as u64, k.as_ptr()),
                    );
                }
                same_bytes(&format!("{name} len={l}"), &ob, &or);
            }
        }
    }
}

// ----------------------------------------------------------------- rows 38-45

#[test]
fn rows38_45_onetimeauth() {
    for prefix in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let (sbc, sbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
        let statebytes = unsafe { sbc() };
        unsafe { assert_eq!(statebytes, sbr(), "{prefix}_statebytes") };

        let (osc, osr) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int,
        >(prefix);
        let (vc, vr) = pair::<
            unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int,
        >(&format!("{prefix}_verify"));
        let (ic, ir) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(&format!("{prefix}_init"));
        let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int>(&format!(
            "{prefix}_update"
        ));
        let (fc, fr) =
            pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{prefix}_final"));

        let mut rng = Rng::seeded();
        let lens = [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1000, 4096];
        for &l in &lens {
            for _ in 0..12 {
                let m = rng.bytes(l);
                let k = rng.bytes(32);
                let mut ob = buf(16);
                let mut or = buf(16);
                unsafe {
                    same_ret(
                        prefix,
                        osc(ob.as_mut_ptr(), m.as_ptr(), l as u64, k.as_ptr()),
                        osr(or.as_mut_ptr(), m.as_ptr(), l as u64, k.as_ptr()),
                    );
                }
                same_bytes(&format!("{prefix} one-shot len={l}"), &ob, &or);

                // verify: correct tag
                unsafe {
                    same_ret(
                        &format!("{prefix}_verify ok"),
                        vc(ob.as_ptr(), m.as_ptr(), l as u64, k.as_ptr()),
                        vr(or.as_ptr(), m.as_ptr(), l as u64, k.as_ptr()),
                    );
                }

                // streaming with N random chunks
                for nchunks in [0usize, 1, 2, 3, 7] {
                    let mut splits: Vec<usize> = Vec::new();
                    if nchunks > 0 && l > 0 {
                        for _ in 0..nchunks - 1 {
                            splits.push(rng.below(l + 1));
                        }
                    }
                    splits.push(l);
                    splits.sort_unstable();
                    let mut stc = buf(statebytes);
                    let mut str_ = buf(statebytes);
                    let mut tc = buf(16);
                    let mut tr = buf(16);
                    unsafe {
                        same_ret("init", ic(stc.as_mut_ptr(), k.as_ptr()), ir(str_.as_mut_ptr(), k.as_ptr()));
                        let mut off = 0usize;
                        for &s in &splits {
                            let n = s - off;
                            same_ret(
                                "update",
                                uc(stc.as_mut_ptr(), m[off..].as_ptr(), n as u64),
                                ur(str_.as_mut_ptr(), m[off..].as_ptr(), n as u64),
                            );
                            off = s;
                        }
                        same_ret(
                            "final",
                            fc(stc.as_mut_ptr(), tc.as_mut_ptr()),
                            fr(str_.as_mut_ptr(), tr.as_mut_ptr()),
                        );
                    }
                    same_bytes(&format!("{prefix} streaming len={l} n={nchunks}"), &tc, &tr);
                    same_bytes(&format!("{prefix} streaming vs one-shot len={l}"), &ob, &tc);
                }
            }
        }
        // getters
        for g in [format!("{prefix}_bytes"), format!("{prefix}_keybytes")] {
            let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
            unsafe { assert_eq!(c(), r(), "{g}") };
        }
    }
    // primitive string
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_onetimeauth_primitive");
    unsafe {
        let sc = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let sr = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_onetimeauth_primitive", &sc, &sr);
    }
}

// ----------------------------------------------------------------- rows 46-51

#[test]
fn rows46_51_ipcrypt() {
    let mut rng = Rng::seeded();

    // deterministic
    {
        let (ec, er) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>("crypto_ipcrypt_encrypt");
        let (dc, dr) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>("crypto_ipcrypt_decrypt");
        for _ in 0..300 {
            let k = rng.bytes(16);
            let ip = rng.bytes(16);
            let mut oc = buf(16);
            let mut or = buf(16);
            unsafe {
                ec(oc.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                er(or.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            same_bytes("ipcrypt_encrypt", &oc, &or);
            let mut bc = buf(16);
            let mut brr = buf(16);
            unsafe {
                dc(bc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                dr(brr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
            }
            same_bytes("ipcrypt_decrypt", &bc, &brr);
            same_bytes("ipcrypt round-trip", &bc, &ip);
        }
    }

    // nd
    {
        let (ec, er) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
        >("crypto_ipcrypt_nd_encrypt");
        let (dc, dr) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>("crypto_ipcrypt_nd_decrypt");
        for _ in 0..300 {
            let k = rng.bytes(16);
            let ip = rng.bytes(16);
            let tw = rng.bytes(8);
            let mut oc = buf(24);
            let mut or = buf(24);
            unsafe {
                ec(oc.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                er(or.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
            }
            same_bytes("nd_encrypt", &oc, &or);
            let mut bc = buf(16);
            let mut brr = buf(16);
            unsafe {
                dc(bc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                dr(brr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
            }
            same_bytes("nd_decrypt", &bc, &brr);
            same_bytes("nd round-trip", &bc, &ip);
        }
    }

    // ndx
    {
        let (ec, er) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
        >("crypto_ipcrypt_ndx_encrypt");
        let (dc, dr) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>("crypto_ipcrypt_ndx_decrypt");
        for _ in 0..300 {
            let k = rng.bytes(32);
            let ip = rng.bytes(16);
            let tw = rng.bytes(16);
            let mut oc = buf(32);
            let mut or = buf(32);
            unsafe {
                ec(oc.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                er(or.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
            }
            same_bytes("ndx_encrypt", &oc, &or);
            let mut bc = buf(16);
            let mut brr = buf(16);
            unsafe {
                dc(bc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                dr(brr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
            }
            same_bytes("ndx_decrypt", &bc, &brr);
            same_bytes("ndx round-trip", &bc, &ip);
        }
        // degenerate key: k == k^0x5a-ish patterns to try to hit the rekey guard
        for pat in [0x00u8, 0xff, 0x5a, 0xa5] {
            let k = vec![pat; 32];
            let ip = vec![0u8; 16];
            let tw = vec![0u8; 16];
            let mut oc = buf(32);
            let mut or = buf(32);
            unsafe {
                ec(oc.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
                er(or.as_mut_ptr(), ip.as_ptr(), tw.as_ptr(), k.as_ptr());
            }
            same_bytes(&format!("ndx degenerate pat={pat:02x}"), &oc, &or);
        }
    }

    // pfx — both the IPv4-mapped and full-128 branches
    {
        let (ec, er) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>("crypto_ipcrypt_pfx_encrypt");
        let (dc, dr) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8)>("crypto_ipcrypt_pfx_decrypt");
        for i in 0..400 {
            let k = rng.bytes(32);
            let mut ip = rng.bytes(16);
            if i % 2 == 0 {
                // IPv4-mapped shape -> prefix_start = 96
                ip[..10].fill(0);
                ip[10] = 0xff;
                ip[11] = 0xff;
            }
            let mut oc = buf(16);
            let mut or = buf(16);
            unsafe {
                ec(oc.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                er(or.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            same_bytes(&format!("pfx_encrypt v4mapped={}", i % 2 == 0), &oc, &or);
            let mut bc = buf(16);
            let mut brr = buf(16);
            unsafe {
                dc(bc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                dr(brr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
            }
            same_bytes("pfx_decrypt", &bc, &brr);
            same_bytes("pfx round-trip", &bc, &ip);
        }
        for pat in [0x00u8, 0xff, 0x5a, 0xa5] {
            let k = vec![pat; 32];
            let ip = vec![0u8; 16];
            let mut oc = buf(16);
            let mut or = buf(16);
            unsafe {
                ec(oc.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                er(or.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            same_bytes(&format!("pfx degenerate pat={pat:02x}"), &oc, &or);
        }
    }
}

// ----------------------------------------------------------------- rows 52-59

#[test]
fn row52_randombytes_buf_deterministic() {
    let (c, r) =
        pair::<unsafe extern "C" fn(*mut u8, Sz, *const u8)>("randombytes_buf_deterministic");
    let mut rng = Rng::seeded();
    for &n in &[0usize, 1, 2, 31, 32, 63, 64, 65, 127, 128, 129, 1000, 4096, 8192] {
        for _ in 0..10 {
            let seed = rng.bytes(32);
            let mut ob = buf(n + 8);
            let mut or = buf(n + 8);
            unsafe {
                c(ob.as_mut_ptr(), n, seed.as_ptr());
                r(or.as_mut_ptr(), n, seed.as_ptr());
            }
            same_bytes(&format!("randombytes_buf_deterministic n={n}"), &ob, &or);
        }
    }
}

#[test]
fn row53_randombytes_uniform() {
    let (c, r) = pair::<unsafe extern "C" fn(u32) -> u32>("randombytes_uniform");
    // Not deterministic; assert the range contract holds identically.
    for &ub in &[0u32, 1, 2, 3, 255, 256, 65537, 1 << 31, u32::MAX] {
        for _ in 0..200 {
            unsafe {
                let a = c(ub);
                let b = r(ub);
                if ub < 2 {
                    assert_eq!(a, 0, "C uniform({ub})");
                    assert_eq!(b, 0, "Rust uniform({ub})");
                } else {
                    assert!(a < ub, "C uniform({ub}) = {a}");
                    assert!(b < ub, "Rust uniform({ub}) = {b}");
                }
            }
        }
    }
}

#[test]
fn rows54_59_getters_and_misc() {
    // size_t () getters
    for name in [
        "randombytes_seedbytes",
        "crypto_shorthash_bytes",
        "crypto_shorthash_keybytes",
        "crypto_shorthash_siphash24_bytes",
        "crypto_shorthash_siphash24_keybytes",
        "crypto_shorthash_siphashx24_bytes",
        "crypto_shorthash_siphashx24_keybytes",
        "crypto_ipcrypt_bytes",
        "crypto_ipcrypt_keybytes",
        "crypto_ipcrypt_nd_keybytes",
        "crypto_ipcrypt_nd_tweakbytes",
        "crypto_ipcrypt_nd_inputbytes",
        "crypto_ipcrypt_nd_outputbytes",
        "crypto_ipcrypt_ndx_keybytes",
        "crypto_ipcrypt_ndx_tweakbytes",
        "crypto_ipcrypt_ndx_inputbytes",
        "crypto_ipcrypt_ndx_outputbytes",
    ] {
        if !libs().has(name) {
            continue;
        }
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(name);
        unsafe { assert_eq!(c(), r(), "{name}") };
    }
    // int () getters
    for name in [
        "sodium_library_version_major",
        "sodium_library_version_minor",
        "sodium_library_minimal",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> c_int>(name);
        unsafe { assert_eq!(c(), r(), "{name}") };
    }
    // const char* () getters
    for name in [
        "sodium_version_string",
        "crypto_shorthash_primitive",
        "randombytes_implementation_name",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>(name);
        unsafe {
            let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
            let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
            same_bytes(name, &a, &b);
        }
    }
    // sodium_init idempotence
    let (c, r) = pair::<unsafe extern "C" fn() -> c_int>("sodium_init");
    unsafe {
        same_ret("sodium_init re-entry", c(), r());
        same_ret("sodium_init re-entry 2", c(), r());
    }
    // sodium_memzero / stackzero
    let (mzc, mzr) = pair::<unsafe extern "C" fn(*mut u8, Sz)>("sodium_memzero");
    let mut rng = Rng::seeded();
    for &n in &[0usize, 1, 16, 64, 257] {
        let base = rng.bytes(n + 8);
        let mut a = base.clone();
        let mut b = base.clone();
        unsafe {
            mzc(a.as_mut_ptr(), n);
            mzr(b.as_mut_ptr(), n);
        }
        same_bytes("sodium_memzero", &a, &b);
    }
    let (szc, szr) = pair::<unsafe extern "C" fn(Sz)>("sodium_stackzero");
    unsafe {
        szc(64);
        szr(64);
    }
    // sodium_malloc / allocarray / free
    let (mc, mr) = pair::<unsafe extern "C" fn(Sz) -> *mut u8>("sodium_malloc");
    let (ac, ar) = pair::<unsafe extern "C" fn(Sz, Sz) -> *mut u8>("sodium_allocarray");
    let (frc, frr) = pair::<unsafe extern "C" fn(*mut u8)>("sodium_free");
    for &n in &[1usize, 8, 64, 4096] {
        unsafe {
            let p = mc(n);
            let q = mr(n);
            assert!(!p.is_null() && !q.is_null(), "sodium_malloc({n})");
            let a = std::slice::from_raw_parts(p, n).to_vec();
            let b = std::slice::from_raw_parts(q, n).to_vec();
            same_bytes(&format!("sodium_malloc prefill n={n}"), &a, &b);
            frc(p);
            frr(q);
            let p = ac(4, n);
            let q = ar(4, n);
            assert!(!p.is_null() && !q.is_null());
            let a = std::slice::from_raw_parts(p, 4 * n).to_vec();
            let b = std::slice::from_raw_parts(q, 4 * n).to_vec();
            same_bytes(&format!("sodium_allocarray prefill n={n}"), &a, &b);
            frc(p);
            frr(q);
        }
    }
    unsafe {
        frc(std::ptr::null_mut());
        frr(std::ptr::null_mut());
    }
    // randombytes_buf / random / stir: nondeterministic, just exercise them
    let (rbc, rbr) = pair::<unsafe extern "C" fn(*mut u8, Sz)>("randombytes_buf");
    let mut t = buf(64);
    unsafe {
        rbc(t.as_mut_ptr(), 0);
        rbr(t.as_mut_ptr(), 0);
        rbc(t.as_mut_ptr(), 64);
        rbr(t.as_mut_ptr(), 64);
    }
    let (rrc, rrr) = pair::<unsafe extern "C" fn() -> u32>("randombytes_random");
    unsafe {
        let _ = rrc();
        let _ = rrr();
    }
    let (sc, sr) = pair::<unsafe extern "C" fn()>("randombytes_stir");
    unsafe {
        sc();
        sr();
    }
}
