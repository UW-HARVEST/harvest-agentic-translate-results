var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = (typeof [({p: o.x, q: true}), [c, ({})]]); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(([] >>> ((g | null) * ([] ? [1,2] : NaN)))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (({p: ("s" ? (c & c) : (NaN <= NaN)), q: undefined})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print((function(p) { return (({}) & false); })(0)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (0.5) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((d || null) & (-1)) ? (g ? "s" : (b ? 0 : 1)) : f(f(o.x)))); } catch (err) { print("TOP", err.name, err.message); }
try { do { print(String([((o.x / b) ? ({p: 0, q: arr[0]}) : "s"), [(!undefined), (~-1)]]), typeof (g)); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: ((-f(1)) >>> [b, [1,2]]), q: []})) { o.y = undefined; print(JSON.stringify(o)); } else { for (i = 0; i < 3; ++i) { try { print((({}) < (typeof c))); } catch (e) { print("caught", e.name); } } } } catch (err) { print("TOP", err.name, err.message); }
try { try { switch (((-o.x) && ((i ^ s) === (void "")))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(arr[0]); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if ((({p: [[], o.x], q: (b < ({}))}) ? 0 : (f(({x:1})) | (({}) ^ [1,2])))) { for (var k in ((f(({})) ? ({p: [], q: -1}) : (f(1) * NaN)) !== 2)) print("k", k); } else { print(undefined); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { arr.push(({p: (~(o.x ? arr[0] : i)), q: false})); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
