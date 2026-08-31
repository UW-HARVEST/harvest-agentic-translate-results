var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print((([d, s] % a) !== 1)); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { c = ((({p: b, q: NaN}) / a) & ((true >> -1) ? (({}) !== 0.5) : (arr.length ? arr.length : d))); } } catch (err) { print("TOP", err.name, err.message); }
try { a = ([[NaN, -1], (i == "")] | false); } catch (err) { print("TOP", err.name, err.message); }
try { print(undefined); } catch (err) { print("TOP", err.name, err.message); }
try { b = (f(arr[0]) ? f([arr.length, true]) : ((({x:1}) ? d : 1) ? (void ({})) : i)); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({x:1}); })(f(([0.5, 0] ? 2 : (!false))))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if ((~({x:1}))) { i = 0; while (i < 2) { ++i; print(((~f([])) % "s")); } } else { if (({p: (NaN && f(null)), q: (("s" ? 1 : c) > (({}) ? i : true))})) { print(f(((0 || a) ? f(i) : ([] | [])))); } else { print(f(b)); } } } } catch (err) { print("TOP", err.name, err.message); }
try { try { o.y = (!({p: f(s), q: (null >> s)})); print(JSON.stringify(o)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(d); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((((c <= null) === f(g)) && ((NaN !== ({})) < ("" ? ({x:1}) : s)))); } catch (err) { print("TOP", err.name, err.message); }
try { if ((typeof true)) { try { print(false); } catch (e) { print("caught", e.name); } } else { o.y = -1; print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return arr[0]; })(c)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
