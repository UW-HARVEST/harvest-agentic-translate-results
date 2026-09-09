//! CONFIGS.md rows 69-75: lexer character classification helpers, token
//! strings, keyword search, and the whole UTF-8 / Unicode-case surface.
//!
//! Strategy for the exhaustive sweeps: every input is exercised, but only
//! "interesting" inputs get a full detail line; everything else is folded into
//! a 64-bit FNV-1a checksum which is printed once per 64K block, so a
//! divergence is localised to a block without producing gigabytes of output.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint};

/* ------------------------------------------------------------------------- */
/* helpers                                                                   */
/* ------------------------------------------------------------------------- */

struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    #[inline]
    fn byte(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(0x100_0000_01b3);
    }
    #[inline]
    fn add(&mut self, v: c_int) {
        let u = v as u32;
        self.byte(u as u8);
        self.byte((u >> 8) as u8);
        self.byte((u >> 16) as u8);
        self.byte((u >> 24) as u8);
    }
}

/// Print a 64-bit checksum as two unsigned halves (harness helpers only).
unsafe fn p_ck(label: &str, v: u64) {
    p_uint(&format!("{}.hi", label), (v >> 32) as c_uint);
    p_uint(&format!("{}.lo", label), (v & 0xffff_ffff) as c_uint);
}

fn hexbytes(b: &[i8]) -> String {
    let mut s = String::with_capacity(b.len() * 3);
    for (i, x) in b.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s += &format!("{:02x}", *x as u8);
    }
    s
}

/* ------------------------------------------------------------------------- */
/* row 69: jsY_iswhite / jsY_isnewline / jsY_ishex / jsY_tohex               */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg69_lex_charclass() {
    diff("cfg69_lex_charclass", |api| unsafe {
        p_line("== row69 jsY_iswhite/isnewline/ishex/tohex ==");
        let mut ck = Fnv::new();
        let detail = |c: c_int, w: c_int, n: c_int, h: c_int, t: c_int| {
            p_line(&format!(
                "c={} (0x{:x}) iswhite={} isnewline={} ishex={} tohex={}",
                c, c as u32, w, n, h, t
            ));
        };
        for c in -5i32..=0x3001 {
            let w = (api.jsY_iswhite)(c);
            let n = (api.jsY_isnewline)(c);
            let h = (api.jsY_ishex)(c);
            let t = (api.jsY_tohex)(c);
            ck.add(c);
            ck.add(w);
            ck.add(n);
            ck.add(h);
            ck.add(t);
            if w != 0 || n != 0 || h != 0 || t != 0 {
                detail(c, w, n, h, t);
            }
        }
        p_ck("ck_69", ck.0);
        for &c in &[i32::MIN, i32::MIN + 1, -1i32, i32::MAX - 1, i32::MAX] {
            detail(
                c,
                (api.jsY_iswhite)(c),
                (api.jsY_isnewline)(c),
                (api.jsY_ishex)(c),
                (api.jsY_tohex)(c),
            );
        }
    });
}

/* ------------------------------------------------------------------------- */
/* row 70: jsY_tokenstring                                                   */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg70_tokenstring() {
    diff("cfg70_tokenstring", |api| unsafe {
        p_line("== row70 jsY_tokenstring ==");
        for t in -5i32..=400 {
            let s = (api.jsY_tokenstring)(t);
            p_str(&format!("tok[{}]", t), s);
        }
    });
}

/* ------------------------------------------------------------------------- */
/* row 71: jsY_findword                                                      */
/* ------------------------------------------------------------------------- */

#[test]
fn cfg71_findword() {
    diff("cfg71_findword", |api| unsafe {
        p_line("== row71 jsY_findword ==");
        /* Same sorted list jslex.c passes in (the keyword table). */
        let names = [
            "break", "case", "catch", "continue", "debugger", "default", "delete", "do", "else",
            "false", "finally", "for", "function", "if", "in", "instanceof", "new", "null",
            "return", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while",
            "with",
        ];
        let owned: Vec<std::ffi::CString> = names.iter().map(|s| cs(s)).collect();
        let list: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        let num = list.len() as c_int;

        /* every word present */
        for (i, n) in names.iter().enumerate() {
            let probe = cs(n);
            let r = (api.jsY_findword)(probe.as_ptr(), list.as_ptr(), num);
            p_line(&format!("present[{}] {} -> {}", i, n, r));
        }

        /* absent words: before/after/inside the range, prefixes, suffixes */
        let absent = [
            "", "a", "aardvark", "brea", "breakk", "Break", "BREAK", "cases", "cat", "catchx",
            "do ", "dp", "elsf", "instance", "instanceoff", "nul", "nullx", "swi", "with ", "withh",
            "z", "zzz", "zzzzzzzzzzzzzzzz", "~", "\x01",
        ];
        for w in absent.iter() {
            let probe = cs(w);
            let r = (api.jsY_findword)(probe.as_ptr(), list.as_ptr(), num);
            p_line(&format!("absent[{}] -> {}", w, r));
        }

        /* num == 0 with a valid list: nothing is ever compared */
        for w in ["break", "with", "", "zzz"].iter() {
            let probe = cs(w);
            let r = (api.jsY_findword)(probe.as_ptr(), list.as_ptr(), 0);
            p_line(&format!("num0[{}] -> {}", w, r));
        }

        /* num == 1: only list[0] ("break") is reachable */
        for w in ["break", "case", "aardvark", "", "with"].iter() {
            let probe = cs(w);
            let r = (api.jsY_findword)(probe.as_ptr(), list.as_ptr(), 1);
            p_line(&format!("num1[{}] -> {}", w, r));
        }

        /* num == 2 and num == num-1 for good measure */
        for &n in [2i32, num - 1].iter() {
            for w in ["break", "case", "catch", "while", "with"].iter() {
                let probe = cs(w);
                let r = (api.jsY_findword)(probe.as_ptr(), list.as_ptr(), n);
                p_line(&format!("num{}[{}] -> {}", n, w, r));
            }
        }

        /* single-element list holding the last word (word beyond the end) */
        let one: Vec<*const c_char> = vec![owned[num as usize - 1].as_ptr()];
        for w in ["with", "break", "zzz"].iter() {
            let probe = cs(w);
            let r = (api.jsY_findword)(probe.as_ptr(), one.as_ptr(), 1);
            p_line(&format!("one[{}] -> {}", w, r));
        }
    });
}

/* ------------------------------------------------------------------------- */
/* row 72: jsU_runelen + runetochar/chartorune round trip                    */
/* ------------------------------------------------------------------------- */

unsafe fn rt_detail(api: &Api, c: c_int) {
    let mut buf = [0i8; 8];
    let rune: c_int = c;
    let rl = (api.jsU_runelen)(c);
    let n = (api.jsU_runetochar)(buf.as_mut_ptr(), &rune);
    let mut dec: c_int = -12345;
    let dl = (api.jsU_chartorune)(&mut dec, buf.as_ptr());
    p_line(&format!(
        "c={} (0x{:x}) runelen={} enc={} bytes=[{}] dec={} (0x{:x}) declen={}",
        c,
        c as u32,
        rl,
        n,
        hexbytes(&buf),
        dec,
        dec as u32,
        dl
    ));
}

#[test]
fn cfg72_runelen_roundtrip() {
    diff("cfg72_runelen_roundtrip", |api| unsafe {
        p_line("== row72 runelen / runetochar->chartorune round trip ==");

        /* negative and extreme inputs */
        p_line("-- extremes --");
        for &c in &[
            -1i32,
            -2,
            i32::MIN,
            i32::MIN + 1,
            0x7FFF_FFFF,
            0x7FFF_FFFE,
            -0x110000,
        ] {
            rt_detail(api, c);
        }

        /* exhaustive detail for the first 300 runes */
        p_line("-- 0..300 --");
        for c in 0i32..300 {
            rt_detail(api, c);
        }

        /* boundary neighbourhoods */
        for &(lo, hi, tag) in &[
            (0x7Fi32, 0x81i32, "0x7F..0x81"),
            (0x7FF, 0x801, "0x7FF..0x801"),
            (0xD7FF, 0xE001, "0xD7FF..0xE001"),
            (0xFFFD, 0x10002, "0xFFFD..0x10002"),
            (0x10FFFE, 0x110002, "0x10FFFE..0x110002"),
        ] {
            p_line(&format!("-- {} --", tag));
            for c in lo..=hi {
                rt_detail(api, c);
            }
        }

        /* full sweep, checksummed per 64K block */
        p_line("-- sweep 0..=0x110010 --");
        let mut ck = Fnv::new();
        for c in 0i32..=0x110010 {
            let mut buf = [0i8; 8];
            let rune: c_int = c;
            let rl = (api.jsU_runelen)(c);
            let n = (api.jsU_runetochar)(buf.as_mut_ptr(), &rune);
            let mut dec: c_int = -12345;
            let dl = (api.jsU_chartorune)(&mut dec, buf.as_ptr());
            ck.add(c);
            ck.add(rl);
            ck.add(n);
            for b in buf.iter() {
                ck.byte(*b as u8);
            }
            ck.add(dec);
            ck.add(dl);
            if (c & 0xFFFF) == 0xFFFF {
                p_ck(&format!("ck_block_{:x}", c >> 16), ck.0);
            }
        }
        p_ck("ck_72_final", ck.0);
    });
}

/* ------------------------------------------------------------------------- */
/* row 72b: jsU_chartorune on invalid / truncated UTF-8                      */
/* ------------------------------------------------------------------------- */

unsafe fn ct_case(api: &Api, label: &str, bytes: &[u8]) {
    let mut buf = [0i8; 16];
    for (i, b) in bytes.iter().enumerate() {
        buf[i] = *b as i8;
    }
    let mut rune: c_int = -12345;
    let n = (api.jsU_chartorune)(&mut rune, buf.as_ptr());
    p_line(&format!(
        "{} seq=[{}] n={} rune={} (0x{:x})",
        label,
        hexbytes(&buf[..bytes.len().max(1)]),
        n,
        rune,
        rune as u32
    ));
}

#[test]
fn cfg72b_chartorune_invalid() {
    diff("cfg72b_chartorune_invalid", |api| unsafe {
        p_line("== row72b chartorune on invalid/truncated sequences ==");

        p_line("-- single bytes 0x00..0xFF --");
        for b in 0u32..=0xFF {
            ct_case(api, &format!("single[{:02x}]", b), &[b as u8]);
        }

        p_line("-- lead 0xC0..0xFF + one follow byte --");
        for lead in 0xC0u32..=0xFF {
            for &f in &[0x00u8, 0x41, 0x80, 0xBF, 0xFF] {
                ct_case(
                    api,
                    &format!("pair[{:02x} {:02x}]", lead, f),
                    &[lead as u8, f],
                );
            }
        }

        p_line("-- overlong encodings --");
        ct_case(api, "overlong_C0_80", &[0xC0, 0x80]);
        ct_case(api, "overlong_C0_80_trail", &[0xC0, 0x80, 0x41]);
        ct_case(api, "overlong_C1_BF", &[0xC1, 0xBF]);
        ct_case(api, "overlong_E0_80_80", &[0xE0, 0x80, 0x80]);
        ct_case(api, "overlong_E0_81_BF", &[0xE0, 0x81, 0xBF]);
        ct_case(api, "overlong_E0_9F_BF", &[0xE0, 0x9F, 0xBF]);
        ct_case(api, "overlong_F0_80_80_80", &[0xF0, 0x80, 0x80, 0x80]);
        ct_case(api, "overlong_F0_8F_BF_BF", &[0xF0, 0x8F, 0xBF, 0xBF]);

        p_line("-- surrogate encodings --");
        ct_case(api, "surr_ED_A0_80", &[0xED, 0xA0, 0x80]);
        ct_case(api, "surr_ED_AF_BF", &[0xED, 0xAF, 0xBF]);
        ct_case(api, "surr_ED_B0_80", &[0xED, 0xB0, 0x80]);
        ct_case(api, "surr_ED_BF_BF", &[0xED, 0xBF, 0xBF]);
        ct_case(api, "surrpair_ED_A0_BD_ED_B8_80", &[0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80]);

        p_line("-- 5/6-byte lead bytes --");
        ct_case(api, "five_F8_88_80_80_80", &[0xF8, 0x88, 0x80, 0x80, 0x80]);
        ct_case(api, "five_F8_80_80_80_80", &[0xF8, 0x80, 0x80, 0x80, 0x80]);
        ct_case(api, "five_FB_BF_BF_BF_BF", &[0xFB, 0xBF, 0xBF, 0xBF, 0xBF]);
        ct_case(api, "six_FC_84_80_80_80_80", &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x80]);
        ct_case(api, "six_FD_BF_BF_BF_BF_BF", &[0xFD, 0xBF, 0xBF, 0xBF, 0xBF, 0xBF]);
        ct_case(api, "seven_FE_80_80_80_80_80_80", &[0xFE, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]);
        ct_case(api, "eight_FF_80x7", &[0xFF, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]);
        for lead in 0xF8u32..=0xFF {
            ct_case(
                api,
                &format!("lead5[{:02x}]", lead),
                &[lead as u8, 0x80, 0x80, 0x80, 0x80],
            );
        }

        p_line("-- above U+10FFFF (4-byte form) --");
        ct_case(api, "F4_90_80_80", &[0xF4, 0x90, 0x80, 0x80]);
        ct_case(api, "F4_8F_BF_BF", &[0xF4, 0x8F, 0xBF, 0xBF]);
        ct_case(api, "F5_80_80_80", &[0xF5, 0x80, 0x80, 0x80]);
        ct_case(api, "F7_BF_BF_BF", &[0xF7, 0xBF, 0xBF, 0xBF]);

        p_line("-- truncated multibyte at end of string --");
        ct_case(api, "trunc_C3", &[0xC3]);
        ct_case(api, "trunc_E2", &[0xE2]);
        ct_case(api, "trunc_E2_82", &[0xE2, 0x82]);
        ct_case(api, "trunc_F0", &[0xF0]);
        ct_case(api, "trunc_F0_9F", &[0xF0, 0x9F]);
        ct_case(api, "trunc_F0_9F_92", &[0xF0, 0x9F, 0x92]);
        ct_case(api, "trunc_F0_9F_92_A9", &[0xF0, 0x9F, 0x92, 0xA9]);
        ct_case(api, "trunc_ED", &[0xED]);
        ct_case(api, "trunc_ED_A0", &[0xED, 0xA0]);
        ct_case(api, "trunc_F8_80", &[0xF8, 0x80]);
        ct_case(api, "trunc_FF_80", &[0xFF, 0x80]);

        p_line("-- valid references --");
        ct_case(api, "valid_41", &[0x41]);
        ct_case(api, "valid_C3_A9", &[0xC3, 0xA9]);
        ct_case(api, "valid_E6_97_A5", &[0xE6, 0x97, 0xA5]);
        ct_case(api, "valid_EF_BF_BD", &[0xEF, 0xBF, 0xBD]);
        ct_case(api, "valid_F0_90_80_80", &[0xF0, 0x90, 0x80, 0x80]);
        ct_case(api, "valid_F4_8F_BF_BF", &[0xF4, 0x8F, 0xBF, 0xBF]);
    });
}

/* ------------------------------------------------------------------------- */
/* row 73: isalpharune / islowerrune / isupperrune / tolowerrune / toupper   */
/* ------------------------------------------------------------------------- */

unsafe fn case_detail(api: &Api, c: c_int) {
    let a = (api.jsU_isalpharune)(c);
    let lo = (api.jsU_islowerrune)(c);
    let up = (api.jsU_isupperrune)(c);
    let tl = (api.jsU_tolowerrune)(c);
    let tu = (api.jsU_toupperrune)(c);
    p_line(&format!(
        "c={} (0x{:x}) alpha={} lower={} upper={} tolower={} (0x{:x}) toupper={} (0x{:x})",
        c, c as u32, a, lo, up, tl, tl as u32, tu, tu as u32
    ));
}

#[test]
fn cfg73_runecase() {
    diff("cfg73_runecase", |api| unsafe {
        p_line("== row73 isalpha/islower/isupper/tolower/toupper ==");

        p_line("-- extremes --");
        for &c in &[
            i32::MIN,
            i32::MIN + 1,
            -0x110000i32,
            -1,
            -2,
            0x10FFFF,
            0x110000,
            0x110001,
            0x1FFFFF,
            0x7FFF_FFFE,
            i32::MAX,
        ] {
            case_detail(api, c);
        }

        p_line("-- sweep 0..=0x11000 --");
        let mut ck = Fnv::new();
        for c in 0i32..=0x11000 {
            let a = (api.jsU_isalpharune)(c);
            let lo = (api.jsU_islowerrune)(c);
            let up = (api.jsU_isupperrune)(c);
            let tl = (api.jsU_tolowerrune)(c);
            let tu = (api.jsU_toupperrune)(c);
            ck.add(c);
            ck.add(a);
            ck.add(lo);
            ck.add(up);
            ck.add(tl);
            ck.add(tu);
            if c < 0x600 && (a != 0 || lo != 0 || up != 0 || tl != c || tu != c) {
                p_line(&format!(
                    "c={} (0x{:x}) alpha={} lower={} upper={} tolower={} (0x{:x}) toupper={} (0x{:x})",
                    c, c as u32, a, lo, up, tl, tl as u32, tu, tu as u32
                ));
            }
            if (c & 0xFFF) == 0xFFF {
                p_ck(&format!("ck_73_blk_{:x}", c >> 12), ck.0);
            }
        }
        p_ck("ck_73_final", ck.0);
    });
}

/* ------------------------------------------------------------------------- */
/* row 74: jsU_tolowerrune_full / jsU_toupperrune_full                       */
/* ------------------------------------------------------------------------- */

/// Read a NUL-terminated Rune sequence (max 8 elements). Returns None on NULL.
unsafe fn seq(p: *const c_int) -> Option<Vec<c_int>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    for i in 0..8 {
        let x = *p.add(i);
        if x == 0 {
            break;
        }
        v.push(x);
    }
    Some(v)
}

fn seqstr(v: &Option<Vec<c_int>>) -> String {
    match v {
        None => "<null>".to_string(),
        Some(v) => {
            let mut s = format!("len={} [", v.len());
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    s.push(' ');
                }
                s += &format!("0x{:x}", *x as u32);
            }
            s.push(']');
            s
        }
    }
}

unsafe fn full_detail(api: &Api, c: c_int) {
    let lo = seq((api.jsU_tolowerrune_full)(c));
    let up = seq((api.jsU_toupperrune_full)(c));
    let tl = (api.jsU_tolowerrune)(c);
    let tu = (api.jsU_toupperrune)(c);
    p_line(&format!(
        "c={} (0x{:x}) tolower_full={} toupper_full={} plain_tolower=0x{:x} plain_toupper=0x{:x}",
        c,
        c as u32,
        seqstr(&lo),
        seqstr(&up),
        tl as u32,
        tu as u32
    ));
}

#[test]
fn cfg74_runecase_full() {
    diff("cfg74_runecase_full", |api| unsafe {
        p_line("== row74 tolowerrune_full / toupperrune_full ==");

        p_line("-- extremes --");
        for &c in &[
            i32::MIN,
            -1i32,
            -2,
            0,
            0x10FFFF,
            0x110000,
            0x1FFFFF,
            i32::MAX,
        ] {
            full_detail(api, c);
        }

        p_line("-- known multi-char cases --");
        let mut known: Vec<c_int> = vec![
            0xDF, 0x130, 0x149, 0x1F0, 0x390, 0x3B0, 0x587, 0x1F50, 0x1F52, 0x1F54, 0x1F56,
            0x1FBC, 0x1FCC, 0x1FFC,
        ];
        for c in 0xFB00..=0xFB17 {
            known.push(c);
        }
        for c in 0x1E96..=0x1E9A {
            known.push(c);
        }
        for c in 0x1F80..=0x1FFF {
            known.push(c);
        }
        for &c in known.iter() {
            full_detail(api, c);
        }

        p_line("-- sweep 0..=0x2000 (detail for every non-NULL result) --");
        let mut ck = Fnv::new();
        for c in 0i32..=0x2000 {
            let lop = (api.jsU_tolowerrune_full)(c);
            let upp = (api.jsU_toupperrune_full)(c);
            let lo = seq(lop);
            let up = seq(upp);
            let tl = (api.jsU_tolowerrune)(c);
            let tu = (api.jsU_toupperrune)(c);
            ck.add(c);
            ck.add(if lop.is_null() { 0 } else { 1 });
            ck.add(if upp.is_null() { 0 } else { 1 });
            ck.add(lo.as_ref().map(|v| v.len() as c_int).unwrap_or(-1));
            ck.add(up.as_ref().map(|v| v.len() as c_int).unwrap_or(-1));
            if let Some(v) = lo.as_ref() {
                for x in v.iter() {
                    ck.add(*x);
                }
            }
            if let Some(v) = up.as_ref() {
                for x in v.iter() {
                    ck.add(*x);
                }
            }
            ck.add(tl);
            ck.add(tu);
            if !lop.is_null() || !upp.is_null() {
                p_line(&format!(
                    "c={} (0x{:x}) tolower_full={} toupper_full={} plain_tolower=0x{:x} plain_toupper=0x{:x}",
                    c,
                    c as u32,
                    seqstr(&lo),
                    seqstr(&up),
                    tl as u32,
                    tu as u32
                ));
            }
        }
        p_ck("ck_74_final", ck.0);
    });
}

/* ------------------------------------------------------------------------- */
/* row 75: js_utflen / js_utfptrtoidx / js_runeat                            */
/* ------------------------------------------------------------------------- */

unsafe fn str_probe(api: &Api, J: JS, label: &str, bytes: &[u8]) {
    /* 40-byte zeroed buffer: NUL-terminated with slack so a truncated
     * multibyte sequence at the end can never read past the allocation. */
    let mut buf = [0i8; 40];
    assert!(bytes.len() < 32);
    for (i, b) in bytes.iter().enumerate() {
        buf[i] = *b as i8;
    }
    let s = buf.as_ptr();
    p_line(&format!("-- {} bytes=[{}] --", label, hexbytes(&buf[..bytes.len()])));
    let len = (api.js_utflen)(s);
    p_int("utflen", len);
    for off in 0..=bytes.len() {
        let r = (api.js_utfptrtoidx)(s, s.add(off));
        p_line(&format!("ptrtoidx off={} -> {}", off, r));
    }
    /* -2/-1 and the surrogate-splitting indices exercise both halves of the
     * surrogate-pair branch in js_runeat. */
    for &i in &[-2i32, -1, 0, 1, 2, 3, len / 2, len - 1, len, len + 1, len + 5] {
        let r = (api.js_runeat)(J, s, i);
        p_line(&format!("runeat i={} -> {} (0x{:x})", i, r, r as u32));
    }
}

#[test]
fn cfg75_utflen_ptrtoidx_runeat() {
    diff("cfg75_utflen_ptrtoidx_runeat", |api| unsafe {
        p_line("== row75 js_utflen / js_utfptrtoidx / js_runeat ==");
        let J = newstate(api, 0);

        str_probe(api, J, "empty", b"");
        str_probe(api, J, "a", b"a");
        str_probe(api, J, "abc", b"abc");
        str_probe(api, J, "hello_world", b"hello world");
        str_probe(api, J, "héllo", "héllo".as_bytes());
        str_probe(api, J, "日本語", "日本語".as_bytes());
        str_probe(api, J, "emoji", "a\u{1F600}b".as_bytes());
        str_probe(api, J, "emoji_only", "\u{1F4A9}".as_bytes());
        str_probe(api, J, "two_emoji", "\u{10000}\u{10FFFF}".as_bytes());
        str_probe(api, J, "mixed", "a\u{e9}\u{65e5}\u{1F600}z".as_bytes());
        str_probe(api, J, "inv_FF", &[0xFF]);
        str_probe(api, J, "inv_FF_mid", &[0x61, 0xFF, 0x62]);
        str_probe(api, J, "inv_C0_41", &[0xC0, 0x41]);
        str_probe(api, J, "inv_E0_80", &[0xE0, 0x80]);
        str_probe(api, J, "inv_E0_80_mid", &[0x61, 0xE0, 0x80, 0x62]);
        str_probe(api, J, "cont_80_embedded", &[0x61, 0x80, 0x62]);
        str_probe(api, J, "overlong_nul", &[0x61, 0xC0, 0x80, 0x62]);
        str_probe(api, J, "trunc_tail_E2_82", &[0x61, 0xE2, 0x82]);
        str_probe(api, J, "trunc_tail_F0_9F_92", &[0x61, 0xF0, 0x9F, 0x92]);
        str_probe(api, J, "surrogate_encoded", &[0xED, 0xA0, 0x80, 0x41]);
        str_probe(api, J, "all_leads", &[0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x92, 0xA9]);

        p_line("-- 200 random byte strings (seed 0x9E3779B97F4A7C15) --");
        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);
        for k in 0..200 {
            let n = 1 + (rng.range(15) as usize); /* 1..=15 */
            let mut bytes = [0u8; 16];
            for i in 0..n {
                bytes[i] = (1 + rng.range(255)) as u8; /* 1..=255, never NUL */
            }
            str_probe(api, J, &format!("rand[{}]", k), &bytes[..n]);
        }

        (api.js_freestate)(J);
    });
}


