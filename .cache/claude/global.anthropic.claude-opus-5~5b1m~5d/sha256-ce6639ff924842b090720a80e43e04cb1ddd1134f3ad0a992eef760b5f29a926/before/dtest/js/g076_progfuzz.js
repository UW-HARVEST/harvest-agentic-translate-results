var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { do { print(o.x); } while (false); } } catch (err) { print("TOP", err.name, err.message); }
try { d = false; } catch (err) { print("TOP", err.name, err.message); }
try { print(arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { print([]); } catch (err) { print("TOP", err.name, err.message); }
try { if (0) { for (i = 0; i < 3; ++i) { for (var k in ((f(true) ? s : ({p: undefined, q: 0.5})) | (("" | f(1)) / Infinity))) print("k", k); } } else { try { arr.push((({p: s, q: (c > ({x:1}))}) % ([s, 1] << (g >= 2)))); print(arr.length, arr.join(",")); } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = (f("") ? (({p: o.x, q: c}) !== (0.5 <= arr[0])) : ({p: (s >= f(1)), q: arr.length})); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = b; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((+(-1 || [])) % ((-1 ? o.x : "s") >>> (g ? b : 0)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(((+("s" & NaN)) !== f([1, d]))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof [(typeof arr.length), arr.length])); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { print([b, (~("" || d))]); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(1)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
