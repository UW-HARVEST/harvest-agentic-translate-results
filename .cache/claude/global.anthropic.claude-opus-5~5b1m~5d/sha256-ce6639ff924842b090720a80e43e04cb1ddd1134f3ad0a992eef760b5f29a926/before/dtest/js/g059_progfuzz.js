var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { do { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print([f((g >> -1)), [[({}), Infinity], [true, arr.length]]]); } } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { c = (((true ? null : Infinity) | c) && f((false ? undefined : 1))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (o.x * 0.5); })((((({x:1}) % 1) >= []) % (undefined / NaN)))); } catch (err) { print("TOP", err.name, err.message); }
try { if (f(null)) { print(({p: "", q: ((arr[0] ? [] : ({})) & -1)})); } else { d = (f((void false)) >= (f(2) / (-false))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(({p: b, q: ["", -1]}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(b); } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (arr.length ? g : 2), q: arr[0]}) ? ((0.5 === 0) || f(true)) : (+(d ? NaN : -1)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f([f(1), 2]))); } catch (err) { print("TOP", err.name, err.message); }
try { if (true) { if (arr.length) { print(f(2)); } else { b = []; } } else { for (i = 0; i < 3; ++i) { print(NaN); } } } catch (err) { print("TOP", err.name, err.message); }
try { if ((f(("" ? [1,2] : [])) + f((0 > "s")))) { print((((1 > s) + (0.5 && NaN)) || d)); } else { try { print((function(p) { return b; })(f((f([1,2]) == ({p: ({}), q: 2}))))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (NaN ? NaN : undefined)) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch ((f(({p: "", q: 1})) > (void (undefined ? [1,2] : 2)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
