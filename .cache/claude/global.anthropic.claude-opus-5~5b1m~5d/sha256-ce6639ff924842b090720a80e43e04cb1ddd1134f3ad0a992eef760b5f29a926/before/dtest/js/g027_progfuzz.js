var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print((~([true, true] != (true - [])))); } catch (err) { print("TOP", err.name, err.message); }
try { print(0.5); } catch (err) { print("TOP", err.name, err.message); }
try { if ((2 ? f(f(({x:1}))) : g)) { print(arr.length); } else { print((function(p) { return f(arr.length); })([(g <= f(undefined)), ((true > "s") === f(a))])); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((0 ^ ({x:1}))); } } catch (err) { print("TOP", err.name, err.message); }
try { try { o.y = [(f(a) ? "s" : (arr.length - [1,2])), f((+o.x))]; print(JSON.stringify(o)); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (var k in ((({p: a, q: ""}) ^ (NaN ? g : 0.5)) && (void a))) print("k", k); } catch (err) { print("TOP", err.name, err.message); }
try { print(([(!arr[0]), ({p: [1,2], q: d})] - (({p: [], q: s}) ^ false))); } catch (err) { print("TOP", err.name, err.message); }
try { print((void (!(+a)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(c); } catch (err) { print("TOP", err.name, err.message); }
try { switch ((([g, undefined] >> (b ? NaN : false)) == 2)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ([((void null) - (c == a)), ((false && false) & [a, Infinity])]) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print((("" && i) ^ ((i ? ({x:1}) : 0.5) << f(s)))); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
