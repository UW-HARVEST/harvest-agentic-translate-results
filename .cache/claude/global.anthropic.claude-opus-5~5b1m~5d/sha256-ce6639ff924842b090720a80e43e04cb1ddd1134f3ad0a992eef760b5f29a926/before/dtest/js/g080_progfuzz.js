var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { do { try { try { print(({x:1})); } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } while (false); } catch (err) { print("TOP", err.name, err.message); }
try { print(i); } catch (err) { print("TOP", err.name, err.message); }
try { print(String((f((s * false)) < true)), typeof (({p: ((-1 !== 1) - (!null)), q: ((g ? 2 : []) != i)}))); } catch (err) { print("TOP", err.name, err.message); }
try { print((f([1,2]) ? (({p: Infinity, q: f(1)}) | (({}) >= 2)) : (({p: 1, q: Infinity}) | (undefined * o.x)))); } catch (err) { print("TOP", err.name, err.message); }
try { print(false); } catch (err) { print("TOP", err.name, err.message); }
try { print(((("s" >> "s") % ("s" << [])) !== [f(1), ({p: s, q: [1,2]})])); } catch (err) { print("TOP", err.name, err.message); }
try { try { b = (([2, NaN] == false) != f(0)); } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { print([1,2]); } catch (err) { print("TOP", err.name, err.message); }
try { a = (void ([({x:1}), -1] == f(true))); } catch (err) { print("TOP", err.name, err.message); }
try { print((~[0, (b ^ "s")])); } catch (err) { print("TOP", err.name, err.message); }
try { o.y = ""; print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { try { if ([undefined, ((~1) & ({p: arr.length, q: 2}))]) { d = (+((i * a) || (({x:1}) === b))); } else { print((f((-0.5)) | (f(0.5) ? (null != false) : false))); } } catch (e) { print("caught", e.name); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
