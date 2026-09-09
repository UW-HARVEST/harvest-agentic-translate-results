var out = [];
try { out.push(String(!((function(){})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("abc"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("NaN"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(((/re/g))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(65535))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(9007199254740993))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(255))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("Infinity"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("abc"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (9007199254740993))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(({})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(65535))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1e-10))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(65535))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1e21))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(65535))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1e-10))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("Infinity"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((function(){})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("abc"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("abc"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(5e-324))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(({})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ((new String('s'))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("1e3"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(2147483647))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("  "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1e308))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("  "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((function(){})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~([]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (({})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("NaN"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(255))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("  "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("NaN"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(9007199254740993))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (9007199254740993))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1000000))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("Infinity"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ([1]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1e308))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e-10))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ((new Number(2))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (1e10))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(""))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((function(){})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~((function(){})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~((new Date(0))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(null))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (4294967295))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (1/3))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("  "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (255))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~((function(){})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(null))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(2147483647))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(-0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ([]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(1e308))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(" 12 "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e-10))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-([1]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (-0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(({})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(2147483647))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(-0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!([]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(3.75))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(1e21))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("0"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("abc"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (3.75))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ([1,2]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(0))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(1e-7))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-2147483648))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (false))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(123456.789))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((new String('s'))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(5e-324))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ([]))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(65535))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(undefined))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-((new Boolean(false))))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(255))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("NaN"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void (4294967296))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~("é"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(({a:1})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1e308))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof ("abc"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(({})))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-("中文"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(-1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(1e21))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(1e10))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(-Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(true))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("true"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(typeof (0.1))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("-1.5"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(~(NaN))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("0x10"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(Infinity))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!("NaN"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+("  "))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(!(2))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(5e-324))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(void ("1"))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(-(-0.5))); } catch (e) { out.push("E:" + e.name); }
try { out.push(String(+(false))); } catch (e) { out.push("E:" + e.name); }
print(out.length); for (var i = 0; i < out.length; ++i) print(i, out[i]);
