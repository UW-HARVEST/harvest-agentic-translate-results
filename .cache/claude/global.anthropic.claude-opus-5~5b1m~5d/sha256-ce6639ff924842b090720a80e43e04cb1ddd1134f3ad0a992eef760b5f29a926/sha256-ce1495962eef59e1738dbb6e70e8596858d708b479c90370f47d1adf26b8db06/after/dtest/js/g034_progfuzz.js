var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { switch (({p: f([1,2]), q: ({})})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [({p: (false % 1), q: (Infinity >> -1)}), ({p: ({p: true, q: -1}), q: ({p: NaN, q: ({x:1})})})]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((0 !== (f(false) >> (g ? arr[0] : -1)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; b = (({p: i, q: (arr[0] < [])}) < [-1, 0]); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ({p: [g, ({x:1})], q: (arr.length !== i)}), q: (a | ({p: "", q: [1,2]}))})); } catch (err) { print("TOP", err.name, err.message); }
try { do { a = [false, 0]; } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(((2 <= (typeof ({}))) >= NaN)); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(f(({p: -1, q: b})))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(String((!undefined)), typeof (((o.x & g) || (typeof f(({x:1})))))); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(([1,2] | d)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((null << arr.length)); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((0.5 ? 1 : (("s" ? o.x : ({x:1})) ? i : (true * null)))) { print((f([d, 0]) ? f(i) : (([] || 2) ? (false ? false : a) : c))); } else { for (var k in (((Infinity | true) && 2) === (i || (~[1,2])))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
