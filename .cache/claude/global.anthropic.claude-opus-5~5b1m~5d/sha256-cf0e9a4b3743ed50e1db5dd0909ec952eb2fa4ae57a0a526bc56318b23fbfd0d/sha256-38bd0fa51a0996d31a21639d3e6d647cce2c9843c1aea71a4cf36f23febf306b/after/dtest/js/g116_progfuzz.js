var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if (-1) { arr.push(arr.length); print(arr.length, arr.join(",")); } else { print([(({p: d, q: 0}) - (b !== b)), ([1,2] ? (!0.5) : f(1))]); } } catch (err) { print("TOP", err.name, err.message); }
try { d = ({p: (+(0 * 1)), q: ((Infinity !== arr.length) + "s")}); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((!"")) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print(c); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return ((s >>> "s") / ({p: [1,2], q: "s"})); })("s")); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(([1,2] <= [0, NaN]))); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { arr.push(0); print(arr.length, arr.join(",")); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((function(p) { return [[], ({x:1})]; })(("" && (f(arr.length) ? (arr.length + a) : b)))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (f(i) || f(2)); })(0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { do { b = ((typeof f(arr[0])) ? i : ((~2) <= (void -1))); } while (false); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (var k in (f((2 >>> 2)) >>> ({p: [-1, g], q: [1,2]}))) print("k", k); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((("s" <= (null != -1)) !== ({p: (true === g), q: 0}))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
