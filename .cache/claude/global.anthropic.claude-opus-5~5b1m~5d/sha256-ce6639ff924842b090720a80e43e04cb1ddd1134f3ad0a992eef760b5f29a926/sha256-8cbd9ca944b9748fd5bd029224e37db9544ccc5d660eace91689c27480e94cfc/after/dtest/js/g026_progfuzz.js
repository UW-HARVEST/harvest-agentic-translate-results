var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { print((typeof f(({p: b, q: NaN})))); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (([Infinity, 1] ? arr[0] : (!0)) % (f(g) - (g < a)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(s)); } catch (err) { print("TOP", err.name, err.message); }
try { if (a) { print(String(((f(Infinity) >= (void c)) ? f((+"s")) : i)), typeof (((~-1) > (("s" ? s : NaN) ? 1 : s)))); } else { for (var k in (([null, arr.length] != (NaN && ({}))) - s)) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(0.5); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: f(({p: -1, q: 1})), q: (({}) ^ undefined)})) { for (var k in i) print("k", k); } else { i = 0; while (i < 2) { ++i; if (0.5) { d = [((false ? -1 : 1) - [c, undefined]), undefined]; } else { if (((f(false) || [b, ""]) ? o.x : 0.5)) { try { for (i = 0; i < 3; ++i) { print(f(f("s"))); } } catch (e) { print("caught", e.name); } } else { arr.push((-("" ? (i / undefined) : c))); print(arr.length, arr.join(",")); } } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: ({p: "s", q: (Infinity == arr[0])}), q: f((g % c))})); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { for (var k in (o.x % "s")) print("k", k); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { if (({p: ((typeof NaN) > (1 != s)), q: (void (arr.length >>> ""))})) { if (({p: ({p: (1 == d), q: ([] || s)}), q: b})) { print((Infinity <= ((c || Infinity) != (arr.length ? 1 : 1)))); } else { arr.push(((({p: arr.length, q: 1}) ? o.x : [a, ""]) * ((f(1) ? s : 0.5) >>> Infinity))); print(arr.length, arr.join(",")); } } else { c = ((arr[0] >>> (b != a)) & (f(f(1)) == (+null))); } } catch (err) { print("TOP", err.name, err.message); }
try { b = ((b * (+Infinity)) | [undefined, (o.x % null)]); } catch (err) { print("TOP", err.name, err.message); }
try { c = (~[]); } catch (err) { print("TOP", err.name, err.message); }
try { print((!(~(arr.length == 0)))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
