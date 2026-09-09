var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; switch ((({p: ({p: d, q: g}), q: (c + "")}) ? g : [(-1 < false), (undefined ? ({x:1}) : 1)])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } } catch (err) { print("TOP", err.name, err.message); }
try { print(false); } catch (err) { print("TOP", err.name, err.message); }
try { b = ({p: o.x, q: ((0.5 < arr[0]) >= (f(1) & f(1)))}); } catch (err) { print("TOP", err.name, err.message); }
try { print((!0)); } catch (err) { print("TOP", err.name, err.message); }
try { d = (({p: (-undefined), q: (false || undefined)}) / ((-1 ? true : true) % [NaN, s])); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = [1,2]; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { do { if ((+d)) { if ((!0)) { for (i = 0; i < 3; ++i) { print((-(~(Infinity == [1,2])))); } } else { b = i; } } else { a = ([(!true), (true && 1)] ^ f(-1)); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(arr[0]); } catch (err) { print("TOP", err.name, err.message); }
try { print(f((([] | i) === ({p: false, q: Infinity})))); } catch (err) { print("TOP", err.name, err.message); }
try { try { switch (({p: ({p: ({p: i, q: a}), q: ([] ^ 2)}), q: ((({x:1}) >> arr.length) < (({}) << -1))})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { print((void (f(arr[0]) <= (false / null)))); } catch (err) { print("TOP", err.name, err.message); }
try { try { if (g) { print([(!(!NaN)), g]); } else { for (i = 0; i < 3; ++i) { switch (({p: ["", (~true)], q: (typeof (a != d))})) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
