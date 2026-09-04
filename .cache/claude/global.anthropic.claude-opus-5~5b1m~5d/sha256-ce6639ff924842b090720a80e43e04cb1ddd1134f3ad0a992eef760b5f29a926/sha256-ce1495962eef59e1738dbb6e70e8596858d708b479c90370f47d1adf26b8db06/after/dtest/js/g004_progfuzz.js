var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { a = (((f(1) && 2) - (true ? d : s)) ? f((typeof 2)) : [true, (null & b)]); } catch (err) { print("TOP", err.name, err.message); }
try { print(("s" && (({p: undefined, q: -1}) >> [s, 1]))); } catch (err) { print("TOP", err.name, err.message); }
try { if (a) { if (([({p: ({}), q: -1}), [arr[0], o.x]] ^ (f(({x:1})) ? null : (2 ? 0.5 : null)))) { switch (0.5) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { i = 0; while (i < 2) { ++i; if (null) { if (f(f(f([1,2])))) { for (i = 0; i < 3; ++i) { print([({p: [1,2], q: (~o.x)}), f(1)]); } } else { i = 0; while (i < 2) { ++i; try { try { if (((([1,2] < b) <= [NaN, g]) / [b, (NaN ? -1 : o.x)])) { i = 0; while (i < 2) { ++i; try { print("s"); } catch (e) { print("caught", e.name); } } } else { o.y = Infinity; print(JSON.stringify(o)); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("caught", e.name); } } } } else { print(Infinity); } } } } else { d = ((({}) | false) ? (f(d) ? ({p: d, q: c}) : (arr[0] ? 0.5 : "s")) : o.x); } } catch (err) { print("TOP", err.name, err.message); }
try { print((-(2 != (0 <= [1,2])))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((function(p) { return ({p: ({x:1}), q: undefined}); })(({p: [b, ("s" / o.x)], q: ([null, g] ? (1 != b) : Infinity)}))); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ("") { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return 0.5; })(undefined)); } catch (err) { print("TOP", err.name, err.message); }
try { b = [((s > f(1)) ? (({x:1}) !== d) : (i ? arr[0] : 2)), [({p: "", q: undefined}), [true, NaN]]]; } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [null, f(1)]; })((void (g >> [1,2])))); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { print(String((f((({}) ? 0 : d)) == f("s"))), typeof ((({p: (arr[0] || undefined), q: [false, 0]}) && ({p: d, q: 2})))); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (var k in (~null)) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { print(g); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
