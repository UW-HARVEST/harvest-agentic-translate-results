var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { b = [((NaN ? o.x : 2) >> (+arr[0])), [(-1 >>> s), undefined]]; } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((arr[0] ? true : "s")); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: f((undefined || NaN)), q: [[true, null], (s ^ g)]})); } catch (err) { print("TOP", err.name, err.message); }
try { print(c); } catch (err) { print("TOP", err.name, err.message); }
try { do { try { print(([1,2] ? [(Infinity ? c : b), (!-1)] : ({p: f(({})), q: [({}), f(1)]}))); } catch (e) { print("caught", e.name); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(({p: g, q: [(a * undefined), ""]})); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ([1,2] || (!(NaN != arr.length)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { if (d) { if ((("" * Infinity) & [(NaN === true), i])) { try { o.y = (f(1) >> (~2)); print(JSON.stringify(o)); } catch (e) { print("caught", e.name); } } else { do { print((f((-1 ? null : 0)) <= 0.5)); } while (false); } } else { for (i = 0; i < 3; ++i) { switch ((({x:1}) && (f(s) ^ (undefined && arr.length)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(Infinity); } catch (err) { print("TOP", err.name, err.message); }
try { do { print(b); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(([(typeof i), (undefined << arr.length)] * f([o.x, 2]))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ({p: (void (-0)), q: ((true ? arr.length : i) / arr.length)})) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
