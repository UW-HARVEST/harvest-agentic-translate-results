var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { print([arr.length, g]); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof ({}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((-((2 === true) >> [f(1), undefined]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(f(b)), typeof ((f([false, f(1)]) * ((true ? f(1) : 1) / ({p: false, q: arr.length}))))); } catch (err) { print("TOP", err.name, err.message); }
try { do { d = ((+({p: 1, q: s})) <= ({p: [a, []], q: (c !== 0)})); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((({p: f(1), q: ""}) ? ({p: null, q: null}) : (({x:1}) ? true : null)) ? ((d & false) ? (i ? "" : [1,2]) : f("s")) : ({x:1}))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { for (i = 0; i < 3; ++i) { switch ((({p: i, q: (-NaN)}) ? true : ({p: (a != []), q: (typeof null)}))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((s ? (o.x & (-1 * [])) : "")); } catch (err) { print("TOP", err.name, err.message); }
try { print(((null ? (Infinity >> 0) : b) / null)); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ({p: ((~b) / (f(1) << null)), q: ({})}); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { b = -1; } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(((arr.length | (0.5 ^ 0)) << ((arr.length != ({})) <= [2, g]))); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
