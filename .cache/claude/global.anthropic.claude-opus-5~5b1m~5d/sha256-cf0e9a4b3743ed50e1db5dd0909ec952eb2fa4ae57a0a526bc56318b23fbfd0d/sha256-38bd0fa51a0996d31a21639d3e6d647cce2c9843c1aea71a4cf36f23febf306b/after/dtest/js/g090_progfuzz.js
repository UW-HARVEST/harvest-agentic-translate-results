var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if ((typeof (void (NaN && c)))) { switch (("s" && ({p: Infinity, q: (-"s")}))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { for (i = 0; i < 3; ++i) { print((function(p) { return arr[0]; })(([[arr.length, d], f(undefined)] <= [(b << f(1)), [d, s]]))); } } } catch (err) { print("TOP", err.name, err.message); }
try { do { print(""); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(((typeof ({p: [], q: Infinity})) * f((a ? arr[0] : ({x:1}))))); } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(("s" && (true ? (!b) : (false >= arr[0])))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print((([] ? f(a) : ("s" >= o.x)) && [])); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(String([([1,2] || Infinity), []]), typeof (f((NaN ? (b || 2) : ({p: b, q: []}))))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([[], ({x:1})]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (((s ? 2 : ({x:1})) != f([])) - ((+d) ? (~arr[0]) : (s ? d : Infinity)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { b = ([f(s), o.x] ? c : (~[0, arr[0]])); } catch (err) { print("TOP", err.name, err.message); }
try { b = ({p: ((g ? i : 0) << b), q: []}); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [1,2]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
