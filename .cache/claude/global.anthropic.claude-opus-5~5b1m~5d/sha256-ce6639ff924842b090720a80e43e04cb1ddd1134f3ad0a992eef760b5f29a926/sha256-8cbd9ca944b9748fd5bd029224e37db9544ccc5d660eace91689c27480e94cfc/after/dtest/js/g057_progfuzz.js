var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(([(NaN === i), ("" ? NaN : true)] ? ((b >> c) >>> (+2)) : (arr[0] == (f(1) === s)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((((+2) ? a : ({p: arr.length, q: arr.length})) ? [(-s), (o.x ? arr[0] : "s")] : (2 * d))); } } catch (err) { print("TOP", err.name, err.message); }
try { b = undefined; } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: (arr[0] ? (0 !== true) : (f(1) < Infinity)), q: f(false)})) { print(({p: (f(({})) / f(NaN)), q: f((-1 == "s"))})); } else { print(""); } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { b = f(1); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print(true); } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((({x:1}) && (arr.length ? (-1 | "") : "s"))); } catch (err) { print("TOP", err.name, err.message); }
try { b = 0.5; } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ({p: ([1,2] || (a ? a : i)), q: f([a, o.x])}); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { print((~(typeof (2 > 0.5)))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [({}), (-1 < b)]; })(({p: ((o.x << true) <= f(b)), q: ((s ? undefined : Infinity) ^ (+2))}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((typeof arr.length) <= (~1)))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
