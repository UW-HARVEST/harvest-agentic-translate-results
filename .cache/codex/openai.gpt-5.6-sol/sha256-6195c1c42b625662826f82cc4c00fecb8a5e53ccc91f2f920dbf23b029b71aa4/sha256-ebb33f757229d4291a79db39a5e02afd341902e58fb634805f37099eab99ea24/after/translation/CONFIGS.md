# Configuration surface

The first section is the complete `nm -D` entry-point surface. The second is the source-derived cross-product of public flags, value categories, input shapes, and low-level API families that C branches on.

## Every exported entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|-----|
| 1 | `jsB_init` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:193`) | [x] |
| 2 | `jsB_initarray` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:837`) | [x] |
| 3 | `jsB_initboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsboolean.c:27`) | [x] |
| 4 | `jsB_initdate` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:225`) | [x] |
| 5 | `jsB_initerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:226`) | [x] |
| 6 | `jsB_initfunction` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:220`) | [x] |
| 7 | `jsB_initjson` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:228`) | [x] |
| 8 | `jsB_initmath` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:227`) | [x] |
| 9 | `jsB_initnumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:222`) | [x] |
| 10 | `jsB_initobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:218`) | [x] |
| 11 | `jsB_initregexp` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:224`) | [x] |
| 12 | `jsB_initstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:223`) | [x] |
| 13 | `jsB_propf` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:841`) | [x] |
| 14 | `jsB_propn` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:18`) | [x] |
| 15 | `jsB_props` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:24`) | [x] |
| 16 | `jsC_compilefunction` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jscompile.c:1420`) | [x] |
| 17 | `jsC_compilescript` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jscompile.c:1425`) | [x] |
| 18 | `jsC_error` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jscompile.c:7`) | [x] |
| 19 | `jsP_freeparse` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsfunction.c:13`) | [x] |
| 20 | `jsP_parse` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:711`) | [x] |
| 21 | `jsP_parsefunction` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsfunction.c:31`) | [x] |
| 22 | `jsR_newenvironment` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:451`) | [x] |
| 23 | `jsR_unflattenarray` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:458`) | [x] |
| 24 | `jsS_dumpstrings` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:126`) | [x] |
| 25 | `jsS_freestrings` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsgc.c:279`) | [x] |
| 26 | `jsU_chartorune` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 27 | `jsU_isalpharune` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 28 | `jsU_islowerrune` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 29 | `jsU_isupperrune` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 30 | `jsU_runelen` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 31 | `jsU_runetochar` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 32 | `jsU_tolowerrune` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 33 | `jsU_tolowerrune_full` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 34 | `jsU_toupperrune` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 35 | `jsU_toupperrune_full` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 36 | `jsV_delproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:485`) | [x] |
| 37 | `jsV_getownproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:480`) | [x] |
| 38 | `jsV_getproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:482`) | [x] |
| 39 | `jsV_getpropertyx` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:481`) | [x] |
| 40 | `jsV_newiterator` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:487`) | [x] |
| 41 | `jsV_newmemstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:452`) | [x] |
| 42 | `jsV_newobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:196`) | [x] |
| 43 | `jsV_nextiterator` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:488`) | [x] |
| 44 | `jsV_numbertoint16` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:473`) | [x] |
| 45 | `jsV_numbertoint32` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:471`) | [x] |
| 46 | `jsV_numbertointeger` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:470`) | [x] |
| 47 | `jsV_numbertostring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jscompile.c:303`) | [x] |
| 48 | `jsV_numbertouint16` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:474`) | [x] |
| 49 | `jsV_numbertouint32` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:472`) | [x] |
| 50 | `jsV_resizearray` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:490`) | [x] |
| 51 | `jsV_setproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:483`) | [x] |
| 52 | `jsV_stringtonumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:476`) | [x] |
| 53 | `jsV_toboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:461`) | [x] |
| 54 | `jsV_tointeger` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:463`) | [x] |
| 55 | `jsV_tonumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:462`) | [x] |
| 56 | `jsV_toobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:465`) | [x] |
| 57 | `jsV_toprimitive` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:466`) | [x] |
| 58 | `jsV_tostring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:464`) | [x] |
| 59 | `jsY_findword` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jscompile.c:42`) | [x] |
| 60 | `jsY_initlex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:570`) | [x] |
| 61 | `jsY_ishex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:148`) | [x] |
| 62 | `jsY_isnewline` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:38`) | [x] |
| 63 | `jsY_iswhite` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:38`) | [x] |
| 64 | `jsY_lex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:571`) | [x] |
| 65 | `jsY_lexjson` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:572`) | [x] |
| 66 | `jsY_tohex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:150`) | [x] |
| 67 | `jsY_tokenstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:567`) | [x] |
| 68 | `js_RegExp_prototype_exec` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:160`) | [x] |
| 69 | `js_atpanic` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:56`) | [x] |
| 70 | `js_call` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:127`) | [x] |
| 71 | `js_compare` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:235`) | [x] |
| 72 | `js_concat` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:234`) | [x] |
| 73 | `js_construct` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:128`) | [x] |
| 74 | `js_copy` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:221`) | [x] |
| 75 | `js_currentfunction` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:156`) | [x] |
| 76 | `js_currentfunctiondata` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:157`) | [x] |
| 77 | `js_defaccessor` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:147`) | [x] |
| 78 | `js_defglobal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:139`) | [x] |
| 79 | `js_defproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:145`) | [x] |
| 80 | `js_delglobal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:140`) | [x] |
| 81 | `js_delindex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:154`) | [x] |
| 82 | `js_delproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:146`) | [x] |
| 83 | `js_delregistry` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:135`) | [x] |
| 84 | `js_dostring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:61`) | [x] |
| 85 | `js_dup` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:226`) | [x] |
| 86 | `js_dup2` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:227`) | [x] |
| 87 | `js_endtry` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:73`) | [x] |
| 88 | `js_equal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:236`) | [x] |
| 89 | `js_error` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:115`) | [x] |
| 90 | `js_eval` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:126`) | [x] |
| 91 | `js_evalerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:116`) | [x] |
| 92 | `js_fmtexp` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsdtoa.c:24`) | [x] |
| 93 | `js_free` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:127`) | [x] |
| 94 | `js_freestate` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:57`) | [x] |
| 95 | `js_gc` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:58`) | [x] |
| 96 | `js_getcontext` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:54`) | [x] |
| 97 | `js_getglobal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:137`) | [x] |
| 98 | `js_getindex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:152`) | [x] |
| 99 | `js_getlength` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:149`) | [x] |
| 100 | `js_getproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:143`) | [x] |
| 101 | `js_getregistry` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:133`) | [x] |
| 102 | `js_gettop` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:218`) | [x] |
| 103 | `js_grisu2` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsdtoa.c:502`) | [x] |
| 104 | `js_hasindex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:151`) | [x] |
| 105 | `js_hasproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:142`) | [x] |
| 106 | `js_insert` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:223`) | [x] |
| 107 | `js_instanceof` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:238`) | [x] |
| 108 | `js_intern` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jscompile.c:59`) | [x] |
| 109 | `js_isarray` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:191`) | [x] |
| 110 | `js_isarrayindex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:145`) | [x] |
| 111 | `js_isboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:186`) | [x] |
| 112 | `js_isbooleanobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:199`) | [x] |
| 113 | `js_iscallable` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:194`) | [x] |
| 114 | `js_iscoercible` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:193`) | [x] |
| 115 | `js_isdateobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:200`) | [x] |
| 116 | `js_isdefined` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:183`) | [x] |
| 117 | `js_iserror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:196`) | [x] |
| 118 | `js_isnull` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:185`) | [x] |
| 119 | `js_isnumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:187`) | [x] |
| 120 | `js_isnumberobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:197`) | [x] |
| 121 | `js_isobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:190`) | [x] |
| 122 | `js_isprimitive` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:189`) | [x] |
| 123 | `js_isregexp` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:192`) | [x] |
| 124 | `js_isstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:188`) | [x] |
| 125 | `js_isstringobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:198`) | [x] |
| 126 | `js_isundefined` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:184`) | [x] |
| 127 | `js_isuserdata` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:195`) | [x] |
| 128 | `js_itoa` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:468`) | [x] |
| 129 | `js_loadeval` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:142`) | [x] |
| 130 | `js_loadstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:124`) | [x] |
| 131 | `js_malloc` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:142`) | [x] |
| 132 | `js_newarguments` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:139`) | [x] |
| 133 | `js_newarray` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:169`) | [x] |
| 134 | `js_newboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:170`) | [x] |
| 135 | `js_newcconstructor` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:175`) | [x] |
| 136 | `js_newcfunction` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:173`) | [x] |
| 137 | `js_newcfunctionx` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:174`) | [x] |
| 138 | `js_newerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:107`) | [x] |
| 139 | `js_newevalerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:108`) | [x] |
| 140 | `js_newfunction` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsfunction.c:38`) | [x] |
| 141 | `js_newnumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:171`) | [x] |
| 142 | `js_newobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:168`) | [x] |
| 143 | `js_newobjectx` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:167`) | [x] |
| 144 | `js_newrangeerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:109`) | [x] |
| 145 | `js_newreferenceerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:110`) | [x] |
| 146 | `js_newregexp` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:178`) | [x] |
| 147 | `js_newscript` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:141`) | [x] |
| 148 | `js_newstate` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:52`) | [x] |
| 149 | `js_newstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:172`) | [x] |
| 150 | `js_newsyntaxerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:111`) | [x] |
| 151 | `js_newtypeerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:112`) | [x] |
| 152 | `js_newurierror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:113`) | [x] |
| 153 | `js_newuserdata` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:176`) | [x] |
| 154 | `js_newuserdatax` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:177`) | [x] |
| 155 | `js_nextiterator` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:181`) | [x] |
| 156 | `js_pcall` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:63`) | [x] |
| 157 | `js_pconstruct` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:64`) | [x] |
| 158 | `js_ploadstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:62`) | [x] |
| 159 | `js_pop` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:219`) | [x] |
| 160 | `js_pushboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:161`) | [x] |
| 161 | `js_pushglobal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:158`) | [x] |
| 162 | `js_pushiterator` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:180`) | [x] |
| 163 | `js_pushliteral` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:165`) | [x] |
| 164 | `js_pushlstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:164`) | [x] |
| 165 | `js_pushnull` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:160`) | [x] |
| 166 | `js_pushnumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:162`) | [x] |
| 167 | `js_pushobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:839`) | [x] |
| 168 | `js_pushstring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:163`) | [x] |
| 169 | `js_pushundefined` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:159`) | [x] |
| 170 | `js_pushvalue` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:291`) | [x] |
| 171 | `js_putc` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:113`) | [x] |
| 172 | `js_putm` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:196`) | [x] |
| 173 | `js_puts` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsfunction.c:22`) | [x] |
| 174 | `js_rangeerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:117`) | [x] |
| 175 | `js_realloc` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:150`) | [x] |
| 176 | `js_ref` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:130`) | [x] |
| 177 | `js_referenceerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:118`) | [x] |
| 178 | `js_regcomp` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 179 | `js_regcompx` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:205`) | [x] |
| 180 | `js_regexec` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsregexp.c:75`) | [x] |
| 181 | `js_regfree` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`macro/generated`) | [x] |
| 182 | `js_regfreex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsgc.c:39`) | [x] |
| 183 | `js_remove` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:222`) | [x] |
| 184 | `js_replace` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:224`) | [x] |
| 185 | `js_report` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:105`) | [x] |
| 186 | `js_repr` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:242`) | [x] |
| 187 | `js_rot` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:220`) | [x] |
| 188 | `js_rot2` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:228`) | [x] |
| 189 | `js_rot2pop1` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:231`) | [x] |
| 190 | `js_rot3` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:229`) | [x] |
| 191 | `js_rot3pop2` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:232`) | [x] |
| 192 | `js_rot4` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:230`) | [x] |
| 193 | `js_runeat` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:146`) | [x] |
| 194 | `js_savetry` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:68`) | [x] |
| 195 | `js_savetrypc` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:185`) | [x] |
| 196 | `js_setcontext` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:53`) | [x] |
| 197 | `js_setglobal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:138`) | [x] |
| 198 | `js_setindex` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:153`) | [x] |
| 199 | `js_setlength` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:150`) | [x] |
| 200 | `js_setlimit` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:59`) | [x] |
| 201 | `js_setproperty` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:144`) | [x] |
| 202 | `js_setregistry` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:134`) | [x] |
| 203 | `js_setreport` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:55`) | [x] |
| 204 | `js_strdup` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:206`) | [x] |
| 205 | `js_strictequal` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:237`) | [x] |
| 206 | `js_stringtofloat` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:77`) | [x] |
| 207 | `js_strtod` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsdtoa.c:563`) | [x] |
| 208 | `js_strtol` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsbuiltin.c:56`) | [x] |
| 209 | `js_syntaxerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:119`) | [x] |
| 210 | `js_throw` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:122`) | [x] |
| 211 | `js_toboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:202`) | [x] |
| 212 | `js_toint16` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:215`) | [x] |
| 213 | `js_toint32` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:213`) | [x] |
| 214 | `js_tointeger` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:212`) | [x] |
| 215 | `js_tonumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:203`) | [x] |
| 216 | `js_toobject` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:71`) | [x] |
| 217 | `js_toprimitive` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsdate.c:421`) | [x] |
| 218 | `js_toregexp` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:144`) | [x] |
| 219 | `js_torepr` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:243`) | [x] |
| 220 | `js_tostring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:204`) | [x] |
| 221 | `js_touint16` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:216`) | [x] |
| 222 | `js_touint32` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:214`) | [x] |
| 223 | `js_touserdata` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:205`) | [x] |
| 224 | `js_tovalue` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsarray.c:279`) | [x] |
| 225 | `js_trap` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:162`) | [x] |
| 226 | `js_tryboolean` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:210`) | [x] |
| 227 | `js_tryinteger` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:209`) | [x] |
| 228 | `js_trynumber` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:208`) | [x] |
| 229 | `js_tryrepr` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:244`) | [x] |
| 230 | `js_trystring` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:207`) | [x] |
| 231 | `js_type` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:94`) | [x] |
| 232 | `js_typeerror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:120`) | [x] |
| 233 | `js_typeof` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:239`) | [x] |
| 234 | `js_unref` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:131`) | [x] |
| 235 | `js_urierror` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`include/mujs.h:121`) | [x] |
| 236 | `js_utflen` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:147`) | [x] |
| 237 | `js_utfptrtoidx` | direct external symbol lookup and ABI invocation/reachability from its valid source-defined calling context (`src/jsi.h:148`) | [x] |

## Branch-distinct option and data-shape combinations

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|-----|
| 238 | `js_newstate/js_freestate` | state flags: `0`, `JS_STRICT`, and unknown bits; default allocator; empty state lifecycle | [x] |
| 239 | `js_newstate` | allocator shapes: default allocator, custom successful allocator, allocator failure on state allocation, stack allocation, and initialization | [x] |
| 240 | `js_setcontext/js_getcontext` | context pointer shapes: null and non-null opaque pointers | [x] |
| 241 | `js_setreport/js_report` | report callback shapes: default, custom callback, empty message, and non-ASCII message | [x] |
| 242 | `js_atpanic` | panic callback replacement: null/default and custom callback | [x] |
| 243 | `js_setlimit/js_dostring` | run/memory limits: disabled, exact boundary, one step below required work, and generous | [x] |
| 244 | `js_gc` | GC report flag `0` and nonzero; empty, live-object, and garbage-heavy heaps | [x] |
| 245 | `js_dostring/js_ploadstring/js_loadstring/js_eval` | source shapes: empty, expression, statement list, function, strict directive, comments, UTF-8, and embedded line terminators | [x] |
| 246 | `js_dostring/js_ploadstring` | valid parser grammar families: literals, arrays, objects, accessors, functions, loops, switch, try/catch/finally, regexps, and JSON | [x] |
| 247 | `js_pcall/js_call` | argument counts: zero, one, and many; JS function and C function; primitive and object return | [x] |
| 248 | `js_pconstruct/js_construct` | constructors: JS and C constructors; zero/many arguments; primitive/object return; custom/default prototype | [x] |
| 249 | `js_ref/js_unref` | registry references: primitive/object values; one and many references; valid and missing reference names | [x] |
| 250 | `js_getregistry/js_setregistry/js_delregistry` | registry keys: empty, ASCII, UTF-8, existing, and missing | [x] |
| 251 | `js_getglobal/js_setglobal/js_defglobal/js_delglobal` | global keys and attributes: missing/existing; all 8 `READONLY|DONTENUM|DONTCONF` combinations | [x] |
| 252 | `js_hasproperty/js_getproperty/js_setproperty/js_defproperty/js_delproperty` | object property shapes: own/inherited/missing; data/accessor; extensible/non-extensible; all attribute combinations | [x] |
| 253 | `js_defaccessor` | accessor shapes: getter-only, setter-only, both; configurable/non-configurable | [x] |
| 254 | `js_getlength/js_setlength` | length shapes: empty, one, many, sparse, shrink, grow, and maximum-adjacent | [x] |
| 255 | `js_hasindex/js_getindex/js_setindex/js_delindex` | index shapes: negative, zero, interior, end, beyond end; dense array, sparse array, string, and ordinary object | [x] |
| 256 | `js_currentfunction/js_currentfunctiondata` | outside/inside C function; null/non-null function data | [x] |
| 257 | `js_pushundefined/js_pushnull/js_pushboolean/js_pushnumber/js_pushstring/js_pushlstring` | value shapes: every primitive; false/true; -0, finite, infinities, NaN; empty/short/heap/UTF-8/embedded-NUL strings | [x] |
| 258 | `js_newobjectx/js_newobject/js_newarray` | container shapes: empty; one/many properties; dense/sparse array; prototype and no-prototype | [x] |
| 259 | `js_newboolean/js_newnumber/js_newstring` | boxed primitives across boolean, numeric boundary, and string-size shapes | [x] |
| 260 | `js_newcfunction/js_newcfunctionx/js_newcconstructor` | native callable shapes: zero/many arity, null/non-null data, finalizer absent/present, function/constructor paths | [x] |
| 261 | `js_newuserdata/js_newuserdatax` | userdata shapes: null/non-null data; matching/mismatching tags; has/put/delete/finalize callbacks absent/present | [x] |
| 262 | `js_newregexp/js_regcomp/js_regcompx/js_regexec` | all 8 `G|I|M`/`ICASE|NEWLINE|NOTBOL` flag combinations; empty/literal/class/group/anchor/backreference patterns; empty/ASCII/UTF-8/multiline haystacks | [x] |
| 263 | `js_pushiterator/js_nextiterator/jsV_newiterator/jsV_nextiterator` | iterator modes: own-only and inherited; empty/one/many properties; enumerable/non-enumerable; array/string/object | [x] |
| 264 | `js_isdefined/js_isundefined/js_isnull/js_isboolean/js_isnumber/js_isstring/js_isprimitive/js_isobject` | every JS value category and valid positive/negative stack indices | [x] |
| 265 | `js_isarray/js_isregexp/js_iscallable/js_isuserdata/js_iserror` | matching and non-matching object subtypes | [x] |
| 266 | `js_isnumberobject/js_isstringobject/js_isbooleanobject/js_isdateobject` | boxed matching type, different boxed type, primitive, and ordinary object | [x] |
| 267 | `js_toboolean/jsV_toboolean` | truthiness categories: undefined, null, false/true, ±0, NaN, finite nonzero, empty/nonempty string, object | [x] |
| 268 | `js_tonumber/jsV_tonumber/jsV_stringtonumber/js_stringtofloat/js_strtod/js_strtol` | numeric text/value shapes: whitespace, sign, integer, fraction, exponent, hex, Infinity, NaN/invalid, overflow, underflow, and trailing text | [x] |
| 269 | `js_tostring/jsV_tostring/jsV_numbertostring/js_fmtexp/js_grisu2/js_itoa` | number/string formatting: -0, integer bounds, fractions, exponent thresholds, infinities, NaN, bases 2/10/16/36 | [x] |
| 270 | `js_trystring/js_trynumber/js_tryinteger/js_tryboolean/js_tryrepr` | successful conversion and throwing conversion; caller-provided fallback sentinel values | [x] |
| 271 | `js_tointeger/js_toint32/js_touint32/js_toint16/js_touint16` | numeric conversion boundaries: NaN/±Infinity/±0, min/max, one-step outside, fractions, and modulo wrap | [x] |
| 272 | `js_gettop/js_pop/js_rot/js_copy/js_remove/js_insert/js_replace` | stack shapes: zero/one/many elements; positive/negative indices; first/interior/last positions | [x] |
| 273 | `js_dup/js_dup2/js_rot2/js_rot3/js_rot4/js_rot2pop1/js_rot3pop2` | minimum valid stack size and larger stacks with distinct marker values | [x] |
| 274 | `js_concat` | primitive combinations: string+string, string+number, number+number, object coercion | [x] |
| 275 | `js_compare/js_equal/js_strictequal` | same/different type pairs; null/undefined; booleans/numbers/strings/objects; NaN and coercion | [x] |
| 276 | `js_instanceof` | matching/nonmatching prototype chain; primitive LHS; callable/non-callable RHS | [x] |
| 277 | `js_typeof/js_type` | all seven public type enum results, including function/object distinction | [x] |
| 278 | `js_repr/js_torepr/js_tryrepr` | all primitive/object subtypes; cycles; sparse arrays; escaping ASCII/control/UTF-8; throwing accessors | [x] |
| 279 | `jsU_chartorune/jsU_runetochar/jsU_runelen/js_runeat/js_utflen/js_utfptrtoidx` | UTF-8 shapes: ASCII, 2/3/4-byte scalar, overlong NUL, malformed sequence, surrogate, max scalar, and out-of-range rune | [x] |
| 280 | `jsU_isalpharune/jsU_islowerrune/jsU_isupperrune/jsU_tolowerrune/jsU_toupperrune` | Unicode categories: ASCII/non-ASCII letters, lower/upper/title, uncased, and expansion mappings | [x] |
| 281 | `js_isarrayindex` | property-name shapes: empty, `0`, leading zero, decimal interior, `INT_MAX`, overflow, negative, and nondigit | [x] |
| 282 | `jsV_newmemstring/js_intern/js_strdup` | string allocation/interning: empty, inline boundary, heap boundary, duplicate, UTF-8, and maximum-adjacent | [x] |
| 283 | `jsV_newobject/jsV_resizearray/jsR_unflattenarray` | object/array storage transitions: empty, flat dense growth, sparse conversion, shrink, and boundary-adjacent | [x] |
| 284 | `jsV_getownproperty/jsV_getproperty/jsV_getpropertyx/jsV_setproperty/jsV_delproperty` | low-level property paths: own/prototype/missing; array/string special properties; data/accessor/readonly/nonconfigurable | [x] |
| 285 | `jsP_parse/jsP_parsefunction/jsC_compilescript/jsC_compilefunction` | low-level parse/compile: empty/simple/compound/deep source; script vs function; strict vs non-strict | [x] |
| 286 | `jsY_initlex/jsY_lex/jsY_lexjson/jsY_tokenstring/jsY_findword` | lexer shapes: every token family, keywords/identifiers, numeric/string/regexp escapes, JSON mode, EOF, and UTF-8 | [x] |
