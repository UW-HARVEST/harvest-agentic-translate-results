var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = []; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(String([({p: (1 || ""), q: (o.x === c)}), ([-1, c] === a)]), typeof ([f(({x:1})), ((arr[0] == 2) !== (0 ? o.x : 0.5))])); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in []) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(1)); } catch (err) { print("TOP", err.name, err.message); }
try { print(({x:1})); } catch (err) { print("TOP", err.name, err.message); }
try { print([[(null ? o.x : 1), arr[0]], ((d / 1) * [[1,2], c])]); } catch (err) { print("TOP", err.name, err.message); }
try { if ((f(null) ? [arr[0], true] : f((2 || f(1))))) { if (i) { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; if ([(+(arr[0] - [])), -1]) { if ((typeof ({p: f(({x:1})), q: g}))) { a = (((!o.x) + []) === f((false ? ({}) : undefined))); } else { if (f(b)) { print(([(d < ""), (i ? 2 : 1)] ? f((!({}))) : ([2, ""] === (~arr.length)))); } else { for (i = 0; i < 3; ++i) { if (((-"") << (a !== ("s" + arr[0])))) { switch ((undefined - [1,2])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { i = 0; while (i < 2) { ++i; if ((-((s ? s : a) > (Infinity * ({x:1}))))) { print(0); } else { d = c; } } } } } } } else { print(f(arr[0])); } } } } else { switch ((c >>> g)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } else { print(([f([1,2]), (~0.5)] <= ({p: [({x:1}), 1], q: f(b)}))); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((typeof ((arr[0] > ({})) / (o.x ? i : null)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(g); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((([2, [1,2]] ? (f(1) > ({})) : s) ? ["s", (undefined != arr[0])] : NaN)); } } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (b === undefined), q: []}) >= "s")); } catch (err) { print("TOP", err.name, err.message); }
try { if ((([] >> (i <= Infinity)) >= f(([] == arr.length)))) { b = (({p: ("" >= Infinity), q: "s"}) ? -1 : (f([]) << ["s", undefined])); } else { print(f(((c << c) * undefined))); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
