var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f(((~[]) === "s"))); } catch (err) { print("TOP", err.name, err.message); }
try { d = (([null, true] / (arr.length === 0)) ? ((g + "") ^ ("s" >>> [1,2])) : ({p: undefined, q: ["s", []]})); } catch (err) { print("TOP", err.name, err.message); }
try { do { do { try { switch (f([(g || ({x:1})), (b < c)])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("caught", e.name); } } while (false); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((([2, false] ? (s == undefined) : (d | a)) << "s")); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(((([] / -1) || (+-1)) >>> (f(f(1)) / ({p: f(1), q: "s"})))); } catch (err) { print("TOP", err.name, err.message); }
try { a = c; } catch (err) { print("TOP", err.name, err.message); }
try { print((((a ? a : 1) >= (f(1) >>> "")) < (({}) | [({}), Infinity]))); } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: c, q: (NaN ? "s" : undefined)}) != [])); } catch (err) { print("TOP", err.name, err.message); }
try { c = c; } catch (err) { print("TOP", err.name, err.message); }
try { a = [((2 >= ({x:1})) !== (0.5 || c)), f(f(arr.length))]; } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (1 ? s : NaN), q: (c || ({x:1}))}) ? (2 ? i : ([1,2] == "s")) : (!2))); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; o.y = b; print(JSON.stringify(o)); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
