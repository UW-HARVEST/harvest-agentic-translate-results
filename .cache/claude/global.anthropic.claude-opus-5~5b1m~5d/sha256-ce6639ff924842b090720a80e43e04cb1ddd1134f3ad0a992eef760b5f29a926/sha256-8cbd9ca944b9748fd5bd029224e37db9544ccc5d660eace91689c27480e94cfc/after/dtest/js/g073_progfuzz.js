var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { arr.push(((+-1) < ((void 0) ? 2 : (-"s")))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((typeof (2 << (d + arr.length)))); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (({p: (-s), q: false}) << f(("s" !== f(1)))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(i); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((false % (f([]) << [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(b); } catch (err) { print("TOP", err.name, err.message); }
try { b = (arr[0] === arr.length); } catch (err) { print("TOP", err.name, err.message); }
try { if (undefined) { if ([d, f((![]))]) { print([1,2]); } else { print((null / f(""))); } } else { o.y = s; print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
try { print(((NaN ? g : (Infinity ? "" : i)) ^ d)); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; o.y = (~arr[0]); print(JSON.stringify(o)); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((((({x:1}) - 0.5) >>> ([] << -1)) ? null : [(d != b), (void 2)])); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return undefined; })(({}))); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
