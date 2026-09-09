var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { d = ((f(a) & (({}) & g)) ? NaN : ({p: [a, d], q: (i ^ 0)})); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(({p: (f(1) < (d != 0)), q: (NaN | (a << a))})); } } catch (err) { print("TOP", err.name, err.message); }
try { print(g); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((arr[0] >> "") <= (void arr[0])))); } catch (err) { print("TOP", err.name, err.message); }
try { print(([[true, f(1)], i] * a)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(({p: (f(arr.length) >= (arr[0] - i)), q: ({})})); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ({p: (b ? "s" : i), q: (-f(1))}); })((+(([] || undefined) ? f(1) : (null >> s))))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((!b) || a); })(({p: (({x:1}) !== [i, g]), q: []}))); } catch (err) { print("TOP", err.name, err.message); }
try { if ([((+NaN) === (s | 1)), [({p: "s", q: undefined}), (i != false)]]) { c = ({p: (arr.length ? (+undefined) : (arr[0] % null)), q: (f(c) ? 2 : (-({})))}); } else { if (a) { arr.push((({p: [1,2], q: (f(1) << arr.length)}) < ((!f(1)) * (c ? NaN : false)))); print(arr.length, arr.join(",")); } else { print(2); } } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((((({}) <= b) ^ [-1, c]) - ((arr[0] ? ({}) : 0.5) ? (void null) : [null, false]))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ({p: ({p: 0.5, q: 0}), q: s}), q: 0.5})); } catch (err) { print("TOP", err.name, err.message); }
try { if (((f([]) - a) | (undefined >>> [1,2]))) { print(b); } else { do { if ((f(2) ? (arr.length || (+[1,2])) : (("" >> ({})) ? ({p: i, q: null}) : (d ? f(1) : 2)))) { for (var k in (({p: (-1 === g), q: [arr[0], d]}) && 1)) print("k", k); } else { try { print(false); } catch (e) { print("caught", e.name); } } } while (false); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
