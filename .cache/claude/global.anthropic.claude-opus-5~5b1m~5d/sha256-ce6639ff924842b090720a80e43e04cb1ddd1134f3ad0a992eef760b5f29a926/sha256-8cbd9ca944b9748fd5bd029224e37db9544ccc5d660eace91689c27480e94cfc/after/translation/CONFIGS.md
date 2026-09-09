# CONFIGS.md — configuration surface (valid inputs)

Derived mechanically from the C sources: every runtime option the public API can
set (`c_src/include/mujs.h`), every internal entry point exported by the `.so`
(`c_src/src/jsi.h`, `c_src/src/regexp.h`, `c_src/src/utf.h`), and every input
*shape* the C code branches on.

## Axes the C code actually branches on

| axis | values the C distinguishes | where |
|---|---|---|
| `js_newstate` `alloc` | `NULL` → `js_defaultalloc`, custom allocator + `actx` | jsstate.c:194 |
| `js_newstate` `flags` | `0`, `JS_STRICT`(1), unknown bits (only bit 0 tested) | jsstate.c:204 |
| limits | `runlimit==0` (off) / `>0` (counts down, `==1` → error), `memlimit==0` (off) / `>0` (subtracted per alloc) | jsrun.c:46-72, 1602 |
| report / panic | default `js_defaultreport` (stderr) vs `js_setreport`, `js_defaultpanic` (abort) vs `js_atpanic` | jsstate.c:211-212 |
| user context | `js_setcontext` / `js_getcontext` | jsstate.c |
| string representation | short string (≤ 8 bytes, inline `u.shrstr`), literal (`u.litstr`, `js_pushliteral`), heap `js_String` (`u.memstr`), interned literal | jsvalue.c, jsintern.c |
| value type | undefined, null, boolean, number, short/lit/mem string, object | jsi.h `js_Type` |
| object class | 12 `js_Class` values (object, array, function, script, cfunction, error, boolean, number, string, regexp, date, math, json, userdata) | jsi.h `js_Class` |
| array shape | *simple/flat* array (`u.a.simple`, sequential `0..len-1`) vs unflattened property tree (`jsR_unflattenarray`, non-sequential or named key) | jsrun.c:673, jsvalue.c |
| property attributes | `0`, `JS_READONLY`, `JS_DONTENUM`, `JS_DONTCONF`, and combinations; getter/setter (`js_defaccessor`) | jsproperty.c, jsobject.c |
| iterator | `own == 0` (walk prototype chain) vs `own == 1` | jsvalue.c `jsV_newiterator` |
| userdata | `js_newuserdata` (tag+finalize) vs `js_newuserdatax` (has/put/delete callbacks, any of them NULL) | jsobject.c |
| regexp flags | `JS_REGEXP_G/I/M` (1/2/4) all 8 combos; `REG_ICASE`/`REG_NEWLINE` compile, `REG_NOTBOL` exec | jsregexp.c, regexp.c |
| `regexec` sub | `sub == NULL` vs `Resub*` (nsub 0..16) | regexp.c |
| `regcompx` alloc | default `regcomp` vs custom allocator | regexp.c |
| number shape | ±0, ±Inf, NaN, integral, fractional, subnormal, 1e21 boundary (exponential switch), ≥1e-6 boundary, INT_MIN/INT_MAX, > 2^32 | jsdtoa.c, jsvalue.c |
| radix | `js_strtol` radix 2..36, 0, 1, 16 with `0x` prefix; `Number.prototype.toString(radix)` | jsdtoa.c, jsnumber.c |
| lexer mode | `jsY_lex` (JS) vs `jsY_lexjson` (JSON) | jslex.c |
| compile mode | `jsC_compilescript(default_strict=0/1)`, `jsC_compilefunction`, `jsP_parse` vs `jsP_parsefunction` | jscompile.c, jsparse.c |
| entry point | `js_dostring`, `js_ploadstring`+`js_pcall`, `js_loadstring`+`js_eval`, `js_call`, `js_construct`, `js_pconstruct`, `js_loadeval` | jsrun.c, jsstate.c |
| GC | `js_gc(report=0)` vs `js_gc(report=1)` (prints), GC triggered by allocation pressure | jsgc.c |
| string table | `js_intern`, `jsS_dumpstrings`, `jsS_freestrings` | jsintern.c |

## Rows (one per meaningful combination)

| # | entry point(s) | configuration (options set + input shape) | [x] test |
|---|----------------|--------------------------------------------|-----|
| 1 | `js_newstate`/`js_freestate` | default alloc, flags=0 | [x] `state.rs::cfg01` |
| 2 | `js_newstate`/`js_freestate` | default alloc, flags=JS_STRICT | [x] `state.rs::cfg02` |
| 3 | `js_newstate` | custom `js_Alloc` + actx, flags=0; allocator call trace compared | [x] `state.rs::cfg03` |
| 4 | `js_newstate` | flags = unknown bits (2, 0xFF, -1) | [x] `state.rs::cfg04` |
| 5 | `js_setcontext`/`js_getcontext` | set then read back, NULL and non-NULL | [x] `state.rs::cfg05` |
| 6 | `js_setreport` + `js_report` | custom report callback receives message | [x] `state.rs::cfg06` |
| 7 | `js_atpanic` | returns previous panic fn; custom panic installed | [x] `state.rs::cfg07` |
| 8 | `js_gc` | report=0 and report=1 on empty state and after allocating objects | [x] `state.rs::cfg08` |
| 9 | `js_setlimit` | runlimit>0 hit / not hit during script | [x] `state.rs::cfg09` |
| 10 | `js_setlimit` | memlimit>0 hit / not hit; `js_malloc`/`js_realloc` under memlimit | [x] `state.rs::cfg10` |
| 11 | push/`js_type`/`js_typeof` | all 7 value kinds (undefined, null, boolean, number, string, function, object) | [x] `values.rs::cfg11` |
| 12 | `js_pushnumber` + `js_tostring` | random doubles + specials (±0, ±Inf, NaN, 1e21, 1e-7, subnormal, huge) | [x] `numbers.rs::cfg12` |
| 13 | `js_pushstring`/`js_pushlstring`/`js_pushliteral` | short (≤8), long, empty, UTF-8, embedded NUL via lstring, n=0 | [x] `values.rs::cfg13` |
| 14 | `js_toboolean`/`js_tonumber`/`js_tostring` | cross product over all value kinds | [x] `values.rs::cfg14` |
| 15 | `js_tointeger`/`js_toint32`/`js_touint32`/`js_toint16`/`js_touint16` | random doubles + boundary values | [x] `numbers.rs::cfg15` |
| 16 | `jsV_numbertointeger`/`int32`/`uint32`/`int16`/`uint16` | direct low-level calls, random + boundary doubles | [x] `numbers.rs::cfg16` |
| 17 | `jsV_numbertostring` | random + boundary doubles into 32-byte buffer | [x] `numbers.rs::cfg17` |
| 18 | `jsV_stringtonumber`/`js_stringtofloat`/`js_strtod` | numeric strings: decimal, hex, exp, whitespace, Infinity, garbage | [x] `numbers.rs::cfg18` |
| 19 | `js_strtol` | radix 0,1,2,8,10,16,36 and out-of-range radix; sign, prefix, overflow | [x] `numbers.rs::cfg19` |
| 20 | `js_grisu2`+`js_fmtexp`+`js_itoa` | random doubles; exponents -350..350; ints incl. INT_MIN/INT_MAX | [x] `numbers.rs::cfg20` |
| 21 | `js_trystring`/`js_trynumber`/`js_tryinteger`/`js_tryboolean`/`js_tryrepr` | coercible and non-coercible (undefined/null/throwing toString) values | [x] `values.rs::cfg21` |
| 22 | `js_compare` | number/number, string/string, mixed, NaN (okay flag), objects | [x] `values.rs::cfg22` |
| 23 | `js_equal`/`js_strictequal` | cross product of value kinds incl. NaN, ±0, string vs number | [x] `values.rs::cfg23` |
| 24 | `js_instanceof` | function prototype match / mismatch / non-object | [x] `values.rs::cfg24` |
| 25 | `js_concat` | string+string, number+number, string+object, object+object | [x] `values.rs::cfg25` |
| 26 | `js_repr`/`js_torepr` | all value kinds, nested objects/arrays, cyclic object | [x] `values.rs::cfg26` |
| 27 | stack ops | `js_pop`,`js_rot`,`js_copy`,`js_remove`,`js_insert`,`js_replace` with positive and negative idx | [x] `values.rs::cfg27` |
| 28 | stack ops | `js_dup`,`js_dup2`,`js_rot2`,`js_rot3`,`js_rot4`,`js_rot2pop1`,`js_rot3pop2` and `js_gettop` after each | [x] `values.rs::cfg28` |
| 29 | `js_newobject`/`js_newobjectx` | plain object, then property set/get | [x] `objects.rs::cfg29` |
| 30 | `js_newarray` + `js_setindex` | flat/simple array: sequential 0..n-1 (stays simple) | [x] `objects.rs::cfg30` |
| 31 | `js_newarray` + `js_setindex` | sparse (skip indices) → unflatten path | [x] `objects.rs::cfg31` |
| 32 | `js_newarray` + `js_setproperty` | named property on array → unflatten path | [x] `objects.rs::cfg32` |
| 33 | `js_getlength`/`js_setlength` | grow, shrink, 0, large length on flat and unflattened arrays | [x] `objects.rs::cfg33` |
| 34 | `jsV_resizearray`/`jsR_unflattenarray` | direct low-level calls on flat array objects | [x] `objects.rs::cfg34` |
| 35 | `js_hasindex`/`js_getindex`/`js_delindex` | idx 0, middle, past end, large | [x] `objects.rs::cfg35` |
| 36 | `js_defproperty` | atts 0 / READONLY / DONTENUM / DONTCONF / all, then set+enumerate | [x] `objects.rs::cfg36` |
| 37 | `js_defaccessor` | getter+setter, getter only, setter only, with DONTENUM | [x] `objects.rs::cfg37` |
| 38 | `js_hasproperty`/`js_getproperty`/`js_setproperty`/`js_delproperty` | own, inherited, missing names | [x] `objects.rs::cfg38` |
| 39 | `jsV_getownproperty`/`jsV_getproperty`/`jsV_getpropertyx`/`jsV_setproperty`/`jsV_delproperty` | low-level on object with tree properties and on flat array | [x] `objects.rs::cfg39` |
| 40 | `jsV_newobject` | each `js_Class` value with/without prototype | [x] `objects.rs::cfg40` |
| 41 | `js_pushiterator`/`js_nextiterator` | own=0 vs own=1 on object with inherited + DONTENUM props, and on array | [x] `objects.rs::cfg41` |
| 42 | `jsV_newiterator`/`jsV_nextiterator` | low-level, own=0/1 | [x] `objects.rs::cfg42` |
| 43 | `js_getglobal`/`js_setglobal`/`js_defglobal`/`js_delglobal` | atts combos; existing and missing names | [x] `objects.rs::cfg43` |
| 44 | `js_getregistry`/`js_setregistry`/`js_delregistry` | set/get/delete/missing | [x] `objects.rs::cfg44` |
| 45 | `js_ref`/`js_unref` | ref a value, read it back through registry, unref | [x] `objects.rs::cfg45` |
| 46 | `js_newcfunction` | callback invoked via `js_call` with 0/1/many args | [x] `functions.rs::cfg46` |
| 47 | `js_newcfunctionx` | data + finalize; `js_currentfunction`/`js_currentfunctiondata` inside call | [x] `functions.rs::cfg47` |
| 48 | `js_newcconstructor` | called as function and via `js_construct`/`js_pconstruct` | [x] `functions.rs::cfg48` |
| 49 | `js_newuserdata` | tag match/mismatch for `js_isuserdata`/`js_touserdata`; finalize on gc | [x] `functions.rs::cfg49` |
| 50 | `js_newuserdatax` | has/put/delete callbacks present and NULL; property access through them | [x] `functions.rs::cfg50` |
| 51 | `js_newboolean`/`js_newnumber`/`js_newstring` | wrapper objects + `js_is*object` predicates | [x] `objects.rs::cfg51` |
| 52 | `js_newerror` + `js_new*error` (7 kinds) | each error constructor + `js_iserror` | [x] `objects.rs::cfg52` |
| 53 | `js_newregexp` | all 8 flag combos (G/I/M) + `js_isregexp`/`js_toregexp` | [x] `regexp.rs::cfg53` |
| 54 | `js_RegExp_prototype_exec` | low-level exec on regexp object with/without global flag, lastIndex advance | [x] `regexp.rs::cfg54` |
| 55 | `js_regcomp`/`js_regexec`/`js_regfree` | flags {0,ICASE,NEWLINE,both} × eflags {0,NOTBOL} × sub{NULL,Resub} × many patterns | [x] `regexp.rs::cfg55` |
| 56 | `js_regcompx`/`js_regfreex` | custom allocator | [x] `regexp.rs::cfg56` |
| 57 | `js_dostring` | non-strict; scripts covering all language features (corpus `dtest/js/*.js`) | [x] `scripts.rs::cfg57` |
| 58 | `js_dostring` | strict state (JS_STRICT) over the same corpus | [x] `scripts.rs::cfg58` |
| 59 | `js_ploadstring`+`js_pcall` | valid script, `n` = 0 args, return value converted | [x] `functions.rs::cfg59` |
| 60 | `js_loadstring`+`js_eval` | valid script, result on stack | [x] `functions.rs::cfg60` |
| 61 | `js_loadeval` | eval-scope compile of a source string | [x] `functions.rs::cfg61` |
| 62 | `js_call`/`js_pcall` | JS function, C function, method with `this`, n = 0/1/5 | [x] `functions.rs::cfg62` |
| 63 | `js_construct`/`js_pconstruct` | JS constructor, built-in constructor, n args | [x] `functions.rs::cfg63` |
| 64 | `jsP_parse`+`jsC_compilescript(0)`+`jsP_freeparse` | low-level pipeline, non-strict | [x] `lexparse.rs::cfg64` |
| 65 | `jsP_parse`+`jsC_compilescript(1)` | low-level pipeline, default_strict=1 | [x] `lexparse.rs::cfg65` |
| 66 | `jsP_parsefunction`+`jsC_compilefunction` | params + body strings, 0/1/many params | [x] `lexparse.rs::cfg66` |
| 67 | `jsY_initlex`+`jsY_lex` | token stream over sources: all token kinds, regexp/div ambiguity, comments, line terminators | [x] `lexparse.rs::cfg67` |
| 68 | `jsY_initlex`+`jsY_lexjson` | JSON token stream: strings, numbers, punctuation, escapes | [x] `lexparse.rs::cfg68` |
| 69 | `jsY_iswhite`/`isnewline`/`ishex`/`tohex` | c = 0..0x3001 sweep | [x] `utf.rs::cfg69` |
| 70 | `jsY_tokenstring` | every token id 0..400 incl. out-of-range | [x] `utf.rs::cfg70` |
| 71 | `jsY_findword` | word in list / not in list / empty list | [x] `utf.rs::cfg71` |
| 72 | `jsU_chartorune`/`runetochar`/`runelen` | all runes 0..0x110000 step + invalid/truncated UTF-8 byte sequences | [x] `utf.rs::cfg72+cfg72b` |
| 73 | `jsU_isalpharune`/`islowerrune`/`isupperrune`/`tolowerrune`/`toupperrune` | rune sweep 0..0x11000 | [x] `utf.rs::cfg73` |
| 74 | `jsU_tolowerrune_full`/`jsU_toupperrune_full` | runes with multi-char case mappings | [x] `utf.rs::cfg74` |
| 75 | `js_utflen`/`js_utfptrtoidx`/`js_runeat` | ASCII, multi-byte, invalid sequences, index 0/mid/end | [x] `utf.rs::cfg75` |
| 76 | `js_intern` + `jsS_dumpstrings` + `jsS_freestrings` | interning many strings (tree rebalance), dump output | [x] `lexparse.rs::cfg76` |
| 77 | `js_malloc`/`js_realloc`/`js_free`/`js_strdup` | plain, and with memlimit set | [x] `lexparse.rs::cfg77` |
| 78 | `js_putc`/`js_puts`/`js_putm` | buffer growth across the 512-byte doubling boundary | [x] `lexparse.rs::cfg78` |
| 79 | `js_savetry`/`js_endtry` + `js_throw` | try body completes; try body throws and is caught | [x] `functions.rs::cfg79` |
| 80 | `js_savetrypc` | try with pc (internal) | [x] `functions.rs::cfg80` |
| 81 | `js_trap` | stack/environment dump to stdout | [x] `functions.rs::cfg81` |
| 82 | `jsB_init` and each `jsB_init*` | individually on a bare state; `jsB_propf`/`propn`/`props` | [x] `functions.rs::cfg82` |
| 83 | `js_newarguments`/`js_newfunction`/`js_newscript` | internal function object creation from compiled function | [x] `functions.rs::cfg83` |
| 84 | `jsR_newenvironment` | new environment with variables object and outer scope | [x] `functions.rs::cfg84` |
| 85 | `jsV_newmemstring`/`js_pushvalue`/`js_pushobject`/`js_tovalue` | raw value/string plumbing | [x] `values.rs::cfg85` |
| 86 | `js_toprimitive`/`jsV_toprimitive` | hint 0 (none), 1 (number), 2 (string) on objects with valueOf/toString | [x] `values.rs::cfg86` |
| 87 | `jsV_toboolean`/`tonumber`/`tointeger`/`tostring`/`toobject` | low-level over all value kinds | [x] `values.rs::cfg87` |
| 88 | `js_toobject`/`js_isarrayindex` | primitives → wrapper objects; index strings "0","-1","4294967295","01" | [x] `values.rs::cfg88` |
| 89 | binary driver (`dtest/driver.c`) | `-api`, `-lowlevel`, `-regexp`, `-ctx` modes; corpus × {plain, strict, dumpstrings, limit, memlimit} | [x] `scripts.rs::cfg89` |
| 90 | Date built-ins | fixed TZ; `Date.UTC`, parsing, formatting, getters/setters | [x] `scripts.rs::cfg90` |
| 91 | JSON built-ins | parse/stringify with replacer/reviver/indent (string and number space) | [x] `scripts.rs::cfg91` |
| 92 | Math built-ins | all functions over random doubles + specials | [x] `scripts.rs::cfg92` |
| 93 | String built-ins | replace with function/`$` patterns, split with regexp/limit, slice/substr negatives, localeCompare, case mapping | [x] `scripts.rs::cfg93` |
| 94 | Array built-ins | sort with/without comparator, splice, concat, join on sparse and flat arrays | [x] `scripts.rs::cfg94` |

## Coverage

Every row is exercised by the named test in `translation/tests/`, which calls
BOTH libraries only through their `.so` exports (loaded with `libloading`) in
separate forked children and asserts byte-identical stdout+stderr and identical
exit status (`tests/common/mod.rs::diff`). Rows whose behaviour is
value-dependent use property-style loops over hundreds to thousands of
fixed-seed random inputs (`common::Rng`), not single hand-picked values.

Feature configurations: the crate declares **no** `[features]`, so
`--no-default-features` and the default build are the same code. Both were built
and the whole suite run under each; all 16 test binaries pass in both.

Full-suite result (`cargo build --release && cargo test --release -- --test-threads=1`):
368 tests, 0 failures — errors_1 64, errors_2 56, errors_3 70, errors_4 67,
evalorder 4, functions 16, lexparse 8, numbers 9, objects 19, regexp 8,
scripts 8, smoke 3, state 10, utf 8, values 15, variadic_errors 3.
All 237 exported symbols are called through `dlsym` by these tests (the 7
variadic `js_*error` entry points and the internal `jsC_error` are covered by
`tests/variadic_errors.rs`, which also asserts the runs are non-vacuous).

Row 89 additionally compares the two BINARIES (`dtest/driver_c` vs
`dtest/driver_r`, the same C driver linked against each library) byte-for-byte
on the `-api`, `-lowlevel`, `-regexp`, `-ctx` modes and on corpus files with
`strict` / `dumpstrings` / `limit` / `memlimit`, and runs the whole 485-case
`dtest/run.sh` suite. Note that `LD_LIBRARY_PATH` must be pinned per driver
(the C `.so` has `SONAME libmujs.so`), otherwise `driver_c` silently loads the
Rust library and the comparison becomes a tautology.

### Bug found and fixed by these tests

`js_setproperty` / `js_setindex` (`translation/src/jsrun.rs`): C evaluates
`jsR_setproperty(J, js_toobject(J, idx), name, !js_isobject(J, idx))`
right-to-left, so the transient flag is computed BEFORE `js_toobject` boxes the
primitive in place. The Rust translation evaluated left-to-right, so
`transient` was always 0 and the "cannot create property '%s' on transient
object" TypeError never fired. Fixed by materialising the flag first;
`tests/evalorder.rs` and `errors_3.rs::err160_transient_object` pin it (verified
to fail against the pre-fix ordering).
