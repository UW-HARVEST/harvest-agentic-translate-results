print(Math.E, Math.LN10, Math.LN2, Math.LOG2E, Math.LOG10E, Math.PI, Math.SQRT1_2, Math.SQRT2);
var vals = [0, -0, 1, -1, 0.5, -0.5, 2, 10, 100, -100, 1e10, 1e-10, 0.1, NaN, Infinity, -Infinity, 3.7, -3.7, 2.5, -2.5, 0.49999999999999994];
for (var i = 0; i < vals.length; ++i) {
  var v = vals[i];
  print(v, Math.abs(v), Math.ceil(v), Math.floor(v), Math.round(v), Math.sqrt(v));
  print("  ", Math.exp(v), Math.log(v), Math.sin(v), Math.cos(v), Math.tan(v));
  print("  ", Math.asin(v), Math.acos(v), Math.atan(v), Math.atan2(v, 2), Math.atan2(2, v));
  print("  ", Math.pow(v, 2), Math.pow(2, v), Math.pow(v, 0.5), Math.pow(v, -1));
}
print(Math.max(), Math.min(), Math.max(1,2,3), Math.min(1,2,3), Math.max(1,NaN), Math.min("2",3));
print(Math.max(-0,0), Math.min(-0,0), 1/Math.max(-0,0), 1/Math.min(-0,0));
print(Math.pow(0,0), Math.pow(-1,0.5), Math.pow(Infinity,0), Math.sqrt(-1));
print(typeof Math.random(), Math.random() >= 0 && Math.random() < 1);
print(5 % 3, -5 % 3, 5 % -3, 5.5 % 2, 0/0, 1/0, -1/0, 0*Infinity);
print(1e308*10, -1e308*10, (0.1+0.2)*10, 7/3);
print(1<<31, 1<<32, -1>>>0, -1>>1, 5&3, 5|3, 5^3, ~5, 1e10|0, NaN|0, Infinity|0);
print(Math.round(-0.5), Math.round(0.5), Math.round(-1.5), Math.round(4503599627370497));
print(Math.floor(-0), 1/Math.floor(-0), Math.ceil(-0.5), 1/Math.ceil(-0.5));
