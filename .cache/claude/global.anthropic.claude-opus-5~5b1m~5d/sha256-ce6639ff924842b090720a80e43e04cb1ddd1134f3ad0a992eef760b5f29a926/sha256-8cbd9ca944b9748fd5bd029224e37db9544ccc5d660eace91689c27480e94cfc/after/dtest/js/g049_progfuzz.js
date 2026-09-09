var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (var k in [({p: [i, 0.5], q: [d, Infinity]}), (f(b) >= (({}) | 2))]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if (({})) { print(null); } else { print(a); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ([(f(c) <= [0.5, ""]), (f(1) % (typeof true))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { b = 2; } } catch (err) { print("TOP", err.name, err.message); }
try { print(String((typeof d)), typeof (s)); } catch (err) { print("TOP", err.name, err.message); }
try { if ([1,2]) { switch (((f(s) === ("s" & NaN)) ? ({}) : d)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print(NaN); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(String((((o.x || arr[0]) && c) - "s")), typeof ((((2 == ({x:1})) ^ (1 >> NaN)) == ((-1 * arr[0]) ^ (d - ({})))))); } catch (err) { print("TOP", err.name, err.message); }
try { print(i); } catch (err) { print("TOP", err.name, err.message); }
try { try { if (0) { arr.push(((arr.length * [f(1), ""]) !== (+f(d)))); print(arr.length, arr.join(",")); } else { print((({p: ("" || d), q: (typeof s)}) !== f(("s" >= o.x)))); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(String(s), typeof (([({p: d, q: "s"}), true] <= ("" && (b <= null))))); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; print(arr[0]); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(1); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
