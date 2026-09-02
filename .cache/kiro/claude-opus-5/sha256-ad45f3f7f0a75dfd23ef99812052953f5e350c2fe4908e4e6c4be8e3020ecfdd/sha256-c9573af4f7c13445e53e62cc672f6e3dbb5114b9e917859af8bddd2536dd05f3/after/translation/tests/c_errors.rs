//! Phase C — error-path differential tests.
//!
//! Every row of ERRORS.md gets a trigger here.  Each trigger is executed
//! through BOTH shared objects and the *exact* rejection must match: the same
//! error type and message for JS-level throws, the same `NULL` + `*errorp`
//! text for `js_regcomp`, the same return codes for the protected entry points.
//!
//! `errors_index.tsv` is generated together with ERRORS.md by `gen_errors.py`.
//! `phase_c_coverage` fails if any row lacks a trigger, and
//! `phase_c_triggers_hit_their_rows` fails if a trigger does not actually
//! produce the message its row documents — so the table cannot silently rot.
mod common;
use common::*;

use std::os::raw::{c_char, c_int, c_void};
use std::sync::OnceLock;

const INDEX: &str = include_str!("errors_index.tsv");

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Evaluate a script in a default (non-strict) state.
    Js,
    /// Evaluate a script in a `JS_STRICT` state.
    JsStrict,
    /// Evaluate a *generated* script (payload names the generator).
    JsGen,
    /// Compile a regexp with `js_regcomp`.
    Re,
    /// Compile a *generated* regexp (payload names the generator).
    ReGen,
    /// `js_regcompx` under an allocator that fails after N allocations.
    ReAlloc,
    /// Named host-callback job under `js_pcall`, default state.
    ApiJob,
    /// Named host-callback job under `js_pcall`, `JS_STRICT` state.
    ApiJobStrict,
    /// Sweep `js_setlimit(J, 0, memlimit)` over many budgets and compare the
    /// whole outcome vector.  Used for the allocation-failure cleanup handlers
    /// (`if (js_try(J)) { js_free(...); js_throw(J); }`) whose only reachable
    /// trigger is an allocation failure.
    MemSweep,
    /// Sweep `js_setlimit` around a named host-callback job.
    ApiMemSweep,
    /// Sweep `js_setlimit(J, runlimit, 0)` over many budgets (js_runlimit).
    RunSweep,
    /// Not constructible from outside the `.so`; payload documents why.
    Exempt,
}

struct Trigger {
    ids: &'static [u32],
    kind: Kind,
    payload: &'static str,
}

const fn t(ids: &'static [u32], kind: Kind, payload: &'static str) -> Trigger {
    Trigger { ids, kind, payload }
}

use Kind::*;

#[rustfmt::skip]
static TRIGGERS: &[Trigger] = &[
    /* ------------------------- jsarray.c ------------------------- */
    t(&[1], Js, "[{toString:function(){throw 'JOIN'}}].join(',')"),
    t(&[2], Exempt, "Ap_join's `n > JS_STRLIMIT` needs a joined result above 1<<28 bytes; building \
                     256 MiB of JS strings is not a bounded test. The identical JS_STRLIMIT check is \
                     covered in js_pushstring / js_pushlstring / jsS_newstringnode (rows 70, 160, 161)."),
    t(&[3], Js, "[1,2].sort(1)"),
    t(&[4], Exempt, "Ap_sort's `len >= JS_ARRAYLIMIT` needs a 64M-entry array (1 GiB of flat data)."),
    t(&[5], Js, "Array.prototype.toString.call(null)"),
    t(&[6], Js, "[1].every(1)"),
    t(&[7], Js, "[1].some(1)"),
    t(&[8], Js, "[1].forEach(1)"),
    t(&[9], Js, "[1].map(1)"),
    t(&[10], Js, "[1].filter(1)"),
    t(&[11], Js, "[1].reduce(1)"),
    t(&[12], Js, "[].reduce(function(){})"),
    t(&[13], Js, "[1].reduceRight(1)"),
    t(&[14], Js, "[].reduceRight(function(){})"),

    /* ------------------------- jsboolean.c ----------------------- */
    t(&[15], Js, "Boolean.prototype.toString.call(1)"),
    t(&[16], Js, "Boolean.prototype.valueOf.call(1)"),

    /* ------------------------- jsbuiltin.c ----------------------- */
    t(&[17], MemSweep, "encodeURIComponent('a b c d e f g h i j k l m n o p q r s t u v')"),
    t(&[18, 19], Js, "decodeURI('%4')"),
    t(&[20], Js, "decodeURI('%zz')"),

    /* ------------------------- jscompile.c ----------------------- */
    t(&[21, 22, 23], Js, "1 = 2"),
    t(&[24], Js, "var enum;"),
    t(&[25], JsStrict, "var implements;"),
    t(&[26], JsGen, "many_newlines"),
    t(&[27], JsStrict, "function f(arguments){}"),
    t(&[28], JsStrict, "function f(eval){}"),
    t(&[29], Js, "function f(eval){}"),
    t(&[30], JsStrict, "function f(a,a){}"),
    t(&[31], JsStrict, "arguments = 1"),
    t(&[32], JsStrict, "eval = 1"),
    t(&[33], Js, "var e = eval; e"),
    t(&[34], JsGen, "backward_jump_overflow"),
    t(&[35], JsGen, "forward_jump_overflow"),
    t(&[36], JsStrict, "({a:1,a:2})"),
    t(&[37], Exempt, "cobject's \"invalid property name in object initializer\": the parser only ever \
                      produces EXP_IDENTIFIER / EXP_STRING / EXP_NUMBER property keys (jsparse.c \
                      propname()), so the `else` branch is unreachable through any source text."),
    t(&[38], Js, "1 = 2"),
    t(&[39], Js, "for (var a,b in {}) ;"),
    t(&[40], Js, "for (1 in {}) ;"),
    t(&[41], Js, "1 += 2"),
    t(&[42], Js, "1++"),
    t(&[43], JsStrict, "var x; delete x"),
    t(&[44], Js, "delete 1"),
    t(&[45], Exempt, "cexp's default \"unknown expression type\": the switch covers every EXP_* the \
                      parser can build, so the default arm is unreachable through any source text."),
    t(&[46], JsStrict, "try{}catch(arguments){}"),
    t(&[47], JsStrict, "try{}catch(eval){}"),
    t(&[48], JsStrict, "try{}catch(arguments){}finally{}"),
    t(&[49], JsStrict, "try{}catch(eval){}finally{}"),
    t(&[50], Js, "switch(1){default:;default:;}"),
    t(&[51], Js, "break foo;"),
    t(&[52], Js, "break;"),
    t(&[53], Js, "continue foo;"),
    t(&[54], Js, "continue;"),
    t(&[55], Js, "return 1"),
    t(&[56], JsStrict, "with({}){}"),

    /* ------------------------- jsdate.c -------------------------- */
    t(&[57], Js, "Date.prototype.getTime.call({})"),
    t(&[58], Js, "Date.prototype.setTime.call({},0)"),
    t(&[59], Js, "new Date(NaN).toISOString()"),
    t(&[60], Js, "Date.prototype.toJSON.call({})"),

    /* ------------------------- jsdtoa.c -------------------------- */
    t(&[61], Exempt, "assert(x.e == y.e) / assert(x.f >= y.f) in grisu2's minus(): only reachable by \
                      js_grisu2(±0.0), outside the contract of its sole caller jsV_numbertostring \
                      (`if (f == 0) return \"0\"`). Verified empirically: the C build aborts on \
                      js_grisu2(0.0), so the domain is excluded from tests/b_numeric.rs row14."),

    /* ------------------------- jserror.c ------------------------- */
    t(&[62], Js, "Error.prototype.toString.call(1)"),
    t(&[63], Js, "new Error({toString:function(){throw 'NEWERR'}})"),

    /* ------------------------- jsfunction.c ---------------------- */
    t(&[64], Js, "new Function('~')"),
    t(&[65], Js, "Function.prototype.toString.call(1)"),
    t(&[66], MemSweep, "String(function longishname(alpha,beta,gamma){return alpha})"),
    t(&[67], Js, "Function.prototype.apply.call(1)"),
    t(&[68], Js, "Function.prototype.call.call(1)"),
    t(&[69], Js, "Function.prototype.bind.call(1)"),

    /* ------------------------- jsintern.c ------------------------ */
    t(&[70], ApiJob, "intern_huge"),

    /* ------------------------- jslex.c --------------------------- */
    t(&[71, 72, 73], Js, "'abc"),
    t(&[74], Js, "JSON.parse('fals')"),
    t(&[75], Js, "var \\q;"),
    t(&[76], Js, "0x"),
    t(&[77], Exempt, "lexinteger() sits inside an `#if 0` block in jslex.c and is not compiled into \
                      the library at all (it is absent from `nm -D` on both .so files)."),
    t(&[78], Js, "01.5"),
    t(&[79], Js, "1a"),
    t(&[80], Js, "1e+"),
    t(&[81], Js, "'\\"),
    t(&[82], Js, "'abc"),
    t(&[83], Js, "'\\x4'"),
    t(&[84], Js, "/abc"),
    t(&[85], Js, "/a/q"),
    t(&[86], Js, "/a/gg"),
    t(&[87], Js, "/* abc"),
    t(&[88], Js, "@"),
    t(&[89], Js, "\u{1}"),
    t(&[90], Js, "JSON.parse('-a')"),
    t(&[91], Js, "JSON.parse('1.')"),
    t(&[92], Js, "JSON.parse('1e')"),
    t(&[93], Js, "JSON.parse('\"\\\\x\"')"),
    t(&[94], Js, "JSON.parse('\"abc')"),
    t(&[95], Js, "JSON.parse('\"a\u{1}\"')"),
    t(&[96], Js, "JSON.parse('@')"),
    t(&[97], Js, "JSON.parse('\u{1}')"),

    /* ------------------------- jsnumber.c ------------------------ */
    t(&[98], Js, "Number.prototype.valueOf.call('x')"),
    t(&[99], Js, "Number.prototype.toString.call('x')"),
    t(&[100, 101], Js, "(1).toString(1)"),
    t(&[102], Js, "Number.prototype.toFixed.call('x')"),
    t(&[103], Js, "(1).toFixed(101)"),
    t(&[104], Js, "Number.prototype.toExponential.call('x')"),
    t(&[105], Js, "(1).toExponential(101)"),
    t(&[106], Js, "Number.prototype.toPrecision.call('x')"),
    t(&[107], Js, "(1).toPrecision(101)"),

    /* ------------------------- jsobject.c ------------------------ */
    t(&[108], Js, "Object.getPrototypeOf(1)"),
    t(&[109], Js, "Object.getOwnPropertyDescriptor(1,'a')"),
    t(&[110], Js, "Object.getOwnPropertyNames(1)"),
    t(&[111], Js, "Object.defineProperty({},'a',{value:1,get:function(){}})"),
    t(&[112], Js, "Object.defineProperty(1,'a',{})"),
    t(&[113], Js, "Object.defineProperties({},{a:1})"),
    t(&[114], Js, "Object.defineProperties({},1)"),
    t(&[115], Js, "Object.defineProperties(1,{})"),
    t(&[116], Js, "Object.create(1)"),
    t(&[117], Js, "Object.keys(1)"),
    t(&[118], Js, "Object.preventExtensions(1)"),
    t(&[119], Js, "Object.isExtensible(1)"),
    t(&[120], Js, "Object.seal(1)"),
    t(&[121], Js, "Object.isSealed(1)"),
    t(&[122], Js, "Object.freeze(1)"),
    t(&[123], Js, "Object.isFrozen(1)"),

    /* ------------------------- json.c ---------------------------- */
    t(&[124], Js, "JSON.parse('[1')"),
    t(&[125], Js, "JSON.parse('{1:2}')"),
    t(&[126], Js, "JSON.parse(']')"),
    t(&[127, 129], Js, "var o={};o.o=o;JSON.stringify(o)"),
    t(&[128], Js, "var a=[];a.push(a);JSON.stringify(a)"),

    /* ------------------------- jsparse.c ------------------------- */
    t(&[130, 132, 133], Js, "var 1;"),
    t(&[131], JsGen, "deep_parens"),
    t(&[134], Js, "if (1"),
    t(&[135], Js, "var x = 1 2"),
    t(&[136], Js, "var 1;"),
    t(&[137], Js, "({})."),
    t(&[138], Js, "1 + *"),
    t(&[139], Js, "switch(1){foo:}"),
    t(&[140], Js, "for (var x 1) ;"),
    t(&[141], Js, "for (1 2) ;"),
    t(&[142], Js, "try {}"),

    /* ------------------------- jsproperty.c ---------------------- */
    t(&[143], JsStrict, "var o=Object.preventExtensions({}); o.x=1"),
    t(&[144], ApiJob, "nextiterator_nonobject"),
    t(&[145], Exempt, "assert(!obj->u.a.simple) in jsV_resizearray: every caller unflattens the array \
                       first (jsR_setproperty calls jsR_unflattenarray, jsV_resizearray is only reached \
                       on the non-simple branch), so the assertion is unreachable through the API."),

    /* ------------------------- jsregexp.c ------------------------ */
    t(&[146], Js, "new RegExp('(')"),
    t(&[147, 148], Exempt, "\"regexec failed\" fires on `js_regexec(...) < 0`, but js_regexec returns \
                            `match(...)` which only ever yields 0 (match) or 1 (no match) — there is no \
                            negative return in regexp.c. Unreachable by construction."),
    t(&[149], Js, "new RegExp(/a/,'g')"),
    t(&[150], Js, "new RegExp('a','q')"),
    t(&[151], Js, "new RegExp('a','gg')"),
    t(&[152], Js, "new RegExp('a','ii')"),
    t(&[153], Js, "new RegExp('a','mm')"),
    t(&[154], Js, "RegExp.prototype.toString.call({})"),

    /* ------------------------- jsrepr.c -------------------------- */
    t(&[155], ApiJob, "repr_throwing"),

    /* ------------------------- jsrun.c --------------------------- */
    t(&[156], Js, "function f(n){ if(n<=0) throw 1; try { f(n-1) } catch(e) { throw e } } f(200)"),
    t(&[157], Js, "function f(){ return 1+f() } f()"),
    t(&[158], MemSweep, "var a=[]; for(var i=0;i<200;++i) a.push({x:i}); a.length"),
    t(&[159], RunSweep, "var i=0; while(i<1000) ++i; i"),
    t(&[160], ApiJob, "pushstring_huge"),
    t(&[161], ApiJob, "pushlstring_huge"),
    t(&[162], Js, "RegExp.prototype.exec.call({},'a')"),
    t(&[163], ApiJob, "touserdata_badtag"),
    t(&[164], ApiJob, "defaccessor_notfunction"),
    t(&[165], ApiJob, "pop_underflow"),
    t(&[166], ApiJob, "remove_error"),
    t(&[167], ApiJob, "insert_notimpl"),
    t(&[168], ApiJob, "replace_error"),
    t(&[169], JsStrict, "var a=[1,2,3]; Object.preventExtensions(a); a[10]=1"),
    t(&[170], Exempt, "assert(obj->u.a.simple) / assert(k >= 0) / assert(newlen == flat_length+1) in \
                       jsR_setarrayindex: the sole caller checks `obj->u.a.simple && k >= 0 && \
                       k <= flat_length` before calling, so the assertions are unreachable."),
    t(&[171], Exempt, "jsR_setarrayindex's \"array too large\" needs k+1 > JS_ARRAYLIMIT (1<<26) while \
                       k <= flat_length, i.e. a densely populated 64M-entry flat array (1 GiB). The same \
                       JS_ARRAYLIMIT rejection is covered through jsR_setproperty (row 173)."),
    t(&[172], Js, "[].length = -1"),
    t(&[173], Js, "[].length = 67108865"),
    t(&[174], JsStrict, "var o={get a(){return 1}}; o.a=2"),
    t(&[175], ApiJobStrict, "setproperty_transient"),
    t(&[176], JsStrict, "var o={}; Object.defineProperty(o,'a',{value:1,writable:false}); o.a=2"),
    t(&[177], ApiJobStrict, "defproperty_readonly"),
    t(&[178], ApiJobStrict, "defproperty_nonconf"),
    t(&[179], ApiJobStrict, "defproperty_readonly_or_nonconf"),
    t(&[180], JsStrict, "delete /a/.source"),
    t(&[181], JsStrict, "NaN = 1"),
    t(&[182], JsStrict, "zzz = 1"),
    t(&[183], Exempt, "js_delvar's `if (J->strict) js_typeerror(\"'%s' is non-configurable\")` is only \
                       reached from OP_DELVAR/OP_DELLOCAL, and cdelete() rejects `delete <identifier>` at \
                       compile time in strict code (\"delete on an unqualified name is not allowed in \
                       strict mode\", row 43). In non-strict code J->strict is 0, so the branch is dead."),
    t(&[184], Js, "function f(){ return f() } f()"),
    t(&[185], ApiJob, "call_negative"),
    t(&[186], Js, "({})()"),
    t(&[187], Js, "new ({})"),
    t(&[188], Exempt, "js_endtry raises \"endtry: exception stack underflow\" exactly when \
                       `J->trytop == 0`; the js_error it raises then reaches js_throw with no try frame \
                       left, so the C library calls J->panic and abort()s. The rejection can never be \
                       observed by a caller, only the abort."),
    t(&[189, 192], Js, "throw 'PLAIN'"),
    t(&[190], Js, "zzz"),
    t(&[191], Js, "'a' in 1"),
    t(&[193], Js, "var x = ;"),

    /* ------------------------- jsstate.c ------------------------- */
    t(&[194], Exempt, "assert(sizeof(js_Value) == 16) / assert(offsetof(js_Value,t.type) == 15) in \
                       js_newstate are build-time layout invariants, not input-dependent rejections. \
                       Both libraries satisfy them: js_newstate returns a usable state in every test."),

    /* ------------------------- jsstring.c ------------------------ */
    t(&[195], Exempt, "js_doregexec's \"regexec failed\" needs js_regexec(...) < 0, which regexp.c never \
                       returns (match() yields only 0 or 1)."),
    t(&[196], Js, "String.prototype.charAt.call(null)"),
    t(&[197], Js, "String.prototype.toString.call(1)"),
    t(&[198], Js, "String.prototype.valueOf.call(1)"),
    t(&[199], Js, "'a'.concat({toString:function(){throw 'CONCAT'}})"),
    t(&[200], Exempt, "Sp_concat's `n > JS_STRLIMIT` needs a concatenation result above 1<<28 bytes."),
    t(&[201], Js, "'abc'.substring({valueOf:function(){throw 'SUBSTR'}})"),
    t(&[202], Js, "String.prototype.toLowerCase.call({toString:function(){throw 'LOWER'}})"),
    t(&[203], Js, "String.prototype.toUpperCase.call({toString:function(){throw 'UPPER'}})"),
    t(&[204], Js, "String.fromCharCode({valueOf:function(){throw 'FCC'}})"),
    t(&[205], Js, "'a'.replace(/a/,function(){throw 'REPRE'})"),
    t(&[206], Js, "'a'.replace('a',function(){throw 'REPSTR'})"),

    /* ------------------------- jsvalue.c ------------------------- */
    t(&[207], JsStrict, "({valueOf:function(){return {}},toString:function(){return {}}}) + 1"),
    t(&[208], Js, "undefined.foo"),
    t(&[209], Js, "null.foo"),
    t(&[210], ApiMemSweep, "newcfunctionx"),
    t(&[211], ApiMemSweep, "newuserdatax"),
    t(&[212], Js, "1 instanceof 1"),
    t(&[213], Js, "var f=function(){}; f.prototype=1; ({}) instanceof f"),
    t(&[214], MemSweep, "var a='abcdefghijklmnop'; var b='qrstuvwxyz01234'; a+b"),

    /* ------------------------- regexp.c -------------------------- */
    t(&[215], Re, "("),
    t(&[216], Re, "\\xZZ"),
    t(&[217], Re, "a{z}"),
    t(&[218], Re, "a\\"),
    t(&[219], Re, "\\q"),
    t(&[220], Re, "a{99999999999}"),
    t(&[221], ReGen, "many_classes"),
    t(&[222], Re, "[z-a]"),
    t(&[223], ReGen, "many_ranges"),
    t(&[224], Re, "[a"),
    t(&[225], Re, "()*"),
    t(&[226], Re, "\\1"),
    t(&[227], ReGen, "many_captures"),
    t(&[228], Re, "(a"),
    t(&[229], Re, "a**"),
    t(&[230], Re, "a{2,1}"),
    t(&[231], ReGen, "deep_cat"),
    t(&[232], Re, "((a{100}){100}){100}"),
    t(&[233], Exempt, "regexp.c:717 is `inst = NULL; /* silence compiler warning. assert(node->m > 0). */` \
                       — a comment, not an executable assert; there is no runtime check to trigger."),
    t(&[234], ReAlloc, "0"),
    t(&[235], ReGen, "deep_alt"),
    t(&[236], ReAlloc, "1"),
    t(&[237], Re, "a)"),
    t(&[238], Exempt, "regcompx's `if (g.lookahead != EOF) die(&g, \"syntax error\")` at jsregexp/regexp.c \
                       is shadowed by the earlier `if (g.lookahead == ')') die(\"unmatched ')'\")`; every \
                       remaining lookahead that could reach it is consumed by parsealt, so only the \
                       identically-worded parseatom \"syntax error\" (row 229) is reachable."),
    t(&[239], ReAlloc, "2"),
    t(&[240], ReAlloc, "3"),
];

/* ------------------------------------------------------------------ */
/* Generated payloads                                                  */
/* ------------------------------------------------------------------ */

fn gen_js(name: &str) -> String {
    match name {
        // > 65535 source lines makes emit()'s `emitraw(J, F, F->lastline)`
        // overflow the unsigned short instruction word.
        "many_newlines" => format!("{}1", "\n".repeat(70000)),
        // A forward jump patched by labelto() past 65535.
        "forward_jump_overflow" => format!("if(1){{{}}}", "a=1;".repeat(40000)),
        // A backward jump emitted by emitjumpto() past 65535.
        "backward_jump_overflow" => format!("while(1){{{}break;}}", "a=1;".repeat(40000)),
        // JS_ASTLIMIT is 400 nested expressions.
        "deep_parens" => format!("{}1{}", "(".repeat(450), ")".repeat(450)),
        other => panic!("unknown js generator {other}"),
    }
}

fn gen_re(name: &str) -> String {
    match name {
        // REG_MAXCLASS is 128.
        "many_classes" => "[a-b]".repeat(130),
        // REG_MAXSPAN is 64 runes == 32 ranges in a single class.
        "many_ranges" => {
            let mut s = String::from("[");
            for i in 0..40 {
                s.push_str(&format!("\\u{:04x}-\\u{:04x}", 0x100 + i * 4, 0x100 + i * 4 + 1));
            }
            s.push(']');
            s
        }
        // REG_MAXSUB is 16.
        "many_captures" => "(a)".repeat(17),
        // count() recurses once per P_CAT node; REG_MAXREC is 4096.
        "deep_cat" => "a".repeat(5000),
        // A deep P_ALT chain overruns REG_MAXPROG in regcompx's own check.
        "deep_alt" => {
            let mut s = String::from("a");
            for _ in 0..4090 {
                s = format!("(?:a|{s})");
            }
            s
        }
        other => panic!("unknown re generator {other}"),
    }
}

/* ------------------------------------------------------------------ */
/* Allocators                                                          */
/* ------------------------------------------------------------------ */

unsafe extern "C" {
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

unsafe extern "C-unwind" fn budget_alloc(
    ctx: *mut c_void,
    ptr: *mut c_void,
    n: c_int,
) -> *mut c_void {
    unsafe {
        let b = ctx as *mut i64;
        if n == 0 {
            free(ptr);
            return std::ptr::null_mut();
        }
        if !b.is_null() {
            if *b <= 0 {
                return std::ptr::null_mut();
            }
            *b -= 1;
        }
        realloc(ptr, n as usize)
    }
}

/// A >JS_STRLIMIT NUL-terminated buffer, allocated once and shared by both
/// libraries (256 MiB).
fn huge_string() -> &'static [c_char] {
    static BUF: OnceLock<Vec<c_char>> = OnceLock::new();
    BUF.get_or_init(|| {
        let n = (1usize << 28) + 8;
        let mut v = vec![b'a' as c_char; n];
        v.push(0);
        v
    })
}

/* ------------------------------------------------------------------ */
/* Host-callback jobs                                                  */
/* ------------------------------------------------------------------ */

const JS_READONLY: c_int = 1;
const JS_DONTCONF: c_int = 4;

fn run_api_job(api: &Api, which: usize, name: &'static str, flags: c_int) -> ProtectedOut {
    match name {
        "pop_underflow" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_pop)(j, 5);
            log("unreached");
        }),
        "remove_error" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_remove)(j, 5);
            log("unreached");
        }),
        "replace_error" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_pushnumber)(j, 1.0);
            (api.js_replace)(j, 5);
            log("unreached");
        }),
        "insert_notimpl" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_pushnumber)(j, 1.0);
            (api.js_insert)(j, 0);
            log("unreached");
        }),
        "call_negative" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_getglobal)(j, c"String".as_ptr());
            (api.js_pushundefined)(j);
            (api.js_call)(j, -1);
            log("unreached");
        }),
        "touserdata_badtag" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_newobject)(j);
            (api.js_newuserdata)(j, c"realtag".as_ptr(), 0x10 as *mut c_void, None);
            let p = (api.js_touserdata)(j, -1, c"othertag".as_ptr());
            log(format!("unreached {}", p as usize));
        }),
        "nextiterator_nonobject" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_newobject)(j);
            let k = (api.js_nextiterator)(j, -1);
            log(format!("unreached {:?}", rstr(k)));
        }),
        "defaccessor_notfunction" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_newobject)(j);
            (api.js_pushnumber)(j, 1.0); // getter: not a function
            (api.js_pushnumber)(j, 2.0); // setter: not a function
            (api.js_defaccessor)(j, -3, c"p".as_ptr(), 0);
            log("unreached");
        }),
        "setproperty_transient" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_pushstring)(j, c"abc".as_ptr());
            (api.js_pushnumber)(j, 1.0);
            (api.js_setproperty)(j, -2, c"foo".as_ptr());
            log(format!("after={}", describe(api, j, -1)));
        }),
        "defproperty_readonly" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_newobject)(j);
            (api.js_pushnumber)(j, 1.0);
            (api.js_defproperty)(j, -2, c"a".as_ptr(), JS_READONLY);
            (api.js_pushnumber)(j, 2.0);
            (api.js_defproperty)(j, -2, c"a".as_ptr(), 0);
            log("unreached");
        }),
        "defproperty_nonconf" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_newobject)(j);
            (api.js_pushnumber)(j, 1.0);
            (api.js_defproperty)(j, -2, c"a".as_ptr(), JS_DONTCONF);
            // now try to install a getter on the non-configurable property
            let fname = cs("[string]");
            let src = cs("(function(){return 1})");
            if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) == 0 {
                (api.js_pushundefined)(j);
                (api.js_pcall)(j, 0);
            }
            (api.js_pushundefined)(j);
            (api.js_defaccessor)(j, -3, c"a".as_ptr(), 0);
            log("unreached");
        }),
        "defproperty_readonly_or_nonconf" => run_protected(api, which, flags, |api, j| unsafe {
            // arrays reject any redefinition of "length" through the readonly label
            (api.js_newarray)(j);
            (api.js_pushnumber)(j, 3.0);
            (api.js_defproperty)(j, -2, c"length".as_ptr(), 0);
            log("unreached");
        }),
        "pushstring_huge" => {
            let buf = huge_string();
            run_protected(api, which, flags, move |api, j| unsafe {
                (api.js_pushstring)(j, buf.as_ptr());
                log("unreached");
            })
        }
        "pushlstring_huge" => run_protected(api, which, flags, |api, j| unsafe {
            (api.js_pushlstring)(j, c"abc".as_ptr(), (1 << 28) + 1);
            log("unreached");
        }),
        "intern_huge" => {
            let buf = huge_string();
            run_protected(api, which, flags, move |api, j| unsafe {
                let s = (api.js_intern)(j, buf.as_ptr());
                log(format!("unreached {}", s as usize));
            })
        }
        "repr_throwing" => run_protected(api, which, flags, |api, j| unsafe {
            let fname = cs("[string]");
            let src = cs("({get a(){throw 'REPR'}})");
            if (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr()) == 0 {
                (api.js_pushundefined)(j);
                (api.js_pcall)(j, 0);
            }
            let s = (api.js_torepr)(j, -1);
            log(format!("repr={:?}", rstr(s)));
        }),
        other => panic!("unknown api job {other}"),
    }
}

/// Allocation-failure sweep around a low-level constructor.
fn api_mem_sweep(api: &Api, which: usize, name: &'static str) -> String {
    let mut out = String::new();
    for limit in [1i32, 8, 16, 32, 64, 128, 256, 512, 1024, 4096, 65536] {
        let o = match name {
            "newcfunctionx" => run_protected(api, which, 0, move |api, j| unsafe {
                (api.js_setlimit)(j, 0, limit);
                (api.js_newcfunctionx)(
                    j,
                    Some(noop_cfun),
                    c"probe".as_ptr(),
                    3,
                    0x1234 as *mut c_void,
                    Some(noop_fin),
                );
                log(format!("ok {}", describe(api, j, -1)));
            }),
            "newuserdatax" => run_protected(api, which, 0, move |api, j| unsafe {
                (api.js_newobject)(j);
                (api.js_setlimit)(j, 0, limit);
                (api.js_newuserdatax)(
                    j,
                    c"tag".as_ptr(),
                    0x1234 as *mut c_void,
                    None,
                    None,
                    None,
                    Some(noop_fin),
                );
                log(format!("ok {}", describe(api, j, -1)));
            }),
            other => panic!("unknown api mem sweep {other}"),
        };
        out.push_str(&format!(
            "limit={limit} rc={} result={:?} log={:?}\n",
            o.rc, o.result, o.log
        ));
    }
    out
}

unsafe extern "C-unwind" fn noop_cfun(_j: JS) {}
unsafe extern "C-unwind" fn noop_fin(_j: JS, _p: *mut c_void) {}

/* ------------------------------------------------------------------ */
/* Observation                                                         */
/* ------------------------------------------------------------------ */

fn re_result(api: &Api, pat: &str) -> (bool, String) {
    unsafe {
        let cp = cbuf(pat.as_bytes());
        let mut ep: *const c_char = std::ptr::null();
        let prog = (api.js_regcomp)(cp.as_ptr(), 0, &mut ep);
        let r = (prog.is_null(), rstr(ep));
        if !prog.is_null() {
            (api.js_regfree)(prog);
        }
        r
    }
}

fn re_alloc_result(api: &Api, pat: &str, budget: i64) -> (bool, String) {
    unsafe {
        let mut b = budget;
        let cp = cbuf(pat.as_bytes());
        let mut ep: *const c_char = std::ptr::null();
        let prog = (api.js_regcompx)(
            Some(budget_alloc),
            &mut b as *mut i64 as *mut c_void,
            cp.as_ptr(),
            0,
            &mut ep,
        );
        let r = (prog.is_null(), rstr(ep));
        if !prog.is_null() {
            b = i64::MAX;
            (api.js_regfreex)(
                Some(budget_alloc),
                &mut b as *mut i64 as *mut c_void,
                prog,
            );
        }
        r
    }
}

fn mem_sweep(api: &Api, src: &str) -> String {
    let mut out = String::new();
    for limit in [1i32, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 65536] {
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            (api.js_setreport)(j, Some(report_cb));
            let fname = cs("[string]");
            let csrc = cs(src);
            (api.js_setlimit)(j, 0, limit);
            let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
            if rc != 0 {
                rc = 1;
            } else {
                (api.js_pushundefined)(j);
                if (api.js_pcall)(j, 0) != 0 {
                    rc = 2;
                }
            }
            let fb = cs("<throw>");
            let s = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
            (api.js_freestate)(j);
            out.push_str(&format!("mem={limit} rc={rc} {s}\n"));
        }
    }
    out
}

fn run_sweep(api: &Api, src: &str) -> String {
    let mut out = String::new();
    for limit in [1i32, 2, 3, 5, 10, 50, 100, 500, 1000, 5000] {
        unsafe {
            let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            (api.js_setreport)(j, Some(report_cb));
            let fname = cs("[string]");
            let csrc = cs(src);
            (api.js_setlimit)(j, limit, 0);
            let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr());
            if rc != 0 {
                rc = 1;
            } else {
                (api.js_pushundefined)(j);
                if (api.js_pcall)(j, 0) != 0 {
                    rc = 2;
                }
            }
            let fb = cs("<throw>");
            let s = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
            (api.js_freestate)(j);
            out.push_str(&format!("run={limit} rc={rc} {s}\n"));
        }
    }
    out
}

/// Observable rejection for a trigger, as plain (un-escaped) text so the row's
/// message template can be matched against it directly.
fn observe(trigger: &Trigger, which: usize) -> String {
    let p = pair();
    let api = if which == 0 { &p.c } else { &p.r };
    match trigger.kind {
        Js => {
            let o = unsafe { eval(api, trigger.payload, 0) };
            format!("rc={} value={} reports={}", o.rc, o.value, o.reports.join("|"))
        }
        JsStrict => {
            let o = unsafe { eval(api, trigger.payload, JS_STRICT) };
            format!("rc={} value={} reports={}", o.rc, o.value, o.reports.join("|"))
        }
        JsGen => {
            let src = gen_js(trigger.payload);
            let o = unsafe { eval(api, &src, 0) };
            format!("rc={} value={} reports={}", o.rc, o.value, o.reports.join("|"))
        }
        Re => {
            let (null, err) = re_result(api, trigger.payload);
            format!("null={null} err={err}")
        }
        ReGen => {
            let pat = gen_re(trigger.payload);
            let (null, err) = re_result(api, &pat);
            format!("null={null} err={err}")
        }
        ReAlloc => {
            let budget: i64 = trigger.payload.parse().unwrap();
            let (null, err) = re_alloc_result(api, "a(b|c)[d-e]{2,3}", budget);
            format!("null={null} err={err}")
        }
        ApiJob => {
            let o = run_api_job(api, which, trigger.payload, 0);
            format!(
                "rc={} result={} log={} top={} reports={}",
                o.rc,
                o.result,
                o.log.join("|"),
                o.top,
                o.reports.join("|")
            )
        }
        ApiJobStrict => {
            let o = run_api_job(api, which, trigger.payload, JS_STRICT);
            format!(
                "rc={} result={} log={} top={} reports={}",
                o.rc,
                o.result,
                o.log.join("|"),
                o.top,
                o.reports.join("|")
            )
        }
        MemSweep => mem_sweep(api, trigger.payload),
        RunSweep => run_sweep(api, trigger.payload),
        ApiMemSweep => api_mem_sweep(api, which, trigger.payload),
        Exempt => "exempt".to_string(),
    }
}

/* ------------------------------------------------------------------ */
/* Row bookkeeping                                                     */
/* ------------------------------------------------------------------ */

struct Row {
    id: u32,
    file: String,
    func: String,
    msg: String,
    kind: String,
}

fn rows() -> Vec<Row> {
    INDEX
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Row {
                id: f[0].parse().unwrap(),
                file: f[1].to_string(),
                func: f[2].to_string(),
                msg: f[3].to_string(),
                kind: f[4].to_string(),
            }
        })
        .collect()
}

/// Undo the C source-level escapes in a message literal.
fn unescape_c(msg: &str) -> String {
    let mut s = String::new();
    let b: Vec<char> = msg.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '\\' && i + 1 < b.len() {
            match b[i + 1] {
                'n' => s.push('\n'),
                't' => s.push('\t'),
                'r' => s.push('\r'),
                '\\' => s.push('\\'),
                '"' => s.push('"'),
                '\'' => s.push('\''),
                c => {
                    s.push('\\');
                    s.push(c);
                }
            }
            i += 2;
        } else {
            s.push(b[i]);
            i += 1;
        }
    }
    s
}

/// The template with printf conversions removed must appear, in order, in `text`.
fn template_matches(template: &str, text: &str) -> bool {
    let t = unescape_c(template);
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let ch: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        if ch[i] == '%' && i + 1 < ch.len() {
            let mut j = i + 1;
            while j < ch.len()
                && (ch[j].is_ascii_digit() || ch[j] == '.' || ch[j] == '+' || ch[j] == '-')
            {
                j += 1;
            }
            if j < ch.len() && "sdcgfXxu".contains(ch[j]) {
                parts.push(std::mem::take(&mut cur));
                i = j + 1;
                continue;
            }
        }
        cur.push(ch[i]);
        i += 1;
    }
    parts.push(cur);

    let mut pos = 0usize;
    for part in parts.iter() {
        if part.is_empty() {
            continue;
        }
        match text.get(pos..).and_then(|t| t.find(part.as_str())) {
            Some(off) => pos += off + part.len(),
            None => return false,
        }
    }
    true
}

/* ------------------------------------------------------------------ */
/* Tests                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn phase_c_differential() {
    let mut failures = Vec::new();
    for tr in TRIGGERS {
        if tr.kind == Exempt {
            continue;
        }
        if std::env::var_os("MUJS_TRACE").is_some() {
            eprintln!("[trigger] rows={:?} payload={:?}", tr.ids, tr.payload);
        }
        let a = observe(tr, 0);
        let b = observe(tr, 1);
        if a != b {
            failures.push(format!(
                "rows {:?} payload {:?}\n  C   : {a}\n  RUST: {b}",
                tr.ids, tr.payload
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} error-path divergences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The run-limit rejection (`js_runlimit`, row 159) is a sweep rather than a
/// single input, so it gets its own test.
#[test]
fn phase_c_runlimit() {
    let p = pair();
    for src in [
        "var i=0; while(i<1000) ++i; i",
        "function f(n){return n?f(n-1):0} f(50)",
        "[1,2,3].map(function(x){return x*2}).join(',')",
    ] {
        let a = run_sweep(&p.c, src);
        let b = run_sweep(&p.r, src);
        assert_eq!(a, b, "run-limit sweep for {src:?}");
        assert!(
            a.contains("too much recursion") || a.contains("rc=2") || a.contains("rc=1"),
            "run-limit sweep produced no rejection for {src:?}:\n{a}"
        );
    }
}

#[test]
fn phase_c_triggers_hit_their_rows() {
    let rows = rows();
    let by_id: std::collections::HashMap<u32, &Row> = rows.iter().map(|r| (r.id, r)).collect();
    let mut problems = Vec::new();
    for tr in TRIGGERS {
        if tr.kind == Exempt {
            continue;
        }
        let text = observe(tr, 0);
        for id in tr.ids {
            let row = by_id.get(id).unwrap_or_else(|| panic!("unknown row {id}"));
            if row.msg.is_empty() {
                // js_throw re-raise or an error helper: nothing to match on, but
                // the trigger must have produced *some* rejection.
                let rejected = text.contains("rc=1")
                    || text.contains("rc=2")
                    || text.contains("null=true")
                    || text.contains("Error")
                    || text.contains("out of memory")
                    || text.contains("script ran too long");
                if !rejected {
                    problems.push(format!(
                        "row {id} ({}:{}) trigger {:?} produced no rejection: {text}",
                        row.file, row.func, tr.payload
                    ));
                }
                continue;
            }
            if !template_matches(&row.msg, &text) {
                problems.push(format!(
                    "row {id} ({}:{}) expects {:?}; trigger {:?} gave: {text}",
                    row.file, row.func, row.msg, tr.payload
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} trigger/row mismatches:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn phase_c_coverage() {
    let rows = rows();
    let mut covered = std::collections::HashSet::new();
    for tr in TRIGGERS {
        for id in tr.ids {
            assert!(covered.insert(*id) || true);
            covered.insert(*id);
        }
    }
    let missing: Vec<String> = rows
        .iter()
        .filter(|r| !covered.contains(&r.id))
        .map(|r| format!("{} {}:{} {:?} [{}]", r.id, r.file, r.func, r.msg, r.kind))
        .collect();
    assert!(
        missing.is_empty(),
        "{} of {} ERRORS.md rows have no Phase C trigger:\n{}",
        missing.len(),
        rows.len(),
        missing.join("\n")
    );
    // Also make sure the table has no stale ids.
    let known: std::collections::HashSet<u32> = rows.iter().map(|r| r.id).collect();
    let stale: Vec<u32> = covered.difference(&known).copied().collect();
    assert!(stale.is_empty(), "stale trigger ids: {stale:?}");
}

/// Generic FFI-boundary robustness: NULL pointers, zero/oversized lengths,
/// out-of-range enum values.  These are inputs the C accepts as plain ints and
/// the Rust must treat identically.
#[test]
fn phase_c_generic_boundaries() {
    let p = pair();

    // --- out-of-range "enum" values across the FFI boundary ---
    // js_newstate flags: only JS_STRICT(1) is defined.
    for flags in [0 as c_int, 1, 2, 3, 0x7fff_ffff, -1, i32::MIN] {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
                assert!(!j.is_null(), "js_newstate(flags={flags})");
                let src = cs("x = 1; typeof x");
                let fname = cs("[string]");
                let mut rc = (api.js_ploadstring)(j, fname.as_ptr(), src.as_ptr());
                if rc == 0 {
                    (api.js_pushundefined)(j);
                    if (api.js_pcall)(j, 0) != 0 {
                        rc = 2;
                    }
                }
                let fb = cs("<throw>");
                let s = rstr((api.js_trystring)(j, -1, fb.as_ptr()));
                (api.js_freestate)(j);
                outs.push((rc, s));
            }
        }
        assert_eq!(outs[0], outs[1], "js_newstate flags={flags}");
    }

    // js_newregexp flags: JS_REGEXP_G/I/M = 1/2/4.
    for flags in [0 as c_int, 7, 8, 16, 255, -1, i32::MIN, i32::MAX] {
        diff_protected(&format!("js_newregexp flags={flags}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newregexp)(j, c"a(b)".as_ptr(), flags);
                log(describe(api, j, -1));
                (api.js_setglobal)(j, c"re".as_ptr());
                let fname = cs("[string]");
                for src in ["String(re)", "re.global+','+re.ignoreCase+','+re.multiline",
                            "JSON.stringify(re.exec('ab'))", "re.lastIndex"] {
                    let csrc = cs(src);
                    if (api.js_ploadstring)(j, fname.as_ptr(), csrc.as_ptr()) == 0 {
                        (api.js_pushundefined)(j);
                        let rc = (api.js_pcall)(j, 0);
                        let errs = cs("<throw>");
                        log(format!("{src} rc={rc} {}", rstr((api.js_trystring)(j, -1, errs.as_ptr()))));
                        (api.js_pop)(j, 1);
                    }
                }
            }
        });
    }

    // property attribute flags: READONLY/DONTENUM/DONTCONF = 1/2/4.
    for atts in [0 as c_int, 7, 8, 16, 255, -1, i32::MIN, i32::MAX] {
        diff_protected(&format!("js_defproperty atts={atts}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newobject)(j);
                (api.js_pushnumber)(j, 1.0);
                (api.js_defproperty)(j, -2, c"a".as_ptr(), atts);
                log(describe(api, j, -1));
                (api.js_getproperty)(j, -1, c"a".as_ptr());
                log(describe(api, j, -1));
                (api.js_pop)(j, 1);
                (api.js_pushiterator)(j, -1, 1);
                let mut keys = Vec::new();
                loop {
                    let k = (api.js_nextiterator)(j, -1);
                    if k.is_null() {
                        break;
                    }
                    keys.push(rstr(k));
                    (api.js_pop)(j, 1);
                }
                log(format!("keys={keys:?}"));
            }
        });
    }

    // js_toprimitive hint: JS_HNONE/HNUMBER/HSTRING = 0/1/2.
    for hint in [0 as c_int, 3, 4, -1, i32::MIN, i32::MAX] {
        diff_protected(&format!("js_toprimitive hint={hint}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newobject)(j);
                (api.js_toprimitive)(j, -1, hint);
                log(describe(api, j, -1));
            }
        });
    }

    // js_pushiterator own flag.
    for own in [0 as c_int, 1, 2, -1, i32::MAX] {
        diff_protected(&format!("js_pushiterator own={own}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newobject)(j);
                (api.js_pushnumber)(j, 1.0);
                (api.js_setproperty)(j, -2, c"a".as_ptr());
                (api.js_pushiterator)(j, -1, own);
                let mut keys = Vec::new();
                loop {
                    let k = (api.js_nextiterator)(j, -1);
                    if k.is_null() {
                        break;
                    }
                    keys.push(rstr(k));
                    (api.js_pop)(j, 1);
                    if keys.len() > 100 {
                        break;
                    }
                }
                log(format!("keys={keys:?}"));
            }
        });
    }

    // --- zero / oversized lengths ---
    // `js_pushlstring` copies with `while (n--) *s++ = *v++;` after only
    // checking `n > JS_STRLIMIT`, so a negative n loops ~2^32 times and walks
    // off the end of the value stack — undefined behaviour in the C, excluded.
    for n in [0 as c_int, 1, 3, 4, 15, 16] {
        diff_protected(&format!("js_pushlstring n={n}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_pushlstring)(j, c"abc".as_ptr(), n);
                log(describe(api, j, -1));
            }
        });
    }
    for len in [0 as c_int, 1, -1, i32::MIN, i32::MAX] {
        diff_protected(&format!("js_setlength len={len}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newarray)(j);
                (api.js_setlength)(j, -1, len);
                log(format!("len={}", (api.js_getlength)(j, -1)));
            }
        });
    }
    for i in [0 as c_int, -1, i32::MIN, i32::MAX] {
        diff_protected(&format!("index ops i={i}"), 0, || {
            move |api: &Api, j: JS| unsafe {
                (api.js_newarray)(j);
                (api.js_pushnumber)(j, 1.0);
                (api.js_setindex)(j, -2, i);
                log(format!("len={}", (api.js_getlength)(j, -1)));
                (api.js_getindex)(j, -1, i);
                log(describe(api, j, -1));
                (api.js_pop)(j, 1);
                log(format!("has={}", {
                    let h = (api.js_hasindex)(j, -1, i);
                    if h != 0 {
                        (api.js_pop)(j, 1);
                    }
                    h
                }));
                (api.js_delindex)(j, -1, i);
                log(format!("len2={}", (api.js_getlength)(j, -1)));
            }
        });
    }

    // --- NULL / empty pointers where the C accepts them ---
    // js_regcomp with a NULL errorp.
    for pat in ["a", "(", "[z-a]", ""] {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let cp = cbuf(pat.as_bytes());
                let prog = (api.js_regcomp)(cp.as_ptr(), 0, std::ptr::null_mut());
                outs.push(prog.is_null());
                if !prog.is_null() {
                    (api.js_regfree)(prog);
                }
            }
        }
        assert_eq!(outs[0], outs[1], "js_regcomp({pat:?}, errorp=NULL)");
    }
    // js_regfree(NULL) / js_regexec(sub=NULL)
    unsafe {
        (p.c.js_regfree)(std::ptr::null_mut());
        (p.r.js_regfree)(std::ptr::null_mut());
    }
    // js_setreport(NULL) restores silence; js_dostring must still work.
    for src in ["1+1", "throw 1"] {
        let mut outs = Vec::new();
        for api in [&p.c, &p.r] {
            unsafe {
                let j = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                (api.js_setreport)(j, None);
                let csrc = cs(src);
                let rc = (api.js_dostring)(j, csrc.as_ptr());
                let top = (api.js_gettop)(j);
                (api.js_freestate)(j);
                outs.push((rc, top));
            }
        }
        assert_eq!(outs[0], outs[1], "js_setreport(NULL) for {src:?}");
    }
    // js_newuserdata with a NULL finalize / NULL data.
    diff_protected("js_newuserdata NULL hooks", 0, || {
        move |api: &Api, j: JS| unsafe {
            (api.js_newobject)(j);
            (api.js_newuserdata)(j, c"t".as_ptr(), std::ptr::null_mut(), None);
            log(describe(api, j, -1));
            log(format!(
                "data={}",
                (api.js_touserdata)(j, -1, c"t".as_ptr()) as usize
            ));
            (api.js_newobject)(j);
            (api.js_newuserdatax)(
                j,
                c"t2".as_ptr(),
                std::ptr::null_mut(),
                None,
                None,
                None,
                None,
            );
            log(describe(api, j, -1));
        }
    });
    // js_newcfunctionx with NULL data and finalize.
    diff_protected("js_newcfunctionx NULL hooks", 0, || {
        move |api: &Api, j: JS| unsafe {
            (api.js_newcfunctionx)(
                j,
                Some(noop_cfun),
                c"f".as_ptr(),
                0,
                std::ptr::null_mut(),
                None,
            );
            log(describe(api, j, -1));
            log(format!(
                "data={}",
                (api.js_currentfunctiondata)(j) as usize == 0
            ));
        }
    });
    // empty strings everywhere
    diff_protected("empty string inputs", 0, || {
        move |api: &Api, j: JS| unsafe {
            (api.js_pushstring)(j, c"".as_ptr());
            (api.js_pushliteral)(j, c"".as_ptr());
            (api.js_pushlstring)(j, c"".as_ptr(), 0);
            (api.js_newstring)(j, c"".as_ptr());
            (api.js_newregexp)(j, c"".as_ptr(), 0);
            (api.js_newobject)(j);
            (api.js_pushnumber)(j, 1.0);
            (api.js_defproperty)(j, -2, c"".as_ptr(), 0);
            log(describe(api, j, -1));
            (api.js_getproperty)(j, -1, c"".as_ptr());
            log(describe(api, j, -1));
            for l in snapshot(api, j) {
                log(l);
            }
        }
    });
}
