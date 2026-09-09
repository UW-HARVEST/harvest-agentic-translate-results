var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((((({x:1}) <= b) << (f(1) ? ({x:1}) : false)) ? (typeof 2) : (!NaN))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print(f(((typeof 2) || ({p: arr.length, q: "s"})))); } } } catch (err) { print("TOP", err.name, err.message); }
try { try { b = ([(s >= f(1)), (2 + ({x:1}))] * ((-1 === ({x:1})) === (g | ""))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((0 >>> f(true))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { b = (!false); } } catch (err) { print("TOP", err.name, err.message); }
try { print(([[], (false && c)] * f((void true)))); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([[Infinity, (NaN ? [] : 0.5)], ((arr.length >> true) / ({p: false, q: s}))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(((true ^ undefined) + []))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { print((f(1) * s)); } catch (err) { print("TOP", err.name, err.message); }
try { try { d = ([(Infinity >>> -1), ({})] == "s"); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(""); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
