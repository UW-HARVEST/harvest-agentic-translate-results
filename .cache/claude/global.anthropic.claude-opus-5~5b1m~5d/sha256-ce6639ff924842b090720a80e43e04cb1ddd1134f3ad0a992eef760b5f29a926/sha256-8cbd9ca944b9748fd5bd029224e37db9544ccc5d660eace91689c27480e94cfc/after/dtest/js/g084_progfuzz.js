var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { arr.push([({x:1}), ((o.x ? a : true) >>> (!0.5))]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { if ([(({}) <= ({p: c, q: true})), 1]) { arr.push(a); print(arr.length, arr.join(",")); } else { print(({})); } } catch (err) { print("TOP", err.name, err.message); }
try { print((!({p: 0, q: (+i)}))); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (var k in ({p: (("s" % s) ^ (b - arr.length)), q: -1})) print("k", k); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((d ? 2 : undefined) ? (2 != g) : (!undefined)) === ([true, []] ? ({p: b, q: Infinity}) : (f(1) <= 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((arr.length ^ ([i, 0.5] - (s ^ "")))); } catch (err) { print("TOP", err.name, err.message); }
try { try { a = ((f(arr.length) == i) === g); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(([b, ""] ? (typeof (typeof arr[0])) : [(false ? g : g), (b ^ null)])); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(1)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String(true), typeof ([(undefined ? ({p: o.x, q: 1}) : s), NaN])); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (((b ? true : b) ? ([] ? 0.5 : NaN) : "") || ((null << []) % ({p: s, q: false})))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(arr.length); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
