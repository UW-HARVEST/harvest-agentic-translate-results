var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(({p: f((false >= [])), q: f((b ? null : NaN))})); } catch (err) { print("TOP", err.name, err.message); }
try { print(((~(void arr.length)) !== ({p: [d, arr[0]], q: [[], s]}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return s; })(o.x)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((({p: (c < b), q: (+a)}) ? 2 : [("" > -1), (typeof -1)])); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((function(p) { return (Infinity >>> [1,2]); })(s)); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (null) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(b); } catch (err) { print("TOP", err.name, err.message); }
try { do { switch (f(c)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { i = 0; while (i < 2) { ++i; try { c = (NaN * ({p: ("" ? NaN : s), q: (b ? b : true)})); } catch (e) { print("caught", e.name); } } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { if ((({p: 0.5, q: o.x}) ? (f(b) ? (undefined - f(1)) : (s - Infinity)) : false)) { a = (f((+-1)) / ((g != o.x) ? (b > c) : (!c))); } else { d = (void ((g >>> ({x:1})) | (c / f(1)))); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(({p: ((-1 == "s") ? (f(1) === b) : (-1 ? g : arr[0])), q: ((i ? Infinity : "s") % true)})); } } catch (err) { print("TOP", err.name, err.message); }
try { try { d = (((0.5 ? 0.5 : 1) * -1) % [({}), (0 ^ [1,2])]); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
