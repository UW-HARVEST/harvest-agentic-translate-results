var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { print((~"s")); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String((({p: (s !== b), q: arr.length}) !== (f(g) === [i, NaN]))), typeof (({p: ((({x:1}) >> -1) >= b), q: o.x}))); } catch (err) { print("TOP", err.name, err.message); }
try { b = undefined; } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(f(d)); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({x:1})); } catch (err) { print("TOP", err.name, err.message); }
try { switch (([2, [1,2]] - [null, (2 + "s")])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { switch (([1,2] != ((void 0) ? (s != 2) : ({p: "s", q: true})))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { switch ([(-1 + (false != 1)), ((i ? false : "s") << o.x)]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((("" << 0) ^ (+Infinity)) == [(o.x - false), (typeof NaN)])); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { switch (false) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((null === null) ? Infinity : true); })((0 ^ ((true | c) % (undefined != arr.length))))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
