//! Phase C — error-path differential tests for `error.c`, `memory.c`,
//! `strbuffer.c`, `strconv.c`, `utf.c`, `hashtable.c`, `dtoa.c` and the generic
//! FFI boundary (ERRORS.md rows 235..300).

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

/* ------------------------------ rows 235..242: error.c ------------------ */

#[test]
fn err235to242_error_struct() {
    diff("ERRORS 235-242 error.c", |api, rec| unsafe {
        let src = cs("source");
        let msg = cs("message %s");
        let arg = cs("arg");

        // rows 235/236/237/239: NULL arguments are no-ops
        (api.jsonp_error_init)(ptr::null_mut(), ptr::null());
        (api.jsonp_error_init)(ptr::null_mut(), src.as_ptr());
        (api.jsonp_error_set_source)(ptr::null_mut(), src.as_ptr());
        (api.jsonp_error_set_source)(ptr::null_mut(), ptr::null());
        (api.jsonp_error_set)(ptr::null_mut(), 1, 2, 3usize, 4, msg.as_ptr(), arg.as_ptr());
        rec.line("null_noops_ok");

        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set_source)(&mut e, ptr::null());
        rec.error("source_null_noop", &e);

        // row 238: source at and beyond the 80 byte limit
        for len in [0usize, 1, 76, 77, 78, 79, 80, 81, 82, 83, 120, 200] {
            let s: String = (b'a'..)
                .take(26)
                .cycle()
                .take(len)
                .map(|c| c as char)
                .collect();
            let cstr = cs(&s);
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set_source)(&mut e, cstr.as_ptr());
            rec.error(&format!("src{len}"), &e);
            // through jsonp_error_init as well
            let mut e2 = JsonError::patterned();
            (api.jsonp_error_init)(&mut e2, cstr.as_ptr());
            rec.error(&format!("init_src{len}"), &e2);
        }

        // row 240: a second jsonp_error_set is ignored
        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set)(&mut e, 1, 2, 3usize, 8, msg.as_ptr(), arg.as_ptr());
        rec.error("first", &e);
        for code in [0i32, 1, 17, 255] {
            (api.jsonp_error_set)(&mut e, 9, 9, 9usize, code, msg.as_ptr(), arg.as_ptr());
            rec.error("still_first", &e);
        }

        // row 241: message truncation at 158 bytes
        for n in [150usize, 156, 157, 158, 159, 160, 300, 1000] {
            let long: String = std::iter::repeat('Z').take(n).collect();
            let cl = cs(&long);
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set)(&mut e, 1, 1, 1usize, 8, cs("%s").as_ptr(), cl.as_ptr());
            rec.error(&format!("trunc{n}"), &e);
            assert!(
                e.text_str().len() <= JSON_ERROR_TEXT_LENGTH - 2,
                "[{}] row 241",
                api.tag
            );
        }

        // row 242: code values outside enum json_error_code
        for code in [-1i32, -128, 18, 19, 100, 127, 128, 200, 254, 255, 256, 511, 1000] {
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set)(&mut e, 0, 0, 0usize, code, cs("x").as_ptr());
            rec.tag_i(&format!("code{code}"), e.code() as i64);
            rec.error(&format!("code{code}.full"), &e);
        }
    });
}

/* ------------------------------ rows 243..250: memory.c ----------------- */

#[test]
fn err243to250_memory() {
    diff("ERRORS 243-250 memory.c", |api, rec| unsafe {
        restore_alloc(api);
        // row 243: jsonp_malloc(0)
        for _ in 0..3 {
            let p = (api.jsonp_malloc)(0);
            rec.tag_ptr_null("malloc0", p);
            assert!(p.is_null(), "[{}] row 243", api.tag);
        }
        // row 244: jsonp_free(NULL)
        (api.jsonp_free)(ptr::null_mut());
        rec.line("free_null_ok");

        // rows 245/246/247: the realloc emulation path
        install_hooks1(api);
        alloc_reset();
        let p = (api.jsonp_malloc)(32);
        assert!(!p.is_null());
        let r = (api.jsonp_realloc)(p, 32, 0);
        rec.tag_ptr_null("emul_zero_with_ptr", r);
        assert!(r.is_null(), "[{}] row 245", api.tag);
        let r = (api.jsonp_realloc)(ptr::null_mut(), 0, 0);
        rec.tag_ptr_null("emul_zero_null_ptr", r);
        assert!(r.is_null(), "[{}] row 246", api.tag);
        // row 247: do_malloc fails -> NULL and the old pointer survives
        let p = (api.jsonp_malloc)(32);
        ptr::write_bytes(p as *mut u8, 0x77, 32);
        alloc_fail_nth(0);
        let r = (api.jsonp_realloc)(p, 32, 64);
        alloc_reset();
        rec.tag_ptr_null("emul_malloc_fail", r);
        assert!(r.is_null(), "[{}] row 247", api.tag);
        rec.tag_bytes(
            "old_intact",
            std::slice::from_raw_parts(p as *const u8, 32),
        );
        (api.jsonp_free)(p);
        restore_alloc(api);
        alloc_reset();

        // row 248: jsonp_strndup when the allocation fails
        install_hooks2(api);
        let text = cs("some text");
        alloc_fail_nth(0);
        let d = (api.jsonp_strndup)(text.as_ptr(), 4);
        alloc_reset();
        rec.tag_ptr_null("strndup_fail", d as *const c_void);
        assert!(d.is_null(), "[{}] row 248", api.tag);
        restore_alloc(api);
        alloc_reset();

        // rows 249/250: NULL out-parameters
        (api.json_get_alloc_funcs)(ptr::null_mut(), ptr::null_mut());
        (api.json_get_alloc_funcs2)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        let mut m: JsonMalloc = None;
        (api.json_get_alloc_funcs)(&mut m, ptr::null_mut());
        rec.tag_i("m_set", m.is_some() as i64);
        let mut f: JsonFree = None;
        (api.json_get_alloc_funcs)(ptr::null_mut(), &mut f);
        rec.tag_i("f_set", f.is_some() as i64);
        let mut r2: JsonRealloc = None;
        (api.json_get_alloc_funcs2)(ptr::null_mut(), &mut r2, ptr::null_mut());
        rec.tag_i("r_set", r2.is_some() as i64);
        rec.line("get_alloc_nulls_ok");

        // installing NULL functions is accepted by the setters
        (api.json_set_alloc_funcs2)(None, None, None);
        let mut m: JsonMalloc = Some(real_malloc);
        let mut r3: JsonRealloc = Some(real_realloc);
        let mut f3: JsonFree = Some(real_free);
        (api.json_get_alloc_funcs2)(&mut m, &mut r3, &mut f3);
        rec.tag_i("m_none", m.is_none() as i64);
        rec.tag_i("r_none", r3.is_none() as i64);
        rec.tag_i("f_none", f3.is_none() as i64);
        restore_alloc(api);
    });
}

/* ---------------------------- rows 251..256: strbuffer.c ---------------- */

#[test]
fn err251to256_strbuffer() {
    diff("ERRORS 251-256 strbuffer.c", |api, rec| unsafe {
        // row 251: strbuffer_init allocation failure
        install_hooks2(api);
        alloc_fail_nth(0);
        let mut sb = Strbuffer::zeroed();
        let r = (api.strbuffer_init)(&mut sb);
        alloc_reset();
        rec.tag_i("init_fail", r as i64);
        assert_eq!(r, -1, "[{}] row 251", api.tag);
        rec.tag_ptr_null("init_fail_value", sb.value as *const c_void);
        restore_alloc(api);
        alloc_reset();

        // row 255: jsonp_realloc failure while growing
        oom_sweep(api, rec, "append_grow", 12, |api, rec| {
            let mut sb = Strbuffer::zeroed();
            if (api.strbuffer_init)(&mut sb) != 0 {
                rec.line("init=-1");
                return;
            }
            let data = vec![b'x'; 100];
            rec.tag_i(
                "app",
                (api.strbuffer_append_bytes)(&mut sb, data.as_ptr() as *const c_char, 100) as i64,
            );
            rec.tag_u("len", sb.length);
            rec.tag_u("size", sb.size);
            if !sb.value.is_null() {
                rec.tag_bytes(
                    "bytes",
                    std::slice::from_raw_parts(sb.value as *const u8, sb.length),
                );
            }
            (api.strbuffer_close)(&mut sb);
        });

        // rows 252/253/254: the three integer-overflow guards.  Each one returns
        // before any memory is touched, so a hand-built strbuffer_t is safe.
        let scratch = (api.jsonp_malloc)(16);
        assert!(!scratch.is_null());
        // Every tuple below satisfies `arg >= size - length` (so the guards are
        // reached) *and* trips exactly one of the three overflow checks, which
        // all `return -1` before the `memcpy`.
        let cases: &[(u32, usize, usize, usize)] = &[
            // (row, strbuff.size, strbuff.length, argument size)
            (252, usize::MAX / 2 + 1, usize::MAX / 2, 1),
            (252, usize::MAX, usize::MAX - 4, 8),
            (253, 16, 0, usize::MAX),
            (253, 16, 4, usize::MAX),
            (254, 16, usize::MAX - 100, 200),
            (254, 16, usize::MAX - 1, 100),
        ];
        for (row, size, length, arg) in cases {
            let mut sb = Strbuffer {
                value: scratch as *mut c_char,
                length: *length,
                size: *size,
            };
            let r = (api.strbuffer_append_bytes)(&mut sb, scratch as *const c_char, *arg);
            rec.tag_i(&format!("row{row}.ret"), r as i64);
            rec.tag_u(&format!("row{row}.len"), sb.length);
            rec.tag_u(&format!("row{row}.size"), sb.size);
            assert_eq!(r, -1, "[{}] row {row}", api.tag);
        }
        (api.jsonp_free)(scratch);

        // row 256: strbuffer_pop on an empty buffer
        let mut sb = Strbuffer::zeroed();
        assert_eq!((api.strbuffer_init)(&mut sb), 0);
        for _ in 0..5 {
            rec.tag_i("pop_empty", (api.strbuffer_pop)(&mut sb) as i64);
            rec.tag_u("len", sb.length);
        }
        assert_eq!((api.strbuffer_append_byte)(&mut sb, b'q' as c_char), 0);
        rec.tag_i("pop_one", (api.strbuffer_pop)(&mut sb) as i64);
        rec.tag_i("pop_again", (api.strbuffer_pop)(&mut sb) as i64);
        rec.tag_u("len_end", sb.length);
        // appending zero bytes never fails
        rec.tag_i(
            "append_zero",
            (api.strbuffer_append_bytes)(&mut sb, scratch as *const c_char, 0) as i64,
        );
        (api.strbuffer_close)(&mut sb);
    });
}

/* ---------------------------- rows 257..260: strconv.c ----------------- */

#[test]
fn err257to260_strconv() {
    diff("ERRORS 257-260 strconv.c", |api, rec| unsafe {
        // rows 257/258: overflow and underflow
        let cases: &[(u32, &str, c_int)] = &[
            (257, "1e400", -1),
            (257, "-1e400", -1),
            (257, "1e999", -1),
            (257, "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497792", -1),
            (258, "1e-400", 0),
            (258, "-1e-400", 0),
            (258, "1e-999", 0),
            (258, "4.9406564584124654e-324", 0),
            (258, "2.4703282292062327e-324", 0),
        ];
        for (row, text, want) in cases {
            let mut sb = Strbuffer::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            assert_eq!(
                (api.strbuffer_append_bytes)(
                    &mut sb,
                    text.as_ptr() as *const c_char,
                    text.len()
                ),
                0
            );
            let mut out: f64 = -98765.0;
            let r = (api.jsonp_strtod)(&mut sb, &mut out);
            rec.tag_i(&format!("row{row}.ret"), r as i64);
            rec.tag_f(&format!("row{row}.out"), out);
            assert_eq!(r, *want, "[{}] row {row} ({text})", api.tag);
            (api.strbuffer_close)(&mut sb);
        }

        // rows 259/260: jsonp_dtostr with buffers that are too small
        let mut saw_fail = false;
        for v in [
            1.0f64,
            -1.0,
            0.1,
            1.0 / 3.0,
            f64::MAX,
            -f64::MAX,
            5e-324,
            1e300,
            -1e-300,
            123456789012345678.0,
        ] {
            for prec in 0..32i32 {
                for size in [0usize, 1, 2, 3, 4, 5, 8, 12, 20, 24, 25, 26, 40] {
                    let mut buf = [0x5Au8; 64];
                    let r = (api.jsonp_dtostr)(buf.as_mut_ptr() as *mut c_char, size, v, prec);
                    rec.tag_i("ret", r as i64);
                    if r < 0 {
                        saw_fail = true;
                    } else {
                        rec.tag_bytes("buf", &buf[..(r as usize) + 1]);
                        assert!(
                            (r as usize) < size,
                            "[{}] row 259: wrote {r} bytes into a {size} byte buffer",
                            api.tag
                        );
                    }
                }
            }
        }
        assert!(saw_fail, "[{}] rows 259/260 never triggered", api.tag);
    });
}

/* ------------------------------- rows 261..277: utf.c ------------------- */

#[test]
fn err261to277_utf() {
    diff("ERRORS 261-277 utf.c", |api, rec| unsafe {
        // rows 261/262: utf8_encode range checks
        for cp in [
            -1i32,
            -2,
            -0x10FFFF,
            i32::MIN,
            0x110000,
            0x110001,
            0x1FFFFF,
            0x7FFFFFFF,
            i32::MAX,
        ] {
            let mut buf = [0x5Au8; 8];
            let mut size: usize = 0xDEAD;
            let r = (api.utf8_encode)(cp, buf.as_mut_ptr() as *mut c_char, &mut size);
            rec.tag_i(&format!("encode{cp}"), r as i64);
            rec.tag_u(&format!("encode{cp}.size"), size);
            rec.tag_bytes(&format!("encode{cp}.buf"), &buf);
            assert_eq!(r, -1, "[{}] rows 261/262 ({cp})", api.tag);
        }
        // surrogates and 0x10FFFF *are* accepted by utf8_encode
        for cp in [0i32, 0xD800, 0xDFFF, 0x10FFFF] {
            let mut buf = [0x5Au8; 8];
            let mut size: usize = 0;
            let r = (api.utf8_encode)(cp, buf.as_mut_ptr() as *mut c_char, &mut size);
            rec.tag_i(&format!("enc_ok{cp}"), r as i64);
            rec.tag_u(&format!("enc_ok{cp}.size"), size);
            rec.tag_bytes(&format!("enc_ok{cp}.buf"), &buf);
        }

        // rows 263/264/265: utf8_check_first
        for b in 0x80u32..=0xBF {
            let n = (api.utf8_check_first)(b as u8 as c_char);
            rec.tag_u("cont", n);
            assert_eq!(n, 0, "[{}] row 263 (0x{b:02x})", api.tag);
        }
        for b in [0xC0u32, 0xC1] {
            let n = (api.utf8_check_first)(b as u8 as c_char);
            rec.tag_u("overlong_lead", n);
            assert_eq!(n, 0, "[{}] row 264", api.tag);
        }
        for b in 0xF5u32..=0xFF {
            let n = (api.utf8_check_first)(b as u8 as c_char);
            rec.tag_u("restricted", n);
            assert_eq!(n, 0, "[{}] row 265 (0x{b:02x})", api.tag);
        }

        // row 266: utf8_check_full with an unsupported size
        let seq = b"\xf0\x90\x80\x80\0\0\0\0";
        for size in [0usize, 1, 5, 6, 7, 100, usize::MAX] {
            let mut cp: i32 = -1;
            let n = (api.utf8_check_full)(seq.as_ptr() as *const c_char, size, &mut cp);
            rec.tag_u(&format!("full_size{size}"), n);
            rec.tag_i(&format!("full_size{size}.cp"), cp as i64);
            assert_eq!(n, 0, "[{}] row 266 (size {size})", api.tag);
        }

        // rows 267..270: bad continuation / range / surrogate / overlong
        let bad: &[(u32, &[u8], usize)] = &[
            (267, b"\xc2\x41", 2),
            (267, b"\xc2\xc2", 2),
            (267, b"\xe0\xa0\x41", 3),
            (267, b"\xf0\x90\x80\x41", 4),
            (268, b"\xf4\x90\x80\x80", 4),
            (268, b"\xf7\xbf\xbf\xbf", 4),
            (269, b"\xed\xa0\x80", 3),
            (269, b"\xed\xbf\xbf", 3),
            (270, b"\xc0\x80", 2),
            (270, b"\xc1\xbf", 2),
            (270, b"\xe0\x80\x80", 3),
            (270, b"\xe0\x9f\xbf", 3),
            (270, b"\xf0\x80\x80\x80", 4),
            (270, b"\xf0\x8f\xbf\xbf", 4),
        ];
        for (row, seq, size) in bad {
            let mut cp: i32 = -1;
            let n = (api.utf8_check_full)(seq.as_ptr() as *const c_char, *size, &mut cp);
            rec.tag_u(&format!("row{row}"), n);
            rec.tag_i(&format!("row{row}.cp"), cp as i64);
            assert_eq!(n, 0, "[{}] row {row} ({seq:?})", api.tag);
            // and through utf8_check_string / utf8_iterate
            let s = (api.utf8_check_string)(seq.as_ptr() as *const c_char, *size);
            rec.tag_i(&format!("row{row}.str"), s as i64);
            assert_eq!(s, 0, "[{}] rows 275-277 ({seq:?})", api.tag);
            let it = (api.utf8_iterate)(seq.as_ptr() as *const c_char, *size, &mut cp);
            rec.tag_ptr_null(&format!("row{row}.iter"), it as *const c_void);
            assert!(it.is_null(), "[{}] row 274 ({seq:?})", api.tag);
        }

        // row 271: bufsize == 0 returns the buffer unchanged
        let buf = b"abc";
        let mut cp: i32 = -4242;
        let it = (api.utf8_iterate)(buf.as_ptr() as *const c_char, 0, &mut cp);
        rec.tag_i(
            "row271.same",
            (it == buf.as_ptr() as *const c_char) as i64,
        );
        rec.tag_i("row271.cp", cp as i64);
        assert_eq!(it, buf.as_ptr() as *const c_char, "[{}] row 271", api.tag);
        assert_eq!(cp, -4242, "[{}] row 271: codepoint must be untouched", api.tag);

        // row 272: invalid lead byte
        for b in [0x80u8, 0xBF, 0xC0, 0xC1, 0xF5, 0xFF] {
            let one = [b, 0, 0, 0];
            let mut cp: i32 = -1;
            let it = (api.utf8_iterate)(one.as_ptr() as *const c_char, 4, &mut cp);
            rec.tag_ptr_null("row272", it as *const c_void);
            assert!(it.is_null(), "[{}] row 272 (0x{b:02x})", api.tag);
        }

        // row 273: truncated sequences (count > bufsize)
        let seqs: &[&[u8]] = &[b"\xc2\x80", b"\xe0\xa0\x80", b"\xf0\x90\x80\x80"];
        for s in seqs {
            for avail in 1..s.len() {
                let mut cp: i32 = -1;
                let it = (api.utf8_iterate)(s.as_ptr() as *const c_char, avail, &mut cp);
                rec.tag_ptr_null(&format!("row273.{avail}"), it as *const c_void);
                assert!(it.is_null(), "[{}] row 273", api.tag);
                // utf8_check_string sees the same truncation (row 276)
                let r = (api.utf8_check_string)(s.as_ptr() as *const c_char, avail);
                rec.tag_i(&format!("row276.{avail}"), r as i64);
                assert_eq!(r, 0, "[{}] row 276", api.tag);
            }
            // the full sequence is accepted
            let mut cp: i32 = -1;
            let it = (api.utf8_iterate)(s.as_ptr() as *const c_char, s.len(), &mut cp);
            rec.tag_i("full_ok", (!it.is_null()) as i64);
            rec.tag_i("full_cp", cp as i64);
        }

        // row 275: an invalid lead byte anywhere in the string
        for pos in 0..4usize {
            let mut v = b"abcd".to_vec();
            v[pos] = 0xFF;
            let r = (api.utf8_check_string)(v.as_ptr() as *const c_char, v.len());
            rec.tag_i(&format!("row275.{pos}"), r as i64);
            assert_eq!(r, 0, "[{}] row 275", api.tag);
        }

        // row 299: length 0 with a non-NULL pointer is accepted everywhere
        let e = b"";
        rec.tag_i(
            "empty_check_string",
            (api.utf8_check_string)(e.as_ptr() as *const c_char, 0) as i64,
        );
        rec.json("empty_stringn", (api.json_stringn)(e.as_ptr() as *const c_char, 0));
        let s0 = (api.json_stringn)(e.as_ptr() as *const c_char, 0);
        rec.tag_u("empty_len", (api.json_string_length)(s0));
        decref(api, s0);
        let o = (api.json_object)();
        rec.tag_i(
            "empty_key",
            (api.json_object_setn_new)(
                o,
                e.as_ptr() as *const c_char,
                0,
                (api.json_integer)(1),
            ) as i64,
        );
        rec.tag_u("empty_key_size", (api.json_object_size)(o));
        rec.json(
            "empty_key_get",
            (api.json_object_getn)(o, e.as_ptr() as *const c_char, 0),
        );
        rec_dump_all(api, rec, "empty_key_obj", o);
        decref(api, o);
    });
}

/* ---------------------------- rows 278..287: hashtable.c ---------------- */

#[test]
fn err278to287_hashtable() {
    diff("ERRORS 278-287 hashtable.c", |api, rec| unsafe {
        // row 278: hashtable_init allocation failure
        install_hooks2(api);
        alloc_fail_nth(0);
        let mut ht = Hashtable::zeroed();
        let r = (api.hashtable_init)(&mut ht);
        alloc_reset();
        rec.tag_i("init_fail", r as i64);
        assert_eq!(r, -1, "[{}] row 278", api.tag);
        restore_alloc(api);
        alloc_reset();

        // rows 279/281: rehash and pair allocation failures
        oom_sweep(api, rec, "ht_set", 32, |api, rec| {
            let mut ht = Hashtable::zeroed();
            if (api.hashtable_init)(&mut ht) != 0 {
                rec.line("init=-1");
                return;
            }
            for i in 0..10i64 {
                let k = cs(&format!("k{i}"));
                let v = (api.json_integer)(i);
                let r = (api.hashtable_set)(&mut ht, k.as_ptr(), 2, v);
                rec.tag_i("set", r as i64);
                if r != 0 {
                    // the value was not adopted
                    decref(api, v);
                }
            }
            rec.tag_u("size", ht.size);
            rec.tag_u("order", ht.order);
            (api.hashtable_close)(&mut ht);
        });

        // rows 282..287: lookup / delete / iteration misses
        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        // row 285: iteration over an empty table
        rec.tag_ptr_null("iter_empty", (api.hashtable_iter)(&mut ht));
        assert!(
            (api.hashtable_iter)(&mut ht).is_null(),
            "[{}] row 285",
            api.tag
        );
        let probe = cs("abc");
        rec.tag_ptr_null(
            "get_empty",
            (api.hashtable_get)(&mut ht, probe.as_ptr(), 3),
        );
        rec.tag_i(
            "del_empty",
            (api.hashtable_del)(&mut ht, probe.as_ptr(), 3) as i64,
        );
        rec.tag_ptr_null(
            "iter_at_empty",
            (api.hashtable_iter_at)(&mut ht, probe.as_ptr(), 3),
        );

        for key in ["abc", "abcd", "ab", ""] {
            let k = cs(key);
            assert_eq!(
                (api.hashtable_set)(&mut ht, k.as_ptr(), key.len(), (api.json_integer)(1)),
                0
            );
        }
        // row 282: absent keys
        for key in ["zzz", "abcde", "a", "ABC"] {
            let k = cs(key);
            rec.tag_ptr_null(
                &format!("miss.{key}"),
                (api.hashtable_get)(&mut ht, k.as_ptr(), key.len()),
            );
            rec.tag_i(
                &format!("del_miss.{key}"),
                (api.hashtable_del)(&mut ht, k.as_ptr(), key.len()) as i64,
            );
            rec.tag_ptr_null(
                &format!("iter_at_miss.{key}"),
                (api.hashtable_iter_at)(&mut ht, k.as_ptr(), key.len()),
            );
        }
        // row 283: same bytes, different key_len
        let k = cs("abcd");
        for kl in [0usize, 1, 2, 3, 4] {
            let g = (api.hashtable_get)(&mut ht, k.as_ptr(), kl);
            rec.tag_ptr_null(&format!("keylen{kl}"), g);
        }
        // row 287: iter_next at the last element
        let mut it = (api.hashtable_iter)(&mut ht);
        let mut n = 0;
        while !it.is_null() {
            let next = (api.hashtable_iter_next)(&mut ht, it);
            if next.is_null() {
                rec.tag_i("last_index", n);
            }
            it = next;
            n += 1;
        }
        rec.tag_i("count", n);
        (api.hashtable_close)(&mut ht);
    });
}

/* -------------------------------- rows 288..293: dtoa.c ---------------- */

#[test]
fn err288to293_dtoa() {
    diff("ERRORS 288-293 dtoa.c", |api, rec| unsafe {
        // row 288: dtoa_r with a buffer that is too small
        let mut saw_null = false;
        for v in [1.0 / 3.0, f64::MAX, 5e-324, 0.1, 1e300] {
            for nd in [1i32, 5, 10, 17, 20, 25, 30, 40] {
                for blen in [0usize, 1, 2, 3, 5, 10, 20, 25, 30, 50] {
                    let mut buf = [0x5Au8; 64];
                    let mut decpt: c_int = -999;
                    let mut sign: c_int = -999;
                    let mut rve: *mut c_char = ptr::null_mut();
                    let r = (api.dtoa_r)(
                        v,
                        2,
                        nd,
                        &mut decpt,
                        &mut sign,
                        &mut rve,
                        buf.as_mut_ptr() as *mut c_char,
                        blen,
                    );
                    if r.is_null() {
                        saw_null = true;
                        rec.line("dtoa_r=NULL");
                    } else {
                        rec.cstring("dtoa_r", r);
                        rec.tag_i("decpt", decpt as i64);
                        rec.tag_i("sign", sign as i64);
                    }
                }
            }
        }
        assert!(saw_null, "[{}] row 288 never triggered", api.tag);

        // row 289: mode / ndigits outside the documented range
        for mode in [-5i32, -1, 6, 7, 100, i32::MAX] {
            for nd in [-100i32, -1, 0, 1, 30] {
                let mut buf = [0x5Au8; 64];
                let mut decpt: c_int = -999;
                let mut sign: c_int = -999;
                let mut rve: *mut c_char = ptr::null_mut();
                let r = (api.dtoa_r)(
                    2.5,
                    mode,
                    nd,
                    &mut decpt,
                    &mut sign,
                    &mut rve,
                    buf.as_mut_ptr() as *mut c_char,
                    40,
                );
                if r.is_null() {
                    rec.line("mode_null");
                } else {
                    rec.cstring("mode_digits", r);
                    rec.tag_i("mode_decpt", decpt as i64);
                    rec.tag_i("mode_sign", sign as i64);
                }
            }
        }

        // row 290: gethex with no hex digits
        for s in ["0x", "0xp1", "0xP-3", "0x.", "0x.p1", "0xg"] {
            for rounding in 0..4i32 {
                for sign in 0..2i32 {
                    let cstr = cs(s);
                    let mut sp: *const c_char = cstr.as_ptr();
                    let mut u = U {
                        L: [0x5A5A_5A5A, 0x5A5A_5A5A],
                    };
                    (api.gethex)(&mut sp, &mut u, rounding, sign);
                    rec.tag_f(&format!("row290.{s}.d"), u.d);
                    rec.tag_i(
                        &format!("row290.{s}.off"),
                        sp.offset_from(cstr.as_ptr()) as i64,
                    );
                }
            }
        }
        // row 291: hex exponent overflow / underflow
        for s in [
            "0x1p100000", "0x1p-100000", "0x1p2000", "0x1p-2000", "0x1p1024", "0x1p-1075",
            "0x1p-1074", "0x1fffffffffffffp971", "0x1p+99999999999999999999",
        ] {
            for rounding in 0..4i32 {
                for sign in 0..2i32 {
                    let cstr = cs(s);
                    let mut sp: *const c_char = cstr.as_ptr();
                    let mut u = U {
                        L: [0x5A5A_5A5A, 0x5A5A_5A5A],
                    };
                    (api.gethex)(&mut sp, &mut u, rounding, sign);
                    rec.tag_f(&format!("row291.{s}.d"), u.d);
                    rec.tag_i(
                        &format!("row291.{s}.off"),
                        sp.offset_from(cstr.as_ptr()) as i64,
                    );
                }
            }
        }

        // rows 292/293: strtod__unused with nothing convertible / out of range
        for s in [
            "", " ", "x", "+", "-", ".", "e5", "+e", "-.", "abc", "0x", "1e400", "-1e400",
            "1e-400", "-1e-400", "1e99999999999999999999", "inf", "-inf", "nan",
        ] {
            let cstr = cs(s);
            let mut end: *mut c_char = 1usize as *mut c_char;
            let v = (api.strtod__unused)(cstr.as_ptr(), &mut end);
            rec.tag_f(&format!("row292.{s}.v"), v);
            rec.tag_i(
                &format!("row292.{s}.off"),
                if end.is_null() {
                    -1
                } else {
                    end.offset_from(cstr.as_ptr()) as i64
                },
            );
        }
    });
}

/* -------------------- rows 295..300: generic FFI boundary -------------- */

#[test]
fn err295to300_generic_boundary() {
    diff("ERRORS 295-300 generic boundary", |api, rec| unsafe {
        let s = cs(r#"{"a":[1,2,{"b":null}],"c":1.5}"#);
        // row 295: out-of-range flag bits on every entry point
        let weird_flags = [
            usize::MAX,
            0xFFFF_FFFF_0000_0000,
            1usize << 63,
            0x8000,
            0xFFFF,
            (1usize << 40) | 0x1F,
        ];
        for f in weird_flags {
            let mut e = JsonError::patterned();
            let j = (api.json_loads)(s.as_ptr(), f, &mut e);
            rec.json("loads", j);
            rec.error("loads_err", &e);
            if !j.is_null() {
                match dumps(api, j, f) {
                    None => rec.line("dumps=NULL"),
                    Some(d) => rec.tag_bytes("dumps", &d),
                }
                rec.tag_u("dumpb", (api.json_dumpb)(j, ptr::null_mut(), 0, f));
            }
            decref(api, j);

            // row 297: pack / unpack with flags outside the documented set
            let pf = cs("{s:i}");
            let k = cs("k");
            let mut e = JsonError::patterned();
            let p = (api.json_pack_ex)(&mut e, f, pf.as_ptr(), k.as_ptr(), 3i32);
            rec.json("pack", p);
            rec.error("pack_err", &e);
            rec_dump_all(api, rec, "pack", p);
            if !p.is_null() {
                // JSON_VALIDATE_ONLY is set in all of these flag words except
                // 0x8000, so only pass keys (see the pack/unpack contract).
                let uf = cs("{s:i}");
                let mut e2 = JsonError::patterned();
                let r = if f & JSON_VALIDATE_ONLY != 0 {
                    (api.json_unpack_ex)(p, &mut e2, f, uf.as_ptr(), k.as_ptr())
                } else {
                    let mut v: c_int = -1;
                    (api.json_unpack_ex)(p, &mut e2, f, uf.as_ptr(), k.as_ptr(), &mut v)
                };
                rec.tag_i("unpack", r as i64);
                rec.error("unpack_err", &e2);
            }
            decref(api, p);
        }

        // row 296: jsonp_error_set with a code that has no enum variant
        for code in [18i32, 64, 127, 128, 255, -1, 1000] {
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set)(&mut e, 1, 1, 1usize, code, cs("m").as_ptr());
            rec.error(&format!("code{code}"), &e);
        }

        // row 298: index == SIZE_MAX everywhere
        let a = (api.json_array)();
        for i in 0..3i64 {
            (api.json_array_append_new)(a, (api.json_integer)(i));
        }
        for idx in [usize::MAX, usize::MAX - 1, usize::MAX / 2] {
            rec.json("get", (api.json_array_get)(a, idx));
            rec.tag_i(
                "set",
                (api.json_array_set_new)(a, idx, (api.json_integer)(1)) as i64,
            );
            rec.tag_i(
                "insert",
                (api.json_array_insert_new)(a, idx, (api.json_integer)(1)) as i64,
            );
            rec.tag_i("remove", (api.json_array_remove)(a, idx) as i64);
        }
        rec_dump_all(api, rec, "a", a);
        decref(api, a);

        // row 300: size == SIZE_MAX where the C code checks it up front
        let buf = b"\xc2\x80\0\0";
        rec.tag_u(
            "check_full_max",
            (api.utf8_check_full)(buf.as_ptr() as *const c_char, usize::MAX, ptr::null_mut()),
        );
        let mut sb = Strbuffer::zeroed();
        assert_eq!((api.strbuffer_init)(&mut sb), 0);
        rec.tag_i(
            "append_max",
            (api.strbuffer_append_bytes)(&mut sb, buf.as_ptr() as *const c_char, usize::MAX) as i64,
        );
        rec.tag_u("sb_len", sb.length);
        (api.strbuffer_close)(&mut sb);

        // json_dumpb with a huge size but a NULL buffer only queries the length
        let j = (api.json_loads)(s.as_ptr(), 0, ptr::null_mut());
        rec.tag_u(
            "dumpb_query",
            (api.json_dumpb)(j, ptr::null_mut(), 0, JSON_COMPACT),
        );
        decref(api, j);
    });
}
