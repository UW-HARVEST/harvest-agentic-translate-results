//! Shared differential-test harness.
//!
//! Loads BOTH the C shared library (`libcjson.so`, plus `libcJSON_test.so` for
//! the `driver` entry point) and the Rust shared library
//! (`target/<profile>/libcJSON_test.so`) with `libloading`, and exposes the
//! public cJSON API through function pointers resolved from the `.so` exports.
//!
//! Nothing in here calls the Rust crate directly - every call goes through
//! `dlsym`, exactly like an external C caller would.
#![allow(non_snake_case, dead_code, non_camel_case_types, non_upper_case_globals)]

use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_float, c_int, c_void};
use std::path::PathBuf;

pub type cJSON_bool = c_int;

pub const cJSON_Invalid: c_int = 0;
pub const cJSON_False: c_int = 1 << 0;
pub const cJSON_True: c_int = 1 << 1;
pub const cJSON_NULL: c_int = 1 << 2;
pub const cJSON_Number: c_int = 1 << 3;
pub const cJSON_String: c_int = 1 << 4;
pub const cJSON_Array: c_int = 1 << 5;
pub const cJSON_Object: c_int = 1 << 6;
pub const cJSON_Raw: c_int = 1 << 7;
pub const cJSON_IsReference: c_int = 256;
pub const cJSON_StringIsConst: c_int = 512;

#[repr(C)]
#[derive(Debug)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

/// Generates an `Api` struct whose fields are raw function pointers pulled out
/// of a dynamically loaded library.
macro_rules! define_api {
    (
        $( fn $name:ident ( $($an:ident : $at:ty),* $(,)? ) $(-> $rt:ty)? ; )*
    ) => {
        pub struct Api {
            #[allow(unused)]
            lib: &'static libloading::Library,
            $( pub $name: unsafe extern "C" fn($($at),*) $(-> $rt)? , )*
        }

        impl Api {
            pub fn load(lib: &'static libloading::Library) -> Api {
                unsafe {
                    Api {
                        lib,
                        $( $name: {
                            let s: libloading::Symbol<unsafe extern "C" fn($($at),*) $(-> $rt)?> =
                                lib.get(concat!(stringify!($name), "\0").as_bytes())
                                    .unwrap_or_else(|e| panic!("missing symbol {}: {}", stringify!($name), e));
                            *s
                        }, )*
                    }
                }
            }

            $(
                #[inline(always)]
                pub unsafe fn $name(&self, $($an: $at),*) $(-> $rt)? {
                    unsafe { (self.$name)($($an),*) }
                }
            )*
        }
    };
}

define_api! {
    fn cJSON_Version() -> *const c_char;
    fn cJSON_InitHooks(hooks: *mut cJSON_Hooks);
    fn cJSON_Parse(value: *const c_char) -> *mut cJSON;
    fn cJSON_ParseWithLength(value: *const c_char, buffer_length: usize) -> *mut cJSON;
    fn cJSON_ParseWithOpts(value: *const c_char, ret_end: *mut *const c_char, req_null: cJSON_bool) -> *mut cJSON;
    fn cJSON_ParseWithLengthOpts(value: *const c_char, len: usize, ret_end: *mut *const c_char, req_null: cJSON_bool) -> *mut cJSON;
    fn cJSON_Print(item: *const cJSON) -> *mut c_char;
    fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char;
    fn cJSON_PrintBuffered(item: *const cJSON, prebuffer: c_int, fmt: cJSON_bool) -> *mut c_char;
    fn cJSON_PrintPreallocated(item: *mut cJSON, buffer: *mut c_char, length: c_int, format: cJSON_bool) -> cJSON_bool;
    fn cJSON_Delete(item: *mut cJSON);
    fn cJSON_GetArraySize(array: *const cJSON) -> c_int;
    fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON;
    fn cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON;
    fn cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON;
    fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> cJSON_bool;
    fn cJSON_GetErrorPtr() -> *const c_char;
    fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char;
    fn cJSON_GetNumberValue(item: *const cJSON) -> c_double;
    fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsString(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool;
    fn cJSON_CreateNull() -> *mut cJSON;
    fn cJSON_CreateTrue() -> *mut cJSON;
    fn cJSON_CreateFalse() -> *mut cJSON;
    fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON;
    fn cJSON_CreateNumber(num: c_double) -> *mut cJSON;
    fn cJSON_CreateString(string: *const c_char) -> *mut cJSON;
    fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON;
    fn cJSON_CreateArray() -> *mut cJSON;
    fn cJSON_CreateObject() -> *mut cJSON;
    fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON;
    fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON;
    fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON;
    fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON;
    fn cJSON_CreateFloatArray(numbers: *const c_float, count: c_int) -> *mut cJSON;
    fn cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON;
    fn cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON;
    fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_AddItemToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_AddItemToObjectCS(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_AddItemReferenceToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON;
    fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON;
    fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int);
    fn cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON;
    fn cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON;
    fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char);
    fn cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char);
    fn cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool;
    fn cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> cJSON_bool;
    fn cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool;
    fn cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool;
    fn cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool;
    fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON;
    fn cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: cJSON_bool) -> cJSON_bool;
    fn cJSON_Minify(json: *mut c_char);
    fn cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    fn cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    fn cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: cJSON_bool) -> *mut cJSON;
    fn cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON;
    fn cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON;
    fn cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON;
    fn cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    fn cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON;
    fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double;
    fn cJSON_SetValuestring(object: *mut cJSON, valuestring: *const c_char) -> *mut c_char;
    fn cJSON_malloc(size: usize) -> *mut c_void;
    fn cJSON_free(object: *mut c_void);
}

// ---------------------------------------------------------------------------
// library discovery / loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

fn c_lib_path() -> PathBuf {
    ensure_c_built();
    let build = workspace_root().join("c_src").join("build");
    for name in [
        "libcjson.so.1.7.19",
        "libcjson.so.1",
        "libcjson.so",
    ] {
        let p = build.join(name);
        if p.exists() {
            return p;
        }
    }
    panic!("C shared library not found in {}", build.display());
}

fn c_driver_lib_path() -> PathBuf {
    ensure_c_built();
    workspace_root()
        .join("c_src")
        .join("build")
        .join("libcJSON_test.so")
}

/// `cargo test` does not build the `cdylib` artifact, so build it explicitly
/// into a dedicated target directory (a separate `--target-dir` avoids
/// deadlocking on the build lock held by the outer `cargo test`).
fn rust_lib_path() -> PathBuf {
    use std::sync::OnceLock;
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest.join("target").join("ffi");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let mut cmd = std::process::Command::new(&cargo);
        cmd.arg("build").arg("--lib");
        if profile == "release" {
            cmd.arg("--release");
        }
        cmd.arg("--target-dir").arg(&target_dir);
        cmd.current_dir(&manifest);
        // Do not inherit the outer invocation's cargo state.
        cmd.env_remove("CARGO_MAKEFLAGS");
        cmd.env_remove("RUSTC_WRAPPER");
        let status = cmd.status().expect("failed to spawn cargo build for cdylib");
        assert!(status.success(), "cargo build --lib failed");
        let p = target_dir.join(profile).join("libcJSON_test.so");
        assert!(
            p.exists(),
            "Rust shared library not found at {}",
            p.display()
        );
        p
    })
    .clone()
}

fn ensure_c_built() {
    use std::sync::OnceLock;
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let root = workspace_root().join("c_src");
        let build = root.join("build");
        if build.join("libcJSON_test.so").exists() && build.join("libcjson.so").exists() {
            return;
        }
        std::fs::create_dir_all(&build).unwrap();
        let ok = std::process::Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "cmake configure failed");
        let ok = std::process::Command::new("cmake")
            .arg("--build")
            .arg(".")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "cmake build failed");
    });
}

fn leak_lib(path: PathBuf) -> &'static libloading::Library {
    let lib = unsafe {
        libloading::Library::new(&path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {}", path.display(), e))
    };
    Box::leak(Box::new(lib))
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

/// The C and Rust APIs, loaded once per test process.
pub fn apis() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Api::load(leak_lib(c_lib_path())),
        rust: Api::load(leak_lib(rust_lib_path())),
    })
}

/// Both libraries keep process-global state (`global_error`, `global_hooks`),
/// so tests inside one binary must not run concurrently.
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub type DriverFn = unsafe extern "C" fn(
    *const *const c_char,
    *mut [c_int; 3],
    *mut c_int,
    *mut Record,
) -> c_int;

#[repr(C)]
pub struct Record {
    pub precision: *const c_char,
    pub lat: c_double,
    pub lon: c_double,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

pub fn c_driver() -> DriverFn {
    static ONCE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let addr = *ONCE.get_or_init(|| {
        let lib = leak_lib(c_driver_lib_path());
        unsafe {
            let s: libloading::Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
            *s as usize
        }
    });
    unsafe { std::mem::transmute::<usize, DriverFn>(addr) }
}

pub fn rust_driver() -> DriverFn {
    static ONCE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let addr = *ONCE.get_or_init(|| {
        let lib = leak_lib(rust_lib_path());
        unsafe {
            let s: libloading::Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
            *s as usize
        }
    });
    unsafe { std::mem::transmute::<usize, DriverFn>(addr) }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Read a NUL terminated string as raw bytes (may be invalid UTF-8).
pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe { Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec()) }
}

pub unsafe fn cstr_show(p: *const c_char) -> String {
    match unsafe { cstr_bytes(p) } {
        None => "<null>".to_string(),
        Some(b) => format!("{:?}", String::from_utf8_lossy(&b)),
    }
}

/// Canonical, allocation-independent dump of a cJSON tree: every field of every
/// node, in structural order.  Pointer *values* are never compared, only their
/// null-ness / relative structure.
pub unsafe fn dump(item: *const cJSON) -> String {
    let mut out = String::new();
    unsafe { dump_into(item, 0, &mut out) };
    out
}

unsafe fn dump_into(item: *const cJSON, depth: usize, out: &mut String) {
    if item.is_null() {
        out.push_str("<null>\n");
        return;
    }
    if depth > 64 {
        out.push_str("<too deep>\n");
        return;
    }
    unsafe {
        let it = &*item;
        for _ in 0..depth {
            out.push(' ');
        }
        out.push_str(&format!(
            "type={} valueint={} valuedouble={:?}({:#018x}) valuestring={} string={} has_child={} has_next={} has_prev={}\n",
            it.type_,
            it.valueint,
            it.valuedouble,
            it.valuedouble.to_bits(),
            cstr_show(it.valuestring),
            cstr_show(it.string),
            !it.child.is_null(),
            !it.next.is_null(),
            !it.prev.is_null(),
        ));
        // walk children
        let mut child = it.child;
        let mut n = 0;
        while !child.is_null() && n < 4096 {
            dump_into(child, depth + 1, out);
            child = (*child).next;
            n += 1;
        }
    }
}

/// Verify the doubly linked list invariants of a printed tree so that the
/// `prev` pointers (not visible in `dump`) are also compared.
pub unsafe fn dump_links(item: *const cJSON) -> String {
    let mut out = String::new();
    unsafe { dump_links_into(item, 0, &mut out) };
    out
}

unsafe fn dump_links_into(item: *const cJSON, depth: usize, out: &mut String) {
    if item.is_null() || depth > 64 {
        return;
    }
    unsafe {
        let it = &*item;
        let mut child = it.child;
        let mut idx = 0;
        let mut last = std::ptr::null_mut();
        while !child.is_null() && idx < 4096 {
            out.push_str(&format!(
                "d{}#{} prev_is_last={} ",
                depth,
                idx,
                (*child).prev == last
            ));
            last = child;
            dump_links_into(child, depth + 1, out);
            child = (*child).next;
            idx += 1;
        }
        if idx > 0 {
            // in cJSON, child->prev points to the last element
            out.push_str(&format!(
                "d{} childprev_is_last={}\n",
                depth,
                (*it.child).prev == last
            ));
        }
    }
}

/// Print with both formatting modes and return the raw bytes.
pub unsafe fn print_both(api: &Api, item: *const cJSON) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    unsafe {
        let f = api.cJSON_Print(item);
        let u = api.cJSON_PrintUnformatted(item);
        let fb = cstr_bytes(f);
        let ub = cstr_bytes(u);
        if !f.is_null() {
            api.cJSON_free(f as *mut c_void);
        }
        if !u.is_null() {
            api.cJSON_free(u as *mut c_void);
        }
        (fb, ub)
    }
}

/// Assert that a tree built the same way in both libraries is identical.
pub unsafe fn assert_tree_eq(ctx: &str, c: *const cJSON, r: *const cJSON) {
    unsafe {
        let cd = dump(c);
        let rd = dump(r);
        assert_eq!(cd, rd, "tree mismatch [{ctx}]\nC:\n{cd}\nRust:\n{rd}");
        let cl = dump_links(c);
        let rl = dump_links(r);
        assert_eq!(cl, rl, "link mismatch [{ctx}]");
        let (cf, cu) = print_both(&apis().c, c);
        let (rf, ru) = print_both(&apis().rust, r);
        assert_eq!(
            cf.as_deref().map(String::from_utf8_lossy),
            rf.as_deref().map(String::from_utf8_lossy),
            "formatted print mismatch [{ctx}]"
        );
        assert_eq!(
            cu.as_deref().map(String::from_utf8_lossy),
            ru.as_deref().map(String::from_utf8_lossy),
            "unformatted print mismatch [{ctx}]"
        );
    }
}
