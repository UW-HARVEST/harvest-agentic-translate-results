var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((-[[true, -1], (0.5 - g)])); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((({x:1}) + Infinity)), typeof ([([] || (0.5 || "")), ([-1, arr.length] ? f(NaN) : [({}), NaN])])); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch ([[1,2], (+b)]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (void ("" != 0)); })(([o.x, c] - (({x:1}) && (NaN <= 1))))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (var k in [({p: "s", q: [false, arr.length]}), (+0.5)]) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (d) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if (0) { if ((((o.x ? "" : ({})) && (d / a)) << ("" ? d : arr.length))) { try { print("s"); } catch (e) { print("caught", e.name); } } else { switch (((g >= (-true)) ^ f([arr.length, -1]))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } else { print(s); } } catch (err) { print("TOP", err.name, err.message); }
try { c = (Infinity !== (f(f(1)) >> [s, 0.5])); } catch (err) { print("TOP", err.name, err.message); }
try { try { d = (1 && (2 < ([] && a))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { a = 0.5; } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { a = [0.5, arr.length]; } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
