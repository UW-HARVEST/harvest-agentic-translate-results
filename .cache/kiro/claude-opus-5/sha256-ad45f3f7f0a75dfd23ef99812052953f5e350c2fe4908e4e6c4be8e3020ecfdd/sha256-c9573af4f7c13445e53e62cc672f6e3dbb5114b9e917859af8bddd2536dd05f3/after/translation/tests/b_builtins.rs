//! Phase B rows 63-75: the built-in library, exercised end to end through
//! script evaluation with randomized inputs.
mod common;
use common::*;

fn arg_pool() -> Vec<&'static str> {
    vec![
        "undefined", "null", "true", "false", "0", "-0", "1", "-1", "2", "3", "0.5", "-0.5",
        "1e21", "1e-7", "NaN", "Infinity", "-Infinity", "2147483647", "2147483648", "-2147483648",
        "-2147483649", "4294967295", "4294967296", "9007199254740993", "1.5", "255", "36",
        "''", "'0'", "'1'", "'a'", "'abc'", "'ABC'", "' '", "'\\t'", "'0x10'", "'1e3'",
        "'Infinity'", "'NaN'", "'\\u00e9'", "'\\ud83d\\ude00'", "'a,b,c'",
        "[]", "[1]", "[1,2,3]", "[,1,]", "[[1],[2]]", "({})", "({a:1})", "({length:2})",
        "(function(){})", "(function(x){return x})", "/a/", "/a/g", "new Date(0)",
        "new Number(1)", "new String('a')", "new Boolean(false)", "Math", "JSON",
    ]
}

/// Second-argument pool: the values a method's *index / count / limit /
/// flags* parameter is likely to branch on.  Kept small so the full
/// cross-product with `arg_pool()` stays affordable.
fn arg_pool2() -> Vec<&'static str> {
    vec![
        "undefined", "null", "0", "-0", "1", "2", "3", "-1", "-2", "-100", "-2147483649",
        "2147483647", "4294967296", "NaN", "Infinity", "-Infinity", "1.5", "true", "false",
        "''", "'1'", "'a'", "[]", "[1]", "({})", "(function(a,b){return a<b?-1:a>b?1:0})",
        "(function(){return 1})", "(function(){throw 'CB'})", "/a/", "/a/g",
    ]
}

fn wrap(s: &str) -> String {
    // Render the result so structured values are still fully compared, and
    // turn a throw into an observable string.
    format!(
        "try {{ var __r = {s}; (typeof __r === 'object' && __r !== null) ?          Object.prototype.toString.call(__r)+':'+JSON.stringify(__r)+':'+String(__r)          : (typeof __r)+':'+String(__r) }} catch(e) {{ 'E:'+String(e) }}"
    )
}

/// Full cross-product driver: every receiver x every method x
/// (0 args, every single argument, every (arg, arg2) pair, and a randomized
/// 3-argument sample).
fn run_cross(recv: &[&str], calls: &[&str]) {
    let args = arg_pool();
    let args2 = arg_pool2();
    let mut rng = Rng::new(0x6767);
    for r in recv {
        let mut scripts: Vec<String> = Vec::new();
        for c in calls {
            scripts.push(wrap(&format!("({r}).{c}()")));
            for a1 in &args {
                scripts.push(wrap(&format!("({r}).{c}({a1})")));
                for a2 in &args2 {
                    scripts.push(wrap(&format!("({r}).{c}({a1},{a2})")));
                }
            }
            for _ in 0..200 {
                let a1 = args[rng.below(args.len() as u32) as usize];
                let a2 = args2[rng.below(args2.len() as u32) as usize];
                let a3 = args[rng.below(args.len() as u32) as usize];
                scripts.push(wrap(&format!("({r}).{c}({a1},{a2},{a3})")));
            }
        }
        diff_eval_batch(&format!("cross {r}"), &scripts, 0);
        // Strict mode changes `this` coercion and several rejections; run a
        // deterministic 1-in-7 sample of the same scripts under JS_STRICT.
        let strict: Vec<String> = scripts.iter().step_by(7).cloned().collect();
        diff_eval_batch(&format!("cross {r} strict"), &strict, JS_STRICT);
    }
}

#[test]
fn row69_object_builtins() {
    run_cross(
        &["Object", "Object.prototype"],
        &[
            "getPrototypeOf",
            "getOwnPropertyDescriptor",
            "getOwnPropertyNames",
            "create",
            "defineProperty",
            "defineProperties",
            "keys",
            "seal",
            "freeze",
            "preventExtensions",
            "isSealed",
            "isFrozen",
            "isExtensible",
            "toString",
            "toLocaleString",
            "valueOf",
            "hasOwnProperty",
            "isPrototypeOf",
            "propertyIsEnumerable",
        ],
    );
}

#[test]
fn row69b_array_builtins() {
    run_cross(
        &["[]", "[1,2,3]", "[3,1,2]", "[,1,,2,]", "['b','a']", "Array.prototype"],
        &[
            "concat", "join", "pop", "push", "reverse", "shift", "slice", "sort", "splice",
            "unshift", "indexOf", "lastIndexOf", "every", "some", "forEach", "map", "filter",
            "reduce", "reduceRight", "toString", "toLocaleString",
        ],
    );
    run_cross(&["Array"], &["isArray"]);
}

#[test]
fn row70_string_builtins() {
    run_cross(
        &["''", "'abc'", "'ABC'", "'a,b,,c'", "'\\u00e9\\u00ff'", "'\\ud83d\\ude00'", "new String('abc')", "String.prototype"],
        &[
            "charAt", "charCodeAt", "concat", "indexOf", "lastIndexOf", "localeCompare", "match",
            "replace", "search", "slice", "split", "substring", "substr", "toLowerCase",
            "toUpperCase", "toLocaleLowerCase", "toLocaleUpperCase", "trim", "toString",
            "valueOf",
        ],
    );
    run_cross(&["String"], &["fromCharCode"]);
    // replace with a function replacer and $-patterns
    let scripts = [
        "'abcabc'.replace('b','X')",
        "'abcabc'.replace(/b/,'X')",
        "'abcabc'.replace(/b/g,'X')",
        "'abcabc'.replace(/(b)/g,'[$1]')",
        "'abcabc'.replace(/(b)/g,'$$')",
        "'abcabc'.replace(/(b)/g,'$&')",
        "'abcabc'.replace(/(b)/g,'$`')",
        "'abcabc'.replace(/(b)/g,\"$'\")",
        "'abcabc'.replace(/(b)/g,'$2')",
        "'abcabc'.replace(/(b)/g,'$0')",
        "'abcabc'.replace(/(b)/g,'$99')",
        "'abcabc'.replace(/b/g,function(m,o,s){return '<'+m+o+s.length+'>'})",
        "'abcabc'.replace(/(b)(c)/g,function(m,p1,p2,o,s){return p2+p1})",
        "'aaa'.replace(/a*/g,'X')",
        "''.replace(/^/,'X')",
        "'abc'.split('')",
        "JSON.stringify('abc'.split(''))",
        "JSON.stringify('a1b2c'.split(/\\d/))",
        "JSON.stringify('a1b2c'.split(/(\\d)/))",
        "JSON.stringify('abc'.split('',2))",
        "JSON.stringify('abc'.split(undefined))",
        "JSON.stringify('abc'.split(/x/))",
        "JSON.stringify(''.split(''))",
        "JSON.stringify(''.split('x'))",
        "JSON.stringify('abc'.match(/(b)/))",
        "JSON.stringify('abcabc'.match(/b/g))",
        "JSON.stringify('abc'.match(/x/g))",
        "'\\u00e9'.toUpperCase()",
        "'\\u00df'.toUpperCase()",
        "'I\\u0307'.toLowerCase()",
        "'\\u1e9e'.toLowerCase()",
        "'abc'.charCodeAt(-1)",
        "'abc'.charCodeAt(99)",
        "'\\ud83d\\ude00'.length",
        "'\\ud83d\\ude00'.charCodeAt(0)",
        "'abc'.substring(2,0)",
        "'abc'.substr(-2)",
        "'abc'.slice(-2,-1)",
        "'  x  '.trim()+'|'",
        "String.fromCharCode(0x10000)",
        "String.fromCharCode(65.9)",
        "String.fromCharCode(-1)",
    ];
    for s in scripts {
        diff_eval_both_modes(s);
    }
}

#[test]
fn row71_number_builtins() {
    let mut scripts: Vec<String> = Vec::new();
    let values = [
        "0", "-0", "1", "-1", "1.5", "-1.5", "0.1", "255", "1e21", "1e-7", "1e100", "1e-100",
        "NaN", "Infinity", "-Infinity", "123456789", "0.000001", "1e-6", "9007199254740993",
        "1.7976931348623157e308", "5e-324",
    ];
    for v in values {
        scripts.push(format!("({v}).toString()"));
        for radix in 2..=36 {
            scripts.push(format!("({v}).toString({radix})"));
        }
        for bad in ["0", "1", "37", "-1", "1.5", "NaN", "undefined", "null", "'16'"] {
            scripts.push(format!("try{{({v}).toString({bad})}}catch(e){{'E:'+e}}"));
        }
        for d in [0, 1, 2, 5, 20, 21, 100, -1] {
            scripts.push(format!("try{{({v}).toFixed({d})}}catch(e){{'E:'+e}}"));
            scripts.push(format!("try{{({v}).toExponential({d})}}catch(e){{'E:'+e}}"));
            scripts.push(format!("try{{({v}).toPrecision({d})}}catch(e){{'E:'+e}}"));
        }
        scripts.push(format!("({v}).toFixed()"));
        scripts.push(format!("({v}).toExponential()"));
        scripts.push(format!("({v}).toPrecision()"));
        scripts.push(format!("({v}).valueOf()"));
        scripts.push(format!("({v}).toLocaleString()"));
        scripts.push(format!("String({v})"));
        scripts.push(format!("Number({v})"));
        scripts.push(format!("({v})|0"));
        scripts.push(format!("({v})>>>0"));
    }
    for k in [
        "Number.MAX_VALUE", "Number.MIN_VALUE", "Number.NaN", "Number.POSITIVE_INFINITY",
        "Number.NEGATIVE_INFINITY",
    ] {
        scripts.push(format!("String({k})"));
    }
    for s in scripts {
        diff_eval_both_modes(&s);
    }
}

#[test]
fn row72_math_builtins() {
    let fns = [
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "round", "sin",
        "sqrt", "tan",
    ];
    let two = ["atan2", "pow"];
    let varargs = ["max", "min"];
    let mut rng = Rng::new(0x7272);
    let mut vals: Vec<String> = vec![
        "0".into(), "-0".into(), "1".into(), "-1".into(), "0.5".into(), "-0.5".into(),
        "NaN".into(), "Infinity".into(), "-Infinity".into(), "1e308".into(), "1e-308".into(),
        "2".into(), "-2".into(), "0.49999999999999994".into(), "-0.5".into(), "1.5".into(),
        "2.5".into(), "-1.5".into(), "-2.5".into(), "1e21".into(),
    ];
    for _ in 0..400 {
        let d = rng.nice_f64();
        if d.is_finite() {
            vals.push(format!("{d:?}"));
        }
    }
    let mut scripts: Vec<String> = Vec::new();
    for f in fns {
        for v in &vals {
            scripts.push(format!("String(Math.{f}({v}))"));
        }
        scripts.push(format!("String(Math.{f}())"));
    }
    for f in two {
        for _ in 0..300 {
            let a = &vals[rng.below(vals.len() as u32) as usize];
            let b = &vals[rng.below(vals.len() as u32) as usize];
            scripts.push(format!("String(Math.{f}({a},{b}))"));
        }
        scripts.push(format!("String(Math.{f}())"));
        scripts.push(format!("String(Math.{f}(1))"));
    }
    for f in varargs {
        scripts.push(format!("String(Math.{f}())"));
        for _ in 0..200 {
            let n = rng.below(4) + 1;
            let a: Vec<String> = (0..n)
                .map(|_| vals[rng.below(vals.len() as u32) as usize].clone())
                .collect();
            scripts.push(format!("String(Math.{f}({}))", a.join(",")));
        }
    }
    for k in ["E", "LN10", "LN2", "LOG2E", "LOG10E", "PI", "SQRT1_2", "SQRT2"] {
        scripts.push(format!("String(Math.{k})"));
    }
    // Math.random must exist but is nondeterministic: only check the range
    scripts.push("var r=Math.random(); (r>=0&&r<1)".into());
    scripts.push("String(Object.prototype.toString.call(Math))".into());
    for s in scripts {
        diff_eval_both_modes(&s);
    }
}

#[test]
fn row73_date_builtins() {
    // Only UTC-based accessors and explicit timestamps, so the result does not
    // depend on the ambient timezone or the current time.
    let ts = [
        "0", "1", "-1", "86400000", "-86400000", "1e12", "-1e12", "NaN", "8.64e15", "8.64e15+1",
        "-8.64e15", "1234567890123", "946684800000", "2147483647000",
    ];
    let getters = [
        "getTime", "valueOf", "getUTCFullYear", "getUTCMonth", "getUTCDate", "getUTCDay",
        "getUTCHours", "getUTCMinutes", "getUTCSeconds", "getUTCMilliseconds",
        "getTimezoneOffset", "toUTCString", "toISOString", "toJSON",
    ];
    let mut scripts: Vec<String> = Vec::new();
    for t in ts {
        for g in getters {
            scripts.push(format!("try{{String(new Date({t}).{g}())}}catch(e){{'E:'+e}}"));
        }
        scripts.push(format!("String(new Date({t}).getTime())"));
    }
    for args in [
        "2000,0,1", "2000,0,1,0,0,0,0", "1999,11,31,23,59,59,999", "2000,13,1", "2000,0,32",
        "0,0,1", "99,0,1", "NaN,0,1",
    ] {
        scripts.push(format!("String(Date.UTC({args}))"));
    }
    for s in [
        "'2000-01-01T00:00:00Z'",
        "'2000-01-01T00:00:00.000Z'",
        "'2000-01-01'",
        "'1970-01-01T00:00:00Z'",
        "'not a date'",
        "''",
        "'2000-01-01T00:00:00+01:00'",
    ] {
        scripts.push(format!("String(Date.parse({s}))"));
        scripts.push(format!("String(new Date({s}).getTime())"));
    }
    scripts.push("String(new Date(0).setTime(1000))".into());
    scripts.push("var d=new Date(0); d.setUTCFullYear(2000); String(d.getTime())".into());
    scripts.push("var d=new Date(0); d.setUTCMonth(5); String(d.getTime())".into());
    scripts.push("var d=new Date(0); d.setUTCDate(15); String(d.getTime())".into());
    scripts.push("var d=new Date(0); d.setUTCHours(5); String(d.getTime())".into());
    scripts.push("var d=new Date(0); d.setUTCMinutes(5); String(d.getTime())".into());
    scripts.push("var d=new Date(0); d.setUTCSeconds(5); String(d.getTime())".into());
    scripts.push("var d=new Date(0); d.setUTCMilliseconds(5); String(d.getTime())".into());
    scripts.push("String(Object.prototype.toString.call(new Date(0)))".into());
    scripts.push("try{Date.prototype.getTime.call({})}catch(e){'E:'+e}".into());
    scripts.push("try{new Date(0).toISOString.call(1)}catch(e){'E:'+e}".into());
    scripts.push("try{String(new Date(NaN).toISOString())}catch(e){'E:'+e}".into());
    for s in scripts {
        diff_eval_both_modes(&s);
    }
}

#[test]
fn row74_global_functions() {
    let mut scripts: Vec<String> = Vec::new();
    let strings = [
        "''", "' '", "'0'", "'1'", "'-1'", "'+1'", "'0x10'", "'0X10'", "'010'", "'1e3'",
        "'1.5'", "'.5'", "'Infinity'", "'-Infinity'", "'NaN'", "'abc'", "'12abc'", "'  42  '",
        "'z'", "'ZZ'", "'%41'", "'%'", "'%4'", "'%zz'", "'%C3%A9'", "'%E0%A4%A'",
        "'a b'", "'a+b'", "'\\u00e9'", "'\\ud83d\\ude00'", "'\\ud800'", "'\\udc00'",
        "';/?:@&=+$,#'", "'-_.!~*()'", "'%u0041'", "'%uZZZZ'", "undefined", "null", "1", "[1,2]",
    ];
    for s in strings {
        scripts.push(format!("String(parseFloat({s}))"));
        for r in ["", ",0", ",2", ",8", ",10", ",16", ",36", ",1", ",37", ",-1", ",1.5", ",NaN"] {
            scripts.push(format!("String(parseInt({s}{r}))"));
        }
        scripts.push(format!("String(isNaN({s}))"));
        scripts.push(format!("String(isFinite({s}))"));
        for f in [
            "encodeURI", "encodeURIComponent", "decodeURI", "decodeURIComponent", "escape",
            "unescape", "String", "Number", "Boolean",
        ] {
            scripts.push(format!("try{{String({f}({s}))}}catch(e){{'E:'+e}}"));
        }
    }
    scripts.push("typeof eval".into());
    scripts.push("String(eval('1+1'))".into());
    scripts.push("try{eval('~')}catch(e){'E:'+e}".into());
    scripts.push("String(eval(1))".into());
    scripts.push("String(eval())".into());
    scripts.push("String(undefined)+String(NaN)+String(Infinity)".into());
    for s in scripts {
        diff_eval_both_modes(&s);
    }
}

#[test]
fn row67_68_json() {
    let mut scripts: Vec<String> = Vec::new();
    let texts = [
        "'1'", "'-1'", "'1.5'", "'1e3'", "'-1e-3'", "'0'", "'-0'", "'\"\"'", "'\"a\"'",
        "'\"\\\\u0041\"'", "'\"\\\\n\"'", "'true'", "'false'", "'null'", "'[]'", "'[1,2,3]'",
        "'[[[]]]'", "'{}'", "'{\"a\":1}'", "'{\"a\":{\"b\":[1,2]}}'", "'  1  '", "''",
        "'01'", "'+1'", "'.5'", "'5.'", "'1e'", "'nul'", "'[1,]'", "'{\"a\":1,}'", "'{a:1}'",
        "\"'a'\"", "'[1 2]'", "'\\\"\\\\u00e9\\\"'", "'{\"__proto__\":1}'",
        "'[1,2,3,4,5,6,7,8,9,10]'", "'{\"a\":1,\"b\":2,\"c\":3}'",
    ];
    for t in texts {
        scripts.push(format!("try{{JSON.stringify(JSON.parse({t}))}}catch(e){{'E:'+e}}"));
        scripts.push(format!(
            "try{{JSON.stringify(JSON.parse({t},function(k,v){{return v}}))}}catch(e){{'E:'+e}}"
        ));
        scripts.push(format!(
            "try{{JSON.stringify(JSON.parse({t},function(k,v){{return typeof v==='number'?v*2:v}}))}}catch(e){{'E:'+e}}"
        ));
        scripts.push(format!(
            "try{{JSON.stringify(JSON.parse({t},function(k,v){{return undefined}}))}}catch(e){{'E:'+e}}"
        ));
    }
    let values = [
        "undefined", "null", "true", "1", "NaN", "Infinity", "'a'", "[]", "[1,'a',null,true]",
        "[undefined,function(){},1]", "({})", "({a:1,b:'x'})", "({a:undefined})",
        "({a:function(){}})", "({toJSON:function(){return 42}})",
        "({a:{toJSON:function(){return [1]}}})", "new Date(0)", "new Number(1)",
        "new String('s')", "new Boolean(true)", "/re/", "({a:[{b:[{c:1}]}]})",
        "(function(){var o={};o.self=o;return o})()",
        "(function(){var a=[];a[0]=a;return a})()",
        "({'\\u00e9':'\\u00ff'})", "({'\\u0001':'\\u001f'})",
        "Object.create(null)",
    ];
    let indents = ["", ",null,0", ",null,1", ",null,2", ",null,10", ",null,11", ",null,-1", ",null,'..'", ",null,'0123456789ab'", ",null,'\\t'", ",null,{}"];
    for v in values {
        for ind in indents {
            scripts.push(format!("try{{String(JSON.stringify({v}{ind}))}}catch(e){{'E:'+e}}"));
        }
        scripts.push(format!(
            "try{{String(JSON.stringify({v},function(k,v){{return v}}))}}catch(e){{'E:'+e}}"
        ));
        scripts.push(format!(
            "try{{String(JSON.stringify({v},function(k,v){{return typeof v==='number'?undefined:v}}))}}catch(e){{'E:'+e}}"
        ));
        scripts.push(format!(
            "try{{String(JSON.stringify({v},['a','b']))}}catch(e){{'E:'+e}}"
        ));
        scripts.push(format!(
            "try{{String(JSON.stringify({v},[]))}}catch(e){{'E:'+e}}"
        ));
        scripts.push(format!(
            "try{{String(JSON.stringify({v},1))}}catch(e){{'E:'+e}}"
        ));
    }
    for s in scripts {
        diff_eval_both_modes(&s);
    }
}

#[test]
fn row64_65_66_type_cross_products() {
    let values = [
        "undefined", "null", "true", "false", "0", "-0", "1", "NaN", "Infinity", "''", "'0'",
        "'a'", "[]", "[0]", "[1,2]", "({})", "({valueOf:function(){return 1}})",
        "({toString:function(){return '1'}})", "(function(){})", "/r/", "new Date(0)",
        "new Number(1)", "new String('1')", "new Boolean(false)", "Math", "JSON",
        "new Error('e')", "Object.create(null)",
    ];
    let ops = ["+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "===", "!==", "&", "|", "^", "<<", ">>", ">>>", "in", "instanceof"];
    for a in values {
        for b in values {
            for op in ops {
                diff_eval(
                    &format!("try{{String(({a}) {op} ({b}))}}catch(e){{'E:'+String(e)}}"),
                    0,
                );
            }
            diff_eval(&format!("try{{String([{a},{b}].join('|'))}}catch(e){{'E:'+String(e)}}"), 0);
        }
        for un in ["typeof ", "!", "-", "+", "~", "void "] {
            diff_eval(&format!("try{{String({un}({a}))}}catch(e){{'E:'+String(e)}}"), 0);
        }
        diff_eval(&format!("try{{String(Object.prototype.toString.call({a}))}}catch(e){{'E:'+String(e)}}"), 0);
        diff_eval(&format!("try{{String(JSON.stringify({a}))}}catch(e){{'E:'+String(e)}}"), 0);
    }
}

#[test]
fn row63_refs_and_gc_interaction() {
    let scripts = [
        "var a=[]; for(var i=0;i<100;++i){a.push({v:i})} a.length",
        "(function(){ var o={}; o.self=o; return 'ok' })()",
        "var f=function(){return 1}; f=null; 'ok'",
        "var s=''; for(var i=0;i<200;++i){s+=String(i)} s.length",
        "var o={}; for(var i=0;i<200;++i){o['k'+i]={n:i}} Object.keys(o).length",
    ];
    for s in scripts {
        diff_eval_both_modes(s);
    }
}

#[test]
fn row66_repr_shapes() {
    let scripts = [
        "1", "'a'", "'\\u00e9'", "'\\n'", "'\\\\'", "'\\''", "'\"'", "null", "undefined", "true",
        "[]", "[1,2]", "[[1],[2]]", "[undefined]", "[,]", "({})", "({a:1})", "({'a b':1})",
        "({0:1})", "(function f(){})", "(function(){})", "/re/gim", "new Date(0)",
        "new Number(1)", "new String('s')", "new Boolean(true)", "new Error('m')",
        "Math", "JSON", "Object.create(null)",
        "(function(){var o={};o.self=o;return o})()",
        "(function(){var a=[];a.push(a);return a})()",
        "({a:{b:{c:{d:1}}}})",
    ];
    let p = pair();
    for s in scripts {
        // js_torepr / js_tryrepr / js_repr
        diff_protected(&format!("repr {s}"), 0, || {
            let s = s.to_string();
            move |api: &Api, j: JS| unsafe {
                let fname = cs("[string]");
                let src = cs(&format!("({s})"));
                if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) != 0 {
                    log("compile-failed");
                    return;
                }
                (api.js_pushundefined)(j);
                if (api.js_pcall)(j, 0) != 0 {
                    log("threw");
                    return;
                }
                log(format!("torepr={:?}", rstr((api.js_torepr)(j, -1))));
                let errs = cs("<throw>");
                log(format!(
                    "tryrepr={:?}",
                    rstr((api.js_tryrepr)(j, -1, errs.as_ptr()))
                ));
                (api.js_repr)(j, -1);
                log(format!("repr-push={}", describe(api, j, -1)));
            }
        });
        let _ = p;
    }
}

#[test]
fn row61_function_builtins() {
    let scripts = [
        "(function(a,b){return a+b}).length",
        "(function(){}).name",
        "(function f(){}).name",
        "String(function f(a,b){return a})",
        "(function(){return 1}).call(null)",
        "(function(){return this}).call(1)",
        "'use strict'; (function(){return this}).call(1)",
        "(function(a,b){return a+b}).apply(null,[1,2])",
        "(function(a,b){return a+b}).apply(null,{length:2,0:1,1:2})",
        "try{(function(){}).apply(null,1)}catch(e){'E:'+e}",
        "try{(function(){}).apply(null,'ab')}catch(e){'E:'+e}",
        "var b=(function(a,b){return a+b}).bind(null,1); b(2)",
        "var b=(function(){return this.x}).bind({x:7}); b()",
        "var B=(function(a){this.a=a}).bind(null,3); (new B()).a",
        "(function(a,b){}).bind(null,1).length",
        "try{Function.prototype.call.call(1)}catch(e){'E:'+e}",
        "try{(function(){}).call.apply(1)}catch(e){'E:'+e}",
        "Function.prototype.toString.call(Math.max)",
        "try{Function.prototype.toString.call({})}catch(e){'E:'+e}",
        "new Function('return 1')()",
        "new Function('a,b','return a+b')(1,2)",
        "new Function('a','b','return a+b')(1,2)",
        "try{new Function('~')}catch(e){'E:'+e}",
        "try{new Function('a~','1')}catch(e){'E:'+e}",
        "Function.length",
        "typeof Function.prototype",
        "Function.prototype()",
        "(function(){ return arguments.length })(1,2,3)",
        "(function(){ 'use strict'; return typeof arguments.callee })()",
        "(function(){ return typeof arguments.callee })()",
    ];
    for s in scripts {
        diff_eval_both_modes(s);
    }
}

#[test]
fn error_builtins() {
    let kinds = ["Error", "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError"];
    let mut scripts: Vec<String> = Vec::new();
    for k in kinds {
        for arg in ["", "'m'", "undefined", "null", "1", "{}", "[1]"] {
            scripts.push(format!("try{{String(new {k}({arg}))}}catch(e){{'E:'+e}}"));
            scripts.push(format!("try{{String({k}({arg}))}}catch(e){{'E:'+e}}"));
            scripts.push(format!("try{{new {k}({arg}).message}}catch(e){{'E:'+e}}"));
            scripts.push(format!("try{{new {k}({arg}).name}}catch(e){{'E:'+e}}"));
            scripts.push(format!("try{{new {k}({arg}).stackTrace}}catch(e){{'E:'+e}}"));
        }
        scripts.push(format!("({k}.prototype instanceof Error)"));
        scripts.push(format!("(new {k}('m') instanceof {k})"));
        scripts.push(format!("Object.prototype.toString.call(new {k}('m'))"));
    }
    scripts.push("var e=new Error('m'); e.name='X'; String(e)".into());
    scripts.push("var e=new Error('m'); delete e.message; String(e)".into());
    scripts.push("Error.prototype.toString.call({})".into());
    scripts.push("try{Error.prototype.toString.call(1)}catch(e){'E:'+e}".into());
    for s in scripts {
        diff_eval_both_modes(&s);
    }
}

#[test]
fn boolean_builtins() {
    let scripts = [
        "Boolean(0)", "Boolean('')", "Boolean('0')", "Boolean([])", "Boolean({})",
        "new Boolean(false).valueOf()", "String(new Boolean(false))",
        "Boolean.prototype.toString.call(true)",
        "try{Boolean.prototype.toString.call(1)}catch(e){'E:'+e}",
        "try{Boolean.prototype.valueOf.call('x')}catch(e){'E:'+e}",
        "Object.prototype.toString.call(new Boolean(true))",
    ];
    for s in scripts {
        diff_eval_both_modes(s);
    }
}

/// The proleptic Gregorian leap-year rule (`y%4==0 && (y%100!=0 || y%400==0)`)
/// only shows up for specific years, so sweep them explicitly.
#[test]
fn date_year_sweep() {
    let mut srcs: Vec<String> = Vec::new();
    let mut years: Vec<i64> = Vec::new();
    for y in 1580..1620 { years.push(y); }
    for y in 1890..1910 { years.push(y); }
    for y in 1990..2010 { years.push(y); }
    for y in [
        0, 1, 4, 100, 200, 300, 400, 500, 800, 900, 1000, 1100, 1200, 1500, 1600, 1700, 1800,
        1900, 2000, 2004, 2100, 2200, 2300, 2400, 2500, 2800, 3000, 4000, 4400, 5000,
        -1, -4, -100, -400, -500, -2000,
    ] {
        years.push(y);
    }
    for y in years {
        for (m, d) in [(0, 1), (1, 28), (1, 29), (1, 30), (2, 1), (11, 31), (12, 1)] {
            srcs.push(format!("String(Date.UTC({y},{m},{d}))"));
            srcs.push(format!(
                "String(new Date(Date.UTC({y},{m},{d})).getUTCFullYear())+','+                 String(new Date(Date.UTC({y},{m},{d})).getUTCMonth())+','+                 String(new Date(Date.UTC({y},{m},{d})).getUTCDate())+','+                 String(new Date(Date.UTC({y},{m},{d})).getUTCDay())"
            ));
        }
        srcs.push(format!("try{{new Date(Date.UTC({y},0,1)).toISOString()}}catch(e){{'E:'+e}}"));
        srcs.push(format!("String(new Date(Date.UTC({y},0,1)).toUTCString())"));
    }
    // millisecond timestamps that straddle day/year boundaries
    for t in [
        0i64, 1, -1, 86399999, 86400000, 86400001, -86400000, -86400001,
        951782400000, 951868800000, 4102444800000, 13569465600000, -62135596800000,
        8640000000000000, -8640000000000000, 8640000000000001, -8640000000000001,
    ] {
        srcs.push(format!(
            "var d=new Date({t}); String(d.getTime())+'|'+String(d.getUTCFullYear())+'|'+             String(d.getUTCMonth())+'|'+String(d.getUTCDate())+'|'+String(d.getUTCDay())+'|'+             String(d.getUTCHours())+'|'+String(d.getUTCMinutes())+'|'+String(d.getUTCSeconds())+             '|'+String(d.getUTCMilliseconds())+'|'+String(d.toUTCString())"
        ));
        srcs.push(format!("try{{new Date({t}).toISOString()}}catch(e){{'E:'+e}}"));
    }
    // setter round-trips over many years
    for y in [1600i64, 1700, 1900, 2000, 2100, 2400, 2500] {
        srcs.push(format!(
            "var d=new Date(0); d.setUTCFullYear({y}); d.setUTCMonth(1); d.setUTCDate(29);              String(d.getTime())+'|'+String(d.getUTCMonth())+'|'+String(d.getUTCDate())"
        ));
    }
    diff_eval_batch("date year sweep", &srcs, 0);
}
