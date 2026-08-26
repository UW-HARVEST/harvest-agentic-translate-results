use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Once};

type State = c_void;
type Alloc = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
type Report = unsafe extern "C" fn(*mut State, *const c_char);

type NewState = unsafe extern "C" fn(Option<Alloc>, *mut c_void, c_int) -> *mut State;
type FreeState = unsafe extern "C" fn(*mut State);
type SetReport = unsafe extern "C" fn(*mut State, Option<Report>);
type DoString = unsafe extern "C" fn(*mut State, *const c_char) -> c_int;
type GetGlobal = unsafe extern "C" fn(*mut State, *const c_char);
type ToString = unsafe extern "C" fn(*mut State, c_int) -> *const c_char;
type SetContext = unsafe extern "C" fn(*mut State, *mut c_void);
type GetContext = unsafe extern "C" fn(*mut State) -> *mut c_void;
type SetLimit = unsafe extern "C" fn(*mut State, c_int, c_int);
type Gc = unsafe extern "C" fn(*mut State, c_int);

static TEST_LOCK: Mutex<()> = Mutex::new(());
static REPORTS: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
static PUSH_NUMBER: AtomicUsize = AtomicUsize::new(0);
static PRELOAD_LIBM: Once = Once::new();

unsafe extern "C" {
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

#[repr(C)]
struct AllocContext {
    calls: usize,
    fail_at: usize,
}

unsafe extern "C" fn controlled_alloc(
    context: *mut c_void,
    pointer: *mut c_void,
    size: c_int,
) -> *mut c_void {
    let context = unsafe { &mut *context.cast::<AllocContext>() };
    if size == 0 {
        unsafe { free(pointer) };
        return ptr::null_mut();
    }
    context.calls += 1;
    if context.calls == context.fail_at {
        return ptr::null_mut();
    }
    unsafe { realloc(pointer, size as usize) }
}

unsafe extern "C" fn return_42(state: *mut State) {
    type PushNumber = unsafe extern "C" fn(*mut State, f64);
    let address = PUSH_NUMBER.load(Ordering::Relaxed);
    assert_ne!(address, 0);
    let push: PushNumber = unsafe { std::mem::transmute(address) };
    unsafe { push(state, 42.0) };
}

unsafe extern "C" fn noop_finalizer(_: *mut State, _: *mut c_void) {}

unsafe extern "C" fn capture_report(_: *mut State, message: *const c_char) {
    let bytes = if message.is_null() {
        b"<null>".to_vec()
    } else {
        unsafe { CStr::from_ptr(message) }.to_bytes().to_vec()
    };
    REPORTS.lock().unwrap().push(bytes);
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libmujs.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libmujs.so")
}

fn open_library(path: impl AsRef<std::ffi::OsStr>) -> Library {
    PRELOAD_LIBM.call_once(|| unsafe {
        use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
        let libm = UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL).unwrap();
        std::mem::forget(libm);
    });
    unsafe { Library::new(path) }.unwrap()
}

unsafe fn load<T: Copy>(library: &Library, name: &[u8]) -> T {
    *unsafe { library.get::<T>(name) }.unwrap_or_else(|error| {
        panic!(
            "failed to load {}: {error}",
            String::from_utf8_lossy(name).trim_end_matches('\0')
        )
    })
}

struct Api {
    newstate: NewState,
    freestate: FreeState,
    setreport: SetReport,
    dostring: DoString,
    getglobal: GetGlobal,
    tostring: ToString,
    setcontext: SetContext,
    getcontext: GetContext,
    setlimit: SetLimit,
    gc: Gc,
    library: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        let library = open_library(path);
        Self {
            newstate: unsafe { load(&library, b"js_newstate\0") },
            freestate: unsafe { load(&library, b"js_freestate\0") },
            setreport: unsafe { load(&library, b"js_setreport\0") },
            dostring: unsafe { load(&library, b"js_dostring\0") },
            getglobal: unsafe { load(&library, b"js_getglobal\0") },
            tostring: unsafe { load(&library, b"js_tostring\0") },
            setcontext: unsafe { load(&library, b"js_setcontext\0") },
            getcontext: unsafe { load(&library, b"js_getcontext\0") },
            setlimit: unsafe { load(&library, b"js_setlimit\0") },
            gc: unsafe { load(&library, b"js_gc\0") },
            library,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ScriptResult {
    status: c_int,
    reports: Vec<Vec<u8>>,
    value: Option<Vec<u8>>,
}

unsafe fn run_script(path: &Path, flags: c_int, source: &str) -> ScriptResult {
    let api = unsafe { Api::open(path) };
    REPORTS.lock().unwrap().clear();
    let state = unsafe { (api.newstate)(None, ptr::null_mut(), flags) };
    assert!(
        !state.is_null(),
        "js_newstate failed for {}",
        path.display()
    );
    unsafe { (api.setreport)(state, Some(capture_report)) };

    let source = CString::new(source).unwrap();
    let status = unsafe { (api.dostring)(state, source.as_ptr()) };
    let value = if status == 0 {
        let result_name = c"__result";
        unsafe { (api.getglobal)(state, result_name.as_ptr()) };
        let value = unsafe { (api.tostring)(state, -1) };
        assert!(!value.is_null());
        Some(unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
    } else {
        None
    };
    let reports = REPORTS.lock().unwrap().clone();
    unsafe { (api.freestate)(state) };
    ScriptResult {
        status,
        reports,
        value,
    }
}

fn compare_script(flags: c_int, source: &str) -> ScriptResult {
    let c = unsafe { run_script(&c_library(), flags, source) };
    let rust = unsafe { run_script(&rust_library(), flags, source) };
    assert_eq!(rust, c, "script diverged:\n{source}");
    c
}

unsafe fn public_api_snapshot(path: &Path) -> Vec<String> {
    type PushVoid = unsafe extern "C" fn(*mut State);
    type PushBool = unsafe extern "C" fn(*mut State, c_int);
    type PushNumber = unsafe extern "C" fn(*mut State, f64);
    type PushString = unsafe extern "C" fn(*mut State, *const c_char);
    type PushLString = unsafe extern "C" fn(*mut State, *const c_char, c_int);
    type Pred = unsafe extern "C" fn(*mut State, c_int) -> c_int;
    type GetTop = unsafe extern "C" fn(*mut State) -> c_int;
    type Pop = unsafe extern "C" fn(*mut State, c_int);
    type StackIndex = unsafe extern "C" fn(*mut State, c_int);
    type Rot = unsafe extern "C" fn(*mut State, c_int);
    type TypeOf = unsafe extern "C" fn(*mut State, c_int) -> *const c_char;
    type ToNumber = unsafe extern "C" fn(*mut State, c_int) -> f64;
    type ToInt = unsafe extern "C" fn(*mut State, c_int) -> c_int;
    type ToUint = unsafe extern "C" fn(*mut State, c_int) -> c_uint;
    type ToI16 = unsafe extern "C" fn(*mut State, c_int) -> i16;
    type ToU16 = unsafe extern "C" fn(*mut State, c_int) -> u16;
    type TryNumber = unsafe extern "C" fn(*mut State, c_int, f64) -> f64;
    type TryInt = unsafe extern "C" fn(*mut State, c_int, c_int) -> c_int;
    type TryString = unsafe extern "C" fn(*mut State, c_int, *const c_char) -> *const c_char;
    type Named = unsafe extern "C" fn(*mut State, *const c_char);
    type NamedAtts = unsafe extern "C" fn(*mut State, *const c_char, c_int);
    type HasNamed = unsafe extern "C" fn(*mut State, c_int, *const c_char) -> c_int;
    type NamedIndex = unsafe extern "C" fn(*mut State, c_int, *const c_char);
    type Index = unsafe extern "C" fn(*mut State, c_int, c_int);
    type HasIndex = unsafe extern "C" fn(*mut State, c_int, c_int) -> c_int;
    type GetLength = unsafe extern "C" fn(*mut State, c_int) -> c_int;
    type SetLength = unsafe extern "C" fn(*mut State, c_int, c_int);
    type NextIterator = unsafe extern "C" fn(*mut State, c_int) -> *const c_char;
    type PushIterator = unsafe extern "C" fn(*mut State, c_int, c_int);
    type Ref = unsafe extern "C" fn(*mut State) -> *const c_char;
    type Unref = unsafe extern "C" fn(*mut State, *const c_char);
    type NewCFunction = unsafe extern "C" fn(
        *mut State,
        Option<unsafe extern "C" fn(*mut State)>,
        *const c_char,
        c_int,
    );
    type NewCFunctionX = unsafe extern "C" fn(
        *mut State,
        Option<unsafe extern "C" fn(*mut State)>,
        *const c_char,
        c_int,
        *mut c_void,
        Option<unsafe extern "C" fn(*mut State, *mut c_void)>,
    );
    type NewUserdata = unsafe extern "C" fn(
        *mut State,
        *const c_char,
        *mut c_void,
        Option<unsafe extern "C" fn(*mut State, *mut c_void)>,
    );
    type ToUserdata = unsafe extern "C" fn(*mut State, c_int, *const c_char) -> *mut c_void;
    type IsUserdata = unsafe extern "C" fn(*mut State, c_int, *const c_char) -> c_int;
    type NewRegexp = unsafe extern "C" fn(*mut State, *const c_char, c_int);
    type ProtectedLoad = unsafe extern "C" fn(*mut State, *const c_char, *const c_char) -> c_int;
    type ProtectedCall = unsafe extern "C" fn(*mut State, c_int) -> c_int;

    let api = unsafe { Api::open(path) };
    let library = &api.library;
    let push_undefined: PushVoid = unsafe { load(library, b"js_pushundefined\0") };
    let push_null: PushVoid = unsafe { load(library, b"js_pushnull\0") };
    let push_boolean: PushBool = unsafe { load(library, b"js_pushboolean\0") };
    let push_number: PushNumber = unsafe { load(library, b"js_pushnumber\0") };
    let push_string: PushString = unsafe { load(library, b"js_pushstring\0") };
    let push_lstring: PushLString = unsafe { load(library, b"js_pushlstring\0") };
    let gettop: GetTop = unsafe { load(library, b"js_gettop\0") };
    let pop: Pop = unsafe { load(library, b"js_pop\0") };
    let copy: StackIndex = unsafe { load(library, b"js_copy\0") };
    let remove: StackIndex = unsafe { load(library, b"js_remove\0") };
    let replace: StackIndex = unsafe { load(library, b"js_replace\0") };
    let rot: Rot = unsafe { load(library, b"js_rot\0") };
    let dup: PushVoid = unsafe { load(library, b"js_dup\0") };
    let dup2: PushVoid = unsafe { load(library, b"js_dup2\0") };
    let rot2: PushVoid = unsafe { load(library, b"js_rot2\0") };
    let rot3: PushVoid = unsafe { load(library, b"js_rot3\0") };
    let rot4: PushVoid = unsafe { load(library, b"js_rot4\0") };
    let rot2pop1: PushVoid = unsafe { load(library, b"js_rot2pop1\0") };
    let rot3pop2: PushVoid = unsafe { load(library, b"js_rot3pop2\0") };
    let typeof_: TypeOf = unsafe { load(library, b"js_typeof\0") };
    let type_: Pred = unsafe { load(library, b"js_type\0") };
    let tonumber: ToNumber = unsafe { load(library, b"js_tonumber\0") };
    let tointeger: ToInt = unsafe { load(library, b"js_tointeger\0") };
    let toint32: ToInt = unsafe { load(library, b"js_toint32\0") };
    let touint32: ToUint = unsafe { load(library, b"js_touint32\0") };
    let toint16: ToI16 = unsafe { load(library, b"js_toint16\0") };
    let touint16: ToU16 = unsafe { load(library, b"js_touint16\0") };
    let trynumber: TryNumber = unsafe { load(library, b"js_trynumber\0") };
    let tryinteger: TryInt = unsafe { load(library, b"js_tryinteger\0") };
    let tryboolean: TryInt = unsafe { load(library, b"js_tryboolean\0") };
    let trystring: TryString = unsafe { load(library, b"js_trystring\0") };
    let tostring: ToString = unsafe { load(library, b"js_tostring\0") };
    let torepr: ToString = unsafe { load(library, b"js_torepr\0") };
    let newobject: PushVoid = unsafe { load(library, b"js_newobject\0") };
    let newarray: PushVoid = unsafe { load(library, b"js_newarray\0") };
    let newboolean: PushBool = unsafe { load(library, b"js_newboolean\0") };
    let newnumber: PushNumber = unsafe { load(library, b"js_newnumber\0") };
    let newstring: PushString = unsafe { load(library, b"js_newstring\0") };
    let setproperty: NamedIndex = unsafe { load(library, b"js_setproperty\0") };
    let getproperty: NamedIndex = unsafe { load(library, b"js_getproperty\0") };
    let hasproperty: HasNamed = unsafe { load(library, b"js_hasproperty\0") };
    let delproperty: NamedIndex = unsafe { load(library, b"js_delproperty\0") };
    let defproperty: unsafe extern "C" fn(*mut State, c_int, *const c_char, c_int) =
        unsafe { load(library, b"js_defproperty\0") };
    let setindex: Index = unsafe { load(library, b"js_setindex\0") };
    let getindex: Index = unsafe { load(library, b"js_getindex\0") };
    let hasindex: HasIndex = unsafe { load(library, b"js_hasindex\0") };
    let delindex: Index = unsafe { load(library, b"js_delindex\0") };
    let getlength: GetLength = unsafe { load(library, b"js_getlength\0") };
    let setlength: SetLength = unsafe { load(library, b"js_setlength\0") };
    let pushiterator: PushIterator = unsafe { load(library, b"js_pushiterator\0") };
    let nextiterator: NextIterator = unsafe { load(library, b"js_nextiterator\0") };
    let setglobal: Named = unsafe { load(library, b"js_setglobal\0") };
    let getglobal: Named = unsafe { load(library, b"js_getglobal\0") };
    let delglobal: Named = unsafe { load(library, b"js_delglobal\0") };
    let defglobal: NamedAtts = unsafe { load(library, b"js_defglobal\0") };
    let setregistry: Named = unsafe { load(library, b"js_setregistry\0") };
    let getregistry: Named = unsafe { load(library, b"js_getregistry\0") };
    let delregistry: Named = unsafe { load(library, b"js_delregistry\0") };
    let ref_: Ref = unsafe { load(library, b"js_ref\0") };
    let unref: Unref = unsafe { load(library, b"js_unref\0") };
    let isarray: Pred = unsafe { load(library, b"js_isarray\0") };
    let isnumberobject: Pred = unsafe { load(library, b"js_isnumberobject\0") };
    let isstringobject: Pred = unsafe { load(library, b"js_isstringobject\0") };
    let isbooleanobject: Pred = unsafe { load(library, b"js_isbooleanobject\0") };
    let iserror: Pred = unsafe { load(library, b"js_iserror\0") };
    let isregexp: Pred = unsafe { load(library, b"js_isregexp\0") };
    let iscallable: Pred = unsafe { load(library, b"js_iscallable\0") };
    let newcfunction: NewCFunction = unsafe { load(library, b"js_newcfunction\0") };
    let newcfunctionx: NewCFunctionX = unsafe { load(library, b"js_newcfunctionx\0") };
    let newuserdata: NewUserdata = unsafe { load(library, b"js_newuserdata\0") };
    let touserdata: ToUserdata = unsafe { load(library, b"js_touserdata\0") };
    let isuserdata: IsUserdata = unsafe { load(library, b"js_isuserdata\0") };
    let newregexp: NewRegexp = unsafe { load(library, b"js_newregexp\0") };
    let ploadstring: ProtectedLoad = unsafe { load(library, b"js_ploadstring\0") };
    let pcall: ProtectedCall = unsafe { load(library, b"js_pcall\0") };
    let pconstruct: ProtectedCall = unsafe { load(library, b"js_pconstruct\0") };

    let state = unsafe { (api.newstate)(None, ptr::null_mut(), 0) };
    assert!(!state.is_null());
    unsafe { (api.setreport)(state, Some(capture_report)) };
    PUSH_NUMBER.store(push_number as usize, Ordering::Relaxed);
    let mut out = Vec::new();

    unsafe {
        push_undefined(state);
        push_null(state);
        push_boolean(state, 7);
        push_number(state, -123.75);
        push_string(state, c"456.5".as_ptr());
        let embedded = b"a\0b";
        push_lstring(state, embedded.as_ptr().cast(), embedded.len() as c_int);
        out.push(format!("top={}", gettop(state)));
        for index in 0..6 {
            out.push(format!(
                "v{index}:type={},typeof={}",
                type_(state, index),
                CStr::from_ptr(typeof_(state, index)).to_string_lossy()
            ));
        }
        out.push(format!(
            "nums={:x},{},{},{},{},{}",
            tonumber(state, 4).to_bits(),
            tointeger(state, 3),
            toint32(state, 3),
            touint32(state, 3),
            toint16(state, 3),
            touint16(state, 3)
        ));
        out.push(format!(
            "tries={:x},{},{}",
            trynumber(state, 4, -1.0).to_bits(),
            tryinteger(state, 4, -1),
            tryboolean(state, 0, -1)
        ));
        out.push(
            CStr::from_ptr(trystring(state, 5, c"fallback".as_ptr()))
                .to_string_lossy()
                .into_owned(),
        );
        pop(state, 6);

        for value in [1.0, 2.0, 3.0, 4.0] {
            push_number(state, value);
        }
        copy(state, 0);
        remove(state, 1);
        push_number(state, 9.0);
        replace(state, 0);
        rot(state, 4);
        rot2(state);
        rot3(state);
        rot4(state);
        dup(state);
        dup2(state);
        rot2pop1(state);
        rot3pop2(state);
        let mut stack = Vec::new();
        for index in 0..gettop(state) {
            stack.push(
                CStr::from_ptr(tostring(state, index))
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        out.push(format!("stack={}", stack.join(",")));
        pop(state, gettop(state));

        newobject(state);
        push_number(state, 10.0);
        setproperty(state, -2, c"x".as_ptr());
        push_string(state, c"hidden".as_ptr());
        defproperty(state, -2, c"y".as_ptr(), 2);
        out.push(format!(
            "props={},{},{}",
            hasproperty(state, -1, c"x".as_ptr()),
            hasproperty(state, -1, c"y".as_ptr()),
            hasproperty(state, -1, c"z".as_ptr())
        ));
        getproperty(state, -1, c"x".as_ptr());
        out.push(format!("x={}", tonumber(state, -1)));
        pop(state, 1);
        pushiterator(state, -1, 1);
        let mut names = Vec::new();
        loop {
            let name = nextiterator(state, -1);
            if name.is_null() {
                break;
            }
            names.push(CStr::from_ptr(name).to_string_lossy().into_owned());
        }
        names.sort();
        out.push(format!("iter={}", names.join(",")));
        pop(state, 1);
        delproperty(state, -1, c"x".as_ptr());
        out.push(format!("del={}", hasproperty(state, -1, c"x".as_ptr())));
        pop(state, 1);

        newarray(state);
        for (index, value) in [(0, 1.0), (2, 3.0), (5, 6.0)] {
            push_number(state, value);
            setindex(state, -2, index);
        }
        out.push(format!(
            "array={},{},{},{}",
            isarray(state, -1),
            getlength(state, -1),
            hasindex(state, -1, 1),
            hasindex(state, -1, 2)
        ));
        getindex(state, -1, 2);
        out.push(format!("i2={}", tonumber(state, -1)));
        pop(state, 1);
        delindex(state, -1, 2);
        setlength(state, -1, 2);
        out.push(format!("len={}", getlength(state, -1)));
        pop(state, 1);

        push_string(state, c"global-value".as_ptr());
        defglobal(state, c"g".as_ptr(), 0);
        getglobal(state, c"g".as_ptr());
        out.push(
            CStr::from_ptr(tostring(state, -1))
                .to_string_lossy()
                .into_owned(),
        );
        pop(state, 1);
        push_number(state, 9.0);
        setglobal(state, c"g".as_ptr());
        delglobal(state, c"g".as_ptr());

        push_string(state, c"registry-value".as_ptr());
        setregistry(state, c"k".as_ptr());
        getregistry(state, c"k".as_ptr());
        out.push(
            CStr::from_ptr(tostring(state, -1))
                .to_string_lossy()
                .into_owned(),
        );
        let reference = CStr::from_ptr(ref_(state)).to_owned();
        getregistry(state, reference.as_ptr());
        out.push(
            CStr::from_ptr(tostring(state, -1))
                .to_string_lossy()
                .into_owned(),
        );
        pop(state, 1);
        unref(state, reference.as_ptr());
        delregistry(state, c"k".as_ptr());
        pop(state, 1);

        newboolean(state, 1);
        newnumber(state, 2.0);
        newstring(state, c"boxed".as_ptr());
        out.push(format!(
            "boxed={},{},{}",
            isbooleanobject(state, -3),
            isnumberobject(state, -2),
            isstringobject(state, -1)
        ));
        pop(state, 3);

        let newtypeerror: Named = load(library, b"js_newtypeerror\0");
        newtypeerror(state, c"bad".as_ptr());
        out.push(format!("error={}", iserror(state, -1)));
        out.push(
            CStr::from_ptr(torepr(state, -1))
                .to_string_lossy()
                .into_owned(),
        );
        pop(state, 1);

        newcfunction(state, Some(return_42), c"answer".as_ptr(), 0);
        out.push(format!("callable={}", iscallable(state, -1)));
        push_undefined(state);
        out.push(format!("pcall={}", pcall(state, 0)));
        out.push(format!("answer={}", tonumber(state, -1)));
        pop(state, 1);
        let mut data = 123_i32;
        newcfunctionx(
            state,
            Some(return_42),
            c"answerx".as_ptr(),
            0,
            (&mut data as *mut i32).cast(),
            Some(noop_finalizer),
        );
        pop(state, 1);

        let mut userdata = 0x55aa_i32;
        newuserdata(
            state,
            c"tag".as_ptr(),
            (&mut userdata as *mut i32).cast(),
            Some(noop_finalizer),
        );
        out.push(format!(
            "userdata={},{}",
            isuserdata(state, -1, c"tag".as_ptr()),
            touserdata(state, -1, c"tag".as_ptr()) == (&mut userdata as *mut i32).cast()
        ));
        pop(state, 1);

        newregexp(state, c"a+".as_ptr(), 1 | 2 | 4 | 0x4000);
        out.push(format!("regexp={}", isregexp(state, -1)));
        pop(state, 1);

        out.push(format!(
            "pload={}",
            ploadstring(state, c"ffi.js".as_ptr(), c"var direct=40+2;".as_ptr())
        ));
        push_undefined(state);
        out.push(format!("pcall-script={}", pcall(state, 0)));
        pop(state, 1);

        newcfunction(state, Some(return_42), c"C".as_ptr(), 0);
        out.push(format!("pconstruct={}", pconstruct(state, 0)));
        pop(state, 1);

        (api.gc)(state, 0);
        (api.freestate)(state);
    }
    out
}

#[test]
fn direct_public_ffi_workflows_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let c = unsafe { public_api_snapshot(&c_library()) };
    let rust = unsafe { public_api_snapshot(&rust_library()) };
    assert_eq!(rust, c);

    for fail_at in 1..=24 {
        let mut outcomes = Vec::new();
        for path in [c_library(), rust_library()] {
            let api = unsafe { Api::open(&path) };
            let mut context = AllocContext { calls: 0, fail_at };
            let state = unsafe {
                (api.newstate)(
                    Some(controlled_alloc),
                    (&mut context as *mut AllocContext).cast(),
                    0,
                )
            };
            outcomes.push((state.is_null(), context.calls));
            if !state.is_null() {
                unsafe { (api.freestate)(state) };
            }
        }
        assert_eq!(
            outcomes[1], outcomes[0],
            "allocator failure at call {fail_at}"
        );
    }

    let limited = |path: &Path| unsafe {
        let api = Api::open(path);
        REPORTS.lock().unwrap().clear();
        let state = (api.newstate)(None, ptr::null_mut(), 0);
        (api.setreport)(state, Some(capture_report));
        (api.setlimit)(state, 50, 0);
        let status = (api.dostring)(state, c"for(;;){}".as_ptr());
        let reports = REPORTS.lock().unwrap().clone();
        (api.freestate)(state);
        (status, reports)
    };
    assert_eq!(limited(&rust_library()), limited(&c_library()));
}

#[test]
fn symbol_surface_is_identical_and_loadable() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let c = open_library(c_library());
    let rust = open_library(rust_library());
    let symbols = include_str!("../SYMBOLS.md")
        .lines()
        .filter(|line| line.starts_with("| ") && !line.starts_with("| #"))
        .map(|line| line.split('`').nth(1).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(symbols.len(), 237);
    for symbol in symbols {
        let name = CString::new(symbol).unwrap();
        unsafe {
            c.get::<*mut c_void>(name.as_bytes_with_nul()).unwrap();
            rust.get::<*mut c_void>(name.as_bytes_with_nul()).unwrap();
        }
    }
}

#[test]
fn state_options_and_valid_javascript_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());

    for path in [c_library(), rust_library()] {
        let api = unsafe { Api::open(&path) };
        for flags in [0, 1] {
            let state = unsafe { (api.newstate)(None, ptr::null_mut(), flags) };
            assert!(!state.is_null());
            let mut marker = 0x1234_5678_u64;
            let context = (&mut marker as *mut u64).cast();
            unsafe {
                (api.setcontext)(state, context);
                assert_eq!((api.getcontext)(state), context);
                (api.setcontext)(state, ptr::null_mut());
                assert!((api.getcontext)(state).is_null());
                (api.setlimit)(state, 0, 0);
                (api.gc)(state, 0);
                (api.gc)(state, 1);
                (api.freestate)(state);
            }
        }
        let _keep_loaded = &api.library;
    }

    let corpus = [
        r#"__result = "empty";"#,
        r#"__result = [undefined === void 0, null === null, !0, !!1].join(",");"#,
        r#"__result = JSON.stringify([0, -0, 1.5, -2.75, 1/0, -1/0, 0/0]);"#,
        r#"__result = JSON.stringify(["", "short", "a\u0000b", "\u00e9", "\u20ac", "\ud83d\ude00"]);"#,
        r#"var a=[1,,3]; a.push(4); a.unshift(0); __result=JSON.stringify([a,a.length,a.slice(-3),a.indexOf(3)]);"#,
        r#"var a=[4,1,3,2]; a.sort(function(x,y){return x-y}); __result=a.join(":");"#,
        r#"var a=[1,2,3,4]; __result=JSON.stringify([a.map(function(x){return x*x}),a.filter(function(x){return x%2}),a.reduce(function(x,y){return x+y},0)]);"#,
        r#"var o={a:1}; Object.defineProperty(o,"b",{value:2,enumerable:false}); var p=Object.create(o); p.c=3; __result=JSON.stringify([p.a,p.c,Object.keys(p),o.b]);"#,
        r#"function F(x){this.x=x} F.prototype.y=7; var f=new F(3); __result=JSON.stringify([f.x,f.y,f instanceof F,typeof F]);"#,
        r#"function add(a,b){return this.x+a+b} var b=add.bind({x:10},2); __result=JSON.stringify([add.call({x:1},2,3),add.apply({x:4},[5,6]),b(3)]);"#,
        r#"__result=JSON.stringify(["Abba".match(/b/gi),"a-b-c".split(/-/),"abc".replace(/b/,"X"),/^b/im.test("a\nB")]);"#,
        r#"var r=/(a+)(b?)/gi; var x=r.exec("xxAAb"); var y=r.exec("aa"); __result=JSON.stringify([x,x.index,r.lastIndex,y]);"#,
        r#"__result=JSON.stringify([Number(" 0x10 "),parseInt("101",2),parseFloat("-1.25e2x"),isNaN(NaN),isFinite(3)]);"#,
        r#"__result=JSON.stringify([(255).toString(16),(1.25).toFixed(3),(12.5).toExponential(2),(12.5).toPrecision(4)]);"#,
        r#"__result=JSON.stringify([Math.abs(-3),Math.round(-0.5),Math.max(),Math.min(),Math.pow(2,10),Math.atan2(1,-1)]);"#,
        r#"var d=new Date("2000-02-29T12:34:56.789Z"); __result=JSON.stringify([d.toISOString(),d.getUTCFullYear(),d.getUTCMonth(),Date.parse("1970-01-01T00:00:00Z")]);"#,
        r#"var x=JSON.parse('{"a":[1,true,null,"x"]}',function(k,v){return k==="0"?2:v}); __result=JSON.stringify(x,null,2);"#,
        r#"__result=JSON.stringify([encodeURI("a b/\u00e9"),encodeURIComponent("a b/\u00e9"),decodeURI("a%20b/%C3%A9")]);"#,
        r#"var o={a:1,b:2}; var k=[], it; for(it in o)k.push(it); k.sort(); __result=k.join(",");"#,
        r#""use strict"; function f(a){return a+1} __result=String(f(41));"#,
    ];
    for flags in [0, 1] {
        for source in corpus {
            let source = format!("var __result;\n{source}");
            let result = compare_script(flags, &source);
            assert_eq!(result.status, 0, "unexpected rejection: {source}");
            assert!(result.reports.is_empty());
        }
    }

    let mut seed = 0x6d2b_79f5_u32;
    for _ in 0..128 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let a = (seed as i32 % 100_000) as i64;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let b = (seed as i32 % 100_000) as i64;
        let source = format!(
            "__result=JSON.stringify([{a}+({b}),({a})-({b}),({a})*({b}),({a})<({b}),String({a}/7)]);"
        );
        assert_eq!(compare_script(0, &source).status, 0);
    }
}

#[test]
fn numeric_and_utf_low_level_exports_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        type NumberToInt = unsafe extern "C" fn(f64) -> c_int;
        type NumberToUint = unsafe extern "C" fn(f64) -> c_uint;
        type NumberToI16 = unsafe extern "C" fn(f64) -> i16;
        type NumberToU16 = unsafe extern "C" fn(f64) -> u16;
        type StrToNum = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64;
        type StrTol = unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> f64;
        type RuneAt = unsafe extern "C" fn(*mut c_int, *const c_char) -> c_int;
        type RuneToChar = unsafe extern "C" fn(*mut c_char, *const c_int) -> c_int;
        type RuneUnary = unsafe extern "C" fn(c_int) -> c_int;

        let c = open_library(c_library());
        let rust = open_library(rust_library());
        let int_names: &[(&[u8], bool)] = &[
            (b"jsV_numbertointeger\0", false),
            (b"jsV_numbertoint32\0", false),
        ];
        let values = [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -0.0,
            0.0,
            -1.9,
            1.9,
            i32::MIN as f64 - 1.0,
            i32::MIN as f64,
            i32::MAX as f64,
            i32::MAX as f64 + 1.0,
            4_294_967_295.0,
            4_294_967_296.0,
        ];
        for (name, _) in int_names {
            let cf: NumberToInt = load(&c, name);
            let rf: NumberToInt = load(&rust, name);
            for value in values {
                assert_eq!(
                    rf(value),
                    cf(value),
                    "{}({value:?})",
                    String::from_utf8_lossy(name)
                );
            }
        }
        let cu: NumberToUint = load(&c, b"jsV_numbertouint32\0");
        let ru: NumberToUint = load(&rust, b"jsV_numbertouint32\0");
        let ci16: NumberToI16 = load(&c, b"jsV_numbertoint16\0");
        let ri16: NumberToI16 = load(&rust, b"jsV_numbertoint16\0");
        let cu16: NumberToU16 = load(&c, b"jsV_numbertouint16\0");
        let ru16: NumberToU16 = load(&rust, b"jsV_numbertouint16\0");
        let mut bits = 0x1234_5678_9abc_def0_u64;
        for _ in 0..10_000 {
            bits = bits
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let value = f64::from_bits(bits);
            assert_eq!(ru(value), cu(value));
            assert_eq!(ri16(value), ci16(value));
            assert_eq!(ru16(value), cu16(value));
        }

        for input in [
            "",
            " ",
            "0",
            "-0",
            "+12.5e-2tail",
            "0x10",
            "Infinity",
            "NaN",
            "1e309",
            "9007199254740993",
        ] {
            let text = CString::new(input).unwrap();
            for name in [b"js_stringtofloat\0".as_slice(), b"js_strtod\0".as_slice()] {
                let cf: StrToNum = load(&c, name);
                let rf: StrToNum = load(&rust, name);
                let mut ce = ptr::null_mut();
                let mut re = ptr::null_mut();
                let cv = cf(text.as_ptr(), &mut ce);
                let rv = rf(text.as_ptr(), &mut re);
                assert_eq!(rv.to_bits(), cv.to_bits(), "{input:?}");
                assert_eq!(re.offset_from(text.as_ptr()), ce.offset_from(text.as_ptr()));
            }
            for radix in [0, 2, 8, 10, 16, 36] {
                let cf: StrTol = load(&c, b"js_strtol\0");
                let rf: StrTol = load(&rust, b"js_strtol\0");
                let mut ce = ptr::null_mut();
                let mut re = ptr::null_mut();
                let cv = cf(text.as_ptr(), &mut ce, radix);
                let rv = rf(text.as_ptr(), &mut re, radix);
                assert_eq!(rv.to_bits(), cv.to_bits(), "{input:?}, radix {radix}");
                assert_eq!(re.offset_from(text.as_ptr()), ce.offset_from(text.as_ptr()));
            }
        }

        let cdecode: RuneAt = load(&c, b"jsU_chartorune\0");
        let rdecode: RuneAt = load(&rust, b"jsU_chartorune\0");
        let cencode: RuneToChar = load(&c, b"jsU_runetochar\0");
        let rencode: RuneToChar = load(&rust, b"jsU_runetochar\0");
        for bytes in [
            vec![0],
            vec![b'A', 0],
            vec![0xc3, 0xa9, 0],
            vec![0xe2, 0x82, 0xac, 0],
            vec![0xf0, 0x9f, 0x98, 0x80, 0],
            vec![0xc0, 0x80, 0],
            vec![0xff, 0],
            vec![0xed, 0xa0, 0x80, 0],
        ] {
            let mut cr = 0;
            let mut rr = 0;
            let cn = cdecode(&mut cr, bytes.as_ptr().cast());
            let rn = rdecode(&mut rr, bytes.as_ptr().cast());
            assert_eq!((rn, rr), (cn, cr), "{bytes:x?}");
        }
        for rune in [
            0, 1, 0x7f, 0x80, 0x7ff, 0x800, 0xd800, 0xffff, 0x10000, 0x10ffff, 0x110000,
        ] {
            let mut cb = [0_i8; 8];
            let mut rb = [0_i8; 8];
            let cn = cencode(cb.as_mut_ptr(), &rune);
            let rn = rencode(rb.as_mut_ptr(), &rune);
            assert_eq!(rn, cn);
            assert_eq!(&rb[..rn as usize], &cb[..cn as usize]);
        }
        for name in [
            b"jsU_runelen\0".as_slice(),
            b"jsU_isalpharune\0".as_slice(),
            b"jsU_islowerrune\0".as_slice(),
            b"jsU_isupperrune\0".as_slice(),
            b"jsU_tolowerrune\0".as_slice(),
            b"jsU_toupperrune\0".as_slice(),
        ] {
            let cf: RuneUnary = load(&c, name);
            let rf: RuneUnary = load(&rust, name);
            for rune in (-1..=0x110000).step_by(257) {
                assert_eq!(
                    rf(rune),
                    cf(rune),
                    "{}({rune})",
                    String::from_utf8_lossy(name)
                );
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Submatch {
    start: *const c_char,
    end: *const c_char,
}

#[repr(C)]
struct Resub {
    count: c_int,
    sub: [Submatch; 16],
}

#[derive(Debug, PartialEq, Eq)]
struct RegexResult {
    compile_error: Option<Vec<u8>>,
    execution: Option<(c_int, c_int, Vec<(isize, isize)>)>,
}

unsafe fn regex_result(path: &Path, pattern: &str, text: &str, flags: c_int) -> RegexResult {
    type RegComp = unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> *mut c_void;
    type RegExec = unsafe extern "C" fn(*mut c_void, *const c_char, *mut Resub, c_int) -> c_int;
    type RegFree = unsafe extern "C" fn(*mut c_void);
    let library = open_library(path);
    let comp: RegComp = unsafe { load(&library, b"js_regcomp\0") };
    let exec: RegExec = unsafe { load(&library, b"js_regexec\0") };
    let free: RegFree = unsafe { load(&library, b"js_regfree\0") };
    let pattern = CString::new(pattern).unwrap();
    let text = CString::new(text).unwrap();
    let mut error = ptr::null();
    let program = unsafe { comp(pattern.as_ptr(), flags, &mut error) };
    if program.is_null() {
        return RegexResult {
            compile_error: Some(unsafe { CStr::from_ptr(error) }.to_bytes().to_vec()),
            execution: None,
        };
    }
    let empty = Submatch {
        start: ptr::null(),
        end: ptr::null(),
    };
    let mut sub = Resub {
        count: 0,
        sub: [empty; 16],
    };
    let result = unsafe { exec(program, text.as_ptr(), &mut sub, 0) };
    let offsets = sub.sub[..sub.count as usize]
        .iter()
        .map(|item| {
            if item.start.is_null() {
                (-1, -1)
            } else {
                unsafe {
                    (
                        item.start.offset_from(text.as_ptr()),
                        item.end.offset_from(text.as_ptr()),
                    )
                }
            }
        })
        .collect();
    unsafe { free(program) };
    RegexResult {
        compile_error: None,
        execution: Some((result, sub.count, offsets)),
    }
}

#[test]
fn low_level_regexp_matches_and_rejects_identically() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let valid = [
        ("", "", 0),
        ("abc", "xxabcxx", 0),
        ("^abc$", "abc", 0),
        ("(a+)(b?)", "xxaaab", 0),
        ("[a-z]+", "123AbC", 1),
        ("^b", "a\nb", 2),
        ("a{2,4}?", "aaaaa", 0),
        ("\\bword\\b", "a word!", 0),
        ("(a|ab)c", "zabc", 0),
    ];
    for (pattern, text, flags) in valid {
        let c = unsafe { regex_result(&c_library(), pattern, text, flags) };
        let rust = unsafe { regex_result(&rust_library(), pattern, text, flags) };
        assert_eq!(rust, c, "/{pattern}/ on {text:?}, flags={flags}");
    }
    for pattern in [
        "\\",
        "[",
        "[z-a]",
        "(",
        ")",
        "*",
        "a{2,1}",
        "(a)\\2",
        "(a?)*",
        "a{999999999999999999999}",
    ] {
        let c = unsafe { regex_result(&c_library(), pattern, "", 0) };
        let rust = unsafe { regex_result(&rust_library(), pattern, "", 0) };
        assert_eq!(rust, c, "/{pattern}/");
        assert!(c.compile_error.is_some());
    }
}

#[test]
fn javascript_error_surface_matches_exactly() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let invalid = [
        r#"Number.prototype.valueOf.call({})"#,
        r#"Number.prototype.toString.call({},10)"#,
        r#"(1).toString(1)"#,
        r#"(1).toString(37)"#,
        r#"(1).toFixed(-1)"#,
        r#"(1).toFixed(21)"#,
        r#"(1).toExponential(-1)"#,
        r#"(1).toExponential(21)"#,
        r#"(1).toPrecision(0)"#,
        r#"(1).toPrecision(22)"#,
        r#"Boolean.prototype.valueOf.call({})"#,
        r#"String.prototype.valueOf.call({})"#,
        r#"String.prototype.indexOf.call(null,"x")"#,
        r#"Array.prototype.sort.call([2,1], 3)"#,
        r#"Array.prototype.toString.call(null)"#,
        r#"[1].every(3)"#,
        r#"[1].some(3)"#,
        r#"[1].forEach(3)"#,
        r#"[1].map(3)"#,
        r#"[1].filter(3)"#,
        r#"[1].reduce(3)"#,
        r#"[].reduce(function(a,b){return a+b})"#,
        r#"Array(3).reduce(function(a,b){return a+b})"#,
        r#"[1].reduceRight(3)"#,
        r#"[].reduceRight(function(a,b){return a+b})"#,
        r#"Function.prototype.toString.call({})"#,
        r#"Function.prototype.apply.call({},null,[])"#,
        r#"Function.prototype.call.call({})"#,
        r#"Function.prototype.bind.call({})"#,
        r#"1 instanceof 2"#,
        r#"function F(){}; F.prototype=1; ({} instanceof F)"#,
        r#"Object.getPrototypeOf(1)"#,
        r#"Object.getOwnPropertyDescriptor(1,"x")"#,
        r#"Object.defineProperty(1,"x",{})"#,
        r#"Object.defineProperty({}, "x", {value:1,get:function(){}})"#,
        r#"Object.create(1)"#,
        r#"Object.preventExtensions(1)"#,
        r#"Object.isExtensible(1)"#,
        r#"Object.seal(1)"#,
        r#"Object.isSealed(1)"#,
        r#"Object.freeze(1)"#,
        r#"Object.isFrozen(1)"#,
        r#"Object.keys(1)"#,
        r#""use strict"; var o={}; Object.preventExtensions(o); o.x=1"#,
        r#""use strict"; var o={}; Object.defineProperty(o,"x",{value:1,writable:false}); o.x=2"#,
        r#""use strict"; var o={}; Object.defineProperty(o,"x",{value:1,configurable:false}); delete o.x"#,
        r#""use strict"; undeclared_name=1"#,
        r#"missing_name"#,
        r#""x" in 3"#,
        r#"Date.prototype.valueOf.call({})"#,
        r#"new Date(NaN).toISOString()"#,
        r#"Date.prototype.toJSON.call({toISOString:3})"#,
        r#"var a=[]; a[0]=a; JSON.stringify(a)"#,
        r#"var o={}; o.x=o; JSON.stringify(o)"#,
        r#"JSON.parse("")"#,
        r#"JSON.parse("{")"#,
        r#"JSON.parse('{"x"}')"#,
        r#"JSON.parse("tru")"#,
        r#"decodeURIComponent("%")"#,
        r#"decodeURIComponent("%zz")"#,
        r#"new RegExp(/a/,"g")"#,
        r#"new RegExp("a","x")"#,
        r#"new RegExp("a","gg")"#,
        r#"new RegExp("a","ii")"#,
        r#"new RegExp("a","mm")"#,
        r#"new RegExp("[")"#,
        r#"01"#,
        r#"0x"#,
        r#"1e"#,
        r#"1abc"#,
        r#""unterminated"#,
        r#""\x0""#,
        "/* unterminated",
        r#"var r=/unterminated;"#,
        r#"var r=/a/gg;"#,
        r#"var = 1"#,
        r#"switch(1){oops:}"#,
        r#"try {}"#,
    ];
    for source in invalid {
        let result = compare_script(0, source);
        assert_ne!(result.status, 0, "expected rejection: {source}");
        assert!(!result.reports.is_empty(), "expected report: {source}");
    }

    for source in [
        r#"__result=String(Date.parse("2000-13-01"));"#,
        r#"__result=String(Date.parse("2000-01-32"));"#,
        r#"__result=String(Date.parse("2000-01-01T25:00:00Z"));"#,
        r#"__result=String(Date.parse("2000-01-01T00:60:00Z"));"#,
        r#"__result=String(Date.parse("2000-01-01T00:00:60Z"));"#,
        r#"__result=String(Date.parse("2000-01-01T24:00:00.001Z"));"#,
        r#"__result=String(Date.parse("2000-01-01T00:00:00+24:00"));"#,
        r#"__result=String(Date.parse("not-a-date"));"#,
    ] {
        let result = compare_script(0, source);
        assert_eq!(result.status, 0);
        assert_eq!(result.value.as_deref(), Some(b"NaN".as_slice()));
    }
}
