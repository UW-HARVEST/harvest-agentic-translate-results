//! Phase B differential tests driven by GENERATED / randomized JavaScript, with
//! fixed RNG seeds for reproducibility. These complement the fixed corpora in
//! `b06_scripts.rs` by covering cross-products (every operator over every value
//! shape, every regexp flag combination over every subject, ...) that a
//! hand-written corpus cannot reach.
mod common;
use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Randomized / generated script tests
// ---------------------------------------------------------------------------

#[test]
fn randomized_arithmetic_and_comparison() {
    // Property test: every binary operator over every value-shape pair. This is
    // the cross-product the per-function tests cannot reach.
    let operands: &[&str] = &[
        "0", "-0", "1", "-1", "0.5", "NaN", "Infinity", "-Infinity", "2147483647", "2147483648",
        "-2147483648", "4294967296", "9007199254740993", "1e21", "1e-7", "''", "'0'", "'1'", "'a'",
        "'  12  '", "'0x10'", "true", "false", "null", "undefined", "[]", "[1]", "[1,2]", "({})",
        "({valueOf:function(){return 3}})", "({toString:function(){return '7'}})", "/re/",
        "function(){}", "new Date(0)", "new Number(2)", "new String('3')", "new Boolean(false)",
    ];
    let ops: &[&str] = &[
        "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "===", "!==", "&", "|", "^",
        "<<", ">>", ">>>", "&&", "||",
    ];
    let mut b = Batch::new();
    for op in ops {
        for a in operands {
            for c in operands {
                b.script(0, &format!("String(({a}) {op} ({c}))"));
            }
        }
    }
    b.finish("binary operators over all value shapes");
}

#[test]
fn randomized_unary_and_typeof() {
    let operands: &[&str] = &[
        "0", "-0", "1", "-1", "0.5", "NaN", "Infinity", "-Infinity", "2147483648", "-2147483649",
        "4294967295", "''", "'a'", "'12'", "true", "false", "null", "undefined", "[]", "[1,2]",
        "({})", "/re/", "function(){}", "new Date(0)", "new Number(2)", "new String('3')",
    ];
    let unops: &[&str] = &["-", "+", "!", "~", "typeof ", "void "];
    let mut b = Batch::new();
    for op in unops {
        for a in operands {
            b.script(0, &format!("String({op}({a}))"));
        }
    }
    // increment/decrement need an lvalue
    for a in operands {
        for form in ["var x = ({}); String(++x)", "var x = ({}); String(x++)",
                     "var x = ({}); String(--x)", "var x = ({}); String(x--)"] {
            b.script(0, &form.replacen("({})", a, 1));
        }
    }
    b.finish("unary operators over all value shapes");
}

#[test]
fn randomized_number_formatting_roundtrips() {
    // Property test with a fixed seed: format random doubles every way the
    // engine can, so grisu2/fmtexp/itoa are exercised through the real pipeline.
    let mut b = Batch::new();
    let mut rng = Rng::new(0x5C21_9001);
    for _ in 0..1200 {
        let v = if rng.bool() { rng.finite_f64() } else { rng.any_f64() };
        let lit = format!("{:e}", v);
        b.script(0, &format!("String({lit})"));
        b.script(0, &format!("({lit}).toString()"));
        b.script(0, &format!("String(-({lit}))"));
        b.script(0, &format!("JSON.stringify({lit})"));
        let radix = 2 + rng.below(35);
        b.script(0, &format!("({lit}).toString({radix})"));
        let digits = rng.below(21);
        b.script(0, &format!("({lit}).toFixed({digits})"));
        b.script(0, &format!("({lit}).toExponential({})", rng.below(21)));
        b.script(0, &format!("({lit}).toPrecision({})", 1 + rng.below(21)));
        b.script(0, &format!("parseFloat('{}')", v));
        b.script(0, &format!("parseInt('{}', {})", v, rng.below(40)));
        b.script(0, &format!("Number('{}')", v));
    }
    b.finish("randomized number formatting");
}

#[test]
fn randomized_string_operations() {
    // Property test: string methods over randomized (incl. non-ASCII and
    // malformed-UTF-8) inputs and random index arguments.
    let mut b = Batch::new();
    let mut rng = Rng::new(0x5C21_9002);
    let pieces: &[&str] = &[
        "a", "b", "Z", "0", " ", "\\t", "\\u00e9", "\\u4f60", "\\ud83d\\ude00", "\\u0000",
        "\\uffff", "-", "_", "\\\\", "'", "\\\"",
    ];
    for _ in 0..900 {
        let n = rng.below(8) as usize;
        let s: String = (0..n).map(|_| *rng.pick(pieces)).collect();
        let lit = format!("'{s}'");
        let i = rng.range_i64(-8, 12);
        let jj = rng.range_i64(-8, 12);
        for expr in [
            format!("{lit}.length"),
            format!("{lit}.charAt({i})"),
            format!("{lit}.charCodeAt({i})"),
            format!("{lit}.indexOf('a')"),
            format!("{lit}.lastIndexOf('a')"),
            format!("{lit}.slice({i},{jj})"),
            format!("{lit}.substring({i},{jj})"),
            format!("{lit}.substr({i},{jj})"),
            format!("{lit}.toUpperCase()"),
            format!("{lit}.toLowerCase()"),
            format!("{lit}.split('').length"),
            format!("{lit}.split('a').join('|')"),
            format!("{lit}.concat('x','y')"),
            format!("{lit}.replace('a','A')"),
            format!("{lit}.replace(/a/g,'A')"),
            format!("{lit}.match(/./g) ? {lit}.match(/./g).length : 'null'"),
            format!("{lit}.search(/a/)"),
            format!("{lit}.trim().length"),
            format!("{lit} < 'm'"),
            format!("{lit}.localeCompare('m')"),
            format!("escape({lit})"),
            format!("encodeURIComponent({lit})"),
            format!("JSON.stringify({lit})"),
            format!("String.fromCharCode({}, {})", rng.below(0x11000), rng.below(0x11000)),
        ] {
            b.script(0, &format!("String({expr})"));
        }
    }
    b.finish("randomized string operations");
}

#[test]
fn randomized_array_operations() {
    let mut b = Batch::new();
    let mut rng = Rng::new(0x5C21_9003);
    let elems: &[&str] = &[
        "0", "1", "-1", "2.5", "NaN", "'a'", "''", "true", "false", "null", "undefined", "[]",
        "[1]", "({})",
    ];
    for _ in 0..900 {
        let n = rng.below(7) as usize;
        let arr: Vec<&str> = (0..n).map(|_| *rng.pick(elems)).collect();
        let lit = format!("[{}]", arr.join(","));
        let i = rng.range_i64(-6, 9);
        let jj = rng.range_i64(-6, 9);
        for expr in [
            format!("{lit}.length"),
            format!("{lit}.join('|')"),
            format!("{lit}.slice({i},{jj}).join('|')"),
            format!("(function(){{var a={lit}; a.splice({i},{jj}); return a.join('|')}})()"),
            format!("{lit}.concat([9,8]).join('|')"),
            format!("{lit}.indexOf(1)"),
            format!("{lit}.lastIndexOf(1)"),
            format!("(function(){{var a={lit}; a.reverse(); return a.join('|')}})()"),
            format!("(function(){{var a={lit}; a.sort(); return a.join('|')}})()"),
            format!("(function(){{var a={lit}; return a.pop()+'/'+a.join('|')}})()"),
            format!("(function(){{var a={lit}; return a.shift()+'/'+a.join('|')}})()"),
            format!("(function(){{var a={lit}; a.push(7); return a.join('|')}})()"),
            format!("(function(){{var a={lit}; a.unshift(7); return a.join('|')}})()"),
            format!("{lit}.map(function(x){{return typeof x}}).join('|')"),
            format!("{lit}.filter(function(x){{return !!x}}).join('|')"),
            format!("{lit}.every(function(x){{return !!x}})"),
            format!("{lit}.some(function(x){{return !!x}})"),
            format!("{lit}.reduce(function(a,b){{return String(a)+String(b)}}, 'S')"),
            format!("{lit}.reduceRight(function(a,b){{return String(a)+String(b)}}, 'S')"),
            format!("JSON.stringify({lit})"),
            format!("{lit}.toString()"),
            format!("Object.keys({lit}).join('|')"),
            format!("(function(){{var k=[]; for(var p in {lit}) k.push(p); return k.join('|')}})()"),
        ] {
            b.script(0, &format!("String({expr})"));
        }
    }
    b.finish("randomized array operations");
}

#[test]
fn randomized_regexp_through_js_api() {
    // CONFIGS part 2 section B: the JS-level flag combinations and the
    // lastIndex/global bookkeeping in js_RegExp_prototype_exec.
    let mut b = Batch::new();
    let mut rng = Rng::new(0x5C21_9004);
    let pats: &[&str] = &[
        "a", "a+", "a*", "(a)(b)", "[a-c]+", "^a", "a$", ".", "\\\\d+", "\\\\w", "\\\\s",
        "(?:ab)+", "a|b", "(a)\\\\1", "a{2,3}", "a+?", "[^a]", "(?=a)", "(?!a)",
    ];
    let flagsets: &[&str] = &["", "g", "i", "m", "gi", "gm", "im", "gim"];
    let subjects: &[&str] = &[
        "", "a", "A", "ab", "aab", "AAB", "abcabc", "a\\nb", "\\nab", "xyz", "aaaa", "12 34",
        " a b ", "\\u00e9a",
    ];
    for pat in pats {
        for fl in flagsets {
            for subj in subjects {
                let re = format!("/{pat}/{fl}");
                for expr in [
                    format!("String({re}.test('{subj}'))"),
                    format!("String({re}.exec('{subj}'))"),
                    format!("String('{subj}'.match({re}))"),
                    format!("String('{subj}'.replace({re}, 'X'))"),
                    format!("String('{subj}'.replace({re}, function(m){{return '['+m+']'}}))"),
                    format!("String('{subj}'.split({re}))"),
                    format!("String('{subj}'.search({re}))"),
                    format!("String({re}.source)+'/'+{re}.global+{re}.ignoreCase+{re}.multiline"),
                    // lastIndex bookkeeping across repeated exec calls
                    format!(
                        "(function(){{var r={re},o=[],m; for(var i=0;i<5;i++){{m=r.exec('{subj}'); o.push(m?m.index+':'+m[0]:'null'); o.push(r.lastIndex)}} return o.join(',')}})()"
                    ),
                    format!("(function(){{var r={re}; r.lastIndex={}; var m=r.exec('{subj}'); return (m?m[0]:'null')+'/'+r.lastIndex}})()", rng.range_i64(-2, 8)),
                ] {
                    b.script(0, &expr);
                }
            }
        }
    }
    b.finish("regexp via the JS API");
}

#[test]
fn deeply_nested_and_large_programs() {
    // Shapes that stress the parser/compiler limits (JS_ASTLIMIT, jump patching,
    // local-variable counts) on the valid side of each boundary.
    let mut b = Batch::new();
    for n in [1usize, 10, 50, 100, 200, 390, 399, 400, 401, 500] {
        // nested parentheses -> AST depth
        b.script(0, &format!("{}1{}", "(".repeat(n), ")".repeat(n)));
        // nested blocks
        b.script(0, &format!("{}var x=1;{}x", "{".repeat(n), "}".repeat(n)));
        // nested functions
        b.script(
            0,
            &format!("{}return 1;{}", "(function(){".repeat(n.min(120)), "})()".repeat(n.min(120))),
        );
        // long additive chain
        b.script(0, &format!("0{}", "+1".repeat(n)));
        // long array literal
        b.script(0, &format!("[{}].length", vec!["1"; n].join(",")));
        // long object literal
        b.script(
            0,
            &format!(
                "Object.keys({{{}}}).length",
                (0..n).map(|i| format!("k{i}:{i}")).collect::<Vec<_>>().join(",")
            ),
        );
        // many locals
        b.script(
            0,
            &format!(
                "(function(){{{} return 1}})()",
                (0..n).map(|i| format!("var v{i}={i};")).collect::<Vec<_>>().join("")
            ),
        );
        // long if/else chain
        b.script(
            0,
            &format!("var x=0;{} x", (0..n).map(|i| format!("if(x=={i})x={};", i + 1)).collect::<Vec<_>>().join("")),
        );
        // deep ternary
        b.script(0, &format!("{}1{}", "true?".repeat(n.min(200)), ":0".repeat(n.min(200))));
    }
    b.finish("deeply nested / large programs");
}

#[test]
fn closures_scoping_and_control_flow() {
    let mut b = Batch::new();
    let cases: &[&str] = &[
        "(function(){var a=[]; for(var i=0;i<3;i++) a.push(function(){return i}); return a.map(function(f){return f()}).join(',')})()",
        "(function(){var x=1; function g(){x++; return x} return g()+','+g()+','+x})()",
        "(function f(n){return n<=1?1:n*f(n-1)})(6)",
        "(function(){var r=[]; outer: for(var i=0;i<3;i++){ for(var j=0;j<3;j++){ if(j==1) continue outer; r.push(i+'-'+j)}} return r.join(',')})()",
        "(function(){var r=[]; outer: for(var i=0;i<3;i++){ for(var j=0;j<3;j++){ if(j==1) break outer; r.push(i+'-'+j)}} return r.join(',')})()",
        "(function(){var r=''; switch(2){case 1: r+='a'; case 2: r+='b'; case 3: r+='c'; break; default: r+='d'} return r})()",
        "(function(){var r=''; switch('x'){case 1: r+='a'; break; default: r+='d'} return r})()",
        "(function(){try{return 'try'}finally{}})()",
        "(function(){try{throw 1}catch(e){return 'caught'+e}finally{}})()",
        "(function(){var r=''; try{r+='t'; throw 1}catch(e){r+='c'}finally{r+='f'} return r})()",
        "(function(){try{ try{throw 1}finally{ } }catch(e){return 'outer'+e}})()",
        "(function(){var o={a:1,b:2}; var r=[]; for(var k in o) r.push(k+'='+o[k]); return r.sort().join(',')})()",
        "(function(){var i=0,r=''; do{r+=i}while(++i<3); return r})()",
        "(function(){var i=0,r=''; while(i<3){r+=i;i++} return r})()",
        "(function(){var r=''; for(;;){r+='x'; if(r.length>2) break} return r})()",
        "(function(){var a=1; { var a=2 } return a})()",
        "(function(){return typeof hoisted; function hoisted(){}})()",
        "(function(){return typeof v; var v=1})()",
        "(function(){var f=function g(){return typeof g}; return f()})()",
        "(function(){return (function(){return arguments.length})(1,2,3)})()",
        "(function(){return (function(a,b){return a+'/'+b})(1)})()",
        "(function(){return (function(a){arguments[0]=9; return a})(1)})()",
        "(function(){function f(a,b){return arguments.length} return f()+','+f(1)+','+f(1,2,3)})()",
        "(function(){var o={m:function(){return this===o}}; return o.m()})()",
        "(function(){var o={m:function(){return typeof this}}; var m=o.m; return m()})()",
        "(function(){return [].constructor===Array})()",
        "(function(){function C(){this.x=1} var c=new C(); return c.x+','+(c instanceof C)})()",
        "(function(){function C(){return {y:2}} return new C().y})()",
        "(function(){function C(){} C.prototype.p=5; return new C().p})()",
        "(function(){function A(){} function B(){} B.prototype=new A(); return (new B()) instanceof A})()",
        "(function(){var s=0; for(var i=0;i<1000;i++) s+=i; return s})()",
        "(function(){var o={}; for(var i=0;i<200;i++) o['k'+i]=i; return Object.keys(o).length})()",
        "(function(){var a=[]; for(var i=0;i<200;i++) a[i]=i*i; return a[199]})()",
        "with({a:5}){ (function(){})(); }",
        "(function(){var o={a:1}; with(o){ a=2 } return o.a})()",
        "(function(){var r=[]; (function(){ r.push(typeof arguments) })(); return r[0]})()",
        "(function(){ return eval('1+1') })()",
        "(function(){ var x=5; return eval('x+1') })()",
        "(function(){ eval('var y=7'); return typeof y })()",
        "(function(){ return (new Function('a','b','return a+b'))(2,3) })()",
        "(function(){ return Function.prototype.call.call(function(){return this.v}, {v:9}) })()",
        "(function(){ return (function(){return this.v}).apply({v:8}) })()",
        "(function(){ return (function(a,b){return a+b}).apply(null,[1,2]) })()",
        "(function(){ return (function(a,b){return a+b}).bind(null,1)(2) })()",
        "(function(){ var f=function(){return this.v}.bind({v:'b'}); return f() })()",
        "(function(){ return [1,2,3].map(function(x,i,a){return x+i+a.length}).join(',') })()",
        "(function(){ var t=0; [1,2,3].forEach(function(x){t+=x}); return t })()",
        "(function(){ return Object.create({p:1}).p })()",
        "(function(){ var o=Object.create(null); o.a=1; return Object.keys(o).join(',') })()",
        "(function(){ var o={}; Object.defineProperty(o,'a',{get:function(){return 3}}); return o.a })()",
        "(function(){ var o={}; Object.freeze(o); return Object.isFrozen(o) })()",
        "(function(){ var o={a:1}; Object.seal(o); delete o.a; return o.a })()",
        "(function(){ var o=Object.preventExtensions({}); return Object.isExtensible(o) })()",
    ];
    for flags in [0 as c_int, JS_STRICT] {
        for s in cases {
            b.script(flags, s);
        }
    }
    b.finish("closures, scoping, control flow");
}
