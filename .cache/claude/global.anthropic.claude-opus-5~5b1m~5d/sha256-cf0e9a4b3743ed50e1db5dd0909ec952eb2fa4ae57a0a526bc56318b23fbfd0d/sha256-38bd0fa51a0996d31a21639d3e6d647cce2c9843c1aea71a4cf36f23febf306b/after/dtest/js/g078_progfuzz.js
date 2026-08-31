var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = ({p: (void (+d)), q: ({p: i, q: (-1 ^ arr[0])})}); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(Infinity); } catch (err) { print("TOP", err.name, err.message); }
try { d = [(f(0) & (c & [1,2])), (~d)]; } catch (err) { print("TOP", err.name, err.message); }
try { try { try { print((function(p) { return false; })((({p: 2, q: (-s)}) ? ((-g) > (({}) ? NaN : c)) : arr.length))); } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { c = ((0 > (-Infinity)) ? (i == "s") : arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { a = f((f("") <= (Infinity ? false : g))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (var k in [null, (+(null ? "s" : 0))]) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { o.y = [[f(1), (+undefined)], (typeof 1)]; print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { print((((({}) & arr[0]) ? arr.length : (s >> g)) === [(NaN ? ({x:1}) : a), undefined])); } catch (err) { print("TOP", err.name, err.message); }
try { print((([arr[0], true] >= (undefined ? [] : 0)) && [(typeof i), (-1 ? "s" : true)])); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({p: s, q: ("" / "")}); })((typeof ((({}) ? "s" : 1) && -1)))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
