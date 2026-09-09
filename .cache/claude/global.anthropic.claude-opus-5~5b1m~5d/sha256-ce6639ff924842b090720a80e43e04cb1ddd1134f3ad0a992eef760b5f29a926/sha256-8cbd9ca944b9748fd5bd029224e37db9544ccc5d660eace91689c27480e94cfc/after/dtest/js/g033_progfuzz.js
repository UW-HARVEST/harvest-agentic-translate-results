var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(null); } catch (err) { print("TOP", err.name, err.message); }
try { b = ([NaN, (2 | NaN)] ? d : ({p: ("" != "s"), q: (null ? undefined : [])})); } catch (err) { print("TOP", err.name, err.message); }
try { c = (+f([undefined, ""])); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { try { arr.push((-(({p: true, q: c}) !== ([] <= NaN)))); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof false)); } catch (err) { print("TOP", err.name, err.message); }
try { print([(void (+-1)), f((0 != b))]); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(1); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((Infinity != 1) - ({p: true, q: f(1)})) && (-1 >= a))); } catch (err) { print("TOP", err.name, err.message); }
try { print(([(g >= o.x), arr.length] ^ (f(({})) ^ 2))); } catch (err) { print("TOP", err.name, err.message); }
try { if ([[(c / Infinity), f(1)], ({p: (typeof undefined), q: (a == b)})]) { print([(+[d, "s"]), (([] !== arr[0]) << (s === arr.length))]); } else { i = 0; while (i < 2) { ++i; arr.push((0.5 <= [1,2])); print(arr.length, arr.join(",")); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (void (d === c)); })(g)); } catch (err) { print("TOP", err.name, err.message); }
try { print((f(f(-1)) && ({p: f(1), q: (undefined ? "s" : g)}))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
