var bad = [
 "1 +", "var", "function", "function f(", "function f(a,", "function f() {", "}", "{", "[", "(",
 "1 +* 2", "var 1 = 2", "var a b", "a b c", "if", "if (", "if (1", "if (1)", "else", "for", "for (;;",
 "while", "while (1", "do", "do {} while", "switch", "switch (1) {", "case 1:", "default:", "try",
 "try {}", "try {} catch", "try {} catch (", "try {} catch (e", "catch (e) {}", "finally {}",
 "return", "break", "continue", "with", "with (", "throw", "throw\n1", "new", "delete", "typeof",
 "1 = 2", "1++", "++1", "a() = 1", "this = 1", "null = 1", "true = 1", "'unterminated", "\"unterminated",
 "0x", "0x_", "1e", "1e+", ".e1", "01.5", "08", "0o8", "1_000", "\\u0041 = 1", "\\", "@", "#", "`",
 "/*unterminated", "/unterminated_re", "var a = /x/zz", "a =", "a ? b", "a ? b :", "{ a: }",
 "[1,,2", "({ get: })", "({ get x })", "({ set x(a) })", "({ get x(a) { } })", "function f(a,a) { 'use strict'; }",
 "'use strict'; delete x;", "'use strict'; with ({}) {}", "'use strict'; var eval = 1;",
 "'use strict'; eval = 1;", "'use strict'; arguments = 1;", "'use strict'; function f(eval) {}",
 "'use strict'; 010;", "'use strict'; var x = 010;", "'use strict'; x = 1;", "class X {}", "let x = 1",
 "const c = 1", "x => x", "function* g() {}", "async function a() {}", "a?.b", "a ?? b", "...x",
 "for (var i in) {}", "for (;;) break x;", "l: for(;;) continue m;", "return 1;", "yield 1",
 "var a = 1; a.1", "a[", "a[1", "a.", "a..b", "1..toString()", "(1).toString()", "new new X",
 "function f() { return\n1; }", "var enum = 1", "var private = 1", "'use strict'; var private = 1",
 "var implements = 1", "'use strict'; var implements = 1", "var super = 1", "é = 1", "$x = 1", "_x = 1"
];
for (var i = 0; i < bad.length; ++i) {
  try {
    var r = eval(bad[i]);
    print(i, JSON.stringify(bad[i]), "OK", String(r));
  } catch (e) {
    print(i, JSON.stringify(bad[i]), e.name + ": " + e.message);
  }
}
print("syntax battery done");
