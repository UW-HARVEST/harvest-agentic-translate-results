var out = [];
try { out.push(String(65535 + "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 - ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" >> null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) / "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) << [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" % 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 < 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) | 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" + "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 & (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 != (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 >> "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" || -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 * 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 <= ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" >>> "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" != 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 != "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) ^ ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 == "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true >> (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 < true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 / 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" & (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" | ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] >>> false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity <= 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined <= [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) | 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 & 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" + -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false , 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" + (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) <= null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null % 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 || "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 + NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 !== undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " < "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN != true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 != (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 <= (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) | NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) + (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity >>> -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) | -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] - 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false | (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 | 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" === null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" != -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 < -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 / (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) / false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 & "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" != ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) << "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 !== Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 > 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" / Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 < "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" / 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) || 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 != 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) << -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) ^ "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] >= -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 != "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) != 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) << [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null + "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] && 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false <= ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 && 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 <= ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 == [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" - "0")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 << undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 >>> "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 != ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity & "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) << 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) >>> -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 > "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" == true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 & 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" > ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] | (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 & 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 % ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 | -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] & false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] ^ ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] == [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 * 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 < 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" || "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " && 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 >= [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" , 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 >= 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" >= -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 > (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" + true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) || 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false % 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 , 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 << 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) * true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" >>> ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 !== 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) / false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" & "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 / undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) >> false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 , 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 - 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN << 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) && true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] || Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" / "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " !== true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 === null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) !== true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false , 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) + 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 | 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 + 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN === 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] + (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 || 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 & "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] ^ " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 / " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 >>> "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] !== -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] / 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " & 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 === "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 != 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" + true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" != "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 > (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 , "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 >> 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 != ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) == undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 < "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 == 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false && 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) * [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) * (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] << 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 !== (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 << 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) !== [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) == ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 / -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(3.75 * "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity % [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 <= undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN < "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] || 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 <= 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 !== ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 | 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >> -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" > "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 > 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) | 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" | 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 === 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) >= "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] && [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 >> 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" + 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) / 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 >= (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " || 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 , " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true | 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " * -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 >= -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) + 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " << "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 == 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 >>> 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 < 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) === (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] === 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN & 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" === 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) | -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" === "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) * false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" + undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" ^ undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 == 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" , NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) >> [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 == -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) >> 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" % (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] === 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 + (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 >>> (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 - (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 << [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 && 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 * (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 == 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 | 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 | (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 >= (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" % 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) , 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 !== "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 !== 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) + -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true && -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 + -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 & 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) < 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 - (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) != "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" ^ 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) > "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity >= " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" >> 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" == null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 || (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 * null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 >>> (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) << 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) % ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] * 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 % "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 - 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" == 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 & Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false >>> null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " == -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 < "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] == Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" % "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 >> " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false === [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >> [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " & -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 >> -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 || 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 !== ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] >= -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 ^ 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 <= Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 - -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" % ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 || "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null >> 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 - [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >>> 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) << "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" | 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " < 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" * -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" * "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN , ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 / 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null >= [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 != 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null > -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) / 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) !== " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) & undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) , (function(){}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] + 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) - "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" ^ "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " | -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 && ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 >>> ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 || ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 == "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 | -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] / [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) << (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) << (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" >= "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity === null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 !== 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) * 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" === "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) | 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" == "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 * 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 >= 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 % -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] <= undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 & "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true ^ 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 >> 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" , "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 / -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false >= 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " & -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false - 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) || "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " >>> 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" / [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] >>> "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" << 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) * 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 - 0.1)); } catch (e) { out.push("E:" + e.name); }
print(out.length); for (var i = 0; i < out.length; ++i) print(i, out[i]);
