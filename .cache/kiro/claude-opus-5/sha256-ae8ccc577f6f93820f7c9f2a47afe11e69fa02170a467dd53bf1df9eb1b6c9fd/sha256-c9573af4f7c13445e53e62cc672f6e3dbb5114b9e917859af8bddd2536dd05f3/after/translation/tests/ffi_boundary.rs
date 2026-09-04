//! Phase C — ERRORS.md rows 252–259: the generic FFI-boundary inputs every C
//! API has, whether or not the source spells them out.
//!
//!  * NULL pointers into every public entry point
//!  * out-of-range `json_type` values crossing the boundary (a C enum accepts
//!    any int, so 8 / 255 / -1 are real inputs)
//!  * out-of-range `enum json_error_code` bytes
//!  * zero and oversized lengths
//!  * values one step past every documented range (indent, precision, flag bits)
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

/* ============ row 252: NULL everywhere ============ */

#[test]
fn row_252_null_pointers_into_every_entry_point() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        let n: Jt = std::ptr::null_mut();
        let nc: *const c_char = std::ptr::null();
        unsafe {
            out.push(format!(
                "obj: size={} get={} getn={} set={} setn={} set_nc={} setn_nc={} del={} deln={} clear={}",
                (api.json_object_size)(n),
                (api.json_object_get)(n, nc).is_null(),
                (api.json_object_getn)(n, nc, 0).is_null(),
                (api.json_object_set_new)(n, nc, n),
                (api.json_object_setn_new)(n, nc, 0, n),
                (api.json_object_set_new_nocheck)(n, nc, n),
                (api.json_object_setn_new_nocheck)(n, nc, 0, n),
                (api.json_object_del)(n, nc),
                (api.json_object_deln)(n, nc, 0),
                (api.json_object_clear)(n)
            ));
            out.push(format!(
                "obj2: upd={} upde={} updm={} updr={} iter={} iter_at={} k2i={} inext={} ikey={} iklen={} ival={} iset={}",
                (api.json_object_update)(n, n),
                (api.json_object_update_existing)(n, n),
                (api.json_object_update_missing)(n, n),
                (api.json_object_update_recursive)(n, n),
                (api.json_object_iter)(n).is_null(),
                (api.json_object_iter_at)(n, nc).is_null(),
                (api.json_object_key_to_iter)(nc).is_null(),
                (api.json_object_iter_next)(n, std::ptr::null_mut()).is_null(),
                (api.json_object_iter_key)(std::ptr::null_mut()).is_null(),
                (api.json_object_iter_key_len)(std::ptr::null_mut()),
                (api.json_object_iter_value)(std::ptr::null_mut()).is_null(),
                (api.json_object_iter_set_new)(n, std::ptr::null_mut(), n)
            ));
            out.push(format!(
                "arr: size={} get={} set={} app={} ins={} rem={} clr={} ext={}",
                (api.json_array_size)(n),
                (api.json_array_get)(n, 0).is_null(),
                (api.json_array_set_new)(n, 0, n),
                (api.json_array_append_new)(n, n),
                (api.json_array_insert_new)(n, 0, n),
                (api.json_array_remove)(n, 0),
                (api.json_array_clear)(n),
                (api.json_array_extend)(n, n)
            ));
            out.push(format!(
                "scal: str={} strn={} str_nc={} strn_nc={} own={} sval={} slen={} ival={} rval={:?} nval={:?}",
                (api.json_string)(nc).is_null(),
                (api.json_stringn)(nc, 0).is_null(),
                (api.json_string_nocheck)(nc).is_null(),
                (api.json_stringn_nocheck)(nc, 0).is_null(),
                (api.jsonp_stringn_nocheck_own)(nc, 0).is_null(),
                (api.json_string_value)(n).is_null(),
                (api.json_string_length)(n),
                (api.json_integer_value)(n),
                (api.json_real_value)(n).to_bits(),
                (api.json_number_value)(n).to_bits()
            ));
            out.push(format!(
                "set: sset={} ssetn={} sset_nc={} ssetn_nc={} iset={} rset={}",
                (api.json_string_set)(n, nc),
                (api.json_string_setn)(n, nc, 0),
                (api.json_string_set_nocheck)(n, nc),
                (api.json_string_setn_nocheck)(n, nc, 0),
                (api.json_integer_set)(n, 0),
                (api.json_real_set)(n, 0.0)
            ));
            out.push(format!(
                "eq/copy: eq={} copy={} deep={}",
                (api.json_equal)(n, n),
                (api.json_copy)(n).is_null(),
                (api.json_deep_copy)(n).is_null()
            ));
            (api.json_delete)(n);
            out.push("delete(NULL) ok".into());
            out.push(format!(
                "dump: dumps={:?} dumpb={} dumpf={} dumpfd={} dumpfile={} dumpcb={}",
                dumps(api, n, JSON_ENCODE_ANY),
                (api.json_dumpb)(n, std::ptr::null_mut(), 0, JSON_ENCODE_ANY),
                (api.json_dumpf)(n, std::ptr::null_mut(), JSON_ENCODE_ANY),
                (api.json_dumpfd)(n, -1, JSON_ENCODE_ANY),
                (api.json_dump_file)(n, nc, JSON_ENCODE_ANY),
                (api.json_dump_callback)(n, None, std::ptr::null_mut(), JSON_ENCODE_ANY)
            ));
            let mut e = JsonError::zeroed();
            out.push(format!(
                "load: loads={} loadb={} loadf={} loadfd={} loadfile={} loadcb={}",
                (api.json_loads)(nc, 0, &mut e).is_null(),
                (api.json_loadb)(nc, 0, 0, std::ptr::null_mut()).is_null(),
                (api.json_loadf)(std::ptr::null_mut(), 0, std::ptr::null_mut()).is_null(),
                (api.json_loadfd)(-1, 0, std::ptr::null_mut()).is_null(),
                (api.json_load_file)(nc, 0, std::ptr::null_mut()).is_null(),
                (api.json_load_callback)(None, std::ptr::null_mut(), 0, std::ptr::null_mut())
                    .is_null()
            ));
            out.push(format!(
                "pack: pack={} pack_ex={} unpack={} unpack_ex={} sprintf={}",
                (api.json_pack)(nc).is_null(),
                (api.json_pack_ex)(std::ptr::null_mut(), 0usize, nc).is_null(),
                (api.json_unpack)(n, nc),
                (api.json_unpack_ex)(n, std::ptr::null_mut(), 0usize, nc),
                (api.json_sprintf)(cstr("x").as_ptr()).is_null()
            ));
            // json_sprintf leaks a value above; take it back
            let leaked = (api.json_sprintf)(cstr("x").as_ptr());
            decref(api, leaked);
            out.push(format!(
                "priv: malloc0={} strndup_ok={} loop_check_ready",
                (api.jsonp_malloc)(0).is_null(),
                {
                    let d = (api.jsonp_strndup)(cstr("").as_ptr(), 0);
                    let ok = !d.is_null();
                    (api.jsonp_free)(d as *mut c_void);
                    ok
                }
            ));
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "NULL-pointer step {i}");
    }
}

/* ============ row 253: out-of-range json_type across the boundary ======== */

#[test]
fn row_253_out_of_range_json_type() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let types: Vec<c_int> = vec![
                -1,
                -2,
                8,
                9,
                10,
                100,
                127,
                128,
                200,
                255,
                256,
                65536,
                i32::MAX,
                i32::MIN,
            ];
            for ty in types {
                let f = forge(api, ty);
                let g = forge(api, ty);
                out.push(format!(
                    "ty={ty}: eq_self={} eq_other={} copy={} deep={} dumps_any={:?} dump_cb={} objsize={} arrsize={} sval={} slen={} ival={} rval={:?} nval={:?}",
                    (api.json_equal)(f, f),
                    (api.json_equal)(f, g),
                    (api.json_copy)(f).is_null(),
                    (api.json_deep_copy)(f).is_null(),
                    dumps(api, f, JSON_ENCODE_ANY),
                    (api.json_dump_callback)(f, None, std::ptr::null_mut(), JSON_ENCODE_ANY),
                    (api.json_object_size)(f),
                    (api.json_array_size)(f),
                    (api.json_string_value)(f).is_null(),
                    (api.json_string_length)(f),
                    (api.json_integer_value)(f),
                    (api.json_real_value)(f).to_bits(),
                    (api.json_number_value)(f).to_bits()
                ));
                // json_delete must take the `default:` branch and not free
                (api.json_delete)(f);
                out.push(format!("ty={ty} after delete type={}", (*f).type_));
                // NOTE: a forged type must NOT be fed to json_unpack_ex.  The C
                // renders wrong-type errors with `type_name(root)` =
                // `type_names[json_typeof(root)]`, an 8-entry table indexed by
                // the raw type tag, so an out-of-range tag is an out-of-bounds
                // read in the C itself (genuine UB, not a defined input).
                // packed with 'o' then dumped
                let mut e = JsonError::zeroed();
                let packed = (api.json_pack_ex)(&mut e, 0usize, cstr("[o]").as_ptr(), f);
                out.push(format!(
                    "ty={ty} pack_o null={} dump={:?} code={}",
                    packed.is_null(),
                    dumps(api, packed, 0),
                    e.code()
                ));
                if !packed.is_null() {
                    (api.json_array_remove)(packed, 0);
                    decref(api, packed);
                }
                (api.jsonp_free)(f as *mut c_void);
                (api.jsonp_free)(g as *mut c_void);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "out-of-range type step {i}");
    }
}

/* ============ row 254: out-of-range enum json_error_code ============ */

#[test]
fn row_254_out_of_range_error_code() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<(u8, Vec<u8>)> {
        let mut out = Vec::new();
        unsafe {
            for code in [
                -256i32, -1, 0, 1, 17, 18, 19, 100, 126, 127, 128, 129, 200, 254, 255,
                256, 257, 512, i32::MAX, i32::MIN,
            ] {
                let mut e = JsonError::zeroed();
                (api.jsonp_error_init)(&mut e, cstr("src").as_ptr());
                (api.jsonp_error_set)(
                    &mut e,
                    1,
                    2,
                    3usize,
                    code,
                    cstr("message").as_ptr(),
                );
                out.push((e.code(), e.text.iter().map(|&c| c as u8).collect()));
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ============ row 255: zero lengths ============ */

#[test]
fn row_255_zero_lengths() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let empty = cstr("");
            // json_stringn(p, 0) is a valid empty string
            let s = (api.json_stringn)(empty.as_ptr(), 0);
            out.push(format!(
                "stringn0 null={} len={} dump={:?}",
                s.is_null(),
                (api.json_string_length)(s),
                dumps(api, s, JSON_ENCODE_ANY)
            ));
            decref(api, s);
            let s = (api.json_stringn_nocheck)(empty.as_ptr(), 0);
            out.push(format!("stringn_nc0 null={}", s.is_null()));
            decref(api, s);
            // json_loadb with buflen 0
            let mut e = JsonError::zeroed();
            let j = (api.json_loadb)(empty.as_ptr(), 0, 0, &mut e);
            out.push(format!("loadb0 null={} code={} text={:?}", j.is_null(), e.code(), e.text_str()));
            decref(api, j);
            // json_dumpb with size 0 and NULL buffer
            let arr = (api.json_loads)(cstr("[1,2,3]").as_ptr(), 0, std::ptr::null_mut());
            out.push(format!(
                "dumpb(NULL,0)={} dumpb(NULL,0,indent)={}",
                (api.json_dumpb)(arr, std::ptr::null_mut(), 0, 0),
                (api.json_dumpb)(arr, std::ptr::null_mut(), 0, json_indent(4))
            ));
            decref(api, arr);
            // strbuffer_append_bytes with size 0
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            out.push(format!(
                "append0={} len={}",
                (api.strbuffer_append_bytes)(&mut sb, empty.as_ptr(), 0),
                sb.length
            ));
            (api.strbuffer_close)(&mut sb);
            // key of length 0
            let o = (api.json_object)();
            out.push(format!(
                "setn0={} size={} dump={:?}",
                (api.json_object_setn_new)(o, empty.as_ptr(), 0, (api.json_integer)(1)),
                (api.json_object_size)(o),
                dumps(api, o, 0)
            ));
            out.push(format!(
                "getn0={} deln0={}",
                (api.json_object_getn)(o, empty.as_ptr(), 0).is_null(),
                (api.json_object_deln)(o, empty.as_ptr(), 0)
            ));
            decref(api, o);
            // utf8 helpers with length 0
            out.push(format!(
                "check_string0={} check_full0={} iterate0_is_input={}",
                (api.utf8_check_string)(empty.as_ptr(), 0),
                (api.utf8_check_full)(empty.as_ptr(), 0, std::ptr::null_mut()),
                (api.utf8_iterate)(empty.as_ptr(), 0, std::ptr::null_mut()) == empty.as_ptr()
            ));
            // jsonp_strndup(len 0)
            let d = (api.jsonp_strndup)(empty.as_ptr(), 0);
            out.push(format!("strndup0 null={} first={}", d.is_null(), *(d as *const u8)));
            (api.jsonp_free)(d as *mut c_void);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ============ row 256: oversized lengths (as far as is safe) ============ */

#[test]
fn row_256_oversized_lengths() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            // The C reads `len` bytes before any guard in json_stringn /
            // hashtable_set, so SIZE_MAX there is genuine UB and not a valid
            // input.  What IS safely observable is a length that overruns a
            // short buffer by a bounded amount, and the guards that fire before
            // any read (strbuffer_append_bytes).
            let src = nul_terminated(b"abcdefgh");
            for len in [8usize, 9, 16] {
                let s = (api.json_stringn_nocheck)(src.as_ptr(), len);
                out.push(format!(
                    "stringn_nc len={len} null={} slen={}",
                    s.is_null(),
                    (api.json_string_length)(s)
                ));
                decref(api, s);
            }
            // strbuffer guards, no data read
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            out.push(format!(
                "append(SIZE_MAX)={} append(SIZE_MAX-1)={}",
                (api.strbuffer_append_bytes)(&mut sb, std::ptr::null(), usize::MAX),
                (api.strbuffer_append_bytes)(&mut sb, std::ptr::null(), usize::MAX - 1)
            ));
            (api.strbuffer_close)(&mut sb);
            // array indices at the very top of the range
            let arr = (api.json_array)();
            for i in 0..3 {
                (api.json_array_append_new)(arr, (api.json_integer)(i));
            }
            for idx in [usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 62] {
                out.push(format!(
                    "idx={idx} get={} set={} insert={} remove={}",
                    (api.json_array_get)(arr, idx).is_null(),
                    (api.json_array_set_new)(arr, idx, (api.json_integer)(1)),
                    (api.json_array_insert_new)(arr, idx, (api.json_integer)(1)),
                    (api.json_array_remove)(arr, idx)
                ));
            }
            out.push(format!("arr still {:?}", dumps(api, arr, 0)));
            decref(api, arr);
            // jsonp_dtostr with a huge `size` argument (buffer is big enough for
            // any real, so the size check simply passes)
            let mut buf = [0i8; 64];
            out.push(format!(
                "dtostr(size=SIZE_MAX)={} dtostr(size=usize::MAX/2)={}",
                (api.jsonp_dtostr)(buf.as_mut_ptr(), usize::MAX, 1.5, 0),
                (api.jsonp_dtostr)(buf.as_mut_ptr(), usize::MAX / 2, 1.5, 0)
            ));
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ============ row 257: one step past every documented range ============ */

#[test]
fn row_257_flag_ranges() {
    let _g = lock();
    let p = pair();
    let docs = [
        "[1,2,3]",
        "{\"b\":1,\"a\":[2,{\"c\":3}]}",
        "[1.5,2.25e-9,\"s\",true,null]",
        "[]",
        "{}",
    ];
    unsafe {
        for doc in docs {
            let jc = (p.c.json_loads)(cstr(doc).as_ptr(), 0, std::ptr::null_mut());
            let jr = (p.r.json_loads)(cstr(doc).as_ptr(), 0, std::ptr::null_mut());
            // indent 31 (max), 32 (wraps to 0), 33, 63, 64
            // precision 31 (max), 32 (wraps to 0), 33
            // plus undefined high bits
            let mut masks: Vec<usize> = Vec::new();
            for n in [30usize, 31, 32, 33, 63, 64, 0xFF] {
                masks.push(n);
                masks.push(json_indent(n));
            }
            for n in [30usize, 31, 32, 33, 63] {
                masks.push(n << 11);
                masks.push(json_real_precision(n));
            }
            for b in [
                1usize << 11,
                1 << 16,
                1 << 17,
                1 << 20,
                1 << 31,
                1 << 32,
                1 << 40,
                1 << 62,
                usize::MAX,
                usize::MAX ^ 0x1F,
            ] {
                masks.push(b);
                masks.push(b | JSON_ENCODE_ANY);
            }
            for m in masks {
                assert_eq!(
                    dumps(p.c, jc, m),
                    dumps(p.r, jr, m),
                    "json_dumps doc={doc} flags={m:#x}"
                );
                let mut cb = vec![0x5ai8; 8192];
                let mut rb = vec![0x5ai8; 8192];
                let n1 = (p.c.json_dumpb)(jc, cb.as_mut_ptr(), 8192, m);
                let n2 = (p.r.json_dumpb)(jr, rb.as_mut_ptr(), 8192, m);
                assert_eq!(n1, n2, "json_dumpb doc={doc} flags={m:#x}");
                assert_eq!(cb, rb, "json_dumpb bytes doc={doc} flags={m:#x}");
            }
            decref(p.c, jc);
            decref(p.r, jr);
            // decoder flags: undefined bits must be ignored identically
            for m in [
                1usize << 5,
                1 << 6,
                1 << 20,
                1 << 31,
                1 << 62,
                usize::MAX,
                0x1F | (1 << 31),
            ] {
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let a = (p.c.json_loads)(cstr(doc).as_ptr(), m, &mut ec);
                let b = (p.r.json_loads)(cstr(doc).as_ptr(), m, &mut er);
                assert_eq!(ec.snapshot(), er.snapshot(), "json_loads doc={doc} flags={m:#x}");
                assert_eq!(
                    dumps(p.c, a, JSON_ENCODE_ANY),
                    dumps(p.r, b, JSON_ENCODE_ANY),
                    "json_loads result doc={doc} flags={m:#x}"
                );
                decref(p.c, a);
                decref(p.r, b);
            }
            // pack/unpack flags: undefined bits
            for m in [1usize << 2, 1 << 31, usize::MAX] {
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let a = (p.c.json_pack_ex)(&mut ec, m, cstr("[i]").as_ptr(), 1i32);
                let b = (p.r.json_pack_ex)(&mut er, m, cstr("[i]").as_ptr(), 1i32);
                assert_eq!(dumps(p.c, a, 0), dumps(p.r, b, 0), "json_pack_ex flags={m:#x}");
                assert_eq!(ec.snapshot(), er.snapshot());
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let mut iv: c_int = 0;
                let ra = (p.c.json_unpack_ex)(a, &mut ec, m, cstr("[i]").as_ptr(), &mut iv);
                let mut iv2: c_int = 0;
                let rb = (p.r.json_unpack_ex)(b, &mut er, m, cstr("[i]").as_ptr(), &mut iv2);
                assert_eq!((ra, iv), (rb, iv2), "json_unpack_ex flags={m:#x}");
                assert_eq!(ec.snapshot(), er.snapshot());
                decref(p.c, a);
                decref(p.r, b);
            }
        }
    }
}

/* ============ rows 258, 259 ============ */

#[test]
fn row_258_array_index_boundaries() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            for n in 0..12usize {
                let a = (api.json_array)();
                for i in 0..n {
                    (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                }
                for idx in [
                    0usize,
                    n.saturating_sub(1),
                    n,
                    n + 1,
                    usize::MAX,
                    usize::MAX - 1,
                ] {
                    out.push(format!(
                        "n={n} idx={idx} get={} set={} rem={} ins={}",
                        (api.json_array_get)(a, idx).is_null(),
                        (api.json_array_set_new)(a, idx, (api.json_integer)(7)),
                        (api.json_array_remove)(a, idx),
                        (api.json_array_insert_new)(a, idx, (api.json_integer)(7))
                    ));
                    out.push(format!("  -> {:?}", dumps(api, a, 0)));
                }
                decref(api, a);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "array index boundary step {i}");
    }
}

#[test]
fn row_259_version_cmp_out_of_range() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x259);
    let mut cases: Vec<(c_int, c_int, c_int)> = vec![
        (2, 15, 0),
        (2, 15, -1),
        (2, -1, 0),
        (-1, 15, 0),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MAX, 0),
        (0, i32::MIN, i32::MAX),
    ];
    for _ in 0..2000 {
        cases.push((
            rng.i64() as c_int,
            rng.i64() as c_int,
            rng.i64() as c_int,
        ));
    }
    unsafe {
        for (a, b, c) in cases {
            assert_eq!(
                (p.c.jansson_version_cmp)(a, b, c),
                (p.r.jansson_version_cmp)(a, b, c),
                "jansson_version_cmp({a},{b},{c})"
            );
        }
    }
}
