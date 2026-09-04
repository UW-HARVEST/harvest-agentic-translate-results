var d = new Date(0);
print(d.getTime(), d.valueOf(), d.toISOString(), d.toJSON());
print(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate(), d.getUTCDay(), d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds(), d.getUTCMilliseconds());
var ts = [0, 1, -1, 86400000, 1234567890123, -1234567890123, 951782400000, 2147483647000, 1e12, 8.64e15, 8.64e15+1];
for (var i = 0; i < ts.length; ++i) {
  var x = new Date(ts[i]);
  print(ts[i], x.getTime(), isFinite(x.getTime()) ? x.toISOString() : "Invalid", x.toUTCString());
}
print(Date.UTC(2000, 0, 1), Date.UTC(1970, 0, 1, 0, 0, 0, 0), Date.UTC(2000, 13, 40));
print(Date.parse("2000-01-01T00:00:00Z"), Date.parse("1970-01-01T00:00:00.000Z"), Date.parse("garbage"));
print(Date.parse("2011-10-10T14:48:00Z"), Date.parse("2011-10-10"));
var e = new Date(NaN);
print(e.getTime(), e.toString(), e.toUTCString());
var m = new Date(1234567890123);
m.setUTCMilliseconds(1); print(m.getTime());
m.setUTCSeconds(2); print(m.getTime());
m.setUTCMinutes(3); print(m.getTime());
m.setUTCHours(4); print(m.getTime());
m.setUTCDate(5); print(m.getTime());
m.setUTCMonth(6); print(m.getTime());
m.setUTCFullYear(2007); print(m.getTime());
m.setTime(999); print(m.getTime());
print(new Date(8.64e15).toISOString(), new Date(-8.64e15).toISOString());
print(typeof Date.now(), Date.now() > 1e12);
print(new Date(1) > new Date(0), new Date(5) - new Date(2));
var u = new Date(Date.UTC(2016, 1, 29, 23, 59, 59, 999));
print(u.getTime(), u.toISOString(), u.getUTCDay(), u.getUTCMonth());
print(new Date(Date.UTC(1900, 0, 1)).toISOString(), new Date(Date.UTC(2100, 11, 31)).toISOString());
