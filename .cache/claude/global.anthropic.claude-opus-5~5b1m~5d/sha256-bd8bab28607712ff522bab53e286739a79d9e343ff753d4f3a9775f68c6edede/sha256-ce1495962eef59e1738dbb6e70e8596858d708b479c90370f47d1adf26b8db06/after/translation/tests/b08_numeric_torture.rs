//! Numeric-argument torture tests.
//!
//! Every builtin that takes a numeric argument is called with an aggressive
//! sweep of extreme values. This is the class of test that catches
//! `double -> narrow int` conversion mismatches: Rust's `f64 as uN/iN`
//! SATURATES (and maps NaN to 0) whereas C truncates toward zero and wraps.
//! A single such mismatch was found this way in the `RegExp.lastIndex` setter.
mod common;
use common::*;
use std::ffi::c_int;

/// Values that sit exactly on, or wildly outside, every integer boundary the C
/// converts through: char, short, ushort, int, uint, and the double integer
/// limits. Written as JS literals.
fn extreme_number_literals() -> Vec<String> {
    let fixed: &[&str] = &[
        "0", "-0", "1", "-1", "2", "-2", "0.5", "-0.5", "1.5", "-1.5", "0.9999999999", "1e-323",
        "-1e-323", "NaN", "Infinity", "-Infinity",
        // char / byte boundaries
        "127", "128", "255", "256", "-128", "-129", "-255", "-256",
        // short / ushort boundaries
        "32767", "32768", "-32768", "-32769", "65535", "65536", "65537", "-65535", "-65536",
        "-65537", "131071", "131072", "70000", "-70000",
        // int / uint boundaries
        "2147483646", "2147483647", "2147483648", "2147483649", "-2147483647", "-2147483648",
        "-2147483649", "4294967294", "4294967295", "4294967296", "4294967297", "-4294967295",
        "-4294967296", "8589934592", "-8589934592",
        // double integer limits and beyond
        "9007199254740991", "9007199254740992", "9007199254740993", "-9007199254740992",
        "1e15", "1e16", "1e17", "1e18", "1e19", "1e20", "1e21", "1e30", "1e100", "1e300",
        "-1e15", "-1e20", "-1e100", "-1e300", "1.7976931348623157e308",
        "-1.7976931348623157e308",
        // fractional near boundaries (truncation direction matters)
        "32767.9", "-32768.9", "65535.9", "-65535.9", "2147483647.9", "-2147483648.9",
        "4294967295.9", "-0.9", "-1.9", "0.9",
    ];
    let mut v: Vec<String> = fixed.iter().map(|s| s.to_string()).collect();
    // A deterministic random tail, so the sweep is not only hand-picked.
    let mut rng = Rng::new(0x7071_2001);
    for _ in 0..40 {
        let x = rng.finite_f64();
        v.push(format!("{:e}", x));
    }
    v
}

/// Run `template` once per extreme number, substituting `{}` with the literal.
fn sweep(b: &mut Batch, template: &str) {
    for lit in extreme_number_literals() {
        let src = template.replace("{N}", &lit);
        b.script(0, &src);
        b.script(JS_STRICT, &src);
    }
}

fn sweep_all(name: &str, templates: &[&str]) {
    let mut b = Batch::new();
    for t in templates {
        sweep(&mut b, t);
    }
    b.finish(name);
}

// ---------------------------------------------------------------------------
// RegExp.lastIndex -- the site where the bug was found. Kept as a regression.
// ---------------------------------------------------------------------------

#[test]
fn regexp_lastindex_numeric_torture() {
    // `js_Regexp.last` is `unsigned short` (jsi.h:367) and the C assigns a
    // double to it, so the value is reduced MODULO 2^16, not clamped.
    sweep_all(
        "RegExp.lastIndex numeric torture",
        &[
            "(function(){var r=/a/; r.lastIndex={N}; return String(r.lastIndex)})()",
            "(function(){var r=/a/g; r.lastIndex={N}; return String(r.lastIndex)})()",
            "(function(){var r=/a/gi; r.lastIndex={N}; return String(r.lastIndex)})()",
            "(function(){var r=/a/gm; r.lastIndex={N}; return String(r.lastIndex)})()",
            "(function(){var r=/a/; r.lastIndex={N}; var m=r.exec('xyz'); return String(m)+'/'+r.lastIndex})()",
            "(function(){var r=/a/g; r.lastIndex={N}; var m=r.exec('xyzabc'); return String(m)+'/'+r.lastIndex})()",
            "(function(){var r=/a/g; r.lastIndex={N}; return r.test('xyzabc')+'/'+r.lastIndex})()",
            "(function(){var r=/a/g; r.lastIndex={N}; return String('aaaa'.replace(r,'X'))+'/'+r.lastIndex})()",
            "(function(){var r=/a/g; r.lastIndex={N}; return String('aaaa'.match(r))+'/'+r.lastIndex})()",
            "(function(){var r=/(a)(b)?/g; r.lastIndex={N}; var m=r.exec('ab'); return String(m)+'/'+r.lastIndex})()",
        ],
    );
}

// ---------------------------------------------------------------------------
// Array
// ---------------------------------------------------------------------------

#[test]
fn array_numeric_torture() {
    sweep_all(
        "Array numeric torture",
        &[
            "(function(){try{return String(new Array({N}).length)}catch(e){return e.name+': '+e.message}})()",
            "(function(){var a=[1,2,3,4,5]; try{a.length={N}}catch(e){return e.name+': '+e.message} return a.length+'/'+a.join('|')})()",
            "String([1,2,3,4,5].slice({N}))",
            "String([1,2,3,4,5].slice(0,{N}))",
            "String([1,2,3,4,5].slice({N},{N}))",
            "(function(){var a=[1,2,3,4,5]; a.splice({N}); return a.join('|')})()",
            "(function(){var a=[1,2,3,4,5]; a.splice(0,{N}); return a.join('|')})()",
            "(function(){var a=[1,2,3,4,5]; a.splice({N},{N}); return a.join('|')})()",
            "(function(){var a=[1,2,3,4,5]; a.splice({N},1,'x'); return a.join('|')})()",
            "String([1,2,3,4,5].indexOf(3,{N}))",
            "String([1,2,3,4,5].lastIndexOf(3,{N}))",
            "(function(){var a=[]; a[{N}]=1; return a.length+'/'+Object.keys(a).join('|')})()",
            "(function(){var a=[1,2,3]; return String(a[{N}])})()",
            "String([1,2,3].concat({N}))",
            "(function(){var a=[3,1,2]; a.sort(function(x,y){return {N}}); return a.join('|')})()",
        ],
    );
}

// ---------------------------------------------------------------------------
// String
// ---------------------------------------------------------------------------

#[test]
fn string_numeric_torture() {
    sweep_all(
        "String numeric torture",
        &[
            "String('abcde'.charAt({N}))",
            "String('abcde'.charCodeAt({N}))",
            "String('abcde'.slice({N}))",
            "String('abcde'.slice(0,{N}))",
            "String('abcde'.slice({N},{N}))",
            "String('abcde'.substring({N}))",
            "String('abcde'.substring(0,{N}))",
            "String('abcde'.substring({N},{N}))",
            "String('abcde'.substr({N}))",
            "String('abcde'.substr(0,{N}))",
            "String('abcde'.substr({N},{N}))",
            "String('abcde'.indexOf('c',{N}))",
            "String('abcde'.lastIndexOf('c',{N}))",
            "String('a,b,c'.split(',',{N}))",
            "String('abcde'[{N}])",
            "String(String.fromCharCode({N}))",
            "String(String.fromCharCode({N},{N}))",
            "String(String.fromCharCode(65,{N},66))",
            // multi-byte, so index arithmetic goes through the UTF-8 helpers
            "String('a\\u00e9\\u4f60\\ud83d\\ude00b'.charAt({N}))",
            "String('a\\u00e9\\u4f60\\ud83d\\ude00b'.charCodeAt({N}))",
            "String('a\\u00e9\\u4f60\\ud83d\\ude00b'.slice({N}))",
            "String('a\\u00e9\\u4f60\\ud83d\\ude00b'.substring({N},{N}))",
        ],
    );
}

// ---------------------------------------------------------------------------
// Number formatting
// ---------------------------------------------------------------------------

#[test]
fn number_method_numeric_torture() {
    sweep_all(
        "Number method numeric torture",
        &[
            "(function(){try{return (123.456).toString({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (0).toString({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (-1.5).toString({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (123.456).toFixed({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (0).toFixed({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (-0.5).toFixed({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (123.456).toExponential({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return (123.456).toPrecision({N})}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return ({N}).toFixed(2)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return ({N}).toExponential(3)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return ({N}).toPrecision(5)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return ({N}).toString(16)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return ({N}).toString(36)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return ({N}).toString(2)}catch(e){return e.name+': '+e.message}})()",
            "String(parseInt('123',{N}))",
            "String(parseInt(String({N})))",
            "String(parseFloat(String({N})))",
            "String(Number(String({N})))",
        ],
    );
}

// ---------------------------------------------------------------------------
// Math
// ---------------------------------------------------------------------------

#[test]
fn math_numeric_torture() {
    sweep_all(
        "Math numeric torture",
        &[
            "String(Math.abs({N}))",
            "String(Math.ceil({N}))",
            "String(Math.floor({N}))",
            "String(Math.round({N}))",
            "String(Math.sqrt({N}))",
            "String(Math.exp({N}))",
            "String(Math.log({N}))",
            "String(Math.sin({N}))",
            "String(Math.cos({N}))",
            "String(Math.tan({N}))",
            "String(Math.asin({N}))",
            "String(Math.acos({N}))",
            "String(Math.atan({N}))",
            "String(Math.atan2({N},1))",
            "String(Math.atan2(1,{N}))",
            "String(Math.pow({N},2))",
            "String(Math.pow(2,{N}))",
            "String(Math.pow({N},{N}))",
            "String(Math.max({N},1))",
            "String(Math.min({N},1))",
            "String(Math.max(1,2,{N},3))",
            "String(Math.min(1,2,{N},3))",
        ],
    );
}

// ---------------------------------------------------------------------------
// Bitwise operators -- these go through ToInt32 / ToUint32
// ---------------------------------------------------------------------------

#[test]
fn bitwise_numeric_torture() {
    sweep_all(
        "bitwise numeric torture",
        &[
            "String({N} | 0)",
            "String({N} & -1)",
            "String({N} ^ 0)",
            "String(~{N})",
            "String({N} << 0)",
            "String({N} << 1)",
            "String({N} << 31)",
            "String({N} >> 0)",
            "String({N} >> 1)",
            "String({N} >> 31)",
            "String({N} >>> 0)",
            "String({N} >>> 1)",
            "String({N} >>> 31)",
            "String(1 << {N})",
            "String(-1 >>> {N})",
            "String(255 >> {N})",
        ],
    );
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

#[test]
fn json_numeric_torture() {
    sweep_all(
        "JSON numeric torture",
        &[
            "String(JSON.stringify({N}))",
            "String(JSON.stringify([{N}]))",
            "String(JSON.stringify({a:{N}}))",
            "String(JSON.stringify({a:1,b:[2,3]},null,{N}))",
            "String(JSON.stringify({a:1,b:[2,3]},null,String({N})))",
            "(function(){try{return String(JSON.parse(String({N})))}catch(e){return e.name+': '+e.message}})()",
        ],
    );
}

// ---------------------------------------------------------------------------
// Date -- the heaviest user of double -> int conversions
// ---------------------------------------------------------------------------

#[test]
fn date_numeric_torture() {
    // NOTE: only timezone-stable observations are compared as raw values; both
    // impls run in the SAME process so the local timezone is identical, which
    // makes the local-time accessors comparable too. `Date.now()` and
    // `new Date()` are excluded because they are not deterministic.
    sweep_all(
        "Date numeric torture",
        &[
            "String(new Date({N}).getTime())",
            "String(new Date({N}).valueOf())",
            "String(new Date({N}).toISOString ? (function(){try{return new Date({N}).toISOString()}catch(e){return e.name}})() : 'n/a')",
            "String(new Date({N}).toUTCString())",
            "String(new Date({N}).getUTCFullYear())",
            "String(new Date({N}).getUTCMonth())",
            "String(new Date({N}).getUTCDate())",
            "String(new Date({N}).getUTCDay())",
            "String(new Date({N}).getUTCHours())",
            "String(new Date({N}).getUTCMinutes())",
            "String(new Date({N}).getUTCSeconds())",
            "String(new Date({N}).getUTCMilliseconds())",
            "String(new Date({N}).getTimezoneOffset())",
            "String(Date.UTC({N},0))",
            "String(Date.UTC(2000,{N}))",
            "String(Date.UTC(2000,0,{N}))",
            "String(Date.UTC(2000,0,1,{N}))",
            "String(Date.UTC(2000,0,1,0,{N}))",
            "String(Date.UTC(2000,0,1,0,0,{N}))",
            "String(Date.UTC(2000,0,1,0,0,0,{N}))",
            "String(new Date({N},0).getTime())",
            "String(new Date(2000,{N}).getTime())",
            "String(new Date(2000,0,{N}).getTime())",
            "(function(){var d=new Date(0); d.setTime({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCFullYear({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCMonth({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCDate({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCHours({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCMinutes({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCSeconds({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCMilliseconds({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setFullYear({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setMonth({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setDate({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setHours({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setMinutes({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setSeconds({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setMilliseconds({N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCHours(1,{N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCHours(1,2,{N}); return String(d.getTime())})()",
            "(function(){var d=new Date(0); d.setUTCHours(1,2,3,{N}); return String(d.getTime())})()",
            "String(new Date({N}).toString())",
            "String(new Date({N}).toDateString())",
            "String(new Date({N}).toTimeString())",
            "String(new Date({N}).toLocaleString())",
            "String(new Date({N}).toJSON ? (function(){try{return String(new Date({N}).toJSON())}catch(e){return e.name}})() : 'n/a')",
            "String(Date.parse(new Date({N}).toUTCString()))",
        ],
    );
}

// ---------------------------------------------------------------------------
// Date range boundary: the +/-8.64e15 valid-time-value limit
// ---------------------------------------------------------------------------

#[test]
fn date_range_boundary_matches() {
    let mut b = Batch::new();
    let boundaries: &[&str] = &[
        "8639999999999999", "8640000000000000", "8640000000000001", "8640000000000002",
        "-8639999999999999", "-8640000000000000", "-8640000000000001", "-8640000000000002",
        "8.64e15", "-8.64e15", "8.64e15+1", "-8.64e15-1", "1e16", "-1e16",
        "8640000000000000.5", "-8640000000000000.5",
    ];
    for lit in boundaries {
        for t in [
            "String(new Date({N}).getTime())",
            "String(new Date({N}).toUTCString())",
            "(function(){try{return new Date({N}).toISOString()}catch(e){return e.name+': '+e.message}})()",
            "(function(){var d=new Date(0); d.setTime({N}); return String(d.getTime())})()",
            "String(new Date({N}).getUTCFullYear())",
        ] {
            let src = t.replace("{N}", lit);
            b.script(0, &src);
            b.script(JS_STRICT, &src);
        }
    }
    b.finish("Date valid-range boundary");
}

// ---------------------------------------------------------------------------
// Array length / index at the 2^32 and JS_ARRAYLIMIT boundaries
// ---------------------------------------------------------------------------

#[test]
fn array_length_boundary_matches() {
    let mut b = Batch::new();
    let lits: &[&str] = &[
        "0", "1", "67108863", "67108864", "67108865", "4294967294", "4294967295", "4294967296",
        "4294967297", "-1", "1.5", "NaN", "Infinity", "-Infinity", "1e21",
    ];
    for lit in lits {
        for t in [
            "(function(){try{var a=[]; a.length={N}; return String(a.length)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{return String(new Array({N}).length)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{var a=[]; a[{N}]=1; return a.length+'/'+Object.keys(a).length}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{var a=[1]; a[{N}]=1; return String(a.length)}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{var a=[1,2,3]; return String(a[{N}])}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{var a=[1,2,3]; delete a[{N}]; return a.length+'/'+a.join('|')}catch(e){return e.name+': '+e.message}})()",
            "(function(){try{var o={}; o[{N}]=1; return Object.keys(o).join('|')}catch(e){return e.name+': '+e.message}})()",
        ] {
            let src = t.replace("{N}", lit);
            b.script(0, &src);
            b.script(JS_STRICT, &src);
        }
    }
    b.finish("Array length/index boundary");
}

// ---------------------------------------------------------------------------
// String.length / index boundary and js_utflen / js_runeat via JS
// ---------------------------------------------------------------------------

#[test]
fn string_index_boundary_matches() {
    let mut b = Batch::new();
    let strings: &[&str] = &[
        "''",
        "'a'",
        "'abc'",
        "'\\u00e9'",
        "'\\u4f60\\u597d'",
        "'\\ud83d\\ude00'",
        "'a\\u00e9\\u4f60\\ud83d\\ude00z'",
        "'\\u0000ab'",
    ];
    let idx: &[&str] = &[
        "-2", "-1", "-0", "0", "1", "2", "3", "4", "5", "6", "7", "0.5", "1.9", "NaN", "Infinity",
        "-Infinity", "65535", "65536", "2147483647", "2147483648", "-2147483648", "4294967296",
    ];
    for s in strings {
        for i in idx {
            for t in [
                "String({S}.charAt({I}))",
                "String({S}.charCodeAt({I}))",
                "String({S}[{I}])",
                "String({S}.slice({I}))",
                "String({S}.substring({I}))",
                "String({S}.substr({I}))",
                "String({S}.indexOf('a',{I}))",
                "String({S}.lastIndexOf('a',{I}))",
                "String({S}.split('',{I}))",
            ] {
                let src = t.replace("{S}", s).replace("{I}", i);
                b.script(0, &src);
            }
        }
        b.script(0, &format!("String({s}.length)"));
        b.script(0, &format!("String({s}.toUpperCase())"));
        b.script(0, &format!("String({s}.toLowerCase())"));
        b.script(0, &format!("String(escape({s}))"));
        b.script(0, &format!("String(encodeURIComponent({s}))"));
        b.script(0, &format!("String(JSON.stringify({s}))"));
    }
    b.finish("String index boundary");
}

/// Sanity: the sweep really is running a large number of distinct cases.
#[test]
fn torture_sweep_is_large() {
    let n = extreme_number_literals().len();
    assert!(n >= 120, "extreme literal set is too small: {n}");
    eprintln!("{n} extreme numeric literals per template");
}

#[allow(dead_code)]
fn _unused(_: c_int) {}
