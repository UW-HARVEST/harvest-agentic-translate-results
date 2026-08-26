//! Phase B — CONFIGS.md row 130: the `get_decimal_point()` / `to_locale()`
//! branch in `strconv.c` only runs when the current `LC_NUMERIC` locale uses a
//! decimal separator other than '.'.

mod common;
use common::*;
use std::os::raw::{c_char, c_int};
use std::ptr;

const LC_NUMERIC: c_int = 1;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

fn pick_comma_locale() -> Option<String> {
    for cand in [
        "de_DE.utf8",
        "de_DE.UTF-8",
        "fr_FR.utf8",
        "nl_NL.utf8",
        "ru_RU.utf8",
        "es_ES.utf8",
        "de_DE",
        "fr_FR",
    ] {
        let c = cs(cand);
        unsafe {
            let prev = setlocale(LC_NUMERIC, ptr::null());
            let prev = if prev.is_null() {
                "C".to_string()
            } else {
                std::ffi::CStr::from_ptr(prev).to_string_lossy().into_owned()
            };
            let got = setlocale(LC_NUMERIC, c.as_ptr());
            let ok = !got.is_null();
            // check the decimal point really is a comma
            let mut buf = [0u8; 8];
            let mut is_comma = false;
            if ok {
                extern "C" {
                    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
                }
                let f = cs("%#.0f");
                snprintf(buf.as_mut_ptr() as *mut c_char, 8, f.as_ptr(), 1.0f64);
                is_comma = buf[1] == b',';
            }
            let back = cs(&prev);
            setlocale(LC_NUMERIC, back.as_ptr());
            if ok && is_comma {
                return Some(cand.to_string());
            }
        }
    }
    None
}

#[test]
fn cfg130_decimal_comma_locale() {
    let Some(loc) = pick_comma_locale() else {
        eprintln!("no comma-decimal locale available; skipping row 130");
        return;
    };
    eprintln!("using locale {loc} for row 130");
    diff("cfg130 comma decimal locale", |api, rec| unsafe {
        let want = cs(&loc);
        let c_loc = cs("C");
        setlocale(LC_NUMERIC, want.as_ptr());

        // jsonp_strtod must still parse '.'-formatted numbers (to_locale rewrites
        // the separator in place first)
        for t in [
            "0", "-0", "1", "0.5", "-0.5", "3.14159265358979", "1e5", "1e-5", "1.5e300",
            "1e400", "1e-400", "2.2250738585072014e-308", "5e-324",
        ] {
            let mut sb = Strbuffer::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            assert_eq!(
                (api.strbuffer_append_bytes)(&mut sb, t.as_ptr() as *const c_char, t.len()),
                0
            );
            let mut out: f64 = -1.0;
            rec.tag_i("strtod_ret", (api.jsonp_strtod)(&mut sb, &mut out) as i64);
            rec.tag_f("strtod_out", out);
            // to_locale rewrote the buffer in place — that is observable
            rec.tag_bytes(
                "buffer_after",
                std::slice::from_raw_parts(sb.value as *const u8, sb.length),
            );
            (api.strbuffer_close)(&mut sb);
        }

        // jsonp_dtostr uses dtoa (no locale awareness) and must stay '.'-based
        for v in [0.5f64, -1.25, 1e16, 1e17, 1.0 / 3.0, 5e-324, f64::MAX] {
            for prec in [0i32, 1, 5, 17] {
                let mut buf = [0x5Au8; 64];
                let r = (api.jsonp_dtostr)(buf.as_mut_ptr() as *mut c_char, 40, v, prec);
                rec.tag_i("dtostr", r as i64);
                if r >= 0 {
                    rec.tag_bytes("dtostr_buf", &buf[..(r as usize) + 1]);
                }
            }
        }

        // and the full parse/encode pipeline
        for doc in [
            r#"[0.5,-1.25,1e300,1e-300,3.141592653589793]"#,
            r#"{"a":1.5,"b":[2.25,1e16,1e17]}"#,
            r#"[1,2,3]"#,
            r#"[1e400]"#,
        ] {
            let z = cs(doc);
            let mut e = JsonError::patterned();
            let j = (api.json_loads)(z.as_ptr(), 0, &mut e);
            rec.json("j", j);
            rec.error("err", &e);
            rec_dump_all(api, rec, "j", j);
            for prec in 0..32usize {
                match dumps(api, j, json_real_precision(prec)) {
                    None => rec.line("dump=NULL"),
                    Some(d) => rec.tag_bytes("dump", &d),
                }
            }
            decref(api, j);
        }
        setlocale(LC_NUMERIC, c_loc.as_ptr());
    });
}
