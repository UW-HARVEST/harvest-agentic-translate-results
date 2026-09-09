//! CONFIGS.md rows 51-59, 61, 74-84, 86-87 — the composed pipeline: regexps
//! through the JS API, script vs eval compilation, the protected entry points,
//! the try/throw plumbing, repr, JSON and every builtin object.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

fn fix_tz() {
    // Date uses localtime()/mktime(); pin the zone so the corpus is reproducible.
    unsafe { std::env::set_var("TZ", "UTC") };
    unsafe extern "C" {
        fn tzset();
    }
    unsafe { tzset() };
}

/* ------------------------------------------------------------------ */
/* row 51 — js_newregexp, all 8 JS_REGEXP_* combinations               */
/* ------------------------------------------------------------------ */
fn task_newregexp(api: &Api, J: JS) {
    unsafe {
        let pats = [
            "a", "A", "^a", "a$", "^a$", "(a)(b)?", "\\d+", "[a-z]+", "a|B", ".",
        ];
        // all 8 documented combos plus out-of-range flag ints
        let flag_sets: Vec<c_int> = (0..8).chain([8, 16, -1, i32::MAX, i32::MIN, 255]).collect();
        for pat in pats {
            for flags in &flag_sets {
                (api.js_newregexp)(J, cs(pat).as_ptr(), *flags);
                emit(format!("newregexp {pat:?} flags={flags}: {}", dump_slot(api, J, -1)));
                for prop in ["source", "global", "ignoreCase", "multiline", "lastIndex"] {
                    (api.js_getproperty)(J, -1, cs(prop).as_ptr());
                    emit(format!("  {prop} -> {}", dump_slot(api, J, -1)));
                    (api.js_pop)(J, 1);
                }
                (api.js_setglobal)(J, cs("re").as_ptr());
                for subj in ["", "a", "A", "aa", "ab", "b\na", "xyz", "12a34"] {
                    eval(
                        api,
                        J,
                        &format!("exec {pat:?}/{flags} on {subj:?}"),
                        &format!(
                            "var r = re.exec({0:?}); r ? r.index+':'+r.lastIndex+':'+r.join('|') \
                             : 'null:'+re.lastIndex",
                            subj
                        ),
                    );
                    eval(
                        api,
                        J,
                        &format!("test {pat:?}/{flags} on {subj:?}"),
                        &format!("re.test({subj:?})+':'+re.lastIndex"),
                    );
                    eval(
                        api,
                        J,
                        &format!("replace {pat:?}/{flags} on {subj:?}"),
                        &format!("{subj:?}.replace(re,'<$&>')"),
                    );
                    eval(
                        api,
                        J,
                        &format!("split {pat:?}/{flags} on {subj:?}"),
                        &format!("{subj:?}.split(re).join(',')"),
                    );
                    eval(
                        api,
                        J,
                        &format!("match {pat:?}/{flags} on {subj:?}"),
                        &format!("var m={subj:?}.match(re); m?m.join('|'):'null'"),
                    );
                }
            }
        }
    }
}

#[test]
fn row51_newregexp_flag_combos() {
    fix_tz();
    diff_all_flags("js_newregexp flag combos", task_newregexp);
}

/* ------------------------------------------------------------------ */
/* row 52 — js_RegExp_prototype_exec called directly across the FFI     */
/* ------------------------------------------------------------------ */
fn task_regexp_exec_direct(api: &Api, J: JS) {
    unsafe {
        for pat in ["a", "(a)(b)?", "^a", "\\d+", "[a-z]", "a|b", "()", "(?:a)"] {
            for flags in 0..8 {
                (api.js_newregexp)(J, cs(pat).as_ptr(), flags);
                let re = (api.js_toregexp)(J, -1);
                emit(format!("exec-direct {pat:?} flags={flags}"));
                for subj in ["", "a", "ab", "ba", "aab", "A", "12", "b\nab"] {
                    let text = cs(subj);
                    // js_RegExp_prototype_exec reads/writes `re->last` for /g
                    (api.js_RegExp_prototype_exec)(J, re, text.as_ptr());
                    emit(format!("  {subj:?} -> {}", dump_slot(api, J, -1)));
                    if (api.js_isobject)(J, -1) != 0 {
                        for prop in ["index", "input", "length", "0", "1"] {
                            (api.js_getproperty)(J, -1, cs(prop).as_ptr());
                            emit(format!("    {prop} -> {}", dump_slot(api, J, -1)));
                            (api.js_pop)(J, 1);
                        }
                    }
                    (api.js_pop)(J, 1);
                    (api.js_getproperty)(J, -1, cs("lastIndex").as_ptr());
                    emit(format!("    lastIndex -> {}", dump_slot(api, J, -1)));
                    (api.js_pop)(J, 1);
                }
                (api.js_pop)(J, 1);
            }
        }
        // js_toregexp on a non-regexp must throw identically
        (api.js_newobject)(J);
        emit("about to js_toregexp on a plain object".into());
        let _ = (api.js_toregexp)(J, -1);
        emit("UNREACHABLE".into());
    }
}

#[test]
fn row52_regexp_exec_direct() {
    diff_all_flags("js_RegExp_prototype_exec", task_regexp_exec_direct);
}

/* ------------------------------------------------------------------ */
/* rows 53-57 — loadstring vs loadeval vs dostring, protected or not    */
/* ------------------------------------------------------------------ */
fn drive_snippets(api: &Api, J: JS, mode: c_int) {
    unsafe {
        let f = cs("snippet.js");
        for (i, src) in js_snippets().iter().enumerate() {
            let c = match std::ffi::CString::new(*src) {
                Ok(c) => c,
                Err(_) => continue,
            };
            match mode {
                // js_loadstring + js_call, both inside the task's protection
                0 => {
                    (api.js_loadstring)(J, f.as_ptr(), c.as_ptr());
                    (api.js_pushundefined)(J);
                    (api.js_call)(J, 0);
                    let sentinel = cs("<throw>");
                    emit(format!(
                        "[{i}] loadstring {src:?} -> {:?}",
                        s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
                    ));
                    (api.js_pop)(J, 1);
                }
                // js_loadeval + js_call (eval semantics: J->strict, current env)
                1 => {
                    (api.js_loadeval)(J, f.as_ptr(), c.as_ptr());
                    (api.js_pushundefined)(J);
                    (api.js_call)(J, 0);
                    let sentinel = cs("<throw>");
                    emit(format!(
                        "[{i}] loadeval {src:?} -> {:?}",
                        s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
                    ));
                    (api.js_pop)(J, 1);
                }
                // js_dostring (report callback captures the error text)
                2 => {
                    let rc = (api.js_dostring)(J, c.as_ptr());
                    emit(format!("[{i}] dostring {src:?} rc={rc}"));
                }
                // protected: js_ploadstring + js_pcall
                _ => {
                    eval(api, J, &format!("[{i}] ploadstring {src:?}"), src);
                }
            }
        }
    }
}

fn task_loadstring(api: &Api, J: JS) {
    drive_snippets(api, J, 0)
}
fn task_loadeval(api: &Api, J: JS) {
    drive_snippets(api, J, 1)
}
fn task_dostring(api: &Api, J: JS) {
    drive_snippets(api, J, 2)
}
fn task_protected(api: &Api, J: JS) {
    drive_snippets(api, J, 3)
}

#[test]
fn row53_loadstring() {
    fix_tz();
    diff_all_flags("js_loadstring + js_call", task_loadstring);
}
#[test]
fn row54_loadeval() {
    fix_tz();
    diff_all_flags("js_loadeval + js_call", task_loadeval);
}
#[test]
fn row55_56_dostring() {
    fix_tz();
    diff("js_dostring [flags=0]", Cfg::default(), task_dostring);
    diff("js_dostring [JS_STRICT]", Cfg::strict(), task_dostring);
    diff("js_dostring [custom alloc]", Cfg::alloc(), task_dostring);
}
#[test]
fn row57_protected_entry_points() {
    fix_tz();
    diff_all_flags("protected entry points", task_protected);
}

/* ------------------------------------------------------------------ */
/* row 57b — js_pconstruct over every callable and non-callable shape   */
/* ------------------------------------------------------------------ */
fn task_pconstruct(api: &Api, J: JS) {
    unsafe {
        let exprs = [
            "Object", "Array", "Boolean", "Number", "String", "RegExp", "Error", "TypeError",
            "Date", "Function", "Math", "JSON", "(function(){this.x=1})", "(function(){return 5})",
            "(function(){return {y:2}})", "1", "'s'", "null", "undefined", "true", "({})", "[]",
            "/x/",
        ];
        for e in exprs {
            for argc in 0..3 {
                let args: String = (0..argc).map(|i| format!(",{i}")).collect();
                eval(
                    api,
                    J,
                    &format!("new {e} argc={argc}"),
                    &format!(
                        "try {{ var o = new ({e})({}); typeof o + ':' + \
                         (o&&o.x)+':'+(o&&o.y) }} catch (err) {{ err.name+': '+err.message }}",
                        args.trim_start_matches(',')
                    ),
                );
            }
            // and through js_pconstruct directly
            let f = cs("c.js");
            let c = cs(e);
            if (api.js_ploadstring)(J, f.as_ptr(), c.as_ptr()) != 0 {
                (api.js_pop)(J, 1);
                continue;
            }
            (api.js_pushundefined)(J);
            if (api.js_pcall)(J, 0) != 0 {
                (api.js_pop)(J, 1);
                continue;
            }
            for argc in 0..3 {
                // `new Date()` (argc == 0) and `Date(...)` called as a plain
                // function both read the wall clock, so their results can never
                // match between two sequential calls. Every other arity of Date
                // is deterministic and IS compared.
                let wallclock = e == "Date" && argc == 0;
                if !wallclock {
                    let base = (api.js_gettop)(J);
                    (api.js_copy)(J, -1);
                    for i in 0..argc {
                        (api.js_pushnumber)(J, i as f64);
                    }
                    let rc = (api.js_pconstruct)(J, argc);
                    emit(format!(
                        "pconstruct {e} argc={argc} rc={rc} -> {}",
                        dump_slot(api, J, -1)
                    ));
                    restore_top(api, J, base);
                }
                if e != "Date" {
                    let base = (api.js_gettop)(J);
                    (api.js_copy)(J, -1);
                    (api.js_pushnull)(J);
                    for i in 0..argc {
                        (api.js_pushnumber)(J, i as f64);
                    }
                    let rc = (api.js_pcall)(J, argc);
                    emit(format!(
                        "pcall {e} argc={argc} rc={rc} -> {}",
                        dump_slot(api, J, -1)
                    ));
                    restore_top(api, J, base);
                }
            }
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn row57b_pconstruct() {
    diff_all_flags("js_pconstruct / js_pcall over all shapes", task_pconstruct);
}

/* ------------------------------------------------------------------ */
/* rows 58, 86 — try/throw plumbing and JS_TRYLIMIT                    */
/* ------------------------------------------------------------------ */
fn task_try_depth(api: &Api, J: JS) {
    unsafe {
        // balanced js_savetry/js_endtry pairs (never jumped through, so no
        // setjmp is needed) up to and past JS_TRYLIMIT (64).
        for depth in [1usize, 10, 32, 60, 63] {
            for _ in 0..depth {
                let _ = (api.js_savetry)(J);
            }
            for _ in 0..depth {
                (api.js_endtry)(J);
            }
            emit(format!("savetry/endtry balanced depth={depth} ok"));
        }
        // nested JS try/catch (uses js_savetrypc) around and past the limit
        for depth in [1usize, 8, 32, 60, 63, 64, 65, 70, 200] {
            let src = format!(
                "{}throw 1;{}",
                "try{".repeat(depth),
                "}catch(e){}".repeat(depth)
            );
            eval(api, J, &format!("nested try depth={depth}"), &src);
        }
        // nested function calls each holding a protected frame
        for depth in [1usize, 10, 40, 62, 63, 64, 70, 100] {
            let src = format!(
                "var n=0; function f(k){{ if (k<=0) throw 'bottom'; n++; \
                 try {{ f(k-1) }} catch (e) {{ throw e }} }} \
                 try {{ f({depth}) }} catch (e) {{ e+':'+n }}"
            );
            eval(api, J, &format!("recursive try depth={depth}"), &src);
        }
        // try/finally interactions, and throwing out of finally
        for src in [
            "var s=''; try { s+='t'; throw 1 } catch (e) { s+='c' } finally { s+='f' } s",
            "var s=''; try { s+='t' } finally { s+='f' } s",
            "(function(){ try { return 'r' } finally { } })()",
            "(function(){ try { return 'r' } finally { return 'f' } })()",
            "(function(){ try { throw 1 } finally { return 'f' } })()",
            "var s=''; try { try { throw 1 } finally { s+='f1' } } catch (e) { s+='c' } s",
            "try { throw new Error('x') } catch (e) { e instanceof Error }",
            "try { } catch (e) { } finally { }",
            "(function(){ for (;;) { try { break } finally { } } return 'b' })()",
            "(function(){ var s=''; for (var i=0;i<3;i++) { try { continue } finally { s+='f' } } \
             return s })()",
        ] {
            eval(api, J, "try/finally", src);
        }
        // js_throw from a C frame that is protected by the task's own pcall
        emit("about to throw an uncaught-in-JS error".into());
        (api.js_newerror)(J, cs("deliberate").as_ptr());
        (api.js_throw)(J);
        emit("UNREACHABLE".into());
    }
}

#[test]
fn row58_86_try_plumbing() {
    diff_all_flags("try/throw plumbing", task_try_depth);
}

/* also drive the JS_TRYLIMIT overflow via nested protected calls */
unsafe extern "C-unwind" fn nest_cb(J: JS) {
    let api = active_api();
    unsafe {
        let n = (api.js_tryinteger)(J, 1, 0);
        emit(format!("nest depth {n}"));
        if n <= 0 {
            (api.js_pushstring)(J, cs("bottom").as_ptr());
            return;
        }
        (api.js_getglobal)(J, cs("nest").as_ptr());
        (api.js_pushnull)(J);
        (api.js_pushnumber)(J, (n - 1) as f64);
        let rc = (api.js_pcall)(J, 1);
        let sentinel = cs("<throw>");
        emit(format!(
            "  level {n} rc={rc} -> {:?}",
            s((api.js_trystring)(J, -1, sentinel.as_ptr()))
        ));
    }
}

fn task_trylimit(api: &Api, J: JS) {
    unsafe {
        (api.js_newcfunction)(J, nest_cb, sstr("nest"), 1);
        (api.js_setglobal)(J, cs("nest").as_ptr());
        for depth in [1, 30, 58, 60, 61, 62, 63, 64, 70, 120] {
            let base = (api.js_gettop)(J);
            (api.js_getglobal)(J, cs("nest").as_ptr());
            (api.js_pushnull)(J);
            (api.js_pushnumber)(J, depth as f64);
            let rc = (api.js_pcall)(J, 1);
            let sentinel = cs("<throw>");
            emit(format!(
                "trylimit depth={depth} rc={rc} -> {:?}",
                s((api.js_trystring)(J, -1, sentinel.as_ptr()))
            ));
            restore_top(api, J, base);
        }
    }
}

#[test]
fn row58b_trylimit_overflow() {
    diff_all_flags("JS_TRYLIMIT overflow", task_trylimit);
}

/* ------------------------------------------------------------------ */
/* row 59 — js_repr / js_torepr / js_tryrepr                           */
/* ------------------------------------------------------------------ */
fn task_repr(api: &Api, J: JS) {
    unsafe {
        let exprs = [
            "undefined", "null", "true", "false", "0", "-0", "1.5", "NaN", "Infinity", "-Infinity",
            "1e21", "''", "'a'", "'\\n\\t\\\\\\''", "'\\u0000'", "'\\u00e9\\u20ac'", "'\\ud83d\\ude00'",
            "{}", "({a:1})", "({a:1,b:'s',c:null,d:[1,2]})", "[]", "[1,2,3]", "[[1],[2,[3]]]",
            "[undefined,null,,3]", "(function(){})", "(function f(a,b){return a})", "Math", "JSON",
            "/a+/gim", "new Date(0)", "new Error('m')", "new TypeError('t')", "new Number(1)",
            "new String('s')", "new Boolean(true)", "Object", "Array.prototype",
            "(function(){var a=[]; a[3]=1; return a})()",
            "(function(){var o={}; o['a b']=1; o['0']=2; return o})()",
            "Object.defineProperty({},'h',{value:1,enumerable:false})",
            "({get x(){return 1}})",
            "({toString:function(){return 'TS'}})",
            "({toString:function(){throw new Error('nope')}})",
            "({valueOf:function(){throw new Error('nope')}})",
        ];
        for e in exprs {
            let f = cs("repr.js");
            let c = cs(e);
            if (api.js_ploadstring)(J, f.as_ptr(), c.as_ptr()) != 0 {
                let sentinel = cs("<throw>");
                emit(format!(
                    "{e}: LOAD-ERR {:?}",
                    s((api.js_trystring)(J, -1, sentinel.as_ptr()))
                ));
                (api.js_pop)(J, 1);
                continue;
            }
            (api.js_pushundefined)(J);
            if (api.js_pcall)(J, 0) != 0 {
                let sentinel = cs("<throw>");
                emit(format!(
                    "{e}: CALL-ERR {:?}",
                    s((api.js_trystring)(J, -1, sentinel.as_ptr()))
                ));
                (api.js_pop)(J, 1);
                continue;
            }
            let sentinel = cs("<throw>");
            emit(format!(
                "{e}: tryrepr={:?}",
                s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
            ));
            (api.js_repr)(J, -1);
            emit(format!("  js_repr pushed: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            emit(format!("  torepr={:?}", s((api.js_torepr)(J, -1))));
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn row59_repr() {
    fix_tz();
    diff_all_flags("repr", task_repr);
}

/* ------------------------------------------------------------------ */
/* row 61 — JSON                                                       */
/* ------------------------------------------------------------------ */
fn task_json(api: &Api, J: JS) {
    unsafe {
        let texts = [
            "null", "true", "false", "0", "-0", "1", "-1", "1.5", "1e10", "1e-10", "1e999",
            "\"\"", "\"a\"", "\"\\\\n\"", "\"\\\\u0041\"", "\"\\\\ud83d\\\\ude00\"", "\"\\\\uD800\"",
            "[]", "[1]", "[1,2,3]", "[[[[[1]]]]]", "{}", "{\"a\":1}", "{\"a\":{\"b\":[1,2]}}",
            "{\"a\":1,\"a\":2}", " [ 1 , 2 ] ", "[1,]", "{,}", "{a:1}", "'a'", "01", ".5", "+1",
            "NaN", "Infinity", "undefined", "", "  ", "[1 2]", "{\"a\" 1}", "\"unterminated",
            "[", "]", "{", "}", "\"\\\\x41\"", "\"\\\\q\"", "[null,true,false]",
            "1 2", "[1][2]", "\"\\\\u00e9\"", "{\"\\\\u0041\":1}",
        ];
        for t in texts {
            eval(
                api,
                J,
                &format!("JSON.parse {t:?}"),
                &format!(
                    "try {{ JSON.stringify(JSON.parse({t:?})) }} catch (e) {{ e.name+': '+e.message }}"
                ),
            );
        }
        let values = [
            "undefined", "null", "true", "1", "NaN", "Infinity", "-0", "1e21", "''", "'a\\nb'",
            "'\\u00e9'", "'\\ud83d\\ude00'", "[]", "[1,2]", "[undefined,function(){}]",
            "({})", "({a:1})", "({a:undefined})", "({a:function(){}})", "({toJSON:function(){return 5}})",
            "({a:{toJSON:function(){return [1]}}})", "new Date(0)", "new Number(3)",
            "new String('s')", "new Boolean(false)", "/x/", "(function(){})",
            "Object.defineProperty({},'h',{value:1,enumerable:false})",
            "(function(){var a=[]; a[2]=1; return a})()",
        ];
        let indents = ["", ",null,0", ",null,1", ",null,2", ",null,10", ",null,20",
                       ",null,'\\t'", ",null,'--'", ",null,'0123456789abc'", ",null,-1"];
        for v in values {
            for ind in indents {
                eval(
                    api,
                    J,
                    &format!("JSON.stringify {v} {ind}"),
                    &format!("try {{ String(JSON.stringify({v}{ind})) }} catch (e) {{ e.name }}"),
                );
            }
        }
        // replacer array + replacer function + reviver
        for src in [
            "JSON.stringify({a:1,b:2,c:3},['a','c'])",
            "JSON.stringify({a:1,b:2},function(k,v){return typeof v=='number'?v*2:v})",
            "JSON.stringify([1,2],function(k,v){return v})",
            "JSON.stringify({a:1},[])",
            "JSON.stringify({a:1},['a','a'])",
            "JSON.parse('{\"a\":1}',function(k,v){return typeof v=='number'?v+1:v}).a",
            "JSON.parse('[1,2]',function(k,v){return v}).join()",
            "JSON.parse('{\"a\":{\"b\":1}}',function(k,v){return v}).a.b",
            "JSON.stringify({a:1},function(){throw new Error('r')})",
            "try{JSON.stringify({a:1},function(){throw new Error('r')})}catch(e){e.message}",
        ] {
            eval(api, J, "JSON adv", src);
        }
        // deep nesting
        for d in [1usize, 10, 100, 400, 1000] {
            let t = format!("{}1{}", "[".repeat(d), "]".repeat(d));
            eval(
                api,
                J,
                &format!("JSON deep {d}"),
                &format!("try {{ JSON.stringify(JSON.parse({t:?})).length }} catch (e) {{ e.name }}"),
            );
        }
    }
}

#[test]
fn row61_json() {
    fix_tz();
    diff_all_flags("JSON", task_json);
}

/* ------------------------------------------------------------------ */
/* rows 74-84 — the builtin objects, driven with randomized values      */
/* ------------------------------------------------------------------ */
fn lit(v: f64) -> String {
    if v.is_nan() {
        "NaN".into()
    } else if v == f64::INFINITY {
        "Infinity".into()
    } else if v == f64::NEG_INFINITY {
        "-Infinity".into()
    } else {
        // exact hexadecimal-free round-trip: 17 significant digits is exact for f64
        format!("{:?}", v)
    }
}

fn task_math(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0074);
        let unary = [
            "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "round", "sin",
            "sqrt", "tan",
        ];
        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
            1e-10, 1e10, 1e300, -1e300, 0.49999999999999994, 2.5, -2.5, 1.5, -1.5,
            std::f64::consts::PI, std::f64::consts::E,
        ];
        for _ in 0..300 {
            vals.push(rng.f64());
        }
        for f in unary {
            for v in &vals {
                eval(
                    api,
                    J,
                    &format!("Math.{f}({v:?})"),
                    &format!("String(Math.{f}({}))", lit(*v)),
                );
            }
        }
        for f in ["atan2", "pow", "max", "min"] {
            for _ in 0..300 {
                let a = rng.f64();
                let b = rng.f64();
                eval(
                    api,
                    J,
                    &format!("Math.{f}({a:?},{b:?})"),
                    &format!("String(Math.{f}({},{}))", lit(a), lit(b)),
                );
            }
        }
        for src in [
            "Math.max()", "Math.min()", "Math.max(1)", "Math.max(NaN,1)", "Math.min(NaN,1)",
            "Math.max(-0,0)", "Math.min(-0,0)", "String(Math.random()===Math.random())",
            "typeof Math.random()",
            "Math.E+','+Math.LN10+','+Math.LN2+','+Math.LOG10E+','+Math.LOG2E+','+Math.PI+','+\
             Math.SQRT1_2+','+Math.SQRT2",
            "Object.getOwnPropertyNames(Math).sort().join(',')",
        ] {
            eval(api, J, "Math misc", src);
        }
    }
}

#[test]
fn row74_math() {
    diff_all_flags("Math", task_math);
}

fn task_number_format(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0075);
        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, 1.005, 1.5, 2.5, 1e-7, 1e20, 1e21, 1e-21, 123.456,
            0.000001234, 999999999999999999999.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
            f64::MAX, f64::MIN_POSITIVE, 1234567890.12345,
        ];
        for _ in 0..120 {
            vals.push(rng.f64());
        }
        for v in &vals {
            for d in [0usize, 1, 2, 3, 5, 10, 15, 20, 21, 100] {
                eval(
                    api,
                    J,
                    &format!("toFixed {v:?} {d}"),
                    &format!("try {{ ({}).toFixed({d}) }} catch (e) {{ e.name }}", lit(*v)),
                );
                eval(
                    api,
                    J,
                    &format!("toExponential {v:?} {d}"),
                    &format!("try {{ ({}).toExponential({d}) }} catch (e) {{ e.name }}", lit(*v)),
                );
                eval(
                    api,
                    J,
                    &format!("toPrecision {v:?} {d}"),
                    &format!("try {{ ({}).toPrecision({d}) }} catch (e) {{ e.name }}", lit(*v)),
                );
            }
            for radix in [1usize, 2, 3, 8, 10, 16, 35, 36, 37] {
                eval(
                    api,
                    J,
                    &format!("toString {v:?} radix {radix}"),
                    &format!("try {{ ({}).toString({radix}) }} catch (e) {{ e.name }}", lit(*v)),
                );
            }
            eval(
                api,
                J,
                &format!("String {v:?}"),
                &format!("String({})+'|'+({}).toString()+'|'+({}).valueOf()", lit(*v), lit(*v), lit(*v)),
            );
        }
        for src in [
            "Number.MAX_VALUE+','+Number.MIN_VALUE+','+Number.NaN+','+\
             Number.POSITIVE_INFINITY+','+Number.NEGATIVE_INFINITY",
            "Number('')+','+Number(' ')+','+Number('0x10')+','+Number('1e3')+','+Number('abc')",
            "Number(null)+','+Number(undefined)+','+Number(true)+','+Number([])+','+Number([1])",
            "parseInt('')+','+parseInt('  42abc')+','+parseInt('-0x1f')+','+parseInt('11',2)",
            "parseInt('z',36)+','+parseInt('z',35)+','+parseInt('1',1)+','+parseInt('1',37)",
            "parseFloat('')+','+parseFloat('.5e1x')+','+parseFloat('Infinityx')",
            "isNaN(NaN)+','+isNaN('a')+','+isFinite(1)+','+isFinite(Infinity)",
            "(1).toFixed()+','+(1).toExponential()+','+(1).toPrecision()",
        ] {
            eval(api, J, "Number misc", src);
        }
    }
}

#[test]
fn row75_number_formatting() {
    diff_all_flags("Number formatting", task_number_format);
}

fn task_strings(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0076);
        let mut subs: Vec<String> = vec![
            "".into(), "a".into(), "abc".into(), "ABC".into(), "aAbB".into(),
            "h\u{e9}ll\u{f6}".into(), "\u{20ac}\u{20ac}".into(), "\u{1f600}".into(),
            "  pad  ".into(), "a,b,,c".into(), "\n\t\r".into(), "\u{130}\u{131}".into(),
            "\u{df}".into(), "\u{fb00}".into(), "0123456789".into(),
        ];
        for _ in 0..60 {
            let n = rng.below(10) as usize;
            let s: String = (0..n)
                .map(|_| *rng.pick(&['a', 'B', '0', ' ', ',', '\u{e9}', '\u{20ac}', '\u{1f600}']))
                .collect();
            subs.push(s);
        }
        let unary = [
            "toUpperCase", "toLowerCase", "toLocaleUpperCase", "toLocaleLowerCase", "trim",
            "valueOf", "toString",
        ];
        for sv in &subs {
            let q = format!("{sv:?}");
            for m in unary {
                eval(api, J, &format!("{q}.{m}"), &format!("({q}).{m}()"));
            }
            eval(api, J, &format!("{q}.length"), &format!("({q}).length"));
            for i in -2i32..5 {
                eval(
                    api,
                    J,
                    &format!("{q}.charAt({i})"),
                    &format!("({q}).charAt({i})+'|'+({q}).charCodeAt({i})"),
                );
            }
            for (a, b) in [(0i32, 2i32), (1, 1), (2, 0), (-2, -1), (0, 100), (-100, 100)] {
                eval(
                    api,
                    J,
                    &format!("{q}.slice({a},{b})"),
                    &format!(
                        "({q}).slice({a},{b})+'|'+({q}).substring({a},{b})+'|'+({q}).substr({a},{b})"
                    ),
                );
            }
            for needle in ["", "a", "b", "abc", "\u{20ac}"] {
                eval(
                    api,
                    J,
                    &format!("{q}.indexOf({needle:?})"),
                    &format!(
                        "({q}).indexOf({needle:?})+'|'+({q}).lastIndexOf({needle:?})+'|'+\
                         ({q}).split({needle:?}).join('#')+'|'+({q}).concat({needle:?})+'|'+\
                         ({q}).localeCompare({needle:?})+'|'+({q}).replace({needle:?},'X')"
                    ),
                );
            }
            for re in ["/a/", "/a/g", "/A/i", "/(a)(b)?/", "/^/", "/$/g", "/./g", "/\\\\s+/g"] {
                eval(
                    api,
                    J,
                    &format!("{q} {re}"),
                    &format!(
                        "var m=({q}).match({re}); (m?m.join('|'):'null')+'/'+\
                         ({q}).search({re})+'/'+({q}).replace({re},'<$&>')+'/'+\
                         ({q}).split({re}).join('#')"
                    ),
                );
            }
            eval(
                api,
                J,
                &format!("{q} replace fn"),
                &format!(
                    "({q}).replace(/(a)/g,function(m,p1,off,str){{return '['+m+p1+off+str.length+']'}})"
                ),
            );
            eval(
                api,
                J,
                &format!("{q} replace patterns"),
                &format!("({q}).replace(/(a)(b)?/,'$$|$&|$`|$\\'|$1|$2|$3')"),
            );
        }
        for src in [
            "String.fromCharCode()+','+String.fromCharCode(0)+','+String.fromCharCode(65,66)",
            "String.fromCharCode(0x10000)+','+String.fromCharCode(-1)+','+String.fromCharCode(NaN)",
            "String()+','+String(null)+','+String(undefined)+','+String(1)+','+String([1,2])",
            "'a'.repeat===undefined",
            "Object.getOwnPropertyNames(String.prototype).sort().join(',')",
            "'abc'[1]+','+'abc'['1']+','+'abc'[5]",
            "'abc'.split(undefined).join('#')",
            "'abc'.split('',2).join('#')",
            "'a-b'.split('-',0).length",
        ] {
            eval(api, J, "String misc", src);
        }
    }
}

#[test]
fn row76_77_strings() {
    diff_all_flags("String prototype", task_strings);
}

fn task_arrays_builtin(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0078);
        let mut arrays: Vec<String> = vec![
            "[]".into(),
            "[1]".into(),
            "[1,2,3]".into(),
            "[3,1,2]".into(),
            "[3,20,100]".into(),
            "['b','a','c']".into(),
            "[1,'a',null,undefined,true,{},[]]".into(),
            "[undefined,undefined]".into(),
            "(function(){var a=[1,2,3]; delete a[1]; return a})()".into(),
            "(function(){var a=[]; a[5]=1; return a})()".into(),
            "(function(){var a=[1]; a.x=2; return a})()".into(),
            "new Array(3)".into(),
            "new Array(1,2)".into(),
            "[NaN,0,-0,Infinity,-Infinity]".into(),
        ];
        for _ in 0..40 {
            let n = rng.below(8) as usize;
            let items: Vec<String> = (0..n).map(|_| format!("{}", rng.i32() % 1000)).collect();
            arrays.push(format!("[{}]", items.join(",")));
        }
        for a in &arrays {
            for m in [
                "join()", "join('-')", "join(undefined)", "reverse().join()", "sort().join()",
                "sort(function(x,y){return x<y?-1:x>y?1:0}).join()",
                "sort(function(x,y){return 0}).join()",
                "toString()", "toLocaleString()", "concat([9]).join()", "slice().join()",
                "slice(1).join()", "slice(-2).join()", "slice(1,2).join()",
                "indexOf(1)", "lastIndexOf(1)", "indexOf(1,1)",
                "every(function(v){return v})", "some(function(v){return v})",
                "filter(function(v){return v}).join()", "map(function(v,i){return i}).join()",
                "forEach(function(){})",
                "length", "pop()+','+this", "push(9)",
            ] {
                eval(api, J, &format!("{a}.{m}"), &format!("try {{ String(({a}).{m}) }} catch (e) {{ e.name }}"));
            }
            for m in [
                "reduce(function(x,y){return x+'/'+y})",
                "reduce(function(x,y){return x+'/'+y},'S')",
                "reduceRight(function(x,y){return x+'/'+y})",
                "reduceRight(function(x,y){return x+'/'+y},'S')",
                "shift()", "unshift(0)", "splice(1,1).join()", "splice(0,0,9).join()",
                "splice(-1).join()", "splice().join()",
            ] {
                eval(
                    api,
                    J,
                    &format!("{a}.{m}"),
                    &format!("try {{ var t={a}; String(t.{m})+'|'+t.join() }} catch (e) {{ e.name }}"),
                );
            }
            eval(api, J, &format!("{a} sort throwing"), &format!(
                "try {{ ({a}).sort(function(){{throw new Error('c')}}) }} catch (e) {{ e.message }}"
            ));
            eval(api, J, &format!("{a} sort bad cmp"), &format!(
                "try {{ String(({a}).sort(1)) }} catch (e) {{ e.name }}"
            ));
            eval(api, J, &format!("{a} enumerate"), &format!(
                "var s=''; for (var k in ({a})) s+=k+','; s"
            ));
        }
        for src in [
            "Array.isArray===undefined",
            "Array(3).length+','+Array('3').length",
            "Object.getOwnPropertyNames(Array.prototype).sort().join(',')",
            "var a=[1,2,3]; a.length=0; a.join()",
            "var a=[1,2,3]; a.length=5; a.join()+':'+a.length",
            "try { var a=[]; a.length=-1 } catch(e) { e.name }",
            "try { var a=[]; a.length=1.5 } catch(e) { e.name }",
            "Array.prototype.join.call({length:2,0:'a',1:'b'})",
            "Array.prototype.slice.call('abc').join()",
            "try { Array.prototype.toString.call(null) } catch (e) { e.name }",
        ] {
            eval(api, J, "Array misc", src);
        }
    }
}

#[test]
fn row78_arrays_builtin() {
    diff_all_flags("Array prototype", task_arrays_builtin);
}

fn task_date(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0079);
        let mut ts: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 1000.0, 86400000.0, -86400000.0, 1234567890123.0,
            8640000000000000.0, -8640000000000000.0, 8640000000000001.0, f64::NAN,
            f64::INFINITY, f64::NEG_INFINITY, 1.5, 946684800000.0, 951782400000.0,
        ];
        for _ in 0..200 {
            ts.push((rng.i32() as f64) * 1000.0);
            ts.push(rng.f64());
        }
        let getters = [
            "getTime", "valueOf", "getUTCFullYear", "getUTCMonth", "getUTCDate", "getUTCDay",
            "getUTCHours", "getUTCMinutes", "getUTCSeconds", "getUTCMilliseconds",
            "getTimezoneOffset", "toISOString", "toJSON", "toUTCString", "toString",
            "toDateString", "toTimeString", "getFullYear", "getMonth", "getDate", "getDay",
            "getHours", "getMinutes", "getSeconds", "getMilliseconds", "toLocaleString",
            "toLocaleDateString", "toLocaleTimeString",
        ];
        for t in &ts {
            for g in getters {
                eval(
                    api,
                    J,
                    &format!("Date({t:?}).{g}"),
                    &format!("try {{ String(new Date({}).{g}()) }} catch (e) {{ e.name }}", lit(*t)),
                );
            }
        }
        for s in [
            "1970-01-01T00:00:00Z", "2001-02-03T04:05:06Z", "2001-02-03T04:05:06.789Z",
            "2001-02-03", "2001-02", "2001", "2001-02-03T04:05", "2001-02-03T04:05:06+01:00",
            "2001-02-03T04:05:06-01:00", "Thu Jan 01 1970 00:00:00 GMT+0000",
            "invalid", "", "0", "1970", "+002001-02-03T00:00:00Z", "2001-13-03",
            "2001-02-30", "2001-02-03T25:00:00Z",
        ] {
            eval(
                api,
                J,
                &format!("Date.parse {s:?}"),
                &format!("String(Date.parse({s:?}))+'|'+String(new Date({s:?}))"),
            );
        }
        for src in [
            "Date.UTC(2000,0,1)", "Date.UTC(2000)", "Date.UTC()", "Date.UTC(2000,0,1,2,3,4,5)",
            "Date.UTC(NaN)", "Date.UTC(1e10,0,1)",
            "new Date(2000,0,1).getUTCFullYear()",
            "new Date(2000,0,1,2,3,4,5).getTime()",
            "new Date(0,0).getTime()",
            "typeof Date.now()",
            "new Date(0).setTime(1000)",
            "var d=new Date(0); d.setUTCFullYear(2000); d.toISOString()",
            "var d=new Date(0); d.setUTCMonth(13); d.toISOString()",
            "var d=new Date(0); d.setUTCDate(0); d.toISOString()",
            "var d=new Date(0); d.setUTCHours(25); d.toISOString()",
            "var d=new Date(0); d.setUTCMinutes(-1); d.toISOString()",
            "var d=new Date(0); d.setUTCSeconds(1e10); String(d)",
            "var d=new Date(0); d.setUTCMilliseconds(NaN); String(d)",
            "var d=new Date(0); d.setFullYear(2000); d.getUTCFullYear()",
            "try { new Date(NaN).toISOString() } catch (e) { e.name }",
            "String(new Date(8640000000000001))",
            "Object.getOwnPropertyNames(Date.prototype).sort().join(',')",
            "typeof Date()",
            "new Date(0) instanceof Date",
        ] {
            eval(api, J, "Date misc", src);
        }
    }
}

#[test]
fn row79_date() {
    fix_tz();
    diff_all_flags("Date", task_date);
}

fn task_object_builtin(api: &Api, J: JS) {
    unsafe {
        for src in [
            "Object.keys({a:1,b:2}).join()",
            "Object.keys([1,2]).join()",
            "Object.keys('ab').join()",
            "try { Object.keys(1) } catch (e) { e.name }",
            "try { Object.keys(null) } catch (e) { e.name }",
            "Object.getOwnPropertyNames({a:1}).join()",
            "Object.getOwnPropertyNames([1]).sort().join()",
            "JSON.stringify(Object.getOwnPropertyDescriptor({a:1},'a'))",
            "String(Object.getOwnPropertyDescriptor({a:1},'b'))",
            "JSON.stringify(Object.getOwnPropertyDescriptor([1],'length'))",
            "var o={}; Object.defineProperty(o,'a',{value:1}); JSON.stringify(o)+Object.keys(o).length",
            "var o={}; Object.defineProperty(o,'a',{value:1,enumerable:true}); Object.keys(o).join()",
            "var o={}; Object.defineProperties(o,{a:{value:1},b:{value:2}}); \
             Object.getOwnPropertyNames(o).sort().join()",
            "var o=Object.create({p:1}); o.p+','+Object.keys(o).length",
            "var o=Object.create(null); String(Object.getPrototypeOf(o))",
            "var o=Object.create({},{a:{value:1,enumerable:true}}); Object.keys(o).join()",
            "Object.getPrototypeOf([])===Array.prototype",
            "Object.getPrototypeOf(Object.prototype)===null",
            "try { Object.getPrototypeOf(1) } catch (e) { e.name }",
            "var o=Object.freeze({a:1}); o.a=2; o.a+','+Object.isFrozen(o)",
            "var o=Object.seal({a:1}); o.b=2; Object.keys(o).join()+','+Object.isSealed(o)",
            "var o=Object.preventExtensions({a:1}); o.b=2; Object.keys(o).join()+','+\
             Object.isExtensible(o)",
            "Object.isFrozen({})+','+Object.isSealed({})+','+Object.isExtensible({})",
            "({}).hasOwnProperty('a')+','+({a:1}).hasOwnProperty('a')",
            "({}).isPrototypeOf({})+','+Object.prototype.isPrototypeOf([])",
            "({a:1}).propertyIsEnumerable('a')+','+[].propertyIsEnumerable('length')",
            "({}).toString()+','+({}).toLocaleString()+','+String(({}).valueOf())",
            "Object.prototype.toString.call(null)+Object.prototype.toString.call(undefined)",
            "Object.prototype.toString.call([])+Object.prototype.toString.call(1)+\
             Object.prototype.toString.call('a')+Object.prototype.toString.call(/x/)+\
             Object.prototype.toString.call(new Date(0))+Object.prototype.toString.call(function(){})",
            "Object(1) instanceof Number",
            "String(Object())+','+String(Object(null))+','+String(Object(undefined))",
            "var o={}; o.__proto__===Object.prototype",
            "try { Object.defineProperty({},'a',{get:1}) } catch (e) { e.name }",
            "try { Object.defineProperty({},'a',{value:1,get:function(){}}) } catch (e) { e.name }",
            "try { Object.defineProperty(1,'a',{}) } catch (e) { e.name }",
            "var o={}; Object.defineProperty(o,'a',{value:1,writable:false}); \
             try { 'use strict'; o.a=2 } catch (e) { e.name }; o.a",
            "var o={}; Object.defineProperty(o,'a',{value:1,configurable:false}); \
             try { delete o.a } catch (e) { e.name }; o.a",
            "var o={}; Object.defineProperty(o,'a',{value:1}); \
             try { Object.defineProperty(o,'a',{value:2}) } catch (e) { e.name+':'+o.a }",
            "Object.getOwnPropertyNames(Object).sort().join(',')",
            "Object.getOwnPropertyNames(Object.prototype).sort().join(',')",
        ] {
            eval(api, J, "Object", src);
        }
    }
}

#[test]
fn row80_object_builtin() {
    diff_all_flags("Object builtin", task_object_builtin);
}

fn task_uri(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0081);
        let mut inputs: Vec<String> = vec![
            "".into(), "a".into(), "a b".into(), "a+b".into(), "/?:@&=+$,#".into(),
            "-_.!~*'()".into(), "%".into(), "%2".into(), "%zz".into(), "%41".into(),
            "%C3%A9".into(), "%E2%82%AC".into(), "%F0%9F%98%80".into(), "%ED%A0%80".into(),
            "%C0%80".into(), "%FF".into(), "%80".into(), "%u00e9".into(), "%u".into(),
            "\u{e9}".into(), "\u{20ac}".into(), "\u{1f600}".into(), "\u{7f}".into(),
            "\u{0}x".into(), "a%20b".into(), "%%".into(), "%C3".into(),
        ];
        for _ in 0..200 {
            let n = rng.below(10) as usize;
            let sv: String = (0..n)
                .map(|_| {
                    *rng.pick(&[
                        'a', 'Z', '0', '%', ' ', '+', '/', '?', '#', '&', '=', ':', '@', '$', ',',
                        ';', '-', '_', '.', '!', '~', '*', '\'', '(', ')', '\u{e9}', '\u{20ac}',
                        '\u{1f600}', '\u{7f}', 'C', '3', 'A', '9', 'F',
                    ])
                })
                .collect();
            inputs.push(sv);
        }
        for i in &inputs {
            for f in [
                "encodeURI", "decodeURI", "encodeURIComponent", "decodeURIComponent", "escape",
                "unescape",
            ] {
                eval(
                    api,
                    J,
                    &format!("{f}({i:?})"),
                    &format!("try {{ {f}({i:?}) }} catch (e) {{ e.name+': '+e.message }}"),
                );
            }
        }
    }
}

#[test]
fn row81_uri_functions() {
    diff_all_flags("URI functions", task_uri);
}

fn task_errors_builtin(api: &Api, J: JS) {
    unsafe {
        let ctors: [(&str, unsafe extern "C-unwind" fn(JS, *const c_char)); 7] = [
            ("Error", api.js_newerror),
            ("EvalError", api.js_newevalerror),
            ("RangeError", api.js_newrangeerror),
            ("ReferenceError", api.js_newreferenceerror),
            ("SyntaxError", api.js_newsyntaxerror),
            ("TypeError", api.js_newtypeerror),
            ("URIError", api.js_newurierror),
        ];
        for (name, f) in ctors {
            for msg in ["", "boom", "with \u{e9} unicode", "a\nb"] {
                f(J, cs(msg).as_ptr());
                emit(format!("{name}({msg:?}): {}", dump_slot(api, J, -1)));
                for prop in ["name", "message", "stackTrace"] {
                    (api.js_getproperty)(J, -1, cs(prop).as_ptr());
                    emit(format!("  {prop} -> {}", dump_slot(api, J, -1)));
                    (api.js_pop)(J, 1);
                }
                (api.js_pop)(J, 1);
            }
        }
        for src in [
            "new Error('m').toString()",
            "new Error().toString()",
            "String(new Error(undefined))",
            "new Error('m') instanceof Error",
            "new TypeError('m') instanceof Error",
            "Error('m').message",
            "Object.getOwnPropertyNames(Error.prototype).sort().join(',')",
            "var e=new Error('m'); e.name='X'; e.toString()",
            "var e=new Error('m'); e.message=''; e.toString()",
            "try { null.x } catch (e) { e instanceof TypeError }",
            "try { eval('(') } catch (e) { e instanceof SyntaxError }",
            "try { decodeURI('%') } catch (e) { e instanceof URIError }",
            "try { (1).toFixed(101) } catch (e) { e instanceof RangeError }",
            "try { nope } catch (e) { e instanceof ReferenceError }",
        ] {
            eval(api, J, "Error builtin", src);
        }
    }
}

#[test]
fn row82_error_constructors() {
    diff_all_flags("error constructors", task_errors_builtin);
}

/* ------------------------------------------------------------------ */
/* row 83 — the varargs error throwers, from a C callback               */
/* ------------------------------------------------------------------ */
unsafe extern "C-unwind" fn cf_throw_kind(J: JS) {
    let api = active_api();
    unsafe {
        let k = (api.js_tryinteger)(J, 1, 0);
        // The message doubles as a printf format string in the C original.
        let fmt = cs("kind %d and %s and %c and %g|%%|%5d|%-5s|");
        match k {
            0 => (api.js_error)(J, fmt.as_ptr()),
            1 => (api.js_evalerror)(J, fmt.as_ptr()),
            2 => (api.js_rangeerror)(J, fmt.as_ptr()),
            3 => (api.js_referenceerror)(J, fmt.as_ptr()),
            4 => (api.js_syntaxerror)(J, fmt.as_ptr()),
            5 => (api.js_typeerror)(J, fmt.as_ptr()),
            _ => (api.js_urierror)(J, fmt.as_ptr()),
        }
    }
}

unsafe extern "C-unwind" fn cf_throw_plain(J: JS) {
    let api = active_api();
    unsafe {
        let k = (api.js_tryinteger)(J, 1, 0);
        let msg = cs("plain message");
        match k {
            0 => (api.js_error)(J, msg.as_ptr()),
            1 => (api.js_evalerror)(J, msg.as_ptr()),
            2 => (api.js_rangeerror)(J, msg.as_ptr()),
            3 => (api.js_referenceerror)(J, msg.as_ptr()),
            4 => (api.js_syntaxerror)(J, msg.as_ptr()),
            5 => (api.js_typeerror)(J, msg.as_ptr()),
            _ => (api.js_urierror)(J, msg.as_ptr()),
        }
    }
}

fn task_vararg_errors(api: &Api, J: JS) {
    unsafe {
        for (name, f) in [
            ("plain", cf_throw_plain as unsafe extern "C-unwind" fn(JS)),
            // Note: the %-format variant is only called with no matching
            // arguments in the C original either; both libraries must agree.
        ] {
            for k in 0..7 {
                let base = (api.js_gettop)(J);
                (api.js_newcfunction)(J, f, sstr("t"), 1);
                (api.js_pushnull)(J);
                (api.js_pushnumber)(J, k as f64);
                let rc = (api.js_pcall)(J, 1);
                let sentinel = cs("<throw>");
                emit(format!(
                    "{name} kind={k} rc={rc} msg={:?} iserror={}",
                    s((api.js_trystring)(J, -1, sentinel.as_ptr())),
                    (api.js_iserror)(J, -1)
                ));
                (api.js_getproperty)(J, -1, cs("name").as_ptr());
                emit(format!("  name -> {}", dump_slot(api, J, -1)));
                restore_top(api, J, base);
            }
        }
        let _ = cf_throw_kind;
    }
}

#[test]
fn row83_vararg_errors() {
    diff_all_flags("vararg error throwers", task_vararg_errors);
}

/* ------------------------------------------------------------------ */
/* row 87 — full pipeline                                              */
/* ------------------------------------------------------------------ */
fn task_full_pipeline(api: &Api, J: JS) {
    unsafe {
        let prog = r#"
            var results = [];
            function Point(x, y) { this.x = x; this.y = y }
            Point.prototype.toString = function () { return '(' + this.x + ',' + this.y + ')' };
            for (var i = 0; i < 20; i++) results.push(new Point(i, i * i).toString());
            var m = {};
            for (var i = 0; i < 50; i++) m['k' + i] = i * 3;
            var keys = Object.keys(m).sort();
            var total = keys.reduce(function (a, k) { return a + m[k] }, 0);
            var text = results.join(' ') + '|' + keys.length + '|' + total;
            var re = /\((\d+),(\d+)\)/g, hit = 0, mm;
            while ((mm = re.exec(text)) !== null) hit += Number(mm[1]) + Number(mm[2]);
            var js = JSON.stringify({ text: text.slice(0, 40), hit: hit, keys: keys.slice(0, 3) });
            var back = JSON.parse(js);
            var d = new Date(0);
            var out = [
                text.length, hit, js.length, back.hit, back.keys.join(),
                d.toISOString(), (1 / 3).toFixed(10), (255).toString(16),
                encodeURIComponent('a b/c'), 'HÉLLÖ'.toLowerCase(),
                [3, 1, 2].sort().join(), typeof Point, Point.length
            ].join(';');
            out;
        "#;
        for pass in 0..3 {
            eval(api, J, &format!("pipeline pass {pass}"), prog);
            (api.js_gc)(J, 0);
            (api.js_gc)(J, 1);
        }
        // and via js_dostring
        let c = cs(prog);
        emit(format!("dostring rc={}", (api.js_dostring)(J, c.as_ptr())));
        (api.js_gc)(J, 1);
    }
}

#[test]
fn row87_full_pipeline() {
    fix_tz();
    diff_all_flags("full pipeline", task_full_pipeline);
    diff("full pipeline [memlimit 64MB]", Cfg::limits(0, 64 << 20), task_full_pipeline);
    diff("full pipeline [runlimit 60]", Cfg::limits(60, 0), task_full_pipeline);
}
