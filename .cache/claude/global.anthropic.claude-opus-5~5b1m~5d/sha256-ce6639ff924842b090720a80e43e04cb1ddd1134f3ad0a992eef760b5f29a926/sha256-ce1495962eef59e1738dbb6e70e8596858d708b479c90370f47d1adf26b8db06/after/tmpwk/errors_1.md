| # | function | trigger (exact invalid input/condition) | expected C result |
|---|---|---|---|
| 1 | `jsY_tokenstring` (jslex.c:66-72) | any error-message formatting where `token < 0`, `token >= nelem(tokenstring)`, or `tokenstring[token] == NULL` (the 0x80..0xFF gap filled with `0`), e.g. `jsY_tokenstring(0x85)` | returns the literal string `"<unknown>"` (sentinel, no throw) |
| 2 | `jsY_findword` (jslex.c:81-96) | binary search miss: `jsY_findword("foo", keywords, 29)` / `jsY_findword("bar", futurewords, 7)` | returns `-1` (sentinel; makes `jsY_findkeyword` return `TK_IDENTIFIER`, makes `checkfutureword` skip) |
| 3 | `jsY_unescape` (jslex.c:184) | identifier-position `\u` with non-hex in 1st hex slot: `js_ploadstring` with source `var \uZ123;` | `goto error` → `SyntaxError: "<file>:<line>: unexpected escape sequence"` (thrown via `js_newsyntaxerror`+`js_throw`) |
| 4 | `jsY_unescape` (jslex.c:185) | non-hex in 2nd hex slot: source `var \u0Z12;` | `SyntaxError: "<file>:<line>: unexpected escape sequence"` |
| 5 | `jsY_unescape` (jslex.c:186) | non-hex in 3rd hex slot: source `var \u00Z1;` | `SyntaxError: "<file>:<line>: unexpected escape sequence"` |
| 6 | `jsY_unescape` (jslex.c:187) | non-hex in 4th hex slot: source `var \u000Z;` | `SyntaxError: "<file>:<line>: unexpected escape sequence"` |
| 7 | `jsY_unescape` (jslex.c:181,191-192) | backslash at token start not followed by `u`: source `var \x41;` or `\q` | falls into `error:` label → `SyntaxError: "<file>:<line>: unexpected escape sequence"` |
| 8 | `lexcomment` (jslex.c:238-248) | `/*` block comment hits EOF without `*/`: source `/* abc` | returns `-1` (sentinel to caller `jsY_lexx`) |
| 9 | `jsY_lexx` (jslex.c:572-574) | `lexcomment` returned non-zero (unterminated `/* ...`): source `var a; /* oops` | `SyntaxError: "<file>:<line>: multi-line comment not terminated"` |
| 10 | `lexhex` (jslex.c:254-255) | `0x`/`0X` prefix with no hex digit following: source `0x;` or `0xg` | `SyntaxError: "<file>:<line>: malformed hexadecimal number"` |
| 11 | `lexnumber` (jslex.c:350-351) | leading `0` followed by a decimal digit (legacy octal): source `012` | `SyntaxError: "<file>:<line>: number with leading zero"` |
| 12 | `lexnumber` (jslex.c:376-377) | `e`/`E` exponent marker with no digits: source `1e`, `1e+`, `1E-` | `SyntaxError: "<file>:<line>: missing exponent"` |
| 13 | `lexnumber` (jslex.c:380-381) | identifier-start rune immediately after a numeric literal: source `123abc`, `1.5px`, `0.1$` | `SyntaxError: "<file>:<line>: number with letter suffix"` |
| 14 | `lexnumber` (jslex.c:356-358) | `.` not followed by a decimal digit (dispatched from `jsY_lexx` case `'.'`): source `a.b` | returns the token `'.'` (0x2E) instead of `TK_NUMBER` (sentinel, no error) |
| 15 | `lexescape` (jslex.c:398-399) | EOF immediately after a backslash inside a string: source `"abc\` (file ends) | `SyntaxError: "<file>:<line>: unterminated escape sequence"` |
| 16 | `lexescape` (jslex.c:402) | `\u` in string, non-hex in 1st slot: source `"\uZ123"` | returns `1` → caller raises `malformed escape sequence` |
| 17 | `lexescape` (jslex.c:403) | `\u` in string, non-hex in 2nd slot: source `"\u0Z12"` | returns `1` |
| 18 | `lexescape` (jslex.c:404) | `\u` in string, non-hex in 3rd slot: source `"\u00Z1"` | returns `1` |
| 19 | `lexescape` (jslex.c:405) | `\u` in string, non-hex in 4th slot: source `"\u000Z"` | returns `1` |
| 20 | `lexescape` (jslex.c:410) | `\x` in string, non-hex in 1st slot: source `"\xZ1"` | returns `1` |
| 21 | `lexescape` (jslex.c:411) | `\x` in string, non-hex in 2nd slot: source `"\x4Z"` | returns `1` |
| 22 | `lexstring` (jslex.c:439-440) | raw newline or EOF inside a `'`/`"` string: source `"abc<LF>def"` or `'abc` (EOF) | `SyntaxError: "<file>:<line>: string not terminated"` |
| 23 | `lexstring` (jslex.c:441-443) | `lexescape` returned 1 (bad `\u`/`\x`): source `"\xZZ"` | `SyntaxError: "<file>:<line>: malformed escape sequence"` |
| 24 | `lexstring` (jslex.c:449) via `jsY_expect` | loop exited but `J->lexchar != q` (defensive; unreachable through normal input) | `SyntaxError: "<file>:<line>: expected '<q>'"` |
| 25 | `lexregexp` (jslex.c:488-490) | EOF or newline inside a regexp body: source `var r = /abc` (EOF) or `/ab<LF>c/` | `SyntaxError: "<file>:<line>: regular expression not terminated"` |
| 26 | `lexregexp` (jslex.c:496-497) | EOF or newline immediately after a backslash in a regexp body: source `var r = /ab\` (EOF) | `SyntaxError: "<file>:<line>: regular expression not terminated"` |
| 27 | `lexregexp` (jslex.c:510) via `jsY_expect` | loop exited with `J->lexchar != '/'` (defensive; unreachable through normal input) | `SyntaxError: "<file>:<line>: expected '/'"` |
| 28 | `lexregexp` (jslex.c:517-521) | identifier-part rune after `/re/` that is not `g`/`i`/`m`: source `/a/x`, `/a/y`, `/a/1` | `SyntaxError: "<file>:<line>: illegal flag in regular expression: <c>"` |
| 29 | `lexregexp` (jslex.c:524-525) | any of `g`,`i`,`m` repeated: source `/a/gg`, `/a/gimg`, `/a/ii` | `SyntaxError: "<file>:<line>: duplicated flag in regular expression"` |
| 30 | `jsY_lexx` (jslex.c:727-728) | printable ASCII (0x20..0x7E) that starts no token and is no identifier start: source `@`, `#`, `` ` ``, `\` | `SyntaxError: "<file>:<line>: unexpected character: '<c>'"` |
| 31 | `jsY_lexx` (jslex.c:729) | non-printable or non-ASCII rune that is no identifier start: source containing raw `\x01`, `\x7F`, or `U+00A1` | `SyntaxError: "<file>:<line>: unexpected character: \u<XXXX>"` |
| 32 | `lexjsonnumber` (jslex.c:754-760) | JSON number whose first (post-sign) char is not `0`..`9`: `JSON.parse("-x")`, `JSON.parse("-")` | `SyntaxError: "<file>:<line>: unexpected non-digit"` |
| 33 | `lexjsonnumber` (jslex.c:762-767) | JSON `.` with no digit after it: `JSON.parse("1.")`, `JSON.parse("1.e5")` | `SyntaxError: "<file>:<line>: missing digits after decimal point"` |
| 34 | `lexjsonnumber` (jslex.c:770-777) | JSON `e`/`E` with no digit after it (or after sign): `JSON.parse("1e")`, `JSON.parse("1e+")` | `SyntaxError: "<file>:<line>: missing digits after exponent indicator"` |
| 35 | `lexjsonescape` (jslex.c:790-791) | JSON string escape char not in `u"\/bfnrt`: `JSON.parse("\"\\q\"")`, `JSON.parse("\"\\'\"")` | `SyntaxError: "<file>:<line>: invalid escape sequence"` |
| 36 | `lexjsonescape` (jslex.c:794) | JSON `\u` non-hex in 1st slot: `JSON.parse("\"\\uZ123\"")` | returns `1` (return value is **discarded** by `lexjsonstring`, so lexing silently continues) |
| 37 | `lexjsonescape` (jslex.c:795) | JSON `\u` non-hex in 2nd slot: `JSON.parse("\"\\u0Z12\"")` | returns `1` (discarded by caller) |
| 38 | `lexjsonescape` (jslex.c:796) | JSON `\u` non-hex in 3rd slot: `JSON.parse("\"\\u00Z1\"")` | returns `1` (discarded by caller) |
| 39 | `lexjsonescape` (jslex.c:797) | JSON `\u` non-hex in 4th slot: `JSON.parse("\"\\u000Z\"")` | returns `1` (discarded by caller) |
| 40 | `lexjsonstring` (jslex.c:818-820) | EOF before the closing `"`: `JSON.parse("\"abc")` | `SyntaxError: "<file>:<line>: unterminated string"` |
| 41 | `lexjsonstring` (jslex.c:821-822) | raw control character `< 32` inside a JSON string: `JSON.parse("\"a\tb\"")` (literal TAB/LF) | `SyntaxError: "<file>:<line>: invalid control character in string"` |
| 42 | `lexjsonstring` (jslex.c:830) via `jsY_expect` | loop exited with `J->lexchar != '"'` (defensive; unreachable through normal input) | `SyntaxError: "<file>:<line>: expected '\"'"` |
| 43 | `jsY_lexjson` (jslex.c:862, expect `'a'`) | JSON token starting with `f` whose 2nd char is not `a`: `JSON.parse("fxlse")` | `SyntaxError: "<file>:<line>: expected 'a'"` |
| 44 | `jsY_lexjson` (jslex.c:862, expect `'l'`) | `JSON.parse("faxse")` | `SyntaxError: "<file>:<line>: expected 'l'"` |
| 45 | `jsY_lexjson` (jslex.c:862, expect `'s'`) | `JSON.parse("falxe")` | `SyntaxError: "<file>:<line>: expected 's'"` |
| 46 | `jsY_lexjson` (jslex.c:862, expect `'e'`) | `JSON.parse("falsx")` | `SyntaxError: "<file>:<line>: expected 'e'"` |
| 47 | `jsY_lexjson` (jslex.c:866, expect `'u'`) | `JSON.parse("nxll")` | `SyntaxError: "<file>:<line>: expected 'u'"` |
| 48 | `jsY_lexjson` (jslex.c:866, expect 1st `'l'`) | `JSON.parse("nuxl")` | `SyntaxError: "<file>:<line>: expected 'l'"` |
| 49 | `jsY_lexjson` (jslex.c:866, expect 2nd `'l'`) | `JSON.parse("nulx")` | `SyntaxError: "<file>:<line>: expected 'l'"` |
| 50 | `jsY_lexjson` (jslex.c:870, expect `'r'`) | `JSON.parse("txue")` | `SyntaxError: "<file>:<line>: expected 'r'"` |
| 51 | `jsY_lexjson` (jslex.c:870, expect `'u'`) | `JSON.parse("trxe")` | `SyntaxError: "<file>:<line>: expected 'u'"` |
| 52 | `jsY_lexjson` (jslex.c:870, expect `'e'`) | `JSON.parse("trux")` | `SyntaxError: "<file>:<line>: expected 'e'"` |
| 53 | `jsY_lexjson` (jslex.c:877-878) | printable ASCII char that starts no JSON token: `JSON.parse("'a'")`, `JSON.parse("(")`, `JSON.parse("+1")` | `SyntaxError: "<file>:<line>: unexpected character: '<c>'"` |
| 54 | `jsY_lexjson` (jslex.c:879) | non-printable/non-ASCII rune outside a string: `JSON.parse("\x01")`, `JSON.parse("\u00e9")` | `SyntaxError: "<file>:<line>: unexpected character: \u<XXXX>"` |
| 55 | `semicolon` (jsparse.c:145-153) | statement not terminated by `;`, `}`, EOF, or a preceding newline: `js_ploadstring` with source `var a = 1 var b = 2;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ';')"` |
| 56 | `identifier` (jsparse.c:158-166) | `TK_IDENTIFIER` required but lookahead is anything else: source `var 1;`, `function 2(){}`, `try{}catch(3){}`, `({set x(1){}})`, `for(var ;;)` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected identifier)"` |
| 57 | `identifiername` (jsparse.c:176-183) | after `.` or as a property name, lookahead is neither `TK_IDENTIFIER` nor `>= TK_BREAK`: source `a.1`, `a."x"`, `({ 'a'+1 : 2 })` reaching `propname`→`identifiername` with `+` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected identifier or keyword)"` |
| 58 | `identifieropt` (jsparse.c:169-173) | lookahead is not `TK_IDENTIFIER` where an optional identifier is allowed: source `var f = function(){}`, `break;`, `continue;` | returns `NULL` (sentinel, no error) |
| 59 | `arrayliteral` (jsparse.c:197-198) | empty array literal: source `[]` | returns `NULL` (empty-production sentinel, no error) |
| 60 | `objectliteral` (jsparse.c:255-256) | empty object literal: source `({})` | returns `NULL` (empty-production sentinel, no error) |
| 61 | `parameters` (jsparse.c:271-272) | empty parameter list: source `function f(){}` | returns `NULL` (empty-production sentinel, no error) |
| 62 | `arguments` (jsparse.c:369-370) | empty argument list: source `f()` | returns `NULL` (empty-production sentinel, no error) |
| 63 | `statementlist` (jsparse.c:674-675) | statement list immediately closed by `}`, `case`, or `default`: source `{}` or `switch(x){case 1: case 2: }` | returns `NULL` (empty-production sentinel, no error) |
| 64 | `caselist` (jsparse.c:706-707) | empty switch body: source `switch(x){}` | returns `NULL` (empty-production sentinel, no error) |
| 65 | `script` (jsparse.c:938-939) | source is empty / body is empty: `js_ploadstring` with source `""`, or `function f(){}` body | returns `NULL` (empty-production sentinel, no error) |
| 66 | `propassign` (jsparse.c:232) `jsP_expect(J,'(')` | getter without `(`: source `({ get x 1 })` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 67 | `propassign` (jsparse.c:233) `jsP_expect(J,')')` | getter with a parameter: source `({ get x(a){} })` | `SyntaxError: "<file>:<line>: unexpected token: (identifier) (expected ')')"` |
| 68 | `propassign` (jsparse.c:239) `jsP_expect(J,'(')` | setter without `(`: source `({ set x 1 })` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 69 | `propassign` (jsparse.c:241) `jsP_expect(J,')')` | setter with two or zero-plus-junk parameters: source `({ set x(a,b){} })` | `SyntaxError: "<file>:<line>: unexpected token: ',' (expected ')')"` |
| 70 | `propassign` (jsparse.c:247) `jsP_expect(J,':')` | object property without `:`: source `({ a 1 })`, `({ 1 2 })`, `({ "a" })` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ':')"` |
| 71 | `fundec` (jsparse.c:284) `jsP_expect(J,'(')` | top-level function declaration missing `(`: source `function f {}` | `SyntaxError: "<file>:<line>: unexpected token: '{' (expected '(')"` |
| 72 | `fundec` (jsparse.c:286) `jsP_expect(J,')')` | declaration parameter list not closed: source `function f(a {}` | `SyntaxError: "<file>:<line>: unexpected token: '{' (expected ')')"` |
| 73 | `funstm` (jsparse.c:295) `jsP_expect(J,'(')` | nested/statement-position function missing `(`: source `if (1) function f {}` | `SyntaxError: "<file>:<line>: unexpected token: '{' (expected '(')"` |
| 74 | `funstm` (jsparse.c:297) `jsP_expect(J,')')` | nested function parameter list not closed: source `if (1) function f(a {}` | `SyntaxError: "<file>:<line>: unexpected token: '{' (expected ')')"` |
| 75 | `funexp` (jsparse.c:307) `jsP_expect(J,'(')` | function expression missing `(`: source `var f = function {}` | `SyntaxError: "<file>:<line>: unexpected token: '{' (expected '(')"` |
| 76 | `funexp` (jsparse.c:309) `jsP_expect(J,')')` | function expression parameter list not closed: source `var f = function(a {}` | `SyntaxError: "<file>:<line>: unexpected token: '{' (expected ')')"` |
| 77 | `primary` (jsparse.c:349) `jsP_expect(J,'}')` | object literal not closed: source `({ a: 1` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected '}')"` |
| 78 | `primary` (jsparse.c:354) `jsP_expect(J,']')` | array literal not closed: source `[1, 2` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected ']')"` |
| 79 | `primary` (jsparse.c:359) `jsP_expect(J,')')` | parenthesized expression not closed: source `(1 + 2` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected ')')"` |
| 80 | `primary` (jsparse.c:363) | no primary-expression token at all: source `var a = ;`, `+ ;`, `* 1`, `a = )` | `SyntaxError: "<file>:<line>: unexpected token in expression: <tok>"` |
| 81 | `newexp` (jsparse.c:387) `jsP_expect(J,')')` | `new` argument list not closed: source `new Foo(1` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected ')')"` |
| 82 | `memberexp` (jsparse.c:405) `INCREC()` | more than `JS_ASTLIMIT` (400) nested member/index accesses in a `new` callee: source `new a.b.c…` with 401+ `.x` links | `SyntaxError: "<file>:<line>: too much recursion"` |
| 83 | `memberexp` (jsparse.c:408) `jsP_expect(J,']')` | index in `new` callee not closed: source `new a[0` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected ']')"` |
| 84 | `callexp` (jsparse.c:419) `INCREC()` | more than 400 chained `.`/`[]`/`()` postfix operations: source `a()()()…` repeated 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 85 | `callexp` (jsparse.c:422) `jsP_expect(J,']')` | index expression not closed: source `a[0` (EOF), `a[1 2]` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ']')"` |
| 86 | `callexp` (jsparse.c:423) `jsP_expect(J,')')` | call argument list not closed: source `f(1` (EOF), `f(1 2)` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 87 | `unary` (jsparse.c:441) `INCREC()` | more than 400 nested unary operators: source `!!!!…x` with 401 `!` | `SyntaxError: "<file>:<line>: too much recursion"` |
| 88 | `multiplicative` (jsparse.c:462) `INCREC()` | more than 400 `*`/`/`/`%` operators in one chain: source `1*1*1*…` with 401 `*` | `SyntaxError: "<file>:<line>: too much recursion"` |
| 89 | `additive` (jsparse.c:477) `INCREC()` | more than 400 `+`/`-` operators in one chain: source `1+1+1+…` with 401 `+` | `SyntaxError: "<file>:<line>: too much recursion"` |
| 90 | `shift` (jsparse.c:491) `INCREC()` | more than 400 `<<`/`>>`/`>>>` operators in one chain: source `1<<1<<1…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 91 | `relational` (jsparse.c:506) `INCREC()` | more than 400 `<`/`>`/`<=`/`>=`/`instanceof`/`in` operators in one chain: source `1<1<1<…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 92 | `equality` (jsparse.c:524) `INCREC()` | more than 400 `==`/`!=`/`===`/`!==` operators in one chain: source `1==1==1…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 93 | `bitand` (jsparse.c:540) `INCREC()` | more than 400 `&` operators in one chain: source `1&1&1&…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 94 | `bitxor` (jsparse.c:554) `INCREC()` | more than 400 `^` operators in one chain: source `1^1^1^…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 95 | `bitor` (jsparse.c:568) `INCREC()` | more than 400 `\|` operators in one chain: source `1\|1\|1\|…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 96 | `logand` (jsparse.c:581) `INCREC()` | more than 400 right-nested `&&` operators: source `1&&1&&1…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 97 | `logor` (jsparse.c:593) `INCREC()` | more than 400 right-nested `\|\|` operators: source `1\|\|1\|\|1…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 98 | `conditional` (jsparse.c:606) `INCREC()` | more than 400 nested `?:` operators: source `1?1:1?1:…` nested 401 deep | `SyntaxError: "<file>:<line>: too much recursion"` |
| 99 | `conditional` (jsparse.c:608) `jsP_expect(J,':')` | conditional expression missing `:`: source `a ? b c`, `a ? b;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ':')"` |
| 100 | `assignment` (jsparse.c:620) `INCREC()` | more than 400 right-nested assignment operators: source `a=a=a=…1` with 401 `=` | `SyntaxError: "<file>:<line>: too much recursion"` |
| 101 | `expression` (jsparse.c:643) `INCREC()` | more than 400 comma operators in one expression: source `1,1,1,…` 401 times | `SyntaxError: "<file>:<line>: too much recursion"` |
| 102 | `caseclause` (jsparse.c:689) `jsP_expect(J,':')` | `case <exp>` without `:`: source `switch(x){ case 1 break; }` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ':')"` |
| 103 | `caseclause` (jsparse.c:695) `jsP_expect(J,':')` | `default` without `:`: source `switch(x){ default break; }` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ':')"` |
| 104 | `caseclause` (jsparse.c:700) | switch body content that is neither `case` nor `default`: source `switch(x){ foo(); }` | `SyntaxError: "<file>:<line>: unexpected token in switch: <tok> (expected 'case' or 'default')"` |
| 105 | `block` (jsparse.c:718) `jsP_expect(J,'{')` | block expected but no `{` (reached from `try`/`catch`/`finally`): source `try 1; catch(e){}` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '{')"` |
| 106 | `block` (jsparse.c:720) `jsP_expect(J,'}')` | block not closed: source `{ var a;` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected '}')"` |
| 107 | `forexpression` (jsparse.c:729) `jsP_expect(J,end)` | `for` header sub-expression not terminated by the expected `;`/`)`: source `for(;1 2;)`, `for(;;1 2)`, `for(;;` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ';')"` or `(expected ')')` |
| 108 | `forstatement` (jsparse.c:736) `jsP_expect(J,'(')` | `for` without `(`: source `for x;;) ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 109 | `forstatement` (jsparse.c:747) `jsP_expect(J,')')` | `for(var x in y` not closed: source `for(var a in b ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 110 | `forstatement` (jsparse.c:751) | after `for(var <declist>`, next token is neither `;` nor `in`: source `for(var a b) ;`, `for(var a)` | `SyntaxError: "<file>:<line>: unexpected token in for-var-statement: <tok>"` |
| 111 | `forstatement` (jsparse.c:766) `jsP_expect(J,')')` | `for(x in y` not closed: source `for(a in b ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 112 | `forstatement` (jsparse.c:770) | after `for(<exp>`, next token is neither `;` nor `in`: source `for(a b) ;`, `for(a)` | `SyntaxError: "<file>:<line>: unexpected token in for-statement: <tok>"` |
| 113 | `statement` (jsparse.c:779) `INCREC()` | more than 400 nested statements: source `{{{{…}}}}` 401 deep, or `if(1)if(1)if(1)…;` 401 deep | `SyntaxError: "<file>:<line>: too much recursion"` |
| 114 | `statement` (jsparse.c:797) `jsP_expect(J,'(')` | `if` without `(`: source `if x ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 115 | `statement` (jsparse.c:799) `jsP_expect(J,')')` | `if (` condition not closed: source `if (x ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 116 | `statement` (jsparse.c:810) `jsP_expect(J,TK_WHILE)` | `do <stm>` not followed by `while`: source `do ; until (0);`, `do ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected 'while')"` |
| 117 | `statement` (jsparse.c:811) `jsP_expect(J,'(')` | `do…while` without `(`: source `do ; while 0;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 118 | `statement` (jsparse.c:813) `jsP_expect(J,')')` | `do…while(` not closed: source `do ; while (0 ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 119 | `statement` (jsparse.c:819) `jsP_expect(J,'(')` | `while` without `(`: source `while x ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 120 | `statement` (jsparse.c:821) `jsP_expect(J,')')` | `while(` not closed: source `while (x ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 121 | `statement` (jsparse.c:852) `jsP_expect(J,'(')` | `with` without `(`: source `with x ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 122 | `statement` (jsparse.c:854) `jsP_expect(J,')')` | `with(` not closed: source `with (x ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 123 | `statement` (jsparse.c:860) `jsP_expect(J,'(')` | `switch` without `(`: source `switch x {}` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 124 | `statement` (jsparse.c:862) `jsP_expect(J,')')` | `switch(` not closed: source `switch (x {}` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 125 | `statement` (jsparse.c:863) `jsP_expect(J,'{')` | `switch(x)` not followed by `{`: source `switch (x) case 1: ;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '{')"` |
| 126 | `statement` (jsparse.c:865) `jsP_expect(J,'}')` | switch body not closed (defensive after `caselist`): source `switch (x) {` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected '}')"` |
| 127 | `statement` (jsparse.c:879) `jsP_expect(J,'(')` | `catch` without `(`: source `try{}catch e {}` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '(')"` |
| 128 | `statement` (jsparse.c:881) `jsP_expect(J,')')` | `catch(` not closed: source `try{}catch(e {}` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected ')')"` |
| 129 | `statement` (jsparse.c:887-888) | `try` block with neither `catch` nor `finally`: source `try {}` | `SyntaxError: "<file>:<line>: unexpected token in try: <tok> (expected 'catch' or 'finally')"` |
| 130 | `statement` (jsparse.c:898) `jsP_warning` | function declaration in statement (non-script-element) position: source `if (1) function f(){}` | non-fatal: `js_report(J, "<file>:<line>: warning: function statements are not standard")`, parsing continues |
| 131 | `funbody` (jsparse.c:949) `jsP_expect(J,'{')` | function body without `{`: source `function f() return 1;` | `SyntaxError: "<file>:<line>: unexpected token: <tok> (expected '{')"` |
| 132 | `funbody` (jsparse.c:951) `jsP_expect(J,'}')` | function body not closed: source `function f(){` (EOF) | `SyntaxError: "<file>:<line>: unexpected token: (end-of-file) (expected '}')"` |
| 133 | `emitraw` (jscompile.c:74-75) | emitted value does not round-trip through `js_Instruction` (default `unsigned short`): compiling a script where `emit()` writes a line number > 65535 (66000-line file), or an opcode arg > 65535 (>65535 locals/args) | `SyntaxError: "integer overflow in instruction coding"` (via `js_syntaxerror`, no file:line prefix) |
| 134 | `checkfutureword` (jscompile.c:42-43) | identifier equal to a future reserved word (`class`,`const`,`enum`,`export`,`extends`,`import`,`super`) used as a var/param/label/catch/function name: source `var class;`, `function const(){}`, `break super;` | `SyntaxError: "<file>:<line>: '<name>' is a future reserved word"` |
| 135 | `checkfutureword` (jscompile.c:44-46) | in strict mode, identifier equal to a strict future reserved word (`implements`,`interface`,`let`,`package`,`private`,`protected`,`public`,`static`,`yield`): source `"use strict"; var let;` | `SyntaxError: "<file>:<line>: '<name>' is a strict mode future reserved word"` |
| 136 | `addlocal` (jscompile.c:112-114) | in strict mode, declaring a var/param/function named `arguments`: source `"use strict"; var arguments;` or `"use strict"; function f(arguments){}` | `SyntaxError: "<file>:<line>: redefining 'arguments' is not allowed in strict mode"` |
| 137 | `addlocal` (jscompile.c:115-116) | in strict mode, declaring a var/param/function named `eval`: source `"use strict"; var eval;` | `SyntaxError: "<file>:<line>: redefining 'eval' is not allowed in strict mode"` |
| 138 | `addlocal` (jscompile.c:117-119) | in non-strict mode, declaring a var/param/function named `eval`: source `var eval;`, `function f(eval){}` | `EvalError: "<file>:<line>: invalid use of 'eval'"` (via `js_evalerror`) |
| 139 | `addlocal` (jscompile.c:121-128) | in strict mode, duplicate formal parameter name (`reuse == 0` path): source `"use strict"; function f(a,a){}` | `SyntaxError: "<file>:<line>: duplicate formal parameter '<name>'"` |
| 140 | `findlocal` (jscompile.c:140-146) | identifier is not in `F->vartab` (a global / free variable): source `undeclaredGlobal;` | returns `-1` (sentinel → `emitlocal` emits the `OP_*VAR` string form instead of `OP_*LOCAL`) |
| 141 | `emitlocal` (jscompile.c:202-204) | in strict mode, assigning to `arguments` (`oploc == OP_SETLOCAL`): source `"use strict"; function f(){ arguments = 1; }` | `SyntaxError: "<file>:<line>: 'arguments' is read-only in strict mode"` |
| 142 | `emitlocal` (jscompile.c:205-206) | in strict mode, assigning to a local named `eval`: source `"use strict"; eval = 1;` (after `eval` became a local) | `SyntaxError: "<file>:<line>: 'eval' is read-only in strict mode"` |
| 143 | `emitlocal` (jscompile.c:208-209) | any non-call reference to the identifier `eval` (read, write, delete, typeof): source `eval = 1;`, `var x = eval;`, `delete eval;` | `EvalError: "<file>:<line>: invalid use of 'eval'"` (via `js_evalerror`) |
| 144 | `emitjumpto` (jscompile.c:236-238) | backward jump target does not fit in `js_Instruction`: compiling a loop whose body exceeds 65535 instructions (`while(1){ …65536+ instrs… }`) | `SyntaxError: "jump address integer overflow"` (via `js_syntaxerror`) |
| 145 | `labelto` (jscompile.c:243-245) | forward jump/patch address does not fit in `js_Instruction`: an `if`/`&&`/`?:`/`try`/`break` whose patch target is at code offset > 65535 | `SyntaxError: "jump address integer overflow"` (via `js_syntaxerror`) |
| 146 | `checkdup` (jscompile.c:307-315) | in strict mode, duplicate property key in an object literal (numeric keys compared after `jsV_numbertostring`): source `"use strict"; ({a:1, a:2})` or `"use strict"; ({1:1, 1.0:2})` | `SyntaxError: "<file>:<line>: duplicate property '<key>' in object literal"` |
| 147 | `cobject` (jscompile.c:329-336) | object-literal key AST node whose type is not `AST_IDENTIFIER`, `EXP_STRING`, or `EXP_NUMBER` (defensive; `propname` cannot produce this — reachable only if constant folding rewrote the key node) | `SyntaxError: "<file>:<line>: invalid property name in object initializer"` |
| 148 | `cassign` (jscompile.c:399-400) | `=` whose LHS is not identifier/index/member: source `1 = 2;`, `f() = 1;`, `(a,b) = 1;`, `this = 1;` | `SyntaxError: "<file>:<line>: invalid l-value in assignment"` |
| 149 | `cassignforin` (jscompile.c:408-410) | `for (var …)` in-loop with more than one declarator: source `for (var a, b in c) ;` | `SyntaxError: "<file>:<line>: more than one loop variable in for-in statement"` |
| 150 | `cassignforin` (jscompile.c:438-439) | `for (<exp> in …)` where the LHS is not identifier/index/member: source `for (1 in x) ;`, `for (f() in x) ;` | `SyntaxError: "<file>:<line>: invalid l-value in for-in loop assignment"` |
| 151 | `cassignop1` (jscompile.c:463-464) | compound assignment or ++/-- whose operand is not identifier/index/member: source `1 += 2;`, `1++;`, `--f();`, `this += 1;` | `SyntaxError: "<file>:<line>: invalid l-value in assignment"` |
| 152 | `cassignop2` (jscompile.c:486-487) | same operand shapes as #151 in the store phase (defensive; `cassignop1` fires first for every reachable input) | `SyntaxError: "<file>:<line>: invalid l-value in assignment"` |
| 153 | `cdelete` (jscompile.c:506-508) | in strict mode, `delete` of a bare identifier: source `"use strict"; delete x;` | `SyntaxError: "<file>:<line>: delete on an unqualified name is not allowed in strict mode"` |
| 154 | `cdelete` (jscompile.c:523-524) | `delete` of an operand that is not identifier/index/member: source `delete 1;`, `delete f();`, `delete this;` | `SyntaxError: "<file>:<line>: invalid l-value in delete expression"` |
| 155 | `cexp` (jscompile.c:779-780) | expression AST node whose `type` matches no `case` in `cexp` (e.g. an `AST_IDENTIFIER`/`AST_LIST`/`EXP_PROP_*` node reaching `cexp` through `cstm`'s `default:` branch) | `SyntaxError: "<file>:<line>: unknown expression type"` |
| 156 | `breaktarget` (jscompile.c:832-846) | walk hits a function boundary (`isfun`) or the AST root without finding a loop/switch (or matching label): source `break;` at top level, `function f(){ break; }`, `x: { function g(){ break x; } }` | returns `NULL` (sentinel consumed at jscompile.c:1216/1220) |
| 157 | `continuetarget` (jscompile.c:849-862) | walk hits a function boundary or the AST root without finding a loop (or matching label): source `continue;` at top level, `switch(x){case 1: continue;}` | returns `NULL` (sentinel consumed at jscompile.c:1232/1236) |
| 158 | `returntarget` (jscompile.c:865-872) | walk reaches the AST root without finding any `AST_FUNDEC`/`EXP_FUN`/`EXP_PROP_GET`/`EXP_PROP_SET` ancestor: source `return 1;` in a top-level script | returns `NULL` (sentinel consumed at jscompile.c:1250) |
| 159 | `ctrycatch` (jscompile.c:959-961) | strict mode `catch` parameter named `arguments`, no `finally`: source `"use strict"; try{}catch(arguments){}` | `SyntaxError: "<file>:<line>: redefining 'arguments' is not allowed in strict mode"` |
| 160 | `ctrycatch` (jscompile.c:962-963) | strict mode `catch` parameter named `eval`, no `finally`: source `"use strict"; try{}catch(eval){}` | `SyntaxError: "<file>:<line>: redefining 'eval' is not allowed in strict mode"` |
| 161 | `ctrycatchfinally` (jscompile.c:992-993) | strict mode `catch` parameter named `arguments`, with `finally`: source `"use strict"; try{}catch(arguments){}finally{}` | `SyntaxError: "<file>:<line>: redefining 'arguments' is not allowed in strict mode"` |
| 162 | `ctrycatchfinally` (jscompile.c:994-995) | strict mode `catch` parameter named `eval`, with `finally`: source `"use strict"; try{}catch(eval){}finally{}` | `SyntaxError: "<file>:<line>: redefining 'eval' is not allowed in strict mode"` |
| 163 | `cswitch` (jscompile.c:1023-1025) | two or more `default` clauses in one `switch`: source `switch(x){default: ; default: ;}` | `SyntaxError: "<file>:<line>: more than one default label in switch"` |
| 164 | `cstm` (jscompile.c:1213-1217) | labelled `break` whose label is not an enclosing label: source `foo: while(1){} break bar;`, `function f(){ break foo; }` | `SyntaxError: "<file>:<line>: break label '<name>' not found"` |
| 165 | `cstm` (jscompile.c:1219-1221) | unlabelled `break` not inside a loop or switch: source `break;`, `if(1) break;`, `function f(){ break; }` | `SyntaxError: "<file>:<line>: unlabelled break must be inside loop or switch"` |
| 166 | `cstm` (jscompile.c:1229-1233) | labelled `continue` whose label does not name an enclosing loop: source `foo: { continue foo; }`, `continue bar;` | `SyntaxError: "<file>:<line>: continue label '<name>' not found"` |
| 167 | `cstm` (jscompile.c:1235-1237) | unlabelled `continue` not inside a loop: source `continue;`, `switch(x){case 1: continue;}` | `SyntaxError: "<file>:<line>: continue must be inside loop"` |
| 168 | `cstm` (jscompile.c:1249-1251) | `return` outside any function: source `return;` or `return 1;` at script top level | `SyntaxError: "<file>:<line>: return not in function"` |
| 169 | `cstm` (jscompile.c:1263-1266) | `with` statement in strict-mode code: source `"use strict"; with(x){}` (or `J->default_strict` set) | `SyntaxError: "<file>:<line>: 'with' statements are not allowed in strict mode"` |
