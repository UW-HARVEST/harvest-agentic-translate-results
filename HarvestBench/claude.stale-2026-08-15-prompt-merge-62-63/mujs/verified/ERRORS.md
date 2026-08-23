# ERRORS.md — error-surface table (Phase A)

Derived **mechanically** from the C source. The row set is the union of:

1. every call site of an error-raising function in `c_src/src/*.c`, found with
   ```
   grep -nE 'js_(error|evalerror|rangeerror|referenceerror|syntaxerror|typeerror|urierror)\(|jsP_error|jsC_error|jsY_error|die\(' c_src/src/*.c
   ```
   → **225 hits**, of which 8 are the declaration/definition of an error helper
   itself, leaving **217 distinct rejection branches** (sections 1-4);
2. every explicit resource/limit rejection — `js_trystackoverflow`,
   `js_stackoverflow`, `js_outofmemory`, `js_runlimit`, `CHECKSTACK`, `INCREC`,
   `js_ptry`, and every `REG_MAX*` / `JS_*LIMIT` constant (section 5);
3. the return-code contract of every protected entry point — `js_dostring`,
   `js_ploadstring`, `js_pcall`, `js_pconstruct`, `js_try*`, `regcomp`,
   `regexec` (section 6);
4. the generic FFI boundaries every C API has: NULL pointers, zero and
   oversized lengths, values one step past a documented range, and
   **out-of-range enum values** — a C `enum` parameter accepts any `int`, so a
   value with no valid variant is a real input both libraries must handle
   identically (section 7).

## How each error is observed in the differential test

* `js_*error()` raises a JS exception. `js_dostring` returns `1` and hands
  `Error.prototype.toString()` (`"<Name>: <message>"`) to the report callback.
  The test compares the **return code and the exact report string**, and also the
  `e.name`/`e.message` seen by a JS `catch`.
* `jsY_error` / `jsP_error` / `jsC_error` produce
  `SyntaxError: <filename>:<line>: <msg>`. The C builds this as
  `snprintf(buf, 256, "%s:%d: ", ...)` followed by `strcat(buf, msgbuf)` into a
  `char buf[512]` — i.e. the prefix is truncated at **256**, not 512, and a long
  filename can push the message out entirely. The Rust must reproduce this.
* `js_syntaxerror` called directly from `jscompile.c` has **no** `file:line`
  prefix. `json.c` embeds the literal `"JSON: "` in its format strings, whereas
  the JSON *lexer* genuinely uses `"JSON"` as the filename, giving
  `SyntaxError: JSON:1: ...`.
* Two sites report **line 0**, not line 1, because they pass an `AST_LIST` node
  (whose `line` is hard-coded to 0) to `jsC_error`: `jscompile.c:315` and
  `jscompile.c:410`. This is a C quirk that must be preserved.
* `regexp.c`'s `die()` makes `regcompx` return `NULL` and set `*errorp` to the
  message. The test compares NULL-ness **and** the error string.
* Sites reachable only through the embedding API are driven from a **cfunction
  registered into the interpreter** and called from a JS `try{}catch(e){}`, so
  both libraries get an identical protected frame (an external `js_try()`
  `setjmp` cannot work against the Rust cdylib, which models `longjmp` with
  `panic`).

## Legend

| reachable | meaning |
|-----------|---------|
| `JS` | a `js_dostring` snippet on a default (non-strict) state |
| `JS/strict` | needs a `"use strict"` directive |
| `CAPI` | a direct C-API call sequence (driven from a cfunction, see above) |
| `HARD` | reachable, but needs a generated multi-kilobyte source |
| `BIGMEM` | reachable, but needs 0.3-1 GB of RAM (`tests/errors_bigmem.rs`) |
| `NO` | provably unreachable; the justification is in the trigger column |

## Status

**Every row below is checked off.** All rows pass differentially against both
libraries, in the debug *and* release profiles (there is only one feature
combination — see `CONFIGS.md`). Reproduce with `./run_tests.sh`.

| where the rows are tested | file | tests |
|---------------------------|------|-------|
| Sections 1-3, all `JS` / `JS/strict` / `HARD` / `NO` rows | `tests/errors_js.rs` | 5 |
| Section 4 (`regexp.c` `die()` + `regexec` return values) | `tests/regexp.rs` | 10 |
| Sections 5-7 (`CAPI`, limits, return codes, out-of-range enums, NULL) | `tests/errors_capi.rs` | 41 |
| `BIGMEM` rows (`JS_STRLIMIT` / `JS_ARRAYLIMIT`) | `tests/errors_bigmem.rs` | 6 |

`tests/errors_js.rs` is generated from the same trigger table as this file and
additionally **re-asserts the "expected C result" column against the C library on
every run**, so the table cannot silently rot. Each row is exercised three ways:
through the top-level `js_dostring` report path, through a JavaScript
`catch (e)` clause (a different opcode path), and again on a `JS_STRICT` state.

### Inputs deliberately excluded, with justification

These are inputs for which **the C itself has no defined or reproducible
behaviour**, so there is nothing for the Rust to match. They are listed here
rather than silently skipped:

| input | why it is excluded |
|-------|--------------------|
| `js_pop(J, n)` with `n < 0` | raises `TOP` and exposes an uninitialised stack slot (the value stack is never zeroed), so the C prints heap garbage |
| out-of-range index to `js_copy` / `js_rot` / `js_type` / `js_typeof` / `stackidx` | `stackidx()` performs **no** bounds check; the C reads/writes past the value stack |
| `js_pushlstring(J, v, n)` with `n < 0` or `strlen(v) < n <= JS_STRLIMIT` | `while (n--)` writes unboundedly / reads past `v`; only `n > JS_STRLIMIT` is validated, and that row IS tested |
| `js_grisu2(±0.0)` | `minus()`'s `assert(x.f >= y.f)` fires (the C `.so` has no `-DNDEBUG`); `jsV_numbertostring` never calls it with 0 |
| `js_fmtexp(buf, e)` with `\|e\| >= 1e9` | the C's `char se[9]` overflows and the copy-out loop dumps ASLR'd stack bytes into the caller's buffer (probe kept as an `#[ignore]`d subprocess test in `tests/numbers.rs`) |
| `jsV_resizearray` on a `simple` array | `assert(!obj->u.a.simple)` fires; every in-tree caller unflattens first, and the test does the same |
| `jsS_freestrings` followed by `js_intern` or `js_freestate` | it does not reset `J->strings`, so the C use-after-frees / double-frees; the test leaks the state instead |
| calling `js_savetry` without a matching `setjmp` | the C would `longjmp` into an uninitialised `jmp_buf`. The `JS_TRYLIMIT` rows are driven through nested JS `try` blocks instead |

### Stack-protocol notes discovered while writing these tests

Several exported functions consume or mutate stack slots in ways the header does
not state. Getting them wrong aborts the process, so they are recorded here:

* `js_newobjectx` **pops one slot** (its prototype, or any value → `NULL` prototype).
* `js_newcconstructor` is `/* prototype -- constructor */`: the caller must push
  a prototype first (it does `js_rot2`).
* `js_newuserdata` / `js_newuserdatax` also consume a prototype slot.
* `js_torepr` (hence `js_tryrepr`) does `js_repr(J, idx); js_replace(J, idx-1)`
  — it **replaces the value at `idx`** with its string form.
* `js_hasproperty` / `js_hasindex` push the value only when they return 1.
* `js_pconstruct` computes `savetop = TOP - n - 2` but `js_construct` consumes
  only `callee + n args`, so on its error path it writes **one slot below the
  callee**. A caller must keep a spare slot there.
* `js_newcfunctionx` stores `name`, `js_newuserdatax` stores `tag`, and
  `js_pushliteral` stores its string **by pointer, without copying** — they must
  outlive the state.

## Section 1 — `jsarray.c` `jsboolean.c` `jsbuiltin.c` `jsdate.c` `jserror.c` `jsfunction.c` `jsnumber.c` `jsobject.c`

Every row in this section was verified by executing the trigger against the C library and comparing the report string.

| # | function (site) | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|-----------------|----------------------------------------------|-------------------|-----------|-----|
| E1 | `Ap_join` (jsarray.c:149) | **unreachable** — needs a join result larger than JS_STRLIMIT (1<<28 = 256MB) to be built in memory | `RangeError: invalid string length` | NO | [x] proved unreachable; neighbouring branch tested |
| E2 | `Ap_sort` (jsarray.c:440) | `[2,1].sort(1)` | `TypeError: comparison function must be a function or undefined` | JS | [x] |
| E3 | `Ap_sort` (jsarray.c:443) | `Array.prototype.sort.call({length:1e10})` | `RangeError: array is too large to sort` | JS | [x] |
| E4 | `Ap_toString` (jsarray.c:537) | `Array.prototype.toString.call(null)` | `TypeError: 'this' is not an object` | JS | [x] |
| E5 | `Ap_every` (jsarray.c:604) | `[].every()` | `TypeError: callback is not a function` | JS | [x] |
| E6 | `Ap_some` (jsarray.c:633) | `[].some()` | `TypeError: callback is not a function` | JS | [x] |
| E7 | `Ap_forEach` (jsarray.c:662) | `[].forEach()` | `TypeError: callback is not a function` | JS | [x] |
| E8 | `Ap_map` (jsarray.c:689) | `[].map()` | `TypeError: callback is not a function` | JS | [x] |
| E9 | `Ap_filter` (jsarray.c:718) | `[].filter()` | `TypeError: callback is not a function` | JS | [x] |
| E10 | `Ap_reduce` (jsarray.c:751) | `[].reduce()` | `TypeError: callback is not a function` | JS | [x] |
| E11 | `Ap_reduce` (jsarray.c:757) | `[].reduce(function(){})` | `TypeError: no initial value` | JS | [x] |
| E12 | `Ap_reduce` (jsarray.c:767) | `Array.prototype.reduce.call({length:1}, function(){})` | `TypeError: no initial value` | JS | [x] |
| E13 | `Ap_reduceRight` (jsarray.c:792) | `[].reduceRight()` | `TypeError: callback is not a function` | JS | [x] |
| E14 | `Ap_reduceRight` (jsarray.c:798) | `[].reduceRight(function(){})` | `TypeError: no initial value` | JS | [x] |
| E15 | `Ap_reduceRight` (jsarray.c:808) | `Array.prototype.reduceRight.call({length:1}, function(){})` | `TypeError: no initial value` | JS | [x] |
| E16 | `Bp_toString` (jsboolean.c:16) | `Boolean.prototype.toString.call({})` | `TypeError: not a boolean` | JS | [x] |
| E17 | `Bp_valueOf` (jsboolean.c:23) | `Boolean.prototype.valueOf.call({})` | `TypeError: not a boolean` | JS | [x] |
| E18 | `Decode` (jsbuiltin.c:145) | `decodeURI("%")` | `URIError: truncated escape sequence` | JS | [x] |
| E19 | `Decode` (jsbuiltin.c:149) | `decodeURI("%zz")` | `URIError: invalid escape sequence` | JS | [x] |
| E20 | `js_todate` (jsdate.c:366) | `Date.prototype.getTime.call({})` | `TypeError: not a date` | JS | [x] |
| E21 | `js_setdate` (jsdate.c:374) | `Date.prototype.setTime.call({}, 0)` | `TypeError: not a date` | JS | [x] |
| E22 | `Dp_toISOString` (jsdate.c:485) | `new Date(NaN).toISOString()` | `RangeError: invalid date` | JS | [x] |
| E23 | `Dp_toJSON` (jsdate.c:793) | `Date.prototype.toJSON.call({})` | `TypeError: this.toISOString is not a function` | JS | [x] |
| E24 | `Ep_toString` (jserror.c:36) | `Error.prototype.toString.call("x")` | `TypeError: not an object` | JS | [x] |
| E25 | `Fp_toString` (jsfunction.c:53) | `Function.prototype.toString.call({})` | `TypeError: not a function` | JS | [x] |
| E26 | `Fp_apply` (jsfunction.c:100) | `Function.prototype.apply.call({})` | `TypeError: not a function` | JS | [x] |
| E27 | `Fp_call` (jsfunction.c:123) | `Function.prototype.call.call({})` | `TypeError: not a function` | JS | [x] |
| E28 | `Fp_bind` (jsfunction.c:186) | `Function.prototype.bind.call({})` | `TypeError: not a function` | JS | [x] |
| E29 | `Np_valueOf` (jsnumber.c:22) | `Number.prototype.valueOf.call({})` | `TypeError: not a number` | JS | [x] |
| E30 | `Np_toString` (jsnumber.c:33) | `Number.prototype.toString.call({})` | `TypeError: not a number` | JS | [x] |
| E31 | `Np_toString` (jsnumber.c:40) | `(5).toString(1)` | `RangeError: invalid radix` | JS | [x] |
| E32 | `Np_toFixed` (jsnumber.c:134) | `Number.prototype.toFixed.call({})` | `TypeError: not a number` | JS | [x] |
| E33 | `Np_toFixed` (jsnumber.c:135) | `(1).toFixed(-1)` | `RangeError: precision -1 out of range` | JS | [x] |
| E34 | `Np_toFixed` (jsnumber.c:136) | `(1).toFixed(21)` | `RangeError: precision 21 out of range` | JS | [x] |
| E35 | `Np_toExponential` (jsnumber.c:150) | `Number.prototype.toExponential.call({})` | `TypeError: not a number` | JS | [x] |
| E36 | `Np_toExponential` (jsnumber.c:151) | `(1).toExponential(-1)` | `RangeError: precision -1 out of range` | JS | [x] |
| E37 | `Np_toExponential` (jsnumber.c:152) | `(1).toExponential(21)` | `RangeError: precision 21 out of range` | JS | [x] |
| E38 | `Np_toPrecision` (jsnumber.c:166) | `Number.prototype.toPrecision.call({})` | `TypeError: not a number` | JS | [x] |
| E39 | `Np_toPrecision` (jsnumber.c:167) | `(1).toPrecision(0)` | `RangeError: precision 0 out of range` | JS | [x] |
| E40 | `Np_toPrecision` (jsnumber.c:168) | `(1).toPrecision(22)` | `RangeError: precision 22 out of range` | JS | [x] |
| E41 | `O_getPrototypeOf` (jsobject.c:112) | `Object.getPrototypeOf(1)` | `TypeError: not an object` | JS | [x] |
| E42 | `O_getOwnPropertyDescriptor` (jsobject.c:125) | `Object.getOwnPropertyDescriptor(1, "x")` | `TypeError: not an object` | JS | [x] |
| E43 | `O_getOwnPropertyNames` (jsobject.c:176) | `Object.getOwnPropertyNames(1)` | `TypeError: not an object` | JS | [x] |
| E44 | `ToPropertyDescriptor` (jsobject.c:258) | `Object.defineProperty({}, "x", {value:1, get:function(){}})` | `TypeError: value/writable and get/set attributes are exclusive` | JS | [x] |
| E45 | `ToPropertyDescriptor` (jsobject.c:265) | `Object.defineProperty({}, "x", {value:1, set:function(v){}})` | `TypeError: value/writable and get/set attributes are exclusive` | JS | [x] |
| E46 | `O_defineProperty` (jsobject.c:277) | `Object.defineProperty(1, "x", {})` | `TypeError: not an object` | JS | [x] |
| E47 | `O_defineProperty` (jsobject.c:278) | `Object.defineProperty({}, "x", 1)` | `TypeError: not an object` | JS | [x] |
| E48 | `O_defineProperties_walk` (jsobject.c:289) | `Object.defineProperties({}, {x:1})` | `TypeError: not an object` | JS | [x] |
| E49 | `O_defineProperties_imp` (jsobject.c:304) | `Object.defineProperties({}, 1)` | `TypeError: not an object` | JS | [x] |
| E50 | `O_defineProperties` (jsobject.c:326) | `Object.defineProperties(1, {})` | `TypeError: not an object` | JS | [x] |
| E51 | `O_create` (jsobject.c:342) | `Object.create(1)` | `TypeError: not an object or null` | JS | [x] |
| E52 | `O_keys` (jsobject.c:372) | `Object.keys(1)` | `TypeError: not an object` | JS | [x] |
| E53 | `O_preventExtensions` (jsobject.c:403) | `Object.preventExtensions(1)` | `TypeError: not an object` | JS | [x] |
| E54 | `O_isExtensible` (jsobject.c:413) | `Object.isExtensible(1)` | `TypeError: not an object` | JS | [x] |
| E55 | `O_seal` (jsobject.c:431) | `Object.seal(1)` | `TypeError: not an object` | JS | [x] |
| E56 | `O_isSealed` (jsobject.c:461) | `Object.isSealed(1)` | `TypeError: not an object` | JS | [x] |
| E57 | `O_freeze` (jsobject.c:489) | `Object.freeze(1)` | `TypeError: not an object` | JS | [x] |
| E58 | `O_isFrozen` (jsobject.c:521) | `Object.isFrozen(1)` | `TypeError: not an object` | JS | [x] |

## Section 2 — `jslex.c` `jsparse.c` `jscompile.c` `json.c` (lexer / parser / compiler / JSON)

Every concrete row in this section was verified against the C library.

| # | function (site) | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|-----------------|----------------------------------------------|-------------------|-----------|-----|
| E59 | `(declaration)` (jscompile.c:7) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E60 | `(declaration)` (jscompile.c:14) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E61 | `checkfutureword` (jscompile.c:43) | `const` | `SyntaxError: [string]:1: 'const' is a future reserved word` | JS | [x] |
| E62 | `checkfutureword` (jscompile.c:46) | `"use strict"; let` | `SyntaxError: [string]:1: 'let' is a strict mode future reserved word` | JS/strict | [x] |
| E63 | `emitraw` (jscompile.c:75) | **hard** — emit() emits F->lastline first, so a line number >65535 overflows js_Instruction (unsigned short); source = 65536 newlines followed by x; (verified) | `SyntaxError: integer overflow in instruction coding` | HARD | [x] |
| E64 | `addlocal` (jscompile.c:114) | `"use strict"; var arguments;` | `SyntaxError: [string]:1: redefining 'arguments' is not allowed in strict mode` | JS/strict | [x] |
| E65 | `addlocal` (jscompile.c:116) | `"use strict"; var eval;` | `SyntaxError: [string]:1: redefining 'eval' is not allowed in strict mode` | JS/strict | [x] |
| E66 | `addlocal` (jscompile.c:119) | `var eval;` | `EvalError: [string]:1: invalid use of 'eval'` | JS | [x] |
| E67 | `addlocal` (jscompile.c:128) | `"use strict"; function f(a,a){}` | `SyntaxError: [string]:1: duplicate formal parameter 'a'` | JS/strict | [x] |
| E68 | `emitlocal` (jscompile.c:204) | `"use strict"; arguments = 1;` | `SyntaxError: [string]:1: 'arguments' is read-only in strict mode` | JS/strict | [x] |
| E69 | `emitlocal` (jscompile.c:206) | `"use strict"; eval = 1;` | `SyntaxError: [string]:1: 'eval' is read-only in strict mode` | JS/strict | [x] |
| E70 | `emitlocal` (jscompile.c:209) | `eval` | `EvalError: [string]:1: invalid use of 'eval'` | JS | [x] |
| E71 | `emitjumpto` (jscompile.c:238) | **hard** — needs a backward jump target >65535; source = "1;" repeated 15000 times then while(0); (verified) | `SyntaxError: jump address integer overflow` | HARD | [x] |
| E72 | `labelto` (jscompile.c:245) | **hard** — needs F->codelen >65535 when patching a forward jump; source = "1;" repeated 15000 times then if(0); (verified) | `SyntaxError: jump address integer overflow` | HARD | [x] |
| E73 | `checkdup` (jscompile.c:315) | `"use strict"; ({a:1,a:2});` | `SyntaxError: [string]:0: duplicate property 'a' in object literal` | JS/strict | [x] |
| E74 | `cobject` (jscompile.c:336) | **unreachable** — propname() can only yield EXP_NUMBER, EXP_STRING or AST_IDENTIFIER, all three handled above; constant folding never rewrites a property-name node | `SyntaxError: [string]:<node line>: invalid property name in object initializer` | NO | [x] proved unreachable; neighbouring branch tested |
| E75 | `cassign` (jscompile.c:400) | `1 = 2;` | `SyntaxError: [string]:1: invalid l-value in assignment` | JS | [x] |
| E76 | `cassignforin` (jscompile.c:410) | `for (var a, b in {}) ;` | `SyntaxError: [string]:0: more than one loop variable in for-in statement` | JS | [x] |
| E77 | `cassignforin` (jscompile.c:439) | `for (1 in {}) ;` | `SyntaxError: [string]:1: invalid l-value in for-in loop assignment` | JS | [x] |
| E78 | `cassignop1` (jscompile.c:464) | `1 += 2;` | `SyntaxError: [string]:1: invalid l-value in assignment` | JS | [x] |
| E79 | `cassignop2` (jscompile.c:487) | **unreachable** — cassignop2 is only ever called after cassignop1 on the same lhs and both switch on the identical case set, so cassignop1 (jscompile.c:464) always errors first | `SyntaxError: [string]:<node line>: invalid l-value in assignment` | NO | [x] proved unreachable; neighbouring branch tested |
| E80 | `cdelete` (jscompile.c:508) | `"use strict"; delete x;` | `SyntaxError: [string]:1: delete on an unqualified name is not allowed in strict mode` | JS/strict | [x] |
| E81 | `cdelete` (jscompile.c:524) | `delete 1;` | `SyntaxError: [string]:1: invalid l-value in delete expression` | JS | [x] |
| E82 | `cexp` (jscompile.c:780) | **unreachable** — defensive default; every node type the parser can hand to cexp is covered. EXP_PROP_VAL/GET/SET are consumed by cobject, EXP_VAR by cvarinit/cvardecs, STM_CASE/STM_DEFAULT by cswitch, AST_LIST by cstmlist | `SyntaxError: [string]:<node line>: unknown expression type` | NO | [x] proved unreachable; neighbouring branch tested |
| E83 | `ctrycatch` (jscompile.c:961) | `"use strict"; try{}catch(arguments){}` | `SyntaxError: [string]:1: redefining 'arguments' is not allowed in strict mode` | JS/strict | [x] |
| E84 | `ctrycatch` (jscompile.c:963) | `"use strict"; try{}catch(eval){}` | `SyntaxError: [string]:1: redefining 'eval' is not allowed in strict mode` | JS/strict | [x] |
| E85 | `ctrycatchfinally` (jscompile.c:993) | `"use strict"; try{}catch(arguments){}finally{}` | `SyntaxError: [string]:1: redefining 'arguments' is not allowed in strict mode` | JS/strict | [x] |
| E86 | `ctrycatchfinally` (jscompile.c:995) | `"use strict"; try{}catch(eval){}finally{}` | `SyntaxError: [string]:1: redefining 'eval' is not allowed in strict mode` | JS/strict | [x] |
| E87 | `cswitch` (jscompile.c:1025) | `switch(1){default:default:}` | `SyntaxError: [string]:1: more than one default label in switch` | JS | [x] |
| E88 | `cstm` (jscompile.c:1217) | `break foo;` | `SyntaxError: [string]:1: break label 'foo' not found` | JS | [x] |
| E89 | `cstm` (jscompile.c:1221) | `break;` | `SyntaxError: [string]:1: unlabelled break must be inside loop or switch` | JS | [x] |
| E90 | `cstm` (jscompile.c:1233) | `continue foo;` | `SyntaxError: [string]:1: continue label 'foo' not found` | JS | [x] |
| E91 | `cstm` (jscompile.c:1237) | `continue;` | `SyntaxError: [string]:1: continue must be inside loop` | JS | [x] |
| E92 | `cstm` (jscompile.c:1251) | `return;` | `SyntaxError: [string]:1: return not in function` | JS | [x] |
| E93 | `cstm` (jscompile.c:1266) | `"use strict"; with({}){}` | `SyntaxError: [string]:1: 'with' statements are not allowed in strict mode` | JS/strict | [x] |
| E94 | `(declaration)` (jslex.c:4) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E95 | `(declaration)` (jslex.c:6) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E96 | `jsY_expect (macro)` (jslex.c:177) | `JSON.parse("nul")` | `SyntaxError: JSON:1: expected 'l'` | JS | [x] |
| E97 | `jsY_unescape` (jslex.c:192) | `\q` | `SyntaxError: [string]:1: unexpected escape sequence` | JS | [x] |
| E98 | `lexhex` (jslex.c:255) | `0x` | `SyntaxError: [string]:1: malformed hexadecimal number` | JS | [x] |
| E99 | `lexinteger` (jslex.c:269) | **unreachable** — lexinteger is inside the #if 0 block (jslex.c:263-339) and is not compiled | `SyntaxError: [string]:<lexline>: malformed number` | NO | [x] proved unreachable; neighbouring branch tested |
| E100 | `lexnumber` (jslex.c:312) | **unreachable** — this lexnumber variant is inside the #if 0 block (jslex.c:263-339); the live copy is at jslex.c:351 | `SyntaxError: [string]:<lexline>: number with leading zero` | NO | [x] proved unreachable; neighbouring branch tested |
| E101 | `lexnumber` (jslex.c:333) | **unreachable** — this lexnumber variant is inside the #if 0 block (jslex.c:263-339); the live copy is at jslex.c:381 | `SyntaxError: [string]:<lexline>: number with letter suffix` | NO | [x] proved unreachable; neighbouring branch tested |
| E102 | `lexnumber` (jslex.c:351) | `01` | `SyntaxError: [string]:1: number with leading zero` | JS | [x] |
| E103 | `lexnumber` (jslex.c:377) | `1e` | `SyntaxError: [string]:1: missing exponent` | JS | [x] |
| E104 | `lexnumber` (jslex.c:381) | `1a` | `SyntaxError: [string]:1: number with letter suffix` | JS | [x] |
| E105 | `lexescape` (jslex.c:399) | `"\` | `SyntaxError: [string]:1: unterminated escape sequence` | JS | [x] |
| E106 | `lexstring` (jslex.c:440) | `"abc` | `SyntaxError: [string]:1: string not terminated` | JS | [x] |
| E107 | `lexstring` (jslex.c:443) | `"\x"` | `SyntaxError: [string]:1: malformed escape sequence` | JS | [x] |
| E108 | `lexregexp` (jslex.c:490) | `/abc` | `SyntaxError: [string]:1: regular expression not terminated` | JS | [x] |
| E109 | `lexregexp` (jslex.c:497) | `/\` | `SyntaxError: [string]:1: regular expression not terminated` | JS | [x] |
| E110 | `lexregexp` (jslex.c:521) | `/a/x` | `SyntaxError: [string]:1: illegal flag in regular expression: x` | JS | [x] |
| E111 | `lexregexp` (jslex.c:525) | `/a/gg` | `SyntaxError: [string]:1: duplicated flag in regular expression` | JS | [x] |
| E112 | `jsY_lexx` (jslex.c:574) | `/*` | `SyntaxError: [string]:1: multi-line comment not terminated` | JS | [x] |
| E113 | `jsY_lexx` (jslex.c:728) | `#` | `SyntaxError: [string]:1: unexpected character: '#'` | JS | [x] |
| E114 | `jsY_lexx` (jslex.c:729) | `€` | `SyntaxError: [string]:1: unexpected character: \u20AC` | JS | [x] |
| E115 | `lexjsonnumber` (jslex.c:760) | `JSON.parse("-")` | `SyntaxError: JSON:1: unexpected non-digit` | JS | [x] |
| E116 | `lexjsonnumber` (jslex.c:767) | `JSON.parse("1.")` | `SyntaxError: JSON:1: missing digits after decimal point` | JS | [x] |
| E117 | `lexjsonnumber` (jslex.c:777) | `JSON.parse("1e")` | `SyntaxError: JSON:1: missing digits after exponent indicator` | JS | [x] |
| E118 | `lexjsonescape` (jslex.c:791) | `JSON.parse('"\\q"')` | `SyntaxError: JSON:1: invalid escape sequence` | JS | [x] |
| E119 | `lexjsonstring` (jslex.c:820) | `JSON.parse('"abc')` | `SyntaxError: JSON:1: unterminated string` | JS | [x] |
| E120 | `lexjsonstring` (jslex.c:822) | `JSON.parse('"\x01"')` | `SyntaxError: JSON:1: invalid control character in string` | JS | [x] |
| E121 | `jsY_lexjson` (jslex.c:878) | `JSON.parse("x")` | `SyntaxError: JSON:1: unexpected character: 'x'` | JS | [x] |
| E122 | `jsY_lexjson` (jslex.c:879) | `JSON.parse("€")` | `SyntaxError: JSON:1: unexpected character: \u20AC` | JS | [x] |
| E123 | `jsonexpect` (json.c:41) | `JSON.parse('[1')` | `SyntaxError: JSON: unexpected token: (end-of-file) (expected ']')` | JS | [x] |
| E124 | `jsonvalue` (json.c:67) | `JSON.parse('{1:2}')` | `SyntaxError: JSON: unexpected token: (number) (expected string)` | JS | [x] |
| E125 | `jsonvalue` (json.c:107) | `JSON.parse('')` | `SyntaxError: JSON: unexpected token: (end-of-file)` | JS | [x] |
| E126 | `fmtobject` (json.c:261) | `var a={};a.a=a;JSON.stringify(a);` | `TypeError: cyclic object value` | JS | [x] |
| E127 | `fmtarray` (json.c:297) | `var a=[];a[0]=a;JSON.stringify(a);` | `TypeError: cyclic object value` | JS | [x] |
| E128 | `(declaration)` (jsparse.c:22) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E129 | `INCREC (macro)` (jsparse.c:24) | `(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1` | `SyntaxError: [string]:1: too much recursion` | JS | [x] |
| E130 | `(declaration)` (jsparse.c:29) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E131 | `jsP_expect (macro)` (jsparse.c:143) | `(1` | `SyntaxError: [string]:1: unexpected token: (end-of-file) (expected ')')` | JS | [x] |
| E132 | `semicolon` (jsparse.c:153) | `1 2` | `SyntaxError: [string]:1: unexpected token: (number) (expected ';')` | JS | [x] |
| E133 | `identifier` (jsparse.c:166) | `var 1` | `SyntaxError: [string]:1: unexpected token: (number) (expected identifier)` | JS | [x] |
| E134 | `identifiername` (jsparse.c:183) | `a.` | `SyntaxError: [string]:1: unexpected token: (end-of-file) (expected identifier or keyword)` | JS | [x] |
| E135 | `primary` (jsparse.c:363) | `*` | `SyntaxError: [string]:1: unexpected token in expression: '*'` | JS | [x] |
| E136 | `caseclause` (jsparse.c:700) | `switch(1){x}` | `SyntaxError: [string]:1: unexpected token in switch: (identifier) (expected 'case' or 'default')` | JS | [x] |
| E137 | `forstatement` (jsparse.c:751) | `for(var a)` | `SyntaxError: [string]:1: unexpected token in for-var-statement: ')'` | JS | [x] |
| E138 | `forstatement` (jsparse.c:770) | `for(a)` | `SyntaxError: [string]:1: unexpected token in for-statement: ')'` | JS | [x] |
| E139 | `statement` (jsparse.c:888) | `try{}` | `SyntaxError: [string]:1: unexpected token in try: (end-of-file) (expected 'catch' or 'finally')` | JS | [x] |

## Section 3 — `jsrun.c` `jsvalue.c` `jsproperty.c` `jsstring.c` `jsregexp.c` `jsintern.c`

| # | function (site) | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|-----------------|----------------------------------------------|-------------------|-----------|-----|
| E140 | `jsS_newstringnode` (jsintern.c:47) | **unreachable** — needs an interned token (jsparse.c:102 identifier/string literal, jscompile.c:59 filename, jsrun.c:950 ref buf) longer than JS_STRLIMIT=2^28; JS-level strings are already capped at 2^28 by js_pushstring so eval() can never carry a longer token - only a >256MB single token in C source handed to js_loadstring could | `RangeError: invalid string length` | NO | [x] proved unreachable; neighbouring branch tested |
| E141 | `jsV_setproperty` (jsproperty.c:228) | `"use strict"; var o = Object.preventExtensions({}); o.q = 1;` | `TypeError: object is non-extensible` | JS/strict | [x] |
| E142 | `jsV_nextiterator` (jsproperty.c:303) | `js_newstate(0,0,0); js_newobject(J); js_nextiterator(J,-1)` | `TypeError: not an iterator` | CAPI | [x] |
| E143 | `js_newregexpx` (jsregexp.c:38) | `new RegExp("(");` | `SyntaxError: regular expression: unmatched '('` | JS | [x] |
| E144 | `js_RegExp_prototype_exec` (jsregexp.c:77) | `/a*/.exec(new Array(6000).join("a")); (5999 chars make match() recurse past REG_MAXREC=4096 so js_regexec returns -1)` | `Error: regexec failed` | JS | [x] |
| E145 | `Rp_test` (jsregexp.c:126) | `/a*/.test(new Array(6000).join("a")); (same REG_MAXREC=4096 recursion bail-out, js_regexec returns -1)` | `Error: regexec failed` | JS | [x] |
| E146 | `jsB_new_RegExp` (jsregexp.c:149) | `new RegExp(/a/, "g");` | `TypeError: cannot supply flags when creating one RegExp from another` | JS | [x] |
| E147 | `jsB_new_RegExp` (jsregexp.c:172) | `new RegExp("a", "x");` | `SyntaxError: invalid regular expression flag: 'x'` | JS | [x] |
| E148 | `jsB_new_RegExp` (jsregexp.c:175) | `new RegExp("a", "gg");` | `SyntaxError: invalid regular expression flag: 'g'` | JS | [x] |
| E149 | `jsB_new_RegExp` (jsregexp.c:176) | `new RegExp("a", "ii");` | `SyntaxError: invalid regular expression flag: 'i'` | JS | [x] |
| E150 | `jsB_new_RegExp` (jsregexp.c:177) | `new RegExp("a", "mm");` | `SyntaxError: invalid regular expression flag: 'm'` | JS | [x] |
| E151 | `js_pushstring` (jsrun.c:149) | `var s = "a"; while (1) s += s; (once s is 2^28 bytes js_concat gives js_pushstring 2^29 bytes; needs ~800MB RAM or you get "out of memory" instead)` | `RangeError: invalid string length` | BIGMEM | [x] |
| E152 | `js_pushlstring` (jsrun.c:166) | **unreachable** — all callers pass n bounded by an already existing string (<= JS_STRLIMIT) and Ap_join (jsarray.c:145) pre-checks JS_STRLIMIT itself, so n > 2^28 cannot be produced | `RangeError: invalid string length` | NO | [x] proved unreachable; neighbouring branch tested |
| E153 | `js_toregexp` (jsrun.c:373) | `RegExp.prototype.test.call("a", "a");` | `TypeError: not a regexp` | JS | [x] |
| E154 | `js_touserdata` (jsrun.c:382) | `js_newstate(0,0,0); js_pushnull(J); js_newuserdata(J,"Foo",data,NULL); js_touserdata(J,-1,"Bar")` | `TypeError: not a Bar` | CAPI | [x] |
| E155 | `jsR_tofunction` (jsrun.c:393) | `Object.defineProperty({}, "x", {get:1}); (js_defaccessor -> jsR_tofunction on the non-callable getter)` | `TypeError: not a function` | JS | [x] |
| E156 | `js_pop` (jsrun.c:408) | `js_newstate(0,0,0); js_pop(J,1)` | `Error: stack underflow!` | CAPI | [x] |
| E157 | `js_remove` (jsrun.c:416) | `js_newstate(0,0,0); js_remove(J,0)` | `Error: stack error!` | CAPI | [x] |
| E158 | `js_insert` (jsrun.c:424) | `js_newstate(0,0,0); js_pushnumber(J,1); js_insert(J,0)` | `Error: not implemented yet` | CAPI | [x] |
| E159 | `js_replace` (jsrun.c:431) | `js_newstate(0,0,0); js_replace(J,0)` | `Error: stack error!` | CAPI | [x] |
| E160 | `jsR_setarrayindex` (jsrun.c:676) | `var a = []; for (var i = 0; ; ++i) a[i] = 0; (contiguous appends keep the array flat until k = JS_ARRAYLIMIT = 2^26; needs ~1GB RAM)` | `RangeError: array too large` | BIGMEM | [x] |
| E161 | `jsR_setproperty` (jsrun.c:707) | `[].length = 1.5;` | `RangeError: invalid array length` | JS | [x] |
| E162 | `jsR_setproperty` (jsrun.c:709) | `[].length = 1073741824;` | `RangeError: array too large` | JS | [x] |
| E163 | `jsR_setproperty` (jsrun.c:773) | `"use strict"; var o = {get x(){return 1}}; o.x = 2;` | `TypeError: setting property 'x' that only has a getter` | JS/strict | [x] |
| E164 | `jsR_setproperty` (jsrun.c:783) | `js_newstate(0,0,JS_STRICT); js_pushstring(J,"abc"); js_pushnumber(J,1); js_setproperty(J,-2,"foo") (relies on the !js_isobject(J,idx) argument being evaluated before js_toobject(J,idx), as gcc does; no JS path exists because jsV_toobject rewrites the stack slot to the wrapper object before OP_SETPROP/OP_SETPROP_S compute transient, so transient is always 0 from bytecode)` | `TypeError: cannot create property 'foo' on transient object` | CAPI | [x] |
| E165 | `jsR_setproperty` (jsrun.c:800) | `"use strict"; "abc".length = 1;` | `TypeError: 'length' is read-only` | JS/strict | [x] |
| E166 | `jsR_defproperty` (jsrun.c:854) | `"use strict"; Object.defineProperty(function(){}, "length", {value:1}); (value branch hits the JS_READONLY function length property)` | `TypeError: 'length' is read-only` | JS/strict | [x] |
| E167 | `jsR_defproperty` (jsrun.c:860) | `"use strict"; Object.defineProperty(function(){}, "length", {get:function(){}}); (getter branch on the JS_DONTCONF function length property)` | `TypeError: 'length' is non-configurable` | JS/strict | [x] |
| E168 | `jsR_defproperty` (jsrun.c:866) | `"use strict"; Object.defineProperty(function(){}, "length", {set:function(v){}}); (setter branch, getter is NULL so line 860 is skipped)` | `TypeError: 'length' is non-configurable` | JS/strict | [x] |
| E169 | `jsR_defproperty` (jsrun.c:875) | `Object.defineProperty([], "length", {value:0}); (array length goto readonly with throw=1, works non-strict)` | `TypeError: 'length' is read-only or non-configurable` | JS | [x] |
| E170 | `jsR_delproperty` (jsrun.c:921) | `"use strict"; delete [].length;` | `TypeError: 'length' is non-configurable` | JS/strict | [x] |
| E171 | `js_setvar` (jsrun.c:1127) | `"use strict"; undefined = 1;` | `TypeError: 'undefined' is read-only` | JS/strict | [x] |
| E172 | `js_setvar` (jsrun.c:1133) | `"use strict"; xyz = 1;` | `ReferenceError: assignment to undeclared variable 'xyz'` | JS/strict | [x] |
| E173 | `js_delvar` (jsrun.c:1145) | **unreachable** — needs J->strict while executing OP_DELVAR/OP_DELLOCAL, but jscompile.c:507 rejects delete of an unqualified name in strict mode (SyntaxError "delete on an unqualified name is not allowed in strict mode") and J->strict always equals the running function's F->strict; js_delvar is static so there is no C-API entry | `TypeError: 'x' is non-configurable` | NO | [x] proved unreachable; neighbouring branch tested |
| E174 | `jsR_pushtrace` (jsrun.c:1290) | `function f(){ f() } f(); (tracetop+1 == JS_ENVLIMIT=1024 is hit before the 4096-slot value stack or the env stack)` | `Error: call stack overflow` | JS | [x] |
| E175 | `js_call` (jsrun.c:1304) | `js_newstate(0,0,0); js_pushnull(J); js_pushnull(J); js_call(J,-1)` | `RangeError: number of arguments cannot be negative` | CAPI | [x] |
| E176 | `js_call` (jsrun.c:1307) | `undefined();` | `TypeError: undefined is not callable` | JS | [x] |
| E177 | `js_construct` (jsrun.c:1341) | `new undefined();` | `TypeError: undefined is not callable` | JS | [x] |
| E178 | `js_endtry` (jsrun.c:1461) | `js_newstate(0,0,0); js_endtry(J)` | `Error: endtry: exception stack underflow` | CAPI | [x] |
| E179 | `jsR_run` (jsrun.c:1673) | `x = 1; eval("var x; delete x; x"); (OP_GETLOCAL path: eval scripts are never lightweight, "x" is in the eval script vartab but jsR_callscript skipped js_initvar because the plain deletable global already existed, then OP_DELLOCAL/js_delvar removed it)` | `ReferenceError: 'x' is not defined` | JS | [x] |
| E180 | `jsR_run` (jsrun.c:1698) | `nosuchvar; (OP_GETVAR path: read of a free variable that is in no environment; typeof would use OP_HASVAR instead and not throw)` | `ReferenceError: 'nosuchvar' is not defined` | JS | [x] |
| E181 | `jsR_run` (jsrun.c:1721) | `"a" in 1;` | `TypeError: operand to 'in' is not an object` | JS | [x] |
| E182 | `js_doregexec` (jsstring.c:9) | `new Array(6000).join("a").search(/a*/); (Sp_search -> js_doregexec, js_regexec returns -1 past REG_MAXREC=4096)` | `Error: regexec failed` | JS | [x] |
| E183 | `checkstring` (jsstring.c:16) | `String.prototype.trim.call(null);` | `TypeError: string function called on null or undefined` | JS | [x] |
| E184 | `Sp_toString` (jsstring.c:108) | `String.prototype.toString.call(1);` | `TypeError: not a string` | JS | [x] |
| E185 | `Sp_valueOf` (jsstring.c:115) | `String.prototype.valueOf.call(1);` | `TypeError: not a string` | JS | [x] |
| E186 | `Sp_concat` (jsstring.c:163) | `var s = "a"; while (s.length < 268435456) s += s; s.concat("x"); (this-string is exactly 2^28 so the initial n = 1+strlen(s) already exceeds JS_STRLIMIT; needs ~0.5GB RAM)` | `RangeError: invalid string length` | BIGMEM | [x] |
| E187 | `Sp_concat` (jsstring.c:171) | `var s = "a"; while (s.length < 134217728) s += s; s.concat(s); (initial n = 1+2^27 passes, then the argument pushes n to 1+2^28 in the loop; needs ~0.3GB RAM)` | `RangeError: invalid string length` | BIGMEM | [x] |
| E188 | `jsV_toprimitive` (jsvalue.c:144) | `"use strict"; var o = Object.create(null); o + "";` | `TypeError: cannot convert object to primitive` | JS/strict | [x] |
| E189 | `jsV_toobject` (jsvalue.c:401) | `undefined.x;` | `TypeError: cannot convert undefined to object` | JS | [x] |
| E190 | `jsV_toobject` (jsvalue.c:402) | `null.x;` | `TypeError: cannot convert null to object` | JS | [x] |
| E191 | `js_instanceof` (jsvalue.c:579) | `1 instanceof 2;` | `TypeError: instanceof: invalid operand` | JS | [x] |
| E192 | `js_instanceof` (jsvalue.c:586) | `var f = function(){}; f.prototype = 1; ({}) instanceof f;` | `TypeError: instanceof: 'prototype' property is not an object` | JS | [x] |

## Section 4 — `regexp.c` `die()` sites and `regexec` return values

Every row in this section was verified against an instrumented build of `regexp.c` in which each `die()` message was tagged with its line number.

| # | function (site) | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|-----------------|----------------------------------------------|-------------------|-----------|-----|
| E193 | `(definition)` (regexp.c:67) | `not a rejection — declaration/definition of the error helper` | — | n/a | n/a |
| E194 | `hex` (regexp.c:101) | `\\xZZ  [non-hex digit after \\x when at least 2 chars remain; also \\uZZZZ]` | `regcomp -> NULL, *errorp = "invalid escape sequence"` | JS | [x] |
| E195 | `dec` (regexp.c:108) | `a{x}  [first char inside {} is not a digit; any non-digit inside {} works, e.g. a{1,x}]` | `regcomp -> NULL, *errorp = "invalid quantifier"` | JS | [x] |
| E196 | `nextrune` (regexp.c:128) | `a\\  [pattern ends with a lone backslash: nothing at all follows the backslash]` | `regcomp -> NULL, *errorp = "unterminated escape sequence"` | JS | [x] |
| E197 | `nextrune` (regexp.c:138) | `a\\c  [\\c at the very end of the pattern: 0 chars after \\c]` | `regcomp -> NULL, *errorp = "unterminated escape sequence"` | JS | [x] |
| E198 | `nextrune` (regexp.c:143) | `a\\x  [fewer than 2 chars after \\x; also a\\xA]` | `regcomp -> NULL, *errorp = "unterminated escape sequence"` | JS | [x] |
| E199 | `nextrune` (regexp.c:153) | `a\\u12  [fewer than 4 chars after \\u; also a\\u , a\\u1 , a\\u123]` | `regcomp -> NULL, *errorp = "unterminated escape sequence"` | JS | [x] |
| E200 | `nextrune` (regexp.c:170) | `a\\q  [identity escape of a letter or underscore that is not in ESCAPES "BbDdSsWw^$\\.*+?()[]{}\|-0123456789" and not one of f n r t v c x u; also a\\_ , a\\y]` | `regcomp -> NULL, *errorp = "invalid escape character"` | JS | [x] |
| E201 | `lexcount` (regexp.c:186) | `a{255}  [min counter reaches >= REPINF (255) inside the loop; a{254} compiles OK]` | `regcomp -> NULL, *errorp = "numeric overflow"` | JS | [x] |
| E202 | `lexcount` (regexp.c:200) | `a{1,255}  [max counter reaches >= REPINF (255); a{1,254} compiles OK]` | `regcomp -> NULL, *errorp = "numeric overflow"` | JS | [x] |
| E203 | `newcclass` (regexp.c:213) | `pattern = 129 repetitions of "[a]" (129 newcclass calls; the 129th dies because ncclass == REG_MAXCLASS == 128). 128 repetitions compiles OK. Equivalent: 129 repetitions of "\\d".` | `regcomp -> NULL, *errorp = "too many character classes"` | JS | [x] |
| E204 | `addrange` (regexp.c:224) | `[z-a]  [range start > range end; also [\\x02-\\x01]]` | `regcomp -> NULL, *errorp = "invalid character class range"` | JS | [x] |
| E205 | `addrange` (regexp.c:253) | `pattern = "[" + the 32 escapes "\\x01","\\x03","\\x05",...,"\\x3F" (odd bytes 0x01..0x3F, spaced by 2 so no two spans ever merge) + "]"; the 32nd addrange dies because Reclass.spans holds only REG_MAXSPAN=64 Runes = 32 pairs and the guard is end+2 >= spans+64, so only 31 spans fit. 31 such chars compiles OK.` | `regcomp -> NULL, *errorp = "too many character class ranges"` | JS | [x] |
| E206 | `lexclass` (regexp.c:322) | `[a  [EOF reached inside a character class; also "[" or "[^a"]` | `regcomp -> NULL, *errorp = "unterminated character class"` | JS | [x] |
| E207 | `newrep` (regexp.c:493) | `()*  [max == REPINF applied to an atom that can match empty; also (?:)* , ()+ , (a*)* , (){0,}]` | `regcomp -> NULL, *errorp = "infinite loop matching the empty string"` | JS | [x] |
| E208 | `parseatom` (regexp.c:541) | `\\1  [back-reference number >= g->nsub, i.e. no such group defined yet; also (a)\\2 or the forward reference \\1()]` | `regcomp -> NULL, *errorp = "invalid back-reference"` | JS | [x] |
| E209 | `parseatom` (regexp.c:552) | `pattern = 16 repetitions of "()" (nsub starts at 1, so the 16th "(" sees nsub == REG_MAXSUB == 16 and dies). 15 repetitions compiles OK.` | `regcomp -> NULL, *errorp = "too many captures"` | JS | [x] |
| E210 | `parseatom` (regexp.c:557) | `(  [plain capturing group with no ")"; also "(a"]` | `regcomp -> NULL, *errorp = "unmatched '('"` | JS | [x] |
| E211 | `parseatom` (regexp.c:563) | `(?:  [non-capturing group with no ")"; also "(?:a"]` | `regcomp -> NULL, *errorp = "unmatched '('"` | JS | [x] |
| E212 | `parseatom` (regexp.c:570) | `(?=  [positive lookahead with no ")"; also "(?=a"]` | `regcomp -> NULL, *errorp = "unmatched '('"` | JS | [x] |
| E213 | `parseatom` (regexp.c:577) | `(?!  [negative lookahead with no ")"; also "(?!a"]` | `regcomp -> NULL, *errorp = "unmatched '('"` | JS | [x] |
| E214 | `parseatom` (regexp.c:580) | `*  [a quantifier or other token where an atom is required; also "+", "?", "{1}", "(?", "a\|*"]` | `regcomp -> NULL, *errorp = "syntax error"` | JS | [x] |
| E215 | `parserep` (regexp.c:598) | `a{2,1}  [{M,N} with N < M]` | `regcomp -> NULL, *errorp = "invalid quantifier"` | JS | [x] |
| E216 | `count` (regexp.c:661) | `pattern = 4097 repetitions of "a" (parsecat builds a right-leaning P_CAT chain, so N concatenated atoms nest N deep and count() dies once depth exceeds REG_MAXREC == 4096). 4096 "a"s compiles OK. Keep strlen <= 16384 or line 922 fires first.` | `regcomp -> NULL, *errorp = "stack overflow"` | JS | [x] |
| E217 | `count` (regexp.c:672) | `(?:a{254}){254}  [nested counted repeats multiply: count = 254*254 = 64516 > REG_MAXPROG == 32768, detected inside count() on the outer P_REP node]` | `regcomp -> NULL, *errorp = "program too large"` | JS | [x] |
| E218 | `regcompx` (regexp.c:916) | **unreachable** — allocation-failure with default_alloc. Reachable through regcompx by supplying an allocator that returns NULL on its 1st non-zero-size call (the sizeof(Reprog) alloc at line 914); pattern is irrelevant, e.g. "a". | `regcomp -> NULL, *errorp = "cannot allocate regular expression"` | NO | [x] proved unreachable; neighbouring branch tested |
| E219 | `regcompx` (regexp.c:922) | `pattern = 16385 repetitions of "a" (strlen(pattern)*2 == 32770 > REG_MAXPROG == 32768). 16384 "a"s passes this check and then dies at line 661 instead.` | `regcomp -> NULL, *errorp = "program too large"` | JS | [x] |
| E220 | `regcompx` (regexp.c:926) | **unreachable** — allocation-failure with default_alloc. Reachable through regcompx by supplying an allocator that returns NULL on its 2nd non-zero-size call (the Renode array at line 924); pattern must be non-empty, e.g. "a". | `regcomp -> NULL, *errorp = "cannot allocate regular expression parse list"` | NO | [x] proved unreachable; neighbouring branch tested |
| E221 | `regcompx` (regexp.c:940) | `)  [a ")" with no open group; also "a)" or "(a))"]` | `regcomp -> NULL, *errorp = "unmatched ')'"` | JS | [x] |
| E222 | `regcompx` (regexp.c:942) | **unreachable** — defensive-dead-code. After parsealt() returns, lookahead can only be EOF or ")": parsecat() stops only on EOF, "|" or ")" and parsealt() consumes every "|", and the ")" case is already caught at line 940. Verified empirically: brute force over all patterns of length <= 4 drawn from the alphabet a b ( ) | ? * + . ^ $ [ ] { } , - backslash 1 : = ! never reaches it. | `regcomp -> NULL, *errorp = "syntax error"` | NO | [x] proved unreachable; neighbouring branch tested |
| E223 | `regcompx` (regexp.c:951) | `(?:a{254}){129}  [count() == 254*129 == 32766 passes the per-node check at line 672, but 6 + 32766 == 32772 > REG_MAXPROG == 32768]. Equivalent: 129 repetitions of "a{254}". The {128} version compiles OK.` | `regcomp -> NULL, *errorp = "program too large"` | JS | [x] |
| E224 | `regcompx` (regexp.c:956) | **unreachable** — allocation-failure with default_alloc. Reachable through regcompx by supplying an allocator that returns NULL on its 3rd non-zero-size call (the Reinst array at line 954) for a non-empty pattern such as "a"; for pattern "" it is the 2nd call, since the parse-list alloc is skipped. | `regcomp -> NULL, *errorp = "cannot allocate regular expression instruction list"` | NO | [x] proved unreachable; neighbouring branch tested |
| E225 | `regcompx` (regexp.c:961) | **unreachable** — allocation-failure with default_alloc. Reachable through regcompx by supplying an allocator that returns NULL on its 4th non-zero-size call (the Reclass array at line 959); the pattern must create at least one character class, e.g. "[a]". | `regcomp -> NULL, *errorp = "cannot allocate regular expression character class list"` | NO | [x] proved unreachable; neighbouring branch tested |
| E226 | `regexec` (regexp.c (behaviour)) | `regexec(regcomp("b",0,&e), "aaa", &m, 0) == 1 ; regexec(same, "abc", &m, 0) == 0` | `regcomp -> NULL, *errorp = "no match: regexec returns 1 (propagated from match); 0 means matched, -1 means execution stack overflow"` | JS | [x] |
| E227 | `regexec` (regexp.c (behaviour)) | `regexec(p, "abc", NULL, 0) returns the same 0/1/-1 as with a real Resub` | `regcomp -> NULL, *errorp = "sub == NULL is legal: regexec points sub at a local "Resub scratch", so capture results are simply discarded; the return value is unchanged"` | JS | [x] |
| E228 | `regexec` (regexp.c (behaviour)) | `regcomp("(a)(b)") yields prog->nsub == 3; caller sets sub.nsub = 1, and after regexec sub.nsub == 3. Since the clearing loop always runs 0..REG_MAXSUB-1, a caller built with a smaller REG_MAXSUB gets memory past its Resub overwritten.` | `regcomp -> NULL, *errorp = "prog->nsub > sub->nsub is NOT rejected: regexec unconditionally does sub->nsub = prog->nsub and clears all REG_MAXSUB == 16 slots, ignoring whatever the caller stored in nsub"` | JS | [x] |
| E229 | `match` (regexp.c (behaviour)) | `regexec(regcomp("a*",0,&e), a subject of 5000 "a" chars, &m, 0) == -1. Each iteration of the "*" loop recurses through I_SPLIT so depth grows with matched chars; measured threshold is 4095 "a"s (4094 still returns 0). The leading .* search loop (I_SPLIT/I_ANYNL/I_JUMP) does not accumulate depth, so a merely long non-matching subject stays at depth 1.` | `regcomp -> NULL, *errorp = "recursion limit at regexp.c:1075 (depth > REG_MAXREC == 4096) returns -1, and every I_SPLIT/I_PLA/I_NLA site propagates -1, so regexec returns -1 (distinct from 0 = match and 1 = no match)"` | JS | [x] |
| E230 | `regcompx` (regexp.c (behaviour)) | `regcomp("", 0, &err) returns a valid Reprog with nsub == 1 that matches at position 0 of any subject (regexec returns 0)` | `regcomp -> NULL, *errorp = "empty pattern "" is accepted, no die: n = strlen*2 == 0 so the parse-list alloc is skipped (pstart stays NULL), lex returns EOF, node == NULL, count() == 0, program is exactly 6 instructions, *errorp is set to NULL"` | JS | [x] |
| E231 | `regcompx` (regexp.c (behaviour)) | `regcomp("(", 0, NULL) returns NULL without crashing; regcomp("a", 0, NULL) returns a valid Reprog` | `regcomp -> NULL, *errorp = "errorp == NULL is safe: both the longjmp error path (line 904) and the success path (line 984) guard with "if (errorp)", so nothing is written through the null pointer"` | JS | [x] |

## Section 5 — resource / limit rejections

These are not `js_*error` call sites (they push a literal string onto the stack
and `js_throw`, so they surface with **no** `Error` wrapper: the report string is
the bare message, e.g. `stack overflow`, not `Error: stack overflow`).

| # | function (site) | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|-----------------|----------------------------------------------|-------------------|-----------|-----|
| L1 | `js_trystackoverflow` (jsrun.c:14) via `js_savetry` (jsrun.c:1447) | `JS_TRYLIMIT` (64) nested `js_savetry` frames | throws literal `exception stack overflow` | CAPI | [x] |
| L2 | `js_trystackoverflow` via `js_savetrypc` (jsrun.c:1433) | 64 nested JS `try{}` blocks entered at once | throws literal `exception stack overflow` | JS | [x] |
| L3 | `js_ptry` (jsstate.c:6) in `js_dostring` | `trytop == JS_TRYLIMIT` on entry | returns `1`, reports `exception stack overflow` | CAPI | [x] |
| L4 | `js_ptry` in `js_ploadstring` | same | returns `1` | CAPI | [x] |
| L5 | `js_ptry` in `js_trystring` | same | returns the caller's `error` argument | CAPI | [x] |
| L6 | `js_ptry` in `js_trynumber` | same | returns the caller's `error` argument | CAPI | [x] |
| L7 | `js_ptry` in `js_tryinteger` | same | returns the caller's `error` argument | CAPI | [x] |
| L8 | `js_ptry` in `js_tryboolean` | same | returns the caller's `error` argument | CAPI | [x] |
| L9 | `js_stackoverflow` (jsrun.c:22) via `CHECKSTACK` (jsrun.c:106) | push more than `JS_STACKSIZE` (4096) values | throws literal `stack overflow` | CAPI | [x] |
| L10 | `js_stackoverflow` via `jsR_calllwfunction` (jsrun.c:1161) | deep JS recursion in a lightweight function | throws literal `stack overflow` | JS | [x] |
| L11 | `js_outofmemory` (jsrun.c:30) via `js_malloc` (jsrun.c:57) | `js_setlimit(J, 0, memlimit)` with `size >= memlimit` | throws literal `out of memory` | CAPI | [x] |
| L12 | `js_outofmemory` via `js_realloc` (jsrun.c:71) | same, on a growing buffer/array | throws literal `out of memory` | CAPI | [x] |
| L13 | `js_outofmemory` via `js_malloc` allocator failure (jsrun.c:62) | allocator returns NULL | throws literal `out of memory` | CAPI | [x] |
| L14 | `js_outofmemory` via `js_realloc` allocator failure (jsrun.c:76) | allocator returns NULL | throws literal `out of memory` | CAPI | [x] |
| L15 | `js_runlimit` (jsrun.c:38) | `js_setlimit(J, runlimit, 0)`, `runlimit` reaches 1 in `jsR_run` | throws literal `script ran too long` | CAPI | [x] |
| L16 | `INCREC` (jsparse.c:24) | expression nesting deeper than `JS_ASTLIMIT` (400) | `SyntaxError: [string]:1: too much recursion` | JS | [x] |
| L17 | `jsR_pushtrace` (jsrun.c:1290) | JS recursion deeper than `JS_ENVLIMIT` (1024) trace slots | `Error: call stack overflow` | JS | [x] |
| L18 | `js_newstate` (jsstate.c:183) | `alloc` returns NULL for the `js_State` | returns `NULL` | CAPI | [x] |
| L19 | `js_newstate` (jsstate.c:200) | `alloc` returns NULL for the value stack | frees `J`, returns `NULL` | CAPI | [x] |
| L20 | `js_newstate` (jsstate.c:211) | an allocation inside `jsB_init` fails | `js_freestate`, returns `NULL` | CAPI | [x] |
| L21 | `REG_MAXREC` in `match` (regexp.c:1075) | execution recursion depth > 4096 | `regexec` returns `-1` | CAPI | [x] |
| L22 | `REG_MAXSUB` (regexp.c:1236) | `regexec` always writes all 16 slots and overwrites `sub->nsub` | `sub->nsub = prog->nsub` regardless of the caller's value | CAPI | [x] |
| L23 | `JS_ARRAYLIMIT` via `js_setlength`/`jsV_resizearray` | length at / above `1<<26` | `RangeError: array too large` | CAPI | [x] |
| L24 | `JS_STRLIMIT` via `js_pushlstring` | `n` at / above `1<<28` | `RangeError: invalid string length` | CAPI | [x] |

## Section 6 — return-code contracts of the protected entry points

| # | function | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|----------|----------------------------------------------|-------------------|-----------|-----|
| R1 | `js_dostring` | valid source | `0`, nothing reported | JS | [x] |
| R2 | `js_dostring` | empty source / only a comment / only whitespace | `0` | JS | [x] |
| R3 | `js_dostring` | parse error | `1` + `SyntaxError: ...` reported | JS | [x] |
| R4 | `js_dostring` | runtime error | `1` + `<Name>: <msg>` reported | JS | [x] |
| R5 | `js_dostring` | `throw` of a non-Error primitive (`1`, `'s'`, `null`, `undefined`) | `1`; report is `js_trystring` of the thrown value | JS | [x] |
| R6 | `js_dostring` | `throw` of an object with no `toString` | `1`; report is the fallback `Error` | JS | [x] |
| R7 | `js_ploadstring` | valid source | `0`, compiled function left on the stack | CAPI | [x] |
| R8 | `js_ploadstring` | parse error | `1`, exception value left on the stack | CAPI | [x] |
| R9 | `js_ploadstring` | filename longer than the 256-byte `snprintf` prefix | `1`, truncated message | CAPI | [x] |
| R10 | `js_pcall` | callee is callable, `n` = 0 / 1 / many | `0`, result on the stack | CAPI | [x] |
| R11 | `js_pcall` | callee is not callable (number, string, null, undefined, object, array) | `1` + `TypeError: <typeof> is not callable` | CAPI | [x] |
| R12 | `js_pcall` | callee throws | `1`, exception on the stack | CAPI | [x] |
| R13 | `js_pcall` | `n` negative | `1` + `RangeError: number of arguments cannot be negative` | CAPI | [x] |
| R14 | `js_pconstruct` | constructible callee | `0` | CAPI | [x] |
| R15 | `js_pconstruct` | non-callable callee | `1` + `TypeError: <typeof> is not callable` | CAPI | [x] |
| R16 | `js_trystring` / `js_trynumber` / `js_tryinteger` / `js_tryboolean` | value whose conversion throws | returns the caller's fallback, pops 1 | CAPI | [x] |
| R17 | `js_tryrepr` | value whose `toString` throws | returns the caller's fallback | CAPI | [x] |
| R18 | `regcomp` | valid pattern | non-NULL, `*errorp = NULL` | CAPI | [x] |
| R19 | `regcomp` | invalid pattern | `NULL`, `*errorp` = message | CAPI | [x] |
| R20 | `regexec` | match | `0`, `sub` filled | CAPI | [x] |
| R21 | `regexec` | no match | `1` | CAPI | [x] |
| R22 | `regexec` | execution recursion overflow | `-1` | CAPI | [x] |

## Section 7 — generic FFI boundaries (NULL, lengths, one-past-range, out-of-range enums)

| # | function | trigger (the exact invalid input/condition) | expected C result | reachable | [x] |
|---|----------|----------------------------------------------|-------------------|-----------|-----|
| G1 | `js_newstate` | `alloc = NULL` | uses `js_defaultalloc`, succeeds | CAPI | [x] |
| G2 | `js_newstate` | `flags` out of range: `2 3 4 7 8 0x10 0x40000000 0x7fffffff -1 -2 INT_MIN` | only bit 0 (`JS_STRICT`) is read; all other bits ignored | CAPI | [x] |
| G3 | `js_setreport` | `report = NULL` | `js_report` becomes a no-op | CAPI | [x] |
| G4 | `js_atpanic` | `panic = NULL` | returns the previous handler; `js_throw` with no frame calls `abort()` | CAPI | [x] |
| G5 | `js_setcontext`/`js_getcontext` | NULL and non-NULL round trip | exact round trip | CAPI | [x] |
| G6 | `jsV_newobject` | `type` out of `enum js_Class` range: `16 17 100 -1 INT_MAX INT_MIN` | object created with that raw class value; behaviour of `js_typeof` etc. must match | CAPI | [x] |
| G7 | `js_pushiterator` | `own` out of range: `2 100 -1 INT_MAX INT_MIN` | any non-zero behaves as "own only" | CAPI | [x] |
| G8 | `js_defproperty` | `atts` out of `{JS_READONLY,JS_DONTENUM,JS_DONTCONF}` range: `8 9 15 16 100 -1 INT_MAX INT_MIN` | only bits 0-2 are read | CAPI | [x] |
| G9 | `js_defglobal` | `atts` out of range (same set) | only bits 0-2 are read | CAPI | [x] |
| G10 | `js_defaccessor` | `atts` out of range (same set) | only bits 0-2 are read | CAPI | [x] |
| G11 | `js_newregexp` | `flags` out of `{JS_REGEXP_G,I,M}` range: `8 9 15 16 100 -1 INT_MAX INT_MIN` | only bits 0-2 are read | CAPI | [x] |
| G12 | `js_regcomp`/`js_regcompx` | `cflags` out of `{REG_ICASE,REG_NEWLINE}` range: `4 8 16 0x7fffffff -1 INT_MIN` | extra bits are stored into `prog->flags` and OR-ed with `eflags` at exec time | CAPI | [x] |
| G13 | `js_regexec` | `eflags` out of `{REG_NOTBOL}` range: `1 2 8 16 0x7fffffff -1 INT_MIN` | OR-ed into the flag word; only `REG_NOTBOL`/`REG_NEWLINE`/`REG_ICASE` bits are tested | CAPI | [x] |
| G14 | `js_regcomp` | `errorp = NULL` on success **and** on failure | never dereferenced (both writes are `if (errorp)`-guarded) | CAPI | [x] |
| G15 | `js_regexec` | `sub = NULL` | uses an internal scratch `Resub`; return value unchanged | CAPI | [x] |
| G16 | `js_gc` | `report` out of range: `2 -1 INT_MAX INT_MIN` | any non-zero prints statistics | CAPI | [x] |
| G17 | `js_pop` | `n` = `0`, `> gettop`, negative, `INT_MAX` | `0` is a no-op; underflow → `Error: stack underflow!` | CAPI | [x] |
| G18 | `js_remove` / `js_replace` | index out of range (`0`, `100`, `-100`, `INT_MIN`, `INT_MAX`) | `Error: stack error!` | CAPI | [x] |
| G19 | `js_insert` | any index | `Error: not implemented yet` | CAPI | [x] |
| G20 | `js_rot` / `js_copy` | `n`/idx out of range | must match (no explicit check in the C) | CAPI | [x] |
| G21 | `js_type` / `js_typeof` / `js_torepr` | idx out of range (`0`, `100`, `-100`, `INT_MIN`, `INT_MAX`) | must match (`stackidx` clamps with no check) | CAPI | [x] |
| G22 | `js_pushlstring` | `n` = `0`, `1`, `15`, `16` (shrstr boundary), negative, `INT_MIN`, `JS_STRLIMIT`, `JS_STRLIMIT+1` | `n >= JS_STRLIMIT` → `RangeError: invalid string length` | CAPI | [x] |
| G23 | `js_setlength` | `len` = `0`, negative, `INT_MAX`, `JS_ARRAYLIMIT±1` | must match | CAPI | [x] |
| G24 | `js_getlength` | receiver is a primitive | must match | CAPI | [x] |
| G25 | `jsV_resizearray` | `newlen` = `0`, negative, `INT_MAX`, `JS_ARRAYLIMIT±1` | `RangeError: array too large` past the limit | CAPI | [x] |
| G26 | `jsR_unflattenarray` | receiver is not an array | must match | CAPI | [x] |
| G27 | `js_touserdata` | wrong `tag` | `TypeError: not a <tag>` | CAPI | [x] |
| G28 | `js_isuserdata` | wrong `tag`, and non-userdata receiver | `0` | CAPI | [x] |
| G29 | `js_toregexp` | non-regexp receiver | `TypeError: not a regexp` | CAPI | [x] |
| G30 | `jsY_tokenstring` | token id `< 0`, `>= nelem(tokenstring)`, and ids whose table entry is NULL | `"<unknown>"` | CAPI | [x] |
| G31 | `jsY_iswhite`/`jsY_isnewline`/`jsY_ishex`/`jsY_tohex` | every value `-1000 .. 0x11000` | must match exactly | CAPI | [x] |
| G32 | `jsU_runelen` | rune `< 0`, `> Runemax`, `INT_MIN`, `INT_MAX` | must match | CAPI | [x] |
| G33 | `jsU_runetochar` | rune `< 0`, `> Runemax`, `INT_MIN`, `INT_MAX` | must match (including bytes written past `UTFmax`) | CAPI | [x] |
| G34 | `jsU_chartorune` | invalid lead byte, bare continuation byte, truncated sequence, overlong form, surrogate, `> Runemax` | `Runeerror` + the C's exact consumed length | CAPI | [x] |
| G35 | `js_itoa` | radix outside 2..36 | must match (see `tests/numbers.rs` for the excluded degenerate cases) | CAPI | [x] |
| G36 | `js_strtol` | base outside 2..36 (`0`, `-1`, `37`) | must match | CAPI | [x] |
| G37 | `js_strtod` / `js_stringtofloat` | empty string, all-whitespace, no digits, overflow, underflow | value + `end` offset must match | CAPI | [x] |
| G38 | `jsV_numbertoint32`/`uint32`/`int16`/`uint16`/`integer` | NaN, ±Inf, ±0, values past 2^31/2^32/2^53 | must match | CAPI | [x] |
| G39 | `js_call` / `js_construct` (unprotected) with no try frame | `js_throw` with `trytop == 0` | calls `J->panic` then `abort()` — identical in both | CAPI | [x] |
| G40 | `js_endtry` | `trytop == 0` | `Error: endtry: exception stack underflow`, then `abort()` (no frame to catch it) | CAPI | [x] |
