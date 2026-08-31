//! Level 7: pack_unpack.c
//!
//! `json_pack_ex` / `json_unpack_ex` are variadic, so each test case names its
//! format string together with the exact argument list, and the same list is
//! passed to both libraries.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_double, c_int, c_void, CString};

const SEED: usize = 0x5eed_1234;
const JSON_VALIDATE_ONLY: usize = 0x1;
const JSON_STRICT: usize = 0x2;
const DUMP_FLAGS: usize = 0x200 | 0x80; // ENCODE_ANY | SORT_KEYS

fn seed_both() -> (&'static Lib, &'static Lib) {
    let (c, r) = libs();
    for l in [c, r] {
        let f: Symbol<FnJsonObjectSeed> = l.sym("json_object_seed");
        unsafe { f(SEED) };
    }
    (c, r)
}

#[derive(PartialEq)]
struct PackOut {
    ok: bool,
    dump: Option<Vec<u8>>,
    err: Vec<u8>,
    err_dbg: String,
}

impl std::fmt::Debug for PackOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackOut")
            .field("ok", &self.ok)
            .field(
                "dump",
                &self.dump.as_ref().map(|d| String::from_utf8_lossy(d).into_owned()),
            )
            .field("err", &self.err_dbg)
            .finish()
    }
}

unsafe fn finish_pack(l: &Lib, v: *mut JsonT, e: &JsonError) -> PackOut {
    let out = PackOut {
        ok: !v.is_null(),
        dump: if v.is_null() { None } else { dump(l, v, DUMP_FLAGS) },
        err: e.raw(),
        err_dbg: format!("{:?} code={}", e, e.text[159] as u8),
    };
    if !v.is_null() {
        let del: Symbol<FnJsonDelete> = l.sym("json_delete");
        del(v);
    }
    out
}

fn fresh_error() -> JsonError {
    JsonError {
        line: 55,
        column: 66,
        position: 77,
        source: [0x41; JSON_ERROR_SOURCE_LENGTH],
        text: [0x42; JSON_ERROR_TEXT_LENGTH],
    }
}

// ---------------------------------------------------------------- json_pack

type PackEx0 = unsafe extern "C" fn(*mut JsonError, usize, *const c_char) -> *mut JsonT;

macro_rules! pack_case {
    ($c:expr, $r:expr, $flags:expr, $fmt:expr, $sig:ty, ($($arg:expr),* $(,)?)) => {{
        let fc: Symbol<$sig> = $c.sym("json_pack_ex");
        let fr: Symbol<$sig> = $r.sym("json_pack_ex");
        let z = cs($fmt);
        let mut ec = fresh_error();
        let mut er = ec;
        unsafe {
            let a = fc(&mut ec, $flags, z.as_ptr(), $($arg),*);
            let b = fr(&mut er, $flags, z.as_ptr(), $($arg),*);
            let ao = finish_pack($c, a, &ec);
            let bo = finish_pack($r, b, &er);
            assert_eq!(ao, bo, "json_pack_ex({:?}, flags {:#x})", $fmt, $flags);
        }
    }};
}

#[test]
fn json_pack_no_arg_formats_match() {
    let (c, r) = seed_both();
    for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT, 0x3] {
        for fmt in [
            "n", "{}", "[]", "[n]", "[nn]", "{}", "[[]]", "[{}]", "",
            " ", "\t\n", "x", "?", "*", "{n:n}", "[,]", "[", "]", "{", "}",
            "{:}", "n n", "nn", "[]]", "{}}", "?n", "*n",
        ] {
            pack_case!(c, r, flags, fmt, PackEx0, ());
        }
    }
}

#[test]
fn json_pack_scalars_match() {
    let (c, r) = seed_both();
    type P1i = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, c_int) -> *mut JsonT;
    type P1I =
        unsafe extern "C" fn(*mut JsonError, usize, *const c_char, JsonInt) -> *mut JsonT;
    type P1f =
        unsafe extern "C" fn(*mut JsonError, usize, *const c_char, c_double) -> *mut JsonT;
    type P1s =
        unsafe extern "C" fn(*mut JsonError, usize, *const c_char, *const c_char) -> *mut JsonT;

    for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
        // 'i' takes an int
        for v in [0i32, 1, -1, i32::MAX, i32::MIN, 42] {
            for fmt in ["i", "[i]", "{s:i}"] {
                if fmt.contains('s') {
                    continue;
                }
                pack_case!(c, r, flags, fmt, P1i, (v));
            }
        }
        // 'I' takes a json_int_t
        for v in [0i64, 1, -1, i64::MAX, i64::MIN, 9007199254740993] {
            for fmt in ["I", "[I]"] {
                pack_case!(c, r, flags, fmt, P1I, (v));
            }
        }
        // 'f' takes a double
        for v in [
            0.0f64,
            -0.0,
            1.0,
            0.5,
            1e300,
            1e-300,
            f64::MAX,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            for fmt in ["f", "[f]", "[ff]"] {
                if fmt == "[ff]" {
                    type P2f = unsafe extern "C" fn(
                        *mut JsonError,
                        usize,
                        *const c_char,
                        c_double,
                        c_double,
                    ) -> *mut JsonT;
                    pack_case!(c, r, flags, fmt, P2f, (v, 1.0f64));
                } else {
                    pack_case!(c, r, flags, fmt, P1f, (v));
                }
            }
        }
        // 'b' takes an int (truthiness)
        for v in [0i32, 1, -1, 2, i32::MIN] {
            pack_case!(c, r, flags, "b", P1i, (v));
            pack_case!(c, r, flags, "[b]", P1i, (v));
        }
        // 's' takes a const char*
        for s in [
            "",
            "a",
            "hello",
            "ünïcödé",
            "日本語",
            "with \"quotes\"",
            "tab\there",
        ] {
            let z = cs(s);
            for fmt in ["s", "[s]", "[ss]", "s?", "s*"] {
                if fmt == "[ss]" {
                    type P2s = unsafe extern "C" fn(
                        *mut JsonError,
                        usize,
                        *const c_char,
                        *const c_char,
                        *const c_char,
                    ) -> *mut JsonT;
                    pack_case!(c, r, flags, fmt, P2s, (z.as_ptr(), z.as_ptr()));
                } else {
                    pack_case!(c, r, flags, fmt, P1s, (z.as_ptr()));
                }
            }
        }
        // invalid UTF-8 and NULL strings
        for bytes in [
            vec![0x80u8],
            vec![0xffu8, 0xfe],
            vec![0xc0u8, 0x80],
            vec![0xedu8, 0xa0, 0x80],
            vec![0xe2u8, 0x82],
        ] {
            let z = CString::new(bytes.clone()).unwrap();
            for fmt in ["s", "[s]", "s?", "s*"] {
                pack_case!(c, r, flags, fmt, P1s, (z.as_ptr()));
            }
        }
        for fmt in ["s", "[s]", "s?", "s*", "{s:n}"] {
            pack_case!(c, r, flags, fmt, P1s, (std::ptr::null::<c_char>()));
        }
    }
}

#[test]
fn json_pack_string_length_modifiers_match() {
    let (c, r) = seed_both();
    type Ps_hash = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        c_int,
    ) -> *mut JsonT;
    type Ps_pct = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        usize,
    ) -> *mut JsonT;
    type Ps_plus = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        *const c_char,
    ) -> *mut JsonT;
    type Ps_plus_hash = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        c_int,
    ) -> *mut JsonT;

    let base = cs("hello world");
    for flags in [0usize, JSON_STRICT] {
        // 's#' takes (const char*, int)
        for n in [0i32, 1, 5, 11, -1] {
            for fmt in ["s#", "[s#]"] {
                pack_case!(c, r, flags, fmt, Ps_hash, (base.as_ptr(), n));
            }
        }
        // 's%' takes (const char*, size_t)
        for n in [0usize, 1, 5, 11] {
            for fmt in ["s%", "[s%]"] {
                pack_case!(c, r, flags, fmt, Ps_pct, (base.as_ptr(), n));
            }
        }
        // 's+' concatenates
        let b = cs(" again");
        pack_case!(c, r, flags, "s+", Ps_plus, (base.as_ptr(), b.as_ptr()));
        pack_case!(c, r, flags, "[s+]", Ps_plus, (base.as_ptr(), b.as_ptr()));
        // 's+#' -> (str1, str2, len2); 's#+#' -> (str1, len1, str2, len2)
        {
            type Ps_plus_hash2 = unsafe extern "C" fn(
                *mut JsonError,
                usize,
                *const c_char,
                *const c_char,
                *const c_char,
                c_int,
            ) -> *mut JsonT;
            pack_case!(
                c,
                r,
                flags,
                "s+#",
                Ps_plus_hash2,
                (base.as_ptr(), b.as_ptr(), 3)
            );
            pack_case!(
                c,
                r,
                flags,
                "s+%",
                unsafe extern "C" fn(
                    *mut JsonError,
                    usize,
                    *const c_char,
                    *const c_char,
                    *const c_char,
                    usize,
                ) -> *mut JsonT,
                (base.as_ptr(), b.as_ptr(), 3usize)
            );
        }
        pack_case!(
            c,
            r,
            flags,
            "s#+#",
            Ps_plus_hash,
            (base.as_ptr(), 5, b.as_ptr(), 3)
        );
        pack_case!(
            c,
            r,
            flags,
            "s#+#+#",
            unsafe extern "C" fn(
                *mut JsonError,
                usize,
                *const c_char,
                *const c_char,
                c_int,
                *const c_char,
                c_int,
                *const c_char,
                c_int,
            ) -> *mut JsonT,
            (base.as_ptr(), 5, b.as_ptr(), 3, base.as_ptr(), 2)
        );
        // '#'/'%'/'+' combined with the optional markers is an error
        for fmt in ["s?#", "s*#", "s?%", "s*+"] {
            pack_case!(c, r, flags, fmt, Ps_hash, (base.as_ptr(), 3));
        }
        // NULL with a length
        pack_case!(
            c,
            r,
            flags,
            "s#",
            Ps_hash,
            (std::ptr::null::<c_char>(), 3)
        );
    }
}

#[test]
fn json_pack_objects_and_arrays_match() {
    let (c, r) = seed_both();
    type PsI = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        c_int,
    ) -> *mut JsonT;
    type PsIsI = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        c_int,
    ) -> *mut JsonT;

    let ka = cs("a");
    let kb = cs("b");
    let bad = CString::new(vec![0xffu8, 0x41]).unwrap();
    for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
        // one key + one int
        for fmt in ["{s:i}", "[{s:i}]", "{s:[i]}", "{s:[iii]}"] {
            if fmt == "{s:[iii]}" {
                type Ps3i = unsafe extern "C" fn(
                    *mut JsonError,
                    usize,
                    *const c_char,
                    *const c_char,
                    c_int,
                    c_int,
                    c_int,
                ) -> *mut JsonT;
                pack_case!(c, r, flags, fmt, Ps3i, (ka.as_ptr(), 1, 2, 3));
            } else {
                pack_case!(c, r, flags, fmt, PsI, (ka.as_ptr(), 1));
            }
        }
        // two keys + two ints
        pack_case!(c, r, flags, "{s:i,s:i}", PsIsI, (ka.as_ptr(), 1, kb.as_ptr(), 2));
        // nested objects consume (outer key, inner key, int)
        for fmt in ["{s:{s:i}}", "{s:[{s:i}]}", "{s:{s:[i]}}"] {
            type PssI = unsafe extern "C" fn(
                *mut JsonError,
                usize,
                *const c_char,
                *const c_char,
                *const c_char,
                c_int,
            ) -> *mut JsonT;
            pack_case!(c, r, flags, fmt, PssI, (ka.as_ptr(), kb.as_ptr(), 1));
        }
        // duplicate keys
        pack_case!(c, r, flags, "{s:i,s:i}", PsIsI, (ka.as_ptr(), 1, ka.as_ptr(), 2));
        // invalid UTF-8 key
        pack_case!(c, r, flags, "{s:i}", PsI, (bad.as_ptr(), 1));
        // NULL key
        pack_case!(c, r, flags, "{s:i}", PsI, (std::ptr::null::<c_char>(), 1));
        // non-string key: pack_object rejects the format before reading args
        pack_case!(c, r, flags, "{i:i}", PsIsI, (ka.as_ptr(), 1, kb.as_ptr(), 2));
        pack_case!(c, r, flags, "{n:n}", PsIsI, (ka.as_ptr(), 1, kb.as_ptr(), 2));
        // unterminated / mismatched containers
        for fmt in ["{s:i", "{s:i]", "{s:i}}", "{s}", "{s:}"] {
            pack_case!(c, r, flags, fmt, PsI, (ka.as_ptr(), 1));
        }
        for fmt in ["[i", "[i}", "[i]]", "[i,i]"] {
            type P1i =
                unsafe extern "C" fn(*mut JsonError, usize, *const c_char, c_int) -> *mut JsonT;
            pack_case!(c, r, flags, fmt, P1i, (1));
        }
        // optional object values
        for fmt in ["{s:i*}", "{s:i?}"] {
            pack_case!(c, r, flags, fmt, PsI, (ka.as_ptr(), 1));
        }
        for fmt in ["{s:s*}", "{s:s?}"] {
            type Pss = unsafe extern "C" fn(
                *mut JsonError,
                usize,
                *const c_char,
                *const c_char,
                *const c_char,
            ) -> *mut JsonT;
            pack_case!(c, r, flags, fmt, Pss, (ka.as_ptr(), kb.as_ptr()));
            pack_case!(
                c,
                r,
                flags,
                fmt,
                Pss,
                (ka.as_ptr(), std::ptr::null::<c_char>())
            );
        }
    }
}

#[test]
fn json_pack_o_and_O_match() {
    let (c, r) = seed_both();
    type PO = unsafe extern "C" fn(
        *mut JsonError,
        usize,
        *const c_char,
        *mut JsonT,
    ) -> *mut JsonT;

    for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
        for fmt in ["O", "[O]", "o", "[o]", "O?", "O*", "o?", "o*"] {
            let fc: Symbol<PO> = c.sym("json_pack_ex");
            let fr: Symbol<PO> = r.sym("json_pack_ex");
            let z = cs(fmt);
            unsafe {
                let ic: Symbol<FnJsonInteger> = c.sym("json_integer");
                let ir: Symbol<FnJsonInteger> = r.sym("json_integer");
                let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
                let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

                let va = ic(7);
                let vb = ir(7);
                let mut ec = fresh_error();
                let mut er = ec;
                let a = fc(&mut ec, flags, z.as_ptr(), va);
                let b = fr(&mut er, flags, z.as_ptr(), vb);
                // record refcounts before the results are freed
                let (rca, rcb) = if fmt.starts_with('O') {
                    ((*va).refcount, (*vb).refcount)
                } else {
                    (0, 0)
                };
                assert_eq!(rca, rcb, "refcount after pack {fmt:?} flags {flags:#x}");
                assert_eq!(
                    finish_pack(c, a, &ec),
                    finish_pack(r, b, &er),
                    "json_pack_ex({fmt:?}, json_t*, flags {flags:#x})"
                );
                if fmt.starts_with('O') {
                    dc(va);
                    dr(vb);
                }
            }
        }
        // NULL json_t*
        for fmt in ["O", "o", "O?", "o*", "[O]"] {
            let fc: Symbol<PO> = c.sym("json_pack_ex");
            let fr: Symbol<PO> = r.sym("json_pack_ex");
            let z = cs(fmt);
            unsafe {
                let mut ec = fresh_error();
                let mut er = ec;
                let a = fc(&mut ec, flags, z.as_ptr(), std::ptr::null_mut());
                let b = fr(&mut er, flags, z.as_ptr(), std::ptr::null_mut());
                assert_eq!(
                    finish_pack(c, a, &ec),
                    finish_pack(r, b, &er),
                    "json_pack_ex({fmt:?}, NULL, flags {flags:#x})"
                );
            }
        }
    }
}

#[test]
fn json_pack_null_format_matches() {
    let (c, r) = seed_both();
    let fc: Symbol<PackEx0> = c.sym("json_pack_ex");
    let fr: Symbol<PackEx0> = r.sym("json_pack_ex");
    unsafe {
        for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
            let mut ec = fresh_error();
            let mut er = ec;
            let a = fc(&mut ec, flags, std::ptr::null());
            let b = fr(&mut er, flags, std::ptr::null());
            assert_eq!(
                finish_pack(c, a, &ec),
                finish_pack(r, b, &er),
                "json_pack_ex(NULL fmt, {flags:#x})"
            );
        }
        // NULL error struct must be tolerated
        let z = cs("[i]");
        type P1i = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, c_int) -> *mut JsonT;
        let fc: Symbol<P1i> = c.sym("json_pack_ex");
        let fr: Symbol<P1i> = r.sym("json_pack_ex");
        let a = fc(std::ptr::null_mut(), 0, z.as_ptr(), 1);
        let b = fr(std::ptr::null_mut(), 0, z.as_ptr(), 1);
        assert_eq!(dump(c, a, DUMP_FLAGS), dump(r, b, DUMP_FLAGS), "NULL error");
        let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
        let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
        if !a.is_null() {
            dc(a);
        }
        if !b.is_null() {
            dr(b);
        }
    }
}

#[test]
fn json_pack_plain_matches() {
    // json_pack() has no error/flags parameters.
    let (c, r) = seed_both();
    type Pk = unsafe extern "C" fn(*const c_char, c_int) -> *mut JsonT;
    type Pk2 = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut JsonT;
    let fc: Symbol<Pk> = c.sym("json_pack");
    let fr: Symbol<Pk> = r.sym("json_pack");
    let f2c: Symbol<Pk2> = c.sym("json_pack");
    let f2r: Symbol<Pk2> = r.sym("json_pack");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
    unsafe {
        for fmt in ["[i]", "i", "{}", "x", "", "[i!]", "[[i]]"] {
            let z = cs(fmt);
            let a = fc(z.as_ptr(), 5);
            let b = fr(z.as_ptr(), 5);
            assert_eq!(
                dump(c, a, DUMP_FLAGS),
                dump(r, b, DUMP_FLAGS),
                "json_pack({fmt:?})"
            );
            assert_eq!(a.is_null(), b.is_null(), "json_pack({fmt:?}) null-ness");
            if !a.is_null() {
                dc(a);
            }
            if !b.is_null() {
                dr(b);
            }
        }
        // formats consuming two ints
        for fmt in ["[ii]", "[i,i]", "[[i][i]]"] {
            let z = cs(fmt);
            let a = f2c(z.as_ptr(), 5, 6);
            let b = f2r(z.as_ptr(), 5, 6);
            assert_eq!(
                dump(c, a, DUMP_FLAGS),
                dump(r, b, DUMP_FLAGS),
                "json_pack({fmt:?})"
            );
            assert_eq!(a.is_null(), b.is_null(), "json_pack({fmt:?}) null-ness");
            if !a.is_null() {
                dc(a);
            }
            if !b.is_null() {
                dr(b);
            }
        }
    }
}

// -------------------------------------------------------------- json_unpack

/// Build the value to unpack from a JSON text, on the given library.
unsafe fn parse(l: &Lib, text: &str) -> *mut JsonT {
    let f: Symbol<FnJsonLoads> = l.sym("json_loads");
    let z = cs(text);
    let mut e = JsonError::default();
    f(z.as_ptr(), 0x4 /* DECODE_ANY */, &mut e)
}

#[test]
fn json_unpack_validate_only_matches() {
    // With JSON_VALIDATE_ONLY no output arguments are consumed, so a huge
    // format matrix can be swept with no varargs at all.
    let (c, r) = seed_both();
    type U0 = unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char) -> c_int;
    let fc: Symbol<U0> = c.sym("json_unpack_ex");
    let fr: Symbol<U0> = r.sym("json_unpack_ex");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    let jsons = [
        "null",
        "true",
        "false",
        "0",
        "42",
        "-1",
        "1.5",
        "\"str\"",
        "\"\"",
        "[]",
        "[1]",
        "[1,2,3]",
        "[1,\"a\",null]",
        "{}",
        "{\"a\":1}",
        "{\"a\":1,\"b\":\"x\"}",
        "{\"a\":{\"b\":[1,2]}}",
        "[[1],[2]]",
    ];
    // Formats that consume *no* varargs under JSON_VALIDATE_ONLY. Note that
    // `unpack_object` reads its key with va_arg *before* checking
    // JSON_VALIDATE_ONLY, so any `{s...}` format is handled separately below.
    let fmts = [
        "n", "b", "i", "I", "f", "F", "s", "o", "O", "[]", "{}", "[i]", "[ii]",
        "[iii]", "[iiii]", "[s]", "[n]", "[[i][i]]", "[[i],[i]]", "[i!]",
        "[i*]", "!", "*", "", "x", "[", "]", "{", "}", "[i,i]",
        "s#", "s%", "s+", "[s#]", "i i", "[ii ]", " [ii] ", "[o]", "[O]",
        "[b]", "[f]", "[F]", "[I]", "[[[i]]]", "[*]", "[!]", "[i!*]", "[i*!]",
    ];
    // Formats with object keys, together with the keys they consume.
    let obj_fmts: &[(&str, &[&str])] = &[
        ("{s:i}", &["a"]),
        ("{s:i}", &["missing"]),
        ("{s:i,s:s}", &["a", "b"]),
        ("{s:o}", &["a"]),
        ("{s:{s:[ii]}}", &["a", "b"]),
        ("{s?i}", &["a"]),
        ("{s?i}", &["missing"]),
        ("{s?:i}", &["missing"]),
        ("{s*:i}", &["a"]),
        ("{s:i!}", &["a"]),
        ("{s:i*}", &["a"]),
        ("{s}", &["a"]),
        ("{s:i,}", &["a"]),
        ("{s:s%}", &["a"]),
        ("{s:i,s:i}", &["a", "a"]),
        ("{!s:i}", &["a"]),
        ("{*s:i}", &["a"]),
        ("{s:i,!}", &["a"]),
        ("{s:i,*}", &["a"]),
    ];

    unsafe {
        for j in jsons {
            let a = parse(c, j);
            let b = parse(r, j);
            assert_eq!(a.is_null(), b.is_null(), "parse {j:?}");
            if a.is_null() {
                continue;
            }
            for fmt in fmts {
                for flags in [
                    JSON_VALIDATE_ONLY,
                    JSON_VALIDATE_ONLY | JSON_STRICT,
                ] {
                    let z = cs(fmt);
                    let mut ec = fresh_error();
                    let mut er = ec;
                    let x = fc(a, &mut ec, flags, z.as_ptr());
                    let y = fr(b, &mut er, flags, z.as_ptr());
                    assert_eq!(
                        (x, ec.raw(), format!("{:?}", ec)),
                        (y, er.raw(), format!("{:?}", er)),
                        "json_unpack_ex({j:?}, {fmt:?}, {flags:#x})"
                    );
                }
            }
            // object formats: supply the keys explicitly
            type U1s =
                unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char, *const c_char) -> c_int;
            type U2s = unsafe extern "C" fn(
                *mut JsonT,
                *mut JsonError,
                usize,
                *const c_char,
                *const c_char,
                *const c_char,
            ) -> c_int;
            let f1c: Symbol<U1s> = c.sym("json_unpack_ex");
            let f1r: Symbol<U1s> = r.sym("json_unpack_ex");
            let f2c: Symbol<U2s> = c.sym("json_unpack_ex");
            let f2r: Symbol<U2s> = r.sym("json_unpack_ex");
            for (fmt, keys) in obj_fmts {
                for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
                    let z = cs(fmt);
                    let kz: Vec<_> = keys.iter().map(|k| cs(k)).collect();
                    let mut ec = fresh_error();
                    let mut er = ec;
                    let (x, y) = match kz.len() {
                        1 => (
                            f1c(a, &mut ec, flags, z.as_ptr(), kz[0].as_ptr()),
                            f1r(b, &mut er, flags, z.as_ptr(), kz[0].as_ptr()),
                        ),
                        2 => (
                            f2c(a, &mut ec, flags, z.as_ptr(), kz[0].as_ptr(), kz[1].as_ptr()),
                            f2r(b, &mut er, flags, z.as_ptr(), kz[0].as_ptr(), kz[1].as_ptr()),
                        ),
                        n => panic!("unhandled key count {n}"),
                    };
                    assert_eq!(
                        (x, ec.raw(), format!("{:?}", ec)),
                        (y, er.raw(), format!("{:?}", er)),
                        "json_unpack_ex({j:?}, {fmt:?}, keys {keys:?}, {flags:#x})"
                    );
                }
                // NULL key
                for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
                    let z = cs(fmt);
                    let mut ec = fresh_error();
                    let mut er = ec;
                    let x = f1c(a, &mut ec, flags, z.as_ptr(), std::ptr::null());
                    let y = f1r(b, &mut er, flags, z.as_ptr(), std::ptr::null());
                    assert_eq!(
                        (x, ec.raw()),
                        (y, er.raw()),
                        "json_unpack_ex({j:?}, {fmt:?}, NULL key, {flags:#x})"
                    );
                }
            }
            // NULL root and NULL format
            for flags in [JSON_VALIDATE_ONLY, JSON_VALIDATE_ONLY | JSON_STRICT] {
                let z = cs("i");
                let mut ec = fresh_error();
                let mut er = ec;
                assert_eq!(
                    {
                        let x = fc(std::ptr::null_mut(), &mut ec, flags, z.as_ptr());
                        (x, ec.raw())
                    },
                    {
                        let y = fr(std::ptr::null_mut(), &mut er, flags, z.as_ptr());
                        (y, er.raw())
                    },
                    "unpack NULL root"
                );
                let mut ec = fresh_error();
                let mut er = fresh_error();
                assert_eq!(
                    {
                        let x = fc(a, &mut ec, flags, std::ptr::null());
                        (x, ec.raw())
                    },
                    {
                        let y = fr(b, &mut er, flags, std::ptr::null());
                        (y, er.raw())
                    },
                    "unpack NULL format"
                );
            }
            dc(a);
            dr(b);
        }
    }
}

#[test]
fn json_unpack_outputs_match() {
    let (c, r) = seed_both();
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    unsafe {
        // 'i' -> int*
        type Ui = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *mut c_int,
        ) -> c_int;
        let fc: Symbol<Ui> = c.sym("json_unpack_ex");
        let fr: Symbol<Ui> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("42", "i"),
            ("-1", "i"),
            ("0", "i"),
            ("[7]", "[i]"),
            ("{\"a\":9}", "{s:i}"),
            ("1.5", "i"),
            ("\"x\"", "i"),
            ("null", "i"),
            ("true", "i"),
            ("9223372036854775807", "i"),
            ("[1,2]", "[i]"),
            ("[1,2]", "[i!]"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    assert!(b.is_null());
                    continue;
                }
                let z = cs(fmt);
                // '{s:i}' needs a key argument first, skip in this typed slot
                if fmt.contains('s') {
                    dc(a);
                    dr(b);
                    continue;
                }
                let mut oa: c_int = -12345;
                let mut ob: c_int = -12345;
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob);
                assert_eq!(
                    (x, oa, ec.raw(), format!("{ec:?}")),
                    (y, ob, er.raw(), format!("{er:?}")),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // 'I' -> json_int_t*
        type UI = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *mut JsonInt,
        ) -> c_int;
        let fc: Symbol<UI> = c.sym("json_unpack_ex");
        let fr: Symbol<UI> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("42", "I"),
            ("9223372036854775807", "I"),
            ("-9223372036854775808", "I"),
            ("[7]", "[I]"),
            ("1.5", "I"),
            ("\"x\"", "I"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let mut oa: JsonInt = -12345;
                let mut ob: JsonInt = -12345;
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob);
                assert_eq!(
                    (x, oa, ec.raw()),
                    (y, ob, er.raw()),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // 'f' and 'F' -> double*
        type Uf = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *mut c_double,
        ) -> c_int;
        let fc: Symbol<Uf> = c.sym("json_unpack_ex");
        let fr: Symbol<Uf> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("1.5", "f"),
            ("1.5", "F"),
            ("42", "f"),
            ("42", "F"),
            ("[1.5]", "[f]"),
            ("[42]", "[F]"),
            ("\"x\"", "f"),
            ("null", "F"),
            ("1e308", "f"),
            ("-0.0", "f"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let mut oa: c_double = -12345.0;
                let mut ob: c_double = -12345.0;
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob);
                assert_eq!(
                    (x, oa.to_bits(), ec.raw()),
                    (y, ob.to_bits(), er.raw()),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // 'b' -> int*
        type Ub = Ui;
        let fc: Symbol<Ub> = c.sym("json_unpack_ex");
        let fr: Symbol<Ub> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("true", "b"),
            ("false", "b"),
            ("[true]", "[b]"),
            ("1", "b"),
            ("null", "b"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let mut oa: c_int = -12345;
                let mut ob: c_int = -12345;
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob);
                assert_eq!(
                    (x, oa, ec.raw()),
                    (y, ob, er.raw()),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // 's' -> const char**
        type Us = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *mut *const c_char,
        ) -> c_int;
        let fc: Symbol<Us> = c.sym("json_unpack_ex");
        let fr: Symbol<Us> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("\"hello\"", "s"),
            ("\"\"", "s"),
            ("\"ünïcödé\"", "s"),
            ("[\"x\"]", "[s]"),
            ("42", "s"),
            ("null", "s"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let mut oa: *const c_char = std::ptr::null();
                let mut ob: *const c_char = std::ptr::null();
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob);
                assert_eq!(
                    (x, cbytes(oa), ec.raw()),
                    (y, cbytes(ob), er.raw()),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // 's%' -> (const char**, size_t*)
        type Uspct = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *mut *const c_char,
            *mut usize,
        ) -> c_int;
        let fc: Symbol<Uspct> = c.sym("json_unpack_ex");
        let fr: Symbol<Uspct> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("\"hello\"", "s%"),
            ("\"\"", "s%"),
            ("[\"abc\"]", "[s%]"),
            ("42", "s%"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let mut oa: *const c_char = std::ptr::null();
                let mut ob: *const c_char = std::ptr::null();
                let mut la: usize = 999;
                let mut lb: usize = 999;
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa, &mut la);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob, &mut lb);
                assert_eq!(
                    (x, cbytes(oa), la, ec.raw()),
                    (y, cbytes(ob), lb, er.raw()),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // 'o'/'O' -> json_t**
        type Uo = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *mut *mut JsonT,
        ) -> c_int;
        let fc: Symbol<Uo> = c.sym("json_unpack_ex");
        let fr: Symbol<Uo> = r.sym("json_unpack_ex");
        for (j, fmt) in [
            ("42", "o"),
            ("42", "O"),
            ("[1,2]", "[o]"),
            ("{\"a\":1}", "o"),
            ("null", "O"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let mut oa: *mut JsonT = std::ptr::null_mut();
                let mut ob: *mut JsonT = std::ptr::null_mut();
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), &mut ob);
                let da = if oa.is_null() { None } else { dump(c, oa, DUMP_FLAGS) };
                let db = if ob.is_null() { None } else { dump(r, ob, DUMP_FLAGS) };
                assert_eq!(
                    (x, da, ec.raw()),
                    (y, db, er.raw()),
                    "unpack {j:?} with {fmt:?} flags {flags:#x}"
                );
                if fmt.contains('O') && !oa.is_null() {
                    dc(oa);
                    dr(ob);
                }
                dc(a);
                dr(b);
            }
        }

        // object key lookups: '{s:i}' takes (key, int*)
        type Usi = unsafe extern "C" fn(
            *mut JsonT,
            *mut JsonError,
            usize,
            *const c_char,
            *const c_char,
            *mut c_int,
        ) -> c_int;
        let fc: Symbol<Usi> = c.sym("json_unpack_ex");
        let fr: Symbol<Usi> = r.sym("json_unpack_ex");
        for (j, fmt, key) in [
            ("{\"a\":1}", "{s:i}", "a"),
            ("{\"a\":1}", "{s:i}", "b"),
            ("{\"a\":1}", "{s?i}", "b"),
            ("{\"a\":1}", "{s?:i}", "b"),
            ("{\"a\":1,\"b\":2}", "{s:i}", "a"),
            ("{\"a\":1,\"b\":2}", "{s:i!}", "a"),
            ("{\"a\":1,\"b\":2}", "{s:i*}", "a"),
            ("[1]", "{s:i}", "a"),
            ("{\"a\":\"x\"}", "{s:i}", "a"),
        ] {
            for flags in [0usize, JSON_STRICT] {
                let a = parse(c, j);
                let b = parse(r, j);
                if a.is_null() {
                    continue;
                }
                let z = cs(fmt);
                let k = cs(key);
                let mut oa: c_int = -12345;
                let mut ob: c_int = -12345;
                let mut ec = fresh_error();
                let mut er = ec;
                let x = fc(a, &mut ec, flags, z.as_ptr(), k.as_ptr(), &mut oa);
                let y = fr(b, &mut er, flags, z.as_ptr(), k.as_ptr(), &mut ob);
                assert_eq!(
                    (x, oa, ec.raw(), format!("{ec:?}")),
                    (y, ob, er.raw(), format!("{er:?}")),
                    "unpack {j:?} with {fmt:?} key {key:?} flags {flags:#x}"
                );
                dc(a);
                dr(b);
            }
        }

        // NULL output pointer. Only 's'/'s%' validate the target in
        // pack_unpack.c; 'i', 'I', 'f', 'F', 'b' and 'o' write through it
        // unconditionally, so passing NULL there would fault in both
        // libraries and is not probed.
        let fc: Symbol<Us> = c.sym("json_unpack_ex");
        let fr: Symbol<Us> = r.sym("json_unpack_ex");
        for (j, fmt) in [("\"x\"", "s"), ("[\"x\"]", "[s]"), ("\"x\"", "s%")] {
            let a = parse(c, j);
            let b = parse(r, j);
            let z = cs(fmt);
            let mut ec = fresh_error();
            let mut er = ec;
            let x = fc(a, &mut ec, 0, z.as_ptr(), std::ptr::null_mut());
            let y = fr(b, &mut er, 0, z.as_ptr(), std::ptr::null_mut());
            assert_eq!(
                (x, ec.raw(), format!("{ec:?}")),
                (y, er.raw(), format!("{er:?}")),
                "unpack {j:?} {fmt:?} with NULL out"
            );
            dc(a);
            dr(b);
        }
        // 's%' with a valid string target but a NULL length target
        let fc: Symbol<Uspct> = c.sym("json_unpack_ex");
        let fr: Symbol<Uspct> = r.sym("json_unpack_ex");
        {
            let a = parse(c, "\"x\"");
            let b = parse(r, "\"x\"");
            let z = cs("s%");
            let mut oa: *const c_char = std::ptr::null();
            let mut ob: *const c_char = std::ptr::null();
            let mut ec = fresh_error();
            let mut er = ec;
            let x = fc(a, &mut ec, 0, z.as_ptr(), &mut oa, std::ptr::null_mut());
            let y = fr(b, &mut er, 0, z.as_ptr(), &mut ob, std::ptr::null_mut());
            assert_eq!(
                (x, ec.raw()),
                (y, er.raw()),
                "unpack s% with NULL length out"
            );
            dc(a);
            dr(b);
        }
    }
}

#[test]
fn json_unpack_plain_matches() {
    let (c, r) = seed_both();
    type Uk = unsafe extern "C" fn(*mut JsonT, *const c_char, *mut c_int) -> c_int;
    let fc: Symbol<Uk> = c.sym("json_unpack");
    let fr: Symbol<Uk> = r.sym("json_unpack");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
    unsafe {
        for (j, fmt) in [("42", "i"), ("[1]", "[i]"), ("\"x\"", "i"), ("[1,2]", "[i]")] {
            let a = parse(c, j);
            let b = parse(r, j);
            let z = cs(fmt);
            let mut oa: c_int = -1;
            let mut ob: c_int = -1;
            assert_eq!(
                (fc(a, z.as_ptr(), &mut oa), oa),
                (fr(b, z.as_ptr(), &mut ob), ob),
                "json_unpack({j:?}, {fmt:?})"
            );
            dc(a);
            dr(b);
        }
    }
}

// ------------------------------------------------------------- json_sprintf

#[test]
fn json_sprintf_matches() {
    let (c, r) = seed_both();
    type Sp0 = unsafe extern "C" fn(*const c_char) -> *mut JsonT;
    type Sp1s = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut JsonT;
    type Sp1i = unsafe extern "C" fn(*const c_char, c_int) -> *mut JsonT;
    type Sp2 = unsafe extern "C" fn(*const c_char, *const c_char, c_int) -> *mut JsonT;

    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
    unsafe {
        // no-arg formats
        let fc: Symbol<Sp0> = c.sym("json_sprintf");
        let fr: Symbol<Sp0> = r.sym("json_sprintf");
        for fmt in ["", "plain", "with spaces", "100%%", "ünïcödé", "日本語"] {
            let z = cs(fmt);
            let a = fc(z.as_ptr());
            let b = fr(z.as_ptr());
            assert_eq!(
                dump(c, a, DUMP_FLAGS),
                dump(r, b, DUMP_FLAGS),
                "json_sprintf({fmt:?})"
            );
            if !a.is_null() {
                dc(a);
            }
            if !b.is_null() {
                dr(b);
            }
        }
        // NULL format
        assert_eq!(
            fc(std::ptr::null()).is_null(),
            fr(std::ptr::null()).is_null(),
            "json_sprintf(NULL)"
        );

        // %s
        let fc: Symbol<Sp1s> = c.sym("json_sprintf");
        let fr: Symbol<Sp1s> = r.sym("json_sprintf");
        for (fmt, arg) in [
            ("%s", "hello"),
            ("pre %s post", "x"),
            ("%s", ""),
            ("%s", "ünïcödé"),
            ("%10s|", "x"),
            ("%-10s|", "x"),
            ("%.2s", "abcdef"),
        ] {
            let z = cs(fmt);
            let s = cs(arg);
            let a = fc(z.as_ptr(), s.as_ptr());
            let b = fr(z.as_ptr(), s.as_ptr());
            assert_eq!(
                dump(c, a, DUMP_FLAGS),
                dump(r, b, DUMP_FLAGS),
                "json_sprintf({fmt:?}, {arg:?})"
            );
            if !a.is_null() {
                dc(a);
            }
            if !b.is_null() {
                dr(b);
            }
        }
        // invalid UTF-8 result must be rejected identically
        let bad = CString::new(vec![0xffu8, 0x41]).unwrap();
        let z = cs("%s");
        let a = fc(z.as_ptr(), bad.as_ptr());
        let b = fr(z.as_ptr(), bad.as_ptr());
        assert_eq!(a.is_null(), b.is_null(), "json_sprintf invalid utf-8");
        if !a.is_null() {
            dc(a);
        }
        if !b.is_null() {
            dr(b);
        }

        // numeric conversions
        let fc: Symbol<Sp1i> = c.sym("json_sprintf");
        let fr: Symbol<Sp1i> = r.sym("json_sprintf");
        for fmt in ["%d", "%i", "%5d", "%-5d|", "%05d", "%+d", "%x", "%X", "%o", "%u", "%c"] {
            for v in [0i32, 1, -1, 42, 65, i32::MAX, i32::MIN] {
                let z = cs(fmt);
                let a = fc(z.as_ptr(), v);
                let b = fr(z.as_ptr(), v);
                assert_eq!(
                    dump(c, a, DUMP_FLAGS),
                    dump(r, b, DUMP_FLAGS),
                    "json_sprintf({fmt:?}, {v})"
                );
                if !a.is_null() {
                    dc(a);
                }
                if !b.is_null() {
                    dr(b);
                }
            }
        }

        // long results (past any internal buffer)
        let fc: Symbol<Sp1s> = c.sym("json_sprintf");
        let fr: Symbol<Sp1s> = r.sym("json_sprintf");
        let long = "y".repeat(9000);
        let s = cs(&long);
        let z = cs("%s");
        let a = fc(z.as_ptr(), s.as_ptr());
        let b = fr(z.as_ptr(), s.as_ptr());
        assert_eq!(
            dump(c, a, DUMP_FLAGS),
            dump(r, b, DUMP_FLAGS),
            "json_sprintf long"
        );
        if !a.is_null() {
            dc(a);
        }
        if !b.is_null() {
            dr(b);
        }

        // mixed
        let fc: Symbol<Sp2> = c.sym("json_sprintf");
        let fr: Symbol<Sp2> = r.sym("json_sprintf");
        let s = cs("k");
        let z = cs("%s=%d");
        let a = fc(z.as_ptr(), s.as_ptr(), 7);
        let b = fr(z.as_ptr(), s.as_ptr(), 7);
        assert_eq!(
            dump(c, a, DUMP_FLAGS),
            dump(r, b, DUMP_FLAGS),
            "json_sprintf mixed"
        );
        if !a.is_null() {
            dc(a);
        }
        if !b.is_null() {
            dr(b);
        }
    }
}

const _: Option<*mut c_void> = None;
