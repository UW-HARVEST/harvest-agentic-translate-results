var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (var k in Infinity) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return f(([1,2] && i)); })(a)); } catch (err) { print("TOP", err.name, err.message); }
try { b = (((false & "") < (null >> undefined)) !== g); } catch (err) { print("TOP", err.name, err.message); }
try { if (f(f(false))) { print((false * o.x)); } else { c = ((f(NaN) != (undefined ? ({}) : arr.length)) == [({p: [], q: arr.length}), ([1,2] !== -1)]); } } catch (err) { print("TOP", err.name, err.message); }
try { if (f(a)) { try { arr.push(((![s, c]) ^ ((-true) >= ({p: s, q: [1,2]})))); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } else { for (var k in ([[arr.length, ({x:1})], null] << [(arr[0] % arr[0]), f(null)])) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [({p: arr.length, q: ("" % o.x)}), [(f(1) === null), (0.5 >>> 0)]]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (s <= [false, s]); })(([(0 - 0), ("" ^ "s")] ? (f(s) / 0) : i))); } catch (err) { print("TOP", err.name, err.message); }
try { print(((!({p: g, q: g})) | ((d >>> false) <= d))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: (f(a) >>> (g ? a : "")), q: (1 == f(-1))})); } catch (err) { print("TOP", err.name, err.message); }
try { print(d); } catch (err) { print("TOP", err.name, err.message); }
try { print(((~f(Infinity)) >> i)); } catch (err) { print("TOP", err.name, err.message); }
try { print(2); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
