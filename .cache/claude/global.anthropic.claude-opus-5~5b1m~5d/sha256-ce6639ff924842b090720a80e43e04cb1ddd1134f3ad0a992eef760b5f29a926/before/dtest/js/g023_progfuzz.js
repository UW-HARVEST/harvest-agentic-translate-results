var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((!(f(1) !== [g, 0.5]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(d); } catch (err) { print("TOP", err.name, err.message); }
try { print((~((({}) ? NaN : arr[0]) << (true != [1,2])))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(({p: ((typeof ({})) & i), q: s})); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((+f(NaN))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; b = ((void -1) ? f((void [1,2])) : (({p: 0, q: c}) != (f(1) ? 1 : ({})))); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ((("" >>> "") || d) * f(([1,2] || Infinity))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({p: (!f(1)), q: [Infinity, g]}); })(arr.length)); } catch (err) { print("TOP", err.name, err.message); }
try { b = (-([d, s] / (-true))); } catch (err) { print("TOP", err.name, err.message); }
try { print((b > Infinity)); } catch (err) { print("TOP", err.name, err.message); }
try { print(([0.5, b] / ((o.x ^ true) > (!1)))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (d) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
