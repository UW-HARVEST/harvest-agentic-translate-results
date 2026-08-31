# SYMBOLS.md — Symbol parity: C `libmujs.so` vs Rust `libmujs.so`

Generated mechanically from `nm -D --defined-only` on both shared libraries.

- C   `.so`: `c_src/build/libmujs.so`
- Rust `.so`: `translation/target/release/libmujs.so`

## Summary

| metric | value |
|---|---|
| symbols exported by C .so | 237 |
| symbols exported by Rust .so | 237 |
| C symbols MISSING from Rust .so | 0 |
| Rust-only extra symbols | 0 |
| undefined non-libc symbols in Rust .so | 0 |

**Result: 0 missing. Symbol parity is COMPLETE.**

### Beyond parity: every symbol is differentially EXERCISED

Symbol parity alone would only prove the names are present. Every one of the 237
symbols is additionally *called by name* through `dlsym` on BOTH `.so` files and
its results compared, enforced by two tests in `tests/a00_harness.rs`:

| gate | test |
|---|---|
| all 237 resolve via `dlsym` in both `.so`s | `all_c_symbols_resolvable_in_both` |
| `tests/symbols.txt` still matches the C `.so` | `symbols_fixture_matches_the_c_so` |
| all 237 are called by name somewhere in `tests/` | `every_exported_symbol_is_exercised_by_name` |

Where each family is exercised:

| symbol family | test file |
|---|---|
| `jsU_*` (utf codec + Unicode tables) | `tests/b01_utf.rs` |
| `js_grisu2`, `js_fmtexp`, `js_itoa`, `js_strtod`, `js_strtol`, `js_stringtofloat`, `jsV_number*`, `jsV_stringtonumber` | `tests/b02_number.rs` |
| `js_regcomp`, `js_regcompx`, `js_regexec`, `js_regfree`, `js_regfreex` | `tests/b03_regexp.rs` |
| `jsY_iswhite/isnewline/ishex/tohex/findword/tokenstring`, `js_isarrayindex`, `js_utflen`, `js_utfptrtoidx`, `js_runeat`, `js_intern` | `tests/b04_lexer_strhelpers.rs` |
| public `mujs.h` stack API, `jsB_init*`, `jsB_prop*`, `js_setlimit`, `js_gc`, allocator | `tests/b05_stackapi.rs` |
| whole-engine behaviour via JS corpora | `tests/b06_scripts.rs`, `tests/c11_corpus_all.rs` |
| generated / randomized whole-engine behaviour | `tests/b07_random.rs`, `tests/b08_numeric_torture.rs` |
| API-level error rows of `ERRORS.md` | `tests/c09_api_errors.rs` |
| slot-boxing side effects and evaluation order | `tests/c10_boxing_order.rs` |
| `jsY_initlex/lex/lexjson`, `jsP_*`, `jsC_*`, `jsV_*` value+object layer, `js_new*error`, `js_*error`, allocator/buffer helpers, `js_savetry*`, `js_torepr`, `js_trap`, `js_RegExp_prototype_exec`, `js_newuserdatax`, `jsR_newenvironment`, `jsR_unflattenarray`, `jsS_*` | `tests/c12_internals.rs` |

```
$ comm -23 c_syms.txt r_syms.txt   # C symbols not in Rust
(empty)
```

## Full symbol table

`in C` / `in Rust` are from `nm -D --defined-only`.

| # | symbol | in C .so | in Rust .so | in public mujs.h |
|---|--------|----------|-------------|------------------|
| 1 | `jsB_init` | yes | yes | - |
| 2 | `jsB_initarray` | yes | yes | - |
| 3 | `jsB_initboolean` | yes | yes | - |
| 4 | `jsB_initdate` | yes | yes | - |
| 5 | `jsB_initerror` | yes | yes | - |
| 6 | `jsB_initfunction` | yes | yes | - |
| 7 | `jsB_initjson` | yes | yes | - |
| 8 | `jsB_initmath` | yes | yes | - |
| 9 | `jsB_initnumber` | yes | yes | - |
| 10 | `jsB_initobject` | yes | yes | - |
| 11 | `jsB_initregexp` | yes | yes | - |
| 12 | `jsB_initstring` | yes | yes | - |
| 13 | `jsB_propf` | yes | yes | - |
| 14 | `jsB_propn` | yes | yes | - |
| 15 | `jsB_props` | yes | yes | - |
| 16 | `jsC_compilefunction` | yes | yes | - |
| 17 | `jsC_compilescript` | yes | yes | - |
| 18 | `jsC_error` | yes | yes | - |
| 19 | `jsP_freeparse` | yes | yes | - |
| 20 | `jsP_parse` | yes | yes | - |
| 21 | `jsP_parsefunction` | yes | yes | - |
| 22 | `jsR_newenvironment` | yes | yes | - |
| 23 | `jsR_unflattenarray` | yes | yes | - |
| 24 | `jsS_dumpstrings` | yes | yes | - |
| 25 | `jsS_freestrings` | yes | yes | - |
| 26 | `jsU_chartorune` | yes | yes | - |
| 27 | `jsU_isalpharune` | yes | yes | - |
| 28 | `jsU_islowerrune` | yes | yes | - |
| 29 | `jsU_isupperrune` | yes | yes | - |
| 30 | `jsU_runelen` | yes | yes | - |
| 31 | `jsU_runetochar` | yes | yes | - |
| 32 | `jsU_tolowerrune` | yes | yes | - |
| 33 | `jsU_tolowerrune_full` | yes | yes | - |
| 34 | `jsU_toupperrune` | yes | yes | - |
| 35 | `jsU_toupperrune_full` | yes | yes | - |
| 36 | `jsV_delproperty` | yes | yes | - |
| 37 | `jsV_getownproperty` | yes | yes | - |
| 38 | `jsV_getproperty` | yes | yes | - |
| 39 | `jsV_getpropertyx` | yes | yes | - |
| 40 | `jsV_newiterator` | yes | yes | - |
| 41 | `jsV_newmemstring` | yes | yes | - |
| 42 | `jsV_newobject` | yes | yes | - |
| 43 | `jsV_nextiterator` | yes | yes | - |
| 44 | `jsV_numbertoint16` | yes | yes | - |
| 45 | `jsV_numbertoint32` | yes | yes | - |
| 46 | `jsV_numbertointeger` | yes | yes | - |
| 47 | `jsV_numbertostring` | yes | yes | - |
| 48 | `jsV_numbertouint16` | yes | yes | - |
| 49 | `jsV_numbertouint32` | yes | yes | - |
| 50 | `jsV_resizearray` | yes | yes | - |
| 51 | `jsV_setproperty` | yes | yes | - |
| 52 | `jsV_stringtonumber` | yes | yes | - |
| 53 | `jsV_toboolean` | yes | yes | - |
| 54 | `jsV_tointeger` | yes | yes | - |
| 55 | `jsV_tonumber` | yes | yes | - |
| 56 | `jsV_toobject` | yes | yes | - |
| 57 | `jsV_toprimitive` | yes | yes | - |
| 58 | `jsV_tostring` | yes | yes | - |
| 59 | `jsY_findword` | yes | yes | - |
| 60 | `jsY_initlex` | yes | yes | - |
| 61 | `jsY_ishex` | yes | yes | - |
| 62 | `jsY_isnewline` | yes | yes | - |
| 63 | `jsY_iswhite` | yes | yes | - |
| 64 | `jsY_lex` | yes | yes | - |
| 65 | `jsY_lexjson` | yes | yes | - |
| 66 | `jsY_tohex` | yes | yes | - |
| 67 | `jsY_tokenstring` | yes | yes | - |
| 68 | `js_RegExp_prototype_exec` | yes | yes | - |
| 69 | `js_atpanic` | yes | yes | yes |
| 70 | `js_call` | yes | yes | yes |
| 71 | `js_compare` | yes | yes | yes |
| 72 | `js_concat` | yes | yes | yes |
| 73 | `js_construct` | yes | yes | yes |
| 74 | `js_copy` | yes | yes | yes |
| 75 | `js_currentfunction` | yes | yes | yes |
| 76 | `js_currentfunctiondata` | yes | yes | yes |
| 77 | `js_defaccessor` | yes | yes | yes |
| 78 | `js_defglobal` | yes | yes | yes |
| 79 | `js_defproperty` | yes | yes | yes |
| 80 | `js_delglobal` | yes | yes | yes |
| 81 | `js_delindex` | yes | yes | yes |
| 82 | `js_delproperty` | yes | yes | yes |
| 83 | `js_delregistry` | yes | yes | yes |
| 84 | `js_dostring` | yes | yes | yes |
| 85 | `js_dup` | yes | yes | yes |
| 86 | `js_dup2` | yes | yes | yes |
| 87 | `js_endtry` | yes | yes | yes |
| 88 | `js_equal` | yes | yes | yes |
| 89 | `js_error` | yes | yes | yes |
| 90 | `js_eval` | yes | yes | yes |
| 91 | `js_evalerror` | yes | yes | yes |
| 92 | `js_fmtexp` | yes | yes | - |
| 93 | `js_free` | yes | yes | - |
| 94 | `js_freestate` | yes | yes | yes |
| 95 | `js_gc` | yes | yes | yes |
| 96 | `js_getcontext` | yes | yes | yes |
| 97 | `js_getglobal` | yes | yes | yes |
| 98 | `js_getindex` | yes | yes | yes |
| 99 | `js_getlength` | yes | yes | yes |
| 100 | `js_getproperty` | yes | yes | yes |
| 101 | `js_getregistry` | yes | yes | yes |
| 102 | `js_gettop` | yes | yes | yes |
| 103 | `js_grisu2` | yes | yes | - |
| 104 | `js_hasindex` | yes | yes | yes |
| 105 | `js_hasproperty` | yes | yes | yes |
| 106 | `js_insert` | yes | yes | yes |
| 107 | `js_instanceof` | yes | yes | yes |
| 108 | `js_intern` | yes | yes | - |
| 109 | `js_isarray` | yes | yes | yes |
| 110 | `js_isarrayindex` | yes | yes | - |
| 111 | `js_isboolean` | yes | yes | yes |
| 112 | `js_isbooleanobject` | yes | yes | yes |
| 113 | `js_iscallable` | yes | yes | yes |
| 114 | `js_iscoercible` | yes | yes | yes |
| 115 | `js_isdateobject` | yes | yes | yes |
| 116 | `js_isdefined` | yes | yes | yes |
| 117 | `js_iserror` | yes | yes | yes |
| 118 | `js_isnull` | yes | yes | yes |
| 119 | `js_isnumber` | yes | yes | yes |
| 120 | `js_isnumberobject` | yes | yes | yes |
| 121 | `js_isobject` | yes | yes | yes |
| 122 | `js_isprimitive` | yes | yes | yes |
| 123 | `js_isregexp` | yes | yes | yes |
| 124 | `js_isstring` | yes | yes | yes |
| 125 | `js_isstringobject` | yes | yes | yes |
| 126 | `js_isundefined` | yes | yes | yes |
| 127 | `js_isuserdata` | yes | yes | yes |
| 128 | `js_itoa` | yes | yes | - |
| 129 | `js_loadeval` | yes | yes | - |
| 130 | `js_loadstring` | yes | yes | yes |
| 131 | `js_malloc` | yes | yes | - |
| 132 | `js_newarguments` | yes | yes | - |
| 133 | `js_newarray` | yes | yes | yes |
| 134 | `js_newboolean` | yes | yes | yes |
| 135 | `js_newcconstructor` | yes | yes | yes |
| 136 | `js_newcfunction` | yes | yes | yes |
| 137 | `js_newcfunctionx` | yes | yes | yes |
| 138 | `js_newerror` | yes | yes | yes |
| 139 | `js_newevalerror` | yes | yes | yes |
| 140 | `js_newfunction` | yes | yes | - |
| 141 | `js_newnumber` | yes | yes | yes |
| 142 | `js_newobject` | yes | yes | yes |
| 143 | `js_newobjectx` | yes | yes | yes |
| 144 | `js_newrangeerror` | yes | yes | yes |
| 145 | `js_newreferenceerror` | yes | yes | yes |
| 146 | `js_newregexp` | yes | yes | yes |
| 147 | `js_newscript` | yes | yes | - |
| 148 | `js_newstate` | yes | yes | yes |
| 149 | `js_newstring` | yes | yes | yes |
| 150 | `js_newsyntaxerror` | yes | yes | yes |
| 151 | `js_newtypeerror` | yes | yes | yes |
| 152 | `js_newurierror` | yes | yes | yes |
| 153 | `js_newuserdata` | yes | yes | yes |
| 154 | `js_newuserdatax` | yes | yes | yes |
| 155 | `js_nextiterator` | yes | yes | yes |
| 156 | `js_pcall` | yes | yes | yes |
| 157 | `js_pconstruct` | yes | yes | yes |
| 158 | `js_ploadstring` | yes | yes | yes |
| 159 | `js_pop` | yes | yes | yes |
| 160 | `js_pushboolean` | yes | yes | yes |
| 161 | `js_pushglobal` | yes | yes | yes |
| 162 | `js_pushiterator` | yes | yes | yes |
| 163 | `js_pushliteral` | yes | yes | yes |
| 164 | `js_pushlstring` | yes | yes | yes |
| 165 | `js_pushnull` | yes | yes | yes |
| 166 | `js_pushnumber` | yes | yes | yes |
| 167 | `js_pushobject` | yes | yes | - |
| 168 | `js_pushstring` | yes | yes | yes |
| 169 | `js_pushundefined` | yes | yes | yes |
| 170 | `js_pushvalue` | yes | yes | - |
| 171 | `js_putc` | yes | yes | - |
| 172 | `js_putm` | yes | yes | - |
| 173 | `js_puts` | yes | yes | - |
| 174 | `js_rangeerror` | yes | yes | yes |
| 175 | `js_realloc` | yes | yes | - |
| 176 | `js_ref` | yes | yes | yes |
| 177 | `js_referenceerror` | yes | yes | yes |
| 178 | `js_regcomp` | yes | yes | - |
| 179 | `js_regcompx` | yes | yes | - |
| 180 | `js_regexec` | yes | yes | - |
| 181 | `js_regfree` | yes | yes | - |
| 182 | `js_regfreex` | yes | yes | - |
| 183 | `js_remove` | yes | yes | yes |
| 184 | `js_replace` | yes | yes | yes |
| 185 | `js_report` | yes | yes | yes |
| 186 | `js_repr` | yes | yes | yes |
| 187 | `js_rot` | yes | yes | yes |
| 188 | `js_rot2` | yes | yes | yes |
| 189 | `js_rot2pop1` | yes | yes | yes |
| 190 | `js_rot3` | yes | yes | yes |
| 191 | `js_rot3pop2` | yes | yes | yes |
| 192 | `js_rot4` | yes | yes | yes |
| 193 | `js_runeat` | yes | yes | - |
| 194 | `js_savetry` | yes | yes | yes |
| 195 | `js_savetrypc` | yes | yes | - |
| 196 | `js_setcontext` | yes | yes | yes |
| 197 | `js_setglobal` | yes | yes | yes |
| 198 | `js_setindex` | yes | yes | yes |
| 199 | `js_setlength` | yes | yes | yes |
| 200 | `js_setlimit` | yes | yes | yes |
| 201 | `js_setproperty` | yes | yes | yes |
| 202 | `js_setregistry` | yes | yes | yes |
| 203 | `js_setreport` | yes | yes | yes |
| 204 | `js_strdup` | yes | yes | - |
| 205 | `js_strictequal` | yes | yes | yes |
| 206 | `js_stringtofloat` | yes | yes | - |
| 207 | `js_strtod` | yes | yes | - |
| 208 | `js_strtol` | yes | yes | - |
| 209 | `js_syntaxerror` | yes | yes | yes |
| 210 | `js_throw` | yes | yes | yes |
| 211 | `js_toboolean` | yes | yes | yes |
| 212 | `js_toint16` | yes | yes | yes |
| 213 | `js_toint32` | yes | yes | yes |
| 214 | `js_tointeger` | yes | yes | yes |
| 215 | `js_tonumber` | yes | yes | yes |
| 216 | `js_toobject` | yes | yes | - |
| 217 | `js_toprimitive` | yes | yes | - |
| 218 | `js_toregexp` | yes | yes | - |
| 219 | `js_torepr` | yes | yes | yes |
| 220 | `js_tostring` | yes | yes | yes |
| 221 | `js_touint16` | yes | yes | - |
| 222 | `js_touint32` | yes | yes | - |
| 223 | `js_touserdata` | yes | yes | yes |
| 224 | `js_tovalue` | yes | yes | - |
| 225 | `js_trap` | yes | yes | - |
| 226 | `js_tryboolean` | yes | yes | yes |
| 227 | `js_tryinteger` | yes | yes | yes |
| 228 | `js_trynumber` | yes | yes | yes |
| 229 | `js_tryrepr` | yes | yes | yes |
| 230 | `js_trystring` | yes | yes | yes |
| 231 | `js_type` | yes | yes | yes |
| 232 | `js_typeerror` | yes | yes | yes |
| 233 | `js_typeof` | yes | yes | yes |
| 234 | `js_unref` | yes | yes | yes |
| 235 | `js_urierror` | yes | yes | yes |
| 236 | `js_utflen` | yes | yes | - |
| 237 | `js_utfptrtoidx` | yes | yes | - |
