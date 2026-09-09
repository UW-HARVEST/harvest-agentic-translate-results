var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(([f(Infinity), 2] / ((arr[0] + "") && (({}) !== null)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((o.x || (1 ? Infinity : (a ^ 0.5)))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print(String(({})), typeof (undefined)); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(-1); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [(-(~NaN)), [(+NaN), undefined]]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { d = (((1 >> Infinity) << a) >> (([1,2] <= -1) + d)); } catch (err) { print("TOP", err.name, err.message); }
try { switch (false) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [[(NaN - []), ({p: [], q: f(1)})], [f(arr[0]), ([1,2] ? [] : o.x)]]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((void 0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print(arr.length); } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { if (((void -1) | ([0.5, a] % (a + false)))) { arr.push([d, ((arr[0] << "s") <= (c | ""))]); print(arr.length, arr.join(",")); } else { arr.push((((1 ? s : 2) * (Infinity < arr.length)) >= (arr[0] ? ({}) : [1,2]))); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
