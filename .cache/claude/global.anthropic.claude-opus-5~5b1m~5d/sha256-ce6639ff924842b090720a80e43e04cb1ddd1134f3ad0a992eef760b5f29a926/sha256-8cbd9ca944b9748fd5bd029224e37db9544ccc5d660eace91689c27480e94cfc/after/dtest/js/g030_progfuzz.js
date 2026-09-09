var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { arr.push([[f(arr.length), arr.length], ((arr.length / ({x:1})) | [1, [1,2]])]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([c, 2]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print([f((0 * s)), []]); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((((o.x ? b : undefined) != arr.length) * ((({}) === arr[0]) <= (true || false)))); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in NaN) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { i = 0; while (i < 2) { ++i; print(String(false), typeof (((typeof (true / d)) ^ [0.5, 2]))); } } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { if (i) { print(-1); } else { i = 0; while (i < 2) { ++i; try { try { print((2 && f(({})))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("caught", e.name); } } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in f(({p: NaN, q: ({p: "s", q: undefined})}))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((s ? (arr[0] === (typeof -1)) : ({p: arr.length, q: f(c)}))); } } catch (err) { print("TOP", err.name, err.message); }
try { if ([f(o.x), o.x]) { print(f(({p: (arr[0] ? i : s), q: (NaN ? a : arr[0])}))); } else { print(String(((2 ? (NaN & 2) : arr.length) ? ({p: (NaN ? -1 : []), q: (o.x ? i : b)}) : (NaN < (({}) ? false : g)))), typeof (Infinity)); } } catch (err) { print("TOP", err.name, err.message); }
try { print((-((a % arr[0]) % s))); } catch (err) { print("TOP", err.name, err.message); }
try { if ([(c % undefined), [(-i), f(0)]]) { for (i = 0; i < 3; ++i) { print((+((null >> "s") ? 0 : "s"))); } } else { for (var k in ((i || -1) ? ((void ({x:1})) & c) : (2 & [true, ""]))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
