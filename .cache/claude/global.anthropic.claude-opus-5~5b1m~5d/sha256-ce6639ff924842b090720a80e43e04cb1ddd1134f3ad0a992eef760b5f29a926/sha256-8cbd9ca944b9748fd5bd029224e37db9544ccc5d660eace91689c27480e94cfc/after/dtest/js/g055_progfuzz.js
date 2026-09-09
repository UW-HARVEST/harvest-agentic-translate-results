var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { d = [f(arr.length), ({})]; } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(null); } } catch (err) { print("TOP", err.name, err.message); }
try { b = arr.length; } catch (err) { print("TOP", err.name, err.message); }
try { print((((!Infinity) % s) & ({p: (0 > arr[0]), q: (undefined < c)}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof ((Infinity + arr[0]) < (2 / d)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ((1 > 0.5) & (NaN & 2)), q: (d > 2)})); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return arr.length; })(f([([1,2] | 2), f(a)]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(g); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((((false & d) / (s <= false)) > (+(arr.length * s)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (s ? ("s" < 0.5) : f((c * NaN)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print((void ({x:1}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return -1; })(((!(NaN - ({x:1}))) ? (!"s") : [2, (g === 0.5)]))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
