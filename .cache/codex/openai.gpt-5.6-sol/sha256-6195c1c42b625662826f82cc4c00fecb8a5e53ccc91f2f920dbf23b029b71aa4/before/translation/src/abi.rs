#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_short, c_uint, c_ushort, c_void};

#[repr(C)]
pub struct js_State {
    _private: [u8; 0],
}

pub type js_Alloc = Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;
pub type js_Panic = Option<unsafe extern "C" fn(*mut js_State)>;
pub type js_CFunction = Option<unsafe extern "C" fn(*mut js_State)>;
pub type js_Finalize = Option<unsafe extern "C" fn(*mut js_State, *mut c_void)>;
pub type js_HasProperty =
    Option<unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Put = Option<unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Delete =
    Option<unsafe extern "C" fn(*mut js_State, *mut c_void, *const c_char) -> c_int>;
pub type js_Report = Option<unsafe extern "C" fn(*mut js_State, *const c_char)>;

macro_rules! forward {
    (fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)?;) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? {
            core::arch::naked_asm!(concat!("jmp __mujs_impl_", stringify!($name)));
        }
    };
}

forward!(
    fn js_newstate(alloc: js_Alloc, actx: *mut c_void, flags: c_int) -> *mut js_State;
);
forward!(
    fn js_setcontext(j: *mut js_State, uctx: *mut c_void);
);
forward!(
    fn js_getcontext(j: *mut js_State) -> *mut c_void;
);
forward!(
    fn js_setreport(j: *mut js_State, report: js_Report);
);
forward!(
    fn js_atpanic(j: *mut js_State, panic: js_Panic) -> js_Panic;
);
forward!(
    fn js_freestate(j: *mut js_State);
);
forward!(
    fn js_gc(j: *mut js_State, report: c_int);
);
forward!(
    fn js_setlimit(j: *mut js_State, runlimit: c_int, memlimit: c_int);
);
forward!(
    fn js_dostring(j: *mut js_State, source: *const c_char) -> c_int;
);
forward!(
    fn js_ploadstring(j: *mut js_State, filename: *const c_char, source: *const c_char) -> c_int;
);
forward!(
    fn js_pcall(j: *mut js_State, n: c_int) -> c_int;
);
forward!(
    fn js_pconstruct(j: *mut js_State, n: c_int) -> c_int;
);
forward!(
    fn js_savetry(j: *mut js_State) -> *mut c_void;
);
forward!(
    fn js_endtry(j: *mut js_State);
);
forward!(
    fn js_report(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newerror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newevalerror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newrangeerror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newreferenceerror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newsyntaxerror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newtypeerror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_newurierror(j: *mut js_State, message: *const c_char);
);
forward!(
    fn js_throw(j: *mut js_State) -> !;
);
forward!(
    fn js_loadstring(j: *mut js_State, filename: *const c_char, source: *const c_char);
);
forward!(
    fn js_eval(j: *mut js_State);
);
forward!(
    fn js_call(j: *mut js_State, n: c_int);
);
forward!(
    fn js_construct(j: *mut js_State, n: c_int);
);
forward!(
    fn js_ref(j: *mut js_State) -> *const c_char;
);
forward!(
    fn js_unref(j: *mut js_State, reference: *const c_char);
);
forward!(
    fn js_getregistry(j: *mut js_State, name: *const c_char);
);
forward!(
    fn js_setregistry(j: *mut js_State, name: *const c_char);
);
forward!(
    fn js_delregistry(j: *mut js_State, name: *const c_char);
);
forward!(
    fn js_getglobal(j: *mut js_State, name: *const c_char);
);
forward!(
    fn js_setglobal(j: *mut js_State, name: *const c_char);
);
forward!(
    fn js_defglobal(j: *mut js_State, name: *const c_char, atts: c_int);
);
forward!(
    fn js_delglobal(j: *mut js_State, name: *const c_char);
);
forward!(
    fn js_hasproperty(j: *mut js_State, idx: c_int, name: *const c_char) -> c_int;
);
forward!(
    fn js_getproperty(j: *mut js_State, idx: c_int, name: *const c_char);
);
forward!(
    fn js_setproperty(j: *mut js_State, idx: c_int, name: *const c_char);
);
forward!(
    fn js_defproperty(j: *mut js_State, idx: c_int, name: *const c_char, atts: c_int);
);
forward!(
    fn js_delproperty(j: *mut js_State, idx: c_int, name: *const c_char);
);
forward!(
    fn js_defaccessor(j: *mut js_State, idx: c_int, name: *const c_char, atts: c_int);
);
forward!(
    fn js_getlength(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_setlength(j: *mut js_State, idx: c_int, len: c_int);
);
forward!(
    fn js_hasindex(j: *mut js_State, idx: c_int, i: c_int) -> c_int;
);
forward!(
    fn js_getindex(j: *mut js_State, idx: c_int, i: c_int);
);
forward!(
    fn js_setindex(j: *mut js_State, idx: c_int, i: c_int);
);
forward!(
    fn js_delindex(j: *mut js_State, idx: c_int, i: c_int);
);
forward!(
    fn js_currentfunction(j: *mut js_State);
);
forward!(
    fn js_currentfunctiondata(j: *mut js_State) -> *mut c_void;
);
forward!(
    fn js_pushglobal(j: *mut js_State);
);
forward!(
    fn js_pushundefined(j: *mut js_State);
);
forward!(
    fn js_pushnull(j: *mut js_State);
);
forward!(
    fn js_pushboolean(j: *mut js_State, v: c_int);
);
forward!(
    fn js_pushnumber(j: *mut js_State, v: f64);
);
forward!(
    fn js_pushstring(j: *mut js_State, v: *const c_char);
);
forward!(
    fn js_pushlstring(j: *mut js_State, v: *const c_char, n: c_int);
);
forward!(
    fn js_pushliteral(j: *mut js_State, v: *const c_char);
);
forward!(
    fn js_newobjectx(j: *mut js_State);
);
forward!(
    fn js_newobject(j: *mut js_State);
);
forward!(
    fn js_newarray(j: *mut js_State);
);
forward!(
    fn js_newboolean(j: *mut js_State, v: c_int);
);
forward!(
    fn js_newnumber(j: *mut js_State, v: f64);
);
forward!(
    fn js_newstring(j: *mut js_State, v: *const c_char);
);
forward!(
    fn js_newcfunction(j: *mut js_State, fun: js_CFunction, name: *const c_char, length: c_int);
);
forward!(
    fn js_newcfunctionx(
        j: *mut js_State,
        fun: js_CFunction,
        name: *const c_char,
        length: c_int,
        data: *mut c_void,
        finalize: js_Finalize,
    );
);
forward!(
    fn js_newcconstructor(
        j: *mut js_State,
        fun: js_CFunction,
        con: js_CFunction,
        name: *const c_char,
        length: c_int,
    );
);
forward!(
    fn js_newuserdata(
        j: *mut js_State,
        tag: *const c_char,
        data: *mut c_void,
        finalize: js_Finalize,
    );
);
forward!(
    fn js_newuserdatax(
        j: *mut js_State,
        tag: *const c_char,
        data: *mut c_void,
        has: js_HasProperty,
        put: js_Put,
        del: js_Delete,
        finalize: js_Finalize,
    );
);
forward!(
    fn js_newregexp(j: *mut js_State, pattern: *const c_char, flags: c_int);
);
forward!(
    fn js_pushiterator(j: *mut js_State, idx: c_int, own: c_int);
);
forward!(
    fn js_nextiterator(j: *mut js_State, idx: c_int) -> *const c_char;
);
forward!(
    fn js_isdefined(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isundefined(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isnull(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isboolean(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isnumber(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isstring(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isprimitive(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isobject(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isarray(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isregexp(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_iscoercible(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_iscallable(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isuserdata(j: *mut js_State, idx: c_int, tag: *const c_char) -> c_int;
);
forward!(
    fn js_iserror(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isnumberobject(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isstringobject(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isbooleanobject(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_isdateobject(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_toboolean(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_tonumber(j: *mut js_State, idx: c_int) -> f64;
);
forward!(
    fn js_tostring(j: *mut js_State, idx: c_int) -> *const c_char;
);
forward!(
    fn js_touserdata(j: *mut js_State, idx: c_int, tag: *const c_char) -> *mut c_void;
);
forward!(
    fn js_trystring(j: *mut js_State, idx: c_int, error: *const c_char) -> *const c_char;
);
forward!(
    fn js_trynumber(j: *mut js_State, idx: c_int, error: f64) -> f64;
);
forward!(
    fn js_tryinteger(j: *mut js_State, idx: c_int, error: c_int) -> c_int;
);
forward!(
    fn js_tryboolean(j: *mut js_State, idx: c_int, error: c_int) -> c_int;
);
forward!(
    fn js_tointeger(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_toint32(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_touint32(j: *mut js_State, idx: c_int) -> c_uint;
);
forward!(
    fn js_toint16(j: *mut js_State, idx: c_int) -> c_short;
);
forward!(
    fn js_touint16(j: *mut js_State, idx: c_int) -> c_ushort;
);
forward!(
    fn js_gettop(j: *mut js_State) -> c_int;
);
forward!(
    fn js_pop(j: *mut js_State, n: c_int);
);
forward!(
    fn js_rot(j: *mut js_State, n: c_int);
);
forward!(
    fn js_copy(j: *mut js_State, idx: c_int);
);
forward!(
    fn js_remove(j: *mut js_State, idx: c_int);
);
forward!(
    fn js_insert(j: *mut js_State, idx: c_int);
);
forward!(
    fn js_replace(j: *mut js_State, idx: c_int);
);
forward!(
    fn js_dup(j: *mut js_State);
);
forward!(
    fn js_dup2(j: *mut js_State);
);
forward!(
    fn js_rot2(j: *mut js_State);
);
forward!(
    fn js_rot3(j: *mut js_State);
);
forward!(
    fn js_rot4(j: *mut js_State);
);
forward!(
    fn js_rot2pop1(j: *mut js_State);
);
forward!(
    fn js_rot3pop2(j: *mut js_State);
);
forward!(
    fn js_concat(j: *mut js_State);
);
forward!(
    fn js_compare(j: *mut js_State, okay: *mut c_int) -> c_int;
);
forward!(
    fn js_equal(j: *mut js_State) -> c_int;
);
forward!(
    fn js_strictequal(j: *mut js_State) -> c_int;
);
forward!(
    fn js_instanceof(j: *mut js_State) -> c_int;
);
forward!(
    fn js_typeof(j: *mut js_State, idx: c_int) -> *const c_char;
);
forward!(
    fn js_type(j: *mut js_State, idx: c_int) -> c_int;
);
forward!(
    fn js_repr(j: *mut js_State, idx: c_int);
);
forward!(
    fn js_torepr(j: *mut js_State, idx: c_int) -> *const c_char;
);
forward!(
    fn js_tryrepr(j: *mut js_State, idx: c_int, error: *const c_char) -> *const c_char;
);
