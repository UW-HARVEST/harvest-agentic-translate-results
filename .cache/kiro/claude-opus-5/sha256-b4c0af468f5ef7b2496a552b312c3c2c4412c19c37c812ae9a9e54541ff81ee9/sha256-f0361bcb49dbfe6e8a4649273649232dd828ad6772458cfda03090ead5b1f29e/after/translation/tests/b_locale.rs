//! CONFIGS rows 9-14 / 58, locale axis: the C library is built with
//! `ENABLE_LOCALES=ON`, so `get_decimal_point()` reads
//! `localeconv()->decimal_point[0]`.  Under a locale whose decimal separator is
//! `,` this changes both the number-parsing path (the `.` in the scratch buffer
//! is rewritten to the locale character before `strtod`) and the number-printing
//! path (the locale character is rewritten back to `.`).
//!
//! `setlocale` is process-global and shared by both loaded libraries, so these
//! tests hold the global lock and always restore `LC_ALL=C` afterwards.
mod harness;
use harness::*;
use std::ffi::{c_char, c_int};

const LC_ALL: c_int = 6;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn localeconv() -> *mut LConvHead;
}

#[repr(C)]
struct LConvHead {
    decimal_point: *const c_char,
}

fn decimal_point() -> u8 {
    unsafe { *(*localeconv()).decimal_point as u8 }
}

fn with_locale<F: FnOnce()>(name: &str, f: F) -> bool {
    let n = cstr(name);
    unsafe {
        let ok = setlocale(LC_ALL, sp(&n));
        if ok.is_null() {
            let c = cstr("C");
            setlocale(LC_ALL, sp(&c));
            return false;
        }
        f();
        let c = cstr("C");
        setlocale(LC_ALL, sp(&c));
    }
    true
}

fn number_corpus() -> Vec<f64> {
    let mut v = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 1.5, 1e5, 1e-5, 1e300, 1e-300, 1.0 / 3.0, 0.1, 0.2,
        3.141592653589793, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, f64::MIN,
        f64::MIN_POSITIVE, f64::EPSILON, 1234567890.12345, 1.0000000000000002,
        i32::MAX as f64, i32::MIN as f64, 5e-324,
    ];
    let mut rng = Rng::new(0x10C4_1E5F);
    for _ in 0..300 {
        v.push(rng.nice_f64());
    }
    v
}

fn run_number_axis(label: &str) {
    let p = pair();
    unsafe {
        for d in number_corpus() {
            let ci = (p.c.cJSON_CreateNumber)(d);
            let ri = (p.r.cJSON_CreateNumber)(d);
            assert_snap_eq(ci, ri, &format!("{label}: CreateNumber({d:?})"));
            assert_eq!(
                print_all(&p.c, ci),
                print_all(&p.r, ri),
                "{label}: print CreateNumber(bits {:#x})",
                d.to_bits()
            );
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // parse side
        let texts = [
            "0", "-0", "1", "1.5", "-1.5", "1e5", "1E+5", "1e-5", "1e400", "-1e400", "1e-400",
            "0.1", "3.141592653589793", "1.0000000000000002", "123456789012345678901234567890",
            "1,5", "1.5.5", ".5", "1.", "[1.5,2.25,-3.125]", "{\"a\":0.1,\"b\":1e-7}",
        ];
        for t in texts {
            let b = cstr(t);
            let ci = (p.c.cJSON_Parse)(sp(&b));
            let ri = (p.r.cJSON_Parse)(sp(&b));
            assert_eq!(ci.is_null(), ri.is_null(), "{label}: parse {t:?} nullness");
            if !ci.is_null() {
                assert_snap_eq(ci, ri, &format!("{label}: parse {t:?}"));
                assert_eq!(
                    print_all(&p.c, ci),
                    print_all(&p.r, ri),
                    "{label}: reprint {t:?}"
                );
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
        // full round trip over randomized documents
        let mut rng = Rng::new(0x1357_9BDF);
        for _ in 0..150 {
            let doc = gen_json(&mut rng, 0);
            let b = cstr(&doc);
            let ci = (p.c.cJSON_Parse)(sp(&b));
            let ri = (p.r.cJSON_Parse)(sp(&b));
            assert_eq!(ci.is_null(), ri.is_null(), "{label}: doc {doc}");
            if !ci.is_null() {
                assert_snap_eq(ci, ri, &format!("{label}: doc {doc}"));
                assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri), "{label}: {doc}");
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}

#[test]
fn locale_c_default() {
    let _g = lock();
    assert!(with_locale("C", || run_number_axis("LC_ALL=C")));
}

#[test]
fn locale_comma_decimal_separator() {
    let _g = lock();
    let mut tried = 0;
    for name in ["de_DE.utf8", "de_DE.UTF-8", "de_DE", "fr_FR.utf8", "es_ES.utf8"] {
        let mut sep = b'?';
        if with_locale(name, || {
            sep = decimal_point();
            run_number_axis(name)
        }) {
            assert_eq!(
                sep, b',',
                "locale {name} was accepted but its decimal separator is {:?}, \
                 so the ENABLE_LOCALES branch was not exercised",
                sep as char
            );
            eprintln!("locale {name}: decimal separator is {:?}", sep as char);
            tried += 1;
            break;
        }
    }
    assert!(
        tried > 0,
        "no comma-decimal locale available on this host; the ENABLE_LOCALES \
         code path could not be exercised"
    );
}
