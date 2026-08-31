var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { print(((false <= (-({x:1}))) || f(1))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in i) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if (((({p: "s", q: 0.5}) * (-1 >> i)) == d)) { try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print(true); } } } catch (e) { print("caught", e.name); } } else { do { b = ((false - [({x:1}), b]) - d); } while (false); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { d = Infinity; } } catch (err) { print("TOP", err.name, err.message); }
try { print((null ? (arr[0] * false) : ([arr.length, i] ? (i ? ({}) : arr[0]) : ({p: NaN, q: c})))); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [(-1 / b), (void f(-1))]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(Infinity); } catch (err) { print("TOP", err.name, err.message); }
try { if (f(f(2))) { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; c = d; } } } else { print(undefined); } } catch (err) { print("TOP", err.name, err.message); }
try { if (f((({p: s, q: null}) && ({p: d, q: arr.length})))) { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; c = (-((0.5 ^ -1) >> f("s"))); } } } } else { switch ((!(f(1) <= f(c)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(2); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print([([] ^ (null === null)), 0.5]); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((+((f(1) > -1) | (~Infinity)))), typeof ((s ? [] : NaN))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
