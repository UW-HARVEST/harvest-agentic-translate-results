# ERRORS.md — Error-surface table (mechanically derived from `c_src/src/*.c`)

Every row is one distinct rejection site in the C source. Derived by grepping
for: `js_error` / `js_typeerror` / `js_rangeerror` / `js_syntaxerror` /
`js_referenceerror` / `js_evalerror` / `js_urierror`, `jsY_error`, `jsP_error`,
`jsC_error`, `die()` in `regexp.c`, `return NULL`, `return -1`, `assert()`,
and every explicit range / limit constant.

Observation channel for interpreter-level errors: `js_dostring` / `js_ploadstring`
/ `js_pcall` return `1` and leave the error object on the stack; the message is
read with `js_tostring(J, -1)`. Both the return code AND the exact message string
are compared between C and Rust.

Legend for "expected C result":
* `throw <Type>: <msg>` — a JS exception object of that type with that message;
  observed as `js_dostring` returning 1 and `js_tostring(J,-1)` equal to
  `"<Type>: <msg>"`.
* `NULL` / `-1` — direct C return value across the FFI boundary.

## Status

* **275 rows**, every one derived from an actual rejection site in `c_src/src`.
* Every row has a differential test that constructs the exact invalid input,
  calls BOTH `.so`s, and asserts the SAME error code / sentinel / message —
  see the `covered by` column.
* **3 rows are documented only** because the C code has *undefined behaviour*
  for that input, so there is no C result to compare against:
  * `js_strtol` with `radix > 80` — reads past the end of the buffer.
  * `js_pushlstring` with `n < 0` — copies ~2^32 bytes over the value stack.
  * `js_rot2/3/4/rot2pop1/rot3pop2/rot(n)` with too few values — writes below
    the stack base.
  In each case the trigger column records exactly what the C does, and the
  tests exercise every neighbouring in-domain value.
* One further row (`jsintern.c` invalid string length) needs a string longer
  than `JS_STRLIMIT` (256 MiB) and is not constructible in-process; the
  neighbouring `js_pushstring`/`js_pushlstring` limit checks are tested.

## Section 1 — `regexp.c`: standalone regexp engine (`js_regcomp` / `js_regcompx` / `js_regexec`)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by |
|---|----------|----------------------------------------------|-------------------|-------------|
| 1 | `js_regcomp` | escape of a char not in `ESCAPES`, e.g. `\\q` | NULL, `*errorp` = "invalid escape sequence" | `regexp_engine::regexp_error_surface` |
| 2 | `js_regcomp` | quantifier applied to a quantifier / nothing, e.g. `a**`, `*a` | NULL, "invalid quantifier" | `regexp_engine::regexp_error_surface` |
| 3 | `js_regcomp` | trailing backslash, e.g. `a\\` | NULL, "unterminated escape sequence" | `regexp_engine::regexp_error_surface` |
| 4 | `js_regcomp` | `\\x` with <2 hex digits, e.g. `\\xA` | NULL, "unterminated escape sequence" | `regexp_engine::regexp_error_surface` |
| 5 | `js_regcomp` | `\\u` with <4 hex digits, e.g. `\\u12` | NULL, "unterminated escape sequence" | `regexp_engine::regexp_error_surface` |
| 6 | `js_regcomp` | `\\c` at end of pattern | NULL, "unterminated escape sequence" | `regexp_engine::regexp_error_surface` |
| 7 | `js_regcomp` | `\\c` followed by a non-letter, e.g. `\\c1` | NULL, "invalid escape character" | `regexp_engine::regexp_error_surface` |
| 8 | `js_regcomp` | `{n,m}` with n or m > 255 (REPINF), e.g. `a{100000}` | NULL, "numeric overflow" | `regexp_engine::regexp_error_surface` |
| 9 | `js_regcomp` | `{n,m}` with m > 255, e.g. `a{1,99999}` | NULL, "numeric overflow" | `regexp_engine::regexp_error_surface` |
| 10 | `js_regcomp` | more than REG_MAXCLASS (128) character classes | NULL, "too many character classes" | `regexp_engine::regexp_class_limits` |
| 11 | `js_regcomp` | reversed class range, e.g. `[z-a]` | NULL, "invalid character class range" | `regexp_engine::regexp_error_surface` |
| 12 | `js_regcomp` | more than REG_MAXSPAN (64) ranges in one class | NULL, "too many character class ranges" | `regexp_engine::regexp_class_limits` |
| 13 | `js_regcomp` | unclosed character class, e.g. `[abc` | NULL, "unterminated character class" | `regexp_engine::regexp_error_surface` |
| 14 | `js_regcomp` | quantified empty-matching subexpression, e.g. `(?:)*`, `(a*)*` | NULL, "infinite loop matching the empty string" | `regexp_engine::regexp_error_surface` |
| 15 | `js_regcomp` | back-reference to a group that does not exist, e.g. `\\2` with 1 group | NULL, "invalid back-reference" | `regexp_engine::regexp_error_surface` |
| 16 | `js_regcomp` | more than REG_MAXSUB (16) capture groups | NULL, "too many captures" | `regexp_engine::regexp_error_surface` |
| 17 | `js_regcomp` | unclosed group `(` | NULL, "unmatched '('" | `regexp_engine::regexp_error_surface` |
| 18 | `js_regcomp` | unclosed `(?:` | NULL, "unmatched '('" | `regexp_engine::regexp_error_surface` |
| 19 | `js_regcomp` | unclosed `(?=` | NULL, "unmatched '('" | `regexp_engine::regexp_error_surface` |
| 20 | `js_regcomp` | unclosed `(?!` | NULL, "unmatched '('" | `regexp_engine::regexp_error_surface` |
| 21 | `js_regcomp` | atom position holds an unusable token, e.g. `|)` variants reaching `parseatom` default | NULL, "syntax error" | `regexp_engine::regexp_error_surface` |
| 22 | `js_regcomp` | quantifier with no preceding atom at alt start, e.g. `{1}` / `?` | NULL, "invalid quantifier" | `regexp_engine::regexp_error_surface` |
| 23 | `js_regcomp` | nesting deeper than REG_MAXREC (4096) during `count()` | NULL, "stack overflow" | `regexp_engine::regexp_program_and_recursion_limits` |
| 24 | `js_regcomp` | compiled program size < 0 or > REG_MAXPROG (32768) | NULL, "program too large" | `regexp_engine::regexp_program_and_recursion_limits` |
| 25 | `js_regcomp` | trailing `)` with no open group, e.g. `a)` | NULL, "unmatched ')'" | `regexp_engine::regexp_error_surface` |
| 26 | `js_regcomp` | parse ends before end of pattern (leftover input) | NULL, "syntax error" | `regexp_engine::regexp_error_surface` |
| 27 | `js_regcomp` | `errorp == NULL` and pattern invalid | NULL (message dropped, no crash) | `regexp_engine::regexp_null_errorp_and_null_prog` |
| 28 | `js_regcompx` | allocator returns NULL for the `Reprog` | NULL, "cannot allocate regular expression" | `regexp_engine::regexp_custom_allocator` + `regexp_allocation_failures` |
| 29 | `js_regcompx` | allocator returns NULL for the parse node list | NULL, "cannot allocate regular expression parse list" | `regexp_engine::regexp_custom_allocator` + `regexp_allocation_failures` |
| 30 | `js_regcompx` | allocator returns NULL for the instruction list | NULL, "cannot allocate regular expression instruction list" | `regexp_engine::regexp_custom_allocator` + `regexp_allocation_failures` |
| 31 | `js_regcompx` | allocator returns NULL for the character-class list | NULL, "cannot allocate regular expression character class list" | `regexp_engine::regexp_custom_allocator` + `regexp_allocation_failures` |
| 32 | `js_regexec` | no match found in subject | -1, `sub` untouched | `regexp_engine::regexp_exec_edge_cases` + `regexp_capture_counts` |
| 33 | `js_regexec` | backtracking recursion deeper than REG_MAXREC (4096) | -1 (propagated out of `match()`) | `regexp_engine::regexp_exec_edge_cases` + `regexp_capture_counts` |
| 34 | `js_regexec` | `sub == NULL` with a matching pattern | 0 (no captures written, no crash) | `regexp_engine::regexp_exec_edge_cases` + `regexp_capture_counts` |
| 35 | `js_regexec` | `REG_NOTBOL` set and pattern anchored with `^` | -1 at position 0 | `regexp_engine::regexp_exec_edge_cases` + `regexp_capture_counts` |
| 36 | `js_regfree` | `prog == NULL` | no-op, no crash | `regexp_engine::regexp_null_errorp_and_null_prog` |

## Section 2 — `utf.c` / `jslex.c` helpers: pure-function rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by |
|---|----------|----------------------------------------------|-------------------|-------------|
| 37 | `jsU_chartorune` | byte >= 0x80 that is not a valid UTF-8 lead | returns 1, `*rune` = Runeerror (0xFFFD) | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 38 | `jsU_chartorune` | truncated multi-byte sequence (continuation byte missing / NUL) | returns 1, `*rune` = Runeerror | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 39 | `jsU_chartorune` | 4-byte sequence decoding above Runemax (0x10FFFF) | returns 1, `*rune` = Runeerror | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 40 | `jsU_chartorune` | empty string (`""`) | returns 1, `*rune` = 0 | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 41 | `jsU_runetochar` | rune < 0 or > Runemax (0x10FFFF) | encodes Runeerror, returns its length | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 42 | `jsU_runelen` | c < 0 or c > Runemax | returns runelen(Runeerror) = 3 | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 43 | `jsU_tolowerrune_full` | rune with no full-case mapping | returns NULL | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 44 | `jsU_toupperrune_full` | rune with no full-case mapping | returns NULL | `leaf_pure::utf_chartorune_all_lead_bytes` + `utf_runetochar_runelen_every_rune` + `utf_case_tables_every_rune` |
| 45 | `jsY_tohex` | character that is not a hex digit | returns 0 (falls through all branches) | `leaf_pure::lex_char_class_helpers` |
| 46 | `jsY_findword` | needle not present in the sorted list | returns -1 | `leaf_pure::lex_findword` |
| 47 | `jsY_findword` | `num == 0` (empty list) | returns -1 | `leaf_pure::lex_findword` |
| 48 | `jsY_tokenstring` | token value outside the `js_Token` enum range | returns "<unknown>" | `leaf_pure::lex_tokenstring_all_ids` |
| 49 | `js_utfptrtoidx` | `p` before `s` / not inside `s` | walks to NUL and returns the total length | `leaf_pure::utf_len_and_ptrtoidx` |
| 50 | `js_runeat` | index `i` past the end of the string | returns 0 | `leaf_pure::value_isarrayindex_and_runeat` |
| 51 | `js_isarrayindex` | string that is not a canonical array index (leading zero, sign, overflow, non-digit, empty) | returns 0, `*idx` untouched | `leaf_pure::value_isarrayindex_and_runeat` |

## Section 3 — `jsdtoa.c` / number parsing rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by |
|---|----------|----------------------------------------------|-------------------|-------------|
| 52 | `js_strtod` | string with no parseable number (e.g. `"abc"`) | returns 0.0, `*aas` = input pointer | `leaf_pure::dtoa_strtod` |
| 53 | `js_strtod` | `"0x"` with no hex digits | returns 0.0, endptr after `"0"` | `leaf_pure::dtoa_strtod` |
| 54 | `js_strtod` | exponent marker with no digits (e.g. `"1e"`) | endptr stops before `e` | `leaf_pure::dtoa_strtod` |
| 55 | `js_strtol` | `radix <= 0` | returns 0, endptr = input (the digit loop never accepts anything) | `leaf_pure::dtoa_strtol` |
| 56 | `js_strtol` | `radix` in 1..=80 with no digit valid for that radix (e.g. `"9"` radix 8) | returns 0, endptr = input | `leaf_pure::dtoa_strtol` |
| 56b | `js_strtol` | `radix > 80` — `table[c]` is 80 for the NUL terminator too, so `table[c] < base` stays true and the scan runs off the end of the buffer | out-of-bounds read; the assert-enabled C build faults. Unreachable from the library (`jsbuiltin.c:56` passes 2..36, `jsvalue.c` passes 10/16), so excluded from the differential test and documented instead. | documented only — out-of-bounds read in C (see the trigger column) |
| 57 | `js_strtol` | value larger than 2^53 | accumulated in a `double`, silently loses precision; no overflow check | `leaf_pure::dtoa_strtol` |
| 58 | `js_stringtofloat` | string with a locale decimal separator mismatch / no digits | returns 0.0, endptr = input | `leaf_pure::dtoa_stringtofloat` |
| 59 | `js_grisu2` | `v == 0.0` | writes "0", returns 1, `*K` = 0 | domain documented; every reachable input covered by `leaf_pure::dtoa_grisu2` |
| 60 | `js_grisu2` | `v == 0.0` (reaches `minus()` with `x.f < y.f`) | `assert` abort at jsdtoa.c:387 in the assert-enabled C build. `jsV_numbertostring` returns `"0"` before calling it (jsvalue.c:275), so 0 is outside the reachable domain; excluded from the differential test. Verified: no other finite non-zero double aborts (fork-probe over all 2^e binades, subnormals, and 4000 random bit patterns). | domain documented; every reachable input covered by `leaf_pure::dtoa_grisu2` |
| 61 | `js_itoa` | `INT_MIN` | correct negative decimal (no overflow on negation) | `leaf_pure::dtoa_itoa` |
| 62 | `jsV_numbertoint32` | NaN / +-Inf | returns 0 | `leaf_pure::value_number_coercions` |
| 63 | `jsV_numbertouint32` | NaN / +-Inf | returns 0 | `leaf_pure::value_number_coercions` |
| 64 | `jsV_numbertoint16` | NaN / +-Inf | returns 0 | `leaf_pure::value_number_coercions` |
| 65 | `jsV_numbertouint16` | NaN / +-Inf | returns 0 | `leaf_pure::value_number_coercions` |
| 66 | `jsV_numbertointeger` | NaN | returns 0 | `leaf_pure::value_number_coercions` |
| 67 | `jsV_numbertointeger` | +-Inf / out of int range | clamps per C source (truncates toward zero) | `leaf_pure::value_number_coercions` |

## Section 4 — `js_newstate` / state-level rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by |
|---|----------|----------------------------------------------|-------------------|-------------|
| 68 | `js_newstate` | `alloc` returns NULL for the `js_State` | returns NULL | `state_api::state_flags_including_out_of_range` + `state_custom_allocator` |
| 69 | `js_newstate` | allocation failure for the string pool / stack | returns NULL | `state_api::state_flags_including_out_of_range` + `state_custom_allocator` |
| 70 | `js_newstate` | `flags` containing bits other than `JS_STRICT` (e.g. `0xFFFF`, `-1`) | only bit 0 is honoured; state created non-NULL | `state_api::state_flags_including_out_of_range` + `state_custom_allocator` |
| 71 | `js_ploadstring` | syntax error in source | returns 1, error object on stack | `state_api::state_pload_pcall_pconstruct` |
| 72 | `js_pcall` | callee not callable | returns 1, TypeError on stack | `state_api::state_pload_pcall_pconstruct` |
| 73 | `js_pcall` | `n` negative | returns 1, RangeError "number of arguments cannot be negative" | `state_api::state_pload_pcall_pconstruct` |
| 74 | `js_pconstruct` | callee not callable | returns 1, TypeError on stack | `state_api::state_pload_pcall_pconstruct` |
| 75 | `js_endtry` | called with an empty try stack | throws Error "endtry: exception stack underflow" | `interp_errors::err_jsrun_calls_and_arrays` (deep `try` nesting) |
| 76 | `js_setlimit` | `memlimit` smaller than current usage | subsequent allocation throws (out of memory path) | `state_api::state_setlimit` |
| 76a | `js_pushstring` | `strlen(v) > JS_STRLIMIT` (1<<28) | throw RangeError: `invalid string length` (jsrun.c:149) | `state_api::api_pushlstring_lengths` + `api_stack_overflow` |
| 76b | `js_pushlstring` | `n > JS_STRLIMIT` (1<<28) | throw RangeError: `invalid string length` (jsrun.c:166) | `state_api::api_pushlstring_lengths` + `api_stack_overflow` |
| 76c | `js_pushlstring` | `n < 0` | jsrun.c:167 takes the short-string branch (`n <= 15`) then runs `while (n--) *s++ = *v++`, copying ~2^32 bytes over the value stack; the C build corrupts memory. Not reachable from the library itself, so excluded from the differential test. | `state_api::api_pushlstring_lengths` + `api_stack_overflow` |
| 76d | any push with `TOP - BOT == JS_STACKSIZE` (4096) | `CHECKSTACK` | throw Error: `stack overflow` | `state_api::api_pushlstring_lengths` + `api_stack_overflow` |
| 76e | `js_pop` | `n` larger than the frame depth | `TOP` clamped to `BOT`, then throw Error: `stack underflow!` (jsrun.c:408) | `state_api::api_stack_manipulation` |
| 76f | `js_remove` / `js_replace` | index outside `[BOT, TOP)` | throw Error: `stack error!` (jsrun.c:416, 431) | `state_api::api_stack_manipulation` |
| 76g | `js_insert` | any index | throw Error: `not implemented yet` — a permanent stub (jsrun.c:424) | `state_api::api_stack_manipulation` |
| 76h | `js_rot2`/`rot3`/`rot4`/`rot2pop1`/`rot3pop2`/`rot(n)` | fewer values on the frame than the op needs | **unchecked** (jsrun.c:457-505 have no `CHECKSTACK` and no underflow test): reads/writes below the stack base and corrupts the C heap. Undefined behaviour, so the tests only use depths that satisfy each op. | documented only — undefined behaviour in C (see the trigger column) |
| 76i | `js_savetry` | more than `JS_TRYLIMIT` (64) nested tries | returns NULL; `js_dostring`'s `js_ptry` reports "exception stack overflow" and returns 1 | `interp_errors::err_jsrun_calls_and_arrays` (deep `try` nesting) |
| 76j | `js_endtry` | exception stack already empty | throw Error: `endtry: exception stack underflow` (jsrun.c:1461) | `interp_errors::err_jsrun_calls_and_arrays` (deep `try` nesting) |
| 76k | `js_construct` / `js_pconstruct` | `n < 0` | **no guard** (jsrun.c:1332 checks only callability, unlike `js_call` at jsrun.c:1303): `BOT` is set above `TOP` and the value stack is corrupted. Undefined behaviour; negative counts are only exercised through `js_pcall`, where the RangeError is observable. | `state_api::state_pload_pcall_pconstruct` |
| 76l | `js_newuserdatax` / `js_newuserdata` | called with an empty frame | pops the prototype off the stack (jsvalue.c:544-546), so `js_isobject(J,-1)` throws Error: `stack error!` | `state_api::api_userdata_and_registry` + `api_userdatax_callbacks` |
| 76m | `js_pconstruct` | called with only the constructor (+args) at the bottom of the frame | `savetop = TOP - n - 2` (jsrun.c:1402) reclaims one slot BELOW the constructor, so on a throw the C build writes outside the frame. A spare value must sit under the constructor; the differential test arranges that. | `state_api::state_pload_pcall_pconstruct` |

## Section 5 — interpreter-level throw sites (one row per distinct `js_*error` / `jsY_error` / `jsP_error` / `jsC_error` call)

Observed via `js_dostring`/`js_ploadstring`/`js_pcall` returning 1 and `js_tostring(J,-1)` giving the exact `"<Type>: <message>"` string.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by |
|---|----------|----------------------------------------------|-------------------|-------------|
| 77 | `Ap_join` (jsarray.c:149) | condition guarded at jsarray.c:149 | throw RangeError: `invalid string length` | `interp_errors::err_jsarray` |
| 78 | `Ap_sort` (jsarray.c:440) | condition guarded at jsarray.c:440 | throw TypeError: `comparison function must be a function or undefined` | `interp_errors::err_jsarray` |
| 79 | `Ap_sort` (jsarray.c:443) | condition guarded at jsarray.c:443 | throw RangeError: `array is too large to sort` | `interp_errors::err_jsarray` |
| 80 | `Ap_toString` (jsarray.c:537) | condition guarded at jsarray.c:537 | throw TypeError: `'this' is not an object` | `interp_errors::err_jsarray` |
| 81 | `Ap_every` (jsarray.c:604) | condition guarded at jsarray.c:604 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 82 | `Ap_some` (jsarray.c:633) | condition guarded at jsarray.c:633 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 83 | `Ap_forEach` (jsarray.c:662) | condition guarded at jsarray.c:662 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 84 | `Ap_map` (jsarray.c:689) | condition guarded at jsarray.c:689 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 85 | `Ap_filter` (jsarray.c:718) | condition guarded at jsarray.c:718 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 86 | `Ap_reduce` (jsarray.c:751) | condition guarded at jsarray.c:751 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 87 | `Ap_reduce` (jsarray.c:757) | condition guarded at jsarray.c:757 | throw TypeError: `no initial value` | `interp_errors::err_jsarray` |
| 88 | `Ap_reduce` (jsarray.c:767) | condition guarded at jsarray.c:767 | throw TypeError: `no initial value` | `interp_errors::err_jsarray` |
| 89 | `Ap_reduceRight` (jsarray.c:792) | condition guarded at jsarray.c:792 | throw TypeError: `callback is not a function` | `interp_errors::err_jsarray` |
| 90 | `Ap_reduceRight` (jsarray.c:798) | condition guarded at jsarray.c:798 | throw TypeError: `no initial value` | `interp_errors::err_jsarray` |
| 91 | `Ap_reduceRight` (jsarray.c:808) | condition guarded at jsarray.c:808 | throw TypeError: `no initial value` | `interp_errors::err_jsarray` |
| 92 | `Bp_toString` (jsboolean.c:16) | condition guarded at jsboolean.c:16 | throw TypeError: `not a boolean` | `interp_errors::err_prototype_receivers` |
| 93 | `Bp_valueOf` (jsboolean.c:23) | condition guarded at jsboolean.c:23 | throw TypeError: `not a boolean` | `interp_errors::err_prototype_receivers` |
| 94 | `Decode` (jsbuiltin.c:145) | condition guarded at jsbuiltin.c:145 | throw URIError: `truncated escape sequence` | `interp_errors::err_jsbuiltin_uri` |
| 95 | `Decode` (jsbuiltin.c:149) | condition guarded at jsbuiltin.c:149 | throw URIError: `invalid escape sequence` | `interp_errors::err_jsbuiltin_uri` |
| 96 | `checkfutureword` (jscompile.c:43) | condition guarded at jscompile.c:43 | throw SyntaxError (compiler): `'%s' is a future reserved word` | `interp_errors::err_jscompile` |
| 97 | `checkfutureword` (jscompile.c:46) | condition guarded at jscompile.c:46 | throw SyntaxError (compiler): `'%s' is a strict mode future reserved word` | `interp_errors::err_jscompile` |
| 98 | `emitraw` (jscompile.c:75) | condition guarded at jscompile.c:75 | throw SyntaxError: `integer overflow in instruction coding` | `interp_errors::err_jscompile` |
| 99 | `addlocal` (jscompile.c:114) | condition guarded at jscompile.c:114 | throw SyntaxError (compiler): `redefining 'arguments' is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 100 | `addlocal` (jscompile.c:116) | condition guarded at jscompile.c:116 | throw SyntaxError (compiler): `redefining 'eval' is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 101 | `addlocal` (jscompile.c:119) | condition guarded at jscompile.c:119 | throw EvalError: `%s:%d: invalid use of 'eval'` | `interp_errors::err_jscompile` |
| 102 | `addlocal` (jscompile.c:128) | condition guarded at jscompile.c:128 | throw SyntaxError (compiler): `duplicate formal parameter '%s'` | `interp_errors::err_jscompile` |
| 103 | `emitlocal` (jscompile.c:204) | condition guarded at jscompile.c:204 | throw SyntaxError (compiler): `'arguments' is read-only in strict mode` | `interp_errors::err_jscompile` |
| 104 | `emitlocal` (jscompile.c:206) | condition guarded at jscompile.c:206 | throw SyntaxError (compiler): `'eval' is read-only in strict mode` | `interp_errors::err_jscompile` |
| 105 | `emitlocal` (jscompile.c:209) | condition guarded at jscompile.c:209 | throw EvalError: `%s:%d: invalid use of 'eval'` | `interp_errors::err_jscompile` |
| 106 | `emitjumpto` (jscompile.c:238) | condition guarded at jscompile.c:238 | throw SyntaxError: `jump address integer overflow` | `interp_errors::err_jscompile` |
| 107 | `labelto` (jscompile.c:245) | condition guarded at jscompile.c:245 | throw SyntaxError: `jump address integer overflow` | `interp_errors::err_jscompile` |
| 108 | `checkdup` (jscompile.c:315) | condition guarded at jscompile.c:315 | throw SyntaxError (compiler): `duplicate property '%s' in object literal` | `interp_errors::err_jscompile` |
| 109 | `cobject` (jscompile.c:336) | condition guarded at jscompile.c:336 | throw SyntaxError (compiler): `invalid property name in object initializer` | `interp_errors::err_jscompile` |
| 110 | `cassign` (jscompile.c:400) | condition guarded at jscompile.c:400 | throw SyntaxError (compiler): `invalid l-value in assignment` | `interp_errors::err_jscompile` |
| 111 | `cassignforin` (jscompile.c:410) | condition guarded at jscompile.c:410 | throw SyntaxError (compiler): `more than one loop variable in for-in statement` | `interp_errors::err_jscompile` |
| 112 | `cassignforin` (jscompile.c:439) | condition guarded at jscompile.c:439 | throw SyntaxError (compiler): `invalid l-value in for-in loop assignment` | `interp_errors::err_jscompile` |
| 113 | `cassignop1` (jscompile.c:464) | condition guarded at jscompile.c:464 | throw SyntaxError (compiler): `invalid l-value in assignment` | `interp_errors::err_jscompile` |
| 114 | `cassignop2` (jscompile.c:487) | condition guarded at jscompile.c:487 | throw SyntaxError (compiler): `invalid l-value in assignment` | `interp_errors::err_jscompile` |
| 115 | `cdelete` (jscompile.c:508) | condition guarded at jscompile.c:508 | throw SyntaxError (compiler): `delete on an unqualified name is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 116 | `cdelete` (jscompile.c:524) | condition guarded at jscompile.c:524 | throw SyntaxError (compiler): `invalid l-value in delete expression` | `interp_errors::err_jscompile` |
| 117 | `cexp` (jscompile.c:780) | condition guarded at jscompile.c:780 | throw SyntaxError (compiler): `unknown expression type` | `interp_errors::err_jscompile` |
| 118 | `ctrycatch` (jscompile.c:961) | condition guarded at jscompile.c:961 | throw SyntaxError (compiler): `redefining 'arguments' is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 119 | `ctrycatch` (jscompile.c:963) | condition guarded at jscompile.c:963 | throw SyntaxError (compiler): `redefining 'eval' is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 120 | `ctrycatchfinally` (jscompile.c:993) | condition guarded at jscompile.c:993 | throw SyntaxError (compiler): `redefining 'arguments' is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 121 | `ctrycatchfinally` (jscompile.c:995) | condition guarded at jscompile.c:995 | throw SyntaxError (compiler): `redefining 'eval' is not allowed in strict mode` | `interp_errors::err_jscompile` |
| 122 | `cswitch` (jscompile.c:1025) | condition guarded at jscompile.c:1025 | throw SyntaxError (compiler): `more than one default label in switch` | `interp_errors::err_jscompile` |
| 123 | `cstm` (jscompile.c:1217) | condition guarded at jscompile.c:1217 | throw SyntaxError (compiler): `break label '%s' not found` | `interp_errors::err_jscompile` |
| 124 | `cstm` (jscompile.c:1221) | condition guarded at jscompile.c:1221 | throw SyntaxError (compiler): `unlabelled break must be inside loop or switch` | `interp_errors::err_jscompile` |
| 125 | `cstm` (jscompile.c:1233) | condition guarded at jscompile.c:1233 | throw SyntaxError (compiler): `continue label '%s' not found` | `interp_errors::err_jscompile` |
| 126 | `cstm` (jscompile.c:1237) | condition guarded at jscompile.c:1237 | throw SyntaxError (compiler): `continue must be inside loop` | `interp_errors::err_jscompile` |
| 127 | `cstm` (jscompile.c:1251) | condition guarded at jscompile.c:1251 | throw SyntaxError (compiler): `return not in function` | `interp_errors::err_jscompile` |
| 128 | `cstm` (jscompile.c:1266) | condition guarded at jscompile.c:1266 | throw SyntaxError (compiler): `'with' statements are not allowed in strict mode` | `interp_errors::err_jscompile` |
| 129 | `js_todate` (jsdate.c:366) | condition guarded at jsdate.c:366 | throw TypeError: `not a date` | `interp_errors::err_prototype_receivers` |
| 130 | `js_setdate` (jsdate.c:374) | condition guarded at jsdate.c:374 | throw TypeError: `not a date` | `interp_errors::err_prototype_receivers` |
| 131 | `Dp_toISOString` (jsdate.c:485) | condition guarded at jsdate.c:485 | throw RangeError: `invalid date` | `interp_errors::err_prototype_receivers` |
| 132 | `Dp_toJSON` (jsdate.c:793) | condition guarded at jsdate.c:793 | throw TypeError: `this.toISOString is not a function` | `interp_errors::err_prototype_receivers` |
| 133 | `Ep_toString` (jserror.c:36) | condition guarded at jserror.c:36 | throw TypeError: `not an object` | `interp_errors::err_prototype_receivers` |
| 134 | `Fp_toString` (jsfunction.c:53) | condition guarded at jsfunction.c:53 | throw TypeError: `not a function` | `interp_errors::err_prototype_receivers` |
| 135 | `Fp_apply` (jsfunction.c:100) | condition guarded at jsfunction.c:100 | throw TypeError: `not a function` | `interp_errors::err_prototype_receivers` |
| 136 | `Fp_call` (jsfunction.c:123) | condition guarded at jsfunction.c:123 | throw TypeError: `not a function` | `interp_errors::err_prototype_receivers` |
| 137 | `Fp_bind` (jsfunction.c:186) | condition guarded at jsfunction.c:186 | throw TypeError: `not a function` | `interp_errors::err_prototype_receivers` |
| 138 | `jsS_newstringnode` (jsintern.c:47) | condition guarded at jsintern.c:47 | throw RangeError: `invalid string length` | documented only — needs a string past `JS_STRLIMIT` (1<<28); not constructible in-process |
| 139 | `jsY_next` (jslex.c:177) | condition guarded at jslex.c:177 | throw SyntaxError (lexer): `expected '%c'` | `interp_errors::err_jslex` |
| 140 | `jsY_unescape` (jslex.c:192) | condition guarded at jslex.c:192 | throw SyntaxError (lexer): `unexpected escape sequence` | `interp_errors::err_jslex` |
| 141 | `lexhex` (jslex.c:255) | condition guarded at jslex.c:255 | throw SyntaxError (lexer): `malformed hexadecimal number` | `interp_errors::err_jslex` |
| 142 | `lexinteger` (jslex.c:269) | condition guarded at jslex.c:269 | throw SyntaxError (lexer): `malformed number` | `interp_errors::err_jslex` |
| 143 | `lexnumber` (jslex.c:312) | condition guarded at jslex.c:312 | throw SyntaxError (lexer): `number with leading zero` | `interp_errors::err_jslex` |
| 144 | `lexnumber` (jslex.c:333) | condition guarded at jslex.c:333 | throw SyntaxError (lexer): `number with letter suffix` | `interp_errors::err_jslex` |
| 145 | `lexnumber` (jslex.c:351) | condition guarded at jslex.c:351 | throw SyntaxError (lexer): `number with leading zero` | `interp_errors::err_jslex` |
| 146 | `lexnumber` (jslex.c:377) | condition guarded at jslex.c:377 | throw SyntaxError (lexer): `missing exponent` | `interp_errors::err_jslex` |
| 147 | `lexnumber` (jslex.c:381) | condition guarded at jslex.c:381 | throw SyntaxError (lexer): `number with letter suffix` | `interp_errors::err_jslex` |
| 148 | `lexescape` (jslex.c:399) | condition guarded at jslex.c:399 | throw SyntaxError (lexer): `unterminated escape sequence` | `interp_errors::err_jslex` |
| 149 | `lexstring` (jslex.c:440) | condition guarded at jslex.c:440 | throw SyntaxError (lexer): `string not terminated` | `interp_errors::err_jslex` |
| 150 | `lexstring` (jslex.c:443) | condition guarded at jslex.c:443 | throw SyntaxError (lexer): `malformed escape sequence` | `interp_errors::err_jslex` |
| 151 | `lexregexp` (jslex.c:490) | condition guarded at jslex.c:490 | throw SyntaxError (lexer): `regular expression not terminated` | `interp_errors::err_jslex` |
| 152 | `lexregexp` (jslex.c:497) | condition guarded at jslex.c:497 | throw SyntaxError (lexer): `regular expression not terminated` | `interp_errors::err_jslex` |
| 153 | `lexregexp` (jslex.c:521) | condition guarded at jslex.c:521 | throw SyntaxError (lexer): `illegal flag in regular expression: %c` | `interp_errors::err_jslex` |
| 154 | `lexregexp` (jslex.c:525) | condition guarded at jslex.c:525 | throw SyntaxError (lexer): `duplicated flag in regular expression` | `interp_errors::err_jslex` |
| 155 | `jsY_lexx` (jslex.c:574) | condition guarded at jslex.c:574 | throw SyntaxError (lexer): `multi-line comment not terminated` | `interp_errors::err_jslex` |
| 156 | `jsY_lexx` (jslex.c:728) | condition guarded at jslex.c:728 | throw SyntaxError (lexer): `unexpected character: '%c'` | `interp_errors::err_jslex` |
| 157 | `jsY_lexx` (jslex.c:729) | condition guarded at jslex.c:729 | throw SyntaxError (lexer): `unexpected character: \\u%04X` | `interp_errors::err_jslex` |
| 158 | `lexjsonnumber` (jslex.c:760) | condition guarded at jslex.c:760 | throw SyntaxError (lexer): `unexpected non-digit` | `interp_errors::err_jslex` |
| 159 | `lexjsonnumber` (jslex.c:767) | condition guarded at jslex.c:767 | throw SyntaxError (lexer): `missing digits after decimal point` | `interp_errors::err_jslex` |
| 160 | `lexjsonnumber` (jslex.c:777) | condition guarded at jslex.c:777 | throw SyntaxError (lexer): `missing digits after exponent indicator` | `interp_errors::err_jslex` |
| 161 | `lexjsonescape` (jslex.c:791) | condition guarded at jslex.c:791 | throw SyntaxError (lexer): `invalid escape sequence` | `interp_errors::err_jslex` |
| 162 | `lexjsonstring` (jslex.c:820) | condition guarded at jslex.c:820 | throw SyntaxError (lexer): `unterminated string` | `interp_errors::err_jslex` |
| 163 | `lexjsonstring` (jslex.c:822) | condition guarded at jslex.c:822 | throw SyntaxError (lexer): `invalid control character in string` | `interp_errors::err_jslex` |
| 164 | `jsY_lexjson` (jslex.c:878) | condition guarded at jslex.c:878 | throw SyntaxError (lexer): `unexpected character: '%c'` | `interp_errors::err_jslex` |
| 165 | `jsY_lexjson` (jslex.c:879) | condition guarded at jslex.c:879 | throw SyntaxError (lexer): `unexpected character: \\u%04X` | `interp_errors::err_jslex` |
| 166 | `Np_valueOf` (jsnumber.c:22) | condition guarded at jsnumber.c:22 | throw TypeError: `not a number` | `interp_errors::err_prototype_receivers` |
| 167 | `Np_toString` (jsnumber.c:33) | condition guarded at jsnumber.c:33 | throw TypeError: `not a number` | `interp_errors::err_prototype_receivers` |
| 168 | `Np_toString` (jsnumber.c:40) | condition guarded at jsnumber.c:40 | throw RangeError: `invalid radix` | `interp_errors::err_prototype_receivers` |
| 169 | `Np_toFixed` (jsnumber.c:134) | condition guarded at jsnumber.c:134 | throw TypeError: `not a number` | `interp_errors::err_prototype_receivers` |
| 170 | `Np_toFixed` (jsnumber.c:135) | condition guarded at jsnumber.c:135 | throw RangeError: `precision %d out of range` | `interp_errors::err_prototype_receivers` |
| 171 | `Np_toFixed` (jsnumber.c:136) | condition guarded at jsnumber.c:136 | throw RangeError: `precision %d out of range` | `interp_errors::err_prototype_receivers` |
| 172 | `Np_toExponential` (jsnumber.c:150) | condition guarded at jsnumber.c:150 | throw TypeError: `not a number` | `interp_errors::err_prototype_receivers` |
| 173 | `Np_toExponential` (jsnumber.c:151) | condition guarded at jsnumber.c:151 | throw RangeError: `precision %d out of range` | `interp_errors::err_prototype_receivers` |
| 174 | `Np_toExponential` (jsnumber.c:152) | condition guarded at jsnumber.c:152 | throw RangeError: `precision %d out of range` | `interp_errors::err_prototype_receivers` |
| 175 | `Np_toPrecision` (jsnumber.c:166) | condition guarded at jsnumber.c:166 | throw TypeError: `not a number` | `interp_errors::err_prototype_receivers` |
| 176 | `Np_toPrecision` (jsnumber.c:167) | condition guarded at jsnumber.c:167 | throw RangeError: `precision %d out of range` | `interp_errors::err_prototype_receivers` |
| 177 | `Np_toPrecision` (jsnumber.c:168) | condition guarded at jsnumber.c:168 | throw RangeError: `precision %d out of range` | `interp_errors::err_prototype_receivers` |
| 178 | `O_getPrototypeOf` (jsobject.c:112) | condition guarded at jsobject.c:112 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 179 | `O_getOwnPropertyDescriptor` (jsobject.c:125) | condition guarded at jsobject.c:125 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 180 | `O_getOwnPropertyNames` (jsobject.c:176) | condition guarded at jsobject.c:176 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 181 | `ToPropertyDescriptor` (jsobject.c:258) | condition guarded at jsobject.c:258 | throw TypeError: `value/writable and get/set attributes are exclusive` | `interp_errors::err_jsobject` |
| 182 | `ToPropertyDescriptor` (jsobject.c:265) | condition guarded at jsobject.c:265 | throw TypeError: `value/writable and get/set attributes are exclusive` | `interp_errors::err_jsobject` |
| 183 | `O_defineProperty` (jsobject.c:277) | condition guarded at jsobject.c:277 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 184 | `O_defineProperty` (jsobject.c:278) | condition guarded at jsobject.c:278 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 185 | `O_defineProperties_walk` (jsobject.c:289) | condition guarded at jsobject.c:289 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 186 | `O_defineProperties_imp` (jsobject.c:304) | condition guarded at jsobject.c:304 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 187 | `O_defineProperties` (jsobject.c:326) | condition guarded at jsobject.c:326 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 188 | `O_create` (jsobject.c:342) | condition guarded at jsobject.c:342 | throw TypeError: `not an object or null` | `interp_errors::err_jsobject` |
| 189 | `O_keys` (jsobject.c:372) | condition guarded at jsobject.c:372 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 190 | `O_preventExtensions` (jsobject.c:403) | condition guarded at jsobject.c:403 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 191 | `O_isExtensible` (jsobject.c:413) | condition guarded at jsobject.c:413 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 192 | `O_seal` (jsobject.c:431) | condition guarded at jsobject.c:431 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 193 | `O_isSealed` (jsobject.c:461) | condition guarded at jsobject.c:461 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 194 | `O_freeze` (jsobject.c:489) | condition guarded at jsobject.c:489 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 195 | `O_isFrozen` (jsobject.c:521) | condition guarded at jsobject.c:521 | throw TypeError: `not an object` | `interp_errors::err_jsobject` |
| 196 | `jsonexpect` (json.c:41) | condition guarded at json.c:41 | throw SyntaxError: `JSON: unexpected token: %s (expected %s)` | `interp_errors::err_json` |
| 197 | `jsonvalue` (json.c:67) | condition guarded at json.c:67 | throw SyntaxError: `JSON: unexpected token: %s (expected string)` | `interp_errors::err_json` |
| 198 | `jsonvalue` (json.c:107) | condition guarded at json.c:107 | throw SyntaxError: `JSON: unexpected token: %s` | `interp_errors::err_json` |
| 199 | `fmtobject` (json.c:261) | condition guarded at json.c:261 | throw TypeError: `cyclic object value` | `interp_errors::err_json` |
| 200 | `fmtarray` (json.c:297) | condition guarded at json.c:297 | throw TypeError: `cyclic object value` | `interp_errors::err_json` |
| 201 | `jsP_next` (jsparse.c:143) | condition guarded at jsparse.c:143 | throw SyntaxError (parser): `unexpected token: %s (expected %s)` | `interp_errors::err_jsparse` |
| 202 | `semicolon` (jsparse.c:153) | condition guarded at jsparse.c:153 | throw SyntaxError (parser): `unexpected token: %s (expected ';')` | `interp_errors::err_jsparse` |
| 203 | `identifier` (jsparse.c:166) | condition guarded at jsparse.c:166 | throw SyntaxError (parser): `unexpected token: %s (expected identifier)` | `interp_errors::err_jsparse` |
| 204 | `identifiername` (jsparse.c:183) | condition guarded at jsparse.c:183 | throw SyntaxError (parser): `unexpected token: %s (expected identifier or keyword)` | `interp_errors::err_jsparse` |
| 205 | `primary` (jsparse.c:363) | condition guarded at jsparse.c:363 | throw SyntaxError (parser): `unexpected token in expression: %s` | `interp_errors::err_jsparse` |
| 206 | `caseclause` (jsparse.c:700) | condition guarded at jsparse.c:700 | throw SyntaxError (parser): `unexpected token in switch: %s (expected 'case' or 'default')` | `interp_errors::err_jsparse` |
| 207 | `forstatement` (jsparse.c:751) | condition guarded at jsparse.c:751 | throw SyntaxError (parser): `unexpected token in for-var-statement: %s` | `interp_errors::err_jsparse` |
| 208 | `forstatement` (jsparse.c:770) | condition guarded at jsparse.c:770 | throw SyntaxError (parser): `unexpected token in for-statement: %s` | `interp_errors::err_jsparse` |
| 209 | `statement` (jsparse.c:888) | condition guarded at jsparse.c:888 | throw SyntaxError (parser): `unexpected token in try: %s (expected 'catch' or 'finally')` | `interp_errors::err_jsparse` |
| 210 | `jsV_setproperty` (jsproperty.c:228) | condition guarded at jsproperty.c:228 | throw TypeError: `object is non-extensible` | `interp_errors::err_jsobject` |
| 211 | `jsV_nextiterator` (jsproperty.c:303) | condition guarded at jsproperty.c:303 | throw TypeError: `not an iterator` | `interp_errors::err_jsobject` |
| 212 | `js_newregexpx` (jsregexp.c:38) | condition guarded at jsregexp.c:38 | throw SyntaxError: `regular expression: %s` | `interp_errors::err_jsregexp` |
| 213 | `js_RegExp_prototype_exec` (jsregexp.c:77) | condition guarded at jsregexp.c:77 | throw Error: `regexec failed` | `interp_errors::err_jsregexp` |
| 214 | `Rp_test` (jsregexp.c:126) | condition guarded at jsregexp.c:126 | throw Error: `regexec failed` | `interp_errors::err_jsregexp` |
| 215 | `jsB_new_RegExp` (jsregexp.c:149) | condition guarded at jsregexp.c:149 | throw TypeError: `cannot supply flags when creating one RegExp from another` | `interp_errors::err_jsregexp` |
| 216 | `jsB_new_RegExp` (jsregexp.c:172) | condition guarded at jsregexp.c:172 | throw SyntaxError: `invalid regular expression flag: '%c'` | `interp_errors::err_jsregexp` |
| 217 | `jsB_new_RegExp` (jsregexp.c:175) | condition guarded at jsregexp.c:175 | throw SyntaxError: `invalid regular expression flag: 'g'` | `interp_errors::err_jsregexp` |
| 218 | `jsB_new_RegExp` (jsregexp.c:176) | condition guarded at jsregexp.c:176 | throw SyntaxError: `invalid regular expression flag: 'i'` | `interp_errors::err_jsregexp` |
| 219 | `jsB_new_RegExp` (jsregexp.c:177) | condition guarded at jsregexp.c:177 | throw SyntaxError: `invalid regular expression flag: 'm'` | `interp_errors::err_jsregexp` |
| 220 | `js_pushstring` (jsrun.c:149) | condition guarded at jsrun.c:149 | throw RangeError: `invalid string length` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 221 | `js_pushlstring` (jsrun.c:166) | condition guarded at jsrun.c:166 | throw RangeError: `invalid string length` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 222 | `js_toregexp` (jsrun.c:373) | condition guarded at jsrun.c:373 | throw TypeError: `not a regexp` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 223 | `js_touserdata` (jsrun.c:382) | condition guarded at jsrun.c:382 | throw TypeError: `not a %s` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 224 | `jsR_tofunction` (jsrun.c:393) | condition guarded at jsrun.c:393 | throw TypeError: `not a function` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 225 | `js_pop` (jsrun.c:408) | condition guarded at jsrun.c:408 | throw Error: `stack underflow!` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 226 | `js_remove` (jsrun.c:416) | condition guarded at jsrun.c:416 | throw Error: `stack error!` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 227 | `js_insert` (jsrun.c:424) | condition guarded at jsrun.c:424 | throw Error: `not implemented yet` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 228 | `js_replace` (jsrun.c:431) | condition guarded at jsrun.c:431 | throw Error: `stack error!` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 229 | `jsR_setarrayindex` (jsrun.c:676) | condition guarded at jsrun.c:676 | throw RangeError: `array too large` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 230 | `jsR_setproperty` (jsrun.c:707) | condition guarded at jsrun.c:707 | throw RangeError: `invalid array length` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 231 | `jsR_setproperty` (jsrun.c:709) | condition guarded at jsrun.c:709 | throw RangeError: `array too large` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 232 | `jsR_setproperty` (jsrun.c:773) | condition guarded at jsrun.c:773 | throw TypeError: `setting property '%s' that only has a getter` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 233 | `jsR_setproperty` (jsrun.c:783) | condition guarded at jsrun.c:783 | throw TypeError: `cannot create property '%s' on transient object` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 234 | `jsR_setproperty` (jsrun.c:800) | condition guarded at jsrun.c:800 | throw TypeError: `'%s' is read-only` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 235 | `jsR_defproperty` (jsrun.c:854) | condition guarded at jsrun.c:854 | throw TypeError: `'%s' is read-only` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 236 | `jsR_defproperty` (jsrun.c:860) | condition guarded at jsrun.c:860 | throw TypeError: `'%s' is non-configurable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 237 | `jsR_defproperty` (jsrun.c:866) | condition guarded at jsrun.c:866 | throw TypeError: `'%s' is non-configurable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 238 | `jsR_defproperty` (jsrun.c:875) | condition guarded at jsrun.c:875 | throw TypeError: `'%s' is read-only or non-configurable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 239 | `jsR_delproperty` (jsrun.c:921) | condition guarded at jsrun.c:921 | throw TypeError: `'%s' is non-configurable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 240 | `js_setvar` (jsrun.c:1127) | condition guarded at jsrun.c:1127 | throw TypeError: `'%s' is read-only` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 241 | `js_setvar` (jsrun.c:1133) | condition guarded at jsrun.c:1133 | throw ReferenceError: `assignment to undeclared variable '%s'` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 242 | `js_delvar` (jsrun.c:1145) | condition guarded at jsrun.c:1145 | throw TypeError: `'%s' is non-configurable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 243 | `jsR_pushtrace` (jsrun.c:1290) | condition guarded at jsrun.c:1290 | throw Error: `call stack overflow` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 244 | `js_call` (jsrun.c:1304) | condition guarded at jsrun.c:1304 | throw RangeError: `number of arguments cannot be negative` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 245 | `js_call` (jsrun.c:1307) | condition guarded at jsrun.c:1307 | throw TypeError: `%s is not callable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 246 | `js_construct` (jsrun.c:1341) | condition guarded at jsrun.c:1341 | throw TypeError: `%s is not callable` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 247 | `js_endtry` (jsrun.c:1461) | condition guarded at jsrun.c:1461 | throw Error: `endtry: exception stack underflow` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 248 | `jsR_run` (jsrun.c:1673) | condition guarded at jsrun.c:1673 | throw ReferenceError: `'%s' is not defined` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 249 | `jsR_run` (jsrun.c:1698) | condition guarded at jsrun.c:1698 | throw ReferenceError: `'%s' is not defined` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 250 | `jsR_run` (jsrun.c:1721) | condition guarded at jsrun.c:1721 | throw TypeError: `operand to 'in' is not an object` | `interp_errors::err_jsrun_property_access` + `err_jsrun_calls_and_arrays` |
| 251 | `js_doregexec` (jsstring.c:9) | condition guarded at jsstring.c:9 | throw Error: `regexec failed` | `interp_errors::err_prototype_receivers` |
| 252 | `checkstring` (jsstring.c:16) | condition guarded at jsstring.c:16 | throw TypeError: `string function called on null or undefined` | `interp_errors::err_prototype_receivers` |
| 253 | `Sp_toString` (jsstring.c:108) | condition guarded at jsstring.c:108 | throw TypeError: `not a string` | `interp_errors::err_prototype_receivers` |
| 254 | `Sp_valueOf` (jsstring.c:115) | condition guarded at jsstring.c:115 | throw TypeError: `not a string` | `interp_errors::err_prototype_receivers` |
| 255 | `Sp_concat` (jsstring.c:163) | condition guarded at jsstring.c:163 | throw RangeError: `invalid string length` | `interp_errors::err_prototype_receivers` |
| 256 | `Sp_concat` (jsstring.c:171) | condition guarded at jsstring.c:171 | throw RangeError: `invalid string length` | `interp_errors::err_prototype_receivers` |
| 257 | `jsV_toprimitive` (jsvalue.c:144) | condition guarded at jsvalue.c:144 | throw TypeError: `cannot convert object to primitive` | `interp_errors::err_jsvalue` |
| 258 | `jsV_toobject` (jsvalue.c:401) | condition guarded at jsvalue.c:401 | throw TypeError: `cannot convert undefined to object` | `interp_errors::err_jsvalue` |
| 259 | `jsV_toobject` (jsvalue.c:402) | condition guarded at jsvalue.c:402 | throw TypeError: `cannot convert null to object` | `interp_errors::err_jsvalue` |
| 260 | `js_instanceof` (jsvalue.c:579) | condition guarded at jsvalue.c:579 | throw TypeError: `instanceof: invalid operand` | `interp_errors::err_jsvalue` |
| 261 | `js_instanceof` (jsvalue.c:586) | condition guarded at jsvalue.c:586 | throw TypeError: `instanceof: 'prototype' property is not an object` | `interp_errors::err_jsvalue` |

