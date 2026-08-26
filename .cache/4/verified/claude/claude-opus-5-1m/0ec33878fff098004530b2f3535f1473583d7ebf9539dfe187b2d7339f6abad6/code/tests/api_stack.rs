//! Differential coverage for CONFIGS.md rows 41-70 (the stack API and every
//! value push) and rows 142-182 (every conversion, predicate and comparison).
//!
//! Everything goes through the two `libmujs.so` exports; nothing calls into the
//! Rust crate directly.
//!
//! # What is deliberately NOT tested (undefined behaviour in the C)
//!
//! * `js_pop(J, n)` with `n < 0` (jsrun.c:403) raises TOP *above* the live part
//!   of the value stack instead of lowering it; the slots above the old TOP were
//!   never initialised, so every later `stackidx` read reads uninitialised
//!   memory (and `n <= -4096` walks off the end of the 4096-entry array
//!   entirely). Every `js_pop` here gets `n >= 0`.
//! * `js_dup` / `js_dup2` / `js_rot2` / `js_rot3` / `js_rot4` / `js_rot2pop1` /
//!   `js_rot3pop2` (jsrun.c:442-497) and `js_rot(J, n)` with `n > js_gettop()`
//!   (jsrun.c:498) have no bounds check at all. At top level BOT is 0, so a
//!   frame holding fewer values than the operation consumes indexes
//!   `STACK[-1]`, `STACK[-2]`, ... i.e. before the start of the malloc'd array.
//!   Every such call below is guarded by a `js_gettop()` check.
//! * `stackidx` (jsrun.c:220) computes `TOP + idx` for `idx < 0`; `idx` near
//!   `INT_MIN` overflows that signed addition. The out-of-range probes here stay
//!   inside +-100000, which is representable.
//! * `js_pushlstring(J, v, n)` with `n < 0` (jsrun.c:163) passes the
//!   `n > JS_STRLIMIT` check and then runs `while (n--) *s++ = *v++`, writing
//!   ~2^31 bytes past the 16-byte slot. `n == JS_STRLIMIT` exactly would copy
//!   256MB out of the caller's buffer, so only `n > JS_STRLIMIT` (which throws
//!   before any copying) and small `n` are used.
//! * `js_isuserdata(J, idx, NULL)` (jsrun.c:266) reaches `strcmp(tag, ...)` once
//!   it knows the value is a JS_CUSERDATA, so a NULL tag is only well defined
//!   when the value at `idx` is not userdata. Both arms are exercised, the NULL
//!   tag only over values that are provably not userdata.
//! * `js_currentfunctiondata` (jsrun.c:211) reads
//!   `STACK[BOT-1].u.object->u.c.data` without checking the callee's class.
//!   `jsV_newobject` memsets the whole object (jsproperty.c:167) so this reads
//!   zeroed, not uninitialised, memory - but there is no public way to reach it
//!   with a JS_CFUNCTION at BOT-1 anyway, because native code only ever runs
//!   with its own JS_CCFUNCTION there (jsrun.c:1322).
//! * `js_pushliteral` (jsrun.c:180) records the caller's pointer with no copy,
//!   so every literal used here has `'static` storage.
//! * `js_tryrepr` (jsrepr.c:275) has no `js_ptry` pre-check, so at
//!   `trytop == JS_TRYLIMIT` its `js_savetry` throws into the enclosing frame
//!   rather than returning the default; tests/api_state.rs covers that, and the
//!   try-limit test here only drives the four `js_ptry`-guarded conversions.

mod common;
use common::*;
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CString};

/* ------------------------------------------------------------- constants */

const JS_STRLIMIT: c_int = 1 << 28;
const JS_STACKSIZE: c_int = 4096;

const MARK: *const c_char = b"#\0".as_ptr() as *const c_char;
const LENGTH: *const c_char = b"length\0".as_ptr() as *const c_char;

const UD_A: *const c_char = b"UDA\0".as_ptr() as *const c_char;
const UD_B: *const c_char = b"UDB\0".as_ptr() as *const c_char;
const UD_MISS: *const c_char = b"NOPE\0".as_ptr() as *const c_char;

const LIT_EMPTY: *const c_char = b"\0".as_ptr() as *const c_char;
const LIT_1: *const c_char = b"L\0".as_ptr() as *const c_char;
const LIT_15: *const c_char = b"L23456789012345\0".as_ptr() as *const c_char;
const LIT_16: *const c_char = b"L234567890123456\0".as_ptr() as *const c_char;
const LIT_NUM: *const c_char = b"  12  \0".as_ptr() as *const c_char;

const S15: *const c_char = b"S23456789012345\0".as_ptr() as *const c_char;
const S16: *const c_char = b"S234567890123456\0".as_ptr() as *const c_char;

const RE_PAT: *const c_char = b"a(b)c\0".as_ptr() as *const c_char;

const PNAME: *const c_char = b"probe\0".as_ptr() as *const c_char;
const CTOR: *const c_char = b"Ctor\0".as_ptr() as *const c_char;
const NOOP: *const c_char = b"noop\0".as_ptr() as *const c_char;

const REG_OP: *const c_char = b"$op\0".as_ptr() as *const c_char;
const REG_CONV: *const c_char = b"$conv\0".as_ptr() as *const c_char;
const REG_CMP: *const c_char = b"$cmp\0".as_ptr() as *const c_char;
const REG_FRAME: *const c_char = b"$frame\0".as_ptr() as *const c_char;
const REG_TRY: *const c_char = b"$try\0".as_ptr() as *const c_char;

/// Handed to `js_newuserdata` / `js_newcfunctionx` in BOTH libraries, so the
/// pointer that comes back out of `js_touserdata` / `js_currentfunctiondata`
/// can be compared for equality across them (they share this process).
static mut UD_DATA: [c_int; 4] = [0x5a5a, 0, 0, 0];

fn ud_data() -> *mut c_void {
    std::ptr::addr_of_mut!(UD_DATA) as *mut c_void
}

/* --------------------------------------------------------- diff driver */

fn one_side(l: &Lib, f: &impl Fn(&Lib) -> String) -> String {
    out_clear();
    set_cur(l);
    let r = f(l);
    format!("{r}\n--out--\n{}", out_take())
}

/// Run `f` against the C library and then the Rust library and assert the two
/// transcripts are byte-identical, reporting a window around the first
/// differing line (the transcripts run to megabytes).
fn diff2(tag: &str, f: impl Fn(&Lib) -> String) {
    let p = libs();
    let a = one_side(&p.c, &f);
    let b = one_side(&p.rs, &f);
    if a == b {
        return;
    }
    let al: Vec<&str> = a.lines().collect();
    let bl: Vec<&str> = b.lines().collect();
    let n = al.len().min(bl.len());
    let mut i = 0;
    while i < n && al[i] == bl[i] {
        i += 1;
    }
    let lo = i.saturating_sub(3);
    let hi = (i + 4).min(al.len().max(bl.len()));
    let mut msg = format!(
        "divergence in [{tag}] at line {i} (C has {} lines, RUST has {} lines)\n",
        al.len(),
        bl.len()
    );
    for k in lo..hi {
        let m = if k == i { ">>" } else { "  " };
        msg.push_str(&format!(
            "{m}{k} C    {}\n{m}{k} RUST {}\n",
            al.get(k).copied().unwrap_or("<eof>"),
            bl.get(k).copied().unwrap_or("<eof>")
        ));
    }
    panic!("{msg}");
}

/* ------------------------------------------------------- slot inspection */

/// Every predicate in rows 57 / 159, in a fixed order, rendered as a bit string.
const PREDS: [&str; 17] = [
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

/// `#` and `length`, read through `js_hasproperty`, to give JS_TOBJECT slots a
/// stable identity.
///
/// Only ever called for a JS_TOBJECT: `js_hasproperty` funnels through
/// `js_toobject` (jsrun.c:1013), which throws for undefined/null and *rewrites
/// the slot* for a primitive. None of the objects built by this file installs a
/// getter named `#` or `length`, so `jsR_hasproperty` cannot re-enter JS and
/// therefore cannot throw.
unsafe fn objmark(l: &Lib, j: JS, idx: c_int) -> String {
    let mut s = String::new();
    for name in [MARK, LENGTH] {
        if l.js_hasproperty(j, idx, name) != 0 {
            if l.pred("js_isnumber", j, -1) != 0 {
                s.push_str(&format!("{:016x},", l.js_tonumber(j, -1).to_bits()));
            } else {
                s.push_str(&format!("t{},", l.js_type(j, -1)));
            }
            l.js_pop(j, 1);
        } else {
            s.push_str("-,");
        }
    }
    s
}

/// Compact, wholly non-throwing description of one stack slot.
///
/// `js_type` / `js_typeof` / the `js_is*` family never throw (jsrun.c:232-283).
/// `jsV_tostring` / `jsV_tonumber` / `jsV_toboolean` only re-enter JS for
/// JS_TOBJECT (jsvalue.c:152/247/330), so they are safe once the tag is known
/// to be primitive.
unsafe fn slot(l: &Lib, j: JS, idx: c_int) -> String {
    let t = l.js_type(j, idx);
    match t {
        JS_ISUNDEFINED => "u".to_string(),
        JS_ISNULL => "z".to_string(),
        JS_ISBOOLEAN => format!("b{}", l.js_toboolean(j, idx)),
        JS_ISNUMBER => format!("n{:016x}", l.js_tonumber(j, idx).to_bits()),
        JS_ISSTRING => format!("s{:?}", from_c(l.js_tostring(j, idx))),
        _ => format!(
            "o{t}:{}:{}{}{}",
            from_c(l.js_typeof(j, idx)),
            objmark(l, j, idx),
            l.js_isuserdata(j, idx, UD_A),
            l.js_isuserdata(j, idx, UD_B),
        ),
    }
}

/// `slot` plus the whole predicate battery.
unsafe fn vdesc(l: &Lib, j: JS, idx: c_int) -> String {
    let mut bits = String::new();
    for name in PREDS {
        bits.push(if l.pred(name, j, idx) != 0 { '1' } else { '0' });
    }
    format!("{}|{bits}", slot(l, j, idx))
}

/// `slot` plus `js_typeof` plus `js_tryrepr`.
///
/// `js_torepr` (and therefore `js_tryrepr`) ends in
/// `js_replace(J, idx < 0 ? idx-1 : idx)` (jsrepr.c:271), i.e. it *overwrites*
/// the slot it was asked about, so it is applied to a `js_copy` of the slot
/// instead and the original is left untouched.
unsafe fn snap_at(l: &Lib, j: JS, idx: c_int) -> String {
    l.js_copy(j, idx);
    let r = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 1);
    format!("{}/{}/{r:?}", slot(l, j, idx), from_c(l.js_typeof(j, idx)))
}

/// The whole observable stack: `js_gettop` plus every slot reached through a
/// positive (BOT-relative) index, every slot reached through a negative
/// (TOP-relative) index, and a fixed ring of out-of-range indices (row 45).
unsafe fn stack_snap(l: &Lib, j: JS) -> String {
    let top = l.js_gettop(j);
    let mut s = format!("top={top}");
    for i in 0..top {
        s.push_str(&format!(" +{i}={}", snap_at(l, j, i)));
    }
    for i in 1..=top {
        s.push_str(&format!(" -{i}={}", snap_at(l, j, -i)));
    }
    for oob in [top, top + 1, top + 37, -(top + 1), -(top + 40), 4100, -4100] {
        s.push_str(&format!(" x{oob}={}", snap_at(l, j, oob)));
    }
    s
}

/* ------------------------------------------------------- value shapes */

/// How to produce one value shape.
#[derive(Clone, Debug)]
enum Mk {
    /// compile+run as a script; the script's value is left on the stack
    Js(&'static str),
    /// one of the native builders in `build_native`
    Nat(u8),
    /// `js_pushnumber`
    Num(f64),
    /// `js_pushlstring` with `n = bytes.len()`, so embedded NULs survive
    Str(Vec<u8>),
    /// `js_pushliteral` of a NUL-terminated `'static` buffer
    Lit(&'static [u8]),
}

unsafe extern "C" fn cf_noop(j: JS) {
    let l = cur();
    l.js_pushnumber(j, 1.0);
}

unsafe fn build_native(l: &Lib, j: JS, k: u8) {
    match k {
        0 => l.js_pushundefined(j),
        1 => l.js_pushnull(j),
        2 => l.js_pushboolean(j, 0),
        3 => l.js_pushboolean(j, 1),
        // row 58: js_pushboolean takes any int and stores !!v
        4 => l.js_pushboolean(j, 42),
        5 => l.js_pushboolean(j, -7),
        6 => l.js_pushboolean(j, c_int::MIN),
        7 => l.js_pushnumber(j, -0.0),
        8 => l.js_pushnumber(j, f64::NAN),
        // rows 61/62: the SHRSTR/MEMSTR boundary at strlen 15 vs 16
        9 => l.js_pushstring(j, S15),
        10 => l.js_pushstring(j, S16),
        // row 67: embedded NUL; all n bytes are stored, every consumer stops
        // at the first NUL
        11 => l.js_pushlstring(j, b"a\0b".as_ptr() as *const c_char, 3),
        12 => l.js_pushlstring(j, b"\0ab".as_ptr() as *const c_char, 3),
        // row 65: n=0 never dereferences v
        13 => l.js_pushlstring(j, std::ptr::null(), 0),
        // rows 68/69: JS_TLITSTR
        14 => l.js_pushliteral(j, LIT_EMPTY),
        15 => l.js_pushliteral(j, LIT_15),
        16 => l.js_pushliteral(j, LIT_16),
        17 => l.js_pushliteral(j, LIT_NUM),
        // row 70
        18 => l.js_pushglobal(j),
        19 => l.js_newobject(j),
        20 => l.js_newarray(j),
        21 => l.js_newboolean(j, 0),
        22 => l.js_newboolean(j, 9),
        23 => l.js_newnumber(j, -0.0),
        24 => l.js_newnumber(j, 1.5),
        // jsvalue.c:385 splits at `n < sizeof shrstr` == 16, i.e. <= 15
        25 => l.js_newstring(j, S15),
        26 => l.js_newstring(j, S16),
        // row 72: js_newobjectx with an object on top vs a non-object
        27 => {
            l.js_pushglobal(j);
            l.js_newobjectx(j)
        }
        28 => {
            l.js_pushnumber(j, 3.0);
            l.js_newobjectx(j)
        }
        29 => {
            l.js_pushnull(j);
            l.js_newuserdata(j, UD_A, ud_data(), None)
        }
        30 => {
            l.js_pushnull(j);
            l.js_newuserdata(j, UD_B, std::ptr::null_mut(), None)
        }
        31 => l.js_newcfunction(j, Some(cf_noop), NOOP, 3),
        32 => l.js_newcfunctionx(j, Some(cf_noop), NOOP, 4, ud_data(), None),
        // js_newcconstructor consumes a prototype object from the stack
        // (jsvalue.c:521) and leaves the constructor
        33 => {
            l.js_newobject(j);
            l.js_newcconstructor(j, Some(cf_noop), Some(cf_noop), CTOR, 2)
        }
        34 => l.js_newregexp(j, RE_PAT, JS_REGEXP_G | JS_REGEXP_I),
        35 => l.js_newregexp(j, RE_PAT, 0),
        // JS_CSCRIPT: js_ploadstring leaves the compiled script object on the
        // stack (jsstate.c:36). Row 158: js_typeof says "object" and js_type
        // says JS_ISOBJECT, yet js_iscallable reports it callable.
        36 => {
            let rc = l.js_ploadstring(j, FILENAME, b"1+1\0".as_ptr() as *const c_char);
            assert_eq!(rc, 0, "{}: JS_CSCRIPT shape failed to compile", l.name);
        }
        // JS_CITERATOR (row 159/167)
        37 => {
            l.js_newobject(j);
            l.js_pushnumber(j, 1.0);
            l.js_setproperty(j, -2, MARK);
            l.js_pushiterator(j, -1, 1);
            l.nullary("js_rot2pop1", j);
        }
        _ => panic!("bad native shape {k}"),
    }
}

/// Push one shape, leaving exactly one value.
unsafe fn build_one(l: &Lib, j: JS, m: &Mk) {
    match m {
        Mk::Js(src) => {
            let cs = cstr(src);
            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
            assert_eq!(rc, 0, "{}: shape {src:?} did not compile", l.name);
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            assert_eq!(
                rc,
                0,
                "{}: shape {src:?} threw {}",
                l.name,
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            );
        }
        Mk::Nat(k) => build_native(l, j, *k),
        Mk::Num(v) => l.js_pushnumber(j, *v),
        Mk::Str(b) => l.js_pushlstring(j, b.as_ptr() as *const c_char, b.len() as c_int),
        Mk::Lit(b) => l.js_pushliteral(j, b.as_ptr() as *const c_char),
    }
}

/// Build every shape and park it in the registry under `v<i>` so any frame can
/// re-push the *exact* value, tag included (`jsR_setproperty` stores the
/// `js_Value` verbatim, jsrun.c:971, and `js_getregistry` pushes it back with
/// `js_pushvalue`).
unsafe fn build_shapes(l: &Lib, j: JS, list: &[Mk]) -> Vec<CString> {
    let mut names = Vec::with_capacity(list.len());
    for (i, m) in list.iter().enumerate() {
        let base = l.js_gettop(j);
        build_one(l, j, m);
        assert_eq!(
            l.js_gettop(j),
            base + 1,
            "{}: shape {i} {m:?} pushed {} values",
            l.name,
            l.js_gettop(j) - base
        );
        let n = CString::new(format!("v{i}")).unwrap();
        l.js_setregistry(j, n.as_ptr());
        names.push(n);
    }
    names
}

/// The full shape list: every value tag, every object class, both string
/// storage classes, and the throwing conversions.
fn shapes_full() -> Vec<Mk> {
    let mut v: Vec<Mk> = vec![];
    for s in [
        "undefined", "null", "true", "false", "0", "-0", "NaN", "Infinity",
        "-Infinity", "1", "-1", "0.5", "-3.25", "1e21", "1e-7", "1e300", "5e-324",
        "2147483647", "-2147483648", "4294967296", "65536", "65535", "32768",
        "1/3", "''", "'a'", "'abc'", "'0'", "'1'", "'  12  '", "'0x1f'",
        "'Infinity'", "'-Infinity'", "'12abc'", "'1.5'", "'1e3'",
        "'0123456789abcde'", "'0123456789abcdef'", "'true'", "({})",
        "({a:1,b:'x'})", "[]", "[1,2,3]",
        "(function(){var a=[1,2];a[5]=6;return a})()",
        "(function f(a,b){return a})", "print", "Math", "JSON", "/a(b)c/gi",
        "new Date(0)", "new Date(1234567890)", "new Number(5)", "new Number(0)",
        "new String('xy')", "new String('')", "new Boolean(false)",
        "new Boolean(true)", "new Error('boom')", "new TypeError('bad')",
        "Object.create(null)", "({toString:function(){throw 'TS!'}})",
        "({valueOf:function(){throw 'VO!'}})", "({valueOf:function(){return 7}})",
        "({toString:function(){return 'TS'}})",
        "({valueOf:function(){return {}},toString:function(){return {}}})",
        "Object.prototype", "Array.prototype", "Function.prototype",
        // JS_CARGUMENTS (row 159)
        "(function(){return arguments})(1,'x')",
        "(function(){var f=function(){}; f.prototype=1; return f})()",
    ] {
        v.push(Mk::Js(s));
    }
    for k in 0..=37u8 {
        v.push(Mk::Nat(k));
    }
    // randomised doubles and strings, fixed seed
    let mut rng = Rng::new(0x5741_0C5A_C001);
    for _ in 0..16 {
        v.push(Mk::Num(rng.f64_sane()));
    }
    for _ in 0..6 {
        v.push(Mk::Num(rng.f64_any()));
    }
    for _ in 0..8 {
        v.push(Mk::Str(rng.ascii_string(24).into_bytes()));
    }
    for _ in 0..6 {
        v.push(Mk::Str(rng.unicode_string(8).into_bytes()));
    }
    for _ in 0..6 {
        v.push(Mk::Str(rng.raw_bytes(20)));
    }
    v.push(Mk::Lit(b"lit\0"));
    v.push(Mk::Lit(b"0\0"));
    v
}

/// A curated subset for the O(n^2) comparison cross product (rows 169-182).
fn shapes_cross() -> Vec<Mk> {
    let mut v: Vec<Mk> = vec![];
    for s in [
        "undefined", "null", "true", "false", "0", "-0", "NaN", "Infinity",
        "-Infinity", "1", "-1", "0.5", "1e21", "2147483647", "''", "'a'", "'0'",
        "'1'", "'  12  '", "'0x1f'", "'Infinity'", "'12abc'",
        "'0123456789abcdef'", "({})", "({a:1})", "[]", "[1,2,3]", "[1]",
        "(function f(a,b){return a})", "print", "Math", "/a(b)c/gi",
        "new Date(0)", "new Number(5)", "new String('xy')", "new String('1')",
        "new Boolean(false)", "new Error('boom')", "Object.create(null)",
        "({toString:function(){throw 'TS!'}})", "({valueOf:function(){return 7}})",
        "Object", "Date", "Error",
        // row 181: callable, but its 'prototype' property is not an object
        "(function(){var f=function(){}; f.prototype=1; return f})()",
        // row 181 again: JS_CCFUNCTION with no 'prototype' property at all
        "Function.prototype",
    ] {
        v.push(Mk::Js(s));
    }
    v.push(Mk::Nat(4)); // js_pushboolean(42)
    v.push(Mk::Nat(11)); // "a\0b"
    v.push(Mk::Nat(15)); // JS_TLITSTR, 15 bytes
    v.push(Mk::Nat(29)); // userdata
    v.push(Mk::Num(-0.0));
    v.push(Mk::Lit(b"1\0"));
    v
}

/* ----------------------------------------------------- probe machinery */

thread_local! {
    static OP: Cell<u32> = const { Cell::new(0) };
    static A0: Cell<c_int> = const { Cell::new(0) };
    static NPRE: Cell<c_int> = const { Cell::new(0) };
    static NAME_A: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
    static NAME_B: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
    static OKAY: Cell<c_int> = const { Cell::new(-1) };
    static DEPTH: Cell<c_int> = const { Cell::new(0) };
}

unsafe fn install(l: &Lib, j: JS, f: js_CFunction, reg: *const c_char) {
    l.js_newcfunction(j, f, PNAME, 0);
    l.js_setregistry(j, reg);
}

/// Call a registry-parked cfunction through `js_pcall`, then report the return
/// code, the repr of the result, that the frame came back balanced, and
/// whatever the probe wrote into the output buffer.
unsafe fn run(l: &Lib, j: JS, reg: *const c_char) -> String {
    let base = l.js_gettop(j);
    l.js_getregistry(j, reg);
    l.js_pushundefined(j);
    let rc = l.js_pcall(j, 0);
    let res = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 1);
    let after = l.js_gettop(j);
    let detail = out_take().replace('\n', " ~ ");
    format!("rc={rc} res={res:?} top={after}/{base} :: {}", detail.trim_end())
}

/* ============================================================ rows 41-54 */

/// Any index at all: `stackidx` is total when BOT == 0.
fn pick_idx(rng: &mut Rng, top: c_int) -> c_int {
    match rng.below(6) {
        0 => rng.range(0, top.max(1) as i64) as c_int,
        1 => -(1 + rng.range(0, top.max(1) as i64) as c_int),
        2 => top + rng.below(4) as c_int,
        3 => -(top + 1 + rng.below(4) as c_int),
        4 => rng.range(-4100, 4100) as c_int,
        _ => 0,
    }
}

/// Only the indices `js_remove` / `js_replace` accept without raising
/// js_error "stack error!" (jsrun.c:412 / :427).
fn pick_valid(rng: &mut Rng, top: c_int) -> c_int {
    assert!(top >= 1);
    if rng.below(2) == 0 {
        rng.range(0, top as i64) as c_int
    } else {
        -(1 + rng.range(0, top as i64) as c_int)
    }
}

fn uniq_val(r: &mut Rng, u: &mut i64) -> f64 {
    *u += 1;
    *u as f64 + (r.below(4) as f64) / 8.0
}

/// One randomised stack-operation sequence, run at top level (BOT == 0).
unsafe fn seq(l: &Lib, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let j = new_state(l, 0);
    let mut t = String::new();
    let mut uniq = 0i64;

    // give the global a mark so JS_TOBJECT slots stay distinguishable
    l.js_pushglobal(j);
    l.js_pushnumber(j, -1.0);
    l.js_setproperty(j, -2, MARK);
    l.js_pop(j, 1);

    let nstart = rng.below(6) as c_int;
    for _ in 0..nstart {
        let v = uniq_val(&mut rng, &mut uniq);
        l.js_pushnumber(j, v);
    }
    let nops = 20 + rng.below(61);
    t.push_str(&format!("start {}\n", stack_snap(l, j)));
    for step in 0..nops {
        let top = l.js_gettop(j);
        // The rot/dup family has no bounds check at all (jsrun.c:442-497), so
        // each op may only be picked once the frame holds the number of values
        // it consumes. Picking from exactly the legal set means rows 51/52/53
        // get driven at their exact boundary (2, 3 resp. 4 values present).
        const MINS: [c_int; 22] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // pushes
            0, // pop
            0, // copy
            1, // remove
            1, // replace
            1, // dup
            2, // dup2
            2, // rot2
            3, // rot3
            4, // rot4
            2, // rot2pop1
            3, // rot3pop2
            1, // rot
        ];
        let mut legal: Vec<u32> = (0..22u32).filter(|&o| MINS[o as usize] <= top).collect();
        // keep the frame bounded; row 55 tests CHECKSTACK on its own
        if top >= 32 {
            legal.retain(|&o| o >= 10);
        }
        let op = legal[rng.below(legal.len() as u32) as usize];
        let desc: String = match op {
            0 => {
                l.js_pushundefined(j);
                "pushundefined".into()
            }
            1 => {
                l.js_pushnull(j);
                "pushnull".into()
            }
            2 => {
                let b = [0, 1, 42, -3, c_int::MIN][rng.below(5) as usize];
                l.js_pushboolean(j, b);
                format!("pushboolean({b})")
            }
            3 => {
                let v = uniq_val(&mut rng, &mut uniq);
                l.js_pushnumber(j, v);
                format!("pushnumber({v})")
            }
            4 => {
                let v = uniq_val(&mut rng, &mut uniq);
                let s = cstr(&format!("s{v}"));
                l.js_pushstring(j, s.as_ptr());
                format!("pushstring(short {v})")
            }
            5 => {
                let v = uniq_val(&mut rng, &mut uniq);
                let s = cstr(&format!("mem-{v}-0123456789abcdef"));
                l.js_pushstring(j, s.as_ptr());
                format!("pushstring(long {v})")
            }
            6 => {
                let k = rng.below(4);
                let p = [LIT_EMPTY, LIT_1, LIT_15, LIT_16][k as usize];
                l.js_pushliteral(j, p);
                format!("pushliteral({k})")
            }
            7 => {
                l.js_pushglobal(j);
                "pushglobal".into()
            }
            8 => {
                let v = uniq_val(&mut rng, &mut uniq);
                l.js_newobject(j);
                l.js_pushnumber(j, v);
                l.js_setproperty(j, -2, MARK);
                format!("newobject({v})")
            }
            9 => {
                let v = uniq_val(&mut rng, &mut uniq);
                l.js_newarray(j);
                l.js_pushnumber(j, v);
                l.js_setproperty(j, -2, MARK);
                format!("newarray({v})")
            }
            // rows 42/43: n=0 and n == js_gettop are the two non-throwing ends
            10 => {
                let n = rng.below(top as u32 + 1) as c_int;
                l.js_pop(j, n);
                format!("pop({n})")
            }
            // rows 44/45: any index, in range or not, is well defined
            11 => {
                let idx = pick_idx(&mut rng, top);
                l.js_copy(j, idx);
                format!("copy({idx})")
            }
            12 => {
                let idx = pick_valid(&mut rng, top);
                l.js_remove(j, idx);
                format!("remove({idx})")
            }
            13 => {
                let idx = pick_valid(&mut rng, top);
                l.js_replace(j, idx);
                format!("replace({idx})")
            }
            14 => {
                l.nullary("js_dup", j);
                "dup".into()
            }
            15 => {
                l.nullary("js_dup2", j);
                "dup2".into()
            }
            16 => {
                l.nullary("js_rot2", j);
                "rot2".into()
            }
            17 => {
                l.nullary("js_rot3", j);
                "rot3".into()
            }
            18 => {
                l.nullary("js_rot4", j);
                "rot4".into()
            }
            19 => {
                l.nullary("js_rot2pop1", j);
                "rot2pop1".into()
            }
            20 => {
                l.nullary("js_rot3pop2", j);
                "rot3pop2".into()
            }
            _ => {
                // row 54: n<=1 is a no-op, n=2 is js_rot2, n=k>2 rotates k
                let n = rng.below(top as u32 + 1) as c_int;
                l.js_rot(j, n);
                format!("rot({n})")
            }
        };
        t.push_str(&format!("{step} {desc} -> {}\n", stack_snap(l, j)));
    }
    l.js_freestate(j);
    t
}

/// Rows 41-54: randomised operation sequences with the whole observable stack
/// compared after every single step.
#[test]
fn t_stack_ops_random() {
    for s in 0..120u64 {
        let seed = 0x5732_0001 + s * 7919;
        diff2(&format!("seq seed={seed}"), move |l| unsafe { seq(l, seed) });
    }
}

/* =================================================== rows 43/47/49/50/55 */

/// The driver for everything that can throw. `NPRE` extra values are pushed
/// first so the frame shape is known, then op `OP` runs with argument `A0`.
unsafe extern "C" fn cf_op(j: JS) {
    let l = cur();
    let npre = NPRE.with(|c| c.get());
    for i in 0..npre {
        l.js_pushnumber(j, 100.0 + i as f64);
    }
    let op = OP.with(|c| c.get());
    let a0 = A0.with(|c| c.get());
    if op < 9 {
        out_push(format!("pre {}", stack_snap(l, j)).as_bytes());
    } else {
        out_push(format!("pre top={}", l.js_gettop(j)).as_bytes());
    }
    match op {
        0 => l.js_pop(j, a0),
        1 => l.js_remove(j, a0),
        2 => l.js_replace(j, a0),
        3 => l.js_insert(j, a0),
        4 => l.js_rot(j, a0),
        5 => l.js_copy(j, a0),
        6 => l.js_pushlstring(j, std::ptr::null(), 0),
        7 => l.js_pushlstring(j, S16, a0),
        8 => l.js_pushstring(j, S16),
        // row 55: push until CHECKSTACK fires
        9 => {
            for i in 0..(JS_STACKSIZE + 64) {
                l.js_pushnumber(j, i as f64);
            }
        }
        // row 55 through js_copy / js_dup / js_dup2 (CHECKSTACK(1) and (2))
        10 => {
            for i in 0..(JS_STACKSIZE + 64) {
                match i % 3 {
                    0 => l.js_copy(j, 0),
                    1 => l.nullary("js_dup", j),
                    _ => l.nullary("js_dup2", j),
                }
            }
        }
        11 => l.js_currentfunction(j),
        _ => {}
    }
    if op < 9 {
        out_push(format!(" post {}", stack_snap(l, j)).as_bytes());
    } else {
        out_push(format!(" post top={}", l.js_gettop(j)).as_bytes());
    }
    l.js_pushundefined(j);
}

/// Rows 43 / 47 / 49: every throwing stack operation, always inside a protected
/// `js_pcall` frame so BOT > 0 and the "resolves below BOT" arm of the
/// `idx < BOT` checks (jsrun.c:415 / :430) is reachable.
#[test]
fn t_stack_errors() {
    diff2("stack errors", |l| unsafe {
        let j = new_state(l, 0);
        install(l, j, Some(cf_op), REG_OP);
        // two top-level values, so BOT-relative and TOP-relative indices that
        // resolve below BOT land on live slots rather than off the array
        l.js_pushnumber(j, 7.0);
        l.js_pushliteral(j, LIT_15);
        let mut s = String::new();
        for npre in [0i32, 1, 3] {
            for (op, name) in [
                (1u32, "remove"),
                (2, "replace"),
                (5, "copy"),
                (4, "rot"),
            ] {
                for a0 in [
                    -100000i32, -9999, -100, -(npre + 6), -(npre + 2), -(npre + 1),
                    -3, -2, -1, 0, 1, 2, 3, npre, npre + 1, npre + 2, npre + 40,
                    4099, 100000,
                ] {
                    // js_rot(J, n) is only bounds-safe for 0 <= n <= gettop
                    if op == 4 && !(0..=(npre + 1)).contains(&a0) {
                        continue;
                    }
                    OP.with(|c| c.set(op));
                    A0.with(|c| c.set(a0));
                    NPRE.with(|c| c.set(npre));
                    s.push_str(&format!(
                        "{name}({a0}) npre={npre} {}\n",
                        run(l, j, REG_OP)
                    ));
                }
            }
            // js_pop: n < 0 is UB (see the file header), so only n >= 0
            for a0 in [0i32, 1, 2, 3, npre, npre + 1, npre + 2, npre + 40, 4099, 100000] {
                OP.with(|c| c.set(0));
                A0.with(|c| c.set(a0));
                NPRE.with(|c| c.set(npre));
                s.push_str(&format!("pop({a0}) npre={npre} {}\n", run(l, j, REG_OP)));
            }
        }
        s.push_str(&format!("after {}\n", stack_snap(l, j)));
        l.js_freestate(j);
        s
    });
}

/// Row 50: `js_insert` is unconditionally `js_error(J, "not implemented yet")`
/// (jsrun.c:422-425), for every index and every stack shape. It therefore must
/// only ever be called from a protected frame.
#[test]
fn t_insert_never_implemented() {
    diff2("insert", |l| unsafe {
        let j = new_state(l, 0);
        install(l, j, Some(cf_op), REG_OP);
        let mut s = String::new();
        for npre in [0i32, 1, 2, 5] {
            for a0 in [
                c_int::MIN + 1,
                -100000,
                -4096,
                -7,
                -1,
                0,
                1,
                5,
                4096,
                100000,
                c_int::MAX - 1,
            ] {
                OP.with(|c| c.set(3));
                A0.with(|c| c.set(a0));
                NPRE.with(|c| c.set(npre));
                s.push_str(&format!("insert({a0}) npre={npre} {}\n", run(l, j, REG_OP)));
            }
        }
        l.js_freestate(j);
        s
    });
}

/// Rows 55 / 64 / 65: the CHECKSTACK overflow, the JS_STRLIMIT guard (which
/// fires before any copying, so a 16-byte buffer with a huge `n` is safe), and
/// `js_pushlstring(NULL, 0)`.
#[test]
fn t_push_limits() {
    diff2("push limits", |l| unsafe {
        let j = new_state(l, 0);
        install(l, j, Some(cf_op), REG_OP);
        let mut s = String::new();
        for (op, a0, tag) in [
            (6u32, 0i32, "pushlstring(NULL,0)"),
            (7, JS_STRLIMIT + 1, "pushlstring(n=STRLIMIT+1)"),
            (7, JS_STRLIMIT + 4096, "pushlstring(n=STRLIMIT+4096)"),
            (7, c_int::MAX, "pushlstring(n=INT_MAX)"),
            (7, 0, "pushlstring(n=0)"),
            (7, 15, "pushlstring(n=15)"),
            (7, 16, "pushlstring(n=16)"),
            (8, 0, "pushstring(16)"),
            (9, 0, "overflow via pushnumber"),
            (10, 0, "overflow via copy/dup/dup2"),
            (11, 0, "currentfunction in frame"),
        ] {
            OP.with(|c| c.set(op));
            A0.with(|c| c.set(a0));
            NPRE.with(|c| c.set(2));
            s.push_str(&format!("{tag} {}\n", run(l, j, REG_OP)));
        }
        s.push_str(&format!("after {}\n", stack_snap(l, j)));
        l.js_freestate(j);
        s
    });
}

/* ================================================== rows 41/44/56: frames */

/// Reports its frame, duplicates `this` with `js_copy(J, 0)` (row 44), asks for
/// `js_currentfunction` / `js_currentfunctiondata` (row 56) and then recurses so
/// BOT keeps growing.
unsafe extern "C" fn cf_frame(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    let mut s = format!("frame gettop={top}");
    for i in 0..top {
        s.push_str(&format!(" a{i}={}", slot(l, j, i)));
    }
    l.js_copy(j, 0);
    s.push_str(&format!(" this={} top={}", slot(l, j, -1), l.js_gettop(j)));
    l.js_pop(j, 1);
    l.js_currentfunction(j);
    s.push_str(&format!(" curfn={} top={}", slot(l, j, -1), l.js_gettop(j)));
    l.js_pop(j, 1);
    let d = l.js_currentfunctiondata(j);
    s.push_str(&format!(
        " curdata={}",
        if d.is_null() {
            "NULL"
        } else if d == ud_data() {
            "UD_DATA"
        } else {
            "other"
        }
    ));
    out_push(s.as_bytes());

    let d = DEPTH.with(|c| c.get());
    if d > 0 {
        DEPTH.with(|c| c.set(d - 1));
        l.js_getregistry(j, REG_FRAME);
        l.js_pushnumber(j, 7.0 + d as f64);
        l.js_pushnumber(j, d as f64);
        l.js_pushliteral(j, LIT_15);
        let rc = l.js_pcall(j, 2);
        out_push(format!(" [nested rc={rc} res={}]", slot(l, j, -1)).as_bytes());
        l.js_pop(j, 1);
    }
    l.js_pushnumber(j, 42.0 + top as f64);
}

/// Rows 41 / 44 / 56: `js_gettop` at top level (BOT == 0) vs inside nested
/// cfunction frames, `js_copy(J, 0)` reaching `this`, and `js_currentfunction` /
/// `js_currentfunctiondata` in both places.
#[test]
fn t_frames_and_currentfunction() {
    diff2("frames", |l| unsafe {
        let mut s = String::new();
        for depth in [0i32, 1, 3] {
            for data in [false, true] {
                let j = new_state(l, 0);
                if data {
                    l.js_newcfunctionx(j, Some(cf_frame), PNAME, 0, ud_data(), None);
                } else {
                    l.js_newcfunction(j, Some(cf_frame), PNAME, 0);
                }
                l.js_setregistry(j, REG_FRAME);

                // row 56 at BOT == 0
                l.js_currentfunction(j);
                s.push_str(&format!(
                    "d={depth} data={data} toplevel curfn={} ",
                    vdesc(l, j, -1)
                ));
                l.js_pop(j, 1);
                s.push_str(&format!(
                    "curdata_null={} gettop={}\n",
                    l.js_currentfunctiondata(j).is_null(),
                    l.js_gettop(j)
                ));

                DEPTH.with(|c| c.set(depth));
                l.js_getregistry(j, REG_FRAME);
                l.js_pushnumber(j, 1.0);
                let rc = l.js_pcall(j, 0);
                s.push_str(&format!(
                    "d={depth} data={data} noargs rc={rc} res={} top={} :: {}\n",
                    slot(l, j, -1),
                    l.js_gettop(j),
                    out_take().replace('\n', " ~ ")
                ));
                l.js_pop(j, 1);

                // and with arguments, so BOT-relative indices 1..n exist
                DEPTH.with(|c| c.set(depth));
                l.js_getregistry(j, REG_FRAME);
                l.js_pushnull(j);
                l.js_pushnumber(j, 2.5);
                l.js_pushliteral(j, LIT_16);
                l.js_pushglobal(j);
                let rc = l.js_pcall(j, 3);
                s.push_str(&format!(
                    "d={depth} data={data} args rc={rc} res={} top={} :: {}\n",
                    slot(l, j, -1),
                    l.js_gettop(j),
                    out_take().replace('\n', " ~ ")
                ));
                l.js_pop(j, 1);
                s.push_str(&format!("d={depth} data={data} end {}\n", stack_snap(l, j)));

                // also driven from JS, so the callee reaches BOT-1 through the
                // interpreter instead of through js_pcall
                l.js_getregistry(j, REG_FRAME);
                l.js_setglobal(j, PNAME);
                DEPTH.with(|c| c.set(0));
                let src = cstr("print(probe(), probe.call(1, 2, 3))");
                let rc = l.js_dostring(j, src.as_ptr());
                s.push_str(&format!("d={depth} data={data} dostring={rc}\n"));
                l.js_freestate(j);
            }
        }
        s
    });
}

/* =========================================== rows 57-70: pushes + strings */

/// Distinguish JS_TSHRSTR / JS_TLITSTR / JS_TMEMSTR without reading the tag.
///
/// `jsV_tostring` (jsvalue.c:330) hands back `v->u.shrstr` for JS_TSHRSTR, i.e.
/// a pointer INTO the 16-byte stack slot; for JS_TLITSTR it hands back the
/// caller's own pointer and for JS_TMEMSTR the `js_String` payload. So copying
/// the value into a fresh slot changes the pointer only for a SHRSTR, and
/// comparing against the literal we pushed separates LITSTR from MEMSTR.
unsafe fn strtag(l: &Lib, j: JS, idx: c_int, lit: *const c_char) -> &'static str {
    if l.pred("js_isstring", j, idx) == 0 {
        return "-";
    }
    let p1 = l.js_tostring(j, idx);
    l.js_copy(j, idx);
    let p2 = l.js_tostring(j, -1);
    l.js_pop(j, 1);
    if p1 != p2 {
        "SHRSTR"
    } else if !lit.is_null() && p1 == lit {
        "LITSTR"
    } else {
        "MEMSTR"
    }
}

/// Rows 57-70: every push entry point over the SHRSTR / LITSTR / MEMSTR length
/// axis. `js_pushstring` / `js_pushlstring` split at
/// `n <= soffsetof(js_Value, t.type)` == 15 (jsrun.c:145/163) while
/// `jsV_newstring` splits at `n < sizeof(obj->u.s.shrstr)` == 16
/// (jsvalue.c:385) - the same boundary written two different ways, so both are
/// swept. Empty strings, embedded NULs and n=0 are included.
#[test]
fn t_pushes_and_strings() {
    let mut bodies: Vec<Vec<u8>> = vec![];
    for n in 0..=20usize {
        bodies.push((0..n).map(|i| b'a' + (i % 26) as u8).collect());
    }
    for n in [30usize, 31, 32, 63, 64, 65] {
        bodies.push((0..n).map(|i| b'A' + (i % 26) as u8).collect());
    }
    bodies.push(b"a\0b".to_vec());
    bodies.push(b"\0abc".to_vec());
    bodies.push(b"abcdefghijklmno\0extra".to_vec());
    bodies.push(b"0123456789abcde\0".to_vec());
    let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);
    for _ in 0..120 {
        bodies.push(rng.ascii_string(20).into_bytes());
    }
    for _ in 0..60 {
        bodies.push(rng.unicode_string(8).into_bytes());
    }
    for _ in 0..60 {
        bodies.push(rng.raw_bytes(20));
    }
    for _ in 0..40 {
        bodies.push(rng.raw_bytes(40));
    }

    diff2("push strings", move |l| unsafe {
        let j = new_state(l, 0);
        let mut s = String::new();
        for body in &bodies {
            let mut z = body.clone();
            z.push(0);
            let zp = z.as_ptr() as *const c_char;
            for (k, what) in [
                (0u32, "pushstring"),
                (1, "pushlstring(len)"),
                (2, "pushliteral"),
                (3, "newstring"),
                (4, "pushlstring(len/2)"),
            ] {
                let base = l.js_gettop(j);
                match k {
                    0 => l.js_pushstring(j, zp),
                    1 => l.js_pushlstring(j, body.as_ptr() as *const c_char, body.len() as c_int),
                    2 => l.js_pushliteral(j, zp),
                    3 => l.js_newstring(j, zp),
                    _ => l.js_pushlstring(
                        j,
                        body.as_ptr() as *const c_char,
                        (body.len() / 2) as c_int,
                    ),
                }
                let lit = if k == 2 { zp } else { std::ptr::null() };
                s.push_str(&format!(
                    "{body:?} {what}: n={} tag={} {} repr={:?} bool={} num={:016x}\n",
                    l.js_gettop(j) - base,
                    strtag(l, j, -1, lit),
                    vdesc(l, j, -1),
                    from_c(l.js_tryrepr(j, -1, ERRSTR)),
                    l.js_tryboolean(j, -1, 9),
                    l.js_trynumber(j, -1, -1.0).to_bits(),
                ));
                l.js_pop(j, l.js_gettop(j) - base);
            }
            // row 69: the same bytes as LITSTR and as SHRSTR/MEMSTR still
            // compare equal, because both comparisons are strcmp-based
            l.js_pushliteral(j, zp);
            l.js_pushstring(j, zp);
            let ok = OKAY.with(|c| c.as_ptr());
            OKAY.with(|c| c.set(-1));
            let cmp = l.js_compare(j, ok);
            s.push_str(&format!(
                "{body:?} lit-vs-str equal={} strict={} cmp={}/{}\n",
                l.nullary_i("js_equal", j),
                l.nullary_i("js_strictequal", j),
                cmp.signum(),
                OKAY.with(|c| c.get())
            ));
            l.js_pop(j, 2);
            // row 62: the SHRSTR/MEMSTR split shows up in the GC string count
            l.js_pushstring(j, zp);
            l.js_pushlstring(j, body.as_ptr() as *const c_char, body.len() as c_int);
            l.js_gc(j, 1);
            l.js_pop(j, 2);
            s.push_str(&format!("{body:?} gc {}\n", out_take().replace('\n', " ~ ")));
        }
        l.js_freestate(j);
        s
    });
}

/// Rows 57-60 / 70-73: the non-string pushes and the `js_new*` constructors,
/// plus the exact stack effect of each.
#[test]
fn t_push_values_and_new() {
    let mut nums: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -3.25,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e21,
        1e-7,
        1e300,
        5e-324,
        2147483647.0,
        -2147483648.0,
        2147483648.0,
        4294967295.0,
        4294967296.0,
        65535.0,
        65536.0,
        32768.0,
        1.0 / 3.0,
        1.2345678901234567e300,
        1e-323,
    ];
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0001);
    for _ in 0..500 {
        nums.push(rng.f64_sane());
    }
    for _ in 0..250 {
        nums.push(rng.f64_any());
    }
    diff2("push numbers", move |l| unsafe {
        let j = new_state(l, 0);
        let mut s = String::new();
        for v in &nums {
            l.js_pushnumber(j, *v);
            l.js_newnumber(j, *v);
            s.push_str(&format!(
                "{:016x}: prim={} repr={:?} str={:?} obj={} orepr={:?}\n",
                v.to_bits(),
                vdesc(l, j, -2),
                from_c(l.js_tryrepr(j, -2, ERRSTR)),
                from_c(l.js_trystring(j, -2, ERRSTR)),
                vdesc(l, j, -1),
                from_c(l.js_tryrepr(j, -1, ERRSTR)),
            ));
            l.js_pop(j, 2);
        }
        l.js_freestate(j);
        s
    });

    diff2("push natives", |l| unsafe {
        let j = new_state(l, 0);
        let mut s = String::new();
        for k in 0..=37u8 {
            let base = l.js_gettop(j);
            build_native(l, j, k);
            s.push_str(&format!(
                "nat{k}: pushed={} {} repr={:?} str={:?} tag={}\n",
                l.js_gettop(j) - base,
                vdesc(l, j, -1),
                from_c(l.js_tryrepr(j, -1, ERRSTR)),
                from_c(l.js_trystring(j, -1, ERRSTR)),
                strtag(l, j, -1, std::ptr::null()),
            ));
        }
        s.push_str(&format!("all {}\n", stack_snap(l, j)));
        l.js_freestate(j);
        s
    });

    // rows 57/58: js_pushboolean normalises any int with !!v
    diff2("pushboolean ints", |l| unsafe {
        let j = new_state(l, 0);
        let mut s = String::new();
        let mut rng = Rng::new(0x00B0_0001);
        let mut vals: Vec<c_int> =
            vec![0, 1, -1, 2, 42, c_int::MIN, c_int::MAX, 256, 0x100, 0x1_0000];
        for _ in 0..64 {
            vals.push(rng.next_u32() as c_int);
        }
        for v in vals {
            l.js_pushboolean(j, v);
            l.js_pushboolean(j, 1);
            s.push_str(&format!(
                "{v}: {} strict1={} eq1={} repr={:?}\n",
                vdesc(l, j, -2),
                l.nullary_i("js_strictequal", j),
                l.nullary_i("js_equal", j),
                from_c(l.js_tryrepr(j, -2, ERRSTR)),
            ));
            l.js_pop(j, 2);
        }
        l.js_freestate(j);
        s
    });
}

/* ============================================ rows 142-158/160/161/163/164 */

/// One conversion applied to one registry-parked shape, with the stack slot
/// described before and after: rows 150/151 note that `js_tostring` on a
/// JS_TNUMBER and `js_toobject` on a primitive rewrite the slot in place.
unsafe extern "C" fn cf_conv(j: JS) {
    let l = cur();
    l.js_getregistry(j, NAME_A.with(|c| c.get()));
    out_push(format!("pre={}", vdesc(l, j, -1)).as_bytes());
    let op = OP.with(|c| c.get());
    let r: String = match op {
        0 => format!("toboolean={}", l.js_toboolean(j, -1)),
        1 => format!("tonumber={:016x}", l.js_tonumber(j, -1).to_bits()),
        2 => format!("tostring={:?}", from_c(l.js_tostring(j, -1))),
        3 => format!("tointeger={}", l.js_tointeger(j, -1)),
        4 => format!("toint32={}", l.js_toint32(j, -1)),
        5 => format!("touint32={}", l.js_touint32(j, -1)),
        6 => format!("toint16={}", l.js_toint16(j, -1)),
        7 => format!("touint16={}", l.js_touint16(j, -1)),
        8 => format!("typeof={}", from_c(l.js_typeof(j, -1))),
        9 => format!("type={}", l.js_type(j, -1)),
        10 => {
            l.js_repr(j, -1);
            format!(
                "repr={:?} top={} under={}",
                from_c(l.js_tostring(j, -1)),
                l.js_gettop(j),
                vdesc(l, j, -2)
            )
        }
        11 => format!("torepr={:?}", from_c(l.js_torepr(j, -1))),
        12 => format!("trystring={:?}", from_c(l.js_trystring(j, -1, ERRSTR))),
        13 => format!(
            "trynumber={:016x}",
            l.js_trynumber(j, -1, -12345.5).to_bits()
        ),
        14 => format!("tryinteger={}", l.js_tryinteger(j, -1, -777)),
        15 => format!("tryboolean={}", l.js_tryboolean(j, -1, 7)),
        16 => format!("tryrepr={:?}", from_c(l.js_tryrepr(j, -1, ERRSTR))),
        17 => {
            let f = l.raw2::<unsafe extern "C" fn(JS, c_int) -> *mut c_void>("js_toobject");
            format!("toobject_null={}", f(j, -1).is_null())
        }
        18 => {
            let f = l.raw2::<unsafe extern "C" fn(JS, c_int, c_int)>("js_toprimitive");
            f(j, -1, A0.with(|c| c.get()));
            "toprimitive".to_string()
        }
        19 => {
            let f = l
                .raw2::<unsafe extern "C" fn(JS, c_int, *const c_char) -> *mut c_void>(
                    "js_touserdata",
                );
            let p = f(j, -1, if A0.with(|c| c.get()) == 0 { UD_A } else { UD_MISS });
            format!(
                "touserdata={}",
                if p.is_null() {
                    "NULL"
                } else if p == ud_data() {
                    "UD_DATA"
                } else {
                    "other"
                }
            )
        }
        20 => {
            let f = l.raw2::<unsafe extern "C" fn(JS, c_int) -> *mut c_void>("js_toregexp");
            format!("toregexp_null={}", f(j, -1).is_null())
        }
        _ => "?".to_string(),
    };
    out_push(format!(" {r} post={} top={}", vdesc(l, j, -1), l.js_gettop(j)).as_bytes());
    l.js_pushundefined(j);
}

/// Rows 142-158, 160, 161, 163, 164: every conversion over every value shape,
/// in both a non-strict and a strict state (row 149 branches on `J->strict`).
#[test]
fn t_conversions() {
    let list = shapes_full();
    for flags in [0, JS_STRICT] {
        let list = list.clone();
        diff2(&format!("conversions flags={flags}"), move |l| unsafe {
            let j = new_state(l, flags);
            let names = build_shapes(l, j, &list);
            install(l, j, Some(cf_conv), REG_CONV);
            let mut s = String::new();
            for (i, n) in names.iter().enumerate() {
                NAME_A.with(|c| c.set(n.as_ptr()));
                for op in 0..=17u32 {
                    OP.with(|c| c.set(op));
                    s.push_str(&format!("s{i} op{op} {}\n", run(l, j, REG_CONV)));
                }
                // js_toprimitive over all three hints (JS_HNONE/NUMBER/STRING)
                OP.with(|c| c.set(18));
                for hint in 0..3 {
                    A0.with(|c| c.set(hint));
                    s.push_str(&format!("s{i} toprim{hint} {}\n", run(l, j, REG_CONV)));
                }
                // row 163: js_touserdata with the right and a wrong tag, and
                // js_toregexp
                for (op, a0) in [(19u32, 0), (19, 1), (20, 0)] {
                    OP.with(|c| c.set(op));
                    A0.with(|c| c.set(a0));
                    s.push_str(&format!("s{i} op{op}/{a0} {}\n", run(l, j, REG_CONV)));
                }
            }
            l.js_freestate(j);
            s
        });
    }
}

/* ================================================= row 162: the try limit */

/// Recurses through `js_pcall` (one `js_try` per level) until `trytop` reaches
/// JS_TRYLIMIT, then runs the four `js_ptry`-guarded conversions.
unsafe extern "C" fn cf_try(j: JS) {
    let l = cur();
    let d = DEPTH.with(|c| c.get());
    if d > 0 {
        DEPTH.with(|c| c.set(d - 1));
        l.js_getregistry(j, REG_TRY);
        l.js_pushundefined(j);
        let rc = l.js_pcall(j, 0);
        if rc != 0 {
            out_push(format!(" [d{d} rc={rc}]").as_bytes());
        }
        l.js_pop(j, 1);
        l.js_pushundefined(j);
        return;
    }
    l.js_getregistry(j, NAME_A.with(|c| c.get()));
    out_push(
        format!(
            " limit: str={:?} num={:016x} int={} bool={} top={}",
            from_c(l.js_trystring(j, -1, ERRSTR)),
            l.js_trynumber(j, -1, -12345.5).to_bits(),
            l.js_tryinteger(j, -1, -777),
            l.js_tryboolean(j, -1, 7),
            l.js_gettop(j)
        )
        .as_bytes(),
    );
    l.js_pushundefined(j);
}

/// Rows 160-162: the try* conversions on a well-behaved value, on a value whose
/// valueOf/toString throws, and with `trytop` already at JS_TRYLIMIT (64).
#[test]
fn t_try_conversions_at_limit() {
    with_big_stack(body_t_try_conversions_at_limit);
}

fn body_t_try_conversions_at_limit() {
    let list: Vec<Mk> = vec![
        Mk::Js("1.5"),
        Mk::Js("'abc'"),
        Mk::Js("undefined"),
        Mk::Js("({toString:function(){throw 'TS!'}})"),
        Mk::Js("({valueOf:function(){throw 'VO!'}})"),
        Mk::Js("Object.create(null)"),
        Mk::Js("({})"),
        Mk::Nat(29),
    ];
    for flags in [0, JS_STRICT] {
        let list = list.clone();
        diff2(&format!("trylimit flags={flags}"), move |l| unsafe {
            let j = new_state(l, flags);
            let names = build_shapes(l, j, &list);
            install(l, j, Some(cf_try), REG_TRY);
            let mut s = String::new();
            // 60..66 straddles JS_TRYLIMIT (64): one js_try per nested js_pcall
            for depth in [0i32, 1, 60, 61, 62, 63, 64, 66] {
                for (i, n) in names.iter().enumerate() {
                    NAME_A.with(|c| c.set(n.as_ptr()));
                    DEPTH.with(|c| c.set(depth));
                    s.push_str(&format!("d{depth} s{i} {}\n", run(l, j, REG_TRY)));
                }
            }
            l.js_freestate(j);
            s
        });
    }
}

/* ========================================== row 159 + out-of-range indices */

/// Row 159 plus row 45: every predicate over every value shape, at in-range
/// (positive and negative) and out-of-range indices.
#[test]
fn t_predicates() {
    let list = shapes_full();
    diff2("predicates", move |l| unsafe {
        let j = new_state(l, 0);
        let n = list.len() as c_int;
        for m in &list {
            build_one(l, j, m);
        }
        let mut s = format!("built {n} top={}\n", l.js_gettop(j));
        let mut idxs: Vec<c_int> = (0..n).collect();
        idxs.extend((1..=n).map(|i| -i));
        idxs.extend([n, n + 1, n + 500, -(n + 1), -(n + 500), 4100, -4100, 100000]);
        for idx in idxs {
            let mut bits = String::new();
            for name in PREDS {
                bits.push(if l.pred(name, j, idx) != 0 { '1' } else { '0' });
            }
            let ua = l.js_isuserdata(j, idx, UD_A);
            let ub = l.js_isuserdata(j, idx, UD_B);
            let um = l.js_isuserdata(j, idx, UD_MISS);
            // A NULL tag is only well defined when the value is not userdata
            // (jsrun.c:266 reaches strcmp only for JS_CUSERDATA). The only
            // userdata this file creates carries UD_A or UD_B, so ua|ub == 0
            // proves the value is not userdata.
            let unull = if ua == 0 && ub == 0 {
                format!("{}", l.js_isuserdata(j, idx, std::ptr::null()))
            } else {
                "skip".to_string()
            };
            s.push_str(&format!(
                "{idx}: {bits} ud={ua}{ub}{um}/{unull} t={} ty={}\n",
                l.js_type(j, idx),
                from_c(l.js_typeof(j, idx))
            ));
        }
        s.push_str(&format!("gettop={}\n", l.js_gettop(j)));
        l.js_freestate(j);
        s
    });
}

/* ======================================================= rows 169-182 */

/// One comparison over one pair of registry-parked shapes.
///
/// `js_compare`'s string arm returns `strcmp`'s value, whose magnitude is
/// implementation defined (jsvalue.c:636); only the sign is part of the
/// contract, so only the sign is compared.
unsafe extern "C" fn cf_cmp(j: JS) {
    let l = cur();
    l.js_getregistry(j, NAME_A.with(|c| c.get()));
    l.js_getregistry(j, NAME_B.with(|c| c.get()));
    let op = OP.with(|c| c.get());
    let r: String = match op {
        0 => {
            OKAY.with(|c| c.set(-1));
            let v = l.js_compare(j, OKAY.with(|c| c.as_ptr()));
            format!("compare={} okay={}", v.signum(), OKAY.with(|c| c.get()))
        }
        1 => format!("equal={}", l.nullary_i("js_equal", j)),
        2 => format!("strictequal={}", l.nullary_i("js_strictequal", j)),
        3 => format!("instanceof={}", l.nullary_i("js_instanceof", j)),
        4 => {
            l.nullary("js_concat", j);
            format!(
                "concat v={} repr={:?}",
                vdesc(l, j, -1),
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            )
        }
        _ => "?".to_string(),
    };
    let top = l.js_gettop(j);
    let a = if top >= 2 {
        slot(l, j, -2)
    } else {
        "<gone>".to_string()
    };
    let b = if top >= 1 {
        slot(l, j, -1)
    } else {
        "<gone>".to_string()
    };
    out_push(format!("{r} top={top} a={a} b={b}").as_bytes());
    l.js_pushundefined(j);
}

/// Rows 169-182: `js_compare` (including its `okay` out-parameter), `js_equal`,
/// `js_strictequal`, `js_instanceof` and `js_concat` over the full cross product
/// of the shape subset. `js_instanceof` throws for a non-callable right-hand
/// side and `js_concat` allocates, so both run protected.
#[test]
fn t_comparisons() {
    let list = shapes_cross();
    for flags in [0, JS_STRICT] {
        let list = list.clone();
        diff2(&format!("comparisons flags={flags}"), move |l| unsafe {
            let j = new_state(l, flags);
            let names = build_shapes(l, j, &list);
            install(l, j, Some(cf_cmp), REG_CMP);
            let mut s = String::new();
            for (ia, na) in names.iter().enumerate() {
                for (ib, nb) in names.iter().enumerate() {
                    NAME_A.with(|c| c.set(na.as_ptr()));
                    NAME_B.with(|c| c.set(nb.as_ptr()));
                    for op in 0..=4u32 {
                        OP.with(|c| c.set(op));
                        s.push_str(&format!("{ia}x{ib}o{op} {}\n", run(l, j, REG_CMP)));
                    }
                }
            }
            l.js_freestate(j);
            s
        });
    }
}

/* ============================================================ rows 164-168 */

/// Row 168: `js_torepr(J, idx)` pushes the repr and then
/// `js_replace(J, idx < 0 ? idx-1 : idx)`, i.e. it overwrites the very slot it
/// was asked about; an out-of-range `idx` makes that `js_replace` raise
/// js_error "stack error!" (jsrepr.c:268-273), so it must run protected.
unsafe extern "C" fn cf_torepr(j: JS) {
    let l = cur();
    for k in 0..4 {
        l.js_pushnumber(j, 10.0 + k as f64);
    }
    l.js_pushliteral(j, LIT_15);
    l.js_newarray(j);
    out_push(format!("pre {}", stack_snap(l, j)).as_bytes());
    let idx = A0.with(|c| c.get());
    let r = match OP.with(|c| c.get()) {
        0 => format!("torepr={:?}", from_c(l.js_torepr(j, idx))),
        1 => format!("tryrepr={:?}", from_c(l.js_tryrepr(j, idx, ERRSTR))),
        _ => {
            l.js_repr(j, idx);
            format!("repr pushed={:?}", from_c(l.js_tostring(j, -1)))
        }
    };
    out_push(format!(" {r} post {}", stack_snap(l, j)).as_bytes());
    l.js_pushundefined(j);
}

/// Rows 164/166/168: `js_repr` / `js_torepr` / `js_tryrepr`, which stack slot
/// each of them touches, and the cycle guard.
#[test]
fn t_repr_family() {
    diff2("repr replace", |l| unsafe {
        let j = new_state(l, 0);
        install(l, j, Some(cf_torepr), REG_OP);
        let mut s = String::new();
        for op in 0..3u32 {
            for idx in [-9i32, -7, -6, -3, -1, 0, 1, 5, 6, 7, 9] {
                OP.with(|c| c.set(op));
                A0.with(|c| c.set(idx));
                s.push_str(&format!("op{op} idx={idx} {}\n", run(l, j, REG_OP)));
            }
        }
        l.js_freestate(j);
        s
    });

    for src in [
        "var o={}; o.self=o; o",
        "var a=[1]; a.push(a); a",
        "var a=[]; a[0]=a; a[1]=a; a",
        "var o={a:{b:{}}}; o.a.b.top=o; o",
        "var a=[1,2]; delete a[0]; a",
        "new Error('boom')",
        "var e=new Error(); delete e.message; e",
        "var e=new Error('x'); e.name=''; e",
        "var e=new Error('x'); e.name={toString:function(){return 'N'}}; e",
        "({'a b':1, '0':2, '01':3, _x:4, $y:5, '':6})",
        "'\\u0000\\u0001\\b\\f\\n\\r\\t\"\\\\\\u00e9\\u4e2d'",
    ] {
        diff2(&format!("repr {src}"), move |l| unsafe {
            let j = new_state(l, 0);
            let cs = cstr(src);
            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
            let mut s = format!("load={rc}");
            if rc == 0 {
                l.js_pushundefined(j);
                let rc = l.js_pcall(j, 0);
                s.push_str(&format!(
                    " call={rc} tryrepr={:?}",
                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                ));
                l.js_pop(j, 1);
            } else {
                l.js_pop(j, 1);
            }
            s.push_str(&format!(" top={}", l.js_gettop(j)));
            l.js_freestate(j);
            s
        });
    }
}


/// Guards the shape-list sizes the rest of the file's coverage claims rest on.
#[test]
fn t_shape_inventory() {
    assert_eq!(shapes_full().len(), 152, "shape inventory shrank");
    assert!(
        shapes_cross().len() >= 40,
        "rows 169-182 want at least a 40x40 cross product"
    );
    println!(
        "shapes_full={} shapes_cross={}",
        shapes_full().len(),
        shapes_cross().len()
    );
}
