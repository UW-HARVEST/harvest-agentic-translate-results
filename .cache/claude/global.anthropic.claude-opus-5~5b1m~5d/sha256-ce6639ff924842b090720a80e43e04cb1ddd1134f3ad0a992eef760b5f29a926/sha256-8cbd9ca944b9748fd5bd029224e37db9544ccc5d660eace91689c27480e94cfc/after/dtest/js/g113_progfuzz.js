var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((function(p) { return (({p: o.x, q: 0}) ? [null, "s"] : [c, 0]); })((((typeof NaN) || (s ? arr[0] : 2)) ^ (o.x & ({p: 0, q: Infinity}))))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(([({}), i] >> (s !== i)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { c = s; } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { b = (((s ? NaN : f(1)) ? (null && arr.length) : (+null)) / [(null == s), (o.x ? s : ({x:1}))]); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { o.y = f(((!a) ? [({}), c] : "s")); print(JSON.stringify(o)); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(0.5); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { try { if (((a != s) - (["s", o.x] - null))) { print(String(f(1)), typeof ((undefined > ((a && []) ? [0.5, s] : ([] > ""))))); } else { print((-2)); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { d = f([({p: o.x, q: g}), (-[])]); } catch (err) { print("TOP", err.name, err.message); }
try { print(([f(Infinity), (-1 ? i : [1,2])] ? 1 : ({p: (1 << arr.length), q: f("s")}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof ((~o.x) | (Infinity === ({x:1}))))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (+[[2, s], (+c)])) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(arr.length); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
