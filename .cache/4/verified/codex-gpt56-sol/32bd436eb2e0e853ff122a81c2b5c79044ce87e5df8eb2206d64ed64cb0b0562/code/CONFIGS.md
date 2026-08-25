# Configuration Surface

Source-derived build matrix:

- Cargo declares no features and no defaults. The only valid combination is the
  empty feature set: `--no-default-features`.
- CMake declares no options. It always compiles every `c_src/src/*.c` file.
- Runtime flags are `JS_STRICT`, `JS_REGEXP_G`, `JS_REGEXP_I`,
  `JS_REGEXP_M`, `JS_READONLY`, `JS_DONTENUM`, and `JS_DONTCONF`.
- Structural limits are `JS_STACKSIZE=4096`, `JS_ENVLIMIT=1024`,
  `JS_TRYLIMIT=64`, `JS_ARRAYLIMIT=1<<26`, `JS_ASTLIMIT=400`,
  `JS_STRLIMIT=1<<28`, `REG_MAXSUB=16`, `REG_MAXPROG=32768`,
  `REG_MAXREC=4096`, `REG_MAXSPAN=64`, and `REG_MAXCLASS=128`.

Each row is exercised with both shared libraries loaded through `libloading`.
Randomized rows use a fixed seed and multiple values.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---:|----------------|--------------------------------------------|:---:|
| 1 | `js_newstate`, `js_freestate` | default allocator, flags `0` | [x] |
| 2 | `js_newstate`, `js_freestate` | default allocator, `JS_STRICT` | [x] |
| 3 | `js_setcontext`, `js_getcontext` | null and non-null opaque context | [x] |
| 4 | `js_setreport`, `js_report`, `js_atpanic` | null/default and callback handlers | [x] |
| 5 | `js_setlimit` | disabled (`0,0`) and positive run/memory limits | [x] |
| 6 | `js_gc` | `report=0` and `report=1`, empty and populated heaps | [x] |
| 7 | `js_dostring` | empty, expression, statement, function, and strict scripts | [x] |
| 8 | `js_ploadstring`, `js_loadstring`, `js_eval` | named source, empty/one/many statements | [x] |
| 9 | `js_pcall`, `js_call` | C/JS callable with zero, one, and many arguments | [x] |
| 10 | `js_pconstruct`, `js_construct` | constructor with zero, one, and many arguments | [x] |
| 11 | `js_savetry`, `js_endtry`, `js_throw` | protected successful and throwing operations | [x] |
| 12 | `js_newerror`, `js_newevalerror`, `js_newrangeerror`, `js_newreferenceerror`, `js_newsyntaxerror`, `js_newtypeerror`, `js_newurierror` | empty and non-empty messages | [x] |
| 13 | `js_ref`, `js_unref`, `js_getregistry`, `js_setregistry`, `js_delregistry` | absent/present registry names and generated references | [x] |
| 14 | `js_getglobal`, `js_setglobal`, `js_defglobal`, `js_delglobal` | absent/present globals with all attribute-bit combinations | [x] |
| 15 | `js_hasproperty`, `js_getproperty`, `js_setproperty`, `js_defproperty`, `js_delproperty`, `js_defaccessor` | own/inherited/missing data and accessor properties; attributes `0..7` | [x] |
| 16 | `js_getlength`, `js_setlength` | object/array/string lengths: empty, one, many, sparse | [x] |
| 17 | `js_hasindex`, `js_getindex`, `js_setindex`, `js_delindex` | indices `0`, interior, final, missing, and negative | [x] |
| 18 | `js_pushundefined`, `js_pushnull`, `js_pushboolean`, `js_pushnumber`, `js_pushstring`, `js_pushlstring`, `js_pushliteral` | every primitive; false/true; `-0`, finite, infinities, NaN; empty/short/long/embedded-NUL UTF-8 strings | [x] |
| 19 | `js_newobjectx`, `js_newobject`, `js_newarray`, `js_newboolean`, `js_newnumber`, `js_newstring` | plain object, empty/sparse/dense array, and boxed primitive shapes | [x] |
| 20 | `js_newcfunction`, `js_newcfunctionx`, `js_currentfunction`, `js_currentfunctiondata` | lengths `0`, `1`, and many; null/non-null data/finalizer | [x] |
| 21 | `js_newcconstructor` | separate call/construct callbacks with zero and multiple arguments | [x] |
| 22 | `js_newuserdata`, `js_newuserdatax`, `js_touserdata`, `js_isuserdata` | matching/mismatching tags; null/non-null data; optional hooks | [x] |
| 23 | `js_newregexp` | flags `0`, G, I, M, GI, GM, IM, GIM; empty/literal/class/capture/anchor patterns | [x] |
| 24 | `js_pushiterator`, `js_nextiterator` | own `0/1`, empty object, inherited properties, enumerable/non-enumerable keys | [x] |
| 25 | `js_isdefined`, `js_isundefined`, `js_isnull`, `js_isboolean`, `js_isnumber`, `js_isstring`, `js_isprimitive`, `js_isobject` | every primitive/object type at positive and negative stack indices | [x] |
| 26 | `js_isarray`, `js_isregexp`, `js_iscallable`, `js_iserror`, boxed-type predicates | plain, array, regexp, function, error, Boolean, Number, String, Date objects | [x] |
| 27 | `js_toboolean` | undefined/null, false/true, `-0`, `0`, finite, infinities, NaN, empty/non-empty strings, objects | [x] |
| 28 | `js_tonumber`, `js_trynumber` | all primitive types; decimal/hex/whitespace/invalid strings; fallback sentinel | [x] |
| 29 | `js_tostring`, `js_trystring` | all primitive and object types; short/heap strings; fallback sentinel | [x] |
| 30 | `js_tointeger`, `js_tryinteger`, `js_toint32`, `js_touint32`, `js_toint16`, `js_touint16` | boundaries, one-past boundaries, fractions, infinities, NaN | [x] |
| 31 | `js_gettop`, `js_pop`, `js_rot`, `js_copy`, `js_remove`, `js_insert`, `js_replace` | empty/one/many stack values and positive/negative indices | [x] |
| 32 | `js_dup`, `js_dup2`, `js_rot2`, `js_rot3`, `js_rot4`, `js_rot2pop1`, `js_rot3pop2` | distinct value sequences at minimum and larger stack depths | [x] |
| 33 | `js_concat`, `js_compare`, `js_equal`, `js_strictequal`, `js_instanceof` | same/different primitive types, objects, coercible strings/numbers, ordered/unordered values | [x] |
| 34 | `js_typeof`, `js_type` | all seven public type classes and callable/non-callable objects | [x] |
| 35 | `js_repr`, `js_torepr`, `js_tryrepr` | primitives, escaped strings, arrays, objects, functions, cyclic objects | [x] |
| 36 | Array built-ins | empty/one/many/sparse arrays; default/custom sort; negative/positive slice/splice indexes; callbacks | [x] |
| 37 | String built-ins | empty/ASCII/multibyte UTF-8; literal/regexp search; global/non-global replacement; split limits | [x] |
| 38 | Number built-ins | random finite values and `-0`/NaN/infinities; radix `2..36`; precision boundaries | [x] |
| 39 | Math built-ins | random finite values plus signed zero, NaN, infinities; zero/one/many `min`/`max` arguments | [x] |
| 40 | Object/Function built-ins | own/inherited descriptors, prototype operations, bind/call/apply with zero/many arguments | [x] |
| 41 | RegExp built-ins | success/failure; captures; G/I/M and combined flags; repeated global execution | [x] |
| 42 | Date built-ins | ISO date/date-time with local/Z/offset forms, leap/boundary dates, finite and non-finite timestamps | [x] |
| 43 | JSON built-ins | null/boolean/number/string/array/object; reviver/replacer; numeric/string indentation; empty/one/many members | [x] |
| 44 | URI/global built-ins | `parseInt` radix modes, `parseFloat`, finite/NaN checks, URI reserved/unreserved and UTF-8 inputs | [x] |
| 45 | `js_strtol`, `js_stringtofloat`, `js_strtod` | signs, whitespace, radix `0/2/8/10/16/36`, fractions/exponents, valid prefix and invalid suffix | [x] |
| 46 | `jsV_numbertointeger`, `jsV_numbertoint32`, `jsV_numbertouint32`, `jsV_numbertoint16`, `jsV_numbertouint16` | random bit patterns and numeric boundaries | [x] |
| 47 | `js_fmtexp`, `js_grisu2`, `jsV_numbertostring`, `js_itoa` | negative/zero/positive exponents and random finite doubles | [x] |
| 48 | `jsU_chartorune`, `jsU_runetochar`, `jsU_runelen` | ASCII, 2/3/4-byte UTF-8, NUL, invalid, overlong, surrogate, and max-rune sequences | [x] |
| 49 | `jsU_isalpharune`, case predicates/conversions, full-case mappings | ASCII and random Unicode scalar/boundary values | [x] |
| 50 | `js_regcomp`, `js_regcompx`, `js_regexec`, `js_regfree`, `js_regfreex` | allocator variants; empty/literal/class/capture/anchor/repetition; success/no-match | [x] |
| 51 | parser/compiler exports (`jsY_*`, `jsP_*`, `jsC_*`) | identifiers/keywords/numbers/strings/regexps and script/function parse-compile pipelines | [x] |
| 52 | GC/intern/property/runtime exports (`jsS_*`, `jsV_*`, `jsR_*`) | empty/populated intern trees, object classes, own/inherited properties, simple/sparse arrays | [x] |

