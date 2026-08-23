//! Differential tests for object creation, the property API with its attribute
//! cross-product, the ARRAY REPRESENTATION axis and the iterators.
//! Covers CONFIGS.md rows 71-141.
//!
//! Everything goes through the two `.so` exports via `tests/common/mod.rs`.
//! Every entry point that can throw is driven from inside a cfunction invoked
//! with `js_pcall` (the `cf_op` / `cf_csnap` probe pattern), so a `js_typeerror`
//! is always caught instead of reaching `js_throw` with `trytop == 0`.
//!
//! Paths deliberately NOT driven, with the reason:
//!
//! * `jsrun.c:551-552` -- the `js_try` handler inside `jsR_unflattenarray`
//!   assigns `obj->properties = NULL` and rethrows.  A NULL property tree is
//!   then dereferenced unconditionally by `jsproperty.c:48` (`lookup()`s
//!   `node != &sentinel` test followed by `strcmp(name, node->name)`), by
//!   `jsgc.c:101` (`jsG_scanobject`s `obj->properties->level`) and by
//!   `jsgc.c:35` (`jsG_freeobject`s `obj->properties->level`).  Reaching that
//!   handler therefore poisons the state so that *any* later property access,
//!   `js_gc` or `js_freestate` performs a NULL dereference -- undefined
//!   behaviour, and unobservable in-process.  Only the first half of row 132
//!   (a reachable simple array scanned by `jsG_scanobject` and freed by
//!   `jsG_freeobject`) is driven, in `t_array_gc_simple`.
//! * `jsrun.c:673-678` -- `jsR_setarrayindex` has three live `assert()`s (the C
//!   is built without `-DNDEBUG`, see `c_src/CMakeLists.txt`).  Every caller
//!   (`jsR_setproperty` at jsrun.c:722 and `jsR_setindex` at jsrun.c:806) first
//!   checks `u.a.simple && k >= 0 && k <= flat_length`, so the asserts cannot be
//!   violated through the public API and no test tries to.
//! * `jsvalue.c:539` -- `js_newuserdatax` pops the prototype *before* installing
//!   its `js_try` handler, so the finalizer-on-construction-failure path needs a
//!   failing allocator; that is already covered by `api_state.rs`
//!   (`t_finalizer_on_construction_failure`) and is not repeated here.
//!
//! Two spots where the C's behaviour is *unspecified* by the C standard but is
//! fixed by the reference build, and where the Rust had to be made to match
//! (verified against `objdump -d c_src/build/libmujs.so`):
//!
//! * `jsrun.c:1011` / `jsrun.c:1049` -- `js_setproperty` / `js_setindex` pass
//!   both `js_toobject(J, idx)` and `!js_isobject(J, idx)` in one argument list,
//!   and `js_toobject` REWRITES the stack slot to the wrapper object.  gcc
//!   x86-64 evaluates the list right-to-left, so `js_isobject` runs first and a
//!   primitive receiver really is `transient`.  Driven by `t_transient_receiver`.
//! * `jsrun.c:1028` -- `js_defaccessor` resolves the *setter* (`-1`) first, then
//!   the getter (`-2`), then `js_toobject(J, idx)`; that decides which of the
//!   three possible typeerrors fires.  Driven by `t_defaccessor_matrix`.
//!
//! Two bounded-work concessions in the snapshot helpers, both symmetric between
//! the two libraries because they are keyed on the (identical) `js_getlength`:
//! `jsrepr.c:125` `reprarray` and `json.c:300` `JSON.stringify` both loop
//! `length` times, so an array whose `u.a.length` was pushed into the millions
//! would make every snapshot quadratic; above 200 the snapshot prints
//! `repr=skip` / `json=skip` instead.
//!
//! Set `MUJS_DUMP=1` to print every transcript (useful when adding cases).

mod common;
use common::*;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void, CString};

/* ----------------------------------------------------------- name literals */

macro_rules! cn {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const N_T: *const c_char = cn!("T");
const N_OP: *const c_char = cn!("op");
const N_TAG: *const c_char = cn!("udtag");
const N_TAG2: *const c_char = cn!("othertag");
const PAYLOAD: *const c_char = cn!("PAY");
const N_JSNAP: *const c_char = cn!("jsnap");
const N_PROBE: *const c_char = cn!("probe");
const N_AMUT: *const c_char = cn!("amut");

/// Every `js_is*` predicate that takes (J, idx).
const PREDS: &[&str] = &[
    "js_isdefined",
    "js_isundefined",
    "js_isnull",
    "js_isboolean",
    "js_isnumber",
    "js_isstring",
    "js_isprimitive",
    "js_isobject",
    "js_isarray",
    "js_isregexp",
    "js_iscoercible",
    "js_iscallable",
    "js_iserror",
    "js_isnumberobject",
    "js_isstringobject",
    "js_isbooleanobject",
    "js_isdateobject",
];

/* ------------------------------------------------------------- diff driver */

/// Run `f` against the C library and then the Rust library with a fresh output
/// buffer each time, and assert the two transcripts are byte-identical.
fn diff2(tag: &str, f: impl Fn(&Lib) -> String) -> String {
    let p = libs();
    let a = {
        out_clear();
        set_cur(&p.c);
        let r = f(&p.c);
        format!("{r}\n--out--\n{}", out_take())
    };
    let b = {
        out_clear();
        set_cur(&p.rs);
        let r = f(&p.rs);
        format!("{r}\n--out--\n{}", out_take())
    };
    if std::env::var_os("MUJS_DUMP").is_some() {
        println!("=== [{tag}] ===\n{a}");
    }
    if a != b {
        // find the first differing byte so the message stays readable
        let (ab, bb) = (a.as_bytes(), b.as_bytes());
        let mut i = 0;
        while i < ab.len() && i < bb.len() && ab[i] == bb[i] {
            i += 1;
        }
        let lo = i.saturating_sub(160);
        panic!(
            "divergence in [{tag}] at byte {i}\n\
             ...C  : {:?}\n...RS : {:?}\n--- full C ---\n{a}\n--- full RS ---\n{b}",
            String::from_utf8_lossy(&ab[lo..(i + 200).min(ab.len())]),
            String::from_utf8_lossy(&bb[lo..(i + 200).min(bb.len())]),
        );
    }
    a
}

/* ---------------------------------------------------------------- helpers */

/// A property name is an arbitrary NUL-free byte string (the C only ever does
/// `strcmp` / `strlen` on it), so names are carried as raw bytes.
type NameV = Vec<u8>;

fn nb(s: &str) -> NameV {
    s.as_bytes().to_vec()
}

fn ncstr(n: &[u8]) -> CString {
    let mut v: Vec<u8> = n.iter().copied().filter(|b| *b != 0).collect();
    v.push(0);
    CString::from_vec_with_nul(v).unwrap()
}

/// Stable rendering of a name for the transcript (identical for both libraries
/// because it is computed from the same input bytes).
fn nshow(n: &[u8]) -> String {
    let mut s = String::new();
    for b in n {
        if *b >= 0x20 && *b < 0x7f && *b != b'\\' {
            s.push(*b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    s
}

/// Pop back down to `base` without ever asking `js_pop` for more than there is.
unsafe fn drain_to(l: &Lib, j: JS, base: c_int) {
    let t = l.js_gettop(j);
    if t > base {
        l.js_pop(j, t - base);
    }
}

/// `js_ploadstring` + `js_pcall` of an expression, leaving its value on the
/// stack (or the error).  Returns the composite return code.
unsafe fn push_expr(l: &Lib, j: JS, src: &str) -> c_int {
    let cs = cstr(src);
    let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
    if rc != 0 {
        return 100 + rc;
    }
    l.js_pushundefined(j);
    l.js_pcall(j, 0)
}

/// Everything observable about the value at `idx` through the non-property part
/// of the API.  `js_tryrepr` comes LAST because `js_torepr` replaces the slot.
unsafe fn describe(l: &Lib, j: JS, idx: c_int) -> String {
    let mut s = format!(
        "ty={} t={}",
        from_c(l.js_typeof(j, idx)),
        l.js_type(j, idx)
    );
    for p in PREDS {
        s.push_str(&format!(" {}", l.pred(p, j, idx)));
    }
    s.push_str(&format!(
        " ud={}/{}",
        l.js_isuserdata(j, idx, N_TAG),
        l.js_isuserdata(j, idx, N_TAG2)
    ));
    s.push_str(&format!(" bool={}", l.js_toboolean(j, idx)));
    s.push_str(&format!(" r={}", from_c(l.js_tryrepr(j, idx, ERRSTR))));
    s
}

/// `describe` the top of the stack WITHOUT consuming it: `js_torepr` replaces
/// the slot it is given (jsrepr.c:271), so describe a duplicate.
unsafe fn describe_top(l: &Lib, j: JS) -> String {
    l.nullary("js_dup", j);
    let s = describe(l, j, -1);
    l.js_pop(j, 1);
    s
}

/* ------------------------------------------------------------- C callbacks */

/// A plain cfunction target / accessor callee.
unsafe extern "C" fn cf_ret(j: JS) {
    let l = cur();
    l.js_pushstring(j, cn!("cf_ret"));
}

unsafe extern "C" fn cc_ctor(j: JS) {
    let l = cur();
    l.js_newobject(j);
    l.js_pushnumber(j, 1.0);
    l.js_setproperty(j, -2, cn!("built"));
}

unsafe extern "C" fn cf_getter(j: JS) {
    let l = cur();
    out_push(b"[getter]");
    l.js_pushstring(j, cn!("GOT"));
}

unsafe extern "C" fn cf_setter(j: JS) {
    let l = cur();
    out_push(
        format!(
            "[setter top={} v={}]",
            l.js_gettop(j),
            from_c(l.js_tryrepr(j, 1, ERRSTR))
        )
        .as_bytes(),
    );
    l.js_pushundefined(j);
}

unsafe extern "C" fn fin_cb(_j: JS, data: *mut c_void) {
    out_push(format!("[fin {}]", from_c(data as *const c_char)).as_bytes());
}

/* ------------------------------------------------------- userdata hooks */

thread_local! {
    /// Transcript of every userdata hook invocation, name + arguments.
    static HOOKLOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// 1 => the `has` hook returns nonzero WITHOUT pushing a value (row 115).
    static HAS_NOPUSH: Cell<bool> = const { Cell::new(false) };
}

fn hook(s: String) {
    HOOKLOG.with(|h| h.borrow_mut().push(s));
}

unsafe extern "C" fn ud_has(j: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    let d = from_c(data as *const c_char);
    if HAS_NOPUSH.with(|c| c.get()) && n.starts_with('q') {
        hook(format!("has(data={d},name={n})->1 nopush"));
        return 1;
    }
    if n.starts_with('h') || n == "7" {
        cur().js_pushnumber(j, 42.0);
        hook(format!("has(data={d},name={n})->1 pushed 42"));
        return 1;
    }
    hook(format!("has(data={d},name={n})->0"));
    let _ = j;
    0
}

unsafe extern "C" fn ud_put(_j: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    let d = from_c(data as *const c_char);
    let r = (n.starts_with('p') || n == "3") as c_int;
    hook(format!("put(data={d},name={n})->{r}"));
    r
}

unsafe extern "C" fn ud_del(_j: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    let d = from_c(data as *const c_char);
    let r = (n.starts_with('d') || n == "5") as c_int;
    hook(format!("del(data={d},name={n})->{r}"));
    r
}

unsafe extern "C" fn ud_fin(_j: JS, data: *mut c_void) {
    hook(format!("finalize(data={})", from_c(data as *const c_char)));
}

/* --------------------------------------------------------------- targets */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TK {
    Plain,
    PlainProtoNull,
    PlainProtoObj,
    Array,
    StrShort,
    StrLong,
    StrUtf,
    NumObj,
    NumNan,
    BoolTrue,
    BoolFalse,
    JsFun,
    CFun,
    CFunX,
    CCtor,
    Regexp,
    Date,
    Err,
    UserData,
    UserDataX,
    UserDataNullProto,
    Args,
    Global,
    MathObj,
    JsonObj,
    IterObj,
    PrimStr,
    PrimNum,
    PrimBool,
    PrimUndef,
    PrimNull,
}

const ALL_TK: &[TK] = &[
    TK::Plain,
    TK::PlainProtoNull,
    TK::PlainProtoObj,
    TK::Array,
    TK::StrShort,
    TK::StrLong,
    TK::StrUtf,
    TK::NumObj,
    TK::NumNan,
    TK::BoolTrue,
    TK::BoolFalse,
    TK::JsFun,
    TK::CFun,
    TK::CFunX,
    TK::CCtor,
    TK::Regexp,
    TK::Date,
    TK::Err,
    TK::UserData,
    TK::UserDataX,
    TK::UserDataNullProto,
    TK::Args,
    TK::Global,
    TK::MathObj,
    TK::JsonObj,
    TK::IterObj,
    TK::PrimStr,
    TK::PrimNum,
    TK::PrimBool,
    TK::PrimUndef,
    TK::PrimNull,
];

/// The classes whose property behaviour the C special-cases, plus a couple of
/// generic ones -- used for the (expensive) attribute cross-product.
const CLASS_TK: &[TK] = &[
    TK::Plain,
    TK::Array,
    TK::StrLong,
    TK::NumObj,
    TK::BoolTrue,
    TK::JsFun,
    TK::CFun,
    TK::Regexp,
    TK::Date,
    TK::UserData,
    TK::UserDataX,
    TK::Args,
    TK::Global,
    TK::PrimStr,
];

/// Build the target and store it in the global `T`.  Returns a transcript of the
/// construction itself.
unsafe fn setup_target(l: &Lib, j: JS, tk: TK) -> String {
    let mut r = String::new();
    match tk {
        TK::Plain => l.js_newobject(j),
        TK::PlainProtoNull => {
            // row 72: a non-object on top -> prototype NULL, argument still popped
            l.js_pushnumber(j, 5.0);
            let before = l.js_gettop(j);
            l.js_newobjectx(j);
            r.push_str(&format!("d={} ", l.js_gettop(j) - before));
        }
        TK::PlainProtoObj => {
            l.js_newobject(j);
            l.js_pushstring(j, cn!("from-proto"));
            l.js_setproperty(j, -2, cn!("ip"));
            let before = l.js_gettop(j);
            l.js_newobjectx(j);
            r.push_str(&format!("d={} ", l.js_gettop(j) - before));
        }
        TK::Array => l.js_newarray(j),
        TK::StrShort => l.js_newstring(j, cn!("short")),
        TK::StrLong => l.js_newstring(j, cn!("a string well over fifteen bytes long")),
        // row 77: astral runes count as 2 in u.s.length
        TK::StrUtf => l.js_newstring(j, cn!("a\u{7f}\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}z")),
        TK::NumObj => l.js_newnumber(j, 12.5),
        TK::NumNan => l.js_newnumber(j, f64::NAN),
        TK::BoolTrue => l.js_newboolean(j, 42), // stored verbatim, not !!v
        TK::BoolFalse => l.js_newboolean(j, 0),
        TK::JsFun => {
            let rc = push_expr(l, j, "(function fx(a,b){ return String(a)+'/'+String(b) })");
            r.push_str(&format!("rc={rc} "));
        }
        TK::CFun => l.js_newcfunction(j, Some(cf_ret), cn!("cfret"), 3),
        TK::CFunX => l.js_newcfunctionx(
            j,
            Some(cf_ret),
            cn!("cfretx"),
            0,
            PAYLOAD as *mut c_void,
            Some(fin_cb),
        ),
        TK::CCtor => {
            l.js_newobject(j);
            l.js_newcconstructor(j, Some(cf_ret), Some(cc_ctor), cn!("Ctor"), 1);
        }
        TK::Regexp => {
            let rc = push_expr(l, j, "/a(b+)c/gim");
            r.push_str(&format!("rc={rc} "));
        }
        TK::Date => {
            let rc = push_expr(l, j, "new Date(1234567890123)");
            r.push_str(&format!("rc={rc} "));
        }
        TK::Err => l.newerror("js_newtypeerror", j, cn!("an error message")),
        TK::UserData => {
            l.js_newobject(j);
            l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(ud_fin));
        }
        TK::UserDataX => {
            l.js_newobject(j);
            l.js_newuserdatax(
                j,
                N_TAG,
                PAYLOAD as *mut c_void,
                Some(ud_has),
                Some(ud_put),
                Some(ud_del),
                Some(ud_fin),
            );
        }
        TK::UserDataNullProto => {
            l.js_pushnumber(j, 7.0); // non-object -> prototype NULL (row 81)
            l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(ud_fin));
        }
        TK::Args => {
            let rc = push_expr(l, j, "(function(){ return arguments })(1,'two',3)");
            r.push_str(&format!("rc={rc} "));
        }
        TK::Global => l.js_pushglobal(j),
        TK::MathObj => {
            let rc = push_expr(l, j, "Math");
            r.push_str(&format!("rc={rc} "));
        }
        TK::JsonObj => {
            let rc = push_expr(l, j, "JSON");
            r.push_str(&format!("rc={rc} "));
        }
        TK::IterObj => {
            l.js_newobject(j);
            l.js_pushnumber(j, 1.0);
            l.js_setproperty(j, -2, cn!("a"));
            l.js_pushiterator(j, -1, 1);
            l.js_replace(j, -2); // drop the target, keep the iterator
        }
        TK::PrimStr => l.js_pushstring(j, cn!("primitive string value")),
        TK::PrimNum => l.js_pushnumber(j, 3.25),
        TK::PrimBool => l.js_pushboolean(j, 1),
        TK::PrimUndef => l.js_pushundefined(j),
        TK::PrimNull => l.js_pushnull(j),
    }
    r.push_str(&format!("top={} ", l.js_gettop(j)));
    l.js_setglobal(j, N_T);
    r
}

/* ------------------------------------------------------------- JS helpers */

const JS_SETUP: &str = r#"
function jsnap() {
  var o = T, r = [];
  function s(f) { try { return String(f()) } catch (e) { return 'E<' + e + '>' } }
  r.push('len=' + s(function(){ return o.length }));
  r.push('forin=' + s(function(){ var a=[]; for (var k in o) a.push(k); return a.join('~') }));
  r.push('keys=' + s(function(){ return Object.keys(o).join('~') }));
  r.push('own=' + s(function(){ return Object.getOwnPropertyNames(o).join('~') }));
  r.push('in=' + s(function(){ var a=[]; for (var i=-1;i<14;++i) a.push((i in o)?1:0); return a.join('') }));
  r.push('el=' + s(function(){ var a=[],i; for (i=0;i<14;++i) a.push(s(function(){ return o[i] })); return a.join('~') }));
  r.push('json=' + s(function(){ var n = o.length; if (typeof n === 'number' && n > 200) return 'skip'; return JSON.stringify(o) }));
  r.push('cls=' + s(function(){ return Object.prototype.toString.call(o) }));
  r.push('ext=' + s(function(){ return Object.isExtensible(o) + '/' + Object.isSealed(o) + '/' + Object.isFrozen(o) }));
  return r.join(' ');
}
function probe() {
  var o = T, r = [];
  function s(f) { try { return String(f()) } catch (e) { return 'E<' + e + '>' } }
  r.push(s(function(){ return typeof o }));
  r.push(s(function(){ return String(o) }));
  r.push(s(function(){ return +o }));
  r.push(s(function(){ return !!o }));
  r.push(s(function(){ return o.toString() }));
  r.push(s(function(){ return o.valueOf() }));
  r.push(s(function(){ return Object.prototype.toString.call(o) }));
  r.push(s(function(){ return o instanceof Object }));
  r.push(s(function(){ return Array.isArray(o) }));
  r.push(s(function(){ return o.length }));
  r.push(s(function(){ o.zzz = 1; return o.zzz }));
  r.push(s(function(){ return delete o.zzz }));
  r.push(s(function(){ return o() }));
  r.push(s(function(){ return new o() }));
  r.push(s(function(){ return o.hasOwnProperty('length') }));
  r.push(s(function(){ return Object.getPrototypeOf(o) === Object.prototype }));
  r.push(s(function(){ return Object.getPrototypeOf(o) === null }));
  r.push(s(function(){ return o[0] }));
  r.push(s(function(){ o[0] = 'W'; return o[0] }));
  r.push(s(function(){ return 'length' in o }));
  return r.join(' | ');
}
function amut(k, a, b) {
  var o = T;
  switch (k) {
  case 0: return String(o.sort());
  case 1: return String(o.splice(a, b, 'S1', 'S2'));
  case 2: return String(o.reverse());
  case 3: return String(o.push('P' + a));
  case 4: return String(o.pop());
  case 5: return String(o.shift());
  case 6: return String(o.unshift('U' + a));
  case 7: return String(o.slice(a, b));
  case 8: return String(o.join('-'));
  case 9: return String(o.concat([a, b]));
  case 10: o[a] = 'J' + b; return 'set';
  case 11: return String(delete o[a]);
  case 12: o.length = a; return 'len';
  case 13: return String(Object.seal(o) === o);
  case 14: return String(Object.freeze(o) === o);
  case 15: return String(Object.preventExtensions(o) === o);
  case 16: Object.defineProperty(o, String(a), {value: 'D' + b, enumerable: false}); return 'def';
  case 17: Object.defineProperty(o, 'g' + a, {get: function(){ return 'G' + b }}); return 'defacc';
  case 18: return String(o.indexOf('P' + a));
  case 19: return String(o.lastIndexOf('P' + a));
  case 20: return String(o.hasOwnProperty(String(a)));
  case 21: return String(o.propertyIsEnumerable(String(a)));
  case 22: o.length = a + 0.5; return 'fraclen';
  case 23: o.length = -a - 1; return 'neglen';
  case 24: o.length = 1 / 0; return 'inflen';
  }
  return 'nop';
}
"#;

unsafe fn install_js(l: &Lib, j: JS) -> c_int {
    let cs = cstr(JS_SETUP);
    let rc = l.js_dostring(j, cs.as_ptr());
    // a broken helper script would make every JS-side comparison vacuous
    assert_eq!(rc, 0, "{}: the JS helper script failed to load", l.name);
    rc
}

/// Call a pre-installed 0-argument JS helper and return its result as a string.
unsafe fn call_js0(l: &Lib, j: JS, name: *const c_char) -> String {
    l.js_getglobal(j, name);
    l.js_pushundefined(j);
    let rc = l.js_pcall(j, 0);
    let s = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 1);
    format!("rc={rc} {s}")
}

unsafe fn call_amut(l: &Lib, j: JS, k: c_int, a: f64, b: f64) -> String {
    l.js_getglobal(j, N_AMUT);
    l.js_pushundefined(j);
    l.js_pushnumber(j, k as f64);
    l.js_pushnumber(j, a);
    l.js_pushnumber(j, b);
    let rc = l.js_pcall(j, 3);
    let s = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 1);
    format!("amut{k}({a},{b}) rc={rc} {s}")
}

/* --------------------------------------------------------- the C-API probe */

#[derive(Clone, Debug)]
enum V {
    Undef,
    Null,
    Bool(c_int),
    Num(f64),
    Str(String),
    Obj,
    Arr,
    Fun,
}

unsafe fn push_v(l: &Lib, j: JS, v: &V) {
    match v {
        V::Undef => l.js_pushundefined(j),
        V::Null => l.js_pushnull(j),
        V::Bool(b) => l.js_pushboolean(j, *b),
        V::Num(x) => l.js_pushnumber(j, *x),
        V::Str(s) => {
            let cs = cstr(s);
            l.js_pushstring(j, cs.as_ptr())
        }
        V::Obj => l.js_newobject(j),
        V::Arr => l.js_newarray(j),
        V::Fun => l.js_newcfunction(j, Some(cf_ret), cn!("v"), 0),
    }
}

/// The accessor operand kinds `jsR_tofunction` distinguishes (row 110-112).
unsafe fn push_acc(l: &Lib, j: JS, kind: u8) -> &'static str {
    match kind {
        0 => {
            l.js_pushundefined(j);
            "undef"
        }
        1 => {
            l.js_pushnull(j);
            "null"
        }
        2 => {
            l.js_newcfunction(j, Some(cf_getter), cn!("g"), 0);
            "cfun-get"
        }
        3 => {
            l.js_newcfunction(j, Some(cf_setter), cn!("s"), 1);
            "cfun-set"
        }
        4 => {
            let _ = push_expr(l, j, "(function(v){ return 'jsacc' })");
            "jsfun"
        }
        5 => {
            l.js_newobject(j);
            "plainobj"
        }
        6 => {
            l.js_pushnumber(j, 42.0);
            "number"
        }
        _ => {
            l.js_newarray(j);
            "array"
        }
    }
}

#[derive(Clone, Debug)]
enum Op {
    Describe,
    Has(NameV),
    Get(NameV),
    Set(NameV, V),
    Def(NameV, V, c_int),
    Del(NameV),
    DefAcc(NameV, c_int, u8, u8),
    GetLen,
    SetLen(c_int),
    HasIdx(c_int),
    GetIdx(c_int),
    SetIdx(c_int, V),
    DelIdx(c_int),
    /// `js_pushiterator(J, idx, own)` then drain it with `js_nextiterator`.
    Iter(c_int),
    /// Iterate, deleting `name` from the target after the first step (row 140).
    IterDelDuring(NameV),
    /// Iterate two steps and show that the first returned pointer aliases
    /// `J->scratch` (row 140).
    IterScratch,
    /// `js_nextiterator` on something that is not a JS_CITERATOR (row 141).
    NextNonIterator,
}

thread_local! {
    static OP: RefCell<Op> = const { RefCell::new(Op::Describe) };
    /// false -> address the target with a positive (BOT-relative) index,
    /// true  -> with a negative (TOP-relative) one.
    static NEG: Cell<bool> = const { Cell::new(false) };
}

fn tidx(nargs: c_int) -> c_int {
    if NEG.with(|c| c.get()) {
        -1 - nargs
    } else {
        1
    }
}

unsafe fn drain_iter(l: &Lib, j: JS, cap: usize) -> String {
    let mut names: Vec<String> = vec![];
    loop {
        let p = l.js_nextiterator(j, -1);
        if p.is_null() {
            break;
        }
        names.push(nshow(std::ffi::CStr::from_ptr(p).to_bytes()));
        if names.len() >= cap {
            names.push("...".into());
            break;
        }
    }
    names.join(",")
}

unsafe fn do_op(l: &Lib, j: JS, op: &Op) -> String {
    l.js_getglobal(j, N_T); // the target lands at BOT-relative index 1
    match op {
        Op::Describe => describe(l, j, tidx(0)),
        Op::Has(n) => {
            let cs = ncstr(n);
            let before = l.js_gettop(j);
            let h = l.js_hasproperty(j, tidx(0), cs.as_ptr());
            let after = l.js_gettop(j);
            let mut s = format!("has={h} d={}", after - before);
            if after > before {
                s.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
            }
            s
        }
        Op::Get(n) => {
            let cs = ncstr(n);
            let before = l.js_gettop(j);
            l.js_getproperty(j, tidx(0), cs.as_ptr());
            let d = l.js_gettop(j) - before;
            let mut s = format!("get d={d}");
            if d > 0 {
                s.push_str(&format!(
                    " ty={} v={}",
                    from_c(l.js_typeof(j, -1)),
                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                ));
            }
            s
        }
        Op::Set(n, v) => {
            let cs = ncstr(n);
            push_v(l, j, v);
            let before = l.js_gettop(j);
            l.js_setproperty(j, tidx(1), cs.as_ptr());
            let mut s = format!("set d={}", l.js_gettop(j) - before);
            // read it straight back through the same index
            l.js_getproperty(j, tidx(0), cs.as_ptr());
            s.push_str(&format!(" back={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            s
        }
        Op::Def(n, v, atts) => {
            let cs = ncstr(n);
            push_v(l, j, v);
            let before = l.js_gettop(j);
            l.js_defproperty(j, tidx(1), cs.as_ptr(), *atts);
            let mut s = format!("def d={}", l.js_gettop(j) - before);
            l.js_getproperty(j, tidx(0), cs.as_ptr());
            s.push_str(&format!(" back={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            s
        }
        Op::Del(n) => {
            let cs = ncstr(n);
            let before = l.js_gettop(j);
            l.js_delproperty(j, tidx(0), cs.as_ptr());
            let mut s = format!("del d={}", l.js_gettop(j) - before);
            let h = l.js_hasproperty(j, tidx(0), cs.as_ptr());
            s.push_str(&format!(" still={h}"));
            if h != 0 {
                l.js_pop(j, 1);
            }
            s
        }
        Op::DefAcc(n, atts, gk, sk) => {
            let cs = ncstr(n);
            let g = push_acc(l, j, *gk);
            let sname = push_acc(l, j, *sk);
            let before = l.js_gettop(j);
            l.js_defaccessor(j, tidx(2), cs.as_ptr(), *atts);
            let mut s = format!("defacc({g},{sname}) d={}", l.js_gettop(j) - before);
            l.js_getproperty(j, tidx(0), cs.as_ptr());
            s.push_str(&format!(" get={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            l.js_pushstring(j, cn!("written"));
            l.js_setproperty(j, tidx(1), cs.as_ptr());
            l.js_getproperty(j, tidx(0), cs.as_ptr());
            s.push_str(&format!(" after={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            s
        }
        Op::GetLen => {
            let before = l.js_gettop(j);
            let n = l.js_getlength(j, tidx(0));
            format!("getlength={n} d={}", l.js_gettop(j) - before)
        }
        Op::SetLen(k) => {
            let before = l.js_gettop(j);
            l.js_setlength(j, tidx(0), *k);
            let mut s = format!("setlength({k}) d={}", l.js_gettop(j) - before);
            s.push_str(&format!(" now={}", l.js_getlength(j, tidx(0))));
            s
        }
        Op::HasIdx(k) => {
            let before = l.js_gettop(j);
            let h = l.js_hasindex(j, tidx(0), *k);
            let after = l.js_gettop(j);
            let mut s = format!("hasindex({k})={h} d={}", after - before);
            if after > before {
                s.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
            }
            s
        }
        Op::GetIdx(k) => {
            let before = l.js_gettop(j);
            l.js_getindex(j, tidx(0), *k);
            let d = l.js_gettop(j) - before;
            let mut s = format!("getindex({k}) d={d}");
            if d > 0 {
                s.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            }
            s
        }
        Op::SetIdx(k, v) => {
            push_v(l, j, v);
            let before = l.js_gettop(j);
            l.js_setindex(j, tidx(1), *k);
            let mut s = format!("setindex({k}) d={}", l.js_gettop(j) - before);
            l.js_getindex(j, tidx(0), *k);
            s.push_str(&format!(" back={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            s
        }
        Op::DelIdx(k) => {
            let before = l.js_gettop(j);
            l.js_delindex(j, tidx(0), *k);
            let mut s = format!("delindex({k}) d={}", l.js_gettop(j) - before);
            let h = l.js_hasindex(j, tidx(0), *k);
            s.push_str(&format!(" still={h}"));
            if h != 0 {
                l.js_pop(j, 1);
            }
            s
        }
        Op::Iter(own) => {
            let before = l.js_gettop(j);
            l.js_pushiterator(j, tidx(0), *own);
            let ty = from_c(l.js_typeof(j, -1));
            let names = drain_iter(l, j, 400);
            let mut s = format!("iter(own={own}) ty={ty} d={} [{names}]", l.js_gettop(j) - before);
            // exhausted iterators keep returning NULL
            s.push_str(&format!(
                " again={}",
                l.js_nextiterator(j, -1).is_null()
            ));
            // the target slot may have been rewritten by js_toobject (row 139)
            s.push_str(&format!(" target_ty={}", from_c(l.js_typeof(j, tidx(1)))));
            l.js_pop(j, 1);
            s
        }
        Op::IterDelDuring(n) => {
            let cs = ncstr(n);
            l.js_pushiterator(j, tidx(0), 1);
            let first = l.js_nextiterator(j, -1);
            let f = if first.is_null() {
                "<NULL>".to_string()
            } else {
                nshow(std::ffi::CStr::from_ptr(first).to_bytes())
            };
            // delete a property that the snapshot already recorded
            l.js_delproperty(j, tidx(1), cs.as_ptr());
            let rest = drain_iter(l, j, 400);
            let s = format!("iterdel first={f} rest=[{rest}]");
            l.js_pop(j, 1);
            s
        }
        Op::IterScratch => {
            l.js_pushiterator(j, tidx(0), 1);
            let p1 = l.js_nextiterator(j, -1);
            let a = if p1.is_null() {
                "<NULL>".to_string()
            } else {
                nshow(std::ffi::CStr::from_ptr(p1).to_bytes())
            };
            let p2 = l.js_nextiterator(j, -1);
            let b = if p2.is_null() {
                "<NULL>".to_string()
            } else {
                nshow(std::ffi::CStr::from_ptr(p2).to_bytes())
            };
            // p1 may now read the SECOND name: J->scratch is one shared buffer
            let a2 = if p1.is_null() {
                "<NULL>".to_string()
            } else {
                nshow(std::ffi::CStr::from_ptr(p1).to_bytes())
            };
            let same = !p1.is_null() && p1 == p2;
            let s = format!("scratch first={a} second={b} first_now={a2} aliased={same}");
            l.js_pop(j, 1);
            s
        }
        Op::NextNonIterator => {
            let p = l.js_nextiterator(j, tidx(0));
            format!("nextiterator on non-iterator = {}", from_c(p))
        }
    }
}

unsafe extern "C" fn cf_op(j: JS) {
    let l = cur();
    let op = OP.with(|o| o.borrow().clone());
    let s = do_op(l, j, &op);
    let cs = cstr(&s);
    l.js_pushstring(j, cs.as_ptr());
}

/// Run one op inside a protected frame and return its transcript.
unsafe fn run(l: &Lib, j: JS, op: Op) -> String {
    OP.with(|o| *o.borrow_mut() = op);
    let base = l.js_gettop(j);
    l.js_newcfunction(j, Some(cf_op), N_OP, 0);
    l.js_pushundefined(j);
    let rc = l.js_pcall(j, 0);
    let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 1);
    let after = l.js_gettop(j);
    let r = format!("[rc={rc} {v} top {base}->{after}]");
    drain_to(l, j, base);
    r
}

/* ------------------------------------------------------- the C-API snapshot */

unsafe extern "C" fn cf_csnap(j: JS) {
    let l = cur();
    l.js_getglobal(j, N_T);
    let mut s = String::new();
    let len = l.js_getlength(j, 1);
    s.push_str(&format!("len={len} "));
    for k in -1..15 {
        let before = l.js_gettop(j);
        let h = l.js_hasindex(j, 1, k);
        if l.js_gettop(j) > before {
            s.push_str(&format!("{}:{} ", k, from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
        } else {
            s.push_str(&format!("{k}:_{h} "));
        }
    }
    for own in [1, 0] {
        l.js_pushiterator(j, 1, own);
        s.push_str(&format!("it{own}=[{}] ", drain_iter(l, j, 400)));
        l.js_pop(j, 1);
    }
    // jsrepr.c:125 `reprarray` loops `js_getlength` times, so an array whose
    // length was pushed into the millions would make this quadratic; the length
    // is identical in both libraries so skipping is symmetric.
    if len <= 200 {
        s.push_str(&format!("repr={}", from_c(l.js_tryrepr(j, 1, ERRSTR))));
    } else {
        s.push_str("repr=skip");
    }
    let cs = cstr(&s);
    l.js_pushstring(j, cs.as_ptr());
}

unsafe fn csnap(l: &Lib, j: JS) -> String {
    let base = l.js_gettop(j);
    l.js_newcfunction(j, Some(cf_csnap), cn!("csnap"), 0);
    l.js_pushundefined(j);
    let rc = l.js_pcall(j, 0);
    let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 1);
    drain_to(l, j, base);
    format!("C(rc={rc} {v})")
}

/// The full observable state: through the C API and through JS.
unsafe fn snap_all(l: &Lib, j: JS) -> String {
    format!("{} JS({})", csnap(l, j), call_js0(l, j, N_JSNAP))
}

/* ====================================================================== */
/*  Rows 71-85: object creation                                            */
/* ====================================================================== */

#[test]
fn t_object_creation_matrix() {
    for &tk in ALL_TK {
        for flags in [0, JS_STRICT] {
            diff2(&format!("create {tk:?} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, tk);
                let mut r = format!("install={irc} mk={mk}");
                r.push_str(&format!(" desc={}", run(l, j, Op::Describe)));
                NEG.with(|c| c.set(true));
                r.push_str(&format!(" descneg={}", run(l, j, Op::Describe)));
                NEG.with(|c| c.set(false));
                r.push_str(&format!(" {}", snap_all(l, j)));
                r.push_str(&format!(" probe={}", call_js0(l, j, N_PROBE)));
                r.push_str(&format!(" after={}", snap_all(l, j)));
                r.push_str(&format!(" top={}", l.js_gettop(j)));
                l.js_gc(j, 0);
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Row 72: `js_newobjectx` with every value tag on top.
#[test]
fn t_newobjectx_prototypes() {
    let mut rng = Rng::new(0x0B1E_C701);
    let mut vals: Vec<V> = vec![
        V::Undef,
        V::Null,
        V::Bool(0),
        V::Bool(1),
        V::Num(0.0),
        V::Num(-0.0),
        V::Num(f64::NAN),
        V::Str(String::new()),
        V::Str("proto".into()),
        V::Obj,
        V::Arr,
        V::Fun,
    ];
    for _ in 0..12 {
        vals.push(V::Num(rng.f64_sane()));
        vals.push(V::Str(rng.ascii_string(20)));
    }
    for (i, v) in vals.into_iter().enumerate() {
        for flags in [0, JS_STRICT] {
            let v = v.clone();
            diff2(&format!("newobjectx {i} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                push_v(l, j, &v);
                if matches!(v, V::Obj) {
                    l.js_pushstring(j, cn!("inherited-value"));
                    l.js_setproperty(j, -2, cn!("ip"));
                }
                let before = l.js_gettop(j);
                l.js_newobjectx(j);
                let mut r = format!("install={irc} d={} ", l.js_gettop(j) - before);
                r.push_str(&describe_top(l, j));
                l.js_setglobal(j, N_T);
                r.push_str(&format!(" {}", snap_all(l, j)));
                r.push_str(&format!(" ip={}", run(l, j, Op::Get(nb("ip")))));
                r.push_str(&format!(" hasownproto={}", run(l, j, Op::Has(nb("toString")))));
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Rows 75-77: `js_newstring` inline vs heap, and the rune count in `u.s.length`.
#[test]
fn t_newstring_lengths() {
    let mut rng = Rng::new(0x5751_4E67);
    let mut strs: Vec<String> = vec![
        String::new(),
        "a".into(),
        "0123456789abcde".into(),  // 15 -> inline
        "0123456789abcdef".into(), // 16 -> js_strdup
        "0123456789abcdefg".into(),
        "\u{7f}".into(),
        "\u{80}".into(),
        "\u{7ff}".into(),
        "\u{800}".into(),
        "\u{ffff}".into(),
        "\u{10000}".into(),
        "\u{10ffff}".into(),
        "\u{10ffff}\u{10ffff}\u{10ffff}\u{10ffff}".into(),
        "a\u{10348}b".into(),
        "x".repeat(300),
    ];
    for _ in 0..40 {
        strs.push(rng.unicode_string(10));
    }
    for _ in 0..20 {
        strs.push(rng.ascii_string(20));
    }
    for (i, s) in strs.into_iter().enumerate() {
        let s2 = s.clone();
        diff2(&format!("newstring {i} len={}", s.len()), move |l| unsafe {
            let j = new_state(l, 0);
            let irc = install_js(l, j);
            let cs = cstr(&s2);
            l.js_newstring(j, cs.as_ptr());
            let mut r = format!("install={irc} ");
            r.push_str(&describe_top(l, j));
            l.js_setglobal(j, N_T);
            r.push_str(&format!(" len={}", run(l, j, Op::Get(nb("length")))));
            r.push_str(&format!(" {}", snap_all(l, j)));
            for k in [-1, 0, 1, 2, 3, 200] {
                r.push_str(&format!(" i{k}={}", run(l, j, Op::GetIdx(k))));
            }
            // in-range index names are readonly, out-of-range ones fall through
            for nm in ["0", "1", "01", "1.5", "-1", "length"] {
                r.push_str(&format!(
                    " s[{nm}]={}",
                    run(l, j, Op::Set(nb(nm), V::Str("W".into())))
                ));
            }
            r.push_str(&format!(" {}", snap_all(l, j)));
            l.js_gc(j, 0);
            l.js_freestate(j);
            r
        });
    }
}

/// Row 74: `js_newboolean` / `js_newnumber` over randomised values.  `u.boolean`
/// is stored VERBATIM (jsvalue.c:369 does not normalise to `!!v`), while
/// `js_pushboolean` does normalise, so `Boolean.prototype.valueOf` and
/// `js_torepr` can disagree about the raw payload.
#[test]
fn t_newnumber_newboolean() {
    let mut rng = Rng::new(0x4E42_0074);
    let mut nums: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0, // subnormal
        i32::MIN as f64,
        i32::MAX as f64,
        u32::MAX as f64,
        2147483648.0,
        4294967296.0,
        9007199254740992.0,
        1e21,
        1e-7,
        1e300,
        -1e300,
    ];
    for _ in 0..80 {
        nums.push(rng.f64_sane());
    }
    let mut bools: Vec<c_int> = vec![0, 1, -1, 2, 42, i32::MIN, i32::MAX, 256, 0x100];
    for _ in 0..20 {
        bools.push(rng.next_u32() as c_int);
    }
    for (i, x) in nums.into_iter().enumerate() {
        diff2(&format!("newnumber {i} {}", fbits(x)), move |l| unsafe {
            let j = new_state(l, 0);
            let irc = install_js(l, j);
            l.js_newnumber(j, x);
            let mut r = format!("install={irc} {}", describe_top(l, j));
            l.js_setglobal(j, N_T);
            r.push_str(&format!(" {}", snap_all(l, j)));
            r.push_str(&format!(" probe={}", call_js0(l, j, N_PROBE)));
            // js_tonumber / js_tointeger through the wrapper
            l.js_getglobal(j, N_T);
            r.push_str(&format!(
                " tonum={} toint={} toi32={} tou32={} bool={}",
                fbits(l.js_tonumber(j, -1)),
                l.js_tointeger(j, -1),
                l.js_toint32(j, -1),
                l.js_touint32(j, -1),
                l.js_toboolean(j, -1)
            ));
            l.js_pop(j, 1);
            l.js_freestate(j);
            r
        });
    }
    for (i, b) in bools.into_iter().enumerate() {
        diff2(&format!("newboolean {i} {b}"), move |l| unsafe {
            let j = new_state(l, 0);
            let irc = install_js(l, j);
            l.js_newboolean(j, b);
            let mut r = format!("install={irc} {}", describe_top(l, j));
            l.js_setglobal(j, N_T);
            r.push_str(&format!(" {}", snap_all(l, j)));
            r.push_str(&format!(" probe={}", call_js0(l, j, N_PROBE)));
            l.js_getglobal(j, N_T);
            r.push_str(&format!(
                " tonum={} bool={}",
                fbits(l.js_tonumber(j, -1)),
                l.js_toboolean(j, -1)
            ));
            l.js_pop(j, 1);
            l.js_freestate(j);
            r
        });
    }
}

/// Rows 78-80: `js_newcfunction` / `js_newcfunctionx` / `js_newcconstructor`.
#[test]
fn t_new_cfunctions() {
    for length in [0, 1, 3, 7, -1, i32::MAX] {
        for kind in 0..3u8 {
            for withfin in [false, true] {
                for flags in [0, JS_STRICT] {
                    let t = diff2(
                        &format!("cfun kind={kind} len={length} fin={withfin} flags={flags}"),
                        move |l| unsafe {
                            let j = new_state(l, flags);
                            let irc = install_js(l, j);
                            let before = l.js_gettop(j);
                            match kind {
                                0 => l.js_newcfunction(j, Some(cf_ret), cn!("f0"), length),
                                1 => l.js_newcfunctionx(
                                    j,
                                    Some(cf_ret),
                                    cn!("f1"),
                                    length,
                                    PAYLOAD as *mut c_void,
                                    if withfin { Some(fin_cb) } else { None },
                                ),
                                _ => {
                                    l.js_newobject(j);
                                    l.js_pushstring(j, cn!("on-proto"));
                                    l.js_setproperty(j, -2, cn!("pp"));
                                    l.js_newcconstructor(
                                        j,
                                        Some(cf_ret),
                                        Some(cc_ctor),
                                        cn!("f2"),
                                        length,
                                    )
                                }
                            }
                            let mut r = format!("install={irc} d={} ", l.js_gettop(j) - before);
                            r.push_str(&describe_top(l, j));
                            l.js_setglobal(j, N_T);
                            for nm in ["length", "prototype", "name", "constructor"] {
                                r.push_str(&format!(" {nm}={}", run(l, j, Op::Get(nb(nm)))));
                            }
                            // 'length' is READONLY|DONTENUM|DONTCONF
                            r.push_str(&format!(
                                " setlen={}",
                                run(l, j, Op::Set(nb("length"), V::Num(99.0)))
                            ));
                            r.push_str(&format!(" dellen={}", run(l, j, Op::Del(nb("length")))));
                            // 'prototype' is DONTENUM|DONTCONF and carries 'constructor'
                            r.push_str(&format!(
                                " delproto={}",
                                run(l, j, Op::Del(nb("prototype")))
                            ));
                            r.push_str(&format!(" {}", snap_all(l, j)));
                            r.push_str(&format!(" probe={}", call_js0(l, j, N_PROBE)));
                            l.js_gc(j, 0);
                            l.js_delglobal(j, N_T);
                            l.js_gc(j, 0);
                            r.push_str(&format!(" top={}", l.js_gettop(j)));
                            l.js_freestate(j);
                            r
                        },
                    );
                    if kind == 1 && withfin {
                        assert!(
                            t.contains("[fin PAY]"),
                            "js_newcfunctionx finalizer never ran: {t}"
                        );
                    }
                    if !withfin || kind != 1 {
                        assert!(
                            !t.contains("[fin PAY]"),
                            "unexpected finalizer for kind={kind} fin={withfin}: {t}"
                        );
                    }
                }
            }
        }
    }
}

/// Rows 81-85: `js_newuserdata` / `js_newuserdatax`, including the exact ORDER
/// and COUNT of every hook invocation.
#[test]
fn t_userdatax_hook_transcript() {
    // one op per line; every one of them may consult a hook
    let ops: Vec<Op> = vec![
        Op::Has(nb("hello")),
        Op::Has(nb("zero")),
        Op::Get(nb("hget")),
        Op::Get(nb("other")),
        Op::Set(nb("pset"), V::Num(1.0)),
        Op::Set(nb("wset"), V::Num(2.0)),
        Op::Def(nb("pdef"), V::Num(3.0), 0),
        Op::Def(nb("wdef"), V::Num(4.0), JS_READONLY | JS_DONTENUM),
        Op::Del(nb("dgone")),
        Op::Del(nb("wdef")),
        Op::DefAcc(nb("pacc"), 0, 2, 3),
        Op::DefAcc(nb("wacc"), 0, 2, 3),
        Op::GetLen,
        Op::SetLen(4),
        Op::HasIdx(7),
        Op::HasIdx(1),
        Op::GetIdx(3),
        Op::SetIdx(3, V::Str("three".into())),
        Op::SetIdx(9, V::Str("nine".into())),
        Op::DelIdx(5),
        Op::DelIdx(6),
        Op::Iter(1),
        Op::Iter(0),
        Op::Describe,
    ];
    for mask in 0..16u32 {
        for flags in [0, JS_STRICT] {
            let ops = ops.clone();
            let t = diff2(
                &format!("userdatax mask={mask} flags={flags}"),
                move |l| unsafe {
                    HOOKLOG.with(|h| h.borrow_mut().clear());
                    let j = new_state(l, flags);
                    let irc = install_js(l, j);
                    l.js_newobject(j);
                    l.js_pushstring(j, cn!("proto-value"));
                    l.js_setproperty(j, -2, cn!("ip"));
                    l.js_newuserdatax(
                        j,
                        N_TAG,
                        PAYLOAD as *mut c_void,
                        if mask & 1 != 0 { Some(ud_has) } else { None },
                        if mask & 2 != 0 { Some(ud_put) } else { None },
                        if mask & 4 != 0 { Some(ud_del) } else { None },
                        if mask & 8 != 0 { Some(ud_fin) } else { None },
                    );
                    let mut r = format!("install={irc} ");
                    r.push_str(&describe_top(l, j));
                    l.js_setglobal(j, N_T);
                    for op in &ops {
                        r.push_str(&format!("\n  {op:?} -> {}", run(l, j, op.clone())));
                        r.push_str(&format!("\n    hooks: {:?}", {
                            HOOKLOG.with(|h| {
                                let v = std::mem::take(&mut *h.borrow_mut());
                                v
                            })
                        }));
                    }
                    r.push_str(&format!("\n  snap {}", snap_all(l, j)));
                    r.push_str(&format!("\n  hooks: {:?}", {
                        HOOKLOG.with(|h| std::mem::take(&mut *h.borrow_mut()))
                    }));
                    r.push_str(&format!("\n  probe {}", call_js0(l, j, N_PROBE)));
                    r.push_str(&format!("\n  hooks: {:?}", {
                        HOOKLOG.with(|h| std::mem::take(&mut *h.borrow_mut()))
                    }));
                    l.js_delglobal(j, N_T);
                    l.js_gc(j, 0);
                    r.push_str(&format!("\n  after-gc hooks: {:?}", {
                        HOOKLOG.with(|h| std::mem::take(&mut *h.borrow_mut()))
                    }));
                    l.js_freestate(j);
                    r.push_str(&format!("\n  after-free hooks: {:?}", {
                        HOOKLOG.with(|h| std::mem::take(&mut *h.borrow_mut()))
                    }));
                    r
                },
            );
            for (bit, m) in [
                (1u32, "has(data="),
                (2, "put(data="),
                (4, "del(data="),
                (8, "finalize(data="),
            ] {
                if mask & bit == 0 {
                    assert!(!t.contains(m), "mask={mask} fired {m} anyway");
                } else {
                    assert!(t.contains(m), "mask={mask} never fired {m}");
                }
            }
        }
    }
}

/// Rows 81/85: the tag identity checked by `js_isuserdata` / `js_touserdata`.
#[test]
fn t_userdata_tags() {
    let mut rng = Rng::new(0x7A6D_1234);
    let mut tags: Vec<String> = vec![
        String::new(),
        "udtag".into(),
        "udta".into(),
        "udtagg".into(),
        "UDTAG".into(),
    ];
    for _ in 0..24 {
        tags.push(rng.ascii_string(12));
    }
    for (i, tag) in tags.into_iter().enumerate() {
        diff2(&format!("udtag {i}"), move |l| unsafe {
            let j = new_state(l, 0);
            let cs = cstr(&tag);
            l.js_newobject(j);
            l.js_newuserdata(j, cs.as_ptr(), PAYLOAD as *mut c_void, None);
            let a = l.js_isuserdata(j, -1, cs.as_ptr());
            let b = l.js_isuserdata(j, -1, N_TAG);
            let c = l.js_isuserdata(j, -1, cn!(""));
            let mut r = format!("same={a} udtag={b} empty={c}");
            // js_touserdata raises typeerror "not a <tag>" on a mismatch, so it
            // has to run inside a protected frame
            l.js_setglobal(j, N_T);
            TOUD_TAG.with(|t| t.set(cs.as_ptr()));
            l.js_newcfunction(j, Some(cf_touserdata), cn!("tud"), 0);
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            r.push_str(&format!(
                " tud_same rc={rc} {}",
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            ));
            l.js_pop(j, 1);
            TOUD_TAG.with(|t| t.set(N_TAG2));
            l.js_newcfunction(j, Some(cf_touserdata), cn!("tud"), 0);
            l.js_pushundefined(j);
            let rc2 = l.js_pcall(j, 0);
            r.push_str(&format!(
                " tud_other rc={rc2} {}",
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            ));
            l.js_pop(j, 1);
            // and on a non-userdata value
            l.js_newobject(j);
            r.push_str(&format!(" plain={}", l.js_isuserdata(j, -1, cs.as_ptr())));
            l.js_pop(j, 1);
            l.js_pushnumber(j, 1.0);
            r.push_str(&format!(" num={}", l.js_isuserdata(j, -1, cs.as_ptr())));
            l.js_pop(j, 1);
            l.js_freestate(j);
            r
        });
    }
}

thread_local! {
    static TOUD_TAG: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
}

unsafe extern "C" fn cf_touserdata(j: JS) {
    let l = cur();
    l.js_getglobal(j, N_T);
    let d = l.js_touserdata(j, -1, TOUD_TAG.with(|t| t.get()));
    let s = if d.is_null() {
        "<NULL>".to_string()
    } else {
        from_c(d as *const c_char)
    };
    let cs = cstr(&format!("data={s}"));
    l.js_pushstring(j, cs.as_ptr());
}

/// Row 115: a `has` hook that returns nonzero WITHOUT pushing anything.
/// `js_hasproperty` then reports 1 with an unchanged stack; nothing is popped by
/// the test, so this stays inside the frame.
#[test]
fn t_userdata_has_without_push() {
    for flags in [0, JS_STRICT] {
        diff2(&format!("has-nopush flags={flags}"), move |l| unsafe {
            HAS_NOPUSH.with(|c| c.set(true));
            HOOKLOG.with(|h| h.borrow_mut().clear());
            let j = new_state(l, flags);
            let irc = install_js(l, j);
            l.js_newobject(j);
            l.js_newuserdatax(
                j,
                N_TAG,
                PAYLOAD as *mut c_void,
                Some(ud_has),
                None,
                None,
                None,
            );
            l.js_setglobal(j, N_T);
            let mut r = format!("install={irc}");
            for nm in ["qsilent", "hvalue", "other"] {
                r.push_str(&format!(" {nm}={}", run(l, j, Op::Has(nb(nm)))));
                r.push_str(&format!(" get{nm}={}", run(l, j, Op::Get(nb(nm)))));
            }
            r.push_str(&format!(
                " hooks={:?}",
                HOOKLOG.with(|h| std::mem::take(&mut *h.borrow_mut()))
            ));
            r.push_str(&format!(" top={}", l.js_gettop(j)));
            l.js_freestate(j);
            HAS_NOPUSH.with(|c| c.set(false));
            r
        });
    }
}

/* ====================================================================== */
/*  Rows 86-117: the property API x attributes x receiver class x strict    */
/* ====================================================================== */

/// The full 8-way cross-product of JS_READONLY | JS_DONTENUM | JS_DONTCONF over
/// every special-cased receiver class, in both strict and sloppy mode.
#[test]
fn t_attribute_crossproduct() {
    for atts in 0..8 {
        for &tk in ALL_TK {
            for flags in [0, JS_STRICT] {
                diff2(
                    &format!("atts={atts} {tk:?} flags={flags}"),
                    move |l| unsafe {
                        let j = new_state(l, flags);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, tk);
                        let mut r = format!("install={irc} mk={mk}");
                        // define with the attributes, then try to observe every
                        // consequence: enumerability, writability, deletability
                        for nm in ["px", "0", "3", "length"] {
                            r.push_str(&format!(
                                "\n {nm}: def={}",
                                run(l, j, Op::Def(nb(nm), V::Str("V1".into()), atts))
                            ));
                            r.push_str(&format!(
                                " get={}",
                                run(l, j, Op::Get(nb(nm)))
                            ));
                            r.push_str(&format!(
                                " set={}",
                                run(l, j, Op::Set(nb(nm), V::Str("V2".into())))
                            ));
                            // row 107: atts are only ever OR-ed in
                            r.push_str(&format!(
                                " redef={}",
                                run(l, j, Op::Def(nb(nm), V::Str("V3".into()), atts ^ 7))
                            ));
                            r.push_str(&format!(
                                " acc={}",
                                run(l, j, Op::DefAcc(nb(nm), 0, 2, 3))
                            ));
                            r.push_str(&format!(" del={}", run(l, j, Op::Del(nb(nm)))));
                            r.push_str(&format!(" has={}", run(l, j, Op::Has(nb(nm)))));
                        }
                        r.push_str(&format!("\n {}", snap_all(l, j)));
                        // and the same names through the index API
                        for k in [0, 3, -1] {
                            r.push_str(&format!(
                                "\n idx{k}: has={} get={} set={} del={}",
                                run(l, j, Op::HasIdx(k)),
                                run(l, j, Op::GetIdx(k)),
                                run(l, j, Op::SetIdx(k, V::Num(k as f64))),
                                run(l, j, Op::DelIdx(k))
                            ));
                        }
                        r.push_str(&format!("\n {}", snap_all(l, j)));
                        r.push_str(&format!("\n probe={}", call_js0(l, j, N_PROBE)));
                        r.push_str(&format!("\n top={}", l.js_gettop(j)));
                        l.js_freestate(j);
                        r
                    },
                );
            }
        }
    }
}

/// Randomised property names (numeric-looking, very long, embedded high bytes)
/// and randomised values, driven over every receiver class in both modes.
#[test]
fn t_random_property_names() {
    let mut rng = Rng::new(0x9E37_79B9);
    let mut names: Vec<NameV> = vec![
        nb(""),
        nb("0"),
        nb("1"),
        nb("00"),
        nb("01"),
        nb("10"),
        nb("9"),
        nb("-1"),
        nb("+1"),
        nb("1.5"),
        nb(" 1"),
        nb("1 "),
        nb("214748364"),
        nb("214748365"),
        nb("2147483647"),
        nb("4294967295"),
        nb("4294967296"),
        nb("1e3"),
        nb("0x10"),
        nb("length"),
        nb("lastIndex"),
        nb("source"),
        nb("global"),
        nb("ignoreCase"),
        nb("multiline"),
        nb("callee"),
        nb("__proto__"),
        nb("toString"),
        nb("hasOwnProperty"),
        nb(&"L".repeat(300)),
        vec![0x80, 0x81, 0xfe, 0xff],
        vec![0xc3, 0xa9],
        vec![0xed, 0xa0, 0x80], // surrogate encoded as UTF-8
        vec![0xf4, 0x8f, 0xbf, 0xbf],
        vec![0xff],
    ];
    for _ in 0..40 {
        names.push(nb(&format!("{}", rng.below(40))));
    }
    for _ in 0..30 {
        names.push(nb(&rng.ascii_string(10)));
    }
    for _ in 0..30 {
        names.push(rng.raw_bytes(12));
    }
    for _ in 0..10 {
        names.push(nb(&rng.unicode_string(6)));
    }

    let mut vals: Vec<V> = vec![V::Undef, V::Null, V::Bool(0), V::Bool(1), V::Obj, V::Arr, V::Fun];
    for _ in 0..16 {
        vals.push(V::Num(rng.f64_sane()));
        vals.push(V::Str(rng.ascii_string(24)));
    }

    for _ in 0..60 {
        names.push(rng.raw_bytes(6));
    }
    for _ in 0..30 {
        names.push(nb(&format!("{}{}", rng.below(1_000_000), rng.ascii_string(2))));
    }
    // chunk the names so each state does a manageable amount of work
    let chunks: Vec<Vec<NameV>> = names.chunks(24).map(|c| c.to_vec()).collect();
    for &tk in ALL_TK {
        for flags in [0, JS_STRICT] {
            for (ci, chunk) in chunks.iter().enumerate() {
                let chunk = chunk.clone();
                let vals = vals.clone();
                let mut rr = Rng::new(0x1234_5678 ^ (ci as u64) ^ ((tk as u64) << 20));
                let picks: Vec<usize> = (0..chunk.len() * 4)
                    .map(|_| rr.below(vals.len() as u32) as usize)
                    .collect();
                let atts: Vec<c_int> = (0..chunk.len()).map(|_| rr.below(8) as c_int).collect();
                let negs: Vec<bool> = (0..chunk.len()).map(|_| rr.below(2) == 1).collect();
                diff2(
                    &format!("randnames {tk:?} flags={flags} chunk={ci}"),
                    move |l| unsafe {
                        let j = new_state(l, flags);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, tk);
                        let mut r = format!("install={irc} mk={mk}");
                        for (i, n) in chunk.iter().enumerate() {
                            NEG.with(|c| c.set(negs[i]));
                            r.push_str(&format!("\n {}:", nshow(n)));
                            r.push_str(&format!(" has={}", run(l, j, Op::Has(n.clone()))));
                            r.push_str(&format!(" get={}", run(l, j, Op::Get(n.clone()))));
                            r.push_str(&format!(
                                " set={}",
                                run(l, j, Op::Set(n.clone(), vals[picks[i * 4]].clone()))
                            ));
                            r.push_str(&format!(
                                " def={}",
                                run(
                                    l,
                                    j,
                                    Op::Def(n.clone(), vals[picks[i * 4 + 1]].clone(), atts[i])
                                )
                            ));
                            r.push_str(&format!(
                                " acc={}",
                                run(l, j, Op::DefAcc(n.clone(), atts[i], 2, 3))
                            ));
                            r.push_str(&format!(" del={}", run(l, j, Op::Del(n.clone()))));
                            r.push_str(&format!(
                                " set2={}",
                                run(l, j, Op::Set(n.clone(), vals[picks[i * 4 + 2]].clone()))
                            ));
                        }
                        NEG.with(|c| c.set(false));
                        r.push_str(&format!("\n {}", snap_all(l, j)));
                        r.push_str(&format!("\n top={}", l.js_gettop(j)));
                        l.js_gc(j, 0);
                        l.js_freestate(j);
                        r
                    },
                );
            }
        }
    }
}

/// The per-class special property names: `length` on arrays / strings /
/// functions, the five regexp names, `callee`/`length` on arguments, and
/// index-like names on string objects.
///
/// Row 109 / the `jsrun.c:840` vs `jsrun.c:749` asymmetry: `jsR_defproperty`
/// refuses a regexp `lastIndex` (goto readonly) while `jsR_setproperty` accepts
/// it and writes `u.r.last`.  Asserted explicitly below.
#[test]
fn t_special_property_names() {
    let specials = [
        "length",
        "source",
        "global",
        "ignoreCase",
        "multiline",
        "lastIndex",
        "callee",
        "0",
        "1",
        "2",
        "3",
    ];
    for &tk in &[
        TK::Array,
        TK::StrShort,
        TK::StrLong,
        TK::StrUtf,
        TK::JsFun,
        TK::CFun,
        TK::Regexp,
        TK::Args,
        TK::Plain,
        TK::NumObj,
        TK::Date,
        TK::Err,
        TK::UserData,
        TK::Global,
    ] {
        for flags in [0, JS_STRICT] {
            diff2(&format!("special {tk:?} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, tk);
                let mut r = format!("install={irc} mk={mk}");
                for nm in specials {
                    r.push_str(&format!("\n {nm}:"));
                    r.push_str(&format!(" has={}", run(l, j, Op::Has(nb(nm)))));
                    r.push_str(&format!(" get={}", run(l, j, Op::Get(nb(nm)))));
                    r.push_str(&format!(
                        " set={}",
                        run(l, j, Op::Set(nb(nm), V::Num(2.0)))
                    ));
                    r.push_str(&format!(
                        " def={}",
                        run(l, j, Op::Def(nb(nm), V::Num(3.0), 0))
                    ));
                    r.push_str(&format!(
                        " acc={}",
                        run(l, j, Op::DefAcc(nb(nm), 0, 2, 3))
                    ));
                    r.push_str(&format!(" del={}", run(l, j, Op::Del(nb(nm)))));
                }
                r.push_str(&format!("\n {}", snap_all(l, j)));
                l.js_freestate(j);
                r
            });
        }
    }

    // jsrun.c:749 vs jsrun.c:840 -- setproperty writes lastIndex, defproperty
    // refuses it.  In sloppy mode defproperty is still a throw, because
    // js_defproperty passes throw=1.
    for flags in [0, JS_STRICT] {
        let t = diff2(&format!("regexp lastIndex asym flags={flags}"), move |l| unsafe {
            let j = new_state(l, flags);
            let irc = install_js(l, j);
            let mk = setup_target(l, j, TK::Regexp);
            let mut r = format!("install={irc} mk={mk}");
            r.push_str(&format!(" pre={}", run(l, j, Op::Get(nb("lastIndex")))));
            r.push_str(&format!(
                " set={}",
                run(l, j, Op::Set(nb("lastIndex"), V::Num(5.0)))
            ));
            r.push_str(&format!(" mid={}", run(l, j, Op::Get(nb("lastIndex")))));
            r.push_str(&format!(
                " def={}",
                run(l, j, Op::Def(nb("lastIndex"), V::Num(9.0), 0))
            ));
            r.push_str(&format!(" post={}", run(l, j, Op::Get(nb("lastIndex")))));
            // u.r.last is an `unsigned short`, so the write truncates
            for v in [-1.0, 0.5, 65535.0, 65536.0, 65537.0, 1e9, f64::NAN] {
                r.push_str(&format!(
                    " w{v}={} -> {}",
                    run(l, j, Op::Set(nb("lastIndex"), V::Num(v))),
                    run(l, j, Op::Get(nb("lastIndex")))
                ));
            }
            l.js_freestate(j);
            r
        });
        assert!(
            t.contains("set=[rc=0 \"set d=-1 back=5\"") && t.contains("mid=[rc=0 \"get d=1 ty=number v=5\""),
            "js_setproperty did not write regexp lastIndex: {t}"
        );
        assert!(
            t.contains("def=[rc=1 (new TypeError(\"'lastIndex' is read-only or non-configurable\"))"),
            "js_defproperty must refuse regexp lastIndex (even sloppy: throw=1): {t}"
        );
        assert!(
            t.contains("post=[rc=0 \"get d=1 ty=number v=5\""),
            "the refused defproperty must leave lastIndex at 5: {t}"
        );
    }
}

/// Rows 87/92/93 and 110-113: getters and setters through every entry point.
#[test]
fn t_defaccessor_matrix() {
    for gk in 0..8u8 {
        for sk in 0..8u8 {
            for flags in [0, JS_STRICT] {
                for atts in [0, JS_DONTCONF, JS_READONLY, JS_DONTENUM] {
                    diff2(
                        &format!("defacc g={gk} s={sk} atts={atts} flags={flags}"),
                        move |l| unsafe {
                            let j = new_state(l, flags);
                            let irc = install_js(l, j);
                            let mk = setup_target(l, j, TK::Plain);
                            let mut r = format!("install={irc} mk={mk}");
                            r.push_str(&format!(
                                " a1={}",
                                run(l, j, Op::DefAcc(nb("acc"), atts, gk, sk))
                            ));
                            // row 111: an undefined/null operand leaves that
                            // slot untouched, so an existing accessor survives
                            r.push_str(&format!(
                                " a2={}",
                                run(l, j, Op::DefAcc(nb("acc"), atts, 0, 0))
                            ));
                            r.push_str(&format!(" get={}", run(l, j, Op::Get(nb("acc")))));
                            r.push_str(&format!(
                                " set={}",
                                run(l, j, Op::Set(nb("acc"), V::Num(1.0)))
                            ));
                            // row 113: redefining a JS_DONTCONF accessor
                            r.push_str(&format!(
                                " a3={}",
                                run(l, j, Op::DefAcc(nb("acc"), 0, 4, 4))
                            ));
                            r.push_str(&format!(" get2={}", run(l, j, Op::Get(nb("acc")))));
                            r.push_str(&format!(" del={}", run(l, j, Op::Del(nb("acc")))));
                            r.push_str(&format!(" {}", snap_all(l, j)));
                            // a getter/setter on the PROTOTYPE (rows 92/95)
                            r.push_str(&format!(
                                " proto={}",
                                run(l, j, Op::Get(nb("toString")))
                            ));
                            r.push_str(&format!(" top={}", l.js_gettop(j)));
                            l.js_freestate(j);
                            r
                        },
                    );
                }
            }
        }
    }

    // js_defaccessor with a non-coercible receiver: jsrun.c:1028 resolves the
    // setter, then the getter, then js_toobject, so the "not a function"
    // typeerror wins over "cannot convert undefined to object".
    for &tk in &[TK::PrimUndef, TK::PrimNull, TK::PrimNum] {
        for (gk, sk) in [(5u8, 5u8), (2, 5), (5, 2), (2, 3), (0, 0)] {
            diff2(
                &format!("defacc order {tk:?} g={gk} s={sk}"),
                move |l| unsafe {
                    let j = new_state(l, 0);
                    let irc = install_js(l, j);
                    let mk = setup_target(l, j, tk);
                    let r = format!(
                        "install={irc} mk={mk} acc={}",
                        run(l, j, Op::DefAcc(nb("acc"), 0, gk, sk))
                    );
                    l.js_freestate(j);
                    r
                },
            );
        }
    }
}

/// Rows 89/96/117/139: a primitive receiver.  `js_toobject` builds a wrapper AND
/// rewrites the stack slot, and `js_setproperty` / `js_setindex` report the
/// receiver as `transient` (see the note at the top of this file).
#[test]
fn t_transient_receiver() {
    for &tk in &[
        TK::PrimStr,
        TK::PrimNum,
        TK::PrimBool,
        TK::PrimUndef,
        TK::PrimNull,
    ] {
        for flags in [0, JS_STRICT] {
            let t = diff2(&format!("transient {tk:?} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, tk);
                let mut r = format!("install={irc} mk={mk}");
                for nm in ["tp", "0", "length"] {
                    r.push_str(&format!(
                        "\n {nm}: set={} get={} def={} del={}",
                        run(l, j, Op::Set(nb(nm), V::Num(1.0))),
                        run(l, j, Op::Get(nb(nm))),
                        run(l, j, Op::Def(nb(nm), V::Num(2.0), 0)),
                        run(l, j, Op::Del(nb(nm)))
                    ));
                }
                for k in [0, 1, 5] {
                    r.push_str(&format!(
                        "\n idx{k}: set={} get={} has={}",
                        run(l, j, Op::SetIdx(k, V::Num(k as f64))),
                        run(l, j, Op::GetIdx(k)),
                        run(l, j, Op::HasIdx(k))
                    ));
                }
                r.push_str(&format!("\n iter={}", run(l, j, Op::Iter(1))));
                r.push_str(&format!("\n {}", snap_all(l, j)));
                l.js_freestate(j);
                r
            });
            if flags == JS_STRICT && tk == TK::PrimStr {
                assert!(
                    t.contains("cannot create property 'tp' on transient object"),
                    "strict transient write must throw: {t}"
                );
            }
        }
    }

    // The wrapper js_toobject writes back into the stack slot is observable:
    // a second op on the SAME slot sees an object, not a primitive.
    diff2("transient slot rewrite", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for k in 0..3 {
            l.js_pushnumber(j, -1.0); // sentinel
            match k {
                0 => l.js_pushstring(j, cn!("prim")),
                1 => l.js_pushnumber(j, 7.5),
                _ => l.js_pushboolean(j, 1),
            }
            r.push_str(&format!("k={k} pre={} ", from_c(l.js_typeof(j, -1))));
            l.js_pushnumber(j, 1.0);
            l.js_setproperty(j, -2, cn!("w"));
            r.push_str(&format!("post={} ", from_c(l.js_typeof(j, -1))));
            l.js_getproperty(j, -1, cn!("w"));
            r.push_str(&format!("w={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            // and the same for js_setindex
            l.js_pushnumber(j, 2.0);
            l.js_setindex(j, -2, 3);
            l.js_getindex(j, -1, 3);
            r.push_str(&format!("i3={} | ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 2);
        }
        r.push_str(&format!("top={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    });
}

/// Rows 97/129: `Object.preventExtensions` / `seal` / `freeze` and the
/// non-extensible write paths.
#[test]
fn t_non_extensible() {
    for mode in 0..3u8 {
        for &tk in &[TK::Plain, TK::Array, TK::JsFun, TK::UserData, TK::Args] {
            for flags in [0, JS_STRICT] {
                diff2(
                    &format!("nonext mode={mode} {tk:?} flags={flags}"),
                    move |l| unsafe {
                        let j = new_state(l, flags);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, tk);
                        let mut r = format!("install={irc} mk={mk}");
                        // seed a few properties, including flat array slots
                        for k in 0..4 {
                            r.push_str(&format!(
                                " s{k}={}",
                                run(l, j, Op::SetIdx(k, V::Num(k as f64)))
                            ));
                        }
                        r.push_str(&format!(
                            " named={}",
                            run(l, j, Op::Set(nb("kept"), V::Str("K".into())))
                        ));
                        r.push_str(&format!(" pre={}", snap_all(l, j)));
                        r.push_str(&format!(
                            " lock={}",
                            call_amut(l, j, 13 + mode as c_int, 0.0, 0.0)
                        ));
                        r.push_str(&format!(" post={}", snap_all(l, j)));
                        for op in [
                            Op::Set(nb("fresh"), V::Num(1.0)),
                            Op::Set(nb("kept"), V::Num(2.0)),
                            Op::Def(nb("fresh2"), V::Num(3.0), 0),
                            Op::Def(nb("kept"), V::Num(4.0), JS_READONLY),
                            Op::DefAcc(nb("fresh3"), 0, 2, 3),
                            Op::Del(nb("kept")),
                            Op::SetIdx(1, V::Num(9.0)),
                            Op::SetIdx(9, V::Num(9.0)),
                            Op::DelIdx(3),
                            Op::SetLen(2),
                            Op::GetLen,
                        ] {
                            r.push_str(&format!("\n  {op:?}={}", run(l, j, op.clone())));
                        }
                        r.push_str(&format!("\n {}", snap_all(l, j)));
                        r.push_str(&format!("\n top={}", l.js_gettop(j)));
                        l.js_freestate(j);
                        r
                    },
                );
            }
        }
    }
}

/// Row 116/117: `js_getlength` / `js_setlength` and the index API with negative
/// and out-of-range indices.
#[test]
fn t_length_and_index_api() {
    let mut rng = Rng::new(0x1E17_4E30);
    let mut lens: Vec<c_int> = vec![0, 1, 2, 5, 8, 9, 16, 17, 33, 60, -1, -5];
    for _ in 0..24 {
        lens.push(rng.range(-4, 60) as c_int);
    }
    for &tk in CLASS_TK {
        for flags in [0, JS_STRICT] {
            let lens = lens.clone();
            diff2(&format!("length {tk:?} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, tk);
                let mut r = format!("install={irc} mk={mk}");
                // dense append so arrays start out flat
                for k in 0..6 {
                    r.push_str(&format!(
                        " a{k}={}",
                        run(l, j, Op::SetIdx(k, V::Num(k as f64 * 10.0)))
                    ));
                }
                r.push_str(&format!(" {}", snap_all(l, j)));
                for len in &lens {
                    for neg in [false, true] {
                        NEG.with(|c| c.set(neg));
                        r.push_str(&format!(
                            "\n len={len} neg={neg}: {} {}",
                            run(l, j, Op::SetLen(*len)),
                            run(l, j, Op::GetLen)
                        ));
                        r.push_str(&format!(" {}", csnap(l, j)));
                    }
                }
                NEG.with(|c| c.set(false));
                // index API with negative and far-out indices
                for k in [-1, -2, -100, 0, 1, 7, 1000, i32::MAX, i32::MIN] {
                    r.push_str(&format!(
                        "\n k={k}: has={} get={} set={} del={}",
                        run(l, j, Op::HasIdx(k)),
                        run(l, j, Op::GetIdx(k)),
                        run(l, j, Op::SetIdx(k, V::Str("N".into()))),
                        run(l, j, Op::DelIdx(k))
                    ));
                }
                r.push_str(&format!("\n {}", snap_all(l, j)));
                r.push_str(&format!("\n top={}", l.js_gettop(j)));
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Row 125: invalid array lengths.
#[test]
fn t_array_length_errors() {
    let bad = [
        1.5f64,
        -1.0,
        -0.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        (1u32 << 26) as f64,
        (1u32 << 26) as f64 + 1.0,
        1e9,
        1e21,
        4294967296.0,
        2147483648.0,
        -0.0,
        0.0,
        1.0,
    ];
    for v in bad {
        for simple in [true, false] {
            for flags in [0, JS_STRICT] {
                diff2(
                    &format!("badlen {v} simple={simple} flags={flags}"),
                    move |l| unsafe {
                        let j = new_state(l, flags);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, TK::Array);
                        let mut r = format!("install={irc} mk={mk}");
                        for k in 0..4 {
                            run(l, j, Op::SetIdx(k, V::Num(k as f64)));
                        }
                        if !simple {
                            // force the hashed representation
                            r.push_str(&format!(
                                " unflat={}",
                                run(l, j, Op::Def(nb("x"), V::Num(0.0), 0))
                            ));
                        }
                        r.push_str(&format!(
                            " set={}",
                            run(l, j, Op::Set(nb("length"), V::Num(v)))
                        ));
                        r.push_str(&format!(" {}", csnap(l, j)));
                        // and through js_setindex, which raises "array too large"
                        for k in [(1i32 << 26) - 1, 1 << 26, (1 << 26) + 1, i32::MAX - 1] {
                            r.push_str(&format!(
                                " k={k}:{}",
                                run(l, j, Op::SetIdx(k, V::Num(1.0)))
                            ));
                        }
                        r.push_str(&format!(" {}", csnap(l, j)));
                        r.push_str(&format!(" top={}", l.js_gettop(j)));
                        l.js_freestate(j);
                        r
                    },
                );
            }
        }
    }
}

/* ====================================================================== */
/*  Rows 118-132: the ARRAY REPRESENTATION axis                            */
/* ====================================================================== */

/// The targeted flat/hashed transitions, one at a time.
#[test]
fn t_array_flat_transitions() {
    // (label, the steps to run)
    let cases: Vec<(&str, Vec<Op>)> = vec![
        // row 118: dense append keeps the array simple, capacity doubles
        (
            "dense append 0..40",
            (0..40).map(|k| Op::SetIdx(k, V::Num(k as f64))).collect(),
        ),
        // row 119: in-place overwrite
        (
            "overwrite in range",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::SetIdx(2, V::Num(2.0)),
                Op::SetIdx(1, V::Str("over".into())),
                Op::SetIdx(0, V::Obj),
            ],
        ),
        // row 118/119 boundary: k == flat_length exactly
        (
            "write at flat_length",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::SetIdx(2, V::Num(2.0)),
                Op::SetIdx(3, V::Num(3.0)),
                Op::SetIdx(3, V::Num(33.0)),
                Op::SetIdx(4, V::Num(4.0)),
            ],
        ),
        // row 120: a sparse write unflattens
        (
            "sparse write",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::SetIdx(2, V::Num(2.0)),
                Op::SetIdx(9, V::Num(9.0)),
                Op::SetIdx(3, V::Num(3.0)),
                Op::SetIdx(20, V::Num(20.0)),
            ],
        ),
        // rows 121/122: length shrink and grow on a simple array
        (
            "length shrink/grow simple",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::SetIdx(2, V::Num(2.0)),
                Op::SetIdx(3, V::Num(3.0)),
                Op::SetLen(2),
                Op::SetLen(6),
                Op::SetIdx(2, V::Num(22.0)),
                Op::SetLen(0),
                Op::SetIdx(0, V::Num(100.0)),
            ],
        ),
        // rows 123/124: shrink an unflattened array through both jsV_resizearray
        // paths (u.a.length > count*2 vs not)
        (
            "resizearray sparse path",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(30, V::Num(30.0)),
                Op::SetLen(10),
                Op::SetLen(1),
                Op::SetLen(0),
            ],
        ),
        (
            "resizearray dense path",
            {
                let mut v: Vec<Op> = (0..8).map(|k| Op::SetIdx(k, V::Num(k as f64))).collect();
                v.push(Op::Def(nb("nonidx"), V::Num(0.0), 0)); // unflatten
                v.push(Op::SetLen(4));
                v.push(Op::SetLen(1));
                v.push(Op::SetLen(0));
                v
            },
        ),
        // row 126: deleting the last flat element keeps the array flat
        (
            "delete last flat",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::SetIdx(2, V::Num(2.0)),
                Op::DelIdx(2),
                Op::DelIdx(1),
                Op::DelIdx(0),
                Op::DelIdx(0),
                Op::SetIdx(0, V::Num(9.0)),
            ],
        ),
        // row 127: deleting a middle element unflattens the whole array
        (
            "delete middle",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::SetIdx(2, V::Num(2.0)),
                Op::SetIdx(3, V::Num(3.0)),
                Op::DelIdx(1),
                Op::SetIdx(1, V::Num(11.0)),
                Op::SetIdx(4, V::Num(4.0)),
            ],
        ),
        // row 128: delete "length"
        (
            "delete length",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::Del(nb("length")),
                Op::GetLen,
                Op::SetLen(3),
            ],
        ),
        // row 129: defproperty / defaccessor unflatten unconditionally
        (
            "defproperty unflattens",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::Def(nb("5"), V::Num(5.0), JS_DONTENUM),
                Op::SetIdx(2, V::Num(2.0)),
            ],
        ),
        (
            "defaccessor unflattens",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::DefAcc(nb("1"), 0, 2, 3),
                Op::GetIdx(1),
                Op::SetIdx(1, V::Num(111.0)),
            ],
        ),
        // row 130: a non-index name keeps the flat part alive
        (
            "non-index name",
            vec![
                Op::SetIdx(0, V::Num(0.0)),
                Op::SetIdx(1, V::Num(1.0)),
                Op::Set(nb("foo"), V::Str("bar".into())),
                Op::SetIdx(2, V::Num(2.0)),
                Op::Get(nb("foo")),
            ],
        ),
        // row 131: index-like names js_isarrayindex accepts vs rejects
        (
            "index-like names",
            vec![
                Op::Set(nb("0"), V::Num(0.0)),
                Op::Set(nb("1"), V::Num(1.0)),
                Op::Set(nb(""), V::Num(-1.0)),
                Op::Set(nb("01"), V::Num(-1.0)),
                Op::Set(nb("1.5"), V::Num(-1.0)),
                Op::Set(nb("-1"), V::Num(-1.0)),
                Op::Set(nb(" 1"), V::Num(-1.0)),
                Op::Set(nb("214748364"), V::Num(-1.0)),
                Op::Set(nb("214748365"), V::Num(-1.0)),
                Op::Set(nb("2147483647"), V::Num(-1.0)),
                Op::Set(nb("2"), V::Num(2.0)),
            ],
        ),
    ];
    for (label, steps) in cases {
        for flags in [0, JS_STRICT] {
            let steps = steps.clone();
            diff2(&format!("flat {label} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, TK::Array);
                let mut r = format!("install={irc} mk={mk}");
                for (i, op) in steps.iter().enumerate() {
                    r.push_str(&format!("\n {i} {op:?} = {}", run(l, j, op.clone())));
                    r.push_str(&format!("\n   {}", snap_all(l, j)));
                }
                l.js_gc(j, 0);
                r.push_str(&format!("\n after-gc {}", snap_all(l, j)));
                r.push_str(&format!("\n top={}", l.js_gettop(j)));
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Hundreds of randomised array mutation sequences.  After EVERY step the whole
/// observable array state is compared: `js_getlength`, every element through
/// `js_getindex` / `js_hasindex`, the `in` operator, `Object.keys` order, the
/// for-in order, `JSON.stringify` and `js_torepr`.
#[test]
fn t_array_representation_random() {
    let mut rng = Rng::new(0xA55A_1234_5678_9ABC);
    const NSEQ: usize = 900;
    const NSTEP: usize = 17;
    let mut nthrow = 0usize;
    let mut marks: std::collections::BTreeSet<&str> = Default::default();
    const MARKERS: &[&str] = &[
        "is read-only",
        "non-extensible",
        "invalid array length",
        "non-configurable",
        "it1=[0,1,10", // AA-tree (string) order -> the array was unflattened
        "it1=[0,1,2,", // flat order -> the array was still simple
        "ext=false",   // seal / freeze / preventExtensions was reached
        "[getter]",
        "[setter",
    ];
    for seq in 0..NSEQ {
        // pre-roll the sequence so both libraries see identical inputs
        let mut steps: Vec<AStep> = vec![];
        for _ in 0..NSTEP {
            steps.push(rand_astep(&mut rng));
        }
        let flags = if seq % 2 == 0 { 0 } else { JS_STRICT };
        let neg = seq % 3 == 0;
        let t = diff2(&format!("arraywalk seq={seq} flags={flags}"), move |l| unsafe {
            let j = new_state(l, flags);
            let irc = install_js(l, j);
            let mk = setup_target(l, j, TK::Array);
            NEG.with(|c| c.set(neg));
            let mut r = format!("install={irc} mk={mk}");
            for (i, st) in steps.iter().enumerate() {
                let d = match st {
                    AStep::Api(op) => format!("{op:?} = {}", run(l, j, op.clone())),
                    AStep::Js(k, a, b) => call_amut(l, j, *k, *a, *b),
                };
                r.push_str(&format!("\n {i} {d}\n   {}", snap_all(l, j)));
            }
            NEG.with(|c| c.set(false));
            l.js_gc(j, 0);
            r.push_str(&format!("\n after-gc {}", snap_all(l, j)));
            r.push_str(&format!("\n top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
        nthrow += t.matches("[rc=1 ").count();
        for m in MARKERS {
            if t.contains(m) {
                marks.insert(m);
            }
        }
    }
    // guard against the walk silently degenerating into a no-op
    assert!(nthrow > 100, "only {nthrow} throwing steps in the array walk");
    let missing: Vec<&&str> = MARKERS.iter().filter(|m| !marks.contains(*m)).collect();
    assert!(
        missing.is_empty(),
        "the array walk never reached: {missing:?}"
    );
}

#[derive(Clone, Debug)]
enum AStep {
    Api(Op),
    Js(c_int, f64, f64),
}

fn rand_astep(rng: &mut Rng) -> AStep {
    let vals = [
        V::Num(1.0),
        V::Num(-0.0),
        V::Num(f64::NAN),
        V::Str("s".into()),
        V::Str("a longer string value here".into()),
        V::Obj,
        V::Arr,
        V::Undef,
        V::Null,
        V::Bool(1),
    ];
    let v = vals[rng.below(vals.len() as u32) as usize].clone();
    match rng.below(20) {
        // dense append / write inside the flat part / write at a gap
        0 | 1 | 2 => AStep::Api(Op::SetIdx(rng.below(8) as c_int, v)),
        3 => AStep::Api(Op::SetIdx(rng.below(24) as c_int, v)),
        4 => AStep::Api(Op::DelIdx(rng.below(10) as c_int)),
        5 => AStep::Api(Op::SetLen(rng.range(-3, 22) as c_int)),
        6 => AStep::Api(Op::Set(
            nb(&format!("{}", rng.below(20))),
            v,
        )),
        7 => AStep::Api(Op::Set(nb(&format!("n{}", rng.below(4))), v)),
        8 => AStep::Api(Op::Def(
            nb(&format!("{}", rng.below(12))),
            v,
            rng.below(8) as c_int,
        )),
        9 => AStep::Api(Op::DefAcc(
            nb(&format!("{}", rng.below(8))),
            rng.below(8) as c_int,
            2,
            3,
        )),
        10 => AStep::Api(Op::Del(nb(&format!("{}", rng.below(12))))),
        11 => AStep::Api(Op::GetLen),
        12 => AStep::Api(Op::Iter(rng.below(2) as c_int)),
        // JS-level mutation
        _ => {
            // seal/freeze/preventExtensions are rare so the rest of the sequence
            // stays interesting
            let k = match rng.below(24) {
                0 => 13,
                1 => 14,
                2 => 15,
                k => {
                    let pool = [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 17, 18, 19, 20, 21, 22,
                        23, 24,
                    ];
                    pool[(k as usize) % pool.len()]
                }
            };
            AStep::Js(
                k,
                rng.below(14) as f64,
                rng.below(6) as f64,
            )
        }
    }
}

/// Row 132 (first half): a reachable simple array is scanned by
/// `jsG_scanobject` (`u.a.array` walked for JS_TMEMSTR / JS_TOBJECT slots) and
/// its flat part is freed by `jsG_freeobject` only while `u.a.simple`.
#[test]
fn t_array_gc_simple() {
    for unflat in [false, true] {
        for n in [0usize, 1, 8, 9, 40] {
            diff2(&format!("arraygc n={n} unflat={unflat}"), move |l| unsafe {
                let j = new_state(l, 0);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, TK::Array);
                let mut r = format!("install={irc} mk={mk}");
                for k in 0..n {
                    // heap strings and objects, so the scan has real work
                    let v = if k % 3 == 0 {
                        V::Str(format!("a heap string value number {k}"))
                    } else if k % 3 == 1 {
                        V::Obj
                    } else {
                        V::Num(k as f64)
                    };
                    run(l, j, Op::SetIdx(k as c_int, v));
                }
                if unflat {
                    run(l, j, Op::Def(nb("tail"), V::Str("t".into()), 0));
                }
                r.push_str(&format!(" pre={}", snap_all(l, j)));
                for _ in 0..3 {
                    l.js_gc(j, 1);
                }
                r.push_str(&format!(" post={}", snap_all(l, j)));
                // drop the only root and collect: jsG_freeobject frees u.a.array
                l.js_delglobal(j, N_T);
                l.js_gc(j, 1);
                l.js_gc(j, 1);
                r.push_str(&format!(" top={}", l.js_gettop(j)));
                l.js_freestate(j);
                r
            });
        }
    }
}

/* ====================================================================== */
/*  Rows 133-141: iterators                                                */
/* ====================================================================== */

#[test]
fn t_iterators() {
    // Each case: a JS setup snippet run after T is created, then both iterator
    // modes plus the JS-level for-in for comparison.
    let setups: &[&str] = &[
        "",
        "T.a = 1; T.b = 2; T.c = 3",
        "T.z = 1; T.a = 2; T.m = 3; T.b = 4; T.y = 5",
        // shadowed prototype-chain properties (row 134)
        "var p = {sh: 'proto', only: 'p'}; T.__proto__ = p",
        "for (var i = 0; i < 40; ++i) T['k' + i] = i",
        // numeric-looking names in the tree
        "T[1] = 'one'; T[10] = 'ten'; T[9] = 'nine'; T[2] = 'two'",
        "T.a = 1; delete T.a; T.b = 2",
        // an ENUMERABLE prototype property shadowed by an own property: the
        // prototype's name is listed and the own one skipped (itflatten's
        // jsV_getenumproperty check, jsproperty.c:257)
        "var p = {sh: 'proto'}; T.__proto__ = p; T.sh = 'own'; T.other = 1",
        // a DONTENUM prototype property does NOT shadow, so the own one shows up
        "var p = {}; Object.defineProperty(p,'sh',{value:1,enumerable:false}); T.__proto__ = p; T.sh = 'own'",
        // two prototype levels, the middle one shadowing
        "var q = {d:'q'}; var p = {d:'p'}; p.__proto__ = q; T.__proto__ = p; T.t = 1",
        // getters on the prototype chain
        "var p = {}; Object.defineProperty(p,'g',{get:function(){return 1},enumerable:true}); T.__proto__ = p",
    ];
    for &tk in &[
        TK::Plain,
        TK::PlainProtoNull,
        TK::Array,
        TK::StrShort,
        TK::StrUtf,
        TK::JsFun,
        TK::CFun,
        TK::Args,
        TK::UserData,
        TK::UserDataX,
        TK::Err,
        TK::Regexp,
        TK::IterObj,
        TK::PrimStr,
        TK::Global,
    ] {
        for (si, setup) in setups.iter().enumerate() {
            for flags in [0, JS_STRICT] {
                let setup = setup.to_string();
                diff2(
                    &format!("iter {tk:?} setup={si} flags={flags}"),
                    move |l| unsafe {
                        let j = new_state(l, flags);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, tk);
                        let mut r = format!("install={irc} mk={mk}");
                        if !setup.is_empty() {
                            let cs = cstr(&setup);
                            r.push_str(&format!(" setup={}", l.js_dostring(j, cs.as_ptr())));
                        }
                        for own in [0, 1] {
                            r.push_str(&format!("\n own={own} {}", run(l, j, Op::Iter(own))));
                        }
                        // DONTENUM properties are skipped in both modes (row 135)
                        r.push_str(&format!(
                            "\n hide={}",
                            run(l, j, Op::Def(nb("hidden"), V::Num(1.0), JS_DONTENUM))
                        ));
                        r.push_str(&format!(
                            " visible={}",
                            run(l, j, Op::Def(nb("visible"), V::Num(2.0), 0))
                        ));
                        for own in [0, 1] {
                            r.push_str(&format!("\n own={own} {}", run(l, j, Op::Iter(own))));
                        }
                        // mutation during iteration (row 140)
                        r.push_str(&format!(
                            "\n during={}",
                            run(l, j, Op::IterDelDuring(nb("visible")))
                        ));
                        r.push_str(&format!("\n scratch={}", run(l, j, Op::IterScratch)));
                        r.push_str(&format!("\n {}", snap_all(l, j)));
                        r.push_str(&format!("\n top={}", l.js_gettop(j)));
                        l.js_freestate(j);
                        r
                    },
                );
            }
        }
    }
}

/// Row 136 vs 137: the same logical array in BOTH representations must enumerate
/// differently -- flat gives "0".."n-1" first, hashed gives AA-tree order so
/// "10" comes before "9".
#[test]
fn t_iterator_array_representations() {
    for n in [0usize, 1, 2, 3, 9, 10, 11, 12, 20, 33] {
        for unflat in [false, true] {
            for extra in [false, true] {
                let t = diff2(
                    &format!("iterarr n={n} unflat={unflat} extra={extra}"),
                    move |l| unsafe {
                        let j = new_state(l, 0);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, TK::Array);
                        let mut r = format!("install={irc} mk={mk}");
                        for k in 0..n {
                            run(l, j, Op::SetIdx(k as c_int, V::Num(k as f64)));
                        }
                        if extra {
                            run(l, j, Op::Set(nb("zz"), V::Str("Z".into())));
                            run(l, j, Op::Set(nb("aa"), V::Str("A".into())));
                        }
                        if unflat {
                            // a sparse write past flat_length unflattens
                            run(l, j, Op::SetIdx(n as c_int + 4, V::Num(-1.0)));
                        }
                        for own in [0, 1] {
                            r.push_str(&format!(" own{own}={}", run(l, j, Op::Iter(own))));
                        }
                        r.push_str(&format!(" {}", snap_all(l, j)));
                        l.js_freestate(j);
                        r
                    },
                );
                // row 136 vs 137: the flat part yields "0".."n-1" in numeric
                // order, the AA-tree yields string order ("10" right after "1")
                if n >= 11 {
                    if unflat {
                        assert!(
                            t.contains("[0,1,10,"),
                            "hashed arrays must enumerate in string order: {t}"
                        );
                        assert!(
                            !t.contains("it1=[0,1,2,"),
                            "hashed array unexpectedly enumerated numerically: {t}"
                        );
                    } else {
                        assert!(
                            t.contains("it1=[0,1,2,") && t.contains("9,10"),
                            "flat arrays must enumerate 0..n-1 first: {t}"
                        );
                    }
                }
            }
        }
    }
    // sparse arrays built purely through the tree
    for spec in [
        "T[0]=0; T[5]=5; T[10]=10",
        "T[100]=1; T[2]=2",
        "T.length=20; T[19]=19",
        "T[1]=1; delete T[1]",
    ] {
        diff2(&format!("itersparse {spec}"), move |l| unsafe {
            let j = new_state(l, 0);
            let irc = install_js(l, j);
            let mk = setup_target(l, j, TK::Array);
            let cs = cstr(spec);
            let rc = l.js_dostring(j, cs.as_ptr());
            let mut r = format!("install={irc} mk={mk} rc={rc}");
            for own in [0, 1] {
                r.push_str(&format!(" own{own}={}", run(l, j, Op::Iter(own))));
            }
            r.push_str(&format!(" {}", snap_all(l, j)));
            l.js_freestate(j);
            r
        });
    }
}

/// Row 141: `js_nextiterator` on a value whose class is not JS_CITERATOR.
#[test]
fn t_nextiterator_not_an_iterator() {
    for &tk in ALL_TK {
        for flags in [0, JS_STRICT] {
            diff2(&format!("notiter {tk:?} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, tk);
                let r = format!(
                    "install={irc} mk={mk} next={}",
                    run(l, j, Op::NextNonIterator)
                );
                l.js_freestate(j);
                r
            });
        }
    }
    // and a live iterator surviving a collection (u.iter.target is marked)
    diff2("iterator survives gc", |l| unsafe {
        let j = new_state(l, 0);
        let irc = install_js(l, j);
        let mut r = format!("install={irc}");
        l.js_newobject(j);
        for nm in ["a", "b", "c", "d"] {
            l.js_pushstring(j, cn!("v"));
            let cs = cstr(nm);
            l.js_setproperty(j, -2, cs.as_ptr());
        }
        l.js_pushiterator(j, -1, 1);
        l.js_replace(j, -2); // only the iterator keeps the target alive
        let first = from_c(l.js_nextiterator(j, -1));
        for _ in 0..3 {
            l.js_gc(j, 1);
        }
        r.push_str(&format!(" first={first} rest=[{}]", drain_iter(l, j, 100)));
        l.js_gc(j, 1);
        r.push_str(&format!(" top={}", l.js_gettop(j)));
        l.js_pop(j, 1);
        l.js_gc(j, 1);
        l.js_freestate(j);
        r
    });
}

/// Randomised objects, compared name-for-name between `js_pushiterator` and the
/// JS-level `for (k in o)`.
#[test]
fn t_iterator_random_vs_forin() {
    let mut rng = Rng::new(0x1723_4567_89AB_CDEF);
    for round in 0..320 {
        let n = 1 + rng.below(24) as usize;
        let mut prog = String::new();
        let mut protoprog = String::new();
        for i in 0..n {
            let nm = match rng.below(6) {
                0 => format!("{}", rng.below(30)),
                1 => format!("k{}", rng.below(10)),
                2 => format!("{}", rng.below(3)),
                3 => format!("z{}", rng.below(40)),
                4 => "shadowed".to_string(),
                _ => format!("p{i}"),
            };
            match rng.below(8) {
                0 => prog.push_str(&format!("delete T['{nm}'];")),
                1 => protoprog.push_str(&format!("P['{nm}']={i};")),
                2 => prog.push_str(&format!(
                    "Object.defineProperty(T,'{nm}',{{value:{i},enumerable:false}});"
                )),
                3 => prog.push_str(&format!(
                    "Object.defineProperty(T,'{nm}',{{get:function(){{return {i}}},enumerable:true}});"
                )),
                _ => prog.push_str(&format!("T['{nm}']={i};")),
            }
        }
        let base = match rng.below(4) {
            0 => TK::Plain,
            1 => TK::Array,
            2 => TK::JsFun,
            _ => TK::Plain,
        };
        let flags = if round % 2 == 0 { 0 } else { JS_STRICT };
        diff2(&format!("iterrand {round}"), move |l| unsafe {
            let j = new_state(l, flags);
            let irc = install_js(l, j);
            let mk = setup_target(l, j, base);
            let src = format!("var P = {{}}; {protoprog} T.__proto__ = P; {prog} 0");
            let cs = cstr(&src);
            let rc = l.js_dostring(j, cs.as_ptr());
            let mut r = format!("install={irc} mk={mk} rc={rc}");
            for own in [0, 1] {
                r.push_str(&format!(" own{own}={}", run(l, j, Op::Iter(own))));
            }
            r.push_str(&format!(" {}", snap_all(l, j)));
            l.js_freestate(j);
            r
        });
    }
}

/// A randomised walk over the WHOLE property API on every receiver class, with
/// the full observable state compared after every single step.  This is the
/// broadest check: it mixes attribute definitions, accessors, deletes, length
/// changes, sealing and iteration on objects of every class.
#[test]
fn t_property_random_walk() {
    let mut rng = Rng::new(0xC0DE_1234_5678_F00D);
    const NSEQ: usize = 700;
    const NSTEP: usize = 13;
    for seq in 0..NSEQ {
        let tk = ALL_TK[seq % ALL_TK.len()];
        let flags = if (seq / ALL_TK.len()) % 2 == 0 { 0 } else { JS_STRICT };
        let neg = seq % 5 == 0;
        let steps: Vec<AStep> = (0..NSTEP).map(|_| rand_pstep(&mut rng)).collect();
        diff2(
            &format!("propwalk seq={seq} {tk:?} flags={flags}"),
            move |l| unsafe {
                let j = new_state(l, flags);
                let irc = install_js(l, j);
                let mk = setup_target(l, j, tk);
                NEG.with(|c| c.set(neg));
                let mut r = format!("install={irc} mk={mk}");
                for (i, st) in steps.iter().enumerate() {
                    let d = match st {
                        AStep::Api(op) => format!("{op:?} = {}", run(l, j, op.clone())),
                        AStep::Js(k, a, b) => call_amut(l, j, *k, *a, *b),
                    };
                    r.push_str(&format!("\n {i} {d}\n   {}", snap_all(l, j)));
                }
                NEG.with(|c| c.set(false));
                l.js_gc(j, 0);
                r.push_str(&format!("\n after-gc {}", snap_all(l, j)));
                r.push_str(&format!("\n top={}", l.js_gettop(j)));
                l.js_freestate(j);
                r
            },
        );
    }
}

fn rand_pstep(rng: &mut Rng) -> AStep {
    let vals = [
        V::Num(0.0),
        V::Num(-0.0),
        V::Num(1.5),
        V::Num(f64::NAN),
        V::Num(f64::INFINITY),
        V::Str(String::new()),
        V::Str("v".into()),
        V::Str("a string that is longer than fifteen bytes".into()),
        V::Obj,
        V::Arr,
        V::Fun,
        V::Undef,
        V::Null,
        V::Bool(0),
        V::Bool(1),
    ];
    let v = vals[rng.below(vals.len() as u32) as usize].clone();
    let names: [&str; 12] = [
        "px", "py", "0", "1", "5", "length", "lastIndex", "callee", "source",
        "toString", "", "global",
    ];
    let n = nb(names[rng.below(names.len() as u32) as usize]);
    match rng.below(18) {
        0 => AStep::Api(Op::Has(n)),
        1 => AStep::Api(Op::Get(n)),
        2 | 3 => AStep::Api(Op::Set(n, v)),
        4 | 5 => AStep::Api(Op::Def(n, v, rng.below(8) as c_int)),
        6 => AStep::Api(Op::DefAcc(
            n,
            rng.below(8) as c_int,
            rng.below(8) as u8,
            rng.below(8) as u8,
        )),
        7 => AStep::Api(Op::Del(n)),
        8 => AStep::Api(Op::GetLen),
        9 => AStep::Api(Op::SetLen(rng.range(-2, 24) as c_int)),
        10 => AStep::Api(Op::HasIdx(rng.range(-3, 20) as c_int)),
        11 => AStep::Api(Op::GetIdx(rng.range(-3, 20) as c_int)),
        12 => AStep::Api(Op::SetIdx(rng.range(-3, 20) as c_int, v)),
        13 => AStep::Api(Op::DelIdx(rng.range(-3, 20) as c_int)),
        14 => AStep::Api(Op::Iter(rng.below(2) as c_int)),
        15 => AStep::Api(Op::IterDelDuring(n)),
        16 => AStep::Api(Op::IterScratch),
        _ => {
            let k = match rng.below(30) {
                0 => 13,
                1 => 14,
                2 => 15,
                k => {
                    let pool = [
                        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 17, 18, 19, 20, 21, 22,
                        23, 24,
                    ];
                    pool[(k as usize) % pool.len()]
                }
            };
            AStep::Js(k, rng.below(10) as f64, rng.below(5) as f64)
        }
    }
}

/* ====================================================================== */
/*  Rows 86/88/91/95: prototype-chain lookup and creation                  */
/* ====================================================================== */

#[test]
fn t_prototype_chain() {
    let progs = [
        "var P = {own:'p', sh:'p'}; T.__proto__ = P; T.sh = 't'",
        "var P = {}; var Q = {deep:'q'}; P.__proto__ = Q; T.__proto__ = P",
        "var P = {}; Object.defineProperty(P,'ro',{value:'r',writable:false}); T.__proto__ = P",
        "var P = {}; Object.defineProperty(P,'ga',{get:function(){return 'G'}}); T.__proto__ = P",
        "var P = {}; Object.defineProperty(P,'sa',{set:function(v){this.seen=v}}); T.__proto__ = P",
        "var P = {}; Object.defineProperty(P,'both',{get:function(){return this.b2},set:function(v){this.b2=v}}); T.__proto__ = P",
        "T.__proto__ = null",
    ];
    for (pi, prog) in progs.iter().enumerate() {
        for &tk in &[TK::Plain, TK::Array, TK::JsFun, TK::UserData] {
            for flags in [0, JS_STRICT] {
                let prog = prog.to_string();
                diff2(
                    &format!("proto {pi} {tk:?} flags={flags}"),
                    move |l| unsafe {
                        let j = new_state(l, flags);
                        let irc = install_js(l, j);
                        let mk = setup_target(l, j, tk);
                        let cs = cstr(&format!("{prog}; 0"));
                        let rc = l.js_dostring(j, cs.as_ptr());
                        let mut r = format!("install={irc} mk={mk} rc={rc}");
                        for nm in ["own", "sh", "deep", "ro", "ga", "sa", "both", "missing"] {
                            r.push_str(&format!("\n {nm}:"));
                            r.push_str(&format!(" has={}", run(l, j, Op::Has(nb(nm)))));
                            r.push_str(&format!(" get={}", run(l, j, Op::Get(nb(nm)))));
                            r.push_str(&format!(
                                " set={}",
                                run(l, j, Op::Set(nb(nm), V::Str("W".into())))
                            ));
                            r.push_str(&format!(" get2={}", run(l, j, Op::Get(nb(nm)))));
                            r.push_str(&format!(" del={}", run(l, j, Op::Del(nb(nm)))));
                            r.push_str(&format!(" get3={}", run(l, j, Op::Get(nb(nm)))));
                        }
                        r.push_str(&format!("\n {}", snap_all(l, j)));
                        r.push_str(&format!("\n top={}", l.js_gettop(j)));
                        l.js_freestate(j);
                        r
                    },
                );
            }
        }
    }
}
