//! Phase C — error-path differential tests for `src/pack_unpack.c` (plus
//! `json_vsprintf`/`json_sprintf`, whose failure paths are rows 87 and 88).
//!
//! Covers ERRORS.md rows **87, 88 and 221-280**: every rejection reachable
//! through `json_pack*` / `json_unpack*` / `json_sprintf`, the out-of-memory
//! paths that need a failing allocator, and the generic FFI boundaries the
//! table has no row for (NULL format, empty format, NULL `json_error_t*`,
//! undefined flag bits).
//!
//! This file is the ERROR-path complement to `a08_pack_unpack.rs`; the happy
//! paths and the randomised round-trips live there and are not repeated.
//!
//! For every case FOUR observables are compared:
//!
//!   a) the return value — `NULL` for `json_pack*`/`json_sprintf`, `-1` for
//!      `json_unpack*`,
//!   b) the **complete byte image** of the caller's `json_error_t` (`.raw()`),
//!      started from `json_error_t::poisoned()` so that "the library did not
//!      write here" is distinguishable from "it wrote a NUL". That single
//!      comparison pins `line`, `column`, `position`, `source`, `text` AND the
//!      code byte at `text[159]` at once — which matters a great deal here,
//!      because `jsonp_error_vset` drops a second error's *text* while
//!      `jsonp_error_set_source` still overwrites the *source*, so several of
//!      these paths end with a source that belongs to a different error than
//!      the text does,
//!   c) the exact numeric `json_error_code()` the ERRORS.md row documents, so
//!      each test proves "the same error", not merely "both failed",
//!   d) for unpack, the full image of a poisoned out-pointer block, so
//!      "wrote nothing" is distinguishable from "wrote a zero".
//!
//! Two rows (237 and 246) document the library returning a failure indicator
//! *without* recording an error; for those the code byte is uninitialised
//! memory and is deliberately NOT asserted (see the comments there).

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};

// ===========================================================================
// Helpers
// ===========================================================================

/// A canonical dump: `JSON_SORT_KEYS` removes any dependence on hash order and
/// `JSON_ENCODE_ANY` lets a bare scalar be dumped too. `None` means either the
/// value was NULL or the dump itself failed (both observable, both compared).
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
    let j = (api.json_loads)(
        t.as_ptr(),
        JSON_DECODE_ANY | JSON_ALLOW_NUL,
        std::ptr::null_mut(),
    );
    assert!(!j.is_null(), "{}: failed to parse root {text:?}", api.which);
    j
}

/// Printable, unambiguous rendering of arbitrary format-string bytes.
fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &x in b {
        match x {
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    s
}

// --- the poisoned out-pointer block (same design as a08) ----------------------

const NSLOT: usize = 8;
const POISON_I32: c_int = 0x5A5A_5A5Au32 as c_int;
const POISON_I64: json_int_t = 0x5A5A_5A5A_5A5A_5A5Au64 as json_int_t;
const POISON_LEN: size_t = 0x5A5A_5A5A_5A5A_5A5A;
const POISON_DBITS: u64 = 0x5A5A_5A5A_5A5A_5A5A;

/// The sentinel a `const char **` slot starts at. It points at a real,
/// readable C string so reading it back is always safe, and it is
/// distinguishable from anything jansson could store there.
static POISON_TEXT: &[u8; 11] = b"<<poison>>\0";

fn poison_str_ptr() -> *const c_char {
    POISON_TEXT.as_ptr() as *const c_char
}

/// The sentinel a `json_t **` slot starts at: a leaked, valid `json_t` shaped
/// like the `null` singleton (refcount `(size_t)-1`, so even an accidental
/// decref is a no-op). Shared by both libraries, so "untouched" compares equal.
fn sentinel_json() -> *mut json_t {
    static S: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        Box::leak(Box::new(json_t {
            type_: JSON_NULL,
            refcount: usize::MAX,
        })) as *mut json_t as usize
    }) as *mut json_t
}

/// One block of every out-pointer type `unpack()` can write through.
#[repr(C)]
struct Slots {
    ints: [c_int; NSLOT],
    i64s: [json_int_t; NSLOT],
    dbls: [f64; NSLOT],
    lens: [size_t; NSLOT],
    strs: [*const c_char; NSLOT],
    objs: [*mut json_t; NSLOT],
}

#[derive(PartialEq, Debug, Clone)]
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

    /// Release the extra reference an `O` took. Only the listed slots were
    /// filled by an `O` (an `o` must NOT be decref'd — it does not incref).
    unsafe fn decref_objs(&self, api: &Api, which: &[usize]) {
        for &i in which {
            let p = self.objs[i];
            if p != sentinel_json() && !p.is_null() {
                decref(api, p);
            }
        }
    }
}

// Slot address helpers, taking a raw `*mut Slots` so several can appear in one
// call's argument list.
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

// ---------------------------------------------------------------------------
// Observations
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct PObs {
    null: bool,
    snap: (c_int, c_int, c_int, String, String, c_int),
    raw: Vec<u8>,
    tree: Option<Vec<u8>>,
}

impl PObs {
    fn code(&self) -> c_int {
        self.snap.5
    }
    fn text(&self) -> &str {
        &self.snap.4
    }
    fn source(&self) -> &str {
        &self.snap.3
    }
    fn lcp(&self) -> (c_int, c_int, c_int) {
        (self.snap.0, self.snap.1, self.snap.2)
    }
}

#[derive(Clone, PartialEq)]
struct UObs {
    ret: c_int,
    snap: (c_int, c_int, c_int, String, String, c_int),
    raw: Vec<u8>,
    slots: SlotSummary,
}

impl UObs {
    fn code(&self) -> c_int {
        self.snap.5
    }
    fn text(&self) -> &str {
        &self.snap.4
    }
    fn source(&self) -> &str {
        &self.snap.3
    }
    fn lcp(&self) -> (c_int, c_int, c_int) {
        (self.snap.0, self.snap.1, self.snap.2)
    }
}

// ===========================================================================
// Comparison macros
// ===========================================================================

/// `json_pack_ex` with library-independent varargs. Returns the C observation.
macro_rules! epk {
    ($c:expr, $r:expr, $flags:expr, $fmt:expr, [$($arg:expr),* $(,)?], $($ctx:tt)*) => {{
        let capi_: &Api = $c;
        let rapi_: &Api = $r;
        let fmt_ = &$fmt;
        let mut cerr = json_error_t::poisoned();
        let mut rerr = json_error_t::poisoned();
        let cj = (capi_.json_pack_ex)(&mut cerr, $flags, fmt_.as_ptr(), $($arg),*);
        let rj = (rapi_.json_pack_ex)(&mut rerr, $flags, fmt_.as_ptr(), $($arg),*);
        let ctx_ = format!($($ctx)*);
        let co = PObs { null: cj.is_null(), snap: cerr.snapshot(), raw: cerr.raw(),
                        tree: canon(capi_, cj) };
        let ro = PObs { null: rj.is_null(), snap: rerr.snapshot(), raw: rerr.raw(),
                        tree: canon(rapi_, rj) };
        diff_eq!(co.null, ro.null, "json_pack_ex returned-NULL — {ctx_}");
        diff_eq!(co.snap.clone(), ro.snap.clone(),
                 "json_pack_ex error (line,col,pos,source,text,code) — {ctx_}");
        diff_eq!(co.raw.clone(), ro.raw.clone(),
                 "json_pack_ex error raw byte image — {ctx_}");
        diff_eq!(co.tree.clone(), ro.tree.clone(), "json_pack_ex packed tree — {ctx_}");
        decref(capi_, cj);
        decref(rapi_, rj);
        co
    }};
}

/// `json_unpack_ex` against a root parsed from `$text` by each library.
/// `$sl` is bound to a `*mut Slots` for the duration of the argument list.
macro_rules! eupk {
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
        let co = UObs { ret: cret, snap: cerr.snapshot(), raw: cerr.raw(),
                        slots: cslots.summary(capi_) };
        let ro = UObs { ret: rret, snap: rerr.snapshot(), raw: rerr.raw(),
                        slots: rslots.summary(rapi_) };
        diff_eq!(co.ret, ro.ret, "json_unpack_ex return — {ctx_}");
        diff_eq!(co.snap.clone(), ro.snap.clone(),
                 "json_unpack_ex error (line,col,pos,source,text,code) — {ctx_}");
        diff_eq!(co.raw.clone(), ro.raw.clone(),
                 "json_unpack_ex error raw byte image — {ctx_}");
        diff_eq!(co.slots.clone(), ro.slots.clone(),
                 "json_unpack_ex out-pointer block — {ctx_}");
        cslots.decref_objs(capi_, $oslots);
        rslots.decref_objs(rapi_, $oslots);
        decref(capi_, croot);
        decref(rapi_, rroot);
        co
    }};
}

// ===========================================================================
// Rows 87 / 88 — json_sprintf / json_vsprintf failure paths
// ===========================================================================

/// Compare one `json_sprintf`-shaped result: NULL-ness plus, when non-NULL,
/// the exact string bytes and length (an embedded NUL must survive).
unsafe fn cmp_string_json(c: &Api, r: &Api, cj: *mut json_t, rj: *mut json_t, ctx: &str) {
    diff_eq!(cj.is_null(), rj.is_null(), "json_sprintf NULL-ness — {ctx}");
    if !cj.is_null() && !rj.is_null() {
        diff_eq!(typeof_(cj), typeof_(rj), "json_sprintf type — {ctx}");
        let cl = (c.json_string_length)(cj);
        let rl = (r.json_string_length)(rj);
        diff_eq!(cl, rl, "json_sprintf length — {ctx}");
        let cb = std::slice::from_raw_parts((c.json_string_value)(cj) as *const u8, cl).to_vec();
        let rb = std::slice::from_raw_parts((r.json_string_value)(rj) as *const u8, rl).to_vec();
        diff_eq!(cb, rb, "json_sprintf bytes — {ctx}");
    }
    decref(c, cj);
    decref(r, rj);
}

/// Row 87 — `vsnprintf(NULL, 0, fmt, ap)` returns `< 0`, so `json_vsprintf`
/// bails out through `goto out` with `json` still NULL and NOTHING allocated.
///
/// ```c
///     length = vsnprintf(NULL, 0, fmt, ap);
///     if (length < 0)
///         goto out;
/// ```
///
/// Two independent ways to make glibc's `vsnprintf` fail are used:
///   * `%ls` / `%lc` with a wide character that has no representation in the
///     process locale (the tests never call `setlocale`, so the locale is "C"
///     and anything above U+007F fails with `EILSEQ`), and
///   * a total field width above `INT_MAX`, which fails with `EOVERFLOW`.
///
/// Both libraries call the same libc, so the failure itself is shared; what is
/// under test is that each of them turns it into `NULL` rather than, say,
/// treating the negative length as a huge unsigned size.
#[test]
fn row_087_vsprintf_vsnprintf_encoding_error() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // wchar_t is 4 bytes on Linux/x86-64.
        let wide_bad: [u32; 2] = [0x100, 0];
        let wide_ok: [u32; 2] = [0x41, 0];
        let a = cs("a");
        let b = cs("b");

        let f_ls = cs("%ls");
        let cj = (c.json_sprintf)(f_ls.as_ptr(), wide_bad.as_ptr());
        let rj = (r.json_sprintf)(f_ls.as_ptr(), wide_bad.as_ptr());
        cmp_string_json(c, r, cj, rj, "row 87: %ls with U+0100 in the C locale");
        assert!(cj.is_null(), "C: %ls with an unconvertible wide char must fail");

        // Control: the same format with a representable wide char succeeds, so
        // the NULL above really came from the `length < 0` arm.
        let cj = (c.json_sprintf)(f_ls.as_ptr(), wide_ok.as_ptr());
        let rj = (r.json_sprintf)(f_ls.as_ptr(), wide_ok.as_ptr());
        assert!(!cj.is_null(), "C: %ls with U+0041 must succeed");
        cmp_string_json(c, r, cj, rj, "row 87 control: %ls with U+0041");

        let f_lc = cs("%lc");
        let cj = (c.json_sprintf)(f_lc.as_ptr(), 0x100 as c_uint);
        let rj = (r.json_sprintf)(f_lc.as_ptr(), 0x100 as c_uint);
        cmp_string_json(c, r, cj, rj, "row 87: %lc with U+0100");
        assert!(cj.is_null(), "C: %lc with an unconvertible wide char must fail");

        // Total width > INT_MAX -> EOVERFLOW, a different libc failure mode.
        let f_ovf = cs("%2000000000s%2000000000s");
        let cj = (c.json_sprintf)(f_ovf.as_ptr(), a.as_ptr(), b.as_ptr());
        let rj = (r.json_sprintf)(f_ovf.as_ptr(), a.as_ptr(), b.as_ptr());
        cmp_string_json(c, r, cj, rj, "row 87: field width above INT_MAX");
        assert!(cj.is_null(), "C: a width above INT_MAX must fail");

        // ... and through the real va_list entry point. The C does `va_copy`
        // BEFORE the first vsnprintf and then returns on the error path
        // without ever touching the copy, which is exactly where a vararg
        // handling bug would hide.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vsprintf");
        let rfn = sym_addr("Rust", b"json_vsprintf");
        let cj = (sh.vsprintf)(cfn, f_ls.as_ptr(), wide_bad.as_ptr());
        let rj = (sh.vsprintf)(rfn, f_ls.as_ptr(), wide_bad.as_ptr());
        cmp_string_json(c, r, cj, rj, "row 87: json_vsprintf %ls");
        assert!(cj.is_null(), "C: json_vsprintf must fail too");
        let cj = (sh.vsprintf)(cfn, f_ovf.as_ptr(), a.as_ptr(), b.as_ptr());
        let rj = (sh.vsprintf)(rfn, f_ovf.as_ptr(), a.as_ptr(), b.as_ptr());
        cmp_string_json(c, r, cj, rj, "row 87: json_vsprintf overflow width");
        assert!(cj.is_null(), "C: json_vsprintf must fail too");
    }
}

/// Row 88 — the formatted result is not valid UTF-8, so the temp buffer is
/// freed and NULL is returned:
///
/// ```c
///     vsnprintf(buf, (size_t)length + 1, fmt, aq);
///     if (!utf8_check_string(buf, length)) {
///         jsonp_free(buf);
///         goto out;
///     }
/// ```
#[test]
fn row_088_vsprintf_result_is_not_valid_utf8() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let fs = cs("%s");
        let bad: Vec<Vec<u8>> = vec![
            b"\xff".to_vec(),
            b"\xfe".to_vec(),
            b"\x80".to_vec(),
            b"a\x80b".to_vec(),
            b"\xc3".to_vec(),             // truncated 2-byte
            b"\xe2\x82".to_vec(),         // truncated 3-byte
            b"\xed\xa0\x80".to_vec(),     // UTF-8 encoded surrogate
            b"\xf4\x90\x80\x80".to_vec(), // above U+10FFFF
            b"\xc0\x80".to_vec(),         // overlong NUL
            b"\xc1\xbf".to_vec(),         // overlong
            b"\xf8\x88\x80\x80\x80".to_vec(), // 5-byte sequence
        ];
        for bytes in &bad {
            let buf = cs_bytes(bytes);
            let cj = (c.json_sprintf)(fs.as_ptr(), buf.as_ptr());
            let rj = (r.json_sprintf)(fs.as_ptr(), buf.as_ptr());
            let ctx = format!("row 88: json_sprintf(\"%s\", \"{}\")", show(bytes));
            cmp_string_json(c, r, cj, rj, &ctx);
            assert!(cj.is_null(), "C: {ctx} must return NULL");
        }
        // Mixed: a valid prefix followed by an invalid tail, so the check has
        // to scan past good bytes first.
        for bytes in &bad {
            let mut v = b"ok\xc3\xa9".to_vec();
            v.extend_from_slice(bytes);
            let buf = cs_bytes(&v);
            let cj = (c.json_sprintf)(fs.as_ptr(), buf.as_ptr());
            let rj = (r.json_sprintf)(fs.as_ptr(), buf.as_ptr());
            let ctx = format!("row 88: valid prefix + \"{}\"", show(bytes));
            cmp_string_json(c, r, cj, rj, &ctx);
            assert!(cj.is_null(), "C: {ctx} must return NULL");
        }
        // `%c` with a lone continuation byte: the buffer is exactly one byte.
        let fc = cs("%c");
        for byte in [0x80u8, 0xbf, 0xc0, 0xff] {
            let cj = (c.json_sprintf)(fc.as_ptr(), byte as c_int);
            let rj = (r.json_sprintf)(fc.as_ptr(), byte as c_int);
            let ctx = format!("row 88: json_sprintf(\"%c\", {byte:#04x})");
            cmp_string_json(c, r, cj, rj, &ctx);
            assert!(cj.is_null(), "C: {ctx} must return NULL");
        }
        // Control: valid UTF-8 output must still succeed.
        let good = cs("ok\u{e9}\u{20ac}\u{1f600}");
        let cj = (c.json_sprintf)(fs.as_ptr(), good.as_ptr());
        let rj = (r.json_sprintf)(fs.as_ptr(), good.as_ptr());
        assert!(!cj.is_null(), "C: valid UTF-8 must succeed");
        cmp_string_json(c, r, cj, rj, "row 88 control: valid UTF-8");

        // Same through json_vsprintf.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vsprintf");
        let rfn = sym_addr("Rust", b"json_vsprintf");
        for bytes in &bad {
            let buf = cs_bytes(bytes);
            let cj = (sh.vsprintf)(cfn, fs.as_ptr(), buf.as_ptr());
            let rj = (sh.vsprintf)(rfn, fs.as_ptr(), buf.as_ptr());
            let ctx = format!("row 88: json_vsprintf(\"%s\", \"{}\")", show(bytes));
            cmp_string_json(c, r, cj, rj, &ctx);
            assert!(cj.is_null(), "C: {ctx} must return NULL");
        }
        // NULL format string: `vsnprintf(NULL, 0, NULL, ap)` is what the C
        // does — glibc rejects it and returns -1, so this lands on row 87's
        // arm. Included because a NULL `fmt` is a real FFI input with no guard
        // anywhere in json_vsprintf.
        let cj = (c.json_sprintf)(std::ptr::null::<c_char>());
        let rj = (r.json_sprintf)(std::ptr::null::<c_char>());
        cmp_string_json(c, r, cj, rj, "json_sprintf(NULL)");
        // Empty format: length == 0 takes the `json_string("")` arm.
        let empty = cs("");
        let cj = (c.json_sprintf)(empty.as_ptr());
        let rj = (r.json_sprintf)(empty.as_ptr());
        assert!(!cj.is_null(), "C: an empty format yields the empty string");
        cmp_string_json(c, r, cj, rj, "json_sprintf(\"\")");
    }
}

// ===========================================================================
// Rows 221 / 222 — NULL and empty pack format string
// ===========================================================================

/// The `!fmt || !*fmt` guard runs BEFORE `jsonp_error_init(error, NULL)`, so it
/// installs source `"<format>"` and line/column/position `-1/-1/0`.
#[test]
fn rows_221_222_pack_null_and_empty_format_string() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let empty = cs("");
        // `flags` is a plain `size_t` with no validation in pack_unpack.c, so
        // every bit pattern is a legal input and must be ignored identically.
        let mut rng = Rng::new(0x14_0221);
        let mut flag_words: Vec<size_t> = vec![
            0,
            JSON_VALIDATE_ONLY,
            JSON_STRICT,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            0x4,
            0x8,
            0x10,
            0x20,
            0x8000,
            1usize << 63,
            size_t::MAX,
        ];
        for _ in 0..24 {
            flag_words.push(rng.next_u64() as size_t);
        }

        for &flags in &flag_words {
            for (fmt, tag) in [
                (std::ptr::null::<c_char>(), "NULL"),
                (empty.as_ptr(), "\"\""),
            ] {
                let mut cerr = json_error_t::poisoned();
                let mut rerr = json_error_t::poisoned();
                let cj = (c.json_pack_ex)(&mut cerr, flags, fmt);
                let rj = (r.json_pack_ex)(&mut rerr, flags, fmt);
                diff_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "json_pack_ex({tag}) NULL-ness flags={flags:#x}"
                );
                diff_eq!(
                    cerr.snapshot(),
                    rerr.snapshot(),
                    "json_pack_ex({tag}) error flags={flags:#x}"
                );
                diff_eq!(
                    cerr.raw(),
                    rerr.raw(),
                    "json_pack_ex({tag}) raw error flags={flags:#x}"
                );
                assert!(cj.is_null(), "C: {tag} format must fail");
                assert_eq!(cerr.source_str(), "<format>", "C: source for {tag}");
                assert_eq!(
                    cerr.text_str(),
                    "NULL or empty format string",
                    "C: text for {tag}"
                );
                assert_eq!(
                    (cerr.line, cerr.column, cerr.position),
                    (-1, -1, 0),
                    "C: line/column/position for {tag}"
                );
                assert_eq!(
                    cerr.code(),
                    JSON_ERROR_INVALID_ARGUMENT,
                    "C: code 4 for {tag}"
                );
                decref(c, cj);
                decref(r, rj);
            }
        }

        // A NULL json_error_t* must be tolerated on every entry point.
        for fmt in [std::ptr::null::<c_char>(), empty.as_ptr()] {
            let cj = (c.json_pack_ex)(std::ptr::null_mut(), 0, fmt);
            let rj = (r.json_pack_ex)(std::ptr::null_mut(), 0, fmt);
            diff_eq!(cj.is_null(), rj.is_null(), "json_pack_ex(NULL error)");
            let cj = (c.json_pack)(fmt);
            let rj = (r.json_pack)(fmt);
            diff_eq!(cj.is_null(), rj.is_null(), "json_pack (error == NULL)");
        }

        // ... and through the real va_list entry point.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vpack_ex");
        let rfn = sym_addr("Rust", b"json_vpack_ex");
        for &flags in &flag_words {
            for fmt in [std::ptr::null::<c_char>(), empty.as_ptr()] {
                let mut cerr = json_error_t::poisoned();
                let mut rerr = json_error_t::poisoned();
                let cj = (sh.vpack_ex)(cfn, &mut cerr, flags, fmt);
                let rj = (sh.vpack_ex)(rfn, &mut rerr, flags, fmt);
                diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex bad fmt NULL-ness");
                diff_eq!(cerr.raw(), rerr.raw(), "vpack_ex bad fmt raw error");
                assert!(cj.is_null(), "C: vpack_ex must reject the format");
                // The va_list is never read on this path, but it IS copied in
                // the C (`va_copy` happens after the guard, so not even that).
                let cj = (sh.vpack_ex)(cfn, std::ptr::null_mut(), flags, fmt);
                let rj = (sh.vpack_ex)(rfn, std::ptr::null_mut(), flags, fmt);
                diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex NULL error tolerated");
            }
        }
    }
}

// ===========================================================================
// Row 223 — garbage after the format string
// ===========================================================================

#[test]
fn row_223_pack_garbage_after_format_string() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("k");
        let v = cs("vv");
        // (format, expected line, column, position)
        let cases: &[(&str, c_int, c_int, c_int)] = &[
            ("[]]", 1, 3, 3),
            ("{}}", 1, 3, 3),
            ("ii", 1, 2, 2),
            ("nn", 1, 2, 2),
            ("s#?", 1, 3, 3),
            ("s?*", 1, 3, 3),
            ("n}", 1, 2, 2),
            ("n]", 1, 2, 2),
            ("nq", 1, 2, 2),
            ("[i]i", 1, 4, 4),
            ("\nn n", 2, 3, 4),
        ];
        for &(fmt, line, col, pos) in cases {
            let f = cs(fmt);
            let o = epk!(
                c, r, 0, f,
                [k.as_ptr(), 1 as c_int, v.as_ptr(), 2 as c_int, 3.5f64],
                "row 223 {fmt:?}"
            );
            assert!(o.null, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<format>", "C: source for {fmt:?}");
            assert_eq!(o.text(), "Garbage after format string", "C: text {fmt:?}");
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9 for {fmt:?}");
            assert_eq!(o.lcp(), (line, col, pos), "C: position for {fmt:?}");
        }
        // "{s:i}x": the built object must be decref'd, not leaked, and the
        // error must be the garbage one rather than anything from the object.
        let f = cs("{s:i}x");
        let o = epk!(c, r, 0, f, [k.as_ptr(), 1 as c_int], "row 223 {{s:i}}x");
        assert!(o.null && o.text() == "Garbage after format string");
        assert_eq!(o.lcp(), (1, 6, 6));
        // Same through json_vpack_ex.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vpack_ex");
        let rfn = sym_addr("Rust", b"json_vpack_ex");
        for fmt in ["[]]", "{s:i}x", "ii"] {
            let f = cs(fmt);
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cfn, &mut ce, 0, f.as_ptr(), k.as_ptr(), 1 as c_int);
            let rj = (sh.vpack_ex)(rfn, &mut re, 0, f.as_ptr(), k.as_ptr(), 1 as c_int);
            diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex garbage {fmt:?}");
            diff_eq!(ce.raw(), re.raw(), "vpack_ex garbage raw error {fmt:?}");
            assert!(cj.is_null());
            assert_eq!(ce.text_str(), "Garbage after format string");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 224 — unrecognised format character in pack's `default:` arm
// ===========================================================================

/// `pack()`'s recognised characters are exactly `{ [ s n b i I f O o`. Every
/// other byte lands in the `default:` arm. That arm consumes no vararg, so the
/// sweep below can call with no arguments at all.
#[test]
fn row_224_pack_unexpected_format_character() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        const PACK_STARTERS: &[u8] = b"{[sniIbfOo";
        for b in 1u8..=255 {
            if PACK_STARTERS.contains(&b) {
                continue; // a value starter: covered elsewhere
            }
            let f = cs_bytes(&[b]);
            let o = epk!(c, r, 0, f, [], "row 224 pack fmt {:?}", show(&[b]));
            // Whitespace, ',' and ':' are skipped by next_token, so the token
            // ends up as the terminating NUL -> still the default arm, but
            // with a NUL inside the message.
            let skipped = matches!(b, b' ' | b'\t' | b'\n' | b',' | b':');
            assert!(o.null, "C: format {:?} must fail", show(&[b]));
            assert_eq!(o.source(), "<format>", "C: source for {:?}", show(&[b]));
            assert_eq!(
                o.code(),
                JSON_ERROR_INVALID_FORMAT,
                "C: code 9 for {:?}",
                show(&[b])
            );
            if skipped {
                // `%c` with a 0 argument writes a NUL byte INTO the message,
                // so the C string stops right after the opening quote.
                assert_eq!(
                    o.text(),
                    "Unexpected format character '",
                    "C: skipped byte {:?}",
                    show(&[b])
                );
            } else if b < 0x80 {
                assert_eq!(
                    o.text(),
                    format!("Unexpected format character '{}'", b as char),
                    "C: text for {:?}",
                    show(&[b])
                );
            }
            // A literal newline in the format is skipped by next_token and
            // bumps the line counter before the token is recorded.
            let want_line = if b == b'\n' { 2 } else { 1 };
            assert_eq!(o.lcp().0, want_line, "C: line for {:?}", show(&[b]));
        }
        // ... and in every other position: array element, object value, and
        // nested inside a container.
        let k = cs("k");
        for b in [b'q', b'x', b'}', b']', b'!', b'*', b'?', b'#', b'%', b'+', b'F'] {
            let f = cs_bytes(&[b'[', b, b']']);
            let o = epk!(c, r, 0, f, [], "row 224 array elem {:?}", show(&[b]));
            assert!(o.null, "C: [{}] must fail", b as char);
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
        }
        // In the object VALUE position. '#', '%' and '+' are excluded because
        // they are swallowed as the KEY's length modifier (read_string peeks at
        // the token after the 's'), so they never reach pack()'s default arm
        // there; rows 238/241/243 cover them instead.
        for b in [b'q', b'x', b'}', b']', b'!', b'*', b'?', b'F'] {
            let f = cs_bytes(&[b'{', b's', b':', b, b'}']);
            let o = epk!(c, r, 0, f, [k.as_ptr()], "row 224 object value {:?}", show(&[b]));
            assert!(o.null, "C: {{s:{}}} must fail", b as char);
            // The value error's TEXT wins (set first) but pack_object then
            // overwrites the SOURCE with "<args>" from the dropped
            // "NULL object value" error — pinned by the raw comparison above.
            if b != b'}' {
                assert_eq!(
                    o.text(),
                    format!("Unexpected format character '{}'", b as char),
                    "C: object value text for {}",
                    b as char
                );
                assert_eq!(o.source(), "<args>", "C: source flipped to <args>");
            }
        }
    }
}

// ===========================================================================
// Rows 225 / 231 — a container that reaches the end of the format string
// ===========================================================================

#[test]
fn rows_225_231_pack_unterminated_containers() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("k");
        let v = cs("vv");
        // (format, line, column, position)
        let cases: &[(&str, c_int, c_int, c_int)] = &[
            ("{", 1, 2, 2),
            ("{s:i", 1, 5, 5),
            ("{s:i,", 1, 6, 6),
            ("{s:s", 1, 5, 5),
            ("[", 1, 2, 2),
            ("[i", 1, 3, 3),
            ("[i,", 1, 4, 4),
            ("[[", 1, 3, 3),
            ("[{", 1, 3, 3),
            ("{s:[", 1, 5, 5),
            ("{s:{", 1, 5, 5),
            ("[\n", 2, 1, 3),
        ];
        // Every vararg is a valid `char *`, so whichever of them a format
        // happens to consume (as a string, as an int, or not at all) can never
        // be dereferenced out of bounds.
        for &(fmt, line, col, pos) in cases {
            let f = cs(fmt);
            let o = epk!(
                c, r, 0, f,
                [k.as_ptr(), v.as_ptr(), k.as_ptr(), v.as_ptr()],
                "rows 225/231 {fmt:?}"
            );
            assert!(o.null, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<format>", "C: source for {fmt:?}");
            assert_eq!(
                o.text(),
                "Unexpected end of format string",
                "C: text for {fmt:?}"
            );
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9 {fmt:?}");
            assert_eq!(o.lcp(), (line, col, pos), "C: position for {fmt:?}");
        }
    }
}

// ===========================================================================
// Row 226 — the object key position is not 's'
// ===========================================================================

#[test]
fn row_226_pack_object_key_position_is_not_s() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Every byte in the key slot. 's' is the only legal one; the NUL and
        // '}' terminate instead; whitespace/','/':' are skipped.
        for b in 1u8..=255 {
            if b == b's' {
                continue;
            }
            let f = cs_bytes(&[b'{', b, b':', b'i', b'}']);
            let o = epk!(c, r, 0, f, [], "row 226 key byte {:?}", show(&[b]));
            if b == b'}' {
                // "{}:i}" — the object closes at once, then ':' is skipped and
                // 'i' is garbage after the format string.
                assert!(o.null);
                assert_eq!(o.text(), "Garbage after format string");
                continue;
            }
            assert!(o.null, "C: key byte {:?} must fail", show(&[b]));
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            if matches!(b, b' ' | b'\t' | b'\n' | b',' | b':') {
                // skipped -> the ':' and 'i' become the key token
                assert_eq!(o.text(), "Expected format 's', got 'i'");
            } else if b < 0x80 {
                assert_eq!(
                    o.text(),
                    format!("Expected format 's', got '{}'", b as char),
                    "C: text for key byte {:?}",
                    show(&[b])
                );
                assert_eq!(o.source(), "<format>");
            }
        }
        // The documented example, with its exact column.
        let f = cs("{i:i}");
        let o = epk!(c, r, 0, f, [1 as c_int, 2 as c_int], "row 226 {{i:i}}");
        assert_eq!(o.text(), "Expected format 's', got 'i'");
        assert_eq!(o.lcp(), (1, 2, 2), "C: column 2");
        assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
    }
}

// ===========================================================================
// Rows 227 / 228 — object key argument NULL / not valid UTF-8
// ===========================================================================

#[test]
fn rows_227_228_pack_object_key_null_or_invalid_utf8() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 227 — NULL key. read_string sets "NULL object key" and the value
        // is still packed (and then discarded because has_error is set).
        for (fmt, extra_int) in [("{s:i}", true), ("{s:n}", false), ("{s:[i]}", true)] {
            let f = cs(fmt);
            let o = if extra_int {
                epk!(c, r, 0, f, [std::ptr::null::<c_char>(), 2 as c_int], "row 227 {fmt:?}")
            } else {
                epk!(c, r, 0, f, [std::ptr::null::<c_char>()], "row 227 {fmt:?}")
            };
            assert!(o.null, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<args>", "C: source for {fmt:?}");
            assert_eq!(o.text(), "NULL object key", "C: text for {fmt:?}");
            assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12 for {fmt:?}");
        }
        let f = cs("{s:i}");
        let o = epk!(
            c, r, 0, f,
            [std::ptr::null::<c_char>(), 2 as c_int],
            "row 227 column"
        );
        assert_eq!(o.lcp(), (1, 2, 2), "C: the key token's position");

        // Row 228 — key not valid UTF-8, both in the plain and the length-
        // modifier (`s#`) path.
        let bad: Vec<Vec<u8>> = vec![
            b"\xff".to_vec(),
            b"\xfe\xfe".to_vec(),
            b"a\x80b".to_vec(),
            b"\xc3".to_vec(),
            b"\xe2\x82".to_vec(),
            b"\xed\xa0\x80".to_vec(),
            b"\xf4\x90\x80\x80".to_vec(),
            b"\xc0\x80".to_vec(),
        ];
        for bytes in &bad {
            let buf = cs_bytes(bytes);
            let f = cs("{s:i}");
            let o = epk!(c, r, 0, f, [buf.as_ptr(), 2 as c_int], "row 228 {:?}", show(bytes));
            assert!(o.null, "C: an invalid UTF-8 key must fail");
            assert_eq!(o.source(), "<args>", "C: source");
            assert_eq!(o.text(), "Invalid UTF-8 object key", "C: text");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8, "C: code 5");

            let f = cs("{s#:i}");
            let o = epk!(
                c, r, 0, f,
                [buf.as_ptr(), bytes.len() as c_int, 2 as c_int],
                "row 228 s# {:?}", show(bytes)
            );
            assert!(o.null, "C: an invalid UTF-8 key via s# must fail");
            assert_eq!(o.text(), "Invalid UTF-8 object key");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8);
        }
    }
}

// ===========================================================================
// Rows 229 / 232 — a value that packs to NULL, and error STICKINESS
// ===========================================================================

/// `jsonp_error_vset` silently drops any SECOND error, so
/// `json_pack("{s:s}", "k", NULL)` reports `"NULL string"` — the error set by
/// `read_string` — and NOT `"NULL object value"`. A port that overwrote the
/// first error would still return NULL, so only the exact text/code proves it.
/// `jsonp_error_set_source`, however, is NOT gated by the same check, so the
/// dropped error's SOURCE still lands in the struct; that is pinned by the raw
/// byte-image comparison inside `epk!`.
#[test]
fn rows_229_232_pack_null_value_and_error_stickiness() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("k");

        // Row 229 — object value packs to NULL, value token is not '*'.
        let f = cs("{s:s}");
        let o = epk!(c, r, 0, f, [k.as_ptr(), std::ptr::null::<c_char>()], "row 229");
        assert!(o.null, "C: must fail");
        assert_eq!(
            o.text(),
            "NULL string",
            "C: the FIRST error wins — not \"NULL object value\""
        );
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
        assert_eq!(o.source(), "<args>", "C: source");
        assert_eq!(o.lcp(), (1, 4, 4), "C: the value token's position");

        // The same with 'o'/'O', where read_string is not involved.
        for fmt in ["{s:o}", "{s:O}"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [k.as_ptr(), std::ptr::null_mut::<json_t>()], "row 229 {fmt}");
            assert!(o.null);
            assert_eq!(o.text(), "NULL object", "C: first error wins for {fmt}");
            assert_eq!(o.code(), JSON_ERROR_NULL_VALUE);
        }
        // And where the first error is a FORMAT error: the text is the format
        // error's, while the source has been overwritten with "<args>".
        let f = cs("{s:q}");
        let o = epk!(c, r, 0, f, [k.as_ptr()], "row 229 format-error stickiness");
        assert!(o.null);
        assert_eq!(o.text(), "Unexpected format character 'q'");
        assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
        assert_eq!(
            o.source(),
            "<args>",
            "C: the dropped \"NULL object value\" error still set the source"
        );

        // Row 229's `'*'` exception: an object value that packs to NULL is NOT
        // an error when the value token is '*'.
        for (fmt, args_null_json) in [("{s:o*}", true), ("{s:O*}", true)] {
            let f = cs(fmt);
            let o = if args_null_json {
                epk!(c, r, 0, f, [k.as_ptr(), std::ptr::null_mut::<json_t>()], "row 229 star {fmt}")
            } else {
                epk!(c, r, 0, f, [k.as_ptr()], "row 229 star {fmt}")
            };
            assert!(!o.null, "C: {fmt} with NULL must SUCCEED (the '*' modifier)");
            assert_eq!(o.tree.as_deref(), Some(&b"{}"[..]), "C: the slot is omitted");
        }
        // `s*` in an object value: read_string returns NULL without an error
        // and the '*' suppresses "NULL object value" too.
        let f = cs("{s:s*}");
        let o = epk!(c, r, 0, f, [k.as_ptr(), std::ptr::null::<c_char>()], "row 229 s*");
        assert!(!o.null, "C: {{s:s*}} with a NULL string must succeed");
        assert_eq!(o.tree.as_deref(), Some(&b"{}"[..]));
        // `s?` yields json_null() in the slot instead.
        let f = cs("{s:s?}");
        let o = epk!(c, r, 0, f, [k.as_ptr(), std::ptr::null::<c_char>()], "row 229 s?");
        assert!(!o.null);
        assert_eq!(o.tree.as_deref(), Some(&b"{\"k\": null}"[..]));

        // Row 232 — array element packs to NULL. NOTE the asymmetry with
        // pack_object: pack_array only sets has_error, it does NOT call
        // set_error, so the source stays whatever read_string wrote.
        let f = cs("[s]");
        let o = epk!(c, r, 0, f, [std::ptr::null::<c_char>()], "row 232");
        assert!(o.null, "C: [s] with NULL must fail");
        assert_eq!(o.text(), "NULL string");
        assert_eq!(o.source(), "<args>");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE);
        assert_eq!(o.lcp(), (1, 2, 2));
        for fmt in ["[o]", "[O]"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [std::ptr::null_mut::<json_t>()], "row 232 {fmt}");
            assert!(o.null);
            assert_eq!(o.text(), "NULL object");
            assert_eq!(o.code(), JSON_ERROR_NULL_VALUE);
        }
        // and the '*' exception inside an array
        for fmt in ["[o*]", "[O*]", "[s*]"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [std::ptr::null_mut::<json_t>()], "row 232 star {fmt}");
            assert!(!o.null, "C: {fmt} with NULL must succeed");
            assert_eq!(o.tree.as_deref(), Some(&b"[]"[..]));
        }
        // Two failing slots in a row: the first error must still be the one
        // reported, which is the whole point of the stickiness rule.
        let f = cs("[s,o]");
        let o = epk!(
            c, r, 0, f,
            [std::ptr::null::<c_char>(), std::ptr::null_mut::<json_t>()],
            "row 232 two failures"
        );
        assert!(o.null);
        assert_eq!(o.text(), "NULL string", "C: the first failure wins");
        assert_eq!(o.lcp(), (1, 2, 2));
    }
}

// ===========================================================================
// Rows 234-238 — `s` and its modifiers
// ===========================================================================

#[test]
fn rows_234_235_pack_string_null_or_invalid_utf8() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 234 — a NULL string with no ?/* modifier.
        let f = cs("s");
        let o = epk!(c, r, 0, f, [std::ptr::null::<c_char>()], "row 234");
        assert!(o.null, "C: pack(\"s\", NULL) must fail");
        assert_eq!(o.source(), "<args>", "C: source");
        assert_eq!(o.text(), "NULL string", "C: text");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
        assert_eq!(o.lcp(), (1, 1, 1), "C: column 1");

        // Row 235 — a non-UTF-8 string, plain and with an explicit length.
        let bad: Vec<Vec<u8>> = vec![
            b"\xff".to_vec(),
            b"\xff\xfe".to_vec(),
            b"a\x80b".to_vec(),
            b"\xc3".to_vec(),
            b"\xe2\x82".to_vec(),
            b"\xed\xa0\x80".to_vec(),
            b"\xf4\x90\x80\x80".to_vec(),
            b"\xc0\x80".to_vec(),
            b"\xc1\xbf".to_vec(),
            b"\xf8\x88\x80\x80\x80".to_vec(),
        ];
        for bytes in &bad {
            let buf = cs_bytes(bytes);
            let f = cs("s");
            let o = epk!(c, r, 0, f, [buf.as_ptr()], "row 235 {:?}", show(bytes));
            assert!(o.null, "C: invalid UTF-8 must fail");
            assert_eq!(o.source(), "<args>");
            assert_eq!(o.text(), "Invalid UTF-8 string");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8, "C: code 5");

            let f = cs("s#");
            let o = epk!(
                c, r, 0, f,
                [buf.as_ptr(), bytes.len() as c_int],
                "row 235 s# {:?}", show(bytes)
            );
            assert!(o.null);
            assert_eq!(o.text(), "Invalid UTF-8 string");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8);

            let f = cs("s%");
            let o = epk!(
                c, r, 0, f,
                [buf.as_ptr(), bytes.len() as size_t],
                "row 235 s% {:?}", show(bytes)
            );
            assert!(o.null);
            assert_eq!(o.text(), "Invalid UTF-8 string");
        }
        // Nested, so the container unwind path runs too.
        let k = cs("k");
        let buf = cs_bytes(b"\xff");
        // The array forms consume only the bad string; the object forms consume
        // a valid key first.
        for fmt in ["[s]", "[[s]]"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [buf.as_ptr()], "row 235 nested {fmt}");
            assert!(o.null, "C: {fmt} must fail");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8, "C: code for {fmt}");
        }
        for fmt in ["{s:s}", "{s:[s]}"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [k.as_ptr(), buf.as_ptr()], "row 235 nested {fmt}");
            assert!(o.null, "C: {fmt} must fail");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8, "C: code for {fmt}");
        }
    }
}

#[test]
fn rows_236_237_pack_optional_strings() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 236 — `s?` with NULL is NOT an error: json_null() fills the slot.
        let f = cs("s?");
        let o = epk!(c, r, 0, f, [std::ptr::null::<c_char>()], "row 236");
        assert!(!o.null, "C: pack(\"s?\", NULL) must succeed");
        assert_eq!(o.tree.as_deref(), Some(&b"null"[..]));
        for fmt in ["[s?]", "{s:s?}"] {
            let k = cs("k");
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [k.as_ptr(), std::ptr::null::<c_char>()], "row 236 {fmt}");
            assert!(!o.null, "C: {fmt} must succeed");
        }

        // Row 237 — `s*` with NULL at TOP LEVEL returns NULL with **no error
        // recorded**. jsonp_error_init has cleared text[0]/line/column/
        // position/source[0] but nothing else, so `json_error_code()` reads
        // text[159], which is still whatever the caller left there —
        // uninitialised memory. We therefore compare the return value and the
        // FULL byte image the two libraries wrote (which must agree), but
        // deliberately DO NOT assert a specific code byte.
        let f = cs("s*");
        let o = epk!(c, r, 0, f, [std::ptr::null::<c_char>()], "row 237");
        assert!(o.null, "C: pack(\"s*\", NULL) must return NULL");
        assert_eq!(o.text(), "", "C: text[0] was cleared, nothing written");
        assert_eq!(o.source(), "", "C: source[0] was cleared");
        assert_eq!(o.lcp(), (-1, -1, 0), "C: only jsonp_error_init ran");
        // The code byte is 0x7f here purely because json_error_t::poisoned()
        // put it there; that is the point of the row, not an assertion.
        assert_eq!(o.code(), 0x7f, "C: json_error_code reads the caller's byte");
    }
}

#[test]
fn row_238_pack_length_modifier_on_an_optional_string() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        for opt in [b'?', b'*'] {
            for m in [b'#', b'%', b'+'] {
                let f = cs_bytes(&[b's', opt, m]);
                let o = epk!(
                    c, r, 0, f,
                    [a.as_ptr(), 1 as c_int, a.as_ptr()],
                    "row 238 s{}{}", opt as char, m as char
                );
                assert!(o.null, "C: s{}{} must fail", opt as char, m as char);
                assert_eq!(o.source(), "<format>", "C: source");
                assert_eq!(
                    o.text(),
                    format!("Cannot use '{}' on optional strings", m as char),
                    "C: text for s{}{}", opt as char, m as char
                );
                assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
                assert_eq!(o.lcp(), (1, 2, 2), "C: the '?'/'*' token position");
            }
        }
        // Inside containers too.
        let k = cs("k");
        for fmt in ["[s?#]", "{s:s*+}"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [k.as_ptr(), a.as_ptr(), 1 as c_int], "row 238 {fmt}");
            assert!(o.null, "C: {fmt} must fail");
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
        }
    }
}

// ===========================================================================
// Rows 240 / 241 / 243 — the '#'/'%'/'+' concatenation path
// ===========================================================================

#[test]
fn rows_240_241_243_pack_concatenation_errors() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let ccs = cs("c");

        // Row 240 — a NULL anywhere in a '+' chain. The error is set on the
        // iteration that saw the NULL, so the column identifies that part.
        let f = cs("s++");
        let o = epk!(
            c, r, 0, f,
            [a.as_ptr(), std::ptr::null::<c_char>(), ccs.as_ptr()],
            "row 240 middle NULL"
        );
        assert!(o.null, "C: a NULL in a '+' chain must fail");
        assert_eq!(o.source(), "<args>", "C: source");
        assert_eq!(o.text(), "NULL string", "C: text");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
        assert_eq!(o.lcp(), (1, 2, 2), "C: column 2");

        let f = cs("s++");
        let o = epk!(
            c, r, 0, f,
            [std::ptr::null::<c_char>(), a.as_ptr(), ccs.as_ptr()],
            "row 240 first NULL"
        );
        assert!(o.null);
        assert_eq!(o.text(), "NULL string");
        let f = cs("s++");
        let o = epk!(
            c, r, 0, f,
            [a.as_ptr(), ccs.as_ptr(), std::ptr::null::<c_char>()],
            "row 240 last NULL"
        );
        assert!(o.null);
        assert_eq!(o.text(), "NULL string");

        // Row 241 — '#'/'%' with a NULL string pointer.
        let f = cs("s#");
        let o = epk!(c, r, 0, f, [std::ptr::null::<c_char>(), 3 as c_int], "row 241 s#");
        assert!(o.null, "C: s# with a NULL pointer must fail");
        assert_eq!(o.text(), "NULL string");
        assert_eq!(o.source(), "<args>");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE);
        assert_eq!(o.lcp(), (1, 1, 1));

        let f = cs("s%");
        let o = epk!(c, r, 0, f, [std::ptr::null::<c_char>(), 3 as size_t], "row 241 s%");
        assert!(o.null);
        assert_eq!(o.text(), "NULL string");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE);

        // A NULL with '+' *and* an explicit length on the same part, so the
        // `length = s->has_error ? 0 : strlen(str)` branch is exercised.
        let f = cs("s#+#");
        let o = epk!(
            c, r, 0, f,
            [std::ptr::null::<c_char>(), 3 as c_int, a.as_ptr(), 1 as c_int],
            "row 241 s#+#"
        );
        assert!(o.null);
        assert_eq!(o.text(), "NULL string");

        // Row 243 — the concatenated result is not valid UTF-8. The check runs
        // on the assembled buffer, so a sequence split across two parts that
        // does NOT reassemble into valid UTF-8 must be rejected.
        let bad = cs_bytes(b"\xff");
        let f = cs("s#");
        let o = epk!(c, r, 0, f, [bad.as_ptr(), 1 as c_int], "row 243 single part");
        assert!(o.null, "C: an invalid concatenation must fail");
        assert_eq!(o.source(), "<args>");
        assert_eq!(o.text(), "Invalid UTF-8 string");
        assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8, "C: code 5");

        let half1 = cs_bytes(b"\xc3");
        let half2 = cs_bytes(b"\xa9");
        // Control: two halves that DO reassemble into U+00E9 must succeed.
        let f = cs("s++");
        let o = epk!(
            c, r, 0, f,
            [half1.as_ptr(), half2.as_ptr(), a.as_ptr()],
            "row 243 control: valid across parts"
        );
        assert!(!o.null, "C: \\xc3 + \\xa9 reassembles into valid UTF-8");
        // ... and two halves that do not.
        let f = cs("s++");
        let o = epk!(
            c, r, 0, f,
            [half1.as_ptr(), half1.as_ptr(), a.as_ptr()],
            "row 243 invalid across parts"
        );
        assert!(o.null, "C: \\xc3\\xc3 is not valid UTF-8");
        assert_eq!(o.text(), "Invalid UTF-8 string");
        assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8);

        // An object KEY through the same path, so `purpose` differs.
        let f = cs("{s#:i}");
        let o = epk!(c, r, 0, f, [bad.as_ptr(), 1 as c_int, 1 as c_int], "row 243 key");
        assert!(o.null);
        assert_eq!(o.text(), "Invalid UTF-8 object key");
        assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8);
    }
}

// ===========================================================================
// Rows 244 / 245 / 246 — 'o' and 'O' with a NULL json_t*
// ===========================================================================

#[test]
fn rows_244_245_246_pack_o_and_O_with_null() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Rows 244/245 — no ?/* modifier -> "NULL object".
        for fmt in ["O", "o"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [std::ptr::null_mut::<json_t>()], "rows 244/245 {fmt}");
            assert!(o.null, "C: {fmt} with NULL must fail");
            assert_eq!(o.source(), "<args>", "C: source for {fmt}");
            assert_eq!(o.text(), "NULL object", "C: text for {fmt}");
            assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12 for {fmt}");
            assert_eq!(o.lcp(), (1, 1, 1), "C: position for {fmt}");
        }
        // `o?`/`O?` substitute json_null() instead.
        for fmt in ["O?", "o?"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [std::ptr::null_mut::<json_t>()], "row 246 {fmt}");
            assert!(!o.null, "C: {fmt} with NULL must succeed");
            assert_eq!(o.tree.as_deref(), Some(&b"null"[..]));
        }

        // Row 246 — `o*`/`O*` with NULL at TOP LEVEL: NULL return, **no error
        // recorded**. As with row 237, `json_error_code()` would read
        // uninitialised memory, so no code byte is asserted; only the return
        // value and the exact bytes both libraries wrote.
        for fmt in ["o*", "O*"] {
            let f = cs(fmt);
            let o = epk!(c, r, 0, f, [std::ptr::null_mut::<json_t>()], "row 246 {fmt}");
            assert!(o.null, "C: {fmt} at top level returns NULL");
            assert_eq!(o.text(), "", "C: nothing was written into text");
            assert_eq!(o.source(), "", "C: nothing was written into source");
            assert_eq!(o.lcp(), (-1, -1, 0), "C: only jsonp_error_init ran");
        }
        // ... but inside a container the slot is simply omitted and the pack
        // succeeds.
        let k = cs("k");
        for fmt in ["[o*]", "[O*]"] {
            let f = cs(fmt);
            let o = epk!(
                c, r, 0, f, [std::ptr::null_mut::<json_t>()],
                "row 246 nested {fmt}"
            );
            assert!(!o.null, "C: {fmt} must succeed");
            assert_eq!(o.tree.as_deref(), Some(&b"[]"[..]), "C: tree for {fmt}");
        }
        for fmt in ["{s:o*}", "{s:O*}"] {
            let f = cs(fmt);
            let o = epk!(
                c, r, 0, f, [k.as_ptr(), std::ptr::null_mut::<json_t>()],
                "row 246 nested {fmt}"
            );
            assert!(!o.null, "C: {fmt} must succeed");
            assert_eq!(o.tree.as_deref(), Some(&b"{}"[..]), "C: tree for {fmt}");
        }
        // A NULL 'o*' between two real elements: the slot is dropped and the
        // surrounding elements keep their order.
        let f = cs("[i,o*,i]");
        let o = epk!(
            c, r, 0, f,
            [1 as c_int, std::ptr::null_mut::<json_t>(), 2 as c_int],
            "row 246 [i,o*,i]"
        );
        assert!(!o.null, "C: [i,o*,i] must succeed");
        assert_eq!(o.tree.as_deref(), Some(&b"[1, 2]"[..]), "C: the slot is omitted");
    }
}

// ===========================================================================
// Row 249 — 'f' with a non-finite double
// ===========================================================================

#[test]
fn row_249_pack_non_finite_real() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("k");
        for v in [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(0x7ff8_0000_0000_0001), // another quiet NaN
            f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        ] {
            let f = cs("f");
            let o = epk!(c, r, 0, f, [v], "row 249 {v:?}");
            assert!(o.null, "C: pack(\"f\", {v:?}) must fail");
            assert_eq!(o.source(), "<args>", "C: source");
            assert_eq!(o.text(), "Invalid floating point value", "C: text");
            assert_eq!(o.code(), JSON_ERROR_NUMERIC_OVERFLOW, "C: code 15");
            assert_eq!(o.lcp(), (1, 1, 1), "C: position");

            // Nested, so the container unwind runs (and json_real(0.0) has
            // already been allocated and must be decref'd).
            for fmt in ["[f]", "{s:f}", "{s:[i,f]}"] {
                let ff = cs(fmt);
                let o = epk!(c, r, 0, ff, [k.as_ptr(), 1 as c_int, v], "row 249 {fmt} {v:?}");
                assert!(o.null, "C: {fmt} with {v:?} must fail");
                assert_eq!(o.code(), JSON_ERROR_NUMERIC_OVERFLOW, "C: code for {fmt}");
            }
        }
    }
}

// ===========================================================================
// Row 250 — the format ends right after a key
// ===========================================================================

/// `"{s"` leaves `pack()` looking at the terminating NUL, so the message is
/// formatted with `%c` and a 0 argument: the NUL is written INTO `error->text`,
/// which is why the C string appears to be cut short right after the opening
/// quote. Then `pack_object` sets (and `jsonp_error_vset` drops) "NULL object
/// value", whose `jsonp_error_set_source("<args>")` DOES take effect, and the
/// loop's next iteration sets (and drops) "Unexpected end of format string",
/// which puts the source back to `"<format>"`. Only the raw byte image pins
/// all of that; `epk!` compares it.
#[test]
fn row_250_pack_format_ends_after_a_key() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("k");
        let f = cs("{s");
        let o = epk!(c, r, 0, f, [k.as_ptr()], "row 250");
        assert!(o.null, "C: \"{{s\" must fail");
        assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
        assert_eq!(
            o.text(),
            "Unexpected format character '",
            "C: the %c wrote a NUL into the message"
        );
        assert_eq!(o.source(), "<format>", "C: the LAST set_error's source wins");
        assert_eq!(o.lcp(), (1, 3, 3), "C: the NUL token's position");
        // The exact tail of the text buffer, proving the embedded NUL and the
        // closing quote after it.
        let text_bytes: Vec<u8> = o.raw[12 + JSON_ERROR_SOURCE_LENGTH..].to_vec();
        assert_eq!(
            &text_bytes[..31],
            b"Unexpected format character '\0'",
            "C: the message contains an embedded NUL then a quote"
        );
        // The same shape one level down.
        let f = cs("{s:{s");
        let o = epk!(c, r, 0, f, [k.as_ptr(), k.as_ptr()], "row 250 nested");
        assert!(o.null);
        assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
        assert_eq!(o.text(), "Unexpected format character '");
    }
}

// ===========================================================================
// The failing allocator, for the out-of-memory rows
// ===========================================================================

// These hooks are *interchangeable with the defaults* (they just forward to
// libc), so installing them once and leaving them installed cannot disturb any
// other test in this binary. The only behavioural change is driven by
// THREAD-LOCAL switches, and every test here holds `global_state_lock()`.
extern "C" {
    fn malloc(n: size_t) -> *mut c_void;
    fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
}

thread_local! {
    /// Fail any allocation of exactly this size.
    static FAIL_SIZE: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
    /// Allow this many allocations, then fail everything. `-1` = unlimited.
    static BUDGET: std::cell::Cell<isize> = const { std::cell::Cell::new(-1) };
}

fn deny(n: size_t) -> bool {
    if n == FAIL_SIZE.with(|f| f.get()) {
        return true;
    }
    let b = BUDGET.with(|b| b.get());
    if b == 0 {
        return true;
    }
    if b > 0 {
        BUDGET.with(|x| x.set(b - 1));
    }
    false
}

unsafe extern "C" fn hook_malloc(n: size_t) -> *mut c_void {
    if deny(n) {
        return std::ptr::null_mut();
    }
    malloc(n)
}
unsafe extern "C" fn hook_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    if deny(n) {
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

fn clear_switches() {
    FAIL_SIZE.with(|f| f.set(usize::MAX));
    BUDGET.with(|b| b.set(-1));
}

// Sizes the C allocates on the paths below, computed from the structure
// definitions so the failure can be aimed at exactly one call site:
//
//   strbuffer_init            STRBUFFER_MIN_SIZE                      = 16
//   strbuffer first grow      max(16*2, length+size+1)                = 32
//   hashtable_init            hashsize(3) * sizeof(bucket_t) = 8 * 16 = 128
//   init_pair                 offsetof(pair_t, key) + key_len + 1     = 57 + key_len
//   json_integer / json_real  sizeof(json_t) + 8                      = 24
//   json_array table grow     16 * sizeof(json_t *)                   = 128
const SZ_STRBUFFER_INIT: usize = 16;
const SZ_STRBUFFER_GROW: usize = 32;
const SZ_HASHTABLE_INIT: usize = 128;
const SZ_SCALAR: usize = 24;
const SZ_ARRAY_GROW: usize = 128;
fn sz_pair(key_len: usize) -> usize {
    56 + key_len + 1
}

// ===========================================================================
// Rows 230 / 233 / 239 / 242 / 247 / 248 — pack's out-of-memory paths
// ===========================================================================

#[test]
fn rows_230_233_239_242_247_248_pack_out_of_memory() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        install_hooks(c, r);
        let key = cs("kkk"); // len 3 -> init_pair asks for 60 bytes
        let a = cs("aa");
        let b = cs("bb");

        macro_rules! oom {
            ($size:expr, $flags:expr, $fmt:expr, [$($arg:expr),* $(,)?], $($ctx:tt)*) => {{
                FAIL_SIZE.with(|s| s.set($size));
                let o = epk!(c, r, $flags, $fmt, [$($arg),*], $($ctx)*);
                clear_switches();
                o
            }};
        }

        // Assert the "<internal>" / "Out of memory" triple on a PObs.
        macro_rules! want_oom {
            ($o:expr, $tag:expr) => {{
                let o_: &PObs = &$o;
                assert!(o_.null, "C: {} must fail on OOM", $tag);
                assert_eq!(o_.source(), "<internal>", "C: source for {}", $tag);
                assert_eq!(o_.text(), "Out of memory", "C: text for {}", $tag);
                assert_eq!(
                    o_.code(),
                    JSON_ERROR_OUT_OF_MEMORY,
                    "C: code 1 for {}",
                    $tag
                );
            }};
        }

        // ---- Row 239: strbuffer_init fails in the '#'/'%'/'+' path. Each
        // format gets exactly the varargs it consumes, so a bogus length can
        // never reach memcpy.
        let f = cs("s#");
        want_oom!(
            oom!(SZ_STRBUFFER_INIT, 0, f, [a.as_ptr(), 2 as c_int], "row 239 s#"),
            "s#"
        );
        let f = cs("s%");
        want_oom!(
            oom!(SZ_STRBUFFER_INIT, 0, f, [a.as_ptr(), 2 as size_t], "row 239 s%"),
            "s%"
        );
        let f = cs("s+");
        want_oom!(
            oom!(SZ_STRBUFFER_INIT, 0, f, [a.as_ptr(), b.as_ptr()], "row 239 s+"),
            "s+"
        );
        let f = cs("{s#:i}");
        want_oom!(
            oom!(SZ_STRBUFFER_INIT, 0, f, [a.as_ptr(), 2 as c_int, 1 as c_int],
                 "row 239 {{s#:i}}"),
            "{s#:i}"
        );
        let f = cs("{s+:i}");
        want_oom!(
            oom!(SZ_STRBUFFER_INIT, 0, f, [a.as_ptr(), b.as_ptr(), 1 as c_int],
                 "row 239 {{s+:i}}"),
            "{s+:i}"
        );
        let f = cs("[s#]");
        want_oom!(
            oom!(SZ_STRBUFFER_INIT, 0, f, [a.as_ptr(), 2 as c_int], "row 239 [s#]"),
            "[s#]"
        );

        // ---- Row 242: strbuffer_append_bytes fails (the first grow).
        // 20 bytes into a 16-byte buffer forces a realloc to 32.
        let long = cs("aaaaaaaaaaaaaaaaaaaa"); // 20 chars
        let f = cs("s#");
        want_oom!(
            oom!(SZ_STRBUFFER_GROW, 0, f, [long.as_ptr(), 20 as c_int], "row 242 s#"),
            "row 242 s#"
        );
        let f = cs("s+");
        want_oom!(
            oom!(SZ_STRBUFFER_GROW, 0, f, [long.as_ptr(), long.as_ptr()], "row 242 s+"),
            "row 242 s+"
        );
        let f = cs("{s#:i}");
        want_oom!(
            oom!(SZ_STRBUFFER_GROW, 0, f, [long.as_ptr(), 20 as c_int, 1 as c_int],
                 "row 242 {{s#:i}}"),
            "row 242 {s#:i}"
        );

        // ---- Rows 247/248: json_integer / json_real allocation fails.
        for (fmt, args_kind) in [("i", 0), ("I", 1), ("f", 2)] {
            let f = cs(fmt);
            let o = match args_kind {
                0 => oom!(SZ_SCALAR, 0, f, [7 as c_int], "row 247 {fmt}"),
                1 => oom!(SZ_SCALAR, 0, f, [7i64 as json_int_t], "row 247 {fmt}"),
                _ => oom!(SZ_SCALAR, 0, f, [7.5f64], "row 248 {fmt}"),
            };
            assert!(o.null, "C: {fmt} must fail when the scalar cannot be allocated");
            assert_eq!(o.source(), "<internal>", "C: source for {fmt}");
            assert_eq!(o.text(), "Out of memory", "C: text for {fmt}");
            assert_eq!(o.code(), JSON_ERROR_OUT_OF_MEMORY, "C: code 1 for {fmt}");
        }

        // ---- Row 230: json_object_setn_new_nocheck fails.
        // Fail exactly the pair allocation for a 3-byte key, so json_object()
        // and json_integer() both still succeed.
        let f = cs("{s:n}");
        let o = oom!(sz_pair(3), 0, f, [key.as_ptr()], "row 230");
        assert!(o.null, "C: the object insert must fail");
        assert_eq!(o.source(), "<internal>", "C: source");
        assert_eq!(
            o.text(),
            "Unable to add key \"kkk\"",
            "C: text names the key"
        );
        assert_eq!(o.code(), JSON_ERROR_OUT_OF_MEMORY, "C: code 1");
        // ... and through the `s#` path, where the key is `ours` and must be
        // jsonp_free'd after the message has been formatted from it.
        let f = cs("{s#:n}");
        let o = oom!(sz_pair(3), 0, f, [key.as_ptr(), 3 as c_int], "row 230 s# key");
        assert!(o.null);
        assert_eq!(o.text(), "Unable to add key \"kkk\"");
        assert_eq!(o.code(), JSON_ERROR_OUT_OF_MEMORY);

        // ---- Row 233: json_array_append_new fails. A fresh array holds 8
        // entries, so the 9th append is the first that reallocs (to 16 * 8).
        let f = cs("[n,n,n,n,n,n,n,n,n]");
        let o = oom!(SZ_ARRAY_GROW, 0, f, [], "row 233");
        assert!(o.null, "C: the 9th array append must fail");
        assert_eq!(o.source(), "<internal>", "C: source");
        assert_eq!(o.text(), "Unable to append to array", "C: text");
        assert_eq!(o.code(), JSON_ERROR_OUT_OF_MEMORY, "C: code 1");

        // ---- A budget sweep over a format that touches every allocating
        // path, so the two libraries are also shown to make the SAME NUMBER of
        // allocations in the same order. Anything the sweep reaches is
        // compared; the assertions above pin the individual messages.
        // The budget must be re-armed before EACH library's call (it is a
        // counter, not a filter) and cleared again before anything else — in
        // particular before json_dumps, which allocates too.
        let f = cs("{s:[i,f,s,s#],s:{s:n}}");
        let mut seen_oom = false;
        for budget in 0..24isize {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.with(|x| x.set(budget));
            let cj = (c.json_pack_ex)(
                &mut ce, 0, f.as_ptr(),
                key.as_ptr(), 1 as c_int, 2.5f64, a.as_ptr(), b.as_ptr(), 2 as c_int,
                a.as_ptr(), b.as_ptr(),
            );
            BUDGET.with(|x| x.set(budget));
            let rj = (r.json_pack_ex)(
                &mut re, 0, f.as_ptr(),
                key.as_ptr(), 1 as c_int, 2.5f64, a.as_ptr(), b.as_ptr(), 2 as c_int,
                a.as_ptr(), b.as_ptr(),
            );
            clear_switches();
            diff_eq!(cj.is_null(), rj.is_null(), "pack budget {budget} NULL-ness");
            diff_eq!(ce.snapshot(), re.snapshot(), "pack budget {budget} error");
            diff_eq!(ce.raw(), re.raw(), "pack budget {budget} raw error");
            diff_eq!(canon(c, cj), canon(r, rj), "pack budget {budget} tree");
            if cj.is_null() && ce.code() == JSON_ERROR_OUT_OF_MEMORY {
                seen_oom = true;
            }
            decref(c, cj);
            decref(r, rj);
        }
        assert!(
            seen_oom,
            "the budget sweep never reached an out-of-memory error — the test would be vacuous"
        );
        clear_switches();
    }
}

// ===========================================================================
// Rows 251 / 252 / 253 — NULL root, NULL and empty unpack format
// ===========================================================================

#[test]
fn rows_251_252_253_unpack_null_root_and_bad_format() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let empty = cs("");
        let good = cs("i");
        let mut rng = Rng::new(0x14_0251);
        let mut flag_words: Vec<size_t> = vec![
            0,
            JSON_VALIDATE_ONLY,
            JSON_STRICT,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            0x4,
            0x8,
            0x10,
            0x20,
            0x8000,
            1usize << 63,
            size_t::MAX,
        ];
        for _ in 0..24 {
            flag_words.push(rng.next_u64() as size_t);
        }

        for &flags in &flag_words {
            // Row 251 — root == NULL. Checked BEFORE the format, so even a
            // NULL format reports "NULL root value".
            for (fmt, tag) in [
                (good.as_ptr(), "\"i\""),
                (std::ptr::null::<c_char>(), "NULL"),
                (empty.as_ptr(), "\"\""),
            ] {
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let mut cs_ = Slots::poisoned();
                let mut rs_ = Slots::poisoned();
                let cret = (c.json_unpack_ex)(
                    std::ptr::null_mut(),
                    &mut ce,
                    flags,
                    fmt,
                    ip(&mut cs_, 0),
                );
                let rret = (r.json_unpack_ex)(
                    std::ptr::null_mut(),
                    &mut re,
                    flags,
                    fmt,
                    ip(&mut rs_, 0),
                );
                diff_eq!(cret, rret, "row 251 return fmt={tag} flags={flags:#x}");
                diff_eq!(ce.raw(), re.raw(), "row 251 raw error fmt={tag}");
                diff_eq!(
                    cs_.summary(c),
                    rs_.summary(r),
                    "row 251 out-pointers fmt={tag}"
                );
                assert_eq!(cret, -1, "C: a NULL root must fail");
                assert_eq!(ce.source_str(), "<root>", "C: source");
                assert_eq!(ce.text_str(), "NULL root value", "C: text");
                assert_eq!(
                    (ce.line, ce.column, ce.position),
                    (-1, -1, 0),
                    "C: line/column/position"
                );
                assert_eq!(ce.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
            }
        }

        // Rows 252/253 — NULL and empty format with a valid root.
        for &flags in &flag_words {
            for (fmt, tag) in [(std::ptr::null::<c_char>(), "NULL"), (empty.as_ptr(), "\"\"")] {
                let croot = load(c, "1");
                let rroot = load(r, "1");
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cret = (c.json_unpack_ex)(croot, &mut ce, flags, fmt);
                let rret = (r.json_unpack_ex)(rroot, &mut re, flags, fmt);
                diff_eq!(cret, rret, "rows 252/253 return fmt={tag} flags={flags:#x}");
                diff_eq!(ce.raw(), re.raw(), "rows 252/253 raw error fmt={tag}");
                assert_eq!(cret, -1, "C: a {tag} format must fail");
                assert_eq!(ce.source_str(), "<format>", "C: source");
                assert_eq!(ce.text_str(), "NULL or empty format string", "C: text");
                assert_eq!((ce.line, ce.column, ce.position), (-1, -1, 0));
                assert_eq!(ce.code(), JSON_ERROR_INVALID_ARGUMENT, "C: code 4");
                decref(c, croot);
                decref(r, rroot);
            }
        }

        // A NULL json_error_t* must be tolerated on all of these.
        let croot = load(c, "1");
        let rroot = load(r, "1");
        for fmt in [std::ptr::null::<c_char>(), empty.as_ptr(), good.as_ptr()] {
            let cret = (c.json_unpack_ex)(std::ptr::null_mut(), std::ptr::null_mut(), 0, fmt);
            let rret = (r.json_unpack_ex)(std::ptr::null_mut(), std::ptr::null_mut(), 0, fmt);
            diff_eq!(cret, rret, "NULL root + NULL error");
        }
        for fmt in [std::ptr::null::<c_char>(), empty.as_ptr()] {
            let cret = (c.json_unpack_ex)(croot, std::ptr::null_mut(), 0, fmt);
            let rret = (r.json_unpack_ex)(rroot, std::ptr::null_mut(), 0, fmt);
            diff_eq!(cret, rret, "bad fmt + NULL error");
            let cret = (c.json_unpack)(croot, fmt);
            let rret = (r.json_unpack)(rroot, fmt);
            diff_eq!(cret, rret, "json_unpack bad fmt");
        }
        let cret = (c.json_unpack)(std::ptr::null_mut(), good.as_ptr());
        let rret = (r.json_unpack)(std::ptr::null_mut(), good.as_ptr());
        diff_eq!(cret, rret, "json_unpack NULL root");
        decref(c, croot);
        decref(r, rroot);

        // ... and through the real va_list entry point.
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vunpack_ex");
        let rfn = sym_addr("Rust", b"json_vunpack_ex");
        let croot = load(c, "1");
        let rroot = load(r, "1");
        for &flags in &flag_words {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cret = (sh.vunpack_ex)(cfn, std::ptr::null_mut(), &mut ce, flags, good.as_ptr());
            let rret = (sh.vunpack_ex)(rfn, std::ptr::null_mut(), &mut re, flags, good.as_ptr());
            diff_eq!(cret, rret, "vunpack_ex NULL root return");
            diff_eq!(ce.raw(), re.raw(), "vunpack_ex NULL root raw error");
            assert_eq!(cret, -1);
            for fmt in [std::ptr::null::<c_char>(), empty.as_ptr()] {
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cret = (sh.vunpack_ex)(cfn, croot, &mut ce, flags, fmt);
                let rret = (sh.vunpack_ex)(rfn, rroot, &mut re, flags, fmt);
                diff_eq!(cret, rret, "vunpack_ex bad fmt return");
                diff_eq!(ce.raw(), re.raw(), "vunpack_ex bad fmt raw error");
                assert_eq!(cret, -1);
                let cret = (sh.vunpack_ex)(cfn, croot, std::ptr::null_mut(), flags, fmt);
                let rret = (sh.vunpack_ex)(rfn, rroot, std::ptr::null_mut(), flags, fmt);
                diff_eq!(cret, rret, "vunpack_ex NULL error tolerated");
            }
        }
        decref(c, croot);
        decref(r, rroot);
    }
}

// ===========================================================================
// Rows 254 / 255 — garbage after the format, unrecognised format character
// ===========================================================================

#[test]
fn rows_254_255_unpack_garbage_and_unexpected_format_character() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 254 — tokens left over after a complete value.
        let cases: &[(&str, &str, c_int, c_int)] = &[
            ("{\"a\":1}", "{}}", 3, 3),
            ("[1,2]", "[]]", 3, 3),
            ("[1,2]", "[i]i", 4, 4),
            ("1", "ii", 2, 2),
            ("1", "iq", 2, 2),
            // NOTE: "nn" is NOT in this list. On an integer root the FIRST 'n'
            // already fails the type check ("Expected null, got integer"), so
            // the format is never fully consumed and row 254 is not reached.
        ];
        for &(root, fmt, col, pos) in cases {
            let f = cs(fmt);
            let o = eupk!(
                c, r, root, 0, f, sl,
                [ip(sl, 0), ip(sl, 1)],
                &[],
                "row 254 root={root} fmt={fmt:?}"
            );
            assert_eq!(o.ret, -1, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<format>", "C: source for {fmt:?}");
            assert_eq!(o.text(), "Garbage after format string", "C: text {fmt:?}");
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            assert_eq!(o.lcp(), (1, col, pos), "C: position");
        }

        // Row 255 — `unpack()`'s recognised characters are exactly
        // `{ [ s i I b f F O o n`. Every other byte lands in the `default:`
        // arm, which consumes no vararg.
        const UNPACK_CHARS: &[u8] = b"{[siIbfFOon";
        for b in 1u8..=255 {
            if UNPACK_CHARS.contains(&b) {
                continue;
            }
            let f = cs_bytes(&[b]);
            let o = eupk!(c, r, "1", 0, f, sl, [], &[], "row 255 byte {:?}", show(&[b]));
            assert_eq!(o.ret, -1, "C: byte {:?} must fail", show(&[b]));
            assert_eq!(o.source(), "<format>", "C: source for {:?}", show(&[b]));
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            if matches!(b, b' ' | b'\t' | b'\n' | b',' | b':') {
                // skipped -> the token becomes the terminating NUL, and `%c`
                // writes that NUL into the message.
                assert_eq!(o.text(), "Unexpected format character '");
            } else if b < 0x80 {
                assert_eq!(
                    o.text(),
                    format!("Unexpected format character '{}'", b as char),
                    "C: text for {:?}", show(&[b])
                );
            }
        }
    }
}

// ===========================================================================
// Rows 256 / 259-264 / 266 / 275 — the type-mismatch matrix
// ===========================================================================

/// Every unpack conversion character against every one of the eight json
/// types, with the exact `"Expected <what>, got <type>"` message from
/// `type_names[]`.
#[test]
fn rows_256_264_266_275_unpack_type_mismatch_matrix() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // (root text, the type_names[] entry the C must print)
        let roots: &[(&str, &str)] = &[
            ("{\"a\":1}", "object"),
            ("[1]", "array"),
            ("\"x\"", "string"),
            ("1", "integer"),
            ("1.5", "real"),
            ("true", "true"),
            ("false", "false"),
            ("null", "null"),
        ];
        for &(root, tname) in roots {
            // 's' -> "Expected string, got T"
            let f = cs("s");
            let o = eupk!(c, r, root, 0, f, sl, [sp(sl, 0)], &[], "row 256 s on {tname}");
            if tname == "string" {
                assert_eq!(o.ret, 0, "C: s on a string must succeed");
            } else {
                assert_eq!(o.ret, -1, "C: s on {tname} must fail");
                assert_eq!(o.source(), "<validation>", "C: source");
                assert_eq!(o.text(), format!("Expected string, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE, "C: code 10");
                assert_eq!(o.lcp(), (1, 1, 1));
            }

            // 's%' — the length target is only read AFTER the type check.
            let f = cs("s%");
            let o = eupk!(c, r, root, 0, f, sl, [sp(sl, 0), lp(sl, 0)], &[],
                          "row 256 s% on {tname}");
            if tname != "string" {
                assert_eq!(o.ret, -1);
                assert_eq!(o.text(), format!("Expected string, got {tname}"));
                assert_eq!(
                    o.slots.lens[0], POISON_LEN,
                    "C: the length target must be untouched on the error path"
                );
            }

            // 'i' and 'I' -> "Expected integer, got T"
            for (fmt, ptr_is_i64) in [("i", false), ("I", true)] {
                let f = cs(fmt);
                let o = if ptr_is_i64 {
                    eupk!(c, r, root, 0, f, sl, [i64p(sl, 0)], &[], "rows 259/260 {fmt} on {tname}")
                } else {
                    eupk!(c, r, root, 0, f, sl, [ip(sl, 0)], &[], "rows 259/260 {fmt} on {tname}")
                };
                if tname == "integer" {
                    assert_eq!(o.ret, 0, "C: {fmt} on an integer must succeed");
                } else {
                    assert_eq!(o.ret, -1, "C: {fmt} on {tname} must fail");
                    assert_eq!(o.text(), format!("Expected integer, got {tname}"));
                    assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
                    assert_eq!(o.source(), "<validation>");
                }
            }

            // 'b' -> "Expected true or false, got T"
            let f = cs("b");
            let o = eupk!(c, r, root, 0, f, sl, [ip(sl, 0)], &[], "row 261 b on {tname}");
            if tname == "true" || tname == "false" {
                assert_eq!(o.ret, 0, "C: b on {tname} must succeed");
            } else {
                assert_eq!(o.ret, -1, "C: b on {tname} must fail");
                assert_eq!(o.text(), format!("Expected true or false, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
            }

            // 'f' -> "Expected real, got T" (an integer is rejected!)
            let f = cs("f");
            let o = eupk!(c, r, root, 0, f, sl, [dp(sl, 0)], &[], "row 262 f on {tname}");
            if tname == "real" {
                assert_eq!(o.ret, 0, "C: f on a real must succeed");
            } else {
                assert_eq!(o.ret, -1, "C: f on {tname} must fail");
                assert_eq!(o.text(), format!("Expected real, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
            }

            // 'F' -> "Expected real or integer, got T"
            let f = cs("F");
            let o = eupk!(c, r, root, 0, f, sl, [dp(sl, 0)], &[], "row 263 F on {tname}");
            if tname == "real" || tname == "integer" {
                assert_eq!(o.ret, 0, "C: F on {tname} must succeed");
            } else {
                assert_eq!(o.ret, -1, "C: F on {tname} must fail");
                assert_eq!(o.text(), format!("Expected real or integer, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
            }

            // 'n' -> "Expected null, got T"; consumes NO vararg either way.
            let f = cs("n");
            let o = eupk!(c, r, root, 0, f, sl, [], &[], "row 264 n on {tname}");
            if tname == "null" {
                assert_eq!(o.ret, 0, "C: n on null must succeed");
            } else {
                assert_eq!(o.ret, -1, "C: n on {tname} must fail");
                assert_eq!(o.text(), format!("Expected null, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
            }

            // '{' -> "Expected object, got T"  (row 266)
            let f = cs("{}");
            let o = eupk!(c, r, root, 0, f, sl, [], &[], "row 266 {{}} on {tname}");
            if tname == "object" {
                assert_eq!(o.ret, 0, "C: {{}} on an object must succeed (non-strict)");
            } else {
                assert_eq!(o.ret, -1, "C: {{}} on {tname} must fail");
                assert_eq!(o.source(), "<validation>");
                assert_eq!(o.text(), format!("Expected object, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
                assert_eq!(o.lcp(), (1, 1, 1));
            }

            // '[' -> "Expected array, got T"  (row 275)
            let f = cs("[]");
            let o = eupk!(c, r, root, 0, f, sl, [], &[], "row 275 [] on {tname}");
            if tname == "array" {
                assert_eq!(o.ret, 0, "C: [] on an array must succeed (non-strict)");
            } else {
                assert_eq!(o.ret, -1, "C: [] on {tname} must fail");
                assert_eq!(o.text(), format!("Expected array, got {tname}"));
                assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
            }

            // 'o'/'O' apply NO type check at all — every type succeeds.
            let f = cs("o");
            let o = eupk!(c, r, root, 0, f, sl, [op(sl, 0)], &[], "o on {tname}");
            assert_eq!(o.ret, 0, "C: o accepts {tname}");
            let f = cs("O");
            let o = eupk!(c, r, root, 0, f, sl, [op(sl, 0)], &[0], "O on {tname}");
            assert_eq!(o.ret, 0, "C: O accepts {tname}");
        }

        // Nested, so the message's line/column reflect the inner token.
        let f = cs("{s:s}");
        let a = cs("a");
        let o = eupk!(
            c, r, "{\"a\":1}", 0, f, sl,
            [a.as_ptr(), sp(sl, 0)], &[],
            "row 256 nested"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Expected string, got integer");
        assert_eq!(o.lcp(), (1, 4, 4), "C: the inner token");
        assert_eq!(
            o.slots.strs[0], None,
            "C: the string target must be untouched"
        );
        // Deeper: the type error inside an array inside an object.
        let f = cs("{s:[s]}");
        let o = eupk!(
            c, r, "{\"a\":[1]}", 0, f, sl,
            [a.as_ptr(), sp(sl, 0)], &[],
            "row 256 deeper"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Expected string, got integer");
        assert_eq!(o.code(), JSON_ERROR_WRONG_TYPE);
    }
}

// ===========================================================================
// Rows 257 / 258 — NULL out-pointers for 's' / 's%'
// ===========================================================================

#[test]
fn rows_257_258_unpack_null_string_targets() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 257 — the `const char **` target is NULL (and JSON_VALIDATE_ONLY
        // is not set, so it IS read).
        let f = cs("s");
        let o = eupk!(
            c, r, "\"x\"", 0, f, sl,
            [std::ptr::null_mut::<*const c_char>()], &[],
            "row 257"
        );
        assert_eq!(o.ret, -1, "C: a NULL string target must fail");
        assert_eq!(o.source(), "<args>", "C: source");
        assert_eq!(o.text(), "NULL string argument", "C: text");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
        assert_eq!(o.lcp(), (1, 1, 1));

        // Under JSON_VALIDATE_ONLY the target is not read at all, so the same
        // call SUCCEEDS — which proves the guard is where the C puts it.
        let f = cs("s");
        let o = eupk!(
            c, r, "\"x\"", JSON_VALIDATE_ONLY, f, sl,
            [std::ptr::null_mut::<*const c_char>()], &[],
            "row 257 under JSON_VALIDATE_ONLY"
        );
        assert_eq!(o.ret, 0, "C: JSON_VALIDATE_ONLY never reads the target");

        // Inside an object, where the key vararg comes first.
        let a = cs("a");
        let f = cs("{s:s}");
        let o = eupk!(
            c, r, "{\"a\":\"x\"}", 0, f, sl,
            [a.as_ptr(), std::ptr::null_mut::<*const c_char>()], &[],
            "row 257 nested"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "NULL string argument");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE);

        // Row 258 — `s%` with a NULL `size_t *`. The string target is read and
        // validated first, so it must be non-NULL here.
        let f = cs("s%");
        let o = eupk!(
            c, r, "\"x\"", 0, f, sl,
            [sp(sl, 0), std::ptr::null_mut::<size_t>()], &[],
            "row 258"
        );
        assert_eq!(o.ret, -1, "C: a NULL length target must fail");
        assert_eq!(o.source(), "<args>", "C: source");
        assert_eq!(o.text(), "NULL string length argument", "C: text");
        assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
        assert_eq!(
            o.slots.strs[0], None,
            "C: *str_target is only written after both targets are validated"
        );

        let f = cs("{s:s%}");
        let o = eupk!(
            c, r, "{\"a\":\"x\"}", 0, f, sl,
            [a.as_ptr(), sp(sl, 0), std::ptr::null_mut::<size_t>()], &[],
            "row 258 nested"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "NULL string length argument");
        // Under JSON_VALIDATE_ONLY the whole `if (!(s->flags & ...))` block is
        // skipped, so the '%' is never consumed as a modifier — it survives to
        // become "Garbage after format string" instead. (Note the asymmetry
        // with plain `s`, which validates cleanly above: the modifier handling
        // lives INSIDE the guarded block.)
        let f = cs("s%");
        let o = eupk!(
            c, r, "\"x\"", JSON_VALIDATE_ONLY, f, sl,
            [sp(sl, 0), std::ptr::null_mut::<size_t>()], &[],
            "row 258 under JSON_VALIDATE_ONLY"
        );
        assert_eq!(o.ret, -1, "C: the unconsumed '%' becomes garbage");
        assert_eq!(o.source(), "<format>", "C: source");
        assert_eq!(o.text(), "Garbage after format string", "C: text");
        assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
        assert_eq!(
            o.slots.strs[0], None,
            "C: no target is written under JSON_VALIDATE_ONLY"
        );
    }
}

// ===========================================================================
// Rows 269 / 270 / 271 — object key errors
// ===========================================================================

#[test]
fn rows_269_270_271_unpack_object_key_errors() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 269 — the key position is not 's'/'!'/'*'/'}'. The check comes
        // after the '!'/'*' handling, so sweep every byte.
        for b in 1u8..=255 {
            if matches!(b, b's' | b'!' | b'*' | b'}') {
                continue;
            }
            let f = cs_bytes(&[b'{', b, b':', b'i', b'}']);
            let o = eupk!(
                c, r, "{\"a\":1}", 0, f, sl, [ip(sl, 0)], &[],
                "row 269 key byte {:?}", show(&[b])
            );
            assert_eq!(o.ret, -1, "C: key byte {:?} must fail", show(&[b]));
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            assert_eq!(o.source(), "<format>", "C: source");
            if matches!(b, b' ' | b'\t' | b'\n' | b',' | b':') {
                assert_eq!(o.text(), "Expected format 's', got 'i'");
            } else if b < 0x80 {
                assert_eq!(
                    o.text(),
                    format!("Expected format 's', got '{}'", b as char),
                    "C: text for {:?}", show(&[b])
                );
            }
        }
        let f = cs("{i:i}");
        let o = eupk!(c, r, "{\"a\":1}", 0, f, sl, [ip(sl, 0)], &[], "row 269 {{i:i}}");
        assert_eq!(o.text(), "Expected format 's', got 'i'");
        assert_eq!(o.lcp(), (1, 2, 2), "C: column 2");

        // Row 270 — the key argument is NULL.
        for fmt in ["{s:i}", "{s?i}", "{s:s}", "{s:n}"] {
            let f = cs(fmt);
            let o = eupk!(
                c, r, "{\"a\":1}", 0, f, sl,
                [std::ptr::null::<c_char>(), ip(sl, 0)], &[],
                "row 270 {fmt}"
            );
            assert_eq!(o.ret, -1, "C: a NULL key must fail for {fmt}");
            assert_eq!(o.source(), "<args>", "C: source");
            assert_eq!(o.text(), "NULL object key", "C: text");
            assert_eq!(o.code(), JSON_ERROR_NULL_VALUE, "C: code 12");
            assert_eq!(o.lcp(), (1, 2, 2), "C: position");
        }
        // The key vararg is consumed even under JSON_VALIDATE_ONLY, so the
        // NULL check fires there too.
        let f = cs("{s:i}");
        let o = eupk!(
            c, r, "{\"a\":1}", JSON_VALIDATE_ONLY, f, sl,
            [std::ptr::null::<c_char>()], &[],
            "row 270 under JSON_VALIDATE_ONLY"
        );
        assert_eq!(o.ret, -1, "C: keys are read even under JSON_VALIDATE_ONLY");
        assert_eq!(o.text(), "NULL object key");

        // Row 271 — the key is absent from the root and not marked '?'.
        for (key, fmt) in [("zz", "{s:i}"), ("", "{s:i}"), ("A", "{s:i}")] {
            let kk = cs(key);
            let f = cs(fmt);
            let o = eupk!(
                c, r, "{\"a\":1}", 0, f, sl, [kk.as_ptr(), ip(sl, 0)], &[],
                "row 271 key={key:?}"
            );
            assert_eq!(o.ret, -1, "C: a missing key must fail");
            assert_eq!(o.source(), "<validation>", "C: source");
            assert_eq!(o.text(), format!("Object item not found: {key}"), "C: text");
            assert_eq!(o.code(), JSON_ERROR_ITEM_NOT_FOUND, "C: code 16");
            assert_eq!(o.lcp(), (1, 4, 4), "C: position");
            assert_eq!(
                o.slots.ints[0], POISON_I32,
                "C: the target must be untouched"
            );
        }
        // ... and the interaction with '?': the same missing key is fine, the
        // value vararg IS consumed, and the target is left untouched because
        // `value` is NULL.
        let kk = cs("zz");
        let f = cs("{s?i}");
        let o = eupk!(
            c, r, "{\"a\":1}", 0, f, sl, [kk.as_ptr(), ip(sl, 0)], &[],
            "row 271 with '?'"
        );
        assert_eq!(o.ret, 0, "C: '?' makes a missing key acceptable");
        assert_eq!(
            o.slots.ints[0], POISON_I32,
            "C: a skipped optional value writes nothing"
        );
        // A '?' key that IS present still writes.
        let ka = cs("a");
        let f = cs("{s?i}");
        let o = eupk!(
            c, r, "{\"a\":1}", 0, f, sl, [ka.as_ptr(), ip(sl, 0)], &[],
            "row 271 '?' present"
        );
        assert_eq!(o.ret, 0);
        assert_eq!(o.slots.ints[0], 1, "C: the present optional value is written");
        // '?' does not rescue a missing key nested deeper without its own '?'.
        let f = cs("{s?{s:i}}");
        let o = eupk!(
            c, r, "{\"a\":{}}", 0, f, sl, [ka.as_ptr(), kk.as_ptr(), ip(sl, 0)], &[],
            "row 271 nested"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Object item not found: zz");
        assert_eq!(o.code(), JSON_ERROR_ITEM_NOT_FOUND);
    }
}

// ===========================================================================
// Rows 267 / 276 — a token after '!' or '*'
// ===========================================================================

#[test]
fn rows_267_276_unpack_token_after_bang_or_star() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let b = cs("b");

        // Row 267 — inside {}.
        let cases: &[(&str, &str, char, char, c_int)] = &[
            ("{\"a\":1,\"b\":2}", "{s:i!s:i}", '!', 's', 6),
            ("{\"a\":1}", "{*s:i}", '*', 's', 3),
            ("{\"a\":1}", "{!s:i}", '!', 's', 3),
            ("{\"a\":1}", "{s:i!i}", '!', 'i', 6),
            ("{\"a\":1}", "{s:i*i}", '*', 'i', 6),
            ("{\"a\":1}", "{s:i!!}", '!', '!', 6),
            ("{\"a\":1}", "{s:i!*}", '!', '*', 6),
            ("{\"a\":1}", "{s:i*!}", '*', '!', 6),
        ];
        for &(root, fmt, marker, got, col) in cases {
            let f = cs(fmt);
            let o = eupk!(
                c, r, root, 0, f, sl,
                [a.as_ptr(), ip(sl, 0), b.as_ptr(), ip(sl, 1)], &[],
                "row 267 {fmt:?}"
            );
            assert_eq!(o.ret, -1, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<format>", "C: source for {fmt:?}");
            assert_eq!(
                o.text(),
                format!("Expected '}}' after '{marker}', got '{got}'"),
                "C: text for {fmt:?}"
            );
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            assert_eq!(o.lcp(), (1, col, col), "C: position");
        }

        // Row 276 — inside [].
        let cases: &[(&str, &str, char, char, c_int)] = &[
            ("[1,2]", "[i!i]", '!', 'i', 4),
            ("[1,2]", "[i*i]", '*', 'i', 4),
            ("[1,2]", "[*i]", '*', 'i', 3),
            ("[1,2]", "[!i]", '!', 'i', 3),
            ("[1,2]", "[i!s]", '!', 's', 4),
            ("[1,2]", "[i!!]", '!', '!', 4),
            ("[1,2]", "[i!*]", '!', '*', 4),
            ("[1,2]", "[i*!]", '*', '!', 4),
            ("[1,2]", "[i!q]", '!', 'q', 4),
        ];
        for &(root, fmt, marker, got, col) in cases {
            let f = cs(fmt);
            let o = eupk!(
                c, r, root, 0, f, sl,
                [ip(sl, 0), ip(sl, 1)], &[],
                "row 276 {fmt:?}"
            );
            assert_eq!(o.ret, -1, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<format>", "C: source for {fmt:?}");
            assert_eq!(
                o.text(),
                format!("Expected ']' after '{marker}', got '{got}'"),
                "C: text for {fmt:?}"
            );
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            assert_eq!(o.lcp(), (1, col, col), "C: position");
        }

        // Sweeping every byte after the '!' shows the check fires for
        // everything except the closer (and the skipped separators).
        for b_ in 1u8..=255 {
            let f = cs_bytes(&[b'[', b'i', b'!', b_, b']']);
            let o = eupk!(
                c, r, "[1]", 0, f, sl, [ip(sl, 0)], &[],
                "row 276 sweep {:?}", show(&[b_])
            );
            // ']' closes the array (and then the extra ']' is garbage after the
            // format string) and the separators are skipped by next_token, so
            // neither reaches the strict check. Both are still COMPARED above;
            // only the row's own message is asserted for the rest.
            if b_ == b']' || matches!(b_, b' ' | b'\t' | b'\n' | b',' | b':') {
                continue;
            }
            assert_eq!(o.ret, -1, "C: [i!{:?}] must fail", show(&[b_]));
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
            if b_ < 0x80 {
                assert_eq!(
                    o.text(),
                    format!("Expected ']' after '!', got '{}'", b_ as char)
                );
            }
        }
    }
}

// ===========================================================================
// Rows 268 / 277 — an unpack container that reaches the end of the format
// ===========================================================================

#[test]
fn rows_268_277_unpack_unterminated_containers() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let cases: &[(&str, &str, c_int, c_int, c_int)] = &[
            ("{\"a\":1}", "{", 1, 2, 2),
            ("{\"a\":1}", "{s:i", 1, 5, 5),
            ("{\"a\":1}", "{s:i,", 1, 6, 6),
            ("{\"a\":{}}", "{s:{", 1, 5, 5),
            ("[1]", "[", 1, 2, 2),
            ("[1]", "[i", 1, 3, 3),
            ("[1]", "[i,", 1, 4, 4),
            ("[[1]]", "[[", 1, 3, 3),
            ("[1]", "[\n", 2, 1, 3),
        ];
        for &(root, fmt, line, col, pos) in cases {
            let f = cs(fmt);
            let o = eupk!(
                c, r, root, 0, f, sl,
                [a.as_ptr(), ip(sl, 0)], &[],
                "rows 268/277 {fmt:?}"
            );
            assert_eq!(o.ret, -1, "C: {fmt:?} must fail");
            assert_eq!(o.source(), "<format>", "C: source for {fmt:?}");
            assert_eq!(
                o.text(),
                "Unexpected end of format string",
                "C: text for {fmt:?}"
            );
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            assert_eq!(
                o.lcp(),
                (line, col, pos),
                "C: position for {fmt:?}"
            );
        }
    }
}

// ===========================================================================
// Row 278 — inside [], a character that is not an unpack value starter
// ===========================================================================

#[test]
fn row_278_unpack_array_element_is_not_a_value_starter() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // `unpack_value_starters` is "{[siIbfFOon"; '!' , '*' and ']' are
        // handled before the strchr, and the separators are skipped.
        const STARTERS: &[u8] = b"{[siIbfFOon";
        for b in 1u8..=255 {
            if STARTERS.contains(&b) || matches!(b, b'!' | b'*' | b']') {
                continue;
            }
            if matches!(b, b' ' | b'\t' | b'\n' | b',' | b':') {
                continue; // skipped by next_token -> reaches the ']' instead
            }
            let f = cs_bytes(&[b'[', b, b']']);
            let o = eupk!(c, r, "[1]", 0, f, sl, [], &[], "row 278 {:?}", show(&[b]));
            assert_eq!(o.ret, -1, "C: [{:?}] must fail", show(&[b]));
            assert_eq!(o.source(), "<format>", "C: source");
            assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT, "C: code 9");
            if b < 0x80 {
                assert_eq!(
                    o.text(),
                    format!("Unexpected format character '{}'", b as char),
                    "C: text for {:?}", show(&[b])
                );
            }
            assert_eq!(o.lcp(), (1, 2, 2), "C: position");
        }
        // Note the asymmetry with the top level: '%' and '#' are legal *after*
        // an 's' but are not value starters on their own.
        let f = cs("[%]");
        let o = eupk!(c, r, "[1]", 0, f, sl, [], &[], "row 278 [%]");
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Unexpected format character '%'");
        // The strchr guard applies to nested arrays too.
        let f = cs("[[q]]");
        let o = eupk!(c, r, "[[1]]", 0, f, sl, [], &[], "row 278 nested");
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Unexpected format character 'q'");
        assert_eq!(o.code(), JSON_ERROR_INVALID_FORMAT);
    }
}

// ===========================================================================
// Row 279 — array index past the end
// ===========================================================================

#[test]
fn row_279_unpack_array_index_out_of_range() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // (root, format, the index the message must name, column)
        let cases: &[(&str, &str, usize, c_int)] = &[
            ("[1,2]", "[iii]", 2, 4),
            ("[]", "[i]", 0, 2),
            ("[1]", "[ii]", 1, 3),
            ("[1,2,3]", "[iiii]", 3, 5),
        ];
        for &(root, fmt, idx, col) in cases {
            let f = cs(fmt);
            let o = eupk!(
                c, r, root, 0, f, sl,
                [ip(sl, 0), ip(sl, 1), ip(sl, 2), ip(sl, 3)], &[],
                "row 279 root={root} fmt={fmt}"
            );
            assert_eq!(o.ret, -1, "C: {fmt} on {root} must fail");
            assert_eq!(o.source(), "<validation>", "C: source");
            assert_eq!(
                o.text(),
                format!("Array index {idx} out of range"),
                "C: text for {fmt} on {root}"
            );
            assert_eq!(o.code(), JSON_ERROR_INDEX_OUT_OF_RANGE, "C: code 17");
            assert_eq!(o.lcp(), (1, col, col), "C: position");
        }
        // The earlier elements HAVE been written before the failure, which is
        // part of the observable state on this error path.
        let f = cs("[iii]");
        let o = eupk!(
            c, r, "[7,9]", 0, f, sl,
            [ip(sl, 0), ip(sl, 1), ip(sl, 2)], &[],
            "row 279 partial writes"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.slots.ints[0], 7, "C: the first element was written");
        assert_eq!(o.slots.ints[1], 9, "C: the second element was written");
        assert_eq!(o.slots.ints[2], POISON_I32, "C: the third was not");
        // Nested.
        let a = cs("a");
        let f = cs("{s:[ii]}");
        let o = eupk!(
            c, r, "{\"a\":[1]}", 0, f, sl,
            [a.as_ptr(), ip(sl, 0), ip(sl, 1)], &[],
            "row 279 nested"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Array index 1 out of range");
        assert_eq!(o.code(), JSON_ERROR_INDEX_OUT_OF_RANGE);
    }
}

// ===========================================================================
// Rows 272 / 273 / 280 — strict-mode leftovers
// ===========================================================================

#[test]
fn rows_272_273_280_unpack_strict_leftovers() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let a = cs("a");
        let b = cs("b");
        let zz = cs("zz");

        // Row 272 — trailing '!' (or JSON_STRICT) with keys never unpacked.
        // The unrecognised-key list is built in the object's INSERTION order,
        // which both libraries share because both parsed the same text.
        let f = cs("{!}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", 0, f, sl, [], &[],
            "row 272 {{!}}"
        );
        assert_eq!(o.ret, -1, "C: {{!}} on a 2-key object must fail");
        assert_eq!(o.source(), "<validation>", "C: source");
        assert_eq!(
            o.text(),
            "2 object item(s) left unpacked: a, b",
            "C: text names every unrecognised key"
        );
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED, "C: code 7");
        assert_eq!(o.lcp(), (1, 3, 3), "C: position");

        let f = cs("{s:i!}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", 0, f, sl, [a.as_ptr(), ip(sl, 0)], &[],
            "row 272 one left"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "1 object item(s) left unpacked: b");
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);
        assert_eq!(o.slots.ints[0], 1, "C: the recognised key was still unpacked");

        // JSON_STRICT is exactly equivalent to the trailing '!'.
        let f = cs("{}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, f, sl, [], &[],
            "row 272 JSON_STRICT"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "2 object item(s) left unpacked: a, b");
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);
        // ... and a trailing '*' switches strictness OFF, so nothing is
        // reported even under JSON_STRICT.
        let f = cs("{*}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, f, sl, [], &[],
            "row 272 trailing '*' under JSON_STRICT"
        );
        assert_eq!(o.ret, 0, "C: '*' disables the strict check");

        // Row 273 — JSON_STRICT plus an optional key, forcing the full sweep
        // even though the counts match (`gotopt` is what triggers it).
        let f = cs("{s?i,s?i}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, f, sl,
            [a.as_ptr(), ip(sl, 0), zz.as_ptr(), ip(sl, 1)], &[],
            "row 273"
        );
        assert_eq!(o.ret, -1, "C: the sweep must find \"b\" unpacked");
        assert_eq!(o.source(), "<validation>");
        assert_eq!(o.text(), "1 object item(s) left unpacked: b");
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED, "C: code 7");
        // The same with the trailing '!' spelling.
        let f = cs("{s?i,s?i!}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", 0, f, sl,
            [a.as_ptr(), ip(sl, 0), zz.as_ptr(), ip(sl, 1)], &[],
            "row 273 with '!'"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "1 object item(s) left unpacked: b");
        // Three unrecognised keys, so the ", " separator is exercised twice.
        let f = cs("{s?i!}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2,\"c\":3,\"d\":4}", 0, f, sl,
            [zz.as_ptr(), ip(sl, 0)], &[],
            "row 273 four unrecognised"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "4 object item(s) left unpacked: a, b, c, d");
        // Counts match AND gotopt is set, but every key was seen -> success.
        let f = cs("{s?i,s?i}");
        let o = eupk!(
            c, r, "{\"a\":1,\"b\":2}", JSON_STRICT, f, sl,
            [a.as_ptr(), ip(sl, 0), b.as_ptr(), ip(sl, 1)], &[],
            "row 273 all keys seen"
        );
        assert_eq!(o.ret, 0, "C: nothing is left unpacked");

        // Row 280 — trailing '!' (or JSON_STRICT) with array items left over.
        let cases: &[(&str, &str, size_t, &str, c_int)] = &[
            ("[1,2]", "[!]", 0, "2 array item(s) left unpacked", 3),
            ("[1,2]", "[i!]", 0, "1 array item(s) left unpacked", 4),
            ("[1,2,3]", "[i!]", 0, "2 array item(s) left unpacked", 4),
        ];
        for &(root, fmt, _n, want, col) in cases {
            let f = cs(fmt);
            let o = eupk!(
                c, r, root, 0, f, sl, [ip(sl, 0)], &[],
                "row 280 {fmt} on {root}"
            );
            assert_eq!(o.ret, -1, "C: {fmt} on {root} must fail");
            assert_eq!(o.source(), "<validation>", "C: source");
            assert_eq!(o.text(), want, "C: text for {fmt} on {root}");
            assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED, "C: code 7");
            assert_eq!(o.lcp(), (1, col, col), "C: position");
        }
        let f = cs("[]");
        let o = eupk!(
            c, r, "[1,2]", JSON_STRICT, f, sl, [], &[],
            "row 280 JSON_STRICT"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "2 array item(s) left unpacked");
        let f = cs("[*]");
        let o = eupk!(
            c, r, "[1,2]", JSON_STRICT, f, sl, [], &[],
            "row 280 trailing '*' under JSON_STRICT"
        );
        assert_eq!(o.ret, 0, "C: '*' disables the strict check");
        // Nested: strict applies per container.
        let f = cs("{s:[i!]}");
        let o = eupk!(
            c, r, "{\"a\":[1,2,3]}", 0, f, sl, [a.as_ptr(), ip(sl, 0)], &[],
            "row 280 nested"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "2 array item(s) left unpacked");
    }
}

// ===========================================================================
// Rows 265 / 274 — unpack's out-of-memory paths
// ===========================================================================

#[test]
fn rows_265_274_unpack_out_of_memory() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        install_hooks(c, r);
        let a = cs("a");
        let zz = cs("zz");

        // The roots must be PARSED BEFORE the allocator switch is armed —
        // json_loads itself allocates 16-byte strbuffers and 128-byte hashtable
        // bucket arrays, which are exactly the sizes being failed below.
        macro_rules! oomu {
            ($size:expr, $root:expr, $flags:expr, $fmt:expr, $sl:ident,
             [$($arg:expr),* $(,)?], $($ctx:tt)*) => {{
                let croot = load(c, $root);
                let rroot = load(r, $root);
                let f_ = cs($fmt);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let mut cslots = Slots::poisoned();
                let mut rslots = Slots::poisoned();
                FAIL_SIZE.with(|s| s.set($size));
                let cret = {
                    let $sl: *mut Slots = &mut cslots;
                    let _ = $sl;
                    (c.json_unpack_ex)(croot, &mut ce, $flags, f_.as_ptr(), $($arg),*)
                };
                let rret = {
                    let $sl: *mut Slots = &mut rslots;
                    let _ = $sl;
                    (r.json_unpack_ex)(rroot, &mut re, $flags, f_.as_ptr(), $($arg),*)
                };
                clear_switches();
                let ctx_ = format!($($ctx)*);
                let co = UObs { ret: cret, snap: ce.snapshot(), raw: ce.raw(),
                                slots: cslots.summary(c) };
                let ro = UObs { ret: rret, snap: re.snapshot(), raw: re.raw(),
                                slots: rslots.summary(r) };
                diff_eq!(co.ret, ro.ret, "unpack OOM return — {ctx_}");
                diff_eq!(co.snap.clone(), ro.snap.clone(), "unpack OOM error — {ctx_}");
                diff_eq!(co.raw.clone(), ro.raw.clone(), "unpack OOM raw error — {ctx_}");
                diff_eq!(co.slots.clone(), ro.slots.clone(),
                         "unpack OOM out-pointers — {ctx_}");
                decref(c, croot);
                decref(r, rroot);
                co
            }};
        }

        // ---- Row 265: hashtable_init(&key_set) fails. hashtable_init is the
        // only 128-byte request `json_vunpack_ex` makes (the roots were built
        // before the switch was armed), so this aims at exactly that call.
        for (root, fmt) in [
            ("{\"a\":1}", "{s:i}"),
            ("{\"a\":1}", "{}"),
            ("{\"a\":1}", "{!}"),
            ("{\"a\":{\"b\":1}}", "{s:{}}"),
        ] {
            let o = oomu!(
                SZ_HASHTABLE_INIT, root, 0, fmt, sl, [a.as_ptr(), ip(sl, 0)],
                "row 265 {fmt} on {root}"
            );
            assert_eq!(o.ret, -1, "C: {fmt} must fail when hashtable_init fails");
            assert_eq!(o.source(), "<internal>", "C: source for {fmt}");
            assert_eq!(o.text(), "Out of memory", "C: text for {fmt}");
            assert_eq!(o.code(), JSON_ERROR_OUT_OF_MEMORY, "C: code 1 for {fmt}");
            // The error is set from the CURRENT token, which at that point is
            // still the initial one.
            assert_eq!(o.lcp(), (1, 1, 1), "C: position");
        }
        // A non-object root: hashtable_init runs BEFORE the type check, so OOM
        // wins over "Expected object, got ...".
        let o = oomu!(
            SZ_HASHTABLE_INIT, "[1]", 0, "{}", sl, [],
            "row 265 precedes the type check"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "Out of memory");
        assert_eq!(o.code(), JSON_ERROR_OUT_OF_MEMORY);

        // ---- Row 274: the strict key-list strbuffer fails, so the message
        // degrades to "<unknown>" instead of listing the keys.
        // strbuffer_init asks for 16 bytes; nothing else on this path does.
        let o = oomu!(
            SZ_STRBUFFER_INIT, "{\"a\":1,\"b\":2}", 0, "{!}", sl, [],
            "row 274 strbuffer_init"
        );
        assert_eq!(o.ret, -1, "C: must still report the leftovers");
        assert_eq!(o.source(), "<validation>", "C: source");
        assert_eq!(
            o.text(),
            "2 object item(s) left unpacked: <unknown>",
            "C: the key list degrades to <unknown>"
        );
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED, "C: code 7");

        let o = oomu!(
            SZ_STRBUFFER_INIT, "{\"a\":1,\"b\":2}", JSON_STRICT, "{}", sl, [],
            "row 274 under JSON_STRICT"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(o.text(), "2 object item(s) left unpacked: <unknown>");
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);

        // The other half of row 274: strbuffer_init succeeds but
        // strbuffer_append_bytes fails. A 20-byte key forces the first append
        // to grow the 16-byte buffer to 32.
        let o = oomu!(
            SZ_STRBUFFER_GROW, "{\"aaaaaaaaaaaaaaaaaaaa\":1,\"b\":2}", 0, "{!}", sl, [],
            "row 274 strbuffer_append_bytes"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(
            o.text(),
            "2 object item(s) left unpacked: <unknown>",
            "C: a failed append degrades the list too"
        );
        assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);

        // Control: with the allocator healthy the keys ARE listed, so the two
        // assertions above are really observing the failure.
        let f = cs("{!}");
        let o = eupk!(
            c, r, "{\"aaaaaaaaaaaaaaaaaaaa\":1,\"b\":2}", 0, f, sl, [], &[],
            "row 274 control"
        );
        assert_eq!(o.ret, -1);
        assert_eq!(
            o.text(),
            "2 object item(s) left unpacked: aaaaaaaaaaaaaaaaaaaa, b"
        );

        // ---- A budget sweep, which also proves the two libraries allocate the
        // same number of times in the same order on the unpack path.
        let f = cs("{s?i,s?i!}");
        let croot = load(c, "{\"a\":1,\"b\":2,\"c\":3}");
        let rroot = load(r, "{\"a\":1,\"b\":2,\"c\":3}");
        let mut seen_oom = false;
        let mut seen_unknown = false;
        for budget in 0..8isize {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let mut cslots = Slots::poisoned();
            let mut rslots = Slots::poisoned();
            BUDGET.with(|x| x.set(budget));
            let cret = (c.json_unpack_ex)(
                croot, &mut ce, 0, f.as_ptr(),
                a.as_ptr(), ip(&mut cslots, 0), zz.as_ptr(), ip(&mut cslots, 1),
            );
            BUDGET.with(|x| x.set(budget));
            let rret = (r.json_unpack_ex)(
                rroot, &mut re, 0, f.as_ptr(),
                a.as_ptr(), ip(&mut rslots, 0), zz.as_ptr(), ip(&mut rslots, 1),
            );
            clear_switches();
            diff_eq!(cret, rret, "unpack budget {budget} return");
            diff_eq!(ce.snapshot(), re.snapshot(), "unpack budget {budget} error");
            diff_eq!(ce.raw(), re.raw(), "unpack budget {budget} raw error");
            diff_eq!(
                cslots.summary(c),
                rslots.summary(r),
                "unpack budget {budget} out-pointers"
            );
            if cret == -1 && ce.code() == JSON_ERROR_OUT_OF_MEMORY {
                seen_oom = true;
            }
            if ce.text_str().ends_with("<unknown>") {
                seen_unknown = true;
            }
        }
        decref(c, croot);
        decref(r, rroot);
        assert!(seen_oom, "the budget sweep never hit hashtable_init's failure");
        assert!(seen_unknown, "the budget sweep never hit the <unknown> key list");
        clear_switches();
    }
}

// ===========================================================================
// Flag words with undefined bits, on the ERROR paths
// ===========================================================================

/// `size_t flags` accepts any value; `unpack` reads only
/// `JSON_VALIDATE_ONLY` (0x1) and `JSON_STRICT` (0x2) and `pack` reads none at
/// all. Every other bit — 0x4, 0x8, 0x10, `SIZE_MAX`, random 64-bit words —
/// must be folded away identically by both implementations, including on the
/// error paths where the flags decide whether a vararg is consumed.
#[test]
fn undefined_flag_bits_are_ignored_on_error_paths() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let mut rng = Rng::new(0x14_f1a9);
        let mut flag_words: Vec<size_t> = vec![
            0,
            0x1,
            0x2,
            0x3,
            0x4,
            0x8,
            0x10,
            0x20,
            0x40,
            0x80,
            0x100,
            0x200,
            0x400,
            0x1_0000,
            0x8000_0000,
            1usize << 63,
            size_t::MAX,
            size_t::MAX - 1,
            size_t::MAX - 2,
            size_t::MAX - 3,
        ];
        for _ in 0..48 {
            flag_words.push(rng.next_u64() as size_t);
        }

        let k = cs("k");
        let a = cs("a");
        let zz = cs("zz");

        // ---- pack: flags are never read, so every error must be identical
        // across all flag words, not merely identical between the libraries.
        // Each format is given exactly the varargs it consumes.
        macro_rules! psweep {
            ($fmt:expr, [$($arg:expr),* $(,)?]) => {{
                let f = cs($fmt);
                let mut reference: Option<Vec<u8>> = None;
                for &flags in &flag_words {
                    let o = epk!(c, r, flags, f, [$($arg),*],
                                 "pack {:?} flags={:#x}", $fmt, flags);
                    match &reference {
                        None => reference = Some(o.raw.clone()),
                        Some(w) => assert_eq!(
                            w, &o.raw,
                            "C: pack must ignore flags entirely, but {:#x} changed \
                             the error for {:?}", flags, $fmt
                        ),
                    }
                }
            }};
        }
        psweep!("q", []);
        psweep!("{", []);
        psweep!("[", []);
        psweep!("{i:i}", []);
        psweep!("[]]", []);
        psweep!("{s", [k.as_ptr()]);
        psweep!("{s:q}", [k.as_ptr()]);
        psweep!("s", [std::ptr::null::<c_char>()]);
        psweep!("s*", [std::ptr::null::<c_char>()]);
        psweep!("s?", [std::ptr::null::<c_char>()]);
        psweep!("s#", [std::ptr::null::<c_char>(), 3 as c_int]);
        psweep!("s*+", [k.as_ptr(), k.as_ptr()]);
        psweep!("[s]", [std::ptr::null::<c_char>()]);
        psweep!("{s:s}", [k.as_ptr(), std::ptr::null::<c_char>()]);
        psweep!("O", [std::ptr::null_mut::<json_t>()]);
        psweep!("o", [std::ptr::null_mut::<json_t>()]);
        psweep!("o*", [std::ptr::null_mut::<json_t>()]);
        psweep!("f", [f64::NAN]);

        // ---- unpack: only bits 0 and 1 may matter, so the observable
        // behaviour must be a function of `flags & 3` alone. Again the varargs
        // are typed for the format, because `JSON_VALIDATE_ONLY` (bit 0) makes
        // some of these formats stop consuming value arguments while others
        // still succeed and write through them.
        macro_rules! usweep {
            ($root:expr, $fmt:expr, $sl:ident, [$($arg:expr),* $(,)?]) => {{
                let f = cs($fmt);
                let mut by_low: [Option<(c_int, Vec<u8>)>; 4] = [None, None, None, None];
                for &flags in &flag_words {
                    let o = eupk!(c, r, $root, flags, f, $sl, [$($arg),*], &[],
                                  "unpack {:?} on {} flags={:#x}", $fmt, $root, flags);
                    let low = (flags & 3) as usize;
                    match &by_low[low] {
                        None => by_low[low] = Some((o.ret, o.raw.clone())),
                        Some((wret, wraw)) => assert_eq!(
                            (*wret, wraw.clone()),
                            (o.ret, o.raw.clone()),
                            "C: only bits 0/1 of flags may matter, but {:#x} behaved \
                             differently from another word with the same low bits \
                             for {:?} on {}", flags, $fmt, $root
                        ),
                    }
                }
            }};
        }
        usweep!("1", "q", sl, []);
        usweep!("1", "s", sl, [sp(sl, 0)]);
        usweep!("1", "n", sl, []);
        usweep!("{\"a\":1}", "{i:i}", sl, [ip(sl, 0)]);
        usweep!("{\"a\":1}", "{s:i", sl, [a.as_ptr(), ip(sl, 0)]);
        usweep!("{\"a\":1,\"b\":2}", "{}", sl, []);
        usweep!("[1,2]", "[]", sl, []);
        usweep!("[1,2]", "[iii]", sl, [ip(sl, 0), ip(sl, 1), ip(sl, 2)]);
        usweep!("{\"a\":1}", "{s:i}", sl, [a.as_ptr(), ip(sl, 0)]);
        usweep!("{\"a\":1}", "{s:i}", sl, [zz.as_ptr(), ip(sl, 0)]);
        usweep!("[1]", "[q]", sl, []);
        usweep!("{\"a\":1}", "{s:s}", sl, [a.as_ptr(), sp(sl, 0)]);
        usweep!("{\"a\":1,\"b\":2}", "{s:i!}", sl, [a.as_ptr(), ip(sl, 0)]);
        usweep!("{\"a\":1}", "{s:i!i}", sl, [a.as_ptr(), ip(sl, 0)]);
        usweep!("\"x\"", "s", sl, [std::ptr::null_mut::<*const c_char>()]);
        usweep!("\"x\"", "s%", sl, [sp(sl, 0), std::ptr::null_mut::<size_t>()]);
    }
}

// ===========================================================================
// A root whose type tag is outside JSON_OBJECT..JSON_NULL
// ===========================================================================

/// A `json_t` with `type` outside `0..=7` is reachable only by corrupting the
/// struct, which no public entry point does — but it IS an input an FFI caller
/// can hand over, so both libraries must agree on what happens.
///
/// Most of `unpack()`'s arms would format the message with
///
/// ```c
///     #define type_name(x) type_names[json_typeof(x)]
/// ```
///
/// which for an out-of-range tag indexes `type_names[]` out of bounds. That is
/// undefined behaviour in the C (it reads whatever follows a static array of 8
/// pointers and then dereferences it), so there is no defined C behaviour to
/// compare against and those arms are deliberately NOT exercised. `'o'`, `'O'`
/// and the `default:` arm never touch `type_name`, so they are well defined for
/// any tag and are checked here — together with the `!root` guard that is the
/// only thing between an FFI caller and that array.
#[test]
fn root_with_an_out_of_range_type_tag() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // refcount == (size_t)-1 makes this immortal, so json_incref and
        // json_decref are both no-ops and neither library can free it.
        for tag in [8, 9, 99, -1, c_int::MAX, c_int::MIN] {
            let mut fake = json_t {
                type_: tag,
                refcount: usize::MAX,
            };
            let root: *mut json_t = &mut fake;

            // 'o' / 'O': no type inspection at all, so the pointer is simply
            // stored and 0 is returned.
            for fmt in ["o", "O"] {
                let f = cs(fmt);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let mut cslots = Slots::poisoned();
                let mut rslots = Slots::poisoned();
                let cret = (c.json_unpack_ex)(root, &mut ce, 0, f.as_ptr(), op(&mut cslots, 0));
                let rret = (r.json_unpack_ex)(root, &mut re, 0, f.as_ptr(), op(&mut rslots, 0));
                diff_eq!(cret, rret, "bad tag {tag} fmt={fmt} return");
                diff_eq!(ce.raw(), re.raw(), "bad tag {tag} fmt={fmt} raw error");
                diff_eq!(
                    cslots.objs[0] == root,
                    rslots.objs[0] == root,
                    "bad tag {tag} fmt={fmt} stored pointer"
                );
                assert_eq!(cret, 0, "C: {fmt} accepts any type tag");
                assert_eq!(cslots.objs[0], root, "C: the root pointer was stored");
                assert_eq!((*root).refcount, usize::MAX, "C: still immortal");
            }
            // The `default:` arm does not look at the root either.
            let f = cs("q");
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cret = (c.json_unpack_ex)(root, &mut ce, 0, f.as_ptr());
            let rret = (r.json_unpack_ex)(root, &mut re, 0, f.as_ptr());
            diff_eq!(cret, rret, "bad tag {tag} default arm return");
            diff_eq!(ce.raw(), re.raw(), "bad tag {tag} default arm raw error");
            assert_eq!(cret, -1);
            assert_eq!(ce.text_str(), "Unexpected format character 'q'");

            // Packing such a value with 'o'/'O' is likewise tag-agnostic.
            for fmt in ["o", "O"] {
                let f = cs(fmt);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cj = (c.json_pack_ex)(&mut ce, 0, f.as_ptr(), root);
                let rj = (r.json_pack_ex)(&mut re, 0, f.as_ptr(), root);
                diff_eq!(cj == root, rj == root, "pack {fmt} of a bad tag");
                diff_eq!(ce.raw(), re.raw(), "pack {fmt} of a bad tag raw error");
                assert_eq!(cj, root, "C: 'o'/'O' hand the pointer straight back");
            }
        }
        // And the guard itself: a NULL root never reaches type_name because
        // json_vunpack_ex rejects it up front (row 251), and every type check
        // in unpack() is written `root && !json_is_X(root)`.
        let f = cs("s");
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cret = (c.json_unpack_ex)(std::ptr::null_mut(), &mut ce, 0, f.as_ptr());
        let rret = (r.json_unpack_ex)(std::ptr::null_mut(), &mut re, 0, f.as_ptr());
        diff_eq!(cret, rret, "NULL root never reaches type_name");
        diff_eq!(ce.raw(), re.raw(), "NULL root raw error");
        assert_eq!(ce.text_str(), "NULL root value");
    }
}

// ===========================================================================
// The v* entry points, on their error paths
// ===========================================================================

/// `json_vpack_ex` / `json_vunpack_ex` do `va_copy(ap_copy, ap)` and then take
/// an early return on every error path, so this is exactly where a vararg
/// handling bug would hide: the copy is made, partially consumed, and
/// abandoned. Every case below is driven through the real `va_list`
/// trampolines in `tests/vashim.c`.
#[test]
fn v_entry_points_on_error_paths() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let sh = vashim();
        let cpk = sym_addr("C", b"json_vpack_ex");
        let rpk = sym_addr("Rust", b"json_vpack_ex");
        let cup = sym_addr("C", b"json_vunpack_ex");
        let rup = sym_addr("Rust", b"json_vunpack_ex");

        let k = cs("k");
        let a = cs("a");
        let bad = cs_bytes(b"\xff");

        // ---- json_vpack_ex: one case per pack error family.
        macro_rules! vpk {
            ($fmt:expr, [$($arg:expr),* $(,)?], $want_text:expr, $want_code:expr) => {{
                let f = cs($fmt);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cj = (sh.vpack_ex)(cpk, &mut ce, 0, f.as_ptr(), $($arg),*);
                let rj = (sh.vpack_ex)(rpk, &mut re, 0, f.as_ptr(), $($arg),*);
                diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex {:?} NULL-ness", $fmt);
                diff_eq!(ce.snapshot(), re.snapshot(), "vpack_ex {:?} error", $fmt);
                diff_eq!(ce.raw(), re.raw(), "vpack_ex {:?} raw error", $fmt);
                diff_eq!(canon(c, cj), canon(r, rj), "vpack_ex {:?} tree", $fmt);
                assert!(cj.is_null(), "C: vpack_ex {:?} must fail", $fmt);
                assert_eq!(ce.text_str(), $want_text, "C: text for {:?}", $fmt);
                assert_eq!(ce.code(), $want_code, "C: code for {:?}", $fmt);
                decref(c, cj);
                decref(r, rj);
            }};
        }
        vpk!("q", [], "Unexpected format character 'q'", JSON_ERROR_INVALID_FORMAT);
        vpk!("{", [], "Unexpected end of format string", JSON_ERROR_INVALID_FORMAT);
        vpk!("[", [], "Unexpected end of format string", JSON_ERROR_INVALID_FORMAT);
        vpk!("[]]", [], "Garbage after format string", JSON_ERROR_INVALID_FORMAT);
        vpk!("{i:i}", [1 as c_int, 2 as c_int], "Expected format 's', got 'i'",
             JSON_ERROR_INVALID_FORMAT);
        vpk!("s", [std::ptr::null::<c_char>()], "NULL string", JSON_ERROR_NULL_VALUE);
        vpk!("s", [bad.as_ptr()], "Invalid UTF-8 string", JSON_ERROR_INVALID_UTF8);
        vpk!("s#", [bad.as_ptr(), 1 as c_int], "Invalid UTF-8 string",
             JSON_ERROR_INVALID_UTF8);
        vpk!("s++", [a.as_ptr(), std::ptr::null::<c_char>(), a.as_ptr()],
             "NULL string", JSON_ERROR_NULL_VALUE);
        vpk!("s?+", [a.as_ptr(), a.as_ptr()], "Cannot use '+' on optional strings",
             JSON_ERROR_INVALID_FORMAT);
        vpk!("{s:i}", [std::ptr::null::<c_char>(), 1 as c_int], "NULL object key",
             JSON_ERROR_NULL_VALUE);
        vpk!("{s:i}", [bad.as_ptr(), 1 as c_int], "Invalid UTF-8 object key",
             JSON_ERROR_INVALID_UTF8);
        vpk!("{s:s}", [k.as_ptr(), std::ptr::null::<c_char>()], "NULL string",
             JSON_ERROR_NULL_VALUE);
        vpk!("[s]", [std::ptr::null::<c_char>()], "NULL string", JSON_ERROR_NULL_VALUE);
        vpk!("O", [std::ptr::null_mut::<json_t>()], "NULL object",
             JSON_ERROR_NULL_VALUE);
        vpk!("o", [std::ptr::null_mut::<json_t>()], "NULL object",
             JSON_ERROR_NULL_VALUE);
        vpk!("f", [f64::NAN], "Invalid floating point value",
             JSON_ERROR_NUMERIC_OVERFLOW);
        vpk!("{s", [k.as_ptr()], "Unexpected format character '",
             JSON_ERROR_INVALID_FORMAT);
        // A failure AFTER many varargs have been consumed, so the copy has
        // really been walked before the early return. The double forces the
        // FP register save area to be used as well.
        vpk!(
            "[i,I,f,s,b,s]",
            [
                1 as c_int, 2i64 as json_int_t, 2.5f64, a.as_ptr(), 1 as c_int,
                std::ptr::null::<c_char>()
            ],
            "NULL string",
            JSON_ERROR_NULL_VALUE
        );
        // Enough arguments to spill past both register save areas.
        vpk!(
            "[i,i,i,i,i,i,i,i,f,f,f,f,f,f,f,f,f,s]",
            [
                1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
                7 as c_int, 8 as c_int, 1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64, 6.0f64,
                7.0f64, 8.0f64, 9.0f64, std::ptr::null::<c_char>()
            ],
            "NULL string",
            JSON_ERROR_NULL_VALUE
        );
        // The no-error-recorded rows through the v* entry point too.
        for fmt in ["s*", "o*", "O*"] {
            let f = cs(fmt);
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (sh.vpack_ex)(cpk, &mut ce, 0, f.as_ptr(), std::ptr::null::<c_char>());
            let rj = (sh.vpack_ex)(rpk, &mut re, 0, f.as_ptr(), std::ptr::null::<c_char>());
            diff_eq!(cj.is_null(), rj.is_null(), "vpack_ex {fmt} NULL-ness");
            diff_eq!(ce.raw(), re.raw(), "vpack_ex {fmt} raw error");
            assert!(cj.is_null());
            assert_eq!(ce.text_str(), "", "C: no error is recorded for {fmt}");
        }

        // ---- json_vunpack_ex.
        macro_rules! vup {
            ($root:expr, $flags:expr, $fmt:expr, $sl:ident, [$($arg:expr),* $(,)?],
             $want_text:expr, $want_code:expr) => {{
                let f = cs($fmt);
                let croot = load(c, $root);
                let rroot = load(r, $root);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let mut cslots = Slots::poisoned();
                let mut rslots = Slots::poisoned();
                let cret = {
                    let $sl: *mut Slots = &mut cslots;
                    let _ = $sl;
                    (sh.vunpack_ex)(cup, croot, &mut ce, $flags, f.as_ptr(), $($arg),*)
                };
                let rret = {
                    let $sl: *mut Slots = &mut rslots;
                    let _ = $sl;
                    (sh.vunpack_ex)(rup, rroot, &mut re, $flags, f.as_ptr(), $($arg),*)
                };
                diff_eq!(cret, rret, "vunpack_ex {:?} return", $fmt);
                diff_eq!(ce.snapshot(), re.snapshot(), "vunpack_ex {:?} error", $fmt);
                diff_eq!(ce.raw(), re.raw(), "vunpack_ex {:?} raw error", $fmt);
                diff_eq!(cslots.summary(c), rslots.summary(r),
                         "vunpack_ex {:?} out-pointers", $fmt);
                assert_eq!(cret, -1, "C: vunpack_ex {:?} must fail", $fmt);
                assert_eq!(ce.text_str(), $want_text, "C: text for {:?}", $fmt);
                assert_eq!(ce.code(), $want_code, "C: code for {:?}", $fmt);
                decref(c, croot);
                decref(r, rroot);
            }};
        }
        vup!("1", 0, "q", sl, [], "Unexpected format character 'q'",
             JSON_ERROR_INVALID_FORMAT);
        vup!("1", 0, "s", sl, [sp(sl, 0)], "Expected string, got integer",
             JSON_ERROR_WRONG_TYPE);
        vup!("\"x\"", 0, "s", sl, [std::ptr::null_mut::<*const c_char>()],
             "NULL string argument", JSON_ERROR_NULL_VALUE);
        vup!("\"x\"", 0, "s%", sl, [sp(sl, 0), std::ptr::null_mut::<size_t>()],
             "NULL string length argument", JSON_ERROR_NULL_VALUE);
        vup!("{\"a\":1}", 0, "{s:i}", sl, [std::ptr::null::<c_char>(), ip(sl, 0)],
             "NULL object key", JSON_ERROR_NULL_VALUE);
        vup!("{\"a\":1}", 0, "{i:i}", sl, [ip(sl, 0)],
             "Expected format 's', got 'i'", JSON_ERROR_INVALID_FORMAT);
        vup!("{\"a\":1}", 0, "{s:i", sl, [a.as_ptr(), ip(sl, 0)],
             "Unexpected end of format string", JSON_ERROR_INVALID_FORMAT);
        vup!("{\"a\":1}", 0, "{}}", sl, [], "Garbage after format string",
             JSON_ERROR_INVALID_FORMAT);
        vup!("[1]", 0, "[q]", sl, [], "Unexpected format character 'q'",
             JSON_ERROR_INVALID_FORMAT);
        vup!("[1]", 0, "[ii]", sl, [ip(sl, 0), ip(sl, 1)],
             "Array index 1 out of range", JSON_ERROR_INDEX_OUT_OF_RANGE);
        vup!("[1,2]", 0, "[!]", sl, [], "2 array item(s) left unpacked",
             JSON_ERROR_END_OF_INPUT_EXPECTED);
        vup!("{\"a\":1,\"b\":2}", 0, "{!}", sl, [],
             "2 object item(s) left unpacked: a, b", JSON_ERROR_END_OF_INPUT_EXPECTED);
        vup!("{\"a\":1}", 0, "{s:i}", sl, [k.as_ptr(), ip(sl, 0)],
             "Object item not found: k", JSON_ERROR_ITEM_NOT_FOUND);
        vup!("[1]", 0, "{}", sl, [], "Expected object, got array",
             JSON_ERROR_WRONG_TYPE);
        vup!("{\"a\":1}", 0, "[]", sl, [], "Expected array, got object",
             JSON_ERROR_WRONG_TYPE);
        vup!("{\"a\":1}", 0, "{s:i!s:i}", sl,
             [a.as_ptr(), ip(sl, 0), a.as_ptr(), ip(sl, 1)],
             "Expected '}' after '!', got 's'", JSON_ERROR_INVALID_FORMAT);
        // A failure only after many out-pointers have been consumed, including
        // enough of them to spill past the GP register save area.
        vup!(
            "[1,2,3,4,5,6,7,8,\"x\"]", 0, "[i,i,i,i,i,i,i,i,i]", sl,
            [
                ip(sl, 0), ip(sl, 1), ip(sl, 2), ip(sl, 3), ip(sl, 4), ip(sl, 5),
                ip(sl, 6), ip(sl, 7), i64p(sl, 0)
            ],
            "Expected integer, got string", JSON_ERROR_WRONG_TYPE
        );
    }
}

// ===========================================================================
// json_pack / json_unpack — the 0-error-pointer variadic shims, on errors
// ===========================================================================

/// `json_pack` and `json_unpack` pass `error == NULL` and `flags == 0` and are
/// separate assembly shims in the Rust port (a different number of named
/// parameters means a different starting `gp_offset`), so their error paths are
/// swept separately from `json_pack_ex`/`json_unpack_ex`.
#[test]
fn json_pack_and_json_unpack_error_paths_with_a_null_error_struct() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let k = cs("k");
        let a = cs("a");
        let bad = cs_bytes(b"\xff");

        for (fmt, kind) in [
            ("q", 0),
            ("{", 0),
            ("[", 0),
            ("[]]", 0),
            ("{s", 1),
            ("{i:i}", 2),
            ("s", 3),
            ("s#", 4),
            ("O", 5),
            ("o", 5),
            ("o*", 5),
            ("s*", 3),
            ("s?", 3),
            ("f", 6),
            ("{s:s}", 7),
            ("[s]", 3),
            ("s?+", 8),
        ] {
            let f = cs(fmt);
            let (cj, rj) = match kind {
                0 => ((c.json_pack)(f.as_ptr()), (r.json_pack)(f.as_ptr())),
                1 => (
                    (c.json_pack)(f.as_ptr(), k.as_ptr()),
                    (r.json_pack)(f.as_ptr(), k.as_ptr()),
                ),
                2 => (
                    (c.json_pack)(f.as_ptr(), 1 as c_int, 2 as c_int),
                    (r.json_pack)(f.as_ptr(), 1 as c_int, 2 as c_int),
                ),
                3 => (
                    (c.json_pack)(f.as_ptr(), std::ptr::null::<c_char>()),
                    (r.json_pack)(f.as_ptr(), std::ptr::null::<c_char>()),
                ),
                4 => (
                    (c.json_pack)(f.as_ptr(), bad.as_ptr(), 1 as c_int),
                    (r.json_pack)(f.as_ptr(), bad.as_ptr(), 1 as c_int),
                ),
                5 => (
                    (c.json_pack)(f.as_ptr(), std::ptr::null_mut::<json_t>()),
                    (r.json_pack)(f.as_ptr(), std::ptr::null_mut::<json_t>()),
                ),
                6 => (
                    (c.json_pack)(f.as_ptr(), f64::INFINITY),
                    (r.json_pack)(f.as_ptr(), f64::INFINITY),
                ),
                7 => (
                    (c.json_pack)(f.as_ptr(), k.as_ptr(), std::ptr::null::<c_char>()),
                    (r.json_pack)(f.as_ptr(), k.as_ptr(), std::ptr::null::<c_char>()),
                ),
                _ => (
                    (c.json_pack)(f.as_ptr(), a.as_ptr(), a.as_ptr()),
                    (r.json_pack)(f.as_ptr(), a.as_ptr(), a.as_ptr()),
                ),
            };
            diff_eq!(cj.is_null(), rj.is_null(), "json_pack({fmt:?}) NULL-ness");
            diff_eq!(canon(c, cj), canon(r, rj), "json_pack({fmt:?}) tree");
            decref(c, cj);
            decref(r, rj);
        }

        // json_unpack, likewise with error == NULL.
        for (root, fmt, kind) in [
            ("1", "q", 0),
            ("1", "s", 1),
            ("{\"a\":1}", "{i:i}", 1),
            ("{\"a\":1}", "{s:i}", 2),
            ("{\"a\":1}", "{s:i", 2),
            ("[1]", "[ii]", 3),
            ("[1,2]", "[!]", 0),
            ("{\"a\":1,\"b\":2}", "{!}", 0),
            ("[1]", "{}", 0),
            ("{\"a\":1}", "[]", 0),
            ("\"x\"", "s", 4),
        ] {
            let f = cs(fmt);
            let croot = load(c, root);
            let rroot = load(r, root);
            let mut cslots = Slots::poisoned();
            let mut rslots = Slots::poisoned();
            let kk = cs("zz");
            let (cret, rret) = match kind {
                0 => (
                    (c.json_unpack)(croot, f.as_ptr()),
                    (r.json_unpack)(rroot, f.as_ptr()),
                ),
                1 => (
                    (c.json_unpack)(croot, f.as_ptr(), sp(&mut cslots, 0)),
                    (r.json_unpack)(rroot, f.as_ptr(), sp(&mut rslots, 0)),
                ),
                2 => (
                    (c.json_unpack)(croot, f.as_ptr(), kk.as_ptr(), ip(&mut cslots, 0)),
                    (r.json_unpack)(rroot, f.as_ptr(), kk.as_ptr(), ip(&mut rslots, 0)),
                ),
                3 => (
                    (c.json_unpack)(croot, f.as_ptr(), ip(&mut cslots, 0), ip(&mut cslots, 1)),
                    (r.json_unpack)(rroot, f.as_ptr(), ip(&mut rslots, 0), ip(&mut rslots, 1)),
                ),
                _ => (
                    (c.json_unpack)(croot, f.as_ptr(), std::ptr::null_mut::<*const c_char>()),
                    (r.json_unpack)(rroot, f.as_ptr(), std::ptr::null_mut::<*const c_char>()),
                ),
            };
            diff_eq!(cret, rret, "json_unpack({root}, {fmt:?}) return");
            diff_eq!(
                cslots.summary(c),
                rslots.summary(r),
                "json_unpack({root}, {fmt:?}) out-pointers"
            );
            decref(c, croot);
            decref(r, rroot);
        }
    }
}
