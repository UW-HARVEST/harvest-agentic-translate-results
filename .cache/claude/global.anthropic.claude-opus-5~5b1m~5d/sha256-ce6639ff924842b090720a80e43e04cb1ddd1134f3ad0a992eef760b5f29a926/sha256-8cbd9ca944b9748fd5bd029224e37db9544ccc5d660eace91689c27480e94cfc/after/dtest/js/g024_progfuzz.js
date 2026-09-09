var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((({}) === f(1))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((((typeof [1,2]) && [NaN, a]) > a)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { switch ([([1,2] != ({p: o.x, q: g})), (2 ? (void f(1)) : [undefined, true])]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { try { print(String(undefined), typeof (((f(d) - ("s" <= f(1))) ? ((true / undefined) >> (arr.length <= "s")) : (~(false & NaN))))); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((({p: (~arr.length), q: ({})}) + ({p: g, q: true}))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { d = 0.5; } catch (err) { print("TOP", err.name, err.message); }
try { print([({x:1}), (+(undefined === a))]); } catch (err) { print("TOP", err.name, err.message); }
try { a = i; } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f((+(+o.x)))); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(f(0.5)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print((({p: (i < 0), q: (0.5 & f(1))}) | ((undefined >>> true) >> ({p: b, q: true})))); } catch (err) { print("TOP", err.name, err.message); }
try { do { do { arr.push(((f(c) / f(Infinity)) & i)); print(arr.length, arr.join(",")); } while (false); } while (false); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
