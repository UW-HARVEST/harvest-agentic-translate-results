var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(((0 / ({})) ? 1 : ((d * f(1)) !== [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { if (((0 > f(1)) >>> null)) { print(1); } else { try { print((!(0 | (undefined > 1)))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { print(d); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(([arr.length, arr[0]] * (typeof b)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch (((!(void ({x:1}))) <= [])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { do { if ((typeof (([] >> f(1)) ? a : "s"))) { for (var k in ({p: Infinity, q: ([arr.length, 0.5] % c)})) print("k", k); } else { if ((f(1) !== (("" === undefined) | (a << 0)))) { print((function(p) { return d; })((((o.x % ({})) % [-1, arr.length]) === [(1 & d), ""]))); } else { c = f(((f(1) - g) >>> 0)); } } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { try { arr.push(arr.length); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { a = 1; } catch (err) { print("TOP", err.name, err.message); }
try { try { a = f(""); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((void ({p: b, q: 1})) === ((f(1) - o.x) / 2))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (((-1 ? "s" : null) === (g < f(1))) | (+(-1 * -1)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
