var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((+"s")); } catch (err) { print("TOP", err.name, err.message); }
try { d = (void f((-1 << 1))); } catch (err) { print("TOP", err.name, err.message); }
try { print([(f(1) == (0 << 2)), ((typeof o.x) ? (c != "s") : ([] && d))]); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return [f(1), (a ? -1 : d)]; })(f((+({p: 1, q: f(1)}))))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((f((+d)) === d)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((-[(arr[0] || false), (o.x >= "s")])) { print(((false << (-1 | "s")) ? [(0.5 <= "s"), (arr[0] ? arr.length : Infinity)] : (f(1) ? 2 : (!d)))); } else { i = 0; while (i < 2) { ++i; print(f(-1)); } } } catch (err) { print("TOP", err.name, err.message); }
try { try { arr.push(({p: arr.length, q: ({p: (o.x ? 1 : arr.length), q: (1 ? ({}) : undefined)})})); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ([[], d] > f(Infinity)), q: 2})); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [(f(d) === (~Infinity)), ((s ? "s" : []) >= 2)]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print([((1 !== NaN) ? (arr[0] + 0.5) : f(0.5)), ([] ? "s" : [i, ({x:1})])]); } catch (err) { print("TOP", err.name, err.message); }
try { do { if ((typeof ((null + false) ? [1,2] : (d || false)))) { arr.push((({p: (Infinity + null), q: (!NaN)}) < (f(0) && (({x:1}) ? "s" : s)))); print(arr.length, arr.join(",")); } else { try { print(((({p: 2, q: NaN}) == [g, Infinity]) < [1, (o.x ? null : arr.length)])); } catch (e) { print("c", e.name); } finally { print("fin"); } } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((0 ? f(({p: 0.5, q: 1})) : (0 + NaN))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
