//! Phase C — ERRORS.md rows 99–145: `utf.c`, `strbuffer.c`, `hashtable.c`,
//! `memory.c`, `error.c`, `strconv.c`.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ============ rows 99..115: utf.c ============ */

#[test]
fn e_rows_99_100_utf8_encode_range() {
    let _g = lock();
    let p = pair();
    unsafe {
        for cp in [
            i32::MIN,
            -1000,
            -2,
            -1,
            0x110000,
            0x110001,
            0x200000,
            0x7FFFFFFF,
            0x10FFFF, // last valid
            0,        // first valid
        ] {
            let mut cb = [0i8; 8];
            let mut rb = [0i8; 8];
            let mut cs = 0xdeadusize;
            let mut rs = 0xdeadusize;
            let a = (p.c.utf8_encode)(cp, cb.as_mut_ptr(), &mut cs);
            let b = (p.r.utf8_encode)(cp, rb.as_mut_ptr(), &mut rs);
            assert_eq!((a, cs), (b, rs), "utf8_encode({cp})");
            if a == 0 {
                assert_eq!(cb, rb);
            }
        }
    }
}

#[test]
fn e_rows_101_103_utf8_check_first_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // continuation bytes, 0xC0/0xC1, and >= 0xF5 must all return 0
        for b in (0x80u16..=0xBF).chain([0xC0, 0xC1]).chain(0xF5..=0xFF) {
            let c = b as u8 as c_char;
            let a = (p.c.utf8_check_first)(c);
            let r = (p.r.utf8_check_first)(c);
            assert_eq!(a, r, "utf8_check_first(0x{b:02x})");
            assert_eq!(a, 0, "C should reject 0x{b:02x}");
        }
    }
}

#[test]
fn e_rows_104_108_utf8_check_full_rejections() {
    let _g = lock();
    let p = pair();
    let cases: &[(&[u8], usize, &str)] = &[
        (b"\xc2\x80", 0, "size 0"),
        (b"\xc2\x80", 1, "size 1"),
        (b"\xc2\x80\x80\x80\x80", 5, "size 5"),
        (b"\xc2\x80\x80\x80\x80", 100, "size 100"),
        (b"\xc2\x7f", 2, "bad continuation < 0x80"),
        (b"\xc2\xc0", 2, "bad continuation > 0xBF"),
        (b"\xe2\x82\x00", 3, "bad continuation NUL"),
        (b"\xf4\x90\x80\x80", 4, "> 0x10FFFF"),
        (b"\xf7\xbf\xbf\xbf", 4, "> 0x10FFFF (2)"),
        (b"\xed\xa0\x80", 3, "surrogate D800"),
        (b"\xed\xbf\xbf", 3, "surrogate DFFF"),
        (b"\xc0\x80", 2, "overlong 2-byte"),
        (b"\xc1\xbf", 2, "overlong 2-byte (2)"),
        (b"\xe0\x80\x80", 3, "overlong 3-byte"),
        (b"\xe0\x9f\xbf", 3, "overlong 3-byte (2)"),
        (b"\xf0\x80\x80\x80", 4, "overlong 4-byte"),
        (b"\xf0\x8f\xbf\xbf", 4, "overlong 4-byte (2)"),
    ];
    unsafe {
        for (buf, size, what) in cases {
            let cb: Vec<c_char> = buf.iter().map(|&x| x as c_char).collect();
            let mut cc: i32 = -1;
            let mut rc: i32 = -1;
            let a = (p.c.utf8_check_full)(cb.as_ptr(), *size, &mut cc);
            let r = (p.r.utf8_check_full)(cb.as_ptr(), *size, &mut rc);
            assert_eq!((a, cc), (r, rc), "utf8_check_full {what}");
            assert_eq!(a, 0, "C should reject {what}");
        }
    }
}

#[test]
fn e_rows_109_115_utf8_iterate_and_check_string_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 109: bufsize == 0 returns the input pointer, not NULL
        let buf = [0x41i8, 0x42, 0x43];
        let a = (p.c.utf8_iterate)(buf.as_ptr(), 0, std::ptr::null_mut());
        let r = (p.r.utf8_iterate)(buf.as_ptr(), 0, std::ptr::null_mut());
        assert_eq!(a, buf.as_ptr());
        assert_eq!(r, buf.as_ptr());

        let cases: &[(&[u8], usize, &str)] = &[
            (b"\x80", 1, "row 110: continuation lead"),
            (b"\xc0\x80", 2, "row 110: 0xC0 lead"),
            (b"\xf5\x80\x80\x80", 4, "row 110: 0xF5 lead"),
            (b"\xc2", 1, "row 111: truncated 2-byte"),
            (b"\xe2\x82", 2, "row 111: truncated 3-byte"),
            (b"\xf0\x9f\x98", 3, "row 111: truncated 4-byte"),
            (b"\xc2\x41", 2, "row 112: bad continuation"),
            (b"\xed\xa0\x80", 3, "row 112: surrogate"),
            (b"\xc0\x80", 2, "row 112: overlong"),
        ];
        for (buf, size, what) in cases {
            let cb: Vec<c_char> = buf.iter().map(|&x| x as c_char).collect();
            let mut cc: i32 = -1;
            let mut rc: i32 = -1;
            let a = (p.c.utf8_iterate)(cb.as_ptr(), *size, &mut cc);
            let r = (p.r.utf8_iterate)(cb.as_ptr(), *size, &mut rc);
            assert_eq!(a.is_null(), r.is_null(), "utf8_iterate {what}");
            assert!(a.is_null(), "C should reject {what}");
            assert_eq!(cc, rc, "codepoint out-param for {what}");
        }

        // rows 113,114,115: utf8_check_string
        let scases: &[(&[u8], &str)] = &[
            (b"\x80", "row 113"),
            (b"\xc0\x80", "row 113 (0xC0)"),
            (b"\xff", "row 113 (0xFF)"),
            (b"\xc2", "row 114 truncated at end"),
            (b"a\xe2\x82", "row 114 truncated 3-byte"),
            (b"\xf0\x9f\x98", "row 114 truncated 4-byte"),
            (b"\xc2\x41", "row 115 bad continuation"),
            (b"\xed\xa0\x80", "row 115 surrogate"),
            (b"abc\xf4\x90\x80\x80", "row 115 out of range"),
        ];
        for (buf, what) in scases {
            let cb: Vec<c_char> = buf.iter().map(|&x| x as c_char).collect();
            let a = (p.c.utf8_check_string)(cb.as_ptr(), buf.len());
            let r = (p.r.utf8_check_string)(cb.as_ptr(), buf.len());
            assert_eq!(a, r, "utf8_check_string {what}");
            assert_eq!(a, 0, "C should reject {what}");
        }
    }
}

/* ============ rows 116..121: strbuffer.c ============ */

#[test]
fn e_rows_117_119_strbuffer_overflow_guards() {
    let _g = lock();
    let p = pair();
    unsafe {
        // Row 118: size == SIZE_MAX.  The guard fires before `data` is read, so
        // a NULL data pointer is safe here (and proves it is never touched).
        for api in [p.c, p.r] {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            let r = (api.strbuffer_append_bytes)(&mut sb, std::ptr::null(), usize::MAX);
            assert_eq!(r, -1, "{}: size==SIZE_MAX must be rejected", api.tag);
            assert_eq!(sb.length, 0);
            (api.strbuffer_close)(&mut sb);
        }
        // Row 119: length > SIZE_MAX - 1 - size
        let mut res = Vec::new();
        for api in [p.c, p.r] {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            assert_eq!(
                (api.strbuffer_append_bytes)(&mut sb, b"12345".as_ptr() as *const c_char, 5),
                0
            );
            let r = (api.strbuffer_append_bytes)(&mut sb, std::ptr::null(), usize::MAX - 3);
            res.push((r, sb.length, sb.size));
            (api.strbuffer_close)(&mut sb);
        }
        assert_eq!(res[0], res[1], "row 119 guard");
        assert_eq!(res[0].0, -1);

        // Row 117: strbuff->size > SIZE_MAX / 2.  Forge the size field, keep a
        // real (small) value buffer; the guard fires before any memcpy.
        let mut res = Vec::new();
        for api in [p.c, p.r] {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            let real_size = sb.size;
            sb.size = usize::MAX / 2 + 1;
            let r = (api.strbuffer_append_bytes)(&mut sb, std::ptr::null(), usize::MAX / 2 + 1);
            res.push((r, sb.length));
            sb.size = real_size;
            (api.strbuffer_close)(&mut sb);
        }
        assert_eq!(res[0], res[1], "row 117 guard");
        assert_eq!(res[0].0, -1);
    }
}

#[test]
fn e_row_121_strbuffer_pop_underflow() {
    let _g = lock();
    let p = pair();
    unsafe {
        let mut res = Vec::new();
        for api in [p.c, p.r] {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            let mut v = Vec::new();
            v.push((api.strbuffer_pop)(&mut sb)); // empty -> '\0'
            (api.strbuffer_append_bytes)(&mut sb, b"ab".as_ptr() as *const c_char, 2);
            v.push((api.strbuffer_pop)(&mut sb));
            v.push((api.strbuffer_pop)(&mut sb));
            v.push((api.strbuffer_pop)(&mut sb)); // underflow again
            res.push((v, sb.length));
            (api.strbuffer_close)(&mut sb);
        }
        assert_eq!(res[0], res[1]);
        assert_eq!(res[0].0[0], 0);
    }
}

/* ============ rows 123..126: hashtable.c lookups that miss ============ */

#[test]
fn e_rows_123_126_hashtable_misses() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let mut ht = HashtableT::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            // row 126: iterator over an empty table
            out.push(format!("empty iter={}", (api.hashtable_iter)(&mut ht).is_null()));
            // rows 123,124,125: missing key
            for k in [&b""[..], &b"a"[..], &b"missing"[..]] {
                out.push(format!(
                    "{k:?} del={} get={} iter_at={}",
                    (api.hashtable_del)(&mut ht, k.as_ptr() as *const c_char, k.len()),
                    (api.hashtable_get)(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null(),
                    (api.hashtable_iter_at)(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null()
                ));
            }
            // populate then miss again (non-empty buckets path)
            for i in 0..12 {
                let k = format!("k{i}");
                (api.hashtable_set)(
                    &mut ht,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    (api.json_integer)(i),
                );
            }
            for k in [&b"k0"[..], &b"k0x"[..], &b"zzz"[..], &b""[..]] {
                out.push(format!(
                    "populated {k:?} get={} del={} iter_at={}",
                    (api.hashtable_get)(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null(),
                    (api.hashtable_del)(&mut ht, k.as_ptr() as *const c_char, k.len()),
                    (api.hashtable_iter_at)(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null()
                ));
            }
            // key_len mismatch: same bytes, different length
            out.push(format!(
                "prefix get={} ",
                (api.hashtable_get)(&mut ht, b"k1".as_ptr() as *const c_char, 1).is_null()
            ));
            (api.hashtable_close)(&mut ht);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ============ rows 129..134: memory.c ============ */

#[test]
fn e_rows_129_134_memory_guards() {
    let _g = lock();
    let p = pair();
    unsafe {
        for api in [p.c, p.r] {
            // row 129
            assert!((api.jsonp_malloc)(0).is_null(), "{}: jsonp_malloc(0)", api.tag);
            // row 130
            (api.jsonp_free)(std::ptr::null_mut());
            // rows 133,134
            (api.json_get_alloc_funcs)(std::ptr::null_mut(), std::ptr::null_mut());
            (api.json_get_alloc_funcs2)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            // only one out-param
            let mut m: MallocFn = None;
            (api.json_get_alloc_funcs)(&mut m, std::ptr::null_mut());
            assert!(m.is_some());
            let mut r: ReallocFn = None;
            (api.json_get_alloc_funcs2)(std::ptr::null_mut(), &mut r, std::ptr::null_mut());
        }
        // rows 131,132 are covered under the instrumented allocator in alloc.rs;
        // here the default (realloc present) path with newSize == 0 is compared.
        let mut res = Vec::new();
        for api in [p.c, p.r] {
            let q = (api.jsonp_malloc)(32);
            let z = (api.jsonp_realloc)(q, 32, 0);
            res.push(z.is_null());
        }
        assert_eq!(res[0], res[1], "jsonp_realloc(..,0) with the default allocator");
    }
}

/* ============ rows 135..142: error.c ============ */

#[test]
fn e_rows_135_142_error_guards() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<(i32, i32, i32, Vec<u8>, Vec<u8>)> {
        let mut out = Vec::new();
        unsafe {
            // rows 135,137,139: NULL error must be a no-op (no crash)
            (api.jsonp_error_init)(std::ptr::null_mut(), cstr("src").as_ptr());
            (api.jsonp_error_set_source)(std::ptr::null_mut(), cstr("src").as_ptr());
            (api.jsonp_error_set)(
                std::ptr::null_mut(),
                1,
                2,
                3usize,
                4,
                cstr("msg").as_ptr(),
            );

            // row 136: NULL source
            let mut e = JsonError::zeroed();
            (api.jsonp_error_init)(&mut e, std::ptr::null());
            out.push(e.snapshot());

            // row 137: NULL source into a live error
            (api.jsonp_error_set_source)(&mut e, std::ptr::null());
            out.push(e.snapshot());

            // row 138: source longer than JSON_ERROR_SOURCE_LENGTH
            for n in [78usize, 79, 80, 81, 100, 300] {
                let src = (0..n)
                    .map(|i| (b'a' + (i % 26) as u8) as char)
                    .collect::<String>();
                let mut e = JsonError::zeroed();
                (api.jsonp_error_init)(&mut e, cstr(&src).as_ptr());
                out.push(e.snapshot());
                (api.jsonp_error_set_source)(&mut e, cstr(&src).as_ptr());
                out.push(e.snapshot());
            }

            // rows 140,141,142: already-set text, over-long text, out-of-range code
            for code in [0i32, 17, 18, 127, 128, 200, 255, 256, -1] {
                let mut e = JsonError::zeroed();
                (api.jsonp_error_init)(&mut e, cstr("s").as_ptr());
                let long = "X".repeat(400);
                (api.jsonp_error_set)(
                    &mut e,
                    7,
                    8,
                    9usize,
                    code,
                    cstr("%s").as_ptr(),
                    cstr(&long).as_ptr(),
                );
                out.push(e.snapshot());
                // second set must be ignored
                (api.jsonp_error_set)(&mut e, 1, 1, 1usize, 1, cstr("second").as_ptr());
                out.push(e.snapshot());
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "error guard row {i}");
    }
}

/* ============ rows 143..145: strconv.c ============ */

#[test]
fn e_rows_143_145_strconv_errors() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 143: overflow to +-HUGE_VAL with ERANGE
        for lit in ["1e999", "-1e999", "1e400", "-1e400", "1" .to_string().as_str(), "1e308", "1e309"] {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let mut sb = StrbufferT::zeroed();
                assert_eq!((api.strbuffer_init)(&mut sb), 0);
                (api.strbuffer_append_bytes)(&mut sb, lit.as_ptr() as *const c_char, lit.len());
                let mut o: f64 = -1.0;
                let r = (api.jsonp_strtod)(&mut sb, &mut o);
                res.push((r, o.to_bits()));
                (api.strbuffer_close)(&mut sb);
            }
            assert_eq!(res[0], res[1], "jsonp_strtod({lit})");
        }
        // rows 144,145: jsonp_dtostr with a buffer that is too small
        for v in [0.0f64, 1.0, -1.0, 1e300, -1.7976931348623157e308, 5e-324] {
            for prec in [0i32, 1, 17, 20, 25, 31] {
                for size in 0..30usize {
                    let mut cb = vec![0i8; 64];
                    let mut rb = vec![0i8; 64];
                    let a = (p.c.jsonp_dtostr)(cb.as_mut_ptr(), size, v, prec);
                    let b = (p.r.jsonp_dtostr)(rb.as_mut_ptr(), size, v, prec);
                    assert_eq!(a, b, "jsonp_dtostr({v:?}, prec={prec}, size={size})");
                    if a >= 0 {
                        assert_eq!(cb, rb, "jsonp_dtostr buffer contents");
                    }
                }
            }
        }
    }
}

/* ============ rows 116/122/127/128: allocation-failure guards ============ */
/*  These need the allocation to fail; see alloc.rs f4 for the sweep.  Here the
    two libraries are compared for the specific inits that can fail.           */

static mut FAIL_C: bool = false;
static mut FAIL_R: bool = false;

unsafe extern "C" fn m_c(n: usize) -> *mut c_void {
    unsafe {
        if FAIL_C {
            std::ptr::null_mut()
        } else {
            (libc().malloc)(n)
        }
    }
}
unsafe extern "C" fn m_r(n: usize) -> *mut c_void {
    unsafe {
        if FAIL_R {
            std::ptr::null_mut()
        } else {
            (libc().malloc)(n)
        }
    }
}
unsafe extern "C" fn fr(pp: *mut c_void) {
    unsafe { (libc().free)(pp) }
}
unsafe extern "C" fn rl(pp: *mut c_void, n: usize) -> *mut c_void {
    unsafe { (libc().realloc)(pp, n) }
}

#[test]
fn e_rows_116_122_init_allocation_failures() {
    let _g = lock();
    let p = pair();
    let l = libc();
    unsafe {
        (p.c.json_set_alloc_funcs2)(Some(m_c), Some(rl), Some(fr));
        (p.r.json_set_alloc_funcs2)(Some(m_r), Some(rl), Some(fr));
        FAIL_C = true;
        FAIL_R = true;
        let mut res = Vec::new();
        for (api, _tag) in [(p.c, 0), (p.r, 1)] {
            let mut sb = StrbufferT::zeroed();
            let si = (api.strbuffer_init)(&mut sb); // row 116
            let mut ht = HashtableT::zeroed();
            let hi = (api.hashtable_init)(&mut ht); // row 122
            // every entry point that starts with one of those inits
            let o = (api.json_object)();
            let a = (api.json_array)();
            let s = (api.json_string)(cstr("x").as_ptr());
            let i = (api.json_integer)(1);
            let r = (api.json_real)(1.0);
            let mut e = JsonError::zeroed();
            let j = (api.json_loads)(cstr("[1]").as_ptr(), 0, &mut e);
            let d = (api.json_dumps)((api.json_true)(), JSON_ENCODE_ANY);
            let dc = (api.json_dump_callback)((api.json_true)(), None, std::ptr::null_mut(), JSON_ENCODE_ANY);
            let dcp = (api.json_deep_copy)((api.json_true)());
            let ur = (api.json_object_update_recursive)(std::ptr::null_mut(), std::ptr::null_mut());
            let pk = (api.json_pack)(cstr("[i]").as_ptr(), 1i32);
            let sp = (api.json_sprintf)(cstr("abc").as_ptr());
            res.push(format!(
                "si={si} hi={hi} obj={} arr={} str={} int={} real={} load={} code={} dumps={} dumpcb={} deep={} updrec={} pack={} sprintf={}",
                o.is_null(),
                a.is_null(),
                s.is_null(),
                i.is_null(),
                r.is_null(),
                j.is_null(),
                e.code(),
                d.is_null(),
                dc,
                dcp.is_null(),
                ur,
                pk.is_null(),
                sp.is_null()
            ));
        }
        FAIL_C = false;
        FAIL_R = false;
        (p.c.json_set_alloc_funcs2)(Some(l.malloc), Some(l.realloc), Some(l.free));
        (p.r.json_set_alloc_funcs2)(Some(l.malloc), Some(l.realloc), Some(l.free));
        assert_eq!(res[0], res[1], "allocation-failure guards");
    }
}

#[allow(unused)]
fn _u(_: c_int) {}
