var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(String("s"), typeof (((f(1) ? Infinity : (c | false)) ? s : (!({p: 0.5, q: ({})}))))); } catch (err) { print("TOP", err.name, err.message); }
try { if (b) { o.y = (~"s"); print(JSON.stringify(o)); } else { print((("" - ({p: ({}), q: d})) !== a)); } } catch (err) { print("TOP", err.name, err.message); }
try { d = "s"; } catch (err) { print("TOP", err.name, err.message); }
try { print((({}) - ((!c) === false))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(((({p: g, q: ""}) ? arr[0] : f([])) ^ f(({p: arr[0], q: 0})))); } } catch (err) { print("TOP", err.name, err.message); }
try { print([f((a >>> b)), (void (undefined - null))]); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { arr.push(true); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((((a == 1) ? ({p: s, q: NaN}) : (i >>> ({}))) & f(([1,2] ? f(1) : "s")))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { switch (((({p: 0.5, q: 0.5}) <= i) && i)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((false || (undefined << o.x)) - ((null ? s : null) ? ({p: b, q: []}) : null))); } catch (err) { print("TOP", err.name, err.message); }
try { print([NaN, Infinity]); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { a = (i ^ c); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
