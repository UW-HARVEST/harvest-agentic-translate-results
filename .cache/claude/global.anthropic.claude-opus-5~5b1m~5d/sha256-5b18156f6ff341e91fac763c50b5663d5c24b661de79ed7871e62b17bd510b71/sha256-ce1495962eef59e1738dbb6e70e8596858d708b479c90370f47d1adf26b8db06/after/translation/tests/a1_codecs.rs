//! Area 1 — `sodium/codecs.c`: hex, base64, IP text codecs.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

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
type EncLen = unsafe extern "C" fn(usize, c_int) -> usize;
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
type Ip2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
type Bin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;

const VARIANTS: [c_int; 4] = [1, 3, 5, 7];

// ---------------------------------------------------------------- bin2hex

#[test]
fn bin2hex_valid_all_shapes() {
    let (c, r) = both::<Bin2Hex>("sodium_bin2hex");
    let mut rng = Rng::new(0x11);
    for bin_len in 0..=64usize {
        for _ in 0..8 {
            let bin = rng.bytes(bin_len);
            // hex_maxlen must be > bin_len*2
            for extra in [1usize, 2, 7] {
                let maxlen = bin_len * 2 + extra;
                let mut ob = padded(maxlen);
                let mut or = padded(maxlen);
                unsafe {
                    let pc = c(ob.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len);
                    let pr = r(or.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len);
                    assert_eq!(pc as usize, ob.as_ptr() as usize);
                    assert_eq!(pr as usize, or.as_ptr() as usize);
                }
                // Only the first bin_len*2+1 bytes are defined by the C code.
                eqb("bin2hex", &ob[..bin_len * 2 + 1], &or[..bin_len * 2 + 1]);
                check_pad("bin2hex(C)", &ob, maxlen);
                check_pad("bin2hex(Rust)", &or, maxlen);
            }
        }
    }
}

#[test]
fn bin2hex_misuse_hex_maxlen_too_small() {
    // hex_maxlen <= bin_len * 2  =>  sodium_misuse() -> abort
    let (c, r) = both::<Bin2Hex>("sodium_bin2hex");
    for (bin_len, maxlen) in [(0usize, 0usize), (1, 0), (1, 1), (1, 2), (16, 32), (16, 5)] {
        let bin = vec![0xABu8; bin_len.max(1)];
        let cc = c.clone();
        let rr = r.clone();
        let b1 = bin.clone();
        let b2 = bin.clone();
        eq_abort(
            &format!("bin2hex(bin_len={bin_len}, hex_maxlen={maxlen})"),
            move || unsafe {
                let mut o = vec![0u8; maxlen + 8];
                cc(o.as_mut_ptr() as *mut c_char, maxlen, b1.as_ptr(), bin_len);
            },
            move || unsafe {
                let mut o = vec![0u8; maxlen + 8];
                rr(o.as_mut_ptr() as *mut c_char, maxlen, b2.as_ptr(), bin_len);
            },
        );
    }
}

// ---------------------------------------------------------------- hex2bin

fn hex2bin_case(
    c: &Hex2Bin,
    r: &Hex2Bin,
    hex: &[u8],
    bin_maxlen: usize,
    ignore: Option<&[u8]>,
    want_bin_len: bool,
    want_hex_end: bool,
    label: &str,
) {
    let ign_c: Option<Vec<u8>> = ignore.map(|s| {
        let mut v = s.to_vec();
        v.push(0);
        v
    });
    let ign_ptr = ign_c
        .as_ref()
        .map(|v| v.as_ptr() as *const c_char)
        .unwrap_or(std::ptr::null());

    let mut bc = padded(bin_maxlen);
    let mut br = padded(bin_maxlen);
    let mut lc: usize = usize::MAX;
    let mut lr: usize = usize::MAX;
    let mut ec: *const c_char = std::ptr::null();
    let mut er: *const c_char = std::ptr::null();

    set_errno(0);
    let rc = unsafe {
        c(
            bc.as_mut_ptr(),
            bin_maxlen,
            hex.as_ptr() as *const c_char,
            hex.len(),
            ign_ptr,
            if want_bin_len { &mut lc } else { std::ptr::null_mut() },
            if want_hex_end { &mut ec } else { std::ptr::null_mut() },
        )
    };
    let errc = errno();
    set_errno(0);
    let rr = unsafe {
        r(
            br.as_mut_ptr(),
            bin_maxlen,
            hex.as_ptr() as *const c_char,
            hex.len(),
            ign_ptr,
            if want_bin_len { &mut lr } else { std::ptr::null_mut() },
            if want_hex_end { &mut er } else { std::ptr::null_mut() },
        )
    };
    let errr = errno();

    eqi(&format!("hex2bin ret [{label}]"), rc, rr);
    if rc != 0 {
        assert_eq!(errc, errr, "hex2bin errno [{label}]");
    }
    if want_bin_len {
        assert_eq!(lc, lr, "hex2bin *bin_len [{label}]");
    }
    if want_hex_end {
        let oc = ec as usize - hex.as_ptr() as usize;
        let or = er as usize - hex.as_ptr() as usize;
        assert_eq!(oc, or, "hex2bin *hex_end offset [{label}]");
    }
    let n = if want_bin_len { lc.min(bin_maxlen) } else { bin_maxlen };
    eqb(&format!("hex2bin out [{label}]"), &bc[..n], &br[..n]);
    check_pad("hex2bin(C)", &bc, bin_maxlen);
    check_pad("hex2bin(Rust)", &br, bin_maxlen);
}

#[test]
fn hex2bin_round_trip_and_shapes() {
    let (c, r) = both::<Hex2Bin>("sodium_hex2bin");
    let mut rng = Rng::new(0x22);
    for bin_len in 0..=48usize {
        for _ in 0..6 {
            let bin = rng.bytes(bin_len);
            let mut hex = String::new();
            for b in &bin {
                hex.push_str(&format!("{b:02X}"));
            }
            let lower = hex.to_lowercase();
            for h in [hex.as_bytes(), lower.as_bytes()] {
                for maxlen in [bin_len, bin_len + 1, bin_len + 7] {
                    for (bl, he) in [(true, true), (true, false), (false, true), (false, false)] {
                        hex2bin_case(&c, &r, h, maxlen, None, bl, he, "roundtrip");
                    }
                }
            }
        }
    }
}

#[test]
fn hex2bin_ignore_sets() {
    let (c, r) = both::<Hex2Bin>("sodium_hex2bin");
    let inputs: &[&[u8]] = &[
        b"",
        b":",
        b"::",
        b"de:ad:be:ef",
        b":deadbeef",
        b"deadbeef:",
        b"de: ad be\tef",
        b"d:e:a:d",
        b"de ad be ef ",
        b"  ",
        b"deadbeefzz",
        b"zzdeadbeef",
        b"de\nad",
        b"0",
        b"0:",
        b":0",
        b"00:0",
        b"aBcDeF0123456789",
        b"g",
        b"@",
        b"/",
        b"\x00ab",
        b"ab\x00cd",
        b"\xff\xfe",
    ];
    let ignores: &[Option<&[u8]>] = &[None, Some(b": \t\n"), Some(b""), Some(b":"), Some(b"z")];
    for inp in inputs {
        for ign in ignores {
            for maxlen in [0usize, 1, 2, 3, 4, 8] {
                for (bl, he) in [(true, true), (true, false), (false, true), (false, false)] {
                    hex2bin_case(
                        &c,
                        &r,
                        inp,
                        maxlen,
                        *ign,
                        bl,
                        he,
                        &format!("{:?}/{:?}/{maxlen}", String::from_utf8_lossy(inp), ign),
                    );
                }
            }
        }
    }
}

#[test]
fn hex2bin_random_fuzz() {
    let (c, r) = both::<Hex2Bin>("sodium_hex2bin");
    let alphabet: &[u8] = b"0123456789abcdefABCDEF:; \t\nxXzZ=/+\x00\xff\x7f";
    let mut rng = Rng::new(0x33);
    for _ in 0..4000 {
        let n = rng.below(24);
        let s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let maxlen = rng.below(13);
        let ign: Option<&[u8]> = match rng.below(4) {
            0 => None,
            1 => Some(b": \t\n"),
            2 => Some(b";"),
            _ => Some(b""),
        };
        let bl = rng.below(2) == 0;
        let he = rng.below(2) == 0;
        hex2bin_case(&c, &r, &s, maxlen, ign, bl, he, "fuzz");
    }
}

// ---------------------------------------------------------------- base64

#[test]
fn base64_encoded_len_all_variants() {
    let (c, r) = both::<EncLen>("sodium_base64_encoded_len");
    for v in VARIANTS {
        for bin_len in 0..300usize {
            unsafe {
                assert_eq!(c(bin_len, v), r(bin_len, v), "encoded_len({bin_len},{v})");
            }
        }
        for bin_len in [1000usize, 65535, 1 << 20, usize::MAX / 8] {
            unsafe {
                assert_eq!(c(bin_len, v), r(bin_len, v), "encoded_len({bin_len},{v})");
            }
        }
    }
}

#[test]
fn base64_encoded_len_invalid_variant_aborts() {
    let (c, r) = both::<EncLen>("sodium_base64_encoded_len");
    for v in [0i32, 2, 4, 6, 8, 9, -1, 100, i32::MIN, i32::MAX] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("base64_encoded_len variant {v}"),
            move || unsafe {
                std::hint::black_box(cc(16, v));
            },
            move || unsafe {
                std::hint::black_box(rr(16, v));
            },
        );
    }
}

#[test]
fn base64_encoded_len_overflow_aborts() {
    let (c, r) = both::<EncLen>("sodium_base64_encoded_len");
    for v in VARIANTS {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("base64_encoded_len overflow variant {v}"),
            move || unsafe {
                std::hint::black_box(cc(usize::MAX, v));
            },
            move || unsafe {
                std::hint::black_box(rr(usize::MAX, v));
            },
        );
    }
}

#[test]
fn bin2base64_valid_all_variants_and_shapes() {
    let (enc, _) = both::<EncLen>("sodium_base64_encoded_len");
    let (c, r) = both::<Bin2B64>("sodium_bin2base64");
    let mut rng = Rng::new(0x44);
    for v in VARIANTS {
        for bin_len in 0..=70usize {
            for _ in 0..6 {
                let bin = rng.bytes(bin_len);
                let need = unsafe { enc(bin_len, v) };
                for maxlen in [need, need + 1, need + 9] {
                    let mut ob = padded(maxlen);
                    let mut or = padded(maxlen);
                    unsafe {
                        c(ob.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len, v);
                        r(or.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len, v);
                    }
                    // bin2base64 zero-fills all the way to b64_maxlen.
                    eqb(&format!("bin2base64 v{v} len{bin_len}"), &ob, &or);
                    check_pad("bin2base64(C)", &ob, maxlen);
                    check_pad("bin2base64(Rust)", &or, maxlen);
                }
            }
        }
    }
}

#[test]
fn bin2base64_b64_maxlen_too_small_aborts() {
    let (enc, _) = both::<EncLen>("sodium_base64_encoded_len");
    let (c, r) = both::<Bin2B64>("sodium_bin2base64");
    for v in VARIANTS {
        for bin_len in [0usize, 1, 2, 3, 4, 5, 6, 31, 32] {
            let need = unsafe { enc(bin_len, v) };
            let too_small = need - 1; // == b64_len, triggers `b64_maxlen <= b64_len`
            let cc = c.clone();
            let rr = r.clone();
            eq_abort(
                &format!("bin2base64 v{v} bin_len{bin_len} maxlen{too_small}"),
                move || unsafe {
                    let bin = vec![0x5Au8; bin_len.max(1)];
                    let mut o = vec![0u8; too_small + 8];
                    cc(o.as_mut_ptr() as *mut c_char, too_small, bin.as_ptr(), bin_len, v);
                },
                move || unsafe {
                    let bin = vec![0x5Au8; bin_len.max(1)];
                    let mut o = vec![0u8; too_small + 8];
                    rr(o.as_mut_ptr() as *mut c_char, too_small, bin.as_ptr(), bin_len, v);
                },
            );
        }
    }
}

#[test]
fn bin2base64_invalid_variant_aborts() {
    let (c, r) = both::<Bin2B64>("sodium_bin2base64");
    for v in [0i32, 2, 4, 6, 8, -1, 255] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("bin2base64 variant {v}"),
            move || unsafe {
                let bin = [1u8, 2, 3];
                let mut o = [0u8; 64];
                cc(o.as_mut_ptr() as *mut c_char, 64, bin.as_ptr(), 3, v);
            },
            move || unsafe {
                let bin = [1u8, 2, 3];
                let mut o = [0u8; 64];
                rr(o.as_mut_ptr() as *mut c_char, 64, bin.as_ptr(), 3, v);
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn b642bin_case(
    c: &B642Bin,
    r: &B642Bin,
    b64: &[u8],
    bin_maxlen: usize,
    ignore: Option<&[u8]>,
    want_bin_len: bool,
    want_end: bool,
    variant: c_int,
    label: &str,
) {
    let ign_c: Option<Vec<u8>> = ignore.map(|s| {
        let mut v = s.to_vec();
        v.push(0);
        v
    });
    let ign_ptr = ign_c
        .as_ref()
        .map(|v| v.as_ptr() as *const c_char)
        .unwrap_or(std::ptr::null());

    let mut bc = padded(bin_maxlen);
    let mut br = padded(bin_maxlen);
    let mut lc: usize = usize::MAX;
    let mut lr: usize = usize::MAX;
    let mut ec: *const c_char = std::ptr::null();
    let mut er: *const c_char = std::ptr::null();

    set_errno(0);
    let rc = unsafe {
        c(
            bc.as_mut_ptr(),
            bin_maxlen,
            b64.as_ptr() as *const c_char,
            b64.len(),
            ign_ptr,
            if want_bin_len { &mut lc } else { std::ptr::null_mut() },
            if want_end { &mut ec } else { std::ptr::null_mut() },
            variant,
        )
    };
    let errc = errno();
    set_errno(0);
    let rr = unsafe {
        r(
            br.as_mut_ptr(),
            bin_maxlen,
            b64.as_ptr() as *const c_char,
            b64.len(),
            ign_ptr,
            if want_bin_len { &mut lr } else { std::ptr::null_mut() },
            if want_end { &mut er } else { std::ptr::null_mut() },
            variant,
        )
    };
    let errr = errno();

    eqi(&format!("base642bin ret [{label}]"), rc, rr);
    if rc != 0 {
        assert_eq!(errc, errr, "base642bin errno [{label}]");
    }
    if want_bin_len {
        assert_eq!(lc, lr, "base642bin *bin_len [{label}]");
    }
    if want_end {
        assert_eq!(
            ec as usize - b64.as_ptr() as usize,
            er as usize - b64.as_ptr() as usize,
            "base642bin *b64_end offset [{label}]"
        );
    }
    let n = if want_bin_len { lc.min(bin_maxlen) } else { bin_maxlen };
    eqb(&format!("base642bin out [{label}]"), &bc[..n], &br[..n]);
    check_pad("base642bin(C)", &bc, bin_maxlen);
    check_pad("base642bin(Rust)", &br, bin_maxlen);
}

#[test]
fn base642bin_round_trip_all_variants() {
    let (enc, _) = both::<EncLen>("sodium_base64_encoded_len");
    let (b2b, _) = both::<Bin2B64>("sodium_bin2base64");
    let (c, r) = both::<B642Bin>("sodium_base642bin");
    let mut rng = Rng::new(0x55);
    for v in VARIANTS {
        for bin_len in 0..=50usize {
            for _ in 0..5 {
                let bin = rng.bytes(bin_len);
                let need = unsafe { enc(bin_len, v) };
                let mut b64 = vec![0u8; need];
                unsafe {
                    b2b(b64.as_mut_ptr() as *mut c_char, need, bin.as_ptr(), bin_len, v);
                }
                let slen = b64.iter().position(|&x| x == 0).unwrap_or(b64.len());
                let s = &b64[..slen];
                for maxlen in [bin_len, bin_len + 1, bin_len + 5] {
                    for (bl, be) in [(true, true), (true, false), (false, true), (false, false)] {
                        b642bin_case(&c, &r, s, maxlen, None, bl, be, v, "roundtrip");
                    }
                    b642bin_case(&c, &r, s, maxlen, Some(b" \n"), true, true, v, "roundtrip+ign");
                }
            }
        }
    }
}

#[test]
fn base642bin_hand_picked_edges() {
    let (c, r) = both::<B642Bin>("sodium_base642bin");
    let inputs: &[&[u8]] = &[
        b"",
        b"=",
        b"==",
        b"===",
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
        b"AAAB",
        b"AB==",
        b"AQ==",
        b"AR==",       // non-zero trailing bits -> rejected
        b"/w==",
        b"_w==",
        b"-w==",
        b"+w==",
        b"//8=",
        b"__8=",
        b"AAAA=",
        b"AAAA==",
        b"A A A A",
        b"AA==AA==",
        b"aGVsbG8=",
        b"aGVsbG8",
        b"aGVsbG8==",
        b"a\nGVsbG8=",
        b"aGVsbG8=\n",
        b"*",
        b"AA*=",
        b"\x00",
        b"A\x00A",
        b"\xff\xff",
        b"AAAAAAAAAAAAAAAA",
        b"AAAAAAAAAAAAAAA=",
    ];
    for v in VARIANTS {
        for inp in inputs {
            for ign in [None, Some(&b" \n"[..]), Some(&b""[..]), Some(&b"*"[..])] {
                for maxlen in [0usize, 1, 2, 3, 4, 12] {
                    for (bl, be) in [(true, true), (true, false), (false, true), (false, false)] {
                        b642bin_case(
                            &c,
                            &r,
                            inp,
                            maxlen,
                            ign,
                            bl,
                            be,
                            v,
                            &format!("v{v}/{:?}/{maxlen}", String::from_utf8_lossy(inp)),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn base642bin_random_fuzz() {
    let (c, r) = both::<B642Bin>("sodium_base642bin");
    let alphabet: &[u8] =
        b"ABCXYZabcxyz0189+/-_= \n\t*\x00\xff";
    let mut rng = Rng::new(0x66);
    for _ in 0..6000 {
        let n = rng.below(20);
        let s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let maxlen = rng.below(14);
        let ign: Option<&[u8]> = match rng.below(4) {
            0 => None,
            1 => Some(b" \n\t"),
            2 => Some(b"*"),
            _ => Some(b""),
        };
        let v = VARIANTS[rng.below(4)];
        b642bin_case(
            &c,
            &r,
            &s,
            maxlen,
            ign,
            rng.below(2) == 0,
            rng.below(2) == 0,
            v,
            "fuzz",
        );
    }
}

#[test]
fn base642bin_invalid_variant_aborts() {
    let (c, r) = both::<B642Bin>("sodium_base642bin");
    for v in [0i32, 2, 4, 6, 8, -1, 1024] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("base642bin variant {v}"),
            move || unsafe {
                let mut o = [0u8; 16];
                let s = b"AAAA\0";
                cc(
                    o.as_mut_ptr(),
                    16,
                    s.as_ptr() as *const c_char,
                    4,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    v,
                );
            },
            move || unsafe {
                let mut o = [0u8; 16];
                let s = b"AAAA\0";
                rr(
                    o.as_mut_ptr(),
                    16,
                    s.as_ptr() as *const c_char,
                    4,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    v,
                );
            },
        );
    }
}

// ------------------------------------------------------------------- ip

fn ip2bin_case(c: &Ip2Bin, r: &Ip2Bin, ip: &[u8], len: usize, label: &str) {
    let mut bc = padded(16);
    let mut br = padded(16);
    set_errno(0);
    let rc = unsafe { c(bc.as_mut_ptr(), ip.as_ptr() as *const c_char, len) };
    set_errno(0);
    let rr = unsafe { r(br.as_mut_ptr(), ip.as_ptr() as *const c_char, len) };
    eqi(&format!("ip2bin ret [{label}]"), rc, rr);
    if rc == 0 {
        eqb(&format!("ip2bin out [{label}]"), &bc[..16], &br[..16]);
    }
    check_pad("ip2bin(C)", &bc, 16);
    check_pad("ip2bin(Rust)", &br, 16);
}

const IPS: &[&str] = &[
    "",
    "0.0.0.0",
    "1.2.3.4",
    "255.255.255.255",
    "256.1.1.1",
    "1.2.3",
    "1.2.3.4.5",
    "1.2.3.",
    ".1.2.3",
    "01.02.03.04",
    "001.002.003.004",
    "0001.2.3.4",
    "1.2.3.4 ",
    " 1.2.3.4",
    "1.2.3.-4",
    "::",
    "::1",
    "1::",
    "::ffff:1.2.3.4",
    "::ffff:0102:0304",
    "0:0:0:0:0:ffff:1.2.3.4",
    "2001:db8::1",
    "2001:0db8:0000:0000:0000:0000:0000:0001",
    "fe80::1%eth0",
    "fe80::1%",
    "fe80::1%eth0!",
    "1.2.3.4%eth0",
    "1:2:3:4:5:6:7:8",
    "1:2:3:4:5:6:7:8:9",
    "1:2:3:4:5:6:7",
    "1:2:3:4:5:6:7::",
    "::1:2:3:4:5:6:7",
    "1::2::3",
    ":1:2:3:4:5:6:7:8",
    "1:2:3:4:5:6:7:8:",
    "12345::1",
    "abcd::ABCD",
    "g::1",
    ":",
    ":::",
    "::.",
    "1:2:3:4:5:6:1.2.3.4",
    "1:2:3:4:5:6:7:1.2.3.4",
    "::1.2.3.4",
    "::ffff:255.255.255.255",
    "0:0:0:0:0:0:0:0",
    "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
    "::%eth0",
    "1.2.3.4\0extra",
    "::1\0extra",
];

#[test]
fn ip2bin_all_cases() {
    let (c, r) = both::<Ip2Bin>("sodium_ip2bin");
    for s in IPS {
        let b = s.as_bytes();
        // exact length, over-length (NUL-terminated scan), under-length
        let mut v = b.to_vec();
        v.push(0);
        v.push(0);
        ip2bin_case(&c, &r, &v, b.len(), &format!("{s:?} exact"));
        ip2bin_case(&c, &r, &v, v.len(), &format!("{s:?} padded"));
        for cut in 0..=b.len().min(6) {
            ip2bin_case(&c, &r, &v, cut, &format!("{s:?} cut{cut}"));
        }
    }
}

#[test]
fn ip2bin_random_fuzz() {
    let (c, r) = both::<Ip2Bin>("sodium_ip2bin");
    let alphabet: &[u8] = b"0123456789abcdefABCDEF:.%-_ \0gz";
    let mut rng = Rng::new(0x77);
    for _ in 0..8000 {
        let n = rng.below(24);
        let s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let mut v = s.clone();
        v.push(0);
        ip2bin_case(&c, &r, &v, s.len(), "fuzz");
    }
}

#[test]
fn bin2ip_all_shapes() {
    let (c, r) = both::<Bin2Ip>("sodium_bin2ip");
    let mut cases: Vec<[u8; 16]> = Vec::new();
    cases.push([0u8; 16]);
    cases.push([0xffu8; 16]);
    // IPv4-mapped
    for last in [[0u8, 0, 0, 0], [1, 2, 3, 4], [255, 255, 255, 255], [10, 0, 0, 1]] {
        let mut a = [0u8; 16];
        a[10] = 0xff;
        a[11] = 0xff;
        a[12..].copy_from_slice(&last);
        cases.push(a);
    }
    // every single-zero-run position/length, to exercise the "::" compression
    for start in 0..8usize {
        for len in 1..=(8 - start) {
            let mut a = [0u8; 16];
            for i in 0..8 {
                let w: u16 = if i >= start && i < start + len { 0 } else { 0x1000 + i as u16 };
                a[i * 2] = (w >> 8) as u8;
                a[i * 2 + 1] = (w & 0xff) as u8;
            }
            cases.push(a);
        }
    }
    // two zero runs of differing length (best-run selection)
    for (a1, l1, a2, l2) in [(0usize, 2usize, 4usize, 3usize), (1, 3, 5, 2), (0, 1, 3, 1)] {
        let mut a = [0u8; 16];
        for i in 0..8 {
            let z = (i >= a1 && i < a1 + l1) || (i >= a2 && i < a2 + l2);
            let w: u16 = if z { 0 } else { 0xabc0 + i as u16 };
            a[i * 2] = (w >> 8) as u8;
            a[i * 2 + 1] = (w & 0xff) as u8;
        }
        cases.push(a);
    }
    let mut rng = Rng::new(0x88);
    for _ in 0..2000 {
        let mut a = [0u8; 16];
        rng.fill(&mut a);
        // sprinkle zero words to hit the compression logic
        for i in 0..8 {
            if rng.below(3) == 0 {
                a[i * 2] = 0;
                a[i * 2 + 1] = 0;
            }
        }
        cases.push(a);
        // also ipv4-mapped-ish
        let mut b = a;
        b[..10].fill(0);
        b[10] = 0xff;
        b[11] = 0xff;
        cases.push(b);
    }

    for bin in &cases {
        for maxlen in [0usize, 1, 2, 3, 4, 5, 8, 10, 16, 20, 39, 40, 45, 46, 64] {
            let mut oc = padded(maxlen);
            let mut or = padded(maxlen);
            let pc = unsafe { c(oc.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr()) };
            let pr = unsafe { r(or.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr()) };
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "bin2ip NULL-ness mismatch for {} maxlen {maxlen}",
                hex(bin)
            );
            if !pc.is_null() {
                assert_eq!(pc as usize, oc.as_ptr() as usize);
                assert_eq!(pr as usize, or.as_ptr() as usize);
                let nc = oc[..maxlen].iter().position(|&x| x == 0).unwrap_or(maxlen);
                let nr = or[..maxlen].iter().position(|&x| x == 0).unwrap_or(maxlen);
                assert_eq!(nc, nr, "bin2ip length mismatch for {}", hex(bin));
                eqb(&format!("bin2ip {} maxlen {maxlen}", hex(bin)), &oc[..nc + 1], &or[..nr + 1]);
            }
            check_pad("bin2ip(C)", &oc, maxlen);
            check_pad("bin2ip(Rust)", &or, maxlen);
        }
    }
}

#[test]
fn ip_round_trip() {
    let (i2b, _) = both::<Ip2Bin>("sodium_ip2bin");
    let (b2i, _) = both::<Bin2Ip>("sodium_bin2ip");
    let (ci2b, ri2b) = both::<Ip2Bin>("sodium_ip2bin");
    let (cb2i, rb2i) = both::<Bin2Ip>("sodium_bin2ip");
    let _ = (&i2b, &b2i);
    let mut rng = Rng::new(0x99);
    for _ in 0..3000 {
        let mut bin = [0u8; 16];
        rng.fill(&mut bin);
        if rng.below(2) == 0 {
            bin[..10].fill(0);
            bin[10] = 0xff;
            bin[11] = 0xff;
        }
        let mut sc = vec![0u8; 64];
        let mut sr = vec![0u8; 64];
        unsafe {
            cb2i(sc.as_mut_ptr() as *mut c_char, 64, bin.as_ptr());
            rb2i(sr.as_mut_ptr() as *mut c_char, 64, bin.as_ptr());
        }
        eqb("bin2ip (round-trip stage 1)", &sc, &sr);
        let n = sc.iter().position(|&x| x == 0).unwrap();
        let mut bc = [0u8; 16];
        let mut br = [0u8; 16];
        let rc = unsafe { ci2b(bc.as_mut_ptr(), sc.as_ptr() as *const c_char, n) };
        let rr = unsafe { ri2b(br.as_mut_ptr(), sr.as_ptr() as *const c_char, n) };
        eqi("ip2bin (round-trip stage 2)", rc, rr);
        eqb("ip2bin (round-trip stage 2 out)", &bc, &br);
        assert_eq!(rc, 0);
        assert_eq!(&bc, &bin, "C round trip lost information for {}", hex(&bin));
    }
}

/// config 1.75: `sodium_base64_encoded_len()` must agree with the
/// `sodium_base64_ENCODED_LEN` header macro (reimplemented here verbatim) for
/// every `(bin_len, variant)` pair where the function does not abort.
#[test]
fn base64_encoded_len_agrees_with_the_header_macro() {
    fn macro_encoded_len(bin_len: usize, variant: c_int) -> usize {
        if bin_len / 3 > (usize::MAX - 5) / 4 {
            return usize::MAX;
        }
        let rem = bin_len - (bin_len / 3) * 3;
        let has_rem = ((rem | (rem >> 1)) & 1) as usize;
        let v = variant as u32;
        let nopad = (0u32.wrapping_sub((v & 2) >> 1)) as usize;
        (bin_len / 3) * 4 + has_rem * (4usize.wrapping_sub(nopad & (3 - rem))) + 1
    }
    let (c, r) = both::<EncLen>("sodium_base64_encoded_len");
    let mut rng = Rng::new(0xB64);
    let mut lens: Vec<usize> = (0..400).collect();
    for _ in 0..200 {
        lens.push(rng.below(1 << 30));
    }
    for v in VARIANTS {
        for &bin_len in &lens {
            let m = macro_encoded_len(bin_len, v);
            unsafe {
                assert_eq!(c(bin_len, v), m, "C encoded_len({bin_len},{v}) vs macro");
                assert_eq!(r(bin_len, v), m, "Rust encoded_len({bin_len},{v}) vs macro");
            }
        }
    }
}
