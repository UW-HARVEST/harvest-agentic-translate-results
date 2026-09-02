//! Phase B rows 54-58, 76: property definition/attributes, accessors,
//! indexed access, iterators and the array internals.
mod common;
use common::*;
use std::os::raw::{c_int, c_void};

const JS_READONLY: c_int = 1;
const JS_DONTENUM: c_int = 2;
const JS_DONTCONF: c_int = 4;

/// Dump every observable aspect of an object at `idx`.
unsafe fn dump_object(api: &Api, j: JS, idx: c_int, names: &[&str]) {
    unsafe {
        log(format!("length={}", (api.js_getlength)(j, idx)));
        for n in names {
            let cn = cs(n);
            log(format!("has[{n}]={}", (api.js_hasproperty)(j, idx, cn.as_ptr())));
            // js_hasproperty pushes the value when it exists
            if (api.js_hasproperty)(j, idx, cn.as_ptr()) != 0 {
                log(format!("  val={}", describe(api, j, -1)));
                (api.js_pop)(j, 1);
            }
            (api.js_getproperty)(j, idx, cn.as_ptr());
            log(format!("get[{n}]={}", describe(api, j, -1)));
            (api.js_pop)(j, 1);
        }
        for i in 0..6 {
            log(format!("hasindex[{i}]={}", (api.js_hasindex)(j, idx, i)));
            if (api.js_hasindex)(j, idx, i) != 0 {
                (api.js_pop)(j, 1);
            }
            (api.js_getindex)(j, idx, i);
            log(format!("getindex[{i}]={}", describe(api, j, -1)));
            (api.js_pop)(j, 1);
        }
        for own in [0, 1] {
            (api.js_pushiterator)(j, idx, own);
            let mut keys = Vec::new();
            loop {
                let k = (api.js_nextiterator)(j, -1);
                if k.is_null() {
                    break;
                }
                keys.push(rstr(k));
                (api.js_pop)(j, 1); // nextiterator pushes the value
                if keys.len() > 200 {
                    break;
                }
            }
            (api.js_pop)(j, 1);
            log(format!("iter(own={own})={keys:?}"));
        }
    }
}

const PROBE_NAMES: &[&str] = &[
    "a", "b", "c", "missing", "0", "1", "2", "length", "toString", "valueOf", "constructor", "",
    "__proto__", "4294967295",
];

#[test]
fn row54_defproperty_attribute_combinations() {
    for atts in 0..8 {
        diff_protected(&format!("js_defproperty atts={atts}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newobject)(j);
                for (n, v) in [("a", 1.0), ("b", 2.0)] {
                    let cn = cs(n);
                    (api.js_pushnumber)(j, v);
                    (api.js_defproperty)(j, -2, cn.as_ptr(), atts);
                }
                dump_object(api, j, -1, PROBE_NAMES);
                // now try to overwrite / delete and observe
                let ca = cs("a");
                (api.js_pushnumber)(j, 99.0);
                (api.js_setproperty)(j, -2, ca.as_ptr());
                (api.js_getproperty)(j, -1, ca.as_ptr());
                log(format!("after-set a={}", describe(api, j, -1)));
                (api.js_pop)(j, 1);
                (api.js_delproperty)(j, -1, ca.as_ptr());
                log(format!("after-del has-a={}", {
                    let h = (api.js_hasproperty)(j, -1, ca.as_ptr());
                    if h != 0 {
                        (api.js_pop)(j, 1);
                    }
                    h
                }));
                dump_object(api, j, -1, PROBE_NAMES);
            }
        });
    }
}

#[test]
fn row54b_defglobal_attribute_combinations() {
    for atts in 0..8 {
        for strict in [0, JS_STRICT] {
            diff_protected(
                &format!("js_defglobal atts={atts} strict={strict}"),
                strict,
                || {
                    move |api: &Api, j: JS| unsafe {
                        let g = cs("gv");
                        (api.js_pushnumber)(j, 5.0);
                        (api.js_defglobal)(j, g.as_ptr(), atts);
                        (api.js_getglobal)(j, g.as_ptr());
                        log(format!("get={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        (api.js_pushnumber)(j, 6.0);
                        (api.js_setglobal)(j, g.as_ptr());
                        (api.js_getglobal)(j, g.as_ptr());
                        log(format!("after-set={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        (api.js_pushglobal)(j);
                        dump_object(api, j, -1, &["gv", "undefined", "NaN", "Infinity"]);
                        (api.js_pop)(j, 1);
                        (api.js_delglobal)(j, g.as_ptr());
                        (api.js_getglobal)(j, g.as_ptr());
                        log(format!("after-del={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        // observable from script too
                        let fname = cs("[string]");
                        let src = cs("typeof gv");
                        if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) == 0 {
                            (api.js_pushundefined)(j);
                            (api.js_pcall)(j, 0);
                            log(format!("script={}", describe(api, j, -1)));
                            (api.js_pop)(j, 1);
                        }
                    }
                },
            );
        }
    }
}

#[test]
fn row55_defaccessor() {
    // getter only / setter only / both / neither, across attribute combos
    for kind in 0..4 {
        for atts in 0..8 {
            diff_protected(
                &format!("js_defaccessor kind={kind} atts={atts}"),
                0,
                || {
                    move |api: &Api, j: JS| unsafe {
                        (api.js_newobject)(j);
                        // build getter and setter with scripts
                        let fname = cs("[string]");
                        let gsrc = cs("(function(){ return 'G' })");
                        let ssrc = cs("(function(v){ this.__v = v })");
                        let mut push = |src: &std::ffi::CString| {
                            if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) == 0 {
                                (api.js_pushundefined)(j);
                                (api.js_pcall)(j, 0);
                            }
                        };
                        match kind {
                            0 => {
                                push(&gsrc);
                                (api.js_pushundefined)(j);
                            }
                            1 => {
                                (api.js_pushundefined)(j);
                                push(&ssrc);
                            }
                            2 => {
                                push(&gsrc);
                                push(&ssrc);
                            }
                            _ => {
                                (api.js_pushundefined)(j);
                                (api.js_pushundefined)(j);
                            }
                        }
                        let cn = cs("p");
                        (api.js_defaccessor)(j, -3, cn.as_ptr(), atts);
                        dump_object(api, j, -1, &["p", "__v", "a"]);
                        (api.js_getproperty)(j, -1, cn.as_ptr());
                        log(format!("read p={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        let vv = cs("V");
                        (api.js_pushstring)(j, vv.as_ptr());
                        (api.js_setproperty)(j, -2, cn.as_ptr());
                        dump_object(api, j, -1, &["p", "__v"]);
                    }
                },
            );
        }
    }
}

#[test]
fn row56_property_access_own_inherited_absent() {
    let setups = [
        "({})",
        "({a:1})",
        "({a:1,b:2,c:3})",
        "Object.create({a:1})",
        "Object.create(null)",
        "[]",
        "[1,2,3]",
        "[1,,3]",
        "'str'",
        "new String('str')",
        "(function(){})",
        "/re/g",
        "new Date(0)",
        "new Number(1)",
        "Math",
        "JSON",
        "Object.defineProperty({}, 'x', {value:1, enumerable:false})",
        "Object.defineProperty({}, 'x', {get:function(){return 2}})",
        "Object.freeze({a:1})",
        "Object.seal({a:1})",
        "Object.preventExtensions({a:1})",
    ];
    for setup in setups {
        diff_protected(&format!("property access on {setup}"), 0, || {
            let setup = setup.to_string();
            move |api: &Api, j: JS| unsafe {
                let fname = cs("[string]");
                let src = cs(&format!("({setup})"));
                if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                    log("compile-failed");
                    return;
                }
                (api.js_pushundefined)(j);
                if (api.js_pcall)(j, 0) != 0 {
                    log(format!("threw {}", describe(api, j, -1)));
                    return;
                }
                dump_object(api, j, -1, PROBE_NAMES);
                // mutate: set / def / del
                for n in ["a", "z", "0", "5", "length"] {
                    let cn = cs(n);
                    (api.js_pushnumber)(j, 7.0);
                    (api.js_setproperty)(j, -2, cn.as_ptr());
                    (api.js_getproperty)(j, -1, cn.as_ptr());
                    log(format!("set-then-get[{n}]={}", describe(api, j, -1)));
                    (api.js_pop)(j, 1);
                    (api.js_delproperty)(j, -1, cn.as_ptr());
                    (api.js_getproperty)(j, -1, cn.as_ptr());
                    log(format!("after-del[{n}]={}", describe(api, j, -1)));
                    (api.js_pop)(j, 1);
                }
                dump_object(api, j, -1, PROBE_NAMES);
            }
        });
    }
}

#[test]
fn row57_length_and_index_operations() {
    let setups = ["[]", "[1]", "[1,2,3]", "[1,,3]", "({})", "({length:2})", "'abc'", "new String('abc')"];
    for setup in setups {
        for len in [0 as c_int, 1, 3, 5, 100] {
            diff_protected(&format!("length ops on {setup} len={len}"), 0, || {
                let setup = setup.to_string();
                move |api: &Api, j: JS| unsafe {
                    let fname = cs("[string]");
                    let src = cs(&format!("({setup})"));
                    if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                        log("compile-failed");
                        return;
                    }
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        log(format!("threw {}", describe(api, j, -1)));
                        return;
                    }
                    (api.js_setlength)(j, -1, len);
                    log(format!("len={}", (api.js_getlength)(j, -1)));
                    for i in [-1 as c_int, 0, 1, 2, 4, 99, 1000] {
                        log(format!("hasindex[{i}]={}", {
                            let h = (api.js_hasindex)(j, -1, i);
                            if h != 0 {
                                (api.js_pop)(j, 1);
                            }
                            h
                        }));
                        (api.js_getindex)(j, -1, i);
                        log(format!("getindex[{i}]={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        (api.js_pushnumber)(j, (i * 100) as f64);
                        (api.js_setindex)(j, -2, i);
                        (api.js_getindex)(j, -1, i);
                        log(format!("after-setindex[{i}]={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        (api.js_delindex)(j, -1, i);
                        (api.js_getindex)(j, -1, i);
                        log(format!("after-delindex[{i}]={}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                    }
                    log(format!("final-len={}", (api.js_getlength)(j, -1)));
                    let errs = cs("<throw>");
                    log(format!(
                        "final={}",
                        rstr((api.js_tryrepr)(j, -1, errs.as_ptr()))
                    ));
                }
            });
        }
    }
}

#[test]
fn row58_iterators() {
    let setups = [
        "({})",
        "({a:1,b:2})",
        "Object.create({inherited:1})",
        "(function(){ var o = Object.create({p:1}); o.own = 2; return o })()",
        "Object.defineProperty({a:1}, 'hidden', {value:2, enumerable:false})",
        "[]",
        "[1,2,3]",
        "[1,,3]",
        "(function(){ var a=[1,2,3]; a.extra=4; return a })()",
        "'abc'",
        "new String('abc')",
        "1",
        "true",
        "(function f(){})",
        "/re/",
        "new Date(0)",
        "Math",
        "(function(){ var a=[]; a[5]=1; return a })()",
        "(function(){ var o={}; for(var i=0;i<40;++i) o['k'+i]=i; return o })()",
    ];
    for setup in setups {
        for own in [0 as c_int, 1, 2, -1] {
            diff_protected(&format!("iterator own={own} on {setup}"), 0, || {
                let setup = setup.to_string();
                move |api: &Api, j: JS| unsafe {
                    let fname = cs("[string]");
                    let src = cs(&format!("({setup})"));
                    if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                        log("compile-failed");
                        return;
                    }
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        log(format!("threw {}", describe(api, j, -1)));
                        return;
                    }
                    (api.js_pushiterator)(j, -1, own);
                    log(format!("iterobj={}", describe(api, j, -1)));
                    let mut n = 0;
                    loop {
                        let k = (api.js_nextiterator)(j, -1);
                        if k.is_null() {
                            break;
                        }
                        log(format!("key={:?} val={}", rstr(k), describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                        n += 1;
                        if n > 300 {
                            break;
                        }
                    }
                    log(format!("count={n}"));
                    // iterator exhausted: repeated calls must stay NULL
                    for _ in 0..3 {
                        log(format!(
                            "exhausted={}",
                            (api.js_nextiterator)(j, -1).is_null()
                        ));
                    }
                }
            });
        }
    }
    // for-in through the interpreter as well
    for setup in setups {
        diff_eval_both_modes(&format!(
            "var s=[]; for (var k in ({setup})) s.push(k); s.join('|')"
        ));
    }
}

#[test]
fn row76_array_internals() {
    // grow / shrink / sparse transitions of the flat array representation
    let scripts = [
        "var a=[]; a.length=10; a.join(',')",
        "var a=[1,2,3]; a.length=1; a.join(',')",
        "var a=[1,2,3]; a.length=0; a.length",
        "var a=[1,2,3]; a[10]=1; a.length+':'+a.join(',')",
        "var a=[1,2,3]; delete a[1]; a.join(',')+':'+(1 in a)",
        "var a=[]; for(var i=0;i<1000;++i)a[i]=i; a.length+':'+a[999]",
        "var a=[]; for(var i=999;i>=0;--i)a[i]=i; a.length+':'+a[0]",
        "var a=[1,2,3]; a.foo=1; var s=[]; for(var k in a)s.push(k); s.join('|')",
        "var a=[1,2,3]; Object.defineProperty(a,'1',{value:9}); a.join(',')",
        "var a=[1,2,3]; a.unshift(0); a.join(',')",
        "var a=[1,2,3]; a.splice(1,1); a.join(',')",
        "var a=[1,2,3]; a.splice(1,0,9,8); a.join(',')",
        "var a=[3,1,2]; a.sort(); a.join(',')",
        "var a=[3,1,2]; a.sort(function(x,y){return y-x}); a.join(',')",
        "var a=[1,2,3]; a.reverse(); a.join(',')",
        "var a=new Array(5); a.length+':'+a.join(',')",
        "var a=new Array(1,2,3); a.join(',')",
        "new Array(-1)",
        "new Array(4294967296)",
        "new Array(1.5)",
        "var a=[1,2,3]; a.length=-1; a.length",
        "var a=[1,2,3]; a.length=4294967296; a.length",
        "var a=[1,2,3]; a.concat([4,5]).join(',')",
        "var a=[1,2,3]; a.slice(1,-1).join(',')",
        "var a=[[1,2],[3]]; a.join(',')",
        "var a=[1,2]; a[1.5]=9; var s=[]; for(var k in a)s.push(k); s.join('|')",
        "var a=[1,2]; a['01']=9; var s=[]; for(var k in a)s.push(k); s.join('|')",
        "var a=[]; a[4294967294]=1; a.length",
        "var a=[]; a[4294967295]=1; a.length+':'+a[4294967295]",
        "var a=[1,2,3]; a.pop()+','+a.shift()+','+a.join('|')",
        "var a=[1,2,3]; a.indexOf(2)+','+a.lastIndexOf(3)",
        "Array.prototype.join.call({length:2,0:'a',1:'b'},'-')",
        "Array.prototype.slice.call({length:2,0:'a',1:'b'}).join('-')",
    ];
    for s in scripts {
        diff_eval_both_modes(s);
    }
    // low-level: build sparse arrays through the C API then read them back
    diff_protected("array via C API", 0, || {
        move |api: &Api, j: JS| unsafe {
            (api.js_newarray)(j);
            for i in [0 as c_int, 1, 2, 10, 5, 3] {
                (api.js_pushnumber)(j, i as f64);
                (api.js_setindex)(j, -2, i);
                log(format!("len={}", (api.js_getlength)(j, -1)));
            }
            dump_object(api, j, -1, PROBE_NAMES);
            (api.js_setlength)(j, -1, 2);
            dump_object(api, j, -1, PROBE_NAMES);
            (api.js_setlength)(j, -1, 20);
            dump_object(api, j, -1, PROBE_NAMES);
            for i in [0 as c_int, 1, 10] {
                (api.js_delindex)(j, -1, i);
            }
            dump_object(api, j, -1, PROBE_NAMES);
        }
    });
}

/* ---- userdata (row 62) ---- */

static mut FINALIZED_C: i32 = 0;
static mut FINALIZED_R: i32 = 0;

unsafe extern "C-unwind" fn ud_fin_c(_j: JS, _p: *mut c_void) {
    unsafe { FINALIZED_C += 1 }
}
unsafe extern "C-unwind" fn ud_fin_r(_j: JS, _p: *mut c_void) {
    unsafe { FINALIZED_R += 1 }
}
unsafe extern "C-unwind" fn ud_has(_j: JS, _p: *mut c_void, name: *const std::os::raw::c_char) -> c_int {
    let n = unsafe { rstr(name) };
    log(format!("ud_has({n})"));
    0
}
unsafe extern "C-unwind" fn ud_put(_j: JS, _p: *mut c_void, name: *const std::os::raw::c_char) -> c_int {
    let n = unsafe { rstr(name) };
    log(format!("ud_put({n})"));
    0
}
unsafe extern "C-unwind" fn ud_del(_j: JS, _p: *mut c_void, name: *const std::os::raw::c_char) -> c_int {
    let n = unsafe { rstr(name) };
    log(format!("ud_del({n})"));
    0
}

#[test]
fn row62_userdata() {
    let p = pair();
    for variant in 0..2 {
        let mut outs = Vec::new();
        for (which, api) in [(0usize, &p.c), (1usize, &p.r)] {
            let fin: unsafe extern "C-unwind" fn(JS, *mut c_void) =
                if which == 0 { ud_fin_c } else { ud_fin_r };
            let out = run_protected(api, which, 0, move |api: &Api, j: JS| unsafe {
                // js_newuserdata* needs a prototype object on the stack
                (api.js_newobject)(j);
                if variant == 0 {
                    (api.js_newuserdata)(j, c"mytag".as_ptr(), 0x1234 as *mut c_void, Some(fin));
                } else {
                    (api.js_newuserdatax)(
                        j,
                        c"mytag".as_ptr(),
                        0x1234 as *mut c_void,
                        Some(ud_has),
                        Some(ud_put),
                        Some(ud_del),
                        Some(fin),
                    );
                }
                log(describe(api, j, -1));
                log(format!(
                    "isuserdata(mytag)={}",
                    (api.js_isuserdata)(j, -1, c"mytag".as_ptr())
                ));
                log(format!(
                    "isuserdata(other)={}",
                    (api.js_isuserdata)(j, -1, c"other".as_ptr())
                ));
                log(format!(
                    "touserdata(mytag)={}",
                    (api.js_touserdata)(j, -1, c"mytag".as_ptr()) as usize
                ));
                dump_object(api, j, -1, &["a", "b", "toString"]);
                (api.js_pushnumber)(j, 1.0);
                (api.js_setproperty)(j, -2, c"a".as_ptr());
                (api.js_delproperty)(j, -1, c"a".as_ptr());
                (api.js_setglobal)(j, c"ud".as_ptr());
                let fname = cs("[string]");
                for src in ["typeof ud", "String(ud)", "ud.a", "'a' in ud", "delete ud.a"] {
                    let csrc = cs(src);
                    if (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr()) == 0 {
                        (api.js_pushundefined)(j);
                        (api.js_pcall)(j, 0);
                        log(format!("{src} -> {}", describe(api, j, -1)));
                        (api.js_pop)(j, 1);
                    }
                }
            });
            outs.push(out);
        }
        assert_eq!(outs[0], outs[1], "userdata variant {variant}");
    }
    // touserdata with a mismatched tag throws the same TypeError
    diff_eval_both_modes("1");
    assert!(unsafe { FINALIZED_C } > 0, "C finalizer must run");
    assert!(unsafe { FINALIZED_R } > 0, "Rust finalizer must run");
}

#[test]
fn row59_regexp_flag_combinations() {
    // JS_REGEXP_G=1, _I=2, _M=4 plus out-of-range bits
    for flags in [0 as c_int, 1, 2, 3, 4, 5, 6, 7, 8, 15, -1] {
        for pat in [
            "a", "A", "^a$", "a.b", "(a)(b)", "\\d+", "[a-z]+", "", "a|b", "x(?=y)",
        ] {
            diff_protected(
                &format!("js_newregexp({pat:?}, {flags})"),
                0,
                || {
                    let pat = pat.to_string();
                    move |api: &Api, j: JS| unsafe {
                        let cp = cs(&pat);
                        (api.js_newregexp)(j, cp.as_ptr(), flags);
                        log(describe(api, j, -1));
                        dump_object(
                            api,
                            j,
                            -1,
                            &["source", "global", "ignoreCase", "multiline", "lastIndex", "exec"],
                        );
                        (api.js_setglobal)(j, c"re".as_ptr());
                        let fname = cs("[string]");
                        for src in [
                            "re.source",
                            "String(re)",
                            "re.global+','+re.ignoreCase+','+re.multiline",
                            "JSON.stringify(re.exec('aAbB\\nxy'))",
                            "re.lastIndex",
                            "JSON.stringify(re.exec('aAbB\\nxy'))",
                            "re.lastIndex",
                            "re.test('aAbB')",
                            "'aAbB\\nxy'.replace(re,'#')",
                            "JSON.stringify('aAbB\\nxy'.match(re))",
                            "JSON.stringify('a,b,c'.split(re))",
                            "'aAbB'.search(re)",
                        ] {
                            let csrc = cs(src);
                            if (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr()) == 0 {
                                (api.js_pushundefined)(j);
                                let rc = (api.js_pcall)(j, 0);
                                let errs = cs("<throw>");
                                log(format!(
                                    "{src} -> rc={rc} {}",
                                    rstr((api.js_trystring)(j, -1, errs.as_ptr()))
                                ));
                                (api.js_pop)(j, 1);
                            }
                        }
                    }
                },
            );
        }
    }
}

#[test]
fn row60_regexp_prototype_exec_export() {
    // js_RegExp_prototype_exec is exported and used as RegExp.prototype.exec.
    let scripts = [
        "var r=/a/g; JSON.stringify([r.exec('aaa'),r.lastIndex,r.exec('aaa'),r.lastIndex,r.exec('aaa'),r.lastIndex])",
        "var r=/a/; JSON.stringify([r.exec('aaa'),r.lastIndex])",
        "var r=/(a)(b)?/; JSON.stringify(r.exec('ab'))",
        "var r=/(a)(b)?/; JSON.stringify(r.exec('a'))",
        "var r=/x/g; r.lastIndex=100; JSON.stringify([r.exec('xxx'),r.lastIndex])",
        "var r=/x/g; r.lastIndex=-5; JSON.stringify([r.exec('xxx'),r.lastIndex])",
        "var r=/x/g; r.lastIndex='2'; JSON.stringify([r.exec('xxx'),r.lastIndex])",
        "RegExp.prototype.exec.call(/a/,'a')[0]",
        "try { RegExp.prototype.exec.call({},'a') } catch(e) { 'E:'+e }",
        "try { RegExp.prototype.exec.call(null,'a') } catch(e) { 'E:'+e }",
        "var r=/(?:)/g; JSON.stringify([r.exec(''),r.lastIndex])",
        "var r=/(?:)/g; var o=[]; for(var i=0;i<4;++i){o.push(r.exec('ab'),r.lastIndex)} JSON.stringify(o)",
        "JSON.stringify(/(a)|(b)/.exec('b'))",
        "JSON.stringify(/\\u00e9/.exec('h\\u00e9'))",
        "JSON.stringify(/./g.exec('\\u00e9x'))",
    ];
    for s in scripts {
        diff_eval_both_modes(s);
    }
}
