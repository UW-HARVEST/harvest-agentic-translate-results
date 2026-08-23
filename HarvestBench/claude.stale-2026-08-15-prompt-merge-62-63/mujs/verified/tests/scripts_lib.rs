//! CONFIGS.md rows **H11..H20** — differential JavaScript corpus for the
//! standard library / lexer / stress surface of the interpreter.
//!
//! Every script is executed through `js_dostring` on a *fresh* `js_State` in
//! BOTH the C `libmujs.so` and the Rust `libmujs.so`; the return code and the
//! captured output (the pre-installed `print` plus the `[report] ...` line
//! emitted for an uncaught error) must be byte-identical.
//!
//! Determinism: no script in the main corpus reads the clock, uses
//! `Math.random()`, or depends on address/iteration-order of the host. The one
//! deliberate exception is `h12b_date_clock_dependent`, which is documented at
//! its definition.

mod common;
use common::*;

/* ------------------------------------------------------------------ */
/*  H11 — RegExp (c_src/src/jsregexp.c)                               */
/* ------------------------------------------------------------------ */

#[test]
fn h11_regexp() {
    diff_scripts_both_modes(H11_SCRIPTS);
}

const H11_SCRIPTS: &[&str] = &[
        /* --- literal form, source / flag properties --- */
        r#"var r = /abc/; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/g; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/i; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/m; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/gi; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/gm; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/im; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        r#"var r = /abc/gim; print(r.source, r.global, r.ignoreCase, r.multiline, r.lastIndex);"#,
        /* --- all 8 flag combos through toString --- */
        r#"print(/x/.toString());"#,
        r#"print(/x/g.toString());"#,
        r#"print(/x/i.toString());"#,
        r#"print(/x/m.toString());"#,
        r#"print(/x/gi.toString());"#,
        r#"print(/x/ig.toString());"#,
        r#"print(/x/gm.toString());"#,
        r#"print(/x/mg.toString());"#,
        r#"print(/x/im.toString());"#,
        r#"print(/x/mi.toString());"#,
        r#"print(/x/gim.toString());"#,
        r#"print(/x/mig.toString());"#,
        r#"print(/x/igm.toString());"#,
        r#"print(String(/a\/b/));"#,
        r#"print(/a\/b/.source);"#,
        /* --- all 8 flag combos through the constructor --- */
        r#"print(new RegExp('a+','').toString(), new RegExp('a+','').global);"#,
        r#"print(new RegExp('a+','g').toString(), new RegExp('a+','g').global);"#,
        r#"print(new RegExp('a+','i').toString(), new RegExp('a+','i').ignoreCase);"#,
        r#"print(new RegExp('a+','m').toString(), new RegExp('a+','m').multiline);"#,
        r#"print(new RegExp('a+','gi').toString());"#,
        r#"print(new RegExp('a+','gm').toString());"#,
        r#"print(new RegExp('a+','im').toString());"#,
        r#"print(new RegExp('a+','gim').toString());"#,
        r#"print(new RegExp('a+','mig').toString());"#,
        /* --- constructor variants --- */
        r#"print(RegExp('abc').toString(), typeof RegExp('abc'));"#,
        r#"print(RegExp('abc','g').toString());"#,
        r#"var a = /q/gi; var b = RegExp(a); print(a === b, b.toString());"#,
        r#"var a = /q/gi; var b = new RegExp(a); print(a === b, b.toString(), b.global, b.ignoreCase);"#,
        r#"try{ var b = new RegExp(/q/g, 'i'); print(b.toString()); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(new RegExp('').source, new RegExp('').toString());"#,
        r#"print(new RegExp().source, new RegExp().toString());"#,
        r#"print(new RegExp(undefined).source);"#,
        r#"print(new RegExp(null).source);"#,
        r#"print(new RegExp(123).source);"#,
        r#"print(new RegExp('a/b').source, new RegExp('a/b').toString());"#,
        r#"print(new RegExp('//').source);"#,
        /* --- lastIndex read / write --- */
        r#"var r = /a/g; print(r.lastIndex); r.lastIndex = 3; print(r.lastIndex);"#,
        r#"var r = /a/g; r.lastIndex = 100; print(r.exec('aaa'), r.lastIndex);"#,
        r#"var r = /a/g; r.lastIndex = 1; print(r.exec('aaa')[0], r.lastIndex);"#,
        r#"var r = /a/; r.lastIndex = 2; print(r.exec('aaa')[0], r.lastIndex);"#,
        r#"var r = /a/g; try{ r.source = 'z'; }catch(e){ print('caught', e.name); } print(r.source);"#,
        r#"var r = /a/g; try{ r.global = false; }catch(e){ print('caught', e.name); } print(r.global);"#,
        r#"var r = /a/g; try{ r.ignoreCase = true; }catch(e){ print('caught', e.name); } print(r.ignoreCase);"#,
        r#"var r = /a/g; try{ r.multiline = true; }catch(e){ print('caught', e.name); } print(r.multiline);"#,
        r#"var r = /a/g; r.lastIndex = 'x'; print(r.lastIndex);"#,
        r#"var r = /a/g; r.lastIndex = -1; print(r.lastIndex, r.exec('aaa'));"#,
        r#"var r = /a/g; r.lastIndex = 1.7; print(r.lastIndex);"#,
        /* --- exec with g: advance, null, reset --- */
        r#"var r = /a/g, s = 'aXaXa', m;
           while ((m = r.exec(s)) !== null) print(m[0], m.index, r.lastIndex);
           print('done', r.lastIndex);
           print(r.exec(s)[0], r.lastIndex);"#,
        r#"var r = /\d+/g, s = '12 345 6', m;
           while ((m = r.exec(s))) print(m[0], m.index, m.input, r.lastIndex);
           print('null?', r.exec(s), r.lastIndex);"#,
        r#"var r = /a/; for (var i=0;i<3;i++) print(r.exec('aaa')[0], r.exec('aaa').index, r.lastIndex);"#,
        r#"var r = /x/g; print(r.exec('aaa'), r.lastIndex);"#,
        r#"var r = /(?:)/g, m, n=0; while ((m = r.exec('abc')) && n < 8) { print(n, m[0], m.index, r.lastIndex); n++; } print('n', n);"#,
        r#"var r = /b*/g, m, n=0; while ((m = r.exec('abc')) && n < 8) { print(n, JSON.stringify(m[0]), m.index, r.lastIndex); n++; } print('n', n);"#,
        r#"var r = /a/g; print(r.exec(''), r.lastIndex); r.lastIndex = 5; print(r.exec(''), r.lastIndex);"#,
        /* --- result array shape --- */
        r#"var m = /(\w)(\d)?(z)?/.exec('a1'); print(m.length, m[0], m[1], m[2], m[3], m.index, m.input);"#,
        r#"var m = /(a)|(b)/.exec('b'); print(m.length, m[0], m[1], m[2], m.index);"#,
        r#"var m = /((a)(b))/.exec('ab'); print(m.length, m[0], m[1], m[2], m[3]);"#,
        r#"var m = /(a)(?:b)(c)/.exec('abc'); print(m.length, m[0], m[1], m[2]);"#,
        r#"var m = /x/.exec('abc'); print(m, m === null, typeof m);"#,
        r#"var m = /a/.exec('abc'); print(m instanceof Array, Array.isArray(m), m.length);"#,
        r#"var m = /(a)(b)/.exec('ab'); var k=[]; for (var p in m) k.push(p); print(k.sort().join(','));"#,
        /* --- test --- */
        r#"print(/a/.test('abc'), /z/.test('abc'), /A/i.test('abc'), /A/.test('abc'));"#,
        r#"var r = /a/g; print(r.test('aa'), r.lastIndex, r.test('aa'), r.lastIndex, r.test('aa'), r.lastIndex);"#,
        r#"var r = /a/g; r.lastIndex = 99; print(r.test('aa'), r.lastIndex);"#,
        r#"print(/a/.test(123), /1/.test(123), /true/.test(true), /null/.test(null), /undefined/.test(undefined));"#,
        /* --- String.prototype.match --- */
        r#"print(JSON.stringify('a1b2c3'.match(/\d/)));"#,
        r#"print(JSON.stringify('a1b2c3'.match(/\d/g)));"#,
        r#"print('abc'.match(/z/), 'abc'.match(/z/g));"#,
        r#"var m = 'a1b2'.match(/(\w)(\d)/); print(m.length, m[0], m[1], m[2], m.index, m.input);"#,
        r#"print(JSON.stringify('aaa'.match(/a*/g)));"#,
        r#"print(JSON.stringify(''.match(/(?:)/g)));"#,
        r#"print(JSON.stringify('abc'.match('b')));"#,
        /* --- search --- */
        r#"print('abc'.search(/b/), 'abc'.search(/z/), 'abc'.search(/a/g), 'abc'.search(/c/));"#,
        r#"print('abc'.search('b'), ''.search(/(?:)/));"#,
        /* --- split with a regexp --- */
        r#"print(JSON.stringify('a1b22c'.split(/\d+/)));"#,
        r#"print(JSON.stringify('a1b22c'.split(/(\d+)/)));"#,
        r#"print(JSON.stringify('abc'.split(/(?:)/)));"#,
        r#"print(JSON.stringify('abc'.split(/x/)));"#,
        r#"print(JSON.stringify('a,b,,c'.split(/,/)));"#,
        r#"print(JSON.stringify('a1b2c3'.split(/\d/, 2)));"#,
        r#"print(JSON.stringify(''.split(/x/)), JSON.stringify(''.split(/(?:)/)));"#,
        /* --- replace with a regexp and with a function --- */
        r#"print('a1b2'.replace(/\d/, 'X'));"#,
        r#"print('a1b2'.replace(/\d/g, 'X'));"#,
        r#"print('a1b2'.replace(/(\w)(\d)/g, '$2$1'));"#,
        r#"print('a1b2'.replace(/\d/g, function(m, off, s){ return '[' + m + ':' + off + ':' + s.length + ']'; }));"#,
        r#"print('a1b2'.replace(/(\w)(\d)/g, function(m, p1, p2, off, s){ return p2 + p1 + off; }));"#,
        r#"print('abc'.replace(/(b)/, '<$&|$1|$`|' + "$'" + '|$$>'));"#,
        r#"print('aaa'.replace(/a*/g, 'X'));"#,
        r#"print('abc'.replace(/z/g, 'X'), 'abc'.replace(/z/, 'X'));"#,
        r#"print('abc'.replace(/(z)?b/, function(m,p1){ return String(p1); }));"#,
        /* --- multi-byte UTF-8 subjects --- */
        r#"print(JSON.stringify('héllo wörld'.match(/\w+/g)));"#,
        r#"var m = /l/.exec('héllo'); print(m.index, m[0]);"#,
        r#"var r = /./g, m, o=[]; while ((m = r.exec('äöü'))) o.push(m[0] + '@' + m.index); print(o.join(' '));"#,
        r#"print('日本語'.match(/./g).length, '日本語'.match(/./g).join('|'));"#,
        r#"print('日本語'.replace(/本/, 'X'));"#,
        r#"print('αβγ'.search(/γ/), 'αβγ'.split(/β/).join('|'));"#,
        r#"print(/^.$/.test('é'), /^..$/.test('é'), 'é'.length);"#,
        /* --- anchors with and without m --- */
        r#"print(/^b/.test('a\nb'), /^b/m.test('a\nb'));"#,
        r#"print(/a$/.test('a\nb'), /a$/m.test('a\nb'));"#,
        r#"print(JSON.stringify('a\nb\nc'.match(/^./gm)));"#,
        r#"print(JSON.stringify('a\nb\nc'.match(/^./g)));"#,
        r#"print(JSON.stringify('a\nb\nc'.match(/.$/gm)));"#,
        r#"print(/^$/.test(''), /^$/m.test('a\n'), /^$/.test('a\n'));"#,
        r#"var r = /^a/gm, m, o=[]; while ((m = r.exec('ab\nac\nad'))) o.push(m.index); print(o.join(','));"#,
        r#"var r = /^a/g, m, o=[]; while ((m = r.exec('ab\nac\nad'))) o.push(m.index); print(o.join(','));"#,
        /* --- back-references --- */
        r#"print(/(a)\1/.test('aa'), /(a)\1/.test('ab'));"#,
        r#"var m = /(\w)\1/.exec('xaab'); print(m.index, m[0], m[1]);"#,
        r#"print(/(a)(b)\2\1/.test('abba'), /(a)(b)\1\2/.test('abba'));"#,
        r#"print('abcabc'.replace(/(abc)\1/, 'X'));"#,
        r#"print(/(z)?a\1/.test('a'));"#,
        /* --- lookahead --- */
        r#"print(/a(?=b)/.test('ab'), /a(?=b)/.test('ac'));"#,
        r#"print(/a(?!b)/.test('ab'), /a(?!b)/.test('ac'));"#,
        r#"var m = /a(?=b)/.exec('ab'); print(m[0], m.index, m.length);"#,
        r#"print(JSON.stringify('a1b2'.match(/[a-z](?=\d)/g)));"#,
        /* --- character classes --- */
        r#"print(JSON.stringify('a1 b2'.match(/\d/g)), JSON.stringify('a1 b2'.match(/\D/g)));"#,
        r#"print(JSON.stringify('a1 b2'.match(/\w/g)), JSON.stringify('a1 b2'.match(/\W/g)));"#,
        r#"print(JSON.stringify('a1 b2'.match(/\s/g)), JSON.stringify('a1 b2'.match(/\S/g)));"#,
        r#"print(JSON.stringify('a-b]c'.match(/[-\]]/g)));"#,
        r#"print(JSON.stringify('abcXYZ'.match(/[a-c]/g)), JSON.stringify('abcXYZ'.match(/[^a-c]/g)));"#,
        r#"print(/[\b]/.test('\b'), /\b/.test('a'), /\B/.test('a'));"#,
        r#"print(JSON.stringify('a\tb\nc'.match(/[\t\n]/g)));"#,
        r#"print(/[\x41]/.test('A'), /[A]/.test('A'), /\x41/.test('A'), /A/.test('A'));"#,
        r#"print(/[0-9a-fA-F]+/.exec('zz1aF!')[0]);"#,
        r#"print(/a{2}/.test('aa'), /a{2,}/.test('a'), /a{2,3}/.exec('aaaa')[0], /a+?/.exec('aaa')[0]);"#,
        /* --- invalid patterns and flags (caught) --- */
        r#"try{ new RegExp('('); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp(')'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('['); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a{2,1}'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('*'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a','x'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a','gg'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a','ii'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a','mm'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a','G'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('a','gim '); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ new RegExp('\\'); }catch(e){ print('caught', e.name, e.message); }"#,
        /* uncaught, so the [report] path is compared too */
        r#"print('before'); new RegExp('(');"#,
        r#"print('before'); new RegExp('a','q');"#,
        r#"print('before'); /a/.exec.call({}, 'x');"#,
        r#"try{ RegExp.prototype.exec.call({}, 'x'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ RegExp.prototype.test.call('str', 'x'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ RegExp.prototype.toString.call(5); }catch(e){ print('caught', e.name, e.message); }"#,
        /* --- misc --- */
        r#"print(typeof /a/, /a/ instanceof RegExp, Object.prototype.toString.call(/a/));"#,
        r#"print(RegExp.prototype.source, RegExp.prototype.global, RegExp.prototype.toString());"#,
        r#"print(RegExp.length, typeof RegExp, RegExp.prototype.exec.length);"#,
        r#"var r=/a/; print(r.hasOwnProperty('source'), 'source' in r, r.propertyIsEnumerable('source'));"#,
        r#"var k=[]; for (var p in /a/g) k.push(p); print(k.length, k.join(','));"#,
        r#"print(Object.getOwnPropertyNames(/a/g).sort().join(','));"#,
        r#"print(delete /a/g.source);"#,
        r#"var r = /a/g; print(JSON.stringify(r));"#,
];

/* ------------------------------------------------------------------ */
/*  H12 — Date, without ever reading the clock (c_src/src/jsdate.c)   */
/* ------------------------------------------------------------------ */

#[test]
fn h12_date_deterministic() {
    // NOTE: the absolute values below depend on the process `TZ` (mujs caches
    // LocalTZA on first use). Both libraries run inside the SAME process and
    // therefore observe the SAME `TZ`, so the comparison is meaningful and
    // stable regardless of what `TZ` happens to be.
    diff_scripts_both_modes(H12_SCRIPTS);
}

const H12_SCRIPTS: &[&str] = &[
        /* --- new Date(ms) --- */
        r#"print(new Date(0).getTime(), new Date(0).valueOf());"#,
        r#"print(new Date(-1).getTime(), new Date(1).getTime());"#,
        r#"print(new Date(-86400000).getTime());"#,
        r#"print(new Date(1e12).getTime());"#,
        r#"print(new Date(-1e12).getTime());"#,
        r#"print(new Date(8.64e15).getTime());"#,
        r#"print(new Date(-8.64e15).getTime());"#,
        r#"print(new Date(8.64e15 + 1).getTime());"#,
        r#"print(new Date(-8.64e15 - 1).getTime());"#,
        r#"print(new Date(NaN).getTime());"#,
        r#"print(new Date(Infinity).getTime(), new Date(-Infinity).getTime());"#,
        r#"print(new Date(0.5).getTime(), new Date(-0.5).getTime(), new Date(1.9).getTime(), new Date(-1.9).getTime());"#,
        r#"print(new Date(-0).getTime());"#,
        r#"var t=[0,-1,1,1000,-1000,1e9,-1e9,1e12,-1e12,8.64e15,-8.64e15,8.64e15+1,NaN,0.5,-0.5];
           for (var i=0;i<t.length;i++) print(t[i], new Date(t[i]).getTime());"#,
        r#"print(new Date(true).getTime(), new Date(false).getTime(), new Date(null).getTime());"#,
        r#"print(new Date('0').getTime());"#,
        r#"print(new Date({valueOf:function(){return 1234;}}).getTime());"#,
        r#"print(new Date({toString:function(){return '2020-01-02';}}).getTime());"#,
        /* --- new Date(y, m, ...) with 2..7 arguments --- */
        r#"print(new Date(2020,0).getTime());"#,
        r#"print(new Date(2020,0,1).getTime());"#,
        r#"print(new Date(2020,0,1,0).getTime());"#,
        r#"print(new Date(2020,0,1,0,0).getTime());"#,
        r#"print(new Date(2020,0,1,0,0,0).getTime());"#,
        r#"print(new Date(2020,0,1,0,0,0,0).getTime());"#,
        r#"print(new Date(2020,0,1,0,0,0,0,999).getTime());"#,
        r#"print(new Date(2020,11,31,23,59,59,999).getTime());"#,
        r#"print(new Date(2020,13,1).getTime());"#,
        r#"print(new Date(2020,-1,1).getTime());"#,
        r#"print(new Date(2020,0,0).getTime());"#,
        r#"print(new Date(2020,0,32).getTime());"#,
        r#"print(new Date(2020,1,30).getTime());"#,
        r#"print(new Date(2020,0,1,24).getTime());"#,
        r#"print(new Date(2020,0,1,-1).getTime());"#,
        r#"print(new Date(2020,0,1,0,60).getTime());"#,
        r#"print(new Date(2020,0,1,0,0,60).getTime());"#,
        r#"print(new Date(2020,0,1,0,0,0,1000).getTime());"#,
        r#"print(new Date(0,0,1).getTime());"#,
        r#"print(new Date(99,0,1).getTime());"#,
        r#"print(new Date(100,0,1).getTime());"#,
        r#"print(new Date(1969,0,1).getTime());"#,
        r#"print(new Date(NaN,0).getTime(), new Date(2020,NaN).getTime(), new Date(2020,0,NaN).getTime());"#,
        r#"print(new Date(1e10,0).getTime());"#,
        r#"print(new Date(2000,1,29).getTime(), new Date(1900,1,29).getTime(), new Date(2100,1,29).getTime());"#,
        r#"for (var m=-2;m<=14;m++) print(m, new Date(2020,m,1).getTime());"#,
        r#"for (var d=-2;d<=33;d++) print(d, new Date(2020,0,d).getTime());"#,
        /* --- new Date(string) / Date.parse --- */
        r#"print(Date.parse('2020-01-01'), new Date('2020-01-01').getTime());"#,
        r#"print(Date.parse('2020'), Date.parse('2020-06'), Date.parse('2020-06-15'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00Z'));"#,
        r#"print(Date.parse('2020-01-01T00:00Z'));"#,
        r#"print(Date.parse('2020-01-01T12:34:56.789Z'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00.123+05:30'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00.123-05:30'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00+00'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00-08'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00'));"#,
        r#"print(Date.parse('2020-01-01T24:00:00Z'), Date.parse('2020-01-01T24:00:01Z'));"#,
        r#"print(Date.parse('2020-01-01T25:00:00Z'));"#,
        r#"print(Date.parse('2020-13-01'), Date.parse('2020-00-01'));"#,
        r#"print(Date.parse('2020-01-32'), Date.parse('2020-01-00'));"#,
        r#"print(Date.parse('2020-01-01T00:60:00Z'), Date.parse('2020-01-01T00:00:60Z'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00.999Z'), Date.parse('2020-01-01T00:00:00.99Z'));"#,
        r#"print(Date.parse('Mon Jan 01 2020'));"#,
        r#"print(Date.parse('Mon, 01 Jan 2020 00:00:00 GMT'));"#,
        r#"print(Date.parse('January 1, 2020'));"#,
        r#"print(Date.parse('garbage'), Date.parse(''), Date.parse(' '));"#,
        r#"print(Date.parse('2020-01-01 '), Date.parse(' 2020-01-01'));"#,
        r#"print(Date.parse('2020-1-1'), Date.parse('20200101'));"#,
        r#"print(Date.parse('+2020-01-01'), Date.parse('-2020-01-01'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00Z '), Date.parse('2020-01-01T00:00:00ZZ'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00+24:00'), Date.parse('2020-01-01T00:00:00+23:59'));"#,
        r#"print(Date.parse('2020-01-01T00:00:00+00:60'));"#,
        r#"print(Date.parse(), Date.parse(undefined), Date.parse(null), Date.parse(0));"#,
        r#"var s=['2020-01-01','1970-01-01','1969-12-31','9999-12-31','0000-01-01','2020-02-29','2019-02-29','2020-01-01T00:00:00.000Z','2020-01-01t00:00:00Z','x'];
           for (var i=0;i<s.length;i++) print(s[i], Date.parse(s[i]));"#,
        r#"print(new Date('garbage').getTime(), String(new Date('garbage')));"#,
        /* --- Date.UTC with 1..7 arguments --- */
        r#"print(Date.UTC(2020));"#,
        r#"print(Date.UTC(2020,0));"#,
        r#"print(Date.UTC(2020,0,1));"#,
        r#"print(Date.UTC(2020,0,1,1));"#,
        r#"print(Date.UTC(2020,0,1,1,2));"#,
        r#"print(Date.UTC(2020,0,1,1,2,3));"#,
        r#"print(Date.UTC(2020,0,1,1,2,3,4));"#,
        r#"print(Date.UTC(2020,0,1,1,2,3,4,5));"#,
        r#"print(Date.UTC(), Date.UTC(NaN), Date.UTC(2020,NaN));"#,
        r#"print(Date.UTC(70,0,1), Date.UTC(99,11,31), Date.UTC(0,0,1));"#,
        r#"print(Date.UTC(2020,13,32,25,61,61,1001));"#,
        r#"print(Date.UTC(275760,8,13), Date.UTC(275760,8,14));"#,
        r#"print(Date.UTC.length, Date.parse.length, Date.length);"#,
        /* --- every getter on several fixed timestamps + invalid --- */
        r#"var G=['getTime','getFullYear','getMonth','getDate','getDay','getHours','getMinutes','getSeconds','getMilliseconds','getTimezoneOffset',
                 'getUTCFullYear','getUTCMonth','getUTCDate','getUTCDay','getUTCHours','getUTCMinutes','getUTCSeconds','getUTCMilliseconds'];
           var T=[0, 1, -1, 1000, 1577836800000, 1234567890123, -1234567890123, 8.64e15, -8.64e15, NaN];
           for (var i=0;i<T.length;i++) { var d = new Date(T[i]); var o=[];
             for (var j=0;j<G.length;j++) o.push(G[j] + '=' + d[G[j]]());
             print(T[i], o.join(' ')); }"#,
        r#"var d = new Date(1577836800000);
           print(d.toString()); print(d.toDateString()); print(d.toTimeString());
           print(d.toUTCString()); print(d.toISOString()); print(d.toJSON()); print(d.valueOf());
           print(d.toLocaleString()); print(d.toLocaleDateString()); print(d.toLocaleTimeString());"#,
        r#"var d = new Date(0);
           print(d.toString()); print(d.toDateString()); print(d.toTimeString());
           print(d.toUTCString()); print(d.toISOString()); print(d.toJSON());"#,
        r#"var d = new Date(-1);
           print(d.toUTCString()); print(d.toISOString());"#,
        r#"var d = new Date(8.64e15); print(d.toISOString(), d.toUTCString());"#,
        r#"var d = new Date(-8.64e15); print(d.toISOString(), d.toUTCString());"#,
        r#"var d = new Date(NaN);
           print(d.toString(), d.toDateString(), d.toTimeString(), d.toUTCString());
           print(d.toJSON());
           try{ d.toISOString(); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(new Date(NaN).toJSON(), JSON.stringify(new Date(NaN)));"#,
        r#"print(JSON.stringify(new Date(1577836800000)));"#,
        r#"print(JSON.stringify({d:new Date(0)}));"#,
        r#"print(new Date(0) + '', typeof (new Date(0) + ''));"#,
        r#"print(+new Date(1234), new Date(1234) * 1, new Date(1234) - 0);"#,
        r#"print(Date(0) === Date(0), typeof Date(1234));"#,
        /* --- every setter, in-range / out-of-range / NaN --- */
        r#"var d = new Date(0); print(d.setTime(1000), d.getTime());"#,
        r#"var d = new Date(0); print(d.setTime(NaN), d.getTime());"#,
        r#"var d = new Date(0); print(d.setTime(8.64e15+1), d.getTime());"#,
        r#"var d = new Date(0); print(d.setTime(), d.getTime());"#,
        r#"var S=['setFullYear','setMonth','setDate','setHours','setMinutes','setSeconds','setMilliseconds',
                 'setUTCFullYear','setUTCMonth','setUTCDate','setUTCHours','setUTCMinutes','setUTCSeconds','setUTCMilliseconds'];
           var V=[0, 1, 5, 60, 100, 400, -1, -100, 2020, NaN, 1e10, 0.5];
           for (var i=0;i<S.length;i++) { var o=[];
             for (var j=0;j<V.length;j++) { var d = new Date(1577836800000); o.push(V[j] + '->' + d[S[i]](V[j])); }
             print(S[i], o.join(' ')); }"#,
        r#"var S=['setFullYear','setMonth','setHours','setMinutes','setSeconds',
                 'setUTCFullYear','setUTCMonth','setUTCHours','setUTCMinutes','setUTCSeconds'];
           for (var i=0;i<S.length;i++) { var d = new Date(0); print(S[i], d[S[i]]()); }"#,
        r#"var d = new Date(1577836800000); print(d.setFullYear(2021, 5, 15), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setHours(1,2,3,4), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setUTCHours(1,2,3,4), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setMonth(1, 29), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setUTCMonth(13), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setMinutes(5, 6, 7), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setUTCMinutes(5, 6, 7), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setSeconds(5, 6), d.toISOString());"#,
        r#"var d = new Date(1577836800000); print(d.setUTCSeconds(5, 6), d.toISOString());"#,
        r#"var d = new Date(NaN); print(d.setFullYear(2020), d.getTime());"#,
        r#"var d = new Date(NaN); print(d.setMilliseconds(5), d.getTime());"#,
        r#"var d = new Date(NaN); print(d.setTime(0), d.getTime(), d.toISOString());"#,
        r#"var d = new Date(0); d.setUTCFullYear(275760, 8, 14); print(d.getTime());"#,
        r#"var d = new Date(0); print(d.setDate(0), d.setDate(-1));"#,
        r#"var d = new Date(0); print(d.setUTCDate(32));"#,
        /* --- prototype methods on non-Date receivers --- */
        r#"try{ Date.prototype.getTime.call({}); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ Date.prototype.getFullYear.call([]); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ Date.prototype.toISOString.call('x'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ Date.prototype.setTime.call({}, 0); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ Date.prototype.valueOf.call(null); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ Date.prototype.toJSON.call({}); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(Date.prototype.getTime.call(Date.prototype));"#,
        r#"print('before'); Date.prototype.getTime.call({});"#,
        r#"print('before'); new Date(NaN).toISOString();"#,
        /* --- misc --- */
        r#"print(typeof Date, typeof Date.prototype, Object.prototype.toString.call(new Date(0)));"#,
        r#"print(new Date(0) instanceof Date, Date.prototype instanceof Date);"#,
        r#"var d = new Date(0); print(d.getTime.length, d.setTime.length, d.setHours.length, d.setFullYear.length);"#,
        r#"var k=[]; for (var p in new Date(0)) k.push(p); print(k.length);"#,
        r#"print(Object.getOwnPropertyNames(Date.prototype).length > 30);"#,
];

/// Clock-dependent Date smoke test.
///
/// `Date.now()` and `new Date()` cannot be compared byte-for-byte between the
/// two libraries: they are evaluated at two different instants, so the values
/// legitimately differ by however long the first call took. Rather than compare
/// output we only assert that both libraries return a plausible epoch
/// millisecond count — within 2000 ms of each other and of the Rust
/// `SystemTime` clock. That checks the plumbing (`gettimeofday` scaling, the
/// `floor`, the pushed number type) without introducing flakiness.
#[test]
fn h12b_date_clock_dependent() {
    use std::time::{SystemTime, UNIX_EPOCH};

    fn epoch_ms_of(out: &[u8]) -> f64 {
        let s = String::from_utf8_lossy(out);
        let line = s.lines().next().unwrap_or("");
        line.trim().parse::<f64>().unwrap_or_else(|e| {
            panic!("could not parse epoch ms from {:?}: {}", s, e);
        })
    }

    let rust_before = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as f64;

    let (c_now, r_now) = both(|api, _| run_script(api, 0, "print(Date.now());"));
    let (c_new, r_new) = both(|api, _| run_script(api, 0, "print(new Date().getTime());"));
    let (c_str, r_str) =
        both(|api, _| run_script(api, 0, "print(typeof Date(), String(Date()).length > 10);"));

    let rust_after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as f64;

    assert_eq!(c_now.0, 0, "C Date.now() failed: {:?}", String::from_utf8_lossy(&c_now.1));
    assert_eq!(r_now.0, 0, "Rust Date.now() failed: {:?}", String::from_utf8_lossy(&r_now.1));
    assert_eq!(c_new.0, 0);
    assert_eq!(r_new.0, 0);

    // `Date()` (the string form) is not clock-value dependent in its *shape*,
    // so this part must agree exactly.
    assert_eq!(c_str, r_str, "Date() shape diverged: C={:?} Rust={:?}", c_str, r_str);

    let vals = [
        ("C Date.now", epoch_ms_of(&c_now.1)),
        ("Rust Date.now", epoch_ms_of(&r_now.1)),
        ("C new Date", epoch_ms_of(&c_new.1)),
        ("Rust new Date", epoch_ms_of(&r_new.1)),
    ];
    for (what, v) in vals.iter() {
        assert!(
            *v >= rust_before - 2000.0 && *v <= rust_after + 2000.0,
            "{} = {} is not within 2000ms of the Rust clock window [{}, {}]",
            what,
            v,
            rust_before,
            rust_after
        );
        assert_eq!(*v, v.floor(), "{} = {} is not an integral millisecond count", what, v);
    }
    for (an, av) in vals.iter() {
        for (bn, bv) in vals.iter() {
            assert!(
                (av - bv).abs() <= 2000.0,
                "{}={} and {}={} differ by more than 2000ms",
                an,
                av,
                bn,
                bv
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/*  H13 — JSON (c_src/src/json.c)                                     */
/* ------------------------------------------------------------------ */

#[test]
fn h13_json() {
    diff_scripts_both_modes(&[
        /* --- JSON.parse: every value shape --- */
        r#"print(JSON.parse('null'), typeof JSON.parse('null'));"#,
        r#"print(JSON.parse('true'), JSON.parse('false'), typeof JSON.parse('true'));"#,
        r#"print(JSON.parse('0'), JSON.parse('-0'), JSON.parse('1'), JSON.parse('-1'));"#,
        r#"print(JSON.parse('1.5'), JSON.parse('-1.5'), JSON.parse('0.0'));"#,
        r#"print(JSON.parse('1e3'), JSON.parse('1E3'), JSON.parse('1e+3'), JSON.parse('1e-3'));"#,
        r#"print(JSON.parse('1.25e2'), JSON.parse('-1.25E-2'));"#,
        r#"print(JSON.parse('1e400'), JSON.parse('-1e400'), JSON.parse('1e-400'));"#,
        r#"print(JSON.parse('9007199254740993'), JSON.parse('123456789012345678901234567890'));"#,
        r#"print(JSON.parse('""').length, JSON.stringify(JSON.parse('"abc"')));"#,
        r#"print(JSON.stringify(JSON.parse('[]')), JSON.parse('[]').length);"#,
        r#"print(JSON.stringify(JSON.parse('{}')), Object.keys(JSON.parse('{}')).length);"#,
        r#"print(JSON.stringify(JSON.parse('[1,2,3]')));"#,
        r#"print(JSON.stringify(JSON.parse('{"a":1,"b":2}')));"#,
        r#"print(JSON.stringify(JSON.parse('[[[[[1]]]]]')));"#,
        r#"print(JSON.stringify(JSON.parse('{"a":{"b":{"c":{"d":1}}}}')));"#,
        r#"print(JSON.stringify(JSON.parse('[1,"a",true,false,null,{},[]]')));"#,
        r#"print(JSON.parse('{"a":1,"a":2}').a, Object.keys(JSON.parse('{"a":1,"a":2}')).length);"#,
        r#"print(JSON.parse('  \t\n [ 1 , 2 ] \r\n ').length);"#,
        r#"var a = JSON.parse('[1,[2,[3,[4]]]]'); print(a[1][1][1][0], a.length, Array.isArray(a));"#,
        /* --- JSON.parse: string escapes --- */
        r#"print(JSON.parse('"a\\"b"'));"#,
        r#"print(JSON.parse('"a\\\\b"'));"#,
        r#"print(JSON.parse('"a\\/b"'));"#,
        r#"print(JSON.parse('"a\\bb"').charCodeAt(1));"#,
        r#"print(JSON.parse('"a\\fb"').charCodeAt(1));"#,
        r#"print(JSON.parse('"a\\nb"').charCodeAt(1), JSON.parse('"a\\nb"').length);"#,
        r#"print(JSON.parse('"a\\rb"').charCodeAt(1));"#,
        r#"print(JSON.parse('"a\\tb"').charCodeAt(1));"#,
        r#"print(JSON.parse('"\\u0041\\u00e9\\u4e2d"'));"#,
        r#"print(JSON.parse('"\\u0000"').length, JSON.parse('"\\u0000"').charCodeAt(0));"#,
        r#"print(JSON.parse('"\\uD834\\uDD1E"').length);"#,
        r#"print(JSON.parse('"\\ud800"').length);"#,
        r#"print(JSON.parse('"héllo 日本"'));"#,
        r#"print(JSON.parse('"héllo"').length);"#,
        r#"print(JSON.stringify(JSON.parse('"\\u0001\\u001f"')));"#,
        /* --- JSON.parse with a reviver --- */
        r#"print(JSON.stringify(JSON.parse('{"a":1,"b":2}', function(k,v){ return typeof v === 'number' ? v*10 : v; })));"#,
        r#"print(JSON.stringify(JSON.parse('{"a":1,"b":2}', function(k,v){ return k === 'a' ? undefined : v; })));"#,
        r#"print(JSON.stringify(JSON.parse('[1,2,3]', function(k,v){ return typeof v === 'number' && v === 2 ? undefined : v; })));"#,
        r#"var o=[]; JSON.parse('{"a":{"b":1}}', function(k,v){ o.push(k + ':' + typeof v); return v; }); print(o.join(' '));"#,
        r#"var o=[]; JSON.parse('[1,[2]]', function(k,v){ o.push(k + ':' + typeof v); return v; }); print(o.join(' '));"#,
        r#"print(JSON.parse('1', function(k,v){ return v + 1; }));"#,
        r#"print(JSON.parse('1', function(k,v){ print('key', JSON.stringify(k), 'this', typeof this); return v; }));"#,
        r#"try{ JSON.parse('{"a":1}', function(k,v){ if (k === 'a') throw new Error('boom'); return v; }); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(JSON.stringify(JSON.parse('{"a":1}', function(k,v){ return undefined; })));"#,
        r#"print(JSON.parse('[1,2]', 5).length, JSON.parse('[1,2]', null).length, JSON.parse('[1,2]', undefined).length);"#,
        r#"print(JSON.stringify(JSON.parse('{"a":[1,2],"b":{"c":3}}', function(k,v){ return v; })));"#,
        /* --- JSON.parse: malformed input --- */
        r#"try{ JSON.parse(''); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('   '); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('{'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('}'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('['); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('[1,'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('[1,]'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('{"a":1,}'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('{a:1}'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse("{'a':1}"); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('{"a"}'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('{"a":}'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('nul'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('NaN'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('Infinity'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('undefined'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('01'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('+1'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('.5'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('1.'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('1e'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('"abc'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('"a\\qb"'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('"\\u00"'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('[1 2]'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('1 2'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ JSON.parse('/*x*/1'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print('before'); JSON.parse('{');"#,
        r#"print('before'); JSON.parse('x');"#,
        /* --- JSON.stringify: value shapes --- */
        r#"print(JSON.stringify(null), JSON.stringify(true), JSON.stringify(false));"#,
        r#"print(JSON.stringify(0), JSON.stringify(-0), JSON.stringify(1), JSON.stringify(-1));"#,
        r#"print(JSON.stringify(1.5), JSON.stringify(1e21), JSON.stringify(1e-7), JSON.stringify(1e100));"#,
        r#"print(JSON.stringify(NaN), JSON.stringify(Infinity), JSON.stringify(-Infinity));"#,
        r#"print(JSON.stringify('abc'), JSON.stringify(''));"#,
        r#"print(JSON.stringify(undefined), typeof JSON.stringify(undefined));"#,
        r#"print(JSON.stringify(function(){}), typeof JSON.stringify(function(){}));"#,
        r#"print(JSON.stringify(print), JSON.stringify(Math.floor));"#,
        r#"print(JSON.stringify([undefined, function(){}, 1]));"#,
        r#"print(JSON.stringify({a:undefined, b:function(){}, c:1}));"#,
        r#"print(JSON.stringify([NaN, Infinity, -Infinity, -0]));"#,
        r#"print(JSON.stringify([]), JSON.stringify({}), JSON.stringify([[]]), JSON.stringify({a:{}}));"#,
        r#"print(JSON.stringify({a:[1,{b:[2,[3]]}]}));"#,
        r#"print(JSON.stringify([1,[2,[3,[4,[5]]]]]));"#,
        r#"var a=[1,2,3]; a[10]=4; print(JSON.stringify(a));"#,
        r#"var a=[]; a.length=3; print(JSON.stringify(a));"#,
        r#"print(JSON.stringify({0:'a',1:'b'}));"#,
        r#"print(JSON.stringify(new Number(5)), JSON.stringify(new String('s')), JSON.stringify(new Boolean(true)));"#,
        r#"print(JSON.stringify([new Number(NaN), new String(''), new Boolean(false)]));"#,
        r#"print(JSON.stringify(new Date(1577836800000)));"#,
        r#"print(JSON.stringify({d:new Date(0), n:new Date(NaN)}));"#,
        r#"print(JSON.stringify({toJSON:function(){ return 42; }}));"#,
        r#"print(JSON.stringify({toJSON:function(k){ return 'k=' + k; }}));"#,
        r#"print(JSON.stringify({a:{toJSON:function(k){ return 'k=' + k; }}}));"#,
        r#"print(JSON.stringify([{toJSON:function(k){ return k; }},{toJSON:function(k){ return k; }}]));"#,
        r#"print(JSON.stringify({toJSON:5}));"#,
        r#"print(JSON.stringify({toJSON:function(){ return undefined; }}));"#,
        r#"print(JSON.stringify({toJSON:function(){ return {a:1}; }}));"#,
        r#"try{ JSON.stringify({toJSON:function(){ throw new Error('tj'); }}); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(JSON.stringify('a"b'), JSON.stringify('a\\b'), JSON.stringify('a/b'));"#,
        r#"print(JSON.stringify('a\nb'), JSON.stringify('a\tb'), JSON.stringify('a\rb'));"#,
        r#"print(JSON.stringify('a\bb'), JSON.stringify('a\fb'), JSON.stringify('a\vb'));"#,
        r#"print(JSON.stringify('\x00\x01\x1f\x20'));"#,
        r#"print(JSON.stringify('héllo 日本語'));"#,
        r#"print(JSON.stringify('\ud800'), JSON.stringify('\udfff'));"#,
        /* --- cyclic structures --- */
        r#"var o={}; o.self=o; try{ JSON.stringify(o); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"var a=[]; a.push(a); try{ JSON.stringify(a); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"var a={}, b={a:a}; a.b=b; try{ JSON.stringify(a); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"var o={}; var s={x:o, y:o}; print(JSON.stringify(s));"#,
        r#"var o={}; o.self=o; print('before'); JSON.stringify(o);"#,
        /* --- replacer function --- */
        r#"print(JSON.stringify({a:1,b:2}, function(k,v){ return typeof v === 'number' ? v+1 : v; }));"#,
        r#"print(JSON.stringify({a:1,b:2}, function(k,v){ return k === 'a' ? undefined : v; }));"#,
        r#"print(JSON.stringify([1,2,3], function(k,v){ return typeof v === 'number' ? v*2 : v; }));"#,
        r#"var o=[]; JSON.stringify({a:{b:1}}, function(k,v){ o.push(JSON.stringify(k)); return v; }); print(o.join(' '));"#,
        r#"print(JSON.stringify(1, function(k,v){ return 'x'; }));"#,
        r#"try{ JSON.stringify({a:1}, function(k,v){ if (k==='a') throw new RangeError('r'); return v; }); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(JSON.stringify({a:1}, function(k,v){ return v; }, 2));"#,
        /* --- replacer array --- */
        r#"print(JSON.stringify({a:1,b:2,c:3}, ['a','c']));"#,
        r#"print(JSON.stringify({a:1,b:2,c:3}, []));"#,
        r#"print(JSON.stringify({a:1,b:2,c:3}, ['a','a','b']));"#,
        r#"print(JSON.stringify({0:'x',1:'y',a:1}, [0,1]));"#,
        r#"print(JSON.stringify({a:1,b:2}, ['b','a']));"#,
        r#"print(JSON.stringify({a:1,b:2}, [new String('a')]));"#,
        r#"print(JSON.stringify({a:1,b:2}, [true, null, 'a']));"#,
        r#"print(JSON.stringify([{a:1,b:2}], ['a']));"#,
        r#"print(JSON.stringify({a:{a:1,b:2},b:2}, ['a']));"#,
        r#"print(JSON.stringify({a:1,b:2}, {}));"#,
        /* --- space --- */
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, undefined));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 0));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 1));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 4));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 10));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 11));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 100));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, -1));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, 2.9));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, NaN));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, '  '));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, '\t'));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, ''));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, '123456789012'));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, true));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, {}));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, new Number(3)));"#,
        r#"print(JSON.stringify({a:1,b:[1,2]}, null, new String('--')));"#,
        r#"print(JSON.stringify([], null, 4), JSON.stringify({}, null, 4));"#,
        r#"print(JSON.stringify([[],{}], null, 2));"#,
        r#"print(JSON.stringify({a:{},b:[]}, null, 2));"#,
        r#"print(JSON.stringify({a:undefined}, null, 2));"#,
        r#"print(JSON.stringify([undefined], null, 2));"#,
        /* --- round trips --- */
        r#"var v=[1,'a',true,null,[1,2],{a:1},{a:[1,{b:2}]},'',[],{},-0,1e21,'\n\t"\\'];
           for (var i=0;i<v.length;i++) { var s = JSON.stringify(v[i]); print(s, JSON.stringify(JSON.parse(s)), JSON.stringify(JSON.parse(s)) === s); }"#,
        r#"var s = JSON.stringify({a:[1,2,{b:'x'}],c:null}); print(s); print(JSON.stringify(JSON.parse(s)) === s);"#,
        r#"var s = '{"a":[1,2,3],"b":{"c":"d"}}'; print(JSON.stringify(JSON.parse(s)) === s);"#,
        r#"print(JSON.stringify(JSON.parse(JSON.stringify('héllo'))));"#,
        /* --- misc --- */
        r#"print(typeof JSON, JSON.parse.length, JSON.stringify.length);"#,
        r#"print(Object.prototype.toString.call(JSON), Object.getOwnPropertyNames(JSON).sort().join(','));"#,
        r#"var k=[]; for (var p in JSON) k.push(p); print(k.length);"#,
        r#"try{ JSON.parse(); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(JSON.stringify());"#,
    ]);
}

/* ------------------------------------------------------------------ */
/*  H14 — Error hierarchy (c_src/src/jserror.c)                       */
/* ------------------------------------------------------------------ */

#[test]
fn h14_errors() {
    diff_scripts_both_modes(&[
        /* --- all seven constructors, with and without `new` --- */
        r#"var C=['Error','EvalError','RangeError','ReferenceError','SyntaxError','TypeError','URIError'];
           for (var i=0;i<C.length;i++) { var e = new (this[C[i]])();
             print(C[i], e.name, JSON.stringify(e.message), e.toString()); }"#,
        r#"var C=['Error','EvalError','RangeError','ReferenceError','SyntaxError','TypeError','URIError'];
           for (var i=0;i<C.length;i++) { var e = (this[C[i]])('m');
             print(C[i], e.name, e.message, e.toString(), e instanceof Error); }"#,
        r#"print(new Error().message === '', new Error().name, new Error().toString());"#,
        r#"print(new Error('x').message, new Error('x').toString());"#,
        r#"print(Error('x').message, Error('x') instanceof Error);"#,
        r#"print(new Error(undefined).message === '', 'message' in new Error(undefined));"#,
        r#"print(new Error(null).message, new Error(0).message, new Error(false).message);"#,
        r#"print(new Error({}).message, new Error([1,2]).message);"#,
        r#"print(new Error({toString:function(){ return 'OBJ'; }}).message);"#,
        r#"try{ new Error({toString:function(){ throw new RangeError('nested'); }}); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(new TypeError('t').name, new TypeError('t').message, new TypeError('t').toString());"#,
        r#"print(new URIError('u').toString(), new EvalError('e').toString());"#,
        r#"print(new RangeError('r') instanceof RangeError, new RangeError('r') instanceof Error, new RangeError('r') instanceof TypeError);"#,
        r#"print(Error.length, TypeError.length, typeof Error, typeof Error.prototype);"#,
        r#"print(Object.prototype.toString.call(new Error('x')));"#,
        r#"print(Error.prototype.name, JSON.stringify(Error.prototype.message), Error.prototype.toString());"#,
        r#"print(TypeError.prototype.name, TypeError.prototype instanceof Error);"#,
        r#"print(Object.getPrototypeOf(new TypeError('x')) === TypeError.prototype);"#,
        r#"print(Object.getPrototypeOf(TypeError.prototype) === Error.prototype);"#,
        /* --- overriding name / message --- */
        r#"var e = new Error('m'); e.name = 'My'; print(e.toString(), e.name, e.message);"#,
        r#"var e = new Error('m'); e.name = ''; print(JSON.stringify(e.toString()));"#,
        r#"var e = new Error('m'); e.message = ''; print(JSON.stringify(e.toString()));"#,
        r#"var e = new Error('m'); e.name = ''; e.message = ''; print(JSON.stringify(e.toString()));"#,
        r#"var e = new Error(); e.name = 'N'; print(JSON.stringify(e.toString()));"#,
        r#"var e = new Error('m'); e.name = 5; print(e.toString());"#,
        r#"var e = new Error('m'); e.message = 5; print(e.toString());"#,
        r#"var e = new Error('m'); delete e.message; print(e.toString(), JSON.stringify(e.message));"#,
        r#"var e = new Error('m'); e.name = {toString:function(){ return 'X'; }}; print(e.toString());"#,
        r#"var e = new Error('m'); e.name = {toString:function(){ throw new Error('inner'); }};
           try{ e.toString(); }catch(x){ print('caught', x.name, x.message); }"#,
        r#"print(Error.prototype.toString.call({}));"#,
        r#"print(Error.prototype.toString.call({name:'A',message:'B'}));"#,
        r#"print(JSON.stringify(Error.prototype.toString.call({name:'',message:''})));"#,
        r#"print(Error.prototype.toString.call({name:'A'}), Error.prototype.toString.call({message:'B'}));"#,
        r#"try{ Error.prototype.toString.call('str'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ Error.prototype.toString.call(null); }catch(e){ print('caught', e.name, e.message); }"#,
        /* --- stack / stackTrace: printed in full, must match byte-for-byte --- */
        r#"var e = new Error('m'); print(typeof e.stack, typeof e.stackTrace);"#,
        r#"var e = new Error('m'); print(e.stack.indexOf('at') >= 0);"#,
        r#"var e = new Error('m'); print(e.stack);"#,
        r#"var e = new Error('m'); print(JSON.stringify(e.stackTrace));"#,
        r#"function f(){ return new Error('inner'); } print(f().stack);"#,
        r#"function f(){ return new Error('inner'); } function g(){ return f(); } print(g().stack);"#,
        r#"function f(){ throw new TypeError('boom'); } try{ f(); }catch(e){ print(e.stack); }"#,
        r#"try{ null.x; }catch(e){ print(e.name, e.message); print(e.stack); }"#,
        r#"try{ undefinedFn(); }catch(e){ print(e.name, e.message); print(e.stack); }"#,
        r#"var e = new Error('m'); print(e.hasOwnProperty('stack'), e.hasOwnProperty('stackTrace'));"#,
        r#"var e = new Error('m'); try{ e.stack = 'x'; }catch(x){ print('caught', x.name); } print(typeof e.stack);"#,
        r#"var e = new Error('m'); print(e.propertyIsEnumerable('message'), e.propertyIsEnumerable('stackTrace'));"#,
        r#"var e = new Error('m'); var k=[]; for (var p in e) k.push(p); print(k.length, k.join(','));"#,
        r#"print(Object.getOwnPropertyNames(new Error('m')).sort().join(','));"#,
        r#"print(JSON.stringify(new Error('m')));"#,
        /* --- throwing non-Error values --- */
        r#"try{ throw 42; }catch(e){ print(typeof e, e, e === 42); }"#,
        r#"try{ throw 'str'; }catch(e){ print(typeof e, e); }"#,
        r#"try{ throw null; }catch(e){ print(typeof e, e, e === null); }"#,
        r#"try{ throw undefined; }catch(e){ print(typeof e, e, e === undefined); }"#,
        r#"try{ throw true; }catch(e){ print(typeof e, e); }"#,
        r#"try{ throw {a:1}; }catch(e){ print(typeof e, e.a, JSON.stringify(e)); }"#,
        r#"try{ throw [1,2]; }catch(e){ print(typeof e, e.length, e.join('-')); }"#,
        r#"try{ throw NaN; }catch(e){ print(e, e !== e); }"#,
        r#"try{ throw function(){}; }catch(e){ print(typeof e); }"#,
        r#"try{ throw /re/g; }catch(e){ print(typeof e, e.source); }"#,
        r#"print('before'); throw 42;"#,
        r#"print('before'); throw 'a string';"#,
        r#"print('before'); throw null;"#,
        r#"print('before'); throw undefined;"#,
        r#"print('before'); throw {a:1};"#,
        r#"print('before'); throw [1,2,3];"#,
        r#"print('before'); throw new Error('reported');"#,
        r#"print('before'); throw new TypeError('reported-t');"#,
        r#"print('before'); throw {toString:function(){ return 'CUSTOM'; }};"#,
        r#"print('before'); throw {toString:function(){ throw 1; }};"#,
        /* --- rethrow / nesting --- */
        r#"try{ try{ throw new Error('a'); }catch(e){ print('inner', e.message); throw e; } }catch(e){ print('outer', e.message); }"#,
        r#"try{ try{ throw new Error('a'); }finally{ print('fin'); } }catch(e){ print('outer', e.message); }"#,
        r#"try{ try{ throw 1; }catch(e){ throw 2; }finally{ print('fin'); } }catch(e){ print('outer', e); }"#,
        r#"function f(){ try{ return 'ret'; }finally{ print('fin'); } } print(f());"#,
        r#"function f(){ try{ throw 1; }catch(e){ return 'c'; }finally{ return 'f'; } } print(f());"#,
        r#"try{ try{ try{ throw new RangeError('deep'); }catch(e){ throw new TypeError(e.message); } }catch(e){ throw new URIError(e.name); } }catch(e){ print(e.name, e.message); }"#,
        r#"var n=0; function f(d){ try{ if (d === 0) throw new Error('bottom'); f(d-1); }catch(e){ n++; throw e; } } try{ f(5); }catch(e){ print(n, e.message); }"#,
        r#"try{ throw new Error('a'); }catch(e){ print(e.message); } print('after');"#,
        r#"var e1 = new Error('keep'); try{ throw e1; }catch(e){ print(e === e1); }"#,
        /* --- errors from getters / toString / JSON / map --- */
        r#"var o = {}; Object.defineProperty(o, 'x', {get:function(){ throw new Error('getter'); }});
           try{ o.x; }catch(e){ print('caught', e.name, e.message); }"#,
        r#"var o = {}; Object.defineProperty(o, 'x', {get:function(){ throw new Error('getter'); }});
           print('before'); o.x;"#,
        r#"var o = {toString:function(){ throw new Error('ts'); }};
           try{ '' + o; }catch(e){ print('caught', e.name, e.message); }"#,
        r#"var o = {valueOf:function(){ throw new Error('vo'); }};
           try{ o * 1; }catch(e){ print('caught', e.name, e.message); }"#,
        r#"var o = {}; Object.defineProperty(o, 'x', {get:function(){ throw new Error('g'); }, enumerable:true});
           try{ JSON.stringify(o); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ [1,2,3].map(function(v){ if (v === 2) throw new Error('map' + v); return v; }); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ [1,2,3].forEach(function(v){ throw new RangeError('fe'); }); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ [3,1,2].sort(function(a,b){ throw new Error('cmp'); }); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ [1,2].reduce(function(){ throw new Error('red'); }); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print('before'); [1,2,3].map(function(v){ throw new Error('map-report'); });"#,
        r#"try{ 'abc'.replace(/b/, function(){ throw new Error('rep'); }); }catch(e){ print('caught', e.name, e.message); }"#,
        /* --- errors the engine itself raises --- */
        r#"try{ null.x; }catch(e){ print(e.name, e.message); }"#,
        r#"try{ undefined.x; }catch(e){ print(e.name, e.message); }"#,
        r#"try{ (void 0)(); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ ({}).nope(); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ nosuchvariable; }catch(e){ print(e.name, e.message); }"#,
        r#"try{ new 5; }catch(e){ print(e.name, e.message); }"#,
        r#"try{ (1)(); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ decodeURI('%'); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ (1).toString(1); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ (1).toFixed(101); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ new Array(-1); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ ({}).hasOwnProperty.call(null, 'x'); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ Object.defineProperty(1, 'x', {}); }catch(e){ print(e.name, e.message); }"#,
        r#"try{ eval('var'); }catch(e){ print(e.name, typeof e.message); }"#,
        r#"try{ eval('1+'); }catch(e){ print(e.name, e instanceof SyntaxError); }"#,
    ]);
}

/* ------------------------------------------------------------------ */
/*  H15 — global functions (c_src/src/jsbuiltin.c)                    */
/* ------------------------------------------------------------------ */

#[test]
fn h15_global_functions() {
    diff_scripts_both_modes(&[
        /* --- parseInt: every radix -1..37 crossed with many strings --- */
        r#"var ss=['0','1','10','-10','+10','0x1f','0X1F','-0x10','  42','42  ','4.9','-0','abc','','z','7f','08','1e3',
                   '9007199254740993','Infinity','-Infinity','  \t\n\r 12','0b11','0o17','ff','FF','zz','1_0','++1','--1',
                   '2147483648','-2147483649','1e400','999999999999999999999999'];
           for (var r=-1;r<=37;r++) { var o=[];
             for (var i=0;i<ss.length;i++) o.push(String(parseInt(ss[i], r)));
             print(r, o.join('|')); }"#,
        r#"print(parseInt('10'), parseInt('10', 0), parseInt('10', undefined), parseInt('10', null), parseInt('10', NaN));"#,
        r#"print(parseInt('10', 2.9), parseInt('10', -0), parseInt('10', 1e10), parseInt('10', '16'));"#,
        r#"print(parseInt(), parseInt(undefined), parseInt(null), parseInt(true), parseInt(0), parseInt(1.5));"#,
        r#"print(parseInt([1,2]), parseInt({}), parseInt(['12']));"#,
        r#"print(parseInt('0x'), parseInt('0x', 16), parseInt('x10', 16), parseInt('-'), parseInt('+'));"#,
        r#"print(parseInt.length, typeof parseInt);"#,
        /* --- parseFloat --- */
        r#"var ss=['0','1','1.5','-1.5','+1.5','.5','5.','1e3','1E-3','1e','1e+','Infinity','+Infinity','-Infinity','InfinityX',
                   'NaN','  1.5  ','abc','','0x10','1.2.3','1,5','-0','1e400','-1e400','1e-400','.','-.','1.7976931348623157e308',
                   '5e-324','2.2250738585072011e-308','9007199254740993'];
           for (var i=0;i<ss.length;i++) print(JSON.stringify(ss[i]), parseFloat(ss[i]));"#,
        r#"print(parseFloat(), parseFloat(undefined), parseFloat(null), parseFloat(true), parseFloat(1.5), parseFloat([2.5]));"#,
        r#"print(1/parseFloat('-0'), parseFloat.length);"#,
        /* --- isNaN / isFinite over every value kind --- */
        r#"var v=[0,-0,1,-1,NaN,Infinity,-Infinity,'','0','1','abc','NaN','Infinity',true,false,null,undefined,[],[1],[1,2],{},function(){},new Number(1),new String('x'),new Boolean(false),/re/];
           for (var i=0;i<v.length;i++) print(i, isNaN(v[i]), isFinite(v[i]));"#,
        r#"print(isNaN(), isFinite(), isNaN.length, isFinite.length);"#,
        r#"try{ isNaN({valueOf:function(){ throw new Error('vo'); }}); }catch(e){ print('caught', e.name, e.message); }"#,
        /* --- encodeURI / encodeURIComponent / decode* --- */
        r##"print(encodeURI('abcXYZ012'), encodeURIComponent('abcXYZ012'));"##,
        r##"print(encodeURI("-_.!~*'()"), encodeURIComponent("-_.!~*'()"));"##,
        r##"print(encodeURI(';/?:@&=+$,#'));"##,
        r##"print(encodeURIComponent(';/?:@&=+$,#'));"##,
        r##"print(encodeURI(' <>"{}|\\^`[]'));"##,
        r##"print(encodeURIComponent(' <>"{}|\\^`[]'));"##,
        r#"var o=[]; for (var i=0;i<128;i++) o.push(encodeURI(String.fromCharCode(i))); print(o.join(''));"#,
        r#"var o=[]; for (var i=0;i<128;i++) o.push(encodeURIComponent(String.fromCharCode(i))); print(o.join(''));"#,
        r#"print(encodeURI('héllo'), encodeURIComponent('héllo'));"#,
        r#"print(encodeURI('日本語'), encodeURIComponent('日本語'));"#,
        r#"print(encodeURI('é中𝄞'));"#,
        r#"print(encodeURIComponent('\ud800'), encodeURIComponent('\udfff'));"#,
        r#"print(encodeURI(''), encodeURIComponent(''), encodeURI(1), encodeURI(null), encodeURI(undefined));"#,
        r#"print(decodeURI('abc'), decodeURIComponent('abc'));"#,
        r#"print(decodeURI('%41%42'), decodeURIComponent('%41%42'));"#,
        r#"print(decodeURI('%3B%2F%3F%3A%40%26%3D%2B%24%2C%23'));"#,
        r#"print(decodeURIComponent('%3B%2F%3F%3A%40%26%3D%2B%24%2C%23'));"#,
        r#"print(decodeURI('%C3%A9'), decodeURIComponent('%C3%A9'));"#,
        r#"print(decodeURIComponent('%E6%97%A5%E6%9C%AC%E8%AA%9E'));"#,
        r#"print(decodeURI('%ed%a0%80').length);"#,
        r#"print(decodeURIComponent('%41%42%43').length, decodeURI('%%41'));"#,
        r#"print(decodeURI('%2f'), decodeURI('%2F'), decodeURIComponent('%2f'));"#,
        r#"try{ decodeURI('%'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ decodeURI('%4'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ decodeURI('%zz'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ decodeURI('%4z'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ decodeURI('a%'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ decodeURIComponent('%E0%A4%A'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print('before'); decodeURI('%');"#,
        r#"var s='a b/c?d#e', a=encodeURI(s), b=encodeURIComponent(s);
           print(a, b, decodeURI(a), decodeURIComponent(b), decodeURI(a) === s, decodeURIComponent(b) === s);"#,
        r#"var s='日本語 é'; print(decodeURIComponent(encodeURIComponent(s)) === s);"#,
        r#"print(encodeURI.length, decodeURI.length, encodeURIComponent.length, decodeURIComponent.length);"#,
        /* --- escape / unescape: not present in this build --- */
        r#"print(typeof escape, typeof unescape, 'escape' in this, 'unescape' in this);"#,
        /* --- eval --- */
        r#"print(eval('1+1'));"#,
        r#"print(eval('"str"'), typeof eval('"str"'));"#,
        r#"print(eval(''), typeof eval(''));"#,
        r#"print(eval('var q = 5; q'), typeof q);"#,
        r#"eval('var w = 7;'); print(typeof w, w);"#,
        r#"print(eval(5), eval(null), eval(undefined), eval(true));"#,
        r#"print(eval('[1,2,3]').length, eval('({a:1})').a);"#,
        r#"print(eval('eval("2+3")'));"#,
        r#"print(eval('eval("eval(\'4*5\')")'));"#,
        r#"var x = 1; function f(){ var x = 2; return eval('x'); } print(f(), x);"#,
        r#"function f(){ eval('var y = 9;'); return typeof y; } print(f(), typeof y);"#,
        r#"print(eval('if (1) 2; else 3;'));"#,
        r#"print(eval('for (var i=0;i<3;i++); i'));"#,
        r#"print(eval('function g(){ return 8; } g()'));"#,
        r#"try{ eval('1+'); }catch(e){ print('caught', e.name); }"#,
        r#"try{ eval('{'); }catch(e){ print('caught', e.name); }"#,
        r#"try{ eval('var 1x = 2;'); }catch(e){ print('caught', e.name); }"#,
        r#"try{ eval('throw new RangeError("in-eval")'); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print('before'); eval('1+');"#,
        r#"print('before'); eval('nosuch()');"#,
        r#"print(eval.length !== undefined || true, typeof eval);"#,
        r#"try{ var f = eval; print(typeof f, f('1+1')); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ print((eval)('1+1')); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ print(this.eval('1+1')); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ print([eval][0]('1+1')); }catch(e){ print('caught', e.name, e.message); }"#,
        /* --- Function constructor --- */
        r#"var f = new Function('return 42;'); print(f(), typeof f, f.length);"#,
        r#"var f = new Function('a', 'return a * 2;'); print(f(21), f.length);"#,
        r#"var f = new Function('a', 'b', 'return a + b;'); print(f(1,2), f.length);"#,
        r#"var f = new Function('a,b', 'return a - b;'); print(f(5,2), f.length);"#,
        r#"var f = new Function(''); print(f(), typeof f());"#,
        r#"var f = Function('return 7;'); print(f());"#,
        r#"var f = new Function(); print(f(), f.length);"#,
        r#"var f = new Function('return this;'); print(typeof f());"#,
        r#"try{ new Function('return'); }catch(e){ print('caught', e.name); }"#,
        r#"try{ new Function('1+'); }catch(e){ print('caught', e.name); }"#,
        r#"try{ new Function('a b', 'return 1;'); }catch(e){ print('caught', e.name); }"#,
        r#"print(typeof Function, Function.length, Function.prototype.length, typeof Function.prototype);"#,
        r#"var f = new Function('a','return a;'); print(f instanceof Function, f.constructor === Function);"#,
        r#"var f = new Function('a','return a;'); print(typeof f.toString(), f.toString().length > 0);"#,
        r#"var f = new Function('return 1;'); var o = new f(); print(typeof o);"#,
        /* --- global NaN / Infinity / undefined --- */
        r#"print(NaN, Infinity, undefined, typeof NaN, typeof Infinity, typeof undefined);"#,
        r#"print(NaN !== NaN, 1/Infinity, -Infinity < 0);"#,
        r#"NaN = 1; print(NaN, NaN !== NaN);"#,
        r#"Infinity = 1; print(Infinity);"#,
        r#"undefined = 1; print(undefined, typeof undefined);"#,
        r#"print(delete NaN, delete Infinity, delete undefined); print(NaN, Infinity, undefined);"#,
        r#"var d = Object.getOwnPropertyDescriptor(this, 'NaN');
           print(d.writable, d.enumerable, d.configurable, d.value);"#,
        r#"var d = Object.getOwnPropertyDescriptor(this, 'Infinity'); print(d.writable, d.enumerable, d.configurable);"#,
        r#"var d = Object.getOwnPropertyDescriptor(this, 'undefined'); print(d.writable, d.enumerable, d.configurable);"#,
        r#"var k=[]; for (var p in this) k.push(p); print(k.length, k.join(','));"#,
        r#"var g = Object.getOwnPropertyNames(this).sort(); print(g.join(','));"#,
        r#"print(this.parseInt === parseInt, this.NaN !== this.NaN, 'NaN' in this);"#,
        r#"print(typeof this, this === this, Object.prototype.toString.call(this));"#,
    ]);
}

/* ------------------------------------------------------------------ */
/*  H16 — enumeration order and prototype chains                      */
/* ------------------------------------------------------------------ */

#[test]
fn h16_enumeration_and_prototypes() {
    diff_scripts_both_modes(&[
        /* --- for-in order over objects built in various ways --- */
        r#"var o = {a:1,b:2,c:3}; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {c:1,b:2,a:3}; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {}; o.c=1; o.a=2; o.b=3; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {}; o.z=1; o.y=2; o.z=3; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {a:1,b:2,c:3}; delete o.b; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {a:1,b:2,c:3}; delete o.b; o.b=9; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {a:1,b:2,c:3}; delete o.a; delete o.c; o.d=4; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {2:'a',1:'b',0:'c'}; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {}; o[2]='a'; o[10]='b'; o[1]='c'; o.x='d'; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {'10':1,'9':2,'a':3,'1':4}; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {'-1':1,'1.5':2,'01':3,'1e2':4,'':5}; var k=[]; for (var p in o) k.push(p); print(k.join('|'));"#,
        r#"var a = [1,2,3]; var k=[]; for (var p in a) k.push(p + ':' + typeof p); print(k.join(','));"#,
        r#"var a = [1,2,3]; a.x='y'; var k=[]; for (var p in a) k.push(p); print(k.join(','));"#,
        r#"var a = [1,2,3]; a[10]=4; var k=[]; for (var p in a) k.push(p); print(k.join(','));"#,
        r#"var a = []; a[5]=1; var k=[]; for (var p in a) k.push(p); print(k.join(','), a.length);"#,
        r#"var a = [1,2,3]; delete a[1]; var k=[]; for (var p in a) k.push(p); print(k.join(','), a.length);"#,
        r#"var a = new Array(3); var k=[]; for (var p in a) k.push(p); print(k.length, a.length);"#,
        r#"var o = {}; for (var i=0;i<10;i++) o['k'+i]=i; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {}; for (var i=9;i>=0;i--) o['k'+i]=i; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = {}; for (var i=0;i<20;i++) o[i]=i; for (var i=0;i<20;i+=2) delete o[i]; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        /* --- prototype chains, 2 and 3 levels, with shadowing --- */
        r#"function A(){} A.prototype.x=1; A.prototype.y=2;
           var a = new A(); a.z=3; var k=[]; for (var p in a) k.push(p); print(k.join(','));"#,
        r#"function A(){} A.prototype.x=1;
           var a = new A(); a.x=9; var k=[]; for (var p in a) k.push(p); print(k.join(','), a.x, A.prototype.x);"#,
        r#"function A(){} function B(){} B.prototype = new A();
           A.prototype.a=1; B.prototype.b=2; var o = new B(); o.c=3;
           var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"function A(){} function B(){} function C(){}
           B.prototype = new A(); C.prototype = new B();
           A.prototype.a=1; B.prototype.b=2; C.prototype.c=3;
           var o = new C(); o.d=4; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"function A(){} function B(){} function C(){}
           B.prototype = new A(); C.prototype = new B();
           A.prototype.v='A'; B.prototype.v='B'; C.prototype.v='C';
           var o = new C(); var k=[]; for (var p in o) k.push(p); print(k.join(','), o.v);"#,
        r#"var base = {a:1,b:2}; var mid = Object.create(base); mid.b=3; mid.c=4;
           var top = Object.create(mid); top.a=5; top.d=6;
           var k=[]; for (var p in top) k.push(p + '=' + top[p]); print(k.join(','));"#,
        r#"var o = Object.create(null); o.a=1; var k=[]; for (var p in o) k.push(p); print(k.join(','));"#,
        r#"var o = Object.create({a:1}); var k=[]; for (var p in o) k.push(p); print(k.join(','), o.hasOwnProperty('a'));"#,
        r#"var o = {}; var k=[]; for (var p in o) k.push(p); print(k.length);"#,
        r#"var k=[]; for (var p in Object.prototype) k.push(p); print(k.length);"#,
        r#"var k=[]; for (var p in Array.prototype) k.push(p); print(k.length);"#,
        r#"Object.prototype.injected = 1; var k=[]; for (var p in {a:1}) k.push(p); print(k.join(',')); delete Object.prototype.injected;"#,
        r#"Object.prototype.injected = 1; var k=[]; for (var p in [1]) k.push(p); print(k.join(','));"#,
        /* --- non-enumerable properties --- */
        r#"var o = {a:1}; Object.defineProperty(o, 'b', {value:2, enumerable:false});
           var k=[]; for (var p in o) k.push(p); print(k.join(','), o.b, Object.keys(o).join(','), Object.getOwnPropertyNames(o).sort().join(','));"#,
        r#"var o = {}; Object.defineProperty(o, 'b', {value:2, enumerable:true});
           var k=[]; for (var p in o) k.push(p); print(k.join(','), Object.keys(o).join(','));"#,
        r#"var o = {}; Object.defineProperty(o, 'g', {get:function(){ return 7; }, enumerable:true});
           var k=[]; for (var p in o) k.push(p + '=' + o[p]); print(k.join(','));"#,
        r#"var o = {}; Object.defineProperty(o, 'g', {get:function(){ return 7; }, enumerable:false});
           var k=[]; for (var p in o) k.push(p); print(k.length, o.g, Object.getOwnPropertyNames(o).join(','));"#,
        r#"var proto = {}; Object.defineProperty(proto, 'h', {value:1, enumerable:false});
           var o = Object.create(proto); o.i=2; var k=[]; for (var p in o) k.push(p); print(k.join(','), o.h);"#,
        r#"var proto = {p:1}; var o = Object.create(proto);
           Object.defineProperty(o, 'p', {value:2, enumerable:false});
           var k=[]; for (var p in o) k.push(p); print(k.length, o.p);"#,
        r#"var o = {a:1,b:2}; print(o.propertyIsEnumerable('a'), o.propertyIsEnumerable('toString'), Object.prototype.propertyIsEnumerable.call(o, 'z'));"#,
        /* --- string objects, functions, arguments --- */
        r#"var s = new String('abc'); var k=[]; for (var p in s) k.push(p); print(k.join(','), s.length);"#,
        r#"var s = new String(''); var k=[]; for (var p in s) k.push(p); print(k.length);"#,
        r#"var k=[]; for (var p in 'abc') k.push(p); print(k.join(','));"#,
        r#"var s = new String('ab'); s.x=1; var k=[]; for (var p in s) k.push(p); print(k.join(','));"#,
        r#"var s = new String('héllo'); var k=[]; for (var p in s) k.push(p); print(k.join(','), s.length);"#,
        r#"function f(a,b){} var k=[]; for (var p in f) k.push(p); print(k.length, f.length, f.name);"#,
        r#"function f(){} f.x=1; f.y=2; var k=[]; for (var p in f) k.push(p); print(k.join(','));"#,
        r#"function f(a,b){ var k=[]; for (var p in arguments) k.push(p); return k.join(','); } print(f(1,2), f(1,2,3), f());"#,
        r#"function f(){ var k=[]; for (var p in arguments) k.push(p + '=' + arguments[p]); return k.join(','); } print(f('x','y'));"#,
        r#"function f(a){ arguments.z = 1; var k=[]; for (var p in arguments) k.push(p); return k.join(','); } print(f(1));"#,
        r#"function f(){ return Object.getOwnPropertyNames(arguments).sort().join(','); } print(f(1,2));"#,
        r#"var k=[]; for (var p in new Number(5)) k.push(p); print(k.length);"#,
        r#"var k=[]; for (var p in new Boolean(true)) k.push(p); print(k.length);"#,
        r#"var k=[]; for (var p in 5) k.push(p); print(k.length);"#,
        r#"var k=[]; for (var p in null) k.push(p); print('null ok', k.length);"#,
        r#"var k=[]; for (var p in undefined) k.push(p); print('undef ok', k.length);"#,
        r#"var k=[]; for (var p in true) k.push(p); print(k.length);"#,
        /* --- mutation during for-in --- */
        r#"var o = {a:1,b:2,c:3,d:4}; var k=[]; for (var p in o) { k.push(p); if (p === 'a') delete o.c; } print(k.join(','));"#,
        r#"var o = {a:1,b:2,c:3}; var k=[]; for (var p in o) { k.push(p); delete o.b; delete o.c; } print(k.join(','));"#,
        r#"var o = {a:1}; var k=[]; for (var p in o) { k.push(p); o.b = 2; } print(k.join(','), Object.keys(o).join(','));"#,
        r#"var o = {a:1,b:2}; var k=[]; for (var p in o) { k.push(p); o['n'+p] = 1; if (k.length > 20) break; } print(k.join(','));"#,
        r#"var o = {a:1,b:2,c:3}; var k=[]; for (var p in o) { k.push(p); o.a = 99; } print(k.join(','), o.a);"#,
        r#"var a=[1,2,3]; var k=[]; for (var p in a) { k.push(p); a.push(9); if (k.length > 20) break; } print(k.join(','), a.length);"#,
        r#"var a=[1,2,3]; var k=[]; for (var p in a) { k.push(p); a.pop(); } print(k.join(','), a.length);"#,
        r#"var o = {a:1,b:2}; var k=[]; for (var p in o) { k.push(p); Object.defineProperty(o, 'z', {value:1, enumerable:true}); if (k.length>10) break; } print(k.join(','));"#,
        /* --- Object.keys vs for-in vs getOwnPropertyNames --- */
        r#"function rep(o){ var k=[]; for (var p in o) k.push(p);
             print('forin:' + k.join(','), 'keys:' + Object.keys(o).join(','), 'own:' + Object.getOwnPropertyNames(o).join(',')); }
           rep({a:1,b:2}); rep([1,2]); rep(Object.create({x:1})); rep(new String('ab'));
           var o={a:1}; Object.defineProperty(o,'h',{value:2}); rep(o);
           function f(a,b){} rep(f);"#,
        r#"var o = {a:1,b:2}; print(Object.keys(o).length === Object.getOwnPropertyNames(o).length);"#,
        r#"var a = [1,2]; print(Object.keys(a).join(','), Object.getOwnPropertyNames(a).join(','));"#,
        r#"var a = []; a[3]=1; print(Object.keys(a).join(','), Object.getOwnPropertyNames(a).join(','));"#,
        r#"print(Object.keys('abc').join(','));"#,
        r#"try{ print(Object.keys(5)); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"try{ print(Object.getOwnPropertyNames(null)); }catch(e){ print('caught', e.name, e.message); }"#,
        r#"print(Object.getOwnPropertyNames(Math).sort().join(','));"#,
        r#"print(Object.keys(Math).length, Object.keys(JSON).length);"#,
        /* --- for-in with a var-less / expression LHS, and labelled control --- */
        r#"var o={a:1,b:2}, k=[], p; for (p in o) k.push(p); print(k.join(','), p);"#,
        r#"var o={a:1,b:2}, t={}, k=[]; for (t.q in o) k.push(t.q); print(k.join(','));"#,
        r#"var o={a:1,b:2}, a=[], k=[]; for (a[0] in o) k.push(a[0]); print(k.join(','));"#,
        r#"var o={a:1,b:2,c:3}, k=[]; outer: for (var p in o) { for (var q in o) { if (q === 'b') continue outer; k.push(p+q); } } print(k.join(','));"#,
        r#"var o={a:1,b:2,c:3}, k=[]; for (var p in o) { if (p === 'b') break; k.push(p); } print(k.join(','));"#,
        r#"var o={a:1,b:2,c:3}, k=[]; for (var p in o) { if (p === 'b') continue; k.push(p); } print(k.join(','));"#,
        r#"function f(o){ for (var p in o) return p; return 'none'; } print(f({z:1}), f({}));"#,
        r#"var o = {a:1,b:2}; var k=[]; with (o) { for (var p in o) k.push(p + a); } print(k.join(','));"#,
    ]);
}

/* ------------------------------------------------------------------ */
/*  H17 — lexing (c_src/src/jslex.c)                                  */
/* ------------------------------------------------------------------ */

#[test]
fn h17_lexing() {
    diff_scripts_both_modes(&[
        /* --- number literals --- */
        r#"print(0, 1, 42, 1000000);"#,
        r#"print(.5, 0.5, 5., 5.0, 0.0, .0);"#,
        r#"print(1e10, 1e-10, 1E10, 1E-10, 1e+10, 1E+10);"#,
        r#"print(1.5e3, .5e3, 5.e3, 1.5E-3);"#,
        r#"print(0x0, 0x1f, 0X1F, 0xFF, 0xff, 0xABCDEF, 0Xabcdef);"#,
        r#"print(0x7fffffff, 0x80000000, 0xffffffff, 0x100000000);"#,
        r#"print(0xfffffffffffffffff);"#,
        r#"print(1e308, 1e309, 1e-308, 1e-323, 1e-324);"#,
        r#"print(1.7976931348623157e308, 5e-324, 9007199254740992, 9007199254740993);"#,
        r#"print(123456789012345678901234567890);"#,
        r#"print(0.1 + 0.2, 1/3, 2/3);"#,
        r#"print(1e21, 1e20, 1e-6, 1e-7);"#,
        r#"print(-0, 1/-0, 0 === -0);"#,
        r#"print(08, 09, 0.8);"#,
        r#"print(00);"#,
        r#"print(01);"#,
        r#"print(0777);"#,
        r#"print(1e);"#,
        r#"print(1e+);"#,
        r#"print(0x);"#,
        r#"print(3abc);"#,
        r#"print(1..toString());"#,
        r#"print((1).toString(), 1 .toString());"#,
        r#"print(1.2.toString());"#,
        /* --- string literals: every escape --- */
        r#"print('a\'b', "a\"b");"#,
        r#"print('a\\b'.length, 'a\\b');"#,
        r#"print('\b'.charCodeAt(0), '\f'.charCodeAt(0), '\n'.charCodeAt(0), '\r'.charCodeAt(0), '\t'.charCodeAt(0), '\v'.charCodeAt(0));"#,
        r#"print('\0'.length, '\0'.charCodeAt(0));"#,
        r#"print('\x41\x42', '\x00'.charCodeAt(0), '\x7f'.charCodeAt(0), '\xff'.charCodeAt(0));"#,
        r#"print('AB', 'é', '中', '\u0000'.charCodeAt(0), '\u0000'.length);"#,
        r#"print('𝄞'.length);"#,
        r#"print('\q', '\q'.length, '\q' === 'q');"#,
        r#"print('\a\c\d\e\g\h\i\j\k\l\m\o\p\s\w\y\z');"#,
        r#"print('\1'.charCodeAt(0), '\8', '\9');"#,
        r#"print("dq\'sq", 'sq\"dq');"#,
        r#"print('single' === "single", 'a' + "b");"#,
        r#"print('héllo', 'héllo'.length, '日本語'.length);"#,
        r#"print('αβγ'.charCodeAt(0), 'αβγ'.charCodeAt(1));"#,
        r#"print('mixed é 日 x'.length);"#,
        r#"print('\x'.length);"#,
        r#"print('\u00'.length);"#,
        r#"print('\xZZ');"#,
        r#"print('\uZZZZ');"#,
        r#"print('unterminated);"#,
        "print('line\\\ncontinued');",
        "print('line\\\ncontinued'.length);",
        "print('a\\\rb'.length);",
        "print(\"a\\\r\nb\".length);",
        /* --- regexp literal vs division --- */
        r#"var a=8,b=2,c=2; print(a/b/c);"#,
        r#"var a=8,b=2; print(a/b, a /b, a/ b, a / b);"#,
        r#"var a; a=/b/g; print(a.source, a.global);"#,
        r#"print((1)/2/3);"#,
        r#"print([4][0]/2);"#,
        r#"var o={x:8}; print(o.x/2);"#,
        r#"function f(){ return 8; } print(f()/2);"#,
        r#"print(typeof /a/, typeof (1/1));"#,
        r#"var a=4; print(a /2/ 1);"#,
        r#"print('x'.replace(/x/,'y'));"#,
        r#"var x=1; x/=2; print(x);"#,
        r#"print(4/2, 4 /2, {}/2);"#,
        r#"if (1) /a/.test('a') && print('regexp after if');"#,
        r#"print(1, /a/.source);"#,
        r#"var r = [/a/, /b/g]; print(r[0].source, r[1].toString());"#,
        r#"print(/[/]/.test('/'));"#,
        r#"print(/a\/b/.source, /a[/]b/.test('a/b'));"#,
        r#"print(/}/.test('}'), /{/.test('{'));"#,
        /* --- comments --- */
        r#"// leading line comment
           print('after line comment');"#,
        r#"print('a'); // trailing
           print('b');"#,
        r#"/* block */ print('after block');"#,
        r#"print(/* inline */ 'inline');"#,
        r#"/* multi
              line
              comment */ print('after multiline');"#,
        r#"print('x'); /* unterminated"#,
        r#"print('x'); // comment at EOF"#,
        r#"print('x'); /* c */"#,
        r#"print(1 /* a */ + /* b */ 2);"#,
        r#"//"#,
        r#"/**/"#,
        r#"/*/ print('tricky'); /**/ print('done');"#,
        r#"print('a')//comment
           print('b')"#,
        r#"var x = 1; /* comment with 'quotes' and "dquotes" and \ backslash */ print(x);"#,
        /* --- line terminators and ASI --- */
        "print(1)\nprint(2)",
        "print(1)\rprint(2)",
        "print(1)\r\nprint(2)",
        "print(1)\u{2028}print(2)",
        "print(1)\u{2029}print(2)",
        "var a = 1\nvar b = 2\nprint(a+b)",
        "function f(){ return\n1 } print(f())",
        "function f(){ return 1 } print(f())",
        "function f(){ return\r1 } print(f())",
        "function f(){ return\u{2028}1 } print(f())",
        "var a=1, b=2\nvar c = a\n+b\nprint(c)",
        "var i=0; loop: for(;;){ i++; if (i>2) break\nloop } print(i)",
        "var x = 1\n++x\nprint(x)",
        "var x=1, y=2\nvar z = x\n++y\nprint(x,y,z)",
        "throw\n1",
        "print(1);;;print(2);",
        "\u{FEFF}print('bom-ish leading whitespace')",
        "print('a')\u{00A0}\u{FEFF}\u{000B}\u{000C}\tprint('b')",
        "var a\u{00A0}=\u{00A0}5; print(a)",
        /* --- identifiers --- */
        r#"var $ = 1, _ = 2, $_ = 3, _1 = 4, a1b2 = 5; print($, _, $_, _1, a1b2);"#,
        r#"var $$$ = 'x'; print($$$);"#,
        r#"var café = 1; print(café);"#,
        r#"var日本 = 1; print(日本);"#,
        r#"var Ωmega = 1; print(Ωmega);"#,
        r#"var abc = 7; print(abc, abc);"#,
        r#"var o = {}; o.a = 1; print(o.a, o['a']);"#,
        r#"print(typeof 1);"#,
        r#"var é = 3; print(é);"#,
        r#"var if = 1; print(typeof this['if']);"#,
        r#"var \u = 1;"#,
        r#"var \u12 = 1;"#,
        r#"var \u123 = 1;"#,
        r#"var \x41 = 1;"#,
        r#"var \n = 1;"#,
        r#"var a b = 1;"#,
        r#"var 1abc = 1;"#,
        /* --- keywords as property names --- */
        r#"var o = {if:1, else:2, for:3, while:4, function:5, return:6, var:7, this:8, new:9, delete:10};
           print(o.if, o.else, o.for, o.while, o.function, o.return, o.var, o.this, o.new, o.delete);"#,
        r#"var o = {class:1, enum:2, extends:3, super:4, const:5, export:6, import:7, implements:8};
           print(o.class, o["class"], o.enum, o.super, o.const, o.import);"#,
        r#"print(({if:1}).if, ({do:2}).do, ({in:3}).in, ({typeof:4}).typeof);"#,
        r#"var o = {null:1, true:2, false:3}; print(o.null, o.true, o.false);"#,
        r#"var o = {}; o.if = 1; o['else'] = 2; print(o.if, o.else, Object.keys(o).join(','));"#,
        r#"var o = {get:1, set:2, of:3, let:4, yield:5, static:6, package:7, private:8, public:9, protected:10, interface:11};
           print(o.get, o.set, o.let, o.yield, o.static, o.private, o.interface);"#,
        r#"var o = {instanceof:1, void:2, with:3, switch:4, case:5, default:6, break:7, continue:8, catch:9, finally:10, throw:11, try:12, debugger:13};
           print(o.instanceof, o.void, o.with, o.switch, o.case, o.default, o.break, o.catch, o.try, o.debugger);"#,
        r#"var if = 1;"#,
        r#"var class = 1;"#,
        r#"print({a:1}.a, {'a':1}.a, {"a":1}.a, {1:'x'}[1], {1.5:'y'}[1.5]);"#,
        /* --- punctuator lexing --- */
        r#"print(1<2, 1>2, 1<=2, 1>=2, 1==2, 1!=2, 1===2, 1!==2);"#,
        r#"print(1<<2, 8>>2, -8>>>28, 5&3, 5|3, 5^3, ~5);"#,
        r#"var x=1; x+=1; x-=2; x*=3; x/=1; x%=5; x<<=2; x>>=1; x&=7; x|=8; x^=1; print(x);"#,
        r#"var x=1; x >>>= 1; print(x);"#,
        r#"print(1 && 2, 0 || 3, !0, !!'', typeof void 0);"#,
        r#"print(1?2:3, (1,2,3));"#,
        r#"print(- -1, + +1, ~ ~1, ! !1);"#,
        r#"print(1 - -1, 1+ +1, 1++ +1);"#,
        r#"print(a=1, a);"#,
        r#"print(2 ** 3);"#,
        r#"print(1 => 2);"#,
        r#"print(3 ... 4);"#,
        r#"print(1 @ 2);"#,
        r#"print(1 # 2);"#,
        r#"print("\u0000abc".length, "\u0000abc".charCodeAt(0));"#,
    ]);
}

/// `\uXXXX` escapes in string literals and in *identifiers*.
///
/// jslex.c `lexescape()` handles the former; `jsY_unescape()` — which jslex.c
/// calls around every identifier start/part character — handles the latter, so
/// `abc` IS a legal spelling of the identifier `abc`. These scripts are
/// assembled from a backslash constant rather than written inline so that the
/// escape text reaches the JS lexer verbatim.
#[test]
fn h17b_lexing_unicode_escapes() {
    let b = "\\"; // one backslash
    let owned: Vec<String> = vec![
        format!("print('{}u0041{}u0042');", b, b),
        format!("print('{}u0041'.length, '{}u0041'.charCodeAt(0));", b, b),
        format!("print('{}u00e9', '{}u00e9'.length);", b, b),
        format!("print('{}u4e2d', '{}u4e2d'.length);", b, b),
        format!("print('{}u0000'.charCodeAt(0), '{}uffff'.charCodeAt(0));", b, b),
        format!("print('{}uFFFF'.length, '{}uffff'.length);", b, b),
        format!("print('{}ud834{}udd1e'.length);", b, b),
        format!("print('{}ud800'.length, '{}udfff'.length);", b, b),
        format!(
            "print('{}u007f'.charCodeAt(0), '{}u0080'.charCodeAt(0), '{}u07ff'.length, '{}u0800'.length);",
            b, b, b, b
        ),
        format!("print('{}u0041{}x42{}103');", b, b, b),
        format!("print('a{}uZZZZb');", b),
        format!("print('a{}u12b');", b),
        format!("print(\"{}u0041\" === 'A');", b),
        format!("print(JSON.stringify('{}u0001'));", b),
        format!("print(/{}u0041/.test('A'), /{}u0041/.source);", b, b),
        // \u escapes in identifiers
        format!("var {}u0061bc = 7; print(abc, {}u0061bc);", b, b),
        format!("var a{}u0062c = 8; print(abc);", b),
        format!("var abc = 9; print({}u0061{}u0062{}u0063);", b, b, b),
        format!("var {}u00e9 = 3; print(é, {}u00e9);", b, b),
        format!("var o = {{}}; o.{}u0061 = 1; print(o.a, o.{}u0061);", b, b),
        format!("var o = {{{}u0061:1}}; print(o.a);", b),
        format!("print(typeo{}u0066 1);", b),
        format!("{}u0069f (1) print('escaped-if');", b),
        format!("var {}u0020 = 1;", b),
        format!("var {}u003$ = 1;", b),
        format!("var a{}u = 1;", b),
        format!("va{}u0072 x = 1; print(x);", b),
        format!("var {}u0024 = 5; print($);", b),
        format!("var {}u005f = 6; print(_);", b),
        format!("var x{}u0030 = 7; print(x0);", b),
    ];
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    diff_scripts_both_modes(&refs);
}

/* ------------------------------------------------------------------ */
/*  H18 — stress / limits (jsparse.c INCREC / JS_ASTLIMIT = 400)      */
/* ------------------------------------------------------------------ */

#[test]
fn h18_stress() {
    let mut owned: Vec<String> = Vec::new();

    /* --- nested object literals --- */
    for depth in [10usize, 50, 100, 200] {
        let mut s = String::from("var o = ");
        for _ in 0..depth {
            s.push_str("{a:");
        }
        s.push('1');
        for _ in 0..depth {
            s.push('}');
        }
        s.push_str("; var n=0, p=o; while (typeof p === 'object') { n++; p = p.a; } print('objdepth', n, p);");
        owned.push(s);
    }

    /* --- nested array literals --- */
    for depth in [10usize, 50, 100, 200] {
        let mut s = String::from("var o = ");
        for _ in 0..depth {
            s.push('[');
        }
        s.push('1');
        for _ in 0..depth {
            s.push(']');
        }
        s.push_str("; var n=0, p=o; while (typeof p === 'object') { n++; p = p[0]; } print('arrdepth', n, p);");
        owned.push(s);
    }

    /* --- 1000-element array literal --- */
    {
        let items: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
        owned.push(format!(
            "var a = [{}]; var s=0; for (var i=0;i<a.length;i++) s+=a[i]; print(a.length, s, a[0], a[999]);",
            items.join(",")
        ));
        let strs: Vec<String> = (0..1000).map(|i| format!("'s{}'", i)).collect();
        owned.push(format!(
            "var a = [{}]; print(a.length, a[0], a[999], a.join('').length);",
            strs.join(",")
        ));
        // 1000-property object literal
        let props: Vec<String> = (0..1000).map(|i| format!("k{}:{}", i, i)).collect();
        owned.push(format!(
            "var o = {{{}}}; var n=0, s=0; for (var p in o) {{ n++; s+=o[p]; }} print(n, s, o.k0, o.k999);",
            props.join(",")
        ));
    }

    /* --- 2000-character string literal --- */
    {
        let body: String = std::iter::repeat("abcdefghij").take(200).collect();
        assert_eq!(body.len(), 2000);
        owned.push(format!(
            "var s = '{}'; var h=0; for (var i=0;i<s.length;i++) h = (h*31 + s.charCodeAt(i)) & 0xffffff; print(s.length, h, s.charAt(0), s.charAt(1999));",
            body
        ));
        let utf: String = std::iter::repeat("héllo日本").take(150).collect();
        owned.push(format!(
            "var s = '{}'; print(s.length);",
            utf
        ));
    }

    /* --- function with 100 parameters --- */
    {
        let ps: Vec<String> = (0..100).map(|i| format!("p{}", i)).collect();
        let args: Vec<String> = (0..100).map(|i| i.to_string()).collect();
        owned.push(format!(
            "function f({}) {{ return p0 + p50 + p99 + arguments.length; }} print(f.length, f({}));",
            ps.join(","),
            args.join(",")
        ));
        owned.push(format!(
            "var f = new Function('{}', 'return p0 + p99;'); print(f.length, f({}));",
            ps.join(","),
            args.join(",")
        ));
    }

    /* --- function with 200 local variables --- */
    {
        let decls: Vec<String> = (0..200).map(|i| format!("var v{} = {};", i, i)).collect();
        owned.push(format!(
            "function f() {{ {} var s=0; s = v0 + v100 + v199; return s; }} print(f());",
            decls.join(" ")
        ));
        let one: Vec<String> = (0..200).map(|i| format!("w{}={}", i, i)).collect();
        owned.push(format!(
            "function f() {{ var {}; return w0 + w199; }} print(f());",
            one.join(",")
        ));
    }

    /* --- nested parentheses around and over JS_ASTLIMIT ---
       Each paren level costs several INCREC() steps in jsparse.c, so the
       actual cut-off is well below 400 parens; both libraries must agree on
       exactly where it is, and on the SyntaxError report when it is crossed. */
    for depth in [1usize, 2, 5, 10, 20, 40, 60, 80, 100, 130, 200, 300, 399, 400, 401, 500] {
        let mut s = String::from("print(");
        for _ in 0..depth {
            s.push('(');
        }
        s.push_str("42");
        for _ in 0..depth {
            s.push(')');
        }
        s.push_str(&format!(", 'parens{}');", depth));
        owned.push(s);
    }
    // same, but caught: the SyntaxError is a compile-time error so it cannot be
    // caught by a try/catch in the same script -- verify that too.
    for depth in [100usize, 500] {
        let mut s = String::from("try{ print(");
        for _ in 0..depth {
            s.push('(');
        }
        s.push_str("7");
        for _ in 0..depth {
            s.push(')');
        }
        s.push_str("); }catch(e){ print('caught', e.name); }");
        owned.push(s);
    }
    // deeply nested unary operators and deeply nested binary additions
    for depth in [50usize, 200, 500] {
        owned.push(format!("print({}1, 'unary{}');", "!".repeat(depth), depth));
        owned.push(format!(
            "print({} 'add{}');",
            std::iter::repeat("1+").take(depth).collect::<String>() + "0,",
            depth
        ));
    }

    /* --- if / else-if chain 200 long --- */
    {
        let mut s = String::from("function f(x){ ");
        for i in 0..200 {
            if i == 0 {
                s.push_str(&format!("if (x === {}) return 'a{}';", i, i));
            } else {
                s.push_str(&format!(" else if (x === {}) return 'a{}';", i, i));
            }
        }
        s.push_str(" else return 'none'; } print(f(0), f(1), f(99), f(199), f(200));");
        owned.push(s);
    }

    /* --- switch with 300 cases --- */
    {
        let mut s = String::from("function f(x){ switch (x) {");
        for i in 0..300 {
            s.push_str(&format!(" case {}: return 'c{}';", i, i));
        }
        s.push_str(" default: return 'def'; } } print(f(0), f(150), f(299), f(300));");
        owned.push(s);
        let mut s2 = String::from("var n=0; function f(x){ switch (x) {");
        for i in 0..300 {
            s2.push_str(&format!(" case {}: n++;", i));
        }
        s2.push_str(" } } f(0); f(298); print(n);");
        owned.push(s2);
    }

    /* --- recursion 500 deep, and deep enough to overflow --- */
    owned.push(
        "function f(n){ return n === 0 ? 0 : 1 + f(n-1); } print(f(500));".to_string(),
    );
    owned.push(
        "function f(n){ if (n === 0) return 'bottom'; return f(n-1); } print(f(900));".to_string(),
    );
    owned.push(
        "var d=0; function f(){ d++; f(); } try{ f(); }catch(e){ print('caught', e.name, e.message, d > 100); }"
            .to_string(),
    );
    owned.push("function f(){ f(); } print('before'); f();".to_string());
    owned.push(
        "var m=0; function f(n){ if (n > m) m = n; try{ f(n+1); }catch(e){ throw e; } } try{ f(0); }catch(e){ print(e.name, e.message, m > 100); }"
            .to_string(),
    );
    owned.push(
        "function fib(n){ return n < 2 ? n : fib(n-1) + fib(n-2); } print(fib(20));".to_string(),
    );
    owned.push(
        "function even(n){ return n === 0 ? true : odd(n-1); } function odd(n){ return n === 0 ? false : even(n-1); } print(even(300), odd(301));"
            .to_string(),
    );

    /* --- 100000-iteration loop --- */
    owned.push(
        "var s=0; for (var i=0;i<100000;i++) s = (s + i) % 1000003; print(s);".to_string(),
    );
    owned.push(
        "var s=0, i=0; while (i < 100000) { s += i & 7; i++; } print(s, i);".to_string(),
    );
    owned.push(
        "var s=0, i=100000; do { s ^= i; i--; } while (i > 0); print(s);".to_string(),
    );
    owned.push(
        "var a=[]; for (var i=0;i<20000;i++) a.push(i%13); var s=0; for (var i=0;i<a.length;i++) s+=a[i]; print(a.length, s);"
            .to_string(),
    );
    owned.push(
        "var o={}; for (var i=0;i<5000;i++) o['k'+i]=i; var n=0; for (var p in o) n++; print(n, o.k4999);"
            .to_string(),
    );

    /* --- build a 64KB string in a loop; print length + checksum only --- */
    owned.push(
        "var s=''; while (s.length < 65536) s += 'abcdefgh';
         var h=0; for (var i=0;i<s.length;i+=7) h = (h*33 + s.charCodeAt(i)) & 0xffffff;
         print(s.length, h);"
            .to_string(),
    );
    owned.push(
        "var s='x'; for (var i=0;i<17;i++) s = s + s;
         var h=0; for (var i=0;i<s.length;i+=1024) h = (h + s.charCodeAt(i)) & 0xffff;
         print(s.length, h, s.charAt(0), s.charAt(s.length-1));"
            .to_string(),
    );
    owned.push(
        "var parts=[]; for (var i=0;i<8192;i++) parts.push('abcdefgh');
         var s = parts.join(''); print(s.length, s.charCodeAt(65535));"
            .to_string(),
    );

    /* --- other limits --- */
    owned.push(
        "var a=[]; for (var i=0;i<2000;i++) a[i]=i; print(a.length, a[1999], a.join(',').length);"
            .to_string(),
    );
    owned.push("var a=[]; a[1000000]=1; print(a.length, a[1000000], a[0]);".to_string());
    owned.push(
        "var n=0; try{ (function g(){ n++; return [g(), g()]; })(); }catch(e){ print(e.name, n > 10); }"
            .to_string(),
    );
    owned.push(
        "var s=''; for (var i=0;i<300;i++) s += '('; s += '1'; for (var i=0;i<300;i++) s += ')';
         try{ print(eval(s)); }catch(e){ print('caught', e.name); }"
            .to_string(),
    );
    owned.push(
        "var s=''; for (var i=0;i<40;i++) s += '('; s += '1'; for (var i=0;i<40;i++) s += ')';
         try{ print(eval(s)); }catch(e){ print('caught', e.name); }"
            .to_string(),
    );

    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    diff_scripts_both_modes(&refs);
}

/* ------------------------------------------------------------------ */
/*  H19 — `this` binding and the `arguments` object                    */
/* ------------------------------------------------------------------ */

/// Scripts shared by the sloppy and the strict run: the *outputs* differ
/// between the two modes (primitive `this` is boxed in sloppy mode and left
/// alone in strict mode; `arguments` aliases parameters only in sloppy mode),
/// which is precisely why each is run in both.
const H19_SCRIPTS: &[&str] = &[
    /* --- `this` at top level --- */
    r#"print(typeof this, this === undefined, Object.prototype.toString.call(this));"#,
    r#"print(this.print === print);"#,
    r#"var tl = this; function f(){ return this; } print(f() === tl, typeof f());"#,
    /* --- plain function call --- */
    r#"function f(){ return typeof this; } print(f());"#,
    r#"function f(){ return this === undefined; } print(f());"#,
    r#"function f(){ return this; } print(String(f() === this));"#,
    r#"function f(){ return (function(){ return typeof this; })(); } print(f());"#,
    /* --- method call --- */
    r#"var o = {n:'o', f:function(){ return this.n; }}; print(o.f());"#,
    r#"var o = {f:function(){ return this === o; }}; print(o.f());"#,
    r#"var o = {f:function(){ return typeof this; }}; var g = o.f; print(o.f(), g());"#,
    r#"var o = {a:{b:{f:function(){ return typeof this.z; }}}}; o.a.b.z=1; print(o.a.b.f());"#,
    r#"var a = [function(){ return this.length; }]; print(a[0]());"#,
    /* --- constructor --- */
    r#"function C(){ this.x = 1; print(typeof this, this instanceof C); } new C();"#,
    r#"function C(){ this.x = 1; } var c = new C(); print(c.x, c instanceof C);"#,
    r#"function C(){ return 5; } var c = new C(); print(typeof c, c.constructor === C);"#,
    r#"function C(){ this.x = 1; return {y:2}; } var c = new C(); print(c.x, c.y);"#,
    r#"function C(){ this.x = 1; return 'prim'; } var c = new C(); print(c.x);"#,
    r#"function C(){ print(typeof this); } C();"#,
    /* --- call / apply / bind with every kind of thisArg --- */
    r#"function f(){ return typeof this + ':' + String(this); }
       print(f.call(undefined)); print(f.call(null)); print(f.call(5)); print(f.call('s'));
       print(f.call(true)); print(f.call({a:1})); print(f.call([1,2]));"#,
    r#"function f(){ return typeof this + ':' + String(this); }
       print(f.apply(undefined)); print(f.apply(null)); print(f.apply(5)); print(f.apply('s'));
       print(f.apply(true)); print(f.apply({a:1}));"#,
    r#"function f(){ return typeof this; }
       print(f.bind(undefined)(), f.bind(null)(), f.bind(5)(), f.bind('s')(), f.bind(true)(), f.bind({})());"#,
    r#"function f(){ return this instanceof Number; } print(f.call(5), f.call(new Number(5)));"#,
    r#"function f(){ return this instanceof String; } print(f.call('s'));"#,
    r#"function f(){ return this instanceof Boolean; } print(f.call(false));"#,
    r#"function f(){ return this === null; } print(f.call(null));"#,
    r#"function f(){ return this === undefined; } print(f.call(), f.call(undefined));"#,
    r#"function f(){ return this === 5; } print(f.call(5));"#,
    r#"function f(){ return this + 0; } print(f.call(5), f.call('5'), f.call(true));"#,
    r#"function f(){ try{ return this.valueOf(); }catch(e){ return 'E:' + e.name; } } print(f.call(5), f.call('x'), f.call(null));"#,
    r#"function f(){ return typeof this.charAt; } print(f.call('abc'));"#,
    r#"function f(a,b){ return [typeof this, a, b].join('|'); } print(f.call(1,2,3), f.apply(1,[2,3]), f.bind(1,2)(3));"#,
    r#"function f(){ return arguments.length + ':' + typeof this; } print(f.apply(null), f.apply(null, []), f.apply(null, [1,2,3]));"#,
    r#"function f(){ return arguments.length; } print(f.apply(null, {length:2, 0:'a', 1:'b'}));"#,
    r#"function f(){ return typeof this; } try{ print(f.apply(null, 5)); }catch(e){ print('caught', e.name); }"#,
    r#"var o = {n:1}; function f(){ return this.n; } var b = f.bind(o); print(b(), b.call({n:2}), b.apply({n:3}));"#,
    r#"function f(a,b,c){ return [a,b,c].join(','); } print(f.bind(null,1).length, f.bind(null,1,2)(3), f.length);"#,
    r#"function C(a){ this.a = a; } var B = C.bind(null, 7); var o = new B(); print(o.a, o instanceof C);"#,
    r#"var o = {f:function(){ return typeof this; }}; print(o.f.call(), o.f.apply());"#,
    r#"print(Object.prototype.toString.call(5), Object.prototype.toString.call('s'), Object.prototype.toString.call(null), Object.prototype.toString.call(undefined));"#,
    r#"print([].concat.call('ab').length);"#,
    r#"function f(){ this.x = 1; } var t = {}; f.call(t); print(t.x);"#,
    r#"function f(){ this.x = 1; } try{ f.call(5); print('ok'); }catch(e){ print('caught', e.name); }"#,
    r#"function f(){ this.x = 1; } try{ f.call(null); print('ok', typeof x); }catch(e){ print('caught', e.name); }"#,
    r#"function f(){ return this; } print(typeof f.call('s'), f.call('s') === 's');"#,
    /* --- arguments aliasing --- */
    r#"function f(a){ arguments[0] = 9; return a; } print(f(1));"#,
    r#"function f(a){ a = 9; return arguments[0]; } print(f(1));"#,
    r#"function f(a,b){ arguments[1] = 'B'; return a + '/' + b; } print(f('a','b'));"#,
    r#"function f(a,b){ b = 'B'; return arguments[0] + '/' + arguments[1]; } print(f('a','b'));"#,
    r#"function f(a){ arguments[0] = 9; return [a, arguments[0], arguments.length].join('|'); } print(f(1), f(), f(1,2));"#,
    r#"function f(a){ delete arguments[0]; return [a, arguments[0], arguments.length].join('|'); } print(f(1));"#,
    r#"function f(a){ arguments.length = 0; return [a, arguments.length].join('|'); } print(f(1));"#,
    r#"function f(a){ arguments[3] = 'x'; return [arguments.length, arguments[3], a].join('|'); } print(f(1));"#,
    r#"function f(a){ var args = arguments; a = 2; return args[0]; } print(f(1));"#,
    r#"function f(a){ return (function(){ return arguments.length; })(); } print(f(1,2,3));"#,
    /* --- arguments.callee, length, and fn.length --- */
    r#"function f(a,b,c){ return arguments.length + '/' + f.length; } print(f(), f(1), f(1,2,3,4,5));"#,
    r#"function f(){ return arguments.callee === f; } print(f());"#,
    r#"function f(n){ return n === 0 ? 0 : 1 + arguments.callee(n-1); } print(f(5));"#,
    r#"print((function(){ return typeof arguments.callee; })());"#,
    r#"print((function(a,b){ return arguments.callee.length; })(1));"#,
    r#"function f(){ return typeof arguments; } print(f(), Object.prototype.toString.call((function(){ return arguments; })()));"#,
    r#"function f(){ return arguments instanceof Array, Array.isArray(arguments); } print(f());"#,
    r#"function f(){ return Array.prototype.join.call(arguments, '-'); } print(f(1,2,3));"#,
    r#"function f(){ return Array.prototype.slice.call(arguments).join('+'); } print(f('a','b'));"#,
    r#"function f(){ arguments.callee.q = 1; return f.q; } print(f());"#,
    r#"function f(a){ var arguments = 5; return arguments; } print(f(1));"#,
    r#"function f(a){ return typeof arguments; var arguments; } print(f(1));"#,
    r#"function f(){ arguments = 7; return arguments; } print(f(1));"#,
    r#"function f(a){ return arguments[0] === a; } print(f(1), f(), f(1,2));"#,
    r#"function f(){ return arguments[-1] + '|' + arguments['0'] + '|' + arguments[0.0]; } print(f('z'));"#,
    r#"var o = {f:function(){ return arguments.length; }}; print(o.f(1,2), o.f.call(null,1,2,3), o.f.apply(null,[1]));"#,
    r#"function outer(){ function inner(){ return arguments.length; } return inner(1,2) + '/' + arguments.length; } print(outer(9));"#,
    r#"function f(a){ if (a) return arguments.length; return f(1,2,3); } print(f(0));"#,
    r#"function f(){ return arguments.toString(); } print(f());"#,
    r#"function f(){ var k=[]; for (var p in arguments) k.push(p); return k.join(','); } print(f(1,2,3));"#,
    r#"function f(){ return JSON.stringify(arguments); } print(f(1,'a'));"#,
    r#"function f(a,a){ return a; } print(f(1,2));"#,
    r#"function f(a,a){ return arguments[0] + '/' + arguments[1]; } print(f(1,2));"#,
];

#[test]
fn h19_this_and_arguments() {
    diff_scripts(0, H19_SCRIPTS);
    diff_scripts(JS_STRICT, H19_SCRIPTS);
    // A few scripts that only make sense with an explicit "use strict"
    // directive in sloppy-mode source (so the two modes see the same text).
    diff_scripts_both_modes(&[
        r#"function f(){ 'use strict'; return typeof this; } print(f(), f.call(5), f.call(null));"#,
        r#"function f(){ 'use strict'; return this === undefined; } print(f());"#,
        r#"function f(a){ 'use strict'; arguments[0] = 9; return a; } print(f(1));"#,
        r#"function f(a){ 'use strict'; a = 9; return arguments[0]; } print(f(1));"#,
        r#"'use strict'; function f(){ return typeof this; } print(f(), f.call(5));"#,
        r#"'use strict'; function f(a){ a = 2; return arguments[0]; } print(f(1));"#,
        r#"'use strict'; print(typeof this);"#,
        r#"function f(){ 'use strict'; return arguments.callee === f; } try{ print(f()); }catch(e){ print('caught', e.name); }"#,
        r#"function f(){ return (function(){ 'use strict'; return typeof this; })(); } print(f());"#,
        r#"function f(){ 'use strict'; return (function(){ return typeof this; })(); } print(f());"#,
    ]);
}

/* ------------------------------------------------------------------ */
/*  H20 — randomized expression fuzz (fixed seed)                      */
/* ------------------------------------------------------------------ */

const FUZZ_VARS: &[&str] = &["v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9"];
const FUZZ_PRELUDE: &str = "var v0=1,v1=-2,v2=0,v3=3.5,v4='7',v5='xy',v6=NaN,v7=Infinity,v8=true,v9=null;";
const FUZZ_NUMS: &[&str] = &[
    "0", "1", "-1", "2", "3", "0.5", "-0.5", "10", "255", "1e3", "1e-3", "0x10", "2147483647",
    "-2147483648", "4294967295", "9007199254740993", "1e21", "1e-7", "NaN", "Infinity",
    "-Infinity", "0.1",
];
const FUZZ_STRS: &[&str] = &[
    "''", "'a'", "'0'", "'1'", "'10'", "'abc'", "' 5 '", "'1e3'", "'-3'", "'0x10'", "'NaN'",
    "'true'",
];
const FUZZ_BINOPS: &[&str] = &[
    "+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", ">>>", "<", ">", "<=", ">=", "==", "!=",
    "===", "!==", "&&", "||",
];
const FUZZ_UNOPS: &[&str] = &["-", "+", "~", "!"];

fn fuzz_leaf(rng: &mut Rng) -> String {
    match rng.below(4) {
        0 => FUZZ_NUMS[rng.below(FUZZ_NUMS.len() as u32) as usize].to_string(),
        1 => FUZZ_STRS[rng.below(FUZZ_STRS.len() as u32) as usize].to_string(),
        _ => FUZZ_VARS[rng.below(FUZZ_VARS.len() as u32) as usize].to_string(),
    }
}

fn fuzz_expr(rng: &mut Rng, depth: u32) -> String {
    if depth == 0 {
        return fuzz_leaf(rng);
    }
    match rng.below(11) {
        0 | 1 => fuzz_leaf(rng),
        2 | 3 | 4 | 5 => format!(
            "({} {} {})",
            fuzz_expr(rng, depth - 1),
            FUZZ_BINOPS[rng.below(FUZZ_BINOPS.len() as u32) as usize],
            fuzz_expr(rng, depth - 1)
        ),
        6 => format!(
            "({}({}))",
            FUZZ_UNOPS[rng.below(FUZZ_UNOPS.len() as u32) as usize],
            fuzz_expr(rng, depth - 1)
        ),
        7 => format!(
            "(({}) ? ({}) : ({}))",
            fuzz_expr(rng, depth - 1),
            fuzz_expr(rng, depth - 1),
            fuzz_expr(rng, depth - 1)
        ),
        8 => format!("(({}).toString())", fuzz_expr(rng, depth - 1)),
        9 => format!("(({}).charAt(1))", fuzz_expr(rng, depth - 1)),
        _ => format!("(Math.floor({}))", fuzz_expr(rng, depth - 1)),
    }
}

#[test]
fn h20_expression_fuzz() {
    let mut rng = Rng::new(0x5EED_1234);
    let mut scripts: Vec<String> = Vec::with_capacity(4000);
    for i in 0..4000 {
        let depth = 1 + (i % 4) as u32;
        let e = fuzz_expr(&mut rng, depth);
        scripts.push(format!(
            "{}try{{print({})}}catch(e){{print('E',e.name)}}",
            FUZZ_PRELUDE, e
        ));
    }
    let refs: Vec<&str> = scripts.iter().map(|s| s.as_str()).collect();
    diff_scripts(0, &refs);
}

#[test]
fn h20_expression_fuzz_strict() {
    // Same grammar, different seed stream, under JS_STRICT.
    let mut rng = Rng::new(0x5EED_1234);
    let mut scripts: Vec<String> = Vec::with_capacity(1200);
    for i in 0..1200 {
        let depth = 2 + (i % 3) as u32;
        let e = fuzz_expr(&mut rng, depth);
        scripts.push(format!(
            "{}try{{print({})}}catch(e){{print('E',e.name)}}",
            FUZZ_PRELUDE, e
        ));
    }
    let refs: Vec<&str> = scripts.iter().map(|s| s.as_str()).collect();
    diff_scripts(JS_STRICT, &refs);
}
