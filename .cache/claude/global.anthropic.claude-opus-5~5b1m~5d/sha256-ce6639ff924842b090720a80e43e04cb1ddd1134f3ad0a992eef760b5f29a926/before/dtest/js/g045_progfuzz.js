var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(({p: 2, q: (f(false) ? [[], f(1)] : c)})); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(({p: ({p: (o.x === undefined), q: [arr.length, o.x]}), q: [g, (b < ({x:1}))]})); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(g); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((~undefined)); } catch (err) { print("TOP", err.name, err.message); }
try { print(arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { if ((-(false ? a : f(undefined)))) { for (var k in 1) print("k", k); } else { for (i = 0; i < 3; ++i) { print((a & ("" + (b & true)))); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(NaN); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((null >>> 0) <= (f(1) && b)); })(({}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(((a ? (!({x:1})) : [1,2]) << [(c ? arr[0] : true), (1 | 0)])), typeof (0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (((1 >>> 0.5) - o.x) ? ((0.5 | 0.5) << b) : [[a, NaN], arr[0]]); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((((NaN - d) ? ([] ? "" : undefined) : (i >>> false)) ? ({p: (arr.length & false), q: ([] && d)}) : null)); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(((f(g) / ("" >>> 2)) <= ({x:1}))), typeof ((((c || false) % g) ? ((({}) & g) ? (1 >> Infinity) : (+a)) : false))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
