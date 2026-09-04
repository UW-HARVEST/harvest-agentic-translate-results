var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f(((typeof s) ? (a >>> []) : (0.5 & [1,2])))); } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return 2; })((o.x && (({p: false, q: d}) >> ({p: c, q: null}))))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (false) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; do { print(String(((("" + ({})) << 0) << ({p: (c * 1), q: f(Infinity)}))), typeof (("" * (("s" | o.x) & (+c))))); } while (false); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((~(a >>> (typeof a)))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (2) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(b); } catch (err) { print("TOP", err.name, err.message); }
try { try { print([([[], 0.5] ^ (+b)), ((-"") || (2 << d))]); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (f(f([1,2])) === b); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(String(([[1,2], ("" >= [])] ? Infinity : b)), typeof (d)); } } catch (err) { print("TOP", err.name, err.message); }
try { print([(+(void arr.length)), -1]); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
