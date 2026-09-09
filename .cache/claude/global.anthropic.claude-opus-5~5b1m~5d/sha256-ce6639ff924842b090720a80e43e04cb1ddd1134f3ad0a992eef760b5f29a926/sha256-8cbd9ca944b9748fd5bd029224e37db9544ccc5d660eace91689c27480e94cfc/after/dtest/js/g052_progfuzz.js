var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { print(""); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(({p: ((i || "") & [1,2]), q: ((typeof c) < Infinity)})); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in "") print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(g); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; print((function(p) { return d; })((([Infinity, true] === f(undefined)) == (false === -1)))); } } } catch (err) { print("TOP", err.name, err.message); }
try { a = (typeof b); } catch (err) { print("TOP", err.name, err.message); }
try { d = arr[0]; } catch (err) { print("TOP", err.name, err.message); }
try { if (f((({x:1}) && [({}), ({})]))) { print(((!["", 2]) == (~f(g)))); } else { o.y = ((Infinity / ({p: 0, q: NaN})) + f((i != 1))); print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((({p: f(1), q: b}) <= (({x:1}) * NaN)) ? (o.x / (f(1) ? arr.length : [1,2])) : (({p: [1,2], q: ""}) ? (({}) | f(1)) : arr[0]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(({p: [s, "s"], q: arr[0]}))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((function(p) { return (i ? g : []); })(f((({p: null, q: -1}) ^ o.x)))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: ("s" / Infinity), q: (f(1) & 0.5)}) * (c ? (null || NaN) : (void arr[0])))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
