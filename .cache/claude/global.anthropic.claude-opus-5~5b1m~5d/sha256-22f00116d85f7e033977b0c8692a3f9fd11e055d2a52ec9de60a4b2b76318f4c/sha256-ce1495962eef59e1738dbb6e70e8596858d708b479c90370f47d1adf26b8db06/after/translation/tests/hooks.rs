//! Phase B — CONFIGS.md rows 66–71 (all six `cJSON_InitHooks` configurations)
//! and Phase C — the ERRORS.md rows that can only be reached with a failing
//! allocator (7, 8, 10, 18, 26, 40, 51, 54, 57, 59, 74, 83, 103, 112, 148, 154,
//! 157, 160, 165, 166, 167).
//!
//! `cJSON_InitHooks` mutates per-library process-wide state, so every test here
//! holds `harness::lock_global_state()` for its whole body and resets the hooks
//! before returning.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicI64, Ordering};

// ---------------------------------------------------------------------------
// allocators installed through cJSON_InitHooks
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc_fn(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free_fn(p: *mut c_void);
}

/// Remaining successful allocations; `<= 0` makes `budget_malloc` return NULL.
static BUDGET: AtomicI64 = AtomicI64::new(i64::MAX);
/// Number of allocation attempts since the last `reset_budget`.
static ATTEMPTS: AtomicI64 = AtomicI64::new(0);

fn set_budget(n: i64) {
    BUDGET.store(n, Ordering::SeqCst);
    ATTEMPTS.store(0, Ordering::SeqCst);
}

fn attempts() -> i64 {
    ATTEMPTS.load(Ordering::SeqCst)
}

/// A distinct function pointer from libc `malloc`, so `cJSON_InitHooks` leaves
/// `reallocate == NULL` and the manual grow/copy paths are taken.
unsafe extern "C" fn budget_malloc(n: usize) -> *mut c_void {
    ATTEMPTS.fetch_add(1, Ordering::SeqCst);
    if BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        return std::ptr::null_mut();
    }
    libc_malloc_fn(n)
}

unsafe extern "C" fn budget_free(p: *mut c_void) {
    libc_free_fn(p)
}

/// Call counters for `plain_malloc` / `plain_free`.
///
/// These make the `reallocate` selection of `cJSON_InitHooks` (cJSON.c:200-204)
/// OBSERVABLE: when `reallocate == NULL`, `ensure` grows the buffer manually
/// with `allocate` + `memcpy` + `deallocate` (cJSON.c:518-529) and `print`
/// finalises with `allocate` + `memcpy` (cJSON.c:1237); when `reallocate` is
/// `realloc`, neither hook is called for those steps.  Comparing the counts
/// between the two libraries therefore proves they made the same decision.
static PLAIN_ALLOCS: AtomicI64 = AtomicI64::new(0);
static PLAIN_FREES: AtomicI64 = AtomicI64::new(0);

fn reset_plain_counts() {
    PLAIN_ALLOCS.store(0, Ordering::SeqCst);
    PLAIN_FREES.store(0, Ordering::SeqCst);
}

fn plain_counts() -> (i64, i64) {
    (
        PLAIN_ALLOCS.load(Ordering::SeqCst),
        PLAIN_FREES.load(Ordering::SeqCst),
    )
}

/// Never fails; still a distinct pointer from libc `malloc`.
unsafe extern "C" fn plain_malloc(n: usize) -> *mut c_void {
    PLAIN_ALLOCS.fetch_add(1, Ordering::SeqCst);
    libc_malloc_fn(n)
}

unsafe extern "C" fn plain_free(p: *mut c_void) {
    PLAIN_FREES.fetch_add(1, Ordering::SeqCst);
    libc_free_fn(p)
}

/// The *real* libc `malloc`/`free` addresses as the loaded shared objects see
/// them.  Taking `&malloc` inside an executable can yield that executable's PLT
/// stub instead, which is not what `cJSON_InitHooks` compares against, so they
/// are resolved with `dlsym` on the global scope.
fn libc_pair() -> (
    unsafe extern "C" fn(usize) -> *mut c_void,
    unsafe extern "C" fn(*mut c_void),
) {
    use libloading::os::unix::Library;
    unsafe {
        let this = Library::this();
        let m = this
            .get::<unsafe extern "C" fn(usize) -> *mut c_void>(b"malloc\0")
            .expect("dlsym(malloc)");
        let f = this
            .get::<unsafe extern "C" fn(*mut c_void)>(b"free\0")
            .expect("dlsym(free)");
        (*m, *f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cfg {
    /// row 66 — `cJSON_InitHooks(NULL)`
    A0Reset,
    /// row 67 — libc `malloc`/`free` passed explicitly
    A1Libc,
    /// row 68 — custom malloc + custom free
    A2Custom,
    /// row 69 — custom malloc, `free_fn == NULL`
    A3CustomMallocOnly,
    /// row 70 — `malloc_fn == NULL`, custom free
    A4CustomFreeOnly,
    /// row 71 — both members NULL
    A5BothNull,
    /// budget allocator (custom malloc + custom free) for failure injection
    Budget,
}

fn install(api: &Api, cfg: Cfg) {
    unsafe {
        match cfg {
            Cfg::A0Reset => (api.cJSON_InitHooks)(std::ptr::null_mut()),
            Cfg::A1Libc => {
                let (m, f) = libc_pair();
                let mut h = CJsonHooks {
                    malloc_fn: Some(m),
                    free_fn: Some(f),
                };
                (api.cJSON_InitHooks)(&mut h)
            }
            Cfg::A2Custom => {
                let mut h = CJsonHooks {
                    malloc_fn: Some(plain_malloc),
                    free_fn: Some(plain_free),
                };
                (api.cJSON_InitHooks)(&mut h)
            }
            Cfg::A3CustomMallocOnly => {
                let mut h = CJsonHooks {
                    malloc_fn: Some(plain_malloc),
                    free_fn: None,
                };
                (api.cJSON_InitHooks)(&mut h)
            }
            Cfg::A4CustomFreeOnly => {
                let mut h = CJsonHooks {
                    malloc_fn: None,
                    free_fn: Some(plain_free),
                };
                (api.cJSON_InitHooks)(&mut h)
            }
            Cfg::A5BothNull => {
                let mut h = CJsonHooks {
                    malloc_fn: None,
                    free_fn: None,
                };
                (api.cJSON_InitHooks)(&mut h)
            }
            Cfg::Budget => {
                let mut h = CJsonHooks {
                    malloc_fn: Some(budget_malloc),
                    free_fn: Some(budget_free),
                };
                (api.cJSON_InitHooks)(&mut h)
            }
        }
    }
}

/// RAII: install `cfg` on both libraries, reset to the default on drop.
struct Hooks<'a> {
    c: &'a Api,
    r: &'a Api,
}

impl<'a> Hooks<'a> {
    fn new(c: &'a Api, r: &'a Api, cfg: Cfg) -> Hooks<'a> {
        install(c, cfg);
        install(r, cfg);
        set_budget(i64::MAX);
        Hooks { c, r }
    }
}

impl<'a> Drop for Hooks<'a> {
    fn drop(&mut self) {
        set_budget(i64::MAX);
        install(self.c, Cfg::A0Reset);
        install(self.r, Cfg::A0Reset);
    }
}

// ---------------------------------------------------------------------------
// rows 66–71 — every allocator configuration re-runs the print battery
// ---------------------------------------------------------------------------
#[test]
fn cfg66_71_all_hook_configurations() {
    let (c, r) = both();
    let _guard = lock_global_state();

    let mut rng = Rng::new(0x6671_6671);
    // one shared corpus so every configuration sees identical inputs
    let specs: Vec<Spec> = (0..90)
        .map(|i| rand_spec(&mut rng, i % 5))
        .chain([
            // payloads large enough to force several growth rounds through
            // whichever path the configuration selects
            Spec::Arr((0..400).map(|i| Spec::Num(i as f64 / 3.0)).collect()),
            Spec::Obj(
                (0..300)
                    .map(|i| (format!("key_number_{i:05}").into_bytes(), Spec::Num(i as f64)))
                    .collect(),
            ),
            Spec::Str(vec![1u8; 900]),
            Spec::Raw(vec![b'r'; 2000]),
        ])
        .collect();

    for cfg in [
        Cfg::A0Reset,
        Cfg::A1Libc,
        Cfg::A2Custom,
        Cfg::A3CustomMallocOnly,
        Cfg::A4CustomFreeOnly,
        Cfg::A5BothNull,
    ] {
        let _h = Hooks::new(&c, &r, cfg);
        for (i, spec) in specs.iter().enumerate() {
            // Build + observe + delete on ONE library at a time so that the
            // allocator call counts can be attributed and compared.
            unsafe {
                reset_plain_counts();
                let bc = build(&c, spec);
                let oc = observe(&c, bc.root);
                bc.delete();
                let counts_c = plain_counts();

                reset_plain_counts();
                let br = build(&r, spec);
                let or = observe(&r, br.root);
                br.delete();
                let counts_r = plain_counts();

                assert_obs_eq(&oc, &or, &format!("hooks {cfg:?} spec #{i}"), spec);
                assert_eq!(
                    counts_c, counts_r,
                    "hooks {cfg:?} spec #{i}: (allocate, deallocate) hook call counts differ — \
                     the two libraries disagree about whether `reallocate` is available"
                );
            }
        }
        // parse also allocates through the hooks
        for text in [
            &b"{\"a\":[1,2,3],\"b\":\"str\\u00e9\",\"c\":{\"d\":null}}"[..],
            &b"[1,2,3,4,5,6,7,8,9,10]"[..],
            &b"\"\\ud83d\\ude00\""[..],
            &b"[bad"[..],
        ] {
            unsafe {
                let b = Bytes::new(text);
                let ic = (c.cJSON_Parse)(b.as_ptr());
                let ir = (r.cJSON_Parse)(b.as_ptr());
                assert_eq!(ic.is_null(), ir.is_null(), "parse nullness under {cfg:?}");
                assert_eq!(snap(ic), snap(ir), "parse snapshot under {cfg:?}");
                assert_eq!(
                    print_and_take(&c, ic),
                    print_and_take(&r, ir),
                    "print of parsed under {cfg:?}"
                );
                (c.cJSON_Delete)(ic);
                (r.cJSON_Delete)(ir);
            }
        }
    }
}

/// Cross-configuration lifetime: build under one allocator, print/delete under
/// another (a real consumer that calls `cJSON_InitHooks` mid-run does this).
#[test]
fn cfg66_71_cross_configuration_lifetimes() {
    let (c, r) = both();
    let _guard = lock_global_state();
    let mut rng = Rng::new(0x6671_C205);

    let configs = [
        Cfg::A0Reset,
        Cfg::A1Libc,
        Cfg::A2Custom,
        Cfg::A3CustomMallocOnly,
        Cfg::A4CustomFreeOnly,
        Cfg::A5BothNull,
    ];
    for build_cfg in configs {
        for use_cfg in configs {
            let spec = rand_spec(&mut rng, 3);
            install(&c, build_cfg);
            install(&r, build_cfg);
            unsafe {
                let bc = build(&c, &spec);
                let br = build(&r, &spec);
                install(&c, use_cfg);
                install(&r, use_cfg);
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("built under {build_cfg:?}, used under {use_cfg:?}"),
                    &spec,
                );
                bc.delete();
                br.delete();
            }
        }
    }
    install(&c, Cfg::A0Reset);
    install(&r, Cfg::A0Reset);
}

// ---------------------------------------------------------------------------
// failure injection helpers
// ---------------------------------------------------------------------------

/// Runs `f` on the C library and then on the Rust library with an identical
/// allocation budget, and requires the two results to match.  `probe` turns the
/// side effects into a comparable value.
fn sweep<T, F>(c: &Api, r: &Api, label: &str, max_budget: i64, mut f: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> T,
{
    for k in 0..=max_budget {
        BUDGET_HOLDER.store(k, Ordering::SeqCst);
        set_budget(k);
        let a = f(c);
        let used_c = attempts();
        BUDGET_HOLDER.store(k, Ordering::SeqCst);
        set_budget(k);
        let b = f(r);
        let used_r = attempts();
        assert_eq!(
            a, b,
            "{label}: results differ with allocation budget {k}\nC = {a:?}\nRust = {b:?}"
        );
        assert_eq!(
            used_c, used_r,
            "{label}: number of allocation attempts differs with budget {k}"
        );
    }
    set_budget(i64::MAX);
}

#[derive(Debug, PartialEq)]
struct PtrProbe {
    is_null: bool,
    snapshot: Option<NodeSnap>,
}

// ---------------------------------------------------------------------------
// ERRORS rows 7, 150, 151, 154, 157, 160, 163 — constructors under allocation failure
// ---------------------------------------------------------------------------
#[test]
fn err_hooks_constructors() {
    let (c, r) = both();
    let _guard = lock_global_state();
    let _h = Hooks::new(&c, &r, Cfg::Budget);

    let payload = Bytes::new(b"a moderately long payload string");
    unsafe {
        // cJSON_CreateString / cJSON_CreateRaw: 2 allocations each (node + strdup)
        sweep(&c, &r, "cJSON_CreateString", 4, |api| {
            let p = (api.cJSON_CreateString)(payload.as_ptr());
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
        sweep(&c, &r, "cJSON_CreateRaw", 4, |api| {
            let p = (api.cJSON_CreateRaw)(payload.as_ptr());
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
        for name in [
            "cJSON_CreateNull",
            "cJSON_CreateTrue",
            "cJSON_CreateFalse",
            "cJSON_CreateArray",
            "cJSON_CreateObject",
        ] {
            sweep(&c, &r, name, 2, |api| {
                let p = match name {
                    "cJSON_CreateNull" => (api.cJSON_CreateNull)(),
                    "cJSON_CreateTrue" => (api.cJSON_CreateTrue)(),
                    "cJSON_CreateFalse" => (api.cJSON_CreateFalse)(),
                    "cJSON_CreateArray" => (api.cJSON_CreateArray)(),
                    _ => (api.cJSON_CreateObject)(),
                };
                let out = PtrProbe {
                    is_null: p.is_null(),
                    snapshot: snap(p),
                };
                set_budget(i64::MAX);
                (api.cJSON_Delete)(p);
                out
            });
        }
        sweep(&c, &r, "cJSON_CreateNumber", 2, |api| {
            let p = (api.cJSON_CreateNumber)(1.5);
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
        sweep(&c, &r, "cJSON_CreateStringReference", 2, |api| {
            let p = (api.cJSON_CreateStringReference)(payload.as_ptr());
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });

        // typed arrays: 1 node + count nodes
        let ints: Vec<c_int> = vec![1, 2, 3, 4, 5];
        let floats: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let doubles: Vec<f64> = vec![1.0, 2.5, 3.0, 4.0, 5.0];
        let strs = [
            Bytes::new(b"s0"),
            Bytes::new(b"s1"),
            Bytes::new(b"s2"),
            Bytes::new(b"s3"),
            Bytes::new(b"s4"),
        ];
        let strptrs: Vec<*const std::ffi::c_char> = strs.iter().map(|b| b.as_ptr()).collect();

        sweep(&c, &r, "cJSON_CreateIntArray", 8, |api| {
            let p = (api.cJSON_CreateIntArray)(ints.as_ptr(), 5);
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
        sweep(&c, &r, "cJSON_CreateFloatArray", 8, |api| {
            let p = (api.cJSON_CreateFloatArray)(floats.as_ptr(), 5);
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
        sweep(&c, &r, "cJSON_CreateDoubleArray", 8, |api| {
            let p = (api.cJSON_CreateDoubleArray)(doubles.as_ptr(), 5);
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
        sweep(&c, &r, "cJSON_CreateStringArray", 14, |api| {
            let p = (api.cJSON_CreateStringArray)(strptrs.as_ptr(), 5);
            let out = PtrProbe {
                is_null: p.is_null(),
                snapshot: snap(p),
            };
            set_budget(i64::MAX);
            (api.cJSON_Delete)(p);
            out
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 8, 10, 40, 51, 74, 83 — parse under allocation failure
// ---------------------------------------------------------------------------
#[test]
fn err_hooks_parse() {
    let (c, r) = both();
    let _guard = lock_global_state();
    let _h = Hooks::new(&c, &r, Cfg::Budget);

    let docs: Vec<Bytes> = [
        &b"null"[..],
        &b"123"[..],
        &b"1.5e3"[..],
        &b"\"a string\""[..],
        &b"\"esc\\u00e9\\n\\t\""[..],
        &b"[]"[..],
        &b"[1]"[..],
        &b"[1,2,3,4,5]"[..],
        &b"{}"[..],
        &b"{\"a\":1}"[..],
        &b"{\"a\":1,\"b\":[1,2],\"c\":{\"d\":\"e\"}}"[..],
        &b"[[[[1]]]]"[..],
    ]
    .iter()
    .map(|t| Bytes::new(t))
    .collect();

    unsafe {
        for (i, d) in docs.iter().enumerate() {
            let label = format!("cJSON_Parse doc #{i}");
            sweep(&c, &r, &label, 20, |api| {
                let p = (api.cJSON_Parse)(d.as_ptr());
                let err = (api.cJSON_GetErrorPtr)();
                let out = (
                    PtrProbe {
                        is_null: p.is_null(),
                        snapshot: snap(p),
                    },
                    if err.is_null() {
                        None
                    } else {
                        Some(err as isize - d.as_ptr() as isize)
                    },
                );
                set_budget(i64::MAX);
                (api.cJSON_Delete)(p);
                out
            });
            let label = format!("cJSON_ParseWithLengthOpts doc #{i}");
            sweep(&c, &r, &label, 20, |api| {
                let mut end: *const std::ffi::c_char = std::ptr::null();
                let p = (api.cJSON_ParseWithLengthOpts)(
                    d.as_ptr(),
                    d.0.len(),
                    &mut end,
                    1,
                );
                let out = (
                    PtrProbe {
                        is_null: p.is_null(),
                        snapshot: snap(p),
                    },
                    if end.is_null() {
                        None
                    } else {
                        Some(end as isize - d.as_ptr() as isize)
                    },
                );
                set_budget(i64::MAX);
                (api.cJSON_Delete)(p);
                out
            });
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 26, 54, 57, 59 — print under allocation failure (manual growth)
// ---------------------------------------------------------------------------
#[test]
fn err_hooks_print() {
    let (c, r) = both();
    let _guard = lock_global_state();
    let _h = Hooks::new(&c, &r, Cfg::Budget);

    let specs = [
        Spec::Null,
        Spec::Num(1.5),
        Spec::Str(b"short".to_vec()),
        // long enough that the 256-byte initial buffer must grow several times
        Spec::Arr((0..200).map(|i| Spec::Num(i as f64 / 7.0)).collect()),
        Spec::Obj(
            (0..80)
                .map(|i| (format!("k{i:04}").into_bytes(), Spec::Str(vec![b'v'; 12])))
                .collect(),
        ),
        Spec::Str(vec![1u8; 400]),
    ];

    unsafe {
        for (i, spec) in specs.iter().enumerate() {
            let bc = build(&c, spec);
            let br = build(&r, spec);
            for &fmt in &[0i32, 1] {
                let label = format!("cJSON_Print spec #{i} fmt={fmt}");
                sweep(&c, &r, &label, 12, |api| {
                    let root = if api.tag == "C" { bc.root } else { br.root };
                    let p = if fmt == 1 {
                        (api.cJSON_Print)(root)
                    } else {
                        (api.cJSON_PrintUnformatted)(root)
                    };
                    let out = cstr(p as *const std::ffi::c_char);
                    set_budget(i64::MAX);
                    if !p.is_null() {
                        (api.cJSON_free)(p as *mut c_void);
                    }
                    out
                });
                for prebuffer in [0, 1, 16, 256, 4096] {
                    let label = format!("cJSON_PrintBuffered spec #{i} fmt={fmt} pb={prebuffer}");
                    sweep(&c, &r, &label, 12, |api| {
                        let root = if api.tag == "C" { bc.root } else { br.root };
                        let p = (api.cJSON_PrintBuffered)(root, prebuffer, fmt);
                        let out = cstr(p as *const std::ffi::c_char);
                        set_budget(i64::MAX);
                        if !p.is_null() {
                            (api.cJSON_free)(p as *mut c_void);
                        }
                        out
                    });
                }
            }
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 18, 103, 112, 148, 165, 166, 167 — mutators under failure
// ---------------------------------------------------------------------------
#[test]
fn err_hooks_mutators() {
    let (c, r) = both();
    let _guard = lock_global_state();
    let _h = Hooks::new(&c, &r, Cfg::Budget);

    let key = Bytes::new(b"a_key_long_enough_to_need_its_own_allocation");
    let longer = Bytes::new(b"a replacement value that is strictly longer than the original");

    unsafe {
        // row 112: add_item_to_object's cJSON_strdup(key) failure
        sweep(&c, &r, "cJSON_AddItemToObject strdup failure", 6, |api| {
            let o = (api.cJSON_CreateObject)();
            let n = (api.cJSON_CreateNumber)(1.0);
            let rc = (api.cJSON_AddItemToObject)(o, key.as_ptr(), n);
            let out = (rc, snap(o), snap(n));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(o);
            if rc == 0 {
                (api.cJSON_Delete)(n);
            }
            out
        });

        // rows 117–127: the nine Add*ToObject helpers
        sweep(&c, &r, "cJSON_AddStringToObject under failure", 8, |api| {
            let o = (api.cJSON_CreateObject)();
            let p = (api.cJSON_AddStringToObject)(o, key.as_ptr(), longer.as_ptr());
            let out = (p.is_null(), snap(o));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(o);
            out
        });
        sweep(&c, &r, "cJSON_AddObjectToObject under failure", 6, |api| {
            let o = (api.cJSON_CreateObject)();
            let p = (api.cJSON_AddObjectToObject)(o, key.as_ptr());
            let out = (p.is_null(), snap(o));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(o);
            out
        });
        sweep(&c, &r, "cJSON_AddNumberToObject under failure", 6, |api| {
            let o = (api.cJSON_CreateObject)();
            let p = (api.cJSON_AddNumberToObject)(o, key.as_ptr(), 3.5);
            let out = (p.is_null(), snap(o));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(o);
            out
        });

        // row 103: create_reference's cJSON_New_Item failure
        sweep(&c, &r, "cJSON_AddItemReferenceToArray under failure", 6, |api| {
            let a = (api.cJSON_CreateArray)();
            let n = (api.cJSON_CreateNumber)(1.0);
            (api.cJSON_AddItemToArray)(a, n);
            let outer = (api.cJSON_CreateArray)();
            let rc = (api.cJSON_AddItemReferenceToArray)(outer, n);
            let out = (rc, snap(outer));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(outer);
            (api.cJSON_Delete)(a);
            out
        });
        sweep(&c, &r, "cJSON_AddItemReferenceToObject under failure", 8, |api| {
            let a = (api.cJSON_CreateArray)();
            let n = (api.cJSON_CreateNumber)(1.0);
            (api.cJSON_AddItemToArray)(a, n);
            let outer = (api.cJSON_CreateObject)();
            let rc = (api.cJSON_AddItemReferenceToObject)(outer, key.as_ptr(), n);
            let out = (rc, snap(outer));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(outer);
            (api.cJSON_Delete)(a);
            out
        });

        // row 18: cJSON_SetValuestring's cJSON_strdup failure (longer value)
        sweep(&c, &r, "cJSON_SetValuestring strdup failure", 6, |api| {
            set_budget(i64::MAX);
            let s = (api.cJSON_CreateString)(cs("short").as_ptr());
            set_budget(BUDGET_HOLDER.load(Ordering::SeqCst));
            let p = (api.cJSON_SetValuestring)(s, longer.as_ptr());
            let out = (p.is_null(), cstr(p), snap(s));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(s);
            out
        });

        // row 148: replace_item_in_object's cJSON_strdup failure
        sweep(&c, &r, "cJSON_ReplaceItemInObject strdup failure", 10, |api| {
            set_budget(i64::MAX);
            let o = (api.cJSON_CreateObject)();
            (api.cJSON_AddNumberToObject)(o, cs("k").as_ptr(), 1.0);
            let n = (api.cJSON_CreateNumber)(2.0);
            set_budget(BUDGET_HOLDER.load(Ordering::SeqCst));
            let rc = (api.cJSON_ReplaceItemInObject)(o, key.as_ptr(), n);
            let out = (rc, snap(o), snap(n));
            set_budget(i64::MAX);
            (api.cJSON_Delete)(o);
            if rc == 0 {
                (api.cJSON_Delete)(n);
            }
            out
        });

        // rows 165–167: cJSON_Duplicate under failure
        let spec = Spec::Obj(vec![
            (b"a".to_vec(), Spec::Str(b"value a".to_vec())),
            (
                b"b".to_vec(),
                Spec::Arr(vec![Spec::Num(1.0), Spec::Str(b"inner".to_vec())]),
            ),
        ]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        for recurse in [0i32, 1] {
            let label = format!("cJSON_Duplicate(recurse={recurse}) under failure");
            sweep(&c, &r, &label, 20, |api| {
                let root = if api.tag == "C" { bc.root } else { br.root };
                let d = (api.cJSON_Duplicate)(root, recurse);
                let out = PtrProbe {
                    is_null: d.is_null(),
                    snapshot: snap(d),
                };
                set_budget(i64::MAX);
                (api.cJSON_Delete)(d);
                out
            });
        }
        bc.delete();
        br.delete();
    }
}

/// `sweep` resets the budget inside the closure for setup/teardown, so the
/// closure needs to know the budget it is being run with.  `sweep` records it
/// here before invoking the closure.
static BUDGET_HOLDER: AtomicI64 = AtomicI64::new(i64::MAX);
