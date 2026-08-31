var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { d = (f((~s)) | ((typeof ({})) && a)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (var k in (+b)) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((((~"s") * (Infinity === d)) * (f(undefined) ? ({p: a, q: ({x:1})}) : ({p: NaN, q: c})))) { for (i = 0; i < 3; ++i) { try { switch (({p: undefined, q: Infinity})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } } else { try { for (i = 0; i < 3; ++i) { try { o.y = f(""); print(JSON.stringify(o)); } catch (e) { print("caught", e.name); } } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (0.5 || (({p: a, q: [1,2]}) == f(({x:1}))))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { c = ({p: ({p: (-[1,2]), q: ["s", o.x]}), q: d}); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in d) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { a = arr.length; } catch (err) { print("TOP", err.name, err.message); }
try { do { arr.push((void [1,2])); print(arr.length, arr.join(",")); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { c = []; } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (var k in (void (d + i))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; if ([({p: true, q: 1}), (f(0) <= f(arr[0]))]) { o.y = (({p: i, q: (-1 !== 1)}) * a); print(JSON.stringify(o)); } else { i = 0; while (i < 2) { ++i; do { try { print([(c >= (arr[0] || Infinity)), (void [({}), NaN])]); } catch (e) { print("caught", e.name); } } while (false); } } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; a = ([] / (undefined >> [c, 0])); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
