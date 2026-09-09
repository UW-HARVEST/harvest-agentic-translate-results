var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((((NaN / 0.5) + [NaN, undefined]) ? [({p: a, q: ({x:1})}), ({p: s, q: ({})})] : ({p: a, q: (-true)}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((((o.x ? -1 : b) >= (0.5 / undefined)) && (typeof (+[1,2])))); } catch (err) { print("TOP", err.name, err.message); }
try { print(s); } catch (err) { print("TOP", err.name, err.message); }
try { print(f((("s" >= "s") >= (i > false)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { print(((0 <= ({p: g, q: a})) - NaN)); } catch (err) { print("TOP", err.name, err.message); }
try { print(((f(null) && (0.5 / g)) >>> (([] << o.x) & (false << true)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((-Infinity)); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([((-g) - (Infinity !== o.x)), ([arr[0], undefined] + (undefined ? c : f(1)))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { c = f(1); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(f([g, NaN]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(((-(c ? true : f(1))) >>> ((0 !== NaN) | b))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
