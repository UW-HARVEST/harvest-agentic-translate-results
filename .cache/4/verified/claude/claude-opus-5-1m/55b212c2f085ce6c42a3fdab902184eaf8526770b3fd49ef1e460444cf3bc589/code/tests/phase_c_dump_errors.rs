//! Phase C — error-path differential tests for `dump.c`
//! (ERRORS.md rows 116..137).

mod common;
use common::tree::*;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

static CHUNKS: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
static FAIL_AT: Mutex<i64> = Mutex::new(-1);

unsafe extern "C" fn cb_record(buf: *const c_char, size: usize, _d: *mut c_void) -> c_int {
    let mut g = CHUNKS.lock().unwrap();
    let n = g.len() as i64;
    g.push(std::slice::from_raw_parts(buf as *const u8, size).to_vec());
    let f = *FAIL_AT.lock().unwrap();
    if f >= 0 && n == f {
        -1
    } else {
        0
    }
}

unsafe extern "C" fn cb_always_fail(_b: *const c_char, _s: usize, _d: *mut c_void) -> c_int {
    -1
}

fn cb_reset(fail_at: i64) {
    CHUNKS.lock().unwrap().clear();
    *FAIL_AT.lock().unwrap() = fail_at;
}

fn cb_take() -> Vec<Vec<u8>> {
    std::mem::take(&mut *CHUNKS.lock().unwrap())
}

/* -------------------- rows 116/117: failing FILE* and fd sinks ---------- */

#[test]
fn err116and117_failing_sinks() {
    diff("ERRORS 116-117 failing sinks", |api, rec| unsafe {
        let path = tmp_file("ro_sink");
        std::fs::write(&path, b"x").unwrap();
        let cp = cs(path.to_str().unwrap());
        let ro = cs("r");
        let s = cs(r#"{"a":[1,2,3],"b":"text"}"#);
        let j = (api.json_loads)(s.as_ptr(), 0, ptr::null_mut());
        assert!(!j.is_null());

        for f in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS] {
            // row 116: fwrite on a read-only FILE*
            let fh = fopen(cp.as_ptr(), ro.as_ptr());
            assert!(!fh.is_null());
            let r = (api.json_dumpf)(j, fh, f);
            rec.tag_i("dumpf_ro", r as i64);
            assert_eq!(r, -1, "[{}] row 116", api.tag);
            fclose(fh);

            // row 117: write() on a read-only fd
            {
                use std::os::unix::io::AsRawFd;
                let file = std::fs::File::open(&path).unwrap();
                let r = (api.json_dumpfd)(j, file.as_raw_fd(), f);
                rec.tag_i("dumpfd_ro", r as i64);
                assert_eq!(r, -1, "[{}] row 117 (read-only fd)", api.tag);
            }
            // row 117: an fd that is not open at all
            for fd in [100_000i32, i32::MAX] {
                let r = (api.json_dumpfd)(j, fd, f);
                rec.tag_i("dumpfd_bad", r as i64);
                assert_eq!(r, -1, "[{}] row 117 (bad fd)", api.tag);
            }
        }
        decref(api, j);
        let _ = std::fs::remove_file(&path);
    });
}

/* --------------------- rows 118/120/137: callback failures -------------- */

#[test]
fn err118_120_137_callback_failures() {
    diff("ERRORS 118/120/137 callback failure", |api, rec| unsafe {
        let mut rng = Rng::new(0xC118);
        let mut specs: Vec<Spec> = vec![
            Spec::Arr(vec![]),
            Spec::Obj(vec![]),
            Spec::Arr(vec![Spec::Int(1), Spec::Int(2)]),
            Spec::Obj(vec![
                (b"a".to_vec(), Spec::Str("x".into())),
                (b"b".to_vec(), Spec::Arr(vec![Spec::Real(1.5)])),
            ]),
            Spec::Str("string with \" and \\ and \u{263A}".into()),
            Spec::Int(-12345),
            Spec::Real(1.0 / 3.0),
            Spec::Null,
        ];
        for _ in 0..10 {
            specs.push(rand_container(&mut rng, 2));
        }
        for (si, spec) in specs.iter().enumerate() {
            let j = build(api, spec);
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | json_indent(4),
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | JSON_SORT_KEYS | json_indent(1),
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_EMBED,
            ] {
                // always-failing callback
                rec.tag_i(
                    &format!("s{si}.f{f}.always"),
                    (api.json_dump_callback)(j, Some(cb_always_fail), ptr::null_mut(), f) as i64,
                );
                // failure at each individual chunk
                cb_reset(-1);
                let _ = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                let total = cb_take().len();
                rec.tag_i(&format!("s{si}.f{f}.total"), total as i64);
                for k in 0..total {
                    cb_reset(k as i64);
                    let r = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                    let got = cb_take();
                    rec.tag_i(&format!("s{si}.f{f}.k{k}.ret"), r as i64);
                    rec.tag_i(&format!("s{si}.f{f}.k{k}.n"), got.len() as i64);
                    for (i, c) in got.iter().enumerate() {
                        rec.tag_bytes(&format!("s{si}.f{f}.k{k}.c{i}"), c);
                    }
                    // NB: `do_dump` deliberately ignores the return value of
                    // `dump_string()` for *object keys*, so a failure at one of
                    // those chunks is swallowed and dumping continues with 0.
                    // Both libraries must agree on exactly which chunks are
                    // fatal, which the recorded transcript checks.
                }
            }
            decref(api, j);
        }
    });
}

/* ------------- rows 121/122/133/134/135: json_dump_callback guards ------ */

#[test]
fn err121_122_133to135_guards() {
    diff("ERRORS 121/122/133-135 guards", |api, rec| unsafe {
        // row 133: scalars without JSON_ENCODE_ANY
        for s in [
            Spec::Null,
            Spec::True,
            Spec::False,
            Spec::Int(1),
            Spec::Real(1.0),
            Spec::Str("x".into()),
        ] {
            let j = build(api, &s);
            for f in [0usize, JSON_COMPACT, JSON_SORT_KEYS, json_indent(3), JSON_EMBED] {
                let r = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                cb_take();
                rec.tag_i("scalar_no_any", r as i64);
                assert_eq!(r, -1, "[{}] row 133", api.tag);
                rec.tag_ptr_null("dumps", (api.json_dumps)(j, f) as *const c_void);
                rec.tag_u("dumpb", (api.json_dumpb)(j, ptr::null_mut(), 0, f));
            }
            // with ENCODE_ANY it succeeds
            let r = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), JSON_ENCODE_ANY);
            let got = cb_take();
            rec.tag_i("scalar_any", r as i64);
            for (i, c) in got.iter().enumerate() {
                rec.tag_bytes(&format!("scalar_any.c{i}"), c);
            }
            decref(api, j);
        }

        // rows 121/134/135: json == NULL
        for f in [0usize, JSON_ENCODE_ANY, JSON_ENCODE_ANY | JSON_COMPACT, usize::MAX] {
            let r = (api.json_dump_callback)(ptr::null(), Some(cb_record), ptr::null_mut(), f);
            cb_take();
            rec.tag_i("null_json", r as i64);
            assert_eq!(r, -1, "[{}] rows 121/134/135", api.tag);
            rec.tag_ptr_null("dumps_null", (api.json_dumps)(ptr::null(), f) as *const c_void);
            rec.tag_u("dumpb_null", (api.json_dumpb)(ptr::null(), ptr::null_mut(), 0, f));
            // a NULL callback must not be dereferenced on this path
            rec.tag_i(
                "null_json_null_cb",
                (api.json_dump_callback)(ptr::null(), None, ptr::null_mut(), f) as i64,
            );
        }

        // row 122: out-of-range type tags
        for t in [8, 9, 255, -1, c_int::MIN, c_int::MAX] {
            let p = forge_json(api, t, 1);
            for f in [0usize, JSON_ENCODE_ANY, JSON_ENCODE_ANY | JSON_SORT_KEYS] {
                let r = (api.json_dump_callback)(p, Some(cb_record), ptr::null_mut(), f);
                cb_take();
                rec.tag_i("forged", r as i64);
                assert_eq!(r, -1, "[{}] row 122", api.tag);
                // C never calls the callback on this path either
                rec.tag_i(
                    "forged_null_cb",
                    (api.json_dump_callback)(p, None, ptr::null_mut(), f) as i64,
                );
            }
            (api.jsonp_free)(p as *mut c_void);
        }
    });
}

/* --------------------- row 119: invalid UTF-8 in a string --------------- */

#[test]
fn err119_invalid_utf8_strings() {
    diff("ERRORS 119 invalid UTF-8 dump", |api, rec| unsafe {
        let bad: &[&[u8]] = &[
            b"\x80",
            b"\xff",
            b"\xc0\x80",
            b"\xc2",
            b"\xc2\x41",
            b"\xed\xa0\x80",
            b"\xf5\x80\x80\x80",
            b"ok\xffbad",
            b"\xf0\x90\x80",
        ];
        for b in bad {
            let z = cbuf(b);
            let s = (api.json_stringn_nocheck)(z.as_ptr() as *const c_char, b.len());
            assert!(!s.is_null());
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_COMPACT,
            ] {
                rec.tag_ptr_null("dumps", (api.json_dumps)(s, f) as *const c_void);
                rec.tag_u("dumpb", (api.json_dumpb)(s, ptr::null_mut(), 0, f));
                cb_reset(-1);
                let r = (api.json_dump_callback)(s, Some(cb_record), ptr::null_mut(), f);
                let got = cb_take();
                rec.tag_i("cb_ret", r as i64);
                for (i, c) in got.iter().enumerate() {
                    rec.tag_bytes(&format!("cb.c{i}"), c);
                }
                assert_eq!(r, -1, "[{}] row 119", api.tag);
            }
            // inside a container, and as an object key
            let a = (api.json_array)();
            incref(api, s);
            (api.json_array_append_new)(a, s);
            rec.tag_ptr_null("arr_dumps", (api.json_dumps)(a, 0) as *const c_void);
            decref(api, a);

            let o = (api.json_object)();
            (api.json_object_setn_new_nocheck)(
                o,
                z.as_ptr() as *const c_char,
                b.len(),
                (api.json_integer)(1),
            );
            for f in [0usize, JSON_SORT_KEYS] {
                rec.tag_ptr_null("obj_dumps", (api.json_dumps)(o, f) as *const c_void);
            }
            decref(api, o);
            decref(api, s);
        }
    });
}

/* ------------------- row 123: jsonp_dtostr fails inside do_dump -------- */

#[test]
fn err123_real_precision_overflow() {
    diff("ERRORS 123 dtostr failure", |api, rec| unsafe {
        let vals = [
            1.0f64,
            0.1,
            1.0 / 3.0,
            f64::MAX,
            -f64::MAX,
            5e-324,
            1e300,
            -1.2345678901234567e-300,
            123456789012345678.0,
        ];
        let mut any_fail = false;
        for v in vals {
            let j = (api.json_real)(v);
            for prec in 0..32usize {
                let f = JSON_ENCODE_ANY | json_real_precision(prec);
                let d = (api.json_dumps)(j, f);
                if d.is_null() {
                    any_fail = true;
                    rec.line(&format!("prec{prec}=NULL"));
                } else {
                    rec.cstring(&format!("prec{prec}"), d);
                    (api.jsonp_free)(d as *mut c_void);
                }
                rec.tag_u(&format!("prec{prec}.dumpb"), (api.json_dumpb)(j, ptr::null_mut(), 0, f));
                // inside a container the failure propagates the same way
                let a = (api.json_array)();
                incref(api, j);
                (api.json_array_append_new)(a, j);
                rec.tag_ptr_null(
                    &format!("prec{prec}.arr"),
                    (api.json_dumps)(a, f) as *const c_void,
                );
                decref(api, a);
            }
            decref(api, j);
        }
        assert!(
            any_fail,
            "[{}] row 123: no precision made jsonp_dtostr overflow the 25 byte buffer",
            api.tag
        );
    });
}

/* ----------------- rows 124/125: cycles rejected by every sink --------- */

#[test]
fn err124and125_cycles_every_sink() {
    diff("ERRORS 124-125 cycles", |api, rec| unsafe {
        let pfile = tmp_file("cycle_dump");
        let cpfile = cs(pfile.to_str().unwrap());
        // array cycle
        let a = (api.json_array)();
        let b = (api.json_array)();
        (api.json_array_append_new)(a, b);
        incref(api, a);
        (api.json_array_append_new)(b, a);
        // object cycle
        let o = (api.json_object)();
        let p = (api.json_object)();
        let k = cs("k");
        (api.json_object_set_new)(o, k.as_ptr(), p);
        incref(api, o);
        (api.json_object_set_new)(p, k.as_ptr(), o);

        for (tag, j) in [("arr", a), ("obj", o)] {
            for f in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS, JSON_EMBED] {
                rec.tag_ptr_null(
                    &format!("{tag}.dumps"),
                    (api.json_dumps)(j, f) as *const c_void,
                );
                rec.tag_u(&format!("{tag}.dumpb"), (api.json_dumpb)(j, ptr::null_mut(), 0, f));
                cb_reset(-1);
                let r = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                let got = cb_take();
                rec.tag_i(&format!("{tag}.cb"), r as i64);
                for (i, c) in got.iter().enumerate() {
                    rec.tag_bytes(&format!("{tag}.c{i}"), c);
                }
                assert_eq!(r, -1, "[{}] rows 124/125", api.tag);
                // row 132: json_dump_file where the dump itself fails
                let r = (api.json_dump_file)(j, cpfile.as_ptr(), f);
                rec.tag_i(&format!("{tag}.dump_file"), r as i64);
                assert_eq!(r, -1, "[{}] row 132", api.tag);
            }
        }
        (api.json_array_clear)(b);
        decref(api, a);
        (api.json_object_clear)(p);
        decref(api, o);
        let _ = std::fs::remove_file(&pfile);
    });
}

/* ------------- rows 126/127/136: allocation failures in the encoder ----- */

#[test]
fn err126_127_136_dump_oom() {
    diff("ERRORS 126/127/136 encoder OOM", |api, rec| unsafe {
        for text in [
            r#"{"a":1}"#,
            r#"{"z":1,"y":2,"x":3,"w":4,"v":5,"u":6,"t":7,"s":8,"r":9}"#,
            r#"[1,2,3,"four",5.5,null,true]"#,
            r#"{"deep":{"er":{"est":[1,2,3]}}}"#,
        ] {
            for f in [
                0usize,
                JSON_SORT_KEYS,
                JSON_SORT_KEYS | JSON_COMPACT,
                json_indent(2),
            ] {
                let t = text.to_string();
                oom_sweep(api, rec, &format!("{text}|{f}"), 40, move |api, rec| {
                    {
                        let z = cs(&t);
                        let j = (api.json_loads)(z.as_ptr(), 0, ptr::null_mut());
                        if j.is_null() {
                            rec.line("load=NULL");
                            return;
                        }
                        // row 127: strbuffer_init / realloc failures
                        let d = (api.json_dumps)(j, f);
                        rec.tag_ptr_null("dumps", d as *const c_void);
                        if !d.is_null() {
                            rec.cstring("dumps_v", d);
                            (api.jsonp_free)(d as *mut c_void);
                        }
                        // row 126: the JSON_SORT_KEYS key array allocation
                        cb_reset(-1);
                        let r =
                            (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                        let got = cb_take();
                        rec.tag_i("cb", r as i64);
                        rec.tag_i("cb_chunks", got.len() as i64);
                        // row 136: parents_set hashtable_init
                        rec.tag_u("dumpb", (api.json_dumpb)(j, ptr::null_mut(), 0, f));
                        decref(api, j);
                    }
                });
            }
        }
    });
}

/* ------------------------- rows 128/129/130: dumps / dumpb ------------- */

#[test]
fn err128to130_dumps_and_dumpb() {
    diff("ERRORS 128-130 dumps/dumpb", |api, rec| unsafe {
        let s = cs(r#"{"a":[1,2,3],"b":"text","c":1.5}"#);
        let j = (api.json_loads)(s.as_ptr(), 0, ptr::null_mut());
        for f in [0usize, JSON_COMPACT, JSON_SORT_KEYS, json_indent(2)] {
            let needed = (api.json_dumpb)(j, ptr::null_mut(), 0, f);
            rec.tag_u("needed", needed);
            for size in [0usize, 1, needed / 2, needed - 1, needed, needed + 1] {
                let mut buf = vec![0xA5u8; size + 4];
                let n = (api.json_dumpb)(j, buf.as_mut_ptr() as *mut c_char, size, f);
                rec.tag_u("n", n);
                rec.tag_bytes("buf", &buf);
                assert_eq!(n, needed, "[{}] row 130", api.tag);
            }
            // row 129: dumpb returns 0 when the dump fails
            let scalar = (api.json_integer)(1);
            rec.tag_u(
                "scalar_dumpb",
                (api.json_dumpb)(scalar, ptr::null_mut(), 0, f),
            );
            assert_eq!(
                (api.json_dumpb)(scalar, ptr::null_mut(), 0, f),
                0,
                "[{}] row 129",
                api.tag
            );
            decref(api, scalar);
            // row 128: json_dumps returns NULL when the dump fails
            let scalar = (api.json_integer)(1);
            rec.tag_ptr_null("scalar_dumps", (api.json_dumps)(scalar, f) as *const c_void);
            decref(api, scalar);
        }
        decref(api, j);
    });
}

/* --------------------------- rows 131/132: json_dump_file -------------- */

#[test]
fn err131and132_dump_file() {
    diff("ERRORS 131-132 json_dump_file", |api, rec| unsafe {
        let s = cs(r#"[1,2,3]"#);
        let j = (api.json_loads)(s.as_ptr(), 0, ptr::null_mut());
        // row 131: fopen failures
        for p in [
            "/nonexistent-dir-abc/out.json",
            "/proc/self/cmdline/impossible",
            "",
            "/",
        ] {
            let cp = cs(p);
            let r = (api.json_dump_file)(j, cp.as_ptr(), 0);
            rec.tag_i(&format!("fopen_fail{p}"), r as i64);
            assert_eq!(r, -1, "[{}] row 131 ({p})", api.tag);
        }
        // row 132: valid path but the dump fails (scalar without ENCODE_ANY)
        let path = tmp_file("dumpfile_err");
        let cp = cs(path.to_str().unwrap());
        let scalar = (api.json_integer)(7);
        let r = (api.json_dump_file)(scalar, cp.as_ptr(), 0);
        rec.tag_i("scalar_dump_file", r as i64);
        assert_eq!(r, -1, "[{}] row 132", api.tag);
        rec.tag_bytes("file_content", &std::fs::read(&path).unwrap_or_default());
        // and with ENCODE_ANY it succeeds
        let r = (api.json_dump_file)(scalar, cp.as_ptr(), JSON_ENCODE_ANY);
        rec.tag_i("scalar_dump_file_any", r as i64);
        rec.tag_bytes("file_content_any", &std::fs::read(&path).unwrap_or_default());
        decref(api, scalar);
        decref(api, j);
        let _ = std::fs::remove_file(&path);
    });
}
