var v = { n: 1.5, s: "a\"b\\c\n\t", b: true, nu: null, a: [1,[2,{x:3}]], o: {} };
print(JSON.stringify(v));
print(JSON.stringify(v, null, 2));
print(JSON.stringify(v, null, "\t"));
print(JSON.stringify(v, ["n","s"]));
print(JSON.stringify(v, function(k, val) { return typeof val === "number" ? val * 2 : val; }));
print(JSON.stringify([undefined, function(){}, NaN, Infinity, -0]));
print(JSON.stringify("plain"), JSON.stringify(5), JSON.stringify(null), JSON.stringify(true));
print(JSON.stringify(undefined), JSON.stringify(function(){}));
var p = JSON.parse('{"a":[1,2,{"b":"c"}],"d":null,"e":1e3,"f":-0.5,"g":"\\u00e9\\n"}');
print(p.a[2].b, p.d, p.e, p.f, p.g, JSON.stringify(p));
print(JSON.stringify(JSON.parse('[1,2,3]')));
print(JSON.parse('{"a":1}', function(k, val) { return typeof val === "number" ? val + 100 : val; }).a);
try { JSON.parse("{bad}"); } catch (e) { print(e.name, e.message); }
try { JSON.parse("[1,]"); } catch (e) { print(e.name, e.message); }
try { var c = {}; c.self = c; JSON.stringify(c); } catch (e) { print(e.name, e.message); }
print(JSON.stringify({ toJSON: function() { return "custom"; } }));
print(JSON.stringify(new Date(0)) !== undefined);
repr(v);
repr([1,"two",{a:null}]);
repr(function(a,b){return a+b;});
repr("stré");
repr(new Error("e"));
repr(/re/g);
repr([[[[1]]]]);
