# Error surface

This table is derived from all `js_*error` calls, regexp `die` calls, explicit
error sentinels, public index/length checks, allocation checks, and assertions
in `../c_src/src`. Rows marked `[x]` are covered by `tests/differential.rs`;
the test compares the exact return code/sentinel and, for protected JavaScript
errors, the exact error name and message rendered by C and Rust.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|---|---|---|
| 1 | `js_newstate` | allocator rejects state allocation | `[x]` NULL |
| 2 | `js_newstate` | allocator accepts state but rejects value-stack allocation | `[x]` NULL and state freed |
| 3 | `js_newstate` | initialization allocation throws after stack allocation | `[x]` NULL and state freed |
| 4 | `js_malloc` | `memlimit > 0 && size >= memlimit` | `[x]` throws string `out of memory` |
| 5 | `js_malloc` | allocator returns NULL | `[x]` throws string `out of memory` |
| 6 | `js_realloc` | `memlimit > 0 && size >= memlimit` | `[x]` throws string `out of memory` |
| 7 | `js_realloc` | allocator returns NULL | `[x]` throws string `out of memory` |
| 8 | VM dispatch | positive run limit reaches 1 | `[x]` throws string `script ran too long` |
| 9 | value stack pushes | `top + required >= JS_STACKSIZE` (4096) | `[x]` throws string `stack overflow` |
| 10 | call/trace stack | `tracetop + 1 == JS_ENVLIMIT` (1024) | `[x]` throws `Error: call stack overflow` |
| 11 | exception stack | `trytop == JS_TRYLIMIT` (64) | `[x]` protected API returns 1/fallback |
| 12 | environment stack | `envtop + 1 >= JS_ENVLIMIT` (1024) | `[x]` throws stack overflow |
| 13 | parser | nesting exceeds `JS_ASTLIMIT` (400) | `[x]` `js_ploadstring` returns 1 with syntax error |
| 14 | `js_pushstring` | `strlen(v) > JS_STRLIMIT` (`1<<28`) | `[x]` throws `RangeError: invalid string length` |
| 15 | `js_pushlstring` | `n > JS_STRLIMIT` | `[x]` throws `RangeError: invalid string length` |
| 16 | string buffer (`js_putc`) | growth would exceed `JS_STRLIMIT` | `[x]` throws `RangeError: invalid string length` |
| 17 | array/string operations | result length would exceed `JS_STRLIMIT` | `[x]` throws `RangeError: invalid string length` |
| 18 | flat array append | new index reaches `JS_ARRAYLIMIT` (`1<<26`) | `[x]` throws `RangeError: array too large` |
| 19 | array `length` setter | number is non-integral, negative, or not exactly uint32 | `[x]` throws `RangeError: invalid array length` |
| 20 | array `length` setter | valid new length is `>= JS_ARRAYLIMIT` | `[x]` throws `RangeError: array too large` |
| 21 | `Array.prototype.sort` | length > 1 and comparator is neither callable nor undefined | `[x]` throws `TypeError: comparison function must be a function or undefined` |
| 22 | `Array.prototype.sort` | length is at least `INT_MAX` | `[x]` throws `RangeError: array is too large to sort` |
| 23 | `Array.prototype.toString` | receiver is not coercible | `[x]` throws `TypeError: 'this' is not an object` |
| 24 | array iteration methods | callback is not callable (`every/some/forEach/map/filter`) | `[x]` throws `TypeError: callback is not a function` |
| 25 | `Array.prototype.reduce` | callback is not callable | `[x]` throws `TypeError: callback is not a function` |
| 26 | `Array.prototype.reduce` | empty/sparse array and no initial value | `[x]` throws `TypeError: no initial value` |
| 27 | `Array.prototype.reduceRight` | callback is not callable | `[x]` throws `TypeError: callback is not a function` |
| 28 | `Array.prototype.reduceRight` | empty/sparse array and no initial value | `[x]` throws `TypeError: no initial value` |
| 29 | Boolean prototype methods | receiver class is not `JS_CBOOLEAN` | `[x]` throws `TypeError: not a boolean` |
| 30 | Number prototype methods | receiver class is not `JS_CNUMBER` | `[x]` throws `TypeError: not a number` |
| 31 | `Number.prototype.toString` | radix less than 2 or greater than 36 | `[x]` throws `RangeError: invalid radix` |
| 32 | `Number.prototype.toFixed` | precision less than 0 | `[x]` throws precision `RangeError` |
| 33 | `Number.prototype.toFixed` | precision greater than 20 | `[x]` throws precision `RangeError` |
| 34 | `Number.prototype.toExponential` | precision less than 0 | `[x]` throws precision `RangeError` |
| 35 | `Number.prototype.toExponential` | precision greater than 20 | `[x]` throws precision `RangeError` |
| 36 | `Number.prototype.toPrecision` | precision less than 1 | `[x]` throws precision `RangeError` |
| 37 | `Number.prototype.toPrecision` | precision greater than 21 | `[x]` throws precision `RangeError` |
| 38 | String prototype value methods | receiver class is not `JS_CSTRING` | `[x]` throws `TypeError: not a string` |
| 39 | generic String methods | receiver is null or undefined | `[x]` throws `TypeError: string function called on null or undefined` |
| 40 | Date prototype methods | receiver class is not `JS_CDATE` | `[x]` throws `TypeError: not a date` |
| 41 | `Date.prototype.toISOString` | date value is not finite | `[x]` throws `RangeError: invalid date` |
| 42 | `Date.prototype.toJSON` | `toISOString` property is not callable | `[x]` throws `TypeError: this.toISOString is not a function` |
| 43 | URI decoder | `%` has fewer than two following bytes | `[x]` throws `URIError: truncated escape sequence` |
| 44 | URI decoder | bytes after `%` are not hexadecimal | `[x]` throws `URIError: invalid escape sequence` |
| 45 | JSON parser | required punctuation/token absent | `[x]` throws `SyntaxError: JSON: unexpected token...` |
| 46 | JSON parser | object key is not a string | `[x]` throws `SyntaxError: JSON: unexpected token... (expected string)` |
| 47 | JSON parser | token is not a JSON value | `[x]` throws `SyntaxError: JSON: unexpected token...` |
| 48 | `JSON.stringify` object walk | object appears in ancestor stack | `[x]` throws `TypeError: cyclic object value` |
| 49 | `JSON.stringify` array walk | array appears in ancestor stack | `[x]` throws `TypeError: cyclic object value` |
| 50 | RegExp constructor | source is RegExp and flags argument is supplied | `[x]` throws flags `TypeError` |
| 51 | RegExp constructor | flag is not `g`, `i`, or `m` | `[x]` throws invalid-flag `SyntaxError` |
| 52 | RegExp constructor | `g` occurs more than once | `[x]` throws invalid `g` `SyntaxError` |
| 53 | RegExp constructor | `i` occurs more than once | `[x]` throws invalid `i` `SyntaxError` |
| 54 | RegExp constructor | `m` occurs more than once | `[x]` throws invalid `m` `SyntaxError` |
| 55 | `js_toregexp` | value is not RegExp object | `[x]` throws `TypeError: not a regexp` |
| 56 | `js_touserdata` | value is not matching userdata tag | `[x]` throws `TypeError: not a <tag>` |
| 57 | function prototype methods | receiver/target is not callable | `[x]` throws `TypeError: not a function` |
| 58 | `js_call` | argument count is negative | `[x]` throws `RangeError: number of arguments cannot be negative` |
| 59 | `js_call` | target at `-n-2` is not callable | `[x]` throws `<type> is not callable` TypeError |
| 60 | `js_construct` | target at `-n-1` is not callable | `[x]` throws `<type> is not callable` TypeError |
| 61 | `js_eval` | top value is not a string | `[x]` throws `TypeError: not a string` |
| 62 | `js_instanceof` | right operand is not callable | `[x]` throws `TypeError: instanceof: invalid operand` |
| 63 | `js_instanceof` | constructor `prototype` is not an object | `[x]` throws prototype `TypeError` |
| 64 | object-to-primitive | strict mode and neither `valueOf` nor `toString` yields primitive | `[x]` throws conversion `TypeError` |
| 65 | object conversion | value is undefined | `[x]` throws `TypeError: cannot convert undefined to object` |
| 66 | object conversion | value is null | `[x]` throws `TypeError: cannot convert null to object` |
| 67 | `Object.*` APIs | required operand is not object | `[x]` throws `TypeError: not an object` |
| 68 | `Object.create` | prototype is neither object nor null | `[x]` throws `TypeError: not an object or null` |
| 69 | property descriptor | data and accessor fields are both present | `[x]` throws exclusivity `TypeError` |
| 70 | strict property creation | object is non-extensible | `[x]` throws `TypeError: object is non-extensible` |
| 71 | strict property set | property only has getter | `[x]` throws getter-only `TypeError` |
| 72 | strict property set | creating property on transient primitive | `[x]` throws transient-object `TypeError` |
| 73 | strict property set | property has `JS_READONLY` | `[x]` throws read-only `TypeError` |
| 74 | strict property definition | incompatible change to read-only property | `[x]` throws read-only `TypeError` |
| 75 | strict property definition | change/delete of `JS_DONTCONF` property | `[x]` throws non-configurable `TypeError` |
| 76 | strict assignment | name does not resolve | `[x]` throws undeclared-variable `ReferenceError` |
| 77 | lookup/typeof-excluded lookup | identifier does not resolve | `[x]` throws not-defined `ReferenceError` |
| 78 | `in` operator | right operand is not object | `[x]` throws operand `TypeError` |
| 79 | `js_pop` | `n` exceeds current stack depth | `[x]` throws `Error: stack underflow!` |
| 80 | `js_remove` | index resolves outside `[bot, top)` | `[x]` throws `Error: stack error!` |
| 81 | `js_replace` | index resolves outside `[bot, top)` | `[x]` throws `Error: stack error!` |
| 82 | `js_insert` | every invocation | `[x]` throws `Error: not implemented yet` |
| 83 | `js_endtry` | exception stack is empty | `[x]` throws `Error: endtry: exception stack underflow` |
| 84 | compiler strict checks | reserved word, redefining/assigning `eval` or `arguments`, delete name, or `with` | `[x]` syntax/eval error |
| 85 | compiler bytecode | instruction operand or jump exceeds unsigned-short encoding | `[x]` throws overflow `SyntaxError` |
| 86 | parser/lexer | malformed token, escape, regexp, or grammar production | `[x]` `js_ploadstring` returns 1 and pushes exact error |
| 87 | `js_regcomp` | invalid hex escape | `[x]` NULL, error `invalid escape sequence` |
| 88 | `js_regcomp` | malformed `{m,n}` digits/order | `[x]` NULL, error `invalid quantifier` |
| 89 | `js_regcomp` | escape ends at NUL | `[x]` NULL, error `unterminated escape sequence` |
| 90 | `js_regcomp` | invalid identity escape of Unicode letter/underscore | `[x]` NULL, error `invalid escape character` |
| 91 | `js_regcomp` | repeat count reaches `REPINF` (255) | `[x]` NULL, error `numeric overflow` |
| 92 | `js_regcomp` | character classes exceed `REG_MAXCLASS` (128) | `[x]` NULL, error `too many character classes` |
| 93 | `js_regcomp` | character class range start is greater than end | `[x]` NULL, error `invalid character class range` |
| 94 | `js_regcomp` | class spans exceed `REG_MAXSPAN` (64) | `[x]` NULL, error `too many character class ranges` |
| 95 | `js_regcomp` | closing `]` is absent | `[x]` NULL, error `unterminated character class` |
| 96 | `js_regcomp` | quantified expression can loop matching empty string | `[x]` NULL, error `infinite loop matching the empty string` |
| 97 | `js_regcomp` | back-reference is zero, forward, or uncaptured | `[x]` NULL, error `invalid back-reference` |
| 98 | `js_regcomp` | captures reach `REG_MAXSUB` (16) | `[x]` NULL, error `too many captures` |
| 99 | `js_regcomp` | opening parenthesis has no close | `[x]` NULL, error `unmatched '('` |
| 100 | `js_regcomp` | top-level unmatched close parenthesis | `[x]` NULL, error `unmatched ')'` |
| 101 | `js_regcomp` | parse leaves an unexpected token | `[x]` NULL, error `syntax error` |
| 102 | `js_regcomp` | parse recursion exceeds `REG_MAXREC` (4096) | `[x]` NULL, error `stack overflow` |
| 103 | `js_regcomp` | pattern/program exceeds `REG_MAXPROG` (32768) | `[x]` NULL, error `program too large` |
| 104 | `js_regcompx` | allocator rejects program object | `[x]` NULL, error `cannot allocate regular expression` |
| 105 | `js_regcompx` | allocator rejects parse list | `[x]` NULL, error `cannot allocate regular expression parse list` |
| 106 | `js_regcompx` | allocator rejects instruction list | `[x]` NULL, error `cannot allocate regular expression instruction list` |
| 107 | `js_regcompx` | allocator rejects class list | `[x]` NULL, error `cannot allocate regular expression character class list` |
| 108 | `js_regexec` | recursive match exceeds `REG_MAXREC` | `[x]` returns -1 |
| 109 | `js_regexec` | no match, including empty input and `REG_NOTBOL` anchor rejection | `[x]` returns 1 |
| 110 | date parser | month index outside `[0,11]` in date construction | `[x]` NaN |
| 111 | ISO date parser | year/month/day/hour/minute/second/ms field missing or non-digit | `[x]` NaN |
| 112 | ISO date parser | time is missing required `:` | `[x]` NaN |
| 113 | ISO date parser | timezone hour > 23 or minute > 59 | `[x]` NaN |
| 114 | ISO date parser | trailing input remains | `[x]` NaN |
| 115 | ISO date parser | month outside `[1,12]` or day outside `[1,31]` | `[x]` NaN |
| 116 | ISO date parser | hour > 24, minute/second > 59, or ms > 999 | `[x]` NaN |
| 117 | ISO date parser | hour is 24 and any smaller unit is nonzero | `[x]` NaN |
| 118 | numeric parser | complete string has trailing non-space junk | `[x]` NaN |
| 119 | `js_try*` | conversion throws (null/object/tag/index conditions included) | `[x]` exact supplied fallback |
| 120 | stack index readers | negative/positive index resolves outside stack | `[x]` acts as undefined sentinel |
| 121 | `js_isarrayindex` | empty, leading zero, nondigit, or decimal overflow | `[x]` returns 0 |
| 122 | `js_currentfunctiondata` | no active function (`bot == 0`) | `[x]` NULL |
| 123 | `jsV_get*property` | property absent in own/prototype tree | `[x]` NULL |
| 124 | `jsV_nextiterator`/`js_nextiterator` | iterator exhausted | `[x]` NULL |
| 125 | UTF case full mapping | rune has no multi-rune mapping | `[x]` NULL |
| 126 | `js_freestate` | state pointer is NULL | `[x]` no-op |
| 127 | internal layout assertions | `sizeof(js_Value) != 16` or type byte offset != 15 | `[x]` process assertion; build ABI invariant |
| 128 | dtoa assertions | internal diy-fp exponents differ or subtraction underflows | `[x]` process assertion; algorithm invariant |
| 129 | array flatten assertions | object not simple array, negative index, or append length mismatch | `[x]` process assertion; caller invariant |
