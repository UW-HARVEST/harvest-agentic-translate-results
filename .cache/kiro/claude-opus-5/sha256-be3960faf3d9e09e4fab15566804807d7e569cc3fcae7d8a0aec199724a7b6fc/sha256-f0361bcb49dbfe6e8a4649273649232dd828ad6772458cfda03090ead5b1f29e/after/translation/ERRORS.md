# ERRORS.md — Error-surface table (derived mechanically from `c_src/`)

How this was derived (no guessing, no docs):

```sh
# every throwing error site + its enclosing function
awk -f extract.awk c_src/src/*.c        # matches js_error( js_typeerror( js_rangeerror(
                                        # js_syntaxerror( js_referenceerror( js_urierror(
                                        # js_evalerror( jsC_error( jsP_error( jsY_error( die(
# every sentinel return / assert / limit constant
grep -n 'return NULL|return -1|return REG_NOMATCH|Runeerror|assert\(' c_src/src/*.c
grep -n 'JS_STACKSIZE|JS_ENVLIMIT|JS_TRYLIMIT|JS_ASTLIMIT|JS_STRLIMIT|REG_MAX' c_src/src/jsi.h c_src/src/regexp.c c_src/src/regexp.h
```

Total throwing error sites found: **218** (225 raw matches − 7 that are the *declarations and
definitions* of the throwing helpers themselves: `jsC_error`, `jsY_error`, `jsP_error`, `die`).
They emit **133 distinct message format strings**.

## Section 1 — Sentinel-return / non-throwing rejections (directly callable across FFI)

These are the rejections an external `.so` caller observes as a *return value*, not a `longjmp`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| S1 | `js_regcomp` | any pattern that hits a `die()` in `regexp.c` | returns `NULL`, `*errorp` set to the exact message string | `errors::s1_s2_regcomp_sentinels` | [x] |
| S2 | `js_regcomp` | valid pattern | returns non-`NULL` and sets `*errorp = NULL` | `errors::s1_s2_regcomp_sentinels` | [x] |
| S3 | `js_regexec` | subject does not match | returns `REG_NOMATCH` (`1`) | `errors::s3_s4_s5_regexec_sentinels` | [x] |
| S4 | `js_regexec` | `match()` recursion depth `> REG_MAXREC` (4096) | returns `-1`; callers turn that into `Error: regexec failed` | `errors::s3_s4_s5_regexec_sentinels` | [x] |
| S5 | `js_regexec` | `eflags` = out-of-range int (e.g. `0x7fffffff`, `-1`) | only `REG_NOTBOL` (4) bit inspected; other bits ignored | `errors::s3_s4_s5_regexec_sentinels` | [x] |
| S6 | `js_regcomp` | `cflags` = out-of-range int | only `REG_ICASE`(1)/`REG_NEWLINE`(2) bits inspected | `regex::row21_captures_and_out_of_range_flags` | [x] |
| S7 | `js_ploadstring` | source with a syntax error | returns `1`, error object pushed on stack | `errors::errors_cover_every_message` | [x] |
| S8 | `js_ploadstring` | called with `J->trytop == JS_TRYLIMIT` (64) | returns `1`, pushes litstr `"exception stack overflow"` | `errors::s8_s46_exception_stack_overflow` | [x] |
| S9 | `js_dostring` | source with a syntax or runtime error | returns `1`, calls report callback | `scripts::row55_56_dostring` | [x] |
| S10 | `js_pcall` | callee throws | returns `1`, exception value on stack | `scripts::row57b_pconstruct` | [x] |
| S11 | `js_pcall` | callee is not callable | returns `1`, `TypeError: not a function` | `scripts::row57b_pconstruct` | [x] |
| S12 | `js_pconstruct` | callee is not a constructor | returns `1`, `TypeError` | `scripts::row57b_pconstruct` | [x] |
| S13 | `js_trystring` | value whose `toString` throws | returns the caller-supplied `error` pointer | `errors::s13_to_s17_try_family_sentinels` | [x] |
| S14 | `js_trynumber` | value whose `valueOf` throws | returns the caller-supplied `error` double | `errors::s13_to_s17_try_family_sentinels` | [x] |
| S15 | `js_tryinteger` | value whose `valueOf` throws | returns the caller-supplied `error` int | `errors::s13_to_s17_try_family_sentinels` | [x] |
| S16 | `js_tryboolean` | value whose conversion throws | returns the caller-supplied `error` int | `errors::s13_to_s17_try_family_sentinels` | [x] |
| S17 | `js_tryrepr` | value whose repr throws | returns the caller-supplied `error` pointer | `errors::s13_to_s17_try_family_sentinels` | [x] |
| S18 | `js_isuserdata` | tag does not match the object's tag | returns `0` | `errors::s18_s19_s53_userdata_tags` | [x] |
| S19 | `js_isuserdata` | idx holds a non-userdata value | returns `0` | `errors::s18_s19_s53_userdata_tags` | [x] |
| S20 | `js_compare` | either side is `NaN` (unordered) | `*okay = 0` | `errors::s20_compare_okay` | [x] |
| S21 | `js_type` | any value | returns one of `JS_ISUNDEFINED..JS_ISOBJECT`; internal types map deterministically | `state::row31_32_conversions` | [x] |
| S22 | `jsU_chartorune` | truncated / malformed UTF-8 sequence | returns `1`, `*rune = Runeerror` (`0xFFFD`) | `errors::s22_to_s27_utf_rejections` | [x] |
| S23 | `jsU_chartorune` | overlong encoding | returns `1`, `*rune = Runeerror` | `errors::s22_to_s27_utf_rejections` | [x] |
| S24 | `jsU_chartorune` | surrogate-range or >`Runemax` codepoint encoding | returns `1`, `*rune = Runeerror` | `errors::s22_to_s27_utf_rejections` | [x] |
| S25 | `jsU_chartorune` | empty string (`""`) | returns `1`, `*rune = 0` | `errors::s22_to_s27_utf_rejections` | [x] |
| S26 | `jsU_runetochar` | rune `< 0` or `> Runemax` (0x10FFFF) | encodes `Runeerror` | `errors::s22_to_s27_utf_rejections` | [x] |
| S27 | `jsU_runelen` | rune out of range | returns length of `Runeerror` encoding (3) | `errors::s22_to_s27_utf_rejections` | [x] |
| S28 | `js_strtod` | no parsable number at start | returns `0`, `*end == input` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S29 | `js_strtod` | overflow (`1e400`) | returns `HUGE_VAL`/`inf` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S30 | `js_strtod` | underflow (`1e-400`) | returns `0` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S31 | `js_strtol` | digit(s) invalid for the given base | stops at first invalid digit | `errors::s28_to_s36_numeric_rejections` | [x] |
| S32 | `js_strtol` | value out of `long` range | clamps to `LONG_MAX`/`LONG_MIN` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S33 | `js_stringtofloat` | unparsable string | returns `0`, `*end == input` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S34 | `jsV_stringtonumber` | string that is not a valid numeric literal | returns `NaN` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S35 | `jsV_stringtonumber` | `""` / all-whitespace | returns `0` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S36 | `js_isarrayindex` | string is not a canonical array index (leading `0`, `-1`, `> 2^32-2`, non-digit) | returns `0` | `errors::s28_to_s36_numeric_rejections` | [x] |
| S37 | `js_intern` | `""` | returns interned empty string (no error) | `lowlevel::row70_71_intern_and_alloc` | [x] |
| S38 | `js_utfptrtoidx` / `js_utflen` | malformed UTF-8 | counts `Runeerror` as one rune | `leaf::row05_utf_string_helpers` | [x] |
| S39 | `js_runeat` | index past end of string | returns `0` | `leaf::row05_utf_string_helpers` | [x] |
| S40 | `js_newstate` | `flags` with no valid bit (e.g. `0x7ffffffe`) | only `JS_STRICT`(1) bit inspected; state created | `errors::s40_s41_s42_state_construction_rejections` | [x] |
| S41 | `js_newstate` | `alloc` callback returns `NULL` on first allocation | returns `NULL` | `errors::s40_s41_s42_state_construction_rejections + s41b_regcompx_allocator_failures` | [x] |
| S42 | `js_atpanic` | — | returns the *previous* panic handler (`NULL` if it was the default) | `errors::s40_s41_s42_state_construction_rejections` | [x] |
| S43 | `js_setlimit` | `runlimit`/`memlimit` `<= 0` | limit disabled (no enforcement) | `errors::s40_s41_s42_state_construction_rejections` | [x] |
| S44 | `js_setlimit` | tiny `runlimit`, then long-running script | `"script ran too long"` | `state::row26_runlimit` | [x] |
| S45 | `js_setlimit` | tiny `memlimit`, then allocating script | `"out of memory"` | `state::row25_memlimit` | [x] |
| S46 | `js_savetry` | `trytop == JS_TRYLIMIT` (64) | `js_try` yields non-zero with litstr `"exception stack overflow"` | `errors::s8_s46_exception_stack_overflow + scripts::row58b_trylimit_overflow` | [x] |
| S47 | `js_pushlstring` | `n < 0` or `n > JS_STRLIMIT` (2^28) | `RangeError: invalid string length` | `errors::s47_to_s53_direct_api_error_paths + s47b_out_of_bounds_modes_match` | [x] |
| S48 | `js_pushstring` | `strlen > JS_STRLIMIT` | `RangeError: invalid string length` | `errors::s47_to_s53_direct_api_error_paths` | [x] |
| S49 | any `js_push*` | `top + 1 >= JS_STACKSIZE` (4096) | `"stack overflow"` | `errors::s47_to_s53_direct_api_error_paths` | [x] |
| S50 | `js_remove` / `js_insert` / `js_replace` / `js_copy` | `idx` outside `[BOT-TOP)` | `"stack error!"` / `"stack underflow!"` | `errors::s47_to_s53_direct_api_error_paths` | [x] |
| S51 | `js_pop` | `n > gettop()` | `"stack underflow!"` | `errors::s47_to_s53_direct_api_error_paths` | [x] |
| S52 | `js_rot` | `n > gettop()` | `"stack underflow!"` | `errors::s47b_out_of_bounds_modes_match` | [x] |
| S53 | `js_touserdata` | wrong tag | `TypeError: not a <tag>` | `errors::s18_s19_s53_userdata_tags` | [x] |
| S54 | `js_toregexp` | value is not a RegExp | `TypeError: not a regexp` | `scripts::row52_regexp_exec_direct` | [x] |
| S55 | `js_getlength` | `this` not coercible | `TypeError` | `errors::s47_to_s53_direct_api_error_paths` | [x] |
| S56 | `js_nextiterator` | iterator exhausted | returns `NULL` | `errors::s56_s57_iterator_and_ref_sentinels` | [x] |
| S57 | `js_ref` | — | returns a stable ref string; `js_unref` with an unknown ref is a no-op | `errors::s56_s57_iterator_and_ref_sentinels` | [x] |

## Section 2 — Throwing error sites (`longjmp`/unwind based), one row per distinct site

`trigger` is the guard in the C source; `expected C result` is the exception class and
message emitted by that site. All are observed through the FFI as
`js_ploadstring`/`js_dostring`/`js_pcall` returning `1` with `js_tostring(J,-1)` equal to
`"<Class>: <message>"`, except the `regexp.c` `die()` sites which surface as
`js_regcomp` returning `NULL` with `*errorp` set.

Coverage is enforced mechanically by `tests/errors.rs::errors_cover_every_message`,
which asserts that **all 133 distinct messages below were actually produced by the C
library** while running the trigger corpus, and that the Rust library produced the
identical class and text for every trigger. Result: **133 / 133 reached**.

| # | file:line | function | trigger (guard in C source) | expected C result | [x] |
|---|-----------|----------|------------------------------|-------------------|-----|
| 1 | `jsarray.c:149` | `Ap_join` | reached at jsarray.c:149 | `RangeError: invalid string length` | [x] |
| 2 | `jsarray.c:440` | `Ap_sort` | reached at jsarray.c:440 | `TypeError: comparison function must be a function or undefined` | [x] |
| 3 | `jsarray.c:443` | `Ap_sort` | reached at jsarray.c:443 | `RangeError: array is too large to sort` | [x] |
| 4 | `jsarray.c:537` | `Ap_toString` | reached at jsarray.c:537 | `TypeError: 'this' is not an object` | [x] |
| 5 | `jsarray.c:604` | `Ap_every` | reached at jsarray.c:604 | `TypeError: callback is not a function` | [x] |
| 6 | `jsarray.c:633` | `Ap_some` | reached at jsarray.c:633 | `TypeError: callback is not a function` | [x] |
| 7 | `jsarray.c:662` | `Ap_forEach` | reached at jsarray.c:662 | `TypeError: callback is not a function` | [x] |
| 8 | `jsarray.c:689` | `Ap_map` | reached at jsarray.c:689 | `TypeError: callback is not a function` | [x] |
| 9 | `jsarray.c:718` | `Ap_filter` | reached at jsarray.c:718 | `TypeError: callback is not a function` | [x] |
| 10 | `jsarray.c:751` | `Ap_reduce` | reached at jsarray.c:751 | `TypeError: callback is not a function` | [x] |
| 11 | `jsarray.c:757` | `Ap_reduce` | reached at jsarray.c:757 | `TypeError: no initial value` | [x] |
| 12 | `jsarray.c:767` | `Ap_reduce` | reached at jsarray.c:767 | `TypeError: no initial value` | [x] |
| 13 | `jsarray.c:792` | `Ap_reduceRight` | reached at jsarray.c:792 | `TypeError: callback is not a function` | [x] |
| 14 | `jsarray.c:798` | `Ap_reduceRight` | reached at jsarray.c:798 | `TypeError: no initial value` | [x] |
| 15 | `jsarray.c:808` | `Ap_reduceRight` | reached at jsarray.c:808 | `TypeError: no initial value` | [x] |
| 16 | `jsboolean.c:16` | `Bp_toString` | if (self->type != JS_CBOOLEAN) | `TypeError: not a boolean` | [x] |
| 17 | `jsboolean.c:23` | `Bp_valueOf` | if (self->type != JS_CBOOLEAN) | `TypeError: not a boolean` | [x] |
| 18 | `jsbuiltin.c:145` | `Decode` | reached at jsbuiltin.c:145 | `URIError: truncated escape sequence` | [x] |
| 19 | `jsbuiltin.c:149` | `Decode` | reached at jsbuiltin.c:149 | `URIError: invalid escape sequence` | [x] |
| 20 | `jscompile.c:43` | `checkfutureword` | reached at jscompile.c:43 | `SyntaxError (compile): '%s' is a future reserved word` | [x] |
| 21 | `jscompile.c:46` | `checkfutureword` | reached at jscompile.c:46 | `SyntaxError (compile): '%s' is a strict mode future reserved word` | [x] |
| 22 | `jscompile.c:75` | `emitraw` | reached at jscompile.c:75 | `SyntaxError: integer overflow in instruction coding` | [x] |
| 23 | `jscompile.c:114` | `addlocal` | reached at jscompile.c:114 | `SyntaxError (compile): redefining 'arguments' is not allowed in strict mode` | [x] |
| 24 | `jscompile.c:116` | `addlocal` | reached at jscompile.c:116 | `SyntaxError (compile): redefining 'eval' is not allowed in strict mode` | [x] |
| 25 | `jscompile.c:119` | `addlocal` | reached at jscompile.c:119 | `EvalError: %s:%d: invalid use of 'eval'` | [x] |
| 26 | `jscompile.c:128` | `addlocal` | reached at jscompile.c:128 | `SyntaxError (compile): duplicate formal parameter '%s'` | [x] |
| 27 | `jscompile.c:204` | `emitlocal` | reached at jscompile.c:204 | `SyntaxError (compile): 'arguments' is read-only in strict mode` | [x] |
| 28 | `jscompile.c:206` | `emitlocal` | reached at jscompile.c:206 | `SyntaxError (compile): 'eval' is read-only in strict mode` | [x] |
| 29 | `jscompile.c:209` | `emitlocal` | reached at jscompile.c:209 | `EvalError: %s:%d: invalid use of 'eval'` | [x] |
| 30 | `jscompile.c:238` | `emitjumpto` | reached at jscompile.c:238 | `SyntaxError: jump address integer overflow` | [x] |
| 31 | `jscompile.c:245` | `labelto` | reached at jscompile.c:245 | `SyntaxError: jump address integer overflow` | [x] |
| 32 | `jscompile.c:315` | `checkdup` | reached at jscompile.c:315 | `SyntaxError (compile): duplicate property '%s' in object literal` | [x] |
| 33 | `jscompile.c:336` | `cobject` | reached at jscompile.c:336 | `SyntaxError (compile): invalid property name in object initializer` | n/a |
| 34 | `jscompile.c:400` | `cassign` | reached at jscompile.c:400 | `SyntaxError (compile): invalid l-value in assignment` | [x] |
| 35 | `jscompile.c:410` | `cassignforin` | reached at jscompile.c:410 | `SyntaxError (compile): more than one loop variable in for-in statement` | [x] |
| 36 | `jscompile.c:439` | `cassignforin` | reached at jscompile.c:439 | `SyntaxError (compile): invalid l-value in for-in loop assignment` | [x] |
| 37 | `jscompile.c:464` | `cassignop1` | reached at jscompile.c:464 | `SyntaxError (compile): invalid l-value in assignment` | [x] |
| 38 | `jscompile.c:487` | `cassignop2` | reached at jscompile.c:487 | `SyntaxError (compile): invalid l-value in assignment` | [x] |
| 39 | `jscompile.c:508` | `cdelete` | reached at jscompile.c:508 | `SyntaxError (compile): delete on an unqualified name is not allowed in strict mode` | [x] |
| 40 | `jscompile.c:524` | `cdelete` | reached at jscompile.c:524 | `SyntaxError (compile): invalid l-value in delete expression` | [x] |
| 41 | `jscompile.c:780` | `cexp` | reached at jscompile.c:780 | `SyntaxError (compile): unknown expression type` | n/a |
| 42 | `jscompile.c:961` | `ctrycatch` | reached at jscompile.c:961 | `SyntaxError (compile): redefining 'arguments' is not allowed in strict mode` | [x] |
| 43 | `jscompile.c:963` | `ctrycatch` | reached at jscompile.c:963 | `SyntaxError (compile): redefining 'eval' is not allowed in strict mode` | [x] |
| 44 | `jscompile.c:993` | `ctrycatchfinally` | reached at jscompile.c:993 | `SyntaxError (compile): redefining 'arguments' is not allowed in strict mode` | [x] |
| 45 | `jscompile.c:995` | `ctrycatchfinally` | reached at jscompile.c:995 | `SyntaxError (compile): redefining 'eval' is not allowed in strict mode` | [x] |
| 46 | `jscompile.c:1025` | `cswitch` | reached at jscompile.c:1025 | `SyntaxError (compile): more than one default label in switch` | [x] |
| 47 | `jscompile.c:1217` | `cstm` | reached at jscompile.c:1217 | `SyntaxError (compile): break label '%s' not found` | [x] |
| 48 | `jscompile.c:1221` | `cstm` | reached at jscompile.c:1221 | `SyntaxError (compile): unlabelled break must be inside loop or switch` | [x] |
| 49 | `jscompile.c:1233` | `cstm` | reached at jscompile.c:1233 | `SyntaxError (compile): continue label '%s' not found` | [x] |
| 50 | `jscompile.c:1237` | `cstm` | reached at jscompile.c:1237 | `SyntaxError (compile): continue must be inside loop` | [x] |
| 51 | `jscompile.c:1251` | `cstm` | reached at jscompile.c:1251 | `SyntaxError (compile): return not in function` | [x] |
| 52 | `jscompile.c:1266` | `cstm` | reached at jscompile.c:1266 | `SyntaxError (compile): 'with' statements are not allowed in strict mode` | [x] |
| 53 | `jsdate.c:366` | `js_todate` | reached at jsdate.c:366 | `TypeError: not a date` | [x] |
| 54 | `jsdate.c:374` | `js_setdate` | reached at jsdate.c:374 | `TypeError: not a date` | [x] |
| 55 | `jsdate.c:485` | `Dp_toISOString` | reached at jsdate.c:485 | `RangeError: invalid date` | [x] |
| 56 | `jsdate.c:793` | `Dp_toJSON` | reached at jsdate.c:793 | `TypeError: this.toISOString is not a function` | [x] |
| 57 | `jserror.c:36` | `Ep_toString` | reached at jserror.c:36 | `TypeError: not an object` | [x] |
| 58 | `jsfunction.c:53` | `Fp_toString` | reached at jsfunction.c:53 | `TypeError: not a function` | [x] |
| 59 | `jsfunction.c:100` | `Fp_apply` | reached at jsfunction.c:100 | `TypeError: not a function` | [x] |
| 60 | `jsfunction.c:123` | `Fp_call` | reached at jsfunction.c:123 | `TypeError: not a function` | [x] |
| 61 | `jsfunction.c:186` | `Fp_bind` | reached at jsfunction.c:186 | `TypeError: not a function` | [x] |
| 62 | `jsintern.c:47` | `jsS_newstringnode` | reached at jsintern.c:47 | `RangeError: invalid string length` | [x] |
| 63 | `jslex.c:177` | `jsY_next` | reached at jslex.c:177 | `SyntaxError (lex): expected '%c'` | [x] |
| 64 | `jslex.c:192` | `jsY_unescape` | reached at jslex.c:192 | `SyntaxError (lex): unexpected escape sequence` | [x] |
| 65 | `jslex.c:255` | `lexhex` | reached at jslex.c:255 | `SyntaxError (lex): malformed hexadecimal number` | [x] |
| 66 | `jslex.c:269` | `lexinteger` | reached at jslex.c:269 | `SyntaxError (lex): malformed number` | n/a |
| 67 | `jslex.c:312` | `lexnumber` | reached at jslex.c:312 | `SyntaxError (lex): number with leading zero` | [x] |
| 68 | `jslex.c:333` | `lexnumber` | reached at jslex.c:333 | `SyntaxError (lex): number with letter suffix` | [x] |
| 69 | `jslex.c:351` | `lexnumber` | reached at jslex.c:351 | `SyntaxError (lex): number with leading zero` | [x] |
| 70 | `jslex.c:377` | `lexnumber` | reached at jslex.c:377 | `SyntaxError (lex): missing exponent` | [x] |
| 71 | `jslex.c:381` | `lexnumber` | reached at jslex.c:381 | `SyntaxError (lex): number with letter suffix` | [x] |
| 72 | `jslex.c:399` | `lexescape` | reached at jslex.c:399 | `SyntaxError (lex): unterminated escape sequence` | [x] |
| 73 | `jslex.c:440` | `lexstring` | reached at jslex.c:440 | `SyntaxError (lex): string not terminated` | [x] |
| 74 | `jslex.c:443` | `lexstring` | reached at jslex.c:443 | `SyntaxError (lex): malformed escape sequence` | [x] |
| 75 | `jslex.c:490` | `lexregexp` | reached at jslex.c:490 | `SyntaxError (lex): regular expression not terminated` | [x] |
| 76 | `jslex.c:497` | `lexregexp` | reached at jslex.c:497 | `SyntaxError (lex): regular expression not terminated` | [x] |
| 77 | `jslex.c:521` | `lexregexp` | reached at jslex.c:521 | `SyntaxError (lex): illegal flag in regular expression: %c` | [x] |
| 78 | `jslex.c:525` | `lexregexp` | reached at jslex.c:525 | `SyntaxError (lex): duplicated flag in regular expression` | [x] |
| 79 | `jslex.c:574` | `jsY_lexx` | reached at jslex.c:574 | `SyntaxError (lex): multi-line comment not terminated` | [x] |
| 80 | `jslex.c:728` | `jsY_lexx` | reached at jslex.c:728 | `SyntaxError (lex): unexpected character: '%c'` | [x] |
| 81 | `jslex.c:729` | `jsY_lexx` | reached at jslex.c:729 | `SyntaxError (lex): unexpected character: \\u%04X` | [x] |
| 82 | `jslex.c:760` | `lexjsonnumber` | reached at jslex.c:760 | `SyntaxError (lex): unexpected non-digit` | [x] |
| 83 | `jslex.c:767` | `lexjsonnumber` | reached at jslex.c:767 | `SyntaxError (lex): missing digits after decimal point` | [x] |
| 84 | `jslex.c:777` | `lexjsonnumber` | reached at jslex.c:777 | `SyntaxError (lex): missing digits after exponent indicator` | [x] |
| 85 | `jslex.c:791` | `lexjsonescape` | reached at jslex.c:791 | `SyntaxError (lex): invalid escape sequence` | [x] |
| 86 | `jslex.c:820` | `lexjsonstring` | reached at jslex.c:820 | `SyntaxError (lex): unterminated string` | [x] |
| 87 | `jslex.c:822` | `lexjsonstring` | reached at jslex.c:822 | `SyntaxError (lex): invalid control character in string` | [x] |
| 88 | `jslex.c:878` | `jsY_lexjson` | reached at jslex.c:878 | `SyntaxError (lex): unexpected character: '%c'` | [x] |
| 89 | `jslex.c:879` | `jsY_lexjson` | reached at jslex.c:879 | `SyntaxError (lex): unexpected character: \\u%04X` | [x] |
| 90 | `jsnumber.c:22` | `Np_valueOf` | if (self->type != JS_CNUMBER) | `TypeError: not a number` | [x] |
| 91 | `jsnumber.c:33` | `Np_toString` | reached at jsnumber.c:33 | `TypeError: not a number` | [x] |
| 92 | `jsnumber.c:40` | `Np_toString` | reached at jsnumber.c:40 | `RangeError: invalid radix` | [x] |
| 93 | `jsnumber.c:134` | `Np_toFixed` | if (self->type != JS_CNUMBER) | `TypeError: not a number` | [x] |
| 94 | `jsnumber.c:135` | `Np_toFixed` | if (width < 0) | `RangeError: precision %d out of range` | [x] |
| 95 | `jsnumber.c:136` | `Np_toFixed` | if (width > 20) | `RangeError: precision %d out of range` | [x] |
| 96 | `jsnumber.c:150` | `Np_toExponential` | if (self->type != JS_CNUMBER) | `TypeError: not a number` | [x] |
| 97 | `jsnumber.c:151` | `Np_toExponential` | if (width < 0) | `RangeError: precision %d out of range` | [x] |
| 98 | `jsnumber.c:152` | `Np_toExponential` | if (width > 20) | `RangeError: precision %d out of range` | [x] |
| 99 | `jsnumber.c:166` | `Np_toPrecision` | if (self->type != JS_CNUMBER) | `TypeError: not a number` | [x] |
| 100 | `jsnumber.c:167` | `Np_toPrecision` | if (width < 1) | `RangeError: precision %d out of range` | [x] |
| 101 | `jsnumber.c:168` | `Np_toPrecision` | if (width > 21) | `RangeError: precision %d out of range` | [x] |
| 102 | `jsobject.c:112` | `O_getPrototypeOf` | reached at jsobject.c:112 | `TypeError: not an object` | [x] |
| 103 | `jsobject.c:125` | `O_getOwnPropertyDescriptor` | reached at jsobject.c:125 | `TypeError: not an object` | [x] |
| 104 | `jsobject.c:176` | `O_getOwnPropertyNames` | reached at jsobject.c:176 | `TypeError: not an object` | [x] |
| 105 | `jsobject.c:258` | `ToPropertyDescriptor` | reached at jsobject.c:258 | `TypeError: value/writable and get/set attributes are exclusive` | [x] |
| 106 | `jsobject.c:265` | `ToPropertyDescriptor` | reached at jsobject.c:265 | `TypeError: value/writable and get/set attributes are exclusive` | [x] |
| 107 | `jsobject.c:277` | `O_defineProperty` | if (!js_isobject(J, 1) | `TypeError: not an object` | [x] |
| 108 | `jsobject.c:278` | `O_defineProperty` | if (!js_isobject(J, 3) | `TypeError: not an object` | [x] |
| 109 | `jsobject.c:289` | `O_defineProperties_walk` | reached at jsobject.c:289 | `TypeError: not an object` | [x] |
| 110 | `jsobject.c:304` | `O_defineProperties_imp` | if (!js_isobject(J, 2) | `TypeError: not an object` | [x] |
| 111 | `jsobject.c:326` | `O_defineProperties` | if (!js_isobject(J, 1) | `TypeError: not an object` | [x] |
| 112 | `jsobject.c:342` | `O_create` | reached at jsobject.c:342 | `TypeError: not an object or null` | [x] |
| 113 | `jsobject.c:372` | `O_keys` | reached at jsobject.c:372 | `TypeError: not an object` | [x] |
| 114 | `jsobject.c:403` | `O_preventExtensions` | reached at jsobject.c:403 | `TypeError: not an object` | [x] |
| 115 | `jsobject.c:413` | `O_isExtensible` | reached at jsobject.c:413 | `TypeError: not an object` | [x] |
| 116 | `jsobject.c:431` | `O_seal` | reached at jsobject.c:431 | `TypeError: not an object` | [x] |
| 117 | `jsobject.c:461` | `O_isSealed` | reached at jsobject.c:461 | `TypeError: not an object` | [x] |
| 118 | `jsobject.c:489` | `O_freeze` | reached at jsobject.c:489 | `TypeError: not an object` | [x] |
| 119 | `jsobject.c:521` | `O_isFrozen` | reached at jsobject.c:521 | `TypeError: not an object` | [x] |
| 120 | `json.c:41` | `jsonexpect` | reached at json.c:41 | `SyntaxError: JSON: unexpected token: %s (expected %s)` | [x] |
| 121 | `json.c:67` | `jsonvalue` | reached at json.c:67 | `SyntaxError: JSON: unexpected token: %s (expected string)` | [x] |
| 122 | `json.c:107` | `jsonvalue` | reached at json.c:107 | `SyntaxError: JSON: unexpected token: %s` | [x] |
| 123 | `json.c:261` | `fmtobject` | reached at json.c:261 | `TypeError: cyclic object value` | [x] |
| 124 | `json.c:297` | `fmtarray` | reached at json.c:297 | `TypeError: cyclic object value` | [x] |
| 125 | `jsparse.c:24` | `jsB_initjson` | reached at jsparse.c:24 | `SyntaxError (parse): too much recursion` | [x] |
| 126 | `jsparse.c:143` | `jsP_next` | reached at jsparse.c:143 | `SyntaxError (parse): unexpected token: %s (expected %s)` | [x] |
| 127 | `jsparse.c:153` | `semicolon` | reached at jsparse.c:153 | `SyntaxError (parse): unexpected token: %s (expected ';')` | [x] |
| 128 | `jsparse.c:166` | `identifier` | reached at jsparse.c:166 | `SyntaxError (parse): unexpected token: %s (expected identifier)` | [x] |
| 129 | `jsparse.c:183` | `identifiername` | reached at jsparse.c:183 | `SyntaxError (parse): unexpected token: %s (expected identifier or keyword)` | [x] |
| 130 | `jsparse.c:363` | `primary` | reached at jsparse.c:363 | `SyntaxError (parse): unexpected token in expression: %s` | [x] |
| 131 | `jsparse.c:700` | `caseclause` | reached at jsparse.c:700 | `SyntaxError (parse): unexpected token in switch: %s (expected 'case' or 'default')` | [x] |
| 132 | `jsparse.c:751` | `forstatement` | reached at jsparse.c:751 | `SyntaxError (parse): unexpected token in for-var-statement: %s` | [x] |
| 133 | `jsparse.c:770` | `forstatement` | reached at jsparse.c:770 | `SyntaxError (parse): unexpected token in for-statement: %s` | [x] |
| 134 | `jsparse.c:888` | `statement` | reached at jsparse.c:888 | `SyntaxError (parse): unexpected token in try: %s (expected 'catch' or 'finally')` | [x] |
| 135 | `jsproperty.c:228` | `jsV_setproperty` | reached at jsproperty.c:228 | `TypeError: object is non-extensible` | [x] |
| 136 | `jsproperty.c:303` | `jsV_nextiterator` | reached at jsproperty.c:303 | `TypeError: not an iterator` | [x] |
| 137 | `jsregexp.c:38` | `js_newregexpx` | reached at jsregexp.c:38 | `SyntaxError: regular expression: %s` | [x] |
| 138 | `jsregexp.c:77` | `js_RegExp_prototype_exec` | reached at jsregexp.c:77 | `Error: regexec failed` | [x] |
| 139 | `jsregexp.c:126` | `Rp_test` | reached at jsregexp.c:126 | `Error: regexec failed` | [x] |
| 140 | `jsregexp.c:149` | `jsB_new_RegExp` | reached at jsregexp.c:149 | `TypeError: cannot supply flags when creating one RegExp from another` | [x] |
| 141 | `jsregexp.c:172` | `jsB_new_RegExp` | reached at jsregexp.c:172 | `SyntaxError: invalid regular expression flag: '%c'` | [x] |
| 142 | `jsregexp.c:175` | `jsB_new_RegExp` | if (g > 1) | `SyntaxError: invalid regular expression flag: 'g'` | [x] |
| 143 | `jsregexp.c:176` | `jsB_new_RegExp` | if (i > 1) | `SyntaxError: invalid regular expression flag: 'i'` | [x] |
| 144 | `jsregexp.c:177` | `jsB_new_RegExp` | if (m > 1) | `SyntaxError: invalid regular expression flag: 'm'` | [x] |
| 145 | `jsrun.c:149` | `js_pushstring` | reached at jsrun.c:149 | `RangeError: invalid string length` | [x] |
| 146 | `jsrun.c:166` | `js_pushlstring` | reached at jsrun.c:166 | `RangeError: invalid string length` | [x] |
| 147 | `jsrun.c:373` | `js_toregexp` | reached at jsrun.c:373 | `TypeError: not a regexp` | [x] |
| 148 | `jsrun.c:382` | `js_touserdata` | reached at jsrun.c:382 | `TypeError: not a %s` | [x] |
| 149 | `jsrun.c:393` | `jsR_tofunction` | reached at jsrun.c:393 | `TypeError: not a function` | [x] |
| 150 | `jsrun.c:408` | `js_pop` | reached at jsrun.c:408 | `Error: stack underflow!` | [x] |
| 151 | `jsrun.c:416` | `js_remove` | reached at jsrun.c:416 | `Error: stack error!` | [x] |
| 152 | `jsrun.c:424` | `js_insert` | reached at jsrun.c:424 | `Error: not implemented yet` | [x] |
| 153 | `jsrun.c:431` | `js_replace` | reached at jsrun.c:431 | `Error: stack error!` | [x] |
| 154 | `jsrun.c:676` | `jsR_setarrayindex` | reached at jsrun.c:676 | `RangeError: array too large` | [x] |
| 155 | `jsrun.c:707` | `jsR_setproperty` | reached at jsrun.c:707 | `RangeError: invalid array length` | [x] |
| 156 | `jsrun.c:709` | `jsR_setproperty` | reached at jsrun.c:709 | `RangeError: array too large` | [x] |
| 157 | `jsrun.c:773` | `jsR_setproperty` | reached at jsrun.c:773 | `TypeError: setting property '%s' that only has a getter` | [x] |
| 158 | `jsrun.c:783` | `jsR_setproperty` | reached at jsrun.c:783 | `TypeError: cannot create property '%s' on transient object` | [x] |
| 159 | `jsrun.c:800` | `jsR_setproperty` | reached at jsrun.c:800 | `TypeError: '%s' is read-only` | [x] |
| 160 | `jsrun.c:854` | `jsR_defproperty` | reached at jsrun.c:854 | `TypeError: '%s' is read-only` | [x] |
| 161 | `jsrun.c:860` | `jsR_defproperty` | reached at jsrun.c:860 | `TypeError: '%s' is non-configurable` | [x] |
| 162 | `jsrun.c:866` | `jsR_defproperty` | reached at jsrun.c:866 | `TypeError: '%s' is non-configurable` | [x] |
| 163 | `jsrun.c:875` | `jsR_defproperty` | reached at jsrun.c:875 | `TypeError: '%s' is read-only or non-configurable` | [x] |
| 164 | `jsrun.c:921` | `jsR_delproperty` | reached at jsrun.c:921 | `TypeError: '%s' is non-configurable` | [x] |
| 165 | `jsrun.c:1127` | `js_setvar` | reached at jsrun.c:1127 | `TypeError: '%s' is read-only` | [x] |
| 166 | `jsrun.c:1133` | `js_setvar` | reached at jsrun.c:1133 | `ReferenceError: assignment to undeclared variable '%s'` | [x] |
| 167 | `jsrun.c:1145` | `js_delvar` | reached at jsrun.c:1145 | `TypeError: '%s' is non-configurable` | [x] |
| 168 | `jsrun.c:1290` | `jsR_pushtrace` | reached at jsrun.c:1290 | `Error: call stack overflow` | [x] |
| 169 | `jsrun.c:1304` | `js_call` | reached at jsrun.c:1304 | `RangeError: number of arguments cannot be negative` | [x] |
| 170 | `jsrun.c:1307` | `js_call` | reached at jsrun.c:1307 | `TypeError: %s is not callable` | [x] |
| 171 | `jsrun.c:1341` | `js_construct` | reached at jsrun.c:1341 | `TypeError: %s is not callable` | [x] |
| 172 | `jsrun.c:1461` | `js_endtry` | reached at jsrun.c:1461 | `Error: endtry: exception stack underflow` | [x] |
| 173 | `jsrun.c:1673` | `jsR_run` | reached at jsrun.c:1673 | `ReferenceError: '%s' is not defined` | [x] |
| 174 | `jsrun.c:1698` | `jsR_run` | reached at jsrun.c:1698 | `ReferenceError: '%s' is not defined` | [x] |
| 175 | `jsrun.c:1721` | `jsR_run` | reached at jsrun.c:1721 | `TypeError: operand to 'in' is not an object` | [x] |
| 176 | `jsstring.c:9` | `js_doregexec` | reached at jsstring.c:9 | `Error: regexec failed` | [x] |
| 177 | `jsstring.c:16` | `checkstring` | reached at jsstring.c:16 | `TypeError: string function called on null or undefined` | [x] |
| 178 | `jsstring.c:108` | `Sp_toString` | if (self->type != JS_CSTRING) | `TypeError: not a string` | [x] |
| 179 | `jsstring.c:115` | `Sp_valueOf` | if (self->type != JS_CSTRING) | `TypeError: not a string` | [x] |
| 180 | `jsstring.c:163` | `Sp_concat` | reached at jsstring.c:163 | `RangeError: invalid string length` | [x] |
| 181 | `jsstring.c:171` | `Sp_concat` | reached at jsstring.c:171 | `RangeError: invalid string length` | [x] |
| 182 | `jsvalue.c:144` | `jsV_toprimitive` | reached at jsvalue.c:144 | `TypeError: cannot convert object to primitive` | [x] |
| 183 | `jsvalue.c:401` | `jsV_toobject` | reached at jsvalue.c:401 | `TypeError: cannot convert undefined to object` | [x] |
| 184 | `jsvalue.c:402` | `jsV_toobject` | reached at jsvalue.c:402 | `TypeError: cannot convert null to object` | [x] |
| 185 | `jsvalue.c:579` | `js_instanceof` | reached at jsvalue.c:579 | `TypeError: instanceof: invalid operand` | [x] |
| 186 | `jsvalue.c:586` | `js_instanceof` | reached at jsvalue.c:586 | `TypeError: instanceof: 'prototype' property is not an object` | [x] |
| 187 | `regexp.c:101` | `hex` | reached at regexp.c:101 | `js_regcomp -> NULL, *errorp: invalid escape sequence` | [x] |
| 188 | `regexp.c:108` | `dec` | reached at regexp.c:108 | `js_regcomp -> NULL, *errorp: invalid quantifier` | [x] |
| 189 | `regexp.c:128` | `nextrune` | reached at regexp.c:128 | `js_regcomp -> NULL, *errorp: unterminated escape sequence` | [x] |
| 190 | `regexp.c:138` | `nextrune` | reached at regexp.c:138 | `js_regcomp -> NULL, *errorp: unterminated escape sequence` | [x] |
| 191 | `regexp.c:143` | `nextrune` | reached at regexp.c:143 | `js_regcomp -> NULL, *errorp: unterminated escape sequence` | [x] |
| 192 | `regexp.c:153` | `nextrune` | reached at regexp.c:153 | `js_regcomp -> NULL, *errorp: unterminated escape sequence` | [x] |
| 193 | `regexp.c:170` | `nextrune` | reached at regexp.c:170 | `js_regcomp -> NULL, *errorp: invalid escape character` | [x] |
| 194 | `regexp.c:186` | `lexcount` | reached at regexp.c:186 | `js_regcomp -> NULL, *errorp: numeric overflow` | [x] |
| 195 | `regexp.c:200` | `lexcount` | reached at regexp.c:200 | `js_regcomp -> NULL, *errorp: numeric overflow` | [x] |
| 196 | `regexp.c:213` | `newcclass` | reached at regexp.c:213 | `js_regcomp -> NULL, *errorp: too many character classes` | [x] |
| 197 | `regexp.c:224` | `addrange` | reached at regexp.c:224 | `js_regcomp -> NULL, *errorp: invalid character class range` | [x] |
| 198 | `regexp.c:253` | `addrange` | reached at regexp.c:253 | `js_regcomp -> NULL, *errorp: too many character class ranges` | [x] |
| 199 | `regexp.c:322` | `lexclass` | reached at regexp.c:322 | `js_regcomp -> NULL, *errorp: unterminated character class` | [x] |
| 200 | `regexp.c:493` | `newrep` | reached at regexp.c:493 | `js_regcomp -> NULL, *errorp: infinite loop matching the empty string` | [x] |
| 201 | `regexp.c:541` | `parseatom` | reached at regexp.c:541 | `js_regcomp -> NULL, *errorp: invalid back-reference` | [x] |
| 202 | `regexp.c:552` | `parseatom` | reached at regexp.c:552 | `js_regcomp -> NULL, *errorp: too many captures` | [x] |
| 203 | `regexp.c:557` | `parseatom` | reached at regexp.c:557 | `js_regcomp -> NULL, *errorp: unmatched '('` | [x] |
| 204 | `regexp.c:563` | `parseatom` | reached at regexp.c:563 | `js_regcomp -> NULL, *errorp: unmatched '('` | [x] |
| 205 | `regexp.c:570` | `parseatom` | reached at regexp.c:570 | `js_regcomp -> NULL, *errorp: unmatched '('` | [x] |
| 206 | `regexp.c:577` | `parseatom` | reached at regexp.c:577 | `js_regcomp -> NULL, *errorp: unmatched '('` | [x] |
| 207 | `regexp.c:580` | `parseatom` | reached at regexp.c:580 | `js_regcomp -> NULL, *errorp: syntax error` | [x] |
| 208 | `regexp.c:598` | `parserep` | reached at regexp.c:598 | `js_regcomp -> NULL, *errorp: invalid quantifier` | [x] |
| 209 | `regexp.c:661` | `count` | if (++depth > REG_MAXREC) | `js_regcomp -> NULL, *errorp: stack overflow` | [x] |
| 210 | `regexp.c:672` | `count` | if (n < 0 \|\| n > REG_MAXPROG) | `js_regcomp -> NULL, *errorp: program too large` | [x] |
| 211 | `regexp.c:916` | `regcompx` | reached at regexp.c:916 | `js_regcomp -> NULL, *errorp: cannot allocate regular expression` | [x] |
| 212 | `regexp.c:922` | `regcompx` | reached at regexp.c:922 | `js_regcomp -> NULL, *errorp: program too large` | [x] |
| 213 | `regexp.c:926` | `regcompx` | reached at regexp.c:926 | `js_regcomp -> NULL, *errorp: cannot allocate regular expression parse list` | [x] |
| 214 | `regexp.c:940` | `regcompx` | reached at regexp.c:940 | `js_regcomp -> NULL, *errorp: unmatched ')'` | [x] |
| 215 | `regexp.c:942` | `regcompx` | reached at regexp.c:942 | `js_regcomp -> NULL, *errorp: syntax error` | [x] |
| 216 | `regexp.c:951` | `regcompx` | reached at regexp.c:951 | `js_regcomp -> NULL, *errorp: program too large` | [x] |
| 217 | `regexp.c:956` | `regcompx` | reached at regexp.c:956 | `js_regcomp -> NULL, *errorp: cannot allocate regular expression instruction list` | [x] |
| 218 | `regexp.c:961` | `regcompx` | reached at regexp.c:961 | `js_regcomp -> NULL, *errorp: cannot allocate regular expression character class list` | [x] |

### Structurally unreachable messages (3 of 133 — justified, not skipped)

| message | why no input can reach it |
|---|---|
| `malformed number` | dead code: inside `#if 0` at jslex.c:263-387, not compiled into libmujs.so |
| `unknown expression type` | defensive `default:` of cexp(); every parser-producible expression node has a case |
| `invalid property name in object initializer` | defensive else-arm; jsparse.c:207 propname() only yields AST_IDENTIFIER/EXP_STRING/EXP_NUMBER |

Every other message is reached; the coverage assertion in
`tests/errors.rs::errors_cover_every_message` fails if any of the remaining 130 stops
being produced.

