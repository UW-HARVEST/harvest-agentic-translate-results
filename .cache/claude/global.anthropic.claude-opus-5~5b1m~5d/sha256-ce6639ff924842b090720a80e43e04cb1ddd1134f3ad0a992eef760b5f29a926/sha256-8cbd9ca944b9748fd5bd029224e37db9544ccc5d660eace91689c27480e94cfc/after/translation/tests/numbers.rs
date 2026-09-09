//! CONFIGS.md rows 12, 15-20: number formatting / parsing / conversion surface.
//!
//! Every value is fed to the C library and to the Rust library in forked
//! children; `diff` compares the captured bytes and the exit status.

mod common;
use common::*;
use std::ffi::{c_char, c_int};
use std::ptr::null_mut;

const SEED: u64 = 0x5EED_1234_ABCD_0001;
const N_NICE: usize = 2000;
const N_BITS: usize = 2000;

/* ------------------------------------------------------------------ values */

/// Specials + boundary values required by rows 12 and 15.
fn specials() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e21,
        1e-7,
        1e-6,
        5e-324,          /* smallest subnormal */
        -5e-324,
        2.2250738585072014e-308, /* smallest normal */
        1.7976931348623157e308,  /* DBL_MAX */
        -1.7976931348623157e308,
        123456789.0,
        2147483647.0,  /* 2^31-1 = INT_MAX */
        2147483648.0,  /* 2^31 */
        4294967295.0,  /* 2^32-1 */
        4294967296.0,  /* 2^32 */
        -2147483648.0, /* -2^31 = INT_MIN */
        -2147483649.0,
        i32::MIN as f64,
        i32::MAX as f64,
        9007199254740992.0,  /* 2^53 */
        -9007199254740992.0, /* -2^53 */
        9007199254740993.0,
        65535.5,
        65536.0,
        65535.0,
        -65535.5,
        32767.5,
        -32768.5,
        1e300,
        -1e300,
        1e-300,
        1e100,
        0.1,
        1.5,
        2.5,
        -2.5,
        3.0000000000000004,
        1e20,
        9.999999999999999e20,
        1.0000000000000002e21,
        999999999999999900000.0,
        1e-5,
        1.0000000000000002e-6,
        0.000001,
        0.0000009999999999999999,
        4503599627370496.0,
        1e15,
        1e16,
        1e17,
        123.456,
        -123.456,
        1.0 / 3.0,
        -1.0 / 3.0,
    ]
}

/// The full value set: specials + 2000 `nice_f64` + 2000 `bits_f64`.
fn all_values() -> Vec<f64> {
    let mut v = specials();
    let mut rng = Rng::new(SEED);
    for _ in 0..N_NICE {
        v.push(rng.nice_f64());
    }
    for _ in 0..N_BITS {
        v.push(rng.bits_f64());
    }
    v
}

/// Label that unambiguously identifies the input double (raw bits + index).
fn vlab(kind: &str, i: usize, v: f64) -> String {
    format!("{}[{} bits={:016x}]", kind, i, v.to_bits())
}

/* ----------------------------------------------------------------- strings */

fn digits_string(n: usize, first: u8) -> String {
    let mut s = String::new();
    s.push(first as char);
    for i in 1..n {
        s.push((b'0' + (i % 10) as u8) as char);
    }
    s
}

/// The fixed string corpus for row 18.
fn string_corpus() -> Vec<String> {
    let mut v: Vec<String> = [
        "", " ", "0", "-0", "+1", "1.", ".5", "1e5", "1E+5", "1e", "0x10", "0X1f", "0b101", "0o17",
        "017", "Infinity", "-Infinity", "+Infinity", "InfinityX", "NaN", "nan", "abc", "1abc",
        " 12 ", "\t\n 3", "1e999", "1e-999", "0.0000000000000000000001", "1_000", ".", "-", "e5",
        "  ", "+", "0x", "0X", "-0x10", "1.5e", "1.5e+", ".e5", "1.2.3", "12345678901234567890",
        "-.5", "+.5", "5.", "0.0", "-0.0", "1e+", "1e-", "9e307", "9e308", "-9e308", "1e-323",
        "1e-324",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.push(digits_string(400, b'1'));
    v.push(digits_string(400, b'9'));
    v
}

/// Random numeric-ish strings from a fixed seed.
fn random_strings(seed: u64, n: usize) -> Vec<String> {
    const POOL: &[u8] = b"0123456789+-..eEeExXbo0123456789 \tabcdefABCDEF_";
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let len = 1 + rng.range(12) as usize;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            s.push(POOL[rng.range(POOL.len() as u32) as usize] as char);
        }
        out.push(s);
    }
    out
}

fn slab(kind: &str, i: usize, s: &str) -> String {
    format!("{}[{} {:?}]", kind, i, s)
}

/* Some of the C entry points below abort() or run off the end of a string and
 * segfault (by design — see the comments at the call sites). The children are
 * expected to die; stop the OS from spending ~9s per crash writing a core dump
 * so the tests stay fast. Only affects this test binary's process tree, never
 * the compared stdout/stderr bytes. */
#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}
extern "C" {
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}
const RLIMIT_CORE: c_int = 4;

fn no_core_dumps() {
    unsafe {
        let z = RLimit { cur: 0, max: 0 };
        setrlimit(RLIMIT_CORE, &z);
    }
}

unsafe fn off(base: *const c_char, ep: *const c_char) -> c_int {
    if ep.is_null() {
        return -1;
    }
    (ep as isize - base as isize) as c_int
}

/* ================================================================= row 12 */

#[test]
fn cfg12_pushnumber_tostring() {
    let vals = all_values();
    diff("cfg12_pushnumber_tostring", |api| unsafe {
        let J = newstate(api, 0);
        for (i, &v) in vals.iter().enumerate() {
            (api.js_pushnumber)(J, v);
            let s = (api.js_tostring)(J, -1);
            p_str(&vlab("tostring", i, v), s);
            (api.js_pop)(J, 1);
        }
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ================================================================= row 15 */

#[test]
fn cfg15_stack_integer_conversions() {
    let vals = all_values();
    diff("cfg15_stack_integer_conversions", |api| unsafe {
        let J = newstate(api, 0);
        for (i, &v) in vals.iter().enumerate() {
            (api.js_pushnumber)(J, v);
            p_int(&vlab("tointeger", i, v), (api.js_tointeger)(J, -1));
            p_int(&vlab("toint32", i, v), (api.js_toint32)(J, -1));
            p_uint(&vlab("touint32", i, v), (api.js_touint32)(J, -1));
            p_int(&vlab("toint16", i, v), (api.js_toint16)(J, -1) as c_int);
            p_int(&vlab("touint16", i, v), (api.js_touint16)(J, -1) as c_int);
            p_num(&vlab("tonumber", i, v), (api.js_tonumber)(J, -1));
            (api.js_pop)(J, 1);
        }
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ================================================================= row 16 */

#[test]
fn cfg16_numbertointeger_direct() {
    let vals = all_values();
    diff("cfg16_numbertointeger_direct", |api| unsafe {
        for (i, &v) in vals.iter().enumerate() {
            p_int(&vlab("numbertointeger", i, v), (api.jsV_numbertointeger)(v));
            p_int(&vlab("numbertoint32", i, v), (api.jsV_numbertoint32)(v));
            p_uint(&vlab("numbertouint32", i, v), (api.jsV_numbertouint32)(v));
            p_int(&vlab("numbertoint16", i, v), (api.jsV_numbertoint16)(v) as c_int);
            p_int(&vlab("numbertouint16", i, v), (api.jsV_numbertouint16)(v) as c_int);
        }
    });
}

/* ================================================================= row 17 */

#[test]
fn cfg17_numbertostring() {
    let vals = all_values();
    diff("cfg17_numbertostring", |api| unsafe {
        let J = newstate(api, 0);
        for (i, &v) in vals.iter().enumerate() {
            /* C declares `char buf[32]`; give it plenty of slack. */
            let mut buf = [0 as c_char; 64];
            let s = (api.jsV_numbertostring)(J, buf.as_mut_ptr(), v);
            p_str(&vlab("numbertostring", i, v), s);
        }
        (api.js_freestate)(J);
    });
}

/* ================================================================= row 18 */

#[test]
fn cfg18_stringtonumber() {
    let mut strs = string_corpus();
    strs.extend(random_strings(SEED ^ 0x1111, 500));
    diff("cfg18_stringtonumber", |api| unsafe {
        let J = newstate(api, 0);
        for (i, s) in strs.iter().enumerate() {
            let c = cs(s);
            let p = c.as_ptr();

            p_num(&slab("stringtonumber", i, s), (api.jsV_stringtonumber)(J, p));

            let mut ep: *mut c_char = null_mut();
            let n = (api.js_stringtofloat)(p, &mut ep);
            p_num(&slab("stringtofloat", i, s), n);
            p_int(&slab("stringtofloat_off", i, s), off(p, ep));

            let mut ep2: *mut c_char = null_mut();
            let n2 = (api.js_strtod)(p, &mut ep2);
            p_num(&slab("strtod", i, s), n2);
            p_int(&slab("strtod_off", i, s), off(p, ep2));
        }
        (api.js_freestate)(J);
    });
}

/* ================================================================= row 19 */

fn strtol_corpus() -> Vec<String> {
    let mut v: Vec<String> = [
        "0", "1", "z", "Z", "7f", "0x1f", "-ff", "+10", "", " 10", "1010", "99999999999999999999999999",
        "777", "0b11", "gg", "ZZZZ", "abcdefghijklmnopqrstuvwxyz", "10.5", "-0", "+0",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.push(digits_string(100, b'1'));
    v
}

#[test]
fn cfg19_strtol_radix() {
    let mut strs = strtol_corpus();
    strs.extend(random_strings(SEED ^ 0x2222, 300));

    /* radix >= 81 makes the C loop accept the NUL terminator (table['\0'] ==
     * 80 < radix) and walk off the end of the string until it faults, so it is
     * exercised in its own child. */
    let radices: [c_int; 9] = [0, 1, 2, 8, 10, 16, 36, 37, -1];
    let strs_ref = &strs;
    diff("cfg19_strtol_radix", move |api| unsafe {
        for &radix in radices.iter() {
            for (i, s) in strs_ref.iter().enumerate() {
                let c = cs(s);
                let p = c.as_ptr();
                let mut ep: *mut c_char = null_mut();
                let n = (api.js_strtol)(p, &mut ep, radix);
                p_num(&format!("strtol[r={} {} {:?}]", radix, i, s), n);
                p_int(&format!("strtol_off[r={} {} {:?}]", radix, i, s), off(p, ep));
            }
        }
    });
}

#[test]
fn cfg19_strtol_radix99() {
    no_core_dumps();
    let strs = strtol_corpus();
    diff("cfg19_strtol_radix99", move |api| unsafe {
        for &radix in [99 as c_int].iter() {
            for (i, s) in strs.iter().enumerate() {
                let c = cs(s);
                let p = c.as_ptr();
                let mut ep: *mut c_char = null_mut();
                let n = (api.js_strtol)(p, &mut ep, radix);
                p_num(&format!("strtol[r={} {} {:?}]", radix, i, s), n);
                p_int(&format!("strtol_off[r={} {} {:?}]", radix, i, s), off(p, ep));
            }
        }
    });
}

/* ================================================================= row 20 */

/// grisu2(±0.0) trips `assert(x.f >= y.f)` in minus() (jsdtoa.c:387) and aborts
/// with SIGABRT; compared in its own test so the abort cannot swallow the
/// output of the rest of row 20.
#[test]
fn cfg20_grisu2_zero_aborts() {
    no_core_dumps();
    diff_stdout("cfg20_grisu2_zero", |api| unsafe {
        let mut buf = [0 as c_char; 64];
        let mut k: c_int = 0;
        let n = (api.js_grisu2)(0.0, buf.as_mut_ptr(), &mut k);
        p_int("grisu2_pos0_n", n);
        p_int("grisu2_pos0_k", k);
        p_str("grisu2_pos0_digits", buf.as_ptr());
    });
    diff_stdout("cfg20_grisu2_negzero", |api| unsafe {
        let mut buf = [0 as c_char; 64];
        let mut k: c_int = 0;
        let n = (api.js_grisu2)(-0.0, buf.as_mut_ptr(), &mut k);
        p_int("grisu2_neg0_n", n);
        p_int("grisu2_neg0_k", k);
        p_str("grisu2_neg0_digits", buf.as_ptr());
    });
}

#[test]
fn cfg20_grisu2_fmtexp_itoa() {
    let vals = all_values();
    let vals_ref = &vals;
    diff("cfg20_grisu2", move |api| unsafe {
        for (i, &v) in vals_ref.iter().enumerate() {
            if v == 0.0 {
                p_line(&format!("{} = skipped(zero)", vlab("grisu2", i, v)));
                continue;
            }
            let mut buf = [0 as c_char; 64];
            let mut k: c_int = 0;
            let n = (api.js_grisu2)(v, buf.as_mut_ptr(), &mut k);
            p_int(&vlab("grisu2_n", i, v), n);
            p_int(&vlab("grisu2_K", i, v), k);
            p_str(&vlab("grisu2_digits", i, v), buf.as_ptr());
        }
    });

    diff("cfg20_fmtexp", |api| unsafe {
        for e in -400..=400i32 {
            let mut buf = [0 as c_char; 64];
            (api.js_fmtexp)(buf.as_mut_ptr(), e as c_int);
            p_str(&format!("fmtexp[{}]", e), buf.as_ptr());
        }
    });

    let mut ints: Vec<i32> = vec![
        i32::MIN,
        i32::MAX,
        0,
        -1,
        1,
        10,
        99,
        100,
        -99999,
        i32::MIN + 1,
        i32::MAX - 1,
        -100,
        1000000000,
        -1000000000,
        7,
        -7,
    ];
    let mut rng = Rng::new(SEED ^ 0x3333);
    for _ in 0..500 {
        ints.push(rng.next_u32() as i32);
    }
    let ints_ref = &ints;
    diff("cfg20_itoa", move |api| unsafe {
        for (i, &a) in ints_ref.iter().enumerate() {
            let mut buf = [0 as c_char; 64];
            let s = (api.js_itoa)(buf.as_mut_ptr(), a as c_int);
            p_str(&format!("itoa[{} a={}]", i, a), s);
        }
    });
}


