var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { a = -1; } catch (err) { print("TOP", err.name, err.message); }
try { switch ((({p: f(1), q: s}) ? ((-1 != "s") ? [({x:1}), ({})] : (+1)) : b)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (([] || ({p: b, q: (({x:1}) || b)}))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { try { o.y = (void (Infinity != arr.length)); print(JSON.stringify(o)); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof (false ? (i == ({x:1})) : (NaN ? false : false)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { print((!(+(-arr.length)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((({}) >= -1) << (arr.length == 0)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { o.y = "s"; print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { if (((-(true > c)) ? [(Infinity ? f(1) : d), [o.x, arr.length]] : (false + (+i)))) { o.y = ((-(+null)) == ({p: (void o.x), q: b})); print(JSON.stringify(o)); } else { a = 0; } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print([]); } } catch (err) { print("TOP", err.name, err.message); }
try { if (arr[0]) { try { c = (({p: [o.x, -1], q: (({}) * 0)}) - ((null ? 0.5 : [1,2]) << g)); } catch (e) { print("caught", e.name); } } else { switch ([1,2]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
