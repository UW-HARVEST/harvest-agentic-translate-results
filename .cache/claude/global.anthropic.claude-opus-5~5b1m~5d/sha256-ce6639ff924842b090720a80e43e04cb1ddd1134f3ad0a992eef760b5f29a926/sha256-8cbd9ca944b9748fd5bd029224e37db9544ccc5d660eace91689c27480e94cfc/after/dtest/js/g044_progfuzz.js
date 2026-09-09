var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((arr[0] > ({}))); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((({p: (arr[0] | 0.5), q: (false % b)}) ? ({p: (o.x ^ null), q: f(false)}) : ([b, ({})] > (1 + g)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { try { print(-1); } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { a = (({p: (arr.length >= [1,2]), q: (c | 0.5)}) * ([2, f(1)] & (2 === c))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { d = ({p: ((NaN > [1,2]) ? a : (0 ? 0 : NaN)), q: ((arr.length && c) % ([1,2] ? -1 : c))}); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = ({x:1}); } catch (err) { print("TOP", err.name, err.message); }
try { print([((undefined ? false : 2) - [1, arr.length]), [(true + []), (b + s)]]); } catch (err) { print("TOP", err.name, err.message); }
try { if ([[("s" < arr.length), 2], d]) { print([((0.5 != 0.5) || 2), ((({}) ? [] : "") == (~"s"))]); } else { switch ([[(undefined & false), NaN], (typeof (-({x:1})))]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { print([]); } catch (err) { print("TOP", err.name, err.message); }
try { print(((!(b == [])) ? f((void "s")) : 0)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; a = (arr.length ? [(-0.5), f(a)] : 1); } } } catch (err) { print("TOP", err.name, err.message); }
try { try { arr.push(("s" >> ({p: (0.5 == s), q: [NaN, undefined]}))); print(arr.length, arr.join(",")); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
