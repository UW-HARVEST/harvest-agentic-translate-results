var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(null); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((!([2, b] ? ([1,2] ? Infinity : c) : (!arr[0])))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((+(f(1) <= true)) | ((+arr.length) >= Infinity))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { c = [[(f(1) % c), [1,2]], ({p: [({}), a], q: i})]; } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(undefined); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(true); } catch (err) { print("TOP", err.name, err.message); }
try { if ((!undefined)) { c = ([1,2] !== (void (s >= c))); } else { if ((({p: f(""), q: (-1 <= NaN)}) ? o.x : i)) { do { i = 0; while (i < 2) { ++i; do { for (var k in ({x:1})) print("k", k); } while (false); } } while (false); } else { i = 0; while (i < 2) { ++i; do { if ((c / [(undefined - "s"), (-1 ? i : "s")])) { switch (([f(undefined), [c, null]] ^ i)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } else { for (var k in true) print("k", k); } } while (false); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print((((d ? 1 : b) && (null % 0.5)) > f(1))); } catch (err) { print("TOP", err.name, err.message); }
try { try { c = ({p: ((({x:1}) % Infinity) + (d >= [])), q: (typeof (o.x == c))}); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { b = d; } catch (err) { print("TOP", err.name, err.message); }
try { a = ((f(-1) ? false : (undefined ? NaN : o.x)) !== [f(arr[0]), [null, ""]]); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { for (i = 0; i < 3; ++i) { switch (c) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (e) { print("caught", e.name); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
