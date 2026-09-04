var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { arr.push(((~(-1 ? [1,2] : arr.length)) + ((-({})) ? (typeof [1,2]) : b))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((([b, 1] != (false || g)) / (1 << 1))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(f("")); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (({p: "s", q: ((void 0.5) + f(f(1)))})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { for (var k in true) print("k", k); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((+(typeof c))); } catch (err) { print("TOP", err.name, err.message); }
try { print(((({p: i, q: s}) >>> a) ? [(void [1,2]), arr.length] : (("s" <= 1) < 0))); } catch (err) { print("TOP", err.name, err.message); }
try { a = (void (f(a) ? (2 == true) : (1 > [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { print((i != 0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((+([i, c] & (f(1) ^ ({x:1}))))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { i = 0; while (i < 2) { ++i; try { switch (2) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({})); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
