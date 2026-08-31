// Level 7: deterministic randomized differential testing.
mod common;

use common::*;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

fn compare_batch(label: &str, scripts: &[String]) {
    let cs = Session::new(Side::C, 0);
    cs.register_print();
    let rs = Session::new(Side::Rust, 0);
    rs.register_print();
    let mut failures = Vec::new();
    for src in scripts {
        let a = run_script(&cs, src);
        let b = run_script(&rs, src);
        if a != b {
            failures.push(format!(
                "--- {} ---\n{}\n--- C ---\n{:?}\n--- Rust ---\n{:?}",
                label, src, a, b
            ));
            if failures.len() >= 10 {
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}: {} mismatches (showing up to 10):\n{}",
        label,
        failures.len(),
        failures.join("\n\n")
    );
}

// ---------------------------------------------------------------------------
// 1. Random source text: mostly garbage, exercises lexer + parser error paths.
// ---------------------------------------------------------------------------

#[test]
fn fuzz_token_soup() {
    const TOKENS: &[&str] = &[
        "var", "function", "return", "if", "else", "for", "while", "do", "break", "continue",
        "switch", "case", "default", "throw", "try", "catch", "finally", "new", "delete",
        "typeof", "instanceof", "in", "this", "null", "true", "false", "void", "with",
        "debugger", "{", "}", "(", ")", "[", "]", ";", ",", ".", ":", "?", "=", "==", "===",
        "!=", "!==", "<", ">", "<=", ">=", "+", "-", "*", "/", "%", "++", "--", "&", "|", "^",
        "~", "!", "&&", "||", "<<", ">>", ">>>", "+=", "-=", "*=", "/=", "%=", "a", "b", "x",
        "0", "1", "1.5", "0x1f", "'s'", "\"d\"", "/re/", "/re/g", "\\u0041", "$", "_",
        "//c\n", "/*c*/", "\n", " ", "\t", "\u{00e9}", "\u{2028}", "\u{feff}", "'\\x41'",
        "'\\u{41}'", "'unterminated", "0b1", "0o7", "1e", "1e+", ".5", "5.", "=>", "...",
        "get", "set", "let", "const", "class", "yield", "async", "await", "of", "static",
    ];
    let mut rng = Rng::new(0xC0FFEE);
    let mut scripts = Vec::new();
    for _ in 0..3000 {
        let n = 1 + rng.below(10);
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(rng.pick(TOKENS));
            if rng.below(3) == 0 {
                s.push(' ');
            }
        }
        scripts.push(s);
    }
    compare_batch("token soup", &scripts);
}

#[test]
fn fuzz_random_bytes_as_source() {
    let mut rng = Rng::new(0xBADC0DE);
    let mut scripts = Vec::new();
    for _ in 0..2000 {
        let n = 1 + rng.below(24);
        let mut s = String::new();
        for _ in 0..n {
            // printable ASCII plus a few interesting non-ASCII code points
            let c = match rng.below(10) {
                0..=7 => char::from(32u8 + (rng.below(95) as u8)),
                8 => *rng.pick(&['\n', '\t', '\r', '\u{0b}', '\u{0c}']),
                _ => *rng.pick(&['\u{00e9}', '\u{4e2d}', '\u{2028}', '\u{2029}', '\u{feff}', '\u{a0}']),
            };
            s.push(c);
        }
        scripts.push(s);
    }
    compare_batch("random bytes", &scripts);
}

// ---------------------------------------------------------------------------
// 2. Grammar-driven random expressions: exercises the compiler + runtime.
// ---------------------------------------------------------------------------

fn gen_expr(rng: &mut Rng, depth: usize) -> String {
    const ATOMS: &[&str] = &[
        "0", "1", "-1", "2", "0.5", "-0", "NaN", "Infinity", "1e21", "1e-7", "2147483647",
        "4294967296", "9007199254740993", "''", "'a'", "'0'", "'abc'", "'1e3'", "' 12 '",
        "true", "false", "null", "undefined", "[]", "[1]", "[1,2]", "[[1],[2]]", "({})",
        "({a:1})", "({valueOf:function(){return 3}})", "({toString:function(){return '4'}})",
        "(function(){return 5})", "new Number(2)", "new String('s')", "new Boolean(false)",
        "[1,2,3]", "'\\u00e9'", "0x10", "x", "y", "obj", "arr", "fn",
    ];
    const BINOPS: &[&str] = &[
        "+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", ">>>", "<", ">", "<=", ">=", "==",
        "!=", "===", "!==", "&&", "||", ",", "instanceof", "in",
    ];
    const UNOPS: &[&str] = &["-", "+", "!", "~", "typeof ", "void "];
    if depth == 0 {
        return rng.pick(ATOMS).to_string();
    }
    match rng.below(10) {
        0..=3 => format!(
            "({} {} {})",
            gen_expr(rng, depth - 1),
            rng.pick(BINOPS),
            gen_expr(rng, depth - 1)
        ),
        4 => format!("({}{})", rng.pick(UNOPS), gen_expr(rng, depth - 1)),
        5 => format!(
            "({} ? {} : {})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
        6 => format!("[{},{}]", gen_expr(rng, depth - 1), gen_expr(rng, depth - 1)),
        7 => format!(
            "({{a:{},b:{}}})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
        8 => format!("({})[{}]", gen_expr(rng, depth - 1), gen_expr(rng, depth - 1)),
        _ => format!(
            "(function(p){{return {}}})({})",
            gen_expr(rng, depth - 1),
            gen_expr(rng, depth - 1)
        ),
    }
}

#[test]
fn fuzz_expressions() {
    let mut rng = Rng::new(0x1234_5678);
    let prelude = "var x=1, y='2', obj={a:1,b:[1,2]}, arr=[1,2,3], fn=function(a){return a};";
    let mut scripts = Vec::new();
    for _ in 0..4000 {
        let d = 1 + rng.below(4);
        scripts.push(format!(
            "{} try {{ String({}) }} catch(e) {{ e.name + ':' + e.message }}",
            prelude,
            gen_expr(&mut rng, d)
        ));
    }
    compare_batch("expressions", &scripts);
}

#[test]
fn fuzz_statements() {
    let mut rng = Rng::new(0xFEED_BEEF);
    const STMTS: &[&str] = &[
        "s+=1;",
        "s+='a';",
        "if(i%2)s+='o';else s+='e';",
        "for(var k=0;k<2;k++)s+=k;",
        "while(i>0){i--;s+='w';}",
        "do{s+='d';i--;}while(i>0);",
        "try{throw i}catch(e){s+=e}",
        "try{s+='t'}finally{s+='f'}",
        "switch(i%3){case 0:s+='0';case 1:s+='1';break;default:s+='D'}",
        "for(var p in obj)s+=p;",
        "s+=(function(){return i})();",
        "with(obj){s+=a}",
        "i++;",
        "i--;",
        "arr.push(i);s+=arr.length;",
        "obj['k'+i]=i;s+=Object.keys(obj).length;",
        "s+=arr.join('');",
        "if(i>2)break;",
        "if(i<0)continue;",
        "s+=typeof obj;",
        "delete obj.a;s+=('a' in obj);",
        "s+=JSON.stringify(obj);",
        "s+=[i,i+1].map(function(v){return v*2}).join(',');",
        "s+=String(i/0);",
        "s+=(i).toString(16);",
        "label"
    ];
    let mut scripts = Vec::new();
    for _ in 0..2000 {
        let n = 1 + rng.below(6);
        let mut body = String::new();
        for _ in 0..n {
            let st = rng.pick(STMTS);
            if *st == "label" {
                continue;
            }
            body.push_str(st);
        }
        scripts.push(format!(
            "var s='', i={}, obj={{a:1}}, arr=[]; \
             try {{ for(var n=0;n<3;n++){{ {} }} }} catch(e) {{ s+='!'+e.name }} s",
            rng.below(5),
            body
        ));
    }
    compare_batch("statements", &scripts);
}

// ---------------------------------------------------------------------------
// 3. Builtin method fuzzing.
// ---------------------------------------------------------------------------

const NUM_LITERALS: &[&str] = &[
    "0", "-0", "1", "-1", "0.5", "-0.5", "1/3", "1e-7", "1e21", "1e-21", "1e308", "1e-308",
    "5e-324", "NaN", "Infinity", "-Infinity", "2147483647", "-2147483648", "4294967295",
    "4294967296", "9007199254740991", "9007199254740993", "123.456", "0.000001", "1e100",
    "255", "1.005", "12345.6789", "1.7976931348623157e308", "3.141592653589793",
];

#[test]
fn fuzz_number_methods() {
    let mut rng = Rng::new(0xAB_CDEF);
    let mut scripts = Vec::new();
    for n in NUM_LITERALS {
        scripts.push(format!("String({})", n));
        scripts.push(format!("({}).toString()", n));
        for radix in 2..=36 {
            scripts.push(format!("try{{({}).toString({})}}catch(e){{e.name}}", n, radix));
        }
        for d in [-1i32, 0, 1, 2, 5, 10, 20, 21, 100, 101] {
            scripts.push(format!("try{{({}).toFixed({})}}catch(e){{e.name}}", n, d));
            scripts.push(format!(
                "try{{({}).toExponential({})}}catch(e){{e.name}}",
                n, d
            ));
            scripts.push(format!("try{{({}).toPrecision({})}}catch(e){{e.name}}", n, d));
        }
        scripts.push(format!("({}).toPrecision()", n));
        scripts.push(format!("({}).toExponential()", n));
        scripts.push(format!("Math.abs({})", n));
        scripts.push(format!("Math.ceil({})", n));
        scripts.push(format!("Math.floor({})", n));
        scripts.push(format!("Math.round({})", n));
        scripts.push(format!("Math.sqrt({})", n));
        scripts.push(format!("Math.exp({})", n));
        scripts.push(format!("Math.log({})", n));
        scripts.push(format!("Math.sin({})", n));
        scripts.push(format!("Math.cos({})", n));
        scripts.push(format!("Math.tan({})", n));
        scripts.push(format!("Math.asin({})", n));
        scripts.push(format!("Math.acos({})", n));
        scripts.push(format!("Math.atan({})", n));
        scripts.push(format!("parseInt('{}')", n));
        scripts.push(format!("parseFloat(String({}))", n));
        scripts.push(format!("JSON.stringify({})", n));
        scripts.push(format!("({})|0", n));
        scripts.push(format!("({})>>>0", n));
        scripts.push(format!("~({})", n));
        scripts.push(format!("({})<<1", n));
    }
    // Random pairs for arithmetic and Math.pow/atan2
    for _ in 0..1500 {
        let a = rng.pick(NUM_LITERALS);
        let b = rng.pick(NUM_LITERALS);
        scripts.push(format!("String(({})+({}))", a, b));
        scripts.push(format!("String(({})*({}))", a, b));
        scripts.push(format!("String(({})/({}))", a, b));
        scripts.push(format!("String(({})%({}))", a, b));
        scripts.push(format!("String(Math.pow({},{}))", a, b));
        scripts.push(format!("String(Math.atan2({},{}))", a, b));
        scripts.push(format!("String(Math.max({},{}))", a, b));
        scripts.push(format!("String(Math.min({},{}))", a, b));
        scripts.push(format!("String(parseInt('{}',{}))", a, rng.below(40)));
    }
    compare_batch("number methods", &scripts);
}

const STR_LITERALS: &[&str] = &[
    "''",
    "'a'",
    "'abc'",
    "'ABC'",
    "'aAbBcC'",
    "'  pad  '",
    "'\\t\\n\\r x'",
    "'a,b,,c'",
    "'aaa'",
    "'abcabcabc'",
    "'\\u00e9\\u00c9'",
    "'\\u00df'",
    "'\\u4e2d\\u6587'",
    "'\\ud83d\\ude00'",
    "'\\u0130\\u0131'",
    "'0123456789'",
    "'a/b?c=d&e'",
    "'%41%C3%A9'",
    "'\\u0000a'",
    "'line1\\nline2'",
];

#[test]
fn fuzz_string_methods() {
    let mut rng = Rng::new(0x5EED);
    let mut scripts = Vec::new();
    for s in STR_LITERALS {
        scripts.push(format!("({}).length", s));
        scripts.push(format!("({}).toUpperCase()", s));
        scripts.push(format!("({}).toLowerCase()", s));
        scripts.push(format!("({}).toLocaleUpperCase()", s));
        scripts.push(format!("({}).toLocaleLowerCase()", s));
        scripts.push(format!("({}).trim()", s));
        scripts.push(format!("JSON.stringify({})", s));
        scripts.push(format!("encodeURI({})", s));
        scripts.push(format!("try{{decodeURI({})}}catch(e){{e.name}}", s));
        scripts.push(format!("encodeURIComponent({})", s));
        scripts.push(format!("try{{decodeURIComponent({})}}catch(e){{e.name}}", s));
        scripts.push(format!("escape({})", s));
        scripts.push(format!("unescape({})", s));
        scripts.push(format!("({}).split('').length", s));
        scripts.push(format!("({}).split('').join('|')", s));
        for i in [-2i32, -1, 0, 1, 2, 5, 100] {
            scripts.push(format!("({}).charAt({})", s, i));
            scripts.push(format!("({}).charCodeAt({})", s, i));
            scripts.push(format!("({}).slice({})", s, i));
            scripts.push(format!("({}).substring({})", s, i));
            scripts.push(format!("({}).substr({})", s, i));
        }
        for _ in 0..30 {
            let a = rng.next() as i32 % 8 - 4;
            let b = rng.next() as i32 % 8 - 4;
            scripts.push(format!("({}).slice({},{})", s, a, b));
            scripts.push(format!("({}).substring({},{})", s, a, b));
            scripts.push(format!("({}).substr({},{})", s, a, b));
            let t = rng.pick(STR_LITERALS);
            scripts.push(format!("({}).indexOf({})", s, t));
            scripts.push(format!("({}).indexOf({},{})", s, t, a));
            scripts.push(format!("({}).lastIndexOf({})", s, t));
            scripts.push(format!("({}).split({}).join('|')", s, t));
            scripts.push(format!("({}).replace({},'X')", s, t));
            scripts.push(format!("({}).concat({})", s, t));
            scripts.push(format!("({}).localeCompare({})", s, t));
            scripts.push(format!("({} < {})", s, t));
            scripts.push(format!("({} == {})", s, t));
            scripts.push(format!("String.fromCharCode({})", rng.below(70000)));
        }
    }
    compare_batch("string methods", &scripts);
}

#[test]
fn fuzz_regexp_via_js() {
    const PATS: &[&str] = &[
        "a", "a*", "a+", "a?", "a{2,3}", "(a)(b)", "(a|b)+", "[a-z]+", "[^a-z]+", "\\d+",
        "\\w+", "\\s+", "\\b\\w+\\b", "^a", "a$", ".", ".*", ".+?", "(?:ab)+", "(?=a)a",
        "(?!a).", "(a)\\1", "[\\d\\s]", "\\u00e9", "[\\u00e0-\\u00ff]+", "()", "(|a)",
        "a|", "[]", "[^]", "((((a))))", "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)",
    ];
    const FLAGS: &[&str] = &["", "g", "i", "m", "gi", "gm", "im", "gim"];
    const SUBJ: &[&str] = &[
        "''", "'a'", "'aaa'", "'abc'", "'ABC'", "'abcabc'", "'a\\nb'", "'  a b  '",
        "'\\u00e9\\u00c9'", "'123abc456'", "'\\u4e2d\\u6587abc'", "'aXbXc'",
    ];
    let mut scripts = Vec::new();
    for p in PATS {
        for f in FLAGS {
            scripts.push(format!("try{{String(new RegExp('{}','{}'))}}catch(e){{e.name}}", p, f));
            for s in SUBJ {
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); String(r.test({}))}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); JSON.stringify(r.exec({}))}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); JSON.stringify({}.match(r))}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); String({}.search(r))}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); {}.replace(r,'<$&>')}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); JSON.stringify({}.split(r))}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); {}.replace(r,function(){{return Array.prototype.join.call(arguments,'|')}})}}catch(e){{e.name}}",
                    p, f, s
                ));
                scripts.push(format!(
                    "try{{var r=new RegExp('{}','{}'); var o=[]; var m; var guard=0; var prev=-1; \
                     while((m=r.exec({}))!==null){{ o.push(m[0]+'@'+m.index); if(!r.global)break; \
                     if(++guard>20)break; if(r.lastIndex===prev)break; prev=r.lastIndex; }} \
                     JSON.stringify(o)}}catch(e){{e.name}}",
                    p, f, s
                ));
            }
        }
    }
    compare_batch("regexp via js", &scripts);
}

#[test]
fn fuzz_array_methods() {
    let mut rng = Rng::new(0xA77A7);
    const ARRS: &[&str] = &[
        "[]",
        "[1]",
        "[1,2,3]",
        "[3,1,2]",
        "['b','a','c']",
        "[1,'a',null,undefined,true]",
        "(function(){var a=[1,2,3];delete a[1];return a})()",
        "(function(){var a=[];a[5]=1;return a})()",
        "[[1,2],[3,[4]]]",
        "[NaN,0,-0,Infinity]",
        "(function(){var a=[1];a.x=2;return a})()",
        "new Array(3)",
        "[1,2,3,4,5,6,7,8,9,10]",
    ];
    let mut scripts = Vec::new();
    for a in ARRS {
        scripts.push(format!("({}).length", a));
        scripts.push(format!("({}).join(',')", a));
        scripts.push(format!("({}).toString()", a));
        scripts.push(format!("JSON.stringify({})", a));
        scripts.push(format!("({}).reverse().join(',')", a));
        scripts.push(format!("({}).sort().join(',')", a));
        scripts.push(format!(
            "({}).sort(function(x,y){{return x<y?-1:x>y?1:0}}).join(',')",
            a
        ));
        scripts.push(format!("({}).concat([9]).join(',')", a));
        scripts.push(format!("({}).indexOf(1)", a));
        scripts.push(format!("({}).lastIndexOf(1)", a));
        scripts.push(format!(
            "({}).map(function(v,i){{return i+':'+v}}).join(';')",
            a
        ));
        scripts.push(format!(
            "({}).filter(function(v){{return !!v}}).join(',')",
            a
        ));
        scripts.push(format!("({}).every(function(v){{return !!v}})", a));
        scripts.push(format!("({}).some(function(v){{return !!v}})", a));
        scripts.push(format!(
            "try{{String(({}).reduce(function(x,y){{return String(x)+String(y)}}))}}catch(e){{e.name}}",
            a
        ));
        scripts.push(format!(
            "try{{String(({}).reduceRight(function(x,y){{return String(x)+String(y)}}))}}catch(e){{e.name}}",
            a
        ));
        scripts.push(format!(
            "var s='';({}).forEach(function(v,i){{s+=i+'='+v+';'}});s",
            a
        ));
        for _ in 0..25 {
            let i = rng.next() as i32 % 9 - 4;
            let n = rng.next() as i32 % 9 - 4;
            scripts.push(format!("({}).slice({},{}).join(',')", a, i, n));
            scripts.push(format!(
                "var t={};var r=t.splice({},{});r.join(',')+'/'+t.join(',')",
                a, i, n
            ));
            scripts.push(format!(
                "var t={};t.splice({},{},'X','Y');t.join(',')",
                a, i, n
            ));
            scripts.push(format!("var t={};t.length={};t.join(',')", a, n.abs()));
            scripts.push(format!("var t={};t.push({});t.join(',')", a, i));
            scripts.push(format!("var t={};t.unshift({});t.join(',')", a, i));
            scripts.push(format!("var t={};String(t.pop())+'/'+t.join(',')", a));
            scripts.push(format!("var t={};String(t.shift())+'/'+t.join(',')", a));
        }
    }
    compare_batch("array methods", &scripts);
}

#[test]
fn fuzz_json_roundtrip() {
    let mut rng = Rng::new(0x1508);
    const VALS: &[&str] = &[
        "1", "-1.5", "0", "'s'", "''", "true", "false", "null", "undefined", "[]", "[1,2]",
        "{}", "{a:1}", "{a:{b:[1,{c:2}]}}", "[[[[1]]]]", "NaN", "Infinity", "1e21",
        "'\\u0001\\u001f\"\\\\'", "'\\u00e9'", "'\\ud83d\\ude00'", "function(){}",
        "new Date(0)", "/re/", "new Number(1)", "new String('x')",
    ];
    let mut scripts = Vec::new();
    for v in VALS {
        scripts.push(format!("String(JSON.stringify({}))", v));
        scripts.push(format!("String(JSON.stringify({},null,2))", v));
        scripts.push(format!("String(JSON.stringify({},null,'ab'))", v));
        scripts.push(format!("String(JSON.stringify({},null,20))", v));
        scripts.push(format!(
            "String(JSON.stringify({},function(k,x){{return x}}))",
            v
        ));
        scripts.push(format!(
            "try{{String(JSON.stringify(JSON.parse(JSON.stringify({}))))}}catch(e){{e.name}}",
            v
        ));
    }
    const TEXTS: &[&str] = &[
        "1", "-1", "1.5", "1e3", "\"a\"", "\"\\\\u00e9\"", "true", "false", "null", "[]",
        "[1,2]", "{}", "{\"a\":1}", "{\"a\":[1,{\"b\":2}]}", " 1 ", "01", "+1", ".5", "1.",
        "'a'", "{a:1}", "[1,]", "{\"a\":1,}", "\"\\\\x41\"", "\"\\\\u12\"", "[", "]", "{", "}",
        "", "  ", "nul", "trues", "1 2", "\"unterminated", "[1 2]", "{\"a\" 1}",
    ];
    for t in TEXTS {
        scripts.push(format!(
            "try{{String(JSON.stringify(JSON.parse('{}')))}}catch(e){{e.name+':'+e.message}}",
            t.replace('\\', "\\\\").replace('\'', "\\'")
        ));
    }
    for _ in 0..500 {
        let a = rng.pick(VALS);
        let b = rng.pick(VALS);
        scripts.push(format!(
            "String(JSON.stringify({{x:{},y:[{}]}},null,{}))",
            a,
            b,
            rng.below(11)
        ));
    }
    compare_batch("json", &scripts);
}

#[test]
fn fuzz_date_methods() {
    let mut rng = Rng::new(0xDA7E);
    let mut times: Vec<i64> = vec![
        0,
        1,
        -1,
        1000,
        -1000,
        86_400_000,
        -86_400_000,
        1_234_567_890_123,
        -1_234_567_890_123,
        8_640_000_000_000_000,
        -8_640_000_000_000_000,
        8_640_000_000_000_001,
        946_684_800_000,
        951_782_400_000, // 2000-02-29
        4_107_542_400_000,
        -62_167_219_200_000,
    ];
    for _ in 0..300 {
        times.push((rng.next() % 17_000_000_000_000) as i64 - 8_500_000_000_000);
    }
    const METHODS: &[&str] = &[
        "getTime", "valueOf", "getFullYear", "getMonth", "getDate", "getDay", "getHours",
        "getMinutes", "getSeconds", "getMilliseconds", "getUTCFullYear", "getUTCMonth",
        "getUTCDate", "getUTCDay", "getUTCHours", "getUTCMinutes", "getUTCSeconds",
        "getUTCMilliseconds", "getTimezoneOffset", "toString", "toDateString",
        "toTimeString", "toUTCString", "toISOString", "toJSON", "toLocaleString",
        "toLocaleDateString", "toLocaleTimeString",
    ];
    let mut scripts = Vec::new();
    for t in &times {
        for m in METHODS {
            scripts.push(format!("try{{String(new Date({}).{}())}}catch(e){{e.name}}", t, m));
        }
        scripts.push(format!("String(new Date({}))", t));
        scripts.push(format!(
            "try{{String(Date.parse(new Date({}).toISOString()))}}catch(e){{e.name}}",
            t
        ));
        scripts.push(format!(
            "try{{String(Date.parse(new Date({}).toUTCString()))}}catch(e){{e.name}}",
            t
        ));
    }
    // setters
    const SETTERS: &[&str] = &[
        "setTime", "setMilliseconds", "setUTCMilliseconds", "setSeconds", "setUTCSeconds",
        "setMinutes", "setUTCMinutes", "setHours", "setUTCHours", "setDate", "setUTCDate",
        "setMonth", "setUTCMonth", "setFullYear", "setUTCFullYear",
    ];
    for s in SETTERS {
        for v in ["0", "1", "-1", "13", "32", "60", "1000", "NaN", "1e21", "2000"] {
            scripts.push(format!(
                "try{{var d=new Date(0); d.{}({}); String(d.getTime())+'|'+String(d)}}catch(e){{e.name}}",
                s, v
            ));
        }
    }
    // constructors
    for args in [
        "", "0", "NaN", "'2000-01-01'", "'2000-01-01T00:00:00Z'", "'bogus'",
        "2000,0", "2000,0,1", "2000,0,1,12", "2000,0,1,12,30", "2000,0,1,12,30,45",
        "2000,0,1,12,30,45,678", "1999,11,31,23,59,59,999", "0,0", "-1,0", "70,0,1",
        "2000,13,1", "2000,0,32", "2000,0,0", "2000,0,1,25", "1e21,0",
    ] {
        scripts.push(format!(
            "try{{String(new Date({}).getTime())}}catch(e){{e.name}}",
            args
        ));
        scripts.push(format!("try{{String(Date.UTC({}))}}catch(e){{e.name}}", args));
    }
    for s in [
        "'2000-01-01'", "'2000-01-01T00:00:00'", "'2000-01-01T00:00:00Z'",
        "'2000-01-01T00:00:00.123Z'", "'2000-01-01T00:00:00+01:00'", "'Sat, 01 Jan 2000 00:00:00 GMT'",
        "'Jan 1 2000'", "'1 Jan 2000'", "'2000'", "'2000-13-01'", "'x'", "''",
    ] {
        scripts.push(format!("String(Date.parse({}))", s));
        scripts.push(format!("String(new Date({}).getTime())", s));
    }
    compare_batch("date methods", &scripts);
}

#[test]
fn fuzz_deep_and_wide_programs() {
    let mut scripts = Vec::new();
    // deep nesting -- exercises parser recursion limits identically
    for depth in [1usize, 5, 10, 50, 100, 200] {
        scripts.push(format!("{}1{}", "(".repeat(depth), ")".repeat(depth)));
        scripts.push(format!("{}1{}", "[".repeat(depth), "]".repeat(depth)));
        scripts.push(format!("{}", "-".repeat(depth) + "1"));
        scripts.push(format!("{}1", "!".repeat(depth)));
        scripts.push(format!(
            "{}{}",
            "if(1)".repeat(depth),
            "1"
        ));
        scripts.push(format!(
            "var s=0;{}{}s",
            "for(var i=0;i<1;i++){".repeat(depth.min(60)),
            "}".repeat(depth.min(60))
        ));
        scripts.push(format!(
            "{}1{}",
            "function f(){return ".repeat(depth.min(60)),
            "}".repeat(depth.min(60))
        ));
        scripts.push(format!("1{}", "+1".repeat(depth * 5)));
        scripts.push(format!("{}", "1?".repeat(depth) + "1" + &":1".repeat(depth)));
    }
    // wide programs
    scripts.push(format!(
        "var a=[{}]; a.length",
        (0..2000).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
    ));
    scripts.push(format!(
        "var o={{{}}}; Object.keys(o).length",
        (0..1000)
            .map(|i| format!("k{}:{}", i, i))
            .collect::<Vec<_>>()
            .join(",")
    ));
    scripts.push(format!(
        "var s={}; s.length",
        (0..500).map(|_| "'x'").collect::<Vec<_>>().join("+")
    ));
    scripts.push(format!(
        "function f({}){{return arguments.length}} f({})",
        (0..200).map(|i| format!("p{}", i)).collect::<Vec<_>>().join(","),
        (0..200).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
    ));
    scripts.push(format!(
        "var s=''; switch(3){{{}}} s",
        (0..300)
            .map(|i| format!("case {}: s+='{}'; break;", i, i))
            .collect::<Vec<_>>()
            .join("")
    ));
    scripts.push(format!("'{}'.length", "y".repeat(5000)));
    compare_batch("deep/wide", &scripts);
}

#[test]
fn fuzz_closures_and_scoping() {
    let mut rng = Rng::new(0xC105);
    const FRAGS: &[&str] = &[
        "var v=1;",
        "function g(){return v}",
        "var g=function(){return v};",
        "v=2;",
        "if(1){var v=3}",
        "for(var v=0;v<2;v++){}",
        "try{throw 4}catch(v){}",
        "(function(){v=5})();",
        "(function(v){})(6);",
        "with({v:7}){}",
        "delete v;",
        "eval('v=8');",
        "eval('var v=9');",
        "function v(){return 10}",
        "var v;",
        "arguments;",
        "this;",
    ];
    let mut scripts = Vec::new();
    for _ in 0..1500 {
        let n = 2 + rng.below(5);
        let mut body = String::new();
        for _ in 0..n {
            body.push_str(rng.pick(FRAGS));
        }
        scripts.push(format!(
            "try{{ (function(){{ {} return typeof v==='function'?'fn':String(v) }})() }}catch(e){{ e.name }}",
            body
        ));
        scripts.push(format!(
            "try{{ {} typeof v==='function'?'fn':String(v) }}catch(e){{ e.name }}",
            body
        ));
    }
    compare_batch("closures/scoping", &scripts);
}

#[test]
fn fuzz_object_operations() {
    let mut rng = Rng::new(0x0B7EC7);
    const OBJS: &[&str] = &[
        "{}",
        "{a:1}",
        "{a:1,b:2,c:3}",
        "[1,2]",
        "'str'",
        "1",
        "true",
        "null",
        "undefined",
        "function(){}",
        "/re/",
        "new Date(0)",
        "Object.create(null)",
        "Object.create({p:1})",
        "Math",
        "JSON",
        "(function(){var o={};Object.defineProperty(o,'h',{value:1});return o})()",
        "Object.freeze({a:1})",
        "Object.seal({a:1})",
        "Object.preventExtensions({a:1})",
    ];
    const OPS: &[&str] = &[
        "Object.keys(X)",
        "Object.getOwnPropertyNames(X)",
        "Object.getPrototypeOf(X)===Object.prototype",
        "Object.isFrozen(X)",
        "Object.isSealed(X)",
        "Object.isExtensible(X)",
        "Object.prototype.toString.call(X)",
        "Object.prototype.hasOwnProperty.call(X,'a')",
        "Object.prototype.propertyIsEnumerable.call(X,'a')",
        "Object.prototype.isPrototypeOf.call(Object.prototype,X)",
        "Object.prototype.valueOf.call(X)===X",
        "JSON.stringify(Object.getOwnPropertyDescriptor(X,'a'))",
        "(function(){var s='';for(var k in X)s+=k+';';return s})()",
        "(function(){var o=X;o.zz=1;return String(o.zz)})()",
        "(function(){var o=X;delete o.a;return String(o.a)})()",
        "Object.defineProperty(X,'q',{value:1}); String(X.q)",
        "String(Object(X))",
        "String(X)",
        "typeof X",
        "String(X&&X.constructor&&X.constructor.name)",
    ];
    let mut scripts = Vec::new();
    for o in OBJS {
        for op in OPS {
            scripts.push(format!(
                "try{{ String({}) }}catch(e){{ e.name }}",
                op.replace('X', &format!("({})", o))
            ));
        }
    }
    for _ in 0..800 {
        let a = rng.pick(OBJS);
        let b = rng.pick(OBJS);
        scripts.push(format!(
            "try{{ String(({}) == ({})) }}catch(e){{ e.name }}",
            a, b
        ));
        scripts.push(format!(
            "try{{ String(({}) === ({})) }}catch(e){{ e.name }}",
            a, b
        ));
        scripts.push(format!(
            "try{{ String(({}) instanceof ({})) }}catch(e){{ e.name }}",
            a, b
        ));
        scripts.push(format!("try{{ String('a' in ({})) }}catch(e){{ e.name }}", b));
        scripts.push(format!(
            "try{{ String(({}) + ({})) }}catch(e){{ e.name }}",
            a, b
        ));
        scripts.push(format!(
            "try{{ String(({}) < ({})) }}catch(e){{ e.name }}",
            a, b
        ));
    }
    compare_batch("object operations", &scripts);
}
