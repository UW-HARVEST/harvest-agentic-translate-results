//! Differential tests for `c_src/src/utf.c`.
//!
//! Covers CONFIGS.md rows 1-5 and ERRORS.md rows 212-228. Every call goes
//! through the two `dlopen`ed shared objects (`d.c.*` / `d.rs.*`); no Rust
//! function is ever called directly.
mod common;
use common::*;

use std::ffi::c_char;
use std::ptr;

/// Sentinel used to fill `utf8_encode`'s output buffer, so that any byte the
/// implementation fails to write (or writes when it should not) is visible.
const PAD: u8 = 0xAA;
/// Sentinel for the `*size` out-param of `utf8_encode` ("not written").
const SENT_SIZE: usize = usize::MAX;
/// Sentinel for the `*codepoint` out-param ("not written").
const SENT_CP: i32 = -12345;

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 3);
    for (i, x) in b.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:02X}", x));
    }
    s
}

// ---------------------------------------------------------------------------
// comparators
// ---------------------------------------------------------------------------

/// `utf8_encode`: return code, whole 8-byte buffer, `*size`.
fn cmp_encode(d: &Duo, cp: i32) {
    let mut cb = [PAD as c_char; 8];
    let mut rb = [PAD as c_char; 8];
    let mut csz: usize = SENT_SIZE;
    let mut rsz: usize = SENT_SIZE;
    let cret = unsafe { (d.c.utf8_encode)(cp, cb.as_mut_ptr(), &mut csz) };
    let rret = unsafe { (d.rs.utf8_encode)(cp, rb.as_mut_ptr(), &mut rsz) };

    let tag = format!("utf8_encode(cp={} / {:#010x})", cp, cp as u32);
    eq(&format!("{} return", tag), cret, rret);
    let cbv: Vec<u8> = cb.iter().map(|&x| x as u8).collect();
    let rbv: Vec<u8> = rb.iter().map(|&x| x as u8).collect();
    eq(&format!("{} buffer", tag), cbv, rbv);
    eq(&format!("{} *size", tag), csz, rsz);
}

/// `utf8_check_full`: return value and `*codepoint`, plus the `codepoint==NULL`
/// branch. `buffer[0]` is always dereferenced by the C, so `buf` must be
/// non-empty and at least `size` bytes long.
fn cmp_check_full(d: &Duo, buf: &[u8], size: usize, tag: &str) {
    assert!(!buf.is_empty(), "test bug: utf8_check_full needs buffer[0]");
    assert!(
        size <= buf.len(),
        "test bug: size {} > buffer len {}",
        size,
        buf.len()
    );
    let p = buf.as_ptr() as *const c_char;

    let mut ccp: i32 = SENT_CP;
    let mut rcp: i32 = SENT_CP;
    let cret = unsafe { (d.c.utf8_check_full)(p, size, &mut ccp) };
    let rret = unsafe { (d.rs.utf8_check_full)(p, size, &mut rcp) };
    eq(
        &format!("utf8_check_full({}, size={}) return", tag, size),
        cret,
        rret,
    );
    eq(
        &format!("utf8_check_full({}, size={}) *codepoint", tag, size),
        ccp,
        rcp,
    );

    // NULL `codepoint` branch.
    let cret2 = unsafe { (d.c.utf8_check_full)(p, size, ptr::null_mut()) };
    let rret2 = unsafe { (d.rs.utf8_check_full)(p, size, ptr::null_mut()) };
    eq(
        &format!("utf8_check_full({}, size={}, cp=NULL) return", tag, size),
        cret2,
        rret2,
    );
}

/// `utf8_iterate`: NULL-ness and the returned pointer expressed as an offset
/// from the (shared) buffer start, plus `*codepoint`, plus the NULL-`codepoint`
/// branch.
fn cmp_iterate(d: &Duo, buf: &[u8], bufsize: usize, tag: &str) {
    assert!(
        bufsize <= buf.len(),
        "test bug: bufsize {} > buffer len {}",
        bufsize,
        buf.len()
    );
    let p = buf.as_ptr() as *const c_char;
    let base = p as usize;

    let mut ccp: i32 = SENT_CP;
    let mut rcp: i32 = SENT_CP;
    let cret = unsafe { (d.c.utf8_iterate)(p, bufsize, &mut ccp) };
    let rret = unsafe { (d.rs.utf8_iterate)(p, bufsize, &mut rcp) };
    eq(
        &format!("utf8_iterate({}, bufsize={}) NULL-ness", tag, bufsize),
        cret.is_null(),
        rret.is_null(),
    );
    if !cret.is_null() {
        eq(
            &format!("utf8_iterate({}, bufsize={}) offset", tag, bufsize),
            cret as usize - base,
            rret as usize - base,
        );
    }
    eq(
        &format!("utf8_iterate({}, bufsize={}) *codepoint", tag, bufsize),
        ccp,
        rcp,
    );

    // NULL `codepoint` branch.
    let cret2 = unsafe { (d.c.utf8_iterate)(p, bufsize, ptr::null_mut()) };
    let rret2 = unsafe { (d.rs.utf8_iterate)(p, bufsize, ptr::null_mut()) };
    eq(
        &format!(
            "utf8_iterate({}, bufsize={}, cp=NULL) NULL-ness",
            tag, bufsize
        ),
        cret2.is_null(),
        rret2.is_null(),
    );
    if !cret2.is_null() {
        eq(
            &format!("utf8_iterate({}, bufsize={}, cp=NULL) offset", tag, bufsize),
            cret2 as usize - base,
            rret2 as usize - base,
        );
    }
}

/// `utf8_check_string` on a shared buffer.
fn cmp_check_string(d: &Duo, buf: &[u8], len: usize, tag: &str) {
    assert!(
        len <= buf.len(),
        "test bug: len {} > buffer len {}",
        len,
        buf.len()
    );
    let p = buf.as_ptr() as *const c_char;
    let cret = unsafe { (d.c.utf8_check_string)(p, len) };
    let rret = unsafe { (d.rs.utf8_check_string)(p, len) };
    eq(
        &format!("utf8_check_string({}, len={})", tag, len),
        cret,
        rret,
    );
}

/// Copy `seq` into an 8-byte buffer padded with `pad`, so that any `bufsize`
/// up to 8 is in-bounds for both libraries.
fn padded8(seq: &[u8], pad: u8) -> Vec<u8> {
    assert!(seq.len() <= 8);
    let mut v = vec![pad; 8];
    v[..seq.len()].copy_from_slice(seq);
    v
}

// ---------------------------------------------------------------------------
// 1. utf8_check_first — CONFIGS 1; ERRORS 214, 215, 216
// ---------------------------------------------------------------------------

#[test]
fn utf8_check_first_all_256_bytes() {
    let d = duo();
    // Guard: the two handles must resolve to two distinct code objects, i.e.
    // no symbol interposition is collapsing them onto one implementation.
    for (name, cp, rp) in [
        (
            "utf8_encode",
            d.c.utf8_encode as usize,
            d.rs.utf8_encode as usize,
        ),
        (
            "utf8_check_first",
            d.c.utf8_check_first as usize,
            d.rs.utf8_check_first as usize,
        ),
        (
            "utf8_check_full",
            d.c.utf8_check_full as usize,
            d.rs.utf8_check_full as usize,
        ),
        (
            "utf8_iterate",
            d.c.utf8_iterate as usize,
            d.rs.utf8_iterate as usize,
        ),
        (
            "utf8_check_string",
            d.c.utf8_check_string as usize,
            d.rs.utf8_check_string as usize,
        ),
    ] {
        assert_ne!(cp, rp, "C and RUST resolved {} to the same address", name);
    }

    for b in 0u16..=255 {
        let byte = b as u8;
        let cret = unsafe { (d.c.utf8_check_first)(byte as c_char) };
        let rret = unsafe { (d.rs.utf8_check_first)(byte as c_char) };
        eq(&format!("utf8_check_first({:#04x})", byte), cret, rret);
    }

    // Ground-truth spot checks (utf.c lines 38-67): guards against the two
    // libraries agreeing on a wrong value.
    for (byte, want) in [
        (0x00u8, 1usize),
        (0x41, 1),
        (0x7F, 1),
        (0x80, 0), // ERRORS 214: continuation byte
        (0xBF, 0), // ERRORS 214
        (0xC0, 0), // ERRORS 215: overlong ASCII
        (0xC1, 0), // ERRORS 215
        (0xC2, 2),
        (0xDF, 2),
        (0xE0, 3),
        (0xEF, 3),
        (0xF0, 4),
        (0xF4, 4),
        (0xF5, 0), // ERRORS 216
        (0xFE, 0), // ERRORS 216
        (0xFF, 0), // ERRORS 216
    ] {
        let cret = unsafe { (d.c.utf8_check_first)(byte as c_char) };
        assert_eq!(
            cret, want,
            "C utf8_check_first({:#04x}) = {}, utf.c says {}",
            byte, cret, want
        );
    }
}

// ---------------------------------------------------------------------------
// 2. utf8_encode boundaries — CONFIGS 3; ERRORS 212, 213
// ---------------------------------------------------------------------------

#[test]
fn utf8_encode_boundaries() {
    let d = duo();
    let cps: [i32; 20] = [
        i32::MIN, // -2147483648
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
        0x10000,
        0x10FFFE,
        0x10FFFF,
        0x110000,
        0x7FFFFFFF,
        0x1FFFFF, // one more above-range 4-byte-shaped value
    ];
    for cp in cps {
        cmp_encode(d, cp);
    }

    // Ground-truth spot checks against utf.c lines 11-36.
    for (cp, want) in [
        (0x24i32, &[0x24u8][..]),
        (0x7F, &[0x7F]),
        (0x80, &[0xC2, 0x80]),
        (0xA2, &[0xC2, 0xA2]),
        (0x7FF, &[0xDF, 0xBF]),
        (0x800, &[0xE0, 0xA0, 0x80]),
        (0x20AC, &[0xE2, 0x82, 0xAC]),
        (0xFFFF, &[0xEF, 0xBF, 0xBF]),
        (0x10000, &[0xF0, 0x90, 0x80, 0x80]),
        (0x10348, &[0xF0, 0x90, 0x8D, 0x88]),
        (0x10FFFF, &[0xF4, 0x8F, 0xBF, 0xBF]),
    ] {
        let mut cb = [PAD as c_char; 8];
        let mut csz: usize = SENT_SIZE;
        let cret = unsafe { (d.c.utf8_encode)(cp, cb.as_mut_ptr(), &mut csz) };
        assert_eq!(cret, 0, "C utf8_encode({:#x}) should succeed", cp);
        assert_eq!(csz, want.len(), "C utf8_encode({:#x}) *size", cp);
        let got: Vec<u8> = cb[..want.len()].iter().map(|&x| x as u8).collect();
        assert_eq!(&got[..], want, "C utf8_encode({:#x}) bytes", cp);
        // The rest of the buffer must still hold the sentinel.
        assert!(
            cb[want.len()..].iter().all(|&x| x as u8 == PAD),
            "C utf8_encode({:#x}) wrote past *size",
            cp
        );
    }

    // ERRORS 212 / 213: `-1` and `*size` untouched.
    for cp in [i32::MIN, -1000, -1, 0x110000, 0x7FFFFFFF] {
        let mut cb = [PAD as c_char; 8];
        let mut csz: usize = SENT_SIZE;
        let cret = unsafe { (d.c.utf8_encode)(cp, cb.as_mut_ptr(), &mut csz) };
        assert_eq!(cret, -1, "C utf8_encode({}) should fail", cp);
        assert_eq!(csz, SENT_SIZE, "C utf8_encode({}) touched *size", cp);
        assert!(
            cb.iter().all(|&x| x as u8 == PAD),
            "C utf8_encode({}) touched the buffer",
            cp
        );
    }
}

// ---------------------------------------------------------------------------
// 3. utf8_encode randomized — CONFIGS 3
// ---------------------------------------------------------------------------

#[test]
fn utf8_encode_randomized() {
    let d = duo();
    let mut rng = Rng::new(0x0000_0000_0000_00A1);
    for _ in 0..20000 {
        cmp_encode(d, rng.next_u64() as i32);
    }
    // Plus an EXHAUSTIVE sweep of every codepoint in the valid range and a
    // little past it, so all four encoding branches and the upper reject bound
    // are covered for every input.
    for cp in 0..=0x11_00FFi32 {
        cmp_encode(d, cp);
    }
    // ... and a random sample of the huge above-range region.
    let mut rng2 = Rng::new(0x0000_0000_0000_00A2);
    for _ in 0..20000 {
        let cp = 0x11_0000i32 + (rng2.next_u32() % 0x7FEF_FFFF) as i32;
        cmp_encode(d, cp);
    }
}

// ---------------------------------------------------------------------------
// 4. utf8_check_full sizes + randomized — CONFIGS 2; ERRORS 217-221
// ---------------------------------------------------------------------------

#[test]
fn utf8_check_full_sizes_and_randomized() {
    let d = duo();
    let mut rng = Rng::new(0x0000_0000_0000_00B1);
    for size in 0usize..=6 {
        for _ in 0..2000 {
            // Half of the buffers get a plausible lead byte + continuation
            // bytes so the "valid"/"overlong"/"surrogate"/"out of range"
            // branches are reached, not just "bad continuation byte".
            let mut buf = rng.random_bytes(8);
            if rng.bool() {
                let leads: [u8; 10] = [
                    0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xED, 0xEF, 0xF0, 0xF4, 0xF7,
                ];
                buf[0] = leads[rng.below(leads.len())];
                for i in 1..8 {
                    if rng.below(8) != 0 {
                        buf[i] = 0x80 + (rng.next_u32() & 0x3F) as u8;
                    }
                }
            }
            let tag = hex(&buf);
            cmp_check_full(d, &buf, size, &tag);
        }
    }

    // Exhaustive: all 65536 two-byte sequences at `size == 2`, which covers
    // every lead/continuation combination of the 2-byte decode path.
    for hi in 0u16..=255 {
        for lo in 0u16..=255 {
            let buf = [hi as u8, lo as u8, 0, 0, 0, 0, 0, 0];
            let tag = format!("{:02X} {:02X}", hi, lo);
            cmp_check_full(d, &buf, 2, &tag);
        }
    }
}

// ---------------------------------------------------------------------------
// 5. utf8_check_full handcrafted — ERRORS 218-221
// ---------------------------------------------------------------------------

#[test]
fn utf8_check_full_handcrafted() {
    let d = duo();
    let cases: &[&[u8]] = &[
        // valid 2-byte
        &[0xC2, 0x80],
        &[0xC2, 0xA9],
        &[0xDF, 0xBF],
        // valid 3-byte
        &[0xE0, 0xA0, 0x80],
        &[0xE2, 0x82, 0xAC],
        &[0xED, 0x9F, 0xBF],
        &[0xEE, 0x80, 0x80],
        &[0xEF, 0xBF, 0xBF],
        // valid 4-byte
        &[0xF0, 0x90, 0x80, 0x80],
        &[0xF4, 0x8F, 0xBF, 0xBF],
        // overlong
        &[0xC0, 0x80],
        &[0xC1, 0xBF],
        &[0xE0, 0x80, 0x80],
        &[0xE0, 0x9F, 0xBF],
        &[0xF0, 0x80, 0x80, 0x80],
        &[0xF0, 0x8F, 0xBF, 0xBF],
        // surrogates
        &[0xED, 0xA0, 0x80],
        &[0xED, 0xBF, 0xBF],
        // above Unicode range
        &[0xF4, 0x90, 0x80, 0x80],
        &[0xF7, 0xBF, 0xBF, 0xBF],
        // bad continuation bytes
        &[0xC2, 0x41],
        &[0xC2, 0xC2],
        &[0xE2, 0x82, 0x41],
        &[0xE2, 0x41, 0x82],
        &[0xF0, 0x90, 0x80, 0x41],
        &[0xF0, 0x90, 0x41, 0x80],
        &[0xF0, 0x41, 0x80, 0x80],
        // ASCII lead (size != 1 forces the decode path anyway)
        &[0x41, 0x42, 0x43, 0x44],
        &[0x00, 0x80, 0x80, 0x80],
    ];
    for seq in cases {
        // Pad so every `size` in 0..=6 is in bounds; the C only ever reads
        // `size` bytes, so padding does not change the result for size <= len.
        for pad in [0x00u8, 0xBFu8] {
            let buf = padded8(seq, pad);
            let tag = format!("{} (pad {:#04x})", hex(seq), pad);
            for size in 0usize..=6 {
                cmp_check_full(d, &buf, size, &tag);
            }
        }
    }

    // Ground-truth spot checks against utf.c lines 69-114.
    let expect: &[(&[u8], usize, usize, i32)] = &[
        // (bytes, size, expected return, expected codepoint when return == 1)
        (&[0xC2, 0xA9], 2, 1, 0xA9),
        (&[0xDF, 0xBF], 2, 1, 0x7FF),
        (&[0xE2, 0x82, 0xAC], 3, 1, 0x20AC),
        (&[0xF0, 0x90, 0x8D, 0x88], 4, 1, 0x10348),
        (&[0xF4, 0x8F, 0xBF, 0xBF], 4, 1, 0x10FFFF),
        (&[0xC0, 0x80], 2, 0, 0),                   // ERRORS 221 overlong
        (&[0xE0, 0x80, 0x80], 3, 0, 0),             // ERRORS 221 overlong
        (&[0xF0, 0x80, 0x80, 0x80], 4, 0, 0),       // ERRORS 221 overlong
        (&[0xED, 0xA0, 0x80], 3, 0, 0),             // ERRORS 220 surrogate
        (&[0xED, 0xBF, 0xBF], 3, 0, 0),             // ERRORS 220 surrogate
        (&[0xF4, 0x90, 0x80, 0x80], 4, 0, 0),       // ERRORS 219 > 0x10FFFF
        (&[0xF7, 0xBF, 0xBF, 0xBF], 4, 0, 0),       // ERRORS 219 > 0x10FFFF
        (&[0xC2, 0x41], 2, 0, 0),                   // ERRORS 218 bad cont.
        (&[0xE2, 0x82, 0x41], 3, 0, 0),             // ERRORS 218 bad cont.
        (&[0xC2, 0xA9], 0, 0, 0),                   // ERRORS 217 size 0
        (&[0xC2, 0xA9], 1, 0, 0),                   // ERRORS 217 size 1
        (&[0xC2, 0xA9], 5, 0, 0),                   // ERRORS 217 size 5
        (&[0xC2, 0xA9], 6, 0, 0),                   // ERRORS 217 size 6
    ];
    for (seq, size, want_ret, want_cp) in expect {
        let buf = padded8(seq, 0x00);
        let mut ccp: i32 = SENT_CP;
        let cret = unsafe { (d.c.utf8_check_full)(buf.as_ptr() as *const c_char, *size, &mut ccp) };
        assert_eq!(
            cret,
            *want_ret,
            "C utf8_check_full({}, size={}) = {}, utf.c says {}",
            hex(seq),
            size,
            cret,
            want_ret
        );
        if *want_ret == 1 {
            assert_eq!(
                ccp,
                *want_cp,
                "C utf8_check_full({}, size={}) *codepoint",
                hex(seq),
                size
            );
        } else {
            assert_eq!(
                ccp,
                SENT_CP,
                "C utf8_check_full({}, size={}) wrote *codepoint on failure",
                hex(seq),
                size
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. utf8_iterate all shapes — CONFIGS 4; ERRORS 222-225
// ---------------------------------------------------------------------------

#[test]
fn utf8_iterate_all_shapes() {
    let d = duo();

    let shapes: &[&[u8]] = &[
        // valid 1-byte
        &[0x00],
        &[0x41],
        &[0x7F],
        // valid 2-byte
        &[0xC2, 0x80],
        &[0xC2, 0xA9],
        &[0xDF, 0xBF],
        // valid 3-byte
        &[0xE0, 0xA0, 0x80],
        &[0xE2, 0x82, 0xAC],
        &[0xEF, 0xBF, 0xBF],
        // valid 4-byte
        &[0xF0, 0x90, 0x80, 0x80],
        &[0xF4, 0x8F, 0xBF, 0xBF],
        // invalid lead bytes (utf8_check_first == 0)
        &[0x80, 0x80, 0x80, 0x80],
        &[0xBF, 0xBF],
        &[0xC0, 0x80],
        &[0xC1, 0xBF],
        &[0xF5, 0x80, 0x80, 0x80],
        &[0xFE, 0x80],
        &[0xFF, 0xFF, 0xFF, 0xFF],
        // truncated sequences (count > bufsize when bufsize is small)
        &[0xC2],
        &[0xE2, 0x82],
        &[0xF0, 0x90, 0x80],
        // utf8_check_full failures
        &[0xED, 0xA0, 0x80],
        &[0xF4, 0x90, 0x80, 0x80],
        &[0xE0, 0x80, 0x80],
        &[0xC2, 0x41],
        &[0xE2, 0x82, 0x41],
        // multi-character buffers: the return offset must land mid-buffer
        &[0x41, 0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F],
        &[0xC2, 0xA9, 0x41, 0x42],
        &[0xF0, 0x9F, 0x98, 0x80, 0x41],
    ];

    for shape in shapes {
        for pad in [0x00u8, 0xBFu8] {
            let buf = padded8(shape, pad);
            let tag = format!("{} (pad {:#04x})", hex(shape), pad);
            let mut sizes = vec![0usize, 1, 2, 3, 4, shape.len(), 8];
            sizes.sort_unstable();
            sizes.dedup();
            for bufsize in sizes {
                cmp_iterate(d, &buf, bufsize, &tag);
            }
        }
    }

    // Randomized buffers.
    let mut rng = Rng::new(0x0000_0000_0000_00C1);
    for _ in 0..3000 {
        let mut buf = rng.random_bytes(8);
        if rng.bool() {
            let leads: [u8; 12] = [
                0x00, 0x41, 0x7F, 0x80, 0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xED, 0xF0, 0xF4,
            ];
            buf[0] = leads[rng.below(leads.len())];
            for i in 1..8 {
                if rng.below(8) != 0 {
                    buf[i] = 0x80 + (rng.next_u32() & 0x3F) as u8;
                }
            }
        }
        let tag = hex(&buf);
        for bufsize in [0usize, 1, 2, 3, 4, 8] {
            cmp_iterate(d, &buf, bufsize, &tag);
        }
    }

    // Exhaustive: all 65536 two-byte buffers, at bufsize 1 and 2, so every
    // lead byte is seen with every possible second byte.
    for hi in 0u16..=255 {
        for lo in 0u16..=255 {
            let buf = [hi as u8, lo as u8];
            let tag = format!("{:02X} {:02X}", hi, lo);
            cmp_iterate(d, &buf, 1, &tag);
            cmp_iterate(d, &buf, 2, &tag);
        }
    }

    // Ground-truth spot checks against utf.c lines 116-138.
    // (bytes, bufsize, expected advance in bytes or None for NULL, codepoint)
    let expect: &[(&[u8], usize, Option<usize>, i32)] = &[
        (&[0x41, 0x42], 0, Some(0), SENT_CP), // ERRORS 222: buffer unchanged
        (&[0x41, 0x42], 2, Some(1), 0x41),
        (&[0x00, 0x42], 2, Some(1), 0x00),
        (&[0xC2, 0xA9], 2, Some(2), 0xA9),
        (&[0xE2, 0x82, 0xAC], 3, Some(3), 0x20AC),
        (&[0xF0, 0x90, 0x8D, 0x88], 4, Some(4), 0x10348),
        (&[0x80, 0x80], 2, None, SENT_CP), // ERRORS 223: bad lead
        (&[0xC0, 0x80], 2, None, SENT_CP), // ERRORS 223: bad lead
        (&[0xF5, 0x80], 2, None, SENT_CP), // ERRORS 223: bad lead
        (&[0xC2, 0xA9], 1, None, SENT_CP), // ERRORS 224: count > bufsize
        (&[0xE2, 0x82, 0xAC], 2, None, SENT_CP), // ERRORS 224
        (&[0xF0, 0x90, 0x8D, 0x88], 3, None, SENT_CP), // ERRORS 224
        (&[0xED, 0xA0, 0x80], 3, None, SENT_CP), // ERRORS 225: check_full fails
        (&[0xC2, 0x41], 2, None, SENT_CP),       // ERRORS 225
    ];
    for (seq, bufsize, want_off, want_cp) in expect {
        let buf = padded8(seq, 0x00);
        let p = buf.as_ptr() as *const c_char;
        let mut ccp: i32 = SENT_CP;
        let cret = unsafe { (d.c.utf8_iterate)(p, *bufsize, &mut ccp) };
        match want_off {
            None => assert!(
                cret.is_null(),
                "C utf8_iterate({}, bufsize={}) should be NULL",
                hex(seq),
                bufsize
            ),
            Some(off) => {
                assert!(
                    !cret.is_null(),
                    "C utf8_iterate({}, bufsize={}) should not be NULL",
                    hex(seq),
                    bufsize
                );
                assert_eq!(
                    cret as usize - p as usize,
                    *off,
                    "C utf8_iterate({}, bufsize={}) advance",
                    hex(seq),
                    bufsize
                );
            }
        }
        assert_eq!(
            ccp,
            *want_cp,
            "C utf8_iterate({}, bufsize={}) *codepoint",
            hex(seq),
            bufsize
        );
    }
}

// ---------------------------------------------------------------------------
// 7. utf8_check_string — CONFIGS 5; ERRORS 226-228
// ---------------------------------------------------------------------------

#[test]
fn utf8_check_string_shapes() {
    let d = duo();

    // len 0 on a 1-byte buffer, and len 0 on a non-empty buffer.
    let one = vec![0x41u8];
    cmp_check_string(d, &one, 0, "\"A\"");
    cmp_check_string(d, &one, 1, "\"A\"");
    let nonempty = b"hello world".to_vec();
    cmp_check_string(d, &nonempty, 0, "\"hello world\" with len 0");
    // len shorter than the real buffer.
    for len in 0..=nonempty.len() {
        cmp_check_string(d, &nonempty, len, "\"hello world\" prefix");
    }
    // A truncated multi-byte character kept out of range by a short `len`.
    let trunc = {
        let mut v = b"ab".to_vec();
        v.extend_from_slice(&[0xE2, 0x82, 0xAC]);
        v.extend_from_slice(b"cd");
        v
    };
    for len in 0..=trunc.len() {
        cmp_check_string(d, &trunc, len, "\"ab<euro>cd\" prefix");
    }

    // Every single byte value as a length-1 string (ERRORS 226 + 227).
    for b in 0u16..=255 {
        let buf = vec![b as u8];
        cmp_check_string(d, &buf, 1, &format!("single byte {:#04x}", b as u8));
    }

    // Random ASCII, len 1..64.
    let mut rng = Rng::new(0x0000_0000_0000_00D1);
    for _ in 0..2000 {
        let n = 1 + rng.below(63);
        let buf = rng.ascii_string(n);
        let tag = hex(&buf);
        cmp_check_string(d, &buf, buf.len(), &tag);
    }

    // Random bytes, len 1..64.
    let mut rng = Rng::new(0x0000_0000_0000_00D2);
    for _ in 0..5000 {
        let n = 1 + rng.below(63);
        let mut buf = rng.random_bytes(n);
        // Half the time bias the bytes towards lead/continuation values so the
        // `utf8_check_full` and `count > length - i` branches are reached often.
        if rng.bool() {
            for i in 0..buf.len() {
                match rng.below(4) {
                    0 => buf[i] = 0x80 + (rng.next_u32() & 0x3F) as u8,
                    1 => buf[i] = 0xC2 + (rng.next_u32() % 0x36) as u8,
                    2 => buf[i] = (rng.next_u32() & 0x7F) as u8,
                    _ => {}
                }
            }
        }
        let tag = hex(&buf);
        cmp_check_string(d, &buf, buf.len(), &tag);
        // Also a length shorter than the buffer.
        let shorter = rng.below(buf.len() + 1);
        cmp_check_string(d, &buf, shorter, &tag);
    }

    // Valid random UTF-8, plus truncations by 1, 2 and 3 bytes.
    let mut rng = Rng::new(0x0000_0000_0000_00D3);
    for _ in 0..2000 {
        let chars = 1 + rng.below(24);
        let buf = rng.utf8_string(chars);
        let tag = hex(&buf);
        cmp_check_string(d, &buf, buf.len(), &tag);
        for cut in 1usize..=3 {
            if buf.len() >= cut {
                cmp_check_string(d, &buf, buf.len() - cut, &format!("{} -{}", tag, cut));
            }
        }
        // Embedded NUL: valid UTF-8 with a NUL spliced into the middle.
        let mut withnul = buf.clone();
        let at = rng.below(withnul.len() + 1);
        withnul.insert(at, 0x00);
        let tag2 = hex(&withnul);
        cmp_check_string(d, &withnul, withnul.len(), &tag2);
    }

    // Explicit embedded-NUL shapes, including NUL-terminated buffers where the
    // declared `len` runs past the terminator.
    let nul_cases: &[&[u8]] = &[
        &[0x00],
        &[0x00, 0x00],
        &[0x41, 0x00, 0x42],
        &[0x00, 0xC2, 0xA9, 0x00],
        &[0xC2, 0x00, 0xA9],
        &[0xE2, 0x82, 0x00],
        &[0x41, 0x42, 0x00, 0xFF, 0xFE],
    ];
    for seq in nul_cases {
        let v = seq.to_vec();
        let tag = hex(seq);
        for len in 0..=v.len() {
            cmp_check_string(d, &v, len, &tag);
        }
        // `cbuf` appends a terminator; a `len` covering it must also match.
        let cb = cbuf(seq);
        cmp_check_string(d, &cb, cb.len(), &format!("{} + NUL", tag));
    }

    // Exhaustive: every two-byte string at len 1 and len 2.
    for hi in 0u16..=255 {
        for lo in 0u16..=255 {
            let buf = [hi as u8, lo as u8];
            let tag = format!("{:02X} {:02X}", hi, lo);
            cmp_check_string(d, &buf, 1, &tag);
            cmp_check_string(d, &buf, 2, &tag);
        }
    }

    // Ground-truth spot checks against utf.c lines 140-159.
    let expect: &[(&[u8], usize, i32)] = &[
        (&[0x41], 0, 1),
        (&[0x41], 1, 1),
        (&[0x00], 1, 1),
        (b"hello", 5, 1),
        (&[0xC2, 0xA9], 2, 1),
        (&[0xE2, 0x82, 0xAC], 3, 1),
        (&[0xF0, 0x90, 0x8D, 0x88], 4, 1),
        (&[0x41, 0xC2, 0xA9, 0x42], 4, 1),
        (&[0x80], 1, 0),             // ERRORS 226: invalid lead byte
        (&[0xC0, 0x80], 2, 0),       // ERRORS 226: invalid lead byte
        (&[0xFF], 1, 0),             // ERRORS 226
        (&[0x41, 0xC2], 2, 0),       // ERRORS 227: truncated at end
        (&[0xE2, 0x82], 2, 0),       // ERRORS 227
        (&[0xF0, 0x90, 0x8D], 3, 0), // ERRORS 227
        (&[0xC2, 0x41], 2, 0),       // ERRORS 228: check_full fails mid-string
        (&[0xED, 0xA0, 0x80], 3, 0), // ERRORS 228: surrogate
        (&[0x41, 0xF4, 0x90, 0x80, 0x80], 5, 0), // ERRORS 228: > 0x10FFFF
        // `length` shorter than the buffer stops before the bad byte.
        (&[0x41, 0xFF], 1, 1),
        (&[0x41, 0x42, 0x80], 2, 1),
    ];
    for (seq, len, want) in expect {
        let v = seq.to_vec();
        let cret = unsafe { (d.c.utf8_check_string)(v.as_ptr() as *const c_char, *len) };
        assert_eq!(
            cret,
            *want,
            "C utf8_check_string({}, len={}) = {}, utf.c says {}",
            hex(seq),
            len,
            cret,
            want
        );
    }
}
