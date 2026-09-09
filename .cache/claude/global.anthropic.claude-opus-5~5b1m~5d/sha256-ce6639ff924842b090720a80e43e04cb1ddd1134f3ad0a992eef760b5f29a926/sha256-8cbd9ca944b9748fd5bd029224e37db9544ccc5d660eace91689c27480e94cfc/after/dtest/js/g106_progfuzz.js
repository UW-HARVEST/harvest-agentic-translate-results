var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((function(p) { return (i >> (null || arr[0])); })(0)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((function(p) { return (null ? arr[0] : [1,2]); })(("" > [d, (null || 0)]))); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print([(0.5 << (a > 2)), (~(2 ? g : f(1)))]); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = f(((true * i) ? ({p: 2, q: "s"}) : f(NaN))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (({p: c, q: 0}) ? (-1 + [1,2]) : (d ? null : -1)); })(d)); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (0.5 >= ([g, arr[0]] ? (null / 1) : [s, Infinity]))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print((f((void d)) / b)); } catch (err) { print("TOP", err.name, err.message); }
try { print([a, b]); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((((void ({})) * (b * null)) - i)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { do { try { print(2); } catch (e) { print("caught", e.name); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(((({p: -1, q: g}) ? (f(1) | arr.length) : ("s" - arr[0])) << [(d >> 2), 1])); } catch (err) { print("TOP", err.name, err.message); }
try { print(f([({p: 1, q: a}), (({}) !== "s")])); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
