//! Phase B — row C91: the `ENABLE_LOCALES` code path (`get_decimal_point`).
//!
//! `setlocale` is process-global, so this file is its own test binary. Under a
//! locale whose decimal separator is `,` the C implementation must still emit
//! `.` (it rewrites the separator by hand in `print_number`) and must still
//! accept `.` on input (`parse_number` rewrites it before calling `strtod`).
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

const LC_ALL: c_int = 6; // glibc

#[repr(C)]
struct Lconv {
    decimal_point: *mut c_char,
}

extern "C" {
    fn localeconv() -> *mut Lconv;
}

fn number_docs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"0".to_vec(),
        b"-0".to_vec(),
        b"0.5".to_vec(),
        b"-0.5".to_vec(),
        b"1.25e3".to_vec(),
        b"1e-5".to_vec(),
        b"3.141592653589793".to_vec(),
        b"1234567.891011".to_vec(),
        b"[0.1,0.2,0.3]".to_vec(),
        b"{\"a\":1.5,\"b\":[2.25,-3.125]}".to_vec(),
        b"1,5".to_vec(),
        b"[1,5]".to_vec(),
        b"0,5".to_vec(),
        b"2147483647.5".to_vec(),
        b"1.7976931348623157e308".to_vec(),
    ];
    let mut rng = Rng::new(0x10CA_1E00_0000_0001);
    for _ in 0..60 {
        v.push(gen_json(&mut rng));
    }
    v
}

fn run_all(api: &Api) -> String {
    let mut log = String::new();
    unsafe {
        // print every number class
        let mut rng = Rng::new(0xD3C1_4A10_0000_0002);
        let mut values: Vec<f64> = vec![
            0.0, -0.0, 0.5, -0.5, 1.0 / 3.0, 1e-5, 1e15, 1e16, 1e17, 1e21, 2147483647.5,
            f64::NAN, f64::INFINITY, f64::MIN_POSITIVE, 5e-324, f64::MAX,
        ];
        for _ in 0..300 {
            values.push(rng.nice_f64());
        }
        for v in values {
            let it = (api.cJSON_CreateNumber)(v);
            let pf = take_print(api, (api.cJSON_Print)(it));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(
                log,
                "print 0x{:016x}: fmt={} unfmt={}",
                v.to_bits(),
                pf.map(|x| show(&x)).unwrap_or("NULL".into()),
                pu.map(|x| show(&x)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(it);
        }
        // parse every document
        for d in number_docs() {
            let b = CBuf::new(&d);
            let root = (api.cJSON_Parse)(b.ptr());
            let e = (api.cJSON_GetErrorPtr)();
            let _ = writeln!(
                log,
                "parse {}: null={} err={}",
                show(&d),
                root.is_null(),
                if e.is_null() {
                    "NULL".to_string()
                } else {
                    format!("+{}", e as isize - b.ptr() as isize)
                }
            );
            let _ = write!(log, "  {}", dump(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(log, "  print={}", pu.map(|x| show(&x)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(root);
        }
    }
    log
}

#[test]
fn c91_comma_decimal_locale() {
    let (c, r) = libs();

    // baseline in the default "C" locale
    let base_c = run_all(c);
    let base_r = run_all(r);
    assert_eq!(base_c, base_r, "C locale: C and Rust differ");

    // single-byte comma separators: cJSON rewrites them, so the JSON must be
    // byte-identical to the "C" locale output.
    for name in ["de_DE.utf8", "fr_FR.utf8"] {
        let nb = cs(name);
        if unsafe { setlocale(LC_ALL, nb.as_ptr()) }.is_null() {
            eprintln!("SKIPPED {name}: locale not installed");
            continue;
        }
        let sep = unsafe { read_cstr((*localeconv()).decimal_point).unwrap() };
        assert_eq!(sep, b",", "{name}: expected a comma separator, got {sep:?}");
        let lc = run_all(c);
        let lr = run_all(r);
        assert_eq!(lc, lr, "{name}: C and Rust differ");
        assert_eq!(
            base_c, lc,
            "{name}: the C implementation's output changed with the locale"
        );
    }

    // A *multi-byte* separator (ps_AF uses U+066B, 0xD9 0xAB): `get_decimal_point`
    // only ever looks at `decimal_point[0]`, so cJSON mangles such output. That is
    // the ground truth and the Rust translation must mangle it identically.
    let ps = cs("ps_AF.utf8");
    if unsafe { setlocale(LC_ALL, ps.as_ptr()) }.is_null() {
        eprintln!("SKIPPED ps_AF.utf8: locale not installed");
    } else {
        let sep = unsafe { read_cstr((*localeconv()).decimal_point).unwrap() };
        assert!(sep.len() > 1, "expected a multi-byte separator, got {sep:?}");
        let lc = run_all(c);
        let lr = run_all(r);
        assert_eq!(lc, lr, "ps_AF.utf8: C and Rust differ");
        assert_ne!(
            base_c, lc,
            "expected the multi-byte separator to change the C output"
        );
    }

    let cl = cs("C");
    unsafe { setlocale(LC_ALL, cl.as_ptr()) };
    let back_c = run_all(c);
    let back_r = run_all(r);
    assert_eq!(back_c, back_r, "back in C locale: C and Rust differ");
    assert_eq!(base_c, back_c, "restoring the locale changed the output");
}
