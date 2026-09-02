//! Phase B rows 47-53: the value-stack API and primitive conversions, driven
//! directly through the low-level `.so` exports (no script involved).
mod common;
use common::*;
use std::os::raw::{c_int, c_void};

/// Build the same stack in both libraries via `ops`, then compare snapshots.
fn diff_stack(label: &str, ops: impl Fn(&Api, JS)) {
    let p = pair();
    let mut outs = Vec::new();
    for api in [&p.c, &p.r] {
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            (api.js_setreport)(j, Some(report_cb));
            ops(api, j);
            let snap = snapshot(api, j);
            (api.js_freestate)(j);
            outs.push(snap);
        }
    }
    assert_eq!(outs[0], outs[1], "{label}");
}

/// Numeric conversion helpers on the whole stack.
fn diff_numeric_conversions(label: &str, ops: impl Fn(&Api, JS)) {
    let p = pair();
    let mut outs = Vec::new();
    for api in [&p.c, &p.r] {
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            ops(api, j);
            let top = (api.js_gettop)(j);
            let mut v = Vec::new();
            for i in 0..top {
                v.push(format!(
                    "{} {} {} {} {} {} {}",
                    (api.js_toboolean)(j, i),
                    (api.js_tonumber)(j, i),
                    (api.js_tointeger)(j, i),
                    (api.js_toint32)(j, i),
                    (api.js_touint32)(j, i),
                    (api.js_toint16)(j, i),
                    (api.js_touint16)(j, i),
                ));
            }
            (api.js_freestate)(j);
            outs.push(v);
        }
    }
    assert_eq!(outs[0], outs[1], "{label}");
}

fn push_sample_values(api: &Api, j: JS) {
    unsafe {
        (api.js_pushundefined)(j);
        (api.js_pushnull)(j);
        (api.js_pushboolean)(j, 0);
        (api.js_pushboolean)(j, 1);
        (api.js_pushboolean)(j, 7);
        (api.js_pushboolean)(j, -1);
        (api.js_pushnumber)(j, 0.0);
        (api.js_pushnumber)(j, -0.0);
        (api.js_pushnumber)(j, 1.5);
        (api.js_pushnumber)(j, f64::NAN);
        (api.js_pushnumber)(j, f64::INFINITY);
        (api.js_pushnumber)(j, f64::NEG_INFINITY);
        (api.js_pushnumber)(j, 4294967296.0);
        (api.js_pushnumber)(j, -2147483649.0);
        for s in ["", "a", "abcdefg", "abcdefgh", "0123456789abcde", "0123456789abcdef", "héllo", "\u{1f600}"] {
            let cstr = cs(s);
            (api.js_pushstring)(j, cstr.as_ptr());
        }
        for s in ["", "lit", "another literal string that is long"] {
            let cstr = cs(s);
            (api.js_pushliteral)(j, cstr.as_ptr());
        }
        (api.js_newobject)(j);
        (api.js_newobjectx)(j);
        (api.js_newarray)(j);
        (api.js_newboolean)(j, 1);
        (api.js_newnumber)(j, 3.25);
        let sv = cs("wrapped");
        (api.js_newstring)(j, sv.as_ptr());
        let pat = cs("a(b)c");
        (api.js_newregexp)(j, pat.as_ptr(), 0);
        (api.js_pushglobal)(j);
        let msg = cs("m");
        (api.js_newerror)(j, msg.as_ptr());
        (api.js_newtypeerror)(j, msg.as_ptr());
        (api.js_newrangeerror)(j, msg.as_ptr());
        (api.js_newsyntaxerror)(j, msg.as_ptr());
        (api.js_newreferenceerror)(j, msg.as_ptr());
        (api.js_newevalerror)(j, msg.as_ptr());
        (api.js_newurierror)(j, msg.as_ptr());
    }
}

#[test]
fn row47_push_and_inspect_all_types() {
    diff_stack("push/inspect all value types", |api, j| {
        push_sample_values(api, j)
    });
}

#[test]
fn row48_pushlstring_lengths() {
    let p = pair();
    let payloads: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"abcdefg".to_vec(),
        b"abcdefgh".to_vec(),
        b"0123456789abcde".to_vec(),
        b"0123456789abcdef".to_vec(),
        (0..64).map(|i| b'a' + (i % 26) as u8).collect(),
        b"a\0b".to_vec(),
        b"\0".to_vec(),
        b"\0\0\0".to_vec(),
        "héllo\0wörld".as_bytes().to_vec(),
        vec![0xff, 0x00, 0xfe],
    ];
    for pl in &payloads {
        for n in 0..=(pl.len() as c_int) {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    let buf: Vec<std::os::raw::c_char> =
                        pl.iter().map(|&b| b as std::os::raw::c_char).collect();
                    let ptr = if buf.is_empty() {
                        c"".as_ptr()
                    } else {
                        buf.as_ptr()
                    };
                    (api.js_pushlstring)(j, ptr, n);
                    let d = describe(api, j, 0);
                    // also read the bytes back
                    let raw = (api.js_tostring)(j, 0);
                    let bytes = if raw.is_null() {
                        Vec::new()
                    } else {
                        let mut v = Vec::new();
                        let mut i = 0isize;
                        loop {
                            let b = *raw.offset(i) as u8;
                            if b == 0 || i > 200 {
                                break;
                            }
                            v.push(b);
                            i += 1;
                        }
                        v
                    };
                    (api.js_freestate)(j);
                    outs.push((d, bytes));
                }
            }
            assert_eq!(outs[0], outs[1], "pushlstring({pl:02x?}, {n})");
        }
    }
}

/// Stack-index manipulation.
///
/// Domain notes taken straight from `jsrun.c`:
///  * `stackidx()` maps `idx>=0` to `BOT+idx` and `idx<0` to `TOP+idx`, then
///    returns a shared `undefined` when the result is outside `[0, TOP)`.
///    Because the lower bound is `0` and not `BOT`, a sufficiently negative
///    index reads *uninitialised* slots below the current frame (the value
///    stack is allocated with `alloc()` and never zeroed), so `idx < -depth`
///    is genuinely non-deterministic in the C and is excluded.
///  * `js_remove` / `js_replace` check against `BOT` and throw
///    `Error: stack error!` — deterministic for every index.
///  * `js_insert` always throws `Error: not implemented yet`.
///  * `js_pop` throws `Error: stack underflow!` past the frame base; a negative
///    `n` would raise TOP over uninitialised slots and is excluded.
///  * `js_rot` is entirely unchecked, so `n` is kept inside the frame.
#[test]
fn row49_index_manipulation() {
    for depth in [1usize, 2, 3, 5, 8] {
        let d = depth as c_int;
        for idx in -d..=d + 2 {
            for op in 0..4 {
                diff_protected(
                    &format!("stack op depth={depth} idx={idx} op={op}"),
                    0,
                    || {
                        move |api: &Api, j: JS| unsafe {
                            for k in 0..depth {
                                (api.js_pushnumber)(j, k as f64);
                            }
                            match op {
                                0 => (api.js_copy)(j, idx),
                                1 => (api.js_remove)(j, idx),
                                2 => {
                                    (api.js_pushnumber)(j, 99.0);
                                    (api.js_insert)(j, idx)
                                }
                                _ => {
                                    (api.js_pushnumber)(j, 99.0);
                                    (api.js_replace)(j, idx)
                                }
                            }
                            for l in snapshot(api, j) {
                                log(l);
                            }
                        }
                    },
                );
            }
        }
        // js_remove / js_replace far below the frame base: still deterministic
        // (they raise "stack error!").
        for idx in [-d - 1, -d - 5, -1000, d + 5, 1000] {
            for op in 0..2 {
                diff_protected(
                    &format!("stack error depth={depth} idx={idx} op={op}"),
                    0,
                    || {
                        move |api: &Api, j: JS| unsafe {
                            for k in 0..depth {
                                (api.js_pushnumber)(j, k as f64);
                            }
                            if op == 0 {
                                (api.js_remove)(j, idx)
                            } else {
                                (api.js_replace)(j, idx)
                            }
                            for l in snapshot(api, j) {
                                log(l);
                            }
                        }
                    },
                );
            }
        }
        // js_pop, including the underflow error
        for n in 0..=d + 3 {
            diff_protected(&format!("js_pop depth={depth} n={n}"), 0, || {
                move |api: &Api, j: JS| unsafe {
                    for k in 0..depth {
                        (api.js_pushnumber)(j, k as f64);
                    }
                    (api.js_pop)(j, n);
                    for l in snapshot(api, j) {
                        log(l);
                    }
                }
            });
        }
        // js_rot within the frame
        for n in 0..=d {
            diff_protected(&format!("js_rot depth={depth} n={n}"), 0, || {
                move |api: &Api, j: JS| unsafe {
                    for k in 0..depth {
                        (api.js_pushnumber)(j, k as f64);
                    }
                    (api.js_rot)(j, n);
                    for l in snapshot(api, j) {
                        log(l);
                    }
                }
            });
        }
    }
}

#[test]
fn row50_stack_shuffles() {
    let p = pair();
    let names: [(&str, unsafe extern "C-unwind" fn(JS)); 7] = [
        ("dup", p.c.js_dup),
        ("dup2", p.c.js_dup2),
        ("rot2", p.c.js_rot2),
        ("rot3", p.c.js_rot3),
        ("rot4", p.c.js_rot4),
        ("rot2pop1", p.c.js_rot2pop1),
        ("rot3pop2", p.c.js_rot3pop2),
    ];
    let need = [1usize, 2, 2, 3, 4, 2, 3];
    for (k, (label, _)) in names.iter().enumerate() {
        for depth in need[k]..need[k] + 4 {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                let f: unsafe extern "C-unwind" fn(JS) = match k {
                    0 => api.js_dup,
                    1 => api.js_dup2,
                    2 => api.js_rot2,
                    3 => api.js_rot3,
                    4 => api.js_rot4,
                    5 => api.js_rot2pop1,
                    _ => api.js_rot3pop2,
                };
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    for i in 0..depth {
                        (api.js_pushnumber)(j, (i * 10) as f64);
                    }
                    f(j);
                    let snap = snapshot(api, j);
                    (api.js_freestate)(j);
                    outs.push(snap);
                }
            }
            assert_eq!(outs[0], outs[1], "{label} at depth {depth}");
        }
    }
    // long random sequences of shuffles
    let mut rng = Rng::new(0x5050);
    for round in 0..200 {
        let plan: Vec<u32> = (0..12).map(|_| rng.below(7)).collect();
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                for i in 0..8 {
                    (api.js_pushnumber)(j, i as f64);
                }
                for &op in &plan {
                    // keep at least 4 slots so every shuffle stays in-domain
                    while (api.js_gettop)(j) < 4 {
                        (api.js_pushnumber)(j, -1.0);
                    }
                    match op {
                        0 => (api.js_dup)(j),
                        1 => (api.js_dup2)(j),
                        2 => (api.js_rot2)(j),
                        3 => (api.js_rot3)(j),
                        4 => (api.js_rot4)(j),
                        5 => (api.js_rot2pop1)(j),
                        _ => (api.js_rot3pop2)(j),
                    }
                }
                let snap = snapshot(api, j);
                (api.js_freestate)(j);
                outs.push(snap);
            }
        }
        assert_eq!(outs[0], outs[1], "shuffle round {round} plan={plan:?}");
    }
}

#[test]
fn row51_integer_conversions_via_stack() {
    diff_numeric_conversions("numeric conversions over mixed values", |api, j| unsafe {
        (api.js_pushundefined)(j);
        (api.js_pushnull)(j);
        (api.js_pushboolean)(j, 0);
        (api.js_pushboolean)(j, 1);
        let mut rng = Rng::new(0x5151);
        for _ in 0..300 {
            (api.js_pushnumber)(j, rng.nice_f64());
        }
        for s in [
            "", " ", "0", "-0", "1", "-1", "1.5", "1e3", "0x10", "Infinity", "-Infinity", "NaN",
            "abc", "  42  ", "4294967296", "2147483648", "-2147483649", "9007199254740993",
        ] {
            let cstr = cs(s);
            (api.js_pushstring)(j, cstr.as_ptr());
        }
    });
}

#[test]
fn row52_protected_conversions_with_throwing_values() {
    let p = pair();
    // Objects whose valueOf/toString throw exercise the js_try* fallbacks.
    let setups = [
        "({valueOf:function(){throw 'v'}})",
        "({toString:function(){throw 't'}})",
        "({valueOf:function(){throw 'v'},toString:function(){throw 't'}})",
        "({valueOf:function(){return {}},toString:function(){return {}}})",
        "({valueOf:function(){return 7}})",
        "({toString:function(){return 'seven'}})",
        "Object.create(null)",
        "({})",
        "[]",
        "[1,2]",
        "[[1],[2]]",
        "(function(){})",
        "/re/g",
        "new Date(0)",
        "new Number(5)",
        "new String('s')",
        "new Boolean(false)",
        "new Error('e')",
    ];
    for setup in setups {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                (api.js_setreport)(j, Some(report_cb));
                let _ = take_reports();
                let fname = cs("[string]");
                let src = cs(&format!("({setup})"));
                let mut ok = true;
                if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                    ok = false;
                } else {
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        ok = false;
                    }
                }
                let d = describe(api, j, -1);
                let snap = snapshot(api, j);
                (api.js_freestate)(j);
                outs.push((ok, d, snap, take_reports()));
            }
        }
        assert_eq!(outs[0], outs[1], "protected conversions for {setup}");
    }
}

#[test]
fn row53_constructors_and_repr() {
    diff_stack("object constructors + repr", |api, j| unsafe {
        (api.js_newobject)(j);
        (api.js_newobjectx)(j);
        (api.js_newarray)(j);
        (api.js_newboolean)(j, 0);
        (api.js_newboolean)(j, 1);
        (api.js_newnumber)(j, 0.0);
        (api.js_newnumber)(j, f64::NAN);
        for s in ["", "x", "a longer wrapped string value"] {
            let cstr = cs(s);
            (api.js_newstring)(j, cstr.as_ptr());
        }
        for (pat, flags) in [
            ("a", 0),
            ("a", 1),
            ("a", 2),
            ("a", 4),
            ("a", 7),
            ("(x)|y", 3),
            ("", 0),
        ] {
            let cp = cs(pat);
            (api.js_newregexp)(j, cp.as_ptr(), flags);
        }
    });
    // js_repr pushes the representation; verify it and the resulting stack.
    let p = pair();
    for setup in [
        "1", "'s'", "null", "undefined", "true", "[1,2,[3]]", "({a:1,b:'c'})", "/x/gi",
        "(function f(){})", "new Date(0)", "({a:undefined})", "[undefined,,1]",
    ] {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                let fname = cs("[string]");
                let src = cs(&format!("({setup})"));
                let mut ok = true;
                if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                    ok = false;
                } else {
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        ok = false;
                    }
                }
                let errs = cs("<throw>");
                let r = rstr((api.js_tryrepr)(j, -1, errs.as_ptr()));
                let snap = snapshot(api, j);
                (api.js_freestate)(j);
                outs.push((ok, r, snap));
            }
        }
        assert_eq!(outs[0], outs[1], "repr of {setup}");
    }
}

#[test]
fn extra_intern_and_registry_and_refs() {
    let p = pair();
    let mut outs = Vec::new();
    for api in [&p.c, &p.r] {
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            let mut v: Vec<String> = Vec::new();
            // interning: same contents must map to the same pointer
            let a = cs("interned");
            let b = cs("interned");
            let pa = (api.js_intern)(j, a.as_ptr());
            let pb = (api.js_intern)(j, b.as_ptr());
            v.push(format!("intern-same-ptr={}", pa == pb));
            v.push(format!("intern-text={}", rstr(pa)));
            // js_ref / js_unref round-trip
            let mut refs = Vec::new();
            for i in 0..20 {
                (api.js_pushnumber)(j, i as f64);
                let r = rstr((api.js_ref)(j));
                refs.push(r);
            }
            v.push(format!("refs={refs:?}"));
            for r in &refs {
                let cr = cs(r);
                (api.js_getregistry)(j, cr.as_ptr());
                v.push(describe(api, j, -1));
                (api.js_pop)(j, 1);
            }
            for r in &refs {
                let cr = cs(r);
                (api.js_unref)(j, cr.as_ptr());
            }
            (api.js_gc)(j, 0);
            // registry by explicit name
            for name in ["k1", "k2", ""] {
                let cn = cs(name);
                (api.js_pushnumber)(j, 42.0);
                (api.js_setregistry)(j, cn.as_ptr());
                (api.js_getregistry)(j, cn.as_ptr());
                v.push(describe(api, j, -1));
                (api.js_pop)(j, 1);
                (api.js_delregistry)(j, cn.as_ptr());
                (api.js_getregistry)(j, cn.as_ptr());
                v.push(describe(api, j, -1));
                (api.js_pop)(j, 1);
            }
            v.push(format!("top={}", (api.js_gettop)(j)));
            (api.js_freestate)(j);
            outs.push(v);
        }
    }
    assert_eq!(outs[0], outs[1], "intern / ref / registry");
}

#[test]
fn extra_concat_equal_compare_instanceof() {
    let p = pair();
    let values = [
        "undefined", "null", "true", "false", "0", "-0", "1", "NaN", "Infinity", "''", "'0'",
        "'1'", "'a'", "[]", "[0]", "[1,2]", "({})", "({valueOf:function(){return 1}})",
        "(function(){})", "/r/", "new Date(0)", "new Number(1)", "new String('1')",
        "new Boolean(false)",
    ];
    for a in values {
        for b in values {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    (api.js_setreport)(j, Some(report_cb));
                    let _ = take_reports();
                    // Build the two operands with a script so every value shape
                    // is available, then drive the low-level comparisons.
                    let fname = cs("[string]");
                    let src = cs(&format!("[{a},{b}]"));
                    let mut ok = true;
                    if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                        ok = false;
                    } else {
                        (api.js_pushundefined)(j);
                        if (api.js_pcall)(j, 0) != 0 {
                            ok = false;
                        }
                    }
                    let mut v = vec![format!("ok={ok}")];
                    if ok {
                        // arr is at -1
                        (api.js_getindex)(j, -1, 0);
                        (api.js_getindex)(j, -2, 1);
                        // stack: arr, a, b
                        (api.js_copy)(j, -2);
                        (api.js_copy)(j, -2);
                        v.push(format!("equal={}", (api.js_equal)(j)));
                        (api.js_copy)(j, -2);
                        (api.js_copy)(j, -2);
                        v.push(format!("strict={}", (api.js_strictequal)(j)));
                        (api.js_copy)(j, -2);
                        (api.js_copy)(j, -2);
                        let mut okay: c_int = -9;
                        let c = (api.js_compare)(j, &mut okay);
                        v.push(format!("compare={c},{okay}"));
                        (api.js_copy)(j, -2);
                        (api.js_copy)(j, -2);
                        (api.js_concat)(j);
                        v.push(describe(api, j, -1));
                        (api.js_pop)(j, 1);
                    }
                    v.push(format!("top={}", (api.js_gettop)(j)));
                    (api.js_freestate)(j);
                    outs.push((v, take_reports()));
                }
            }
            assert_eq!(outs[0], outs[1], "compare ops for ({a}, {b})");
        }
    }
}

#[test]
fn extra_instanceof() {
    // instanceof needs a callable RHS; a non-callable RHS must throw the same
    // TypeError in both.
    let cases = [
        ("[]", "Array"),
        ("[]", "Object"),
        ("({})", "Array"),
        ("(function(){})", "Function"),
        ("new Date(0)", "Date"),
        ("1", "Number"),
        ("new Number(1)", "Number"),
        ("null", "Object"),
        ("({})", "1"),
        ("({})", "({})"),
        ("({})", "undefined"),
    ];
    for (a, b) in cases {
        diff_eval_both_modes(&format!("try {{ ({a}) instanceof ({b}) }} catch(e) {{ 'E:'+e }}"));
    }
}

#[test]
fn extra_toprimitive_hints() {
    let p = pair();
    let setups = [
        "({})",
        "[]",
        "[1]",
        "({valueOf:function(){return 1}})",
        "({toString:function(){return 'x'}})",
        "({valueOf:function(){return 1},toString:function(){return 'x'}})",
        "new Date(0)",
        "new Number(3)",
        "new String('s')",
        "1",
        "'s'",
        "null",
        "undefined",
        "true",
    ];
    // JS_HNONE=0, JS_HNUMBER=1, JS_HSTRING=2 in jsi.h; also probe an
    // out-of-range hint, which C accepts as a plain int.
    for hint in [0 as c_int, 1, 2, 3, -1, 99] {
        for setup in setups {
            let mut outs = Vec::new();
            for api in [&p.c, &p.r] {
                unsafe {
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                    (api.js_setreport)(j, Some(report_cb));
                    let _ = take_reports();
                    let fname = cs("[string]");
                    let src = cs(&format!("({setup})"));
                    let mut ok = true;
                    if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                        ok = false;
                    } else {
                        (api.js_pushundefined)(j);
                        if (api.js_pcall)(j, 0) != 0 {
                            ok = false;
                        }
                    }
                    let mut v = vec![format!("ok={ok}")];
                    if ok {
                        // js_toprimitive can throw; wrap by using js_dostring-free
                        // path is impossible, so only call it on values whose
                        // conversion cannot throw (all setups above are safe).
                        (api.js_toprimitive)(j, -1, hint);
                        v.push(describe(api, j, -1));
                    }
                    v.push(format!("top={}", (api.js_gettop)(j)));
                    (api.js_freestate)(j);
                    outs.push((v, take_reports()));
                }
            }
            assert_eq!(outs[0], outs[1], "toprimitive hint={hint} on {setup}");
        }
    }
}

/* --- host C functions, called back from inside the interpreter --- */

thread_local! {
    static PROBE_LOG: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

unsafe fn probe_body(api: &Api, j: JS) {
    unsafe {
        let errs = cs("<throw>");
        let top = (api.js_gettop)(j);
        let mut line = format!("probe top={top}");
        for i in 0..top.min(6) {
            line.push_str(&format!(
                " [{i}]={:?}",
                rstr((api.js_trystring)(j, i, errs.as_ptr()))
            ));
        }
        (api.js_currentfunction)(j);
        line.push_str(&format!(
            " curfn={:?}",
            rstr((api.js_trystring)(j, -1, errs.as_ptr()))
        ));
        (api.js_pop)(j, 1);
        let data = (api.js_currentfunctiondata)(j);
        line.push_str(&format!(" data_is_marker={}", data as usize == 0xdead_beef));
        PROBE_LOG.with(|l| l.borrow_mut().push(line));
        (api.js_pushnumber)(j, 42.0);
    }
}

unsafe extern "C-unwind" fn probe_c(j: JS) {
    unsafe { probe_body(&pair().c, j) }
}
unsafe extern "C-unwind" fn probe_r(j: JS) {
    unsafe { probe_body(&pair().r, j) }
}

#[test]
fn extra_currentfunction_and_cfunctions() {
    let p = pair();
    let mut outs = Vec::new();
    for (which, api) in [(0usize, &p.c), (1usize, &p.r)] {
        let probe: unsafe extern "C-unwind" fn(JS) = if which == 0 { probe_c } else { probe_r };
        // The C library stores `name` by pointer without copying, so the
        // CStrings must outlive the state.
        let names: Vec<std::ffi::CString> = ["f0", "f1", "f3", "fx", "Ctor", "UD"]
            .iter()
            .map(|s| cs(s))
            .collect();
        PROBE_LOG.with(|l| l.borrow_mut().clear());
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            (api.js_setreport)(j, Some(report_cb));
            let _ = take_reports();
            let mut v = Vec::new();
            for (k, len) in [(0usize, 0), (1, 1), (2, 3)] {
                (api.js_newcfunction)(j, Some(probe), names[k].as_ptr(), len);
                v.push(describe(api, j, -1));
                (api.js_setglobal)(j, names[k].as_ptr());
            }
            (api.js_newcfunctionx)(
                j,
                Some(probe),
                names[3].as_ptr(),
                2,
                0xdead_beefusize as *mut c_void,
                Some(fin_noop),
            );
            v.push(describe(api, j, -1));
            (api.js_setglobal)(j, names[3].as_ptr());

            // js_newcconstructor consumes a prototype object from the stack.
            (api.js_newobject)(j);
            (api.js_newcconstructor)(j, Some(probe), Some(probe), names[4].as_ptr(), 1);
            v.push(describe(api, j, -1));
            (api.js_setglobal)(j, names[4].as_ptr());

            for src in [
                "f0()",
                "f1(1)",
                "f3(1,2,3)",
                "f3(1)",
                "fx()",
                "new Ctor()",
                "Ctor()",
                "f0.length+','+f1.length+','+f3.length+','+fx.length+','+Ctor.length",
                "f0.name+','+Ctor.name",
                "typeof f0",
                "String(f0)",
                "typeof f0.prototype",
                "Ctor.prototype.constructor === Ctor",
                "f0.call(null)",
                "f0.apply(null,[1,2])",
                "f1.bind(null,5)()",
                "new (f1.bind(null,5))()",
                "[3,1,2].sort(f0)",
            ] {
                let fname = cs("[string]");
                let csrc = cs(src);
                let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
                if rc == 0 {
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        rc = 2;
                    }
                } else {
                    rc = 1;
                }
                let errs = cs("<throw>");
                v.push(format!(
                    "{src} -> rc={rc} {}",
                    rstr((api.js_trystring)(j, -1, errs.as_ptr()))
                ));
                (api.js_pop)(j, 1);
            }
            v.push(format!("top={}", (api.js_gettop)(j)));
            (api.js_freestate)(j);
            let log = PROBE_LOG.with(|l| l.borrow().clone());
            outs.push((v, take_reports(), log));
        }
        drop(names);
    }
    assert_eq!(outs[0], outs[1], "C function registration / currentfunction");
}

unsafe extern "C-unwind" fn fin_noop(_j: JS, _p: *mut c_void) {}
