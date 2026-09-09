var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { do { print((true ? ({p: (false || s), q: (0.5 / 0.5)}) : ([false, arr.length] ^ (true <= 0.5)))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { d = arr.length; } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (({p: s, q: -1}) << (c == (![])))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(((c > f(-1)) === (("s" ? a : b) >>> (undefined << o.x)))); } catch (err) { print("TOP", err.name, err.message); }
try { print([(~f([])), (0 > [a, -1])]); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return f((NaN > undefined)); })((({p: (s - [1,2]), q: (2 ? ({}) : 2)}) ? ((2 >>> s) | [a, s]) : (true >= (false ? "" : a))))); } catch (err) { print("TOP", err.name, err.message); }
try { d = (([1,2] == -1) << ({p: (NaN ? a : 0), q: (arr.length + "s")})); } catch (err) { print("TOP", err.name, err.message); }
try { try { arr.push(((b | (s >> 0.5)) % ((0.5 / ({})) ? ("" > "s") : (undefined != a)))); print(arr.length, arr.join(",")); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((void "s") + (a ^ false)) && ([[1,2], ({})] !== d))); } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof ((({x:1}) < i) < ("" < NaN)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(""); } catch (err) { print("TOP", err.name, err.message); }
try { print((+({p: ["", [1,2]], q: (d ? s : f(1))}))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
