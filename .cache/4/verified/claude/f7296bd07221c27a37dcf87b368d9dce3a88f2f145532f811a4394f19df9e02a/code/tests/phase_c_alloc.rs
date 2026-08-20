//! Phase C — allocation-failure rows of `ERRORS.md`
//! (8, 9, 12, 19, 26, 27, 41, 52, 55, 57, 60, 111, 119, 146, 152, 155, 156,
//! 157, 177) plus rows 26/27 (`ensure` realloc/allocate failure).
//!
//! A `fail-on-the-Nth-malloc` allocator is installed into **both** libraries via
//! `cJSON_InitHooks`, and the whole scenario is replayed for every N. This
//! verifies not only that both implementations fail identically, but that they
//! request exactly the same number of allocations in exactly the same order.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_void};
use std::fmt::Write as _;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::Mutex;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

static LOCK: Mutex<()> = Mutex::new(());

static COUNT: AtomicUsize = AtomicUsize::new(0);
static FAIL_AT: AtomicIsize = AtomicIsize::new(-1);

unsafe extern "C" fn failing_malloc(size: usize) -> *mut c_void {
    let n = COUNT.fetch_add(1, Ordering::SeqCst) as isize;
    let fail = FAIL_AT.load(Ordering::SeqCst);
    if fail >= 0 && n == fail {
        return null_mut();
    }
    malloc(size)
}

unsafe extern "C" fn counting_free(p: *mut c_void) {
    free(p)
}

/// Everything that can fail an allocation, in a fixed order.
unsafe fn scenario(api: &Api, log: &mut String) {
    let text = cs("{\"key\":[1,2.5,\"str\\u0041\",null,true,{\"n\":{}}]}");
    let name = cs("added");
    let long = cs("a much longer replacement string");
    let short = cs("s");

    // rows 9 + 8 + 148/149
    let it = (api.cJSON_CreateString)(short.as_ptr());
    let _ = writeln!(log, "CreateString null={}", it.is_null());
    let raw = (api.cJSON_CreateRaw)(short.as_ptr());
    let _ = writeln!(log, "CreateRaw null={}", raw.is_null());
    let num = (api.cJSON_CreateNumber)(1.5);
    let _ = writeln!(log, "CreateNumber null={}", num.is_null());
    let arr = (api.cJSON_CreateArray)();
    let _ = writeln!(log, "CreateArray null={}", arr.is_null());
    let obj = (api.cJSON_CreateObject)();
    let _ = writeln!(log, "CreateObject null={}", obj.is_null());

    // row 119: strdup of the key
    let _ = writeln!(
        log,
        "AddItemToObject rc={}",
        (api.cJSON_AddItemToObject)(obj, name.as_ptr(), num)
    );
    let _ = write!(log, "obj: {}", dump(obj));

    // row 111: create_reference
    let _ = writeln!(
        log,
        "AddItemReferenceToArray rc={}",
        (api.cJSON_AddItemReferenceToArray)(arr, obj)
    );

    // rows 152: element creation inside the bulk constructors
    let ints = [1i32, 2, 3];
    let ia = (api.cJSON_CreateIntArray)(ints.as_ptr(), 3);
    let _ = writeln!(log, "CreateIntArray null={}", ia.is_null());
    let _ = write!(log, "ia: {}", dump(ia));
    let s1 = cs("one");
    let s2 = cs("two");
    let strs: [*const c_char; 2] = [s1.as_ptr(), s2.as_ptr()];
    let sa = (api.cJSON_CreateStringArray)(strs.as_ptr(), 2);
    let _ = writeln!(log, "CreateStringArray null={}", sa.is_null());
    let _ = write!(log, "sa: {}", dump(sa));

    // rows 52 + 12 + 41 + 75/86: the parser
    let root = (api.cJSON_Parse)(text.as_ptr());
    let _ = writeln!(log, "Parse null={}", root.is_null());
    let _ = write!(log, "root: {}", dump(root));

    // rows 55 + 57 + 26/27 + 30/46: the printers
    let pf = take_print(api, (api.cJSON_Print)(root));
    let _ = writeln!(log, "Print={}", pf.map(|v| show(&v)).unwrap_or("NULL".into()));
    let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
    let _ = writeln!(
        log,
        "PrintUnformatted={}",
        pu.map(|v| show(&v)).unwrap_or("NULL".into())
    );
    // row 60: cJSON_PrintBuffered's own prebuffer allocation
    for pre in [0i32, 4, 512] {
        let pb = take_print(api, (api.cJSON_PrintBuffered)(root, pre, 1));
        let _ = writeln!(
            log,
            "PrintBuffered({pre})={}",
            pb.map(|v| show(&v)).unwrap_or("NULL".into())
        );
    }

    // row 19: SetValuestring taking the strdup path
    if !it.is_null() {
        let ret = (api.cJSON_SetValuestring)(it, long.as_ptr());
        let _ = writeln!(
            log,
            "SetValuestring null={} now={:?}",
            ret.is_null(),
            read_cstr((*it).valuestring).map(|v| show(&v))
        );
    } else {
        let _ = writeln!(log, "SetValuestring skipped");
    }

    // row 146: replace_item_in_object strdup
    let _ = writeln!(
        log,
        "ReplaceItemInObject rc={}",
        (api.cJSON_ReplaceItemInObject)(obj, name.as_ptr(), (api.cJSON_CreateNumber)(9.0))
    );
    let _ = write!(log, "obj after replace: {}", dump(obj));

    // rows 155..157: duplicate
    for recurse in [0i32, 1] {
        let d = (api.cJSON_Duplicate)(root, recurse);
        let _ = writeln!(log, "Duplicate({recurse}) null={}", d.is_null());
        let _ = write!(log, "  {}", dump(d));
        (api.cJSON_Delete)(d);
    }
    // duplicate an object with keys (row 157) and with a valuestring (row 156)
    let d = (api.cJSON_Duplicate)(obj, 1);
    let _ = writeln!(log, "Duplicate(obj) null={}", d.is_null());
    let _ = write!(log, "  {}", dump(d));
    (api.cJSON_Delete)(d);

    // row 177
    let p = (api.cJSON_malloc)(32);
    let _ = writeln!(log, "cJSON_malloc null={}", p.is_null());
    (api.cJSON_free)(p);

    (api.cJSON_Delete)(root);
    (api.cJSON_Delete)(ia);
    (api.cJSON_Delete)(sa);
    (api.cJSON_Delete)(arr);
    (api.cJSON_Delete)(obj);
    (api.cJSON_Delete)(it);
    (api.cJSON_Delete)(raw);
}

fn run(fail_at: isize) {
    diff(&format!("ERRORS alloc-failure at #{fail_at}"), |api| unsafe {
        let mut hooks = CJsonHooks {
            malloc_fn: Some(failing_malloc),
            free_fn: Some(counting_free),
        };
        (api.cJSON_InitHooks)(&mut hooks);
        COUNT.store(0, Ordering::SeqCst);
        FAIL_AT.store(fail_at, Ordering::SeqCst);

        let mut log = String::new();
        scenario(api, &mut log);

        FAIL_AT.store(-1, Ordering::SeqCst);
        let _ = writeln!(log, "total allocations = {}", COUNT.load(Ordering::SeqCst));
        (api.cJSON_InitHooks)(null_mut());
        log
    });
}

#[test]
fn alloc_failure_at_every_allocation() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // baseline: how many allocations does the scenario perform?
    run(-1);
    let total = COUNT.load(Ordering::SeqCst);
    assert!(total > 20, "scenario should allocate a lot, got {total}");
    println!("baseline allocations per scenario: {total}");
    for fail_at in 0..(total as isize + 2) {
        run(fail_at);
    }
}
