var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(o.x); } catch (err) { print("TOP", err.name, err.message); }
try { d = null; } catch (err) { print("TOP", err.name, err.message); }
try { print((f(0.5) ? ((f(1) == a) && (c < [1,2])) : (({p: false, q: NaN}) ? [({}), undefined] : d))); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([[f(b), (b <= arr.length)], ({p: (0.5 == c), q: (o.x == false)})]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { d = ((typeof ({p: null, q: g})) * f(arr.length)); } catch (err) { print("TOP", err.name, err.message); }
try { switch (((f(1) !== (0.5 == i)) ? ((undefined / g) | -1) : i)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { if ([o.x, ((Infinity == 0) > [2, 0.5])]) { for (var k in ((({p: 0.5, q: true}) & (({x:1}) && s)) | ((false ? ({}) : []) >>> (a >> 1)))) print("k", k); } else { d = (a ^ f(f(b))); } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(i); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({x:1})); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (void [({}), ""]); })(((f(Infinity) != (arr.length - f(1))) ? arr.length : ({p: (2 << i), q: ([1,2] + ({x:1}))})))); } catch (err) { print("TOP", err.name, err.message); }
try { print(String([]), typeof (([[undefined, Infinity], (null || ({x:1}))] ? (({p: true, q: i}) > (void a)) : ([[1,2], NaN] / a)))); } catch (err) { print("TOP", err.name, err.message); }
try { if ([((NaN ? undefined : null) ? [null, null] : 0.5), ((true ^ 0.5) << c)]) { print((o.x < [(({}) >>> d), ({x:1})])); } else { for (i = 0; i < 3; ++i) { print((function(p) { return true; })((([[1,2], 0] >> (b >> 0)) < (!a)))); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
