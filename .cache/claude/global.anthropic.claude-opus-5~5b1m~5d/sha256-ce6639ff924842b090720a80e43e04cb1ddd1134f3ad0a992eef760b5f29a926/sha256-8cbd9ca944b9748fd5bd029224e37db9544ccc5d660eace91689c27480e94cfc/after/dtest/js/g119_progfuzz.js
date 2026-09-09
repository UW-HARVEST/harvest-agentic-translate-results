var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { switch (false) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(1); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ((d >= Infinity) - ((f(1) < c) > (true < ""))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (i || 2), q: -1}) - "s")); } catch (err) { print("TOP", err.name, err.message); }
try { do { switch ([({x:1}), ({p: (-({x:1})), q: (null || d)})]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { d = ((i > (c && d)) ^ Infinity); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((c & []) << ([1,2] ^ f(1))); })(((-([] <= [])) ? (("s" ? ({}) : -1) % (o.x <= true)) : (-"")))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print([a, d]); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { try { try { print(NaN); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print((false >= f(1))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { if (((("" | 0) != ([] != f(1))) >= g)) { for (i = 0; i < 3; ++i) { for (var k in (([null, b] && f(o.x)) + [(typeof true), (arr[0] ? arr[0] : o.x)])) print("k", k); } } else { for (i = 0; i < 3; ++i) { try { for (var k in arr[0]) print("k", k); } catch (e) { print("caught", e.name); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(undefined); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
