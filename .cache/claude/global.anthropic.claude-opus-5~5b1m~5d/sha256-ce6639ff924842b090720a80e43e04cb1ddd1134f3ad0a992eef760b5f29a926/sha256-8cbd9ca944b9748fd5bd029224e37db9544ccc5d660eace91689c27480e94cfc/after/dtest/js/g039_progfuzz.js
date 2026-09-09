var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { arr.push(({p: (!(o.x <= -1)), q: ({p: (undefined ? a : NaN), q: (b < null)})})); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(o.x); } catch (err) { print("TOP", err.name, err.message); }
try { print(((f(arr.length) % (b + i)) != (i ? (0 != []) : (b / i)))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; if (f(true)) { for (i = 0; i < 3; ++i) { print((f(arr.length) ? ((false - f(1)) && ("" - "s")) : a)); } } else { for (i = 0; i < 3; ++i) { print([[(+a), (d === d)], ((!undefined) >> (-1 <= NaN))]); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(String((f([null, true]) ? (f(a) ? (false ? "" : 1) : (0.5 | d)) : [2, (1 >> arr[0])])), typeof (arr[0])); } catch (err) { print("TOP", err.name, err.message); }
try { do { for (i = 0; i < 3; ++i) { if (([] + ((g !== 0) - ([1,2] * -1)))) { d = (({p: (false >= undefined), q: (o.x != b)}) ? [(typeof Infinity), NaN] : [1,2]); } else { print((function(p) { return ""; })((f((s ? d : [1,2])) >= (1 ? f(1) : f([]))))); } } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(({x:1})); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { print(String(({p: null, q: ((-false) !== 2)})), typeof ([(s ? ["s", 2] : arr.length), (1 | ({p: -1, q: 0.5}))])); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print([null, (false == (-1 | b))]); } catch (err) { print("TOP", err.name, err.message); }
try { print(((void 2) + (({p: i, q: ({})}) / b))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((function(p) { return d; })((((arr.length >= "s") === [({}), i]) / ((a ? g : arr[0]) > undefined)))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((({x:1}) < (({x:1}) & ({p: c, q: ""})))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
