var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { o.y = (f(("" ^ ({x:1}))) > 0.5); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { if (-1) { if (({p: f((undefined < 2)), q: (+(arr[0] ? false : b))})) { print((function(p) { return 0; })((((d * [1,2]) ? (b ? undefined : arr.length) : f(false)) * f(("" == [1,2]))))); } else { for (i = 0; i < 3; ++i) { print((function(p) { return NaN; })(Infinity)); } } } else { print(String((+(~true))), typeof ([((f(1) / a) < (+g)), [({x:1}), ({p: "s", q: undefined})]])); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(((typeof 1) >= [(!f(1)), (f(1) != ({}))])); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((((g >= "s") * (-1 ? 1 : 0.5)) / b)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { print(({x:1})); } catch (err) { print("TOP", err.name, err.message); }
try { c = ({p: arr[0], q: a}); } catch (err) { print("TOP", err.name, err.message); }
try { print(s); } catch (err) { print("TOP", err.name, err.message); }
try { print((([d, i] >> Infinity) <= ((i < undefined) % (false << 0.5)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { print(({p: [(undefined * a), a], q: "s"})); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { do { print((({}) || f((false % c)))); } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print([[(!"s"), f(true)], (({p: false, q: s}) ? -1 : d)]); } catch (err) { print("TOP", err.name, err.message); }
try { c = (g || -1); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
