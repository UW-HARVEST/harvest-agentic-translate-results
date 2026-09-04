//! Phase C — ERRORS.md rows 146–162 (`dump.c`).
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

unsafe fn forge(api: &Api, ty: c_int) -> Jt {
    unsafe {
        let p = (api.jsonp_malloc)(64) as *mut JsonT;
        (*p).type_ = ty;
        (*p).refcount = 1;
        p
    }
}

/* rows 146,148,149: json_dump_callback / do_dump type gate */

#[test]
fn e_rows_146_149_encode_any_gate_and_bad_types() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            // row 146: scalars (and NULL) rejected without JSON_ENCODE_ANY
            let scalars: Vec<Jt> = vec![
                std::ptr::null_mut(),
                (api.json_true)(),
                (api.json_false)(),
                (api.json_null)(),
                (api.json_integer)(1),
                (api.json_real)(1.0),
                (api.json_string)(cstr("s").as_ptr()),
            ];
            for (i, j) in scalars.iter().enumerate() {
                out.push(format!(
                    "{i} dumps={:?} dumpb={} dumpf_ret={} dump_cb={}",
                    dumps(api, *j, 0),
                    (api.json_dumpb)(*j, std::ptr::null_mut(), 0, 0),
                    (api.json_dumpfd)(*j, -1, 0),
                    (api.json_dump_callback)(*j, None, std::ptr::null_mut(), 0)
                ));
            }
            for (i, j) in scalars.iter().enumerate().skip(1) {
                decref(api, *j);
                let _ = i;
            }
            // row 148: NULL json even with JSON_ENCODE_ANY
            out.push(format!(
                "NULL+ANY dumps={:?} dumpb={} dump_cb={}",
                dumps(api, std::ptr::null_mut(), JSON_ENCODE_ANY),
                (api.json_dumpb)(std::ptr::null_mut(), std::ptr::null_mut(), 0, JSON_ENCODE_ANY),
                (api.json_dump_callback)(
                    std::ptr::null_mut(),
                    None,
                    std::ptr::null_mut(),
                    JSON_ENCODE_ANY
                )
            ));
            // row 149: out-of-range type byte, with and without JSON_ENCODE_ANY
            for ty in [8i32, 9, 100, 255, -1, i32::MAX, i32::MIN] {
                let f = forge(api, ty);
                out.push(format!(
                    "ty={ty} plain={:?} any={:?} cb_any={}",
                    dumps(api, f, 0),
                    dumps(api, f, JSON_ENCODE_ANY),
                    (api.json_dump_callback)(f, None, std::ptr::null_mut(), JSON_ENCODE_ANY)
                ));
                (api.jsonp_free)(f as *mut c_void);
            }
            // and nested inside a valid container
            for ty in [8i32, 255] {
                let f = forge(api, ty);
                let a = (api.json_array)();
                (api.json_array_append_new)(a, f);
                out.push(format!("nested ty={ty} dumps={:?}", dumps(api, a, 0)));
                // remove it again so json_delete never sees the forged value
                (api.json_array_remove)(a, 0);
                decref(api, a);
                let o = (api.json_object)();
                let f2 = forge(api, ty);
                (api.json_object_set_new)(o, cstr("k").as_ptr(), f2);
                out.push(format!("nested-obj ty={ty} dumps={:?}", dumps(api, o, 0)));
                (api.json_object_del)(o, cstr("k").as_ptr());
                decref(api, o);
                (api.jsonp_free)(f as *mut c_void);
                (api.jsonp_free)(f2 as *mut c_void);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "dump gate row {i}");
    }
}

/* row 150: jsonp_dtostr failure inside do_dump (real needs > 25 bytes) */

#[test]
fn e_row_150_real_does_not_fit() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x150);
    let mut vals: Vec<f64> = vec![
        1.7976931348623157e308,
        -1.7976931348623157e308,
        5e-324,
        -5e-324,
        1.2345678901234567e-300,
        -1.2345678901234567e-300,
        0.1234567890123456789,
    ];
    for _ in 0..500 {
        vals.push(rng.finite_f64());
    }
    unsafe {
        for v in vals {
            let jc = (p.c.json_real)(v);
            let jr = (p.r.json_real)(v);
            assert_eq!(jc.is_null(), jr.is_null());
            if jc.is_null() {
                continue;
            }
            for prec in 0..=31usize {
                let f = JSON_ENCODE_ANY | json_real_precision(prec);
                let a = dumps(p.c, jc, f);
                let b = dumps(p.r, jr, f);
                assert_eq!(a, b, "real {v:?} bits={:#x} prec={prec}", v.to_bits());
                // and inside an array, where the failure must propagate as -1
                let ac = (p.c.json_array)();
                let ar = (p.r.json_array)();
                (p.c.json_array_append_new)(ac, incref(p.c, jc));
                (p.r.json_array_append_new)(ar, incref(p.r, jr));
                assert_eq!(
                    dumps(p.c, ac, f | json_indent(2)),
                    dumps(p.r, ar, f | json_indent(2)),
                    "nested real {v:?} prec={prec}"
                );
                decref(p.c, ac);
                decref(p.r, ar);
            }
            decref(p.c, jc);
            decref(p.r, jr);
        }
    }
}

/* row 151: circular references */

#[test]
fn e_row_151_circular_references() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            // direct self reference in an array
            let a = (api.json_array)();
            (api.json_array_append_new)(a, incref(api, a));
            for f in [0usize, json_indent(2), JSON_SORT_KEYS, JSON_EMBED] {
                out.push(format!("self-array f={f:#x} {:?}", dumps(api, a, f)));
            }
            (api.json_array_clear)(a);
            decref(api, a);

            // direct self reference in an object
            let o = (api.json_object)();
            (api.json_object_set_new)(o, cstr("me").as_ptr(), incref(api, o));
            for f in [0usize, json_indent(2), JSON_SORT_KEYS] {
                out.push(format!("self-object f={f:#x} {:?}", dumps(api, o, f)));
            }
            (api.json_object_clear)(o);
            decref(api, o);

            // indirect cycle a -> b -> a
            let a = (api.json_array)();
            let b = (api.json_object)();
            (api.json_object_set_new)(b, cstr("back").as_ptr(), incref(api, a));
            (api.json_array_append_new)(a, incref(api, b));
            for f in [0usize, json_indent(1), JSON_SORT_KEYS] {
                out.push(format!("cycle f={f:#x} a={:?} b={:?}", dumps(api, a, f), dumps(api, b, f)));
            }
            (api.json_array_clear)(a);
            (api.json_object_clear)(b);
            decref(api, a);
            decref(api, b);

            // shared (but acyclic) child must still dump fine
            let shared = (api.json_array)();
            (api.json_array_append_new)(shared, (api.json_integer)(1));
            let parent = (api.json_array)();
            (api.json_array_append_new)(parent, incref(api, shared));
            (api.json_array_append_new)(parent, incref(api, shared));
            out.push(format!("shared {:?}", dumps(api, parent, 0)));
            decref(api, parent);
            decref(api, shared);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* row 153: invalid UTF-8 inside a *_nocheck string */

#[test]
fn e_row_153_invalid_utf8_in_string() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            for badb in [
                &b"\xff"[..],
                &b"\x80"[..],
                &b"\xc0\x80"[..],
                &b"\xc2"[..],
                &b"\xed\xa0\x80"[..],
                &b"\xf5\x80\x80\x80"[..],
                &b"ok\xffbad"[..],
                &b"\xe2\x82"[..],
            ] {
                let z = nul_terminated(badb);
                let s = (api.json_stringn_nocheck)(z.as_ptr(), badb.len());
                for f in [
                    JSON_ENCODE_ANY,
                    JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                    JSON_ENCODE_ANY | JSON_ESCAPE_SLASH,
                ] {
                    out.push(format!("{badb:?} f={f:#x} {:?}", dumps(api, s, f)));
                }
                // and as a value inside an array / as a key inside an object
                let a = (api.json_array)();
                (api.json_array_append_new)(a, incref(api, s));
                out.push(format!("{badb:?} in-array {:?}", dumps(api, a, 0)));
                decref(api, a);
                let o = (api.json_object)();
                (api.json_object_setn_new_nocheck)(
                    o,
                    z.as_ptr(),
                    badb.len(),
                    (api.json_integer)(1),
                );
                out.push(format!("{badb:?} as-key {:?}", dumps(api, o, 0)));
                out.push(format!("{badb:?} as-key-sorted {:?}", dumps(api, o, JSON_SORT_KEYS)));
                decref(api, o);
                decref(api, s);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "invalid-utf8 dump row {i}");
    }
}

/* row 154: user callback returning non-zero, at every chunk index */

static mut FAIL_AT: usize = 0;
static mut SEEN: usize = 0;

unsafe extern "C" fn failing(_b: *const c_char, _n: usize, _d: *mut c_void) -> c_int {
    unsafe {
        SEEN += 1;
        if SEEN == FAIL_AT { -7 } else { 0 }
    }
}

#[test]
fn e_row_154_callback_failure_propagates() {
    let _g = lock();
    let p = pair();
    let docs = [
        "[1,2,3]",
        "{\"a\":1,\"b\":[2,3],\"c\":{\"d\":null}}",
        "[]",
        "{}",
        "[\"str\",1.5,true,false,null]",
        "{\"z\":1,\"a\":2}",
    ];
    unsafe {
        for doc in docs {
            for f in [0usize, json_indent(2), JSON_COMPACT, JSON_SORT_KEYS, JSON_EMBED] {
                for n in 1..=40usize {
                    let mut res = Vec::new();
                    for api in [p.c, p.r] {
                        let j = (api.json_loads)(cstr(doc).as_ptr(), 0, std::ptr::null_mut());
                        FAIL_AT = n;
                        SEEN = 0;
                        let r = (api.json_dump_callback)(
                            j,
                            Some(failing),
                            std::ptr::null_mut(),
                            f,
                        );
                        res.push((r, SEEN));
                        decref(api, j);
                    }
                    assert_eq!(res[0], res[1], "callback failure doc={doc} f={f:#x} n={n}");
                }
            }
        }
        FAIL_AT = 0;
    }
}

/* rows 155,157,158: json_dumps / json_dumpb on failure */

#[test]
fn e_rows_155_158_dumps_dumpb_failures() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            // failing input (scalar without ENCODE_ANY, and a cycle)
            let s = (api.json_integer)(1);
            let a = (api.json_array)();
            (api.json_array_append_new)(a, incref(api, a));
            for j in [s, a, std::ptr::null_mut()] {
                out.push(format!(
                    "dumps={:?} dumpb0={} dumpb64={}",
                    dumps(api, j, 0),
                    (api.json_dumpb)(j, std::ptr::null_mut(), 0, 0),
                    {
                        let mut buf = [0x5ai8; 64];
                        let n = (api.json_dumpb)(j, buf.as_mut_ptr(), 64, 0);
                        format!("{n}:{}", buf[0])
                    }
                ));
            }
            (api.json_array_clear)(a);
            decref(api, a);
            decref(api, s);

            // row 158: too-small buffer must NOT be written past `size`
            let j = (api.json_loads)(cstr("[1,2,3,4,5]").as_ptr(), 0, std::ptr::null_mut());
            let need = (api.json_dumpb)(j, std::ptr::null_mut(), 0, 0);
            for size in 0..=need + 2 {
                let mut buf = vec![0x5ai8; need + 8];
                let n = (api.json_dumpb)(j, buf.as_mut_ptr(), size, 0);
                out.push(format!(
                    "size={size} n={n} buf={:?}",
                    buf.iter().map(|&c| c as u8).collect::<Vec<u8>>()
                ));
            }
            decref(api, j);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "dumps/dumpb failure row {i}");
    }
}

/* rows 159,160,161,162: FILE / fd / path sink failures */

#[test]
fn e_rows_159_162_sink_failures() {
    let _g = lock();
    let p = pair();
    let libc = libc();
    unsafe {
        for doc in ["[1,2,3]", "{}", "{\"a\":[1,2]}"] {
            for f in [0usize, json_indent(2), JSON_SORT_KEYS] {
                let mut res = Vec::new();
                for api in [p.c, p.r] {
                    let j = (api.json_loads)(cstr(doc).as_ptr(), 0, std::ptr::null_mut());
                    // row 160: invalid fd
                    let a = (api.json_dumpfd)(j, -1, f);
                    let b = (api.json_dumpfd)(j, 99999, f);
                    // row 159: FILE* opened read-only -> fwrite fails
                    let path = temp_path("ro");
                    std::fs::write(&path, b"x").unwrap();
                    let zp = cstr(path.to_str().unwrap());
                    let fp = (libc.fopen)(zp.as_ptr(), cstr("rb").as_ptr());
                    let c = (api.json_dumpf)(j, fp, f);
                    (libc.fclose)(fp);
                    std::fs::remove_file(&path).ok();
                    // row 161: unwritable path
                    let d = (api.json_dump_file)(
                        j,
                        cstr("/proc/definitely/not/writable/x.json").as_ptr(),
                        f,
                    );
                    let e = (api.json_dump_file)(j, cstr("").as_ptr(), f);
                    // row 162: unwritable path with a value that would also fail
                    let g = (api.json_dump_file)(
                        j,
                        cstr("/proc/definitely/not/writable/x.json").as_ptr(),
                        f,
                    );
                    let h = (api.json_dump_file)(
                        std::ptr::null_mut(),
                        cstr("/proc/definitely/not/writable/x.json").as_ptr(),
                        f,
                    );
                    res.push((a, b, c, d, e, g, h));
                    decref(api, j);
                }
                assert_eq!(res[0], res[1], "sink failures doc={doc} f={f:#x}");
            }
        }
    }
}
