var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f(({}))); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = f(((arr[0] % arr.length) < (f(1) == true))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { try { c = i; } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((a >> ((~Infinity) !== (c ? 0.5 : i)))) { print(((typeof (+1)) ? arr.length : 0)); } else { i = 0; while (i < 2) { ++i; print((((+({})) & f(undefined)) < (-1 % Infinity))); } } } catch (err) { print("TOP", err.name, err.message); }
try { b = ("s" !== (typeof d)); } catch (err) { print("TOP", err.name, err.message); }
try { c = i; } catch (err) { print("TOP", err.name, err.message); }
try { a = (((d <= undefined) !== (c ? c : false)) || ((void 1) !== f(0))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: (typeof [f(1), ({x:1})]), q: ((a < 0.5) >> "s")})); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { print((false ? i : [(d || d), (Infinity ? [1,2] : true)])); } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = (typeof (c === o.x)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(NaN); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((-false) != (false ^ 0)))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
