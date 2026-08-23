#![feature(c_variadic)]
//! Phase B — PACK / UNPACK (varargs) differential tests. CONFIGS.md rows 139-223.
//!
//! Every case drives BOTH the C `libjansson.so` and the Rust `libjansson.so`
//! purely through their exported C ABI (variadic function POINTERS fetched with
//! `dlsym`) and compares the return value, the full `json_error_t` snapshot, the
//! `json_dumps` round-trip of any produced value, and the bytes written into the
//! out-parameters.
//!
//! The `v*` entry points (`json_vpack_ex`, `json_vunpack_ex`, `json_vsprintf`)
//! take a real `va_list`, so they are reached through Rust-defined `extern "C"`
//! variadic trampolines that forward their own `...` onward — that is ABI-exact
//! for both libraries with no hand-rolled `__va_list_tag` guesswork.

mod common;

use common::*;
use core::ffi::VaList;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_longlong, CString};
use std::ptr;

// ---------------------------------------------------------------- call shapes

type Pack = unsafe extern "C" fn(*const c_char, ...) -> *mut json_t;
type PackEx = unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, ...) -> *mut json_t;
type Unpack = unsafe extern "C" fn(*mut json_t, *const c_char, ...) -> c_int;
type UnpackEx =
    unsafe extern "C" fn(*mut json_t, *mut json_error_t, usize, *const c_char, ...) -> c_int;
type Sprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut json_t;

type VPackEx = unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, VaList) -> *mut json_t;
type VUnpackEx =
    unsafe extern "C" fn(*mut json_t, *mut json_error_t, usize, *const c_char, VaList) -> c_int;
type VSprintf = unsafe extern "C" fn(*const c_char, VaList) -> *mut json_t;

unsafe fn fp_pack(lib: &Library) -> Pack {
    *sym::<Pack>(lib, "json_pack")
}
unsafe fn fp_pack_ex(lib: &Library) -> PackEx {
    *sym::<PackEx>(lib, "json_pack_ex")
}
unsafe fn fp_unpack(lib: &Library) -> Unpack {
    *sym::<Unpack>(lib, "json_unpack")
}
unsafe fn fp_unpack_ex(lib: &Library) -> UnpackEx {
    *sym::<UnpackEx>(lib, "json_unpack_ex")
}
unsafe fn fp_sprintf(lib: &Library) -> Sprintf {
    *sym::<Sprintf>(lib, "json_sprintf")
}

// Trampolines: build a genuine platform `va_list` and hand it to the `v*` export.
unsafe extern "C" fn via_vpack_ex(
    f: VPackEx,
    err: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: ...
) -> *mut json_t {
    f(err, flags, fmt, ap)
}

unsafe extern "C" fn via_vunpack_ex(
    f: VUnpackEx,
    root: *mut json_t,
    err: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: ...
) -> c_int {
    f(root, err, flags, fmt, ap)
}

unsafe extern "C" fn via_vsprintf(f: VSprintf, fmt: *const c_char, ap: ...) -> *mut json_t {
    f(fmt, ap)
}

// ---------------------------------------------------------------- local helpers

const DUMP: usize = JSON_ENCODE_ANY | JSON_COMPACT;

/// Comparable outcome of a pack call: the dump of the produced value (None when
/// the call returned NULL) plus the complete error struct.
#[derive(PartialEq, Eq, Debug)]
struct Pk {
    dump: Option<String>,
    err: ErrSnap,
}

unsafe fn pk_fin(lib: &Library, v: *mut json_t, e: &json_error_t) -> Pk {
    let dump = if v.is_null() { None } else { dumps_to_string(lib, v, DUMP) };
    if !v.is_null() {
        decref(lib, v);
    }
    Pk { dump, err: e.snapshot() }
}

/// Same, for the entry points that take no `json_error_t`.
unsafe fn pk_noerr(lib: &Library, v: *mut json_t) -> Option<String> {
    let dump = if v.is_null() { None } else { dumps_to_string(lib, v, DUMP) };
    if !v.is_null() {
        decref(lib, v);
    }
    dump
}

unsafe fn dump_ref(lib: &Library, v: *const json_t) -> Option<String> {
    if v.is_null() {
        None
    } else {
        dumps_to_string(lib, v, DUMP)
    }
}

fn opt_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(cstr_to_string(p))
    }
}

/// Bytes actually pointed at by an `s%` target, using the reported length.
unsafe fn str_bytes(p: *const c_char, len: usize) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(p as *const u8, len).to_vec())
    }
}

/// Parse `text` through the library under test; panics if the fixture is bad.
unsafe fn root(lib: &Library, text: &str) -> *mut json_t {
    let loads: Symbol<FnLoads> = sym(lib, "json_loads");
    let t = cs(text);
    let mut e = json_error_t::new();
    let r = loads(t.as_ptr(), JSON_DECODE_ANY, &mut e);
    assert!(!r.is_null(), "fixture json_loads({:?}) failed: {}", text, e.text_str());
    r
}

/// `{s:i}` repeated `n` times, optionally with a trailing `!`.
fn obj_fmt(n: usize, tail: &str) -> String {
    let mut s = String::from("{");
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str("s:i");
    }
    s.push_str(tail);
    s.push('}');
    s
}

fn keys(n: usize) -> Vec<CString> {
    (0..n).map(|i| cs(&format!("k{:02}", i))).collect()
}

// ================================================================ PACK
// ---------------------------------------------------------------- rows 139-146

#[test]
fn rows139_146_pack_containers() {
    // row 139 — "{}" through json_pack and json_pack_ex.
    diff("row139/{}/json_pack", |lib| unsafe {
        let f = fp_pack(lib);
        let fmt = cs("{}");
        pk_noerr(lib, f(fmt.as_ptr()))
    });
    diff("row139/{}/json_pack_ex", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{}");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr());
        pk_fin(lib, v, &e)
    });

    // row 140 — "[]"
    diff("row140/[]/json_pack", |lib| unsafe {
        let f = fp_pack(lib);
        let fmt = cs("[]");
        pk_noerr(lib, f(fmt.as_ptr()))
    });
    diff("row140/[]/json_pack_ex", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[]");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr());
        pk_fin(lib, v, &e)
    });

    // row 141 — single key
    diff("row141/{s:i}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 42 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 142 — two keys (insertion order must be preserved)
    diff("row142/{s:i,s:i}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:i,s:i}");
        let (a, b) = (cs("zeta"), cs("alpha"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), 1 as c_int, b.as_ptr(), 2 as c_int);
        pk_fin(lib, v, &e)
    });
    // duplicate key inside one pack: second setn replaces the value in place
    diff("row142/{s:i,s:i}/dup-key", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:i,s:i}");
        let a = cs("same");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), 1 as c_int, a.as_ptr(), 2 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 143 — 12 keys: the 9th distinct key rehashes the hashtable mid-pack.
    diff("row143/12-keys-rehash", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs(&obj_fmt(12, ""));
        let k = keys(12);
        let mut e = json_error_t::new();
        let v = f(
            &mut e,
            0,
            fmt.as_ptr(),
            k[0].as_ptr(),
            0 as c_int,
            k[1].as_ptr(),
            1 as c_int,
            k[2].as_ptr(),
            2 as c_int,
            k[3].as_ptr(),
            3 as c_int,
            k[4].as_ptr(),
            4 as c_int,
            k[5].as_ptr(),
            5 as c_int,
            k[6].as_ptr(),
            6 as c_int,
            k[7].as_ptr(),
            7 as c_int,
            k[8].as_ptr(),
            8 as c_int,
            k[9].as_ptr(),
            9 as c_int,
            k[10].as_ptr(),
            10 as c_int,
            k[11].as_ptr(),
            11 as c_int,
        );
        pk_fin(lib, v, &e)
    });

    // row 144 — "[i]", "[i,i]" and a 12-element array (array grow at the 9th).
    diff("row144/[i]", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[i]");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), 7 as c_int);
        pk_fin(lib, v, &e)
    });
    diff("row144/[i,i]", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[i,i]");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), 7 as c_int, -8 as c_int);
        pk_fin(lib, v, &e)
    });
    diff("row144/12-elements-grow", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[i,i,i,i,i,i,i,i,i,i,i,i]");
        let mut e = json_error_t::new();
        let v = f(
            &mut e,
            0,
            fmt.as_ptr(),
            0 as c_int,
            1 as c_int,
            2 as c_int,
            3 as c_int,
            4 as c_int,
            5 as c_int,
            6 as c_int,
            7 as c_int,
            8 as c_int,
            9 as c_int,
            10 as c_int,
            11 as c_int,
        );
        pk_fin(lib, v, &e)
    });

    // row 145 — nested containers
    diff("row145/{s:[i,i],s:{s:n}}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:[i,i],s:{s:n}}");
        let (a, b, c) = (cs("arr"), cs("obj"), cs("inner"));
        let mut e = json_error_t::new();
        let v = f(
            &mut e,
            0,
            fmt.as_ptr(),
            a.as_ptr(),
            1 as c_int,
            2 as c_int,
            b.as_ptr(),
            c.as_ptr(),
        );
        pk_fin(lib, v, &e)
    });

    // row 146 — decoration-only characters (space/tab/LF/,/:) around a format
    for (label, fmt_text) in [
        ("row146/spaces", " { s : i , s : i } "),
        ("row146/tabs", "\t{\ts\t:\ti\t,\ts\t:\ti\t}\t"),
        ("row146/newlines", "\n{\ns\n:\ni\n,\ns\n:\ni\n}\n"),
        ("row146/commas-colons", ",,{,s,:,i,:,s,:,i,},,"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let (a, b) = (cs("a"), cs("b"));
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), 1 as c_int, b.as_ptr(), 2 as c_int);
            pk_fin(lib, v, &e)
        });
    }
}

// ---------------------------------------------------------------- rows 147-157

#[test]
fn rows147_157_pack_strings() {
    // row 147/148/149 — top-level "s"
    for (label, text) in [
        ("row147/s/ascii", "hello"),
        ("row148/s/empty", ""),
        ("row149/s/utf8", "h\u{e9}llo \u{20ac} \u{4e2d} \u{1d11e}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("s");
            let s = cs(text);
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), s.as_ptr());
            pk_fin(lib, v, &e)
        });
    }
    // invalid UTF-8 through the simple (non-concat) path
    diff("row149/s/invalid-utf8-rejected", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s");
        let s = cs_bytes(&[0x61, 0xff, 0xfe]);
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), s.as_ptr() as *const c_char);
        pk_fin(lib, v, &e)
    });
    // NULL with no modifier at all
    diff("row147/s/NULL-rejected", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>());
        pk_fin(lib, v, &e)
    });

    // row 150 — "s#" with an int length
    for (label, len) in [
        ("row150/s#/len<strlen", 3 as c_int),
        ("row150/s#/len=0", 0 as c_int),
        ("row150/s#/len=strlen", 5 as c_int),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("s#");
            let s = cs("hello");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), s.as_ptr(), len);
            pk_fin(lib, v, &e)
        });
    }
    // s# that slices a multi-byte sequence in half => invalid UTF-8
    diff("row150/s#/splits-utf8-rejected", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s#");
        let s = cs("\u{20ac}");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), s.as_ptr(), 2 as c_int);
        pk_fin(lib, v, &e)
    });
    // s# with a NULL string still consumes the length argument
    diff("row150/s#/NULL-rejected", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s#");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>(), 3 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 151 — "s%" with a size_t length
    diff("row151/s%", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s%");
        let s = cs("hello");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), s.as_ptr(), 2usize);
        pk_fin(lib, v, &e)
    });

    // row 152 — "s+"
    diff("row152/s+", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s+");
        let (a, b) = (cs("foo"), cs("bar"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), b.as_ptr());
        pk_fin(lib, v, &e)
    });

    // row 153 — "s++"
    diff("row153/s++", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s++");
        let (a, b, c) = (cs("a"), cs("b"), cs("c"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr());
        pk_fin(lib, v, &e)
    });

    // row 154 — "s+#": first arg strlen'd, second length-limited
    diff("row154/s+#", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s+#");
        let (a, b) = (cs("foo"), cs("barbaz"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), b.as_ptr(), 3 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 155 — "s#+#"
    diff("row155/s#+#", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s#+#");
        let (a, b) = (cs("foobar"), cs("bazqux"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), 3 as c_int, b.as_ptr(), 3 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 156 — "s%+%"
    diff("row156/s%+%", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s%+%");
        let (a, b) = (cs("foobar"), cs("bazqux"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), 3usize, b.as_ptr(), 3usize);
        pk_fin(lib, v, &e)
    });
    // mixed: "s#+%"
    diff("row156/s#+%", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s#+%");
        let (a, b) = (cs("foobar"), cs("bazqux"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), 2 as c_int, b.as_ptr(), 4usize);
        pk_fin(lib, v, &e)
    });

    // row 157 — modifiers on an object KEY (optional == 0 there)
    diff("row157/{s#:i}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s#:i}");
        let k = cs("keyXX");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 3 as c_int, 7 as c_int);
        pk_fin(lib, v, &e)
    });
    diff("row157/{s%:i}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s%:i}");
        let k = cs("keyXX");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 3usize, 7 as c_int);
        pk_fin(lib, v, &e)
    });
    diff("row157/{s+:i}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s+:i}");
        let (a, b) = (cs("ke"), cs("y"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), b.as_ptr(), 7 as c_int);
        pk_fin(lib, v, &e)
    });
    // key length 0 via s#
    diff("row157/{s#:i}/empty-key", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s#:i}");
        let k = cs("ignored");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 0 as c_int, 7 as c_int);
        pk_fin(lib, v, &e)
    });
}

// ---------------------------------------------------------------- rows 158-163

#[test]
fn rows158_163_pack_optional_strings() {
    // row 158 — "s?" with a real string
    diff("row158/s?/non-NULL", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s?");
        let s = cs("abc");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), s.as_ptr());
        pk_fin(lib, v, &e)
    });

    // row 159 — "s?" with NULL yields json_null()
    diff("row159/s?/NULL->null", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s?");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>());
        // json_null() is the refcount==SIZE_MAX singleton; decref is a no-op.
        let ty = if v.is_null() { -1 } else { (*v).type_ };
        let rc_is_max = !v.is_null() && (*v).refcount == usize::MAX;
        (pk_fin(lib, v, &e), ty, rc_is_max)
    });

    // row 160 — "[s*]" with NULL omits the element entirely
    diff("row160/[s*]/NULL-omits", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[s*]");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>());
        pk_fin(lib, v, &e)
    });
    // "[s?]" with NULL keeps a null element
    diff("row160/[s?]/NULL->[null]", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[s?]");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>());
        pk_fin(lib, v, &e)
    });
    // top-level "s*" with NULL: pack() yields NULL with has_error == 0, so
    // json_vpack_ex returns NULL leaving the error struct freshly initialised.
    diff("row160/s*/top-level-NULL", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("s*");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>());
        pk_fin(lib, v, &e)
    });
    // "[s*,i]" — the omitted element shifts the surviving one to index 0
    diff("row160/[s*,i]/NULL-shifts", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("[s*,i]");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), ptr::null::<c_char>(), 9 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 161 — "{s:s*}" with a NULL value omits the whole member
    diff("row161/{s:s*}/NULL-omits", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:s*}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), ptr::null::<c_char>());
        pk_fin(lib, v, &e)
    });
    diff("row161/{s:s*,s:i}/NULL-omits-one", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:s*,s:i}");
        let (a, b) = (cs("gone"), cs("kept"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), a.as_ptr(), ptr::null::<c_char>(), b.as_ptr(), 5 as c_int);
        pk_fin(lib, v, &e)
    });

    // row 162 — "{s:s?}" with a NULL value keeps the member as null
    diff("row162/{s:s?}/NULL->null", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:s?}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), ptr::null::<c_char>());
        pk_fin(lib, v, &e)
    });
    // "{s:s?}" with a real string behaves like plain s
    diff("row162/{s:s?}/non-NULL", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:s?}");
        let (k, s) = (cs("k"), cs("v"));
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), s.as_ptr());
        pk_fin(lib, v, &e)
    });

    // row 163 — "#"/"%"/"+" are refused on optional strings
    for (label, fmt_text) in [
        ("row163/s?# rejected", "s?#"),
        ("row163/s?% rejected", "s?%"),
        ("row163/s?+ rejected", "s?+"),
        ("row163/s*# rejected", "s*#"),
        ("row163/{s:s?#} rejected", "{s:s?#}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            // Nothing is consumed before the format error, but pass plenty.
            let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), k.as_ptr(), 0 as c_int);
            pk_fin(lib, v, &e)
        });
    }
}

// ---------------------------------------------------------------- rows 164-169

#[test]
fn rows164_169_pack_scalars() {
    // row 164 — "n"
    diff("row164/n", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("n");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr());
        let rc_is_max = !v.is_null() && (*v).refcount == usize::MAX;
        (pk_fin(lib, v, &e), rc_is_max)
    });

    // row 165 — "b" with 0 and non-zero
    for (label, b) in
        [("row165/b/0", 0 as c_int), ("row165/b/1", 1 as c_int), ("row165/b/-7", -7 as c_int)]
    {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("b");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), b);
            let rc_is_max = !v.is_null() && (*v).refcount == usize::MAX;
            (pk_fin(lib, v, &e), rc_is_max)
        });
    }

    // row 166 — "i" (int-width vararg)
    for (label, i) in [
        ("row166/i/0", 0 as c_int),
        ("row166/i/INT_MIN", c_int::MIN),
        ("row166/i/INT_MAX", c_int::MAX),
        ("row166/i/-1", -1 as c_int),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("i");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), i);
            pk_fin(lib, v, &e)
        });
    }

    // row 167 — "I" (json_int_t vararg)
    for (label, i) in [
        ("row167/I/0", 0 as c_longlong),
        ("row167/I/LLONG_MIN", c_longlong::MIN),
        ("row167/I/LLONG_MAX", c_longlong::MAX),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("I");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), i);
            pk_fin(lib, v, &e)
        });
    }

    // row 168 — "f" with finite doubles
    for (label, d) in [
        ("row168/f/0.0", 0.0f64),
        ("row168/f/-0.0", -0.0f64),
        ("row168/f/3.5", 3.5f64),
        ("row168/f/DBL_MAX", f64::MAX),
        ("row168/f/-DBL_MAX", f64::MIN),
        ("row168/f/5e-324", f64::from_bits(1)),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("f");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), d);
            pk_fin(lib, v, &e)
        });
    }

    // row 169 — NaN / Inf are refused by json_real_set
    for (label, d) in [
        ("row169/f/NaN rejected", f64::NAN),
        ("row169/f/+Inf rejected", f64::INFINITY),
        ("row169/f/-Inf rejected", f64::NEG_INFINITY),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("f");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), d);
            pk_fin(lib, v, &e)
        });
        // Inside a container the "NULL object value" / array error path overwrites.
        diff(&format!("{}/in-object", label), move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("{s:f}");
            let k = cs("k");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), d);
            pk_fin(lib, v, &e)
        });
        diff(&format!("{}/in-array", label), move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("[f]");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), d);
            pk_fin(lib, v, &e)
        });
    }
}

// ---------------------------------------------------------------- rows 170-174

#[test]
fn rows170_174_pack_json_args_and_refcounts() {
    // row 170 — "O" increments the refcount; "[O,O]" shares one pointer.
    diff("row170/O/top-level-increfs", |lib| unsafe {
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let f = fp_pack_ex(lib);
        let fmt = cs("O");
        let src = int(42);
        let rc0 = (*src).refcount;
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr(), src);
        let rc1 = (*src).refcount;
        let same = v == src;
        let dump = dump_ref(lib, v);
        decref(lib, v);
        let rc2 = (*src).refcount;
        decref(lib, src);
        (rc0, rc1, rc2, same, dump, e.snapshot())
    });
    diff("row170/[O,O]/shares-pointer", |lib| unsafe {
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let aget: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let f = fp_pack_ex(lib);
        let fmt = cs("[O,O]");
        let src = int(42);
        let rc0 = (*src).refcount;
        let mut e = json_error_t::new();
        let arr = f(&mut e, 0, fmt.as_ptr(), src, src);
        let rc1 = (*src).refcount; // expect 3
        let a0 = aget(arr, 0);
        let a1 = aget(arr, 1);
        let shared = a0 == a1 && a0 == src;
        let dump = dump_ref(lib, arr);
        decref(lib, arr);
        let rc2 = (*src).refcount; // expect 1
        decref(lib, src);
        (rc0, rc1, rc2, shared, dump, e.snapshot())
    });
    // "O" on a singleton: incref must be a no-op (refcount stays SIZE_MAX).
    diff("row170/O/singleton-null", |lib| unsafe {
        let nul: Symbol<FnVoidPtr> = sym(lib, "json_null");
        let f = fp_pack_ex(lib);
        let fmt = cs("[O]");
        let src = nul();
        let mut e = json_error_t::new();
        let arr = f(&mut e, 0, fmt.as_ptr(), src);
        let rc = (*src).refcount;
        let dump = dump_ref(lib, arr);
        decref(lib, arr);
        (rc == usize::MAX, dump, e.snapshot())
    });

    // row 171 — "o" steals the reference (no incref)
    diff("row171/o/steals", |lib| unsafe {
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let f = fp_pack_ex(lib);
        let fmt = cs("[o]");
        let src = int(7);
        let rc0 = (*src).refcount;
        let mut e = json_error_t::new();
        let arr = f(&mut e, 0, fmt.as_ptr(), src);
        let rc1 = (*src).refcount; // still 1
        let dump = dump_ref(lib, arr);
        decref(lib, arr); // frees src too
        (rc0, rc1, dump, e.snapshot())
    });
    diff("row171/{s:o}/steals", |lib| unsafe {
        let obj: Symbol<FnVoidPtr> = sym(lib, "json_object");
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:o}");
        let k = cs("k");
        let src = obj();
        let rc0 = (*src).refcount;
        let mut e = json_error_t::new();
        let out = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), src);
        let rc1 = (*src).refcount;
        let dump = dump_ref(lib, out);
        decref(lib, out);
        (rc0, rc1, dump, e.snapshot())
    });

    // row 172 — "O?" / "o?" with NULL -> json_null()
    for (label, fmt_text) in
        [("row172/O?/NULL->null", "O?"), ("row172/o?/NULL->null", "o?")]
    {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), ptr::null_mut::<json_t>());
            let ty = if v.is_null() { -1 } else { (*v).type_ };
            (pk_fin(lib, v, &e), ty)
        });
    }

    // row 173 — "O*" / "o*" with NULL omits the value
    for (label, fmt_text) in [
        ("row173/[O*]/NULL-omits", "[O*]"),
        ("row173/[o*]/NULL-omits", "[o*]"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), ptr::null_mut::<json_t>());
            pk_fin(lib, v, &e)
        });
    }
    for (label, fmt_text) in [
        ("row173/{s:O*}/NULL-omits", "{s:O*}"),
        ("row173/{s:o*}/NULL-omits", "{s:o*}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), ptr::null_mut::<json_t>());
            pk_fin(lib, v, &e)
        });
    }

    // row 174 — "O" / "o" with NULL and no modifier.
    //
    // These formats consume DIFFERENT vararg lists, so they cannot share one
    // call site: "O", "o" and "[O]" consume a single `json_t*`, while "{s:o}"
    // consumes a key `const char*` FIRST and then the `json_t*`. Passing the key
    // to the former group makes the library reinterpret the key's `char*` as a
    // `json_t*` — a non-NULL garbage pointer, which stops testing the NULL
    // rejection path at all and corrupts the heap.
    for (label, fmt_text) in [
        ("row174/O/NULL rejected", "O"),
        ("row174/o/NULL rejected", "o"),
        ("row174/[O]/NULL rejected", "[O]"),
        ("row174/[o]/NULL rejected", "[o]"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), ptr::null_mut::<json_t>());
            pk_fin(lib, v, &e)
        });
    }
    for (label, fmt_text) in [
        ("row174/{s:o}/NULL rejected", "{s:o}"),
        ("row174/{s:O}/NULL rejected", "{s:O}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), ptr::null_mut::<json_t>());
            pk_fin(lib, v, &e)
        });
    }
}

// ---------------------------------------------------------------- rows 175-176

#[test]
fn rows175_176_pack_rejected_format_chars() {
    // row 175 — 'r' is not a pack format character in 2.15.
    // row 176 — 'F' is unpack-only.
    for (label, fmt_text) in [
        ("row175/r rejected", "r"),
        ("row175/[r] rejected", "[r]"),
        ("row175/{s:r} rejected", "{s:r}"),
        ("row176/F rejected", "F"),
        ("row176/[F] rejected", "[F]"),
        ("row176/{s:F} rejected", "{s:F}"),
        ("row176/#-alone rejected", "#"),
        ("row176/%-alone rejected", "%"),
        ("row176/!-alone rejected", "!"),
        ("row176/'*'-alone rejected", "*"),
        ("row176/'?'-alone rejected", "?"),
        ("row176/'}'-alone rejected", "}"),
        ("row176/']'-alone rejected", "]"),
        ("row176/unterminated-{ rejected", "{"),
        ("row176/unterminated-[ rejected", "["),
        ("row176/{i:i} rejected", "{i:i}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 0 as c_int);
            pk_fin(lib, v, &e)
        });
    }
}

// ---------------------------------------------------------------- rows 177-180

#[test]
fn rows177_180_pack_entry_points_and_flags() {
    // row 177 — both flags are inert for packing (pack never reads s->flags).
    for (label, flags) in [
        ("row177/VALIDATE_ONLY inert", JSON_VALIDATE_ONLY),
        ("row177/STRICT inert", JSON_STRICT),
        ("row177/both inert", JSON_VALIDATE_ONLY | JSON_STRICT),
        ("row177/unknown-high-bits ignored", 0xFFFF_0000usize),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs("{s:i,s:[s,n]}");
            let (a, b, s) = (cs("n"), cs("a"), cs("x"));
            let mut e = json_error_t::new();
            let v =
                f(&mut e, flags, fmt.as_ptr(), a.as_ptr(), 1 as c_int, b.as_ptr(), s.as_ptr());
            pk_fin(lib, v, &e)
        });
    }
    // VALIDATE_ONLY does NOT stop "%" from being consumed on the pack side.
    diff("row177/VALIDATE_ONLY+{s:s%}", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:s%}");
        let (k, s) = (cs("k"), cs("hello"));
        let mut e = json_error_t::new();
        let v = f(&mut e, JSON_VALIDATE_ONLY, fmt.as_ptr(), k.as_ptr(), s.as_ptr(), 2usize);
        pk_fin(lib, v, &e)
    });

    // row 178 — error == NULL vs a real json_error_t, success and failure.
    diff("row178/error=NULL/success", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:i}");
        let k = cs("k");
        let v = f(ptr::null_mut(), 0, fmt.as_ptr(), k.as_ptr(), 3 as c_int);
        pk_noerr(lib, v)
    });
    diff("row178/error=NULL/failure", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("{s:q}");
        let k = cs("k");
        let v = f(ptr::null_mut(), 0, fmt.as_ptr(), k.as_ptr());
        pk_noerr(lib, v)
    });
    diff("row178/json_pack/success", |lib| unsafe {
        let f = fp_pack(lib);
        let fmt = cs("{s:i}");
        let k = cs("k");
        pk_noerr(lib, f(fmt.as_ptr(), k.as_ptr(), 3 as c_int))
    });

    // row 179 — fmt == NULL / "" (also through json_pack, whose error is NULL).
    diff("row179/json_pack_ex/fmt=NULL rejected", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, ptr::null());
        pk_fin(lib, v, &e)
    });
    diff("row179/json_pack_ex/fmt=\"\" rejected", |lib| unsafe {
        let f = fp_pack_ex(lib);
        let fmt = cs("");
        let mut e = json_error_t::new();
        let v = f(&mut e, 0, fmt.as_ptr());
        pk_fin(lib, v, &e)
    });
    diff("row179/json_pack/fmt=NULL rejected", |lib| unsafe {
        let f = fp_pack(lib);
        pk_noerr(lib, f(ptr::null()))
    });

    // row 180 — garbage after the format string
    for (label, fmt_text) in [
        ("row180/{s:i}x rejected", "{s:i}x"),
        ("row180/[i]] rejected", "[i]]"),
        ("row180/i-i rejected", "ii"),
        ("row180/{}{} rejected", "{}{}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_pack_ex(lib);
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            let v = f(&mut e, 0, fmt.as_ptr(), k.as_ptr(), 1 as c_int, 2 as c_int);
            pk_fin(lib, v, &e)
        });
    }
}

// ---------------------------------------------------------------- rows 181-183

#[test]
fn rows181_183_sprintf() {
    // row 181 — ASCII conversions
    diff("row181/json_sprintf/%s-%d", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("%s-%d");
        let s = cs("abc");
        pk_noerr(lib, f(fmt.as_ptr(), s.as_ptr(), 42 as c_int))
    });
    diff("row181/json_sprintf/no-conversions", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("plain ascii text");
        pk_noerr(lib, f(fmt.as_ptr()))
    });
    diff("row181/json_sprintf/long-forces-realloc", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("%s%s%s%s");
        let s = cs("0123456789abcdef0123456789abcdef");
        pk_noerr(lib, f(fmt.as_ptr(), s.as_ptr(), s.as_ptr(), s.as_ptr(), s.as_ptr()))
    });

    // row 182 — zero-length result takes the json_string("") shortcut
    diff("row182/json_sprintf/empty-fmt", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("");
        pk_noerr(lib, f(fmt.as_ptr()))
    });
    diff("row182/json_sprintf/%s-of-empty", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("%s");
        let s = cs("");
        pk_noerr(lib, f(fmt.as_ptr(), s.as_ptr()))
    });

    // row 183 — multi-byte UTF-8 result, and invalid UTF-8 (rejected)
    diff("row183/json_sprintf/utf8", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("%s/%s/\u{4e2d}");
        let (a, b) = (cs("h\u{e9}llo"), cs("\u{20ac}\u{1d11e}"));
        pk_noerr(lib, f(fmt.as_ptr(), a.as_ptr(), b.as_ptr()))
    });
    diff("row183/json_sprintf/invalid-utf8-arg rejected", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("%s");
        let bad = cs_bytes(&[0xff, 0xfe]);
        pk_noerr(lib, f(fmt.as_ptr(), bad.as_ptr() as *const c_char))
    });
    diff("row183/json_sprintf/invalid-utf8-fmt rejected", |lib| unsafe {
        let f = fp_sprintf(lib);
        let bad = cs_bytes(&[0x61, 0xc0, 0x80, 0x62]);
        pk_noerr(lib, f(bad.as_ptr() as *const c_char))
    });
    // truncated 2-byte lead at the very end
    diff("row183/json_sprintf/truncated-lead rejected", |lib| unsafe {
        let f = fp_sprintf(lib);
        let fmt = cs("%s");
        let bad = cs_bytes(&[0x61, 0xc3]);
        pk_noerr(lib, f(fmt.as_ptr(), bad.as_ptr() as *const c_char))
    });
}

// ================================================================ UNPACK
// ---------------------------------------------------------------- rows 184-189

#[test]
fn rows184_189_unpack_scalars() {
    // row 184 — "{s:s}" through json_unpack and json_unpack_ex.
    diff("row184/{s:s}/json_unpack", |lib| unsafe {
        let f = fp_unpack(lib);
        let r = root(lib, r#"{"k":"v"}"#);
        let fmt = cs("{s:s}");
        let k = cs("k");
        let mut out: *const c_char = ptr::null();
        let ret = f(r, fmt.as_ptr(), k.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, got)
    });
    diff("row184/{s:s}/json_unpack_ex", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"v"}"#);
        let fmt = cs("{s:s}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });
    // missing key, and a NULL string target
    diff("row184/{s:s}/missing-key rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":"v"}"#);
        let fmt = cs("{s:s}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });
    diff("row184/{s:s}/NULL-target rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"v"}"#);
        let fmt = cs("{s:s}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), ptr::null_mut::<*const c_char>());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row184/{s:s}/NULL-key rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"v"}"#);
        let fmt = cs("{s:s}");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), ptr::null::<c_char>(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });

    // row 185 — "{s:s%}" also fills a size_t length target
    diff("row185/{s:s%}", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"hello"}"#);
        let fmt = cs("{s:s%}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let mut len: usize = usize::MAX;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out, &mut len);
        let bytes = if len == usize::MAX { None } else { str_bytes(out, len) };
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got, len, bytes)
    });
    // a string whose length differs from strlen (embedded NUL)
    diff("row185/{s:s%}/embedded-NUL", |lib| unsafe {
        let obj: Symbol<FnVoidPtr> = sym(lib, "json_object");
        let strn: Symbol<FnStrN> = sym(lib, "json_stringn");
        let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let f = fp_unpack_ex(lib);
        let r = obj();
        let payload = cs_bytes(b"a\0bc");
        let key = cs("k");
        oset(r, key.as_ptr(), strn(payload.as_ptr() as *const c_char, 4));
        let fmt = cs("{s:s%}");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let mut len: usize = usize::MAX;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), key.as_ptr(), &mut out, &mut len);
        let bytes = if len == usize::MAX { None } else { str_bytes(out, len) };
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got, len, bytes)
    });
    // NULL length target
    diff("row185/{s:s%}/NULL-len-target rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"hello"}"#);
        let fmt = cs("{s:s%}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret =
            f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out, ptr::null_mut::<usize>());
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });

    // row 186 — i / I / b(true,false) / f / n
    diff("row186/{s:i}", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":-2147483648}"#);
        let fmt = cs("{s:i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // 'i' truncates a json_int_t to int
    diff("row186/{s:i}/truncates", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":9223372036854775807}"#);
        let fmt = cs("{s:i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row186/{s:I}", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":-9223372036854775808}"#);
        let fmt = cs("{s:I}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_longlong = 1;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    for (label, text) in
        [("row186/{s:b}/true", r#"{"k":true}"#), ("row186/{s:b}/false", r#"{"k":false}"#)]
    {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, text);
            let fmt = cs("{s:b}");
            let k = cs("k");
            let mut e = json_error_t::new();
            let mut out: c_int = 0x5A5A_5A5A;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out)
        });
    }
    diff("row186/{s:f}", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":-2.5e10}"#);
        let fmt = cs("{s:f}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    diff("row186/{s:n}", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":null}"#);
        let fmt = cs("{s:n}");
        let k = cs("k");
        let mut e = json_error_t::new();
        // 'n' never assigns, so no target argument is consumed.
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row186/{s:n}/on-integer rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":1}"#);
        let fmt = cs("{s:n}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row186/{s:b}/on-null rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":null}"#);
        let fmt = cs("{s:b}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 187 — "{s:F}" on a REAL
    diff("row187/{s:F}/on-real", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":3.25}"#);
        let fmt = cs("{s:F}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });

    // row 188 — "{s:F}" on an INTEGER (the only numeric-widening format)
    diff("row188/{s:F}/on-integer", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":-7}"#);
        let fmt = cs("{s:F}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    // json_number_value on a huge integer goes through a double conversion
    diff("row188/{s:F}/on-LLONG_MAX", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":9223372036854775807}"#);
        let fmt = cs("{s:F}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    diff("row188/{s:F}/on-string rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"x"}"#);
        let fmt = cs("{s:F}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });

    // row 189 — "{s:f}" on an integer is refused ('f' requires a REAL)
    diff("row189/{s:f}/on-integer rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":7}"#);
        let fmt = cs("{s:f}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    // and 'I'/'i' on a real
    diff("row189/{s:I}/on-real rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":7.5}"#);
        let fmt = cs("{s:I}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_longlong = 1;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
}

// ---------------------------------------------------------------- rows 190-191

#[test]
fn rows190_191_unpack_borrow_and_incref() {
    // row 190 — "{s:o}" borrows: no refcount change.
    diff("row190/{s:o}/borrowed", |lib| unsafe {
        let oget: Symbol<FnObjGet> = sym(lib, "json_object_get");
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":[1,2]}"#);
        let key = cs("k");
        let expect = oget(r, key.as_ptr());
        let rc0 = (*expect).refcount;
        let fmt = cs("{s:o}");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), key.as_ptr(), &mut out);
        let rc1 = (*expect).refcount;
        let same = out == expect;
        let dump = dump_ref(lib, out);
        decref(lib, r);
        (ret, e.snapshot(), rc0, rc1, same, dump)
    });

    // row 191 — "{s:O}" increfs: the caller owns a reference.
    diff("row191/{s:O}/increfs", |lib| unsafe {
        let oget: Symbol<FnObjGet> = sym(lib, "json_object_get");
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":{"a":1}}"#);
        let key = cs("k");
        let expect = oget(r, key.as_ptr());
        let rc0 = (*expect).refcount;
        let fmt = cs("{s:O}");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), key.as_ptr(), &mut out);
        let rc1 = (*expect).refcount; // rc0 + 1
        let same = out == expect;
        let dump = dump_ref(lib, out);
        decref(lib, r); // container gone, our own reference keeps it alive
        let rc2 = (*out).refcount;
        let dump_after = dump_ref(lib, out);
        decref(lib, out);
        (ret, e.snapshot(), rc0, rc1, rc2, same, dump, dump_after)
    });
    // "{s:O}" on a singleton member: incref is a no-op
    diff("row191/{s:O}/singleton", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":true}"#);
        let key = cs("k");
        let fmt = cs("{s:O}");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), key.as_ptr(), &mut out);
        let rc_is_max = !out.is_null() && (*out).refcount == usize::MAX;
        let dump = dump_ref(lib, out);
        decref(lib, r);
        (ret, e.snapshot(), rc_is_max, dump)
    });
}

// ---------------------------------------------------------------- rows 192-196

#[test]
fn rows192_196_unpack_optional_keys() {
    // row 192 — "{s?i}" with the key present
    diff("row192/{s?i}/present", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":11}"#);
        let fmt = cs("{s?i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 193 — "{s?i}" with the key missing: the target is left untouched but
    // the vararg IS still consumed.
    diff("row193/{s?i}/missing", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":11}"#);
        let fmt = cs("{s?i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // a missing optional never type-checks, so a wrong format still passes
    diff("row193/{s?i}/missing-wrong-type-unchecked", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":"s"}"#);
        let fmt = cs("{s?s}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });
    // present-but-wrong-type IS checked
    diff("row193/{s?i}/present-wrong-type rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"s"}"#);
        let fmt = cs("{s?i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 194 — "{s?{s:i}}" with the outer key missing recurses with root=NULL,
    // i.e. format-only skipping: every vararg is consumed, nothing is written.
    diff("row194/{s?{s:i}}/outer-missing", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":1}"#);
        let fmt = cs("{s?{s:i}}");
        let (ko, ki) = (cs("outer"), cs("inner"));
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), ko.as_ptr(), ki.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // skipping mode also ignores '!' (no root to compare against)
    diff("row194/{s?{s:i!}}/outer-missing", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":1}"#);
        let fmt = cs("{s?{s:i!}}");
        let (ko, ki) = (cs("outer"), cs("inner"));
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), ko.as_ptr(), ki.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // and a skipped array
    diff("row194/{s?[i,i]}/outer-missing", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":1}"#);
        let fmt = cs("{s?[i,i]}");
        let ko = cs("outer");
        let mut e = json_error_t::new();
        let mut a: c_int = 0x5A5A_5A5A;
        let mut b: c_int = 0x3C3C_3C3C;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), ko.as_ptr(), &mut a, &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });
    // a format error inside the skipped subtree is still reported
    diff("row194/{s?{s:q}}/format-error-in-skip", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"other":1}"#);
        let fmt = cs("{s?{s:q}}");
        let (ko, ki) = (cs("outer"), cs("inner"));
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), ko.as_ptr(), ki.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 195 — the SAME key named twice; key_set dedupes
    diff("row195/{s:i,s:i}/same-key-twice", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":5}"#);
        let fmt = cs("{s:i,s:i}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut a, k.as_ptr(), &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });
    // ... and it stays consistent with '!' (size 1 == key_set size 1)
    diff("row195/{s:i,s:i!}/same-key-twice-strict", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":5}"#);
        let fmt = cs("{s:i,s:i!}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut a, k.as_ptr(), &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });
    // two names for one key on a 2-key object under '!' must still fail
    diff("row195/{s:i,s:i!}/dup-name-misses-key", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":5,"z":6}"#);
        let fmt = cs("{s:i,s:i!}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut a, k.as_ptr(), &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });

    // row 196 — "{}" on an empty object, and on a non-empty one
    diff("row196/{}/on-empty", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "{}");
        let fmt = cs("{}");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row196/{}/on-non-empty", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("{}");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row196/{!}/on-non-empty rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("{!}");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
}

// ---------------------------------------------------------------- rows 197-203

#[test]
fn rows197_203_unpack_object_strict() {
    // row 197 — "{s:i!}" satisfied
    diff("row197/{s:i!}/satisfied", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("{s:i!}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 198 — "{s:i!}" with keys left over; the message lists them in
    // insertion order, separated by ", ".
    diff("row198/{s:i!}/2-left-unpacked rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"bee":2,"cee":3}"#);
        let fmt = cs("{s:i!}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // leftover key in the middle of the iteration order
    diff("row198/{s:i,s:i!}/1-left-unpacked rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"b":2,"c":3}"#);
        let fmt = cs("{s:i,s:i!}");
        let (ka, kc) = (cs("a"), cs("c"));
        let mut e = json_error_t::new();
        let mut x: c_int = 0;
        let mut y: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), ka.as_ptr(), &mut x, kc.as_ptr(), &mut y);
        decref(lib, r);
        (ret, e.snapshot(), x, y)
    });
    // an optional key sets gotopt, forcing the full key scan even when the
    // counts happen to agree
    diff("row198/{s?i,s:i!}/gotopt-forces-scan rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"b":2}"#);
        let fmt = cs("{s?i,s:i!}");
        let (kz, ka) = (cs("zz"), cs("a"));
        let mut e = json_error_t::new();
        let mut x: c_int = 0x5A5A_5A5A;
        let mut y: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), kz.as_ptr(), &mut x, ka.as_ptr(), &mut y);
        decref(lib, r);
        (ret, e.snapshot(), x, y)
    });
    // keys containing the separator, to pin the exact joined text
    diff("row198/{s:i!}/keys-with-commas rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"x, y":2,"":3}"#);
        let fmt = cs("{s:i!}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 199 — "{s:i*}" is explicitly non-strict
    diff("row199/{s:i*}/non-strict", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"b":2}"#);
        let fmt = cs("{s:i*}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 200 — JSON_STRICT with every key consumed
    diff("row200/STRICT/{s:i}/satisfied", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("{s:i}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 201 — JSON_STRICT promotes a bare container to '!'
    diff("row201/STRICT/{s:i}/promoted rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"bee":2}"#);
        let fmt = cs("{s:i}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // JSON_STRICT is inherited by nested containers too
    diff("row201/STRICT/nested rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":{"x":1,"y":2}}"#);
        let fmt = cs("{s:{s:i}}");
        let (ka, kx) = (cs("a"), cs("x"));
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), ka.as_ptr(), kx.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 202 — an explicit '*' suppresses JSON_STRICT
    diff("row202/STRICT+{s:i*}/suppressed", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"b":2}"#);
        let fmt = cs("{s:i*}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // an explicit '!' under JSON_STRICT is the same as STRICT alone
    diff("row202/STRICT+{s:i!}", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"b":2}"#);
        let fmt = cs("{s:i!}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 203 — '!'/'*' must be the last thing in the container
    for (label, fmt_text) in [
        ("row203/{s:i!x} rejected", "{s:i!x}"),
        ("row203/{s:i*x} rejected", "{s:i*x}"),
        ("row203/{s:i!s:i} rejected", "{s:i!s:i}"),
        ("row203/{!s:i} rejected", "{!s:i}"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, r#"{"a":1,"b":2}"#);
            let fmt = cs(fmt_text);
            let (ka, kb) = (cs("a"), cs("b"));
            let mut e = json_error_t::new();
            let mut x: c_int = 0;
            let mut y: c_int = 0;
            let ret =
                f(r, &mut e, 0, fmt.as_ptr(), ka.as_ptr(), &mut x, kb.as_ptr(), &mut y);
            decref(lib, r);
            (ret, e.snapshot(), x, y)
        });
    }
    // unterminated object format
    diff("row203/{s:i/unterminated rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("{s:i");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
}

// ---------------------------------------------------------------- rows 204-209

#[test]
fn rows204_209_unpack_arrays() {
    // row 204 — "[i,i]" on a 2-element array
    diff("row204/[i,i]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[10,-20]");
        let fmt = cs("[i,i]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut a, &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });
    diff("row204/[i,i]/json_unpack", |lib| unsafe {
        let f = fp_unpack(lib);
        let r = root(lib, "[10,-20]");
        let fmt = cs("[i,i]");
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, fmt.as_ptr(), &mut a, &mut b);
        decref(lib, r);
        (ret, a, b)
    });

    // row 205 — "[]" on an empty array, and on a non-empty one
    diff("row205/[]/on-empty", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[]");
        let fmt = cs("[]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row205/[]/on-non-empty", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1]");
        let fmt = cs("[]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row205/[!]/on-non-empty rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2]");
        let fmt = cs("[!]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 206 — "[i!]" on a longer array
    diff("row206/[i!]/2-left-unpacked rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2,3]");
        let fmt = cs("[i!]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut a);
        decref(lib, r);
        (ret, e.snapshot(), a)
    });

    // row 207 — "[i*]" on a longer array
    diff("row207/[i*]/non-strict", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2,3]");
        let fmt = cs("[i*]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut a);
        decref(lib, r);
        (ret, e.snapshot(), a)
    });

    // row 208 — JSON_STRICT with arrays
    diff("row208/STRICT/[i,i]/satisfied", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2]");
        let fmt = cs("[i,i]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), &mut a, &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });
    diff("row208/STRICT/[i,i]/promoted rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2,3]");
        let fmt = cs("[i,i]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), &mut a, &mut b);
        decref(lib, r);
        (ret, e.snapshot(), a, b)
    });
    diff("row208/STRICT/[i*]/suppressed", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2,3]");
        let fmt = cs("[i*]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr(), &mut a);
        decref(lib, r);
        (ret, e.snapshot(), a)
    });
    diff("row208/STRICT/[]/on-empty", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[]");
        let fmt = cs("[]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_STRICT, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 209 — index past the end of the array
    diff("row209/[i,i,i]/index-out-of-range rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2]");
        let fmt = cs("[i,i,i]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        let mut c: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut a, &mut b, &mut c);
        decref(lib, r);
        (ret, e.snapshot(), a, b, c)
    });
    diff("row209/[i]/on-empty rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[]");
        let fmt = cs("[i]");
        let mut e = json_error_t::new();
        let mut a: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut a);
        decref(lib, r);
        (ret, e.snapshot(), a)
    });
}

// ---------------------------------------------------------------- rows 210-212

#[test]
fn rows210_212_unpack_array_value_starters() {
    // row 210 — every member of unpack_value_starters = "{[siIbfFOon"
    diff("row210/[s]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"["x"]"#);
        let fmt = cs("[s]");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });
    diff("row210/[i]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[7]");
        let fmt = cs("[i]");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row210/[I]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[7]");
        let fmt = cs("[I]");
        let mut e = json_error_t::new();
        let mut out: c_longlong = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row210/[b]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[true]");
        let fmt = cs("[b]");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row210/[f]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1.5]");
        let fmt = cs("[f]");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    for (label, text) in [("row210/[F]/real", "[1.5]"), ("row210/[F]/integer", "[7]")] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, text);
            let fmt = cs("[F]");
            let mut e = json_error_t::new();
            let mut out: f64 = -1.0;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out.to_bits())
        });
    }
    diff("row210/[o]", |lib| unsafe {
        let aget: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[7]");
        let expect = aget(r, 0);
        let rc0 = (*expect).refcount;
        let fmt = cs("[o]");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        let rc1 = (*expect).refcount;
        let same = out == expect;
        let dump = dump_ref(lib, out);
        decref(lib, r);
        (ret, e.snapshot(), rc0, rc1, same, dump)
    });
    diff("row210/[O]", |lib| unsafe {
        let aget: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[7]");
        let expect = aget(r, 0);
        let rc0 = (*expect).refcount;
        let fmt = cs("[O]");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        let rc1 = (*expect).refcount;
        let same = out == expect;
        let dump = dump_ref(lib, out);
        decref(lib, r);
        let rc2 = (*out).refcount;
        decref(lib, out);
        (ret, e.snapshot(), rc0, rc1, rc2, same, dump)
    });
    diff("row210/[n]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[null]");
        let fmt = cs("[n]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row210/[{s:i}]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"[{"a":1}]"#);
        let fmt = cs("[{s:i}]");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row210/[[i]]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[[9]]");
        let fmt = cs("[[i]]");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 211 — '?' is only handled inside objects
    diff("row211/[?i] rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1]");
        let fmt = cs("[?i]");
        let mut e = json_error_t::new();
        let mut out: c_int = 0x5A5A_5A5A;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });

    // row 212 — '%' and '#' are not value starters
    for (label, fmt_text) in [
        ("row212/[%] rejected", "[%]"),
        ("row212/[#] rejected", "[#]"),
        ("row212/[r] rejected", "[r]"),
        ("row212/[x] rejected", "[x]"),
        ("row212/[+] rejected", "[+]"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, "[1]");
            let fmt = cs(fmt_text);
            let mut e = json_error_t::new();
            let mut out: c_int = 0x5A5A_5A5A;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out)
        });
    }
    // "[s%]" — '%' after an 's' inside an array IS accepted (it is consumed by
    // the 's' case, not by unpack_array's value-starter check).
    diff("row212/[s%]/accepted", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"["abc"]"#);
        let fmt = cs("[s%]");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let mut len: usize = usize::MAX;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out, &mut len);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got, len)
    });
}

// ---------------------------------------------------------------- rows 213-217

#[test]
fn rows213_217_unpack_flags_and_rejected_chars() {
    // row 213 — JSON_VALIDATE_ONLY consumes object KEYS but no value targets.
    diff("row213/VALIDATE_ONLY/{s:s,s:i}/no-value-args", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":"x","b":1}"#);
        let fmt = cs("{s:s,s:i}");
        let (ka, kb) = (cs("a"), cs("b"));
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    // still type-checks
    diff("row213/VALIDATE_ONLY/type-mismatch rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1,"b":1}"#);
        let fmt = cs("{s:s,s:i}");
        let (ka, kb) = (cs("a"), cs("b"));
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    // arrays consume nothing at all under VALIDATE_ONLY
    diff("row213/VALIDATE_ONLY/[i,s,b,f,F,o,O,n]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"[1,"x",true,1.5,2,3,4,null]"#);
        let fmt = cs("[i,s,b,f,F,o,O,n]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    // 'O' under VALIDATE_ONLY must NOT incref
    diff("row213/VALIDATE_ONLY/[O]/no-incref", |lib| unsafe {
        let aget: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"[{"a":1}]"#);
        let expect = aget(r, 0);
        let rc0 = (*expect).refcount;
        let fmt = cs("[O]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr());
        let rc1 = (*expect).refcount;
        decref(lib, r);
        (ret, e.snapshot(), rc0, rc1)
    });

    // row 214 — VALIDATE_ONLY leaves the '%' unconsumed, so the object loop then
    // sees '%' where it expects 's'.
    diff("row214/VALIDATE_ONLY/{s:s%} rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"k":"hello"}"#);
        let fmt = cs("{s:s%}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr(), k.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    // same at top level: '%' becomes garbage after the format string
    diff("row214/VALIDATE_ONLY/s% rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#""hello""#);
        let fmt = cs("s%");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    // and inside an array
    diff("row214/VALIDATE_ONLY/[s%] rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"["hello"]"#);
        let fmt = cs("[s%]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 215 — JSON_VALIDATE_ONLY | JSON_STRICT
    diff("row215/VALIDATE_ONLY|STRICT/satisfied", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":"x","b":1}"#);
        let fmt = cs("{s:s,s:i}");
        let (ka, kb) = (cs("a"), cs("b"));
        let mut e = json_error_t::new();
        let ret = f(
            r,
            &mut e,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            fmt.as_ptr(),
            ka.as_ptr(),
            kb.as_ptr(),
        );
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row215/VALIDATE_ONLY|STRICT/leftover rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":"x","b":1,"cee":2}"#);
        let fmt = cs("{s:s,s:i}");
        let (ka, kb) = (cs("a"), cs("b"));
        let mut e = json_error_t::new();
        let ret = f(
            r,
            &mut e,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            fmt.as_ptr(),
            ka.as_ptr(),
            kb.as_ptr(),
        );
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row215/VALIDATE_ONLY|STRICT/array-leftover rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2,3]");
        let fmt = cs("[i,i]");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, JSON_VALIDATE_ONLY | JSON_STRICT, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 216 — '#' is pack-only
    for (label, fmt_text) in
        [("row216/{s:s#} rejected", "{s:s#}"), ("row216/{s:s+} rejected", "{s:s+}")]
    {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, r#"{"k":"hello"}"#);
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            let mut out: *const c_char = ptr::null();
            let mut extra: c_int = 0;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out, &mut extra);
            let got = opt_str(out);
            decref(lib, r);
            (ret, e.snapshot(), got)
        });
    }
    diff("row216/s# rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#""hello""#);
        let fmt = cs("s#");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let mut extra: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out, &mut extra);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });

    // row 217 — 'r' does not exist in 2.15
    for (label, fmt_text, in_obj) in [
        ("row217/{s:r} rejected", "{s:r}", true),
        ("row217/[r] rejected", "[r]", false),
        ("row217/r rejected", "r", false),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, if in_obj { r#"{"k":1}"# } else { "[1]" });
            let fmt = cs(fmt_text);
            let k = cs("k");
            let mut e = json_error_t::new();
            let mut out: c_int = 0x5A5A_5A5A;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out)
        });
    }
}

// ---------------------------------------------------------------- rows 218-219

#[test]
fn rows218_219_unpack_roots() {
    // row 218 — a root scalar needs no container in the format.
    diff("row218/root-scalar/i", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "-5");
        let fmt = cs("i");
        let mut e = json_error_t::new();
        let mut out: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row218/root-scalar/I", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "-5");
        let fmt = cs("I");
        let mut e = json_error_t::new();
        let mut out: c_longlong = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    diff("row218/root-scalar/s", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#""str""#);
        let fmt = cs("s");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });
    diff("row218/root-scalar/s%", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#""str""#);
        let fmt = cs("s%");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let mut len: usize = usize::MAX;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out, &mut len);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got, len)
    });
    for (label, text) in
        [("row218/root-scalar/b/true", "true"), ("row218/root-scalar/b/false", "false")]
    {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, text);
            let fmt = cs("b");
            let mut e = json_error_t::new();
            let mut out: c_int = 0x5A5A_5A5A;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out)
        });
    }
    diff("row218/root-scalar/n", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "null");
        let fmt = cs("n");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row218/root-scalar/f", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "2.5");
        let fmt = cs("f");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    diff("row218/root-scalar/F", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "3");
        let fmt = cs("F");
        let mut e = json_error_t::new();
        let mut out: f64 = -1.0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        decref(lib, r);
        (ret, e.snapshot(), out.to_bits())
    });
    diff("row218/root-scalar/o", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "42");
        let rc0 = (*r).refcount;
        let fmt = cs("o");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        let rc1 = (*r).refcount;
        let same = out == r;
        let dump = dump_ref(lib, out);
        decref(lib, r);
        (ret, e.snapshot(), rc0, rc1, same, dump)
    });
    diff("row218/root-scalar/O", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "42");
        let rc0 = (*r).refcount;
        let fmt = cs("O");
        let mut e = json_error_t::new();
        let mut out: *mut json_t = ptr::null_mut();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
        let rc1 = (*r).refcount;
        let same = out == r;
        let dump = dump_ref(lib, out);
        decref(lib, out);
        decref(lib, r);
        (ret, e.snapshot(), rc0, rc1, same, dump)
    });

    // row 219 — wrong root type for '{' / '[': the message embeds type_names[].
    for (label, text) in [
        ("row219/{-on-array rejected", "[1]"),
        ("row219/{-on-string rejected", r#""s""#),
        ("row219/{-on-integer rejected", "1"),
        ("row219/{-on-real rejected", "1.5"),
        ("row219/{-on-true rejected", "true"),
        ("row219/{-on-false rejected", "false"),
        ("row219/{-on-null rejected", "null"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, text);
            let fmt = cs("{s:i}");
            let k = cs("k");
            let mut e = json_error_t::new();
            let mut out: c_int = 0x5A5A_5A5A;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out)
        });
    }
    for (label, text) in [
        ("row219/[-on-object rejected", r#"{"a":1}"#),
        ("row219/[-on-string rejected", r#""s""#),
        ("row219/[-on-integer rejected", "1"),
        ("row219/[-on-real rejected", "1.5"),
        ("row219/[-on-true rejected", "true"),
        ("row219/[-on-false rejected", "false"),
        ("row219/[-on-null rejected", "null"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, text);
            let fmt = cs("[i]");
            let mut e = json_error_t::new();
            let mut out: c_int = 0x5A5A_5A5A;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut out);
            decref(lib, r);
            (ret, e.snapshot(), out)
        });
    }
    // scalar formats against every wrong root type
    for (label, fmt_text) in [
        ("row219/s-wrong-type", "s"),
        ("row219/i-wrong-type", "i"),
        ("row219/I-wrong-type", "I"),
        ("row219/b-wrong-type", "b"),
        ("row219/f-wrong-type", "f"),
        ("row219/F-wrong-type", "F"),
        ("row219/n-wrong-type", "n"),
    ] {
        for text in
            [r#"{"a":1}"#, "[1]", r#""s""#, "1", "1.5", "true", "false", "null"]
        {
            diff(&format!("{}/on-{}", label, text), move |lib| unsafe {
                let f = fp_unpack_ex(lib);
                let r = root(lib, text);
                let fmt = cs(fmt_text);
                let mut e = json_error_t::new();
                const SENTINEL: u64 = 0x5A5A_5A5A_5A5A_5A5A;
                let mut slot: [u64; 4] = [SENTINEL; 4];
                let ret = f(r, &mut e, 0, fmt.as_ptr(), slot.as_mut_ptr());
                decref(lib, r);
                // 's' writes a pointer into slot[0], which necessarily differs
                // between the two libraries, so only compare *whether* each
                // slot was written (the concrete values are pinned by row 218).
                let written = slot.map(|w| w != SENTINEL);
                (ret, e.snapshot(), written)
            });
        }
    }
}

// ---------------------------------------------------------------- rows 220-223

#[test]
fn rows220_223_unpack_decoration_null_root_and_rehash() {
    // row 220 — decoration characters in the format string
    for (label, fmt_text) in [
        ("row220/spaces", " { s : i , s : i } "),
        ("row220/tabs", "\t{\ts\t:\ti\t,\ts\t:\ti\t}\t"),
        ("row220/newlines", "\n{\ns\n:\ni\n,\ns\n:\ni\n}\n"),
        ("row220/commas-colons", ",:{,:s,:i,:,:s,:i,:},:"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, r#"{"a":1,"b":2}"#);
            let fmt = cs(fmt_text);
            let (ka, kb) = (cs("a"), cs("b"));
            let mut e = json_error_t::new();
            let mut x: c_int = 0;
            let mut y: c_int = 0;
            let ret =
                f(r, &mut e, 0, fmt.as_ptr(), ka.as_ptr(), &mut x, kb.as_ptr(), &mut y);
            decref(lib, r);
            (ret, e.snapshot(), x, y)
        });
    }
    // decoration around the '!' and inside arrays (line/column bookkeeping)
    diff("row220/newlines-before-error", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("\n\n{\n s : q }");
        let k = cs("a");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row220/[ i , i ]", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, "[1,2]");
        let fmt = cs("[ i , i ]");
        let mut e = json_error_t::new();
        let mut x: c_int = 0;
        let mut y: c_int = 0;
        let ret = f(r, &mut e, 0, fmt.as_ptr(), &mut x, &mut y);
        decref(lib, r);
        (ret, e.snapshot(), x, y)
    });

    // row 221 — garbage after the format string
    for (label, fmt_text) in [
        ("row221/{s:i}x rejected", "{s:i}x"),
        ("row221/[i]] rejected", "[i]]"),
        ("row221/ii rejected", "ii"),
    ] {
        diff(label, move |lib| unsafe {
            let f = fp_unpack_ex(lib);
            let r = root(lib, r#"{"a":1}"#);
            let fmt = cs(fmt_text);
            let k = cs("a");
            let mut e = json_error_t::new();
            let mut x: c_int = 0;
            let mut y: c_int = 0;
            let ret = f(r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut x, &mut y);
            decref(lib, r);
            (ret, e.snapshot(), x, y)
        });
    }

    // row 222 — root == NULL, fmt == NULL, fmt == "" (through json_unpack_ex).
    diff("row222/root=NULL rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let fmt = cs("{s:i}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut x: c_int = 0;
        let ret = f(ptr::null_mut(), &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut x);
        (ret, e.snapshot(), x)
    });
    diff("row222/fmt=NULL rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, ptr::null());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row222/fmt=\"\" rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("");
        let mut e = json_error_t::new();
        let ret = f(r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });
    // root == NULL is checked BEFORE fmt
    diff("row222/root=NULL+fmt=NULL rejected", |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let mut e = json_error_t::new();
        let ret = f(ptr::null_mut(), &mut e, 0, ptr::null());
        (ret, e.snapshot())
    });
    diff("row222/json_unpack/root=NULL rejected", |lib| unsafe {
        let f = fp_unpack(lib);
        let fmt = cs("{s:i}");
        let k = cs("a");
        let mut x: c_int = 0;
        f(ptr::null_mut(), fmt.as_ptr(), k.as_ptr(), &mut x)
    });

    // row 223 — 12 keys under '!' makes the internal key_set hashtable rehash
    // (INITIAL_HASHTABLE_ORDER 3 => the 9th key grows it to order 4).
    let root_text = {
        let mut s = String::from("{");
        for i in 0..12 {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!("\"k{:02}\":{}", i, i));
        }
        s.push('}');
        s
    };
    let rt = root_text.clone();
    diff("row223/12-keys-under-!/satisfied", move |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, &rt);
        let fmt = cs(&obj_fmt(12, "!"));
        let k = keys(12);
        let mut out = [0 as c_int; 12];
        let o = out.as_mut_ptr();
        let mut e = json_error_t::new();
        let ret = f(
            r,
            &mut e,
            0,
            fmt.as_ptr(),
            k[0].as_ptr(),
            o.add(0),
            k[1].as_ptr(),
            o.add(1),
            k[2].as_ptr(),
            o.add(2),
            k[3].as_ptr(),
            o.add(3),
            k[4].as_ptr(),
            o.add(4),
            k[5].as_ptr(),
            o.add(5),
            k[6].as_ptr(),
            o.add(6),
            k[7].as_ptr(),
            o.add(7),
            k[8].as_ptr(),
            o.add(8),
            k[9].as_ptr(),
            o.add(9),
            k[10].as_ptr(),
            o.add(10),
            k[11].as_ptr(),
            o.add(11),
        );
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    let rt2 = root_text.clone();
    diff("row223/11-of-12-keys-under-! rejected", move |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, &rt2);
        let fmt = cs(&obj_fmt(11, "!"));
        let k = keys(12);
        let mut out = [0 as c_int; 12];
        let o = out.as_mut_ptr();
        let mut e = json_error_t::new();
        let ret = f(
            r,
            &mut e,
            0,
            fmt.as_ptr(),
            k[0].as_ptr(),
            o.add(0),
            k[1].as_ptr(),
            o.add(1),
            k[2].as_ptr(),
            o.add(2),
            k[3].as_ptr(),
            o.add(3),
            k[4].as_ptr(),
            o.add(4),
            k[5].as_ptr(),
            o.add(5),
            k[6].as_ptr(),
            o.add(6),
            k[7].as_ptr(),
            o.add(7),
            k[8].as_ptr(),
            o.add(8),
            k[9].as_ptr(),
            o.add(9),
            k[10].as_ptr(),
            o.add(10),
        );
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
    // same shape with JSON_STRICT instead of an explicit '!'
    let rt3 = root_text.clone();
    diff("row223/11-of-12-keys-under-STRICT rejected", move |lib| unsafe {
        let f = fp_unpack_ex(lib);
        let r = root(lib, &rt3);
        let fmt = cs(&obj_fmt(11, ""));
        let k = keys(12);
        let mut out = [0 as c_int; 12];
        let o = out.as_mut_ptr();
        let mut e = json_error_t::new();
        let ret = f(
            r,
            &mut e,
            JSON_STRICT,
            fmt.as_ptr(),
            k[0].as_ptr(),
            o.add(0),
            k[1].as_ptr(),
            o.add(1),
            k[2].as_ptr(),
            o.add(2),
            k[3].as_ptr(),
            o.add(3),
            k[4].as_ptr(),
            o.add(4),
            k[5].as_ptr(),
            o.add(5),
            k[6].as_ptr(),
            o.add(6),
            k[7].as_ptr(),
            o.add(7),
            k[8].as_ptr(),
            o.add(8),
            k[9].as_ptr(),
            o.add(9),
            k[10].as_ptr(),
            o.add(10),
        );
        decref(lib, r);
        (ret, e.snapshot(), out)
    });
}

// ---------------------------------------------------------------- v* entry points
// rows 139 / 179 / 184 / 222 through json_vpack_ex, json_vunpack_ex, json_vsprintf

#[test]
fn v_entry_points_rows139_179_184_222() {
    // row 139/140 — json_vpack_ex with containers
    for (label, fmt_text) in [
        ("row139/{}/json_vpack_ex", "{}"),
        ("row140/[]/json_vpack_ex", "[]"),
        ("row164/n/json_vpack_ex", "n"),
    ] {
        diff(label, move |lib| unsafe {
            let f = *sym::<VPackEx>(lib, "json_vpack_ex");
            let fmt = cs(fmt_text);
            let mut e = json_error_t::new();
            let v = via_vpack_ex(f, &mut e, 0, fmt.as_ptr());
            pk_fin(lib, v, &e)
        });
    }
    // json_vpack_ex with real varargs
    diff("row141/{s:i,s:s,s:f}/json_vpack_ex", |lib| unsafe {
        let f = *sym::<VPackEx>(lib, "json_vpack_ex");
        let fmt = cs("{s:i,s:s,s:f}");
        let (ka, kb, kc, sv) = (cs("a"), cs("b"), cs("c"), cs("v"));
        let mut e = json_error_t::new();
        let v = via_vpack_ex(
            f,
            &mut e,
            0,
            fmt.as_ptr(),
            ka.as_ptr(),
            7 as c_int,
            kb.as_ptr(),
            sv.as_ptr(),
            kc.as_ptr(),
            1.5f64,
        );
        pk_fin(lib, v, &e)
    });
    diff("row170/[O,O]/json_vpack_ex", |lib| unsafe {
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let f = *sym::<VPackEx>(lib, "json_vpack_ex");
        let fmt = cs("[O,O]");
        let src = int(5);
        let mut e = json_error_t::new();
        let v = via_vpack_ex(f, &mut e, 0, fmt.as_ptr(), src, src);
        let rc = (*src).refcount;
        let dump = dump_ref(lib, v);
        decref(lib, v);
        let rc2 = (*src).refcount;
        decref(lib, src);
        (rc, rc2, dump, e.snapshot())
    });

    // row 179 — json_vpack_ex with fmt == NULL / ""
    diff("row179/json_vpack_ex/fmt=NULL rejected", |lib| unsafe {
        let f = *sym::<VPackEx>(lib, "json_vpack_ex");
        let mut e = json_error_t::new();
        let v = via_vpack_ex(f, &mut e, 0, ptr::null());
        pk_fin(lib, v, &e)
    });
    diff("row179/json_vpack_ex/fmt=\"\" rejected", |lib| unsafe {
        let f = *sym::<VPackEx>(lib, "json_vpack_ex");
        let fmt = cs("");
        let mut e = json_error_t::new();
        let v = via_vpack_ex(f, &mut e, 0, fmt.as_ptr());
        pk_fin(lib, v, &e)
    });
    diff("row179/json_vpack_ex/error=NULL rejected", |lib| unsafe {
        let f = *sym::<VPackEx>(lib, "json_vpack_ex");
        let v = via_vpack_ex(f, ptr::null_mut(), 0, ptr::null());
        pk_noerr(lib, v)
    });

    // row 184 — json_vunpack_ex on "{s:s}" plus a couple of value shapes
    diff("row184/{s:s}/json_vunpack_ex", |lib| unsafe {
        let f = *sym::<VUnpackEx>(lib, "json_vunpack_ex");
        let r = root(lib, r#"{"k":"v"}"#);
        let fmt = cs("{s:s}");
        let k = cs("k");
        let mut e = json_error_t::new();
        let mut out: *const c_char = ptr::null();
        let ret = via_vunpack_ex(f, r, &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut out);
        let got = opt_str(out);
        decref(lib, r);
        (ret, e.snapshot(), got)
    });
    diff("row184/{s:i,s:f,s:s%}/json_vunpack_ex", |lib| unsafe {
        let f = *sym::<VUnpackEx>(lib, "json_vunpack_ex");
        let r = root(lib, r#"{"a":3,"b":1.5,"c":"xyz"}"#);
        let fmt = cs("{s:i,s:f,s:s%}");
        let (ka, kb, kc) = (cs("a"), cs("b"), cs("c"));
        let mut e = json_error_t::new();
        let mut i: c_int = 0;
        let mut d: f64 = -1.0;
        let mut s: *const c_char = ptr::null();
        let mut len: usize = usize::MAX;
        let ret = via_vunpack_ex(
            f,
            r,
            &mut e,
            0,
            fmt.as_ptr(),
            ka.as_ptr(),
            &mut i,
            kb.as_ptr(),
            &mut d,
            kc.as_ptr(),
            &mut s,
            &mut len,
        );
        let got = opt_str(s);
        decref(lib, r);
        (ret, e.snapshot(), i, d.to_bits(), got, len)
    });
    diff("row196/{}/json_vunpack_ex", |lib| unsafe {
        let f = *sym::<VUnpackEx>(lib, "json_vunpack_ex");
        let r = root(lib, "{}");
        let fmt = cs("{}");
        let mut e = json_error_t::new();
        let ret = via_vunpack_ex(f, r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 222 — json_vunpack_ex with root == NULL / fmt == NULL / fmt == ""
    diff("row222/json_vunpack_ex/root=NULL rejected", |lib| unsafe {
        let f = *sym::<VUnpackEx>(lib, "json_vunpack_ex");
        let fmt = cs("{s:i}");
        let k = cs("a");
        let mut e = json_error_t::new();
        let mut x: c_int = 0;
        let ret =
            via_vunpack_ex(f, ptr::null_mut(), &mut e, 0, fmt.as_ptr(), k.as_ptr(), &mut x);
        (ret, e.snapshot(), x)
    });
    diff("row222/json_vunpack_ex/fmt=NULL rejected", |lib| unsafe {
        let f = *sym::<VUnpackEx>(lib, "json_vunpack_ex");
        let r = root(lib, r#"{"a":1}"#);
        let mut e = json_error_t::new();
        let ret = via_vunpack_ex(f, r, &mut e, 0, ptr::null());
        decref(lib, r);
        (ret, e.snapshot())
    });
    diff("row222/json_vunpack_ex/fmt=\"\" rejected", |lib| unsafe {
        let f = *sym::<VUnpackEx>(lib, "json_vunpack_ex");
        let r = root(lib, r#"{"a":1}"#);
        let fmt = cs("");
        let mut e = json_error_t::new();
        let ret = via_vunpack_ex(f, r, &mut e, 0, fmt.as_ptr());
        decref(lib, r);
        (ret, e.snapshot())
    });

    // row 181/182/183 — json_vsprintf
    diff("row181/json_vsprintf/%s-%d", |lib| unsafe {
        let f = *sym::<VSprintf>(lib, "json_vsprintf");
        let fmt = cs("%s-%d");
        let s = cs("abc");
        pk_noerr(lib, via_vsprintf(f, fmt.as_ptr(), s.as_ptr(), 42 as c_int))
    });
    diff("row182/json_vsprintf/empty", |lib| unsafe {
        let f = *sym::<VSprintf>(lib, "json_vsprintf");
        let fmt = cs("");
        pk_noerr(lib, via_vsprintf(f, fmt.as_ptr()))
    });
    diff("row183/json_vsprintf/utf8", |lib| unsafe {
        let f = *sym::<VSprintf>(lib, "json_vsprintf");
        let fmt = cs("%s\u{20ac}%s");
        let (a, b) = (cs("h\u{e9}"), cs("\u{4e2d}"));
        pk_noerr(lib, via_vsprintf(f, fmt.as_ptr(), a.as_ptr(), b.as_ptr()))
    });
    diff("row183/json_vsprintf/invalid-utf8 rejected", |lib| unsafe {
        let f = *sym::<VSprintf>(lib, "json_vsprintf");
        let fmt = cs("%s");
        let bad = cs_bytes(&[0xff, 0xfe]);
        pk_noerr(lib, via_vsprintf(f, fmt.as_ptr(), bad.as_ptr() as *const c_char))
    });
}

// ---------------------------------------------------------------- randomized

/// Pack/dump/reload/dump one randomly shaped format per iteration. Each template
/// needs its own call site because the vararg WIDTHS differ per format character
/// (`i`/`b` = int, `I` = long long, `f` = double in an SSE register, `s` = pointer).
unsafe fn pk_roundtrip(lib: &Library, v: *mut json_t, e: &json_error_t) -> String {
    if v.is_null() {
        return format!("NULL err={:?}", e.snapshot());
    }
    let d = dumps_to_string(lib, v, DUMP);
    decref(lib, v);
    let (rt, rterr) = match &d {
        Some(s) => load_then_dump(
            lib,
            s.as_bytes(),
            JSON_DECODE_ANY | JSON_ALLOW_NUL,
            DUMP,
        ),
        None => (None, e.snapshot()),
    };
    format!("dump={:?} rt={:?} rterr={:?} err={:?}", d, rt, rterr, e.snapshot())
}

fn nice_double(rng: &mut Rng) -> f64 {
    match rng.below(10) {
        0 => 0.0,
        1 => -0.0,
        2 => 1.5,
        3 => -2.25,
        4 => 1e10,
        5 => 1e-10,
        6 => f64::NAN,      // exercises the "Invalid floating point value" path
        7 => f64::INFINITY, // ditto
        8 => (rng.i64() % 100_000) as f64 / 4.0,
        _ => (rng.i64() % 1_000) as f64,
    }
}

#[test]
fn rand_pack_roundtrip() {
    diff_n("randA/pack-roundtrip", 300, |lib, it| unsafe {
        let mut rng = Rng::new(0xC0FF_EE00_0000_0001u64 ^ it);
        let f = fp_pack_ex(lib);
        let mut e = json_error_t::new();

        let k0 = cs(&rng.ascii_string(6));
        let k1 = cs(&rng.ascii_string(6));
        let k2 = cs(&rng.ascii_string(6));
        let k3 = cs(&rng.ascii_string(6));
        let k4 = cs(&rng.ascii_string(6));
        let sv0 = cs(&rng.utf8_string(5));
        let sv1 = cs(&rng.utf8_string(5));
        let i0 = (rng.i64() % 1_000_000) as c_int;
        let l0 = rng.i64() as c_longlong;
        let b0 = rng.below(2) as c_int;
        let d0 = nice_double(&mut rng);
        let n0 = sv0.as_bytes().len();
        let n1 = sv1.as_bytes().len();
        let len0 = rng.below(n0 as u64 + 1) as c_int;
        let len1 = rng.below(n1 as u64 + 1) as usize;
        let p0 = if rng.below(3) == 0 { ptr::null() } else { sv0.as_ptr() };
        let p1 = if rng.below(3) == 0 { ptr::null() } else { sv1.as_ptr() };
        // flags are inert for packing; vary them anyway
        let flags = [0usize, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT]
            [rng.below(4) as usize];

        let t = rng.below(10);
        let body = match t {
            0 => {
                let fmt = cs("{s:i}");
                let v = f(&mut e, flags, fmt.as_ptr(), k0.as_ptr(), i0);
                pk_roundtrip(lib, v, &e)
            }
            1 => {
                let fmt = cs("{s:s}");
                let v = f(&mut e, flags, fmt.as_ptr(), k0.as_ptr(), sv0.as_ptr());
                pk_roundtrip(lib, v, &e)
            }
            2 => {
                let fmt = cs("{s:f}");
                let v = f(&mut e, flags, fmt.as_ptr(), k0.as_ptr(), d0);
                pk_roundtrip(lib, v, &e)
            }
            3 => {
                let fmt = cs("{s:i,s:s}");
                let v = f(
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    k0.as_ptr(),
                    i0,
                    k1.as_ptr(),
                    sv0.as_ptr(),
                );
                pk_roundtrip(lib, v, &e)
            }
            4 => {
                let fmt = cs("{s:s,s:i,s:f}");
                let v = f(
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    k0.as_ptr(),
                    sv0.as_ptr(),
                    k1.as_ptr(),
                    i0,
                    k2.as_ptr(),
                    d0,
                );
                pk_roundtrip(lib, v, &e)
            }
            5 => {
                let fmt = cs("{s:i,s:I,s:b,s:f,s:s}");
                let v = f(
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    k0.as_ptr(),
                    i0,
                    k1.as_ptr(),
                    l0,
                    k2.as_ptr(),
                    b0,
                    k3.as_ptr(),
                    d0,
                    k4.as_ptr(),
                    sv0.as_ptr(),
                );
                pk_roundtrip(lib, v, &e)
            }
            6 => {
                let fmt = cs("[i,s,f,n,b]");
                let v = f(&mut e, flags, fmt.as_ptr(), i0, sv0.as_ptr(), d0, b0);
                pk_roundtrip(lib, v, &e)
            }
            7 => {
                let fmt = cs("{s:[i,s],s:{s:f}}");
                let v = f(
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    k0.as_ptr(),
                    i0,
                    sv0.as_ptr(),
                    k1.as_ptr(),
                    k2.as_ptr(),
                    d0,
                );
                pk_roundtrip(lib, v, &e)
            }
            8 => {
                let fmt = cs("{s:s#,s:s%}");
                let v = f(
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    k0.as_ptr(),
                    sv0.as_ptr(),
                    len0,
                    k1.as_ptr(),
                    sv1.as_ptr(),
                    len1,
                );
                pk_roundtrip(lib, v, &e)
            }
            _ => {
                let fmt = cs("{s:s?,s:s*,s:O?}");
                let v = f(
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    k0.as_ptr(),
                    p0,
                    k1.as_ptr(),
                    p1,
                    k2.as_ptr(),
                    ptr::null_mut::<json_t>(),
                );
                pk_roundtrip(lib, v, &e)
            }
        };
        format!("t={} flags={:#x} {}", t, flags, body)
    });
}

/// Randomly generated object (or array / scalar) unpacked with one of a set of
/// fixed formats and flag combinations, comparing return code, error struct and
/// every out-parameter.
fn rand_literal(rng: &mut Rng) -> String {
    match rng.below(9) {
        0 => format!("{}", rng.i64() % 100_000),
        1 => "\"str\"".to_string(),
        2 => "\"\"".to_string(),
        3 => "1.5".to_string(),
        4 => "true".to_string(),
        5 => "false".to_string(),
        6 => "null".to_string(),
        7 => "[1,2]".to_string(),
        _ => "{\"x\":1}".to_string(),
    }
}

#[test]
fn rand_unpack_objects() {
    diff_n("randB/unpack-objects", 300, |lib, it| unsafe {
        let mut rng = Rng::new(0xBEEF_0000_0000_0001u64 ^ it);
        let f = fp_unpack_ex(lib);

        // Build the root text deterministically from the seed.
        let text = if rng.below(12) == 0 {
            "[1,2]".to_string()
        } else if rng.below(12) == 0 {
            "7".to_string()
        } else {
            let mut s = String::from("{");
            let mut first = true;
            for k in ["a", "b", "c", "z"] {
                if rng.below(5) == 0 {
                    continue; // sometimes omit the key entirely
                }
                if !first {
                    s.push(',');
                }
                first = false;
                s.push_str(&format!("\"{}\":{}", k, rand_literal(&mut rng)));
            }
            s.push('}');
            s
        };
        // JSON_VALIDATE_ONLY suppresses the va_arg consumption of every value
        // target, so it needs its own key-only argument lists — passing targets
        // anyway would misalign the vararg list and make the library read
        // uninitialised test stack.
        let validate = rng.below(4) == 0;
        let flags = if validate {
            JSON_VALIDATE_ONLY | [0usize, JSON_STRICT][rng.below(2) as usize]
        } else {
            [0usize, JSON_STRICT][rng.below(2) as usize]
        };
        let t = rng.below(8);

        let loads: Symbol<FnLoads> = sym(lib, "json_loads");
        let ct = cs(&text);
        let mut le = json_error_t::new();
        let r = loads(ct.as_ptr(), JSON_DECODE_ANY, &mut le);
        assert!(!r.is_null(), "fixture {:?} failed to parse", text);

        let (ka, kb, kc) = (cs("a"), cs("b"), cs("c"));
        let mut e = json_error_t::new();
        let mut i0: c_int = 0x5A5A_5A5A;
        let mut i1: c_int = 0x3C3C_3C3C;
        let mut d0: f64 = -1.0;
        let mut s0: *const c_char = ptr::null();
        let mut len0: usize = usize::MAX;
        let mut o0: *mut json_t = ptr::null_mut();

        let ret = if validate {
            match t {
                0 => {
                    let fmt = cs("{s:i,s:s,s:f}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr())
                }
                1 => {
                    let fmt = cs("{s:i!}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr())
                }
                2 => {
                    let fmt = cs("{s?i,s:s}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr())
                }
                3 => {
                    let fmt = cs("{s:F,s:O}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr())
                }
                4 => {
                    let fmt = cs("{s:n,s:b}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr())
                }
                5 => {
                    let fmt = cs("[i,i]");
                    f(r, &mut e, flags, fmt.as_ptr())
                }
                6 => {
                    // '%' is never consumed under VALIDATE_ONLY => rejected
                    let fmt = cs("{s:s%}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr())
                }
                _ => {
                    let fmt = cs("{s:i*}");
                    f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr())
                }
            }
        } else {
            match t {
            0 => {
                let fmt = cs("{s:i,s:s,s:f}");
                f(
                    r,
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    ka.as_ptr(),
                    &mut i0,
                    kb.as_ptr(),
                    &mut s0,
                    kc.as_ptr(),
                    &mut d0,
                )
            }
            1 => {
                let fmt = cs("{s:i!}");
                f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), &mut i0)
            }
            2 => {
                let fmt = cs("{s?i,s:s}");
                f(
                    r,
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    ka.as_ptr(),
                    &mut i0,
                    kb.as_ptr(),
                    &mut s0,
                )
            }
            3 => {
                let fmt = cs("{s:F,s:O}");
                f(
                    r,
                    &mut e,
                    flags,
                    fmt.as_ptr(),
                    ka.as_ptr(),
                    &mut d0,
                    kb.as_ptr(),
                    &mut o0,
                )
            }
            4 => {
                let fmt = cs("{s:n,s:b}");
                f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), kb.as_ptr(), &mut i0)
            }
            5 => {
                let fmt = cs("[i,i]");
                f(r, &mut e, flags, fmt.as_ptr(), &mut i0, &mut i1)
            }
            6 => {
                let fmt = cs("{s:s%}");
                f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), &mut s0, &mut len0)
            }
            _ => {
                let fmt = cs("{s:i*}");
                f(r, &mut e, flags, fmt.as_ptr(), ka.as_ptr(), &mut i0)
            }
            }
        };

        let o_dump = dump_ref(lib, o0);
        let o_rc = if o0.is_null() { 0 } else { (*o0).refcount };
        let s_got = opt_str(s0);
        let s_bytes = if len0 == usize::MAX { None } else { str_bytes(s0, len0) };
        if !o0.is_null() {
            decref(lib, o0); // 'O' handed us an owned reference
        }
        decref(lib, r);

        format!(
            "text={} t={} flags={:#x} ret={} err={:?} i0={} i1={} d0={:#x} s={:?} len={} sb={:?} o={:?} orc={}",
            text,
            t,
            flags,
            ret,
            e.snapshot(),
            i0,
            i1,
            d0.to_bits(),
            s_got,
            len0,
            s_bytes,
            o_dump,
            o_rc
        )
    });
}
