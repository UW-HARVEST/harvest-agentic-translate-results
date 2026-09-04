var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { print(((!(arr.length ? [1,2] : arr[0])) != ((undefined != null) ? arr[0] : (true - Infinity)))); } catch (err) { print("TOP", err.name, err.message); }
try { switch (d) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((arr[0] != (f(null) & a))); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [b, (f(({})) | (d ? Infinity : true))]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((function(p) { return (({}) ? 2 : false); })(true)); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([[f(({})), ({p: "", q: undefined})], (~b)]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(({p: (-(NaN > b)), q: [undefined, ({p: undefined, q: ""})]})); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (~(void NaN)); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (o.x > ([] == o.x)); })(Infinity)); } catch (err) { print("TOP", err.name, err.message); }
try { d = [(o.x < (-0.5)), (void ({p: undefined, q: -1}))]; } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; i = 0; while (i < 2) { ++i; for (var k in (i < b)) print("k", k); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
