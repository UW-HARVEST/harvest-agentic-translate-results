var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = (((g ? 2 : arr[0]) ? [undefined, 0.5] : undefined) & f((~NaN))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print([[f(NaN), ([] ? a : 0)], ([-1, 1] !== (g !== [1,2]))]); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; arr.push(([(!d), (f(1) <= "")] == ("" !== -1))); print(arr.length, arr.join(",")); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(((("s" / []) >>> ({p: "s", q: a})) ? [] : ((1 + -1) ? d : [undefined, -1]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((((0 <= Infinity) | true) ? ({p: "s", q: f(false)}) : o.x)), typeof (([false, f(true)] & (({p: f(1), q: b}) ? (!"s") : f([]))))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; arr.push(f((~Infinity))); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [b, (f(-1) & 2)]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(b); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: 0.5, q: s})); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(a); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { arr.push(({x:1})); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { c = f([(0 + ({})), (true === [])]); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
