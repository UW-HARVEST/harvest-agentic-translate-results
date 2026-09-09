var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { d = ((({p: [1,2], q: null}) * (undefined && [1,2])) !== [f(arr.length), f(arr.length)]); } catch (err) { print("TOP", err.name, err.message); }
try { switch (((Infinity ? [arr[0], Infinity] : (~2)) - [f(g), (s >> NaN)])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (s) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((((undefined || 1) - true) ? ((void "s") < [({}), ({})]) : ((s === -1) + (c - false)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((((c ? 0.5 : -1) >= d) | o.x)), typeof ([])); } catch (err) { print("TOP", err.name, err.message); }
try { print(([({p: arr.length, q: Infinity}), ([] ? ({}) : "s")] << i)); } catch (err) { print("TOP", err.name, err.message); }
try { c = ({p: (2 + (0.5 !== 0.5)), q: (typeof (Infinity === false))}); } catch (err) { print("TOP", err.name, err.message); }
try { print(((0 ^ (false | null)) + f((true || 0)))); } catch (err) { print("TOP", err.name, err.message); }
try { print([f([({}), "s"]), ({})]); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = f((typeof (0 <= 0))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { if (f(1)) { print(({p: arr.length, q: d})); } else { print(a); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f((~(NaN | c)))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
