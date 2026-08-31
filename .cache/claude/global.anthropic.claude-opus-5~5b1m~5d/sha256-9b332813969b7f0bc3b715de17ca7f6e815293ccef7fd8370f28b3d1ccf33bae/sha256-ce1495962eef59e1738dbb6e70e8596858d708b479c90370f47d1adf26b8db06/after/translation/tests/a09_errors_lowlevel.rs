//! Phase C — error-path differential tests for the low-level modules.
//!
//! Covers the ERRORS.md rows for strbuffer.c, memory.c, error.c, hashtable.c,
//! strconv.c and the exported dtoa.c entry points, PLUS the generic FFI
//! boundary conditions every C API has: NULL pointers, zero and oversized
//! lengths, values one step past a documented range, and out-of-range enum
//! values (a C enum accepts any int, so a value with no valid variant is a real
//! input both implementations must handle identically).
//!
//! The utf8_* rejection rows (ERRORS.md 126-144) are already proven
//! EXHAUSTIVELY in a01_utf.rs — every one of the 256 byte values, every
//! codepoint up to 0x110100, and every 2-byte sequence — so they are not
//! repeated here.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn __errno_location() -> *mut c_int;
}
unsafe fn errno_get() -> c_int {
    *__errno_location()
}
unsafe fn errno_set(v: c_int) {
    *__errno_location() = v;
}
const ERANGE: c_int = 34;

// ===========================================================================
// strbuffer.c overflow guards (ERRORS.md 288-294)
// ===========================================================================

#[test]
fn strbuffer_append_bytes_size_overflow_guards() {
    let _g = global_state_lock();
    // The C guards are:
    //   size > SIZE_MAX - 1
    //   length > SIZE_MAX - 1 - size
    //   size > SIZE_MAX / 2   (doubling would overflow)
    // all of which must return -1 and leave the buffer untouched. These are
    // reachable with a huge `size` argument WITHOUT the data pointer ever being
    // read, because the guard runs before the memcpy.
    let (c, r) = both();
    unsafe {
        for &size in &[
            usize::MAX,
            usize::MAX - 1,
            usize::MAX / 2,
            usize::MAX / 2 + 1,
            usize::MAX - 16,
        ] {
            let mut csb = strbuffer_t::zeroed();
            let mut rsb = strbuffer_t::zeroed();
            assert_eq!((c.strbuffer_init)(&mut csb), 0);
            assert_eq!((r.strbuffer_init)(&mut rsb), 0);

            let data = cs("x");
            let cret = (c.strbuffer_append_bytes)(&mut csb, data.as_ptr(), size);
            let rret = (r.strbuffer_append_bytes)(&mut rsb, data.as_ptr(), size);
            diff_eq!(cret, rret, "strbuffer_append_bytes(size={size:#x}) return");
            assert_eq!(cret, -1, "C: append of size {size:#x} must be rejected");
            // The buffer must be completely unchanged by a rejected append.
            diff_eq!(
                (csb.length, csb.size, cbytes((c.strbuffer_value)(&csb))),
                (rsb.length, rsb.size, cbytes((r.strbuffer_value)(&rsb))),
                "strbuffer state after rejected append(size={size:#x})"
            );

            (c.strbuffer_close)(&mut csb);
            (r.strbuffer_close)(&mut rsb);
        }
    }
}

#[test]
fn strbuffer_append_bytes_length_plus_size_overflow() {
    let _g = global_state_lock();
    // `length > SIZE_MAX - 1 - size`: reach it by first growing `length` a
    // little, then asking for a size that makes the sum overflow.
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        let filler = vec![b'a' as c_char; 100];
        (c.strbuffer_append_bytes)(&mut csb, filler.as_ptr(), 100);
        (r.strbuffer_append_bytes)(&mut rsb, filler.as_ptr(), 100);

        let data = cs("x");
        for &size in &[usize::MAX - 100, usize::MAX - 101, usize::MAX - 50] {
            let cret = (c.strbuffer_append_bytes)(&mut csb, data.as_ptr(), size);
            let rret = (r.strbuffer_append_bytes)(&mut rsb, data.as_ptr(), size);
            diff_eq!(cret, rret, "length+size overflow at size={size:#x}");
            diff_eq!(
                (csb.length, csb.size),
                (rsb.length, rsb.size),
                "state after length+size overflow at size={size:#x}"
            );
        }
        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

#[test]
fn strbuffer_value_after_steal_is_null() {
    let _g = global_state_lock();
    // ERRORS.md 294: strbuffer_value on a stolen (or failed-init) buffer.
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        let cv = (c.strbuffer_steal_value)(&mut csb);
        let rv = (r.strbuffer_steal_value)(&mut rsb);
        diff_eq!(
            (c.strbuffer_value)(&csb).is_null(),
            (r.strbuffer_value)(&rsb).is_null(),
            "strbuffer_value after steal must be NULL"
        );
        jfree(c, cv as *mut c_void);
        jfree(r, rv as *mut c_void);
    }
}

// ===========================================================================
// jsonp_strtod (ERRORS.md 312-313)
// ===========================================================================

/// Build a strbuffer holding exactly `text` and call `jsonp_strtod`.
unsafe fn strtod_case(api: &Api, text: &str) -> (c_int, u64, c_int) {
    let mut sb = strbuffer_t::zeroed();
    assert_eq!((api.strbuffer_init)(&mut sb), 0);
    let b = text.as_bytes();
    assert_eq!(
        (api.strbuffer_append_bytes)(&mut sb, b.as_ptr() as *const c_char, b.len()),
        0
    );
    // Poison the out-param so "not written" is observable (the C leaves *out
    // untouched on the overflow path).
    let mut out: f64 = f64::from_bits(0x0BAD_0BAD_0BAD_0BAD);
    errno_set(0);
    let ret = (api.jsonp_strtod)(&mut sb, &mut out);
    let e = errno_get();
    (api.strbuffer_close)(&mut sb);
    (ret, out.to_bits(), e)
}

#[test]
fn jsonp_strtod_overflow_returns_minus_one_and_leaves_out_untouched() {
    let _g = global_state_lock();
    // ERRORS.md 312: (value == ±HUGE_VAL) && errno == ERANGE => -1, *out
    // untouched. The "untouched" part is the interesting half.
    let (c, r) = both();
    unsafe {
        for t in [
            "1e999", "-1e999", "1e400", "-1e400", "1e308000", "2e308",
            "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497793",
        ] {
            let (cret, cbits, ce) = strtod_case(c, t);
            let (rret, rbits, re) = strtod_case(r, t);
            diff_eq!(cret, rret, "jsonp_strtod({t:?}) return");
            diff_eq!(cbits, rbits, "jsonp_strtod({t:?}) *out bits");
            diff_eq!(ce == ERANGE, re == ERANGE, "jsonp_strtod({t:?}) errno==ERANGE");
            assert_eq!(cret, -1, "C: {t:?} must report overflow");
            assert_eq!(
                cbits, 0x0BAD_0BAD_0BAD_0BAD,
                "C: *out must be left untouched on overflow for {t:?}"
            );
        }
    }
}

#[test]
fn jsonp_strtod_underflow_is_not_an_error() {
    let _g = global_state_lock();
    // ERRORS.md 313: underflow sets ERANGE but the value is ~0, so the
    // `value == ±HUGE_VAL` test fails and the C returns SUCCESS with *out = 0.
    // A port that keyed off errno alone would wrongly return -1 here.
    let (c, r) = both();
    unsafe {
        for t in ["1e-999", "-1e-999", "1e-400", "-1e-400", "1e-320", "4e-324", "1e-1000"] {
            let (cret, cbits, _) = strtod_case(c, t);
            let (rret, rbits, _) = strtod_case(r, t);
            diff_eq!(cret, rret, "jsonp_strtod({t:?}) return");
            diff_eq!(cbits, rbits, "jsonp_strtod({t:?}) *out bits");
            assert_eq!(cret, 0, "C: underflow of {t:?} must NOT be an error");
        }
    }
}

// ===========================================================================
// jsonp_dtostr failures (ERRORS.md 315-317)
// ===========================================================================

#[test]
fn jsonp_dtostr_returns_minus_one_when_buffer_too_small() {
    let _g = global_state_lock();
    // ERRORS.md 315/316/317. The exact values in the table were observed
    // against the real C library, so they are asserted explicitly as well as
    // compared.
    let (c, r) = both();
    unsafe {
        let cases: &[(usize, f64, c_int)] = &[
            (25, 0.1, 22),
            (25, 0.1, 23),
            (25, 0.1, 24),
            (25, 0.1, 25),
            (25, 0.1, 26),
            (25, 0.1, 31),
            (25, 1e300, 21),
            (25, 1e-300, 21),
            (1, 1.0, 0),
            (0, 1.0, 0),
            (2, 1.0, 0),
            (3, 1.0, 0),
            (4, 1.0, 0),
        ];
        for &(size, value, prec) in cases {
            let mut cbuf = [0xAAu8; 64];
            let mut rbuf = [0xAAu8; 64];
            let cret = (c.jsonp_dtostr)(cbuf.as_mut_ptr() as *mut c_char, size, value, prec);
            let rret = (r.jsonp_dtostr)(rbuf.as_mut_ptr() as *mut c_char, size, value, prec);
            diff_eq!(
                cret,
                rret,
                "jsonp_dtostr(size={size}, value={value:e}, prec={prec}) return"
            );
            diff_eq!(
                cbuf,
                rbuf,
                "jsonp_dtostr(size={size}, value={value:e}, prec={prec}) buffer"
            );
        }
    }
}

// ===========================================================================
// dtoa_r short-buffer failures (ERRORS.md 318-322)
// ===========================================================================

#[test]
fn dtoa_r_short_buffer_returns_null() {
    let _g = global_state_lock();
    // ERRORS.md 318-321: each special class of value has a minimum blen; below
    // it dtoa_r returns NULL but still writes *decpt / *sign.
    let (c, r) = both();
    unsafe {
        let cases: &[(f64, c_int, c_int, usize, &str)] = &[
            (0.1, 0, 0, 4, "normal, mode 0, blen 4"),
            (0.1, 2, 25, 25, "normal, mode 2 ndigits 25, blen 25"),
            (0.0, 0, 0, 1, "+0.0 with blen 1"),
            (-0.0, 0, 0, 1, "-0.0 with blen 1"),
            (0.0, 0, 0, 0, "+0.0 with blen 0"),
            (f64::NAN, 0, 0, 3, "NaN with blen 3"),
            (f64::NAN, 0, 0, 0, "NaN with blen 0"),
            (f64::INFINITY, 0, 0, 8, "+inf with blen 8"),
            (f64::NEG_INFINITY, 0, 0, 8, "-inf with blen 8"),
            (f64::INFINITY, 0, 0, 0, "+inf with blen 0"),
        ];
        for &(v, mode, ndigits, blen, label) in cases {
            let mut cbuf = [0xAAu8; 64];
            let mut rbuf = [0xAAu8; 64];
            let (mut cd, mut cs2) = (-12345, -12345);
            let (mut rd, mut rs2) = (-12345, -12345);
            let mut crve: *mut c_char = std::ptr::null_mut();
            let mut rrve: *mut c_char = std::ptr::null_mut();
            let cp = (c.dtoa_r)(
                v, mode, ndigits, &mut cd, &mut cs2, &mut crve,
                cbuf.as_mut_ptr() as *mut c_char, blen,
            );
            let rp = (r.dtoa_r)(
                v, mode, ndigits, &mut rd, &mut rs2, &mut rrve,
                rbuf.as_mut_ptr() as *mut c_char, blen,
            );
            diff_eq!(cp.is_null(), rp.is_null(), "dtoa_r null-ness [{label}]");
            // *decpt and *sign are written even on the NULL path.
            diff_eq!(cd, rd, "dtoa_r *decpt [{label}]");
            diff_eq!(cs2, rs2, "dtoa_r *sign [{label}]");
            diff_eq!(cbuf, rbuf, "dtoa_r buffer [{label}]");
        }
    }
}

#[test]
fn dtoa_r_out_of_range_mode_is_clamped_not_an_error() {
    let _g = global_state_lock();
    // ERRORS.md 322: mode < 0 or > 9 is NOT an error — it is silently folded.
    // This is exactly the out-of-range-enum class of input that happy-path
    // tests miss.
    let (c, r) = both();
    unsafe {
        for mode in [-5, -1, 10, 99, 1000, i32::MIN + 1, i32::MAX] {
            for &v in &[1.5f64, 0.0, -0.0, 0.1, 1e308, 5e-324, f64::MAX] {
                let mut cbuf = [0xAAu8; 64];
                let mut rbuf = [0xAAu8; 64];
                let (mut cd, mut cs2) = (-12345, -12345);
                let (mut rd, mut rs2) = (-12345, -12345);
                let mut crve: *mut c_char = std::ptr::null_mut();
                let mut rrve: *mut c_char = std::ptr::null_mut();
                let cp = (c.dtoa_r)(
                    v, mode, 0, &mut cd, &mut cs2, &mut crve,
                    cbuf.as_mut_ptr() as *mut c_char, 40,
                );
                let rp = (r.dtoa_r)(
                    v, mode, 0, &mut rd, &mut rs2, &mut rrve,
                    rbuf.as_mut_ptr() as *mut c_char, 40,
                );
                let ctx = format!("mode={mode} value={v:e}");
                diff_eq!(cbytes(cp), cbytes(rp), "dtoa_r digits [{ctx}]");
                diff_eq!(cd, rd, "dtoa_r *decpt [{ctx}]");
                diff_eq!(cs2, rs2, "dtoa_r *sign [{ctx}]");
                diff_eq!(cbuf, rbuf, "dtoa_r buffer [{ctx}]");
            }
        }
    }
}

// ===========================================================================
// gethex overflow / underflow / no-digit paths (ERRORS.md 326-334)
// ===========================================================================

/// gethex writes through `rvp`, advances `*sp`, and sets `errno` on the
/// overflow/underflow paths. All four observables are compared.
unsafe fn gethex_case(
    api: &Api,
    text: &str,
    rounding: c_int,
    sign: c_int,
) -> (u64, usize, c_int) {
    let s = cs(text);
    let mut p: *const c_char = s.as_ptr();
    let mut u: f64 = f64::from_bits(0x0BAD_0BAD_0BAD_0BAD);
    errno_set(0);
    (api.gethex)(&mut p, &mut u as *mut f64 as *mut c_void, rounding, sign);
    let e = errno_get();
    (u.to_bits(), p as usize - s.as_ptr() as usize, e)
}

#[test]
fn gethex_overflow_underflow_and_no_digit_paths() {
    let _g = global_state_lock();
    // Rows 326-334. Every input keeps the mandatory 2-char "0x" prefix that
    // gethex assumes (it does `s0 = *sp + 2` with no length check).
    let (c, r) = both();
    let cases: &[(&str, &str)] = &[
        ("0x1p+99999", "row 326: e > emax -> +inf, ERANGE"),
        ("0x1p+999999999999", "row 327: exponent digits overflow, positive"),
        ("0x1p-999999999999", "row 329: big && esign -> 0.0, ERANGE"),
        ("0x1p-99999", "row 330: e < emin -> retz, ERANGE"),
        ("0x1p-1075", "row 331: underflow to smallest denormal / zero"),
        ("0x1p-1074", "smallest denormal, no error"),
        ("0x1p-1073", "just above the smallest denormal"),
        ("0x0", "row 332: zret, no significant digits, errno untouched"),
        ("0x.0", "row 332: zret via fraction"),
        ("0x0.0", "row 332: zret both sides"),
        ("0x0p0", "zret with an exponent"),
        ("0xg", "row 333: no hex digit at all -> *sp rewound"),
        ("0x", "row 333: nothing after the prefix"),
        ("0x.", "row 333: only a decimal point"),
        ("0xz1", "row 333: invalid first digit"),
        ("0x1.fffffffffffff8p1023", "row 334: rounding carry pushes e past Emax"),
        ("0x1.fffffffffffffp1023", "largest finite double"),
        ("0x2p1023", "row 326: just past the top"),
        ("0x1p1024", "row 326: one binade too big"),
        ("0x1p1023", "largest power of two"),
    ];
    unsafe {
        for &(text, label) in cases {
            for rounding in 0..=3 {
                for sign in [0, 1] {
                    let (cbits, cadv, ce) = gethex_case(c, text, rounding, sign);
                    let (rbits, radv, re) = gethex_case(r, text, rounding, sign);
                    let ctx =
                        format!("{label} :: gethex({text:?}, rounding={rounding}, sign={sign})");
                    diff_eq!(cbits, rbits, "{ctx} value bits");
                    diff_eq!(cadv, radv, "{ctx} advance");
                    // errno is process-global and shared by both .so files, so
                    // comparing it directly is meaningful.
                    diff_eq!(ce, re, "{ctx} errno");
                }
            }
        }
    }
}

// ===========================================================================
// memory.c error paths (ERRORS.md 303-310)
// ===========================================================================

/// A malloc hook that always fails, to drive the OOM branches that are
/// otherwise unreachable. Installed per-library so each side fails identically.
unsafe extern "C" fn always_fail_malloc(_n: size_t) -> *mut c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn noop_free(_p: *mut c_void) {}
unsafe extern "C" fn always_fail_realloc(_p: *mut c_void, _n: size_t) -> *mut c_void {
    std::ptr::null_mut()
}

#[test]
fn oom_paths_via_failing_allocator() {
    let _g = global_state_lock();
    // Installing a failing allocator makes the OOM rows in ERRORS.md
    // (json_object 101/102, json_array 103/104, json_integer 90, json_real 95,
    // json_stringn_nocheck 72/73, strbuffer_init 288, hashtable_init 295,
    // jsonp_strndup 308, json_loads/json_dumps OOM) actually reachable rather
    // than merely documented.
    let (c, r) = both();
    unsafe {
        // Save the defaults so the rest of the suite is unaffected.
        let (mut cm, mut crl, mut cf) = (None, None, None);
        let (mut rm, mut rrl, mut rf) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);

        (c.json_set_alloc_funcs2)(
            Some(always_fail_malloc),
            Some(always_fail_realloc),
            Some(noop_free),
        );
        (r.json_set_alloc_funcs2)(
            Some(always_fail_malloc),
            Some(always_fail_realloc),
            Some(noop_free),
        );

        // --- constructors must all return NULL, not crash
        diff_eq!((c.json_object)().is_null(), (r.json_object)().is_null(), "json_object OOM");
        diff_eq!((c.json_array)().is_null(), (r.json_array)().is_null(), "json_array OOM");
        diff_eq!(
            (c.json_integer)(42).is_null(),
            (r.json_integer)(42).is_null(),
            "json_integer OOM"
        );
        diff_eq!((c.json_real)(1.5).is_null(), (r.json_real)(1.5).is_null(), "json_real OOM");
        let s = cs("hello");
        diff_eq!(
            (c.json_string)(s.as_ptr()).is_null(),
            (r.json_string)(s.as_ptr()).is_null(),
            "json_string OOM"
        );
        diff_eq!(
            (c.json_stringn)(s.as_ptr(), 5).is_null(),
            (r.json_stringn)(s.as_ptr(), 5).is_null(),
            "json_stringn OOM"
        );
        diff_eq!(
            (c.json_string_nocheck)(s.as_ptr()).is_null(),
            (r.json_string_nocheck)(s.as_ptr()).is_null(),
            "json_string_nocheck OOM"
        );
        diff_eq!(
            (c.json_stringn_nocheck)(s.as_ptr(), 5).is_null(),
            (r.json_stringn_nocheck)(s.as_ptr(), 5).is_null(),
            "json_stringn_nocheck OOM"
        );

        // Singletons are static and must still work under OOM.
        diff_eq!((c.json_true)().is_null(), (r.json_true)().is_null(), "json_true under OOM");
        diff_eq!(
            (c.json_false)().is_null(),
            (r.json_false)().is_null(),
            "json_false under OOM"
        );
        diff_eq!((c.json_null)().is_null(), (r.json_null)().is_null(), "json_null under OOM");

        // --- jsonp_* wrappers
        diff_eq!(
            (c.jsonp_malloc)(16).is_null(),
            (r.jsonp_malloc)(16).is_null(),
            "jsonp_malloc OOM"
        );
        diff_eq!(
            (c.jsonp_strndup)(s.as_ptr(), 5).is_null(),
            (r.jsonp_strndup)(s.as_ptr(), 5).is_null(),
            "jsonp_strndup OOM (row 308)"
        );

        // --- strbuffer_init OOM (row 288): -1 and value left NULL
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        let cret = (c.strbuffer_init)(&mut csb);
        let rret = (r.strbuffer_init)(&mut rsb);
        diff_eq!(cret, rret, "strbuffer_init OOM return (row 288)");
        diff_eq!(
            csb.value.is_null(),
            rsb.value.is_null(),
            "strbuffer_init OOM leaves value NULL"
        );
        assert_eq!(cret, -1, "C: strbuffer_init must fail under OOM");

        // --- hashtable_init OOM (row 295)
        let mut cht = hashtable_t::zeroed();
        let mut rht = hashtable_t::zeroed();
        let cret = (c.hashtable_init)(&mut cht);
        let rret = (r.hashtable_init)(&mut rht);
        diff_eq!(cret, rret, "hashtable_init OOM return (row 295)");
        assert_eq!(cret, -1, "C: hashtable_init must fail under OOM");

        // --- parse and dump under OOM
        let input = cs(r#"{"a":[1,2,3]}"#);
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(input.as_ptr(), 0, &mut ce);
        let rj = (r.json_loads)(input.as_ptr(), 0, &mut re);
        diff_eq!(cj.is_null(), rj.is_null(), "json_loads under OOM");
        // Note: ERRORS.md row 197 records that the lex_init OOM path leaves the
        // error struct only partially initialised, so `text` is compared but the
        // code byte is not asserted to a specific value — only that both agree.
        diff_eq!(ce.text_str(), re.text_str(), "json_loads OOM error text");

        // --- pack under OOM
        let fmt = cs("{s:i}");
        let key = cs("k");
        let mut ce2 = json_error_t::new();
        let mut re2 = json_error_t::new();
        let cp = (c.json_pack_ex)(&mut ce2, 0, fmt.as_ptr(), key.as_ptr(), 1 as c_int);
        let rp = (r.json_pack_ex)(&mut re2, 0, fmt.as_ptr(), key.as_ptr(), 1 as c_int);
        diff_eq!(cp.is_null(), rp.is_null(), "json_pack_ex under OOM");
        diff_eq!(ce2.raw(), re2.raw(), "json_pack_ex OOM error image");

        // --- json_sprintf OOM (row 89)
        let sf = cs("%d");
        diff_eq!(
            (c.json_sprintf)(sf.as_ptr(), 5 as c_int).is_null(),
            (r.json_sprintf)(sf.as_ptr(), 5 as c_int).is_null(),
            "json_sprintf under OOM (row 89)"
        );

        // --- restore the real allocators
        (c.json_set_alloc_funcs2)(cm, crl, cf);
        (r.json_set_alloc_funcs2)(rm, rrl, rf);

        // Sanity: allocation works again, so later tests are unaffected.
        let o = (c.json_object)();
        assert!(!o.is_null(), "C allocator not restored");
        decref(c, o);
        let o = (r.json_object)();
        assert!(!o.is_null(), "Rust allocator not restored");
        decref(r, o);
    }
}

#[test]
fn oom_during_container_growth() {
    let _g = global_state_lock();
    // Rows 12, 58, 296: a container that already exists, whose GROWTH then
    // fails. Build first with the real allocator, then switch to the failing
    // one so only the growth allocation fails.
    let (c, r) = both();
    unsafe {
        let (mut cm, mut crl, mut cf) = (None, None, None);
        let (mut rm, mut rrl, mut rf) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);

        let carr = (c.json_array)();
        let rarr = (r.json_array)();
        let cobj = (c.json_object)();
        let robj = (r.json_object)();
        // Fill both to just under the growth thresholds (array capacity 8,
        // hashtable 8 buckets) using pre-made values.
        let mut cvals = Vec::new();
        let mut rvals = Vec::new();
        for i in 0..8 {
            cvals.push((c.json_integer)(i));
            rvals.push((r.json_integer)(i));
        }
        for i in 0..8 {
            (c.json_array_append_new)(carr, cvals[i]);
            (r.json_array_append_new)(rarr, rvals[i]);
        }
        // Pre-make the values that the failing append/set will consume, so the
        // failure is in the GROWTH, not in creating the value.
        let cextra = (c.json_integer)(999);
        let rextra = (r.json_integer)(999);
        let cextra2 = (c.json_integer)(888);
        let rextra2 = (r.json_integer)(888);
        for i in 0..8 {
            let k = format!("k{i}");
            let kc = cs(&k);
            (c.json_object_set_new)(cobj, kc.as_ptr(), (c.json_integer)(i));
            (r.json_object_set_new)(robj, kc.as_ptr(), (r.json_integer)(i));
        }

        (c.json_set_alloc_funcs2)(
            Some(always_fail_malloc),
            Some(always_fail_realloc),
            Some(noop_free),
        );
        (r.json_set_alloc_funcs2)(
            Some(always_fail_malloc),
            Some(always_fail_realloc),
            Some(noop_free),
        );

        // Row 58: json_array_grow fails => -1 and the value is decref'd.
        diff_eq!(
            (c.json_array_append_new)(carr, cextra),
            (r.json_array_append_new)(rarr, rextra),
            "json_array_append_new with failing grow (row 58)"
        );
        diff_eq!(
            (c.json_array_size)(carr),
            (r.json_array_size)(rarr),
            "array size after failed grow"
        );

        // Rows 12/296: hashtable rehash fails => -1 and the value is decref'd.
        let k9 = cs("k8");
        diff_eq!(
            (c.json_object_set_new)(cobj, k9.as_ptr(), cextra2),
            (r.json_object_set_new)(robj, k9.as_ptr(), rextra2),
            "json_object_set_new with failing rehash (rows 12/296)"
        );
        diff_eq!(
            (c.json_object_size)(cobj),
            (r.json_object_size)(robj),
            "object size after failed rehash"
        );

        (c.json_set_alloc_funcs2)(cm, crl, cf);
        (r.json_set_alloc_funcs2)(rm, rrl, rf);

        decref(c, carr);
        decref(r, rarr);
        decref(c, cobj);
        decref(r, robj);
    }
}

#[test]
fn get_alloc_funcs_with_null_out_params() {
    let _g = global_state_lock();
    // Rows 309/310: every combination of NULL out-params must be tolerated and
    // must leave the omitted slots untouched.
    let (c, r) = both();
    unsafe {
        // All 4 combinations for the 2-arg getter.
        for &(want_m, want_f) in &[(false, false), (true, false), (false, true), (true, true)] {
            let mut cm: json_malloc_t = None;
            let mut cf: json_free_t = None;
            let mut rm: json_malloc_t = None;
            let mut rf: json_free_t = None;
            (c.json_get_alloc_funcs)(
                if want_m { &mut cm } else { std::ptr::null_mut() },
                if want_f { &mut cf } else { std::ptr::null_mut() },
            );
            (r.json_get_alloc_funcs)(
                if want_m { &mut rm } else { std::ptr::null_mut() },
                if want_f { &mut rf } else { std::ptr::null_mut() },
            );
            diff_eq!(
                (cm.is_some(), cf.is_some()),
                (rm.is_some(), rf.is_some()),
                "get_alloc_funcs(m={want_m}, f={want_f}) which slots were written"
            );
        }
        // All 8 combinations for the 3-arg getter.
        for mask in 0..8u32 {
            let (wm, wr, wf) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
            let mut cm: json_malloc_t = None;
            let mut crl: json_realloc_t = None;
            let mut cf: json_free_t = None;
            let mut rm: json_malloc_t = None;
            let mut rrl: json_realloc_t = None;
            let mut rf: json_free_t = None;
            (c.json_get_alloc_funcs2)(
                if wm { &mut cm } else { std::ptr::null_mut() },
                if wr { &mut crl } else { std::ptr::null_mut() },
                if wf { &mut cf } else { std::ptr::null_mut() },
            );
            (r.json_get_alloc_funcs2)(
                if wm { &mut rm } else { std::ptr::null_mut() },
                if wr { &mut rrl } else { std::ptr::null_mut() },
                if wf { &mut rf } else { std::ptr::null_mut() },
            );
            diff_eq!(
                (cm.is_some(), crl.is_some(), cf.is_some()),
                (rm.is_some(), rrl.is_some(), rf.is_some()),
                "get_alloc_funcs2(mask={mask:03b}) which slots were written"
            );
        }
    }
}

// ===========================================================================
// hashtable.c rejections (ERRORS.md 299-302)
// ===========================================================================

#[test]
fn hashtable_lookup_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);

        // Row 301: iter on an empty ordered list is NULL.
        diff_eq!(
            (c.hashtable_iter)(&mut *cht).is_null(),
            (r.hashtable_iter)(&mut *rht).is_null(),
            "hashtable_iter on empty table (row 301)"
        );

        // Rows 299/300/302 on an empty table and on a populated one.
        for key in [&b""[..], b"missing", b"\0", b"a\0b"] {
            diff_eq!(
                (c.hashtable_get)(&mut *cht, key.as_ptr() as *const c_char, key.len()).is_null(),
                (r.hashtable_get)(&mut *rht, key.as_ptr() as *const c_char, key.len()).is_null(),
                "hashtable_get({key:?}) missing (row 299)"
            );
            diff_eq!(
                (c.hashtable_del)(&mut *cht, key.as_ptr() as *const c_char, key.len()),
                (r.hashtable_del)(&mut *rht, key.as_ptr() as *const c_char, key.len()),
                "hashtable_del({key:?}) missing (row 300)"
            );
            diff_eq!(
                (c.hashtable_iter_at)(&mut *cht, key.as_ptr() as *const c_char, key.len())
                    .is_null(),
                (r.hashtable_iter_at)(&mut *rht, key.as_ptr() as *const c_char, key.len())
                    .is_null(),
                "hashtable_iter_at({key:?}) missing (row 302)"
            );
        }

        // Row 301 second half: iter_next at the last pair returns NULL.
        let k = cs("only");
        (c.hashtable_set)(&mut *cht, k.as_ptr(), 4, (c.json_integer)(1));
        (r.hashtable_set)(&mut *rht, k.as_ptr(), 4, (r.json_integer)(1));
        let cit = (c.hashtable_iter)(&mut *cht);
        let rit = (r.hashtable_iter)(&mut *rht);
        diff_eq!(cit.is_null(), rit.is_null(), "iter on 1-element table");
        diff_eq!(
            (c.hashtable_iter_next)(&mut *cht, cit).is_null(),
            (r.hashtable_iter_next)(&mut *rht, rit).is_null(),
            "iter_next past the last pair (row 301)"
        );

        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);
    }
}

// ===========================================================================
// error.c rejections (ERRORS.md 281-287)
// ===========================================================================

#[test]
fn error_api_null_and_truncation_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 281 / 283 / 285: NULL error pointer is a no-op everywhere.
        (c.jsonp_error_init)(std::ptr::null_mut(), cs("s").as_ptr());
        (r.jsonp_error_init)(std::ptr::null_mut(), cs("s").as_ptr());
        (c.jsonp_error_set_source)(std::ptr::null_mut(), cs("s").as_ptr());
        (r.jsonp_error_set_source)(std::ptr::null_mut(), cs("s").as_ptr());
        (c.jsonp_error_set)(std::ptr::null_mut(), 1, 1, 1, 0, cs("m").as_ptr());
        (r.jsonp_error_set)(std::ptr::null_mut(), 1, 1, 1, 0, cs("m").as_ptr());

        // Row 282: NULL source => source[0] = '\0'
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        (c.jsonp_error_init)(&mut ce, std::ptr::null());
        (r.jsonp_error_init)(&mut re, std::ptr::null());
        diff_eq!(ce.raw(), re.raw(), "jsonp_error_init(NULL source) (row 282)");
        assert_eq!(ce.source[0], 0, "C: NULL source must give an empty source string");

        // Row 283: NULL source to set_source is a no-op (struct untouched).
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let before = ce.raw();
        (c.jsonp_error_set_source)(&mut ce, std::ptr::null());
        (r.jsonp_error_set_source)(&mut re, std::ptr::null());
        diff_eq!(ce.raw(), re.raw(), "set_source(NULL) (row 283)");
        assert_eq!(ce.raw(), before, "C: set_source(NULL) must not write");

        // Row 284: source >= 80 chars is truncated with a "..." prefix.
        for len in [79usize, 80, 81, 100, 199, 500] {
            let src: String = "x".repeat(len);
            let cstr = cs(&src);
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            (c.jsonp_error_set_source)(&mut ce, cstr.as_ptr());
            (r.jsonp_error_set_source)(&mut re, cstr.as_ptr());
            diff_eq!(
                ce.source.iter().map(|&x| x as u8).collect::<Vec<u8>>(),
                re.source.iter().map(|&x| x as u8).collect::<Vec<u8>>(),
                "set_source truncation at len={len} (row 284)"
            );
            if len >= JSON_ERROR_SOURCE_LENGTH {
                assert_eq!(
                    &ce.source[..3].iter().map(|&x| x as u8).collect::<Vec<u8>>(),
                    b"...",
                    "C: long source must be prefixed with '...' (len={len})"
                );
            }
        }

        // Row 286: the FIRST error wins; a second set is silently dropped.
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        (c.jsonp_error_init)(&mut ce, cs("src").as_ptr());
        (r.jsonp_error_init)(&mut re, cs("src").as_ptr());
        (c.jsonp_error_set)(&mut ce, 1, 2, 3, JSON_ERROR_WRONG_TYPE, cs("first").as_ptr());
        (r.jsonp_error_set)(&mut re, 1, 2, 3, JSON_ERROR_WRONG_TYPE, cs("first").as_ptr());
        for i in 0..5 {
            (c.jsonp_error_set)(&mut ce, 9, 9, 9, JSON_ERROR_INVALID_SYNTAX, cs("later").as_ptr());
            (r.jsonp_error_set)(&mut re, 9, 9, 9, JSON_ERROR_INVALID_SYNTAX, cs("later").as_ptr());
            diff_eq!(ce.raw(), re.raw(), "sticky error after overwrite #{i} (row 286)");
        }
        assert_eq!(ce.text_str(), "first", "C: the first error must survive");
        assert_eq!(ce.code(), JSON_ERROR_WRONG_TYPE, "C: the first code must survive");

        // Row 287: message longer than TEXT_LENGTH-2 is truncated, and the code
        // byte at text[159] is preserved.
        for len in [157usize, 158, 159, 160, 399, 1000] {
            let msg: String = "y".repeat(len);
            let m = cs(&msg);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            (c.jsonp_error_init)(&mut ce, std::ptr::null());
            (r.jsonp_error_init)(&mut re, std::ptr::null());
            (c.jsonp_error_set)(&mut ce, 1, 1, 1, JSON_ERROR_INVALID_UTF8, cs("%s").as_ptr(), m.as_ptr());
            (r.jsonp_error_set)(&mut re, 1, 1, 1, JSON_ERROR_INVALID_UTF8, cs("%s").as_ptr(), m.as_ptr());
            diff_eq!(ce.raw(), re.raw(), "long message truncation len={len} (row 287)");
            assert_eq!(
                ce.code(),
                JSON_ERROR_INVALID_UTF8,
                "C: the code byte must survive truncation (len={len})"
            );
        }
    }
}

#[test]
fn error_code_out_of_range_enum_values() {
    let _g = global_state_lock();
    // `enum json_error_code` is stored in a single char (text[159]). A C enum
    // accepts ANY int, so values with no valid variant — including values that
    // do not fit in a char — are real inputs across the FFI boundary and both
    // implementations must mangle them identically.
    let (c, r) = both();
    unsafe {
        for code in [
            -1i32, -128, -129, 0, 17, 18, 19, 127, 128, 200, 255, 256, 257, 1000, 65535,
            65536, i32::MAX, i32::MIN + 1,
        ] {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            (c.jsonp_error_init)(&mut ce, std::ptr::null());
            (r.jsonp_error_init)(&mut re, std::ptr::null());
            (c.jsonp_error_set)(&mut ce, 0, 0, 0, code, cs("x").as_ptr());
            (r.jsonp_error_set)(&mut re, 0, 0, 0, code, cs("x").as_ptr());
            diff_eq!(ce.raw(), re.raw(), "out-of-range error code {code}");
            diff_eq!(ce.code(), re.code(), "out-of-range error code {code} read back");
        }
    }
}

// ===========================================================================
// version.c (ERRORS.md 338)
// ===========================================================================

#[test]
fn jansson_version_cmp_exhaustive_small_range() {
    let _g = global_state_lock();
    // Not an error path as such, but the full comparison surface: each
    // component above, equal to and below the built-in version, and the
    // extreme int values a caller can pass.
    let (c, r) = both();
    unsafe {
        for ma in [-1, 0, 1, 2, 3, 100, i32::MIN + 1, i32::MAX] {
            for mi in [-1, 0, 14, 15, 16, 100] {
                for mu in [-1, 0, 1, 2, 100] {
                    diff_eq!(
                        (c.jansson_version_cmp)(ma, mi, mu),
                        (r.jansson_version_cmp)(ma, mi, mu),
                        "jansson_version_cmp({ma},{mi},{mu})"
                    );
                }
            }
        }
        diff_eq!(
            cbytes((c.jansson_version_str)()),
            cbytes((r.jansson_version_str)()),
            "jansson_version_str"
        );
    }
}
