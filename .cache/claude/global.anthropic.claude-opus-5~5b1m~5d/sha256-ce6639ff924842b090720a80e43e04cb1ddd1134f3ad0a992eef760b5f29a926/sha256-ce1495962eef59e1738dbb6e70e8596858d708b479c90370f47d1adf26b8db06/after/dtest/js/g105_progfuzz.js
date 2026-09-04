var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(String(f(((-1 * Infinity) & g))), typeof (arr[0])); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { do { print(((undefined > (g ? 2 : 2)) > 1)); } while (false); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((([0.5, s] > ({p: arr[0], q: ({})})) * (({p: undefined, q: ({x:1})}) % f(f(1))))); } catch (err) { print("TOP", err.name, err.message); }
try { print((((i === NaN) >= (null | [1,2])) ? [[[], ({x:1})], (--1)] : [f(i), (2 << undefined)])); } catch (err) { print("TOP", err.name, err.message); }
try { c = ((("" && 2) !== (~[])) ? Infinity : (({p: "", q: a}) ? (arr[0] ? ({x:1}) : "") : b)); } catch (err) { print("TOP", err.name, err.message); }
try { try { d = g; } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((f((typeof "")) > ((({x:1}) ? undefined : arr.length) ? d : (NaN === "")))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ({p: (b || (c ? true : a)), q: (~(b ? 0 : undefined))})) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((({p: ({p: g, q: NaN}), q: arr[0]}) >>> [(0.5 > a), f(i)])); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (["s", ({p: ({}), q: (null ? 2 : b)})]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (var k in (void (i <= ({p: undefined, q: 0.5})))) print("k", k); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print([[false, f(1)], (d != (~false))]); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
