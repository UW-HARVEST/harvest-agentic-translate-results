/* local time date methods (same TZ in both processes) */
var d = new Date(1234567890123);
print(d.getFullYear(), d.getMonth(), d.getDate(), d.getDay(), d.getHours(), d.getMinutes(), d.getSeconds(), d.getMilliseconds(), d.getTimezoneOffset());
print(d.toString(), d.toDateString(), d.toTimeString(), d.toLocaleString(), d.toLocaleDateString(), d.toLocaleTimeString());
print(d.getYear ? d.getYear() : "no getYear");
var e = new Date(1234567890123);
e.setMilliseconds(9); print(e.getTime());
e.setSeconds(8); print(e.getTime());
e.setMinutes(7); print(e.getTime());
e.setHours(6); print(e.getTime());
e.setDate(5); print(e.getTime());
e.setMonth(4); print(e.getTime());
e.setFullYear(2003); print(e.getTime());
print(new Date(2000, 0, 1).getFullYear(), new Date(2000, 0, 1, 1, 2, 3, 4).getSeconds());
print(new Date(1999, 11, 31, 23, 59, 59).getFullYear());
print(new Date("2000-01-01T00:00:00Z").getTime());
print(new Date(2000, 0).getMonth(), new Date(70, 0, 1).getFullYear());

/* regexp backrefs and greedy/lazy */
print(/(a)\1/.exec("aa"), /(a)\1/.exec("ab"));
print(/(\w+)\s\1/.exec("hi hi there"));
print(/a+?b/.exec("aaab"), /a+b/.exec("aaab"));
try { print(/^(a*)*$/.exec("aaa")); } catch (e) { print(e.name, e.message); }
try { print(/(a|ab)+/.exec("abab")); } catch (e) { print(e.name, e.message); }
print("aaa".replace(/(a)(?=a)/g, "$1!"));
print(/[\b]/.exec("\b"), /\x41/.exec("A"), /A/.exec("A"), /\cA/.exec(""));
print(/[a-]/.exec("-"), /[-a]/.exec("-"), /[\d-x]/.exec("-"));
print("a1b2c3".split(/(\d)/), "aaa".split("a"), "abc".split("", 2), "a,b,c".split(",", 2));
print(/(?:a(b))?c/.exec("c"), /(a)|b/.exec("b"));

/* arguments */
function am() { arguments[0] = 99; arguments.length = 5; return [arguments.length, arguments[0], arguments[1]]; }
print(am(1, 2));
function ac() { return arguments.callee.length; }
print(ac(1,2,3));
(function() { print(typeof arguments, arguments.length); })(1,2,3);

/* sorting objects */
var objs = [{k:3},{k:1},{k:2}];
print(objs.sort(function(a,b){ return a.k - b.k; }).map(function(o){ return o.k; }).join(","));
var strs = ["b","B","a","A","1","_"];
print(strs.sort().join(","));

/* prototype chain shadowing in for-in */
function P() {} P.prototype.shared = 1; P.prototype.over = 2;
var inst = new P(); inst.own = 3; inst.over = 4;
var keys = []; for (var k in inst) keys.push(k + "=" + inst[k]);
print(keys.sort().join(" "));

/* nested json */
var deep = 1;
for (var i = 0; i < 20; ++i) deep = { level: i, child: deep };
print(JSON.stringify(deep).length, JSON.stringify(deep, null, 1).length);
print(JSON.stringify([[[[[[[[[[1]]]]]]]]]]));

/* accessors on arrays */
var arr = [1,2,3];
Object.defineProperty(arr, "sum", { get: function() { return this[0]+this[1]+this[2]; } });
print(arr.sum, arr.length, JSON.stringify(arr));

/* string building and interning */
var acc = "";
for (var i = 0; i < 30; ++i) acc += String.fromCharCode(65 + i % 26);
print(acc, acc.length);
var o2 = {};
for (var i = 0; i < 30; ++i) o2["key" + i] = "val" + i;
print(Object.keys(o2).length, o2.key29);

/* eval scoping */
var ev = 1;
function evf() { var ev = 2; return eval("ev"); }
print(evf(), eval("ev"));
print((function(){ eval("var inner = 5;"); return typeof inner; })());
print(eval("(function(){ return 7; })()"));

/* misc coercions */
print(+[], +[1], +[1,2], +{}, +"", +" ", +null, +undefined, +true);
print([] == "", [0] == false, [1] == true, "1" == true);
print(1 + null, 1 + undefined, "a" + null, null + null);
print((0.1).toFixed(20), (1e-7).toFixed(10));
print(String(1e21), String(1e-7), String(-1e-7), (123).toString(36));
