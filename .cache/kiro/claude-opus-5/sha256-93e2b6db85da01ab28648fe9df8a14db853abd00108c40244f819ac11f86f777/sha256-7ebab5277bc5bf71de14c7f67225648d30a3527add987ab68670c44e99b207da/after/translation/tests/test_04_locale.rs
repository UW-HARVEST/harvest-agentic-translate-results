//! The C library is compiled with `-DENABLE_LOCALES`, so `get_decimal_point()`
//! reads `localeconv()->decimal_point[0]`.  `setlocale` is process global and
//! affects both libraries at once, so this lives in its own test binary.
mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}
const LC_ALL: c_int = 6; // glibc

const NUMBERS: &[&str] = &[
    "0", "1", "-1", "1.5", "-1.5", "0.1", "3.14159265358979", "1e3", "1.5e3",
    "1.5e-3", "-0.0", "123456.789", "1e308", "1e-308", "2147483648",
    "9007199254740993", "0.30000000000000004", "1.0000000000000002",
];

const DOCS: &[&str] = &[
    "[1.5,2.5,3.5]",
    "{\"a\":1.25,\"b\":[0.5,-0.5]}",
    "1.7976931348623157e308",
    "\"1.5\"",
    "[1.5e-7,0.1,0.2,0.3]",
];

fn locale_names() -> Vec<&'static str> {
    vec!["C", "de_DE.utf8", "de_DE", "fr_FR.utf8", "ru_RU.utf8", "C.utf8", ""]
}

#[test]
fn decimal_point_locales() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for name in locale_names() {
            let ln = CString::new(name).unwrap();
            let applied = setlocale(LC_ALL, ln.as_ptr());
            if applied.is_null() {
                // locale not installed on this machine - nothing to compare
                continue;
            }

            for n in NUMBERS.iter().chain(DOCS.iter()) {
                let input = CString::new(*n).unwrap();
                let ct = a.c.cJSON_Parse(input.as_ptr());
                let rt = a.rust.cJSON_Parse(input.as_ptr());
                let ctx = format!("locale={name} parse({n:?})");
                assert_eq!(ct.is_null(), rt.is_null(), "{ctx}: nullness");
                if !ct.is_null() {
                    assert_tree_eq(&ctx, ct, rt);
                }
                a.c.cJSON_Delete(ct);
                a.rust.cJSON_Delete(rt);
            }

            // printing numbers built programmatically
            for v in [
                0.0f64, 1.0, -1.0, 1.5, 0.1, 1e-7, 1e21, 1.0 / 3.0, 2147483648.0,
                f64::MAX, f64::MIN_POSITIVE, f64::INFINITY, f64::NAN, 1234.5678,
            ] {
                let cp = a.c.cJSON_CreateNumber(v);
                let rp = a.rust.cJSON_CreateNumber(v);
                assert_tree_eq(&format!("locale={name} CreateNumber({v:?})"), cp, rp);
                a.c.cJSON_Delete(cp);
                a.rust.cJSON_Delete(rp);
            }
        }

        // restore
        let c = CString::new("C").unwrap();
        setlocale(LC_ALL, c.as_ptr());
    }
}
