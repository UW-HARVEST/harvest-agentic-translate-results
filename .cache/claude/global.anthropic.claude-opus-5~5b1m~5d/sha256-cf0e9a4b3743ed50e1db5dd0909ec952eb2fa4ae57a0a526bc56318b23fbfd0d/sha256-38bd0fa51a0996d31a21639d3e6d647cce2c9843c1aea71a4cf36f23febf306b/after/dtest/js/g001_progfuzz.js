var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if (((({}) && (false && b)) & ([0, false] ? ({x:1}) : ([] & 0.5)))) { print((({p: (null ? g : false), q: (void -1)}) ? Infinity : i)); } else { print(({p: Infinity, q: (f(1) * (NaN / s))})); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((({p: ([1,2] == Infinity), q: f(1)}) / ({p: "", q: []}))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((false > NaN)); } catch (err) { print("TOP", err.name, err.message); }
try { switch (((!true) ^ 2)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if (f([])) { if ((-((g >= ({})) <= (c % NaN)))) { for (i = 0; i < 3; ++i) { arr.push((((null * g) ? i : f(d)) ? ({p: (b - -1), q: ({p: NaN, q: arr[0]})}) : (!({p: c, q: i})))); print(arr.length, arr.join(",")); } } else { try { print((f(f(d)) == ((o.x ^ i) / [i, 2]))); } catch (e) { print("c", e.name); } finally { print("fin"); } } } else { try { print((({p: arr.length, q: f(2)}) ? 1 : ({}))); } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(((NaN ? ({}) : s) / [])); } catch (err) { print("TOP", err.name, err.message); }
try { c = (({p: "s", q: ("s" != -1)}) && f((({x:1}) ? NaN : 1))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [f(NaN), arr[0]]; })([[(b !== g), (-g)], ((b < undefined) & (d ? c : i))])); } catch (err) { print("TOP", err.name, err.message); }
try { print(0); } catch (err) { print("TOP", err.name, err.message); }
try { print((([({x:1}), o.x] / (f(1) !== 0)) ^ ((true % g) ? (f(1) | true) : null))); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch (0.5) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({x:1})); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
