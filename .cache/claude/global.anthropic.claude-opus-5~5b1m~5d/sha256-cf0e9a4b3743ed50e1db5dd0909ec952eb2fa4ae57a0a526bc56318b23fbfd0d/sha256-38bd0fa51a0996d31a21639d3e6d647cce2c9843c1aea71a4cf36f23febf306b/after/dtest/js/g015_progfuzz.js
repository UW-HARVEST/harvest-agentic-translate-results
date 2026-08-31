var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { if ((~(void ({x:1})))) { for (i = 0; i < 3; ++i) { d = ({x:1}); } } else { a = g; } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { b = (-1 ? (f(0.5) ? (arr[0] >> undefined) : 2) : (([] ? f(1) : true) ? [s, s] : (false && d))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((-o.x) % (-0)); })(0)); } catch (err) { print("TOP", err.name, err.message); }
try { try { i = 0; while (i < 2) { ++i; for (var k in ({p: (({p: o.x, q: "s"}) ? (NaN >> false) : (~f(1))), q: (-(undefined >> arr[0]))})) print("k", k); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: f(f(1)), q: (void ({p: c, q: d}))})); } catch (err) { print("TOP", err.name, err.message); }
try { print((f(f(null)) !== (+true))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(([(+({})), (+g)] ? [(-1 ^ Infinity), (0 <= arr.length)] : (-(g >>> ({}))))), typeof (({p: (undefined ? (null % 1) : (Infinity ? 0 : "s")), q: ((2 ? "s" : 2) ? (undefined - true) : f(1))}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ((false | g) || (null >> "")), q: ([true, 0.5] !== (undefined + false))})); } catch (err) { print("TOP", err.name, err.message); }
try { d = arr[0]; } catch (err) { print("TOP", err.name, err.message); }
try { c = ((({p: ({}), q: c}) & (b != i)) !== (typeof (arr[0] << s))); } catch (err) { print("TOP", err.name, err.message); }
try { a = f(""); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (var k in 0) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
