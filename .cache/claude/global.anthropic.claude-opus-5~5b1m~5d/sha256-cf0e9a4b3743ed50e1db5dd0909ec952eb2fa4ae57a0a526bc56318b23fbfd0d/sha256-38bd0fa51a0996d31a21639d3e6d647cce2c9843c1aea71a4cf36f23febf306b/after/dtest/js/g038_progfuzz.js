var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { i = 0; while (i < 2) { ++i; c = [((0 >> b) >>> (a > NaN)), (f(1) ? [f(1), ""] : s)]; } } catch (err) { print("TOP", err.name, err.message); }
try { d = (arr[0] ? (+[[], c]) : ((void []) << arr[0])); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((+f((void [1,2])))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((("" > (arr.length >> b)) + f(([1,2] && false)))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((!(d ? NaN : false)) + (("s" ? d : s) % [o.x, 2]))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(g); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((f(undefined) ? f(({x:1})) : (arr.length ? [1,2] : b)) <= (f(true) ? f(true) : ("" ? s : -1)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((~f((arr[0] | s)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(({x:1})); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(f(1)); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((Infinity ? b : 1) / (NaN ^ true)) == (void undefined))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(Infinity), typeof (null)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
