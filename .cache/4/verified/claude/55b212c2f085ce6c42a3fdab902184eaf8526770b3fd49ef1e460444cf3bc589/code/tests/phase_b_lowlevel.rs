//! Phase B — differential tests for the *lowest level* entry points
//! (CONFIGS.md rows 1..41).  Every call goes through the `.so` exports.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

/* ------------------------------------------------- row 1: version.c ------ */

#[test]
fn cfg01_version() {
    diff("cfg01 version", |api, rec| unsafe {
        rec.cstring("str", (api.jansson_version_str)());
        let mut rng = Rng::new(1);
        for _ in 0..200 {
            let a = rng.range_i64(-3, 6) as c_int;
            let b = rng.range_i64(-3, 20) as c_int;
            let c = rng.range_i64(-3, 6) as c_int;
            rec.tag_i("cmp", (api.jansson_version_cmp)(a, b, c) as i64);
        }
        for t in [
            (2, 15, 0),
            (2, 15, 1),
            (2, 15, -1),
            (2, 14, 0),
            (2, 16, 0),
            (1, 99, 99),
            (3, 0, 0),
            (i32::MIN, 0, 0),
            (i32::MAX, 0, 0),
            (0, i32::MIN, 0),
            (0, 0, i32::MIN),
        ] {
            rec.tag_i("cmpx", (api.jansson_version_cmp)(t.0, t.1, t.2) as i64);
        }
    });
}

/* ------------------------------------ rows 2..7: memory.c --------------- */

#[test]
fn cfg02_jsonp_malloc_free() {
    diff("cfg02 jsonp_malloc", |api, rec| unsafe {
        for size in [0usize, 1, 2, 7, 8, 15, 16, 17, 1024, 4096] {
            let p = (api.jsonp_malloc)(size);
            rec.tag_ptr_null("malloc", p);
            if !p.is_null() {
                ptr::write_bytes(p as *mut u8, 0xAB, size);
                let s = std::slice::from_raw_parts(p as *const u8, size);
                rec.tag_u("sum", s.iter().map(|&b| b as usize).sum());
            }
            (api.jsonp_free)(p);
        }
        // jsonp_free(NULL) must be a no-op
        (api.jsonp_free)(ptr::null_mut());
        rec.line("free_null_ok");
    });
}

#[test]
fn cfg03_jsonp_realloc_real() {
    diff("cfg03 jsonp_realloc (real realloc)", |api, rec| unsafe {
        restore_alloc(api);
        // grow
        let p = (api.jsonp_malloc)(16);
        ptr::write_bytes(p as *mut u8, 0x11, 16);
        let p2 = (api.jsonp_realloc)(p, 16, 64);
        rec.tag_ptr_null("grow", p2);
        rec.tag_bytes("grow.head", std::slice::from_raw_parts(p2 as *const u8, 16));
        // shrink
        let p3 = (api.jsonp_realloc)(p2, 64, 8);
        rec.tag_ptr_null("shrink", p3);
        rec.tag_bytes("shrink.head", std::slice::from_raw_parts(p3 as *const u8, 8));
        // newSize == 0
        let p4 = (api.jsonp_realloc)(p3, 8, 0);
        rec.tag_ptr_null("zero", p4);
        // ptr == NULL behaves like malloc
        let p5 = (api.jsonp_realloc)(ptr::null_mut(), 0, 32);
        rec.tag_ptr_null("from_null", p5);
        (api.jsonp_free)(p5);
        // ptr == NULL, newSize == 0
        let p6 = (api.jsonp_realloc)(ptr::null_mut(), 0, 0);
        rec.tag_ptr_null("null_zero", p6);
        (api.jsonp_free)(p6);
    });
}

#[test]
fn cfg04_jsonp_realloc_emulated() {
    diff("cfg04 jsonp_realloc (emulation)", |api, rec| unsafe {
        alloc_reset();
        install_hooks1(api); // do_realloc == NULL
        let p = (api.jsonp_malloc)(16);
        ptr::write_bytes(p as *mut u8, 0x22, 16);
        let p2 = (api.jsonp_realloc)(p, 16, 64);
        rec.tag_ptr_null("grow", p2);
        rec.tag_bytes("grow.head", std::slice::from_raw_parts(p2 as *const u8, 16));
        let p3 = (api.jsonp_realloc)(p2, 64, 8);
        rec.tag_ptr_null("shrink", p3);
        rec.tag_bytes("shrink.head", std::slice::from_raw_parts(p3 as *const u8, 8));
        let p4 = (api.jsonp_realloc)(p3, 8, 0);
        rec.tag_ptr_null("zero_frees", p4);
        let p5 = (api.jsonp_realloc)(ptr::null_mut(), 0, 0);
        rec.tag_ptr_null("null_zero", p5);
        let p6 = (api.jsonp_realloc)(ptr::null_mut(), 0, 24);
        rec.tag_ptr_null("null_grow", p6);
        (api.jsonp_free)(p6);
        rec.tag_i("allocs", alloc_count());
        restore_alloc(api);
        alloc_reset();
    });
}

#[test]
fn cfg05_jsonp_strndup() {
    diff("cfg05 jsonp_strndup", |api, rec| unsafe {
        let src = b"hello\0world-tail\0";
        for len in 0usize..=16 {
            let p = (api.jsonp_strndup)(src.as_ptr() as *const c_char, len);
            rec.tag_ptr_null("dup", p as *const c_void);
            if !p.is_null() {
                rec.tag_bytes("dup.bytes", std::slice::from_raw_parts(p as *const u8, len + 1));
                (api.jsonp_free)(p as *mut c_void);
            }
        }
    });
}

#[test]
fn cfg06_get_alloc_funcs() {
    diff("cfg06 get_alloc_funcs", |api, rec| unsafe {
        restore_alloc(api);
        let mut m: JsonMalloc = None;
        let mut r: JsonRealloc = None;
        let mut f: JsonFree = None;
        (api.json_get_alloc_funcs2)(&mut m, &mut r, &mut f);
        rec.tag_i("m_is_real", (m == Some(real_malloc)) as i64);
        rec.tag_i("r_is_real", (r == Some(real_realloc)) as i64);
        rec.tag_i("f_is_real", (f == Some(real_free)) as i64);

        // NULL out params must be tolerated
        (api.json_get_alloc_funcs2)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        (api.json_get_alloc_funcs)(ptr::null_mut(), ptr::null_mut());
        rec.line("null_outparams_ok");

        // set_alloc_funcs -> realloc becomes NULL
        install_hooks1(api);
        let mut m2: JsonMalloc = None;
        let mut r2: JsonRealloc = Some(real_realloc);
        let mut f2: JsonFree = None;
        (api.json_get_alloc_funcs2)(&mut m2, &mut r2, &mut f2);
        rec.tag_i("m2_is_hook", (m2 == Some(hook_malloc)) as i64);
        rec.tag_i("r2_is_none", r2.is_none() as i64);
        rec.tag_i("f2_is_hook", (f2 == Some(hook_free)) as i64);

        let mut m3: JsonMalloc = None;
        let mut f3: JsonFree = None;
        (api.json_get_alloc_funcs)(&mut m3, &mut f3);
        rec.tag_i("m3_is_hook", (m3 == Some(hook_malloc)) as i64);
        rec.tag_i("f3_is_hook", (f3 == Some(hook_free)) as i64);

        // set_alloc_funcs2 -> all three
        install_hooks2(api);
        let mut m4: JsonMalloc = None;
        let mut r4: JsonRealloc = None;
        let mut f4: JsonFree = None;
        (api.json_get_alloc_funcs2)(&mut m4, &mut r4, &mut f4);
        rec.tag_i("m4_is_hook", (m4 == Some(hook_malloc)) as i64);
        rec.tag_i("r4_is_hook", (r4 == Some(hook_realloc)) as i64);
        rec.tag_i("f4_is_hook", (f4 == Some(hook_free)) as i64);

        restore_alloc(api);
    });
}

#[test]
fn cfg07_custom_allocator_pipeline() {
    // The exact number of allocator calls a full parse+dump pipeline performs is
    // observable through a custom allocator, so it must match.
    diff("cfg07 allocator call counts", |api, rec| unsafe {
        for text in [
            r#"{"a":1}"#,
            r#"[1,2,3,4,5,6,7,8,9,10]"#,
            r#"{"a":{"b":[1,2,{"c":"ddddddddddddddddddddddd"}]},"e":1.5,"f":true}"#,
            r#"[]"#,
            r#"{}"#,
        ] {
            for &hooks2 in &[true, false] {
                alloc_reset();
                if hooks2 {
                    install_hooks2(api);
                } else {
                    install_hooks1(api);
                }
                let s = cs(text);
                let mut err = JsonError::patterned();
                let j = (api.json_loads)(s.as_ptr(), 0, &mut err);
                rec.json("loaded", j);
                rec.tag_i("after_load", alloc_count());
                let d = (api.json_dumps)(j, JSON_SORT_KEYS);
                rec.cstring("dumped", d);
                rec.tag_i("after_dump", alloc_count());
                (api.jsonp_free)(d as *mut c_void);
                (api.json_delete)(j);
                rec.tag_i("after_free", alloc_count());
                restore_alloc(api);
                alloc_reset();
            }
        }
    });
}

/* ------------------------------------------- rows 8..12: utf.c ---------- */

#[test]
fn cfg08_utf8_check_first_all_bytes() {
    diff("cfg08 utf8_check_first", |api, rec| unsafe {
        for b in 0..=255u32 {
            rec.tag_u("n", (api.utf8_check_first)(b as u8 as c_char));
        }
    });
}

#[test]
fn cfg09_utf8_check_full() {
    diff("cfg09 utf8_check_full", |api, rec| unsafe {
        let mut rng = Rng::new(0x0900);
        // hand-picked vectors: valid, overlong, surrogate, >max, bad continuation
        let vectors: &[&[u8]] = &[
            b"\xC2\x80",
            b"\xDF\xBF",
            b"\xC0\x80",
            b"\xC1\xBF",
            b"\xE0\xA0\x80",
            b"\xEF\xBF\xBF",
            b"\xE0\x80\x80",
            b"\xED\xA0\x80",
            b"\xED\xBF\xBF",
            b"\xF0\x90\x80\x80",
            b"\xF4\x8F\xBF\xBF",
            b"\xF4\x90\x80\x80",
            b"\xF0\x80\x80\x80",
            b"\xC2\x00",
            b"\xE0\xA0\x00",
            b"\xF0\x90\x80\xFF",
            b"\x41\x42\x43\x44",
        ];
        for v in vectors {
            let mut buf = [0u8; 8];
            buf[..v.len()].copy_from_slice(v);
            for size in 0usize..=6 {
                let mut cp: i32 = -12345;
                let r = (api.utf8_check_full)(buf.as_ptr() as *const c_char, size, &mut cp);
                rec.tag_u("r", r);
                rec.tag_i("cp", cp as i64);
                let r2 = (api.utf8_check_full)(buf.as_ptr() as *const c_char, size, ptr::null_mut());
                rec.tag_u("r_nullcp", r2);
            }
        }
        // randomised
        for _ in 0..2000 {
            let buf = rng.bytes(4, false);
            let size = 2 + rng.below(3);
            let mut cp: i32 = -1;
            let r = (api.utf8_check_full)(buf.as_ptr() as *const c_char, size, &mut cp);
            rec.tag_u("rr", r);
            rec.tag_i("rcp", cp as i64);
        }
        // SIZE_MAX size (row 300)
        let buf = b"\xC2\x80\0\0";
        rec.tag_u(
            "size_max",
            (api.utf8_check_full)(buf.as_ptr() as *const c_char, usize::MAX, ptr::null_mut()),
        );
    });
}

#[test]
fn cfg10_utf8_encode() {
    diff("cfg10 utf8_encode", |api, rec| unsafe {
        let mut cps: Vec<i32> = vec![
            0, 1, 0x7F, 0x80, 0x7FF, 0x800, 0xFFF, 0x1000, 0xD7FF, 0xD800, 0xDFFF, 0xE000, 0xFFFF,
            0x10000, 0x10FFFF, 0x110000, 0x1FFFFF, i32::MAX, -1, -2, i32::MIN,
        ];
        let mut rng = Rng::new(0x1000);
        for _ in 0..500 {
            cps.push(rng.next_u32() as i32);
        }
        for cp in cps {
            let mut buf = [0x5Au8; 8];
            let mut size: usize = 0xDEAD;
            let r = (api.utf8_encode)(cp, buf.as_mut_ptr() as *mut c_char, &mut size);
            rec.tag_i("r", r as i64);
            rec.tag_u("size", size);
            rec.tag_bytes("buf", &buf);
        }
    });
}

#[test]
fn cfg11_utf8_iterate() {
    diff("cfg11 utf8_iterate", |api, rec| unsafe {
        let mut rng = Rng::new(0x1100);
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"a".to_vec(),
            b"\x00".to_vec(),
            b"\x7F".to_vec(),
            b"\xC2".to_vec(),
            b"\xC2\x80".to_vec(),
            b"\xE0\xA0".to_vec(),
            b"\xE0\xA0\x80".to_vec(),
            b"\xF0\x90\x80".to_vec(),
            b"\xF0\x90\x80\x80".to_vec(),
            b"\x80".to_vec(),
            b"\xFF\xFE".to_vec(),
        ];
        for _ in 0..1000 {
            { let n = 1 + rng.below(5); cases.push(rng.bytes(n, false)); }
        }
        for c in cases {
            for bufsize in 0..=c.len() {
                let mut cp: i32 = -777;
                let base = c.as_ptr() as *const c_char;
                let r = (api.utf8_iterate)(base, bufsize, &mut cp);
                if r.is_null() {
                    rec.line("ret=NULL");
                } else {
                    rec.tag_i("ret_off", r.offset_from(base) as i64);
                }
                rec.tag_i("cp", cp as i64);
                let r2 = (api.utf8_iterate)(base, bufsize, ptr::null_mut());
                rec.tag_i("ret2_null", r2.is_null() as i64);
            }
        }
    });
}

#[test]
fn cfg12_utf8_check_string() {
    diff("cfg12 utf8_check_string", |api, rec| unsafe {
        let mut rng = Rng::new(0x1200);
        for _ in 0..1500 {
            let mode = rng.below(3);
            let bytes: Vec<u8> = match mode {
                0 => { let n = 1 + rng.below(8); rng.utf8(n).into_bytes() }
                1 => { let n = 1 + rng.below(10); rng.bytes(n, false) }
                _ => {
                    // valid utf8 truncated in the middle of a sequence
                    let n = 3 + rng.below(5); let mut v = rng.utf8(n).into_bytes();
                    let keep = if v.is_empty() { 0 } else { rng.below(v.len()) };
                    v.truncate(keep);
                    v
                }
            };
            for l in [bytes.len(), bytes.len().saturating_sub(1), 0] {
                rec.tag_i(
                    "ok",
                    (api.utf8_check_string)(bytes.as_ptr() as *const c_char, l) as i64,
                );
            }
        }
        rec.tag_i(
            "empty",
            (api.utf8_check_string)(b"".as_ptr() as *const c_char, 0) as i64,
        );
    });
}

/* --------------------------------------- rows 13..18: strbuffer.c ------- */

fn rec_sb(rec: &mut Rec, api: &Api, tag: &str, sb: &Strbuffer) {
    unsafe {
        rec.tag_u(&format!("{tag}.len"), sb.length);
        rec.tag_u(&format!("{tag}.size"), sb.size);
        rec.tag_ptr_null(&format!("{tag}.value"), sb.value as *const c_void);
        if !sb.value.is_null() {
            rec.tag_bytes(
                &format!("{tag}.bytes"),
                std::slice::from_raw_parts(sb.value as *const u8, sb.length + 1),
            );
            rec.cstring(&format!("{tag}.strbuffer_value"), (api.strbuffer_value)(sb));
        }
    }
}

#[test]
fn cfg13to18_strbuffer() {
    diff("cfg13-18 strbuffer", |api, rec| unsafe {
        // row 13: fresh buffer
        let mut sb = Strbuffer::zeroed();
        rec.tag_i("init", (api.strbuffer_init)(&mut sb) as i64);
        rec_sb(rec, api, "fresh", &sb);

        // row 14: byte-at-a-time across growth boundaries
        for i in 0..80u32 {
            let c = (b'A' + (i % 26) as u8) as c_char;
            rec.tag_i("ab", (api.strbuffer_append_byte)(&mut sb, c) as i64);
            if i % 7 == 0 {
                rec_sb(rec, api, "grow", &sb);
            }
        }
        rec_sb(rec, api, "after80", &sb);

        // row 16/17: pop then clear
        for _ in 0..5 {
            rec.tag_i("pop", (api.strbuffer_pop)(&mut sb) as i64);
        }
        rec_sb(rec, api, "popped", &sb);
        (api.strbuffer_clear)(&mut sb);
        rec_sb(rec, api, "cleared", &sb);
        // pop on empty
        for _ in 0..3 {
            rec.tag_i("pop_empty", (api.strbuffer_pop)(&mut sb) as i64);
        }
        rec_sb(rec, api, "pop_empty_state", &sb);
        (api.strbuffer_close)(&mut sb);
        rec_sb(rec, api, "closed", &sb);

        // row 15: append_bytes with sizes straddling the free space exactly
        for n in [0usize, 1, 14, 15, 16, 17, 31, 32, 33, 100, 1000] {
            let mut sb = Strbuffer::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            let data = vec![b'x'; n];
            rec.tag_i(
                "ab_n",
                (api.strbuffer_append_bytes)(&mut sb, data.as_ptr() as *const c_char, n) as i64,
            );
            rec_sb(rec, api, &format!("abn{n}"), &sb);
            // second append of the same size
            rec.tag_i(
                "ab_n2",
                (api.strbuffer_append_bytes)(&mut sb, data.as_ptr() as *const c_char, n) as i64,
            );
            rec_sb(rec, api, &format!("abn{n}b"), &sb);
            (api.strbuffer_close)(&mut sb);
        }

        // NUL bytes inside the data
        let mut sb = Strbuffer::zeroed();
        assert_eq!((api.strbuffer_init)(&mut sb), 0);
        let data = b"ab\0cd\0\0ef";
        rec.tag_i(
            "ab_nul",
            (api.strbuffer_append_bytes)(&mut sb, data.as_ptr() as *const c_char, data.len())
                as i64,
        );
        rec_sb(rec, api, "nuls", &sb);
        (api.strbuffer_close)(&mut sb);

        // row 18: steal_value then close
        let mut sb = Strbuffer::zeroed();
        assert_eq!((api.strbuffer_init)(&mut sb), 0);
        assert_eq!(
            (api.strbuffer_append_bytes)(&mut sb, b"stolen".as_ptr() as *const c_char, 6),
            0
        );
        let stolen = (api.strbuffer_steal_value)(&mut sb);
        rec.cstring("stolen", stolen);
        rec_sb(rec, api, "after_steal", &sb);
        (api.strbuffer_close)(&mut sb);
        rec_sb(rec, api, "after_steal_close", &sb);
        (api.jsonp_free)(stolen as *mut c_void);

        // randomised interleaving
        let mut rng = Rng::new(0x1300);
        let mut sb = Strbuffer::zeroed();
        assert_eq!((api.strbuffer_init)(&mut sb), 0);
        for _ in 0..400 {
            match rng.below(4) {
                0 => {
                    let n = rng.below(40); let b = rng.bytes(n, true);
                    rec.tag_i(
                        "r_ab",
                        (api.strbuffer_append_bytes)(
                            &mut sb,
                            b.as_ptr() as *const c_char,
                            b.len(),
                        ) as i64,
                    );
                }
                1 => {
                    rec.tag_i(
                        "r_abyte",
                        (api.strbuffer_append_byte)(&mut sb, (rng.below(256)) as c_char) as i64,
                    );
                }
                2 => rec.tag_i("r_pop", (api.strbuffer_pop)(&mut sb) as i64),
                _ => (api.strbuffer_clear)(&mut sb),
            }
            rec.tag_u("r_len", sb.length);
            rec.tag_u("r_size", sb.size);
        }
        rec_sb(rec, api, "rand_final", &sb);
        (api.strbuffer_close)(&mut sb);
    });
}

/* --------------------------------------- rows 19..28: hashtable.c ------- */

unsafe fn rec_ht(rec: &mut Rec, api: &Api, tag: &str, ht: &mut Hashtable) {
    rec.tag_u(&format!("{tag}.size"), ht.size);
    rec.tag_u(&format!("{tag}.order"), ht.order);
    rec.tag_ptr_null(&format!("{tag}.buckets"), ht.buckets as *const c_void);
    // full traversal, in iteration order
    let mut it = (api.hashtable_iter)(ht);
    let mut n = 0;
    while !it.is_null() {
        let k = (api.hashtable_iter_key)(it);
        let kl = (api.hashtable_iter_key_len)(it);
        let v = (api.hashtable_iter_value)(it);
        rec.tag_bytes(
            &format!("{tag}.k{n}"),
            std::slice::from_raw_parts(k as *const u8, kl),
        );
        rec.tag_u(&format!("{tag}.kl{n}"), kl);
        rec.json(&format!("{tag}.v{n}"), v);
        if !v.is_null() && (*v).type_ == JSON_INTEGER {
            rec.tag_i(
                &format!("{tag}.vi{n}"),
                (api.json_integer_value)(v) as i64,
            );
        }
        it = (api.hashtable_iter_next)(ht, it);
        n += 1;
    }
    rec.tag_i(&format!("{tag}.count"), n);
}

#[test]
fn cfg19to28_hashtable() {
    diff("cfg19-28 hashtable", |api, rec| unsafe {
        // row 19: fresh table
        let mut ht = Hashtable::zeroed();
        rec.tag_i("init", (api.hashtable_init)(&mut ht) as i64);
        rec_ht(rec, api, "fresh", &mut ht);
        // row 25 (empty iter) -> covered by rec_ht count == 0

        // row 20: cross both rehash points
        for i in 0..64i64 {
            let key = format!("key{i:03}");
            let k = cs(&key);
            let v = (api.json_integer)(i);
            rec.tag_i(
                "set",
                (api.hashtable_set)(&mut ht, k.as_ptr(), key.len(), v) as i64,
            );
            if [0, 1, 7, 8, 9, 15, 16, 17, 31, 32, 63].contains(&i) {
                rec_ht(rec, api, &format!("after{i}"), &mut ht);
            }
        }

        // row 21: overwrite existing keys
        for i in 0..8i64 {
            let key = format!("key{i:03}");
            let k = cs(&key);
            let v = (api.json_integer)(1000 + i);
            rec.tag_i(
                "overwrite",
                (api.hashtable_set)(&mut ht, k.as_ptr(), key.len(), v) as i64,
            );
        }
        rec_ht(rec, api, "overwritten", &mut ht);

        // row 23: get present / absent / prefix / wrong key_len
        for probe in ["key000", "key063", "key064", "key", "", "KEY000"] {
            let k = cs(probe);
            for kl in [probe.len(), probe.len() + 1, probe.len().saturating_sub(1)] {
                let g = (api.hashtable_get)(&mut ht, k.as_ptr(), kl);
                rec.tag_ptr_null("get", g);
                if !g.is_null() {
                    rec.tag_i("get_v", (api.json_integer_value)(g as *const Json) as i64);
                }
            }
        }

        // row 27: iter_at
        for probe in ["key000", "key031", "nope", ""] {
            let k = cs(probe);
            let it = (api.hashtable_iter_at)(&mut ht, k.as_ptr(), probe.len());
            rec.tag_ptr_null("iter_at", it);
            if !it.is_null() {
                let kl = (api.hashtable_iter_key_len)(it);
                rec.tag_bytes(
                    "iter_at_k",
                    std::slice::from_raw_parts((api.hashtable_iter_key)(it) as *const u8, kl),
                );
                // row 28: set through iterator
                (api.hashtable_iter_set)(it, (api.json_integer)(-7));
                rec.json("iter_at_v", (api.hashtable_iter_value)(it));
                rec.tag_i(
                    "iter_at_vi",
                    (api.json_integer_value)((api.hashtable_iter_value)(it)) as i64,
                );
            }
        }

        // row 24: delete first/middle/last + absent
        for probe in ["key000", "key032", "key063", "key000", "absent"] {
            let k = cs(probe);
            rec.tag_i(
                "del",
                (api.hashtable_del)(&mut ht, k.as_ptr(), probe.len()) as i64,
            );
        }
        rec_ht(rec, api, "after_del", &mut ht);
        // re-insert a deleted key
        let k = cs("key000");
        rec.tag_i(
            "reinsert",
            (api.hashtable_set)(&mut ht, k.as_ptr(), 6, (api.json_integer)(4242)) as i64,
        );
        rec_ht(rec, api, "reinserted", &mut ht);

        // row 25: clear + reuse
        (api.hashtable_clear)(&mut ht);
        rec_ht(rec, api, "cleared", &mut ht);
        for i in 0..10i64 {
            let key = format!("z{i}");
            let k = cs(&key);
            rec.tag_i(
                "set2",
                (api.hashtable_set)(&mut ht, k.as_ptr(), key.len(), (api.json_integer)(i)) as i64,
            );
        }
        rec_ht(rec, api, "refilled", &mut ht);
        (api.hashtable_close)(&mut ht);

        // row 22: key_len 0, embedded NUL, shared prefixes
        let mut ht2 = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht2), 0);
        let keys: &[&[u8]] = &[
            b"",
            b"a",
            b"a\0b",
            b"a\0c",
            b"ab",
            b"abc",
            b"abcd",
            b"\0",
            b"\0\0",
            b"\xff\xfe",
        ];
        for (i, kb) in keys.iter().enumerate() {
            rec.tag_i(
                "setk",
                (api.hashtable_set)(
                    &mut ht2,
                    kb.as_ptr() as *const c_char,
                    kb.len(),
                    (api.json_integer)(i as i64),
                ) as i64,
            );
        }
        rec_ht(rec, api, "binkeys", &mut ht2);
        for kb in keys.iter() {
            let g = (api.hashtable_get)(&mut ht2, kb.as_ptr() as *const c_char, kb.len());
            rec.tag_ptr_null("getk", g);
            if !g.is_null() {
                rec.tag_i("getk_v", (api.json_integer_value)(g as *const Json) as i64);
            }
        }
        (api.hashtable_close)(&mut ht2);

        // randomised mixed workload
        let mut rng = Rng::new(0x1900);
        let mut ht3 = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht3), 0);
        for step in 0..600i64 {
            let key = format!("k{}", rng.below(40));
            let k = cs(&key);
            match rng.below(4) {
                0 | 1 => rec.tag_i(
                    "r_set",
                    (api.hashtable_set)(
                        &mut ht3,
                        k.as_ptr(),
                        key.len(),
                        (api.json_integer)(step),
                    ) as i64,
                ),
                2 => rec.tag_i(
                    "r_del",
                    (api.hashtable_del)(&mut ht3, k.as_ptr(), key.len()) as i64,
                ),
                _ => {
                    let g = (api.hashtable_get)(&mut ht3, k.as_ptr(), key.len());
                    rec.tag_ptr_null("r_get", g);
                }
            }
            rec.tag_u("r_size", ht3.size);
            rec.tag_u("r_order", ht3.order);
        }
        rec_ht(rec, api, "rand", &mut ht3);
        (api.hashtable_close)(&mut ht3);
    });
}

/* ---------------------------------- rows 29..30: seed + loop check ------ */

#[test]
fn cfg29_object_seed() {
    diff("cfg29 json_object_seed", |api, rec| unsafe {
        rec.tag_u("seed_before", *api.hashtable_seed as usize);
        (api.json_object_seed)(0xDEAD_BEEF); // already seeded -> no-op
        rec.tag_u("seed_after_nonzero", *api.hashtable_seed as usize);
        (api.json_object_seed)(0); // autoseed request, still a no-op
        rec.tag_u("seed_after_zero", *api.hashtable_seed as usize);
    });
}

#[test]
fn cfg30_loop_check() {
    diff("cfg30 jsonp_loop_check", |api, rec| unsafe {
        const LOOP_KEY_LEN: usize = 2 + 8 * 2 + 1;
        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        let a = (api.json_array)();
        let b = (api.json_object)();
        for (tag, j) in [("a", a), ("b", b), ("a2", a), ("b2", b)] {
            let mut key = [0u8; LOOP_KEY_LEN];
            let mut kl: usize = 0;
            let r = (api.jsonp_loop_check)(
                &mut ht,
                j,
                key.as_mut_ptr() as *mut c_char,
                LOOP_KEY_LEN,
                &mut kl,
            );
            rec.tag_i(&format!("{tag}.ret"), r as i64);
            // Pointer values differ between the two libraries, so only the
            // *shape* of the generated key is comparable.
            let s = &key[..kl.min(LOOP_KEY_LEN)];
            rec.tag_i(&format!("{tag}.kl_eq_strlen"), (kl == strlen_u8(&key)) as i64);
            rec.tag_i(&format!("{tag}.starts_0x"), s.starts_with(b"0x") as i64);
            rec.tag_i(
                &format!("{tag}.all_hex"),
                s[2..].iter().all(|c| c.is_ascii_hexdigit()) as i64,
            );
            rec.tag_u(&format!("{tag}.size"), ht.size);
        }
        // NULL key_len_out must be tolerated
        let mut key = [0u8; LOOP_KEY_LEN];
        let c = (api.json_array)();
        rec.tag_i(
            "null_out",
            (api.jsonp_loop_check)(
                &mut ht,
                c,
                key.as_mut_ptr() as *mut c_char,
                LOOP_KEY_LEN,
                ptr::null_mut(),
            ) as i64,
        );
        rec.tag_u("size_end", ht.size);
        (api.hashtable_close)(&mut ht);
        (api.json_delete)(a);
        (api.json_delete)(b);
        (api.json_delete)(c);
    });
}

fn strlen_u8(b: &[u8]) -> usize {
    b.iter().position(|&c| c == 0).unwrap_or(b.len())
}

/* --------------------------------------- rows 31..33: strconv.c --------- */

const NUMBER_TEXTS: &[&str] = &[
    "0",
    "-0",
    "1",
    "-1",
    "0.5",
    "-0.5",
    "3.14159265358979",
    "1e5",
    "1E5",
    "1e+5",
    "1e-5",
    "1.5e300",
    "1.5e-300",
    "2.2250738585072014e-308",
    "5e-324",
    "1.7976931348623157e308",
    "123456789012345678901234567890",
    "0.000000000000000000001",
    "9007199254740993",
    "1e16",
    "1e17",
    "-1e16",
    "12345678901234567890.5",
    "1e-400",
    "-1e-400",
];

#[test]
fn cfg31_jsonp_strtod() {
    diff("cfg31 jsonp_strtod", |api, rec| unsafe {
        for t in NUMBER_TEXTS {
            let mut sb = Strbuffer::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            assert_eq!(
                (api.strbuffer_append_bytes)(&mut sb, t.as_ptr() as *const c_char, t.len()),
                0
            );
            let mut out: f64 = -12345.0;
            let r = (api.jsonp_strtod)(&mut sb, &mut out);
            rec.tag_i("ret", r as i64);
            rec.tag_f("out", out);
            (api.strbuffer_close)(&mut sb);
        }
    });
}

#[test]
fn cfg32and33_jsonp_dtostr() {
    diff("cfg32-33 jsonp_dtostr", |api, rec| unsafe {
        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            0.1,
            1.0 / 3.0,
            1e-4,
            1e-5,
            9.9e-5,
            1e15,
            1e16,
            1e17,
            1e18,
            1.2345678901234567e16,
            1.2345678901234567e17,
            f64::MAX,
            -f64::MAX,
            f64::MIN_POSITIVE,
            5e-324,
            123456789012345678.0,
            1e300,
            1e-300,
            2.5,
            3.5,
            -2.5,
        ];
        let mut rng = Rng::new(0x3200);
        for _ in 0..400 {
            vals.push(rng.f64_interesting());
        }
        for v in vals {
            for prec in 0..32i32 {
                for size in [25usize, 32, 64] {
                    let mut buf = [0x5Au8; 80];
                    let r = (api.jsonp_dtostr)(buf.as_mut_ptr() as *mut c_char, size, v, prec);
                    rec.tag_i("ret", r as i64);
                    if r >= 0 {
                        rec.tag_bytes("buf", &buf[..(r as usize) + 1]);
                    }
                }
            }
        }
        // tiny buffers (row 260 in ERRORS.md)
        for size in 0usize..25 {
            let mut buf = [0x5Au8; 80];
            let r = (api.jsonp_dtostr)(buf.as_mut_ptr() as *mut c_char, size, 1.5, 0);
            rec.tag_i("tiny", r as i64);
        }
    });
}

/* --------------------------------------- rows 34..38: dtoa.c ----------- */

#[test]
fn cfg34_dtoa_r() {
    diff("cfg34 dtoa_r", |api, rec| unsafe {
        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 1e16, 1e17, 1e-5, f64::MAX, 5e-324, 1.0 / 3.0,
            1234.5678, 1e300, 9007199254740993.0,
        ];
        let mut rng = Rng::new(0x3400);
        for _ in 0..250 {
            vals.push(rng.f64_interesting());
        }
        for v in vals {
            for mode in 0..7i32 {
                for nd in [0i32, 1, 2, 5, 15, 17, 20, 25, 30, -1] {
                    for blen in [25usize, 40] {
                        let mut buf = [0x5Au8; 64];
                        let mut decpt: c_int = -999;
                        let mut sign: c_int = -999;
                        let mut rve: *mut c_char = ptr::null_mut();
                        let r = (api.dtoa_r)(
                            v,
                            mode,
                            nd,
                            &mut decpt,
                            &mut sign,
                            &mut rve,
                            buf.as_mut_ptr() as *mut c_char,
                            blen,
                        );
                        if r.is_null() {
                            rec.line("dtoa_r=NULL");
                        } else {
                            rec.cstring("dtoa_r", r);
                            rec.tag_i("decpt", decpt as i64);
                            rec.tag_i("sign", sign as i64);
                            rec.tag_i(
                                "rve_off",
                                if rve.is_null() {
                                    -1
                                } else {
                                    rve.offset_from(r) as i64
                                },
                            );
                        }
                    }
                }
            }
        }
    });
}

#[test]
fn cfg35_dtoa_and_freedtoa() {
    diff("cfg35 dtoa/freedtoa", |api, rec| unsafe {
        let mut vals: Vec<f64> = vec![0.0, -0.0, 1.0, -1.5, 1e16, 1e-5, f64::MAX, 5e-324];
        let mut rng = Rng::new(0x3500);
        for _ in 0..150 {
            vals.push(rng.f64_interesting());
        }
        for v in vals {
            for mode in 0..6i32 {
                for nd in [0i32, 1, 6, 17, 25] {
                    let mut decpt: c_int = -999;
                    let mut sign: c_int = -999;
                    let mut rve: *mut c_char = ptr::null_mut();
                    let r = (api.dtoa)(v, mode, nd, &mut decpt, &mut sign, &mut rve);
                    if r.is_null() {
                        rec.line("dtoa=NULL");
                    } else {
                        rec.cstring("dtoa", r);
                        rec.tag_i("decpt", decpt as i64);
                        rec.tag_i("sign", sign as i64);
                        rec.tag_i(
                            "rve_off",
                            if rve.is_null() {
                                -1
                            } else {
                                rve.offset_from(r) as i64
                            },
                        );
                        (api.freedtoa)(r);
                    }
                }
            }
        }
        // repeated calls without freeing recycle the static result buffer
        for _ in 0..5 {
            let mut decpt: c_int = 0;
            let mut sign: c_int = 0;
            let mut rve: *mut c_char = ptr::null_mut();
            let r = (api.dtoa)(2.718281828459045, 0, 0, &mut decpt, &mut sign, &mut rve);
            rec.cstring("recycled", r);
        }
    });
}

#[test]
fn cfg36_strtod_unused() {
    diff("cfg36 strtod__unused", |api, rec| unsafe {
        let inputs: &[&str] = &[
            "0", "-0", "1", "  12.5", "+3.5e2", "1e", "1e+", ".5", "-.5", ".", "", "x", "0x10",
            "0X1p4", "0x1.8p1", "0x.8p1", "0xp1", "0x", "1e400", "-1e400", "1e-400", "inf",
            "infinity", "nan", "nan(1)", "1.7976931348623157e309", "4.9406564584124654e-324",
            "9007199254740993", "0.000000000000000000000000001", "1234567890123456789012345",
            "0x1fffffffffffffp0", "0x1p-1080", "0x1p1080", "\t\n 42abc",
        ];
        for s in inputs {
            let cstr = cs(s);
            let mut end: *mut c_char = ptr::null_mut();
            let v = (api.strtod__unused)(cstr.as_ptr(), &mut end);
            rec.tag_f("val", v);
            rec.tag_i(
                "end_off",
                if end.is_null() {
                    -1
                } else {
                    end.offset_from(cstr.as_ptr()) as i64
                },
            );
            // NULL se
            let v2 = (api.strtod__unused)(cstr.as_ptr(), ptr::null_mut());
            rec.tag_f("val_nullse", v2);
        }
    });
}

#[test]
fn cfg37_gethex() {
    diff("cfg37 gethex", |api, rec| unsafe {
        let inputs: &[&str] = &[
            "0x1", "0x1p0", "0x1p1", "0x1p-1", "0x1.8p1", "0x.8p1", "0x0p0", "0x", "0xp1",
            "0x1fffffffffffffp0", "0x1p1080", "0x1p-1080", "0x1p+10", "0X1ABCDEFp-4",
            "0x0000001p0", "0x1.fffffffffffffp1023", "0x1p-1074", "0x1p-1075", "0x8000000000000p0",
            "0x1234567890abcdefp+3",
        ];
        for s in inputs {
            for rounding in 0..4i32 {
                for sign in 0..2i32 {
                    let cstr = cs(s);
                    let mut sp: *const c_char = cstr.as_ptr();
                    let mut u = U { L: [0x5A5A_5A5A, 0x5A5A_5A5A] };
                    (api.gethex)(&mut sp, &mut u, rounding, sign);
                    rec.tag_f("d", u.d);
                    rec.tag_i("sp_off", sp.offset_from(cstr.as_ptr()) as i64);
                }
            }
        }
    });
}

#[test]
fn cfg38_dtoa_divmax() {
    diff("cfg38 dtoa_divmax", |api, rec| unsafe {
        rec.tag_i("dtoa_divmax", *api.dtoa_divmax as i64);
    });
}

/* --------------------------------------- rows 39..41: error.c ---------- */

#[test]
fn cfg39_error_init() {
    diff("cfg39 jsonp_error_init", |api, rec| unsafe {
        for src in [Some(""), Some("x"), Some("<string>"), None] {
            let mut e = JsonError::patterned();
            match src {
                Some(s) => {
                    let c = cs(s);
                    (api.jsonp_error_init)(&mut e, c.as_ptr());
                }
                None => (api.jsonp_error_init)(&mut e, ptr::null()),
            }
            rec.error("init", &e);
        }
        // error == NULL
        (api.jsonp_error_init)(ptr::null_mut(), ptr::null());
        let c = cs("src");
        (api.jsonp_error_init)(ptr::null_mut(), c.as_ptr());
        rec.line("null_error_ok");
    });
}

#[test]
fn cfg40_error_set_source() {
    diff("cfg40 jsonp_error_set_source", |api, rec| unsafe {
        for len in [0usize, 1, 2, 77, 78, 79, 80, 81, 82, 100, 160, 300] {
            let s: String = (0..len).map(|i| (b'a' + (i % 26) as u8) as char).collect();
            let c = cs(&s);
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set_source)(&mut e, c.as_ptr());
            rec.error(&format!("src{len}"), &e);
        }
        // NULLs
        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set_source)(&mut e, ptr::null());
        rec.error("null_source", &e);
        (api.jsonp_error_set_source)(ptr::null_mut(), ptr::null());
        let c = cs("abc");
        (api.jsonp_error_set_source)(ptr::null_mut(), c.as_ptr());
        rec.line("null_error_ok");
    });
}

#[test]
fn cfg41_error_set_variadic() {
    diff("cfg41 jsonp_error_set", |api, rec| unsafe {
        let long: String = (0..300).map(|i| (b'A' + (i % 26) as u8) as char).collect();
        let long_c = cs(&long);
        let fmt_s = cs("msg %s");
        let fmt_d = cs("num %d and %d");
        let fmt_c = cs("chr '%c'");
        let fmt_p6 = cs("esc '%.6s'");
        let fmt_plain = cs("plain");
        let arg = cs("argument");

        // every enum value + out-of-range codes (ERRORS.md row 242)
        for code in [0i32, 1, 5, 17, 18, 100, 127, 128, 200, 255, 256, -1, -128] {
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set)(&mut e, 3, 4, 5usize, code, fmt_s.as_ptr(), arg.as_ptr());
            rec.error(&format!("code{code}"), &e);
        }

        // formats
        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set)(&mut e, -1, -1, 0usize, 8, fmt_d.as_ptr(), 42i32, -7i32);
        rec.error("fmt_d", &e);

        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set)(&mut e, 1, 2, 3usize, 8, fmt_c.as_ptr(), b'Z' as i32);
        rec.error("fmt_c", &e);

        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set)(&mut e, 1, 2, 3usize, 8, fmt_p6.as_ptr(), long_c.as_ptr());
        rec.error("fmt_p6", &e);

        // truncation of a >158 byte message
        let mut e = JsonError::patterned();
        (api.jsonp_error_init)(&mut e, ptr::null());
        (api.jsonp_error_set)(&mut e, 9, 9, 9usize, 6, fmt_s.as_ptr(), long_c.as_ptr());
        rec.error("truncated", &e);

        // second call must be ignored (error already set)
        (api.jsonp_error_set)(&mut e, 1, 1, 1usize, 1, fmt_plain.as_ptr());
        rec.error("second_ignored", &e);

        // NULL error
        (api.jsonp_error_set)(ptr::null_mut(), 1, 1, 1usize, 1, fmt_plain.as_ptr());
        rec.line("null_error_ok");

        // extreme line/column/position
        for (l, c2, p) in [
            (i32::MIN, i32::MAX, 0usize),
            (0, 0, usize::MAX),
            (-1, -1, 1usize << 40),
        ] {
            let mut e = JsonError::patterned();
            (api.jsonp_error_init)(&mut e, ptr::null());
            (api.jsonp_error_set)(&mut e, l, c2, p, 2, fmt_plain.as_ptr());
            rec.error("extremes", &e);
        }
    });
}
