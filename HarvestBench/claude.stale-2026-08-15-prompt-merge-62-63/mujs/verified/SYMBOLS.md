# SYMBOLS.md — exported-symbol parity (Phase A / Phase D gate)

Generated mechanically by `nm -D --defined-only` on all three shared objects.

```
C    .so : c_src/build/libmujs.so           (cmake, -w, no -DNDEBUG)
Rust .so : target/debug/libmujs.so         (cargo build)
Rust .so : target/release/libmujs.so       (cargo build --release)
```

| metric | value |
|--------|-------|
| C exports (defined, dynamic) | **237** |
| Rust exports, debug | 248 |
| Rust exports, release | 248 |
| **C symbols missing from the Rust debug .so** | **0** |
| **C symbols missing from the Rust release .so** | **0** |
| Extra symbols in the Rust .so (private shim glue) | 11 |
| Undefined non-libc / non-unwind symbols in the Rust .so | 0 |

There is only ONE valid build configuration (`Cargo.toml` declares no
`[features]`, `c_src/CMakeLists.txt` declares no options), so this table covers
every feature combination. See `CONFIGS.md` for the enumeration and
`./run_tests.sh` for the automated loop that re-checks it.

## Notes on the exported surface

* The seven variadic `js_*error` functions and `jsC_error` cannot be defined
  by a Rust `cdylib` on stable (no variadic `extern "C"` definitions), so they
  are defined in `shim.c`, which formats the varargs with `vsnprintf` exactly
  like the C original and then calls a Rust `rs_*` implementation. `export.map`
  (applied via `cargo:rustc-cdylib-link-arg`, so it only affects the cdylib and
  not the test binaries) forces those eight names to be globally visible.
* `shim.c` is compiled with `-fexceptions -funwind-tables` so the Rust panic
  used to model `longjmp` can unwind through its frames.

## Parity table

| # | symbol | nm (C) | nm (Rust rel) | Rust module | present |
|---|--------|--------|---------------|-------------|---------|
| 1 | `jsB_init` | T | T | jsbuiltin | YES |
| 2 | `jsB_initarray` | T | T | jsarray | YES |
| 3 | `jsB_initboolean` | T | T | jsboolean | YES |
| 4 | `jsB_initdate` | T | T | jsdate | YES |
| 5 | `jsB_initerror` | T | T | jserror | YES |
| 6 | `jsB_initfunction` | T | T | jsfunction | YES |
| 7 | `jsB_initjson` | T | T | json | YES |
| 8 | `jsB_initmath` | T | T | jsmath | YES |
| 9 | `jsB_initnumber` | T | T | jsnumber | YES |
| 10 | `jsB_initobject` | T | T | jsobject | YES |
| 11 | `jsB_initregexp` | T | T | jsregexp | YES |
| 12 | `jsB_initstring` | T | T | jsstring | YES |
| 13 | `jsB_propf` | T | T | jsbuiltin | YES |
| 14 | `jsB_propn` | T | T | jsbuiltin | YES |
| 15 | `jsB_props` | T | T | jsbuiltin | YES |
| 16 | `jsC_compilefunction` | T | T | jscompile | YES |
| 17 | `jsC_compilescript` | T | T | jscompile | YES |
| 18 | `jsC_error` | T | T | shim.c (variadic) + jscompile | YES |
| 19 | `jsP_freeparse` | T | T | jsparse | YES |
| 20 | `jsP_parse` | T | T | jsparse | YES |
| 21 | `jsP_parsefunction` | T | T | jsparse | YES |
| 22 | `jsR_newenvironment` | T | T | jsrun | YES |
| 23 | `jsR_unflattenarray` | T | T | jsrun | YES |
| 24 | `jsS_dumpstrings` | T | T | jsintern | YES |
| 25 | `jsS_freestrings` | T | T | jsintern | YES |
| 26 | `jsU_chartorune` | T | T | ? | YES |
| 27 | `jsU_isalpharune` | T | T | ? | YES |
| 28 | `jsU_islowerrune` | T | T | ? | YES |
| 29 | `jsU_isupperrune` | T | T | ? | YES |
| 30 | `jsU_runelen` | T | T | ? | YES |
| 31 | `jsU_runetochar` | T | T | ? | YES |
| 32 | `jsU_tolowerrune` | T | T | ? | YES |
| 33 | `jsU_tolowerrune_full` | T | T | ? | YES |
| 34 | `jsU_toupperrune` | T | T | ? | YES |
| 35 | `jsU_toupperrune_full` | T | T | ? | YES |
| 36 | `jsV_delproperty` | T | T | jsproperty | YES |
| 37 | `jsV_getownproperty` | T | T | jsproperty | YES |
| 38 | `jsV_getproperty` | T | T | jsproperty | YES |
| 39 | `jsV_getpropertyx` | T | T | jsproperty | YES |
| 40 | `jsV_newiterator` | T | T | jsproperty | YES |
| 41 | `jsV_newmemstring` | T | T | jsrun | YES |
| 42 | `jsV_newobject` | T | T | jsproperty | YES |
| 43 | `jsV_nextiterator` | T | T | jsproperty | YES |
| 44 | `jsV_numbertoint16` | T | T | jsvalue | YES |
| 45 | `jsV_numbertoint32` | T | T | jsvalue | YES |
| 46 | `jsV_numbertointeger` | T | T | jsvalue | YES |
| 47 | `jsV_numbertostring` | T | T | jsvalue | YES |
| 48 | `jsV_numbertouint16` | T | T | jsvalue | YES |
| 49 | `jsV_numbertouint32` | T | T | jsvalue | YES |
| 50 | `jsV_resizearray` | T | T | jsproperty | YES |
| 51 | `jsV_setproperty` | T | T | jsproperty | YES |
| 52 | `jsV_stringtonumber` | T | T | jsvalue | YES |
| 53 | `jsV_toboolean` | T | T | jsvalue | YES |
| 54 | `jsV_tointeger` | T | T | jsvalue | YES |
| 55 | `jsV_tonumber` | T | T | jsvalue | YES |
| 56 | `jsV_toobject` | T | T | jsvalue | YES |
| 57 | `jsV_toprimitive` | T | T | jsvalue | YES |
| 58 | `jsV_tostring` | T | T | jsvalue | YES |
| 59 | `jsY_findword` | T | T | jslex | YES |
| 60 | `jsY_initlex` | T | T | jslex | YES |
| 61 | `jsY_ishex` | T | T | jslex | YES |
| 62 | `jsY_isnewline` | T | T | jslex | YES |
| 63 | `jsY_iswhite` | T | T | jslex | YES |
| 64 | `jsY_lex` | T | T | jslex | YES |
| 65 | `jsY_lexjson` | T | T | jslex | YES |
| 66 | `jsY_tohex` | T | T | jslex | YES |
| 67 | `jsY_tokenstring` | T | T | jslex | YES |
| 68 | `js_RegExp_prototype_exec` | T | T | jsregexp | YES |
| 69 | `js_atpanic` | T | T | jsstate | YES |
| 70 | `js_call` | T | T | jsrun | YES |
| 71 | `js_compare` | T | T | jsvalue | YES |
| 72 | `js_concat` | T | T | jsvalue | YES |
| 73 | `js_construct` | T | T | jsrun | YES |
| 74 | `js_copy` | T | T | jsrun | YES |
| 75 | `js_currentfunction` | T | T | jsrun | YES |
| 76 | `js_currentfunctiondata` | T | T | jsrun | YES |
| 77 | `js_defaccessor` | T | T | jsrun | YES |
| 78 | `js_defglobal` | T | T | jsrun | YES |
| 79 | `js_defproperty` | T | T | jsrun | YES |
| 80 | `js_delglobal` | T | T | jsrun | YES |
| 81 | `js_delindex` | T | T | jsrun | YES |
| 82 | `js_delproperty` | T | T | jsrun | YES |
| 83 | `js_delregistry` | T | T | jsrun | YES |
| 84 | `js_dostring` | T | T | jsstate | YES |
| 85 | `js_dup` | T | T | jsrun | YES |
| 86 | `js_dup2` | T | T | jsrun | YES |
| 87 | `js_endtry` | T | T | jsrun | YES |
| 88 | `js_equal` | T | T | jsvalue | YES |
| 89 | `js_error` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 90 | `js_eval` | T | T | jsrun | YES |
| 91 | `js_evalerror` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 92 | `js_fmtexp` | T | T | jsdtoa | YES |
| 93 | `js_free` | T | T | jsrun | YES |
| 94 | `js_freestate` | T | T | jsgc | YES |
| 95 | `js_gc` | T | T | jsgc | YES |
| 96 | `js_getcontext` | T | T | jsstate | YES |
| 97 | `js_getglobal` | T | T | jsrun | YES |
| 98 | `js_getindex` | T | T | jsrun | YES |
| 99 | `js_getlength` | T | T | jsarray | YES |
| 100 | `js_getproperty` | T | T | jsrun | YES |
| 101 | `js_getregistry` | T | T | jsrun | YES |
| 102 | `js_gettop` | T | T | jsrun | YES |
| 103 | `js_grisu2` | T | T | jsdtoa | YES |
| 104 | `js_hasindex` | T | T | jsrun | YES |
| 105 | `js_hasproperty` | T | T | jsrun | YES |
| 106 | `js_insert` | T | T | jsrun | YES |
| 107 | `js_instanceof` | T | T | jsvalue | YES |
| 108 | `js_intern` | T | T | jsintern | YES |
| 109 | `js_isarray` | T | T | jsrun | YES |
| 110 | `js_isarrayindex` | T | T | jsrun | YES |
| 111 | `js_isboolean` | T | T | jsrun | YES |
| 112 | `js_isbooleanobject` | T | T | json | YES |
| 113 | `js_iscallable` | T | T | jsrun | YES |
| 114 | `js_iscoercible` | T | T | jsrun | YES |
| 115 | `js_isdateobject` | T | T | json | YES |
| 116 | `js_isdefined` | T | T | jsrun | YES |
| 117 | `js_iserror` | T | T | jsrun | YES |
| 118 | `js_isnull` | T | T | jsrun | YES |
| 119 | `js_isnumber` | T | T | jsrun | YES |
| 120 | `js_isnumberobject` | T | T | json | YES |
| 121 | `js_isobject` | T | T | jsrun | YES |
| 122 | `js_isprimitive` | T | T | jsrun | YES |
| 123 | `js_isregexp` | T | T | jsrun | YES |
| 124 | `js_isstring` | T | T | jsrun | YES |
| 125 | `js_isstringobject` | T | T | json | YES |
| 126 | `js_isundefined` | T | T | jsrun | YES |
| 127 | `js_isuserdata` | T | T | jsrun | YES |
| 128 | `js_itoa` | T | T | jsvalue | YES |
| 129 | `js_loadeval` | T | T | jsstate | YES |
| 130 | `js_loadstring` | T | T | jsstate | YES |
| 131 | `js_malloc` | T | T | jsrun | YES |
| 132 | `js_newarguments` | T | T | jsvalue | YES |
| 133 | `js_newarray` | T | T | jsvalue | YES |
| 134 | `js_newboolean` | T | T | jsvalue | YES |
| 135 | `js_newcconstructor` | T | T | jsvalue | YES |
| 136 | `js_newcfunction` | T | T | jsvalue | YES |
| 137 | `js_newcfunctionx` | T | T | jsvalue | YES |
| 138 | `js_newerror` | T | T | ? | YES |
| 139 | `js_newevalerror` | T | T | ? | YES |
| 140 | `js_newfunction` | T | T | jsvalue | YES |
| 141 | `js_newnumber` | T | T | jsvalue | YES |
| 142 | `js_newobject` | T | T | jsvalue | YES |
| 143 | `js_newobjectx` | T | T | jsvalue | YES |
| 144 | `js_newrangeerror` | T | T | ? | YES |
| 145 | `js_newreferenceerror` | T | T | ? | YES |
| 146 | `js_newregexp` | T | T | jsregexp | YES |
| 147 | `js_newscript` | T | T | jsvalue | YES |
| 148 | `js_newstate` | T | T | jsstate | YES |
| 149 | `js_newstring` | T | T | jsvalue | YES |
| 150 | `js_newsyntaxerror` | T | T | ? | YES |
| 151 | `js_newtypeerror` | T | T | ? | YES |
| 152 | `js_newurierror` | T | T | ? | YES |
| 153 | `js_newuserdata` | T | T | jsvalue | YES |
| 154 | `js_newuserdatax` | T | T | jsvalue | YES |
| 155 | `js_nextiterator` | T | T | jsrun | YES |
| 156 | `js_pcall` | T | T | jsrun | YES |
| 157 | `js_pconstruct` | T | T | jsrun | YES |
| 158 | `js_ploadstring` | T | T | jsstate | YES |
| 159 | `js_pop` | T | T | jsrun | YES |
| 160 | `js_pushboolean` | T | T | jsrun | YES |
| 161 | `js_pushglobal` | T | T | jsrun | YES |
| 162 | `js_pushiterator` | T | T | jsrun | YES |
| 163 | `js_pushliteral` | T | T | jsrun | YES |
| 164 | `js_pushlstring` | T | T | jsrun | YES |
| 165 | `js_pushnull` | T | T | jsrun | YES |
| 166 | `js_pushnumber` | T | T | jsrun | YES |
| 167 | `js_pushobject` | T | T | jsrun | YES |
| 168 | `js_pushstring` | T | T | jsrun | YES |
| 169 | `js_pushundefined` | T | T | jsrun | YES |
| 170 | `js_pushvalue` | T | T | jsrun | YES |
| 171 | `js_putc` | T | T | jsintern | YES |
| 172 | `js_putm` | T | T | jsintern | YES |
| 173 | `js_puts` | T | T | jsintern | YES |
| 174 | `js_rangeerror` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 175 | `js_realloc` | T | T | jsrun | YES |
| 176 | `js_ref` | T | T | jsrun | YES |
| 177 | `js_referenceerror` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 178 | `js_regcomp` | T | T | ? | YES |
| 179 | `js_regcompx` | T | T | ? | YES |
| 180 | `js_regexec` | T | T | ? | YES |
| 181 | `js_regfree` | T | T | ? | YES |
| 182 | `js_regfreex` | T | T | ? | YES |
| 183 | `js_remove` | T | T | jsrun | YES |
| 184 | `js_replace` | T | T | jsrun | YES |
| 185 | `js_report` | T | T | jsstate | YES |
| 186 | `js_repr` | T | T | jsrepr | YES |
| 187 | `js_rot` | T | T | jsrun | YES |
| 188 | `js_rot2` | T | T | jsrun | YES |
| 189 | `js_rot2pop1` | T | T | jsrun | YES |
| 190 | `js_rot3` | T | T | jsrun | YES |
| 191 | `js_rot3pop2` | T | T | jsrun | YES |
| 192 | `js_rot4` | T | T | jsrun | YES |
| 193 | `js_runeat` | T | T | jsstring | YES |
| 194 | `js_savetry` | T | T | jsrun | YES |
| 195 | `js_savetrypc` | T | T | jsrun | YES |
| 196 | `js_setcontext` | T | T | jsstate | YES |
| 197 | `js_setglobal` | T | T | jsrun | YES |
| 198 | `js_setindex` | T | T | jsrun | YES |
| 199 | `js_setlength` | T | T | jsarray | YES |
| 200 | `js_setlimit` | T | T | jsrun | YES |
| 201 | `js_setproperty` | T | T | jsrun | YES |
| 202 | `js_setregistry` | T | T | jsrun | YES |
| 203 | `js_setreport` | T | T | jsstate | YES |
| 204 | `js_strdup` | T | T | jsrun | YES |
| 205 | `js_strictequal` | T | T | jsvalue | YES |
| 206 | `js_stringtofloat` | T | T | jsvalue | YES |
| 207 | `js_strtod` | T | T | jsdtoa | YES |
| 208 | `js_strtol` | T | T | jsvalue | YES |
| 209 | `js_syntaxerror` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 210 | `js_throw` | T | T | jsrun | YES |
| 211 | `js_toboolean` | T | T | jsrun | YES |
| 212 | `js_toint16` | T | T | jsrun | YES |
| 213 | `js_toint32` | T | T | jsrun | YES |
| 214 | `js_tointeger` | T | T | jsrun | YES |
| 215 | `js_tonumber` | T | T | jsrun | YES |
| 216 | `js_toobject` | T | T | jsrun | YES |
| 217 | `js_toprimitive` | T | T | jsrun | YES |
| 218 | `js_toregexp` | T | T | jsrun | YES |
| 219 | `js_torepr` | T | T | jsrepr | YES |
| 220 | `js_tostring` | T | T | jsrun | YES |
| 221 | `js_touint16` | T | T | jsrun | YES |
| 222 | `js_touint32` | T | T | jsrun | YES |
| 223 | `js_touserdata` | T | T | jsrun | YES |
| 224 | `js_tovalue` | T | T | jsrun | YES |
| 225 | `js_trap` | T | T | jsrun | YES |
| 226 | `js_tryboolean` | T | T | jsstate | YES |
| 227 | `js_tryinteger` | T | T | jsstate | YES |
| 228 | `js_trynumber` | T | T | jsstate | YES |
| 229 | `js_tryrepr` | T | T | jsrepr | YES |
| 230 | `js_trystring` | T | T | jsstate | YES |
| 231 | `js_type` | T | T | jsrun | YES |
| 232 | `js_typeerror` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 233 | `js_typeof` | T | T | jsrun | YES |
| 234 | `js_unref` | T | T | jsrun | YES |
| 235 | `js_urierror` | T | T | shim.c (variadic) + jserror/jscompile | YES |
| 236 | `js_utflen` | T | T | jsstring | YES |
| 237 | `js_utfptrtoidx` | T | T | jsstring | YES |

## Extra symbols exported by the Rust .so

These are the Rust-side implementations that `shim.c` calls after formatting
its varargs. They are additive (no C name is shadowed) and harmless.

| # | symbol |
|---|--------|
| 1 | `rs_jsC_error` |
| 2 | `rs_jsP_error` |
| 3 | `rs_jsP_warning` |
| 4 | `rs_jsY_error` |
| 5 | `rs_js_error` |
| 6 | `rs_js_evalerror` |
| 7 | `rs_js_rangeerror` |
| 8 | `rs_js_referenceerror` |
| 9 | `rs_js_syntaxerror` |
| 10 | `rs_js_typeerror` |
| 11 | `rs_js_urierror` |

## Undefined (imported) symbols in the Rust .so

All 84 imports are libc, libgcc unwinder or dynamic-loader symbols.
Non-libc leftovers: **NONE**

Note that the C `libmujs.so` itself has *unresolved* math imports (`floor`,
`fmod`, ...) because `c_src/CMakeLists.txt` does not link `m`; the test harness
therefore `dlopen`s libm with `RTLD_GLOBAL` before loading either library.
