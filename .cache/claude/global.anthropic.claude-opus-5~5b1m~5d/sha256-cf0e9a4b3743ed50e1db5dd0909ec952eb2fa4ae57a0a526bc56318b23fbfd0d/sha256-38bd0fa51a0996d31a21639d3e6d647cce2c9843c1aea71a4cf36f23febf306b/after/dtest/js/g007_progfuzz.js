var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { if (-1) { do { for (i = 0; i < 3; ++i) { arr.push((((~a) >= (false && f(1))) * ((a ? g : -1) > ({x:1})))); print(arr.length, arr.join(",")); } } while (false); } else { print((true >>> (s ? null : 2))); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(f(0.5)); } } catch (err) { print("TOP", err.name, err.message); }
try { if ((+1)) { d = (((undefined <= 0.5) - (-1 != NaN)) <= f([Infinity, c])); } else { a = (b || (arr.length && (0.5 + s))); } } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; if ((typeof [])) { if (i) { print((f([1,2]) > ([] || ""))); } else { d = f(({p: (f(1) + "s"), q: (null % NaN)})); } } else { arr.push(([[[1,2], true], ""] >> (typeof (NaN <= 2)))); print(arr.length, arr.join(",")); } } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { for (i = 0; i < 3; ++i) { print((1 >= ((d & []) ^ (~f(1))))); } } } catch (err) { print("TOP", err.name, err.message); }
try { print(s); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { arr.push(({p: (1 ? ([] - a) : (c ? 0 : 0.5)), q: ({})})); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { print((NaN === (null + f(1)))); } catch (err) { print("TOP", err.name, err.message); }
try { d = (f(1) >= (typeof (false >>> ""))); } catch (err) { print("TOP", err.name, err.message); }
try { print(([g, ({p: 1, q: o.x})] ? ((2 < arr[0]) + [undefined, Infinity]) : (2 & [1,2]))); } catch (err) { print("TOP", err.name, err.message); }
try { print(("" >= ((f(1) ? false : arr.length) < ([1,2] | [])))); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { print(String((1 ? [(({}) >= [1,2]), ({})] : (g > [({}), arr.length]))), typeof (([({}), (0.5 == c)] + ((~2) >>> arr[0])))); } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
