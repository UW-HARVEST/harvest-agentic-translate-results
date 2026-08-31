var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (var k in ((typeof (~o.x)) ? (i + f(Infinity)) : [(+arr.length), o.x])) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ({p: 2, q: ((typeof 0) < (s ? true : ({x:1})))}); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(({p: null, q: (typeof (s + 1))})); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { c = [("s" != (2 * ({x:1}))), -1]; } catch (err) { print("TOP", err.name, err.message); }
try { print([true, ({p: "", q: (b || [])})]); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [1,2]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ((g > f(1)) + ({p: 2, q: f("")}))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((i && (-[NaN, b]))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { b = 2; } catch (err) { print("TOP", err.name, err.message); }
try { if (((g ? f(({})) : Infinity) << ({p: (2 == ""), q: (0.5 ? d : c)}))) { print(("s" == ({p: (2 == a), q: (Infinity - -1)}))); } else { for (var k in ([undefined, arr.length] < (~([] > a)))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; print(o.x); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: [], q: ([] | (0.5 ? -1 : NaN))})); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
