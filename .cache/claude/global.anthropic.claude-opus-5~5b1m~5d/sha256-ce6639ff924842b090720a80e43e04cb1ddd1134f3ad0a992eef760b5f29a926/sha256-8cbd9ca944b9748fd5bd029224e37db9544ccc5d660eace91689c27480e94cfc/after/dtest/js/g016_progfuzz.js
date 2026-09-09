var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((a && (([1,2] << b) & ([1,2] >= Infinity)))); } catch (err) { print("TOP", err.name, err.message); }
try { c = [[(c ? 2 : d), (-1 <= null)], ""]; } catch (err) { print("TOP", err.name, err.message); }
try { print([arr[0], f((~false))]); } catch (err) { print("TOP", err.name, err.message); }
try { print((!(o.x - ([] / o.x)))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(((("" || 2) <= Infinity) && f(f(0.5)))); } } catch (err) { print("TOP", err.name, err.message); }
try { d = (f([0, undefined]) + ({p: f(""), q: (!a)})); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((+[(+Infinity), f("")])); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { a = (((+g) | (-true)) - o.x); } catch (err) { print("TOP", err.name, err.message); }
try { print(({})); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ((void f(NaN)) != [(d / g), [null, i]]); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print([[(arr.length ? ({}) : arr[0]), f(s)], -1]); } catch (err) { print("TOP", err.name, err.message); }
try { d = (({p: (i ? a : 0.5), q: NaN}) || ({p: true, q: (true < [1,2])})); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
