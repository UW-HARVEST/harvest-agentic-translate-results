var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(String(([[null, 2], (arr.length ^ ({x:1}))] !== ({p: (g == d), q: (0 - ({}))}))), typeof ((false >> [f(g), f(arr.length)]))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (({p: "s", q: (a >> undefined)}) ? (b && g) : f(undefined))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { switch (((+f(null)) >= (~[[], b]))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return []; })((f(false) ? f((o.x ? arr.length : 0)) : [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { c = (((s > "s") % (-true)) ? ((i * c) % (!true)) : (f(0) >> (g - arr.length))); } catch (err) { print("TOP", err.name, err.message); }
try { do { o.y = (-[1, f(false)]); print(JSON.stringify(o)); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((-s)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { switch ([]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { try { print(Infinity); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(null); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((!g) <= (void [])); })(((({p: ({}), q: true}) ? undefined : f(({}))) % ({p: arr[0], q: a})))); } catch (err) { print("TOP", err.name, err.message); }
try { print(undefined); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
