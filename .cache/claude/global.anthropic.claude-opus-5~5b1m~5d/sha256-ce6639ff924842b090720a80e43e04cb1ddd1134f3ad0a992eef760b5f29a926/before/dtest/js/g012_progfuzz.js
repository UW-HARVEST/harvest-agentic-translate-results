var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { print(String(([(({x:1}) + f(1)), (2 || 2)] ^ f((d & false)))), typeof (o.x)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { do { o.y = ((1 === 0.5) >= (-[1,2])); print(JSON.stringify(o)); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(((typeof 0.5) != (+(false >> null)))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (var k in ({p: (({p: a, q: 0}) !== (g ? 2 : undefined)), q: i})) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { try { b = (({p: b, q: (+NaN)}) < f((({}) ? i : undefined))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { if ((({p: b, q: f(g)}) != ({p: ({p: "", q: true}), q: (+s)}))) { try { arr.push([(void (o.x * null)), f((d * null))]); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } else { print(({p: ((g ? Infinity : arr.length) ? (f(1) & 0) : (arr[0] < "s")), q: ({p: o.x, q: [NaN, null]})})); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { do { print((NaN | false)); } while (false); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ((void (undefined < -1)) ? f(({p: NaN, q: d})) : f(Infinity)); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(Infinity); } catch (err) { print("TOP", err.name, err.message); }
try { print((-NaN)); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [(arr.length | false), ((a ? Infinity : b) << (NaN > b))]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
