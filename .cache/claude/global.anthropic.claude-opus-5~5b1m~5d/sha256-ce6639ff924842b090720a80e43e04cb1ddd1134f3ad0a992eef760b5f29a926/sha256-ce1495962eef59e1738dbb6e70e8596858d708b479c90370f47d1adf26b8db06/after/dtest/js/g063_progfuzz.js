var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { switch ((i ? ((!1) & b) : (f(2) < (Infinity ? f(1) : Infinity)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((({x:1}) >= ((-1 == undefined) !== (false > o.x)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { switch ((0.5 != (f(s) < 0.5))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((+f(1))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (((void (typeof null)) / (+({x:1})))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(([NaN, undefined] - ((arr.length === 1) !== [0.5, [1,2]]))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print((function(p) { return [1,2]; })([(({p: 1, q: "s"}) ? (NaN + 0) : [i, arr.length]), undefined])); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print((~([2, arr[0]] > ({p: b, q: arr.length})))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(arr.length); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((!0)); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((0 <= b) ? (true != d) : ([1,2] ? g : d)); })([0.5, ({x:1})])); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((~(f(f(1)) ^ (a ? "s" : true)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
