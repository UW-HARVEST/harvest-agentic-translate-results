var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { try { switch (NaN) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print(String([[(arr.length ? s : arr[0]), i], (~(({x:1}) ? null : ({})))]), typeof ((([1, "s"] << f(2)) === (-"s")))); } catch (err) { print("TOP", err.name, err.message); }
try { print((undefined > ((o.x >> null) - undefined))); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((void o.x) - (undefined <= undefined)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print([(c === (o.x && i)), [(f(1) ? 0 : b), [g, [1,2]]]]); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return o.x; })((void ({p: (0.5 + a), q: null})))); } catch (err) { print("TOP", err.name, err.message); }
try { a = f(({})); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [(f(null) ? ({p: c, q: a}) : true), (-(0.5 & NaN))]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print(String(s), typeof (0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { a = ([f(s), [b, true]] <= ""); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; print(([] ? ({p: -1, q: (1 ? ({}) : 2)}) : f(1))); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (!(-f(Infinity))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
