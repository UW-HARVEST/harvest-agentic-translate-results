var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; for (var k in ((-(({}) < b)) | ((a !== o.x) + (2 ? [] : "")))) print("k", k); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((!([a, 1] > (arr[0] ? a : 2)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((Infinity != o.x)), typeof (b)); } catch (err) { print("TOP", err.name, err.message); }
try { print((((Infinity ? 0 : s) >> NaN) < ([true, s] % null))); } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if (d) { i = 0; while (i < 2) { ++i; print(({p: undefined, q: ({p: (typeof false), q: (void ({x:1}))})})); } } else { print(f(((s == d) >> (b < ({x:1}))))); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push("s"); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((void (-"")) <= f((-1 ^ Infinity)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { a = Infinity; } catch (err) { print("TOP", err.name, err.message); }
try { print(f((typeof (+0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { switch (((+({p: "s", q: Infinity})) ? ((arr.length != "") << (({x:1}) * a)) : ((c && o.x) >> f(-1)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { switch ([1,2]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
