//! Phase B — differential tests for the built-in library, driven through
//! `js_dostring` and observed via the `__out` accumulator.
//!
//! CONFIGS.md rows 53-59 and 62.

mod common;

use common::*;

const SEED: u64 = 0xB01D_FACE_0000_0001;

/// Format a double as a JS expression that reproduces it exactly.
fn js_num(v: f64) -> String {
    if v.is_nan() {
        "NaN".into()
    } else if v == f64::INFINITY {
        "Infinity".into()
    } else if v == f64::NEG_INFINITY {
        "-Infinity".into()
    } else if v == 0.0 && v.is_sign_negative() {
        "(-0)".into()
    } else {
        // `{:?}` gives the shortest representation that round-trips.
        let s = format!("{v:?}");
        if s.starts_with('-') {
            format!("({s})")
        } else {
            s
        }
    }
}

fn number_corpus(rng: &mut Rng, n: usize) -> Vec<f64> {
    let mut v: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.1,
        1.0 / 3.0,
        2.0 / 3.0,
        1e-7,
        1e-6,
        1e20,
        1e21,
        1e-320,
        5e-324,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        4294967295.0,
        4294967296.0,
        9007199254740991.0,
        9007199254740992.0,
        123456789.0,
        1.005,
        2.5,
        -2.5,
        0.5 - f64::EPSILON,
        1234.5678,
        99999999999999999999.0,
        1e100,
        1e-100,
        255.0,
        256.0,
        65535.0,
        65536.0,
    ];
    for _ in 0..n {
        v.push(rng.double());
    }
    v
}

fn string_corpus(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut v: Vec<String> = [
        "",
        "a",
        "abc",
        "ABC",
        "  padded  ",
        "\t\n\r ",
        "0",
        "123",
        "-1.5e3",
        "0x1F",
        "NaN",
        "Infinity",
        "true",
        "null",
        "\u{e9}",
        "\u{4e2d}\u{6587}",
        "\u{1F600}\u{1F601}",
        "a\u{e9}b\u{4e2d}c",
        "line1\nline2",
        "quote\"and'apos",
        "back\\slash",
        "tab\there",
        "\u{0}embedded",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "The Quick Brown Fox Jumps Over The Lazy Dog",
        "\u{130}\u{131}\u{17F}\u{1E9E}\u{FB00}",
        "\u{FEFF}bom",
        "%20%41%E4%B8%AD",
        "a,b,,c,",
        "  ",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let alphabets: [&[char]; 4] = [
        &['a', 'b', 'c', 'A', 'B', ' ', '0', '1', '\n', '\t'],
        &['\u{e9}', '\u{4e2d}', '\u{1F600}', 'x', '\u{130}', '\u{17F}'],
        &['"', '\'', '\\', '/', '<', '>', '&', '%', '\u{0}'],
        &['0', '1', '2', '.', 'e', '+', '-', 'x', 'A', 'F'],
    ];
    for _ in 0..n {
        let ab = alphabets[rng.below(4) as usize];
        let len = rng.below(14) as usize;
        v.push(
            (0..len)
                .map(|_| ab[rng.below(ab.len() as u32) as usize])
                .collect(),
        );
    }
    v
}

/// Escape a Rust string into a JS single-quoted literal using only `\uXXXX`
/// escapes so the generated source is pure ASCII and unambiguous.
fn js_str(s: &str) -> String {
    let mut out = String::from("'");
    for ch in s.chars() {
        let c = ch as u32;
        if c < 0x80 && ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if c <= 0xFFFF {
            out.push_str(&format!("\\u{c:04X}"));
        } else {
            // JS (ES5) strings are UTF-16; emit a surrogate pair
            let v = c - 0x10000;
            out.push_str(&format!(
                "\\u{:04X}\\u{:04X}",
                0xD800 + (v >> 10),
                0xDC00 + (v & 0x3FF)
            ));
        }
    }
    out.push('\'');
    out
}

// ===========================================================================
// CONFIGS.md row 53: Number.prototype.toString for every radix
// ===========================================================================

#[test]
fn builtin_number_tostring_radix() {
    let mut rng = Rng::new(SEED);
    let nums = number_corpus(&mut rng, 400);
    for flags in [0, 1] {
        for radix in 2..=36 {
            let mut src = String::new();
            for v in &nums {
                src.push_str(&format!(
                    "ok(function(){{ return ({}).toString({radix}) }});\n",
                    js_num(*v)
                ));
            }
            assert_same_program(flags, &format!("toString radix {radix}"), &src);
        }
        // out-of-range radices must produce the same RangeError
        for radix in ["0", "1", "37", "-1", "1.5", "NaN", "Infinity", "'16'", "null", "undefined"] {
            let mut src = String::new();
            for v in nums.iter().take(40) {
                src.push_str(&format!(
                    "ok(function(){{ return ({}).toString({radix}) }});\n",
                    js_num(*v)
                ));
            }
            assert_same_program(flags, &format!("toString bad radix {radix}"), &src);
        }
        // parseInt round trip for every radix
        let mut src = String::new();
        for radix in 2..=36 {
            for v in nums.iter().take(60) {
                src.push_str(&format!(
                    "ok(function(){{ return parseInt(({}).toString({radix}), {radix}) }});\n",
                    js_num(*v)
                ));
            }
        }
        assert_same_program(flags, "parseInt round trip", &src);
    }
}

// ===========================================================================
// CONFIGS.md row 54: toFixed / toExponential / toPrecision
// ===========================================================================

#[test]
fn builtin_number_formatting_precision() {
    let mut rng = Rng::new(SEED ^ 0x11);
    let nums = number_corpus(&mut rng, 300);
    for flags in [0, 1] {
        // toFixed: 0..20 valid, -1 / 21 out of range
        for w in -1i32..=21 {
            let mut src = String::new();
            for v in &nums {
                src.push_str(&format!(
                    "ok(function(){{ return ({}).toFixed({w}) }});\n",
                    js_num(*v)
                ));
            }
            assert_same_program(flags, &format!("toFixed {w}"), &src);
        }
        // toExponential: 0..20 valid, -1 / 21 out of range; also undefined
        for w in -1i32..=21 {
            let mut src = String::new();
            for v in &nums {
                src.push_str(&format!(
                    "ok(function(){{ return ({}).toExponential({w}) }});\n",
                    js_num(*v)
                ));
            }
            assert_same_program(flags, &format!("toExponential {w}"), &src);
        }
        // toPrecision: 1..21 valid, 0 / 22 out of range
        for w in 0i32..=22 {
            let mut src = String::new();
            for v in &nums {
                src.push_str(&format!(
                    "ok(function(){{ return ({}).toPrecision({w}) }});\n",
                    js_num(*v)
                ));
            }
            assert_same_program(flags, &format!("toPrecision {w}"), &src);
        }
        // no-argument forms and non-numeric arguments
        let mut src = String::new();
        for v in nums.iter().take(120) {
            let n = js_num(*v);
            for call in [
                "toFixed()",
                "toExponential()",
                "toPrecision()",
                "toFixed(undefined)",
                "toExponential(undefined)",
                "toPrecision(undefined)",
                "toFixed(null)",
                "toFixed('3')",
                "toFixed(2.9)",
                "toFixed(NaN)",
                "toString()",
                "valueOf()",
                "toLocaleString()",
            ] {
                src.push_str(&format!("ok(function(){{ return ({n}).{call} }});\n"));
            }
        }
        assert_same_program(flags, "number formatting defaults", &src);
    }
}

/// CONFIGS.md row 62: `String(n)` / implicit number-to-string for many doubles.
#[test]
fn builtin_number_to_string_conversion() {
    let mut rng = Rng::new(SEED ^ 0x22);
    let nums = number_corpus(&mut rng, 4000);
    for flags in [0, 1] {
        for chunk in nums.chunks(500) {
            let mut src = String::new();
            for v in chunk {
                let n = js_num(*v);
                src.push_str(&format!("o({n}); o(String({n})); o(''+{n}); o(({n}).toString()); o(-{n}); o(1/{n});\n"));
            }
            assert_same_program(flags, "number to string", &src);
        }
        // Number(...) parsing of the string corpus
        let strs = string_corpus(&mut rng, 400);
        let mut src = String::new();
        for s in &strs {
            let l = js_str(s);
            src.push_str(&format!(
                "o(Number({l})); o(parseFloat({l})); o(parseInt({l})); o(parseInt({l},16)); o(+{l});\n"
            ));
        }
        assert_same_program(flags, "string to number", &src);
    }
}

// ===========================================================================
// CONFIGS.md row 55: String methods
// ===========================================================================

#[test]
fn builtin_string_methods() {
    let mut rng = Rng::new(SEED ^ 0x33);
    let strs = string_corpus(&mut rng, 220);
    let unary = [
        "length",
        "toString()",
        "valueOf()",
        "toUpperCase()",
        "toLowerCase()",
        "toLocaleUpperCase()",
        "toLocaleLowerCase()",
        "trim()",
        "split('')",
        "split()",
        "split(',')",
        "split(/[,;]/)",
        "split('',3)",
        "concat('X')",
        "concat('X','Y')",
    ];
    for flags in [0, 1] {
        for chunk in strs.chunks(40) {
            let mut src = String::new();
            for s in chunk {
                let l = js_str(s);
                for m in unary {
                    src.push_str(&format!("ok(function(){{ return {l}.{m} }});\n"));
                }
                // index-taking methods across boundaries
                for i in [-3i32, -1, 0, 1, 2, 5, 100] {
                    for m in [
                        "charAt",
                        "charCodeAt",
                        "indexOf",
                        "lastIndexOf",
                        "slice",
                        "substring",
                        "substr",
                    ] {
                        src.push_str(&format!("ok(function(){{ return {l}.{m}({i}) }});\n"));
                    }
                }
                for (i, k) in [
                    (0i32, 0i32),
                    (0, 1),
                    (1, 3),
                    (-2, -1),
                    (3, 1),
                    (0, 100),
                    (-100, 100),
                ] {
                    for m in ["slice", "substring", "substr"] {
                        src.push_str(&format!("ok(function(){{ return {l}.{m}({i},{k}) }});\n"));
                    }
                }
                src.push_str(&format!(
                    "ok(function(){{ return {l}.replace('a','Z') }});\n\
                     ok(function(){{ return {l}.replace(/a/g,'Z') }});\n\
                     ok(function(){{ return {l}.replace(/(.)(.)/g,'$2$1') }});\n\
                     ok(function(){{ return {l}.replace(/a/g, function(m){{ return '['+m+']' }}) }});\n\
                     ok(function(){{ return {l}.match(/[a-z]+/g) }});\n\
                     ok(function(){{ return {l}.search(/b/) }});\n\
                     ok(function(){{ return {l}.localeCompare('abc') }});\n\
                     ok(function(){{ return escape({l}) }});\n\
                     ok(function(){{ return unescape({l}) }});\n\
                     ok(function(){{ return encodeURI({l}) }});\n\
                     ok(function(){{ return encodeURIComponent({l}) }});\n\
                     ok(function(){{ return decodeURI({l}) }});\n\
                     ok(function(){{ return decodeURIComponent({l}) }});\n\
                     ok(function(){{ return JSON.stringify({l}) }});\n"
                ));
            }
            assert_same_program(flags, "string methods", &src);
        }
        // String.fromCharCode over interesting code units
        let mut src = String::new();
        for c in [
            "0", "1", "65", "97", "127", "128", "255", "256", "0xD800", "0xDC00",
            "0xFFFF", "0x10000", "-1", "NaN", "Infinity", "1.9", "65536.5",
        ] {
            src.push_str(&format!(
                "ok(function(){{ return String.fromCharCode({c}) }});\n\
                 ok(function(){{ return String.fromCharCode({c},{c}) }});\n"
            ));
        }
        src.push_str("ok(function(){ return String.fromCharCode() });\n");
        assert_same_program(flags, "fromCharCode", &src);
    }
}

// ===========================================================================
// CONFIGS.md row 56: Array methods
// ===========================================================================

#[test]
fn builtin_array_methods() {
    let mut rng = Rng::new(SEED ^ 0x44);
    let shapes = [
        "[]",
        "[1]",
        "[1,2,3]",
        "[3,1,2]",
        "[1,2,3,4,5,6,7,8,9,10]",
        "[1,,3]",
        "new Array(5)",
        "['b','a','c']",
        "[10,9,80,1000,1]",
        "[true,false,null,undefined,0,'',NaN]",
        "[[1,2],[3,4]]",
        "[{a:1},{a:2}]",
        "['\\u00e9','\\u4e2d','a']",
    ];
    let mut random_shapes: Vec<String> = Vec::new();
    for _ in 0..40 {
        let n = rng.below(12) as usize;
        let items: Vec<String> = (0..n)
            .map(|_| match rng.below(6) {
                0 => js_num(rng.double()),
                1 => js_str(&format!("s{}", rng.below(20))),
                2 => "null".into(),
                3 => "undefined".into(),
                4 => format!("{}", rng.range_i32(-50, 50)),
                _ => format!("{}", rng.bool()),
            })
            .collect();
        random_shapes.push(format!("[{}]", items.join(",")));
    }

    let methods = [
        "join()",
        "join('-')",
        "join('')",
        "toString()",
        "concat([9])",
        "concat(1,[2],[[3]])",
        "slice()",
        "slice(1)",
        "slice(1,3)",
        "slice(-2)",
        "slice(-2,-1)",
        "slice(100)",
        "indexOf(1)",
        "indexOf(1,2)",
        "lastIndexOf(1)",
        "reverse()",
        "sort()",
        "sort(function(a,b){ return a<b?-1:a>b?1:0 })",
        "sort(function(a,b){ return b-a })",
        "sort(function(){ return 0 })",
        "every(function(x){ return !!x })",
        "some(function(x){ return !!x })",
        "filter(function(x){ return !!x })",
        "map(function(x){ return typeof x })",
        "forEach(function(x,i){ __out += i })",
        "reduce(function(a,b){ return String(a)+String(b) })",
        "reduce(function(a,b){ return String(a)+String(b) }, 'I')",
        "reduceRight(function(a,b){ return String(a)+String(b) })",
        "reduceRight(function(a,b){ return String(a)+String(b) }, 'I')",
        "length",
    ];
    let mutators = [
        "push(9)",
        "push(9,10)",
        "push()",
        "pop()",
        "shift()",
        "unshift(0)",
        "unshift()",
        "splice(1,1)",
        "splice(1,0,'x')",
        "splice(-1,1)",
        "splice(0,100)",
        "splice()",
    ];

    for flags in [0, 1] {
        let all: Vec<&str> = shapes
            .iter()
            .copied()
            .chain(random_shapes.iter().map(|s| s.as_str()))
            .collect();
        for chunk in all.chunks(10) {
            let mut src = String::new();
            for sh in chunk {
                for m in methods {
                    src.push_str(&format!("ok(function(){{ var a = {sh}; return a.{m} }});\n"));
                }
                for m in mutators {
                    src.push_str(&format!(
                        "ok(function(){{ var a = {sh}; var r = a.{m}; return String(r)+'|'+String(a)+'|'+a.length }});\n"
                    ));
                }
                src.push_str(&format!(
                    "ok(function(){{ return Array.isArray({sh}) }});\n\
                     ok(function(){{ return JSON.stringify({sh}) }});\n\
                     ok(function(){{ var a={sh}; var s=''; for (var k in a) s += k+','; return s }});\n"
                ));
            }
            assert_same_program(flags, "array methods", &src);
        }
        // sort stability / comparator misbehaviour
        let mut src = String::new();
        for cmp in [
            "undefined",
            "null",
            "function(){ return NaN }",
            "function(a,b){ return a-b }",
            "function(a,b){ return b-a }",
            "function(){ throw 'cmp' }",
            "1",
            "'x'",
        ] {
            src.push_str(&format!(
                "ok(function(){{ var a=[5,3,9,1,3,7,0,-2]; a.sort({cmp}); return String(a) }});\n"
            ));
        }
        assert_same_program(flags, "sort comparators", &src);
    }
}

// ===========================================================================
// CONFIGS.md row 57: Math
// ===========================================================================

#[test]
fn builtin_math() {
    let mut rng = Rng::new(SEED ^ 0x55);
    let nums = number_corpus(&mut rng, 600);
    let unary = [
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log",
        "round", "sin", "sqrt", "tan",
    ];
    for flags in [0, 1] {
        for chunk in nums.chunks(200) {
            let mut src = String::new();
            for v in chunk {
                let n = js_num(*v);
                for f in unary {
                    src.push_str(&format!("o(Math.{f}({n}));"));
                }
                src.push('\n');
            }
            assert_same_program(flags, "math unary", &src);
        }
        // binary and variadic
        let pairs: Vec<(f64, f64)> = {
            let mut v = Vec::new();
            for i in 0..300 {
                v.push((nums[i % nums.len()], nums[(i * 7 + 3) % nums.len()]));
            }
            v
        };
        for chunk in pairs.chunks(100) {
            let mut src = String::new();
            for (a, b) in chunk {
                let (x, y) = (js_num(*a), js_num(*b));
                src.push_str(&format!(
                    "o(Math.pow({x},{y})); o(Math.atan2({x},{y})); o(Math.min({x},{y})); o(Math.max({x},{y})); o({x}%{y}); o({x}/{y}); o({x}*{y}); o({x}+{y}); o({x}-{y});\n"
                ));
            }
            assert_same_program(flags, "math binary", &src);
        }
        let mut src = String::from(
            "o(Math.min()); o(Math.max()); o(Math.min(1)); o(Math.max(1)); \
             o(Math.min(1,2,3)); o(Math.max(1,2,3)); o(Math.min(NaN,1)); o(Math.max(NaN,1)); \
             o(Math.E); o(Math.LN10); o(Math.LN2); o(Math.LOG10E); o(Math.LOG2E); \
             o(Math.PI); o(Math.SQRT1_2); o(Math.SQRT2);\n",
        );
        // Math.random is seeded from the state; verify the same sequence
        src.push_str("for (var i=0;i<50;++i) o(Math.random());\n");
        assert_same_program(flags, "math constants and random", &src);
    }
}

// ===========================================================================
// CONFIGS.md row 58: JSON
// ===========================================================================

#[test]
fn builtin_json() {
    let mut rng = Rng::new(SEED ^ 0x66);
    let values = [
        "undefined",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "1e21",
        "5e-324",
        "''",
        "'a'",
        "'\\u00e9\\u4e2d'",
        "'\\u0000\\u0001\\u001f'",
        "'\\\"\\\\\\/\\b\\f\\n\\r\\t'",
        "'\\ud800'",
        "[]",
        "[1,2,3]",
        "[1,[2,[3,[4]]]]",
        "[undefined,null,function(){}]",
        "{}",
        "{a:1}",
        "{a:{b:{c:[1,2]}}}",
        "{a:undefined,b:function(){},c:1}",
        "{'':1}",
        "new Date(0)",
        "new Number(5)",
        "new String('s')",
        "new Boolean(true)",
        "/re/g",
        "function(){}",
        "Math",
        "{toJSON:function(){ return 'custom' }}",
        "[{toJSON:function(){ return 1 }}]",
    ];
    let texts = [
        "null", "true", "false", "0", "-0", "1", "-1", "1.5", "1e3", "1E3",
        "1e+3", "1e-3", "\"\"", "\"a\"", "\"\\u00e9\"", "\"\\n\"", "\"\\\\\"",
        "[]", "[1]", "[1,2]", "[1,[2]]", "{}", "{\"a\":1}", "{\"a\":{\"b\":2}}",
        "  1  ", "01", "1.", ".1", "+1", "'a'", "{a:1}", "[1,]", "{\"a\":1,}",
        "nan", "NaN", "Infinity", "undefined", "", " ", "[1 2]", "{\"a\" 1}",
        "\"unterminated", "[", "{", "]", "}", "1 2", "\"\\x41\"", "\"\\u00\"",
        "1e", "0x10", "tru", "\"\\uD800\"",
    ];
    let replacers = [
        "undefined",
        "null",
        "function(k,v){ return v }",
        "function(k,v){ return typeof v === 'number' ? v+1 : v }",
        "function(k,v){ return k === 'skip' ? undefined : v }",
        "['a','b']",
        "[]",
        "1",
    ];
    let indents = ["undefined", "0", "1", "2", "10", "11", "-1", "'  '", "'\\t'", "'0123456789ab'", "null", "true"];

    for flags in [0, 1] {
        let mut src = String::new();
        for v in values {
            for r in replacers {
                for i in indents {
                    src.push_str(&format!(
                        "ok(function(){{ return JSON.stringify({v},{r},{i}) }});\n"
                    ));
                }
            }
        }
        assert_same_program(flags, "JSON.stringify", &src);

        let mut src = String::new();
        for t in texts {
            let l = js_str(t);
            src.push_str(&format!(
                "ok(function(){{ return JSON.stringify(JSON.parse({l})) }});\n\
                 ok(function(){{ return JSON.parse({l}, function(k,v){{ return v }}) }});\n\
                 ok(function(){{ return JSON.parse({l}, function(k,v){{ return typeof v==='number'?v*2:v }}) }});\n"
            ));
        }
        assert_same_program(flags, "JSON.parse", &src);

        // cyclic structures must produce the same TypeError
        let src = "ok(function(){ var a=[]; a[0]=a; return JSON.stringify(a) });\n\
                   ok(function(){ var o={}; o.s=o; return JSON.stringify(o) });\n\
                   ok(function(){ var a=[], b={x:a}; a.push(b); return JSON.stringify(a) });\n";
        assert_same_program(flags, "JSON cycles", src);

        // randomized JSON round trips
        let mut src = String::new();
        for _ in 0..300 {
            let v = random_json(&mut rng, 0);
            src.push_str(&format!(
                "ok(function(){{ return JSON.stringify(JSON.parse(JSON.stringify({v}))) }});\n"
            ));
        }
        assert_same_program(flags, "JSON round trip", &src);
    }
}

fn random_json(rng: &mut Rng, depth: u32) -> String {
    match rng.below(if depth >= 3 { 6 } else { 8 }) {
        0 => "null".into(),
        1 => format!("{}", rng.bool()),
        2 => js_num(rng.double()),
        3 => format!("{}", rng.range_i32(-1000, 1000)),
        4 => js_str(&format!("s{}", rng.below(1000))),
        5 => js_str(&["", "\u{e9}", "\u{4e2d}", "\"", "\\", "\n", "\u{1F600}"][rng.below(7) as usize]),
        6 => {
            let n = rng.below(4);
            let items: Vec<String> = (0..n).map(|_| random_json(rng, depth + 1)).collect();
            format!("[{}]", items.join(","))
        }
        _ => {
            let n = rng.below(4);
            let items: Vec<String> = (0..n)
                .map(|i| format!("'k{i}':{}", random_json(rng, depth + 1)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

// ===========================================================================
// CONFIGS.md row 59: Date  (TZ pinned so the C and Rust runs agree)
// ===========================================================================

#[test]
fn builtin_date() {
    // Both libraries call localtime/mktime, so pin the timezone for the process.
    unsafe {
        std::env::set_var("TZ", "UTC");
    }
    unsafe extern "C" {
        fn tzset();
    }
    unsafe {
        tzset();
    }

    let mut rng = Rng::new(SEED ^ 0x77);
    let mut stamps: Vec<String> = [
        "0",
        "1",
        "-1",
        "1000",
        "-1000",
        "86400000",
        "1234567890000",
        "-2208988800000",
        "8640000000000000",
        "8640000000000001",
        "-8640000000000000",
        "-8640000000000001",
        "NaN",
        "Infinity",
        "-Infinity",
        "1.5",
        "0.5",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for _ in 0..120 {
        stamps.push(format!("{}", rng.range_i32(i32::MIN, i32::MAX) as i64 * 1000));
    }

    let getters = [
        "getTime", "valueOf", "getFullYear", "getMonth", "getDate", "getDay",
        "getHours", "getMinutes", "getSeconds", "getMilliseconds",
        "getUTCFullYear", "getUTCMonth", "getUTCDate", "getUTCDay",
        "getUTCHours", "getUTCMinutes", "getUTCSeconds", "getUTCMilliseconds",
        "getTimezoneOffset", "toString", "toDateString", "toTimeString",
        "toUTCString", "toISOString", "toJSON", "toLocaleString",
        "toLocaleDateString", "toLocaleTimeString",
    ];
    for flags in [0, 1] {
        for chunk in stamps.chunks(30) {
            let mut src = String::new();
            for s in chunk {
                for g in getters {
                    src.push_str(&format!("ok(function(){{ return new Date({s}).{g}() }});\n"));
                }
            }
            assert_same_program(flags, "date getters", &src);
        }
        // constructors and parsing
        let mut src = String::new();
        for ctor in [
            "new Date(2000,0)",
            "new Date(2000,0,1)",
            "new Date(2000,0,1,12)",
            "new Date(2000,0,1,12,30)",
            "new Date(2000,0,1,12,30,45)",
            "new Date(2000,0,1,12,30,45,678)",
            "new Date(1970,0,1)",
            "new Date(99,11,31)",
            "new Date(-1,0,1)",
            "new Date(275760,8,13)",
            "new Date(275760,8,14)",
            "new Date(NaN,0)",
            "new Date(2000,13,32,25,61,61,1001)",
            "new Date('2000-01-01')",
            "new Date('2000-01-01T00:00:00Z')",
            "new Date('2000-01-01T12:34:56.789Z')",
            "new Date('not a date')",
            "new Date('')",
            "new Date(undefined)",
            "new Date(null)",
            "new Date(true)",
            "new Date([])",
            "new Date({})",
        ] {
            src.push_str(&format!(
                "ok(function(){{ return {ctor}.getTime() }});\nok(function(){{ return {ctor}.toISOString() }});\n"
            ));
        }
        for p in [
            "'2000-01-01'",
            "'2000-01-01T00:00:00Z'",
            "'2000-01-01T00:00:00.000Z'",
            "'1970-01-01T00:00:00Z'",
            "'bogus'",
            "''",
            "'0'",
            "'2000'",
            "'2000-13-01'",
        ] {
            src.push_str(&format!("ok(function(){{ return Date.parse({p}) }});\n"));
        }
        src.push_str(
            "ok(function(){ return Date.UTC(2000,0,1) });\n\
             ok(function(){ return Date.UTC(2000,0,1,12,30,45,678) });\n\
             ok(function(){ return Date.UTC() });\n\
             ok(function(){ return typeof Date.now() });\n\
             ok(function(){ return typeof new Date().getTime() });\n",
        );
        // setters
        for s in [
            "setTime(0)",
            "setMilliseconds(500)",
            "setSeconds(30)",
            "setMinutes(30)",
            "setHours(5)",
            "setDate(15)",
            "setMonth(6)",
            "setFullYear(1999)",
            "setUTCMilliseconds(500)",
            "setUTCSeconds(30)",
            "setUTCMinutes(30)",
            "setUTCHours(5)",
            "setUTCDate(15)",
            "setUTCMonth(6)",
            "setUTCFullYear(1999)",
            "setYear(99)",
            "setTime(NaN)",
            "setFullYear(NaN)",
        ] {
            src.push_str(&format!(
                "ok(function(){{ var d = new Date(0); d.{s}; return d.getTime()+'/'+String(d.toISOString ? (isNaN(d.getTime())?'NaN':d.toISOString()) : '') }});\n"
            ));
        }
        // toISOString on an invalid date must throw the same RangeError
        src.push_str("ok(function(){ return new Date(NaN).toISOString() });\n");
        src.push_str("ok(function(){ return new Date(NaN).toJSON() });\n");
        src.push_str("ok(function(){ return Date.prototype.toISOString.call({}) });\n");
        assert_same_program(flags, "date constructors and setters", &src);
    }
}
