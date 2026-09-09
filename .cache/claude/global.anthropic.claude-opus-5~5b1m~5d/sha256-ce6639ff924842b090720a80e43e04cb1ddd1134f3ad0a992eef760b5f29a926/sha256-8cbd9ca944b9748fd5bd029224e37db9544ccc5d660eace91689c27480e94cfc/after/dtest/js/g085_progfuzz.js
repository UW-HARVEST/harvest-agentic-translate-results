var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((((null ? s : i) === f([1,2])) == 0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { b = ([(f(1) === true), (void c)] % a); } catch (err) { print("TOP", err.name, err.message); }
try { print(2); } catch (err) { print("TOP", err.name, err.message); }
try { try { print([1,2]); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(undefined); } catch (err) { print("TOP", err.name, err.message); }
try { print(String([([0.5, o.x] ? 0 : (typeof null)), 0.5]), typeof (((({p: NaN, q: [1,2]}) | ([1,2] > -1)) ? (f(arr[0]) ? (0 != s) : [i, ({x:1})]) : (arr[0] < (false >>> b))))); } catch (err) { print("TOP", err.name, err.message); }
try { print([["", [[], [1,2]]], a]); } catch (err) { print("TOP", err.name, err.message); }
try { do { c = arr[0]; } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (var k in undefined) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { try { try { i = 0; while (i < 2) { ++i; print(({p: 2, q: [(0.5 >= g), (c ? [1,2] : i)]})); } } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((typeof s) ? (-"") : (NaN ? -1 : b)) && (({p: ({}), q: ""}) * (d << 2)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(NaN); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
