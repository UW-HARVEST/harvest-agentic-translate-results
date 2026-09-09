var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { print((({p: (void 2), q: "s"}) < f((Infinity == 2)))); } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (g & (null * g)); })((typeof NaN))); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: f((s + undefined)), q: f(([1,2] ? d : i))})); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(""); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { try { a = (((~a) != 2) ? f((2 > 0)) : ((0 ? true : arr[0]) !== 0)); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (void [f(b), (d < b)]); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { a = true; } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((c || arr.length) ? (f(1) ? -1 : c) : [d, d]); })((((+g) - g) !== ({})))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (f(-1) >>> f(c)); })((void f(1)))); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return f(({p: undefined, q: true})); })(([1,2] !== Infinity))); } catch (err) { print("TOP", err.name, err.message); }
try { print((f(-1) != ({p: (2 >= true), q: (-c)}))); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (i = 0; i < 3; ++i) { if ((f((g ? 0 : false)) | ((typeof "") + (false ? Infinity : "")))) { arr.push(arr.length); print(arr.length, arr.join(",")); } else { print((!(f(1) >= g))); } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
