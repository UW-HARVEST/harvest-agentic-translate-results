function fact(n) { return n <= 1 ? 1 : n * fact(n-1); }
print(fact(10), fact(20));
var i, s = 0;
for (i = 0; i < 10; ++i) { if (i == 3) continue; if (i == 8) break; s += i; }
print(i, s);
outer: for (var x = 0; x < 3; ++x) { for (var y = 0; y < 3; ++y) { if (y == 1) continue outer; if (x == 2) break outer; print(x, y); } }
var n = 0; do { ++n; } while (n < 5); print(n);
while (n > 0) { --n; } print(n);
switch (3) { case 1: print("one"); case 3: print("three"); case 4: print("four"); break; default: print("def"); }
switch ("x") { default: print("default first"); case "y": print("y"); }
try { throw new Error("thrown"); } catch (e) { print("caught", e.name, e.message, e instanceof Error); } finally { print("finally"); }
try { try { throw 42; } finally { print("inner finally"); } } catch (e) { print("outer", e); }
function f() { try { return "try"; } finally { print("f finally"); } }
print(f());
try { null.x; } catch (e) { print("TypeError?", e.name, e.message); }
try { undefinedFunction(); } catch (e) { print(e.name, e.message); }
try { var q = {}; q.a.b = 1; } catch (e) { print(e.name); }
print((function(){ return arguments.length + ":" + arguments[0]; })(1,2,3));
var clo = []; for (var j = 0; j < 3; ++j) clo.push(function() { return j; });
print(clo[0](), clo[1]());
function counter() { var c = 0; return function() { return ++c; }; }
var cnt = counter(); print(cnt(), cnt(), cnt());
print((function fib(n){ return n<2?n:fib(n-1)+fib(n-2); })(20));
print(typeof (function(){}), (function(a,b){}).length);
print(eval("1+1"), eval("var ev = 5; ev*2"), typeof ev);
print((function(){ "use strict"; return this === undefined; })());
var obj = { m: function() { return this === obj; } }; print(obj.m());
function F(){ this.z = 1; } print(new F().z, (new F()) instanceof F);
print(void 0, !0, !!"", -"3", +"4", 1 && 2, 0 || "x");
var w = { a: 1 };
with (w) { a = 2; var b = 3; }
print(w.a, b, typeof a);
for (var kk in { x: 1 }) { with ({ y: 2 }) { print(kk, y); } }
function args() { arguments[0] = "mod"; return arguments.length + arguments[0] + (arguments.callee === args); }
print(args("orig", 2));
var nested = function() { return function() { return function() { return "deep"; }; }; };
print(nested()()());
var recurse = function(n) { return n > 0 ? recurse(n - 1) + "." : ""; };
print(recurse(20).length);
print((function(a, a) { return a; })(1, 2));
print(function(){ return; }(), (function(){ return undefined; })());
var t = 0; for (var q1 = 0; q1 < 3; q1++) for (var q2 = 0; q2 < 3; q2++) t += q1 * q2;
print(t);
if (true) print("if-true"); else print("if-false");
print(1 ? 2 ? "a" : "b" : "c");
print(typeof (0, print));
