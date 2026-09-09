//! Differential tests for the value / stack / coercion surface of MuJS.
//!
//! CONFIGS.md rows 11, 13, 14, 21, 22, 23, 24, 25, 26, 27, 28, 85 (partial),
//! 86, 87, 88.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/* ------------------------------------------------------------------ helpers */

static NULLSTR: &[u8] = b"<null>\0";
static ERRSTR: &[u8] = b"<ERR>\0";
static LIT: &[u8] = b"literal-string-value\0";
static LONGSTR: &[u8] = b"the quick brown fox jumps over the lazy dog 0123456789\0";
static UTF8STR: &[u8] = b"h\xc3\xa9ll\xe2\x82\xac \xf0\x9d\x84\x9e\0";
/* pointers handed to js_pushliteral / js_newcfunction are stored by the
 * library, so they must outlive the state: use statics, never temporaries. */
static LIT_EMPTY: &[u8] = b"\0";
static LIT_ABC: &[u8] = b"abc\0";
static CFUN_NAME: &[u8] = b"cfun\0";

unsafe fn nn(s: *const c_char) -> *const c_char {
    if s.is_null() {
        NULLSTR.as_ptr() as *const c_char
    } else {
        s
    }
}

unsafe fn err() -> *const c_char {
    ERRSTR.as_ptr() as *const c_char
}

unsafe fn pk_i(k: &str, p: &str, v: c_int) {
    printf(cs("%s.%s=%d\n").as_ptr(), cs(k).as_ptr(), cs(p).as_ptr(), v);
}

unsafe fn pk_s(k: &str, p: &str, s: *const c_char) {
    printf(cs("%s.%s=<%s>\n").as_ptr(), cs(k).as_ptr(), cs(p).as_ptr(), nn(s));
}

unsafe fn pk_n(k: &str, p: &str, v: f64) {
    printf(cs("%s.%s=%.17g\n").as_ptr(), cs(k).as_ptr(), cs(p).as_ptr(), v);
}

unsafe fn pi_i(k: &str, i: c_int, v: c_int) {
    printf(cs("%s[%d]=%d\n").as_ptr(), cs(k).as_ptr(), i, v);
}

unsafe fn pi_s(k: &str, i: c_int, s: *const c_char) {
    printf(cs("%s[%d]=<%s>\n").as_ptr(), cs(k).as_ptr(), i, nn(s));
}

unsafe fn hdr(tag: &str) {
    printf(cs("== %s\n").as_ptr(), cs(tag).as_ptr());
}

unsafe fn hdr1(tag: &str, a: &str) {
    printf(cs("== %s %s\n").as_ptr(), cs(tag).as_ptr(), cs(a).as_ptr());
}

fn sgn(v: c_int) -> c_int {
    if v < 0 {
        -1
    } else if v > 0 {
        1
    } else {
        0
    }
}

/// internal js_Type tag of the value at `idx` (0=shrstr 1=undef 2=null
/// 3=bool 4=number 5=litstr 6=memstr 7=object)
unsafe fn itype(api: &Api, J: JS, idx: c_int) -> c_int {
    let v = (api.js_tovalue)(J, idx);
    if v.is_null() {
        return -1;
    }
    (*v).t.type_ as c_int
}

static mut G_PUSHUNDEF: Option<unsafe extern "C" fn(JS)> = None;

unsafe extern "C" fn cfun_noop(J: JS) {
    if let Some(f) = G_PUSHUNDEF {
        f(J);
    }
}

static SETUP: &[u8] = b"
function fn(a,b){return a+b}
var arr=[1,2,3];
var obj={a:1,b:'two'};
var d=new Date(0);
var ov={valueOf:function(){return 42}};
var ots={toString:function(){return 'ts'}};
var oboth={valueOf:function(){return 7},toString:function(){return 'seven'}};
var othrow={valueOf:function(){throw new Error('vo boom')},toString:function(){throw new Error('ts boom')}};
var cyc={n:1}; cyc.self=cyc;
var nested={a:[1,[2,3]],b:{c:{d:4}},e:'s'};
var sparse=[1,2,3]; sparse[7]=8;
var gthrow={};
Object.defineProperty(gthrow,'x',{get:function(){throw new Error('get boom')},enumerable:true});
var nullproto=Object.create(null);
function C(){}
function D(){}
function B(){}
B.prototype=new C();
function E(){}
E.prototype=5;
var cinst=new C();
var binst=new B();
\0";

unsafe fn gg(api: &Api, J: JS, name: &str) {
    (api.js_getglobal)(J, cs(name).as_ptr());
}

unsafe fn setup(api: &Api, J: JS) {
    G_PUSHUNDEF = Some(api.js_pushundefined);
    let rc = (api.js_dostring)(J, SETUP.as_ptr() as *const c_char);
    p_int("setup_rc", rc);
    p_int("setup_top", (api.js_gettop)(J));
}

/* The value-kind table. `KINDS[k]` is the name of the value pushed by
 * `push_kind(api, J, k)`. */
static KINDS: &[&str] = &[
    "undefined",    // 0
    "null",         // 1
    "false",        // 2
    "true",         // 3
    "num_p0",       // 4
    "num_n0",       // 5
    "num_42_5",     // 6
    "num_nan",      // 7
    "num_inf",      // 8
    "num_1",        // 9
    "shrstr_empty", // 10
    "shrstr_abc",   // 11
    "shrstr_1",     // 12
    "litstr",       // 13
    "memstr_long",  // 14
    "memstr_utf8",  // 15
    "object",       // 16
    "array",        // 17
    "function",     // 18
    "cfunction",    // 19
    "stringobj",    // 20
    "numberobj",    // 21
    "booleanobj",   // 22
    "dateobj",      // 23
    "regexp",       // 24
    "errorobj",     // 25
    "obj_valueOf",  // 26
    "obj_toString", // 27
    "obj_both",     // 28
    "math",         // 29
    "json",         // 30
    "obj_throw",    // 31
    "obj_getthrow", // 32
    "nullproto",    // 33
];

/// number of kinds whose coercion never throws (0..NSAFE)
const NSAFE: c_int = 31;

unsafe fn push_kind(api: &Api, J: JS, k: c_int) {
    match k {
        0 => (api.js_pushundefined)(J),
        1 => (api.js_pushnull)(J),
        2 => (api.js_pushboolean)(J, 0),
        3 => (api.js_pushboolean)(J, 1),
        4 => (api.js_pushnumber)(J, 0.0),
        5 => (api.js_pushnumber)(J, -0.0),
        6 => (api.js_pushnumber)(J, 42.5),
        7 => (api.js_pushnumber)(J, f64::NAN),
        8 => (api.js_pushnumber)(J, f64::INFINITY),
        9 => (api.js_pushnumber)(J, 1.0),
        10 => (api.js_pushstring)(J, cs("").as_ptr()),
        11 => (api.js_pushstring)(J, cs("abc").as_ptr()),
        12 => (api.js_pushstring)(J, cs("1").as_ptr()),
        13 => (api.js_pushliteral)(J, LIT.as_ptr() as *const c_char),
        14 => (api.js_pushstring)(J, LONGSTR.as_ptr() as *const c_char),
        15 => (api.js_pushstring)(J, UTF8STR.as_ptr() as *const c_char),
        16 => gg(api, J, "obj"),
        17 => gg(api, J, "arr"),
        18 => gg(api, J, "fn"),
        19 => (api.js_newcfunction)(J, Some(cfun_noop), CFUN_NAME.as_ptr() as *const c_char, 0),
        20 => (api.js_newstring)(J, cs("xy").as_ptr()),
        21 => (api.js_newnumber)(J, 7.0),
        22 => (api.js_newboolean)(J, 1),
        23 => gg(api, J, "d"),
        24 => (api.js_newregexp)(J, cs("a+").as_ptr(), JS_REGEXP_G | JS_REGEXP_I),
        25 => (api.js_newerror)(J, cs("boom").as_ptr()),
        26 => gg(api, J, "ov"),
        27 => gg(api, J, "ots"),
        28 => gg(api, J, "oboth"),
        29 => gg(api, J, "Math"),
        30 => gg(api, J, "JSON"),
        31 => gg(api, J, "othrow"),
        32 => gg(api, J, "gthrow"),
        33 => gg(api, J, "nullproto"),
        _ => (api.js_pushundefined)(J),
    }
}

unsafe fn dumpstack(api: &Api, J: JS, tag: &str) {
    let top = (api.js_gettop)(J);
    printf(cs("%s: top=%d\n").as_ptr(), cs(tag).as_ptr(), top);
    for i in 0..top {
        let t = (api.js_type)(J, i);
        let tn = (api.js_typeof)(J, i);
        let it = itype(api, J, i);
        (api.js_copy)(J, i);
        let s = (api.js_trystring)(J, -1, err());
        printf(
            cs("  [%d] itype=%d type=%d typeof=%s str=<%s>\n").as_ptr(),
            i,
            it,
            t,
            nn(tn),
            nn(s),
        );
        (api.js_pop)(J, 1);
    }
}

unsafe fn dump_bytes(tag: &str, i: c_int, s: *const c_char, n: c_int) {
    for j in 0..n {
        printf(
            cs("%s[%d].byte[%d]=%d\n").as_ptr(),
            cs(tag).as_ptr(),
            i,
            j,
            *s.add(j as usize) as c_int,
        );
    }
}

/// full report about the string value on the top of the stack
unsafe fn report_str(api: &Api, J: JS, tag: &str, i: c_int) {
    let it = itype(api, J, -1);
    let isstr = (api.js_isstring)(J, -1);
    let ty = (api.js_type)(J, -1);
    let s = (api.js_tostring)(J, -1);
    let ln = if s.is_null() { -1 } else { strlen(s) as c_int };
    let ul = if s.is_null() { -1 } else { (api.js_utflen)(s) };
    printf(
        cs("%s[%d] itype=%d isstring=%d type=%d strlen=%d utflen=%d s=<%s>\n").as_ptr(),
        cs(tag).as_ptr(),
        i,
        it,
        isstr,
        ty,
        ln,
        ul,
        nn(s),
    );
}

unsafe fn rand_str(rng: &mut Rng, buf: &mut [c_char; 96]) -> c_int {
    static ALPHA: &[u8] = b"abcABZ019 \t.-_/\xc3\xa9\xe2\x82\xac\xff\x80\x7fz";
    let n = rng.range(28) as usize;
    for i in 0..n {
        buf[i] = ALPHA[rng.range(ALPHA.len() as u32) as usize] as c_char;
    }
    buf[n] = 0;
    n as c_int
}

/* ============================================================== row 11 ==== */

#[test]
fn cfg11_kinds_type_typeof_predicates() {
    diff("cfg11", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        let n = KINDS.len() as c_int;
        for k in 0..n {
            push_kind(api, J, k);
        }
        p_int("top", (api.js_gettop)(J));

        let preds: [(&str, unsafe extern "C" fn(JS, c_int) -> c_int); 17] = [
            ("isdefined", api.js_isdefined),
            ("isundefined", api.js_isundefined),
            ("isnull", api.js_isnull),
            ("isboolean", api.js_isboolean),
            ("isnumber", api.js_isnumber),
            ("isstring", api.js_isstring),
            ("isprimitive", api.js_isprimitive),
            ("isobject", api.js_isobject),
            ("isarray", api.js_isarray),
            ("isregexp", api.js_isregexp),
            ("iscoercible", api.js_iscoercible),
            ("iscallable", api.js_iscallable),
            ("iserror", api.js_iserror),
            ("isnumberobject", api.js_isnumberobject),
            ("isstringobject", api.js_isstringobject),
            ("isbooleanobject", api.js_isbooleanobject),
            ("isdateobject", api.js_isdateobject),
        ];

        for k in 0..n {
            let name = KINDS[k as usize];
            hdr1("kind", name);
            pk_i(name, "idx", k);
            pk_i(name, "itype", itype(api, J, k));
            pk_i(name, "type", (api.js_type)(J, k));
            pk_s(name, "typeof", (api.js_typeof)(J, k));
            for (pn, f) in preds.iter() {
                pk_i(name, pn, f(J, k));
            }
            pk_i(name, "isuserdata_tag", (api.js_isuserdata)(J, k, cs("tag").as_ptr()));
            /* same slot addressed with a negative index */
            pk_i(name, "neg_type", (api.js_type)(J, k - n));
            pk_s(name, "neg_typeof", (api.js_typeof)(J, k - n));
        }

        /* out-of-range indices all read the shared `undefined` slot */
        hdr("oob");
        pk_i("oob", "type_99", (api.js_type)(J, 99));
        pk_s("oob", "typeof_99", (api.js_typeof)(J, 99));
        pk_i("oob", "itype_99", itype(api, J, 99));
        pk_i("oob", "type_neg99", (api.js_type)(J, -99));
        pk_s("oob", "typeof_neg99", (api.js_typeof)(J, -99));
        pk_i("oob", "isdefined_99", (api.js_isdefined)(J, 99));
        pk_i("oob", "iscoercible_99", (api.js_iscoercible)(J, 99));

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 13 ==== */

#[test]
fn cfg13_push_strings() {
    diff("cfg13", |api| unsafe {
        let J = newstate(api, 0);

        hdr("pushstring");
        let cases: [&str; 12] = [
            "",
            "a",
            "abcdefgh",
            "abcdefghi",
            "123456789012345",
            "1234567890123456",
            "12345678901234567890",
            "the quick brown fox jumps over the lazy dog",
            "h\u{e9}ll\u{20ac}",
            "\u{1d11e}\u{1d11e}\u{1d11e}",
            "tab\there\nnl",
            "\u{7f}\u{80}\u{7ff}\u{800}",
        ];
        for (i, s) in cases.iter().enumerate() {
            let c = cs(s);
            (api.js_pushstring)(J, c.as_ptr());
            report_str(api, J, "pushstring", i as c_int);
            (api.js_pop)(J, 1);
        }

        hdr("pushliteral");
        (api.js_pushliteral)(J, LIT.as_ptr() as *const c_char);
        report_str(api, J, "pushliteral", 0);
        pi_i("lit_isstring", 0, (api.js_isstring)(J, -1));
        (api.js_pop)(J, 1);
        (api.js_pushliteral)(J, LIT_EMPTY.as_ptr() as *const c_char);
        report_str(api, J, "pushliteral", 1);
        (api.js_pop)(J, 1);
        (api.js_pushliteral)(J, UTF8STR.as_ptr() as *const c_char);
        report_str(api, J, "pushliteral", 2);
        (api.js_pop)(J, 1);

        hdr("pushlstring");
        /* (bytes, n) pairs: embedded NUL, n=0, truncation mid UTF-8, boundary */
        let raw: [(&[u8], c_int); 14] = [
            (b"abc", 3),
            (b"abc", 0),
            (b"abc", 1),
            (b"a\0b", 3),
            (b"a\0b\0c", 5),
            (b"\xc3\xa9", 2),
            (b"\xc3\xa9", 1),
            (b"\xe2\x82\xac", 3),
            (b"\xe2\x82\xac", 2),
            (b"\xe2\x82\xac", 1),
            (b"0123456789abcde", 15),
            (b"0123456789abcdef", 16),
            (b"0123456789abcdefg", 17),
            (b"h\xc3\xa9ll\xe2\x82\xac \xf0\x9d\x84\x9e", 13),
        ];
        for (i, (b, n)) in raw.iter().enumerate() {
            (api.js_pushlstring)(J, b.as_ptr() as *const c_char, *n);
            report_str(api, J, "pushlstring", i as c_int);
            let s = (api.js_tostring)(J, -1);
            dump_bytes("pushlstring", i as c_int, s, *n + 1);
            (api.js_pop)(J, 1);
        }

        hdr("random");
        let mut rng = Rng::new(0x13_1313);
        let mut buf = [0 as c_char; 96];
        for i in 0..60 {
            let n = rand_str(&mut rng, &mut buf);
            (api.js_pushstring)(J, buf.as_ptr());
            report_str(api, J, "rnd_pushstring", i);
            (api.js_pop)(J, 1);
            (api.js_pushlstring)(J, buf.as_ptr(), n);
            report_str(api, J, "rnd_pushlstring_n", i);
            (api.js_pop)(J, 1);
            (api.js_pushlstring)(J, buf.as_ptr(), n / 2);
            report_str(api, J, "rnd_pushlstring_half", i);
            let s = (api.js_tostring)(J, -1);
            dump_bytes("rnd_half", i, s, n / 2 + 1);
            (api.js_pop)(J, 1);
        }

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 14 ==== */

#[test]
fn cfg14_toboolean_tonumber_tostring() {
    diff("cfg14", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            hdr1("cfg14", name);

            push_kind(api, J, k);
            let before = itype(api, J, -1);
            let b = (api.js_toboolean)(J, -1);
            pk_i(name, "toboolean", b);
            pk_i(name, "itype_before", before);
            pk_i(name, "itype_after_bool", itype(api, J, -1));
            (api.js_pop)(J, 1);

            push_kind(api, J, k);
            let num = (api.js_tonumber)(J, -1);
            pk_n(name, "tonumber", num);
            pk_i(name, "itype_after_num", itype(api, J, -1));
            (api.js_pop)(J, 1);

            push_kind(api, J, k);
            let s = (api.js_tostring)(J, -1);
            pk_s(name, "tostring", s);
            pk_i(name, "tostring_len", if s.is_null() { -1 } else { strlen(s) as c_int });
            pk_i(name, "itype_after_str", itype(api, J, -1));
            (api.js_pop)(J, 1);

            /* integer conversions on the same kind for good measure */
            push_kind(api, J, k);
            pk_i(name, "tointeger", (api.js_tointeger)(J, -1));
            (api.js_pop)(J, 1);
        }

        hdr("cfg14_oob");
        pk_i("oob", "toboolean", (api.js_toboolean)(J, 77));
        pk_n("oob", "tonumber", (api.js_tonumber)(J, 77));
        pk_s("oob", "tostring", (api.js_tostring)(J, 77));

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 21 ==== */

#[test]
fn cfg21_try_conversions() {
    diff("cfg21", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        let kinds: [c_int; 16] = [
            0, 1, 2, 3, 6, 7, 8, 11, 12, 13, 16, 17, 26, 27, 31, 32,
        ];
        for k in kinds.iter() {
            let name = KINDS[*k as usize];
            hdr1("cfg21", name);

            push_kind(api, J, *k);
            pk_s(name, "trystring", (api.js_trystring)(J, -1, err()));
            pk_i(name, "top_after_trystring", (api.js_gettop)(J));
            (api.js_pop)(J, 1);

            push_kind(api, J, *k);
            pk_n(name, "trynumber", (api.js_trynumber)(J, -1, -12345.5));
            pk_i(name, "top_after_trynumber", (api.js_gettop)(J));
            (api.js_pop)(J, 1);

            push_kind(api, J, *k);
            pk_i(name, "tryinteger", (api.js_tryinteger)(J, -1, -999));
            (api.js_pop)(J, 1);

            push_kind(api, J, *k);
            pk_i(name, "tryboolean", (api.js_tryboolean)(J, -1, -1));
            (api.js_pop)(J, 1);

            push_kind(api, J, *k);
            pk_s(name, "tryrepr", (api.js_tryrepr)(J, -1, err()));
            pk_i(name, "itype_after_tryrepr", itype(api, J, -1));
            pk_i(name, "top_after_tryrepr", (api.js_gettop)(J));
            (api.js_pop)(J, 1);
        }

        hdr("cfg21_numbers");
        let nums: [f64; 8] = [3.9, -3.9, 0.0, -0.0, f64::NAN, f64::INFINITY, 2147483647.0, -2147483648.0];
        for (i, x) in nums.iter().enumerate() {
            (api.js_pushnumber)(J, *x);
            pi_i("tryinteger_num", i as c_int, (api.js_tryinteger)(J, -1, -999));
            pi_i("tryboolean_num", i as c_int, (api.js_tryboolean)(J, -1, -1));
            pi_s("trystring_num", i as c_int, (api.js_trystring)(J, -1, err()));
            (api.js_pop)(J, 1);
        }

        hdr("cfg21_strings");
        let strs: [&str; 7] = ["42", " 42 ", "0x10", "", "abc", "Infinity", "1e3"];
        for (i, s) in strs.iter().enumerate() {
            let c = cs(s);
            (api.js_pushstring)(J, c.as_ptr());
            pi_i("tryinteger_str", i as c_int, (api.js_tryinteger)(J, -1, -999));
            printf(
                cs("trynumber_str[%d]=%.17g\n").as_ptr(),
                i as c_int,
                (api.js_trynumber)(J, -1, -12345.5),
            );
            pi_i("tryboolean_str", i as c_int, (api.js_tryboolean)(J, -1, -1));
            pi_s("tryrepr_str", i as c_int, (api.js_tryrepr)(J, -1, err()));
            (api.js_pop)(J, 1);
        }

        hdr("cfg21_oob");
        pk_s("oob", "trystring", (api.js_trystring)(J, 55, err()));
        pk_n("oob", "trynumber", (api.js_trynumber)(J, 55, -12345.5));
        pk_i("oob", "tryinteger", (api.js_tryinteger)(J, 55, -999));
        pk_i("oob", "tryboolean", (api.js_tryboolean)(J, 55, -1));

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 22 ==== */

unsafe fn do_compare(api: &Api, J: JS, tag: &str, i: c_int) {
    let mut okay: c_int = -7;
    let rv = (api.js_compare)(J, &mut okay);
    printf(
        cs("cmp %s[%d] rv=%d sign=%d okay=%d itype_a=%d itype_b=%d\n").as_ptr(),
        cs(tag).as_ptr(),
        i,
        rv,
        sgn(rv),
        okay,
        itype(api, J, -2),
        itype(api, J, -1),
    );
    (api.js_pop)(J, 2);
}

#[test]
fn cfg22_compare() {
    diff("cfg22", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        hdr("cfg22_kinds");
        let kinds: [c_int; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 16, 26];
        let mut c: c_int = 0;
        for a in kinds.iter() {
            for b in kinds.iter() {
                printf(
                    cs("pair %s / %s\n").as_ptr(),
                    cs(KINDS[*a as usize]).as_ptr(),
                    cs(KINDS[*b as usize]).as_ptr(),
                );
                push_kind(api, J, *a);
                push_kind(api, J, *b);
                do_compare(api, J, "kinds", c);
                c += 1;
            }
        }

        hdr("cfg22_strings");
        let strs: [&str; 8] = ["a", "b", "ab", "abc", "", "A", "abd", "\u{e9}"];
        let mut c: c_int = 0;
        for a in strs.iter() {
            for b in strs.iter() {
                let ca = cs(a);
                let cb = cs(b);
                (api.js_pushstring)(J, ca.as_ptr());
                (api.js_pushstring)(J, cb.as_ptr());
                do_compare(api, J, "strings", c);
                c += 1;
            }
        }

        hdr("cfg22_objects");
        (api.js_newobject)(J);
        (api.js_newobject)(J);
        do_compare(api, J, "obj_obj", 0);
        gg(api, J, "ov");
        (api.js_pushnumber)(J, 7.0);
        do_compare(api, J, "ov_7", 1);
        gg(api, J, "ots");
        (api.js_pushstring)(J, cs("ts").as_ptr());
        do_compare(api, J, "ots_ts", 2);
        gg(api, J, "arr");
        gg(api, J, "arr");
        do_compare(api, J, "arr_arr", 3);
        gg(api, J, "d");
        (api.js_pushnumber)(J, 0.0);
        do_compare(api, J, "date_0", 4);

        hdr("cfg22_rand_numbers");
        let mut rng = Rng::new(0x22_2222);
        for i in 0..80 {
            let x = if i % 3 == 0 { rng.bits_f64() } else { rng.nice_f64() };
            let y = if i % 4 == 0 { rng.bits_f64() } else { rng.nice_f64() };
            printf(cs("rnum[%d] x=%.17g y=%.17g\n").as_ptr(), i as c_int, x, y);
            (api.js_pushnumber)(J, x);
            (api.js_pushnumber)(J, y);
            do_compare(api, J, "rnum", i as c_int);
        }

        hdr("cfg22_rand_strings");
        let mut buf = [0 as c_char; 96];
        let mut buf2 = [0 as c_char; 96];
        for i in 0..60 {
            rand_str(&mut rng, &mut buf);
            rand_str(&mut rng, &mut buf2);
            printf(
                cs("rstr[%d] a=<%s> b=<%s>\n").as_ptr(),
                i as c_int,
                buf.as_ptr(),
                buf2.as_ptr(),
            );
            (api.js_pushstring)(J, buf.as_ptr());
            (api.js_pushstring)(J, buf2.as_ptr());
            do_compare(api, J, "rstr", i as c_int);
        }

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 23 ==== */

#[test]
fn cfg23_equal_strictequal() {
    diff("cfg23", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        let kinds: [c_int; 20] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 20, 21, 26,
        ];
        for a in kinds.iter() {
            for b in kinds.iter() {
                let na = KINDS[*a as usize];
                let nb = KINDS[*b as usize];
                push_kind(api, J, *a);
                push_kind(api, J, *b);
                let eq = (api.js_equal)(J);
                (api.js_pop)(J, 2);
                push_kind(api, J, *a);
                push_kind(api, J, *b);
                let se = (api.js_strictequal)(J);
                (api.js_pop)(J, 2);
                printf(
                    cs("eq %s %s equal=%d strict=%d\n").as_ptr(),
                    cs(na).as_ptr(),
                    cs(nb).as_ptr(),
                    eq,
                    se,
                );
            }
        }

        hdr("cfg23_identity");
        gg(api, J, "obj");
        gg(api, J, "obj");
        pk_i("same_obj", "equal", (api.js_equal)(J));
        (api.js_pop)(J, 2);
        gg(api, J, "obj");
        gg(api, J, "obj");
        pk_i("same_obj", "strict", (api.js_strictequal)(J));
        (api.js_pop)(J, 2);
        (api.js_newobject)(J);
        (api.js_newobject)(J);
        pk_i("new_objs", "equal", (api.js_equal)(J));
        (api.js_pop)(J, 2);
        (api.js_newobject)(J);
        (api.js_newobject)(J);
        pk_i("new_objs", "strict", (api.js_strictequal)(J));
        (api.js_pop)(J, 2);

        hdr("cfg23_special");
        /* NaN vs NaN, +0 vs -0, "1" vs 1, null vs undefined, litstr vs shrstr */
        let pairs: [(c_int, c_int); 10] = [
            (7, 7),
            (4, 5),
            (5, 4),
            (12, 9),
            (9, 12),
            (1, 0),
            (0, 1),
            (13, 13),
            (11, 11),
            (14, 14),
        ];
        for (i, (a, b)) in pairs.iter().enumerate() {
            push_kind(api, J, *a);
            push_kind(api, J, *b);
            pi_i("sp_equal", i as c_int, (api.js_equal)(J));
            (api.js_pop)(J, 2);
            push_kind(api, J, *a);
            push_kind(api, J, *b);
            pi_i("sp_strict", i as c_int, (api.js_strictequal)(J));
            (api.js_pop)(J, 2);
        }

        hdr("cfg23_lit_vs_shr");
        (api.js_pushliteral)(J, LIT_ABC.as_ptr() as *const c_char);
        (api.js_pushstring)(J, cs("abc").as_ptr());
        pk_i("lit_shr", "itype_a", itype(api, J, -2));
        pk_i("lit_shr", "itype_b", itype(api, J, -1));
        pk_i("lit_shr", "equal", (api.js_equal)(J));
        (api.js_pop)(J, 2);
        (api.js_pushliteral)(J, LIT_ABC.as_ptr() as *const c_char);
        (api.js_pushstring)(J, cs("abc").as_ptr());
        pk_i("lit_shr", "strict", (api.js_strictequal)(J));
        (api.js_pop)(J, 2);

        hdr("cfg23_rand");
        let mut rng = Rng::new(0x23_2323);
        for i in 0..50 {
            let x = rng.nice_f64();
            (api.js_pushnumber)(J, x);
            (api.js_pushnumber)(J, x);
            printf(cs("rnd[%d] x=%.17g\n").as_ptr(), i as c_int, x);
            pi_i("rnd_equal", i, (api.js_equal)(J));
            (api.js_pop)(J, 2);
            (api.js_pushnumber)(J, x);
            (api.js_pushnumber)(J, x);
            pi_i("rnd_strict", i, (api.js_strictequal)(J));
            (api.js_pop)(J, 2);
            /* number vs its own string form */
            let mut nb = [0 as c_char; 40];
            let ns = (api.jsV_numbertostring)(J, nb.as_mut_ptr(), x);
            pi_s("rnd_numstr", i, ns);
            (api.js_pushnumber)(J, x);
            (api.js_pushstring)(J, ns);
            pi_i("rnd_num_vs_str_equal", i, (api.js_equal)(J));
            (api.js_pop)(J, 2);
            (api.js_pushnumber)(J, x);
            (api.js_pushstring)(J, ns);
            pi_i("rnd_num_vs_str_strict", i, (api.js_strictequal)(J));
            (api.js_pop)(J, 2);
        }

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 24 ==== */

#[test]
fn cfg24_instanceof() {
    diff("cfg24_api", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        /* value at -2, constructor at -1 */
        let pairs: [(&str, &str); 12] = [
            ("cinst", "C"),
            ("cinst", "D"),
            ("cinst", "B"),
            ("binst", "C"),
            ("binst", "B"),
            ("binst", "D"),
            ("obj", "C"),
            ("arr", "Array"),
            ("arr", "Object"),
            ("d", "Date"),
            ("d", "Array"),
            ("fn", "Function"),
        ];
        for (i, (v, c)) in pairs.iter().enumerate() {
            gg(api, J, v);
            gg(api, J, c);
            let r = (api.js_instanceof)(J);
            printf(
                cs("instanceof[%d] %s instanceof %s = %d top=%d\n").as_ptr(),
                i as c_int,
                cs(v).as_ptr(),
                cs(c).as_ptr(),
                r,
                (api.js_gettop)(J),
            );
            (api.js_pop)(J, 2);
        }

        /* primitive left-hand side: returns 0 without throwing */
        hdr("primitive_lhs");
        (api.js_pushnumber)(J, 5.0);
        gg(api, J, "C");
        pk_i("num_C", "instanceof", (api.js_instanceof)(J));
        (api.js_pop)(J, 2);
        (api.js_pushundefined)(J);
        gg(api, J, "C");
        pk_i("undef_C", "instanceof", (api.js_instanceof)(J));
        (api.js_pop)(J, 2);
        (api.js_pushstring)(J, cs("abc").as_ptr());
        gg(api, J, "String");
        pk_i("str_String", "instanceof", (api.js_instanceof)(J));
        (api.js_pop)(J, 2);

        /* throwing cases through the script engine (caught).  `print` is not
         * installed by js_newstate, so results are collected in a global. */
        hdr("script");
        let rc = (api.js_dostring)(
            J,
            cs("var res=[];\n\
                function T(f){ try { res.push(String(f())) } catch (e) { res.push(e.name+': '+e.message) } }\n\
                T(function(){ return cinst instanceof C });\n\
                T(function(){ return cinst instanceof D });\n\
                T(function(){ return binst instanceof C });\n\
                T(function(){ return ({}) instanceof {} });\n\
                T(function(){ return ({}) instanceof E });\n\
                T(function(){ return 5 instanceof 5 });\n\
                T(function(){ return ({}) instanceof null });\n\
                T(function(){ return ({}) instanceof undefined });\n\
                T(function(){ return ({}) instanceof 'str' });\n\
                T(function(){ return arr instanceof Array });\n\
                T(function(){ return 'lit' instanceof String });\n\
                T(function(){ return new C() instanceof B });\n")
                .as_ptr(),
        );
        p_int("script_rc", rc);
        gg(api, J, "res");
        let n = (api.js_getlength)(J, -1);
        p_int("script_res_len", n);
        for i in 0..n {
            (api.js_getindex)(J, -1, i);
            pi_s("script_res", i, (api.js_trystring)(J, -1, err()));
            (api.js_pop)(J, 1);
        }
        (api.js_pop)(J, 1);

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* non-callable right-hand side through the raw API: both libraries must
     * abort in the same way (uncaught TypeError -> panic -> abort) */
    diff("cfg24_noncallable_abort", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);
        gg(api, J, "obj");
        gg(api, J, "obj");
        p_line("about to js_instanceof with non-callable rhs");
        fflush(std::ptr::null_mut());
        let r = (api.js_instanceof)(J);
        p_int("unreached_instanceof", r);
        (api.js_freestate)(J);
    });

    /* callable rhs whose .prototype is not an object */
    diff("cfg24_badproto_abort", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);
        gg(api, J, "obj");
        gg(api, J, "E");
        p_line("about to js_instanceof with non-object prototype");
        fflush(std::ptr::null_mut());
        let r = (api.js_instanceof)(J);
        p_int("unreached_instanceof", r);
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 25 ==== */

unsafe fn do_concat(api: &Api, J: JS, tag: &str, i: c_int) {
    (api.js_concat)(J);
    let it = itype(api, J, -1);
    let ty = (api.js_type)(J, -1);
    let s = (api.js_trystring)(J, -1, err());
    printf(
        cs("concat %s[%d] itype=%d type=%d len=%d s=<%s>\n").as_ptr(),
        cs(tag).as_ptr(),
        i,
        it,
        ty,
        if s.is_null() { -1 } else { strlen(s) as c_int },
        nn(s),
    );
    (api.js_pop)(J, 1);
}

#[test]
fn cfg25_concat() {
    diff("cfg25", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        hdr("cfg25_kinds");
        let kinds: [c_int; 16] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 13, 14, 16, 17, 26,
        ];
        let mut c: c_int = 0;
        for a in kinds.iter() {
            for b in kinds.iter() {
                printf(
                    cs("cat %s + %s\n").as_ptr(),
                    cs(KINDS[*a as usize]).as_ptr(),
                    cs(KINDS[*b as usize]).as_ptr(),
                );
                push_kind(api, J, *a);
                push_kind(api, J, *b);
                do_concat(api, J, "kinds", c);
                c += 1;
            }
        }

        hdr("cfg25_strings");
        let pairs: [(&str, &str); 8] = [
            ("", ""),
            ("a", ""),
            ("", "b"),
            ("ab", "cd"),
            ("12345678", "90123456"),
            (
                "the quick brown fox jumps over the lazy dog",
                "0123456789 0123456789 0123456789",
            ),
            ("h\u{e9}", "ll\u{20ac}"),
            ("x", "\u{1d11e}"),
        ];
        for (i, (a, b)) in pairs.iter().enumerate() {
            let ca = cs(a);
            let cb = cs(b);
            (api.js_pushstring)(J, ca.as_ptr());
            (api.js_pushstring)(J, cb.as_ptr());
            do_concat(api, J, "strings", i as c_int);
        }

        hdr("cfg25_objects");
        (api.js_newobject)(J);
        (api.js_newobject)(J);
        do_concat(api, J, "obj_obj", 0);
        (api.js_pushstring)(J, cs("s:").as_ptr());
        (api.js_newobject)(J);
        do_concat(api, J, "str_obj", 1);
        gg(api, J, "ov");
        gg(api, J, "ov");
        do_concat(api, J, "ov_ov", 2);
        gg(api, J, "ots");
        gg(api, J, "ots");
        do_concat(api, J, "ots_ots", 3);
        gg(api, J, "oboth");
        gg(api, J, "oboth");
        do_concat(api, J, "oboth_oboth", 4);
        gg(api, J, "arr");
        gg(api, J, "arr");
        do_concat(api, J, "arr_arr", 5);
        gg(api, J, "d");
        (api.js_pushstring)(J, cs("!").as_ptr());
        do_concat(api, J, "date_str", 6);

        hdr("cfg25_rand");
        let mut rng = Rng::new(0x25_2525);
        let mut buf = [0 as c_char; 96];
        let mut buf2 = [0 as c_char; 96];
        for i in 0..50 {
            let x = rng.nice_f64();
            let y = rng.nice_f64();
            printf(cs("rnum[%d] x=%.17g y=%.17g\n").as_ptr(), i as c_int, x, y);
            (api.js_pushnumber)(J, x);
            (api.js_pushnumber)(J, y);
            do_concat(api, J, "rnum", i);
            rand_str(&mut rng, &mut buf);
            rand_str(&mut rng, &mut buf2);
            printf(
                cs("rstr[%d] a=<%s> b=<%s>\n").as_ptr(),
                i as c_int,
                buf.as_ptr(),
                buf2.as_ptr(),
            );
            (api.js_pushstring)(J, buf.as_ptr());
            (api.js_pushstring)(J, buf2.as_ptr());
            do_concat(api, J, "rstr", i);
            (api.js_pushnumber)(J, x);
            (api.js_pushstring)(J, buf.as_ptr());
            do_concat(api, J, "rmix", i);
        }

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 26 ==== */

#[test]
fn cfg26_repr() {
    diff("cfg26", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        hdr("cfg26_repr_all_kinds");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            (api.js_repr)(J, -1);
            let s = (api.js_tostring)(J, -1);
            pk_s(name, "repr", s);
            pk_i(name, "repr_itype", itype(api, J, -1));
            pk_i(name, "top", (api.js_gettop)(J));
            (api.js_pop)(J, 2);
        }

        hdr("cfg26_torepr_all_kinds");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let s = (api.js_torepr)(J, -1);
            pk_s(name, "torepr", s);
            pk_i(name, "torepr_itype", itype(api, J, -1));
            pk_i(name, "top", (api.js_gettop)(J));
            (api.js_pop)(J, 1);
        }

        hdr("cfg26_torepr_positive_index");
        p_int("top_before", (api.js_gettop)(J));
        (api.js_pushnumber)(J, 12.5);
        pk_s("posidx", "torepr0", (api.js_torepr)(J, 0));
        pk_i("posidx", "top", (api.js_gettop)(J));
        pk_i("posidx", "itype", itype(api, J, 0));
        (api.js_pop)(J, 1);

        hdr("cfg26_nested");
        let names: [&str; 7] = ["nested", "arr", "sparse", "cyc", "obj", "d", "nullproto"];
        for (i, n) in names.iter().enumerate() {
            gg(api, J, n);
            (api.js_repr)(J, -1);
            pi_s("nested_repr", i as c_int, (api.js_tostring)(J, -1));
            (api.js_pop)(J, 2);
        }

        hdr("cfg26_scripted");
        let rc = (api.js_dostring)(
            J,
            cs("var deep={a:{b:{c:{d:{e:[1,2,{f:'g'}]}}}}};\n\
                var cyc2=[1,2]; cyc2.push(cyc2);\n\
                var mixed=[undefined,null,true,1.5,'s\\n\"q\"',{k:1},[1,[2]],/re/g];\n\
                var keys={'0':1,'01':2,'a b':3,'_x':4,'\\u00e9':5};\n")
                .as_ptr(),
        );
        p_int("script_rc", rc);
        let names2: [&str; 4] = ["deep", "cyc2", "mixed", "keys"];
        for (i, n) in names2.iter().enumerate() {
            gg(api, J, n);
            (api.js_repr)(J, -1);
            pi_s("script_repr", i as c_int, (api.js_tostring)(J, -1));
            (api.js_pop)(J, 2);
            gg(api, J, n);
            pi_s("script_torepr", i as c_int, (api.js_torepr)(J, -1));
            (api.js_pop)(J, 1);
        }

        hdr("cfg26_tryrepr_throwing");
        gg(api, J, "gthrow");
        pk_s("gthrow", "tryrepr", (api.js_tryrepr)(J, -1, err()));
        pk_i("gthrow", "top", (api.js_gettop)(J));
        (api.js_pop)(J, 1);
        gg(api, J, "othrow");
        pk_s("othrow", "tryrepr", (api.js_tryrepr)(J, -1, err()));
        (api.js_pop)(J, 1);

        hdr("cfg26_repr_strings");
        let strs: [&str; 8] = [
            "",
            "plain",
            "quo\"te",
            "back\\slash",
            "ctl\u{1}\u{2}\u{1f}",
            "tab\tnl\ncr\rbs\u{8}ff\u{c}",
            "h\u{e9}ll\u{20ac}",
            "\u{1d11e}",
        ];
        for (i, s) in strs.iter().enumerate() {
            let c = cs(s);
            (api.js_pushstring)(J, c.as_ptr());
            (api.js_repr)(J, -1);
            pi_s("repr_str", i as c_int, (api.js_tostring)(J, -1));
            (api.js_pop)(J, 2);
        }

        hdr("cfg26_repr_numbers");
        let mut rng = Rng::new(0x26_2626);
        for i in 0..40 {
            let x = if i % 5 == 0 { rng.bits_f64() } else { rng.nice_f64() };
            (api.js_pushnumber)(J, x);
            printf(cs("rnum[%d] x=%.17g\n").as_ptr(), i as c_int, x);
            (api.js_repr)(J, -1);
            pi_s("repr_num", i, (api.js_tostring)(J, -1));
            (api.js_pop)(J, 2);
        }

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 27 ==== */

unsafe fn push_six(api: &Api, J: JS) {
    (api.js_pushnumber)(J, 100.0);
    (api.js_pushstring)(J, cs("s1").as_ptr());
    (api.js_pushboolean)(J, 1);
    (api.js_pushnull)(J);
    (api.js_pushstring)(J, cs("a-long-string-value-here").as_ptr());
    (api.js_pushnumber)(J, -6.5);
}

#[test]
fn cfg27_stack_ops() {
    diff("cfg27", |api| unsafe {
        let J = newstate(api, 0);

        hdr("cfg27_copy");
        push_six(api, J);
        dumpstack(api, J, "start");
        (api.js_copy)(J, 0);
        dumpstack(api, J, "copy(0)");
        (api.js_copy)(J, -1);
        dumpstack(api, J, "copy(-1)");
        (api.js_copy)(J, 2);
        dumpstack(api, J, "copy(2)");
        (api.js_copy)(J, -3);
        dumpstack(api, J, "copy(-3)");
        (api.js_copy)(J, 99);
        dumpstack(api, J, "copy(99)");
        (api.js_copy)(J, -99);
        dumpstack(api, J, "copy(-99)");
        (api.js_pop)(J, (api.js_gettop)(J));
        dumpstack(api, J, "emptied");

        hdr("cfg27_rot");
        push_six(api, J);
        (api.js_rot)(J, 2);
        dumpstack(api, J, "rot(2)");
        (api.js_rot)(J, 3);
        dumpstack(api, J, "rot(3)");
        (api.js_rot)(J, 6);
        dumpstack(api, J, "rot(6)");
        (api.js_rot)(J, 1);
        dumpstack(api, J, "rot(1)");
        (api.js_rot)(J, 0);
        dumpstack(api, J, "rot(0)");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg27_remove");
        push_six(api, J);
        (api.js_remove)(J, 0);
        dumpstack(api, J, "remove(0)");
        (api.js_remove)(J, -1);
        dumpstack(api, J, "remove(-1)");
        (api.js_remove)(J, 2);
        dumpstack(api, J, "remove(2)");
        (api.js_remove)(J, -2);
        dumpstack(api, J, "remove(-2)");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg27_replace");
        push_six(api, J);
        (api.js_replace)(J, 0);
        dumpstack(api, J, "replace(0)");
        (api.js_replace)(J, -2);
        dumpstack(api, J, "replace(-2)");
        (api.js_replace)(J, 1);
        dumpstack(api, J, "replace(1)");
        (api.js_replace)(J, -1);
        dumpstack(api, J, "replace(-1)");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg27_pop");
        push_six(api, J);
        (api.js_pop)(J, 0);
        dumpstack(api, J, "pop(0)");
        (api.js_pop)(J, 1);
        dumpstack(api, J, "pop(1)");
        (api.js_pop)(J, 3);
        dumpstack(api, J, "pop(3)");
        (api.js_pop)(J, 2);
        dumpstack(api, J, "pop(2)");

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* js_insert is `js_error(J, "not implemented yet")` in 1.3.8: uncaught
     * error -> panic -> abort, in both libraries. */
    diff("cfg27_insert_abort", |api| unsafe {
        let J = newstate(api, 0);
        push_six(api, J);
        dumpstack(api, J, "before_insert");
        p_line("about to js_insert(2)");
        fflush(std::ptr::null_mut());
        (api.js_insert)(J, 2);
        p_line("unreached after js_insert");
        dumpstack(api, J, "after_insert");
        (api.js_freestate)(J);
    });

    diff("cfg27_remove_oob_abort", |api| unsafe {
        let J = newstate(api, 0);
        push_six(api, J);
        p_line("about to js_remove(99)");
        fflush(std::ptr::null_mut());
        (api.js_remove)(J, 99);
        p_line("unreached after js_remove");
        (api.js_freestate)(J);
    });

    diff("cfg27_replace_oob_abort", |api| unsafe {
        let J = newstate(api, 0);
        push_six(api, J);
        p_line("about to js_replace(-99)");
        fflush(std::ptr::null_mut());
        (api.js_replace)(J, -99);
        p_line("unreached after js_replace");
        (api.js_freestate)(J);
    });

    diff("cfg27_pop_underflow_abort", |api| unsafe {
        let J = newstate(api, 0);
        (api.js_pushnumber)(J, 1.0);
        p_line("about to js_pop(5)");
        fflush(std::ptr::null_mut());
        (api.js_pop)(J, 5);
        p_line("unreached after js_pop");
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 28 ==== */

#[test]
fn cfg28_dup_rot_ops() {
    diff("cfg28", |api| unsafe {
        let J = newstate(api, 0);

        hdr("cfg28_dup");
        push_six(api, J);
        dumpstack(api, J, "start");
        (api.js_dup)(J);
        dumpstack(api, J, "dup");
        (api.js_dup2)(J);
        dumpstack(api, J, "dup2");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg28_rot2");
        push_six(api, J);
        (api.js_rot2)(J);
        dumpstack(api, J, "rot2");
        (api.js_rot2)(J);
        dumpstack(api, J, "rot2_again");
        (api.js_rot3)(J);
        dumpstack(api, J, "rot3");
        (api.js_rot3)(J);
        dumpstack(api, J, "rot3_again");
        (api.js_rot4)(J);
        dumpstack(api, J, "rot4");
        (api.js_rot4)(J);
        dumpstack(api, J, "rot4_again");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg28_rotpop");
        push_six(api, J);
        (api.js_rot2pop1)(J);
        dumpstack(api, J, "rot2pop1");
        (api.js_rot3pop2)(J);
        dumpstack(api, J, "rot3pop2");
        (api.js_rot2pop1)(J);
        dumpstack(api, J, "rot2pop1_again");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg28_minimal");
        /* exactly enough values for each op */
        (api.js_pushnumber)(J, 1.0);
        (api.js_dup)(J);
        dumpstack(api, J, "dup_of_1");
        (api.js_dup2)(J);
        dumpstack(api, J, "dup2_of_2");
        (api.js_rot2)(J);
        dumpstack(api, J, "rot2_of_4");
        (api.js_rot3)(J);
        dumpstack(api, J, "rot3_of_4");
        (api.js_rot4)(J);
        dumpstack(api, J, "rot4_of_4");
        (api.js_rot3pop2)(J);
        dumpstack(api, J, "rot3pop2_of_4");
        (api.js_rot2pop1)(J);
        dumpstack(api, J, "rot2pop1_of_2");
        (api.js_pop)(J, (api.js_gettop)(J));

        hdr("cfg28_mixed_values");
        push_six(api, J);
        (api.js_dup)(J);
        (api.js_rot4)(J);
        (api.js_rot2pop1)(J);
        (api.js_dup2)(J);
        (api.js_rot3)(J);
        (api.js_rot3pop2)(J);
        dumpstack(api, J, "chain");
        p_int("gettop", (api.js_gettop)(J));
        (api.js_pop)(J, (api.js_gettop)(J));

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 85 ==== */

#[test]
fn cfg85_value_plumbing() {
    diff("cfg85", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        hdr("cfg85_roundtrip");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let v = (api.js_tovalue)(J, -1);
            pk_i(name, "tovalue_null", if v.is_null() { 1 } else { 0 });
            pk_i(name, "itype", (*v).t.type_ as c_int);
            let copy = *v;
            (api.js_pushvalue)(J, copy);
            pk_i(name, "rt_itype", itype(api, J, -1));
            pk_i(name, "rt_type", (api.js_type)(J, -1));
            pk_s(name, "rt_typeof", (api.js_typeof)(J, -1));
            pk_s(name, "rt_str", (api.js_trystring)(J, -1, err()));
            pk_i(name, "rt_strictequal", (api.js_strictequal)(J));
            pk_i(name, "rt_top", (api.js_gettop)(J));
            (api.js_pop)(J, 2);
        }

        hdr("cfg85_tovalue_oob");
        let v = (api.js_tovalue)(J, 42);
        pk_i("oob", "itype", (*v).t.type_ as c_int);
        let copy = *v;
        (api.js_pushvalue)(J, copy);
        pk_i("oob", "pushed_itype", itype(api, J, -1));
        pk_s("oob", "pushed_typeof", (api.js_typeof)(J, -1));
        (api.js_pop)(J, 1);

        hdr("cfg85_handmade");
        /* undefined / null / boolean / number built by hand */
        let mut hv = js_Value::zero();
        hv.t.type_ = 1;
        (api.js_pushvalue)(J, hv);
        pk_i("hand_undef", "itype", itype(api, J, -1));
        pk_s("hand_undef", "typeof", (api.js_typeof)(J, -1));
        pk_s("hand_undef", "str", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);

        let mut hv = js_Value::zero();
        hv.t.type_ = 2;
        (api.js_pushvalue)(J, hv);
        pk_i("hand_null", "itype", itype(api, J, -1));
        pk_s("hand_null", "typeof", (api.js_typeof)(J, -1));
        pk_s("hand_null", "str", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);

        let mut hv = js_Value::zero();
        hv.u.boolean = 1;
        hv.t.type_ = 3;
        (api.js_pushvalue)(J, hv);
        pk_i("hand_bool", "itype", itype(api, J, -1));
        pk_i("hand_bool", "toboolean", (api.js_toboolean)(J, -1));
        pk_s("hand_bool", "str", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);

        let mut hv = js_Value::zero();
        hv.u.number = 1.5e21;
        hv.t.type_ = 4;
        (api.js_pushvalue)(J, hv);
        pk_i("hand_num", "itype", itype(api, J, -1));
        pk_n("hand_num", "tonumber", (api.js_tonumber)(J, -1));
        pk_s("hand_num", "str", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);

        let mut hv = js_Value::zero();
        hv.u.litstr = LIT.as_ptr() as *const c_char;
        hv.t.type_ = 5;
        (api.js_pushvalue)(J, hv);
        pk_i("hand_lit", "itype", itype(api, J, -1));
        pk_s("hand_lit", "str", (api.js_tostring)(J, -1));
        pk_i("hand_lit", "isstring", (api.js_isstring)(J, -1));
        (api.js_pop)(J, 1);

        /* out-of-range type tag: hits the `default:` arms (treated as shrstr) */
        let mut hv = js_Value::zero();
        hv.u.shrstr[0] = b'a' as c_char;
        hv.u.shrstr[1] = b'b' as c_char;
        hv.u.shrstr[2] = b'c' as c_char;
        hv.t.type_ = 42;
        (api.js_pushvalue)(J, hv);
        pk_i("hand_badtype", "itype", itype(api, J, -1));
        pk_i("hand_badtype", "type", (api.js_type)(J, -1));
        pk_s("hand_badtype", "typeof", (api.js_typeof)(J, -1));
        pk_i("hand_badtype", "isstring", (api.js_isstring)(J, -1));
        pk_i("hand_badtype", "toboolean", (api.js_toboolean)(J, -1));
        pk_s("hand_badtype", "str", (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);

        hdr("cfg85_newmemstring");
        let raw: [(&[u8], c_int); 6] = [
            (b"hello world!!!!", 15),
            (b"hello world!!!!!", 16),
            (b"", 0),
            (b"abc", 1),
            (b"a\0bc", 4),
            (b"h\xc3\xa9ll\xe2\x82\xac", 8),
        ];
        for (i, (b, n)) in raw.iter().enumerate() {
            let st = (api.jsV_newmemstring)(J, b.as_ptr() as *const c_char, *n);
            pi_i("memstr_nonnull", i as c_int, if st.is_null() { 0 } else { 1 });
            let mut hv = js_Value::zero();
            hv.u.memstr = st;
            hv.t.type_ = 6;
            (api.js_pushvalue)(J, hv);
            report_str(api, J, "memstr", i as c_int);
            let s = (api.js_tostring)(J, -1);
            dump_bytes("memstr", i as c_int, s, *n + 1);
            (api.js_pop)(J, 1);
        }

        hdr("cfg85_pushobject");
        let names: [&str; 6] = ["obj", "arr", "fn", "d", "Math", "JSON"];
        for (i, n) in names.iter().enumerate() {
            gg(api, J, n);
            let o = (api.js_toobject)(J, -1);
            pi_i("obj_nonnull", i as c_int, if o.is_null() { 0 } else { 1 });
            (api.js_pushobject)(J, o);
            pi_i("obj_type", i as c_int, (api.js_type)(J, -1));
            pi_s("obj_typeof", i as c_int, (api.js_typeof)(J, -1));
            pi_i("obj_strictequal", i as c_int, (api.js_strictequal)(J));
            pi_s("obj_repr", i as c_int, (api.js_tryrepr)(J, -1, err()));
            (api.js_pop)(J, 2);
        }
        /* object obtained from a primitive */
        (api.js_pushstring)(J, cs("prim").as_ptr());
        let o = (api.js_toobject)(J, -1);
        pk_i("prim_str", "slot_itype_after", itype(api, J, -1));
        (api.js_pushobject)(J, o);
        pk_s("prim_str", "typeof", (api.js_typeof)(J, -1));
        pk_i("prim_str", "isstringobject", (api.js_isstringobject)(J, -1));
        pk_s("prim_str", "repr", (api.js_tryrepr)(J, -1, err()));
        pk_i("prim_str", "strictequal", (api.js_strictequal)(J));
        (api.js_pop)(J, 2);

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 86 ==== */

#[test]
fn cfg86_toprimitive() {
    diff("cfg86", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        let hints: [c_int; 4] = [0, 1, 2, 99];
        let kinds: [c_int; 18] = [
            0, 1, 2, 4, 6, 7, 11, 13, 14, 16, 17, 18, 20, 21, 22, 23, 24, 26,
        ];

        hdr("cfg86_js_toprimitive");
        for h in hints.iter() {
            for k in kinds.iter() {
                let name = KINDS[*k as usize];
                push_kind(api, J, *k);
                let before = itype(api, J, -1);
                (api.js_toprimitive)(J, -1, *h);
                printf(
                    cs("toprim hint=%d %s before=%d after=%d type=%d typeof=%s str=<%s>\n").as_ptr(),
                    *h,
                    cs(name).as_ptr(),
                    before,
                    itype(api, J, -1),
                    (api.js_type)(J, -1),
                    nn((api.js_typeof)(J, -1)),
                    nn((api.js_trystring)(J, -1, err())),
                );
                (api.js_pop)(J, 1);
            }
        }

        hdr("cfg86_others");
        let names: [&str; 4] = ["ots", "oboth", "nullproto", "cyc"];
        for h in hints.iter() {
            for n in names.iter() {
                gg(api, J, n);
                (api.js_toprimitive)(J, -1, *h);
                printf(
                    cs("toprim2 hint=%d %s after=%d str=<%s>\n").as_ptr(),
                    *h,
                    cs(n).as_ptr(),
                    itype(api, J, -1),
                    nn((api.js_trystring)(J, -1, err())),
                );
                (api.js_pop)(J, 1);
            }
        }

        hdr("cfg86_jsV_toprimitive");
        for h in hints.iter() {
            for k in kinds.iter() {
                let name = KINDS[*k as usize];
                push_kind(api, J, *k);
                let pv = (api.js_tovalue)(J, -1);
                (api.jsV_toprimitive)(J, pv, *h);
                printf(
                    cs("Vtoprim hint=%d %s after=%d str=<%s>\n").as_ptr(),
                    *h,
                    cs(name).as_ptr(),
                    itype(api, J, -1),
                    nn((api.js_trystring)(J, -1, err())),
                );
                (api.js_pop)(J, 1);
            }
        }

        hdr("cfg86_toprimitive_oob_index");
        (api.js_toprimitive)(J, 33, 1);
        p_line("toprimitive on out-of-range index survived");

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* strict mode: an object with no primitive conversion throws */
    diff("cfg86_strict_abort", |api| unsafe {
        let J = newstate(api, JS_STRICT);
        setup(api, J);
        gg(api, J, "nullproto");
        p_int("itype", itype(api, J, -1));
        p_line("about to js_toprimitive in strict mode");
        fflush(std::ptr::null_mut());
        (api.js_toprimitive)(J, -1, 1);
        p_line("unreached");
        p_int("itype_after", itype(api, J, -1));
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 87 ==== */

#[test]
fn cfg87_jsV_conversions() {
    diff("cfg87", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        hdr("cfg87_toboolean");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let v = (api.js_tovalue)(J, -1);
            pk_i(name, "jsV_toboolean", (api.jsV_toboolean)(J, v));
            pk_i(name, "itype_after", itype(api, J, -1));
            (api.js_pop)(J, 1);
        }

        hdr("cfg87_tonumber");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let v = (api.js_tovalue)(J, -1);
            pk_n(name, "jsV_tonumber", (api.jsV_tonumber)(J, v));
            pk_i(name, "itype_after", itype(api, J, -1));
            (api.js_pop)(J, 1);
        }

        hdr("cfg87_tointeger");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let v = (api.js_tovalue)(J, -1);
            pk_n(name, "jsV_tointeger", (api.jsV_tointeger)(J, v));
            pk_i(name, "itype_after", itype(api, J, -1));
            (api.js_pop)(J, 1);
        }

        hdr("cfg87_tostring");
        for k in 0..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let v = (api.js_tovalue)(J, -1);
            let s = (api.jsV_tostring)(J, v);
            pk_s(name, "jsV_tostring", s);
            pk_i(name, "len", if s.is_null() { -1 } else { strlen(s) as c_int });
            pk_i(name, "utflen", if s.is_null() { -1 } else { (api.js_utflen)(s) });
            pk_i(name, "itype_after", itype(api, J, -1));
            (api.js_pop)(J, 1);
        }

        hdr("cfg87_toobject");
        /* every coercible kind (undefined/null throw and are checked below) */
        for k in 2..NSAFE {
            let name = KINDS[k as usize];
            push_kind(api, J, k);
            let v = (api.js_tovalue)(J, -1);
            let o = (api.jsV_toobject)(J, v);
            pk_i(name, "jsV_toobject_nonnull", if o.is_null() { 0 } else { 1 });
            pk_i(name, "itype_after", itype(api, J, -1));
            (api.js_pushobject)(J, o);
            pk_s(name, "obj_typeof", (api.js_typeof)(J, -1));
            pk_s(name, "obj_repr", (api.js_tryrepr)(J, -1, err()));
            pk_i(name, "strictequal_with_slot", (api.js_strictequal)(J));
            (api.js_pop)(J, 2);
        }

        hdr("cfg87_oob_value");
        let v = (api.js_tovalue)(J, 91);
        pk_i("oob", "jsV_toboolean", (api.jsV_toboolean)(J, v));
        pk_n("oob", "jsV_tonumber", (api.jsV_tonumber)(J, v));
        pk_n("oob", "jsV_tointeger", (api.jsV_tointeger)(J, v));
        pk_s("oob", "jsV_tostring", (api.jsV_tostring)(J, v));

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    diff("cfg87_toobject_undefined_abort", |api| unsafe {
        let J = newstate(api, 0);
        (api.js_pushundefined)(J);
        let v = (api.js_tovalue)(J, -1);
        p_line("about to jsV_toobject(undefined)");
        fflush(std::ptr::null_mut());
        let o = (api.jsV_toobject)(J, v);
        p_ptr_nonnull("unreached", o);
        (api.js_freestate)(J);
    });

    diff("cfg87_toobject_null_abort", |api| unsafe {
        let J = newstate(api, 0);
        (api.js_pushnull)(J);
        let v = (api.js_tovalue)(J, -1);
        p_line("about to jsV_toobject(null)");
        fflush(std::ptr::null_mut());
        let o = (api.jsV_toobject)(J, v);
        p_ptr_nonnull("unreached", o);
        (api.js_freestate)(J);
    });
}

/* ============================================================== row 88 ==== */

#[test]
fn cfg88_toobject_isarrayindex() {
    diff("cfg88_toobject", |api| unsafe {
        let J = newstate(api, 0);
        setup(api, J);

        let kinds: [c_int; 17] = [
            2, 3, 4, 5, 6, 7, 8, 10, 11, 13, 14, 15, 16, 17, 18, 20, 24,
        ];
        for k in kinds.iter() {
            let name = KINDS[*k as usize];
            hdr1("cfg88", name);
            push_kind(api, J, *k);
            pk_i(name, "itype_before", itype(api, J, -1));
            let o = (api.js_toobject)(J, -1);
            pk_i(name, "nonnull", if o.is_null() { 0 } else { 1 });
            pk_i(name, "itype_after", itype(api, J, -1));
            pk_i(name, "isobject_after", (api.js_isobject)(J, -1));
            pk_s(name, "typeof_after", (api.js_typeof)(J, -1));
            pk_i(name, "isnumberobject", (api.js_isnumberobject)(J, -1));
            pk_i(name, "isstringobject", (api.js_isstringobject)(J, -1));
            pk_i(name, "isbooleanobject", (api.js_isbooleanobject)(J, -1));
            pk_i(name, "isdateobject", (api.js_isdateobject)(J, -1));
            pk_s(name, "repr", (api.js_tryrepr)(J, -1, err()));
            /* second call must return the very same object */
            (api.js_pushobject)(J, o);
            pk_i(name, "strictequal", (api.js_strictequal)(J));
            pk_s(name, "wrapped_tostring", (api.js_trystring)(J, -1, err()));
            pk_n(name, "wrapped_tonumber", (api.js_trynumber)(J, -1, -1.0));
            (api.js_pop)(J, 2);
        }

        p_int("final_top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    diff("cfg88_toobject_undefined_abort", |api| unsafe {
        let J = newstate(api, 0);
        (api.js_pushundefined)(J);
        p_line("about to js_toobject(undefined)");
        fflush(std::ptr::null_mut());
        let o = (api.js_toobject)(J, -1);
        p_ptr_nonnull("unreached", o);
        (api.js_freestate)(J);
    });

    diff("cfg88_toobject_null_abort", |api| unsafe {
        let J = newstate(api, 0);
        (api.js_pushnull)(J);
        p_line("about to js_toobject(null)");
        fflush(std::ptr::null_mut());
        let o = (api.js_toobject)(J, -1);
        p_ptr_nonnull("unreached", o);
        (api.js_freestate)(J);
    });

    diff("cfg88_isarrayindex", |api| unsafe {
        let J = newstate(api, 0);

        let cases: [&str; 30] = [
            "0",
            "1",
            "-1",
            "01",
            "4294967295",
            "4294967294",
            "",
            "abc",
            "1e2",
            " 1",
            "0x10",
            "00",
            "000",
            "0.5",
            "+1",
            "1 ",
            "10",
            "9",
            "2147483647",
            "2147483646",
            "214748364",
            "214748365",
            "2147483648",
            "999999999999999999999",
            "12345678901234567890123456789",
            "1a",
            "a1",
            "1\t",
            "007",
            "4294967296",
        ];
        for (i, s) in cases.iter().enumerate() {
            let c = cs(s);
            let mut idx: c_int = -12345;
            let rv = (api.js_isarrayindex)(J, c.as_ptr(), &mut idx);
            printf(
                cs("isarrayindex[%d] s=<%s> rv=%d idx=%d\n").as_ptr(),
                i as c_int,
                c.as_ptr(),
                rv,
                idx,
            );
        }

        hdr("cfg88_isarrayindex_random");
        let mut rng = Rng::new(0x88_8888);
        let mut buf = [0 as c_char; 96];
        for i in 0..80 {
            let n = 1 + rng.range(13) as usize;
            for j in 0..n {
                buf[j] = (b'0' + (rng.range(10) as u8)) as c_char;
            }
            buf[n] = 0;
            let mut idx: c_int = -12345;
            let rv = (api.js_isarrayindex)(J, buf.as_ptr(), &mut idx);
            printf(
                cs("rnd_digits[%d] s=<%s> rv=%d idx=%d\n").as_ptr(),
                i as c_int,
                buf.as_ptr(),
                rv,
                idx,
            );
        }
        for i in 0..60 {
            rand_str(&mut rng, &mut buf);
            let mut idx: c_int = -12345;
            let rv = (api.js_isarrayindex)(J, buf.as_ptr(), &mut idx);
            printf(
                cs("rnd_garbage[%d] s=<%s> rv=%d idx=%d\n").as_ptr(),
                i as c_int,
                buf.as_ptr(),
                rv,
                idx,
            );
        }

        (api.js_freestate)(J);
    });
}
