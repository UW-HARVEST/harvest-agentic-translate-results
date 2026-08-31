var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f((({p: arr[0], q: []}) ? c : (Infinity ? ({}) : [])))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (f((!(void c)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f((typeof (1 * i)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in 2) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print([((({}) ? d : Infinity) % (i ? 0 : g)), Infinity]); } } } catch (err) { print("TOP", err.name, err.message); }
try { if ((s ? f((null == c)) : ((o.x == "s") ? (void Infinity) : undefined))) { if ((-o.x)) { switch ((!(("" <= true) >>> i))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { print(((void (({}) + NaN)) !== s)); } } else { arr.push(f((false ? f(f(1)) : f("s")))); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { b = ""; } catch (err) { print("TOP", err.name, err.message); }
try { try { b = (((f(1) == ({x:1})) % b) % ([] - ({p: d, q: Infinity}))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if (0.5) { if (0) { print((+(+g))); } else { if (((Infinity - (true >= d)) && f([[], ({x:1})]))) { switch ((([1, [1,2]] >>> (0 || s)) * ((null * 2) ? ({p: false, q: arr[0]}) : f(g)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { print(({p: b, q: null})); } } } else { arr.push(NaN); print(arr.length, arr.join(",")); } } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(({p: (b !== 2), q: ((typeof f(1)) && (g + undefined))})); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ((i >> f(1)) ? ({}) : (!s)), q: null})); } catch (err) { print("TOP", err.name, err.message); }
try { print(((typeof (1 ? true : s)) / 0)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
