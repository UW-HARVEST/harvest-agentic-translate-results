//! Phase C — differential error-path tests for the builtin objects
//! (ERRORS.md rows 339..499: jsobject.c, jsarray.c, jsfunction.c, jsboolean.c,
//! jsnumber.c, jsstring.c, jsdate.c, jsmath.c, json.c, jsbuiltin.c, jsrepr.c).
//!
//! Every case is executed in BOTH shared libraries through the public C entry
//! points only (`js_ploadstring` + `js_pcall`, or a native trampoline that is
//! itself wrapped in `js_pcall`), and the rendered result is compared byte for
//! byte.  Nothing here calls a Rust function of the crate directly.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

const SEED: u64 = 0x5EED_C0FFEE_11u64;

/* Every source is exercised under both state-flag shapes. */
const FLAGS: [c_int; 2] = [0, JS_STRICT];

#[track_caller]
fn ev(label: &str, src: &str) {
    for f in FLAGS {
        diff_eval(label, src, f);
    }
}

#[track_caller]
fn evs(label: &str, srcs: &[&str]) {
    for s in srcs {
        ev(label, s);
    }
}

/* ------------------------------------------------------------------ helpers */

/// Render an f64 as a JS expression that reproduces it exactly.
fn jsnum(x: f64) -> String {
    if x.is_nan() {
        "NaN".to_string()
    } else if x == f64::INFINITY {
        "Infinity".to_string()
    } else if x == f64::NEG_INFINITY {
        "(-Infinity)".to_string()
    } else {
        /* Rust's LowerExp is the shortest round-tripping form. */
        format!("({:e})", x)
    }
}

/// Render a Rust string as a JS double-quoted literal.
fn jsstr(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        let u = c as u32;
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            _ if u < 0x20 || u == 0x7f => o.push_str(&format!("\\u{:04X}", u)),
            _ => o.push(c),
        }
    }
    o.push('"');
    o
}

/// The complete set of primitive expressions used for wrong-`this` probing.
const PRIMS: [&str; 12] = [
    "undefined", "null", "true", "false", "0", "-0", "1.5", "NaN", "Infinity", "''", "'x'",
    "'0'",
];

/* =========================================================== jsobject.c */

/// rows 339, 340, 341, 344, 345, 347, 348, 350, 351, 352, 353, 354, 355, 356
#[test]
fn object_statics_on_non_objects() {
    /* one entry per O_* static that starts with an `js_isobject` guard */
    let unary = [
        "getPrototypeOf",
        "getOwnPropertyNames",
        "keys",
        "preventExtensions",
        "isExtensible",
        "seal",
        "isSealed",
        "freeze",
        "isFrozen",
    ];
    for f in unary {
        for p in PRIMS {
            ev("O_unary_prim", &format!("Object.{}({})", f, p));
        }
        ev("O_unary_none", &format!("Object.{}()", f));
        ev("O_unary_obj", &format!("Object.{}({{a:1}})", f));
        ev("O_unary_arr", &format!("Object.{}([1,2])", f));
        ev("O_unary_fun", &format!("Object.{}(function(){{}})", f));
        ev("O_unary_str_obj", &format!("Object.{}(new String('ab'))", f));
    }

    /* row 340: getOwnPropertyDescriptor */
    for p in PRIMS {
        ev(
            "O_gopd_prim",
            &format!("Object.getOwnPropertyDescriptor({}, 'x')", p),
        );
    }
    evs(
        "O_gopd",
        &[
            "Object.getOwnPropertyDescriptor()",
            "Object.getOwnPropertyDescriptor({})",
            "Object.getOwnPropertyDescriptor({}, 'x')",
            "JSON.stringify(Object.getOwnPropertyDescriptor({a:1}, 'a'))",
            "JSON.stringify(Object.getOwnPropertyDescriptor([1], 'length'))",
            "var o={}; Object.defineProperty(o,'g',{get:function(){return 1}}); \
             var d=Object.getOwnPropertyDescriptor(o,'g'); \
             [typeof d.get, typeof d.set, d.enumerable, d.configurable].join(',')",
        ],
    );

    /* rows 344, 345: defineProperty target / descriptor guards */
    for p in PRIMS {
        ev("O_dp_target", &format!("Object.defineProperty({}, 'x', {{}})", p));
        ev("O_dp_desc", &format!("Object.defineProperty({{}}, 'x', {})", p));
    }
    evs(
        "O_dp_arity",
        &[
            "Object.defineProperty()",
            "Object.defineProperty({})",
            "Object.defineProperty({}, 'x')",
        ],
    );

    /* rows 347, 348: defineProperties guards */
    for p in PRIMS {
        ev("O_dps_target", &format!("Object.defineProperties({}, {{}})", p));
        ev("O_dps_props", &format!("Object.defineProperties({{}}, {})", p));
        ev("O_create_props", &format!("Object.create({{}}, {})", p));
    }
    evs(
        "O_dps",
        &[
            "Object.defineProperties()",
            "Object.defineProperties({})",
            "JSON.stringify(Object.defineProperties({}, {x:{value:1,enumerable:true}}))",
        ],
    );
}

/// rows 342, 343, 346, 349 + invalid accessors + non-configurable redefinition
#[test]
fn object_descriptors_and_accessors() {
    /* rows 342/343: value|writable and get|set are exclusive */
    evs(
        "O_desc_exclusive",
        &[
            "Object.defineProperty({}, 'x', {value:1, get:function(){}})",
            "Object.defineProperty({}, 'x', {value:1, set:function(v){}})",
            "Object.defineProperty({}, 'x', {writable:true, get:function(){}})",
            "Object.defineProperty({}, 'x', {writable:true, set:function(v){}})",
            "Object.defineProperty({}, 'x', {writable:false, get:function(){}})",
            "Object.defineProperty({}, 'x', {writable:false, set:function(v){}})",
            "Object.defineProperty({}, 'x', {value:undefined, get:function(){}})",
            "Object.defineProperty({}, 'x', {value:1, writable:true, get:function(){}, set:function(v){}})",
            /* legal: accessors only */
            "var o={}; Object.defineProperty(o,'x',{get:function(){return 7}}); o.x",
            "var o={}; Object.defineProperty(o,'x',{set:function(v){this.y=v}}); o.x=3; o.y",
            /* legal: data only */
            "var o={}; Object.defineProperty(o,'x',{value:1,writable:true}); o.x=2; o.x",
        ],
    );

    /* accessors that are not callable */
    let bad = ["1", "'x'", "null", "true", "{}", "[]", "NaN"];
    for b in bad {
        ev(
            "O_desc_badget",
            &format!("Object.defineProperty({{}}, 'x', {{get:{}}})", b),
        );
        ev(
            "O_desc_badset",
            &format!("Object.defineProperty({{}}, 'x', {{set:{}}})", b),
        );
        ev(
            "O_desc_badboth",
            &format!(
                "Object.defineProperty({{}}, 'x', {{get:function(){{}}, set:{}}})",
                b
            ),
        );
    }
    /* `undefined` accessors are the "absent" encoding, not an error */
    evs(
        "O_desc_undef_accessor",
        &[
            "Object.defineProperty({}, 'x', {get:undefined})",
            "Object.defineProperty({}, 'x', {set:undefined})",
            "Object.defineProperty({}, 'x', {get:undefined, set:undefined})",
        ],
    );

    /* row 346: defineProperties walk over a non-object descriptor value */
    evs(
        "O_dps_walk",
        &[
            "Object.defineProperties({}, {x:1})",
            "Object.defineProperties({}, {x:null})",
            "Object.defineProperties({}, {x:undefined})",
            "Object.defineProperties({}, {x:'s'})",
            "Object.defineProperties({}, {x:{value:1}, y:2})",
            "Object.defineProperties({}, {x:{value:1, get:function(){}}})",
            /* non-enumerable properties are skipped by the walk */
            "var p={}; Object.defineProperty(p,'x',{value:1}); \
             JSON.stringify(Object.defineProperties({}, p))",
        ],
    );

    /* row 349: Object.create proto must be object or null */
    for p in PRIMS {
        ev("O_create_proto", &format!("Object.create({})", p));
    }
    evs(
        "O_create",
        &[
            "Object.create()",
            "typeof Object.create(null)",
            "Object.getPrototypeOf(Object.create(null))",
            "var o=Object.create({a:5}); o.a",
            "JSON.stringify(Object.create(null, {x:{value:1,enumerable:true}}))",
            "Object.create(null, {x:1})",
        ],
    );

    /* redefining / writing non-configurable & non-writable properties */
    evs(
        "O_nonconfigurable",
        &[
            "var o={}; Object.defineProperty(o,'x',{value:1}); \
             Object.defineProperty(o,'x',{value:2}); o.x",
            "var o={}; Object.defineProperty(o,'x',{value:1,configurable:true}); \
             Object.defineProperty(o,'x',{value:2}); o.x",
            "var o={}; Object.defineProperty(o,'x',{value:1}); delete o.x",
            "var o={}; Object.defineProperty(o,'x',{value:1}); o.x=2; o.x",
            "var o={}; Object.defineProperty(o,'x',{get:function(){return 1}}); o.x=2; o.x",
            "var o={}; Object.defineProperty(o,'x',{value:1}); \
             Object.defineProperty(o,'x',{get:function(){return 2}}); o.x",
            "var o=Object.freeze({a:1}); o.a=2; o.a",
            "var o=Object.freeze({a:1}); delete o.a",
            "var o=Object.freeze({a:1}); o.b=2; o.b",
            "var o=Object.seal({a:1}); o.a=2; o.b=3; [o.a,o.b].join(',')",
            "var o=Object.seal({a:1}); delete o.a",
            "var o=Object.preventExtensions({a:1}); o.b=2; o.b",
            "var o=Object.preventExtensions({}); Object.defineProperty(o,'x',{value:1}); o.x",
            "var a=Object.freeze([1,2]); a[0]=9; a.join(',')",
            "var a=Object.freeze([1,2]); a.push(3)",
            "var a=Object.seal([1,2]); a.length=0; a.length",
            "[Object.isSealed(Object.seal({})), Object.isFrozen(Object.freeze({})), \
              Object.isExtensible(Object.preventExtensions({}))].join(',')",
            "[Object.isSealed({}), Object.isFrozen({}), Object.isExtensible({})].join(',')",
            "[Object.isSealed(Object.freeze({a:1})), Object.isFrozen(Object.seal({a:1}))].join(',')",
        ],
    );
}

/// rows 357, 358, 359 + Object.prototype.toString.call on every primitive
#[test]
fn object_prototype_methods_on_primitives() {
    let subjects = [
        "undefined",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1.5",
        "NaN",
        "Infinity",
        "''",
        "'x'",
        "{}",
        "[]",
        "function(){}",
        "new Date(0)",
        "/a/g",
        "new Error('e')",
        "Math",
        "JSON",
        "new Number(1)",
        "new String('s')",
        "new Boolean(true)",
        "Object.prototype",
        "Object.create(null)",
        "arguments",
    ];
    for s in subjects {
        ev(
            "Op_toString",
            &format!("Object.prototype.toString.call({})", s),
        );
        ev(
            "Op_toLocaleString",
            &format!("Object.prototype.toLocaleString.call({})", s),
        );
        ev("Op_valueOf", &format!(
            "typeof Object.prototype.valueOf.call({})", s));
    }
    /* row 357 */
    for p in PRIMS {
        ev(
            "Op_hasOwnProperty",
            &format!("Object.prototype.hasOwnProperty.call({}, 'x')", p),
        );
        ev(
            "Op_hasOwnProperty0",
            &format!("Object.prototype.hasOwnProperty.call({}, '0')", p),
        );
        /* row 358 */
        ev(
            "Op_isPrototypeOf",
            &format!("Object.prototype.isPrototypeOf.call({}, {{}})", p),
        );
        ev(
            "Op_isPrototypeOf2",
            &format!("Object.prototype.isPrototypeOf.call({{}}, {})", p),
        );
        /* row 359 */
        ev(
            "Op_propertyIsEnumerable",
            &format!("Object.prototype.propertyIsEnumerable.call({}, 'x')", p),
        );
    }
    evs(
        "Op_misc",
        &[
            "Object.prototype.hasOwnProperty.call('abc','0')",
            "Object.prototype.hasOwnProperty.call('abc','length')",
            "Object.prototype.hasOwnProperty.call([1],'0')",
            "Object.prototype.hasOwnProperty.call({a:1},'a')",
            "Object.prototype.hasOwnProperty.call({},'toString')",
            "Object.prototype.hasOwnProperty.call({})",
            "Object.prototype.isPrototypeOf.call(Object.prototype, {})",
            "Object.prototype.isPrototypeOf.call(Object.prototype, 1)",
            "Object.prototype.isPrototypeOf.call({}, {})",
            "Object.prototype.isPrototypeOf.call({})",
            "Object.prototype.propertyIsEnumerable.call({a:1},'a')",
            "Object.prototype.propertyIsEnumerable.call([1],'length')",
            "Object.prototype.propertyIsEnumerable.call('ab','0')",
            "Object.prototype.propertyIsEnumerable.call({})",
            "Object.prototype.toString.call()",
            "Object.getOwnPropertyNames(/a/g).join(',')",
            "Object.getOwnPropertyNames(new String('ab')).join(',')",
            "Object.getOwnPropertyNames([1,2]).join(',')",
            "Object.getOwnPropertyNames(function(a,b){}).join(',')",
            "Object.keys(new String('ab')).join(',')",
            "Object.keys([1,2]).join(',')",
        ],
    );
}

/* ============================================================ jsarray.c */

/// rows 360, 361, 362 + `length` assignment errors
#[test]
fn array_constructor_and_length() {
    let lens = [
        "-1",
        "1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "1e10",
        "4294967296",
        "4294967295",
        "134217728", /* 1<<27 > JS_ARRAYLIMIT */
        "67108864",  /* 1<<26 == JS_ARRAYLIMIT */
        "67108865",
        "100000000",
        "-0",
        "0",
        "1",
        "'3'",
        "'x'",
        "null",
        "undefined",
        "true",
        "{}",
        "[]",
        "[3]",
    ];
    for l in lens {
        ev("new_Array", &format!("var a = new Array({}); a.length", l));
        ev("Array_call", &format!("var a = Array({}); a.length", l));
        ev("Array_len_set", &format!("var a=[1,2,3]; a.length={}; a.length", l));
    }
    evs(
        "Array_ctor_misc",
        &[
            "new Array().length",
            "new Array(1,2,3).join(',')",
            "new Array('a','b').join(',')",
            "Array.isArray(new Array(3))",
            "JSON.stringify(new Array(3))",
            "new Array(3).length",
            "var a=[1,2,3]; a.length=1; a.join(',')",
            "var a=[1,2,3]; a.length=0; a.join(',')",
            "var a=[1,2,3]; a.length=5; a.join(',')",
            "var a=[]; a[4294967294]=1; a.length",
            "var a=[]; a['4294967295']=1; a.length",
        ],
    );
}

/// rows 363, 364 — cyclic join/concat and the JS_STRLIMIT guard in Ap_join
#[test]
fn array_join_concat_cycles_and_string_limit() {
    evs(
        "Ap_join_cycle",
        &[
            "var a=[]; a[0]=a; a.join()",
            "var a=[]; a[0]=a; a.join('-')",
            "var a=[1]; a[1]=a; a.toString()",
            "var a=[1]; a[1]=a; a.join(',')",
            "var a=[], b=[a]; a[0]=b; a.join()",
            "var a=[]; a[0]=a; a[1]=a; a.join('|')",
            "var a=[]; a[0]=a; String(a)",
            "var a=[]; a[0]=a; a.toLocaleString()",
            "var a=[]; a[0]=a; a.concat(a).length",
            "var a=[]; a[0]=a; JSON.stringify(a.concat(1).length)",
            "var a=[1,2]; a.concat(a, a).join(',')",
            "var a=[[1,[2]]]; a.concat([3]).join(',')",
            "var a=[null,undefined,1]; a.join('-')",
            "[].join()",
            "[].join(undefined)",
            "[1,2].join(null)",
            "[1,2].join(undefined)",
            "var a=[]; a.length=3; a.join('-')",
        ],
    );
    /* row 364: n + seplen + rlen > JS_STRLIMIT (1<<28) */
    ev(
        "Ap_join_strlimit",
        "var s='x'; var i; for (i=0;i<27;++i) s=s+s; \
         var a=[s,s,s]; a.join('')",
    );
    ev(
        "Ap_join_strlimit_sep",
        "var s='x'; var i; for (i=0;i<26;++i) s=s+s; \
         var a=[s,s,s,s,s]; a.join('--')",
    );
}

/// rows 365, 366 — sort comparator / length guards
#[test]
fn array_sort_guards() {
    let bad = ["1", "'x'", "null", "true", "{}", "[]", "NaN", "0"];
    for b in bad {
        ev("Ap_sort_badcmp", &format!("[3,1,2].sort({})", b));
        ev("Ap_sort_badcmp1", &format!("[1].sort({})", b));
        ev("Ap_sort_badcmp0", &format!("[].sort({})", b));
    }
    evs(
        "Ap_sort",
        &[
            "[3,1,2].sort().join(',')",
            "[3,1,2].sort(undefined).join(',')",
            "[3,1,2].sort(function(a,b){return a-b}).join(',')",
            "[3,1,2].sort(function(a,b){throw new Error('cmp')})",
            "[3,1,2].sort(function(a,b){return NaN}).join(',')",
            "[3,1,2].sort(function(a,b){return 'x'}).join(',')",
            "var a=[1,,3]; a.sort().join(',')",
            "[undefined,1,null].sort().join(',')",
            "Array.prototype.sort.call({length:2, 0:2, 1:1}, function(a,b){return a-b}).length",
            /* row 366: length clamps at INT_MAX */
            "Array.prototype.sort.call({length:1e300, 0:2, 1:1}, function(a,b){return a-b})",
            "Array.prototype.sort.call({length:1e300, 0:2, 1:1})",
            "Array.prototype.sort.call({length:2147483647, 0:2, 1:1}, function(a,b){return a-b})",
            "Array.prototype.sort.call({length:-1}, function(a,b){return a-b}).length",
            "Array.prototype.sort.call({length:1}, function(a,b){return a-b}).length",
        ],
    );
}

/// rows 367..379 — Array.prototype methods on a bad `this` / bad callback
#[test]
fn array_methods_bad_this_and_callbacks() {
    let methods = [
        "toString",
        "toLocaleString",
        "concat",
        "join",
        "pop",
        "push",
        "reverse",
        "shift",
        "slice",
        "sort",
        "splice",
        "unshift",
        "indexOf",
        "lastIndexOf",
        "every",
        "some",
        "forEach",
        "map",
        "filter",
        "reduce",
        "reduceRight",
    ];
    for m in methods {
        for t in ["null", "undefined"] {
            ev(
                "Ap_bad_this",
                &format!("Array.prototype.{}.call({})", m, t),
            );
            ev(
                "Ap_bad_this_cb",
                &format!(
                    "Array.prototype.{}.call({}, function(){{return 1}})",
                    m, t
                ),
            );
        }
        /* primitives are boxed by js_toobject, so these are not errors */
        for t in ["1", "'ab'", "true"] {
            ev(
                "Ap_prim_this",
                &format!("Array.prototype.{}.call({})", m, t),
            );
        }
        ev(
            "Ap_plain_this",
            &format!("Array.prototype.{}.call({{length:0}})", m),
        );
    }

    /* rows 368..373, 376: callback is not a function */
    let cbmethods = [
        "every",
        "some",
        "forEach",
        "map",
        "filter",
        "reduce",
        "reduceRight",
    ];
    let badcb = ["1", "'x'", "null", "true", "{}", "[]", "NaN", "undefined"];
    for m in cbmethods {
        for b in badcb {
            ev("Ap_badcb", &format!("[1].{}({})", m, b));
            ev("Ap_badcb_empty", &format!("[].{}({})", m, b));
            ev("Ap_badcb_arg", &format!("[1,2].{}({}, 0)", m, b));
        }
        ev("Ap_nocb", &format!("[1].{}()", m));
        ev(
            "Ap_cb_throws",
            &format!("[1,2].{}(function(){{throw new Error('cb')}})", m),
        );
    }

    /* rows 374, 375, 377, 378: reduce/reduceRight without an initial value */
    evs(
        "Ap_reduce",
        &[
            "[].reduce(function(a,b){return a})",
            "[].reduceRight(function(a,b){return a})",
            "new Array(3).reduce(function(a,b){return a})",
            "new Array(3).reduceRight(function(a,b){return a})",
            "[].reduce(function(a,b){return a}, 0)",
            "[].reduceRight(function(a,b){return a}, 0)",
            "new Array(3).reduce(function(a,b){return a}, 7)",
            "[1,2,3].reduce(function(a,b){return a+b})",
            "[1,2,3].reduceRight(function(a,b){return a+b})",
            "[1,2,3].reduce(function(a,b){return a+b}, 10)",
            "var a=[,,3]; a.reduce(function(x,y){return x+y})",
            "var a=[,,3]; a.reduceRight(function(x,y){return x+y})",
            "Array.prototype.reduce.call({length:0}, function(a,b){return a})",
            "Array.prototype.reduce.call({length:3}, function(a,b){return a})",
        ],
    );

    /* row 379: js_getlength -> js_toobject on null/undefined */
    evs(
        "Ap_getlength_this",
        &[
            "Array.prototype.pop.call(null)",
            "Array.prototype.pop.call(undefined)",
            "Array.prototype.push.call(null, 1)",
            "Array.prototype.length",
            "Array.prototype.join.call(null)",
            "Array.prototype.slice.call(null)",
            "Array.prototype.slice.call('abc', 1).join(',')",
            "Array.prototype.slice.call({length:2,0:'a',1:'b'}).join(',')",
        ],
    );

    /* row 367: Ap_toString on a `this` that has no callable join */
    evs(
        "Ap_toString",
        &[
            "Array.prototype.toString.call(null)",
            "Array.prototype.toString.call(undefined)",
            "Array.prototype.toString.call({})",
            "Array.prototype.toString.call({join:1})",
            "Array.prototype.toString.call({join:function(){return 'J'}})",
            "Array.prototype.toString.call(1)",
            "Array.prototype.toString.call('ab')",
        ],
    );
}

/* ========================================================= jsfunction.c */

/// rows 380..388
#[test]
fn function_prototype_and_constructor() {
    /* rows 381, 382: Fp_toString */
    for p in PRIMS {
        ev(
            "Fp_toString_prim",
            &format!("Function.prototype.toString.call({})", p),
        );
    }
    evs(
        "Fp_toString",
        &[
            "Function.prototype.toString.call({})",
            "Function.prototype.toString.call([])",
            "Function.prototype.toString.call(new Date(0))",
            "Function.prototype.toString.call(/a/)",
            "Function.prototype.toString.call()",
            "(function f(a,b){return a}).toString()",
            "(function (){}).toString()",
            "Math.max.toString()",
            "Object.toString()",
            "Function.prototype.toString()",
            "Function.prototype.toString.toString()",
            "(function(a){}).bind(null,1).toString()",
            "(function(a){}).bind(null).name",
            "String(function g(x,y,z){})",
        ],
    );

    /* rows 383, 385, 386: apply / call / bind on non-functions */
    for m in ["apply", "call", "bind"] {
        for p in PRIMS {
            ev(
                "Fp_notfun",
                &format!("Function.prototype.{}.call({})", m, p),
            );
        }
        ev("Fp_notfun_obj", &format!("Function.prototype.{}.call({{}})", m));
        ev("Fp_notfun_arr", &format!("Function.prototype.{}.call([])", m));
        ev(
            "Fp_notfun_args",
            &format!("Function.prototype.{}.call({{}}, null, [1,2])", m),
        );
        ev("Fp_notfun_none", &format!("Function.prototype.{}()", m));
    }

    /* row 384 + apply with non-array-like arguments */
    evs(
        "Fp_apply",
        &[
            "(function(){return arguments.length}).apply(null, {length:-1})",
            "(function(){return arguments.length}).apply(null, {length:0})",
            "(function(){return arguments.length}).apply(null, {length:3})",
            "(function(){return arguments.length}).apply(null, {length:'2'})",
            "(function(){return arguments.length}).apply(null, {length:NaN})",
            "(function(){return arguments.length}).apply(null, {length:1.9})",
            "(function(){return arguments.length}).apply(null, null)",
            "(function(){return arguments.length}).apply(null, undefined)",
            "(function(){return arguments.length}).apply(null)",
            "(function(){return arguments.length}).apply()",
            "(function(){return arguments.length}).apply(null, 1)",
            "(function(){return arguments.length}).apply(null, 'abc')",
            "(function(){return arguments.length}).apply(null, true)",
            "(function(){return arguments.length}).apply(null, [1,2,3])",
            "(function(a,b){return a+b}).apply(null, [1,2])",
            "(function(){return this}).apply(1) === 1",
            "(function(){return typeof this}).apply(null)",
            "(function(){return arguments.length}).apply(null, {length:1e300})",
        ],
    );

    /* row 385 details: Fp_call argument shuffling */
    evs(
        "Fp_call",
        &[
            "(function(){return arguments.length}).call()",
            "(function(){return arguments.length}).call(null)",
            "(function(){return arguments.length}).call(null,1,2)",
            "(function(a){return a}).call(null, 'z')",
            "Function.prototype.call.call(function(){return arguments.length}, null, 1, 2)",
            "Function.prototype.apply.call(function(){return arguments.length}, null, [1,2])",
        ],
    );

    /* rows 386, 387, 388: bind, callbound, constructbound */
    evs(
        "Fp_bind",
        &[
            "Function.prototype.bind.call({})",
            "(function(a,b,c){}).bind(null).length",
            "(function(a,b,c){}).bind(null,1).length",
            "(function(a,b,c){}).bind(null,1,2,3,4).length",
            "(function(){return arguments.length}).bind(null)()",
            "(function(){return arguments.length}).bind(null,1,2)(3)",
            "(function(){return this.v}).bind({v:9})()",
            "var B=(function(a){this.a=a}).bind(null,5); (new B()).a",
            "var B=(function(a,b){this.s=a+b}).bind(null,5); (new B(6)).s",
            "var f=function(){return 1}; var b=f.bind(null); b.__TargetFunction__",
            "var f=function(){return 1}; var b=f.bind(null); \
             Object.getOwnPropertyNames(b).join(',')",
            "var f=function(){}; var b=f.bind(null,1); b.__BoundArguments__ = {length:-1}; \
             typeof b.__BoundArguments__",
            "var f=function(){return arguments.length}; var b=f.bind(null,1); \
             try { Object.defineProperty(b,'__BoundArguments__',{value:{length:-1}}) } \
             catch (e) { } b()",
            "var f=function(){return arguments.length}; var b=f.bind(null,1); b.bind(null,2)()",
            "typeof (function(){}).bind(null).prototype",
            "var f=function(){}; (f.bind(null)) instanceof f",
        ],
    );

    /* row 380: the Function constructor parses its arguments */
    evs(
        "jsB_Function",
        &[
            "Function('@')",
            "new Function('bad syntax')",
            "Function('a b', '')",
            "Function('a', 'b c')",
            "Function('return')",
            "Function(')')",
            "Function('a,', '')",
            "Function('1', '')",
            "Function('a', 'a', 'return a')(1,2)",
            "Function('a', 'return a+1')(2)",
            "Function('')()",
            "Function()()",
            "Function('return 1').length",
            "Function('a','b','').length",
            "new Function('a','return a*2')(21)",
            "Function('var 1')",
            "Function('return }')",
            "Function('/*')",
            "Function(1)()",
            "Function(null)()",
            "Function('a', 'return typeof a')()",
            "Function.length",
            "Function.prototype.length",
            "Function.prototype()",
            "typeof Function.prototype",
        ],
    );
}

/* ========================================================== jsboolean.c */

/// rows 389, 390, 391
#[test]
fn boolean_prototype_wrong_this() {
    for m in ["toString", "valueOf"] {
        for p in PRIMS {
            ev(
                "Bp_wrong_this",
                &format!("Boolean.prototype.{}.call({})", m, p),
            );
        }
        for t in ["{}", "[]", "new Number(1)", "new String('x')", "new Date(0)"] {
            ev(
                "Bp_wrong_obj",
                &format!("Boolean.prototype.{}.call({})", m, t),
            );
        }
        ev("Bp_ok", &format!("Boolean.prototype.{}.call(new Boolean(true))", m));
        ev("Bp_ok2", &format!("Boolean.prototype.{}.call(new Boolean(false))", m));
        ev("Bp_none", &format!("Boolean.prototype.{}()", m));
    }
    evs(
        "Boolean_misc",
        &[
            "new Boolean(1).toString()",
            "new Boolean().valueOf()",
            "Boolean()",
            "Boolean(0)",
            "Boolean('false')",
            "typeof new Boolean(1)",
            "String(new Boolean(true))",
            "Boolean.prototype.toString()",
            "Boolean.prototype.valueOf()",
        ],
    );
}

/* =========================================================== jsnumber.c */

/// rows 392..405
#[test]
fn number_prototype_ranges() {
    /* rows 392, 393, 396, 399, 402, 405: not a number / null this */
    let methods = [
        "valueOf",
        "toString",
        "toFixed",
        "toExponential",
        "toPrecision",
        "toLocaleString",
    ];
    for m in methods {
        for p in PRIMS {
            ev(
                "Np_wrong_this",
                &format!("Number.prototype.{}.call({}, 2)", m, p),
            );
        }
        for t in ["{}", "[]", "new String('1')", "new Boolean(true)"] {
            ev(
                "Np_wrong_obj",
                &format!("Number.prototype.{}.call({}, 2)", m, t),
            );
        }
        ev(
            "Np_ok",
            &format!("Number.prototype.{}.call(new Number(5), 2)", m),
        );
        ev("Np_none", &format!("Number.prototype.{}()", m));
    }

    /* rows 394, 395: radix must be 2..36 */
    let radices = [
        "-1", "0", "1", "2", "8", "10", "16", "35", "36", "37", "100", "1.5", "2.5", "35.9",
        "NaN", "Infinity", "-Infinity", "'16'", "'x'", "null", "undefined", "true", "{}",
        "[]", "[16]",
    ];
    for r in radices {
        ev("Np_toString_radix", &format!("(255).toString({})", r));
        ev("Np_toString_radix_neg", &format!("(-255.5).toString({})", r));
        ev("Np_toString_radix_nan", &format!("(NaN).toString({})", r));
        ev("Np_toString_radix_inf", &format!("(Infinity).toString({})", r));
        ev("Np_toString_radix_zero", &format!("(0).toString({})", r));
        ev("Np_toString_radix_frac", &format!("(0.1).toString({})", r));
    }

    /* rows 397, 398: toFixed digits 0..100 */
    let digits = [
        "-1", "-0.5", "0", "1", "20", "21", "100", "101", "1000", "1.9", "NaN", "Infinity",
        "-Infinity", "'2'", "'x'", "null", "undefined", "true", "{}", "[]",
    ];
    for d in digits {
        ev("Np_toFixed", &format!("(5).toFixed({})", d));
        ev("Np_toFixed_neg", &format!("(-1.25).toFixed({})", d));
        ev("Np_toFixed_big", &format!("(1e21).toFixed({})", d));
        ev("Np_toFixed_nan", &format!("(NaN).toFixed({})", d));
        ev("Np_toFixed_inf", &format!("(Infinity).toFixed({})", d));
        /* rows 400, 401: toExponential digits 0..20 */
        ev("Np_toExponential", &format!("(5).toExponential({})", d));
        ev("Np_toExponential_z", &format!("(0).toExponential({})", d));
        ev("Np_toExponential_n", &format!("(-1234.5).toExponential({})", d));
        /* rows 403, 404: toPrecision width 1..21 */
        ev("Np_toPrecision", &format!("(5).toPrecision({})", d));
        ev("Np_toPrecision_s", &format!("(0.000123).toPrecision({})", d));
        ev("Np_toPrecision_b", &format!("(123456789).toPrecision({})", d));
    }
    evs(
        "Np_edge",
        &[
            "(5).toFixed()",
            "(5).toExponential()",
            "(5).toPrecision()",
            "(5).toPrecision(undefined)",
            "(5).toFixed(undefined)",
            "(5).toExponential(undefined)",
            "(NaN).toPrecision(3)",
            "(Infinity).toPrecision(3)",
            "(-Infinity).toExponential(3)",
            "(0).toFixed(20)",
            "(1e-7).toFixed(20)",
            "(1e-7).toExponential(20)",
            "(1e21).toPrecision(21)",
            "Number.MAX_VALUE.toFixed(0)",
            "Number.MIN_VALUE.toExponential(20)",
            "Number.MAX_VALUE.toString(36)",
            "Number.MIN_VALUE.toString(2).length",
            "(1/3).toString(3)",
            "(-0).toString(2)",
            "(-0).toFixed(2)",
        ],
    );
}

/* =========================================================== jsstring.c */

/// rows 406..410, 413..420
#[test]
fn string_prototype_and_indices() {
    /* row 407: checkstring on null/undefined for every Sp_* */
    let sp = [
        "toString",
        "valueOf",
        "charAt",
        "charCodeAt",
        "concat",
        "indexOf",
        "lastIndexOf",
        "localeCompare",
        "match",
        "replace",
        "search",
        "slice",
        "split",
        "substring",
        "substr",
        "toLowerCase",
        "toUpperCase",
        "toLocaleLowerCase",
        "toLocaleUpperCase",
        "trim",
    ];
    for m in sp {
        for t in ["null", "undefined"] {
            ev("Sp_bad_this", &format!("String.prototype.{}.call({})", m, t));
            ev(
                "Sp_bad_this_arg",
                &format!("String.prototype.{}.call({}, 0)", m, t),
            );
        }
        /* rows 408, 409, 410: toString/valueOf demand a real string object */
        ev("Sp_num_this", &format!("String.prototype.{}.call(1)", m));
        ev("Sp_bool_this", &format!("String.prototype.{}.call(true)", m));
        ev("Sp_obj_this", &format!("String.prototype.{}.call({{}})", m));
        ev("Sp_arr_this", &format!("String.prototype.{}.call([1])", m));
        ev(
            "Sp_strobj_this",
            &format!("String.prototype.{}.call(new String('ab'))", m),
        );
        ev("Sp_none", &format!("String.prototype.{}()", m));
    }

    /* rows 413, 414: charAt / charCodeAt out of range */
    let idx = [
        "-1", "-1.5", "0", "1", "2", "3", "5", "1e10", "2147483648", "4294967296", "1e300",
        "NaN", "Infinity", "-Infinity", "'1'", "'x'", "null", "undefined", "true", "{}", "[]",
    ];
    for i in idx {
        ev("Sp_charAt", &format!("'abc'.charAt({})", i));
        ev("Sp_charCodeAt", &format!("'abc'.charCodeAt({})", i));
        ev("Sp_charAt_u", &format!("'a\\u00e9\\u4e2d'.charAt({})", i));
        ev("Sp_charCodeAt_u", &format!("'a\\u00e9\\u4e2d'.charCodeAt({})", i));
        ev("Sp_indexOf", &format!("'abcabc'.indexOf('b', {})", i));
        ev("Sp_lastIndexOf", &format!("'abcabc'.lastIndexOf('b', {})", i));
        ev("Sp_slice1", &format!("'abcdef'.slice({})", i));
        ev("Sp_slice2", &format!("'abcdef'.slice(1, {})", i));
        ev("Sp_substring1", &format!("'abcdef'.substring({})", i));
        ev("Sp_substring2", &format!("'abcdef'.substring(1, {})", i));
        ev("Sp_substr1", &format!("'abcdef'.substr({})", i));
        ev("Sp_substr2", &format!("'abcdef'.substr(1, {})", i));
        /* rows 416, 417: split limits */
        ev("Sp_split_str", &format!("JSON.stringify('abc'.split('b', {}))", i));
        ev("Sp_split_re", &format!("JSON.stringify('abc'.split(/b/, {}))", i));
        ev("Sp_split_empty", &format!("JSON.stringify('abc'.split('', {}))", i));
    }

    /* row 415: fromCharCode out of range */
    let codes = [
        "0", "65", "0x7f", "0x80", "0x7ff", "0x800", "0xffff", "0x10000", "0x10ffff",
        "0x110000", "0x7fffffff", "0x80000000", "4294967296", "-1", "-65", "1.5", "65.9",
        "NaN", "Infinity", "-Infinity", "'65'", "'x'", "null", "undefined", "true", "{}",
        "[]", "0xd800", "0xdfff",
    ];
    for c in codes {
        ev(
            "S_fromCharCode",
            &format!("var s = String.fromCharCode({}); [s.length, s.charCodeAt(0)].join(',')", c),
        );
    }
    evs(
        "S_fromCharCode_misc",
        &[
            "String.fromCharCode()",
            "String.fromCharCode(72,73).length",
            "String.fromCharCode(0xd800,0xdc00).length",
            "escape(String.fromCharCode(0x110000))",
            "String.fromCharCode.length",
        ],
    );

    /* rows 418, 419, 420 + replace with a non-callable second argument */
    evs(
        "Sp_replace_match_search",
        &[
            "'abc'.replace(/(a)/, '$9')",
            "'abc'.replace(/(a)/, '$0')",
            "'abc'.replace(/(a)/, '$1')",
            "'abc'.replace(/(a)/, '$2')",
            "'abc'.replace(/(a)(b)/, '$1$2$3')",
            "'abc'.replace(/a/, '$$')",
            "'abc'.replace(/a/, '$&')",
            "'abc'.replace(/b/, \"$`\")",
            "'abc'.replace(/b/, \"$'\")",
            "'abc'.replace(/a/, '$')",
            "'abc'.replace(/a/, '$x')",
            "'abc'.replace('a', 1)",
            "'abc'.replace('a', null)",
            "'abc'.replace('a', undefined)",
            "'abc'.replace('a', {})",
            "'abc'.replace('a', [1])",
            "'abc'.replace(/a/, 1)",
            "'abc'.replace(/a/, {})",
            "'abc'.replace(/a/g, function(m){return m.toUpperCase()})",
            "'abc'.replace(/(a)/, function(m,p1,off,s){return [m,p1,off,s].join(':')})",
            "'abc'.replace(/a/, function(){throw new Error('rep')})",
            "'abc'.replace()",
            "'abc'.replace('a')",
            "'a'.match('(')",
            "'a'.search('(')",
            "'a'.split('(')",
            "'a'.replace('(', 'x')",
            "'a'.match('[')",
            "'a'.search(')')",
            "'a'.match(/a/g).join(',')",
            "'a'.match(/b/)",
            "'a'.match()",
            "'a'.search()",
            "'a'.match(undefined) === null ? 'null' : JSON.stringify('a'.match(undefined))",
            "'abc'.search(/b/)",
            "'abc'.search(/z/)",
            "JSON.stringify('a,b,,c'.split(','))",
            "JSON.stringify('abc'.split(undefined))",
            "JSON.stringify('abc'.split())",
            "JSON.stringify('abc'.split(/(b)/))",
            "JSON.stringify(''.split(''))",
            "JSON.stringify(''.split('x'))",
        ],
    );

    /* malformed / astral surrogate input */
    evs(
        "Sp_surrogates",
        &[
            "'\\ud800'.length",
            "'\\ud800'.charCodeAt(0)",
            "'\\udfff\\ud800'.length",
            "'\\ud83d\\ude00'.length",
            "'\\ud83d\\ude00'.charCodeAt(0)",
            "'\\ud83d\\ude00'.charCodeAt(1)",
            "'\\ud800'.toUpperCase().length",
            "'\\ud800'.split('').length",
            "JSON.stringify('\\ud800')",
            "encodeURIComponent('\\ud83d\\ude00')",
            "'\\u0000'.length",
            "'a\\u0000b'.length",
            "'a\\u0000b'.indexOf('b')",
            "'\\uffff'.charCodeAt(0)",
            "'\\ufffd'.charCodeAt(0)",
        ],
    );

    /* row 406: regexp recursion depth limit */
    evs(
        "js_doregexec",
        &[
            "new Array(6000).join('a').search(/a*b/)",
            "new Array(6000).join('a').match(/a*b/)",
            "/a*b/.test(new Array(6000).join('a'))",
            "new Array(6000).join('a').replace(/a*b/, 'x').length",
            "new Array(100).join('a').search(/a*b/)",
        ],
    );
}

/// rows 411, 412 — Sp_concat and the JS_STRLIMIT guards
#[test]
fn string_concat_limits() {
    /* row 412: accumulated length crosses 1<<28 */
    ev(
        "Sp_concat_limit_accum",
        "var s='x'; var i; for (i=0;i<27;++i) s=s+s; s.concat(s).length",
    );
    ev(
        "Sp_concat_limit_many",
        "var s='x'; var i; for (i=0;i<26;++i) s=s+s; ''.concat(s,s,s,s,s).length",
    );
    /* row 411: this string is already at the limit, so 1+n exceeds it */
    ev(
        "Sp_concat_limit_self",
        "var s='x'; var i; for (i=0;i<28;++i) s=s+s; s.concat('x').length",
    );
    evs(
        "Sp_concat_ok",
        &[
            "''.concat()",
            "'a'.concat()",
            "'a'.concat('b','c')",
            "'a'.concat(1,null,undefined,true,{})",
            "'a'.concat([1,2])",
            "String.prototype.concat.call(1, 2)",
        ],
    );
}

/* ============================================================= jsdate.c */

/// rows 421..423, 425, 427..449 — constructors, parse, setters, wrong `this`
#[test]
fn date_constructors_parse_and_this() {
    /* 0 argument: the value itself is the wall clock, so only shape is stable */
    evs(
        "Date_0args",
        &[
            "typeof new Date()",
            "new Date() instanceof Date",
            "isFinite(new Date().getTime())",
            "new Date().getTime() > 1000000000000",
            "typeof Date()",
            "typeof Date.now()",
            "isFinite(Date.now())",
        ],
    );
    /* 1..7 arguments, valid and invalid */
    let args1 = [
        "0",
        "-0",
        "1",
        "1.5",
        "-1",
        "NaN",
        "Infinity",
        "-Infinity",
        "8.64e15",
        "8640000000000001",
        "-8.64e15",
        "-8640000000000001",
        "1e300",
        "'2000-01-01'",
        "'2000-01-01T00:00:00Z'",
        "'garbage'",
        "''",
        "null",
        "undefined",
        "true",
        "{}",
        "[]",
        "[0]",
        "new Date(0)",
    ];
    for a in args1 {
        ev("Date_1arg", &format!("new Date({}).getTime()", a));
        ev("Date_1arg_iso", &format!("try {{ new Date({}).toISOString() }} catch (e) {{ String(e) }}", a));
        ev("Date_1arg_str", &format!("String(new Date({}))", a));
        ev("Date_1arg_json", &format!("JSON.stringify(new Date({}))", a));
    }
    let multi = [
        "2000, 0",
        "2000, NaN",
        "2000, 12",
        "2000, -1",
        "2000, 0, 1",
        "2000, 0, NaN",
        "2000, 0, 32",
        "2000, 0, 1, 0",
        "2000, 0, 1, NaN",
        "2000, 0, 1, 25",
        "2000, 0, 1, 0, 0",
        "2000, 0, 1, 0, NaN",
        "2000, 0, 1, 0, 0, 0",
        "2000, 0, 1, 0, 0, NaN",
        "2000, 0, 1, 0, 0, 0, 0",
        "2000, 0, 1, 0, 0, 0, NaN",
        "2000, 0, 1, 0, 0, 0, 0, 99",
        "NaN, 0",
        "Infinity, 0",
        "1e300, 0",
        "0, 0",
        "99, 0",
        "'2000', '0'",
        "{}, {}",
        "null, null",
        "undefined, undefined",
    ];
    for a in multi {
        ev("Date_multi", &format!("new Date({}).getTime()", a));
        ev("Date_multi_utc", &format!("Date.UTC({})", a));
    }

    /* rows 430..449: Date.parse of malformed input */
    let parses = [
        "", " ", "20x0", "abcd", "2000", "2000-", "2000-x", "2000-0", "2000-01",
        "2000-01-", "2000-01-x", "2000-01-0", "2000-01-01", "2000-01-01T",
        "2000-01-01Tx", "2000-01-01T12", "2000-01-01T12x", "2000-01-01T12:",
        "2000-01-01T12:x", "2000-01-01T12:00", "2000-01-01T12:00:",
        "2000-01-01T12:00:x", "2000-01-01T12:00:00", "2000-01-01T12:00:00.",
        "2000-01-01T12:00:00.x", "2000-01-01T12:00:00.1", "2000-01-01T12:00:00.123",
        "2000-01-01T12:00:00.1234", "2000-01-01T12:00+", "2000-01-01T12:00+x",
        "2000-01-01T12:00+01", "2000-01-01T12:00+01:", "2000-01-01T12:00+01:x",
        "2000-01-01T12:00+01:00", "2000-01-01T12:00+24:00", "2000-01-01T12:00+23:60",
        "2000-01-01T12:00-01:00", "2000-01-01T12:00Z", "2000-01-01T12:00Zx",
        "2000-01-01x", "2000-13-01", "2000-00-01", "2000-12-01", "2000-01-32",
        "2000-01-00", "2000-01-31", "2000-01-01T25:00", "2000-01-01T24:00",
        "2000-01-01T24:01", "2000-01-01T24:00:01", "2000-01-01T24:00:00.001",
        "2000-01-01T12:60", "2000-01-01T12:59", "2000-01-01T12:00:60",
        "2000-01-01T12:00:59", "+2000-01-01", "-2000-01-01", "0000-01-01",
        "9999-12-31T23:59:59.999Z", "275760-09-13", "1970-01-01T00:00:00.000Z",
        "Thu Jan 01 1970", "1/1/1970", "T12:00", "Z",
    ];
    for p in parses {
        ev("Date_parse", &format!("Date.parse({})", jsstr(p)));
    }
    evs(
        "Date_parse_misc",
        &[
            "Date.parse()",
            "Date.parse(0)",
            "Date.parse(null)",
            "Date.parse(undefined)",
            "Date.parse({})",
            "Date.parse(new Date(0))",
            "Date.parse(new Date(0).toISOString())",
        ],
    );

    /* rows 421, 422, 423, 425: wrong `this` */
    let dp = [
        "getTime",
        "valueOf",
        "getFullYear",
        "getMonth",
        "getDate",
        "getDay",
        "getHours",
        "getMinutes",
        "getSeconds",
        "getMilliseconds",
        "getUTCFullYear",
        "getUTCMonth",
        "getUTCDate",
        "getUTCDay",
        "getUTCHours",
        "getUTCMinutes",
        "getUTCSeconds",
        "getUTCMilliseconds",
        "getTimezoneOffset",
        "setTime",
        "setFullYear",
        "setMonth",
        "setDate",
        "setHours",
        "setMinutes",
        "setSeconds",
        "setMilliseconds",
        "setUTCFullYear",
        "setUTCMonth",
        "setUTCDate",
        "setUTCHours",
        "setUTCMinutes",
        "setUTCSeconds",
        "setUTCMilliseconds",
        "toISOString",
        "toJSON",
        "toString",
        "toDateString",
        "toTimeString",
        "toUTCString",
        "toLocaleString",
        "toLocaleDateString",
        "toLocaleTimeString",
    ];
    for m in dp {
        for t in ["null", "undefined", "{}", "1", "'x'", "true", "[]", "new Number(0)"] {
            ev(
                "Dp_wrong_this",
                &format!("Date.prototype.{}.call({}, 0)", m, t),
            );
        }
        ev("Dp_none", &format!("Date.prototype.{}()", m));
    }
    /* row 425: toJSON goes through this.toISOString */
    evs(
        "Dp_toJSON",
        &[
            "Date.prototype.toJSON.call({toISOString:1})",
            "Date.prototype.toJSON.call({toISOString:null})",
            "Date.prototype.toJSON.call({})",
            "Date.prototype.toJSON.call({valueOf:function(){return 0}, toISOString:function(){return 'X'}})",
            "Date.prototype.toJSON.call({valueOf:function(){return NaN}, toISOString:function(){return 'X'}})",
            "Date.prototype.toJSON.call(1)",
            "new Date(NaN).toJSON()",
            "new Date(0).toJSON()",
            "new Date(Infinity).toJSON()",
            "JSON.stringify({d:new Date(NaN)})",
            "JSON.stringify({d:new Date(0)})",
        ],
    );

    /* setters with NaN and other odd values */
    let setters = [
        "setTime",
        "setMilliseconds",
        "setSeconds",
        "setMinutes",
        "setHours",
        "setDate",
        "setMonth",
        "setFullYear",
        "setUTCMilliseconds",
        "setUTCSeconds",
        "setUTCMinutes",
        "setUTCHours",
        "setUTCDate",
        "setUTCMonth",
        "setUTCFullYear",
    ];
    let vals = [
        "NaN", "Infinity", "-Infinity", "0", "-1", "1.5", "1e300", "'x'", "'5'", "null",
        "undefined", "true", "{}", "[]",
    ];
    for s in setters {
        for v in vals {
            ev(
                "Dp_setter",
                &format!("var d=new Date(0); d.{}({}); d.getTime()", s, v),
            );
        }
        ev("Dp_setter_noarg", &format!("var d=new Date(0); d.{}(); d.getTime()", s));
        ev(
            "Dp_setter_on_invalid",
            &format!("var d=new Date(NaN); d.{}(1); d.getTime()", s),
        );
        ev(
            "Dp_setter_multi",
            &format!("var d=new Date(0); d.{}(1,2,3,4); d.getTime()", s),
        );
    }
}

/// rows 424, 426, 450..469 — invalid-date getters and formatting
#[test]
fn date_invalid_getters_and_formatting() {
    let getters = [
        "getFullYear",
        "getMonth",
        "getDate",
        "getDay",
        "getHours",
        "getMinutes",
        "getSeconds",
        "getMilliseconds",
        "getUTCFullYear",
        "getUTCMonth",
        "getUTCDate",
        "getUTCDay",
        "getUTCHours",
        "getUTCMinutes",
        "getUTCSeconds",
        "getUTCMilliseconds",
        "getTimezoneOffset",
        "getTime",
        "valueOf",
    ];
    for g in getters {
        for d in [
            "NaN",
            "Infinity",
            "-Infinity",
            "8640000000000001",
            "-8640000000000001",
            "8.64e15",
            "0",
            "'garbage'",
        ] {
            ev("Dp_getter", &format!("new Date({}).{}()", d, g));
        }
    }
    /* rows 450, 451, 452: fmtdate / fmttime / fmtdatetime of an invalid date */
    let fmt = [
        "toString",
        "toDateString",
        "toTimeString",
        "toUTCString",
        "toLocaleString",
        "toLocaleDateString",
        "toLocaleTimeString",
    ];
    for f in fmt {
        for d in ["NaN", "Infinity", "-Infinity", "8640000000000001", "'garbage'", "0"] {
            ev("Dp_fmt", &format!("new Date({}).{}()", d, f));
        }
    }
    /* row 424: toISOString on an invalid date is a RangeError */
    for d in [
        "NaN",
        "Infinity",
        "-Infinity",
        "8640000000000001",
        "-8640000000000001",
        "8.64e15",
        "-8.64e15",
        "0",
        "'garbage'",
        "-62167219200000",
        "253402300799999",
    ] {
        ev("Dp_toISOString", &format!("new Date({}).toISOString()", d));
    }
    evs(
        "Date_misc",
        &[
            "String(new Date(NaN))",
            "new Date(NaN) + ''",
            "+new Date(NaN)",
            "new Date(NaN).getTime() === new Date(NaN).getTime()",
            "isNaN(new Date(NaN))",
            "new Date(0).toISOString()",
            "new Date(-1).toISOString()",
            "new Date(1).toISOString()",
            "new Date(8.64e15).toISOString()",
            "new Date(-8.64e15).toISOString()",
            "Date.length",
            "Date.prototype.constructor === Date",
            "typeof Date.prototype.toISOString",
            "new Date(2000, 0, 1).getFullYear()",
            "new Date(Date.UTC(2000,0,1)).toISOString()",
        ],
    );
}

/* ============================================================= jsmath.c */

/// rows 470..476
#[test]
fn math_edge_values() {
    let unary = [
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "round", "sin",
        "sqrt", "tan",
    ];
    let args = [
        "", "NaN", "Infinity", "-Infinity", "0", "-0", "1", "-1", "0.5", "-0.5", "1.5",
        "-1.5", "2.5", "-2.5", "1e300", "-1e300", "1e-300", "'2'", "'x'", "''", "null",
        "undefined", "true", "false", "{}", "[]", "[2]", "new Number(3)",
    ];
    for f in unary {
        for a in args {
            ev("Math_unary", &format!("Math.{}({})", f, a));
        }
        ev("Math_unary_2args", &format!("Math.{}(1, 2)", f));
        ev("Math_len", &format!("Math.{}.length", f));
    }
    /* atan2 / pow: two arguments (row 472) */
    let pairs = [
        ("", ""),
        ("1", ""),
        ("NaN", "1"),
        ("1", "NaN"),
        ("-1", "Infinity"),
        ("-1", "-Infinity"),
        ("1", "Infinity"),
        ("1", "-Infinity"),
        ("Infinity", "Infinity"),
        ("-Infinity", "0.5"),
        ("0", "0"),
        ("-0", "-1"),
        ("0", "-1"),
        ("2", "10"),
        ("-8", "0.3333333333333333"),
        ("-8", "3"),
        ("'2'", "'3'"),
        ("null", "null"),
        ("undefined", "undefined"),
        ("{}", "{}"),
        ("1e300", "2"),
        ("0.5", "1e300"),
    ];
    for (x, y) in pairs {
        let sep = if y.is_empty() { "" } else { ", " };
        ev("Math_pow", &format!("Math.pow({}{}{})", x, sep, y));
        ev("Math_atan2", &format!("Math.atan2({}{}{})", x, sep, y));
    }
    /* rows 473..476 */
    evs(
        "Math_maxmin",
        &[
            "Math.max()",
            "Math.min()",
            "Math.max(1)",
            "Math.min(1)",
            "Math.max(1, NaN, 3)",
            "Math.min(1, NaN, 3)",
            "Math.max(NaN)",
            "Math.min(NaN)",
            "Math.max(NaN, 1)",
            "Math.min(NaN, 1)",
            "Math.max(1, 2, 3)",
            "Math.min(1, 2, 3)",
            "Math.max(-0, 0)",
            "Math.min(-0, 0)",
            "1/Math.max(-0, 0)",
            "1/Math.min(-0, 0)",
            "Math.max(Infinity, -Infinity)",
            "Math.min(Infinity, -Infinity)",
            "Math.max('2', '10')",
            "Math.min('2', '10')",
            "Math.max(null, undefined)",
            "Math.min(null, undefined)",
            "Math.max({}, 1)",
            "Math.min([], 1)",
            "Math.max.length",
            "Math.min.length",
        ],
    );
    /* rows 470, 471: jsM_round */
    evs(
        "Math_round",
        &[
            "Math.round(NaN)",
            "Math.round(Infinity)",
            "Math.round(-Infinity)",
            "Math.round(0.5)",
            "Math.round(-0.5)",
            "1/Math.round(-0.5)",
            "Math.round(-0.6)",
            "Math.round(-1.5)",
            "Math.round(2.5)",
            "Math.round(1e300)",
            "Math.round(-0)",
            "1/Math.round(-0)",
            "Math.round(4503599627370496.5)",
            "Math.round(0.49999999999999994)",
        ],
    );
    /* Math.random is not deterministic: check only its contract */
    evs(
        "Math_random",
        &[
            "typeof Math.random()",
            "var r=Math.random(); r >= 0 && r < 1",
            "Math.random.length",
            "Math.random(1,2) >= 0",
        ],
    );
    evs(
        "Math_constants",
        &[
            "[Math.E, Math.LN10, Math.LN2, Math.LOG2E, Math.LOG10E, Math.PI, \
              Math.SQRT1_2, Math.SQRT2].join(',')",
            "Object.prototype.toString.call(Math)",
            "typeof Math",
            "Math()",
            "new Math()",
        ],
    );
}

/* =============================================================== json.c */

/// rows 477..482
#[test]
fn json_parse_errors() {
    let bad = [
        "",
        " ",
        "\t\n",
        "{",
        "}",
        "[",
        "]",
        "[1",
        "[1,",
        "[1,]",
        "[,1]",
        "[1 2]",
        "{\"a\"",
        "{\"a\":",
        "{\"a\":1",
        "{\"a\":1,}",
        "{\"a\" 1}",
        "{\"a\":1 \"b\":2}",
        "{1:2}",
        "{true:2}",
        "{null:1}",
        "{'a':1}",
        "'a'",
        "\"a",
        "\"a\\",
        "\"\\x41\"",
        "\"\\q\"",
        "\"\\u12\"",
        "\"\\uZZZZ\"",
        "\"\\u{41}\"",
        "\"a\nb\"",
        "01",
        "1.",
        ".5",
        "+1",
        "-",
        "-.5",
        "1e",
        "1e+",
        "0x10",
        "Infinity",
        "NaN",
        "tru",
        "nul",
        "TRUE",
        "undefined",
        "1 2",
        "[1][2]",
        "{}{}",
        "[[1],",
        "{\"a\":{\"b\":}}",
        "//c\n1",
        "/*c*/1",
        "\u{feff}1",
        "\u{a0}1",
    ];
    for b in bad {
        ev("JSON_parse_bad", &format!("JSON.parse({})", jsstr(b)));
    }
    let good = [
        "0",
        "-0",
        "1",
        "-1.5",
        "1e3",
        "1E-3",
        "\"\"",
        "\"a\\u0041\\n\\t\\\"\\\\\\/\\b\\f\\r\"",
        "true",
        "false",
        "null",
        "[]",
        "{}",
        "[1,2,3]",
        "{\"a\":1,\"b\":[1,{\"c\":null}]}",
        " [ 1 , 2 ] ",
        "[[[[[[1]]]]]]",
    ];
    for g in good {
        ev(
            "JSON_parse_good",
            &format!("JSON.stringify(JSON.parse({}))", jsstr(g)),
        );
    }
    /* row 482: deep (but legal, non-crashing) nesting */
    for depth in [1usize, 2, 8, 64, 200] {
        let s = format!(
            "var n={}; var s=new Array(n+1).join('[')+new Array(n+1).join(']'); \
             var v=JSON.parse(s); var d=0; while (v instanceof Array && v.length===0) \
             {{ d=d+1; break }} JSON.stringify(v).length",
            depth
        );
        ev("JSON_parse_deep", &s);
        let s2 = format!(
            "var n={}; var s=new Array(n+1).join('{{\"a\":')+'1'+new Array(n+1).join('}}'); \
             JSON.stringify(JSON.parse(s)).length",
            depth
        );
        ev("JSON_parse_deep_obj", &s2);
    }
    /* non-string arguments and the reviver */
    evs(
        "JSON_parse_args",
        &[
            "JSON.parse()",
            "JSON.parse(undefined)",
            "JSON.parse(null)",
            "JSON.parse(1)",
            "JSON.parse(true)",
            "JSON.parse({})",
            "JSON.parse([1])",
            "JSON.parse([])",
            "JSON.parse('1', 1)",
            "JSON.parse('1', function(k,v){return v})",
            "JSON.parse('{\"a\":1}', function(k,v){return v})",
            "JSON.parse('{\"a\":1}', function(k,v){throw new Error('rev')})",
            "JSON.parse('[1,2]', function(k,v){return undefined})",
            "JSON.parse.length",
        ],
    );
}

/// rows 483..490
#[test]
fn json_stringify_errors_and_shapes() {
    /* rows 483, 484: cyclic values */
    evs(
        "JSON_cyclic",
        &[
            "var a={}; a.a=a; JSON.stringify(a)",
            "var a=[]; a[0]=a; JSON.stringify(a)",
            "var a={}, b={a:a}; a.b=b; JSON.stringify(a)",
            "var a=[], b=[a]; a[0]=b; JSON.stringify(a)",
            "var a={}; a.x={y:a}; JSON.stringify(a)",
            "var a={}; JSON.stringify([a,a])",
            "var a={}; JSON.stringify({p:a,q:a})",
            "var a={}; a.a=a; JSON.stringify({wrap:a})",
        ],
    );
    /* rows 485, 486: undefined / callable values */
    evs(
        "JSON_undefined_callable",
        &[
            "JSON.stringify(undefined)",
            "JSON.stringify()",
            "JSON.stringify(function(){})",
            "JSON.stringify(Math.max)",
            "JSON.stringify({a:undefined, b:function(){}})",
            "JSON.stringify({a:undefined, b:function(){}, c:1})",
            "JSON.stringify([undefined, function(){}, 1])",
            "JSON.stringify(null)",
            "JSON.stringify(NaN)",
            "JSON.stringify(Infinity)",
            "JSON.stringify(-Infinity)",
            "JSON.stringify(-0)",
            "JSON.stringify(1e300)",
            "JSON.stringify('a\\u0000b')",
            "JSON.stringify('\\u001f\\u007f')",
            "JSON.stringify(new Number(1))",
            "JSON.stringify(new String('s'))",
            "JSON.stringify(new Boolean(true))",
            "JSON.stringify(/re/)",
            "JSON.stringify({toJSON:function(){return 1}})",
            "JSON.stringify({toJSON:1})",
            "JSON.stringify({toJSON:function(){throw new Error('tj')}})",
            "JSON.stringify({a:{toJSON:function(){return undefined}}})",
        ],
    );
    /* row 490 + replacer function / array */
    evs(
        "JSON_replacer",
        &[
            "JSON.stringify({a:1,b:2}, ['a'])",
            "JSON.stringify({a:1,b:2}, ['b','a'])",
            "JSON.stringify({a:1,b:2}, [])",
            "JSON.stringify({a:1,b:2}, ['c'])",
            "JSON.stringify({a:1,b:2}, [1])",
            "JSON.stringify({a:1,b:2}, [null])",
            "JSON.stringify({1:1,b:2}, [1])",
            "JSON.stringify([1,2], ['0'])",
            "JSON.stringify({a:{b:1,c:2}}, ['a','b'])",
            "JSON.stringify({a:1,b:2}, function(k,v){return v})",
            "JSON.stringify({a:1,b:2}, function(k,v){return k==='a'?undefined:v})",
            "JSON.stringify({a:1}, function(k,v){throw new Error('rp')})",
            "JSON.stringify({a:1}, 1)",
            "JSON.stringify({a:1}, 'x')",
            "JSON.stringify({a:1}, null)",
            "JSON.stringify({a:1}, undefined)",
            "JSON.stringify({a:1}, {})",
            "JSON.stringify({a:1}, true)",
        ],
    );
    /* rows 487, 488, 489: the `space` argument */
    let spaces = [
        "-5", "-1", "0", "1", "2", "10", "11", "100", "1e300", "1.9", "NaN", "Infinity",
        "-Infinity", "''", "'  '", "'aaaaaaaaaaaaaaaaaaaa'", "'\\t'", "'\\n'", "null",
        "undefined", "true", "{}", "[]", "'1'", "new Number(4)", "new String('..')",
    ];
    for s in spaces {
        ev(
            "JSON_space",
            &format!("JSON.stringify({{a:1,b:[1,2]}}, null, {})", s),
        );
        ev(
            "JSON_space_arr",
            &format!("JSON.stringify([1,[2,{{c:3}}]], null, {})", s),
        );
        ev("JSON_space_empty", &format!("JSON.stringify({{}}, null, {})", s));
        ev("JSON_space_scalar", &format!("JSON.stringify(1, null, {})", s));
    }
    evs(
        "JSON_misc",
        &[
            "JSON.stringify.length",
            "typeof JSON",
            "Object.prototype.toString.call(JSON)",
            "JSON()",
            "new JSON()",
            "JSON.stringify(JSON)",
            "JSON.stringify(Object.create(null))",
            "var a=[]; a.length=3; JSON.stringify(a)",
            "var a=[1]; a.x=2; JSON.stringify(a)",
        ],
    );
}

/* ========================================================== jsbuiltin.c */

/// rows 491..495 + eval with non-string arguments
#[test]
fn builtin_uri_parse_and_eval() {
    /* rows 494, 495: Decode */
    let enc = [
        "%", "%A", "%a", "%zz", "%g0", "%0g", "%%", "%%41", "%41", "%4", "abc%", "a%2",
        "%2G", "%C3%A9", "%C3", "%C3%28", "%E4%B8%AD", "%E4%B8", "%F0%9F%98%80",
        "%F0%9F%98", "%ED%A0%80", "%ED%BF%BF", "%C0%80", "%FE%FF", "%FF", "%80", "%00",
        "%7F", "%2F", "%3F", "%23", "a%2Fb", "%u0041", "", "abc", "a b", "a+b",
    ];
    for e in enc {
        for f in [
            "decodeURI",
            "decodeURIComponent",
            "encodeURI",
            "encodeURIComponent",
            "escape",
            "unescape",
        ] {
            ev("URI", &format!("{}({})", f, jsstr(e)));
        }
    }
    /* lone surrogates and astral characters through the encoders */
    let raw = [
        "'\\ud800'",
        "'\\udbff'",
        "'\\udc00'",
        "'\\udfff'",
        "'\\ud800\\ud800'",
        "'\\udc00\\ud800'",
        "'\\ud83d\\ude00'",
        "'a\\ud800b'",
        "'\\u00e9'",
        "'\\u4e2d'",
        "'\\u0000'",
        "'\\ufffd'",
        "'\\uffff'",
    ];
    for r in raw {
        for f in [
            "encodeURI",
            "encodeURIComponent",
            "decodeURI",
            "decodeURIComponent",
            "escape",
            "unescape",
        ] {
            ev("URI_raw", &format!("{}({})", f, r));
        }
    }
    evs(
        "URI_args",
        &[
            "encodeURI()",
            "decodeURI()",
            "encodeURIComponent()",
            "decodeURIComponent()",
            "encodeURI(null)",
            "decodeURI(null)",
            "encodeURI(1)",
            "decodeURI(1)",
            "encodeURI({})",
            "decodeURI({})",
            "encodeURI.length",
            "decodeURI.length",
            "encodeURI(\";/?:@&=+$,#\")",
            "encodeURIComponent(\";/?:@&=+$,#\")",
            "decodeURI(encodeURI('a b/c?d')) ",
            "decodeURIComponent(encodeURIComponent('a b/c?d'))",
        ],
    );

    /* rows 491, 492: parseInt */
    let radices = [
        "", "0", "1", "2", "8", "10", "16", "35", "36", "37", "-1", "1.5", "2.9", "NaN",
        "Infinity", "-Infinity", "'16'", "'x'", "null", "undefined", "true", "{}", "[]",
        "1e300",
    ];
    let subjects = [
        "'10'", "'abc'", "''", "' '", "'0x10'", "'0X10'", "'-10'", "'+10'", "'  12ab'",
        "'1.9'", "'.5'", "'Infinity'", "'NaN'", "'zz'", "'ZZ'", "'-0'", "'08'",
        "'9007199254740993'", "'1e3'", "0", "1.9", "NaN", "Infinity", "null", "undefined",
        "true", "{}", "[]", "'\\u00e9'",
    ];
    for s in subjects {
        for r in radices {
            let sep = if r.is_empty() { "" } else { ", " };
            ev("parseInt", &format!("parseInt({}{}{})", s, sep, r));
        }
        ev("parseFloat", &format!("parseFloat({})", s));
    }
    evs(
        "parse_misc",
        &[
            "parseInt()",
            "parseFloat()",
            "parseInt.length",
            "parseFloat.length",
            "parseFloat('1e')",
            "parseFloat('1e+')",
            "parseFloat('-Infinityx')",
            "parseFloat('.')",
            "parseFloat('-.')",
            "parseFloat('  -1.5e2xyz')",
            "isNaN(parseInt('x'))",
            "isFinite(parseFloat('Infinity'))",
        ],
    );

    /* eval with non-string arguments */
    evs(
        "eval_args",
        &[
            "eval()",
            "eval(1)",
            "eval(null)",
            "eval(undefined)",
            "eval(true)",
            "eval({})",
            "eval([1])",
            "eval([])",
            "eval(function(){})",
            "eval('1+1')",
            "eval('var q=3; q')",
            "eval('@')",
            "eval('throw new Error(\"ev\")')",
            "eval(new String('1+1'))",
            "eval.length",
            "typeof eval",
            "eval('eval(\"1\")')",
            "(function(){ return eval('typeof arguments') })()",
        ],
    );

    /* the remaining jsbuiltin.c globals */
    evs(
        "builtin_globals",
        &[
            "isNaN()",
            "isNaN(NaN)",
            "isNaN('x')",
            "isFinite()",
            "isFinite('1')",
            "String(undefined)",
            "String()",
            "Number()",
            "Number('')",
            "Number('0x10')",
            "Number('  12  ')",
            "Number('12x')",
            "Number(null)",
            "Number(undefined)",
            "Number([])",
            "Number([1])",
            "Number([1,2])",
            "Number({})",
            "[typeof globalThis, typeof undefined, typeof NaN, typeof Infinity].join(',')",
        ],
    );
}

/* ============================================================ jsrepr.c */

/// rows 496..499 through the exported repr entry points
#[test]
fn repr_cycles_and_nesting() {
    /* rows 496, 497: the cycle guard makes the repeated object print as {} / [] */
    evs(
        "repr_cycle_eval",
        &[
            "var a={}; a.a=a; a",
            "var a=[]; a[0]=a; a",
            "var a={}; a.b={c:a}; a",
            "var a=[]; a[0]=[a]; a",
            "var a={}; a.a=a; [a]",
            "var a=[]; a[0]=a; ({x:a})",
            "var a={x:1}; ({p:a,q:a})",
            "var a=[1]; [a,a]",
            "var a={}; a['a b']=1; a['0']=2; a['_x']=3; a",
            "({'':1})",
            "[,1,,2]",
            "var a=[]; a.length=3; a",
            "/re/g",
            "new Date(0)",
            "function f(a,b){}",
            "Math.max",
            "new Error('e')",
        ],
    );
    /* row 499: js_tryrepr must swallow a throwing getter and yield the placeholder */
    evs(
        "repr_throwing_getter",
        &[
            "var o={}; Object.defineProperty(o,'x',{get:function(){throw new Error('g')},\
             enumerable:true}); o",
            "var o={}; Object.defineProperty(o,'x',{get:function(){throw 1},\
             enumerable:true}); [o]",
            "var o={}; Object.defineProperty(o,'x',{get:function(){throw 1},\
             enumerable:false}); o",
            "var a=[]; Object.defineProperty(a,'0',{get:function(){throw 1},\
             enumerable:true}); a.length=1; a",
        ],
    );

    /* rows 496..499 through the raw API: js_repr / js_torepr / js_tryrepr */
    fn cyc_obj(a: &Api, J: JS) {
        unsafe {
            (a.js_newobject)(J);
            (a.js_copy)(J, -1);
            (a.js_setproperty)(J, -2, cs("self").as_ptr());
            /* row 498: js_repr always buffers at least the NUL terminator */
            (a.js_repr)(J, -1);
            emit(&format!("repr={}", str_at(a, J, -1)));
            (a.js_pop)(J, 1);
            /* js_torepr replaces in place */
            (a.js_copy)(J, -1);
            emit(&format!("torepr={}", rs((a.js_torepr)(J, -1))));
            (a.js_pop)(J, 1);
            let e = cs("<PLACE>");
            emit(&format!(
                "tryrepr={}",
                rs((a.js_tryrepr)(J, -1, e.as_ptr()))
            ));
        }
    }
    diff_native("repr_cyclic_object", cyc_obj, 0);
    diff_native("repr_cyclic_object_strict", cyc_obj, JS_STRICT);

    fn cyc_arr(a: &Api, J: JS) {
        unsafe {
            (a.js_newarray)(J);
            (a.js_copy)(J, -1);
            (a.js_setindex)(J, -2, 0);
            (a.js_repr)(J, -1);
            emit(&format!("repr={}", str_at(a, J, -1)));
            (a.js_pop)(J, 1);
            let e = cs("<PLACE>");
            emit(&format!("tryrepr={}", rs((a.js_tryrepr)(J, -1, e.as_ptr()))));
            emit(&format!("top={}", (a.js_gettop)(J)));
        }
    }
    diff_native("repr_cyclic_array", cyc_arr, 0);
    diff_native("repr_cyclic_array_strict", cyc_arr, JS_STRICT);

    /* deeply nested (but finite) structures */
    fn deep(a: &Api, J: JS) {
        unsafe {
            let depth = pi(0) as c_int;
            (a.js_newarray)(J);
            for _ in 0..depth {
                (a.js_newarray)(J);
                (a.js_copy)(J, -2);
                (a.js_setindex)(J, -2, 0);
                (a.js_remove)(J, -2);
            }
            let e = cs("<PLACE>");
            let s = rs((a.js_tryrepr)(J, -1, e.as_ptr()));
            emit(&format!("len={}", s.len()));
            emit(&format!("head={}", &s[..s.len().min(20)]));
        }
    }
    for d in [0i64, 1, 5, 40, 200] {
        set_pi(0, d);
        diff_native("repr_deep_array", deep, 0);
        diff_native("repr_deep_array_strict", deep, JS_STRICT);
    }

    /* js_tryrepr on a value whose repr throws, via the raw API */
    fn throwing(a: &Api, J: JS) {
        unsafe {
            let name = cs("test.js");
            let src = cs(
                "var o={}; Object.defineProperty(o,'x',\
                 {get:function(){throw new Error('boom')},enumerable:true}); o",
            );
            (a.js_loadstring)(J, name.as_ptr(), src.as_ptr());
            (a.js_pushundefined)(J);
            (a.js_call)(J, 0);
            let e = cs("<PLACE>");
            emit(&format!("tryrepr={}", rs((a.js_tryrepr)(J, -1, e.as_ptr()))));
            emit(&format!("top={}", (a.js_gettop)(J)));
            emit(&format!("isobject={}", (a.js_isobject)(J, -1)));
        }
    }
    diff_native("repr_tryrepr_throws", throwing, 0);
    diff_native("repr_tryrepr_throws_strict", throwing, JS_STRICT);

    /* js_repr of every primitive shape */
    fn prims(a: &Api, J: JS) {
        unsafe {
            (a.js_pushundefined)(J);
            (a.js_pushnull)(J);
            (a.js_pushboolean)(J, 1);
            (a.js_pushnumber)(J, -0.0);
            (a.js_pushnumber)(J, f64::NAN);
            (a.js_pushnumber)(J, f64::INFINITY);
            (a.js_pushstring)(J, cs("a\"b\\c\n").as_ptr());
            (a.js_pushstring)(J, cs("\u{e9}\u{4e2d}\u{1f600}").as_ptr());
            (a.js_newobject)(J);
            (a.js_newarray)(J);
            let n = (a.js_gettop)(J);
            for i in 0..n {
                emit(&format!("[{}]={}", i, repr_at(a, J, i)));
            }
            (a.js_pop)(J, n - 1);
        }
    }
    diff_native("repr_primitives", prims, 0);
    diff_native("repr_primitives_strict", prims, JS_STRICT);
}

/* ==================================================== randomized sweeps */

/// Randomized numeric-method coverage (jsnumber.c, jsmath.c) with a fixed seed.
#[test]
fn random_number_and_math() {
    let mut rng = Rng::new(SEED);
    let unary = [
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "round", "sin",
        "sqrt", "tan",
    ];
    for _ in 0..400 {
        let x = rng.f64();
        let xs = jsnum(x);
        let f = unary[rng.below(unary.len() as u64) as usize];
        ev("rnd_math_unary", &format!("Math.{}({})", f, xs));
        let y = jsnum(rng.f64());
        ev("rnd_math_pow", &format!("Math.pow({}, {})", xs, y));
        ev("rnd_math_atan2", &format!("Math.atan2({}, {})", xs, y));
        ev("rnd_math_max", &format!("Math.max({}, {})", xs, y));
        ev("rnd_math_min", &format!("Math.min({}, {}, {})", xs, y, xs));
        ev("rnd_num_tostring", &format!("String({})", xs));
        let radix = rng.range_i64(-2, 40);
        ev(
            "rnd_num_radix",
            &format!("({}).toString({})", xs, radix),
        );
        let d = rng.range_i64(-3, 105);
        ev("rnd_toFixed", &format!("({}).toFixed({})", xs, d));
        ev("rnd_toExponential", &format!("({}).toExponential({})", xs, d));
        ev("rnd_toPrecision", &format!("({}).toPrecision({})", xs, d));
        ev("rnd_new_Array", &format!("var a=new Array({}); a.length", xs));
        ev("rnd_json_space", &format!("JSON.stringify({{a:1}}, null, {})", xs));
        ev("rnd_date", &format!("new Date({}).getTime()", xs));
        ev(
            "rnd_date_iso",
            &format!("try {{ new Date({}).toISOString() }} catch (e) {{ String(e) }}", xs),
        );
        ev("rnd_date_str", &format!("String(new Date({}))", xs));
        ev("rnd_parseInt", &format!("parseInt(String({}), {})", xs, radix));
        ev("rnd_parseFloat", &format!("parseFloat(String({}))", xs));
    }
}

/// Randomized string-method coverage (jsstring.c, json.c, jsbuiltin.c).
#[test]
fn random_string_methods() {
    let mut rng = Rng::new(SEED ^ 0xA5A5);
    for _ in 0..300 {
        let s = rng.string(14);
        let sl = jsstr(&s);
        let n = jsnum(rng.f64());
        let t = jsstr(&rng.string(3));
        ev("rnd_charAt", &format!("{}.charAt({})", sl, n));
        ev("rnd_charCodeAt", &format!("{}.charCodeAt({})", sl, n));
        ev("rnd_indexOf", &format!("{}.indexOf({}, {})", sl, t, n));
        ev("rnd_lastIndexOf", &format!("{}.lastIndexOf({}, {})", sl, t, n));
        ev("rnd_slice", &format!("JSON.stringify({}.slice({}))", sl, n));
        ev("rnd_substring", &format!("JSON.stringify({}.substring(0, {}))", sl, n));
        ev("rnd_substr", &format!("JSON.stringify({}.substr({}, 2))", sl, n));
        ev("rnd_split", &format!("JSON.stringify({}.split({}, {}))", sl, t, n));
        ev("rnd_replace", &format!("JSON.stringify({}.replace({}, {}))", sl, t, n));
        ev(
            "rnd_replace_fn",
            &format!(
                "JSON.stringify({}.replace({}, function(m){{return m.length}}))",
                sl, t
            ),
        );
        ev("rnd_case", &format!("JSON.stringify([{}.toUpperCase(), {}.toLowerCase()])", sl, sl));
        ev("rnd_trim", &format!("JSON.stringify({}.trim())", sl));
        ev("rnd_concat", &format!("JSON.stringify({}.concat({}, {}))", sl, t, n));
        ev("rnd_json_str", &format!("JSON.stringify({})", sl));
        ev(
            "rnd_json_round",
            &format!("JSON.stringify(JSON.parse(JSON.stringify({})))", sl),
        );
        ev("rnd_json_parse", &format!("try {{ JSON.stringify(JSON.parse({})) }} catch (e) {{ String(e) }}", sl));
        ev("rnd_uri", &format!(
            "try {{ encodeURIComponent({}) }} catch (e) {{ String(e) }}", sl));
        ev("rnd_uri_dec", &format!(
            "try {{ decodeURIComponent({}) }} catch (e) {{ String(e) }}", sl));
        ev("rnd_uri_round", &format!(
            "try {{ decodeURI(encodeURI({})) === {} }} catch (e) {{ String(e) }}", sl, sl));
        ev("rnd_escape", &format!("unescape(escape({})) === {}", sl, sl));
        ev("rnd_parseInt_s", &format!("parseInt({})", sl));
        ev("rnd_parseFloat_s", &format!("parseFloat({})", sl));
        ev("rnd_date_parse", &format!("Date.parse({})", sl));
        ev("rnd_fromCharCode", &format!("JSON.stringify(String.fromCharCode({}))", n));
        ev("rnd_obj_key", &format!("var o={{}}; o[{}]=1; JSON.stringify(o)", sl));
        ev("rnd_repr", sl.as_str());
    }
}

/// Randomized Array / Object / Function coverage.
#[test]
fn random_array_object_function() {
    let mut rng = Rng::new(SEED ^ 0x1234_5678);
    let amethods = [
        "join", "pop", "push", "reverse", "shift", "slice", "sort", "splice", "unshift",
        "indexOf", "lastIndexOf", "concat", "toString",
    ];
    let cbmethods = ["every", "some", "forEach", "map", "filter", "reduce", "reduceRight"];
    let badvals = ["1", "'x'", "null", "undefined", "true", "{}", "[]", "NaN", "0"];
    for _ in 0..260 {
        let m = amethods[rng.below(amethods.len() as u64) as usize];
        let n = jsnum(rng.f64());
        ev("rnd_a_method", &format!("JSON.stringify([1,2,3].{}({}))", m, n));
        ev(
            "rnd_a_method_this",
            &format!("JSON.stringify(Array.prototype.{}.call({{length:2,0:1,1:2}}, {}))", m, n),
        );
        ev(
            "rnd_a_method_len",
            &format!("var a=[1,2,3]; a.length={}; JSON.stringify(a.{}())", n, m),
        );
        let c = cbmethods[rng.below(cbmethods.len() as u64) as usize];
        let b = badvals[rng.below(badvals.len() as u64) as usize];
        ev("rnd_a_badcb", &format!("[1,2].{}({})", c, b));
        ev(
            "rnd_a_cb",
            &format!("JSON.stringify([1,2,3].{}(function(v,i,a){{return v>{}}}))", c, n),
        );
        ev("rnd_o_static", &format!("Object.keys({})", b));
        ev("rnd_o_gopd", &format!("Object.getOwnPropertyDescriptor({}, 'x')", b));
        ev("rnd_o_defprop", &format!("Object.defineProperty({{}}, 'x', {{value:{}}})", b));
        ev("rnd_o_defprop_get", &format!("Object.defineProperty({{}}, 'x', {{get:{}}})", b));
        ev("rnd_o_create", &format!("typeof Object.create({})", b));
        ev("rnd_o_tostring", &format!("Object.prototype.toString.call({})", b));
        ev("rnd_f_apply", &format!(
            "(function(){{return arguments.length}}).apply(null, {{length:{}}})", n));
        ev("rnd_f_apply2", &format!(
            "(function(){{return arguments.length}}).apply(null, {})", b));
        ev("rnd_f_call", &format!("Function.prototype.call.call({})", b));
        ev("rnd_f_bind", &format!("Function.prototype.bind.call({})", b));
        ev("rnd_f_tostring", &format!("Function.prototype.toString.call({})", b));
        ev("rnd_b_sort", &format!("[3,1,2].sort({})", b));
    }
}

/// Randomized jsdate.c coverage: constructors, setters, getters, Date.parse.
#[test]
fn random_date() {
    let mut rng = Rng::new(SEED ^ 0xDA7E);
    let setters = [
        "setTime",
        "setMilliseconds",
        "setSeconds",
        "setMinutes",
        "setHours",
        "setDate",
        "setMonth",
        "setFullYear",
        "setUTCMilliseconds",
        "setUTCSeconds",
        "setUTCMinutes",
        "setUTCHours",
        "setUTCDate",
        "setUTCMonth",
        "setUTCFullYear",
    ];
    let getters = [
        "getTime",
        "getFullYear",
        "getMonth",
        "getDate",
        "getDay",
        "getHours",
        "getMinutes",
        "getSeconds",
        "getMilliseconds",
        "getUTCFullYear",
        "getUTCMonth",
        "getUTCDate",
        "getUTCDay",
        "getUTCHours",
        "getUTCMinutes",
        "getUTCSeconds",
        "getUTCMilliseconds",
        "getTimezoneOffset",
    ];
    let fmts = [
        "toString",
        "toDateString",
        "toTimeString",
        "toUTCString",
        "toLocaleString",
        "toLocaleDateString",
        "toLocaleTimeString",
    ];
    for _ in 0..220 {
        /* a millisecond value biased towards the legal +-8.64e15 window */
        let t = if rng.below(2) == 0 {
            rng.range_i64(-8_640_000_000_000_100, 8_640_000_000_000_100) as f64
        } else {
            rng.f64()
        };
        let ts = jsnum(t);
        let s = setters[rng.below(setters.len() as u64) as usize];
        let g = getters[rng.below(getters.len() as u64) as usize];
        let f = fmts[rng.below(fmts.len() as u64) as usize];
        let v = jsnum(rng.f64());
        ev("rnd_d_get", &format!("new Date({}).{}()", ts, g));
        ev("rnd_d_fmt", &format!("new Date({}).{}()", ts, f));
        ev(
            "rnd_d_iso",
            &format!("try {{ new Date({}).toISOString() }} catch (e) {{ String(e) }}", ts),
        );
        ev("rnd_d_json", &format!("JSON.stringify(new Date({}))", ts));
        ev(
            "rnd_d_set",
            &format!("var d=new Date({}); d.{}({}); d.getTime()", ts, s, v),
        );
        ev(
            "rnd_d_set_multi",
            &format!("var d=new Date({}); d.{}({}, {}); d.getTime()", ts, s, v, v),
        );
        ev(
            "rnd_d_ctor",
            &format!("new Date({}, {}).getTime()", jsnum(rng.f64()), jsnum(rng.f64())),
        );
        ev(
            "rnd_d_utc",
            &format!(
                "Date.UTC({}, {}, {}, {})",
                jsnum(rng.f64()),
                jsnum(rng.f64()),
                jsnum(rng.f64()),
                jsnum(rng.f64())
            ),
        );
        /* structured, mostly-plausible ISO strings with random field values */
        let iso = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}{}",
            rng.range_i64(0, 9999),
            rng.range_i64(0, 20),
            rng.range_i64(0, 40),
            rng.range_i64(0, 30),
            rng.range_i64(0, 70),
            rng.range_i64(0, 70),
            rng.range_i64(0, 999),
            ["", "Z", "+01:00", "-05:30", "+24:00", "x"][rng.below(6) as usize]
        );
        ev("rnd_d_parse", &format!("Date.parse({})", jsstr(&iso)));
        ev(
            "rnd_d_parse_trunc",
            &format!("Date.parse({})", jsstr(&iso[..1 + rng.below(iso.len() as u64) as usize])),
        );
    }
}
