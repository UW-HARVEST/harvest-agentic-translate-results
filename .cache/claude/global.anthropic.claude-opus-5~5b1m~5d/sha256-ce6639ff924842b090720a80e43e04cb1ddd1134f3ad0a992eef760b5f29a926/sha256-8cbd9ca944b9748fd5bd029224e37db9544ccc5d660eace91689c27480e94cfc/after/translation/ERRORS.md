# ERRORS.md — error / rejection surface

Mechanically extracted from `c_src/src/*.c`: every `js_*error()` throw,
every `jsC_error`/`jsP_error`/`jsY_error` compile-time error, every regexp
`die()`, and every runtime `assert()` (the C library is built **with**
assertions: `CMakeLists.txt` sets no `NDEBUG`, so a failing assert aborts
the process with SIGABRT — the Rust library must abort too).

`trigger` is the guarding condition as it appears in the C source;
`expected C result` is the thrown error class + message (or SIGABRT).

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `Ap_join` (jsarray.c:149) | if (n + seplen + rlen > JS_STRLIMIT) ... js_rangeerror(J, "invalid string length"); | `js_rangeerror` : "invalid string length" || [x] |
| 2 | `Ap_sort` (jsarray.c:440) | if (!js_iscallable(J, 1) && !js_isundefined(J, 1)) ... js_typeerror(J, "comparison function must be a function or undefined"); | `js_typeerror` : "comparison function must be a function or undefined" || [x] |
| 3 | `Ap_sort` (jsarray.c:443) | if (len >= INT_MAX) ... js_rangeerror(J, "array is too large to sort"); | `js_rangeerror` : "array is too large to sort" || [x] |
| 4 | `Ap_toString` (jsarray.c:537) | if (!js_iscoercible(J, 0)) ... js_typeerror(J, "'this' is not an object"); | `js_typeerror` : "'this' is not an object" || [x] |
| 5 | `Ap_every` (jsarray.c:604) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 6 | `Ap_some` (jsarray.c:633) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 7 | `Ap_forEach` (jsarray.c:662) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 8 | `Ap_map` (jsarray.c:689) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 9 | `Ap_filter` (jsarray.c:718) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 10 | `Ap_reduce` (jsarray.c:751) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 11 | `Ap_reduce` (jsarray.c:757) | if (len == 0 && !hasinitial) ... js_typeerror(J, "no initial value"); | `js_typeerror` : "no initial value" || [x] |
| 12 | `Ap_reduce` (jsarray.c:767) | if (k == len) ... js_typeerror(J, "no initial value"); | `js_typeerror` : "no initial value" || [x] |
| 13 | `Ap_reduceRight` (jsarray.c:792) | if (!js_iscallable(J, 1)) ... js_typeerror(J, "callback is not a function"); | `js_typeerror` : "callback is not a function" || [x] |
| 14 | `Ap_reduceRight` (jsarray.c:798) | if (len == 0 && !hasinitial) ... js_typeerror(J, "no initial value"); | `js_typeerror` : "no initial value" || [x] |
| 15 | `Ap_reduceRight` (jsarray.c:808) | if (k < 0) ... js_typeerror(J, "no initial value"); | `js_typeerror` : "no initial value" || [x] |
| 16 | `Bp_toString` (jsboolean.c:16) | if (self->type != JS_CBOOLEAN) js_typeerror(J, "not a boolean"); | `js_typeerror` : "not a boolean" || [x] |
| 17 | `Bp_valueOf` (jsboolean.c:23) | if (self->type != JS_CBOOLEAN) js_typeerror(J, "not a boolean"); | `js_typeerror` : "not a boolean" || [x] |
| 18 | `Decode` (jsbuiltin.c:145) | if (!str[0] \|\| !str[1]) ... js_urierror(J, "truncated escape sequence"); | `js_urierror` : "truncated escape sequence" || [x] |
| 19 | `Decode` (jsbuiltin.c:149) | if (!jsY_ishex(a) \|\| !jsY_ishex(b)) ... js_urierror(J, "invalid escape sequence"); | `js_urierror` : "invalid escape sequence" || [x] |
| 20 | `checkfutureword` (jscompile.c:43) | if (jsY_findword(exp->string, futurewords, nelem(futurewords)) >= 0) ... jsC_error(J, exp, "'%s' is a future reserved word", exp->string); | `jsC_error` : "'%s' is a future reserved word" || [x] |
| 21 | `checkfutureword` (jscompile.c:46) | if (jsY_findword(exp->string, strictfuturewords, nelem(strictfuturewords)) >= 0) ... jsC_error(J, exp, "'%s' is a strict mode future reserved word", exp->string); | `jsC_error` : "'%s' is a strict mode future reserved word" || [x] |
| 22 | `emitraw` (jscompile.c:75) | if (value != (js_Instruction)value) ... js_syntaxerror(J, "integer overflow in instruction coding"); | `js_syntaxerror` : "integer overflow in instruction coding" || [x] |
| 23 | `addlocal` (jscompile.c:114) | if (!strcmp(name, "arguments")) ... jsC_error(J, ident, "redefining 'arguments' is not allowed in strict mode"); | `jsC_error` : "redefining 'arguments' is not allowed in strict mode" || [x] |
| 24 | `addlocal` (jscompile.c:116) | if (!strcmp(name, "eval")) ... jsC_error(J, ident, "redefining 'eval' is not allowed in strict mode"); | `jsC_error` : "redefining 'eval' is not allowed in strict mode" || [x] |
| 25 | `addlocal` (jscompile.c:119) | if (!strcmp(name, "eval")) ... js_evalerror(J, "%s:%d: invalid use of 'eval'", J->filename, ident->line); | `js_evalerror` : "%s:%d: invalid use of 'eval'" || [x] |
| 26 | `addlocal` (jscompile.c:128) | if (F->strict) ... jsC_error(J, ident, "duplicate formal parameter '%s'", name); | `jsC_error` : "duplicate formal parameter '%s'" || [x] |
| 27 | `emitlocal` (jscompile.c:204) | if (is_arguments) ... jsC_error(J, ident, "'arguments' is read-only in strict mode"); | `jsC_error` : "'arguments' is read-only in strict mode" || [x] |
| 28 | `emitlocal` (jscompile.c:206) | if (is_eval) ... jsC_error(J, ident, "'eval' is read-only in strict mode"); | `jsC_error` : "'eval' is read-only in strict mode" || [x] |
| 29 | `emitlocal` (jscompile.c:209) | if (is_eval) ... js_evalerror(J, "%s:%d: invalid use of 'eval'", J->filename, ident->line); | `js_evalerror` : "%s:%d: invalid use of 'eval'" || [x] |
| 30 | `emitjumpto` (jscompile.c:238) | if (dest != (js_Instruction)dest) ... js_syntaxerror(J, "jump address integer overflow"); | `js_syntaxerror` : "jump address integer overflow" || [x] |
| 31 | `labelto` (jscompile.c:245) | if (addr != (js_Instruction)addr) ... js_syntaxerror(J, "jump address integer overflow"); | `js_syntaxerror` : "jump address integer overflow" || [x] |
| 32 | `checkdup` (jscompile.c:315) | if (!strcmp(needle, straw)) ... jsC_error(J, list, "duplicate property '%s' in object literal", needle); | `jsC_error` : "duplicate property '%s' in object literal" || [x] |
| 33 | `cobject` (jscompile.c:336) | } else { ... jsC_error(J, prop, "invalid property name in object initializer"); | `jsC_error` : "invalid property name in object initializer" || [p] |
| 34 | `cassign` (jscompile.c:400) | jsC_error(J, lhs, "invalid l-value in assignment"); | `jsC_error` : "invalid l-value in assignment" || [x] |
| 35 | `cassignforin` (jscompile.c:410) | jsC_error(J, lhs->b, "more than one loop variable in for-in statement"); | `jsC_error` : "more than one loop variable in for-in statement" || [x] |
| 36 | `cassignforin` (jscompile.c:439) | jsC_error(J, lhs, "invalid l-value in for-in loop assignment"); | `jsC_error` : "invalid l-value in for-in loop assignment" || [x] |
| 37 | `cassignop1` (jscompile.c:464) | jsC_error(J, lhs, "invalid l-value in assignment"); | `jsC_error` : "invalid l-value in assignment" || [x] |
| 38 | `cassignop2` (jscompile.c:487) | jsC_error(J, lhs, "invalid l-value in assignment"); | `jsC_error` : "invalid l-value in assignment" || [p] |
| 39 | `cdelete` (jscompile.c:508) | if (F->strict) ... jsC_error(J, exp, "delete on an unqualified name is not allowed in strict mode"); | `jsC_error` : "delete on an unqualified name is not allowed in strict mode" || [x] |
| 40 | `cdelete` (jscompile.c:524) | jsC_error(J, exp, "invalid l-value in delete expression"); | `jsC_error` : "invalid l-value in delete expression" || [x] |
| 41 | `cexp` (jscompile.c:780) | jsC_error(J, exp, "unknown expression type"); | `jsC_error` : "unknown expression type" || [p] |
| 42 | `ctrycatch` (jscompile.c:961) | if (!strcmp(catchvar->string, "arguments")) ... jsC_error(J, catchvar, "redefining 'arguments' is not allowed in strict mode"); | `jsC_error` : "redefining 'arguments' is not allowed in strict mode" || [x] |
| 43 | `ctrycatch` (jscompile.c:963) | if (!strcmp(catchvar->string, "eval")) ... jsC_error(J, catchvar, "redefining 'eval' is not allowed in strict mode"); | `jsC_error` : "redefining 'eval' is not allowed in strict mode" || [x] |
| 44 | `ctrycatchfinally` (jscompile.c:993) | if (!strcmp(catchvar->string, "arguments")) ... jsC_error(J, catchvar, "redefining 'arguments' is not allowed in strict mode"); | `jsC_error` : "redefining 'arguments' is not allowed in strict mode" || [x] |
| 45 | `ctrycatchfinally` (jscompile.c:995) | if (!strcmp(catchvar->string, "eval")) ... jsC_error(J, catchvar, "redefining 'eval' is not allowed in strict mode"); | `jsC_error` : "redefining 'eval' is not allowed in strict mode" || [x] |
| 46 | `cswitch` (jscompile.c:1025) | if (def) ... jsC_error(J, clause, "more than one default label in switch"); | `jsC_error` : "more than one default label in switch" || [x] |
| 47 | `cstm` (jscompile.c:1217) | if (!target) ... jsC_error(J, stm, "break label '%s' not found", stm->a->string); | `jsC_error` : "break label '%s' not found" || [x] |
| 48 | `cstm` (jscompile.c:1221) | if (!target) ... jsC_error(J, stm, "unlabelled break must be inside loop or switch"); | `jsC_error` : "unlabelled break must be inside loop or switch" || [x] |
| 49 | `cstm` (jscompile.c:1233) | if (!target) ... jsC_error(J, stm, "continue label '%s' not found", stm->a->string); | `jsC_error` : "continue label '%s' not found" || [x] |
| 50 | `cstm` (jscompile.c:1237) | if (!target) ... jsC_error(J, stm, "continue must be inside loop"); | `jsC_error` : "continue must be inside loop" || [x] |
| 51 | `cstm` (jscompile.c:1251) | if (!target) ... jsC_error(J, stm, "return not in function"); | `jsC_error` : "return not in function" || [x] |
| 52 | `cstm` (jscompile.c:1266) | if (F->strict) ... jsC_error(J, stm->a, "'with' statements are not allowed in strict mode"); | `jsC_error` : "'with' statements are not allowed in strict mode" || [x] |
| 53 | `js_todate` (jsdate.c:366) | if (self->type != JS_CDATE) ... js_typeerror(J, "not a date"); | `js_typeerror` : "not a date" || [x] |
| 54 | `js_setdate` (jsdate.c:374) | if (self->type != JS_CDATE) ... js_typeerror(J, "not a date"); | `js_typeerror` : "not a date" || [x] |
| 55 | `Dp_toISOString` (jsdate.c:485) | if (!isfinite(t)) ... js_rangeerror(J, "invalid date"); | `js_rangeerror` : "invalid date" || [x] |
| 56 | `Dp_toJSON` (jsdate.c:793) | if (!js_iscallable(J, -1)) ... js_typeerror(J, "this.toISOString is not a function"); | `js_typeerror` : "this.toISOString is not a function" || [x] |
| 57 | `minus` (jsdtoa.c:386) | assert(x.e == y.e); | **SIGABRT** (assertion failure) || [p] |
| 58 | `minus` (jsdtoa.c:387) | assert(x.f >= y.f); | **SIGABRT** (assertion failure) || [x] |
| 59 | `Ep_toString` (jserror.c:36) | if (!js_isobject(J, -1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 60 | `Fp_toString` (jsfunction.c:53) | if (!js_iscallable(J, 0)) ... js_typeerror(J, "not a function"); | `js_typeerror` : "not a function" || [x] |
| 61 | `Fp_apply` (jsfunction.c:100) | if (!js_iscallable(J, 0)) ... js_typeerror(J, "not a function"); | `js_typeerror` : "not a function" || [x] |
| 62 | `Fp_call` (jsfunction.c:123) | if (!js_iscallable(J, 0)) ... js_typeerror(J, "not a function"); | `js_typeerror` : "not a function" || [x] |
| 63 | `Fp_bind` (jsfunction.c:186) | if (!js_iscallable(J, 0)) ... js_typeerror(J, "not a function"); | `js_typeerror` : "not a function" || [x] |
| 64 | `jsS_newstringnode` (jsintern.c:47) | if (n > JS_STRLIMIT) ... js_rangeerror(J, "invalid string length"); | `js_rangeerror` : "invalid string length" || [x] |
| 65 | `jsY_unescape` (jslex.c:192) | jsY_error(J, "unexpected escape sequence"); | `jsY_error` : "unexpected escape sequence" || [x] |
| 66 | `lexhex` (jslex.c:255) | if (!jsY_ishex(J->lexchar)) ... jsY_error(J, "malformed hexadecimal number"); | `jsY_error` : "malformed hexadecimal number" || [x] |
| 67 | `lexinteger` (jslex.c:269) | if (!jsY_isdec(J->lexchar)) ... jsY_error(J, "malformed number"); | `jsY_error` : "malformed number" || [p] |
| 68 | `lexnumber` (jslex.c:312) | if (jsY_isdec(J->lexchar)) ... jsY_error(J, "number with leading zero"); | `jsY_error` : "number with leading zero" || [p] |
| 69 | `lexnumber` (jslex.c:333) | if (jsY_isidentifierstart(J->lexchar)) ... jsY_error(J, "number with letter suffix"); | `jsY_error` : "number with letter suffix" || [p] |
| 70 | `lexnumber` (jslex.c:351) | if (jsY_isdec(J->lexchar)) ... jsY_error(J, "number with leading zero"); | `jsY_error` : "number with leading zero" || [x] |
| 71 | `lexnumber` (jslex.c:377) | else ... jsY_error(J, "missing exponent"); | `jsY_error` : "missing exponent" || [x] |
| 72 | `lexnumber` (jslex.c:381) | if (jsY_isidentifierstart(J->lexchar)) ... jsY_error(J, "number with letter suffix"); | `jsY_error` : "number with letter suffix" || [x] |
| 73 | `lexescape` (jslex.c:399) | case EOF: jsY_error(J, "unterminated escape sequence"); | `jsY_error` : "unterminated escape sequence" || [x] |
| 74 | `lexstring` (jslex.c:440) | if (J->lexchar == EOF \|\| J->lexchar == '\n') ... jsY_error(J, "string not terminated"); | `jsY_error` : "string not terminated" || [x] |
| 75 | `lexstring` (jslex.c:443) | if (lexescape(J)) ... jsY_error(J, "malformed escape sequence"); | `jsY_error` : "malformed escape sequence" || [x] |
| 76 | `lexregexp` (jslex.c:490) | if (J->lexchar == EOF \|\| J->lexchar == '\n') { ... jsY_error(J, "regular expression not terminated"); | `jsY_error` : "regular expression not terminated" || [x] |
| 77 | `lexregexp` (jslex.c:497) | if (J->lexchar == EOF \|\| J->lexchar == '\n') ... jsY_error(J, "regular expression not terminated"); | `jsY_error` : "regular expression not terminated" || [x] |
| 78 | `lexregexp` (jslex.c:521) | else jsY_error(J, "illegal flag in regular expression: %c", J->lexchar); | `jsY_error` : "illegal flag in regular expression: %c" || [x] |
| 79 | `lexregexp` (jslex.c:525) | if (g > 1 \|\| i > 1 \|\| m > 1) ... jsY_error(J, "duplicated flag in regular expression"); | `jsY_error` : "duplicated flag in regular expression" || [x] |
| 80 | `jsY_lexx` (jslex.c:574) | if (lexcomment(J)) ... jsY_error(J, "multi-line comment not terminated"); | `jsY_error` : "multi-line comment not terminated" || [x] |
| 81 | `jsY_lexx` (jslex.c:728) | if (J->lexchar >= 0x20 && J->lexchar <= 0x7E) ... jsY_error(J, "unexpected character: '%c'", J->lexchar); | `jsY_error` : "unexpected character: '%c'" || [x] |
| 82 | `jsY_lexx` (jslex.c:729) | if (J->lexchar >= 0x20 && J->lexchar <= 0x7E) ... jsY_error(J, "unexpected character: \\u%04X", J->lexchar); | `jsY_error` : "unexpected character: \\u%04X" || [x] |
| 83 | `lexjsonnumber` (jslex.c:760) | else ... jsY_error(J, "unexpected non-digit"); | `jsY_error` : "unexpected non-digit" || [x] |
| 84 | `lexjsonnumber` (jslex.c:767) | else ... jsY_error(J, "missing digits after decimal point"); | `jsY_error` : "missing digits after decimal point" || [x] |
| 85 | `lexjsonnumber` (jslex.c:777) | else ... jsY_error(J, "missing digits after exponent indicator"); | `jsY_error` : "missing digits after exponent indicator" || [x] |
| 86 | `lexjsonescape` (jslex.c:791) | switch (J->lexchar) { ... default: jsY_error(J, "invalid escape sequence"); | `jsY_error` : "invalid escape sequence" || [x] |
| 87 | `lexjsonstring` (jslex.c:820) | if (J->lexchar == EOF) ... jsY_error(J, "unterminated string"); | `jsY_error` : "unterminated string" || [x] |
| 88 | `lexjsonstring` (jslex.c:822) | else if (J->lexchar < 32) ... jsY_error(J, "invalid control character in string"); | `jsY_error` : "invalid control character in string" || [x] |
| 89 | `jsY_lexjson` (jslex.c:878) | if (J->lexchar >= 0x20 && J->lexchar <= 0x7E) ... jsY_error(J, "unexpected character: '%c'", J->lexchar); | `jsY_error` : "unexpected character: '%c'" || [x] |
| 90 | `jsY_lexjson` (jslex.c:879) | if (J->lexchar >= 0x20 && J->lexchar <= 0x7E) ... jsY_error(J, "unexpected character: \\u%04X", J->lexchar); | `jsY_error` : "unexpected character: \\u%04X" || [x] |
| 91 | `Np_valueOf` (jsnumber.c:22) | if (self->type != JS_CNUMBER) js_typeerror(J, "not a number"); | `js_typeerror` : "not a number" || [x] |
| 92 | `Np_toString` (jsnumber.c:33) | if (self->type != JS_CNUMBER) ... js_typeerror(J, "not a number"); | `js_typeerror` : "not a number" || [x] |
| 93 | `Np_toString` (jsnumber.c:40) | if (radix < 2 \|\| radix > 36) ... js_rangeerror(J, "invalid radix"); | `js_rangeerror` : "invalid radix" || [x] |
| 94 | `Np_toFixed` (jsnumber.c:134) | if (self->type != JS_CNUMBER) js_typeerror(J, "not a number"); | `js_typeerror` : "not a number" || [x] |
| 95 | `Np_toFixed` (jsnumber.c:135) | if (width < 0) js_rangeerror(J, "precision %d out of range", width); | `js_rangeerror` : "precision %d out of range" || [x] |
| 96 | `Np_toFixed` (jsnumber.c:136) | if (width > 20) js_rangeerror(J, "precision %d out of range", width); | `js_rangeerror` : "precision %d out of range" || [x] |
| 97 | `Np_toExponential` (jsnumber.c:150) | if (self->type != JS_CNUMBER) js_typeerror(J, "not a number"); | `js_typeerror` : "not a number" || [x] |
| 98 | `Np_toExponential` (jsnumber.c:151) | if (width < 0) js_rangeerror(J, "precision %d out of range", width); | `js_rangeerror` : "precision %d out of range" || [x] |
| 99 | `Np_toExponential` (jsnumber.c:152) | if (width > 20) js_rangeerror(J, "precision %d out of range", width); | `js_rangeerror` : "precision %d out of range" || [x] |
| 100 | `Np_toPrecision` (jsnumber.c:166) | if (self->type != JS_CNUMBER) js_typeerror(J, "not a number"); | `js_typeerror` : "not a number" || [x] |
| 101 | `Np_toPrecision` (jsnumber.c:167) | if (width < 1) js_rangeerror(J, "precision %d out of range", width); | `js_rangeerror` : "precision %d out of range" || [x] |
| 102 | `Np_toPrecision` (jsnumber.c:168) | if (width > 21) js_rangeerror(J, "precision %d out of range", width); | `js_rangeerror` : "precision %d out of range" || [x] |
| 103 | `O_getPrototypeOf` (jsobject.c:112) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 104 | `O_getOwnPropertyDescriptor` (jsobject.c:125) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 105 | `O_getOwnPropertyNames` (jsobject.c:176) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 106 | `ToPropertyDescriptor` (jsobject.c:258) | if (haswritable \|\| hasvalue) ... js_typeerror(J, "value/writable and get/set attributes are exclusive"); | `js_typeerror` : "value/writable and get/set attributes are exclusive" || [x] |
| 107 | `ToPropertyDescriptor` (jsobject.c:265) | if (haswritable \|\| hasvalue) ... js_typeerror(J, "value/writable and get/set attributes are exclusive"); | `js_typeerror` : "value/writable and get/set attributes are exclusive" || [x] |
| 108 | `O_defineProperty` (jsobject.c:277) | if (!js_isobject(J, 1)) js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 109 | `O_defineProperty` (jsobject.c:278) | if (!js_isobject(J, 3)) js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 110 | `O_defineProperties_walk` (jsobject.c:289) | if (ref->value.t.type != JS_TOBJECT) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 111 | `O_defineProperties_imp` (jsobject.c:304) | if (!js_isobject(J, 2)) js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 112 | `O_defineProperties` (jsobject.c:326) | if (!js_isobject(J, 1)) js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 113 | `O_create` (jsobject.c:342) | else ... js_typeerror(J, "not an object or null"); | `js_typeerror` : "not an object or null" || [x] |
| 114 | `O_keys` (jsobject.c:372) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 115 | `O_preventExtensions` (jsobject.c:403) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 116 | `O_isExtensible` (jsobject.c:413) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 117 | `O_seal` (jsobject.c:431) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 118 | `O_isSealed` (jsobject.c:461) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 119 | `O_freeze` (jsobject.c:489) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 120 | `O_isFrozen` (jsobject.c:521) | if (!js_isobject(J, 1)) ... js_typeerror(J, "not an object"); | `js_typeerror` : "not an object" || [x] |
| 121 | `jsonvalue` (json.c:67) | if (J->lookahead != TK_STRING) ... js_syntaxerror(J, "JSON: unexpected token: %s (expected string)", jsY_tokenstring(J->lookahead)); | `js_syntaxerror` : "JSON: unexpected token: %s (expected string)" || [x] |
| 122 | `jsonvalue` (json.c:107) | js_syntaxerror(J, "JSON: unexpected token: %s", jsY_tokenstring(J->lookahead)); | `js_syntaxerror` : "JSON: unexpected token: %s" || [x] |
| 123 | `fmtobject` (json.c:261) | if (js_toobject(J, i) == js_toobject(J, -1)) ... js_typeerror(J, "cyclic object value"); | `js_typeerror` : "cyclic object value" || [x] |
| 124 | `fmtarray` (json.c:297) | if (js_toobject(J, i) == js_toobject(J, -1)) ... js_typeerror(J, "cyclic object value"); | `js_typeerror` : "cyclic object value" || [x] |
| 125 | `semicolon` (jsparse.c:153) | if (J->newline \|\| J->lookahead == '}' \|\| J->lookahead == 0) ... jsP_error(J, "unexpected token: %s (expected ';')", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token: %s (expected ';')" || [x] |
| 126 | `identifier` (jsparse.c:166) | jsP_error(J, "unexpected token: %s (expected identifier)", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token: %s (expected identifier)" || [x] |
| 127 | `identifiername` (jsparse.c:183) | jsP_error(J, "unexpected token: %s (expected identifier or keyword)", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token: %s (expected identifier or keyword)" || [x] |
| 128 | `primary` (jsparse.c:363) | jsP_error(J, "unexpected token in expression: %s", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token in expression: %s" || [x] |
| 129 | `caseclause` (jsparse.c:700) | jsP_error(J, "unexpected token in switch: %s (expected 'case' or 'default')", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token in switch: %s (expected 'case' or 'default')" || [x] |
| 130 | `forstatement` (jsparse.c:751) | jsP_error(J, "unexpected token in for-var-statement: %s", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token in for-var-statement: %s" || [x] |
| 131 | `forstatement` (jsparse.c:770) | jsP_error(J, "unexpected token in for-statement: %s", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token in for-statement: %s" || [x] |
| 132 | `statement` (jsparse.c:888) | if (!b && !d) ... jsP_error(J, "unexpected token in try: %s (expected 'catch' or 'finally')", jsY_tokenstring(J->lookahead)); | `jsP_error` : "unexpected token in try: %s (expected 'catch' or 'finally')" || [x] |
| 133 | `jsV_setproperty` (jsproperty.c:228) | if (J->strict && !result) ... js_typeerror(J, "object is non-extensible"); | `js_typeerror` : "object is non-extensible" || [x] |
| 134 | `jsV_nextiterator` (jsproperty.c:303) | if (io->type != JS_CITERATOR) ... js_typeerror(J, "not an iterator"); | `js_typeerror` : "not an iterator" || [x] |
| 135 | `jsV_resizearray` (jsproperty.c:325) | assert(!obj->u.a.simple); | **SIGABRT** (assertion failure) || [x] |
| 136 | `js_newregexpx` (jsregexp.c:38) | if (!prog) ... js_syntaxerror(J, "regular expression: %s", error); | `js_syntaxerror` : "regular expression: %s" || [x] |
| 137 | `js_RegExp_prototype_exec` (jsregexp.c:77) | if (result < 0) ... js_error(J, "regexec failed"); | `js_error` : "regexec failed" || [x] |
| 138 | `Rp_test` (jsregexp.c:126) | if (result < 0) ... js_error(J, "regexec failed"); | `js_error` : "regexec failed" || [x] |
| 139 | `jsB_new_RegExp` (jsregexp.c:149) | if (js_isdefined(J, 2)) ... js_typeerror(J, "cannot supply flags when creating one RegExp from another"); | `js_typeerror` : "cannot supply flags when creating one RegExp from another" || [x] |
| 140 | `jsB_new_RegExp` (jsregexp.c:172) | else js_syntaxerror(J, "invalid regular expression flag: '%c'", *s); | `js_syntaxerror` : "invalid regular expression flag: '%c'" || [x] |
| 141 | `jsB_new_RegExp` (jsregexp.c:175) | if (g > 1) js_syntaxerror(J, "invalid regular expression flag: 'g'"); | `js_syntaxerror` : "invalid regular expression flag: 'g'" || [x] |
| 142 | `jsB_new_RegExp` (jsregexp.c:176) | if (i > 1) js_syntaxerror(J, "invalid regular expression flag: 'i'"); | `js_syntaxerror` : "invalid regular expression flag: 'i'" || [x] |
| 143 | `jsB_new_RegExp` (jsregexp.c:177) | if (m > 1) js_syntaxerror(J, "invalid regular expression flag: 'm'"); | `js_syntaxerror` : "invalid regular expression flag: 'm'" || [x] |
| 144 | `js_pushstring` (jsrun.c:149) | if (n > JS_STRLIMIT) ... js_rangeerror(J, "invalid string length"); | `js_rangeerror` : "invalid string length" || [x] |
| 145 | `js_pushlstring` (jsrun.c:166) | if (n > JS_STRLIMIT) ... js_rangeerror(J, "invalid string length"); | `js_rangeerror` : "invalid string length" || [x] |
| 146 | `js_toregexp` (jsrun.c:373) | if (v->t.type == JS_TOBJECT && v->u.object->type == JS_CREGEXP) ... js_typeerror(J, "not a regexp"); | `js_typeerror` : "not a regexp" || [x] |
| 147 | `js_touserdata` (jsrun.c:382) | if (!strcmp(tag, v->u.object->u.user.tag)) ... js_typeerror(J, "not a %s", tag); | `js_typeerror` : "not a %s" || [x] |
| 148 | `jsR_tofunction` (jsrun.c:393) | if (v->u.object->type == JS_CFUNCTION \|\| v->u.object->type == JS_CCFUNCTION) ... js_typeerror(J, "not a function"); | `js_typeerror` : "not a function" || [x] |
| 149 | `js_pop` (jsrun.c:408) | if (TOP < BOT) { ... js_error(J, "stack underflow!"); | `js_error` : "stack underflow!" || [x] |
| 150 | `js_remove` (jsrun.c:416) | if (idx < BOT \|\| idx >= TOP) ... js_error(J, "stack error!"); | `js_error` : "stack error!" || [x] |
| 151 | `js_insert` (jsrun.c:424) | js_error(J, "not implemented yet"); | `js_error` : "not implemented yet" || [x] |
| 152 | `js_replace` (jsrun.c:431) | if (idx < BOT \|\| idx >= TOP) ... js_error(J, "stack error!"); | `js_error` : "stack error!" || [x] |
| 153 | `jsR_setarrayindex` (jsrun.c:673) | assert(obj->u.a.simple); | **SIGABRT** (assertion failure) || [p] |
| 154 | `jsR_setarrayindex` (jsrun.c:674) | assert(k >= 0); | **SIGABRT** (assertion failure) || [p] |
| 155 | `jsR_setarrayindex` (jsrun.c:676) | if (newlen > JS_ARRAYLIMIT) ... js_rangeerror(J, "array too large"); | `js_rangeerror` : "array too large" || [p] |
| 156 | `jsR_setarrayindex` (jsrun.c:678) | if (newlen > obj->u.a.flat_length) { ... assert(newlen == obj->u.a.flat_length + 1); | **SIGABRT** (assertion failure) || [p] |
| 157 | `jsR_setproperty` (jsrun.c:707) | if (newlen != rawlen \|\| newlen < 0) ... js_rangeerror(J, "invalid array length"); | `js_rangeerror` : "invalid array length" || [x] |
| 158 | `jsR_setproperty` (jsrun.c:709) | if (newlen > JS_ARRAYLIMIT) ... js_rangeerror(J, "array too large"); | `js_rangeerror` : "array too large" || [x] |
| 159 | `jsR_setproperty` (jsrun.c:773) | if (ref->getter) ... js_typeerror(J, "setting property '%s' that only has a getter", name); | `js_typeerror` : "setting property '%s' that only has a getter" || [x] |
| 160 | `jsR_setproperty` (jsrun.c:783) | if (J->strict) ... js_typeerror(J, "cannot create property '%s' on transient object", name); | `js_typeerror` : "cannot create property '%s' on transient object" || [x] |
| 161 | `jsR_setproperty` (jsrun.c:800) | if (J->strict) ... js_typeerror(J, "'%s' is read-only", name); | `js_typeerror` : "'%s' is read-only" || [x] |
| 162 | `jsR_defproperty` (jsrun.c:854) | else if (J->strict) ... js_typeerror(J, "'%s' is read-only", name); | `js_typeerror` : "'%s' is read-only" || [x] |
| 163 | `jsR_defproperty` (jsrun.c:860) | else if (J->strict) ... js_typeerror(J, "'%s' is non-configurable", name); | `js_typeerror` : "'%s' is non-configurable" || [x] |
| 164 | `jsR_defproperty` (jsrun.c:866) | else if (J->strict) ... js_typeerror(J, "'%s' is non-configurable", name); | `js_typeerror` : "'%s' is non-configurable" || [x] |
| 165 | `jsR_defproperty` (jsrun.c:875) | if (J->strict \|\| throw) ... js_typeerror(J, "'%s' is read-only or non-configurable", name); | `js_typeerror` : "'%s' is read-only or non-configurable" || [x] |
| 166 | `jsR_delproperty` (jsrun.c:921) | if (J->strict) ... js_typeerror(J, "'%s' is non-configurable", name); | `js_typeerror` : "'%s' is non-configurable" || [x] |
| 167 | `js_setvar` (jsrun.c:1127) | else if (J->strict) ... js_typeerror(J, "'%s' is read-only", name); | `js_typeerror` : "'%s' is read-only" || [x] |
| 168 | `js_setvar` (jsrun.c:1133) | if (J->strict) ... js_referenceerror(J, "assignment to undeclared variable '%s'", name); | `js_referenceerror` : "assignment to undeclared variable '%s'" || [x] |
| 169 | `js_delvar` (jsrun.c:1145) | if (J->strict) ... js_typeerror(J, "'%s' is non-configurable", name); | `js_typeerror` : "'%s' is non-configurable" || [p] |
| 170 | `jsR_pushtrace` (jsrun.c:1290) | if (J->tracetop + 1 == JS_ENVLIMIT) ... js_error(J, "call stack overflow"); | `js_error` : "call stack overflow" || [x] |
| 171 | `js_call` (jsrun.c:1304) | if (n < 0) ... js_rangeerror(J, "number of arguments cannot be negative"); | `js_rangeerror` : "number of arguments cannot be negative" || [x] |
| 172 | `js_call` (jsrun.c:1307) | if (!js_iscallable(J, -n-2)) ... js_typeerror(J, "%s is not callable", js_typeof(J, -n-2)); | `js_typeerror` : "%s is not callable" || [x] |
| 173 | `js_construct` (jsrun.c:1341) | if (!js_iscallable(J, -n-1)) ... js_typeerror(J, "%s is not callable", js_typeof(J, -n-1)); | `js_typeerror` : "%s is not callable" || [x] |
| 174 | `js_endtry` (jsrun.c:1461) | if (J->trytop == 0) ... js_error(J, "endtry: exception stack underflow"); | `js_error` : "endtry: exception stack underflow" || [x] |
| 175 | `jsR_run` (jsrun.c:1673) | if (!js_hasvar(J, str)) ... js_referenceerror(J, "'%s' is not defined", str); | `js_referenceerror` : "'%s' is not defined" || [p] |
| 176 | `jsR_run` (jsrun.c:1698) | if (!js_hasvar(J, str)) ... js_referenceerror(J, "'%s' is not defined", str); | `js_referenceerror` : "'%s' is not defined" || [x] |
| 177 | `jsR_run` (jsrun.c:1721) | if (!js_isobject(J, -1)) ... js_typeerror(J, "operand to 'in' is not an object"); | `js_typeerror` : "operand to 'in' is not an object" || [x] |
| 178 | `js_newstate` (jsstate.c:191) | assert(sizeof(js_Value) == 16); | **SIGABRT** (assertion failure) || [p] |
| 179 | `js_newstate` (jsstate.c:192) | assert(soffsetof(js_Value, t.type) == 15); | **SIGABRT** (assertion failure) || [p] |
| 180 | `js_doregexec` (jsstring.c:9) | if (result < 0) ... js_error(J, "regexec failed"); | `js_error` : "regexec failed" || [x] |
| 181 | `checkstring` (jsstring.c:16) | if (!js_iscoercible(J, idx)) ... js_typeerror(J, "string function called on null or undefined"); | `js_typeerror` : "string function called on null or undefined" || [x] |
| 182 | `Sp_toString` (jsstring.c:108) | if (self->type != JS_CSTRING) js_typeerror(J, "not a string"); | `js_typeerror` : "not a string" || [x] |
| 183 | `Sp_valueOf` (jsstring.c:115) | if (self->type != JS_CSTRING) js_typeerror(J, "not a string"); | `js_typeerror` : "not a string" || [x] |
| 184 | `Sp_concat` (jsstring.c:163) | if (n > JS_STRLIMIT) ... js_rangeerror(J, "invalid string length"); | `js_rangeerror` : "invalid string length" || [x] |
| 185 | `Sp_concat` (jsstring.c:171) | if (n > JS_STRLIMIT) ... js_rangeerror(J, "invalid string length"); | `js_rangeerror` : "invalid string length" || [x] |
| 186 | `jsV_toprimitive` (jsvalue.c:144) | if (J->strict) ... js_typeerror(J, "cannot convert object to primitive"); | `js_typeerror` : "cannot convert object to primitive" || [x] |
| 187 | `jsV_toobject` (jsvalue.c:401) | case JS_TUNDEFINED: js_typeerror(J, "cannot convert undefined to object"); | `js_typeerror` : "cannot convert undefined to object" || [x] |
| 188 | `jsV_toobject` (jsvalue.c:402) | case JS_TNULL: js_typeerror(J, "cannot convert null to object"); | `js_typeerror` : "cannot convert null to object" || [x] |
| 189 | `js_instanceof` (jsvalue.c:579) | if (!js_iscallable(J, -1)) ... js_typeerror(J, "instanceof: invalid operand"); | `js_typeerror` : "instanceof: invalid operand" || [x] |
| 190 | `js_instanceof` (jsvalue.c:586) | if (!js_isobject(J, -1)) ... js_typeerror(J, "instanceof: 'prototype' property is not an object"); | `js_typeerror` : "instanceof: 'prototype' property is not an object" || [x] |
| 191 | `hex` (regexp.c:101) | if (c >= 'A' && c <= 'F') return c - 'A' + 0xA; ... die(g, "invalid escape sequence"); | `die` : "invalid escape sequence" || [x] |
| 192 | `dec` (regexp.c:108) | if (c >= '0' && c <= '9') return c - '0'; ... die(g, "invalid quantifier"); | `die` : "invalid quantifier" || [x] |
| 193 | `nextrune` (regexp.c:128) | if (!*g->source) ... die(g, "unterminated escape sequence"); | `die` : "unterminated escape sequence" || [x] |
| 194 | `nextrune` (regexp.c:138) | if (!g->source[0]) ... die(g, "unterminated escape sequence"); | `die` : "unterminated escape sequence" || [x] |
| 195 | `nextrune` (regexp.c:143) | if (!g->source[0] \|\| !g->source[1]) ... die(g, "unterminated escape sequence"); | `die` : "unterminated escape sequence" || [x] |
| 196 | `nextrune` (regexp.c:153) | if (!g->source[0] \|\| !g->source[1] \|\| !g->source[2] \|\| !g->source[3]) ... die(g, "unterminated escape sequence"); | `die` : "unterminated escape sequence" || [x] |
| 197 | `nextrune` (regexp.c:170) | if (isunicodeletter(g->yychar) \|\| g->yychar == '_') /* check identity escape */ ... die(g, "invalid escape character"); | `die` : "invalid escape character" || [x] |
| 198 | `lexcount` (regexp.c:186) | if (g->yymin >= REPINF) ... die(g, "numeric overflow"); | `die` : "numeric overflow" || [x] |
| 199 | `lexcount` (regexp.c:200) | if (g->yymax >= REPINF) ... die(g, "numeric overflow"); | `die` : "numeric overflow" || [x] |
| 200 | `newcclass` (regexp.c:213) | if (g->ncclass >= REG_MAXCLASS) ... die(g, "too many character classes"); | `die` : "too many character classes" || [x] |
| 201 | `addrange` (regexp.c:224) | if (a > b) ... die(g, "invalid character class range"); | `die` : "invalid character class range" || [x] |
| 202 | `addrange` (regexp.c:253) | if (cc->end + 2 >= cc->spans + nelem(cc->spans)) ... die(g, "too many character class ranges"); | `die` : "too many character class ranges" || [x] |
| 203 | `lexclass` (regexp.c:322) | if (g->yychar == EOF) ... die(g, "unterminated character class"); | `die` : "unterminated character class" || [x] |
| 204 | `newrep` (regexp.c:493) | if (max == REPINF && empty(atom)) ... die(g, "infinite loop matching the empty string"); | `die` : "infinite loop matching the empty string" || [x] |
| 205 | `parseatom` (regexp.c:541) | if (g->yychar == 0 \|\| g->yychar >= g->nsub \|\| !g->sub[g->yychar]) ... die(g, "invalid back-reference"); | `die` : "invalid back-reference" || [x] |
| 206 | `parseatom` (regexp.c:552) | if (g->nsub == REG_MAXSUB) ... die(g, "too many captures"); | `die` : "too many captures" || [x] |
| 207 | `parseatom` (regexp.c:557) | if (!accept(g, ')')) ... die(g, "unmatched '('"); | `die` : "unmatched '('" || [x] |
| 208 | `parseatom` (regexp.c:563) | if (!accept(g, ')')) ... die(g, "unmatched '('"); | `die` : "unmatched '('" || [x] |
| 209 | `parseatom` (regexp.c:570) | if (!accept(g, ')')) ... die(g, "unmatched '('"); | `die` : "unmatched '('" || [x] |
| 210 | `parseatom` (regexp.c:577) | if (!accept(g, ')')) ... die(g, "unmatched '('"); | `die` : "unmatched '('" || [x] |
| 211 | `parseatom` (regexp.c:580) | die(g, "syntax error"); | `die` : "syntax error" || [x] |
| 212 | `parserep` (regexp.c:598) | if (max < min) ... die(g, "invalid quantifier"); | `die` : "invalid quantifier" || [x] |
| 213 | `count` (regexp.c:661) | if (++depth > REG_MAXREC) die(g, "stack overflow"); | `die` : "stack overflow" || [x] |
| 214 | `count` (regexp.c:672) | if (n < 0 \|\| n > REG_MAXPROG) die(g, "program too large"); | `die` : "program too large" || [x] |
| 215 | `regcompx` (regexp.c:916) | if (!g.prog) ... die(&g, "cannot allocate regular expression"); | `die` : "cannot allocate regular expression" || [x] |
| 216 | `regcompx` (regexp.c:922) | if (n > REG_MAXPROG) ... die(&g, "program too large"); | `die` : "program too large" || [x] |
| 217 | `regcompx` (regexp.c:926) | if (!g.pstart) ... die(&g, "cannot allocate regular expression parse list"); | `die` : "cannot allocate regular expression parse list" || [x] |
| 218 | `regcompx` (regexp.c:940) | if (g.lookahead == ')') ... die(&g, "unmatched ')'"); | `die` : "unmatched ')'" || [x] |
| 219 | `regcompx` (regexp.c:942) | if (g.lookahead != EOF) ... die(&g, "syntax error"); | `die` : "syntax error" || [p] |
| 220 | `regcompx` (regexp.c:951) | if (n < 0 \|\| n > REG_MAXPROG) ... die(&g, "program too large"); | `die` : "program too large" || [x] |
| 221 | `regcompx` (regexp.c:956) | if (!g.prog->start) ... die(&g, "cannot allocate regular expression instruction list"); | `die` : "cannot allocate regular expression instruction list" || [x] |
| 222 | `regcompx` (regexp.c:961) | if (!g.prog->cclass) ... die(&g, "cannot allocate regular expression character class list"); | `die` : "cannot allocate regular expression character class list" || [x] |

| 223 | `jsY_expect` macro (jslex.c:177) | `if (!jsY_accept(J, x))` — expected char missing (e.g. unterminated regexp class) | `jsY_error` : "expected '%c'" || [x] |
| 224 | `jsP_expect` macro (jsparse.c:143) | `if (!jsP_accept(J, x))` — any expected token missing (`if (1`, `{`, `)`, ...) | `jsP_error` : "unexpected token: %s (expected %s)" || [x] |
| 225 | `INCREC` macro (jsparse.c:24) | `++J->astdepth > JS_ASTLIMIT` (400 nested expressions) | `jsP_error` : "too much recursion" || [x] |
| 226 | `jsonnext`/`jsonexpect` (json.c:41) | unexpected JSON token in `JSON.parse` | `js_syntaxerror` : "JSON: unexpected token: %s (expected %s)" || [x] |
| 227 | `js_ptry` (jsstate.c:6) | `J->trytop == JS_TRYLIMIT` (64) — try stack exhausted in `js_ploadstring`/`js_pcall`/`js_pconstruct` | returns 1, pushes litstr "exception stack overflow" || [x] |
| 228 | `js_ploadstring` (jsstate.c:36) | syntax error in source | returns 1 with error object on stack || [x] |
| 229 | `js_pcall` (jsstate.c:140) | callee throws / not callable | returns 1, `js_report` of error string || [x] |
| 230 | `js_pconstruct` (jsstate.c) | constructor throws / not a constructor | returns 1, `js_report` of error string || [x] |
| 231 | `js_dostring` (jsstate.c) | syntax error or runtime exception | returns 1, message via report callback || [x] |
| 232 | `js_newstate` (jsstate.c:198) | `alloc` returns NULL for the state or the value stack | returns `NULL` || [x] |
| 233 | `js_regcomp` (regexp.c) | any `die()` above; the error string is returned through `errorp` | returns `NULL`, `*errorp` set || [x] |
| 234 | `js_regexec` (regexp.c) | no match | returns `REG_NOMATCH` (1) || [x] |
| 235 | `js_regexec` (regexp.c) | recursion/backtrack limit exceeded during matching | returns non-zero error code || [x] |
| 236 | `jsV_resizearray`/array ops (jsvalue.c) | `newlen > JS_ARRAYLIMIT` (1<<26) or negative length | `js_rangeerror` : "invalid array length" || [x] |
| 237 | string concat / join (jsvalue.c, jsarray.c) | resulting length `> JS_STRLIMIT` (1<<28) | `js_rangeerror` : "invalid string length" || [p] |
| 238 | environment stack (jsrun.c) | `> JS_ENVLIMIT` (1024) nested scopes | `js_error` : "environment stack overflow" (stack overflow class) || [p] |
| 239 | value stack (jsrun.c) | push beyond `JS_STACKSIZE` (4096) | `js_error` : "stack overflow" || [x] |
| 240 | `js_setlimit` runlimit (jsrun.c:1602) | `runlimit` counts down to 1 during execution | `js_error` (run limit exceeded) via `js_runlimit` || [x] |
| 241 | `js_setlimit` memlimit (jsrun.c:55-72) | requested `size >= J->memlimit` in `js_malloc`/`js_realloc` | `js_error` (out of memory) || [x] |
| 242 | out-of-range enum across FFI | `js_newstate(NULL,NULL,0x7fffffff)`, `js_newregexp(...,flags=255)`, `js_regexec(...,eflags=255)`, `jsV_newobject(J,99,...)`, `jsY_tokenstring(9999)`, `js_toprimitive(J,idx,99)`, `js_strtol(s,ep,99)` | C accepts any int: only bit tests / table lookups happen; Rust must match bit-for-bit || [x] |
| 243 | NULL / boundary pointers | `js_pushstring(J,"")`, `js_pushlstring(J,s,0)`, `js_intern(J,"")`, `js_regcomp("",0,&err)`, `js_regexec(prog,"",NULL,0)`, `js_utflen("")`, `jsU_chartorune` on truncated sequence | must match C exactly || [x] |

## Coverage

Legend: `[x]` = a differential test constructs that exact invalid input and both
libraries reject identically. `[p]` = the branch is **unreachable from outside
the library** (dead code, or shadowed by an earlier identical check, or a
compile-time-true assert, or it needs >1 GB of live data); the test exercises
the nearest reachable trigger and the reason is documented in a comment next to
the test.

Tests: `translation/tests/errors_1.rs` (rows 1-64, 64 tests),
`errors_2.rs` (65-120, 56), `errors_3.rs` (121-190, 70),
`errors_4.rs` (191-243, 67). Each test is named `errNNN_*` after its row,
prints the row number, and asserts C and Rust produce byte-identical
stdout/stderr and the same exit status (including identical SIGABRT/SIGSEGV).

Rows marked `[p]` and why: 33, 38, 41 (jscompile.c branches the parser can
never produce), 57 (`assert(x.e == y.e)` cannot fail: `normalized_boundaries`
forces equal exponents), 67-69 (inside the `#if 0` block of jslex.c; the live
twins with the identical messages are tested), 153-156 (asserts in the static
`jsR_setarrayindex`, whose callers already guarantee the conditions), 155/237
(JS_ARRAYLIMIT/JS_STRLIMIT on a *flat* array / by concatenation would need
~1 GB; the identical checks on `a.length` and in `js_pushlstring` are tested),
169 (`js_delvar` in strict mode: `delete <ident>` is rejected at compile time),
175 (`OP_GETLOCAL` on an uninitialised local: `jsR_callfunction` always
`js_initvar`s), 178/179 (compile-time layout asserts), 219 (regexp.c "syntax
error" after `parsealt` is dead), 238 (`jsR_savescope`'s env-stack check is
shadowed by `jsR_pushtrace`'s "call stack overflow").

### The one behaviour that cannot be byte-identical

The C library is compiled with assertions live, so a failing `assert()` prints
a glibc diagnostic containing the **program name and the absolute path of the
C source file** (`prog: /…/c_src/src/jsdtoa.c:387: minus: Assertion 'x.f >= y.f'
failed.`) before aborting. No translation can reproduce those bytes. The Rust
library therefore aborts silently at exactly the same point, and the tests for
assert rows (57, 58, 135, and the `js_grisu2(0.0)` cases) compare
**stdout + exit status** with stderr discarded (`common::diff_stdout`), which
still pins the abort point (all output written before the abort must match) and
the SIGABRT status. `dtest/run.sh`-style comparisons strip those lines
(`scripts.rs::strip_assert`).
