var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((function(p) { return (f([]) ^ (g ? 0 : null)); })(("s" ^ a))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((+[]) - (2 !== [1,2])))); } catch (err) { print("TOP", err.name, err.message); }
try { print((arr.length >>> ((NaN & "") <= (i % 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { a = ([(+null), (null && arr[0])] >>> ((o.x / Infinity) >> (arr.length > s))); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (((({p: f(1), q: i}) != f(1)) >> ([({}), arr[0]] % (-1 << s)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((s ^ 0.5) && false))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((({p: arr[0], q: o.x}) ? f(o.x) : [Infinity, ({x:1})]) ? g : [({p: "s", q: null}), (s ? ({x:1}) : g)])); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { print((function(p) { return ({}); })(i)); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; switch (f([([1,2] ? null : s), d])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { switch (arr.length) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(((c & undefined) / (null ? Infinity : false)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; try { d = ((true === [c, f(1)]) || g); } catch (e) { print("c", e.name); } finally { print("fin"); } } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
