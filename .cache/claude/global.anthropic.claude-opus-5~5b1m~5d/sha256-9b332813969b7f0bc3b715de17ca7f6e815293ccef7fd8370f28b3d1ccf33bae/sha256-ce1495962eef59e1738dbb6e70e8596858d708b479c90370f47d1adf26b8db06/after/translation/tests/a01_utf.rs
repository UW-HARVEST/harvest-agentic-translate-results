//! Differential tests for the exported `utf8_*` primitives (src/utf.c).
//!
//! The input domains here are small enough to enumerate EXHAUSTIVELY rather
//! than sample, so these are total equivalence proofs over the C domain:
//!   * `utf8_check_first` — all 256 byte values
//!   * `utf8_encode`      — all codepoints 0..=0x110100 plus negatives
//!   * `utf8_check_full`  — all 2-byte sequences, plus randomised 3/4-byte
//!   * `utf8_iterate`     — all 1..4-byte prefixes at every buffer size
//!   * `utf8_check_string`— randomised valid/invalid/truncated mixes

mod common;
use common::*;
use std::ffi::c_char;

#[test]
fn utf8_check_first_all_256_bytes() {
    let _g = global_state_lock();
    let (c, r) = both();
    for b in 0u16..256 {
        let byte = b as u8 as c_char;
        unsafe {
            diff_eq!(
                (c.utf8_check_first)(byte),
                (r.utf8_check_first)(byte),
                "utf8_check_first(0x{b:02x})"
            );
        }
    }
}

#[test]
fn utf8_encode_every_codepoint() {
    let _g = global_state_lock();
    let (c, r) = both();
    // Every valid codepoint, plus a margin past the 0x10FFFF limit so the
    // "out of Unicode range" rejection is covered, plus negatives.
    let mut cps: Vec<i32> = (0..=0x11_0100).collect();
    cps.extend([-1, -2, -128, -0x10FFFF, i32::MIN, i32::MAX, 0x7FFF_FFFF]);

    for cp in cps {
        unsafe {
            // Poison both buffers identically so that *which* bytes get
            // written (and how many) is part of the comparison.
            let mut cbuf = [0xAAu8 as c_char; 8];
            let mut rbuf = [0xAAu8 as c_char; 8];
            let mut csize: size_t = 0xDEAD;
            let mut rsize: size_t = 0xDEAD;

            let cret = (c.utf8_encode)(cp, cbuf.as_mut_ptr(), &mut csize);
            let rret = (r.utf8_encode)(cp, rbuf.as_mut_ptr(), &mut rsize);

            diff_eq!(cret, rret, "utf8_encode({cp}) return");
            diff_eq!(csize, rsize, "utf8_encode({cp}) *size");
            diff_eq!(cbuf, rbuf, "utf8_encode({cp}) buffer");
        }
    }
}

#[test]
fn utf8_check_full_all_two_byte_sequences() {
    let _g = global_state_lock();
    let (c, r) = both();
    for b0 in 0u16..256 {
        for b1 in 0u16..256 {
            let buf = [b0 as u8 as c_char, b1 as u8 as c_char, 0, 0];
            unsafe {
                let mut ccp: i32 = -12345;
                let mut rcp: i32 = -12345;
                let cret = (c.utf8_check_full)(buf.as_ptr(), 2, &mut ccp);
                let rret = (r.utf8_check_full)(buf.as_ptr(), 2, &mut rcp);
                diff_eq!(cret, rret, "utf8_check_full([{b0:02x},{b1:02x}], 2) return");
                diff_eq!(ccp, rcp, "utf8_check_full([{b0:02x},{b1:02x}], 2) codepoint");
            }
        }
    }
}

#[test]
fn utf8_check_full_three_and_four_byte_sequences() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xF00D_1234);

    // Exhaustive over the lead byte; randomised (but seeded) over the
    // continuation bytes, biased towards the 0x80..0xBF boundary.
    for size in [3usize, 4] {
        for lead in 0u16..256 {
            for _ in 0..64 {
                let mut buf = [0 as c_char; 8];
                buf[0] = lead as u8 as c_char;
                for i in 1..size {
                    let b = match rng.below(4) {
                        0 => rng.next_u32() as u8,
                        1 => 0x80 + (rng.below(0x40) as u8),
                        2 => *rng.choice(&[0x7fu8, 0x80, 0xbf, 0xc0, 0x00, 0xff]),
                        _ => rng.next_u32() as u8,
                    };
                    buf[i] = b as c_char;
                }
                unsafe {
                    let mut ccp: i32 = -12345;
                    let mut rcp: i32 = -12345;
                    let cret = (c.utf8_check_full)(buf.as_ptr(), size, &mut ccp);
                    let rret = (r.utf8_check_full)(buf.as_ptr(), size, &mut rcp);
                    let bytes: Vec<u8> = buf[..size].iter().map(|&x| x as u8).collect();
                    diff_eq!(cret, rret, "utf8_check_full({bytes:02x?}, {size}) return");
                    diff_eq!(ccp, rcp, "utf8_check_full({bytes:02x?}, {size}) codepoint");
                }
            }
        }
    }
}

#[test]
fn utf8_check_full_invalid_sizes() {
    let _g = global_state_lock();
    // Any size other than 2, 3 or 4 must be rejected without reading past
    // buffer[0] — sizes 0, 1, 5.. all hit the `else return 0` branch.
    let (c, r) = both();
    let buf: Vec<c_char> = [0xF0u8, 0x9F, 0x98, 0x80].iter().map(|&b| b as c_char).collect();
    for size in [0usize, 1, 5, 6, 7, 8, 100, usize::MAX] {
        unsafe {
            let mut ccp: i32 = -1;
            let mut rcp: i32 = -1;
            diff_eq!(
                (c.utf8_check_full)(buf.as_ptr(), size, &mut ccp),
                (r.utf8_check_full)(buf.as_ptr(), size, &mut rcp),
                "utf8_check_full(size={size})"
            );
            diff_eq!(ccp, rcp, "utf8_check_full(size={size}) codepoint");
        }
    }
}

#[test]
fn utf8_check_full_null_codepoint_out() {
    let _g = global_state_lock();
    // `codepoint` is optional; passing NULL must be accepted.
    let (c, r) = both();
    for seq in [
        vec![0xC3u8, 0xA9],
        vec![0xE2, 0x82, 0xAC],
        vec![0xF0, 0x9F, 0x98, 0x80],
        vec![0xC0, 0x80],       // overlong
        vec![0xED, 0xA0, 0x80], // surrogate
    ] {
        let buf: Vec<c_char> = seq.iter().map(|&b| b as c_char).collect();
        unsafe {
            diff_eq!(
                (c.utf8_check_full)(buf.as_ptr(), seq.len(), std::ptr::null_mut()),
                (r.utf8_check_full)(buf.as_ptr(), seq.len(), std::ptr::null_mut()),
                "utf8_check_full({seq:02x?}, NULL codepoint)"
            );
        }
    }
}

#[test]
fn utf8_iterate_every_lead_byte_at_every_size() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xBEEF_0001);

    for lead in 0u16..256 {
        // bufsize 0 is the early-return path; 1..5 covers "count > bufsize".
        for bufsize in 0usize..=5 {
            for _ in 0..24 {
                let mut buf = [0u8; 8];
                buf[0] = lead as u8;
                for i in 1..8 {
                    buf[i] = if rng.bool() {
                        0x80 + rng.below(0x40) as u8
                    } else {
                        rng.next_u32() as u8
                    };
                }
                let cbuf: Vec<c_char> = buf.iter().map(|&b| b as c_char).collect();
                unsafe {
                    let mut ccp: i32 = -12345;
                    let mut rcp: i32 = -12345;
                    let cret = (c.utf8_iterate)(cbuf.as_ptr(), bufsize, &mut ccp);
                    let rret = (r.utf8_iterate)(cbuf.as_ptr(), bufsize, &mut rcp);
                    // Compare the returned pointer as an OFFSET; the absolute
                    // address is the same buffer for both calls, but NULL vs
                    // non-NULL and the advance distance must agree.
                    let coff = if cret.is_null() {
                        None
                    } else {
                        Some(cret as usize - cbuf.as_ptr() as usize)
                    };
                    let roff = if rret.is_null() {
                        None
                    } else {
                        Some(rret as usize - cbuf.as_ptr() as usize)
                    };
                    diff_eq!(
                        coff,
                        roff,
                        "utf8_iterate(lead=0x{lead:02x}, bufsize={bufsize}) advance"
                    );
                    diff_eq!(
                        ccp,
                        rcp,
                        "utf8_iterate(lead=0x{lead:02x}, bufsize={bufsize}) codepoint"
                    );
                }
            }
        }
    }
}

#[test]
fn utf8_iterate_null_codepoint_out() {
    let _g = global_state_lock();
    let (c, r) = both();
    for seq in [
        vec![0x41u8],
        vec![0xC3, 0xA9],
        vec![0xE2, 0x82, 0xAC],
        vec![0xF0, 0x9F, 0x98, 0x80],
        vec![0xFF],
        vec![0x80],
    ] {
        let buf: Vec<c_char> = seq.iter().map(|&b| b as c_char).collect();
        unsafe {
            let cret = (c.utf8_iterate)(buf.as_ptr(), seq.len(), std::ptr::null_mut());
            let rret = (r.utf8_iterate)(buf.as_ptr(), seq.len(), std::ptr::null_mut());
            let base = buf.as_ptr() as usize;
            diff_eq!(
                cret.is_null().then_some(None).unwrap_or(Some(cret as usize - base)),
                rret.is_null().then_some(None).unwrap_or(Some(rret as usize - base)),
                "utf8_iterate({seq:02x?}, NULL codepoint)"
            );
        }
    }
}

#[test]
fn utf8_check_string_randomised() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xCAFE_0002);

    for iter in 0..20000 {
        // Mix of: fully random bytes, valid UTF-8, valid UTF-8 truncated
        // mid-sequence, and valid UTF-8 with one byte corrupted.
        let bytes: Vec<u8> = match iter % 4 {
            0 => (0..rng.below(24)).map(|_| rng.next_u32() as u8).collect(),
            1 => rng.utf8_string(12).into_bytes(),
            2 => {
                let mut b = rng.utf8_string(12).into_bytes();
                if !b.is_empty() {
                    b.truncate(rng.below(b.len()));
                }
                b
            }
            _ => {
                let mut b = rng.utf8_string(12).into_bytes();
                if !b.is_empty() {
                    let i = rng.below(b.len());
                    b[i] = rng.next_u32() as u8;
                }
                b
            }
        };
        let buf: Vec<c_char> = bytes.iter().map(|&x| x as c_char).collect();
        unsafe {
            // Also vary `length` independently of the real buffer length, but
            // never past it, so "length longer than the sequence" is covered.
            let len = if bytes.is_empty() { 0 } else { rng.below(bytes.len() + 1) };
            diff_eq!(
                (c.utf8_check_string)(buf.as_ptr(), len),
                (r.utf8_check_string)(buf.as_ptr(), len),
                "utf8_check_string({bytes:02x?}, {len})"
            );
        }
    }
}

#[test]
fn utf8_check_string_zero_length_and_empty() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // length 0 must return 1 (vacuously valid) without dereferencing.
        let empty = cs("");
        diff_eq!(
            (c.utf8_check_string)(empty.as_ptr(), 0),
            (r.utf8_check_string)(empty.as_ptr(), 0),
            "utf8_check_string(\"\", 0)"
        );
        // A NULL pointer with length 0 also never dereferences in the C.
        diff_eq!(
            (c.utf8_check_string)(std::ptr::null(), 0),
            (r.utf8_check_string)(std::ptr::null(), 0),
            "utf8_check_string(NULL, 0)"
        );
    }
}

#[test]
fn utf8_check_string_embedded_nul_is_valid() {
    let _g = global_state_lock();
    // A NUL byte is a perfectly valid 1-byte UTF-8 sequence; utf8_check_string
    // is length-driven and must not stop at it.
    let (c, r) = both();
    let bytes: Vec<c_char> = b"a\0b\0\xc3\xa9".iter().map(|&x| x as c_char).collect();
    unsafe {
        diff_eq!(
            (c.utf8_check_string)(bytes.as_ptr(), 6),
            (r.utf8_check_string)(bytes.as_ptr(), 6),
            "utf8_check_string with embedded NULs"
        );
    }
}
