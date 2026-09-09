var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((![f(NaN), g])); } catch (err) { print("TOP", err.name, err.message); }
try { print(2); } catch (err) { print("TOP", err.name, err.message); }
try { print(({})); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(-1); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { b = ([(+true), [NaN, 2]] >>> (({p: 2, q: arr.length}) === d)); } catch (err) { print("TOP", err.name, err.message); }
try { if ((({p: [undefined, s], q: (b * arr.length)}) <= (~2))) { for (var k in 0.5) print("k", k); } else { switch (g) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { a = a; } catch (err) { print("TOP", err.name, err.message); }
try { d = [[(arr[0] < []), [g, 0.5]], g]; } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { print(String((~null)), typeof (((void (({x:1}) ? o.x : true)) && [({p: null, q: 0}), ""]))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((f((d & c)) ? ({p: [d, 0.5], q: f(1)}) : ((undefined > undefined) - (typeof [])))); } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (+f(1)), q: ("s" ^ o.x)}) ? undefined : (({p: 0, q: arr[0]}) >= (b ? i : g)))); } catch (err) { print("TOP", err.name, err.message); }
try { a = [([f(1), c] <= ([] ? c : true)), ((b / 1) ? ([1,2] <= -1) : b)]; } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
