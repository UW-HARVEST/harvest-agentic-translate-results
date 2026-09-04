var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(({p: f((1 <= b)), q: f((0.5 === ({x:1})))})); } catch (err) { print("TOP", err.name, err.message); }
try { if ((((null - ({x:1})) * [2, -1]) & c)) { print(f(((-1 >> i) < ({p: g, q: true})))); } else { print(((f(arr[0]) !== g) > (f(({})) ? NaN : ({p: a, q: "s"})))); } } catch (err) { print("TOP", err.name, err.message); }
try { if (0) { try { do { arr.push(([f(c), o.x] <= ((+NaN) >>> (true > -1)))); print(arr.length, arr.join(",")); } while (false); } catch (e) { print("caught", e.name); } } else { try { print((f([b, d]) >>> ((0 & o.x) / [a, arr[0]]))); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [(void []), ""]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(("" !== ((true / true) ? (i == true) : (({x:1}) >>> i)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((~((NaN ? c : false) ? ([1,2] ? [1,2] : Infinity) : ({p: null, q: 0.5})))); } catch (err) { print("TOP", err.name, err.message); }
try { if ((([-1, -1] >>> [1,2]) && (("s" >> undefined) ? true : (0.5 && b)))) { switch (({p: [f(1), arr[0]], q: ({p: 0.5, q: i})})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { arr.push((((Infinity ? true : NaN) === (d >= true)) ? (([1,2] | "s") <= [arr.length, 1]) : f((1 % ({}))))); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { try { if ((!((arr.length ? b : s) && (g || arr[0])))) { print(2); } else { print((function(p) { return "s"; })("")); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print((undefined == (b ? (f(1) & 2) : arr[0]))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((((a | 0.5) >= o.x) ^ -1)); } catch (err) { print("TOP", err.name, err.message); }
try { print([]); } catch (err) { print("TOP", err.name, err.message); }
try { print((s | (("" == o.x) - (NaN ? g : "s")))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
