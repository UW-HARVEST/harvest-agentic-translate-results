var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { d = [((!a) >= ({p: "s", q: i})), ([i, 0] >> ({p: undefined, q: -1}))]; } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ({p: (({p: o.x, q: s}) != ("s" > 0.5)), q: [(-undefined), Infinity]}); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { if (o.x) { c = [f([c, 2]), ({p: ("s" >>> null), q: f(1)})]; } else { a = (typeof arr[0]); } } catch (err) { print("TOP", err.name, err.message); }
try { print(([(+({})), (arr[0] === 2)] === ((true ? f(1) : ({x:1})) ? f(undefined) : [[], "s"]))); } catch (err) { print("TOP", err.name, err.message); }
try { try { i = 0; while (i < 2) { ++i; try { print(({p: ((a || undefined) === (Infinity - arr.length)), q: 0.5})); } catch (e) { print("caught", e.name); } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(([] <= ([true, "s"] / ({p: d, q: false})))); } catch (err) { print("TOP", err.name, err.message); }
try { a = arr[0]; } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(((NaN >= f(1)) <= (+Infinity))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((([b, arr[0]] == f(0.5)) != [(0.5 & s), (void -1)])); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: f(1), q: [(({x:1}) ? [] : NaN), Infinity]})); } catch (err) { print("TOP", err.name, err.message); }
try { c = ([(arr[0] < b), i] ? ({}) : a); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return f((f(1) % null)); })(true)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
