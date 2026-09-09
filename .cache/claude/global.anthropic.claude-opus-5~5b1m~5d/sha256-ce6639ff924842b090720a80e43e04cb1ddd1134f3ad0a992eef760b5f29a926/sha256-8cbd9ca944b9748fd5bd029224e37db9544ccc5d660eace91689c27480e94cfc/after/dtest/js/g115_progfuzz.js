var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((function(p) { return [(f(1) == undefined), arr.length]; })((typeof 2))); } catch (err) { print("TOP", err.name, err.message); }
try { if (((-2) < (("" == 1) ? (true ? s : Infinity) : ([] / "s")))) { print(({})); } else { print([[a, 1], ({})]); } } catch (err) { print("TOP", err.name, err.message); }
try { if (({})) { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; print((![(i != "s"), f([])])); } } } } else { print((function(p) { return (null ? arr.length : null); })([f("s"), (void f(2))])); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { if ((typeof false)) { for (i = 0; i < 3; ++i) { try { try { for (i = 0; i < 3; ++i) { print((function(p) { return true; })((~false))); } } catch (e) { print("caught", e.name); } } catch (e) { print("c", e.name); } finally { print("fin"); } } } else { b = arr[0]; } } } catch (err) { print("TOP", err.name, err.message); }
try { try { print(String(([["", [1,2]], (c ? false : c)] >>> f((NaN ? ({x:1}) : true)))), typeof (([a, (d * false)] < ((arr.length - 1) / (i >= c))))); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = b; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(0.5); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((("s" >> (typeof null)) % ([arr[0], undefined] <= NaN))) { print([[], ((null >= ({x:1})) | (-1 / true))]); } else { print(false); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; c = (((1 > f(1)) > ({p: 2, q: null})) % 1); } } } catch (err) { print("TOP", err.name, err.message); }
try { print([((void s) > (({x:1}) >> [])), f((c / [1,2]))]); } catch (err) { print("TOP", err.name, err.message); }
try { print((-([i, a] ? (f(1) > Infinity) : []))); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (((d ? 0 : ({x:1})) ? ({p: g, q: 1}) : f(null)) < (([1,2] ? arr.length : o.x) ? ({p: s, q: s}) : (f(1) == Infinity)))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
