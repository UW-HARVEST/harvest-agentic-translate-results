var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { switch ((g !== s)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String((null >> o.x)), typeof ((({}) > (typeof (false === -1))))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(({p: ((1 ? b : "s") !== f(arr.length)), q: (0 - f(o.x))})); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { if (undefined) { try { arr.push(null); print(arr.length, arr.join(",")); } catch (e) { print("c", e.name); } finally { print("fin"); } } else { print(2); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (void (({p: c, q: false}) == (g && [1,2])))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { c = (~f(g)); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return undefined; })((+[(o.x !== true), [o.x, [1,2]]]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(NaN); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { b = 1; } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((null & ("s" ? arr[0] : a)) ? ({p: 0, q: ({p: a, q: f(1)})}) : (({p: o.x, q: false}) ? ({p: ({x:1}), q: 0.5}) : (true ? NaN : "")))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { if (([1,2] != ((0 & d) / (2 === true)))) { o.y = ((0.5 ^ [c, 0.5]) < undefined); print(JSON.stringify(o)); } else { arr.push([((-undefined) > (-1 / b)), [({p: 0.5, q: undefined}), (true !== undefined)]]); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((!({p: "", q: (d && i)}))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
