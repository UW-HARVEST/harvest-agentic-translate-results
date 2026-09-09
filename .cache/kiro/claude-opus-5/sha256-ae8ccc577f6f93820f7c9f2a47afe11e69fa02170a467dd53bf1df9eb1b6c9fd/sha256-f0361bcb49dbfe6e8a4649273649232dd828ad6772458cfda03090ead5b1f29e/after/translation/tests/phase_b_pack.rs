//! Phase B — pack / unpack (CONFIGS rows 68-80).
#![allow(unused_unsafe, dead_code, unsafe_op_in_unsafe_fn)]
mod common;
use common::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_longlong, c_void};

type PackEx =
    unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut JsonT;
type Pack = unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT;
type UnpackEx =
    unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char, ...) -> c_int;
type Unpack = unsafe extern "C" fn(*mut JsonT, *const c_char, ...) -> c_int;

unsafe fn pack_ex(l: &Lib) -> PackEx {
    let s: libloading::Symbol<PackEx> = l.lib.get(b"json_pack_ex\0").unwrap();
    *s
}
unsafe fn pack_fn(l: &Lib) -> Pack {
    let s: libloading::Symbol<Pack> = l.lib.get(b"json_pack\0").unwrap();
    *s
}
unsafe fn unpack_ex(l: &Lib) -> UnpackEx {
    let s: libloading::Symbol<UnpackEx> = l.lib.get(b"json_unpack_ex\0").unwrap();
    *s
}
unsafe fn unpack_fn(l: &Lib) -> Unpack {
    let s: libloading::Symbol<Unpack> = l.lib.get(b"json_unpack\0").unwrap();
    *s
}

const ANY: usize = JSON_ENCODE_ANY | JSON_SORT_KEYS;

/// Run one `json_pack_ex` scenario on both libraries and compare the resulting
/// value (dumped) plus the whole `json_error_t`.
fn diff_pack<F>(name: &str, flags: usize, body: F)
where
    F: Fn(&Lib, PackEx, *mut JsonError, usize) -> *mut JsonT,
{
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        let mut e = JsonError::new();
        e.source = [0x7f; ERR_SRC_LEN];
        e.text = [0x7f; ERR_TEXT_LEN];
        unsafe {
            let f = pack_ex(l);
            let j = body(l, f, &mut e, flags);
            let d = if j.is_null() {
                None
            } else {
                let d = dumps(l, j, ANY);
                decref(l, j);
                d
            };
            out.push((d, e));
        }
    }
    assert_bytes_eq(
        &format!("pack {} flags={:#x}: value differs", name, flags),
        &out[0].0,
        &out[1].0,
    );
    assert_err_eq(
        &format!("pack {} flags={:#x}: error differs", name, flags),
        &out[0].1,
        &out[1].1,
    );
}

/// Run one `json_unpack_ex` scenario on both libraries.  `body` returns a
/// printable rendering of every out-parameter it received.
fn diff_unpack<B, F>(name: &str, flags: usize, build_root: B, body: F)
where
    B: Fn(&Lib) -> *mut JsonT,
    F: Fn(&Lib, UnpackEx, *mut JsonT, *mut JsonError, usize) -> String,
{
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        let mut e = JsonError::new();
        e.source = [0x7f; ERR_SRC_LEN];
        e.text = [0x7f; ERR_TEXT_LEN];
        unsafe {
            let root = build_root(l);
            let f = unpack_ex(l);
            let rendered = body(l, f, root, &mut e, flags);
            let dumped = if root.is_null() {
                None
            } else {
                dumps(l, root, ANY)
            };
            if !root.is_null() {
                decref(l, root);
            }
            out.push((rendered, e, dumped));
        }
    }
    assert_eq!(
        out[0].0, out[1].0,
        "unpack {} flags={:#x}: outputs differ",
        name, flags
    );
    assert_err_eq(
        &format!("unpack {} flags={:#x}: error differs", name, flags),
        &out[0].1,
        &out[1].1,
    );
    assert_bytes_eq(
        &format!("unpack {} flags={:#x}: root mutated differently", name, flags),
        &out[0].2,
        &out[1].2,
    );
}

fn from_text(text: &'static str) -> impl Fn(&Lib) -> *mut JsonT {
    move |l: &Lib| unsafe {
        let (j, _) = loads(l, text.as_bytes(), JSON_DECODE_ANY);
        j
    }
}

unsafe fn cstr_out(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".into()
    } else {
        format!("{:?}", CStr::from_ptr(p).to_bytes())
    }
}

// ------------------------------------------------------------- rows 68-73

#[test]
fn row68_pack_single_char_formats() {
    for flags in [0usize, 1, 2, 3, 0xFF] {
        diff_pack("{}", flags, |_, f, e, fl| unsafe {
            let fmt = CString::new("{}").unwrap();
            f(e, fl, fmt.as_ptr())
        });
        diff_pack("[]", flags, |_, f, e, fl| unsafe {
            let fmt = CString::new("[]").unwrap();
            f(e, fl, fmt.as_ptr())
        });
        diff_pack("n", flags, |_, f, e, fl| unsafe {
            let fmt = CString::new("n").unwrap();
            f(e, fl, fmt.as_ptr())
        });
        for b in [0i32, 1, -1, 2, 1000] {
            diff_pack("b", flags, move |_, f, e, fl| unsafe {
                let fmt = CString::new("b").unwrap();
                f(e, fl, fmt.as_ptr(), b)
            });
        }
        for i in [0i32, 1, -1, i32::MAX, i32::MIN] {
            diff_pack("i", flags, move |_, f, e, fl| unsafe {
                let fmt = CString::new("i").unwrap();
                f(e, fl, fmt.as_ptr(), i)
            });
        }
        for i in [0i64, 1, -1, i64::MAX, i64::MIN, 1 << 40] {
            diff_pack("I", flags, move |_, f, e, fl| unsafe {
                let fmt = CString::new("I").unwrap();
                f(e, fl, fmt.as_ptr(), i as c_longlong)
            });
        }
        for v in [0.0f64, -0.0, 1.5, 1e300, 1e-300, -1.0 / 3.0] {
            diff_pack("f", flags, move |_, f, e, fl| unsafe {
                let fmt = CString::new("f").unwrap();
                f(e, fl, fmt.as_ptr(), v)
            });
        }
        for s in ["", "a", "hello", "caf\u{e9}", "\u{1f600}"] {
            diff_pack("s", flags, move |_, f, e, fl| unsafe {
                let fmt = CString::new("s").unwrap();
                let a = CString::new(s).unwrap();
                f(e, fl, fmt.as_ptr(), a.as_ptr())
            });
        }
        // o / O with a freshly built child
        diff_pack("o", flags, |l, f, e, fl| unsafe {
            let fmt = CString::new("[o]").unwrap();
            let child = sym!(l, "json_integer", (i64) -> *mut JsonT)(7);
            f(e, fl, fmt.as_ptr(), child)
        });
        diff_pack("O", flags, |l, f, e, fl| unsafe {
            let fmt = CString::new("[O]").unwrap();
            let child = sym!(l, "json_integer", (i64) -> *mut JsonT)(7);
            let v = f(e, fl, fmt.as_ptr(), child);
            decref(l, child);
            v
        });
    }
}

#[test]
fn row69_pack_string_modifiers() {
    let flags = 0usize;
    for (fmt, s, n) in [
        ("s#", "hello", 0usize),
        ("s#", "hello", 1),
        ("s#", "hello", 5),
        ("s#", "", 0),
        ("s#", "caf\u{e9}xx", 4),
        ("s#", "caf\u{e9}xx", 3),
    ] {
        diff_pack(fmt, flags, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let cs = CString::new(s).unwrap();
            f(e, fl, cf.as_ptr(), cs.as_ptr(), n as c_int)
        });
    }
    for (s, n) in [("hello", 0usize), ("hello", 3), ("hello", 5), ("", 0)] {
        diff_pack("s%", flags, move |_, f, e, fl| unsafe {
            let cf = CString::new("s%").unwrap();
            let cs = CString::new(s).unwrap();
            f(e, fl, cf.as_ptr(), cs.as_ptr(), n)
        });
    }
    for (a, b) in [("he", "llo"), ("", ""), ("caf", "\u{e9}"), ("x", "")] {
        diff_pack("s+", flags, move |_, f, e, fl| unsafe {
            let cf = CString::new("s+").unwrap();
            let ca = CString::new(a).unwrap();
            let cb = CString::new(b).unwrap();
            f(e, fl, cf.as_ptr(), ca.as_ptr(), cb.as_ptr())
        });
    }
    diff_pack("s++", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s++").unwrap();
        let a = CString::new("a").unwrap();
        let b = CString::new("bb").unwrap();
        let c = CString::new("ccc").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr())
    });
    diff_pack("s+#", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s+#").unwrap();
        let a = CString::new("abc").unwrap();
        let b = CString::new("defgh").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), b.as_ptr(), 2 as c_int)
    });
    diff_pack("s#+", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s#+").unwrap();
        let a = CString::new("abcdef").unwrap();
        let b = CString::new("xy").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), 2 as c_int, b.as_ptr())
    });
    diff_pack("s+%", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s+%").unwrap();
        let a = CString::new("abc").unwrap();
        let b = CString::new("defgh").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), b.as_ptr(), 3usize)
    });
    // invalid UTF-8 through s and s#
    for bad in [b"\x80".as_ref(), b"a\xC0\x80".as_ref(), b"\xED\xA0\x80".as_ref()] {
        let owned = bad.to_vec();
        diff_pack("s invalid utf8", flags, move |_, f, e, fl| unsafe {
            let cf = CString::new("s").unwrap();
            let cs = CString::new(owned.clone()).unwrap();
            f(e, fl, cf.as_ptr(), cs.as_ptr())
        });
    }
}

#[test]
fn row70_pack_optional() {
    let flags = 0usize;
    // s? with NULL -> json_null
    diff_pack("s? NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s?").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>())
    });
    diff_pack("s? value", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s?").unwrap();
        let a = CString::new("v").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr())
    });
    diff_pack("s* NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("s*").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>())
    });
    // inside object: value omitted with *, null with ?
    diff_pack("{s:s*} NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:s*, s:i}").unwrap();
        let k1 = CString::new("a").unwrap();
        let k2 = CString::new("b").unwrap();
        f(
            e,
            fl,
            cf.as_ptr(),
            k1.as_ptr(),
            std::ptr::null::<c_char>(),
            k2.as_ptr(),
            1 as c_int,
        )
    });
    diff_pack("{s:s?} NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:s?, s:i}").unwrap();
        let k1 = CString::new("a").unwrap();
        let k2 = CString::new("b").unwrap();
        f(
            e,
            fl,
            cf.as_ptr(),
            k1.as_ptr(),
            std::ptr::null::<c_char>(),
            k2.as_ptr(),
            1 as c_int,
        )
    });
    diff_pack("[s*, i] NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("[s*, i]").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>(), 5 as c_int)
    });
    diff_pack("[s?, i] NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("[s?, i]").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>(), 5 as c_int)
    });
    diff_pack("o? NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("o?").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<JsonT>())
    });
    diff_pack("o* NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("[o*, i]").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<JsonT>(), 3 as c_int)
    });
    diff_pack("O? NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("O?").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<JsonT>())
    });
    diff_pack("{s:o*} NULL", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:o*, s:i}").unwrap();
        let k1 = CString::new("a").unwrap();
        let k2 = CString::new("b").unwrap();
        f(
            e,
            fl,
            cf.as_ptr(),
            k1.as_ptr(),
            std::ptr::null::<JsonT>(),
            k2.as_ptr(),
            2 as c_int,
        )
    });
}

#[test]
fn row71_row72_pack_composite() {
    let flags = 0usize;
    diff_pack("[i,i,i]", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("[i,i,i]").unwrap();
        f(e, fl, cf.as_ptr(), 1 as c_int, 2 as c_int, 3 as c_int)
    });
    diff_pack("{s:i}", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:i}").unwrap();
        let k = CString::new("k").unwrap();
        f(e, fl, cf.as_ptr(), k.as_ptr(), 9 as c_int)
    });
    diff_pack("{s:[i,i],s:{s:s}}", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:[i,i],s:{s:s}}").unwrap();
        let a = CString::new("arr").unwrap();
        let b = CString::new("obj").unwrap();
        let c = CString::new("in").unwrap();
        let d = CString::new("v").unwrap();
        f(
            e,
            fl,
            cf.as_ptr(),
            a.as_ptr(),
            1 as c_int,
            2 as c_int,
            b.as_ptr(),
            c.as_ptr(),
            d.as_ptr(),
        )
    });
    // whitespace / separators must be ignored
    for fmt in [
        "{s:i}",
        " { s : i } ",
        "{ s: i }",
        "{\ns\t:\ni\n}",
        "{s , : i}",
    ] {
        diff_pack(fmt, flags, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let k = CString::new("k").unwrap();
            f(e, fl, cf.as_ptr(), k.as_ptr(), 9 as c_int)
        });
    }
    diff_pack("deep", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("[[[[{s:[{s:i}]}]]]]").unwrap();
        let a = CString::new("x").unwrap();
        let b = CString::new("y").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), b.as_ptr(), 5 as c_int)
    });
    // duplicate keys in one pack
    diff_pack("{s:i,s:i} dup", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:i,s:i}").unwrap();
        let k = CString::new("k").unwrap();
        f(e, fl, cf.as_ptr(), k.as_ptr(), 1 as c_int, k.as_ptr(), 2 as c_int)
    });
    // keys with embedded modifiers
    diff_pack("{s#:i}", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s#:i}").unwrap();
        let k = CString::new("keyXX").unwrap();
        f(e, fl, cf.as_ptr(), k.as_ptr(), 3 as c_int, 7 as c_int)
    });
    diff_pack("{s%:i}", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s%:i}").unwrap();
        let k = CString::new("keyXX").unwrap();
        f(e, fl, cf.as_ptr(), k.as_ptr(), 3usize, 7 as c_int)
    });
    diff_pack("{s+:i}", flags, |_, f, e, fl| unsafe {
        let cf = CString::new("{s+:i}").unwrap();
        let a = CString::new("ke").unwrap();
        let b = CString::new("y").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), b.as_ptr(), 7 as c_int)
    });
}

#[test]
fn row73_json_pack_no_error_arg() {
    let (c, r) = both();
    for (fmt, kind) in [
        ("{}", 0),
        ("[]", 0),
        ("n", 0),
        ("[i,i]", 1),
        ("{s:i}", 2),
        ("s", 3),
        ("x", 0),
        ("", 0),
    ] {
        let mut out = Vec::new();
        for l in [c, r] {
            unsafe {
                let f = pack_fn(l);
                let cf = CString::new(fmt).unwrap();
                let k = CString::new("k").unwrap();
                let s = CString::new("v").unwrap();
                let j = match kind {
                    1 => f(cf.as_ptr(), 1 as c_int, 2 as c_int),
                    2 => f(cf.as_ptr(), k.as_ptr(), 5 as c_int),
                    3 => f(cf.as_ptr(), s.as_ptr()),
                    _ => f(cf.as_ptr()),
                };
                let d = if j.is_null() {
                    None
                } else {
                    let d = dumps(l, j, ANY);
                    decref(l, j);
                    d
                };
                out.push(d);
            }
        }
        assert_bytes_eq(&format!("json_pack({:?})", fmt), &out[0], &out[1]);
    }
}

// ------------------------------------------------------------- rows 74-80

#[test]
fn row74_unpack_scalars() {
    for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
        // s
        for root in ["\"abc\"", "\"\"", "1", "null", "true", "[]", "{}", "1.5"] {
            diff_unpack("s", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("s").unwrap();
                let mut p: *const c_char = 0x1 as *const c_char;
                let rc = f(root, e, fl, cf.as_ptr(), &mut p);
                format!("rc={} s={}", rc, if p as usize == 1 { "<untouched>".into() } else { cstr_out(p) })
            });
            diff_unpack("s%", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("s%").unwrap();
                let mut p: *const c_char = std::ptr::null();
                let mut n: usize = 0xDEAD;
                let rc = f(root, e, fl, cf.as_ptr(), &mut p, &mut n);
                format!("rc={} s={} n={}", rc, cstr_out(p), n)
            });
            diff_unpack("i", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("i").unwrap();
                let mut v: c_int = -12345;
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                format!("rc={} i={}", rc, v)
            });
            diff_unpack("I", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("I").unwrap();
                let mut v: c_longlong = -12345;
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                format!("rc={} I={}", rc, v)
            });
            diff_unpack("b", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("b").unwrap();
                let mut v: c_int = -12345;
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                format!("rc={} b={}", rc, v)
            });
            diff_unpack("f", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("f").unwrap();
                let mut v: c_double = -1.0;
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                format!("rc={} f={}", rc, v.to_bits())
            });
            diff_unpack("F", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("F").unwrap();
                let mut v: c_double = -1.0;
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                format!("rc={} F={}", rc, v.to_bits())
            });
            diff_unpack("n", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("n").unwrap();
                let rc = f(root, e, fl, cf.as_ptr());
                format!("rc={}", rc)
            });
            diff_unpack("o", flags, from_text(root), |l, f, root, e, fl| unsafe {
                let cf = CString::new("o").unwrap();
                let mut v: *mut JsonT = std::ptr::null_mut();
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                let d = if v.is_null() { None } else { dumps(l, v, ANY) };
                format!("rc={} o={:?}", rc, d)
            });
            diff_unpack("O", flags, from_text(root), |l, f, root, e, fl| unsafe {
                let cf = CString::new("O").unwrap();
                let mut v: *mut JsonT = std::ptr::null_mut();
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                let d = if v.is_null() {
                    None
                } else {
                    let d = dumps(l, v, ANY);
                    // `O` increfs; give the reference back so refcounts stay comparable
                    decref(l, v);
                    d
                };
                format!("rc={} O={:?}", rc, d)
            });
        }
    }
}

#[test]
fn row75_row76_row77_row78_unpack_containers() {
    for flags in [
        0usize,
        JSON_STRICT,
        JSON_VALIDATE_ONLY,
        JSON_STRICT | JSON_VALIDATE_ONLY,
    ] {
        for root in [
            "{}",
            "{\"a\":1}",
            "{\"a\":1,\"b\":2}",
            "{\"a\":1,\"b\":\"s\"}",
            "{\"b\":2}",
            "[]",
            "[1]",
            "[1,2]",
            "[1,2,3]",
            "[1,\"s\"]",
            "1",
            "null",
        ] {
            diff_unpack("{}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{}").unwrap();
                format!("rc={}", f(root, e, fl, cf.as_ptr()))
            });
            diff_unpack("{s:i}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:i}").unwrap();
                let k = CString::new("a").unwrap();
                let mut v: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut v);
                format!("rc={} a={}", rc, v)
            });
            diff_unpack("{s?i}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s?i}").unwrap();
                let k = CString::new("a").unwrap();
                let mut v: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut v);
                format!("rc={} a={}", rc, v)
            });
            diff_unpack("{s:i!}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:i!}").unwrap();
                let k = CString::new("a").unwrap();
                let mut v: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut v);
                format!("rc={} a={}", rc, v)
            });
            diff_unpack("{s:i*}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:i*}").unwrap();
                let k = CString::new("a").unwrap();
                let mut v: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut v);
                format!("rc={} a={}", rc, v)
            });
            // NOTE: under JSON_VALIDATE_ONLY the C API consumes the KEY
            // va_args but not the VALUE ones, so a multi-key format must be
            // called with keys only.  Passing value pointers there would
            // desynchronise the va_list (undefined behaviour), so the two
            // shapes are exercised separately.
            if flags & JSON_VALIDATE_ONLY == 0 {
                diff_unpack("{s:i,s:i}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                    let cf = CString::new("{s:i,s:i}").unwrap();
                    let k1 = CString::new("a").unwrap();
                    let k2 = CString::new("b").unwrap();
                    let mut v1: c_int = -1;
                    let mut v2: c_int = -1;
                    let rc = f(root, e, fl, cf.as_ptr(), k1.as_ptr(), &mut v1, k2.as_ptr(), &mut v2);
                    format!("rc={} a={} b={}", rc, v1, v2)
                });
            } else {
                diff_unpack("{s:i,s:i} keys-only", flags, from_text(root), |_, f, root, e, fl| unsafe {
                    let cf = CString::new("{s:i,s:i}").unwrap();
                    let k1 = CString::new("a").unwrap();
                    let k2 = CString::new("b").unwrap();
                    let rc = f(root, e, fl, cf.as_ptr(), k1.as_ptr(), k2.as_ptr());
                    format!("rc={}", rc)
                });
                diff_unpack("{s:i,s:i!} keys-only", flags, from_text(root), |_, f, root, e, fl| unsafe {
                    let cf = CString::new("{s:i,s:i!}").unwrap();
                    let k1 = CString::new("a").unwrap();
                    let k2 = CString::new("b").unwrap();
                    let rc = f(root, e, fl, cf.as_ptr(), k1.as_ptr(), k2.as_ptr());
                    format!("rc={}", rc)
                });
                diff_unpack("{s:s,s?i} keys-only", flags, from_text(root), |_, f, root, e, fl| unsafe {
                    let cf = CString::new("{s:s,s?i}").unwrap();
                    let k1 = CString::new("b").unwrap();
                    let k2 = CString::new("a").unwrap();
                    let rc = f(root, e, fl, cf.as_ptr(), k1.as_ptr(), k2.as_ptr());
                    format!("rc={}", rc)
                });
            }
            diff_unpack("[i]", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("[i]").unwrap();
                let mut v: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), &mut v);
                format!("rc={} 0={}", rc, v)
            });
            diff_unpack("[i,i]", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("[i,i]").unwrap();
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), &mut a, &mut b);
                format!("rc={} {} {}", rc, a, b)
            });
            diff_unpack("[i,i!]", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("[i,i!]").unwrap();
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), &mut a, &mut b);
                format!("rc={} {} {}", rc, a, b)
            });
            diff_unpack("[i,i*]", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("[i,i*]").unwrap();
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let rc = f(root, e, fl, cf.as_ptr(), &mut a, &mut b);
                format!("rc={} {} {}", rc, a, b)
            });
            diff_unpack("[]", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("[]").unwrap();
                format!("rc={}", f(root, e, fl, cf.as_ptr()))
            });
        }
    }
}

#[test]
fn row79_unpack_string_length_and_optional() {
    for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
        for root in ["{\"a\":\"xyz\"}", "{}", "{\"a\":1}", "{\"a\":null}"] {
            diff_unpack("{s:s%}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:s%}").unwrap();
                let k = CString::new("a").unwrap();
                let mut p: *const c_char = std::ptr::null();
                let mut n: usize = 0xDEAD;
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut p, &mut n);
                format!("rc={} s={} n={}", rc, cstr_out(p), n)
            });
            diff_unpack("{s?s}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s?s}").unwrap();
                let k = CString::new("a").unwrap();
                let mut p: *const c_char = std::ptr::null();
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut p);
                format!("rc={} s={}", rc, cstr_out(p))
            });
            diff_unpack("{s?s%}", flags, from_text(root), |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s?s%}").unwrap();
                let k = CString::new("a").unwrap();
                let mut p: *const c_char = std::ptr::null();
                let mut n: usize = 0xDEAD;
                let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut p, &mut n);
                format!("rc={} s={} n={}", rc, cstr_out(p), n)
            });
        }
    }
    // NUL-containing string value: s% must report the real length
    diff_unpack(
        "{s:s%} with NUL",
        0,
        |l| unsafe {
            let (j, _) = loads(l, b"{\"a\":\"x\\u0000y\"}", JSON_ALLOW_NUL);
            j
        },
        |_, f, root, e, fl| unsafe {
            let cf = CString::new("{s:s%}").unwrap();
            let k = CString::new("a").unwrap();
            let mut p: *const c_char = std::ptr::null();
            let mut n: usize = 0;
            let rc = f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut p, &mut n);
            let raw = if p.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(p as *const u8, n).to_vec()
            };
            format!("rc={} n={} raw={:02x?}", rc, n, raw)
        },
    );
}

#[test]
fn row80_json_unpack_no_error_arg() {
    let (c, r) = both();
    for (root, fmt, kind) in [
        ("{\"a\":1}", "{s:i}", 1),
        ("{\"a\":1}", "{s:i!}", 1),
        ("[1,2]", "[i,i]", 2),
        ("[1,2]", "[i]", 3),
        ("1", "i", 3),
        ("\"s\"", "s", 4),
        ("{}", "x", 0),
        ("{}", "", 0),
    ] {
        let mut out = Vec::new();
        for l in [c, r] {
            unsafe {
                let (j, _) = loads(l, root.as_bytes(), JSON_DECODE_ANY);
                let f = unpack_fn(l);
                let cf = CString::new(fmt).unwrap();
                let k = CString::new("a").unwrap();
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let mut p: *const c_char = std::ptr::null();
                let rc = match kind {
                    1 => f(j, cf.as_ptr(), k.as_ptr(), &mut a),
                    2 => f(j, cf.as_ptr(), &mut a, &mut b),
                    3 => f(j, cf.as_ptr(), &mut a),
                    4 => f(j, cf.as_ptr(), &mut p),
                    _ => f(j, cf.as_ptr()),
                };
                let s = if p.is_null() {
                    "<null>".to_string()
                } else {
                    cstr_out(p)
                };
                out.push(format!("rc={} a={} b={} s={}", rc, a, b, s));
                if !j.is_null() {
                    decref(l, j);
                }
            }
        }
        assert_eq!(out[0], out[1], "json_unpack({:?},{:?})", root, fmt);
    }
}
