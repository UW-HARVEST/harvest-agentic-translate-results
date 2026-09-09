var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if (Infinity) { try { print((((NaN ? c : d) / ({x:1})) == undefined)); } catch (e) { print("caught", e.name); } } else { switch ((({p: (-undefined), q: [false, arr[0]]}) - ((NaN === ({x:1})) << (arr[0] / arr.length)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((((null <= 0) & ({p: 0.5, q: false})) == (!(i % ({x:1}))))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (({p: f((void ({}))), q: ([({}), i] !== (({}) | g))})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if ([((+Infinity) >= arr.length), 0]) { print(((("" ? -1 : 0) >>> (void false)) - f((b + d)))); } else { print(c); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ({p: (g != false), q: (--1)}), q: (typeof [NaN, 2])})); } catch (err) { print("TOP", err.name, err.message); }
try { print([([0, []] || 0), (typeof (a + ({})))]); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; arr.push((f(1) ? ((null + []) & (NaN ? null : s)) : (({p: b, q: ({})}) ? [2, c] : f(arr[0])))); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((true === (null ? d : d)) ? ((-1 === false) != 2) : [["", f(1)], (1 | ({}))])); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: false, q: (-[Infinity, undefined])})); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; o.y = ({p: (~null), q: (+NaN)}); print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { a = ([({p: i, q: ({})}), (arr.length <= 0)] >= ((f(1) !== d) && (arr.length && [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(Infinity); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
