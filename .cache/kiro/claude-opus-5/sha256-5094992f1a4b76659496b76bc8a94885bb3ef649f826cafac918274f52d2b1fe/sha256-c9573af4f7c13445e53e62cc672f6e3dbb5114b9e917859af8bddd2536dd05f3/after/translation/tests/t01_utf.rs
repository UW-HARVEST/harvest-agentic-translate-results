//! Phase B/C — utf.c: `utf8_encode`, `utf8_check_first`, `utf8_check_full`,
//! `utf8_iterate`, `utf8_check_string`.
//! CONFIGS rows 1-5 · ERRORS rows 97-113.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

/* ---------------- CONFIGS 1 / ERRORS 99,100,101 ---------------- */

#[test]
fn utf8_check_first_all_256_bytes() {
    unsafe {
        for b in 0u16..256 {
            let byte = b as u8 as c_char;
            let cv = (c().utf8_check_first)(byte);
            let rv = (r().utf8_check_first)(byte);
            assert_eq!(cv, rv, "utf8_check_first(0x{b:02x})");
        }
        // ERRORS 99: continuation bytes 0x80..=0xBF => 0
        for b in 0x80u16..=0xBF {
            assert_eq!((c().utf8_check_first)(b as u8 as c_char), 0);
        }
        // ERRORS 100: 0xC0 / 0xC1 => 0
        assert_eq!((c().utf8_check_first)(0xC0u8 as c_char), 0);
        assert_eq!((c().utf8_check_first)(0xC1u8 as c_char), 0);
        // ERRORS 101: >= 0xF5 => 0
        for b in 0xF5u16..256 {
            assert_eq!((c().utf8_check_first)(b as u8 as c_char), 0);
        }
    }
}

/* ---------------- CONFIGS 3 / ERRORS 97,98,251 ---------------- */

#[test]
fn utf8_encode_boundaries_and_random() {
    unsafe {
        let mut cps: Vec<i32> = vec![
            i32::MIN,
            -1000,
            -1,
            0,
            1,
            0x7F,
            0x80,
            0x7FF,
            0x800,
            0xD7FF,
            0xD800,
            0xDFFF,
            0xE000,
            0xFFFF,
            0x1_0000,
            0x10_FFFF,
            0x11_0000,
            0x20_0000,
            i32::MAX,
        ];
        let mut rng = Rng::new(0x0000_08E1);
        for _ in 0..4000 {
            cps.push(rng.next_u32() as i32);
            cps.push((rng.next_u32() % 0x11_0010) as i32);
        }
        for cp in cps {
            let mut cbuf = [0i8; 8];
            let mut rbuf = [0i8; 8];
            let mut csz: usize = 0xdead;
            let mut rsz: usize = 0xdead;
            let cr = (c().utf8_encode)(cp, cbuf.as_mut_ptr(), &mut csz);
            let rr = (r().utf8_encode)(cp, rbuf.as_mut_ptr(), &mut rsz);
            assert_eq!(cr, rr, "utf8_encode({cp}) return");
            if cr == 0 {
                assert_eq!(csz, rsz, "utf8_encode({cp}) size");
                assert_eq!(&cbuf[..csz], &rbuf[..rsz], "utf8_encode({cp}) bytes");
            }
            assert_eq!(csz, rsz, "utf8_encode({cp}) size out-param");
        }
    }
}

/* ---------------- CONFIGS 2 / ERRORS 102-106 ---------------- */

#[test]
fn utf8_check_full_sizes_and_random() {
    unsafe {
        let mut rng = Rng::new(0xF0112233);
        // hand-picked: overlong, surrogate, out-of-range, bad continuation
        let cases: Vec<(Vec<u8>, usize)> = vec![
            (vec![0xC2, 0x80], 2),
            (vec![0xC0, 0x80], 2),          // overlong (value < 0x80)
            (vec![0xC2, 0x7F], 2),          // bad continuation
            (vec![0xC2, 0xC0], 2),          // bad continuation
            (vec![0xE0, 0xA0, 0x80], 3),
            (vec![0xE0, 0x80, 0x80], 3),    // overlong
            (vec![0xED, 0xA0, 0x80], 3),    // surrogate D800
            (vec![0xED, 0xBF, 0xBF], 3),    // surrogate DFFF
            (vec![0xEE, 0x80, 0x80], 3),
            (vec![0xF0, 0x90, 0x80, 0x80], 4),
            (vec![0xF0, 0x80, 0x80, 0x80], 4), // overlong
            (vec![0xF4, 0x8F, 0xBF, 0xBF], 4),
            (vec![0xF4, 0x90, 0x80, 0x80], 4), // > 0x10FFFF
            (vec![0xF7, 0xBF, 0xBF, 0xBF], 4), // > 0x10FFFF
            (vec![0x41], 1),                   // size 1 -> 0
            (vec![0x41, 0x42, 0x43, 0x44, 0x45], 5), // size 5 -> 0
            (vec![0x41], 0),                   // size 0 -> 0
        ];
        for (bytes, size) in cases {
            check_full_one(&bytes, size);
        }
        for _ in 0..6000 {
            let n = 1 + rng.below(5);
            let bytes = rng.bytes(n.max(4));
            let size = rng.below(6);
            check_full_one(&bytes, size);
        }
    }
}

unsafe fn check_full_one(bytes: &[u8], size: usize) {
    unsafe {
        // Ensure the buffer is at least `size` long so the C read is in bounds.
        let mut buf: Vec<i8> = bytes.iter().map(|&b| b as i8).collect();
        while buf.len() < size.max(1) + 1 {
            buf.push(0);
        }
        let mut ccp: i32 = -12345;
        let mut rcp: i32 = -12345;
        let cv = (c().utf8_check_full)(buf.as_ptr(), size, &mut ccp);
        let rv = (r().utf8_check_full)(buf.as_ptr(), size, &mut rcp);
        assert_eq!(cv, rv, "utf8_check_full({bytes:02x?}, {size}) ret");
        assert_eq!(
            ccp, rcp,
            "utf8_check_full({bytes:02x?}, {size}) codepoint out-param"
        );
        // NULL codepoint out-pointer must behave the same
        let cv2 = (c().utf8_check_full)(buf.as_ptr(), size, std::ptr::null_mut());
        let rv2 = (r().utf8_check_full)(buf.as_ptr(), size, std::ptr::null_mut());
        assert_eq!(cv2, rv2, "utf8_check_full NULL cp ({bytes:02x?}, {size})");
        assert_eq!(cv, cv2);
    }
}

/* ---------------- CONFIGS 4 / ERRORS 107-110 ---------------- */

#[test]
fn utf8_iterate_random_and_boundaries() {
    unsafe {
        let mut rng = Rng::new(0x1727_3747);
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![0x41],
            vec![0x00],
            vec![0x80],
            vec![0xC2],             // truncated 2-byte
            vec![0xC2, 0x80],
            vec![0xE2, 0x82],       // truncated 3-byte
            vec![0xE2, 0x82, 0xAC],
            vec![0xF0, 0x9F],       // truncated 4-byte
            vec![0xF0, 0x9F, 0x92, 0xA9],
            vec![0xF5, 0x80, 0x80, 0x80],
            vec![0xED, 0xA0, 0x80],
        ];
        for _ in 0..6000 {
            let n = 1 + rng.below(6); cases.push(rng.bytes(n));
        }
        for bytes in cases {
            for &bufsize in &[0usize, 1, 2, 3, 4, bytes.len()] {
                if bufsize > bytes.len() {
                    continue;
                }
                let buf: Vec<i8> = bytes.iter().map(|&b| b as i8).collect();
                let base = if buf.is_empty() {
                    // still need a valid pointer for bufsize == 0
                    std::ptr::NonNull::dangling().as_ptr()
                } else {
                    buf.as_ptr() as *mut i8
                };
                let mut ccp: i32 = -999;
                let mut rcp: i32 = -999;
                let cp_ret = (c().utf8_iterate)(base, bufsize, &mut ccp);
                let rp_ret = (r().utf8_iterate)(base, bufsize, &mut rcp);
                let coff = if cp_ret.is_null() {
                    None
                } else {
                    Some(cp_ret as usize - base as usize)
                };
                let roff = if rp_ret.is_null() {
                    None
                } else {
                    Some(rp_ret as usize - base as usize)
                };
                assert_eq!(
                    coff, roff,
                    "utf8_iterate({bytes:02x?}, {bufsize}) returned offset"
                );
                assert_eq!(
                    ccp, rcp,
                    "utf8_iterate({bytes:02x?}, {bufsize}) codepoint"
                );
                // NULL codepoint pointer
                let c2 = (c().utf8_iterate)(base, bufsize, std::ptr::null_mut());
                let r2 = (r().utf8_iterate)(base, bufsize, std::ptr::null_mut());
                assert_eq!(c2.is_null(), r2.is_null());
                if !c2.is_null() {
                    assert_eq!(c2 as usize - base as usize, r2 as usize - base as usize);
                }
            }
        }
    }
}

/* ---------------- CONFIGS 5 / ERRORS 111-113 ---------------- */

#[test]
fn utf8_check_string_random_valid_and_invalid() {
    unsafe {
        let mut rng = Rng::new(0xCAFE_0001);
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"hello".to_vec(),
            vec![0x00],
            b"a\0b".to_vec(),
            "héllo".as_bytes().to_vec(),
            "€".as_bytes().to_vec(),
            "😀".as_bytes().to_vec(),
            vec![0xC2],
            vec![0xE2, 0x82],
            vec![0xF0, 0x9F, 0x92],
            vec![0xED, 0xA0, 0x80],
            vec![0xFF, 0xFE],
            vec![0x80],
        ];
        for _ in 0..3000 {
            cases.push(rng.utf8(10).into_bytes());
            let n = rng.below(20); cases.push(rng.bytes(n));
            // valid prefix + garbage tail
            let mut v = rng.utf8(5).into_bytes();
            let n = 1 + rng.below(3); v.extend(rng.bytes(n));
            cases.push(v);
        }
        for bytes in cases {
            for &len in &[bytes.len(), bytes.len().saturating_sub(1), 0] {
                if len > bytes.len() {
                    continue;
                }
                let buf: Vec<i8> = bytes.iter().map(|&b| b as i8).collect();
                let p = if buf.is_empty() {
                    std::ptr::NonNull::dangling().as_ptr()
                } else {
                    buf.as_ptr() as *mut i8
                };
                let cv: c_int = (c().utf8_check_string)(p, len);
                let rv: c_int = (r().utf8_check_string)(p, len);
                assert_eq!(cv, rv, "utf8_check_string({bytes:02x?}, {len})");
            }
        }
    }
}
