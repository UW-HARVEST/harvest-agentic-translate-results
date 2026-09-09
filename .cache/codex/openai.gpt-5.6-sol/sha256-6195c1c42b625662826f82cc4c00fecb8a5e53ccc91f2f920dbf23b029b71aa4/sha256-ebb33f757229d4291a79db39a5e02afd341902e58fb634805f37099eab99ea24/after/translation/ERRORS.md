# Error surface

Generated from every C source occurrence of an explicit MuJS error/throw helper, regexp `die`, `assert`, or `return -1/NULL/NAN` sentinel. The source location keeps same-message branches distinct.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|----------|---------------------------------------------|-------------------|
| 1 | Ap_join (`jsarray.c:149`) | `n + seplen + rlen > JS_STRLIMIT` | raises/reports RangeError [x] |
| 2 | Ap_sort_cmp (`jsarray.c:286`) | `und_b` | returns `-1` [x] |
| 3 | Ap_sort_cmp (`jsarray.c:321`) | `control reaches this rejection site (return -1;)` | returns `-1` [x] |
| 4 | Ap_sort_cmp (`jsarray.c:336`) | `control reaches this rejection site (return -1;)` | returns `-1` [x] |
| 5 | Ap_sort (`jsarray.c:440`) | `!js_iscallable(J, 1) && !js_isundefined(J, 1)` | raises/reports TypeError [x] |
| 6 | Ap_sort (`jsarray.c:443`) | `len >= INT_MAX` | raises/reports RangeError [x] |
| 7 | Ap_toString (`jsarray.c:537`) | `!js_iscoercible(J, 0)` | raises/reports TypeError [x] |
| 8 | Ap_every (`jsarray.c:604`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 9 | Ap_some (`jsarray.c:633`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 10 | Ap_forEach (`jsarray.c:662`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 11 | Ap_map (`jsarray.c:689`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 12 | Ap_filter (`jsarray.c:718`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 13 | Ap_reduce (`jsarray.c:751`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 14 | Ap_reduce (`jsarray.c:757`) | `len == 0 && !hasinitial` | raises/reports TypeError [x] |
| 15 | Ap_reduce (`jsarray.c:767`) | `k == len` | raises/reports TypeError [x] |
| 16 | Ap_reduceRight (`jsarray.c:792`) | `!js_iscallable(J, 1)` | raises/reports TypeError [x] |
| 17 | Ap_reduceRight (`jsarray.c:798`) | `len == 0 && !hasinitial` | raises/reports TypeError [x] |
| 18 | Ap_reduceRight (`jsarray.c:808`) | `k < 0` | raises/reports TypeError [x] |
| 19 | Bp_toString (`jsboolean.c:16`) | `self->type != JS_CBOOLEAN` | raises/reports TypeError [x] |
| 20 | Bp_valueOf (`jsboolean.c:23`) | `self->type != JS_CBOOLEAN` | raises/reports TypeError [x] |
| 21 | Decode (`jsbuiltin.c:145`) | `!str[0] \|\| !str[1]` | raises/reports URIError [x] |
| 22 | Decode (`jsbuiltin.c:149`) | `!jsY_ishex(a) \|\| !jsY_ishex(b)` | raises/reports URIError [x] |
| 23 | <file-scope> (`jscompile.c:7`) | `control reaches this rejection site (JS_NORETURN void jsC_error(js_State *J, js_Ast *node, const char *fmt, ...) JS_PRINTFLIKE(3,4);)` | raises/reports SyntaxError [x] |
| 24 | <file-scope> (`jscompile.c:14`) | `control reaches this rejection site (void jsC_error(js_State *J, js_Ast *node, const char *fmt, ...))` | raises/reports SyntaxError [x] |
| 25 | checkfutureword (`jscompile.c:43`) | `jsY_findword(exp->string, futurewords, nelem(futurewords)) >= 0` | raises/reports SyntaxError [x] |
| 26 | checkfutureword (`jscompile.c:46`) | `jsY_findword(exp->string, strictfuturewords, nelem(strictfuturewords)) >= 0` | raises/reports SyntaxError [x] |
| 27 | emitraw (`jscompile.c:75`) | `value != (js_Instruction)value` | raises/reports SyntaxError [x] |
| 28 | addlocal (`jscompile.c:114`) | `!strcmp(name, "arguments")` | raises/reports SyntaxError [x] |
| 29 | addlocal (`jscompile.c:116`) | `!strcmp(name, "eval")` | raises/reports SyntaxError [x] |
| 30 | addlocal (`jscompile.c:119`) | `!strcmp(name, "eval")` | raises/reports EvalError [x] |
| 31 | addlocal (`jscompile.c:128`) | `F->strict` | raises/reports SyntaxError [x] |
| 32 | findlocal (`jscompile.c:146`) | `control reaches this rejection site (return -1;)` | returns `-1` [x] |
| 33 | emitlocal (`jscompile.c:204`) | `is_arguments` | raises/reports SyntaxError [x] |
| 34 | emitlocal (`jscompile.c:206`) | `is_eval` | raises/reports SyntaxError [x] |
| 35 | emitlocal (`jscompile.c:209`) | `is_eval` | raises/reports EvalError [x] |
| 36 | emitjumpto (`jscompile.c:238`) | `dest != (js_Instruction)dest` | raises/reports SyntaxError [x] |
| 37 | labelto (`jscompile.c:245`) | `addr != (js_Instruction)addr` | raises/reports SyntaxError [x] |
| 38 | checkdup (`jscompile.c:315`) | `!strcmp(needle, straw)` | raises/reports SyntaxError [x] |
| 39 | cobject (`jscompile.c:336`) | `control reaches this rejection site (jsC_error(J, prop, "invalid property name in object initializer");)` | raises/reports SyntaxError [x] |
| 40 | cassign (`jscompile.c:400`) | `control reaches this rejection site (jsC_error(J, lhs, "invalid l-value in assignment");)` | raises/reports SyntaxError [x] |
| 41 | cassignforin (`jscompile.c:410`) | `lhs->b` | raises/reports SyntaxError [x] |
| 42 | cassignforin (`jscompile.c:439`) | `control reaches this rejection site (jsC_error(J, lhs, "invalid l-value in for-in loop assignment");)` | raises/reports SyntaxError [x] |
| 43 | cassignop1 (`jscompile.c:464`) | `control reaches this rejection site (jsC_error(J, lhs, "invalid l-value in assignment");)` | raises/reports SyntaxError [x] |
| 44 | cassignop2 (`jscompile.c:487`) | `control reaches this rejection site (jsC_error(J, lhs, "invalid l-value in assignment");)` | raises/reports SyntaxError [x] |
| 45 | cdelete (`jscompile.c:508`) | `F->strict` | raises/reports SyntaxError [x] |
| 46 | cdelete (`jscompile.c:524`) | `control reaches this rejection site (jsC_error(J, exp, "invalid l-value in delete expression");)` | raises/reports SyntaxError [x] |
| 47 | cexp (`jscompile.c:780`) | `control reaches this rejection site (jsC_error(J, exp, "unknown expression type");)` | raises/reports SyntaxError [x] |
| 48 | breaktarget (`jscompile.c:846`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 49 | continuetarget (`jscompile.c:862`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 50 | returntarget (`jscompile.c:872`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 51 | ctrycatch (`jscompile.c:961`) | `!strcmp(catchvar->string, "arguments")` | raises/reports SyntaxError [x] |
| 52 | ctrycatch (`jscompile.c:963`) | `!strcmp(catchvar->string, "eval")` | raises/reports SyntaxError [x] |
| 53 | ctrycatchfinally (`jscompile.c:993`) | `!strcmp(catchvar->string, "arguments")` | raises/reports SyntaxError [x] |
| 54 | ctrycatchfinally (`jscompile.c:995`) | `!strcmp(catchvar->string, "eval")` | raises/reports SyntaxError [x] |
| 55 | cswitch (`jscompile.c:1025`) | `def` | raises/reports SyntaxError [x] |
| 56 | cstm (`jscompile.c:1217`) | `!target` | raises/reports SyntaxError [x] |
| 57 | cstm (`jscompile.c:1221`) | `!target` | raises/reports SyntaxError [x] |
| 58 | cstm (`jscompile.c:1233`) | `!target` | raises/reports SyntaxError [x] |
| 59 | cstm (`jscompile.c:1237`) | `!target` | raises/reports SyntaxError [x] |
| 60 | cstm (`jscompile.c:1251`) | `!target` | raises/reports SyntaxError [x] |
| 61 | cstm (`jscompile.c:1266`) | `F->strict` | raises/reports SyntaxError [x] |
| 62 | MakeDay (`jsdate.c:215`) | `im < 0 \|\| im >= 12` | returns `NAN` [x] |
| 63 | TimeClip (`jsdate.c:231`) | `!isfinite(t)` | returns `NAN` [x] |
| 64 | TimeClip (`jsdate.c:233`) | `fabs(t) > 8.64e15` | returns `NAN` [x] |
| 65 | parseDateTime (`jsdate.c:259`) | `!toint(&s, 4, &y` | returns `NAN` [x] |
| 66 | parseDateTime (`jsdate.c:262`) | `!toint(&s, 2, &m` | returns `NAN` [x] |
| 67 | parseDateTime (`jsdate.c:265`) | `!toint(&s, 2, &d` | returns `NAN` [x] |
| 68 | parseDateTime (`jsdate.c:271`) | `!toint(&s, 2, &H` | returns `NAN` [x] |
| 69 | parseDateTime (`jsdate.c:272`) | `*s != ':'` | returns `NAN` [x] |
| 70 | parseDateTime (`jsdate.c:274`) | `!toint(&s, 2, &M` | returns `NAN` [x] |
| 71 | parseDateTime (`jsdate.c:277`) | `!toint(&s, 2, &S` | returns `NAN` [x] |
| 72 | parseDateTime (`jsdate.c:280`) | `!toint(&s, 3, &ms` | returns `NAN` [x] |
| 73 | parseDateTime (`jsdate.c:290`) | `!toint(&s, 2, &tzh` | returns `NAN` [x] |
| 74 | parseDateTime (`jsdate.c:293`) | `!toint(&s, 2, &tzm` | returns `NAN` [x] |
| 75 | parseDateTime (`jsdate.c:295`) | `tzh > 23 \|\| tzm > 59` | returns `NAN` [x] |
| 76 | parseDateTime (`jsdate.c:302`) | `*s` | returns `NAN` [x] |
| 77 | parseDateTime (`jsdate.c:304`) | `m < 1 \|\| m > 12` | returns `NAN` [x] |
| 78 | parseDateTime (`jsdate.c:305`) | `d < 1 \|\| d > 31` | returns `NAN` [x] |
| 79 | parseDateTime (`jsdate.c:306`) | `H < 0 \|\| H > 24` | returns `NAN` [x] |
| 80 | parseDateTime (`jsdate.c:307`) | `M < 0 \|\| M > 59` | returns `NAN` [x] |
| 81 | parseDateTime (`jsdate.c:308`) | `S < 0 \|\| S > 59` | returns `NAN` [x] |
| 82 | parseDateTime (`jsdate.c:309`) | `ms < 0 \|\| ms > 999` | returns `NAN` [x] |
| 83 | parseDateTime (`jsdate.c:310`) | `H == 24 && (M != 0 \|\| S != 0 \|\| ms != 0` | returns `NAN` [x] |
| 84 | js_todate (`jsdate.c:366`) | `self->type != JS_CDATE` | raises/reports TypeError [x] |
| 85 | js_setdate (`jsdate.c:374`) | `self->type != JS_CDATE` | raises/reports TypeError [x] |
| 86 | Dp_toISOString (`jsdate.c:485`) | `!isfinite(t)` | raises/reports RangeError [x] |
| 87 | Dp_toJSON (`jsdate.c:793`) | `!js_iscallable(J, -1)` | raises/reports TypeError [x] |
| 88 | minus (`jsdtoa.c:386`) | `control reaches this rejection site (assert(x.e == y.e);)` | `assert(x.e == y.e)` must hold; otherwise abort [x] |
| 89 | minus (`jsdtoa.c:387`) | `control reaches this rejection site (assert(x.f >= y.f);)` | `assert(x.f >= y.f)` must hold; otherwise abort [x] |
| 90 | Ep_toString (`jserror.c:36`) | `!js_isobject(J, -1)` | raises/reports TypeError [x] |
| 91 | Fp_toString (`jsfunction.c:53`) | `!js_iscallable(J, 0)` | raises/reports TypeError [x] |
| 92 | Fp_apply (`jsfunction.c:100`) | `!js_iscallable(J, 0)` | raises/reports TypeError [x] |
| 93 | Fp_call (`jsfunction.c:123`) | `!js_iscallable(J, 0)` | raises/reports TypeError [x] |
| 94 | Fp_bind (`jsfunction.c:186`) | `!js_iscallable(J, 0)` | raises/reports TypeError [x] |
| 95 | jsS_newstringnode (`jsintern.c:47`) | `n > JS_STRLIMIT` | raises/reports RangeError [x] |
| 96 | <file-scope> (`jslex.c:4`) | `control reaches this rejection site (JS_NORETURN static void jsY_error(js_State *J, const char *fmt, ...) JS_PRINTFLIKE(2,3);)` | raises/reports SyntaxError [x] |
| 97 | <file-scope> (`jslex.c:6`) | `control reaches this rejection site (static void jsY_error(js_State *J, const char *fmt, ...))` | raises/reports SyntaxError [x] |
| 98 | jsY_findword (`jslex.c:95`) | `control reaches this rejection site (return -1;)` | returns `-1` [x] |
| 99 | <file-scope> (`jslex.c:177`) | `!jsY_accept(J, x` | raises/reports SyntaxError [x] |
| 100 | jsY_unescape (`jslex.c:192`) | `control reaches this rejection site (jsY_error(J, "unexpected escape sequence");)` | raises/reports SyntaxError [x] |
| 101 | lexcomment (`jslex.c:248`) | `control reaches this rejection site (return -1;)` | returns `-1` [x] |
| 102 | lexhex (`jslex.c:255`) | `!jsY_ishex(J->lexchar)` | raises/reports SyntaxError [x] |
| 103 | lexinteger (`jslex.c:269`) | `!jsY_isdec(J->lexchar)` | raises/reports SyntaxError [x] |
| 104 | lexnumber (`jslex.c:312`) | `jsY_isdec(J->lexchar)` | raises/reports SyntaxError [x] |
| 105 | lexnumber (`jslex.c:333`) | `jsY_isidentifierstart(J->lexchar)` | raises/reports SyntaxError [x] |
| 106 | lexnumber (`jslex.c:351`) | `jsY_isdec(J->lexchar)` | raises/reports SyntaxError [x] |
| 107 | lexnumber (`jslex.c:377`) | `control reaches this rejection site (jsY_error(J, "missing exponent");)` | raises/reports SyntaxError [x] |
| 108 | lexnumber (`jslex.c:381`) | `jsY_isidentifierstart(J->lexchar)` | raises/reports SyntaxError [x] |
| 109 | lexescape (`jslex.c:399`) | `control reaches this rejection site (case EOF: jsY_error(J, "unterminated escape sequence");)` | raises/reports SyntaxError [x] |
| 110 | lexstring (`jslex.c:440`) | `J->lexchar == EOF \|\| J->lexchar == '\n'` | raises/reports SyntaxError [x] |
| 111 | lexstring (`jslex.c:443`) | `lexescape(J)` | raises/reports SyntaxError [x] |
| 112 | lexregexp (`jslex.c:490`) | `J->lexchar == EOF \|\| J->lexchar == '\n'` | raises/reports SyntaxError [x] |
| 113 | lexregexp (`jslex.c:497`) | `J->lexchar == EOF \|\| J->lexchar == '\n'` | raises/reports SyntaxError [x] |
| 114 | lexregexp (`jslex.c:521`) | `jsY_accept(J, 'm')` | raises/reports SyntaxError [x] |
| 115 | lexregexp (`jslex.c:525`) | `g > 1 \|\| i > 1 \|\| m > 1` | raises/reports SyntaxError [x] |
| 116 | jsY_lexx (`jslex.c:574`) | `lexcomment(J)` | raises/reports SyntaxError [x] |
| 117 | jsY_lexx (`jslex.c:728`) | `J->lexchar >= 0x20 && J->lexchar <= 0x7E` | raises/reports SyntaxError [x] |
| 118 | jsY_lexx (`jslex.c:729`) | `control reaches this rejection site (jsY_error(J, "unexpected character: \\u%04X", J->lexchar);)` | raises/reports SyntaxError [x] |
| 119 | lexjsonnumber (`jslex.c:760`) | `control reaches this rejection site (jsY_error(J, "unexpected non-digit");)` | raises/reports SyntaxError [x] |
| 120 | lexjsonnumber (`jslex.c:767`) | `control reaches this rejection site (jsY_error(J, "missing digits after decimal point");)` | raises/reports SyntaxError [x] |
| 121 | lexjsonnumber (`jslex.c:777`) | `control reaches this rejection site (jsY_error(J, "missing digits after exponent indicator");)` | raises/reports SyntaxError [x] |
| 122 | lexjsonescape (`jslex.c:791`) | `control reaches this rejection site (default: jsY_error(J, "invalid escape sequence");)` | raises/reports SyntaxError [x] |
| 123 | lexjsonstring (`jslex.c:820`) | `J->lexchar == EOF` | raises/reports SyntaxError [x] |
| 124 | lexjsonstring (`jslex.c:822`) | `J->lexchar < 32` | raises/reports SyntaxError [x] |
| 125 | jsY_lexjson (`jslex.c:878`) | `J->lexchar >= 0x20 && J->lexchar <= 0x7E` | raises/reports SyntaxError [x] |
| 126 | jsY_lexjson (`jslex.c:879`) | `control reaches this rejection site (jsY_error(J, "unexpected character: \\u%04X", J->lexchar);)` | raises/reports SyntaxError [x] |
| 127 | Np_valueOf (`jsnumber.c:22`) | `self->type != JS_CNUMBER` | raises/reports TypeError [x] |
| 128 | Np_toString (`jsnumber.c:33`) | `self->type != JS_CNUMBER` | raises/reports TypeError [x] |
| 129 | Np_toString (`jsnumber.c:40`) | `radix < 2 \|\| radix > 36` | raises/reports RangeError [x] |
| 130 | Np_toFixed (`jsnumber.c:134`) | `self->type != JS_CNUMBER` | raises/reports TypeError [x] |
| 131 | Np_toFixed (`jsnumber.c:135`) | `width < 0` | raises/reports RangeError [x] |
| 132 | Np_toFixed (`jsnumber.c:136`) | `width > 20` | raises/reports RangeError [x] |
| 133 | Np_toExponential (`jsnumber.c:150`) | `self->type != JS_CNUMBER` | raises/reports TypeError [x] |
| 134 | Np_toExponential (`jsnumber.c:151`) | `width < 0` | raises/reports RangeError [x] |
| 135 | Np_toExponential (`jsnumber.c:152`) | `width > 20` | raises/reports RangeError [x] |
| 136 | Np_toPrecision (`jsnumber.c:166`) | `self->type != JS_CNUMBER` | raises/reports TypeError [x] |
| 137 | Np_toPrecision (`jsnumber.c:167`) | `width < 1` | raises/reports RangeError [x] |
| 138 | Np_toPrecision (`jsnumber.c:168`) | `width > 21` | raises/reports RangeError [x] |
| 139 | O_getPrototypeOf (`jsobject.c:112`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 140 | O_getOwnPropertyDescriptor (`jsobject.c:125`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 141 | O_getOwnPropertyNames (`jsobject.c:176`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 142 | ToPropertyDescriptor (`jsobject.c:258`) | `haswritable \|\| hasvalue` | raises/reports TypeError [x] |
| 143 | ToPropertyDescriptor (`jsobject.c:265`) | `haswritable \|\| hasvalue` | raises/reports TypeError [x] |
| 144 | O_defineProperty (`jsobject.c:277`) | `!js_isobject(J, 1` | raises/reports TypeError [x] |
| 145 | O_defineProperty (`jsobject.c:278`) | `!js_isobject(J, 3` | raises/reports TypeError [x] |
| 146 | O_defineProperties_walk (`jsobject.c:289`) | `ref->value.t.type != JS_TOBJECT` | raises/reports TypeError [x] |
| 147 | O_defineProperties_imp (`jsobject.c:304`) | `!js_isobject(J, 2` | raises/reports TypeError [x] |
| 148 | O_defineProperties (`jsobject.c:326`) | `!js_isobject(J, 1` | raises/reports TypeError [x] |
| 149 | O_create (`jsobject.c:342`) | `control reaches this rejection site (js_typeerror(J, "not an object or null");)` | raises/reports TypeError [x] |
| 150 | O_keys (`jsobject.c:372`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 151 | O_preventExtensions (`jsobject.c:403`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 152 | O_isExtensible (`jsobject.c:413`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 153 | O_seal (`jsobject.c:431`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 154 | O_isSealed (`jsobject.c:461`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 155 | O_freeze (`jsobject.c:489`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 156 | O_isFrozen (`jsobject.c:521`) | `!js_isobject(J, 1)` | raises/reports TypeError [x] |
| 157 | jsonexpect (`json.c:41`) | `!jsonaccept(J, t)` | raises/reports SyntaxError [x] |
| 158 | jsonvalue (`json.c:67`) | `J->lookahead != TK_STRING` | raises/reports SyntaxError [x] |
| 159 | jsonvalue (`json.c:107`) | `control reaches this rejection site (js_syntaxerror(J, "JSON: unexpected token: %s", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 160 | fmtobject (`json.c:261`) | `js_toobject(J, i) == js_toobject(J, -1)` | raises/reports TypeError [x] |
| 161 | fmtarray (`json.c:297`) | `js_toobject(J, i) == js_toobject(J, -1)` | raises/reports TypeError [x] |
| 162 | <file-scope> (`jsparse.c:22`) | `control reaches this rejection site (JS_NORETURN static void jsP_error(js_State *J, const char *fmt, ...) JS_PRINTFLIKE(2,3);)` | raises/reports SyntaxError [x] |
| 163 | <file-scope> (`jsparse.c:24`) | `++J->astdepth > JS_ASTLIMIT` | raises/reports SyntaxError [x] |
| 164 | <file-scope> (`jsparse.c:29`) | `control reaches this rejection site (static void jsP_error(js_State *J, const char *fmt, ...))` | raises/reports SyntaxError [x] |
| 165 | <file-scope> (`jsparse.c:143`) | `!jsP_accept(J, x` | raises/reports SyntaxError [x] |
| 166 | <file-scope> (`jsparse.c:153`) | `control reaches this rejection site (jsP_error(J, "unexpected token: %s (expected ';')", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 167 | identifier (`jsparse.c:166`) | `control reaches this rejection site (jsP_error(J, "unexpected token: %s (expected identifier)", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 168 | identifieropt (`jsparse.c:173`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 169 | identifiername (`jsparse.c:183`) | `control reaches this rejection site (jsP_error(J, "unexpected token: %s (expected identifier or keyword)", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 170 | arrayliteral (`jsparse.c:198`) | `J->lookahead == ']'` | returns `NULL` [x] |
| 171 | <file-scope> (`jsparse.c:256`) | `J->lookahead == '}'` | returns `NULL` [x] |
| 172 | parameters (`jsparse.c:272`) | `J->lookahead == ')'` | returns `NULL` [x] |
| 173 | primary (`jsparse.c:363`) | `control reaches this rejection site (jsP_error(J, "unexpected token in expression: %s", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 174 | arguments (`jsparse.c:370`) | `J->lookahead == ')'` | returns `NULL` [x] |
| 175 | <file-scope> (`jsparse.c:675`) | `J->lookahead == '}' \|\| J->lookahead == TK_CASE \|\| J->lookahead == TK_DEFAULT` | returns `NULL` [x] |
| 176 | <file-scope> (`jsparse.c:700`) | `control reaches this rejection site (jsP_error(J, "unexpected token in switch: %s (expected 'case' or 'default')", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 177 | <file-scope> (`jsparse.c:707`) | `J->lookahead == '}'` | returns `NULL` [x] |
| 178 | forstatement (`jsparse.c:751`) | `control reaches this rejection site (jsP_error(J, "unexpected token in for-var-statement: %s", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 179 | forstatement (`jsparse.c:770`) | `control reaches this rejection site (jsP_error(J, "unexpected token in for-statement: %s", jsY_tokenstring(J->lookahead));)` | raises/reports SyntaxError [x] |
| 180 | statement (`jsparse.c:888`) | `!b && !d` | raises/reports SyntaxError [x] |
| 181 | script (`jsparse.c:939`) | `J->lookahead == terminator` | returns `NULL` [x] |
| 182 | lookup (`jsproperty.c:57`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 183 | jsV_getpropertyx (`jsproperty.c:196`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 184 | jsV_getproperty (`jsproperty.c:207`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 185 | jsV_getenumproperty (`jsproperty.c:218`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 186 | jsV_setproperty (`jsproperty.c:228`) | `J->strict && !result` | raises/reports TypeError [x] |
| 187 | jsV_nextiterator (`jsproperty.c:303`) | `io->type != JS_CITERATOR` | raises/reports TypeError [x] |
| 188 | jsV_nextiterator (`jsproperty.c:315`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 189 | jsV_resizearray (`jsproperty.c:325`) | `control reaches this rejection site (assert(!obj->u.a.simple);)` | `assert(!obj->u.a.simple)` must hold; otherwise abort [x] |
| 190 | js_newregexpx (`jsregexp.c:38`) | `!prog` | raises/reports SyntaxError [x] |
| 191 | js_RegExp_prototype_exec (`jsregexp.c:77`) | `result < 0` | raises/reports Error [x] |
| 192 | Rp_test (`jsregexp.c:126`) | `result < 0` | raises/reports Error [x] |
| 193 | jsB_new_RegExp (`jsregexp.c:149`) | `js_isdefined(J, 2)` | raises/reports TypeError [x] |
| 194 | jsB_new_RegExp (`jsregexp.c:172`) | `*s == 'm'` | raises/reports SyntaxError [x] |
| 195 | jsB_new_RegExp (`jsregexp.c:175`) | `g > 1` | raises/reports SyntaxError [x] |
| 196 | jsB_new_RegExp (`jsregexp.c:176`) | `i > 1` | raises/reports SyntaxError [x] |
| 197 | jsB_new_RegExp (`jsregexp.c:177`) | `m > 1` | raises/reports SyntaxError [x] |
| 198 | <file-scope> (`jsrun.c:14`) | `control reaches this rejection site (static void js_trystackoverflow(js_State *J))` | raises/reports exception-stack-overflow error [x] |
| 199 | <file-scope> (`jsrun.c:22`) | `control reaches this rejection site (static void js_stackoverflow(js_State *J))` | raises/reports stack-overflow error [x] |
| 200 | <file-scope> (`jsrun.c:30`) | `control reaches this rejection site (static void js_outofmemory(js_State *J))` | raises/reports out-of-memory error [x] |
| 201 | <file-scope> (`jsrun.c:38`) | `control reaches this rejection site (static void js_runlimit(js_State *J))` | raises/reports run-limit error [x] |
| 202 | js_malloc (`jsrun.c:57`) | `size >= J->memlimit` | raises/reports out-of-memory error [x] |
| 203 | js_malloc (`jsrun.c:62`) | `!ptr` | raises/reports out-of-memory error [x] |
| 204 | js_realloc (`jsrun.c:71`) | `size >= J->memlimit` | raises/reports out-of-memory error [x] |
| 205 | js_realloc (`jsrun.c:76`) | `!ptr` | raises/reports out-of-memory error [x] |
| 206 | <file-scope> (`jsrun.c:106`) | `TOP + n >= JS_STACKSIZE` | raises/reports stack-overflow error [x] |
| 207 | js_pushstring (`jsrun.c:149`) | `n > JS_STRLIMIT` | raises/reports RangeError [x] |
| 208 | js_pushlstring (`jsrun.c:166`) | `n > JS_STRLIMIT` | raises/reports RangeError [x] |
| 209 | js_currentfunctiondata (`jsrun.c:215`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 210 | js_toregexp (`jsrun.c:373`) | `control reaches this rejection site (js_typeerror(J, "not a regexp");)` | raises/reports TypeError [x] |
| 211 | js_touserdata (`jsrun.c:382`) | `control reaches this rejection site (js_typeerror(J, "not a %s", tag);)` | raises/reports TypeError [x] |
| 212 | jsR_tofunction (`jsrun.c:389`) | `v->t.type == JS_TUNDEFINED \|\| v->t.type == JS_TNULL` | returns `NULL` [x] |
| 213 | jsR_tofunction (`jsrun.c:393`) | `control reaches this rejection site (js_typeerror(J, "not a function");)` | raises/reports TypeError [x] |
| 214 | js_pop (`jsrun.c:408`) | `control reaches this rejection site (js_error(J, "stack underflow!");)` | raises/reports Error [x] |
| 215 | js_remove (`jsrun.c:416`) | `idx < BOT \|\| idx >= TOP` | raises/reports Error [x] |
| 216 | js_insert (`jsrun.c:424`) | `control reaches this rejection site (js_error(J, "not implemented yet");)` | raises/reports Error [x] |
| 217 | js_replace (`jsrun.c:431`) | `idx < BOT \|\| idx >= TOP` | raises/reports Error [x] |
| 218 | jsR_setarrayindex (`jsrun.c:673`) | `control reaches this rejection site (assert(obj->u.a.simple);)` | `assert(obj->u.a.simple)` must hold; otherwise abort [x] |
| 219 | jsR_setarrayindex (`jsrun.c:674`) | `control reaches this rejection site (assert(k >= 0);)` | `assert(k >= 0)` must hold; otherwise abort [x] |
| 220 | jsR_setarrayindex (`jsrun.c:676`) | `newlen > JS_ARRAYLIMIT` | raises/reports RangeError [x] |
| 221 | jsR_setarrayindex (`jsrun.c:678`) | `newlen > obj->u.a.flat_length` | `assert(newlen == obj->u.a.flat_length + 1)` must hold; otherwise abort [x] |
| 222 | jsR_setproperty (`jsrun.c:707`) | `newlen != rawlen \|\| newlen < 0` | raises/reports RangeError [x] |
| 223 | jsR_setproperty (`jsrun.c:709`) | `newlen > JS_ARRAYLIMIT` | raises/reports RangeError [x] |
| 224 | jsR_setproperty (`jsrun.c:773`) | `ref->getter` | raises/reports TypeError [x] |
| 225 | jsR_setproperty (`jsrun.c:783`) | `J->strict` | raises/reports TypeError [x] |
| 226 | jsR_setproperty (`jsrun.c:800`) | `J->strict` | raises/reports TypeError [x] |
| 227 | jsR_defproperty (`jsrun.c:854`) | `J->strict` | raises/reports TypeError [x] |
| 228 | jsR_defproperty (`jsrun.c:860`) | `J->strict` | raises/reports TypeError [x] |
| 229 | jsR_defproperty (`jsrun.c:866`) | `J->strict` | raises/reports TypeError [x] |
| 230 | jsR_defproperty (`jsrun.c:875`) | `J->strict \|\| throw` | raises/reports TypeError [x] |
| 231 | jsR_delproperty (`jsrun.c:921`) | `J->strict` | raises/reports TypeError [x] |
| 232 | js_setvar (`jsrun.c:1127`) | `J->strict` | raises/reports TypeError [x] |
| 233 | js_setvar (`jsrun.c:1133`) | `J->strict` | raises/reports ReferenceError [x] |
| 234 | js_delvar (`jsrun.c:1145`) | `J->strict` | raises/reports TypeError [x] |
| 235 | jsR_savescope (`jsrun.c:1161`) | `J->envtop + 1 >= JS_ENVLIMIT` | raises/reports stack-overflow error [x] |
| 236 | jsR_pushtrace (`jsrun.c:1290`) | `J->tracetop + 1 == JS_ENVLIMIT` | raises/reports Error [x] |
| 237 | js_call (`jsrun.c:1304`) | `n < 0` | raises/reports RangeError [x] |
| 238 | js_call (`jsrun.c:1307`) | `!js_iscallable(J, -n-2)` | raises/reports TypeError [x] |
| 239 | js_construct (`jsrun.c:1341`) | `!js_iscallable(J, -n-1)` | raises/reports TypeError [x] |
| 240 | js_savetrypc (`jsrun.c:1433`) | `J->trytop == JS_TRYLIMIT` | raises/reports exception-stack-overflow error [x] |
| 241 | js_savetry (`jsrun.c:1447`) | `J->trytop == JS_TRYLIMIT` | raises/reports exception-stack-overflow error [x] |
| 242 | js_endtry (`jsrun.c:1461`) | `J->trytop == 0` | raises/reports Error [x] |
| 243 | jsR_run (`jsrun.c:1604`) | `J->runlimit == 1` | raises/reports run-limit error [x] |
| 244 | jsR_run (`jsrun.c:1673`) | `!js_hasvar(J, str)` | raises/reports ReferenceError [x] |
| 245 | jsR_run (`jsrun.c:1698`) | `!js_hasvar(J, str)` | raises/reports ReferenceError [x] |
| 246 | jsR_run (`jsrun.c:1721`) | `!js_isobject(J, -1)` | raises/reports TypeError [x] |
| 247 | js_defaultalloc (`jsstate.c:19`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 248 | js_newstate (`jsstate.c:191`) | `control reaches this rejection site (assert(sizeof(js_Value) == 16);)` | `assert(sizeof(js_Value) == 16)` must hold; otherwise abort [x] |
| 249 | js_newstate (`jsstate.c:192`) | `control reaches this rejection site (assert(soffsetof(js_Value, t.type) == 15);)` | `assert(soffsetof(js_Value, t.type) == 15)` must hold; otherwise abort [x] |
| 250 | js_newstate (`jsstate.c:199`) | `!J` | returns `NULL` [x] |
| 251 | js_newstate (`jsstate.c:217`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 252 | js_newstate (`jsstate.c:226`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 253 | js_doregexec (`jsstring.c:9`) | `result < 0` | raises/reports Error [x] |
| 254 | checkstring (`jsstring.c:16`) | `!js_iscoercible(J, idx)` | raises/reports TypeError [x] |
| 255 | Sp_toString (`jsstring.c:108`) | `self->type != JS_CSTRING` | raises/reports TypeError [x] |
| 256 | Sp_valueOf (`jsstring.c:115`) | `self->type != JS_CSTRING` | raises/reports TypeError [x] |
| 257 | Sp_concat (`jsstring.c:163`) | `n > JS_STRLIMIT` | raises/reports RangeError [x] |
| 258 | Sp_concat (`jsstring.c:171`) | `n > JS_STRLIMIT` | raises/reports RangeError [x] |
| 259 | ToPrimitive (`jsvalue.c:144`) | `J->strict` | raises/reports TypeError [x] |
| 260 | ToNumber (`jsvalue.c:242`) | `*e` | returns `NAN` [x] |
| 261 | ToNumber (`jsvalue.c:252`) | `control reaches this rejection site (case JS_TUNDEFINED: return NAN;)` | returns `NAN` [x] |
| 262 | ToObject (`jsvalue.c:401`) | `control reaches this rejection site (case JS_TUNDEFINED: js_typeerror(J, "cannot convert undefined to object");)` | raises/reports TypeError [x] |
| 263 | ToObject (`jsvalue.c:402`) | `control reaches this rejection site (case JS_TNULL: js_typeerror(J, "cannot convert null to object");)` | raises/reports TypeError [x] |
| 264 | js_instanceof (`jsvalue.c:579`) | `!js_iscallable(J, -1)` | raises/reports TypeError [x] |
| 265 | js_instanceof (`jsvalue.c:586`) | `!js_isobject(J, -1)` | raises/reports TypeError [x] |
| 266 | <file-scope> (`regexp.c:67`) | `control reaches this rejection site (static void die(struct cstate *g, const char *message))` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 267 | hex (`regexp.c:101`) | `c >= 'A' && c <= 'F'` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 268 | dec (`regexp.c:108`) | `c >= '0' && c <= '9'` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 269 | nextrune (`regexp.c:128`) | `!*g->source` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 270 | nextrune (`regexp.c:138`) | `!g->source[0]` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 271 | nextrune (`regexp.c:143`) | `!g->source[0] \|\| !g->source[1]` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 272 | nextrune (`regexp.c:153`) | `!g->source[0] \|\| !g->source[1] \|\| !g->source[2] \|\| !g->source[3]` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 273 | nextrune (`regexp.c:170`) | `isunicodeletter(g->yychar) \|\| g->yychar == '_'` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 274 | lexcount (`regexp.c:186`) | `g->yymin >= REPINF` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 275 | <file-scope> (`regexp.c:200`) | `g->yymax >= REPINF` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 276 | newcclass (`regexp.c:213`) | `g->ncclass >= REG_MAXCLASS` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 277 | addrange (`regexp.c:224`) | `a > b` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 278 | addrange (`regexp.c:253`) | `cc->end + 2 >= cc->spans + nelem(cc->spans)` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 279 | lexclass (`regexp.c:322`) | `g->yychar == EOF` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 280 | lex (`regexp.c:493`) | `max == REPINF && empty(atom)` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 281 | lex (`regexp.c:541`) | `g->yychar == 0 \|\| g->yychar >= g->nsub \|\| !g->sub[g->yychar]` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 282 | lex (`regexp.c:552`) | `g->nsub == REG_MAXSUB` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 283 | lex (`regexp.c:557`) | `!accept(g, ')')` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 284 | lex (`regexp.c:563`) | `!accept(g, ')')` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 285 | lex (`regexp.c:570`) | `!accept(g, ')')` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 286 | lex (`regexp.c:577`) | `!accept(g, ')')` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 287 | lex (`regexp.c:580`) | `control reaches this rejection site (die(g, "syntax error");)` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 288 | lex (`regexp.c:581`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 289 | lex (`regexp.c:598`) | `max < min` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 290 | lex (`regexp.c:623`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 291 | lex (`regexp.c:661`) | `++depth > REG_MAXREC` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 292 | lex (`regexp.c:672`) | `n < 0 \|\| n > REG_MAXPROG` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 293 | lex (`regexp.c:911`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 294 | lex (`regexp.c:916`) | `!g.prog` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 295 | lex (`regexp.c:922`) | `n > REG_MAXPROG` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 296 | lex (`regexp.c:926`) | `!g.pstart` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 297 | lex (`regexp.c:940`) | `g.lookahead == ')'` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 298 | lex (`regexp.c:942`) | `g.lookahead != EOF` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 299 | lex (`regexp.c:951`) | `n < 0 \|\| n > REG_MAXPROG` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 300 | lex (`regexp.c:956`) | `!g.prog->start` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 301 | lex (`regexp.c:961`) | `!g.prog->cclass` | raises/reports regexp compile failure (`NULL` plus error string) [x] |
| 302 | lex (`regexp.c:1002`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 303 | lex (`regexp.c:1056`) | `!*a` | returns `-1` [x] |
| 304 | lex (`regexp.c:1076`) | `depth > REG_MAXREC` | returns `-1` [x] |
| 305 | lex (`regexp.c:1089`) | `result == -1` | returns `-1` [x] |
| 306 | lex (`regexp.c:1100`) | `result == -1` | returns `-1` [x] |
| 307 | lex (`regexp.c:1109`) | `result == -1` | returns `-1` [x] |
| 308 | tolowerrune_full (`utf.c:294`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
| 309 | toupperrune_full (`utf.c:304`) | `control reaches this rejection site (return NULL;)` | returns `NULL` [x] |
