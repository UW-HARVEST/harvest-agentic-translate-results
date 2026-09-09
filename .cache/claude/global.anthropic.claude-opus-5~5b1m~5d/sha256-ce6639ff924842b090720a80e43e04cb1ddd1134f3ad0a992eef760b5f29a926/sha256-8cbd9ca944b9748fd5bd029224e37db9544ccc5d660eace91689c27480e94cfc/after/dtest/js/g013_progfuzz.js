var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((+((a > i) ? (NaN ? g : a) : a))); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((+[(undefined ? NaN : 2), [false, "s"]])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(2); } catch (err) { print("TOP", err.name, err.message); }
try { print((f(arr[0]) <= ([d, null] !== i))); } catch (err) { print("TOP", err.name, err.message); }
try { print((d ? b : undefined)); } catch (err) { print("TOP", err.name, err.message); }
try { print(b); } catch (err) { print("TOP", err.name, err.message); }
try { print((true ? [(({x:1}) | -1), s] : (d >> (~c)))); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [(arr.length >> (-[1,2])), null]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { try { c = false; } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { try { print(([1,2] / (i >= [s, g]))); } catch (e) { print("caught", e.name); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { i = 0; while (i < 2) { ++i; if ((true | (!f(0.5)))) { try { if ((((o.x || d) | -1) <= ("s" / [o.x, d]))) { for (var k in (~((c == f(1)) ? ({p: d, q: g}) : (({}) ? arr[0] : d)))) print("k", k); } else { if ((f([NaN, b]) ? (typeof (null | o.x)) : ({p: (0 === o.x), q: f("s")}))) { print(((0.5 << (i ? undefined : ({x:1}))) >> ([b, d] || 2))); } else { if ((((void i) ^ ({p: ({}), q: a})) & [(a == c), [1,2]])) { do { try { o.y = f((false != (({}) | []))); print(JSON.stringify(o)); } catch (e) { print("caught", e.name); } } while (false); } else { for (var k in [1, ((arr.length || null) < (({x:1}) >> d))]) print("k", k); } } } } catch (e) { print("c", e.name); } finally { print("fin"); } } else { switch (f(f(1))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { do { for (i = 0; i < 3; ++i) { do { a = (([i, 0.5] >> (arr.length ? f(1) : [])) | f((~b))); } while (false); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
