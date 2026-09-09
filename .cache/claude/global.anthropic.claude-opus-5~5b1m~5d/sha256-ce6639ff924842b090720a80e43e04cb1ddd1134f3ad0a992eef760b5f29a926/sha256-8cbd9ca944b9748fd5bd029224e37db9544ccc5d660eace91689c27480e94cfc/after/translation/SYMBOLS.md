# SYMBOLS.md — exported symbol parity (nm -D)

* C .so:    `c_src/build/libmujs.so`
* Rust .so: `translation/target/release/libmujs.so`

Generated mechanically from `nm -D --defined-only` (T symbols) on both libraries.

| metric | value |
|---|---|
| C dynamic text symbols | 237 |
| Rust dynamic text symbols | 237 |
| **missing in Rust** | 0 |
| extra in Rust | 0 |

Undefined symbols in the Rust .so are libc/libgcc only (malloc, printf, setjmp, math, unwind helpers).

| # | symbol | C .so | Rust .so | defining C file |
|---|--------|-------|----------|-----------------|
| 1 | `jsB_init` | yes | yes | jsbuiltin.c |
| 2 | `jsB_initarray` | yes | yes | jsarray.c |
| 3 | `jsB_initboolean` | yes | yes | jsboolean.c |
| 4 | `jsB_initdate` | yes | yes | jsdate.c |
| 5 | `jsB_initerror` | yes | yes | jserror.c |
| 6 | `jsB_initfunction` | yes | yes | jsfunction.c |
| 7 | `jsB_initjson` | yes | yes | json.c |
| 8 | `jsB_initmath` | yes | yes | jsmath.c |
| 9 | `jsB_initnumber` | yes | yes | jsnumber.c |
| 10 | `jsB_initobject` | yes | yes | jsobject.c |
| 11 | `jsB_initregexp` | yes | yes | jsregexp.c |
| 12 | `jsB_initstring` | yes | yes | jsstring.c |
| 13 | `jsB_propf` | yes | yes | jsbuiltin.c |
| 14 | `jsB_propn` | yes | yes | jsbuiltin.c |
| 15 | `jsB_props` | yes | yes | jsbuiltin.c |
| 16 | `jsC_compilefunction` | yes | yes | jscompile.c |
| 17 | `jsC_compilescript` | yes | yes | jscompile.c |
| 18 | `jsC_error` | yes | yes | jscompile.c |
| 19 | `jsP_freeparse` | yes | yes | jsparse.c |
| 20 | `jsP_parse` | yes | yes | jsparse.c |
| 21 | `jsP_parsefunction` | yes | yes | jsparse.c |
| 22 | `jsR_newenvironment` | yes | yes | jsrun.c |
| 23 | `jsR_unflattenarray` | yes | yes | jsrun.c |
| 24 | `jsS_dumpstrings` | yes | yes | jsintern.c |
| 25 | `jsS_freestrings` | yes | yes | jsintern.c |
| 26 | `jsU_chartorune` | yes | yes | ? |
| 27 | `jsU_isalpharune` | yes | yes | ? |
| 28 | `jsU_islowerrune` | yes | yes | ? |
| 29 | `jsU_isupperrune` | yes | yes | ? |
| 30 | `jsU_runelen` | yes | yes | ? |
| 31 | `jsU_runetochar` | yes | yes | ? |
| 32 | `jsU_tolowerrune` | yes | yes | ? |
| 33 | `jsU_tolowerrune_full` | yes | yes | ? |
| 34 | `jsU_toupperrune` | yes | yes | ? |
| 35 | `jsU_toupperrune_full` | yes | yes | ? |
| 36 | `jsV_delproperty` | yes | yes | jsproperty.c |
| 37 | `jsV_getownproperty` | yes | yes | jsproperty.c |
| 38 | `jsV_getproperty` | yes | yes | jsproperty.c |
| 39 | `jsV_getpropertyx` | yes | yes | jsproperty.c |
| 40 | `jsV_newiterator` | yes | yes | jsproperty.c |
| 41 | `jsV_newmemstring` | yes | yes | jsrun.c |
| 42 | `jsV_newobject` | yes | yes | jsproperty.c |
| 43 | `jsV_nextiterator` | yes | yes | jsproperty.c |
| 44 | `jsV_numbertoint16` | yes | yes | jsvalue.c |
| 45 | `jsV_numbertoint32` | yes | yes | jsvalue.c |
| 46 | `jsV_numbertointeger` | yes | yes | jsvalue.c |
| 47 | `jsV_numbertostring` | yes | yes | jsvalue.c |
| 48 | `jsV_numbertouint16` | yes | yes | jsvalue.c |
| 49 | `jsV_numbertouint32` | yes | yes | jsvalue.c |
| 50 | `jsV_resizearray` | yes | yes | jsproperty.c |
| 51 | `jsV_setproperty` | yes | yes | jsproperty.c |
| 52 | `jsV_stringtonumber` | yes | yes | jsvalue.c |
| 53 | `jsV_toboolean` | yes | yes | jsvalue.c |
| 54 | `jsV_tointeger` | yes | yes | jsvalue.c |
| 55 | `jsV_tonumber` | yes | yes | jsvalue.c |
| 56 | `jsV_toobject` | yes | yes | jsvalue.c |
| 57 | `jsV_toprimitive` | yes | yes | jsvalue.c |
| 58 | `jsV_tostring` | yes | yes | jsvalue.c |
| 59 | `jsY_findword` | yes | yes | jslex.c |
| 60 | `jsY_initlex` | yes | yes | jslex.c |
| 61 | `jsY_ishex` | yes | yes | jslex.c |
| 62 | `jsY_isnewline` | yes | yes | jslex.c |
| 63 | `jsY_iswhite` | yes | yes | jslex.c |
| 64 | `jsY_lex` | yes | yes | jslex.c |
| 65 | `jsY_lexjson` | yes | yes | jslex.c |
| 66 | `jsY_tohex` | yes | yes | jslex.c |
| 67 | `jsY_tokenstring` | yes | yes | jslex.c |
| 68 | `js_RegExp_prototype_exec` | yes | yes | jsregexp.c |
| 69 | `js_atpanic` | yes | yes | jsstate.c |
| 70 | `js_call` | yes | yes | jsrun.c |
| 71 | `js_compare` | yes | yes | jsvalue.c |
| 72 | `js_concat` | yes | yes | jsvalue.c |
| 73 | `js_construct` | yes | yes | jsrun.c |
| 74 | `js_copy` | yes | yes | jsrun.c |
| 75 | `js_currentfunction` | yes | yes | jsrun.c |
| 76 | `js_currentfunctiondata` | yes | yes | jsrun.c |
| 77 | `js_defaccessor` | yes | yes | jsrun.c |
| 78 | `js_defglobal` | yes | yes | jsrun.c |
| 79 | `js_defproperty` | yes | yes | jsrun.c |
| 80 | `js_delglobal` | yes | yes | jsrun.c |
| 81 | `js_delindex` | yes | yes | jsrun.c |
| 82 | `js_delproperty` | yes | yes | jsrun.c |
| 83 | `js_delregistry` | yes | yes | jsrun.c |
| 84 | `js_dostring` | yes | yes | jsstate.c |
| 85 | `js_dup` | yes | yes | jsrun.c |
| 86 | `js_dup2` | yes | yes | jsrun.c |
| 87 | `js_endtry` | yes | yes | jsrun.c |
| 88 | `js_equal` | yes | yes | jsvalue.c |
| 89 | `js_error` | yes | yes | ? |
| 90 | `js_eval` | yes | yes | jsrun.c |
| 91 | `js_evalerror` | yes | yes | ? |
| 92 | `js_fmtexp` | yes | yes | ? |
| 93 | `js_free` | yes | yes | jsrun.c |
| 94 | `js_freestate` | yes | yes | jsgc.c |
| 95 | `js_gc` | yes | yes | jsgc.c |
| 96 | `js_getcontext` | yes | yes | jsstate.c |
| 97 | `js_getglobal` | yes | yes | jsrun.c |
| 98 | `js_getindex` | yes | yes | jsrun.c |
| 99 | `js_getlength` | yes | yes | jsarray.c |
| 100 | `js_getproperty` | yes | yes | jsrun.c |
| 101 | `js_getregistry` | yes | yes | jsrun.c |
| 102 | `js_gettop` | yes | yes | jsrun.c |
| 103 | `js_grisu2` | yes | yes | ? |
| 104 | `js_hasindex` | yes | yes | jsrun.c |
| 105 | `js_hasproperty` | yes | yes | jsrun.c |
| 106 | `js_insert` | yes | yes | jsrun.c |
| 107 | `js_instanceof` | yes | yes | jsvalue.c |
| 108 | `js_intern` | yes | yes | jsintern.c |
| 109 | `js_isarray` | yes | yes | jsrun.c |
| 110 | `js_isarrayindex` | yes | yes | jsrun.c |
| 111 | `js_isboolean` | yes | yes | jsrun.c |
| 112 | `js_isbooleanobject` | yes | yes | json.c |
| 113 | `js_iscallable` | yes | yes | jsrun.c |
| 114 | `js_iscoercible` | yes | yes | jsrun.c |
| 115 | `js_isdateobject` | yes | yes | json.c |
| 116 | `js_isdefined` | yes | yes | jsrun.c |
| 117 | `js_iserror` | yes | yes | jsrun.c |
| 118 | `js_isnull` | yes | yes | jsrun.c |
| 119 | `js_isnumber` | yes | yes | jsrun.c |
| 120 | `js_isnumberobject` | yes | yes | json.c |
| 121 | `js_isobject` | yes | yes | jsrun.c |
| 122 | `js_isprimitive` | yes | yes | jsrun.c |
| 123 | `js_isregexp` | yes | yes | jsrun.c |
| 124 | `js_isstring` | yes | yes | jsrun.c |
| 125 | `js_isstringobject` | yes | yes | json.c |
| 126 | `js_isundefined` | yes | yes | jsrun.c |
| 127 | `js_isuserdata` | yes | yes | jsrun.c |
| 128 | `js_itoa` | yes | yes | jsvalue.c |
| 129 | `js_loadeval` | yes | yes | jsstate.c |
| 130 | `js_loadstring` | yes | yes | jsstate.c |
| 131 | `js_malloc` | yes | yes | jsrun.c |
| 132 | `js_newarguments` | yes | yes | jsvalue.c |
| 133 | `js_newarray` | yes | yes | jsvalue.c |
| 134 | `js_newboolean` | yes | yes | jsvalue.c |
| 135 | `js_newcconstructor` | yes | yes | jsvalue.c |
| 136 | `js_newcfunction` | yes | yes | jsvalue.c |
| 137 | `js_newcfunctionx` | yes | yes | jsvalue.c |
| 138 | `js_newerror` | yes | yes | ? |
| 139 | `js_newevalerror` | yes | yes | ? |
| 140 | `js_newfunction` | yes | yes | jsvalue.c |
| 141 | `js_newnumber` | yes | yes | jsvalue.c |
| 142 | `js_newobject` | yes | yes | jsvalue.c |
| 143 | `js_newobjectx` | yes | yes | jsvalue.c |
| 144 | `js_newrangeerror` | yes | yes | ? |
| 145 | `js_newreferenceerror` | yes | yes | ? |
| 146 | `js_newregexp` | yes | yes | jsregexp.c |
| 147 | `js_newscript` | yes | yes | jsvalue.c |
| 148 | `js_newstate` | yes | yes | jsstate.c |
| 149 | `js_newstring` | yes | yes | jsvalue.c |
| 150 | `js_newsyntaxerror` | yes | yes | ? |
| 151 | `js_newtypeerror` | yes | yes | ? |
| 152 | `js_newurierror` | yes | yes | ? |
| 153 | `js_newuserdata` | yes | yes | jsvalue.c |
| 154 | `js_newuserdatax` | yes | yes | jsvalue.c |
| 155 | `js_nextiterator` | yes | yes | jsrun.c |
| 156 | `js_pcall` | yes | yes | jsrun.c |
| 157 | `js_pconstruct` | yes | yes | jsrun.c |
| 158 | `js_ploadstring` | yes | yes | jsstate.c |
| 159 | `js_pop` | yes | yes | jsrun.c |
| 160 | `js_pushboolean` | yes | yes | jsrun.c |
| 161 | `js_pushglobal` | yes | yes | jsrun.c |
| 162 | `js_pushiterator` | yes | yes | jsrun.c |
| 163 | `js_pushliteral` | yes | yes | jsrun.c |
| 164 | `js_pushlstring` | yes | yes | jsrun.c |
| 165 | `js_pushnull` | yes | yes | jsrun.c |
| 166 | `js_pushnumber` | yes | yes | jsrun.c |
| 167 | `js_pushobject` | yes | yes | jsrun.c |
| 168 | `js_pushstring` | yes | yes | jsrun.c |
| 169 | `js_pushundefined` | yes | yes | jsrun.c |
| 170 | `js_pushvalue` | yes | yes | jsrun.c |
| 171 | `js_putc` | yes | yes | jsintern.c |
| 172 | `js_putm` | yes | yes | jsintern.c |
| 173 | `js_puts` | yes | yes | jsintern.c |
| 174 | `js_rangeerror` | yes | yes | ? |
| 175 | `js_realloc` | yes | yes | jsrun.c |
| 176 | `js_ref` | yes | yes | jsrun.c |
| 177 | `js_referenceerror` | yes | yes | ? |
| 178 | `js_regcomp` | yes | yes | ? |
| 179 | `js_regcompx` | yes | yes | ? |
| 180 | `js_regexec` | yes | yes | ? |
| 181 | `js_regfree` | yes | yes | ? |
| 182 | `js_regfreex` | yes | yes | ? |
| 183 | `js_remove` | yes | yes | jsrun.c |
| 184 | `js_replace` | yes | yes | jsrun.c |
| 185 | `js_report` | yes | yes | jsstate.c |
| 186 | `js_repr` | yes | yes | jsrepr.c |
| 187 | `js_rot` | yes | yes | jsrun.c |
| 188 | `js_rot2` | yes | yes | jsrun.c |
| 189 | `js_rot2pop1` | yes | yes | jsrun.c |
| 190 | `js_rot3` | yes | yes | jsrun.c |
| 191 | `js_rot3pop2` | yes | yes | jsrun.c |
| 192 | `js_rot4` | yes | yes | jsrun.c |
| 193 | `js_runeat` | yes | yes | jsstring.c |
| 194 | `js_savetry` | yes | yes | jsrun.c |
| 195 | `js_savetrypc` | yes | yes | jsrun.c |
| 196 | `js_setcontext` | yes | yes | jsstate.c |
| 197 | `js_setglobal` | yes | yes | jsrun.c |
| 198 | `js_setindex` | yes | yes | jsrun.c |
| 199 | `js_setlength` | yes | yes | jsarray.c |
| 200 | `js_setlimit` | yes | yes | jsrun.c |
| 201 | `js_setproperty` | yes | yes | jsrun.c |
| 202 | `js_setregistry` | yes | yes | jsrun.c |
| 203 | `js_setreport` | yes | yes | jsstate.c |
| 204 | `js_strdup` | yes | yes | jsrun.c |
| 205 | `js_strictequal` | yes | yes | jsvalue.c |
| 206 | `js_stringtofloat` | yes | yes | jsvalue.c |
| 207 | `js_strtod` | yes | yes | ? |
| 208 | `js_strtol` | yes | yes | jsvalue.c |
| 209 | `js_syntaxerror` | yes | yes | ? |
| 210 | `js_throw` | yes | yes | jsrun.c |
| 211 | `js_toboolean` | yes | yes | jsrun.c |
| 212 | `js_toint16` | yes | yes | jsrun.c |
| 213 | `js_toint32` | yes | yes | jsrun.c |
| 214 | `js_tointeger` | yes | yes | jsrun.c |
| 215 | `js_tonumber` | yes | yes | jsrun.c |
| 216 | `js_toobject` | yes | yes | jsrun.c |
| 217 | `js_toprimitive` | yes | yes | jsrun.c |
| 218 | `js_toregexp` | yes | yes | jsrun.c |
| 219 | `js_torepr` | yes | yes | jsrepr.c |
| 220 | `js_tostring` | yes | yes | jsrun.c |
| 221 | `js_touint16` | yes | yes | jsrun.c |
| 222 | `js_touint32` | yes | yes | jsrun.c |
| 223 | `js_touserdata` | yes | yes | jsrun.c |
| 224 | `js_tovalue` | yes | yes | jsrun.c |
| 225 | `js_trap` | yes | yes | jsrun.c |
| 226 | `js_tryboolean` | yes | yes | jsstate.c |
| 227 | `js_tryinteger` | yes | yes | jsstate.c |
| 228 | `js_trynumber` | yes | yes | jsstate.c |
| 229 | `js_tryrepr` | yes | yes | jsrepr.c |
| 230 | `js_trystring` | yes | yes | jsstate.c |
| 231 | `js_type` | yes | yes | jsrun.c |
| 232 | `js_typeerror` | yes | yes | ? |
| 233 | `js_typeof` | yes | yes | jsrun.c |
| 234 | `js_unref` | yes | yes | jsrun.c |
| 235 | `js_urierror` | yes | yes | ? |
| 236 | `js_utflen` | yes | yes | jsstring.c |
| 237 | `js_utfptrtoidx` | yes | yes | jsstring.c |

## Verification

* `nm -D --defined-only` on both libraries: **237 exported text symbols each,
  0 missing in Rust, 0 extra** (regenerated after every source change).
* Undefined symbols in the Rust `.so` are libc/libm/libgcc only: `malloc`,
  `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`, `memmove`, `memset`,
  `memchr`, `bcmp`, `str*`, `printf`, `snprintf`, `sprintf`, `vsnprintf`,
  `puts`, `putchar`, `fputs`, `fputc`, `stderr`, `abort`, `_setjmp`, `longjmp`,
  `time`, `gettimeofday`, `localtime`, `gmtime`, `mktime`, `getenv`, `atoi`,
  the math functions, and the Rust runtime's unwind/TLS helpers.
* Every symbol is additionally *called* through `dlsym` by the differential
  tests in `translation/tests/`, so the `#[no_mangle] extern "C"` wrappers are
  exercised, not merely present. (The `.so` for the tests is resolved by
  `tests/common/mod.rs`, which refuses to run when `libmujs.so` is older than
  any file in `src/` — `cargo test` does not rebuild a cdylib, and a stale
  library would make every comparison vacuous.)
