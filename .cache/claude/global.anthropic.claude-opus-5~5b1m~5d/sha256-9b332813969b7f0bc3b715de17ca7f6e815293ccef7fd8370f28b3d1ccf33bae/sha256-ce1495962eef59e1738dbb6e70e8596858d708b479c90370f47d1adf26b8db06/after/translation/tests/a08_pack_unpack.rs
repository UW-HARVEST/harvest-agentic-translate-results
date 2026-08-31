//! Differential tests for src/pack_unpack.c (plus `json_sprintf` /
//! `json_vsprintf`, whose only real driver is the same variadic machinery).
//!
//! `pack_unpack.c` is entirely driven by a hand-written scanner over the format
//! string PLUS a `va_list` walk, so a divergence can hide in three places:
//!   * the token stream (`next_token`/`prev_token`, the one-token pushback, the
//!     whitespace/`,`/`:` skipping and the line/column/pos bookkeeping which
//!     lands verbatim in `json_error_t`),
//!   * which varargs are consumed and in what ORDER (the `#`/`%`/`+`/`?`/`*`
//!     modifiers change the count, and `JSON_VALIDATE_ONLY` consumes the key
//!     varargs but NOT the value varargs), and
//!   * the resulting tree / the out-pointer writes.
//!
//! So every comparison here checks all three: the return value, the FULL
//! `json_error_t` byte image, and either a canonical dump of the packed tree or
//! the complete image of a poisoned out-pointer block.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};

// ===========================================================================
// Helpers
// ===========================================================================

/// A canonical dump: `JSON_SORT_KEYS` removes any dependence on hash order and
/// `JSON_ENCODE_ANY` lets a bare scalar be dumped too. `None` means either the
/// value was NULL or the dump itself failed (both are observable states and both
/// must match).
unsafe fn canon(api: &Api, j: *const json_t) -> Option<Vec<u8>> {
    if j.is_null() {
        return None;
    }
    let p = (api.json_dumps)(j, JSON_SORT_KEYS | JSON_ENCODE_ANY);
    let b = cbytes(p);
    jfree(api, p as *mut c_void);
    b
}

/// Parse `text` with the given library, so both libraries hold structurally
/// identical roots built by their own code.
unsafe fn load(api: &Api, text: &str) -> *mut json_t {
    let t = cs(text);
    // JSON_DECODE_ANY so a bare scalar can be a root; JSON_ALLOW_NUL so a root
    // string may contain an embedded NUL (json_string_length > strlen).
    let j = (api.json_loads)(
        t.as_ptr(),
        JSON_DECODE_ANY | JSON_ALLOW_NUL,
        std::ptr::null_mut(),
    );
    assert!(!j.is_null(), "{}: failed to parse root {text:?}", api.which);
    j
}

// --- the poisoned out-pointer block -------------------------------------------

const NSLOT: usize = 24;
const POISON_I32: c_int = 0x5A5A_5A5Au32 as c_int;
const POISON_I64: json_int_t = 0x5A5A_5A5A_5A5A_5A5Au64 as json_int_t;
const POISON_LEN: size_t = 0x5A5A_5A5A_5A5A_5A5A;
const POISON_DBITS: u64 = 0x5A5A_5A5A_5A5A_5A5A;

/// The sentinel a `const char **` slot starts at. It points at a real, readable
/// C string so that reading it back is always safe, and it is distinguishable
/// from anything jansson could store there.
static POISON_TEXT: &[u8; 11] = b"<<poison>>\0";

fn poison_str_ptr() -> *const c_char {
    POISON_TEXT.as_ptr() as *const c_char
}

/// The sentinel a `json_t **` slot starts at: a leaked, valid `json_t` shaped
/// like the `null` singleton (refcount `(size_t)-1`, so even an accidental
/// decref is a no-op). Shared by both libraries, so "untouched" is the same
/// pointer value on both sides.
fn sentinel_json() -> *mut json_t {
    static S: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        Box::leak(Box::new(json_t {
            type_: JSON_NULL,
            refcount: usize::MAX,
        })) as *mut json_t as usize
    }) as *mut json_t
}

/// One block of every out-pointer type `unpack()` can write through. Every slot
/// starts poisoned, so "the library did not write here" is distinguishable from
/// "it wrote a zero" — which is the whole point when checking that
/// `JSON_VALIDATE_ONLY` leaves value targets alone.
#[repr(C)]
struct Slots {
    ints: [c_int; NSLOT],
    i64s: [json_int_t; NSLOT],
    dbls: [f64; NSLOT],
    lens: [size_t; NSLOT],
    strs: [*const c_char; NSLOT],
    objs: [*mut json_t; NSLOT],
}

#[derive(PartialEq, Debug)]
struct SlotSummary {
    ints: Vec<c_int>,
    i64s: Vec<json_int_t>,
    dbits: Vec<u64>,
    lens: Vec<size_t>,
    /// `None` = untouched; `Some(None)` = library stored a NULL; otherwise the
    /// raw bytes of the stored string.
    strs: Vec<Option<Option<Vec<u8>>>>,
    /// `None` = untouched; otherwise (type tag, canonical dump).
    objs: Vec<Option<(c_int, Option<Vec<u8>>)>>,
}

impl Slots {
    fn poisoned() -> Slots {
        Slots {
            ints: [POISON_I32; NSLOT],
            i64s: [POISON_I64; NSLOT],
            dbls: [f64::from_bits(POISON_DBITS); NSLOT],
            lens: [POISON_LEN; NSLOT],
            strs: [poison_str_ptr(); NSLOT],
            objs: [sentinel_json(); NSLOT],
        }
    }

    unsafe fn summary(&self, api: &Api) -> SlotSummary {
        SlotSummary {
            ints: self.ints.to_vec(),
            i64s: self.i64s.to_vec(),
            dbits: self.dbls.iter().map(|d| d.to_bits()).collect(),
            lens: self.lens.to_vec(),
            strs: self
                .strs
                .iter()
                .map(|&p| {
                    if p == poison_str_ptr() {
                        None
                    } else {
                        Some(cbytes(p))
                    }
                })
                .collect(),
            objs: self
                .objs
                .iter()
                .map(|&p| {
                    if p == sentinel_json() {
                        None
                    } else if p.is_null() {
                        Some((-1, None))
                    } else {
                        Some((typeof_(p), canon(api, p)))
                    }
                })
                .collect(),
        }
    }

    /// Release the extra reference `O` took. Only the slots listed were filled
    /// by an `O` (an `o` must NOT be decref'd — it does not incref).
    unsafe fn decref_objs(&self, api: &Api, which: &[usize]) {
        for &i in which {
            let p = self.objs[i];
            if p != sentinel_json() && !p.is_null() {
                decref(api, p);
            }
        }
    }
}

// Slot address helpers. These take a raw `*mut Slots` rather than `&mut self`
// precisely so that several of them can appear in one call's argument list.
unsafe fn ip(s: *mut Slots, i: usize) -> *mut c_int {
    (*s).ints.as_mut_ptr().add(i)
}
unsafe fn i64p(s: *mut Slots, i: usize) -> *mut json_int_t {
    (*s).i64s.as_mut_ptr().add(i)
}
unsafe fn dp(s: *mut Slots, i: usize) -> *mut f64 {
    (*s).dbls.as_mut_ptr().add(i)
}
unsafe fn lp(s: *mut Slots, i: usize) -> *mut size_t {
    (*s).lens.as_mut_ptr().add(i)
}
unsafe fn sp(s: *mut Slots, i: usize) -> *mut *const c_char {
    (*s).strs.as_mut_ptr().add(i)
}
unsafe fn op(s: *mut Slots, i: usize) -> *mut *mut json_t {
    (*s).objs.as_mut_ptr().add(i)
}

// ===========================================================================
// Comparison macros
// ===========================================================================

/// `json_pack_ex` with library-independent varargs.
macro_rules! pk {
    ($c:expr, $r:expr, $flags:expr, $fmt:expr, [$($arg:expr),* $(,)?], $($ctx:tt)*) => {{
        let capi_: &Api = $c;
        let rapi_: &Api = $r;
        let fmt_ = &$fmt;
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (capi_.json_pack_ex)(&mut cerr, $flags, fmt_.as_ptr(), $($arg),*);
        let rj = (rapi_.json_pack_ex)(&mut rerr, $flags, fmt_.as_ptr(), $($arg),*);
        let ctx_ = format!($($ctx)*);
        diff_eq!(cj.is_null(), rj.is_null(), "json_pack_ex NULL-ness [{ctx_}]");
        diff_eq!(cerr.raw(), rerr.raw(), "json_pack_ex error image [{ctx_}]");
        diff_eq!(canon(capi_, cj), canon(rapi_, rj), "packed tree [{ctx_}]");
        decref(capi_, cj);
        decref(rapi_, rj);
    }};
}

/// `json_pack` — the 1-named-arg variadic shim, `error == NULL`, `flags == 0`.
macro_rules! pkn {
    ($c:expr, $r:expr, $fmt:expr, [$($arg:expr),* $(,)?], $($ctx:tt)*) => {{
        let capi_: &Api = $c;
        let rapi_: &Api = $r;
        let fmt_ = &$fmt;
        let cj = (capi_.json_pack)(fmt_.as_ptr(), $($arg),*);
        let rj = (rapi_.json_pack)(fmt_.as_ptr(), $($arg),*);
        let ctx_ = format!($($ctx)*);
        diff_eq!(cj.is_null(), rj.is_null(), "json_pack NULL-ness [{ctx_}]");
        diff_eq!(canon(capi_, cj), canon(rapi_, rj), "json_pack tree [{ctx_}]");
        decref(capi_, cj);
        decref(rapi_, rj);
    }};
}

/// `json_unpack_ex` against a root parsed from `$text` by each library.
/// `$sl` is bound to a `*mut Slots` for the duration of the argument list.
/// `$oslots` lists the object slots an `O` in the format increfs.
macro_rules! upk {
    ($c:expr, $r:expr, $text:expr, $flags:expr, $fmt:expr, $sl:ident,
     [$($arg:expr),* $(,)?], $oslots:expr, $($ctx:tt)*) => {{
        let capi_: &Api = $c;
        let rapi_: &Api = $r;
        let croot = load(capi_, $text);
        let rroot = load(rapi_, $text);
        let fmt_ = &$fmt;
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let mut cslots = Slots::poisoned();
        let mut rslots = Slots::poisoned();
        let cret = {
            let $sl: *mut Slots = &mut cslots;
            let _ = $sl;
            (capi_.json_unpack_ex)(croot, &mut cerr, $flags, fmt_.as_ptr(), $($arg),*)
        };
        let rret = {
            let $sl: *mut Slots = &mut rslots;
            let _ = $sl;
            (rapi_.json_unpack_ex)(rroot, &mut rerr, $flags, fmt_.as_ptr(), $($arg),*)
        };
        let ctx_ = format!($($ctx)*);
        diff_eq!(cret, rret, "json_unpack_ex return [{ctx_}]");
        diff_eq!(cerr.raw(), rerr.raw(), "json_unpack_ex error image [{ctx_}]");
        diff_eq!(
            cslots.summary(capi_),
            rslots.summary(rapi_),
            "out-pointer targets [{ctx_}]"
        );
        cslots.decref_objs(capi_, $oslots);
        rslots.decref_objs(rapi_, $oslots);
        decref(capi_, croot);
        decref(rapi_, rroot);
    }};
}

/// `json_unpack` — the 2-named-arg variadic shim, `error == NULL`, `flags == 0`.
macro_rules! upkn {
    ($c:expr, $r:expr, $text:expr, $fmt:expr, $sl:ident,
     [$($arg:expr),* $(,)?], $oslots:expr, $($ctx:tt)*) => {{
        let capi_: &Api = $c;
        let rapi_: &Api = $r;
        let croot = load(capi_, $text);
        let rroot = load(rapi_, $text);
        let fmt_ = &$fmt;
        let mut cslots = Slots::poisoned();
        let mut rslots = Slots::poisoned();
        let cret = {
            let $sl: *mut Slots = &mut cslots;
            let _ = $sl;
            (capi_.json_unpack)(croot, fmt_.as_ptr(), $($arg),*)
        };
        let rret = {
            let $sl: *mut Slots = &mut rslots;
            let _ = $sl;
            (rapi_.json_unpack)(rroot, fmt_.as_ptr(), $($arg),*)
        };
        let ctx_ = format!($($ctx)*);
        diff_eq!(cret, rret, "json_unpack return [{ctx_}]");
        diff_eq!(
            cslots.summary(capi_),
            rslots.summary(rapi_),
            "out-pointer targets [{ctx_}]"
        );
        cslots.decref_objs(capi_, $oslots);
        rslots.decref_objs(rapi_, $oslots);
        decref(capi_, croot);
        decref(rapi_, rroot);
    }};
}

// ===========================================================================
// Row 227 — NULL / empty format string
// ===========================================================================

#[test]
fn r227_pack_null_and_empty_format() {
    let (c, r) = both();
    unsafe {
        // The `!fmt || !*fmt` guard runs BEFORE jsonp_error_init(error, NULL),
        // so it installs source "<format>" and line/column/position -1/-1/0.
        for flags in [0, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, flags, std::ptr::null());
            let rj = (r.json_pack_ex)(&mut rerr, flags, std::ptr::null());
            diff_eq!(cj.is_null(), rj.is_null(), "pack_ex(NULL fmt) NULL-ness flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "pack_ex(NULL fmt) error flags={flags}");
            assert!(cj.is_null(), "C: NULL format must fail");
            assert_eq!(cerr.snapshot().3, "<format>", "C: source");
            assert_eq!(cerr.snapshot().4, "NULL or empty format string", "C: text");
            assert_eq!((cerr.line, cerr.column, cerr.position), (-1, -1, 0), "C: position");
            assert_eq!(cerr.code(), JSON_ERROR_INVALID_ARGUMENT, "C: code");

            let empty = cs("");
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, flags, empty.as_ptr());
            let rj = (r.json_pack_ex)(&mut rerr, flags, empty.as_ptr());
            diff_eq!(cj.is_null(), rj.is_null(), "pack_ex(\"\") NULL-ness flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "pack_ex(\"\") error flags={flags}");
        }

        // json_pack passes error == NULL: the guard must cope with that too.
        let cj = (c.json_pack)(std::ptr::null::<c_char>());
        let rj = (r.json_pack)(std::ptr::null::<c_char>());
        diff_eq!(cj.is_null(), rj.is_null(), "json_pack(NULL)");
        let empty = cs("");
        let cj = (c.json_pack)(empty.as_ptr());
        let rj = (r.json_pack)(empty.as_ptr());
        diff_eq!(cj.is_null(), rj.is_null(), "json_pack(\"\")");

        // ... and through the real va_list entry point.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vpack_ex");
        let rfn = sym_addr("Rust", b"json_vpack_ex");
        for fmt in [std::ptr::null::<c_char>(), empty.as_ptr()] {
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cfn, &mut cerr, 0, fmt);
            let rj = (sh.vpack_ex)(rfn, &mut rerr, 0, fmt);
            diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex bad fmt NULL-ness");
            diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex bad fmt error image");
        }
    }
}

// ===========================================================================
// Rows 228-230 — the scalar pack format characters n b i I f
// ===========================================================================

#[test]
fn r228_pack_null_and_boolean() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0228);
    unsafe {
        pk!(c, r, 0, cs("n"), [], "pack n");
        // `n` returns the null SINGLETON: refcount is (size_t)-1.
        let mut e = json_error_t::new();
        let f = cs("n");
        let cj = (c.json_pack_ex)(&mut e, 0, f.as_ptr());
        let rj = (r.json_pack_ex)(&mut e, 0, f.as_ptr());
        diff_eq!(cj == (c.json_null)(), rj == (r.json_null)(), "pack n is the singleton");
        diff_eq!((*cj).refcount, (*rj).refcount, "pack n refcount");
        assert_eq!((*cj).refcount, usize::MAX, "C: singleton refcount");

        // `b` takes an int; ANY non-zero is true (not just 1).
        for v in [0i32, 1, 2, -1, i32::MAX, i32::MIN, 0x1_0000, 0xFF00] {
            pk!(c, r, 0, cs("b"), [v as c_int], "pack b with {v}");
            let f = cs("b");
            let cj = (c.json_pack_ex)(&mut e, 0, f.as_ptr(), v as c_int);
            let rj = (r.json_pack_ex)(&mut e, 0, f.as_ptr(), v as c_int);
            let cwant = if v != 0 { (c.json_true)() } else { (c.json_false)() };
            let rwant = if v != 0 { (r.json_true)() } else { (r.json_false)() };
            diff_eq!(cj == cwant, rj == rwant, "pack b {v} is the right singleton");
            diff_eq!((*cj).refcount, (*rj).refcount, "pack b {v} refcount");
        }
        for _ in 0..500 {
            let v = rng.next_u32() as c_int;
            pk!(c, r, 0, cs("[b,b]"), [v, !v], "pack [b,b] with {v}");
        }
    }
}

#[test]
fn r229_pack_integers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0229);
    unsafe {
        // `i` reads an `int` and widens to json_int_t (sign extension matters).
        for v in [0i32, 1, -1, i32::MAX, i32::MIN, 12345, -12345] {
            pk!(c, r, 0, cs("i"), [v as c_int], "pack i {v}");
        }
        // `I` reads a json_int_t directly.
        for v in [
            0i64,
            1,
            -1,
            i64::MAX,
            i64::MIN,
            i32::MAX as i64,
            i32::MIN as i64,
            i32::MAX as i64 + 1,
            i32::MIN as i64 - 1,
        ] {
            pk!(c, r, 0, cs("I"), [v as json_int_t], "pack I {v}");
        }
        for _ in 0..2000 {
            let a = rng.next_u32() as c_int;
            let b = rng.json_int();
            pk!(c, r, 0, cs("[i,I,i,I]"), [a, b, !a, b.wrapping_neg()], "pack ints {a} {b}");
        }
    }
}

#[test]
fn r230_pack_reals() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0230);
    unsafe {
        for v in [
            0.0f64,
            -0.0,
            1.0,
            -1.0,
            0.1,
            1e308,
            -1e308,
            5e-324,
            f64::MAX,
            f64::MIN,
            1.0 / 3.0,
            std::f64::consts::PI,
        ] {
            pk!(c, r, 0, cs("f"), [v], "pack f {v:e}");
        }
        // json_real_set rejects non-finite values -> json_error_numeric_overflow
        // with source <args>.
        for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            pk!(c, r, 0, cs("f"), [v], "pack f non-finite {v}");
            let f = cs("f");
            let mut cerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), v);
            assert!(cj.is_null(), "C: non-finite must fail");
            assert_eq!(cerr.snapshot().3, "<args>", "C: source for non-finite");
            assert_eq!(cerr.snapshot().4, "Invalid floating point value", "C: text");
            assert_eq!(cerr.code(), JSON_ERROR_NUMERIC_OVERFLOW, "C: code");
            // The same failure nested, so the unwind path is exercised too.
            pk!(c, r, 0, cs("{s:[i,f]}"), [cs("k").as_ptr(), 1 as c_int, v], "nested bad f");
        }
        for _ in 0..3000 {
            let a = rng.real();
            let b = rng.real();
            pk!(c, r, 0, cs("[f,f]"), [a, b], "pack [f,f] {a:e} {b:e}");
        }
    }
}

// ===========================================================================
// Rows 231-238 — `s` and every string modifier (# % + ? *)
// ===========================================================================

#[test]
fn r231_pack_plain_strings() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0231);
    unsafe {
        // read_string's fast path: the next token is not #/%/+, so no strbuffer
        // is used, `ours == 0`, and json_stringn_nocheck takes the pointer.
        let fixed = [
            String::from(""),
            String::from("a"),
            String::from("hello world"),
            String::from("\u{e9}\u{20ac}\u{1f600}"),
            String::from("tab\there\nnewline\"quote\\back/slash"),
            "x".repeat(1024),
            "\u{1f600}".repeat(300),
        ];
        for s in &fixed {
            let cstr = cs(s);
            pk!(c, r, 0, cs("s"), [cstr.as_ptr()], "pack s {:?}", s);
            pk!(c, r, 0, cs("[s,s]"), [cstr.as_ptr(), cstr.as_ptr()], "pack [s,s] {:?}", s);
        }
        for i in 0..3000 {
            let s = rng.utf8_string(24);
            if s.as_bytes().contains(&0) {
                continue; // an interior NUL needs `s#`, tested in r233
            }
            let cstr = cs(&s);
            pk!(c, r, 0, cs("s"), [cstr.as_ptr()], "iter {i}: pack s {:?}", s);
        }
    }
}

#[test]
fn r232_pack_string_null_and_invalid_utf8() {
    let (c, r) = both();
    unsafe {
        // NULL and !optional -> json_error_null_value "NULL string", <args>.
        pk!(c, r, 0, cs("s"), [std::ptr::null::<c_char>()], "pack s NULL");
        let f = cs("s");
        let mut cerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), std::ptr::null::<c_char>());
        assert!(cj.is_null(), "C: NULL string must fail");
        assert_eq!(cerr.snapshot().3, "<args>", "C: source");
        assert_eq!(cerr.snapshot().4, "NULL string", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_NULL_VALUE, "C: code");

        // Invalid UTF-8 -> json_error_invalid_utf8 "Invalid UTF-8 string".
        let bad: Vec<Vec<u8>> = vec![
            b"\xff".to_vec(),
            b"\xfe\xfe".to_vec(),
            b"a\x80b".to_vec(),
            b"\xc3".to_vec(),          // truncated 2-byte
            b"\xe2\x82".to_vec(),      // truncated 3-byte
            b"\xed\xa0\x80".to_vec(),  // surrogate
            b"\xf4\x90\x80\x80".to_vec(), // > U+10FFFF
            b"\xc0\x80".to_vec(),      // overlong NUL
        ];
        for b in &bad {
            let buf = cs_bytes(b);
            pk!(c, r, 0, cs("s"), [buf.as_ptr()], "pack s invalid utf8 {b:?}");
            pk!(c, r, 0, cs("[i,s]"), [1 as c_int, buf.as_ptr()], "nested invalid utf8 {b:?}");
        }
        // ... and as an object KEY (same read_string, purpose "object key").
        for b in &bad {
            let buf = cs_bytes(b);
            pk!(c, r, 0, cs("{s:i}"), [buf.as_ptr(), 1 as c_int], "pack key invalid utf8 {b:?}");
        }
        pk!(c, r, 0, cs("{s:i}"), [std::ptr::null::<c_char>(), 1 as c_int], "pack NULL key");
    }
}

#[test]
fn r233_pack_string_with_explicit_length() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0233);
    unsafe {
        // `s#` reads an int length, `s%` a size_t. Both take the strbuffer path
        // (`ours == 1` -> jsonp_stringn_nocheck_own).
        let samples: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"hello".to_vec(),
            b"a\0b".to_vec(),
            b"\0".to_vec(),
            b"ab\0cd\0ef".to_vec(),
            "\u{e9}\u{20ac}".to_string().into_bytes(),
            vec![b'z'; 2048],
        ];
        for s in &samples {
            let buf = cs_bytes(s);
            for len in 0..=s.len() {
                pk!(c, r, 0, cs("s#"), [buf.as_ptr(), len as c_int], "pack s# {s:?} len {len}");
                pk!(c, r, 0, cs("s%"), [buf.as_ptr(), len as size_t], "pack s% {s:?} len {len}");
                // s# and s% must agree with each other for the same value.
                let f1 = cs("s#");
                let f2 = cs("s%");
                let mut e = json_error_t::new();
                let a = (c.json_pack_ex)(&mut e, 0, f1.as_ptr(), buf.as_ptr(), len as c_int);
                let b = (c.json_pack_ex)(&mut e, 0, f2.as_ptr(), buf.as_ptr(), len as size_t);
                assert_eq!(canon(c, a), canon(c, b), "C: s# and s% agree on {s:?}/{len}");
                decref(c, a);
                decref(c, b);
            }
        }
        for i in 0..2000 {
            let s = rng.ascii_string(20);
            let buf = cs_bytes(s.as_bytes());
            let len = rng.below(s.len() + 1);
            pk!(c, r, 0, cs("{s#:s%}"),
                [buf.as_ptr(), len as c_int, buf.as_ptr(), len as size_t],
                "iter {i}: s#/s% key+value {s:?} len {len}");
        }
    }
}

#[test]
fn r234_pack_string_concatenation() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0234);
    unsafe {
        let a = cs("foo");
        let b = cs("bar");
        let e = cs("");
        let big = cs(&"q".repeat(700));
        pk!(c, r, 0, cs("s+"), [a.as_ptr(), b.as_ptr()], "pack s+ 2 parts");
        pk!(c, r, 0, cs("s++"), [a.as_ptr(), b.as_ptr(), a.as_ptr()], "pack s++ 3 parts");
        pk!(c, r, 0, cs("s+++"), [a.as_ptr(), b.as_ptr(), a.as_ptr(), b.as_ptr()],
            "pack s+++ 4 parts");
        pk!(c, r, 0, cs("s+"), [e.as_ptr(), b.as_ptr()], "pack s+ empty first");
        pk!(c, r, 0, cs("s+"), [a.as_ptr(), e.as_ptr()], "pack s+ empty second");
        pk!(c, r, 0, cs("s+"), [e.as_ptr(), e.as_ptr()], "pack s+ both empty");
        // >1KiB total forces strbuffer growth.
        pk!(c, r, 0, cs("s++"), [big.as_ptr(), big.as_ptr(), big.as_ptr()], "pack s+ >1KiB");
        // As an object key as well.
        pk!(c, r, 0, cs("{s+:i}"), [a.as_ptr(), b.as_ptr(), 1 as c_int], "pack s+ key");
        for i in 0..1500 {
            let p = cs(&rng.ascii_string(12).replace('\0', "x"));
            let q = cs(&rng.ascii_string(12).replace('\0', "x"));
            pk!(c, r, 0, cs("s+"), [p.as_ptr(), q.as_ptr()], "iter {i}: random s+");
        }
    }
}

#[test]
fn r235_pack_concat_with_mixed_lengths() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0235);
    unsafe {
        let a = cs("abcdef");
        let b = cs("012345");
        // Every mix of per-part explicit lengths. The order the length varargs
        // are consumed is what makes these interesting.
        pk!(c, r, 0, cs("s+#"), [a.as_ptr(), b.as_ptr(), 3 as c_int], "s+#");
        pk!(c, r, 0, cs("s#+#"), [a.as_ptr(), 2 as c_int, b.as_ptr(), 4 as c_int], "s#+#");
        pk!(c, r, 0, cs("s+%"), [a.as_ptr(), b.as_ptr(), 5 as size_t], "s+%");
        pk!(c, r, 0, cs("s%+%"), [a.as_ptr(), 1 as size_t, b.as_ptr(), 6 as size_t], "s%+%");
        pk!(c, r, 0, cs("s#+%"), [a.as_ptr(), 4 as c_int, b.as_ptr(), 2 as size_t], "s#+%");
        pk!(c, r, 0, cs("s%+#"), [a.as_ptr(), 4 as size_t, b.as_ptr(), 2 as c_int], "s%+#");
        pk!(c, r, 0, cs("s#+"), [a.as_ptr(), 4 as c_int, b.as_ptr()], "s#+");
        pk!(c, r, 0, cs("s%+"), [a.as_ptr(), 4 as size_t, b.as_ptr()], "s%+");
        pk!(c, r, 0, cs("s#+#+#"),
            [a.as_ptr(), 1 as c_int, b.as_ptr(), 2 as c_int, a.as_ptr(), 3 as c_int], "s#+#+#");
        pk!(c, r, 0, cs("s%+#+%"),
            [a.as_ptr(), 1 as size_t, b.as_ptr(), 2 as c_int, a.as_ptr(), 3 as size_t],
            "s%+#+%");
        // Same shapes as object keys.
        pk!(c, r, 0, cs("{s#+#:i}"),
            [a.as_ptr(), 2 as c_int, b.as_ptr(), 3 as c_int, 7 as c_int], "{{s#+#:i}}");
        for i in 0..1500 {
            let la = rng.below(7);
            let lb = rng.below(7);
            pk!(c, r, 0, cs("s#+%"),
                [a.as_ptr(), la as c_int, b.as_ptr(), lb as size_t],
                "iter {i}: s#+% {la}/{lb}");
        }
    }
}

#[test]
fn r236_pack_concat_errors_and_utf8_across_parts() {
    let (c, r) = both();
    unsafe {
        let a = cs("foo");
        let nul: *const c_char = std::ptr::null();
        // A NULL part sets has_error but the loop keeps consuming varargs, so
        // the remaining `+` parts are still read.
        pk!(c, r, 0, cs("s+"), [nul, a.as_ptr()], "s+ NULL first");
        pk!(c, r, 0, cs("s+"), [a.as_ptr(), nul], "s+ NULL second");
        pk!(c, r, 0, cs("s++"), [a.as_ptr(), nul, a.as_ptr()], "s+ NULL middle");
        pk!(c, r, 0, cs("s#+#"), [a.as_ptr(), 2 as c_int, nul, 2 as c_int], "s#+# NULL second");

        // A multi-byte sequence SPLIT across two parts is legal: the UTF-8 check
        // runs on the concatenated buffer only.
        let e_hi = cs_bytes(&[0xc3]);
        let e_lo = cs_bytes(&[0xa9]);
        pk!(c, r, 0, cs("s#+#"),
            [e_hi.as_ptr(), 1 as c_int, e_lo.as_ptr(), 1 as c_int], "utf8 split across parts");
        let emoji = "\u{1f600}".to_string().into_bytes();
        let p1 = cs_bytes(&emoji[..2]);
        let p2 = cs_bytes(&emoji[2..]);
        pk!(c, r, 0, cs("s#+#"),
            [p1.as_ptr(), 2 as c_int, p2.as_ptr(), 2 as c_int], "emoji split 2+2");
        let p1 = cs_bytes(&emoji[..1]);
        let p2 = cs_bytes(&emoji[1..]);
        pk!(c, r, 0, cs("s#+#"),
            [p1.as_ptr(), 1 as c_int, p2.as_ptr(), 3 as c_int], "emoji split 1+3");
        // A single part cut mid-sequence is invalid UTF-8 after concatenation.
        pk!(c, r, 0, cs("s#+#"),
            [p1.as_ptr(), 1 as c_int, p2.as_ptr(), 1 as c_int], "emoji truncated");
        pk!(c, r, 0, cs("s#+"), [e_hi.as_ptr(), 1 as c_int, a.as_ptr()], "lone lead byte + tail");
    }
}

#[test]
fn r237_pack_optional_strings() {
    let (c, r) = both();
    unsafe {
        let a = cs("val");
        let nul: *const c_char = std::ptr::null();
        // `s?` with NULL -> json_null(); `s*` with NULL -> NULL value (which the
        // enclosing container then skips, or which becomes the whole result).
        pk!(c, r, 0, cs("s?"), [a.as_ptr()], "s? non-NULL");
        pk!(c, r, 0, cs("s?"), [nul], "s? NULL");
        pk!(c, r, 0, cs("s*"), [a.as_ptr()], "s* non-NULL");
        pk!(c, r, 0, cs("s*"), [nul], "s* NULL at top level");
        pk!(c, r, 0, cs("[s?,s?]"), [nul, a.as_ptr()], "[s?,s?]");
        pk!(c, r, 0, cs("[s*,i]"), [nul, 5 as c_int], "[s*,i] skips the element");
        pk!(c, r, 0, cs("[s*,s*]"), [nul, nul], "[s*,s*] both skipped");
        pk!(c, r, 0, cs("{s:s?}"), [cs("k").as_ptr(), nul], "{{s:s?}} keeps key as null");
        pk!(c, r, 0, cs("{s:s*}"), [cs("k").as_ptr(), nul], "{{s:s*}} omits key");
        pk!(c, r, 0, cs("{s:s*,s:i}"), [cs("k").as_ptr(), nul, cs("j").as_ptr(), 2 as c_int],
            "{{s:s*,s:i}} mixed");
        // An invalid-UTF-8 optional string still fails (optional only guards NULL).
        let bad = cs_bytes(b"\xff");
        pk!(c, r, 0, cs("s?"), [bad.as_ptr()], "s? invalid utf8");
        pk!(c, r, 0, cs("s*"), [bad.as_ptr()], "s* invalid utf8");
    }
}

#[test]
fn r238_pack_length_modifier_on_optional_string_is_a_format_error() {
    let (c, r) = both();
    unsafe {
        let a = cs("val");
        // read_string's `else if (optional)` arm: "Cannot use '%c' on optional
        // strings", source <format>.
        for f in ["s?#", "s?%", "s?+", "s*#", "s*%", "s*+"] {
            pk!(c, r, 0, cs(f), [a.as_ptr(), 2 as c_int], "pack {f}");
            let fmt = cs(f);
            let mut cerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, fmt.as_ptr(), a.as_ptr(), 2 as c_int);
            assert!(cj.is_null(), "C: {f} must fail");
            assert_eq!(cerr.snapshot().3, "<format>", "C: {f} source");
            assert_eq!(cerr.code(), JSON_ERROR_INVALID_FORMAT, "C: {f} code");
            let want = format!("Cannot use '{}' on optional strings", &f[2..3]);
            assert_eq!(cerr.snapshot().4, want, "C: {f} text");
        }
        // Nested, so the container unwind path runs as well.
        pk!(c, r, 0, cs("{s:s?#}"), [cs("k").as_ptr(), a.as_ptr(), 1 as c_int], "{{s:s?#}}");
        pk!(c, r, 0, cs("[s*%]"), [a.as_ptr(), 1 as size_t], "[s*%]");
    }
}

// ===========================================================================
// Rows 239-240 — `o` / `O` and their `?` / `*` modifiers
// ===========================================================================

/// Build the same value in both libraries. Index selects one of the 8 types.
unsafe fn mk(api: &Api, which: usize) -> *mut json_t {
    match which {
        0 => (api.json_object)(),
        1 => (api.json_array)(),
        2 => (api.json_string)(cs("str").as_ptr()),
        3 => (api.json_integer)(42),
        4 => (api.json_real)(1.5),
        5 => (api.json_true)(),
        6 => (api.json_false)(),
        _ => (api.json_null)(),
    }
}

#[test]
fn r239_pack_o_and_O_refcounts() {
    let (c, r) = both();
    unsafe {
        for which in 0..8 {
            // ---- `o` steals the reference: refcount unchanged, same pointer.
            let cv = mk(c, which);
            let rv = mk(r, which);
            let cbefore = (*cv).refcount;
            let rbefore = (*rv).refcount;
            diff_eq!(cbefore, rbefore, "type {which}: refcount before o");
            let f = cs("o");
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), cv);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), rv);
            diff_eq!(cerr.raw(), rerr.raw(), "type {which}: o error image");
            diff_eq!(cj == cv, rj == rv, "type {which}: o returns the same pointer");
            diff_eq!((*cj).refcount, (*rj).refcount, "type {which}: refcount after o");
            diff_eq!((*cj).refcount == cbefore, (*rj).refcount == rbefore,
                     "type {which}: o did not incref");
            diff_eq!(canon(c, cj), canon(r, rj), "type {which}: o tree");
            decref(c, cj);
            decref(r, rj);

            // ---- `O` increfs: the caller keeps its own reference.
            let cv = mk(c, which);
            let rv = mk(r, which);
            let cbefore = (*cv).refcount;
            let f = cs("O");
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), cv);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), rv);
            diff_eq!((*cv).refcount, (*rv).refcount, "type {which}: refcount after O");
            if cbefore != usize::MAX {
                assert_eq!((*cv).refcount, cbefore + 1, "C: type {which}: O must incref");
            }
            diff_eq!(canon(c, cj), canon(r, rj), "type {which}: O tree");
            decref(c, cj);
            decref(r, rj);
            diff_eq!((*cv).refcount, (*rv).refcount, "type {which}: refcount after decref");
            decref(c, cv);
            decref(r, rv);

            // ---- inside containers, once with `o` (moved) and once with `O`.
            let cv = mk(c, which);
            let rv = mk(r, which);
            let f = cs("{s:o,s:i}");
            let k = cs("v");
            let k2 = cs("n");
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), k.as_ptr(), cv, k2.as_ptr(), 1 as c_int);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), k.as_ptr(), rv, k2.as_ptr(), 1 as c_int);
            diff_eq!(canon(c, cj), canon(r, rj), "type {which}: {{s:o,s:i}} tree");
            decref(c, cj);
            decref(r, rj);

            let cv = mk(c, which);
            let rv = mk(r, which);
            let f = cs("[O,O]");
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), cv, cv);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), rv, rv);
            diff_eq!((*cv).refcount, (*rv).refcount, "type {which}: refcount after [O,O]");
            diff_eq!(canon(c, cj), canon(r, rj), "type {which}: [O,O] tree");
            decref(c, cj);
            decref(r, rj);
            decref(c, cv);
            decref(r, rv);
        }
    }
}

#[test]
fn r240_pack_o_O_null_and_optional() {
    let (c, r) = both();
    unsafe {
        let nul: *mut json_t = std::ptr::null_mut();
        // NULL without a modifier -> json_error_null_value "NULL object".
        for f in ["o", "O"] {
            pk!(c, r, 0, cs(f), [nul], "pack {f} NULL");
            let fmt = cs(f);
            let mut cerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, fmt.as_ptr(), nul);
            assert!(cj.is_null(), "C: {f} NULL must fail");
            assert_eq!(cerr.snapshot().3, "<args>", "C: {f} source");
            assert_eq!(cerr.snapshot().4, "NULL object", "C: {f} text");
            assert_eq!(cerr.code(), JSON_ERROR_NULL_VALUE, "C: {f} code");
        }
        // `?` substitutes json_null(), `*` yields a NULL value (skipped).
        for f in ["o?", "O?", "o*", "O*"] {
            pk!(c, r, 0, cs(f), [nul], "pack {f} NULL at top level");
            pk!(c, r, 0, cs(&format!("[{f},i]")), [nul, 3 as c_int], "pack [{f},i]");
            pk!(c, r, 0, cs(&format!("{{s:{f}}}")), [cs("k").as_ptr(), nul], "pack {{s:{f}}}");
            pk!(c, r, 0, cs(&format!("{{s:{f},s:i}}")),
                [cs("k").as_ptr(), nul, cs("j").as_ptr(), 4 as c_int], "pack {{s:{f},s:i}}");
        }
        // The modifiers with a non-NULL value must behave like plain o/O.
        for f in ["o?", "O?", "o*", "O*"] {
            let cv = (c.json_integer)(9);
            let rv = (r.json_integer)(9);
            let fmt = cs(f);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, fmt.as_ptr(), cv);
            let rj = (r.json_pack_ex)(&mut rerr, 0, fmt.as_ptr(), rv);
            diff_eq!(cerr.raw(), rerr.raw(), "pack {f} non-NULL error");
            diff_eq!(canon(c, cj), canon(r, rj), "pack {f} non-NULL tree");
            diff_eq!((*cv).refcount, (*rv).refcount, "pack {f} non-NULL refcount");
            decref(c, cj);
            decref(r, rj);
            if f.starts_with('O') {
                decref(c, cv);
                decref(r, rv);
            }
        }
    }
}

// ===========================================================================
// Rows 241-249 — containers
// ===========================================================================

#[test]
fn r241_pack_arrays() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0241);
    unsafe {
        pk!(c, r, 0, cs("[]"), [], "pack []");
        pk!(c, r, 0, cs("[i]"), [1 as c_int], "pack [i]");
        pk!(c, r, 0, cs("[iii]"), [1 as c_int, 2 as c_int, 3 as c_int], "pack [iii]");
        pk!(c, r, 0, cs("[i,i,i]"), [1 as c_int, 2 as c_int, 3 as c_int], "pack [i,i,i]");
        // Every producible element type in one array.
        let cv = (c.json_integer)(7);
        let rv = (r.json_integer)(7);
        let cv2 = (c.json_string)(cs("shared").as_ptr());
        let rv2 = (r.json_string)(cs("shared").as_ptr());
        let f = cs("[s,i,I,f,b,n,o,O]");
        let sarg = cs("txt");
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), sarg.as_ptr(), 1 as c_int,
                                  2i64 as json_int_t, 3.5f64, 1 as c_int, cv, cv2);
        let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), sarg.as_ptr(), 1 as c_int,
                                  2i64 as json_int_t, 3.5f64, 1 as c_int, rv, rv2);
        diff_eq!(cerr.raw(), rerr.raw(), "pack all-types array error");
        diff_eq!(canon(c, cj), canon(r, rj), "pack all-types array");
        decref(c, cj);
        decref(r, rj);
        decref(c, cv2);
        decref(r, rv2);

        // 20 elements forces json_array_append_new growth twice inside pack_array.
        let f20: String = format!("[{}]", vec!["i"; 20].join(","));
        pk!(c, r, 0, cs(&f20),
            [0 as c_int, 1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int,
             6 as c_int, 7 as c_int, 8 as c_int, 9 as c_int, 10 as c_int, 11 as c_int,
             12 as c_int, 13 as c_int, 14 as c_int, 15 as c_int, 16 as c_int, 17 as c_int,
             18 as c_int, 19 as c_int],
            "pack 20-element array");
        for i in 0..1000 {
            let a = rng.json_int();
            let b = rng.real();
            pk!(c, r, 0, cs("[I,f,[I,f],[]]"), [a, b, a, b], "iter {i}: nested arrays");
        }
    }
}

#[test]
fn r242_pack_objects() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0242);
    unsafe {
        pk!(c, r, 0, cs("{}"), [], "pack {{}}");
        let k = cs("a");
        pk!(c, r, 0, cs("{si}"), [k.as_ptr(), 1 as c_int], "pack {{si}}");
        pk!(c, r, 0, cs("{s:i}"), [k.as_ptr(), 1 as c_int], "pack {{s:i}}");
        let k2 = cs("b");
        let k3 = cs("c");
        pk!(c, r, 0, cs("{sisisi}"),
            [k.as_ptr(), 1 as c_int, k2.as_ptr(), 2 as c_int, k3.as_ptr(), 3 as c_int],
            "pack {{sisisi}}");
        // One object per value format character.
        let sv = cs("v");
        pk!(c, r, 0, cs("{ss}"), [k.as_ptr(), sv.as_ptr()], "pack {{ss}}");
        pk!(c, r, 0, cs("{sI}"), [k.as_ptr(), 9i64 as json_int_t], "pack {{sI}}");
        pk!(c, r, 0, cs("{sf}"), [k.as_ptr(), 2.25f64], "pack {{sf}}");
        pk!(c, r, 0, cs("{sb}"), [k.as_ptr(), 1 as c_int], "pack {{sb}}");
        pk!(c, r, 0, cs("{sn}"), [k.as_ptr()], "pack {{sn}}");
        let cv = (c.json_integer)(5);
        let rv = (r.json_integer)(5);
        let f = cs("{so}");
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), k.as_ptr(), cv);
        let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), k.as_ptr(), rv);
        diff_eq!(canon(c, cj), canon(r, rj), "pack {{so}}");
        decref(c, cj);
        decref(r, rj);
        let cv = (c.json_integer)(5);
        let rv = (r.json_integer)(5);
        let f = cs("{sO}");
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), k.as_ptr(), cv);
        let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), k.as_ptr(), rv);
        diff_eq!(canon(c, cj), canon(r, rj), "pack {{sO}}");
        diff_eq!((*cv).refcount, (*rv).refcount, "pack {{sO}} refcount");
        decref(c, cj);
        decref(r, rj);
        decref(c, cv);
        decref(r, rv);

        // 12 keys forces a rehash of the object's hashtable during pack_object.
        let keys: Vec<CString> = (0..12).map(|i| cs(&format!("key{i:02}"))).collect();
        let f12: String = format!("{{{}}}", vec!["s:i"; 12].join(","));
        pk!(c, r, 0, cs(&f12),
            [keys[0].as_ptr(), 0 as c_int, keys[1].as_ptr(), 1 as c_int,
             keys[2].as_ptr(), 2 as c_int, keys[3].as_ptr(), 3 as c_int,
             keys[4].as_ptr(), 4 as c_int, keys[5].as_ptr(), 5 as c_int,
             keys[6].as_ptr(), 6 as c_int, keys[7].as_ptr(), 7 as c_int,
             keys[8].as_ptr(), 8 as c_int, keys[9].as_ptr(), 9 as c_int,
             keys[10].as_ptr(), 10 as c_int, keys[11].as_ptr(), 11 as c_int],
            "pack 12-key object");
        for i in 0..1000 {
            let ka = cs(&rng.ascii_string(8).replace('\0', "z"));
            let kb = cs(&rng.ascii_string(8).replace('\0', "z"));
            let v = rng.json_int();
            pk!(c, r, 0, cs("{s:I,s:n}"), [ka.as_ptr(), v, kb.as_ptr()],
                "iter {i}: random keys");
        }
    }
}

#[test]
fn r243_pack_object_keys_via_every_read_string_variant() {
    let (c, r) = both();
    unsafe {
        let a = cs("keyname");
        let b = cs("suffix");
        pk!(c, r, 0, cs("{s#i}"), [a.as_ptr(), 3 as c_int, 1 as c_int], "pack {{s#i}}");
        pk!(c, r, 0, cs("{s%i}"), [a.as_ptr(), 3 as size_t, 1 as c_int], "pack {{s%i}}");
        pk!(c, r, 0, cs("{s+i}"), [a.as_ptr(), b.as_ptr(), 1 as c_int], "pack {{s+i}}");
        pk!(c, r, 0, cs("{s+#i}"), [a.as_ptr(), b.as_ptr(), 2 as c_int, 1 as c_int],
            "pack {{s+#i}}");
        pk!(c, r, 0, cs("{s+%i}"), [a.as_ptr(), b.as_ptr(), 2 as size_t, 1 as c_int],
            "pack {{s+%i}}");
        pk!(c, r, 0, cs("{s#+#i}"),
            [a.as_ptr(), 3 as c_int, b.as_ptr(), 3 as c_int, 1 as c_int], "pack {{s#+#i}}");
        // Two such keys in one object (the jsonp_free(key) path runs twice).
        pk!(c, r, 0, cs("{s#:i,s+:i}"),
            [a.as_ptr(), 2 as c_int, 1 as c_int, a.as_ptr(), b.as_ptr(), 2 as c_int],
            "pack two strbuffer keys");
    }
}

#[test]
fn r244_pack_duplicate_object_keys() {
    let (c, r) = both();
    unsafe {
        let k = cs("dup");
        pk!(c, r, 0, cs("{sisi}"), [k.as_ptr(), 1 as c_int, k.as_ptr(), 2 as c_int],
            "pack duplicate keys, last wins");
        pk!(c, r, 0, cs("{sisisi}"),
            [k.as_ptr(), 1 as c_int, k.as_ptr(), 2 as c_int, k.as_ptr(), 3 as c_int],
            "pack triplicate keys");
        // Keys equal only up to the given length.
        let long = cs("dupXYZ");
        pk!(c, r, 0, cs("{s#:i,s#:i}"),
            [long.as_ptr(), 3 as c_int, 1 as c_int, k.as_ptr(), 3 as c_int, 2 as c_int],
            "pack s# keys equal up to length");
        // Different values types for the duplicate.
        pk!(c, r, 0, cs("{s:i,s:s}"), [k.as_ptr(), 1 as c_int, k.as_ptr(), long.as_ptr()],
            "pack duplicate key differing value types");
    }
}

#[test]
fn r245_pack_object_key_shapes() {
    let (c, r) = both();
    unsafe {
        let empty = cs("");
        pk!(c, r, 0, cs("{s:i}"), [empty.as_ptr(), 1 as c_int], "pack empty key");
        let big = cs(&"K".repeat(1500));
        pk!(c, r, 0, cs("{s:i}"), [big.as_ptr(), 1 as c_int], "pack >1KiB key");
        let utf = cs("k\u{e9}\u{20ac}\u{1f600}");
        pk!(c, r, 0, cs("{s:i}"), [utf.as_ptr(), 1 as c_int], "pack UTF-8 key");
        let bad = cs_bytes(b"k\xffz");
        pk!(c, r, 0, cs("{s:i}"), [bad.as_ptr(), 1 as c_int], "pack invalid-UTF-8 key");
        // Via s#, a key containing an embedded NUL: pack_object uses
        // json_object_setn_new_nocheck, so nothing rejects it.
        let nulkey = cs_bytes(b"a\0b");
        pk!(c, r, 0, cs("{s#:i}"), [nulkey.as_ptr(), 3 as c_int, 1 as c_int],
            "pack key with embedded NUL");
        pk!(c, r, 0, cs("{s#:i,s#:i}"),
            [nulkey.as_ptr(), 3 as c_int, 1 as c_int, nulkey.as_ptr(), 1 as c_int, 2 as c_int],
            "pack NUL key vs its prefix");
    }
}

#[test]
fn r246_pack_object_value_star_and_plain_null() {
    let (c, r) = both();
    unsafe {
        let nul: *mut json_t = std::ptr::null_mut();
        let k = cs("k");
        let j = cs("j");
        pk!(c, r, 0, cs("{s:o*}"), [k.as_ptr(), nul], "pack {{s:o*}} skips key");
        pk!(c, r, 0, cs("{s:o}"), [k.as_ptr(), nul], "pack {{s:o}} fails");
        let f = cs("{s:o}");
        let mut cerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), k.as_ptr(), nul);
        assert!(cj.is_null(), "C: {{s:o}} with NULL must fail");
        // pack_object_inter sets "NULL object" first; pack_object then overwrites
        // it with "NULL object value" only when has_error was NOT already set...
        // both messages are <args>/null_value, so just pin the observed one.
        assert_eq!(cerr.snapshot().3, "<args>", "C: source");
        assert_eq!(cerr.code(), JSON_ERROR_NULL_VALUE, "C: code");
        pk!(c, r, 0, cs("{s:o*,s:i}"), [k.as_ptr(), nul, j.as_ptr(), 1 as c_int],
            "pack {{s:o*,s:i}}");
        pk!(c, r, 0, cs("{s:i,s:o*}"), [j.as_ptr(), 1 as c_int, k.as_ptr(), nul],
            "pack {{s:i,s:o*}}");
        pk!(c, r, 0, cs("{s:o*,s:o*}"), [k.as_ptr(), nul, j.as_ptr(), nul],
            "pack all keys skipped");
        // A `*` key whose key itself came from the strbuffer: jsonp_free(key)
        // must still run on the skip path.
        pk!(c, r, 0, cs("{s#:o*}"), [k.as_ptr(), 1 as c_int, nul],
            "pack {{s#:o*}} frees the owned key");
        pk!(c, r, 0, cs("{s+:o*}"), [k.as_ptr(), j.as_ptr(), nul],
            "pack {{s+:o*}} frees the owned key");
    }
}

#[test]
fn r247_pack_object_value_optional_strings_and_O() {
    let (c, r) = both();
    unsafe {
        let nul: *mut json_t = std::ptr::null_mut();
        let nuls: *const c_char = std::ptr::null();
        let k = cs("k");
        pk!(c, r, 0, cs("{s:s?}"), [k.as_ptr(), nuls], "pack {{s:s?}} -> null value");
        pk!(c, r, 0, cs("{s:s*}"), [k.as_ptr(), nuls], "pack {{s:s*}} -> key omitted");
        pk!(c, r, 0, cs("{s:O*}"), [k.as_ptr(), nul], "pack {{s:O*}}");
        pk!(c, r, 0, cs("{s:O?}"), [k.as_ptr(), nul], "pack {{s:O?}}");
        pk!(c, r, 0, cs("{s:s?,s:s*,s:i}"),
            [k.as_ptr(), nuls, cs("m").as_ptr(), nuls, cs("n").as_ptr(), 3 as c_int],
            "pack mixed optional object values");
    }
}

#[test]
fn r248_pack_array_optional_elements() {
    let (c, r) = both();
    unsafe {
        let nul: *mut json_t = std::ptr::null_mut();
        let nuls: *const c_char = std::ptr::null();
        pk!(c, r, 0, cs("[o*]"), [nul], "pack [o*] -> empty array");
        pk!(c, r, 0, cs("[o]"), [nul], "pack [o] -> error");
        pk!(c, r, 0, cs("[o*,i]"), [nul, 5 as c_int], "pack [o*,i]");
        pk!(c, r, 0, cs("[i,o*]"), [5 as c_int, nul], "pack [i,o*]");
        pk!(c, r, 0, cs("[o*,o*]"), [nul, nul], "pack [o*,o*]");
        pk!(c, r, 0, cs("[s*]"), [nuls], "pack [s*]");
        pk!(c, r, 0, cs("[s?]"), [nuls], "pack [s?] -> [null]");
        pk!(c, r, 0, cs("[O*,i,s*]"), [nul, 1 as c_int, nuls], "pack [O*,i,s*]");
    }
}

#[test]
fn r249_pack_deeply_nested() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0249);
    unsafe {
        let a = cs("a");
        let b = cs("b");
        let cc = cs("c");
        let sv = cs("txt");
        pk!(c, r, 0, cs("{s:{s:[i,i]},s:[{s:s}]}"),
            [a.as_ptr(), b.as_ptr(), 1 as c_int, 2 as c_int, cc.as_ptr(), a.as_ptr(),
             sv.as_ptr()],
            "pack two-level nest");
        // A 5-level alternating object/array nest.
        pk!(c, r, 0, cs("{s:[{s:[{s:i}]}]}"),
            [a.as_ptr(), b.as_ptr(), cc.as_ptr(), a.as_ptr(), 7 as c_int],
            "pack 5-level nest");
        pk!(c, r, 0, cs("[[[[[i]]]]]"), [1 as c_int], "pack [[[[[i]]]]]");
        pk!(c, r, 0, cs("{s:{s:{s:{s:{s:i}}}}}"),
            [a.as_ptr(), a.as_ptr(), a.as_ptr(), a.as_ptr(), a.as_ptr(), 1 as c_int],
            "pack 5-level objects");
        // An error injected at the deepest level must unwind and free everything.
        pk!(c, r, 0, cs("{s:[{s:[{s:f}]}]}"),
            [a.as_ptr(), b.as_ptr(), cc.as_ptr(), a.as_ptr(), f64::NAN],
            "pack deep NAN error");
        pk!(c, r, 0, cs("{s:[{s:[{s:s}]}]}"),
            [a.as_ptr(), b.as_ptr(), cc.as_ptr(), a.as_ptr(), std::ptr::null::<c_char>()],
            "pack deep NULL-string error");
        pk!(c, r, 0, cs("[[[q]]]"), [], "pack deep bad format char");
        for i in 0..800 {
            let x = rng.json_int();
            let y = rng.real();
            pk!(c, r, 0, cs("{s:[i,{s:[I,f]}]}"),
                [a.as_ptr(), 1 as c_int, b.as_ptr(), x, y],
                "iter {i}: randomized nest");
        }
    }
}

// ===========================================================================
// Rows 250-251 — token stream and format errors
// ===========================================================================

#[test]
fn r250_pack_whitespace_and_separators() {
    let (c, r) = both();
    unsafe {
        let k = cs("a");
        let j = cs("b");
        // Every one of ' ', '\t', '\n', ',' and ':' is skipped by next_token, so
        // all of these must produce byte-identical trees.
        let equivalents = [
            "{s:i,s:i}",
            "{ s : i , s : i }",
            "{s i s i}",
            "{\ts\t:\ti\t,\ts\t:\ti\t}",
            "{,,s::i,,,s:i,}",
            "{\ns\n:\ni\n,\ns:i}",
            "  {s:i,s:i}  ",
            ":,{s:i s:i},:",
        ];
        let mut want: Option<Vec<u8>> = None;
        for f in equivalents {
            pk!(c, r, 0, cs(f), [k.as_ptr(), 1 as c_int, j.as_ptr(), 2 as c_int],
                "pack whitespace variant {f:?}");
            let fmt = cs(f);
            let mut e = json_error_t::new();
            let cj = (c.json_pack_ex)(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 1 as c_int,
                                      j.as_ptr(), 2 as c_int);
            let got = canon(c, cj);
            decref(c, cj);
            match &want {
                None => want = Some(got.unwrap()),
                Some(w) => assert_eq!(
                    Some(w.clone()),
                    got,
                    "C: whitespace variant {f:?} must match the compact form"
                ),
            }
        }
        for f in ["[i,i]", "[ i , i ]", "[i i]", "[\ti\t,\ti]", "[,i,,i,]"] {
            pk!(c, r, 0, cs(f), [1 as c_int, 2 as c_int], "pack array whitespace {f:?}");
        }
        // A failing variant containing newlines: the reported line/column/pos
        // come straight from the token bookkeeping.
        for f in ["{\nq}", "\n\nq", "[\n\nq]", "{s:i,\nq}", "  \n q", "\n\t\nx"] {
            pk!(c, r, 0, cs(f), [k.as_ptr(), 1 as c_int], "pack multiline error {f:?}");
            let fmt = cs(f);
            let mut cerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, fmt.as_ptr(), k.as_ptr(), 1 as c_int);
            assert!(cj.is_null(), "C: {f:?} must fail");
            decref(c, cj);
        }
    }
}

#[test]
fn r251_pack_format_errors() {
    let (c, r) = both();
    unsafe {
        let k = cs("a");
        let sv = cs("v");
        // Unknown format characters -> "Unexpected format character '%c'".
        for ch in [
            'q', 'x', 'z', 'S', 'B', 'N', 'd', 'l', 'u', '#', '%', '+', '?', '*', '!', ']',
            '}', '\'', '"', '\\', '(', ')', '<', '>', '=', '.', ';', '&', '|', '@', '~',
            '0', '9', 'A', 'Z',
        ] {
            let f = ch.to_string();
            pk!(c, r, 0, cs(&f), [], "pack unknown char {f:?}");
            pk!(c, r, 0, cs(&format!("[{f}]")), [], "pack [{f}]");
            // '#', '%' and '+' directly after the key's 's' are *modifiers* on
            // the key's read_string, so they consume a further vararg; they are
            // covered with matching arguments in rows 243/245 instead.
            if !matches!(ch, '#' | '%' | '+') {
                pk!(c, r, 0, cs(&format!("{{s:{f}}}")), [k.as_ptr()], "pack {{s:{f}}}");
            }
        }
        let mut cerr = json_error_t::poisoned();
        let f = cs("q");
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr());
        assert!(cj.is_null(), "C: 'q' must fail");
        assert_eq!(cerr.snapshot().3, "<format>", "C: source");
        assert_eq!(cerr.snapshot().4, "Unexpected format character 'q'", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_INVALID_FORMAT, "C: code");

        // Unterminated containers -> "Unexpected end of format string".
        for f in ["{", "[", "{s:i", "[i", "[[", "{{", "{s", "[i,", "{s:"] {
            pk!(c, r, 0, cs(f), [k.as_ptr(), 1 as c_int], "pack unterminated {f:?}");
        }
        let f = cs("{");
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr());
        assert!(cj.is_null(), "C: '{{' must fail");
        assert_eq!(cerr.snapshot().4, "Unexpected end of format string", "C: text");

        // A non-'s' object key -> "Expected format 's', got '%c'".
        for f in ["{i:i}", "{n}", "{[i]:i}", "{b:i}", "{f:i}", "{o:i}", "{q:i}"] {
            pk!(c, r, 0, cs(f), [1 as c_int, 2 as c_int], "pack bad key format {f:?}");
        }
        let f = cs("{i:i}");
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), 1 as c_int, 2 as c_int);
        assert!(cj.is_null(), "C: {{i:i}} must fail");
        assert_eq!(cerr.snapshot().4, "Expected format 's', got 'i'", "C: text");

        // Garbage after a complete format -> the built value is decref'd first.
        // NOTE: the varargs must still MATCH the format, because the scanner
        // consumes the whole first value before it notices the trailing token.
        for f in ["ii", "i x", "[i]i", "n n", "[i][i]", "{}{}", "i]", "i}", "i,i", "i:i"] {
            pk!(c, r, 0, cs(f), [1 as c_int, 2 as c_int], "pack garbage after {f:?}");
        }
        for f in ["ss", "s s", "s+s"] {
            pk!(c, r, 0, cs(f), [sv.as_ptr(), sv.as_ptr(), sv.as_ptr()],
                "pack garbage after {f:?}");
        }
        for f in ["{s:i}i", "{s:i}{", "{s:i}]"] {
            pk!(c, r, 0, cs(f), [k.as_ptr(), 1 as c_int, 2 as c_int],
                "pack garbage after {f:?}");
        }
        let f = cs("ii");
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), 1 as c_int, 2 as c_int);
        assert!(cj.is_null(), "C: 'ii' must fail");
        assert_eq!(cerr.snapshot().3, "<format>", "C: source");
        assert_eq!(cerr.snapshot().4, "Garbage after format string", "C: text");
        // Garbage after a format whose string args came from the strbuffer.
        pk!(c, r, 0, cs("s+ i"), [sv.as_ptr(), sv.as_ptr(), 1 as c_int], "pack 's+ i'");
    }
}

// ===========================================================================
// Row 252 — json_pack_ex error struct fields, including the <internal> source
// ===========================================================================

// The allocator hooks below are *interchangeable with the defaults* (they just
// forward to libc), so installing them once and leaving them installed cannot
// disturb any other test in this binary. The only behavioural change is driven
// by a THREAD-LOCAL "fail this exact size" switch, so a concurrently running
// test on another thread is unaffected.
extern "C" {
    fn malloc(n: size_t) -> *mut c_void;
    fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
}

thread_local! {
    static FAIL_SIZE: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

unsafe extern "C" fn hook_malloc(n: size_t) -> *mut c_void {
    if n == FAIL_SIZE.with(|f| f.get()) {
        return std::ptr::null_mut();
    }
    malloc(n)
}
unsafe extern "C" fn hook_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    if n == FAIL_SIZE.with(|f| f.get()) {
        return std::ptr::null_mut();
    }
    realloc(p, n)
}
unsafe extern "C" fn hook_free(p: *mut c_void) {
    free(p)
}

fn install_hooks(c: &Api, r: &Api) {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        (c.json_set_alloc_funcs2)(Some(hook_malloc), Some(hook_realloc), Some(hook_free));
        (r.json_set_alloc_funcs2)(Some(hook_malloc), Some(hook_realloc), Some(hook_free));
    });
}

#[test]
fn r252_pack_ex_error_fields_all_three_sources() {
    let (c, r) = both();
    unsafe {
        let k = cs("key");
        // ---- source "<args>"
        for (fmt, args_null_string) in [("s", true), ("{s:i}", true)] {
            let f = cs(fmt);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), std::ptr::null::<c_char>(),
                                      1 as c_int);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), std::ptr::null::<c_char>(),
                                      1 as c_int);
            assert!(args_null_string && cj.is_null(), "C: {fmt} must fail");
            diff_eq!(cerr.raw(), rerr.raw(), "<args> error image for {fmt}");
            diff_eq!(cerr.snapshot(), rerr.snapshot(), "<args> error snapshot for {fmt}");
            assert_eq!(cerr.snapshot().3, "<args>", "C: source for {fmt}");
            decref(c, cj);
            decref(r, rj);
        }
        // ---- source "<format>", with line/column/position from the token
        for fmt in ["q", "{q}", "\nq", "{s:i}x", "{i:i}", "s?#"] {
            let f = cs(fmt);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), k.as_ptr(), 1 as c_int);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), k.as_ptr(), 1 as c_int);
            diff_eq!(cj.is_null(), rj.is_null(), "<format> NULL-ness for {fmt:?}");
            diff_eq!(cerr.raw(), rerr.raw(), "<format> error image for {fmt:?}");
            diff_eq!(
                (cerr.line, cerr.column, cerr.position),
                (rerr.line, rerr.column, rerr.position),
                "<format> line/column/position for {fmt:?}"
            );
            decref(c, cj);
            decref(r, rj);
        }
        // ---- source "<internal>": the only way in is an allocation failure.
        // strbuffer_init is the sole 16-byte (STRBUFFER_MIN_SIZE) request on the
        // "{s+i}" path, so failing exactly that size reaches read_string's
        // "Out of memory" without perturbing anything else.
        install_hooks(c, r);
        let a = cs("aa");
        let b = cs("bb");
        for fmt in ["{s+i}", "s+", "s#", "s%", "{s#:i}"] {
            let f = cs(fmt);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            FAIL_SIZE.with(|s| s.set(16));
            let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), a.as_ptr(), b.as_ptr(),
                                      1 as c_int);
            let rj = (r.json_pack_ex)(&mut rerr, 0, f.as_ptr(), a.as_ptr(), b.as_ptr(),
                                      1 as c_int);
            FAIL_SIZE.with(|s| s.set(usize::MAX));
            diff_eq!(cj.is_null(), rj.is_null(), "<internal> NULL-ness for {fmt:?}");
            diff_eq!(cerr.raw(), rerr.raw(), "<internal> error image for {fmt:?}");
            assert!(cj.is_null(), "C: {fmt:?} must fail when strbuffer_init fails");
            assert_eq!(cerr.snapshot().3, "<internal>", "C: source for {fmt:?}");
            assert_eq!(cerr.snapshot().4, "Out of memory", "C: text for {fmt:?}");
            assert_eq!(cerr.code(), JSON_ERROR_OUT_OF_MEMORY, "C: code for {fmt:?}");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 253 — pack ignores JSON_VALIDATE_ONLY and JSON_STRICT
// ===========================================================================

#[test]
fn r253_pack_flags_are_ignored() {
    let (c, r) = both();
    unsafe {
        let k = cs("a");
        let j = cs("b");
        let sv = cs("text");
        let fmt = cs("{s:[i,I,f,b,n,s],s:{s:s#}}");
        let mut reference: Option<Vec<u8>> = None;
        for flags in [
            0,
            JSON_VALIDATE_ONLY,
            JSON_STRICT,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            // Unrelated bits must be ignored too.
            JSON_ENCODE_ANY,
            usize::MAX,
        ] {
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_pack_ex)(&mut cerr, flags, fmt.as_ptr(),
                k.as_ptr(), 1 as c_int, 2i64 as json_int_t, 3.5f64, 1 as c_int, sv.as_ptr(),
                j.as_ptr(), k.as_ptr(), sv.as_ptr(), 2 as c_int);
            let rj = (r.json_pack_ex)(&mut rerr, flags, fmt.as_ptr(),
                k.as_ptr(), 1 as c_int, 2i64 as json_int_t, 3.5f64, 1 as c_int, sv.as_ptr(),
                j.as_ptr(), k.as_ptr(), sv.as_ptr(), 2 as c_int);
            diff_eq!(cerr.raw(), rerr.raw(), "pack flags={flags:#x} error image");
            let cd = canon(c, cj);
            diff_eq!(cd.clone(), canon(r, rj), "pack flags={flags:#x} tree");
            match &reference {
                None => reference = Some(cd.unwrap()),
                Some(w) => assert_eq!(Some(w.clone()), cd, "C: flags={flags:#x} must not matter"),
            }
            decref(c, cj);
            decref(r, rj);
        }
        // Same for an error case: the flags must not change the error either.
        for flags in [0, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
            pk!(c, r, flags, cs("{s:q}"), [k.as_ptr()], "pack error flags={flags:#x}");
        }
    }
}

// ===========================================================================
// Row 254 — json_vpack_ex through a real va_list
// ===========================================================================

#[test]
fn r254_vpack_ex_through_a_real_va_list() {
    let (c, r) = both();
    unsafe {
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vpack_ex");
        let rfn = sym_addr("Rust", b"json_vpack_ex");
        let k1 = cs("str");
        let k2 = cs("arr");
        let k3 = cs("obj");
        let sv = cs("abcdef");

        for flags in [0, JSON_VALIDATE_ONLY, JSON_STRICT] {
            // ---- success path: 12 varargs of mixed integer/double/pointer class
            let fmt = cs("{s:s#,s:[i,I,f,b,n],s:o}");
            let cv = (c.json_integer)(77);
            let rv = (r.json_integer)(77);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cfn, &mut cerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sv.as_ptr(), 4 as c_int,
                k2.as_ptr(), 1 as c_int, 2i64 as json_int_t, 3.25f64, 1 as c_int,
                k3.as_ptr(), cv);
            let rj = (sh.vpack_ex)(rfn, &mut rerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sv.as_ptr(), 4 as c_int,
                k2.as_ptr(), 1 as c_int, 2i64 as json_int_t, 3.25f64, 1 as c_int,
                k3.as_ptr(), rv);
            diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex flags={flags} NULL-ness");
            diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex flags={flags} error image");
            diff_eq!(canon(c, cj), canon(r, rj), "vpack_ex flags={flags} tree");
            assert!(!cj.is_null(), "C: vpack_ex success path");
            decref(c, cj);
            decref(r, rj);

            // ---- the same format with a `%` length instead of `#`
            let fmt = cs("{s:s%,s:[i,I,f,b,n],s:O}");
            let cv = (c.json_string)(sv.as_ptr());
            let rv = (r.json_string)(sv.as_ptr());
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cfn, &mut cerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sv.as_ptr(), 3 as size_t,
                k2.as_ptr(), -5 as c_int, i64::MIN as json_int_t, -0.0f64, 0 as c_int,
                k3.as_ptr(), cv);
            let rj = (sh.vpack_ex)(rfn, &mut rerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sv.as_ptr(), 3 as size_t,
                k2.as_ptr(), -5 as c_int, i64::MIN as json_int_t, -0.0f64, 0 as c_int,
                k3.as_ptr(), rv);
            diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex s% flags={flags} error image");
            diff_eq!(canon(c, cj), canon(r, rj), "vpack_ex s% flags={flags} tree");
            diff_eq!((*cv).refcount, (*rv).refcount, "vpack_ex O refcount flags={flags}");
            decref(c, cj);
            decref(r, rj);
            decref(c, cv);
            decref(r, rv);

            // ---- EARLY error path: the failure happens after only some of the
            // varargs have been consumed, which is where a va_copy bug shows up.
            let fmt = cs("{s:s#,s:[i,I,f,b,n],s:o}");
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cfn, &mut cerr, flags, fmt.as_ptr(),
                std::ptr::null::<c_char>(), sv.as_ptr(), 4 as c_int,
                k2.as_ptr(), 1 as c_int, 2i64 as json_int_t, f64::NAN, 1 as c_int,
                k3.as_ptr(), std::ptr::null_mut::<json_t>());
            let rj = (sh.vpack_ex)(rfn, &mut rerr, flags, fmt.as_ptr(),
                std::ptr::null::<c_char>(), sv.as_ptr(), 4 as c_int,
                k2.as_ptr(), 1 as c_int, 2i64 as json_int_t, f64::NAN, 1 as c_int,
                k3.as_ptr(), std::ptr::null_mut::<json_t>());
            diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex error path NULL-ness flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex error path image flags={flags}");
            assert!(cj.is_null(), "C: vpack_ex error path must return NULL");
            decref(c, cj);
            decref(r, rj);

            // ---- an unrecoverable FORMAT error, which returns before most
            // varargs are read at all.
            let fmt = cs("{s:s#,q}");
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cfn, &mut cerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sv.as_ptr(), 4 as c_int, k2.as_ptr(), 1 as c_int);
            let rj = (sh.vpack_ex)(rfn, &mut rerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sv.as_ptr(), 4 as c_int, k2.as_ptr(), 1 as c_int);
            diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex fmt error NULL-ness flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex fmt error image flags={flags}");
            decref(c, cj);
            decref(r, rj);
        }

        // Many varargs (more than the 6 GP / 8 SSE registers), so the va_list has
        // to walk into the overflow area.
        let fmt = cs("[i,i,i,i,i,i,i,i,f,f,f,f,f,f,f,f,f,f,s,s]");
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (sh.vpack_ex)(cfn, &mut cerr, 0, fmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64, 6.0f64, 7.0f64, 8.0f64, 9.0f64, 10.0f64,
            sv.as_ptr(), k1.as_ptr());
        let rj = (sh.vpack_ex)(rfn, &mut rerr, 0, fmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64, 6.0f64, 7.0f64, 8.0f64, 9.0f64, 10.0f64,
            sv.as_ptr(), k1.as_ptr());
        diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex overflow-area error image");
        diff_eq!(canon(c, cj), canon(r, rj), "vpack_ex overflow-area tree");
        assert!(!cj.is_null(), "C: overflow-area vpack must succeed");
        decref(c, cj);
        decref(r, rj);
    }
}

// ===========================================================================
// Row 255 — unpack argument guards
// ===========================================================================

/// One root text per json type, in `json_typeof` order, so error messages that
/// embed `type_name(root)` are all reachable.
const ROOTS: [(&str, &str); 8] = [
    ("{}", "object"),
    ("[]", "array"),
    ("\"txt\"", "string"),
    ("7", "integer"),
    ("1.5", "real"),
    ("true", "true"),
    ("false", "false"),
    ("null", "null"),
];

#[test]
fn r255_unpack_null_root_and_bad_format() {
    let (c, r) = both();
    unsafe {
        let f = cs("i");
        for flags in [0, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
            // root == NULL is checked before the format, and installs "<root>".
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cret = (c.json_unpack_ex)(std::ptr::null_mut(), &mut cerr, flags, f.as_ptr());
            let rret = (r.json_unpack_ex)(std::ptr::null_mut(), &mut rerr, flags, f.as_ptr());
            diff_eq!(cret, rret, "unpack NULL root return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "unpack NULL root error flags={flags}");
            assert_eq!(cret, -1, "C: NULL root must fail");
            assert_eq!(cerr.snapshot().3, "<root>", "C: source");
            assert_eq!(cerr.snapshot().4, "NULL root value", "C: text");
            assert_eq!((cerr.line, cerr.column, cerr.position), (-1, -1, 0), "C: position");
            assert_eq!(cerr.code(), JSON_ERROR_NULL_VALUE, "C: code");

            // A NULL root wins over a NULL format.
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cret = (c.json_unpack_ex)(std::ptr::null_mut(), &mut cerr, flags,
                                          std::ptr::null());
            let rret = (r.json_unpack_ex)(std::ptr::null_mut(), &mut rerr, flags,
                                          std::ptr::null());
            diff_eq!(cret, rret, "unpack NULL root+fmt return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "unpack NULL root+fmt error flags={flags}");

            // NULL / empty format with a valid root.
            let empty = cs("");
            for (i, fmt) in [std::ptr::null::<c_char>(), empty.as_ptr()].iter().enumerate() {
                let croot = load(c, "{\"a\":1}");
                let rroot = load(r, "{\"a\":1}");
                let mut cerr = json_error_t::poisoned();
                let mut rerr = json_error_t::poisoned();
                let cret = (c.json_unpack_ex)(croot, &mut cerr, flags, *fmt);
                let rret = (r.json_unpack_ex)(rroot, &mut rerr, flags, *fmt);
                diff_eq!(cret, rret, "unpack bad fmt #{i} return flags={flags}");
                diff_eq!(cerr.raw(), rerr.raw(), "unpack bad fmt #{i} error flags={flags}");
                assert_eq!(cerr.snapshot().3, "<format>", "C: source");
                assert_eq!(cerr.code(), JSON_ERROR_INVALID_ARGUMENT, "C: code");
                decref(c, croot);
                decref(r, rroot);
            }
        }
        // json_unpack (error == NULL) must survive the same paths.
        let croot = load(c, "1");
        let rroot = load(r, "1");
        diff_eq!(
            (c.json_unpack)(std::ptr::null_mut(), f.as_ptr()),
            (r.json_unpack)(std::ptr::null_mut(), f.as_ptr()),
            "json_unpack NULL root"
        );
        diff_eq!(
            (c.json_unpack)(croot, std::ptr::null::<c_char>()),
            (r.json_unpack)(rroot, std::ptr::null::<c_char>()),
            "json_unpack NULL fmt"
        );
        decref(c, croot);
        decref(r, rroot);
    }
}

// ===========================================================================
// Rows 256-257 — unpack `s` and `s%`
// ===========================================================================

#[test]
fn r256_unpack_string() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0256);
    unsafe {
        for text in ["\"\"", "\"a\"", "\"hello\"", "\"\\u00e9\\u20ac\"", "\"\\u0000\""] {
            upk!(c, r, text, 0, cs("s"), sl, [sp(sl, 0)], &[], "unpack s from {text}");
            upkn!(c, r, text, cs("s"), sl, [sp(sl, 3)], &[], "json_unpack s from {text}");
        }
        // Every non-string root -> "Expected string, got <type>", <validation>.
        for (text, name) in ROOTS {
            upk!(c, r, text, 0, cs("s"), sl, [sp(sl, 1)], &[],
                 "unpack s from {text} ({name})");
            if name != "string" {
                let croot = load(c, text);
                let mut cerr = json_error_t::poisoned();
                let mut tgt: *const c_char = poison_str_ptr();
                let f = cs("s");
                let cret = (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut tgt);
                assert_eq!(cret, -1, "C: s on {text} must fail");
                assert_eq!(cerr.snapshot().3, "<validation>", "C: source for {text}");
                assert_eq!(cerr.snapshot().4, format!("Expected string, got {name}"),
                           "C: text for {text}");
                assert_eq!(cerr.code(), JSON_ERROR_WRONG_TYPE, "C: code for {text}");
                decref(c, croot);
            }
        }
        // A NULL `const char **` -> "NULL string argument", <args>.
        upk!(c, r, "\"x\"", 0, cs("s"), sl, [std::ptr::null_mut::<*const c_char>()], &[],
             "unpack s with NULL target");
        upk!(c, r, "[\"x\",\"y\"]", 0, cs("[s,s]"), sl,
             [sp(sl, 0), std::ptr::null_mut::<*const c_char>()], &[],
             "unpack [s,s] with second target NULL");
        for i in 0..2000 {
            let s = rng.utf8_string(20);
            if s.as_bytes().contains(&0) {
                continue;
            }
            let cj = (c.json_string)(cs(&s).as_ptr());
            let rj = (r.json_string)(cs(&s).as_ptr());
            if cj.is_null() {
                decref(c, cj);
                decref(r, rj);
                continue;
            }
            let mut ctgt: *const c_char = poison_str_ptr();
            let mut rtgt: *const c_char = poison_str_ptr();
            let f = cs("s");
            let cret = (c.json_unpack)(cj, f.as_ptr(), &mut ctgt);
            let rret = (r.json_unpack)(rj, f.as_ptr(), &mut rtgt);
            diff_eq!(cret, rret, "iter {i}: unpack s return");
            diff_eq!(cbytes(ctgt), cbytes(rtgt), "iter {i}: unpack s bytes");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn r257_unpack_string_with_length() {
    let (c, r) = both();
    unsafe {
        // `s%` fills both the char** and the size_t*.
        for text in ["\"\"", "\"abc\"", "\"\\u00e9\"", "\"a\\u0000b\"", "\"\\u0000\""] {
            upk!(c, r, text, 0, cs("s%"), sl, [sp(sl, 0), lp(sl, 0)], &[],
                 "unpack s% from {text}");
            // The length target must also be filled inside containers.
            let arr = format!("[{text}]");
            upk!(c, r, &arr, 0, cs("[s%]"), sl, [sp(sl, 1), lp(sl, 1)], &[],
                 "unpack [s%] from {arr}");
            let obj = format!("{{\"k\":{text}}}");
            upk!(c, r, &obj, 0, cs("{s:s%}"), sl, [cs("k").as_ptr(), sp(sl, 2), lp(sl, 2)],
                 &[], "unpack {{s:s%}} from {obj}");
        }
        // A NULL size_t* -> "NULL string length argument", <args>.
        upk!(c, r, "\"abc\"", 0, cs("s%"), sl, [sp(sl, 0), std::ptr::null_mut::<size_t>()],
             &[], "unpack s% with NULL length target");
        let croot = load(c, "\"abc\"");
        let mut cerr = json_error_t::poisoned();
        let mut tgt: *const c_char = poison_str_ptr();
        let f = cs("s%");
        let cret = (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut tgt,
                                      std::ptr::null_mut::<size_t>());
        assert_eq!(cret, -1, "C: NULL length target must fail");
        assert_eq!(cerr.snapshot().3, "<args>", "C: source");
        assert_eq!(cerr.snapshot().4, "NULL string length argument", "C: text");
        decref(c, croot);

        // Under JSON_VALIDATE_ONLY neither vararg is consumed and neither target
        // is written. (Passing them anyway is safe: nothing follows.)
        for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
            upk!(c, r, "\"abc\"", flags, cs("s%"), sl, [sp(sl, 4), lp(sl, 4)], &[],
                 "unpack s% VALIDATE_ONLY flags={flags}");
            // ... and the type check still runs.
            upk!(c, r, "7", flags, cs("s%"), sl, [sp(sl, 5), lp(sl, 5)], &[],
                 "unpack s% VALIDATE_ONLY on integer flags={flags}");
        }
    }
}

// ===========================================================================
// Rows 258-262 — unpack i I b f F n o O
// ===========================================================================

#[test]
fn r258_unpack_integers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0258);
    unsafe {
        for text in ["0", "1", "-1", "2147483647", "-2147483648", "2147483648",
                     "9223372036854775807", "-9223372036854775808", "4294967296"] {
            // `i` truncates to int; `I` keeps the full json_int_t.
            upk!(c, r, text, 0, cs("i"), sl, [ip(sl, 0)], &[], "unpack i from {text}");
            upk!(c, r, text, 0, cs("I"), sl, [i64p(sl, 0)], &[], "unpack I from {text}");
            upk!(c, r, text, 0, cs("[i,I]"), sl, [ip(sl, 1), i64p(sl, 1)], &[],
                 "unpack [i,I] is not applicable"); // wrong type on purpose: root is scalar
        }
        for (text, name) in ROOTS {
            for f in ["i", "I"] {
                upk!(c, r, text, 0, cs(f), sl, [ip(sl, 2)], &[],
                     "unpack {f} from {text} ({name})");
            }
            if name != "integer" {
                let croot = load(c, text);
                let mut cerr = json_error_t::poisoned();
                let mut t: c_int = POISON_I32;
                let f = cs("i");
                assert_eq!(
                    (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut t),
                    -1,
                    "C: i on {text} must fail"
                );
                assert_eq!(cerr.snapshot().4, format!("Expected integer, got {name}"),
                           "C: text for {text}");
                decref(c, croot);
            }
        }
        for i in 0..2000 {
            let v = rng.json_int();
            let cj = (c.json_integer)(v);
            let rj = (r.json_integer)(v);
            let mut ci: c_int = POISON_I32;
            let mut ri: c_int = POISON_I32;
            let mut cI: json_int_t = POISON_I64;
            let mut rI: json_int_t = POISON_I64;
            let f = cs("i");
            diff_eq!(
                (c.json_unpack)(cj, f.as_ptr(), &mut ci),
                (r.json_unpack)(rj, f.as_ptr(), &mut ri),
                "iter {i}: unpack i return for {v}"
            );
            diff_eq!(ci, ri, "iter {i}: unpack i value for {v}");
            let f = cs("I");
            diff_eq!(
                (c.json_unpack)(cj, f.as_ptr(), &mut cI),
                (r.json_unpack)(rj, f.as_ptr(), &mut rI),
                "iter {i}: unpack I return for {v}"
            );
            diff_eq!(cI, rI, "iter {i}: unpack I value for {v}");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn r259_unpack_boolean() {
    let (c, r) = both();
    unsafe {
        upk!(c, r, "true", 0, cs("b"), sl, [ip(sl, 0)], &[], "unpack b from true");
        upk!(c, r, "false", 0, cs("b"), sl, [ip(sl, 1)], &[], "unpack b from false");
        for (text, name) in ROOTS {
            upk!(c, r, text, 0, cs("b"), sl, [ip(sl, 2)], &[],
                 "unpack b from {text} ({name})");
            if name != "true" && name != "false" {
                let croot = load(c, text);
                let mut cerr = json_error_t::poisoned();
                let mut t: c_int = POISON_I32;
                let f = cs("b");
                assert_eq!(
                    (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut t),
                    -1,
                    "C: b on {text} must fail"
                );
                assert_eq!(cerr.snapshot().4, format!("Expected true or false, got {name}"),
                           "C: text for {text}");
                decref(c, croot);
            }
        }
        upk!(c, r, "[true,false,true]", 0, cs("[b,b,b]"), sl,
             [ip(sl, 3), ip(sl, 4), ip(sl, 5)], &[], "unpack [b,b,b]");
    }
}

#[test]
fn r260_unpack_reals_and_numbers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0260);
    unsafe {
        for text in ["1.5", "-0.0", "0.0", "1e308", "5e-324", "3.141592653589793"] {
            upk!(c, r, text, 0, cs("f"), sl, [dp(sl, 0)], &[], "unpack f from {text}");
            upk!(c, r, text, 0, cs("F"), sl, [dp(sl, 1)], &[], "unpack F from {text}");
        }
        // `f` rejects an integer root; `F` accepts it via json_number_value.
        for text in ["7", "0", "9223372036854775807", "-9223372036854775808"] {
            upk!(c, r, text, 0, cs("f"), sl, [dp(sl, 2)], &[], "unpack f from integer {text}");
            upk!(c, r, text, 0, cs("F"), sl, [dp(sl, 3)], &[], "unpack F from integer {text}");
        }
        for (text, name) in ROOTS {
            for f in ["f", "F"] {
                upk!(c, r, text, 0, cs(f), sl, [dp(sl, 4)], &[],
                     "unpack {f} from {text} ({name})");
            }
            let croot = load(c, text);
            let mut cerr = json_error_t::poisoned();
            let mut t: f64 = f64::from_bits(POISON_DBITS);
            let f = cs("f");
            let ret = (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut t);
            if name != "real" {
                assert_eq!(ret, -1, "C: f on {text} must fail");
                assert_eq!(cerr.snapshot().4, format!("Expected real, got {name}"),
                           "C: f text for {text}");
            }
            let mut cerr = json_error_t::poisoned();
            let f = cs("F");
            let ret = (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut t);
            if name != "real" && name != "integer" {
                assert_eq!(ret, -1, "C: F on {text} must fail");
                assert_eq!(cerr.snapshot().4,
                           format!("Expected real or integer, got {name}"),
                           "C: F text for {text}");
            } else {
                assert_eq!(ret, 0, "C: F on {text} must succeed");
            }
            decref(c, croot);
        }
        for i in 0..2000 {
            let v = rng.real();
            let cj = (c.json_real)(v);
            let rj = (r.json_real)(v);
            if cj.is_null() {
                decref(c, cj);
                decref(r, rj);
                continue;
            }
            let mut cd = f64::from_bits(POISON_DBITS);
            let mut rd = f64::from_bits(POISON_DBITS);
            let f = cs("[f]");
            let carr = (c.json_array)();
            let rarr = (r.json_array)();
            (c.json_array_append_new)(carr, cj);
            (r.json_array_append_new)(rarr, rj);
            diff_eq!(
                (c.json_unpack)(carr, f.as_ptr(), &mut cd),
                (r.json_unpack)(rarr, f.as_ptr(), &mut rd),
                "iter {i}: unpack [f] return for {v:e}"
            );
            // Compare BITS so -0.0 stays distinct from 0.0.
            diff_eq!(cd.to_bits(), rd.to_bits(), "iter {i}: unpack [f] bits for {v:e}");
            decref(c, carr);
            decref(r, rarr);
        }
    }
}

#[test]
fn r261_unpack_null_consumes_no_vararg() {
    let (c, r) = both();
    unsafe {
        for flags in [0, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
            upk!(c, r, "null", flags, cs("n"), sl, [], &[], "unpack n flags={flags}");
            // `n` never reads a vararg, so a following format character still
            // sees the FIRST vararg — that is what this pairing proves.
            upk!(c, r, "[null,7]", flags, cs("[n,i]"), sl, [ip(sl, 0)], &[],
                 "unpack [n,i] flags={flags}");
            upk!(c, r, "[7,null]", flags, cs("[i,n]"), sl, [ip(sl, 1)], &[],
                 "unpack [i,n] flags={flags}");
            for (text, name) in ROOTS {
                upk!(c, r, text, flags, cs("n"), sl, [], &[],
                     "unpack n from {text} ({name}) flags={flags}");
            }
        }
        for (text, name) in ROOTS {
            if name == "null" {
                continue;
            }
            let croot = load(c, text);
            let mut cerr = json_error_t::poisoned();
            let f = cs("n");
            assert_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr()),
                -1,
                "C: n on {text} must fail"
            );
            assert_eq!(cerr.snapshot().4, format!("Expected null, got {name}"),
                       "C: text for {text}");
            decref(c, croot);
        }
    }
}

#[test]
fn r262_unpack_o_and_O() {
    let (c, r) = both();
    unsafe {
        for (text, name) in ROOTS {
            // `o` borrows: refcount unchanged.
            let croot = load(c, text);
            let rroot = load(r, text);
            let cbefore = (*croot).refcount;
            let mut ctgt: *mut json_t = sentinel_json();
            let mut rtgt: *mut json_t = sentinel_json();
            let f = cs("o");
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            diff_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut ctgt),
                (r.json_unpack_ex)(rroot, &mut rerr, 0, f.as_ptr(), &mut rtgt),
                "unpack o from {text} ({name}) return"
            );
            diff_eq!(cerr.raw(), rerr.raw(), "unpack o from {text} error");
            diff_eq!(ctgt == croot, rtgt == rroot, "unpack o target is the root ({name})");
            diff_eq!((*croot).refcount, (*rroot).refcount, "unpack o refcount ({name})");
            assert_eq!((*croot).refcount, cbefore, "C: o must not incref ({name})");

            // `O` increfs.
            let mut ctgt: *mut json_t = sentinel_json();
            let mut rtgt: *mut json_t = sentinel_json();
            let f = cs("O");
            diff_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut ctgt),
                (r.json_unpack_ex)(rroot, &mut rerr, 0, f.as_ptr(), &mut rtgt),
                "unpack O from {text} return"
            );
            diff_eq!((*croot).refcount, (*rroot).refcount, "unpack O refcount ({name})");
            if cbefore != usize::MAX {
                assert_eq!((*croot).refcount, cbefore + 1, "C: O must incref ({name})");
            }
            decref(c, ctgt);
            decref(r, rtgt);

            // Under JSON_VALIDATE_ONLY neither the vararg nor the incref happens.
            for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
                for fs in ["o", "O"] {
                    let f = cs(fs);
                    let mut ctgt: *mut json_t = sentinel_json();
                    let mut rtgt: *mut json_t = sentinel_json();
                    let cr_ = (c.json_unpack_ex)(croot, &mut cerr, flags, f.as_ptr(),
                                                 &mut ctgt);
                    let rr_ = (r.json_unpack_ex)(rroot, &mut rerr, flags, f.as_ptr(),
                                                 &mut rtgt);
                    diff_eq!(cr_, rr_, "unpack {fs} VALIDATE_ONLY return ({name})");
                    diff_eq!(ctgt == sentinel_json(), rtgt == sentinel_json(),
                             "unpack {fs} VALIDATE_ONLY leaves the target ({name})");
                    assert!(ctgt == sentinel_json(),
                            "C: {fs} under VALIDATE_ONLY must not write the target");
                    diff_eq!((*croot).refcount, (*rroot).refcount,
                             "unpack {fs} VALIDATE_ONLY refcount ({name})");
                    assert_eq!((*croot).refcount, cbefore,
                               "C: {fs} under VALIDATE_ONLY must not incref ({name})");
                }
            }
            decref(c, croot);
            decref(r, rroot);
        }
        // o/O reaching into a container, plus the extracted value's dump.
        upk!(c, r, "{\"a\":[1,2],\"b\":{\"c\":3}}", 0, cs("{s:o,s:O}"), sl,
             [cs("a").as_ptr(), op(sl, 0), cs("b").as_ptr(), op(sl, 1)], &[1usize],
             "unpack {{s:o,s:O}}");
        upk!(c, r, "[[1],{\"k\":2},\"s\",4,5.5,true,false,null]", 0,
             cs("[o,o,o,o,o,o,o,o]"), sl,
             [op(sl, 0), op(sl, 1), op(sl, 2), op(sl, 3), op(sl, 4), op(sl, 5), op(sl, 6),
              op(sl, 7)], &[], "unpack 8 o targets");
    }
}

// ===========================================================================
// Rows 263-269 — unpack containers
// ===========================================================================

#[test]
fn r263_unpack_empty_container_formats() {
    let (c, r) = both();
    unsafe {
        // With flags = 0 an empty format is non-strict, so extra content is fine.
        for text in ["{}", "{\"a\":1}", "{\"a\":1,\"b\":2}"] {
            upk!(c, r, text, 0, cs("{}"), sl, [], &[], "unpack {{}} from {text}");
        }
        for text in ["[]", "[1]", "[1,2,3]"] {
            upk!(c, r, text, 0, cs("[]"), sl, [], &[], "unpack [] from {text}");
        }
        // Type mismatches: "Expected object/array, got <type>".
        for (text, name) in ROOTS {
            upk!(c, r, text, 0, cs("{}"), sl, [], &[], "unpack {{}} from {text} ({name})");
            upk!(c, r, text, 0, cs("[]"), sl, [], &[], "unpack [] from {text} ({name})");
            let croot = load(c, text);
            if name != "object" {
                let mut cerr = json_error_t::poisoned();
                let f = cs("{}");
                assert_eq!((c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr()), -1,
                           "C: {{}} on {text} must fail");
                assert_eq!(cerr.snapshot().3, "<validation>", "C: source for {text}");
                assert_eq!(cerr.snapshot().4, format!("Expected object, got {name}"),
                           "C: text for {text}");
            }
            if name != "array" {
                let mut cerr = json_error_t::poisoned();
                let f = cs("[]");
                assert_eq!((c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr()), -1,
                           "C: [] on {text} must fail");
                assert_eq!(cerr.snapshot().4, format!("Expected array, got {name}"),
                           "C: text for {text}");
            }
            decref(c, croot);
        }
    }
}

#[test]
fn r264_unpack_object_keys() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0264);
    unsafe {
        let a = cs("a");
        let b = cs("b");
        upk!(c, r, "{\"a\":1}", 0, cs("{s:i}"), sl, [a.as_ptr(), ip(sl, 0)], &[],
             "unpack {{s:i}}");
        upk!(c, r, "{\"a\":1,\"b\":\"txt\"}", 0, cs("{s:i,s:s}"), sl,
             [a.as_ptr(), ip(sl, 1), b.as_ptr(), sp(sl, 1)], &[], "unpack {{s:i,s:s}}");
        // Keys are looked up with json_object_getn(root, key, strlen(key)).
        let utf = cs("k\u{e9}\u{20ac}");
        upk!(c, r, "{\"k\\u00e9\\u20ac\":5}", 0, cs("{s:i}"), sl, [utf.as_ptr(), ip(sl, 2)],
             &[], "unpack UTF-8 key");
        let empty = cs("");
        upk!(c, r, "{\"\":5}", 0, cs("{s:i}"), sl, [empty.as_ptr(), ip(sl, 3)], &[],
             "unpack empty key");
        // 12 pairs: the key_set hashtable inside unpack_object rehashes.
        let keys: Vec<CString> = (0..12).map(|i| cs(&format!("k{i:02}"))).collect();
        let text12: String = format!(
            "{{{}}}",
            (0..12).map(|i| format!("\"k{i:02}\":{i}")).collect::<Vec<_>>().join(",")
        );
        let fmt12: String = format!("{{{}}}", vec!["s:i"; 12].join(","));
        upk!(c, r, &text12, 0, cs(&fmt12), sl,
             [keys[0].as_ptr(), ip(sl, 0), keys[1].as_ptr(), ip(sl, 1),
              keys[2].as_ptr(), ip(sl, 2), keys[3].as_ptr(), ip(sl, 3),
              keys[4].as_ptr(), ip(sl, 4), keys[5].as_ptr(), ip(sl, 5),
              keys[6].as_ptr(), ip(sl, 6), keys[7].as_ptr(), ip(sl, 7),
              keys[8].as_ptr(), ip(sl, 8), keys[9].as_ptr(), ip(sl, 9),
              keys[10].as_ptr(), ip(sl, 10), keys[11].as_ptr(), ip(sl, 11)],
             &[], "unpack 12-pair object");
        // ... and with JSON_STRICT, so the whole key_set scan runs too.
        upk!(c, r, &text12, JSON_STRICT, cs(&fmt12), sl,
             [keys[0].as_ptr(), ip(sl, 0), keys[1].as_ptr(), ip(sl, 1),
              keys[2].as_ptr(), ip(sl, 2), keys[3].as_ptr(), ip(sl, 3),
              keys[4].as_ptr(), ip(sl, 4), keys[5].as_ptr(), ip(sl, 5),
              keys[6].as_ptr(), ip(sl, 6), keys[7].as_ptr(), ip(sl, 7),
              keys[8].as_ptr(), ip(sl, 8), keys[9].as_ptr(), ip(sl, 9),
              keys[10].as_ptr(), ip(sl, 10), keys[11].as_ptr(), ip(sl, 11)],
             &[], "unpack 12-pair object STRICT");
        for i in 0..1500 {
            // Keys go through json_loads as literal JSON text, so keep to bytes
            // that need no escaping.
            let kn: String = (0..rng.below(9))
                .map(|_| *rng.choice(&['a', 'b', 'Z', '0', '9', '-', '_', ' ', '.', '!']))
                .collect();
            let v = rng.range(-1000, 1000);
            let text = format!("{{\"{kn}\":{v}}}");
            let key = cs(&kn);
            upk!(c, r, &text, 0, cs("{s:I}"), sl, [key.as_ptr(), i64p(sl, 0)], &[],
                 "iter {i}: random key {kn:?}");
        }
    }
}

#[test]
fn r265_unpack_missing_and_null_keys() {
    let (c, r) = both();
    unsafe {
        let missing = cs("nope");
        upk!(c, r, "{\"a\":1}", 0, cs("{s:i}"), sl, [missing.as_ptr(), ip(sl, 0)], &[],
             "unpack missing key");
        let croot = load(c, "{\"a\":1}");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        let f = cs("{s:i}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), missing.as_ptr(), &mut t),
            -1,
            "C: missing key must fail"
        );
        assert_eq!(cerr.snapshot().3, "<validation>", "C: source");
        assert_eq!(cerr.snapshot().4, "Object item not found: nope", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_ITEM_NOT_FOUND, "C: code");
        // A NULL key vararg -> "NULL object key", <args>.
        let mut cerr = json_error_t::poisoned();
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), std::ptr::null::<c_char>(),
                               &mut t),
            -1,
            "C: NULL key must fail"
        );
        assert_eq!(cerr.snapshot().3, "<args>", "C: source");
        assert_eq!(cerr.snapshot().4, "NULL object key", "C: text");
        decref(c, croot);
        upk!(c, r, "{\"a\":1}", 0, cs("{s:i}"), sl,
             [std::ptr::null::<c_char>(), ip(sl, 1)], &[], "unpack NULL key");
        // Missing key deeper in the format, after a successful one.
        let a = cs("a");
        upk!(c, r, "{\"a\":1}", 0, cs("{s:i,s:i}"), sl,
             [a.as_ptr(), ip(sl, 2), missing.as_ptr(), ip(sl, 3)], &[],
             "unpack second key missing");
        // Key present but of the wrong type.
        upk!(c, r, "{\"a\":\"x\"}", 0, cs("{s:i}"), sl, [a.as_ptr(), ip(sl, 4)], &[],
             "unpack key of wrong type");
    }
}

#[test]
fn r266_unpack_optional_keys() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let b = cs("b");
        let z = cs("zz");
        // Present: the target IS assigned.
        upk!(c, r, "{\"a\":1}", 0, cs("{s?i}"), sl, [a.as_ptr(), ip(sl, 0)], &[],
             "unpack {{s?i}} present");
        // Absent: value == NULL, unpack runs in skipping mode, target untouched.
        upk!(c, r, "{\"a\":1}", 0, cs("{s?i}"), sl, [z.as_ptr(), ip(sl, 1)], &[],
             "unpack {{s?i}} absent");
        upk!(c, r, "{}", 0, cs("{s?i}"), sl, [z.as_ptr(), ip(sl, 2)], &[],
             "unpack {{s?i}} on empty object");
        // Mixed optional/required.
        upk!(c, r, "{\"b\":\"t\"}", 0, cs("{s?i,s:s}"), sl,
             [a.as_ptr(), ip(sl, 3), b.as_ptr(), sp(sl, 3)], &[], "unpack {{s?i,s:s}}");
        upk!(c, r, "{\"a\":1,\"b\":\"t\"}", 0, cs("{s?i,s:s}"), sl,
             [a.as_ptr(), ip(sl, 4), b.as_ptr(), sp(sl, 4)], &[],
             "unpack {{s?i,s:s}} both present");
        // Several consecutive optional keys, none present.
        upk!(c, r, "{}", 0, cs("{s?i,s?s,s?f,s?b,s?n,s?o}"), sl,
             [a.as_ptr(), ip(sl, 5), b.as_ptr(), sp(sl, 5), z.as_ptr(), dp(sl, 5),
              a.as_ptr(), ip(sl, 6), b.as_ptr(), z.as_ptr(), op(sl, 5)], &[],
             "unpack six absent optional keys");
        // The same, all present.
        upk!(c, r, "{\"a\":1,\"b\":\"t\",\"zz\":null}", 0, cs("{s?i,s?s,s?n}"), sl,
             [a.as_ptr(), ip(sl, 7), b.as_ptr(), sp(sl, 7), z.as_ptr()], &[],
             "unpack three present optional keys");
        // `?` on an optional key whose value is the wrong type still errors.
        upk!(c, r, "{\"a\":\"x\"}", 0, cs("{s?i}"), sl, [a.as_ptr(), ip(sl, 8)], &[],
             "unpack {{s?i}} wrong type");
    }
}

#[test]
fn r267_unpack_optional_container_values_skip_mode() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let b = cs("b");
        let z = cs("zz");
        // Absent key whose value is a container: the nested unpack_object /
        // unpack_array runs with root == NULL. The varargs are still consumed
        // but nothing is written and no index/type error is raised.
        upk!(c, r, "{}", 0, cs("{s?{s:i}}"), sl, [z.as_ptr(), a.as_ptr(), ip(sl, 0)], &[],
             "unpack {{s?{{s:i}}}} absent");
        upk!(c, r, "{\"zz\":{\"a\":9}}", 0, cs("{s?{s:i}}"), sl,
             [z.as_ptr(), a.as_ptr(), ip(sl, 1)], &[], "unpack {{s?{{s:i}}}} present");
        upk!(c, r, "{}", 0, cs("{s?[i,i]}"), sl, [z.as_ptr(), ip(sl, 2), ip(sl, 3)], &[],
             "unpack {{s?[i,i]}} absent");
        upk!(c, r, "{\"zz\":[4,5]}", 0, cs("{s?[i,i]}"), sl,
             [z.as_ptr(), ip(sl, 4), ip(sl, 5)], &[], "unpack {{s?[i,i]}} present");
        // Nested two levels of skipping, with every scalar kind inside.
        upk!(c, r, "{}", 0, cs("{s?{s:[i,I,f,F,b,n,s,o,O]}}"), sl,
             [z.as_ptr(), a.as_ptr(), ip(sl, 6), i64p(sl, 6), dp(sl, 6), dp(sl, 7),
              ip(sl, 7), sp(sl, 6), op(sl, 6), op(sl, 7)], &[],
             "unpack deep skip mode");
        // Skipping mode also ignores a missing *inner* key and out-of-range index.
        upk!(c, r, "{}", 0, cs("{s?{s:i}}"), sl, [z.as_ptr(), b.as_ptr(), ip(sl, 8)], &[],
             "unpack skip mode ignores inner missing key");
        upk!(c, r, "{}", 0, cs("{s?[i,i,i,i,i]}"), sl,
             [z.as_ptr(), ip(sl, 9), ip(sl, 10), ip(sl, 11), ip(sl, 12), ip(sl, 13)], &[],
             "unpack skip mode ignores index range");
        // ... but a *format* error inside the skipped part still fires.
        upk!(c, r, "{}", 0, cs("{s?{q}}"), sl, [z.as_ptr()], &[],
             "unpack skip mode still reports format errors");
        // JSON_STRICT inside skipping mode: `root == NULL` so no key scan runs.
        upk!(c, r, "{}", JSON_STRICT, cs("{s?{s:i}!}"), sl,
             [z.as_ptr(), a.as_ptr(), ip(sl, 14)], &[], "unpack skip mode with STRICT");
    }
}

#[test]
fn r268_unpack_arrays() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0268);
    unsafe {
        upk!(c, r, "[1,2]", 0, cs("[i,i]"), sl, [ip(sl, 0), ip(sl, 1)], &[], "unpack [i,i]");
        upk!(c, r, "[\"s\",1,1.5,true,null,7]", 0, cs("[s,i,f,b,n,o]"), sl,
             [sp(sl, 0), ip(sl, 2), dp(sl, 0), ip(sl, 3), op(sl, 0)], &[],
             "unpack [s,i,f,b,n,o]");
        // A 20-element array fully unpacked.
        let text20: String = format!("[{}]", (0..20).map(|i| i.to_string())
                                                    .collect::<Vec<_>>().join(","));
        let fmt20: String = format!("[{}]", vec!["i"; 20].join(","));
        upk!(c, r, &text20, 0, cs(&fmt20), sl,
             [ip(sl, 0), ip(sl, 1), ip(sl, 2), ip(sl, 3), ip(sl, 4), ip(sl, 5), ip(sl, 6),
              ip(sl, 7), ip(sl, 8), ip(sl, 9), ip(sl, 10), ip(sl, 11), ip(sl, 12),
              ip(sl, 13), ip(sl, 14), ip(sl, 15), ip(sl, 16), ip(sl, 17), ip(sl, 18),
              ip(sl, 19)], &[], "unpack 20-element array");
        // Index out of range.
        upk!(c, r, "[1,2]", 0, cs("[i,i,i]"), sl, [ip(sl, 0), ip(sl, 1), ip(sl, 2)], &[],
             "unpack [i,i,i] on a 2-element array");
        let croot = load(c, "[1,2]");
        let mut cerr = json_error_t::poisoned();
        let mut t: [c_int; 3] = [POISON_I32; 3];
        let tp = t.as_mut_ptr();
        let f = cs("[i,i,i]");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), tp, tp.add(1), tp.add(2)),
            -1,
            "C: too many elements must fail"
        );
        assert_eq!(cerr.snapshot().3, "<validation>", "C: source");
        assert_eq!(cerr.snapshot().4, "Array index 2 out of range", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_INDEX_OUT_OF_RANGE, "C: code");
        decref(c, croot);
        upk!(c, r, "[]", 0, cs("[i]"), sl, [ip(sl, 0)], &[], "unpack [i] on []");
        for i in 0..1500 {
            let n = rng.below(6);
            let text = format!(
                "[{}]",
                (0..n).map(|_| rng.range(-500, 500).to_string()).collect::<Vec<_>>().join(",")
            );
            let m = rng.below(6);
            let fmt = format!("[{}]", vec!["i"; m].join(","));
            upk!(c, r, &text, 0, cs(&fmt), sl,
                 [ip(sl, 0), ip(sl, 1), ip(sl, 2), ip(sl, 3), ip(sl, 4)], &[],
                 "iter {i}: {n} elements vs {m} format slots");
        }
    }
}

#[test]
fn r269_unpack_value_starters_inside_arrays() {
    let (c, r) = both();
    unsafe {
        // Every character of `unpack_value_starters` == "{[siIbfFOon".
        upk!(c, r, "[{}]", 0, cs("[{}]"), sl, [], &[], "starter '{{'");
        upk!(c, r, "[[]]", 0, cs("[[]]"), sl, [], &[], "starter '['");
        upk!(c, r, "[\"x\"]", 0, cs("[s]"), sl, [sp(sl, 0)], &[], "starter 's'");
        upk!(c, r, "[1]", 0, cs("[i]"), sl, [ip(sl, 0)], &[], "starter 'i'");
        upk!(c, r, "[1]", 0, cs("[I]"), sl, [i64p(sl, 0)], &[], "starter 'I'");
        upk!(c, r, "[true]", 0, cs("[b]"), sl, [ip(sl, 1)], &[], "starter 'b'");
        upk!(c, r, "[1.5]", 0, cs("[f]"), sl, [dp(sl, 0)], &[], "starter 'f'");
        upk!(c, r, "[1.5]", 0, cs("[F]"), sl, [dp(sl, 1)], &[], "starter 'F'");
        upk!(c, r, "[1]", 0, cs("[O]"), sl, [op(sl, 0)], &[0usize], "starter 'O'");
        upk!(c, r, "[1]", 0, cs("[o]"), sl, [op(sl, 1)], &[], "starter 'o'");
        upk!(c, r, "[null]", 0, cs("[n]"), sl, [], &[], "starter 'n'");
        // All eleven in one array.
        upk!(c, r, "[{},[],\"x\",1,2,true,1.5,2.5,3,4,null]", 0,
             cs("[{},[],s,i,I,b,f,F,O,o,n]"), sl,
             [sp(sl, 2), ip(sl, 2), i64p(sl, 2), ip(sl, 3), dp(sl, 2), dp(sl, 3),
              op(sl, 2), op(sl, 3)], &[2usize], "all value starters");
        // Anything else -> "Unexpected format character '%c'" from unpack_array.
        for ch in ['x', '#', '%', '?', 'q', 'S', 'd', ':', '{', '}'] {
            if ch == '{' {
                continue; // '{' is a starter
            }
            let fmt = format!("[{ch}]");
            upk!(c, r, "[1]", 0, cs(&fmt), sl, [ip(sl, 4)], &[], "rejected starter {ch:?}");
        }
        let croot = load(c, "[1]");
        let mut cerr = json_error_t::poisoned();
        let f = cs("[x]");
        assert_eq!((c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr()), -1,
                   "C: [x] must fail");
        assert_eq!(cerr.snapshot().3, "<format>", "C: source");
        assert_eq!(cerr.snapshot().4, "Unexpected format character 'x'", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_INVALID_FORMAT, "C: code");
        decref(c, croot);
        // The starters gate applies only INSIDE an array: at top level and as an
        // object value the `unpack` switch is entered directly.
        upk!(c, r, "1", 0, cs("F"), sl, [dp(sl, 5)], &[], "top-level F is fine");
        upk!(c, r, "{\"a\":1}", 0, cs("{s:F}"), sl, [cs("a").as_ptr(), dp(sl, 6)], &[],
             "object value F is fine");
    }
}

// ===========================================================================
// Rows 270-278 — JSON_STRICT, trailing `!` / `*`, JSON_VALIDATE_ONLY
// ===========================================================================

#[test]
fn r270_unpack_strict_objects() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        upk!(c, r, "{\"a\":1}", JSON_STRICT, cs("{s:i}"), sl, [a.as_ptr(), ip(sl, 0)], &[],
             "STRICT exact match");
        upk!(c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, cs("{s:i}"), sl,
             [a.as_ptr(), ip(sl, 1)], &[], "STRICT one extra key");
        let croot = load(c, "{\"a\":1,\"b\":2}");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        let f = cs("{s:i}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, f.as_ptr(), a.as_ptr(),
                               &mut t),
            -1,
            "C: STRICT with an extra key must fail"
        );
        assert_eq!(cerr.snapshot().3, "<validation>", "C: source");
        assert_eq!(cerr.snapshot().4, "1 object item(s) left unpacked: b", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_END_OF_INPUT_EXPECTED, "C: code");
        decref(c, croot);
        // Three extra keys: the strbuffer accumulation path joins them with ", ".
        upk!(c, r, "{\"a\":1,\"b\":2,\"c\":3,\"d\":4}", JSON_STRICT, cs("{s:i}"), sl,
             [a.as_ptr(), ip(sl, 2)], &[], "STRICT three extra keys");
        upk!(c, r, "{\"a\":1,\"b\":2,\"c\":3,\"d\":4}", JSON_STRICT, cs("{s:i,s:i}"), sl,
             [a.as_ptr(), ip(sl, 3), cs("b").as_ptr(), ip(sl, 4)], &[],
             "STRICT two extra keys");
        // Empty format with STRICT.
        upk!(c, r, "{}", JSON_STRICT, cs("{}"), sl, [], &[], "STRICT {{}} on {{}}");
        upk!(c, r, "{\"a\":1}", JSON_STRICT, cs("{}"), sl, [], &[], "STRICT {{}} on 1 key");
        // Many extra keys, so the unrecognized-key strbuffer has to grow.
        let text: String = format!(
            "{{{}}}",
            (0..30).map(|i| format!("\"k{i:02}\":{i}")).collect::<Vec<_>>().join(",")
        );
        upk!(c, r, &text, JSON_STRICT, cs("{}"), sl, [], &[], "STRICT 30 extra keys");
        upk!(c, r, &text, JSON_STRICT, cs("{s:i}"), sl, [cs("k00").as_ptr(), ip(sl, 5)],
             &[], "STRICT 29 extra keys");
    }
}

#[test]
fn r271_unpack_strict_arrays() {
    let (c, r) = both();
    unsafe {
        upk!(c, r, "[1]", JSON_STRICT, cs("[i]"), sl, [ip(sl, 0)], &[],
             "STRICT array exact match");
        upk!(c, r, "[1,2,3]", JSON_STRICT, cs("[i]"), sl, [ip(sl, 1)], &[],
             "STRICT array two left");
        let croot = load(c, "[1,2,3]");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        let f = cs("[i]");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, f.as_ptr(), &mut t),
            -1,
            "C: STRICT short array format must fail"
        );
        assert_eq!(cerr.snapshot().3, "<validation>", "C: source");
        assert_eq!(cerr.snapshot().4, "2 array item(s) left unpacked", "C: text");
        assert_eq!(cerr.code(), JSON_ERROR_END_OF_INPUT_EXPECTED, "C: code");
        decref(c, croot);
        upk!(c, r, "[]", JSON_STRICT, cs("[]"), sl, [], &[], "STRICT [] on []");
        for text in ["[1]", "[1,2]", "[1,2,3,4,5,6,7,8,9,10]"] {
            upk!(c, r, text, JSON_STRICT, cs("[]"), sl, [], &[],
                 "STRICT [] on {text}");
        }
        upk!(c, r, "[1,2]", JSON_STRICT, cs("[i,i]"), sl, [ip(sl, 2), ip(sl, 3)], &[],
             "STRICT [i,i] on [1,2]");
    }
}

#[test]
fn r272_unpack_trailing_bang_and_star_in_objects() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        // Trailing `!` sets strict = 1 even with flags = 0.
        upk!(c, r, "{\"a\":1}", 0, cs("{s:i!}"), sl, [a.as_ptr(), ip(sl, 0)], &[],
             "{{s:i!}} exact");
        upk!(c, r, "{\"a\":1,\"b\":2}", 0, cs("{s:i!}"), sl, [a.as_ptr(), ip(sl, 1)], &[],
             "{{s:i!}} extra key");
        // Trailing `*` sets strict = -1, which SUPPRESSES the JSON_STRICT
        // promotion (`strict == 0 && (flags & JSON_STRICT)`).
        for flags in [0, JSON_STRICT] {
            upk!(c, r, "{\"a\":1}", flags, cs("{s:i*}"), sl, [a.as_ptr(), ip(sl, 2)], &[],
                 "{{s:i*}} exact flags={flags}");
            upk!(c, r, "{\"a\":1,\"b\":2,\"c\":3}", flags, cs("{s:i*}"), sl,
                 [a.as_ptr(), ip(sl, 3)], &[], "{{s:i*}} extra keys flags={flags}");
        }
        // Pin it in the C: `*` must succeed where plain JSON_STRICT fails.
        let croot = load(c, "{\"a\":1,\"b\":2}");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        let fstar = cs("{s:i*}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, fstar.as_ptr(), a.as_ptr(),
                               &mut t),
            0,
            "C: trailing '*' must suppress the JSON_STRICT promotion"
        );
        let fbang = cs("{s:i!}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, fbang.as_ptr(), a.as_ptr(), &mut t),
            -1,
            "C: trailing '!' must be strict even with flags = 0"
        );
        decref(c, croot);
        // `!` / `*` as the ONLY thing in the format.
        for f in ["{!}", "{*}"] {
            for text in ["{}", "{\"a\":1}", "{\"a\":1,\"b\":2}"] {
                for flags in [0, JSON_STRICT] {
                    upk!(c, r, text, flags, cs(f), sl, [], &[],
                         "unpack {f} on {text} flags={flags}");
                }
            }
        }
        // Both a `!` and a `*`: the second one overwrites `strict` only if the
        // loop reaches it — it does not, because strict != 0 raises an error
        // first. Covered by row 274.
        upk!(c, r, "{\"a\":1}", JSON_STRICT, cs("{s:i!}"), sl, [a.as_ptr(), ip(sl, 4)],
             &[], "{{s:i!}} with JSON_STRICT too");
    }
}

#[test]
fn r273_unpack_trailing_bang_and_star_in_arrays() {
    let (c, r) = both();
    unsafe {
        for flags in [0, JSON_STRICT] {
            for text in ["[1]", "[1,2,3]"] {
                for f in ["[i!]", "[i*]"] {
                    upk!(c, r, text, flags, cs(f), sl, [ip(sl, 0)], &[],
                         "unpack {f} on {text} flags={flags}");
                }
                for f in ["[!]", "[*]"] {
                    upk!(c, r, text, flags, cs(f), sl, [], &[],
                         "unpack {f} on {text} flags={flags}");
                }
            }
            upk!(c, r, "[]", flags, cs("[!]"), sl, [], &[], "unpack [!] on [] flags={flags}");
            upk!(c, r, "[]", flags, cs("[*]"), sl, [], &[], "unpack [*] on [] flags={flags}");
        }
        // In the C: `[i*]` accepts a longer array even under JSON_STRICT.
        let croot = load(c, "[1,2,3]");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        let f = cs("[i*]");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, f.as_ptr(), &mut t),
            0,
            "C: '[i*]' must suppress the JSON_STRICT promotion"
        );
        let f = cs("[i!]");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut t),
            -1,
            "C: '[i!]' must be strict even with flags = 0"
        );
        assert_eq!(cerr.snapshot().4, "2 array item(s) left unpacked", "C: text");
        decref(c, croot);
    }
}

#[test]
fn r274_unpack_tokens_after_bang_or_star() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        // "Expected '}' after '%c', got '%c'"
        for f in ["{s:i!s:i}", "{s:i*s:i}", "{!s:i}", "{*s:i}", "{s:i!!}", "{s:i*!}",
                  "{s:i!*}", "{s:i**}"] {
            upk!(c, r, "{\"a\":1,\"b\":2}", 0, cs(f), sl,
                 [a.as_ptr(), ip(sl, 0), cs("b").as_ptr(), ip(sl, 1)], &[],
                 "unpack {f}");
        }
        let croot = load(c, "{\"a\":1,\"b\":2}");
        let mut cerr = json_error_t::poisoned();
        let mut t: [c_int; 2] = [POISON_I32; 2];
        let tp = t.as_mut_ptr();
        let f = cs("{s:i!s:i}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), a.as_ptr(), tp,
                               cs("b").as_ptr(), tp.add(1)),
            -1,
            "C: token after '!' must fail"
        );
        assert_eq!(cerr.snapshot().3, "<format>", "C: source");
        assert_eq!(cerr.snapshot().4, "Expected '}' after '!', got 's'", "C: text");
        let f = cs("{s:i*s:i}");
        let mut cerr = json_error_t::poisoned();
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), a.as_ptr(), tp,
                               cs("b").as_ptr(), tp.add(1)),
            -1,
            "C: token after '*' must fail"
        );
        assert_eq!(cerr.snapshot().4, "Expected '}' after '*', got 's'", "C: text");
        decref(c, croot);

        // "Expected ']' after '%c', got '%c'"
        for f in ["[i!i]", "[i*i]", "[!i]", "[*i]", "[i!!]", "[i*s]"] {
            upk!(c, r, "[1,2]", 0, cs(f), sl, [ip(sl, 2), ip(sl, 3)], &[], "unpack {f}");
        }
        let croot = load(c, "[1,2]");
        let mut cerr = json_error_t::poisoned();
        let f = cs("[i!i]");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), tp, tp.add(1)),
            -1,
            "C: token after '!' in an array must fail"
        );
        assert_eq!(cerr.snapshot().4, "Expected ']' after '!', got 'i'", "C: text");
        let f = cs("[i*i]");
        let mut cerr = json_error_t::poisoned();
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), tp, tp.add(1)),
            -1,
            "C: token after '*' in an array must fail"
        );
        assert_eq!(cerr.snapshot().4, "Expected ']' after '*', got 'i'", "C: text");
        decref(c, croot);
    }
}

#[test]
fn r275_unpack_strict_with_optional_keys() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        // `gotopt` forces the key scan even when the sizes match.
        for text in ["{}", "{\"a\":1}", "{\"b\":2}", "{\"a\":1,\"b\":2}"] {
            for f in ["{s?i}", "{s?i!}", "{s?i*}"] {
                upk!(c, r, text, JSON_STRICT, cs(f), sl, [a.as_ptr(), ip(sl, 0)], &[],
                     "unpack {f} on {text} with JSON_STRICT");
                upk!(c, r, text, 0, cs(f), sl, [a.as_ptr(), ip(sl, 1)], &[],
                     "unpack {f} on {text} with flags=0");
            }
        }
        // Pin the three canonical outcomes in the C.
        let mut t: c_int = POISON_I32;
        let f = cs("{s?i}");
        for (text, want) in [("{}", 0), ("{\"a\":1}", 0), ("{\"b\":2}", -1)] {
            let croot = load(c, text);
            let mut cerr = json_error_t::poisoned();
            assert_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, f.as_ptr(), a.as_ptr(),
                                   &mut t),
                want,
                "C: {{s?i}} on {text} with JSON_STRICT"
            );
            if want == -1 {
                assert_eq!(cerr.snapshot().4, "1 object item(s) left unpacked: b",
                           "C: text for {text}");
            }
            decref(c, croot);
        }
        // A mix of optional and required with extra keys.
        upk!(c, r, "{\"a\":1,\"b\":2,\"c\":3}", JSON_STRICT, cs("{s?i,s:i}"), sl,
             [a.as_ptr(), ip(sl, 2), cs("b").as_ptr(), ip(sl, 3)], &[],
             "STRICT optional + required + extra");
        upk!(c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, cs("{s?i,s:i}"), sl,
             [a.as_ptr(), ip(sl, 4), cs("b").as_ptr(), ip(sl, 5)], &[],
             "STRICT optional + required exact");
    }
}

#[test]
fn r276_unpack_strict_with_a_key_unpacked_twice() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        // key_set.size (1) != json_object_size (2), so the scan runs and reports
        // the key that was never accessed.
        upk!(c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, cs("{s:i,s:i}"), sl,
             [a.as_ptr(), ip(sl, 0), a.as_ptr(), ip(sl, 1)], &[],
             "STRICT same key twice, 2-key root");
        let croot = load(c, "{\"a\":1,\"b\":2}");
        let mut cerr = json_error_t::poisoned();
        let mut t: [c_int; 2] = [POISON_I32; 2];
        let tp = t.as_mut_ptr();
        let f = cs("{s:i,s:i}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, f.as_ptr(), a.as_ptr(), tp,
                               a.as_ptr(), tp.add(1)),
            -1,
            "C: STRICT must notice the un-accessed key"
        );
        assert_eq!(cerr.snapshot().4, "1 object item(s) left unpacked: b", "C: text");
        decref(c, croot);
        // On a 1-key root the same format succeeds.
        upk!(c, r, "{\"a\":1}", JSON_STRICT, cs("{s:i,s:i}"), sl,
             [a.as_ptr(), ip(sl, 2), a.as_ptr(), ip(sl, 3)], &[],
             "STRICT same key twice, 1-key root");
        let croot = load(c, "{\"a\":1}");
        let mut cerr = json_error_t::poisoned();
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_STRICT, f.as_ptr(), a.as_ptr(), tp,
                               a.as_ptr(), tp.add(1)),
            0,
            "C: STRICT with a duplicated key on a 1-key root must succeed"
        );
        decref(c, croot);
        // Three times, and with different value formats for the same key.
        upk!(c, r, "{\"a\":1}", JSON_STRICT, cs("{s:i,s:I,s:F}"), sl,
             [a.as_ptr(), ip(sl, 4), a.as_ptr(), i64p(sl, 4), a.as_ptr(), dp(sl, 4)], &[],
             "STRICT same key three times");
    }
}

#[test]
fn r277_unpack_validate_only() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        // ---- SAME argument list, flags = 0 vs JSON_VALIDATE_ONLY. With a single
        // key nothing follows the (unconsumed) target, so the vararg walk stays
        // aligned and the asymmetry is directly observable.
        upk!(c, r, "{\"a\":1}", 0, cs("{s:i}"), sl, [a.as_ptr(), ip(sl, 0)], &[],
             "{{s:i}} flags=0 writes the target");
        upk!(c, r, "{\"a\":1}", JSON_VALIDATE_ONLY, cs("{s:i}"), sl,
             [a.as_ptr(), ip(sl, 0)], &[], "{{s:i}} VALIDATE_ONLY leaves the target");
        let croot = load(c, "{\"a\":1}");
        let f = cs("{s:i}");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        assert_eq!((c.json_unpack_ex)(croot, &mut cerr, JSON_VALIDATE_ONLY, f.as_ptr(),
                                      a.as_ptr(), &mut t), 0, "C: VALIDATE_ONLY succeeds");
        assert_eq!(t, POISON_I32, "C: VALIDATE_ONLY must not write the value target");
        assert_eq!((c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), a.as_ptr(), &mut t),
                   0, "C: flags=0 succeeds");
        assert_eq!(t, 1, "C: flags=0 must write the value target");
        decref(c, croot);

        // ---- arrays consume NO key varargs at all, so a multi-element format can
        // be compared with an identical argument list under both flag settings.
        for flags in [0, JSON_VALIDATE_ONLY] {
            upk!(c, r, "[\"s\",1,2,1.5,2.5,true,null,[false,3.5,null],7,8]", flags,
                 cs("[s,i,I,f,F,b,n,[b,f,n],o,O]"), sl,
                 [sp(sl, 0), ip(sl, 0), i64p(sl, 0), dp(sl, 0), dp(sl, 1), ip(sl, 1),
                  ip(sl, 2), dp(sl, 2), op(sl, 0), op(sl, 1)],
                 if flags == 0 { &[1usize][..] } else { &[][..] },
                 "array of every kind, flags={flags}");
        }

        // ---- the big object format from the row. Under JSON_VALIDATE_ONLY the
        // KEY varargs are still consumed but the value varargs are not, so a
        // correct caller passes keys only; the poisoned value block is passed
        // afterwards and must come back untouched.
        let ks: Vec<CString> = (0..10).map(|i| cs(&format!("k{i}"))).collect();
        let text = "{\"k0\":\"s\",\"k1\":1,\"k2\":2,\"k3\":1.5,\"k4\":2.5,\"k5\":true,\
                    \"k6\":null,\"k7\":[false,3.5,null],\"k8\":7,\"k9\":8}";
        let fmt = cs("{s:s,s:i,s:I,s:f,s:F,s:b,s:n,s:[b,f,n],s:o,s:O}");
        for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
            upk!(c, r, text, flags, fmt, sl,
                 [ks[0].as_ptr(), ks[1].as_ptr(), ks[2].as_ptr(), ks[3].as_ptr(),
                  ks[4].as_ptr(), ks[5].as_ptr(), ks[6].as_ptr(), ks[7].as_ptr(),
                  ks[8].as_ptr(), ks[9].as_ptr(),
                  // never consumed under VALIDATE_ONLY:
                  sp(sl, 0), ip(sl, 0), i64p(sl, 0), dp(sl, 0), dp(sl, 1), ip(sl, 1),
                  ip(sl, 2), dp(sl, 2), op(sl, 0), op(sl, 1)],
                 &[], "big object VALIDATE_ONLY flags={flags}");
        }
        // In the C, every target must still be poison afterwards.
        let croot = load(c, text);
        let mut sl_ = Slots::poisoned();
        let s: *mut Slots = &mut sl_;
        let mut cerr = json_error_t::poisoned();
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, JSON_VALIDATE_ONLY, fmt.as_ptr(),
                ks[0].as_ptr(), ks[1].as_ptr(), ks[2].as_ptr(), ks[3].as_ptr(),
                ks[4].as_ptr(), ks[5].as_ptr(), ks[6].as_ptr(), ks[7].as_ptr(),
                ks[8].as_ptr(), ks[9].as_ptr(),
                sp(s, 0), ip(s, 0), i64p(s, 0), dp(s, 0), dp(s, 1), ip(s, 1), ip(s, 2),
                dp(s, 2), op(s, 0), op(s, 1)),
            0,
            "C: big VALIDATE_ONLY format must validate"
        );
        assert_eq!(
            sl_.summary(c),
            Slots::poisoned().summary(c),
            "C: VALIDATE_ONLY must not write ANY value target"
        );
        decref(c, croot);

        // ---- a type mismatch deep inside gives the same error as flags = 0.
        let bad = "{\"k0\":\"s\",\"k1\":1,\"k2\":2,\"k3\":1.5,\"k4\":2.5,\"k5\":true,\
                   \"k6\":null,\"k7\":[false,\"oops\",null],\"k8\":7,\"k9\":8}";
        // With JSON_VALIDATE_ONLY the value varargs are NOT consumed, so a caller
        // must pass keys only — that variant is checked separately below with an
        // argument list that stays aligned.
        for flags in [0usize] {
            let croot = load(c, bad);
            let rroot = load(r, bad);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let mut cs_ = Slots::poisoned();
            let mut rs_ = Slots::poisoned();
            let cs2: *mut Slots = &mut cs_;
            let rs2: *mut Slots = &mut rs_;
            let cret = (c.json_unpack_ex)(croot, &mut cerr, flags, fmt.as_ptr(),
                ks[0].as_ptr(), sp(cs2, 0), ks[1].as_ptr(), ip(cs2, 0),
                ks[2].as_ptr(), i64p(cs2, 0), ks[3].as_ptr(), dp(cs2, 0),
                ks[4].as_ptr(), dp(cs2, 1), ks[5].as_ptr(), ip(cs2, 1),
                ks[6].as_ptr(), ks[7].as_ptr(), ip(cs2, 2), dp(cs2, 2),
                ks[8].as_ptr(), op(cs2, 0), ks[9].as_ptr(), op(cs2, 1));
            let rret = (r.json_unpack_ex)(rroot, &mut rerr, flags, fmt.as_ptr(),
                ks[0].as_ptr(), sp(rs2, 0), ks[1].as_ptr(), ip(rs2, 0),
                ks[2].as_ptr(), i64p(rs2, 0), ks[3].as_ptr(), dp(rs2, 0),
                ks[4].as_ptr(), dp(rs2, 1), ks[5].as_ptr(), ip(rs2, 1),
                ks[6].as_ptr(), ks[7].as_ptr(), ip(rs2, 2), dp(rs2, 2),
                ks[8].as_ptr(), op(rs2, 0), ks[9].as_ptr(), op(rs2, 1));
            diff_eq!(cret, rret, "deep mismatch return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "deep mismatch error flags={flags}");
            assert_eq!(cret, -1, "C: deep mismatch must fail (flags={flags})");
            assert_eq!(cerr.snapshot().4, "Expected real, got string",
                       "C: deep mismatch text (flags={flags})");
            decref(c, croot);
            decref(r, rroot);
        }
        // The same deep mismatch under JSON_VALIDATE_ONLY: keys only, so the
        // vararg walk stays aligned. The error must be byte-identical to the
        // flags = 0 case above.
        for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
            upk!(c, r, bad, flags, fmt, sl,
                 [ks[0].as_ptr(), ks[1].as_ptr(), ks[2].as_ptr(), ks[3].as_ptr(),
                  ks[4].as_ptr(), ks[5].as_ptr(), ks[6].as_ptr(), ks[7].as_ptr(),
                  ks[8].as_ptr(), ks[9].as_ptr()], &[],
                 "deep mismatch VALIDATE_ONLY flags={flags}");
            let croot = load(c, bad);
            let mut cerr = json_error_t::poisoned();
            assert_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, flags, fmt.as_ptr(),
                    ks[0].as_ptr(), ks[1].as_ptr(), ks[2].as_ptr(), ks[3].as_ptr(),
                    ks[4].as_ptr(), ks[5].as_ptr(), ks[6].as_ptr(), ks[7].as_ptr(),
                    ks[8].as_ptr(), ks[9].as_ptr()),
                -1,
                "C: deep mismatch must fail under VALIDATE_ONLY too"
            );
            assert_eq!(cerr.snapshot().4, "Expected real, got string",
                       "C: VALIDATE_ONLY deep mismatch text");
            decref(c, croot);
        }
        // An array deep mismatch works with an identical argument list under both
        // flag settings, because arrays consume no key varargs.
        for flags in [0, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
            upk!(c, r, "[1,[2,\"oops\"]]", flags, cs("[i,[i,f]]"), sl,
                 [ip(sl, 10), ip(sl, 11), dp(sl, 10)], &[],
                 "array deep mismatch flags={flags}");
        }
    }
}

#[test]
fn r278_unpack_validate_only_with_strict() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let b = cs("b");
        for flags in [
            JSON_VALIDATE_ONLY,
            JSON_STRICT,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            0,
        ] {
            // exact-match object / extra keys / optional keys
            upk!(c, r, "{\"a\":1}", flags, cs("{s:i}"), sl, [a.as_ptr(), ip(sl, 0)], &[],
                 "obj exact flags={flags}");
            upk!(c, r, "{\"a\":1,\"b\":2}", flags, cs("{s:i}"), sl,
                 [a.as_ptr(), ip(sl, 1)], &[], "obj extra flags={flags}");
            upk!(c, r, "{\"a\":1}", flags, cs("{s?i}"), sl, [b.as_ptr(), ip(sl, 2)], &[],
                 "obj optional absent flags={flags}");
            upk!(c, r, "{}", flags, cs("{s?i}"), sl, [a.as_ptr(), ip(sl, 3)], &[],
                 "obj optional on empty flags={flags}");
            // the same three shapes for arrays (no key varargs at all)
            upk!(c, r, "[1]", flags, cs("[i]"), sl, [ip(sl, 4)], &[],
                 "arr exact flags={flags}");
            upk!(c, r, "[1,2,3]", flags, cs("[i]"), sl, [ip(sl, 5)], &[],
                 "arr extra flags={flags}");
            upk!(c, r, "[1,2,3]", flags, cs("[i*]"), sl, [ip(sl, 6)], &[],
                 "arr trailing star flags={flags}");
            upk!(c, r, "[1,2,3]", flags, cs("[i!]"), sl, [ip(sl, 7)], &[],
                 "arr trailing bang flags={flags}");
            // trailing `!` / `*` in objects under both flags
            upk!(c, r, "{\"a\":1,\"b\":2}", flags, cs("{s:i*}"), sl,
                 [a.as_ptr(), ip(sl, 8)], &[], "obj star flags={flags}");
            upk!(c, r, "{\"a\":1,\"b\":2}", flags, cs("{s:i!}"), sl,
                 [a.as_ptr(), ip(sl, 9)], &[], "obj bang flags={flags}");
        }
    }
}

// ===========================================================================
// Rows 279-281 — nesting, format errors and the token stream in unpack
// ===========================================================================

#[test]
fn r279_unpack_deeply_nested() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0279);
    unsafe {
        let a = cs("a");
        let b = cs("b");
        let d = cs("d");
        let good = "{\"a\":{\"b\":[{\"d\":1},\"txt\"]},\"b\":[[2,3],[4]]}";
        upk!(c, r, good, 0, cs("{s:{s:[{s:i},s]},s:[[i,i],[i]]}"), sl,
             [a.as_ptr(), b.as_ptr(), d.as_ptr(), ip(sl, 0), sp(sl, 0),
              b.as_ptr(), ip(sl, 1), ip(sl, 2), ip(sl, 3)], &[],
             "unpack 4-level nest");
        // A type mismatch at the deepest level.
        let bad = "{\"a\":{\"b\":[{\"d\":\"no\"},\"txt\"]},\"b\":[[2,3],[4]]}";
        upk!(c, r, bad, 0, cs("{s:{s:[{s:i},s]},s:[[i,i],[i]]}"), sl,
             [a.as_ptr(), b.as_ptr(), d.as_ptr(), ip(sl, 4), sp(sl, 1),
              b.as_ptr(), ip(sl, 5), ip(sl, 6), ip(sl, 7)], &[],
             "unpack 4-level nest with a deep mismatch");
        // JSON_STRICT with an extra key at the deepest object.
        let extra = "{\"a\":{\"b\":[{\"d\":1,\"e\":2},\"txt\"]},\"b\":[[2,3],[4]]}";
        for flags in [0, JSON_STRICT] {
            upk!(c, r, extra, flags, cs("{s:{s:[{s:i},s]},s:[[i,i],[i]]}"), sl,
                 [a.as_ptr(), b.as_ptr(), d.as_ptr(), ip(sl, 8), sp(sl, 2),
                  b.as_ptr(), ip(sl, 9), ip(sl, 10), ip(sl, 11)], &[],
                 "unpack deep extra key flags={flags}");
        }
        // 6 levels of pure arrays and pure objects.
        upk!(c, r, "[[[[[[7]]]]]]", 0, cs("[[[[[[i]]]]]]"), sl, [ip(sl, 12)], &[],
             "unpack 6-level arrays");
        upk!(c, r, "{\"a\":{\"a\":{\"a\":{\"a\":{\"a\":7}}}}}", 0,
             cs("{s:{s:{s:{s:{s:i}}}}}"), sl,
             [a.as_ptr(), a.as_ptr(), a.as_ptr(), a.as_ptr(), a.as_ptr(), ip(sl, 13)], &[],
             "unpack 5-level objects");
        for i in 0..1000 {
            let x = rng.range(-100, 100);
            let y = rng.range(-100, 100);
            let text = format!("{{\"a\":[{x},{{\"b\":[{y}]}}]}}");
            upk!(c, r, &text, 0, cs("{s:[i,{s:[I]}]}"), sl,
                 [a.as_ptr(), ip(sl, 14), b.as_ptr(), i64p(sl, 14)], &[],
                 "iter {i}: randomized nest");
        }
    }
}

#[test]
fn r280_unpack_format_errors() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        // "Unexpected end of format string"
        for f in ["{", "{s:i", "{s", "{s:", "[", "[i", "[i,", "[[", "{{"] {
            upk!(c, r, "{\"a\":1}", 0, cs(f), sl, [a.as_ptr(), ip(sl, 0)], &[],
                 "unpack unterminated {f:?}");
        }
        let croot = load(c, "{\"a\":1}");
        let mut cerr = json_error_t::poisoned();
        let f = cs("{");
        assert_eq!((c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr()), -1,
                   "C: '{{' must fail");
        assert_eq!(cerr.snapshot().3, "<format>", "C: source");
        assert_eq!(cerr.snapshot().4, "Unexpected end of format string", "C: text");
        // "Expected format 's', got '%c'"
        for f in ["{i:i}", "{n}", "{b}", "{[i]}", "{q}", "{f}", "{o}"] {
            upk!(c, r, "{\"a\":1}", 0, cs(f), sl, [ip(sl, 1)], &[],
                 "unpack bad key format {f:?}");
        }
        let f = cs("{i:i}");
        let mut cerr = json_error_t::poisoned();
        assert_eq!((c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr()), -1,
                   "C: {{i:i}} must fail");
        assert_eq!(cerr.snapshot().4, "Expected format 's', got 'i'", "C: text");
        // "Unexpected format character '%c'" from unpack's default arm.
        for f in ["{s:q}", "q", "x", "{s:x}", "#", "%", "+", "?", "!", "*", "]", "}"] {
            upk!(c, r, "{\"a\":1}", 0, cs(f), sl, [a.as_ptr(), ip(sl, 2)], &[],
                 "unpack unexpected char {f:?}");
        }
        let f = cs("{s:q}");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), a.as_ptr(), &mut t),
            -1,
            "C: {{s:q}} must fail"
        );
        assert_eq!(cerr.snapshot().4, "Unexpected format character 'q'", "C: text");
        // "Garbage after format string" from json_vunpack_ex's trailing check.
        for f in ["{s:i} i", "{s:i}i", "{s:i}{", "{s:i} q", "{s:i}]"] {
            upk!(c, r, "{\"a\":1}", 0, cs(f), sl, [a.as_ptr(), ip(sl, 3), ip(sl, 4)], &[],
                 "unpack garbage after {f:?}");
        }
        let f = cs("{s:i}i");
        let mut cerr = json_error_t::poisoned();
        let mut t2: c_int = POISON_I32;
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), a.as_ptr(), &mut t,
                               &mut t2),
            -1,
            "C: garbage after format must fail"
        );
        assert_eq!(cerr.snapshot().3, "<format>", "C: source");
        assert_eq!(cerr.snapshot().4, "Garbage after format string", "C: text");
        decref(c, croot);
        // Scalar roots with trailing garbage.
        for f in ["i i", "i x", "ii"] {
            upk!(c, r, "1", 0, cs(f), sl, [ip(sl, 5), ip(sl, 6)], &[],
                 "unpack scalar garbage {f:?}");
        }
    }
}

#[test]
fn r281_unpack_whitespace_and_separators() {
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let b = cs("b");
        let text = "{\"a\":1,\"b\":2}";
        for f in [
            "{s:i,s:i}",
            "{ s : i , s : i }",
            "{s i s i}",
            "{\ts\t:\ti\t,\ts\t:\ti}",
            "{,,s::i,,,s:i,}",
            "{\ns\n:\ni\n,\ns:i}",
            "  {s:i,s:i}  ",
            ":,{s:i s:i},:",
        ] {
            upk!(c, r, text, 0, cs(f), sl,
                 [a.as_ptr(), ip(sl, 0), b.as_ptr(), ip(sl, 1)], &[],
                 "unpack whitespace variant {f:?}");
            // Every variant must ALSO produce exactly the same result in the C.
            let croot = load(c, text);
            let fmt = cs(f);
            let mut cerr = json_error_t::poisoned();
            let mut t: [c_int; 2] = [POISON_I32; 2];
            let tp = t.as_mut_ptr();
            assert_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, 0, fmt.as_ptr(), a.as_ptr(), tp,
                                   b.as_ptr(), tp.add(1)),
                0,
                "C: whitespace variant {f:?} must succeed"
            );
            assert_eq!(t, [1, 2], "C: whitespace variant {f:?} must give the same values");
            decref(c, croot);
        }
        for f in ["[i,i]", "[ i , i ]", "[i i]", "[\ti\t,\ti]", "[,i,,i,]"] {
            upk!(c, r, "[1,2]", 0, cs(f), sl, [ip(sl, 2), ip(sl, 3)], &[],
                 "unpack array whitespace {f:?}");
        }
        // Multi-line failing formats: line/column/position must match exactly.
        for f in ["{\nq}", "\n\nq", "{s:i,\nq}", "[\n\nq]", "{s:i}\n\nq", "\n\t\nx"] {
            upk!(c, r, text, 0, cs(f), sl, [a.as_ptr(), ip(sl, 4), ip(sl, 5)], &[],
                 "unpack multiline error {f:?}");
            let croot = load(c, text);
            let fmt = cs(f);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let rroot = load(r, text);
            let mut t: [c_int; 2] = [POISON_I32; 2];
            let tp = t.as_mut_ptr();
            let cret = (c.json_unpack_ex)(croot, &mut cerr, 0, fmt.as_ptr(), a.as_ptr(),
                                          tp, tp.add(1));
            let rret = (r.json_unpack_ex)(rroot, &mut rerr, 0, fmt.as_ptr(), a.as_ptr(),
                                          tp, tp.add(1));
            diff_eq!(cret, rret, "multiline {f:?} return");
            diff_eq!(
                (cerr.line, cerr.column, cerr.position),
                (rerr.line, rerr.column, rerr.position),
                "multiline {f:?} line/column/position"
            );
            assert_eq!(cret, -1, "C: {f:?} must fail");
            decref(c, croot);
            decref(r, rroot);
        }
    }
}

// ===========================================================================
// Row 282 — json_vunpack_ex through a real va_list
// ===========================================================================

#[test]
fn r282_vunpack_ex_through_a_real_va_list() {
    let (c, r) = both();
    unsafe {
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vunpack_ex");
        let rfn = sym_addr("Rust", b"json_vunpack_ex");
        let k1 = cs("str");
        let k2 = cs("arr");
        let k3 = cs("obj");
        let fmt = cs("{s:s%,s:[i,I,F,b,n],s:O}");
        let text = "{\"str\":\"hello\",\"arr\":[1,2,3,true,null],\"obj\":{\"x\":1}}";

        for flags in [0, JSON_STRICT] {
            let croot = load(c, text);
            let rroot = load(r, text);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let mut cs_ = Slots::poisoned();
            let mut rs_ = Slots::poisoned();
            let cp: *mut Slots = &mut cs_;
            let rp: *mut Slots = &mut rs_;
            let cret = (sh.vunpack_ex)(cfn, croot, &mut cerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sp(cp, 0), lp(cp, 0),
                k2.as_ptr(), ip(cp, 0), i64p(cp, 0), dp(cp, 0), ip(cp, 1),
                k3.as_ptr(), op(cp, 0));
            let rret = (sh.vunpack_ex)(rfn, rroot, &mut rerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sp(rp, 0), lp(rp, 0),
                k2.as_ptr(), ip(rp, 0), i64p(rp, 0), dp(rp, 0), ip(rp, 1),
                k3.as_ptr(), op(rp, 0));
            diff_eq!(cret, rret, "vunpack_ex return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "vunpack_ex error image flags={flags}");
            diff_eq!(cs_.summary(c), rs_.summary(r), "vunpack_ex targets flags={flags}");
            assert_eq!(cret, 0, "C: vunpack_ex success path flags={flags}");
            cs_.decref_objs(c, &[0]);
            rs_.decref_objs(r, &[0]);
            decref(c, croot);
            decref(r, rroot);
        }

        // JSON_VALIDATE_ONLY consumes the KEY varargs only, so the aligned call
        // passes just the three keys; the value block must come back untouched.
        for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
            let croot = load(c, text);
            let rroot = load(r, text);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cs_ = Slots::poisoned();
            let rs_ = Slots::poisoned();
            let cret = (sh.vunpack_ex)(cfn, croot, &mut cerr, flags, fmt.as_ptr(),
                k1.as_ptr(), k2.as_ptr(), k3.as_ptr());
            let rret = (sh.vunpack_ex)(rfn, rroot, &mut rerr, flags, fmt.as_ptr(),
                k1.as_ptr(), k2.as_ptr(), k3.as_ptr());
            diff_eq!(cret, rret, "vunpack_ex VALIDATE_ONLY return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "vunpack_ex VALIDATE_ONLY error flags={flags}");
            diff_eq!(cs_.summary(c), rs_.summary(r), "vunpack_ex VALIDATE_ONLY targets");
            // NOTE: under JSON_VALIDATE_ONLY the whole `case 's'` body is skipped,
            // so the `%` after the `s` is never consumed by the string handler and
            // unpack_object then sees it as a key token: the C reports
            // "Expected format 's', got '%'". That is a genuine asymmetry of the
            // C, so `s%` + VALIDATE_ONLY is an ERROR, not a validation pass.
            assert_eq!(cret, -1, "C: s% under VALIDATE_ONLY is a format error");
            assert_eq!(cerr.snapshot().4, "Expected format 's', got '%'",
                       "C: VALIDATE_ONLY s% error text");
            assert_eq!(cs_.summary(c), Slots::poisoned().summary(c),
                       "C: VALIDATE_ONLY wrote a value target");
            decref(c, croot);
            decref(r, rroot);
        }
        // The same shape without `%` DOES validate under JSON_VALIDATE_ONLY, and
        // leaves every value target poisoned.
        let fmt_nopct = cs("{s:s,s:[i,I,F,b,n],s:O}");
        for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
            let croot = load(c, text);
            let rroot = load(r, text);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cs_ = Slots::poisoned();
            let rs_ = Slots::poisoned();
            let cret = (sh.vunpack_ex)(cfn, croot, &mut cerr, flags, fmt_nopct.as_ptr(),
                k1.as_ptr(), k2.as_ptr(), k3.as_ptr());
            let rret = (sh.vunpack_ex)(rfn, rroot, &mut rerr, flags, fmt_nopct.as_ptr(),
                k1.as_ptr(), k2.as_ptr(), k3.as_ptr());
            diff_eq!(cret, rret, "vunpack_ex VALIDATE_ONLY (no %) return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "vunpack_ex VALIDATE_ONLY (no %) error");
            diff_eq!(cs_.summary(c), rs_.summary(r),
                     "vunpack_ex VALIDATE_ONLY (no %) targets");
            assert_eq!(cret, 0, "C: VALIDATE_ONLY must validate flags={flags}");
            assert_eq!(cs_.summary(c), Slots::poisoned().summary(c),
                       "C: VALIDATE_ONLY wrote a value target");
            decref(c, croot);
            decref(r, rroot);
        }

        // The EARLY -1 return path: the mismatch is in the first key's value, so
        // only some of the varargs have been consumed when it bails out.
        let bad = "{\"str\":7,\"arr\":[1,2,3,true,null],\"obj\":{\"x\":1}}";
        for flags in [0, JSON_STRICT] {
            let croot = load(c, bad);
            let rroot = load(r, bad);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let mut cs_ = Slots::poisoned();
            let mut rs_ = Slots::poisoned();
            let cp: *mut Slots = &mut cs_;
            let rp: *mut Slots = &mut rs_;
            let cret = (sh.vunpack_ex)(cfn, croot, &mut cerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sp(cp, 0), lp(cp, 0),
                k2.as_ptr(), ip(cp, 0), i64p(cp, 0), dp(cp, 0), ip(cp, 1),
                k3.as_ptr(), op(cp, 0));
            let rret = (sh.vunpack_ex)(rfn, rroot, &mut rerr, flags, fmt.as_ptr(),
                k1.as_ptr(), sp(rp, 0), lp(rp, 0),
                k2.as_ptr(), ip(rp, 0), i64p(rp, 0), dp(rp, 0), ip(rp, 1),
                k3.as_ptr(), op(rp, 0));
            diff_eq!(cret, rret, "vunpack_ex early error return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "vunpack_ex early error image flags={flags}");
            diff_eq!(cs_.summary(c), rs_.summary(r), "vunpack_ex early error targets");
            assert_eq!(cret, -1, "C: early error must return -1");
            decref(c, croot);
            decref(r, rroot);
        }

        // A NULL root and a NULL format straight through the va_list entry point.
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cret = (sh.vunpack_ex)(cfn, std::ptr::null_mut(), &mut cerr, 0, fmt.as_ptr());
        let rret = (sh.vunpack_ex)(rfn, std::ptr::null_mut(), &mut rerr, 0, fmt.as_ptr());
        diff_eq!(cret, rret, "vunpack_ex NULL root return");
        diff_eq!(cerr.raw(), rerr.raw(), "vunpack_ex NULL root error");

        // Enough varargs to spill past the six GP registers into the overflow area.
        let text20: String = format!("[{}]", (0..20).map(|i| i.to_string())
                                                    .collect::<Vec<_>>().join(","));
        let fmt20 = cs(&format!("[{}]", vec!["i"; 20].join(",")));
        let croot = load(c, &text20);
        let rroot = load(r, &text20);
        let mut cs_ = Slots::poisoned();
        let mut rs_ = Slots::poisoned();
        let cp: *mut Slots = &mut cs_;
        let rp: *mut Slots = &mut rs_;
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cret = (sh.vunpack_ex)(cfn, croot, &mut cerr, 0, fmt20.as_ptr(),
            ip(cp, 0), ip(cp, 1), ip(cp, 2), ip(cp, 3), ip(cp, 4), ip(cp, 5), ip(cp, 6),
            ip(cp, 7), ip(cp, 8), ip(cp, 9), ip(cp, 10), ip(cp, 11), ip(cp, 12),
            ip(cp, 13), ip(cp, 14), ip(cp, 15), ip(cp, 16), ip(cp, 17), ip(cp, 18),
            ip(cp, 19));
        let rret = (sh.vunpack_ex)(rfn, rroot, &mut rerr, 0, fmt20.as_ptr(),
            ip(rp, 0), ip(rp, 1), ip(rp, 2), ip(rp, 3), ip(rp, 4), ip(rp, 5), ip(rp, 6),
            ip(rp, 7), ip(rp, 8), ip(rp, 9), ip(rp, 10), ip(rp, 11), ip(rp, 12),
            ip(rp, 13), ip(rp, 14), ip(rp, 15), ip(rp, 16), ip(rp, 17), ip(rp, 18),
            ip(rp, 19));
        diff_eq!(cret, rret, "vunpack_ex overflow-area return");
        diff_eq!(cs_.summary(c), rs_.summary(r), "vunpack_ex overflow-area targets");
        assert_eq!(cret, 0, "C: overflow-area vunpack must succeed");
        decref(c, croot);
        decref(r, rroot);
    }
}

// ===========================================================================
// Row 283 — pack / unpack round trip
// ===========================================================================

#[test]
fn r283_pack_unpack_round_trip() {
    let (c, r) = both();
    unsafe {
        let ks: Vec<CString> = ["s", "i", "f", "b", "n", "a", "o"]
            .iter()
            .map(|k| cs(k))
            .collect();
        let inner = cs("in");
        let sv = cs("hello");
        let pfmt = cs("{s:s,s:i,s:f,s:b,s:n,s:[i,i],s:{s:s}}");
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, pfmt.as_ptr(),
            ks[0].as_ptr(), sv.as_ptr(), ks[1].as_ptr(), 42 as c_int,
            ks[2].as_ptr(), 2.5f64, ks[3].as_ptr(), 1 as c_int, ks[4].as_ptr(),
            ks[5].as_ptr(), 7 as c_int, 8 as c_int,
            ks[6].as_ptr(), inner.as_ptr(), sv.as_ptr());
        let rj = (r.json_pack_ex)(&mut rerr, 0, pfmt.as_ptr(),
            ks[0].as_ptr(), sv.as_ptr(), ks[1].as_ptr(), 42 as c_int,
            ks[2].as_ptr(), 2.5f64, ks[3].as_ptr(), 1 as c_int, ks[4].as_ptr(),
            ks[5].as_ptr(), 7 as c_int, 8 as c_int,
            ks[6].as_ptr(), inner.as_ptr(), sv.as_ptr());
        diff_eq!(cerr.raw(), rerr.raw(), "round trip pack error");
        let cdump = canon(c, cj);
        diff_eq!(cdump.clone(), canon(r, rj), "round trip packed tree");
        assert!(!cj.is_null(), "C: round trip pack must succeed");

        // An independently built tree must compare equal.
        let ctext = String::from_utf8(cdump.unwrap()).unwrap();
        let cref = load(c, &ctext);
        let rref = load(r, &ctext);
        diff_eq!((c.json_equal)(cj, cref), (r.json_equal)(rj, rref),
                 "round trip json_equal against a parsed tree");
        assert_eq!((c.json_equal)(cj, cref), 1, "C: packed tree must equal the parsed one");
        // ... and so must a deep copy.
        let ccopy = (c.json_deep_copy)(cj);
        let rcopy = (r.json_deep_copy)(rj);
        diff_eq!((c.json_equal)(cj, ccopy), (r.json_equal)(rj, rcopy),
                 "round trip json_equal against a deep copy");
        diff_eq!(canon(c, ccopy), canon(r, rcopy), "round trip deep copy dump");
        decref(c, ccopy);
        decref(r, rcopy);
        decref(c, cref);
        decref(r, rref);

        // Now unpack the packed tree with the mirrored format.
        let ufmt = cs("{s:s,s:i,s:f,s:b,s:n,s:[i,i],s:{s:s}}");
        for flags in [0, JSON_STRICT] {
            let mut cs_ = Slots::poisoned();
            let mut rs_ = Slots::poisoned();
            let cp: *mut Slots = &mut cs_;
            let rp: *mut Slots = &mut rs_;
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cret = (c.json_unpack_ex)(cj, &mut cerr, flags, ufmt.as_ptr(),
                ks[0].as_ptr(), sp(cp, 0), ks[1].as_ptr(), ip(cp, 0),
                ks[2].as_ptr(), dp(cp, 0), ks[3].as_ptr(), ip(cp, 1), ks[4].as_ptr(),
                ks[5].as_ptr(), ip(cp, 2), ip(cp, 3),
                ks[6].as_ptr(), inner.as_ptr(), sp(cp, 1));
            let rret = (r.json_unpack_ex)(rj, &mut rerr, flags, ufmt.as_ptr(),
                ks[0].as_ptr(), sp(rp, 0), ks[1].as_ptr(), ip(rp, 0),
                ks[2].as_ptr(), dp(rp, 0), ks[3].as_ptr(), ip(rp, 1), ks[4].as_ptr(),
                ks[5].as_ptr(), ip(rp, 2), ip(rp, 3),
                ks[6].as_ptr(), inner.as_ptr(), sp(rp, 1));
            diff_eq!(cret, rret, "round trip unpack return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "round trip unpack error flags={flags}");
            diff_eq!(cs_.summary(c), rs_.summary(r), "round trip targets flags={flags}");
            assert_eq!(cret, 0, "C: round trip unpack must succeed flags={flags}");
        }
        // JSON_VALIDATE_ONLY: keys only, so the walk stays aligned.
        for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cret = (c.json_unpack_ex)(cj, &mut cerr, flags, ufmt.as_ptr(),
                ks[0].as_ptr(), ks[1].as_ptr(), ks[2].as_ptr(), ks[3].as_ptr(),
                ks[4].as_ptr(), ks[5].as_ptr(), ks[6].as_ptr(), inner.as_ptr());
            let rret = (r.json_unpack_ex)(rj, &mut rerr, flags, ufmt.as_ptr(),
                ks[0].as_ptr(), ks[1].as_ptr(), ks[2].as_ptr(), ks[3].as_ptr(),
                ks[4].as_ptr(), ks[5].as_ptr(), ks[6].as_ptr(), inner.as_ptr());
            diff_eq!(cret, rret, "round trip VALIDATE_ONLY return flags={flags}");
            diff_eq!(cerr.raw(), rerr.raw(), "round trip VALIDATE_ONLY error flags={flags}");
            assert_eq!(cret, 0, "C: VALIDATE_ONLY round trip must validate");
        }

        // Refcount bookkeeping for `O` varargs: extracting with `O` must leave the
        // subtree alive after the root is dropped.
        let mut ctgt: *mut json_t = sentinel_json();
        let mut rtgt: *mut json_t = sentinel_json();
        let ofmt = cs("{s:O}");
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        diff_eq!(
            (c.json_unpack_ex)(cj, &mut cerr, 0, ofmt.as_ptr(), ks[5].as_ptr(), &mut ctgt),
            (r.json_unpack_ex)(rj, &mut rerr, 0, ofmt.as_ptr(), ks[5].as_ptr(), &mut rtgt),
            "extract with O"
        );
        diff_eq!((*ctgt).refcount, (*rtgt).refcount, "extracted refcount");
        assert_eq!((*ctgt).refcount, 2, "C: O gives the caller its own reference");
        decref(c, cj);
        decref(r, rj);
        // The extracted array must still be usable after the root is gone.
        diff_eq!((*ctgt).refcount, (*rtgt).refcount, "extracted refcount after root drop");
        diff_eq!(canon(c, ctgt), canon(r, rtgt), "extracted subtree after root drop");
        decref(c, ctgt);
        decref(r, rtgt);
    }
}

// ===========================================================================
// json_sprintf / json_vsprintf (rows 225-226) — the same variadic machinery
// ===========================================================================

/// Compare two `json_t*` that are expected to be strings (or NULL).
unsafe fn cmp_string_json(c: &Api, r: &Api, cj: *mut json_t, rj: *mut json_t, ctx: &str) {
    diff_eq!(cj.is_null(), rj.is_null(), "NULL-ness [{ctx}]");
    if !cj.is_null() {
        diff_eq!(typeof_(cj), typeof_(rj), "type [{ctx}]");
        diff_eq!(
            (c.json_string_length)(cj),
            (r.json_string_length)(rj),
            "json_string_length [{ctx}]"
        );
        diff_eq!(
            cbytes((c.json_string_value)(cj)),
            cbytes((r.json_string_value)(rj)),
            "string bytes [{ctx}]"
        );
    }
    diff_eq!(canon(c, cj), canon(r, rj), "canonical dump [{ctx}]");
}

#[test]
fn sprintf_every_printf_conversion() {
    let (c, r) = both();
    unsafe {
        let s = cs("abc");
        // No conversions at all, including the length == 0 early-out.
        for f in ["", "plain", "%%", "a%%b", "100%%"] {
            let fmt = cs(f);
            let cj = (c.json_sprintf)(fmt.as_ptr());
            let rj = (r.json_sprintf)(fmt.as_ptr());
            cmp_string_json(c, r, cj, rj, &format!("json_sprintf({f:?})"));
            decref(c, cj);
            decref(r, rj);
        }
        // %s
        for text in ["", "a", "hello world", "\u{e9}\u{20ac}\u{1f600}"] {
            let arg = cs(text);
            for f in ["%s", "[%s]", "%s%s", "%-10s|", "%10s|", "%.2s", "%-3.1s|"] {
                let fmt = cs(f);
                let cj = (c.json_sprintf)(fmt.as_ptr(), arg.as_ptr(), arg.as_ptr());
                let rj = (r.json_sprintf)(fmt.as_ptr(), arg.as_ptr(), arg.as_ptr());
                cmp_string_json(c, r, cj, rj, &format!("json_sprintf({f:?}, {text:?})"));
                decref(c, cj);
                decref(r, rj);
            }
        }
        // %d / %i / %x / %o / %u
        for v in [0i32, 1, -1, i32::MAX, i32::MIN, 12345, -9999] {
            for f in ["%d", "%i", "%x", "%X", "%o", "%u", "%5d|", "%-5d|", "%05d",
                      "%+d", "% d", "%#x", "%d/%i"] {
                let fmt = cs(f);
                let cj = (c.json_sprintf)(fmt.as_ptr(), v as c_int, v as c_int);
                let rj = (r.json_sprintf)(fmt.as_ptr(), v as c_int, v as c_int);
                cmp_string_json(c, r, cj, rj, &format!("json_sprintf({f:?}, {v})"));
                decref(c, cj);
                decref(r, rj);
            }
        }
        // %f / %g / %e and width/precision variants
        for v in [0.0f64, -0.0, 1.5, -1.5, 1e10, 1e-10, 1e308, 1.0 / 3.0,
                  std::f64::consts::PI] {
            for f in ["%f", "%g", "%e", "%E", "%G", "%5.2f", "%-12.4g|", "%+.3e",
                      "%.0f", "%.17g", "%f/%g/%e"] {
                let fmt = cs(f);
                let cj = (c.json_sprintf)(fmt.as_ptr(), v, v, v);
                let rj = (r.json_sprintf)(fmt.as_ptr(), v, v, v);
                cmp_string_json(c, r, cj, rj, &format!("json_sprintf({f:?}, {v:e})"));
                decref(c, cj);
                decref(r, rj);
            }
        }
        // %c and mixed conversions
        let fmt = cs("%c%s%d%%%f");
        let cj = (c.json_sprintf)(fmt.as_ptr(), 'Z' as c_int, s.as_ptr(), 7 as c_int, 1.5f64);
        let rj = (r.json_sprintf)(fmt.as_ptr(), 'Z' as c_int, s.as_ptr(), 7 as c_int, 1.5f64);
        cmp_string_json(c, r, cj, rj, "json_sprintf mixed");
        decref(c, cj);
        decref(r, rj);

        // A very long result: >1KiB forces the jsonp_malloc(length+1) path.
        let long = cs(&"L".repeat(5000));
        let fmt = cs("%s%s");
        let cj = (c.json_sprintf)(fmt.as_ptr(), long.as_ptr(), long.as_ptr());
        let rj = (r.json_sprintf)(fmt.as_ptr(), long.as_ptr(), long.as_ptr());
        cmp_string_json(c, r, cj, rj, "json_sprintf 10 KiB");
        assert_eq!((c.json_string_length)(cj), 10000, "C: 10000-byte result");
        decref(c, cj);
        decref(r, rj);
        let fmt = cs("%.4000f");
        let cj = (c.json_sprintf)(fmt.as_ptr(), 1.0 / 3.0);
        let rj = (r.json_sprintf)(fmt.as_ptr(), 1.0 / 3.0);
        cmp_string_json(c, r, cj, rj, "json_sprintf %.4000f");
        decref(c, cj);
        decref(r, rj);
        let fmt = cs("%9000d");
        let cj = (c.json_sprintf)(fmt.as_ptr(), 5 as c_int);
        let rj = (r.json_sprintf)(fmt.as_ptr(), 5 as c_int);
        cmp_string_json(c, r, cj, rj, "json_sprintf %9000d");
        decref(c, cj);
        decref(r, rj);

        // Invalid UTF-8 output -> the buffer is freed and NULL is returned.
        for bad in [&b"\xff\xfe"[..], b"\x80", b"a\xc3", b"\xed\xa0\x80"] {
            let arg = cs_bytes(bad);
            let fmt = cs("%s");
            let cj = (c.json_sprintf)(fmt.as_ptr(), arg.as_ptr());
            let rj = (r.json_sprintf)(fmt.as_ptr(), arg.as_ptr());
            cmp_string_json(c, r, cj, rj, &format!("json_sprintf invalid utf8 {bad:?}"));
            assert!(cj.is_null(), "C: invalid UTF-8 output must give NULL");
            decref(c, cj);
            decref(r, rj);
        }
        // Valid multi-byte UTF-8 -> json_string_length counts BYTES.
        let arg = cs("\u{1f600}\u{20ac}\u{e9}");
        let fmt = cs("%s");
        let cj = (c.json_sprintf)(fmt.as_ptr(), arg.as_ptr());
        let rj = (r.json_sprintf)(fmt.as_ptr(), arg.as_ptr());
        cmp_string_json(c, r, cj, rj, "json_sprintf multi-byte UTF-8");
        assert_eq!((c.json_string_length)(cj), 4 + 3 + 2, "C: byte length");
        decref(c, cj);
        decref(r, rj);
    }
}

#[test]
fn vsprintf_through_a_real_va_list() {
    let (c, r) = both();
    unsafe {
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vsprintf");
        let rfn = sym_addr("Rust", b"json_vsprintf");
        let s = cs("value");
        // Both vsnprintf passes (sizing on `ap`, filling on the va_copy'd `aq`)
        // with 5+ varargs of mixed classes.
        let fmt = cs("%s %d %f %% %s %i %g");
        let cj = (sh.vsprintf)(cfn, fmt.as_ptr(), s.as_ptr(), 42 as c_int, 1.5f64,
                               s.as_ptr(), -7 as c_int, 2.25f64);
        let rj = (sh.vsprintf)(rfn, fmt.as_ptr(), s.as_ptr(), 42 as c_int, 1.5f64,
                               s.as_ptr(), -7 as c_int, 2.25f64);
        cmp_string_json(c, r, cj, rj, "json_vsprintf mixed conversions");
        assert!(!cj.is_null(), "C: vsprintf must succeed");
        decref(c, cj);
        decref(r, rj);

        // The length == 0 early-out through the va_list entry point.
        let fmt = cs("");
        let cj = (sh.vsprintf)(cfn, fmt.as_ptr());
        let rj = (sh.vsprintf)(rfn, fmt.as_ptr());
        cmp_string_json(c, r, cj, rj, "json_vsprintf empty format");
        decref(c, cj);
        decref(r, rj);

        // The invalid-UTF-8 `goto out` path (which still has to va_end(aq)).
        let bad = cs_bytes(b"\xff\xfe");
        let fmt = cs("x%sy");
        let cj = (sh.vsprintf)(cfn, fmt.as_ptr(), bad.as_ptr());
        let rj = (sh.vsprintf)(rfn, fmt.as_ptr(), bad.as_ptr());
        cmp_string_json(c, r, cj, rj, "json_vsprintf invalid UTF-8");
        assert!(cj.is_null(), "C: invalid UTF-8 must give NULL");
        decref(c, cj);
        decref(r, rj);

        // A long result and many varargs spilling into the overflow area.
        let long = cs(&"Z".repeat(3000));
        let fmt = cs("%s%s%d%d%d%d%d%d%d%d%f%f%f%f%f%f%f%f%f%f");
        let cj = (sh.vsprintf)(cfn, fmt.as_ptr(), long.as_ptr(), long.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64, 6.0f64, 7.0f64, 8.0f64, 9.0f64,
            10.0f64);
        let rj = (sh.vsprintf)(rfn, fmt.as_ptr(), long.as_ptr(), long.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64, 6.0f64, 7.0f64, 8.0f64, 9.0f64,
            10.0f64);
        cmp_string_json(c, r, cj, rj, "json_vsprintf overflow area");
        decref(c, cj);
        decref(r, rj);
    }
}

#[test]
fn sprintf_randomised() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_5F01);
    unsafe {
        for i in 0..60000 {
            // A format built from a fixed argument signature (string, int, double)
            // so the varargs always match, but with randomised flags, widths and
            // precisions plus randomised argument values.
            let conv_s = *rng.choice(&["%s", "%.3s", "%-8s", "%12s"]);
            let conv_d = *rng.choice(&["%d", "%i", "%x", "%05d", "%+d", "%-6d"]);
            let conv_f = *rng.choice(&["%f", "%g", "%e", "%.2f", "%12.5g", "%-14.3e"]);
            let fmt_text = format!("{conv_s}|{conv_d}|{conv_f}|%%");
            let fmt = cs(&fmt_text);
            let sarg_text = rng.utf8_string(12);
            if sarg_text.as_bytes().contains(&0) {
                continue;
            }
            let sarg = cs(&sarg_text);
            let darg = rng.next_u32() as c_int;
            let farg = rng.real();
            let cj = (c.json_sprintf)(fmt.as_ptr(), sarg.as_ptr(), darg, farg);
            let rj = (r.json_sprintf)(fmt.as_ptr(), sarg.as_ptr(), darg, farg);
            cmp_string_json(c, r, cj, rj,
                            &format!("iter {i}: {fmt_text:?} {sarg_text:?} {darg} {farg:e}"));
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Randomised format/argument pairs
//
// Rust cannot build a variadic call whose ARITY is decided at run time, which is
// what a randomised format needs. It can, however, build a `va_list` by hand:
// on x86-64 SysV a `va_list` is the four-field `__va_list_tag`, and setting
// `gp_offset = 48` / `fp_offset = 176` marks both register save areas exhausted
// so that EVERY `va_arg` reads from `overflow_arg_area` — a plain array of
// 8-byte slots, exactly the layout the ABI gives stack-passed INTEGER/SSE
// arguments of size <= 8. That array is built from a plan generated together
// with the format, so the varargs always match, and both libraries are handed
// byte-identical argument blocks.
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy)]
struct VaTag {
    gp_offset: c_uint,
    fp_offset: c_uint,
    overflow_arg_area: *mut c_void,
    reg_save_area: *mut c_void,
}

fn va_tag(words: &mut [u64]) -> VaTag {
    VaTag {
        gp_offset: 48,   // > 48-8 => integer args come from the overflow area
        fp_offset: 176,  // > 176-16 => SSE args come from the overflow area
        overflow_arg_area: words.as_mut_ptr() as *mut c_void,
        reg_save_area: std::ptr::null_mut(),
    }
}

/// One vararg in a generated plan.
#[derive(Clone, Debug)]
enum A {
    Int(c_int),
    I64(json_int_t),
    F64(f64),
    Sz(size_t),
    /// A `const char *` pointing at `keep[i]`.
    Str(usize),
    /// An object KEY: consumed even under `JSON_VALIDATE_ONLY`.
    Key(usize),
    NullPtr,
    ISlot(usize),
    I64Slot(usize),
    DSlot(usize),
    LSlot(usize),
    SSlot(usize),
    OSlot(usize),
}

/// Lay a plan out as 8-byte overflow-area slots. When `skip_values` is set only
/// the `Key` arguments are emitted, which is the calling convention
/// `JSON_VALIDATE_ONLY` actually requires.
unsafe fn materialize(
    plan: &[A],
    keep: &[CString],
    sl: *mut Slots,
    skip_values: bool,
) -> Vec<u64> {
    let mut w: Vec<u64> = Vec::with_capacity(plan.len() + 8);
    for a in plan {
        if skip_values && !matches!(a, A::Key(_)) {
            continue;
        }
        let v: u64 = match a {
            A::Int(x) => *x as u32 as u64,
            A::I64(x) => *x as u64,
            A::F64(x) => x.to_bits(),
            A::Sz(x) => *x as u64,
            A::Str(i) | A::Key(i) => keep[*i].as_ptr() as usize as u64,
            A::NullPtr => 0,
            A::ISlot(i) => ip(sl, *i) as usize as u64,
            A::I64Slot(i) => i64p(sl, *i) as usize as u64,
            A::DSlot(i) => dp(sl, *i) as usize as u64,
            A::LSlot(i) => lp(sl, *i) as usize as u64,
            A::SSlot(i) => sp(sl, *i) as usize as u64,
            A::OSlot(i) => op(sl, *i) as usize as u64,
        };
        w.push(v);
    }
    // Padding, so that even a hypothetical over-read stays inside the buffer.
    w.extend_from_slice(&[0u64; 8]);
    w
}

/// Random strings that need no JSON escaping and contain no interior NUL.
fn safe_text(rng: &mut Rng, maxlen: usize) -> String {
    const POOL: &[char] = &[
        'a', 'b', 'c', 'X', 'Y', '0', '9', ' ', '-', '_', '.', '!', '\u{e9}', '\u{20ac}',
        '\u{1f600}',
    ];
    let n = rng.below(maxlen + 1);
    (0..n).map(|_| *rng.choice(POOL)).collect()
}

fn push_keep(keep: &mut Vec<CString>, s: &str) -> usize {
    keep.push(cs(s));
    keep.len() - 1
}

/// A little whitespace/separator noise, all of which `next_token` must skip.
fn noise(rng: &mut Rng) -> &'static str {
    match rng.below(8) {
        0 => " ",
        1 => "\t",
        2 => ",",
        3 => ":",
        4 => "\n",
        5 => " , ",
        6 => "\n\t",
        _ => "",
    }
}

// --- pack -------------------------------------------------------------------

fn gen_pack(
    rng: &mut Rng,
    depth: usize,
    fmt: &mut String,
    plan: &mut Vec<A>,
    keep: &mut Vec<CString>,
) {
    let choice = if depth >= 3 { rng.below(12) } else { rng.below(16) };
    match choice {
        0 => fmt.push('n'),
        1 => {
            fmt.push('b');
            plan.push(A::Int(rng.next_u32() as c_int));
        }
        2 => {
            fmt.push('i');
            plan.push(A::Int(rng.next_u32() as c_int));
        }
        3 => {
            fmt.push('I');
            plan.push(A::I64(rng.json_int()));
        }
        4 => {
            fmt.push('f');
            // Occasionally non-finite, which pack_real rejects.
            let v = if rng.below(20) == 0 {
                *rng.choice(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY])
            } else {
                rng.real()
            };
            plan.push(A::F64(v));
        }
        5 => {
            let t = safe_text(rng, 10);
            let i = push_keep(keep, &t);
            fmt.push('s');
            plan.push(A::Str(i));
        }
        6 => {
            let t = safe_text(rng, 10);
            let n = t.len();
            let i = push_keep(keep, &t);
            fmt.push_str("s#");
            plan.push(A::Str(i));
            plan.push(A::Int(rng.below(n + 1) as c_int));
        }
        7 => {
            let t = safe_text(rng, 10);
            let n = t.len();
            let i = push_keep(keep, &t);
            fmt.push_str("s%");
            plan.push(A::Str(i));
            plan.push(A::Sz(rng.below(n + 1)));
        }
        8 => {
            // s+ with 2..4 parts, each optionally length-qualified.
            let parts = 2 + rng.below(3);
            fmt.push('s');
            for p in 0..parts {
                if p > 0 {
                    fmt.push('+');
                }
                let t = safe_text(rng, 8);
                let n = t.len();
                let i = push_keep(keep, &t);
                if rng.below(10) == 0 {
                    plan.push(A::NullPtr);
                } else {
                    plan.push(A::Str(i));
                }
                match rng.below(3) {
                    0 => {
                        fmt.push('#');
                        plan.push(A::Int(rng.below(n + 1) as c_int));
                    }
                    1 => {
                        fmt.push('%');
                        plan.push(A::Sz(rng.below(n + 1)));
                    }
                    _ => {}
                }
            }
        }
        9 => {
            let t = safe_text(rng, 8);
            let i = push_keep(keep, &t);
            fmt.push('s');
            fmt.push(if rng.bool() { '?' } else { '*' });
            if rng.bool() {
                plan.push(A::Str(i));
            } else {
                plan.push(A::NullPtr);
            }
        }
        10 => {
            // o / O with a NULL value (library-independent) and every modifier.
            fmt.push(if rng.bool() { 'o' } else { 'O' });
            match rng.below(3) {
                0 => fmt.push('?'),
                1 => fmt.push('*'),
                _ => {}
            }
            plan.push(A::NullPtr);
        }
        11 => {
            // An outright bad format character.
            fmt.push(*rng.choice(&['q', 'x', 'z', 'd', 'S']));
        }
        12 | 13 => {
            fmt.push('[');
            let n = rng.below(4);
            for _ in 0..n {
                fmt.push_str(noise(rng));
                gen_pack(rng, depth + 1, fmt, plan, keep);
            }
            fmt.push_str(noise(rng));
            fmt.push(']');
        }
        _ => {
            fmt.push('{');
            let n = rng.below(4);
            for _ in 0..n {
                fmt.push_str(noise(rng));
                let k = safe_text(rng, 8);
                let klen = k.len();
                let ki = push_keep(keep, &k);
                fmt.push('s');
                match rng.below(6) {
                    0 => {
                        fmt.push('#');
                        plan.push(A::Str(ki));
                        plan.push(A::Int(rng.below(klen + 1) as c_int));
                    }
                    1 => {
                        fmt.push('%');
                        plan.push(A::Str(ki));
                        plan.push(A::Sz(rng.below(klen + 1)));
                    }
                    2 => {
                        fmt.push('+');
                        plan.push(A::Str(ki));
                        let k2 = safe_text(rng, 6);
                        let ki2 = push_keep(keep, &k2);
                        plan.push(A::Str(ki2));
                    }
                    _ => plan.push(A::Str(ki)),
                }
                fmt.push_str(noise(rng));
                gen_pack(rng, depth + 1, fmt, plan, keep);
            }
            fmt.push_str(noise(rng));
            fmt.push('}');
        }
    }
}

#[test]
fn randomised_pack_formats_with_matching_varargs() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0A01);
    unsafe {
        for iter in 0..250000 {
            let mut fmt = String::new();
            let mut plan: Vec<A> = Vec::new();
            let mut keep: Vec<CString> = Vec::new();
            gen_pack(&mut rng, 0, &mut fmt, &mut plan, &mut keep);
            if fmt.is_empty() {
                continue;
            }
            let flags = *rng.choice(&[
                0,
                JSON_VALIDATE_ONLY,
                JSON_STRICT,
                JSON_VALIDATE_ONLY | JSON_STRICT,
            ]);
            let fmt_c = cs(&fmt);
            // Pack consumes the same varargs regardless of flags, so one plan
            // serves both libraries.
            let mut cw = materialize(&plan, &keep, std::ptr::null_mut(), false);
            let mut rw = cw.clone();
            let mut ctag = va_tag(&mut cw);
            let mut rtag = va_tag(&mut rw);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cj = (c.json_vpack_ex)(&mut cerr, flags, fmt_c.as_ptr(),
                                       &mut ctag as *mut VaTag as *mut c_void);
            let rj = (r.json_vpack_ex)(&mut rerr, flags, fmt_c.as_ptr(),
                                       &mut rtag as *mut VaTag as *mut c_void);
            let ctx = format!("iter {iter}: fmt={fmt:?} flags={flags:#x} plan={plan:?}");
            diff_eq!(cj.is_null(), rj.is_null(), "vpack NULL-ness [{ctx}]");
            diff_eq!(cerr.raw(), rerr.raw(), "vpack error image [{ctx}]");
            diff_eq!(canon(c, cj), canon(r, rj), "vpack tree [{ctx}]");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// --- unpack -----------------------------------------------------------------

/// A generated JSON value: rendered to text for `json_loads` AND used to build a
/// matching unpack format.
#[derive(Clone, Debug)]
enum V {
    Null,
    Bool(bool),
    Int(json_int_t),
    /// A literal that both `json_loads` and a human can read back exactly.
    Real(&'static str),
    Str(String),
    Arr(Vec<V>),
    Obj(Vec<(String, V)>),
}

const REALS: [&str; 8] = [
    "1.5", "-0.0", "0.0", "1e308", "5e-324", "3.141592653589793", "-2.25", "1e-308",
];

impl V {
    fn render(&self) -> String {
        match self {
            V::Null => "null".into(),
            V::Bool(b) => if *b { "true".into() } else { "false".into() },
            V::Int(i) => i.to_string(),
            V::Real(s) => (*s).into(),
            V::Str(s) => format!("\"{s}\""),
            V::Arr(v) => format!(
                "[{}]",
                v.iter().map(|x| x.render()).collect::<Vec<_>>().join(",")
            ),
            V::Obj(kv) => format!(
                "{{{}}}",
                kv.iter()
                    .map(|(k, v)| format!("\"{k}\":{}", v.render()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

fn gen_value(rng: &mut Rng, depth: usize, budget: &mut usize) -> V {
    if *budget == 0 {
        return V::Null;
    }
    *budget -= 1;
    let n = if depth >= 3 { rng.below(5) } else { rng.below(7) };
    match n {
        0 => V::Null,
        1 => V::Bool(rng.bool()),
        2 => V::Int(rng.json_int()),
        3 => V::Real(*rng.choice(&REALS)),
        4 => V::Str(safe_text(rng, 10)),
        5 => {
            let k = rng.below(4);
            V::Arr((0..k).map(|_| gen_value(rng, depth + 1, budget)).collect())
        }
        _ => {
            let k = rng.below(4);
            let mut kv = Vec::new();
            for i in 0..k {
                // Distinct keys, so json_object_size matches the pair count.
                kv.push((format!("{}{i}", safe_text(rng, 4)), gen_value(rng, depth + 1, budget)));
            }
            V::Obj(kv)
        }
    }
}

/// Slot allocator: `O` needs a slot of its own (each write takes a reference we
/// have to give back), everything else may reuse.
struct Ctr {
    i: usize,
    i64_: usize,
    d: usize,
    l: usize,
    s: usize,
    o: usize,
}

impl Ctr {
    fn new() -> Ctr {
        Ctr { i: 0, i64_: 0, d: 0, l: 0, s: 0, o: 0 }
    }
    fn ni(&mut self) -> usize { let v = self.i % NSLOT; self.i += 1; v }
    fn nI(&mut self) -> usize { let v = self.i64_ % NSLOT; self.i64_ += 1; v }
    fn nd(&mut self) -> usize { let v = self.d % NSLOT; self.d += 1; v }
    fn nl(&mut self) -> usize { let v = self.l % NSLOT; self.l += 1; v }
    fn ns(&mut self) -> usize { let v = self.s % NSLOT; self.s += 1; v }
    /// `None` once every `O` slot is taken.
    fn no(&mut self) -> Option<usize> {
        if self.o >= NSLOT { None } else { let v = self.o; self.o += 1; Some(v) }
    }
    fn no_reuse(&mut self) -> usize { NSLOT - 1 }
}

#[allow(clippy::too_many_arguments)]
fn gen_unpack_fmt(
    rng: &mut Rng,
    v: &V,
    fmt: &mut String,
    plan: &mut Vec<A>,
    keep: &mut Vec<CString>,
    oslots: &mut Vec<usize>,
    ctr: &mut Ctr,
    depth: usize,
) {
    // 1 in 12: use o/O instead of the type-specific format character.
    if rng.below(12) == 0 {
        if rng.bool() {
            if let Some(i) = ctr.no() {
                fmt.push('O');
                plan.push(A::OSlot(i));
                oslots.push(i);
                return;
            }
        }
        fmt.push('o');
        plan.push(A::OSlot(ctr.no_reuse()));
        return;
    }
    // 1 in 16: deliberately use the WRONG format character, so the validation
    // error paths are exercised too.
    if rng.below(16) == 0 {
        match rng.below(6) {
            0 => { fmt.push('i'); plan.push(A::ISlot(ctr.ni())); }
            1 => { fmt.push('s'); plan.push(A::SSlot(ctr.ns())); }
            2 => { fmt.push('f'); plan.push(A::DSlot(ctr.nd())); }
            3 => { fmt.push('b'); plan.push(A::ISlot(ctr.ni())); }
            4 => fmt.push('n'),
            _ => { fmt.push('I'); plan.push(A::I64Slot(ctr.nI())); }
        }
        return;
    }
    match v {
        V::Null => fmt.push('n'),
        V::Bool(_) => {
            fmt.push('b');
            plan.push(A::ISlot(ctr.ni()));
        }
        V::Int(_) => match rng.below(3) {
            0 => { fmt.push('i'); plan.push(A::ISlot(ctr.ni())); }
            1 => { fmt.push('I'); plan.push(A::I64Slot(ctr.nI())); }
            _ => { fmt.push('F'); plan.push(A::DSlot(ctr.nd())); }
        },
        V::Real(_) => {
            fmt.push(if rng.bool() { 'f' } else { 'F' });
            plan.push(A::DSlot(ctr.nd()));
        }
        V::Str(_) => {
            fmt.push('s');
            plan.push(A::SSlot(ctr.ns()));
            if rng.bool() {
                fmt.push('%');
                plan.push(A::LSlot(ctr.nl()));
            }
        }
        V::Arr(items) => {
            fmt.push('[');
            // Sometimes stop short, sometimes go one too far.
            let take = if rng.below(6) == 0 {
                items.len() + 1
            } else if rng.below(6) == 0 && !items.is_empty() {
                items.len() - 1
            } else {
                items.len()
            };
            for k in 0..take {
                fmt.push_str(noise(rng));
                let child = items.get(k).cloned().unwrap_or(V::Null);
                gen_unpack_fmt(rng, &child, fmt, plan, keep, oslots, ctr, depth + 1);
            }
            if rng.below(4) == 0 {
                fmt.push_str(noise(rng));
                fmt.push(if rng.bool() { '!' } else { '*' });
            }
            fmt.push_str(noise(rng));
            fmt.push(']');
        }
        V::Obj(kv) => {
            fmt.push('{');
            let take = if rng.below(6) == 0 && !kv.is_empty() {
                kv.len() - 1 // leave a key unpacked, so JSON_STRICT can complain
            } else {
                kv.len()
            };
            for k in 0..take {
                fmt.push_str(noise(rng));
                let (key, child) = &kv[k];
                // Sometimes ask for a key that is not there.
                let absent = rng.below(10) == 0;
                let key_text = if absent { format!("{key}~absent") } else { key.clone() };
                let ki = push_keep(keep, &key_text);
                fmt.push('s');
                plan.push(A::Key(ki));
                if rng.below(3) == 0 {
                    fmt.push('?');
                }
                fmt.push_str(noise(rng));
                gen_unpack_fmt(rng, child, fmt, plan, keep, oslots, ctr, depth + 1);
            }
            if rng.below(4) == 0 {
                fmt.push_str(noise(rng));
                fmt.push(if rng.bool() { '!' } else { '*' });
            }
            fmt.push_str(noise(rng));
            fmt.push('}');
        }
    }
}

#[test]
fn randomised_unpack_formats_with_matching_out_pointers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0A02);
    unsafe {
        for iter in 0..250000 {
            let mut budget = 8usize;
            let v = gen_value(&mut rng, 0, &mut budget);
            let text = v.render();
            let mut fmt = String::new();
            let mut plan: Vec<A> = Vec::new();
            let mut keep: Vec<CString> = Vec::new();
            let mut oslots: Vec<usize> = Vec::new();
            let mut ctr = Ctr::new();
            gen_unpack_fmt(&mut rng, &v, &mut fmt, &mut plan, &mut keep, &mut oslots,
                           &mut ctr, 0);
            if fmt.is_empty() {
                continue;
            }
            let flags = *rng.choice(&[
                0,
                JSON_VALIDATE_ONLY,
                JSON_STRICT,
                JSON_VALIDATE_ONLY | JSON_STRICT,
            ]);
            let validate_only = flags & JSON_VALIDATE_ONLY != 0;

            let croot = load(c, &text);
            let rroot = load(r, &text);
            let fmt_c = cs(&fmt);
            let mut cslots = Slots::poisoned();
            let mut rslots = Slots::poisoned();
            let cp: *mut Slots = &mut cslots;
            let rp: *mut Slots = &mut rslots;
            let mut cw = materialize(&plan, &keep, cp, validate_only);
            let mut rw = materialize(&plan, &keep, rp, validate_only);
            let mut ctag = va_tag(&mut cw);
            let mut rtag = va_tag(&mut rw);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cret = (c.json_vunpack_ex)(croot, &mut cerr, flags, fmt_c.as_ptr(),
                                           &mut ctag as *mut VaTag as *mut c_void);
            let rret = (r.json_vunpack_ex)(rroot, &mut rerr, flags, fmt_c.as_ptr(),
                                           &mut rtag as *mut VaTag as *mut c_void);
            let ctx = format!(
                "iter {iter}: text={text} fmt={fmt:?} flags={flags:#x} plan={plan:?}"
            );
            diff_eq!(cret, rret, "vunpack return [{ctx}]");
            diff_eq!(cerr.raw(), rerr.raw(), "vunpack error image [{ctx}]");
            diff_eq!(
                cslots.summary(c),
                rslots.summary(r),
                "vunpack out-pointer targets [{ctx}]"
            );
            if !validate_only {
                cslots.decref_objs(c, &oslots);
                rslots.decref_objs(r, &oslots);
            }
            decref(c, croot);
            decref(r, rroot);
        }
    }
}

#[test]
fn randomised_pack_then_unpack_round_trips() {
    // Generate a value, pack an equivalent tree, dump it, re-parse it and unpack
    // it again — every step compared between the two libraries.
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0A03);
    unsafe {
        for iter in 0..80000 {
            let mut budget = 8usize;
            let v = gen_value(&mut rng, 0, &mut budget);
            let text = v.render();
            let croot = load(c, &text);
            let rroot = load(r, &text);
            diff_eq!(canon(c, croot), canon(r, rroot), "iter {iter}: parsed roots differ");

            // "O" the whole root back out, then re-dump it.
            let mut cslots = Slots::poisoned();
            let mut rslots = Slots::poisoned();
            let mut ctgt: *mut json_t = sentinel_json();
            let mut rtgt: *mut json_t = sentinel_json();
            let f = cs("O");
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            diff_eq!(
                (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), &mut ctgt),
                (r.json_unpack_ex)(rroot, &mut rerr, 0, f.as_ptr(), &mut rtgt),
                "iter {iter}: unpack O return"
            );
            diff_eq!(canon(c, ctgt), canon(r, rtgt), "iter {iter}: O round trip");
            decref(c, ctgt);
            decref(r, rtgt);
            let _ = (&mut cslots, &mut rslots);

            // Pack a one-key object holding the root with "o" (which steals the
            // reference we own) and compare the result.
            let k = cs("wrapped");
            let fmt = cs("{s:o}");
            let cj = (c.json_pack_ex)(&mut cerr, 0, fmt.as_ptr(), k.as_ptr(), croot);
            let rj = (r.json_pack_ex)(&mut rerr, 0, fmt.as_ptr(), k.as_ptr(), rroot);
            diff_eq!(cerr.raw(), rerr.raw(), "iter {iter}: wrap error image");
            diff_eq!(canon(c, cj), canon(r, rj), "iter {iter}: wrapped tree");
            // Unpack it straight back out.
            let ufmt = cs("{s:o}");
            let mut ctgt: *mut json_t = sentinel_json();
            let mut rtgt: *mut json_t = sentinel_json();
            diff_eq!(
                (c.json_unpack_ex)(cj, &mut cerr, 0, ufmt.as_ptr(), k.as_ptr(), &mut ctgt),
                (r.json_unpack_ex)(rj, &mut rerr, 0, ufmt.as_ptr(), k.as_ptr(), &mut rtgt),
                "iter {iter}: unwrap return"
            );
            diff_eq!(canon(c, ctgt), canon(r, rtgt), "iter {iter}: unwrapped subtree");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn hand_built_va_list_is_abi_correct() {
    // Sanity check for the generator above: a hand-built overflow-only va_list
    // must be read by the C exactly like a compiler-built one. If this ever
    // failed, every randomised comparison would be comparing garbage.
    let (c, r) = both();
    unsafe {
        let keep = vec![cs("k"), cs("txt"), cs("j")];
        let fmt = cs("{s:i,s:[s,I,f],s:b}");
        // Three keys, the third of which repeats "k" — so the boolean must
        // OVERWRITE the integer, which is itself a useful check of the duplicate
        // key path.
        let plan = vec![
            A::Str(0),        // key "k"
            A::Int(42),       // i
            A::Str(2),        // key "j"
            A::Str(1),        // s   (the array's first element)
            A::I64(i64::MIN), // I
            A::F64(2.5),      // f
            A::Str(0),        // key "k" again
            A::Int(1),        // b
        ];
        let mut cw = materialize(&plan, &keep, std::ptr::null_mut(), false);
        let mut rw = cw.clone();
        let mut ctag = va_tag(&mut cw);
        let mut rtag = va_tag(&mut rw);
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (c.json_vpack_ex)(&mut cerr, 0, fmt.as_ptr(),
                                   &mut ctag as *mut VaTag as *mut c_void);
        let rj = (r.json_vpack_ex)(&mut rerr, 0, fmt.as_ptr(),
                                   &mut rtag as *mut VaTag as *mut c_void);
        diff_eq!(cerr.raw(), rerr.raw(), "hand-built va_list error image");
        let cd = canon(c, cj);
        diff_eq!(cd.clone(), canon(r, rj), "hand-built va_list tree");
        assert_eq!(
            cd.as_deref(),
            Some(&b"{\"j\": [\"txt\", -9223372036854775808, 2.5], \"k\": true}"[..]),
            "C: the hand-built va_list must be read exactly like a compiler-built one"
        );
        decref(c, cj);
        decref(r, rj);

        // The same through the shim, so a compiler-built va_list is shown to give
        // an identical result for identical arguments.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vpack_ex");
        let mut cerr2 = json_error_t::poisoned();
        let cj2 = (sh.vpack_ex)(cfn, &mut cerr2, 0, fmt.as_ptr(),
            keep[0].as_ptr(), 42 as c_int, keep[2].as_ptr(), keep[1].as_ptr(),
            i64::MIN as json_int_t, 2.5f64, keep[0].as_ptr(), 1 as c_int);
        assert_eq!(canon(c, cj2), cd,
                   "C: hand-built and compiler-built va_lists must agree");
        assert_eq!(cerr2.raw(), cerr.raw(), "C: error images must agree too");
        decref(c, cj2);

        // And for unpack, where the args are out-pointers.
        let mut sl_ = Slots::poisoned();
        let s: *mut Slots = &mut sl_;
        let uplan = vec![A::Key(0), A::ISlot(0), A::Key(2), A::SSlot(0), A::LSlot(0)];
        let ufmt = cs("{s:i,s:s%}");
        let croot = load(c, "{\"k\":7,\"j\":\"abc\"}");
        let mut w = materialize(&uplan, &keep, s, false);
        let mut tag = va_tag(&mut w);
        let mut e = json_error_t::poisoned();
        assert_eq!(
            (c.json_vunpack_ex)(croot, &mut e, 0, ufmt.as_ptr(),
                                &mut tag as *mut VaTag as *mut c_void),
            0,
            "C: hand-built va_list unpack must succeed"
        );
        assert_eq!(sl_.ints[0], 7, "C: the int target must be written");
        assert_eq!(cbytes(sl_.strs[0]).as_deref(), Some(&b"abc"[..]),
                   "C: the string target must be written");
        assert_eq!(sl_.lens[0], 3, "C: the length target must be written");
        decref(c, croot);
    }
}

// ===========================================================================
// The variadic shims themselves (src/varargs.rs)
//
// json_pack / json_pack_ex / json_unpack / json_unpack_ex / json_sprintf /
// jsonp_error_set are hand-written naked-asm variadic prologues in the Rust
// port. Each has a different number of NAMED parameters, hence a different
// starting `gp_offset`, and each has to spill rdi..r9 plus xmm0..xmm7 and point
// `overflow_arg_area` at [rbp+16]. These tests drive every one of them with
// enough arguments to exhaust both register save areas and spill onto the stack.
// ===========================================================================

#[test]
fn variadic_shims_register_save_and_overflow_areas() {
    let (c, r) = both();
    unsafe {
        let s1 = cs("alpha");
        let s2 = cs("beta");
        // 8 ints (6 GP registers then the stack) + 10 doubles (8 xmm then the
        // stack) + 2 pointers, interleaved so both classes overflow.
        let fmt = cs("[i,i,i,i,i,i,i,i,f,f,f,f,f,f,f,f,f,f,s,s]");

        // ---- json_pack_ex (3 named args => gp_offset 24)
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, fmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64, 9.5f64,
            10.5f64, s1.as_ptr(), s2.as_ptr());
        let rj = (r.json_pack_ex)(&mut rerr, 0, fmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64, 9.5f64,
            10.5f64, s1.as_ptr(), s2.as_ptr());
        diff_eq!(cerr.raw(), rerr.raw(), "json_pack_ex wide error image");
        let want = canon(c, cj);
        diff_eq!(want.clone(), canon(r, rj), "json_pack_ex wide tree");
        assert_eq!(
            want.as_deref(),
            Some(&b"[1, 2, 3, 4, 5, 6, 7, 8, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 10.5, \"alpha\", \"beta\"]"[..]),
            "C: every argument class must survive the variadic prologue"
        );
        decref(c, cj);
        decref(r, rj);

        // ---- json_pack (1 named arg => gp_offset 8)
        let cj = (c.json_pack)(fmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64, 9.5f64,
            10.5f64, s1.as_ptr(), s2.as_ptr());
        let rj = (r.json_pack)(fmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64, 9.5f64,
            10.5f64, s1.as_ptr(), s2.as_ptr());
        diff_eq!(canon(c, cj), canon(r, rj), "json_pack wide tree");
        assert_eq!(canon(c, cj), want, "C: json_pack must match json_pack_ex");
        decref(c, cj);
        decref(r, rj);

        // ---- json_unpack (2 named args => gp_offset 16) and json_unpack_ex
        // (4 named args => gp_offset 32), with 20 out-pointers of three classes.
        let text = "[1,2,3,4,5,6,7,8,1.5,2.5,3.5,4.5,5.5,6.5,7.5,8.5,9.5,10.5,\
                    \"alpha\",\"beta\"]";
        upkn!(c, r, text, cs(&fmt.to_str().unwrap().to_string()), sl,
             [ip(sl, 0), ip(sl, 1), ip(sl, 2), ip(sl, 3), ip(sl, 4), ip(sl, 5), ip(sl, 6),
              ip(sl, 7), dp(sl, 0), dp(sl, 1), dp(sl, 2), dp(sl, 3), dp(sl, 4), dp(sl, 5),
              dp(sl, 6), dp(sl, 7), dp(sl, 8), dp(sl, 9), sp(sl, 0), sp(sl, 1)], &[],
             "json_unpack wide");
        upk!(c, r, text, 0, cs(&fmt.to_str().unwrap().to_string()), sl,
             [ip(sl, 0), ip(sl, 1), ip(sl, 2), ip(sl, 3), ip(sl, 4), ip(sl, 5), ip(sl, 6),
              ip(sl, 7), dp(sl, 0), dp(sl, 1), dp(sl, 2), dp(sl, 3), dp(sl, 4), dp(sl, 5),
              dp(sl, 6), dp(sl, 7), dp(sl, 8), dp(sl, 9), sp(sl, 0), sp(sl, 1)], &[],
             "json_unpack_ex wide");
        // Pin the values in the C so the test would notice a shifted vararg walk.
        let croot = load(c, text);
        let mut sl_ = Slots::poisoned();
        let s: *mut Slots = &mut sl_;
        let ufmt = cs("[i,i,i,i,i,i,i,i,f,f,f,f,f,f,f,f,f,f,s,s]");
        assert_eq!(
            (c.json_unpack)(croot, ufmt.as_ptr(),
                ip(s, 0), ip(s, 1), ip(s, 2), ip(s, 3), ip(s, 4), ip(s, 5), ip(s, 6),
                ip(s, 7), dp(s, 0), dp(s, 1), dp(s, 2), dp(s, 3), dp(s, 4), dp(s, 5),
                dp(s, 6), dp(s, 7), dp(s, 8), dp(s, 9), sp(s, 0), sp(s, 1)),
            0,
            "C: wide json_unpack must succeed"
        );
        assert_eq!(&sl_.ints[..8], &[1, 2, 3, 4, 5, 6, 7, 8], "C: int targets");
        assert_eq!(
            &sl_.dbls[..10],
            &[1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 10.5],
            "C: double targets"
        );
        assert_eq!(cbytes(sl_.strs[0]).as_deref(), Some(&b"alpha"[..]), "C: string 0");
        assert_eq!(cbytes(sl_.strs[1]).as_deref(), Some(&b"beta"[..]), "C: string 1");
        decref(c, croot);

        // ---- json_sprintf (1 named arg => gp_offset 8), many args of both classes
        let sfmt = cs("%d%d%d%d%d%d%d%d%f %f %f %f %f %f %f %f %f %f %s%s");
        let cj = (c.json_sprintf)(sfmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64, 9.5f64,
            10.5f64, s1.as_ptr(), s2.as_ptr());
        let rj = (r.json_sprintf)(sfmt.as_ptr(),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int,
            1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64, 9.5f64,
            10.5f64, s1.as_ptr(), s2.as_ptr());
        cmp_string_json(c, r, cj, rj, "json_sprintf wide");
        assert_eq!(
            cbytes((c.json_string_value)(cj)).as_deref(),
            Some(&b"123456781.500000 2.500000 3.500000 4.500000 5.500000 6.500000 7.500000 8.500000 9.500000 10.500000 alphabeta"[..]),
            "C: wide json_sprintf output"
        );
        decref(c, cj);
        decref(r, rj);

        // ---- jsonp_error_set (6 named args => gp_offset 48, so the whole GP
        // register save area is already spent and the va_list starts in the
        // overflow area; the shim also passes the va_list itself on the stack).
        let msg = cs("%s/%d/%f/%s/%d/%f/%x");
        // NOTE: jsonp_error_vset refuses to overwrite an already-set error
        // (`if (error->text[0] != '\0') return;`), so the struct must start
        // ZEROED here — a poisoned one would take the early return. Both cases
        // are compared below.
        for (line, col, pos, code) in [(1, 2, 3usize, 4), (-1, -1, 0, 0), (99, 88, 77, 17)] {
            let mut cerr = json_error_t::new();
            let mut rerr = json_error_t::new();
            (c.jsonp_error_set)(&mut cerr, line, col, pos, code, msg.as_ptr(),
                s1.as_ptr(), 42 as c_int, 1.5f64, s2.as_ptr(), -7 as c_int, 2.5f64,
                255 as c_uint);
            (r.jsonp_error_set)(&mut rerr, line, col, pos, code, msg.as_ptr(),
                s1.as_ptr(), 42 as c_int, 1.5f64, s2.as_ptr(), -7 as c_int, 2.5f64,
                255 as c_uint);
            diff_eq!(cerr.raw(), rerr.raw(),
                     "jsonp_error_set({line},{col},{pos},{code}) error image");
            assert_eq!(cerr.snapshot().4, "alpha/42/1.500000/beta/-7/2.500000/ff",
                       "C: jsonp_error_set text");
            // A second call must be a no-op ("error already set").
            (c.jsonp_error_set)(&mut cerr, 0, 0, 0, 0, cs("other").as_ptr());
            (r.jsonp_error_set)(&mut rerr, 0, 0, 0, 0, cs("other").as_ptr());
            diff_eq!(cerr.raw(), rerr.raw(), "jsonp_error_set second call is a no-op");
            assert_eq!(cerr.snapshot().4, "alpha/42/1.500000/beta/-7/2.500000/ff",
                       "C: the first error must survive");
            // A poisoned struct also takes the early return.
            let mut cp = json_error_t::poisoned();
            let mut rp = json_error_t::poisoned();
            (c.jsonp_error_set)(&mut cp, line, col, pos, code, msg.as_ptr(),
                s1.as_ptr(), 42 as c_int, 1.5f64, s2.as_ptr(), -7 as c_int, 2.5f64,
                255 as c_uint);
            (r.jsonp_error_set)(&mut rp, line, col, pos, code, msg.as_ptr(),
                s1.as_ptr(), 42 as c_int, 1.5f64, s2.as_ptr(), -7 as c_int, 2.5f64,
                255 as c_uint);
            diff_eq!(cp.raw(), rp.raw(), "jsonp_error_set on a poisoned struct");
            assert_eq!(cp.raw(), json_error_t::poisoned().raw(),
                       "C: an already-set error must not be touched");
        }
        // ... and through the va_list shim, which must give the same bytes.
        let sh = vashim();
        let cfn = sym_addr("C", b"jsonp_error_vset");
        let rfn = sym_addr("Rust", b"jsonp_error_vset");
        let mut cerr = json_error_t::new();
        let mut rerr = json_error_t::new();
        (sh.error_vset)(cfn, &mut cerr, 1, 2, 3, 4, msg.as_ptr(),
            s1.as_ptr(), 42 as c_int, 1.5f64, s2.as_ptr(), -7 as c_int, 2.5f64,
            255 as c_uint);
        (sh.error_vset)(rfn, &mut rerr, 1, 2, 3, 4, msg.as_ptr(),
            s1.as_ptr(), 42 as c_int, 1.5f64, s2.as_ptr(), -7 as c_int, 2.5f64,
            255 as c_uint);
        diff_eq!(cerr.raw(), rerr.raw(), "jsonp_error_vset error image");
    }
}

// ===========================================================================
// Edge inputs the C handles deterministically
// ===========================================================================

#[test]
fn pack_negative_explicit_lengths() {
    // `s#` reads an `int`, which is then widened to `size_t`. A negative value
    // therefore becomes an enormous length — and strbuffer_append_bytes rejects
    // it by its own overflow guard BEFORE any memcpy, so this is a well-defined
    // "Out of memory" path rather than an out-of-bounds read.
    let (c, r) = both();
    unsafe {
        let a = cs("abcdef");
        for len in [-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
            pk!(c, r, 0, cs("s#"), [a.as_ptr(), len as c_int], "pack s# len {len}");
            pk!(c, r, 0, cs("{s#:i}"), [a.as_ptr(), len as c_int, 1 as c_int],
                "pack {{s#:i}} len {len}");
            pk!(c, r, 0, cs("s#+#"),
                [a.as_ptr(), 2 as c_int, a.as_ptr(), len as c_int], "pack s#+# len {len}");
        }
        let f = cs("s#");
        let mut cerr = json_error_t::poisoned();
        let cj = (c.json_pack_ex)(&mut cerr, 0, f.as_ptr(), a.as_ptr(), -1 as c_int);
        assert!(cj.is_null(), "C: a negative s# length must fail");
        assert_eq!(cerr.snapshot().3, "<internal>", "C: source");
        assert_eq!(cerr.snapshot().4, "Out of memory", "C: text");
        // `s%` with SIZE_MAX behaves the same way.
        pk!(c, r, 0, cs("s%"), [a.as_ptr(), usize::MAX], "pack s% SIZE_MAX");
        pk!(c, r, 0, cs("s%"), [a.as_ptr(), usize::MAX - 1], "pack s% SIZE_MAX-1");
    }
}

#[test]
fn error_text_truncation_with_very_long_keys() {
    // Several messages embed a key with `%s`, and `json_error_t::text` is only
    // 160 bytes, so long keys exercise jsonp_error_vset's truncation (the last
    // byte of `text` also carries the error code).
    let (c, r) = both();
    unsafe {
        for n in [1usize, 100, 140, 150, 155, 156, 157, 158, 159, 160, 161, 200, 1000] {
            let key = "K".repeat(n);
            let kc = cs(&key);
            // "Object item not found: <key>"
            upk!(c, r, "{\"a\":1}", 0, cs("{s:i}"), sl, [kc.as_ptr(), ip(sl, 0)], &[],
                 "missing key of length {n}");
            // "%li object item(s) left unpacked: <keys>"
            let text = format!("{{\"{key}\":1,\"a\":2}}");
            upk!(c, r, &text, JSON_STRICT, cs("{s:i}"), sl,
                 [cs("a").as_ptr(), ip(sl, 1)], &[],
                 "STRICT unpacked key of length {n}");
            // Several long unrecognized keys at once.
            let text = format!("{{\"{key}1\":1,\"{key}2\":2,\"{key}3\":3,\"a\":4}}");
            upk!(c, r, &text, JSON_STRICT, cs("{s:i}"), sl,
                 [cs("a").as_ptr(), ip(sl, 2)], &[],
                 "STRICT three long keys of length {n}");
        }
    }
}

#[test]
fn pack_string_modifiers_in_every_position() {
    // `s`, `s#`, `s%` and `s+` as an object VALUE and as an array ELEMENT, not
    // just at top level or as a key.
    let (c, r) = both();
    unsafe {
        let a = cs("abcdef");
        let b = cs("012345");
        let k = cs("key");
        let nul: *const c_char = std::ptr::null();
        pk!(c, r, 0, cs("{s:s#}"), [k.as_ptr(), a.as_ptr(), 3 as c_int], "{{s:s#}}");
        pk!(c, r, 0, cs("{s:s%}"), [k.as_ptr(), a.as_ptr(), 3 as size_t], "{{s:s%}}");
        pk!(c, r, 0, cs("{s:s+}"), [k.as_ptr(), a.as_ptr(), b.as_ptr()], "{{s:s+}}");
        pk!(c, r, 0, cs("{s:s+#}"), [k.as_ptr(), a.as_ptr(), b.as_ptr(), 2 as c_int],
            "{{s:s+#}}");
        pk!(c, r, 0, cs("[s#]"), [a.as_ptr(), 4 as c_int], "[s#]");
        pk!(c, r, 0, cs("[s%]"), [a.as_ptr(), 4 as size_t], "[s%]");
        pk!(c, r, 0, cs("[s+]"), [a.as_ptr(), b.as_ptr()], "[s+]");
        pk!(c, r, 0, cs("[s#,s%,s+,s]"),
            [a.as_ptr(), 1 as c_int, a.as_ptr(), 2 as size_t, a.as_ptr(), b.as_ptr(),
             b.as_ptr()], "[s#,s%,s+,s]");
        pk!(c, r, 0, cs("{s#:s#,s%:s%,s+:s+}"),
            [k.as_ptr(), 3 as c_int, a.as_ptr(), 3 as c_int,
             k.as_ptr(), 2 as size_t, a.as_ptr(), 2 as size_t,
             k.as_ptr(), b.as_ptr(), a.as_ptr(), b.as_ptr()], "keys and values together");
        // A NULL pointer with an explicit length.
        pk!(c, r, 0, cs("s#"), [nul, 0 as c_int], "s# with a NULL pointer");
        pk!(c, r, 0, cs("s%"), [nul, 3 as size_t], "s% with a NULL pointer");
        pk!(c, r, 0, cs("{s#:i}"), [nul, 2 as c_int, 1 as c_int], "s# NULL key");
        pk!(c, r, 0, cs("[s%]"), [nul, 2 as size_t], "[s%] NULL");
    }
}

#[test]
fn unpack_object_keys_take_no_length_modifiers() {
    // In unpack, an object key is always a plain `va_arg(const char *)` with
    // strlen() — there is no `s#`/`s%`/`s+` for keys. The modifier therefore
    // falls through to `unpack`'s default arm as the VALUE's format character.
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        for f in ["{s#:i}", "{s%:i}", "{s+:i}", "{s#i}", "{s%i}"] {
            upk!(c, r, "{\"a\":1}", 0, cs(f), sl, [a.as_ptr(), ip(sl, 0), ip(sl, 1)], &[],
                 "unpack {f}");
        }
        let croot = load(c, "{\"a\":1}");
        let mut cerr = json_error_t::poisoned();
        let mut t: c_int = POISON_I32;
        let f = cs("{s%:i}");
        assert_eq!(
            (c.json_unpack_ex)(croot, &mut cerr, 0, f.as_ptr(), a.as_ptr(), &mut t),
            -1,
            "C: a key length modifier must fail"
        );
        assert_eq!(cerr.snapshot().4, "Unexpected format character '%'", "C: text");
        decref(c, croot);
        // But `s%` IS valid for a string VALUE, including right after a `?`.
        upk!(c, r, "{\"a\":\"txt\"}", 0, cs("{s:s%}"), sl,
             [a.as_ptr(), sp(sl, 0), lp(sl, 0)], &[], "unpack {{s:s%}}");
        upk!(c, r, "{\"a\":\"txt\"}", 0, cs("{s?s%}"), sl,
             [a.as_ptr(), sp(sl, 1), lp(sl, 1)], &[], "unpack {{s?s%}} present");
        upk!(c, r, "{}", 0, cs("{s?s%}"), sl, [a.as_ptr(), sp(sl, 2), lp(sl, 2)], &[],
             "unpack {{s?s%}} absent");
        upk!(c, r, "[\"txt\"]", 0, cs("[s%]"), sl, [sp(sl, 3), lp(sl, 3)], &[],
             "unpack [s%]");
    }
}

#[test]
fn deeply_recursive_formats() {
    // pack/unpack recurse once per nesting level with no depth limit of their
    // own, so the two implementations must at least agree at the depths a real
    // caller could use.
    let (c, r) = both();
    unsafe {
        for depth in [1usize, 2, 8, 32, 100] {
            // [[[[...1...]]]]
            let pfmt = format!("{}i{}", "[".repeat(depth), "]".repeat(depth));
            pk!(c, r, 0, cs(&pfmt), [7 as c_int], "pack {depth}-deep array");
            let text = format!("{}7{}", "[".repeat(depth), "]".repeat(depth));
            upk!(c, r, &text, 0, cs(&pfmt), sl, [ip(sl, 0)], &[],
                 "unpack {depth}-deep array");
            upk!(c, r, &text, JSON_STRICT, cs(&pfmt), sl, [ip(sl, 1)], &[],
                 "unpack {depth}-deep array STRICT");
            // {"a":{"a":{...7...}}}
            let ofmt = format!("{}i{}", "{s:".repeat(depth), "}".repeat(depth));
            let key = cs("a");
            let keys: Vec<*const c_char> = (0..depth).map(|_| key.as_ptr()).collect();
            let otext = format!("{}7{}", "{\"a\":".repeat(depth), "}".repeat(depth));
            // Build the argument list dynamically via the hand-built va_list.
            let keep: Vec<CString> = (0..depth).map(|_| cs("a")).collect();
            let mut plan: Vec<A> = (0..depth).map(A::Key).collect();
            plan.push(A::ISlot(0));
            let fmt_c = cs(&ofmt);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            // For pack, every A::ISlot must be an int instead.
            let mut ppl: Vec<A> = (0..depth).map(A::Key).collect();
            ppl.push(A::Int(7));
            let mut cw2 = materialize(&ppl, &keep, std::ptr::null_mut(), false);
            let mut rw2 = cw2.clone();
            let mut ct2 = va_tag(&mut cw2);
            let mut rt2 = va_tag(&mut rw2);
            let cj = (c.json_vpack_ex)(&mut cerr, 0, fmt_c.as_ptr(),
                                       &mut ct2 as *mut VaTag as *mut c_void);
            let rj = (r.json_vpack_ex)(&mut rerr, 0, fmt_c.as_ptr(),
                                       &mut rt2 as *mut VaTag as *mut c_void);
            diff_eq!(cerr.raw(), rerr.raw(), "pack {depth}-deep object error");
            diff_eq!(canon(c, cj), canon(r, rj), "pack {depth}-deep object");
            decref(c, cj);
            decref(r, rj);

            let croot = load(c, &otext);
            let rroot = load(r, &otext);
            let mut cslots = Slots::poisoned();
            let mut rslots = Slots::poisoned();
            let cp: *mut Slots = &mut cslots;
            let rp: *mut Slots = &mut rslots;
            let mut cw = materialize(&plan, &keep, cp, false);
            let mut rw = materialize(&plan, &keep, rp, false);
            let mut ctag = va_tag(&mut cw);
            let mut rtag = va_tag(&mut rw);
            let mut cerr = json_error_t::poisoned();
            let mut rerr = json_error_t::poisoned();
            let cret = (c.json_vunpack_ex)(croot, &mut cerr, JSON_STRICT, fmt_c.as_ptr(),
                                           &mut ctag as *mut VaTag as *mut c_void);
            let rret = (r.json_vunpack_ex)(rroot, &mut rerr, JSON_STRICT, fmt_c.as_ptr(),
                                           &mut rtag as *mut VaTag as *mut c_void);
            diff_eq!(cret, rret, "unpack {depth}-deep object return");
            diff_eq!(cerr.raw(), rerr.raw(), "unpack {depth}-deep object error");
            diff_eq!(cslots.summary(c), rslots.summary(r),
                     "unpack {depth}-deep object targets");
            assert_eq!(cret, 0, "C: {depth}-deep unpack must succeed");
            decref(c, croot);
            decref(r, rroot);
            let _ = keys;
        }
    }
}

#[test]
fn json_pack_matches_json_pack_ex_across_many_formats() {
    // `json_pack` is a distinct variadic entry point (error == NULL, flags == 0,
    // one named parameter => gp_offset 8), so every format is also driven through
    // it. `error == NULL` additionally exercises the `!error` guards in
    // jsonp_error_init / jsonp_error_vset.
    let (c, r) = both();
    let mut rng = Rng::new(0x08_0B01);
    unsafe {
        let a = cs("abcdef");
        let b = cs("012345");
        let k = cs("key");
        let nul: *const c_char = std::ptr::null();
        let jnul: *mut json_t = std::ptr::null_mut();
        pkn!(c, r, cs("n"), [], "json_pack n");
        pkn!(c, r, cs("b"), [1 as c_int], "json_pack b");
        pkn!(c, r, cs("i"), [-5 as c_int], "json_pack i");
        pkn!(c, r, cs("I"), [i64::MIN as json_int_t], "json_pack I");
        pkn!(c, r, cs("f"), [2.5f64], "json_pack f");
        pkn!(c, r, cs("f"), [f64::NAN], "json_pack f NAN");
        pkn!(c, r, cs("s"), [a.as_ptr()], "json_pack s");
        pkn!(c, r, cs("s"), [nul], "json_pack s NULL");
        pkn!(c, r, cs("s#"), [a.as_ptr(), 3 as c_int], "json_pack s#");
        pkn!(c, r, cs("s%"), [a.as_ptr(), 3 as size_t], "json_pack s%");
        pkn!(c, r, cs("s+"), [a.as_ptr(), b.as_ptr()], "json_pack s+");
        pkn!(c, r, cs("s?"), [nul], "json_pack s?");
        pkn!(c, r, cs("s*"), [nul], "json_pack s*");
        pkn!(c, r, cs("o?"), [jnul], "json_pack o?");
        pkn!(c, r, cs("O*"), [jnul], "json_pack O*");
        pkn!(c, r, cs("[]"), [], "json_pack []");
        pkn!(c, r, cs("{}"), [], "json_pack {{}}");
        pkn!(c, r, cs("{s:[i,i],s:s#}"),
             [k.as_ptr(), 1 as c_int, 2 as c_int, k.as_ptr(), a.as_ptr(), 2 as c_int],
             "json_pack nested");
        pkn!(c, r, cs("q"), [], "json_pack bad char");
        pkn!(c, r, cs("{"), [], "json_pack unterminated");
        pkn!(c, r, cs("ii"), [1 as c_int, 2 as c_int], "json_pack garbage after");
        for i in 0..3000 {
            let x = rng.next_u32() as c_int;
            let y = rng.real();
            let t = cs(&safe_text(&mut rng, 10));
            pkn!(c, r, cs("{s:[i,f,s]}"), [k.as_ptr(), x, y, t.as_ptr()],
                 "iter {i}: json_pack randomised");
        }
    }
}
