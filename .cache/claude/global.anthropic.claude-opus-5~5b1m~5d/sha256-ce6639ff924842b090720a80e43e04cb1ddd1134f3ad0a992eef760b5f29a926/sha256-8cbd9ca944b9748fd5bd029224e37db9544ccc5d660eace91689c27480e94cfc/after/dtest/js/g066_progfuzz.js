var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((((true > "s") > (d >>> arr.length)) >= [f(g), [[1,2], o.x]])); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; if (f(((true ? "" : Infinity) == d))) { print((((f(1) >> o.x) < (typeof undefined)) ? b : ({p: (b ^ ""), q: [Infinity, 1]}))); } else { try { print(((1 >> 0.5) >>> ("" > (b - d)))); } catch (e) { print("c", e.name); } finally { print("fin"); } } } } catch (err) { print("TOP", err.name, err.message); }
try { try { c = (((d ? arr[0] : []) << (0 - g)) === 0); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { a = Infinity; } catch (err) { print("TOP", err.name, err.message); }
try { o.y = f(null); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: (f(null) != ({p: -1, q: c})), q: [d, (null ? "" : i)]})); } catch (err) { print("TOP", err.name, err.message); }
try { switch (null) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if (Infinity) { do { o.y = (((({x:1}) || f(1)) ? [1, null] : i) !== ({p: [0.5, d], q: (null >= 0)})); print(JSON.stringify(o)); } while (false); } else { switch ((Infinity % f((({}) | -1)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in arr[0]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { c = (((i ? g : f(1)) || (i + false)) >= [({p: [1,2], q: d}), (s >>> arr.length)]); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((((f(1) ? 2 : arr[0]) >> [null, true]) + c)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(a)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
