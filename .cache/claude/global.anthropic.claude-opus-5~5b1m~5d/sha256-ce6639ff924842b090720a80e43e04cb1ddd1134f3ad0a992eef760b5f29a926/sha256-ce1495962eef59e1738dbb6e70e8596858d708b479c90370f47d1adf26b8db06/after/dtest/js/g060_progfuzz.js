var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print([1,2]); } catch (err) { print("TOP", err.name, err.message); }
try { print((void (([] << 1) >>> -1))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { arr.push(f([[undefined, s], f(NaN)])); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { if (((i + 0) >> (({p: NaN, q: s}) - NaN))) { o.y = ({p: [], q: []}); print(JSON.stringify(o)); } else { print(String(f(undefined)), typeof (([] >>> ({x:1})))); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((((undefined | true) >> [f(1), i]) | (g < (false || -1)))) { if (([({p: a, q: NaN}), f(b)] % (NaN <= (b > "")))) { for (i = 0; i < 3; ++i) { print(c); } } else { if (f(((1 % Infinity) & b))) { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print(f(({}))); } } } else { for (i = 0; i < 3; ++i) { print(({x:1})); } } } } else { for (i = 0; i < 3; ++i) { b = f(((-arr.length) || (~0.5))); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (Infinity >>> i); })("s")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((void (("s" || g) ? ([1,2] / "s") : (-1 ? 2 : ({x:1}))))); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ((-(({}) - c)) - (2 == [1,2])); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(arr.length); } catch (err) { print("TOP", err.name, err.message); }
try { d = ((+b) && ({p: (f(1) || true), q: f(1)})); } catch (err) { print("TOP", err.name, err.message); }
try { do { arr.push(({p: s, q: ((+f(1)) ? f(arr[0]) : (f(1) % g))})); print(arr.length, arr.join(",")); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(((-1 && (({x:1}) ^ undefined)) ? ([o.x, g] && d) : -1)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
