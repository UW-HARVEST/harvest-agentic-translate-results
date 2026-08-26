# Error Surface

The rows below are derived from explicit throw calls, sentinel returns,
assertions, and range/limit checks in `c_src/src`. Repeated messages remain
separate where the C source has separate rejection branches. `protected error`
means a nonzero protected-call result with the thrown value left on the stack.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---:|----------|---------------------------------------------|-------------------|:---:|
| 1 | `js_newstate` | allocator cannot allocate `js_State` | `NULL` | [x] |
| 2 | `js_newstate` | allocator cannot allocate `JS_STACKSIZE` values | frees state, returns `NULL` | [x] |
| 3 | `js_newstate` | initialization throws after stack allocation | frees state, returns `NULL` | [x] |
| 4 | `js_defaultalloc` | requested size is zero | frees pointer, returns `NULL` | [x] |
| 5 | `js_newstate` | `sizeof(js_Value) != 16` | assertion failure | [x] |
| 6 | `js_newstate` | `offsetof(js_Value,t.type) != 15` | assertion failure | [x] |
| 7 | `js_pushstring` | `strlen(v) > JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 8 | `js_pushlstring` | `n > JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 9 | `jsS_newstringnode` | interned string length exceeds `JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 10 | `jsV_newmemstring` | requested string length exceeds `JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 11 | `Ap_join` | accumulated output length exceeds `JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 12 | `Sp_concat` | initial string length exceeds `JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 13 | `Sp_concat` | adding a later argument exceeds `JS_STRLIMIT` | `RangeError: invalid string length` | [x] |
| 14 | `jsR_setarrayindex` | `newlen > JS_ARRAYLIMIT` | `RangeError: array too large` | [x] |
| 15 | `jsR_setarrayindex` | target is not a simple array | assertion failure | [x] |
| 16 | `jsR_setarrayindex` | array index `k < 0` | assertion failure | [x] |
| 17 | `jsR_setarrayindex` | growth is not exactly one beyond flat length | assertion failure | [x] |
| 18 | `jsR_setproperty` (`length`) | numeric length is fractional, non-finite, or negative | `RangeError: invalid array length` | [x] |
| 19 | `jsR_setproperty` (`length`) | numeric length exceeds `JS_ARRAYLIMIT` | `RangeError: array too large` | [x] |
| 20 | `Ap_sort` | comparator is neither callable nor undefined | `TypeError: comparison function must be a function or undefined` | [x] |
| 21 | `Ap_sort` | length is at least `INT_MAX` | `RangeError: array is too large to sort` | [x] |
| 22 | `Ap_toString` | receiver is null or undefined | `TypeError: 'this' is not an object` | [x] |
| 23 | `Ap_every` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 24 | `Ap_some` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 25 | `Ap_forEach` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 26 | `Ap_map` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 27 | `Ap_filter` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 28 | `Ap_reduce` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 29 | `Ap_reduce` | empty array and no initial value | `TypeError: no initial value` | [x] |
| 30 | `Ap_reduce` | sparse array has no present element and no initial value | `TypeError: no initial value` | [x] |
| 31 | `Ap_reduceRight` | callback is not callable | `TypeError: callback is not a function` | [x] |
| 32 | `Ap_reduceRight` | empty array and no initial value | `TypeError: no initial value` | [x] |
| 33 | `Ap_reduceRight` | sparse array has no present element and no initial value | `TypeError: no initial value` | [x] |
| 34 | `Np_valueOf` | receiver is not a Number object | `TypeError: not a number` | [x] |
| 35 | `Np_toString` | receiver is not a Number object | `TypeError: not a number` | [x] |
| 36 | `Np_toString` | radix is below 2 or above 36 | `RangeError: invalid radix` | [x] |
| 37 | `Np_toFixed` | receiver is not a Number object | `TypeError: not a number` | [x] |
| 38 | `Np_toFixed` | precision is below 0 | `RangeError: precision ... out of range` | [x] |
| 39 | `Np_toFixed` | precision is above 20 | `RangeError: precision ... out of range` | [x] |
| 40 | `Np_toExponential` | receiver is not a Number object | `TypeError: not a number` | [x] |
| 41 | `Np_toExponential` | precision is below 0 | `RangeError: precision ... out of range` | [x] |
| 42 | `Np_toExponential` | precision is above 20 | `RangeError: precision ... out of range` | [x] |
| 43 | `Np_toPrecision` | receiver is not a Number object | `TypeError: not a number` | [x] |
| 44 | `Np_toPrecision` | precision is below 1 | `RangeError: precision ... out of range` | [x] |
| 45 | `Np_toPrecision` | precision is above 21 | `RangeError: precision ... out of range` | [x] |
| 46 | `Bp_toString` | receiver is not a Boolean object | `TypeError: not a boolean` | [x] |
| 47 | `Bp_valueOf` | receiver is not a Boolean object | `TypeError: not a boolean` | [x] |
| 48 | `Sp_toString` | receiver is not a String object | `TypeError: not a string` | [x] |
| 49 | `Sp_valueOf` | receiver is not a String object | `TypeError: not a string` | [x] |
| 50 | string methods via `checkstring` | receiver is null or undefined | `TypeError: string function called on null or undefined` | [x] |
| 51 | `js_doregexec` | regexp executor returns negative | `Error: regexec failed` | [x] |
| 52 | `jsV_toprimitive` | strict conversion finds neither usable `toString` nor `valueOf` | `TypeError: cannot convert object to primitive` | [x] |
| 53 | `jsV_toobject` | input is undefined | `TypeError: cannot convert undefined to object` | [x] |
| 54 | `jsV_toobject` | input is null | `TypeError: cannot convert null to object` | [x] |
| 55 | `js_instanceof` | right operand is not callable | `TypeError: instanceof: invalid operand` | [x] |
| 56 | `js_instanceof` | right operand's `prototype` is not an object | `TypeError: instanceof: 'prototype' property is not an object` | [x] |
| 57 | `js_toregexp` | stack value is not a RegExp object | `TypeError: not a regexp` | [x] |
| 58 | `js_touserdata` | stack value/tag does not match userdata | `TypeError: not a <tag>` | [x] |
| 59 | `jsR_tofunction` | stack value is non-null/non-undefined and not callable | `TypeError: not a function` | [x] |
| 60 | `js_pop` | pop count makes `TOP < BOT` | restores `TOP=BOT`, throws `Error: stack underflow!` | [x] |
| 61 | `js_remove` | normalized index is outside `[BOT,TOP)` | `Error: stack error!` | [x] |
| 62 | `js_insert` | any call | `Error: not implemented yet` | [x] |
| 63 | `js_replace` | normalized index is outside `[BOT,TOP)` | `Error: stack error!` | [x] |
| 64 | `CHECKSTACK` | push would make `TOP+n >= JS_STACKSIZE` | stack-overflow error/panic | [x] |
| 65 | `jsR_pushtrace` | trace depth reaches `JS_ENVLIMIT-1` | `Error: call stack overflow` | [x] |
| 66 | `js_call` | argument count is negative | `RangeError: number of arguments cannot be negative` | [x] |
| 67 | `js_call` | callee at `-n-2` is not callable | `TypeError: <type> is not callable` | [x] |
| 68 | `js_construct` | constructor at `-n-1` is not callable | `TypeError: <type> is not callable` | [x] |
| 69 | `js_endtry` | exception stack is empty | `Error: endtry: exception stack underflow` | [x] |
| 70 | `js_savetrypc` | exception stack reaches `JS_TRYLIMIT` | exception-stack overflow panic/error | [x] |
| 71 | `jsR_run` (`OP_HASVAR`) | identifier is absent from every environment | `ReferenceError: '<name>' is not defined` | [x] |
| 72 | `jsR_run` (`OP_GETVAR`) | identifier is absent from every environment | `ReferenceError: '<name>' is not defined` | [x] |
| 73 | `js_setvar` | strict assignment targets an undeclared identifier | `ReferenceError: assignment to undeclared variable '<name>'` | [x] |
| 74 | `jsR_run` (`OP_IN`) | right operand is not an object | `TypeError: operand to 'in' is not an object` | [x] |
| 75 | `jsR_setproperty` | strict assignment targets getter-only property | `TypeError: setting property ... that only has a getter` | [x] |
| 76 | `jsR_setproperty` | strict assignment tries to create property on transient primitive | `TypeError: cannot create property ... on transient object` | [x] |
| 77 | `jsR_setproperty` | strict assignment targets `JS_READONLY` property | `TypeError: '...' is read-only` | [x] |
| 78 | `jsR_defproperty` value | strict redefine targets `JS_READONLY` property | `TypeError: '...' is read-only` | [x] |
| 79 | `jsR_defproperty` getter | strict redefine targets `JS_DONTCONF` property | `TypeError: '...' is non-configurable` | [x] |
| 80 | `jsR_defproperty` setter | strict redefine targets `JS_DONTCONF` property | `TypeError: '...' is non-configurable` | [x] |
| 81 | `jsR_defproperty` attribute conflict | strict/throwing redefine violates read-only or non-configurable attributes | `TypeError: '...' is read-only or non-configurable` | [x] |
| 82 | `jsR_delproperty` | strict delete targets `JS_DONTCONF` property | `TypeError: '...' is non-configurable` | [x] |
| 83 | `js_setvar` | strict environment assignment targets read-only binding | `TypeError: '...' is read-only` | [x] |
| 84 | `js_delvar` | strict delete targets non-configurable binding | `TypeError: '...' is non-configurable` | [x] |
| 85 | `jsV_setproperty` | adding an own property to non-extensible object | `TypeError: object is non-extensible` | [x] |
| 86 | `jsV_nextiterator` | object class is not `JS_CITERATOR` | `TypeError: not an iterator` | [x] |
| 87 | `jsV_resizearray` | target array is still marked simple | assertion failure | [x] |
| 88 | `Fp_toString` | receiver is not callable | `TypeError: not a function` | [x] |
| 89 | `Fp_apply` | receiver is not callable | `TypeError: not a function` | [x] |
| 90 | `Fp_call` | receiver is not callable | `TypeError: not a function` | [x] |
| 91 | `Fp_bind` | receiver is not callable | `TypeError: not a function` | [x] |
| 92 | `Ep_toString` | receiver is not an object | `TypeError: not an object` | [x] |
| 93 | `Op_toString` | receiver is not an object | `TypeError: not an object` | [x] |
| 94 | `Op_valueOf` | receiver is not an object | `TypeError: not an object` | [x] |
| 95 | `Object_defineProperty` | descriptor has both value/writable and get/set fields | `TypeError: value/writable and get/set attributes are exclusive` | [x] |
| 96 | `Object_defineProperties` | descriptor has both value/writable and get/set fields | same `TypeError` | [x] |
| 97 | `Object_getOwnPropertyDescriptor` | argument 1 is not an object | `TypeError: not an object` | [x] |
| 98 | `Object_defineProperty` | target or descriptor is not an object | `TypeError: not an object` | [x] |
| 99 | `Object_defineProperties` | descriptor collection is not an object | `TypeError: not an object` | [x] |
| 100 | `Object_create` | prototype is neither object nor null | `TypeError: not an object or null` | [x] |
| 101 | `Object_getPrototypeOf` | argument is not an object | `TypeError: not an object` | [x] |
| 102 | `Object_preventExtensions` | argument is not an object | `TypeError: not an object` | [x] |
| 103 | `Object_isExtensible` | argument is not an object | `TypeError: not an object` | [x] |
| 104 | `Object_seal` | argument is not an object | `TypeError: not an object` | [x] |
| 105 | `Object_isSealed` | argument is not an object | `TypeError: not an object` | [x] |
| 106 | `Object_freeze` | argument is not an object | `TypeError: not an object` | [x] |
| 107 | `Object_isFrozen` | argument is not an object | `TypeError: not an object` | [x] |
| 108 | `Object_keys` | argument is not an object | `TypeError: not an object` | [x] |
| 109 | `Dp_valueOf` | receiver is not a Date object | `TypeError: not a date` | [x] |
| 110 | date getter/setter via `js_todate` | receiver is not a Date object | `TypeError: not a date` | [x] |
| 111 | `Dp_toISOString` | clipped date is non-finite | `RangeError: invalid date` | [x] |
| 112 | `Dp_toJSON` | `toISOString` property is not callable | `TypeError: this.toISOString is not a function` | [x] |
| 113 | `MakeDay` | month is non-finite or converts outside integer range | returns NaN | [x] |
| 114 | `TimeClip` | time is non-finite | returns NaN | [x] |
| 115 | `TimeClip` | `abs(time) > 8.64e15` | returns NaN | [x] |
| 116 | `parseDate` | year/month/day/time/timezone digit field is truncated or nonnumeric | returns NaN | [x] |
| 117 | `parseDate` | timezone hour exceeds 23 or minute exceeds 59 | returns NaN | [x] |
| 118 | `parseDate` | trailing input remains | returns NaN | [x] |
| 119 | `parseDate` | month outside `1..12` | returns NaN | [x] |
| 120 | `parseDate` | day outside `1..31` | returns NaN | [x] |
| 121 | `parseDate` | hour outside `0..24` | returns NaN | [x] |
| 122 | `parseDate` | minute or second outside `0..59` | returns NaN | [x] |
| 123 | `parseDate` | milliseconds outside `0..999` | returns NaN | [x] |
| 124 | `parseDate` | hour is 24 with nonzero minute/second/millisecond | returns NaN | [x] |
| 125 | `jsonexpect` | current token is not the required token | `SyntaxError: JSON: unexpected token ...` | [x] |
| 126 | `jsonvalue` object | object key token is not a string | `SyntaxError: JSON: unexpected token ... (expected string)` | [x] |
| 127 | `jsonvalue` | token cannot begin a JSON value | `SyntaxError: JSON: unexpected token ...` | [x] |
| 128 | `jsonstringify` array | value graph contains an array cycle | `TypeError: cyclic object value` | [x] |
| 129 | `jsonstringify` object | value graph contains an object cycle | `TypeError: cyclic object value` | [x] |
| 130 | URI decoder | percent escape ends before two hex digits | `URIError: truncated escape sequence` | [x] |
| 131 | URI decoder | percent escape/UTF-8 sequence is malformed | `URIError: invalid escape sequence` | [x] |
| 132 | `js_newregexpx` | regexp compiler reports an error | `SyntaxError: regular expression: <message>` | [x] |
| 133 | `js_RegExp_prototype_exec` | regexp executor returns negative | `Error: regexec failed` | [x] |
| 134 | `Rp_test` | regexp executor returns negative | `Error: regexec failed` | [x] |
| 135 | `jsB_new_RegExp` | RegExp input is cloned while flags are also supplied | `TypeError: cannot supply flags when creating one RegExp from another` | [x] |
| 136 | `jsB_new_RegExp` | flags contain a character other than `g`, `i`, or `m` | `SyntaxError: invalid regular expression flag` | [x] |
| 137 | `jsB_new_RegExp` | `g` occurs more than once | `SyntaxError: invalid regular expression flag: 'g'` | [x] |
| 138 | `jsB_new_RegExp` | `i` occurs more than once | `SyntaxError: invalid regular expression flag: 'i'` | [x] |
| 139 | `jsB_new_RegExp` | `m` occurs more than once | `SyntaxError: invalid regular expression flag: 'm'` | [x] |
| 140 | regexp lexer `nextesc` | escape is truncated | compiler returns `NULL`, sets `errorp` | [x] |
| 141 | regexp lexer `nextesc` | escape character is invalid | compiler returns `NULL`, sets `errorp` | [x] |
| 142 | regexp parser | quantifier syntax/range is invalid | compiler returns `NULL`, sets `errorp` | [x] |
| 143 | regexp parser | hexadecimal/unicode numeric escape overflows | compiler returns `NULL`, sets `errorp` | [x] |
| 144 | regexp parser | character-class count exceeds `REG_MAXCLASS` | compiler returns `NULL`, sets `errorp` | [x] |
| 145 | regexp parser | character-class range is descending/invalid | compiler returns `NULL`, sets `errorp` | [x] |
| 146 | regexp parser | character-class spans exceed `REG_MAXSPAN` | compiler returns `NULL`, sets `errorp` | [x] |
| 147 | regexp parser | character class is unterminated | compiler returns `NULL`, sets `errorp` | [x] |
| 148 | regexp parser | repetition can loop forever on an empty match | compiler returns `NULL`, sets `errorp` | [x] |
| 149 | regexp parser | back-reference is absent or invalid | compiler returns `NULL`, sets `errorp` | [x] |
| 150 | regexp parser | captures exceed `REG_MAXSUB` | compiler returns `NULL`, sets `errorp` | [x] |
| 151 | regexp parser | parenthesis is unmatched | compiler returns `NULL`, sets `errorp` | [x] |
| 152 | regexp compiler | recursion exceeds `REG_MAXREC` | compiler returns `NULL`, sets `errorp` to `stack overflow` | [x] |
| 153 | regexp compiler | program size exceeds `REG_MAXPROG` | compiler returns `NULL`, sets `errorp` | [x] |
| 154 | regexp compiler | allocator cannot allocate program/parse/instruction/class storage | compiler returns `NULL`, sets `errorp` | [x] |
| 155 | regexp executor | recursive match depth exceeds `REG_MAXREC` | returns `-1` | [x] |
| 156 | regexp executor | recursive submatch returns `-1` | propagates `-1` | [x] |
| 157 | lexer numeric literal | malformed hex, missing digits/exponent, leading zero, or letter suffix | protected `SyntaxError` | [x] |
| 158 | lexer string literal | truncated/invalid escape, control character, or unterminated string | protected `SyntaxError` | [x] |
| 159 | lexer regexp literal | unterminated body/class, illegal flag, or duplicate flag | protected `SyntaxError` | [x] |
| 160 | lexer comment/input | unterminated multiline comment or unexpected character | protected `SyntaxError` | [x] |
| 161 | JSON lexer | non-digit, missing fraction/exponent digits, invalid escape/control, unterminated string, unexpected character | protected `SyntaxError` | [x] |
| 162 | parser recursion | `astdepth > JS_ASTLIMIT` | protected `SyntaxError: too much recursion` | [x] |
| 163 | parser token expectations | required punctuation/identifier/expression/switch/for/try token is absent | protected `SyntaxError` | [x] |
| 164 | compiler instruction emitter | instruction operand exceeds encoding width | `SyntaxError: integer overflow in instruction coding` | [x] |
| 165 | compiler jump patcher | jump address exceeds instruction range | `SyntaxError: jump address integer overflow` | [x] |
| 166 | compiler eval check | invalid use of `eval` in binding/reference position | protected `EvalError` | [x] |
| 167 | dtoa `minus` | operands have unequal exponents | assertion failure | [x] |
| 168 | dtoa `minus` | left significand is less than right significand | assertion failure | [x] |
| 169 | exported APIs taking `js_State *` | state pointer is null | C has no guard; caller contract is invalid and process faults/has undefined behavior | [x] |
| 170 | string-taking APIs | required C string pointer is null | C has no guard; caller contract is invalid and process faults/has undefined behavior | [x] |
| 171 | stack-index APIs | zero/oversized positive or negative index outside stack | C `stackidx` resolves to undefined sentinel for reads; mutating operations reject as listed above | [x] |
| 172 | `js_newregexp` flags | integer contains bits outside G/I/M | C passes the integer through; unsupported bits are ignored by matching branches | [x] |

