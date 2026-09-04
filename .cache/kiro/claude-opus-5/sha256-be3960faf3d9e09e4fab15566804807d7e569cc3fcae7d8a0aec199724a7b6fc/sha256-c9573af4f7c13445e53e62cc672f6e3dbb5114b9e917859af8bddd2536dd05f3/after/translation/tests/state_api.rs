//! Phase B/C — differential tests that drive the **low-level C API** directly
//! through the `.so` exports: the value stack, the type predicates, the
//! coercions, the property/index/global/registry API, the iterators and the
//! operators.
//!
//! The API is exercised from inside a `js_newcfunction` callback invoked with
//! `js_pcall`, so operations that throw are observable as an error instead of
//! aborting the process. The same op-script runs against both `.so`s and the
//! full log of every intermediate observation is compared.
//!
//! CONFIGS.md rows 26-51. ERRORS.md section 4 plus the generic FFI boundaries
//! (null pointers, zero/oversized lengths, out-of-range enum values).

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

const SEED: u64 = 0x571A_7E00_0000_0001;

// ---------------------------------------------------------------------------
// Op script
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    // push / new
    PushUndefined,
    PushNull,
    PushBool(c_int),
    PushNumber(f64),
    PushString(Vec<u8>),
    PushLString(Vec<u8>, c_int),
    PushLiteral(Vec<u8>),
    PushGlobal,
    NewObject,
    NewObjectX,
    NewArray,
    NewBoolean(c_int),
    NewNumber(f64),
    NewString(Vec<u8>),
    NewRegexp(Vec<u8>, c_int),
    // predicates & types
    Type(c_int),
    TypeOf(c_int),
    Preds(c_int),
    // coercions
    ToBoolean(c_int),
    ToNumber(c_int),
    ToString(c_int),
    ToInteger(c_int),
    ToInt32(c_int),
    ToUint32(c_int),
    ToInt16(c_int),
    ToUint16(c_int),
    TryString(c_int, Vec<u8>),
    TryNumber(c_int, f64),
    TryInteger(c_int, c_int),
    TryBoolean(c_int, c_int),
    TryRepr(c_int, Vec<u8>),
    ToRepr(c_int),
    Repr(c_int),
    // stack
    GetTop,
    Pop(c_int),
    Rot(c_int),
    Copy(c_int),
    Remove(c_int),
    Insert(c_int),
    Replace(c_int),
    Dup,
    Dup2,
    Rot2,
    Rot3,
    Rot4,
    Rot2Pop1,
    Rot3Pop2,
    // properties
    HasProperty(c_int, Vec<u8>),
    GetProperty(c_int, Vec<u8>),
    SetProperty(c_int, Vec<u8>),
    DefProperty(c_int, Vec<u8>, c_int),
    DelProperty(c_int, Vec<u8>),
    DefAccessor(c_int, Vec<u8>, c_int),
    GetLength(c_int),
    SetLength(c_int, c_int),
    HasIndex(c_int, c_int),
    GetIndex(c_int, c_int),
    SetIndex(c_int, c_int),
    DelIndex(c_int, c_int),
    // globals & registry
    GetGlobal(Vec<u8>),
    SetGlobal(Vec<u8>),
    DefGlobal(Vec<u8>, c_int),
    DelGlobal(Vec<u8>),
    SetRegistry(Vec<u8>),
    GetRegistry(Vec<u8>),
    DelRegistry(Vec<u8>),
    Ref,
    // iterators
    PushIterator(c_int, c_int),
    DrainIterator(c_int),
    // operators
    Concat,
    Equal,
    StrictEqual,
    Compare,
    InstanceOf,
    // userdata
    NewUserdata(Vec<u8>),
    IsUserdata(c_int, Vec<u8>),
    ToUserdata(c_int, Vec<u8>),
    // functions
    NewCFunction(Vec<u8>, c_int),
    NewCFunctionX(Vec<u8>, c_int),
    NewCConstructor(Vec<u8>, c_int),
    // evaluate a snippet through the interpreter to build interesting values
    Eval(Vec<u8>),
}

// ---------------------------------------------------------------------------
// Trampoline plumbing. The log is a raw `*mut String` rather than a Mutex:
// a throwing API call `longjmp`s out of the callback, which would leak a
// `MutexGuard` forever. `RUN_LOCK` already serializes runs.
// ---------------------------------------------------------------------------

static mut LOG: *mut String = std::ptr::null_mut();
static mut SCRIPT: *const Vec<Op> = std::ptr::null();
static mut API: *const Api = std::ptr::null();

fn log(s: &str) {
    unsafe {
        if !LOG.is_null() {
            (*LOG).push_str(s);
            (*LOG).push('\n');
        }
    }
}

fn logf(v: f64) -> String {
    if v.is_nan() {
        "nan".into()
    } else {
        format!("{:#018x}", v.to_bits())
    }
}

fn cs(p: *const c_char) -> String {
    match unsafe { read_cstr(p) } {
        None => "<NULL>".into(),
        Some(b) => format!("{:?}", String::from_utf8_lossy(&b)),
    }
}

/// Some MuJS entry points **retain** the caller's `const char *` instead of
/// copying it: `js_pushliteral` (jsrun.c:184 stores `u.litstr = v`),
/// `js_newcfunctionx` (jsvalue.c:493 stores `u.c.name = name`) and
/// `js_newuserdatax` (stores the tag pointer). Those need a pointer that
/// outlives the state, so intern them into a leaked, deduplicated table.
fn static_cstr(bytes: &[u8]) -> *const c_char {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static TABLE: Mutex<Option<HashMap<Vec<u8>, usize>>> = Mutex::new(None);
    let mut g = TABLE.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    let key = bytes.to_vec();
    if let Some(&p) = map.get(&key) {
        return p as *const c_char;
    }
    let leaked: &'static [u8] = Box::leak(cstr_bytes(bytes).into_boxed_slice());
    let p = leaked.as_ptr() as usize;
    map.insert(key, p);
    p as *const c_char
}

static mut UD_SLOT: i32 = 0;

extern "C" fn dummy_cfun(j: JsState) {
    let api = unsafe { &*API };
    (api.js_pushnumber)(j, 42.0);
}

extern "C" fn dummy_con(j: JsState) {
    let api = unsafe { &*API };
    (api.js_newobject)(j);
}

extern "C" fn dummy_finalize(_j: JsState, _p: *mut c_void) {}

extern "C" fn run_script(j: JsState) {
    let api = unsafe { &*API };
    let script: &Vec<Op> = unsafe { &*SCRIPT };
    for (i, op) in script.iter().enumerate() {
        // Every op logs its own result; the index makes divergences locatable.
        match op {
            Op::PushUndefined => {
                (api.js_pushundefined)(j);
                log(&format!("{i} pushundefined"));
            }
            Op::PushNull => {
                (api.js_pushnull)(j);
                log(&format!("{i} pushnull"));
            }
            Op::PushBool(v) => {
                (api.js_pushboolean)(j, *v);
                log(&format!("{i} pushboolean {v}"));
            }
            Op::PushNumber(v) => {
                (api.js_pushnumber)(j, *v);
                log(&format!("{i} pushnumber {}", logf(*v)));
            }
            Op::PushString(s) => {
                let z = cstr_bytes(s);
                (api.js_pushstring)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} pushstring"));
            }
            Op::PushLString(s, n) => {
                (api.js_pushlstring)(j, s.as_ptr() as *const c_char, *n);
                log(&format!("{i} pushlstring n={n}"));
            }
            Op::PushLiteral(s) => {
                (api.js_pushliteral)(j, static_cstr(s));
                log(&format!("{i} pushliteral"));
            }
            Op::PushGlobal => {
                (api.js_pushglobal)(j);
                log(&format!("{i} pushglobal"));
            }
            Op::NewObject => {
                (api.js_newobject)(j);
                log(&format!("{i} newobject"));
            }
            Op::NewObjectX => {
                (api.js_newobjectx)(j);
                log(&format!("{i} newobjectx"));
            }
            Op::NewArray => {
                (api.js_newarray)(j);
                log(&format!("{i} newarray"));
            }
            Op::NewBoolean(v) => {
                (api.js_newboolean)(j, *v);
                log(&format!("{i} newboolean {v}"));
            }
            Op::NewNumber(v) => {
                (api.js_newnumber)(j, *v);
                log(&format!("{i} newnumber {}", logf(*v)));
            }
            Op::NewString(s) => {
                let z = cstr_bytes(s);
                (api.js_newstring)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} newstring"));
            }
            Op::NewRegexp(p, f) => {
                let z = cstr_bytes(p);
                (api.js_newregexp)(j, z.as_ptr() as *const c_char, *f);
                log(&format!("{i} newregexp flags={f}"));
            }
            Op::Type(idx) => log(&format!("{i} type({idx}) = {}", (api.js_type)(j, *idx))),
            Op::TypeOf(idx) => {
                log(&format!("{i} typeof({idx}) = {}", cs((api.js_typeof)(j, *idx))))
            }
            Op::Preds(idx) => {
                let x = *idx;
                log(&format!(
                    "{i} preds({x}) defined={} undefined={} null={} bool={} num={} str={} prim={} obj={} arr={} re={} coerce={} call={} err={} numo={} stro={} boolo={} dateo={}",
                    (api.js_isdefined)(j, x),
                    (api.js_isundefined)(j, x),
                    (api.js_isnull)(j, x),
                    (api.js_isboolean)(j, x),
                    (api.js_isnumber)(j, x),
                    (api.js_isstring)(j, x),
                    (api.js_isprimitive)(j, x),
                    (api.js_isobject)(j, x),
                    (api.js_isarray)(j, x),
                    (api.js_isregexp)(j, x),
                    (api.js_iscoercible)(j, x),
                    (api.js_iscallable)(j, x),
                    (api.js_iserror)(j, x),
                    (api.js_isnumberobject)(j, x),
                    (api.js_isstringobject)(j, x),
                    (api.js_isbooleanobject)(j, x),
                    (api.js_isdateobject)(j, x),
                ));
            }
            Op::ToBoolean(x) => log(&format!("{i} toboolean({x}) = {}", (api.js_toboolean)(j, *x))),
            Op::ToNumber(x) => log(&format!(
                "{i} tonumber({x}) = {}",
                logf((api.js_tonumber)(j, *x))
            )),
            Op::ToString(x) => log(&format!("{i} tostring({x}) = {}", cs((api.js_tostring)(j, *x)))),
            Op::ToInteger(x) => log(&format!("{i} tointeger({x}) = {}", (api.js_tointeger)(j, *x))),
            Op::ToInt32(x) => log(&format!("{i} toint32({x}) = {}", (api.js_toint32)(j, *x))),
            Op::ToUint32(x) => log(&format!("{i} touint32({x}) = {}", (api.js_touint32)(j, *x))),
            Op::ToInt16(x) => log(&format!("{i} toint16({x}) = {}", (api.js_toint16)(j, *x))),
            Op::ToUint16(x) => log(&format!("{i} touint16({x}) = {}", (api.js_touint16)(j, *x))),
            Op::TryString(x, e) => {
                let z = cstr_bytes(e);
                log(&format!(
                    "{i} trystring({x}) = {}",
                    cs((api.js_trystring)(j, *x, z.as_ptr() as *const c_char))
                ));
            }
            Op::TryNumber(x, e) => log(&format!(
                "{i} trynumber({x}) = {}",
                logf((api.js_trynumber)(j, *x, *e))
            )),
            Op::TryInteger(x, e) => log(&format!(
                "{i} tryinteger({x}) = {}",
                (api.js_tryinteger)(j, *x, *e)
            )),
            Op::TryBoolean(x, e) => log(&format!(
                "{i} tryboolean({x}) = {}",
                (api.js_tryboolean)(j, *x, *e)
            )),
            Op::TryRepr(x, e) => {
                let z = cstr_bytes(e);
                log(&format!(
                    "{i} tryrepr({x}) = {}",
                    cs((api.js_tryrepr)(j, *x, z.as_ptr() as *const c_char))
                ));
            }
            Op::ToRepr(x) => log(&format!("{i} torepr({x}) = {}", cs((api.js_torepr)(j, *x)))),
            Op::Repr(x) => {
                (api.js_repr)(j, *x);
                log(&format!("{i} repr({x})"));
            }
            Op::GetTop => log(&format!("{i} gettop = {}", (api.js_gettop)(j))),
            Op::Pop(n) => {
                (api.js_pop)(j, *n);
                log(&format!("{i} pop {n} -> top {}", (api.js_gettop)(j)));
            }
            Op::Rot(n) => {
                (api.js_rot)(j, *n);
                log(&format!("{i} rot {n}"));
            }
            Op::Copy(x) => {
                (api.js_copy)(j, *x);
                log(&format!("{i} copy {x}"));
            }
            Op::Remove(x) => {
                (api.js_remove)(j, *x);
                log(&format!("{i} remove {x}"));
            }
            Op::Insert(x) => {
                (api.js_insert)(j, *x);
                log(&format!("{i} insert {x}"));
            }
            Op::Replace(x) => {
                (api.js_replace)(j, *x);
                log(&format!("{i} replace {x}"));
            }
            Op::Dup => {
                (api.js_dup)(j);
                log(&format!("{i} dup"));
            }
            Op::Dup2 => {
                (api.js_dup2)(j);
                log(&format!("{i} dup2"));
            }
            Op::Rot2 => {
                (api.js_rot2)(j);
                log(&format!("{i} rot2"));
            }
            Op::Rot3 => {
                (api.js_rot3)(j);
                log(&format!("{i} rot3"));
            }
            Op::Rot4 => {
                (api.js_rot4)(j);
                log(&format!("{i} rot4"));
            }
            Op::Rot2Pop1 => {
                (api.js_rot2pop1)(j);
                log(&format!("{i} rot2pop1"));
            }
            Op::Rot3Pop2 => {
                (api.js_rot3pop2)(j);
                log(&format!("{i} rot3pop2"));
            }
            Op::HasProperty(x, n) => {
                let z = cstr_bytes(n);
                let r = (api.js_hasproperty)(j, *x, z.as_ptr() as *const c_char);
                log(&format!("{i} hasproperty({x},{:?}) = {r}", String::from_utf8_lossy(n)));
            }
            Op::GetProperty(x, n) => {
                let z = cstr_bytes(n);
                (api.js_getproperty)(j, *x, z.as_ptr() as *const c_char);
                log(&format!("{i} getproperty({x},{:?})", String::from_utf8_lossy(n)));
            }
            Op::SetProperty(x, n) => {
                let z = cstr_bytes(n);
                (api.js_setproperty)(j, *x, z.as_ptr() as *const c_char);
                log(&format!("{i} setproperty({x},{:?})", String::from_utf8_lossy(n)));
            }
            Op::DefProperty(x, n, a) => {
                let z = cstr_bytes(n);
                (api.js_defproperty)(j, *x, z.as_ptr() as *const c_char, *a);
                log(&format!(
                    "{i} defproperty({x},{:?},{a})",
                    String::from_utf8_lossy(n)
                ));
            }
            Op::DelProperty(x, n) => {
                let z = cstr_bytes(n);
                (api.js_delproperty)(j, *x, z.as_ptr() as *const c_char);
                log(&format!("{i} delproperty({x},{:?})", String::from_utf8_lossy(n)));
            }
            Op::DefAccessor(x, n, a) => {
                let z = cstr_bytes(n);
                (api.js_defaccessor)(j, *x, z.as_ptr() as *const c_char, *a);
                log(&format!(
                    "{i} defaccessor({x},{:?},{a})",
                    String::from_utf8_lossy(n)
                ));
            }
            Op::GetLength(x) => log(&format!("{i} getlength({x}) = {}", (api.js_getlength)(j, *x))),
            Op::SetLength(x, n) => {
                (api.js_setlength)(j, *x, *n);
                log(&format!("{i} setlength({x},{n})"));
            }
            Op::HasIndex(x, n) => {
                log(&format!("{i} hasindex({x},{n}) = {}", (api.js_hasindex)(j, *x, *n)))
            }
            Op::GetIndex(x, n) => {
                (api.js_getindex)(j, *x, *n);
                log(&format!("{i} getindex({x},{n})"));
            }
            Op::SetIndex(x, n) => {
                (api.js_setindex)(j, *x, *n);
                log(&format!("{i} setindex({x},{n})"));
            }
            Op::DelIndex(x, n) => {
                (api.js_delindex)(j, *x, *n);
                log(&format!("{i} delindex({x},{n})"));
            }
            Op::GetGlobal(n) => {
                let z = cstr_bytes(n);
                (api.js_getglobal)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} getglobal({:?})", String::from_utf8_lossy(n)));
            }
            Op::SetGlobal(n) => {
                let z = cstr_bytes(n);
                (api.js_setglobal)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} setglobal({:?})", String::from_utf8_lossy(n)));
            }
            Op::DefGlobal(n, a) => {
                let z = cstr_bytes(n);
                (api.js_defglobal)(j, z.as_ptr() as *const c_char, *a);
                log(&format!("{i} defglobal({:?},{a})", String::from_utf8_lossy(n)));
            }
            Op::DelGlobal(n) => {
                let z = cstr_bytes(n);
                (api.js_delglobal)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} delglobal({:?})", String::from_utf8_lossy(n)));
            }
            Op::SetRegistry(n) => {
                let z = cstr_bytes(n);
                (api.js_setregistry)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} setregistry({:?})", String::from_utf8_lossy(n)));
            }
            Op::GetRegistry(n) => {
                let z = cstr_bytes(n);
                (api.js_getregistry)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} getregistry({:?})", String::from_utf8_lossy(n)));
            }
            Op::DelRegistry(n) => {
                let z = cstr_bytes(n);
                (api.js_delregistry)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} delregistry({:?})", String::from_utf8_lossy(n)));
            }
            Op::Ref => {
                let r = (api.js_ref)(j);
                // The ref name is a generated counter string; log its shape only
                let s = unsafe { read_cstr(r) }.unwrap_or_default();
                log(&format!("{i} ref len={}", s.len()));
                (api.js_unref)(j, r);
                log(&format!("{i} unref"));
            }
            Op::PushIterator(x, own) => {
                (api.js_pushiterator)(j, *x, *own);
                log(&format!("{i} pushiterator({x},{own})"));
            }
            Op::DrainIterator(x) => {
                let mut names = Vec::new();
                loop {
                    let p = (api.js_nextiterator)(j, *x);
                    match unsafe { read_cstr(p) } {
                        None => break,
                        Some(b) => {
                            names.push(String::from_utf8_lossy(&b).into_owned());
                            if names.len() > 512 {
                                break;
                            }
                        }
                    }
                }
                log(&format!("{i} iterator names = {names:?}"));
            }
            Op::Concat => {
                (api.js_concat)(j);
                log(&format!("{i} concat"));
            }
            Op::Equal => log(&format!("{i} equal = {}", (api.js_equal)(j))),
            Op::StrictEqual => log(&format!("{i} strictequal = {}", (api.js_strictequal)(j))),
            Op::Compare => {
                let mut okay: c_int = -99;
                let r = (api.js_compare)(j, &mut okay);
                log(&format!("{i} compare = {r} okay={okay}"));
            }
            Op::InstanceOf => log(&format!("{i} instanceof = {}", (api.js_instanceof)(j))),
            Op::NewUserdata(tag) => {
                let slot = &raw mut UD_SLOT as *mut c_void;
                (api.js_newuserdata)(j, static_cstr(tag), slot, Some(dummy_finalize));
                log(&format!("{i} newuserdata({:?})", String::from_utf8_lossy(tag)));
            }
            Op::IsUserdata(x, tag) => {
                log(&format!(
                    "{i} isuserdata({x},{:?}) = {}",
                    String::from_utf8_lossy(tag),
                    (api.js_isuserdata)(j, *x, static_cstr(tag))
                ));
            }
            Op::ToUserdata(x, tag) => {
                let p = (api.js_touserdata)(j, *x, static_cstr(tag));
                log(&format!(
                    "{i} touserdata({x},{:?}) = {}",
                    String::from_utf8_lossy(tag),
                    if p.is_null() { "NULL" } else { "SLOT" }
                ));
            }
            Op::NewCFunction(name, len) => {
                (api.js_newcfunction)(j, dummy_cfun, static_cstr(name), *len);
                log(&format!("{i} newcfunction len={len}"));
            }
            Op::NewCFunctionX(name, len) => {
                let slot = &raw mut UD_SLOT as *mut c_void;
                (api.js_newcfunctionx)(
                    j,
                    dummy_cfun,
                    static_cstr(name),
                    *len,
                    slot,
                    Some(dummy_finalize),
                );
                log(&format!("{i} newcfunctionx len={len}"));
            }
            Op::NewCConstructor(name, len) => {
                (api.js_newcconstructor)(j, dummy_cfun, dummy_con, static_cstr(name), *len);
                log(&format!("{i} newcconstructor len={len}"));
            }
            Op::Eval(src) => {
                let z = cstr_bytes(src);
                let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
                log(&format!("{i} eval rc={rc}"));
            }
        }
    }
    // leave a well-defined return value
    (api.js_pushundefined)(j);
}

#[derive(Debug, PartialEq, Eq)]
struct ScriptResult {
    pcall_rc: c_int,
    err: Option<String>,
    log: String,
    final_top: c_int,
}

fn exec_script(api: &'static Api, flags: c_int, script: &Vec<Op>) -> ScriptResult {
    let mut logbuf = String::new();
    unsafe {
        LOG = &mut logbuf as *mut String;
        SCRIPT = script as *const Vec<Op>;
        API = api as *const Api;
    }
    let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
    assert!(!j.is_null());
    // Make `o`/`oj`/`ok`/`__out` available to any `Op::Eval` in the script.
    let pro = cstr(PROLOGUE);
    let pro_rc = (api.js_dostring)(j, pro.as_ptr() as *const c_char);
    assert_eq!(pro_rc, 0, "prologue failed to load");
    let name = cstr("script");
    (api.js_newcfunction)(j, run_script, name.as_ptr() as *const c_char, 0);
    (api.js_pushundefined)(j);
    let rc = (api.js_pcall)(j, 0);
    let err = if rc != 0 {
        let e = cstr("<err>");
        unsafe { read_cstr((api.js_trystring)(j, -1, e.as_ptr() as *const c_char)) }
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    } else {
        None
    };
    (api.js_pop)(j, 1);
    let final_top = (api.js_gettop)(j);
    (api.js_freestate)(j);
    unsafe {
        LOG = std::ptr::null_mut();
        SCRIPT = std::ptr::null();
        API = std::ptr::null();
    }
    ScriptResult {
        pcall_rc: rc,
        err,
        log: logbuf,
        final_top,
    }
}

fn assert_same_script(label: &str, flags: c_int, script: Vec<Op>) {
    let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (c, r) = both_apis();
    let a = exec_script(c, flags, &script);
    let b = exec_script(r, flags, &script);
    if a != b {
        // find the first differing log line for a compact report
        let al: Vec<&str> = a.log.lines().collect();
        let bl: Vec<&str> = b.log.lines().collect();
        let mut first = None;
        for k in 0..al.len().max(bl.len()) {
            if al.get(k) != bl.get(k) {
                first = Some(k);
                break;
            }
        }
        panic!(
            "DIVERGENCE [{label}] flags={flags}\nfirst differing log line {first:?}\n  C   : {:?}\n  RUST: {:?}\nC  rc={} err={:?} top={}\nRUST rc={} err={:?} top={}\nscript={script:?}",
            first.and_then(|k| al.get(k)),
            first.and_then(|k| bl.get(k)),
            a.pcall_rc,
            a.err,
            a.final_top,
            b.pcall_rc,
            b.err,
            b.final_top,
        );
    }
}

// ---------------------------------------------------------------------------
// Value builders used across the tests
// ---------------------------------------------------------------------------

fn all_value_pushes() -> Vec<Vec<Op>> {
    vec![
        vec![Op::PushUndefined],
        vec![Op::PushNull],
        vec![Op::PushBool(0)],
        vec![Op::PushBool(1)],
        vec![Op::PushBool(-1)],
        vec![Op::PushBool(255)],
        vec![Op::PushNumber(0.0)],
        vec![Op::PushNumber(-0.0)],
        vec![Op::PushNumber(1.0)],
        vec![Op::PushNumber(-1.5)],
        vec![Op::PushNumber(f64::NAN)],
        vec![Op::PushNumber(f64::INFINITY)],
        vec![Op::PushNumber(f64::NEG_INFINITY)],
        vec![Op::PushNumber(2147483648.0)],
        vec![Op::PushNumber(-2147483649.0)],
        vec![Op::PushNumber(4294967296.0)],
        vec![Op::PushNumber(1e21)],
        vec![Op::PushNumber(5e-324)],
        vec![Op::PushString(b"".to_vec())],
        vec![Op::PushString(b"0".to_vec())],
        vec![Op::PushString(b"1".to_vec())],
        vec![Op::PushString(b" 12 ".to_vec())],
        vec![Op::PushString(b"abc".to_vec())],
        vec![Op::PushString("\u{e9}\u{4e2d}\u{1F600}".as_bytes().to_vec())],
        vec![Op::PushString(vec![0xFF, 0xFE, 0x41])],
        vec![Op::PushString(b"0x10".to_vec())],
        vec![Op::PushString(b"Infinity".to_vec())],
        vec![Op::PushString(b"NaN".to_vec())],
        // long string: past the inline short-string representation
        vec![Op::PushString(vec![b'x'; 200])],
        vec![Op::PushLiteral(b"literal".to_vec())],
        vec![Op::NewObject],
        vec![Op::NewObjectX],
        vec![Op::NewArray],
        vec![Op::NewBoolean(1)],
        vec![Op::NewBoolean(0)],
        vec![Op::NewNumber(3.5)],
        vec![Op::NewString(b"boxed".to_vec())],
        vec![Op::NewRegexp(b"a+b".to_vec(), 0)],
        vec![Op::NewRegexp(b"a+b".to_vec(), 7)],
        vec![Op::NewCFunction(b"f".to_vec(), 1)],
        vec![Op::NewCFunctionX(b"g".to_vec(), 2)],
        vec![Op::NewCConstructor(b"C".to_vec(), 0)],
        vec![Op::NewUserdata(b"tag".to_vec())],
        vec![Op::PushGlobal],
        vec![Op::Eval(b"var v = new Date(0);".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = new Error('e');".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = [1,2,3];".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = {a:1};".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = function(a,b){return a+b};".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = {valueOf:function(){return 7}};".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = {toString:function(){return 'ts'}};".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = {valueOf:function(){throw 'boom'}};".to_vec()), Op::GetGlobal(b"v".to_vec())],
        vec![Op::Eval(b"var v = Object.create(null);".to_vec()), Op::GetGlobal(b"v".to_vec())],
    ]
}

// ===========================================================================
// CONFIGS.md rows 33, 36, 37: push, predicates, conversions over every type
// ===========================================================================

#[test]
fn api_predicates_and_conversions_over_all_values() {
    for flags in [0, 1] {
        for (n, mut push) in all_value_pushes().into_iter().enumerate() {
            let mut s = Vec::new();
            s.append(&mut push);
            s.push(Op::GetTop);
            s.push(Op::Type(-1));
            s.push(Op::TypeOf(-1));
            s.push(Op::Preds(-1));
            s.push(Op::ToBoolean(-1));
            s.push(Op::TryBoolean(-1, 42));
            s.push(Op::TryNumber(-1, -7.5));
            s.push(Op::TryInteger(-1, -3));
            s.push(Op::TryString(-1, b"<fallback>".to_vec()));
            s.push(Op::TryRepr(-1, b"<reprfallback>".to_vec()));
            assert_same_script(&format!("value#{n} nonthrowing"), flags, s);

            // Throwing coercions are exercised separately: each in its own
            // script so that a throw does not hide the following ops.
            for (name, op) in [
                ("tonumber", Op::ToNumber(-1)),
                ("tostring", Op::ToString(-1)),
                ("tointeger", Op::ToInteger(-1)),
                ("toint32", Op::ToInt32(-1)),
                ("touint32", Op::ToUint32(-1)),
                ("toint16", Op::ToInt16(-1)),
                ("touint16", Op::ToUint16(-1)),
                ("torepr", Op::ToRepr(-1)),
                ("repr", Op::Repr(-1)),
            ] {
                let mut s2 = all_value_pushes()[n].clone();
                s2.push(op);
                s2.push(Op::GetTop);
                assert_same_script(&format!("value#{n} {name}"), flags, s2);
            }
        }
    }
}

/// CONFIGS.md row 34 + generic FFI boundaries: `js_pushlstring` lengths.
#[test]
fn api_pushlstring_lengths() {
    let payload: Vec<u8> = b"ab\0cd\xff\xfe\x41gh".to_vec();
    for flags in [0, 1] {
        for n in [0i32, 1, 2, 3, 5, 9, 10] {
            let s = vec![
                Op::PushLString(payload.clone(), n),
                Op::Type(-1),
                Op::ToString(-1),
                Op::GetLength(-1),
                Op::GetTop,
            ];
            assert_same_script(&format!("pushlstring n={n}"), flags, s);
        }
        // NOTE: a negative `n` is NOT tested. `js_pushlstring` (jsrun.c:163)
        // takes the short-string branch for `n <= 15` and then runs
        // `while (n--) *s++ = *v++;`, so `n = -1` copies ~2^32 bytes over the
        // value stack. The C build corrupts memory / faults, so negative
        // lengths are outside the reachable domain (documented in ERRORS.md).
        // long strings across the short/mem string boundary
        for n in [7i32, 8, 15, 16, 17, 63, 64, 65, 255, 256] {
            let big = vec![b'q'; 400];
            let s = vec![
                Op::PushLString(big, n),
                Op::ToString(-1),
                Op::GetLength(-1),
                Op::Type(-1),
            ];
            assert_same_script(&format!("pushlstring big n={n}"), flags, s);
        }
    }
}

// ===========================================================================
// CONFIGS.md row 35: stack manipulation
// ===========================================================================

#[test]
fn api_stack_manipulation() {
    let base = || {
        vec![
            Op::PushNumber(1.0),
            Op::PushNumber(2.0),
            Op::PushNumber(3.0),
            Op::PushNumber(4.0),
            Op::PushString(b"five".to_vec()),
        ]
    };
    let dump = || {
        let mut v = vec![Op::GetTop];
        for i in -6..=6 {
            v.push(Op::TryString(i, b"?".to_vec()));
        }
        v
    };

    let ops: Vec<(&str, Vec<Op>)> = vec![
        ("dup", vec![Op::Dup]),
        ("dup2", vec![Op::Dup2]),
        ("rot2", vec![Op::Rot2]),
        ("rot3", vec![Op::Rot3]),
        ("rot4", vec![Op::Rot4]),
        ("rot2pop1", vec![Op::Rot2Pop1]),
        ("rot3pop2", vec![Op::Rot3Pop2]),
        ("pop0", vec![Op::Pop(0)]),
        ("pop1", vec![Op::Pop(1)]),
        ("pop3", vec![Op::Pop(3)]),
        ("pop5", vec![Op::Pop(5)]),
        ("rot0", vec![Op::Rot(0)]),
        ("rot1", vec![Op::Rot(1)]),
        ("rot3n", vec![Op::Rot(3)]),
        ("rot5n", vec![Op::Rot(5)]),
        ("copy-1", vec![Op::Copy(-1)]),
        ("copy-3", vec![Op::Copy(-3)]),
        ("copy0", vec![Op::Copy(0)]),
        ("copy1", vec![Op::Copy(1)]),
        ("remove-1", vec![Op::Remove(-1)]),
        ("remove-3", vec![Op::Remove(-3)]),
        ("remove1", vec![Op::Remove(1)]),
        ("insert-1", vec![Op::Insert(-1)]),
        ("insert-3", vec![Op::Insert(-3)]),
        ("insert1", vec![Op::Insert(1)]),
        ("replace-1", vec![Op::Replace(-1)]),
        ("replace-3", vec![Op::Replace(-3)]),
        ("replace1", vec![Op::Replace(1)]),
    ];

    for flags in [0, 1] {
        for (name, op) in &ops {
            let mut s = base();
            s.extend(op.clone());
            s.extend(dump());
            assert_same_script(&format!("stack {name}"), flags, s);
        }
        // `js_pop` is bounds-checked (jsrun.c:403), so underflow is observable.
        for n in [1i32, 2, 6, 7, 100] {
            let s = vec![Op::PushNumber(1.0), Op::Pop(n), Op::GetTop];
            assert_same_script(&format!("pop underflow {n}"), flags, s);
        }
        for n in [-1i32, -5] {
            let s = vec![Op::PushNumber(1.0), Op::Pop(n), Op::GetTop];
            assert_same_script(&format!("pop negative {n}"), flags, s);
        }
        // `js_rot(n)` is NOT bounds-checked (jsrun.c:498): with fewer than `n`
        // values it walks below the stack base and corrupts the heap in the C
        // build. Only depths that satisfy the op are exercised.
        for n in [0i32, 1, 2, 3, 4, 5] {
            let mut s = base(); // depth 5
            s.push(Op::Rot(n));
            s.extend(dump());
            assert_same_script(&format!("rot in-range {n}"), flags, s);
        }
        // out-of-range stack indices (ERRORS: one past the valid range)
        for idx in [-10i32, -7, -6, 0, 1, 5, 6, 10, 4096, -4096] {
            let mut s = base();
            s.push(Op::Copy(idx));
            s.push(Op::GetTop);
            assert_same_script(&format!("copy oob {idx}"), flags, s);
            let mut s = base();
            s.push(Op::Type(idx));
            assert_same_script(&format!("type oob {idx}"), flags, s);
        }
    }
}

/// ERRORS section 4: driving the stack to `JS_STACKSIZE` (4096).
#[test]
fn api_stack_overflow() {
    for flags in [0, 1] {
        for n in [4090usize, 4095, 4096, 4097, 4200] {
            let mut s: Vec<Op> = (0..n).map(|k| Op::PushNumber(k as f64)).collect();
            s.push(Op::GetTop);
            assert_same_script(&format!("stack fill {n}"), flags, s);
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 38-41: properties, attributes, indices
// ===========================================================================

#[test]
fn api_properties_all_attribute_combos() {
    for flags in [0, 1] {
        for atts in 0..8 {
            // also feed attribute values with bits outside JS_READONLY|DONTENUM|DONTCONF
            for atts in [atts, atts | 8, atts | 0x7FFF_FFF8] {
                let s = vec![
                    Op::NewObject,
                    Op::PushNumber(1.0),
                    Op::DefProperty(-2, b"p".to_vec(), atts),
                    Op::HasProperty(-1, b"p".to_vec()),
                    Op::GetProperty(-1, b"p".to_vec()),
                    Op::ToString(-1),
                    Op::Pop(1),
                    Op::PushIterator(-1, 0),
                    Op::DrainIterator(-1),
                    Op::Pop(1),
                    Op::PushIterator(-1, 1),
                    Op::DrainIterator(-1),
                    Op::Pop(1),
                    Op::PushNumber(2.0),
                    Op::SetProperty(-2, b"p".to_vec()),
                    Op::GetProperty(-1, b"p".to_vec()),
                    Op::ToString(-1),
                    Op::Pop(1),
                    Op::DelProperty(-1, b"p".to_vec()),
                    Op::HasProperty(-1, b"p".to_vec()),
                    Op::GetTop,
                ];
                assert_same_script(&format!("defproperty atts={atts}"), flags, s);

                // strict-mode writes to a read-only property throw
                let s2 = vec![
                    Op::NewObject,
                    Op::PushNumber(1.0),
                    Op::DefProperty(-2, b"p".to_vec(), atts),
                    Op::Eval(b"var strictset;".to_vec()),
                    Op::PushNumber(9.0),
                    Op::SetProperty(-2, b"p".to_vec()),
                    Op::GetProperty(-1, b"p".to_vec()),
                    Op::ToString(-1),
                ];
                assert_same_script(&format!("defproperty strictset atts={atts}"), flags, s2);

                // accessors
                let s3 = vec![
                    Op::NewObject,
                    Op::Eval(b"var G = function(){ return 'got' }; var S = function(v){ this.sv = v };".to_vec()),
                    Op::GetGlobal(b"G".to_vec()),
                    Op::GetGlobal(b"S".to_vec()),
                    Op::DefAccessor(-3, b"acc".to_vec(), atts),
                    Op::GetProperty(-1, b"acc".to_vec()),
                    Op::ToString(-1),
                    Op::Pop(1),
                    Op::PushString(b"newval".to_vec()),
                    Op::SetProperty(-2, b"acc".to_vec()),
                    Op::HasProperty(-1, b"sv".to_vec()),
                    Op::GetProperty(-1, b"sv".to_vec()),
                    Op::TryString(-1, b"?".to_vec()),
                    Op::GetTop,
                ];
                assert_same_script(&format!("defaccessor atts={atts}"), flags, s3);

                // getter only / setter only
                let s4 = vec![
                    Op::NewObject,
                    Op::Eval(b"var G = function(){ return 1 };".to_vec()),
                    Op::GetGlobal(b"G".to_vec()),
                    Op::PushUndefined,
                    Op::DefAccessor(-3, b"g".to_vec(), atts),
                    Op::GetProperty(-1, b"g".to_vec()),
                    Op::TryString(-1, b"?".to_vec()),
                    Op::Pop(1),
                    Op::PushNumber(5.0),
                    Op::SetProperty(-2, b"g".to_vec()),
                    Op::GetTop,
                ];
                assert_same_script(&format!("getter only atts={atts}"), flags, s4);

                let s5 = vec![
                    Op::NewObject,
                    Op::PushUndefined,
                    Op::Eval(b"var S = function(v){ this.sv = v };".to_vec()),
                    Op::GetGlobal(b"S".to_vec()),
                    Op::DefAccessor(-3, b"s".to_vec(), atts),
                    Op::GetProperty(-1, b"s".to_vec()),
                    Op::TryString(-1, b"?".to_vec()),
                    Op::Pop(1),
                    Op::PushNumber(5.0),
                    Op::SetProperty(-2, b"s".to_vec()),
                    Op::GetProperty(-1, b"sv".to_vec()),
                    Op::TryString(-1, b"?".to_vec()),
                ];
                assert_same_script(&format!("setter only atts={atts}"), flags, s5);
            }
        }
    }
}

#[test]
fn api_globals_all_attribute_combos() {
    for flags in [0, 1] {
        for atts in [0, 1, 2, 3, 4, 5, 6, 7, 8, -1] {
            let s = vec![
                Op::PushNumber(1.0),
                Op::DefGlobal(b"gg".to_vec(), atts),
                Op::GetGlobal(b"gg".to_vec()),
                Op::ToString(-1),
                Op::Pop(1),
                Op::PushNumber(2.0),
                Op::SetGlobal(b"gg".to_vec()),
                Op::GetGlobal(b"gg".to_vec()),
                Op::ToString(-1),
                Op::Pop(1),
                Op::Eval(b"o = typeof gg;".to_vec()),
                Op::DelGlobal(b"gg".to_vec()),
                Op::GetGlobal(b"gg".to_vec()),
                Op::Type(-1),
                Op::GetTop,
            ];
            assert_same_script(&format!("defglobal atts={atts}"), flags, s);
        }
        // missing / shadowed / weird names
        for name in [
            &b""[..],
            &b"undefined"[..],
            &b"Object"[..],
            &b"Math"[..],
            &b"NaN"[..],
            &b"Infinity"[..],
            &b"this"[..],
            &b"__proto__"[..],
            &b"a b"[..],
            "\u{4e2d}".as_bytes(),
        ] {
            let s = vec![
                Op::GetGlobal(name.to_vec()),
                Op::Type(-1),
                Op::Pop(1),
                Op::PushNumber(5.0),
                Op::SetGlobal(name.to_vec()),
                Op::GetGlobal(name.to_vec()),
                Op::TryString(-1, b"?".to_vec()),
                Op::Pop(1),
                Op::DelGlobal(name.to_vec()),
                Op::GetGlobal(name.to_vec()),
                Op::Type(-1),
            ];
            assert_same_script(
                &format!("global name {:?}", String::from_utf8_lossy(name)),
                flags,
                s,
            );
        }
    }
}

#[test]
fn api_indices_and_lengths() {
    let shapes: Vec<(&str, &[u8])> = vec![
        ("empty array", b"var v = [];"),
        ("one", b"var v = [7];"),
        ("many", b"var v = [1,2,3,4,5,6,7,8,9,10];"),
        ("sparse", b"var v = [1]; v[5] = 6;"),
        ("sparse2", b"var v = []; v[100] = 1;"),
        ("holes", b"var v = [1,,3];"),
        ("object", b"var v = {};"),
        ("object len", b"var v = {length: 4};"),
        ("string obj", b"var v = new String('abcd');"),
        ("arguments", b"var v = (function(){ return arguments })(1,2,3);"),
    ];
    for flags in [0, 1] {
        for (name, prog) in &shapes {
            let mut s = vec![Op::Eval(prog.to_vec()), Op::GetGlobal(b"v".to_vec())];
            s.push(Op::GetLength(-1));
            for i in [-1i32, 0, 1, 2, 5, 9, 10, 100, 101] {
                s.push(Op::HasIndex(-1, i));
                s.push(Op::GetIndex(-1, i));
                s.push(Op::TryString(-1, b"?".to_vec()));
                s.push(Op::Pop(1));
            }
            for i in [0i32, 3, 7, 50] {
                s.push(Op::PushNumber(i as f64 * 1.5));
                s.push(Op::SetIndex(-2, i));
            }
            s.push(Op::GetLength(-1));
            for i in [0i32, 3, 7, 50, 999] {
                s.push(Op::DelIndex(-1, i));
                s.push(Op::HasIndex(-1, i));
            }
            for n in [0i32, 1, 5, 100, -1] {
                s.push(Op::SetLength(-1, n));
                s.push(Op::GetLength(-1));
            }
            s.push(Op::PushIterator(-1, 0));
            s.push(Op::DrainIterator(-1));
            s.push(Op::Pop(1));
            s.push(Op::PushIterator(-1, 1));
            s.push(Op::DrainIterator(-1));
            s.push(Op::GetTop);
            assert_same_script(&format!("index shape {name}"), flags, s);
        }
    }
}

/// ERRORS section 4 + jsrun.c:676/707/709: array length limits.
#[test]
fn api_array_length_limits() {
    for flags in [0, 1] {
        for n in [
            0i32,
            1,
            100,
            65535,
            65536,
            (1 << 26) - 1,
            1 << 26,
            (1 << 26) + 1,
            i32::MAX,
            -1,
            i32::MIN,
        ] {
            let s = vec![
                Op::NewArray,
                Op::SetLength(-1, n),
                Op::GetLength(-1),
                Op::GetTop,
            ];
            assert_same_script(&format!("setlength {n}"), flags, s);
        }
        // via the interpreter: `new Array(n)` and `a.length = n`
        for n in [
            "0",
            "1",
            "65536",
            "67108863",
            "67108864",
            "67108865",
            "4294967295",
            "4294967296",
            "-1",
            "1.5",
            "NaN",
            "'abc'",
        ] {
            assert_same_program(
                flags,
                "array length",
                &format!("ok(function(){{ var a = new Array({n}); return a.length }});\nok(function(){{ var a=[]; a.length = {n}; return a.length }});"),
            );
        }
    }
}

// ===========================================================================
// CONFIGS.md row 42: iterators
// ===========================================================================

#[test]
fn api_iterators() {
    let setups: Vec<(&str, &[u8])> = vec![
        ("plain", b"var v = {a:1,b:2,c:3};"),
        ("array", b"var v = [1,2,3];"),
        ("sparse array", b"var v = []; v[3]=1; v[10]=2;"),
        ("proto chain", b"function P(){}; P.prototype.pp = 1; var v = new P(); v.own = 2;"),
        ("dontenum", b"var v = {}; Object.defineProperty(v, 'hidden', {value:1, enumerable:false}); v.shown = 2;"),
        ("string", b"var v = 'abc';"),
        ("string object", b"var v = new String('abc');"),
        ("number", b"var v = 5;"),
        ("boolean", b"var v = true;"),
        ("function", b"var v = function(a,b){};"),
        ("regexp", b"var v = /a/g;"),
        ("date", b"var v = new Date(0);"),
        ("math", b"var v = Math;"),
        ("global", b"var v = this;"),
        ("null proto", b"var v = Object.create(null); v.z = 1;"),
        ("many", b"var v = {}; for (var i=0;i<200;++i) v['k'+i] = i;"),
    ];
    for flags in [0, 1] {
        for (name, prog) in &setups {
            for own in [0, 1, 2, -1] {
                let s = vec![
                    Op::Eval(prog.to_vec()),
                    Op::GetGlobal(b"v".to_vec()),
                    Op::PushIterator(-1, own),
                    Op::DrainIterator(-1),
                    Op::DrainIterator(-1),
                    Op::GetTop,
                ];
                assert_same_script(&format!("iterator {name} own={own}"), flags, s);
            }
        }
        // ERRORS jsproperty.c:303 "not an iterator"
        let s = vec![Op::NewObject, Op::DrainIterator(-1)];
        assert_same_script("nextiterator on non-iterator", flags, s);
    }
}

// ===========================================================================
// CONFIGS.md rows 45-46: userdata and the registry
// ===========================================================================

#[test]
fn api_userdata_and_registry() {
    for flags in [0, 1] {
        let s = vec![
            Op::NewUserdata(b"MyTag".to_vec()),
            Op::Type(-1),
            Op::Preds(-1),
            Op::IsUserdata(-1, b"MyTag".to_vec()),
            Op::IsUserdata(-1, b"OtherTag".to_vec()),
            Op::IsUserdata(-1, b"".to_vec()),
            Op::ToUserdata(-1, b"MyTag".to_vec()),
            Op::GetTop,
        ];
        assert_same_script("userdata basics", flags, s);

        // wrong tag must throw the same TypeError
        let s = vec![
            Op::NewUserdata(b"MyTag".to_vec()),
            Op::ToUserdata(-1, b"OtherTag".to_vec()),
        ];
        assert_same_script("touserdata wrong tag", flags, s);

        // js_isuserdata / js_touserdata on non-userdata values
        for (n, mut push) in all_value_pushes().into_iter().enumerate() {
            let mut s = Vec::new();
            s.append(&mut push);
            s.push(Op::IsUserdata(-1, b"MyTag".to_vec()));
            assert_same_script(&format!("isuserdata value#{n}"), flags, s);
        }

        // registry round trip and missing key
        let s = vec![
            Op::PushString(b"regval".to_vec()),
            Op::SetRegistry(b"key1".to_vec()),
            Op::GetRegistry(b"key1".to_vec()),
            Op::TryString(-1, b"?".to_vec()),
            Op::Pop(1),
            Op::GetRegistry(b"missing".to_vec()),
            Op::Type(-1),
            Op::Pop(1),
            Op::DelRegistry(b"key1".to_vec()),
            Op::GetRegistry(b"key1".to_vec()),
            Op::Type(-1),
            Op::Pop(1),
            Op::DelRegistry(b"missing".to_vec()),
            Op::PushString(b"refme".to_vec()),
            Op::Ref,
            Op::GetTop,
        ];
        assert_same_script("registry", flags, s);
    }
}

// ===========================================================================
// CONFIGS.md rows 48-49: operators and repr
// ===========================================================================

#[test]
fn api_operators_cross_product() {
    let pushes = all_value_pushes();
    for flags in [0, 1] {
        for (i, a) in pushes.iter().enumerate() {
            for (k, b) in pushes.iter().enumerate() {
                for (name, op) in [
                    ("equal", Op::Equal),
                    ("strictequal", Op::StrictEqual),
                    ("compare", Op::Compare),
                    ("concat", Op::Concat),
                    ("instanceof", Op::InstanceOf),
                ] {
                    let mut s = a.clone();
                    s.extend(b.clone());
                    s.push(op);
                    s.push(Op::GetTop);
                    s.push(Op::TryString(-1, b"?".to_vec()));
                    assert_same_script(&format!("{name} {i}x{k}"), flags, s);
                }
            }
        }
    }
}

#[test]
fn api_repr() {
    let setups: Vec<&[u8]> = vec![
        b"var v = undefined;",
        b"var v = null;",
        b"var v = true;",
        b"var v = 1.5;",
        b"var v = 'a\"b\\'c';",
        b"var v = [1,2,[3,[4]]];",
        b"var v = {a:1,b:{c:2}};",
        b"var v = []; v[0] = v;",
        b"var v = {}; v.self = v;",
        b"var v = /re+g/gim;",
        b"var v = new Date(0);",
        b"var v = function foo(a,b){ return a };",
        b"var v = new Error('msg');",
        b"var v = Object.create(null);",
        b"var v = [undefined, null, NaN, Infinity, -0];",
        b"var v = {'\\u00e9':1, '\\u4e2d':2};",
        b"var v = Math;",
        b"var v = new Array(5);",
    ];
    for flags in [0, 1] {
        for (n, prog) in setups.iter().enumerate() {
            let s = vec![
                Op::Eval(prog.to_vec()),
                Op::GetGlobal(b"v".to_vec()),
                Op::TryRepr(-1, b"<fb>".to_vec()),
                Op::ToRepr(-1),
                Op::GetTop,
            ];
            assert_same_script(&format!("repr#{n}"), flags, s);
            let s2 = vec![
                Op::Eval(prog.to_vec()),
                Op::GetGlobal(b"v".to_vec()),
                Op::Repr(-1),
                Op::TryString(-1, b"?".to_vec()),
                Op::GetTop,
            ];
            assert_same_script(&format!("repr op#{n}"), flags, s2);
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 26-32: state creation flags, allocator, report, gc, limits
// ===========================================================================

/// A `js_Alloc` that forwards to realloc/free and counts calls.
#[repr(C)]
struct AllocStats {
    n_alloc: u64,
    n_free: u64,
    live: i64,
    limit: i64,
}

unsafe extern "C" {
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

extern "C" fn tracking_alloc(ctx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    unsafe {
        let st = &mut *(ctx as *mut AllocStats);
        if size == 0 {
            if !ptr.is_null() {
                st.n_free += 1;
                libc_free(ptr);
            }
            return std::ptr::null_mut();
        }
        if st.limit > 0 && st.live + size as i64 > st.limit {
            return std::ptr::null_mut();
        }
        st.n_alloc += 1;
        st.live += size as i64;
        libc_realloc(ptr, size as usize)
    }
}

/// CONFIGS.md rows 26-29: every `js_newstate` flag value, incl. out of range.
#[test]
fn state_flags_including_out_of_range() {
    let corpus = [
        "o(1);",
        "'use strict'; o(1);",
        "function f(){ undeclared = 1; return undeclared } ok(f);",
        "ok(function(){ with ({a:1}) { return a } });",
        "ok(function(){ return arguments.callee });",
        "ok(function(){ eval('var q = 1'); return typeof q });",
        "ok(function(){ delete Math; return typeof Math });",
        "ok(function(){ var o = {}; Object.freeze(o); o.x = 1; return o.x });",
        "ok(function(){ return (function(){ return this })() });",
        "ok(function(){ arguments = 1; });",
    ];
    for flags in [
        0,
        1,
        2,
        3,
        0xFF,
        0xFFFE,
        0xFFFF,
        -1,
        i32::MIN,
        i32::MAX,
        0x7FFF_FFFE,
    ] {
        for src in corpus {
            assert_same_program(flags, &format!("flags={flags}"), src);
        }
    }
}

/// CONFIGS.md row 28: identical behaviour under a custom allocator, and the
/// allocator call counts must match too.
#[test]
fn state_custom_allocator() {
    let (capi, rapi) = both_apis();
    let corpus = [
        "o(1+1);",
        "var a = []; for (var i=0;i<500;++i) a.push(i*i); o(a.length); o(a[499]);",
        "var s = ''; for (var i=0;i<200;++i) s += 'x'; o(s.length);",
        "var o1 = {}; for (var i=0;i<200;++i) o1['k'+i] = i; oj(Object.keys(o1).length);",
        "o(JSON.stringify({a:[1,2,3],b:'x'}));",
        "ok(function(){ return null.x });",
        "function fib(n){ return n<2?n:fib(n-1)+fib(n-2) } o(fib(18));",
    ];
    for flags in [0, 1] {
        for src in corpus {
            let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let run = |api: &Api| {
                let mut st = AllocStats {
                    n_alloc: 0,
                    n_free: 0,
                    live: 0,
                    limit: 0,
                };
                let ctx = &mut st as *mut AllocStats as *mut c_void;
                let _ = report_sink::take();
                let j = (api.js_newstate)(Some(tracking_alloc), ctx, flags);
                assert!(!j.is_null());
                (api.js_setreport)(j, Some(report_trampoline));
                let full = format!("{PROLOGUE}\n{src}\n");
                let z = cstr(&full);
                let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
                let name = cstr("__out");
                (api.js_getglobal)(j, name.as_ptr() as *const c_char);
                let out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
                    .map(|b| String::from_utf8_lossy(&b).into_owned());
                (api.js_pop)(j, 1);
                let reports = report_sink::take();
                (api.js_freestate)(j);
                (rc, out, reports, st.n_alloc, st.n_free, st.live)
            };
            let a = run(capi);
            let b = run(rapi);
            assert_eq!(
                (a.0, &a.1, &a.2),
                (b.0, &b.1, &b.2),
                "custom allocator behaviour, flags={flags}, src={src}"
            );
            assert_eq!(
                (a.3, a.4, a.5),
                (b.3, b.4, b.5),
                "custom allocator accounting, flags={flags}, src={src}: C=(alloc {} free {} live {}) RUST=(alloc {} free {} live {})",
                a.3, a.4, a.5, b.3, b.4, b.5
            );
        }
    }
}

/// CONFIGS.md row 30: `js_setreport` (already used everywhere) plus
/// `js_report` called directly, and no report callback at all.
#[test]
fn state_report_callback() {
    let (capi, rapi) = both_apis();
    for flags in [0, 1] {
        for src in ["null.x;", "throw 1;", "var = ;", "o(1);", "throw new Error('e');"] {
            // with report
            let a = run_string(capi, flags, None, std::ptr::null_mut(), true, src);
            let b = run_string(rapi, flags, None, std::ptr::null_mut(), true, src);
            assert_eq!(a, b, "report on, flags={flags}, src={src}");
            // without report: `js_report` becomes a no-op, rc must still match
            let a = run_string(capi, flags, None, std::ptr::null_mut(), false, src);
            let b = run_string(rapi, flags, None, std::ptr::null_mut(), false, src);
            assert_eq!(a, b, "report off, flags={flags}, src={src}");
        }
    }
}

/// CONFIGS.md row 31: `js_gc(J, 0)` and `js_gc(J, 1)`, plus out-of-range values.
#[test]
fn state_gc() {
    let (capi, rapi) = both_apis();
    for flags in [0, 1] {
        for report in [0, 1, 2, -1] {
            let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let run = |api: &Api| {
                let _ = report_sink::take();
                let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
                (api.js_setreport)(j, Some(report_trampoline));
                let z = cstr(&format!(
                    "{PROLOGUE}\nvar a=[]; for(var i=0;i<2000;++i) a.push({{k:i,s:'s'+i}}); a = null;"
                ));
                let rc1 = (api.js_dostring)(j, z.as_ptr() as *const c_char);
                (api.js_gc)(j, report);
                let z2 = cstr("var b=[]; for(var i=0;i<500;++i) b.push(i); __out += b.length;");
                let rc2 = (api.js_dostring)(j, z2.as_ptr() as *const c_char);
                (api.js_gc)(j, report);
                let name = cstr("__out");
                (api.js_getglobal)(j, name.as_ptr() as *const c_char);
                let out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
                    .map(|b| String::from_utf8_lossy(&b).into_owned());
                (api.js_pop)(j, 1);
                let reports = report_sink::take();
                (api.js_freestate)(j);
                (rc1, rc2, out, reports)
            };
            let a = run(capi);
            let b = run(rapi);
            // `js_gc(J,1)` prints its statistics to stdout in the C source
            // (`printf`), which we cannot capture; the report list and program
            // output must still agree exactly.
            assert_eq!(a, b, "js_gc report={report}, flags={flags}");
        }
    }
}

/// CONFIGS.md row 32 + ERRORS section 4: `js_setlimit`.
#[test]
fn state_setlimit() {
    let (capi, rapi) = both_apis();
    for flags in [0, 1] {
        for (runlimit, memlimit) in [
            (0, 0),
            (1 << 30, 1 << 30),
            (1000, 0),
            (0, 1 << 20),
            (10, 0),
            (1, 0),
            (0, 1024),
            (0, 1),
            (-1, -1),
            (i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN),
        ] {
            for src in [
                "o(1);",
                "var a=[]; for(var i=0;i<3000;++i) a.push('x'+i); o(a.length);",
                "for (var i=0;i<100000;++i) ; o('done');",
                "function f(n){ return n<=0?0:f(n-1)+1 } ok(function(){ return f(200) });",
            ] {
                let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                let run = |api: &Api| {
                    let _ = report_sink::take();
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
                    (api.js_setreport)(j, Some(report_trampoline));
                    (api.js_setlimit)(j, runlimit, memlimit);
                    let z = cstr(&format!("{PROLOGUE}\n{src}"));
                    let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
                    let name = cstr("__out");
                    (api.js_getglobal)(j, name.as_ptr() as *const c_char);
                    let out = if (api.js_isstring)(j, -1) != 0 {
                        unsafe { read_cstr((api.js_tostring)(j, -1)) }
                            .map(|b| String::from_utf8_lossy(&b).into_owned())
                    } else {
                        None
                    };
                    (api.js_pop)(j, 1);
                    let reports = report_sink::take();
                    (api.js_freestate)(j);
                    (rc, out, reports)
                };
                let a = run(capi);
                let b = run(rapi);
                assert_eq!(
                    a, b,
                    "js_setlimit({runlimit},{memlimit}) flags={flags} src={src}"
                );
            }
        }
    }
}

/// CONFIGS.md rows 50-51: `js_ploadstring` + `js_pcall` / `js_pconstruct`.
#[test]
fn state_pload_pcall_pconstruct() {
    let (capi, rapi) = both_apis();
    let programs: Vec<(&str, &str)> = vec![
        ("ok", "(function(a,b){ return String(a) + '/' + String(b) })"),
        ("throws", "(function(){ throw new Error('inner') })"),
        ("not callable", "42"),
        ("ctor", "(function(a){ this.a = a })"),
        ("ctor returns obj", "(function(){ return {r:1} })"),
        ("ctor returns prim", "(function(){ return 5 })"),
        ("syntax error", "function ("),
        ("empty", ""),
        ("strict fn", "'use strict'; (function(){ return typeof this })"),
        ("many args", "(function(){ return arguments.length })"),
    ];
    for flags in [0, 1] {
        for (label, src) in &programs {
            for nargs in [0i32, 1, 2, 3, 8, -1, -5] {
                let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                let run = |api: &Api, construct: bool| {
                    let _ = report_sink::take();
                    let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
                    (api.js_setreport)(j, Some(report_trampoline));
                    let fname = cstr("[test]");
                    let zsrc = cstr(src);
                    let load_rc = (api.js_ploadstring)(
                        j,
                        fname.as_ptr() as *const c_char,
                        zsrc.as_ptr() as *const c_char,
                    );
                    let mut call_rc = -100;
                    let mut result = None;
                    if load_rc == 0 {
                        // evaluate the script to get the function value
                        (api.js_pushundefined)(j);
                        let r0 = (api.js_pcall)(j, 0);
                        if r0 == 0 {
                            call_rc = if construct {
                                // `js_pconstruct` computes `savetop = TOP-n-2`
                                // (jsrun.c:1402) even though it only consumes
                                // the constructor plus `n` arguments, so it
                                // reclaims one slot BELOW the constructor. A
                                // spare value must therefore sit under it, or
                                // the C build writes outside the frame.
                                (api.js_pushundefined)(j); // spare slot
                                (api.js_rot2)(j); // [spare, fn]
                                for k in 0..nargs.max(0).min(8) {
                                    (api.js_pushnumber)(j, k as f64);
                                }
                                (api.js_pconstruct)(j, nargs)
                            } else {
                                (api.js_pushundefined)(j); // `this`
                                for k in 0..nargs.max(0).min(8) {
                                    (api.js_pushnumber)(j, k as f64);
                                }
                                (api.js_pcall)(j, nargs)
                            };
                            let e = cstr("<e>");
                            result = unsafe {
                                read_cstr((api.js_tryrepr)(j, -1, e.as_ptr() as *const c_char))
                            }
                            .map(|b| String::from_utf8_lossy(&b).into_owned());
                        } else {
                            call_rc = -200;
                        }
                    }
                    let reports = report_sink::take();
                    let top = (api.js_gettop)(j);
                    (api.js_freestate)(j);
                    (load_rc, call_rc, result, reports, top)
                };
                for construct in [false, true] {
                    // `js_construct` (jsrun.c:1332) has NO `n < 0` guard — only
                    // `js_call` does (jsrun.c:1303). With a negative `n` the C
                    // code sets `BOT = TOP - n - 1` above `TOP` and corrupts the
                    // value stack, so negative counts are only exercised for
                    // `js_pcall`, where the RangeError is observable.
                    if construct && nargs < 0 {
                        continue;
                    }
                    if std::env::var_os("PCALL_TRACE").is_some() {
                        use std::io::Write;
                        let mut e = std::io::stderr();
                        let _ = writeln!(
                            e,
                            "CASE label={label} construct={construct} nargs={nargs} flags={flags}"
                        );
                        let _ = e.flush();
                    }
                    let a = run(capi, construct);
                    let b = run(rapi, construct);
                    assert_eq!(
                        a, b,
                        "{label} construct={construct} nargs={nargs} flags={flags}"
                    );
                }
            }
        }
    }
}

/// CONFIGS.md row 44: cfunction data + `js_currentfunction*`.
#[test]
fn api_cfunction_data() {
    let (capi, rapi) = both_apis();
    static mut DATA: i64 = 0x1234_5678;

    extern "C" fn probe_cfun(j: JsState) {
        let api = unsafe { &*API };
        (api.js_currentfunction)(j);
        let t = (api.js_type)(j, -1);
        let d = (api.js_currentfunctiondata)(j);
        (api.js_pop)(j, 1);
        (api.js_pushnumber)(j, (t as i64 + if d.is_null() { 0 } else { 1 }) as f64);
    }

    for flags in [0, 1] {
        let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let run = |api: &'static Api| {
            unsafe {
                API = api as *const Api;
            }
            let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
            let nm = cstr("probe");
            let data = &raw mut DATA as *mut c_void;
            (api.js_newcfunctionx)(
                j,
                probe_cfun,
                nm.as_ptr() as *const c_char,
                3,
                data,
                Some(dummy_finalize),
            );
            let g = cstr("probe");
            (api.js_setglobal)(j, g.as_ptr() as *const c_char);
            let nm2 = cstr("probe2");
            (api.js_newcfunction)(j, probe_cfun, nm2.as_ptr() as *const c_char, 0);
            let g2 = cstr("probe2");
            (api.js_setglobal)(j, g2.as_ptr() as *const c_char);
            let z = cstr(&format!(
                "{PROLOGUE}\no(probe()); o(probe2()); o(probe.length); o(probe.name); o(probe2.length); o(probe2.name);"
            ));
            let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
            let n = cstr("__out");
            (api.js_getglobal)(j, n.as_ptr() as *const c_char);
            let out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            (api.js_pop)(j, 1);
            (api.js_freestate)(j);
            unsafe {
                API = std::ptr::null();
            }
            (rc, out)
        };
        let a = run(capi);
        let b = run(rapi);
        assert_eq!(a, b, "cfunction data flags={flags}");
    }
}

/// CONFIGS.md row 45: `js_newuserdatax` callbacks.
#[test]
fn api_userdatax_callbacks() {
    let (capi, rapi) = both_apis();
    static mut UD: i64 = 7;

    extern "C" fn ud_has(j: JsState, _p: *mut c_void, name: *const c_char) -> c_int {
        let api = unsafe { &*API };
        let n = unsafe { read_cstr(name) }.unwrap_or_default();
        if n == b"magic" {
            (api.js_pushnumber)(j, 99.0);
            1
        } else {
            0
        }
    }
    // The `put` callback receives the value through the C API's internal
    // `*value` argument, NOT on the value stack (jsrun.c:756/844), so it must
    // not touch the stack. Returning 1 means "handled".
    extern "C" fn ud_put(_j: JsState, _p: *mut c_void, name: *const c_char) -> c_int {
        let n = unsafe { read_cstr(name) }.unwrap_or_default();
        i32::from(n == b"magic")
    }
    extern "C" fn ud_del(_j: JsState, _p: *mut c_void, name: *const c_char) -> c_int {
        let n = unsafe { read_cstr(name) }.unwrap_or_default();
        i32::from(n == b"magic")
    }

    for flags in [0, 1] {
        for variant in 0..4 {
            let _g = RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let run = |api: &'static Api| {
                unsafe {
                    API = api as *const Api;
                }
                let _ = report_sink::take();
                let j = (api.js_newstate)(None, std::ptr::null_mut(), flags);
                (api.js_setreport)(j, Some(report_trampoline));
                let tag = cstr("UDX");
                let p = &raw mut UD as *mut c_void;
                // `js_newuserdatax` pops the top of the stack as the prototype
                // (jsvalue.c:544-546), so a value must be present first.
                (api.js_pushundefined)(j);
                (api.js_newuserdatax)(
                    j,
                    tag.as_ptr() as *const c_char,
                    p,
                    if variant & 1 != 0 { Some(ud_has) } else { None },
                    if variant & 2 != 0 { Some(ud_put) } else { None },
                    Some(ud_del),
                    Some(dummy_finalize),
                );
                let g = cstr("u");
                (api.js_setglobal)(j, g.as_ptr() as *const c_char);
                let z = cstr(&format!(
                    "{PROLOGUE}\no(u.magic); o(u.other); u.magic = 5; o(u.magic); u.other = 6; o(u.other); o(delete u.magic); o(delete u.other); o(typeof u);"
                ));
                let rc = (api.js_dostring)(j, z.as_ptr() as *const c_char);
                let n = cstr("__out");
                (api.js_getglobal)(j, n.as_ptr() as *const c_char);
                let out = unsafe { read_cstr((api.js_tostring)(j, -1)) }
                    .map(|b| String::from_utf8_lossy(&b).into_owned());
                (api.js_pop)(j, 1);
                let reports = report_sink::take();
                (api.js_freestate)(j);
                unsafe {
                    API = std::ptr::null();
                }
                (rc, out, reports)
            };
            let a = run(capi);
            let b = run(rapi);
            assert_eq!(a, b, "userdatax variant={variant} flags={flags}");
        }
    }
}

/// CONFIGS.md row 43: `js_newregexp` across all `JS_REGEXP_*` flag combos and
/// out-of-range flag values.
#[test]
fn api_newregexp_all_flags() {
    for flags in [0, 1] {
        for rf in [0, 1, 2, 3, 4, 5, 6, 7, 8, 15, -1, i32::MAX] {
            for pat in [
                &b"a"[..],
                &b"a+b"[..],
                &b"^x$"[..],
                &b"(a)(b)"[..],
                &b"["[..],
                &b"a**"[..],
                &b""[..],
            ] {
                let s = vec![
                    Op::NewRegexp(pat.to_vec(), rf),
                    Op::Type(-1),
                    Op::Preds(-1),
                    Op::SetGlobal(b"re".to_vec()),
                    Op::Eval(b"o(re.source); o(re.global); o(re.ignoreCase); o(re.multiline); o(re.lastIndex); o(String(re));".to_vec()),
                    Op::Eval(b"o(re.exec('aXbaab')); o(re.test('aab')); o('aXbaab'.replace(re,'#')); oj('a,b,a'.split(re));".to_vec()),
                    Op::GetGlobal(b"__out".to_vec()),
                    Op::TryString(-1, b"?".to_vec()),
                    Op::GetTop,
                ];
                assert_same_script(
                    &format!("newregexp /{}/ rf={rf}", String::from_utf8_lossy(pat)),
                    flags,
                    s,
                );
            }
        }
    }
}

/// Randomized op-script fuzzing: build random stacks and apply random ops.
///
/// The generator tracks the stack depth because several stack primitives are
/// unchecked in the C source (`js_rot2/3/4`, `js_rot2pop1`, `js_rot3pop2`,
/// `js_rot(n)` — jsrun.c:457-505 have no `CHECKSTACK`/underflow test). Running
/// them with too few values writes below the stack base and corrupts the C
/// heap, so the generator only emits an op when the tracked depth satisfies it.
#[test]
fn api_randomized_scripts() {
    let pushes = all_value_pushes();
    let mut rng = Rng::new(SEED);
    let names: Vec<Vec<u8>> = [
        "a", "b", "length", "0", "1", "2", "toString", "valueOf", "__proto__",
        "constructor", "", "prototype", "x y",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();

    for round in 0..3000 {
        let flags = if round % 2 == 0 { 0 } else { 1 };
        let mut s: Vec<Op> = Vec::new();
        // guarantee a floor of 4 values so the unchecked rot ops are legal
        let mut depth: i32 = 0;
        for _ in 0..4 {
            s.push(Op::PushUndefined);
            depth += 1;
        }
        let nv = 1 + rng.below(3);
        for _ in 0..nv {
            let p = &pushes[rng.below(pushes.len() as u32) as usize];
            s.extend(p.clone());
            // every entry of `all_value_pushes` nets exactly one value, except
            // the `Eval`+`GetGlobal` pairs which also net one.
            depth += 1;
        }
        let nops = 1 + rng.below(8);
        for _ in 0..nops {
            let idx = rng.range_i32(-4, 4);
            let name = names[rng.below(names.len() as u32) as usize].clone();
            // (op, minimum required depth, net depth change)
            let (op, need, delta) = match rng.below(28) {
                0 => (Op::GetTop, 0, 0),
                1 => (Op::Type(idx), 0, 0),
                2 => (Op::TypeOf(idx), 0, 0),
                3 => (Op::Preds(idx), 0, 0),
                4 => (Op::ToBoolean(idx), 0, 0),
                5 => (Op::TryNumber(idx, 1.5), 0, 0),
                6 => (Op::TryString(idx, b"fb".to_vec()), 0, 0),
                7 => (Op::TryInteger(idx, -1), 0, 0),
                8 => (Op::TryBoolean(idx, 1), 0, 0),
                9 => (Op::TryRepr(idx, b"fb".to_vec()), 0, 0),
                10 => (Op::Dup, 1, 1),
                11 => (Op::Dup2, 2, 2),
                12 => (Op::Rot2, 2, 0),
                13 => (Op::Rot3, 3, 0),
                14 => (Op::Rot4, 4, 0),
                15 => (Op::Rot2Pop1, 2, -1),
                16 => (Op::Rot3Pop2, 3, -2),
                17 => {
                    let n = rng.range_i32(0, 4);
                    (Op::Pop(n), 0, -n)
                }
                18 => {
                    let n = rng.range_i32(0, 5);
                    (Op::Rot(n), n, 0)
                }
                19 => (Op::Copy(idx), 0, 1),
                20 => (Op::Remove(idx), 1, -1),
                21 => (Op::Insert(idx), 0, 0),
                22 => (Op::Replace(idx), 1, -1),
                23 => (Op::HasProperty(idx, name), 0, 0),
                24 => (Op::GetLength(idx), 0, 0),
                25 => (Op::HasIndex(idx, rng.range_i32(-2, 4)), 0, 0),
                26 => (Op::Concat, 2, -1),
                _ => (Op::StrictEqual, 2, -2),
            };
            if depth < need.max(0) {
                continue;
            }
            let newdepth = depth + delta;
            if newdepth < 4 {
                // keep the floor so later unchecked ops stay legal
                continue;
            }
            s.push(op);
            depth = newdepth;
        }
        s.push(Op::GetTop);
        if std::env::var_os("API_TRACE").is_some() {
            use std::io::Write;
            let mut e = std::io::stderr();
            let _ = writeln!(e, "ROUND {round} flags={flags} script={s:?}");
            let _ = e.flush();
        }
        assert_same_script(&format!("random#{round}"), flags, s);
    }
}
