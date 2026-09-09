var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(([(g === []), ("s" ? d : null)] > [(i ? null : Infinity), -1])); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (var k in NaN) print("k", k); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (false) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; d = (({p: (o.x ? arr.length : ({})), q: (void a)}) < [f(0), f("")]); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (arr[0] > (g <= 2)); })([[(f(1) * b), null], ({})])); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [s, [(arr.length > true), ({p: NaN, q: f(1)})]]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((f([arr[0], false]) != (i ? 2 : (o.x << "")))); } catch (err) { print("TOP", err.name, err.message); }
try { d = (~false); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; b = (({x:1}) == ((({x:1}) >= null) ^ false)); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f(d))); } catch (err) { print("TOP", err.name, err.message); }
try { print(((({p: 0, q: 0.5}) * 0.5) | (+1))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ({p: ((-1 % 0.5) - (a / "s")), q: (({p: g, q: 2}) < (g ? true : []))})) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
