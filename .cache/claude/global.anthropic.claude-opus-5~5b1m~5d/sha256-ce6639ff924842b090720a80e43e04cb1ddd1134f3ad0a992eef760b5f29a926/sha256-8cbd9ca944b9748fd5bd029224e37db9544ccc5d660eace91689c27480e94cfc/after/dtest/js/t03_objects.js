var o = { a: 1, b: "two", c: [1,2,3], d: { nested: true }, 5: "five", "with space": 1 };
for (var k in o) print("key", k, "=", o[k]);
print(Object.keys(o), Object.getOwnPropertyNames(o));
print(o.hasOwnProperty("a"), o.hasOwnProperty("toString"), o.propertyIsEnumerable("a"));
print(JSON.stringify(Object.getOwnPropertyDescriptor(o, "a")));
var p = Object.create(o, { x: { value: 9, enumerable: true } });
print(p.a, p.x, Object.getPrototypeOf(p) === o, o.isPrototypeOf(p));
Object.defineProperty(p, "ro", { value: 1, writable: false, enumerable: false, configurable: false });
print(p.ro); p.ro = 2; print(p.ro);
var g = { get v() { return 42; }, set v(x) { this.got = x; } };
print(g.v); g.v = 7; print(g.got);
Object.defineProperties(g, { q: { get: function() { return "Q"; } } });
print(g.q);
print(Object.isExtensible(g), Object.isSealed(g), Object.isFrozen(g));
Object.seal(g); print(Object.isExtensible(g), Object.isSealed(g));
Object.freeze(g); print(Object.isFrozen(g));
var pe = {}; Object.preventExtensions(pe); pe.nope = 1; print(pe.nope, Object.isExtensible(pe));
function Base() { this.base = 1; }
Base.prototype.hello = function() { return "hello " + this.base; };
function Derived() { Base.call(this); this.derived = 2; }
Derived.prototype = new Base();
Derived.prototype.constructor = Derived;
var d = new Derived();
print(d.hello(), d instanceof Derived, d instanceof Base, d.constructor === Derived);
print(({}).toString(), [].toString(), (function(){}).toString());
print(typeof null, typeof undefined, typeof 0, typeof "", typeof {}, typeof [], typeof print, typeof unknownvar);
print(delete o.a, o.a, delete o.nothere);
with (o) { print(b, c.length); }
var chain = Object.create(Object.create({ deep: "yes" }));
print(chain.deep, chain.hasOwnProperty("deep"), "deep" in chain);
var acc = {};
Object.defineProperty(acc, "both", { get: function(){ return this._v; }, set: function(v){ this._v = v * 2; }, enumerable: true });
acc.both = 21; print(acc.both, JSON.stringify(acc));
print(Object.getOwnPropertyNames([1,2]), Object.keys("abc"));
print(Object.getPrototypeOf([]) === Array.prototype, Object.getPrototypeOf(Object.prototype));
var sealed = Object.seal({ a: 1 }); sealed.a = 2; delete sealed.a; sealed.b = 3;
print(JSON.stringify(sealed));
var frozen = Object.freeze({ a: 1 }); frozen.a = 2; print(frozen.a);
