var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(["", (f(arr[0]) !== [d, 0])]); } catch (err) { print("TOP", err.name, err.message); }
try { print(i); } catch (err) { print("TOP", err.name, err.message); }
try { try { arr.push((((false ? 2 : d) && (true < s)) !== (({p: arr[0], q: true}) * (null && "s")))); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({x:1})); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((d ? a : ((0.5 <= arr.length) >> (s ^ f(1))))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((f(2) >>> f((c === ({}))))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (1) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print([([1,2] && g), (~({p: a, q: d}))]); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(Infinity); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (b ? ((0 ? d : undefined) ? (+b) : (0 < Infinity)) : arr[0]); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { c = ((void f(1)) === (f([1,2]) >> i)); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { print((~(f(1) >>> (g - arr.length)))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
