//! CONFIGS.md rows 36-50 — objects, the property API under every attribute
//! combination, accessors, array shapes (flat vs hashed), globals, the
//! registry/ref API, iterators, C functions, constructors and userdata.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ------------------------------------------------------------------ */
/* row 36 — object constructors                                        */
/* ------------------------------------------------------------------ */
fn task_new_objects(api: &Api, J: JS) {
    unsafe {
        let ctors: [(&str, fn(&Api, JS)); 8] = [
            ("newobject", |a, j| unsafe { (a.js_newobject)(j) }),
            ("newobjectx", |a, j| unsafe { (a.js_newobjectx)(j) }),
            ("newarray", |a, j| unsafe { (a.js_newarray)(j) }),
            ("newboolean0", |a, j| unsafe { (a.js_newboolean)(j, 0) }),
            ("newboolean7", |a, j| unsafe { (a.js_newboolean)(j, 7) }),
            ("newnumber", |a, j| unsafe { (a.js_newnumber)(j, -0.0) }),
            ("newstring", |a, j| unsafe {
                (a.js_newstring)(j, cs("h\u{e9}llo").as_ptr())
            }),
            ("newregexp", |a, j| unsafe {
                (a.js_newregexp)(j, cs("(a)|b").as_ptr(), JS_REGEXP_G | JS_REGEXP_I)
            }),
        ];
        for (name, f) in ctors {
            f(api, J);
            emit(format!("{name}: {}", dump_slot(api, J, -1)));
            // a property round-trip on each
            (api.js_pushnumber)(J, 5.0);
            (api.js_setproperty)(J, -2, cs("k").as_ptr());
            (api.js_getproperty)(J, -1, cs("k").as_ptr());
            emit(format!("  k -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            emit(format!(
                "  has(k)={} has(nope)={} len={}",
                (api.js_hasproperty)(J, -1, cs("k").as_ptr()),
                {
                    let r = (api.js_hasproperty)(J, -1, cs("nope").as_ptr());
                    if r != 0 {
                        (api.js_pop)(J, 1);
                    }
                    r
                },
                (api.js_getlength)(J, -1)
            ));
            if (api.js_hasproperty)(J, -1, cs("k").as_ptr()) != 0 {
                (api.js_pop)(J, 1);
            }
            (api.js_delproperty)(J, -1, cs("k").as_ptr());
            emit(format!(
                "  after delete has(k)={}",
                (api.js_hasproperty)(J, -1, cs("k").as_ptr())
            ));
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn row36_object_constructors() {
    diff_all_flags("object constructors", task_new_objects);
}

/* ------------------------------------------------------------------ */
/* rows 37-38 — js_defproperty and friends, all 8 attribute combos      */
/* ------------------------------------------------------------------ */
fn attr_name(a: c_int) -> String {
    let mut v = Vec::new();
    if a & JS_READONLY != 0 {
        v.push("RO");
    }
    if a & JS_DONTENUM != 0 {
        v.push("DE");
    }
    if a & JS_DONTCONF != 0 {
        v.push("DC");
    }
    if v.is_empty() {
        "none".into()
    } else {
        v.join("|")
    }
}

fn task_properties(api: &Api, J: JS) {
    unsafe {
        // every attribute combination, plus out-of-range attribute ints
        let attrs: Vec<c_int> = (0..8).chain([8, 16, -1, i32::MAX, i32::MIN, 7 | 32]).collect();
        for atts in attrs {
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_defproperty)(J, -2, cs("p").as_ptr(), atts);
            emit(format!("atts={} ({atts})", attr_name(atts)));
            (api.js_getproperty)(J, -1, cs("p").as_ptr());
            emit(format!("  get p -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            // write attempt (throws in strict mode when read-only)
            (api.js_pushnumber)(J, 2.0);
            (api.js_setproperty)(J, -2, cs("p").as_ptr());
            (api.js_getproperty)(J, -1, cs("p").as_ptr());
            emit(format!("  after set p -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            // enumerability
            (api.js_pushiterator)(J, -1, 1);
            loop {
                let k = (api.js_nextiterator)(J, -1);
                match opt_str(k) {
                    None => break,
                    Some(b) => emit(format!("  iter own: {:?}", String::from_utf8_lossy(&b))),
                }
            }
            (api.js_pop)(J, 1);
            // configurability
            (api.js_delproperty)(J, -1, cs("p").as_ptr());
            emit(format!(
                "  after delete has={}",
                (api.js_hasproperty)(J, -1, cs("p").as_ptr())
            ));
            if (api.js_hasproperty)(J, -1, cs("p").as_ptr()) != 0 {
                (api.js_pop)(J, 1);
            }
            // redefining
            (api.js_pushnumber)(J, 3.0);
            (api.js_defproperty)(J, -2, cs("p").as_ptr(), 0);
            (api.js_getproperty)(J, -1, cs("p").as_ptr());
            emit(format!("  after redefine -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 2);
        }
    }
}

#[test]
fn row37_38_property_attributes() {
    diff("properties [flags=0]", Cfg::default(), task_properties);
    diff("properties [JS_STRICT]", Cfg::strict(), task_properties);
    diff("properties [custom alloc]", Cfg::alloc(), task_properties);
}

/* ------------------------------------------------------------------ */
/* row 39 — js_defaccessor                                             */
/* ------------------------------------------------------------------ */
unsafe extern "C-unwind" fn getter_7(J: JS) {
    // the active library is the one in SLOT; a getter only needs to push
    let (api, _) = current_api();
    unsafe { (api.js_pushnumber)(J, 7.0) }
}
unsafe extern "C-unwind" fn setter_rec(J: JS) {
    let (api, _) = current_api();
    unsafe {
        // record the assigned value on the receiver under a different name
        (api.js_copy)(J, 1);
        (api.js_setproperty)(J, 0, cs("__set").as_ptr());
        (api.js_pushundefined)(J);
    }
}

fn task_accessors(api: &Api, J: JS) {
    unsafe {
        for atts in 0..8 {
            for mode in 0..3 {
                (api.js_newobject)(J);
                match mode {
                    0 => {
                        // getter only
                        (api.js_newcfunction)(J, getter_7, sstr("g"), 0);
                        (api.js_pushnull)(J);
                    }
                    1 => {
                        // setter only
                        (api.js_pushnull)(J);
                        (api.js_newcfunction)(J, setter_rec, sstr("s"), 1);
                    }
                    _ => {
                        (api.js_newcfunction)(J, getter_7, sstr("g"), 0);
                        (api.js_newcfunction)(J, setter_rec, sstr("s"), 1);
                    }
                }
                (api.js_defaccessor)(J, -3, cs("acc").as_ptr(), atts);
                emit(format!("accessor mode={mode} atts={}", attr_name(atts)));
                (api.js_getproperty)(J, -1, cs("acc").as_ptr());
                emit(format!("  get -> {}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
                (api.js_pushnumber)(J, 99.0);
                (api.js_setproperty)(J, -2, cs("acc").as_ptr());
                (api.js_getproperty)(J, -1, cs("__set").as_ptr());
                emit(format!("  __set -> {}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
                (api.js_pushiterator)(J, -1, 1);
                loop {
                    let k = (api.js_nextiterator)(J, -1);
                    match opt_str(k) {
                        None => break,
                        Some(b) => emit(format!("  iter: {:?}", String::from_utf8_lossy(&b))),
                    }
                }
                (api.js_pop)(J, 2);
            }
        }
        // accessors defined from script must behave the same
        for src in [
            "var o={get x(){return 1}}; o.x",
            "var o={set x(v){this.y=v}}; o.x=2; o.y",
            "var o={get x(){return 1}, set x(v){this.y=v}}; o.x=3; o.x+','+o.y",
            "var o={}; Object.defineProperty(o,'x',{get:function(){return 4},enumerable:true}); \
             var s=''; for (var k in o) s+=k; s+o.x",
            "var o={}; Object.defineProperty(o,'x',{get:function(){throw new Error('boom')}}); \
             try { o.x } catch (e) { e.message }",
        ] {
            eval(api, J, "accessor", src);
        }
    }
}

#[test]
fn row39_accessors() {
    diff_all_flags("accessors", task_accessors);
}

/* ------------------------------------------------------------------ */
/* rows 40-41 — array index API on flat and hashed arrays              */
/* ------------------------------------------------------------------ */
fn dump_array(api: &Api, J: JS, idx: c_int) -> String {
    unsafe {
        let sentinel = cs("<throw>");
        let len = (api.js_getlength)(J, idx);
        let mut o = format!("len={len}");
        for i in -1..(len + 3).min(64) {
            let has = (api.js_hasindex)(J, idx, i);
            if has != 0 {
                o.push_str(&format!(
                    " [{i}]={:?}",
                    s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
                ));
                (api.js_pop)(J, 1);
            } else {
                o.push_str(&format!(" [{i}]=-"));
            }
        }
        o.push_str(&format!(
            " repr={:?}",
            s((api.js_tryrepr)(J, idx, sentinel.as_ptr()))
        ));
        o
    }
}

fn task_arrays(api: &Api, J: JS, unflatten: bool) {
    unsafe {
        let mut rng = Rng::new(if unflatten { 0xC0DE_0041 } else { 0xC0DE_0040 });
        for round in 0..120 {
            (api.js_newarray)(J);
            if unflatten {
                // force the hashed representation up front
                let o = (api.js_toobject)(J, -1);
                (api.jsR_unflattenarray)(J, o);
            }
            emit(format!("r{round} fresh {}", dump_array(api, J, -1)));
            for step in 0..10 {
                let op = rng.below(8);
                let i = (rng.below(12) as c_int) - 2;
                match op {
                    0 => {
                        (api.js_pushnumber)(J, i as f64);
                        (api.js_setindex)(J, -2, i);
                        emit(format!("r{round}s{step} set[{i}] {}", dump_array(api, J, -1)));
                    }
                    1 => {
                        (api.js_delindex)(J, -1, i);
                        emit(format!("r{round}s{step} del[{i}] {}", dump_array(api, J, -1)));
                    }
                    2 => {
                        let l = rng.below(10) as c_int;
                        (api.js_setlength)(J, -1, l);
                        emit(format!("r{round}s{step} setlen({l}) {}", dump_array(api, J, -1)));
                    }
                    3 => {
                        // a non-index key converts the array away from `simple`
                        (api.js_pushstring)(J, cs("v").as_ptr());
                        (api.js_setproperty)(J, -2, cs("key").as_ptr());
                        emit(format!("r{round}s{step} setprop {}", dump_array(api, J, -1)));
                    }
                    4 => {
                        // large sparse index
                        let big = 1000 + rng.below(1000) as c_int;
                        (api.js_pushnumber)(J, 1.0);
                        (api.js_setindex)(J, -2, big);
                        emit(format!(
                            "r{round}s{step} sparse[{big}] {}",
                            dump_array(api, J, -1)
                        ));
                    }
                    5 => {
                        let o = (api.js_toobject)(J, -1);
                        (api.jsR_unflattenarray)(J, o);
                        emit(format!("r{round}s{step} unflatten {}", dump_array(api, J, -1)));
                    }
                    6 => {
                        (api.js_getindex)(J, -1, i);
                        emit(format!(
                            "r{round}s{step} get[{i}] -> {}",
                            dump_slot(api, J, -1)
                        ));
                        (api.js_pop)(J, 1);
                    }
                    _ => {
                        // push a non-number element
                        (api.js_pushstring)(J, cs("s").as_ptr());
                        (api.js_setindex)(J, -2, i);
                        emit(format!("r{round}s{step} setstr[{i}] {}", dump_array(api, J, -1)));
                    }
                }
            }
            (api.js_pop)(J, 1);
        }
        // js_setlength boundary values
        for l in [0, 1, -1, i32::MAX, i32::MIN, 1 << 20] {
            (api.js_newarray)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_setindex)(J, -2, 0);
            (api.js_setlength)(J, -1, l);
            emit(format!("setlength({l}) -> len={}", (api.js_getlength)(J, -1)));
            (api.js_pop)(J, 1);
        }
        // jsV_resizearray on a hashed array (asserts !simple in C)
        for l in [0, 1, 5, 100] {
            (api.js_newarray)(J);
            let o = (api.js_toobject)(J, -1);
            (api.jsR_unflattenarray)(J, o);
            (api.jsV_resizearray)(J, o, l);
            emit(format!("resizearray({l}) -> {}", dump_array(api, J, -1)));
            (api.js_pop)(J, 1);
        }
    }
}

fn task_arrays_flat(api: &Api, J: JS) {
    task_arrays(api, J, false)
}
fn task_arrays_hashed(api: &Api, J: JS) {
    task_arrays(api, J, true)
}

#[test]
fn row40_arrays_flat() {
    diff_all_flags("arrays (flat)", task_arrays_flat);
}
#[test]
fn row41_arrays_hashed() {
    diff_all_flags("arrays (hashed)", task_arrays_hashed);
}

/* ------------------------------------------------------------------ */
/* row 42 — globals with attributes                                    */
/* ------------------------------------------------------------------ */
fn task_globals(api: &Api, J: JS) {
    unsafe {
        for atts in 0..8 {
            let n = format!("g{atts}");
            let name = cs(&n);
            (api.js_pushnumber)(J, atts as f64);
            (api.js_defglobal)(J, name.as_ptr(), atts);
            (api.js_getglobal)(J, name.as_ptr());
            emit(format!("defglobal {n} {}: {}", attr_name(atts), dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_pushnumber)(J, 100.0);
            (api.js_setglobal)(J, name.as_ptr());
            (api.js_getglobal)(J, name.as_ptr());
            emit(format!("  after setglobal: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            eval(api, J, &format!("read {n}"), &format!("{n}"));
            eval(api, J, &format!("write {n}"), &format!("{n} = 555; {n}"));
            eval(
                api,
                J,
                &format!("enum {n}"),
                &format!("var s=''; for (var k in this) if (k=='{n}') s='seen'; s"),
            );
            (api.js_delglobal)(J, name.as_ptr());
            (api.js_getglobal)(J, name.as_ptr());
            emit(format!("  after delglobal: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        // deleting / reading globals that do not exist
        (api.js_delglobal)(J, cs("neverdefined").as_ptr());
        (api.js_getglobal)(J, cs("neverdefined").as_ptr());
        emit(format!("undefined global: {}", dump_slot(api, J, -1)));
        (api.js_pop)(J, 1);
        // js_pushglobal identity
        (api.js_pushglobal)(J);
        (api.js_pushglobal)(J);
        emit(format!("pushglobal strictequal={}", (api.js_strictequal)(J)));
        (api.js_pop)(J, 2);
    }
}

#[test]
fn row42_globals() {
    diff_all_flags("globals", task_globals);
}

/* ------------------------------------------------------------------ */
/* row 43 — registry and refs                                          */
/* ------------------------------------------------------------------ */

/// Primitive value shapes only (no addresses in their `js_ref` name).
unsafe fn push_shape_lite(api: &Api, J: JS, k: c_int) {
    unsafe {
        match k {
            0 => (api.js_pushundefined)(J),
            1 => (api.js_pushnull)(J),
            2 => (api.js_pushboolean)(J, 0),
            3 => (api.js_pushboolean)(J, 1),
            4 => (api.js_pushboolean)(J, 42),
            5 => (api.js_pushnumber)(J, 0.0),
            6 => (api.js_pushnumber)(J, -0.0),
            7 => (api.js_pushnumber)(J, 1.5),
            8 => (api.js_pushnumber)(J, f64::NAN),
            9 => (api.js_pushnumber)(J, f64::INFINITY),
            10 => (api.js_pushstring)(J, cs("").as_ptr()),
            11 => (api.js_pushstring)(J, cs("abc").as_ptr()),
            12 => (api.js_pushstring)(J, cs("0123456789abcdef").as_ptr()),
            13 => (api.js_pushliteral)(J, sstr("lit")),
            14 => (api.js_pushlstring)(J, b"a\0b".as_ptr() as *const c_char, 3),
            _ => (api.js_pushstring)(J, cs("\u{20ac}").as_ptr()),
        }
    }
}

fn task_registry(api: &Api, J: JS) {
    unsafe {
        for k in ["a", "", "x/y", "0", "very long registry key name 0123456789"] {
            let key = cs(k);
            (api.js_pushnumber)(J, 1.5);
            (api.js_setregistry)(J, key.as_ptr());
            (api.js_getregistry)(J, key.as_ptr());
            emit(format!("registry {k:?}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_delregistry)(J, key.as_ptr());
            (api.js_getregistry)(J, key.as_ptr());
            emit(format!("  after del: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        (api.js_getregistry)(J, cs("nope").as_ptr());
        emit(format!("missing registry: {}", dump_slot(api, J, -1)));
        (api.js_pop)(J, 1);

        // js_ref / js_unref.
        // For JS_TOBJECT, jsrun.c:949 formats the object *address* with "%p",
        // which can never match between two libraries. Normalise that one shape;
        // every other branch (`_Undefined`, `_Null`, `_True`, `_False` and the
        // `J->nextref` counter) is deterministic and IS compared verbatim.
        fn norm(b: &[u8]) -> String {
            let t = String::from_utf8_lossy(b).into_owned();
            if t.starts_with("0x") && t.len() > 2 && t[2..].bytes().all(|c| c.is_ascii_hexdigit()) {
                "<objptr>".to_string()
            } else {
                t
            }
        }
        let mut refs: Vec<Vec<u8>> = Vec::new();
        for k in 0..8 {
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, k as f64);
            (api.js_setproperty)(J, -2, cs("id").as_ptr());
            let r = (api.js_ref)(J);
            let rb = opt_str(r).unwrap_or_default();
            emit(format!("ref {k} -> {}", norm(&rb)));
            refs.push(rb);
        }
        // every non-object value shape: fully deterministic ref names
        for k in 0..16 {
            push_shape_lite(api, J, k);
            let r = (api.js_ref)(J);
            let rb = opt_str(r).unwrap_or_default();
            emit(format!("ref shape {k} -> {}", norm(&rb)));
            let mut z = rb.clone();
            z.push(0);
            (api.js_getregistry)(J, z.as_ptr() as *const c_char);
            emit(format!("  deref -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_unref)(J, z.as_ptr() as *const c_char);
        }
        for rb in &refs {
            let mut z = rb.clone();
            z.push(0);
            (api.js_getregistry)(J, z.as_ptr() as *const c_char);
            (api.js_getproperty)(J, -1, cs("id").as_ptr());
            emit(format!("  deref -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 2);
        }
        for rb in &refs {
            let mut z = rb.clone();
            z.push(0);
            (api.js_unref)(J, z.as_ptr() as *const c_char);
        }
        // unref an unknown ref, and unref twice
        (api.js_unref)(J, cs("nosuchref").as_ptr());
        for rb in &refs {
            let mut z = rb.clone();
            z.push(0);
            (api.js_unref)(J, z.as_ptr() as *const c_char);
        }
        emit("unref done".into());
        (api.js_gc)(J, 0);
    }
}

#[test]
fn row43_registry_and_refs() {
    diff_all_flags("registry and refs", task_registry);
}

/* ------------------------------------------------------------------ */
/* rows 44-45 — iterators, own and prototype-chain                     */
/* ------------------------------------------------------------------ */
fn task_iterators(api: &Api, J: JS) {
    unsafe {
        let setups: [(&str, &str); 8] = [
            ("plain", "({a:1,b:2,c:3})"),
            ("array", "[10,20,30]"),
            ("sparse array", "(function(){var a=[1]; a[5]=2; return a})()"),
            ("string object", "new String('abc')"),
            ("proto chain", "Object.create({inherited:1,shadow:2})"),
            ("with dontenum", "Object.defineProperty({v:1},'h',{value:2,enumerable:false})"),
            ("empty", "({})"),
            ("nested", "({a:{b:1}})"),
        ];
        for (name, src) in setups {
            for own in [0, 1, 2, -1] {
                let f = cs("iter.js");
                let c = cs(src);
                if (api.js_ploadstring)(J, f.as_ptr(), c.as_ptr()) != 0 {
                    (api.js_pop)(J, 1);
                    continue;
                }
                (api.js_pushundefined)(J);
                if (api.js_pcall)(J, 0) != 0 {
                    (api.js_pop)(J, 1);
                    continue;
                }
                // add an own shadowing property for the proto-chain case
                if name == "proto chain" {
                    (api.js_pushnumber)(J, 9.0);
                    (api.js_setproperty)(J, -2, cs("shadow").as_ptr());
                }
                (api.js_pushiterator)(J, -1, own);
                let mut keys = Vec::new();
                loop {
                    let k = (api.js_nextiterator)(J, -1);
                    match opt_str(k) {
                        None => break,
                        Some(b) => {
                            keys.push(String::from_utf8_lossy(&b).into_owned());
                            if keys.len() > 200 {
                                break;
                            }
                        }
                    }
                }
                emit(format!("{name} own={own}: {keys:?}"));
                // exhausted iterator keeps returning NULL
                let again = (api.js_nextiterator)(J, -1);
                emit(format!("  exhausted -> {:?}", opt_str(again)));
                (api.js_pop)(J, 2);
            }
        }
        // jsV_newiterator / jsV_nextiterator directly
        for src in ["({a:1,b:2})", "[1,2,3]", "({})"] {
            let f = cs("iter.js");
            let c = cs(src);
            if (api.js_ploadstring)(J, f.as_ptr(), c.as_ptr()) != 0 {
                (api.js_pop)(J, 1);
                continue;
            }
            (api.js_pushundefined)(J);
            if (api.js_pcall)(J, 0) != 0 {
                (api.js_pop)(J, 1);
                continue;
            }
            let o = (api.js_toobject)(J, -1);
            for own in [0, 1] {
                let it = (api.jsV_newiterator)(J, o, own);
                let mut keys = Vec::new();
                loop {
                    match opt_str((api.jsV_nextiterator)(J, it)) {
                        None => break,
                        Some(b) => {
                            keys.push(String::from_utf8_lossy(&b).into_owned());
                            if keys.len() > 200 {
                                break;
                            }
                        }
                    }
                }
                emit(format!("jsV iter {src} own={own}: {keys:?}"));
            }
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn row44_45_iterators() {
    diff_all_flags("iterators", task_iterators);
}

/* ------------------------------------------------------------------ */
/* rows 46-48 — C functions, cfunctionx data, constructors             */
/* ------------------------------------------------------------------ */
fn current_api() -> (&'static Api, ()) {
    // The trampoline installs the active Api in SLOT before the call; callbacks
    // registered from a task run under the same library.
    (active_api(), ())
}

unsafe extern "C-unwind" fn cf_report(J: JS) {
    let api = active_api();
    unsafe {
        let top = (api.js_gettop)(J);
        let mut o = format!("cfun top={top}");
        for i in 0..top {
            let sentinel = cs("<throw>");
            o.push_str(&format!(
                " [{i}]={:?}",
                s((api.js_tryrepr)(J, i, sentinel.as_ptr()))
            ));
        }
        let d = (api.js_currentfunctiondata)(J);
        o.push_str(&format!(" data={}", if d.is_null() { 0 } else { *(d as *const u64) }));
        (api.js_currentfunction)(J);
        o.push_str(&format!(" self={}", dump_slot(api, J, -1)));
        (api.js_pop)(J, 1);
        emit(o);
        (api.js_pushnumber)(J, top as f64);
    }
}

unsafe extern "C-unwind" fn cf_throw(J: JS) {
    let api = active_api();
    unsafe {
        (api.js_typeerror)(J, cs("thrown from C").as_ptr());
    }
}

unsafe extern "C-unwind" fn cf_ctor(J: JS) {
    let api = active_api();
    unsafe {
        (api.js_pushnumber)(J, (api.js_gettop)(J) as f64);
        (api.js_setproperty)(J, 0, cs("argc").as_ptr());
        (api.js_copy)(J, 0);
    }
}

static mut FINALIZED: u64 = 0;
unsafe extern "C-unwind" fn fin(_J: JS, _p: *mut c_void) {
    unsafe { FINALIZED += 1 }
}

fn task_cfunctions(api: &Api, J: JS) {
    unsafe {
        let mut data: u64 = 0xABCD;
        for length in [0, 1, 3] {
            // plain cfunction
            (api.js_newcfunction)(J, cf_report, sstr("f"), length);
            emit(format!("newcfunction len={length}: {}", dump_slot(api, J, -1)));
            for argc in 0..4 {
                let base = (api.js_gettop)(J);
                (api.js_copy)(J, -1);
                (api.js_pushnull)(J);
                for a in 0..argc {
                    (api.js_pushnumber)(J, a as f64);
                }
                let rc = (api.js_pcall)(J, argc);
                emit(format!(
                    "  call argc={argc} rc={rc} -> {}",
                    dump_slot(api, J, -1)
                ));
                restore_top(api, J, base);
            }
            (api.js_pop)(J, 1);

            // cfunctionx with data + finalizer
            let base = (api.js_gettop)(J);
            (api.js_newcfunctionx)(
                J,
                cf_report,
                sstr("fx"),
                length,
                &mut data as *mut u64 as *mut c_void,
                Some(fin),
            );
            (api.js_pushnull)(J);
            let rc = (api.js_pcall)(J, 0);
            emit(format!("newcfunctionx rc={rc} -> {}", dump_slot(api, J, -1)));
            restore_top(api, J, base);

            // cconstructor: called as function and with new
            (api.js_newcconstructor)(J, cf_report, cf_ctor, sstr("C"), length);
            let base = (api.js_gettop)(J);
            (api.js_copy)(J, -1);
            (api.js_pushnull)(J);
            let rc = (api.js_pcall)(J, 0);
            emit(format!("cconstructor as-call rc={rc} -> {}", dump_slot(api, J, -1)));
            restore_top(api, J, base);
            (api.js_copy)(J, -1);
            for a in 0..2 {
                (api.js_pushnumber)(J, a as f64);
            }
            let rc = (api.js_pconstruct)(J, 2);
            emit(format!("cconstructor as-new rc={rc} -> {}", dump_slot(api, J, -1)));
            if rc == 0 {
                (api.js_getproperty)(J, -1, cs("argc").as_ptr());
                emit(format!("  argc -> {}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
            }
            restore_top(api, J, base - 1);
        }
        // a throwing C function, caught by js_pcall and by JS try/catch
        let base = (api.js_gettop)(J);
        (api.js_newcfunction)(J, cf_throw, sstr("boom"), 0);
        (api.js_pushnull)(J);
        let rc = (api.js_pcall)(J, 0);
        emit(format!("throwing cfun rc={rc} -> {}", dump_slot(api, J, -1)));
        restore_top(api, J, base);
        (api.js_newcfunction)(J, cf_throw, sstr("boom"), 0);
        (api.js_setglobal)(J, cs("boom").as_ptr());
        eval(api, J, "catch C throw", "try { boom() } catch (e) { e.name+': '+e.message }");
        // pconstruct on something that is not a constructor
        let base = (api.js_gettop)(J);
        (api.js_newcfunction)(J, cf_report, sstr("nf"), 0);
        let rc = (api.js_pconstruct)(J, 0);
        emit(format!("pconstruct plain cfun rc={rc} -> {}", dump_slot(api, J, -1)));
        restore_top(api, J, base);
        (api.js_gc)(J, 0);
    }
}

#[test]
fn row46_47_48_cfunctions() {
    diff_all_flags("cfunctions", task_cfunctions);
}

/* ------------------------------------------------------------------ */
/* rows 49-50 — userdata, with and without hooks                       */
/* ------------------------------------------------------------------ */
unsafe extern "C-unwind" fn ud_has(J: JS, p: *mut c_void, name: *const c_char) -> c_int {
    let api = active_api();
    unsafe {
        let n = s(name);
        emit(format!("ud_has({n})"));
        if n == "magic" {
            (api.js_pushnumber)(J, *(p as *const u64) as f64);
            1
        } else {
            0
        }
    }
}
unsafe extern "C-unwind" fn ud_put(J: JS, p: *mut c_void, name: *const c_char) -> c_int {
    let api = active_api();
    unsafe {
        let n = s(name);
        let v = (api.js_tryinteger)(J, -1, -1);
        emit(format!("ud_put({n}={v})"));
        if n == "magic" {
            *(p as *mut u64) = v as u64;
            1
        } else {
            0
        }
    }
}
unsafe extern "C-unwind" fn ud_del(_J: JS, _p: *mut c_void, name: *const c_char) -> c_int {
    emit(format!("ud_del({})", unsafe { s(name) }));
    1
}

fn task_userdata(api: &Api, J: JS) {
    unsafe {
        let mut payload: u64 = 11;
        for tag in ["Tag", "", "Other"] {
            (api.js_newuserdata)(
                J,
                sstr(tag),
                &mut payload as *mut u64 as *mut c_void,
                Some(fin),
            );
            emit(format!("userdata tag={tag:?}: {}", dump_slot(api, J, -1)));
            for probe in ["Tag", "", "Other", "nope"] {
                let ok = (api.js_isuserdata)(J, -1, cs(probe).as_ptr());
                emit(format!("  isuserdata({probe:?})={ok}"));
                if ok != 0 {
                    let d = (api.js_touserdata)(J, -1, cs(probe).as_ptr());
                    emit(format!("    touserdata -> {}", *(d as *const u64)));
                }
            }
            // property access on plain userdata
            (api.js_pushnumber)(J, 1.0);
            (api.js_setproperty)(J, -2, cs("k").as_ptr());
            (api.js_getproperty)(J, -1, cs("k").as_ptr());
            emit(format!("  k -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 2);
        }
        // userdatax with hooks, driven from JS
        (api.js_newuserdatax)(
            J,
            sstr("Hooked"),
            &mut payload as *mut u64 as *mut c_void,
            Some(ud_has),
            Some(ud_put),
            Some(ud_del),
            Some(fin),
        );
        (api.js_setglobal)(J, cs("ud").as_ptr());
        for src in [
            "ud.magic",
            "ud.magic = 42",
            "ud.magic",
            "ud.other",
            "ud.other = 1; ud.other",
            "delete ud.magic",
            "'magic' in ud",
            "var s=''; for (var k in ud) s+=k+','; s",
            "typeof ud",
            "String(ud)",
        ] {
            eval(api, J, "userdatax", src);
        }
        (api.js_gc)(J, 0);
        (api.js_gc)(J, 1);
    }
}

#[test]
fn row49_50_userdata() {
    diff_all_flags("userdata", task_userdata);
}
