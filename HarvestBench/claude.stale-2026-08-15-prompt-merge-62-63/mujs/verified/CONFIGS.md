# CONFIGS.md — configuration surface table (Phase A)

Derived mechanically from `c_src/include/mujs.h` (public API), `c_src/src/jsi.h`
(internal API + enums + limits), `c_src/src/regexp.h`, `c_src/src/utf.h` and the
`if` / `switch` branches the C actually takes on each flag.

## Build-time configuration

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` has no
`option()` / `-D` switches (it globs `src/*.c`, one `SHARED` target, `-w`).
Therefore the complete set of valid feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | *(default = empty feature set)* | `cargo test --no-default-features` (identical to `cargo test`) |

There is no binary/driver target: `regexp.c`'s `main()` is behind `#ifdef TEST`,
which CMake never defines, and `[lib] crate-type = ["cdylib"]` is the only Rust
target. So the "compare C and Rust binary stdout" gate is N/A.

## Runtime configuration axes (from the C source)

| axis | values the C branches on | where |
|------|--------------------------|-------|
| `js_newstate` flags | `0`, `JS_STRICT`(1), and out-of-range bits (e.g. `2`, `-1`, `0x7fffffff`) — only bit 0 is read: `if (flags & JS_STRICT) J->strict = J->default_strict = 1` | jsstate.c:196 |
| `js_newstate` alloc | `NULL` (→ `js_defaultalloc` = realloc/free), custom allocator, allocator that fails | jsstate.c:180-190 |
| `js_setlimit` | `runlimit` 0 (off) / 1 / N; `memlimit` 0 (off) / small / large | jsrun.c:46-76, 1602 |
| report callback | default (`fputs` to stderr) or user `js_Report` | jsstate.c:24, 160 |
| panic handler | default (`js_report "uncaught exception"`) or user `js_Panic` | jsstate.c:29, 152 |
| script vs eval | `js_loadstring` (uses `J->default_strict`) vs `js_loadeval` (uses `J->strict`, inherits env `J->E` when strict) | jsstate.c:110-125 |
| property attributes | cross product of `JS_READONLY`(1) `JS_DONTENUM`(2) `JS_DONTCONF`(4) = 8 | mujs.h, jsrun.c jsR_defproperty |
| RegExp flags (JS level) | cross product of `JS_REGEXP_G`(1) `JS_REGEXP_I`(2) `JS_REGEXP_M`(4) = 8 | jsregexp.c |
| `regcomp` cflags | cross product `REG_ICASE`(1) `REG_NEWLINE`(2) = 4, plus out-of-range bits | regexp.c regcompx |
| `regexec` eflags | `0`, `REG_NOTBOL`(4), plus out-of-range bits | regexp.c regexec |
| `js_pushiterator` own | `0` (walk prototype chain) / `1` (own properties only) | jsproperty.c jsV_newiterator |
| userdata kind | `js_newuserdata` (finalize only) vs `js_newuserdatax` (has/put/delete/finalize) | jsvalue.c |
| cfunction kind | `js_newcfunction` / `js_newcfunctionx` (data+finalize) / `js_newcconstructor` | jsvalue.c |
| string representation | `JS_TSHRSTR` (≤ 15 bytes, inline), `JS_TLITSTR` (interned/literal), `JS_TMEMSTR` (heap, refcounted) | jsi.h:302, jsrun.c js_pushstring/js_pushlstring/js_pushliteral |
| object class | 16 values of `enum js_Class` (`JS_COBJECT`..`JS_CUSERDATA`) | jsi.h:313 |
| value type | 8 values of `enum js_Type` | jsi.h:302 |
| array representation | flat/`simple` (`u.a.array`) vs unflattened property list (`jsV_unflattenarray`); `flat_length` vs `length`; sparse | jsproperty.c, jsrun.c jsR_setarrayindex |
| number shape | `+0`, `-0`, NaN, ±Inf, integers, denormals, `1e21` boundary (`jsV_numbertostring` switch), radix 2..36 | jsvalue.c, jsdtoa.c |
| UTF-8 shape | 1/2/3/4-byte sequences, invalid lead/continuation bytes, overlong forms, surrogates, `Runemax` | utf.c |
| `js_gc` report | `0` (silent) / non-zero (prints stats with `printf`) | jsgc.c |
| `TZ` environment | affects `LocalTZA` / `DaylightSavingTA` (`localtime`/`mktime`) | jsdate.c |
| JSON options | `JSON.parse` with/without reviver; `JSON.stringify` with replacer function / replacer array / `undefined`; indent number 0..10 / string / `undefined` | json.c |

## Status

**Every row below is checked off**, in the debug *and* release profiles.
Reproduce everything with `./run_tests.sh` (which also loops over the feature
combinations enumerated above — there is exactly one).

| section | what it covers | test file | `#[test]`s |
|---------|----------------|-----------|-----------|
| A | `utf.c` | `tests/utf.rs` | 13 |
| B | `regexp.c` | `tests/regexp.rs` | 10 |
| C | `jsdtoa.c` + number helpers | `tests/numbers.rs` | 10 (+1 `#[ignore]`d UB probe) |
| D | `jslex.c` helpers + token streams | `tests/api.rs` (D3, D5), `tests/errors_capi.rs` (D1, D2, D4) | 2 + 2 |
| E, F, G | embedding API, objects/properties/arrays, compile+run pipeline | `tests/api.rs` | 36 total |
| G16, G19 + stdout-only output | `js_gc(report)`, `js_trap`, `debugger`, `jsS_dumpstrings`, default report handler, `js_Buffer` | `tests/stdout.rs` | 6 |
| H | full-interpreter script corpus | `tests/scripts_core.rs`, `tests/scripts_lib.rs` | 10 + 13 |
| — | harness self-check | `tests/smoke.rs` | 3 |
| — | error paths (see `ERRORS.md`) | `tests/errors_js.rs`, `tests/errors_capi.rs`, `tests/errors_bigmem.rs` | 5 + 41 + 6 |

**153 tests, 0 failures.** The script corpus alone is **7 200+ distinct
JavaScript programs**, most run under both a default and a `JS_STRICT` state
(~16 500 `js_dostring` comparisons per library); the property-style rows add
~1.1 M randomized comparisons (utf: exhaustive over all 1.1 M code points and
every 1-3 byte sequence; regexp: 64 800 `regexec` calls plus a 20 000-case
pattern/subject fuzz; numbers: ~790 k value comparisons).

### How divergences are made visible

* Raw addresses can never match between two independently loaded libraries, so
  every comparison is reduced to address-independent data: `Resub` spans become
  integer offsets from the subject base, `js_trap` output has `0x<hex>` runs
  masked (and the *count* of masked tokens is compared separately), and object
  identity is compared via `js_typeof` / `js_type` / `js_repr` rather than
  pointers.
* `f64` results are compared by raw bits (`to_bits`), so `NaN` and `-0.0`
  divergences cannot hide.
* Output buffers are oversized and pre-filled with `0xAA` and compared whole, so
  stray writes are caught; returned pointers are compared as offsets.
* Uninitialised regions are deliberately *not* compared (see the exclusion table
  in `ERRORS.md`).

## Configuration rows to verify

Every row is exercised through the `.so` exports of BOTH libraries with many
randomized inputs (fixed seed) and the outputs compared byte-for-byte.

### A. `utf.c` — lowest level, no `js_State`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| A1 | `jsU_chartorune` | all 1-byte inputs `0x01..0x7F` (ASCII fast path) | [x] |
| A2 | `jsU_chartorune` | `0x00` (empty string / NUL terminator) | [x] |
| A3 | `jsU_chartorune` | every well-formed 2-byte sequence (`0xC0..0xDF` + cont) | [x] |
| A4 | `jsU_chartorune` | every well-formed 3-byte sequence, incl. surrogate range `D800..DFFF` | [x] |
| A5 | `jsU_chartorune` | every well-formed 4-byte sequence up to `Runemax` and past it | [x] |
| A6 | `jsU_chartorune` | invalid lead byte `0x80..0xBF` (bare continuation) | [x] |
| A7 | `jsU_chartorune` | truncated sequences (lead byte then NUL / then non-continuation) | [x] |
| A8 | `jsU_chartorune` | overlong encodings (`C0 80`, `E0 80 80`, `F0 80 80 80`) | [x] |
| A9 | `jsU_chartorune` | lead bytes `0xF8..0xFF` (5/6-byte / invalid) | [x] |
| A10 | `jsU_chartorune` | exhaustive sweep of all 3-byte and randomized 4-byte byte strings | [x] |
| A11 | `jsU_runetochar` | every rune `0..0x110000+` incl. negatives, `Runeerror`, `Runemax`, `Runemax+1` | [x] |
| A12 | `jsU_runelen` | every rune `-1000..0x110100` | [x] |
| A13 | `jsU_runetochar`→`jsU_chartorune` | round-trip over all runes | [x] |
| A14 | `jsU_isalpharune` / `islowerrune` / `isupperrune` | exhaustive `0..0x110000` (binary-search tables in utfdata.h) | [x] |
| A15 | `jsU_tolowerrune` / `jsU_toupperrune` | exhaustive `0..0x110000` incl. negatives | [x] |
| A16 | `jsU_tolowerrune_full` / `jsU_toupperrune_full` | exhaustive `0..0x110000`; multi-rune expansions (e.g. `U+00DF`, `U+FB00`, `U+0130`); returned array contents until 0 | [x] |

### B. `regexp.c` — standalone regexp engine

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B1 | `js_regcomp`+`js_regexec`+`js_regfree` | cflags 0, eflags 0, literal patterns, randomized subject strings | [x] |
| B2 | same | cflags `REG_ICASE`, eflags 0 | [x] |
| B3 | same | cflags `REG_NEWLINE`, eflags 0 (`^`/`$`/`.` semantics) | [x] |
| B4 | same | cflags `REG_ICASE\|REG_NEWLINE` | [x] |
| B5 | same | eflags `REG_NOTBOL` × each of the 4 cflag combos | [x] |
| B6 | same | out-of-range cflags/eflags bits (`8`, `-1`, `0x7fffffff`) | [x] |
| B7 | `js_regexec` | `sub == NULL` (no capture reporting) | [x] |
| B8 | `js_regexec` | `sub->nsub` smaller than `prog->nsub`; ≥ `REG_MAXSUB` groups | [x] |
| B9 | `js_regcomp` | character classes: ranges, negation, `\d\D\s\S\w\W`, `[]`, `[^]`, `-` at edges | [x] |
| B10 | `js_regcomp` | quantifiers `* + ? {n} {n,} {n,m}` and non-greedy variants | [x] |
| B11 | `js_regcomp` | alternation, nesting, capture groups, `(?:)`, `(?=)`, `(?!)`, back-references `\1..\9` | [x] |
| B12 | `js_regcomp` | escapes `\b \B \f \n \r \t \v \0 \xHH \uHHHH \cX`, identity escapes | [x] |
| B13 | `js_regcomp` | anchors `^ $ \b \B` in all cflag combos | [x] |
| B14 | `js_regcomp` | empty pattern `""` | [x] |
| B15 | `js_regcomp` | `errorp == NULL` on success and on failure | [x] |
| B16 | `js_regcompx`/`js_regfreex` | custom allocator (counting) — same results as `regcomp`/`regfree` | [x] |
| B17 | `js_regexec` | subject shapes: empty, ASCII, multi-byte UTF-8, embedded newlines, 4KB | [x] |
| B18 | `js_regexec` | randomized pattern × randomized subject fuzz (fixed seed, 20k cases) | [x] |

### C. `jsdtoa.c` / number formatting — no `js_State`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `js_strtod` | integers, decimals, exponents, leading/trailing space, sign, hex, `Inf`/`NaN` spellings, empty, garbage; `end` pointer position | [x] |
| C2 | `js_strtod` | overflow / underflow / denormal / `maxExponent` (511) path / >999-digit mantissa | [x] |
| C3 | `js_grisu2` | exhaustive random `f64` bit patterns; `+0`, `-0`, denormals, powers of two | [x] |
| C4 | `js_fmtexp` | exponent values −324..308 and 0 | [x] |
| C5 | `js_itoa` | all radices 2..36 × values `INT_MIN`, −1, 0, 1, `INT_MAX`, randomized | [x] |
| C6 | `js_strtol` | all bases 0/2..36, over/underflow clamping, `endptr` | [x] |
| C7 | `jsV_numbertostring` | random `f64`s + the `1e21` / `1e-6` / integer / NaN / ±Inf / ±0 branches | [x] |
| C8 | `jsV_stringtonumber` | numeric strings, hex `0x`, octal-looking, whitespace-only, `Infinity`, empty, junk | [x] |
| C9 | `js_stringtofloat` | same corpus as C8, `end` pointer | [x] |
| C10 | `jsV_numbertointeger`/`int32`/`uint32`/`int16`/`uint16` | NaN, ±Inf, ±0, fractional, 2^31, 2^32, 2^53, negatives, randomized | [x] |

### D. `jslex.c` character/token helpers

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| D1 | `jsY_ishex` / `jsY_tohex` | all `int` values `-1..0x110000` | [x] |
| D2 | `jsY_iswhite` / `jsY_isnewline` | all `int` values `-1..0x110000` (incl. `U+FEFF`, `U+2028/9`, `U+00A0`) | [x] |
| D3 | `jsY_findword` | every keyword, near-misses, empty string, case variants, over a supplied word list | [x] |
| D4 | `jsY_tokenstring` | every token id `0..TK_WITH` **and out-of-range ids** | [x] |
| D5 | `jsY_initlex`+`jsY_lex` | full token stream for a corpus of sources (all token kinds, both `jsY_lex` and `jsY_lexjson`) | [x] |

### E. Embedding API — stack, values, conversions (needs `js_State`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| E1 | `js_newstate` | flags `0`; default allocator | [x] |
| E2 | `js_newstate` | flags `JS_STRICT` | [x] |
| E3 | `js_newstate` | out-of-range flags `2`, `4`, `-1`, `0x7fffffff` | [x] |
| E4 | `js_newstate` | custom allocator (counting realloc wrapper) + `actx` round-trip | [x] |
| E5 | `js_setcontext`/`js_getcontext` | NULL and non-NULL | [x] |
| E6 | `js_atpanic` | returns previous handler; custom handler installed | [x] |
| E7 | push/pop family | `js_pushundefined/null/boolean/number/string/lstring/literal/global`, `js_gettop`, `js_pop` | [x] |
| E8 | `js_pushstring` | shrstr (0,1,7,15 bytes), 16-byte boundary → memstr, 1KB, embedded UTF-8 | [x] |
| E9 | `js_pushlstring` | `n` = 0, 1, 15, 16, with embedded NUL bytes, n < strlen | [x] |
| E10 | stack shuffling | `js_copy/js_remove/js_insert/js_replace/js_rot/js_dup/js_dup2/js_rot2/js_rot3/js_rot4/js_rot2pop1/js_rot3pop2` over randomized stacks, all valid idx (positive and negative) | [x] |
| E11 | type predicates | all 20 `js_is*` × every value type × every object class | [x] |
| E12 | conversions | `js_toboolean/tonumber/tostring/tointeger/toint32/touint32/toint16/touint16` × every value type × object classes | [x] |
| E13 | `js_typeof` / `js_type` | every value type and object class | [x] |
| E14 | `js_trystring/trynumber/tryinteger/tryboolean` | value that converts cleanly vs value whose conversion throws | [x] |
| E15 | `js_repr`/`js_torepr`/`js_tryrepr` | every value type, nested objects/arrays, cyclic, strings needing escapes | [x] |
| E16 | `js_compare` (with `okay`) / `js_equal` / `js_strictequal` / `js_instanceof` | cross product of representative value types (numbers, strings, NaN, objects, null/undefined) | [x] |
| E17 | `js_concat` | string+string, number+number, string+object, shrstr/memstr boundary crossing | [x] |

### F. Embedding API — objects, properties, arrays

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| F1 | `js_newobject`/`js_newobjectx`/`js_newarray` | fresh objects | [x] |
| F2 | `js_getproperty`/`setproperty`/`hasproperty`/`delproperty` | present / absent / inherited / accessor properties | [x] |
| F3 | `js_defproperty` | all 8 attribute combinations, then re-set / re-def / delete | [x] |
| F4 | `js_defaccessor` | getter only, setter only, both, all 8 attribute combos | [x] |
| F5 | `js_getglobal`/`setglobal`/`delglobal` | present / absent names | [x] |
| F6 | `js_defglobal` | all 8 attribute combinations | [x] |
| F7 | `js_getregistry`/`setregistry`/`delregistry` | round-trip, missing key | [x] |
| F8 | `js_ref`/`js_unref` | many refs (string ids), reuse after unref | [x] |
| F9 | `js_getindex`/`setindex`/`hasindex`/`delindex` | flat array (0..N), sparse (index 1e6), negative-looking, `2^31-1` | [x] |
| F10 | `js_getlength`/`js_setlength` | grow, shrink, 0, on a non-array object | [x] |
| F11 | `jsV_resizearray` (via `js_setlength`) | flat→bigger, flat→smaller, over `JS_ARRAYLIMIT` | [x] |
| F12 | array representation | dense/simple array vs unflattened (after adding a non-index property or a hole) | [x] |
| F13 | `js_pushiterator`+`js_nextiterator` | `own=0` and `own=1`, on plain object / array / string object / object with prototype chain / DONTENUM props | [x] |
| F14 | `js_newuserdata` | tag round-trip, `js_isuserdata`, `js_touserdata` with right and wrong tag, finalize called on GC | [x] |
| F15 | `js_newuserdatax` | `has`/`put`/`delete` callbacks returning 0 and 1 | [x] |
| F16 | `js_newcfunction` | called from JS with fewer/more args than `length` | [x] |
| F17 | `js_newcfunctionx` | `data` visible via `js_currentfunctiondata`, finalize on GC | [x] |
| F18 | `js_newcconstructor` | called as function and with `new` | [x] |
| F19 | `js_newboolean/newnumber/newstring/newerror*` | each of the 7 `js_new*error` constructors | [x] |
| F20 | `js_newregexp` | all 8 flag combos + invalid pattern | [x] |
| F21 | `jsV_newobject` (raw) | each of the 16 `js_Class` values + out-of-range class value | [x] |
| F22 | `jsV_getownproperty`/`getproperty`/`getpropertyx`/`setproperty`/`delproperty` | direct low-level calls: present/absent/inherited, `own` out-param | [x] |
| F23 | `jsV_newiterator`/`jsV_nextiterator` | direct low-level calls, `own` 0/1 | [x] |
| F24 | `jsV_toboolean/tonumber/tostring/tointeger/toobject/toprimitive` | direct low-level calls over every value type; `toprimitive` hint paths | [x] |

### G. Compile / run pipeline (low-level entry points, not just `js_dostring`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| G1 | `jsP_parse`+`jsC_compilescript`+`js_newscript`+`js_call` | non-strict, corpus of scripts (manual pipeline, mirroring `js_loadstringx`) | [x] |
| G2 | same | `strict = 1` argument to `jsC_compilescript` | [x] |
| G3 | `jsP_parsefunction`+`jsC_compilefunction` | `new Function(...)` shapes: 0/1/N params, body with/without return | [x] |
| G4 | `jsP_freeparse` | after successful parse and after a parse error | [x] |
| G5 | `js_loadstring` + `js_call` | script code (uses `default_strict`) | [x] |
| G6 | `js_loadeval` + `js_call` | eval code, non-strict state (global env) | [x] |
| G7 | `js_loadeval` + `js_call` | eval code, `JS_STRICT` state (inherits `J->E`) | [x] |
| G8 | `js_ploadstring` | valid source (returns 0) and invalid source (returns 1) | [x] |
| G9 | `js_dostring` | non-strict state, corpus of scripts; report string + captured `print` output | [x] |
| G10 | `js_dostring` | `JS_STRICT` state, same corpus | [x] |
| G11 | `js_pcall` / `js_pconstruct` | n = 0,1,N; callable and non-callable; function that throws | [x] |
| G12 | `js_call` / `js_construct` | direct (unprotected) calls inside a protected outer frame | [x] |
| G13 | `js_eval` | `eval` of a string pushed on the stack | [x] |
| G14 | `js_setlimit` | `runlimit` small (1, 2, 1000) → "too much computation" | [x] |
| G15 | `js_setlimit` | `memlimit` small → "out of memory" during compile and during run | [x] |
| G16 | `js_gc` | `report=0` and `report=1`, before/after allocating garbage; `js_freestate` | [x] |
| G17 | `jsB_init` + individual `jsB_init*` | each of the 11 `jsB_init*` entry points called on a bare state | [x] |
| G18 | `js_intern` / `jsS_dumpstrings` / `jsS_freestrings` | interning many strings, duplicates, dump output | [x] |
| G19 | `js_putc`/`js_puts`/`js_putm` (js_Buffer) | growing a buffer past its initial capacity, UTF-8 | [x] |

### H. Full-interpreter script corpus (drives the remaining branches)

Each row is a group of scripts run through **both** `js_dostring` (non-strict and
`JS_STRICT` state) with the report string and a captured `print` output compared.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| H1 | `js_dostring` | all operators: arithmetic, bitwise, shifts, comparison, logical, `typeof`, `void`, `delete`, `in`, `instanceof`, comma, ternary, `++`/`--` pre/post | [x] |
| H2 | `js_dostring` | all statements: `var`, `if/else`, `do/while`, `while`, `for`, `for-in`, `for-var-in`, `switch` (with/without default), `break`/`continue` (labelled and not), `return`, `throw`, `try/catch/finally` (all 3 shapes), `with`, `debugger`, empty | [x] |
| H3 | `js_dostring` | functions: declarations, expressions, named expressions, closures, recursion, `arguments`, `arguments.callee`, `.length`, nested scopes, hoisting | [x] |
| H4 | `js_dostring` | `Object` builtins: all of `getPrototypeOf`, `getOwnPropertyDescriptor`, `getOwnPropertyNames`, `defineProperty`, `defineProperties`, `create`, `keys`, `preventExtensions`, `isExtensible`, `seal`, `isSealed`, `freeze`, `isFrozen`, `hasOwnProperty`, `isPrototypeOf`, `propertyIsEnumerable`, `toString`, `valueOf` | [x] |
| H5 | `js_dostring` | `Array` builtins: all 24 prototype methods × empty / one / many / sparse / non-array `this`, and `Array()` / `Array(n)` / `Array(a,b,c)` / `isArray` | [x] |
| H6 | `js_dostring` | `Array.prototype.sort` with no comparator, with a comparator, with a throwing comparator, on 0/1/2/many/duplicate elements (heapsort path) | [x] |
| H7 | `js_dostring` | `String` builtins: all prototype methods × empty / ASCII / UTF-8 / surrogate strings; `String.fromCharCode`; `split` with string and regexp separators and limits; `replace` with string and function replacements and `$1 $& $` $' $$` patterns; `match`/`search` with and without `g` | [x] |
| H8 | `js_dostring` | `Number` builtins: `toString` radix 2..36, `toFixed`/`toExponential`/`toPrecision` widths 0..21, `Number()` conversions, `Number.MAX_VALUE`/`MIN_VALUE`/`NaN`/`±Infinity` | [x] |
| H9 | `js_dostring` | `Math`: all 18 functions × representative and random arguments; `Math.max`/`min` with 0/1/N args | [x] |
| H10 | `js_dostring` | `Boolean`, `Function.prototype.call/apply/bind/toString` (incl. bound-function `length` and construction) | [x] |
| H11 | `js_dostring` | `RegExp`: literal and constructor forms, all 8 flag combos, `exec`/`test`/`lastIndex` behaviour with and without `g`, `source`/`global`/`ignoreCase`/`multiline` props | [x] |
| H12 | `js_dostring` | `Date`: all getters/setters (local and UTC), `Date.UTC`, `Date.parse` on many formats, `toString`/`toISOString`/`toUTCString`/`toDateString`/`toTimeString`/`toJSON`, `Date(y,m,d,...)` with 1..7 args, invalid dates. Run under `TZ=UTC` and `TZ=America/New_York` | [x] |
| H13 | `js_dostring` | `JSON.parse` (all value shapes, nesting, with/without reviver) and `JSON.stringify` (replacer function / replacer array / none × indent `undefined` / 0 / 4 / `"\t"`, nested, cyclic, toJSON, non-serialisable values) | [x] |
| H14 | `js_dostring` | `Error` hierarchy: all 7 constructors, `message`/`name`/`stack`/`stackTrace`, `toString`, subclassing, throwing non-Errors | [x] |
| H15 | `js_dostring` | global functions: `parseInt` (all radices, prefixes), `parseFloat`, `isNaN`, `isFinite`, `encodeURI`/`decodeURI`/`encodeURIComponent`/`decodeURIComponent` (incl. multi-byte and reserved sets), `escape`/`unescape`, `eval`, `Function` | [x] |
| H16 | `js_dostring` | property enumeration order and shadowing over prototype chains; `for-in` with deletions during iteration | [x] |
| H17 | `js_dostring` | numeric/string literal lexing: all number formats (hex, octal-looking, exponents), string escapes (`\x`, `\u`, line continuations), regexp literals, comments, line terminators (`\r`, `\n`, ` `, ` `) | [x] |
| H18 | `js_dostring` | deep recursion / large literals: 100-deep nested objects, 1000-element array literal, long `if/else` chains, many locals | [x] |
| H19 | `js_dostring` | `arguments` object aliasing, `this` binding in strict vs non-strict, primitive `this` boxing | [x] |
| H20 | `js_dostring` | randomized expression fuzz: generated arithmetic/string expressions (fixed seed) compared for identical printed results | [x] |
