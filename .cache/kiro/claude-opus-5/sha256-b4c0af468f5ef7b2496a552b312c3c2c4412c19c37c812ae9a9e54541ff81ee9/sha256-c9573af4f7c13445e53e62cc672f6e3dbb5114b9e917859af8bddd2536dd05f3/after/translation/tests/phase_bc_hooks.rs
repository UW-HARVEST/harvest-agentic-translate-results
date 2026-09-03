//! CONFIGS.md rows 85-89 (`cJSON_InitHooks` configurations) and every
//! ERRORS.md row marked **[alloc]**: allocation-failure paths are reached by
//! installing a hook that fails the N-th allocation and sweeping N.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering::SeqCst};

/* ------------------------------------------------------------------ */
/* instrumented allocator                                             */
/* ------------------------------------------------------------------ */

static ALLOC_N: AtomicUsize = AtomicUsize::new(0);
static FREE_N: AtomicUsize = AtomicUsize::new(0);
static FAIL_AT: AtomicIsize = AtomicIsize::new(-1);
static LOG_SIZES: AtomicUsize = AtomicUsize::new(0); // 1 = record sizes

static SIZES: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

unsafe extern "C" fn rec_malloc(size: usize) -> *mut c_void {
    let n = ALLOC_N.fetch_add(1, SeqCst);
    if LOG_SIZES.load(SeqCst) == 1 {
        SIZES.lock().unwrap().push(size);
    }
    let f = FAIL_AT.load(SeqCst);
    if f >= 0 && (n as isize) == f {
        return std::ptr::null_mut();
    }
    libc::malloc(size)
}

unsafe extern "C" fn rec_free(p: *mut c_void) {
    FREE_N.fetch_add(1, SeqCst);
    libc::free(p);
}

unsafe extern "C" fn plain_malloc(size: usize) -> *mut c_void {
    libc::malloc(size)
}

unsafe extern "C" fn plain_free(p: *mut c_void) {
    libc::free(p);
}

fn reset_counters(fail_at: isize, log_sizes: bool) {
    ALLOC_N.store(0, SeqCst);
    FREE_N.store(0, SeqCst);
    FAIL_AT.store(fail_at, SeqCst);
    LOG_SIZES.store(if log_sizes { 1 } else { 0 }, SeqCst);
    SIZES.lock().unwrap().clear();
}

unsafe fn install_recording_hooks(api: &Api) {
    reset_counters(-1, false);
    let mut h = CJsonHooks {
        malloc_fn: Some(rec_malloc),
        free_fn: Some(rec_free),
    };
    (api.cJSON_InitHooks)(&mut h);
}

unsafe fn restore_default_hooks(api: &Api) {
    (api.cJSON_InitHooks)(std::ptr::null_mut());
    // Never leave a pending injected failure behind for the next test.
    reset_counters(-1, false);
}

/* ------------------------------------------------------------------ */
/* workloads                                                          */
/* ------------------------------------------------------------------ */

/// A workload observable purely through return values, so C and Rust can be
/// compared even when an allocation fails half way through.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    labels: Vec<(String, String)>,
    allocs: usize,
}

const DOC: &str = "{\"name\":\"Jack \\\"Bee\\\" Nimble\",\"fmt\":{\"type\":\"rect\",\
\"width\":1920,\"height\":1080,\"interlace\":false,\"frame rate\":24},\
\"arr\":[1,2,3,\"a\\u0041b\",null,true,{},[]],\"deep\":{\"a\":{\"b\":{\"c\":[1e10,-2.5]}}}}";

unsafe fn workload(api: &Api) -> Outcome {
    let mut labels: Vec<(String, String)> = Vec::new();
    macro_rules! rec {
        ($k:expr, $v:expr) => {
            labels.push(($k.to_string(), $v));
        };
    }

    // --- parse -------------------------------------------------------
    let buf = cbytes(DOC.as_bytes());
    let root = (api.cJSON_Parse)(buf.as_ptr());
    rec!("parse.null", root.is_null().to_string());
    rec!(
        "parse.errptr",
        {
            let e = (api.cJSON_GetErrorPtr)();
            if e.is_null() { "null".to_string() } else { e.offset_from(buf.as_ptr()).to_string() }
        }
    );

    if !root.is_null() {
        rec!("size", (api.cJSON_GetArraySize)(root).to_string());

        // --- print (all four variants) -------------------------------
        for (name, ptr) in [
            ("Print", (api.cJSON_Print)(root)),
            ("PrintUnformatted", (api.cJSON_PrintUnformatted)(root)),
            ("PrintBuffered0", (api.cJSON_PrintBuffered)(root, 0, 1)),
            ("PrintBuffered4096", (api.cJSON_PrintBuffered)(root, 4096, 0)),
        ] {
            rec!(name, show(&take_printed(api, ptr)));
        }
        let mut pre = vec![0u8; 4096];
        rec!(
            "PrintPreallocated",
            format!(
                "{} {}",
                (api.cJSON_PrintPreallocated)(root, pre.as_mut_ptr() as *mut c_char, 4096, 1),
                String::from_utf8_lossy(
                    &pre[..pre.iter().position(|&b| b == 0).unwrap_or(0)]
                )
            )
        );

        // --- duplicate + compare -------------------------------------
        for rec_flag in [0i32, 1] {
            let d = (api.cJSON_Duplicate)(root, rec_flag);
            rec!(format!("dup{rec_flag}.null"), d.is_null().to_string());
            rec!(
                format!("dup{rec_flag}.cmp"),
                (api.cJSON_Compare)(root, d, 1).to_string()
            );
            rec!(
                format!("dup{rec_flag}.print"),
                show(&take_printed(api, (api.cJSON_PrintUnformatted)(d)))
            );
            (api.cJSON_Delete)(d);
        }

        // --- mutation -------------------------------------------------
        let k = cs("added");
        let n = (api.cJSON_AddNumberToObject)(root, k.as_ptr(), 42.0);
        rec!("addnum.null", n.is_null().to_string());
        let s = cs("str");
        let si = (api.cJSON_AddStringToObject)(root, s.as_ptr(), s.as_ptr());
        rec!("addstr.null", si.is_null().to_string());
        let raw = cs("raw");
        rec!(
            "addraw.null",
            (api.cJSON_AddRawToObject)(root, raw.as_ptr(), raw.as_ptr())
                .is_null()
                .to_string()
        );
        rec!(
            "addobj.null",
            (api.cJSON_AddObjectToObject)(root, cs("o").as_ptr())
                .is_null()
                .to_string()
        );
        rec!(
            "addarr.null",
            (api.cJSON_AddArrayToObject)(root, cs("a").as_ptr())
                .is_null()
                .to_string()
        );
        let repl = (api.cJSON_CreateNumber)(7.0);
        rec!("createnum.null", repl.is_null().to_string());
        rec!(
            "replace",
            (api.cJSON_ReplaceItemInObject)(root, cs("name").as_ptr(), repl).to_string()
        );

        // --- SetValuestring (short and long) --------------------------
        let sitem = (api.cJSON_CreateString)(cs("abcdef").as_ptr());
        rec!("createstr.null", sitem.is_null().to_string());
        if !sitem.is_null() {
            rec!(
                "setvs.short",
                show(&take_cstr((api.cJSON_SetValuestring)(sitem, cs("ab").as_ptr())))
            );
            rec!(
                "setvs.long",
                show(&take_cstr((api.cJSON_SetValuestring)(
                    sitem,
                    cs("a much longer replacement").as_ptr()
                )))
            );
        }
        (api.cJSON_Delete)(sitem);

        // --- typed array constructors --------------------------------
        let ints = [1i32, -2, 3];
        let ia = (api.cJSON_CreateIntArray)(ints.as_ptr(), 3);
        rec!("intarray", show(&take_printed(api, (api.cJSON_PrintUnformatted)(ia))));
        (api.cJSON_Delete)(ia);
        let owned: Vec<Vec<c_char>> = ["x", "yy", "zzz"].iter().map(|s| cbytes(s.as_bytes())).collect();
        let ptrs: Vec<*const c_char> = owned.iter().map(|v| v.as_ptr()).collect();
        let sa = (api.cJSON_CreateStringArray)(ptrs.as_ptr(), 3);
        rec!("strarray", show(&take_printed(api, (api.cJSON_PrintUnformatted)(sa))));
        (api.cJSON_Delete)(sa);

        rec!(
            "final",
            show(&take_printed(api, (api.cJSON_PrintUnformatted)(root)))
        );
    }
    (api.cJSON_Delete)(root);

    Outcome {
        labels,
        allocs: ALLOC_N.load(SeqCst),
    }
}

/* ================================================================== */
/* CONFIGS rows 85-88: hook configurations                             */
/* ================================================================== */

#[test]
fn rows_85_88_hook_configurations() {
    let _g = lock();
    let p = pair();
    unsafe {
        // (malloc_fn, free_fn) combinations, plus the NULL-hooks reset.
        let combos: [(Option<MallocFn>, Option<FreeFn>); 4] = [
            (Some(plain_malloc), Some(plain_free)),
            (None, Some(plain_free)),
            (Some(plain_malloc), None),
            (None, None),
        ];
        for (i, (m, f)) in combos.into_iter().enumerate() {
            let mut hc = CJsonHooks { malloc_fn: m, free_fn: f };
            let mut hr = CJsonHooks { malloc_fn: m, free_fn: f };
            (p.c.cJSON_InitHooks)(&mut hc);
            (p.r.cJSON_InitHooks)(&mut hr);

            reset_counters(-1, false);
            let co = workload(p.c);
            reset_counters(-1, false);
            let ro = workload(p.r);
            assert_eq!(co.labels, ro.labels, "hook combo #{i}: workload output differs");

            // row 85: NULL hooks reset
            (p.c.cJSON_InitHooks)(std::ptr::null_mut());
            (p.r.cJSON_InitHooks)(std::ptr::null_mut());
            reset_counters(-1, false);
            let co = workload(p.c);
            reset_counters(-1, false);
            let ro = workload(p.r);
            assert_eq!(
                co.labels, ro.labels,
                "hook combo #{i}: workload after NULL-hook reset differs"
            );
        }
        restore_default_hooks(p.c);
        restore_default_hooks(p.r);
    }
}

/* ================================================================== */
/* CONFIGS row 86 + 89: custom hooks disable realloc; allocation trace  */
/* ================================================================== */

#[test]
fn row_86_89_allocation_sequence_matches() {
    let _g = lock();
    let p = pair();
    unsafe {
        install_recording_hooks(p.c);
        install_recording_hooks(p.r);

        reset_counters(-1, true);
        let co = workload(p.c);
        let csizes = SIZES.lock().unwrap().clone();
        let cfrees = FREE_N.load(SeqCst);

        reset_counters(-1, true);
        let ro = workload(p.r);
        let rsizes = SIZES.lock().unwrap().clone();
        let rfrees = FREE_N.load(SeqCst);

        assert_eq!(co.labels, ro.labels, "workload output differs under custom hooks");
        assert_eq!(
            co.allocs, ro.allocs,
            "allocation count differs: C={} R={}",
            co.allocs, ro.allocs
        );
        assert_eq!(
            csizes, rsizes,
            "allocation size sequence differs\n C: {:?}\n R: {:?}",
            csizes, rsizes
        );
        assert_eq!(cfrees, rfrees, "free count differs: C={cfrees} R={rfrees}");
        assert!(csizes.len() > 50, "workload should allocate a lot; got {}", csizes.len());

        restore_default_hooks(p.c);
        restore_default_hooks(p.r);
    }
}

/* ================================================================== */
/* ERRORS.md [alloc] rows: fail the N-th allocation, sweep N            */
/* ================================================================== */

#[test]
fn alloc_failure_sweep_full_workload() {
    let _g = lock();
    let p = pair();
    unsafe {
        install_recording_hooks(p.c);
        install_recording_hooks(p.r);

        // How many allocations does a clean run need?
        reset_counters(-1, false);
        let base = workload(p.c);
        let total = base.allocs;
        assert!(total > 50, "expected a substantial workload, got {total} allocations");

        for n in 0..total {
            reset_counters(n as isize, false);
            let co = workload(p.c);
            reset_counters(n as isize, false);
            let ro = workload(p.r);
            assert_eq!(
                co.labels, ro.labels,
                "allocation #{n} failure: workload output differs\n C: {:?}\n R: {:?}",
                co.labels, ro.labels
            );
        }

        restore_default_hooks(p.c);
        restore_default_hooks(p.r);
    }
}

/// Targeted sweeps for the small entry points whose allocation-failure paths
/// are otherwise buried inside the big workload.
#[test]
fn alloc_failure_sweep_individual_entry_points() {
    let _g = lock();
    let p = pair();

    type Probe = fn(&Api) -> String;

    let probes: Vec<(&str, Probe)> = vec![
        ("CreateNull", |a| unsafe {
            let x = (a.cJSON_CreateNull)();
            let s = x.is_null().to_string();
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateNumber", |a| unsafe {
            let x = (a.cJSON_CreateNumber)(1.5);
            let s = x.is_null().to_string();
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateString", |a| unsafe {
            let v = cs("hello");
            let x = (a.cJSON_CreateString)(v.as_ptr());
            let s = format!("{} {}", x.is_null(), show(&take_cstr((a.cJSON_GetStringValue)(x))));
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateRaw", |a| unsafe {
            let v = cs("raw");
            let x = (a.cJSON_CreateRaw)(v.as_ptr());
            let s = x.is_null().to_string();
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateIntArray", |a| unsafe {
            let n = [1i32, 2, 3, 4];
            let x = (a.cJSON_CreateIntArray)(n.as_ptr(), 4);
            let s = format!("{} {}", x.is_null(), show(&take_printed(a, (a.cJSON_PrintUnformatted)(x))));
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateFloatArray", |a| unsafe {
            let n = [1.5f32, 2.5];
            let x = (a.cJSON_CreateFloatArray)(n.as_ptr(), 2);
            let s = format!("{} {}", x.is_null(), show(&take_printed(a, (a.cJSON_PrintUnformatted)(x))));
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateDoubleArray", |a| unsafe {
            let n = [1.5f64, 2.5];
            let x = (a.cJSON_CreateDoubleArray)(n.as_ptr(), 2);
            let s = format!("{} {}", x.is_null(), show(&take_printed(a, (a.cJSON_PrintUnformatted)(x))));
            (a.cJSON_Delete)(x);
            s
        }),
        ("CreateStringArray", |a| unsafe {
            let o: Vec<Vec<c_char>> = ["a", "bb"].iter().map(|s| cbytes(s.as_bytes())).collect();
            let ptrs: Vec<*const c_char> = o.iter().map(|v| v.as_ptr()).collect();
            let x = (a.cJSON_CreateStringArray)(ptrs.as_ptr(), 2);
            let s = format!("{} {}", x.is_null(), show(&take_printed(a, (a.cJSON_PrintUnformatted)(x))));
            (a.cJSON_Delete)(x);
            s
        }),
        ("AddItemToObject", |a| unsafe {
            let o = (a.cJSON_CreateObject)();
            let k = cs("key");
            let n = (a.cJSON_CreateNumber)(1.0);
            let r = (a.cJSON_AddItemToObject)(o, k.as_ptr(), n);
            let s = format!("{r} {}", show(&take_printed(a, (a.cJSON_PrintUnformatted)(o))));
            if r == 0 {
                (a.cJSON_Delete)(n);
            }
            (a.cJSON_Delete)(o);
            s
        }),
        ("AddItemReferenceToArray", |a| unsafe {
            let arr = (a.cJSON_CreateArray)();
            let sub = (a.cJSON_CreateNumber)(1.0);
            let r = (a.cJSON_AddItemReferenceToArray)(arr, sub);
            let s = format!("{r} {}", show(&take_printed(a, (a.cJSON_PrintUnformatted)(arr))));
            (a.cJSON_Delete)(arr);
            (a.cJSON_Delete)(sub);
            s
        }),
        ("ReplaceItemInObject", |a| unsafe {
            let o = (a.cJSON_CreateObject)();
            let k = cs("k");
            (a.cJSON_AddNumberToObject)(o, k.as_ptr(), 1.0);
            let n = (a.cJSON_CreateNumber)(2.0);
            let r = (a.cJSON_ReplaceItemInObject)(o, k.as_ptr(), n);
            let s = format!("{r} {}", show(&take_printed(a, (a.cJSON_PrintUnformatted)(o))));
            if r == 0 {
                (a.cJSON_Delete)(n);
            }
            (a.cJSON_Delete)(o);
            s
        }),
        ("SetValuestringLonger", |a| unsafe {
            let v = cs("ab");
            let x = (a.cJSON_CreateString)(v.as_ptr());
            let s = if x.is_null() {
                "null-item".to_string()
            } else {
                show(&take_cstr((a.cJSON_SetValuestring)(x, cs("abcdef").as_ptr())))
            };
            (a.cJSON_Delete)(x);
            s
        }),
        ("Parse", |a| unsafe {
            let b = cbytes(b"{\"a\":[1,2,\"xyz\"],\"b\":null}");
            let x = (a.cJSON_Parse)(b.as_ptr());
            let e = (a.cJSON_GetErrorPtr)();
            let s = format!(
                "{} {} {}",
                x.is_null(),
                if e.is_null() { -1 } else { e.offset_from(b.as_ptr()) },
                show(&take_printed(a, (a.cJSON_PrintUnformatted)(x)))
            );
            (a.cJSON_Delete)(x);
            s
        }),
        ("ParseLongString", |a| unsafe {
            let b = cbytes(b"\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\\u0041\\ud83d\\ude00\"");
            let x = (a.cJSON_Parse)(b.as_ptr());
            let s = format!("{} {}", x.is_null(), show(&take_cstr((a.cJSON_GetStringValue)(x))));
            (a.cJSON_Delete)(x);
            s
        }),
        ("PrintGrowth", |a| unsafe {
            let arr = (a.cJSON_CreateArray)();
            for i in 0..80 {
                (a.cJSON_AddItemToArray)(arr, (a.cJSON_CreateNumber)(i as f64 * 1.25));
            }
            let s = format!(
                "{}|{}",
                show(&take_printed(a, (a.cJSON_Print)(arr))),
                show(&take_printed(a, (a.cJSON_PrintUnformatted)(arr)))
            );
            (a.cJSON_Delete)(arr);
            s
        }),
        ("PrintBufferedTiny", |a| unsafe {
            let arr = (a.cJSON_CreateArray)();
            for i in 0..40 {
                (a.cJSON_AddItemToArray)(arr, (a.cJSON_CreateNumber)(i as f64));
            }
            let s = show(&take_printed(a, (a.cJSON_PrintBuffered)(arr, 1, 1)));
            (a.cJSON_Delete)(arr);
            s
        }),
        ("DuplicateDeep", |a| unsafe {
            let b = cbytes(b"{\"a\":{\"b\":[1,\"s\",{\"c\":2}]},\"d\":\"e\"}");
            let src = (a.cJSON_Parse)(b.as_ptr());
            let d = (a.cJSON_Duplicate)(src, 1);
            let s = format!("{} {}", d.is_null(), show(&take_printed(a, (a.cJSON_PrintUnformatted)(d))));
            (a.cJSON_Delete)(d);
            (a.cJSON_Delete)(src);
            s
        }),
        ("DuplicateConstKey", |a| unsafe {
            let o = (a.cJSON_CreateObject)();
            let k = cs("ck");
            (a.cJSON_AddItemToObjectCS)(o, k.as_ptr(), (a.cJSON_CreateNumber)(1.0));
            let d = (a.cJSON_Duplicate)(o, 1);
            let s = format!("{} {}", d.is_null(), show(&take_printed(a, (a.cJSON_PrintUnformatted)(d))));
            (a.cJSON_Delete)(d);
            (a.cJSON_Delete)(o);
            s
        }),
    ];

    unsafe {
        install_recording_hooks(p.c);
        install_recording_hooks(p.r);
        for (name, probe) in &probes {
            reset_counters(-1, false);
            probe(p.c);
            let total = ALLOC_N.load(SeqCst);
            // +2 so that "fail after the last allocation" is also covered.
            for n in 0..(total + 2) {
                reset_counters(n as isize, false);
                let cout = probe(p.c);
                let cn = ALLOC_N.load(SeqCst);
                reset_counters(n as isize, false);
                let rout = probe(p.r);
                let rn = ALLOC_N.load(SeqCst);
                assert_eq!(
                    cout, rout,
                    "{name}: fail-at-{n} output differs\n C: {cout}\n R: {rout}"
                );
                assert_eq!(cn, rn, "{name}: fail-at-{n} allocation count differs");
            }
        }
        restore_default_hooks(p.c);
        restore_default_hooks(p.r);
    }
}

/* ================================================================== */
/* the hook-selection logic itself                                     */
/* ================================================================== */

#[test]
fn init_hooks_realloc_selection_is_observable_via_alloc_trace() {
    let _g = lock();
    let p = pair();
    unsafe {
        // Custom hooks => `reallocate` must be NULL in both, so `ensure`
        // takes the allocate+memcpy path. The allocation trace makes the
        // difference observable: the growth path allocates a *new* buffer for
        // every step instead of calling realloc.
        install_recording_hooks(p.c);
        install_recording_hooks(p.r);

        let build = |a: &Api| -> *mut CJson {
            let arr = (a.cJSON_CreateArray)();
            for i in 0..120 {
                (a.cJSON_AddItemToArray)(arr, (a.cJSON_CreateNumber)(i as f64 * 3.5));
            }
            arr
        };

        let carr = build(p.c);
        let rarr = build(p.r);

        reset_counters(-1, true);
        let cout = take_printed(p.c, (p.c.cJSON_Print)(carr));
        let ctrace = SIZES.lock().unwrap().clone();

        reset_counters(-1, true);
        let rout = take_printed(p.r, (p.r.cJSON_Print)(rarr));
        let rtrace = SIZES.lock().unwrap().clone();

        assert!(cout == rout, "print output differs under custom hooks\n C({}): {}\n R({}): {}", cout.as_ref().map(|v|v.len()).unwrap_or(0), show(&cout), rout.as_ref().map(|v|v.len()).unwrap_or(0), show(&rout));
        assert_eq!(
            ctrace, rtrace,
            "print-buffer growth allocation trace differs\n C: {:?}\n R: {:?}",
            ctrace, rtrace
        );
        assert!(
            ctrace.len() >= 3,
            "expected several growth steps, got {:?}",
            ctrace
        );

        (p.c.cJSON_Delete)(carr);
        (p.r.cJSON_Delete)(rarr);
        restore_default_hooks(p.c);
        restore_default_hooks(p.r);
    }
}

#[test]
fn hooks_do_not_leak_into_later_tests() {
    let _g = lock();
    let p = pair();
    unsafe {
        restore_default_hooks(p.c);
        restore_default_hooks(p.r);
        let b = cbytes(b"[1,2,3]");
        let c = (p.c.cJSON_Parse)(b.as_ptr());
        let r = (p.r.cJSON_Parse)(b.as_ptr());
        let co = take_printed(p.c, (p.c.cJSON_Print)(c));
        let ro = take_printed(p.r, (p.r.cJSON_Print)(r));
        assert!(co == ro);
        assert!(co.is_some());
        (p.c.cJSON_Delete)(c);
        (p.r.cJSON_Delete)(r);
        let _ = c_int::from(0);
    }
}
