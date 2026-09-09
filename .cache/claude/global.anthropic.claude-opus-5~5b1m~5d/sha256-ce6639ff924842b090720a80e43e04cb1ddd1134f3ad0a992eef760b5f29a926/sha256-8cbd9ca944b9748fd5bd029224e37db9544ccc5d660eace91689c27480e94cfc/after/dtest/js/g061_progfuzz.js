var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((({}) + [(({x:1}) != NaN), f(({x:1}))])); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(f((("" ? f(1) : o.x) >> (NaN != arr.length)))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((!(({p: arr[0], q: i}) + (({}) - [])))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(([(!s), [false, s]] | 0)); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = []; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { a = f(d); } catch (err) { print("TOP", err.name, err.message); }
try { print([((({}) ? o.x : arr[0]) ? [0.5, null] : ({p: c, q: 0.5})), (typeof ({p: g, q: i}))]); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print([(void [null, -1]), ({p: (f(1) / ({})), q: (b ^ s)})]); } } } } catch (err) { print("TOP", err.name, err.message); }
try { do { try { try { print((false == f((true || -1)))); } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { switch (((("s" < 1) ? f([1,2]) : d) ? f(1) : arr[0])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(({}))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
