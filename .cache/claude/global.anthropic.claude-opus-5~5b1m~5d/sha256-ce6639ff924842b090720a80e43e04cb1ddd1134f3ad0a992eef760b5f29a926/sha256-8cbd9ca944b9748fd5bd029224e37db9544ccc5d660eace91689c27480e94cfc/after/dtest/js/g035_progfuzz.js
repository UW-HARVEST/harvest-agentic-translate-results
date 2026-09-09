var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if (f(o.x)) { print(o.x); } else { for (var k in -1) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print(((c - 1) ? f((-1 <= 2)) : ((-1 >>> NaN) ^ [o.x, i]))); } } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(String(f(({p: (a % false), q: ({p: ({x:1}), q: 2})}))), typeof (f(f((arr[0] ? arr.length : arr[0]))))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String(f(1)), typeof (f((void (s ? a : arr.length))))); } catch (err) { print("TOP", err.name, err.message); }
try { print((~({}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(""); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(0); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((typeof ((g && a) ? (2 ? o.x : [1,2]) : ("" >> arr.length)))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { c = (((2 << i) < ("" != a)) / [f(arr[0]), [[1,2], undefined]]); } catch (err) { print("TOP", err.name, err.message); }
try { print((((arr.length ? -1 : c) << (({x:1}) ? d : [1,2])) * f(({p: c, q: ({})})))); } catch (err) { print("TOP", err.name, err.message); }
try { try { o.y = (~[[b, ({x:1})], ("" != 2)]); print(JSON.stringify(o)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { print(((typeof s) * "")); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
