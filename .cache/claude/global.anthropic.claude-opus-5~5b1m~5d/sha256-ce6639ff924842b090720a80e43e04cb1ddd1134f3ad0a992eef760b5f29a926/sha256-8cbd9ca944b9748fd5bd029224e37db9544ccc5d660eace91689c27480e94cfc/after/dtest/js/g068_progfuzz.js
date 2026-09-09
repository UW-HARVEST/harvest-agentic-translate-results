var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { do { o.y = ([true, f(o.x)] + arr[0]); print(JSON.stringify(o)); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((arr[0] | (!(~NaN)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((((0 >= s) > (typeof d)) + ({x:1}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({})); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; do { for (var k in undefined) print("k", k); } while (false); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f(b)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { d = [(b * (({x:1}) << true)), undefined]; } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; b = (((b ? "s" : b) ? [true, g] : (~Infinity)) > (+[1,2])); } } catch (err) { print("TOP", err.name, err.message); }
try { print((0 ? 0.5 : ({p: (i ? "" : i), q: (f(1) & [1,2])}))); } catch (err) { print("TOP", err.name, err.message); }
try { a = (f((null ? "" : g)) >> (!(a <= arr.length))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ((({p: "", q: ({x:1})}) || "") + ((i < undefined) ? 2 : i))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if (({})) { a = f([(~[1,2]), g]); } else { try { print(String(((a ? f(true) : ({p: "s", q: []})) & arr.length)), typeof (({p: ((b % ({})) === [f(1), []]), q: ((false ? o.x : Infinity) * (c & d))}))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
