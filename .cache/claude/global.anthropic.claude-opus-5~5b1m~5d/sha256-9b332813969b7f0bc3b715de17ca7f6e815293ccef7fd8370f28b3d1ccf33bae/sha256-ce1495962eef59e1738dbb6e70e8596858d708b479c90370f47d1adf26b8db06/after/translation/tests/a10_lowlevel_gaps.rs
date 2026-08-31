//! Supplementary low-level coverage: the CONFIGS.md rows in the low-level
//! section that the other low-level test files do not reach.
//!
//! Specifically CONFIGS rows 290 (hashtable rehash OOM, driven through
//! `hashtable_set` directly), 321 (strbuffer realloc failure), 327
//! (strbuffer_value aliasing), 344/345 (jsonp_strtod errno hygiene and
//! `to_locale`), and 355 (`dtoa`/`dtoa_r` with a NULL `rve` out-param).

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn malloc(n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
}
unsafe fn errno_get() -> c_int {
    *__errno_location()
}
unsafe fn errno_set(v: c_int) {
    *__errno_location() = v;
}

// ---------------------------------------------------------------------------
// A malloc hook that succeeds N times and then fails, so a specific
// allocation deep inside an operation can be made to fail.
// ---------------------------------------------------------------------------

static mut C_BUDGET: i64 = i64::MAX;
static mut R_BUDGET: i64 = i64::MAX;

unsafe extern "C" fn c_budget_malloc(n: size_t) -> *mut c_void {
    if C_BUDGET <= 0 {
        return std::ptr::null_mut();
    }
    C_BUDGET -= 1;
    malloc(n)
}
unsafe extern "C" fn r_budget_malloc(n: size_t) -> *mut c_void {
    if R_BUDGET <= 0 {
        return std::ptr::null_mut();
    }
    R_BUDGET -= 1;
    malloc(n)
}
unsafe extern "C" fn budget_free(p: *mut c_void) {
    free(p)
}

/// Install the budgeted allocator on both libraries with the SAME budget, run
/// `body`, then restore the previous allocators.
unsafe fn with_budget<F: FnOnce()>(c: &Api, r: &Api, budget: i64, body: F) {
    let (mut cm, mut crl, mut cf) = (None, None, None);
    let (mut rm, mut rrl, mut rf) = (None, None, None);
    (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
    (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);

    C_BUDGET = budget;
    R_BUDGET = budget;
    // realloc slot NULL forces the malloc+copy+free emulation, so every
    // allocation — including reallocs — goes through the budgeted malloc.
    (c.json_set_alloc_funcs)(Some(c_budget_malloc), Some(budget_free));
    (r.json_set_alloc_funcs)(Some(r_budget_malloc), Some(budget_free));

    body();

    C_BUDGET = i64::MAX;
    R_BUDGET = i64::MAX;
    (c.json_set_alloc_funcs2)(cm, crl, cf);
    (r.json_set_alloc_funcs2)(rm, rrl, rf);
}

// ===========================================================================
// CONFIGS 290 — hashtable_set rehash OOM, driven through hashtable_set itself
// ===========================================================================

#[test]
fn hashtable_set_rehash_oom_returns_minus_one() {
    let _g = global_state_lock();
    // The rehash happens when `size >= hashsize(order)` (8 entries at order 3).
    // Filling to 8 with a generous budget and then cutting the budget to 0
    // makes the rehash allocation — and only it — fail.
    let (c, r) = both();
    unsafe {
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);

        // Fill to exactly the load factor with the real allocator.
        let mut cvals = Vec::new();
        let mut rvals = Vec::new();
        for i in 0..8 {
            let k = format!("k{i}");
            let kc = cs(&k);
            assert_eq!(
                (c.hashtable_set)(&mut *cht, kc.as_ptr(), k.len(), (c.json_integer)(i)),
                0
            );
            assert_eq!(
                (r.hashtable_set)(&mut *rht, kc.as_ptr(), k.len(), (r.json_integer)(i)),
                0
            );
            cvals.push(i);
            rvals.push(i);
        }
        // Values for the failing insert, made while allocation still works.
        let cextra = (c.json_integer)(999);
        let rextra = (r.json_integer)(999);

        with_budget(c, r, 0, || {
            let k = cs("k8");
            let cret = (c.hashtable_set)(&mut *cht, k.as_ptr(), 2, cextra);
            let rret = (r.hashtable_set)(&mut *rht, k.as_ptr(), 2, rextra);
            diff_eq!(cret, rret, "hashtable_set with failing rehash (CONFIGS 290)");
            assert_eq!(cret, -1, "C: rehash OOM must return -1");
            // The old bucket array must still be live: size/order unchanged.
            diff_eq!(
                (cht.size, cht.order),
                (rht.size, rht.order),
                "hashtable state after failed rehash"
            );
        });

        // The table must still be fully usable after the failed rehash.
        let k = cs("after");
        diff_eq!(
            (c.hashtable_set)(&mut *cht, k.as_ptr(), 5, (c.json_integer)(1)),
            (r.hashtable_set)(&mut *rht, k.as_ptr(), 5, (r.json_integer)(1)),
            "hashtable_set after a failed rehash"
        );
        diff_eq!(
            (cht.size, cht.order),
            (rht.size, rht.order),
            "hashtable state after recovery"
        );

        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);
    }
}

#[test]
fn hashtable_set_init_pair_oom_returns_minus_one() {
    let _g = global_state_lock();
    // CONFIGS 285/298-equivalent: the pair allocation itself failing, with no
    // rehash involved (table well under the load factor).
    let (c, r) = both();
    unsafe {
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);
        let cextra = (c.json_integer)(7);
        let rextra = (r.json_integer)(7);

        with_budget(c, r, 0, || {
            let k = cs("solo");
            let cret = (c.hashtable_set)(&mut *cht, k.as_ptr(), 4, cextra);
            let rret = (r.hashtable_set)(&mut *rht, k.as_ptr(), 4, rextra);
            diff_eq!(cret, rret, "hashtable_set with failing init_pair");
            assert_eq!(cret, -1, "C: init_pair OOM must return -1");
            diff_eq!((cht.size, cht.order), (rht.size, rht.order), "state unchanged");
        });

        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);
    }
}

// ===========================================================================
// CONFIGS 321 — strbuffer_append_bytes realloc failure
// ===========================================================================

#[test]
fn strbuffer_append_bytes_realloc_failure() {
    let _g = global_state_lock();
    // Init with a working allocator (so `value` is a valid 16-byte buffer),
    // then cut the budget so only the GROWTH allocation fails. The C must
    // return -1 and leave the buffer contents and size untouched.
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        assert_eq!((c.strbuffer_init)(&mut csb), 0);
        assert_eq!((r.strbuffer_init)(&mut rsb), 0);
        // Put a few bytes in so "unchanged" is observable.
        let seed = cs("abc");
        (c.strbuffer_append_bytes)(&mut csb, seed.as_ptr(), 3);
        (r.strbuffer_append_bytes)(&mut rsb, seed.as_ptr(), 3);

        with_budget(c, r, 0, || {
            // 100 bytes forces a grow past the 16-byte initial size.
            let big = vec![b'Z' as c_char; 100];
            let cret = (c.strbuffer_append_bytes)(&mut csb, big.as_ptr(), 100);
            let rret = (r.strbuffer_append_bytes)(&mut rsb, big.as_ptr(), 100);
            diff_eq!(cret, rret, "strbuffer_append_bytes realloc failure (CONFIGS 321)");
            assert_eq!(cret, -1, "C: realloc failure must return -1");
            diff_eq!(
                (csb.length, csb.size, cbytes((c.strbuffer_value)(&csb))),
                (rsb.length, rsb.size, cbytes((r.strbuffer_value)(&rsb))),
                "strbuffer unchanged after a failed grow"
            );
        });

        // Still usable afterwards.
        let more = cs("de");
        diff_eq!(
            (c.strbuffer_append_bytes)(&mut csb, more.as_ptr(), 2),
            (r.strbuffer_append_bytes)(&mut rsb, more.as_ptr(), 2),
            "strbuffer usable after a failed grow"
        );
        diff_eq!(
            (csb.length, csb.size, cbytes((c.strbuffer_value)(&csb))),
            (rsb.length, rsb.size, cbytes((r.strbuffer_value)(&rsb))),
            "strbuffer state after recovery"
        );

        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

// ===========================================================================
// CONFIGS 327 — strbuffer_value aliasing
// ===========================================================================

#[test]
fn strbuffer_value_aliases_the_live_buffer() {
    let _g = global_state_lock();
    // `strbuffer_value` returns `strbuff->value` directly, so the pointer must
    // equal the struct field and must track reallocation. Both libraries must
    // agree on when the pointer changes identity (i.e. when a grow happened).
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);

        assert_eq!(
            (c.strbuffer_value)(&csb),
            csb.value as *const c_char,
            "C: value() must alias the field"
        );
        assert_eq!(
            (r.strbuffer_value)(&rsb),
            rsb.value as *const c_char,
            "Rust: value() must alias the field"
        );

        let mut csize_prev = csb.size;
        let mut rsize_prev = rsb.size;
        for i in 0..200 {
            let b = (b'a' + (i % 26) as u8) as c_char;
            (c.strbuffer_append_byte)(&mut csb, b);
            (r.strbuffer_append_byte)(&mut rsb, b);

            let cnow = (c.strbuffer_value)(&csb);
            let rnow = (r.strbuffer_value)(&rsb);
            // The returned pointer must always still alias the struct field —
            // that is the whole contract of strbuffer_value.
            assert_eq!(cnow, csb.value as *const c_char, "C: aliasing broke at step {i}");
            assert_eq!(rnow, rsb.value as *const c_char, "Rust: aliasing broke at step {i}");
            // Compare whether a GROW happened this step via the `size` field.
            // (Pointer identity is deliberately NOT compared: whether realloc
            // moves the block in place is an allocator decision, not library
            // behaviour, and differs run to run.)
            diff_eq!(
                csb.size != csize_prev,
                rsb.size != rsize_prev,
                "grew at the same step (step {i})"
            );
            diff_eq!(
                (csb.length, csb.size, cbytes(cnow)),
                (rsb.length, rsb.size, cbytes(rnow)),
                "aliased contents at step {i}"
            );
            csize_prev = csb.size;
            rsize_prev = rsb.size;
        }

        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

// ===========================================================================
// CONFIGS 344/345 — jsonp_strtod errno hygiene and to_locale
// ===========================================================================

#[test]
fn jsonp_strtod_errno_hygiene() {
    let _g = global_state_lock();
    // The C sets `errno = 0` before calling strtod, so a stale ERANGE left by
    // an earlier operation must NOT be mistaken for an overflow. Pre-poison
    // errno and confirm a perfectly ordinary literal still succeeds.
    let (c, r) = both();
    unsafe {
        for stale in [0, 34 /* ERANGE */, 22 /* EINVAL */, 2, 999] {
            for text in ["1.5", "0", "-2.25", "1e10", "1e-10"] {
                let run = |api: &Api| -> (c_int, u64, c_int) {
                    let mut sb = strbuffer_t::zeroed();
                    (api.strbuffer_init)(&mut sb);
                    let b = text.as_bytes();
                    (api.strbuffer_append_bytes)(&mut sb, b.as_ptr() as *const c_char, b.len());
                    let mut out = f64::from_bits(0x0BAD_0BAD_0BAD_0BAD);
                    errno_set(stale);
                    let ret = (api.jsonp_strtod)(&mut sb, &mut out);
                    let e = errno_get();
                    (api.strbuffer_close)(&mut sb);
                    (ret, out.to_bits(), e)
                };
                let (cret, cbits, ce) = run(c);
                let (rret, rbits, re) = run(r);
                diff_eq!(cret, rret, "jsonp_strtod({text:?}) with stale errno={stale}: return");
                diff_eq!(cbits, rbits, "jsonp_strtod({text:?}) with stale errno={stale}: bits");
                diff_eq!(ce, re, "jsonp_strtod({text:?}) with stale errno={stale}: errno after");
                assert_eq!(
                    cret, 0,
                    "C: a stale errno={stale} must not make {text:?} fail"
                );
            }
        }
    }
}

#[test]
fn jsonp_strtod_to_locale_rewrites_the_decimal_point() {
    let _g = global_state_lock();
    // `to_locale` finds the locale's decimal point via sprintf("%#.0f", 1.0)
    // and rewrites the first '.' in the buffer to match. Neither library calls
    // setlocale, so the point stays '.' and the conversion is a no-op — but the
    // function still SCANS the buffer, so a buffer with several dots, with no
    // dot, or with a leading dot must be handled identically.
    let (c, r) = both();
    unsafe {
        // Only complete double literals, because of the live assert in
        // jsonp_strtod that all input be consumed.
        for text in [
            "1.5", ".5", "0.0", "-0.5", "1", "-1", "1e5", "1.5e5", "0.000001",
            "123456.789012", "-0.0",
        ] {
            let run = |api: &Api| -> (c_int, u64, Option<Vec<u8>>) {
                let mut sb = strbuffer_t::zeroed();
                (api.strbuffer_init)(&mut sb);
                let b = text.as_bytes();
                (api.strbuffer_append_bytes)(&mut sb, b.as_ptr() as *const c_char, b.len());
                let mut out = 0.0f64;
                let ret = (api.jsonp_strtod)(&mut sb, &mut out);
                // to_locale may have MUTATED the buffer in place; capture it.
                let buf = cbytes((api.strbuffer_value)(&sb));
                (api.strbuffer_close)(&mut sb);
                (ret, out.to_bits(), buf)
            };
            let (cret, cbits, cbuf) = run(c);
            let (rret, rbits, rbuf) = run(r);
            diff_eq!(cret, rret, "to_locale({text:?}) return");
            diff_eq!(cbits, rbits, "to_locale({text:?}) value bits");
            diff_eq!(cbuf, rbuf, "to_locale({text:?}) buffer after conversion");
        }
    }
}

// ===========================================================================
// CONFIGS 355 — dtoa / dtoa_r with a NULL rve out-param
// ===========================================================================

#[test]
fn dtoa_with_null_rve() {
    let _g = global_state_lock();
    // `rve` is optional: the C writes through it only `if (rve)`. Passing NULL
    // must be accepted and must not change the digits produced.
    let (c, r) = both();
    let mut rng = Rng::new(0xA0_0001);
    unsafe {
        let mut values: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.1, 1e308, 5e-324, 1.0 / 3.0,
            f64::MAX, f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
        ];
        for _ in 0..3000 {
            values.push(rng.real());
        }
        for v in values {
            for mode in [0, 1, 2, 3, 4, 5] {
                for ndigits in [0, 1, 17] {
                    let mut cd = -12345;
                    let mut cs2 = -12345;
                    let mut rd = -12345;
                    let mut rs2 = -12345;
                    let cp = (c.dtoa)(
                        v, mode, ndigits, &mut cd, &mut cs2, std::ptr::null_mut(),
                    );
                    let rp = (r.dtoa)(
                        v, mode, ndigits, &mut rd, &mut rs2, std::ptr::null_mut(),
                    );
                    let ctx = format!("value={v:e} mode={mode} ndigits={ndigits}");
                    diff_eq!(cbytes(cp), cbytes(rp), "dtoa(rve=NULL) digits [{ctx}]");
                    diff_eq!(cd, rd, "dtoa(rve=NULL) *decpt [{ctx}]");
                    diff_eq!(cs2, rs2, "dtoa(rve=NULL) *sign [{ctx}]");
                    if !cp.is_null() {
                        (c.freedtoa)(cp);
                    }
                    if !rp.is_null() {
                        (r.freedtoa)(rp);
                    }
                }
            }
        }
    }
}

#[test]
fn dtoa_r_with_null_rve() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xA0_0002);
    unsafe {
        for i in 0..8000 {
            let v = rng.real();
            let mode = rng.below(10) as c_int;
            let ndigits = rng.below(25) as c_int;
            let blen = rng.below(50);
            let mut cbuf = [0xAAu8; 64];
            let mut rbuf = [0xAAu8; 64];
            let mut cd = -12345;
            let mut cs2 = -12345;
            let mut rd = -12345;
            let mut rs2 = -12345;
            let cp = (c.dtoa_r)(
                v, mode, ndigits, &mut cd, &mut cs2, std::ptr::null_mut(),
                cbuf.as_mut_ptr() as *mut c_char, blen,
            );
            let rp = (r.dtoa_r)(
                v, mode, ndigits, &mut rd, &mut rs2, std::ptr::null_mut(),
                rbuf.as_mut_ptr() as *mut c_char, blen,
            );
            let ctx =
                format!("iter={i} value={v:e} mode={mode} ndigits={ndigits} blen={blen}");
            diff_eq!(cp.is_null(), rp.is_null(), "dtoa_r(rve=NULL) null-ness [{ctx}]");
            diff_eq!(cbytes(cp), cbytes(rp), "dtoa_r(rve=NULL) digits [{ctx}]");
            diff_eq!(cd, rd, "dtoa_r(rve=NULL) *decpt [{ctx}]");
            diff_eq!(cs2, rs2, "dtoa_r(rve=NULL) *sign [{ctx}]");
            diff_eq!(cbuf, rbuf, "dtoa_r(rve=NULL) buffer [{ctx}]");
        }
    }
}

#[test]
fn dtoa_with_null_decpt_and_sign_out_params() {
    let _g = global_state_lock();
    // decpt and sign are dereferenced UNCONDITIONALLY by the C
    // (`*decpt = ...`), so NULL there would be a null deref in the C itself and
    // is NOT a supported input. This test documents that boundary by exercising
    // the supported shape — non-NULL decpt/sign with NULL rve — across the
    // special-value fast paths where the C returns early.
    let (c, r) = both();
    unsafe {
        for &v in &[0.0f64, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            for mode in 0..=9 {
                let mut cd = -12345;
                let mut cs2 = -12345;
                let mut rd = -12345;
                let mut rs2 = -12345;
                let cp = (c.dtoa)(v, mode, 0, &mut cd, &mut cs2, std::ptr::null_mut());
                let rp = (r.dtoa)(v, mode, 0, &mut rd, &mut rs2, std::ptr::null_mut());
                let ctx = format!("special v={v:e} mode={mode}");
                diff_eq!(cbytes(cp), cbytes(rp), "digits [{ctx}]");
                diff_eq!(cd, rd, "*decpt [{ctx}]");
                diff_eq!(cs2, rs2, "*sign [{ctx}]");
                if !cp.is_null() {
                    (c.freedtoa)(cp);
                }
                if !rp.is_null() {
                    (r.freedtoa)(rp);
                }
            }
        }
    }
}
