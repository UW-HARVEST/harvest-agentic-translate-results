var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { switch ((i ? a : (true * (o.x != 2)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { b = (typeof f(({p: [], q: undefined}))); } catch (err) { print("TOP", err.name, err.message); }
try { do { print((function(p) { return (typeof o.x); })((a & [["", o.x], (false != 2)]))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((((({x:1}) - [1,2]) ? (true ? a : []) : f(0)) > ((undefined || g) ? (a - arr.length) : (~d)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f(f([i, []]))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((f((a + undefined)) ? [0, f(({}))] : ([1,2] ? (({}) / 2) : (i || f(1))))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (1 < (c + b)); })(((void ({p: ({x:1}), q: o.x})) && 0.5))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ([({}), ({x:1})] ? [f(1), [1,2]] : f(o.x)); })(f(((arr.length + arr.length) >= ({}))))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (("s" === ("s" >= (Infinity !== a)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (a) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((((typeof NaN) == [g, ({})]) / ((~arr.length) ? (true - c) : f(true)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
