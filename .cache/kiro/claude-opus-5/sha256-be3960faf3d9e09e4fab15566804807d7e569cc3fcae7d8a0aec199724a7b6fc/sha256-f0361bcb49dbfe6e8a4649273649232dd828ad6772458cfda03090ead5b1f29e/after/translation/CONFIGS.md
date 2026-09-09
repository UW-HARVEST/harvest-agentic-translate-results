# CONFIGS.md — Configuration-surface table (derived mechanically from `c_src/`)

Axes were enumerated by grepping the public header for every runtime option, and the
C sources for every `if`/`switch` those options and input shapes drive:

```sh
grep -n 'enum {' -A8 c_src/include/mujs.h        # JS_STRICT, JS_REGEXP_[GIM], JS_READONLY/DONTENUM/DONTCONF, JS_IS*
grep -n 'J->strict|default_strict' c_src/src/*.c  # state-flag branch
grep -n 'REG_ICASE|REG_NEWLINE|REG_NOTBOL' c_src/src/regexp.c
grep -n 'u.a.simple|flat_length' c_src/src/*.c    # simple(flat) vs hashed array shape
grep -n 'JS_TSHRSTR|JS_TMEMSTR|JS_TLITSTR' c_src/src/*.h c_src/src/*.c  # 3 string representations
grep -n 'js_setlimit|runlimit|memlimit|gcthresh' c_src/src/*.c
grep -n 'chartorune|runetochar|Runeself|Runemax' c_src/src/utf.c        # 1/2/3/4-byte shapes
```

## Runtime option axes

| axis | values the C actually branches on | where |
|---|---|---|
| `js_newstate` flags | `0`, `JS_STRICT` (and out-of-range ints — only bit 0 read) | `jsstate.c:205` |
| `js_setlimit` | `(0,0)` = off, small `runlimit`, small `memlimit` | `jsstate.c`, `jsrun.c` |
| property attributes | cross-product of `JS_READONLY`/`JS_DONTENUM`/`JS_DONTCONF` (8) | `jsproperty.c` |
| regexp compile flags | `0`, `REG_ICASE`, `REG_NEWLINE`, `ICASE\|NEWLINE` | `regexp.c` |
| regexp exec flags | `0`, `REG_NOTBOL` | `regexp.c` |
| `js_newregexp` flags | cross-product `JS_REGEXP_G`/`_I`/`_M` (8) | `jsregexp.c` |
| `js_pushiterator` own | `0` (incl. prototype chain), `1` (own only) | `jsproperty.c:294` |
| script vs eval compile | `js_loadstring` (`default_strict`, global env) vs `js_loadeval` (`J->strict`, current env) | `jsstate.c:122` |
| `js_gc` report | `0`, `1` | `jsgc.c` |
| cfunction kind | `js_newcfunction`, `js_newcfunctionx` (data+finalize), `js_newcconstructor` | `jsfunction.c` |
| userdata kind | `js_newuserdata`, `js_newuserdatax` (has/put/del hooks) | `jsvalue.c` |

## Input-shape axes

| axis | values the C special-cases |
|---|---|
| string representation | shrstr (len ≤ 7, inline), memstr (heap, refcounted), litstr (borrowed `const char*`) |
| string content | empty, ASCII, 2/3/4-byte UTF-8, malformed UTF-8, embedded NUL via `js_pushlstring` |
| array shape | simple/flat (`u.a.simple==1`), converted-to-hashed (after sparse/`delete`/non-index key), empty, one, many |
| number shape | `+0`, `-0`, integer, fraction, `1e21` boundary (`numbertostring`), `inf`, `-inf`, `NaN`, `2^31`, `2^32`, denormals |
| integer coercions | `tointeger`/`toint32`/`touint32`/`toint16`/`touint16` on the above |
| stack depth | empty, 1, near `JS_STACKSIZE` (4096) |
| regexp captures | 0, 1, up to `REG_MAXSUB` (16), > 16 |

## Rows — one per configuration combination the C treats differently

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jsU_chartorune`/`jsU_runetochar`/`jsU_runelen` | full round-trip over runes `0..0x10FFFF` incl. surrogates + `Runemax`+1 ||| [x] |
| 2 | `jsU_chartorune` | randomized raw byte buffers (malformed, truncated, overlong) ||| [x] |
| 3 | `jsU_isalpharune`/`islowerrune`/`isupperrune`/`tolowerrune`/`toupperrune` | every rune `0..0x10FFFF` ||| [x] |
| 4 | `jsU_tolowerrune_full`/`jsU_toupperrune_full` | every rune `0..0x2FFFF` (multi-char case folding) ||| [x] |
| 5 | `js_utflen`/`js_utfptrtoidx`/`js_runeat` | ASCII / multibyte / malformed strings, index 0..len+1 ||| [x] |
| 6 | `js_itoa` | randomized `int` incl. `INT_MIN`, `0`, `-1` ||| [x] |
| 7 | `js_grisu2`/`js_fmtexp`/`jsV_numbertostring` | randomized doubles (uniform bits) ||| [x] |
| 8 | `jsV_numbertostring` | boundary doubles: `±0`, `1e-7`, `1e20`, `1e21`, `inf`, `NaN`, integers ||| [x] |
| 9 | `js_strtod`/`js_stringtofloat` | randomized numeric text + garbage prefixes/suffixes ||| [x] |
| 10 | `js_strtol` | randomized digits × base `2,8,10,16,36`, overflow inputs ||| [x] |
| 11 | `jsV_stringtonumber` | decimal / hex `0x` / octal-looking / whitespace / `Infinity` / garbage ||| [x] |
| 12 | `jsV_numbertointeger`/`_toint32`/`_touint32`/`_toint16`/`_touint16` | randomized doubles incl. out-of-range, `NaN`, `±inf` ||| [x] |
| 13 | `js_isarrayindex` | canonical indices, leading zeros, `2^32-2`, `2^32-1`, negatives, non-digits ||| [x] |
| 14 | `js_regcomp` + `js_regexec` | `cflags=0`, `eflags=0`, randomized patterns/subjects ||| [x] |
| 15 | `js_regcomp` + `js_regexec` | `cflags=REG_ICASE` ||| [x] |
| 16 | `js_regcomp` + `js_regexec` | `cflags=REG_NEWLINE` ||| [x] |
| 17 | `js_regcomp` + `js_regexec` | `cflags=REG_ICASE\|REG_NEWLINE` ||| [x] |
| 18 | `js_regexec` | `eflags=REG_NOTBOL` × all 4 cflags ||| [x] |
| 19 | `js_regexec` | `sub=NULL` (no capture output) vs full `Resub` ||| [x] |
| 20 | `js_regcompx`/`js_regfreex` | custom `alloc` callback instead of `regcomp`/`regfree` ||| [x] |
| 21 | `js_regexec` | patterns with 0, 1, 9, 16, >16 capture groups ||| [x] |
| 22 | `js_newstate`/`js_freestate` | `flags=0`, default alloc, no limits ||| [x] |
| 23 | `js_newstate` | `flags=JS_STRICT` ||| [x] |
| 24 | `js_newstate` | custom `js_Alloc` callback (accounting) ||| [x] |
| 25 | `js_newstate` + `js_setlimit` | small `memlimit` → `"out of memory"` ||| [x] |
| 26 | `js_newstate` + `js_setlimit` | small `runlimit` → `"script ran too long"` ||| [x] |
| 27 | `js_push*`/`js_gettop`/`js_pop`/`js_copy`/`js_remove`/`js_insert`/`js_replace` | raw stack manipulation, randomized op sequences ||| [x] |
| 28 | `js_dup`/`js_dup2`/`js_rot2`/`js_rot3`/`js_rot4`/`js_rot2pop1`/`js_rot3pop2`/`js_rot` | randomized op sequences over a randomized stack ||| [x] |
| 29 | `js_pushlstring` | embedded NUL, len 0..7 (shrstr) and 8..64 (memstr) ||| [x] |
| 30 | `js_pushliteral` vs `js_pushstring` | litstr vs shrstr/memstr representation for the same bytes ||| [x] |
| 31 | `js_toboolean`/`js_tonumber`/`js_tostring`/`js_tointeger`/`js_toint32`/`js_touint32`/`js_toint16`/`js_touint16` | over every value shape pushed on the stack ||| [x] |
| 32 | `js_typeof`/`js_type`/`js_is*` (all 19 predicates) | over every value shape ||| [x] |
| 33 | `js_equal`/`js_strictequal`/`js_compare` | cross-product of value shapes (incl. NaN → `okay=0`) ||| [x] |
| 34 | `js_concat` | string+string, number+string, object+string ||| [x] |
| 35 | `js_instanceof` | matching / non-matching / non-object rhs ||| [x] |
| 36 | `js_newobject`/`js_newobjectx`/`js_newarray`/`js_newboolean`/`js_newnumber`/`js_newstring` | plain construction + property round-trip || [x] |
| 37 | `js_defproperty`/`js_getproperty`/`js_setproperty`/`js_hasproperty`/`js_delproperty` | all 8 attribute combos, non-strict || [x] |
| 38 | same as 37 | all 8 attribute combos, `JS_STRICT` state (writes to readonly throw) || [x] |
| 39 | `js_defaccessor` | getter-only, setter-only, both, × attribute combos || [x] |
| 40 | `js_getindex`/`js_setindex`/`js_hasindex`/`js_delindex`/`js_getlength`/`js_setlength` | simple/flat array || [x] |
| 41 | same as 40 | array converted to hashed (sparse write / delete) || [x] |
| 42 | `js_getglobal`/`js_setglobal`/`js_defglobal`/`js_delglobal` | × attribute combos || [x] |
| 43 | `js_getregistry`/`js_setregistry`/`js_delregistry`/`js_ref`/`js_unref` | round-trip, missing keys || [x] |
| 44 | `js_pushiterator`/`js_nextiterator` | `own=1` on plain object / array / string || [x] |
| 45 | `js_pushiterator`/`js_nextiterator` | `own=0` (walks prototype chain), DONTENUM filtering || [x] |
| 46 | `js_newcfunction` + `js_call`/`js_pcall` | 0, 1, many args; fewer/more args than `length` || [x] |
| 47 | `js_newcfunctionx` | with `data` + `finalize`; `js_currentfunctiondata` || [x] |
| 48 | `js_newcconstructor` + `js_construct`/`js_pconstruct` | as function and as constructor || [x] |
| 49 | `js_newuserdata` + `js_touserdata`/`js_isuserdata` | matching and mismatching tags || [x] |
| 50 | `js_newuserdatax` | `has`/`put`/`delete` hooks invoked from JS || [x] |
| 51 | `js_newregexp` + JS `exec` | all 8 `JS_REGEXP_G/I/M` combos || [x] |
| 52 | `js_RegExp_prototype_exec` | called directly across FFI, global (lastIndex) and non-global || [x] |
| 53 | `js_loadstring` + `js_call` | randomized valid programs, non-strict || [x] |
| 54 | `js_loadeval` + `js_call` | same programs via eval path (`J->strict`, current env) || [x] |
| 55 | `js_dostring` | randomized programs, non-strict state, report callback captured || [x] |
| 56 | `js_dostring` | randomized programs, `JS_STRICT` state || [x] |
| 57 | `js_ploadstring`/`js_pcall`/`js_pconstruct` | protected variants over the same programs || [x] |
| 58 | `js_savetry`/`js_endtry`/`js_throw` (`js_try`) | nested try depth 1..64 and past `JS_TRYLIMIT` || [x] |
| 59 | `js_repr`/`js_torepr`/`js_tryrepr` | every value shape incl. cyclic objects/arrays || [x] |
| 60 | `jsB_init*` (`jsB_initarray`…`jsB_initstring`, `jsB_init`) | builtin tables present after `js_newstate`; `jsB_propf/propn/props` || [x] |
| 61 | JSON via `js_dostring` | `JSON.parse`/`stringify`: nesting, indent arg, replacer/reviver, `toJSON` || [x] |
| 62 | `jsY_initlex`/`jsY_lex`/`jsY_tokenstring`/`jsY_findword`/`jsY_ishex`/`jsY_tohex`/`jsY_iswhite`/`jsY_isnewline` | lexer driven directly over randomized source text || [x] |
| 63 | `jsY_lexjson` | JSON lexer over randomized JSON text || [x] |
| 64 | `jsP_parse`/`jsP_parsefunction`/`jsP_freeparse` | randomized programs and function bodies || [x] |
| 65 | `jsC_compilescript`/`jsC_compilefunction` | compiled from the ASTs of row 64, strict and non-strict || [x] |
| 66 | `js_newfunction`/`js_newscript`/`js_newarguments`/`jsR_newenvironment` | called across FFI on compiled functions || [x] |
| 67 | `jsV_newobject`/`jsV_getproperty`/`jsV_getownproperty`/`jsV_getpropertyx`/`jsV_setproperty`/`jsV_delproperty` | low-level value API, all object types || [x] |
| 68 | `jsV_toboolean`/`jsV_tonumber`/`jsV_tostring`/`jsV_tointeger`/`jsV_toprimitive`/`jsV_toobject` | low-level conversions over every value shape || [x] |
| 69 | `jsV_newiterator`/`jsV_nextiterator`/`jsR_unflattenarray`/`jsV_resizearray` | flat→hashed array transitions || [x] |
| 70 | `js_intern`/`jsS_dumpstrings`/`jsS_freestrings` | string interning table, randomized string sets || [x] |
| 71 | `js_malloc`/`js_realloc`/`js_free`/`js_strdup` | allocator wrappers via FFI, incl. size 0 || [x] |
| 72 | `js_gc` | `report=0` and `report=1` after building garbage || [x] |
| 73 | `js_setreport`/`js_report`/`js_atpanic`/`js_setcontext`/`js_getcontext` | callback plumbing || [x] |
| 74 | `Math.*` via `js_dostring` | all methods over randomized doubles || [x] |
| 75 | `Number.prototype.toFixed/toExponential/toPrecision/toString(radix)` | randomized values × digits 0..20 × radix 2..36 || [x] |
| 76 | `String.prototype.*` via `js_dostring` | randomized strings incl. multibyte UTF-8 || [x] |
| 77 | `String.prototype.replace/match/split/search` | with regexps from all flag combos || [x] |
| 78 | `Array.prototype.*` | sort/join/reduce/etc. over flat and hashed arrays || [x] |
| 79 | `Date.*` via `js_dostring` | UTC accessors + parse/format over randomized timestamps (TZ fixed to UTC) || [x] |
| 80 | `Object.*` via `js_dostring` | defineProperty/getOwnPropertyDescriptor/keys/freeze/seal || [x] |
| 81 | `encodeURI`/`decodeURI`/`encodeURIComponent`/`decodeURIComponent`/`escape`/`unescape` | randomized byte strings || [x] |
| 82 | `js_newerror`/`js_newevalerror`/…/`js_newurierror` + `js_iserror` | all 7 error constructors across FFI || [x] |
| 83 | `js_error`/`js_typeerror`/…/`js_urierror` (varargs) | thrown from a C callback inside `js_pcall`, caught, message compared || [x] |
| 84 | `js_putc`/`js_puts`/`js_putm`/`js_trap` | stdout-writing helpers across FFI || [x] |
| 85 | `js_pushvalue`/`js_tovalue`/`js_pushobject`/`js_toobject`/`js_toprimitive` | raw `js_Value` marshalling across FFI ||| [x] |
| 86 | `js_savetrypc` | used by the VM; exercised via deeply nested try/catch in scripts || [x] |
| 87 | full pipeline | `js_newstate` → `js_dostring` of a multi-feature program → `js_gc` → `js_freestate`, strict & non-strict || [x] |

## Which test covers which row

| rows | test file |
|---|---|
| 1-13 | `tests/leaf.rs` (UTF-8 codec, unicode tables, dtoa/strtod, coercions) |
| 14-21 | `tests/regex.rs` (`regexp.c` through `js_regcomp*`/`js_regexec`/`js_regfree*`) |
| 22-35, 85 | `tests/state.rs` (state, raw stack, conversions, predicates, operators) |
| 36-50 | `tests/objects.rs` (objects, properties, accessors, arrays, globals, refs, iterators, cfunctions, userdata) |
| 51-59, 61, 74-84, 86-87 | `tests/scripts.rs` (regexps via JS, script vs eval, protected entry points, try/throw, repr, JSON, builtins, pipeline) |
| 60, 62-73, 84 | `tests/lowlevel.rs` (builtin installers, lexer, parser, compiler, `jsV_*`, interning, allocator, GC, stdout writers) |

Every row is driven with **many randomized inputs from a fixed seed** (`Rng::new(0xC0DE_xxxx)`
in `tests/common/mod.rs`), and each row is exercised under all three state configurations
(`flags=0`, `JS_STRICT`, caller-supplied `js_Alloc`) via `diff_all_flags`.

## Binary executable

Neither project builds a driver binary: `c_src/CMakeLists.txt` has no `add_executable`, and
`translation/Cargo.toml` has no `[[bin]]` / `src/main.rs` / `src/bin/`. The closest analogue —
the entry points that write to `stdout` (`js_trap`, `jsS_dumpstrings`, `js_gc(J,1)`,
`js_putc`/`js_puts`/`js_putm`) — **is** compared byte-for-byte in
`tests/lowlevel.rs::row84_stdout_writers` (object addresses printed by `js_dumpvalue`'s
`[Object %p]` are masked; everything else is verbatim).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly one build
configuration; `--no-default-features` and the default build are identical. This is derived
mechanically, not assumed: `phase_d.sh` parses `[features]` out of `Cargo.toml`, forms the
full cross-product, and runs `cargo check` + `cargo build --release` + `nm -D` diff +
`cargo test --release` for each. Result:

```
features found: 0 :: <none>
############ combination: <default> ############           237/237 symbols, 0 missing, 0 extra, all tests ok
############ combination: --no-default-features ############ 237/237 symbols, 0 missing, 0 extra, all tests ok
PHASE D: ALL COMBINATIONS PASS (symbol diff empty, all tests green)
```

## Status

**87 / 87 rows pass** across randomized inputs, under every state configuration and every
feature combination.
