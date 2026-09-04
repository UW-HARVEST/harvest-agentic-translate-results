var vals = [0, -0, 1, -1, 0.5, -0.5, 1/3, 2/3, 1e21, 1e-7, 1e-6, 123456789012345678901234, 5e-324,
  1.7976931348623157e308, NaN, Infinity, -Infinity, 100, 1e6, 1e7, 0.1+0.2, 1e-10, 255, 65535,
  2147483647, -2147483648, 4294967295, 4294967296, 9007199254740993, 1234.5678e-30, 3.14159265358979,
  1e20, 1e-20, 12345.6789, -0.000001234, 1e300, 1e-300, 0.3333333333333333];
for (var i = 0; i < vals.length; ++i) {
  var v = vals[i];
  print(v, String(v), v.toString(), (typeof v));
  if (isFinite(v)) {
    print(' fixed:', v.toFixed(0), v.toFixed(2), v.toFixed(7));
    print(' exp:', v.toExponential(), v.toExponential(3));
    print(' prec:', v.toPrecision(1), v.toPrecision(5), v.toPrecision(21));
  }
  print(' radix:', (v).toString(2), (v).toString(8), (v).toString(16), (v).toString(36));
  print(' int:', parseInt(String(v)), parseFloat(String(v)), Number(String(v)), +v, -v, ~v, v|0, v>>>0);
}
print(parseInt("0x1f"), parseInt("  42abc"), parseInt("z", 36), parseInt("-17", 8), parseInt(""), parseInt("Infinity"));
print(parseFloat("3.14xyz"), parseFloat(".5e3"), parseFloat("-.5"), parseFloat("+Infinity"), parseFloat("abc"));
print(Number(""), Number(" 12 "), Number("0x10"), Number("1e3"), Number("Infinity"), Number("abc"), Number(null), Number(undefined), Number(true));
print(Number.MAX_VALUE, Number.MIN_VALUE, Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY);
print((1e21).toFixed(2), (0.000001).toFixed(7), (-1.5).toFixed(0), (1.005).toFixed(2));
print(0.1, 0.2, 0.3, 0.7, 1.1, 2.675, 1e-323, 4.35, 1.45, 1000000000000000128);
print((123.456).toFixed(1), (0).toFixed(2), (-0).toFixed(2), (1.999).toFixed(2));
