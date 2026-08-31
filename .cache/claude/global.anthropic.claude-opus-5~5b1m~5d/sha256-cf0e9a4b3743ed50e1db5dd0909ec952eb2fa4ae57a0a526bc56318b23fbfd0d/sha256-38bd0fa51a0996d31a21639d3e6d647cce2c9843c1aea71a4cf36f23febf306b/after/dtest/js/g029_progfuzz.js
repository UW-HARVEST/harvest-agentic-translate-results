var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = 0; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { a = (!(f(b) || a)); } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: ((({}) != i) ? (0 >= [1,2]) : (0 << b)), q: ((false ? NaN : d) !== [o.x, ""])})) { try { d = ((typeof (g ? s : Infinity)) ? (d > f(i)) : (void 0.5)); } catch (e) { print("caught", e.name); } } else { arr.push(""); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((!f(undefined)) ? ((d && false) === (Infinity ? -1 : Infinity)) : Infinity)); } catch (err) { print("TOP", err.name, err.message); }
try { print(f([d, a])); } catch (err) { print("TOP", err.name, err.message); }
try { if (f(1)) { print(((false < [({}), true]) % f(i))); } else { print(f(Infinity)); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in a) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return true; })(((f(d) | (b % f(1))) - ((false <= f(1)) <= (false === -1))))); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { print(f(NaN)); } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (1 == -1), q: [NaN, "s"]}) && [d, (0 || i)])); } catch (err) { print("TOP", err.name, err.message); }
try { print((NaN == f(1))); } catch (err) { print("TOP", err.name, err.message); }
try { d = (!(!(s ? ({x:1}) : Infinity))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
