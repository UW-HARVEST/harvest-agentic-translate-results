var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = d; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { if (((~([] ? i : -1)) >> [false, (~2)])) { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print([({p: ({p: ({}), q: s}), q: g}), ((f(1) ? s : "") !== ({p: 0.5, q: arr.length}))]); } } } else { c = d; } } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [[({p: ({x:1}), q: undefined}), f(c)], f(1)]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(s); } catch (err) { print("TOP", err.name, err.message); }
try { print((~((({}) >> NaN) || (s ? b : ({x:1}))))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (+g)) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print((f([o.x, -1]) && (~["s", b]))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(([(false >> g), (NaN * "s")] == ({x:1}))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((void (f(1) ? f(s) : (-arr.length)))) { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { arr.push((typeof ((c & f(1)) != arr[0]))); print(arr.length, arr.join(",")); } } } else { for (var k in 0) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String(({p: c, q: 2})), typeof (false)); } catch (err) { print("TOP", err.name, err.message); }
try { print([[f(1), ([1,2] << g)], [("" && 2), 1]]); } catch (err) { print("TOP", err.name, err.message); }
try { if ((arr.length / ([arr[0], undefined] ? (i << o.x) : c))) { print((function(p) { return NaN; })(f(({p: [d, f(1)], q: [[], undefined]})))); } else { o.y = (({p: (!0.5), q: (+({x:1}))}) ? ({p: s, q: (0 == [1,2])}) : null); print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
