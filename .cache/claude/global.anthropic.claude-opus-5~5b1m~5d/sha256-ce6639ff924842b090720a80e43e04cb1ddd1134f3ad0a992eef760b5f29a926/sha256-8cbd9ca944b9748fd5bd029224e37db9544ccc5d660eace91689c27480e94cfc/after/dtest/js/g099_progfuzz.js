var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print(f(s)); } catch (err) { print("TOP", err.name, err.message); }
try { print(((~({p: "", q: ({})})) || (f(a) >> ("" !== [])))); } catch (err) { print("TOP", err.name, err.message); }
try { do { for (i = 0; i < 3; ++i) { print((f((false / arr.length)) >= ((-[]) ? f(({x:1})) : (!undefined)))); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; try { if (0.5) { try { print((({p: f(i), q: ({p: "s", q: null})}) < (f(-1) && (a !== arr[0])))); } catch (e) { print("caught", e.name); } } else { arr.push(((({p: Infinity, q: undefined}) >>> (0 === true)) ? (({}) !== (~g)) : -1)); print(arr.length, arr.join(",")); } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
try { c = (((2 ? 0 : i) < [o.x, NaN]) != (([] ? null : Infinity) != (({}) << [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { c = ((-"s") ? "" : (c > (+c))); } catch (err) { print("TOP", err.name, err.message); }
try { print(0); } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return (s | (Infinity - false)); })((typeof (!(!Infinity))))); } catch (err) { print("TOP", err.name, err.message); }
try { print((((NaN - "") ? (2 || i) : (0.5 >> -1)) ? ({}) : false)); } catch (err) { print("TOP", err.name, err.message); }
try { b = [[(NaN >> ""), [NaN, a]], true]; } catch (err) { print("TOP", err.name, err.message); }
try { print((f((typeof arr[0])) | ((+c) % c))); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push([NaN, s]); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
