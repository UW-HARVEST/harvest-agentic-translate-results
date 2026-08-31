// Shared harness: loads the C libmujs.so and the Rust libmujs.so through
// libloading so that *both* implementations are exercised purely through their
// exported C ABI symbols.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Impls {
    pub c: Library,
    pub rust: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("MUJS_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libmujs.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("MUJS_RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir().join("target/release/libmujs.so");
    if rel.exists() {
        return rel;
    }
    manifest_dir().join("target/debug/libmujs.so")
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

/// The C library is linked without `-lm`, so `ceil`/`floor`/... are undefined in
/// it. Load libm with RTLD_GLOBAL first so those resolve at dlopen time.
fn preload_libm() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
    for name in ["libm.so.6", "libm.so"] {
        if let Ok(lib) = unsafe { UnixLibrary::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
            std::mem::forget(lib);
            return;
        }
    }
}

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        preload_libm();
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C shared library not found at {:?}", cp);
        assert!(rp.exists(), "Rust shared library not found at {:?}", rp);
        unsafe {
            Impls {
                c: Library::new(&cp).expect("load C .so"),
                rust: Library::new(&rp).expect("load Rust .so"),
            }
        }
    })
}

/// Fetch the same symbol from both libraries.
pub unsafe fn both<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let i = impls();
    let c: Symbol<T> = unsafe {
        i.c.get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"))
    };
    let r: Symbol<T> = unsafe {
        i.rust
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"))
    };
    (c, r)
}

pub unsafe fn cstr_to_string(p: *const std::os::raw::c_char) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(
            unsafe { std::ffi::CStr::from_ptr(p) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

pub unsafe fn cstr_to_bytes(p: *const std::os::raw::c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

// ---------------------------------------------------------------------------
// js_State level harness
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int, c_short, c_ushort, c_void};

pub enum JsStateOpaque {}
pub type JsPtr = *mut JsStateOpaque;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Side {
    C,
    Rust,
}

thread_local! {
    static REPORTS_C: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static REPORTS_R: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static OUT_C: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static OUT_R: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static CUR_C: RefCell<Option<Vm>> = RefCell::new(None);
    static CUR_R: RefCell<Option<Vm>> = RefCell::new(None);
}

macro_rules! decl_vm {
    ( $( $field:ident : $name:literal : $ty:ty ),* $(,)? ) => {
        #[derive(Clone, Copy)]
        pub struct Vm {
            pub side: Side,
            $( pub $field: $ty, )*
        }
        impl Vm {
            pub fn new(side: Side) -> Vm {
                let i = impls();
                let lib = match side { Side::C => &i.c, Side::Rust => &i.rust };
                unsafe {
                    Vm {
                        side,
                        $( $field: *lib
                            .get::<$ty>(concat!($name, "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("missing {}: {}", $name, e)), )*
                    }
                }
            }
        }
    };
}

decl_vm! {
    newstate: "js_newstate": unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> JsPtr,
    freestate: "js_freestate": unsafe extern "C-unwind" fn(JsPtr),
    gc: "js_gc": unsafe extern "C-unwind" fn(JsPtr, c_int),
    setlimit: "js_setlimit": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int),
    setreport: "js_setreport": unsafe extern "C-unwind" fn(JsPtr, Option<unsafe extern "C-unwind" fn(JsPtr, *const c_char)>),
    dostring: "js_dostring": unsafe extern "C-unwind" fn(JsPtr, *const c_char) -> c_int,
    ploadstring: "js_ploadstring": unsafe extern "C-unwind" fn(JsPtr, *const c_char, *const c_char) -> c_int,
    pcall: "js_pcall": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    pconstruct: "js_pconstruct": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    gettop: "js_gettop": unsafe extern "C-unwind" fn(JsPtr) -> c_int,
    pop: "js_pop": unsafe extern "C-unwind" fn(JsPtr, c_int),
    rot: "js_rot": unsafe extern "C-unwind" fn(JsPtr, c_int),
    copy: "js_copy": unsafe extern "C-unwind" fn(JsPtr, c_int),
    remove: "js_remove": unsafe extern "C-unwind" fn(JsPtr, c_int),
    insert: "js_insert": unsafe extern "C-unwind" fn(JsPtr, c_int),
    replace: "js_replace": unsafe extern "C-unwind" fn(JsPtr, c_int),
    dup: "js_dup": unsafe extern "C-unwind" fn(JsPtr),
    dup2: "js_dup2": unsafe extern "C-unwind" fn(JsPtr),
    rot2: "js_rot2": unsafe extern "C-unwind" fn(JsPtr),
    rot3: "js_rot3": unsafe extern "C-unwind" fn(JsPtr),
    rot4: "js_rot4": unsafe extern "C-unwind" fn(JsPtr),
    rot2pop1: "js_rot2pop1": unsafe extern "C-unwind" fn(JsPtr),
    rot3pop2: "js_rot3pop2": unsafe extern "C-unwind" fn(JsPtr),
    pushundefined: "js_pushundefined": unsafe extern "C-unwind" fn(JsPtr),
    pushnull: "js_pushnull": unsafe extern "C-unwind" fn(JsPtr),
    pushboolean: "js_pushboolean": unsafe extern "C-unwind" fn(JsPtr, c_int),
    pushnumber: "js_pushnumber": unsafe extern "C-unwind" fn(JsPtr, c_double),
    pushstring: "js_pushstring": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    pushlstring: "js_pushlstring": unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int),
    pushliteral: "js_pushliteral": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    pushglobal: "js_pushglobal": unsafe extern "C-unwind" fn(JsPtr),
    pushvalue: "js_pushvalue": unsafe extern "C-unwind" fn(JsPtr, c_int),
    newobject: "js_newobject": unsafe extern "C-unwind" fn(JsPtr),
    newarray: "js_newarray": unsafe extern "C-unwind" fn(JsPtr),
    newboolean: "js_newboolean": unsafe extern "C-unwind" fn(JsPtr, c_int),
    newnumber: "js_newnumber": unsafe extern "C-unwind" fn(JsPtr, c_double),
    newstring: "js_newstring": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    newregexp: "js_newregexp": unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int),
    newcfunction: "js_newcfunction": unsafe extern "C-unwind" fn(JsPtr, unsafe extern "C-unwind" fn(JsPtr), *const c_char, c_int),
    getglobal: "js_getglobal": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    setglobal: "js_setglobal": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    defglobal: "js_defglobal": unsafe extern "C-unwind" fn(JsPtr, *const c_char, c_int),
    delglobal: "js_delglobal": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    hasproperty: "js_hasproperty": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char) -> c_int,
    getproperty: "js_getproperty": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char),
    setproperty: "js_setproperty": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char),
    defproperty: "js_defproperty": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char, c_int),
    delproperty: "js_delproperty": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char),
    getlength: "js_getlength": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    setlength: "js_setlength": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int),
    hasindex: "js_hasindex": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int) -> c_int,
    getindex: "js_getindex": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int),
    setindex: "js_setindex": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int),
    delindex: "js_delindex": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int),
    isdefined: "js_isdefined": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isundefined: "js_isundefined": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isnull: "js_isnull": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isboolean: "js_isboolean": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isnumber: "js_isnumber": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isstring: "js_isstring": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isprimitive: "js_isprimitive": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isobject: "js_isobject": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isarray: "js_isarray": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isregexp: "js_isregexp": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    iscoercible: "js_iscoercible": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    iscallable: "js_iscallable": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    iserror: "js_iserror": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isnumberobject: "js_isnumberobject": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isstringobject: "js_isstringobject": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isbooleanobject: "js_isbooleanobject": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    isdateobject: "js_isdateobject": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    tryboolean: "js_tryboolean": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int) -> c_int,
    trynumber: "js_trynumber": unsafe extern "C-unwind" fn(JsPtr, c_int, c_double) -> c_double,
    tryinteger: "js_tryinteger": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int) -> c_int,
    trystring: "js_trystring": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char) -> *const c_char,
    tryrepr: "js_tryrepr": unsafe extern "C-unwind" fn(JsPtr, c_int, *const c_char) -> *const c_char,
    toboolean: "js_toboolean": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    tonumber: "js_tonumber": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_double,
    tostring: "js_tostring": unsafe extern "C-unwind" fn(JsPtr, c_int) -> *const c_char,
    tointeger: "js_tointeger": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    toint32: "js_toint32": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    touint32: "js_touint32": unsafe extern "C-unwind" fn(JsPtr, c_int) -> u32,
    toint16: "js_toint16": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_short,
    touint16: "js_touint16": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_ushort,
    typeof_: "js_typeof": unsafe extern "C-unwind" fn(JsPtr, c_int) -> *const c_char,
    type_: "js_type": unsafe extern "C-unwind" fn(JsPtr, c_int) -> c_int,
    concat: "js_concat": unsafe extern "C-unwind" fn(JsPtr),
    compare: "js_compare": unsafe extern "C-unwind" fn(JsPtr, *mut c_int) -> c_int,
    equal: "js_equal": unsafe extern "C-unwind" fn(JsPtr) -> c_int,
    strictequal: "js_strictequal": unsafe extern "C-unwind" fn(JsPtr) -> c_int,
    instanceof: "js_instanceof": unsafe extern "C-unwind" fn(JsPtr) -> c_int,
    repr: "js_repr": unsafe extern "C-unwind" fn(JsPtr, c_int),
    ref_: "js_ref": unsafe extern "C-unwind" fn(JsPtr) -> *const c_char,
    unref: "js_unref": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    getregistry: "js_getregistry": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    setregistry: "js_setregistry": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    delregistry: "js_delregistry": unsafe extern "C-unwind" fn(JsPtr, *const c_char),
    pushiterator: "js_pushiterator": unsafe extern "C-unwind" fn(JsPtr, c_int, c_int),
    nextiterator: "js_nextiterator": unsafe extern "C-unwind" fn(JsPtr, c_int) -> *const c_char,
    intern: "js_intern": unsafe extern "C-unwind" fn(JsPtr, *const c_char) -> *const c_char,
    utflen: "js_utflen": unsafe extern "C-unwind" fn(*const c_char) -> c_int,
    isarrayindex: "js_isarrayindex": unsafe extern "C-unwind" fn(JsPtr, *const c_char, *mut c_int) -> c_int,
}

/// The `Vm` currently associated with the live `Session` for `side`.
pub fn current_vm(side: Side) -> Vm {
    match side {
        Side::C => CUR_C.with(|c| c.borrow().expect("no current C vm")),
        Side::Rust => CUR_R.with(|c| c.borrow().expect("no current Rust vm")),
    }
}

unsafe extern "C-unwind" fn report_c(_j: JsPtr, msg: *const c_char) {
    let s = unsafe { cstr_to_string(msg) }.unwrap_or_default();
    REPORTS_C.with(|r| r.borrow_mut().push(s));
}

unsafe extern "C-unwind" fn report_r(_j: JsPtr, msg: *const c_char) {
    let s = unsafe { cstr_to_string(msg) }.unwrap_or_default();
    REPORTS_R.with(|r| r.borrow_mut().push(s));
}

/// A `print`-alike registered as a C function in the interpreter. It reads its
/// arguments back through the *same* library it belongs to, exercising the
/// cfunction calling convention on both sides.
unsafe extern "C-unwind" fn print_c(j: JsPtr) {
    let vm = CUR_C.with(|c| c.borrow().expect("no current C vm"));
    let s = collect_print_args(&vm, j);
    OUT_C.with(|o| o.borrow_mut().push(s));
    unsafe { (vm.pushundefined)(j) };
}

unsafe extern "C-unwind" fn print_r(j: JsPtr) {
    let vm = CUR_R.with(|c| c.borrow().expect("no current Rust vm"));
    let s = collect_print_args(&vm, j);
    OUT_R.with(|o| o.borrow_mut().push(s));
    unsafe { (vm.pushundefined)(j) };
}

fn collect_print_args(vm: &Vm, j: JsPtr) -> String {
    let top = unsafe { (vm.gettop)(j) };
    let mut parts: Vec<String> = Vec::new();
    for i in 1..top {
        let p = unsafe { (vm.tostring)(j, i) };
        parts.push(unsafe { cstr_to_string(p) }.unwrap_or_default());
    }
    parts.join(" ")
}

pub struct Session {
    pub vm: Vm,
    pub j: JsPtr,
}

impl Session {
    pub fn new(side: Side, flags: c_int) -> Session {
        let vm = Vm::new(side);
        let j = unsafe { (vm.newstate)(std::ptr::null_mut(), std::ptr::null_mut(), flags) };
        assert!(!j.is_null(), "js_newstate returned NULL");
        match side {
            Side::C => {
                unsafe { (vm.setreport)(j, Some(report_c)) };
                REPORTS_C.with(|r| r.borrow_mut().clear());
                OUT_C.with(|r| r.borrow_mut().clear());
                CUR_C.with(|c| *c.borrow_mut() = Some(vm));
            }
            Side::Rust => {
                unsafe { (vm.setreport)(j, Some(report_r)) };
                REPORTS_R.with(|r| r.borrow_mut().clear());
                OUT_R.with(|r| r.borrow_mut().clear());
                CUR_R.with(|c| *c.borrow_mut() = Some(vm));
            }
        }
        Session { vm, j }
    }

    /// Register the `print` builtin.
    ///
    /// NOTE: js_newcfunction stores the `name` pointer *without copying it*, so
    /// the string must outlive the state. Use a `'static` NUL-terminated name.
    pub fn register_print(&self) {
        const NAME: &[u8] = b"print\0";
        let f: unsafe extern "C-unwind" fn(JsPtr) = match self.vm.side {
            Side::C => print_c,
            Side::Rust => print_r,
        };
        unsafe { (self.vm.newcfunction)(self.j, f, NAME.as_ptr() as *const c_char, 1) };
        unsafe { (self.vm.setglobal)(self.j, NAME.as_ptr() as *const c_char) };
    }

    pub fn reports(&self) -> Vec<String> {
        match self.vm.side {
            Side::C => REPORTS_C.with(|r| r.borrow().clone()),
            Side::Rust => REPORTS_R.with(|r| r.borrow().clone()),
        }
    }

    pub fn output(&self) -> Vec<String> {
        match self.vm.side {
            Side::C => OUT_C.with(|r| r.borrow().clone()),
            Side::Rust => OUT_R.with(|r| r.borrow().clone()),
        }
    }

    pub fn clear_logs(&self) {
        match self.vm.side {
            Side::C => {
                REPORTS_C.with(|r| r.borrow_mut().clear());
                OUT_C.with(|r| r.borrow_mut().clear());
            }
            Side::Rust => {
                REPORTS_R.with(|r| r.borrow_mut().clear());
                OUT_R.with(|r| r.borrow_mut().clear());
            }
        }
    }

    pub fn top(&self) -> c_int {
        unsafe { (self.vm.gettop)(self.j) }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { (self.vm.freestate)(self.j) };
        match self.vm.side {
            Side::C => CUR_C.with(|c| *c.borrow_mut() = None),
            Side::Rust => CUR_R.with(|c| *c.borrow_mut() = None),
        }
    }
}

/// Outcome of running a script.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RunResult {
    pub load_error: bool,
    pub call_error: bool,
    pub value: Option<String>,
    pub reports: Vec<String>,
    pub output: Vec<String>,
    pub top_after: c_int,
}

/// Compile and run `src`; capture the completion value (as a string), any
/// reported errors, and any `print` output.
pub fn run_script(s: &Session, src: &str) -> RunResult {
    s.clear_logs();
    let vm = &s.vm;
    let j = s.j;
    let fname = CString::new("test.js").unwrap();
    let csrc = CString::new(src).unwrap();
    let err = CString::new("<<error>>").unwrap();

    let top0 = unsafe { (vm.gettop)(j) };
    let le = unsafe { (vm.ploadstring)(j, fname.as_ptr(), csrc.as_ptr()) };
    if le != 0 {
        let msg = unsafe { (vm.trystring)(j, -1, err.as_ptr()) };
        let m = unsafe { cstr_to_string(msg) };
        unsafe { (vm.pop)(j, (vm.gettop)(j) - top0) };
        return RunResult {
            load_error: true,
            call_error: false,
            value: m,
            reports: s.reports(),
            output: s.output(),
            top_after: unsafe { (vm.gettop)(j) },
        };
    }
    unsafe { (vm.pushundefined)(j) };
    let ce = unsafe { (vm.pcall)(j, 0) };
    let msg = unsafe { (vm.tryrepr)(j, -1, err.as_ptr()) };
    let m = unsafe { cstr_to_string(msg) };
    let extra = unsafe { (vm.gettop)(j) } - top0;
    if extra > 0 {
        unsafe { (vm.pop)(j, extra) };
    }
    RunResult {
        load_error: false,
        call_error: ce != 0,
        value: m,
        reports: s.reports(),
        output: s.output(),
        top_after: unsafe { (vm.gettop)(j) },
    }
}

/// Run the same script on both implementations and assert identical behaviour.
pub fn assert_same_script(src: &str, flags: c_int) {
    let cs = Session::new(Side::C, flags);
    cs.register_print();
    let rs = Session::new(Side::Rust, flags);
    rs.register_print();
    let a = run_script(&cs, src);
    let b = run_script(&rs, src);
    assert_eq!(a, b, "script mismatch (flags={}):\n---\n{}\n---", flags, src);
}

// ---------------------------------------------------------------------------
// Running arbitrary C-API sequences inside a protected (try) frame, so that
// operations which raise JS errors can be compared instead of aborting.
// ---------------------------------------------------------------------------

type Action = std::rc::Rc<dyn Fn(&Vm, JsPtr)>;

thread_local! {
    static ACTION_C: RefCell<Option<Action>> = RefCell::new(None);
    static ACTION_R: RefCell<Option<Action>> = RefCell::new(None);
}

unsafe extern "C-unwind" fn action_c(j: JsPtr) {
    let vm = CUR_C.with(|c| c.borrow().expect("no current C vm"));
    // NOTE: clone the Rc out first. On the C side a JS error is a longjmp,
    // which skips Rust destructors, so no RefCell guard may be alive here.
    let f = ACTION_C.with(|a| a.borrow().clone()).expect("no action set");
    f(&vm, j);
    unsafe { (vm.pushundefined)(j) };
}

unsafe extern "C-unwind" fn action_r(j: JsPtr) {
    let vm = CUR_R.with(|c| c.borrow().expect("no current Rust vm"));
    let f = ACTION_R.with(|a| a.borrow().clone()).expect("no action set");
    f(&vm, j);
    unsafe { (vm.pushundefined)(j) };
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProtResult {
    pub error: bool,
    pub message: Option<String>,
    pub log: Vec<String>,
}

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Append a line to the comparison log from inside an action.
pub fn logln(s: impl Into<String>) {
    LOG.with(|l| l.borrow_mut().push(s.into()));
}

/// Run `f` inside a cfunction invoked through js_pcall on `s`, capturing any
/// thrown JS error message and everything the action wrote via `logln`.
pub fn protected<F>(s: &Session, f: F) -> ProtResult
where
    F: Fn(&Vm, JsPtr) + 'static,
{
    let vm = &s.vm;
    let j = s.j;
    LOG.with(|l| l.borrow_mut().clear());
    let cb: unsafe extern "C-unwind" fn(JsPtr) = match vm.side {
        Side::C => {
            ACTION_C.with(|a| *a.borrow_mut() = Some(std::rc::Rc::new(f)));
            action_c
        }
        Side::Rust => {
            ACTION_R.with(|a| *a.borrow_mut() = Some(std::rc::Rc::new(f)));
            action_r
        }
    };
    let name: &[u8] = b"action\0";
    let err = CString::new("<<error>>").unwrap();
    let top0 = unsafe { (vm.gettop)(j) };
    unsafe { (vm.newcfunction)(j, cb, name.as_ptr() as *const c_char, 0) };
    unsafe { (vm.pushundefined)(j) };
    let e = unsafe { (vm.pcall)(j, 0) };
    let msg = unsafe { (vm.trystring)(j, -1, err.as_ptr()) };
    let m = unsafe { cstr_to_string(msg) };
    let extra = unsafe { (vm.gettop)(j) } - top0;
    if extra > 0 {
        unsafe { (vm.pop)(j, extra) };
    }
    ProtResult {
        error: e != 0,
        message: m,
        log: LOG.with(|l| l.borrow().clone()),
    }
}

/// Run the same action against both implementations and compare.
pub fn assert_same_protected<F>(cs: &Session, rs: &Session, label: &str, f: F)
where
    F: Fn(&Vm, JsPtr) + Clone + 'static,
{
    let a = protected(cs, f.clone());
    let b = protected(rs, f);
    assert_eq!(a, b, "protected action mismatch: {}", label);
}

/// Snapshot the whole visible stack as strings (uses js_tryrepr so that
/// throwing getters/toString cannot abort).
pub fn stack_snapshot(vm: &Vm, j: JsPtr) -> Vec<String> {
    let err = CString::new("<<err>>").unwrap();
    let n = unsafe { (vm.gettop)(j) };
    (0..n)
        .map(|i| {
            let p = unsafe { (vm.tryrepr)(j, i, err.as_ptr()) };
            unsafe { cstr_to_string(p) }.unwrap_or_default()
        })
        .collect()
}
