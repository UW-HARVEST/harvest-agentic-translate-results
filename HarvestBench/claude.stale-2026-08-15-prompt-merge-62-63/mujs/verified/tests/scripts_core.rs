//! Differential JavaScript corpus for the core language and the core builtins
//! (CONFIGS.md rows H1..H10).
//!
//! Every script is executed with `js_dostring` on a fresh state in the C library
//! and in the Rust translation; the return code and the whole captured output
//! (`print` lines plus `[report] ...` lines for uncaught errors) must be
//! byte-identical.
//!
//! Conventions used throughout:
//!   * scripts are small and single-purpose, so a divergence names the construct;
//!   * everything interesting is `print`ed, usually together with `typeof` and/or
//!     `JSON.stringify` so the *representation* is compared, not just the value;
//!   * deliberately-throwing snippets are normally wrapped in
//!     `try{...}catch(e){print('caught', e.name, e.message)}`, but a handful are
//!     left unwrapped on purpose so the top-level report path is compared too;
//!   * no clock, no `Math.random`, no `new Date()` - the corpus is deterministic.

mod common;
use common::*;

/* ================================================================== */
/*  H1 - every operator                                                */
/* ================================================================== */

#[test]
fn h1_operators() {
    diff_scripts_both_modes(&[
        /* ---- arithmetic ---- */
        r#"print(1+2, 3-4, 5*6, 7/8, 9%4);"#,
        r#"print(1/0, -1/0, 0/0, 0/0 === 0/0, typeof (0/0));"#,
        r#"print(-0, 1/-0, (-0).toString(), 0 === -0, String(-0));"#,
        r#"print(0*-1, 1/(0*-1), -0+-0, 1/(-0+-0), -0+0, 1/(-0+0));"#,
        r#"print(5%0, -5%3, 5%-3, -5%-3, 5.5%2.5, -5.5%2.5);"#,
        r#"print(Infinity%2, 2%Infinity, Infinity-Infinity, Infinity/Infinity, Infinity*0);"#,
        r#"print("a"+1, 1+"a", "3"*"4", "3"-"1", "3"/"1", "3"%"2");"#,
        r#"print([]+[], typeof ([]+[]), JSON.stringify([]+[]));"#,
        r#"print([]+{}, typeof ([]+{}));"#,
        r#"print({}+[], typeof ({}+[]));"#,
        r#"print(1+null, 1+undefined, "x"+null, "x"+undefined, null+"");"#,
        r#"print(true+true, true+"1", null+null, undefined+undefined, true+null);"#,
        r#"print(1+{}, ({})+1, [1,2]+[3], [1]-1, [2]*[3]);"#,
        r#"print(Number.MAX_VALUE*2, -Number.MAX_VALUE*2, Number.MIN_VALUE/2, Number.MAX_VALUE+Number.MAX_VALUE);"#,
        r#"print(9007199254740992+1, 9007199254740993, 1e308*10, 2e-308/1e10);"#,
        r#"var o={valueOf:function(){return 7}}; print(o+1, o*2, o+"x", o-1, ""+o);"#,
        r#"var o={toString:function(){return "T"}}; print(o+1, o*2, o+"x");"#,
        r#"var o={valueOf:function(){return {}},toString:function(){return "S"}}; print(o+1, o*2);"#,
        r#"var o={valueOf:function(){return {}},toString:function(){return {}}}; print(o+1);"#,
        r#"print(0.1+0.2, 0.1+0.2===0.3, (0.1+0.2).toString(), 0.3-0.1);"#,
        r#"print(1e21+1, 1e-7*1e-7, 123456789*987654321);"#,

        /* ---- bitwise ---- */
        r#"print(1&3, 1|2, 5^3, ~5, ~0, ~-1, ~~3.7);"#,
        r#"print(2147483647&-1, 2147483648|0, 4294967295|0, 4294967296|0, 4294967297|0);"#,
        r#"print(NaN|0, Infinity|0, -Infinity|0, undefined|0, null|0, ""|0);"#,
        r#"print(-2147483649|0, 1e21|0, 1e-7|0, 2.9|0, -2.9|0, -0|0);"#,
        r#"print(~NaN, ~Infinity, ~-Infinity, ~1e21, ~"5", ~"x", ~null, ~undefined);"#,
        r#"print(0xffffffff & 0xffffffff, 0xffffffff >>> 0, 0x80000000|0, 0x7fffffff|0);"#,
        r#"print(-1 & 0xff, 255 ^ 0xff00, (1<<30)|(1<<30), 12345678901234 & 0xffff);"#,
        r#"print(true&1, "3"&"1", [4]|0, ({})|0, [1,2]|0);"#,

        /* ---- shifts ---- */
        r#"print(1<<0, 1<<1, 1<<31, 1<<32, 1<<33, 1<<-1);"#,
        r#"print(-8>>0, -8>>1, -8>>31, -8>>32, -8>>33, -8>>-1);"#,
        r#"print(-8>>>0, -8>>>1, -8>>>31, -8>>>32, -8>>>33, -8>>>-1);"#,
        r#"print(3<<0, 3<<30, 3<<31, -1>>>0, -1>>>31, -1>>>32);"#,
        r#"print(NaN<<1, Infinity>>1, "8">>>1, null<<3, undefined>>>2, 2.9<<1);"#,
        r#"print(1<<"3", 1<<3.9, 1<<"x", 1e21>>>0, -1e21>>>0);"#,

        /* ---- comparison ---- */
        r#"print(1<2, 2<1, 1<=1, 2>=3, 1>1, 1>=1);"#,
        r#"print("a"<"b", "b"<"a", "abc"<"abd", ""<"a", "10"<"9", "A"<"a");"#,
        r#"print(1<"2", "1"<2, "10"<9, "a"<1, 1<"a", "2">"10");"#,
        r#"print(NaN<1, NaN>1, NaN<=1, NaN>=1, NaN<NaN, NaN>=NaN);"#,
        r#"print(null<1, null<=0, null>=0, undefined<1, undefined<=undefined, undefined>=0);"#,
        r#"print([]<[], [1]<[2], [1]<"2", [2]<[10], ({})<({}));"#,
        r#"print(true<2, false<1, true<=1, true>=1, false>=0, true>false);"#,
        r#"print(Infinity>1e308, -Infinity<-1e308, 0<Number.MIN_VALUE, -0<0, -0<=0);"#,
        r#"var o={valueOf:function(){return 5}}; print(o<6, o>6, o<=5, o>=5);"#,

        /* ---- equality ---- */
        r#"print(1==1, 1==="1", 1=="1", 1!="1", 1!=="1", 1!=2);"#,
        r#"print(null==undefined, null===undefined, null==0, undefined==0, null==false);"#,
        r#"print(NaN==NaN, NaN!=NaN, NaN===NaN, NaN!==NaN, -0==0, -0===0);"#,
        r#"print({}=={}, []==[], []=="", [0]==0, [1]==1, [[]]==0);"#,
        r#"print(new Number(1)==1, new Number(1)===1, new String("a")=="a", new Boolean(true)==true);"#,
        r#"var a={}; var b=a; print(a==b, a===b, a!=b, a!==b);"#,
        r#"print(true==1, true==="1", false==0, false=="", false==null, false==undefined);"#,
        r#"print("0"==false, ""==0, " "==0, "0x10"==16, "1e2"==100, "\n"==0);"#,
        r#"print(null==null, undefined==undefined, undefined===undefined, null===null);"#,
        r#"print("a"==="a", "a"=="a", "a"!=="b", typeof "a"==="string");"#,
        r#"print(print==print, print===print, Math==Math, [1]==="1", [1]=="1");"#,

        /* ---- logical & short circuit ---- */
        r#"var s=""; function f(x){s+=x; return x} print(f(0)&&f(1), s);"#,
        r#"var s=""; function f(x){s+=x; return x} print(f(1)||f(2), s);"#,
        r#"var s=""; function f(x){s+=x; return x} print(f(1)&&f(2)&&f(0)&&f(3), s);"#,
        r#"var s=""; function f(x){s+=x; return x} print(f(0)||f("")||f(7)||f(8), s);"#,
        r#"print(!0,!1,!"",!"a",!null,!undefined,!NaN,!{},![],!!"0");"#,
        r#"print(1&&2, 0||3, null&&1, undefined||"d", ""||0, NaN&&1, 0&&nosuch);"#,
        r#"print(!!Infinity, !!-0, !!"false", !!new Boolean(false), !![]);"#,

        /* ---- typeof / void ---- */
        r#"print(typeof undefined, typeof null, typeof true, typeof 1, typeof "s");"#,
        r#"print(typeof {}, typeof [], typeof print, typeof Math, typeof JSON);"#,
        r#"print(typeof nosuchvar, typeof typeof 1, typeof (void 0));"#,
        r#"print(typeof new Number(1), typeof new String("a"), typeof new Boolean(1));"#,
        r#"print(typeof /x/, typeof new Date(0), typeof Object, typeof Object.prototype);"#,
        r#"print(void 0, void 1, typeof void 0, void print(1), void "x");"#,

        /* ---- delete ---- */
        r#"var o={a:1,b:2}; print(delete o.a, o.a, delete o.zz, JSON.stringify(o));"#,
        r#"var a=[1,2,3]; print(delete a[1], a.length, JSON.stringify(a), 1 in a, a[1]);"#,
        r#"var a=[1,2,3]; print(delete a[5], delete a[-1], a.length, JSON.stringify(a));"#,
        r#"print(delete nosuch);"#,
        r#"var o={}; Object.defineProperty(o,"x",{value:1,configurable:false}); try{ print(delete o.x, o.x); }catch(e){ print("caught", e.name, e.message); }"#,
        r#"print(delete Math.PI, Math.PI, delete Object.prototype, typeof Object.prototype);"#,
        r#"var o={a:1}; print(delete o["a"], JSON.stringify(o), delete o[0]);"#,
        r#"print(delete 1, delete "a", delete (1+2));"#,

        /* ---- in / instanceof ---- */
        r#"var o={a:1}; print("a" in o, "b" in o, "toString" in o, "valueOf" in o);"#,
        r#"print(0 in [1], 1 in [1], "length" in [1], "0" in [1], 5 in [1]);"#,
        r#"try{ print("a" in "abc"); }catch(e){ print("caught", e.name, e.message); }"#,
        r#"try{ print("a" in null); }catch(e){ print("caught", e.name, e.message); }"#,
        r#"print([] instanceof Array, [] instanceof Object, ({}) instanceof Array, print instanceof Function);"#,
        r#"function F(){} var f=new F(); print(f instanceof F, f instanceof Object, F instanceof F);"#,
        r#"try{ print(1 instanceof 2); }catch(e){ print("caught", e.name, e.message); }"#,
        r#"try{ print({} instanceof {}); }catch(e){ print("caught", e.name, e.message); }"#,
        r#"function F(){} F.prototype=null; try{ print(({}) instanceof F); }catch(e){ print("caught", e.name, e.message); }"#,

        /* ---- comma / ternary ---- */
        r#"var x=(1,2,3); print(x, (print(9),4));"#,
        r#"print(1?2:3, 0?2:3, ""?"a":"b", null?1:2, ({}?1:2), []?1:2);"#,
        r#"print(1?0?"a":"b":"c", 0?1?"a":"b":"c");"#,
        r#"var s=""; print((s+="a", s+="b", s), s);"#,

        /* ---- ++ / -- ---- */
        r#"var x=5; print(x++, x, x--, x, ++x, x, --x, x);"#,
        r#"var o={n:1}; print(o.n++, o.n, ++o.n, o.n, o.n--, o.n, --o.n, o.n);"#,
        r#"var a=[1]; print(a[0]++, a[0], --a[0], a[0], a[0]--, a[0]);"#,
        r#"var s="5"; print(s++, s, typeof s);"#,
        r#"var x; print(x++, x); var y; print(++y, y);"#,
        r#"var o={}; print(o.x++, o.x, typeof o.x);"#,
        r#"var s="a"; print(s++, s, isNaN(s));"#,
        r#"var i=0, a=[]; a[i++]=i++; print(JSON.stringify(a), i);"#,

        /* ---- compound assignment ---- */
        r#"var x=10; x+=5; print(x); x-=3; print(x); x*=2; print(x); x/=4; print(x); x%=4; print(x);"#,
        r#"var x=1; x<<=4; print(x); x>>=2; print(x); x>>>=1; print(x); x&=6; print(x); x|=9; print(x); x^=3; print(x);"#,
        r#"var x=-16; x>>>=2; print(x); x=-16; x>>=2; print(x); x=-16; x&=-1; print(x);"#,
        r#"var s="a"; s+=1; s+=true; s+=null; s+=[1,2]; print(s);"#,
        r#"var o={v:5}; o.v+=1; o.v*=3; print(o.v); var a=[1]; a[0]+=9; print(a[0]);"#,
        r#"var o={v:"a"}; o.v+="b"; o["v"]+="c"; print(o.v);"#,
        r#"var x=NaN; x+=1; print(x); x=Infinity; x-=Infinity; print(x); x=0; x/=0; print(x);"#,

        /* ---- unary + / - ---- */
        r#"print(+"1", +"1.5", +" 12 ", +"", +"x", +"0x10", +"1e3");"#,
        r#"print(+"Infinity", +"-Infinity", +"+5", +"-5", +".5", +"5.", +"1_0");"#,
        r#"print(-"1", -"x", -true, -null, -undefined, -[], -[5], -{}, -"");"#,
        r#"print(+true, +false, +null, +undefined, +[], +[5], +[1,2], +{});"#,
        r#"print(+new Number(3), +new String("4"), +new Boolean(true), +new Boolean(false));"#,
        r#"print(1/-"0", 1/+"-0", -(-0), 1/-(-0), - -1);"#,
    ]);
}

/* ================================================================== */
/*  H2 - every statement                                               */
/* ================================================================== */

#[test]
fn h2_statements() {
    diff_scripts_both_modes(&[
        /* ---- var ---- */
        r#"var x; print(x, typeof x);"#,
        r#"var x=1, y=2, z=x+y; print(x, y, z);"#,
        r#"print(typeof x, x); var x=1; print(x);"#,
        r#"function f(){ print(x); var x=2; print(x); } f();"#,
        r#"var x=1; var x; print(x); var x=2; print(x);"#,
        r#"var a=1, b=a+1, c=b+1, d=c+1; print(a,b,c,d);"#,
        r#"function f(){ var x=1; { var x=2; } return x; } print(f());"#,

        /* ---- if / else ---- */
        r#"if (1) print("t"); else print("f");"#,
        r#"if (0) print("t"); else print("f");"#,
        r#"if (0) print("a"); else if (1) print("b"); else print("c");"#,
        r#"var r=""; for (var i=0;i<5;i++){ if(i<2) r+="a"; else if(i<4) r+="b"; else r+="c"; } print(r);"#,
        r#"if ("") print("t"); else if ("0") print("s"); else print("f");"#,
        r#"if (1) { print("blk"); } else { print("no"); } print("after");"#,
        r#"if (1) ; else print("no"); print("empty-then");"#,

        /* ---- do/while, while, for ---- */
        r#"var i=0; do { print(i); i++; } while (i<3);"#,
        r#"var i=10; do { print("once", i); } while (0);"#,
        r#"var i=0; while (i<3) { print(i); i++; }"#,
        r#"var i=0; while (0) { print("never"); } print("done", i);"#,
        r#"for (var i=0;i<3;i++) print(i);"#,
        r#"var i=0; for (;;) { if (i>=3) break; print(i); i++; }"#,
        r#"var i=0; for (; i<3 ;) { print(i); i++; }"#,
        r#"for (var i=0, j=5; i<j; i++, j--) print(i, j);"#,
        r#"var s=""; for (var i=3;i;i--) s+=i; print(s);"#,
        r#"var i; for (i=0;i<2;i++); print("body-empty", i);"#,

        /* ---- for-in ---- */
        r#"var o={a:1,b:2,c:3}; var k=[]; for (var p in o) k.push(p+"="+o[p]); print(k.join(","));"#,
        r#"var a=[10,20,30]; var k=[]; for (var i in a) k.push(i+":"+a[i]+":"+typeof i); print(k.join(","));"#,
        r#"var a=[1,,3]; var k=[]; for (var i in a) k.push(i); print(k.join(","), a.length);"#,
        r#"var a=[1,2,3]; delete a[1]; var k=[]; for (var i in a) k.push(i); print(k.join(","));"#,
        r#"var k=[]; for (var p in new String("abc")) k.push(p); print(k.join(","));"#,
        r#"var k=[]; for (var p in "abc") k.push(p); print(k.join(","));"#,
        r#"function P(){} P.prototype.pp=1; var o=new P(); o.oo=2; var k=[]; for (var p in o) k.push(p); print(k.join(","));"#,
        r#"function P(){} P.prototype.x=1; var o=new P(); o.x=2; var k=[]; for (var p in o) k.push(p+"="+o[p]); print(k.join(","));"#,
        r#"var o={}; Object.defineProperty(o,"h",{value:1,enumerable:false}); o.v=2; var k=[]; for (var p in o) k.push(p); print(k.join(","));"#,
        r#"var k=[]; for (var p in Math) k.push(p); print(k.length, k.join(","));"#,
        r#"var k=[]; for (var p in {}) k.push(p); print("empty", k.length);"#,
        r#"var k=[]; for (var p in null) k.push(p); print("null", k.length);"#,
        r#"var k=[]; for (var p in undefined) k.push(p); print("undef", k.length);"#,
        r#"var k=[]; for (var p in 5) k.push(p); print("num", k.length);"#,
        r#"var o={a:1,b:2}, p, k=[]; for (p in o) k.push(p); print(k.join(","), p);"#,
        r#"var o={a:1,b:2}, k=[], t={}; for (t.key in o) k.push(t.key); print(k.join(","));"#,
        r#"var o={a:1,b:2,c:3}, k=[]; for (var p in o) { if (p==="b") continue; k.push(p); } print(k.join(","));"#,
        r#"var o={a:1,b:2,c:3}, k=[]; for (var p in o) { k.push(p); break; } print(k.join(","));"#,

        /* ---- switch ---- */
        r#"function f(x){ switch(x){ case 1: return "one"; case 2: return "two"; default: return "other"; } } print(f(1),f(2),f(3));"#,
        r#"function f(x){ switch(x){ case 1: return "one"; case 2: return "two"; } return "none"; } print(f(1),f(3));"#,
        r#"function f(x){ switch(x){ case 1: return "a"; default: return "d"; case 2: return "b"; } } print(f(1),f(2),f(9));"#,
        r#"var r=""; switch(2){ case 1: r+="1"; case 2: r+="2"; case 3: r+="3"; default: r+="d"; } print(r);"#,
        r#"var r=""; switch(2){ case 1: r+="1"; break; case 2: r+="2"; break; default: r+="d"; } print(r);"#,
        r#"switch("b"){ case "a": print("A"); break; case "b": print("B"); break; }"#,
        r#"var o={}; switch(o){ case o: print("same"); break; default: print("diff"); }"#,
        r#"switch(1){ case "1": print("string"); break; case 1: print("number"); break; }"#,
        r#"switch(1){} print("empty-body");"#,
        r#"switch(1){ default: print("only-default"); }"#,
        r#"var s=""; function g(x){ s+=x; return x; } switch(g(3)){ case g(1): s+="c1"; case g(2): s+="c2"; case g(3): s+="c3"; case g(4): s+="c4"; } print(s);"#,
        r#"var r=""; for(var i=0;i<3;i++){ switch(i){ case 0: r+="z"; continue; case 1: r+="o"; break; } r+="."; } print(r);"#,

        /* ---- break / continue, labelled ---- */
        r#"var r=""; for(var i=0;i<5;i++){ if(i===3) break; r+=i; } print(r);"#,
        r#"var r=""; for(var i=0;i<5;i++){ if(i%2) continue; r+=i; } print(r);"#,
        r#"var r=""; outer: for(var i=0;i<3;i++){ for(var j=0;j<3;j++){ if(j===1) continue outer; r+=i+""+j; } } print(r);"#,
        r#"var r=""; outer: for(var i=0;i<3;i++){ for(var j=0;j<3;j++){ if(i===1&&j===1) break outer; r+=i+""+j; } } print(r);"#,
        r#"var r=""; a: for(var i=0;i<2;i++){ b: for(var j=0;j<2;j++){ c: for(var k=0;k<2;k++){ if(k===1) continue b; r+=""+i+j+k; } } } print(r);"#,
        r#"var r=""; L: { r+="a"; break L; r+="b"; } print(r);"#,
        r#"var r=""; A: B: C: for(var i=0;i<3;i++){ if(i===1) continue A; r+=i; } print(r);"#,
        r#"var r=""; var i=0; L: while(i<5){ i++; if(i===2) continue L; if(i===4) break L; r+=i; } print(r, i);"#,
        r#"var r=""; var i=0; do { i++; if (i===2) continue; if (i===4) break; r+=i; } while (i<10); print(r, i);"#,
        r#"var r=""; L: do { r+="x"; break L; } while(1); print(r);"#,
        r#"var r=""; for(var p in {a:1,b:2,c:3}) { if (p==="b") continue; r+=p; } print(r);"#,
        r#"var r=""; outer: for(var p in {a:1,b:2}) { for(var q in {x:1,y:2}) { r+=p+q; continue outer; } } print(r);"#,

        /* ---- return ---- */
        r#"function f(){ return; } print(f(), typeof f());"#,
        r#"function f(){ return 42; } print(f());"#,
        r#"function f(){ if(1) return "a"; return "b"; } print(f());"#,
        r#"function f(){ try { return "try"; } finally { print("fin"); } } print(f());"#,
        r#"function f(){ try { return "try"; } finally { return "fin"; } } print(f());"#,
        r#"function f(){ for(var i=0;i<5;i++){ if(i===2) return i; } return -1; } print(f());"#,
        r#"function f(){ return
1; } print(f(), typeof f());"#,

        /* ---- throw ---- */
        r#"try { throw 1; } catch(e) { print("caught", e, typeof e); }"#,
        r#"try { throw "s"; } catch(e) { print("caught", e, typeof e); }"#,
        r#"try { throw new Error("m"); } catch(e) { print("caught", e.name, e.message, e instanceof Error); }"#,
        r#"try { throw null; } catch(e) { print("caught", e, typeof e); }"#,
        r#"try { throw {a:1}; } catch(e) { print("caught", JSON.stringify(e)); }"#,
        r#"throw new Error("uncaught-top");"#,
        r#"throw "raw string";"#,
        r#"function f(){ throw new TypeError("deep"); } function g(){ f(); } try { g(); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- try / catch / finally ---- */
        r#"try { print("body"); } catch(e) { print("no"); } print("after");"#,
        r#"try { throw 1; } catch(e) { print("c", e); } print("after");"#,
        r#"try { print("b"); } finally { print("f"); } print("after");"#,
        r#"try { throw 1; } finally { print("f"); } print("unreachable");"#,
        r#"try { try { throw 1; } finally { print("inner-f"); } } catch(e) { print("outer-c", e); }"#,
        r#"try { throw 1; } catch(e) { print("c", e); } finally { print("f"); } print("after");"#,
        r#"try { print("b"); } catch(e) { print("c"); } finally { print("f"); }"#,
        r#"try { throw 1; } catch(e) { throw 2; } finally { print("f"); }"#,
        r#"try { try { throw 1; } catch(e) { throw 2; } finally { throw 3; } } catch(e) { print("outer", e); }"#,
        r#"try { try { throw 1; } finally { throw 2; } } catch(e) { print("override", e); }"#,
        r#"var r=""; for(var i=0;i<3;i++){ try { if(i===1) continue; r+="b"+i; } finally { r+="f"+i; } } print(r);"#,
        r#"var r=""; for(var i=0;i<3;i++){ try { if(i===1) break; r+="b"+i; } finally { r+="f"+i; } } print(r);"#,
        r#"var e2; try { nosuchfunction(); } catch(e) { e2=e; print("caught", e.name); } print(e2 instanceof ReferenceError);"#,
        r#"try { var e=1; throw 2; } catch(e) { print("shadow", e); } print("outer-e", e);"#,
        r#"try { throw 1; } catch(e) { print(e); } try { throw 2; } catch(e) { print(e); }"#,
        r#"function f(){ try { throw "x"; } catch(e) { return "c"; } finally { print("f"); } } print(f());"#,
        r#"var r=""; L: try { r+="a"; break L; } finally { r+="f"; } print(r);"#,

        /* ---- misc statements ---- */
        r#";;; print("empty-statements"); ;"#,
        r#"{ var x=1; print(x); } { var y=2; print(y); } print(x+y);"#,
        r#"debugger; print("after-debugger");"#,
        r#"{ } print("empty-block");"#,
        r#"var x=1
var y=2
print(x+y)"#,
        r#"var x=1; var y = x
+2; print(y);"#,
        r#"function f(){ return 1 } print(f())"#,
        r#"var a=1; a
++
a
print(a)"#,
        r#"print("a"
,"b");"#,
    ]);

    /* `with` is a SyntaxError under strict mode, so those live here only. */
    diff_scripts(
        0,
        &[
            r#"with({a:1}) { print(a); }"#,
            r#"var a=9; with({a:1}) { print(a); } print(a);"#,
            r#"var o={a:1}; with(o) { a=2; } print(o.a, typeof a);"#,
            r#"with({}) { print(typeof nosuch); }"#,
            r#"with(Math) { print(floor(1.5), PI===Math.PI); }"#,
            r#"var o={a:1}; with(o) { var b=a+1; } print(b, typeof o.b);"#,
            r#"with({a:1}) with({b:2}) print(a+b);"#,
            r#"try { with(null) { print("x"); } } catch(e) { print("caught", e.name, e.message); }"#,
            r#"try { with(undefined) { print("x"); } } catch(e) { print("caught", e.name, e.message); }"#,
            r#"with("abc") { print(length, charAt(1)); }"#,
            r#"var o={a:1}; with(o) { delete a; } print("a" in o, typeof a);"#,
            r#"function f(){ with({x:5}) { return x; } } print(f());"#,
            r#"var k=""; with({a:1,b:2}) { for (var p in this) {} } print("with-forin-ok");"#,
            r#"with([1,2,3]) { print(length, join("-")); }"#,
        ],
    );
}

/* ================================================================== */
/*  H3 - functions                                                     */
/* ================================================================== */

#[test]
fn h3_functions() {
    diff_scripts_both_modes(&[
        /* ---- declarations vs expressions vs named expressions ---- */
        r#"function f(){ return 1; } print(f(), typeof f);"#,
        r#"var f = function(){ return 2; }; print(f(), typeof f);"#,
        r#"var f = function g(){ return 3; }; print(f(), typeof f, typeof g);"#,
        r#"var f = function g(n){ return n<=0 ? 0 : n + g(n-1); }; print(f(4));"#,
        r#"print(f()); function f(){ return "hoisted"; }"#,
        r#"print(typeof f, typeof g); function f(){} var g = function(){};"#,
        r#"function f(){ return "first"; } function f(){ return "second"; } print(f());"#,
        r#"var f = 1; function f(){} print(typeof f);"#,
        r#"function f(){} var f; print(typeof f);"#,
        r#"print((function(){ return "iife"; })());"#,
        r#"print((function(a,b){ return a+b; })(3,4));"#,
        r#"print(function(){}.constructor === Function, Function.prototype.constructor === Function);"#,
        r#"print(typeof Function.prototype, Function.prototype.length, typeof Function.prototype.call);"#,

        /* ---- fn.length / fn.name / fn.prototype ---- */
        r#"function f(a,b,c){} print(f.length, typeof f.length);"#,
        r#"print((function(){}).length, (function(a){}).length, (function(a,b,c,d){}).length);"#,
        r#"function f(a,b){} print(f.name, typeof f.name, "name" in f);"#,
        r#"function f(){} print(Object.getOwnPropertyNames(f).join(","), typeof f.prototype);"#,
        r#"function f(){} print(f.prototype.constructor === f, Object.getPrototypeOf(f) === Function.prototype);"#,
        r#"print(print.length, Math.max.length, Object.keys.length, Array.prototype.slice.length);"#,
        r#"function f(){} print(JSON.stringify(Object.getOwnPropertyDescriptor(f, "length")));"#,
        r#"function f(){} print(JSON.stringify(Object.getOwnPropertyDescriptor(f, "prototype")));"#,

        /* ---- closures ---- */
        r#"function mk(){ var n=0; return function(){ return ++n; }; } var c=mk(); print(c(),c(),c());"#,
        r#"function mk(){ var n=0; return { inc:function(){return ++n}, get:function(){return n} }; } var o=mk(); o.inc(); o.inc(); print(o.get());"#,
        r#"var fs=[]; for (var i=0;i<3;i++) fs.push(function(){ return i; }); print(fs[0](), fs[1](), fs[2]());"#,
        r#"var fs=[]; for (var i=0;i<3;i++) (function(j){ fs.push(function(){ return j; }); })(i); print(fs[0](), fs[1](), fs[2]());"#,
        r#"var a=1; function o(){ var a=2; function i(){ return a; } return i(); } print(o(), a);"#,
        r#"function o(){ var x="o"; function m(){ var y="m"; function i(){ return x+y; } return i(); } return m(); } print(o());"#,
        r#"var c=(function(){ var s=0; return function(n){ s+=n; return s; }; })(); print(c(1),c(2),c(3));"#,
        r#"function counter(){ var n=0; return function(){ n=n+1; return n; }; } var a=counter(), b=counter(); print(a(),a(),b());"#,

        /* ---- recursion ---- */
        r#"function fac(n){ return n<=1 ? 1 : n*fac(n-1); } print(fac(0),fac(1),fac(5),fac(10),fac(20));"#,
        r#"function fib(n){ return n<2 ? n : fib(n-1)+fib(n-2); } print(fib(0),fib(1),fib(10),fib(18));"#,
        r#"function ev(n){ return n===0 ? true : od(n-1); } function od(n){ return n===0 ? false : ev(n-1); } print(ev(10), od(10), ev(7), od(7));"#,
        r#"function sum(a,i){ i=i||0; return i>=a.length ? 0 : a[i]+sum(a,i+1); } print(sum([1,2,3,4,5]));"#,
        r#"function ack(m,n){ if(m===0) return n+1; if(n===0) return ack(m-1,1); return ack(m-1,ack(m,n-1)); } print(ack(2,3));"#,
        r#"function deep(n){ return n===0 ? 0 : 1+deep(n-1); } print(deep(100));"#,

        /* ---- arguments ---- */
        r#"function f(){ return arguments.length; } print(f(), f(1), f(1,2), f(1,2,3));"#,
        r#"function f(){ var s=""; for(var i=0;i<arguments.length;i++) s+=arguments[i]; return s; } print(f(1,2,3));"#,
        r#"function f(a){ return [a, arguments[0], arguments[1], typeof arguments[5]].join("|"); } print(f(7));"#,
        r#"function f(){ return typeof arguments + "/" + (arguments instanceof Object) + "/" + Array.isArray(arguments); } print(f());"#,
        r#"function f(a){ arguments[0]=99; return a; } print(f(1));"#,
        r#"function f(a){ a=99; return arguments[0]; } print(f(1));"#,
        r#"function f(a,b){ arguments[1]=9; return a+","+b; } print(f(1,2));"#,
        r#"function f(){ return Object.prototype.toString.call(arguments); } print(f(1,2));"#,
        r#"function f(){ arguments.length=0; return arguments.length; } print(f(1,2,3));"#,
        r#"function f(){ return Array.prototype.join.call(arguments,"-"); } print(f(1,2,3));"#,
        r#"function f(){ return Array.prototype.slice.call(arguments).join("+"); } print(f(4,5,6));"#,
        r#"function f(){ var k=[]; for (var p in arguments) k.push(p); return k.join(","); } print(f(1,2));"#,
        r#"function f(a,b,c){ return [a,b,c,arguments.length].join("|"); } print(f(1), f(1,2,3,4,5));"#,
        r#"function f(a){ return a===undefined; } print(f(), f(undefined), f(null));"#,

        /* ---- this ---- */
        r#"var o={n:5, m:function(){ return this.n; }}; print(o.m());"#,
        r#"var o={n:5, m:function(){ return this.n; }}; var g=o.m; print(typeof g());"#,
        r#"var o={n:5, m:function(){ return typeof this; }}; print(o.m());"#,
        r#"function f(){ return this === undefined ? "undef" : (this === null ? "null" : typeof this); } print(f());"#,
        r#"var o={a:{b:{m:function(){ return this.v; }, v:7}}}; print(o.a.b.m());"#,
        r#"var o={n:1, m:function(){ var self=this; return (function(){ return self.n; })(); }}; print(o.m());"#,
        r#"var o={n:1, m:function(){ return (function(){ return typeof this; })(); }}; print(o.m());"#,
        r#"var o={v:"o"}; function f(){ return this.v; } print(f.call(o), f.apply(o));"#,

        /* ---- new ---- */
        r#"function P(a){ this.a=a; } var p=new P(1); print(p.a, p instanceof P, typeof p);"#,
        r#"function P(){ this.a=1; return 5; } var p=new P(); print(p.a, typeof p);"#,
        r#"function P(){ this.a=1; return {b:2}; } var p=new P(); print(p.a, p.b, p instanceof P);"#,
        r#"function P(){ this.a=1; return null; } var p=new P(); print(p.a, p instanceof P);"#,
        r#"function P(){ this.a=1; return undefined; } var p=new P(); print(p.a, p instanceof P);"#,
        r#"function P(){ this.a=1; return [7]; } var p=new P(); print(p.a, Array.isArray(p), p[0]);"#,
        r#"function P(){} P.prototype.m=function(){ return "proto"; }; print(new P().m());"#,
        r#"function A(){} function B(){} B.prototype=new A(); var b=new B(); print(b instanceof B, b instanceof A, b instanceof Object);"#,
        r#"function A(){ this.x=1; } function B(){ A.call(this); this.y=2; } B.prototype=new A(); var b=new B(); print(b.x, b.y, b instanceof A);"#,
        r#"function P(){} var p=new P; print(p instanceof P, typeof p);"#,
        r#"function P(a,b){ this.s=a+b; } print(new P(1,2).s, new P("a","b").s);"#,
        r#"try { new print(); print("ok"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { var x = new 5; } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { new (function(){ throw new Error("ctor"); })(); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"function P(){} P.prototype=null; var p=new P(); print(typeof p, Object.getPrototypeOf(p)===Object.prototype);"#,
        r#"function P(){} P.prototype={m:function(){return 1}}; var p=new P(); print(p.m(), p.constructor===Object);"#,

        /* ---- functions returning functions / higher order ---- */
        r#"function add(a){ return function(b){ return a+b; }; } print(add(1)(2), add("x")("y"));"#,
        r#"function comp(f,g){ return function(x){ return f(g(x)); }; } print(comp(function(x){return x+1}, function(x){return x*2})(5));"#,
        r#"function ap(f,a){ return f.apply(null, a); } print(ap(function(x,y){return x*y},[3,4]));"#,
        r#"var fs={a:function(){return 1}, b:function(){return 2}}; print(fs.a()+fs.b());"#,
        r#"print((function(f){ return f(f, 5); })(function(self,n){ return n<=0?0:n+self(self,n-1); }));"#,

        /* ---- nested scopes / hoisting order ---- */
        r#"function f(){ print(typeof g); function g(){} print(typeof g); } f();"#,
        r#"function f(){ var x="outer"; if(1){ var x="inner"; } return x; } print(f());"#,
        r#"var x="g"; function f(){ print(x); var x="l"; print(x); } f(); print(x);"#,
        r#"function f(a){ function a(){} return typeof a; } print(f(1));"#,
        r#"function f(a,a){ return a; } print(f(1,2));"#,
        r#"function f(){ return function(){ return function(){ return "deep"; }; }; } print(f()()());"#,
        r#"function f(){ var r=[]; for (var i=0;i<2;i++){ function g(){ return i; } r.push(g()); } return r.join(","); } print(f());"#,
    ]);

    /* arguments.callee is only meaningful outside strict mode. */
    diff_scripts(
        0,
        &[
            r#"function f(){ return arguments.callee === f; } print(f());"#,
            r#"print((function(n){ return n<=1 ? 1 : n*arguments.callee(n-1); })(5));"#,
            r#"function f(){ return typeof arguments.callee; } print(f());"#,
            r#"function f(){ return Object.getOwnPropertyNames(arguments).join(","); } print(f(1,2));"#,
            r#"function f(a){ arguments[0]=2; return a+","+arguments[0]; } print(f(1));"#,
        ],
    );
}

/* ================================================================== */
/*  H4 - Object builtins (every function in jsobject.c)                */
/* ================================================================== */

#[test]
fn h4_object_builtins() {
    diff_scripts_both_modes(&[
        /* ---- Object() as function and as constructor ---- */
        r#"print(typeof Object(), JSON.stringify(Object()), Object.getOwnPropertyNames(Object()).length);"#,
        r#"print(typeof Object(undefined), typeof Object(null), JSON.stringify(Object(null)));"#,
        r#"var o=Object(1); print(typeof o, o, o+1, o.valueOf(), o instanceof Number);"#,
        r#"var o=Object("ab"); print(typeof o, o, o.length, o[0], o instanceof String);"#,
        r#"var o=Object(true); print(typeof o, o, o.valueOf(), o instanceof Boolean);"#,
        r#"var a=[1]; print(Object(a)===a, Object(Math)===Math, Object(print)===print);"#,
        r#"print(typeof new Object(), typeof new Object(1), typeof new Object("s"), typeof new Object(null));"#,
        r#"var o=new Object(5); print(o.valueOf(), o instanceof Number, Object.prototype.toString.call(o));"#,
        r#"print(new Object(false).valueOf(), new Object(NaN).valueOf(), new Object(-0).valueOf());"#,
        r#"print(Object.length, Object.prototype.constructor===Object, typeof Object.prototype);"#,

        /* ---- Object.getPrototypeOf ---- */
        r#"print(Object.getPrototypeOf({})===Object.prototype, Object.getPrototypeOf([])===Array.prototype);"#,
        r#"print(Object.getPrototypeOf(Object.prototype), Object.getPrototypeOf(function(){})===Function.prototype);"#,
        r#"function F(){} print(Object.getPrototypeOf(new F())===F.prototype);"#,
        r#"try { print(Object.getPrototypeOf(1)); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { print(Object.getPrototypeOf(null)); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { print(Object.getPrototypeOf()); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- Object.getOwnPropertyDescriptor ---- */
        r#"var d=Object.getOwnPropertyDescriptor({a:1},"a"); print(d.value,d.writable,d.enumerable,d.configurable);"#,
        r#"print(Object.getOwnPropertyDescriptor({a:1},"b"), typeof Object.getOwnPropertyDescriptor({},"x"));"#,
        r#"print(JSON.stringify(Object.getOwnPropertyDescriptor([1,2],"0")));"#,
        r#"print(JSON.stringify(Object.getOwnPropertyDescriptor([1,2],"length")));"#,
        r#"var o={}; Object.defineProperty(o,"g",{get:function(){return 1}}); var d=Object.getOwnPropertyDescriptor(o,"g"); print(typeof d.get, typeof d.set, d.enumerable, d.configurable, "value" in d);"#,
        r#"print(JSON.stringify(Object.getOwnPropertyDescriptor(Math,"PI")));"#,
        r#"function f(){} print(JSON.stringify(Object.getOwnPropertyDescriptor(f,"prototype")));"#,
        r#"try { Object.getOwnPropertyDescriptor(1,"x"); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- Object.getOwnPropertyNames / keys ---- */
        r#"print(Object.getOwnPropertyNames({b:1,a:2,c:3}).join(","));"#,
        r#"print(Object.getOwnPropertyNames([1,2,3]).join(","));"#,
        r#"print(Object.getOwnPropertyNames([]).join(","), Object.getOwnPropertyNames({}).length);"#,
        r#"var o={}; Object.defineProperty(o,"h",{value:1,enumerable:false}); o.v=1; print(Object.getOwnPropertyNames(o).join(","), Object.keys(o).join(","));"#,
        r#"print(Object.keys({b:1,a:2}).join(","), Object.keys([7,8]).join(","), Object.keys([]).length);"#,
        r#"function F(){} F.prototype.p=1; var o=new F(); o.q=2; print(Object.keys(o).join(","), Object.getOwnPropertyNames(o).join(","));"#,
        r#"print(Object.keys(Math).length, Object.getOwnPropertyNames(Math).length);"#,
        r#"var a=[1,,3]; print(Object.keys(a).join(","), Object.getOwnPropertyNames(a).join(","));"#,
        r#"try { Object.keys("ab"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Object.getOwnPropertyNames(null); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- Object.create ---- */
        r#"var p={a:1}; var o=Object.create(p); print(o.a, Object.getPrototypeOf(o)===p, Object.keys(o).length);"#,
        r#"var o=Object.create(null); print(typeof o, Object.getPrototypeOf(o), typeof o.toString);"#,
        r#"var o=Object.create(null); o.x=1; print(o.x, Object.keys(o).join(","));"#,
        r#"var o=Object.create({}, {a:{value:1,enumerable:true}, b:{value:2}}); print(o.a, o.b, Object.keys(o).join(","), Object.getOwnPropertyNames(o).join(","));"#,
        r#"var o=Object.create(Object.prototype, {x:{get:function(){return 9},enumerable:true}}); print(o.x, Object.keys(o).join(","));"#,
        r#"try { Object.create(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Object.create(); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o=Object.create(Array.prototype); print(Array.isArray(o), typeof o.join, o.length);"#,

        /* ---- Object.defineProperty / defineProperties, all attribute combos ---- */
        r#"var o={}; Object.defineProperty(o,"a",{value:1}); print(o.a, Object.keys(o).length, JSON.stringify(Object.getOwnPropertyDescriptor(o,"a")));"#,
        r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:true,enumerable:true,configurable:true}); print(JSON.stringify(Object.getOwnPropertyDescriptor(o,"a")));"#,
        r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:false,enumerable:true,configurable:true}); o.a=2; print(o.a);"#,
        r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:true,enumerable:false,configurable:true}); print(Object.keys(o).length, o.a);"#,
        r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:true,enumerable:true,configurable:false}); print(delete o.a, o.a);"#,
        r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:false,enumerable:false,configurable:false}); print(JSON.stringify(Object.getOwnPropertyDescriptor(o,"a")), Object.keys(o).length);"#,
        r#"var o={}; var v=0; Object.defineProperty(o,"a",{get:function(){return v},set:function(x){v=x*2}}); o.a=5; print(o.a, v);"#,
        r#"var o={}; Object.defineProperty(o,"a",{get:function(){return 1}}); o.a=5; print(o.a);"#,
        r#"var o={}; Object.defineProperty(o,"a",{set:function(x){this.b=x}}); o.a=3; print(o.a, o.b);"#,
        r#"var o={}; Object.defineProperty(o,"a",{get:function(){return 1},enumerable:true,configurable:true}); print(Object.keys(o).join(","), delete o.a, "a" in o);"#,
        r#"var o={}; try { Object.defineProperty(o,"a",{value:1,get:function(){return 1}}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={}; try { Object.defineProperty(o,"a",{get:1}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={a:1}; Object.defineProperty(o,"a",{value:2}); print(o.a, JSON.stringify(Object.getOwnPropertyDescriptor(o,"a")));"#,
        r#"var o={}; Object.defineProperty(o,"a",{value:1,configurable:false}); try { Object.defineProperty(o,"a",{value:2}); print("redef", o.a); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o=[]; Object.defineProperty(o,"0",{value:9,enumerable:true}); print(o.length, o[0], JSON.stringify(o));"#,
        r#"try { Object.defineProperty(1,"a",{value:1}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={}; try { Object.defineProperty(o,"a",1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={}; Object.defineProperties(o,{a:{value:1,enumerable:true},b:{get:function(){return 2},enumerable:true}}); print(o.a,o.b,Object.keys(o).join(","));"#,
        r#"var o={}; Object.defineProperties(o,{}); print(Object.keys(o).length);"#,
        r#"try { Object.defineProperties({},1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={}; print(Object.defineProperty(o,"a",{value:1})===o, Object.defineProperties(o,{})===o);"#,

        /* ---- seal / freeze / preventExtensions ---- */
        r#"var o={a:1}; print(Object.isExtensible(o), Object.isSealed(o), Object.isFrozen(o));"#,
        r#"var o={a:1}; Object.preventExtensions(o); o.b=2; print(o.b, Object.isExtensible(o), Object.isSealed(o), delete o.a);"#,
        r#"var o={a:1}; Object.seal(o); o.b=2; print(o.b, delete o.a, o.a, Object.isSealed(o), Object.isFrozen(o));"#,
        r#"var o={a:1}; Object.seal(o); o.a=5; print(o.a);"#,
        r#"var o={a:1}; Object.freeze(o); o.a=5; o.b=2; print(o.a, o.b, Object.isFrozen(o), Object.isSealed(o), Object.isExtensible(o));"#,
        r#"var o={}; Object.preventExtensions(o); print(Object.isSealed(o), Object.isFrozen(o));"#,
        r#"var o={}; Object.seal(o); print(Object.isSealed(o), Object.isFrozen(o));"#,
        r#"var o=Object.freeze({a:1}); print(JSON.stringify(Object.getOwnPropertyDescriptor(o,"a")));"#,
        r#"var o=Object.seal({a:1}); print(JSON.stringify(Object.getOwnPropertyDescriptor(o,"a")));"#,
        r#"var a=[1,2]; Object.freeze(a); a[0]=9; print(a[0], Object.isFrozen(a));"#,
        r#"var o={a:1}; print(Object.seal(o)===o, Object.freeze(o)===o, Object.preventExtensions(o)===o);"#,
        r#"try { Object.seal(1); print("sealed-primitive"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { print(Object.isSealed(1), Object.isFrozen(1), Object.isExtensible(1)); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={a:1}; Object.freeze(o); try { Object.defineProperty(o,"a",{value:2}); print("redef"); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- Object.prototype methods ---- */
        r#"var o={a:1}; print(o.hasOwnProperty("a"), o.hasOwnProperty("toString"), o.hasOwnProperty("b"));"#,
        r#"print([1,2].hasOwnProperty(0), [1,2].hasOwnProperty("length"), [1,2].hasOwnProperty(5));"#,
        r#"print("ab".hasOwnProperty(0), "ab".hasOwnProperty("length"), (5).hasOwnProperty("x"));"#,
        r#"try { Object.prototype.hasOwnProperty.call(null,"a"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={}; print(o.hasOwnProperty(), Object.prototype.hasOwnProperty.call({undefined:1}));"#,
        r#"function F(){} var f=new F(); print(F.prototype.isPrototypeOf(f), Object.prototype.isPrototypeOf(f), f.isPrototypeOf(F));"#,
        r#"print(Object.prototype.isPrototypeOf([]), Array.prototype.isPrototypeOf([]), Array.prototype.isPrototypeOf({}));"#,
        r#"print(Object.prototype.isPrototypeOf(1), Object.prototype.isPrototypeOf(null), Object.prototype.isPrototypeOf());"#,
        r#"var o={a:1}; Object.defineProperty(o,"h",{value:2,enumerable:false}); print(o.propertyIsEnumerable("a"), o.propertyIsEnumerable("h"), o.propertyIsEnumerable("toString"), o.propertyIsEnumerable("zz"));"#,
        r#"print([1].propertyIsEnumerable(0), [1].propertyIsEnumerable("length"), Math.propertyIsEnumerable("PI"));"#,
        r#"print(({}).toString(), [1,2].toString(), (5).toString(), "s".toString(), true.toString());"#,
        r#"print(Object.prototype.toString.call([]), Object.prototype.toString.call(null), Object.prototype.toString.call(undefined));"#,
        r#"print(Object.prototype.toString.call(1), Object.prototype.toString.call("s"), Object.prototype.toString.call(true));"#,
        r#"print(Object.prototype.toString.call(print), Object.prototype.toString.call(Math), Object.prototype.toString.call(/x/));"#,
        r#"print(Object.prototype.toString.call(new Date(0)), Object.prototype.toString.call(new Error("e")), Object.prototype.toString.call(JSON));"#,
        r#"print(({}).toLocaleString(), [1,2].toLocaleString(), Object.prototype.toLocaleString.call(5));"#,
        r#"var o={toString:function(){return "TS"}}; print(o.toLocaleString(), ""+o, String(o));"#,
        r#"var o={a:1}; print(o.valueOf()===o, [1].valueOf().length, Object.prototype.valueOf.call(5), typeof Object.prototype.valueOf.call("s"));"#,
        r#"try { Object.prototype.valueOf.call(null); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Object.prototype.toLocaleString.call(null); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={}; print(o.toString===Object.prototype.toString, o.toLocaleString===Object.prototype.toString);"#,

        /* ---- uncaught paths (top-level report) ---- */
        r#"Object.getPrototypeOf(null);"#,
        r#"Object.defineProperty({}, "a", {get:1});"#,
    ]);

    /* Silent-vs-throwing mutation of non-writable properties differs between
     * sloppy and strict mode, so run each mode explicitly and compare. */
    diff_scripts(
        0,
        &[
            r#"var o=Object.freeze({a:1}); o.a=2; print("sloppy", o.a);"#,
            r#"var o=Object.seal({a:1}); o.b=2; print("sloppy", o.b);"#,
            r#"var o=Object.preventExtensions({}); o.x=1; print("sloppy", o.x);"#,
            r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:false}); o.a=2; print("sloppy", o.a);"#,
            r#"var o={}; Object.defineProperty(o,"a",{get:function(){return 1}}); o.a=2; print("sloppy", o.a);"#,
            r#"var o={}; Object.defineProperty(o,"a",{value:1,configurable:false}); print("sloppy", delete o.a, o.a);"#,
        ],
    );
    diff_scripts(
        JS_STRICT,
        &[
            r#"var o=Object.freeze({a:1}); try { o.a=2; } catch(e) { print("caught", e.name, e.message); } print(o.a);"#,
            r#"var o=Object.seal({a:1}); try { o.b=2; } catch(e) { print("caught", e.name, e.message); } print(o.b);"#,
            r#"var o=Object.preventExtensions({}); try { o.x=1; } catch(e) { print("caught", e.name, e.message); } print(o.x);"#,
            r#"var o={}; Object.defineProperty(o,"a",{value:1,writable:false}); try { o.a=2; } catch(e) { print("caught", e.name, e.message); } print(o.a);"#,
            r#"var o={}; Object.defineProperty(o,"a",{get:function(){return 1}}); try { o.a=2; } catch(e) { print("caught", e.name, e.message); } print(o.a);"#,
            r#"var o={}; Object.defineProperty(o,"a",{value:1,configurable:false}); try { print(delete o.a); } catch(e) { print("caught", e.name, e.message); } print(o.a);"#,
            r#"var o=Object.freeze({a:1}); o.a=2;"#,
        ],
    );
}

/* ================================================================== */
/*  H5 - Array builtins (every prototype method in jsarray.c)          */
/* ================================================================== */

#[test]
fn h5_array_builtins() {
    diff_scripts_both_modes(&[
        /* ---- the Array constructor ---- */
        r#"var a=Array(); print(a.length, JSON.stringify(a), Array.isArray(a));"#,
        r#"var a=Array(5); print(a.length, JSON.stringify(a), 0 in a, a[0]);"#,
        r#"var a=Array(1,2,3); print(a.length, JSON.stringify(a));"#,
        r#"var a=Array("5"); print(a.length, JSON.stringify(a));"#,
        r#"var a=new Array(3); print(a.length, a.join("-"), JSON.stringify(a));"#,
        r#"var a=new Array(0); print(a.length, JSON.stringify(a));"#,
        r#"try { Array(-1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Array(1.5); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Array(4294967296); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(Array(undefined).length, JSON.stringify(Array(undefined)), Array(null).length);"#,
        r#"print(Array.length, Array.prototype.length, Array.isArray(Array.prototype));"#,
        r#"print(Array.isArray([]), Array.isArray({}), Array.isArray("s"), Array.isArray(1), Array.isArray(null));"#,
        r#"print(Array.isArray(undefined), Array.isArray(arguments!==undefined), Array.isArray(new Array(2)), Array.isArray(Array));"#,
        r#"print(Array.isArray(), Array.isArray({length:0}), Array.isArray(Object.create(Array.prototype)));"#,

        /* ---- length assignment / sparse transition ---- */
        r#"var a=[1,2,3]; a.length=1; print(a.length, JSON.stringify(a)); a.length=3; print(a.length, JSON.stringify(a), 1 in a);"#,
        r#"var a=[1,2,3]; a.length=0; print(a.length, JSON.stringify(a), a[0]);"#,
        r#"var a=[]; a.length=5; print(a.length, JSON.stringify(a), Object.keys(a).length);"#,
        r#"var a=[1,2,3]; try { a.length=-1; } catch(e) { print("caught", e.name, e.message); } print(a.length);"#,
        r#"var a=[1,2,3]; try { a.length=1.5; } catch(e) { print("caught", e.name, e.message); } print(a.length);"#,
        r#"var a=[1]; try { a.length=4294967295; print("grew", a.length); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var a=[1]; a.length="3"; print(a.length, JSON.stringify(a));"#,
        r#"var a=[0,1,2,3,4,5]; a[1000]=1000; print(a.length, a[1000], a[7], JSON.stringify(a).length);"#,
        r#"var a=[]; for(var i=0;i<6;i++) a[i]=i; a[1000]=1000; var k=[]; for(var p in a) k.push(p); print(k.join(","));"#,
        r#"var a=[]; a[3]=3; print(a.length, JSON.stringify(a), 0 in a, Object.keys(a).join(","));"#,
        r#"var a=[1,2,3]; a[-1]=9; a["x"]=8; print(a.length, a[-1], a.x, JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var a=[1,2,3]; a["1"]=9; print(JSON.stringify(a), a[1]);"#,
        r#"var a=[1,2,3]; a[1.0]=7; a[2.5]=8; print(JSON.stringify(a), a[2.5], a.length);"#,

        /* ---- toString / join ---- */
        r#"print([].toString(), [1].toString(), [1,2,3].toString(), [[1,2],[3]].toString());"#,
        r#"print([null,undefined,1].toString(), [,,].toString(), [,1,].toString());"#,
        r#"print(String([1,[2,[3]]]), ""+[1,2], [1,2]+"");"#,
        r#"print(Array.prototype.toString.call({length:2,0:"a",1:"b"}));"#,
        r#"var o={join:function(){return "custom"}}; print(Array.prototype.toString.call(o));"#,
        r#"print([].join(), [].join("-"), [1].join("-"), [1,2,3].join("-"), [1,2,3].join(""));"#,
        r#"print([1,null,2,undefined,3].join("-"), [,1,,2,].join("-"));"#,
        r#"print([1,2].join(undefined), [1,2].join(null), [1,2].join(0), [1,2].join({}));"#,
        r#"print(Array.prototype.join.call({length:3,0:"a",2:"c"},"|"));"#,
        r#"print(Array.prototype.join.call("abc","-"), Array.prototype.join.call({length:0},"-"));"#,
        r#"print([[1],[2,[3]]].join("+"), [{}].join("-"), [{toString:function(){return "T"}}].join());"#,

        /* ---- concat ---- */
        r#"print(JSON.stringify([].concat()), JSON.stringify([].concat([])), JSON.stringify([1].concat(2)));"#,
        r#"print(JSON.stringify([1,2].concat([3,4],5,[6,[7]])));"#,
        r#"print(JSON.stringify([1].concat(undefined,null)), [1].concat(undefined).length);"#,
        r#"var a=[1,,3]; var b=a.concat([4]); print(b.length, JSON.stringify(b), 1 in b);"#,
        r#"print(Array.prototype.concat.call({length:2,0:"a"},1).length, JSON.stringify(Array.prototype.concat.call({length:2,0:"a"},1)));"#,
        r#"print(Array.prototype.concat.call("ab",1).length, JSON.stringify(Array.prototype.concat.call("ab",1)));"#,
        r#"var a=[1,2]; var b=a.concat(); print(b===a, JSON.stringify(b));"#,

        /* ---- push / pop ---- */
        r#"var a=[]; print(a.push(), a.length, a.push(1), a.push(2,3), a.length, JSON.stringify(a));"#,
        r#"var a=[1]; print(a.pop(), a.length, a.pop(), a.length, JSON.stringify(a));"#,
        r#"var a=[]; print(a.pop(), a.length, typeof a.pop());"#,
        r#"var a=[1,,3]; print(a.pop(), a.pop(), a.length, JSON.stringify(a));"#,
        r#"var o={length:2,0:"a",1:"b"}; print(Array.prototype.push.call(o,"c"), o.length, o[2], JSON.stringify(o));"#,
        r#"var o={length:0}; print(Array.prototype.pop.call(o), o.length);"#,
        r#"var o={length:3,2:"z"}; print(Array.prototype.pop.call(o), o.length);"#,
        r#"var o={}; print(Array.prototype.push.call(o,1,2), o.length, o[0], o[1]);"#,

        /* ---- shift / unshift ---- */
        r#"var a=[1,2,3]; print(a.shift(), a.length, JSON.stringify(a));"#,
        r#"var a=[]; print(a.shift(), a.length, typeof a.shift());"#,
        r#"var a=[1]; print(a.shift(), a.length, JSON.stringify(a));"#,
        r#"var a=[1,,3]; print(a.shift(), a.length, JSON.stringify(a), 0 in a);"#,
        r#"var a=[1]; print(a.unshift(0), a.length, JSON.stringify(a), a.unshift(-2,-1), JSON.stringify(a));"#,
        r#"var a=[]; print(a.unshift(), a.length, JSON.stringify(a));"#,
        r#"var o={length:2,0:"a",1:"b"}; print(Array.prototype.shift.call(o), o.length, JSON.stringify(o));"#,
        r#"var o={length:1,0:"a"}; print(Array.prototype.unshift.call(o,"z"), o.length, o[0], o[1]);"#,

        /* ---- slice ---- */
        r#"var a=[1,2,3,4,5]; print(JSON.stringify(a.slice()), JSON.stringify(a.slice(1)), JSON.stringify(a.slice(1,3)));"#,
        r#"var a=[1,2,3,4,5]; print(JSON.stringify(a.slice(-2)), JSON.stringify(a.slice(-2,-1)), JSON.stringify(a.slice(-99,99)));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.slice(3)), JSON.stringify(a.slice(2,1)), JSON.stringify(a.slice(99)));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.slice(undefined)), JSON.stringify(a.slice(0,undefined)), JSON.stringify(a.slice(1.7,2.9)));"#,
        r#"var a=[1,,3]; var s=a.slice(0); print(s.length, JSON.stringify(s), 1 in s);"#,
        r#"print(JSON.stringify([].slice()), JSON.stringify([1].slice(0,0)));"#,
        r#"print(JSON.stringify(Array.prototype.slice.call({length:3,0:"a",1:"b",2:"c"},1)));"#,
        r#"print(JSON.stringify(Array.prototype.slice.call("abcd",1,3)));"#,
        r#"print(JSON.stringify(Array.prototype.slice.call({length:2})), Array.prototype.slice.call({}).length);"#,

        /* ---- splice ---- */
        r#"var a=[1,2,3,4,5]; print(JSON.stringify(a.splice(1,2)), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(1,0,"x","y")), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice()), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(0)), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(-1)), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(-99,99)), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(1,-1)), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(99,1,"z")), JSON.stringify(a));"#,
        r#"var a=[1,2,3]; print(JSON.stringify(a.splice(1,99)), JSON.stringify(a));"#,
        r#"var a=[1,,3]; var r=a.splice(1,1); print(r.length, JSON.stringify(r), JSON.stringify(a), a.length);"#,
        r#"var a=[1,2]; print(JSON.stringify(a.splice(undefined,undefined)), JSON.stringify(a));"#,
        r#"var o={length:3,0:"a",1:"b",2:"c"}; print(JSON.stringify(Array.prototype.splice.call(o,1,1)), o.length, JSON.stringify(o));"#,
        r#"var a=[]; print(JSON.stringify(a.splice(0,0,1,2,3)), JSON.stringify(a), a.length);"#,

        /* ---- reverse ---- */
        r#"print(JSON.stringify([].reverse()), JSON.stringify([1].reverse()), JSON.stringify([1,2].reverse()), JSON.stringify([1,2,3].reverse()));"#,
        r#"var a=[1,2,3,4]; print(a.reverse()===a, JSON.stringify(a));"#,
        r#"var a=[1,,3]; a.reverse(); print(JSON.stringify(a), 1 in a, a.length);"#,
        r#"var a=[1,2,,4]; a.reverse(); print(JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var o={length:3,0:"a",1:"b",2:"c"}; Array.prototype.reverse.call(o); print(o[0],o[1],o[2]);"#,
        r#"var o={length:2,0:"a"}; Array.prototype.reverse.call(o); print(JSON.stringify(o), "0" in o, "1" in o);"#,
        r#"print(Array.prototype.reverse.call("abc"));"#,

        /* ---- indexOf / lastIndexOf ---- */
        r#"var a=[1,2,3,2,1]; print(a.indexOf(2), a.indexOf(2,2), a.indexOf(2,-2), a.indexOf(9), a.indexOf(1,-1));"#,
        r#"var a=[1,2,3,2,1]; print(a.lastIndexOf(2), a.lastIndexOf(2,2), a.lastIndexOf(2,-3), a.lastIndexOf(9));"#,
        r#"print([].indexOf(1), [].lastIndexOf(1), [1].indexOf(1), [1].lastIndexOf(1));"#,
        r#"print([1,2].indexOf("1"), [1,2].indexOf(1,99), [1,2].indexOf(1,-99), [NaN].indexOf(NaN));"#,
        r#"print([undefined].indexOf(undefined), [1,,3].indexOf(undefined), [null].indexOf(null));"#,
        r#"print([1,2].indexOf(), [1,2].lastIndexOf(), [-0].indexOf(0), [0].indexOf(-0));"#,
        r#"print(Array.prototype.indexOf.call({length:3,0:"a",1:"b"},"b"), Array.prototype.indexOf.call("abc","c"));"#,
        r#"print([1,2,3].indexOf(3,undefined), [1,2,3].lastIndexOf(1,undefined), [1,2,3].indexOf(2,1.9));"#,

        /* ---- every / some / forEach ---- */
        r#"print([1,2,3].every(function(x){return x>0}), [1,2,3].every(function(x){return x>1}), [].every(function(){return false}));"#,
        r#"print([1,2,3].some(function(x){return x>2}), [1,2,3].some(function(x){return x>9}), [].some(function(){return true}));"#,
        r#"var s=[]; [1,2,3].forEach(function(v,i,a){ s.push(i+":"+v+":"+a.length); }); print(s.join(","));"#,
        r#"var s=[]; print([].forEach(function(){s.push(1)}), s.length);"#,
        r#"var s=[]; [1,,3].forEach(function(v,i){ s.push(i+":"+v); }); print(s.join(","));"#,
        r#"var s=[]; [1,2].forEach(function(){ s.push(this===undefined?"u":typeof this); }); print(s.join(","));"#,
        r#"var s=[]; [1,2].forEach(function(v){ s.push(v+this.k); }, {k:10}); print(s.join(","));"#,
        r#"var n=0; print([1,2,3].every(function(x){ n++; return x<2; }), n);"#,
        r#"var n=0; print([1,2,3].some(function(x){ n++; return x>1; }), n);"#,
        r#"try { [1].forEach(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { [1].every(); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var s=[]; Array.prototype.forEach.call({length:2,0:"a",1:"b"}, function(v,i){s.push(i+v)}); print(s.join(","));"#,
        r#"var s=[]; Array.prototype.forEach.call("ab", function(v,i){s.push(i+v)}); print(s.join(","));"#,

        /* ---- map / filter ---- */
        r#"print(JSON.stringify([1,2,3].map(function(x){return x*2})), JSON.stringify([].map(function(x){return x})));"#,
        r#"print(JSON.stringify([1,2,3].map(function(v,i,a){return i+"/"+a.length})));"#,
        r#"var m=[1,,3].map(function(x){return x}); print(m.length, JSON.stringify(m), 1 in m);"#,
        r#"print(JSON.stringify([1,2,3].filter(function(x){return x%2})), JSON.stringify([1,2].filter(function(){return false})));"#,
        r#"var f=[1,,3].filter(function(){return true}); print(f.length, JSON.stringify(f));"#,
        r#"print(JSON.stringify([1,2].map(function(x){return x+this.k}, {k:1})));"#,
        r#"print(JSON.stringify(Array.prototype.map.call({length:2,0:1,1:2}, function(x){return x*3})));"#,
        r#"print(JSON.stringify(Array.prototype.filter.call("abc", function(c){return c!=="b"})));"#,
        r#"try { [1].map(); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- reduce / reduceRight ---- */
        r#"print([1,2,3].reduce(function(a,b){return a+b}), [1,2,3].reduce(function(a,b){return a+b},10));"#,
        r#"print([1].reduce(function(a,b){return a+b}), [].reduce(function(a,b){return a+b},5));"#,
        r#"try { [].reduce(function(a,b){return a+b}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print([1,2,3].reduceRight(function(a,b){return a+"-"+b}), [1,2,3].reduce(function(a,b){return a+"-"+b}));"#,
        r#"print([1,2,3].reduceRight(function(a,b){return a+b},10), [].reduceRight(function(a,b){return a+b},7));"#,
        r#"try { [].reduceRight(function(a,b){return a+b}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var s=[]; [1,2,3].reduce(function(acc,v,i,a){ s.push(acc+"/"+v+"/"+i+"/"+a.length); return v; }); print(s.join(" "));"#,
        r#"var s=[]; [1,2,3].reduceRight(function(acc,v,i){ s.push(acc+"/"+v+"/"+i); return v; }); print(s.join(" "));"#,
        r#"print([1,,3].reduce(function(a,b){return a+"|"+b}), [1,,3].reduceRight(function(a,b){return a+"|"+b}));"#,
        r#"print(Array.prototype.reduce.call({length:3,0:1,1:2,2:3}, function(a,b){return a+b}));"#,
        r#"print(Array.prototype.reduce.call("abc", function(a,b){return a+b}));"#,
        r#"try { [1,2].reduce(1); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- receiver / this coercion errors ---- */
        r#"try { Array.prototype.join.call(null); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Array.prototype.push.call(undefined,1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(Array.prototype.slice.call(5).length, Array.prototype.join.call(5,"-"));"#,
        r#"Array.prototype.pop.call(null);"#,
    ]);
}

/* ================================================================== */
/*  H6 - Array.prototype.sort / reverse                                */
/* ================================================================== */

#[test]
fn h6_array_sort() {
    diff_scripts_both_modes(&[
        /* ---- default (string) comparison ---- */
        r#"print(JSON.stringify([].sort()), JSON.stringify([1].sort()), JSON.stringify([2,1].sort()));"#,
        r#"print(JSON.stringify([3,1,2].sort()), JSON.stringify([10,9,1].sort()), JSON.stringify([1,10,2,20].sort()));"#,
        r#"print(JSON.stringify(["b","a","c"].sort()), JSON.stringify(["B","a","A","b"].sort()));"#,
        r#"print(JSON.stringify([true,false,true].sort()), JSON.stringify([null,1,"a"].sort()));"#,
        r#"print(JSON.stringify([undefined,1,undefined,2].sort()), [undefined,1].sort().length);"#,
        r#"var a=[1,undefined,2,undefined,3]; a.sort(); print(a.length, JSON.stringify(a), 3 in a, 4 in a);"#,
        r#"var a=[3,,1,,2]; a.sort(); print(a.length, JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var a=[,,1]; a.sort(); print(a.length, JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var a=[1,2,3]; print(a.sort()===a);"#,
        r#"print(JSON.stringify([-1,-2,-10].sort()), JSON.stringify([0,-0,1].sort()));"#,
        r#"print(JSON.stringify([{},[1],1,"a"].sort()));"#,

        /* ---- numeric comparator ---- */
        r#"function n(a,b){return a-b} print(JSON.stringify([3,1,2].sort(n)), JSON.stringify([10,9,1].sort(n)));"#,
        r#"function n(a,b){return b-a} print(JSON.stringify([1,2,3].sort(n)), JSON.stringify([].sort(n)), JSON.stringify([5].sort(n)));"#,
        r#"function n(a,b){return a-b} print(JSON.stringify([1,-1,0,-0,2].sort(n)));"#,
        r#"function n(a,b){return a<b?-1:(a>b?1:0)} print(JSON.stringify(["b","a","c","a"].sort(n)));"#,
        r#"print(JSON.stringify([3,1,2].sort(undefined)));"#,
        r#"try { [3,1,2].sort(null); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { [3,1,2].sort(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { [3,1,2].sort("x"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { [1].sort(1); print("len1-no-check"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { [].sort(1); print("len0-no-check"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"function n(a,b){return NaN} print(JSON.stringify([3,1,2].sort(n)));"#,
        r#"function n(a,b){return 0} print(JSON.stringify([3,1,2,5,4].sort(n)));"#,
        r#"function n(){return -1} print(JSON.stringify([1,2,3,4].sort(n)));"#,
        r#"function n(){return 1} print(JSON.stringify([1,2,3,4].sort(n)));"#,
        r#"function n(a,b){return a>b?-1:1} print(JSON.stringify([1,2,3,4,5].sort(n)));"#,
        r#"var s=[]; [2,1,3].sort(function(a,b){ s.push(a+"?"+b); return a-b; }); print(s.join(" "));"#,

        /* ---- throwing / mutating comparators ---- */
        r#"var a=[3,1,2]; try { a.sort(function(){ throw new Error("boom"); }); } catch(e) { print("caught", e.name, e.message); } print(JSON.stringify(a));"#,
        r#"var a=[3,1,2,5,4]; var n=0; try { a.sort(function(x,y){ if(++n===2) throw new TypeError("t"); return x-y; }); } catch(e) { print("caught", e.name, e.message); } print(a.length);"#,
        r#"var a=[3,1,2]; a.sort(function(x,y){ a.push(9); return x-y; }); print(a.length, JSON.stringify(a));"#,
        r#"var a=[3,1,2,4]; a.sort(function(x,y){ a.length=2; return x-y; }); print(a.length, JSON.stringify(a));"#,
        r#"var a=[3,1,2,4]; a.sort(function(x,y){ delete a[0]; return x-y; }); print(a.length, JSON.stringify(a));"#,
        r#"var a=[3,1,2]; a.sort(function(x,y){ a[0]="z"; return x<y?-1:1; }); print(JSON.stringify(a));"#,
        r#"[3,1,2].sort(function(){ throw new Error("uncaught-cmp"); });"#,

        /* ---- element counts around the heapsort code path ---- */
        r#"var a=[]; for(var i=0;i<2;i++) a.push((i*7)%2); a.sort(function(x,y){return x-y}); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<3;i++) a.push((i*7)%3); a.sort(function(x,y){return x-y}); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<17;i++) a.push((i*7)%17); a.sort(function(x,y){return x-y}); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<17;i++) a.push((i*11)%17); a.sort(); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<16;i++) a.push(16-i); a.sort(function(x,y){return x-y}); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<18;i++) a.push(18-i); a.sort(function(x,y){return x-y}); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<200;i++) a.push((i*7919)%200); a.sort(function(x,y){return x-y}); print(a.length, a[0], a[99], a[199], a.join(",").length);"#,
        r#"var a=[]; for(var i=0;i<200;i++) a.push((i*7919)%200); a.sort(); print(a[0], a[1], a[199], a.join(",").length);"#,
        r#"var a=[]; for(var i=0;i<200;i++) a.push(i%5); a.sort(function(x,y){return x-y}); print(a.join(""));"#,
        r#"var a=[]; for(var i=0;i<64;i++) a.push(0); a.sort(function(x,y){return x-y}); print(a.length, a.join("").length);"#,
        r#"var a=[]; for(var i=0;i<33;i++) a.push(i); a.sort(function(x,y){return y-x}); print(a.join(","));"#,

        /* ---- sparse / undefined / array-like receivers ---- */
        r#"var a=[]; a[0]=3; a[2]=1; a[5]=2; a.sort(function(x,y){return x-y}); print(a.length, JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var a=[3,undefined,1,,2]; a.sort(function(x,y){ if(x===undefined||y===undefined) return 0; return x-y; }); print(a.length, JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var o={length:3,0:"c",1:"a",2:"b"}; var r=Array.prototype.sort.call(o); print(r===o, o[0],o[1],o[2], o.length);"#,
        r#"var o={length:3,0:3,2:1}; Array.prototype.sort.call(o, function(x,y){return x-y}); print(o.length, "0" in o, "1" in o, "2" in o, o[0], o[1]);"#,
        r#"var o={length:0}; print(JSON.stringify(Array.prototype.sort.call(o)));"#,
        r#"print(Array.prototype.sort.call("cba"), Array.prototype.sort.call("a"));"#,
        r#"try { Array.prototype.sort.call(null); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var o={length:2,0:2,1:1}; Array.prototype.sort.call(o, function(x,y){return x-y}); print(o[0], o[1]);"#,

        /* ---- reverse on the same shapes ---- */
        r#"print(JSON.stringify([].reverse()), JSON.stringify([1].reverse()), JSON.stringify([1,2,3,4].reverse()));"#,
        r#"var a=[]; for(var i=0;i<17;i++) a.push(i); a.reverse(); print(a.join(","));"#,
        r#"var a=[]; for(var i=0;i<200;i++) a.push(i); a.reverse(); print(a[0], a[199], a.join(",").length);"#,
        r#"var a=[]; a[0]=1; a[4]=5; a.reverse(); print(a.length, JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var a=[1,undefined,3]; a.reverse(); print(JSON.stringify(a), Object.keys(a).join(","));"#,
        r#"var o={length:4,0:"a",3:"d"}; Array.prototype.reverse.call(o); print(o.length, "0" in o, "3" in o, o[0], o[3]);"#,
        r#"var a=[1,2,3]; a.reverse().reverse(); print(JSON.stringify(a));"#,
        r#"var a=[]; for(var i=0;i<17;i++) a.push(i); a.sort(function(x,y){return y-x}); a.reverse(); print(a.join(","));"#,
    ]);
}

/* ================================================================== */
/*  H7 - String builtins (every prototype method in jsstring.c)        */
/* ================================================================== */

#[test]
fn h7_string_builtins() {
    diff_scripts_both_modes(&[
        /* ---- String() / new String() / fromCharCode ---- */
        r#"print(String(), String(1), String(null), String(undefined), String(true), String([1,2]));"#,
        r#"print(String({}), String(-0), String(NaN), String(Infinity), String(1e21), String(1e-7));"#,
        r#"var s=new String("ab"); print(typeof s, s.length, s[0], s+"", s.valueOf(), s instanceof String);"#,
        r#"var s=new String(); print(s.length, JSON.stringify(s.valueOf()), typeof s);"#,
        r#"print(String.length, typeof String.prototype, String.prototype.length, JSON.stringify(String.prototype.valueOf()));"#,
        r#"print(String.fromCharCode(), JSON.stringify(String.fromCharCode()), String.fromCharCode().length);"#,
        r#"print(String.fromCharCode(65), String.fromCharCode(65,66,67), String.fromCharCode(0x263A));"#,
        r#"print(String.fromCharCode(233), String.fromCharCode(233).length, String.fromCharCode(0xD83D,0xDE00).length);"#,
        r#"print(String.fromCharCode(-1).charCodeAt(0), String.fromCharCode(65536).charCodeAt(0), String.fromCharCode(65.9));"#,
        r#"print(String.fromCharCode(NaN).charCodeAt(0), String.fromCharCode(undefined).charCodeAt(0), String.fromCharCode("65"));"#,

        /* ---- length / index access / UTF-8 ---- */
        r#"print("".length, "a".length, "abc".length, "héllo".length, "日本語".length);"#,
        r#"print("héllo"[1], "héllo".charAt(1), "日本語".charAt(1), "日本語"[2]);"#,
        r#"var s="😀"; print(s.length, s.charCodeAt(0), s.charCodeAt(1), s === "😀");"#,
        r#"var s="0123456789abcde"; print(s.length, s.charAt(14), s.charCodeAt(14));"#,
        r#"var s="0123456789abcdef"; print(s.length, s.charAt(15), s.slice(1).length);"#,
        r#"var s="0123456789abcdefg"; print(s.length, s.charAt(16), s.substring(1,16));"#,
        r#"print("abc"[0], "abc"[3], typeof "abc"[3], "abc"["1"], "abc"[-1]);"#,

        /* ---- toString / valueOf ---- */
        r#"print("a".toString(), "a".valueOf(), new String("a").toString(), new String("a").valueOf());"#,
        r#"try { String.prototype.toString.call(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { String.prototype.valueOf.call([]); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(String.prototype.toString.call(new String("z")), typeof String.prototype.valueOf.call(new String("")));"#,

        /* ---- charAt / charCodeAt ---- */
        r#"var s="abc"; print(s.charAt(0), s.charAt(2), JSON.stringify(s.charAt(3)), JSON.stringify(s.charAt(-1)));"#,
        r#"var s="abc"; print(JSON.stringify(s.charAt()), JSON.stringify(s.charAt(undefined)), s.charAt(1.9), s.charAt("1"));"#,
        r#"var s="abc"; print(s.charCodeAt(0), s.charCodeAt(2), s.charCodeAt(3), s.charCodeAt(-1), s.charCodeAt());"#,
        r#"print("héllo".charCodeAt(1), "日本語".charCodeAt(0), "日本語".charCodeAt(2), "".charCodeAt(0));"#,
        r#"print("abc".charCodeAt(1.7), "abc".charCodeAt(NaN), "abc".charCodeAt(Infinity), "abc".charCodeAt(-Infinity));"#,
        r#"print(String.prototype.charAt.call(12345, 2), String.prototype.charCodeAt.call(true, 0));"#,

        /* ---- concat ---- */
        r#"print("".concat(), "a".concat("b"), "a".concat("b","c",1,null), "a".concat(undefined));"#,
        r#"print("a".concat([1,2]), "a".concat({}), "".concat("").length, "日".concat("本").length);"#,
        r#"print(String.prototype.concat.call(1,2,3), String.prototype.concat.call(null===null,"x"));"#,

        /* ---- indexOf / lastIndexOf ---- */
        r#"var s="abcabc"; print(s.indexOf("a"), s.indexOf("a",1), s.indexOf("c",-5), s.indexOf("z"), s.indexOf(""));"#,
        r#"var s="abcabc"; print(s.indexOf("",3), s.indexOf("",99), s.indexOf("abc",99), s.indexOf("b",1.9));"#,
        r#"var s="abcabc"; print(s.lastIndexOf("a"), s.lastIndexOf("a",2), s.lastIndexOf("z"), s.lastIndexOf(""), s.lastIndexOf("",2));"#,
        r#"print("".indexOf(""), "".indexOf("a"), "".lastIndexOf(""), "a".indexOf("a",-99));"#,
        r#"print("abc".indexOf(), "abc".indexOf(undefined), "undefined".indexOf(), "abc".lastIndexOf());"#,
        r#"print("héllo".indexOf("l"), "héllo".indexOf("é"), "日本語".indexOf("本"), "日本語".lastIndexOf("語"));"#,
        r#"print("aXbXc".indexOf("X",2), "aXbXc".lastIndexOf("X",2), "aaa".indexOf("aa"), "aaa".lastIndexOf("aa"));"#,
        r#"print(String.prototype.indexOf.call(12321, "2"), String.prototype.lastIndexOf.call(12321, "2"));"#,

        /* ---- localeCompare ---- */
        r#"print("a".localeCompare("a"), "a".localeCompare("b"), "b".localeCompare("a"), "".localeCompare(""));"#,
        r#"print("abc".localeCompare("abd"), "abc".localeCompare("ab"), "ab".localeCompare("abc"));"#,
        r#"print("a".localeCompare(), "a".localeCompare(undefined), "1".localeCompare(1));"#,

        /* ---- slice / substring ---- */
        r#"var s="abcdef"; print(s.slice(), s.slice(1), s.slice(1,3), s.slice(-2), s.slice(-3,-1));"#,
        r#"var s="abcdef"; print(JSON.stringify(s.slice(3,1)), JSON.stringify(s.slice(99)), s.slice(-99), s.slice(0,99));"#,
        r#"var s="abcdef"; print(s.slice(undefined), s.slice(0,undefined), s.slice(1.9,3.9), s.slice(NaN,2));"#,
        r#"var s="abcdef"; print(s.substring(), s.substring(1), s.substring(1,3), s.substring(3,1), s.substring(-2));"#,
        r#"var s="abcdef"; print(s.substring(-99,99), JSON.stringify(s.substring(99)), s.substring(NaN,2), s.substring(2,NaN));"#,
        r#"print("héllo".slice(1,3), "héllo".substring(1,3), "日本語".slice(1), "日本語".substring(0,2));"#,
        r#"print("".slice(0,1), JSON.stringify("".substring(0,1)), "a".slice(0,1), "a".substring(1));"#,
        r#"print(String.prototype.slice.call(123456,1,3), String.prototype.substring.call(123456,2));"#,
        r#"try { print("abc".substr(1)); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(typeof "abc".substr, "substr" in String.prototype);"#,

        /* ---- case conversion / trim ---- */
        r#"print("".toUpperCase(), "abc".toUpperCase(), "ABC".toLowerCase(), "AbC".toLowerCase());"#,
        r#"print("héllo".toUpperCase(), "HÉLLO".toLowerCase(), "日本語".toUpperCase(), "ß".toUpperCase());"#,
        r#"print("abc".toLocaleUpperCase(), "ABC".toLocaleLowerCase(), "Straße".toUpperCase());"#,
        r#"print("0123456789abcde".toUpperCase(), "0123456789abcdef".toUpperCase(), "0123456789abcdefg".toUpperCase());"#,
        r#"print(String.prototype.toUpperCase.call(123), String.prototype.toLowerCase.call(true));"#,
        r#"print(JSON.stringify("  a b  ".trim()), JSON.stringify("".trim()), JSON.stringify("   ".trim()));"#,
        r#"print(JSON.stringify("\t\n\r x \t\n\r".trim()), JSON.stringify(" x ".trim()));"#,
        r#"print(String.prototype.trim.call(" 1 "), JSON.stringify(String.prototype.trim.call(null===null)));"#,

        /* ---- split with string separators ---- */
        r#"print(JSON.stringify("a,b,c".split(",")), JSON.stringify("a,b,c".split()), JSON.stringify("".split(",")));"#,
        r#"print(JSON.stringify("".split("")), "".split("").length, JSON.stringify("abc".split("")));"#,
        r#"print(JSON.stringify("a,b,,c".split(",")), JSON.stringify(",a,".split(",")));"#,
        r#"print(JSON.stringify("abc".split("b")), JSON.stringify("abc".split("z")), JSON.stringify("aaa".split("a")));"#,
        r#"print(JSON.stringify("日本語".split("")), "日本語".split("").length);"#,
        r#"print(JSON.stringify("a1b2c".split(1)), JSON.stringify("a,b".split(undefined)), JSON.stringify("a,b".split(null)));"#,
        r#"var s="a,b,c,d,e"; for (var i=0;i<=5;i++) print(i, JSON.stringify(s.split(",",i)));"#,
        r#"print(JSON.stringify("a,b,c".split(",",undefined)), JSON.stringify("a,b,c".split(",",-1)), JSON.stringify("a,b,c".split(",",1.9)));"#,
        r#"print(JSON.stringify("abc".split("",2)), JSON.stringify("abc".split("",0)));"#,

        /* ---- split with regexp separators ---- */
        r#"print(JSON.stringify("a1b2c".split(/\d/)), JSON.stringify("a1b2c".split(/(\d)/)));"#,
        r#"print(JSON.stringify("abc".split(/b/)), JSON.stringify("abc".split(/(b)/)), JSON.stringify("abc".split(/x/)));"#,
        r#"print(JSON.stringify("abc".split(/(?:)/)), JSON.stringify("abc".split(new RegExp(""))));"#,
        r#"print(JSON.stringify("a1b22c".split(/(\d)(\d)?/)));"#,
        r#"print(JSON.stringify("a,b;c".split(/[,;]/)), JSON.stringify("AxBxC".split(/x/i)));"#,
        r#"var s="a1b2c3d"; for (var i=0;i<=5;i++) print(i, JSON.stringify(s.split(/\d/,i)));"#,
        r#"print(JSON.stringify("aaa".split(/a/)), JSON.stringify("".split(/a/)), JSON.stringify("".split(/(?:)/)));"#,
        r#"print(JSON.stringify(String.prototype.split.call(12321, /2/)));"#,

        /* ---- replace with string replacements ---- */
        r#"print("abc".replace("b","X"), "abc".replace("z","X"), "abcabc".replace("b","X"));"#,
        r#"print("abc".replace(/b/,"X"), "abcabc".replace(/b/g,"X"), "abc".replace(/(b)/,"[$1]"));"#,
        r#"print("aaa".replace(/a/g,"$$"), "aaa".replace(/a/,"$&$&"), "abc".replace(/b/,"$`|$'"));"#,
        r#"print("abc".replace(/(a)(b)(c)/,"$3$2$1"), "abc".replace(/(a)/,"$1$2"), "abc".replace(/(a)/,"$9"));"#,
        r#"print("abc".replace(/(a)/,"$10"), "abc".replace(/(a)(b)/,"$10"), "abc".replace(/b/,"$0"));"#,
        r#"print("abc".replace(/b/,"$"), "abc".replace(/b/,"$x"), "abc".replace(/b/,"a$$b"));"#,
        r#"print("a-b-c".replace(/-/g,"+"), "a-b-c".replace("-","+"), "".replace(/x/g,"y"));"#,
        r#"print("abc".replace("","X"), "abc".replace(/(?:)/,"X"), "abc".replace(/(?:)/g,"X"));"#,
        r#"print("héllo".replace("é","e"), "日本語".replace("本","X"), "日本語".replace(/./,"Y"));"#,
        r#"print("abc".replace(/b/,1), "abc".replace(/b/,null), "abc".replace(/b/,undefined));"#,
        r#"print("abcABC".replace(/b/gi,"X"), "aAbB".replace(/[ab]/g,"-"));"#,

        /* ---- replace with a function replacement ---- */
        r#"print("abc".replace(/b/, function(m){ return "["+m+"]"; }));"#,
        r#"var s=[]; "abc".replace(/b/, function(){ s.push(arguments.length); for(var i=0;i<arguments.length;i++) s.push(i+"="+arguments[i]); return "X"; }); print(s.join(" "));"#,
        r#"var s=[]; "a1b2".replace(/([a-z])(\d)/g, function(m,p1,p2,off,str){ s.push([m,p1,p2,off,str].join("/")); return m; }); print(s.join(" "));"#,
        r#"print("abcabc".replace(/b/g, function(m,off){ return off; }));"#,
        r#"print("abc".replace("b", function(m,off,str){ return m+off+str; }));"#,
        r#"print("abc".replace(/(x)?b/, function(m,p1){ return typeof p1; }));"#,
        r#"print("aaa".replace(/a/g, function(){ return ""; }), JSON.stringify("aaa".replace(/a/g, function(){ return ""; })));"#,
        r#"print("abc".replace(/b/, function(){ return 5; }), "abc".replace(/b/, function(){ return; }));"#,
        r#"try { "abc".replace(/b/, function(){ throw new Error("rep"); }); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- match / search ---- */
        r#"var m="abcabc".match(/b/); print(m.length, m[0], m.index, m.input);"#,
        r#"var m="abcabc".match(/b/g); print(m.length, JSON.stringify(m), m.index, m.input);"#,
        r#"var m="a1b2".match(/([a-z])(\d)/); print(m.length, JSON.stringify(m), m.index);"#,
        r#"print("abc".match(/z/), "abc".match(/z/g), typeof "abc".match(/z/));"#,
        r#"var m="abc".match("b"); print(m[0], m.index, m.length);"#,
        r#"var m="abc".match(); print(m.length, JSON.stringify(m), m.index);"#,
        r#"print(JSON.stringify("aaa".match(/a/g)), JSON.stringify("".match(/(?:)/g)));"#,
        r#"var m="日本語".match(/./); print(m[0], m.index, m[0].length);"#,
        r#"print("abcabc".search(/b/), "abc".search(/z/), "abc".search(), "abc".search(/(?:)/));"#,
        r#"print("abc".search("c"), "abc".search(/c/g), "".search(/a/), "日本語".search(/語/));"#,
        r#"var re=/b/g; re.lastIndex=5; print("abcabc".search(re), re.lastIndex);"#,
        r#"var re=/b/g; print(JSON.stringify("abcabc".match(re)), re.lastIndex);"#,

        /* ---- receivers / uncaught ---- */
        r#"try { String.prototype.charAt.call(null,0); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { String.prototype.split.call(undefined,","); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { String.prototype.replace.call(null,"a","b"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"String.prototype.trim.call(null);"#,
        r#"print(Object.getOwnPropertyNames(String.prototype).join(","));"#,
    ]);
}

/* ================================================================== */
/*  H8 - Number builtins & number-to-string                            */
/* ================================================================== */

#[test]
fn h8_number_builtins() {
    diff_scripts_both_modes(&[
        /* ---- Number() conversions ---- */
        r#"print(Number(), Number(0), Number(1), Number(-1), Number(1.5), Number(-0), 1/Number(-0));"#,
        r#"print(Number(""), Number(" "), Number("\t\n"), Number("1"), Number(" 1 "), Number("1x"));"#,
        r#"print(Number("0x10"), Number("0X10"), Number("010"), Number("1e3"), Number("1e-3"), Number(".5"));"#,
        r#"print(Number("Infinity"), Number("-Infinity"), Number("+Infinity"), Number("infinity"), Number("NaN"));"#,
        r#"print(Number(true), Number(false), Number(null), Number(undefined), Number(NaN));"#,
        r#"print(Number([]), Number([5]), Number([1,2]), Number({}), Number(new Number(3)));"#,
        r#"print(Number(new String("7")), Number(new Boolean(true)), Number({valueOf:function(){return 9}}));"#,
        r#"print(Number("1.7976931348623157e+308"), Number("1e309"), Number("5e-324"), Number("1e-400"));"#,
        r#"print(typeof Number(1), typeof new Number(1), new Number(1) instanceof Number, Number.length);"#,
        r#"var n=new Number(5); print(n.valueOf(), n+1, n.toString(), typeof n, JSON.stringify(n));"#,
        r#"print(Number.MAX_VALUE, Number.MIN_VALUE, Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY);"#,
        r#"print(Number.MAX_VALUE===1.7976931348623157e308, Number.MIN_VALUE===5e-324, Number.NaN===Number.NaN);"#,
        r#"print(JSON.stringify(Object.getOwnPropertyDescriptor(Number,"MAX_VALUE")), Object.keys(Number).length);"#,

        /* ---- String(v) / ""+v for the interesting value set ---- */
        r#"print(String(0), String(-0), String(1), String(-1), String(0.5), String(-0.5));"#,
        r#"print(""+0, ""+-0, ""+1e21, ""+1e-7, ""+NaN, ""+Infinity, ""+-Infinity);"#,
        r#"print(String(1e20), String(1e21), String(1e-6), String(1e-7), String(123456789012345678901));"#,
        r#"print(String(0.1), String(0.2), String(0.1+0.2), String(1/3), String(2/3));"#,
        r#"print(String(9007199254740992), String(9007199254740993), String(Math.pow(2,53)), String(Math.pow(2,53)+2));"#,
        r#"print(String(-1e-323), String(5e-324), String(1.7976931348623157e308), String(2.2250738585072014e-308));"#,
        r#"print(String(100), String(1000000), String(10000000000000000000), String(0.000001), String(0.0000001));"#,
        r#"var vs=[0,-0,1,-1,0.5,-0.5,NaN,Infinity,-Infinity,1e21,1e-7,4294967296,2147483648]; for(var i=0;i<vs.length;i++) print(i, String(vs[i]), ""+vs[i], vs[i].toString());"#,

        /* ---- toString(radix) ---- */
        r#"print((255).toString(), (255).toString(10), (255).toString(2), (255).toString(8), (255).toString(16), (255).toString(36));"#,
        r#"for (var r=2;r<=36;r++) print(r, (255).toString(r));"#,
        r#"for (var r=2;r<=36;r++) print(r, (-1234567).toString(r));"#,
        r#"for (var r=2;r<=36;r++) print(r, (0).toString(r), (-0).toString(r));"#,
        r#"for (var r=2;r<=36;r++) print(r, (0.5).toString(r));"#,
        r#"for (var r=2;r<=36;r++) print(r, NaN.toString(r), Infinity.toString(r), (-Infinity).toString(r));"#,
        r#"for (var r=2;r<=36;r++) print(r, (1e21).toString(r));"#,
        r#"for (var r=2;r<=36;r++) print(r, (9007199254740992).toString(r));"#,
        r#"print((1e-7).toString(2), (1e-7).toString(16), (123.456).toString(16), (-123.456).toString(2));"#,
        r#"try { (5).toString(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (5).toString(37); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (5).toString(0); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print((5).toString(undefined), (5).toString(10.9), (5).toString("16"), (255).toString(16.9));"#,
        r#"try { (5).toString(NaN); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print((5).toLocaleString(), (1234.5).toLocaleString(), Number.prototype.toLocaleString.call(7));"#,
        r#"try { Number.prototype.toString.call("5"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(Number.prototype.toString.call(new Number(31), 16), Number.prototype.valueOf.call(new Number(2)));"#,
        r#"try { Number.prototype.valueOf.call({}); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- toFixed ---- */
        r#"for (var d=0;d<=20;d++) print(d, (1.5).toFixed(d));"#,
        r#"for (var d=0;d<=20;d++) print(d, (0).toFixed(d), (-0).toFixed(d));"#,
        r#"for (var d=0;d<=20;d++) print(d, (123.456).toFixed(d));"#,
        r#"for (var d=0;d<=10;d++) print(d, (-123.456).toFixed(d), (0.000001).toFixed(d));"#,
        r#"for (var d=0;d<=10;d++) print(d, NaN.toFixed(d), Infinity.toFixed(d), (-Infinity).toFixed(d));"#,
        r#"for (var d=0;d<=10;d++) print(d, (1e21).toFixed(d), (1e-7).toFixed(d));"#,
        r#"print((2.5).toFixed(0), (1.5).toFixed(0), (0.5).toFixed(0), (-0.5).toFixed(0), (1.005).toFixed(2));"#,
        r#"print((9007199254740992).toFixed(2), (1.7976931348623157e308).toFixed(0).length);"#,
        r#"print((5).toFixed(), (5).toFixed(undefined), (5.678).toFixed(1.9));"#,
        r#"try { (1).toFixed(21); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (1).toFixed(-1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (1).toFixed(101); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- toExponential ---- */
        r#"for (var d=0;d<=20;d++) print(d, (123.456).toExponential(d));"#,
        r#"for (var d=0;d<=20;d++) print(d, (0).toExponential(d));"#,
        r#"for (var d=0;d<=10;d++) print(d, (-0).toExponential(d), (1).toExponential(d));"#,
        r#"for (var d=0;d<=10;d++) print(d, (1e21).toExponential(d), (1e-7).toExponential(d));"#,
        r#"for (var d=0;d<=10;d++) print(d, NaN.toExponential(d), Infinity.toExponential(d), (-Infinity).toExponential(d));"#,
        r#"print((5).toExponential(), (0).toExponential(), (1.5).toExponential(), (-1.5).toExponential());"#,
        r#"print((9007199254740992).toExponential(5), (1.7976931348623157e308).toExponential(3), (5e-324).toExponential(3));"#,
        r#"try { (1).toExponential(21); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (1).toExponential(-1); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- toPrecision ---- */
        r#"for (var p=1;p<=21;p++) print(p, (123.456).toPrecision(p));"#,
        r#"for (var p=1;p<=21;p++) print(p, (0).toPrecision(p));"#,
        r#"for (var p=1;p<=10;p++) print(p, (-0).toPrecision(p), (1).toPrecision(p));"#,
        r#"for (var p=1;p<=10;p++) print(p, (1e21).toPrecision(p), (1e-7).toPrecision(p));"#,
        r#"for (var p=1;p<=10;p++) print(p, NaN.toPrecision(p), Infinity.toPrecision(p), (-Infinity).toPrecision(p));"#,
        r#"for (var p=1;p<=10;p++) print(p, (0.000001).toPrecision(p), (9007199254740992).toPrecision(p));"#,
        r#"print((5).toPrecision(), (5).toPrecision(undefined), (1.5).toPrecision(2.9));"#,
        r#"try { (1).toPrecision(0); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (1).toPrecision(22); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (1).toPrecision(-1); } catch(e) { print("caught", e.name, e.message); }"#,

        /* ---- uncaught paths ---- */
        r#"(5).toString(99);"#,
        r#"Number.prototype.toFixed.call("x", 2);"#,
    ]);
}

/* ================================================================== */
/*  H9 - Math (every function in jsmath.c except random)               */
/* ================================================================== */

#[test]
fn h9_math_builtins() {
    // Shared prologue: V is the argument set, and `show` prints the result with
    // full precision plus 1/x so that -0 is distinguishable from 0.
    const P: &str = concat!(
        "var V=[-0,0,0.5,-0.5,1,-1,2,-2,0.1,1e300,-1e300,1e-300,",
        "NaN,Infinity,-Infinity,Math.PI,Math.E,1e21,-1e21];",
        "function show(t,x){ print(t, x.toString(), 1/x); }",
        "function one(nm,f){ for (var i=0;i<V.length;i++) show(nm+'('+V[i]+')', f(V[i])); }",
    );

    let mut owned: Vec<String> = Vec::new();

    /* one script per Math function, sweeping the whole argument set */
    for name in [
        "abs", "acos", "asin", "atan", "ceil", "cos", "exp", "floor", "log", "round", "sin",
        "sqrt", "tan",
    ] {
        owned.push(format!(
            r#"{}one("{}", function(x){{ return Math.{}(x); }});"#,
            P, name, name
        ));
        /* zero-argument call */
        owned.push(format!(
            r#"show("{}()", Math.{}());"#,
            name, name
        ));
        /* extra arguments are ignored */
        owned.push(format!(
            r#"var P=Math.{}; print("{}", P(1,2,3).toString(), P.length);"#,
            name, name
        ));
    }

    /* two-argument functions across the full cross product */
    for name in ["atan2", "pow"] {
        owned.push(format!(
            r#"{}for (var i=0;i<V.length;i++) for (var j=0;j<V.length;j++) show("{}("+V[i]+","+V[j]+")", Math.{}(V[i],V[j]));"#,
            P, name, name
        ));
        owned.push(format!(
            r#"show("{}()", Math.{}()); show("{}(1)", Math.{}(1)); print(Math.{}.length);"#,
            name, name, name, name, name
        ));
    }

    let mut scripts: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();

    /* max / min with 0/1/3/5 arguments, NaN and -0 vs 0 */
    let extra: &[&str] = &[
        r#"print(Math.max(), Math.min(), 1/Math.max(), 1/Math.min());"#,
        r#"print(Math.max(1), Math.min(1), Math.max(-0), 1/Math.max(-0), 1/Math.min(-0));"#,
        r#"print(Math.max(-0,0), 1/Math.max(-0,0), Math.max(0,-0), 1/Math.max(0,-0));"#,
        r#"print(Math.min(-0,0), 1/Math.min(-0,0), Math.min(0,-0), 1/Math.min(0,-0));"#,
        r#"print(Math.max(1,2,3), Math.min(1,2,3), Math.max(3,2,1), Math.min(3,2,1));"#,
        r#"print(Math.max(1,2,3,4,5), Math.min(5,4,3,2,1), Math.max(-1,-2,-3,-4,-5));"#,
        r#"print(Math.max(NaN,1), Math.min(NaN,1), Math.max(1,NaN,2), Math.min(1,NaN,2));"#,
        r#"print(Math.max(NaN), Math.min(NaN), Math.max(Infinity,NaN), Math.min(-Infinity,NaN));"#,
        r#"print(Math.max(Infinity,-Infinity), Math.min(Infinity,-Infinity), Math.max(1e300,1e21));"#,
        r#"print(Math.max("5",2), Math.min("5",2), Math.max(null,-1), Math.min(undefined,1));"#,
        r#"print(Math.max(true,0.5), Math.min(false,-0.5), Math.max([2],1), Math.min({},1));"#,
        r#"print(Math.max.length, Math.min.length, typeof Math.max, typeof Math.min);"#,
        /* constants */
        r#"print(Math.E, Math.LN10, Math.LN2, Math.LOG2E, Math.LOG10E, Math.PI, Math.SQRT1_2, Math.SQRT2);"#,
        r#"print(Math.E.toString(), Math.PI.toString(), Math.SQRT2.toString(), Math.LN2.toString());"#,
        r#"print(Math.PI===3.141592653589793, Math.E===2.718281828459045, Math.SQRT2*Math.SQRT2);"#,
        r#"print(Object.getOwnPropertyNames(Math).join(","), typeof Math, Math.toString());"#,
        r#"print(JSON.stringify(Object.getOwnPropertyDescriptor(Math,"PI")), Object.keys(Math).length);"#,
        /* interesting identities and rounding corners */
        r#"print(Math.round(0.5), Math.round(-0.5), 1/Math.round(-0.5), Math.round(1.5), Math.round(-1.5), Math.round(2.5));"#,
        r#"print(Math.round(-0), 1/Math.round(-0), Math.round(0.49999999999999994), Math.round(4503599627370497.0));"#,
        r#"print(Math.ceil(-0.5), 1/Math.ceil(-0.5), Math.floor(-0), 1/Math.floor(-0), Math.ceil(-0), 1/Math.ceil(-0));"#,
        r#"print(Math.abs(-0), 1/Math.abs(-0), Math.abs(-Infinity), Math.abs("-5"), Math.abs("x"));"#,
        r#"print(Math.pow(0,0), Math.pow(-0,-1), Math.pow(-1,0.5), Math.pow(1,Infinity), Math.pow(-1,Infinity));"#,
        r#"print(Math.pow(2,1024), Math.pow(2,-1075), Math.pow(-2,3), Math.pow(-2,3.5), Math.pow(2,53));"#,
        r#"print(Math.sqrt(-0), 1/Math.sqrt(-0), Math.sqrt(-1), Math.sqrt(4), Math.sqrt(2));"#,
        r#"print(Math.log(0), Math.log(-1), Math.log(1), Math.log(Math.E), Math.exp(0), Math.exp(1), Math.exp(710));"#,
        r#"print(Math.atan2(0,0), Math.atan2(-0,0), Math.atan2(0,-0), Math.atan2(-0,-0), Math.atan2(1,0));"#,
        r#"print(Math.atan2(Infinity,Infinity), Math.atan2(-Infinity,-Infinity), Math.atan2(1,-0));"#,
        r#"print(Math.sin(0), 1/Math.sin(-0), Math.cos(0), Math.tan(0), Math.sin(Math.PI), Math.cos(Math.PI));"#,
        r#"print(Math.asin(1), Math.asin(2), Math.acos(1), Math.acos(2), Math.atan(Infinity), Math.atan(-Infinity));"#,
        r#"print(Math.floor(1e21), Math.ceil(1e21), Math.round(1e21), Math.floor(-1e21));"#,
        r#"print(Math.abs("5"), Math.floor(null), Math.ceil(undefined), Math.round([2]), Math.sqrt({}));"#,
        r#"print(Math.abs.length, Math.pow.length, Math.atan2.length, Math.round.length);"#,
        r#"try { new Math.abs(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Math(); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(typeof Math.random, Math.random.length, "random" in Math);"#,
        r#"Math.floor();"#,
    ];
    scripts.extend_from_slice(extra);

    diff_scripts_both_modes(&scripts);
}

/* ================================================================== */
/*  H10 - Boolean and Function.prototype methods                       */
/* ================================================================== */

#[test]
fn h10_boolean_and_function_methods() {
    diff_scripts_both_modes(&[
        /* ---- Boolean() / new Boolean() ---- */
        r#"print(Boolean(), Boolean(true), Boolean(false), Boolean(0), Boolean(1), Boolean(-0));"#,
        r#"print(Boolean(""), Boolean("0"), Boolean("false"), Boolean(NaN), Boolean(Infinity));"#,
        r#"print(Boolean(null), Boolean(undefined), Boolean({}), Boolean([]), Boolean(function(){}));"#,
        r#"print(Boolean(new Boolean(false)), Boolean(new Number(0)), Boolean(new String("")));"#,
        r#"print(typeof Boolean(1), typeof new Boolean(1), Boolean.length, typeof Boolean.prototype);"#,
        r#"var b=new Boolean(false); print(typeof b, b.valueOf(), !!b, b?1:0, b+"", JSON.stringify(b));"#,
        r#"var b=new Boolean(0); print(b.valueOf(), b.toString(), b==false, b===false);"#,
        r#"print(new Boolean().valueOf(), new Boolean(undefined).valueOf(), new Boolean([]).valueOf());"#,
        r#"print(Boolean.prototype.valueOf(), Boolean.prototype.toString(), Boolean.prototype.constructor===Boolean);"#,
        r#"print(Object.getOwnPropertyNames(Boolean.prototype).join(","), Object.keys(Boolean).length);"#,

        /* ---- Boolean.prototype.toString / valueOf on wrong receivers ---- */
        r#"print(true.toString(), false.toString(), (true).valueOf(), typeof true.toString());"#,
        r#"print(Boolean.prototype.toString.call(true), Boolean.prototype.valueOf.call(false));"#,
        r#"print(Boolean.prototype.toString.call(new Boolean(true)), Boolean.prototype.valueOf.call(new Boolean(true)));"#,
        r#"try { Boolean.prototype.toString.call(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Boolean.prototype.valueOf.call("x"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Boolean.prototype.toString.call(null); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Boolean.prototype.valueOf.call({}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Boolean.prototype.toString.call([]); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"Boolean.prototype.valueOf.call(3);"#,

        /* ---- Function.prototype.call ---- */
        r#"function f(){ return typeof this; } print(f.call(), f.call(null), f.call(undefined));"#,
        r#"function f(){ return this===undefined ? "u" : Object.prototype.toString.call(this); } print(f.call(1), f.call("s"), f.call(true));"#,
        r#"function f(){ return this.v; } print(f.call({v:7}), f.call(new Number(3))===undefined);"#,
        r#"function f(a,b){ return [this===undefined?"u":this.v, a, b].join("|"); } print(f.call({v:1},2,3), f.call({v:1}), f.call({v:1},2,3,4));"#,
        r#"function f(){ return arguments.length; } print(f.call(null), f.call(null,1), f.call(null,1,2,3,4,5,6,7,8));"#,
        r#"print(Function.prototype.call.length, Function.prototype.apply.length, Function.prototype.bind.length);"#,
        r#"try { Function.prototype.call.call(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"print(Math.max.call(null,1,5,3), Object.prototype.toString.call([]), Array.prototype.join.call([1,2],"-"));"#,

        /* ---- Function.prototype.apply ---- */
        r#"function f(a,b){ return [a,b].join("|"); } print(f.apply(null,[1,2]), f.apply(null,[1]), f.apply(null,[]));"#,
        r#"function f(){ return arguments.length; } print(f.apply(null), f.apply(null,undefined), f.apply(null,[]));"#,
        r#"function f(){ return arguments.length; } print(f.apply(null,{length:3,0:"a",1:"b",2:"c"}));"#,
        r#"function f(a,b,c){ return [a,b,c].join("|"); } print(f.apply(null,{length:3,0:"a",2:"c"}));"#,
        r#"function f(){ return Array.prototype.join.call(arguments,","); } function g(){ return f.apply(null,arguments); } print(g(1,2,3));"#,
        r#"function f(){ return this.v; } print(f.apply({v:"o"},[]), f.apply({v:"o"}));"#,
        r#"try { (function(){}).apply(null,1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { (function(){}).apply(null,"ab"); print("string-arglist-ok"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"function f(){ return arguments.length; } var a=[]; for (var i=0;i<40;i++) a.push(i); print(f.apply(null,a));"#,
        r#"print(Math.max.apply(null,[1,9,3]), Math.min.apply(null,[1,9,3]), Math.max.apply(null,[]));"#,

        /* ---- Function.prototype.bind ---- */
        r#"function f(a,b){ return [this===undefined?"u":this.v,a,b].join("|"); } var b=f.bind({v:1}); print(b(2,3), b(), b.length);"#,
        r#"function f(a,b,c){ return [a,b,c].join("|"); } var b=f.bind(null,1); print(b(2,3), b(2), b(), b.length);"#,
        r#"function f(a,b,c){ return [a,b,c].join("|"); } var b=f.bind(null,1,2,3); print(b(), b(9), b.length);"#,
        r#"function f(a,b,c,d){} print(f.bind(null).length, f.bind(null,1).length, f.bind(null,1,2).length, f.bind(null,1,2,3,4,5).length);"#,
        r#"function f(){ return this.v; } var b=f.bind({v:"a"}); var b2=b.bind({v:"b"}); print(b(), b2());"#,
        r#"function f(a,b){ return a+"/"+b; } var b=f.bind(null,1).bind(null,2); print(b(), b(3));"#,
        r#"function P(a,b){ this.a=a; this.b=b; } var B=P.bind(null,1); var o=new B(2); print(o.a, o.b, o instanceof P);"#,
        r#"function P(a){ this.a=a; } var B=P.bind({v:9}, 5); var o=new B(); print(o.a, o.v, o instanceof P);"#,
        r#"var b=Math.max.bind(null,1,2); print(b(), b(3), b(0), b.length);"#,
        r#"var b=Object.prototype.toString.bind([]); print(b());"#,
        r#"var b=Array.prototype.join.bind([1,2,3]); print(b("-"), b(), b.length);"#,
        r#"try { Function.prototype.bind.call(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { ({}).bind; print(typeof ({}).bind); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var b=(function(){ return arguments.length; }).bind(null,1,2); print(b(), b(3), b(3,4));"#,
        r#"var b=(function(){}).bind(null); print(typeof b, b instanceof Function, typeof b.prototype);"#,
        r#"var b=(function(){}).bind(null); print(Object.getOwnPropertyNames(b).sort().join(","));"#,

        /* ---- Function.prototype.toString ---- */
        r#"function f(a,b){ return a+b; } print(f.toString());"#,
        r#"print((function(){}).toString(), (function g(x){}).toString());"#,
        // NOTE: `print.toString()` is deliberately *not* used here - see the
        // "HARNESS ARTIFACTS" note at the bottom of this file.
        r#"print(Math.max.toString(), Object.keys.toString(), Array.prototype.slice.toString());"#,
        r#"print(Math.abs.toString(), JSON.stringify.toString(), Number.prototype.toFixed.toString());"#,
        r#"print(Function.prototype.toString.call(function(a,b,c){}), (function(){}).bind(null).toString());"#,
        r#"print(Function.toString(), Object.toString(), Array.toString(), Boolean.toString());"#,
        r#"print(Function.prototype.toString(), typeof Function.prototype.toString());"#,
        r#"try { Function.prototype.toString.call({}); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { Function.prototype.toString.call(1); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"function f(){} print(""+f, String(f), f+"", [f].join(""));"#,

        /* ---- new Function(...) ---- */
        r#"var f=new Function("a","b","return a+b"); print(f(3,4), f.length, typeof f);"#,
        r#"var f=Function("return 42"); print(f(), f.length, f.toString());"#,
        r#"var f=new Function("return this===undefined?'u':typeof this"); print(f());"#,
        r#"var f=new Function("a,b","return a*b"); print(f(3,4), f.length);"#,
        r#"var f=new Function(); print(typeof f, f(), f.length);"#,
        r#"var f=new Function("x","return x*2"); print(f(5), f.toString());"#,
        r#"try { Function("return )"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { new Function("a","var 1;"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"try { new Function("1a","return 1"); } catch(e) { print("caught", e.name, e.message); }"#,
        r#"var f=new Function("return arguments.length"); print(f(), f(1,2,3));"#,
        r#"print(new Function("return 1")() + new Function("return 2")());"#,
        r#"print(Function.length, Function.prototype.length, typeof Function.prototype.constructor);"#,
        r#"Function("return )");"#,
    ]);
}

/* ==================================================================
 *  CRASHES: (scripts that abort/segfault the test process rather than
 *  merely diverge)
 *
 *  None.  Every script in this file runs to completion in both
 *  libraries; the whole corpus was pre-screened against both shared
 *  objects before being committed here.
 * ==================================================================
 */

/* ==================================================================
 *  KNOWN DIVERGENCES (left in place on purpose - they are bugs in the
 *  Rust translation, to be fixed separately, NOT test bugs)
 *
 *  h9_math_builtins, `Math.round` on -0.5 <= x < 0:
 *      C   : Math.round(-0.5) === +0   (1/Math.round(-0.5) ===  Infinity)
 *      Rust: Math.round(-0.5) === -0   (1/Math.round(-0.5) === -Infinity)
 *  Cause: c_src/src/jsmath.c jsM_round() ends the negative branch with
 *  `return -0;` - in C that is the *integer* constant 0 negated, i.e.
 *  plain 0, which converts to +0.0.  src/jsmath.rs translated it as the
 *  floating literal `-0.0`, which really is negative zero.  The faithful
 *  translation is `return 0.0;`.
 *  Affected scripts: h9 [27] (the `Math.round` sweep) and h9 [60].
 * ==================================================================
 */

/* ==================================================================
 *  HARNESS ARTIFACTS (deliberately avoided in the corpus above)
 *
 *  `print.toString()` must not be used.  `js_newcfunctionx` (jsvalue.c)
 *  stores the `name` argument as a bare pointer without copying it:
 *      obj->u.c.name = name;
 *  and tests/common/mod.rs registers the global `print` with
 *      (api.js_newcfunction)(J, Some(print_cb), cs("print").as_ptr(), 1)
 *  where the `CString` temporary is dropped at the end of the statement.
 *  `Function.prototype.toString` on that cfunction therefore reads freed
 *  heap memory and prints garbage that differs run-to-run and between the
 *  two libraries.  This is a use-after-free in the *harness*, not a
 *  divergence in the interpreter, so h10 exercises
 *  `Function.prototype.toString` on `Math.max`, `Object.keys`,
 *  `Array.prototype.slice`, `JSON.stringify` and
 *  `Number.prototype.toFixed` instead (all registered from string
 *  literals with static lifetime).
 *
 *  Note also that `debugger;` (h2) makes both libraries dump the VM stack
 *  to *stderr*; the harness only captures `print` output and the report
 *  callback, so those dumps show up in the test log but are not compared.
 * ==================================================================
 */
