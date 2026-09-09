function deep(n) { return n > 0 ? deep(n - 1) + 1 : 0; }
try { print(deep(100)); } catch (e) { print("100:", e.name, e.message); }
try { print(deep(400)); } catch (e) { print("400:", e.name, e.message); }
try { print(deep(100000)); } catch (e) { print("100000:", e.name, e.message); }
function deept(n) { try { return n > 0 ? deept(n - 1) : 0; } catch (e) { throw e; } }
try { print(deept(30)); } catch (e) { print("try30:", e.name, e.message); }
try { print(deept(200)); } catch (e) { print("try200:", e.name, e.message); }
var s = "x";
try { while (true) s = s + s; } catch (e) { print("string growth:", e.name, e.message, s.length > 0); }
var a = [];
try { a[100000] = 1; print("sparse ok", a.length); } catch (e) { print("sparse:", e.name, e.message); }
try { a.length = 4294967295; print("huge length ok"); } catch (e) { print("huge length:", e.name, e.message); }
try { new Array(1 << 30); print("big array alloc ok"); } catch (e) { print("big array:", e.name, e.message); }
var nest = "1";
for (var i = 0; i < 300; ++i) nest = "(" + nest + ")";
try { print(eval(nest)); } catch (e) { print("nest300:", e.name); }
var nest2 = "1";
for (var i = 0; i < 600; ++i) nest2 = "(" + nest2 + ")";
try { print(eval(nest2)); } catch (e) { print("nest600:", e.name); }
var manyvars = "";
for (var i = 0; i < 300; ++i) manyvars += "var v" + i + " = " + i + ";";
manyvars += "v299";
try { print(eval(manyvars)); } catch (e) { print("manyvars:", e.name, e.message); }
print("limits done");
