var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { print([(undefined % (undefined ? false : d)), ([d, NaN] / [])]); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(String(d), typeof ((void ({p: ({p: [1,2], q: s}), q: (false & o.x)})))); } } catch (err) { print("TOP", err.name, err.message); }
try { print(2); } catch (err) { print("TOP", err.name, err.message); }
try { print(2); } catch (err) { print("TOP", err.name, err.message); }
try { try { try { print([(-Infinity), (~(0.5 ? [1,2] : "s"))]); } catch (e) { print("caught", e.name); } } catch (e) { print("caught", e.name); } } catch (err) { print("TOP", err.name, err.message); }
try { arr.push(({p: (-1 - (1 ? arr[0] : ({}))), q: g})); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { print(f(((false - "s") ? 1 : 2))); } catch (err) { print("TOP", err.name, err.message); }
try { a = [true, (("s" ? ({}) : 0) % null)]; } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return -1; })(arr[0])); } catch (err) { print("TOP", err.name, err.message); }
try { arr.push((void 0)); print(arr.length, arr.join(",")); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(2); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((~(s ? f(1) : ({p: arr[0], q: a})))) { do { print(((false % (~s)) ? ((o.x ? g : arr.length) > ({p: Infinity, q: 2})) : (!({x:1})))); } while (false); } else { arr.push(((i !== (arr[0] >= "s")) | [[c, true], (({x:1}) > i)])); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
