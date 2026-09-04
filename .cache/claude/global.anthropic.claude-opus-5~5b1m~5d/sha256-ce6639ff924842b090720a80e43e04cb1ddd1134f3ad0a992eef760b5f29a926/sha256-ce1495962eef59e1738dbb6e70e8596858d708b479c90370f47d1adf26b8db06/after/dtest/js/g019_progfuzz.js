var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print([(void b), ((f(1) ? 0 : arr[0]) >>> (a ? 0.5 : false))]); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((((NaN | []) ? arr[0] : (void Infinity)) !== (0.5 ? ({x:1}) : (true % [])))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; arr.push(arr[0]); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(false); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print([((a - d) ? ({p: o.x, q: 0}) : (1 >>> 0)), [(2 || ({})), [1,2]]]); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ([f(1), arr.length] <= [1, []]), q: (undefined <= (0.5 ? arr.length : "s"))})); } catch (err) { print("TOP", err.name, err.message); }
try { print(false); } catch (err) { print("TOP", err.name, err.message); }
try { print((a >= g)); } catch (err) { print("TOP", err.name, err.message); }
try { b = NaN; } catch (err) { print("TOP", err.name, err.message); }
try { b = [[(true && a), (b ? a : true)], i]; } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if (({p: f(false), q: f(({}))})) { print((i < ((!0.5) << 0.5))); } else { try { for (i = 0; i < 3; ++i) { print((false / f(f("")))); } } catch (e) { print("c", e.name); } finally { print("fin"); } } } } catch (err) { print("TOP", err.name, err.message); }
try { switch (0.5) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
