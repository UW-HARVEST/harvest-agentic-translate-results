function f(a, b) { var loc = a + b; debugger; return loc; }
print(f(1, 2));
var o = { arr: [1, "two", null], fn: f, num: 3.25, re: /x/g, err: new Error("e"), d: new Date(0), s: new String("s"), n: new Number(1), b: new Boolean(1) };
function g() { debugger; }
g();
print("trap done");
