var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = (((s == d) % 0) | (({}) ? (NaN ? arr.length : f(1)) : arr[0])); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((f(([1,2] <= "s")) + f(f("s")))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in arr[0]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (((2 ? f(1) : [1,2]) & (NaN >>> [])) + "s"); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { i = 0; while (i < 2) { ++i; print((~-1)); } } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ([(f(0.5) >> (true % 0.5)), ((!d) * (arr.length ? arr[0] : a))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ([([a, c] || (arr.length !== "s")), (({p: Infinity, q: ({})}) ? NaN : f(({x:1})))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if ([(-({p: 0.5, q: false})), ((s ? d : arr.length) ^ (!2))]) { print(f(f(g))); } else { try { i = 0; while (i < 2) { ++i; switch (a) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { try { print(f((-({p: c, q: 0.5})))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f([2, f(i)])); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = f(((NaN == 0) === (i ? s : a))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(({})); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
