var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(a); } catch (err) { print("TOP", err.name, err.message); }
try { print(((s >>> (0 * ({}))) <= (+(Infinity << null)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: true, q: ({p: [0, 2], q: (2 >> arr[0])})})); } catch (err) { print("TOP", err.name, err.message); }
try { try { b = true; } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (([] + [(({}) >>> arr[0]), [false, ({x:1})]])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (([f(1), (-1 & f(1))] ? (void ({p: "s", q: o.x})) : (({p: ({x:1}), q: [1,2]}) ^ ([1,2] % "")))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((2 == ((~arr[0]) ? (true << []) : (true ? "s" : c)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(("s" >>> ({p: 0.5, q: (1 << [1,2])}))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { a = (({}) <= (typeof (arr.length != b))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [(0 > true), (b >= a)]; })(({p: [f(null), ({p: g, q: "s"})], q: (~[1,2])}))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; o.y = (0.5 - f((Infinity != g))); print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((+2) && a)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
