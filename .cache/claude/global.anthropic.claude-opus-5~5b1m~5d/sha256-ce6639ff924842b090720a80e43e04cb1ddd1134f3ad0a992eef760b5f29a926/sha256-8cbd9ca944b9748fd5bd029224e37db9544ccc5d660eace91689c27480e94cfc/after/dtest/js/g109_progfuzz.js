var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((null << (0.5 >>> null))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { print((Infinity == f((+Infinity)))); } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((true & g)); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f(1)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: "s", q: f(0.5)})); } catch (err) { print("TOP", err.name, err.message); }
try { c = (i >= (({p: d, q: ({x:1})}) * (arr[0] ? false : o.x))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; b = ((i < arr.length) | [(c & null), (0 ? true : 1)]); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(((d ? (0.5 ? 1 : arr.length) : (undefined <= "")) ? Infinity : (({p: "s", q: []}) >>> (i ? "s" : arr[0])))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String([(f(1) ? ({p: s, q: d}) : (!d)), [(d * o.x), 1]]), typeof ([(0.5 <= f(1)), undefined])); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (void false)) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if (((f(a) && (d >>> arr.length)) % ((s & c) / undefined))) { b = null; } else { d = false; } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({p: o.x, q: ({p: ({}), q: 0.5})}); })(((g ? 0.5 : (~o.x)) ? ({p: ({x:1}), q: ""}) : (f(0) != (arr[0] ^ c))))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
