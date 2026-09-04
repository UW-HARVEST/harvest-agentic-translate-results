var out = [];
try { out.push(String((-0.5).toExponential(20))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((1e308).toExponential())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((5e-324).toExponential(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((1/3).toExponential(36))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((2147483647).toString(2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((1e-7).toString(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((-0.5).toFixed(20))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((1/3).toPrecision(20))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((1/3).toPrecision(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").lastIndexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").localeCompare(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").indexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").localeCompare(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").split((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").toUpperCase("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").charCodeAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").search(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").toUpperCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").match(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").indexOf((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").concat("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").lastIndexOf((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").substring((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").lastIndexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").toLowerCase((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").charCodeAt((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").toUpperCase("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").toLowerCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").lastIndexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").match())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").slice(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").localeCompare())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").indexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").split(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").trim(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charCodeAt(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").localeCompare((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").replace(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").match((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").substring(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").match(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").match("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").split((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charAt("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").charAt(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").match("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").indexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").toUpperCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").split(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").trim((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").trim((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").slice(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").charAt(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").slice(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").charAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").concat((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").search(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").substring())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").match(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").localeCompare("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").toLowerCase("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").toUpperCase())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").toUpperCase(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").replace(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").localeCompare("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").charCodeAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charCodeAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").indexOf("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").replace((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").toLowerCase())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").concat(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").match(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").toLowerCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").toLowerCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").charAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").search(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").charCodeAt(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").split("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").substring("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").toUpperCase((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").charAt("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").substring(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").charCodeAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").match(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").match(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").replace("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").replace(((/a/))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").lastIndexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").split((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").match((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").toLowerCase((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").search("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").charAt(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").slice(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").toLowerCase("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").toLowerCase("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").toUpperCase((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").lastIndexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").substring(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").toLowerCase())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").search(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").slice())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").lastIndexOf())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").concat(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").substring("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").replace("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").match(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").charAt())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").replace(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").search(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").split(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").replace((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").charAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").trim(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").charCodeAt())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").replace(((/a/))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").indexOf())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").toLowerCase((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").concat("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").toLowerCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").substring(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").substring((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").indexOf())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").search((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charAt("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").charAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").toLowerCase(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").localeCompare(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").trim((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").split("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").charCodeAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").trim((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").lastIndexOf("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").indexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").toLowerCase())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").replace("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").substring("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").replace(((/a/))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").toUpperCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").charCodeAt("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").split((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").localeCompare(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").charAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").concat())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").charCodeAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charCodeAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").localeCompare(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").match(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").toLowerCase((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").match(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").search(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").charAt((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").indexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").charAt("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").split((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").localeCompare((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").indexOf("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").charCodeAt(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charCodeAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").slice())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").lastIndexOf("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").charCodeAt(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").concat((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").lastIndexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").toUpperCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").charCodeAt())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").concat(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").indexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").toLowerCase((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").toLowerCase((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").toUpperCase("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").slice("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").charAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").match((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").search((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").toUpperCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").toLowerCase((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").indexOf((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").search("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").trim())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").trim((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").search(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").match(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").charAt())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").indexOf((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").split(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").lastIndexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").search((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").toUpperCase(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").substring(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").slice())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").localeCompare("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").charAt(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").slice(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").split())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").slice("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").lastIndexOf("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").search("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").toLowerCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").search(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").match())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").charAt())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").toUpperCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").localeCompare("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").toUpperCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").slice(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").replace("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").charCodeAt((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").substring((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").lastIndexOf((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").slice(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").indexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").localeCompare((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").substring(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").indexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").localeCompare("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").replace(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").replace("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").concat(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").indexOf("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").localeCompare())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").charAt(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").indexOf(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").toUpperCase((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").toLowerCase(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").substring("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").toUpperCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").replace("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").search(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").substring(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").charAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").slice(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").split("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").localeCompare((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").substring((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").charAt(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").lastIndexOf("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").toUpperCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").split((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").match("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").split((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").match())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").toUpperCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").lastIndexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").concat(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").localeCompare(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").replace(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("Infinity").toLowerCase("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").indexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").charAt(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").concat((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").toLowerCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").search(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").search())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").search("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").replace("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").trim(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").substring((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").substring(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").charAt((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").substring())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").search())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").toLowerCase())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").substring(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("").match((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").toUpperCase(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1e3").charCodeAt(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").lastIndexOf((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("abc").match((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((" 12 ").localeCompare((/a/)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("中文").replace())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("1").replace("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").charAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("true").slice("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").lastIndexOf(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0").charAt((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").split(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("NaN").slice(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").toUpperCase(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("  ").lastIndexOf())); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").concat((/./g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("é").indexOf(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("-1.5").lastIndexOf("a"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(("0x10").match((/a/)))); } catch (e) { out.push("E:" + e.name); }
print(out.length); for (var i = 0; i < out.length; ++i) print(i, out[i]);
