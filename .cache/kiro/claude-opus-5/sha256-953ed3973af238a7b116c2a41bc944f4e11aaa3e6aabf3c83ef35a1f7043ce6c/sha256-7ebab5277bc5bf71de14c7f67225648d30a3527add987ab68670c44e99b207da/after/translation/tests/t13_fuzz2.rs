// Level 13: a second, wider randomized sweep -- source-level string/regexp
// literals, property descriptors, receiver/method matrices, and mixed programs.
mod common;

use common::*;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0xD1B54A32D192ED03)
    }
    fn next(&mut self) -> u64 {
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
            if failures.len() >= 8 {
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}: {} mismatches:\n{}",
        label,
        failures.len(),
        failures.join("\n\n")
    );
}

/// Random *source text* for a string literal, exercising jslex's escape handling.
fn random_string_literal(rng: &mut Rng) -> String {
    const PIECES: &[&str] = &[
        "a", "Z", "0", " ", "\\n", "\\r", "\\t", "\\b", "\\f", "\\v", "\\0", "\\\\", "\\'",
        "\\\"", "\\/", "\\x41", "\\x7f", "\\xff", "\\x0", "\\xg", "\\u0041", "\\u00e9",
        "\\u4e2d", "\\ud83d", "\\ude00", "\\uffff", "\\u{41}", "\\u{1F600}", "\\u{}",
        "\\u{110000}", "\\1", "\\7", "\\8", "\\012", "\\377", "\\400", "\\a", "\\q", "\\$",
        "\u{00e9}", "\u{4e2d}", "\u{1F600}", "\u{a0}", "\u{feff}", "$", "_", "%", "\\\n",
    ];
    let n = 1 + rng.below(6);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(rng.pick(PIECES));
    }
    s
}

#[test]
fn fuzz_string_literals_in_source() {
    let mut rng = Rng::new(0x5721);
    let mut scripts = Vec::new();
    for _ in 0..2500 {
        let body = random_string_literal(&mut rng);
        let q = if rng.below(2) == 0 { '\'' } else { '"' };
        scripts.push(format!(
            "try{{ var s = {q}{body}{q}; s.length+':'+JSON.stringify(s) }}catch(e){{ e.name+':'+e.message }}",
            q = q,
            body = body
        ));
        scripts.push(format!(
            "try{{ var s = {q}{body}{q}; encodeURIComponent(s) }}catch(e){{ e.name }}",
            q = q,
            body = body
        ));
        scripts.push(format!(
            "try{{ var s = {q}{body}{q}; s.toUpperCase()+'|'+s.toLowerCase() }}catch(e){{ e.name }}",
            q = q,
            body = body
        ));
    }
    compare_batch("string literals", &scripts);
}

#[test]
fn fuzz_regexp_literals_in_source() {
    let mut rng = Rng::new(0x9A31);
    const PIECES: &[&str] = &[
        "a", "b", ".", "*", "+", "?", "|", "(", ")", "[", "]", "{", "}", "^", "$", "\\d",
        "\\w", "\\s", "\\D", "\\W", "\\S", "\\b", "\\B", "\\n", "\\/", "\\\\", "\\x41",
        "\\u0041", "\\cA", "\\1", "\\0", "a-z", "0-9", "(?:", "(?=", "(?!", "{1,2}", "{2}",
        "{,2}", "[^", "]", "-",
    ];
    const FLAGS: &[&str] = &["", "g", "i", "m", "gi", "gm", "im", "gim", "x", "gg"];
    const SUBJ: &[&str] = &["''", "'a'", "'ab'", "'aaa'", "'A B'", "'a\\nb'", "'\\u00e9'", "'123'"];
    let mut scripts = Vec::new();
    for _ in 0..2500 {
        let n = 1 + rng.below(6);
        let mut p = String::new();
        for _ in 0..n {
            p.push_str(rng.pick(PIECES));
        }
        let fl = rng.pick(FLAGS);
        let s = rng.pick(SUBJ);
        // as a source-level literal (lexer path)
        scripts.push(format!(
            "try{{ var r = /{p}/{fl}; String(r)+'|'+String(r.test({s})) }}catch(e){{ e.name+':'+e.message }}",
            p = p,
            fl = fl,
            s = s
        ));
        // and via the constructor (jsregexp path)
        scripts.push(format!(
            "try{{ var r = new RegExp('{p}','{fl}'); String(r)+'|'+JSON.stringify(r.exec({s})) }}catch(e){{ e.name+':'+e.message }}",
            p = p.replace('\\', "\\\\"),
            fl = fl,
            s = s
        ));
    }
    compare_batch("regexp literals", &scripts);
}

#[test]
fn fuzz_property_descriptors() {
    let mut rng = Rng::new(0xDE5C);
    const BOOLS: &[&str] = &["true", "false", "undefined", "0", "1", "''", "'x'"];
    const VALS: &[&str] = &["1", "'s'", "undefined", "null", "{}", "[]"];
    let mut scripts = Vec::new();
    for _ in 0..1500 {
        let mut parts: Vec<String> = Vec::new();
        if rng.below(3) != 0 {
            parts.push(format!("value:{}", rng.pick(VALS)));
        }
        if rng.below(3) == 0 {
            parts.push("get:function(){return 'g'}".into());
        }
        if rng.below(3) == 0 {
            parts.push("set:function(v){this._v=v}".into());
        }
        if rng.below(2) == 0 {
            parts.push(format!("writable:{}", rng.pick(BOOLS)));
        }
        if rng.below(2) == 0 {
            parts.push(format!("enumerable:{}", rng.pick(BOOLS)));
        }
        if rng.below(2) == 0 {
            parts.push(format!("configurable:{}", rng.pick(BOOLS)));
        }
        let desc = parts.join(",");
        let target = rng.pick(&["{}", "[]", "[1,2]", "function(){}", "Object.create({p:1})"]);
        let key = rng.pick(&["'k'", "'0'", "'length'", "'p'", "0", "'toString'"]);
        scripts.push(format!(
            "try{{ var o={t}; Object.defineProperty(o,{k},{{{d}}}); \
             [JSON.stringify(Object.getOwnPropertyDescriptor(o,{k})), \
              Object.keys(o).join('/'), \
              Object.getOwnPropertyNames(o).sort().join('/'), \
              String(o[{k}])].join(';') }}catch(e){{ e.name }}",
            t = target,
            k = key,
            d = desc
        ));
        scripts.push(format!(
            "try{{ var o={t}; Object.defineProperty(o,{k},{{{d}}}); o[{k}]='new'; String(o[{k}])+'|'+String(o._v) }}catch(e){{ e.name }}",
            t = target,
            k = key,
            d = desc
        ));
        scripts.push(format!(
            "try{{ var o={t}; Object.defineProperty(o,{k},{{{d}}}); String(delete o[{k}])+'|'+({k} in o) }}catch(e){{ e.name }}",
            t = target,
            k = key,
            d = desc
        ));
    }
    compare_batch("property descriptors", &scripts);
}

#[test]
fn fuzz_receiver_method_matrix() {
    let mut rng = Rng::new(0x4E17);
    const RECEIVERS: &[&str] = &[
        "undefined", "null", "true", "1", "0", "NaN", "''", "'abc'", "[]", "[1,2,3]", "{}",
        "{a:1}", "function(){}", "/re/g", "new Date(0)", "new Number(1)", "new String('s')",
        "new Boolean(false)", "new Error('e')", "Math", "JSON", "arguments",
    ];
    const METHODS: &[&str] = &[
        "toString", "valueOf", "toLocaleString", "hasOwnProperty", "propertyIsEnumerable",
        "isPrototypeOf", "join", "concat", "slice", "indexOf", "lastIndexOf", "push", "pop",
        "shift", "unshift", "reverse", "sort", "splice", "forEach", "map", "filter", "every",
        "some", "reduce", "reduceRight", "charAt", "charCodeAt", "substring", "substr",
        "toUpperCase", "toLowerCase", "trim", "split", "replace", "search", "match",
        "localeCompare", "test", "exec", "toFixed", "toPrecision", "toExponential",
        "getTime", "getFullYear", "toISOString", "toJSON", "call", "apply", "bind",
    ];
    const ARGS: &[&str] = &["", "1", "'a'", "0,1", "/a/", "function(x){return x}", "undefined", "null", "{}"];
    let mut scripts = Vec::new();
    for _ in 0..6000 {
        let r = rng.pick(RECEIVERS);
        let m = rng.pick(METHODS);
        let a = rng.pick(ARGS);
        scripts.push(format!(
            "try{{ String(({r}).{m}({a})) }}catch(e){{ e.name }}",
            r = r,
            m = m,
            a = a
        ));
        // and through .call to hit generic (non-native-receiver) paths
        let host = rng.pick(&[
            "Array.prototype",
            "String.prototype",
            "Object.prototype",
            "Number.prototype",
            "Date.prototype",
            "RegExp.prototype",
            "Boolean.prototype",
            "Error.prototype",
        ]);
        scripts.push(format!(
            "try{{ String({h}.{m}.call({r}{sep}{a})) }}catch(e){{ e.name }}",
            h = host,
            m = m,
            r = r,
            sep = if a.is_empty() { "" } else { "," },
            a = a
        ));
    }
    compare_batch("receiver/method matrix", &scripts);
}

#[test]
fn fuzz_mixed_programs() {
    let mut rng = Rng::new(0x71C4);
    const DECLS: &[&str] = &[
        "var a=[1,2,3];",
        "var o={x:1,y:2};",
        "var s='hello';",
        "var n=42;",
        "var f=function(v){return v*2};",
        "var r=/l+/g;",
        "var d=new Date(0);",
        "function g(){return arguments.length}",
        "var e=new Error('e');",
    ];
    const STEPS: &[&str] = &[
        "out.push(a.join('-'));",
        "out.push(a.map(f).join('-'));",
        "a.push(a.length);",
        "a.sort(function(p,q){return q-p});",
        "out.push(JSON.stringify(o));",
        "o[s]=n; out.push(Object.keys(o).sort().join('/'));",
        "delete o.x; out.push('x' in o);",
        "out.push(s.replace(r,'L'));",
        "out.push(s.match(r)&&s.match(r).join(','));",
        "out.push(s.split('l').join('|'));",
        "out.push(n.toString(16));",
        "out.push((n/7).toFixed(3));",
        "out.push(d.toISOString());",
        "out.push(g(1,2,3));",
        "out.push(e.name+':'+e.message);",
        "out.push(typeof f);",
        "try{ null.x }catch(err){ out.push(err.name) }",
        "out.push([].concat(a,a).length);",
        "for(var k in o) out.push('k:'+k);",
        "out.push(Object.getOwnPropertyNames(o).sort().join('+'));",
        "out.push(String(a.indexOf(2)));",
        "a.length=2; out.push(a.join(','));",
        "out.push(encodeURIComponent(s));",
        "out.push(s.charCodeAt(1));",
        "n = n*2+1; out.push(n);",
        "out.push(Math.max.apply(null,a));",
        "out.push(a.reduce(function(p,q){return p+q},0));",
        "out.push(JSON.stringify(JSON.parse(JSON.stringify(o))));",
        "out.push(eval('n+1'));",
        "(function(){ var n=99; out.push(n) })();",
        "with(o){ out.push(typeof y) }",
        "out.push(a.slice(-2).join(','));",
        "out.push(r.lastIndex);",
        "out.push(String(r.exec(s)));",
    ];
    let mut scripts = Vec::new();
    for _ in 0..3000 {
        let mut prog = String::from("var out=[];");
        let nd = 1 + rng.below(DECLS.len());
        let mut used: Vec<&str> = Vec::new();
        for d in DECLS.iter().take(nd) {
            prog.push_str(d);
            used.push(d);
        }
        let ns = 1 + rng.below(8);
        for _ in 0..ns {
            prog.push_str(rng.pick(STEPS));
        }
        prog.push_str("out.join(';')");
        scripts.push(format!("try{{ {} }}catch(e){{ e.name+':'+e.message }}", prog));
    }
    compare_batch("mixed programs", &scripts);
}

#[test]
fn fuzz_numeric_source_literals() {
    let mut rng = Rng::new(0x1D07);
    let mut scripts = Vec::new();
    const PARTS: &[&str] = &[
        "0", "1", "9", ".", "e", "E", "+", "-", "x", "X", "b", "o", "f", "F", "_", "0x", "00",
        "1e", "e1", ".5", "5.", "8", "7",
    ];
    for _ in 0..3000 {
        let n = 1 + rng.below(6);
        let mut lit = String::new();
        for _ in 0..n {
            lit.push_str(rng.pick(PARTS));
        }
        scripts.push(format!("try{{ String({}) }}catch(e){{ e.name }}", lit));
        scripts.push(format!("try{{ String(-{}) }}catch(e){{ e.name }}", lit));
        scripts.push(format!("try{{ String(Number('{}')) }}catch(e){{ e.name }}", lit));
        scripts.push(format!("try{{ String(parseFloat('{}')) }}catch(e){{ e.name }}", lit));
        scripts.push(format!("try{{ String(parseInt('{}')) }}catch(e){{ e.name }}", lit));
        scripts.push(format!("try{{ String(JSON.parse('{}')) }}catch(e){{ e.name }}", lit));
    }
    compare_batch("numeric literals", &scripts);
}
