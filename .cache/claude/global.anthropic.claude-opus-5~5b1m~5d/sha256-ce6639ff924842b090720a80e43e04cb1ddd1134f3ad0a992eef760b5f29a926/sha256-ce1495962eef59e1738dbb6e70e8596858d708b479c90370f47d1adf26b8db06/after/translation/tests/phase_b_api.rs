//! Phase B — differential tests driving the raw C API (the lowest-level public
//! entry points) through both `.so` files: state/flags, stack manipulation,
//! push/new constructors, properties + attribute flags, arrays (flat/sparse),
//! iterators, userdata, cfunctions, operators, repr, gc/dumpstrings.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0xC0FFEE_1234_5678;

/* All state flag shapes the C actually distinguishes (only bit 0 is read). */
const FLAGSETS: [c_int; 6] = [0, JS_STRICT, 2, 3, -1, 0x7fffffff];

/// `js_pushliteral` does NOT copy: the pointer must outlive the state, so it
/// has to be a static string (a temporary CString would be a dangling read).
fn lit_literal() -> *const c_char {
    b"literal\0".as_ptr() as *const c_char
}
fn lit_lit() -> *const c_char {
    b"lit\0".as_ptr() as *const c_char
}

fn dump(a: &Api, J: JS) {
    unsafe {
        let n = (a.js_gettop)(J);
        emit(&format!("top={}", n));
        let e = cs("<repr!>");
        for i in 0..n {
            emit(&format!("[{}]={}", i, repr_at(a, J, i)));
        }
    }
}

/* push a fixed, varied set of values on the stack */
fn push_zoo(a: &Api, J: JS) {
    unsafe {
        (a.js_pushundefined)(J);
        (a.js_pushnull)(J);
        (a.js_pushboolean)(J, 0);
        (a.js_pushboolean)(J, 1);
        (a.js_pushboolean)(J, 42);
        (a.js_pushnumber)(J, 0.0);
        (a.js_pushnumber)(J, -0.0);
        (a.js_pushnumber)(J, f64::NAN);
        (a.js_pushnumber)(J, f64::INFINITY);
        (a.js_pushnumber)(J, -1.5);
        (a.js_pushnumber)(J, 1e21);
        (a.js_pushnumber)(J, 2147483647.0);
        (a.js_pushstring)(J, cs("").as_ptr());
        (a.js_pushstring)(J, cs("short").as_ptr());
        (a.js_pushstring)(J, cs("123456789012345").as_ptr()); /* 15 == shrstr max */
        (a.js_pushstring)(J, cs("1234567890123456").as_ptr()); /* 16 == memstr */
        (a.js_pushliteral)(J, lit_literal());
        (a.js_pushlstring)(J, cs("abcdef").as_ptr(), 3);
        (a.js_pushstring)(J, cs("\u{e9}\u{4e2d}\u{1f600}").as_ptr());
        (a.js_newobject)(J);
        (a.js_newarray)(J);
        (a.js_newboolean)(J, 1);
        (a.js_newnumber)(J, 7.25);
        (a.js_newstring)(J, cs("boxed").as_ptr());
        (a.js_newregexp)(J, cs("a+b").as_ptr(), JS_REGEXP_G);
        (a.js_pushglobal)(J);
        (a.js_newobjectx)(J);
    }
}

/* ------------------------------------------------------- state / context */

#[test]
fn state_flags_context_atpanic_report() {
    unsafe extern "C" fn rep(_J: JS, msg: *const c_char) {
        emit(&format!("report:{}", unsafe { rs(msg) }));
    }
    unsafe extern "C" fn pan(_J: JS) {
        emit("panic");
    }
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_setcontext)(J, 0x1234 as *mut c_void);
            emit(&format!("ctx={:?}", (a.js_getcontext)(J)));
            (a.js_setreport)(J, Some(rep));
            (a.js_report)(J, cs("hello report").as_ptr());
            let old = (a.js_atpanic)(J, Some(pan));
            emit(&format!("oldpanic_is_none={}", old.is_none()));
            let old2 = (a.js_atpanic)(J, old);
            emit(&format!("restored={}", old2 == Some(pan as unsafe extern "C" fn(JS))));
            (a.js_pushnumber)(J, 1.0);
        }
    }
    for f in FLAGSETS {
        diff_native("state/context", act, f);
    }
}

#[test]
fn state_strict_flag_paths() {
    /* every place the C tests J->strict */
    let srcs = [
        "x = 1; x",
        "with({a:1}){a}",
        "delete x",
        "var o={}; delete o.a",
        "function f(){ arguments = 1 } f()",
        "function f(){ eval = 1 } f()",
        "try{ null.x }catch(e){ e.toString() }",
        "(function(){ return this })()",
        "var o = Object.freeze({a:1}); o.a = 2; o.a",
        "var o = Object.freeze({a:1}); delete o.a",
        "'use strict'; x2 = 1",
        "function f(){ 'use strict'; return this } f()",
        "var s='abc'; s.length = 9; s.length",
        "eval('var q=1; q')",
        "function f(a,a){return a} f(1,2)",
        "var f = function(){ return typeof arguments }; f()",
        "try{ undefinedVariable }catch(e){ e.name }",
    ];
    for f in [0, JS_STRICT] {
        for s in srcs {
            diff_eval("strict", s, f);
        }
    }
}

#[test]
fn setlimit_run_and_mem() {
    /* runlimit fires on instruction count; memlimit on js_malloc/js_realloc */
    let cases: [(c_int, c_int); 8] = [
        (0, 0),
        (1, 0),
        (2, 0),
        (1000, 0),
        (0, 1),
        (0, 4096),
        (0, 1 << 20),
        (100, 1 << 16),
    ];
    for (rl, ml) in cases {
        for f in [0, JS_STRICT] {
            let p = libs();
            set_pi(0, rl as i64);
            set_pi(1, ml as i64);
            fn act(a: &Api, J: JS) {
                unsafe {
                    (a.js_setlimit)(J, pic(0), pic(1));
                    let src = cs("var s=0; for (var i=0;i<200;i++) s+=i; s");
                    let n = cs("limit.js");
                    let rc = (a.js_ploadstring)(J, n.as_ptr(), src.as_ptr());
                    emit(&format!("load={}", rc));
                    if rc == 0 {
                        (a.js_pushundefined)(J);
                        let rc = (a.js_pcall)(J, 0);
                        emit(&format!("call={}", rc));
                        let e = cs("<x>");
                        emit(&str_at(a, J, -1));
                        (a.js_pop)(J, 1);
                    } else {
                        (a.js_pop)(J, 1);
                    }
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let c = p.c.run_native(act, f);
            let r = p.r.run_native(act, f);
            same(&format!("setlimit({},{}) flags={}", rl, ml, f), &c, &r);
        }
    }
}

/* ------------------------------------------------------------ stack ops */

#[test]
fn stack_push_and_dump() {
    fn act(a: &Api, J: JS) {
        push_zoo(a, J);
        dump(a, J);
        unsafe { (a.js_pushnumber)(J, 0.0) };
    }
    for f in FLAGSETS {
        diff_native("push_zoo", act, f);
    }
}

/* minimum stack depth (including `this` at index 0) each operator needs */
fn op_needs(op: i64) -> i64 {
    match op {
        1 => 1,  /* dup   */
        2 => 2,  /* dup2  */
        3 => 2,  /* rot2  */
        4 => 3,  /* rot3  */
        5 => 4,  /* rot4  */
        6 => 2,  /* rot2pop1 */
        7 => 3,  /* rot3pop2 */
        _ => 0,
    }
}

#[test]
fn stack_manipulators_randomized() {
    /* VALID uses only: indices inside the frame and counts within the depth.
     * Out-of-range indices/counts are C undefined behaviour (js_pop with a
     * negative count moves TOP up) and are covered in the isolated Phase C
     * tests instead. */
    let mut rng = Rng::new(SEED);
    for iter in 0..600 {
        let op = rng.range_i64(0, 11);
        let npush = rng.range_i64(op_needs(op).max(0), 6);
        let depth = npush + 1; /* + `this` */
        let idx = match op {
            0 | 9 | 10 => {
                /* copy / remove / replace: any valid absolute or negative index */
                if rng.below(2) == 0 {
                    rng.range_i64(0, depth - 1)
                } else {
                    rng.range_i64(-depth, -1)
                }
            }
            8 => rng.range_i64(1, depth), /* rot(n): 1..depth */
            11 => rng.range_i64(0, depth), /* pop(n): 0..depth */
            _ => 0,
        };
        set_pi(0, op);
        set_pi(1, idx);
        set_pi(2, npush);
        fn act(a: &Api, J: JS) {
            unsafe {
                let n = pi(2);
                for k in 0..n {
                    (a.js_pushnumber)(J, k as f64 + 0.5);
                }
                let idx = pic(1);
                match pi(0) {
                    0 => (a.js_copy)(J, idx),
                    1 => (a.js_dup)(J),
                    2 => (a.js_dup2)(J),
                    3 => (a.js_rot2)(J),
                    4 => (a.js_rot3)(J),
                    5 => (a.js_rot4)(J),
                    6 => (a.js_rot2pop1)(J),
                    7 => (a.js_rot3pop2)(J),
                    8 => (a.js_rot)(J, idx),
                    9 => (a.js_remove)(J, idx),
                    10 => (a.js_replace)(J, idx),
                    _ => (a.js_pop)(J, idx),
                }
                dump(a, J);
                (a.js_pushnumber)(J, 1.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(
            &format!("stackop iter={} op={} idx={} n={}", iter, pi(0), pi(1), pi(2)),
            &c,
            &r,
        );
    }
}

/* ------------------------------------------------- predicates + conversions */

#[test]
fn predicates_and_conversions_over_zoo() {
    fn act(a: &Api, J: JS) {
        unsafe {
            push_zoo(a, J);
            let n = (a.js_gettop)(J);
            let tag = cs("tag");
            for i in 0..n {
                emit(&format!(
                    "[{}] {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                    i,
                    (a.js_isdefined)(J, i),
                    (a.js_isundefined)(J, i),
                    (a.js_isnull)(J, i),
                    (a.js_isboolean)(J, i),
                    (a.js_isnumber)(J, i),
                    (a.js_isstring)(J, i),
                    (a.js_isprimitive)(J, i),
                    (a.js_isobject)(J, i),
                    (a.js_isarray)(J, i),
                    (a.js_isregexp)(J, i),
                    (a.js_iscoercible)(J, i),
                    (a.js_iscallable)(J, i),
                    (a.js_isuserdata)(J, i, tag.as_ptr()),
                    (a.js_iserror)(J, i),
                    (a.js_isnumberobject)(J, i),
                    (a.js_isstringobject)(J, i),
                    (a.js_isbooleanobject)(J, i),
                    (a.js_isdateobject)(J, i),
                ));
                emit(&format!(
                    "typeof={} type={}",
                    rs((a.js_typeof)(J, i)),
                    (a.js_type)(J, i)
                ));
                /* the try* family never throws, so it is safe for every value */
                let err = cs("<ERR>");
                emit(&format!(
                    "tryb={} tryi={} tryn={:#x} trys={:?} tryrepr={:?}",
                    (a.js_tryboolean)(J, i, -7),
                    (a.js_tryinteger)(J, i, -7),
                    (a.js_trynumber)(J, i, -7.5).to_bits(),
                    rs((a.js_trystring)(J, i, err.as_ptr())),
                    rs((a.js_tryrepr)(J, i, err.as_ptr())),
                ));
                emit(&format!("tob={}", (a.js_toboolean)(J, i)));
            }
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in FLAGSETS {
        diff_native("predicates", act, f);
    }
}

#[test]
fn numeric_conversions_through_stack() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483647.5,
        -2147483648.5,
        4294967296.0,
        65535.5,
        1e21,
        -1e21,
        9007199254740993.0,
    ];
    for _ in 0..600 {
        vals.push(rng.f64());
    }
    for v in vals {
        set_pf(0, v);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_pushnumber)(J, pf(0));
                emit(&format!(
                    "{} {} {} {} {} {:#x} {:?}",
                    (a.js_tointeger)(J, -1),
                    (a.js_toint32)(J, -1),
                    (a.js_touint32)(J, -1),
                    (a.js_toint16)(J, -1),
                    (a.js_touint16)(J, -1),
                    (a.js_tonumber)(J, -1).to_bits(),
                    rs((a.js_tostring)(J, -1)),
                ));
                /* the same value as a string, re-converted */
                let s = (a.js_tostring)(J, -1);
                (a.js_pushstring)(J, s);
                emit(&format!(
                    "str->{:#x} {} {}",
                    (a.js_tonumber)(J, -1).to_bits(),
                    (a.js_tointeger)(J, -1),
                    (a.js_toint32)(J, -1)
                ));
                (a.js_pop)(J, 2);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("numconv {:e} ({:#x})", v, v.to_bits()), &c, &r);
    }
}

#[test]
fn string_shapes_through_stack() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut strs: Vec<String> = vec![
        "".into(),
        "a".into(),
        "0123456789ABCDE".into(),
        "0123456789ABCDEF".into(),
        "\u{e9}".into(),
        "\u{4e2d}\u{6587}".into(),
        "\u{1f600}".into(),
        "tab\there\nnl".into(),
        "\u{7f}\u{1}".into(),
        "quote\"and'".into(),
        "back\\slash".into(),
    ];
    for _ in 0..500 {
        strs.push(rng.string(20));
    }
    for s in strs {
        set_ps(0, &s);
        fn act(a: &Api, J: JS) {
            unsafe {
                let s = ps(0);
                let n = s.as_bytes().len() as c_int;
                (a.js_pushstring)(J, s.as_ptr());
                (a.js_pushliteral)(J, s.as_ptr());
                (a.js_pushlstring)(J, s.as_ptr(), n);
                if n > 0 {
                    (a.js_pushlstring)(J, s.as_ptr(), n - 1);
                }
                (a.js_newstring)(J, s.as_ptr());
                dump(a, J);
                let m = (a.js_gettop)(J);
                for i in 0..m {
                    emit(&format!(
                        "len={} num={:#x} bool={}",
                        (a.js_getlength)(J, i),
                        (a.js_tonumber)(J, i).to_bits(),
                        (a.js_toboolean)(J, i)
                    ));
                }
                /* js_runeat / js_utflen / js_isarrayindex on the same bytes */
                let mut idx: c_int = -1;
                emit(&format!(
                    "arrayindex={} idx={} utflen={}",
                    (a.js_isarrayindex)(J, s.as_ptr(), &mut idx),
                    idx,
                    (a.js_utflen)(s.as_ptr())
                ));
                for k in -2..6 {
                    emit(&format!("runeat({})={}", k, (a.js_runeat)(J, s.as_ptr(), k)));
                }
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("stringshape {:?}", s), &c, &r);
    }
}

#[test]
fn isarrayindex_randomized() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut probes: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "-1".into(),
        "01".into(),
        "10".into(),
        "4294967294".into(),
        "4294967295".into(),
        "4294967296".into(),
        "99999999999999999999".into(),
        "1.5".into(),
        "".into(),
        " 1".into(),
        "1 ".into(),
        "+1".into(),
        "0x1".into(),
        "1e3".into(),
        "2147483647".into(),
        "2147483648".into(),
    ];
    for _ in 0..2000 {
        let mut s = String::new();
        for _ in 0..rng.below(12) {
            s.push([
                '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '+', '.', 'e', ' ',
            ][rng.below(15) as usize]);
        }
        probes.push(s);
    }
    for s in probes {
        set_ps(0, &s);
        fn act(a: &Api, J: JS) {
            unsafe {
                let s = ps(0);
                let mut idx: c_int = -12345;
                emit(&format!(
                    "{} {}",
                    (a.js_isarrayindex)(J, s.as_ptr(), &mut idx),
                    idx
                ));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("isarrayindex {:?}", s),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

/* ------------------------------------------------ properties + attributes */

#[test]
fn properties_all_attribute_combinations() {
    for atts in 0..16 {
        for f in [0, JS_STRICT] {
            set_pi(0, atts);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let atts = pic(0);
                    (a.js_newobject)(J);
                    let n1 = cs("a");
                    let n2 = cs("b");
                    (a.js_pushnumber)(J, 1.0);
                    (a.js_defproperty)(J, -2, n1.as_ptr(), atts);
                    (a.js_pushnumber)(J, 2.0);
                    (a.js_setproperty)(J, -2, n2.as_ptr());
                    emit(&format!(
                        "has_a={} has_b={} has_c={}",
                        (a.js_hasproperty)(J, -1, n1.as_ptr()),
                        (a.js_hasproperty)(J, -1, n2.as_ptr()),
                        (a.js_hasproperty)(J, -1, cs("c").as_ptr())
                    ));
                    /* hasproperty pushes the value when found */
                    let e = cs("<x>");
                    dump(a, J);
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    /* attempt to overwrite the (maybe readonly) property */
                    (a.js_pushnumber)(J, 99.0);
                    (a.js_setproperty)(J, -2, n1.as_ptr());
                    (a.js_getproperty)(J, -1, n1.as_ptr());
                    emit(&format!("after_set_a={}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    /* re-define with different attributes */
                    (a.js_pushnumber)(J, 5.0);
                    (a.js_defproperty)(J, -2, n1.as_ptr(), atts ^ 7);
                    (a.js_getproperty)(J, -1, n1.as_ptr());
                    emit(&format!("after_redef={}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    /* enumerate: DONTENUM must be honoured */
                    (a.js_pushiterator)(J, -1, 1);
                    loop {
                        let nm = (a.js_nextiterator)(J, -1);
                        if nm.is_null() {
                            break;
                        }
                        emit(&format!("iter={:?}", rs(nm)));
                    }
                    (a.js_pop)(J, 1);
                    /* delete: DONTCONF must be honoured */
                    (a.js_delproperty)(J, -1, n1.as_ptr());
                    emit(&format!("has_a_after_del={}", (a.js_hasproperty)(J, -1, n1.as_ptr())));
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    /* accessors */
                    (a.js_newcfunction)(J, Some(cf_sum), b"g\0".as_ptr() as *const c_char, 0);
                    (a.js_pushundefined)(J);
                    (a.js_defaccessor)(J, -3, cs("acc").as_ptr(), atts);
                    emit(&format!("has_acc={}", (a.js_hasproperty)(J, -1, cs("acc").as_ptr())));
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    /* global variants of the same operations */
                    (a.js_pushnumber)(J, 3.0);
                    (a.js_defglobal)(J, cs("gv").as_ptr(), atts);
                    (a.js_getglobal)(J, cs("gv").as_ptr());
                    emit(&format!("gv={}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 4.0);
                    (a.js_setglobal)(J, cs("gv").as_ptr());
                    (a.js_getglobal)(J, cs("gv").as_ptr());
                    emit(&format!("gv2={}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    (a.js_delglobal)(J, cs("gv").as_ptr());
                    (a.js_getglobal)(J, cs("gv").as_ptr());
                    emit(&format!("gv3={}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            same(
                &format!("attrs={} flags={}", atts, f),
                &p.c.run_native(act, f),
                &p.r.run_native(act, f),
            );
        }
    }
}

#[test]
fn arrays_flat_and_sparse() {
    let mut rng = Rng::new(SEED ^ 4);
    for iter in 0..300 {
        set_pi(0, rng.range_i64(0, 5)); /* build mode */
        set_pi(1, rng.range_i64(0, 12)); /* count */
        set_pi(2, rng.range_i64(-3, 20)); /* index to poke */
        fn act(a: &Api, J: JS) {
            unsafe {
                let mode = pi(0);
                let n = pi(1) as c_int;
                let poke = pic(2);
                (a.js_newarray)(J);
                match mode {
                    0 => {
                        /* dense, ascending: stays flat */
                        for i in 0..n {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i);
                        }
                    }
                    1 => {
                        /* descending: forces sparse */
                        for i in (0..n).rev() {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i);
                        }
                    }
                    2 => {
                        /* holes */
                        for i in 0..n {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i * 3);
                        }
                    }
                    3 => {
                        /* defproperty unflattens */
                        for i in 0..n {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i);
                        }
                        (a.js_pushnumber)(J, 42.0);
                        (a.js_defproperty)(J, -2, cs("1").as_ptr(), JS_READONLY);
                    }
                    4 => {
                        for i in 0..n {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i);
                        }
                        (a.js_setlength)(J, -1, n / 2);
                    }
                    _ => {
                        (a.js_setlength)(J, -1, n);
                        for i in 0..n {
                            (a.js_pushstring)(J, cs("v").as_ptr());
                            (a.js_setindex)(J, -2, i);
                        }
                    }
                }
                let e = cs("<x>");
                emit(&format!("len={}", (a.js_getlength)(J, -1)));
                emit(&format!("repr={}", repr_at(a, J, -1)));
                emit(&format!("hasidx={}", (a.js_hasindex)(J, -1, poke)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_getindex)(J, -1, poke);
                emit(&format!("get={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_delindex)(J, -1, poke);
                emit(&format!("after_del={}", repr_at(a, J, -1)));
                (a.js_pushnumber)(J, 7.0);
                (a.js_setindex)(J, -2, poke);
                emit(&format!("after_set={}", repr_at(a, J, -1)));
                /* iterate own and inherited */
                for own in [0, 1] {
                    (a.js_pushiterator)(J, -1, own);
                    let mut k = 0;
                    loop {
                        let nm = (a.js_nextiterator)(J, -1);
                        if nm.is_null() {
                            break;
                        }
                        emit(&format!("it{}[{}]={:?}", own, k, rs(nm)));
                        k += 1;
                        if k > 64 {
                            break;
                        }
                    }
                    (a.js_pop)(J, 1);
                }
                emit(&format!("isarray={}", (a.js_isarray)(J, -1)));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("array iter={} mode={} n={} poke={}", iter, pi(0), pi(1), pi(2)),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

#[test]
fn setlength_boundaries() {
    for len in [-1i64, 0, 1, 2, 100, 1 << 20, (1 << 26) - 1, 1 << 26, (1 << 26) + 1, i32::MAX as i64] {
        set_pi(0, len);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_newarray)(J);
                (a.js_setlength)(J, -1, pic(0));
                emit(&format!("len={}", (a.js_getlength)(J, -1)));
                (a.js_pushnumber)(J, 1.0);
                (a.js_setindex)(J, -2, 0);
                emit(&format!("len2={}", (a.js_getlength)(J, -1)));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("setlength {}", len),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

/* --------------------------------------------------------- iterators */

#[test]
fn iterators_own_and_inherited() {
    let srcs = [
        "({a:1,b:2})",
        "[1,2,3]",
        "'abc'",
        "new String('xy')",
        "(function(){})",
        "Object.create({p:1})",
        "(function(){var o=Object.create({p:1}); o.q=2; return o})()",
        "Math",
        "JSON",
        "new Date(0)",
        "/re/g",
        "new Error('e')",
        "(function(){return arguments})(1,2)",
        "Object.defineProperty({}, 'h', {value:1, enumerable:false})",
        "[]",
        "({})",
        "null",
        "undefined",
        "42",
        "true",
    ];
    for src in srcs {
        for own in [0, 1] {
            set_ps(0, src);
            set_pi(0, own);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let src = ps(0);
                    let name = cs("it.js");
                    let full = cs(&format!("({})", ps(0).to_str().unwrap()));
                    let _ = src;
                    if (a.js_ploadstring)(J, name.as_ptr(), full.as_ptr()) != 0 {
                        emit("loadfail");
                        return;
                    }
                    (a.js_pushundefined)(J);
                    if (a.js_pcall)(J, 0) != 0 {
                        emit("callfail");
                        (a.js_pop)(J, 1);
                        (a.js_pushnumber)(J, 0.0);
                        return;
                    }
                    (a.js_pushiterator)(J, -1, pic(0));
                    let mut k = 0;
                    loop {
                        let nm = (a.js_nextiterator)(J, -1);
                        if nm.is_null() {
                            break;
                        }
                        emit(&format!("{}", rs(nm)));
                        k += 1;
                        if k > 200 {
                            break;
                        }
                    }
                    (a.js_pop)(J, 2);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            same(
                &format!("iterator {:?} own={}", src, own),
                &p.c.run_native(act, 0),
                &p.r.run_native(act, 0),
            );
        }
    }
}

/* --------------------------------------------------------- userdata */

unsafe extern "C" fn ud_has(_J: JS, _p: *mut c_void, name: *const c_char) -> c_int {
    emit(&format!("has:{}", unsafe { rs(name) }));
    let a = cur();
    if unsafe { rs(name) } == "magic" {
        unsafe { (a.js_pushnumber)(_J, 123.0) };
        return 1;
    }
    0
}
unsafe extern "C" fn ud_put(_J: JS, _p: *mut c_void, name: *const c_char) -> c_int {
    emit(&format!("put:{}", unsafe { rs(name) }));
    1
}
unsafe extern "C" fn ud_del(_J: JS, _p: *mut c_void, name: *const c_char) -> c_int {
    emit(&format!("del:{}", unsafe { rs(name) }));
    1
}
unsafe extern "C" fn ud_fin(_J: JS, _p: *mut c_void) {
    emit("finalize");
}

#[test]
fn userdata_plain_and_extended() {
    for mode in 0..2 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                let tag = cs("mytag"); /* copied by js_newuserdata? compared by strcmp */
                let other = cs("othertag");
                let data = 0xBEEF as *mut c_void;
                (a.js_newobject)(J); /* prototype for newuserdata */
                if pi(0) == 0 {
                    (a.js_newuserdata)(J, tag.as_ptr(), data, Some(ud_fin));
                } else {
                    (a.js_newuserdatax)(
                        J,
                        tag.as_ptr(),
                        data,
                        Some(ud_has),
                        Some(ud_put),
                        Some(ud_del),
                        Some(ud_fin),
                    );
                }
                let e = cs("<x>");
                emit(&format!(
                    "isud={} isud_other={} ud={:?} ud_other={:?}",
                    (a.js_isuserdata)(J, -1, tag.as_ptr()),
                    (a.js_isuserdata)(J, -1, other.as_ptr()),
                    (a.js_touserdata)(J, -1, tag.as_ptr()),
                    repr_at(a, J, -1),
                ));
                emit(&format!("typeof={} type={}", rs((a.js_typeof)(J, -1)), (a.js_type)(J, -1)));
                emit(&format!("has_magic={}", (a.js_hasproperty)(J, -1, cs("magic").as_ptr())));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                emit(&format!("has_zap={}", (a.js_hasproperty)(J, -1, cs("zap").as_ptr())));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 5.0);
                (a.js_setproperty)(J, -2, cs("zap").as_ptr());
                (a.js_delproperty)(J, -1, cs("zap").as_ptr());
                (a.js_gc)(J, 0);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("userdata mode={}", mode),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

/* --------------------------------------------------------- cfunctions */

unsafe extern "C" fn cf_sum(J: JS) {
    let a = cur();
    unsafe {
        let n = (a.js_gettop)(J);
        emit(&format!("cf_sum top={}", n));
        let mut s = 0.0;
        for i in 1..n {
            s += (a.js_trynumber)(J, i, 0.0);
        }
        (a.js_currentfunction)(J);
        emit(&format!("cur={:?}", rs((a.js_tryrepr)(J, -1, cs("<x>").as_ptr()))));
        (a.js_pop)(J, 1);
        emit(&format!("data={:?}", (a.js_currentfunctiondata)(J)));
        (a.js_pushnumber)(J, s);
    }
}

unsafe extern "C" fn cf_ctor(J: JS) {
    let a = cur();
    unsafe {
        emit(&format!("cf_ctor top={}", (a.js_gettop)(J)));
        (a.js_newobject)(J);
        (a.js_pushnumber)(J, 9.0);
        (a.js_setproperty)(J, -2, cs("made").as_ptr());
    }
}

#[test]
fn cfunctions_and_constructors() {
    for mode in 0..3 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    0 => (a.js_newcfunction)(J, Some(cf_sum), b"sum\0".as_ptr() as *const c_char, 2),
                    1 => (a.js_newcfunctionx)(
                        J,
                        Some(cf_sum),
                        b"sumx\0".as_ptr() as *const c_char,
                        2,
                        0xD00D as *mut c_void,
                        Some(ud_fin),
                    ),
                    _ => (a.js_newcconstructor)(
                        J,
                        Some(cf_sum),
                        Some(cf_ctor),
                        b"Ctor\0".as_ptr() as *const c_char,
                        1,
                    ),
                }
                let e = cs("<x>");
                emit(&format!("fn={:?}", repr_at(a, J, -1)));
                emit(&format!(
                    "callable={} typeof={} len={}",
                    (a.js_iscallable)(J, -1),
                    rs((a.js_typeof)(J, -1)),
                    (a.js_getlength)(J, -1)
                ));
                /* plain call with 3 args */
                (a.js_copy)(J, -1);
                (a.js_pushundefined)(J);
                (a.js_pushnumber)(J, 1.0);
                (a.js_pushnumber)(J, 2.0);
                (a.js_pushnumber)(J, 3.0);
                let rc = (a.js_pcall)(J, 3);
                emit(&format!("pcall={} res={:?}", rc, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                /* construct */
                (a.js_copy)(J, -1);
                let rc = (a.js_pconstruct)(J, 0);
                emit(&format!("pconstruct={} res={:?}", rc, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                /* register as a global and drive it from JS */
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("F").as_ptr());
                let src = cs("[F(1,2,3), typeof F, F.length, F.name]");
                let nm = cs("cf.js");
                if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                    (a.js_pushundefined)(J);
                    let rc = (a.js_pcall)(J, 0);
                    emit(&format!("js={} {:?}", rc, repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                }
                (a.js_gc)(J, 0);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("cfunction mode={}", mode),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

/* --------------------------------------------------------- operators */

#[test]
fn operators_over_value_pairs() {
    /* js_compare / js_equal / js_strictequal / js_instanceof / js_concat */
    let mut rng = Rng::new(SEED ^ 5);
    for iter in 0..600 {
        set_pi(0, rng.range_i64(0, 26));
        set_pi(1, rng.range_i64(0, 26));
        set_pi(2, rng.range_i64(0, 4));
        fn pushkind(a: &Api, J: JS, k: i64) {
            unsafe {
                match k {
                    0 => (a.js_pushundefined)(J),
                    1 => (a.js_pushnull)(J),
                    2 => (a.js_pushboolean)(J, 0),
                    3 => (a.js_pushboolean)(J, 1),
                    4 => (a.js_pushnumber)(J, 0.0),
                    5 => (a.js_pushnumber)(J, -0.0),
                    6 => (a.js_pushnumber)(J, 1.0),
                    7 => (a.js_pushnumber)(J, -1.0),
                    8 => (a.js_pushnumber)(J, f64::NAN),
                    9 => (a.js_pushnumber)(J, f64::INFINITY),
                    10 => (a.js_pushnumber)(J, 1e21),
                    11 => (a.js_pushstring)(J, cs("").as_ptr()),
                    12 => (a.js_pushstring)(J, cs("0").as_ptr()),
                    13 => (a.js_pushstring)(J, cs("1").as_ptr()),
                    14 => (a.js_pushstring)(J, cs("abc").as_ptr()),
                    15 => (a.js_pushstring)(J, cs("ABC").as_ptr()),
                    16 => (a.js_pushstring)(J, cs("\u{4e2d}").as_ptr()),
                    17 => (a.js_newobject)(J),
                    18 => (a.js_newarray)(J),
                    19 => (a.js_newboolean)(J, 1),
                    20 => (a.js_newnumber)(J, 1.0),
                    21 => (a.js_newstring)(J, cs("abc").as_ptr()),
                    22 => (a.js_newregexp)(J, cs("x").as_ptr(), 0),
                    23 => (a.js_pushglobal)(J),
                    24 => (a.js_newcfunction)(J, None, b"z\0".as_ptr() as *const c_char, 0),
                    _ => (a.js_pushliteral)(J, lit_lit()),
                }
            }
        }
        fn act(a: &Api, J: JS) {
            unsafe {
                pushkind(a, J, pi(0));
                pushkind(a, J, pi(1));
                let e = cs("<x>");
                match pi(2) {
                    0 => {
                        let mut okay: c_int = -5;
                        let r = (a.js_compare)(J, &mut okay);
                        emit(&format!("compare={} okay={}", r, okay));
                    }
                    1 => emit(&format!("equal={}", (a.js_equal)(J))),
                    2 => emit(&format!("strictequal={}", (a.js_strictequal)(J))),
                    3 => {
                        (a.js_concat)(J);
                        emit(&format!("concat={:?}", repr_at(a, J, -1)));
                    }
                    _ => emit(&format!("instanceof={}", (a.js_instanceof)(J))),
                }
                dump(a, J);
                (a.js_pop)(J, (a.js_gettop)(J));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("operator iter={} a={} b={} op={}", iter, pi(0), pi(1), pi(2)),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

#[test]
fn instanceof_realistic() {
    let srcs = [
        "[] instanceof Array",
        "[] instanceof Object",
        "({}) instanceof Array",
        "(function(){}) instanceof Function",
        "new Date(0) instanceof Date",
        "'x' instanceof String",
        "new String('x') instanceof String",
        "1 instanceof Number",
        "function C(){}; new C() instanceof C",
        "function C(){}; C.prototype = 1; try { ({}) instanceof C } catch(e) { e.name }",
        "try { ({}) instanceof 1 } catch(e) { e.name }",
        "try { ({}) instanceof {} } catch(e) { e.name }",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("instanceof", s, f);
        }
    }
}

/* --------------------------------------------------- repr / registry / ref */

#[test]
fn repr_family() {
    fn act(a: &Api, J: JS) {
        unsafe {
            push_zoo(a, J);
            /* cyclic structures */
            (a.js_newobject)(J);
            (a.js_copy)(J, -1);
            (a.js_setproperty)(J, -2, cs("self").as_ptr());
            (a.js_newarray)(J);
            (a.js_copy)(J, -1);
            (a.js_setindex)(J, -2, 0);
            let n = (a.js_gettop)(J);
            let e = cs("<ERR>");
            for i in 0..n {
                emit(&format!("torepr[{}]={:?}", i, repr_at(a, J, i)));
                (a.js_repr)(J, i);
                emit(&format!("repr[{}]={:?}", i, str_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in FLAGSETS {
        diff_native("repr", act, f);
    }
}

#[test]
fn registry_and_refs() {
    fn act(a: &Api, J: JS) {
        unsafe {
            let e = cs("<x>");
            (a.js_pushnumber)(J, 11.0);
            (a.js_setregistry)(J, cs("k1").as_ptr());
            (a.js_getregistry)(J, cs("k1").as_ptr());
            emit(&format!("k1={:?}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
            (a.js_getregistry)(J, cs("nope").as_ptr());
            emit(&format!("nope={:?}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
            (a.js_delregistry)(J, cs("k1").as_ptr());
            (a.js_getregistry)(J, cs("k1").as_ptr());
            emit(&format!("k1after={:?}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
            /* js_ref / js_unref round trip for several value kinds */
            for k in 0..6 {
                match k {
                    0 => (a.js_newobject)(J),
                    1 => (a.js_pushnumber)(J, 3.5),
                    2 => (a.js_pushstring)(J, cs("refstr").as_ptr()),
                    3 => (a.js_newarray)(J),
                    4 => (a.js_pushundefined)(J),
                    _ => (a.js_pushnull)(J),
                }
                let r = (a.js_ref)(J);
                emit(&format!("ref{}={:?}", k, rs(r)));
                (a.js_getregistry)(J, r);
                emit(&format!("deref{}={:?}", k, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_unref)(J, r);
                (a.js_getregistry)(J, r);
                emit(&format!("afterunref{}={:?}", k, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            (a.js_gc)(J, 0);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in FLAGSETS {
        let p = libs();
        /* js_ref names objects by their heap address, which differs per library */
        same(
            &format!("registry flags={}", f),
            &mask_ptrs(&p.c.run_native(act, f)),
            &mask_ptrs(&p.r.run_native(act, f)),
        );
    }
}

/* ---------------------------------------------- allocation / interning */

#[test]
fn malloc_realloc_intern_strdup() {
    fn act(a: &Api, J: JS) {
        unsafe {
            let p1 = (a.js_malloc)(J, 32);
            emit(&format!("malloc_nonnull={}", !p1.is_null()));
            let p2 = (a.js_realloc)(J, p1, 64);
            emit(&format!("realloc_nonnull={}", !p2.is_null()));
            (a.js_free)(J, p2);
            let s1 = (a.js_strdup)(J, cs("hello").as_ptr());
            emit(&format!("strdup={:?}", rs(s1)));
            (a.js_free)(J, s1 as *mut c_void);
            let i1 = (a.js_intern)(J, cs("interned").as_ptr());
            let i2 = (a.js_intern)(J, cs("interned").as_ptr());
            emit(&format!("intern_same={} val={:?}", i1 == i2, rs(i1)));
            /* interning many strings exercises the string table growth */
            for k in 0..200 {
                let s = cs(&format!("str{}", k));
                let _ = (a.js_intern)(J, s.as_ptr());
            }
            (a.js_gc)(J, 0);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("alloc", act, f);
    }
}

/* ------------------------------------------------ stdout-producing calls */

#[test]
fn gc_report_and_dumpstrings_stdout() {
    let p = libs();
    for report in [0, 1] {
        for f in [0, JS_STRICT] {
            set_pi(0, report);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let src = cs("var a=[]; for(var i=0;i<50;i++) a.push({x:i,s:'s'+i}); a.length");
                    let nm = cs("gc.js");
                    if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                        (a.js_pushundefined)(J);
                        let _ = (a.js_pcall)(J, 0);
                        (a.js_pop)(J, 1);
                    }
                    (a.js_gc)(J, pic(0));
                    (a.js_gc)(J, pic(0));
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let oc = capture_stdout(|| {
                let _ = p.c.run_native(act, f);
            });
            let or_ = capture_stdout(|| {
                let _ = p.r.run_native(act, f);
            });
            /* gc report prints byte counts which are allocator dependent; compare
             * the message shape (all non-digit characters) plus the line count. */
            let strip = |s: &str| {
                s.lines()
                    .map(|l| l.chars().filter(|c| !c.is_ascii_digit()).collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            same(
                &format!("js_gc(report={}) flags={}", report, f),
                &strip(&oc),
                &strip(&or_),
            );
        }
    }
}

#[test]
fn dumpstrings_stdout() {
    let p = libs();
    fn act(a: &Api, J: JS) {
        unsafe {
            for k in 0..40 {
                let s = cs(&format!("key{}", k * 7 % 40));
                let _ = (a.js_intern)(J, s.as_ptr());
            }
            (a.jsS_dumpstrings)(J);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    let oc = capture_stdout(|| {
        let _ = p.c.run_native(act, 0);
    });
    let or_ = capture_stdout(|| {
        let _ = p.r.run_native(act, 0);
    });
    same("jsS_dumpstrings", &oc, &or_);
}

#[test]
fn trap_stdout() {
    let p = libs();
    for pc in [-1i64, 0, 1, 5] {
        set_pi(0, pc);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_pushnumber)(J, 1.0);
                (a.js_pushstring)(J, cs("two").as_ptr());
                (a.js_newobject)(J);
                (a.js_trap)(J, pic(0));
                (a.js_pop)(J, 3);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let oc = capture_stdout(|| {
            let _ = p.c.run_native(act, 0);
        });
        let or_ = capture_stdout(|| {
            let _ = p.r.run_native(act, 0);
        });
        /* addresses differ between the two libraries: mask hex pointers */
        let mask = |s: &str| {
            let mut out = String::new();
            let mut it = s.chars().peekable();
            while let Some(c) = it.next() {
                if c == '0' && it.peek() == Some(&'x') {
                    it.next();
                    while it.peek().map_or(false, |c| c.is_ascii_hexdigit()) {
                        it.next();
                    }
                    out.push_str("0xPTR");
                } else {
                    out.push(c);
                }
            }
            out
        };
        same(&format!("js_trap({})", pc), &mask(&oc), &mask(&or_));
    }
}

/* ------------------------------------------------------- eval entry points */

#[test]
fn eval_entry_points() {
    let srcs = [
        "1+1",
        "var x = 5; x*2",
        "throw new Error('boom')",
        "var 1",
        "function f(){return 3} f()",
        "(function(){ return arguments.length })(1,2,3)",
        "this",
        "eval('2+2')",
        "",
        "   ",
        "//comment",
        "/*unterminated",
        "'\\u0041'",
        "return 5",
    ];
    for src in srcs {
        for f in [0, JS_STRICT] {
            set_ps(0, src);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let src = ps(0);
                    let nm = cs("e.js");
                    let e = cs("<x>");
                    /* js_dostring */
                    let rc = (a.js_dostring)(J, src.as_ptr());
                    emit(&format!("dostring={}", rc));
                    /* js_ploadstring + js_pcall */
                    let rc = (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr());
                    emit(&format!("ploadstring={}", rc));
                    if rc == 0 {
                        (a.js_pushundefined)(J);
                        let rc = (a.js_pcall)(J, 0);
                        emit(&format!("pcall={} {:?}", rc, repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    } else {
                        emit(&format!("err={:?}", str_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    /* js_loadstring (throws on syntax error) inside this pcall */
                    (a.js_loadstring)(J, nm.as_ptr(), src.as_ptr());
                    emit(&format!("loadstring_ok={:?}", repr_at(a, J, -1)));
                    (a.js_pushundefined)(J);
                    (a.js_call)(J, 0);
                    emit(&format!("call={:?}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    /* js_loadeval + js_eval */
                    (a.js_pushstring)(J, src.as_ptr());
                    (a.js_eval)(J);
                    emit(&format!("eval={:?}", repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            /* js_dostring reports errors to stderr; capture stdout for parity too */
            same(
                &format!("eval entrypoints {:?} flags={}", src, f),
                &p.c.run_native(act, f),
                &p.r.run_native(act, f),
            );
        }
    }
}

/* --------------------------------------------------------- regexp API */

#[test]
fn newregexp_all_flag_combinations() {
    let pats = ["a+b", "(a)(b)", "^x$", "[a-z]+", "\\bword\\b", "", "a|b", "\\d{2,3}"];
    for pat in pats {
        for flags in 0..8 {
            set_ps(0, pat);
            set_pi(0, flags);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let pat = ps(0);
                    (a.js_newregexp)(J, pat.as_ptr(), pic(0));
                    let e = cs("<x>");
                    emit(&format!("re={:?}", repr_at(a, J, -1)));
                    emit(&format!(
                        "isregexp={} typeof={} rx={:?}",
                        (a.js_isregexp)(J, -1),
                        rs((a.js_typeof)(J, -1)),
                        !(a.js_toregexp)(J, -1).is_null()
                    ));
                    for k in ["source", "global", "ignoreCase", "multiline", "lastIndex"] {
                        (a.js_getproperty)(J, -1, cs(k).as_ptr());
                        emit(&format!("{}={:?}", k, repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    /* drive exec/test repeatedly to observe lastIndex with /g */
                    (a.js_copy)(J, -1);
                    (a.js_setglobal)(J, cs("RE").as_ptr());
                    let src = cs("var s='aabab xaxb word ab'; var o=[]; for(var i=0;i<4;i++){o.push(String(RE.exec(s))); o.push(RE.lastIndex); o.push(RE.test(s)); o.push(RE.lastIndex);} o.join(',')");
                    let nm = cs("re.js");
                    if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                        (a.js_pushundefined)(J);
                        let rc = (a.js_pcall)(J, 0);
                        emit(&format!("drive={} {:?}", rc, repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            same(
                &format!("js_newregexp {:?} flags={}", pat, flags),
                &p.c.run_native(act, 0),
                &p.r.run_native(act, 0),
            );
        }
    }
}

/* ------------------------------------------------- jsV_* object level API */

/* Classes whose bare (jsV_newobject) instance can be stringified without
 * dereferencing the class payload that only the real constructors fill in.
 * JS_CFUNCTION/CSCRIPT/CCFUNCTION/CSTRING/CREGEXP/CITERATOR/CUSERDATA all
 * deref a NULL payload in BOTH libraries (identical C behaviour: a crash),
 * so they are exercised without repr/tostring. */
fn class_payload_safe(c: i64) -> bool {
    matches!(c, 0 | 1 | 5 | 6 | 7 | 10 | 11 | 12 | 13)
}

#[test]
fn jsV_object_and_property_level() {
    for class in 0..16 {
        set_pi(0, class);
        set_pi(3, class_payload_safe(class) as i64);
        fn act(a: &Api, J: JS) {
            unsafe {
                let obj = (a.jsV_newobject)(J, pic(0), std::ptr::null_mut());
                emit(&format!("obj_nonnull={}", !obj.is_null()));
                (a.js_pushobject)(J, obj);
                let e = cs("<x>");
                if pi(3) != 0 {
                    emit(&format!("repr={:?}", repr_at(a, J, -1)));
                }
                emit(&format!("typeof={} type={}", rs((a.js_typeof)(J, -1)), (a.js_type)(J, -1)));
                /* jsV_setproperty / getownproperty / getproperty / getpropertyx */
                for name in ["p", "q", "0", "1", "length"] {
                    let n = cs(name);
                    let prop = (a.jsV_setproperty)(J, obj, n.as_ptr());
                    emit(&format!("set {} nonnull={}", name, !prop.is_null()));
                    let own = (a.jsV_getownproperty)(J, obj, n.as_ptr());
                    emit(&format!("own {} nonnull={}", name, !own.is_null()));
                    let mut isown: c_int = -1;
                    let px = (a.jsV_getpropertyx)(J, obj, n.as_ptr(), &mut isown);
                    emit(&format!("x {} nonnull={} own={}", name, !px.is_null(), isown));
                    let g = (a.jsV_getproperty)(J, obj, n.as_ptr());
                    emit(&format!("get {} nonnull={}", name, !g.is_null()));
                }
                (a.jsV_delproperty)(J, obj, cs("p").as_ptr());
                emit(&format!(
                    "after_del={}",
                    !(a.jsV_getownproperty)(J, obj, cs("p").as_ptr()).is_null()
                ));
                /* jsV_newiterator / jsV_nextiterator */
                for own in [0, 1] {
                    let it = (a.jsV_newiterator)(J, obj, own);
                    let mut k = 0;
                    loop {
                        let nm = (a.jsV_nextiterator)(J, it);
                        if nm.is_null() {
                            break;
                        }
                        emit(&format!("it{}={:?}", own, rs(nm)));
                        k += 1;
                        if k > 64 {
                            break;
                        }
                    }
                }
                /* jsV_resizearray only makes sense for arrays but the C never checks */
                if pi(0) == 1 {
                    for n in [0, 1, 5, 2] {
                        (a.jsV_resizearray)(J, obj, n);
                        emit(&format!("resize {} len={}", n, (a.js_getlength)(J, -1)));
                    }
                }
                /* value level conversions via js_tovalue */
                let v = (a.js_tovalue)(J, -1);
                emit(&format!(
                    "v: bool={} num={:#x} int={:#x}",
                    (a.jsV_toboolean)(J, v),
                    (a.jsV_tonumber)(J, v).to_bits(),
                    (a.jsV_tointeger)(J, v).to_bits()
                ));
                (a.js_gc)(J, 0);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("jsV_ class={}", class),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

#[test]
fn jsV_value_conversions_over_zoo() {
    fn act(a: &Api, J: JS) {
        unsafe {
            push_zoo(a, J);
            let n = (a.js_gettop)(J);
            for i in 0..n {
                let v = (a.js_tovalue)(J, i);
                emit(&format!(
                    "[{}] bool={} num={:#x} int={:#x}",
                    i,
                    (a.jsV_toboolean)(J, v),
                    (a.jsV_tonumber)(J, v).to_bits(),
                    (a.jsV_tointeger)(J, v).to_bits()
                ));
                /* jsV_tostring must not be called on objects with throwing
                 * toString; the zoo has none, so this is the valid path */
                emit(&format!("str={:?}", rs((a.jsV_tostring)(J, v))));
            }
            /* jsV_newmemstring */
            for s in ["", "x", "0123456789ABCDEF"] {
                let cstr = cs(s);
                let ms = (a.jsV_newmemstring)(J, cstr.as_ptr(), cstr.as_bytes().len() as c_int);
                emit(&format!("memstr {:?} nonnull={}", s, !ms.is_null()));
            }
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("jsV conversions", act, f);
    }
}

#[test]
fn toprimitive_hints() {
    /* js_toprimitive hint: JS_TNUMBER(4) / JS_TSTRING-ish(any) / 0 */
    for hint in [-1i64, 0, 4, 5, 8, 99] {
        set_pi(0, hint);
        fn act(a: &Api, J: JS) {
            unsafe {
                let e = cs("<x>");
                for k in 0..6 {
                    match k {
                        0 => (a.js_newobject)(J),
                        1 => (a.js_newarray)(J),
                        2 => (a.js_newnumber)(J, 5.0),
                        3 => (a.js_newstring)(J, cs("s").as_ptr()),
                        4 => (a.js_newboolean)(J, 1),
                        _ => (a.js_newregexp)(J, cs("r").as_ptr(), 0),
                    }
                    (a.js_toprimitive)(J, -1, pic(0));
                    emit(&format!("[{}]={:?}", k, repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                }
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("toprimitive hint={}", hint),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

#[test]
fn newarguments_and_currentfunction() {
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_newarguments)(J);
            let e = cs("<x>");
            emit(&format!("args={:?}", repr_at(a, J, -1)));
            (a.js_currentfunction)(J);
            emit(&format!("curfn={:?}", repr_at(a, J, -1)));
            emit(&format!("data={:?}", (a.js_currentfunctiondata)(J)));
            (a.js_pop)(J, 2);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("newarguments", act, f);
    }
}
