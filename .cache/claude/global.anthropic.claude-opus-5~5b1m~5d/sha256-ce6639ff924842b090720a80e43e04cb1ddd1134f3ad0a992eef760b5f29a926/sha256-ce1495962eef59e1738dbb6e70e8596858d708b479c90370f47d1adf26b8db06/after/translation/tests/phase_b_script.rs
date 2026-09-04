//! Phase B — VALID-PATH differential tests driven from **JS source** (the rows of
//! `CONFIGS.md` that are reachable only through the language / builtin library
//! rather than the raw C API).
//!
//! The raw-API rows are covered by `tests/phase_b_api.rs` and
//! `tests/phase_b_lowlevel.rs`; this file deliberately never touches the raw API
//! except through `diff_eval`, which runs the *same source bytes* through both
//! `.so` files and compares the rendered result byte for byte.
//!
//! Style: property-based. Every area builds a large set of randomized inputs
//! from a FIXED seed, formats them into one script that loops over
//! `inputs x expressions` and joins every result into a single string, so a few
//! thousand cases cost one `js_newstate` per library.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_int;

const SEED: u64 = 0x5EED_0B_5C_71_9701;

/* ------------------------------------------------------------------ helpers */

/// Escape an arbitrary (NUL-free) Rust string into a JS double-quoted literal.
/// Bytes >= 0x80 are emitted RAW (as UTF-8) so that exactly the same bytes reach
/// the lexer of both libraries; `\uXXXX` is only used for C0 controls, which the
/// mujs lexer turns back into the identical single byte.
fn jstr(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                o.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// A parenthesised JS numeric literal for `x`. The decimal text is what reaches
/// both libraries, so the exact double both of them see is by construction the
/// same (whatever `js_strtod` makes of it).
fn jsnum(x: f64) -> String {
    if x.is_nan() {
        "(NaN)".to_string()
    } else if x == f64::INFINITY {
        "(Infinity)".to_string()
    } else if x == f64::NEG_INFINITY {
        "(-Infinity)".to_string()
    } else if x == 0.0 {
        if x.is_sign_negative() {
            "(-0)".to_string()
        } else {
            "(0)".to_string()
        }
    } else {
        format!("({:e})", x)
    }
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Build the batch script: `inputs` are expressions (wrapped in a thunk so every
/// expression gets a FRESH object and mutating methods can not leak between
/// cases), `exprs` are expressions in the single parameter `x`.
fn mk_script(inputs: &[String], exprs: &[String]) -> String {
    let mut s = String::from("var MK=[");
    for (k, v) in inputs.iter().enumerate() {
        if k > 0 {
            s.push(',');
        }
        s.push_str("function(){return (");
        s.push_str(v);
        s.push_str(");}");
    }
    s.push_str("];var FS=[");
    for (k, e) in exprs.iter().enumerate() {
        if k > 0 {
            s.push(',');
        }
        s.push_str("function(x){return (");
        s.push_str(e);
        s.push_str(");}");
    }
    s.push_str("];var out=[];for(var j=0;j<FS.length;j++){for(var i=0;i<MK.length;i++){");
    s.push_str("try{out.push(j+'/'+i+'='+String(FS[j](MK[i]())));}");
    s.push_str("catch(e){out.push(j+'/'+i+'!'+String(e));}}}out.join('~')");
    s
}

/// Run one batch in both libraries. On divergence, re-run every single
/// (input, expr) pair to report the smallest failing case exactly.
#[track_caller]
fn diff_batch(label: &str, inputs: &[String], exprs: &[String], flags: c_int) {
    let p = libs();
    let src = mk_script(inputs, exprs);
    let c = p.c.eval(&src, flags);
    let r = p.r.eval(&src, flags);
    if c == r {
        /* guard against a silently mistyped generated script */
        assert!(
            c.starts_with("ok "),
            "batch [{}] did not run: {}\nsrc={}",
            label,
            c,
            src
        );
        return;
    }
    for (j, e) in exprs.iter().enumerate() {
        for (i, inp) in inputs.iter().enumerate() {
            let s1 = mk_script(std::slice::from_ref(inp), std::slice::from_ref(e));
            let c1 = p.c.eval(&s1, flags);
            let r1 = p.r.eval(&s1, flags);
            if c1 != r1 {
                panic!(
                    "DIVERGENCE [{}] flags={}\n  expr#{} = {}\n  input#{} = {}\n  src  = {}\n  C    : {:?}\n  RUST : {:?}",
                    label, flags, j, e, i, inp, s1, c1, r1
                );
            }
        }
    }
    same(
        &format!("{} (whole batch only) flags={} src={}", label, flags, src),
        &c,
        &r,
    );
}

/// `diff_eval` plus a self-check that the source really compiled and ran in the
/// C library (so a typo in a generated script can never masquerade as a pass by
/// producing the same SyntaxError in both libraries).
#[track_caller]
fn diff_ok(label: &str, src: &str, flags: c_int) {
    let p = libs();
    let c = p.c.eval(src, flags);
    let r = p.r.eval(src, flags);
    same(&format!("{} | flags={} | src={:?}", label, flags, src), &c, &r);
    assert!(
        c.starts_with("ok "),
        "[{}] source did not run: {}\nsrc={}",
        label,
        c,
        src
    );
}

/// Compare the `js_repr` rendering of an array literal of `elems`: this drives
/// `reprvalue`/`reprnum`/`reprstr` for every element in a single eval.
#[track_caller]
fn diff_repr(label: &str, elems: &[String], flags: c_int) {
    let src = format!("[{}]", elems.join(","));
    diff_ok(label, &src, flags);
}

/// Hand-written sources that are DELIBERATELY not parseable (the differential
/// check is then on the SyntaxError text itself).
const PARSE_ERROR_OK: [&str; 3] = [
    "({a:1,a:2})",
    "({get a(){return 1},get a(){return 2}})",
    "({a:1,'a':2})",
];

/// Every source in `srcs` under both flag settings, with the same
/// "it really compiled" self-check as `diff_ok` (only at `flags == 0`: several
/// sources are deliberately rejected by the strict-mode parser).
#[track_caller]
fn diff_each(label: &str, srcs: &[&str]) {
    let p = libs();
    for f in [0, JS_STRICT] {
        for s in srcs {
            let c = p.c.eval(s, f);
            let r = p.r.eval(s, f);
            same(&format!("{} | flags={} | src={:?}", label, f, s), &c, &r);
            if f == 0 && c.starts_with("load-error") && !PARSE_ERROR_OK.contains(s) {
                panic!("[{}] hand-written source does not compile: {:?} -> {}", label, s, c);
            }
        }
    }
}

/// Freeze the timezone for the Date rows. `LocalTZA()` is computed once per
/// library and cached in a function-local static, so TZ must be settled BEFORE
/// either library evaluates its first Date expression — this helper does that
/// and verifies both libraries agree on the resulting offset.
fn tz_init() {
    use std::sync::OnceLock;
    static ONCE: OnceLock<String> = OnceLock::new();
    ONCE.get_or_init(|| {
        std::env::set_var("TZ", "America/New_York");
        let p = libs();
        let src = "new Date(0).getTimezoneOffset()+'/'+new Date(1500000000000).getTimezoneOffset()";
        let c = p.c.eval(src, 0);
        let r = p.r.eval(src, 0);
        same("tz init (LocalTZA cache)", &c, &r);
        c
    });
}

/* =================================================================== STRINGS */

/* rows 95-102: shrstr (<=15) vs memstr (>=16) vs literal, and the string OBJECT
 * representations, all reached from JS source. */
#[test]
fn strings_representation_boundaries() {
    let mut inputs: Vec<String> = Vec::new();
    for n in 0..40usize {
        inputs.push(jstr(&"a".repeat(n)));
    }
    /* multi-byte: byte length crosses 15/16 well before the rune count does */
    for n in 0..12usize {
        inputs.push(jstr(&"\u{e9}".repeat(n)));
        inputs.push(jstr(&"\u{4e2d}".repeat(n)));
        inputs.push(jstr(&"\u{1f600}".repeat(n)));
    }
    inputs.push("new String('short')".to_string());
    inputs.push("new String('a string longer than fifteen')".to_string());
    inputs.push("new String('')".to_string());
    inputs.push("String('')".to_string());

    let exprs = strs(&[
        "x.length",
        "typeof x",
        "String(x)",
        "JSON.stringify(x)",
        "x+'#'+x.length",
        /* concatenation crossing the shrstr/memstr boundary */
        "(x+'0123456789abcdef').length",
        "(x+x)+'|'+(x+x).length",
        "x===String(x)",
        "x==String(x)",
        "(''+x).length",
        "x.valueOf().length",
        "x.toString().length",
        "Object.prototype.toString.call(x)",
        "x.charAt(0)+'/'+x.charAt(x.length-1)+'/'+x.charAt(x.length)",
        "x.substring(0,15)+'|'+x.substring(15)",
        "x.slice(-16)",
    ]);
    for f in [0, JS_STRICT] {
        diff_batch("string reps", &inputs, &exprs, f);
    }
}

/* rows 99,100: js_utflen / js_runeat — 1/2/3/4-byte UTF-8, surrogate synthesis
 * for astral runes, and index past the end. */
#[test]
fn strings_utf8_runes_and_surrogates() {
    let mut rng = Rng::new(SEED ^ 0x99);
    let mut inputs = strs(&[
        r#""abc""#,
        r#""\u00e9llo""#,
        r#""\u65e5\u672c\u8a9e""#,
        "\"\u{1f600}\"",
        "\"a\u{1f600}b\u{4e2d}c\u{e9}\"",
        r#""\uD800""#,
        r#""\uDFFF""#,
        r#""\uD83D\uDE00""#,
        r#""\u0080\u07ff\u0800\uffff""#,
        r#"String.fromCharCode(0xD83D,0xDE00)"#,
        r#"String.fromCharCode(65,0x80,0x7ff,0x800,0xffff)"#,
        r#""""#,
    ]);
    for _ in 0..120 {
        inputs.push(jstr(&rng.string(10)));
    }
    let exprs = strs(&[
        "x.length",
        /* every rune index, plus two past the end */
        "(function(){var s='';for(var i=0;i<x.length+2;i++)s+=x.charCodeAt(i)+',';return s})()",
        "(function(){var s='';for(var i=0;i<x.length+2;i++)s+='['+x.charAt(i)+']';return s})()",
        "(function(){var s='';for(var i=-2;i<2;i++)s+=x.charCodeAt(i)+',';return s})()",
        /* rebuild from char codes: astral runes come back as a surrogate pair */
        "(function(){var s='';for(var i=0;i<x.length;i++)s+=String.fromCharCode(x.charCodeAt(i));return s+'#'+s.length})()",
        "encodeURIComponent(x)",
        "escape===undefined",
        "x.split('').length",
        "x.split('').join('.')",
        "x.indexOf('\\u00e9')+'/'+x.lastIndexOf('\\u00e9')",
        "x.toUpperCase()+'|'+x.toLowerCase()",
        "x.toUpperCase().length+'/'+x.toLowerCase().length",
        "JSON.stringify(x)",
        "JSON.parse(JSON.stringify(x))===x",
        "decodeURIComponent(encodeURIComponent(x))===x",
        "encodeURI(x)",
        "(function(){try{return decodeURI(x)}catch(e){return 'E:'+e.name+':'+e.message}})()",
        "(function(){try{return decodeURIComponent(x)}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return decodeURIComponent('%'+x)}catch(e){return 'E:'+e.name}})()",
        "(function(){var s='';for(var i=0;i<x.length;i++)s+=x.charCodeAt(i).toString(16)+'.';return s})()",
        "String.fromCharCode.apply(null,[].map?[]:[])+x.length",
        "x.slice(0,1)+'|'+x.slice(-1)+'|'+x.substring(1,2)",
        "Object.prototype.toString.call(x)+'#'+(typeof x)",
    ]);
    for f in [0, JS_STRICT] {
        diff_batch("utf8 runes", &inputs, &exprs, f);
    }
}

/* row 101 + row 126: reprstr vs fmtstr (JSON) escaping. */
#[test]
fn strings_repr_vs_json_escaping() {
    let mut rng = Rng::new(SEED ^ 0x101);
    let mut elems: Vec<String> = strs(&[
        r#""""#,
        r#""a""#,
        r#""\t\n\r\b\f\u000b""#,
        r#""\u0001\u001f""#,
        r#""\\""#,
        r#""'""#,
        r#""\"""#,
        r#""/""#,
        r#""a b""#,
        r#""1a""#,
        r#""_a1""#,
        r#""12""#,
        r#""\u007f\u0080\u00ff""#,
        r#""\u07ff\u0800\uffff""#,
        r#""\uD800\uDBFF\uDC00\uDFFF""#,
        r#""\uD83D\uDE00""#,
        "\"\u{1f600}\u{10ffff}\"",
        r#""\u65e5\u672c\u8a9e""#,
    ]);
    for _ in 0..200 {
        elems.push(jstr(&rng.string(12)));
    }
    for f in [0, JS_STRICT] {
        /* reprstr, through js_tryrepr of the whole array */
        for (k, ch) in elems.chunks(20).enumerate() {
            diff_repr(&format!("reprstr chunk{}", k), ch, f);
        }
        /* fmtstr, through JSON.stringify */
        let exprs = strs(&[
            "JSON.stringify(x)",
            "JSON.stringify([x])",
            "JSON.stringify({k:x})",
            "JSON.stringify(x).length",
            "JSON.parse(JSON.stringify(x))===x",
            "x.length",
        ]);
        diff_batch("fmtstr", &elems, &exprs, f);
    }
    /* embedded NUL gets its own eval: it truncates every C string that follows */
    diff_each(
        "nul in string",
        &[
            "'a\\u0000b'.length",
            "JSON.stringify('a\\u0000b').length",
            "'a\\u0000b'.charCodeAt(1)",
            "'a\\u0000b'.indexOf('b')",
        ],
    );
}

/* the full String method surface over randomized inputs. */
#[test]
fn strings_method_surface_randomized() {
    let mut rng = Rng::new(SEED ^ 0x5721);
    let mut inputs: Vec<String> = strs(&[
        r#""""#,
        r#""a""#,
        r#""  spaced  ""#,
        r#""\t\n\r\u000b\f x \t\n""#,
        r#""aaabbbccc""#,
        r#""a\nb\nc""#,
        r#""ABCdefGHI""#,
        r#""\u00c9\u00e9\u0130\u0131\u00df""#,
        r#""\u65e5a\u672cb\u8a9e""#,
        "\"x\u{1f600}y\"",
        "new String('boxed')",
        "String(12345)",
    ]);
    for _ in 0..160 {
        inputs.push(jstr(&rng.string(18)));
    }
    let exprs = strs(&[
        "x.length",
        "x.trim()+'#'+x.trim().length",
        "x.toUpperCase()",
        "x.toLowerCase()",
        "x.toLocaleUpperCase()+'|'+x.toLocaleLowerCase()",
        "x.indexOf('a')+','+x.indexOf('a',2)+','+x.indexOf('')+','+x.indexOf('zz')",
        "x.lastIndexOf('a')+','+x.lastIndexOf('a',2)+','+x.lastIndexOf('')",
        "x.slice()+'|'+x.slice(1)+'|'+x.slice(1,3)+'|'+x.slice(-3)+'|'+x.slice(-3,-1)+'|'+x.slice(5,2)",
        "x.substring()+'|'+x.substring(2)+'|'+x.substring(2,4)+'|'+x.substring(4,2)+'|'+x.substring(-1,99)",
        "x.concat('-','x',1,null)",
        "x.charAt()+'|'+x.charCodeAt()",
        "x.split('').join('+')",
        "x.split('a').join('+')",
        "x.split('',3).join('+')",
        "x.split(undefined).join('+')",
        "String(x.split(/[ab]/))",
        "x.localeCompare('m')+','+x.localeCompare(x)",
        "x.replace('a','[$&]')",
        "x.replace('a',function(m,o,s){return '<'+m+'@'+o+'>'})",
        "x.replace(/[ab]/g,'[$&|$`|$\\'|$1|$$]')",
        "x.replace(/(a)(b)?/g,function(){return '{'+arguments.length+'}'})",
        "String(x.match(/[a-z]+/))",
        "String(x.match(/[a-z]+/g))",
        "x.search(/[0-9]/)+','+x.search(/zzz/)",
        "x+x",
        "x<'m'",
        "x>'m'",
        "x=='a'",
        "x==='a'",
        "JSON.stringify(x)",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(24).enumerate() {
            diff_batch(&format!("string methods chunk{}", k), ch, &exprs, f);
        }
    }
}

/* =================================================================== NUMBERS */

fn number_inputs(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut v: Vec<String> = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483647.0,
        -2147483648.0,
        2147483648.0,
        4294967295.0,
        4294967296.0,
        65535.0,
        65536.0,
        32768.0,
        1e-7,
        1e-6,
        5e-324,
        1e20,
        1e21,
        1e22,
        1.7976931348623157e308,
        0.00123,
        123.456,
        -255.0,
        9007199254740992.0,
        -9007199254740993.0,
        1e10,
        -1e10,
        1.9,
        -1.9,
        0.1,
        1.0 / 3.0,
    ]
    .iter()
    .map(|x| jsnum(*x))
    .collect();
    for _ in 0..n {
        v.push(jsnum(rng.f64()));
    }
    v
}

/* rows 103-106: jsV_numbertostring — integer fast path and all four grisu2
 * branches; row 104 -0; row 105 NaN/Infinity. */
#[test]
fn numbers_tostring_all_branches() {
    let mut rng = Rng::new(SEED ^ 0x103);
    let inputs = number_inputs(&mut rng, 4200);
    let exprs = strs(&[
        "String(x)",
        "x+''",
        "x.toString()",
        "x.toString(10)",
        "x.toLocaleString()",
        "JSON.stringify(x)",
        "JSON.stringify([x])",
        "String(-x)",
        "String(1/x)",
        "String(x+0)",
        "Number(String(x))===x",
        "String([x])",
        "({}).toString.call(x)",
        "typeof x",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(120).enumerate() {
            diff_batch(&format!("numbertostring chunk{}", k), ch, &exprs, f);
        }
    }
}

/* row 104/105 through `js_repr` (reprnum has its own -0 / NaN handling). */
#[test]
fn numbers_repr_of_values() {
    let mut rng = Rng::new(SEED ^ 0x104);
    let inputs = number_inputs(&mut rng, 500);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(40).enumerate() {
            diff_repr(&format!("reprnum chunk{}", k), ch, f);
        }
        diff_eval("repr -0", "-0", f);
        diff_eval("repr 0", "0", f);
        diff_eval("repr NaN", "NaN", f);
        diff_eval("repr Inf", "Infinity", f);
        diff_eval("repr -Inf", "-Infinity", f);
        diff_eval("repr -0 in obj", "({a:-0,b:0,c:NaN,d:Infinity})", f);
        diff_eval("JSON -0", "JSON.stringify([-0,0,NaN,Infinity,-Infinity])", f);
    }
}

/* row 110: Number.prototype.toString(radix) for every radix 2..36 plus the
 * invalid ones. */
#[test]
fn numbers_radix_tostring() {
    let mut rng = Rng::new(SEED ^ 0x110);
    let inputs = number_inputs(&mut rng, 300);
    let mut exprs: Vec<String> = Vec::new();
    for radix in 2..=36 {
        exprs.push(format!("x.toString({})", radix));
    }
    for radix in ["undefined", "0", "1", "37", "-1", "2.5", "10.9", "NaN", "'16'"] {
        exprs.push(format!("x.toString({})", radix));
    }
    exprs.push("(-x).toString(16)".to_string());
    exprs.push("Number.prototype.toString.call(x,8)".to_string());
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(50).enumerate() {
            diff_batch(&format!("radix chunk{}", k), ch, &exprs, f);
        }
    }
    diff_each(
        "radix non-number this",
        &[
            "Number.prototype.toString.call('7',16)",
            "Number.prototype.toString.call({},16)",
            "(0.5).toString(2)",
            "(0.1).toString(3)",
            "(-255).toString(16)",
            "(255).toString(16)",
        ],
    );
}

/* rows 111-113: toFixed / toExponential / toPrecision at every legal digit
 * count and just outside the legal range. */
#[test]
fn numbers_tofixed_toexponential_toprecision() {
    let mut rng = Rng::new(SEED ^ 0x111);
    let inputs = number_inputs(&mut rng, 220);
    let mut fixed: Vec<String> = Vec::new();
    for w in 0..=20 {
        fixed.push(format!("x.toFixed({})", w));
    }
    for w in ["-1", "21", "undefined", "NaN", "'3'", "2.9"] {
        fixed.push(format!("x.toFixed({})", w));
    }
    let mut expo: Vec<String> = Vec::new();
    for w in 0..=20 {
        expo.push(format!("x.toExponential({})", w));
    }
    for w in ["-1", "21", "undefined", "'3'"] {
        expo.push(format!("x.toExponential({})", w));
    }
    let mut prec: Vec<String> = Vec::new();
    for w in 1..=21 {
        prec.push(format!("x.toPrecision({})", w));
    }
    for w in ["0", "22", "-1", "undefined", "'3'"] {
        prec.push(format!("x.toPrecision({})", w));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(40).enumerate() {
            diff_batch(&format!("toFixed chunk{}", k), ch, &fixed, f);
            diff_batch(&format!("toExponential chunk{}", k), ch, &expo, f);
            diff_batch(&format!("toPrecision chunk{}", k), ch, &prec, f);
        }
    }
    diff_each(
        "digit methods on wrong this",
        &[
            "Number.prototype.toFixed.call('1',2)",
            "Number.prototype.toFixed.call({},2)",
            "Number.prototype.toExponential.call([],2)",
            "Number.prototype.toPrecision.call(null,2)",
            "(1e21).toFixed(2)",
            "(-1e21).toFixed(2)",
            "new Number(1.5).toFixed(3)",
        ],
    );
}

/* rows 114,115: jsV_stringtonumber, parseInt, parseFloat. */
#[test]
fn numbers_string_to_number() {
    let mut rng = Rng::new(SEED ^ 0x114);
    let mut inputs: Vec<String> = strs(&[
        r#""0x1f""#,
        r#""0X1F""#,
        r#""0x""#,
        r#""0x10000000000000000""#,
        r#""Infinity""#,
        r#""+Infinity""#,
        r#""-Infinity""#,
        r#""infinity""#,
        r#""  12  ""#,
        r#""\t\n 12 \r\n""#,
        r#""""#,
        r#""  ""#,
        r#""12abc""#,
        r#""1e3""#,
        r#""1E3""#,
        r#""1.5""#,
        r#""-12""#,
        r#""+12""#,
        r#"".""#,
        r#"".5""#,
        r#""5.""#,
        r#""1e""#,
        r#""1e+""#,
        r#""1e-3""#,
        r#""-.5e-3""#,
        r#""08""#,
        r#""0b11""#,
        r#""0o17""#,
        r#""1_000""#,
        r#""zz""#,
        r#""-""#,
        r#""+""#,
        r#""1 2""#,
        r#""0.0000001""#,
        r#""123456789012345678901234567890""#,
        r#""1e400""#,
        r#""1e-400""#,
        r#""NaN""#,
        r#""true""#,
        "null",
        "undefined",
        "true",
        "false",
        "[]",
        "[7]",
        "[1,2]",
        "({})",
        "new String('42')",
        "new Number(42)",
    ]);
    for _ in 0..200 {
        inputs.push(jstr(&rng.string(8)));
    }
    /* plus numeric-looking random strings */
    let mut r2 = Rng::new(SEED ^ 0x115);
    for _ in 0..200 {
        let x = r2.f64();
        let s = if x.is_finite() {
            format!("{:e}", x)
        } else {
            "Infinity".to_string()
        };
        inputs.push(jstr(&s));
    }
    let mut exprs = strs(&[
        "Number(x)",
        "+x",
        "-x",
        "x*1",
        "String(Number(x))",
        "isNaN(x)+','+isFinite(x)",
        "parseFloat(x)",
        "parseInt(x)",
        "x==0",
        "x>=0",
        "Math.abs(x)",
    ]);
    for radix in [0, 1, 2, 8, 10, 16, 36, 37, -1] {
        exprs.push(format!("parseInt(x,{})", radix));
    }
    exprs.push("parseInt(x,undefined)".to_string());
    exprs.push("parseInt(x,'16')".to_string());
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(60).enumerate() {
            diff_batch(&format!("stringtonumber chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 107-109 reached from JS: jsV_numbertointeger / int32 / uint32 / int16. */
#[test]
fn numbers_integer_coercions() {
    let mut rng = Rng::new(SEED ^ 0x107);
    let inputs = number_inputs(&mut rng, 900);
    let exprs = strs(&[
        "x|0",
        "x>>>0",
        "x>>0",
        "x<<0",
        "~~x",
        "~x",
        "x&0xffff",
        "x^0",
        "x>>16",
        "x<<16",
        "x>>>16",
        "x&-1",
        "Math.floor(x)",
        "Math.ceil(x)",
        "Math.round(x)",
        "x%1",
        "x%2",
        "x/0",
        "String([].slice(0,x).length)",
        "'abcdefgh'.charAt(x)",
        "'abcdefgh'.slice(x)",
        "'abcdefgh'.substring(x)",
        "[1,2,3].slice(x).join('')",
        "[1,2,3].indexOf(1,x)",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(80).enumerate() {
            diff_batch(&format!("int coercion chunk{}", k), ch, &exprs, f);
        }
    }
}

/* Math (row 171 JS_CMATH tag + the whole Math surface). */
#[test]
fn math_surface_randomized() {
    let mut rng = Rng::new(SEED ^ 0x111a);
    let inputs = number_inputs(&mut rng, 800);
    let exprs = strs(&[
        "Math.abs(x)",
        "Math.acos(x)",
        "Math.asin(x)",
        "Math.atan(x)",
        "Math.atan2(x,2)",
        "Math.atan2(2,x)",
        "Math.ceil(x)",
        "Math.cos(x)",
        "Math.exp(x)",
        "Math.floor(x)",
        "Math.log(x)",
        "Math.max(x,1)+','+Math.max(1,x)+','+Math.max()+','+Math.max(x)",
        "Math.min(x,1)+','+Math.min(1,x)+','+Math.min()+','+Math.min(x)",
        "Math.max(x,-0,0)",
        "Math.min(x,-0,0)",
        "Math.pow(x,2)",
        "Math.pow(2,x)",
        "Math.pow(x,x)",
        "Math.round(x)",
        "Math.sin(x)",
        "Math.sqrt(x)",
        "Math.tan(x)",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(80).enumerate() {
            diff_batch(&format!("math chunk{}", k), ch, &exprs, f);
        }
    }
    diff_each(
        "math constants",
        &[
            "[Math.E,Math.LN10,Math.LN2,Math.LOG2E,Math.LOG10E,Math.PI,Math.SQRT1_2,Math.SQRT2].join('|')",
            "Object.prototype.toString.call(Math)",
            "String(Math)",
            "typeof Math.random",
            "(function(){var r=Math.random();return typeof r==='number'&&r>=0&&r<1})()",
            "Object.getOwnPropertyNames(Math).sort().join(',')",
            "[Number.MAX_VALUE,Number.MIN_VALUE,Number.NaN,Number.NEGATIVE_INFINITY,Number.POSITIVE_INFINITY].join('|')",
        ],
    );
    /* Math is a JS_CMATH object: its repr has its own branch (row 124) */
    for f in [0, JS_STRICT] {
        diff_eval("repr Math", "Math", f);
        diff_eval("repr JSON", "JSON", f);
    }
}

/* ==================================================================== ARRAYS */

fn array_inputs(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut v: Vec<String> = strs(&[
        "[]",
        "[1]",
        "[1,2,3]",
        "[3,1,2]",
        "['b','a','c']",
        "[1,'1',true,null,undefined,'']",
        "[[1,2],[3],[]]",
        "[{a:1},{b:2}]",
        "new Array()",
        "new Array(0)",
        "new Array(5)",
        "new Array(1,2,3)",
        "(function(){var a=[1,2,3];delete a[1];return a})()",
        "(function(){var a=[];a[5]=1;return a})()",
        "(function(){var a=[1,2,3];a.length=1;return a})()",
        "(function(){var a=[1,2,3];a.length=6;return a})()",
        "(function(){var a=[];a[0]=0;a[2]=2;return a})()",
        "(function(){var a=[1,2,3];Object.defineProperty(a,'0',{value:9});return a})()",
        "(function(){var a=[1,2];a.x='named';return a})()",
        "(function(){var a=[];for(var i=0;i<40;i++)a[i]=i;return a})()",
        "(function(){var a=[];for(var i=0;i<9;i++)a.push(i);return a})()",
        "(function(){var a=[];for(var i=0;i<17;i++)a.push(i);return a})()",
        "'not an array'.split('')",
        "[NaN,Infinity,-Infinity,-0,0]",
    ]);
    for _ in 0..n {
        let len = rng.below(9) as usize;
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..len {
            parts.push(match rng.below(7) {
                0 => jsnum(rng.f64()),
                1 => jstr(&rng.string(4)),
                2 => "true".to_string(),
                3 => "null".to_string(),
                4 => "undefined".to_string(),
                5 => format!("{}", rng.range_i64(-5, 5)),
                _ => jstr(&rng.string(1)),
            });
        }
        v.push(format!("[{}]", parts.join(",")));
    }
    v
}

/* rows 82-90,94: flat vs sparse arrays, unflatten triggers, holes, named props. */
#[test]
fn arrays_flat_sparse_and_shape() {
    let mut rng = Rng::new(SEED ^ 0x82);
    let inputs = array_inputs(&mut rng, 140);
    let exprs = strs(&[
        "x.length",
        "String(x)",
        "x.join('|')",
        "JSON.stringify(x)",
        "Object.keys(x).join(',')",
        "Object.getOwnPropertyNames(x).join(',')",
        "(function(){var s='';for(var k in x)s+=k+'='+x[k]+';';return s})()",
        "Array.isArray(x)",
        "Object.prototype.toString.call(x)",
        "x.hasOwnProperty('0')+','+x.hasOwnProperty(0)+','+x.hasOwnProperty('length')",
        "String(x[0])+'/'+String(x[x.length-1])+'/'+String(x[x.length])+'/'+String(x[-1])",
        /* unflatten by writing past the end */
        "(function(){x[x.length+3]=1;return x.length+'#'+x.join('|')+'#'+Object.keys(x).join(',')})()",
        /* delete last vs middle element */
        "(function(){delete x[x.length-1];return x.length+'#'+x.join('|')+'#'+Object.keys(x).join(',')})()",
        "(function(){delete x[1];return x.length+'#'+x.join('|')+'#'+Object.keys(x).join(',')})()",
        /* shrink and grow through length */
        "(function(){x.length=2;return x.length+'#'+x.join('|')})()",
        "(function(){x.length=0;return x.length+'#'+x.join('|')})()",
        "(function(){x.length=12;return x.length+'#'+x.join('|')+'#'+Object.keys(x).join(',')})()",
        /* defineProperty on an index forces unflatten */
        "(function(){Object.defineProperty(x,'0',{value:42,enumerable:true});return x.join('|')+'#'+JSON.stringify(Object.getOwnPropertyDescriptor(x,'0'))})()",
        /* readonly index */
        "(function(){Object.defineProperty(x,'0',{value:1});try{x[0]=99}catch(e){return 'E:'+e.name}return x.join('|')})()",
        "(function(){x.x='named';x[1.5]='frac';x['01']='str';return Object.keys(x).join(',')+'#'+x.length})()",
        "(function(){var n=0;for(var k in x)n++;return n})()",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(24).enumerate() {
            diff_batch(&format!("array shape chunk{}", k), ch, &exprs, f);
        }
    }
}

/* The whole Array.prototype surface over randomized arrays. */
#[test]
fn arrays_method_surface_randomized() {
    let mut rng = Rng::new(SEED ^ 0x84);
    let inputs = array_inputs(&mut rng, 140);
    let exprs = strs(&[
        "x.sort().join('|')",
        "x.sort(function(a,b){return a<b?-1:(a>b?1:0)}).join('|')",
        "x.sort(function(a,b){return b-a}).join('|')",
        "x.sort(function(){return 0}).join('|')",
        "(function(){try{return x.sort(function(){throw new Error('cmp')}).join('|')}catch(e){return 'E:'+e.message}})()",
        "x.sort(function(a,b){return 'x'}).join('|')",
        "x.sort(function(a,b){return NaN}).join('|')",
        "(function(){try{return x.sort(1).join('|')}catch(e){return 'E:'+e.name}})()",
        "x.reverse().join('|')",
        "x.concat().join('|')",
        "x.concat([1,2],3).join('|')",
        "x.concat(x).length",
        "x.join()",
        "x.join('')",
        "x.join(undefined)",
        "x.slice().join('|')",
        "x.slice(1).join('|')",
        "x.slice(1,-1).join('|')",
        "x.slice(-2).join('|')",
        "x.slice(99).join('|')",
        "x.splice(1,2).join('|')+'#'+x.join('|')+'#'+x.length",
        "x.splice(0,0,'n1','n2').join('|')+'#'+x.join('|')",
        "x.splice(-1).join('|')+'#'+x.join('|')",
        "x.splice().join('|')+'#'+x.join('|')",
        "x.indexOf(x[0])+','+x.lastIndexOf(x[0])+','+x.indexOf('nope')",
        "x.indexOf(1)+','+x.indexOf(1,1)+','+x.lastIndexOf(1,1)",
        "x.push('p')+'#'+x.join('|')",
        "x.push()+'#'+x.join('|')",
        "String(x.pop())+'#'+x.join('|')+'#'+x.length",
        "String(x.shift())+'#'+x.join('|')+'#'+x.length",
        "x.unshift('u')+'#'+x.join('|')",
        "x.unshift()+'#'+x.join('|')",
        "x.map(function(v,i,a){return typeof v}).join('|')",
        "x.filter(function(v){return !!v}).join('|')",
        "x.every(function(v){return !!v})+','+x.some(function(v){return !!v})",
        "x.every(function(){return true})+','+x.some(function(){return false})",
        "(function(){var s='';x.forEach(function(v,i){s+=i+':'+v+';'});return s})()",
        "x.reduce(function(a,b){return a+'/'+b},'S')",
        "x.reduceRight(function(a,b){return a+'/'+b},'S')",
        "(function(){try{return x.reduce(function(a,b){return a})}catch(e){return 'E:'+e.name}})()",
        "x.toString()",
        "x.toLocaleString()",
        "Array.prototype.join.call({length:2,0:'a',1:'b'},'-')",
        "(function(){var s=[];x.forEach(function(v,i,a){s.push(a===x)});return s.join(',')})()",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(24).enumerate() {
            diff_batch(&format!("array methods chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 91,92,93: length range errors, JS_ARRAYLIMIT, array-like length. */
#[test]
fn arrays_length_errors_and_limits() {
    diff_each(
        "array length errors",
        &[
            "(function(){var a=[];try{a.length=1.5}catch(e){return 'E:'+e.name+':'+e.message}return a.length})()",
            "(function(){var a=[];try{a.length=-1}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=[];try{a.length=NaN}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=[];try{a.length='abc'}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=[];a.length='7';return a.length})()",
            "(function(){var a=[];try{a.length=(1<<26)}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=[];try{a.length=(1<<26)+1}catch(e){return 'E:'+e.name+':'+e.message}return a.length})()",
            "(function(){var a=[];try{a[(1<<26)-1]=1}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=[];try{a[(1<<26)]=1}catch(e){return 'E:'+e.name+':'+e.message}return a.length})()",
            "(function(){try{return new Array(-1).length}catch(e){return 'E:'+e.name}})()",
            "(function(){try{return new Array(1.5).length}catch(e){return 'E:'+e.name}})()",
            "(function(){try{return new Array(4294967296).length}catch(e){return 'E:'+e.name}})()",
            "Array.prototype.join.call({length:-1},'-')",
            "Array.prototype.join.call({},'-')",
            "Array.prototype.join.call({length:'3',0:'a'},'-')",
            "Array.prototype.slice.call({length:2,0:'a',1:'b'}).join('|')",
            "(function(){try{Object.defineProperty([],'length',{get:function(){return 1}})}catch(e){return 'E:'+e.name}return 'ok'})()",
            "(function(){var a=[1,2,3];a.length=1e10;return a.length})()",
        ],
    );
}

/* =================================================== OBJECTS / ACCESSORS */

/* row 191: Object.defineProperty attribute mapping, defineProperties, create. */
#[test]
fn objects_descriptors_and_attributes() {
    let inputs = strs(&[
        "({})",
        "({a:1})",
        "[1,2,3]",
        "(function(){})",
        "new String('ab')",
        "new Number(1)",
        "/re/g",
        "Object.create(null)",
        "Object.create({inherited:1})",
    ]);
    let exprs = strs(&[
        "(function(){Object.defineProperty(x,'p',{});return JSON.stringify(Object.getOwnPropertyDescriptor(x,'p'))+'#'+String(x.p)})()",
        "(function(){Object.defineProperty(x,'p',{value:1});return JSON.stringify(Object.getOwnPropertyDescriptor(x,'p'))})()",
        "(function(){Object.defineProperty(x,'p',{value:1,writable:true});x.p=2;return JSON.stringify(Object.getOwnPropertyDescriptor(x,'p'))})()",
        "(function(){Object.defineProperty(x,'p',{value:1,writable:true,enumerable:true,configurable:true});return JSON.stringify(Object.getOwnPropertyDescriptor(x,'p'))+'#'+Object.keys(x).join(',')})()",
        "(function(){Object.defineProperty(x,'p',{value:1,enumerable:false});return Object.keys(x).join(',')+'#'+Object.getOwnPropertyNames(x).join(',')})()",
        "(function(){Object.defineProperty(x,'p',{get:function(){return 7}});return String(x.p)+'#'+JSON.stringify(Object.getOwnPropertyDescriptor(x,'p'))})()",
        "(function(){var v=0;Object.defineProperty(x,'p',{set:function(n){v=n}});x.p=5;return String(x.p)+'#'+v})()",
        "(function(){var v=0;Object.defineProperty(x,'p',{get:function(){return v*2},set:function(n){v=n}});x.p=5;return String(x.p)})()",
        "(function(){try{Object.defineProperty(x,'p',{value:1,get:function(){}})}catch(e){return 'E:'+e.name+':'+e.message}return 'no-throw'})()",
        "(function(){try{Object.defineProperty(x,'p',{writable:true,set:function(){}})}catch(e){return 'E:'+e.name}return 'no-throw'})()",
        "(function(){Object.defineProperty(x,'p',{value:1,configurable:false});try{delete x.p}catch(e){return 'E:'+e.name}return String(x.p)})()",
        "(function(){Object.defineProperty(x,'p',{value:1});try{Object.defineProperty(x,'p',{get:function(){return 2}})}catch(e){return 'E:'+e.name}return String(x.p)})()",
        "(function(){Object.defineProperties(x,{a:{value:1,enumerable:true},b:{get:function(){return 2}}});return Object.keys(x).join(',')+'#'+String(x.a)+String(x.b)})()",
        "(function(){var o=Object.create(x,{q:{value:9,enumerable:true}});return Object.keys(o).join(',')+'#'+String(o.q)+'#'+(Object.getPrototypeOf(o)===x)})()",
        "(function(){try{return Object.defineProperty(1,'p',{value:1})}catch(e){return 'E:'+e.name}})()",
        "String(Object.getOwnPropertyDescriptor(x,'nope'))",
        "Object.getOwnPropertyNames(x).sort().join(',')",
        "Object.keys(x).sort().join(',')",
        "(function(){var s='';for(var k in x)s+=k+',';return s})()",
        "x.propertyIsEnumerable('0')+','+x.propertyIsEnumerable('length')+','+x.hasOwnProperty('length')",
        "String(Object.getPrototypeOf(x)===Object.prototype)",
    ]);
    for f in [0, JS_STRICT] {
        diff_batch("descriptors", &inputs, &exprs, f);
    }
    diff_each(
        "descriptor edge cases",
        &[
            "JSON.stringify(Object.getOwnPropertyDescriptor([1,2],'0'))",
            "JSON.stringify(Object.getOwnPropertyDescriptor([1,2],'length'))",
            "JSON.stringify(Object.getOwnPropertyDescriptor('ab',0))",
            "JSON.stringify(Object.getOwnPropertyDescriptor(new String('ab'),'0'))",
            "Object.getOwnPropertyNames(new String('abc')).join(',')",
            "Object.getOwnPropertyNames(/a/g).join(',')",
            "JSON.stringify(Object.getOwnPropertyDescriptor(/a/g,'source'))",
            "Object.getOwnPropertyNames(function(a,b){}).sort().join(',')",
            "(function(){try{return Object.keys(1)}catch(e){return 'E:'+e.name}})()",
            "(function(){try{return Object.getPrototypeOf('s')===String.prototype}catch(e){return 'E:'+e.name}})()",
            "(function(){var o=Object.create(null);o.a=1;return Object.keys(o).join(',')+'#'+(typeof o.toString)})()",
            "(function(){var o=Object.create(null);try{return ''+o}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "'abc'.hasOwnProperty(0)+','+'abc'.hasOwnProperty(3)+','+'abc'.hasOwnProperty('length')",
            "new String('abc').hasOwnProperty(0)+','+new String('abc').hasOwnProperty(3)",
            "[1,2].hasOwnProperty(0)+','+[1,2].hasOwnProperty(2)+','+[1,2].hasOwnProperty('length')",
            "(function(){var a=[];a[5]=1;return a.hasOwnProperty(0)+','+a.hasOwnProperty(5)})()",
            "(function(){return Object.prototype.toString.call(arguments)})(1,2)",
            "(function(){return Object.getOwnPropertyNames(arguments).sort().join(',')})(1,2)",
            "(function(){var s='';for(var k in arguments)s+=k+'='+arguments[k]+';';return s})(7,8)",
            "[Object.prototype.toString.call(Math),Object.prototype.toString.call(JSON),Object.prototype.toString.call(/a/),Object.prototype.toString.call(new Date(0)),Object.prototype.toString.call(new Error('m')),Object.prototype.toString.call(new Number(1)),Object.prototype.toString.call(new String('s')),Object.prototype.toString.call(new Boolean(1)),Object.prototype.toString.call(function(){}),Object.prototype.toString.call([]),Object.prototype.toString.call({})].join('|')",
        ],
    );
}

/* row 192: seal / freeze / preventExtensions and their predicates. */
#[test]
fn objects_seal_freeze_extensible() {
    let inputs = strs(&[
        "({})",
        "({a:1,b:2})",
        "[1,2,3]",
        "[]",
        "(function(){var a=[];a[3]=1;return a})()",
        "new String('ab')",
        "(function(){})",
        "(function(){var o={};Object.defineProperty(o,'a',{value:1});return o})()",
        "(function(){var o={a:1};Object.preventExtensions(o);return o})()",
    ]);
    let exprs = strs(&[
        "Object.isExtensible(x)+','+Object.isSealed(x)+','+Object.isFrozen(x)",
        "(function(){Object.preventExtensions(x);return Object.isExtensible(x)+','+Object.isSealed(x)+','+Object.isFrozen(x)})()",
        "(function(){Object.seal(x);return Object.isExtensible(x)+','+Object.isSealed(x)+','+Object.isFrozen(x)})()",
        "(function(){Object.freeze(x);return Object.isExtensible(x)+','+Object.isSealed(x)+','+Object.isFrozen(x)})()",
        "(function(){Object.freeze(x);try{x.zz=1}catch(e){return 'E:'+e.name+':'+e.message}return String(x.zz)})()",
        "(function(){Object.freeze(x);try{x[0]=9}catch(e){return 'E:'+e.name}return String(x[0])})()",
        "(function(){Object.seal(x);try{delete x.a}catch(e){return 'E:'+e.name}return Object.getOwnPropertyNames(x).join(',')})()",
        "(function(){Object.preventExtensions(x);try{x.zz=1}catch(e){return 'E:'+e.name}return String(x.zz)})()",
        "(function(){Object.freeze(x);return Object.getOwnPropertyNames(x).sort().join(',')+'#'+JSON.stringify(x)})()",
        "(function(){Object.seal(x);x.a=99;return String(x.a)})()",
        "(function(){return Object.freeze(x)===x})()",
    ]);
    for f in [0, JS_STRICT] {
        diff_batch("seal/freeze", &inputs, &exprs, f);
    }
}

/* getters/setters written in JS, prototype chains, accessor inheritance. */
#[test]
fn objects_accessors_and_prototypes() {
    diff_each(
        "accessors",
        &[
            "(function(){var o={get a(){return 1}};return String(o.a)+'#'+JSON.stringify(Object.getOwnPropertyDescriptor(o,'a'))})()",
            "(function(){var v=0;var o={set a(n){v=n}};o.a=5;return String(o.a)+'#'+v})()",
            "(function(){var v=1;var o={get a(){return v},set a(n){v=n*2}};o.a=5;return String(o.a)})()",
            "(function(){var o={get a(){throw new Error('boom')}};try{return o.a}catch(e){return 'E:'+e.message}})()",
            "(function(){var o={get a(){throw new Error('boom')}};try{return String(o)}catch(e){return 'E:'+e.message}})()",
            "(function(){var o={get a(){throw new Error('boom')}};try{return JSON.stringify(o)}catch(e){return 'E:'+e.message}})()",
            "(function(){var p={get a(){return this.b}};var o=Object.create(p);o.b=7;return String(o.a)})()",
            "(function(){var v='';var p={set a(n){v='proto:'+n}};var o=Object.create(p);o.a=3;return v+'#'+o.hasOwnProperty('a')})()",
            "(function(){var o={a:1};var p={a:2};return String(o.a)+String(Object.create(p).a)})()",
            "(function(){function A(){};A.prototype.m=function(){return 'A'};function B(){};B.prototype=new A();var b=new B();return b.m()+'#'+(b instanceof A)+'#'+(b instanceof B)})()",
            "(function(){function A(){this.x=1};var a=new A();return a.constructor===A})()",
            "(function(){var o={};o.__proto__===Object.prototype})()",
            "(function(){var o={toString:function(){return 'TS'},valueOf:function(){return 42}};return (o+'')+'#'+(o*1)+'#'+String(o)})()",
            "(function(){var o={valueOf:function(){return {}},toString:function(){return 'TS'}};return o+''})()",
            "(function(){var o={toString:function(){return {}},valueOf:function(){return 9}};return o+''})()",
            "(function(){var o={toString:function(){return {}},valueOf:function(){return {}}};try{return o+''}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){var o={};Object.defineProperty(o,'a',{get:function(){return 1}});try{o.a=2}catch(e){return 'E:'+e.name+':'+e.message}return String(o.a)})()",
            "(function(){var o={};Object.defineProperty(o,'a',{value:1});try{o.a=2}catch(e){return 'E:'+e.name+':'+e.message}return String(o.a)})()",
            "(function(){var o={};Object.preventExtensions(o);try{o.a=1}catch(e){return 'E:'+e.name+':'+e.message}return String(o.a)})()",
            "(function(){var s='abc';try{s.foo=1}catch(e){return 'E:'+e.name+':'+e.message}return String(s.foo)})()",
            "(function(){var n=1;try{n.foo=1}catch(e){return 'E:'+e.name}return String(n.foo)})()",
            "(function(){var o={a:1,b:2};var d=Object.getOwnPropertyDescriptor(o,'a');return [d.value,d.writable,d.enumerable,d.configurable].join(',')})()",
            "Object.prototype.isPrototypeOf.call(Object.prototype,{})+','+Object.prototype.isPrototypeOf.call({},{})",
            "(function(){var o={};return o.propertyIsEnumerable('toString')+','+Object.prototype.propertyIsEnumerable.call(Object.prototype,'toString')})()",
            "(function(){var o={1:'a','01':'b',1.5:'c','-1':'d','':'e'};return Object.keys(o).join('|')+'#'+o[1]+o['01']})()",
            "({a:1,a:2})",
            "({get a(){return 1},get a(){return 2}})",
            "({a:1,'a':2})",
            "(function(){try{return eval('({a:1,a:2})').a}catch(e){return 'E:'+e.name}})()",
        ],
    );
}

/* row 194: for-in over every target shape + randomized property names. */
#[test]
fn objects_forin_and_property_names() {
    tz_init();
    let mut rng = Rng::new(SEED ^ 0x194);
    let mut inputs = strs(&[
        "({})",
        "({a:1,b:2,c:3})",
        "Object.create({p:1})",
        "(function(){var o=Object.create({p:1});o.p=2;o.q=3;return o})()",
        "[1,2,3]",
        "(function(){var a=[1,2,3];delete a[1];return a})()",
        "(function(){var a=[];a[7]=1;return a})()",
        "'str'",
        "new String('str')",
        "undefined",
        "null",
        "1",
        "true",
        "(function(){})",
        "/re/g",
        "new Date(0)",
        "Math",
        "JSON",
        "(function(){var o={};Object.defineProperty(o,'h',{value:1});o.v=2;return o})()",
    ]);
    /* randomized property names, including non-identifier and numeric-looking */
    for _ in 0..80 {
        let a = rng.string(6);
        let b = rng.string(6);
        let n = rng.range_i64(-3, 40);
        inputs.push(format!(
            "(function(){{var o={{}};o[{}]=1;o[{}]=2;o[{}]=3;return o}})()",
            jstr(&a),
            jstr(&b),
            n
        ));
    }
    let exprs = strs(&[
        "(function(){var s='';for(var k in x)s+=k+'='+String(x[k])+';';return s})()",
        "(function(){var n=0;for(var k in x)n++;return n})()",
        "(function(){try{return Object.keys(x).join('|')}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return Object.getOwnPropertyNames(x).join('|')}catch(e){return 'E:'+e.name}})()",
        /* deleting during iteration */
        "(function(){var s='';for(var k in x){s+=k+';';delete x[k]}return s+'#'+Object.keys(x).join(',')})()",
        "(function(){var s='';for(var k in x){s+=k+';';if(x.zz===undefined)x.zz=1}return s})()",
        "String(x)",
        "typeof x",
        "Object.prototype.toString.call(x)",
        "JSON.stringify(x)",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(25).enumerate() {
            diff_batch(&format!("for-in chunk{}", k), ch, &exprs, f);
        }
    }
}

/* ====================================================================== DATE */

fn date_inputs(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut v: Vec<String> = [
        0.0,
        -1.0,
        1.0,
        1000.0,
        -1000.0,
        86400000.0,
        1500000000000.0,
        1e12,
        -1e12,
        8.64e15,
        -8.64e15,
        8.64e15 + 1.0,
        f64::NAN,
        951782400000.0,  /* 2000-02-29 */
        1078012800000.0, /* 2004-02-29 */
        -2208988800000.0,
        1.6e12,
        253402300800000.0,  /* year 10000 */
        -62167219200000.0,  /* year 0 */
        -62198755200000.0,  /* year -1 */
        -62735596800000.0,  /* year -18 */
        8.64e15 - 1.0,
        -8.64e15 + 1.0,
    ]
    .iter()
    .map(|x| format!("new Date({})", jsnum(*x)))
    .collect();
    for _ in 0..n {
        let ms = rng.range_i64(-2_200_000_000_000, 2_200_000_000_000);
        v.push(format!("new Date({})", ms));
    }
    v.push("new Date('invalid')".to_string());
    v.push("new Date(NaN)".to_string());
    v
}

/* rows 173-175,177: every Date getter under a fixed non-UTC local zone. */
#[test]
fn date_getters_randomized() {
    tz_init();
    let mut rng = Rng::new(SEED ^ 0x173);
    let inputs = date_inputs(&mut rng, 700);
    let exprs = strs(&[
        "x.getTime()",
        "x.valueOf()",
        "x.getFullYear()+'/'+x.getUTCFullYear()",
        "x.getMonth()+'/'+x.getUTCMonth()",
        "x.getDate()+'/'+x.getUTCDate()",
        "x.getDay()+'/'+x.getUTCDay()",
        "x.getHours()+'/'+x.getUTCHours()",
        "x.getMinutes()+'/'+x.getUTCMinutes()",
        "x.getSeconds()+'/'+x.getUTCSeconds()",
        "x.getMilliseconds()+'/'+x.getUTCMilliseconds()",
        "x.getTimezoneOffset()",
        "x.toString()",
        "x.toDateString()",
        "x.toTimeString()",
        "x.toLocaleString()",
        "x.toLocaleDateString()",
        "x.toLocaleTimeString()",
        "x.toUTCString()",
        "(function(){try{return x.toISOString()}catch(e){return 'E:'+e.name+':'+e.message}})()",
        "(function(){try{return String(x.toJSON())}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return JSON.stringify(x)}catch(e){return 'E:'+e.name}})()",
        "String(x)+'#'+(x+'')+'#'+(+x)",
        "Object.prototype.toString.call(x)",
        "Date.parse(x.toISOString?String(x):'x')",
        "typeof x.getTime",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(40).enumerate() {
            diff_batch(&format!("date getters chunk{}", k), ch, &exprs, f);
        }
    }
}

/* row 176: every paired local/UTC setter. */
#[test]
fn date_setters_randomized() {
    tz_init();
    let mut rng = Rng::new(SEED ^ 0x176);
    let inputs = date_inputs(&mut rng, 100);
    let mut exprs: Vec<String> = Vec::new();
    let setters: [(&str, &str); 7] = [
        ("setMilliseconds", "setUTCMilliseconds"),
        ("setSeconds", "setUTCSeconds"),
        ("setMinutes", "setUTCMinutes"),
        ("setHours", "setUTCHours"),
        ("setDate", "setUTCDate"),
        ("setMonth", "setUTCMonth"),
        ("setFullYear", "setUTCFullYear"),
    ];
    for (loc, utc) in setters {
        for args in ["3", "3,4", "3,4,5", "3,4,5,6", "-1", "1e10", "NaN", ""] {
            exprs.push(format!(
                "(function(){{var d=new Date(x.getTime());d.{}({});return d.getTime()+'/'+d.toUTCString()}})()",
                loc, args
            ));
            exprs.push(format!(
                "(function(){{var d=new Date(x.getTime());d.{}({});return d.getTime()+'/'+d.toUTCString()}})()",
                utc, args
            ));
        }
    }
    for args in ["0", "1e13", "NaN", "'123'", ""] {
        exprs.push(format!(
            "(function(){{var d=new Date(x.getTime());d.setTime({});return d.getTime()}})()",
            args
        ));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(20).enumerate() {
            diff_batch(&format!("date setters chunk{}", k), ch, &exprs, f);
        }
    }
}

/* row 177: construction and parsing. */
#[test]
fn date_construct_and_parse() {
    tz_init();
    let mut rng = Rng::new(SEED ^ 0x177);
    let mut inputs: Vec<String> = strs(&[
        r#""1970-01-01T00:00:00Z""#,
        r#""1970-01-01T00:00:00""#,
        r#""1970-01-01""#,
        r#""2017-07-14T02:40:00.000Z""#,
        r#""2017-07-14T02:40:00.000+05:30""#,
        r#""2017-07-14T02:40:00.000-05:00""#,
        r#""2017-07-14T02:40:00+0530""#,
        r#""2017-07-14T02:40Z""#,
        r#""2017-07""#,
        r#""2017""#,
        r#""+002017-07-14T02:40:00Z""#,
        r#""-000001-07-14T02:40:00Z""#,
        r#""not a date""#,
        r#""""#,
        r#""0000-01-01T00:00:00Z""#,
        r#""275760-09-13T00:00:00Z""#,
        r#""275760-09-14T00:00:00Z""#,
        "0",
        "1",
        "NaN",
        "1e13",
        "1e17",
        "-1e13",
        "'0'",
        "true",
        "null",
        "undefined",
        "[]",
        "({})",
    ]);
    for _ in 0..30 {
        let y = rng.range_i64(-1, 3000);
        let mo = rng.range_i64(-2, 14);
        let d = rng.range_i64(-2, 35);
        let h = rng.range_i64(-2, 26);
        let mi = rng.range_i64(-2, 62);
        let s = rng.range_i64(-2, 62);
        let ms = rng.range_i64(-2, 1002);
        inputs.push(format!("[{},{},{},{},{},{},{}]", y, mo, d, h, mi, s, ms));
    }
    let exprs = strs(&[
        "(function(){try{return new Date(x).getTime()}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return String(new Date(x))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return Date.parse(String(x))}catch(e){return 'E:'+e.name}})()",
        "(function(){if(!(x instanceof Array))return 'skip';return new Date(x[0],x[1],x[2],x[3],x[4],x[5],x[6]).getTime()})()",
        "(function(){if(!(x instanceof Array))return 'skip';return Date.UTC(x[0],x[1],x[2],x[3],x[4],x[5],x[6])})()",
        "(function(){if(!(x instanceof Array))return 'skip';return new Date(x[0],x[1]).getTime()+'/'+new Date(x[0]).getTime()})()",
        "(function(){if(!(x instanceof Array))return 'skip';return String(new Date(x[0],x[1],x[2]))})()",
        "(function(){var d=new Date(x);var t=d.getTime();if(t!==t)return 'NaN-date';return Date.parse(d.toISOString())===t})()",
        "(function(){var d=new Date(x);var t=d.getTime();if(t!==t)return 'NaN-date';return Date.parse(d.toUTCString())})()",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(30).enumerate() {
            diff_batch(&format!("date construct chunk{}", k), ch, &exprs, f);
        }
    }
    diff_each(
        "date now/no-arg",
        &[
            "typeof Date.now()",
            "Date.now()>0",
            "typeof new Date().getTime()",
            "new Date() instanceof Date",
            "typeof Date()",
            "Date.length",
            "Date.UTC()!==Date.UTC()",
            "String(Date.UTC(1970,0))",
            "Object.prototype.toString.call(new Date(0))",
            "(function(){var d=new Date(0);d.setTime(1);return d.getTime()})()",
        ],
    );
}

/* ====================================================================== JSON */

fn json_value_inputs(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut v: Vec<String> = strs(&[
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
        "''",
        "'a\"b\\\\c'",
        "'\\u0000'",
        "'\\u001f'",
        "'\\uD800'",
        "({})",
        "[]",
        "[1,2,3]",
        "[[[]]]",
        "({a:1,b:'s',c:[1,{d:null}]})",
        "({a:undefined,b:function(){},c:1})",
        "[undefined,function(){},1]",
        "({toJSON:function(){return 'TJ'}})",
        "({a:{toJSON:function(k){return 'k='+k}}})",
        "new Date(0)",
        "new Number(5)",
        "new String('bx')",
        "new Boolean(true)",
        "(function(){})",
        "/re/g",
        "Object.create(null)",
        "({a:{b:{c:{d:{e:1}}}}})",
        "(function(){var a=[];a[3]=1;return a})()",
        "(function(){var o={};Object.defineProperty(o,'h',{value:1});o.v=2;return o})()",
        "(function(){var o={};Object.defineProperty(o,'g',{get:function(){return 3},enumerable:true});return o})()",
        "Math",
        "[new Date(0),new Date(NaN)]",
    ]);
    for _ in 0..n {
        v.push(rand_json_expr(rng, 0));
    }
    v
}

fn rand_json_expr(rng: &mut Rng, depth: u32) -> String {
    match rng.below(if depth >= 3 { 6 } else { 9 }) {
        0 => "null".to_string(),
        1 => "true".to_string(),
        2 => "false".to_string(),
        3 => jsnum(rng.f64()),
        4 => jstr(&rng.string(6)),
        5 => format!("{}", rng.range_i64(-100, 100)),
        6 | 7 => {
            let n = rng.below(4);
            let mut p: Vec<String> = Vec::new();
            for _ in 0..n {
                p.push(rand_json_expr(rng, depth + 1));
            }
            format!("[{}]", p.join(","))
        }
        _ => {
            let n = rng.below(4);
            let mut p: Vec<String> = Vec::new();
            let mut seen: Vec<String> = Vec::new();
            for _ in 0..n {
                /* mujs rejects duplicate property names in an object literal at
                 * compile time, so the generator must not emit one */
                let k = jstr(&rng.string(4));
                if seen.contains(&k) {
                    continue;
                }
                seen.push(k.clone());
                p.push(format!("{}:{}", k, rand_json_expr(rng, depth + 1)));
            }
            format!("({{{}}})", p.join(","))
        }
    }
}

/* rows 178-180: stringify with no gap and with every numeric / string indent. */
#[test]
fn json_stringify_indent_shapes() {
    tz_init();
    let mut rng = Rng::new(SEED ^ 0x178);
    let inputs = json_value_inputs(&mut rng, 220);
    let mut exprs = strs(&["String(JSON.stringify(x))", "String(JSON.stringify(x,null))"]);
    for sp in [
        "0", "1", "2", "4", "10", "11", "20", "-1", "1.9", "NaN", "''", "'\\t'", "'ab'",
        "'123456789012345'", "new Number(3)", "new String('--')", "true", "null", "undefined",
        "[]", "({})",
    ] {
        exprs.push(format!("String(JSON.stringify(x,null,{}))", sp));
    }
    exprs.push("String(JSON.stringify(x,undefined,2))".to_string());
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(30).enumerate() {
            diff_batch(&format!("json indent chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 181-183: replacer function, replacer array, cyclic values. */
#[test]
fn json_stringify_replacer_and_cycles() {
    tz_init();
    let mut rng = Rng::new(SEED ^ 0x181);
    let inputs = json_value_inputs(&mut rng, 160);
    let exprs = strs(&[
        "String(JSON.stringify(x,function(k,v){return v}))",
        "String(JSON.stringify(x,function(k,v){return typeof v==='number'?v+1:v}))",
        "String(JSON.stringify(x,function(k,v){return k==='a'?undefined:v}))",
        "String(JSON.stringify(x,function(k,v){return undefined}))",
        "String(JSON.stringify(x,function(k,v){return typeof v==='object'&&v!==null?v:String(v)}))",
        "(function(){var log=[];JSON.stringify(x,function(k,v){log.push(k+':'+typeof v);return v});return log.join('|')})()",
        "(function(){var log=[];JSON.stringify(x,function(k,v){log.push(this===null?'null':typeof this);return v});return log.join('|')})()",
        "String(JSON.stringify(x,function(k,v){return v},2))",
        "String(JSON.stringify(x,[]))",
        "String(JSON.stringify(x,['a']))",
        "String(JSON.stringify(x,['a','b','c']))",
        "String(JSON.stringify(x,[0,1,2]))",
        "String(JSON.stringify(x,[new String('a'),new Number(1)]))",
        "String(JSON.stringify(x,['a',{},null,true]))",
        "String(JSON.stringify(x,['a','b'],4))",
        "String(JSON.stringify(x,{}))",
        "String(JSON.stringify(x,'nope'))",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(25).enumerate() {
            diff_batch(&format!("json replacer chunk{}", k), ch, &exprs, f);
        }
    }
    diff_each(
        "json cyclic",
        &[
            "(function(){var o={};o.self=o;try{return JSON.stringify(o)}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){var a=[];a.push(a);try{return JSON.stringify(a)}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){var o={};o.self=o;try{return JSON.stringify(o,null,4)}catch(e){return 'E:'+e.name}})()",
            "(function(){var a=[];a.push(a);try{return JSON.stringify(a,null,'\\t')}catch(e){return 'E:'+e.name}})()",
            "(function(){var o={a:{}};o.a.b=o;try{return JSON.stringify(o)}catch(e){return 'E:'+e.name}})()",
            "JSON.stringify({},null,2)",
            "JSON.stringify([],null,2)",
            "JSON.stringify({a:{}},null,2)",
            "JSON.stringify([[]],null,2)",
            "JSON.stringify({a:[]},null,'..')",
            "(function(){var o={};o.self=o;try{return JSON.stringify(o,function(k,v){return k==='self'?1:v})}catch(e){return 'E:'+e.name}})()",
            "String(JSON.stringify())",
            "(function(){try{return JSON.stringify(undefined,undefined,undefined)}catch(e){return 'E:'+e.name}})()",
            "Object.prototype.toString.call(JSON)",
        ],
    );
}

/* row 184: JSON.parse with and without a reviver, plus malformed inputs. */
#[test]
fn json_parse_roundtrip_and_reviver() {
    let mut rng = Rng::new(SEED ^ 0x184);
    let mut inputs: Vec<String> = strs(&[
        r#""{}""#,
        r#""[]""#,
        r#""[1,2,3]""#,
        r#""null""#,
        r#""true""#,
        r#""false""#,
        r#""0""#,
        r#""-0""#,
        r#""1.5e3""#,
        r#""-1.5E-3""#,
        r#""\"\\u0041\"""#,
        r#""\"\\uD83D\\uDE00\"""#,
        r#""\"\\u0000\"""#,
        r#""\"a\\/b\\\\c\\\"d\\b\\f\\n\\r\\t\"""#,
        r#""{\"a\":1,\"b\":[2,{\"c\":null}]}""#,
        r#""  \n\t [ 1 , 2 ] \r\n ""#,
        r#""{""#,
        r#""{a:1}""#,
        r#""[1,]""#,
        r#""[1 2]""#,
        r#""""#,
        r#""nul""#,
        r#""01""#,
        r#""+1""#,
        r#""1.""#,
        r#""'a'""#,
        r#""\"unterminated""#,
        r#""[[[[[[1]]]]]]""#,
        r#""{\"\":1}""#,
        r#""1e400""#,
        r#""undefined""#,
        r#""NaN""#,
        r#""Infinity""#,
        r#""[0x10]""#,
    ]);
    /* round trip through stringify of random values */
    let mut r2 = Rng::new(SEED ^ 0x1841);
    for _ in 0..140 {
        let e = rand_json_expr(&mut r2, 0);
        inputs.push(format!("JSON.stringify({})", e));
    }
    for _ in 0..20 {
        inputs.push(jstr(&rng.string(8)));
    }
    let exprs = strs(&[
        "(function(){try{return String(JSON.parse(x))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return JSON.stringify(JSON.parse(x))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return JSON.stringify(JSON.parse(x),null,2)}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return typeof JSON.parse(x)}catch(e){return 'E:'+e.name+':'+e.message}})()",
        "(function(){try{return JSON.stringify(JSON.parse(x,function(k,v){return v}))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return JSON.stringify(JSON.parse(x,function(k,v){return typeof v==='number'?v*2:v}))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return String(JSON.parse(x,function(k,v){return undefined}))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{var log=[];JSON.parse(x,function(k,v){log.push(k+':'+typeof v);return v});return log.join('|')}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return JSON.stringify(JSON.parse(x,function(k,v){return k==='a'?undefined:v}))}catch(e){return 'E:'+e.name}})()",
        "(function(){try{return JSON.stringify(JSON.parse(JSON.stringify(JSON.parse(x))))}catch(e){return 'E:'+e.name}})()",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(30).enumerate() {
            diff_batch(&format!("json parse chunk{}", k), ch, &exprs, f);
        }
    }
}

/* ==================================================================== REGEXP */

const PATTERNS: [&str; 26] = [
    "a",
    "a+b",
    "(a)(b)",
    "[a-z]+",
    "^ab",
    "b$",
    "a|b",
    "a*",
    "a?",
    ".",
    "\\\\d+",
    "\\\\w+",
    "\\\\s",
    "(?:ab)+",
    "a{2,3}",
    "[^a]",
    "\\\\bfoo\\\\b",
    "(a)(b)(c)",
    "x(?=y)",
    "x(?!y)",
    "\\\\u00e9",
    "[\\\\u0080-\\\\uffff]",
    "()",
    "(|a)",
    "\\\\n",
    "$",
];

fn regexp_inputs(rng: &mut Rng, n: usize) -> Vec<String> {
    let hay: [&str; 12] = [
        "",
        "a",
        "aaabbbccc",
        "xaaabz",
        "a\nb\nc",
        "ABC",
        "abcabc",
        "foo bar foo",
        "x\u{e9}y",
        "1 2 3",
        "\txy\n",
        "xyxy",
    ];
    let mut v: Vec<String> = Vec::new();
    for p in PATTERNS.iter() {
        for h in hay.iter() {
            v.push(format!("[\"{}\",{}]", p, jstr(h)));
        }
    }
    for _ in 0..n {
        let p = PATTERNS[rng.below(PATTERNS.len() as u64) as usize];
        let h = rng.string(10);
        v.push(format!("[\"{}\",{}]", p, jstr(&h)));
    }
    v
}

const RE_FLAGS: [&str; 8] = ["", "g", "i", "m", "gi", "gm", "im", "gim"];

/* rows 49-57: all 8 flag combinations through exec / test / lastIndex. */
#[test]
fn regexp_flags_exec_and_test() {
    let mut rng = Rng::new(SEED ^ 0x49);
    let inputs = regexp_inputs(&mut rng, 320);
    let mut exprs: Vec<String> = Vec::new();
    for fl in RE_FLAGS {
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return String(re)+'#'+re.source+'#'+re.global+re.ignoreCase+re.multiline+'#'+re.lastIndex}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');var m=re.exec(x[1]);return String(m)+'#'+(m?m.index+'/'+m.input+'/'+m.length:'-')+'#'+re.lastIndex}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return re.test(x[1])+','+re.test(x[1])+','+re.test(x[1])+'#'+re.lastIndex}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');var o='';for(var k=0;k<5;k++){{var m=re.exec(x[1]);o+=re.lastIndex+':'+String(m)+';';}}return o}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');re.lastIndex=2;var m=re.exec(x[1]);return String(m)+'#'+re.lastIndex}})()",
            fl
        ));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(45).enumerate() {
            diff_batch(&format!("regexp exec chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 49-57 continued: match / replace / split / search per flag combination. */
#[test]
fn regexp_flags_string_methods() {
    let mut rng = Rng::new(SEED ^ 0x50);
    let inputs = regexp_inputs(&mut rng, 200);
    let mut exprs: Vec<String> = Vec::new();
    for fl in RE_FLAGS {
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return String(x[1].match(re))}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].replace(re,'<$&>')}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].replace(re,'[$1|$2|$`|$\\'|$$|$9|$00]')}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].replace(re,function(){{var a=[];for(var i=0;i<arguments.length;i++)a.push(String(arguments[i]));return '{{'+a.join(',')+'}}'}})}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].split(re).join('#')+'|'+x[1].split(re).length}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].split(re,2).join('#')}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].search(re)}})()",
            fl
        ));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(40).enumerate() {
            diff_batch(&format!("regexp strmethods chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 58,59,60: RegExp construction, source escaping, invalid patterns/flags. */
#[test]
fn regexp_construction_and_errors() {
    diff_each(
        "regexp construction",
        &[
            "String(new RegExp())",
            "String(new RegExp(''))",
            "String(new RegExp('','g'))",
            "String(new RegExp('a','gim'))",
            "String(new RegExp('a','mig'))",
            "(function(){try{return String(new RegExp('a','gg'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('a','x'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('a','G'))}catch(e){return 'E:'+e.name}})()",
            "(function(){var a=/a/g;var b=new RegExp(a);return String(b)+'#'+(a===b)+'#'+b.global})()",
            "(function(){try{return String(new RegExp(/a/g,'i'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){var a=/a/;var b=RegExp(a);return a===b})()",
            "(function(){var b=RegExp('a','g');return String(b)})()",
            "String(new RegExp('a/b'))",
            "new RegExp('a/b').source",
            "/a\\/b/.source",
            "(function(){try{return String(new RegExp('('))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('[z-a]'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('a{2,1}'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('*'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('\\\\'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return String(new RegExp('(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)'))}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){var s='';for(var i=0;i<200;i++)s+='[a]';try{return String(new RegExp(s)).length}catch(e){return 'E:'+e.name}})()",
            "(function(){var s='';for(var i=0;i<300;i++)s+='(';for(i=0;i<300;i++)s+=')';try{return String(new RegExp(s)).length}catch(e){return 'E:'+e.name}})()",
            "Object.prototype.toString.call(/a/)",
            "(function(){var re=/a/g;var d=[];d.push(re.source,re.global,re.ignoreCase,re.multiline,re.lastIndex);return d.join('|')})()",
            "(function(){var re=/a/g;re.lastIndex=5;return re.lastIndex})()",
            "(function(){var re=/a/g;try{re.source='x'}catch(e){return 'E:'+e.name+':'+e.message}return re.source})()",
            "(function(){var re=/a/g;try{re.global=false}catch(e){return 'E:'+e.name}return re.global})()",
            "(function(){var re=/a/g;try{delete re.source}catch(e){return 'E:'+e.name}return re.source})()",
            "typeof RegExp.prototype.exec",
            "(function(){try{return RegExp.prototype.exec.call({},'a')}catch(e){return 'E:'+e.name}})()",
            "(function(){try{return String(/a/.exec())}catch(e){return 'E:'+e.name}})()",
            "String(/(a)(b)?/.exec('a'))",
            "String('abc'.match(/(a)(b)?/))",
            "'a1b2c'.split(/(\\d)/).join('|')",
            "'a1b2c'.split(/(\\d)?/).join('|')",
            "'a1b2c'.split(/\\d/,2).join('|')",
            "'abc'.split(/(?:)/).join('|')",
            "'abc'.split(/x*/).join('|')",
            "'ab'.split(/a*?/).join('|')",
            "'ab'.split(/a*/).join('|')",
            "''.split(/a/).length+','+''.split('').length+','+''.split('a').length",
            "'abc'.split(/b/).join('|')+'#'+'abc'.split('b').join('|')",
            "'aaa'.split(/a/).length",
            "'abc'.replace(/(b)/,'[$1$2$3]')",
            "'abc'.replace(/b/,'[$&$`$\\'$$]')",
            "'abc'.replace('b','[$&$`$\\'$$$1]')",
            "'aaa'.replace(/a/g,'')",
            "'aaa'.replace(/x*/g,'-')",
            "'abc'.replace(/(a)(b)(c)/,'$3$2$1')",
            "'abc'.replace(/(a)/,'$10$11$99')",
            "'abc'.replace(/b/g,function(m,o,s){return o+':'+s.length})",
            "String('abcabc'.match(/(a)(b)/g))",
            "'x'.replace(/(x)|(y)/,function(m,a,b){return String(a)+'/'+String(b)})",
        ],
    );
    for f in [0, JS_STRICT] {
        diff_eval("repr regexps", "[/a/,/a/g,/a/i,/a/m,/a/gi,/a/gm,/a/im,/a/gim]", f);
    }
}

/* ================================================== FUNCTIONS / CONTROL FLOW */

/* rows 185-187: apply / call / bind. */
#[test]
fn function_apply_call_bind() {
    diff_each(
        "apply",
        &[
            "(function(){function f(){return arguments.length+':'+Array.prototype.join.call(arguments,',')};return f.apply(null)})()",
            "(function(){function f(){return arguments.length};return f.apply(null,null)+','+f.apply(null,undefined)})()",
            "(function(){function f(){return Array.prototype.join.call(arguments,'|')};return f.apply(null,[1,2,3])})()",
            "(function(){function f(){return Array.prototype.join.call(arguments,'|')};return f.apply(null,{length:2,0:'a',1:'b'})})()",
            "(function(){function f(){return arguments.length};return f.apply(null,{length:-1})})()",
            "(function(){function f(){return arguments.length};return f.apply(null,{})})()",
            "(function(){function f(){return arguments.length};return f.apply(null,'abc')})()",
            "(function(){function f(){return typeof this};return f.apply(null)+','+f.apply(undefined)+','+f.apply(1)+','+f.apply('s')})()",
            "(function(){function f(){return String(this)};return f.apply({toString:function(){return 'T'}})})()",
            "(function(){try{return Function.prototype.apply.call(1,null,[])}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return ({}).apply}catch(e){return 'E:'+e.name}})()",
            "(function(){function f(a,b){return a+'/'+b};return f.apply(null,[1])})()",
            "Math.max.apply(null,[1,5,3])",
            "(function(){var a=[];for(var i=0;i<200;i++)a.push(i);return Math.max.apply(null,a)})()",
        ],
    );
    diff_each(
        "call",
        &[
            "(function(){function f(){return arguments.length};return f.call()+','+f.call(null)+','+f.call(null,1)+','+f.call(null,1,2,3)})()",
            "(function(){function f(a,b){return a+'/'+b};return f.call(null,1,2)})()",
            "(function(){function f(){return typeof this};return f.call()+','+f.call(null)+','+f.call(undefined)+','+f.call(1)+','+f.call('s')+','+f.call(true)})()",
            "(function(){function f(){return this===undefined?'undef':Object.prototype.toString.call(this)};return f.call(1)})()",
            "(function(){try{return Function.prototype.call.call(1)}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){'use strict';function f(){return typeof this};return f.call(1)+','+f.call(null)})()",
            "[].slice.call('abc').join('|')",
            "Object.prototype.toString.call(null)+','+Object.prototype.toString.call(undefined)",
        ],
    );
    diff_each(
        "bind",
        &[
            "(function(){function f(){return Array.prototype.join.call(arguments,'|')};return f.bind(null)()+'#'+f.bind(null)(1,2)})()",
            "(function(){function f(){return Array.prototype.join.call(arguments,'|')};return f.bind(null,'a')('b')+'#'+f.bind(null,'a','b','c')('d','e')})()",
            "(function(){function f(a,b,c){};return f.length+','+f.bind(null).length+','+f.bind(null,1).length+','+f.bind(null,1,2,3).length+','+f.bind(null,1,2,3,4,5).length})()",
            "(function(){function f(){return String(this)};return f.bind('T')()+'#'+f.bind({toString:function(){return 'O'}})()})()",
            "(function(){function f(){return typeof this};return f.bind(null)()+','+f.bind(undefined)()})()",
            "(function(){function F(a){this.a=a};var B=F.bind(null,7);var o=new B();return o.a+'#'+(o instanceof F)+'#'+(o instanceof B)})()",
            "(function(){function F(a,b){this.v=a+'/'+b};var B=F.bind(null,'x');var o=new B('y');return o.v})()",
            "(function(){function f(){};var b=f.bind(null);return Object.getOwnPropertyNames(b).sort().join(',')})()",
            "(function(){function f(){};var b=f.bind(null,1);var d=Object.getOwnPropertyDescriptor(b,'length');return d?[d.writable,d.enumerable,d.configurable].join(','):'none'})()",
            "(function(){function f(){};var b=f.bind(null);return typeof b+','+(b.prototype===f.prototype)})()",
            "(function(){try{return Function.prototype.bind.call(1)}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){function f(){return 'x'};return String(f.bind(null))})()",
            "(function(){function f(){return Array.prototype.join.call(arguments,'|')};var b=f.bind(null,1).bind(null,2);return b(3)})()",
        ],
    );
    /* row 188: the Function constructor */
    diff_each(
        "Function constructor",
        &[
            "(function(){var f=Function();return typeof f+','+String(f())})()",
            "(function(){var f=Function('return 1');return f()})()",
            "(function(){var f=Function('a','b','return a+b');return f(1,2)+'#'+f.length})()",
            "(function(){var f=Function('a,b','return a+b');return f(1,2)})()",
            "(function(){try{return Function('return')()}catch(e){return 'E:'+e.name}})()",
            "(function(){try{return Function('(')}catch(e){return 'E:'+e.name}})()",
            "(function(){try{return Function('a','b')}catch(e){return 'E:'+e.name}})()",
            "(function(){var f=new Function('return this');return typeof f()})()",
            "String(Function.prototype)",
            "typeof Function.prototype",
            "Function.prototype()",
            "(function(){function f(a,b){return a};return String(f)})()",
            "(function(){function f(a,b){return a};return f.length+','+f.name})()",
        ],
    );
}

/* rows 14,15,16,189,190: closures, lightweight vs heavyweight functions,
 * argument-count mismatches, control flow, strict directives. */
#[test]
fn closures_and_control_flow() {
    diff_each(
        "closures / arity",
        &[
            "(function(){function lw(a,b){return a+'/'+b};return lw()+'#'+lw(1)+'#'+lw(1,2)+'#'+lw(1,2,3,4)})()",
            "(function(){function hw(a,b){return arguments.length+':'+a+'/'+b};return hw()+'#'+hw(1)+'#'+hw(1,2)+'#'+hw(1,2,3,4)})()",
            "(function(){function hw(){return Array.prototype.join.call(arguments,'|')};return hw(1,2,3)})()",
            "(function(){function f(a){arguments[0]=9;return a};return f(1)})()",
            "(function(){function f(a){a=9;return arguments[0]};return f(1)})()",
            "(function(){function f(){return typeof arguments.callee};return f()})()",
            "(function(){function mk(){var n=0;return function(){return ++n}};var c=mk();return c()+','+c()+','+c()})()",
            "(function(){var fs=[];for(var i=0;i<3;i++)fs.push(function(){return i});return fs[0]()+','+fs[1]()+','+fs[2]()})()",
            "(function(){var fs=[];for(var i=0;i<3;i++)(function(j){fs.push(function(){return j})})(i);return fs[0]()+','+fs[1]()+','+fs[2]()})()",
            "(function(){function outer(){var x=1;function inner(){x++;return x};inner();return x+'/'+inner()};return outer()})()",
            "(function(){var o={n:1,get:function(){return this.n}};var g=o.get;return o.get()+','+typeof g})()",
            "(function(){function f(){return f2()};function f2(){return 'hoisted'};return f()})()",
            "(function(){var r='';try{r+='t';throw 1}catch(e){r+='c'+e}finally{r+='f'}return r})()",
            "(function(){var r='';for(var i=0;i<5;i++){if(i==1)continue;if(i==3)break;r+=i}return r})()",
            "(function(){var r='';outer:for(var i=0;i<3;i++){for(var j=0;j<3;j++){if(j==1)continue outer;if(i==2)break outer;r+=i+''+j}}return r})()",
            "(function(){var r='';switch(2){case 1:r+='1';case 2:r+='2';case 3:r+='3';break;default:r+='d'}return r})()",
            "(function(){var r='';switch('x'){default:r+='d';case 1:r+='1'}return r})()",
            "(function(){var i=0;do{i++}while(i<3);return i})()",
            "(function(){var r='';var o={a:1,b:2};for(var k in o)r+=k;return r})()",
            "(function(){var x=1;{var x=2}return x})()",
            "(function(){return (function(){return typeof undeclaredXyz})()})()",
            "(function(){var a=1,b=2;return [a&&b,a||b,!a,a?b:0,typeof void a].join('|')})()",
            "(function(){var n=0;while(n<10)n+=3;return n})()",
            "(function(){function fact(n){return n<=1?1:n*fact(n-1)};return fact(10)})()",
            "(function(){function fib(n){return n<2?n:fib(n-1)+fib(n-2)};return fib(18)})()",
            "(function(){var r=[];with({a:1,b:2}){r.push(a,b)};return r.join(',')})()",
            "(function(){var q=1;return eval('q+1')})()",
            "(function(){return typeof eval('var ev1=5;ev1')})()",
            "(function(){'use strict';return typeof this})()",
            "(function(){return typeof this})()",
            "function f(){'use strict'; return this===undefined} f()",
            "'use strict'; (function(){return typeof this})()",
            "(function(){var s='';try{null.x}catch(e){s=e.name+':'+e.message}return s})()",
            "(function(){var s='';try{undefinedFn()}catch(e){s=e.name+':'+e.message}return s})()",
            "(function(){var s='';try{(1)()}catch(e){s=e.name+':'+e.message}return s})()",
            "(function(){var s='';try{new 1}catch(e){s=e.name}return s})()",
            "(function(){var s='';try{({}).x.y}catch(e){s=e.name+':'+e.message}return s})()",
            "(function(){function f(){return f.caller===undefined};return typeof f()})()",
            "(function(){var x=0;var f=function g(n){return n?g(n-1)+1:0};return f(5)})()",
        ],
    );
    /* row 190: var re-declaration across scripts is not observable from one
     * eval, but the in-script equivalent is */
    diff_each(
        "var init",
        &[
            "var x=1; var x; x",
            "var x; x=1; var x=2; x",
            "(function(){return typeof y; var y=1})()",
            "(function(){var r=typeof z;var z=1;return r+'/'+z})()",
            "eval('var e1=1'); typeof e1",
            "(function(){eval('var e2=1');return typeof e2})()",
        ],
    );
}

/* Error objects and their string forms (row 143 as seen from JS). */
#[test]
fn errors_and_value_reprs() {
    tz_init();
    diff_each(
        "errors",
        &[
            "String(new Error())",
            "String(new Error(''))",
            "String(new Error('msg'))",
            "[new Error('m'),new EvalError('m'),new RangeError('m'),new ReferenceError('m'),new SyntaxError('m'),new TypeError('m'),new URIError('m')].join('|')",
            "(function(){var e=new Error('m');return e.name+'/'+e.message+'/'+(e instanceof Error)+'/'+typeof e.stackTrace})()",
            "(function(){var e=new Error('m');e.name='';return String(e)})()",
            "(function(){var e=new Error('m');e.message='';return String(e)})()",
            "(function(){var e=new Error();e.name='';e.message='';return String(e)})()",
            "(function(){var e=new TypeError('m');return Object.prototype.toString.call(e)})()",
            "(function(){try{null.x}catch(e){return e instanceof TypeError}})()",
            "(function(){try{throw 1}catch(e){return typeof e}})()",
            "(function(){try{throw 'str'}catch(e){return e}})()",
            "(function(){try{throw {a:1}}catch(e){return JSON.stringify(e)}})()",
            "Error.prototype.name+'/'+Error.prototype.message",
            "(function(){var e=Error('m');return String(e)+'/'+(e instanceof Error)})()",
            "JSON.stringify(new Error('m'))",
        ],
    );
    /* row 124: the reprvalue classes reachable from JS in one array */
    for f in [0, JS_STRICT] {
        diff_eval(
            "repr zoo",
            "[undefined,null,true,false,0,-0,NaN,'s',{},[],function(){},Math,JSON,/a/g,new Error('m'),new Error(),new Date(0),new Number(1),new String('s'),new Boolean(true),{a:1,'b c':2,'12':3,'':4,_d1:5}]",
            f,
        );
        diff_eval("repr cycle obj", "(function(){var o={};o.self=o;return o})()", f);
        diff_eval("repr cycle arr", "(function(){var a=[];a.push(a);return a})()", f);
        diff_eval("repr nested", "({a:{b:{c:[1,[2,[3]]]}}})", f);
        diff_eval(
            "repr keys",
            "({abc:1,_a1:2,'12':3,'a b':4,'':5,'1a':6,'\\u00e9':7})",
            f,
        );
    }
}

/* Operators / equality / comparison driven from JS over a value zoo. */
#[test]
fn operators_over_js_value_zoo() {
    let zoo = strs(&[
        "undefined",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1",
        "-1",
        "NaN",
        "Infinity",
        "''",
        "'0'",
        "'1'",
        "'abc'",
        "'123456789012345'",
        "'1234567890123456'",
        "[]",
        "[0]",
        "[1,2]",
        "({})",
        "({valueOf:function(){return 1}})",
        "({toString:function(){return '1'}})",
        "(function(){})",
        "new Number(1)",
        "new String('1')",
        "new Boolean(false)",
    ]);
    let mut exprs: Vec<String> = Vec::new();
    for b in zoo.iter() {
        for op in [
            "==", "!=", "===", "!==", "<", "<=", ">", ">=", "+", "-", "*", "/", "%", "&", "|", "^",
        ] {
            exprs.push(format!("String(x {} ({}))", op, b));
        }
        exprs.push(format!("String(x && ({}))+'/'+String(x || ({})) ", b, b));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in exprs.chunks(80).enumerate() {
            diff_batch(&format!("operators chunk{}", k), &zoo, ch, f);
        }
    }
    diff_each(
        "unary / misc operators",
        &[
            "[typeof undefined,typeof null,typeof true,typeof 1,typeof 'a',typeof {},typeof [],typeof function(){},typeof /a/].join('|')",
            "(function(){var r=[];var vs=[undefined,null,0,-0,NaN,'',{},[]];for(var i=0;i<vs.length;i++)r.push(!vs[i]);return r.join(',')})()",
            "[+'1',-'1',+'',+'a',+[],+[1],+{},+true,+null,+undefined].join('|')",
            "['a' in {a:1},'b' in {a:1},0 in [1],1 in [1],'length' in []].join('|')",
            "[1 instanceof Object,({}) instanceof Object,[] instanceof Array,(function(){}) instanceof Function].join('|')",
            "(function(){try{return 1 instanceof 2}catch(e){return 'E:'+e.name+':'+e.message}})()",
            "(function(){try{return 'a' in 1}catch(e){return 'E:'+e.name}})()",
            "[void 0,~5,-(-5),!!'x'].join('|')",
            "(function(){var i=5;return [i++,i,++i,i--,i,--i].join('|')})()",
            "(function(){var o={n:1};return [o.n++,o.n,++o.n,o.n].join('|')})()",
            "(function(){var a=[1];return [a[0]++,a[0],++a[0]].join('|')})()",
        ],
    );
}


/* ============================================== ADDITIONAL RANDOMIZED STRESS */

/// Doubles that sit exactly on the branch boundaries of `jsV_numbertostring`
/// (`point < -5`, `point > 21`, `point <= 0`, normal + zero fill) and of
/// `toFixed`/`toExponential`/`toPrecision`.
fn boundary_numbers() -> Vec<String> {
    let mut v: Vec<f64> = Vec::new();
    /* every decimal exponent */
    let mut k = -323i32;
    while k <= 308 {
        v.push(10f64.powi(k));
        v.push(-10f64.powi(k));
        v.push(1.2345678901234567 * 10f64.powi(k));
        k += 1;
    }
    /* every 7th binary exponent */
    let mut e = -1074i32;
    while e <= 1023 {
        v.push(2f64.powi(e));
        e += 7;
    }
    /* the exact branch boundaries */
    for x in [
        1e-7, 9.999999e-8, 1.0000001e-7, 1e-6, 0.000001, 0.0000009, 1e21, 1e20, 9.999999e20,
        1.0000001e21, 1e22, 999999999999999999999.0, 1000000000000000000000.0,
        (1u64 << 53) as f64, ((1u64 << 53) - 1) as f64, ((1u64 << 53) + 2) as f64,
        f64::MAX, f64::MIN_POSITIVE, 5e-324, 4.9e-324, 1.5, 2.5, 0.5, 1.005, 1.0 / 3.0,
        2.0 / 3.0, 1e15, 1e16, 1e17, 123456.789, 0.1 + 0.2,
    ] {
        v.push(x);
        v.push(-x);
    }
    v.iter().map(|x| jsnum(*x)).collect()
}

/* rows 103,106,110-113: dtoa / numtostr stress on the branch boundaries. */
#[test]
fn numbers_dtoa_boundary_stress() {
    let inputs = boundary_numbers();
    let mut exprs = strs(&[
        "String(x)",
        "JSON.stringify(x)",
        "x.toString()",
        "Number(String(x))===x",
        "String(x*10)+'/'+String(x/10)",
        "String(x+1)+'/'+String(x-1)",
    ]);
    for w in [0, 1, 2, 3, 6, 10, 17, 20] {
        exprs.push(format!("x.toFixed({})", w));
        exprs.push(format!("x.toExponential({})", w));
    }
    for w in [1, 2, 3, 6, 10, 17, 21] {
        exprs.push(format!("x.toPrecision({})", w));
    }
    for radix in [2, 3, 7, 8, 16, 20, 36] {
        exprs.push(format!("x.toString({})", radix));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(120).enumerate() {
            diff_batch(&format!("dtoa boundary chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 99,101: jsU_toupperrune_full / jsU_tolowerrune_full through
 * String.prototype.toUpperCase / toLowerCase over a wide code-point sweep. */
#[test]
fn strings_case_conversion_sweep() {
    let mut bases: Vec<u32> = (0..0x3000u32).step_by(0x200).collect();
    bases.extend_from_slice(&[
        0xA600, 0xA700, 0xAB50, 0xFB00, 0xFF00, 0x10400, 0x104B0, 0x10C80, 0x118A0, 0x16E40,
        0x1E900, 0xD800,
    ]);
    for f in [0, JS_STRICT] {
        for b in bases.iter() {
            let src = format!(
                "(function(){{function cc(s){{var r='';for(var i=0;i<s.length;i++)r+=s.charCodeAt(i).toString(16)+'.';return r}}\
                 var out=[];for(var k={};k<{};k++){{var s=String.fromCharCode(k);\
                 out.push(k.toString(16)+'>'+cc(s)+'>'+cc(s.toUpperCase())+'/'+cc(s.toLowerCase())+'/'+s.length+'/'+s.toUpperCase().length)}}\
                 return out.join('~')}})()",
                b,
                b + 0x200
            );
            diff_ok("case sweep", &src, f);
        }
    }
    /* real code points (including astral) emitted as raw UTF-8 in the source */
    let mut rng = Rng::new(SEED ^ 0xCA5E);
    let mut inputs: Vec<String> = Vec::new();
    for _ in 0..400 {
        let mut s = String::new();
        for _ in 0..1 + rng.below(4) {
            let cp = match rng.below(4) {
                0 => rng.below(0x80) as u32,
                1 => rng.below(0x800) as u32,
                2 => rng.below(0x10000) as u32,
                _ => rng.below(0x110000) as u32,
            };
            s.push(char::from_u32(cp).unwrap_or('?'));
        }
        inputs.push(jstr(&s));
    }
    let exprs = strs(&[
        "x.toUpperCase()+'|'+x.toLowerCase()",
        "x.toUpperCase().length+'/'+x.toLowerCase().length+'/'+x.length",
        "x.toUpperCase().toLowerCase()===x.toLowerCase()",
        "x.toLowerCase().toUpperCase()===x.toUpperCase()",
        "JSON.stringify(x.toUpperCase())",
        "(function(){var r='';var u=x.toUpperCase();for(var i=0;i<u.length;i++)r+=u.charCodeAt(i).toString(16)+'.';return r})()",
        "x.localeCompare(x.toUpperCase())",
        "encodeURIComponent(x).length",
    ]);
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(100).enumerate() {
            diff_batch(&format!("case random chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 49-59 continued: a second, harder pattern set (backreferences, nested
 * quantifiers, empty matches, classes, boundaries) against random haystacks. */
#[test]
fn regexp_pattern_stress() {
    const P2: [&str; 34] = [
        "(a)\\\\1",
        "(a+)(b+)?\\\\2",
        "(?:a|b)*c",
        "a{0,}",
        "a{1,}",
        "(a*)*",
        "(a|)*",
        "[\\\\d\\\\w\\\\s]+",
        "[^\\\\d\\\\w]",
        "\\\\D+",
        "\\\\S+",
        "\\\\W+",
        "\\\\B",
        "\\\\b",
        "[\\\\b]",
        "(?:)",
        "^$",
        "^",
        "^.*$",
        "(.)(.)(.)",
        "[a-c-e]",
        "[]]",
        "[.]",
        "\\\\.",
        "a(?=b)(?!c)",
        "[\\\\x41-\\\\x5a]+",
        "\\\\0",
        "\\\\cA",
        "[\\\\-]",
        "(a)|(b)",
        "((((a))))",
        "x*?",
        "a+?b",
        "[\\\\u00c0-\\\\u00ff]+",
    ];
    let mut rng = Rng::new(SEED ^ 0x5717);
    let hay: [&str; 14] = [
        "",
        "a",
        "aa",
        "aab",
        "abab",
        "aaabbb",
        "abbaab",
        "\n",
        "a\n\nb",
        "A-Z",
        "\u{c0}\u{e9}\u{ff}z",
        "x.y",
        "]]",
        "\t \u{b}\u{c}",
    ];
    let mut inputs: Vec<String> = Vec::new();
    for p in P2.iter() {
        for h in hay.iter() {
            inputs.push(format!("[\"{}\",{}]", p, jstr(h)));
        }
    }
    for _ in 0..200 {
        let p = P2[rng.below(P2.len() as u64) as usize];
        inputs.push(format!(
            "[\"{}\",{}]",
            p,
            jstr(&rng.string(10))
        ));
    }
    let mut exprs: Vec<String> = Vec::new();
    for fl in ["", "g", "i", "m", "gim"] {
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');var m=re.exec(x[1]);return String(m)+'#'+(m?m.index:'-')}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return String(x[1].match(re))}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].replace(re,'<$&>')}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].split(re).join('#')}})()",
            fl
        ));
        exprs.push(format!(
            "(function(){{var re=new RegExp(x[0],'{}');return x[1].search(re)+'/'+re.test(x[1])}})()",
            fl
        ));
    }
    for f in [0, JS_STRICT] {
        for (k, ch) in inputs.chunks(80).enumerate() {
            diff_batch(&format!("regexp stress chunk{}", k), ch, &exprs, f);
        }
    }
}

/* rows 87,88,89: `jsV_resizearray` (dense loop vs own-iterator branch) and
 * sparse-array iteration order, all from JS. */
#[test]
fn arrays_sparse_resize_and_iteration_order() {
    diff_each(
        "sparse resize",
        &[
            "(function(){var a=[];a[999]=1;a[500]=2;a[3]=3;a.length=10;return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];for(var i=0;i<20;i++)a[i*3]=i;a.length=10;return Object.keys(a).join(',')})()",
            "(function(){var a=[];a[5]=1;for(var i=0;i<6;i++)a[i]=i;a.length=3;return a.join('|')+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[2]=1;a['x']=2;a['3x']=3;a.length=1;return Object.keys(a).join(',')})()",
            "(function(){var a=[];a[1000000]=1;return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[1000000]=1;a.length=5;return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[1,2,3,4,5];delete a[2];a.length=2;return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[3]=1;var s='';for(var k in a)s+=k+',';return s})()",
            "(function(){var a=[1,2,3];a.foo=1;var s='';for(var k in a)s+=k+',';return s})()",
            "(function(){var a=[];try{a[4294967294]=1}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=[];try{a[4294967295]=1}catch(e){return 'E:'+e.name}return a.length})()",
            "(function(){var a=new Array(100);var n=0;for(var k in a)n++;return a.length+'#'+n})()",
            "(function(){var a=[];a[10]=1;return a.join('|')+'#'+String(a)})()",
            "(function(){var a=[];a[10]=1;return JSON.stringify(a)})()",
            "(function(){var a=[];a[10]=1;return a.slice(8,12).length+'#'+a.indexOf(1)+'#'+a.lastIndexOf(1)})()",
            "(function(){var a=[];a[10]=1;return a.reverse().join('|')+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[10]=1;return a.sort().length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[10]=1;return a.splice(5,3).length+'#'+a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[10]=1;var n=0;a.forEach(function(){n++});return n})()",
            "(function(){var a=[];a[10]=1;return a.map(function(v){return v}).length+'#'+a.filter(function(){return true}).length})()",
            "(function(){var a=[];a[10]=1;return String(a.reduce(function(p,c){return p+'/'+c},'S'))})()",
            "(function(){var a=[];a[10]=1;return a.every(function(){return false})+','+a.some(function(){return true})})()",
            "(function(){var a=[];a[10]=1;a.unshift(0);return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[10]=1;return String(a.shift())+'#'+a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[];a[10]=1;return String(a.pop())+'#'+a.length})()",
            "(function(){var a=[];a[10]=1;return a.concat([1]).length})()",
            "(function(){var a=[];for(var i=0;i<9;i++)a.push(i);a[20]=1;a.length=9;return a.join('|')})()",
            "(function(){var a=[];for(var i=0;i<40;i++)a.push(i);a.length=17;return a.length+'#'+a[16]+'#'+String(a[17])})()",
            "(function(){var a=[0,1,2,3,4];delete a[4];return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[0,1,2,3,4];delete a[0];return a.length+'#'+Object.keys(a).join(',')})()",
            "(function(){var a=[0,1,2];Object.defineProperty(a,'1',{value:9,enumerable:false});return a.join('|')+'#'+Object.keys(a).join(',')})()",
        ],
    );
}
