var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { if ("s") { print(String(f(({p: (b <= 0.5), q: (void false)}))), typeof ((f(f("s")) >> false))); } else { print(-1); } } } catch (err) { print("TOP", err.name, err.message); }
try { a = [({p: (true < false), q: (0 ^ false)}), 1]; } catch (err) { print("TOP", err.name, err.message); }
try { if ((({p: (o.x / c), q: (0.5 ? "" : -1)}) || f(1))) { c = []; } else { for (i = 0; i < 3; ++i) { print(({x:1})); } } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((({p: (({}) ^ -1), q: f(null)}) << f((i || b)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((((f(1) << 2) + false) ? [("" ? false : g), f(arr.length)] : ((arr.length + d) < [1,2]))) { print(Infinity); } else { a = arr.length; } } catch (err) { print("TOP", err.name, err.message); }
try { if ((-(["s", g] + ({p: ({}), q: b})))) { switch ((f((({}) ? 2 : g)) ? [({p: o.x, q: 0}), (undefined & f(1))] : ((f(1) >> null) ? 0.5 : ({p: null, q: b})))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { for (i = 0; i < 3; ++i) { for (var k in a) print("k", k); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(((g - [0, "s"]) | ((arr.length < true) < (undefined ? null : 0.5)))); } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { if (((f(a) ? f("") : (-g)) ? (({}) << (undefined || a)) : ((arr[0] >>> null) !== f(true)))) { print(0); } else { a = [((0.5 + true) % d), ({p: (void g), q: (arr[0] != o.x)})]; } } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: 0, q: f(f(a))})) { i = 0; while (i < 2) { ++i; for (var k in NaN) print("k", k); } } else { try { print((typeof [f(b), b])); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (1 !== true); })(((-g) != ((i === true) > [f(1), null])))); } catch (err) { print("TOP", err.name, err.message); }
try { print((~[1,2])); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([((({x:1}) | null) ? ("" === true) : ({p: arr[0], q: a})), undefined]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
