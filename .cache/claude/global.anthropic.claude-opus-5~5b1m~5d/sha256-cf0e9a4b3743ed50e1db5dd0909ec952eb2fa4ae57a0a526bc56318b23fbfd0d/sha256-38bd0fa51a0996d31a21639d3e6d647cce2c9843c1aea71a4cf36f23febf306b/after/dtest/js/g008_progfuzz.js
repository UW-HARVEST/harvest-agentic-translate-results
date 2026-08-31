var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((([({x:1}), 0] | (undefined <= true)) >>> f((typeof "s")))); } catch (err) { print("TOP", err.name, err.message); }
try { d = ([(({x:1}) ? o.x : 1), 2] > 0.5); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(((d == [true, 0]) === (("" ? "" : undefined) ? f(o.x) : [({x:1}), f(1)]))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((void ((~true) >> b))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String((![s, (NaN || 0.5)])), typeof (0)); } catch (err) { print("TOP", err.name, err.message); }
try { try { a = (({p: (NaN && g), q: (({}) && [])}) - f(({p: s, q: ""}))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ((typeof (void -1)) ? (1 !== ([1,2] >>> g)) : (("s" ? d : -1) !== (NaN && [1,2])))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if (((false ? ([1,2] ? b : 2) : ({p: b, q: []})) !== [undefined, (f(1) <= d)])) { print(({p: undefined, q: ""})); } else { a = (f((NaN >= s)) <= undefined); } } catch (err) { print("TOP", err.name, err.message); }
try { d = (false || ("s" ? (g ^ -1) : (i ? false : undefined))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((c >>> (({p: Infinity, q: ({})}) - [null, true]))); } } catch (err) { print("TOP", err.name, err.message); }
try { if (arr[0]) { print(1); } else { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print([((true >= ({})) >= 2), ((NaN ? "s" : 0.5) != f(({x:1})))]); } } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (+(1 == null))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
