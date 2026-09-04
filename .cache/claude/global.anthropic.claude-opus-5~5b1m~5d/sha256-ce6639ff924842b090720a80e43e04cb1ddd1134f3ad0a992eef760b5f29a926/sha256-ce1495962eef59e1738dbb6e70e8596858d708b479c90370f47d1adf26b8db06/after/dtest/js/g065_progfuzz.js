var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { try { try { print((f((null ? o.x : null)) !== (f(NaN) ? s : ({p: [1,2], q: 0.5})))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = f(1); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f(g))); } catch (err) { print("TOP", err.name, err.message); }
try { c = ({p: (f(null) - 1), q: (({p: undefined, q: null}) | ({p: a, q: i}))}); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print([((+0.5) >= ({p: ({x:1}), q: ""})), [(({}) ? 0 : 0.5), null]]); } } catch (err) { print("TOP", err.name, err.message); }
try { b = ({x:1}); } catch (err) { print("TOP", err.name, err.message); }
try { print((~({}))); } catch (err) { print("TOP", err.name, err.message); }
try { c = ((f(false) ? ({p: b, q: arr.length}) : arr[0]) | -1); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(f(null)); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((c * ({})) ? (~2) : [false, -1]); })(((!-1) < ((o.x || c) || arr.length)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({p: f(1), q: (0 % [1,2])}); })((({p: (1 > c), q: f(arr[0])}) ? (0 ? [Infinity, c] : arr.length) : (arr.length * (!({})))))); } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
