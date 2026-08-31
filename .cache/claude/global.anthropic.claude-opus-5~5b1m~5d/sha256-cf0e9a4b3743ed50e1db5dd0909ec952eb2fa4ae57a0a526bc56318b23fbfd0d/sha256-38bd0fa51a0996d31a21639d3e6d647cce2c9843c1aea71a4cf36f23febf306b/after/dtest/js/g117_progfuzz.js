var a = 1, b = "two", c = null, d = [1,2,3], i = 0, s = "str";
var o = { x: 5 };
var arr = [1,2];
function f(v) { return v; }
function g() { return 42; }
try { for (i = 0; i < 3; ++i) { arr.push(f([(1 != -1), (s / a)])); print(arr.length, arr.join(",")); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (f(-1)) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { o.y = (f((2 ? a : g)) <= (!(-[1,2]))); print(JSON.stringify(o)); } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { b = (b || ({p: arr.length, q: (undefined > a)})); } } catch (err) { print("TOP", err.name, err.message); }
try { for (i = 0; i < 3; ++i) { i = 0; while (i < 2) { ++i; print((0.5 & "s")); } } } catch (err) { print("TOP", err.name, err.message); }
try { a = (f((g ? Infinity : c)) ? b : undefined); } catch (err) { print("TOP", err.name, err.message); }
try { i = 0; while (i < 2) { ++i; switch ((s != ((a > ({x:1})) >= f(2)))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } } catch (err) { print("TOP", err.name, err.message); }
try { print((function(p) { return f(([] == 2)); })([((2 * 2) <= (true * 2)), ({p: c, q: ([] ? f(1) : undefined)})])); } catch (err) { print("TOP", err.name, err.message); }
try { try { for (var k in [({p: "", q: [[], "s"]}), [("s" !== c), true]]) print("k", k); } catch (e) { print("c", e.name); } finally { print("fin"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch ((true % [(o.x > null), (+1)])) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { switch (([({p: "s", q: undefined}), [o.x, i]] ? d : ((c <= true) === [g, arr.length]))) { case 1: print("one"); break; case "s": print("s"); default: print("def"); } } catch (err) { print("TOP", err.name, err.message); }
try { if (arr.length) { for (var k in (~f((typeof [])))) print("k", k); } else { try { print([(("s" >> NaN) + (o.x ? b : true)), ([d, ({x:1})] << ({p: [], q: arr.length}))]); } catch (e) { print("c", e.name); } finally { print("fin"); } } } catch (err) { print("TOP", err.name, err.message); }
print("end", a, b, c, i, arr.length, JSON.stringify(o));
