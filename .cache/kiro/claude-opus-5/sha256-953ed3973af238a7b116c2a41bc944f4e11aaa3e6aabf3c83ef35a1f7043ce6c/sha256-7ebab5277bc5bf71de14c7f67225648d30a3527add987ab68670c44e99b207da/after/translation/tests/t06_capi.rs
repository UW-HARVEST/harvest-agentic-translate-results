// Level 6: the rest of the public C API -- properties, indices, iterators,
// registry, references, userdata, constructors, buffers.
mod common;

use common::*;
use libloading::Symbol;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

fn sessions() -> (Session, Session) {
    (Session::new(Side::C, 0), Session::new(Side::Rust, 0))
}

/// Names used with property APIs. Kept as static NUL-terminated byte strings
/// because MuJS interns/stores some name pointers directly.
const NAMES: &[&[u8]] = &[
    b"a\0",
    b"b\0",
    b"length\0",
    b"toString\0",
    b"0\0",
    b"1\0",
    b"10\0",
    b"-1\0",
    b"\0",
    b"__proto__\0",
    b"constructor\0",
    b"prototype\0",
    b"4294967295\0",
    b"1.5\0",
    b"nope\0",
];

#[test]
fn property_api_matches() {
    let (cs, rs) = sessions();
    for name in NAMES {
        let nm = *name;
        for atts in [0i32, 1, 2, 4, 7] {
            let f = move |vm: &Vm, j: JsPtr| {
                let p = nm.as_ptr() as *const c_char;
                unsafe { (vm.newobject)(j) };
                logln(format!("has0={}", unsafe { (vm.hasproperty)(j, -1, p) }));
                // hasproperty pushes the value when found
                if unsafe { (vm.hasproperty)(j, -1, p) } != 0 {
                    logln(format!("hadval={:?}", stack_snapshot(vm, j)));
                    unsafe { (vm.pop)(j, 1) };
                }
                unsafe { (vm.pushnumber)(j, 42.0) };
                unsafe { (vm.setproperty)(j, -2, p) };
                logln(format!("after set: {:?}", stack_snapshot(vm, j)));
                unsafe { (vm.getproperty)(j, -1, p) };
                logln(format!("get={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.pushnumber)(j, 7.0) };
                unsafe { (vm.defproperty)(j, -2, p, atts) };
                unsafe { (vm.getproperty)(j, -1, p) };
                logln(format!("after def({}): {:?}", atts, stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                logln(format!("has1={}", unsafe { (vm.hasproperty)(j, -1, p) }));
                if unsafe { (vm.hasproperty)(j, -1, p) } != 0 {
                    unsafe { (vm.pop)(j, 1) };
                }
                unsafe { (vm.delproperty)(j, -1, p) };
                logln(format!("has2={}", unsafe { (vm.hasproperty)(j, -1, p) }));
                logln(format!("len={}", unsafe { (vm.getlength)(j, -1) }));
                logln(format!("final: {:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(
                &cs,
                &rs,
                &format!("property {:?} atts={}", nm, atts),
                f,
            );
        }
    }
}

#[test]
fn property_api_on_non_objects() {
    let (cs, rs) = sessions();
    let pushers: Vec<(&str, fn(&Vm, JsPtr))> = vec![
        ("undefined", |vm, j| unsafe { (vm.pushundefined)(j) }),
        ("null", |vm, j| unsafe { (vm.pushnull)(j) }),
        ("number", |vm, j| unsafe { (vm.pushnumber)(j, 3.5) }),
        ("boolean", |vm, j| unsafe { (vm.pushboolean)(j, 1) }),
        ("string", |vm, j| unsafe {
            (vm.pushliteral)(j, b"hello\0".as_ptr() as *const c_char)
        }),
        ("array", |vm, j| unsafe { (vm.newarray)(j) }),
        ("regexp", |vm, j| unsafe {
            (vm.newregexp)(j, b"a+\0".as_ptr() as *const c_char, 0)
        }),
        ("global", |vm, j| unsafe { (vm.pushglobal)(j) }),
    ];
    for (label, push) in pushers {
        for name in NAMES {
            let nm = *name;
            let p = push;
            let f = move |vm: &Vm, j: JsPtr| {
                p(vm, j);
                let np = nm.as_ptr() as *const c_char;
                logln(format!("has={}", unsafe { (vm.hasproperty)(j, -1, np) }));
                logln(format!("stack={:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(&cs, &rs, &format!("has {} {:?}", label, nm), f);

            let f = move |vm: &Vm, j: JsPtr| {
                p(vm, j);
                let np = nm.as_ptr() as *const c_char;
                unsafe { (vm.getproperty)(j, -1, np) };
                logln(format!("get={:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(&cs, &rs, &format!("get {} {:?}", label, nm), f);

            let f = move |vm: &Vm, j: JsPtr| {
                p(vm, j);
                let np = nm.as_ptr() as *const c_char;
                unsafe { (vm.pushnumber)(j, 1.0) };
                unsafe { (vm.setproperty)(j, -2, np) };
                logln(format!("set={:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(&cs, &rs, &format!("set {} {:?}", label, nm), f);

            let f = move |vm: &Vm, j: JsPtr| {
                p(vm, j);
                let np = nm.as_ptr() as *const c_char;
                unsafe { (vm.delproperty)(j, -1, np) };
                logln(format!("del={:?}", stack_snapshot(vm, j)));
            };
            assert_same_protected(&cs, &rs, &format!("del {} {:?}", label, nm), f);
        }
        let f = move |vm: &Vm, j: JsPtr| {
            push(vm, j);
            logln(format!("getlength={}", unsafe { (vm.getlength)(j, -1) }));
        };
        assert_same_protected(&cs, &rs, &format!("getlength {}", label), f);
    }
}

#[test]
fn index_api_matches() {
    let (cs, rs) = sessions();
    for i in [-2i32, -1, 0, 1, 2, 5, 100, 2147483647] {
        let f = move |vm: &Vm, j: JsPtr| {
            unsafe { (vm.newarray)(j) };
            logln(format!("has0={}", unsafe { (vm.hasindex)(j, -1, i) }));
            if unsafe { (vm.hasindex)(j, -1, i) } != 0 {
                unsafe { (vm.pop)(j, 1) };
            }
            unsafe { (vm.pushnumber)(j, i as f64) };
            unsafe { (vm.setindex)(j, -2, i) };
            logln(format!("len={}", unsafe { (vm.getlength)(j, -1) }));
            unsafe { (vm.getindex)(j, -1, i) };
            logln(format!("get={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pop)(j, 1) };
            logln(format!("has1={}", unsafe { (vm.hasindex)(j, -1, i) }));
            if unsafe { (vm.hasindex)(j, -1, i) } != 0 {
                unsafe { (vm.pop)(j, 1) };
            }
            unsafe { (vm.delindex)(j, -1, i) };
            logln(format!("has2={}", unsafe { (vm.hasindex)(j, -1, i) }));
            logln(format!("final={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("index {}", i), f);
    }

    for len in [-1i32, 0, 1, 3, 10] {
        let f = move |vm: &Vm, j: JsPtr| {
            unsafe { (vm.newarray)(j) };
            for k in 0..3 {
                unsafe { (vm.pushnumber)(j, k as f64) };
                unsafe { (vm.setindex)(j, -2, k) };
            }
            unsafe { (vm.setlength)(j, -1, len) };
            logln(format!("len={}", unsafe { (vm.getlength)(j, -1) }));
            logln(format!("stack={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("setlength {}", len), f);
    }
}

#[test]
fn global_and_registry_matches() {
    let (cs, rs) = sessions();
    for name in NAMES {
        let nm = *name;
        for atts in [0i32, 1, 2, 4] {
            let f = move |vm: &Vm, j: JsPtr| {
                let p = nm.as_ptr() as *const c_char;
                unsafe { (vm.getglobal)(j, p) };
                logln(format!("g0={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.pushnumber)(j, 5.0) };
                unsafe { (vm.setglobal)(j, p) };
                unsafe { (vm.getglobal)(j, p) };
                logln(format!("g1={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.pushnumber)(j, 6.0) };
                unsafe { (vm.defglobal)(j, p, atts) };
                unsafe { (vm.getglobal)(j, p) };
                logln(format!("g2={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.delglobal)(j, p) };
                unsafe { (vm.getglobal)(j, p) };
                logln(format!("g3={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };

                unsafe { (vm.getregistry)(j, p) };
                logln(format!("r0={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.pushnumber)(j, 9.0) };
                unsafe { (vm.setregistry)(j, p) };
                unsafe { (vm.getregistry)(j, p) };
                logln(format!("r1={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
                unsafe { (vm.delregistry)(j, p) };
                unsafe { (vm.getregistry)(j, p) };
                logln(format!("r2={:?}", stack_snapshot(vm, j)));
                unsafe { (vm.pop)(j, 1) };
            };
            assert_same_protected(&cs, &rs, &format!("global/registry {:?}", nm), f);
        }
    }
}

#[test]
fn ref_unref_matches() {
    let (cs, rs) = sessions();
    let f = |vm: &Vm, j: JsPtr| {
        // js_ref returns a generated key; the *shape* should match.
        for i in 0..5 {
            unsafe { (vm.pushnumber)(j, i as f64) };
            let r = unsafe { (vm.ref_)(j) };
            let s = unsafe { cstr_to_string(r) }.unwrap_or_default();
            logln(format!("ref{}={}", i, s));
            unsafe { (vm.getregistry)(j, r) };
            logln(format!("val={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pop)(j, 1) };
            unsafe { (vm.unref)(j, r) };
            unsafe { (vm.getregistry)(j, r) };
            logln(format!("after unref={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pop)(j, 1) };
        }
    };
    assert_same_protected(&cs, &rs, "ref/unref", f);
}

#[test]
fn iterator_api_matches() {
    let (cs, rs) = sessions();
    let setups: Vec<(&str, &str)> = vec![
        ("empty object", "({})"),
        ("simple object", "({a:1,b:2,c:3})"),
        ("array", "[10,20,30]"),
        ("sparse array", "(function(){var a=[1,2,3]; delete a[1]; return a})()"),
        ("string", "'abc'"),
        ("number", "1"),
        ("null", "null"),
        ("undefined", "undefined"),
        ("with proto", "Object.create({p:1},{q:{value:2,enumerable:true}})"),
        ("nonenum", "(function(){var o={}; Object.defineProperty(o,'h',{value:1}); o.v=2; return o})()"),
        ("array with props", "(function(){var a=[1]; a.z=2; return a})()"),
    ];
    for (label, expr) in setups {
        for own in [0i32, 1] {
            let src = format!("var __v = {};", expr);
            let name: &'static [u8] = b"__v\0";
            let f = move |vm: &Vm, j: JsPtr| {
                let p = name.as_ptr() as *const c_char;
                unsafe { (vm.getglobal)(j, p) };
                unsafe { (vm.pushiterator)(j, -1, own) };
                loop {
                    let k = unsafe { (vm.nextiterator)(j, -1) };
                    if k.is_null() {
                        break;
                    }
                    logln(unsafe { cstr_to_string(k) }.unwrap_or_default());
                }
                logln(format!("stack={:?}", stack_snapshot(vm, j)));
            };
            // seed the global on both sides
            let cs_r = run_script(&cs, &src);
            let rs_r = run_script(&rs, &src);
            assert_eq!(cs_r, rs_r, "iterator setup {}", label);
            assert_same_protected(&cs, &rs, &format!("iterator {} own={}", label, own), f);
        }
    }
}

#[test]
fn newcfunction_and_construct_matches() {
    // A cfunction and a cconstructor implemented in Rust, registered into both
    // interpreters, then called from JS.
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    register_adder(&cs);
    register_adder(&rs);
    for src in [
        "adder(1,2)",
        "adder()",
        "adder(1)",
        "adder('a','b')",
        "adder.length",
        "adder.name",
        "typeof adder",
        "adder.call(null,3,4)",
        "adder.apply(null,[5,6])",
        "try { new adder(1,2) } catch(e) { e.name }",
        "new Ctor(1).v",
        "Ctor(2)",
        "typeof Ctor.prototype",
        "(new Ctor(3)) instanceof Ctor",
        "Ctor.length",
        "String(adder)",
        "String(Ctor)",
        "adder.toString()",
    ] {
        let a = run_script(&cs, src);
        let b = run_script(&rs, src);
        assert_eq!(a, b, "cfunction script: {}", src);
    }
}

unsafe extern "C-unwind" fn adder_c(j: JsPtr) {
    let vm = current_vm(Side::C);
    do_add(&vm, j);
}
unsafe extern "C-unwind" fn adder_r(j: JsPtr) {
    let vm = current_vm(Side::Rust);
    do_add(&vm, j);
}
fn do_add(vm: &Vm, j: JsPtr) {
    let top = unsafe { (vm.gettop)(j) };
    let mut sum = 0.0;
    for i in 1..top {
        sum += unsafe { (vm.tonumber)(j, i) };
    }
    unsafe { (vm.pushnumber)(j, sum) };
}

unsafe extern "C-unwind" fn ctor_c(j: JsPtr) {
    let vm = current_vm(Side::C);
    do_ctor(&vm, j);
}
unsafe extern "C-unwind" fn ctor_r(j: JsPtr) {
    let vm = current_vm(Side::Rust);
    do_ctor(&vm, j);
}
fn do_ctor(vm: &Vm, j: JsPtr) {
    unsafe { (vm.newobject)(j) };
    unsafe { (vm.pushnumber)(j, (vm.tonumber)(j, 1)) };
    unsafe { (vm.setproperty)(j, -2, b"v\0".as_ptr() as *const c_char) };
}

fn register_adder(s: &Session) {
    let vm = &s.vm;
    let j = s.j;
    let (fun, con): (
        unsafe extern "C-unwind" fn(JsPtr),
        unsafe extern "C-unwind" fn(JsPtr),
    ) = match vm.side {
        Side::C => (adder_c, ctor_c),
        Side::Rust => (adder_r, ctor_r),
    };
    unsafe { (vm.newcfunction)(j, fun, b"adder\0".as_ptr() as *const c_char, 2) };
    unsafe { (vm.setglobal)(j, b"adder\0".as_ptr() as *const c_char) };

    type NewCConstructor = unsafe extern "C-unwind" fn(
        JsPtr,
        unsafe extern "C-unwind" fn(JsPtr),
        unsafe extern "C-unwind" fn(JsPtr),
        *const c_char,
        c_int,
    );
    let i = impls();
    let lib = match vm.side {
        Side::C => &i.c,
        Side::Rust => &i.rust,
    };
    let ncc: Symbol<NewCConstructor> =
        unsafe { lib.get(b"js_newcconstructor\0").unwrap() };
    // js_newcconstructor expects the prototype object to already be on the
    // stack; it consumes it and leaves the constructor.
    unsafe { (vm.newobject)(j) };
    unsafe { ncc(j, fun, con, b"Ctor\0".as_ptr() as *const c_char, 1) };
    unsafe { (vm.setglobal)(j, b"Ctor\0".as_ptr() as *const c_char) };
}

#[test]
fn userdata_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for s in [&cs, &rs] {
        let vm = &s.vm;
        let j = s.j;
        type NewUserdata = unsafe extern "C-unwind" fn(
            JsPtr,
            *const c_char,
            *mut c_void,
            Option<unsafe extern "C-unwind" fn(JsPtr, *mut c_void)>,
        );
        let i = impls();
        let lib = match vm.side {
            Side::C => &i.c,
            Side::Rust => &i.rust,
        };
        let nu: Symbol<NewUserdata> = unsafe { lib.get(b"js_newuserdata\0").unwrap() };
        // js_newuserdata(x) pops a prototype (or non-object) off the stack.
        unsafe { (vm.newobject)(j) };
        unsafe { nu(j, b"MyTag\0".as_ptr() as *const c_char, 0x1234 as *mut c_void, None) };
        unsafe { (vm.setglobal)(j, b"ud\0".as_ptr() as *const c_char) };
    }
    for src in [
        "typeof ud",
        "String(ud)",
        "ud instanceof Object",
        "Object.prototype.toString.call(ud)",
        "ud.x",
        "ud.x = 1; ud.x",
        "delete ud.x",
        "var s=''; for (var k in ud) s+=k; s",
        "JSON.stringify(ud)",
    ] {
        let a = run_script(&cs, src);
        let b = run_script(&rs, src);
        assert_eq!(a, b, "userdata script: {}", src);
    }
    // js_isuserdata / js_touserdata
    for tag in [&b"MyTag\0"[..], &b"Other\0"[..]] {
        let t = tag;
        let f = move |vm: &Vm, j: JsPtr| {
            let i = impls();
            let lib = match vm.side {
                Side::C => &i.c,
                Side::Rust => &i.rust,
            };
            type IsUd = unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char) -> c_int;
            type ToUd = unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char) -> *mut c_void;
            let isud: Symbol<IsUd> = unsafe { lib.get(b"js_isuserdata\0").unwrap() };
            let toud: Symbol<ToUd> = unsafe { lib.get(b"js_touserdata\0").unwrap() };
            unsafe { (vm.getglobal)(j, b"ud\0".as_ptr() as *const c_char) };
            logln(format!("is={}", unsafe {
                isud(j, -1, t.as_ptr() as *const c_char)
            }));
            logln(format!("to={:?}", unsafe {
                toud(j, -1, t.as_ptr() as *const c_char)
            }));
        };
        assert_same_protected(&cs, &rs, "userdata tag check", f);
    }
}

#[test]
fn defaccessor_matches() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    // Build getter/setter in JS, then use js_defaccessor from the C API.
    let setup = "var __get = function(){return this.__v===undefined?'unset':this.__v};\
                 var __set = function(v){this.__v = v*2};";
    assert_eq!(run_script(&cs, setup), run_script(&rs, setup));
    for atts in [0i32, 2, 4] {
        let f = move |vm: &Vm, j: JsPtr| {
            unsafe { (vm.newobject)(j) };
            unsafe { (vm.getglobal)(j, b"__get\0".as_ptr() as *const c_char) };
            unsafe { (vm.getglobal)(j, b"__set\0".as_ptr() as *const c_char) };
            let da = {
                let i = impls();
                let lib = match vm.side {
                    Side::C => &i.c,
                    Side::Rust => &i.rust,
                };
                type DefAcc = unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char, c_int);
                let s: Symbol<DefAcc> = unsafe { lib.get(b"js_defaccessor\0").unwrap() };
                *s
            };
            unsafe { da(j, -3, b"p\0".as_ptr() as *const c_char, atts) };
            unsafe { (vm.getproperty)(j, -1, b"p\0".as_ptr() as *const c_char) };
            logln(format!("get1={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pop)(j, 1) };
            unsafe { (vm.pushnumber)(j, 21.0) };
            unsafe { (vm.setproperty)(j, -2, b"p\0".as_ptr() as *const c_char) };
            unsafe { (vm.getproperty)(j, -1, b"p\0".as_ptr() as *const c_char) };
            logln(format!("get2={:?}", stack_snapshot(vm, j)));
            unsafe { (vm.pop)(j, 1) };
            unsafe { (vm.pushiterator)(j, -1, 1) };
            loop {
                let k = unsafe { (vm.nextiterator)(j, -1) };
                if k.is_null() {
                    break;
                }
                logln(format!("key={}", unsafe { cstr_to_string(k) }.unwrap_or_default()));
            }
        };
        assert_same_protected(&cs, &rs, &format!("defaccessor atts={}", atts), f);
    }
}

#[test]
fn dostring_and_reports_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    for src in [
        "1+1",
        "throw new Error('boom')",
        "throw 'str'",
        "throw {toString:function(){return 'obj'}}",
        "throw {toString:function(){throw 1}}",
        "nosuch()",
        "(",
        "var x = 1",
        "print",
        "null.x",
    ] {
        let mut rets = Vec::new();
        for s in [&cs, &rs] {
            s.clear_logs();
            let c = CString::new(src).unwrap();
            rets.push(unsafe { (s.vm.dostring)(s.j, c.as_ptr()) });
        }
        assert_eq!(rets[0], rets[1], "js_dostring return differs for {:?}", src);
        let (a, b) = (cs.reports(), rs.reports());
        let ta = cs.top();
        let tb = rs.top();
        assert_eq!(a, b, "js_dostring reports differ for {:?}", src);
        assert_eq!(ta, tb, "js_dostring left different stack for {:?}", src);
    }
}

#[test]
fn instanceof_and_repr_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let setup = "function A(){}; var a = new A(); var notctor = {}; var badproto = function(){}; badproto.prototype = 1;";
    assert_eq!(run_script(&cs, setup), run_script(&rs, setup));
    let pairs = [
        ("a", "A"),
        ("a", "Object"),
        ("{}", "A"),
        ("1", "A"),
        ("a", "notctor"),
        ("a", "badproto"),
        ("[]", "Array"),
        ("null", "Object"),
    ];
    for (l, r) in pairs {
        let lhs = format!("var __l = {};", l);
        let rhs = format!("var __r = {};", r);
        assert_eq!(run_script(&cs, &lhs), run_script(&rs, &lhs));
        assert_eq!(run_script(&cs, &rhs), run_script(&rs, &rhs));
        let f = |vm: &Vm, j: JsPtr| {
            unsafe { (vm.getglobal)(j, b"__l\0".as_ptr() as *const c_char) };
            unsafe { (vm.getglobal)(j, b"__r\0".as_ptr() as *const c_char) };
            logln(format!("instanceof={}", unsafe { (vm.instanceof)(j) }));
            logln(format!("stack={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("{} instanceof {}", l, r), f);
    }

    // js_repr / js_torepr on many kinds of values
    for expr in [
        "1", "'a'", "true", "null", "undefined", "{}", "[]", "[1,[2,{a:3}]]",
        "{a:1,b:'x',c:[1,2],d:{e:1}}", "function(){}", "/re/gi", "new Date(0)",
        "new Error('e')", "(function(){var o={}; o.self=o; return o})()",
        "Math", "JSON", "Object", "new Number(1)", "new String('s')",
        "{'weird key':1}", "{0:1}", "[undefined,null]",
    ] {
        let src = format!("var __v = ({});", expr);
        assert_eq!(run_script(&cs, &src), run_script(&rs, &src), "repr setup {}", expr);
        let f = |vm: &Vm, j: JsPtr| {
            unsafe { (vm.getglobal)(j, b"__v\0".as_ptr() as *const c_char) };
            unsafe { (vm.repr)(j, -1) };
            logln(format!("repr={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("repr {}", expr), f);
    }
}

#[test]
fn try_conversions_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let exprs = [
        "1",
        "'2'",
        "'abc'",
        "null",
        "undefined",
        "{}",
        "[]",
        "{valueOf:function(){throw new Error('nope')}}",
        "{toString:function(){throw new Error('nope')}}",
        "{valueOf:function(){return 5}}",
        "{toString:function(){return '6'}}",
        "1/0",
        "NaN",
    ];
    for e in exprs {
        let src = format!("var __v = ({});", e);
        assert_eq!(run_script(&cs, &src), run_script(&rs, &src), "setup {}", e);
        let f = |vm: &Vm, j: JsPtr| {
            unsafe { (vm.getglobal)(j, b"__v\0".as_ptr() as *const c_char) };
            let err = b"ERR\0".as_ptr() as *const c_char;
            logln(format!("trystring={:?}", unsafe {
                cstr_to_string((vm.trystring)(j, -1, err))
            }));
            logln(format!("trynumber={}", unsafe {
                (vm.trynumber)(j, -1, -12.5)
            }));
            logln(format!("tryinteger={}", unsafe { (vm.tryinteger)(j, -1, -7) }));
            logln(format!("tryboolean={}", unsafe { (vm.tryboolean)(j, -1, -7) }));
            logln(format!("tryrepr={:?}", unsafe {
                cstr_to_string((vm.tryrepr)(j, -1, err))
            }));
            logln(format!("stack={:?}", stack_snapshot(vm, j)));
        };
        assert_same_protected(&cs, &rs, &format!("tryX {}", e), f);
    }
}

#[test]
fn setlimit_matches() {
    for (runlimit, memlimit) in [(0i32, 0i32), (10, 0), (0, 1024), (100, 1 << 20)] {
        let cs = Session::new(Side::C, 0);
        let rs = Session::new(Side::Rust, 0);
        unsafe { (cs.vm.setlimit)(cs.j, runlimit, memlimit) };
        unsafe { (rs.vm.setlimit)(rs.j, runlimit, memlimit) };
        for src in [
            "1+1",
            "function f(n){return n==0?0:f(n-1)+1} f(50)",
            "var a=[]; for(var i=0;i<200;i++) a.push({i:i}); a.length",
            "var s='x'; for(var i=0;i<10;i++) s+=s; s.length",
        ] {
            let a = run_script(&cs, src);
            let b = run_script(&rs, src);
            assert_eq!(
                a, b,
                "setlimit({},{}) script {:?}",
                runlimit, memlimit, src
            );
        }
    }
}

#[test]
fn runeat_and_utfptrtoidx_match() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let strings: [&[u8]; 8] = [
        b"\0",
        b"a\0",
        b"abc\0",
        b"\xc3\xa9abc\0",
        b"\xe4\xb8\xad\xe6\x96\x87\0",
        b"\xf0\x9f\x98\x80x\0",
        b"\xff\xfe\0",
        b"a\xc3\xa9b\xe4\xb8\xadc\0",
    ];
    for st in strings {
        let s: &'static [u8] = st;
        let f = move |vm: &Vm, j: JsPtr| {
            let i = impls();
            let lib = match vm.side {
                Side::C => &i.c,
                Side::Rust => &i.rust,
            };
            type Runeat = unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int) -> c_int;
            type Idx = unsafe extern "C-unwind" fn(*const c_char, *const c_char) -> c_int;
            let ra: Symbol<Runeat> = unsafe { lib.get(b"js_runeat\0").unwrap() };
            let pi: Symbol<Idx> = unsafe { lib.get(b"js_utfptrtoidx\0").unwrap() };
            let p = s.as_ptr() as *const c_char;
            for i in -2..8i32 {
                logln(format!("runeat({})={}", i, unsafe { ra(j, p, i) }));
            }
            for off in 0..s.len() {
                logln(format!("idx({})={}", off, unsafe {
                    pi(p, p.add(off))
                }));
            }
        };
        assert_same_protected(&cs, &rs, &format!("runeat {:?}", st), f);
    }
}

#[test]
fn dumpstrings_does_not_crash_and_matches_state() {
    // jsS_dumpstrings writes to stdout; just make sure both survive it and the
    // state stays usable afterwards.
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let src = "var o={}; for(var i=0;i<50;i++) o['key'+i]=i; Object.keys(o).length";
    assert_eq!(run_script(&cs, src), run_script(&rs, src));
    for s in [&cs, &rs] {
        let i = impls();
        let lib = match s.vm.side {
            Side::C => &i.c,
            Side::Rust => &i.rust,
        };
        type Dump = unsafe extern "C-unwind" fn(JsPtr);
        let d: Symbol<Dump> = unsafe { lib.get(b"jsS_dumpstrings\0").unwrap() };
        unsafe { d(s.j) };
    }
    assert_eq!(
        run_script(&cs, "Object.keys(o).length"),
        run_script(&rs, "Object.keys(o).length")
    );
}
