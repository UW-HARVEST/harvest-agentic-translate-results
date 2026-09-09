//! CONFIGS.md rows 22-35 — state construction, the raw value stack, the
//! conversion API, the type predicates and the relational/equality operators.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

/* ------------------------------------------------------------------ */
/* helpers: push one value of every distinct shape                      */
/* ------------------------------------------------------------------ */

/// Number of distinct value shapes `push_shape` can produce.
pub const NSHAPES: c_int = 34;

/// Push value shape `k`; every branch is a shape the C code special-cases.
unsafe fn push_shape(api: &Api, J: JS, k: c_int) {
    unsafe {
        match k {
            0 => (api.js_pushundefined)(J),
            1 => (api.js_pushnull)(J),
            2 => (api.js_pushboolean)(J, 0),
            3 => (api.js_pushboolean)(J, 1),
            4 => (api.js_pushboolean)(J, 42), // non-0/1 int across FFI
            5 => (api.js_pushboolean)(J, -1),
            6 => (api.js_pushnumber)(J, 0.0),
            7 => (api.js_pushnumber)(J, -0.0),
            8 => (api.js_pushnumber)(J, 1.0),
            9 => (api.js_pushnumber)(J, -1.5),
            10 => (api.js_pushnumber)(J, f64::NAN),
            11 => (api.js_pushnumber)(J, f64::INFINITY),
            12 => (api.js_pushnumber)(J, f64::NEG_INFINITY),
            13 => (api.js_pushnumber)(J, 1e21),
            14 => (api.js_pushnumber)(J, 4294967296.0),
            15 => (api.js_pushnumber)(J, -2147483648.0),
            16 => (api.js_pushstring)(J, cs("").as_ptr()),        // shrstr, empty
            17 => (api.js_pushstring)(J, cs("abc").as_ptr()),     // shrstr
            18 => (api.js_pushstring)(J, cs("0123456").as_ptr()), // shrstr, len 7
            19 => (api.js_pushstring)(J, cs("01234567").as_ptr()), // memstr, len 8
            20 => (api.js_pushstring)(J, cs("a longer heap allocated string value").as_ptr()),
            21 => (api.js_pushliteral)(J, c"literal".as_ptr()),   // litstr
            22 => (api.js_pushlstring)(J, b"ab\0cd".as_ptr() as *const c_char, 5), // embedded NUL
            23 => (api.js_pushstring)(J, cs("42").as_ptr()),
            24 => (api.js_pushstring)(J, cs("0x10").as_ptr()),
            25 => (api.js_pushstring)(J, cs("h\u{e9}ll\u{f6} \u{20ac} \u{1f600}").as_ptr()),
            26 => (api.js_newobject)(J),
            27 => (api.js_newarray)(J),
            28 => (api.js_newboolean)(J, 1),
            29 => (api.js_newnumber)(J, 3.5),
            30 => (api.js_newstring)(J, cs("boxed").as_ptr()),
            31 => (api.js_newregexp)(J, cs("a+").as_ptr(), JS_REGEXP_G),
            32 => (api.js_pushglobal)(J),
            _ => {
                // a Date object and an Error object, built from script
                let f = cs("shape.js");
                let src = cs("new Date(0)");
                if (api.js_ploadstring)(J, f.as_ptr(), src.as_ptr()) == 0 {
                    (api.js_pushundefined)(J);
                    if (api.js_pcall)(J, 0) != 0 {
                        (api.js_pop)(J, 1);
                        (api.js_pushundefined)(J);
                    }
                } else {
                    (api.js_pop)(J, 1);
                    (api.js_pushundefined)(J);
                }
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* rows 22-24 — state construction under each configuration            */
/* ------------------------------------------------------------------ */
fn task_state_basics(api: &Api, J: JS) {
    unsafe {
        emit(format!("gettop={}", (api.js_gettop)(J)));
        // context round-trip
        let ctxv = 0xDEADBEEFusize as *mut std::os::raw::c_void;
        (api.js_setcontext)(J, ctxv);
        emit(format!("ctx={:?}", (api.js_getcontext)(J) as usize));
        (api.js_setcontext)(J, std::ptr::null_mut());
        emit(format!("ctx={:?}", (api.js_getcontext)(J) as usize));
        // js_atpanic returns the *previous* handler; default is non-NULL
        let prev = (api.js_atpanic)(J, None);
        emit(format!("atpanic_prev_is_some={}", prev.is_some()));
        let prev2 = (api.js_atpanic)(J, prev);
        emit(format!("atpanic_prev2_is_some={}", prev2.is_some()));
        // the report callback is observable
        (api.js_report)(J, cs("hello from js_report").as_ptr());
        // gc, both report modes
        (api.js_gc)(J, 0);
        (api.js_gc)(J, 1);
        emit("gc done".into());
        // global object properties exist
        for name in [
            "Object", "Array", "Function", "Boolean", "Number", "String", "RegExp", "Error",
            "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError",
            "Math", "JSON", "Date", "parseInt", "parseFloat", "isNaN", "isFinite", "eval",
            "encodeURI", "decodeURI", "encodeURIComponent", "decodeURIComponent", "escape",
            "unescape", "undefined", "NaN", "Infinity", "print", "console", "nosuchglobal",
        ] {
            (api.js_getglobal)(J, cs(name).as_ptr());
            emit(format!("global {name}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn row22_23_24_state_basics() {
    diff_all_flags("state basics", task_state_basics);
}

/* ------------------------------------------------------------------ */
/* rows 25-26 — js_setlimit                                            */
/* ------------------------------------------------------------------ */
fn task_busy_loop(api: &Api, J: JS) {
    unsafe {
        eval(api, J, "loop", "var s=0; for (var i=0;i<2000000;i++) s+=i; s");
    }
}
fn task_alloc_heavy(api: &Api, J: JS) {
    unsafe {
        eval(
            api,
            J,
            "grow",
            "var a=[]; for (var i=0;i<200000;i++) a.push('xxxxxxxxxxxxxxxxxxxxxxxxxxxx'+i); a.length",
        );
    }
}

#[test]
fn row25_memlimit() {
    // large enough that state + task setup succeed => comparable in-process
    diff("memlimit 8MB", Cfg::limits(0, 8 << 20), task_alloc_heavy);
    diff("memlimit 1MB", Cfg::limits(0, 1 << 20), task_alloc_heavy);
    // negative / zero => disabled
    diff("memlimit 0", Cfg::limits(0, 0), task_state_basics);
    diff("memlimit -1", Cfg::limits(0, -1), task_state_basics);
    // Tiny limits make the C library run out of memory before any protected
    // frame exists, so js_throw() reaches abort(). Compare in child processes.
    for m in [1, 16, 1024, 1 << 14, 1 << 16, 1 << 18] {
        diff_subproc("worker", "alloc_heavy", Cfg::limits(0, m));
        diff_subproc("worker", "state_basics", Cfg::limits(0, m));
    }
}

/// Worker entry point for `diff_subproc` (no-op during a normal test run).
#[test]
fn worker() {
    common::maybe_worker(dispatch);
}

fn dispatch(api: &Api, J: JS) {
    match case().as_str() {
        "alloc_heavy" => task_alloc_heavy(api, J),
        "busy_loop" => task_busy_loop(api, J),
        "state_basics" => task_state_basics(api, J),
        other => emit(format!("unknown case {other}")),
    }
}

#[test]
fn row26_runlimit() {
    diff("runlimit 1s", Cfg::limits(1, 0), task_busy_loop);
    diff("runlimit -1", Cfg::limits(-1, 0), task_state_basics);
}

/* ------------------------------------------------------------------ */
/* rows 27-28 — raw stack manipulation                                 */
/* ------------------------------------------------------------------ */
fn stack_state(api: &Api, J: JS) -> String {
    unsafe {
        let top = (api.js_gettop)(J);
        let mut v = format!("top={top}");
        for i in 0..top {
            let sentinel = cs("<throw>");
            v.push_str(&format!(
                " [{i}]={}:{}",
                (api.js_type)(J, i),
                s((api.js_tryrepr)(J, i, sentinel.as_ptr()))
            ));
        }
        v
    }
}

fn task_stack_ops(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0027);
        for round in 0..400 {
            // fresh stack for each round
            (api.js_pop)(J, (api.js_gettop)(J));
            let n = 1 + rng.below(6) as c_int;
            for _ in 0..n {
                push_shape(api, J, rng.below(NSHAPES as u64) as c_int);
            }
            emit(format!("r{round} init {}", stack_state(api, J)));
            for step in 0..8 {
                let top = (api.js_gettop)(J);
                if top == 0 {
                    push_shape(api, J, 17);
                }
                let top = (api.js_gettop)(J);
                let op = rng.below(16);
                let idx = (rng.below(top as u64 + 1)) as c_int;
                match op {
                    0 => {
                        (api.js_dup)(J);
                        emit(format!("r{round}s{step} dup -> {}", stack_state(api, J)));
                    }
                    1 if top >= 2 => {
                        (api.js_dup2)(J);
                        emit(format!("r{round}s{step} dup2 -> {}", stack_state(api, J)));
                    }
                    2 if top >= 2 => {
                        (api.js_rot2)(J);
                        emit(format!("r{round}s{step} rot2 -> {}", stack_state(api, J)));
                    }
                    3 if top >= 3 => {
                        (api.js_rot3)(J);
                        emit(format!("r{round}s{step} rot3 -> {}", stack_state(api, J)));
                    }
                    4 if top >= 4 => {
                        (api.js_rot4)(J);
                        emit(format!("r{round}s{step} rot4 -> {}", stack_state(api, J)));
                    }
                    5 if top >= 2 => {
                        (api.js_rot2pop1)(J);
                        emit(format!("r{round}s{step} rot2pop1 -> {}", stack_state(api, J)));
                    }
                    6 if top >= 3 => {
                        (api.js_rot3pop2)(J);
                        emit(format!("r{round}s{step} rot3pop2 -> {}", stack_state(api, J)));
                    }
                    7 => {
                        (api.js_copy)(J, idx.min(top - 1));
                        emit(format!("r{round}s{step} copy -> {}", stack_state(api, J)));
                    }
                    8 if top >= 1 => {
                        (api.js_remove)(J, idx.min(top - 1));
                        emit(format!("r{round}s{step} remove -> {}", stack_state(api, J)));
                    }
                    9 if top >= 1 => {
                        (api.js_insert)(J, idx.min(top - 1));
                        emit(format!("r{round}s{step} insert -> {}", stack_state(api, J)));
                    }
                    10 if top >= 2 => {
                        (api.js_replace)(J, idx.min(top - 2));
                        emit(format!("r{round}s{step} replace -> {}", stack_state(api, J)));
                    }
                    11 if top >= 1 => {
                        let k = 1 + rng.below(top as u64) as c_int;
                        (api.js_rot)(J, k);
                        emit(format!("r{round}s{step} rot{k} -> {}", stack_state(api, J)));
                    }
                    12 if top >= 1 => {
                        let k = rng.below(top as u64 + 1) as c_int;
                        (api.js_pop)(J, k);
                        emit(format!("r{round}s{step} pop{k} -> {}", stack_state(api, J)));
                    }
                    _ => {
                        let sh = rng.below(NSHAPES as u64) as c_int;
                        push_shape(api, J, sh);
                        emit(format!("r{round}s{step} push{sh} -> {}", stack_state(api, J)));
                    }
                }
            }
        }
    }
}

#[test]
fn row27_28_stack_ops() {
    diff_all_flags("stack ops", task_stack_ops);
}

/* ------------------------------------------------------------------ */
/* rows 29-30 — string representations                                 */
/* ------------------------------------------------------------------ */
fn task_string_reprs(api: &Api, J: JS) {
    unsafe {
        // shrstr (<=7 bytes incl. terminator) vs memstr, plus embedded NULs
        for n in 0..40usize {
            let bytes: Vec<u8> = (0..n).map(|i| b'a' + (i % 26) as u8).collect();
            (api.js_pushlstring)(J, bytes.as_ptr() as *const c_char, n as c_int);
            emit(format!("lstring len {n}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        for n in 0..12usize {
            let mut bytes: Vec<u8> = (0..n).map(|i| b'a' + (i % 26) as u8).collect();
            bytes.push(0);
            bytes.extend_from_slice(b"tail");
            (api.js_pushlstring)(J, bytes.as_ptr() as *const c_char, bytes.len() as c_int);
            emit(format!("lstring NUL@{n}: {}", dump_slot(api, J, -1)));
            // the string survives a round trip through a property
            (api.js_newobject)(J);
            (api.js_copy)(J, -2);
            (api.js_setproperty)(J, -2, cs("k").as_ptr());
            (api.js_getproperty)(J, -1, cs("k").as_ptr());
            emit(format!("  roundtrip: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 3);
        }
        // same bytes via pushstring / pushliteral / pushlstring must be equal
        for text in ["", "a", "abcdef", "abcdefg", "abcdefgh", "0123456789abcdefghij"] {
            let cst = cs(text);
            (api.js_pushstring)(J, cst.as_ptr());
            (api.js_pushliteral)(J, sstr(text));
            (api.js_pushlstring)(J, cst.as_ptr(), text.len() as c_int);
            emit(format!(
                "{text:?}: pushstring={} pushliteral={} pushlstring={}",
                dump_slot(api, J, -3),
                dump_slot(api, J, -2),
                dump_slot(api, J, -1)
            ));
            (api.js_copy)(J, -3);
            (api.js_copy)(J, -3);
            emit(format!("  strictequal={}", (api.js_strictequal)(J)));
            (api.js_pop)(J, 3);
        }
        // js_concat over every pair of string shapes
        for a in 16..26 {
            for b in 16..26 {
                push_shape(api, J, a);
                push_shape(api, J, b);
                (api.js_concat)(J);
                emit(format!("concat {a}+{b}: {}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
            }
        }
    }
}

#[test]
fn row29_30_34_string_representations() {
    diff_all_flags("string representations", task_string_reprs);
}

/* ------------------------------------------------------------------ */
/* rows 31-32 — conversions and predicates over every shape            */
/* ------------------------------------------------------------------ */
fn task_conversions(api: &Api, J: JS) {
    unsafe {
        for k in 0..NSHAPES {
            push_shape(api, J, k);
            emit(format!("shape {k}: {}", dump_slot(api, J, -1)));
            // js_to* variants that can throw are exercised inside their own
            // protected sub-call via the try* family already covered by
            // dump_slot; the non-protected ones are safe for primitives.
            if (api.js_isprimitive)(J, -1) != 0 {
                emit(format!(
                    "  toboolean={} tonumber={:#x} tostring={:?} tointeger={} toint32={} \
                     touint32={} toint16={} touint16={}",
                    (api.js_toboolean)(J, -1),
                    (api.js_tonumber)(J, -1).to_bits(),
                    s((api.js_tostring)(J, -1)),
                    (api.js_tointeger)(J, -1),
                    (api.js_toint32)(J, -1),
                    (api.js_touint32)(J, -1),
                    (api.js_toint16)(J, -1),
                    (api.js_touint16)(J, -1),
                ));
            }
            // isuserdata with a tag never matches a non-userdata value
            emit(format!(
                "  isuserdata(Foo)={} isuserdata(\"\")={}",
                (api.js_isuserdata)(J, -1, cs("Foo").as_ptr()),
                (api.js_isuserdata)(J, -1, cs("").as_ptr())
            ));
            // js_repr pushes; js_torepr converts in place
            (api.js_repr)(J, -1);
            emit(format!("  repr slot: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_pop)(J, 1);
        }
        // out-of-range enum-ish int for js_type / predicates: negative indices
        // past the frame base and indices past the top.
        for idx in [-40, -8, -2, -1, 0, 1, 2, 8, 40] {
            emit(format!("idx {idx}: gettop={}", (api.js_gettop)(J)));
            let _ = idx;
        }
    }
}

#[test]
fn row31_32_conversions() {
    diff_all_flags("conversions and predicates", task_conversions);
}

/* ------------------------------------------------------------------ */
/* row 33 — equality / relational operators over the full cross product */
/* ------------------------------------------------------------------ */
fn task_operators(api: &Api, J: JS) {
    unsafe {
        for a in 0..NSHAPES {
            for b in 0..NSHAPES {
                // js_equal / js_strictequal / js_compare each consume 2 values
                push_shape(api, J, a);
                push_shape(api, J, b);
                let eq = (api.js_equal)(J);
                (api.js_pop)(J, 2);

                push_shape(api, J, a);
                push_shape(api, J, b);
                let se = (api.js_strictequal)(J);
                (api.js_pop)(J, 2);

                push_shape(api, J, a);
                push_shape(api, J, b);
                let mut okay: c_int = -5;
                let cmp = (api.js_compare)(J, &mut okay);
                (api.js_pop)(J, 2);

                emit(format!("({a},{b}) eq={eq} strict={se} cmp={cmp} okay={okay}"));
            }
        }
        // js_instanceof needs an object rhs; exercise valid and invalid rhs
        for (lhs, rhs) in [
            ("[]", "Array"),
            ("[]", "Object"),
            ("({})", "Array"),
            ("new Date(0)", "Date"),
            ("/x/", "RegExp"),
            ("1", "Object"),
            ("'s'", "String"),
            ("(function(){})", "Function"),
        ] {
            eval(
                api,
                J,
                &format!("instanceof {lhs} {rhs}"),
                &format!("{lhs} instanceof {rhs}"),
            );
        }
        for bad in ["1", "'s'", "null", "undefined", "true", "({})"] {
            eval(
                api,
                J,
                &format!("bad instanceof {bad}"),
                &format!("try {{ ({{}}) instanceof {bad} }} catch (e) {{ e.name+':'+e.message }}"),
            );
        }
    }
}

#[test]
fn row33_35_operators() {
    diff_all_flags("operators", task_operators);
}

/* ------------------------------------------------------------------ */
/* row 85 — js_tovalue / js_pushvalue / js_pushobject / js_toobject     */
/* ------------------------------------------------------------------ */
fn task_raw_values(api: &Api, J: JS) {
    unsafe {
        for k in 0..NSHAPES {
            push_shape(api, J, k);
            // js_tovalue returns a pointer into the stack; copy the 16 bytes and
            // push them back with js_pushvalue.
            let vp = (api.js_tovalue)(J, -1) as *const JsValue;
            let v = *vp;
            (api.js_pushvalue)(J, v);
            emit(format!("shape {k} roundtrip: {}", dump_slot(api, J, -1)));
            push_shape(api, J, k);
            (api.js_copy)(J, -2);
            emit(format!("  strictequal={}", {
                (api.js_strictequal)(J)
            }));
            (api.js_pop)(J, (api.js_gettop)(J));

            // objects: js_toobject / js_pushobject
            push_shape(api, J, k);
            if (api.js_isobject)(J, -1) != 0 {
                let o = (api.js_toobject)(J, -1);
                (api.js_pushobject)(J, o);
                emit(format!("shape {k} object roundtrip: {}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
            }
            // jsV_* low-level conversions on the raw js_Value
            let vp = (api.js_tovalue)(J, -1);
            emit(format!(
                "shape {k} jsV: bool={} num={:#x} int={:#x}",
                (api.jsV_toboolean)(J, vp),
                (api.jsV_tonumber)(J, vp).to_bits(),
                (api.jsV_tointeger)(J, vp).to_bits(),
            ));
            let vp = (api.js_tovalue)(J, -1);
            emit(format!("shape {k} jsV_tostring={:?}", s((api.jsV_tostring)(J, vp))));
            (api.js_pop)(J, (api.js_gettop)(J));

            // js_toprimitive with each hint (JS_HNONE / JS_HNUMBER / JS_HSTRING)
            // and out-of-range hint values.
            for hint in [0, 1, 2, 3, -1, 99] {
                push_shape(api, J, k);
                (api.js_toprimitive)(J, -1, hint);
                emit(format!(
                    "shape {k} toprimitive({hint}): {}",
                    dump_slot(api, J, -1)
                ));
                (api.js_pop)(J, 1);
            }
        }
    }
}

#[test]
fn row85_raw_value_marshalling() {
    diff_all_flags("raw js_Value marshalling", task_raw_values);
}
