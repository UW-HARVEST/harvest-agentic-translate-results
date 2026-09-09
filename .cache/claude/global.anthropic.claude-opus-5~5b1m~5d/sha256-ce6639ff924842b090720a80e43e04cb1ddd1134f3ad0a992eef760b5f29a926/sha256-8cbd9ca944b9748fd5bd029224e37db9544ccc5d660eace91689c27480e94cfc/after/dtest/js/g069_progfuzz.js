var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print(-1); } } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { try { if ((~f(f(1)))) { print(false); } else { b = (({p: (1 <= i), q: (c ? "s" : false)}) ? false : -1); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((function(p) { return [arr.length, false]; })(f(arr[0]))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: (-({})), q: ((+c) != ({p: false, q: -1}))})); } catch (err) { print("TOP", err.name, err.message); }
try { d = [(~({p: undefined, q: ({x:1})})), f([o.x, true])]; } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: false, q: (!o.x)}) ? (f(a) >> b) : (f(undefined) && ({p: 1, q: 0.5})))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print((function(p) { return (!Infinity); })((i / [(arr.length > s), (0.5 ^ o.x)]))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f((-([] ? arr.length : f(1))))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (var k in (f(({p: 0, q: arr.length})) === ({}))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (((g ? b : arr.length) !== ({p: [1,2], q: 1})) + (f(undefined) >> [d, false]))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { b = (![1,2]); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
