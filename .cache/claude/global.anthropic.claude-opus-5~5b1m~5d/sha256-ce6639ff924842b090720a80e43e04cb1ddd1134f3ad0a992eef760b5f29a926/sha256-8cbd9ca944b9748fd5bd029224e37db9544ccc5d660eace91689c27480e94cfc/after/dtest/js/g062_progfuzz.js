var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((!f(f(a)))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((((arr[0] !== -1) == [false, -1]) <= (c !== ["", ({})]))); } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; try { print(-1); } catch (e) { print("caught", e.name); } } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(-1); } catch (err) { print("TOP", err.name, err.message); }
try { a = (((arr.length & []) ^ (1 << s)) >= ([0, []] == (2 ? i : f(1)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((0.5 << ((false && b) !== b))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({x:1}); })([(1 === arr.length), ((a >= "s") | (void s))])); } catch (err) { print("TOP", err.name, err.message); }
try { print(undefined); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { switch (((typeof f(g)) <= [(null ? "" : [1,2]), (-1 <= 1)])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { do { try { print(f(1)); } catch (e) { print("caught", e.name); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { switch (o.x) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { switch (o.x) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
