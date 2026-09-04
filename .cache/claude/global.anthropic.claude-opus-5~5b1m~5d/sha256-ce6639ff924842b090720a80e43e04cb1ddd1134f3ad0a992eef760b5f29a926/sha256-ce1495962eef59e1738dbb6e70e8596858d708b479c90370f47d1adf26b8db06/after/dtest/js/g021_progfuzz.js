var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(("s" !== ({p: [], q: (true >> 2)}))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { print((function(p) { return []; })((((NaN ^ arr.length) > (0 ? 0 : "s")) | ((2 === s) ? [true, ({})] : o.x)))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = ({x:1}); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch (((({p: 0, q: ({x:1})}) <= ([] >>> "s")) ^ false)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print((f((-1 - [1,2])) / (void (~[1,2])))); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(f(f(({p: -1, q: arr.length})))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((-[-1, (s & ({x:1}))])); } catch (err) { print("TOP", err.name, err.message); }
try { d = (({p: (![]), q: (i >= "")}) && ([Infinity, -1] < "")); } catch (err) { print("TOP", err.name, err.message); }
try { print(([s, [s, c]] < ((({}) ? false : b) ? (typeof "s") : 0))); } catch (err) { print("TOP", err.name, err.message); }
try { print(false); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print(c); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print((f(({x:1})) * ((g === 1) << [g, arr.length]))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
