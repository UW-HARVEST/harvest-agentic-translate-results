var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f((!(null ? -1 : 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(null); } catch (err) { print("TOP", err.name, err.message); }
try { print(false); } catch (err) { print("TOP", err.name, err.message); }
try { print((((i === Infinity) | (i ? null : 2)) ^ ([arr[0], []] && (+f(1))))); } catch (err) { print("TOP", err.name, err.message); }
try { print(([[true, []], [i, ({x:1})]] | [(NaN < a), c])); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print([undefined, ((arr.length < undefined) ^ (undefined * arr[0]))]); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print([([1, 0.5] ? (arr.length * Infinity) : [null, f(1)]), (c == (void 1))]); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(1)); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([(i >> ([] <= "s")), Infinity]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((+(a ^ f(null)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [1,2]; })(((typeof [true, Infinity]) === ({x:1})))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (f((+c)) ? (f("") - (-1 ? arr.length : g)) : ((+o.x) != ([] || g)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
