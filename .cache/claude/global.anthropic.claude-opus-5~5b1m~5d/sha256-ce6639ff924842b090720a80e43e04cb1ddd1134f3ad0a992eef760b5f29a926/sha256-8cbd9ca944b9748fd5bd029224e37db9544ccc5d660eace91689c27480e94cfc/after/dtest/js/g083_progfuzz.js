var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { i = 0; while (i < 2) { ++i; print(true); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (({p: 0.5, q: (NaN ? -1 : 1)}) + b)) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((((undefined ? NaN : a) >= o.x) ? ([0.5, false] - f(NaN)) : f((i % i)))), typeof (-1)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(f([(0 ? g : ""), [arr.length, 0]])); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((c >> ({})) ? [[1,2], b] : NaN) && [NaN, (arr[0] & [])])); } catch (err) { print("TOP", err.name, err.message); }
try { print(((!f(1)) ? [(!arr[0]), (2 === s)] : f((-d)))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(1); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((b == ((~a) ? (2 ? false : arr[0]) : ["", 0]))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((c | (f(c) & (c * f(1))))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print((null - (f(1) - ({p: "s", q: [1,2]})))); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if (((a !== (null ? true : 1)) << ((0 - null) * ["", a]))) { try { c = [d, f(({p: arr[0], q: false}))]; } catch (e) { print("caught", e.name); } } else { print(""); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if ((!([arr[0], 0] % ({p: [1,2], q: undefined})))) { print(undefined); } else { print(null); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
