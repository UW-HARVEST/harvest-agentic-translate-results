var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = ((({p: b, q: d}) >= (({}) ? "s" : [1,2])) ? (-1 | ("" === d)) : (true - NaN)); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { b = b; } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(Infinity); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((+(arr.length >= null)) ? ((a + d) ? ({p: undefined, q: g}) : (typeof f(1))) : ({p: (f(1) << c), q: "s"}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ""; })([(undefined << (a - [1,2])), [(typeof 2), (d > 0.5)]])); } catch (err) { print("TOP", err.name, err.message); }
try { a = (typeof ((c | o.x) == ("" == []))); } catch (err) { print("TOP", err.name, err.message); }
try { print((i & f(1))); } catch (err) { print("TOP", err.name, err.message); }
try { try { do { print((((a < undefined) <= (0.5 ? arr[0] : b)) ? ((-1 ? true : []) ? ({p: undefined, q: false}) : 0) : (undefined == 0))); } while (false); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(((a ? (+arr.length) : (NaN >= f(1))) / o.x)); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print((f(f(arr.length)) % d)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { arr.push(f(i)); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in (((0.5 ? 2 : b) > (Infinity != "s")) <= a)) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
