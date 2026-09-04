//! Phase B — differential tests for the language core: the lexer, parser,
//! compiler and the bytecode interpreter, plus RegExp through JS syntax and
//! GC / large-data pressure.
//!
//! CONFIGS.md rows 52, 60, 61.

mod common;

use common::*;

const SEED: u64 = 0x1A46_0000_0000_0001;

/// CONFIGS.md row 52: expressions, statements and operators.
#[test]
fn lang_expressions_and_statements() {
    let snippets: &[&str] = &[
        // literals & operators
        "o(1); o(1.5); o(.5); o(1e3); o(1E-3); o(0x1f); o(0X1F); o(017); o(0);",
        "o('a'); o(\"b\"); o('\\n'); o('\\t'); o('\\x41'); o('\\u0041'); o('\\0'); o('\\\\');",
        "o(true); o(false); o(null); o(undefined); o(typeof undefined);",
        "o(1+2); o(1-2); o(1*2); o(1/2); o(1%2); o(-1); o(+1); o(!1); o(~1);",
        "o(1<2); o(1>2); o(1<=2); o(1>=2); o(1==2); o(1!=2); o(1===2); o(1!==2);",
        "o(1&3); o(1|3); o(1^3); o(1<<3); o(16>>2); o(-16>>2); o(-16>>>2);",
        "o(1&&2); o(0&&2); o(1||2); o(0||2); o(null||'x'); o(undefined&&'x');",
        "o(1?2:3); o(0?2:3); o((1,2,3));",
        "var a=1; o(a++); o(a); o(a--); o(a); o(++a); o(a); o(--a); o(a);",
        "var a=1; a+=2; o(a); a-=1; o(a); a*=3; o(a); a/=2; o(a); a%=2; o(a);",
        "var a=6; a&=3; o(a); a|=8; o(a); a^=1; o(a); a<<=2; o(a); a>>=1; o(a); a>>>=1; o(a);",
        "o('a'+1); o(1+'a'); o('3'*2); o('3'-1); o([]+[]); o([]+{}); o({}+[]); o(1+null); o(1+undefined);",
        "o(NaN===NaN); o(NaN!==NaN); o(0===-0); o(1/0); o(-1/0); o(0/0);",
        "o(void 0); o(typeof void 0);",
        // string/number conversion corners
        "o(''+0); o(''+(-0)); o(''+1e21); o(''+1e-7); o(''+0.1); o(''+1/3);",
        // objects
        "var o1 = {a:1,'b':2,3:4,if:5}; oj(o1); o(o1.a); o(o1['b']); o(o1[3]); o(o1.if);",
        "var o1 = {}; o1.x = 1; o(o1.x); delete o1.x; o(o1.x); o('x' in o1);",
        "var o1 = {get p(){return 7}, set p(v){this.q=v}}; o(o1.p); o1.p=3; o(o1.q);",
        "var a=[1,2,3]; o(a[0]); o(a.length); a[5]=6; o(a.length); o(a[3]);",
        "o([1,2,3].length); o([].length); o([,,].length); o([1,].length);",
        // functions & closures
        "function f(a,b){ return a+b } o(f(1,2)); o(f(1)); o(f()); o(f(1,2,3)); o(f.length);",
        "var f = function(a){ return a*2 }; o(f(4)); o(f.name);",
        "var f = function named(a){ return a }; o(f.name); o(typeof named);",
        "function f(){ return arguments.length + ':' + arguments[0] } o(f()); o(f(1)); o(f(1,2));",
        "function mk(n){ return function(){ return ++n } } var c = mk(0); o(c()); o(c()); o(c());",
        "function f(){ return this } o(typeof f()); o(typeof f.call(null)); o(f.call(5).valueOf ? typeof f.call(5) : 'x');",
        "function f(a,b){ return a+b } o(f.apply(null,[1,2])); o(f.call(null,1,2)); o(f.apply(null)); o(f.bind(null,1)(2));",
        "function f(){ return 1 } function f(){ return 2 } o(f());",
        "o(typeof g); function g(){} o(typeof g);",
        "var h; o(typeof h); h = function(){}; o(typeof h);",
        "function f(a,a){ return a } o(f(1,2));",
        // recursion
        "function fib(n){ return n<2?n:fib(n-1)+fib(n-2) } o(fib(20));",
        "function fact(n){ return n<=1?1:n*fact(n-1) } o(fact(20)); o(fact(170)); o(fact(171));",
        // prototypes and constructors
        "function P(x){ this.x=x } P.prototype.get=function(){ return this.x }; var p=new P(5); o(p.get()); o(p instanceof P); o(p.constructor===P);",
        "function A(){} function B(){} B.prototype = new A(); var b=new B(); o(b instanceof A); o(b instanceof B);",
        "o(Object.getPrototypeOf([])===Array.prototype); o(({}).hasOwnProperty('x'));",
        "var o1 = Object.create({inherited:1}); o1.own=2; var s=''; for (var k in o1) s+=k+','; o(s); o(Object.keys(o1).join(','));",
        // control flow
        "var s=''; for (var i=0;i<5;++i) s+=i; o(s);",
        "var s=''; var i=0; while (i<5) { s+=i; ++i } o(s);",
        "var s=''; var i=0; do { s+=i; ++i } while (i<5); o(s);",
        "var s=''; for (var k in {a:1,b:2}) s+=k; o(s);",
        "var s=''; for (var i=0;i<10;++i) { if (i%2) continue; if (i>6) break; s+=i } o(s);",
        "var s=''; outer: for (var i=0;i<3;++i) { for (var j=0;j<3;++j) { if (j==1) continue outer; if (i==2) break outer; s+=''+i+j } } o(s);",
        "switch (2) { case 1: o('one'); case 2: o('two'); case 3: o('three'); break; default: o('def') }",
        "switch (9) { case 1: o('one'); break; default: o('def'); case 2: o('two') }",
        "switch ('a') { case 'a': o(1); break } switch (null) { case null: o(2); break } switch (undefined) { default: o(3) }",
        "var s=''; try { s+='t'; throw 1 } catch (e) { s+='c'+e } finally { s+='f' } o(s);",
        "var s=''; try { try { throw 'x' } finally { s+='f1' } } catch (e) { s+='c'+e } o(s);",
        "function f(){ try { return 1 } finally { return 2 } } o(f());",
        "function f(){ for(;;) { try { break } finally { } } return 'done' } o(f());",
        "var s=''; try { throw {a:1} } catch (e) { s += typeof e + e.a } o(s);",
        "o((function(){ try { throw 1 } catch (e) { return e } })());",
        "var e = 'outer'; try { throw 'inner' } catch (e) { o(e) } o(e);",
        // with (non-strict only)
        "try { with ({a:1}) { o(a) } } catch (e) { o('E:'+e) }",
        // eval
        "o(eval('1+1')); o(eval('\"x\"')); var q=5; o(eval('q')); eval('var w = 9'); o(typeof w);",
        "o(typeof eval); o(eval('')); o(eval('var z=1')); o((function(){ return eval('this') })() === undefined ? 'u' : 'o');",
        "o((function(){ eval('var local=1'); return typeof local })());",
        // getters via defineProperty
        "var o1={}; Object.defineProperty(o1,'a',{value:1,enumerable:true}); oj(o1); oj(Object.getOwnPropertyDescriptor(o1,'a'));",
        "var o1={}; Object.defineProperty(o1,'a',{get:function(){return 2}}); o(o1.a); oj(Object.getOwnPropertyDescriptor(o1,'a')?'desc':'none');",
        "var o1=Object.freeze({a:1}); o1.a=2; o(o1.a); o(Object.isFrozen(o1)); o(Object.isSealed(o1)); o(Object.isExtensible(o1));",
        "var o1=Object.seal({a:1}); o1.b=2; o(o1.b); o1.a=3; o(o1.a); o(Object.isSealed(o1));",
        "var o1={a:1}; Object.preventExtensions(o1); o1.b=2; o(o1.b); o(Object.isExtensible(o1));",
        "oj(Object.getOwnPropertyNames({a:1,b:2})); oj(Object.keys({a:1,b:2}));",
        // hoisting, scoping
        "o(typeof x); var x = 1; o(x);",
        "function f(){ o(typeof y); var y = 1; o(y) } f();",
        "var a=1; function f(){ var a=2; return a } o(f()); o(a);",
        "function f(){ a = 5 } f(); o(a);",
        // this / global
        "o(typeof this); o(this === undefined ? 'u' : 'o');",
        // misc builtins
        "o(typeof Object); o(typeof Array); o(typeof Function); o(typeof String); o(typeof Number); o(typeof Boolean); o(typeof Math); o(typeof JSON); o(typeof Date); o(typeof RegExp); o(typeof Error);",
        "o(Object.prototype.toString.call([])); o(Object.prototype.toString.call({})); o(Object.prototype.toString.call(null)); o(Object.prototype.toString.call(1)); o(Object.prototype.toString.call('s')); o(Object.prototype.toString.call(true)); o(Object.prototype.toString.call(undefined));",
        "o(new Function('a','b','return a+b')(1,2)); o(new Function('return 1')());",
        "o(String(new Error('m'))); o(new Error('m').message); o(new Error('m').name); o(new TypeError('t').name);",
        "o([1,2,3] instanceof Array); o('' instanceof String); o(new String('') instanceof String);",
        "o(isNaN(NaN)); o(isNaN('a')); o(isFinite(1)); o(isFinite(Infinity)); o(isFinite('1'));",
        "o(Number.MAX_VALUE); o(Number.MIN_VALUE); o(Number.NaN); o(Number.POSITIVE_INFINITY); o(Number.NEGATIVE_INFINITY);",
        // getter/setter on prototype chain
        "function P(){}; Object.defineProperty(P.prototype,'v',{get:function(){return 'proto'}}); var p=new P(); o(p.v); p.v='x'; o(p.v);",
        // deeply nested expressions (jsparse AST depth)
        "o(((((((((((1)))))))))));",
        "o(1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1+1);",
    ];
    for flags in [0, 1] {
        for (i, s) in snippets.iter().enumerate() {
            assert_same_program(flags, &format!("lang#{i}"), s);
        }
    }
}

/// CONFIGS.md row 52: lexer edge cases (comments, line terminators, ASI).
#[test]
fn lang_lexer_and_asi() {
    let snippets: &[&str] = &[
        "// comment\no(1);",
        "/* block */ o(1); /* multi\nline */ o(2);",
        "o(1) // trailing",
        "o(1);\n\no(2);\r\no(3);\u{2028}o(4);\u{2029}o(5);",
        "var a = 1\nvar b = 2\no(a+b)",
        "function f(){ return\n1 } o(f());",
        "var a=1; var b=2; var c = a\n+b; o(c);",
        "o(1)\no(2)",
        "var x = 1; { var y = 2 } o(x+y);",
        ";;;o(1);;;",
        "{ } o(1);",
        "o(\u{FEFF}1);",
        "\u{FEFF}o(1);",
        "var \u{e9} = 1; o(\u{e9});",
        "var $x = 1, _y = 2; o($x+_y);",
        "o('\\u00e9'.length); o('\\ud83d\\ude00'.length); o('\u{1F600}'.length);",
        "o(0.1.toFixed(1));",
        "o((1).toString());",
        "o(1 .toString());",
        "var a = {}; a\n.b = 1; o(a.b);",
        "o(1/2/2);",
        "var re = /a\\/b/; o(re.source);",
        "var a = 4; var b = 2; o(a\n/b/1);",
        "o(typeof /x/);",
    ];
    for flags in [0, 1] {
        for (i, s) in snippets.iter().enumerate() {
            assert_same_program(flags, &format!("lex#{i}"), s);
        }
    }
}

/// CONFIGS.md row 60: RegExp through JS syntax, all flag combinations.
#[test]
fn lang_regexp_via_js() {
    let patterns = [
        "a", "a+", "a*", "a?", "a{2,3}", ".", "^a", "a$", "\\ba\\b", "[a-z]+",
        "[^a-z]+", "(a)(b)", "(?:ab)+", "(a)\\1", "(?=a)a", "(?!a)b", "a|b",
        "\\d+", "\\w+", "\\s+", "\\S+", "[\\d\\s]", "\\u00e9", "\\x41",
        "(\\w+)\\s(\\w+)", "a.*?b", "^$", "", "[]", "[^]",
    ];
    let flagsets = ["", "g", "i", "m", "gi", "gm", "im", "gim"];
    let subjects = [
        "''",
        "'a'",
        "'A'",
        "'abc'",
        "'aab'",
        "'ABCabc'",
        "'a\\nb'",
        "'foo bar'",
        "'\\u00e9\\u4e2d'",
        "'aaaa'",
        "'x'",
        "'line1\\nline2'",
        "'a,b;c'",
    ];
    for flags in [0, 1] {
        for pat in patterns {
            let mut src = String::new();
            for fs in flagsets {
                let re = format!("new RegExp('{}','{}')", pat.replace('\\', "\\\\"), fs);
                for s in subjects {
                    src.push_str(&format!(
                        "ok(function(){{ var r={re}; return String(r.exec({s})) }});\n\
                         ok(function(){{ var r={re}; return r.test({s}) }});\n\
                         ok(function(){{ var r={re}; return String({s}.match(r)) }});\n\
                         ok(function(){{ var r={re}; return {s}.search(r) }});\n\
                         ok(function(){{ var r={re}; return {s}.replace(r,'#') }});\n\
                         ok(function(){{ var r={re}; return {s}.replace(r,'[$&|$1|$$]') }});\n\
                         ok(function(){{ var r={re}; return {s}.replace(r,function(){{ return arguments.length }}) }});\n\
                         ok(function(){{ var r={re}; return JSON.stringify({s}.split(r)) }});\n\
                         ok(function(){{ var r={re}; var a=[],m; while ((m=r.exec({s}))) {{ a.push(m[0]+'@'+r.lastIndex); if (!r.global) break; if (a.length>20) break }} return JSON.stringify(a) }});\n\
                         ok(function(){{ var r={re}; return r.source+'/'+r.global+r.ignoreCase+r.multiline }});\n"
                    ));
                }
            }
            assert_same_program(flags, &format!("regexp /{pat}/"), &src);
        }
        // regexp literals and lastIndex handling
        let src = "var r=/a/g; o(r.lastIndex); o(r.test('aa')); o(r.lastIndex); o(r.test('aa')); o(r.lastIndex); o(r.test('aa')); o(r.lastIndex);\n\
                   var r2=/a/; o(r2.test('aa')); o(r2.lastIndex);\n\
                   var r3=/a/g; r3.lastIndex=5; o(r3.exec('aaa')); o(r3.lastIndex);\n\
                   var r4=/a/g; r4.lastIndex=-1; o(String(r4.exec('aaa'))); o(r4.lastIndex);\n\
                   var r5=/(a)|(b)/; o(JSON.stringify(r5.exec('b')));\n\
                   o(String(/x/)); o(String(/x/gim)); o(/x/.toString()); o(typeof /x/.exec);\n\
                   ok(function(){ return new RegExp(/a/g) });\n\
                   ok(function(){ return new RegExp(/a/g,'i') });\n\
                   ok(function(){ return new RegExp('a','x') });\n\
                   ok(function(){ return new RegExp('a','gg') });\n\
                   ok(function(){ return new RegExp('(') });\n\
                   ok(function(){ return new RegExp() });\n\
                   ok(function(){ return new RegExp(undefined) });\n\
                   ok(function(){ return RegExp('a').source });\n";
        assert_same_program(flags, "regexp lastIndex", src);
    }
}

/// CONFIGS.md row 61: GC pressure, large arrays / strings, deep recursion.
#[test]
fn lang_gc_and_large_data() {
    let snippets: &[&str] = &[
        // array growth crosses the flat/unflattened boundary
        "var a=[]; for (var i=0;i<20000;++i) a.push(i); o(a.length); o(a[0]); o(a[19999]); o(a[10000]);",
        "var a=[]; for (var i=0;i<5000;++i) a[i*3]=i; o(a.length); o(a[0]); o(a[14997]); o(a[1]);",
        "var a=new Array(1000); o(a.length); a[999]=1; o(a[999]); o(a[0]); o(String(a).length);",
        "var a=[]; for (var i=0;i<3000;++i) a.unshift(i); o(a.length); o(a[0]); o(a[2999]);",
        "var a=[]; for (var i=0;i<3000;++i) a.push(i); for (var i=0;i<1500;++i) a.shift(); o(a.length); o(a[0]);",
        "var a=[]; for (var i=0;i<2000;++i) a.push(i); a.reverse(); o(a[0]); o(a[1999]);",
        "var a=[]; for (var i=0;i<2000;++i) a.push((i*7919)%2000); a.sort(function(x,y){return x-y}); o(a[0]); o(a[1999]); o(a[1000]);",
        "var a=[]; for (var i=0;i<2000;++i) a.push((i*7919)%2000); a.sort(); o(a[0]); o(a[1999]);",
        // string growth
        "var s=''; for (var i=0;i<2000;++i) s+='ab'; o(s.length); o(s.charAt(0)); o(s.charAt(3999));",
        "var s='x'; for (var i=0;i<15;++i) s+=s; o(s.length);",
        "var a=[]; for (var i=0;i<3000;++i) a.push('item'+i); o(a.join(',').length);",
        // many objects -> GC
        "var a=[]; for (var i=0;i<20000;++i) a.push({k:i,s:'s'+i}); o(a.length); o(a[19999].s); a=null; o('freed');",
        "for (var round=0;round<5;++round) { var a=[]; for (var i=0;i<5000;++i) a.push({v:i}); } o('ok');",
        // property churn
        "var o1={}; for (var i=0;i<5000;++i) o1['k'+i]=i; o(Object.keys(o1).length); for (var i=0;i<2500;++i) delete o1['k'+i]; o(Object.keys(o1).length);",
        // closures retaining scopes
        "var fs=[]; for (var i=0;i<2000;++i) fs.push((function(n){ return function(){ return n } })(i)); o(fs.length); o(fs[0]()); o(fs[1999]());",
        // deep recursion (bounded well under the C stack)
        "function f(n){ return n<=0?0:1+f(n-1) } o(f(100)); o(f(500));",
        "function f(n){ return n<=0?0:1+f(n-1) } ok(function(){ return f(100000) });",
        // deeply nested data
        "var d={}; var c=d; for (var i=0;i<200;++i) { c.n={}; c=c.n } o(JSON.stringify(d).length);",
        "var a=[]; var c=a; for (var i=0;i<150;++i) { var n=[]; c.push(n); c=n } o(JSON.stringify(a).length);",
        // interning many distinct strings
        "var a=[]; for (var i=0;i<10000;++i) a.push(String(i)); o(a.length); o(a[9999]);",
        // regexp compiled repeatedly
        "var n=0; for (var i=0;i<2000;++i) { if (new RegExp('a'+i).test('a'+i)) ++n } o(n);",
        // try/catch churn
        "var n=0; for (var i=0;i<2000;++i) { try { throw i } catch (e) { n+=e } } o(n);",
        // arguments objects
        "function f(){ return arguments } var a=[]; for (var i=0;i<2000;++i) a.push(f(i,i+1)); o(a.length); o(a[1999][1]);",
    ];
    for flags in [0, 1] {
        for (i, s) in snippets.iter().enumerate() {
            assert_same_program(flags, &format!("gc#{i}"), s);
        }
    }
}

/// CONFIGS.md row 52: randomized program generation over the language core.
#[test]
fn lang_randomized_programs() {
    let mut rng = Rng::new(SEED);
    for flags in [0, 1] {
        for round in 0..900 {
            let mut src = String::new();
            let n = 1 + rng.below(6);
            for _ in 0..n {
                src.push_str(&gen_stmt(&mut rng, 0));
                src.push('\n');
            }
            if std::env::var_os("LANG_TRACE").is_some() {
                use std::io::Write;
                let mut e = std::io::stderr();
                let _ = writeln!(e, "START flags={flags} round={round} src={src:?}");
                let _ = e.flush();
            }
            assert_same_program(flags, &format!("rand-prog#{round}"), &src);
            if std::env::var_os("LANG_TRACE").is_some() {
                use std::io::Write;
                let mut e = std::io::stderr();
                let _ = writeln!(e, "DONE flags={flags} round={round}");
                let _ = e.flush();
            }
        }
    }
}

fn gen_expr(rng: &mut Rng, depth: u32) -> String {
    if depth >= 3 {
        return match rng.below(8) {
            0 => format!("{}", rng.range_i32(-20, 20)),
            1 => "1.5".into(),
            2 => "'s'".into(),
            3 => "true".into(),
            4 => "null".into(),
            5 => "undefined".into(),
            6 => "NaN".into(),
            _ => "v".into(),
        };
    }
    match rng.below(16) {
        0 => format!("{}", rng.range_i32(-100, 100)),
        1 => format!("{}", rng.double().abs().min(1e6)),
        2 => "'str'".into(),
        3 => "''".into(),
        4 => "true".into(),
        5 => "false".into(),
        6 => "null".into(),
        7 => "undefined".into(),
        8 => {
            let ops = [
                "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "===",
                "!==", "&", "|", "^", "<<", ">>", ">>>", "&&", "||",
            ];
            format!(
                "({} {} {})",
                gen_expr(rng, depth + 1),
                ops[rng.below(ops.len() as u32) as usize],
                gen_expr(rng, depth + 1)
            )
        }
        9 => {
            let ops = ["-", "+", "!", "~", "typeof ", "void "];
            format!(
                "({}{})",
                ops[rng.below(ops.len() as u32) as usize],
                gen_expr(rng, depth + 1)
            )
        }
        10 => format!(
            "({} ? {} : {})",
            gen_expr(rng, depth + 1),
            gen_expr(rng, depth + 1),
            gen_expr(rng, depth + 1)
        ),
        11 => format!(
            "[{},{}]",
            gen_expr(rng, depth + 1),
            gen_expr(rng, depth + 1)
        ),
        12 => format!(
            "{{a:{},b:{}}}",
            gen_expr(rng, depth + 1),
            gen_expr(rng, depth + 1)
        ),
        13 => format!("String({})", gen_expr(rng, depth + 1)),
        14 => format!("Number({})", gen_expr(rng, depth + 1)),
        _ => format!("(function(x){{ return x }})({})", gen_expr(rng, depth + 1)),
    }
}

/// Unique loop-variable counter: nested `for (var i=...)` loops that reuse the
/// same variable are a genuine infinite loop in JS, so each generated loop gets
/// its own name.
static LOOPVAR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn next_loopvar() -> String {
    let n = LOOPVAR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("i{n}")
}

fn gen_stmt(rng: &mut Rng, depth: u32) -> String {
    if depth >= 2 {
        return format!("o({});", gen_expr(rng, 0));
    }
    match rng.below(10) {
        0 => format!("o({});", gen_expr(rng, 0)),
        1 => format!("oj({});", gen_expr(rng, 0)),
        2 => format!("var v = {}; o(v);", gen_expr(rng, 0)),
        3 => format!(
            "if ({}) {{ {} }} else {{ {} }}",
            gen_expr(rng, 1),
            gen_stmt(rng, depth + 1),
            gen_stmt(rng, depth + 1)
        ),
        4 => {
            let lv = next_loopvar();
            let bound = 1 + rng.below(4);
            let body = gen_stmt(rng, depth + 1);
            format!("for (var {lv}=0;{lv}<{bound};++{lv}) {{ {body} }}")
        }
        5 => format!(
            "try {{ {} }} catch (e) {{ __out += 'C'+String(e)+'|' }}",
            gen_stmt(rng, depth + 1)
        ),
        6 => format!(
            "ok(function(){{ return {} }});",
            gen_expr(rng, 0)
        ),
        7 => format!(
            "switch ({}) {{ case 1: {} break; default: {} }}",
            gen_expr(rng, 1),
            gen_stmt(rng, depth + 1),
            gen_stmt(rng, depth + 1)
        ),
        8 => format!(
            "(function(){{ var v = {}; {} }})();",
            gen_expr(rng, 1),
            gen_stmt(rng, depth + 1)
        ),
        _ => {
            let kv = next_loopvar();
            format!(
                "var v = {}; for (var {kv} in v) {{ __out += {kv}+'='+String(v[{kv}])+'|' }}",
                gen_expr(rng, 1)
            )
        }
    }
}
