# SYMBOLS.md — Symbol parity (C `.so` vs Rust `.so`)

Generated mechanically:
```
nm -D --defined-only c_src/build/libmujs.so         | awk "{print \$3}" | sort > c_syms.txt
nm -D --defined-only translation/target/release/libmujs.so | awk "{print \$3}" | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt   # missing from Rust
comm -13 c_syms.txt rust_syms.txt   # extra in Rust
```

| metric | value |
|---|---|
| C `.so` dynamic defined symbols | 237 |
| Rust `.so` dynamic defined symbols | 237 |
| Missing from Rust | 0 |
| Extra in Rust | 0 |
| Undefined non-libc/non-unwind refs in Rust | 0 |

**Result: symbol diff is EMPTY. Every C symbol is exported by the Rust `.so` with the exact same name.**

No symbol had to be added and no C module was missing: the Rust crate already contains a
translation of all 25 `c_src/src/*.c` files (`src/lib.rs` declares the matching 26 modules)
and every symbol is exported through a `#[unsafe(no_mangle)] extern "C-unwind"` wrapper.
Nothing is stubbed — `grep -rn 'unimplemented!\|todo!\|unreachable!("stub")' src/` finds
nothing, and every exported symbol is exercised by the differential tests.

Re-verified in Phase D for **every** feature combination by `phase_d.sh`:

```
############ combination: <default> ############
C symbols: 237  Rust symbols: 237  missing from Rust: 0  extra in Rust: 0
############ combination: --no-default-features ############
C symbols: 237  Rust symbols: 237  missing from Rust: 0  extra in Rust: 0
PHASE D: ALL COMBINATIONS PASS (symbol diff empty, all tests green)
```

The only undefined symbols in the Rust `.so` are libc / libm / libgcc-unwind imports
(`malloc`, `memcpy`, `sin`, `_Unwind_RaiseException`, …) — the same class of imports the C
`.so` has.

## Full symbol table

`P` = declared in public `include/mujs.h`; `I` = internal (`src/jsi.h`, `src/utf.h`, `src/regexp.h`) but
still dynamically exported by the C `.so` and therefore callable through the FFI boundary.

| # | symbol | in C .so | in Rust .so | vis |
|---|--------|----------|-------------|-----|
| 1 | `jsB_init` | yes | yes | I |
| 2 | `jsB_initarray` | yes | yes | I |
| 3 | `jsB_initboolean` | yes | yes | I |
| 4 | `jsB_initdate` | yes | yes | I |
| 5 | `jsB_initerror` | yes | yes | I |
| 6 | `jsB_initfunction` | yes | yes | I |
| 7 | `jsB_initjson` | yes | yes | I |
| 8 | `jsB_initmath` | yes | yes | I |
| 9 | `jsB_initnumber` | yes | yes | I |
| 10 | `jsB_initobject` | yes | yes | I |
| 11 | `jsB_initregexp` | yes | yes | I |
| 12 | `jsB_initstring` | yes | yes | I |
| 13 | `jsB_propf` | yes | yes | I |
| 14 | `jsB_propn` | yes | yes | I |
| 15 | `jsB_props` | yes | yes | I |
| 16 | `jsC_compilefunction` | yes | yes | I |
| 17 | `jsC_compilescript` | yes | yes | I |
| 18 | `jsC_error` | yes | yes | I |
| 19 | `jsP_freeparse` | yes | yes | I |
| 20 | `jsP_parse` | yes | yes | I |
| 21 | `jsP_parsefunction` | yes | yes | I |
| 22 | `jsR_newenvironment` | yes | yes | I |
| 23 | `jsR_unflattenarray` | yes | yes | I |
| 24 | `jsS_dumpstrings` | yes | yes | I |
| 25 | `jsS_freestrings` | yes | yes | I |
| 26 | `jsU_chartorune` | yes | yes | I |
| 27 | `jsU_isalpharune` | yes | yes | I |
| 28 | `jsU_islowerrune` | yes | yes | I |
| 29 | `jsU_isupperrune` | yes | yes | I |
| 30 | `jsU_runelen` | yes | yes | I |
| 31 | `jsU_runetochar` | yes | yes | I |
| 32 | `jsU_tolowerrune` | yes | yes | I |
| 33 | `jsU_tolowerrune_full` | yes | yes | I |
| 34 | `jsU_toupperrune` | yes | yes | I |
| 35 | `jsU_toupperrune_full` | yes | yes | I |
| 36 | `jsV_delproperty` | yes | yes | I |
| 37 | `jsV_getownproperty` | yes | yes | I |
| 38 | `jsV_getproperty` | yes | yes | I |
| 39 | `jsV_getpropertyx` | yes | yes | I |
| 40 | `jsV_newiterator` | yes | yes | I |
| 41 | `jsV_newmemstring` | yes | yes | I |
| 42 | `jsV_newobject` | yes | yes | I |
| 43 | `jsV_nextiterator` | yes | yes | I |
| 44 | `jsV_numbertoint16` | yes | yes | I |
| 45 | `jsV_numbertoint32` | yes | yes | I |
| 46 | `jsV_numbertointeger` | yes | yes | I |
| 47 | `jsV_numbertostring` | yes | yes | I |
| 48 | `jsV_numbertouint16` | yes | yes | I |
| 49 | `jsV_numbertouint32` | yes | yes | I |
| 50 | `jsV_resizearray` | yes | yes | I |
| 51 | `jsV_setproperty` | yes | yes | I |
| 52 | `jsV_stringtonumber` | yes | yes | I |
| 53 | `jsV_toboolean` | yes | yes | I |
| 54 | `jsV_tointeger` | yes | yes | I |
| 55 | `jsV_tonumber` | yes | yes | I |
| 56 | `jsV_toobject` | yes | yes | I |
| 57 | `jsV_toprimitive` | yes | yes | I |
| 58 | `jsV_tostring` | yes | yes | I |
| 59 | `jsY_findword` | yes | yes | I |
| 60 | `jsY_initlex` | yes | yes | I |
| 61 | `jsY_ishex` | yes | yes | I |
| 62 | `jsY_isnewline` | yes | yes | I |
| 63 | `jsY_iswhite` | yes | yes | I |
| 64 | `jsY_lex` | yes | yes | I |
| 65 | `jsY_lexjson` | yes | yes | I |
| 66 | `jsY_tohex` | yes | yes | I |
| 67 | `jsY_tokenstring` | yes | yes | I |
| 68 | `js_RegExp_prototype_exec` | yes | yes | I |
| 69 | `js_atpanic` | yes | yes | P |
| 70 | `js_call` | yes | yes | P |
| 71 | `js_compare` | yes | yes | P |
| 72 | `js_concat` | yes | yes | P |
| 73 | `js_construct` | yes | yes | P |
| 74 | `js_copy` | yes | yes | P |
| 75 | `js_currentfunction` | yes | yes | P |
| 76 | `js_currentfunctiondata` | yes | yes | P |
| 77 | `js_defaccessor` | yes | yes | P |
| 78 | `js_defglobal` | yes | yes | P |
| 79 | `js_defproperty` | yes | yes | P |
| 80 | `js_delglobal` | yes | yes | P |
| 81 | `js_delindex` | yes | yes | P |
| 82 | `js_delproperty` | yes | yes | P |
| 83 | `js_delregistry` | yes | yes | P |
| 84 | `js_dostring` | yes | yes | P |
| 85 | `js_dup` | yes | yes | P |
| 86 | `js_dup2` | yes | yes | P |
| 87 | `js_endtry` | yes | yes | P |
| 88 | `js_equal` | yes | yes | P |
| 89 | `js_error` | yes | yes | P |
| 90 | `js_eval` | yes | yes | P |
| 91 | `js_evalerror` | yes | yes | P |
| 92 | `js_fmtexp` | yes | yes | I |
| 93 | `js_free` | yes | yes | I |
| 94 | `js_freestate` | yes | yes | P |
| 95 | `js_gc` | yes | yes | P |
| 96 | `js_getcontext` | yes | yes | P |
| 97 | `js_getglobal` | yes | yes | P |
| 98 | `js_getindex` | yes | yes | P |
| 99 | `js_getlength` | yes | yes | P |
| 100 | `js_getproperty` | yes | yes | P |
| 101 | `js_getregistry` | yes | yes | P |
| 102 | `js_gettop` | yes | yes | P |
| 103 | `js_grisu2` | yes | yes | I |
| 104 | `js_hasindex` | yes | yes | P |
| 105 | `js_hasproperty` | yes | yes | P |
| 106 | `js_insert` | yes | yes | P |
| 107 | `js_instanceof` | yes | yes | P |
| 108 | `js_intern` | yes | yes | I |
| 109 | `js_isarray` | yes | yes | P |
| 110 | `js_isarrayindex` | yes | yes | I |
| 111 | `js_isboolean` | yes | yes | P |
| 112 | `js_isbooleanobject` | yes | yes | P |
| 113 | `js_iscallable` | yes | yes | P |
| 114 | `js_iscoercible` | yes | yes | P |
| 115 | `js_isdateobject` | yes | yes | P |
| 116 | `js_isdefined` | yes | yes | P |
| 117 | `js_iserror` | yes | yes | P |
| 118 | `js_isnull` | yes | yes | P |
| 119 | `js_isnumber` | yes | yes | P |
| 120 | `js_isnumberobject` | yes | yes | P |
| 121 | `js_isobject` | yes | yes | P |
| 122 | `js_isprimitive` | yes | yes | P |
| 123 | `js_isregexp` | yes | yes | P |
| 124 | `js_isstring` | yes | yes | P |
| 125 | `js_isstringobject` | yes | yes | P |
| 126 | `js_isundefined` | yes | yes | P |
| 127 | `js_isuserdata` | yes | yes | P |
| 128 | `js_itoa` | yes | yes | I |
| 129 | `js_loadeval` | yes | yes | I |
| 130 | `js_loadstring` | yes | yes | P |
| 131 | `js_malloc` | yes | yes | I |
| 132 | `js_newarguments` | yes | yes | I |
| 133 | `js_newarray` | yes | yes | P |
| 134 | `js_newboolean` | yes | yes | P |
| 135 | `js_newcconstructor` | yes | yes | P |
| 136 | `js_newcfunction` | yes | yes | P |
| 137 | `js_newcfunctionx` | yes | yes | P |
| 138 | `js_newerror` | yes | yes | P |
| 139 | `js_newevalerror` | yes | yes | P |
| 140 | `js_newfunction` | yes | yes | I |
| 141 | `js_newnumber` | yes | yes | P |
| 142 | `js_newobject` | yes | yes | P |
| 143 | `js_newobjectx` | yes | yes | P |
| 144 | `js_newrangeerror` | yes | yes | P |
| 145 | `js_newreferenceerror` | yes | yes | P |
| 146 | `js_newregexp` | yes | yes | P |
| 147 | `js_newscript` | yes | yes | I |
| 148 | `js_newstate` | yes | yes | P |
| 149 | `js_newstring` | yes | yes | P |
| 150 | `js_newsyntaxerror` | yes | yes | P |
| 151 | `js_newtypeerror` | yes | yes | P |
| 152 | `js_newurierror` | yes | yes | P |
| 153 | `js_newuserdata` | yes | yes | P |
| 154 | `js_newuserdatax` | yes | yes | P |
| 155 | `js_nextiterator` | yes | yes | P |
| 156 | `js_pcall` | yes | yes | P |
| 157 | `js_pconstruct` | yes | yes | P |
| 158 | `js_ploadstring` | yes | yes | P |
| 159 | `js_pop` | yes | yes | P |
| 160 | `js_pushboolean` | yes | yes | P |
| 161 | `js_pushglobal` | yes | yes | P |
| 162 | `js_pushiterator` | yes | yes | P |
| 163 | `js_pushliteral` | yes | yes | P |
| 164 | `js_pushlstring` | yes | yes | P |
| 165 | `js_pushnull` | yes | yes | P |
| 166 | `js_pushnumber` | yes | yes | P |
| 167 | `js_pushobject` | yes | yes | I |
| 168 | `js_pushstring` | yes | yes | P |
| 169 | `js_pushundefined` | yes | yes | P |
| 170 | `js_pushvalue` | yes | yes | I |
| 171 | `js_putc` | yes | yes | I |
| 172 | `js_putm` | yes | yes | I |
| 173 | `js_puts` | yes | yes | I |
| 174 | `js_rangeerror` | yes | yes | P |
| 175 | `js_realloc` | yes | yes | I |
| 176 | `js_ref` | yes | yes | P |
| 177 | `js_referenceerror` | yes | yes | P |
| 178 | `js_regcomp` | yes | yes | I |
| 179 | `js_regcompx` | yes | yes | I |
| 180 | `js_regexec` | yes | yes | I |
| 181 | `js_regfree` | yes | yes | I |
| 182 | `js_regfreex` | yes | yes | I |
| 183 | `js_remove` | yes | yes | P |
| 184 | `js_replace` | yes | yes | P |
| 185 | `js_report` | yes | yes | P |
| 186 | `js_repr` | yes | yes | P |
| 187 | `js_rot` | yes | yes | P |
| 188 | `js_rot2` | yes | yes | P |
| 189 | `js_rot2pop1` | yes | yes | P |
| 190 | `js_rot3` | yes | yes | P |
| 191 | `js_rot3pop2` | yes | yes | P |
| 192 | `js_rot4` | yes | yes | P |
| 193 | `js_runeat` | yes | yes | I |
| 194 | `js_savetry` | yes | yes | P |
| 195 | `js_savetrypc` | yes | yes | I |
| 196 | `js_setcontext` | yes | yes | P |
| 197 | `js_setglobal` | yes | yes | P |
| 198 | `js_setindex` | yes | yes | P |
| 199 | `js_setlength` | yes | yes | P |
| 200 | `js_setlimit` | yes | yes | P |
| 201 | `js_setproperty` | yes | yes | P |
| 202 | `js_setregistry` | yes | yes | P |
| 203 | `js_setreport` | yes | yes | P |
| 204 | `js_strdup` | yes | yes | I |
| 205 | `js_strictequal` | yes | yes | P |
| 206 | `js_stringtofloat` | yes | yes | I |
| 207 | `js_strtod` | yes | yes | I |
| 208 | `js_strtol` | yes | yes | I |
| 209 | `js_syntaxerror` | yes | yes | P |
| 210 | `js_throw` | yes | yes | P |
| 211 | `js_toboolean` | yes | yes | P |
| 212 | `js_toint16` | yes | yes | P |
| 213 | `js_toint32` | yes | yes | P |
| 214 | `js_tointeger` | yes | yes | P |
| 215 | `js_tonumber` | yes | yes | P |
| 216 | `js_toobject` | yes | yes | I |
| 217 | `js_toprimitive` | yes | yes | I |
| 218 | `js_toregexp` | yes | yes | I |
| 219 | `js_torepr` | yes | yes | P |
| 220 | `js_tostring` | yes | yes | P |
| 221 | `js_touint16` | yes | yes | P |
| 222 | `js_touint32` | yes | yes | P |
| 223 | `js_touserdata` | yes | yes | P |
| 224 | `js_tovalue` | yes | yes | I |
| 225 | `js_trap` | yes | yes | I |
| 226 | `js_tryboolean` | yes | yes | P |
| 227 | `js_tryinteger` | yes | yes | P |
| 228 | `js_trynumber` | yes | yes | P |
| 229 | `js_tryrepr` | yes | yes | P |
| 230 | `js_trystring` | yes | yes | P |
| 231 | `js_type` | yes | yes | P |
| 232 | `js_typeerror` | yes | yes | P |
| 233 | `js_typeof` | yes | yes | P |
| 234 | `js_unref` | yes | yes | P |
| 235 | `js_urierror` | yes | yes | P |
| 236 | `js_utflen` | yes | yes | I |
| 237 | `js_utfptrtoidx` | yes | yes | I |
