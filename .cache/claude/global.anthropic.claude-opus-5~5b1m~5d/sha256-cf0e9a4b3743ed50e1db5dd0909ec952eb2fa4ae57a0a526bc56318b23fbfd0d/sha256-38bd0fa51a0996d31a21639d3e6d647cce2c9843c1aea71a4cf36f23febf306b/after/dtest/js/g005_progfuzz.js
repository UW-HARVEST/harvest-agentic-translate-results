var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f(((0.5 > -1) != (({x:1}) + "")))); } catch (err) { print("TOP", err.name, err.message); }
try { if ((f((b & f(1))) <= [(({}) && 0), []])) { try { print((((a ? true : arr.length) != f(c)) ^ ((false << ({x:1})) === (true !== 1)))); } catch (e) { print("caught", e.name); } } else { print((Infinity > NaN)); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (var k in (((g >>> d) >> (({x:1}) >> a)) + ((undefined | null) == (0.5 >> ({}))))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(({x:1}))); } catch (err) { print("TOP", err.name, err.message); }
try { print(g); } catch (err) { print("TOP", err.name, err.message); }
try { d = ((typeof "") || ([1,2] !== "s")); } catch (err) { print("TOP", err.name, err.message); }
try { d = f(["s", ({p: undefined, q: d})]); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch ((f(Infinity) | f(({p: -1, q: ""})))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((f((0.5 - "s")) >>> (~(undefined < 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { a = c; } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: i, q: ({})})); } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(((s !== arr.length) === g))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
