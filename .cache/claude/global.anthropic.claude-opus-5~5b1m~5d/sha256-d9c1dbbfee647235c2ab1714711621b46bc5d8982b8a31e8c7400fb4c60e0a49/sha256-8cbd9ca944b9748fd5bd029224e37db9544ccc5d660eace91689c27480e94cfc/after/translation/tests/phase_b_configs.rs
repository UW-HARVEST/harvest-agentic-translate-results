//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH `libdriver.so` files (C and Rust) with `libloading`
//! and compares their `custom_strdup` output byte-for-byte. Randomized rows use
//! a fixed seed so failures reproduce exactly.

mod common;

use common::{CStrBuf, Rng, assert_same, libs};
use std::ffi::c_char;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Row 1 — empty string `""` (len 0, the zero boundary).
#[test]
fn row01_empty_string() {
    let buf = CStrBuf::new(b"");
    let out = assert_same(&buf, "row01 empty");
    assert_eq!(out, vec![0u8], "empty string must copy to exactly one NUL byte");
}

/// Row 2 — single byte, swept over ALL 255 non-NUL byte values.
#[test]
fn row02_single_byte_all_values() {
    for b in 1u8..=255 {
        let buf = CStrBuf::new(&[b]);
        let out = assert_same(&buf, &format!("row02 byte={b:#04x}"));
        assert_eq!(out, vec![b, 0]);
    }
}

/// Row 3 — short random ASCII-printable strings, lengths 2..=16.
#[test]
fn row03_short_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..4000 {
        let len = rng.range(2, 16);
        let body: Vec<u8> = (0..len).map(|_| rng.ascii_printable()).collect();
        let buf = CStrBuf::new(&body);
        assert_same(&buf, &format!("row03 i={i} len={len}"));
    }
}

/// Row 4 — short random BINARY strings (bytes 0x01..=0xFF), lengths 2..=16.
#[test]
fn row04_short_binary() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..4000 {
        let len = rng.range(2, 16);
        let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        let buf = CStrBuf::new(&body);
        assert_same(&buf, &format!("row04 i={i} len={len}"));
    }
}

/// Row 5 — medium random binary strings, lengths 17..=1024.
#[test]
fn row05_medium_binary() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..2000 {
        let len = rng.range(17, 1024);
        let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        let buf = CStrBuf::new(&body);
        assert_same(&buf, &format!("row05 i={i} len={len}"));
    }
}

/// Row 6 — page-boundary lengths (4095/4096/4097/8191/8192/8193), where
/// `strlen`/`memcpy` vectorization and page crossing behave differently.
#[test]
fn row06_page_boundaries() {
    let mut rng = Rng::new(SEED ^ 6);
    for &len in &[4095usize, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537] {
        for rep in 0..3 {
            let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
            let buf = CStrBuf::new(&body);
            let out = assert_same(&buf, &format!("row06 len={len} rep={rep}"));
            assert_eq!(out.len(), len + 1);
        }
    }
}

/// Row 7 — large strings: 64 KiB, 1 MiB, 4 MiB.
#[test]
fn row07_large_strings() {
    let mut rng = Rng::new(SEED ^ 7);
    for &len in &[64 * 1024usize, 1024 * 1024, 4 * 1024 * 1024] {
        let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        let buf = CStrBuf::new(&body);
        let out = assert_same(&buf, &format!("row07 len={len}"));
        assert_eq!(out.len(), len + 1);
        assert_eq!(&out[..len], &body[..]);
    }
}

/// Row 8 — embedded NUL bytes: the copy must stop at the FIRST NUL, so only
/// `strlen+1` bytes are compared and the tail must not be copied.
#[test]
fn row08_embedded_nuls() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);

    // Hand-picked shapes first.
    for raw in [
        &b"ab\0cd\0"[..],
        &b"\0trailing-garbage\0"[..],
        &b"x\0\0\0\0"[..],
        &b"hello\0world\0more\0"[..],
    ] {
        let mut bytes = raw.to_vec();
        if *bytes.last().unwrap() != 0 {
            bytes.push(0);
        }
        let buf = CStrBuf { bytes };
        let expect = buf.expected_copy().to_vec();
        let out = assert_same(&buf, &format!("row08 raw={raw:x?}"));
        assert_eq!(out, expect, "row08: copy must stop at the first NUL");
    }

    // Randomized: a NUL planted at a random position inside a random body.
    for i in 0..2000 {
        let len = rng.range(1, 200);
        let mut body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        let nul_at = rng.range(0, len - 1);
        body[nul_at] = 0;
        let mut bytes = body.clone();
        bytes.push(0);
        let buf = CStrBuf { bytes };
        let out = assert_same(&buf, &format!("row08 rand i={i} len={len} nul_at={nul_at}"));
        assert_eq!(out.len(), nul_at + 1, "row08: length must be strlen+1 = {}", nul_at + 1);
        let _ = l; // keep the libs handle alive/used
    }
}

/// Row 9 — invalid-UTF-8 byte sequences. The Rust translation must never
/// interpret the input as UTF-8.
#[test]
fn row09_invalid_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0xFF],
        vec![0xFF, 0xFF, 0xFF, 0xFF],
        vec![0x80],                          // lone continuation byte
        vec![0xC3],                          // truncated 2-byte sequence
        vec![0xE2, 0x82],                    // truncated 3-byte sequence
        vec![0xF0, 0x9F, 0x92],              // truncated 4-byte sequence
        vec![0xED, 0xA0, 0x80],              // UTF-16 surrogate encoded as UTF-8
        vec![0xC0, 0xAF],                    // overlong encoding of '/'
        vec![0xF5, 0x80, 0x80, 0x80],        // beyond U+10FFFF
        (0x80u8..=0xFF).collect(),           // every high byte
        (1u8..=0x7F).collect(),              // every low non-NUL byte
    ];
    for (i, body) in cases.iter().enumerate() {
        let buf = CStrBuf::new(body);
        let out = assert_same(&buf, &format!("row09 case={i}"));
        assert_eq!(&out[..body.len()], &body[..]);
    }

    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..3000 {
        let len = rng.range(1, 64);
        // Bias heavily toward high bytes so most draws are invalid UTF-8.
        let body: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 & 0x7F)).collect();
        let buf = CStrBuf::new(&body);
        assert_same(&buf, &format!("row09 rand i={i} len={len}"));
    }
}

/// Row 10 — unaligned `const char *`: the pointer is offset 1..=7 bytes into an
/// allocation, which changes the `strlen`/`memcpy` alignment path.
#[test]
fn row10_unaligned_input() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..3000 {
        let off = rng.range(1, 15);
        let len = rng.range(0, 300);
        // Backing buffer: `off` junk bytes, then the body, then the terminator.
        let mut backing: Vec<u8> = (0..off).map(|_| rng.nonnul_byte()).collect();
        let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        backing.extend_from_slice(&body);
        backing.push(0);

        let p = unsafe { backing.as_ptr().add(off) } as *const c_char;
        let cp = unsafe { (l.c)(p) };
        let rp = unsafe { (l.rs)(p) };
        assert!(!cp.is_null() && !rp.is_null(), "row10 i={i}: unexpected NULL");
        let n = len + 1;
        let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, n) }.to_vec();
        let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, n) }.to_vec();
        assert_eq!(cout, rout, "row10 i={i} off={off} len={len}: C/Rust differ");
        assert_eq!(&cout[..len], &body[..], "row10 i={i}: wrong bytes copied");
        assert_eq!(cout[len], 0, "row10 i={i}: missing terminator");
        unsafe {
            common::free(cp as *mut _);
            common::free(rp as *mut _);
        }
    }
}

/// Row 11 — repeated calls: every returned pointer is a distinct fresh
/// allocation with identical contents (no caching, no aliasing, no reuse of a
/// live buffer).
#[test]
fn row11_repeated_calls_distinct_buffers() {
    let l = libs();
    let buf = CStrBuf::new(b"repeatable-input-0123456789");
    let n = buf.expected_copy().len();

    let mut held: Vec<*mut c_char> = Vec::new();
    for _ in 0..200 {
        let cp = unsafe { (l.c)(buf.ptr()) };
        let rp = unsafe { (l.rs)(buf.ptr()) };
        assert!(!cp.is_null() && !rp.is_null());
        let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, n) };
        let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, n) };
        assert_eq!(cout, rout, "row11: C/Rust differ across repeated calls");
        assert_eq!(cout, buf.expected_copy());
        assert_ne!(cp as *const c_char, buf.ptr());
        assert_ne!(rp as *const c_char, buf.ptr());
        held.push(cp);
        held.push(rp);
    }
    // All 400 live pointers must be pairwise distinct.
    let mut sorted: Vec<usize> = held.iter().map(|p| *p as usize).collect();
    sorted.sort_unstable();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(before, sorted.len(), "row11: a live buffer was handed out twice");

    for p in held {
        unsafe { common::free(p as *mut _) };
    }
}

/// Row 12 — allocator-ownership parity under a randomized mix of shapes: every
/// returned pointer is `free()`d through libc immediately (already asserted by
/// `assert_same`, exercised here at volume to surface allocator mismatches).
#[test]
fn row12_free_parity_mixed_shapes() {
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..5000 {
        let len = match rng.range(0, 3) {
            0 => 0,
            1 => rng.range(1, 32),
            2 => rng.range(33, 4096),
            _ => rng.range(4097, 20000),
        };
        let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        let buf = CStrBuf::new(&body);
        assert_same(&buf, &format!("row12 i={i} len={len}"));
    }
}

/// Row 13 — randomized fuzz sweep over the full cross-product of the axes
/// (length class x byte content class x alignment x embedded NUL), fixed seed.
#[test]
fn row13_fuzz_cross_product() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..20_000 {
        let len = match rng.range(0, 5) {
            0 => 0,
            1 => 1,
            2 => rng.range(2, 8),
            3 => rng.range(9, 64),
            4 => rng.range(65, 512),
            _ => rng.range(513, 5000),
        };
        let content_class = rng.range(0, 3);
        let mut body: Vec<u8> = (0..len)
            .map(|_| match content_class {
                0 => rng.ascii_printable(),
                1 => rng.nonnul_byte(),
                2 => 0x80 | (rng.next_u64() as u8 & 0x7F),
                _ => 0xFF,
            })
            .collect();
        // 1-in-4 chance of an embedded NUL somewhere in the body.
        if len > 0 && rng.range(0, 3) == 0 {
            let at = rng.range(0, len - 1);
            body[at] = 0;
        }
        let off = rng.range(0, 15);

        let mut backing: Vec<u8> = (0..off).map(|_| rng.nonnul_byte()).collect();
        backing.extend_from_slice(&body);
        backing.push(0);

        let p = unsafe { backing.as_ptr().add(off) } as *const c_char;
        let expect_len = body.iter().position(|&b| b == 0).unwrap_or(len) + 1;

        let cp = unsafe { (l.c)(p) };
        let rp = unsafe { (l.rs)(p) };
        assert!(!cp.is_null() && !rp.is_null(), "row13 i={i}: unexpected NULL");
        let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, expect_len) }.to_vec();
        let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, expect_len) }.to_vec();
        assert_eq!(
            cout, rout,
            "row13 i={i} len={len} off={off} class={content_class}: C/Rust differ"
        );
        assert_eq!(cout.len(), expect_len);
        assert_eq!(*cout.last().unwrap(), 0);
        unsafe {
            common::free(cp as *mut _);
            common::free(rp as *mut _);
        }
    }
}
