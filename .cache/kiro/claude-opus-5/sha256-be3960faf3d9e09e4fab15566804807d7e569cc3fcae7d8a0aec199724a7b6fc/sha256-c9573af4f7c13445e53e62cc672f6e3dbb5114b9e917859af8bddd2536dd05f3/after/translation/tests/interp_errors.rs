//! Phase C — error-path differential tests.
//!
//! One entry per distinct rejection in `ERRORS.md` section 5 (the interpreter
//! throw sites) plus the lexer/parser/compiler diagnostics. Each entry runs the
//! exact invalid input through BOTH `.so`s and asserts they agree on the return
//! code AND the exact `"<Type>: <message>"` string, not merely that both failed.

mod common;

use common::*;

/// `(label, snippet)`. Each snippet is run with `assert_same_program`, which
/// compares `js_dostring`'s return code, the `js_report` text and the `__out`
/// accumulator between C and Rust.
type Cases = &'static [(&'static str, &'static str)];

fn run_cases(cases: Cases) {
    for flags in [0, 1] {
        for (label, src) in cases {
            assert_same_program(flags, label, src);
        }
    }
}

// ===========================================================================
// jsvalue.c — coercion errors
// ===========================================================================

#[test]
fn err_jsvalue() {
    let cases: Cases = &[
        ("jsvalue:144 cannot convert object to primitive",
         "ok(function(){ return Object.create(null) + 1 });\n\
          ok(function(){ return String(Object.create(null)) });\n\
          ok(function(){ var o = {valueOf:null, toString:null}; return o + 1 });\n\
          ok(function(){ var o = {valueOf:1, toString:2}; return o + 1 });\n\
          ok(function(){ var o = {valueOf:function(){return {}}, toString:function(){return {}}}; return o + 1 });"),
        ("jsvalue:401 cannot convert undefined to object",
         "ok(function(){ return undefined.x });\n\
          ok(function(){ return Object.keys(undefined) });\n\
          ok(function(){ undefined.x = 1 });\n\
          ok(function(){ return 'a' in undefined });\n\
          ok(function(){ return undefined[0] });\n\
          ok(function(){ return void 0 .toString() });"),
        ("jsvalue:402 cannot convert null to object",
         "ok(function(){ return null.x });\n\
          ok(function(){ return Object.keys(null) });\n\
          ok(function(){ null.x = 1 });\n\
          ok(function(){ return 'a' in null });\n\
          ok(function(){ return null[0] });\n\
          ok(function(){ for (var k in null) ; return 'ok' });"),
        ("jsvalue:579 instanceof: invalid operand",
         "ok(function(){ return 1 instanceof 2 });\n\
          ok(function(){ return {} instanceof 'x' });\n\
          ok(function(){ return {} instanceof null });\n\
          ok(function(){ return {} instanceof undefined });\n\
          ok(function(){ return {} instanceof {} });\n\
          ok(function(){ return {} instanceof [] });"),
        ("jsvalue:586 instanceof: 'prototype' property is not an object",
         "ok(function(){ function F(){}; F.prototype = 1; return {} instanceof F });\n\
          ok(function(){ function F(){}; F.prototype = null; return {} instanceof F });\n\
          ok(function(){ function F(){}; F.prototype = 'x'; return {} instanceof F });\n\
          ok(function(){ function F(){}; F.prototype = undefined; return {} instanceof F });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsrun.c — the interpreter core
// ===========================================================================

#[test]
fn err_jsrun_property_access() {
    let cases: Cases = &[
        ("jsrun:773 setting property that only has a getter",
         "ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'g',{get:function(){return 1}}); o.g = 2; return o.g });\n\
          ok(function(){ var o={}; Object.defineProperty(o,'g',{get:function(){return 1}}); o.g = 2; return o.g });\n\
          ok(function(){ 'use strict'; var o={get g(){return 1}}; o.g=2; return o.g });"),
        ("jsrun:783 cannot create property on transient object",
         "ok(function(){ 'use strict'; 'abc'.newprop = 1 });\n\
          ok(function(){ 'use strict'; (5).newprop = 1 });\n\
          ok(function(){ 'use strict'; true.newprop = 1 });\n\
          ok(function(){ 'abc'.newprop = 1; return 'abc'.newprop });\n\
          ok(function(){ (5).newprop = 1; return (5).newprop });"),
        ("jsrun:800/854 read-only property",
         "ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'r',{value:1,writable:false}); o.r=2; return o.r });\n\
          ok(function(){ var o={}; Object.defineProperty(o,'r',{value:1,writable:false}); o.r=2; return o.r });\n\
          ok(function(){ 'use strict'; var o=Object.freeze({a:1}); o.a=2; return o.a });\n\
          ok(function(){ 'use strict'; 'abc'.length = 5 });\n\
          ok(function(){ 'use strict'; var a=[1]; Object.freeze(a); a[0]=2; return a[0] });"),
        ("jsrun:860/866/921/1145 non-configurable property",
         "ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'n',{value:1,configurable:false}); delete o.n; return o.n });\n\
          ok(function(){ var o={}; Object.defineProperty(o,'n',{value:1,configurable:false}); return delete o.n });\n\
          ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'n',{value:1,configurable:false}); Object.defineProperty(o,'n',{value:2}); return o.n });\n\
          ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'n',{get:function(){return 1},configurable:false}); Object.defineProperty(o,'n',{get:function(){return 2}}); return o.n });\n\
          ok(function(){ 'use strict'; delete Object.prototype });\n\
          ok(function(){ return delete Object.prototype });"),
        ("jsrun:875 read-only or non-configurable",
         "ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'x',{value:1}); Object.defineProperty(o,'x',{get:function(){return 2}}); return o.x });\n\
          ok(function(){ 'use strict'; var o={}; Object.defineProperty(o,'x',{get:function(){return 1}}); Object.defineProperty(o,'x',{value:2}); return o.x });"),
        ("jsrun:1127/1133/1145 variable assignment",
         "ok(function(){ 'use strict'; undeclaredvar = 1 });\n\
          ok(function(){ undeclaredvar2 = 1; return undeclaredvar2 });\n\
          ok(function(){ 'use strict'; NaN = 1 });\n\
          ok(function(){ 'use strict'; Infinity = 1 });\n\
          ok(function(){ 'use strict'; undefined = 1 });\n\
          ok(function(){ NaN = 1; return NaN });"),
        ("jsrun:1673/1698 not defined",
         "ok(function(){ return totallyundefinedname });\n\
          ok(function(){ return totallyundefinedname.x });\n\
          ok(function(){ return typeof totallyundefinedname });\n\
          ok(function(){ ++totallyundefinedname2 });\n\
          ok(function(){ totallyundefinedname3++ });\n\
          ok(function(){ return -totallyundefinedname4 });"),
        ("jsrun:1721 operand to 'in' is not an object",
         "ok(function(){ return 'a' in 1 });\n\
          ok(function(){ return 'a' in 'str' });\n\
          ok(function(){ return 'a' in true });\n\
          ok(function(){ return 0 in 1 });"),
    ];
    run_cases(cases);
}

#[test]
fn err_jsrun_calls_and_arrays() {
    let cases: Cases = &[
        ("jsrun:1290 call stack overflow",
         "ok(function(){ function f(){ return f() } return f() });\n\
          ok(function(){ function f(n){ return f(n+1) } return f(0) });\n\
          ok(function(){ function a(){ return b() } function b(){ return a() } return a() });"),
        ("jsrun:1307/1341 not callable",
         "ok(function(){ return (1)() });\n\
          ok(function(){ return 'x'() });\n\
          ok(function(){ return undefined() });\n\
          ok(function(){ return null() });\n\
          ok(function(){ return ({})() });\n\
          ok(function(){ return ([])() });\n\
          ok(function(){ var o={}; return o.missing() });\n\
          ok(function(){ return new 1 });\n\
          ok(function(){ return new 'x' });\n\
          ok(function(){ return new ({}) });\n\
          ok(function(){ return new (undefined) });\n\
          ok(function(){ return new (/re/) });"),
        ("jsrun:373 not a regexp",
         "ok(function(){ return RegExp.prototype.exec.call({}, 'a') });\n\
          ok(function(){ return RegExp.prototype.test.call(1, 'a') });\n\
          ok(function(){ return RegExp.prototype.toString.call('x') });\n\
          ok(function(){ return RegExp.prototype.exec.call([], 'a') });"),
        ("jsrun:393 not a function",
         "ok(function(){ return Function.prototype.apply.call(1, null, []) });\n\
          ok(function(){ return Function.prototype.call.call({}, null) });\n\
          ok(function(){ return Function.prototype.bind.call('x') });\n\
          ok(function(){ return Function.prototype.toString.call([]) });"),
        ("jsrun:676/707/709 array limits",
         "ok(function(){ return new Array(-1) });\n\
          ok(function(){ return new Array(1.5) });\n\
          ok(function(){ return new Array(4294967296) });\n\
          ok(function(){ return new Array(4294967295) });\n\
          ok(function(){ var a=[]; a.length = -1; return a.length });\n\
          ok(function(){ var a=[]; a.length = 1.5; return a.length });\n\
          ok(function(){ var a=[]; a.length = 4294967296; return a.length });\n\
          ok(function(){ var a=[]; a.length = 'abc'; return a.length });\n\
          ok(function(){ var a=[]; a.length = 67108864; return a.length });\n\
          ok(function(){ var a=[]; a.length = 67108865; return a.length });\n\
          ok(function(){ var a=[]; a[67108864] = 1; return a.length });"),
        ("jsrun:1461 endtry underflow / deep try nesting (JS_TRYLIMIT 64)",
         "ok(function(){ var s=''; function deep(n){ if (n<=0) return 'bottom'; try { return deep(n-1) } catch (e) { return 'c' } } return deep(100) });\n\
          ok(function(){ function deep(n){ if (n<=0) throw 'boom'; try { deep(n-1) } finally { } } try { deep(80) } catch (e) { return 'caught:'+e } return 'none' });"),
        ("jsrun:408/416/424 stack primitives via arguments",
         "ok(function(){ return Function.prototype.apply.call(function(){return arguments.length}, null, {length:100000}) });\n\
          ok(function(){ var a=[]; a.length=10000; return Function.prototype.apply.call(function(){return arguments.length}, null, a) });\n\
          ok(function(){ return Function.prototype.apply.call(function(){}, null, 1) });\n\
          ok(function(){ return Function.prototype.apply.call(function(){}, null, 'x') });\n\
          ok(function(){ return Math.max.apply(null, {length:5}) });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsstring.c / jsnumber.c / jsboolean.c — prototype method receivers
// ===========================================================================

#[test]
fn err_prototype_receivers() {
    let cases: Cases = &[
        ("jsstring:16 string function called on null or undefined",
         "ok(function(){ return String.prototype.charAt.call(null, 0) });\n\
          ok(function(){ return String.prototype.charAt.call(undefined, 0) });\n\
          ok(function(){ return String.prototype.indexOf.call(null, 'a') });\n\
          ok(function(){ return String.prototype.slice.call(undefined) });\n\
          ok(function(){ return String.prototype.toUpperCase.call(null) });\n\
          ok(function(){ return String.prototype.replace.call(null,'a','b') });\n\
          ok(function(){ return String.prototype.split.call(undefined,'') });\n\
          ok(function(){ return String.prototype.trim.call(null) });\n\
          ok(function(){ return String.prototype.concat.call(null,'a') });\n\
          ok(function(){ return String.prototype.match.call(null,/a/) });\n\
          ok(function(){ return String.prototype.search.call(undefined,/a/) });\n\
          ok(function(){ return String.prototype.localeCompare.call(null,'a') });\n\
          ok(function(){ return String.prototype.charCodeAt.call(null,0) });\n\
          ok(function(){ return String.prototype.lastIndexOf.call(undefined,'a') });\n\
          ok(function(){ return String.prototype.substring.call(null,0) });\n\
          ok(function(){ return String.prototype.substr.call(undefined,0) });"),
        ("jsstring:108/115 not a string",
         "ok(function(){ return String.prototype.toString.call(1) });\n\
          ok(function(){ return String.prototype.valueOf.call(1) });\n\
          ok(function(){ return String.prototype.toString.call({}) });\n\
          ok(function(){ return String.prototype.valueOf.call([]) });\n\
          ok(function(){ return String.prototype.toString.call(new Number(1)) });\n\
          ok(function(){ return String.prototype.toString.call(null) });"),
        ("jsnumber:22/33 not a number",
         "ok(function(){ return Number.prototype.toString.call('x') });\n\
          ok(function(){ return Number.prototype.valueOf.call('x') });\n\
          ok(function(){ return Number.prototype.toString.call({}) });\n\
          ok(function(){ return Number.prototype.toFixed.call('x', 2) });\n\
          ok(function(){ return Number.prototype.toExponential.call({}, 2) });\n\
          ok(function(){ return Number.prototype.toPrecision.call([], 2) });\n\
          ok(function(){ return Number.prototype.toLocaleString.call('x') });\n\
          ok(function(){ return Number.prototype.valueOf.call(new String('1')) });"),
        ("jsnumber:40 invalid radix",
         "ok(function(){ return (5).toString(0) });\n\
          ok(function(){ return (5).toString(1) });\n\
          ok(function(){ return (5).toString(37) });\n\
          ok(function(){ return (5).toString(-1) });\n\
          ok(function(){ return (5).toString(1.5) });\n\
          ok(function(){ return (5).toString(NaN) });\n\
          ok(function(){ return (5).toString(Infinity) });\n\
          ok(function(){ return (5).toString(2) });\n\
          ok(function(){ return (5).toString(36) });"),
        ("jsnumber:135/151/167 precision out of range",
         "ok(function(){ return (1).toFixed(-1) });\n\
          ok(function(){ return (1).toFixed(21) });\n\
          ok(function(){ return (1).toFixed(20) });\n\
          ok(function(){ return (1).toFixed(0) });\n\
          ok(function(){ return (1).toExponential(-1) });\n\
          ok(function(){ return (1).toExponential(21) });\n\
          ok(function(){ return (1).toExponential(20) });\n\
          ok(function(){ return (1).toPrecision(0) });\n\
          ok(function(){ return (1).toPrecision(22) });\n\
          ok(function(){ return (1).toPrecision(1) });\n\
          ok(function(){ return (1).toPrecision(21) });"),
        ("jsboolean:16/23 not a boolean",
         "ok(function(){ return Boolean.prototype.toString.call(1) });\n\
          ok(function(){ return Boolean.prototype.valueOf.call(1) });\n\
          ok(function(){ return Boolean.prototype.toString.call({}) });\n\
          ok(function(){ return Boolean.prototype.valueOf.call(null) });\n\
          ok(function(){ return Boolean.prototype.toString.call(new Boolean(true)) });"),
        ("jsdate:366/374 not a date",
         "ok(function(){ return Date.prototype.getTime.call({}) });\n\
          ok(function(){ return Date.prototype.valueOf.call(1) });\n\
          ok(function(){ return Date.prototype.getFullYear.call('x') });\n\
          ok(function(){ return Date.prototype.setTime.call({}, 0) });\n\
          ok(function(){ return Date.prototype.toISOString.call({}) });\n\
          ok(function(){ return Date.prototype.toString.call({}) });"),
        ("jsdate:485 invalid date",
         "ok(function(){ return new Date(NaN).toISOString() });\n\
          ok(function(){ return new Date(Infinity).toISOString() });\n\
          ok(function(){ return new Date(8.64e15+1).toISOString() });\n\
          ok(function(){ return new Date('nope').toISOString() });"),
        ("jsdate:793 this.toISOString is not a function",
         "ok(function(){ return Date.prototype.toJSON.call({}) });\n\
          ok(function(){ return Date.prototype.toJSON.call({toISOString:1}) });\n\
          ok(function(){ return Date.prototype.toJSON.call({toISOString:function(){return 'x'}}) });"),
        ("jsfunction:53/100/123/186 not a function",
         "ok(function(){ return Function.prototype.toString.call(1) });\n\
          ok(function(){ return Function.prototype.apply.call(1) });\n\
          ok(function(){ return Function.prototype.call.call(1) });\n\
          ok(function(){ return Function.prototype.bind.call(1) });"),
        ("jserror:36 not an object",
         "ok(function(){ return Error.prototype.toString.call(1) });\n\
          ok(function(){ return Error.prototype.toString.call(null) });\n\
          ok(function(){ return Error.prototype.toString.call('x') });\n\
          ok(function(){ return Error.prototype.toString.call({}) });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsarray.c
// ===========================================================================

#[test]
fn err_jsarray() {
    let cases: Cases = &[
        ("jsarray:440 comparison function must be a function or undefined",
         "ok(function(){ return [2,1].sort(1) });\n\
          ok(function(){ return [2,1].sort('x') });\n\
          ok(function(){ return [2,1].sort({}) });\n\
          ok(function(){ return [2,1].sort([]) });\n\
          ok(function(){ return [2,1].sort(null) });\n\
          ok(function(){ return String([2,1].sort(undefined)) });"),
        ("jsarray:443 array is too large to sort",
         "ok(function(){ var a=[]; a.length=67108864; return a.sort() });\n\
          ok(function(){ var a=[]; a.length=1048576; a.sort(); return 'ok' });"),
        ("jsarray:537 'this' is not an object",
         "ok(function(){ return Array.prototype.toString.call(null) });\n\
          ok(function(){ return Array.prototype.toString.call(undefined) });\n\
          ok(function(){ return Array.prototype.toString.call(1) });\n\
          ok(function(){ return Array.prototype.toString.call('x') });"),
        ("jsarray:604-792 callback is not a function",
         "ok(function(){ return [1].every(1) });\n\
          ok(function(){ return [1].every() });\n\
          ok(function(){ return [1].some(1) });\n\
          ok(function(){ return [1].some() });\n\
          ok(function(){ return [1].forEach(1) });\n\
          ok(function(){ return [1].forEach() });\n\
          ok(function(){ return [1].map(1) });\n\
          ok(function(){ return [1].map() });\n\
          ok(function(){ return [1].filter(1) });\n\
          ok(function(){ return [1].filter() });\n\
          ok(function(){ return [1].reduce(1) });\n\
          ok(function(){ return [1].reduce() });\n\
          ok(function(){ return [1].reduceRight(1) });\n\
          ok(function(){ return [1].reduceRight() });\n\
          ok(function(){ return [1].every(null) });\n\
          ok(function(){ return [1].map('x') });"),
        ("jsarray:757/767/798/808 no initial value",
         "ok(function(){ return [].reduce(function(a,b){return a+b}) });\n\
          ok(function(){ return [].reduceRight(function(a,b){return a+b}) });\n\
          ok(function(){ var a=[]; a.length=3; return a.reduce(function(x,y){return x+y}) });\n\
          ok(function(){ var a=[]; a.length=3; return a.reduceRight(function(x,y){return x+y}) });\n\
          ok(function(){ return [].reduce(function(a,b){return a+b}, 0) });\n\
          ok(function(){ return [].reduceRight(function(a,b){return a+b}, 0) });"),
        ("jsarray:149 join invalid string length (documented; not constructible)",
         "ok(function(){ var a=[]; a.length=1000; return a.join('xx').length });\n\
          ok(function(){ var a=[]; for (var i=0;i<1000;++i) a.push('x'); return a.join('yyy').length });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsobject.c / jsproperty.c
// ===========================================================================

#[test]
fn err_jsobject() {
    let cases: Cases = &[
        ("jsobject not an object",
         "ok(function(){ return Object.getPrototypeOf(1) });\n\
          ok(function(){ return Object.getOwnPropertyDescriptor(1,'a') });\n\
          ok(function(){ return Object.getOwnPropertyNames(1) });\n\
          ok(function(){ return Object.defineProperty(1,'a',{}) });\n\
          ok(function(){ return Object.defineProperties(1,{}) });\n\
          ok(function(){ return Object.seal(1) });\n\
          ok(function(){ return Object.freeze(1) });\n\
          ok(function(){ return Object.preventExtensions(1) });\n\
          ok(function(){ return Object.isSealed(1) });\n\
          ok(function(){ return Object.isFrozen(1) });\n\
          ok(function(){ return Object.isExtensible(1) });\n\
          ok(function(){ return Object.keys(1) });\n\
          ok(function(){ return Object.keys('x') });\n\
          ok(function(){ return Object.keys(null) });\n\
          ok(function(){ return Object.keys(undefined) });\n\
          ok(function(){ return Object.defineProperty({}, 'a', 1) });\n\
          ok(function(){ return Object.defineProperties({}, 1) });"),
        ("jsobject:258/265 value/writable and get/set attributes are exclusive",
         "ok(function(){ return Object.defineProperty({},'a',{value:1,get:function(){}}) });\n\
          ok(function(){ return Object.defineProperty({},'a',{value:1,set:function(){}}) });\n\
          ok(function(){ return Object.defineProperty({},'a',{writable:true,get:function(){}}) });\n\
          ok(function(){ return Object.defineProperty({},'a',{writable:true,set:function(){}}) });\n\
          ok(function(){ return Object.defineProperty({},'a',{value:1,writable:true}) });\n\
          ok(function(){ return Object.defineProperty({},'a',{get:function(){return 1}}) });"),
        ("jsobject:342 not an object or null",
         "ok(function(){ return Object.create(1) });\n\
          ok(function(){ return Object.create('x') });\n\
          ok(function(){ return Object.create(undefined) });\n\
          ok(function(){ return Object.create(true) });\n\
          ok(function(){ return typeof Object.create(null) });\n\
          ok(function(){ return typeof Object.create({}) });"),
        ("jsproperty:228 object is non-extensible",
         "ok(function(){ 'use strict'; var o=Object.preventExtensions({}); o.n=1; return o.n });\n\
          ok(function(){ var o=Object.preventExtensions({}); o.n=1; return o.n });\n\
          ok(function(){ 'use strict'; var o=Object.freeze({}); o.n=1; return o.n });\n\
          ok(function(){ 'use strict'; var o=Object.seal({}); o.n=1; return o.n });\n\
          ok(function(){ var a=Object.preventExtensions([]); a[0]=1; return a.length });\n\
          ok(function(){ 'use strict'; var o=Object.preventExtensions({}); Object.defineProperty(o,'n',{value:1}); return o.n });"),
        ("jsproperty:303 not an iterator",
         "ok(function(){ var o={}; for (var k in o) ; return 'ok' });"),
        ("Object.prototype methods on odd receivers",
         "ok(function(){ return Object.prototype.hasOwnProperty.call(null,'a') });\n\
          ok(function(){ return Object.prototype.hasOwnProperty.call(1,'a') });\n\
          ok(function(){ return Object.prototype.isPrototypeOf.call(null,{}) });\n\
          ok(function(){ return Object.prototype.propertyIsEnumerable.call(null,'a') });\n\
          ok(function(){ return Object.prototype.toString.call(null) });\n\
          ok(function(){ return Object.prototype.valueOf.call(null) });\n\
          ok(function(){ return Object.prototype.toLocaleString.call(null) });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// json.c
// ===========================================================================

#[test]
fn err_json() {
    let cases: Cases = &[
        ("json:41/67/107 unexpected token",
         "ok(function(){ return JSON.parse('') });\n\
          ok(function(){ return JSON.parse(' ') });\n\
          ok(function(){ return JSON.parse('[') });\n\
          ok(function(){ return JSON.parse(']') });\n\
          ok(function(){ return JSON.parse('{') });\n\
          ok(function(){ return JSON.parse('}') });\n\
          ok(function(){ return JSON.parse('[1') });\n\
          ok(function(){ return JSON.parse('[1,') });\n\
          ok(function(){ return JSON.parse('[1,]') });\n\
          ok(function(){ return JSON.parse('{\"a\"}') });\n\
          ok(function(){ return JSON.parse('{\"a\":}') });\n\
          ok(function(){ return JSON.parse('{a:1}') });\n\
          ok(function(){ return JSON.parse('{1:2}') });\n\
          ok(function(){ return JSON.parse('{\"a\":1,}') });\n\
          ok(function(){ return JSON.parse(\"'a'\") });\n\
          ok(function(){ return JSON.parse('undefined') });\n\
          ok(function(){ return JSON.parse('NaN') });\n\
          ok(function(){ return JSON.parse('Infinity') });\n\
          ok(function(){ return JSON.parse('nan') });\n\
          ok(function(){ return JSON.parse('tru') });\n\
          ok(function(){ return JSON.parse('01') });\n\
          ok(function(){ return JSON.parse('+1') });\n\
          ok(function(){ return JSON.parse('.1') });\n\
          ok(function(){ return JSON.parse('1.') });\n\
          ok(function(){ return JSON.parse('1e') });\n\
          ok(function(){ return JSON.parse('0x10') });\n\
          ok(function(){ return JSON.parse('1 2') });\n\
          ok(function(){ return JSON.parse('\"unterminated') });\n\
          ok(function(){ return JSON.parse('\"\\\\x41\"') });\n\
          ok(function(){ return JSON.parse('\"\\\\u00\"') });\n\
          ok(function(){ return JSON.parse('\"\\\\q\"') });\n\
          ok(function(){ return JSON.parse('[1 2]') });\n\
          ok(function(){ return JSON.parse() });\n\
          ok(function(){ return JSON.parse(undefined) });\n\
          ok(function(){ return JSON.parse(null) });\n\
          ok(function(){ return JSON.parse(1) });"),
        ("json:261/297 cyclic object value",
         "ok(function(){ var a=[]; a[0]=a; return JSON.stringify(a) });\n\
          ok(function(){ var o={}; o.s=o; return JSON.stringify(o) });\n\
          ok(function(){ var a=[], b={x:a}; a.push(b); return JSON.stringify(a) });\n\
          ok(function(){ var a=[], b={x:a}; a.push(b); return JSON.stringify(b) });\n\
          ok(function(){ var a=[1]; return JSON.stringify([a,a]) });\n\
          ok(function(){ var o={a:1}; return JSON.stringify({x:o,y:o}) });"),
        ("JSON.stringify with throwing toJSON / replacer",
         "ok(function(){ return JSON.stringify({toJSON:function(){ throw 'tj' }}) });\n\
          ok(function(){ return JSON.stringify({a:1}, function(){ throw 'rep' }) });\n\
          ok(function(){ return JSON.stringify({a:1}, 1) });\n\
          ok(function(){ return JSON.parse('1', function(){ throw 'rev' }) });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsregexp.c
// ===========================================================================

#[test]
fn err_jsregexp() {
    let cases: Cases = &[
        ("jsregexp:38 regular expression syntax errors",
         "ok(function(){ return new RegExp('(') });\n\
          ok(function(){ return new RegExp(')') });\n\
          ok(function(){ return new RegExp('a**') });\n\
          ok(function(){ return new RegExp('[z-a]') });\n\
          ok(function(){ return new RegExp('[abc') });\n\
          ok(function(){ return new RegExp('\\\\q') });\n\
          ok(function(){ return new RegExp('a{100000}') });\n\
          ok(function(){ return new RegExp('(a*)*') });\n\
          ok(function(){ return new RegExp('\\\\1') });\n\
          ok(function(){ return new RegExp('(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)(q)') });\n\
          ok(function(){ return new RegExp('*') });\n\
          ok(function(){ return new RegExp('{1}') });\n\
          ok(function(){ return new RegExp('\\\\xA') });\n\
          ok(function(){ return new RegExp('\\\\u12') });\n\
          ok(function(){ return new RegExp('\\\\c') });\n\
          ok(function(){ return new RegExp('\\\\c1') });\n\
          ok(function(){ return new RegExp('(?:') });\n\
          ok(function(){ return new RegExp('(?=') });\n\
          ok(function(){ return new RegExp('(?!') });"),
        ("jsregexp:149 cannot supply flags when creating one RegExp from another",
         "ok(function(){ return new RegExp(/a/, 'g') });\n\
          ok(function(){ return new RegExp(/a/g, 'i') });\n\
          ok(function(){ return new RegExp(/a/, '') });\n\
          ok(function(){ return String(new RegExp(/a/g)) });\n\
          ok(function(){ return String(new RegExp(/a/g, undefined)) });"),
        ("jsregexp:172/175/176/177 invalid or duplicated flags",
         "ok(function(){ return new RegExp('a','x') });\n\
          ok(function(){ return new RegExp('a','G') });\n\
          ok(function(){ return new RegExp('a','gg') });\n\
          ok(function(){ return new RegExp('a','ii') });\n\
          ok(function(){ return new RegExp('a','mm') });\n\
          ok(function(){ return new RegExp('a','gim') });\n\
          ok(function(){ return new RegExp('a','gimg') });\n\
          ok(function(){ return new RegExp('a','1') });\n\
          ok(function(){ return new RegExp('a',' ') });\n\
          ok(function(){ return new RegExp('a','\\u00e9') });"),
        ("regexp exec failure paths",
         "ok(function(){ var r=/(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)/; return String(r.exec('abcdefghijklmno')) });\n\
          ok(function(){ var r=/a/g; r.lastIndex = 1e9; return String(r.exec('aaa')) });\n\
          ok(function(){ var r=/a/g; r.lastIndex = 'x'; return String(r.exec('aaa')) });\n\
          ok(function(){ var r=/a/g; r.lastIndex = -5; return String(r.exec('aaa')) });\n\
          ok(function(){ var r=/a/g; r.lastIndex = NaN; return String(r.exec('aaa')) });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsbuiltin.c — URI handling
// ===========================================================================

#[test]
fn err_jsbuiltin_uri() {
    let cases: Cases = &[
        ("jsbuiltin:145 truncated escape sequence",
         "ok(function(){ return decodeURI('%') });\n\
          ok(function(){ return decodeURI('%A') });\n\
          ok(function(){ return decodeURIComponent('%') });\n\
          ok(function(){ return decodeURIComponent('%4') });\n\
          ok(function(){ return decodeURI('abc%') });\n\
          ok(function(){ return decodeURI('%E4%B8') });"),
        ("jsbuiltin:149 invalid escape sequence",
         "ok(function(){ return decodeURI('%zz') });\n\
          ok(function(){ return decodeURI('%GG') });\n\
          ok(function(){ return decodeURIComponent('%zz') });\n\
          ok(function(){ return decodeURI('%C0%80') });\n\
          ok(function(){ return decodeURI('%80') });\n\
          ok(function(){ return decodeURI('%FF') });\n\
          ok(function(){ return decodeURI('%ED%A0%80') });\n\
          ok(function(){ return decodeURI('%F5%80%80%80') });"),
        ("encodeURI on lone surrogates",
         "ok(function(){ return encodeURI('\\ud800') });\n\
          ok(function(){ return encodeURI('\\udc00') });\n\
          ok(function(){ return encodeURIComponent('\\ud800') });\n\
          ok(function(){ return encodeURI('\\ud83d\\ude00') });\n\
          ok(function(){ return encodeURI('\\u00e9\\u4e2d') });\n\
          ok(function(){ return escape('\\ud800') });\n\
          ok(function(){ return unescape('%u00') });\n\
          ok(function(){ return unescape('%u0041') });\n\
          ok(function(){ return unescape('%') });\n\
          ok(function(){ return unescape('%4') });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jslex.c — every lexer diagnostic
// ===========================================================================

#[test]
fn err_jslex() {
    let cases: Cases = &[
        ("jslex:192 unexpected escape sequence", "var \\u0041 = 1;"),
        ("jslex:192 unexpected escape sequence 2", "o(\\u0041);"),
        ("jslex:255 malformed hexadecimal number", "o(0x);"),
        ("jslex:255 malformed hexadecimal number 2", "o(0X);"),
        ("jslex:269 malformed number", "o(0xg);"),
        ("jslex:312 number with leading zero", "'use strict'; o(01);"),
        ("jslex:333 number with letter suffix", "o(1a);"),
        ("jslex:333 number with letter suffix 2", "o(1abc);"),
        ("jslex:351 number with leading zero 2", "'use strict'; o(00);"),
        ("jslex:377 missing exponent", "o(1e);"),
        ("jslex:377 missing exponent 2", "o(1e+);"),
        ("jslex:377 missing exponent 3", "o(1E-);"),
        ("jslex:381 number with letter suffix 3", "o(1e5x);"),
        ("jslex:399 unterminated escape sequence", "o('a\\"),
        ("jslex:440 string not terminated", "o('abc);"),
        ("jslex:440 string not terminated 2", "o(\"abc);"),
        ("jslex:440 string not terminated 3", "o('abc\ndef');"),
        ("jslex:443 malformed escape sequence", "o('\\x');"),
        ("jslex:443 malformed escape sequence 2", "o('\\xZZ');"),
        ("jslex:443 malformed escape sequence 3", "o('\\u');"),
        ("jslex:443 malformed escape sequence 4", "o('\\uZZZZ');"),
        ("jslex:443 malformed escape sequence 5", "o('\\u12');"),
        ("jslex:490 regular expression not terminated", "var r = /abc;"),
        ("jslex:497 regular expression not terminated 2", "var r = /abc\n/;"),
        ("jslex:497 regular expression not terminated 3", "var r = /[abc/;"),
        ("jslex:521 illegal flag in regular expression", "var r = /a/x;"),
        ("jslex:521 illegal flag 2", "var r = /a/q;"),
        ("jslex:525 duplicated flag in regular expression", "var r = /a/gg;"),
        ("jslex:525 duplicated flag 2", "var r = /a/gimg;"),
        ("jslex:574 multi-line comment not terminated", "/* unterminated"),
        ("jslex:574 multi-line comment not terminated 2", "o(1); /* abc"),
        ("jslex:728 unexpected character", "o(1) # 2;"),
        ("jslex:728 unexpected character 2", "o(1) @ 2;"),
        ("jslex:728 unexpected character 3", "var a = 1 ` 2;"),
        ("jslex:729 unexpected character unicode", "var a = \u{4e2d}\u{6587}\u{ff01};"),
        ("jslex:729 unexpected character unicode 2", "o(1 \u{2764} 2);"),
        // JSON lexer (jslex.c:760-822, reached through JSON.parse)
        ("jslex:760 unexpected non-digit", "ok(function(){ return JSON.parse('-') });"),
        ("jslex:767 missing digits after decimal point", "ok(function(){ return JSON.parse('1.') });"),
        ("jslex:777 missing digits after exponent indicator", "ok(function(){ return JSON.parse('1e') });\nok(function(){ return JSON.parse('1e+') });"),
        ("jslex:791 invalid escape sequence (json)", "ok(function(){ return JSON.parse('\"\\\\q\"') });\nok(function(){ return JSON.parse('\"\\\\x41\"') });"),
        ("jslex:820 unterminated string (json)", "ok(function(){ return JSON.parse('\"abc') });"),
        ("jslex:822 invalid control character in string (json)", "ok(function(){ return JSON.parse('\"a\\u0001b\"') });\nok(function(){ return JSON.parse('\"a\\tb\"') });"),
        ("jslex:878 unexpected character (json)", "ok(function(){ return JSON.parse('@') });\nok(function(){ return JSON.parse('#') });"),
        ("jslex:879 unexpected character unicode (json)", "ok(function(){ return JSON.parse('\\u4e2d') });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jsparse.c — every parser diagnostic
// ===========================================================================

#[test]
fn err_jsparse() {
    let cases: Cases = &[
        ("jsparse:143 jsP_expect", "o(1;"),
        ("jsparse:143 jsP_expect 2", "if (1 o(2);"),
        ("jsparse:143 jsP_expect 3", "function f( { }"),
        ("jsparse:143 jsP_expect 4", "var a = {1;"),
        ("jsparse:143 jsP_expect 5", "var a = [1;"),
        ("jsparse:153 expected ';'", "var a = 1 var b = 2 o(a);"),
        ("jsparse:153 expected ';' 2", "do o(1); while (0) o(2)"),
        ("jsparse:166 expected identifier", "var = 1;"),
        ("jsparse:166 expected identifier 2", "var 1 = 2;"),
        ("jsparse:166 expected identifier 3", "function 1(){}"),
        ("jsparse:166 expected identifier 4", "var if = 1;"),
        ("jsparse:183 expected identifier or keyword", "var a = {}; o(a.1);"),
        ("jsparse:183 expected identifier or keyword 2", "var a = {}; o(a.);"),
        ("jsparse:363 unexpected token in expression", "o(*);"),
        ("jsparse:363 unexpected token in expression 2", "o(});"),
        ("jsparse:363 unexpected token in expression 3", "var a = ;"),
        ("jsparse:363 unexpected token in expression 4", "o(1 + );"),
        ("jsparse:363 unexpected token in expression 5", "o(else);"),
        ("jsparse:700 unexpected token in switch", "switch (1) { o(2); }"),
        ("jsparse:700 unexpected token in switch 2", "switch (1) { 2: o(3); }"),
        ("jsparse:751 unexpected token in for-var-statement", "for (var a = 1 o(2)) ;"),
        ("jsparse:770 unexpected token in for-statement", "for (1 o(2)) ;"),
        ("jsparse:770 unexpected token in for-statement 2", "for (a b c) ;"),
        ("jsparse:888 unexpected token in try", "try { } o(1);"),
        ("jsparse:888 unexpected token in try 2", "try { } else { }"),
        ("jsparse:24 too much recursion (JS_ASTLIMIT 400)",
         // 500 nested parentheses exceeds JS_ASTLIMIT
         "o((((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          (((((((((((((((((((((((((((((((((((((((((((((((((\
          1\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          )))))))))))))))))))))))))))))))))))))))))))))))))\
          ))))))))))))))))))))))))))))))))))))))))))))))))));"),
        ("unbalanced braces", "if (1) { o(2);"),
        ("unbalanced parens", "o((1);"),
        ("stray closing brace", "o(1); }"),
        ("empty source", ""),
        ("only whitespace", "   \n\t  "),
        ("only comment", "// nothing"),
    ];
    run_cases(cases);
}

// ===========================================================================
// jscompile.c — every compiler diagnostic
// ===========================================================================

#[test]
fn err_jscompile() {
    let cases: Cases = &[
        ("jscompile:43 future reserved word",
         "var class = 1;"),
        ("jscompile:43 future reserved word 2", "var enum = 1;"),
        ("jscompile:43 future reserved word 3", "var extends = 1;"),
        ("jscompile:43 future reserved word 4", "var super = 1;"),
        ("jscompile:43 future reserved word 5", "var const = 1;"),
        ("jscompile:43 future reserved word 6", "var export = 1;"),
        ("jscompile:43 future reserved word 7", "var import = 1;"),
        ("jscompile:46 strict mode future reserved word",
         "'use strict'; var implements = 1;"),
        ("jscompile:46 strict mode future reserved word 2", "'use strict'; var interface = 1;"),
        ("jscompile:46 strict mode future reserved word 3", "'use strict'; var let = 1;"),
        ("jscompile:46 strict mode future reserved word 4", "'use strict'; var package = 1;"),
        ("jscompile:46 strict mode future reserved word 5", "'use strict'; var private = 1;"),
        ("jscompile:46 strict mode future reserved word 6", "'use strict'; var protected = 1;"),
        ("jscompile:46 strict mode future reserved word 7", "'use strict'; var public = 1;"),
        ("jscompile:46 strict mode future reserved word 8", "'use strict'; var static = 1;"),
        ("jscompile:46 strict mode future reserved word 9", "'use strict'; var yield = 1;"),
        ("jscompile:46 non-strict allows them", "var implements = 1; o(implements);"),
        ("jscompile:114 redefining 'arguments' in strict mode",
         "'use strict'; function f(arguments){ return arguments } o(f(1));"),
        ("jscompile:114 redefining 'arguments' 2", "'use strict'; function f(){ var arguments = 1; return arguments } o(f());"),
        ("jscompile:116 redefining 'eval' in strict mode",
         "'use strict'; function f(eval){ return eval } o(f(1));"),
        ("jscompile:116 redefining 'eval' 2", "'use strict'; function f(){ var eval = 1; return eval } o(f());"),
        ("jscompile:119/209 invalid use of 'eval'",
         "'use strict'; var eval = 1;"),
        ("jscompile:119 invalid use of 'eval' 2", "'use strict'; function eval(){}"),
        ("jscompile:128 duplicate formal parameter",
         "'use strict'; function f(a,a){ return a } o(f(1,2));"),
        ("jscompile:128 duplicate formal parameter non-strict", "function f(a,a){ return a } o(f(1,2));"),
        ("jscompile:204 'arguments' is read-only in strict mode",
         "'use strict'; function f(){ arguments = 1 } f();"),
        ("jscompile:206 'eval' is read-only in strict mode",
         "'use strict'; eval = 1;"),
        ("jscompile:315 duplicate property in object literal",
         "'use strict'; var o1 = {a:1,a:2}; o(o1.a);"),
        ("jscompile:315 duplicate property non-strict", "var o1 = {a:1,a:2}; o(o1.a);"),
        ("jscompile:315 duplicate property getter/value", "'use strict'; var o1 = {a:1,get a(){return 2}};"),
        ("jscompile:336 invalid property name in object initializer",
         "var o1 = {get 1(){ return 2 }}; o(o1[1]);"),
        ("jscompile:400 invalid l-value in assignment",
         "1 = 2;"),
        ("jscompile:400 invalid l-value 2", "'a' = 2;"),
        ("jscompile:400 invalid l-value 3", "f() = 2;"),
        ("jscompile:400 invalid l-value 4", "(1+2) = 3;"),
        ("jscompile:400 invalid l-value 5", "this = 1;"),
        ("jscompile:400 invalid l-value 6", "null = 1;"),
        ("jscompile:410 more than one loop variable in for-in statement",
         "for (var a, b in {x:1}) ;"),
        ("jscompile:439 invalid l-value in for-in loop assignment",
         "for (1 in {x:1}) ;"),
        ("jscompile:439 invalid l-value in for-in 2", "for (f() in {x:1}) ;"),
        ("jscompile:464/487 invalid l-value in assignment (compound / update)",
         "1 += 2;"),
        ("jscompile:464 invalid l-value compound 2", "'a' *= 2;"),
        ("jscompile:487 invalid l-value update", "1++;"),
        ("jscompile:487 invalid l-value update 2", "++1;"),
        ("jscompile:487 invalid l-value update 3", "--'a';"),
        ("jscompile:508 delete on an unqualified name in strict mode",
         "'use strict'; var x = 1; delete x;"),
        ("jscompile:508 delete unqualified non-strict", "var x = 1; o(delete x);"),
        ("jscompile:524 invalid l-value in delete expression",
         "delete (1+2);"),
        ("jscompile:524 invalid l-value in delete 2", "o(delete 1);"),
        ("jscompile:961/963/993/995 redefining 'arguments'/'eval' in catch (strict)",
         "'use strict'; try { } catch (arguments) { }"),
        ("jscompile:963 catch eval strict", "'use strict'; try { } catch (eval) { }"),
        ("jscompile:1025 more than one default label in switch",
         "switch (1) { default: ; default: ; }"),
        ("jscompile:1217 break label not found",
         "lbl: { } break lbl;"),
        ("jscompile:1217 break label not found 2", "break nosuchlabel;"),
        ("jscompile:1221 unlabelled break must be inside loop or switch",
         "break;"),
        ("jscompile:1221 unlabelled break 2", "if (1) break;"),
        ("jscompile:1221 unlabelled break 3", "lbl: { break; }"),
        ("jscompile:1233 continue label not found",
         "continue nosuchlabel;"),
        ("jscompile:1233 continue label not found 2", "lbl: { continue lbl; }"),
        ("jscompile:1237 continue must be inside loop",
         "continue;"),
        ("jscompile:1237 continue must be inside loop 2", "switch (1) { default: continue }"),
        ("jscompile:1251 return not in function",
         "return;"),
        ("jscompile:1251 return not in function 2", "return 1;"),
        ("jscompile:1266 'with' statements are not allowed in strict mode",
         "'use strict'; with ({a:1}) { o(a) }"),
        ("jscompile:1266 with allowed non-strict", "with ({a:1}) { o(a) }"),
        ("jscompile:75/238/245 instruction / jump address overflow (documented; needs a >2^16 program)",
         "var s = ''; for (var i=0;i<200;++i) s += 'o(' + i + ');'; ok(function(){ return eval(s) });"),
        // strict-mode octal escapes and other strict-only diagnostics
        ("strict octal literal", "'use strict'; o(010);"),
        ("strict octal escape", "'use strict'; o('\\101');"),
        ("strict delete of a variable in eval", "ok(function(){ 'use strict'; return eval('var q=1; delete q') });"),
    ];
    run_cases(cases);
}

// ===========================================================================
// Generic FFI boundaries: out-of-range enum values crossing the boundary
// ===========================================================================

/// C enums accept any `int`, so values with no valid variant are real inputs.
/// These are already covered per-entry-point in `state_api.rs`; this test pins
/// the specific enum-typed parameters named in `mujs.h`.
#[test]
fn err_out_of_range_enum_values() {
    use std::os::raw::{c_char, c_int};

    let (capi, rapi) = both_apis();

    // --- js_newstate flags: only JS_STRICT (1) is defined ---
    for flags in [
        -1i32,
        2,
        3,
        4,
        8,
        0x10,
        0xFF,
        0xFFFF,
        0x7FFF_FFFF,
        i32::MIN,
        i32::MIN + 1,
    ] {
        assert_same_program(flags, &format!("newstate flags {flags}"), "o(1); o(typeof this);");
    }

    // --- js_newregexp flags: only JS_REGEXP_G|I|M (1|2|4) are defined ---
    for rf in [-1i32, 8, 9, 15, 16, 0xFF, 0x7FFF_FFFF, i32::MIN] {
        let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let run = |api: &Api| {
            let _ = report_sink::take();
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            let pro = cstr(PROLOGUE);
            let _ = (api.js_dostring)(j, pro.as_ptr() as *const c_char);
            let pat = cstr("a+");
            (api.js_newregexp)(j, pat.as_ptr() as *const c_char, rf);
            let g = cstr("re");
            (api.js_setglobal)(j, g.as_ptr() as *const c_char);
            let z = cstr(
                "o(re.source); o(re.global); o(re.ignoreCase); o(re.multiline); \
                 o(re.lastIndex); o(String(re)); o(String(re.exec('xaay'))); \
                 o('xaay'.replace(re,'#')); o(JSON.stringify('a,aa,b'.split(re)));",
            );
            let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
            let n = cstr("__out");
            (api.js_getglobal)(j, n.as_ptr() as *const c_char);
            let out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            (api.js_pop)(j, 1);
            let reports = report_sink::take();
            (api.js_freestate)(j);
            (rc, out, reports)
        };
        let a = run(capi);
        let b = run(rapi);
        assert_eq!(a, b, "js_newregexp with out-of-range flags {rf}");
    }

    // --- property attribute flags: only JS_READONLY|DONTENUM|DONTCONF (1|2|4) ---
    for atts in [-1i32, 8, 9, 15, 16, 0xFF, 0x7FFF_FFFF, i32::MIN] {
        let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let run = |api: &Api| {
            let _ = report_sink::take();
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            let pro = cstr(PROLOGUE);
            let _ = (api.js_dostring)(j, pro.as_ptr() as *const c_char);
            (api.js_pushnumber)(j, 7.0);
            let nm = cstr("gp");
            (api.js_defglobal)(j, nm.as_ptr() as *const c_char, atts);
            (api.js_newobject)(j);
            (api.js_pushnumber)(j, 8.0);
            let pn = cstr("op");
            (api.js_defproperty)(j, -2, pn.as_ptr() as *const c_char, atts);
            let og = cstr("obj");
            (api.js_setglobal)(j, og.as_ptr() as *const c_char);
            let z = cstr(
                "o(gp); ok(function(){ gp = 1; return gp }); ok(function(){ return delete gp }); \
                 o(obj.op); ok(function(){ obj.op = 2; return obj.op }); \
                 ok(function(){ return delete obj.op }); \
                 o(JSON.stringify(Object.keys(obj))); \
                 o(JSON.stringify(Object.getOwnPropertyDescriptor(obj,'op')));",
            );
            let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
            let n = cstr("__out");
            (api.js_getglobal)(j, n.as_ptr() as *const c_char);
            let out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            (api.js_pop)(j, 1);
            let reports = report_sink::take();
            (api.js_freestate)(j);
            (rc, out, reports)
        };
        let a = run(capi);
        let b = run(rapi);
        assert_eq!(a, b, "attribute flags {atts}");
    }

    // --- js_type()'s result enum is read-only, but js_pushiterator's `own`
    //     and js_gc's `report` are int-typed flags: cover invalid values ---
    for own in [-1i32, 2, 7, i32::MAX, i32::MIN] {
        let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let run = |api: &Api| {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            let z = cstr("var v = {a:1,b:2};");
            let _ = (api.js_dostring)(j, z.as_ptr() as *const c_char);
            let g = cstr("v");
            (api.js_getglobal)(j, g.as_ptr() as *const c_char);
            (api.js_pushiterator)(j, -1, own);
            let mut names: Vec<String> = Vec::new();
            loop {
                match unsafe { read_cstr((api.js_nextiterator)(j, -1)) } {
                    None => break,
                    Some(b) => {
                        names.push(String::from_utf8_lossy(&b).into_owned());
                        if names.len() > 64 {
                            break;
                        }
                    }
                }
            }
            let top = (api.js_gettop)(j);
            (api.js_freestate)(j);
            (names, top)
        };
        let a = run(capi);
        let b = run(rapi);
        assert_eq!(a, b, "js_pushiterator own={own}");
    }

    // --- js_type on every stack index, including invalid ones ---
    for idx in [-4096i32, -2, -1, 0, 1, 4096, i32::MAX, i32::MIN] {
        let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let run = |api: &Api| -> (c_int, Option<String>) {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            // wrap in a protected call so a "stack error!" throw is observable
            let nm = cstr("probe");
            (api.js_newcfunction)(j, TYPE_PROBE.load(), nm.as_ptr() as *const c_char, 0);
            (api.js_pushundefined)(j);
            IDX.store(idx, std::sync::atomic::Ordering::Relaxed);
            let rc = (api.js_pcall)(j, 0);
            let e = cstr("<e>");
            let s = unsafe { read_cstr((api.js_tryrepr)(j, -1, e.as_ptr() as *const c_char)) }
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            (api.js_pop)(j, 1);
            (api.js_freestate)(j);
            (rc, s)
        };
        // the probe needs the right Api; set it before each run
        CUR.store(capi as *const Api as *mut Api, std::sync::atomic::Ordering::Relaxed);
        let a = run(capi);
        CUR.store(rapi as *const Api as *mut Api, std::sync::atomic::Ordering::Relaxed);
        let b = run(rapi);
        assert_eq!(a, b, "js_type / js_typeof at idx={idx}");
    }
}

use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};

static CUR: AtomicPtr<Api> = AtomicPtr::new(std::ptr::null_mut());
static IDX: AtomicI32 = AtomicI32::new(0);

extern "C" fn type_probe(j: JsState) {
    let api = unsafe { &*CUR.load(Ordering::Relaxed) };
    let idx = IDX.load(Ordering::Relaxed);
    let t = (api.js_type)(j, idx);
    let s = unsafe { read_cstr((api.js_typeof)(j, idx)) }.unwrap_or_default();
    let msg = format!("{t}/{}", String::from_utf8_lossy(&s));
    let z = cstr(&msg);
    (api.js_pushstring)(j, z.as_ptr() as *const std::os::raw::c_char);
}

/// Trivial holder so the probe can be named in `js_newcfunction`.
struct ProbeSlot;
impl ProbeSlot {
    fn load(&self) -> JsCFunction {
        type_probe
    }
}
static TYPE_PROBE: ProbeSlot = ProbeSlot;
