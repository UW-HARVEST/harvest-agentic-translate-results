| # | function | trigger (exact invalid input/condition) | expected C result |
|---|---|---|---|
| 1 | O_getPrototypeOf (jsobject.c:111) | `Object.getPrototypeOf(1)` — arg 1 is not an object | TypeError: not an object |
| 2 | O_getOwnPropertyDescriptor (jsobject.c:124) | `Object.getOwnPropertyDescriptor(1, "x")` | TypeError: not an object |
| 3 | O_getOwnPropertyNames (jsobject.c:175) | `Object.getOwnPropertyNames("s")` | TypeError: not an object |
| 4 | ToPropertyDescriptor, get branch (jsobject.c:258) | `Object.defineProperty({}, "x", {value:1, get:function(){}})` | TypeError: value/writable and get/set attributes are exclusive |
| 5 | ToPropertyDescriptor, set branch (jsobject.c:265) | `Object.defineProperty({}, "x", {writable:true, set:function(v){}})` | TypeError: value/writable and get/set attributes are exclusive |
| 6 | O_defineProperty target check (jsobject.c:277) | `Object.defineProperty(1, "x", {})` | TypeError: not an object |
| 7 | O_defineProperty descriptor check (jsobject.c:278) | `Object.defineProperty({}, "x", 1)` | TypeError: not an object |
| 8 | O_defineProperties_walk (jsobject.c:289) | `Object.defineProperties({}, {x:1})` — enumerable prop whose value is not an object | TypeError: not an object |
| 9 | O_defineProperties_imp (jsobject.c:304) | `Object.defineProperties({}, 1)` (also `Object.create({}, 1)`) | TypeError: not an object |
| 10 | O_defineProperties (jsobject.c:326) | `Object.defineProperties(1, {})` | TypeError: not an object |
| 11 | O_create (jsobject.c:342) | `Object.create(1)` — proto neither object nor null | TypeError: not an object or null |
| 12 | O_keys (jsobject.c:371) | `Object.keys(1)` | TypeError: not an object |
| 13 | O_preventExtensions (jsobject.c:402) | `Object.preventExtensions(1)` | TypeError: not an object |
| 14 | O_isExtensible (jsobject.c:412) | `Object.isExtensible(1)` | TypeError: not an object |
| 15 | O_seal (jsobject.c:430) | `Object.seal(1)` | TypeError: not an object |
| 16 | O_isSealed (jsobject.c:460) | `Object.isSealed(1)` | TypeError: not an object |
| 17 | O_freeze (jsobject.c:488) | `Object.freeze(1)` | TypeError: not an object |
| 18 | O_isFrozen (jsobject.c:520) | `Object.isFrozen(1)` | TypeError: not an object |
| 19 | Op_hasOwnProperty, js_toobject(J,0) (jsobject.c:61) | `Object.prototype.hasOwnProperty.call(null, "x")` | TypeError: cannot convert null to object |
| 20 | Op_isPrototypeOf, js_toobject(J,0) (jsobject.c:86) | `Object.prototype.isPrototypeOf.call(undefined, {})` | TypeError: cannot convert undefined to object |
| 21 | Op_propertyIsEnumerable, js_toobject(J,0) (jsobject.c:102) | `Object.prototype.propertyIsEnumerable.call(null, "x")` | TypeError: cannot convert null to object |
| 22 | jsB_new_Array -> length setter (jsarray.c:31) | `new Array(-1)` | RangeError: invalid array length |
| 23 | jsB_new_Array -> length setter (jsarray.c:31) | `new Array(1.5)` (also `new Array(NaN)`, `new Array(1e10)`) | RangeError: invalid array length |
| 24 | jsB_new_Array -> length setter (jsarray.c:31) | `new Array(100000000)` — newlen > JS_ARRAYLIMIT (1<<26) | RangeError: array too large |
| 25 | Ap_join_cycle (jsarray.c:69-96) | `var a = []; a[0] = a; a.join()` — self-referential array reached again through join | no error; pushes "" and returns |
| 26 | Ap_join (jsarray.c:148) | `var a=[],i; for(i=0;i<300000;i++) a[i]=new Array(1001).join("x"); a.join("")` — n+seplen+rlen > JS_STRLIMIT (1<<28) | RangeError: invalid string length |
| 27 | Ap_sort (jsarray.c:439) | `[3,1,2].sort(1)` — comparator neither callable nor undefined | TypeError: comparison function must be a function or undefined |
| 28 | Ap_sort (jsarray.c:442) | `Array.prototype.sort.call({length:1e300, 0:2, 1:1}, function(a,b){return a-b})` — len clamps to INT_MAX | RangeError: array is too large to sort |
| 29 | Ap_toString (jsarray.c:536) | `Array.prototype.toString.call(null)` | TypeError: 'this' is not an object |
| 30 | Ap_every (jsarray.c:603) | `[1].every(1)` | TypeError: callback is not a function |
| 31 | Ap_some (jsarray.c:632) | `[1].some(null)` | TypeError: callback is not a function |
| 32 | Ap_forEach (jsarray.c:661) | `[1].forEach(1)` | TypeError: callback is not a function |
| 33 | Ap_map (jsarray.c:688) | `[1].map(1)` | TypeError: callback is not a function |
| 34 | Ap_filter (jsarray.c:717) | `[1].filter(1)` | TypeError: callback is not a function |
| 35 | Ap_reduce (jsarray.c:750) | `[1].reduce(1)` | TypeError: callback is not a function |
| 36 | Ap_reduce (jsarray.c:756) | `[].reduce(function(a,b){return a})` — len 0 and no initial value | TypeError: no initial value |
| 37 | Ap_reduce (jsarray.c:767) | `new Array(3).reduce(function(a,b){return a})` — all elements are holes | TypeError: no initial value |
| 38 | Ap_reduceRight (jsarray.c:791) | `[1].reduceRight(1)` | TypeError: callback is not a function |
| 39 | Ap_reduceRight (jsarray.c:797) | `[].reduceRight(function(a,b){return a})` | TypeError: no initial value |
| 40 | Ap_reduceRight (jsarray.c:808) | `new Array(3).reduceRight(function(a,b){return a})` | TypeError: no initial value |
| 41 | js_getlength -> js_getproperty -> js_toobject (jsarray.c:10) | `Array.prototype.pop.call(null)` | TypeError: cannot convert null to object |
| 42 | jsB_Function -> jsP_parsefunction (jsfunction.c:31) | `Function("@")` (also bad param list `Function("a b", "")`) | SyntaxError: [string]:1: unexpected character: '@' |
| 43 | Fp_toString (jsfunction.c:52) | `Function.prototype.toString.call({})` | TypeError: not a function |
| 44 | Fp_toString, js_toobject(J,0) (jsfunction.c:48) | `Function.prototype.toString.call(null)` | TypeError: cannot convert null to object |
| 45 | Fp_apply (jsfunction.c:99) | `Function.prototype.apply.call({}, null, [])` | TypeError: not a function |
| 46 | Fp_apply (jsfunction.c:109) | `(function(){}).apply(null, {length:-1})` — negative arraylike length | no error; n clamped to 0, zero args passed |
| 47 | Fp_call (jsfunction.c:122) | `Function.prototype.call.call({})` | TypeError: not a function |
| 48 | Fp_bind (jsfunction.c:185) | `Function.prototype.bind.call({})` | TypeError: not a function |
| 49 | callbound (jsfunction.c:144) | invoking a bound function whose `__BoundArguments__` length reads negative | no error; n clamped to 0 |
| 50 | constructbound (jsfunction.c:168) | `new` on a bound function whose `__BoundArguments__` length reads negative | no error; n clamped to 0 |
| 51 | Bp_toString (jsboolean.c:16) | `Boolean.prototype.toString.call(1)` | TypeError: not a boolean |
| 52 | Bp_valueOf (jsboolean.c:23) | `Boolean.prototype.valueOf.call("x")` | TypeError: not a boolean |
| 53 | Bp_toString, js_toobject(J,0) (jsboolean.c:15) | `Boolean.prototype.toString.call(null)` | TypeError: cannot convert null to object |
| 54 | Np_valueOf (jsnumber.c:22) | `Number.prototype.valueOf.call("1")` | TypeError: not a number |
| 55 | Np_toString (jsnumber.c:32) | `Number.prototype.toString.call("1")` | TypeError: not a number |
| 56 | Np_toString (jsnumber.c:39) | `(255).toString(1)` (also `(255).toString(0)`) — radix < 2 | RangeError: invalid radix |
| 57 | Np_toString (jsnumber.c:39) | `(255).toString(37)` — radix > 36 | RangeError: invalid radix |
| 58 | Np_toFixed (jsnumber.c:134) | `Number.prototype.toFixed.call("1", 2)` | TypeError: not a number |
| 59 | Np_toFixed (jsnumber.c:135) | `(5).toFixed(-1)` | RangeError: precision -1 out of range |
| 60 | Np_toFixed (jsnumber.c:136) | `(5).toFixed(101)` | RangeError: precision 101 out of range |
| 61 | Np_toExponential (jsnumber.c:150) | `Number.prototype.toExponential.call({}, 2)` | TypeError: not a number |
| 62 | Np_toExponential (jsnumber.c:151) | `(5).toExponential(-1)` | RangeError: precision -1 out of range |
| 63 | Np_toExponential (jsnumber.c:152) | `(5).toExponential(21)` | RangeError: precision 21 out of range |
| 64 | Np_toPrecision (jsnumber.c:166) | `Number.prototype.toPrecision.call({}, 2)` | TypeError: not a number |
| 65 | Np_toPrecision (jsnumber.c:167) | `(5).toPrecision(0)` — width < 1 | RangeError: precision 0 out of range |
| 66 | Np_toPrecision (jsnumber.c:168) | `(5).toPrecision(22)` — width > 21 | RangeError: precision 22 out of range |
| 67 | Np_toFixed, js_toobject(J,0) (jsnumber.c:130) | `Number.prototype.toFixed.call(null, 2)` | TypeError: cannot convert null to object |
| 68 | js_doregexec (jsstring.c:8) | `new Array(6000).join("a").search(/a*b/)` — regexp match recursion depth > REG_MAXREC (4096) | Error: regexec failed |
| 69 | checkstring (jsstring.c:15) | `String.prototype.charAt.call(null, 0)` (any Sp_* via checkstring, e.g. `String.prototype.trim.call(undefined)`) | TypeError: string function called on null or undefined |
| 70 | Sp_toString (jsstring.c:108) | `String.prototype.toString.call(1)` | TypeError: not a string |
| 71 | Sp_valueOf (jsstring.c:115) | `String.prototype.valueOf.call(1)` | TypeError: not a string |
| 72 | Sp_toString, js_toobject(J,0) (jsstring.c:107) | `String.prototype.toString.call(null)` | TypeError: cannot convert null to object |
| 73 | Sp_concat, initial size check (jsstring.c:162) | `s.concat("x")` where `s.length + 1 > JS_STRLIMIT` (1<<28) | RangeError: invalid string length |
| 74 | Sp_concat, per-argument check (jsstring.c:170) | `"".concat(big, big, big, ...)` so accumulated n > JS_STRLIMIT (1<<28) | RangeError: invalid string length |
| 75 | js_runeat / Sp_charCodeAt (jsstring.c:27, 138) | `"abc".charCodeAt(5)` (also `"abc".charCodeAt(-1)`) — index past end / negative | no error; pushes NaN (js_runeat returns EOF) |
| 76 | js_runeat / Sp_charAt (jsstring.c:27, 125) | `"abc".charAt(5)` (also `"abc".charAt(-1)`) | no error; pushes "" |
| 77 | S_fromCharCode -> runetochar (jsstring.c:459) | `String.fromCharCode(0x110000)` — code point > Runemax | no error; emits U+FFFD (Runeerror) |
| 78 | Sp_split_regexp (jsstring.c:732) | `"abc".split(/b/, 0)` — limit 0 | no error; returns empty array |
| 79 | Sp_split_string (jsstring.c:787) | `"abc".split("b", 0)` — limit 0 | no error; returns empty array |
| 80 | Sp_replace_regexp `$n` handling (jsstring.c:601-611) | `"abc".replace(/(a)/, "$9")` — capture index >= m.nsub | no error; emits literal "$9" |
| 81 | Sp_match -> js_newregexp (jsstring.c:484) | `"a".match("(")` — non-regexp argument that is an invalid pattern | SyntaxError: regular expression: unmatched ( |
| 82 | Sp_search -> js_newregexp (jsstring.c:533) | `"a".search("(")` | SyntaxError: regular expression: unmatched ( |
| 83 | js_todate (jsdate.c:365) | `Date.prototype.getTime.call({})` | TypeError: not a date |
| 84 | js_todate, js_toobject(J,idx) (jsdate.c:364) | `Date.prototype.getTime.call(null)` | TypeError: cannot convert null to object |
| 85 | js_setdate (jsdate.c:373) | `Date.prototype.setTime.call({}, 0)` | TypeError: not a date |
| 86 | Dp_toISOString (jsdate.c:484) | `new Date(NaN).toISOString()` | RangeError: invalid date |
| 87 | Dp_toJSON (jsdate.c:792) | `Date.prototype.toJSON.call({toISOString:1})` | TypeError: this.toISOString is not a function |
| 88 | Dp_toJSON (jsdate.c:785) | `new Date(NaN).toJSON()` — primitive value not finite | no error; returns null |
| 89 | TimeClip (jsdate.c:230) | `new Date(Infinity).getTime()` — !isfinite(t) | no error; NaN (invalid date) |
| 90 | TimeClip (jsdate.c:232) | `new Date(8.64e15 + 1).getTime()` — fabs(t) > 8.64e15 | no error; NaN (invalid date) |
| 91 | MakeDay (jsdate.c:214) | `new Date(2000, NaN).getTime()` — month index outside 0..11 after pmod | no error; NAN propagates to invalid date |
| 92 | toint (jsdate.c:243) | `Date.parse("20x0")` — non-digit inside fixed-width field | returns 0 (failure) to caller, which yields NaN |
| 93 | parseDateTime (jsdate.c:259) | `Date.parse("abcd")` — 4-digit year missing | no error; NaN |
| 94 | parseDateTime (jsdate.c:262) | `Date.parse("2000-x")` — 2-digit month missing after '-' | no error; NaN |
| 95 | parseDateTime (jsdate.c:265) | `Date.parse("2000-01-x")` — 2-digit day missing after second '-' | no error; NaN |
| 96 | parseDateTime (jsdate.c:271) | `Date.parse("2000-01-01Tx")` — 2-digit hour missing after 'T' | no error; NaN |
| 97 | parseDateTime (jsdate.c:272) | `Date.parse("2000-01-01T12x")` — ':' expected after hour | no error; NaN |
| 98 | parseDateTime (jsdate.c:274) | `Date.parse("2000-01-01T12:x")` — 2-digit minute missing | no error; NaN |
| 99 | parseDateTime (jsdate.c:277) | `Date.parse("2000-01-01T12:00:x")` — 2-digit second missing after ':' | no error; NaN |
| 100 | parseDateTime (jsdate.c:280) | `Date.parse("2000-01-01T12:00:00.x")` — 3-digit millisecond missing after '.' | no error; NaN |
| 101 | parseDateTime (jsdate.c:290) | `Date.parse("2000-01-01T12:00+x")` — 2-digit tz hour missing | no error; NaN |
| 102 | parseDateTime (jsdate.c:293) | `Date.parse("2000-01-01T12:00+01:x")` — 2-digit tz minute missing | no error; NaN |
| 103 | parseDateTime (jsdate.c:295) | `Date.parse("2000-01-01T12:00+24:00")` — tzh > 23 (or tzm > 59) | no error; NaN |
| 104 | parseDateTime (jsdate.c:302) | `Date.parse("2000-01-01T12:00Zx")` (also `Date.parse("2000-01-01x")`) — trailing garbage | no error; NaN |
| 105 | parseDateTime (jsdate.c:304) | `Date.parse("2000-13-01")` — month outside 1..12 (also "2000-00-01") | no error; NaN |
| 106 | parseDateTime (jsdate.c:305) | `Date.parse("2000-01-32")` — day outside 1..31 (also "2000-01-00") | no error; NaN |
| 107 | parseDateTime (jsdate.c:306) | `Date.parse("2000-01-01T25:00")` — hour > 24 | no error; NaN |
| 108 | parseDateTime (jsdate.c:307) | `Date.parse("2000-01-01T12:60")` — minute > 59 | no error; NaN |
| 109 | parseDateTime (jsdate.c:308) | `Date.parse("2000-01-01T12:00:60")` — second > 59 | no error; NaN |
| 110 | parseDateTime (jsdate.c:309) | ms outside 0..999 — unreachable, toint reads exactly 3 digits so max is 999 | dead check; would return NaN |
| 111 | parseDateTime (jsdate.c:310) | `Date.parse("2000-01-01T24:01")` — H==24 with nonzero M/S/ms | no error; NaN |
| 112 | fmtdate (jsdate.c:324) | `new Date(NaN).toDateString()` | no error; returns the string "Invalid Date" |
| 113 | fmttime (jsdate.c:338) | `new Date(NaN).toTimeString()` | no error; returns the string "Invalid Date" |
| 114 | fmtdatetime (jsdate.c:352) | `new Date(NaN).toString()` (also `Date.prototype.toUTCString`) | no error; returns the string "Invalid Date" |
| 115 | Dp_getFullYear (jsdate.c:492) | `new Date(NaN).getFullYear()` | no error; pushes NaN |
| 116 | Dp_getMonth (jsdate.c:501) | `new Date(NaN).getMonth()` | no error; pushes NaN |
| 117 | Dp_getDate (jsdate.c:510) | `new Date(NaN).getDate()` | no error; pushes NaN |
| 118 | Dp_getDay (jsdate.c:519) | `new Date(NaN).getDay()` | no error; pushes NaN |
| 119 | Dp_getHours (jsdate.c:528) | `new Date(NaN).getHours()` | no error; pushes NaN |
| 120 | Dp_getMinutes (jsdate.c:537) | `new Date(NaN).getMinutes()` | no error; pushes NaN |
| 121 | Dp_getSeconds (jsdate.c:546) | `new Date(NaN).getSeconds()` | no error; pushes NaN |
| 122 | Dp_getMilliseconds (jsdate.c:555) | `new Date(NaN).getMilliseconds()` | no error; pushes NaN |
| 123 | Dp_getUTCFullYear (jsdate.c:564) | `new Date(NaN).getUTCFullYear()` | no error; pushes NaN |
| 124 | Dp_getUTCMonth (jsdate.c:573) | `new Date(NaN).getUTCMonth()` | no error; pushes NaN |
| 125 | Dp_getUTCDate (jsdate.c:582) | `new Date(NaN).getUTCDate()` | no error; pushes NaN |
| 126 | Dp_getUTCDay (jsdate.c:591) | `new Date(NaN).getUTCDay()` | no error; pushes NaN |
| 127 | Dp_getUTCHours (jsdate.c:600) | `new Date(NaN).getUTCHours()` | no error; pushes NaN |
| 128 | Dp_getUTCMinutes (jsdate.c:609) | `new Date(NaN).getUTCMinutes()` | no error; pushes NaN |
| 129 | Dp_getUTCSeconds (jsdate.c:618) | `new Date(NaN).getUTCSeconds()` | no error; pushes NaN |
| 130 | Dp_getUTCMilliseconds (jsdate.c:627) | `new Date(NaN).getUTCMilliseconds()` | no error; pushes NaN |
| 131 | Dp_getTimezoneOffset (jsdate.c:636) | `new Date(NaN).getTimezoneOffset()` | no error; pushes NaN |
| 132 | jsM_round (jsmath.c:14) | `Math.round(NaN)` | no error; returns NaN unchanged |
| 133 | jsM_round (jsmath.c:15) | `Math.round(Infinity)` (also `Math.round(-Infinity)`) | no error; returns Infinity unchanged |
| 134 | Math_pow (jsmath.c:78) | `Math.pow(-1, Infinity)` (also `Math.pow(1, NaN)` via !isfinite(y)) | no error; pushes NaN instead of C pow() result |
| 135 | Math_max (jsmath.c:130) | `Math.max(1, NaN, 3)` | no error; loop breaks, pushes NaN |
| 136 | Math_min (jsmath.c:148) | `Math.min(1, NaN, 3)` | no error; loop breaks, pushes NaN |
| 137 | Math_max (jsmath.c:127) | `Math.max()` — no arguments | no error; pushes -Infinity |
| 138 | Math_min (jsmath.c:145) | `Math.min()` — no arguments | no error; pushes Infinity |
| 139 | jsonexpect (json.c:41) | `JSON.parse("[1")` — token mismatch | SyntaxError: JSON: unexpected token: (end-of-file) (expected ']') |
| 140 | jsonexpect (json.c:41) | `JSON.parse("{\"a\" 1}")` — ':' expected | SyntaxError: JSON: unexpected token: (number) (expected ':') |
| 141 | jsonvalue, object key check (json.c:67) | `JSON.parse("{1:2}")` (also `JSON.parse("{")`) | SyntaxError: JSON: unexpected token: (number) (expected string) |
| 142 | jsonvalue, default case (json.c:107) | `JSON.parse("")` (also `JSON.parse("[")`) | SyntaxError: JSON: unexpected token: (end-of-file) |
| 143 | JSON_parse -> jsY_lexjson (json.c:162-163) | `JSON.parse("'a'")` — character not legal in JSON | SyntaxError: JSON:1: unexpected character: ''' |
| 144 | jsonvalue recursion (json.c:45-109) | `JSON.parse(new Array(100000).join("[") + new Array(100000).join("]"))` — no depth limit on recursive descent | no check; C stack overflow / crash |
| 145 | fmtobject cycle check (json.c:261) | `var a = {}; a.a = a; JSON.stringify(a)` | TypeError: cyclic object value |
| 146 | fmtarray cycle check (json.c:297) | `var a = []; a[0] = a; JSON.stringify(a)` | TypeError: cyclic object value |
| 147 | fmtvalue (json.c:359-360) | `JSON.stringify({a: undefined, b: function(){}})` — value is undefined/callable | returns 0; property omitted, result "{}" |
| 148 | JSON_stringify (json.c:402-403) | `JSON.stringify(undefined)` (also `JSON.stringify(function(){})`) | no error; pushes undefined |
| 149 | JSON_stringify gap (json.c:380) | `JSON.stringify({a:1}, null, -5)` — n < 0 | no error; n clamped to 0, no indentation |
| 150 | JSON_stringify gap (json.c:381) | `JSON.stringify({a:1}, null, 100)` — n > 10 | no error; n clamped to 10 spaces |
| 151 | JSON_stringify gap (json.c:388) | `JSON.stringify({a:1}, null, "aaaaaaaaaaaaaaaaaaaa")` — string longer than 10 | no error; truncated to first 10 chars |
| 152 | filterprop (json.c:239-247) | `JSON.stringify({a:1,b:2}, ["a"])` — key absent from property list | returns 0; key skipped, result `{"a":1}` |
| 153 | jsB_parseInt (jsbuiltin.c:52) | `parseInt("10", 1)` (also `parseInt("10", 37)`) — radix outside 2..36 | no error; pushes NaN |
| 154 | jsB_parseInt (jsbuiltin.c:57) | `parseInt("abc")` — no digits consumed (s == e) | no error; pushes NaN |
| 155 | jsB_parseFloat (jsbuiltin.c:78) | `parseFloat("abc")` — no characters consumed (e == s) | no error; pushes NaN |
| 156 | Decode (jsbuiltin.c:144-145) | `decodeURI("%")` (also `decodeURI("%A")`, `decodeURIComponent("%")`) | URIError: truncated escape sequence |
| 157 | Decode (jsbuiltin.c:148-149) | `decodeURI("%zz")` (also `decodeURIComponent("%g0")`) | URIError: invalid escape sequence |
| 158 | reprobject cycle check (jsrepr.c:85-91) | `var a = {}; a.a = a; a` at the REPL / `js_repr` on a self-referential object | no error; emits "{}" for the repeated object |
| 159 | reprarray cycle check (jsrepr.c:115-121) | `var a = []; a[0] = a; a` at the REPL / `js_repr` on a self-referential array | no error; emits "[]" for the repeated array |
| 160 | js_repr (jsrepr.c:262) | `js_repr` where nothing was buffered (sb == NULL) | no error; pushes "undefined" |
| 161 | js_tryrepr (jsrepr.c:278-281) | `js_tryrepr` on a value whose repr throws (e.g. getter that throws, or OOM in js_putc) | catches, pops, returns the caller-supplied `error` placeholder string |
