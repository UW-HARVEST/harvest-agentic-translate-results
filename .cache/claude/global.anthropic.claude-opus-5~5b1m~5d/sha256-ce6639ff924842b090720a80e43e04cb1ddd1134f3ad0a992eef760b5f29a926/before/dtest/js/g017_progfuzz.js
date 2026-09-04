var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { switch (f(arr.length)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (0.5 / -1), q: (0 ? "s" : s)}) <= [(1 <= arr[0]), (c ? "" : -1)])); } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(f((a >>> o.x)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((typeof ((null ? [] : b) >>> [1, undefined]))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [(({}) & 2), f(1)]; })((((typeof "s") > [[1,2], -1]) / ([NaN, []] <= (b ? NaN : [1,2]))))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((arr[0] * [1,2])); } } catch (err) { print("TOP", err.name, err.message); }
try { if (g) { o.y = -1; print(JSON.stringify(o)); } else { if (({})) { c = i; } else { for (i = 0; i < 3; ++i) { print((((arr.length + ({})) % [({x:1}), ({})]) == (({p: 1, q: d}) >>> (g / "s")))); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print((f((g == 0)) ? arr[0] : (-("" & ({x:1}))))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (false) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in 1) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { do { print(({p: (d !== (void false)), q: ((0 < Infinity) * ({p: "", q: ({})}))})); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(true), typeof ("s")); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
