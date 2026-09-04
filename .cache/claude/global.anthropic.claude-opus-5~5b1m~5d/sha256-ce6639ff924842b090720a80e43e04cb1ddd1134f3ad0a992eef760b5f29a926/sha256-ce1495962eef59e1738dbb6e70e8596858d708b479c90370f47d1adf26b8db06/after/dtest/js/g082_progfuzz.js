var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print("s"); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((((null ? 0.5 : null) ? (s ? g : NaN) : []) * (({p: ({x:1}), q: null}) ? (o.x === c) : (d ? 1 : 0)))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (({p: ({x:1}), q: [o.x, NaN]}) ^ (~i))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if ((-((f(1) | true) ? (-1 != Infinity) : b))) { print(String(((null << (-1 ? [] : f(1))) & (+({p: 2, q: i})))), typeof (({x:1}))); } else { print(([(arr.length || 0), (d ? -1 : "s")] || 1)); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((true ? null : false) ? f(0.5) : undefined); })(c)); } catch (err) { print("TOP", err.name, err.message); }
try { if ((([] === a) / g)) { print((typeof ({p: f(false), q: (typeof NaN)}))); } else { i = 0; while (i < 2) { ++i; try { print(f(((a && i) * d))); } catch (e) { print("caught", e.name); } } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; if ((((g >> undefined) % (~0.5)) !== (void "s"))) { switch (({p: Infinity, q: ((false | 0) * (i ? b : 2))})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { do { switch (({p: 0, q: arr.length})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } while (false); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(o.x); } catch (err) { print("TOP", err.name, err.message); }
try { if ((((arr[0] >>> Infinity) < ({p: ({}), q: 2})) ? ((1 ? false : f(1)) << f(g)) : (f(false) ? s : o.x))) { print(([1,2] | f((g ? "" : f(1))))); } else { if ((arr.length > ((f(1) - arr.length) && a))) { print((function(p) { return ({x:1}); })((({}) ? (f(i) != [[1,2], -1]) : (+undefined)))); } else { if ([((void 2) ^ (-1 ? arr[0] : s)), f((a ? Infinity : [1,2]))]) { c = (([g, 0.5] ^ ({x:1})) - (+(+true))); } else { print(null); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print((g ? ((true ? -1 : i) | (0 > arr.length)) : [])); } catch (err) { print("TOP", err.name, err.message); }
try { if (i) { if ((0 - [(void Infinity), ({p: a, q: null})])) { for (var k in ({x:1})) print("k", k); } else { try { for (var k in (({p: (-1 >> false), q: (f(1) - s)}) && ((i ? c : 2) === (typeof f(1))))) print("k", k); } catch (e) { print("caught", e.name); } } } else { print(f(o.x)); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
