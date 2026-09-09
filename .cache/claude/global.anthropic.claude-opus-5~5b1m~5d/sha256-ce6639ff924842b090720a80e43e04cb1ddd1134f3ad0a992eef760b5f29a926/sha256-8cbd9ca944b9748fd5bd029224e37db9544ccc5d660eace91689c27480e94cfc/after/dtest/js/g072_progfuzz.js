var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { i = 0; while (i < 2) { ++i; if ((arr[0] && ((void f(1)) >>> (0 ^ o.x)))) { try { do { print(((false % (i ? b : false)) ? (![0, 0.5]) : (~1))); } while (false); } catch (e) { print("c", e.name); } finally { print("fin"); } } else { print([arr.length, ((o.x === undefined) % null)]); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(([(1 * s), (NaN ? "s" : arr.length)] >> ((void false) > -1))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return undefined; })(f((undefined * [0.5, ({x:1})])))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((typeof ({p: ([1,2] | a), q: null}))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; a = ([b, f(({x:1}))] >> (void f(({x:1})))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((Infinity ? ((arr[0] >>> arr.length) == [1,2]) : false)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(s); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(([] !== [(Infinity << false), (f(1) > null)])); } catch (err) { print("TOP", err.name, err.message); }
try { print(f([(undefined | [1,2]), (({x:1}) ^ c)])); } catch (err) { print("TOP", err.name, err.message); }
try { print((!2)); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(([b, NaN] / d)), typeof ([])); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ([o.x, (0 && [])] | null); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
