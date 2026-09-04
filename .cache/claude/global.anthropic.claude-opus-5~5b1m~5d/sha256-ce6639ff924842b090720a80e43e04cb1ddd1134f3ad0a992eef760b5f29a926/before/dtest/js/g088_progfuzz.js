var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { switch (f(f((({x:1}) - null)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(null); } catch (err) { print("TOP", err.name, err.message); }
try { if ((({p: d, q: [arr.length, true]}) >= (typeof [c, i]))) { for (var k in (g / f(""))) print("k", k); } else { i = 0; while (i < 2) { ++i; o.y = [c, (-(o.x ? arr[0] : false))]; print(JSON.stringify(o)); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = arr[0]; } catch (err) { print("TOP", err.name, err.message); }
try { print((+(-1 ? 1 : (true | undefined)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (d <= (+0.5))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(((typeof arr[0]) ? ({x:1}) : (void ({})))); } catch (err) { print("TOP", err.name, err.message); }
try { if (d) { print((function(p) { return (arr[0] % false); })(((d - f(false)) > (!({p: ({x:1}), q: 0}))))); } else { for (i = 0; i < 3; ++i) { print(([(false % -1), (void 0)] >> ([1,2] < (!"s")))); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(""); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = f(((void arr.length) % ({x:1}))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(d); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([({p: (s ? "s" : i), q: 0}), (s != (!g))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
