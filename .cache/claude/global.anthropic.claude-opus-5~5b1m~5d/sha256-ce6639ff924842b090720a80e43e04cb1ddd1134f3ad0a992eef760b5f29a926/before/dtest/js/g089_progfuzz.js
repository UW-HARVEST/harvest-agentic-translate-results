var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { arr.push(f((1 / arr[0]))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { if ((-({p: (({}) ? "s" : d), q: f(s)}))) { print(String((+(({p: i, q: false}) & (false ? [] : ({x:1}))))), typeof ((~[(2 >>> 0.5), (0 ? arr[0] : NaN)]))); } else { print([((!({})) === (g | s)), [(!false), -1]]); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(arr.length); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(false); } catch (err) { print("TOP", err.name, err.message); }
try { if (("s" ? f([Infinity, 2]) : null)) { print((function(p) { return (arr[0] ? 2 : arr[0]); })(2)); } else { for (var k in (({p: (true ? d : null), q: (arr.length ? i : 0.5)}) & arr[0])) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((([arr.length, ({x:1})] || (void undefined)) % (({p: b, q: 0}) & (+[1,2])))) { print((+(({p: a, q: d}) + null))); } else { b = (+(~(i || o.x))); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(([0, (Infinity == i)] !== arr.length)); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { b = [(+[]), a]; } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { if ((f(({p: g, q: i})) ? [1,2] : (void [null, [1,2]]))) { try { print(f(Infinity)); } catch (e) { print("caught", e.name); } } else { for (i = 0; i < 3; ++i) { o.y = ((-null) ? (void (-1 / a)) : ((i ? b : null) % (i ? Infinity : arr.length))); print(JSON.stringify(o)); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof f([[], true]))); } catch (err) { print("TOP", err.name, err.message); }
try { print((arr[0] >>> (o.x / 2))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([1,2]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
