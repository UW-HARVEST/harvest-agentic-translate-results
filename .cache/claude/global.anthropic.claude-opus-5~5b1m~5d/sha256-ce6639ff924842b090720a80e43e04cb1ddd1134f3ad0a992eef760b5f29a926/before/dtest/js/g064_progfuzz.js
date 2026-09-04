var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { i = 0; while (i < 2) { ++i; for (var k in (((g ^ "s") < [[1,2], [1,2]]) === g)) print("k", k); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(f(1)); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { do { for (i = 0; i < 3; ++i) { if ((-((arr.length / 0.5) * (1 ? undefined : null)))) { do { try { print((function(p) { return i; })((~({p: 2, q: [s, ({x:1})]})))); } catch (e) { print("c", e.name); } finally { print("fin"); } } while (false); } else { print((o.x >>> ((arr.length >>> b) != i))); } } } while (false); } catch (e) { print("caught", e.name); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((((-[]) >> (true < "s")) ? (2 <= (1 >= i)) : [])) { try { print(({p: (void ({})), q: ((null === "s") | ({x:1}))})); } catch (e) { print("caught", e.name); } } else { try { print((function(p) { return Infinity; })([([1,2] ? 0.5 : 1), ((NaN ? false : [1,2]) ^ f([1,2]))])); } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ((typeof (({x:1}) ? arr[0] : null)) ^ (-1 < (f(1) !== NaN)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { c = [({}), f((1 & -1))]; } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; do { print(NaN); } while (false); } } } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((0 << ((~1) | (f(1) == 1)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; switch (f(1)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in Infinity) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(([(i || s), ([1,2] == arr[0])] ^ (({p: 0, q: 0}) >>> (1 << o.x)))), typeof ("")); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in [((i != "") == NaN), (arr.length | c)]) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
