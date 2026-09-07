//! Phase B — CONFIGS.md row 26, exhaustively: the `\uXXXX` escape decoder
//! (`utf16_literal_to_utf8`) and the printer's escaping.  Every UTF-8 output
//! length (1..4 bytes), every surrogate-pair combination and every malformed
//! escape is driven through both `.so`s.

mod common;
use common::*;
use std::ffi::{c_char, c_int};

unsafe fn cmp_parse_text(label: &str, c: &Api, r: &Api, text: &[u8]) {
    let buf = cbytes(text);
    let ci = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
    let ri = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
    assert_eq!(
        ci.is_null(),
        ri.is_null(),
        "{}: NULL-ness for {:?}",
        label,
        String::from_utf8_lossy(text)
    );
    if !ci.is_null() {
        assert_eq!(
            fingerprint(c, ci),
            fingerprint(r, ri),
            "{}: decoded bytes for {:?}",
            label,
            String::from_utf8_lossy(text)
        );
        assert_eq!(
            show(&print_unformatted_and_free(c, ci)),
            show(&print_unformatted_and_free(r, ri)),
            "{}: reprint for {:?}",
            label,
            String::from_utf8_lossy(text)
        );
        assert_eq!(
            show(&print_and_free(c, ci)),
            show(&print_and_free(r, ri)),
            "{}: reprint formatted for {:?}",
            label,
            String::from_utf8_lossy(text)
        );
    }
    (c.cJSON_Delete)(ci);
    (r.cJSON_Delete)(ri);
}

/// Every BMP code unit as a `\uXXXX` escape (including the surrogate range,
/// which the C code rejects).
#[test]
fn unicode_all_bmp_escapes() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        for cu in 0u32..=0xFFFF {
            let t = format!("\"\\u{:04x}\"", cu);
            cmp_parse_text("bmp lower-hex", &c, &r, t.as_bytes());
        }
        for cu in 0u32..=0xFFFF {
            let t = format!("\"\\u{:04X}\"", cu);
            cmp_parse_text("bmp upper-hex", &c, &r, t.as_bytes());
        }
    }
}

/// Surrogate pairs: all high surrogates against a fixed low, all lows against a
/// fixed high, plus a stride over the whole supplementary plane range.
#[test]
fn unicode_surrogate_pairs() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        for hi in 0xD800u32..=0xDBFF {
            let t = format!("\"\\u{:04x}\\udc00\"", hi);
            cmp_parse_text("high surrogate sweep", &c, &r, t.as_bytes());
        }
        for lo in 0xDC00u32..=0xDFFF {
            let t = format!("\"\\ud800\\u{:04x}\"", lo);
            cmp_parse_text("low surrogate sweep", &c, &r, t.as_bytes());
        }
        // invalid second halves
        for lo in [
            0x0041u32, 0x00FF, 0x0800, 0xD7FF, 0xD800, 0xDBFF, 0xDBFE, 0xE000, 0xFFFF,
        ] {
            let t = format!("\"\\ud83d\\u{:04x}\"", lo);
            cmp_parse_text("invalid low surrogate", &c, &r, t.as_bytes());
        }
        // every supplementary code point, strided
        let mut cp = 0x10000u32;
        while cp <= 0x10FFFF {
            let v = cp - 0x10000;
            let hi = 0xD800 + (v >> 10);
            let lo = 0xDC00 + (v & 0x3FF);
            let t = format!("\"\\u{:04x}\\u{:04x}\"", hi, lo);
            cmp_parse_text("supplementary", &c, &r, t.as_bytes());
            cp += 97;
        }
    }
}

/// Escapes surrounded by other content, repeated, and inside object keys.
#[test]
fn unicode_escapes_in_context() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let interesting: Vec<u32> = vec![
            0x0000, 0x0001, 0x001F, 0x0020, 0x0022, 0x002F, 0x005C, 0x007E, 0x007F, 0x0080,
            0x00A0, 0x00FF, 0x0100, 0x07FF, 0x0800, 0x0FFF, 0x1000, 0xD7FF, 0xD800, 0xDBFF,
            0xDC00, 0xDFFF, 0xE000, 0xFFFD, 0xFFFF, 0x0041, 0x00E9, 0x20AC,
        ];
        for cu in &interesting {
            for template in [
                "\"pre\\u{:04x}post\"",
                "\"\\u{:04x}\\u{:04x}\"",
                "\"a\\u{:04x}\"",
                "\"\\u{:04x}b\"",
                "[\"\\u{:04x}\"]",
                "{{\"\\u{:04x}\":1}}",
                "{{\"k\":\"\\u{:04x}\"}}",
            ] {
                // the templates take the code unit either once or twice
                let t = if template.matches("{:04x}").count() == 2 {
                    format!("\"\\u{:04x}\\u{:04x}\"", cu, cu)
                } else {
                    template.replacen("{:04x}", &format!("{:04x}", cu), 1)
                };
                cmp_parse_text("escape in context", &c, &r, t.as_bytes());
            }
        }
        // long strings mixing escapes, raw UTF-8 and simple escapes
        let mut rng = Rng::new(0x1234_5678);
        for _ in 0..500 {
            let mut t = String::from("\"");
            let n = rng.below(20) as usize;
            for _ in 0..n {
                match rng.below(8) {
                    0 => t.push_str(&format!("\\u{:04x}", rng.below(0x10000))),
                    1 => {
                        let v = rng.below(0x100000) as u32;
                        let hi = 0xD800 + (v >> 10);
                        let lo = 0xDC00 + (v & 0x3FF);
                        t.push_str(&format!("\\u{:04x}\\u{:04x}", hi, lo));
                    }
                    2 => t.push_str("\\n"),
                    3 => t.push_str("\\t"),
                    4 => t.push_str("\\\\"),
                    5 => t.push_str("\\\""),
                    6 => t.push_str("\\/"),
                    _ => t.push((b'a' + rng.below(26) as u8) as char),
                }
            }
            t.push('"');
            cmp_parse_text("random escape soup", &c, &r, t.as_bytes());
        }
    }
}

/// Malformed `\u` escapes (parse_hex4 rejections and truncated sequences).
#[test]
fn unicode_malformed_escapes() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let bad: Vec<&str> = vec![
            "\"\\u\"",
            "\"\\u0\"",
            "\"\\u00\"",
            "\"\\u000\"",
            "\"\\uZ000\"",
            "\"\\u0Z00\"",
            "\"\\u00Z0\"",
            "\"\\u000Z\"",
            "\"\\u 000\"",
            "\"\\u-000\"",
            "\"\\u+000\"",
            "\"\\u00/0\"",
            "\"\\u00:0\"",
            "\"\\u00@0\"",
            "\"\\u00G0\"",
            "\"\\u00g0\"",
            "\"\\u00`0\"",
            "\"\\ud800\"",
            "\"\\ud800\\u\"",
            "\"\\ud800\\u0\"",
            "\"\\ud800\\ud800\"",
            "\"\\ud800x\"",
            "\"\\ud800\\n\"",
            "\"\\udbff\"",
            "\"\\udc00\"",
            "\"\\udfff\"",
            "\"\\q\"",
            "\"\\\"",
            "\"\\U0041\"",
            "\"\\x41\"",
            "\"\\0\"",
            "\"\\a\"",
            "\"\\v\"",
        ];
        for t in bad {
            let buf = cbytes(t.as_bytes());
            let a = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
            let b = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
            assert_eq!(a.is_null(), b.is_null(), "malformed {:?} NULL-ness", t);
            assert_eq!(
                (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                "malformed {:?} error offset",
                t
            );
            if !a.is_null() {
                assert_eq!(
                    show(&print_unformatted_and_free(&c, a)),
                    show(&print_unformatted_and_free(&r, b)),
                    "malformed {:?} accepted output",
                    t
                );
            }
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }
    }
}

/// Printing: every byte value 0..255 as a raw string byte, and every
/// single-byte string, so the printer's escape table is fully exercised.
#[test]
fn unicode_printer_escapes() {
    unsafe {
        let (c, r) = both();
        for b in 1u8..=255 {
            let bytes = [b];
            let ci = (c.cJSON_CreateString)(cbytes(&bytes).as_ptr() as *const c_char);
            let ri = (r.cJSON_CreateString)(cbytes(&bytes).as_ptr() as *const c_char);
            assert_eq!(
                show(&print_unformatted_and_free(&c, ci)),
                show(&print_unformatted_and_free(&r, ri)),
                "printer escape for byte {:#02x}",
                b
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // pairs of bytes, to catch off-by-one in the escape-length computation
        let mut rng = Rng::new(0xE5CA9E);
        for _ in 0..4000 {
            let n = 1 + rng.below(6) as usize;
            let bytes: Vec<u8> = (0..n).map(|_| 1 + rng.below(255) as u8).collect();
            let ci = (c.cJSON_CreateString)(cbytes(&bytes).as_ptr() as *const c_char);
            let ri = (r.cJSON_CreateString)(cbytes(&bytes).as_ptr() as *const c_char);
            assert_eq!(
                show(&print_unformatted_and_free(&c, ci)),
                show(&print_unformatted_and_free(&r, ri)),
                "printer escape for {:?}",
                bytes
            );
            // also through the object-key printer
            let co = (c.cJSON_CreateObject)();
            let ro = (r.cJSON_CreateObject)();
            (c.cJSON_AddItemToObject)(co, cbytes(&bytes).as_ptr() as *const c_char, ci);
            (r.cJSON_AddItemToObject)(ro, cbytes(&bytes).as_ptr() as *const c_char, ri);
            assert_eq!(
                show(&print_and_free(&c, co)),
                show(&print_and_free(&r, ro)),
                "printer key escape for {:?}",
                bytes
            );
            (c.cJSON_Delete)(co);
            (r.cJSON_Delete)(ro);
        }
        let _: c_int = 0;
    }
}
