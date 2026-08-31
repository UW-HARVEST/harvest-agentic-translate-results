var out = [];
try { out.push(String(2 , " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 < "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null === -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 + Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" - (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 ^ (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 == "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" == 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 << 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 === NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" ^ "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] - 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" & 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" <= (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 <= false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" & (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 + NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity >= false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 == true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 !== 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) == 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" !== undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" & "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) >= 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity * (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) >= [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] || ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true << (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 ^ 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) != 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " + 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] >>> 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity && 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 * 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 - 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 & false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 / 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true + 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 == null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) ^ 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 || 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) === 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " * "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" + "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 || 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 % NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 << 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(3.75 | null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) != 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 || NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" != ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 , (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) >> -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 && "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 != ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" < 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " - "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] - "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("é" | 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0x10" & 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" % undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 >> "0")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" >>> 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 == "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " >> (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 % [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" <= 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 | NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 , 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("  " < 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) < 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN , NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" == " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 < "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" / -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 && -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) ^ "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 !== "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) || 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " | 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 <= "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" & null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" >> 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) >> -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true >> false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 ^ 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 || 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" >= "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("中文" & [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 || 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 ^ (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 >>> -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(NaN >= (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 != "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) !== 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) > ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 != "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 > "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined / (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 ^ 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" / "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 << "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" !== 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 % " 12 ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 || "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] || "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("abc" | (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" | (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true === "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 << 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] !== true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 - [1,2])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " ^ 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 <= (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] >>> 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 === 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) && 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({a:1}) || "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false % 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 << "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 === false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 || 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 || "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 | "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false , 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity >= "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" != "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] | 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 <= -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true / 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0 > 0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 * Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] / 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 , 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) >= 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" | 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 === 1e308)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" / "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" >> 1e10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 <= (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 + false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 !== "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 >> -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 >>> 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 * "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" != ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 != "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("true" < 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity , "é")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 && 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" > 4294967295)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(undefined != -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 || (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Date(0)) & Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 <= 4294967296)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e308 !== 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) === "")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 & 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.5 >> false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 << "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("-1.5" === 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new String('s')) != 1e21)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 > 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 >>> "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 >>> 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true == 3.75)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 || 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-7 <= 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 >> "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null / [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(3.75 / 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" !== 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(123456.789 , 2)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 ^ true)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2147483647 >> (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 , (new Date(0)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] + false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e10 <= 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 | (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] + 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] % "1e3")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" | 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 ^ "NaN")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 & 0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] && "  ")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 ^ -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 !== 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 / null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(255 / 1e-7)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 + -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) < ({a:1}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) != "true")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e-10 * NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 & 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" >>> "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 === [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 >> [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) !== 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 , -0)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(Infinity > Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(null || "1")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 / -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 == false)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1e21 >> 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Number(2)) === [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 & 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity ^ "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" !== "0x10")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 << 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 !== 1e-10)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("0" >>> 1000000)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967296 / 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(65535 / (new Number(2)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true , -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true && "abc")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0 >> "Infinity")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(((/re/g)) & "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" > [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" | 5e-324)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(9007199254740993 !== (new String('s')))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-Infinity + 9007199254740993)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(({}) < (new Boolean(false)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1] === Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(4294967295 & 0.1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] !== "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1" , -Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([] && 1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("" * "中文")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 , -0.5)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("NaN" ^ -2147483648)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(2 / Infinity)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("1e3" , [])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(true * NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 ^ 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-0.5 || NaN)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((new Boolean(false)) , ({}))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(0.1 >>> "-1.5")); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1 == [1])); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(false >= 2147483647)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-2147483648 === null)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1/3 + 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(5e-324 & -1)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String([1,2] >> 123456.789)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-1 , 1/3)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String((function(){}) | 65535)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String("Infinity" | 255)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(1000000 < ((/re/g)))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(" 12 " >= undefined)); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(255))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("0"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(((/re/g))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (3.75))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e21))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (((/re/g))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("NaN"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(5e-324))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(((/re/g))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(null))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (5e-324))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("  "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(null))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-([1]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(((/re/g))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ([]))); } catch (e) { out.push("E:" + e.name); }
print(out.length); for (var i = 0; i < out.length; ++i) print(i, out[i]);
