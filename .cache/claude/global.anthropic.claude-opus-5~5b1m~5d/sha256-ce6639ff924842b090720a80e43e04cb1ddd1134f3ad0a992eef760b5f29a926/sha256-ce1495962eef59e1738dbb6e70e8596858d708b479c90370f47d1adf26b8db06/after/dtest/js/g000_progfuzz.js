var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = ([1,2] !== (NaN + [b, NaN])); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { print(s); } catch (err) { print("TOP", err.name, err.message); }
try { print(((({x:1}) ? false : ("s" | [])) ? (~("s" === NaN)) : (o.x & ("s" == 2)))); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ([(true ? -1 : ({})), (g <= "")] ? (null - (0 + ({}))) : (typeof false)); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { try { if (f(f((f(1) && 0)))) { for (i = 0; i < 3; ++i) { a = ({p: [(~null), a], q: (({p: g, q: Infinity}) ? (undefined === undefined) : ({x:1}))}); } } else { if (({p: (!f([])), q: i})) { print((i ? 2 : (-({x:1})))); } else { o.y = (a < a); print(JSON.stringify(o)); } } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; if (NaN) { print(true); } else { print((function(p) { return [1,2]; })((null ? ({p: (~arr[0]), q: [0.5, d]}) : (([1,2] < 0.5) != ({p: true, q: []}))))); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(f((["", i] ? (({x:1}) * o.x) : undefined))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(0); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { d = [1,2]; } catch (err) { print("TOP", err.name, err.message); }
try { print([(!(i | 2)), 0.5]); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
