var out = [];
try { out.push(String([1] >>> "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" , 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity << "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" != 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true << (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) << 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false / 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " | [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) || " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 === 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] || "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true & -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 <= "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 || "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 / "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >= 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 ^ 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 >>> "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 % 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null ^ 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) >> NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" > (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " & -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) === "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " >>> -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 / true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity / ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 - (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 >> false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null << "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false < 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 % true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined ^ (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 - (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " * undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 == 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 >> ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 >>> 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" > ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" + 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) != "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 < 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " & false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " * "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) === "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" ^ 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" | "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) / 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" , (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] ^ 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] === " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 * -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" - ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 | ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 > null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" >= 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" >> -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" < true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" & ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity >= 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] + 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" === "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 | "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" % "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 >= "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" && (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 , "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" * 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " & "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 | -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 == 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN / -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" , "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) == -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) >= "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" + false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" * 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 ^ 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) >>> -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" - ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " == "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 + 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 + 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" | [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 != " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" >>> true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 <= 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" , (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 >> 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 % -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" << 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 + null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) >= -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 === 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 <= "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 | -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 * undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 === [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 | "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 <= 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 >> 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 & undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 > "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(3.75 === [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" / "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" < 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 >>> [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) == undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] % true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 % 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 == 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) & 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) + 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 || (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] | 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 | "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 <= [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 == false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 || -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" * ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) == 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 && ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 <= 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 * Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] < "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" , 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 >>> 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" / 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 > -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 < 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] >> -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" == 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(3.75 == "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 == "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 / 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) >> null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" & "0")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 || "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 !== 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 < [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 - 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" && 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) < 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 > 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 , "0")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 != 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 >= 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) - -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 * undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] >>> "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) | 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 < "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" | "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 >> 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" === true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 && [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) << "0")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 / 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " !== "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 !== (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null && -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 != 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " == 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null % "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" && [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 > 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 >= 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) != "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 / "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined | 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) < -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 || 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 && [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" ^ Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 < 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity , 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 < -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true * 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 || -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" === "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 >= "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " - 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 == "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 >= (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(3.75 , 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 && "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" >> "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN , -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 !== null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) >> "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 >>> (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 * (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" / 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" && 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity % 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 / NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 + " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >>> [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 || (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" | ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" , null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 >>> "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >> -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 , "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity < 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" % 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 == false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 != "0")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false >= undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 ^ "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 >= 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) ^ ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" >>> false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 - ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) !== 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 & -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false !== 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) < "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 != Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 >= ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined / false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) == "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" , (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 === "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true << "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 < ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) <= null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) != 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] != -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 , "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 + 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true % ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) == (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" === "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 < 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" == "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " - "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) | [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" >>> ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] >= [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 != 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity | "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" < -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" , undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 !== [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " + 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false <= 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" === NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 && -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 && " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " | 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" < 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" === "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) == 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false - 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null === -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] % "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 < " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " * "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 * false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 * 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 > -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 === -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) + "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" && 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 >> "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity << [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 << 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] === "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 >= 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" >= 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 <= 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" === 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 >> 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) || false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 | 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 < 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity ^ "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) < -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null % 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" % " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) % (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 + "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 || "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 && "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" < 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" ^ -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined >= 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 - 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] != ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" === (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) / (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" , -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null * "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " <= ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] % "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 <= true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) > [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined == 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 - (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 % Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity <= 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) | "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 ^ 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 >= [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 > 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 | 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 - false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 * "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) <= (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" >= "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false << (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 || 65535)); } catch (e) { out.push("E:" + e.name); }
print(out.length); for (var i = 0; i < out.length; ++i) print(i, out[i]);
