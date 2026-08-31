var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { i = 0; while (i < 2) { ++i; a = [(f(0.5) - (b << 0.5)), ((({}) >= 0) * (arr[0] >= ({})))]; } } catch (err) { print("TOP", err.name, err.message); }
try { try { for (var k in ((g >= (c / "s")) & ({p: (Infinity - ""), q: a}))) print("k", k); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(([({p: i, q: ({x:1})}), (+-1)] / (([] >> d) ? [1,2] : f(a)))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f((typeof (-arr[0])))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return null; })((((0.5 ? b : 0) <= f(false)) ? (f(0) != ({p: true, q: NaN})) : ((o.x || b) !== (arr[0] || g))))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (typeof arr.length); })(f(([o.x, 2] ? [] : (f(1) !== ""))))); } catch (err) { print("TOP", err.name, err.message); }
try { d = ((NaN <= (1 % o.x)) >>> ({p: g, q: (d <= 2)})); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f([[o.x, b], (false >= [1,2])])); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { if (((["s", NaN] << (typeof true)) ^ f([1,2]))) { print((0.5 === arr[0])); } else { do { for (i = 0; i < 3; ++i) { try { b = (~(0.5 / (0 << undefined))); } catch (e) { print("c", e.name); } finally { print("fin"); } } } while (false); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((+f([1, [1,2]]))) { o.y = ({}); print(JSON.stringify(o)); } else { i = 0; while (i < 2) { ++i; switch (a) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { try { if (((f(d) ^ (false && arr[0])) ^ (({p: [], q: g}) ? (arr[0] - null) : (void d)))) { print(String(((+(g << 2)) * (f(arr.length) ? -1 : ({})))), typeof ([[(arr.length ? 2 : []), false], [("s" ? -1 : a), (0.5 >> false)]])); } else { print(0.5); } } catch (e) { print("caught", e.name); } } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { a = [({p: "", q: (o.x <= true)}), (!({x:1}))]; } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
