# Configuration surface

The rows below are the pruned cross-product of public/exported entry-point
groups with the runtime axes that C branches on. Compile-time feature search
found no Cargo features and one CMake configuration; platform-only branches do
not create a selectable crate configuration. `[x]` means the fixed-seed,
multi-input differential case in `tests/differential.rs` passes through both
shared libraries.

Every one of the 237 dynamic symbols is named in this table or in the
symbol-only row 32. Opaque internal parser/compiler/VM helpers are exercised
end-to-end through the public state boundary; callable low-level scalar,
UTF, regexp, lexer predicates, numeric, and object/value helpers are also
called directly through `libloading`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|---|---|:---:|
| 1 | `js_newstate`, `js_freestate`, `js_setcontext`, `js_getcontext`, `js_setreport`, `js_report`, `js_atpanic` | default allocator and custom allocator; flags 0, `JS_STRICT`, unknown flag bits; null/non-null context and callbacks | [x] |
| 2 | `js_setlimit`, `js_gc`, `js_malloc`, `js_realloc`, `js_free`, `js_strdup`, `js_intern`, `jsS_dumpstrings`, `jsS_freestrings` | run/memory limits disabled and positive; GC report 0/1; allocation zero/one/many and intern duplicate/distinct strings | [x] |
| 3 | `js_pushundefined`, `js_pushnull`, `js_pushboolean`, `js_pushnumber`, `js_pushstring`, `js_pushlstring`, `js_pushliteral`, `js_gettop`, `js_pop` | every primitive type; boolean 0/nonzero; finite, signed zero, NaN, infinities; string empty/short-inline/long-heap/embedded-NUL | [x] |
| 4 | `js_isdefined`, `js_isundefined`, `js_isnull`, `js_isboolean`, `js_isnumber`, `js_isstring`, `js_isprimitive`, `js_isobject`, `js_iscoercible`, `js_iscallable`, `js_type`, `js_typeof` | each primitive/object/function class at positive, negative, and out-of-range stack indices | [x] |
| 5 | `js_toboolean`, `js_tonumber`, `js_tointeger`, `js_toint32`, `js_touint32`, `js_toint16`, `js_touint16`, `js_tostring`, `js_tryboolean`, `js_trynumber`, `js_tryinteger`, `js_trystring` | undefined/null/boolean/number/string/object; decimal/hex/whitespace/junk; integer boundaries and fallback-on-throw | [x] |
| 6 | `js_copy`, `js_remove`, `js_replace`, `js_dup`, `js_dup2`, `js_rot`, `js_rot2`, `js_rot3`, `js_rot4`, `js_rot2pop1`, `js_rot3pop2`, `js_insert` | stack sizes 1/2/3/4/many; positive and negative indices; valid and invalid depths | [x] |
| 7 | `js_newobjectx`, `js_newobject`, `js_newarray`, `js_newboolean`, `js_newnumber`, `js_newstring`, `js_newerror`, `js_newevalerror`, `js_newrangeerror`, `js_newreferenceerror`, `js_newsyntaxerror`, `js_newtypeerror`, `js_newurierror` | each object class; empty and populated objects; wrapped primitive boundary values and all error prototypes | [x] |
| 8 | `js_getglobal`, `js_setglobal`, `js_defglobal`, `js_delglobal`, `js_getregistry`, `js_setregistry`, `js_delregistry`, `js_pushglobal` | absent/present names; replace/delete; attributes all 8 combinations of `READONLY`, `DONTENUM`, `DONTCONF` | [x] |
| 9 | `js_hasproperty`, `js_getproperty`, `js_setproperty`, `js_defproperty`, `js_delproperty`, `js_defaccessor` | own/inherited/absent/data/getter/setter properties; strict/non-strict; all 8 attribute masks; primitive transient and object receivers | [x] |
| 10 | `js_getlength`, `js_setlength`, `js_hasindex`, `js_getindex`, `js_setindex`, `js_delindex` | string/array/object; empty/one/many/sparse; index negative/zero/interior/end/large; shrink/grow/same length | [x] |
| 11 | `js_pushiterator`, `js_nextiterator`, `jsV_newiterator`, `jsV_nextiterator` | `own` 0/1; own and prototype enumerable/non-enumerable properties; empty/one/many and exhausted iterator | [x] |
| 12 | `js_newcfunction`, `js_newcfunctionx`, `js_newcconstructor`, `js_newuserdata`, `js_newuserdatax`, `js_currentfunction`, `js_currentfunctiondata`, `js_isuserdata`, `js_touserdata` | null/non-null data and finalizer; callback length 0/positive; matching/wrong tags; userdata has/put/delete hooks absent/present | [x] |
| 13 | `js_loadstring`, `js_loadeval`, `js_ploadstring`, `js_dostring`, `js_eval`, `js_call`, `js_pcall`, `js_construct`, `js_pconstruct`, `js_ref`, `js_unref` | script/eval; strict/non-strict; call/construct with 0/one/many args; protected success/error; references create/use/delete | [x] |
| 14 | `js_concat`, `js_compare`, `js_equal`, `js_strictequal`, `js_instanceof` | same/different primitive types, objects, NaN, signed zero; string/number coercion; relational unordered (`okay=0`) and ordered outcomes | [x] |
| 15 | `js_repr`, `js_torepr`, `js_tryrepr` | all primitive classes; arrays/objects/functions/regexp/date/error/userdata; cyclic and escaped strings; conversion fallback | [x] |
| 16 | `js_newregexp`, `js_isregexp`, `js_toregexp`, `js_RegExp_prototype_exec` | flags all 8 combinations of G/I/M; empty/literal/class/anchor/capture/lookahead/backref patterns; empty/ASCII/Unicode/multiline text | [x] |
| 17 | `js_regcomp`, `js_regexec`, `js_regfree` | compile flags 0/I/NEWLINE/I+NEWLINE; exec 0/NOTBOL; null/non-null `Resub`; match/no-match/capture/empty/Unicode | [x] |
| 18 | `js_regcompx`, `js_regfreex` | custom allocator success and failure at each allocation phase; null/non-null error pointer; null/non-null program free | [x] |
| 19 | `jsU_chartorune`, `jsU_runetochar`, `jsU_runelen` | ASCII; 2/3/4-byte UTF-8; truncated/invalid/overlong/surrogate/out-of-range; rune 0, boundaries, `Runeerror`, `Runemax+1` | [x] |
| 20 | `jsU_isalpharune`, `jsU_islowerrune`, `jsU_isupperrune`, `jsU_tolowerrune`, `jsU_toupperrune`, `jsU_tolowerrune_full`, `jsU_toupperrune_full` | ASCII and Unicode letters; nonletters; lower/upper/title; one-to-one and multi-rune mappings; negative/out-of-range rune | [x] |
| 21 | `js_runeat`, `js_utflen`, `js_utfptrtoidx` | empty/ASCII/multibyte/invalid UTF-8; index before/at/after end; pointer at rune boundaries | [x] |
| 22 | `js_strtod`, `js_strtol`, `js_stringtofloat`, `jsV_stringtonumber` | empty/whitespace/sign; decimal/fraction/exponent; hex; Infinity/NaN; trailing junk; radix 0/2/8/10/16/36 and out-of-range | [x] |
| 23 | `js_grisu2`, `js_fmtexp`, `js_itoa`, `jsV_numbertostring` | signed zero; smallest/subnormal/normal/largest finite; integer/fraction; positive/negative exponent; NaN/infinities | [x] |
| 24 | `jsV_numbertointeger`, `jsV_numbertoint32`, `jsV_numbertouint32`, `jsV_numbertoint16`, `jsV_numbertouint16` | NaN/infinities/zero/signed zero; fractions; exact and one-past signed/unsigned 16/32-bit boundaries | [x] |
| 25 | `jsV_toboolean`, `jsV_tonumber`, `jsV_tointeger`, `jsV_tostring`, `jsV_toobject`, `jsV_toprimitive`, `js_tovalue`, `js_pushvalue`, `js_pushobject`, `js_toobject`, `js_toprimitive` | direct low-level value access for every value type; preferred hint none/number/string; primitive wrapping and object unwrapping | [x] |
| 26 | `jsV_newobject`, `jsV_newmemstring`, `jsV_getownproperty`, `jsV_getpropertyx`, `jsV_getproperty`, `jsV_setproperty`, `jsV_delproperty`, `jsV_resizearray`, `jsR_unflattenarray` | every object class reached by public constructors; own/prototype/absent; own flag 0/1; array simple/flattened and shrink/grow | [x] |
| 27 | `js_isarrayindex` | empty; zero; leading zero; decimal; nondigit; `INT_MAX`; overflow; negative spelling | [x] |
| 28 | `jsY_ishex`, `jsY_tohex`, `jsY_iswhite`, `jsY_isnewline`, `jsY_findword`, `jsY_tokenstring` | digits/lower/upper hex and nonhex; all ECMAScript whitespace/newlines; keyword/identifier; valid/invalid token integers | [x] |
| 29 | `jsY_initlex`, `jsY_lex`, `jsY_lexjson`, `jsP_parse`, `jsP_parsefunction`, `jsP_freeparse`, `jsC_compilescript`, `jsC_compilefunction`, `jsC_error` | empty/one/many tokens; JS vs JSON lexing; script/function; strict/non-strict; valid and malformed grammar | [x] |
| 30 | `jsB_init`, `jsB_initarray`, `jsB_initboolean`, `jsB_initdate`, `jsB_initerror`, `jsB_initfunction`, `jsB_initjson`, `jsB_initmath`, `jsB_initnumber`, `jsB_initobject`, `jsB_initregexp`, `jsB_initstring`, `jsB_propf`, `jsB_propn`, `jsB_props` | initialized state and representative constructor/prototype/static call for every built-in family | [x] |
| 31 | `jsR_newenvironment`, `js_newarguments`, `js_newfunction`, `js_newscript`, `js_savetry`, `js_savetrypc`, `js_endtry`, `js_throw`, `js_error`, `js_evalerror`, `js_rangeerror`, `js_referenceerror`, `js_syntaxerror`, `js_typeerror`, `js_urierror` | global/nested environment; strict/non-strict arguments; C/script functions; protected throw of every error class | [x] |
| 32 | `js_free`, `js_putc`, `js_puts`, `js_putm`, `js_trap` plus all 237 names in `SYMBOLS.md` | symbol lookup through both `.so` files; buffer empty/short/growing; trap excluded from output comparison because it writes process stdout | [x] |
| 33 | JavaScript Array/String/Number/Boolean/Date/Object/Function/RegExp/Error/JSON/Math built-ins | fixed-seed randomized values across empty/one/many/sparse, ASCII/Unicode, finite/nonfinite, callbacks, constructors and prototype methods | [x] |
| 34 | property and language semantics | strict flag 0/1 crossed with writable/enumerable/configurable 0/1 and own/inherited/absent; loops/branches/exceptions/scopes/closures | [x] |
| 35 | crate feature set | default feature set and `--no-default-features`; Cargo.toml declares no named features, so these are the complete feature combinations | [x] |
