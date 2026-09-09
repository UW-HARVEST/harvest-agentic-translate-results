//! Phase C — error-path differential tests, one per ERRORS.md row.
#![allow(unused_unsafe, dead_code, unsafe_op_in_unsafe_fn)]
mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int, c_longlong, c_void};

const ANY: usize = JSON_ENCODE_ANY | JSON_SORT_KEYS;

macro_rules! diff {
    ($name:expr, $body:expr) => {{
        let (c, r) = both();
        let a = unsafe { $body(c) };
        let b = unsafe { $body(r) };
        assert_eq!(a, b, "{}", $name);
    }};
}

// ============================== rows 1-17: utf.c ==========================

#[test]
fn err_rows_01_17_utf() {
    let (c, r) = both();
    // rows 1,2: utf8_encode rejects
    for cp in [-1i32, i32::MIN, 0x110000, 0x1FFFFF, i32::MAX] {
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "utf8_encode", (i32, *mut c_char, *mut usize) -> c_int);
            let mut b = [0x5Au8; 8];
            let mut s: usize = 0xDEAD;
            out.push((unsafe { f(cp, b.as_mut_ptr() as *mut c_char, &mut s) }, s, b));
        }
        assert_eq!(out[0], out[1], "utf8_encode({}) reject", cp);
        assert_eq!(out[0].0, -1);
    }
    // rows 3,4,5: utf8_check_first rejects
    for b in [0x80u8, 0xBF, 0xC0, 0xC1, 0xF5, 0xFE, 0xFF] {
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "utf8_check_first", (c_char) -> usize);
            out.push(unsafe { f(b as c_char) });
        }
        assert_eq!(out[0], out[1]);
        assert_eq!(out[0], 0, "utf8_check_first(0x{:02x}) must be 0", b);
    }
    // rows 6-10: utf8_check_full rejects
    let cases: Vec<(Vec<u8>, usize)> = vec![
        (vec![0xC2, 0xA9], 0),
        (vec![0xC2, 0xA9], 1),
        (vec![0xC2, 0xA9], 5),
        (vec![0xC2, 0x41], 2),
        (vec![0xF4, 0x90, 0x80, 0x80], 4),
        (vec![0xED, 0xA0, 0x80], 3),
        (vec![0xED, 0xBF, 0xBF], 3),
        (vec![0xC0, 0x80], 2),
        (vec![0xE0, 0x80, 0x80], 3),
        (vec![0xF0, 0x80, 0x80, 0x80], 4),
    ];
    for (buf, size) in cases {
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "utf8_check_full", (*const c_char, usize, *mut i32) -> usize);
            let mut cp = -1i32;
            out.push((
                unsafe { f(buf.as_ptr() as *const c_char, size, &mut cp) },
                cp,
            ));
        }
        assert_eq!(out[0], out[1], "utf8_check_full {:02x?}/{}", buf, size);
        assert_eq!(out[0].0, 0);
    }
    // rows 11-14: utf8_iterate
    for (buf, size, expect_null) in [
        (vec![0x41u8], 0usize, false), // row 11: bufsize 0 -> returns buffer
        (vec![0x80], 1, true),
        (vec![0xC2], 1, true), // truncated
        (vec![0xC2, 0x41], 2, true),
        (vec![0xED, 0xA0, 0x80], 3, true),
    ] {
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "utf8_iterate", (*const c_char, usize, *mut i32) -> *const c_char);
            let mut cp = -1i32;
            let p = unsafe { f(buf.as_ptr() as *const c_char, size, &mut cp) };
            out.push((
                p.is_null(),
                if p.is_null() {
                    0
                } else {
                    unsafe { p.offset_from(buf.as_ptr() as *const c_char) }
                },
                cp,
            ));
        }
        assert_eq!(out[0], out[1], "utf8_iterate {:02x?}/{}", buf, size);
        assert_eq!(out[0].0, expect_null);
    }
    // rows 15-17: utf8_check_string
    for (buf, len) in [
        (vec![0x80u8], 1usize),
        (vec![0xC2], 1),
        (vec![0xE2, 0x82], 2),
        (vec![0xED, 0xA0, 0x80], 3),
        (vec![0x41, 0xC0, 0x80], 3),
        (vec![0x41, 0xC2], 2),
    ] {
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "utf8_check_string", (*const c_char, usize) -> c_int);
            out.push(unsafe { f(buf.as_ptr() as *const c_char, len) });
        }
        assert_eq!(out[0], out[1], "utf8_check_string {:02x?}", buf);
        assert_eq!(out[0], 0);
    }
}

// ======================= rows 18-23: strbuffer.c =========================

#[test]
fn err_rows_18_23_strbuffer() {
    let (c, r) = both();
    // rows 19-21: overflow guards.  A real allocation of SIZE_MAX/2 is
    // impossible, so drive the guard by forging the strbuffer state.  The
    // guard is only reached when `size >= strbuff->size - strbuff->length`.
    for (size, length, add) in [
        (usize::MAX / 2 + 1, 0usize, usize::MAX / 2 + 1), // size > SIZE_MAX/2
        (usize::MAX / 2 + 1, 4, usize::MAX),
        (16usize, 0, usize::MAX),         // add > SIZE_MAX - 1
        (16usize, 8, usize::MAX - 8),     // length > SIZE_MAX - 1 - add
        (16usize, 1, usize::MAX - 1),     // length > SIZE_MAX - 1 - add
    ] {
        let mut out = Vec::new();
        for l in [c, r] {
            let ap = sym!(l, "strbuffer_append_bytes", (*mut StrBuffer, *const c_char, usize) -> c_int);
            let malloc = sym!(l, "jsonp_malloc", (usize) -> *mut c_void);
            let free = sym!(l, "jsonp_free", (*mut c_void));
            let mut sb = StrBuffer::zeroed();
            unsafe {
                // Allocate through the library so any realloc it attempts is
                // symmetric with its own allocator.
                let backing = malloc(32);
                sb.value = backing as *mut c_char;
                sb.size = size;
                sb.length = length;
                let data = [0u8; 1];
                out.push(ap(&mut sb, data.as_ptr() as *const c_char, add));
                free(backing);
            }
        }
        assert_eq!(
            out[0], out[1],
            "strbuffer_append_bytes overflow guard size={} len={} add={}",
            size, length, add
        );
        assert_eq!(
            out[0], -1,
            "expected the overflow guard to reject size={} len={} add={}",
            size, length, add
        );
    }
    // row 23: pop on empty
    let mut out = Vec::new();
    for l in [c, r] {
        let init = sym!(l, "strbuffer_init", (*mut StrBuffer) -> c_int);
        let pop = sym!(l, "strbuffer_pop", (*mut StrBuffer) -> c_char);
        let close = sym!(l, "strbuffer_close", (*mut StrBuffer));
        let mut sb = StrBuffer::zeroed();
        unsafe {
            init(&mut sb);
            let a = pop(&mut sb);
            let b = pop(&mut sb);
            let l0 = sb.length;
            close(&mut sb);
            out.push((a as u8, b as u8, l0, sb.size, sb.length, sb.value.is_null()));
        }
    }
    assert_eq!(out[0], out[1], "strbuffer_pop on empty");
    assert_eq!(out[0].0, 0);
}

// ========================= rows 24-29: memory.c ==========================

#[test]
fn err_rows_24_29_memory() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        let malloc = sym!(l, "jsonp_malloc", (usize) -> *mut c_void);
        let free = sym!(l, "jsonp_free", (*mut c_void));
        let get1 = sym!(l, "json_get_alloc_funcs", (*mut Pfn, *mut Pfn));
        let get2 = sym!(l, "json_get_alloc_funcs2", (*mut Pfn, *mut Pfn, *mut Pfn));
        unsafe {
            let a = malloc(0).is_null(); // row 24
            free(std::ptr::null_mut()); // row 25
            get1(std::ptr::null_mut(), std::ptr::null_mut()); // row 28
            get2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()); // row 29
            let mut m: Pfn = std::ptr::null();
            get1(&mut m, std::ptr::null_mut());
            let mut f2: Pfn = std::ptr::null();
            get2(std::ptr::null_mut(), std::ptr::null_mut(), &mut f2);
            out.push((a, !m.is_null(), !f2.is_null()));
        }
    }
    assert_eq!(out[0], out[1], "memory.c NULL tolerance");
    assert_eq!(out[0].0, true);
}

// ========================== rows 30-35: error.c ==========================

#[test]
fn err_rows_30_35_error() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        let init = sym!(l, "jsonp_error_init", (*mut JsonError, *const c_char));
        let set_src = sym!(l, "jsonp_error_set_source", (*mut JsonError, *const c_char));
        let set = sym!(l, "jsonp_error_set", (*mut JsonError, c_int, c_int, usize, c_int, *const c_char));
        unsafe {
            // rows 30, 31, 33: NULL tolerance (must not crash)
            let src = cstr("s");
            init(std::ptr::null_mut(), src.as_ptr());
            set_src(std::ptr::null_mut(), src.as_ptr());
            set(std::ptr::null_mut(), 1, 2, 3, 4, src.as_ptr());
            // row 32: source longer than JSON_ERROR_SOURCE_LENGTH
            let mut e = JsonError::new();
            e.source = [0x7f; ERR_SRC_LEN];
            e.text = [0x7f; ERR_TEXT_LEN];
            let long = cstr(&"Z".repeat(300));
            init(&mut e, long.as_ptr());
            let a = e.raw();
            // row 31 (NULL source arg is a no-op)
            set_src(&mut e, std::ptr::null());
            let b = e.raw();
            // rows 34, 35
            let msg = cstr(&"m".repeat(500));
            set(&mut e, 7, 8, 9, 12, msg.as_ptr());
            let d = e.raw();
            let msg2 = cstr("ignored");
            set(&mut e, 1, 1, 1, 1, msg2.as_ptr());
            let f = e.raw();
            out.push((a, b, d, f));
        }
    }
    assert_eq!(out[0], out[1], "error.c surface");
}

// ======================== rows 36-39: strconv.c ==========================

#[test]
fn err_rows_36_39_strconv() {
    let (c, r) = both();
    // row 36: overflow -> -1
    for t in ["1e400", "-1e400", "1e999999", "-1e999999"] {
        let mut out = Vec::new();
        for l in [c, r] {
            let init = sym!(l, "strbuffer_init", (*mut StrBuffer) -> c_int);
            let ap = sym!(l, "strbuffer_append_bytes", (*mut StrBuffer, *const c_char, usize) -> c_int);
            let close = sym!(l, "strbuffer_close", (*mut StrBuffer));
            let f = sym!(l, "jsonp_strtod", (*mut StrBuffer, *mut c_double) -> c_int);
            let mut sb = StrBuffer::zeroed();
            unsafe {
                init(&mut sb);
                ap(&mut sb, t.as_ptr() as *const c_char, t.len());
                let mut d = -7.0f64;
                let rc = f(&mut sb, &mut d);
                close(&mut sb);
                out.push((rc, d.to_bits()));
            }
        }
        assert_eq!(out[0], out[1], "jsonp_strtod overflow {:?}", t);
        assert_eq!(out[0].0, -1);
    }
    // row 39: jsonp_dtostr buffer too short
    for (v, size, prec) in [
        (1.0f64, 1usize, 0i32),
        (1.0, 2, 0),
        (1.0, 3, 0),
        (1e300, 5, 0),
        (-1.234567890123e-300, 8, 0),
        (0.1, 1, 17),
        (f64::MAX, 4, 0),
    ] {
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "jsonp_dtostr", (*mut c_char, usize, c_double, c_int) -> c_int);
            let mut b = vec![0x5Au8; 64];
            out.push((
                unsafe { f(b.as_mut_ptr() as *mut c_char, size, v, prec) },
                b,
            ));
        }
        assert_eq!(
            out[0], out[1],
            "jsonp_dtostr short buffer v={:e} size={} prec={}",
            v, size, prec
        );
    }
}

// ======================= rows 40-46: hashtable.c =========================

#[test]
fn err_rows_40_46_hashtable() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        let init = sym!(l, "hashtable_init", (*mut HashTable) -> c_int);
        let close = sym!(l, "hashtable_close", (*mut HashTable));
        let get = sym!(l, "hashtable_get", (*mut HashTable, *const c_char, usize) -> *mut c_void);
        let del = sym!(l, "hashtable_del", (*mut HashTable, *const c_char, usize) -> c_int);
        let iter = sym!(l, "hashtable_iter", (*mut HashTable) -> *mut c_void);
        let iter_at = sym!(l, "hashtable_iter_at", (*mut HashTable, *const c_char, usize) -> *mut c_void);
        let iter_next = sym!(l, "hashtable_iter_next", (*mut HashTable, *mut c_void) -> *mut c_void);
        let set = sym!(l, "hashtable_set", (*mut HashTable, *const c_char, usize, *mut JsonT) -> c_int);
        let null = sym!(l, "json_null", () -> *mut JsonT);
        let mut ht = HashTable::zeroed();
        unsafe {
            let rc = init(&mut ht);
            let k = b"missing";
            let a = get(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null(); // row 42
            let b = del(&mut ht, k.as_ptr() as *const c_char, k.len()); // row 43
            let d = iter(&mut ht).is_null(); // row 44
            let e = iter_at(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null(); // row 45
            let kk = b"one";
            set(&mut ht, kk.as_ptr() as *const c_char, kk.len(), null());
            let it = iter(&mut ht);
            let f = iter_next(&mut ht, it).is_null(); // row 46
            close(&mut ht);
            out.push((rc, a, b, d, e, f));
        }
    }
    assert_eq!(out[0], out[1], "hashtable error surface");
    assert_eq!(out[0], (0, true, -1, true, true, true));
}

// ==================== rows 47-115: value.c rejections ====================

/// A forged `json_t` whose `type` byte has no valid variant (ERRORS row 205).
fn forged(typ: c_int) -> Box<JsonT> {
    Box::new(JsonT {
        typ,
        refcount: usize::MAX, // never freed by the library
    })
}

#[test]
fn err_rows_47_73_object_rejections() {
    let (c, r) = both();
    let key = cstr("k");
    let mut out = Vec::new();
    for l in [c, r] {
        unsafe {
            let obj = sym!(l, "json_object", () -> *mut JsonT)();
            let arr = sym!(l, "json_array", () -> *mut JsonT)();
            let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
            let mut v: Vec<String> = Vec::new();

            let size = sym!(l, "json_object_size", (*const JsonT) -> usize);
            v.push(format!("size(NULL)={}", size(std::ptr::null())));
            v.push(format!("size(arr)={}", size(arr)));

            let get = sym!(l, "json_object_get", (*const JsonT, *const c_char) -> *mut JsonT);
            v.push(format!("get(obj,NULL)={}", get(obj, std::ptr::null()).is_null()));
            v.push(format!("get(NULL,k)={}", get(std::ptr::null(), key.as_ptr()).is_null()));
            v.push(format!("get(arr,k)={}", get(arr, key.as_ptr()).is_null()));

            let getn = sym!(l, "json_object_getn", (*const JsonT, *const c_char, usize) -> *mut JsonT);
            v.push(format!("getn(obj,NULL,0)={}", getn(obj, std::ptr::null(), 0).is_null()));
            v.push(format!("getn(arr,k,1)={}", getn(arr, key.as_ptr(), 1).is_null()));

            let set_nc = sym!(l, "json_object_set_new_nocheck", (*mut JsonT, *const c_char, *mut JsonT) -> c_int);
            v.push(format!("set_nc(obj,NULL)={}", set_nc(obj, std::ptr::null(), integer(1))));
            let setn_nc = sym!(l, "json_object_setn_new_nocheck", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
            v.push(format!("setn_nc(obj,k,NULLval)={}", setn_nc(obj, key.as_ptr(), 1, std::ptr::null_mut())));
            v.push(format!("setn_nc(arr,k,v)={}", setn_nc(arr, key.as_ptr(), 1, integer(1))));
            v.push(format!("setn_nc(obj,k,obj)={}", setn_nc(obj, key.as_ptr(), 1, obj)));
            v.push(format!("setn_nc(obj,NULL,1,v)={}", setn_nc(obj, std::ptr::null(), 1, integer(1))));

            let set = sym!(l, "json_object_set_new", (*mut JsonT, *const c_char, *mut JsonT) -> c_int);
            v.push(format!("set(obj,NULL)={}", set(obj, std::ptr::null(), integer(1))));
            let setn = sym!(l, "json_object_setn_new", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
            v.push(format!("setn(obj,NULL,0,v)={}", setn(obj, std::ptr::null(), 0, integer(1))));
            for bad in [b"\x80".as_ref(), b"\xC0\x80".as_ref(), b"\xED\xA0\x80".as_ref()] {
                v.push(format!(
                    "setn(obj,badutf8)={}",
                    setn(obj, bad.as_ptr() as *const c_char, bad.len(), integer(1))
                ));
            }

            let del = sym!(l, "json_object_del", (*mut JsonT, *const c_char) -> c_int);
            v.push(format!("del(obj,NULL)={}", del(obj, std::ptr::null())));
            v.push(format!("del(obj,missing)={}", del(obj, key.as_ptr())));
            let deln = sym!(l, "json_object_deln", (*mut JsonT, *const c_char, usize) -> c_int);
            v.push(format!("deln(arr,k,1)={}", deln(arr, key.as_ptr(), 1)));
            v.push(format!("deln(obj,NULL,0)={}", deln(obj, std::ptr::null(), 0)));

            let clear = sym!(l, "json_object_clear", (*mut JsonT) -> c_int);
            v.push(format!("clear(arr)={}", clear(arr)));
            v.push(format!("clear(NULL)={}", clear(std::ptr::null_mut())));

            for name in [
                "json_object_update",
                "json_object_update_existing",
                "json_object_update_missing",
                "json_object_update_recursive",
            ] {
                let f = match name {
                    "json_object_update" => sym!(l, "json_object_update", (*mut JsonT, *mut JsonT) -> c_int),
                    "json_object_update_existing" => sym!(l, "json_object_update_existing", (*mut JsonT, *mut JsonT) -> c_int),
                    "json_object_update_missing" => sym!(l, "json_object_update_missing", (*mut JsonT, *mut JsonT) -> c_int),
                    _ => sym!(l, "json_object_update_recursive", (*mut JsonT, *mut JsonT) -> c_int),
                };
                v.push(format!("{}(arr,obj)={}", name, f(arr, obj)));
                v.push(format!("{}(obj,arr)={}", name, f(obj, arr)));
                v.push(format!("{}(NULL,obj)={}", name, f(std::ptr::null_mut(), obj)));
                v.push(format!("{}(obj,NULL)={}", name, f(obj, std::ptr::null_mut())));
            }

            let iter = sym!(l, "json_object_iter", (*mut JsonT) -> *mut c_void);
            v.push(format!("iter(arr)={}", iter(arr).is_null()));
            v.push(format!("iter(NULL)={}", iter(std::ptr::null_mut()).is_null()));
            let iter_at = sym!(l, "json_object_iter_at", (*mut JsonT, *const c_char) -> *mut c_void);
            v.push(format!("iter_at(obj,NULL)={}", iter_at(obj, std::ptr::null()).is_null()));
            v.push(format!("iter_at(arr,k)={}", iter_at(arr, key.as_ptr()).is_null()));
            let iter_next = sym!(l, "json_object_iter_next", (*mut JsonT, *mut c_void) -> *mut c_void);
            v.push(format!("iter_next(obj,NULL)={}", iter_next(obj, std::ptr::null_mut()).is_null()));
            v.push(format!("iter_next(arr,NULL)={}", iter_next(arr, std::ptr::null_mut()).is_null()));
            let iter_key = sym!(l, "json_object_iter_key", (*mut c_void) -> *const c_char);
            v.push(format!("iter_key(NULL)={}", iter_key(std::ptr::null_mut()).is_null()));
            let iter_klen = sym!(l, "json_object_iter_key_len", (*mut c_void) -> usize);
            v.push(format!("iter_key_len(NULL)={}", iter_klen(std::ptr::null_mut())));
            let iter_val = sym!(l, "json_object_iter_value", (*mut c_void) -> *mut JsonT);
            v.push(format!("iter_value(NULL)={}", iter_val(std::ptr::null_mut()).is_null()));
            let iter_set = sym!(l, "json_object_iter_set_new", (*mut JsonT, *mut c_void, *mut JsonT) -> c_int);
            v.push(format!("iter_set(arr,NULL,v)={}", iter_set(arr, std::ptr::null_mut(), integer(1))));
            v.push(format!("iter_set(obj,NULL,v)={}", iter_set(obj, std::ptr::null_mut(), integer(1))));
            let ck = cstr("real");
            set(obj, ck.as_ptr(), integer(1));
            let it = iter_at(obj, ck.as_ptr());
            v.push(format!("iter_set(obj,it,NULL)={}", iter_set(obj, it, std::ptr::null_mut())));
            let k2i = sym!(l, "json_object_key_to_iter", (*const c_char) -> *mut c_void);
            v.push(format!("key_to_iter(NULL)={}", k2i(std::ptr::null()).is_null()));

            decref(l, obj);
            decref(l, arr);
            out.push(v);
        }
    }
    assert_eq!(out[0], out[1], "object rejections");
}

#[test]
fn err_rows_74_88_array_rejections() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        unsafe {
            let arr = sym!(l, "json_array", () -> *mut JsonT)();
            let obj = sym!(l, "json_object", () -> *mut JsonT)();
            let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
            let app = sym!(l, "json_array_append_new", (*mut JsonT, *mut JsonT) -> c_int);
            let mut v: Vec<String> = Vec::new();

            let size = sym!(l, "json_array_size", (*const JsonT) -> usize);
            v.push(format!("size(NULL)={} size(obj)={}", size(std::ptr::null()), size(obj)));
            let get = sym!(l, "json_array_get", (*const JsonT, usize) -> *mut JsonT);
            v.push(format!("get(obj,0)={}", get(obj, 0).is_null()));
            v.push(format!("get(NULL,0)={}", get(std::ptr::null(), 0).is_null()));
            for i in [0usize, 1, 2, usize::MAX, usize::MAX - 1] {
                v.push(format!("get(empty,{})={}", i, get(arr, i).is_null()));
            }
            let setn = sym!(l, "json_array_set_new", (*mut JsonT, usize, *mut JsonT) -> c_int);
            v.push(format!("set(arr,0,NULL)={}", setn(arr, 0, std::ptr::null_mut())));
            v.push(format!("set(obj,0,v)={}", setn(obj, 0, integer(1))));
            v.push(format!("set(arr,0,arr)={}", setn(arr, 0, arr)));
            v.push(format!("set(arr,0,v)={}", setn(arr, 0, integer(1))));
            v.push(format!("set(NULL,0,v)={}", setn(std::ptr::null_mut(), 0, integer(1))));
            v.push(format!("append(arr,NULL)={}", app(arr, std::ptr::null_mut())));
            v.push(format!("append(obj,v)={}", app(obj, integer(1))));
            v.push(format!("append(arr,arr)={}", app(arr, arr)));
            v.push(format!("append(NULL,v)={}", app(std::ptr::null_mut(), integer(1))));
            let ins = sym!(l, "json_array_insert_new", (*mut JsonT, usize, *mut JsonT) -> c_int);
            v.push(format!("ins(arr,0,NULL)={}", ins(arr, 0, std::ptr::null_mut())));
            v.push(format!("ins(obj,0,v)={}", ins(obj, 0, integer(1))));
            v.push(format!("ins(arr,0,arr)={}", ins(arr, 0, arr)));
            v.push(format!("ins(arr,1,v)={}", ins(arr, 1, integer(1))));
            v.push(format!("ins(arr,MAX,v)={}", ins(arr, usize::MAX, integer(1))));
            v.push(format!("ins(NULL,0,v)={}", ins(std::ptr::null_mut(), 0, integer(1))));
            let rem = sym!(l, "json_array_remove", (*mut JsonT, usize) -> c_int);
            v.push(format!("rem(obj,0)={}", rem(obj, 0)));
            v.push(format!("rem(arr,0)={}", rem(arr, 0)));
            v.push(format!("rem(arr,MAX)={}", rem(arr, usize::MAX)));
            v.push(format!("rem(NULL,0)={}", rem(std::ptr::null_mut(), 0)));
            let clear = sym!(l, "json_array_clear", (*mut JsonT) -> c_int);
            v.push(format!("clear(obj)={}", clear(obj)));
            v.push(format!("clear(NULL)={}", clear(std::ptr::null_mut())));
            let ext = sym!(l, "json_array_extend", (*mut JsonT, *mut JsonT) -> c_int);
            v.push(format!("ext(obj,arr)={}", ext(obj, arr)));
            v.push(format!("ext(arr,obj)={}", ext(arr, obj)));
            v.push(format!("ext(NULL,arr)={}", ext(std::ptr::null_mut(), arr)));
            v.push(format!("ext(arr,NULL)={}", ext(arr, std::ptr::null_mut())));

            // now with one element, index bounds again
            app(arr, integer(5));
            v.push(format!("get(1)={} set(1)={} rem(1)={} ins(2)={}",
                get(arr, 1).is_null(),
                setn(arr, 1, integer(1)),
                rem(arr, 1),
                ins(arr, 2, integer(1))));
            v.push(format!("dump={:?}", dumps(l, arr, ANY)));
            decref(l, arr);
            decref(l, obj);
            out.push(v);
        }
    }
    assert_eq!(out[0], out[1], "array rejections");
}

#[test]
fn err_rows_89_115_scalar_and_misc_rejections() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        unsafe {
            let mut v: Vec<String> = Vec::new();
            let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
            let arr = sym!(l, "json_array", () -> *mut JsonT)();

            // rows 89, 90
            for name in ["json_string", "json_string_nocheck"] {
                let f = if name == "json_string" {
                    sym!(l, "json_string", (*const c_char) -> *mut JsonT)
                } else {
                    sym!(l, "json_string_nocheck", (*const c_char) -> *mut JsonT)
                };
                v.push(format!("{}(NULL)={}", name, f(std::ptr::null()).is_null()));
            }
            let stringn = sym!(l, "json_stringn", (*const c_char, usize) -> *mut JsonT);
            let stringn_nc = sym!(l, "json_stringn_nocheck", (*const c_char, usize) -> *mut JsonT);
            v.push(format!("stringn(NULL,0)={}", stringn(std::ptr::null(), 0).is_null()));
            v.push(format!("stringn_nc(NULL,0)={}", stringn_nc(std::ptr::null(), 0).is_null()));
            for bad in [b"\x80".as_ref(), b"\xC0\x80".as_ref(), b"\xED\xA0\x80".as_ref(), b"\xC2".as_ref()] {
                v.push(format!(
                    "stringn(bad)={}",
                    stringn(bad.as_ptr() as *const c_char, bad.len()).is_null()
                ));
            }
            let own = sym!(l, "jsonp_stringn_nocheck_own", (*const c_char, usize) -> *mut JsonT);
            v.push(format!("own(NULL,0)={}", own(std::ptr::null(), 0).is_null()));

            // rows 91, 92
            let sval = sym!(l, "json_string_value", (*const JsonT) -> *const c_char);
            let slen = sym!(l, "json_string_length", (*const JsonT) -> usize);
            v.push(format!("sval(NULL)={} sval(arr)={}", sval(std::ptr::null()).is_null(), sval(arr).is_null()));
            v.push(format!("slen(NULL)={} slen(arr)={}", slen(std::ptr::null()), slen(arr)));

            // rows 93-96
            let s = stringn(b"ab".as_ptr() as *const c_char, 2);
            let sset = sym!(l, "json_string_set", (*mut JsonT, *const c_char) -> c_int);
            let sset_nc = sym!(l, "json_string_set_nocheck", (*mut JsonT, *const c_char) -> c_int);
            let ssetn = sym!(l, "json_string_setn", (*mut JsonT, *const c_char, usize) -> c_int);
            let ssetn_nc = sym!(l, "json_string_setn_nocheck", (*mut JsonT, *const c_char, usize) -> c_int);
            v.push(format!("sset(s,NULL)={}", sset(s, std::ptr::null())));
            v.push(format!("sset_nc(s,NULL)={}", sset_nc(s, std::ptr::null())));
            v.push(format!("ssetn(s,NULL,0)={}", ssetn(s, std::ptr::null(), 0)));
            v.push(format!("ssetn_nc(s,NULL,0)={}", ssetn_nc(s, std::ptr::null(), 0)));
            v.push(format!("ssetn_nc(arr,ab,2)={}", ssetn_nc(arr, b"ab".as_ptr() as *const c_char, 2)));
            v.push(format!("ssetn(arr,ab,2)={}", ssetn(arr, b"ab".as_ptr() as *const c_char, 2)));
            v.push(format!("sset(NULL,ab)={}", sset(std::ptr::null_mut(), b"ab\0".as_ptr() as *const c_char)));
            for bad in [b"\x80".as_ref(), b"\xED\xA0\x80".as_ref()] {
                v.push(format!(
                    "ssetn(s,bad)={}",
                    ssetn(s, bad.as_ptr() as *const c_char, bad.len())
                ));
            }
            decref(l, s);

            // rows 97, 98
            let ival = sym!(l, "json_integer_value", (*const JsonT) -> i64);
            let iset = sym!(l, "json_integer_set", (*mut JsonT, i64) -> c_int);
            v.push(format!("ival(NULL)={} ival(arr)={}", ival(std::ptr::null()), ival(arr)));
            v.push(format!("iset(arr,1)={} iset(NULL,1)={}", iset(arr, 1), iset(std::ptr::null_mut(), 1)));

            // rows 99-102
            let real = sym!(l, "json_real", (c_double) -> *mut JsonT);
            v.push(format!("real(nan)={}", real(f64::NAN).is_null()));
            v.push(format!("real(inf)={}", real(f64::INFINITY).is_null()));
            v.push(format!("real(-inf)={}", real(f64::NEG_INFINITY).is_null()));
            let rval = sym!(l, "json_real_value", (*const JsonT) -> c_double);
            let rset = sym!(l, "json_real_set", (*mut JsonT, c_double) -> c_int);
            v.push(format!("rval(NULL)={} rval(arr)={}", rval(std::ptr::null()).to_bits(), rval(arr).to_bits()));
            let rr = real(1.0);
            v.push(format!(
                "rset(arr)={} rset(NULL)={} rset(r,nan)={} rset(r,inf)={} rset(r,-inf)={}",
                rset(arr, 1.0),
                rset(std::ptr::null_mut(), 1.0),
                rset(rr, f64::NAN),
                rset(rr, f64::INFINITY),
                rset(rr, f64::NEG_INFINITY)
            ));
            decref(l, rr);

            // row 103
            let nval = sym!(l, "json_number_value", (*const JsonT) -> c_double);
            v.push(format!("nval(NULL)={} nval(arr)={}", nval(std::ptr::null()).to_bits(), nval(arr).to_bits()));

            // rows 104-106
            let eq = sym!(l, "json_equal", (*const JsonT, *const JsonT) -> c_int);
            let i1 = integer(1);
            v.push(format!(
                "eq(NULL,i)={} eq(i,NULL)={} eq(NULL,NULL)={} eq(i,arr)={}",
                eq(std::ptr::null(), i1),
                eq(i1, std::ptr::null()),
                eq(std::ptr::null(), std::ptr::null()),
                eq(i1, arr)
            ));
            // row 205: forged out-of-range type byte
            for t in [8i32, 9, 99, -1, i32::MAX, i32::MIN] {
                let f1 = forged(t);
                let f2 = forged(t);
                let p1 = Box::into_raw(f1);
                let p2 = Box::into_raw(f2);
                v.push(format!("type={} eq={} eq_self={}", t, eq(p1, p2), eq(p1, p1)));
                let copy = sym!(l, "json_copy", (*mut JsonT) -> *mut JsonT);
                v.push(format!("type={} copy={}", t, copy(p1).is_null()));
                let deep = sym!(l, "json_deep_copy", (*const JsonT) -> *mut JsonT);
                v.push(format!("type={} deep={}", t, deep(p1).is_null()));
                let dmp = sym!(l, "json_dumps", (*const JsonT, usize) -> *mut c_char);
                let d = dmp(p1, ANY);
                v.push(format!("type={} dumps={}", t, d.is_null()));
                if !d.is_null() {
                    sym!(l, "jsonp_free", (*mut c_void))(d as *mut c_void);
                }
                let del = sym!(l, "json_delete", (*mut JsonT));
                del(p1); // C `default:` arm is a no-op
                v.push(format!("type={} after_delete={}", t, (*p1).typ));
                drop(Box::from_raw(p1));
                drop(Box::from_raw(p2));
            }

            // rows 107-110
            let copy = sym!(l, "json_copy", (*mut JsonT) -> *mut JsonT);
            let deep = sym!(l, "json_deep_copy", (*const JsonT) -> *mut JsonT);
            let do_deep = sym!(l, "do_deep_copy", (*const JsonT, *mut HashTable) -> *mut JsonT);
            v.push(format!("copy(NULL)={} deep(NULL)={}", copy(std::ptr::null_mut()).is_null(), deep(std::ptr::null()).is_null()));
            let mut ht = HashTable::zeroed();
            sym!(l, "hashtable_init", (*mut HashTable) -> c_int)(&mut ht);
            v.push(format!("do_deep(NULL)={}", do_deep(std::ptr::null(), &mut ht).is_null()));
            sym!(l, "hashtable_close", (*mut HashTable))(&mut ht);

            // rows 65, 111, 119: circular references
            let a = sym!(l, "json_array", () -> *mut JsonT)();
            let b = sym!(l, "json_array", () -> *mut JsonT)();
            let app = sym!(l, "json_array_append_new", (*mut JsonT, *mut JsonT) -> c_int);
            app(a, incref(l, b));
            app(b, incref(l, a));
            v.push(format!("cyclic dumps={}", dumps(l, a, ANY).is_none()));
            v.push(format!("cyclic deep_copy={}", deep(a).is_null()));
            let oa = sym!(l, "json_object", () -> *mut JsonT)();
            let ob = sym!(l, "json_object", () -> *mut JsonT)();
            let setk = sym!(l, "json_object_set_new", (*mut JsonT, *const c_char, *mut JsonT) -> c_int);
            let ka = cstr("a");
            setk(oa, ka.as_ptr(), incref(l, ob));
            setk(ob, ka.as_ptr(), incref(l, oa));
            v.push(format!("cyclic obj dumps={}", dumps(l, oa, ANY).is_none()));
            v.push(format!("cyclic obj deep_copy={}", deep(oa).is_null()));
            let upd_rec = sym!(l, "json_object_update_recursive", (*mut JsonT, *mut JsonT) -> c_int);
            let target = sym!(l, "json_object", () -> *mut JsonT)();
            v.push(format!("cyclic update_recursive={}", upd_rec(target, oa)));

            // row 114
            let lc = sym!(l, "jsonp_loop_check", (*mut HashTable, *const JsonT, *mut c_char, usize, *mut usize) -> c_int);
            let mut ht2 = HashTable::zeroed();
            sym!(l, "hashtable_init", (*mut HashTable) -> c_int)(&mut ht2);
            let mut kbuf = [0u8; 19];
            let mut klen = 0usize;
            let r1 = lc(&mut ht2, a, kbuf.as_mut_ptr() as *mut c_char, 19, &mut klen);
            let r2 = lc(&mut ht2, a, kbuf.as_mut_ptr() as *mut c_char, 19, &mut klen);
            v.push(format!("loop_check first={} second={}", r1, r2));
            sym!(l, "hashtable_close", (*mut HashTable))(&mut ht2);

            decref(l, arr);
            decref(l, i1);
            out.push(v);
        }
    }
    assert_eq!(out[0], out[1], "scalar/misc rejections");
}

// ======================= rows 116-130: dump.c ============================

unsafe extern "C" fn failing_cb(_b: *const c_char, _n: usize, data: *mut c_void) -> c_int {
    let st = unsafe { &mut *(data as *mut (usize, usize)) };
    st.0 += 1;
    if st.0 > st.1 { -1 } else { 0 }
}

#[test]
fn err_rows_116_130_dump() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        unsafe {
            let mut v: Vec<String> = Vec::new();
            let arr = sym!(l, "json_array", () -> *mut JsonT)();
            let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
            let app = sym!(l, "json_array_append_new", (*mut JsonT, *mut JsonT) -> c_int);
            app(arr, integer(1));
            app(arr, integer(2));

            // row 116/124: NULL json
            let dmps = sym!(l, "json_dumps", (*const JsonT, usize) -> *mut c_char);
            v.push(format!("dumps(NULL)={}", dmps(std::ptr::null(), ANY).is_null()));
            let dmpb = sym!(l, "json_dumpb", (*const JsonT, *mut c_char, usize, usize) -> usize);
            let mut buf = [0u8; 64];
            v.push(format!("dumpb(NULL)={}", dmpb(std::ptr::null(), buf.as_mut_ptr() as *mut c_char, 64, ANY)));
            // row 207: size 0
            v.push(format!("dumpb(arr,size=0)={}", dmpb(arr, buf.as_mut_ptr() as *mut c_char, 0, ANY)));
            // row 126: size too small
            v.push(format!("dumpb(arr,size=1)={}", dmpb(arr, buf.as_mut_ptr() as *mut c_char, 1, ANY)));
            v.push(format!("dumpb(arr,size=4)={}", dmpb(arr, buf.as_mut_ptr() as *mut c_char, 4, ANY)));

            // row 122: top-level scalar without JSON_ENCODE_ANY
            for txt in ["1", "\"s\"", "true", "null", "1.5"] {
                let (j, _) = loads(l, txt.as_bytes(), JSON_DECODE_ANY);
                v.push(format!(
                    "dumps({},noany)={} dumps(any)={:?}",
                    txt,
                    dumps(l, j, 0).is_none(),
                    dumps(l, j, JSON_ENCODE_ANY)
                ));
                let dcb = sym!(l, "json_dump_callback",
                    (*const JsonT, unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int, *mut c_void, usize) -> c_int);
                let mut st = (0usize, usize::MAX);
                v.push(format!(
                    "dump_callback({},noany)={}",
                    txt,
                    dcb(j, failing_cb, &mut st as *mut (usize, usize) as *mut c_void, 0)
                ));
                decref(l, j);
            }

            // row 121: invalid UTF-8 inside a string value (built via nocheck)
            let bad = sym!(l, "json_stringn_nocheck", (*const c_char, usize) -> *mut JsonT);
            for raw in [b"\x80".as_ref(), b"\xC0\x80".as_ref(), b"\xED\xA0\x80".as_ref(), b"a\xC2".as_ref()] {
                let s = bad(raw.as_ptr() as *const c_char, raw.len());
                v.push(format!("dumps(badutf8 {:02x?})={}", raw, dumps(l, s, JSON_ENCODE_ANY).is_none()));
                v.push(format!("dumps(badutf8 ascii {:02x?})={}", raw, dumps(l, s, JSON_ENCODE_ANY | JSON_ENSURE_ASCII).is_none()));
                decref(l, s);
            }

            // row 120: callback failure at chunk N
            let dcb = sym!(l, "json_dump_callback",
                (*const JsonT, unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int, *mut c_void, usize) -> c_int);
            let (j, _) = loads(l, b"{\"a\":[1,2,{\"b\":\"x\"}],\"c\":1.5}", 0);
            for limit in 0..12usize {
                let mut st = (0usize, limit);
                v.push(format!(
                    "dump_callback fail@{} rc={} calls={}",
                    limit,
                    dcb(j, failing_cb, &mut st as *mut (usize, usize) as *mut c_void, JSON_ENCODE_ANY | json_indent(2)),
                    st.0
                ));
            }
            decref(l, j);

            // row 128: json_dump_file to an unwritable path
            let dfile = sym!(l, "json_dump_file", (*const JsonT, *const c_char, usize) -> c_int);
            for p in ["/proc/definitely/not/writable/x.json", "/", ""] {
                let cp = cstr(p);
                v.push(format!("dump_file({:?})={}", p, dfile(arr, cp.as_ptr(), ANY)));
            }
            // row 129: json_dumpfd with a closed / invalid fd
            let dfd = sym!(l, "json_dumpfd", (*const JsonT, c_int, usize) -> c_int);
            for fd in [-1i32, 100000] {
                v.push(format!("dumpfd({})={}", fd, dfd(arr, fd, ANY)));
            }
            v.push(format!("dumpfd(NULL json)={}", dfd(std::ptr::null(), 1, ANY)));
            // row 127
            let dumpf = sym!(l, "json_dumpf", (*const JsonT, *mut c_void, usize) -> c_int);
            v.push(format!("dumpf(NULL json, NULL FILE) skipped"));
            let _ = dumpf;

            decref(l, arr);
            out.push(v);
        }
    }
    assert_eq!(out[0], out[1], "dump error surface");
}

// ======================= rows 131-165: load.c ============================

#[test]
fn err_rows_131_137_load_null_args() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        unsafe {
            let mut v: Vec<String> = Vec::new();
            let mut e = JsonError::new();
            let render = |e: &JsonError| format!("{:?}", e.raw());

            let loads_f = sym!(l, "json_loads", (*const c_char, usize, *mut JsonError) -> *mut JsonT);
            e = JsonError::new();
            v.push(format!("loads(NULL)={} {}", loads_f(std::ptr::null(), 0, &mut e).is_null(), render(&e)));
            v.push(format!("loads(NULL,no err)={}", loads_f(std::ptr::null(), 0, std::ptr::null_mut()).is_null()));

            let loadb = sym!(l, "json_loadb", (*const c_char, usize, usize, *mut JsonError) -> *mut JsonT);
            e = JsonError::new();
            v.push(format!("loadb(NULL)={} {}", loadb(std::ptr::null(), 0, 0, &mut e).is_null(), render(&e)));
            // row 208: buflen 0
            e = JsonError::new();
            v.push(format!("loadb(\"\",0)={} {}", loadb(b"".as_ptr() as *const c_char, 0, 0, &mut e).is_null(), render(&e)));

            let loadf = sym!(l, "json_loadf", (*mut c_void, usize, *mut JsonError) -> *mut JsonT);
            e = JsonError::new();
            v.push(format!("loadf(NULL)={} {}", loadf(std::ptr::null_mut(), 0, &mut e).is_null(), render(&e)));

            let loadfd = sym!(l, "json_loadfd", (c_int, usize, *mut JsonError) -> *mut JsonT);
            e = JsonError::new();
            v.push(format!("loadfd(-1)={} {}", loadfd(-1, 0, &mut e).is_null(), render(&e)));
            e = JsonError::new();
            v.push(format!("loadfd(-99)={} {}", loadfd(-99, 0, &mut e).is_null(), render(&e)));

            let loadfile = sym!(l, "json_load_file", (*const c_char, usize, *mut JsonError) -> *mut JsonT);
            e = JsonError::new();
            v.push(format!("load_file(NULL)={} {}", loadfile(std::ptr::null(), 0, &mut e).is_null(), render(&e)));

            let loadcb = sym!(l, "json_load_callback",
                (*const c_void, *mut c_void, usize, *mut JsonError) -> *mut JsonT);
            e = JsonError::new();
            v.push(format!("load_callback(NULL)={} {}", loadcb(std::ptr::null(), std::ptr::null_mut(), 0, &mut e).is_null(), render(&e)));
            out.push(v);
        }
    }
    assert_eq!(out[0], out[1], "load NULL args");
}

#[test]
fn err_row_136_load_file_cannot_open() {
    let (c, r) = both();
    for p in [
        "/definitely/does/not/exist/xyz.json",
        "/proc/self/nonexistent",
        "",
        "/",
        "/dev/null/impossible",
    ] {
        let cp = cstr(p);
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "json_load_file", (*const c_char, usize, *mut JsonError) -> *mut JsonT);
            let mut e = JsonError::new();
            let j = unsafe { f(cp.as_ptr(), 0, &mut e) };
            out.push((j.is_null(), e));
        }
        assert_eq!(out[0].0, out[1].0, "load_file({:?}) null-ness", p);
        assert_err_eq(&format!("load_file({:?}) error", p), &out[0].1, &out[1].1);
        assert!(out[0].0, "load_file({:?}) must fail", p);
    }
}

#[test]
fn err_rows_138_165_parse_errors() {
    let (c, r) = both();
    let mut texts: Vec<Vec<u8>> = invalid_corpus().into_iter().map(|s| s.into_bytes()).collect();
    texts.extend(invalid_utf8_corpus());
    // extra targeted triggers, one per remaining ERRORS.md load row
    for extra in [
        "\"\\ud834\\ud834\"",
        "\"\\udfff\"",
        "\"\\u{\"",
        "\"\\u000\"",
        "\"\\U0041\"",
        "\"\\x41\"",
        "{\"a\\u0000b\":1}",
        "{\"\\u0000\":1}",
        "[\"\\u0000\"]",
        "0123",
        "1e+999999999999",
        "-1e+999999999999",
        "18446744073709551616",
        "-18446744073709551617",
        "[1,2,]",
        "{\"a\":1,,}",
        "{:1}",
        "[:]",
        "[}",
        "{]",
        "\t\"unterminated",
        "\"\u{7f}\"",
        "\"\x01\"",
    ] {
        texts.push(extra.as_bytes().to_vec());
    }
    // control characters 0x00..0x1F inside a string literal
    for b in 0u8..0x20 {
        texts.push(vec![b'"', b, b'"']);
        texts.push(vec![b'[', b'"', b, b'"', b']']);
    }
    // every invalid escape character
    for b in 0x20u8..0x7f {
        if !b"\"\\/bfnrtu".contains(&b) {
            texts.push(vec![b'"', b'\\', b, b'"']);
        }
    }
    // non-hex in \u escapes
    for b in 0x20u8..0x7f {
        if !b.is_ascii_hexdigit() {
            texts.push(format!("\"\\u00{}0\"", b as char).into_bytes());
        }
    }

    for dflags in [0usize, JSON_DECODE_ANY, 0x1F, JSON_ALLOW_NUL, JSON_REJECT_DUPLICATES] {
        for t in &texts {
            let mut out = Vec::new();
            for l in [c, r] {
                let (j, e) = unsafe { loads(l, t, dflags) };
                let d = if j.is_null() {
                    None
                } else {
                    let d = unsafe { dumps(l, j, ANY) };
                    unsafe { decref(l, j) };
                    d
                };
                out.push((d, e));
            }
            assert_bytes_eq(
                &format!("parse error value {:02x?} dflags={:#x}", t, dflags),
                &out[0].0,
                &out[1].0,
            );
            assert_err_eq(
                &format!("parse error struct {:02x?} dflags={:#x}", t, dflags),
                &out[0].1,
                &out[1].1,
            );
        }
    }
}

#[test]
fn err_row_158_stack_overflow() {
    let (c, r) = both();
    for d in [2047usize, 2048, 2049, 2050, 4096, 20000] {
        for open in ["[", "{\"a\":"] {
            let close = if open == "[" { "]" } else { "}" };
            let t = format!("{}1{}", open.repeat(d), close.repeat(d));
            let mut out = Vec::new();
            for l in [c, r] {
                let (j, e) = unsafe { loads(l, t.as_bytes(), 0) };
                let ok = !j.is_null();
                if ok {
                    unsafe { decref(l, j) };
                }
                out.push((ok, e));
            }
            assert_eq!(out[0].0, out[1].0, "depth {} success differs", d);
            assert_err_eq(&format!("depth {} error", d), &out[0].1, &out[1].1);
        }
    }
}

// ======================= rows 166-203: pack_unpack.c =====================

type PackEx = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut JsonT;
type UnpackEx =
    unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char, ...) -> c_int;

fn pack_err<F>(name: &str, flags: usize, body: F)
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
            let s: libloading::Symbol<PackEx> = l.lib.get(b"json_pack_ex\0").unwrap();
            let j = body(l, *s, &mut e, flags);
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
    assert_bytes_eq(&format!("pack err {}: value", name), &out[0].0, &out[1].0);
    assert_err_eq(&format!("pack err {}: error", name), &out[0].1, &out[1].1);
}

#[test]
fn err_rows_166_178_pack() {
    // rows 166, 167
    pack_err("fmt NULL", 0, |_, f, e, fl| unsafe {
        f(e, fl, std::ptr::null())
    });
    pack_err("fmt empty", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("").unwrap();
        f(e, fl, cf.as_ptr())
    });
    // row 168
    for fmt in ["[]]", "{}}", "n n", "[]x", "{} ", "[] [1]", "nn"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            f(e, fl, cf.as_ptr())
        });
    }
    // row 169: unknown format characters
    for ch in b"xyzXYZFqQ0123456789.-_=<>()".iter() {
        let fmt = (*ch as char).to_string();
        let name = fmt.clone();
        pack_err(&name, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt.clone()).unwrap();
            f(e, fl, cf.as_ptr())
        });
    }
    // rows 170, 173
    for fmt in ["{", "[", "{s", "{s:", "[i", "{s:i", "{s:i,", "[i,"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let k = CString::new("k").unwrap();
            f(e, fl, cf.as_ptr(), k.as_ptr(), 1 as c_int)
        });
    }
    // row 171: non-'s' key format
    for fmt in ["{i:i}", "{n:i}", "{b:i}", "{[]:i}", "{{}:i}", "{f:i}", "{o:i}"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            f(e, fl, cf.as_ptr(), 1 as c_int, 2 as c_int)
        });
    }
    // row 172: NULL object value without '*'
    pack_err("{s:s} NULL value", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:s}").unwrap();
        let k = CString::new("k").unwrap();
        f(e, fl, cf.as_ptr(), k.as_ptr(), std::ptr::null::<c_char>())
    });
    pack_err("{s:o} NULL value", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("{s:o}").unwrap();
        let k = CString::new("k").unwrap();
        f(e, fl, cf.as_ptr(), k.as_ptr(), std::ptr::null::<JsonT>())
    });
    // row 174: NULL string arg
    pack_err("s NULL", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("s").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>())
    });
    pack_err("[s] NULL", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("[s]").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>())
    });
    pack_err("s+ NULL first", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("s+").unwrap();
        let b = CString::new("x").unwrap();
        f(e, fl, cf.as_ptr(), std::ptr::null::<c_char>(), b.as_ptr())
    });
    pack_err("s+ NULL second", 0, |_, f, e, fl| unsafe {
        let cf = CString::new("s+").unwrap();
        let a = CString::new("x").unwrap();
        f(e, fl, cf.as_ptr(), a.as_ptr(), std::ptr::null::<c_char>())
    });
    // row 175: invalid UTF-8
    for bad in [b"\x80".as_ref(), b"\xC0\x80".as_ref(), b"\xED\xA0\x80".as_ref(), b"a\xC2".as_ref()] {
        let owned = bad.to_vec();
        pack_err("s bad utf8", 0, move |_, f, e, fl| unsafe {
            let cf = CString::new("s").unwrap();
            let cs = CString::new(owned.clone()).unwrap();
            f(e, fl, cf.as_ptr(), cs.as_ptr())
        });
        let owned2 = bad.to_vec();
        pack_err("s+ bad utf8", 0, move |_, f, e, fl| unsafe {
            let cf = CString::new("s+").unwrap();
            let a = CString::new("ok").unwrap();
            let cs = CString::new(owned2.clone()).unwrap();
            f(e, fl, cf.as_ptr(), a.as_ptr(), cs.as_ptr())
        });
    }
    // row 176: '#', '%', '+' with optional.  The va_list shape depends on which
    // modifier the C code reaches, so each format gets arguments that are valid
    // under every interpretation (a real length, never a truncated pointer).
    for fmt in ["s?#", "s#?"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let a = CString::new("abc").unwrap();
            f(e, fl, cf.as_ptr(), a.as_ptr(), 2 as c_int)
        });
    }
    for fmt in ["s?%", "s%?"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let a = CString::new("abc").unwrap();
            f(e, fl, cf.as_ptr(), a.as_ptr(), 2usize)
        });
    }
    for fmt in ["s?+", "s+?"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let a = CString::new("abc").unwrap();
            let b = CString::new("def").unwrap();
            f(e, fl, cf.as_ptr(), a.as_ptr(), b.as_ptr())
        });
    }
    for fmt in ["s*#", "s*%", "s*+"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let a = CString::new("abc").unwrap();
            f(e, fl, cf.as_ptr(), a.as_ptr(), 2usize)
        });
    }
    // row 177: NULL json for o / O without ?/*
    for fmt in ["o", "O", "[o]", "[O]"] {
        pack_err(fmt, 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            f(e, fl, cf.as_ptr(), std::ptr::null::<JsonT>())
        });
    }
    // row 178: non-finite real
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        pack_err("f nonfinite", 0, move |_, f, e, fl| unsafe {
            let cf = CString::new("f").unwrap();
            f(e, fl, cf.as_ptr(), v)
        });
        pack_err("[f] nonfinite", 0, move |_, f, e, fl| unsafe {
            let cf = CString::new("[f]").unwrap();
            f(e, fl, cf.as_ptr(), v)
        });
    }
}

fn unpack_err<B, F>(name: &str, flags: usize, build_root: B, body: F)
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
            let s: libloading::Symbol<UnpackEx> = l.lib.get(b"json_unpack_ex\0").unwrap();
            let rendered = body(l, *s, root, &mut e, flags);
            if !root.is_null() {
                decref(l, root);
            }
            out.push((rendered, e));
        }
    }
    assert_eq!(out[0].0, out[1].0, "unpack err {}: output", name);
    assert_err_eq(&format!("unpack err {}: error", name), &out[0].1, &out[1].1);
}

fn root_of(text: &'static str) -> impl Fn(&Lib) -> *mut JsonT {
    move |l: &Lib| unsafe {
        let (j, _) = loads(l, text.as_bytes(), JSON_DECODE_ANY);
        j
    }
}

#[test]
fn err_rows_179_203_unpack() {
    // row 179
    unpack_err("root NULL", 0, |_| std::ptr::null_mut(), |_, f, root, e, fl| unsafe {
        let cf = CString::new("{}").unwrap();
        format!("rc={}", f(root, e, fl, cf.as_ptr()))
    });
    // row 180
    unpack_err("fmt NULL", 0, root_of("{}"), |_, f, root, e, fl| unsafe {
        format!("rc={}", f(root, e, fl, std::ptr::null()))
    });
    unpack_err("fmt empty", 0, root_of("{}"), |_, f, root, e, fl| unsafe {
        let cf = CString::new("").unwrap();
        format!("rc={}", f(root, e, fl, cf.as_ptr()))
    });
    // row 181
    for fmt in ["{}}", "[]]", "nn", "[] x", "{} {}"] {
        unpack_err(fmt, 0, root_of("{}"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            format!("rc={}", f(root, e, fl, cf.as_ptr()))
        });
    }
    // rows 182, 189, 194, 197-201: wrong types
    for (fmt, root) in [
        ("{}", "[]"),
        ("{}", "1"),
        ("{}", "\"s\""),
        ("{}", "null"),
        ("{}", "true"),
        ("{}", "1.5"),
        ("[]", "{}"),
        ("[]", "1"),
        ("[]", "\"s\""),
        ("s", "1"),
        ("s", "{}"),
        ("s", "null"),
        ("i", "\"s\""),
        ("i", "1.5"),
        ("I", "1.5"),
        ("b", "1"),
        ("b", "null"),
        ("f", "1"),
        ("f", "\"s\""),
        ("F", "\"s\""),
        ("F", "null"),
        ("n", "1"),
        ("n", "false"),
    ] {
        unpack_err(
            &format!("{} on {}", fmt, root),
            0,
            root_of(root),
            move |_, f, r0, e, fl| unsafe {
                let cf = CString::new(fmt).unwrap();
                let mut a: c_longlong = -1;
                let mut p: *const c_char = std::ptr::null();
                let rc = if fmt == "s" {
                    f(r0, e, fl, cf.as_ptr(), &mut p)
                } else if fmt == "n" || fmt == "{}" || fmt == "[]" {
                    f(r0, e, fl, cf.as_ptr())
                } else {
                    f(r0, e, fl, cf.as_ptr(), &mut a)
                };
                format!("rc={}", rc)
            },
        );
    }
    // row 183: non-'s' key format
    for fmt in ["{i:i}", "{n:i}", "{[]:i}", "{f:i}"] {
        unpack_err(fmt, 0, root_of("{\"a\":1}"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let mut a: c_int = -1;
            format!("rc={}", f(root, e, fl, cf.as_ptr(), &mut a))
        });
    }
    // row 184
    for fmt in ["{", "{s", "{s:", "[", "[i"] {
        unpack_err(fmt, 0, root_of("{\"a\":1}"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let k = CString::new("a").unwrap();
            let mut a: c_int = -1;
            format!("rc={}", f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut a))
        });
    }
    // row 185: NULL key
    unpack_err("{s:i} NULL key", 0, root_of("{\"a\":1}"), |_, f, root, e, fl| unsafe {
        let cf = CString::new("{s:i}").unwrap();
        let mut a: c_int = -1;
        format!("rc={}", f(root, e, fl, cf.as_ptr(), std::ptr::null::<c_char>(), &mut a))
    });
    // row 186: item not found
    for key in ["missing", "", "A"] {
        unpack_err(
            &format!("{{s:i}} missing {}", key),
            0,
            root_of("{\"a\":1}"),
            move |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:i}").unwrap();
                let k = CString::new(key).unwrap();
                let mut a: c_int = -1;
                format!("rc={}", f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut a))
            },
        );
    }
    // row 187: strict leftover object keys
    for (root, fmt) in [
        ("{\"a\":1,\"b\":2}", "{s:i!}"),
        ("{\"a\":1,\"b\":2,\"c\":3}", "{s:i!}"),
        ("{\"a\":1,\"b\":2}", "{s:i}"),
    ] {
        unpack_err(
            &format!("strict {} {}", root, fmt),
            JSON_STRICT,
            root_of(root),
            move |_, f, r0, e, fl| unsafe {
                let cf = CString::new(fmt).unwrap();
                let k = CString::new("a").unwrap();
                let mut a: c_int = -1;
                format!("rc={}", f(r0, e, fl, cf.as_ptr(), k.as_ptr(), &mut a))
            },
        );
    }
    // row 188: chars after '!' / '*'
    for fmt in ["{s:i!s}", "{s:i*s}", "{s:i!i}", "{!s:i}", "{*s:i}"] {
        unpack_err(fmt, 0, root_of("{\"a\":1}"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let k = CString::new("a").unwrap();
            let k2 = CString::new("b").unwrap();
            let mut a: c_int = -1;
            let mut b: c_int = -1;
            format!("rc={}", f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut a, k2.as_ptr(), &mut b))
        });
    }
    // row 190: array index out of range
    for (root, fmt) in [("[]", "[i]"), ("[1]", "[i,i]"), ("[1,2]", "[i,i,i]")] {
        unpack_err(
            &format!("range {} {}", root, fmt),
            0,
            root_of(root),
            move |_, f, r0, e, fl| unsafe {
                let cf = CString::new(fmt).unwrap();
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                let mut d: c_int = -1;
                format!("rc={}", f(r0, e, fl, cf.as_ptr(), &mut a, &mut b, &mut d))
            },
        );
    }
    // row 191: strict leftover array items
    for (root, fmt) in [("[1,2]", "[i!]"), ("[1,2,3]", "[i,i!]"), ("[1,2]", "[i]")] {
        unpack_err(
            &format!("strict arr {} {}", root, fmt),
            JSON_STRICT,
            root_of(root),
            move |_, f, r0, e, fl| unsafe {
                let cf = CString::new(fmt).unwrap();
                let mut a: c_int = -1;
                let mut b: c_int = -1;
                format!("rc={}", f(r0, e, fl, cf.as_ptr(), &mut a, &mut b))
            },
        );
    }
    // row 192: chars after '!' / '*' in array
    for fmt in ["[i!i]", "[i*i]", "[!i]", "[*i]"] {
        unpack_err(fmt, 0, root_of("[1,2]"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let mut a: c_int = -1;
            let mut b: c_int = -1;
            format!("rc={}", f(root, e, fl, cf.as_ptr(), &mut a, &mut b))
        });
    }
    // rows 193, 202: unknown format characters
    for ch in b"xyzXYZqQ0123456789.-_=<>()".iter() {
        let fmt = (*ch as char).to_string();
        let name = fmt.clone();
        unpack_err(&name, 0, root_of("{}"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt.clone()).unwrap();
            let mut a: c_longlong = 0;
            format!("rc={}", f(root, e, fl, cf.as_ptr(), &mut a))
        });
        let fmt2 = format!("[{}]", *ch as char);
        let name2 = fmt2.clone();
        unpack_err(&name2, 0, root_of("[1]"), move |_, f, root, e, fl| unsafe {
            let cf = CString::new(fmt2.clone()).unwrap();
            let mut a: c_longlong = 0;
            format!("rc={}", f(root, e, fl, cf.as_ptr(), &mut a))
        });
    }
    // row 195: NULL string target
    for flags in [0usize, JSON_VALIDATE_ONLY] {
        unpack_err("s NULL target", flags, root_of("\"abc\""), |_, f, root, e, fl| unsafe {
            let cf = CString::new("s").unwrap();
            format!("rc={}", f(root, e, fl, cf.as_ptr(), std::ptr::null_mut::<*const c_char>()))
        });
        // row 196: NULL length target
        unpack_err("s% NULL len", flags, root_of("\"abc\""), |_, f, root, e, fl| unsafe {
            let cf = CString::new("s%").unwrap();
            let mut p: *const c_char = std::ptr::null();
            format!("rc={}", f(root, e, fl, cf.as_ptr(), &mut p, std::ptr::null_mut::<usize>()))
        });
    }
}

// ================== rows 204-209: generic FFI boundary ===================

/// Error messages longer than `JSON_ERROR_TEXT_LENGTH - 2` must truncate
/// identically, and pack/unpack messages must keep raw bytes (C formats
/// straight into `error->text` with `vsnprintf`, so an embedded NUL from
/// `'%c'` at end-of-format does not cut the message short).
#[test]
fn err_long_and_raw_error_texts() {
    // "Object item not found: %s" with keys of every length around the limit
    for klen in [1usize, 50, 130, 135, 136, 137, 138, 158, 159, 160, 300, 1000] {
        let key: String = "K".repeat(klen);
        unpack_err(
            &format!("missing key len {}", klen),
            0,
            root_of("{\"a\":1}"),
            move |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:i}").unwrap();
                let k = CString::new(key.clone()).unwrap();
                let mut a: c_int = -1;
                format!("rc={}", f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut a))
            },
        );
    }
    // "%li object item(s) left unpacked: %s" with many long keys
    for n in [1usize, 3, 10, 40] {
        let pairs: Vec<String> = (0..n)
            .map(|i| format!("\"{}{:03}\":{}", "L".repeat(20), i, i))
            .collect();
        let text: &'static str = Box::leak(format!("{{{}}}", pairs.join(",")).into_boxed_str());
        unpack_err(
            &format!("strict leftover {} keys", n),
            JSON_STRICT,
            move |l| unsafe {
                let (j, _) = loads(l, text.as_bytes(), 0);
                j
            },
            |_, f, root, e, fl| unsafe {
                let cf = CString::new("{}").unwrap();
                format!("rc={}", f(root, e, fl, cf.as_ptr()))
            },
        );
    }
    // "Invalid UTF-8 %s" / "NULL %s" purposes, plus long source strings
    for src_len in [1usize, 78, 79, 80, 81, 200] {
        let long_key: String = "S".repeat(src_len);
        unpack_err(
            &format!("source len {}", src_len),
            0,
            root_of("{\"a\":1}"),
            move |_, f, root, e, fl| unsafe {
                let cf = CString::new("{s:s}").unwrap();
                let k = CString::new(long_key.clone()).unwrap();
                let mut p: *const c_char = std::ptr::null();
                format!("rc={}", f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut p))
            },
        );
    }
    // Truncated formats: '%c' receives the NUL terminator of the format string.
    for fmt in ["{s:", "{s", "[", "{", "[i,", "{s:i,", "{s:i!", "[i!", "{!", "[*"] {
        pack_err(&format!("truncated {:?}", fmt), 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt).unwrap();
            let k = CString::new("k").unwrap();
            f(e, fl, cf.as_ptr(), k.as_ptr(), 1 as c_int)
        });
        unpack_err(
            &format!("truncated unpack {:?}", fmt),
            0,
            root_of("{\"a\":1}"),
            move |_, f, root, e, fl| unsafe {
                let cf = CString::new(fmt).unwrap();
                let k = CString::new("a").unwrap();
                let mut a: c_int = -1;
                format!("rc={}", f(root, e, fl, cf.as_ptr(), k.as_ptr(), &mut a))
            },
        );
    }
    // Long format strings drive jsonp_error_set_source's ">= 80" branch.
    for pad in [70usize, 78, 79, 80, 81, 100, 300] {
        let fmt: String = format!("{}x", " ".repeat(pad));
        pack_err(&format!("long fmt {}", pad), 0, move |_, f, e, fl| unsafe {
            let cf = CString::new(fmt.clone()).unwrap();
            f(e, fl, cf.as_ptr())
        });
    }
}

#[test]
fn err_rows_204_209_generic_boundary() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        unsafe {
            let mut v: Vec<String> = Vec::new();
            // row 206: unknown high flag bits are ignored
            for extra in [
                0usize,
                1 << 20,
                1 << 40,
                usize::MAX & !0x1F,
                usize::MAX,
            ] {
                let (j, e) = loads(l, b"[1,{\"a\":\"x\"}]", extra & !0x1F);
                v.push(format!(
                    "loads(extra={:#x})={:?} err={:?}",
                    extra,
                    if j.is_null() { None } else { dumps(l, j, ANY) },
                    e.raw()
                ));
                if !j.is_null() {
                    v.push(format!("dumps(extra)={:?}", dumps(l, j, extra & !0x1_07FF | JSON_ENCODE_ANY)));
                    decref(l, j);
                }
            }
            // row 209: jansson_version_cmp extremes
            let cmp = sym!(l, "jansson_version_cmp", (c_int, c_int, c_int) -> c_int);
            for a in [i32::MIN, i32::MIN + 1, -1, 0, 2, i32::MAX] {
                for b in [i32::MIN, -1, 15, i32::MAX] {
                    for m in [i32::MIN, -1, 0, i32::MAX] {
                        v.push(format!("cmp({},{},{})={}", a, b, m, cmp(a, b, m)));
                    }
                }
            }
            out.push(v);
        }
    }
    assert_eq!(out[0], out[1], "generic FFI boundary");
}
