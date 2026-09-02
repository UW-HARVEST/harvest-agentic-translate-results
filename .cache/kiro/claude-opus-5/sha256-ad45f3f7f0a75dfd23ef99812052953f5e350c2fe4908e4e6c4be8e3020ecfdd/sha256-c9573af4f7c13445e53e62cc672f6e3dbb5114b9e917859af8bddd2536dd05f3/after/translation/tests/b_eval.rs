//! Phase B rows 41-46: the compile/eval/call entry points.
mod common;
use common::*;
use std::os::raw::c_int;

fn valid_scripts() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "1", "1+2*3", "'a'+1", "[1,2,3].length", "({a:1}).a",
        "var x=1; x", "var x; x", "function f(){return 1} f()",
        "(function(){ return 1 })()", "(function(){ })()",
        "if (1) 2; else 3;", "if (0) 2; else 3;",
        "var s=0; for (var i=0;i<10;++i) s+=i; s",
        "var s=0, i=0; while (i<10) { s+=i; ++i } s",
        "var s=0, i=0; do { s+=i; ++i } while (i<10); s",
        "var s=''; for (var k in {a:1,b:2}) s+=k; s",
        "switch(2){case 1:'one';break;case 2:'two';break;default:'d'}",
        "switch(9){case 1:'one';break;default:'d'}",
        "switch(1){case 1:case 2:'fall'}",
        "try { throw 1 } catch(e) { 'c'+e } finally { }",
        "try { 1 } finally { 2 }",
        "(function(){ try { return 'a' } finally { } })()",
        "(function(){ try { return 'a' } finally { return 'b' } })()",
        "var a=[1,2,3]; a.map(function(x){return x*2}).join(',')",
        "typeof void 0", "typeof null", "typeof 1", "typeof 'a'", "typeof {}", "typeof []",
        "typeof function(){}", "typeof undefinedVariable",
        "delete ({a:1}).a", "'a' in {a:1}", "1 in [1,2]",
        "(1,2,3)", "1?2:3", "0?2:3",
        "-1", "+'1'", "!0", "~0", "1&&2", "0||3", "null??1",
        "1<2", "'a'<'b'", "[]<[]", "({})<({})",
        "1==='1'", "1=='1'", "NaN!==NaN",
        "new Object()", "new Array(3).length", "new Date(0).getTime()",
        "String(1)+Number('2')+Boolean(0)",
        "[1,2,3].reduce(function(a,b){return a+b},0)",
        "Object.keys({a:1,b:2}).join(',')",
        "JSON.stringify({a:[1,'2',null,true]})",
        "'abc'.toUpperCase()", "'ABC'.toLowerCase()",
        "'a,b,c'.split(',').join('|')",
        "'abc'.charCodeAt(1)", "String.fromCharCode(97,98)",
        "Math.max(1,2,3)", "Math.min()", "Math.abs(-1)",
        "(123.456).toFixed(2)", "(255).toString(16)",
        "parseInt('0x10')", "parseFloat('1.5e2')",
        "encodeURIComponent('a b/ä')", "decodeURIComponent('a%20b')",
        "escape('a b')", "unescape('a%20b')",
        "isNaN('x')", "isFinite('1')",
        "eval('1+1')",
        "(function(){ 'use strict'; return 1 })()",
        "var o={get a(){return 1}, set a(v){this.b=v}}; o.a=5; o.a+','+o.b",
        "var o={}; Object.defineProperty(o,'x',{value:1}); o.x",
        "[].concat(1,[2,[3]]).length",
        "(function(a,b,c){return arguments.length})(1,2)",
        "(function(){return this})() === undefined",
        "'use strict'; (function(){return this})() === undefined",
        "[1,2,3].sort(function(a,b){return b-a}).join(',')",
        "var f = function g(){ return typeof g }; f()",
        "(function(){ var a; return a===undefined })()",
        "with ({x:1}) x",
        "label: { break label; } 'ok'",
        "var a = 1\nvar b = 2\na+b",
        "return",
        "(function(){ return })()",
        ";;;",
        "/* only a comment */",
        "// only a line comment",
        "",
        "  ",
        "\n\n",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // deterministic random expression trees
    let mut rng = Rng::new(0x4141);
    fn expr(rng: &mut Rng, depth: u32) -> String {
        let atoms = [
            "0", "1", "2", "-1", "0.5", "NaN", "Infinity", "'a'", "''", "'0'", "true", "false",
            "null", "undefined", "[]", "[1]", "({})", "({a:1})", "(function(){return 1})",
        ];
        if depth == 0 {
            return atoms[rng.below(atoms.len() as u32) as usize].to_string();
        }
        let ops = [
            "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "===", "!==", "&&", "||",
            "&", "|", "^", "<<", ">>", ">>>", ",",
        ];
        let l = expr(rng, depth - 1);
        let r = expr(rng, depth - 1);
        match rng.below(8) {
            0 => format!("(typeof {l})"),
            1 => format!("(!{l})"),
            2 => format!("(-{l})"),
            3 => format!("(~{l})"),
            4 => format!("({l} ? {r} : {l})"),
            5 => format!("(String({l})+String({r}))"),
            _ => format!(
                "({l} {} {r})",
                ops[rng.below(ops.len() as u32) as usize]
            ),
        }
    }
    for _ in 0..3000 {
        v.push(expr(&mut rng, 3));
    }
    v
}

#[test]
fn row41_dostring_nonstrict() {
    for s in valid_scripts() {
        diff_eval(&s, 0);
    }
}

#[test]
fn row42_dostring_strict() {
    for s in valid_scripts() {
        diff_eval(&s, JS_STRICT);
    }
}

#[test]
fn row43_ploadstring_pcall() {
    // js_dostring goes through the report hook; compare that path too.
    let p = pair();
    for s in valid_scripts() {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let _ = take_reports();
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                (api.js_setreport)(j, Some(report_cb));
                let csrc = cs(&s);
                let rc = (api.js_dostring)(j, csrc.as_ptr());
                let top = (api.js_gettop)(j);
                (api.js_freestate)(j);
                outs.push((rc, top, take_reports()));
            }
        }
        assert_eq!(outs[0], outs[1], "js_dostring({s:?})");
    }
}

#[test]
fn row44_ploadstring_pcall_strict() {
    let p = pair();
    for s in valid_scripts() {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let _ = take_reports();
                let j = (api.js_newstate)(None, std::ptr::null_mut(), JS_STRICT);
                (api.js_setreport)(j, Some(report_cb));
                let csrc = cs(&s);
                let rc = (api.js_dostring)(j, csrc.as_ptr());
                let top = (api.js_gettop)(j);
                (api.js_freestate)(j);
                outs.push((rc, top, take_reports()));
            }
        }
        assert_eq!(outs[0], outs[1], "strict js_dostring({s:?})");
    }
}

#[test]
fn row45_pconstruct() {
    let p = pair();
    let ctors = [
        ("Object", vec![]),
        ("Object", vec!["1"]),
        ("Array", vec![]),
        ("Array", vec!["3"]),
        ("Array", vec!["1", "2", "3"]),
        ("Array", vec!["-1"]),
        ("Boolean", vec!["0"]),
        ("Number", vec!["'5'"]),
        ("String", vec!["1"]),
        ("Date", vec!["0"]),
        ("Date", vec!["2000", "0", "1"]),
        ("RegExp", vec!["'a'", "'g'"]),
        ("RegExp", vec!["'a'", "'x'"]),
        ("Error", vec!["'m'"]),
        ("TypeError", vec!["'m'"]),
        ("Function", vec!["'return 1'"]),
        ("Function", vec!["'a'", "'return a'"]),
        ("Math", vec![]),
        ("JSON", vec![]),
        ("isNaN", vec![]),
    ];
    for (name, args) in ctors {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let _ = take_reports();
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                (api.js_setreport)(j, Some(report_cb));
                let cn = cs(name);
                (api.js_getglobal)(j, cn.as_ptr());
                (api.js_pushundefined)(j);
                let fname = cs("[string]");
                for a in &args {
                    let csrc = cs(&format!("({a})"));
                    if (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr()) == 0 {
                        (api.js_pushundefined)(j);
                        (api.js_pcall)(j, 0);
                    }
                }
                let rc = (api.js_pconstruct)(j, args.len() as c_int);
                let errs = cs("<throw>");
                let s = rstr((api.js_trystring)(j, -1, errs.as_ptr()));
                let d = describe(api, j, -1);
                let top = (api.js_gettop)(j);
                (api.js_freestate)(j);
                outs.push((rc, s, d, top, take_reports()));
            }
        }
        assert_eq!(outs[0], outs[1], "js_pconstruct({name}, {args:?})");
    }
}

#[test]
fn row46_loadstring_eval_inside_try() {
    // js_loadstring + js_eval (unprotected) reached through a host callback so
    // exceptions are caught by the library itself.
    let sources: Vec<String> = valid_scripts().into_iter().take(400).collect();
    for src in sources {
        diff_protected(&format!("loadstring/eval {src:?}"), 0, || {
            let src = src.clone();
            move |api: &Api, j: JS| unsafe {
                let fname = cs("[eval]");
                let csrc = cs(&src);
                (api.js_loadstring)(j, fname.as_ptr(), csrc.as_ptr());
                log(describe(api, j, -1));
                (api.js_pushundefined)(j);
                (api.js_call)(j, 0);
                log(describe(api, j, -1));
            }
        });
    }
    // js_eval consumes a string from the stack
    for src in ["1+1", "var q=2; q", "throw 1", "syntax ~", "(function(){return 3})()"] {
        diff_protected(&format!("js_eval {src:?}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                let csrc = cs(src);
                (api.js_pushstring)(j, csrc.as_ptr());
                (api.js_eval)(j);
                log(describe(api, j, -1));
            }
        });
    }
}
