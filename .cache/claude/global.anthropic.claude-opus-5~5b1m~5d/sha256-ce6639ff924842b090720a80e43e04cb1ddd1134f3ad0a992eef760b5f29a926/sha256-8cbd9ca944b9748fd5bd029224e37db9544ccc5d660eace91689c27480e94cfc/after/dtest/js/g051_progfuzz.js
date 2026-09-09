var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { print((void (+(typeof c)))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f(({p: 2, q: g})))); } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { do { print(([1,2] !== ({p: (NaN < -1), q: [a, false]}))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((o.x & (+(arr[0] == 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(true), typeof (({p: (1 < false), q: s}))); } catch (err) { print("TOP", err.name, err.message); }
try { b = true; } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(String(f(({p: [1,2], q: [1, f(1)]}))), typeof ((~(b >> 0)))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(g); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f([])); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(((({p: null, q: []}) + f(0)) && ((arr.length ? i : s) == (NaN != undefined)))), typeof ((((~b) * false) ? d : undefined))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f((b == s)))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
