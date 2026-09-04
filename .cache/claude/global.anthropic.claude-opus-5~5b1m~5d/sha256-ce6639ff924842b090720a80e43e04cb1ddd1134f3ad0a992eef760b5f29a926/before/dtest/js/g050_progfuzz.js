var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if ([((({x:1}) || ({x:1})) >= (a ? true : true)), ((!undefined) * (Infinity <= 1))]) { b = (c == []); } else { a = (typeof f((1 <= i))); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(((({x:1}) ? a : ("" ? d : f(1))) ? (!("s" < [1,2])) : ([1,2] * (~true)))); } } catch (err) { print("TOP", err.name, err.message); }
try { c = arr[0]; } catch (err) { print("TOP", err.name, err.message); }
try { print(((({p: undefined, q: b}) ? (1 == Infinity) : (arr[0] >>> true)) | ((+b) - (true > -1)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print([2, (-[b, arr.length])]); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((f(1) ? g : [1,2]) - [2, null]) ? ((({x:1}) ? o.x : arr.length) ? (-1 / i) : []) : (void (!"")))); } catch (err) { print("TOP", err.name, err.message); }
try { print(""); } catch (err) { print("TOP", err.name, err.message); }
try { d = arr.length; } catch (err) { print("TOP", err.name, err.message); }
try { print((f((s >= true)) ^ (f(true) >= arr[0]))); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((+((s ? d : c) ^ -1))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(([1,2] ^ -1)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if (d) { print(({p: [({x:1}), ({p: arr.length, q: [1,2]})], q: ({p: c, q: s})})); } else { do { for (i = 0; i < 3; ++i) { c = (((({x:1}) && NaN) && ("s" ^ 0.5)) >>> ([undefined, -1] ? (1 <= 0.5) : (i < arr.length))); } } while (false); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
