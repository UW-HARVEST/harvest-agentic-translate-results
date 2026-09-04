var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((~false)); } catch (err) { print("TOP", err.name, err.message); }
try { print(g); } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: g, q: (1 ^ (({}) ^ a))})) { try { for (i = 0; i < 3; ++i) { arr.push((-1 ? 1 : (-({p: undefined, q: arr.length})))); print(arr.length, arr.join(",")); } } catch (e) { print("c", e.name); } finally { print("fin"); } } else { try { print(({p: (({x:1}) | ([1,2] - c)), q: [({p: o.x, q: g}), arr[0]]})); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(""); } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: [arr.length, (void [1,2])], q: f([[], "s"])})) { i = 0; while (i < 2) { ++i; print(({x:1})); } } else { switch ((f(1) >>> f((-d)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(([null, b] > (o.x ? true : "s"))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { try { b = g; } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print([[1,2], [(NaN | "s"), [({}), "s"]]]); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { b = [(null ? ({p: undefined, q: true}) : 2), a]; } catch (err) { print("TOP", err.name, err.message); }
try { print((b >>> [({p: ({}), q: "s"}), undefined])); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { c = (((a + g) >= f(1)) ? (["", null] > 0) : [g, (({}) ? o.x : -1)]); } } catch (err) { print("TOP", err.name, err.message); }
try { if ([]) { switch ((f(1) || ([] !== (Infinity ? ({}) : NaN)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { do { i = 0; while (i < 2) { ++i; for (i = 0; i < 3; ++i) { if ([(+(g != c)), ([o.x, arr[0]] * i)]) { i = 0; while (i < 2) { ++i; a = (((2 && null) ? f(f(1)) : (typeof f(1))) >> ((d == 2) ? NaN : (g & 0))); } } else { print((function(p) { return 0; })((![f(c), (c ? [] : "")]))); } } } } while (false); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
