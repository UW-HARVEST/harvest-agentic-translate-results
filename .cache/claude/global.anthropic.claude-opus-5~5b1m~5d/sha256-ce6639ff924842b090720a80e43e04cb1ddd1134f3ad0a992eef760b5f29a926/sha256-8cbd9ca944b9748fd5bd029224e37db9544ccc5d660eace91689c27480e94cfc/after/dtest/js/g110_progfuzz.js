var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print([]); } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ((f(true) == (f(1) ^ i)) ^ [0, (Infinity * [1,2])])) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(({p: 0, q: f(2)})); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { do { arr.push(f(false)); print(arr.length, arr.join(",")); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(NaN); } catch (err) { print("TOP", err.name, err.message); }
try { d = ({p: (void i), q: [({p: g, q: Infinity}), (({x:1}) ? [] : 0)]}); } catch (err) { print("TOP", err.name, err.message); }
try { print((((true & s) !== ([] * "s")) * 0.5)); } catch (err) { print("TOP", err.name, err.message); }
try { print((((c + g) ? NaN : c) === "")); } catch (err) { print("TOP", err.name, err.message); }
try { switch (NaN) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((0.5 >> [c, (b ? Infinity : NaN)])); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print((false | f(o.x))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([(f(1) && (i ? true : s)), o.x]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
