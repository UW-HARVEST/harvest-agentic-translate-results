var types = ["Error", "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError", "URIError"];
for (var i = 0; i < types.length; ++i) {
  var E = this[types[i]];
  var e = new E("msg " + i);
  print(e.name, e.message, e.toString(), String(e), e instanceof Error, e instanceof E);
  print(typeof e.stackTrace, typeof e.stack);
}
print(new Error().message, new Error().toString(), Error("no new").message);
var e2 = new Error("m"); e2.name = ""; print(e2.toString());
e2.name = "N"; e2.message = ""; print(e2.toString());
function deep(n) { if (n == 0) throw new Error("deep error"); return deep(n-1); }
try { deep(5); } catch (e) { print(e.message); print(typeof e.stackTrace === "string" ? "has trace" : "no trace"); }
try { [].length = -1; } catch (e) { print(e.name, e.message); }
try { "x".length = 5; print("no error"); } catch (e) { print(e.name); }
try { decodeURIComponent("%"); } catch (e) { print(e.name, e.message); }
try { decodeURI("%C0%80"); } catch (e) { print(e.name, e.message); }
try { (1).toFixed(101); } catch (e) { print(e.name, e.message); }
try { (1).toPrecision(0); } catch (e) { print(e.name, e.message); }
try { (1).toString(1); } catch (e) { print(e.name, e.message); }
try { new Array(-1); } catch (e) { print(e.name, e.message); }
try { null(); } catch (e) { print(e.name, e.message); }
try { ({}).x.y; } catch (e) { print(e.name, e.message); }
try { eval("}"); } catch (e) { print(e.name); }
try { Object.defineProperty(1, "x", {}); } catch (e) { print(e.name, e.message); }
try { throw "string throw"; } catch (e) { print(typeof e, e); }
try { throw { custom: 1 }; } catch (e) { print(e.custom); }
try { undefined.foo(); } catch (e) { print(e.name, e.message); }
try { var x = {}; x(); } catch (e) { print(e.name, e.message); }
try { new 5; } catch (e) { print(e.name, e.message); }
try { for (var k in 5) print(k); } catch (e) { print("forin", e.name); }
try { ("x" in "y"); } catch (e) { print(e.name, e.message); }
try { unknown_global_var.x = 1; } catch (e) { print(e.name, e.message); }
print("done errors");
