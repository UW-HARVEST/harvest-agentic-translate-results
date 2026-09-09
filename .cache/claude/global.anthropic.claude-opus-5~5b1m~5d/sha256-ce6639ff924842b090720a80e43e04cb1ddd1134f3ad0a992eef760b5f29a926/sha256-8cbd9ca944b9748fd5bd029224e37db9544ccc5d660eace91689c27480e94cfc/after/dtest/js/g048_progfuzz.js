var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((-(f(d) ? ({p: o.x, q: s}) : (-1 & false)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(((typeof NaN) | ((b >>> f(1)) ? (NaN !== NaN) : (-true)))), typeof (({}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: (b & -1), q: (typeof f(false))})); } catch (err) { print("TOP", err.name, err.message); }
try { b = (f((Infinity >= b)) << (d ^ (void i))); } catch (err) { print("TOP", err.name, err.message); }
try { print((null === f(i))); } catch (err) { print("TOP", err.name, err.message); }
try { print((-o.x)); } catch (err) { print("TOP", err.name, err.message); }
try { print(c); } catch (err) { print("TOP", err.name, err.message); }
try { if ([]) { for (var k in ({p: [(--1), (null && g)], q: (d % ("s" == undefined))})) print("k", k); } else { switch (({p: [f(0), [c, g]], q: d})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { do { i = 0; while (i < 2) { ++i; print((function(p) { return null; })(true)); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { if ((~(i ? [({}), "s"] : (+1)))) { print(0.5); } else { try { print((+(f(f(1)) - [arr.length, c]))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f((undefined % 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { d = [[(({x:1}) ? b : o.x), [b, Infinity]], [1,2]]; } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
