var pats = ["a", "a+", "a*b", "(a|b)c", "[a-z]+", "[^a-z]", "^abc$", "a{2,3}", "\\d\\w\\s\\S\\W\\D",
  "(?:x)y", "(?=a)a", "(?!a)b", "a.c", "[\\]]", "\\bword\\b", "(a)(b)(c)", "x|", "(a+)+b", "a??b", "[a-c]{1,}"];
var strs = ["a", "aa", "aab", "bc", "hello world", "ABC", "abc", "aaa", "1a \t", "xy", "b", "a.c", "]", "a word here", "", "aaab", "abcabc"];
for (var i = 0; i < pats.length; ++i) {
  var re;
  try { re = new RegExp(pats[i]); } catch (e) { print("compile error", pats[i], e.name, e.message); continue; }
  for (var j = 0; j < strs.length; ++j) {
    var m = re.exec(strs[j]);
    print(pats[i], "|", strs[j], "=>", m ? (m.length + ":" + m.join("+") + ":" + m.index) : "null", re.test(strs[j]));
  }
}
var g = /a/g;
print(g.exec("aaa"), g.lastIndex, g.exec("aaa"), g.lastIndex, g.exec("aaa"), g.lastIndex, g.exec("aaa"), g.lastIndex);
print(/A/i.exec("a"), /^a$/m.exec("b\na"), /a/gi.source, /x/g.global, /x/i.ignoreCase, /x/m.multiline, String(/x/gim));
print("aaa".replace(/a/g, function(m, o) { return o; }));
print("a-b_c".split(/[-_]/), "aXbXXc".split(/X+/), "abc".split(/(b)/));
print(new RegExp("a", "g").toString(), new RegExp(/x/g).source);
try { new RegExp("["); } catch (e) { print(e.name, e.message); }
try { new RegExp("a", "z"); } catch (e) { print(e.name, e.message); }
try { new RegExp("a{2,1}"); } catch (e) { print(e.name, e.message); }
try { new RegExp("(((((((((((((((((((((a)))))))))))))))))))))"); } catch (e) { print(e.name, e.message); }
try { new RegExp("a**"); } catch (e) { print(e.name, e.message); }
try { new RegExp("\\"); } catch (e) { print(e.name, e.message); }
print(/(a)|(b)/.exec("b"));
print("é中".match(/./g), /./.exec("é")[0].length);
print("aaa".search(/b/), "abc".match(/(a)(?:b)(c)/));
print("x1y22z".replace(/(\d+)/g, "[$1|$&|$$]"));
print("abc".replace(/(a)(b)(c)/, "$3$2$1$4$0"));
