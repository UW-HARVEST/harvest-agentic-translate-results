//! Phase C — C-vs-Rust differential tests for the **lexer / parser error paths**
//! (`ERRORS.md` rows 135-179).
//!
//! Everything goes through `json_loadb` on the two loaded `.so`s, so the exact
//! `json_error_t` produced by the C is the ground truth: `load_then_dump`
//! compares the round-tripped document AND the full error snapshot
//! (line, column, position, source, text, code) in one shot.
//!
//! The three cross-cutting mechanics from the top of `ERRORS.md` are what make
//! these rows interesting, and they are all observable through `ErrSnap`:
//!   * first error wins (`jsonp_error_vset` bails when `text[0] != 0`),
//!   * `error_set` appends `" near '<saved_text>'"` only for a non-empty
//!     saved_text of length <= 20,
//!   * an empty saved_text promotes `invalid_syntax` to
//!     `premature_end_of_input` and appends `" near end of file"`, except while
//!     the stream sits in `STREAM_STATE_ERROR` (UTF-8 failures).
//!
//! Rows covered: 135-141, 143-146, 148-156, 158, 160-163, 165-166, 168,
//! 170-173, 175-179. Rows 142/147/157/159/164/167/169/174 are `INT`/`OOM` and
//! belong to the OOM suite; rows 180+ are entry-point argument errors and are
//! deliberately out of scope here.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::c_char;

/// Dump flags used for every round trip.
///
/// `JSON_ENCODE_ANY` so a scalar root (`JSON_DECODE_ANY` cases) still dumps,
/// `JSON_SORT_KEYS` so hashtable iteration order can never be a source of
/// difference, `JSON_COMPACT` so the expected text is exactly the input text.
const DUMP: usize = JSON_ENCODE_ANY | JSON_SORT_KEYS | JSON_COMPACT;

// ---------------------------------------------------------------- plumbing

/// Readable rendering of an arbitrary byte string, for assertion messages.
fn hx(b: &[u8]) -> String {
    let mut s = String::from("b\"");
    for &c in b {
        match c {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7E => s.push(c as char),
            _ => s.push_str(&format!("\\x{:02x}", c)),
        }
    }
    s.push('"');
    s
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// Differentially compare `(round-trip dump, full ErrSnap)` for one input, then
/// return what the C did so callers can additionally pin the C's behaviour down.
#[track_caller]
fn diff_load(label: &str, bytes: &[u8], flags: usize) -> (Option<String>, ErrSnap) {
    diff(label, |lib: &Library| unsafe { load_then_dump(lib, bytes, flags, DUMP) });
    unsafe { load_then_dump(&libs().c, bytes, flags, DUMP) }
}

/// Compare C vs Rust *and* assert the C really rejects the input, so a row can
/// never silently degrade into a no-op.
#[track_caller]
fn rej(label: &str, bytes: &[u8], flags: usize) -> ErrSnap {
    let (v, e) = diff_load(label, bytes, flags);
    assert!(
        v.is_none(),
        "[{}] expected the C to REJECT {} (flags={:#x}) but it parsed to {:?}",
        label,
        hx(bytes),
        flags,
        v
    );
    assert_ne!(
        e.code, JSON_ERROR_UNKNOWN,
        "[{}] the C rejected {} but left no error code: {:?}",
        label,
        hx(bytes),
        e
    );
    e
}

/// Compare C vs Rust *and* assert the C accepts the input, returning its dump.
#[track_caller]
fn acc(label: &str, bytes: &[u8], flags: usize) -> String {
    let (v, e) = diff_load(label, bytes, flags);
    match v {
        Some(s) => s,
        None => panic!(
            "[{}] expected the C to ACCEPT {} (flags={:#x}) but it failed: {:?}",
            label,
            hx(bytes),
            flags,
            e
        ),
    }
}

// ================================================================ rows 135-136

/// Every byte `utf8_check_first` rejects outright: `0x80-0xBF` (bare
/// continuation), `0xC0`/`0xC1` (overlong ASCII lead), `0xF5-0xFF`.
fn bad_lead_bytes() -> Vec<u8> {
    let mut v: Vec<u8> = (0x80u8..=0xC1).collect();
    v.extend(0xF5u8..=0xFF);
    v
}

/// Valid lead byte + a sequence `utf8_check_full` rejects.
const BAD_SEQS: &[&[u8]] = &[
    // bad continuation byte(s)
    b"\xC2\x41",
    b"\xC2\x7F",
    b"\xC2\xC2",
    b"\xC2\xFF",
    b"\xC2\x00",
    b"\xDF\x20",
    b"\xE2\x82\x41",
    b"\xE2\x41\xAC",
    b"\xE2\xC2\xAC",
    b"\xF0\x9F\x98\x41",
    b"\xF0\x9F\x41\x80",
    b"\xF0\x41\x98\x80",
    b"\xED\x41\x80",
    // overlong encodings
    b"\xC0\x80",
    b"\xC1\xBF",
    b"\xE0\x80\x80",
    b"\xE0\x9F\xBF",
    b"\xF0\x80\x80\x80",
    b"\xF0\x8F\xBF\xBF",
    // surrogates encoded as UTF-8
    b"\xED\xA0\x80",
    b"\xED\xAF\xBF",
    b"\xED\xB0\x80",
    b"\xED\xBF\xBF",
    // beyond U+10FFFF
    b"\xF4\x90\x80\x80",
    b"\xF4\xA0\x80\x80",
    b"\xF4\xBF\xBF\xBF",
];

/// Well-formed UTF-8 that must keep working (guards against an over-eager check).
const GOOD_SEQS: &[&[u8]] = &[
    b"\xC2\x80",
    b"\xC3\xA9",
    b"\xDF\xBF",
    b"\xE0\xA0\x80",
    b"\xE2\x82\xAC",
    b"\xED\x9F\xBF",
    b"\xEE\x80\x80",
    b"\xEF\xBF\xBF",
    b"\xF0\x90\x80\x80",
    b"\xF0\x9F\x98\x80",
    b"\xF4\x8F\xBF\xBF",
];

#[test]
fn rows135_136_invalid_utf8() {
    // ---- row 135: lead byte rejected by utf8_check_first ------------------
    for b in bad_lead_bytes() {
        let one = [b];
        // As a bare token (stream_get fails from lex_scan's whitespace loop:
        // saved_text is still empty and the stream is in STREAM_STATE_ERROR, so
        // the message gets NO " near ..." suffix at all).
        rej(&format!("row135/0x{:02x} bare", b), &one, JSON_DECODE_ANY);
        rej(&format!("row135/0x{:02x} bare no-any", b), &one, 0);
        // As an array element / after a value / inside an object.
        rej(&format!("row135/0x{:02x} in array", b), &cat(&[b"[", &one, b"]"]), 0);
        rej(&format!("row135/0x{:02x} after elem", b), &cat(&[b"[1,", &one, b"]"]), 0);
        // Inside a string literal -- here saved_text already holds the opening
        // quote, so the suffix rule fires on a non-empty saved_text.
        rej(&format!("row135/0x{:02x} in string", b), &cat(&[b"[\"", &one, b"\"]"]), 0);
        rej(&format!("row135/0x{:02x} mid string", b), &cat(&[b"[\"ab", &one, b"cd\"]"]), 0);
        // Inside an object key.
        rej(&format!("row135/0x{:02x} in key", b), &cat(&[b"{\"", &one, b"\":1}"]), 0);
        // Right after an escape, where the lexer is mid-escape-sequence.
        rej(&format!("row135/0x{:02x} after esc", b), &cat(&[b"[\"\\", &one, b"\"]"]), 0);
        rej(&format!("row135/0x{:02x} in \\u", b), &cat(&[b"[\"\\u00", &one, b"0\"]"]), 0);
    }

    // ---- row 136: valid lead, utf8_check_full fails -----------------------
    for seq in BAD_SEQS {
        let l = hx(seq);
        rej(&format!("row136/{} bare", l), seq, JSON_DECODE_ANY);
        rej(&format!("row136/{} in array", l), &cat(&[b"[", seq, b"]"]), 0);
        rej(&format!("row136/{} in string", l), &cat(&[b"[\"", seq, b"\"]"]), 0);
        rej(&format!("row136/{} mid string", l), &cat(&[b"[\"x", seq, b"y\"]"]), 0);
        rej(&format!("row136/{} in key", l), &cat(&[b"{\"", seq, b"\":1}"]), 0);
    }

    // ---- row 136: sequence truncated by end-of-input ----------------------
    // `stream_get` keeps calling `get()` past the end; each EOF stores 0xFF into
    // the buffer, so `utf8_check_full` fails and the LEAD byte is reported.
    for seq in GOOD_SEQS {
        for keep in 1..seq.len() {
            let trunc = &seq[..keep];
            let l = format!("row136/trunc {} ({}/{})", hx(seq), keep, seq.len());
            rej(&format!("{} bare", l), trunc, JSON_DECODE_ANY);
            rej(&format!("{} in string", l), &cat(&[b"[\"", trunc]), 0);
            rej(&format!("{} in array", l), &cat(&[b"[", trunc]), 0);
            rej(&format!("{} in key", l), &cat(&[b"{\"", trunc]), 0);
        }
    }

    // ---- well-formed UTF-8 still parses ----------------------------------
    for seq in GOOD_SEQS {
        let l = hx(seq);
        acc(&format!("row136/ok {} in string", l), &cat(&[b"[\"", seq, b"\"]"]), 0);
        acc(&format!("row136/ok {} in key", l), &cat(&[b"{\"", seq, b"\":1}"]), 0);
        // As a bare token it is still an invalid *token*, but the failure must be
        // "invalid token", not a UTF-8 error -- and the message must contain the
        // re-assembled UTF-8 (lex_save_cached).
        rej(&format!("row136/ok {} bare token", l), seq, JSON_DECODE_ANY);
    }
}

// ================================================================ row 137

#[test]
fn row137_unterminated_string() {
    for (i, inp) in [
        &b"[\"abc"[..],
        &b"[\""[..],
        &b"[\"abc\\\""[..], // the escaped quote does not close the string
        &b"{\"a"[..],
        &b"[\"a\\"[..],   // EOF straight after a backslash
        &b"[\"a\\u"[..],  // EOF inside a \u escape
        &b"[\"a\\u1"[..],
        &b"[\"a\\u12"[..],
        &b"[\"a\\u123"[..],
        &b"[\"a\\u1234"[..], // escape complete, string not
        &b"[\"\\uD834"[..],
        &b"[\"\\uD834\\uDD1E"[..],
        &b"[1,\"x"[..],
        &b"{\"a\":\"x"[..],
        &b"[\"0123456789012345678901234567890123456789"[..], // saved_text > 20: no suffix
    ]
    .iter()
    .enumerate()
    {
        rej(&format!("row137/{} {}", i, hx(inp)), inp, 0);
        rej(&format!("row137/{} {} any", i, hx(inp)), inp, JSON_DECODE_ANY);
        // JSON_DISABLE_EOF_CHECK must not rescue an unterminated string.
        rej(&format!("row137/{} {} noeof", i, hx(inp)), inp, JSON_DISABLE_EOF_CHECK);
    }
    // A bare unterminated string at the root, with and without DECODE_ANY.
    rej("row137/bare \"abc", &b"\"abc"[..], JSON_DECODE_ANY);
    rej("row137/bare \"abc no-any", &b"\"abc"[..], 0);
}

// ================================================================ rows 138-139

#[test]
fn rows138_139_raw_control_bytes_in_string() {
    // Row 138/139: every raw control byte 0x00..=0x1F inside a string literal.
    // 0x0A is "unexpected newline"; every other byte is "control character 0x%x"
    // -- and 0x0A also moves stream.line/column, which ErrSnap pins down.
    for b in 0x00u8..=0x1F {
        let one = [b];
        let lab = format!("row{}/0x{:02x}", if b == 0x0A { 138 } else { 139 }, b);
        rej(&format!("{} lone in string", lab), &cat(&[b"[\"", &one, b"\"]"]), 0);
        rej(&format!("{} after char", lab), &cat(&[b"[\"a", &one, b"b\"]"]), 0);
        rej(&format!("{} in key", lab), &cat(&[b"{\"k", &one, b"\":1}"]), 0);
        rej(&format!("{} at eof", lab), &cat(&[b"[\"a", &one]), 0);
        // Control bytes must stay rejected under every decode flag, including
        // JSON_ALLOW_NUL (which only covers the \u0000 *escape*, not raw bytes).
        rej(&format!("{} allow_nul", lab), &cat(&[b"[\"", &one, b"\"]"]), JSON_ALLOW_NUL);
        rej(
            &format!("{} any+allow_nul", lab),
            &cat(&[b"\"", &one, b"\""]),
            JSON_DECODE_ANY | JSON_ALLOW_NUL,
        );
        // Same byte in an *escaped* form is fine for the ones JSON can express.
        rej(&format!("{} second line", lab), &cat(&[b"[\n1,\n\"", &one, b"\"]"]), 0);
    }
    // Sanity: the escaped forms of the same characters are accepted.
    assert_eq!(acc("row138/escaped newline ok", &b"[\"\\n\"]"[..], 0), "[\"\\n\"]");
    assert_eq!(acc("row139/escaped tab ok", &b"[\"\\t\"]"[..], 0), "[\"\\t\"]");
    assert_eq!(acc("row139/escaped 0x1f ok", &b"[\"\\u001f\"]"[..], 0), "[\"\\u001F\"]");
    // 0x7F is NOT a control character for the lexer (only 0x00..0x1F are).
    acc("row139/0x7f is not control", &cat(&[b"[\"", &[0x7Fu8], b"\"]"]), 0);
}

// ================================================================ rows 140-141

#[test]
fn rows140_141_invalid_escapes() {
    // ---- row 140: \u with too few / non-hex digits ------------------------
    for inp in [
        &b"[\"\\u\"]"[..],
        &b"[\"\\u1\"]"[..],
        &b"[\"\\u12\"]"[..],
        &b"[\"\\u123\"]"[..],
        &b"[\"\\u12z4\"]"[..],
        &b"[\"\\uz234\"]"[..],
        &b"[\"\\u1z34\"]"[..],
        &b"[\"\\u123z\"]"[..],
        &b"[\"\\uZZZZ\"]"[..],
        &b"[\"\\u 000\"]"[..],
        &b"[\"\\u00\"]"[..],
        &b"[\"\\u-123\"]"[..],
        &b"[\"\\u+123\"]"[..],
        &b"[\"\\u12.4\"]"[..],
        &b"[\"\\u00g0\"]"[..],
        &b"[\"a\\u00\\u0041\"]"[..],
        &b"{\"\\u00\":1}"[..],
    ] {
        rej(&format!("row140/{}", hx(inp)), inp, 0);
        rej(&format!("row140/{} allow_nul", hx(inp)), inp, JSON_ALLOW_NUL);
    }
    // Boundary: all four hex digits present, both cases, IS valid.
    assert_eq!(acc("row140/ok \\u0041", &b"[\"\\u0041\"]"[..], 0), "[\"A\"]");
    assert_eq!(acc("row140/ok \\uABCD", &b"[\"\\uabcd\"]"[..], 0), acc("row140/ok \\uabcd", &b"[\"\\uABCD\"]"[..], 0));

    // ---- row 141: every byte 0x20..=0x7E right after a backslash ----------
    // Only `"` `\` `/` `b` `f` `n` `r` `t` are complete escapes; `u` starts a
    // unicode escape (so `["\u"]` is a row-140 failure); everything else is
    // "invalid escape".
    const VALID: &[u8] = b"\"\\/bfnrt";
    for b in 0x20u8..=0x7E {
        let one = [b];
        let inp = cat(&[b"[\"\\", &one, b"\"]"]);
        let lab = format!("row141/backslash 0x{:02x} {:?}", b, b as char);
        let (v, _e) = diff_load(&lab, &inp, 0);
        if VALID.contains(&b) {
            assert!(v.is_some(), "[{}] the C should ACCEPT {}", lab, hx(&inp));
        } else {
            assert!(
                v.is_none(),
                "[{}] the C should REJECT {} but produced {:?}",
                lab,
                hx(&inp),
                v
            );
        }
        // Also as an object key and mid-string, and with the flags that could
        // plausibly interact.
        rej_or_acc(&format!("{} in key", lab), &cat(&[b"{\"\\", &one, b"\":1}"]), 0, VALID.contains(&b));
        rej_or_acc(
            &format!("{} mid string", lab),
            &cat(&[b"[\"a\\", &one, b"b\"]"]),
            JSON_ALLOW_NUL,
            VALID.contains(&b),
        );
    }
    // Non-ASCII and control bytes after a backslash too.
    for b in [0x00u8, 0x09, 0x0A, 0x0D, 0x1F, 0x7F, 0x80, 0xC2, 0xFF] {
        let one = [b];
        rej(
            &format!("row141/backslash 0x{:02x}", b),
            &cat(&[b"[\"\\", &one, b"\"]"]),
            0,
        );
    }
}

#[track_caller]
fn rej_or_acc(label: &str, bytes: &[u8], flags: usize, expect_ok: bool) {
    if expect_ok {
        acc(label, bytes, flags);
    } else {
        rej(label, bytes, flags);
    }
}

// ================================================================ rows 143-146

#[test]
fn rows143_146_surrogate_escapes() {
    const HIGH: &[u16] = &[0xD800, 0xD888, 0xDA00, 0xDBFF, 0xD83D];
    const LOW: &[u16] = &[0xDC00, 0xDD1E, 0xDE00, 0xDFFF];
    const NOT_LOW: &[u16] = &[0x0000, 0x0041, 0x00E9, 0xD7FF, 0xD800, 0xDBFF, 0xE000, 0xFFFF];

    for &h in HIGH {
        // row 145: lone high surrogate, end of string right after it.
        rej(&format!("row145/lone high U+{:04X}", h), format!("[\"\\u{:04X}\"]", h).as_bytes(), 0);
        // row 145: lone high surrogate followed by an ordinary character.
        rej(&format!("row145/high U+{:04X} + x", h), format!("[\"\\u{:04X}x\"]", h).as_bytes(), 0);
        rej(
            &format!("row145/high U+{:04X} + escape", h),
            format!("[\"\\u{:04X}\\n\"]", h).as_bytes(),
            0,
        );
        rej(
            &format!("row145/high U+{:04X} + utf8", h),
            &cat(&[format!("[\"\\u{:04X}", h).as_bytes(), b"\xE3\x88\x90\"]"]),
            0,
        );
        // row 143: high surrogate then a \u whose digits are not valid hex --
        // the first pass rejects the hex before the second pass ever runs, so
        // this is really a row-140 "invalid escape"; the differential compare
        // pins down which message the C actually emits.
        rej(
            &format!("row143/high U+{:04X} + \\uZZZZ", h),
            format!("[\"\\u{:04X}\\uZZZZ\"]", h).as_bytes(),
            0,
        );
        rej(
            &format!("row143/high U+{:04X} + \\u12", h),
            format!("[\"\\u{:04X}\\u12\"]", h).as_bytes(),
            0,
        );
        rej(
            &format!("row143/high U+{:04X} + \\u", h),
            format!("[\"\\u{:04X}\\u\"]", h).as_bytes(),
            0,
        );
        // row 144: high surrogate then a well-formed \u that is not a low one.
        for &n in NOT_LOW {
            rej(
                &format!("row144/U+{:04X}+U+{:04X}", h, n),
                format!("[\"\\u{:04X}\\u{:04X}\"]", h, n).as_bytes(),
                0,
            );
        }
        // row 146 mirror: valid pairs must still work.
        for &l in LOW {
            let inp = format!("[\"\\u{:04X}\\u{:04X}\"]", h, l);
            acc(&format!("row144/ok pair U+{:04X}+U+{:04X}", h, l), inp.as_bytes(), 0);
        }
    }

    // ---- row 146: lone LOW surrogate -------------------------------------
    for &l in LOW {
        rej(&format!("row146/lone low U+{:04X}", l), format!("[\"\\u{:04X}\"]", l).as_bytes(), 0);
        rej(&format!("row146/low U+{:04X} + x", l), format!("[\"\\u{:04X}x\"]", l).as_bytes(), 0);
        rej(
            &format!("row146/low U+{:04X} pair-order", l),
            format!("[\"\\u{:04X}\\uD800\"]", l).as_bytes(),
            0,
        );
        rej(
            &format!("row146/low U+{:04X} in key", l),
            format!("{{\"\\u{:04X}\":1}}", l).as_bytes(),
            0,
        );
    }
    // Just outside the surrogate block is fine.
    acc("row146/ok U+D7FF", &b"[\"\\uD7FF\"]"[..], 0);
    acc("row146/ok U+E000", &b"[\"\\uE000\"]"[..], 0);
}

// ================================================================ rows 148-154

#[test]
fn rows148_154_number_errors() {
    // ---- row 148: leading zero followed by a digit ------------------------
    for inp in [
        &b"[01]"[..],
        &b"[00]"[..],
        &b"[-012]"[..],
        &b"[-00]"[..],
        &b"[0123456789]"[..],
        &b"[00.5]"[..],
        &b"[01e5]"[..],
        &b"{\"a\":01}"[..],
    ] {
        rej(&format!("row148/{}", hx(inp)), inp, 0);
        rej(&format!("row148/{} int_as_real", hx(inp)), inp, JSON_DECODE_INT_AS_REAL);
    }
    // A single 0 (and -0) is of course fine, as is 0.5 / 0e1.
    assert_eq!(acc("row148/ok [0]", &b"[0]"[..], 0), "[0]");
    assert_eq!(acc("row148/ok [-0]", &b"[-0]"[..], 0), "[0]");
    acc("row148/ok [0.5]", &b"[0.5]"[..], 0);
    acc("row148/ok [0e1]", &b"[0e1]"[..], 0);

    // ---- row 149: `-` not followed by a digit ----------------------------
    for inp in [
        &b"[-]"[..],
        &b"[-x]"[..],
        &b"[-.5]"[..],
        &b"[- 1]"[..],
        &b"[-,]"[..],
        &b"[--1]"[..],
        &b"[-e5]"[..],
        &b"[-\"a\"]"[..],
        &b"[-"[..],
        &b"{\"a\":-}"[..],
    ] {
        rej(&format!("row149/{}", hx(inp)), inp, 0);
        rej(&format!("row149/{} int_as_real", hx(inp)), inp, JSON_DECODE_INT_AS_REAL);
    }

    // ---- rows 150/151: integer literals just past the json_int_t range ----
    // Exact boundaries: LLONG_MAX/LLONG_MIN are accepted, +/-1 past is not.
    assert_eq!(
        acc("row151/i64 max accepted", &b"[9223372036854775807]"[..], 0),
        "[9223372036854775807]"
    );
    assert_eq!(
        acc("row150/i64 min accepted", &b"[-9223372036854775808]"[..], 0),
        "[-9223372036854775808]"
    );
    let e = rej("row151/too big integer", &b"[9223372036854775808]"[..], 0);
    assert_eq!(e.code, JSON_ERROR_NUMERIC_OVERFLOW, "row151 C snapshot: {:?}", e);
    let e = rej("row150/too big negative integer", &b"[-9223372036854775809]"[..], 0);
    assert_eq!(e.code, JSON_ERROR_NUMERIC_OVERFLOW, "row150 C snapshot: {:?}", e);
    for inp in [
        &b"[9223372036854775808]"[..],
        &b"[9223372036854775809]"[..],
        &b"[18446744073709551616]"[..],
        &b"[99999999999999999999999999999]"[..],
        &b"[-9223372036854775809]"[..],
        &b"[-18446744073709551617]"[..],
        &b"[-99999999999999999999999999999]"[..],
        // 21+ digits: saved_text length > 20, so error_set drops the near-suffix.
        &b"[123456789012345678901]"[..],
        &b"[-123456789012345678901]"[..],
        &b"{\"a\":9223372036854775808}"[..],
    ] {
        rej(&format!("rows150-151/{}", hx(inp)), inp, 0);
        // ...and with JSON_DECODE_INT_AS_REAL the integer branch is skipped
        // entirely, so these stop being errors and become doubles.
        acc(
            &format!("rows150-151/{} int_as_real", hx(inp)),
            inp,
            JSON_DECODE_INT_AS_REAL,
        );
    }
    // INT_AS_REAL also turns the in-range integers into reals.
    acc("rows150-151/max int_as_real", &b"[9223372036854775807]"[..], JSON_DECODE_INT_AS_REAL);
    acc("rows150-151/min int_as_real", &b"[-9223372036854775808]"[..], JSON_DECODE_INT_AS_REAL);

    // ---- row 152: `.` not followed by a digit ----------------------------
    for inp in [
        &b"[1.]"[..],
        &b"[1.e5]"[..],
        &b"[0.]"[..],
        &b"[-1.]"[..],
        &b"[1..2]"[..],
        &b"[1.,2]"[..],
        &b"[1."[..],
        &b"[1.E-2]"[..],
        &b"[.5]"[..],
        &b"{\"a\":1.}"[..],
    ] {
        rej(&format!("row152/{}", hx(inp)), inp, 0);
        rej(&format!("row152/{} int_as_real", hx(inp)), inp, JSON_DECODE_INT_AS_REAL);
    }

    // ---- row 153: exponent without digits --------------------------------
    for inp in [
        &b"[1e]"[..],
        &b"[1E]"[..],
        &b"[1e+]"[..],
        &b"[1e-]"[..],
        &b"[1E-x]"[..],
        &b"[1E+x]"[..],
        &b"[1e++1]"[..],
        &b"[1e.]"[..],
        &b"[1e"[..],
        &b"[0.5e]"[..],
        &b"[-1.5E+]"[..],
        &b"{\"a\":1e}"[..],
    ] {
        rej(&format!("row153/{}", hx(inp)), inp, 0);
        rej(&format!("row153/{} int_as_real", hx(inp)), inp, JSON_DECODE_INT_AS_REAL);
    }

    // ---- row 154: the double itself overflows ----------------------------
    for inp in [
        &b"[1e999]"[..],
        &b"[-1e309]"[..],
        &b"[1e309]"[..],
        &b"[-1e999]"[..],
        &b"[1.7976931348623159e308]"[..],
        &b"[1e400]"[..],
        &b"[123456789e999999]"[..],
        &b"{\"a\":1e999}"[..],
    ] {
        let e = rej(&format!("row154/{}", hx(inp)), inp, 0);
        assert_eq!(e.code, JSON_ERROR_NUMERIC_OVERFLOW, "row154 {} snapshot {:?}", hx(inp), e);
        rej(&format!("row154/{} int_as_real", hx(inp)), inp, JSON_DECODE_INT_AS_REAL);
    }
    // Underflow (row 300) is NOT an error.
    acc("row154/underflow ok", &b"[1e-999]"[..], 0);
    acc("row154/DBL_MAX ok", &b"[1.7976931348623157e308]"[..], 0);
}

// ================================================================ rows 155-156

#[test]
fn rows155_156_bad_identifiers_and_stray_chars() {
    // ---- row 155: alphabetic identifier that is not true/false/null -------
    for id in [
        "nul", "tru", "fals", "TRUE", "False", "NULL", "foo", "nullx", "truee", "falsey", "n",
        "t", "f", "trueX", "nulll", "undefined", "NaN", "Infinity", "e", "E",
    ] {
        let arr = format!("[{}]", id);
        rej(&format!("row155/[{}]", id), arr.as_bytes(), 0);
        rej(&format!("row155/{} bare", id), id.as_bytes(), JSON_DECODE_ANY);
        rej(&format!("row155/{{\"a\":{}}}", id), format!("{{\"a\":{}}}", id).as_bytes(), 0);
        // the identifier is eaten whole for a clearer message; a trailing
        // non-alpha stops it, which ErrSnap's text pins down.
        rej(&format!("row155/[{}1]", id), format!("[{}1]", id).as_bytes(), 0);
    }
    // The three real keywords are accepted (case-sensitively).
    assert_eq!(acc("row155/ok true", &b"[true]"[..], 0), "[true]");
    assert_eq!(acc("row155/ok false", &b"[false]"[..], 0), "[false]");
    assert_eq!(acc("row155/ok null", &b"[null]"[..], 0), "[null]");
    // A 21+ char identifier trips the saved_text length > 20 rule.
    rej("row155/long identifier", &b"[abcdefghijklmnopqrstuvwxyz]"[..], 0);
    rej("row155/exactly 20", &b"[abcdefghijklmnopqrst]"[..], 0);
    rej("row155/exactly 21", &b"[abcdefghijklmnopqrstu]"[..], 0);

    // ---- row 156: any other leading character ---------------------------
    for b in [
        b'\'', b'#', b'@', b'+', b'.', b'*', b'`', b';', b'!', b'$', b'%', b'&', b'(', b')',
        b'/', b'<', b'=', b'>', b'?', b'\\', b'^', b'_', b'|', b'~', 0x7F,
    ] {
        let one = [b];
        let lab = format!("row156/0x{:02x} {:?}", b, b as char);
        rej(&format!("{} bare", lab), &one, JSON_DECODE_ANY);
        rej(&format!("{} bare no-any", lab), &one, 0);
        rej(&format!("{} in array", lab), &cat(&[b"[", &one, b"]"]), 0);
        rej(&format!("{} after elem", lab), &cat(&[b"[1,", &one, b"]"]), 0);
        rej(&format!("{} as key", lab), &cat(&[b"{", &one, b":1}"]), 0);
        rej(&format!("{} as value", lab), &cat(&[b"{\"a\":", &one, b"}"]), 0);
    }
    // Whitespace is skipped, not an error.
    acc("row156/ws ok", &b" \t\r\n[ 1 , 2 ]\t\n"[..], 0);
}

// ================================================================ rows 158-166

#[test]
fn rows158_166_object_errors() {
    // ---- row 158: key token is not a string ------------------------------
    for inp in [
        &b"{1:2}"[..],
        &b"{,}"[..],
        &b"{\"a\":1,}"[..],
        &b"{:}"[..],
        &b"{[]:1}"[..],
        &b"{{}:1}"[..],
        &b"{true:1}"[..],
        &b"{null:1}"[..],
        &b"{-1:2}"[..],
        &b"{\"a\":1,,\"b\":2}"[..],
        &b"{\"a\":1,2:3}"[..],
        &b"{]}"[..],
    ] {
        rej(&format!("row158/{}", hx(inp)), inp, 0);
    }
    assert_eq!(acc("row158/ok {}", &b"{}"[..], 0), "{}");
    assert_eq!(acc("row158/ok {\"a\":1}", &b"{\"a\":1}"[..], 0), "{\"a\":1}");

    // ---- row 160: NUL byte in an object key, even with JSON_ALLOW_NUL -----
    for inp in [
        &b"{\"\\u0000\":1}"[..],
        &b"{\"a\\u0000b\":1}"[..],
        &b"{\"\\u0000a\":1}"[..],
        &b"{\"a\\u0000\":1}"[..],
        &b"{\"ok\":1,\"a\\u0000b\":2}"[..],
    ] {
        let e = rej(&format!("row160/{}", hx(inp)), inp, 0);
        assert_eq!(e.code, JSON_ERROR_NULL_BYTE_IN_KEY, "row160 {} {:?}", hx(inp), e);
        // JSON_ALLOW_NUL must NOT rescue it: parse_object checks the key before
        // parse_value's JSON_ALLOW_NUL check ever comes into play.
        let e = rej(&format!("row160/{} allow_nul", hx(inp)), inp, JSON_ALLOW_NUL);
        assert_eq!(
            e.code, JSON_ERROR_NULL_BYTE_IN_KEY,
            "row160 {} with JSON_ALLOW_NUL {:?}",
            hx(inp),
            e
        );
        rej(
            &format!("row160/{} allow_nul+dups", hx(inp)),
            inp,
            JSON_ALLOW_NUL | JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
        );
    }

    // ---- row 161: duplicate keys -----------------------------------------
    for inp in [
        &b"{\"a\":1,\"a\":2}"[..],
        &b"{\"a\":1,\"b\":2,\"a\":3}"[..],
        &b"{\"\":1,\"\":2}"[..],
        &b"{\"a\":{\"b\":1,\"b\":2}}"[..],
        &b"{\"x\":1,\"y\":2,\"z\":3,\"x\":4}"[..],
    ] {
        let e = rej(&format!("row161/{} reject_dups", hx(inp)), inp, JSON_REJECT_DUPLICATES);
        assert_eq!(e.code, JSON_ERROR_DUPLICATE_KEY, "row161 {} {:?}", hx(inp), e);
        // Without the flag the later value silently wins.
        acc(&format!("row161/{} no flag", hx(inp)), inp, 0);
    }
    assert_eq!(
        acc("row161/last wins", &b"{\"a\":1,\"a\":2}"[..], 0),
        "{\"a\":2}"
    );
    acc("row161/no dup ok", &b"{\"a\":1,\"b\":2}"[..], JSON_REJECT_DUPLICATES);

    // ---- row 162: token after the key is not ':' -------------------------
    for inp in [
        &b"{\"a\" 1}"[..],
        &b"{\"a\",1}"[..],
        &b"{\"a\"}"[..],
        &b"{\"a\"[1]}"[..],
        &b"{\"a\"\"b\"}"[..],
        &b"{\"a\"::1}"[..],
        &b"{\"a\""[..],
    ] {
        rej(&format!("row162/{}", hx(inp)), inp, 0);
    }

    // ---- row 163: the member VALUE fails; the inner code is preserved -----
    let e = rej("row163/inner numeric overflow", &b"{\"a\":1e999}"[..], 0);
    assert_eq!(e.code, JSON_ERROR_NUMERIC_OVERFLOW, "row163 {:?}", e);
    let e = rej("row163/inner utf8", &cat(&[b"{\"a\":", &[0x80u8], b"}"]), 0);
    assert_eq!(e.code, JSON_ERROR_INVALID_UTF8, "row163 {:?}", e);
    for inp in [
        &b"{\"a\":}"[..],
        &b"{\"a\":,}"[..],
        &b"{\"a\":tru}"[..],
        &b"{\"a\":[}]}"[..],
        &b"{\"a\":{\"b\":}}"[..],
        &b"{\"a\":01}"[..],
    ] {
        rej(&format!("row163/{}", hx(inp)), inp, 0);
    }

    // ---- row 165: after a member, next token is neither ',' nor '}' -------
    for inp in [
        &b"{\"a\":1 \"b\":2}"[..],
        &b"{\"a\":1:2}"[..],
        &b"{\"a\":1]}"[..],
        &b"{\"a\":1 1}"[..],
        &b"{\"a\":1[]}"[..],
        &b"{\"a\":1,\"b\":2 \"c\":3}"[..],
    ] {
        rej(&format!("row165/{}", hx(inp)), inp, 0);
    }

    // ---- row 166: EOF before '}' -----------------------------------------
    for inp in [
        &b"{"[..],
        &b"{\"a\""[..],
        &b"{\"a\":"[..],
        &b"{\"a\":1"[..],
        &b"{\"a\":1,"[..],
        &b"{\"a\":1,\"b\""[..],
        &b"{\"a\":{"[..],
        &b"{\"a\":[1"[..],
        &b"{ "[..],
    ] {
        let e = rej(&format!("row166/{}", hx(inp)), inp, 0);
        assert_eq!(
            e.code, JSON_ERROR_PREMATURE_END_OF_INPUT,
            "row166 {} should be promoted to premature-end: {:?}",
            hx(inp),
            e
        );
        // JSON_DISABLE_EOF_CHECK does not help: the object itself is incomplete.
        rej(&format!("row166/{} noeof", hx(inp)), inp, JSON_DISABLE_EOF_CHECK);
    }
}

// ================================================================ rows 168-171

#[test]
fn rows168_171_array_errors() {
    // ---- row 168: element parse_value fails, incl. the trailing comma -----
    for inp in [
        &b"[1,]"[..],
        &b"[1,2,]"[..],
        &b"[,1]"[..],
        &b"[,]"[..],
        &b"[1,,2]"[..],
        &b"[1e999]"[..],
        &b"[tru]"[..],
        &b"[[1,]]"[..],
        &b"[1,:]"[..],
    ] {
        rej(&format!("row168/{}", hx(inp)), inp, 0);
    }
    assert_eq!(acc("row168/ok []", &b"[]"[..], 0), "[]");
    assert_eq!(acc("row168/ok [1,2]", &b"[1,2]"[..], 0), "[1,2]");

    // ---- row 170: after an element the token is neither ',' nor ']' -------
    for inp in [
        &b"[1 2]"[..],
        &b"[1:2]"[..],
        &b"[1}"[..],
        &b"[1 ]]"[..],
        &b"[[1] [2]]"[..],
        &b"[\"a\"\"b\"]"[..],
        &b"[1,2 3]"[..],
    ] {
        rej(&format!("row170/{}", hx(inp)), inp, 0);
    }

    // ---- row 171: EOF before ']' -----------------------------------------
    for inp in [
        &b"["[..],
        &b"[1"[..],
        &b"[1,"[..],
        &b"[1,2"[..],
        &b"[["[..],
        &b"[[]"[..],
        &b"[{}"[..],
        &b"[\"a\""[..],
        &b"[ "[..],
    ] {
        let e = rej(&format!("row171/{}", hx(inp)), inp, 0);
        assert_eq!(
            e.code, JSON_ERROR_PREMATURE_END_OF_INPUT,
            "row171 {} should be promoted to premature-end: {:?}",
            hx(inp),
            e
        );
        rej(&format!("row171/{} noeof", hx(inp)), inp, JSON_DISABLE_EOF_CHECK);
    }
}

// ================================================================ row 172

/// `n` nested containers wrapping a single scalar, balanced.
fn nested(n: usize, obj: bool) -> Vec<u8> {
    let (open, close): (&[u8], &[u8]) = if obj { (b"{\"a\":", b"}") } else { (b"[", b"]") };
    let mut v = Vec::with_capacity(n * (open.len() + 1) + 8);
    for _ in 0..n {
        v.extend_from_slice(open);
    }
    v.push(b'1');
    for _ in 0..n {
        v.extend_from_slice(close);
    }
    v
}

/// `n` nested containers with nothing inside (the innermost is `[]` / `{}`).
fn nested_empty(n: usize, obj: bool) -> Vec<u8> {
    let (open, close): (&[u8], &[u8]) = if obj { (b"{\"a\":", b"}") } else { (b"[", b"]") };
    let mut v = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(open);
    }
    v.extend_from_slice(if obj { b"{}" } else { b"[]" });
    for _ in 0..n {
        v.extend_from_slice(close);
    }
    v
}

unsafe fn parses(lib: &Library, bytes: &[u8]) -> bool {
    let loadb: Symbol<FnLoadb> = sym(lib, "json_loadb");
    let mut err = json_error_t::new();
    let j = loadb(bytes.as_ptr() as *const c_char, bytes.len(), 0, &mut err);
    let ok = !j.is_null();
    if ok {
        decref(lib, j);
    }
    ok
}

/// Binary-search the exact nesting boundary. `parses` is monotone in `n`, so
/// this returns `(last accepted n, first rejected n)`.
unsafe fn depth_boundary(lib: &Library, gen: fn(usize, bool) -> Vec<u8>, obj: bool) -> (usize, usize) {
    let mut lo = 1usize;
    let mut hi = 4096usize;
    assert!(parses(lib, &gen(lo, obj)), "nesting {} must be accepted", lo);
    assert!(!parses(lib, &gen(hi, obj)), "nesting {} must be rejected", hi);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if parses(lib, &gen(mid, obj)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo, hi)
}

#[test]
fn row172_parser_depth_limit() {
    // Deep nesting recurses in both libraries; run on a generous stack so the
    // test measures the parser's own limit and not the thread's.
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            for (obj, kind) in [(false, "array"), (true, "object")] {
                for (gen, what) in [
                    (nested as fn(usize, bool) -> Vec<u8>, "scalar-inside"),
                    (nested_empty as fn(usize, bool) -> Vec<u8>, "empty-inside"),
                ] {
                    let c = unsafe { depth_boundary(&libs().c, gen, obj) };
                    eprintln!(
                        "row172 depth boundary [{} {}]: last accepted = {}, first rejected = {}",
                        kind, what, c.0, c.1
                    );
                    // The boundary itself must be identical in both libraries.
                    diff(&format!("row172/{} {} boundary", kind, what), |lib: &Library| unsafe {
                        depth_boundary(lib, gen, obj)
                    });
                    // ...and so must the full behaviour at and around it.
                    for n in [c.0 - 1, c.0, c.1, c.1 + 1] {
                        let inp = gen(n, obj);
                        diff(
                            &format!("row172/{} {} n={}", kind, what, n),
                            |lib: &Library| unsafe {
                                let (d, e) = load_then_dump(lib, &inp, 0, DUMP);
                                // dumps of thousands of brackets are huge; compare
                                // a cheap fingerprint plus the error snapshot.
                                (d.as_ref().map(|s| (s.len(), s.as_bytes()[0], *s.as_bytes().last().unwrap())), e)
                            },
                        );
                    }
                    // The last accepted depth parses; the first rejected one
                    // fails with JSON_ERROR_STACK_OVERFLOW.
                    assert!(
                        unsafe { parses(&libs().c, &gen(c.0, obj)) },
                        "row172 {} {}: n={} should parse",
                        kind,
                        what,
                        c.0
                    );
                    let inp = gen(c.1, obj);
                    let (v, e) =
                        unsafe { load_then_dump(&libs().c, &inp, 0, DUMP) };
                    assert!(v.is_none());
                    assert_eq!(
                        e.code, JSON_ERROR_STACK_OVERFLOW,
                        "row172 {} {} n={} snapshot {:?}",
                        kind, what, c.1, e
                    );
                    // Same boundary and same message from the Rust side.
                    diff(&format!("row172/{} {} first-rejected", kind, what), |lib: &Library| unsafe {
                        load_then_dump(lib, &inp, 0, DUMP).1
                    });
                }
            }
            // JSON_PARSER_MAX_DEPTH is a value-depth counter, so `lex->depth`
            // also counts scalars: an array of many *siblings* is unaffected.
            let wide = {
                let mut v = vec![b'['];
                for i in 0..5000 {
                    if i > 0 {
                        v.push(b',');
                    }
                    v.push(b'1');
                }
                v.push(b']');
                v
            };
            diff("row172/5000 siblings not a depth error", |lib: &Library| unsafe {
                let (d, e) = load_then_dump(lib, &wide, 0, DUMP);
                (d.map(|s| s.len()), e)
            });
        })
        .expect("spawn")
        .join()
        .expect("row172 worker panicked");
}

// ================================================================ row 173

unsafe fn first_string_info(
    lib: &Library,
    bytes: &[u8],
    flags: usize,
) -> (Option<(usize, Vec<u8>)>, Option<String>, ErrSnap) {
    let loadb: Symbol<FnLoadb> = sym(lib, "json_loadb");
    let mut err = json_error_t::new();
    let j = loadb(bytes.as_ptr() as *const c_char, bytes.len(), flags, &mut err);
    if j.is_null() {
        return (None, None, err.snapshot());
    }
    let get: Symbol<FnArrGet> = sym(lib, "json_array_get");
    let slen: Symbol<FnSize> = sym(lib, "json_string_length");
    let sval: Symbol<FnStrVal> = sym(lib, "json_string_value");
    let s = get(j, 0);
    let info = if s.is_null() {
        None
    } else {
        let n = slen(s);
        let p = sval(s);
        let raw = if p.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(p as *const u8, n).to_vec()
        };
        Some((n, raw))
    };
    let dump = dumps_to_string(lib, j, DUMP);
    decref(lib, j);
    (info, dump, err.snapshot())
}

#[test]
fn row173_nul_character_in_string_value() {
    // Without JSON_ALLOW_NUL a decoded NUL is json_error_null_character.
    for inp in [
        &b"[\"\\u0000\"]"[..],
        &b"[\"a\\u0000b\"]"[..],
        &b"[\"\\u0000a\"]"[..],
        &b"[\"a\\u0000\"]"[..],
        &b"[\"\\u0000\\u0000\"]"[..],
        &b"{\"k\":\"a\\u0000b\"}"[..],
        &b"[[[\"\\u0000\"]]]"[..],
    ] {
        let e = rej(&format!("row173/{}", hx(inp)), inp, 0);
        assert_eq!(e.code, JSON_ERROR_NULL_CHARACTER, "row173 {} {:?}", hx(inp), e);
        // ...and it is accepted with the flag.
        acc(&format!("row173/{} allow_nul", hx(inp)), inp, JSON_ALLOW_NUL);
    }
    rej("row173/bare any", &b"\"a\\u0000b\""[..], JSON_DECODE_ANY);
    acc("row173/bare any+allow_nul", &b"\"a\\u0000b\""[..], JSON_DECODE_ANY | JSON_ALLOW_NUL);

    // With the flag the string really is 3 bytes long, NUL in the middle.
    diff("row173/allow_nul keeps the NUL", |lib: &Library| unsafe {
        first_string_info(lib, &b"[\"a\\u0000b\"]"[..], JSON_ALLOW_NUL)
    });
    let (info, dump, _e) =
        unsafe { first_string_info(&libs().c, &b"[\"a\\u0000b\"]"[..], JSON_ALLOW_NUL) };
    assert_eq!(info, Some((3usize, vec![b'a', 0u8, b'b'])), "row173 C string info");
    assert_eq!(dump.as_deref(), Some("[\"a\\u0000b\"]"), "row173 C round trip");

    // Length bookkeeping for a few more shapes.
    for inp in [&b"[\"\\u0000\"]"[..], &b"[\"\\u0000\\u0000x\"]"[..], &b"[\"x\\u0000\"]"[..]] {
        diff(&format!("row173/info {}", hx(inp)), |lib: &Library| unsafe {
            first_string_info(lib, inp, JSON_ALLOW_NUL)
        });
    }
}

// ================================================================ rows 175-176

#[test]
fn rows175_176_unexpected_tokens() {
    // Row 176: parse_value's `default:` -- token is '}' ']' ':' ',' or EOF.
    for inp in [
        &b"[}]"[..],
        &b"[,1]"[..],
        &b"[:]"[..],
        &b"[]]"[..],
        &b"{}}"[..],
        &b"[[}]]"[..],
        &b"{\"a\":}"[..],
        &b"{\"a\"::}"[..],
        &b"[1,}]"[..],
        &b"[1,]]"[..],
    ] {
        rej(&format!("row176/{}", hx(inp)), inp, 0);
        rej(&format!("row176/{} any", hx(inp)), inp, JSON_DECODE_ANY);
    }
    // Bare structural characters at the root -- with JSON_DECODE_ANY these reach
    // parse_value's default branch instead of parse_json's "'[' or '{' expected".
    for b in [b'}', b']', b':', b','] {
        let one = [b];
        rej(&format!("row176/bare {:?} any", b as char), &one, JSON_DECODE_ANY);
        rej(&format!("row176/bare {:?}", b as char), &one, 0);
        rej(
            &format!("row176/bare {:?} any+noeof", b as char),
            &one,
            JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
        );
    }
    // Row 175: TOKEN_INVALID with no error set by the lexer -- lex_scan returns
    // TOKEN_INVALID for a bad identifier without touching `error`, so
    // parse_value supplies "invalid token".
    for inp in [&b"[nul]"[..], &b"[zzz]"[..], &b"[Q]"[..]] {
        let e = rej(&format!("row175/{}", hx(inp)), inp, 0);
        assert_eq!(e.code, JSON_ERROR_INVALID_SYNTAX, "row175 {} {:?}", hx(inp), e);
    }
}

// ================================================================ rows 177-179

#[test]
fn rows177_179_root_and_eof_checks() {
    // ---- row 177: root is not a container and JSON_DECODE_ANY is unset ----
    for inp in [
        &b"42"[..],
        &b"\"str\""[..],
        &b"true"[..],
        &b"false"[..],
        &b"null"[..],
        &b"-1.5"[..],
        &b"0"[..],
        &b"1e5"[..],
        &b" \n 42 "[..],
        &b"\"\""[..],
    ] {
        rej(&format!("row177/{}", hx(inp)), inp, 0);
        // ...accepted as soon as JSON_DECODE_ANY is given.
        acc(&format!("row177/{} any", hx(inp)), inp, JSON_DECODE_ANY);
    }
    assert_eq!(acc("row177/42 any", &b"42"[..], JSON_DECODE_ANY), "42");
    assert_eq!(acc("row177/str any", &b"\"str\""[..], JSON_DECODE_ANY), "\"str\"");

    // ---- row 178: empty input --------------------------------------------
    for flags in [
        0usize,
        JSON_DECODE_ANY,
        JSON_DISABLE_EOF_CHECK,
        JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
        JSON_ALLOW_NUL | JSON_REJECT_DUPLICATES | JSON_DECODE_INT_AS_REAL,
    ] {
        let empty = &b" "[..0]; // non-NULL pointer, zero length
        assert_eq!(empty.len(), 0);
        let e = rej(&format!("row178/empty flags={:#x}", flags), empty, flags);
        assert_eq!(e.code, JSON_ERROR_PREMATURE_END_OF_INPUT, "row178 {:?}", e);
        // whitespace-only is the same thing
        rej(&format!("row178/ws-only flags={:#x}", flags), &b" \t\r\n"[..], flags);
    }

    // ---- row 179: trailing data and JSON_DISABLE_EOF_CHECK ---------------
    for inp in [
        &b"[1] [2]"[..],
        &b"{} x"[..],
        &b"[1]\"a\""[..],
        &b"[1]]"[..],
        &b"[1]}"[..],
        &b"[1],"[..],
        &b"[1]:"[..],
        &b"{}{}"[..],
        &b"[1] 2"[..],
        &b"[1]\x00"[..],
        &b"[1] \xC3\xA9"[..],
        &b"[1] \x80"[..],
    ] {
        rej(&format!("row179/{}", hx(inp)), inp, 0);
        // With JSON_DISABLE_EOF_CHECK the leading document is returned and the
        // trailing bytes are never even lexed.
        acc(&format!("row179/{} noeof", hx(inp)), inp, JSON_DISABLE_EOF_CHECK);
    }
    // Trailing whitespace only is fine either way.
    assert_eq!(acc("row179/trailing ws ok", &b"[1] \t\r\n"[..], 0), "[1]");
    // The same rule applies to a scalar root under JSON_DECODE_ANY.
    rej("row179/any 1 2", &b"1 2"[..], JSON_DECODE_ANY);
    acc("row179/any 1 2 noeof", &b"1 2"[..], JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK);
    // A NUL byte inside the buffer is a real byte for json_loadb (row 182 is
    // the json_loads variant, which treats it as EOF).
    rej("row179/nul after doc", &b"[1]\x00[2]"[..], 0);
    acc("row179/nul after doc noeof", &b"[1]\x00[2]"[..], JSON_DISABLE_EOF_CHECK);
}

// ================================================================ fuzz sweep

/// Valid documents used as fuzz seeds.
const SEED_DOCS: &[&str] = &[
    "[1,2,3]",
    "{\"a\":1,\"b\":[true,false,null]}",
    "[\"\\u00e9\",\"x\\ty\"]",
    "{\"k\":{\"n\":-1.5e3}}",
    "[]",
    "{}",
    "[0,-0,1e5,0.5,\"\"]",
    "[\"\\uD834\\uDD1E\"]",
    "{\"a\":\"b\",\"c\":[1,{\"d\":null}]}",
    "[9223372036854775807,-9223372036854775808]",
    "[\"\\\\\",\"\\\"\",\"\\/\",\"\\b\\f\\n\\r\\t\"]",
    "{\"\\u00e9\":[[[[1]]]]}",
    "[1.7976931348623157e308,1e-999,-0.0]",
    "[\"\\u0041\\u00e9\\u20ac\\ud83d\\ude00\"]",
    // Only valid without JSON_REJECT_DUPLICATES / with JSON_ALLOW_NUL, so these
    // two also drive the row-160/161 paths through the sweep.
    "{\"a\":1,\"a\":2,\"b\":3}",
    "{\"a\\u0000b\":1,\"c\":\"x\\u0000y\"}",
];

/// Bytes chosen to hit lexer edges as often as possible.
const BYTE_POOL: &[u8] = &[
    0x00, 0x01, 0x08, 0x09, 0x0A, 0x0D, 0x1F, 0x20, b'"', b'\\', b'/', b'{', b'}', b'[', b']',
    b':', b',', b'-', b'+', b'.', b'0', b'1', b'9', b'e', b'E', b'a', b'u', b'n', b't', b'f',
    b'x', b'z', 0x7F, 0x80, 0xA0, 0xBF, 0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xED, 0xEF, 0xF0, 0xF4,
    0xF5, 0xFE, 0xFF,
];

/// Token soup alphabet.
const TOKENS: &[&str] = &[
    "[", "]", "{", "}", ":", ",", "\"a\"", "\"\"", "1", "-1", "0", "01", "1e5", "1e999", "1.",
    "1e", "-", ".5", "true", "false", "null", "tru", "nul", "NULL", "\"\\u0000\"",
    "\"\\uD800\"", "\"\\uDC00\"", "\"\\uD834\\uDD1E\"", "\"\\uZZ\"", "\"\\x\"", "\"\\\"",
    "9223372036854775808", "-9223372036854775809", " ", "\t", "\n", "\r", "#", "'", "@", "+",
    "*", "`", ";", "\"", "\\", "\u{e9}", "\u{20ac}",
];

const FUZZ_FLAGS: &[usize] = &[
    0,
    JSON_DECODE_ANY,
    JSON_DECODE_ANY | JSON_ALLOW_NUL,
    JSON_DECODE_ANY | JSON_REJECT_DUPLICATES,
    JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
    JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
    JSON_DISABLE_EOF_CHECK,
    JSON_REJECT_DUPLICATES,
    JSON_ALLOW_NUL,
    JSON_DECODE_INT_AS_REAL,
    JSON_DECODE_ANY
        | JSON_ALLOW_NUL
        | JSON_REJECT_DUPLICATES
        | JSON_DISABLE_EOF_CHECK
        | JSON_DECODE_INT_AS_REAL,
];

/// Deterministic input generator: iteration `i` always yields the same
/// `(bytes, flags)`, so both libraries see byte-identical work.
fn fuzz_input(i: u64) -> (Vec<u8>, usize) {
    let mut rng = Rng::new(0xB16B00B5 ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let flags = FUZZ_FLAGS[rng.below(FUZZ_FLAGS.len() as u64) as usize];
    let doc = SEED_DOCS[rng.below(SEED_DOCS.len() as u64) as usize].as_bytes().to_vec();
    let pick = |r: &mut Rng| BYTE_POOL[r.below(BYTE_POOL.len() as u64) as usize];

    let bytes = match i % 8 {
        // flip one byte of a valid document
        0 => {
            let mut v = doc;
            if !v.is_empty() {
                let at = rng.below(v.len() as u64) as usize;
                v[at] = if rng.below(4) == 0 { rng.below(256) as u8 } else { pick(&mut rng) };
            }
            v
        }
        // insert one byte into a valid document
        1 => {
            let mut v = doc;
            let at = rng.below(v.len() as u64 + 1) as usize;
            let b = if rng.below(4) == 0 { rng.below(256) as u8 } else { pick(&mut rng) };
            v.insert(at, b);
            v
        }
        // delete one byte from a valid document
        2 => {
            let mut v = doc;
            if !v.is_empty() {
                let at = rng.below(v.len() as u64) as usize;
                v.remove(at);
            }
            v
        }
        // truncate a valid document at a random point
        3 => {
            let mut v = doc;
            let keep = rng.below(v.len() as u64 + 1) as usize;
            v.truncate(keep);
            v
        }
        // random printable ASCII
        4 => rng.ascii_string(24).into_bytes(),
        // uniformly random bytes, including 0x80-0xFF and NUL
        5 => {
            let n = rng.below(15) as usize;
            (0..n)
                .map(|_| if rng.below(2) == 0 { rng.below(256) as u8 } else { pick(&mut rng) })
                .collect()
        }
        // token soup
        6 => {
            let n = rng.below(8) as usize;
            let mut v = Vec::new();
            for _ in 0..n {
                v.extend_from_slice(TOKENS[rng.below(TOKENS.len() as u64) as usize].as_bytes());
            }
            v
        }
        // a valid document with random trailing / leading junk
        _ => {
            let mut v = Vec::new();
            let lead = rng.below(3) as usize;
            for _ in 0..lead {
                v.push(pick(&mut rng));
            }
            v.extend_from_slice(&doc);
            let trail = rng.below(4) as usize;
            for _ in 0..trail {
                v.push(pick(&mut rng));
            }
            v
        }
    };
    (bytes, flags)
}

#[test]
fn fuzz_lexer_and_parser_error_paths() {
    // The input description is part of the compared value so that a divergence
    // report names the exact bytes and flags.
    diff_n("fuzz/lexer+parser", 3000, |lib: &Library, i| unsafe {
        let (bytes, flags) = fuzz_input(i);
        let (dump, snap) = load_then_dump(lib, &bytes, flags, DUMP);
        (format!("{} flags={:#x}", hx(&bytes), flags), dump, snap)
    });

    // Non-vacuity guard: prove the sweep actually reaches a broad set of error
    // paths (and still parses a healthy number of documents) rather than, say,
    // producing 3000 empty buffers.
    let mut ok = 0usize;
    let mut bad = 0usize;
    let mut codes = std::collections::BTreeMap::<i32, usize>::new();
    let mut msgs = std::collections::BTreeSet::<String>::new();
    for i in 0..3000u64 {
        let (bytes, flags) = fuzz_input(i);
        let (dump, snap) = unsafe { load_then_dump(&libs().c, &bytes, flags, DUMP) };
        if dump.is_some() {
            ok += 1;
        } else {
            bad += 1;
            *codes.entry(snap.code).or_default() += 1;
            // strip the variable " near '...'" context to count distinct messages
            let t = snap.text;
            msgs.insert(t.split(" near ").next().unwrap_or("").to_string());
        }
    }
    eprintln!("fuzz coverage: {} parsed, {} rejected, codes={:?}", ok, bad, codes);
    eprintln!("fuzz distinct messages ({}): {:?}", msgs.len(), msgs);
    assert!(ok >= 200, "fuzz sweep parsed only {} inputs successfully", ok);
    assert!(bad >= 1000, "fuzz sweep rejected only {} inputs", bad);
    assert!(codes.len() >= 8, "fuzz sweep only reached error codes {:?}", codes);
    assert!(msgs.len() >= 60, "fuzz sweep only reached {} messages", msgs.len());
    for want in [
        JSON_ERROR_INVALID_UTF8,
        JSON_ERROR_PREMATURE_END_OF_INPUT,
        JSON_ERROR_END_OF_INPUT_EXPECTED,
        JSON_ERROR_INVALID_SYNTAX,
        JSON_ERROR_NULL_CHARACTER,
        JSON_ERROR_NULL_BYTE_IN_KEY,
        JSON_ERROR_DUPLICATE_KEY,
        JSON_ERROR_NUMERIC_OVERFLOW,
    ] {
        assert!(codes.contains_key(&want), "fuzz never produced code {}: {:?}", want, codes);
    }

    // Second sweep: the same generator but every input is also fed through the
    // remaining decode-flag combinations, so flag interactions get covered too.
    diff_n("fuzz/lexer+parser all-flags", 2000, |lib: &Library, i| unsafe {
        let (bytes, _) = fuzz_input(i * 7 + 3);
        let mut out = Vec::with_capacity(FUZZ_FLAGS.len());
        for &f in FUZZ_FLAGS {
            let (dump, snap) = load_then_dump(lib, &bytes, f, DUMP);
            out.push((f, dump, snap));
        }
        (hx(&bytes), out)
    });
}
