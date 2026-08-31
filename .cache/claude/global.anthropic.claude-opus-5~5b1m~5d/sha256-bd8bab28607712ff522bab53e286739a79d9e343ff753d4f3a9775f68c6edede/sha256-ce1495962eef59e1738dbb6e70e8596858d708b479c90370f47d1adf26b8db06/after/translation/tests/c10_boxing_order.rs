//! Differential tests for the *side effects* of the index-taking public API.
//!
//! `js_toobject` (and therefore every wrapper that calls it) REWRITES the stack
//! slot it is given, boxing a primitive into its wrapper object. That makes the
//! order in which a wrapper evaluates `js_toobject` relative to any other
//! inspection of the same slot observable -- and C leaves argument evaluation
//! order unspecified while Rust fixes it left-to-right. A real bug of exactly
//! this shape was found in `js_setproperty`/`js_setindex` (the strict-mode
//! "cannot create property '%s' on transient object" TypeError was lost).
//!
//! These tests therefore compare, for EVERY index-taking entry point x EVERY
//! primitive value shape:
//!   * whether the call throws, and with which exact message,
//!   * the type of the slot AFTERWARDS (i.e. whether it got boxed), and
//!   * the resulting stack depth.
mod common;
use common::*;
use std::ffi::c_int;

/// The primitive (non-object) shapes, which are the ones that can be boxed.
const PRIMITIVES: &[&str] = &[
    "undefined", "null", "true", "false", "number0", "number42", "numberNaN", "numberInf",
    "stringEmpty", "stringAbc", "string0", "string42", "stringUtf8", "stringLong", "literal",
];

static LIT: &[u8] = b"literal-value\0";

fn push_primitive(imp: &Impl, j: JsState, which: &str) {
    match which {
        "undefined" => imp.pushundefined(j),
        "null" => imp.pushnull(j),
        "true" => imp.pushboolean(j, 1),
        "false" => imp.pushboolean(j, 0),
        "number0" => imp.pushnumber(j, 0.0),
        "number42" => imp.pushnumber(j, 42.0),
        "numberNaN" => imp.pushnumber(j, f64::NAN),
        "numberInf" => imp.pushnumber(j, f64::INFINITY),
        "stringEmpty" => imp.pushstring(j, b""),
        "stringAbc" => imp.pushstring(j, b"abc"),
        "string0" => imp.pushstring(j, b"0"),
        "string42" => imp.pushstring(j, b"42"),
        "stringUtf8" => imp.pushstring(j, "caf\u{e9}\u{4f60}".as_bytes()),
        "stringLong" => imp.pushstring(j, &vec![b'z'; 64]),
        "literal" => unsafe {
            imp.f::<FnVoidStr>("js_pushliteral")(j, LIT.as_ptr() as *const std::ffi::c_char)
        },
        other => panic!("unknown primitive {other}"),
    }
}

/// Snapshot of the slot at `idx` plus the stack depth, using only non-throwing
/// accessors -- this is what reveals an unexpected boxing.
fn slot_snapshot(imp: &Impl, j: JsState, idx: c_int) -> String {
    format!(
        "top={} ty={} tyof={} isobj={} isprim={} isstr={} isnum={} isstrobj={} isnumobj={} v={}",
        imp.gettop(j),
        imp.ty(j, idx),
        show(&imp.typeof_(j, idx)),
        imp.is(j, "js_isobject", idx),
        imp.is(j, "js_isprimitive", idx),
        imp.is(j, "js_isstring", idx),
        imp.is(j, "js_isnumber", idx),
        imp.is(j, "js_isstringobject", idx),
        imp.is(j, "js_isnumberobject", idx),
        show(&imp.trystring(j, idx)),
    )
}

// Which primitive the current probe should push, and which operation to apply.
// Thread-locals let the `fn`-typed probes be parameterised.
thread_local! {
    static WHICH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static OP: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Every index-taking operation that may box its target slot.
const OPS: &[&str] = &[
    "getproperty",
    "setproperty",
    "defproperty",
    "delproperty",
    "hasproperty",
    "getindex",
    "setindex",
    "delindex",
    "hasindex",
    "getlength",
    "setlength",
    "pushiterator_own",
    "pushiterator_all",
    "defaccessor",
    "toobject_direct",
    "toprimitive_direct",
    "tostring",
    "tonumber",
    "repr",
];

fn apply_op(imp: &Impl, j: JsState, op: &str) {
    // The target primitive is at the TOP of the stack when we enter, so any
    // extra pushes shift its index; each arm keeps track explicitly.
    match op {
        "getproperty" => imp.getproperty(j, -1, "foo"),
        "setproperty" => {
            imp.pushnumber(j, 1.0);
            imp.setproperty(j, -2, "foo");
        }
        "defproperty" => {
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "foo", 0);
        }
        "delproperty" => imp.delproperty(j, -1, "foo"),
        "hasproperty" => {
            let n = imp.hasproperty(j, -1, "foo");
            if n != 0 {
                imp.pop(j, 1);
            }
        }
        "getindex" => imp.getindex(j, -1, 0),
        "setindex" => {
            imp.pushnumber(j, 1.0);
            imp.setindex(j, -2, 0);
        }
        "delindex" => imp.delindex(j, -1, 0),
        "hasindex" => {
            let n = imp.hasindex(j, -1, 0);
            if n != 0 {
                imp.pop(j, 1);
            }
        }
        "getlength" => {
            let _ = imp.getlength(j, -1);
        }
        "setlength" => imp.setlength(j, -1, 2),
        "pushiterator_own" => imp.pushiterator(j, -1, 1),
        "pushiterator_all" => imp.pushiterator(j, -1, 0),
        "defaccessor" => {
            imp.pushundefined(j);
            imp.pushnull(j);
            imp.defaccessor(j, -3, "foo", 0);
        }
        "toobject_direct" => {
            let _ = unsafe {
                imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut std::ffi::c_void>(
                    "js_toobject",
                )(j, -1)
            };
        }
        "toprimitive_direct" => {
            let _ = unsafe { imp.f::<FnVoidInt>("js_toprimitive")(j, -1) };
        }
        "tostring" => {
            let _ = imp.tostring(j, -1);
        }
        "tonumber" => {
            let _ = imp.tonumber(j, -1);
        }
        "repr" => imp.repr(j, -1),
        other => panic!("unknown op {other}"),
    }
}

fn probe(imp: &Impl, j: JsState) {
    let which = PRIMITIVES[WHICH.with(|c| c.get())];
    let op = OPS[OP.with(|c| c.get())];
    push_primitive(imp, j, which);
    let before = slot_snapshot(imp, j, -1);
    apply_op(imp, j, op);
    // Find the original slot again: it is at index 0 of this call frame.
    let after = slot_snapshot(imp, j, 0);
    let result = format!("BEFORE[{before}] AFTER[{after}]");
    imp.pushstring(j, result.as_bytes());
}

#[test]
fn boxing_side_effects_match_for_every_op_and_primitive() {
    let mut b = Batch::new();
    for (wi, which) in PRIMITIVES.iter().enumerate() {
        for (oi, op) in OPS.iter().enumerate() {
            WHICH.with(|c| c.set(wi));
            OP.with(|c| c.set(oi));
            for flags in [0 as c_int, JS_STRICT] {
                b.probe(flags, &format!("{op} on {which}"), probe as ProbeFn);
            }
        }
    }
    b.finish("boxing side effects (op x primitive x mode)");
}

/// The specific regression: writing a NEW property to a primitive must raise
/// `TypeError "cannot create property 'x' on transient object"` in strict mode
/// and be a silent no-op otherwise -- for both `js_setproperty` and
/// `js_setindex`, and for every primitive shape.
#[test]
fn transient_property_creation_matches() {
    thread_local! {
        static T_WHICH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static T_KIND: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    fn tprobe(imp: &Impl, j: JsState) {
        let which = PRIMITIVES[T_WHICH.with(|c| c.get())];
        let kind = T_KIND.with(|c| c.get());
        push_primitive(imp, j, which);
        imp.pushnumber(j, 7.0);
        match kind {
            0 => imp.setproperty(j, -2, "brandnew"),
            1 => imp.setindex(j, -2, 0),
            2 => imp.setindex(j, -2, 99),
            // existing built-in properties, which take a different path
            3 => imp.setproperty(j, -2, "length"),
            _ => imp.setproperty(j, -2, "toString"),
        }
        let s = slot_snapshot(imp, j, 0);
        imp.pushstring(j, s.as_bytes());
    }

    let mut b = Batch::new();
    for (wi, which) in PRIMITIVES.iter().enumerate() {
        for kind in 0..5usize {
            T_WHICH.with(|c| c.set(wi));
            T_KIND.with(|c| c.set(kind));
            for flags in [0 as c_int, JS_STRICT] {
                b.probe(flags, &format!("transient set kind={kind} on {which}"), tprobe as ProbeFn);
            }
        }
    }
    b.finish("transient property creation");
}

/// The same semantics reached from JavaScript rather than the C API, so the
/// bytecode paths (`OP_SETPROP`, `OP_SETINDEX`) are covered too.
#[test]
fn transient_property_creation_via_script_matches() {
    let mut b = Batch::new();
    let targets: &[&str] = &[
        "'abc'", "''", "(42)", "(0)", "NaN", "Infinity", "true", "false", "(1.5)",
        "'caf\\u00e9'",
    ];
    for t in targets {
        for expr in [
            format!("(function(){{ var x={t}; x.brandnew = 1; return String(x.brandnew) }})()"),
            format!("(function(){{ var x={t}; x[0] = 1; return String(x[0]) }})()"),
            format!("(function(){{ var x={t}; x[99] = 1; return String(x[99]) }})()"),
            format!("(function(){{ var x={t}; x.length = 9; return String(x.length) }})()"),
            format!("(function(){{ var x={t}; x.toString = 1; return typeof x.toString }})()"),
            format!("(function(){{ {t}.brandnew = 1; return 'done' }})()"),
            format!("(function(){{ {t}[0] = 1; return 'done' }})()"),
            format!("{t}.brandnew = 1"),
            format!("{t}[0] = 1"),
            format!("{t}.length = 9"),
        ] {
            b.script(0, &expr);
            b.script(JS_STRICT, &expr);
        }
    }
    // undefined / null targets throw in BOTH modes (a different message).
    for t in ["undefined", "null", "void 0"] {
        for expr in [
            format!("{t}.brandnew = 1"),
            format!("{t}[0] = 1"),
            format!("{t}.foo"),
            format!("{t}[0]"),
            format!("delete {t}.foo"),
            format!("'foo' in {t}"),
        ] {
            b.script(0, &expr);
            b.script(JS_STRICT, &expr);
        }
    }
    b.finish("transient property creation via script");
}

/// `js_toprimitive` is the inverse side effect: it replaces an object slot with
/// its primitive. Verify both directions and the round trip.
#[test]
fn toprimitive_side_effect_matches() {
    fn p(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        for k in 0..8 {
            match k {
                0 => imp.newobject(j),
                1 => imp.newarray(j),
                2 => imp.newnumber(j, 3.5),
                3 => imp.newstring(j, "wrapped"),
                4 => imp.newboolean(j, 1),
                5 => imp.newregexp(j, "a", 0),
                6 => imp.pushnumber(j, 5.0),
                _ => imp.pushstring(j, b"plain"),
            }
            let before = slot_snapshot(imp, j, -1);
            unsafe { imp.f::<FnVoidInt>("js_toprimitive")(j, -1) };
            let after = slot_snapshot(imp, j, -1);
            // and back again
            let _ = unsafe {
                imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut std::ffi::c_void>(
                    "js_toobject",
                )(j, -1)
            };
            let boxed = slot_snapshot(imp, j, -1);
            acc.push_str(&format!("{k}: {before} => {after} => {boxed}\n"));
            imp.pop(j, 1);
        }
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "js_toprimitive / js_toobject round trip", p);
    }
}

/// `js_concat` coerces BOTH operands, so it exercises two side-effecting
/// conversions in one expression.
#[test]
fn concat_coercion_order_matches() {
    fn p(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        let shapes = 8;
        for a in 0..shapes {
            for b2 in 0..shapes {
                for (i, k) in [a, b2].iter().enumerate() {
                    let _ = i;
                    match k {
                        0 => imp.pushnumber(j, 1.0),
                        1 => imp.pushstring(j, b"s"),
                        2 => imp.newobject(j),
                        3 => imp.newarray(j),
                        4 => imp.pushundefined(j),
                        5 => imp.pushnull(j),
                        6 => imp.pushboolean(j, 1),
                        _ => imp.newnumber(j, 2.5),
                    }
                }
                imp.concat(j);
                acc.push_str(&format!("{a},{b2}={};", show(&imp.trystring(j, -1))));
                imp.pop(j, 1);
            }
        }
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "js_concat coercion", p);
    }
}

/// `js_equal` / `js_strictequal` / `js_compare` / `js_instanceof` all consume two
/// operands and may coerce both. Cross-product every shape pair.
#[test]
fn binary_api_coercion_matches() {
    thread_local! {
        static OPI: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    fn p(imp: &Impl, j: JsState) {
        let which = OPI.with(|c| c.get());
        let mut acc = String::new();
        for a in 0..9 {
            for b2 in 0..9 {
                for k in [a, b2] {
                    match k {
                        0 => imp.pushnumber(j, 1.0),
                        1 => imp.pushnumber(j, f64::NAN),
                        2 => imp.pushstring(j, b"1"),
                        3 => imp.pushstring(j, b"x"),
                        4 => imp.newobject(j),
                        5 => imp.newarray(j),
                        6 => imp.pushundefined(j),
                        7 => imp.pushnull(j),
                        _ => imp.pushboolean(j, 1),
                    }
                }
                let r = match which {
                    0 => format!("{}", imp.equal(j)),
                    1 => format!("{}", imp.strictequal(j)),
                    _ => {
                        let (rc, okay) = imp.compare(j);
                        format!("{rc}/{okay}")
                    }
                };
                acc.push_str(&format!("{a},{b2}={r};"));
            }
        }
        imp.pushstring(j, acc.as_bytes());
    }
    let mut b = Batch::new();
    for which in 0..3usize {
        OPI.with(|c| c.set(which));
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, &format!("binary api {which}"), p as ProbeFn);
        }
    }
    b.finish("js_equal/js_strictequal/js_compare coercion");
}

// ---------------------------------------------------------------------------
// Regressions for the argument-evaluation-order bugs found in the C wrappers.
//
// C leaves the order of evaluation of function arguments UNSPECIFIED; gcc on
// x86-64 evaluates argument lists RIGHT TO LEFT while Rust guarantees LEFT TO
// RIGHT. Wherever two arguments both have side effects (boxing a slot, running
// user JS via toString/valueOf, or throwing), the order is observable.
// ---------------------------------------------------------------------------

/// `Rp_exec` is `js_RegExp_prototype_exec(J, js_toregexp(J,0), js_tostring(J,1))`
/// (jsregexp.c:221). gcc runs `js_tostring(J,1)` FIRST, so the argument's
/// `toString`/`valueOf` side effects happen even when `this` is not a RegExp and
/// `js_toregexp` is about to throw.
#[test]
fn regexp_exec_argument_order_matches() {
    let mut b = Batch::new();
    let receivers: &[&str] = &[
        "/a+/", "/a+/g", "({})", "[]", "(42)", "'str'", "null", "undefined",
        "Object.create(RegExp.prototype)", "RegExp.prototype", "new Date(0)",
        "(function(){})",
    ];
    let args: &[&str] = &[
        "'abc'",
        "({toString:function(){ log.push('toString'); return 'abc' }})",
        "({valueOf:function(){ log.push('valueOf'); return 'abc' }})",
        "({toString:function(){ log.push('t'); return 'a' }, valueOf:function(){ log.push('v'); return 'b' }})",
        "({toString:function(){ log.push('throwing'); throw new Error('from toString') }})",
        "(42)",
        "null",
        "undefined",
        "[1,2]",
    ];
    for recv in receivers {
        for arg in args {
            // `log` records the ORDER of the observable side effects, and is
            // reported even when the call throws.
            let src = format!(
                "var log=[]; var r; try{{ r = String(RegExp.prototype.exec.call({recv}, {arg})) }} \
                 catch(e){{ r = e.name+': '+e.message }} log.join(',')+' => '+r"
            );
            b.script(0, &src);
            b.script(JS_STRICT, &src);
            // Same for `test`, which has the same shape.
            let src2 = format!(
                "var log=[]; var r; try{{ r = String(RegExp.prototype.test.call({recv}, {arg})) }} \
                 catch(e){{ r = e.name+': '+e.message }} log.join(',')+' => '+r"
            );
            b.script(0, &src2);
            b.script(JS_STRICT, &src2);
        }
    }
    b.finish("RegExp.prototype.exec/test argument order");
}

/// `js_defaccessor` is
/// `jsR_defproperty(J, js_toobject(J,idx), name, atts, NULL, jsR_tofunction(J,-2), jsR_tofunction(J,-1), 1)`
/// (jsrun.c:1028). gcc evaluates `jsR_tofunction(J,-1)` first, then
/// `jsR_tofunction(J,-2)`, then `js_toobject(J,idx)`, so a bad SETTER is reported
/// before a bad getter, and both are reported before a bad target object.
#[test]
fn defaccessor_argument_order_matches() {
    thread_local! {
        static TARGET: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static GETTER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static SETTER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// 0 = valid object, 1 = undefined, 2 = null, 3 = number, 4 = string
    fn push_target(imp: &Impl, j: JsState, k: usize) {
        match k {
            0 => imp.newobject(j),
            1 => imp.pushundefined(j),
            2 => imp.pushnull(j),
            3 => imp.pushnumber(j, 42.0),
            _ => imp.pushstring(j, b"str"),
        }
    }
    /// 0 = function, 1 = undefined, 2 = null, 3 = number, 4 = string, 5 = object
    fn push_fn(imp: &Impl, j: JsState, k: usize) {
        match k {
            0 => {
                imp.eval_on(j, b"__acc = function(){return 1}; 1");
                imp.getglobal(j, "__acc");
            }
            1 => imp.pushundefined(j),
            2 => imp.pushnull(j),
            3 => imp.pushnumber(j, 7.0),
            4 => imp.pushstring(j, b"nope"),
            _ => imp.newobject(j),
        }
    }

    fn probe(imp: &Impl, j: JsState) {
        let t = TARGET.with(|c| c.get());
        let g = GETTER.with(|c| c.get());
        let s = SETTER.with(|c| c.get());
        push_target(imp, j, t);
        push_fn(imp, j, g);
        push_fn(imp, j, s);
        imp.defaccessor(j, -3, "p", 0);
        // If it did not throw, report what the target looks like afterwards
        // (whether it got boxed) and what reading the accessor yields.
        let mut acc = format!(
            "ok target_ty={} isobj={};",
            imp.ty(j, 0),
            imp.is(j, "js_isobject", 0)
        );
        imp.getproperty(j, 0, "p");
        acc.push_str(&format!("p={};", show(&imp.trystring(j, -1))));
        imp.pop(j, 1);
        imp.pushstring(j, acc.as_bytes());
    }

    let mut b = Batch::new();
    for t in 0..5usize {
        for g in 0..6usize {
            for s in 0..6usize {
                TARGET.with(|c| c.set(t));
                GETTER.with(|c| c.set(g));
                SETTER.with(|c| c.set(s));
                for flags in [0 as c_int, JS_STRICT] {
                    b.probe(flags, &format!("defaccessor t={t} g={g} s={s}"), probe as ProbeFn);
                }
            }
        }
    }
    b.finish("js_defaccessor argument order");
}

/// `O_defineProperty` is
/// `ToPropertyDescriptor(J, js_toobject(J,1), js_tostring(J,2), js_toobject(J,3))`
/// (jsobject.c:279) -- three side-effecting arguments, evaluated right to left by
/// gcc. Cover the JS-visible surface of that ordering.
#[test]
fn defineproperty_argument_order_matches() {
    let mut b = Batch::new();
    let targets: &[&str] = &["({})", "[]", "(function(){})", "(42)", "'s'", "null", "undefined"];
    let keys: &[&str] = &[
        "'k'",
        "({toString:function(){ log.push('key.toString'); return 'k' }})",
        "({toString:function(){ log.push('key.throws'); throw new Error('key') }})",
        "(0)",
        "null",
        "undefined",
    ];
    let descs: &[&str] = &[
        "({value:1})",
        "({value:1,writable:true,enumerable:true,configurable:true})",
        "({get:function(){return 1}})",
        "({set:function(v){}})",
        "({get:function(){return 1},set:function(v){}})",
        "({value:1,get:function(){return 1}})",
        "(42)",
        "null",
        "undefined",
        "'s'",
    ];
    for t in targets {
        for k in keys {
            for d in descs {
                let src = format!(
                    "var log=[]; var r; try{{ Object.defineProperty({t}, {k}, {d}); r='ok' }} \
                     catch(e){{ r=e.name+': '+e.message }} log.join(',')+' => '+r"
                );
                b.script(0, &src);
                b.script(JS_STRICT, &src);
            }
        }
    }
    b.finish("Object.defineProperty argument order");
}
