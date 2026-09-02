//! Phase B/C — value.c scalars: string / integer / real / singletons.
//! CONFIGS rows 22-29 · ERRORS rows 60-82, 91, 94-96.
mod common;
use common::*;
use std::ffi::{c_char, c_void};

/* ---------------- CONFIGS 22 / ERRORS 75, 76 ---------------- */

#[test]
fn json_integer_roundtrip() {
    unsafe {
        let mut vals: Vec<i64> = vec![0, 1, -1, i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1];
        let mut rng = Rng::new(0x1001);
        for _ in 0..5000 {
            vals.push(rng.next_u64() as i64);
        }
        for &v in &vals {
            let cj = (c().json_integer)(v);
            let rj = (r().json_integer)(v);
            assert!(!cj.is_null() && !rj.is_null());
            assert_eq!((*cj).type_, (*rj).type_);
            assert_eq!((*cj).type_, JSON_INTEGER);
            assert_eq!((*cj).refcount, (*rj).refcount);
            assert_eq!(
                (c().json_integer_value)(cj),
                (r().json_integer_value)(rj),
                "json_integer_value({v})"
            );
            assert_eq!(
                (c().json_number_value)(cj).to_bits(),
                (r().json_number_value)(rj).to_bits(),
                "json_number_value({v})"
            );
            // json_integer_set
            let nv = v.wrapping_neg();
            assert_eq!(
                (c().json_integer_set)(cj, nv),
                (r().json_integer_set)(rj, nv)
            );
            assert_eq!((c().json_integer_value)(cj), (r().json_integer_value)(rj));

            // ERRORS 75/76: wrong type
            let cs_ = (c().json_string)(cs("x").as_ptr());
            let rs_ = (r().json_string)(cs("x").as_ptr());
            assert_eq!(
                (c().json_integer_value)(cs_),
                (r().json_integer_value)(rs_)
            );
            assert_eq!((c().json_integer_value)(cs_), 0);
            assert_eq!((c().json_integer_set)(cs_, 1), (r().json_integer_set)(rs_, 1));
            assert_eq!((c().json_integer_set)(cs_, 1), -1);
            decref(c(), cs_);
            decref(r(), rs_);
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 75: NULL
        assert_eq!(
            (c().json_integer_value)(std::ptr::null()),
            (r().json_integer_value)(std::ptr::null())
        );
        assert_eq!(
            (c().json_integer_set)(std::ptr::null_mut(), 5),
            (r().json_integer_set)(std::ptr::null_mut(), 5)
        );
    }
}

/* ---------------- CONFIGS 23 / ERRORS 77-81 ---------------- */

#[test]
fn json_real_roundtrip_and_nan_inf_rejection() {
    let _g = dtoa_guard();
    unsafe {
        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            5e-324,
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut rng = Rng::new(0x1002);
        for _ in 0..4000 {
            vals.push(f64::from_bits(rng.next_u64()));
            vals.push(rng.f64_smallish());
        }
        for &v in &vals {
            let cj = (c().json_real)(v);
            let rj = (r().json_real)(v);
            assert_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_real({v:?} bits={:#x}) null-ness (ERRORS 77/78)",
                v.to_bits()
            );
            if cj.is_null() {
                assert!(v.is_nan() || v.is_infinite());
                continue;
            }
            assert_eq!((*cj).type_, JSON_REAL);
            assert_eq!((*rj).type_, JSON_REAL);
            assert_eq!(
                (c().json_real_value)(cj).to_bits(),
                (r().json_real_value)(rj).to_bits(),
                "json_real_value"
            );
            assert_eq!(
                (c().json_number_value)(cj).to_bits(),
                (r().json_number_value)(rj).to_bits(),
                "json_number_value"
            );
            // ERRORS 81: json_real_set with NaN/Inf must fail
            for &bad in &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert_eq!(
                    (c().json_real_set)(cj, bad),
                    (r().json_real_set)(rj, bad),
                    "json_real_set(NaN/Inf)"
                );
                assert_eq!((c().json_real_set)(cj, bad), -1);
            }
            // value unchanged after failed set
            assert_eq!(
                (c().json_real_value)(cj).to_bits(),
                (r().json_real_value)(rj).to_bits()
            );
            let nv = -v;
            assert_eq!((c().json_real_set)(cj, nv), (r().json_real_set)(rj, nv));
            assert_eq!(
                (c().json_real_value)(cj).to_bits(),
                (r().json_real_value)(rj).to_bits()
            );
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 79/80: wrong type
        let ci = (c().json_integer)(7);
        let ri = (r().json_integer)(7);
        assert_eq!(
            (c().json_real_value)(ci).to_bits(),
            (r().json_real_value)(ri).to_bits()
        );
        assert_eq!((c().json_real_value)(ci), 0.0);
        assert_eq!((c().json_real_set)(ci, 1.0), (r().json_real_set)(ri, 1.0));
        assert_eq!((c().json_real_set)(ci, 1.0), -1);
        decref(c(), ci);
        decref(r(), ri);
        // NULL
        assert_eq!(
            (c().json_real_value)(std::ptr::null()).to_bits(),
            (r().json_real_value)(std::ptr::null()).to_bits()
        );
        assert_eq!(
            (c().json_real_set)(std::ptr::null_mut(), 1.0),
            (r().json_real_set)(std::ptr::null_mut(), 1.0)
        );
    }
}

/* ---------------- CONFIGS 24 / ERRORS 82 ---------------- */

#[test]
fn json_number_value_on_every_type() {
    unsafe {
        for api in both() {
            let vals: Vec<*mut JsonT> = vec![
                (api.json_integer)(-42),
                (api.json_real)(1.5),
                (api.json_string)(cs("x").as_ptr()),
                (api.json_true)(),
                (api.json_false)(),
                (api.json_null)(),
                (api.json_array)(),
                (api.json_object)(),
            ];
            let got: Vec<u64> = vals
                .iter()
                .map(|&p| (api.json_number_value)(p).to_bits())
                .collect();
            let expect: Vec<u64> = vec![
                (-42.0f64).to_bits(),
                1.5f64.to_bits(),
                0.0f64.to_bits(),
                0.0f64.to_bits(),
                0.0f64.to_bits(),
                0.0f64.to_bits(),
                0.0f64.to_bits(),
                0.0f64.to_bits(),
            ];
            assert_eq!(got, expect, "{}: json_number_value", api.tag);
            assert_eq!(
                (api.json_number_value)(std::ptr::null()).to_bits(),
                0.0f64.to_bits()
            );
            for p in vals {
                decref(api, p);
            }
        }
    }
}

/* ------- CONFIGS 25/26/27/28 · ERRORS 60-74 ------- */

#[test]
fn json_string_constructors_and_setters() {
    unsafe {
        let mut rng = Rng::new(0x1003);
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"hello world".to_vec(),
            "héllo".as_bytes().to_vec(),
            "€uro".as_bytes().to_vec(),
            "😀😀".as_bytes().to_vec(),
            vec![0xC2],             // invalid: truncated
            vec![0xED, 0xA0, 0x80], // invalid: surrogate
            vec![0xFF],             // invalid
            vec![0x41, 0x00, 0x42], // embedded NUL
        ];
        for _ in 0..1500 {
            cases.push(rng.utf8(20).into_bytes());
            let n = rng.below(24);
            cases.push(rng.bytes(n));
        }
        let big = vec![b'z'; 4096];
        cases.push(big);

        for bytes in &cases {
            let buf = cbytes(bytes);
            let p = buf.as_ptr() as *const c_char;
            let n = bytes.len();
            let strlen_n = bytes.iter().position(|&b| b == 0).unwrap_or(n);

            // json_stringn / json_stringn_nocheck with explicit len
            let cj = (c().json_stringn)(p, n);
            let rj = (r().json_stringn)(p, n);
            assert_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_stringn({bytes:02x?}, {n}) null-ness"
            );
            if !cj.is_null() {
                assert_eq!(shape(c(), cj), shape(r(), rj), "json_stringn shape");
                assert_eq!((c().json_string_length)(cj), n);
            }
            let cn = (c().json_stringn_nocheck)(p, n);
            let rn = (r().json_stringn_nocheck)(p, n);
            assert_eq!(cn.is_null(), rn.is_null());
            assert_eq!(shape(c(), cn), shape(r(), rn), "json_stringn_nocheck shape");

            // json_string / json_string_nocheck use strlen
            let cs2 = (c().json_string)(p);
            let rs2 = (r().json_string)(p);
            assert_eq!(cs2.is_null(), rs2.is_null(), "json_string({bytes:02x?})");
            if !cs2.is_null() {
                assert_eq!(shape(c(), cs2), shape(r(), rs2), "json_string shape");
                assert_eq!((c().json_string_length)(cs2), strlen_n);
            }
            let cs3 = (c().json_string_nocheck)(p);
            let rs3 = (r().json_string_nocheck)(p);
            assert_eq!(cs3.is_null(), rs3.is_null());
            assert_eq!(shape(c(), cs3), shape(r(), rs3), "json_string_nocheck shape");

            // setters on a live string
            let ctarget = (c().json_string_nocheck)(cs("initial").as_ptr());
            let rtarget = (r().json_string_nocheck)(cs("initial").as_ptr());
            assert_eq!(
                (c().json_string_setn)(ctarget, p, n),
                (r().json_string_setn)(rtarget, p, n),
                "json_string_setn({bytes:02x?}, {n})"
            );
            assert_eq!(shape(c(), ctarget), shape(r(), rtarget), "after setn");
            assert_eq!(
                (c().json_string_setn_nocheck)(ctarget, p, n),
                (r().json_string_setn_nocheck)(rtarget, p, n)
            );
            assert_eq!(shape(c(), ctarget), shape(r(), rtarget), "after setn_nocheck");
            assert_eq!(
                (c().json_string_set)(ctarget, p),
                (r().json_string_set)(rtarget, p),
                "json_string_set({bytes:02x?})"
            );
            assert_eq!(shape(c(), ctarget), shape(r(), rtarget), "after set");
            assert_eq!(
                (c().json_string_set_nocheck)(ctarget, p),
                (r().json_string_set_nocheck)(rtarget, p)
            );
            assert_eq!(shape(c(), ctarget), shape(r(), rtarget), "after set_nocheck");

            for x in [cj, cn, cs2, cs3, ctarget] {
                decref(c(), x);
            }
            for x in [rj, rn, rs2, rs3, rtarget] {
                decref(r(), x);
            }
        }
    }
}

#[test]
fn json_string_null_and_wrong_type_rejections() {
    unsafe {
        let nul: *const c_char = std::ptr::null();
        // ERRORS 61/63/65/66
        assert_eq!(
            (c().json_string)(nul).is_null(),
            (r().json_string)(nul).is_null()
        );
        assert!((c().json_string)(nul).is_null());
        assert!((r().json_string)(nul).is_null());
        assert!((c().json_stringn)(nul, 0).is_null());
        assert!((r().json_stringn)(nul, 0).is_null());
        assert!((c().json_stringn)(nul, 10).is_null());
        assert!((r().json_stringn)(nul, 10).is_null());
        assert!((c().json_string_nocheck)(nul).is_null());
        assert!((r().json_string_nocheck)(nul).is_null());
        assert!((c().json_stringn_nocheck)(nul, 5).is_null());
        assert!((r().json_stringn_nocheck)(nul, 5).is_null());
        // ERRORS 60: jsonp_stringn_nocheck_own(NULL)
        assert!((c().jsonp_stringn_nocheck_own)(nul, 0).is_null());
        assert!((r().jsonp_stringn_nocheck_own)(nul, 0).is_null());

        // ERRORS 67/68: getters on wrong type
        for api in both() {
            for &p in &[
                (api.json_integer)(1),
                (api.json_real)(1.0),
                (api.json_true)(),
                (api.json_null)(),
                (api.json_array)(),
                (api.json_object)(),
            ] {
                assert!(
                    (api.json_string_value)(p).is_null(),
                    "{}: json_string_value wrong type",
                    api.tag
                );
                assert_eq!((api.json_string_length)(p), 0);
                // ERRORS 70/72/73: setters on wrong type
                assert_eq!((api.json_string_set)(p, cs("x").as_ptr()), -1);
                assert_eq!((api.json_string_setn)(p, cs("x").as_ptr(), 1), -1);
                assert_eq!((api.json_string_set_nocheck)(p, cs("x").as_ptr()), -1);
                assert_eq!((api.json_string_setn_nocheck)(p, cs("x").as_ptr(), 1), -1);
                decref(api, p);
            }
            assert!((api.json_string_value)(std::ptr::null()).is_null());
            assert_eq!((api.json_string_length)(std::ptr::null()), 0);
            // ERRORS 69/71/72/73: NULL value
            let s = (api.json_string)(cs("live").as_ptr());
            assert_eq!((api.json_string_set)(s, nul), -1);
            assert_eq!((api.json_string_setn)(s, nul, 0), -1);
            assert_eq!((api.json_string_set_nocheck)(s, nul), -1);
            assert_eq!((api.json_string_setn_nocheck)(s, nul, 0), -1);
            // ERRORS 74: invalid UTF-8 through the checking setter
            let bad = [0xC2u8, 0x00];
            assert_eq!(
                (api.json_string_setn)(s, bad.as_ptr() as *const c_char, 1),
                -1
            );
            assert_eq!(
                (api.json_string_set)(s, bad.as_ptr() as *const c_char),
                -1
            );
            // value must still be the original
            assert_eq!(
                std::ffi::CStr::from_ptr((api.json_string_value)(s)).to_bytes(),
                b"live"
            );
            decref(api, s);
        }
    }
}

/* ---------------- CONFIGS 28: jsonp_stringn_nocheck_own ---------------- */

#[test]
fn jsonp_stringn_nocheck_own_takes_ownership() {
    unsafe {
        let mut rng = Rng::new(0x1004);
        for _ in 0..500 {
            let n = rng.below(40);
            let data = rng.bytes(n);
            for api in both() {
                // buffer must come from the same library's allocator
                let buf = (api.jsonp_malloc)(n + 1) as *mut u8;
                assert!(!buf.is_null());
                std::ptr::copy_nonoverlapping(data.as_ptr(), buf, n);
                *buf.add(n) = 0;
                let j = (api.jsonp_stringn_nocheck_own)(buf as *const c_char, n);
                assert!(!j.is_null());
                assert_eq!((api.json_string_length)(j), n);
                let sv = (api.json_string_value)(j);
                assert_eq!(sv as usize, buf as usize, "{}: must reuse buffer", api.tag);
                assert_eq!(
                    std::slice::from_raw_parts(sv as *const u8, n),
                    &data[..]
                );
                decref(api, j);
            }
        }
    }
}

/* ---------------- CONFIGS 29 / ERRORS 91, 92 ---------------- */

#[test]
fn singletons_identity_and_refcount() {
    unsafe {
        for api in both() {
            let t1 = (api.json_true)();
            let t2 = (api.json_true)();
            let f1 = (api.json_false)();
            let n1 = (api.json_null)();
            assert_eq!(t1, t2, "{}: json_true is a singleton", api.tag);
            assert_eq!((api.json_false)(), f1);
            assert_eq!((api.json_null)(), n1);
            assert_eq!((*t1).type_, JSON_TRUE);
            assert_eq!((*f1).type_, JSON_FALSE);
            assert_eq!((*n1).type_, JSON_NULL);
            assert_eq!((*t1).refcount, usize::MAX, "{}: refcount == (size_t)-1", api.tag);
            assert_eq!((*f1).refcount, usize::MAX);
            assert_eq!((*n1).refcount, usize::MAX);
            // ERRORS 91: json_delete(NULL) is a no-op
            (api.json_delete)(std::ptr::null_mut());
            // json_delete must not be called for singletons; the refcount stays.
            assert_eq!((*t1).refcount, usize::MAX);
        }
    }
}

/* ---------------- CONFIGS 108 / ERRORS 92 ---------------- */

#[test]
fn json_delete_every_type_and_bad_type() {
    unsafe {
        for api in both() {
            for mk in 0..5 {
                let p = match mk {
                    0 => (api.json_object)(),
                    1 => (api.json_array)(),
                    2 => (api.json_string)(cs("del").as_ptr()),
                    3 => (api.json_integer)(9),
                    _ => (api.json_real)(9.0),
                };
                assert_eq!((*p).refcount, 1);
                (api.json_delete)(p); // direct delete, as the C header allows internally
            }
            // ERRORS 92: out-of-range type => json_delete returns without freeing
            let p = (api.json_integer)(1);
            (*p).type_ = 99;
            (api.json_delete)(p); // must be a no-op; would double-free otherwise
            (*p).type_ = JSON_INTEGER;
            (api.json_delete)(p);
        }
    }
}

/* ---------------- CONFIGS 105 / ERRORS 94-96: sprintf ---------------- */

#[test]
fn json_sprintf_differential() {
    let _g = dtoa_guard();
    unsafe {
        // ERRORS 96: empty result must yield json_string("")
        let fmt = cs("%s");
        let empty = cs("");
        let cj = (c().json_sprintf)(fmt.as_ptr(), empty.as_ptr());
        let rj = (r().json_sprintf)(fmt.as_ptr(), empty.as_ptr());
        assert!(!cj.is_null() && !rj.is_null());
        assert_eq!(shape(c(), cj), shape(r(), rj), "json_sprintf empty");
        assert_eq!((c().json_string_length)(cj), 0);
        decref(c(), cj);
        decref(r(), rj);

        // %d / %s / %% / %f / long output
        let mut rng = Rng::new(0x1005);
        for _ in 0..400 {
            let n = rng.next_u32() as i32;
            let f = cs("n=%d s=%s pct=%% f=%.3f");
            let sarg = cs(&rng.key(12));
            let d = rng.f64_smallish();
            let cj = (c().json_sprintf)(f.as_ptr(), n, sarg.as_ptr(), d);
            let rj = (r().json_sprintf)(f.as_ptr(), n, sarg.as_ptr(), d);
            assert_eq!(cj.is_null(), rj.is_null(), "json_sprintf null-ness");
            if !cj.is_null() {
                assert_eq!(shape(c(), cj), shape(r(), rj), "json_sprintf({n}, {d:?})");
                decref(c(), cj);
                decref(r(), rj);
            }
        }

        // long (> 4096) result
        let long_arg = cs(&"q".repeat(5000));
        let f = cs("%s");
        let cj = (c().json_sprintf)(f.as_ptr(), long_arg.as_ptr());
        let rj = (r().json_sprintf)(f.as_ptr(), long_arg.as_ptr());
        assert_eq!(shape(c(), cj), shape(r(), rj), "json_sprintf long");
        assert_eq!((c().json_string_length)(cj), 5000);
        decref(c(), cj);
        decref(r(), rj);

        // multi-byte UTF-8 through the formatter
        let utf = cs("héllo€😀");
        let cj = (c().json_sprintf)(f.as_ptr(), utf.as_ptr());
        let rj = (r().json_sprintf)(f.as_ptr(), utf.as_ptr());
        assert_eq!(shape(c(), cj), shape(r(), rj), "json_sprintf utf8");
        decref(c(), cj);
        decref(r(), rj);

        // ERRORS 95: invalid UTF-8 in the formatted result => NULL
        let bad = [0xFFu8, 0xFE, 0x00];
        let cj = (c().json_sprintf)(f.as_ptr(), bad.as_ptr() as *const c_char);
        let rj = (r().json_sprintf)(f.as_ptr(), bad.as_ptr() as *const c_char);
        assert_eq!(cj.is_null(), rj.is_null(), "ERRORS 95: invalid UTF-8");
        assert!(cj.is_null());
        assert!(rj.is_null());

        // literal format with no args
        for lit in ["", "plain", "100%%", "\t\n"] {
            let f = cs(lit);
            let cj = (c().json_sprintf)(f.as_ptr());
            let rj = (r().json_sprintf)(f.as_ptr());
            assert_eq!(cj.is_null(), rj.is_null(), "json_sprintf({lit:?})");
            if !cj.is_null() {
                assert_eq!(shape(c(), cj), shape(r(), rj), "json_sprintf({lit:?})");
                decref(c(), cj);
                decref(r(), rj);
            }
        }
        let _ = std::mem::size_of::<*mut c_void>();
    }
}
