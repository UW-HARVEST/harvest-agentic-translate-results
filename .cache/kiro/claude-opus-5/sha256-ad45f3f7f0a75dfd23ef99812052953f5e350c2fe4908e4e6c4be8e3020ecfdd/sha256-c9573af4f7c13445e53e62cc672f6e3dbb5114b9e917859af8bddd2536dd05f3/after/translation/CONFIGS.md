# CONFIGS.md — configuration surface (VALID inputs) of the C library

Derived mechanically from the branches the C source actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `js_newstate` flags | `0`, `JS_STRICT` (`flags & JS_STRICT` → `J->strict = J->default_strict = 1`) | `jsstate.c:204` |
| `js_newstate` alloc | `NULL` → `js_defaultalloc`, custom `js_Alloc` | `jsstate.c:194` |
| `js_setlimit` | `runlimit>0` (per-instruction countdown), `memlimit>0` (allocation cap), `0` = unlimited | `jsrun.c:46,55,1602` |
| `js_gc(J,report)` | `report=0` silent, `report!=0` prints stats | `jsgc.c` |
| script strictness | `"use strict"` directive vs `default_strict` | `jscompile.c`, `jsparse.c` |
| compile entry | `jsC_compilescript(prog, default_strict)` with `0`/`1`; `jsC_compilefunction` | `jsi.h:849` |
| parse entry | `jsP_parse(file,src)` vs `jsP_parsefunction(file,params,body)` | `jsi.h:710` |
| lexer mode | `jsY_lex` (JS) vs `jsY_lexjson` (JSON) | `jsi.h:571` |
| regexp compile flags | `0`, `REG_ICASE(1)`, `REG_NEWLINE(2)` and their combination | `regexp.h` |
| regexp exec flags | `0`, `REG_NOTBOL(4)` | `regexp.h` |
| regexp `sub` arg | `NULL` (no captures wanted) vs `Resub*` | `regexp.c` regexec |
| regexp alloc | `js_regcomp` (default alloc) vs `js_regcompx(alloc,ctx,...)` | `regexp.h` |
| JS RegExp flags | `JS_REGEXP_G(1)`, `_I(2)`, `_M(4)` — all 8 combinations | `mujs.h`, `jsregexp.c` |
| property attributes | `JS_READONLY(1)`, `JS_DONTENUM(2)`, `JS_DONTCONF(4)` — all 8 combinations | `mujs.h`, `jsproperty.c` |
| iterator ownership | `js_pushiterator(J, idx, own)` with `own=0` (proto chain) / `own=1` | `jsvalue.c` `jsV_newiterator` |
| `js_toprimitive` hint | `JS_HNONE`, `JS_HNUMBER`, `JS_HSTRING` | `jsvalue.c` |
| value type | undefined, null, boolean, number, shrstr, litstr, memstr, object | `jsi.h:303-309` |
| object class | 16 `js_Class` values (object/array/function/script/cfunction/error/boolean/number/string/regexp/date/math/json/arguments/iterator/userdata) | `jsi.h` |
| string shape | empty, ≤7 bytes (`shrstr` inline), 8–15, ≥16 (`memstr`), interned literal, multi-byte UTF-8, invalid UTF-8, embedded NUL via `js_pushlstring` | `jsi.h:343-346`, `jsvalue.c` |
| array shape | empty, simple/flat (`u.a.simple`), sparse (after unflatten), length 0/1/many, huge length | `jsvalue.c` `jsV_resizearray`, `jsR_unflattenarray` |
| number shape | `+0`, `-0`, NaN, ±Inf, small ints, ±2^31, ±2^32, ±2^53, denormal, huge, fractional | `jsvalue.c` `jsV_numberto*`, `jsdtoa.c` |
| `js_strtol` radix | 0 (auto), 2, 8, 10, 16, 36, and out-of-range radix | `jsdtoa.c` |
| stack index | positive absolute, negative relative, `0`, `-1` | `jsrun.c` `stackidx` |

## Configuration rows

Each row is a combination the C treats differently. `[x]` = differential test
passes across randomized inputs.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jsU_chartorune` | all 1-byte sequences 0x00..0x7F (`Runeself` fast path) | [x] |
| 2 | `jsU_chartorune` | 2-byte sequences, valid + overlong + truncated | [x] |
| 3 | `jsU_chartorune` | 3-byte sequences, valid + overlong + surrogates + truncated | [x] |
| 4 | `jsU_chartorune` | 4-byte sequences, valid + overlong + > Runemax + truncated | [x] |
| 5 | `jsU_chartorune` | random byte soup (invalid continuations → `Runeerror`) | [x] |
| 6 | `jsU_runetochar` | every rune class: <0x80, <0x800, <0x10000, ≤Runemax, >Runemax, negative | [x] |
| 7 | `jsU_runelen` | full rune range incl. out-of-range values | [x] |
| 8 | `jsU_isalpharune`/`islowerrune`/`isupperrune` | full 0..0x110000 sweep + negatives | [x] |
| 9 | `jsU_tolowerrune`/`toupperrune` | full 0..0x110000 sweep + negatives | [x] |
| 10 | `jsU_tolowerrune_full`/`toupperrune_full` | runes with multi-char mappings vs none (NULL) | [x] |
| 11 | `js_utflen`, `js_utfptrtoidx` | empty / ASCII / multibyte / invalid-byte strings | [x] |
| 12 | `js_itoa` | INT_MIN, INT_MAX, 0, ±random | [x] |
| 13 | `js_fmtexp` | e = 0, ±1..±9, ±10..±99, ±100..±999 (digit-count branches) | [x] |
| 14 | `js_grisu2` | normal doubles, denormals, powers of two, ±0 | [x] |
| 15 | `js_strtod` | decimal / exponent / hex / leading space / sign / garbage; `endptr` non-NULL | [x] |
| 16 | `js_strtod` | `endptr == NULL` | [x] |
| 17 | `js_strtol` | radix 0 (auto-detect 0x/0/dec) | [x] |
| 18 | `js_strtol` | radix 2, 8, 10, 16, 36 | [x] |
| 19 | `js_strtol` | radix 1 / 37 / negative (out of range) | [x] |
| 20 | `js_stringtofloat` | full numeric-literal grammar + trailing garbage | [x] |
| 21 | `jsV_numbertointeger`/`toint32`/`touint32`/`toint16`/`touint16` | ±0, NaN, ±Inf, ±2^31, ±2^32, ±2^53, denormal, fractional, huge | [x] |
| 22 | `jsV_numbertostring` | integer-valued (fast path), fractional, exponent range, ±0, NaN, ±Inf | [x] |
| 23 | `jsV_stringtonumber` | "", whitespace-only, "0x..", "Infinity", "-Infinity", decimal, garbage | [x] |
| 24 | `jsY_iswhite`/`isnewline`/`ishex`/`tohex` | full 0..0x2100 sweep + negatives | [x] |
| 25 | `jsY_tokenstring` | every token id 0..350 incl. out-of-range | [x] |
| 26 | `jsY_findword` | present / absent / boundary words in a sorted list | [x] |
| 27 | `js_regcomp` | flags = 0, whole pattern corpus, `sub != NULL` | [x] |
| 28 | `js_regcomp` | flags = `REG_ICASE` | [x] |
| 29 | `js_regcomp` | flags = `REG_NEWLINE` | [x] |
| 30 | `js_regcomp` | flags = `REG_ICASE|REG_NEWLINE` | [x] |
| 31 | `js_regexec` | eflags = 0 vs `REG_NOTBOL`, on multi-line subjects | [x] |
| 32 | `js_regexec` | `sub == NULL` (no capture output) | [x] |
| 33 | `js_regcompx`/`js_regfreex` | custom allocator callback | [x] |
| 34 | `js_regexec` | randomized subjects incl. empty, UTF-8, newline-terminated | [x] |
| 35 | `js_newstate` | `alloc=NULL`, `flags=0` — full boot, `js_freestate` | [x] |
| 36 | `js_newstate` | `alloc=NULL`, `flags=JS_STRICT` — strict global code | [x] |
| 37 | `js_newstate` | custom `js_Alloc` (counting allocator) | [x] |
| 38 | `js_setlimit` | `runlimit>0` small (instruction budget exhausted) | [x] |
| 39 | `js_setlimit` | `memlimit>0` small (allocation cap hit) | [x] |
| 40 | `js_gc` | `report=0` after allocating garbage; `js_gc` repeated | [x] |
| 41 | `js_dostring` | corpus of valid scripts, non-strict state | [x] |
| 42 | `js_dostring` | corpus of valid scripts, `JS_STRICT` state | [x] |
| 43 | `js_ploadstring` + `js_pcall` | compile-then-call, result stringified | [x] |
| 44 | `js_ploadstring` + `js_pcall` | strict state | [x] |
| 45 | `js_pconstruct` | constructor invocation with 0/1/many args | [x] |
| 46 | `js_loadstring`/`js_eval` inside `js_try` | eval of expression source | [x] |
| 47 | push/pop/stack ops | `js_pushundefined/null/boolean/number/string/lstring/literal` then `js_type`,`js_typeof`,`js_tostring`,`js_tonumber`,`js_toboolean` | [x] |
| 48 | `js_pushlstring` | n = 0, 1, 7, 8, 15, 16, 64; embedded NUL bytes | [x] |
| 49 | `js_copy`/`js_remove`/`js_insert`/`js_replace`/`js_rot`/`js_pop` | positive and negative indices, deep stacks | [x] |
| 50 | `js_dup`/`js_dup2`/`js_rot2`/`js_rot3`/`js_rot4`/`js_rot2pop1`/`js_rot3pop2` | full stack-shuffle sequences | [x] |
| 51 | `js_toint32`/`touint32`/`toint16`/`touint16`/`tointeger` | via stack, on numbers/strings/booleans/null/undefined | [x] |
| 52 | `js_trystring`/`trynumber`/`tryinteger`/`tryboolean`/`tryrepr` | coercible and throwing values (getter that throws) | [x] |
| 53 | `js_newobject`/`newobjectx`/`newarray`/`newboolean`/`newnumber`/`newstring` | then `js_torepr` | [x] |
| 54 | `js_defproperty`/`js_defglobal` | atts = all 8 combinations of READONLY/DONTENUM/DONTCONF | [x] |
| 55 | `js_defaccessor` | getter only / setter only / both, atts combos | [x] |
| 56 | `js_hasproperty`/`getproperty`/`setproperty`/`delproperty` | own / inherited / absent names, array indices as names | [x] |
| 57 | `js_getlength`/`js_setlength`/`js_hasindex`/`js_getindex`/`js_setindex`/`js_delindex` | len 0/1/many, index in range / past end / negative | [x] |
| 58 | `js_pushiterator` + `js_nextiterator` | `own=0` and `own=1`, on object / array / string / with DONTENUM props | [x] |
| 59 | `js_newregexp` | all 8 combinations of `JS_REGEXP_G/I/M`; then `exec`/`test`/`String.replace` | [x] |
| 60 | `js_RegExp_prototype_exec` | called as C function through the interpreter with `g` flag (lastIndex state) | [x] |
| 61 | `js_newcfunction`/`newcfunctionx`/`newcconstructor` | length 0/1/n, with/without data+finalize; call and construct | [x] |
| 62 | `js_newuserdata`/`newuserdatax` | tag match / mismatch, with has/put/delete/finalize hooks | [x] |
| 63 | `js_ref`/`js_unref`/`js_getregistry`/`js_setregistry`/`js_delregistry` | many refs, GC between | [x] |
| 64 | `js_concat`/`js_equal`/`js_strictequal`/`js_compare`/`js_instanceof` | full cross-product of value types | [x] |
| 65 | `js_is*` predicates (20 of them) | full cross-product of value types incl. wrapper objects | [x] |
| 66 | `js_repr`/`js_torepr`/`js_tryrepr` | primitives, arrays, nested objects, cycles, functions, regexps | [x] |
| 67 | JSON | `JSON.parse` corpus (all literal shapes) via `jsY_lexjson` path | [x] |
| 68 | JSON | `JSON.stringify` with replacer function / array / undefined, indent number / string / none | [x] |
| 69 | builtins | `Object.*`, `Array.prototype.*` on flat and sparse arrays | [x] |
| 70 | builtins | `String.prototype.*` on ASCII / UTF-8 / empty strings | [x] |
| 71 | builtins | `Number.prototype.toString/toFixed/toExponential/toPrecision` radix 2..36, digits 0..20 | [x] |
| 72 | builtins | `Math.*` over randomized doubles and special values | [x] |
| 73 | builtins | `Date` parsing/formatting (UTC-only fields to stay deterministic) | [x] |
| 74 | builtins | `encodeURI*`/`decodeURI*`, `escape`/`unescape`, `parseInt`/`parseFloat` | [x] |
| 75 | `js_toprimitive` | hint NONE / NUMBER / STRING on objects with valueOf/toString | [x] |
| 76 | array internals | `jsV_resizearray` grow/shrink, `jsR_unflattenarray` on simple arrays | [x] |
| 77 | `js_setreport` + `js_atpanic` | custom report callback receives identical messages | [x] |
| 78 | `js_trap` | disassembly / stack dump output to stdout | [x] |
| 79 | closures & scopes | `jsR_newenvironment` via nested functions, `with`, `try/catch` scopes | [x] |
| 80 | `js_savetry`/`js_savetrypc`/`js_endtry`/`js_throw` | nested try depth 1..JS_TRYLIMIT | [x] |


## Where each row is verified

| rows | test file |
|------|-----------|
| 1-11 | `tests/b_utf.rs` |
| 12-23 | `tests/b_numeric.rs` |
| 24-26 (+ lexer/JSON token shapes) | `tests/b_lex.rs` |
| 27-34 | `tests/b_regexp.rs` |
| 35-40, 77, 79, 80 | `tests/b_state.rs` |
| 40b-40e, 78 | `tests/b_stdout.rs` (run with `--test-threads=1`) |
| 41-46 | `tests/b_eval.rs` |
| 47-53 | `tests/b_stack.rs` |
| 54-60, 62, 76 | `tests/b_props.rs` |
| 61, 63-75 | `tests/b_builtins.rs` |
| every limit/threshold above (JS_ASTLIMIT, JS_TRYLIMIT, JS_STRLIMIT, JS_ARRAYLIMIT, JS_ENVLIMIT, JS_STACKSIZE, REG_MAX*) | `tests/b_limits.rs` |

All rows are driven through the `.so` export tables of BOTH libraries with
`libloading`; no Rust function is ever called directly.  Randomised rows use the
fixed-seed `Rng` in `tests/common/mod.rs`, so every run is reproducible.

## Notes on domains excluded as undefined behaviour in the C

These are inputs the C original does not define, so they are not comparable:

| entry point | excluded domain | why |
|-------------|-----------------|-----|
| `js_fmtexp` | `|e| > 999999999` | formats into `char se[9]`; a 10-digit exponent overflows that stack buffer |
| `js_grisu2` | `±0.0` | trips `assert(x.f >= y.f)`; the sole caller guards `f == 0` |
| `js_strtol` | `base > 80` | the digit table's "invalid" marker is 80, so the NUL terminator becomes a digit and the loop runs off the buffer |
| `js_pushlstring` | `n < 0` | `while (n--)` copies ~2^32 bytes |
| `js_copy`, `js_pop` | indices below the current frame base | `stackidx()` bounds-checks against 0, not `BOT`, so it reads never-initialised value-stack slots |
| `js_rot` | `n` greater than the live frame | completely unchecked walk |
